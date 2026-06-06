use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use serde_json::Value;

use crate::operator_contracts::build_release1_operator_output_payload;
use crate::operator_toon_report::OperatorToonField;
use crate::release1_contracts::{blocker_code_str, BlockerCode};
use crate::state_store::{StateStore, TaskRecord};
use crate::surface_render::print_surface_json;
use crate::{SessionArgs, SessionCommand, SessionTriageArgs};

const SESSION_TRIAGE_SURFACE: &str = "vida session triage";

pub(crate) async fn run_session(args: SessionArgs) -> ExitCode {
    match args.command {
        SessionCommand::Triage(command) => run_session_triage(command).await,
    }
}

async fn run_session_triage(command: SessionTriageArgs) -> ExitCode {
    let payload = build_session_triage_payload(&command).await;
    if !print_surface_json(
        &payload,
        command.json,
        "session triage payload should render as json",
    ) {
        print_session_triage_toon(&payload);
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn build_session_triage_payload(command: &SessionTriageArgs) -> Value {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let store = match StateStore::open_existing_read_only_with_timeout(
        state_dir.clone(),
        Duration::from_secs(2),
    )
    .await
    {
        Ok(store) => store,
        Err(error) => {
            return blocked_session_triage_payload(
                state_dir,
                vec![blocker_code_str(BlockerCode::ProjectActivationUnknown).to_string()],
                vec![
                    "Run `vida project-activator` before session triage if this project has no initialized VIDA state."
                        .to_string(),
                ],
                serde_json::json!({
                    "state_dir_error": error.to_string(),
                }),
            );
        }
    };

    let tasks = match store.all_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => {
            return blocked_session_triage_payload(
                state_dir,
                vec![blocker_code_str(BlockerCode::TaskGraphEmpty).to_string()],
                vec![
                    "Run `vida task validate-graph` after restoring the TaskFlow snapshot."
                        .to_string(),
                ],
                serde_json::json!({
                    "task_read_error": error.to_string(),
                }),
            );
        }
    };
    let graph_issues = match store.validate_task_graph().await {
        Ok(issues) => issues,
        Err(error) => {
            return blocked_session_triage_payload(
                state_dir,
                vec![blocker_code_str(BlockerCode::DependencyGraphIssues).to_string()],
                vec!["Run `vida task validate-graph` and resolve graph issues.".to_string()],
                serde_json::json!({
                    "graph_validation_error": error.to_string(),
                }),
            );
        }
    };

    let active_tasks = active_bounded_task_candidates(&tasks);
    let target_task_id = command
        .task_id
        .as_deref()
        .or_else(|| active_tasks.first().map(|task| task.id.as_str()));
    let target_task =
        target_task_id.and_then(|task_id| tasks.iter().find(|task| task.id == task_id));
    let explicit_target_missing = command.task_id.is_some() && target_task.is_none();
    let current_epic = target_task
        .and_then(|task| parent_task_id(task))
        .and_then(|parent_id| tasks.iter().find(|task| task.id == parent_id));

    let latest_run_graph_status = store.latest_run_graph_status().await.ok().flatten();
    let current_session_run_graph_status = store
        .latest_run_graph_status_for_current_session()
        .await
        .ok()
        .flatten();
    let latest_run_graph_recovery = store
        .latest_run_graph_recovery_summary()
        .await
        .ok()
        .flatten();
    let current_session_run_graph_recovery = store
        .latest_run_graph_recovery_summary_for_current_session()
        .await
        .ok()
        .flatten();
    let latest_dispatch_receipt = store
        .latest_run_graph_dispatch_receipt_summary()
        .await
        .ok()
        .flatten();
    let current_session_dispatch_receipt = store
        .latest_run_graph_dispatch_receipt_summary_for_current_session()
        .await
        .ok()
        .flatten();

    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    if !graph_issues.is_empty() {
        blocker_codes.push(blocker_code_str(BlockerCode::DependencyGraphIssues).to_string());
        next_actions.push("Run `vida task validate-graph` and resolve graph issues.".to_string());
    }
    if explicit_target_missing {
        blocker_codes.push(blocker_code_str(BlockerCode::NextActionTargetMissing).to_string());
        next_actions.push("Run `vida task list` and select an existing TaskFlow task.".to_string());
    }
    let active_binding_ambiguous = command.task_id.is_none() && active_tasks.len() > 1;
    if active_binding_ambiguous {
        blocker_codes.push(blocker_code_str(BlockerCode::ForeignClaimConflictBlocked).to_string());
        next_actions.push(
            "Run `vida session triage --task <task-id>` for one explicit active bounded unit."
                .to_string(),
        );
    }

    let explicit_active_target = command.task_id.as_ref().and(target_task).filter(|task| {
        task.status == "in_progress"
            && !crate::state_store::work_item_is_program_container(&task.issue_type)
    });
    let active_bounded_unit = explicit_active_target
        .or_else(|| {
            (!active_binding_ambiguous)
                .then(|| active_tasks.first().copied())
                .flatten()
        })
        .map(task_summary);
    let why_this_unit = if explicit_active_target.is_some() {
        "Explicit TaskFlow task argument is the active bounded unit."
    } else if active_tasks.len() == 1 {
        "Single TaskFlow in_progress task is the authoritative active bounded unit."
    } else if active_tasks.is_empty() {
        "No in_progress TaskFlow task is currently active."
    } else {
        "Multiple in_progress TaskFlow tasks exist; bind one task before write-producing work."
    };
    let sequential_vs_parallel_posture = if active_tasks.len() <= 1 {
        "sequential_only_taskflow_active"
    } else {
        "ambiguous_until_explicit_binding"
    };

    let artifact_refs = serde_json::json!({
        "surface": SESSION_TRIAGE_SURFACE,
        "state_dir": state_dir.display().to_string(),
        "target_task_id": target_task_id,
        "latest_run_graph_run_id": latest_run_graph_status.as_ref().map(|status| status.run_id.as_str()),
        "current_session_run_graph_run_id": current_session_run_graph_status.as_ref().map(|status| status.run_id.as_str()),
    });

    let extra_fields = serde_json::json!({
        "active_bounded_unit": active_bounded_unit,
        "why_this_unit": why_this_unit,
        "sequential_vs_parallel_posture": sequential_vs_parallel_posture,
        "current_epic": current_epic.map(task_summary),
        "target_task": target_task.map(task_summary),
        "active_bounded_unit_candidates": active_tasks.iter().map(|task| task_summary(task)).collect::<Vec<_>>(),
        "task_tree_summary": task_tree_summary(&tasks, current_epic.or(target_task)),
        "graph_validation": {
            "valid": graph_issues.is_empty(),
            "issue_count": graph_issues.len(),
            "issues": graph_issues,
        },
        "latest_run_parity": {
            "status": latest_run_parity_label(
                latest_run_graph_status.as_ref().map(|status| status.run_id.as_str()),
                current_session_run_graph_status.as_ref().map(|status| status.run_id.as_str()),
            ),
            "latest_status": latest_run_graph_status.as_ref().map(compact_run_graph_status),
            "current_session_status": current_session_run_graph_status.as_ref().map(compact_run_graph_status),
            "latest_recovery_run_id": latest_run_graph_recovery.as_ref().map(|recovery| recovery.run_id.as_str()),
            "current_session_recovery_run_id": current_session_run_graph_recovery.as_ref().map(|recovery| recovery.run_id.as_str()),
            "latest_dispatch_receipt_run_id": latest_dispatch_receipt.as_ref().map(|receipt| receipt.run_id.as_str()),
            "current_session_dispatch_receipt_run_id": current_session_dispatch_receipt.as_ref().map(|receipt| receipt.run_id.as_str()),
        },
        "vida_owned_evidence": {
            "active_bounded_unit_source": "TaskFlow in_progress authoritative state",
            "task_graph_source": "StateStore::validate_task_graph",
            "latest_run_source": "StateStore latest run graph summaries",
            "state_store_shared_inputs": true,
        },
        "external_evidence": {
            "github": "not_read_by_default",
            "git": "not_read_by_default",
            "reason": "session triage separates VIDA-owned runtime evidence from external repository checks",
        },
    });

    build_release1_operator_output_payload(
        SESSION_TRIAGE_SURFACE,
        blocker_codes,
        next_actions,
        artifact_refs,
        extra_fields,
    )
    .unwrap_or_else(|error| {
        blocked_session_triage_payload(
            state_dir,
            vec![blocker_code_str(BlockerCode::Unsupported).to_string()],
            vec![
                "Report session triage operator-contract payload construction failure.".to_string(),
            ],
            serde_json::json!({ "operator_contract_error": error }),
        )
    })
}

fn blocked_session_triage_payload(
    state_dir: PathBuf,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    diagnostics: Value,
) -> Value {
    build_release1_operator_output_payload(
        SESSION_TRIAGE_SURFACE,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "surface": SESSION_TRIAGE_SURFACE,
            "state_dir": state_dir.display().to_string(),
        }),
        serde_json::json!({
            "active_bounded_unit": Value::Null,
            "why_this_unit": Value::Null,
            "sequential_vs_parallel_posture": "unknown_until_state_available",
            "current_epic": Value::Null,
            "target_task": Value::Null,
            "task_tree_summary": Value::Null,
            "graph_validation": {
                "valid": false,
                "issue_count": Value::Null,
                "issues": [],
            },
            "latest_run_parity": {
                "status": "unavailable",
            },
            "vida_owned_evidence": diagnostics,
            "external_evidence": {
                "github": "not_read_by_default",
                "git": "not_read_by_default",
            },
        }),
    )
    .expect("blocked session triage payload should satisfy operator contract")
}

fn print_session_triage_toon(payload: &Value) {
    crate::operator_toon_report::print(
        SESSION_TRIAGE_SURFACE,
        vec![
            OperatorToonField::value("status", payload["status"].clone()),
            OperatorToonField::value(
                "active_bounded_unit",
                payload["active_bounded_unit"].clone(),
            ),
            OperatorToonField::value("current_epic", payload["current_epic"].clone()),
            OperatorToonField::value("graph_validation", payload["graph_validation"].clone()),
            OperatorToonField::value("latest_run_parity", payload["latest_run_parity"].clone()),
            OperatorToonField::value("blocker_codes", payload["blocker_codes"].clone()),
            OperatorToonField::value("next_actions", payload["next_actions"].clone()),
            OperatorToonField::value("external_evidence", payload["external_evidence"].clone()),
        ],
    );
}

fn active_bounded_task_candidates(tasks: &[TaskRecord]) -> Vec<&TaskRecord> {
    let mut active = tasks
        .iter()
        .filter(|task| task.status == "in_progress")
        .filter(|task| !crate::state_store::work_item_is_program_container(&task.issue_type))
        .collect::<Vec<_>>();
    active.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    active
}

fn parent_task_id(task: &TaskRecord) -> Option<&str> {
    task.dependencies
        .iter()
        .find(|dependency| dependency.edge_type == "parent-child")
        .map(|dependency| dependency.depends_on_id.as_str())
}

fn task_summary(task: &TaskRecord) -> Value {
    serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": task.status,
        "issue_type": task.issue_type,
        "priority": task.priority,
        "parent_id": parent_task_id(task),
    })
}

fn task_tree_summary(tasks: &[TaskRecord], root: Option<&TaskRecord>) -> Value {
    let Some(root) = root else {
        return Value::Null;
    };
    let direct_children = tasks
        .iter()
        .filter(|task| parent_task_id(task) == Some(root.id.as_str()))
        .collect::<Vec<_>>();
    let mut status_counts = BTreeMap::new();
    for child in &direct_children {
        *status_counts.entry(child.status.clone()).or_insert(0usize) += 1;
    }
    let open_or_in_progress_count = direct_children
        .iter()
        .filter(|task| matches!(task.status.as_str(), "open" | "in_progress"))
        .count();
    serde_json::json!({
        "root_id": root.id,
        "direct_child_count": direct_children.len(),
        "open_or_in_progress_count": open_or_in_progress_count,
        "status_counts": status_counts,
    })
}

fn latest_run_parity_label(
    latest_run_id: Option<&str>,
    current_session_run_id: Option<&str>,
) -> &'static str {
    match (latest_run_id, current_session_run_id) {
        (None, None) => "no_latest_run",
        (Some(_), None) => "no_current_session_run",
        (Some(latest), Some(current)) if latest == current => "match",
        (Some(_), Some(_)) => "mismatch",
        (None, Some(_)) => "current_session_only",
    }
}

fn compact_run_graph_status(status: &crate::state_store::RunGraphStatus) -> Value {
    serde_json::json!({
        "run_id": status.run_id,
        "task_id": status.task_id,
        "status": status.status,
        "active_node": status.active_node,
        "lifecycle_stage": status.lifecycle_stage,
        "recovery_ready": status.recovery_ready,
        "resume_target": status.resume_target,
    })
}
