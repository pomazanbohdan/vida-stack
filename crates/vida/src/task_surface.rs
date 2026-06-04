use super::*;
use crate::task_cli_render::{
    print_task_bulk_reparent_result, print_task_defect_batch_rehome_result,
    print_task_dependency_bulk_add_result, print_task_direct_children,
    print_task_update_graph_blocked, task_read_metadata_value, task_ready_payload,
    task_show_payload,
};
use crate::taskflow_proxy::paths_intersect;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct TaskReadMetadata {
    pub mode: &'static str,
    pub degraded: bool,
    pub snapshot_path: Option<String>,
    pub detail: &'static str,
}

impl TaskReadMetadata {
    fn authoritative_live() -> Self {
        Self {
            mode: "authoritative_live",
            degraded: false,
            snapshot_path: None,
            detail: "served from the authoritative state store",
        }
    }

    fn snapshot(path: &std::path::Path, detail: &'static str) -> Self {
        Self {
            mode: "snapshot",
            degraded: true,
            snapshot_path: Some(path.display().to_string()),
            detail,
        }
    }

    fn fresh_snapshot(path: &std::path::Path) -> Self {
        Self {
            mode: "fresh_snapshot",
            degraded: false,
            snapshot_path: Some(path.display().to_string()),
            detail: "served from canonical task snapshot evidence with freshness metadata",
        }
    }
}

const TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT: usize = 25;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskCloseEpicProgressSummary {
    closed_task_id: String,
    epic_count: usize,
    reported_epic_count: usize,
    omitted_epic_count: usize,
    scope: String,
    epics: Vec<TaskCloseEpicProgressRow>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskCloseEpicProgressRow {
    epic_id: String,
    epic_title: String,
    epic_status: String,
    epic_priority: u32,
    closed_count: usize,
    total_count: usize,
    percent_closed: f64,
    child_task_count: usize,
    reported_child_task_count: usize,
    child_task_report_limit: usize,
    truncated_child_tasks: bool,
    tasks: Vec<TaskCloseEpicProgressTaskRow>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskCloseEpicProgressTaskRow {
    task_id: String,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
    blocker_state: String,
    blockers: Vec<TaskCloseEpicProgressBlocker>,
    next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskCloseEpicProgressBlocker {
    task_id: String,
    status: String,
    title: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskEpicProgressSummary {
    epic_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    total_descendant_count: usize,
    total_open_descendant_count: usize,
    total_in_progress_descendant_count: usize,
    total_closed_descendant_count: usize,
    percent_closed: f64,
    include_closed_epics: bool,
    progress_basis: String,
    epic_filter: Option<String>,
    epics: Vec<TaskEpicProgressRow>,
    read_metadata: TaskReadMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskEpicProgressRow {
    epic_id: String,
    epic_title: String,
    epic_status: String,
    epic_priority: u32,
    total_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    percent_complete: f64,
    direct_child_count: usize,
    nested_epic_count: usize,
    closure_candidate: bool,
    closure_candidate_state: String,
    recommended_next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskProofTargetStatus {
    target: String,
    status: String,
    evidence_source: String,
    evidence_detail: String,
    artifact_status: String,
    next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskProofAttachBrowserReceipt {
    surface: &'static str,
    status: &'static str,
    task_id: String,
    route: String,
    result: String,
    expect: Option<String>,
    screenshot: Option<String>,
    evidence: Vec<String>,
    proof_target: String,
    notes_appended: bool,
    task: state_store::TaskRecord,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskTakeoverStatusReceipt {
    surface: &'static str,
    status: String,
    task_id: String,
    allowed: bool,
    local_exception_takeover_state: String,
    root_local_write_allowed: bool,
    paths: Vec<String>,
    packet: serde_json::Value,
    lane: serde_json::Value,
    root_write_guard: serde_json::Value,
    active_takeover_state: String,
    takeover_ready_state: String,
    recommended_surface: Option<String>,
    reason: String,
    recommended_command: Option<String>,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TaskExceptionTakeoverMetadata {
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    dispatch_target: Option<String>,
    #[serde(default)]
    source_exception_path_receipt_id: Option<String>,
    #[serde(default)]
    owned_write_scope: Vec<String>,
}

impl TaskExceptionTakeoverMetadata {
    fn matches_summary(&self, summary: &state_store::RunGraphDispatchReceiptSummary) -> bool {
        let run_id_matches = self
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.run_id);
        let target_matches = self
            .dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.dispatch_target);
        let source_receipt_matches = self
            .source_exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| {
                summary
                    .exception_path_receipt_id
                    .as_deref()
                    .is_some_and(|summary_value| value == summary_value)
            });

        run_id_matches && target_matches && source_receipt_matches
    }
}

fn task_exception_takeover_metadata_filename(run_id: &str) -> Result<String, String> {
    if run_id.is_empty() {
        return Err("Run id cannot be empty for exception takeover metadata.".to_string());
    }
    if !run_id
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return Err(format!(
            "Run id `{run_id}` contains unsupported characters for exception takeover metadata filename."
        ));
    }
    Ok(format!("{run_id}.json"))
}

fn task_exception_takeover_metadata_path(
    state_root: &std::path::Path,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    let file_name = task_exception_takeover_metadata_filename(run_id)?;
    Ok(state_root
        .join("lane-exception-path-metadata")
        .join(file_name))
}

fn read_task_exception_takeover_metadata(
    state_root: &std::path::Path,
    run_id: &str,
) -> Result<Option<TaskExceptionTakeoverMetadata>, String> {
    let path = task_exception_takeover_metadata_path(state_root, run_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "Failed to read persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    let metadata: TaskExceptionTakeoverMetadata = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to decode persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    Ok(Some(metadata))
}

fn task_exception_takeover_owned_write_scope(
    state_root: &std::path::Path,
    summary: &state_store::RunGraphDispatchReceiptSummary,
) -> Vec<String> {
    read_task_exception_takeover_metadata(state_root, &summary.run_id)
        .ok()
        .flatten()
        .filter(|metadata| metadata.matches_summary(summary))
        .map(|metadata| {
            metadata
                .owned_write_scope
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskBlockReceipt {
    surface: &'static str,
    status: &'static str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task_id: String,
    blocked: bool,
    closed: bool,
    previous_status: String,
    reason: String,
    evidence: Option<String>,
    notes_appended: bool,
    task: state_store::TaskRecord,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskVerifyReceipt {
    surface: &'static str,
    status: &'static str,
    task_id: String,
    partial: bool,
    closed: bool,
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
    proof_blocked_by_runtime: bool,
    proof_blocker: Option<String>,
    evidence: Vec<String>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task: state_store::TaskRecord,
}

fn task_json_success_status() -> &'static str {
    crate::contract_profile_adapter::release_contract_status(true)
}

fn proof_target_has_close_reason_evidence(task: &state_store::TaskRecord, target: &str) -> bool {
    let Some(reason) = task.close_reason.as_deref() else {
        return false;
    };
    let target = target.trim();
    !target.is_empty()
        && reason
            .to_ascii_lowercase()
            .contains(&target.to_ascii_lowercase())
}

fn task_close_reason_reports_runtime_proof_blocker(task: &state_store::TaskRecord) -> bool {
    let Some(reason) = task.close_reason.as_deref() else {
        return false;
    };
    let normalized = reason.to_ascii_lowercase();
    normalized.contains("proof blocked by runtime")
        || normalized.contains("runtime proof blocker")
        || normalized.contains("runtime blocker")
}

fn task_reports_runtime_proof_blocker(task: &state_store::TaskRecord) -> bool {
    task_close_reason_reports_runtime_proof_blocker(task)
        || task
            .labels
            .iter()
            .any(|label| label == "proof-blocked-by-runtime" || label == "runtime-proof-blocked")
}

fn normalize_browser_proof_result(result: &str) -> Result<String, String> {
    let result = result.trim().to_ascii_lowercase();
    match result.as_str() {
        "pass" | "passed" | "success" | "satisfied" => Ok("pass".to_string()),
        "fail" | "failed" | "failure" => Ok("fail".to_string()),
        "blocked" | "block" => Ok("blocked".to_string()),
        _ => Err("--result must be one of: pass, fail, blocked".to_string()),
    }
}

fn browser_proof_target(route: &str, expect: Option<&str>) -> String {
    match expect.map(str::trim).filter(|value| !value.is_empty()) {
        Some(expect) => format!(
            "vida proof browser --route {} --expect {}",
            route.trim(),
            expect
        ),
        None => format!("vida proof browser --route {}", route.trim()),
    }
}

fn task_notes_have_browser_proof_evidence(task: &state_store::TaskRecord, target: &str) -> bool {
    let Some(notes) = task.notes.as_deref() else {
        return false;
    };
    let target = target.trim();
    if target.is_empty() {
        return false;
    }

    let mut in_browser_proof_record = false;
    let mut proof_target: Option<&str> = None;
    let mut command: Option<&str> = None;
    let mut result: Option<&str> = None;

    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed == "task_browser_proof:" {
            if browser_proof_record_satisfies_target(proof_target, command, result, target) {
                return true;
            }
            in_browser_proof_record = true;
            proof_target = None;
            command = None;
            result = None;
            continue;
        }

        if !in_browser_proof_record {
            continue;
        }

        let field = line.trim_start();
        if proof_target.is_none() {
            proof_target = field.strip_prefix("proof_target:").map(str::trim);
        }
        if command.is_none() {
            command = field.strip_prefix("command:").map(str::trim);
        }
        if result.is_none() {
            result = field.strip_prefix("result:").map(str::trim);
        }
    }

    browser_proof_record_satisfies_target(proof_target, command, result, target)
}

fn browser_proof_record_satisfies_target(
    proof_target: Option<&str>,
    command: Option<&str>,
    result: Option<&str>,
    target: &str,
) -> bool {
    result == Some("pass") && (proof_target == Some(target) || command == Some(target))
}

fn task_proof_target_status(task: &state_store::TaskRecord, target: &str) -> TaskProofTargetStatus {
    let target = target.trim().to_string();
    let runtime_blocked = task_reports_runtime_proof_blocker(task);
    if proof_target_has_close_reason_evidence(task, &target) {
        return TaskProofTargetStatus {
            target,
            status: "satisfied".to_string(),
            evidence_source: "close_reason".to_string(),
            evidence_detail: "target text is present in task close_reason".to_string(),
            artifact_status: "not_recorded".to_string(),
            next_action: "No action for this proof target.".to_string(),
        };
    }
    if task_notes_have_browser_proof_evidence(task, &target) {
        return TaskProofTargetStatus {
            target,
            status: "satisfied".to_string(),
            evidence_source: "task_browser_proof_note".to_string(),
            evidence_detail: "attached browser proof note reports result pass".to_string(),
            artifact_status: "recorded_in_task_notes".to_string(),
            next_action: "No action for this proof target.".to_string(),
        };
    }
    if runtime_blocked {
        return TaskProofTargetStatus {
            target: target.clone(),
            status: "blocked_by_runtime".to_string(),
            evidence_source: "close_reason".to_string(),
            evidence_detail: "task close_reason reports runtime proof blocker context".to_string(),
            artifact_status: "not_recorded".to_string(),
            next_action: format!(
                "Resolve runtime proof blocker, then record evidence for proof target `{}`.",
                target
            ),
        };
    }
    let status = if state_store::StateStore::task_status_is_closed_like(&task.status) {
        "missing_evidence"
    } else {
        "pending"
    };
    TaskProofTargetStatus {
        target: target.clone(),
        status: status.to_string(),
        evidence_source: "planner_metadata.proof_targets".to_string(),
        evidence_detail: "no matching close_reason or structured proof artifact found".to_string(),
        artifact_status: "not_recorded".to_string(),
        next_action: format!(
            "Run or attach evidence for proof target `{}`, then close or update `{}`.",
            target, task.id
        ),
    }
}

fn task_proof_status_payload(
    task: &state_store::TaskRecord,
    read_metadata: Option<&TaskReadMetadata>,
) -> serde_json::Value {
    let targets = task
        .planner_metadata
        .proof_targets
        .iter()
        .map(|target| task_proof_target_status(task, target))
        .collect::<Vec<_>>();
    let configured_count = targets.len();
    let satisfied_count = targets
        .iter()
        .filter(|target| target.status == "satisfied")
        .count();
    let runtime_blocked_count = targets
        .iter()
        .filter(|target| target.status == "blocked_by_runtime")
        .count();
    let missing_count = targets
        .iter()
        .filter(|target| target.status != "satisfied")
        .count();
    let missing_targets = targets
        .iter()
        .filter(|target| target.status != "satisfied")
        .map(|target| target.target.clone())
        .collect::<Vec<_>>();
    let quoted_task_id = crate::shell_quote(&task.id);
    let next_required_command = if configured_count == 0 {
        format!(
            "Add proof targets with `vida task update {} --proof-target <command-or-artifact> --json`.",
            quoted_task_id
        )
    } else if missing_count == 0 {
        "No proof action required; all configured proof targets have close evidence.".to_string()
    } else {
        format!(
            "Run or attach missing proof evidence, then inspect again with `vida task proof status {} --json`.",
            quoted_task_id
        )
    };
    serde_json::json!({
        "surface": "vida task proof status",
        "status": task_json_success_status(),
        "task_id": task.id,
        "task_status": task.status,
        "configured_proof_target_count": configured_count,
        "satisfied_count": satisfied_count,
        "missing_count": missing_count,
        "runtime_blocked_count": runtime_blocked_count,
        "missing_proof": configured_count > 0 && missing_count > 0,
        "proof_blocked_by_runtime": runtime_blocked_count > 0,
        "proof_targets": targets,
        "missing_targets": missing_targets,
        "next_required_command": next_required_command,
            "evidence_model": {
                "configured_targets_source": "task.planner_metadata.proof_targets",
                "satisfaction_source": "task.close_reason substring match or task_browser_proof note",
                "artifact_registry": "task_notes.task_browser_proof"
            },
        "state_access": task_read_metadata_value(read_metadata),
    })
}

fn print_task_proof_status(
    render: RenderMode,
    task: &state_store::TaskRecord,
    payload: &serde_json::Value,
) {
    print_surface_header(render, "vida task proof status");
    print_surface_line(render, "task", &task.id);
    print_surface_line(render, "task status", &task.status);
    print_surface_line(
        render,
        "proof targets",
        &payload["configured_proof_target_count"].to_string(),
    );
    print_surface_line(render, "satisfied", &payload["satisfied_count"].to_string());
    print_surface_line(render, "missing", &payload["missing_count"].to_string());
    print_surface_line(
        render,
        "runtime blocked",
        &payload["proof_blocked_by_runtime"].to_string(),
    );
    print_surface_line(
        render,
        "next",
        payload["next_required_command"].as_str().unwrap_or(""),
    );
}

fn exception_takeover_state_label(
    state: crate::release1_contracts::ExceptionTakeoverState,
) -> &'static str {
    match state {
        crate::release1_contracts::ExceptionTakeoverState::NotRecorded => "not_recorded",
        crate::release1_contracts::ExceptionTakeoverState::ReceiptRecorded => "receipt_recorded",
        crate::release1_contracts::ExceptionTakeoverState::ActiveTakeover => "active",
    }
}

async fn task_takeover_status_receipt(
    store: &StateStore,
    task: &state_store::TaskRecord,
    status_override: Option<state_store::RunGraphStatus>,
    lane_source_override: Option<&str>,
) -> TaskTakeoverStatusReceipt {
    let (lane_source, status) = if let Some(status) = status_override {
        (lane_source_override.unwrap_or("run_id"), Some(status))
    } else {
        let current_status = store
            .latest_run_graph_status_for_current_session()
            .await
            .ok()
            .flatten();
        match current_status {
            Some(status) => ("current_session", Some(status)),
            None => (
                "latest",
                store.latest_run_graph_status().await.ok().flatten(),
            ),
        }
    };
    let Some(status) = status else {
        return TaskTakeoverStatusReceipt {
            surface: "vida task takeover status",
            status: "blocked".to_string(),
            task_id: task.id.clone(),
            allowed: false,
            local_exception_takeover_state: "not_recorded".to_string(),
            root_local_write_allowed: false,
            paths: Vec::new(),
            packet: serde_json::json!({
                "dispatch_packet_path": serde_json::Value::Null,
                "dispatch_result_path": serde_json::Value::Null,
            }),
            lane: serde_json::json!({
                "source": lane_source,
                "run_id": serde_json::Value::Null,
                "task_id": serde_json::Value::Null,
            }),
            root_write_guard: serde_json::json!({
                "status": "blocked_by_default",
                "root_local_write_allowed": false,
                "root_local_write_allowed_for_only_these_paths": [],
                "local_exception_takeover_state": "not_recorded",
                "latest_lane_status": serde_json::Value::Null,
                "local_exception_takeover_gate": serde_json::Value::Null,
                "latest_run_graph_task_stale": false,
                "reason": "no run-graph lane evidence is available",
            }),
            active_takeover_state: "not_recorded".to_string(),
            takeover_ready_state: "not_ready".to_string(),
            recommended_surface: Some("vida lane show".to_string()),
            reason: "no run-graph lane evidence is available for takeover status".to_string(),
            recommended_command: Some("vida lane show --latest --json".to_string()),
            next_actions: vec![
                "Run `vida lane show --latest --json` to inspect lane evidence before attempting exception takeover."
                    .to_string(),
            ],
            blocker_codes: vec!["missing_latest_lane_receipt".to_string()],
        };
    };
    let summary = store
        .run_graph_dispatch_receipt_summary_for_status(&status)
        .await
        .ok()
        .flatten();
    let recovery = store.run_graph_recovery_summary(&status.run_id).await.ok();
    let recovery_gate = recovery.as_ref().map(|recovery| {
        recovery
            .delegation_gate
            .local_exception_takeover_gate
            .as_str()
    });
    let (summary, takeover_state) = match summary {
        Some(summary) => {
            let state = crate::release1_contracts::exception_takeover_state(
                summary.exception_path_receipt_id.as_deref(),
                summary.supersedes_receipt_id.as_deref(),
                recovery_gate,
            );
            (Some(summary), state)
        }
        None => (
            None,
            crate::release1_contracts::ExceptionTakeoverState::NotRecorded,
        ),
    };
    let task_matches_lane = status.task_id.trim() == task.id.trim();
    let state_label = exception_takeover_state_label(takeover_state).to_string();
    let metadata_paths = summary
        .as_ref()
        .filter(|_| takeover_state.is_active())
        .map(|summary| task_exception_takeover_owned_write_scope(store.root(), summary))
        .unwrap_or_default();
    let paths = metadata_paths;
    let root_local_write_allowed =
        task_matches_lane && takeover_state.is_active() && !paths.is_empty();
    let allowed = root_local_write_allowed;
    let (reason, blocker_codes, next_actions, recommended_command, recommended_surface) =
        if !task_matches_lane {
            (
            format!(
                "latest lane task `{}` does not match requested task `{}`",
                status.task_id, task.id
            ),
            vec!["latest_lane_task_mismatch".to_string()],
            vec![format!(
                "Bind or inspect the correct bounded unit before local writes: `vida task show {} --json` and `vida lane show --latest --json`.",
                crate::shell_quote(&task.id)
            )],
            Some("vida lane show --latest --json".to_string()),
            Some("vida lane show".to_string()),
        )
        } else if allowed {
            (
            "exception takeover is active for this task; local writes are lawful only inside listed paths"
                .to_string(),
            Vec::new(),
            Vec::new(),
            None,
            None,
        )
        } else if takeover_state
            == crate::release1_contracts::ExceptionTakeoverState::ReceiptRecorded
        {
            let command = summary.as_ref().and_then(|summary| {
                summary
                    .exception_path_receipt_id
                    .as_deref()
                    .map(|receipt_id| {
                        format!(
                            "vida lane supersede {} --receipt-id {} --json",
                            crate::shell_quote(&summary.run_id),
                            crate::shell_quote(receipt_id)
                        )
                    })
            });
            (
            "exception receipt is recorded but supersession is required before local write is active"
                .to_string(),
            vec!["supersession_required".to_string()],
            command
                .iter()
                .cloned()
                .chain([
                    "Run `vida lane show --latest --json` if the receipt id is missing or stale."
                        .to_string(),
                ])
                .collect(),
            command,
            Some("vida lane supersede".to_string()),
        )
        } else if takeover_state.is_active() && paths.is_empty() {
            (
                "exception takeover is active but receipt-bound owned_write_scope could not be read"
                    .to_string(),
                vec!["exception_takeover_scope_missing".to_string()],
                vec![format!(
                    "Inspect the lane receipt and exception metadata: `vida lane show {} --json`.",
                    crate::shell_quote(&status.run_id)
                )],
                Some(format!(
                    "vida lane show {} --json",
                    crate::shell_quote(&status.run_id)
                )),
                Some("vida lane show".to_string()),
            )
        } else {
            let command = format!(
                "vida lane takeover-ready {} --json",
                crate::shell_quote(&status.run_id)
            );
            (
                "exception takeover is not recorded for this task".to_string(),
                vec!["exception_takeover_not_recorded".to_string()],
                vec![command.clone()],
                Some(command),
                Some("vida lane takeover-ready".to_string()),
            )
        };
    let packet = serde_json::json!({
        "dispatch_packet_path": summary
            .as_ref()
            .and_then(|summary| summary.dispatch_packet_path.clone()),
        "dispatch_result_path": summary
            .as_ref()
            .and_then(|summary| summary.dispatch_result_path.clone()),
        "downstream_dispatch_packet_path": summary
            .as_ref()
            .and_then(|summary| summary.downstream_dispatch_packet_path.clone()),
        "downstream_dispatch_result_path": summary
            .as_ref()
            .and_then(|summary| summary.downstream_dispatch_result_path.clone()),
    });
    let lane = serde_json::json!({
        "source": lane_source,
        "run_id": status.run_id,
        "task_id": status.task_id,
        "dispatch_target": summary.as_ref().map(|summary| summary.dispatch_target.clone()),
        "lane_status": summary.as_ref().map(|summary| summary.lane_status.clone()),
        "dispatch_status": summary.as_ref().map(|summary| summary.dispatch_status.clone()),
        "selected_backend": summary.as_ref().and_then(|summary| summary.selected_backend.clone()),
        "exception_path_receipt_id": summary.as_ref().and_then(|summary| summary.exception_path_receipt_id.clone()),
        "supersedes_receipt_id": summary.as_ref().and_then(|summary| summary.supersedes_receipt_id.clone()),
        "exception_path_metadata_path": summary
            .as_ref()
            .and_then(|summary| task_exception_takeover_metadata_path(store.root(), &summary.run_id).ok())
            .map(|path| path.display().to_string()),
        "recovery_gate": recovery_gate,
    });
    let takeover_ready_state = if allowed {
        "active"
    } else if takeover_state == crate::release1_contracts::ExceptionTakeoverState::ReceiptRecorded {
        "supersession_required"
    } else if task_matches_lane {
        "not_ready"
    } else {
        "stale_task_blocked"
    }
    .to_string();
    let root_write_guard = serde_json::json!({
        "status": if root_local_write_allowed { "exception_takeover_active" } else { "blocked_by_default" },
        "root_local_write_allowed": root_local_write_allowed,
        "root_local_write_allowed_for_only_these_paths": if root_local_write_allowed { paths.clone() } else { Vec::<String>::new() },
        "local_exception_takeover_state": state_label.clone(),
        "latest_lane_status": summary.as_ref().map(|summary| summary.lane_status.clone()),
        "local_exception_takeover_gate": recovery_gate,
        "latest_run_graph_task_stale": !task_matches_lane,
        "reason": if root_local_write_allowed { serde_json::Value::Null } else { serde_json::json!(reason.clone()) },
    });

    TaskTakeoverStatusReceipt {
        surface: "vida task takeover status",
        status: if allowed {
            task_json_success_status().to_string()
        } else {
            "blocked".to_string()
        },
        task_id: task.id.clone(),
        allowed,
        local_exception_takeover_state: state_label.clone(),
        root_local_write_allowed,
        paths,
        packet,
        lane,
        root_write_guard,
        active_takeover_state: if task_matches_lane {
            state_label.clone()
        } else {
            "stale_task_blocked".to_string()
        },
        takeover_ready_state,
        recommended_surface,
        reason,
        recommended_command,
        next_actions,
        blocker_codes,
    }
}

fn print_task_takeover_status(render: RenderMode, receipt: &TaskTakeoverStatusReceipt) {
    print_surface_header(render, "vida task takeover status");
    print_surface_line(render, "status", &receipt.status);
    print_surface_line(render, "task", &receipt.task_id);
    print_surface_line(render, "allowed", &receipt.allowed.to_string());
    print_surface_line(
        render,
        "takeover state",
        &receipt.local_exception_takeover_state,
    );
    print_surface_line(
        render,
        "root local write",
        &receipt.root_local_write_allowed.to_string(),
    );
    if let Some(command) = receipt.recommended_command.as_deref() {
        print_surface_line(render, "recommended command", command);
    }
}

fn normalize_task_block_list(values: &[String]) -> Vec<String> {
    parse_label_values(values)
}

fn append_task_block_note(
    existing_notes: Option<&str>,
    reason: &str,
    evidence: Option<&str>,
    blocker_codes: &[String],
    next_actions: &[String],
) -> String {
    let mut note = format!(
        "task_block:\n  recorded_at_unix_nanos: {}\n  reason: {}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
        reason.trim()
    );
    if let Some(evidence) = evidence.map(str::trim).filter(|value| !value.is_empty()) {
        note.push_str("\n  evidence: ");
        note.push_str(evidence);
    }
    if !blocker_codes.is_empty() {
        note.push_str("\n  blocker_codes: ");
        note.push_str(&blocker_codes.join(", "));
    }
    if !next_actions.is_empty() {
        note.push_str("\n  next_actions:");
        for action in next_actions {
            note.push_str("\n    - ");
            note.push_str(action.trim());
        }
    }

    match existing_notes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    }
}

fn print_task_block_receipt(render: RenderMode, receipt: &TaskBlockReceipt, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(receipt).expect("task block receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "reason", &receipt.reason);
    if !receipt.blocker_codes.is_empty() {
        print_surface_line(render, "blocker codes", &receipt.blocker_codes.join(", "));
    }
    if !receipt.next_actions.is_empty() {
        print_surface_line(render, "next actions", &receipt.next_actions.join(" | "));
    }
}

fn normalized_task_verify_evidence(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn append_task_verify_note(
    existing_notes: Option<&str>,
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
) -> String {
    let mut note = format!(
        "task_partial_verification:\n  recorded_at_unix_nanos: {}\n  source_fixed: {}\n  tests_green: {}\n  proof_blocked: {}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
        source_fixed,
        tests_green,
        proof_blocked
    );
    if let Some(proof_blocker) = proof_blocker
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  proof_blocker: ");
        note.push_str(proof_blocker);
    }
    if !evidence.is_empty() {
        note.push_str("\n  evidence:");
        for item in evidence {
            note.push_str("\n    - ");
            note.push_str(item);
        }
    }

    match existing_notes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    }
}

fn task_verify_labels(source_fixed: bool, tests_green: bool, proof_blocked: bool) -> Vec<String> {
    let mut labels = Vec::new();
    if source_fixed {
        labels.push("source-fixed".to_string());
    }
    if tests_green {
        labels.push("tests-green".to_string());
    }
    if proof_blocked {
        labels.push("proof-blocked-by-runtime".to_string());
    }
    labels
}

fn task_verify_planner_metadata(
    existing: &state_store::TaskPlannerMetadata,
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
) -> Option<state_store::TaskPlannerMetadata> {
    if !proof_blocked || !existing.proof_targets.is_empty() {
        return None;
    }
    let mut metadata = existing.clone();
    if evidence.is_empty() {
        if let Some(proof_blocker) = proof_blocker
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            metadata.proof_targets.push(proof_blocker.to_string());
        }
    } else {
        metadata.proof_targets.extend(evidence.iter().cloned());
    }
    if metadata.proof_targets.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

fn browser_proof_note_scalar(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn append_task_browser_proof_note(
    existing_notes: Option<&str>,
    proof_target: &str,
    route: &str,
    result: &str,
    expect: Option<&str>,
    screenshot: Option<&str>,
    evidence: &[String],
) -> String {
    let proof_target = browser_proof_note_scalar(proof_target);
    let route = browser_proof_note_scalar(route);
    let result = browser_proof_note_scalar(result);
    let mut note = format!(
        "task_browser_proof:\n  recorded_at_unix_nanos: {}\n  proof_target: {}\n  command: {}\n  route: {}\n  result: {}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
        proof_target, proof_target, route, result
    );
    if let Some(expect) = expect
        .map(browser_proof_note_scalar)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  expect: ");
        note.push_str(&expect);
    }
    if let Some(screenshot) = screenshot
        .map(browser_proof_note_scalar)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  screenshot: ");
        note.push_str(&screenshot);
    }
    let evidence = evidence
        .iter()
        .map(|value| browser_proof_note_scalar(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !evidence.is_empty() {
        note.push_str("\n  evidence: ");
        note.push_str(&evidence.join(" | "));
    }

    match existing_notes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    }
}

fn task_browser_proof_planner_metadata(
    existing: &state_store::TaskPlannerMetadata,
    proof_target: &str,
) -> state_store::TaskPlannerMetadata {
    let mut metadata = existing.clone();
    if !metadata
        .proof_targets
        .iter()
        .any(|target| target.trim() == proof_target.trim())
    {
        metadata.proof_targets.push(proof_target.trim().to_string());
    }
    metadata
}

fn print_task_verify_receipt(render: RenderMode, receipt: &TaskVerifyReceipt, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(receipt).expect("task verify receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "partial", &receipt.partial.to_string());
    print_surface_line(render, "source fixed", &receipt.source_fixed.to_string());
    print_surface_line(render, "tests green", &receipt.tests_green.to_string());
    print_surface_line(render, "proof blocked", &receipt.proof_blocked.to_string());
}

fn print_task_browser_proof_receipt(
    render: RenderMode,
    receipt: &TaskProofAttachBrowserReceipt,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(receipt)
            .expect("task browser proof receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "route", &receipt.route);
    print_surface_line(render, "result", &receipt.result);
    print_surface_line(render, "proof target", &receipt.proof_target);
    if let Some(screenshot) = receipt.screenshot.as_deref() {
        print_surface_line(render, "screenshot", screenshot);
    }
}

fn task_import_jsonl_error_payload(path: &str, error: &str) -> serde_json::Value {
    let blocker_codes = vec![crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::DependencyGraphIssues,
    )
    .unwrap_or_else(|| "dependency_graph_issues".to_string())];
    let next_actions = vec![
        "Repair the JSONL dependency graph issues, then rerun `vida task import-jsonl <path> --json`."
            .to_string(),
    ];
    let artifact_refs = serde_json::json!({
        "surface": "vida task import-jsonl",
        "source_path": path,
    });
    let shared_fields = serde_json::json!({
        "status": crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status,
        "trace_id": serde_json::Value::Null,
        "workflow_class": serde_json::Value::Null,
        "risk_tier": serde_json::Value::Null,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
    });
    let operator_contracts = serde_json::json!({
        "contract_id": crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC.contract_id,
        "schema_version": crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC.schema_version,
        "status": shared_fields["status"],
        "trace_id": serde_json::Value::Null,
        "workflow_class": serde_json::Value::Null,
        "risk_tier": serde_json::Value::Null,
        "blocker_codes": shared_fields["blocker_codes"],
        "next_actions": shared_fields["next_actions"],
        "artifact_refs": shared_fields["artifact_refs"],
    });
    serde_json::json!({
        "status": crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status,
        "surface": "vida task import-jsonl",
        "trace_id": serde_json::Value::Null,
        "workflow_class": serde_json::Value::Null,
        "risk_tier": serde_json::Value::Null,
        "source_path": path,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "error": error,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "artifact_refs": artifact_refs,
    })
}

fn task_next_lawful_projection_name() -> &'static str {
    "task-next-lawful-latest"
}

fn safe_task_projection_component(value: &str) -> String {
    let mut safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    safe.truncate(160);
    if safe.is_empty() {
        "unknown".to_string()
    } else {
        safe
    }
}

fn task_show_projection_name(task_id: &str) -> String {
    format!(
        "task-show-{}-latest",
        safe_task_projection_component(task_id)
    )
}

fn task_ready_projection_name(scope_task_id: Option<&str>) -> String {
    format!(
        "task-ready-scope-{}-latest",
        safe_task_projection_component(scope_task_id.unwrap_or("default"))
    )
}

const TASK_READ_RECENT_PROJECTION_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(300);

fn task_update_graph_issue_from_invalid_record_reason(
    reason: &str,
) -> Option<state_store::TaskGraphIssue> {
    let rest = reason.strip_prefix("task update would create invalid graph: ")?;
    let (issue_type, issue_id) = rest.split_once(" on ")?;
    if issue_type != "open_parent_has_no_open_child" || issue_id.trim().is_empty() {
        return None;
    }
    Some(state_store::TaskGraphIssue {
        issue_type: issue_type.to_string(),
        issue_id: issue_id.to_string(),
        depends_on_id: None,
        edge_type: Some("parent-child".to_string()),
        detail: "open or in-progress parent has no direct non-closed child".to_string(),
    })
}

fn canonical_json_string_array_entries(value: &serde_json::Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

fn normalize_task_json_contract_arrays(summary_json: &mut serde_json::Value) -> Result<(), String> {
    let Some(summary) = summary_json.as_object_mut() else {
        return Ok(());
    };
    for key in ["blocker_codes", "next_actions"] {
        if let Some(value) = summary.get(key) {
            let entries = canonical_json_string_array_entries(value).ok_or_else(|| {
                format!(
                    "task json contract inconsistency: `{key}` must contain canonical nonempty string entries"
                )
            })?;
            summary.insert(key.to_string(), serde_json::json!(entries));
        }
    }
    Ok(())
}

async fn open_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    if state_dir.exists() {
        StateStore::open_existing(state_dir).await
    } else {
        StateStore::open(state_dir).await
    }
}

pub(crate) async fn open_read_only_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    StateStore::open_existing_read_only(state_dir).await
}

fn is_authoritative_state_lock_error(error: &state_store::StateStoreError) -> bool {
    StateStore::error_is_lock_contention(error)
}

fn load_task_snapshot_rows(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path)
}

pub(crate) async fn load_task_snapshot_rows_with_retry(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    for attempt in 0..80 {
        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
            Ok(rows) => return Ok(rows),
            Err(error @ state_store::StateStoreError::Io(_)) if attempt < 79 => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }
    load_task_snapshot_rows(state_dir)
}

async fn load_task_snapshot_rows_fallback_with_metadata(
    state_dir: &std::path::Path,
    snapshot_path: &std::path::Path,
    detail: &'static str,
    authoritative_error: state_store::StateStoreError,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    match load_task_snapshot_rows_with_retry(state_dir).await {
        Ok(rows) => Ok((rows, TaskReadMetadata::snapshot(snapshot_path, detail))),
        Err(state_store::StateStoreError::Io(_)) => Err(authoritative_error),
        Err(snapshot_error) => Err(snapshot_error),
    }
}

async fn load_task_snapshot_rows_authoritative_first(
    state_dir: &std::path::Path,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    match open_read_only_task_store(state_dir.to_path_buf()).await {
        Ok(store) => match store.list_tasks(None, true).await {
            Ok(rows) => Ok((rows, TaskReadMetadata::authoritative_live())),
            Err(error) if is_authoritative_state_lock_error(&error) => {
                load_task_snapshot_rows_fallback_with_metadata(
                    state_dir,
                    &snapshot_path,
                    "served from canonical task snapshot evidence after authoritative state lock contention",
                    error,
                )
                .await
            }
            Err(error) => Err(error),
        },
        Err(error @ state_store::StateStoreError::MissingStateDir(_)) => {
            load_task_snapshot_rows_fallback_with_metadata(
                state_dir,
                &snapshot_path,
                "served from canonical task snapshot evidence because authoritative state store is missing",
                error,
            )
            .await
        }
        Err(error) if is_authoritative_state_lock_error(&error) => {
            load_task_snapshot_rows_fallback_with_metadata(
                state_dir,
                &snapshot_path,
                "served from canonical task snapshot evidence after authoritative state lock contention",
                error,
            )
            .await
        }
        Err(error) => Err(error),
    }
}

fn resolve_task_from_rows(
    rows: &[state_store::TaskRecord],
    task_id_or_display_id: &str,
) -> Result<state_store::TaskRecord, state_store::StateStoreError> {
    if let Some(task) = rows.iter().find(|task| task.id == task_id_or_display_id) {
        return Ok(task.clone());
    }
    if let Some(task) = rows
        .iter()
        .find(|task| task.display_id.as_deref() == Some(task_id_or_display_id))
    {
        return Ok(task.clone());
    }
    Err(state_store::StateStoreError::MissingTask {
        task_id: task_id_or_display_id.to_string(),
    })
}

async fn refresh_task_snapshot_after_mutation(
    store: &StateStore,
    surface: &str,
) -> Result<(), ExitCode> {
    crate::operator_projection_cache::touch_state_mutation_marker(store.root());
    StateStore::touch_task_snapshot_state_marker(store.root());
    store
        .refresh_task_snapshot()
        .await
        .map(|_| ())
        .map_err(|error| {
            eprintln!("Failed to refresh canonical task snapshot after {surface}: {error}");
            ExitCode::from(1)
        })
}

async fn refresh_task_snapshot_for_task_after_mutation(
    store: &StateStore,
    task: &state_store::TaskRecord,
    surface: &str,
) -> Result<(), ExitCode> {
    crate::operator_projection_cache::touch_state_mutation_marker(store.root());
    StateStore::touch_task_snapshot_state_marker(store.root());
    store
        .refresh_task_snapshot_for_task(task)
        .await
        .map(|_| ())
        .map_err(|error| {
            eprintln!("Failed to refresh canonical task snapshot after {surface}: {error}");
            ExitCode::from(1)
        })
}

pub(crate) async fn ready_tasks_scoped_read_only(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.ready_tasks_scoped(scope_task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn task_dependency_tree_read_only(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<state_store::TaskDependencyTreeNode, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.task_dependency_tree(task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::task_dependency_tree_from_rows(&rows, task_id)
        }
        Err(error) => Err(error),
    }
}

fn parse_task_dependency_bulk_edge(
    raw: &str,
) -> Result<state_store::TaskDependencyBulkAddInput, String> {
    let parts = raw.split(':').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(format!(
            "invalid bulk dependency edge `{raw}`; expected issue_id:depends_on_id:edge_type"
        ));
    }
    Ok(state_store::TaskDependencyBulkAddInput {
        issue_id: parts[0].to_string(),
        depends_on_id: parts[1].to_string(),
        edge_type: parts[2].to_string(),
    })
}

fn task_dependency_bulk_edge_inputs(
    inline_edges: &[String],
    edge_file: Option<&std::path::Path>,
) -> Result<Vec<state_store::TaskDependencyBulkAddInput>, String> {
    let mut raw_edges = inline_edges.to_vec();
    if let Some(path) = edge_file {
        let content = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read dependency edge file `{}`: {error}",
                path.display()
            )
        })?;
        raw_edges.extend(
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(ToOwned::to_owned),
        );
    }
    if raw_edges.is_empty() {
        return Err("at least one --edge or --edge-file entry is required".to_string());
    }
    raw_edges
        .iter()
        .map(|edge| parse_task_dependency_bulk_edge(edge))
        .collect()
}

async fn task_list_authoritative_first(
    state_dir: std::path::PathBuf,
    status: Option<&str>,
    include_all: bool,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let filtered = rows
        .into_iter()
        .filter(|task| include_all || task.status != "closed")
        .filter(|task| status.map(|wanted| task.status == wanted).unwrap_or(true))
        .collect();
    Ok((filtered, metadata))
}

async fn task_show_authoritative_first(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<(state_store::TaskRecord, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let task = resolve_task_from_rows(&rows, task_id)?;
    Ok((task, metadata))
}

async fn task_ready_authoritative_first(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let tasks = StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)?;
    Ok((tasks, metadata))
}

async fn task_critical_path_snapshot_first(
    state_dir: std::path::PathBuf,
) -> Result<state_store::TaskCriticalPath, state_store::StateStoreError> {
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => store.critical_path().await,
        Err(error @ state_store::StateStoreError::MissingStateDir(_)) => {
            match load_task_snapshot_rows_with_retry(&state_dir).await {
                Ok(rows) => StateStore::critical_path_from_rows(&rows),
                Err(state_store::StateStoreError::Io(_)) => Err(error),
                Err(snapshot_error) => Err(snapshot_error),
            }
        }
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::critical_path_from_rows(&rows)
        }
        Err(error) => Err(error),
    }
}

fn task_rows_as_values(
    tasks: &[state_store::TaskRecord],
) -> Result<Vec<serde_json::Value>, String> {
    tasks
        .iter()
        .map(|task| serde_json::to_value(task).map_err(|error| error.to_string()))
        .collect()
}

fn task_close_epic_progress_summary(
    rows: &[state_store::TaskRecord],
    closed_task_id: &str,
    include_global_progress: bool,
) -> Result<TaskCloseEpicProgressSummary, state_store::StateStoreError> {
    let task_by_id = rows
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut children_by_parent =
        std::collections::BTreeMap::<String, Vec<&state_store::TaskRecord>>::new();
    for task in rows {
        if let Some(parent_id) = task_parent_id(task) {
            children_by_parent.entry(parent_id).or_default().push(task);
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.status.cmp(&right.status))
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let scoped_epic_ids = task_close_scoped_epic_ids(rows, closed_task_id);
    let total_epic_count = rows.iter().filter(|task| task.issue_type == "epic").count();
    let mut epics = rows
        .iter()
        .filter(|task| task.issue_type == "epic")
        .filter(|task| include_global_progress || scoped_epic_ids.contains(&task.id))
        .collect::<Vec<_>>();
    epics.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.status.cmp(&right.status))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut epic_rows = Vec::with_capacity(epics.len());
    for epic in epics {
        let progress = StateStore::task_progress_summary_from_rows(rows, &epic.id)?;
        let children = children_by_parent
            .get(&epic.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let tasks = children
            .iter()
            .take(TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT)
            .map(|task| task_close_epic_progress_task_row(task, &task_by_id))
            .collect::<Vec<_>>();
        epic_rows.push(TaskCloseEpicProgressRow {
            epic_id: epic.id.clone(),
            epic_title: epic.title.clone(),
            epic_status: epic.status.clone(),
            epic_priority: epic.priority,
            closed_count: progress.closed_count,
            total_count: progress.descendant_count,
            percent_closed: progress.percent_closed,
            child_task_count: children.len(),
            reported_child_task_count: tasks.len(),
            child_task_report_limit: TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT,
            truncated_child_tasks: children.len() > TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT,
            tasks,
        });
    }

    Ok(TaskCloseEpicProgressSummary {
        closed_task_id: closed_task_id.to_string(),
        epic_count: epic_rows.len(),
        reported_epic_count: epic_rows.len(),
        omitted_epic_count: total_epic_count.saturating_sub(epic_rows.len()),
        scope: if include_global_progress {
            "all_epics"
        } else {
            "closed_task_ancestor_epics"
        }
        .to_string(),
        epics: epic_rows,
    })
}

fn task_close_scoped_epic_ids(
    rows: &[state_store::TaskRecord],
    closed_task_id: &str,
) -> std::collections::BTreeSet<String> {
    let by_id = rows
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut scoped = std::collections::BTreeSet::new();
    let mut current_id = Some(closed_task_id.to_string());
    let mut visited = std::collections::BTreeSet::new();
    while let Some(task_id) = current_id {
        if !visited.insert(task_id.clone()) {
            break;
        }
        let Some(task) = by_id.get(task_id.as_str()) else {
            break;
        };
        if task.issue_type == "epic" {
            scoped.insert(task.id.clone());
        }
        current_id = task_parent_id(task);
    }
    scoped
}

fn task_epic_progress_summary(
    rows: &[state_store::TaskRecord],
    metadata: TaskReadMetadata,
    include_closed_epics: bool,
    epic_filter: Option<&str>,
    basis: &str,
) -> Result<TaskEpicProgressSummary, state_store::StateStoreError> {
    if let Some(epic_id) = epic_filter {
        let Some(epic) = rows.iter().find(|task| task.id == epic_id) else {
            return Err(state_store::StateStoreError::MissingTask {
                task_id: epic_id.to_string(),
            });
        };
        if epic.issue_type != "epic" {
            return Err(state_store::StateStoreError::InvalidTaskRecord {
                reason: format!("task `{epic_id}` is not an epic"),
            });
        }
    }
    let mut epics = rows
        .iter()
        .filter(|task| task.issue_type == "epic")
        .filter(|task| {
            epic_filter
                .map(|epic_id| task.id == epic_id)
                .unwrap_or(true)
        })
        .filter(|task| {
            include_closed_epics || matches!(task.status.as_str(), "open" | "in_progress")
        })
        .collect::<Vec<_>>();
    epics.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.status.cmp(&right.status))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut open_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut closed_count = 0usize;
    let mut total_descendant_count = 0usize;
    let mut total_open_descendant_count = 0usize;
    let mut total_in_progress_descendant_count = 0usize;
    let mut total_closed_descendant_count = 0usize;
    let mut epic_rows = Vec::with_capacity(epics.len());

    for epic in epics {
        match epic.status.as_str() {
            "open" => open_count += 1,
            "in_progress" => in_progress_count += 1,
            "closed" => closed_count += 1,
            _ => {}
        }

        let progress = task_progress_summary_for_basis(rows, &epic.id, basis)?;
        total_descendant_count += progress.descendant_count;
        total_open_descendant_count += progress.open_count;
        total_in_progress_descendant_count += progress.in_progress_count;
        total_closed_descendant_count += progress.closed_count;

        epic_rows.push(TaskEpicProgressRow {
            epic_id: epic.id.clone(),
            epic_title: epic.title.clone(),
            epic_status: epic.status.clone(),
            epic_priority: epic.priority,
            total_count: progress.descendant_count,
            open_count: progress.open_count,
            in_progress_count: progress.in_progress_count,
            closed_count: progress.closed_count,
            percent_complete: progress.percent_closed,
            direct_child_count: progress.direct_child_count,
            nested_epic_count: progress.epic_count,
            closure_candidate: progress.closure_candidate,
            closure_candidate_state: progress.closure_candidate_state,
            recommended_next_action: progress.recommended_next_action,
        });
    }

    let percent_closed = if total_descendant_count == 0 {
        0.0
    } else {
        (total_closed_descendant_count as f64 / total_descendant_count as f64) * 100.0
    };

    Ok(TaskEpicProgressSummary {
        epic_count: epic_rows.len(),
        open_count,
        in_progress_count,
        closed_count,
        total_descendant_count,
        total_open_descendant_count,
        total_in_progress_descendant_count,
        total_closed_descendant_count,
        percent_closed,
        include_closed_epics,
        progress_basis: basis.to_string(),
        epic_filter: epic_filter.map(ToOwned::to_owned),
        epics: epic_rows,
        read_metadata: metadata,
    })
}

fn task_progress_basis_arg(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "" | "descendants" | "descendants_excluding_root" => Ok("descendants_excluding_root"),
        "direct-children" | "direct_children" | "children" => Ok("direct_children"),
        other => Err(format!(
            "unsupported progress basis `{other}`; expected descendants or direct-children"
        )),
    }
}

fn task_progress_summary_for_basis(
    rows: &[state_store::TaskRecord],
    task_id: &str,
    basis: &str,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    match basis {
        "direct_children" => task_direct_child_progress_summary_from_rows(rows, task_id),
        _ => StateStore::task_progress_summary_from_rows(rows, task_id),
    }
}

fn task_direct_child_progress_summary_from_rows(
    rows: &[state_store::TaskRecord],
    task_id: &str,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    let root_task = rows
        .iter()
        .find(|task| task.id == task_id)
        .cloned()
        .ok_or_else(|| state_store::StateStoreError::MissingTask {
            task_id: task_id.to_string(),
        })?;
    let children = rows
        .iter()
        .filter(|task| task_parent_id(task).as_deref() == Some(task_id))
        .collect::<Vec<_>>();
    let mut status_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut open_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut closed_count = 0usize;
    let mut epic_count = 0usize;

    for task in &children {
        *status_counts.entry(task.status.clone()).or_insert(0) += 1;
        match task.status.as_str() {
            "open" => open_count += 1,
            "in_progress" => in_progress_count += 1,
            "closed" => closed_count += 1,
            _ => {}
        }
        if task.issue_type == "epic" {
            epic_count += 1;
        }
    }

    let descendant_count = children.len();
    let percent_closed = if descendant_count == 0 {
        0.0
    } else {
        (closed_count as f64 / descendant_count as f64) * 100.0
    };
    let root_closed = StateStore::task_status_is_closed_like(&root_task.status);
    let all_children_closed_like = children
        .iter()
        .all(|task| StateStore::task_status_is_closed_like(&task.status));
    let closure_candidate = root_task.issue_type == "epic"
        && !root_closed
        && descendant_count > 0
        && all_children_closed_like;
    let next_required_command = if closure_candidate {
        Some(format!(
            "vida task close {} --reason \"direct children closed\" --json",
            crate::launcher_task_commands::shell_quote(&root_task.id)
        ))
    } else if descendant_count == 0 {
        Some("Add child work items or close with an explicit operator reason.".to_string())
    } else if !all_children_closed_like {
        Some("Continue or close remaining direct children before closing the parent.".to_string())
    } else {
        None
    };
    let recommended_next_action = next_required_command.clone().unwrap_or_else(|| {
        "No action; task is already closed or has no direct-child blocker.".to_string()
    });

    Ok(state_store::TaskProgressSummary {
        root_task,
        progress_basis: "direct_children".to_string(),
        direct_child_count: descendant_count,
        descendant_count,
        open_count,
        in_progress_count,
        closed_count,
        epic_count,
        status_counts,
        percent_closed,
        closure_candidate,
        closure_candidate_state: if closure_candidate {
            "ready_to_close".to_string()
        } else if root_closed {
            "already_closed".to_string()
        } else if descendant_count == 0 {
            "container_without_direct_children".to_string()
        } else {
            "direct_children_remaining".to_string()
        },
        closure_candidate_reason: Some("direct-child basis selected by operator".to_string()),
        ready_for_close: closure_candidate,
        missing_proof: false,
        proof_blocked_by_runtime: false,
        blocked_by_runtime: false,
        next_required_command,
        recommended_next_action,
        canonical_commands: Vec::new(),
    })
}

fn task_close_epic_progress_task_row(
    task: &state_store::TaskRecord,
    task_by_id: &std::collections::BTreeMap<&str, &state_store::TaskRecord>,
) -> TaskCloseEpicProgressTaskRow {
    let blockers = task_close_progress_blockers(task, task_by_id);
    let blocker_state = if blockers.is_empty() {
        "clear"
    } else {
        "blocked"
    };
    let next_action = task_close_progress_next_action(task, &blockers);
    TaskCloseEpicProgressTaskRow {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
        blocker_state: blocker_state.to_string(),
        blockers,
        next_action,
    }
}

fn task_close_progress_blockers(
    task: &state_store::TaskRecord,
    task_by_id: &std::collections::BTreeMap<&str, &state_store::TaskRecord>,
) -> Vec<TaskCloseEpicProgressBlocker> {
    task.dependencies
        .iter()
        .filter(|dependency| dependency.edge_type == "blocks")
        .filter_map(
            |dependency| match task_by_id.get(dependency.depends_on_id.as_str()) {
                Some(blocker) if !StateStore::task_status_is_closed_like(&blocker.status) => {
                    Some(TaskCloseEpicProgressBlocker {
                        task_id: blocker.id.clone(),
                        status: blocker.status.clone(),
                        title: Some(blocker.title.clone()),
                    })
                }
                Some(_) => None,
                None => Some(TaskCloseEpicProgressBlocker {
                    task_id: dependency.depends_on_id.clone(),
                    status: "missing".to_string(),
                    title: None,
                }),
            },
        )
        .collect()
}

fn task_close_progress_next_action(
    task: &state_store::TaskRecord,
    blockers: &[TaskCloseEpicProgressBlocker],
) -> String {
    if !blockers.is_empty() {
        let blocker_ids = blockers
            .iter()
            .map(|blocker| blocker.task_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "Resolve blocking tasks before closing `{}`: {blocker_ids}",
            task.id
        );
    }
    if StateStore::task_status_is_closed_like(&task.status) {
        return "No action; task is already closed.".to_string();
    }
    if task.issue_type == "epic" {
        return format!(
            "Inspect nested epic progress with `vida task progress {} --json`.",
            task.id
        );
    }
    format!(
        "Continue `{}` or close it after proof is complete.",
        task.id
    )
}

fn task_close_automation_is_blocked(automation: Option<&TaskCloseAutomationReceipt>) -> bool {
    automation
        .map(|receipt| receipt.status != "pass")
        .unwrap_or(false)
}

fn task_close_result_payload(
    task: &state_store::TaskRecord,
    telemetry: &serde_json::Value,
    automation: Option<&TaskCloseAutomationReceipt>,
    telemetry_feedback_blocker: Option<&(Vec<String>, Vec<String>)>,
    epic_progress_summary: Option<&TaskCloseEpicProgressSummary>,
) -> serde_json::Value {
    let automation_blocked = task_close_automation_is_blocked(automation);
    let feedback_blocked = telemetry_feedback_blocker.is_some();
    let blocker_codes = if let Some((blocker_codes, _)) = telemetry_feedback_blocker {
        blocker_codes.clone()
    } else {
        automation
            .map(|receipt| receipt.blocker_codes.clone())
            .unwrap_or_default()
    };
    let next_actions = if let Some((_, next_actions)) = telemetry_feedback_blocker {
        next_actions.clone()
    } else {
        automation
            .map(|receipt| receipt.next_actions.clone())
            .unwrap_or_default()
    };
    let continuation_blocked = automation_blocked || feedback_blocked;
    let status = if automation_blocked {
        "blocked"
    } else {
        "pass"
    };
    serde_json::json!({
        "status": status,
        "closed": true,
        "continuation_blocked": continuation_blocked,
        "automation_blocked": automation_blocked,
        "feedback_blocked": feedback_blocked,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "task": task,
        "host_agent_telemetry": telemetry,
        "automation": automation,
        "epic_progress_summary": epic_progress_summary,
    })
}

fn print_task_close_epic_progress_summary(
    render: RenderMode,
    summary: &TaskCloseEpicProgressSummary,
) {
    print_surface_line(
        render,
        "epic progress",
        &format!(
            "{} scoped epics after closing {} ({} omitted)",
            summary.reported_epic_count, summary.closed_task_id, summary.omitted_epic_count
        ),
    );
    for epic in &summary.epics {
        print_surface_line(
            render,
            &format!("epic {}", epic.epic_id),
            &format!(
                "{}/{} closed ({:.2}%)",
                epic.closed_count, epic.total_count, epic.percent_closed
            ),
        );
    }
}

fn print_task_epic_progress_summary(
    render: RenderMode,
    summary: &TaskEpicProgressSummary,
    as_json: bool,
) {
    let payload = crate::task_cli_render::build_pass_operator_surface_payload(
        "vida task progress --epics",
        serde_json::json!({
            "epic_progress_summary": summary,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task epic progress should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task progress --epics");
    print_surface_line(render, "epics", &summary.epic_count.to_string());
    print_surface_line(render, "open epics", &summary.open_count.to_string());
    print_surface_line(
        render,
        "in progress epics",
        &summary.in_progress_count.to_string(),
    );
    print_surface_line(render, "closed epics", &summary.closed_count.to_string());
    print_surface_line(
        render,
        "descendants",
        &summary.total_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "open descendants",
        &summary.total_open_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "in progress descendants",
        &summary.total_in_progress_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "closed descendants",
        &summary.total_closed_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "percent complete",
        &format!("{:.2}%", summary.percent_closed),
    );
    for epic in &summary.epics {
        print_surface_line(
            render,
            &format!("epic {}", epic.epic_id),
            &format!(
                "{}: {}/{} closed ({:.2}%), open={}, in_progress={}",
                epic.epic_status,
                epic.closed_count,
                epic.total_count,
                epic.percent_complete,
                epic.open_count,
                epic.in_progress_count
            ),
        );
    }
}

fn project_root_for_task_state(state_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir)
        .or_else(|| crate::resolve_runtime_project_root().ok())
}

fn task_close_uses_isolated_state_dir(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
) -> bool {
    explicit_state_dir
        && crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir).is_none()
}

fn task_close_host_agent_telemetry(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
    project_root: Option<&std::path::Path>,
    task_value: &serde_json::Value,
    close_reason: &str,
    feedback_source: &str,
) -> serde_json::Value {
    if task_close_uses_isolated_state_dir(state_dir, explicit_state_dir) {
        return serde_json::json!({
            "status": "skipped",
            "reason": "isolated_state_dir",
            "state_dir": state_dir.display().to_string(),
            "feedback_store": "not_recorded",
        });
    }

    if let Some((canonical_status, canonical_gate)) =
        crate::agent_feedback_surface::canonical_close_status_from_reason(close_reason)
    {
        return serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "canonical_status": canonical_status,
            "canonical_gate": canonical_gate,
        });
    }

    match project_root {
        Some(project_root) => {
            crate::agent_feedback_surface::maybe_record_task_close_host_agent_feedback(
                project_root,
                task_value,
                close_reason,
                feedback_source,
            )
        }
        None => serde_json::json!({
            "status": "skipped",
            "reason": "project_root_unavailable",
        }),
    }
}

fn task_close_feedback_blocker_summary(
    telemetry: &serde_json::Value,
) -> Option<(Vec<String>, Vec<String>)> {
    let reason = telemetry
        .get("reason")
        .and_then(serde_json::Value::as_str)?;
    if reason != "feedback_deferred_for_canonical_close_status" {
        return None;
    }
    let canonical_status = telemetry
        .get("canonical_status")
        .and_then(serde_json::Value::as_str)?;
    let canonical_gate = telemetry
        .get("canonical_gate")
        .and_then(serde_json::Value::as_str)?;
    if canonical_status == "awaiting_approval" {
        return None;
    }
    let blocker_code = match canonical_status {
        "blocked" => "close_feedback_canonical_status_blocked",
        "awaiting_approval" => "close_feedback_canonical_status_awaiting_approval",
        _ => "close_feedback_canonical_status_deferred",
    };
    let next_action = match canonical_status {
        "blocked" => {
            "Resolve the blocked condition described in the close reason, then rerun `vida task close ... --json`."
        }
        "awaiting_approval" => {
            "Satisfy the approval requirement described in the close reason, then rerun `vida task close ... --json`."
        }
        _ => {
            "Resolve the deferred canonical close condition, then rerun `vida task close ... --json`."
        }
    };
    Some((
        vec![
            blocker_code.to_string(),
            format!("canonical_gate_{canonical_gate}"),
        ],
        vec![next_action.to_string()],
    ))
}

fn resolve_optional_text_arg(
    label: &str,
    direct: Option<&str>,
    file_path: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    const MAX_FILE_BYTES: u64 = 64 * 1024;

    if direct.is_some() && file_path.is_some() {
        return Err(format!(
            "Use only one {label} source: --{label} <text> or --{label}-file <path>"
        ));
    }
    if let Some(path) = file_path {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            format!(
                "Failed to inspect {label} file `{}` metadata: {error}",
                path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Refusing to read {label} file `{}`: symlinks are not allowed",
                path.display()
            ));
        }
        if !metadata.is_file() {
            return Err(format!(
                "Refusing to read {label} file `{}`: expected a regular file",
                path.display()
            ));
        }
        if metadata.len() > MAX_FILE_BYTES {
            return Err(format!(
                "Refusing to read {label} file `{}`: file is {} bytes, limit is {} bytes",
                path.display(),
                metadata.len(),
                MAX_FILE_BYTES
            ));
        }
        let value = std::fs::read_to_string(path).map_err(|error| {
            format!("Failed to read {label} file `{}`: {error}", path.display())
        })?;
        return Ok(Some(value));
    }
    Ok(direct.map(ToOwned::to_owned))
}

fn task_execution_semantics_from_create_args(
    command: &TaskCreateArgs,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: command.execution_mode.clone(),
        order_bucket: command.order_bucket.clone(),
        parallel_group: command.parallel_group.clone(),
        conflict_domain: command.conflict_domain.clone(),
    }
}

fn task_create_semantics_requested(command: &TaskCreateArgs) -> bool {
    command.execution_mode.is_some()
        || command.order_bucket.is_some()
        || command.parallel_group.is_some()
        || command.conflict_domain.is_some()
}

fn task_create_semantics_mismatch(
    existing: &state_store::TaskExecutionSemantics,
    command: &TaskCreateArgs,
) -> bool {
    command
        .execution_mode
        .as_deref()
        .is_some_and(|expected| existing.execution_mode.as_deref() != Some(expected))
        || command
            .order_bucket
            .as_deref()
            .is_some_and(|expected| existing.order_bucket.as_deref() != Some(expected))
        || command
            .parallel_group
            .as_deref()
            .is_some_and(|expected| existing.parallel_group.as_deref() != Some(expected))
        || command
            .conflict_domain
            .as_deref()
            .is_some_and(|expected| existing.conflict_domain.as_deref() != Some(expected))
}

fn task_update_semantics_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err(
            "Use either the value flag or the matching clear flag for execution semantics, not both."
                .to_string(),
        );
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn task_update_parent_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err("Use either --parent-id or --clear-parent-id, not both.".to_string());
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn parse_label_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
}

fn parse_proof_target_values(values: &[String]) -> Vec<String> {
    normalize_proof_target_commands(parse_label_values(values))
}

fn normalize_proof_target_commands(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|value| normalize_proof_target_command(&value))
        .collect()
}

fn normalize_proof_target_command(value: &str) -> Vec<String> {
    let command = normalize_stale_proof_target_command(value);
    split_cargo_test_proof_target(&command).unwrap_or_else(|| vec![command])
}

fn normalize_stale_proof_target_command(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed == "vida diagnostics --json" {
        return "vida diagnostics post-commit --json".to_string();
    }
    if trimmed.starts_with("vida docflow protocol-coverage-check ") {
        let mut tokens = trimmed.split_whitespace().peekable();
        let mut normalized = Vec::new();
        while let Some(token) = tokens.next() {
            if token == "--format" {
                let _ = tokens.next();
                continue;
            }
            normalized.push(token);
        }
        return normalized.join(" ");
    }
    trimmed.to_string()
}

fn split_cargo_test_proof_target(command: &str) -> Option<Vec<String>> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 4 || tokens[0] != "cargo" || tokens[1] != "test" {
        return None;
    }

    let separator_index = tokens
        .iter()
        .position(|token| *token == "--")
        .unwrap_or(tokens.len());
    let mut base = vec![tokens[0], tokens[1]];
    let mut filters = Vec::new();
    let mut index = 2;
    while index < separator_index {
        let token = tokens[index];
        if token.starts_with('-') {
            base.push(token);
            if cargo_test_option_takes_value(token) && index + 1 < separator_index {
                index += 1;
                base.push(tokens[index]);
            }
        } else {
            filters.push(token);
        }
        index += 1;
    }

    if filters.len() <= 1 {
        return None;
    }

    let tail = &tokens[separator_index..];
    Some(
        filters
            .into_iter()
            .map(|filter| {
                base.iter()
                    .chain(std::iter::once(&filter))
                    .chain(tail.iter())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect(),
    )
}

fn cargo_test_option_takes_value(option: &str) -> bool {
    matches!(
        option,
        "-p" | "--package"
            | "--exclude"
            | "--features"
            | "--target"
            | "--target-dir"
            | "--manifest-path"
            | "--message-format"
            | "--profile"
            | "--jobs"
            | "-j"
    )
}

fn parse_optional_label_value(value: Option<&str>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.to_string())
            .collect::<Vec<_>>()
    })
}

fn task_update_planner_metadata_requested(command: &crate::TaskUpdateArgs) -> bool {
    !command.owned_paths.is_empty()
        || !command.acceptance_targets.is_empty()
        || !command.proof_targets.is_empty()
}

fn task_create_planner_metadata_arg(command: &TaskCreateArgs) -> state_store::TaskPlannerMetadata {
    state_store::TaskPlannerMetadata {
        owned_paths: parse_label_values(&command.owned_paths),
        acceptance_targets: parse_label_values(&command.acceptance_targets),
        proof_targets: parse_proof_target_values(&command.proof_targets),
        ..state_store::TaskPlannerMetadata::default()
    }
}

fn task_update_planner_metadata_arg(
    existing: &state_store::TaskPlannerMetadata,
    command: &crate::TaskUpdateArgs,
) -> Option<state_store::TaskPlannerMetadata> {
    if !task_update_planner_metadata_requested(command) {
        return None;
    }
    let mut metadata = existing.clone();
    let owned_paths = parse_label_values(&command.owned_paths);
    if !owned_paths.is_empty() {
        metadata.owned_paths = owned_paths;
    }
    let acceptance_targets = parse_label_values(&command.acceptance_targets);
    if !acceptance_targets.is_empty() {
        metadata.acceptance_targets = acceptance_targets;
    }
    let proof_targets = parse_label_values(&command.proof_targets);
    if !proof_targets.is_empty() {
        metadata.proof_targets = normalize_proof_target_commands(proof_targets);
    }
    Some(metadata)
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationPlannedTask {
    task_id: String,
    title: String,
    description: String,
    issue_type: String,
    status: String,
    priority: u32,
    parent_id: Option<String>,
    labels: Vec<String>,
    execution_semantics: state_store::TaskExecutionSemantics,
    planner_metadata: state_store::TaskPlannerMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationPlannedDependency {
    issue_id: String,
    depends_on_id: String,
    edge_type: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationValidationSummary {
    status: String,
    issue_count: usize,
    blocker_codes: Vec<String>,
    issues: Vec<state_store::TaskGraphIssue>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskGraphMutationValidationReceipt {
    receipt_kind: String,
    schema_version: String,
    receipt_id: String,
    mutation_kind: String,
    surface: String,
    source_task_id: String,
    dry_run: bool,
    applied: bool,
    reason: String,
    before_validation: TaskMutationValidationSummary,
    after_validation: TaskMutationValidationSummary,
    before_task_count: usize,
    after_task_count: usize,
    planned_task_ids: Vec<String>,
    planned_dependency_edges: Vec<TaskMutationPlannedDependency>,
    validation_scope: String,
    operator_truth: serde_json::Value,
}

#[allow(dead_code)]
pub(crate) const ADAPTIVE_REPLAN_FINDING_KINDS: &[&str] = &[
    "verification_finding",
    "proof_gap",
    "scope_drift",
    "oversized_task",
];

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct AdaptiveReplanFindingInput {
    schema_version: String,
    input_kind: String,
    finding_kind: String,
    source_task_id: String,
    summary: String,
    evidence_refs: Vec<String>,
    operator_truth: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct AdaptiveReplanFindingInputError {
    status: String,
    blocker_codes: Vec<String>,
    reason: String,
    field: Option<String>,
    supported_finding_kinds: Vec<String>,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct AdaptiveReplanFindingPreview {
    status: String,
    surface: String,
    dry_run: bool,
    applied: bool,
    planned_mutation_category: String,
    planned_mutation_kind: String,
    source_task_id: String,
    finding: AdaptiveReplanFindingInput,
    preview_receipt: AdaptiveReplanFindingPreviewReceipt,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct AdaptiveReplanFindingPreviewReceipt {
    receipt_kind: String,
    schema_version: String,
    receipt_id: String,
    surface: String,
    source_task_id: String,
    finding_kind: String,
    planned_mutation_category: String,
    planned_mutation_kind: String,
    dry_run: bool,
    applied: bool,
    graph_state_opened: bool,
    graph_state_mutated: bool,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationResult {
    status: String,
    surface: String,
    mutation_kind: String,
    source_task_id: String,
    dry_run: bool,
    applied: bool,
    reason: String,
    planned_tasks: Vec<TaskMutationPlannedTask>,
    planned_dependencies: Vec<TaskMutationPlannedDependency>,
    created_task_ids: Vec<String>,
    validation: TaskMutationValidationSummary,
    graph_mutation_receipt: TaskGraphMutationValidationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSplitChildSpec {
    task_id: String,
    title: String,
}

fn task_mutation_validation_summary(
    issues: Vec<state_store::TaskGraphIssue>,
) -> TaskMutationValidationSummary {
    let blocker_codes = if issues.is_empty() {
        Vec::new()
    } else {
        vec!["invalid_task_graph".to_string()]
    };
    TaskMutationValidationSummary {
        status: if issues.is_empty() {
            task_json_success_status().to_string()
        } else {
            "blocked".to_string()
        },
        issue_count: issues.len(),
        blocker_codes,
        issues,
    }
}

#[allow(dead_code)]
pub(crate) fn adaptive_replan_finding_input_operator_truth() -> serde_json::Value {
    serde_json::json!({
        "input_model": "adaptive_replan_finding_input",
        "schema_version": "1",
        "accepted_finding_kinds": ADAPTIVE_REPLAN_FINDING_KINDS,
        "parsing_and_validation_only": true,
        "adaptive_mutation_execution_loop_implemented": false,
        "adaptive_mutation_execution_loop_truth": "not_implemented_in_this_slice",
        "valid_input_does_not_mutate_task_graph": true,
    })
}

#[allow(dead_code)]
fn adaptive_replan_finding_input_error(
    reason: impl Into<String>,
    field: Option<&str>,
) -> AdaptiveReplanFindingInputError {
    AdaptiveReplanFindingInputError {
        status: "blocked".to_string(),
        blocker_codes: vec!["invalid_adaptive_replan_finding_input".to_string()],
        reason: reason.into(),
        field: field.map(str::to_string),
        supported_finding_kinds: ADAPTIVE_REPLAN_FINDING_KINDS
            .iter()
            .map(|kind| kind.to_string())
            .collect(),
        operator_truth: adaptive_replan_finding_input_operator_truth(),
    }
}

#[allow(dead_code)]
fn required_non_empty_json_string(
    input: &serde_json::Value,
    field: &str,
) -> Result<String, AdaptiveReplanFindingInputError> {
    input
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            adaptive_replan_finding_input_error(
                format!("`{field}` must be a non-empty string"),
                Some(field),
            )
        })
}

#[allow(dead_code)]
fn optional_json_string_list(
    input: &serde_json::Value,
    field: &str,
) -> Result<Vec<String>, AdaptiveReplanFindingInputError> {
    let Some(value) = input.get(field) else {
        return Ok(Vec::new());
    };
    let rows = value.as_array().ok_or_else(|| {
        adaptive_replan_finding_input_error(format!("`{field}` must be an array"), Some(field))
    })?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(entry) = row
            .as_str()
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        else {
            return Err(adaptive_replan_finding_input_error(
                format!("`{field}` entries must be non-empty strings"),
                Some(field),
            ));
        };
        values.push(entry.to_string());
    }
    values.sort();
    values.dedup();
    Ok(values)
}

#[allow(dead_code)]
pub(crate) fn parse_adaptive_replan_finding_input(
    input: &serde_json::Value,
) -> Result<AdaptiveReplanFindingInput, AdaptiveReplanFindingInputError> {
    if !input.is_object() {
        return Err(adaptive_replan_finding_input_error(
            "adaptive replan finding input must be a JSON object",
            None,
        ));
    }
    let finding_kind = required_non_empty_json_string(input, "finding_kind")?;
    if !ADAPTIVE_REPLAN_FINDING_KINDS.contains(&finding_kind.as_str()) {
        return Err(adaptive_replan_finding_input_error(
            format!("unsupported adaptive replan finding kind `{finding_kind}`"),
            Some("finding_kind"),
        ));
    }
    Ok(AdaptiveReplanFindingInput {
        schema_version: input
            .get("schema_version")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("1")
            .to_string(),
        input_kind: "adaptive_replan_finding_input".to_string(),
        finding_kind,
        source_task_id: required_non_empty_json_string(input, "source_task_id")?,
        summary: required_non_empty_json_string(input, "summary")?,
        evidence_refs: optional_json_string_list(input, "evidence_refs")?,
        operator_truth: adaptive_replan_finding_input_operator_truth(),
    })
}

fn adaptive_replan_preview_operator_truth() -> serde_json::Value {
    serde_json::json!({
        "surface": "vida task adaptive-preview",
        "schema_version": "1",
        "preview_only": true,
        "finding_json_parsed": true,
        "planned_mutation_category_only": true,
        "preview_receipt_emitted": true,
        "graph_state_opened": false,
        "graph_state_mutated": false,
        "adaptive_mutation_execution_loop_implemented": false,
        "adaptive_mutation_execution_loop_truth": "not_implemented_in_this_slice",
    })
}

fn planned_mutation_for_finding_kind(finding_kind: &str) -> (&'static str, &'static str) {
    match finding_kind {
        "verification_finding" | "proof_gap" => ("blocker_resolution", "spawn_blocker_task"),
        "scope_drift" => ("scope_replan", "replan_scope_review"),
        "oversized_task" => ("task_decomposition", "split_task"),
        _ => ("unsupported", "blocked"),
    }
}

fn adaptive_replan_preview_receipt_id(
    finding: &AdaptiveReplanFindingInput,
    planned_mutation_category: &str,
    planned_mutation_kind: &str,
) -> String {
    let evidence_fingerprint = if finding.evidence_refs.is_empty() {
        "none".to_string()
    } else {
        finding.evidence_refs.join("+")
    };
    format!(
        "adaptive-replan-preview:{}:{}:{}:{}:evidence={}",
        finding.source_task_id,
        finding.finding_kind,
        planned_mutation_category,
        planned_mutation_kind,
        evidence_fingerprint
    )
}

fn build_adaptive_replan_finding_preview_receipt(
    finding: &AdaptiveReplanFindingInput,
    surface: &str,
    planned_mutation_category: &str,
    planned_mutation_kind: &str,
) -> AdaptiveReplanFindingPreviewReceipt {
    AdaptiveReplanFindingPreviewReceipt {
        receipt_kind: "adaptive_replan_finding_preview_receipt".to_string(),
        schema_version: "1".to_string(),
        receipt_id: adaptive_replan_preview_receipt_id(
            finding,
            planned_mutation_category,
            planned_mutation_kind,
        ),
        surface: surface.to_string(),
        source_task_id: finding.source_task_id.clone(),
        finding_kind: finding.finding_kind.clone(),
        planned_mutation_category: planned_mutation_category.to_string(),
        planned_mutation_kind: planned_mutation_kind.to_string(),
        dry_run: true,
        applied: false,
        graph_state_opened: false,
        graph_state_mutated: false,
        operator_truth: adaptive_replan_preview_operator_truth(),
    }
}

fn build_adaptive_replan_finding_preview(
    finding_json: &serde_json::Value,
    surface: &str,
) -> Result<AdaptiveReplanFindingPreview, AdaptiveReplanFindingInputError> {
    let finding = parse_adaptive_replan_finding_input(finding_json)?;
    let (planned_mutation_category, planned_mutation_kind) =
        planned_mutation_for_finding_kind(&finding.finding_kind);
    let preview_receipt = build_adaptive_replan_finding_preview_receipt(
        &finding,
        surface,
        planned_mutation_category,
        planned_mutation_kind,
    );
    Ok(AdaptiveReplanFindingPreview {
        status: task_json_success_status().to_string(),
        surface: surface.to_string(),
        dry_run: true,
        applied: false,
        planned_mutation_category: planned_mutation_category.to_string(),
        planned_mutation_kind: planned_mutation_kind.to_string(),
        source_task_id: finding.source_task_id.clone(),
        finding,
        preview_receipt,
        operator_truth: adaptive_replan_preview_operator_truth(),
    })
}

fn print_adaptive_replan_finding_preview(
    render: RenderMode,
    result: &AdaptiveReplanFindingPreview,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(result)
            .expect("adaptive replan finding preview should serialize to json");
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, &result.surface);
    print_surface_line(render, "status", &result.status);
    print_surface_line(
        render,
        "planned_mutation_category",
        &result.planned_mutation_category,
    );
    print_surface_line(
        render,
        "planned_mutation_kind",
        &result.planned_mutation_kind,
    );
    print_surface_line(render, "source_task_id", &result.source_task_id);
    print_surface_line(render, "dry_run", "true");
    print_surface_line(render, "applied", "false");
    print_surface_line(render, "graph_state_mutated", "false");
    print_surface_line(
        render,
        "preview_receipt_id",
        &result.preview_receipt.receipt_id,
    );
}

fn print_adaptive_replan_finding_input_error(
    error: &AdaptiveReplanFindingInputError,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(error)
            .expect("adaptive replan finding input error should serialize to json");
        crate::print_json_pretty(&payload);
    } else {
        eprintln!("{}", error.reason);
    }
}

fn parse_adaptive_preview_finding_json_text(
    finding_text: &str,
    field: Option<&str>,
) -> Result<serde_json::Value, AdaptiveReplanFindingInputError> {
    match serde_json::from_str::<serde_json::Value>(finding_text) {
        Ok(value) => Ok(value),
        Err(error) => Err(adaptive_replan_finding_input_error(
            format!("finding input must be valid JSON: {error}"),
            field,
        )),
    }
}

fn load_adaptive_preview_finding_json(
    finding_json: Option<&str>,
    finding_file: Option<&std::path::Path>,
) -> Result<serde_json::Value, AdaptiveReplanFindingInputError> {
    match (finding_json, finding_file) {
        (Some(_), Some(_)) => Err(adaptive_replan_finding_input_error(
            "Use only one finding source: --finding-json <json> or --finding-file <path>",
            None,
        )),
        (Some(value), None) => parse_adaptive_preview_finding_json_text(value, None),
        (None, Some(path)) => {
            let value = std::fs::read_to_string(path).map_err(|error| {
                adaptive_replan_finding_input_error(
                    format!("Failed to read finding file `{}`: {error}", path.display()),
                    Some("finding_file"),
                )
            })?;
            parse_adaptive_preview_finding_json_text(&value, Some("finding_file"))
        }
        (None, None) => Err(adaptive_replan_finding_input_error(
            "Provide --finding-json <json> or --finding-file <path>",
            None,
        )),
    }
}

async fn run_task_adaptive_preview(command: TaskAdaptivePreviewArgs) -> ExitCode {
    let finding_json = match load_adaptive_preview_finding_json(
        command.finding_json.as_deref(),
        command.finding_file.as_deref(),
    ) {
        Ok(value) => value,
        Err(error) => {
            print_adaptive_replan_finding_input_error(&error, command.json);
            return ExitCode::from(2);
        }
    };
    match build_adaptive_replan_finding_preview(&finding_json, "vida task adaptive-preview") {
        Ok(result) => {
            print_adaptive_replan_finding_preview(command.render, &result, command.json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            print_adaptive_replan_finding_input_error(&error, command.json);
            ExitCode::from(2)
        }
    }
}

fn graph_mutation_receipt_id(
    mutation_kind: &str,
    source_task_id: &str,
    planned_tasks: &[TaskMutationPlannedTask],
    planned_dependencies: &[TaskMutationPlannedDependency],
) -> String {
    let planned_task_ids = planned_tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let dependency_fingerprint = planned_dependencies
        .iter()
        .map(|dependency| {
            format!(
                "{}>{}:{}",
                dependency.issue_id, dependency.depends_on_id, dependency.edge_type
            )
        })
        .collect::<Vec<_>>()
        .join("+");
    format!(
        "task-graph-mutation:{mutation_kind}:{source_task_id}:tasks={planned_task_ids}:edges={dependency_fingerprint}"
    )
}

struct GraphMutationReceiptInput<'a> {
    mutation_kind: &'a str,
    surface: &'a str,
    source_task_id: &'a str,
    dry_run: bool,
    applied: bool,
    reason: &'a str,
    before_validation: TaskMutationValidationSummary,
    after_validation: TaskMutationValidationSummary,
    before_task_count: usize,
    after_task_count: usize,
    planned_tasks: &'a [TaskMutationPlannedTask],
    planned_dependencies: &'a [TaskMutationPlannedDependency],
}

fn build_graph_mutation_receipt(
    input: GraphMutationReceiptInput<'_>,
) -> TaskGraphMutationValidationReceipt {
    let planned_task_ids = input
        .planned_tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    TaskGraphMutationValidationReceipt {
        receipt_kind: "task_graph_mutation_receipt".to_string(),
        schema_version: "1".to_string(),
        receipt_id: graph_mutation_receipt_id(
            input.mutation_kind,
            input.source_task_id,
            input.planned_tasks,
            input.planned_dependencies,
        ),
        mutation_kind: input.mutation_kind.to_string(),
        surface: input.surface.to_string(),
        source_task_id: input.source_task_id.to_string(),
        dry_run: input.dry_run,
        applied: input.applied,
        reason: input.reason.to_string(),
        before_validation: input.before_validation,
        after_validation: input.after_validation,
        before_task_count: input.before_task_count,
        after_task_count: input.after_task_count,
        planned_task_ids,
        planned_dependency_edges: input.planned_dependencies.to_vec(),
        validation_scope:
            "before=current_authoritative_task_rows; after=planned_simulated_task_rows".to_string(),
        operator_truth: serde_json::json!({
            "receipt_records_graph_mutation_shape": true,
            "records_before_after_validation": true,
            "adaptive_replanner_loop_implemented": false,
            "adaptive_replanner_loop_truth": "not_implemented_in_this_slice",
            "applied_mutation_requires_after_validation_pass": true,
        }),
    }
}

fn task_parent_id(task: &state_store::TaskRecord) -> Option<String> {
    task.dependencies
        .iter()
        .find(|dependency| dependency.edge_type == "parent-child")
        .map(|dependency| dependency.depends_on_id.clone())
}

fn open_child_ids_for_task(rows: &[state_store::TaskRecord], task_id: &str) -> Vec<String> {
    let mut child_ids = rows
        .iter()
        .filter(|task| {
            task.status != "closed"
                && task.dependencies.iter().any(|dependency| {
                    dependency.edge_type == "parent-child" && dependency.depends_on_id == task_id
                })
        })
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    child_ids.sort();
    child_ids
}

fn inherited_split_execution_semantics(
    task: &state_store::TaskRecord,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: Some("sequential".to_string()),
        order_bucket: task.execution_semantics.order_bucket.clone(),
        parallel_group: None,
        conflict_domain: task
            .execution_semantics
            .conflict_domain
            .clone()
            .or_else(|| Some(task.id.clone())),
    }
}

fn blocker_execution_semantics(
    task: &state_store::TaskRecord,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: Some("sequential".to_string()),
        order_bucket: task.execution_semantics.order_bucket.clone(),
        parallel_group: None,
        conflict_domain: task.execution_semantics.conflict_domain.clone(),
    }
}

fn parse_split_child_specs(values: &[String]) -> Result<Vec<ParsedSplitChildSpec>, String> {
    if values.len() < 2 {
        return Err(
            "Use at least two `--child <task-id>:<title>` entries for `vida task split`."
                .to_string(),
        );
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let Some((task_id, title)) = value.split_once(':') else {
            return Err(format!(
                "Invalid `--child` value `{value}`. Expected `<task-id>:<title>`."
            ));
        };
        let task_id = task_id.trim();
        let title = title.trim();
        if task_id.is_empty() || title.is_empty() {
            return Err(format!(
                "Invalid `--child` value `{value}`. Both task id and title are required."
            ));
        }
        if !seen.insert(task_id.to_string()) {
            return Err(format!("Duplicate split child task id `{task_id}`."));
        }
        parsed.push(ParsedSplitChildSpec {
            task_id: task_id.to_string(),
            title: title.to_string(),
        });
    }
    Ok(parsed)
}

fn build_split_mutation_preview(
    rows: &[state_store::TaskRecord],
    source: &state_store::TaskRecord,
    child_specs: &[ParsedSplitChildSpec],
    reason: &str,
    surface: &str,
    dry_run: bool,
) -> Result<(TaskMutationResult, Vec<state_store::TaskRecord>), String> {
    if source.issue_type == "epic" {
        return Err(format!(
            "Cannot split epic `{}` through `vida task split`; choose a bounded non-epic task.",
            source.id
        ));
    }
    let existing_children = open_child_ids_for_task(rows, &source.id);
    if !existing_children.is_empty() {
        return Err(format!(
            "Cannot split task `{}` while open child tasks already exist: {}",
            source.id,
            existing_children.join(", ")
        ));
    }
    if let Some(existing) = child_specs
        .iter()
        .find(|spec| rows.iter().any(|task| task.id == spec.task_id))
    {
        return Err(format!(
            "Cannot split task `{}` because child task id `{}` already exists.",
            source.id, existing.task_id
        ));
    }

    let non_parent_dependencies = source
        .dependencies
        .iter()
        .filter(|dependency| dependency.edge_type != "parent-child")
        .cloned()
        .collect::<Vec<_>>();
    let parent_id = Some(source.id.clone());
    let inherited_semantics = inherited_split_execution_semantics(source);
    let mut planned_tasks = Vec::with_capacity(child_specs.len());
    let mut planned_dependencies = Vec::new();
    let mut simulated_rows = rows.to_vec();
    let source_index = simulated_rows
        .iter()
        .position(|task| task.id == source.id)
        .ok_or_else(|| {
            format!(
                "Source task `{}` is missing from current task rows.",
                source.id
            )
        })?;
    if source.status == "closed" {
        simulated_rows[source_index].status = "in_progress".to_string();
        simulated_rows[source_index].closed_at = None;
        simulated_rows[source_index].close_reason = None;
    }

    let mut previous_child_id = None::<String>;
    for (index, spec) in child_specs.iter().enumerate() {
        let description = if source.description.trim().is_empty() {
            format!("Split from `{}`: {reason}", source.id)
        } else {
            source.description.clone()
        };
        let mut dependencies = vec![state_store::TaskDependencyRecord {
            issue_id: spec.task_id.clone(),
            depends_on_id: source.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        if index == 0 {
            for dependency in &non_parent_dependencies {
                dependencies.push(state_store::TaskDependencyRecord {
                    issue_id: spec.task_id.clone(),
                    depends_on_id: dependency.depends_on_id.clone(),
                    edge_type: dependency.edge_type.clone(),
                    created_at: source.updated_at.clone(),
                    created_by: surface.to_string(),
                    metadata: "{}".to_string(),
                    thread_id: String::new(),
                });
                planned_dependencies.push(TaskMutationPlannedDependency {
                    issue_id: spec.task_id.clone(),
                    depends_on_id: dependency.depends_on_id.clone(),
                    edge_type: dependency.edge_type.clone(),
                    reason: "inherit_source_dependency".to_string(),
                });
            }
        }

        if let Some(previous_child_id) = previous_child_id.as_ref() {
            dependencies.push(state_store::TaskDependencyRecord {
                issue_id: spec.task_id.clone(),
                depends_on_id: previous_child_id.clone(),
                edge_type: "depends-on".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
            planned_dependencies.push(TaskMutationPlannedDependency {
                issue_id: spec.task_id.clone(),
                depends_on_id: previous_child_id.clone(),
                edge_type: "depends-on".to_string(),
                reason: "sequential_split_chain".to_string(),
            });
        }

        simulated_rows.push(state_store::TaskRecord {
            id: spec.task_id.clone(),
            display_id: None,
            title: spec.title.clone(),
            description: description.clone(),
            status: "open".to_string(),
            priority: source.priority,
            issue_type: source.issue_type.clone(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            updated_at: source.updated_at.clone(),
            closed_at: None,
            close_reason: None,
            source_repo: source.source_repo.clone(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: source.labels.clone(),
            planner_metadata: source.planner_metadata.clone(),
            execution_semantics: inherited_semantics.clone(),
            provider_mapping: None,
            dependencies,
        });
        planned_tasks.push(TaskMutationPlannedTask {
            task_id: spec.task_id.clone(),
            title: spec.title.clone(),
            description,
            issue_type: source.issue_type.clone(),
            status: "open".to_string(),
            priority: source.priority,
            parent_id: parent_id.clone(),
            labels: source.labels.clone(),
            execution_semantics: inherited_semantics.clone(),
            planner_metadata: source.planner_metadata.clone(),
        });
        previous_child_id = Some(spec.task_id.clone());
    }

    if let Some(last_child_id) = previous_child_id {
        simulated_rows[source_index]
            .dependencies
            .push(state_store::TaskDependencyRecord {
                issue_id: source.id.clone(),
                depends_on_id: last_child_id.clone(),
                edge_type: "depends-on".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        planned_dependencies.push(TaskMutationPlannedDependency {
            issue_id: source.id.clone(),
            depends_on_id: last_child_id,
            edge_type: "depends-on".to_string(),
            reason: "block_source_until_split_children_complete".to_string(),
        });
    }

    let before_validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(rows));
    let validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(&simulated_rows));
    let status = if validation.issue_count > 0 {
        "blocked".to_string()
    } else if dry_run {
        "dry_run".to_string()
    } else {
        task_json_success_status().to_string()
    };
    let created_task_ids = if dry_run || validation.issue_count > 0 {
        Vec::new()
    } else {
        planned_tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect()
    };
    let applied = !dry_run && validation.issue_count == 0;
    let graph_mutation_receipt = build_graph_mutation_receipt(GraphMutationReceiptInput {
        mutation_kind: "split_task",
        surface,
        source_task_id: &source.id,
        dry_run,
        applied,
        reason,
        before_validation,
        after_validation: validation.clone(),
        before_task_count: rows.len(),
        after_task_count: simulated_rows.len(),
        planned_tasks: &planned_tasks,
        planned_dependencies: &planned_dependencies,
    });
    Ok((
        TaskMutationResult {
            status,
            surface: surface.to_string(),
            mutation_kind: "split_task".to_string(),
            source_task_id: source.id.clone(),
            dry_run,
            applied,
            reason: reason.to_string(),
            planned_tasks,
            planned_dependencies,
            created_task_ids,
            validation,
            graph_mutation_receipt,
        },
        simulated_rows,
    ))
}

fn build_spawn_blocker_preview(
    rows: &[state_store::TaskRecord],
    source: &state_store::TaskRecord,
    command: &TaskSpawnBlockerArgs,
    surface: &str,
) -> Result<(TaskMutationResult, Vec<state_store::TaskRecord>), String> {
    if source.status == "closed" {
        return Err(format!(
            "Cannot spawn blocker for closed task `{}`.",
            source.id
        ));
    }
    if rows.iter().any(|task| task.id == command.blocker_task_id) {
        return Err(format!(
            "Cannot create blocker task `{}` because it already exists.",
            command.blocker_task_id
        ));
    }

    let mut blocker_labels = source.labels.clone();
    blocker_labels.extend(parse_label_values(&command.labels));
    blocker_labels.sort();
    blocker_labels.dedup();

    let blocker_priority = command.priority.unwrap_or(source.priority);
    let blocker_description = command
        .description
        .clone()
        .unwrap_or_else(|| format!("Blocker for `{}`: {}", source.id, command.reason));
    let blocker_parent_id = task_parent_id(source);
    let blocker_semantics = blocker_execution_semantics(source);

    let mut simulated_rows = rows.to_vec();
    let source_index = simulated_rows
        .iter()
        .position(|task| task.id == source.id)
        .ok_or_else(|| {
            format!(
                "Source task `{}` is missing from current task rows.",
                source.id
            )
        })?;
    simulated_rows.push(state_store::TaskRecord {
        id: command.blocker_task_id.clone(),
        display_id: None,
        title: command.title.clone(),
        description: blocker_description.clone(),
        status: command.status.clone(),
        priority: blocker_priority,
        issue_type: command.issue_type.clone(),
        created_at: source.updated_at.clone(),
        created_by: surface.to_string(),
        updated_at: source.updated_at.clone(),
        closed_at: None,
        close_reason: None,
        source_repo: source.source_repo.clone(),
        compaction_level: 0,
        original_size: 0,
        notes: None,
        labels: blocker_labels.clone(),
        planner_metadata: source.planner_metadata.clone(),
        execution_semantics: blocker_semantics.clone(),
        provider_mapping: None,
        dependencies: blocker_parent_id
            .iter()
            .map(|parent_id| state_store::TaskDependencyRecord {
                issue_id: command.blocker_task_id.clone(),
                depends_on_id: parent_id.clone(),
                edge_type: "parent-child".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            })
            .collect(),
    });
    simulated_rows[source_index]
        .dependencies
        .push(state_store::TaskDependencyRecord {
            issue_id: source.id.clone(),
            depends_on_id: command.blocker_task_id.clone(),
            edge_type: "blocks".to_string(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });

    let before_validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(rows));
    let validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(&simulated_rows));
    let dry_run = command.dry_run;
    let status = if validation.issue_count > 0 {
        "blocked".to_string()
    } else if dry_run {
        "dry_run".to_string()
    } else {
        task_json_success_status().to_string()
    };
    let planned_tasks = vec![TaskMutationPlannedTask {
        task_id: command.blocker_task_id.clone(),
        title: command.title.clone(),
        description: blocker_description.clone(),
        issue_type: command.issue_type.clone(),
        status: command.status.clone(),
        priority: blocker_priority,
        parent_id: blocker_parent_id,
        labels: blocker_labels,
        execution_semantics: blocker_semantics,
        planner_metadata: source.planner_metadata.clone(),
    }];
    let planned_dependencies = vec![TaskMutationPlannedDependency {
        issue_id: source.id.clone(),
        depends_on_id: command.blocker_task_id.clone(),
        edge_type: "blocks".to_string(),
        reason: "spawn_blocker_dependency".to_string(),
    }];
    let created_task_ids = if dry_run || validation.issue_count > 0 {
        Vec::new()
    } else {
        vec![command.blocker_task_id.clone()]
    };
    let applied = !dry_run && validation.issue_count == 0;
    let graph_mutation_receipt = build_graph_mutation_receipt(GraphMutationReceiptInput {
        mutation_kind: "spawn_blocker_task",
        surface,
        source_task_id: &source.id,
        dry_run,
        applied,
        reason: &command.reason,
        before_validation,
        after_validation: validation.clone(),
        before_task_count: rows.len(),
        after_task_count: simulated_rows.len(),
        planned_tasks: &planned_tasks,
        planned_dependencies: &planned_dependencies,
    });
    Ok((
        TaskMutationResult {
            status,
            surface: surface.to_string(),
            mutation_kind: "spawn_blocker_task".to_string(),
            source_task_id: source.id.clone(),
            dry_run,
            applied: !dry_run && validation.issue_count == 0,
            reason: command.reason.clone(),
            planned_tasks,
            planned_dependencies,
            created_task_ids,
            validation,
            graph_mutation_receipt,
        },
        simulated_rows,
    ))
}

fn print_task_mutation_preview(render: RenderMode, result: &TaskMutationResult, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(result).expect("task mutation preview should serialize to json");
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, &result.surface);
    print_surface_line(render, "status", &result.status);
    print_surface_line(render, "mutation_kind", &result.mutation_kind);
    print_surface_line(render, "source_task_id", &result.source_task_id);
    print_surface_line(
        render,
        "dry_run",
        if result.dry_run { "true" } else { "false" },
    );
    print_surface_line(
        render,
        "applied",
        if result.applied { "true" } else { "false" },
    );
    print_surface_line(
        render,
        "planned_task_count",
        &result.planned_tasks.len().to_string(),
    );
    print_surface_line(
        render,
        "planned_dependency_count",
        &result.planned_dependencies.len().to_string(),
    );
    if !result.created_task_ids.is_empty() {
        print_surface_line(
            render,
            "created_task_ids",
            &result.created_task_ids.join(", "),
        );
    }
    if !result.validation.blocker_codes.is_empty() {
        print_surface_line(
            render,
            "blocker_codes",
            &result.validation.blocker_codes.join(", "),
        );
    }
}

async fn run_task_split_like(command: TaskSplitArgs, surface: &str) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let child_specs = match parse_split_child_specs(&command.children) {
        Ok(specs) => specs,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let store = match open_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let source = match store.show_task(&command.task_id).await {
        Ok(task) => task,
        Err(error) => {
            eprintln!("Failed to load split source task: {error}");
            return ExitCode::from(1);
        }
    };
    let rows = match store.all_tasks().await {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Failed to read current task graph before split: {error}");
            return ExitCode::from(1);
        }
    };
    let (result, _) = match build_split_mutation_preview(
        &rows,
        &source,
        &child_specs,
        &command.reason,
        surface,
        command.dry_run,
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if result.validation.issue_count > 0 {
        print_task_mutation_preview(command.render, &result, command.json);
        return ExitCode::from(1);
    }

    if !command.dry_run {
        let source_repo = source.source_repo.clone();
        for task in &result.planned_tasks {
            if let Err(error) = store
                .create_task(state_store::CreateTaskRequest {
                    task_id: &task.task_id,
                    title: &task.title,
                    display_id: None,
                    description: &task.description,
                    issue_type: &task.issue_type,
                    status: &task.status,
                    priority: task.priority,
                    parent_id: task.parent_id.as_deref(),
                    labels: &task.labels,
                    execution_semantics: task.execution_semantics.clone(),
                    planner_metadata: task.planner_metadata.clone(),
                    created_by: surface,
                    source_repo: &source_repo,
                })
                .await
            {
                eprintln!(
                    "Failed to create split child task `{}`: {error}",
                    task.task_id
                );
                return ExitCode::from(1);
            }
        }
        for dependency in &result.planned_dependencies {
            if let Err(error) = store
                .add_task_dependency(
                    &dependency.issue_id,
                    &dependency.depends_on_id,
                    &dependency.edge_type,
                    surface,
                )
                .await
            {
                eprintln!(
                    "Failed to add split dependency `{}` -> `{}`: {error}",
                    dependency.issue_id, dependency.depends_on_id
                );
                return ExitCode::from(1);
            }
        }
        if let Err(code) = refresh_task_snapshot_after_mutation(&store, surface).await {
            return code;
        }
    }

    print_task_mutation_preview(command.render, &result, command.json);
    ExitCode::SUCCESS
}

async fn run_task_spawn_blocker_like(command: TaskSpawnBlockerArgs, surface: &str) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let store = match open_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let source = match store.show_task(&command.task_id).await {
        Ok(task) => task,
        Err(error) => {
            eprintln!("Failed to load blocker source task: {error}");
            return ExitCode::from(1);
        }
    };
    let rows = match store.all_tasks().await {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Failed to read current task graph before blocker mutation: {error}");
            return ExitCode::from(1);
        }
    };
    let (result, _) = match build_spawn_blocker_preview(&rows, &source, &command, surface) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if result.validation.issue_count > 0 {
        print_task_mutation_preview(command.render, &result, command.json);
        return ExitCode::from(1);
    }

    if !command.dry_run {
        let planned_task = result
            .planned_tasks
            .first()
            .expect("spawn blocker preview should include one planned task");
        if let Err(error) = store
            .create_task(state_store::CreateTaskRequest {
                task_id: &planned_task.task_id,
                title: &planned_task.title,
                display_id: None,
                description: &planned_task.description,
                issue_type: &planned_task.issue_type,
                status: &planned_task.status,
                priority: planned_task.priority,
                parent_id: planned_task.parent_id.as_deref(),
                labels: &planned_task.labels,
                execution_semantics: planned_task.execution_semantics.clone(),
                planner_metadata: planned_task.planner_metadata.clone(),
                created_by: surface,
                source_repo: &source.source_repo,
            })
            .await
        {
            eprintln!(
                "Failed to create blocker task `{}`: {error}",
                planned_task.task_id
            );
            return ExitCode::from(1);
        }
        let dependency = result
            .planned_dependencies
            .first()
            .expect("spawn blocker preview should include one dependency");
        if let Err(error) = store
            .add_task_dependency(
                &dependency.issue_id,
                &dependency.depends_on_id,
                &dependency.edge_type,
                surface,
            )
            .await
        {
            eprintln!(
                "Failed to attach blocker task `{}` to source `{}`: {error}",
                dependency.depends_on_id, dependency.issue_id
            );
            return ExitCode::from(1);
        }
        if let Err(code) = refresh_task_snapshot_after_mutation(&store, surface).await {
            return code;
        }
    }

    print_task_mutation_preview(command.render, &result, command.json);
    ExitCode::SUCCESS
}

async fn run_task_create_like(command: TaskCreateArgs, ensure_existing: bool) -> ExitCode {
    let title = match task_create_title(&command) {
        Ok(title) => title,
        Err(error) => {
            if command.json {
                crate::print_json_pretty(&serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": ["invalid_task_title_input"],
                    "reason": error,
                    "usage": "vida task create <task-id> <title> --json OR vida task create <task-id> --title <title> --json",
                }));
            } else {
                eprintln!("{error}");
                eprintln!(
                    "Usage: vida task create <task-id> <title> --json OR vida task create <task-id> --title <title> --json"
                );
            }
            return ExitCode::from(2);
        }
    };
    if let Some(path) = command.notes_file.as_deref() {
        let action = format!(
            "Use `vida task {} <task-id> <title> --notes <text> --json` for trusted inline create-time notes, or create the task first and then run `vida task update <task-id> --notes-file {} --json` when recording operator-owned progress.",
            if ensure_existing { "ensure" } else { "create" },
            crate::shell_quote(&path.display().to_string())
        );
        if command.json {
            crate::print_json_pretty(&serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["untrusted_create_notes_file"],
                "surface": if ensure_existing { "vida task ensure" } else { "vida task create" },
                "rejected_option": "--notes-file",
                "rejected_path": path,
                "next_action": action,
                "next_actions": [action],
            }));
        } else {
            eprintln!(
                "Refusing --notes-file for `vida task {}`: path `{}` is outside the trusted inline intake boundary.",
                if ensure_existing { "ensure" } else { "create" },
                path.display()
            );
            eprintln!("{action}");
        }
        return ExitCode::from(2);
    }
    let notes = match resolve_optional_text_arg("notes", command.notes.as_deref(), None) {
        Ok(notes) => notes,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let planner_metadata = task_create_planner_metadata_arg(&command);
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let project_root = project_root_for_task_state(&state_dir).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    match open_task_store(state_dir.clone()).await {
        Ok(store) => {
            let mut parent_id = command.parent_id.clone();
            let mut display_id = command.display_id.clone().unwrap_or_default();
            let auto_display_from = command.auto_display_from.clone().unwrap_or_default();
            let parent_display_id = command.parent_display_id.clone().unwrap_or_default();
            if display_id.is_empty() && !auto_display_from.is_empty() && parent_id.is_some() {
                display_id = format!("{auto_display_from}.1");
            }
            if (display_id.is_empty() && !auto_display_from.is_empty())
                || (parent_id.is_none() && !parent_display_id.is_empty())
            {
                match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            if display_id.is_empty() && !auto_display_from.is_empty() {
                                let next = crate::taskflow_task_bridge::next_display_id_payload(
                                    &rows,
                                    &auto_display_from,
                                );
                                if !next
                                    .get("valid")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    print_task_next_display_id(command.render, &next, command.json);
                                    return ExitCode::from(1);
                                }
                                display_id = next
                                    .get("next_display_id")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                            }
                            if parent_id.is_none() && !parent_display_id.is_empty() {
                                let resolved =
                                    crate::taskflow_task_bridge::resolve_task_id_by_display_id(
                                        &rows,
                                        &parent_display_id,
                                    );
                                if !resolved
                                    .get("found")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    if command.json {
                                        crate::print_json_pretty(&resolved);
                                    } else {
                                        eprintln!(
                                            "{}",
                                            resolved
                                                .get("reason")
                                                .and_then(serde_json::Value::as_str)
                                                .unwrap_or("parent_display_id_not_found")
                                        );
                                    }
                                    return ExitCode::from(1);
                                }
                                parent_id = Some(
                                    resolved
                                        .get("task_id")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_string(),
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Failed to {} task: {error}",
                                if ensure_existing { "ensure" } else { "create" }
                            );
                            return ExitCode::from(1);
                        }
                    },
                    Err(error) => {
                        eprintln!(
                            "Failed to {} task: {error}",
                            if ensure_existing { "ensure" } else { "create" }
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            if ensure_existing {
                if let Ok(task) = store.show_task(&command.task_id).await {
                    let labels = parse_label_values(&command.labels);
                    if let Some(reason) = ensure_existing_task_mismatch_reason(
                        &task,
                        &title,
                        (!display_id.is_empty()).then_some(display_id.as_str()),
                        &command.issue_type,
                        &command.status,
                        parent_id.as_deref(),
                        &labels,
                    ) {
                        eprintln!("Failed to ensure task: {reason}");
                        return ExitCode::from(1);
                    }
                    if task_create_semantics_requested(&command)
                        && task_create_semantics_mismatch(&task.execution_semantics, &command)
                    {
                        eprintln!(
                            "Failed to ensure task: execution semantics mismatch for existing task; use `vida task update` to modify semantics explicitly."
                        );
                        return ExitCode::from(1);
                    }
                    print_task_mutation(command.render, "vida task ensure", &task, command.json);
                    return ExitCode::SUCCESS;
                }
            }
            let labels = parse_label_values(&command.labels);
            let source_repo = project_root.display().to_string();

            // Multi-session admission check (rule #3)
            // Check if another session holds an active exclusive claim on the same work scope
            let owner_evidence = crate::orchestrator_session_surface::build_runtime_owner_evidence(
                &state_dir, false,
            );
            let current_session_id = match &owner_evidence {
                Ok(evidence) => evidence["current_session"]["session_id"]
                    .as_str()
                    .unwrap_or("unknown"),
                Err(_) => "unknown",
            };
            let active_foreign_claims = store.active_foreign_claims(current_session_id).await;

            // Build a temporary task record for conflict checking
            let temp_execution_semantics = task_execution_semantics_from_create_args(&command);
            let temp_planner_metadata: state_store::TaskPlannerMetadata = planner_metadata.clone();
            let temp_task_id = command.task_id.trim().to_string();

            // Check for foreign claim conflicts
            if let Ok(foreign_claims) = &active_foreign_claims {
                // Use the same conflict checking logic as taskflow_proxy
                // We need to check if any foreign claim conflicts with our task
                let has_conflict = foreign_claims.iter().any(|claim| {
                    let claim_status = claim.status.trim().to_ascii_lowercase();
                    let claim_is_blocking_status = claim_status == "blocked";
                    let claim_is_exclusive = claim.lease_mode == "exclusive";
                    if !claim_is_blocking_status && !claim_is_exclusive {
                        return false;
                    }
                    // Check same task_id
                    if claim.task_id.as_deref() == Some(temp_task_id.as_str()) {
                        return true;
                    }
                    // Check same conflict_domain
                    if let Some(claim_domain) = claim.conflict_domain.as_deref() {
                        if temp_execution_semantics.conflict_domain.as_deref() == Some(claim_domain)
                        {
                            return true;
                        }
                    }
                    // Check intersecting owned_paths (from planner_metadata, not execution_semantics)
                    if !claim.owned_paths.is_empty()
                        && !temp_planner_metadata.owned_paths.is_empty()
                    {
                        for claim_path in &claim.owned_paths {
                            for task_path in &temp_planner_metadata.owned_paths {
                                if paths_intersect(claim_path, task_path) {
                                    return true;
                                }
                            }
                        }
                    }
                    // Exclusive claims also block writes intersecting their read-only paths.
                    if claim_is_exclusive {
                        if !claim.read_only_paths.is_empty()
                            && !temp_planner_metadata.owned_paths.is_empty()
                        {
                            for claim_path in &claim.read_only_paths {
                                for task_path in &temp_planner_metadata.owned_paths {
                                    if paths_intersect(claim_path, task_path) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    false
                });

                if has_conflict {
                    if command.json {
                        crate::print_json_pretty(&serde_json::json!({
                            "status": "blocked",
                            "blocker_codes": ["foreign_claim_conflict_blocked"],
                            "reason": "Another orchestrator session holds an active exclusive claim on the same task, run, conflict domain, or intersecting paths. Wait for that session to complete or explicitly reclaim/supersede the claim before continuing.",
                            "next_action": "vida orchestrator-session show --json",
                            "blocking_surface": "vida orchestrator-session show",
                            "current_session_id": current_session_id,
                        }));
                    } else {
                        eprintln!(
                            "Another orchestrator session holds an active exclusive claim on this work scope."
                        );
                        eprintln!(
                            "Inspect active sessions and claims with `vida orchestrator-session show --json`"
                        );
                    }
                    return ExitCode::from(1);
                }
            }

            match store
                .create_task(state_store::CreateTaskRequest {
                    task_id: &command.task_id,
                    title: &title,
                    display_id: (!display_id.is_empty()).then_some(display_id.as_str()),
                    description: &command.description,
                    issue_type: &command.issue_type,
                    status: &command.status,
                    priority: command.priority,
                    parent_id: parent_id.as_deref(),
                    labels: &labels,
                    execution_semantics: task_execution_semantics_from_create_args(&command),
                    planner_metadata: planner_metadata.clone(),
                    created_by: "vida task",
                    source_repo: &source_repo,
                })
                .await
            {
                Ok(task) => {
                    let task = if let Some(notes) = notes.as_deref() {
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: None,
                            })
                            .await
                        {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!("Failed to apply task notes after create: {error}");
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        task
                    };
                    if let Err(code) =
                        refresh_task_snapshot_after_mutation(&store, "vida task create").await
                    {
                        return code;
                    }
                    print_task_mutation(
                        command.render,
                        if ensure_existing {
                            "vida task ensure"
                        } else {
                            "vida task create"
                        },
                        &task,
                        command.json,
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!(
                        "Failed to {} task: {error}",
                        if ensure_existing { "ensure" } else { "create" }
                    );
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn ensure_existing_task_mismatch_reason(
    task: &state_store::TaskRecord,
    expected_title: &str,
    expected_display_id: Option<&str>,
    expected_issue_type: &str,
    expected_status: &str,
    expected_parent_id: Option<&str>,
    expected_labels: &[String],
) -> Option<String> {
    if task.title != expected_title {
        return Some(format!(
            "existing task '{}' title mismatch (expected '{}', got '{}')",
            task.id, expected_title, task.title
        ));
    }
    if task.display_id.as_deref() != expected_display_id {
        return Some(format!(
            "existing task '{}' display_id mismatch (expected '{}', got '{}')",
            task.id,
            expected_display_id.unwrap_or(""),
            task.display_id.as_deref().unwrap_or("")
        ));
    }
    if task.issue_type != expected_issue_type {
        return Some(format!(
            "existing task '{}' issue_type mismatch (expected '{}', got '{}')",
            task.id, expected_issue_type, task.issue_type
        ));
    }
    if task.status != expected_status {
        return Some(format!(
            "existing task '{}' status mismatch (expected '{}', got '{}')",
            task.id, expected_status, task.status
        ));
    }
    let existing_parent_id = task_parent_id(task);
    if existing_parent_id.as_deref() != expected_parent_id {
        return Some(format!(
            "existing task '{}' parent_id mismatch (expected '{}', got '{}')",
            task.id,
            expected_parent_id.unwrap_or(""),
            existing_parent_id.as_deref().unwrap_or("")
        ));
    }
    if expected_labels
        .iter()
        .any(|label| !task.labels.iter().any(|existing| existing == label))
    {
        let missing_labels: Vec<String> = expected_labels
            .iter()
            .filter(|label| !task.labels.iter().any(|existing| existing == *label))
            .cloned()
            .collect();
        return Some(format!(
            "existing task '{}' missing required labels: {}",
            task.id,
            missing_labels.join(",")
        ));
    }
    None
}

fn task_create_title(command: &TaskCreateArgs) -> Result<String, String> {
    let positional = command
        .positional_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let option = command
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (positional, option) {
        (Some(_), Some(_)) => Err(
            "Provide only one task title source: positional <TITLE> or --title <TITLE>."
                .to_string(),
        ),
        (Some(title), None) | (None, Some(title)) => Ok(title.to_string()),
        (None, None) => {
            Err("Missing task title. Use positional <TITLE> or --title <TITLE>.".to_string())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskCloseAutomationReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    release_build: Option<crate::release_surface::ReleaseBuildReceipt>,
    release_install: Option<crate::release_surface::ReleaseInstallReceipt>,
    git: Option<TaskCloseGitAutomationReceipt>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskCloseGitAutomationReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    explicit_files: Vec<String>,
    stage_error_detail: Option<String>,
    commit_message: Option<String>,
    commit_exit_code: Option<i32>,
    push_exit_code: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskOwnedStatusReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task_id: String,
    ownership_source: String,
    owned_paths: Vec<String>,
    dirty_files: Vec<String>,
    owned_files: Vec<String>,
    unowned_files: Vec<String>,
    stageable_files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskHandoffAcceptReceipt {
    status: String,
    task_id: String,
    agent_id: String,
    accepted_at: String,
    changed_files: Vec<String>,
    proof_commands: Vec<String>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    receipt_path: String,
    receipt_root: String,
    isolation: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskContinuationCandidate {
    task_id: String,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
    ready_parallel_safe: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskNextLawfulReceipt {
    status: String,
    active_bounded_unit: serde_json::Value,
    binding_source: Option<String>,
    why_this_unit: String,
    sequential_vs_parallel_posture: String,
    recommended_primary: Option<TaskContinuationCandidate>,
    recommended_parallel_batch: Vec<TaskContinuationCandidate>,
    why_not_auto_bound: Option<String>,
    bind_command: Option<String>,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_codes: Vec<String>,
    next_action: Option<String>,
    next_actions: Vec<String>,
    source_surfaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_explanation: Option<serde_json::Value>,
}

fn task_close_automation_requested(command: &TaskCloseArgs) -> bool {
    command.release || command.install || command.commit || command.push || command.stage_owned
}

fn task_close_automation_receipt(
    command: &TaskCloseArgs,
    project_root: Option<&std::path::Path>,
    task: Option<&state_store::TaskRecord>,
) -> TaskCloseAutomationReceipt {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();

    let release_install = if command.install {
        let receipt = crate::release_surface::release_install_receipt(&crate::ReleaseInstallArgs {
            target: command.install_target.clone(),
            skip_build: command.skip_release_build,
            source_binary: command.source_binary.clone(),
            install_root: command.install_root.clone(),
            json: true,
        });
        if receipt.status != "pass" {
            blocker_codes.extend(receipt.blocker_codes.iter().cloned());
            next_actions.extend(receipt.next_actions.iter().cloned());
        }
        Some(receipt)
    } else {
        None
    };

    let release_build = if command.release && !command.install {
        let receipt = crate::release_surface::release_build_receipt(false);
        if receipt.status != "pass" {
            blocker_codes.push("release_build_failed".to_string());
            next_actions.push(
                "Fix release build failures, then rerun `vida task close --release --json`."
                    .to_string(),
            );
        }
        Some(receipt)
    } else {
        None
    };

    let git = if command.commit || command.push || command.stage_owned {
        let receipt = task_close_git_automation_receipt(command, project_root, task);
        if receipt.status != "pass" {
            blocker_codes.extend(receipt.blocker_codes.iter().cloned());
            next_actions.extend(receipt.next_actions.iter().cloned());
        }
        Some(receipt)
    } else {
        None
    };

    TaskCloseAutomationReceipt {
        status: if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        blocker_codes,
        next_actions,
        release_build,
        release_install,
        git,
    }
}

fn task_close_git_automation_receipt(
    command: &TaskCloseArgs,
    project_root: Option<&std::path::Path>,
    task: Option<&state_store::TaskRecord>,
) -> TaskCloseGitAutomationReceipt {
    let explicit_files = task_close_commit_file_strings(command, task);
    let commit_message = command.commit_message.clone().or_else(|| {
        command
            .commit
            .then(|| format!("Close {}: {}", command.task_id, command.reason))
    });

    if command.push && !command.commit {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "push_requires_commit",
            "Pass `--commit --commit-file <path>` with `--push` so the pushed change is explicit.",
        );
    }
    if command.stage_owned && !command.commit {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "stage_owned_requires_commit",
            "Pass `--commit --stage-owned` so owned-path staging is tied to an explicit commit request.",
        );
    }
    if command.commit && explicit_files.is_empty() {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "dirty_ownership_ambiguous",
            "Pass one or more `--commit-file <path>` values, or pass `--stage-owned` when the task has planner_metadata.owned_paths.",
        );
    }

    let repo_root = project_root
        .map(std::path::Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let ignored_dirty_files = Vec::new();
    if command.commit {
        match dirty_paths_for_repo(&repo_root) {
            Ok(dirty_paths) => {
                let ambiguous: Vec<String> = dirty_paths
                    .into_iter()
                    .filter(|path| !path_is_explicitly_owned(path, &explicit_files))
                    .collect();
                if !ambiguous.is_empty() {
                    return blocked_task_close_git_receipt(
                        explicit_files,
                        commit_message,
                        "dirty_ownership_ambiguous",
                        "Clean unrelated dirty files or include only the owned paths with repeated `--commit-file` values.",
                    );
                }
            }
            Err(_) => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "git_status_failed",
                    "Run the command from a git worktree or resolve git status errors before committing.",
                );
            }
        }

        let stage_files: Vec<std::path::PathBuf> = explicit_files
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let add_status = std::process::Command::new("git")
            .arg("add")
            .arg("--")
            .args(&stage_files)
            .current_dir(&repo_root)
            .output();
        match add_status {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let failure = classify_task_close_git_stage_failure(
                    String::from_utf8_lossy(&output.stderr).trim(),
                    None,
                );
                return blocked_task_close_git_receipt_with_stage_detail(
                    explicit_files,
                    commit_message,
                    failure.blocker_code,
                    failure.next_action,
                    Some(failure.detail),
                );
            }
            Err(error) => {
                let failure = classify_task_close_git_stage_failure("", Some(&error));
                return blocked_task_close_git_receipt_with_stage_detail(
                    explicit_files,
                    commit_message,
                    failure.blocker_code,
                    failure.next_action,
                    Some(failure.detail),
                );
            }
        }

        let diff_status = std::process::Command::new("git")
            .args(["diff", "--cached", "--quiet", "--"])
            .args(&stage_files)
            .current_dir(&repo_root)
            .status();
        match diff_status {
            Ok(status) if status.success() => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "no_explicit_commit_changes",
                    "Ensure at least one explicit `--commit-file` has a staged content change.",
                );
            }
            Ok(status) if status.code() == Some(1) => {}
            _ => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "git_status_failed",
                    "Resolve git diff errors before committing.",
                );
            }
        }

        let message = commit_message
            .as_deref()
            .unwrap_or("Close task with post-close automation");
        let commit_status = std::process::Command::new("git")
            .args(["commit", "-m", message, "--"])
            .args(&stage_files)
            .current_dir(&repo_root)
            .status();
        match commit_status {
            Ok(status) if status.success() => {
                if command.push {
                    let push_status = std::process::Command::new("git")
                        .arg("push")
                        .current_dir(&repo_root)
                        .status();
                    match push_status {
                        Ok(push) if push.success() => TaskCloseGitAutomationReceipt {
                            status: "pass".to_string(),
                            blocker_codes: Vec::new(),
                            next_actions: task_close_commit_allowlist_next_actions(
                                &ignored_dirty_files,
                            ),
                            explicit_files,
                            stage_error_detail: None,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: push.code(),
                        },
                        Ok(push) => TaskCloseGitAutomationReceipt {
                            status: "blocked".to_string(),
                            blocker_codes: vec!["git_push_failed".to_string()],
                            next_actions: vec![
                                "Fix git push configuration or remote state, then push manually."
                                    .to_string(),
                            ],
                            explicit_files,
                            stage_error_detail: None,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: push.code(),
                        },
                        Err(_) => TaskCloseGitAutomationReceipt {
                            status: "blocked".to_string(),
                            blocker_codes: vec!["git_push_failed".to_string()],
                            next_actions: vec![
                                "Ensure `git push` can run in this worktree, then push manually."
                                    .to_string(),
                            ],
                            explicit_files,
                            stage_error_detail: None,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: None,
                        },
                    }
                } else {
                    TaskCloseGitAutomationReceipt {
                        status: "pass".to_string(),
                        blocker_codes: Vec::new(),
                        next_actions: task_close_commit_allowlist_next_actions(
                            &ignored_dirty_files,
                        ),
                        explicit_files,
                        stage_error_detail: None,
                        commit_message,
                        commit_exit_code: status.code(),
                        push_exit_code: None,
                    }
                }
            }
            Ok(status) => TaskCloseGitAutomationReceipt {
                status: "blocked".to_string(),
                blocker_codes: vec!["git_commit_failed".to_string()],
                next_actions: vec![
                    "Inspect git commit output and resolve commit blockers before retrying."
                        .to_string(),
                ],
                explicit_files,
                stage_error_detail: None,
                commit_message,
                commit_exit_code: status.code(),
                push_exit_code: None,
            },
            Err(_) => TaskCloseGitAutomationReceipt {
                status: "blocked".to_string(),
                blocker_codes: vec!["git_commit_failed".to_string()],
                next_actions: vec![
                    "Ensure `git commit` can run in this worktree before retrying.".to_string(),
                ],
                explicit_files,
                stage_error_detail: None,
                commit_message,
                commit_exit_code: None,
                push_exit_code: None,
            },
        }
    } else {
        TaskCloseGitAutomationReceipt {
            status: "pass".to_string(),
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
            explicit_files,
            stage_error_detail: None,
            commit_message,
            commit_exit_code: None,
            push_exit_code: None,
        }
    }
}

struct TaskCloseGitStageFailure<'a> {
    blocker_code: &'a str,
    next_action: &'a str,
    detail: String,
}

fn classify_task_close_git_stage_failure(
    stderr: &str,
    error: Option<&std::io::Error>,
) -> TaskCloseGitStageFailure<'static> {
    let normalized_stderr = stderr.trim();
    let normalized_lower = normalized_stderr.to_ascii_lowercase();
    if normalized_lower.contains("read-only")
        || normalized_lower.contains("permission denied")
        || normalized_lower.contains("operation not permitted")
        || normalized_lower.contains("sandbox")
    {
        let detail = if normalized_stderr.is_empty() {
            "git add failed because the worktree appears read-only or sandbox-blocked".to_string()
        } else {
            normalized_stderr.to_string()
        };
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_read_only_or_sandbox_blocked",
            next_action: "Make the worktree writable or rerun outside the blocking sandbox, then retry the task-close command.",
            detail,
        };
    }
    if normalized_lower.contains("index.lock") {
        let detail = if normalized_stderr.is_empty() {
            "git add failed because `.git/index.lock` is present".to_string()
        } else {
            normalized_stderr.to_string()
        };
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_index_lock_blocked",
            next_action: "Clear the `.git/index.lock` blocker or stop the concurrent git writer, then retry the task-close command.",
            detail,
        };
    }
    if let Some(error) = error {
        let detail = format!("git add failed to start: {error}");
        let lower = detail.to_ascii_lowercase();
        if lower.contains("read-only")
            || lower.contains("permission denied")
            || lower.contains("operation not permitted")
            || lower.contains("sandbox")
        {
            return TaskCloseGitStageFailure {
                blocker_code: "git_stage_read_only_or_sandbox_blocked",
                next_action: "Make the worktree writable or rerun outside the blocking sandbox, then retry the task-close command.",
                detail,
            };
        }
        if lower.contains("index.lock") {
            return TaskCloseGitStageFailure {
                blocker_code: "git_stage_index_lock_blocked",
                next_action: "Clear the `.git/index.lock` blocker or stop the concurrent git writer, then retry the task-close command.",
                detail,
            };
        }
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_failed",
            next_action: "Verify the explicit commit files exist and can be staged.",
            detail,
        };
    }

    TaskCloseGitStageFailure {
        blocker_code: "git_stage_failed",
        next_action: "Verify the explicit commit files exist and can be staged.",
        detail: if normalized_stderr.is_empty() {
            "git add failed without stderr output".to_string()
        } else {
            normalized_stderr.to_string()
        },
    }
}

fn task_close_commit_allowlist_next_actions(ignored_dirty_files: &[String]) -> Vec<String> {
    if ignored_dirty_files.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Ignored {} unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied.",
            ignored_dirty_files.len()
        )]
    }
}

fn blocked_task_close_git_receipt(
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    blocker_code: &str,
    next_action: &str,
) -> TaskCloseGitAutomationReceipt {
    blocked_task_close_git_receipt_with_stage_detail(
        explicit_files,
        commit_message,
        blocker_code,
        next_action,
        None,
    )
}

fn blocked_task_close_git_receipt_with_stage_detail(
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    blocker_code: &str,
    next_action: &str,
    stage_error_detail: Option<String>,
) -> TaskCloseGitAutomationReceipt {
    TaskCloseGitAutomationReceipt {
        status: "blocked".to_string(),
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        explicit_files,
        stage_error_detail,
        commit_message,
        commit_exit_code: None,
        push_exit_code: None,
    }
}

fn task_close_commit_file_strings(
    command: &TaskCloseArgs,
    task: Option<&state_store::TaskRecord>,
) -> Vec<String> {
    if !command.commit_files.is_empty() {
        let files: Vec<String> = command
            .commit_files
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        return canonical_owned_paths(files);
    }

    if command.stage_owned {
        if let Some(task) = task {
            return canonical_owned_paths(task.planner_metadata.owned_paths.clone());
        }
    }
    Vec::new()
}

fn canonical_owned_paths(paths: Vec<String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for path in paths {
        let trimmed = path.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
}

fn task_owned_status_receipt(
    task_id: &str,
    metadata_owned_paths: Vec<String>,
    override_files: Vec<String>,
    dirty_files: Vec<String>,
) -> TaskOwnedStatusReceipt {
    let override_files = canonical_owned_paths(override_files);
    let metadata_owned_paths = canonical_owned_paths(metadata_owned_paths);
    let (owned_paths, ownership_source) = if !override_files.is_empty() {
        (override_files, "explicit_file_overrides".to_string())
    } else if !metadata_owned_paths.is_empty() {
        (
            metadata_owned_paths,
            "planner_metadata.owned_paths".to_string(),
        )
    } else {
        (Vec::new(), "missing".to_string())
    };

    if owned_paths.is_empty() {
        return TaskOwnedStatusReceipt {
            status: "blocked".to_string(),
            blocker_codes: vec!["missing_owned_paths".to_string()],
            next_actions: vec![
                "Add planner_metadata.owned_paths to the task or rerun with repeated `--file <path>` overrides.".to_string(),
            ],
            task_id: task_id.to_string(),
            ownership_source,
            owned_paths,
            dirty_files,
            owned_files: Vec::new(),
            unowned_files: Vec::new(),
            stageable_files: Vec::new(),
        };
    }

    let mut owned_files = Vec::new();
    let mut unowned_files = Vec::new();
    for path in &dirty_files {
        if path_is_explicitly_owned(path, &owned_paths) {
            owned_files.push(path.clone());
        } else {
            unowned_files.push(path.clone());
        }
    }
    let stageable_files = owned_files.clone();
    let blocked = !unowned_files.is_empty();

    TaskOwnedStatusReceipt {
        status: if blocked { "blocked" } else { "pass" }.to_string(),
        blocker_codes: if blocked {
            vec!["dirty_ownership_ambiguous".to_string()]
        } else {
            Vec::new()
        },
        next_actions: if blocked {
            vec![
                "Commit/stash unrelated dirty files or expand the explicit owned path set before staging.".to_string(),
            ]
        } else if stageable_files.is_empty() {
            vec!["No dirty files are covered by the selected ownership source.".to_string()]
        } else {
            vec!["Stage only `stageable_files` before committing this task.".to_string()]
        },
        task_id: task_id.to_string(),
        ownership_source,
        owned_paths,
        dirty_files,
        owned_files,
        unowned_files,
        stageable_files,
    }
}

fn task_handoff_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render")
}

fn task_handoff_receipt_filename_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos()
        .to_string()
}

fn task_handoff_project_receipt_root(project_root: &std::path::Path) -> std::path::PathBuf {
    project_root.join(".vida").join("receipts")
}

fn task_handoff_isolated_receipt_root(state_dir: &std::path::Path) -> std::path::PathBuf {
    state_dir.join("receipts")
}

fn task_handoff_receipt_dir(receipt_root: &std::path::Path) -> std::path::PathBuf {
    receipt_root.join("task-handoffs")
}

fn task_handoff_receipt_root(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
) -> (std::path::PathBuf, &'static str) {
    if task_close_uses_isolated_state_dir(state_dir, explicit_state_dir) {
        return (
            task_handoff_isolated_receipt_root(state_dir),
            "isolated_state_dir",
        );
    }
    let project_root = project_root_for_task_state(state_dir)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    (
        task_handoff_project_receipt_root(&project_root),
        "project_state_dir",
    )
}

fn sanitize_task_handoff_receipt_component(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.to_string()
    }
}

fn task_handoff_receipt_path(
    receipt_root: &std::path::Path,
    task_id: &str,
    filename_timestamp: &str,
) -> std::path::PathBuf {
    task_handoff_receipt_dir(receipt_root).join(format!(
        "{}-{}.json",
        sanitize_task_handoff_receipt_component(task_id),
        filename_timestamp
    ))
}

fn canonical_nonempty_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
}

fn blocked_task_handoff_accept_receipt(
    task_id: &str,
    agent_id: &str,
    blocker_code: &str,
    next_action: &str,
) -> TaskHandoffAcceptReceipt {
    TaskHandoffAcceptReceipt {
        status: "blocked".to_string(),
        task_id: task_id.to_string(),
        agent_id: agent_id.to_string(),
        accepted_at: task_handoff_timestamp(),
        changed_files: Vec::new(),
        proof_commands: Vec::new(),
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        receipt_path: "not_persisted".to_string(),
        receipt_root: "not_persisted".to_string(),
        isolation: "not_persisted".to_string(),
    }
}

fn task_handoff_accept_receipt(
    command: &TaskHandoffAcceptArgs,
    receipt_path: &std::path::Path,
    receipt_root: &std::path::Path,
    isolation: &str,
    accepted_at: String,
) -> TaskHandoffAcceptReceipt {
    let changed_files = canonical_owned_paths(
        command
            .files
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    );
    let proof_commands = canonical_nonempty_strings(command.proofs.clone());
    let blocker_codes = canonical_nonempty_strings(command.blockers.clone());
    let next_actions = canonical_nonempty_strings(command.next_actions.clone());
    TaskHandoffAcceptReceipt {
        status: command.status.as_str().to_string(),
        task_id: command.task_id.trim().to_string(),
        agent_id: command.agent.as_deref().unwrap_or("").trim().to_string(),
        accepted_at,
        changed_files,
        proof_commands,
        blocker_codes,
        next_actions,
        receipt_path: receipt_path.display().to_string(),
        receipt_root: receipt_root.display().to_string(),
        isolation: isolation.to_string(),
    }
}

fn validate_task_handoff_accept_receipt(
    receipt: &TaskHandoffAcceptReceipt,
) -> Result<(), (&'static str, &'static str)> {
    if receipt.agent_id.trim().is_empty() {
        return Err((
            "missing_agent_id",
            "Pass `--agent <id>` with the delegated agent or carrier id.",
        ));
    }
    if receipt.status == "blocked"
        && receipt.blocker_codes.is_empty()
        && receipt.proof_commands.is_empty()
    {
        return Err((
            "blocked_handoff_requires_detail",
            "Pass `--blocker <code>` or `--proof <command>` when accepting a blocked handoff.",
        ));
    }
    Ok(())
}

fn persist_task_handoff_accept_receipt(
    receipt: &TaskHandoffAcceptReceipt,
    receipt_path: &std::path::Path,
) -> Result<(), String> {
    if let Some(parent) = receipt_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create task handoff receipt directory `{}`: {error}",
                parent.display()
            )
        })?;
    }
    let rendered = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("failed to render task handoff receipt json: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(receipt_path)
        .map_err(|error| {
            format!(
                "failed to create task handoff receipt `{}` without overwrite: {error}",
                receipt_path.display()
            )
        })?;
    use std::io::Write;
    file.write_all(&rendered).map_err(|error| {
        format!(
            "failed to write task handoff receipt `{}`: {error}",
            receipt_path.display()
        )
    })?;
    file.write_all(b"\n").map_err(|error| {
        format!(
            "failed to finish task handoff receipt `{}`: {error}",
            receipt_path.display()
        )
    })
}

fn task_continuation_candidate(
    task: &state_store::TaskRecord,
    ready_parallel_safe: bool,
) -> TaskContinuationCandidate {
    TaskContinuationCandidate {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
        ready_parallel_safe,
    }
}

fn task_continuation_active_unit(task: &state_store::TaskRecord) -> serde_json::Value {
    serde_json::json!({
        "task_id": task.id,
        "title": task.title,
        "status": task.status,
        "issue_type": task.issue_type,
    })
}

fn task_next_lawful_bind_command(candidate: &TaskContinuationCandidate) -> String {
    format!(
        "vida taskflow run-graph dispatch-init {} --json",
        crate::shell_quote(&candidate.task_id)
    )
}

fn task_next_lawful_recommended_primary(
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Option<TaskContinuationCandidate> {
    ready_task_candidates.first().cloned()
}

fn task_next_lawful_recommended_parallel_batch(
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Vec<TaskContinuationCandidate> {
    ready_task_candidates
        .iter()
        .filter(|candidate| candidate.ready_parallel_safe)
        .cloned()
        .collect()
}

fn task_epic_ancestor_id(tasks: &[state_store::TaskRecord], task_id: &str) -> Option<String> {
    let by_id = tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut current_id = task_id;
    let mut visited = std::collections::BTreeSet::<String>::new();
    loop {
        if !visited.insert(current_id.to_string()) {
            return None;
        }
        let task = by_id.get(current_id)?;
        if state_store::work_item_is_program_container(&task.issue_type) {
            return Some(task.id.clone());
        }
        let Some(parent_id) = task_parent_id(task) else {
            return None;
        };
        current_id = by_id.get(parent_id.as_str())?.id.as_str();
    }
}

fn task_next_lawful_apply_strategy(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    strategy: Option<&str>,
) -> Vec<TaskContinuationCandidate> {
    match strategy.unwrap_or("default") {
        "epic-sequential" => {
            let Some(primary) = ready_task_candidates.first() else {
                return ready_task_candidates;
            };
            let primary_epic_id = task_epic_ancestor_id(tasks, &primary.task_id);
            ready_task_candidates
                .into_iter()
                .filter(|candidate| {
                    task_epic_ancestor_id(tasks, &candidate.task_id) == primary_epic_id
                })
                .collect()
        }
        _ => ready_task_candidates,
    }
}

fn task_next_lawful_why_not_auto_bound(
    blocker_code: Option<&str>,
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Option<String> {
    match blocker_code {
        Some("ambiguous_ready_task_candidates") => Some(format!(
            "multiple ready candidates ({}) require an explicit bounded-unit binding; recommendations are ranked guidance only",
            ready_task_candidates.len()
        )),
        Some("multiple_active_tasks") => Some(
            "multiple active TaskFlow tasks require reconciliation before automatic binding".to_string(),
        ),
        Some("runtime_ready_candidate_conflict") => Some(
            "runtime binding conflicts with ready TaskFlow candidates, so operator confirmation is required".to_string(),
        ),
        Some("continuation_source_drift") => Some(
            "continuation sources disagree, so automatic binding would risk selecting the wrong bounded unit".to_string(),
        ),
        Some(_) => Some("blocking runtime evidence prevents automatic binding".to_string()),
        None => None,
    }
}

fn task_continuation_source_surfaces() -> Vec<String> {
    vec![
        "vida task next-lawful".to_string(),
        "StateStore::latest_explicit_run_graph_continuation_binding".to_string(),
        "StateStore::latest_run_graph_status".to_string(),
        "StateStore::run_graph_continuation_binding(latest_run_id)".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida task ready --json".to_string(),
        "vida status --json continuation_binding".to_string(),
        "vida taskflow run-graph status --json projection_truth.continuation_binding".to_string(),
    ]
}

fn continuation_binding_active_kind(binding: &state_store::RunGraphContinuationBinding) -> &str {
    binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}

fn continuation_binding_requires_open_task(
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    continuation_binding_active_kind(binding) != "downstream_dispatch_target"
}

fn task_status_for_binding<'a>(
    tasks: &'a [state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> Option<&'a str> {
    tasks
        .iter()
        .find(|task| task.id == binding.task_id)
        .map(|task| task.status.as_str())
}

fn continuation_binding_has_live_unit(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    if !continuation_binding_requires_open_task(binding) {
        return true;
    }
    task_status_for_binding(tasks, binding).is_some_and(|status| status != "closed")
}

fn continuation_binding_is_closed_downstream_marker(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    !continuation_binding_requires_open_task(binding)
        && task_status_for_binding(tasks, binding).is_some_and(|status| status == "closed")
}

fn task_exists_for_binding(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    tasks.iter().any(|task| task.id == binding.task_id)
}

fn continuation_bindings_same_unit(
    left: &state_store::RunGraphContinuationBinding,
    right: &state_store::RunGraphContinuationBinding,
) -> bool {
    left.run_id == right.run_id
        && left.task_id == right.task_id
        && left.active_bounded_unit == right.active_bounded_unit
}

fn continuation_binding_is_historical_task_close_reconcile(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "task_close_reconcile" && explicit.run_id != current.run_id
}

fn continuation_binding_is_superseded_same_task_explicit(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && explicit.run_id != current.run_id
        && explicit.task_id == current.task_id
}

fn continuation_binding_is_unscoped_dispatch_init_projection(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "run_graph_dispatch_init"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
}

fn continuation_binding_is_unrelated_prelaunch_blocked_projection(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "dispatch_prelaunch_blocked"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
}

fn continuation_binding_is_newer_explicit_task_override(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "explicit_continuation_bind"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
        && explicit.recorded_at > current.recorded_at
}

fn select_task_next_lawful_binding<'a>(
    tasks: &[state_store::TaskRecord],
    explicit_binding: Option<&'a state_store::RunGraphContinuationBinding>,
    current_binding: Option<&'a state_store::RunGraphContinuationBinding>,
) -> Result<Option<&'a state_store::RunGraphContinuationBinding>, TaskNextLawfulReceipt> {
    let has_single_active_task =
        crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks).len() == 1;
    match (explicit_binding, current_binding) {
        (Some(explicit), Some(current)) if !continuation_bindings_same_unit(explicit, current) => {
            let explicit_live = continuation_binding_has_live_unit(tasks, explicit);
            let current_live = continuation_binding_has_live_unit(tasks, current);
            if continuation_binding_is_historical_task_close_reconcile(explicit, current)
                && current_live
            {
                return Ok(Some(current));
            }
            if continuation_binding_is_superseded_same_task_explicit(explicit, current)
                && current_live
            {
                return Ok(Some(current));
            }
            if continuation_binding_is_unscoped_dispatch_init_projection(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            if continuation_binding_is_unrelated_prelaunch_blocked_projection(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            if continuation_binding_is_newer_explicit_task_override(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            match (explicit_live, current_live) {
                (false, false) => return Ok(None),
                (false, true) => return Ok(Some(current)),
                (true, false) => return Ok(Some(explicit)),
                (true, true) => {}
            }
            Err(blocked_task_next_lawful_receipt(
                explicit.active_bounded_unit.clone(),
                Vec::new(),
                "continuation_source_drift",
                &format!(
                    "Continuation sources disagree: explicit binding `{}`/`{}` points to `{}`, while current latest-run binding `{}`/`{}` from `{}` points to `{}`. Inspect current blocked-run recovery with `vida taskflow recovery status {} --json`, lane evidence with `vida lane show {} --json`, and explicit binding state with `vida taskflow run-graph status {} --json` before continuing.",
                    explicit.run_id,
                    explicit.binding_source,
                    explicit.task_id,
                    current.run_id,
                    current.binding_source,
                    current.binding_source,
                    current.task_id,
                    crate::shell_quote(&current.run_id),
                    crate::shell_quote(&current.run_id),
                    crate::shell_quote(&explicit.run_id)
                ),
            ))
        }
        (Some(explicit), Some(_current)) => {
            if continuation_binding_has_live_unit(tasks, explicit) {
                Ok(Some(explicit))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(explicit))
            }
        }
        (Some(explicit), None) => {
            if continuation_binding_has_live_unit(tasks, explicit) {
                Ok(Some(explicit))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(explicit))
            }
        }
        (None, Some(current)) => {
            if continuation_binding_has_live_unit(tasks, current) {
                Ok(Some(current))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(current))
            }
        }
        (None, None) => Ok(None),
    }
}

fn blocked_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_code: &str,
    next_action: &str,
) -> TaskNextLawfulReceipt {
    let next_actions = vec![next_action.to_string()];
    let recommended_primary = task_next_lawful_recommended_primary(&ready_task_candidates);
    let bind_command = recommended_primary
        .as_ref()
        .map(task_next_lawful_bind_command);
    let recommended_parallel_batch =
        task_next_lawful_recommended_parallel_batch(&ready_task_candidates);
    let why_not_auto_bound =
        task_next_lawful_why_not_auto_bound(Some(blocker_code), &ready_task_candidates);
    TaskNextLawfulReceipt {
        status: "blocked".to_string(),
        active_bounded_unit,
        binding_source: None,
        why_this_unit: "blocked_until_unique_lawful_continuation_is_evidenced".to_string(),
        sequential_vs_parallel_posture: "unknown_until_explicit_binding".to_string(),
        recommended_primary,
        recommended_parallel_batch,
        why_not_auto_bound,
        bind_command,
        ready_task_candidates,
        blocker_codes: vec![blocker_code.to_string()],
        next_action: next_actions.first().cloned(),
        next_actions,
        source_surfaces: task_continuation_source_surfaces(),
        operator_explanation: None,
    }
}

fn pass_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    binding_source: Option<String>,
    why_this_unit: &str,
    sequential_vs_parallel_posture: &str,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    next_action: String,
) -> TaskNextLawfulReceipt {
    let next_actions = vec![next_action];
    let recommended_primary = task_next_lawful_recommended_primary(&ready_task_candidates);
    let bind_command = recommended_primary
        .as_ref()
        .map(task_next_lawful_bind_command);
    let recommended_parallel_batch =
        task_next_lawful_recommended_parallel_batch(&ready_task_candidates);
    TaskNextLawfulReceipt {
        status: task_json_success_status().to_string(),
        active_bounded_unit,
        binding_source,
        why_this_unit: why_this_unit.to_string(),
        sequential_vs_parallel_posture: sequential_vs_parallel_posture.to_string(),
        recommended_primary,
        recommended_parallel_batch,
        why_not_auto_bound: None,
        bind_command,
        ready_task_candidates,
        blocker_codes: Vec::new(),
        next_action: next_actions.first().cloned(),
        next_actions,
        source_surfaces: task_continuation_source_surfaces(),
        operator_explanation: None,
    }
}

fn task_next_lawful_attach_explanation(
    mut receipt: TaskNextLawfulReceipt,
    explain: bool,
    strategy: Option<&str>,
    selected_task_id: Option<&str>,
) -> TaskNextLawfulReceipt {
    if explain {
        receipt.operator_explanation = Some(serde_json::json!({
            "strategy": strategy.unwrap_or("default"),
            "selected_task_id": selected_task_id,
            "status": receipt.status,
            "blocker_codes": receipt.blocker_codes,
            "why_this_unit": receipt.why_this_unit,
            "why_not_auto_bound": receipt.why_not_auto_bound,
            "bind_command": receipt.bind_command,
            "candidate_count": receipt.ready_task_candidates.len()
        }));
    }
    receipt
}

fn task_next_lawful_select_ready_candidate_receipt(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    selected_task_id: &str,
) -> TaskNextLawfulReceipt {
    match crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks).as_slice() {
        [active] => {
            return blocked_task_next_lawful_receipt(
                task_continuation_active_unit(active),
                ready_task_candidates,
                "select_conflicts_with_active_taskflow_task",
                &format!(
                    "TaskFlow task `{}` is already in_progress; continue or close it before selecting another continuation item.",
                    active.id
                ),
            );
        }
        [] => {}
        _ => {
            return blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "multiple_active_tasks",
                "Close or reconcile extra in_progress tasks before selecting a continuation item.",
            );
        }
    }

    let Some(selected_index) = ready_task_candidates
        .iter()
        .position(|candidate| candidate.task_id == selected_task_id)
    else {
        return blocked_task_next_lawful_receipt(
            serde_json::Value::Null,
            ready_task_candidates,
            "selected_task_not_ready",
            &format!(
                "Selected task `{}` is not a ready lawful candidate; choose one of the returned ready_task_candidates.",
                selected_task_id
            ),
        );
    };
    let mut ordered_candidates = ready_task_candidates;
    let selected = ordered_candidates.remove(selected_index);
    ordered_candidates.insert(0, selected.clone());
    pass_task_next_lawful_receipt(
        serde_json::json!({
            "task_id": selected.task_id,
            "title": selected.title,
            "status": selected.status,
            "issue_type": selected.issue_type,
        }),
        Some("operator_selected_ready_candidate".to_string()),
        "Operator selected a ready TaskFlow candidate with --select.",
        if selected.ready_parallel_safe {
            "parallel_safe_operator_selected_candidate"
        } else {
            "sequential_only_operator_selected_candidate"
        },
        ordered_candidates,
        format!(
            "Bind selected ready task `{}` with the returned bind_command.",
            selected_task_id
        ),
    )
}

fn runtime_binding_task_closed_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let run_id = binding.run_id.trim();
    if run_id.is_empty() {
        return crate::status_surface_signals::continuation_binding_ambiguous_next_action()
            .to_string();
    }
    let run_id = crate::shell_quote(run_id);
    format!(
        "Runtime binding points to closed task `{}` for run `{run_id}`. Inspect the concrete recovery state with `vida taskflow recovery status {run_id} --json`; resolve or retire the blocked run, then refresh continuation evidence with `vida taskflow consume continue --json` before selecting the next bounded step.",
        binding.task_id
    )
}

fn runtime_binding_task_paused_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let task_id = crate::shell_quote(&binding.task_id);
    let run_id = crate::shell_quote(&binding.run_id);
    format!(
        "Runtime binding points to paused task `{}`. Resume it with `vida task update {} --status in_progress --json`, or bind a different lawful unit with `vida taskflow continuation bind {} --task-id <task-id> --json` if the pause is still intentional.",
        binding.task_id, task_id, run_id
    )
}

fn runtime_binding_task_missing_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let base = crate::status_surface_signals::runtime_binding_task_missing_next_action(
        Some(binding.run_id.as_str()),
        &binding.task_id,
    );
    let run_id = binding.run_id.trim();
    if run_id.is_empty() {
        return base;
    }
    let run_id = crate::shell_quote(run_id);
    format!(
        "{base} After recovery proves the run is safe to rebind, record the explicit replacement with `vida taskflow continuation bind {run_id} --task-id <task-id> --json` for missing task `{}`.",
        binding.task_id
    )
}

fn runtime_binding_open_delegated_cycle_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    format!(
        "Runtime binding for task `{}` is still inside an open delegated cycle for run `{}`. Inspect `vida lane show {} --json` and `vida taskflow recovery status {} --json`; wait for a receipt-backed delegated completion or record structured exception takeover before selecting another TaskFlow step.",
        binding.task_id, binding.run_id, binding.run_id, binding.run_id
    )
}

fn runtime_dispatch_receipt_has_ready_downstream_handoff(
    expected_run_id: Option<&str>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    dispatch.is_some_and(|dispatch| {
        expected_run_id.is_some_and(|run_id| dispatch.run_id == run_id)
            && dispatch.dispatch_status == "executed"
            && dispatch.blocker_code.is_none()
            && dispatch.downstream_dispatch_ready
            && dispatch.downstream_dispatch_blockers.is_empty()
            && dispatch
                .downstream_dispatch_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"))
    })
}

fn runtime_dispatch_receipt_has_completed_lane(
    expected_run_id: Option<&str>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    dispatch.is_some_and(|dispatch| {
        expected_run_id.is_some_and(|run_id| dispatch.run_id == run_id)
            && dispatch.dispatch_status == "executed"
            && dispatch.lane_status == "lane_completed"
            && dispatch.blocker_code.is_none()
    })
}

fn downstream_dispatch_command_for_task_next_lawful(
    dispatch: &state_store::RunGraphDispatchReceiptSummary,
) -> Option<String> {
    crate::continuation_binding_summary::downstream_dispatch_command_for_summary(dispatch)
}

fn runtime_recovery_blocks_task_next_lawful(
    recovery: Option<&state_store::RunGraphRecoverySummary>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    recovery.is_some_and(|recovery| {
        (recovery.delegation_gate.delegated_cycle_open
            || recovery.delegation_gate.local_exception_takeover_gate
                == "blocked_open_delegated_cycle"
            || recovery.resume_status == "running")
            && !runtime_dispatch_receipt_has_ready_downstream_handoff(
                Some(recovery.run_id.as_str()),
                dispatch,
            )
            && !runtime_dispatch_receipt_has_completed_lane(
                Some(recovery.run_id.as_str()),
                dispatch,
            )
    })
}

fn runtime_binding_has_active_exception_takeover(
    binding: &state_store::RunGraphContinuationBinding,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    let Some(dispatch) = dispatch else {
        return false;
    };
    let exception_takeover_state = crate::release1_contracts::exception_takeover_state(
        dispatch.exception_path_receipt_id.as_deref(),
        dispatch.supersedes_receipt_id.as_deref(),
        None,
    );
    dispatch.run_id == binding.run_id
        && (dispatch.lane_status == "lane_exception_takeover"
            || exception_takeover_state.is_active())
        && dispatch
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && dispatch
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn pass_exception_takeover_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
) -> TaskNextLawfulReceipt {
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_exception_takeover_dispatch".to_string()),
        &format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}`.",
            binding.task_id
        ),
        "sequential_only_exception_takeover",
        ready_task_candidates,
        format!(
            "Finish the active exception-takeover unit for `{}` before selecting another TaskFlow step.",
            binding.task_id
        ),
    )
}

fn pass_ready_downstream_handoff_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    terminal_consume_continue_run_id: Option<&str>,
    downstream_dispatch_command: Option<&str>,
) -> TaskNextLawfulReceipt {
    let next_action = if terminal_consume_continue_run_id == Some(binding.run_id.as_str()) {
        downstream_dispatch_command
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|command| {
                format!(
                    "Continue `{}` with downstream handoff command `{}`.",
                    binding.task_id, command
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "Inspect `{}` with `vida lane show {} --json`.",
                    binding.task_id,
                    crate::shell_quote(&binding.run_id)
                )
            })
    } else {
        format!(
            "Continue `{}` with `vida taskflow consume continue --run-id {} --json`.",
            binding.task_id,
            crate::shell_quote(&binding.run_id)
        )
    };
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_ready_downstream_handoff".to_string()),
        &format!(
            "Latest runtime dispatch records a ready downstream handoff for task `{}`.",
            binding.task_id
        ),
        "sequential_only_downstream_bound",
        ready_task_candidates,
        next_action,
    )
}

fn pass_completed_lane_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
) -> TaskNextLawfulReceipt {
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_completed_dispatch_receipt".to_string()),
        &format!(
            "Latest dispatch receipt records completed delegated lane evidence for task `{}`.",
            binding.task_id
        ),
        "sequential_only_completed_lane_reconciled",
        ready_task_candidates,
        format!(
            "Continue `{}` after completed delegated lane reconciliation; inspect `vida taskflow run-graph status {} --json` if downstream binding is still expected.",
            binding.task_id,
            crate::shell_quote(&binding.run_id)
        ),
    )
}

fn task_next_lawful_receipt(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    runtime_binding: Option<&state_store::RunGraphContinuationBinding>,
) -> TaskNextLawfulReceipt {
    let active_tasks = crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks);

    if let Some(binding) = runtime_binding {
        let binding_task = tasks.iter().find(|task| task.id == binding.task_id);
        let missing_runtime_binding_with_single_active_task =
            continuation_binding_requires_open_task(binding)
                && binding_task.is_none()
                && active_tasks.len() == 1;
        if !missing_runtime_binding_with_single_active_task
            && !continuation_binding_is_closed_downstream_marker(tasks, binding)
        {
            let conflicting_active = active_tasks
                .iter()
                .find(|task| task.id != binding.task_id)
                .map(|task| task.id.clone());
            if let Some(conflicting_task_id) = conflicting_active {
                return blocked_task_next_lawful_receipt(
                    binding.active_bounded_unit.clone(),
                    ready_task_candidates,
                    "runtime_taskflow_active_conflict",
                    &format!(
                        "Runtime binding points to `{}` but TaskFlow has active `{}`; reconcile or close the stale active task before continuing.",
                        binding.task_id, conflicting_task_id
                    ),
                );
            }
            if continuation_binding_requires_open_task(binding) {
                let Some(task) = binding_task else {
                    return blocked_task_next_lawful_receipt(
                        binding.active_bounded_unit.clone(),
                        ready_task_candidates,
                        "runtime_binding_task_missing",
                        &runtime_binding_task_missing_next_action(binding),
                    );
                };
                if task.status == "closed" {
                    return blocked_task_next_lawful_receipt(
                        binding.active_bounded_unit.clone(),
                        ready_task_candidates,
                        "runtime_binding_task_closed",
                        &runtime_binding_task_closed_next_action(binding),
                    );
                }
                if task.status == "paused" {
                    return blocked_task_next_lawful_receipt(
                        binding.active_bounded_unit.clone(),
                        ready_task_candidates,
                        "runtime_binding_task_paused",
                        &runtime_binding_task_paused_next_action(binding),
                    );
                }
            }
            let ready_conflict = ready_task_candidates
                .iter()
                .any(|candidate| candidate.task_id != binding.task_id);
            if ready_conflict
                && binding.binding_source != "explicit_continuation_bind_task"
                && !ready_task_candidates
                    .iter()
                    .any(|candidate| candidate.task_id == binding.task_id)
            {
                return blocked_task_next_lawful_receipt(
                    binding.active_bounded_unit.clone(),
                    ready_task_candidates,
                    "runtime_ready_candidate_conflict",
                    crate::status_surface_signals::continuation_binding_ambiguous_next_action(),
                );
            }
            return pass_task_next_lawful_receipt(
                binding.active_bounded_unit.clone(),
                Some(binding.binding_source.clone()),
                &binding.why_this_unit,
                &binding.sequential_vs_parallel_posture,
                ready_task_candidates,
                format!(
                    "Continue `{}` via the bound runtime path: {}.",
                    binding.task_id, binding.primary_path
                ),
            );
        }
    }

    match active_tasks.as_slice() {
        [active] => pass_task_next_lawful_receipt(
            task_continuation_active_unit(active),
            Some("taskflow_single_in_progress".to_string()),
            "Single TaskFlow in_progress task is the authoritative active bounded unit.",
            "sequential_only_taskflow_active",
            ready_task_candidates,
            format!("Continue active task `{}`.", active.id),
        ),
        [] => match ready_task_candidates.as_slice() {
            [candidate] => pass_task_next_lawful_receipt(
                serde_json::json!({
                    "task_id": candidate.task_id,
                    "title": candidate.title,
                    "status": candidate.status,
                    "issue_type": candidate.issue_type,
                }),
                None,
                "single ready TaskFlow candidate after close/release automation",
                if candidate.ready_parallel_safe {
                    "parallel_safe_single_candidate"
                } else {
                    "sequential_only_single_candidate"
                },
                ready_task_candidates.clone(),
                format!("Continue ready task `{}`.", candidate.task_id),
            ),
            [] => blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "no_ready_task_candidates",
                "Create/import the next task or refresh TaskFlow state before continuing.",
            ),
            _ => blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "ambiguous_ready_task_candidates",
                "Multiple ready tasks are available; choose and bind the intended bounded unit explicitly before implementation.",
            ),
        },
        _ => blocked_task_next_lawful_receipt(
            serde_json::Value::Null,
            ready_task_candidates,
            "multiple_active_tasks",
            "Close or reconcile extra in_progress tasks before selecting a continuation item.",
        ),
    }
}

fn dirty_paths_for_repo(repo_root: &std::path::Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(porcelain_status_path)
        .collect::<Vec<_>>())
}

fn porcelain_status_path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    if path.is_empty() {
        None
    } else {
        Some(
            path.rsplit_once(" -> ")
                .map(|(_, destination)| destination)
                .unwrap_or(path)
                .to_string(),
        )
    }
}

fn path_is_explicitly_owned(path: &str, explicit_files: &[String]) -> bool {
    explicit_files.iter().any(|explicit| {
        path == explicit
            || path
                .strip_prefix(explicit)
                .map(|suffix| suffix.starts_with('/'))
                .unwrap_or(false)
    })
}

pub(crate) async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::Help(command) => match command.topic.as_deref() {
            None | Some("task") => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some("parallelism" | "scheduling") => {
                print_taskflow_proxy_help(Some("parallelism"));
                ExitCode::SUCCESS
            }
            Some("next") => {
                print_taskflow_proxy_help(Some("next"));
                ExitCode::SUCCESS
            }
            Some("graph-summary") => {
                print_taskflow_proxy_help(Some("graph-summary"));
                ExitCode::SUCCESS
            }
            Some(
                "ready"
                | "deps"
                | "reverse-deps"
                | "blocked"
                | "children"
                | "reparent-children"
                | "move-children"
                | "defect-batch-rehome"
                | "defect-batch"
                | "tree"
                | "subtree"
                | "critical-path"
                | "next-display-id"
                | "create"
                | "ensure"
                | "update"
                | "close"
                | "split"
                | "spawn-blocker"
                | "list"
                | "adaptive-preview"
                | "show"
                | "import-jsonl"
                | "replace-jsonl"
                | "export-jsonl"
                | "validate-graph"
                | "dep"
                | "handoff"
                | "next-lawful",
            ) => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some(topic) => {
                eprintln!("Unsupported task help topic: {topic}");
                ExitCode::from(2)
            }
        },
        TaskCommand::ImportJsonl(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
                    Ok(summary) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task import-jsonl")
                                .await
                        {
                            return code;
                        }
                        if command.json {
                            let mut summary_json = serde_json::json!({
                                "status": task_json_success_status(),
                                "source_path": summary.source_path,
                                "imported_count": summary.imported_count,
                                "unchanged_count": summary.unchanged_count,
                                "updated_count": summary.updated_count,
                            });
                            if let Err(error) =
                                normalize_task_json_contract_arrays(&mut summary_json)
                            {
                                eprintln!("Failed to render task import-jsonl json: {error}");
                                return ExitCode::from(1);
                            }
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&summary_json)
                                    .expect("json import summary should render")
                            );
                        } else {
                            print_surface_header(command.render, "vida task import-jsonl");
                            print_surface_line(command.render, "import", &summary.as_display());
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        if command.json {
                            let mut payload = task_import_jsonl_error_payload(
                                &command.path.display().to_string(),
                                &error.to_string(),
                            );
                            if let Err(render_error) =
                                normalize_task_json_contract_arrays(&mut payload)
                            {
                                eprintln!(
                                    "Failed to render task import-jsonl json: {render_error}"
                                );
                                return ExitCode::from(1);
                            }
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&payload)
                                    .expect("json import error should render")
                            );
                        } else {
                            eprintln!("Failed to import tasks from JSONL: {error}");
                        }
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReplaceJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store
                    .replace_with_taskflow_snapshot_file(&command.path)
                    .await
                {
                    Ok(()) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task replace-jsonl")
                                .await
                        {
                            return code;
                        }
                        let source_path = command.path.display().to_string();
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": task_json_success_status(),
                                "operation": "replace_snapshot",
                                "source_path": source_path,
                            }));
                        } else {
                            print_surface_header(command.render, "vida task replace-jsonl");
                            print_surface_line(command.render, "status", "pass");
                            print_surface_line(command.render, "operation", "replace_snapshot");
                            print_surface_line(command.render, "source path", &source_path);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to replace tasks from snapshot file: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ExportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_read_only_task_store(state_dir).await {
                Ok(store) => match store.export_tasks_to_jsonl(&command.path).await {
                    Ok(exported_count) => {
                        print_task_export_summary(
                            command.render,
                            u64::try_from(exported_count)
                                .expect("task export count should fit u64"),
                            &command.path.display().to_string(),
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to export tasks to JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::List(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_list_authoritative_first(state_dir, command.status.as_deref(), command.all)
                .await
            {
                Ok((tasks, metadata)) => {
                    let summary_only = command.summary || !command.all;
                    print_task_list(
                        command.render,
                        &tasks,
                        summary_only,
                        command.all,
                        command.json,
                        Some(&metadata),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to list tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            if command.json {
                let projection_name = task_show_projection_name(&command.task_id);
                if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
                    &state_dir,
                    &projection_name,
                ) {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
                if let Some(cached) =
                    crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                        &state_dir,
                        &projection_name,
                        TASK_READ_RECENT_PROJECTION_MAX_AGE,
                    )
                {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
            }
            match task_show_authoritative_first(state_dir.clone(), &command.task_id).await {
                Ok((task, metadata)) => {
                    if command.json {
                        let payload = task_show_payload(&task, Some(&metadata));
                        crate::print_json_pretty(&payload);
                        crate::operator_projection_cache::write_json_projection(
                            &state_dir,
                            &task_show_projection_name(&command.task_id),
                            &payload,
                        );
                    } else {
                        print_task_show(command.render, &task, false, Some(&metadata));
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to show task: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::OwnedStatus(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let repo_root = project_root_for_task_state(&state_dir)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            match task_show_authoritative_first(state_dir, &command.task_id).await {
                Ok((task, _metadata)) => {
                    let dirty_files = match dirty_paths_for_repo(&repo_root) {
                        Ok(paths) => paths,
                        Err(error) => {
                            let receipt = TaskOwnedStatusReceipt {
                                status: "blocked".to_string(),
                                blocker_codes: vec!["git_status_failed".to_string()],
                                next_actions: vec![
                                    "Run the command from a git worktree or resolve git status errors before staging.".to_string(),
                                ],
                                task_id: command.task_id.clone(),
                                ownership_source: "unresolved".to_string(),
                                owned_paths: Vec::new(),
                                dirty_files: Vec::new(),
                                owned_files: Vec::new(),
                                unowned_files: Vec::new(),
                                stageable_files: Vec::new(),
                            };
                            if command.json {
                                let mut value = serde_json::to_value(&receipt)
                                    .expect("owned status receipt should serialize");
                                value["git_error"] = serde_json::json!(error);
                                crate::print_json_pretty(&value);
                            } else {
                                eprintln!("Failed to inspect git status: {error}");
                            }
                            return ExitCode::from(1);
                        }
                    };
                    let receipt = task_owned_status_receipt(
                        &task.id,
                        task.planner_metadata.owned_paths.clone(),
                        command
                            .files
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect(),
                        dirty_files,
                    );
                    if command.json {
                        crate::print_json_pretty(
                            &serde_json::to_value(&receipt)
                                .expect("owned status receipt should serialize"),
                        );
                    } else {
                        print_surface_line(command.render, "owned status", &receipt.status);
                        if !receipt.blocker_codes.is_empty() {
                            print_surface_line(
                                command.render,
                                "blockers",
                                &receipt.blocker_codes.join(", "),
                            );
                        }
                        print_surface_line(
                            command.render,
                            "stageable files",
                            &receipt.stageable_files.len().to_string(),
                        );
                    }
                    if receipt.status == "pass" {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to inspect task owned status: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Handoff(command) => match command.command {
            TaskHandoffCommand::Accept(command) => {
                let explicit_state_dir = command.state_dir.is_some();
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let (receipt_root, isolation) =
                    task_handoff_receipt_root(&state_dir, explicit_state_dir);
                let accepted_at = task_handoff_timestamp();
                let receipt_path = task_handoff_receipt_path(
                    &receipt_root,
                    &command.task_id,
                    &task_handoff_receipt_filename_timestamp(),
                );
                let mut receipt = task_handoff_accept_receipt(
                    &command,
                    &receipt_path,
                    &receipt_root,
                    isolation,
                    accepted_at,
                );
                match task_show_authoritative_first(state_dir, &command.task_id).await {
                    Ok((_task, _metadata)) => {}
                    Err(error) => {
                        receipt = blocked_task_handoff_accept_receipt(
                            &command.task_id,
                            command.agent.as_deref().unwrap_or(""),
                            "missing_task",
                            "Create or import the task before accepting delegated handoff evidence.",
                        );
                        if command.json {
                            crate::print_json_pretty(
                                &serde_json::to_value(&receipt)
                                    .expect("task handoff blocked receipt should serialize"),
                            );
                        } else {
                            eprintln!("Failed to accept task handoff: {error}");
                        }
                        return ExitCode::from(1);
                    }
                }
                if let Err((blocker_code, next_action)) =
                    validate_task_handoff_accept_receipt(&receipt)
                {
                    receipt = blocked_task_handoff_accept_receipt(
                        &command.task_id,
                        command.agent.as_deref().unwrap_or(""),
                        blocker_code,
                        next_action,
                    );
                    if command.json {
                        crate::print_json_pretty(
                            &serde_json::to_value(&receipt)
                                .expect("task handoff blocked receipt should serialize"),
                        );
                    } else {
                        eprintln!("Failed to accept task handoff: {blocker_code}");
                    }
                    return ExitCode::from(1);
                }
                if let Err(error) = persist_task_handoff_accept_receipt(&receipt, &receipt_path) {
                    let blocked = blocked_task_handoff_accept_receipt(
                        &command.task_id,
                        command.agent.as_deref().unwrap_or(""),
                        "task_handoff_receipt_write_failed",
                        "Resolve receipt directory permissions and rerun handoff acceptance.",
                    );
                    if command.json {
                        let mut value = serde_json::to_value(&blocked)
                            .expect("task handoff blocked receipt should serialize");
                        value["write_error"] = serde_json::json!(error);
                        crate::print_json_pretty(&value);
                    } else {
                        eprintln!("Failed to persist task handoff receipt: {error}");
                    }
                    return ExitCode::from(1);
                }
                if command.json {
                    crate::print_json_pretty(
                        &serde_json::to_value(&receipt)
                            .expect("task handoff receipt should serialize"),
                    );
                } else {
                    print_surface_line(command.render, "handoff", &receipt.status);
                    print_surface_line(command.render, "receipt", &receipt.receipt_path);
                }
                if receipt.status == "pass" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
        },
        TaskCommand::Takeover(command) => match command.command {
            TaskTakeoverCommand::Status(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let requested_task_id = match (&command.task_id, &command.task_id_filter) {
                    (Some(positional), Some(flag)) if positional.trim() != flag.trim() => {
                        let receipt = TaskTakeoverStatusReceipt {
                            surface: "vida task takeover status",
                            status: "blocked".to_string(),
                            task_id: positional.clone(),
                            allowed: false,
                            local_exception_takeover_state: "not_recorded".to_string(),
                            root_local_write_allowed: false,
                            paths: Vec::new(),
                            packet: serde_json::json!({}),
                            lane: serde_json::json!({}),
                            root_write_guard: serde_json::json!({
                                "status": "blocked_by_default",
                                "root_local_write_allowed": false,
                                "root_local_write_allowed_for_only_these_paths": [],
                                "local_exception_takeover_state": "not_recorded",
                                "reason": "conflicting task id filters",
                            }),
                            active_takeover_state: "not_recorded".to_string(),
                            takeover_ready_state: "not_ready".to_string(),
                            recommended_surface: None,
                            reason: "positional task id and --task-id disagree".to_string(),
                            recommended_command: None,
                            next_actions: vec![
                                "Rerun with one task id source or matching positional and --task-id values."
                                    .to_string(),
                            ],
                            blocker_codes: vec!["task_filter_conflict".to_string()],
                        };
                        if command.json {
                            crate::print_json_pretty(
                                &serde_json::to_value(&receipt)
                                    .expect("takeover status receipt should serialize"),
                            );
                        } else {
                            print_task_takeover_status(command.render, &receipt);
                        }
                        return ExitCode::from(1);
                    }
                    (Some(positional), _) => Some(positional.trim().to_string()),
                    (_, Some(flag)) => Some(flag.trim().to_string()),
                    (None, None) => None,
                };
                match StateStore::open_existing_read_only(state_dir).await {
                    Ok(store) => {
                        let (status_override, lane_source) =
                            if let Some(run_id) = command.run_id.as_deref() {
                                match store.run_graph_status(run_id).await {
                                    Ok(status) => (Some(status), Some("run_id")),
                                    Err(error) => {
                                        eprintln!("Failed to inspect run graph status: {error}");
                                        return ExitCode::from(1);
                                    }
                                }
                            } else {
                                let current = store
                                    .latest_run_graph_status_for_current_session()
                                    .await
                                    .ok()
                                    .flatten();
                                match current {
                                    Some(status) => (Some(status), Some("current_session")),
                                    None => (
                                        store.latest_run_graph_status().await.ok().flatten(),
                                        Some("latest"),
                                    ),
                                }
                            };
                        let Some(task_id) = requested_task_id
                            .filter(|value| !value.trim().is_empty())
                            .or_else(|| {
                                status_override
                                    .as_ref()
                                    .map(|status| status.task_id.clone())
                            })
                        else {
                            let receipt = TaskTakeoverStatusReceipt {
                                surface: "vida task takeover status",
                                status: "blocked".to_string(),
                                task_id: String::new(),
                                allowed: false,
                                local_exception_takeover_state: "not_recorded".to_string(),
                                root_local_write_allowed: false,
                                paths: Vec::new(),
                                packet: serde_json::json!({}),
                                lane: serde_json::json!({}),
                                root_write_guard: serde_json::json!({
                                    "status": "blocked_by_default",
                                    "root_local_write_allowed": false,
                                    "root_local_write_allowed_for_only_these_paths": [],
                                    "local_exception_takeover_state": "not_recorded",
                                    "reason": "missing task and lane evidence",
                                }),
                                active_takeover_state: "not_recorded".to_string(),
                                takeover_ready_state: "not_ready".to_string(),
                                recommended_surface: Some("vida lane show".to_string()),
                                reason: "no task id was supplied and no latest lane task id is available"
                                    .to_string(),
                                recommended_command: Some("vida lane show --latest --json".to_string()),
                                next_actions: vec![
                                    "Supply --task-id or inspect lane evidence with `vida lane show --latest --json`."
                                        .to_string(),
                                ],
                                blocker_codes: vec!["missing_task_and_lane_evidence".to_string()],
                            };
                            if command.json {
                                crate::print_json_pretty(
                                    &serde_json::to_value(&receipt)
                                        .expect("takeover status receipt should serialize"),
                                );
                            } else {
                                print_task_takeover_status(command.render, &receipt);
                            }
                            return ExitCode::from(1);
                        };
                        match store.show_task(&task_id).await {
                            Ok(task) => {
                                let receipt = task_takeover_status_receipt(
                                    &store,
                                    &task,
                                    status_override,
                                    lane_source,
                                )
                                .await;
                                if command.json {
                                    crate::print_json_pretty(
                                        &serde_json::to_value(&receipt)
                                            .expect("takeover status receipt should serialize"),
                                    );
                                } else {
                                    print_task_takeover_status(command.render, &receipt);
                                }
                                if receipt.allowed {
                                    ExitCode::SUCCESS
                                } else {
                                    ExitCode::from(1)
                                }
                            }
                            Err(error) => {
                                eprintln!("Failed to inspect task takeover status: {error}");
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::Progress(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            if command.epics {
                let basis = match task_progress_basis_arg(&command.basis) {
                    Ok(basis) => basis,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                    Ok((rows, metadata)) => {
                        match task_epic_progress_summary(
                            &rows,
                            metadata,
                            command.all,
                            command.epic.as_deref(),
                            basis,
                        ) {
                            Ok(summary) => {
                                print_task_epic_progress_summary(
                                    command.render,
                                    &summary,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to compute epic progress summary: {error}");
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to read task progress rows: {error}");
                        ExitCode::from(1)
                    }
                }
            } else {
                let Some(task_id) = command.task_id.as_deref() else {
                    eprintln!("Task id is required unless --epics is set");
                    return ExitCode::from(1);
                };
                let basis = match task_progress_basis_arg(&command.basis) {
                    Ok(basis) => basis,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                if basis == "direct_children" {
                    match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                        Ok((rows, _metadata)) => {
                            match task_progress_summary_for_basis(&rows, task_id, basis) {
                                Ok(summary) => {
                                    print_task_progress(command.render, &summary, command.json);
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!("Failed to compute task progress: {error}");
                                    ExitCode::from(1)
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to read task progress rows: {error}");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    match StateStore::open_existing_read_only(state_dir.clone()).await {
                        Ok(store) => match store.task_progress_summary(task_id).await {
                            Ok(summary) => {
                                print_task_progress(command.render, &summary, command.json);
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to compute task progress: {error}");
                                ExitCode::from(1)
                            }
                        },
                        Err(error) if is_authoritative_state_lock_error(&error) => {
                            let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                                Ok(rows) => rows,
                                Err(snapshot_error) => {
                                    eprintln!(
                                        "Failed to read task progress from snapshot: {snapshot_error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                            match StateStore::task_progress_summary_from_rows(&rows, task_id) {
                                Ok(summary) => {
                                    print_task_progress(command.render, &summary, command.json);
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to compute task progress from snapshot: {error}"
                                    );
                                    ExitCode::from(1)
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to open authoritative state store: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
            }
        }
        TaskCommand::ClosureReady(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let basis = match task_progress_basis_arg(&command.basis) {
                Ok(basis) => basis,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                Ok((rows, metadata)) => {
                    match task_progress_summary_for_basis(&rows, &command.task_id, basis) {
                        Ok(summary) => {
                            let payload =
                                crate::task_cli_render::build_pass_operator_surface_payload(
                                    "vida task closure-ready",
                                    serde_json::json!({
                                       "task_id": command.task_id,
                                       "state_access": task_read_metadata_value(Some(&metadata)),
                                       "basis": basis,
                                       "ready_for_close": summary.ready_for_close,
                                       "closure_candidate": summary.closure_candidate,
                                       "closure_candidate_state": summary.closure_candidate_state,
                                        "closure_candidate_reason": summary.closure_candidate_reason,
                                        "next_required_command": summary.next_required_command,
                                        "recommended_next_action": summary.recommended_next_action,
                                        "progress": crate::task_cli_render::task_progress_value(&summary),
                                    }),
                                );
                            if command.json {
                                crate::print_json_pretty(&payload);
                            } else if matches!(command.render, crate::RenderMode::Plain) {
                                println!(
                                    "{}",
                                    crate::task_cli_render::task_progress_toon_text(
                                        "vida task closure-ready",
                                        &summary,
                                    )
                                );
                            } else {
                                print_surface_header(command.render, "vida task closure-ready");
                                print_surface_line(command.render, "task", &command.task_id);
                                print_surface_line(
                                    command.render,
                                    "ready",
                                    if payload["ready_for_close"].as_bool().unwrap_or(false) {
                                        "true"
                                    } else {
                                        "false"
                                    },
                                );
                                print_surface_line(
                                    command.render,
                                    "state",
                                    payload["closure_candidate_state"]
                                        .as_str()
                                        .unwrap_or("unknown"),
                                );
                                if let Some(command_text) =
                                    payload["next_required_command"].as_str()
                                {
                                    print_surface_line(
                                        command.render,
                                        "next command",
                                        command_text,
                                    );
                                }
                            }
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to compute closure readiness: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to read task progress rows: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Proof(command) => match command.command {
            TaskProofCommand::Status(command) => {
                let state_dir = command
                    .state_dir
                    .unwrap_or_else(state_store::default_state_dir);
                match task_show_authoritative_first(state_dir, &command.task_id).await {
                    Ok((task, metadata)) => {
                        let payload = task_proof_status_payload(&task, Some(&metadata));
                        if command.json {
                            crate::print_json_pretty(&payload);
                        } else {
                            print_task_proof_status(command.render, &task, &payload);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to inspect task proof status: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskProofCommand::AttachBrowser(command) => {
                let route = command.route.trim();
                if route.is_empty() {
                    eprintln!("--route cannot be empty");
                    return ExitCode::from(2);
                }
                let result = match normalize_browser_proof_result(&command.result) {
                    Ok(result) => result,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => {
                        let existing = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read task before browser proof attachment: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let proof_target = browser_proof_target(route, command.expect.as_deref());
                        let evidence = normalized_task_verify_evidence(&command.evidence);
                        let notes = append_task_browser_proof_note(
                            existing.notes.as_deref(),
                            &proof_target,
                            route,
                            &result,
                            command.expect.as_deref(),
                            command.screenshot.as_deref(),
                            &evidence,
                        );
                        let planner_metadata = task_browser_proof_planner_metadata(
                            &existing.planner_metadata,
                            &proof_target,
                        );
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(&notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: Some(planner_metadata),
                            })
                            .await
                        {
                            Ok(task) => {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task proof attach-browser",
                                )
                                .await
                                {
                                    return code;
                                }
                                let receipt = TaskProofAttachBrowserReceipt {
                                    surface: "vida task proof attach-browser",
                                    status: task_json_success_status(),
                                    task_id: task.id.clone(),
                                    route: route.to_string(),
                                    result,
                                    expect: command.expect,
                                    screenshot: command.screenshot,
                                    evidence,
                                    proof_target,
                                    notes_appended: true,
                                    task,
                                };
                                print_task_browser_proof_receipt(
                                    command.render,
                                    &receipt,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to attach browser proof to task: {error}");
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::Ready(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            if command.json {
                let projection_name = task_ready_projection_name(command.scope.as_deref());
                if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
                    &state_dir,
                    &projection_name,
                ) {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
                if let Some(cached) =
                    crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                        &state_dir,
                        &projection_name,
                        TASK_READ_RECENT_PROJECTION_MAX_AGE,
                    )
                {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
            }
            match task_ready_authoritative_first(state_dir.clone(), command.scope.as_deref()).await
            {
                Ok((tasks, metadata)) => {
                    if command.json {
                        let payload =
                            task_ready_payload(command.scope.as_deref(), &tasks, Some(&metadata));
                        crate::print_json_pretty(&payload);
                        crate::operator_projection_cache::write_json_projection(
                            &state_dir,
                            &task_ready_projection_name(command.scope.as_deref()),
                            &payload,
                        );
                    } else {
                        print_task_ready(
                            command.render,
                            command.scope.as_deref(),
                            &tasks,
                            false,
                            Some(&metadata),
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to compute ready tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Next(command) => {
            let mut proxy_args = vec!["next".to_string()];
            if let Some(scope) = command.scope.as_deref() {
                proxy_args.push("--scope".to_string());
                proxy_args.push(scope.to_string());
            }
            if let Some(state_dir) = command.state_dir.as_ref().and_then(|path| path.to_str()) {
                proxy_args.push("--state-dir".to_string());
                proxy_args.push(state_dir.to_string());
            }
            if command.json {
                proxy_args.push("--json".to_string());
            }
            crate::taskflow_proxy::run_taskflow_next_surface(&proxy_args).await
        }
        TaskCommand::NextLawful(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => {
                    let tasks = match store.list_tasks(None, true).await {
                        Ok(tasks) => tasks,
                        Err(error) => {
                            eprintln!("Failed to list tasks for lawful continuation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let explicit_binding =
                        match store.latest_explicit_run_graph_continuation_binding().await {
                            Ok(binding) => binding,
                            Err(error) => {
                                eprintln!("Failed to read explicit continuation binding: {error}");
                                return ExitCode::from(1);
                            }
                        };
                    let latest_run_graph_status = match store.latest_run_graph_status().await {
                        Ok(status) => status,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph status: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let current_binding = match latest_run_graph_status.as_ref() {
                        Some(status) => match store
                            .run_graph_status_is_stale_after_release_admission_complete(status)
                            .await
                        {
                            Ok(true) => None,
                            Ok(false) => {
                                match store.run_graph_continuation_binding(&status.run_id).await {
                                    Ok(binding) => binding,
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to read current latest-run continuation binding: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to classify release-admitted stale run-graph status: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        },
                        None => None,
                    };
                    let runtime_binding = match select_task_next_lawful_binding(
                        &tasks,
                        explicit_binding.as_ref(),
                        current_binding.as_ref(),
                    ) {
                        Ok(binding) => binding,
                        Err(receipt) => {
                            if command.json {
                                let receipt_json = serde_json::to_value(&receipt).expect(
                                    "task next-lawful source drift receipt should serialize",
                                );
                                if command.scope.is_none() {
                                    crate::operator_projection_cache::write_json_projection(
                                        &state_dir,
                                        task_next_lawful_projection_name(),
                                        &receipt_json,
                                    );
                                }
                                crate::print_json_pretty(&receipt_json);
                            } else {
                                print_surface_line(command.render, "next lawful", &receipt.status);
                                print_surface_line(
                                    command.render,
                                    "blockers",
                                    &receipt.blocker_codes.join(", "),
                                );
                            }
                            return ExitCode::from(1);
                        }
                    };
                    let runtime_binding_task_missing_in_explicit_scope = command.scope.is_some()
                        && runtime_binding
                            .map(|binding| !task_exists_for_binding(&tasks, binding))
                            .unwrap_or(false);
                    let runtime_binding_is_closed_downstream_marker = runtime_binding
                        .map(|binding| {
                            continuation_binding_is_closed_downstream_marker(&tasks, binding)
                        })
                        .unwrap_or(false);
                    let scoped_runtime_binding = if runtime_binding_task_missing_in_explicit_scope
                        || runtime_binding_is_closed_downstream_marker
                    {
                        None
                    } else {
                        runtime_binding
                    };
                    let projection = match store
                        .scheduling_projection_scoped(
                            command.scope.as_deref(),
                            scoped_runtime_binding.map(|binding| binding.task_id.as_str()),
                        )
                        .await
                    {
                        Ok(projection) => projection,
                        Err(error) => {
                            eprintln!("Failed to compute lawful continuation candidates: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let ready_task_candidates = projection
                        .ready
                        .iter()
                        .map(|candidate| {
                            task_continuation_candidate(
                                &candidate.task,
                                candidate.ready_parallel_safe,
                            )
                        })
                        .collect::<Vec<_>>();
                    let ready_task_candidates = task_next_lawful_apply_strategy(
                        &tasks,
                        ready_task_candidates,
                        command.strategy.as_deref(),
                    );
                    let runtime_recovery = match scoped_runtime_binding {
                        Some(binding) => {
                            store.run_graph_recovery_summary(&binding.run_id).await.ok()
                        }
                        None => None,
                    };
                    let latest_dispatch_receipt = match store
                        .latest_run_graph_dispatch_receipt_summary()
                        .await
                    {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!("Failed to read latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let terminal_consume_continue_run_id =
                        crate::latest_terminal_consume_continue_snapshot_run_id(&state_dir)
                            .ok()
                            .flatten();
                    let mut receipt = if let Some(selected_task_id) = command.select.as_deref() {
                        if scoped_runtime_binding.is_some() {
                            blocked_task_next_lawful_receipt(
                                serde_json::Value::Null,
                                ready_task_candidates,
                                "select_conflicts_with_active_runtime_binding",
                                "Cannot apply --select while an active runtime binding is present; resolve or complete the current binding first.",
                            )
                        } else {
                            task_next_lawful_select_ready_candidate_receipt(
                                &tasks,
                                ready_task_candidates,
                                selected_task_id,
                            )
                        }
                    } else {
                        match scoped_runtime_binding {
                            Some(binding)
                                if latest_dispatch_receipt.as_ref().is_some_and(|dispatch| {
                                    dispatch.run_id == binding.run_id
                                        && runtime_dispatch_receipt_has_ready_downstream_handoff(
                                            Some(binding.run_id.as_str()),
                                            Some(dispatch),
                                        )
                                }) =>
                            {
                                pass_ready_downstream_handoff_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                    terminal_consume_continue_run_id.as_deref(),
                                    latest_dispatch_receipt
                                        .as_ref()
                                        .and_then(downstream_dispatch_command_for_task_next_lawful)
                                        .as_deref(),
                                )
                            }
                            Some(binding)
                                if runtime_recovery_blocks_task_next_lawful(
                                    runtime_recovery.as_ref(),
                                    latest_dispatch_receipt.as_ref(),
                                ) && runtime_binding_has_active_exception_takeover(
                                    binding,
                                    latest_dispatch_receipt.as_ref(),
                                ) =>
                            {
                                pass_exception_takeover_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                )
                            }
                            Some(binding)
                                if runtime_dispatch_receipt_has_completed_lane(
                                    Some(binding.run_id.as_str()),
                                    latest_dispatch_receipt.as_ref(),
                                ) =>
                            {
                                pass_completed_lane_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                )
                            }
                            Some(binding)
                                if runtime_recovery_blocks_task_next_lawful(
                                    runtime_recovery.as_ref(),
                                    latest_dispatch_receipt.as_ref(),
                                ) =>
                            {
                                blocked_task_next_lawful_receipt(
                                    binding.active_bounded_unit.clone(),
                                    ready_task_candidates,
                                    "open_delegated_cycle",
                                    &runtime_binding_open_delegated_cycle_next_action(binding),
                                )
                            }
                            _ => task_next_lawful_receipt(
                                &tasks,
                                ready_task_candidates,
                                scoped_runtime_binding,
                            ),
                        }
                    };
                    receipt = task_next_lawful_attach_explanation(
                        receipt,
                        command.explain,
                        command.strategy.as_deref(),
                        command.select.as_deref(),
                    );
                    if command.json {
                        let receipt_json = serde_json::to_value(&receipt)
                            .expect("task next-lawful receipt should serialize");
                        if command.scope.is_none() {
                            crate::operator_projection_cache::write_json_projection(
                                &state_dir,
                                task_next_lawful_projection_name(),
                                &receipt_json,
                            );
                        }
                        crate::print_json_pretty(&receipt_json);
                    } else {
                        print_surface_line(command.render, "next lawful", &receipt.status);
                        print_surface_line(
                            command.render,
                            "posture",
                            &receipt.sequential_vs_parallel_posture,
                        );
                        if !receipt.blocker_codes.is_empty() {
                            print_surface_line(
                                command.render,
                                "blockers",
                                &receipt.blocker_codes.join(", "),
                            );
                        }
                        if let Some(task_id) = receipt
                            .active_bounded_unit
                            .get("task_id")
                            .and_then(serde_json::Value::as_str)
                        {
                            print_surface_line(command.render, "active bounded unit", task_id);
                        }
                    }
                    if receipt.status == task_json_success_status() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::NextDisplayId(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_read_only_task_store(state_dir).await {
                Ok(store) => match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            let payload = crate::taskflow_task_bridge::next_display_id_payload(
                                &rows,
                                &command.parent_display_id,
                            );
                            let valid = payload
                                .get("valid")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false);
                            print_task_next_display_id(command.render, &payload, command.json);
                            if valid {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to compute next display id: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to list tasks for next display id: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Create(command) => run_task_create_like(command, false).await,
        TaskCommand::Ensure(command) => run_task_create_like(command, true).await,
        TaskCommand::Update(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let notes = match resolve_optional_text_arg(
                "notes",
                command.notes.as_deref(),
                command.notes_file.as_deref(),
            ) {
                Ok(notes) => notes,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let add_labels = parse_label_values(&command.add_labels);
            let remove_labels = parse_label_values(&command.remove_labels);
            let set_labels = parse_optional_label_value(command.set_labels.as_deref());
            let execution_mode = match task_update_semantics_arg(
                command.execution_mode.as_deref(),
                command.clear_execution_mode,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let order_bucket = match task_update_semantics_arg(
                command.order_bucket.as_deref(),
                command.clear_order_bucket,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parallel_group = match task_update_semantics_arg(
                command.parallel_group.as_deref(),
                command.clear_parallel_group,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let conflict_domain = match task_update_semantics_arg(
                command.conflict_domain.as_deref(),
                command.clear_conflict_domain,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parent_id =
                match task_update_parent_arg(command.parent_id.as_deref(), command.clear_parent_id)
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let planner_metadata = if task_update_planner_metadata_requested(&command) {
                        match store.show_task(&command.task_id).await {
                            Ok(existing) => task_update_planner_metadata_arg(
                                &existing.planner_metadata,
                                &command,
                            ),
                            Err(error) => {
                                eprintln!(
                                    "Failed to read task before planner metadata update: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        None
                    };
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: command.title.as_deref(),
                            status: command.status.as_deref(),
                            priority: command.priority,
                            notes: notes.as_deref(),
                            description: command.description.as_deref(),
                            parent_id,
                            add_labels: &add_labels,
                            remove_labels: &remove_labels,
                            set_labels: set_labels.as_deref(),
                            execution_mode,
                            order_bucket,
                            parallel_group,
                            conflict_domain,
                            planner_metadata,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task update")
                                    .await
                            {
                                return code;
                            }
                            print_task_mutation(
                                command.render,
                                "vida task update",
                                &task,
                                command.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            if command.json {
                                if let state_store::StateStoreError::InvalidTaskRecord { reason } =
                                    &error
                                {
                                    if let Some(issue) =
                                        task_update_graph_issue_from_invalid_record_reason(reason)
                                    {
                                        print_task_update_graph_blocked(&issue, command.json);
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            eprintln!("Failed to update task: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Note(command) => match command.command {
            TaskNoteCommand::Append(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let message = match resolve_optional_text_arg(
                    "message",
                    command.message.as_deref(),
                    command.message_file.as_deref(),
                ) {
                    Ok(Some(message)) if !message.trim().is_empty() => message.trim().to_string(),
                    Ok(_) => {
                        eprintln!("A non-empty --message or --message-file value is required");
                        return ExitCode::from(2);
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => {
                        let existing = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!("Failed to read task before note append: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        let appended_notes = match existing.notes.as_deref() {
                            Some(notes) if !notes.trim().is_empty() => {
                                format!("{}{}{}", notes, command.separator, message)
                            }
                            _ => message,
                        };
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(&appended_notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: None,
                            })
                            .await
                        {
                            Ok(task) => {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task note append",
                                )
                                .await
                                {
                                    return code;
                                }
                                print_task_mutation(
                                    command.render,
                                    "vida task note append",
                                    &task,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to append task note: {error}");
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::Block(command) => {
            let reason = command.reason.trim();
            if reason.is_empty() {
                eprintln!("--reason cannot be empty");
                return ExitCode::from(2);
            }
            let evidence = command
                .evidence
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let blocker_codes = normalize_task_block_list(&command.blockers);
            let next_actions = command
                .next_actions
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let existing = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to read task before block mutation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let previous_status = existing.status.clone();
                    if state_store::StateStore::task_status_is_closed_like(&existing.status) {
                        let receipt = TaskBlockReceipt {
                            surface: "vida task block",
                            status: "blocked",
                            blocker_codes: vec!["task_already_closed".to_string()],
                            next_actions: vec![
                                "Inspect the closed task or reopen it before recording a runtime blocker."
                                    .to_string(),
                            ],
                            task_id: existing.id.clone(),
                            blocked: false,
                            closed: true,
                            previous_status,
                            reason: reason.to_string(),
                            evidence: evidence.map(str::to_string),
                            notes_appended: false,
                            task: existing,
                        };
                        print_task_block_receipt(command.render, &receipt, command.json);
                        return ExitCode::from(1);
                    }

                    let notes = append_task_block_note(
                        existing.notes.as_deref(),
                        reason,
                        evidence,
                        &blocker_codes,
                        &next_actions,
                    );
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: None,
                            status: Some("blocked"),
                            priority: None,
                            notes: Some(&notes),
                            description: None,
                            parent_id: None,
                            add_labels: &[],
                            remove_labels: &[],
                            set_labels: None,
                            execution_mode: None,
                            order_bucket: None,
                            parallel_group: None,
                            conflict_domain: None,
                            planner_metadata: None,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task block")
                                    .await
                            {
                                return code;
                            }
                            let receipt = TaskBlockReceipt {
                                surface: "vida task block",
                                status: task_json_success_status(),
                                blocker_codes,
                                next_actions,
                                task_id: task.id.clone(),
                                blocked: task.status == "blocked",
                                closed: state_store::StateStore::task_status_is_closed_like(
                                    &task.status,
                                ),
                                previous_status,
                                reason: reason.to_string(),
                                evidence: evidence.map(str::to_string),
                                notes_appended: true,
                                task,
                            };
                            print_task_block_receipt(command.render, &receipt, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to block task: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Verify(command) => {
            let evidence = normalized_task_verify_evidence(&command.evidence);
            let proof_blocker = command
                .proof_blocker
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if command.proof_blocked && proof_blocker.is_none() && evidence.is_empty() {
                eprintln!("--proof-blocked requires --proof-blocker or --evidence");
                return ExitCode::from(2);
            }
            if !command.source_fixed && !command.tests_green && !command.proof_blocked {
                eprintln!(
                    "task verify requires at least one of --source-fixed, --tests-green, or --proof-blocked"
                );
                return ExitCode::from(2);
            }
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let existing = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to read task before verify mutation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    if state_store::StateStore::task_status_is_closed_like(&existing.status) {
                        let receipt = TaskVerifyReceipt {
                            surface: "vida task verify",
                            status: "blocked",
                            task_id: existing.id.clone(),
                            partial: false,
                            closed: true,
                            source_fixed: command.source_fixed,
                            tests_green: command.tests_green,
                            proof_blocked: command.proof_blocked,
                            proof_blocked_by_runtime: false,
                            proof_blocker: proof_blocker.map(str::to_string),
                            evidence,
                            blocker_codes: vec!["task_already_closed".to_string()],
                            next_actions: vec![
                                "Inspect the closed task or reopen it before recording partial verification."
                                    .to_string(),
                            ],
                            task: existing,
                        };
                        print_task_verify_receipt(command.render, &receipt, command.json);
                        return ExitCode::from(1);
                    }
                    let notes = append_task_verify_note(
                        existing.notes.as_deref(),
                        command.source_fixed,
                        command.tests_green,
                        command.proof_blocked,
                        proof_blocker,
                        &evidence,
                    );
                    let add_labels = task_verify_labels(
                        command.source_fixed,
                        command.tests_green,
                        command.proof_blocked,
                    );
                    let planner_metadata = task_verify_planner_metadata(
                        &existing.planner_metadata,
                        command.proof_blocked,
                        proof_blocker,
                        &evidence,
                    );
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: None,
                            status: None,
                            priority: None,
                            notes: Some(&notes),
                            description: None,
                            parent_id: None,
                            add_labels: &add_labels,
                            remove_labels: &[],
                            set_labels: None,
                            execution_mode: None,
                            order_bucket: None,
                            parallel_group: None,
                            conflict_domain: None,
                            planner_metadata,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task verify")
                                    .await
                            {
                                return code;
                            }
                            let proof_blocked_by_runtime =
                                command.proof_blocked && task_reports_runtime_proof_blocker(&task);
                            let receipt = TaskVerifyReceipt {
                                surface: "vida task verify",
                                status: task_json_success_status(),
                                task_id: task.id.clone(),
                                partial: true,
                                closed: state_store::StateStore::task_status_is_closed_like(
                                    &task.status,
                                ),
                                source_fixed: command.source_fixed,
                                tests_green: command.tests_green,
                                proof_blocked: command.proof_blocked,
                                proof_blocked_by_runtime,
                                proof_blocker: proof_blocker.map(str::to_string),
                                evidence,
                                blocker_codes: Vec::new(),
                                next_actions: if command.proof_blocked {
                                    vec![
                                        "Resolve or attach final proof evidence before closing this task."
                                            .to_string(),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                task,
                            };
                            print_task_verify_receipt(command.render, &receipt, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to verify task: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Split(command) => run_task_split_like(command, "vida task split").await,
        TaskCommand::SpawnBlocker(command) => {
            run_task_spawn_blocker_like(command, "vida task spawn-blocker").await
        }
        TaskCommand::AdaptivePreview(command) => run_task_adaptive_preview(command).await,
        TaskCommand::Close(command) => {
            let explicit_state_dir = command.state_dir.is_some();
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let project_root = project_root_for_task_state(&state_dir);
            let feedback_source = command.source.as_deref().unwrap_or("vida task close");
            match StateStore::open_existing(state_dir.clone()).await {
                Ok(store) => {
                    if crate::agent_feedback_surface::canonical_close_status_from_reason(
                        &command.reason,
                    )
                    .is_some()
                    {
                        let preclose_task = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!("Failed to close task: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        let task_value = serde_json::to_value(&preclose_task)
                            .expect("task close payload should serialize");
                        let telemetry = task_close_host_agent_telemetry(
                            &state_dir,
                            explicit_state_dir,
                            project_root.as_deref(),
                            &task_value,
                            &command.reason,
                            feedback_source,
                        );
                        if let Some((blocker_codes, next_actions)) =
                            task_close_feedback_blocker_summary(&telemetry)
                        {
                            if command.json {
                                crate::print_json_pretty(&serde_json::json!({
                                    "status": "blocked",
                                    "blocker_codes": blocker_codes,
                                    "next_actions": next_actions,
                                    "task": preclose_task,
                                    "host_agent_telemetry": telemetry,
                                    "automation": null,
                                }));
                            } else {
                                print_task_mutation(
                                    command.render,
                                    "vida task close",
                                    &preclose_task,
                                    false,
                                );
                                print_surface_line(
                                    command.render,
                                    "telemetry blockers",
                                    &blocker_codes.join(", "),
                                );
                            }
                            return ExitCode::from(1);
                        }
                    }
                    match store.close_task(&command.task_id, &command.reason).await {
                        Ok(_task) => {
                            if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_specification_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                            if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_implementer_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                            let task = match store.show_task(&command.task_id).await {
                                Ok(task) if task.status == "closed" => task,
                                Ok(task) => {
                                    eprintln!(
                                        "Task close drifted after post-close reconciliation: `{}` is `{}` instead of `closed`.",
                                        command.task_id, task.status
                                    );
                                    return ExitCode::from(1);
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to re-read closed task after post-close reconciliation: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task close")
                                    .await
                            {
                                return code;
                            }
                            let task_value = serde_json::to_value(&task)
                                .expect("task close payload should serialize");
                            let telemetry = task_close_host_agent_telemetry(
                                &state_dir,
                                explicit_state_dir,
                                project_root.as_deref(),
                                &task_value,
                                &command.reason,
                                feedback_source,
                            );
                            let automation = if task_close_automation_requested(&command) {
                                Some(task_close_automation_receipt(
                                    &command,
                                    project_root.as_deref(),
                                    Some(&task),
                                ))
                            } else {
                                None
                            };
                            let telemetry_feedback_blocker =
                                task_close_feedback_blocker_summary(&telemetry);
                            let epic_progress_summary = match store.all_tasks().await {
                                Ok(rows) => {
                                    match task_close_epic_progress_summary(
                                        &rows,
                                        &command.task_id,
                                        command.include_global_progress,
                                    ) {
                                        Ok(summary) => Some(summary),
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to compute task close epic progress summary: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read tasks for task close epic progress summary: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                            if command.json {
                                let payload = task_close_result_payload(
                                    &task,
                                    &telemetry,
                                    automation.as_ref(),
                                    telemetry_feedback_blocker.as_ref(),
                                    epic_progress_summary.as_ref(),
                                );
                                crate::print_json_pretty(&payload);
                            } else {
                                print_task_mutation(
                                    command.render,
                                    "vida task close",
                                    &task,
                                    false,
                                );
                                let telemetry_status = telemetry
                                    .get("status")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("unknown");
                                let telemetry_reason = telemetry
                                    .get("reason")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("");
                                let telemetry_summary = if telemetry_reason.is_empty() {
                                    telemetry_status.to_string()
                                } else {
                                    format!("{telemetry_status}: {telemetry_reason}")
                                };
                                print_surface_line(
                                    command.render,
                                    "host agent telemetry",
                                    &telemetry_summary,
                                );
                                if let Some((blocker_codes, _)) = &telemetry_feedback_blocker {
                                    print_surface_line(
                                        command.render,
                                        "telemetry blockers",
                                        &blocker_codes.join(", "),
                                    );
                                }
                                if let Some(automation) = &automation {
                                    print_surface_line(
                                        command.render,
                                        "automation",
                                        &automation.status,
                                    );
                                    if !automation.blocker_codes.is_empty() {
                                        print_surface_line(
                                            command.render,
                                            "automation blockers",
                                            &automation.blocker_codes.join(", "),
                                        );
                                    }
                                }
                                if let Some(summary) = &epic_progress_summary {
                                    print_task_close_epic_progress_summary(command.render, summary);
                                }
                            }
                            if task_close_automation_is_blocked(automation.as_ref()) {
                                ExitCode::from(1)
                            } else {
                                ExitCode::SUCCESS
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to close task: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReconcileClosedRuns(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .reconcile_historical_closed_task_active_runs(command.limit)
                    .await
                {
                    Ok(summary) => {
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": "pass",
                                "surface": "vida task reconcile-closed-runs",
                                "summary": summary,
                                "blocker_codes": [],
                                "next_actions": [],
                            }));
                        } else {
                            print_surface_line(
                                command.render,
                                "reconciled closed-task runs",
                                &summary.reconciled_count.to_string(),
                            );
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to reconcile closed-task runs: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Deps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.task_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read task dependencies from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::task_dependencies_from_rows(&rows, &command.task_id) {
                        Ok(dependencies) => {
                            print_task_dependencies(
                                command.render,
                                "vida task deps",
                                &command.task_id,
                                &dependencies,
                                command.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to read task dependencies from snapshot: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReverseDeps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.reverse_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task reverse-deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read reverse dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read reverse dependencies from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::reverse_dependencies_from_rows(&rows, &command.task_id) {
                        Ok(dependencies) => {
                            print_task_dependencies(
                                command.render,
                                "vida task reverse-deps",
                                &command.task_id,
                                &dependencies,
                                command.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to read reverse dependencies from snapshot: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Blocked(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.blocked_tasks().await {
                    Ok(tasks) => {
                        print_blocked_tasks(command.render, &tasks, command.summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute blocked tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read blocked tasks from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    let tasks = StateStore::blocked_tasks_from_rows(&rows);
                    print_blocked_tasks(command.render, &tasks, command.summary, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Children(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_direct_children(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    if command.json {
                        crate::print_json_pretty(&serde_json::json!({
                            "status": "blocked",
                            "surface": "vida task children",
                            "blocker_codes": ["task_tree_traversal_failed"],
                            "task_id": command.task_id,
                            "reason": error.to_string(),
                            "next_action": "Run `vida task validate-graph --json` to inspect graph cycles or reduce traversal scope with `vida task children <task-id> --json`.",
                            "next_actions": [
                                "Run `vida task validate-graph --json` to inspect graph cycles or reduce traversal scope with `vida task children <task-id> --json`."
                            ],
                        }));
                    } else {
                        eprintln!("Failed to read task direct children: {error}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_dependency_tree(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    if command.json {
                        crate::print_json_pretty(&serde_json::json!({
                            "status": "blocked",
                            "surface": "vida task tree",
                            "blocker_codes": ["task_tree_traversal_failed"],
                            "task_id": command.task_id,
                            "reason": error.to_string(),
                            "next_action": "Run `vida task validate-graph --json` to inspect graph cycles or reduce traversal scope with `vida task children <task-id> --json`.",
                            "next_actions": [
                                "Run `vida task validate-graph --json` to inspect graph cycles or reduce traversal scope with `vida task children <task-id> --json`."
                            ],
                        }));
                    } else {
                        eprintln!("Failed to read task dependency tree: {error}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReparentChildren(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .reparent_children(
                        &command.from_parent_id,
                        &command.to_parent_id,
                        &command.child_ids,
                        command.dry_run,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Err(code) = refresh_task_snapshot_after_mutation(
                            &store,
                            "vida task reparent-children",
                        )
                        .await
                        {
                            return code;
                        }
                        print_task_bulk_reparent_result(command.render, &result, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to bulk-reparent children: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::DefectBatchRehome(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .defect_batch_rehome(
                        &command.from_parent_id,
                        &command.to_parent_id,
                        &command.child_ids,
                        &command.pause_task_ids,
                        &command.start_task_ids,
                        command.dry_run,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Err(code) = refresh_task_snapshot_after_mutation(
                            &store,
                            "vida task defect-batch-rehome",
                        )
                        .await
                        {
                            return code;
                        }
                        print_task_defect_batch_rehome_result(
                            command.render,
                            &result,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to defect-batch rehome tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ValidateGraph(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.validate_task_graph().await {
                    Ok(issues) => {
                        print_task_graph_issues(command.render, &issues, command.json);
                        if issues.is_empty() {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to validate task graph: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!("Failed to read task graph snapshot: {snapshot_error}");
                            return ExitCode::from(1);
                        }
                    };
                    let issues = StateStore::validate_task_graph_rows(&rows);
                    print_task_graph_issues(command.render, &issues, command.json);
                    if issues.is_empty() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Dep(command) => match command.command {
            TaskDependencyCommand::Add(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependency(
                            &add.task_id,
                            &add.depends_on_id,
                            &add.edge_type,
                            &add.created_by,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep add")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                add.render,
                                "vida task dep add",
                                &dependency,
                                add.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::AddBulk(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let edges =
                    match task_dependency_bulk_edge_inputs(&add.edges, add.edge_file.as_deref()) {
                        Ok(edges) => edges,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependencies_bulk(&edges, &add.created_by, add.dry_run)
                        .await
                    {
                        Ok(result) => {
                            if result.failed_count == 0 && !result.dry_run {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task dep add-bulk",
                                )
                                .await
                                {
                                    return code;
                                }
                            }
                            let exit_code = if result.failed_count == 0 {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            };
                            print_task_dependency_bulk_add_result(add.render, &result, add.json);
                            exit_code
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependencies in bulk: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Ensure(ensure) => {
                let state_dir = ensure
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let edges = vec![state_store::TaskDependencyBulkAddInput {
                    issue_id: ensure.task_id.clone(),
                    depends_on_id: ensure.depends_on_id.clone(),
                    edge_type: ensure.edge_type.clone(),
                }];
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependencies_bulk(&edges, &ensure.created_by, false)
                        .await
                    {
                        Ok(result) => {
                            if result.failed_count == 0 && result.created_count > 0 {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task dep ensure",
                                )
                                .await
                                {
                                    return code;
                                }
                            }
                            let exit_code = if result.failed_count == 0 {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            };
                            print_task_dependency_bulk_add_result(
                                ensure.render,
                                &result,
                                ensure.json,
                            );
                            exit_code
                        }
                        Err(error) => {
                            eprintln!("Failed to ensure task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Remove(remove) => {
                let state_dir = remove
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .remove_task_dependency(
                            &remove.task_id,
                            &remove.depends_on_id,
                            &remove.edge_type,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep remove")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                remove.render,
                                "vida task dep remove",
                                &dependency,
                                remove.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to remove task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::CriticalPath(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_critical_path_snapshot_first(state_dir).await {
                Ok(path) => {
                    print_task_critical_path(command.render, &path, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to compute critical path: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        blocked_task_next_lawful_receipt, build_adaptive_replan_finding_preview,
        build_spawn_blocker_preview, build_split_mutation_preview,
        canonical_json_string_array_entries, classify_task_close_git_stage_failure,
        ensure_existing_task_mismatch_reason, exception_takeover_state_label,
        load_adaptive_preview_finding_json, normalize_task_json_contract_arrays,
        parse_adaptive_replan_finding_input, parse_label_values, parse_optional_label_value,
        parse_proof_target_values, parse_split_child_specs,
        pass_completed_lane_task_next_lawful_receipt,
        pass_exception_takeover_task_next_lawful_receipt,
        pass_ready_downstream_handoff_task_next_lawful_receipt,
        persist_task_handoff_accept_receipt, runtime_binding_has_active_exception_takeover,
        runtime_binding_open_delegated_cycle_next_action, runtime_recovery_blocks_task_next_lawful,
        select_task_next_lawful_binding, task_close_automation_is_blocked,
        task_close_automation_receipt, task_close_commit_allowlist_next_actions,
        task_close_commit_file_strings, task_close_epic_progress_summary,
        task_close_feedback_blocker_summary, task_close_host_agent_telemetry,
        task_close_result_payload, task_close_uses_isolated_state_dir, task_continuation_candidate,
        task_create_planner_metadata_arg, task_create_semantics_mismatch,
        task_create_semantics_requested, task_create_title, task_critical_path_snapshot_first,
        task_exception_takeover_metadata_path, task_exception_takeover_owned_write_scope,
        task_handoff_accept_receipt, task_handoff_project_receipt_root, task_handoff_receipt_path,
        task_handoff_receipt_root, task_json_success_status, task_next_lawful_apply_strategy,
        task_next_lawful_receipt, task_next_lawful_select_ready_candidate_receipt,
        task_owned_status_receipt, task_parent_id, task_ready_authoritative_first,
        task_takeover_status_receipt, task_update_planner_metadata_arg,
        validate_task_handoff_accept_receipt, TaskCloseAutomationReceipt,
        ADAPTIVE_REPLAN_FINDING_KINDS,
    };
    use crate::state_store;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::cli;
    use crate::test_cli_support::guard_current_dir;
    use crate::test_cli_support::EnvVarGuard;
    use std::fs;
    use std::process::ExitCode;

    async fn create_task_for_test(
        store: &crate::StateStore,
        task_id: &str,
        title: &str,
        issue_type: &str,
        status: &str,
        priority: u32,
        parent_id: Option<&str>,
    ) {
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id,
                title,
                display_id: None,
                description: "",
                issue_type,
                status,
                priority,
                parent_id,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("task should create");
    }

    #[test]
    fn task_takeover_status_cli_accepts_json_task_and_run_filters() {
        let parsed = cli(&[
            "task",
            "takeover",
            "status",
            "--task-id",
            "task-1",
            "--run-id",
            "run-1",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Takeover(takeover) = args.command else {
            panic!("takeover command should parse");
        };
        let crate::TaskTakeoverCommand::Status(status) = takeover.command;

        assert_eq!(status.task_id_filter.as_deref(), Some("task-1"));
        assert_eq!(status.run_id.as_deref(), Some("run-1"));
        assert!(status.json);
    }

    #[test]
    fn task_takeover_status_cli_accepts_positional_task_id() {
        let parsed = cli(&["task", "takeover", "status", "task-1", "--json"]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Takeover(takeover) = args.command else {
            panic!("takeover command should parse");
        };
        let crate::TaskTakeoverCommand::Status(status) = takeover.command;

        assert_eq!(status.task_id.as_deref(), Some("task-1"));
        assert!(status.json);
    }

    #[test]
    fn task_takeover_status_labels_release_takeover_states() {
        assert_eq!(
            exception_takeover_state_label(
                crate::release1_contracts::ExceptionTakeoverState::NotRecorded
            ),
            "not_recorded"
        );
        assert_eq!(
            exception_takeover_state_label(
                crate::release1_contracts::ExceptionTakeoverState::ReceiptRecorded
            ),
            "receipt_recorded"
        );
        assert_eq!(
            exception_takeover_state_label(
                crate::release1_contracts::ExceptionTakeoverState::ActiveTakeover
            ),
            "active"
        );
    }

    #[test]
    fn task_takeover_status_reads_receipt_bound_owned_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let run_id = "task-takeover-status-scope";
        let metadata_path =
            task_exception_takeover_metadata_path(harness.path(), run_id).expect("metadata path");
        fs::create_dir_all(metadata_path.parent().expect("metadata dir should exist"))
            .expect("metadata dir should create");
        fs::write(
            &metadata_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "source_exception_path_receipt_id": "takeover-receipt",
                "owned_write_scope": [
                    " crates/vida/src/task_surface.rs ",
                    ""
                ]
            })
            .to_string(),
        )
        .expect("metadata should write");
        let summary = state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("takeover-receipt".to_string()),
            exception_path_receipt_id: Some("takeover-receipt".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("host_tool_bridge".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("host_tool_capability_missing".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("implementer".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-06-04T00:00:00Z".to_string(),
        };

        assert_eq!(
            task_exception_takeover_owned_write_scope(harness.path(), &summary),
            vec!["crates/vida/src/task_surface.rs".to_string()]
        );
    }

    #[test]
    fn task_takeover_status_blocks_active_takeover_without_receipt_bound_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record(
                "task-takeover-wide-scope",
                vec!["attacker-controlled/wide-scope"],
            );
            let status = state_store::RunGraphStatus {
                run_id: "task-takeover-no-metadata".to_string(),
                task_id: task.id.clone(),
                task_class: "implementation".to_string(),
                active_node: "implementer".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: "implementer".to_string(),
                lifecycle_stage: "implementer_blocked".to_string(),
                policy_gate: "blocked_open_delegated_cycle".to_string(),
                handoff_state: "bridge_request_pending".to_string(),
                context_state: "ready".to_string(),
                checkpoint_kind: "runtime_dispatch".to_string(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
            store
                .record_run_graph_status(&status)
                .await
                .expect("run graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&state_store::RunGraphDispatchReceipt {
                    run_id: status.run_id.clone(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "blocked".to_string(),
                    lane_status: "lane_exception_takeover".to_string(),
                    supersedes_receipt_id: Some("takeover-receipt".to_string()),
                    exception_path_receipt_id: Some("takeover-receipt".to_string()),
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("junior".to_string()),
                    activation_runtime_role: Some("implementer".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-04T00:00:00Z".to_string(),
                })
                .await
                .expect("dispatch receipt should persist");

            let receipt =
                task_takeover_status_receipt(&store, &task, Some(status), Some("run_id")).await;

            assert!(!receipt.allowed);
            assert!(!receipt.root_local_write_allowed);
            assert!(receipt.paths.is_empty());
            assert_eq!(
                receipt.root_write_guard["root_local_write_allowed_for_only_these_paths"],
                serde_json::json!([])
            );
            assert_eq!(
                receipt.blocker_codes,
                vec!["exception_takeover_scope_missing".to_string()]
            );
        });
    }

    #[test]
    fn task_block_cli_accepts_reason_evidence_and_repeated_recovery_fields() {
        let parsed = cli(&[
            "task",
            "block",
            "task-1",
            "--reason",
            "runtime bridge unavailable",
            "--evidence",
            "agent-init receipt path",
            "--blocker",
            "host_tool_capability_missing,bridge_request_pending",
            "--next-action",
            "run host bridge repair",
            "--next-action",
            "retry agent-init",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Block(command) = args.command else {
            panic!("block command should parse");
        };

        assert_eq!(command.task_id, "task-1");
        assert_eq!(command.reason, "runtime bridge unavailable");
        assert_eq!(command.evidence.as_deref(), Some("agent-init receipt path"));
        assert_eq!(
            command.blockers,
            vec![
                "host_tool_capability_missing".to_string(),
                "bridge_request_pending".to_string()
            ]
        );
        assert_eq!(
            command.next_actions,
            vec![
                "run host bridge repair".to_string(),
                "retry agent-init".to_string()
            ]
        );
        assert!(command.json);
    }

    #[test]
    fn task_verify_cli_accepts_partial_proof_flags() {
        let parsed = cli(&[
            "task",
            "verify",
            "task-1",
            "--source-fixed",
            "--tests-green",
            "--proof-blocked",
            "--proof-blocker",
            "browser proof unavailable",
            "--evidence",
            "cargo test -p vida task_verify",
            "--evidence",
            "target/debug/vida task verify smoke",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Verify(command) = args.command else {
            panic!("verify command should parse");
        };

        assert_eq!(command.task_id, "task-1");
        assert!(command.source_fixed);
        assert!(command.tests_green);
        assert!(command.proof_blocked);
        assert_eq!(
            command.proof_blocker.as_deref(),
            Some("browser proof unavailable")
        );
        assert_eq!(command.evidence.len(), 2);
        assert!(command.json);
    }

    #[test]
    fn task_verify_command_records_partial_state_without_closing() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "verify-task",
                "Verify task",
                "task",
                "in_progress",
                2,
                Some("parent-epic"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Verify(crate::TaskVerifyArgs {
                    task_id: "verify-task".to_string(),
                    source_fixed: true,
                    tests_green: true,
                    proof_blocked: true,
                    proof_blocker: Some("browser proof unavailable".to_string()),
                    evidence: vec!["cargo test -p vida task_verify".to_string()],
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("verify-task")
                .await
                .expect("verify task should load");
            assert_eq!(task.status, "in_progress");
            assert_eq!(task.closed_at, None);
            assert_eq!(task.close_reason, None);
            assert!(task.labels.contains(&"source-fixed".to_string()));
            assert!(task.labels.contains(&"tests-green".to_string()));
            assert!(task
                .labels
                .contains(&"proof-blocked-by-runtime".to_string()));
            assert_eq!(
                task.planner_metadata.proof_targets,
                vec!["cargo test -p vida task_verify".to_string()]
            );
            let notes = task
                .notes
                .expect("partial verification note should persist");
            assert!(notes.contains("task_partial_verification:"));
            assert!(notes.contains("source_fixed: true"));
            assert!(notes.contains("tests_green: true"));
            assert!(notes.contains("proof_blocked: true"));
            assert!(notes.contains("browser proof unavailable"));

            let progress = store
                .task_progress_summary("verify-task")
                .await
                .expect("progress should compute");
            assert!(progress.proof_blocked_by_runtime);
            assert!(progress.blocked_by_runtime);
        });
    }

    #[test]
    fn task_verify_command_rejects_closed_task_without_mutation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "closed-verify-task",
                "Closed verify task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("closed-verify-task", "done")
                .await
                .expect("task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Verify(crate::TaskVerifyArgs {
                    task_id: "closed-verify-task".to_string(),
                    source_fixed: true,
                    tests_green: true,
                    proof_blocked: true,
                    proof_blocker: Some("browser proof unavailable".to_string()),
                    evidence: Vec::new(),
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("closed-verify-task")
                .await
                .expect("closed verify task should load");
            assert_eq!(task.status, "closed");
            assert_eq!(task.close_reason.as_deref(), Some("done"));
            assert!(task.labels.is_empty());
            assert!(task.notes.is_none());
        });
    }

    #[test]
    fn task_block_command_marks_task_blocked_without_closing() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "blocked-task",
                "Blocked task",
                "task",
                "in_progress",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .update_task(crate::state_store::UpdateTaskRequest {
                    task_id: "blocked-task",
                    title: None,
                    status: None,
                    priority: None,
                    notes: Some("existing note"),
                    description: None,
                    parent_id: None,
                    add_labels: &[],
                    remove_labels: &[],
                    set_labels: None,
                    execution_mode: None,
                    order_bucket: None,
                    parallel_group: None,
                    conflict_domain: None,
                    planner_metadata: None,
                })
                .await
                .expect("notes update should persist");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Block(crate::TaskBlockArgs {
                    task_id: "blocked-task".to_string(),
                    reason: "runtime bridge unavailable".to_string(),
                    evidence: Some("agent-init returned host_tool_capability_missing".to_string()),
                    blockers: vec!["host_tool_capability_missing".to_string()],
                    next_actions: vec!["retry after host bridge repair".to_string()],
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("blocked-task")
                .await
                .expect("blocked task should load");
            assert_eq!(task.status, "blocked");
            assert_eq!(task.closed_at, None);
            assert_eq!(task.close_reason, None);
            let notes = task.notes.expect("block note should persist");
            assert!(notes.contains("existing note"));
            assert!(notes.contains("task_block:"));
            assert!(notes.contains("runtime bridge unavailable"));
            assert!(notes.contains("host_tool_capability_missing"));
            assert!(notes.contains("retry after host bridge repair"));
        });
    }

    #[test]
    fn task_block_command_rejects_closed_task_without_mutation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "closed-task",
                "Closed task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("closed-task", "done")
                .await
                .expect("task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Block(crate::TaskBlockArgs {
                    task_id: "closed-task".to_string(),
                    reason: "runtime bridge unavailable".to_string(),
                    evidence: Some("receipt path".to_string()),
                    blockers: Vec::new(),
                    next_actions: Vec::new(),
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("closed-task")
                .await
                .expect("closed task should load");
            assert_eq!(task.status, "closed");
            assert_eq!(task.close_reason.as_deref(), Some("done"));
            assert!(task.notes.is_none());
        });
    }

    fn minimal_task_create_args(
        positional_title: Option<&str>,
        title: Option<&str>,
    ) -> crate::TaskCreateArgs {
        crate::TaskCreateArgs {
            task_id: "task-title-test".to_string(),
            positional_title: positional_title.map(str::to_string),
            title: title.map(str::to_string),
            issue_type: "task".to_string(),
            status: "open".to_string(),
            priority: 2,
            display_id: None,
            parent_id: None,
            parent_display_id: None,
            auto_display_from: None,
            description: String::new(),
            notes: None,
            notes_file: None,
            labels: Vec::new(),
            execution_mode: None,
            order_bucket: None,
            parallel_group: None,
            conflict_domain: None,
            owned_paths: Vec::new(),
            acceptance_targets: Vec::new(),
            proof_targets: Vec::new(),
            state_dir: None,
            render: crate::RenderMode::Plain,
            json: false,
        }
    }

    fn owned_task_record(task_id: &str, owned_paths: Vec<&str>) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: "Owned task".to_string(),
            description: String::new(),
            status: "in_progress".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata {
                owned_paths: owned_paths.into_iter().map(str::to_string).collect(),
                acceptance_targets: Vec::new(),
                proof_targets: Vec::new(),
                risk: None,
                estimate: None,
                lane_hint: None,
            },
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn task_update_planner_metadata_sets_requested_lists_and_preserves_existing_fields() {
        let existing = crate::state_store::TaskPlannerMetadata {
            owned_paths: vec!["old/path.rs".to_string()],
            acceptance_targets: vec!["old acceptance".to_string()],
            proof_targets: vec!["old proof".to_string()],
            risk: Some("high".to_string()),
            estimate: Some("small".to_string()),
            lane_hint: Some("worker".to_string()),
        };
        let command = crate::TaskUpdateArgs {
            task_id: "task-owned".to_string(),
            owned_paths: vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "crates/vida/src/cli.rs".to_string(),
            ],
            proof_targets: vec![
                "cargo test -p vida task_update_planner_metadata proof_target_values".to_string(),
            ],
            ..Default::default()
        };

        let metadata = task_update_planner_metadata_arg(&existing, &command)
            .expect("metadata update should be requested");

        assert_eq!(
            metadata.owned_paths,
            vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "crates/vida/src/cli.rs".to_string(),
            ]
        );
        assert_eq!(metadata.acceptance_targets, existing.acceptance_targets);
        assert_eq!(
            metadata.proof_targets,
            vec![
                "cargo test -p vida task_update_planner_metadata".to_string(),
                "cargo test -p vida proof_target_values".to_string(),
            ]
        );
        assert_eq!(metadata.risk, existing.risk);
        assert_eq!(metadata.estimate, existing.estimate);
        assert_eq!(metadata.lane_hint, existing.lane_hint);
    }

    #[test]
    fn task_create_planner_metadata_normalizes_proof_targets() {
        let mut command = minimal_task_create_args(Some("Task"), None);
        command.proof_targets = vec![
            "vida diagnostics --json".to_string(),
            "vida docflow protocol-coverage-check --profile active-canon --format jsonl"
                .to_string(),
        ];

        let metadata = task_create_planner_metadata_arg(&command);

        assert_eq!(
            metadata.proof_targets,
            vec![
                "vida diagnostics post-commit --json",
                "vida docflow protocol-coverage-check --profile active-canon",
            ]
        );
    }

    #[test]
    fn task_owned_status_splits_dirty_files_by_owned_paths() {
        let receipt = task_owned_status_receipt(
            "task-owned",
            vec!["crates/vida/src".to_string()],
            Vec::new(),
            vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "README.md".to_string(),
            ],
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "planner_metadata.owned_paths");
        assert_eq!(receipt.owned_files, vec!["crates/vida/src/task_surface.rs"]);
        assert_eq!(
            receipt.stageable_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(receipt.unowned_files, vec!["README.md"]);
        assert_eq!(receipt.blocker_codes, vec!["dirty_ownership_ambiguous"]);
    }

    #[test]
    fn ensure_existing_task_rejects_contract_mismatch() {
        let mut task = owned_task_record("task-ensure", vec![]);
        task.title = "Unexpected".to_string();
        task.status = "closed".to_string();
        task.issue_type = "bug".to_string();
        task.labels = vec!["other".to_string()];
        task.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: task.id.clone(),
            depends_on_id: "other-parent".to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        assert_eq!(task_parent_id(&task).as_deref(), Some("other-parent"));
        let reason = ensure_existing_task_mismatch_reason(
            &task,
            "Expected",
            None,
            "task",
            "open",
            Some("expected-parent"),
            &["tracked-pack".to_string()],
        )
        .expect("mismatch reason should exist");
        assert!(reason.contains("title mismatch"));
    }

    #[test]
    fn task_ensure_detects_requested_execution_semantics_backfill() {
        let existing = crate::state_store::TaskExecutionSemantics::default();
        let mut command = minimal_task_create_args(Some("Ensure semantics"), None);
        command.execution_mode = Some("parallel_safe".to_string());
        command.order_bucket = Some("feature-x".to_string());
        command.parallel_group = Some("dev-pack".to_string());
        command.conflict_domain = Some("task-ensure-semantics".to_string());

        assert!(task_create_semantics_requested(&command));
        assert!(task_create_semantics_mismatch(&existing, &command));
    }

    #[test]
    fn task_owned_status_fails_closed_without_ownership_source() {
        let receipt = task_owned_status_receipt(
            "task-owned",
            Vec::new(),
            Vec::new(),
            vec!["crates/vida/src/task_surface.rs".to_string()],
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "missing");
        assert_eq!(receipt.blocker_codes, vec!["missing_owned_paths"]);
        assert!(receipt.stageable_files.is_empty());
    }

    #[test]
    fn task_close_epic_progress_summary_reports_epic_percentages_and_child_rows() {
        let mut epic = owned_task_record("epic-a", vec![]);
        epic.title = "Epic A".to_string();
        epic.issue_type = "epic".to_string();
        epic.status = "open".to_string();
        epic.priority = 1;

        let mut closed_child = owned_task_record("child-closed", vec![]);
        closed_child.title = "Closed child".to_string();
        closed_child.status = "closed".to_string();
        closed_child.priority = 1;
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: epic.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        let mut blocked_child = owned_task_record("child-blocked", vec![]);
        blocked_child.title = "Blocked child".to_string();
        blocked_child.status = "open".to_string();
        blocked_child.priority = 2;
        blocked_child.dependencies = vec![
            crate::state_store::TaskDependencyRecord {
                issue_id: blocked_child.id.clone(),
                depends_on_id: epic.id.clone(),
                edge_type: "parent-child".to_string(),
                created_at: "2026-06-02T00:00:00Z".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            },
            crate::state_store::TaskDependencyRecord {
                issue_id: blocked_child.id.clone(),
                depends_on_id: "blocker-task".to_string(),
                edge_type: "blocks".to_string(),
                created_at: "2026-06-02T00:00:00Z".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            },
        ];

        let mut blocker = owned_task_record("blocker-task", vec![]);
        blocker.title = "Blocking task".to_string();
        blocker.status = "open".to_string();

        let mut unrelated_epic = owned_task_record("epic-unrelated", vec![]);
        unrelated_epic.issue_type = "epic".to_string();
        unrelated_epic.status = "open".to_string();

        let rows = vec![
            epic,
            closed_child.clone(),
            blocked_child,
            blocker,
            unrelated_epic,
        ];
        let summary = task_close_epic_progress_summary(&rows, &closed_child.id, false)
            .expect("epic progress summary should build from task graph rows");

        assert_eq!(summary.closed_task_id, "child-closed");
        assert_eq!(summary.epic_count, 1);
        assert_eq!(summary.omitted_epic_count, 1);
        assert_eq!(summary.scope, "closed_task_ancestor_epics");
        let epic_row = &summary.epics[0];
        assert_eq!(epic_row.epic_id, "epic-a");
        assert_eq!(epic_row.closed_count, 1);
        assert_eq!(epic_row.total_count, 2);
        assert_eq!(epic_row.percent_closed, 50.0);
        assert_eq!(epic_row.child_task_count, 2);
        assert_eq!(epic_row.reported_child_task_count, 2);
        let blocked_row = epic_row
            .tasks
            .iter()
            .find(|task| task.task_id == "child-blocked")
            .expect("blocked child should be reported");
        assert_eq!(blocked_row.blocker_state, "blocked");
        assert_eq!(blocked_row.blockers[0].task_id, "blocker-task");
        assert!(blocked_row.next_action.contains("Resolve blocking tasks"));
    }

    #[test]
    fn task_close_result_payload_includes_epic_progress_summary() {
        let mut epic = owned_task_record("epic-a", vec![]);
        epic.issue_type = "epic".to_string();
        let mut closed_child = owned_task_record("child-closed", vec![]);
        closed_child.status = "closed".to_string();
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: epic.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let summary =
            task_close_epic_progress_summary(&[epic, closed_child.clone()], "child-closed", false)
                .expect("summary should build");

        let payload = task_close_result_payload(
            &closed_child,
            &serde_json::json!({"status": "recorded"}),
            None,
            None,
            Some(&summary),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], false);
        assert_eq!(payload["task"]["id"], "child-closed");
        assert_eq!(
            payload["epic_progress_summary"]["epics"][0]["epic_id"],
            "epic-a"
        );
        assert_eq!(
            payload["epic_progress_summary"]["epics"][0]["closed_count"],
            1
        );
    }

    #[test]
    fn task_proof_status_payload_reports_missing_and_satisfied_targets() {
        let mut task = owned_task_record("proof-task", vec![]);
        task.status = "closed".to_string();
        task.close_reason =
            Some("Proof: cargo test -p vida proof_status_payload passed.".to_string());
        task.planner_metadata.proof_targets = vec![
            "cargo test -p vida proof_status_payload".to_string(),
            "cargo build -p vida".to_string(),
        ];

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["task_id"], "proof-task");
        assert_eq!(payload["configured_proof_target_count"], 2);
        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["missing_proof"], true);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "close_reason"
        );
        assert_eq!(payload["proof_targets"][1]["status"], "missing_evidence");
        assert_eq!(payload["missing_targets"][0], "cargo build -p vida");
    }

    #[test]
    fn task_proof_attach_browser_cli_accepts_artifact_fields() {
        let parsed = cli(&[
            "task",
            "proof",
            "attach-browser",
            "proof-task",
            "--route",
            "/odoo",
            "--expect",
            "My Tasks",
            "--result",
            "pass",
            "--screenshot",
            "artifacts/proof.png",
            "--evidence",
            "console clean",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Proof(proof) = args.command else {
            panic!("task proof command should parse");
        };
        let crate::TaskProofCommand::AttachBrowser(command) = proof.command else {
            panic!("attach-browser command should parse");
        };

        assert_eq!(command.task_id, "proof-task");
        assert_eq!(command.route, "/odoo");
        assert_eq!(command.expect.as_deref(), Some("My Tasks"));
        assert_eq!(command.result, "pass");
        assert_eq!(command.screenshot.as_deref(), Some("artifacts/proof.png"));
        assert_eq!(command.evidence, vec!["console clean".to_string()]);
        assert!(command.json);
    }

    #[test]
    fn task_proof_status_payload_accepts_browser_attach_note() {
        let mut task = owned_task_record("proof-task", vec![]);
        let proof_target = super::browser_proof_target("/odoo", Some("My Tasks"));
        task.planner_metadata.proof_targets = vec![proof_target.clone()];
        task.notes = Some(super::append_task_browser_proof_note(
            None,
            &proof_target,
            "/odoo",
            "pass",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &["console clean".to_string()],
        ));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 0);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "task_browser_proof_note"
        );
        assert_eq!(
            payload["evidence_model"]["artifact_registry"],
            "task_notes.task_browser_proof"
        );
    }

    #[test]
    fn task_proof_status_payload_rejects_failed_browser_note_with_pass_text_in_evidence() {
        let mut task = owned_task_record("proof-task", vec![]);
        let proof_target = super::browser_proof_target("/secure", Some("OK"));
        task.planner_metadata.proof_targets = vec![proof_target.clone()];
        task.notes = Some(super::append_task_browser_proof_note(
            None,
            &proof_target,
            "/secure",
            "fail",
            Some("OK"),
            Some("artifacts/proof.png"),
            &["console included result: pass text".to_string()],
        ));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 0);
        assert_eq!(payload["missing_count"], 1);
        assert_ne!(payload["proof_targets"][0]["status"], "satisfied");
    }

    #[test]
    fn task_proof_status_payload_scopes_browser_pass_to_matching_target_record() {
        let mut task = owned_task_record("proof-task", vec![]);
        let other_target = super::browser_proof_target("/other", None);
        let secure_target = super::browser_proof_target("/secure", None);
        task.planner_metadata.proof_targets = vec![other_target.clone(), secure_target.clone()];
        let notes = super::append_task_browser_proof_note(
            None,
            &other_target,
            "/other",
            "pass",
            None,
            None,
            &[],
        );
        task.notes = Some(super::append_task_browser_proof_note(
            Some(&notes),
            &secure_target,
            "/secure",
            "fail",
            None,
            None,
            &[],
        ));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_ne!(payload["proof_targets"][1]["status"], "satisfied");
        assert_eq!(payload["missing_targets"][0], secure_target);
    }

    #[test]
    fn append_task_browser_proof_note_normalizes_newlines_in_untrusted_fields() {
        let note = super::append_task_browser_proof_note(
            None,
            "vida proof browser --route /secure",
            "/secure",
            "fail",
            Some("OK\n  result: pass"),
            Some("artifacts/proof.png\n  result: pass"),
            &["first line\n  result: pass".to_string()],
        );

        assert!(note.contains("  result: fail\n"));
        assert!(!note.contains("\n  expect: OK\n  result: pass"));
        assert!(!note.contains("\n  screenshot: artifacts/proof.png\n  result: pass"));
        assert!(!note.contains("\n  evidence: first line\n  result: pass"));
    }

    #[test]
    fn task_proof_status_payload_reports_unconfigured_targets() {
        let task = owned_task_record("proofless-task", vec![]);

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["configured_proof_target_count"], 0);
        assert_eq!(payload["missing_proof"], false);
        assert!(payload["next_required_command"]
            .as_str()
            .expect("next command should render")
            .contains("vida task update proofless-task --proof-target"));
    }

    #[test]
    fn task_proof_status_payload_quotes_unconfigured_task_id_command_hint() {
        let task = owned_task_record("safe; touch /tmp/vida_pwned #", vec![]);

        let payload = super::task_proof_status_payload(&task, None);
        let next_required_command = payload["next_required_command"]
            .as_str()
            .expect("next command should render");

        assert!(next_required_command
            .contains("vida task update 'safe; touch /tmp/vida_pwned #' --proof-target"));
        assert!(!next_required_command.contains("vida task update safe; touch"));
    }

    #[test]
    fn task_proof_status_payload_quotes_missing_proof_task_id_command_hint() {
        let mut task = owned_task_record("safe; touch /tmp/vida_pwned #", vec![]);
        task.status = "closed".to_string();
        task.planner_metadata.proof_targets = vec!["cargo test -p vida".to_string()];

        let payload = super::task_proof_status_payload(&task, None);
        let next_required_command = payload["next_required_command"]
            .as_str()
            .expect("next command should render");

        assert!(next_required_command
            .contains("vida task proof status 'safe; touch /tmp/vida_pwned #' --json"));
        assert!(!next_required_command.contains("vida task proof status safe; touch"));
    }

    #[test]
    fn task_close_result_payload_keeps_success_status_when_continuation_is_blocked() {
        let mut closed_task = owned_task_record("closed-with-blocker", vec![]);
        closed_task.status = "closed".to_string();
        let telemetry = serde_json::json!({
            "status": "recorded",
            "reason": "feedback recorded after close"
        });
        let blockers = (
            vec!["post_close_feedback_blocked".to_string()],
            vec!["Inspect continuation blocker separately.".to_string()],
        );

        let payload =
            task_close_result_payload(&closed_task, &telemetry, None, Some(&blockers), None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], true);
        assert_eq!(payload["feedback_blocked"], true);
        assert_eq!(payload["automation_blocked"], false);
        assert_eq!(payload["blocker_codes"][0], "post_close_feedback_blocked");
    }

    #[test]
    fn task_close_result_payload_reports_blocked_status_for_blocked_automation() {
        let mut closed_task = owned_task_record("closed-with-automation-blocker", vec![]);
        closed_task.status = "closed".to_string();
        let telemetry = serde_json::json!({
            "status": "recorded",
            "reason": "feedback recorded after close"
        });
        let automation = TaskCloseAutomationReceipt {
            status: "blocked".to_string(),
            blocker_codes: vec!["push_requires_commit".to_string()],
            next_actions: vec!["Pass `--commit --commit-file <path>` with `--push`.".to_string()],
            release_build: None,
            release_install: None,
            git: None,
        };

        let payload =
            task_close_result_payload(&closed_task, &telemetry, Some(&automation), None, None);

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], true);
        assert_eq!(payload["automation_blocked"], true);
        assert_eq!(payload["feedback_blocked"], false);
        assert_eq!(payload["blocker_codes"][0], "push_requires_commit");
        assert!(task_close_automation_is_blocked(Some(&automation)));
    }

    #[test]
    fn task_close_commit_files_prioritize_explicit_commit_paths() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let files = task_close_commit_file_strings(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: "done".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
                stage_owned: true,
                commit_files: vec![std::path::PathBuf::from("README.md")],
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            Some(&task),
        );

        assert_eq!(files, vec!["README.md"]);
    }

    #[test]
    fn task_close_commit_files_falls_back_to_stage_owned_without_explicit_paths() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let files = task_close_commit_file_strings(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: "done".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
                stage_owned: true,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            Some(&task),
        );

        assert_eq!(files, vec!["crates/vida/src"]);
    }

    #[test]
    fn task_close_commit_allowlist_reports_ignored_dirty_files_diagnostically() {
        let next_actions = task_close_commit_allowlist_next_actions(&[
            "AGENTS.md".to_string(),
            "crates/vida/src/taskflow_proxy.rs".to_string(),
        ]);

        assert_eq!(
            next_actions,
            vec![
                "Ignored 2 unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied."
            ]
        );
        assert!(task_close_commit_allowlist_next_actions(&[]).is_empty());
    }

    #[test]
    fn task_close_git_stage_failure_classifies_read_only_or_sandbox_stderr() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: Unable to create '/repo/.git/index.lock': Read-only file system",
            None,
        );

        assert_eq!(
            failure.blocker_code,
            "git_stage_read_only_or_sandbox_blocked"
        );
        assert!(failure.detail.contains("Read-only file system"));
        assert!(failure.next_action.contains("writable"));
    }

    #[test]
    fn task_close_git_stage_failure_classifies_index_lock_stderr() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: Unable to create '/repo/.git/index.lock': File exists.",
            None,
        );

        assert_eq!(failure.blocker_code, "git_stage_index_lock_blocked");
        assert!(failure.detail.contains(".git/index.lock"));
        assert!(failure.next_action.contains(".git/index.lock"));
    }

    #[test]
    fn task_close_git_stage_failure_preserves_fallback_stderr_detail() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: pathspec 'missing-file' did not match any files",
            None,
        );

        assert_eq!(failure.blocker_code, "git_stage_failed");
        assert_eq!(
            failure.detail,
            "fatal: pathspec 'missing-file' did not match any files"
        );
        assert_eq!(
            failure.next_action,
            "Verify the explicit commit files exist and can be staged."
        );
    }

    #[test]
    fn task_close_stage_owned_without_commit_fails_closed() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: "done".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: false,
                include_global_progress: false,
                stage_owned: true,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            Some(&task),
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["stage_owned_requires_commit"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["stage_owned_requires_commit"]);
        assert_eq!(git.explicit_files, vec!["crates/vida/src"]);
    }

    #[test]
    fn task_handoff_accept_receipt_records_queryable_contents() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task/handoff", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task/handoff".to_string(),
                agent: Some("worker-1".to_string()),
                files: vec![
                    std::path::PathBuf::from("crates/vida/src/task_surface.rs"),
                    std::path::PathBuf::from("crates/vida/src/task_surface.rs"),
                ],
                proofs: vec![
                    " cargo test -p vida --bin vida task_handoff ".to_string(),
                    "cargo check -p vida --bin vida".to_string(),
                ],
                status: crate::TaskHandoffStatusArg::Pass,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.task_id, "task/handoff");
        assert_eq!(receipt.agent_id, "worker-1");
        assert_eq!(
            receipt.changed_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(
            receipt.proof_commands,
            vec![
                "cargo test -p vida --bin vida task_handoff",
                "cargo check -p vida --bin vida"
            ]
        );
        assert!(receipt
            .receipt_path
            .replace('\\', "/")
            .ends_with(".vida/receipts/task-handoffs/task-handoff-123.json"));
        assert_eq!(receipt.receipt_root, receipt_root.display().to_string());
        assert_eq!(receipt.isolation, "project_state_dir");
        validate_task_handoff_accept_receipt(&receipt)
            .expect("pass handoff with agent should validate");
        persist_task_handoff_accept_receipt(&receipt, &receipt_path)
            .expect("receipt should persist");
        let persisted = fs::read_to_string(&receipt_path).expect("receipt should be readable");
        let value: serde_json::Value =
            serde_json::from_str(&persisted).expect("receipt json should parse");
        assert_eq!(value["status"], "pass");
        assert_eq!(value["task_id"], "task/handoff");
        assert_eq!(value["agent_id"], "worker-1");
        assert_eq!(
            value["changed_files"],
            serde_json::json!(["crates/vida/src/task_surface.rs"])
        );
        let overwrite_error = persist_task_handoff_accept_receipt(&receipt, &receipt_path)
            .expect_err("receipt writer should not overwrite existing receipts");
        assert!(overwrite_error.contains("without overwrite"));
    }

    #[test]
    fn blocked_task_handoff_without_detail_fails_validation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task-a", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task-a".to_string(),
                agent: Some("worker-1".to_string()),
                files: Vec::new(),
                proofs: Vec::new(),
                status: crate::TaskHandoffStatusArg::Blocked,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        let error = validate_task_handoff_accept_receipt(&receipt)
            .expect_err("blocked handoff without blocker or proof should fail closed");
        assert_eq!(error.0, "blocked_handoff_requires_detail");
    }

    #[test]
    fn task_handoff_accept_without_agent_fails_validation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task-a", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task-a".to_string(),
                agent: None,
                files: vec![std::path::PathBuf::from("crates/vida/src/task_surface.rs")],
                proofs: vec!["cargo check -p vida --bin vida".to_string()],
                status: crate::TaskHandoffStatusArg::Pass,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        let error =
            validate_task_handoff_accept_receipt(&receipt).expect_err("missing agent should block");
        assert_eq!(error.0, "missing_agent_id");
    }

    #[test]
    fn task_handoff_accept_isolated_state_dir_writes_receipt_under_state_dir() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path().join("project");
        fs::create_dir_all(project_root.join(".vida/receipts"))
            .expect("project receipt directory should initialize");
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker directory should initialize");
        let isolated_state_dir = harness.path().join("isolated-state");
        runtime.block_on(async {
            let store = crate::StateStore::open(isolated_state_dir.clone())
                .await
                .expect("isolated state store should open");
            create_task_for_test(
                &store,
                "task-handoff",
                "Task handoff",
                "epic",
                "open",
                2,
                None,
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let (receipt_root, isolation) = task_handoff_receipt_root(&isolated_state_dir, true);
        assert_eq!(isolation, "isolated_state_dir");
        assert_eq!(receipt_root, isolated_state_dir.join("receipts"));

        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let _cwd = guard_current_dir(&project_root);
        let code = runtime.block_on(crate::run(cli(&[
            "task",
            "handoff",
            "accept",
            "task-handoff",
            "--agent",
            "worker-1",
            "--file",
            "crates/vida/src/task_surface.rs",
            "--proof",
            "cargo check -p vida --bin vida",
            "--state-dir",
            isolated_state_dir
                .to_str()
                .expect("state dir should be utf8"),
            "--json",
        ])));
        drop(_cwd);

        assert_eq!(code, ExitCode::SUCCESS);
        let project_handoff_receipts = project_root.join(".vida/receipts/task-handoffs");
        assert!(
            !project_handoff_receipts.exists(),
            "isolated handoff must not write project receipts at {}",
            project_handoff_receipts.display()
        );
        let isolated_handoff_receipts = isolated_state_dir.join("receipts/task-handoffs");
        let receipts = fs::read_dir(&isolated_handoff_receipts)
            .expect("isolated receipt directory should exist")
            .collect::<Result<Vec<_>, _>>()
            .expect("isolated receipts should list");
        assert_eq!(receipts.len(), 1);
        let receipt_text =
            fs::read_to_string(receipts[0].path()).expect("isolated receipt should read");
        let receipt: serde_json::Value =
            serde_json::from_str(&receipt_text).expect("isolated receipt should parse");
        assert_eq!(receipt["status"], "pass");
        assert_eq!(receipt["task_id"], "task-handoff");
        assert_eq!(receipt["isolation"], "isolated_state_dir");
        assert_eq!(
            receipt["receipt_root"],
            isolated_state_dir.join("receipts").display().to_string()
        );
        assert!(receipt["receipt_path"]
            .as_str()
            .expect("receipt path should be string")
            .replace('\\', "/")
            .starts_with(
                isolated_handoff_receipts
                    .to_str()
                    .expect("receipt dir should be utf8")
                    .replace('\\', "/")
                    .as_str()
            ));
    }

    #[test]
    fn task_next_lawful_selects_single_ready_candidate() {
        let mut task = owned_task_record("task-ready", vec![]);
        task.status = "open".to_string();
        task.title = "Ready task".to_string();
        let ready = vec![super::task_continuation_candidate(&task, false)];

        let receipt = task_next_lawful_receipt(&[task], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-ready");
        assert_eq!(
            receipt.why_this_unit,
            "single ready TaskFlow candidate after close/release automation"
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_single_candidate"
        );
        assert_eq!(receipt.binding_source, None);
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida task next-lawful"));
    }

    #[test]
    fn task_next_lawful_blocks_multiple_ready_candidates() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            super::task_continuation_candidate(&first, false),
            super::task_continuation_candidate(&second, false),
        ];

        let receipt = task_next_lawful_receipt(&[first, second], ready, None);

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["ambiguous_ready_task_candidates"]
        );
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(receipt.ready_task_candidates.len(), 2);
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-a")
        );
        assert_eq!(
            receipt.bind_command.as_deref(),
            Some("vida taskflow run-graph dispatch-init task-a --json")
        );
        assert!(receipt
            .why_not_auto_bound
            .as_deref()
            .is_some_and(|reason| reason.contains("multiple ready candidates")));
    }

    #[test]
    fn task_next_lawful_epic_sequential_strategy_keeps_primary_epic_candidates() {
        let mut epic_a = owned_task_record("epic-a", vec![]);
        epic_a.issue_type = "epic".to_string();
        let mut epic_b = owned_task_record("epic-b", vec![]);
        epic_b.issue_type = "epic".to_string();
        let mut a_first = owned_task_record("task-a-first", vec![]);
        a_first.status = "open".to_string();
        a_first.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: a_first.id.clone(),
            depends_on_id: epic_a.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let mut a_second = owned_task_record("task-a-second", vec![]);
        a_second.status = "open".to_string();
        a_second.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: a_second.id.clone(),
            depends_on_id: epic_a.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let mut b_first = owned_task_record("task-b-first", vec![]);
        b_first.status = "open".to_string();
        b_first.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: b_first.id.clone(),
            depends_on_id: epic_b.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let ready = vec![
            task_continuation_candidate(&a_first, false),
            task_continuation_candidate(&a_second, false),
            task_continuation_candidate(&b_first, false),
        ];

        let filtered = task_next_lawful_apply_strategy(
            &[epic_a, epic_b, a_first, a_second, b_first],
            ready,
            Some("epic-sequential"),
        );

        let ids = filtered
            .iter()
            .map(|candidate| candidate.task_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["task-a-first", "task-a-second"]);
    }

    #[test]
    fn task_next_lawful_select_ready_candidate_returns_selected_bind_command() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            task_continuation_candidate(&first, false),
            task_continuation_candidate(&second, true),
        ];

        let receipt =
            task_next_lawful_select_ready_candidate_receipt(&[first, second], ready, "task-b");

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-b");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("operator_selected_ready_candidate")
        );
        assert_eq!(
            receipt.bind_command.as_deref(),
            Some("vida taskflow run-graph dispatch-init task-b --json")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "parallel_safe_operator_selected_candidate"
        );
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-b")
        );
    }

    #[test]
    fn task_next_lawful_select_missing_candidate_fails_closed() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&first, false)];

        let receipt =
            task_next_lawful_select_ready_candidate_receipt(&[first], ready, "task-missing");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["selected_task_not_ready"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-a")
        );
    }

    #[test]
    fn task_next_lawful_select_blocks_when_taskflow_task_is_active() {
        let active = owned_task_record("task-active", vec![]);
        let mut ready_task = owned_task_record("task-ready", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, true)];

        let receipt = task_next_lawful_select_ready_candidate_receipt(
            &[active, ready_task],
            ready,
            "task-ready",
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["select_conflicts_with_active_taskflow_task"]
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-active");
        assert!(receipt
            .next_action
            .as_deref()
            .is_some_and(|action| action.contains("task-active")));
    }

    #[test]
    fn task_next_lawful_select_blocks_when_multiple_taskflow_tasks_are_active() {
        let active_a = owned_task_record("task-active-a", vec![]);
        let active_b = owned_task_record("task-active-b", vec![]);
        let mut ready_task = owned_task_record("task-ready", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, true)];

        let receipt = task_next_lawful_select_ready_candidate_receipt(
            &[active_a, active_b, ready_task],
            ready,
            "task-ready",
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["multiple_active_tasks"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
    }

    #[test]
    fn task_next_lawful_selects_in_progress_child_leaf_over_active_parent() {
        let mut parent = owned_task_record("generic-runtime-foundation-release-readiness", vec![]);
        parent.title = "Release readiness".to_string();
        let mut child = owned_task_record(
            "todo-repair-status-cold-after-mutation-timing-20260602",
            vec![],
        );
        child.title = "Repair cold timing".to_string();
        child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        let receipt = task_next_lawful_receipt(&[parent, child], Vec::new(), None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "todo-repair-status-cold-after-mutation-timing-20260602"
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_taskflow_active"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_blocks_runtime_derived_taskflow_active_conflict() {
        let mut runtime_task = owned_task_record("runtime-task", vec![]);
        runtime_task.status = "open".to_string();
        let active_task = owned_task_record("active-task", vec![]);
        let ready = vec![super::task_continuation_candidate(&runtime_task, false)];
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-1".to_string(),
            task_id: "runtime-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "task_id": "runtime-task",
                "kind": "run_graph_task"
            }),
            binding_source: "latest_run_graph_status".to_string(),
            why_this_unit: "runtime binding".to_string(),
            primary_path: "vida taskflow consume continue --run-id run-1 --json".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        };

        let receipt = task_next_lawful_receipt(&[runtime_task, active_task], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["runtime_taskflow_active_conflict"]
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "runtime-task");
    }

    #[test]
    fn task_next_lawful_ignores_missing_single_source_binding_for_taskflow_active() {
        let active_task = owned_task_record("active-task", vec![]);
        let binding = test_continuation_binding(
            "missing-run",
            "missing-runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            std::slice::from_ref(&active_task),
            Some(&binding),
            None,
        )
        .expect("missing binding should not fail source selection");
        assert!(selected.is_none());

        let receipt = task_next_lawful_receipt(&[active_task], Vec::new(), selected);

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "active-task");
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_keeps_missing_single_source_binding_without_taskflow_active() {
        let binding = test_continuation_binding(
            "missing-run",
            "missing-runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&binding), None)
            .expect("single stale binding should remain selectable without active fallback");

        assert_eq!(
            selected.map(|binding| binding.task_id.as_str()),
            Some("missing-runtime-task")
        );
    }

    #[test]
    fn task_next_lawful_blocks_explicit_task_binding_with_parallel_active_tasks() {
        let mut runtime_task = owned_task_record("runtime-task", vec![]);
        runtime_task.status = "open".to_string();
        let active_task = owned_task_record("active-task", vec![]);
        let ready = vec![super::task_continuation_candidate(&runtime_task, false)];
        let binding = test_continuation_binding(
            "run-1",
            "runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[runtime_task, active_task], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["runtime_taskflow_active_conflict"]
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "runtime-task");
    }

    fn test_continuation_binding(
        run_id: &str,
        task_id: &str,
        binding_source: &str,
        active_kind: &str,
    ) -> crate::state_store::RunGraphContinuationBinding {
        crate::state_store::RunGraphContinuationBinding {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": active_kind,
                "task_id": task_id,
                "run_id": run_id,
            }),
            binding_source: binding_source.to_string(),
            why_this_unit: format!("{binding_source} selects {task_id}"),
            primary_path: "vida taskflow consume continue --json".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn task_next_lawful_prefers_current_same_task_over_stale_explicit_run_binding() {
        let task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[task], Some(&explicit), Some(&current))
            .expect("same-task current run should supersede stale explicit task binding")
            .expect("current binding should select");

        assert_eq!(selected.run_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(
            selected.binding_source,
            "consume_continue_deferred_agent_handoff"
        );
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_over_unscoped_dispatch_init_projection() {
        let explicit_task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let current_task =
            owned_task_record("agent-mode-dev-team-test-first-operating-model", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "agent-mode-dev-team-test-first-operating-model",
            "agent-mode-dev-team-test-first-operating-model",
            "run_graph_dispatch_init",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect(
            "unscoped dispatch-init latest projection should not override live explicit binding",
        )
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_over_unrelated_prelaunch_blocked_projection() {
        let explicit_task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let current_task =
            owned_task_record("agent-mode-dev-team-test-first-operating-model", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "agent-mode-dev-team-test-first-operating-model",
            "agent-mode-dev-team-test-first-operating-model",
            "dispatch_prelaunch_blocked",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("unrelated prelaunch-blocked projection should not override live explicit binding")
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_binding_over_unrelated_ready_candidates() {
        let mut explicit_task =
            owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        explicit_task.status = "open".to_string();
        let mut ready_task = owned_task_record(
            "agent-mode-defect-model-not-pinned-after-dispatch-init",
            vec![],
        );
        ready_task.status = "open".to_string();
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let ready = vec![super::task_continuation_candidate(&ready_task, false)];

        let receipt =
            task_next_lawful_receipt(&[explicit_task, ready_task], ready, Some(&explicit));

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "taskflow-case-18-rollout-regression-gate"
        );
        assert_eq!(
            receipt.binding_source,
            Some("explicit_continuation_bind_task".to_string())
        );
    }

    #[test]
    fn task_next_lawful_blocks_paused_runtime_binding_with_concrete_resume_action() {
        let mut explicit_task =
            owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        explicit_task.status = "paused".to_string();
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[explicit_task], Vec::new(), Some(&explicit));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_paused"]);
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains(
                "vida task update taskflow-case-18-rollout-regression-gate --status in_progress --json",
            ) && action.contains("vida taskflow continuation bind codebase-audit-runtime-helper-dedup-refactor --task-id <task-id> --json")
        }));
    }

    #[test]
    fn task_next_lawful_prefers_current_binding_over_stale_closed_explicit_binding() {
        let mut stale_task = owned_task_record("stale-task", vec![]);
        stale_task.status = "closed".to_string();
        let current_task = owned_task_record("current-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "stale-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[stale_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("stale closed explicit binding should yield to current binding")
        .expect("current binding should select");

        assert_eq!(selected.task_id, "current-task");
        assert_eq!(
            selected.binding_source,
            "consume_continue_after_downstream_chain"
        );
    }

    #[test]
    fn task_next_lawful_prefers_current_binding_over_historical_task_close_reconcile() {
        let mut closed_task = owned_task_record("closed-task", vec![]);
        closed_task.status = "closed".to_string();
        let current_task = owned_task_record("current-task", vec![]);
        let mut explicit = test_continuation_binding(
            "old-run",
            "closed-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );
        explicit.active_bounded_unit = serde_json::json!({
            "kind": "downstream_dispatch_target",
            "task_id": "closed-task",
            "run_id": "old-run",
            "dispatch_target": "closure",
        });
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[closed_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("historical task-close reconcile should yield to current latest-run binding")
        .expect("current binding should select");

        assert_eq!(selected.task_id, "current-task");
        assert_eq!(
            selected.binding_source,
            "consume_continue_after_downstream_chain"
        );
    }

    #[test]
    fn task_next_lawful_blocks_open_explicit_and_current_source_drift() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let current_task = owned_task_record("current-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect_err("open explicit/current disagreement should fail closed");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["continuation_source_drift"]);
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("consume_continue_after_downstream_chain")));
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains("vida taskflow recovery status current-run --json")
        }));
    }

    #[test]
    fn task_next_lawful_prefers_newer_explicit_task_override_over_current_run_binding() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let current_task = owned_task_record("current-task", vec![]);
        let mut explicit = test_continuation_binding(
            "parent-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        explicit.recorded_at = "2026-05-22T21:48:46Z".to_string();
        let mut current = test_continuation_binding(
            "current-run",
            "current-task",
            "explicit_continuation_bind",
            "task_graph_task",
        );
        current.recorded_at = "2026-05-22T21:47:08Z".to_string();

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("newer explicit task binding should override prior current-run binding")
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "explicit-task");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_ignores_both_stale_source_drift_bindings() {
        let explicit = test_continuation_binding(
            "old-run",
            "missing-explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&explicit), Some(&current))
            .expect("stale explicit/current disagreement should defer to TaskFlow selection");

        assert!(selected.is_none());
    }

    #[test]
    fn task_next_lawful_selects_live_explicit_over_missing_current_binding() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected =
            select_task_next_lawful_binding(&[explicit_task], Some(&explicit), Some(&current))
                .expect("live explicit binding should win over stale current binding")
                .expect("explicit binding should select");

        assert_eq!(selected.task_id, "explicit-task");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_keeps_downstream_dispatch_target_live_during_source_drift() {
        let explicit = test_continuation_binding(
            "old-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&explicit), Some(&current))
            .expect("downstream dispatch target should remain live without an open task")
            .expect("explicit downstream dispatch target should select");

        assert_eq!(selected.task_id, "closed-feature-task");
        assert_eq!(selected.binding_source, "task_close_reconcile");
    }

    #[test]
    fn task_next_lawful_allows_downstream_dispatch_target_from_current_binding() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );

        let receipt = task_next_lawful_receipt(&[closed_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(receipt.binding_source, None);
        assert!(receipt.ready_task_candidates.is_empty());
    }

    #[test]
    fn task_next_lawful_allows_active_task_over_closed_downstream_dispatch_target() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let active_task = owned_task_record("live-active-task", vec![]);
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );

        let receipt =
            task_next_lawful_receipt(&[closed_task, active_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "live-active-task");
        assert_eq!(
            receipt.why_this_unit,
            "Single TaskFlow in_progress task is the authoritative active bounded unit."
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_prefers_single_active_task_over_other_ready_candidates() {
        let active_task = owned_task_record("live-active-task", vec![]);
        let mut ready_task = owned_task_record("other-ready-task", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, false)];

        let receipt = task_next_lawful_receipt(&[active_task, ready_task], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "live-active-task");
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_taskflow_active"
        );
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(receipt.ready_task_candidates.len(), 1);
    }

    #[test]
    fn task_next_lawful_blocks_closed_run_graph_binding_with_concrete_recovery_action() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[closed_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_closed"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run --json")
                && action.contains("closed-feature-task")
        }));
    }

    #[test]
    fn task_next_lawful_blocks_closed_run_graph_binding_before_ready_ambiguity() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let mut first = owned_task_record("ready-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("ready-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            super::task_continuation_candidate(&first, false),
            super::task_continuation_candidate(&second, false),
        ];
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt =
            task_next_lawful_receipt(&[closed_task, first, second], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_closed"]);
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "closed-feature-task"
        );
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run --json")
                && action.contains("closed-feature-task")
        }));
    }

    #[test]
    fn task_next_lawful_blocks_missing_run_graph_binding_with_concrete_recovery_action() {
        let binding = test_continuation_binding(
            "current-run",
            "missing-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_missing"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run --json")
                && action.contains(
                    "vida taskflow continuation bind current-run --task-id <task-id> --json",
                )
                && action.contains("missing-feature-task")
        }));
    }

    #[test]
    fn task_next_lawful_prefers_single_active_task_over_missing_runtime_binding() {
        let active_task = owned_task_record("authoritative-active-task", vec![]);
        let binding = test_continuation_binding(
            "stale-run",
            "missing-feature-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[active_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "authoritative-active-task"
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
    }

    #[test]
    fn task_next_lawful_blocks_open_delegated_cycle_binding() {
        let runtime_task = owned_task_record("running-runtime-task", vec![]);
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "open".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: None,
                lifecycle_stage: "analysis_active".to_string(),
            },
        };

        assert!(runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            None
        ));
        let receipt = blocked_task_next_lawful_receipt(
            binding.active_bounded_unit.clone(),
            Vec::new(),
            "open_delegated_cycle",
            &runtime_binding_open_delegated_cycle_next_action(&binding),
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["open_delegated_cycle"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida lane show running-run --json")
                && action.contains("vida taskflow recovery status running-run --json")
        }));
        assert_eq!(
            task_next_lawful_receipt(&[runtime_task], Vec::new(), Some(&binding)).status,
            "pass",
            "baseline receipt still represents the raw binding; command-level recovery gate blocks it"
        );
    }

    #[test]
    fn task_next_lawful_allows_ready_downstream_handoff_despite_open_cycle_gate() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: Some("writer".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "analysis_active".to_string(),
            },
        };
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "running-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        };

        assert!(!runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&dispatch)
        ));
        let receipt = pass_ready_downstream_handoff_task_next_lawful_receipt(
            &binding,
            Vec::new(),
            None,
            None,
        );

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains("vida taskflow consume continue --run-id running-run --json")
        }));
    }

    #[test]
    fn task_next_lawful_allows_completed_lane_despite_stale_open_cycle_gate() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution_started",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_active".to_string(),
            },
        };
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "running-run".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-26T01:00:00Z".to_string(),
        };

        assert!(!runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&dispatch)
        ));
        let receipt = pass_completed_lane_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_completed_dispatch_receipt")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_completed_lane_reconciled"
        );
    }

    #[test]
    fn task_next_lawful_does_not_allow_unrelated_ready_downstream_handoff() {
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: Some("writer".to_string()),
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "open".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "analysis_active".to_string(),
            },
        };
        let unrelated_dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "newer-unrelated-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        };

        assert!(runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&unrelated_dispatch)
        ));
    }

    #[test]
    fn task_next_lawful_uses_downstream_execute_command_after_terminal_ready_downstream_handoff() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution",
            "run_graph_task",
        );

        let receipt = pass_ready_downstream_handoff_task_next_lawful_receipt(
            &binding,
            Vec::new(),
            Some("running-run"),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json"),
        );

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt
            .next_action
            .as_deref()
            .is_some_and(|action| action.contains(
                "vida agent-init --downstream-packet packet.json --execute-dispatch --json"
            )));
    }

    #[test]
    fn task_next_lawful_surfaces_exception_takeover_binding_source() {
        let runtime_task = owned_task_record("exception-task", vec![]);
        let mut binding = test_continuation_binding(
            "exception-run",
            "exception-task",
            "latest_run_graph_exception_takeover_dispatch",
            "run_graph_task",
        );
        binding.active_bounded_unit = serde_json::json!({
            "active_node": "specification",
            "kind": "run_graph_task",
            "run_id": "exception-run",
            "task_id": "exception-task",
        });

        let receipt = task_next_lawful_receipt(&[runtime_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["active_node"], "specification");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
    }

    #[test]
    fn task_next_lawful_exception_takeover_bypasses_open_cycle_blocker() {
        let binding = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("case18-previous-takeover".to_string()),
            exception_path_receipt_id: Some("case18-current-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-21T12:28:00Z".to_string(),
        };

        assert!(runtime_binding_has_active_exception_takeover(
            &binding,
            Some(&dispatch)
        ));
        let receipt = pass_exception_takeover_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_exception_takeover"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_accepts_active_exception_takeover_with_recorded_lane_status() {
        let binding = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: Some("case18-supersession-evidence".to_string()),
            exception_path_receipt_id: Some("case18-exception-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["configured_backend_dispatch_failed".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-21T14:47:00Z".to_string(),
        };

        assert!(runtime_binding_has_active_exception_takeover(
            &binding,
            Some(&dispatch)
        ));
        let receipt = pass_exception_takeover_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_exception_takeover"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_command_runs_with_single_ready_task() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-ready",
                "Ready task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let code = runtime.block_on(crate::run(cli(&[
            "task",
            "next-lawful",
            "--state-dir",
            harness.path().to_str().expect("state path should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn task_ready_prefers_authoritative_store() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-ready",
                "Ready task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
            let snapshot_path =
                crate::StateStore::canonical_task_snapshot_path_for_state_root(harness.path());
            fs::write(&snapshot_path, "").expect("snapshot should be writable");
            drop(store);

            let (tasks, metadata) =
                task_ready_authoritative_first(harness.path().to_path_buf(), None)
                    .await
                    .expect("ready tasks should load from authoritative store");

            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].id, "task-ready");
            assert_eq!(metadata.mode, "authoritative_live");
            assert!(!metadata.degraded);
            assert!(metadata.snapshot_path.is_none());
        });
    }

    #[test]
    fn task_critical_path_prefers_authoritative_store() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "critical-ready",
                "Critical ready",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
            let snapshot_path =
                crate::StateStore::canonical_task_snapshot_path_for_state_root(harness.path());
            fs::write(&snapshot_path, "").expect("snapshot should be writable");
            drop(store);

            let path = task_critical_path_snapshot_first(harness.path().to_path_buf())
                .await
                .expect("critical path should load from authoritative store");

            assert_eq!(path.length, 1);
            assert_eq!(path.root_task_id.as_deref(), Some("critical-ready"));
            assert_eq!(path.terminal_task_id.as_deref(), Some("critical-ready"));
        });
    }

    #[test]
    fn task_next_lawful_command_selects_ready_task_over_closed_downstream_closure_marker() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "taskflow-case-11-actual-agent-autonomy",
                "Actual ready candidate",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "taskflow-defect-case-11-closed-downstream-binding-blocks-ready",
                "Closed downstream marker",
                "task",
                "closed",
                1,
                Some("parent-epic"),
            )
            .await;
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "run-closed-downstream-marker".to_string(),
                    task_id: "taskflow-defect-case-11-closed-downstream-binding-blocks-ready"
                        .to_string(),
                    task_class: "worker".to_string(),
                    active_node: "closure".to_string(),
                    next_node: None,
                    status: "ready".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "taskflow_state_store".to_string(),
                    lane_id: "closure_lane".to_string(),
                    lifecycle_stage: "closure_active".to_string(),
                    policy_gate: "not_required".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "execution_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: true,
                })
                .await
                .expect("run graph status should record");
            let binding = test_continuation_binding(
                "run-closed-downstream-marker",
                "taskflow-defect-case-11-closed-downstream-binding-blocks-ready",
                "task_close_reconcile",
                "downstream_dispatch_target",
            );
            store
                .record_run_graph_continuation_binding(&binding)
                .await
                .expect("continuation binding should record");
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let code = runtime.block_on(crate::run(cli(&[
            "task",
            "next-lawful",
            "--state-dir",
            harness.path().to_str().expect("state path should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
        let projection_path = harness
            .path()
            .join("operator-projections")
            .join("task-next-lawful-latest.json");
        let projection: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(projection_path).expect("next-lawful projection should be written"),
        )
        .expect("next-lawful projection should parse");
        assert_eq!(projection["status"], task_json_success_status());
        assert_eq!(
            projection["active_bounded_unit"]["task_id"],
            "taskflow-case-11-actual-agent-autonomy"
        );
        assert_eq!(projection["binding_source"], serde_json::Value::Null);
        assert!(projection["blocker_codes"]
            .as_array()
            .expect("blockers should be an array")
            .is_empty());
    }

    #[test]
    fn task_create_title_resolves_positional_or_title_option() {
        assert_eq!(
            task_create_title(&minimal_task_create_args(Some("Positional title"), None))
                .expect("positional title should resolve"),
            "Positional title"
        );
        assert_eq!(
            task_create_title(&minimal_task_create_args(None, Some("Flag title")))
                .expect("--title should resolve"),
            "Flag title"
        );
    }

    #[test]
    fn task_create_title_rejects_missing_or_duplicate_sources() {
        let missing = task_create_title(&minimal_task_create_args(None, None))
            .expect_err("missing title should fail");
        assert!(missing.contains("Missing task title"));

        let duplicate = task_create_title(&minimal_task_create_args(Some("A"), Some("B")))
            .expect_err("duplicate title sources should fail");
        assert!(duplicate.contains("only one task title source"));
    }

    #[test]
    fn task_close_feedback_skips_isolated_explicit_state_dir() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path().join("project");
        fs::create_dir_all(project_root.join(".vida/state"))
            .expect("project state directory should initialize");
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        let isolated_state_dir = harness.path().join("isolated-state");
        let task_value = serde_json::json!({
            "id": "audit-p1-task-close-state-dir-feedback-isolation",
            "status": "closed",
        });

        assert!(task_close_uses_isolated_state_dir(
            &isolated_state_dir,
            true
        ));
        let telemetry = task_close_host_agent_telemetry(
            &isolated_state_dir,
            true,
            Some(&project_root),
            &task_value,
            "closed with isolated temp state",
            "vida task close",
        );

        assert_eq!(telemetry["status"], "skipped");
        assert_eq!(telemetry["reason"], "isolated_state_dir");
        assert_eq!(
            telemetry["state_dir"],
            isolated_state_dir.display().to_string()
        );
        assert_eq!(telemetry["feedback_store"], "not_recorded");
        assert!(!project_root
            .join(crate::HOST_AGENT_OBSERVABILITY_STATE)
            .exists());
        assert!(!project_root.join(crate::WORKER_STRATEGY_STATE).exists());
    }

    #[test]
    fn task_close_feedback_keeps_project_state_dir_admissible() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path();
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker directory should initialize");
        let project_state_dir = project_root.join(crate::state_store::default_state_dir());

        assert!(!task_close_uses_isolated_state_dir(
            &project_state_dir,
            true
        ));
    }

    #[test]
    fn task_close_feedback_keeps_noncanonical_feedback_recorded_by_default() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path();
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker directory should initialize");
        let task_value = serde_json::json!({
            "id": "audit-p1-fast-task-close-feedback",
            "status": "closed",
        });
        let telemetry = task_close_host_agent_telemetry(
            &project_root.join(crate::state_store::default_state_dir()),
            false,
            Some(project_root),
            &task_value,
            "fixed_by_commit_abc_tests_pass",
            "vida task close",
        );

        assert_eq!(telemetry["status"], "recorded");
        assert_eq!(
            telemetry["feedback"]["mode"],
            "lightweight_task_close_feedback"
        );
    }

    #[test]
    fn task_close_feedback_blocker_summary_surfaces_deferred_canonical_close() {
        let telemetry = serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "canonical_status": "blocked",
            "canonical_gate": "blocked",
        });

        let (blocker_codes, next_actions) = task_close_feedback_blocker_summary(&telemetry)
            .expect("deferred canonical close should produce blocker summary");

        assert_eq!(
            blocker_codes,
            vec![
                "close_feedback_canonical_status_blocked".to_string(),
                "canonical_gate_blocked".to_string()
            ]
        );
        assert!(next_actions[0].contains("Resolve the blocked condition"));
    }

    #[test]
    fn task_close_feedback_blocker_summary_ignores_historical_blocker_proof_context() {
        let reason = "Closed after proof: previous task close JSON returned close_feedback_canonical_status_blocked/canonical_gate_blocked as historical blocker context; proof passed.";
        let telemetry = task_close_host_agent_telemetry(
            std::path::Path::new(".vida/data/state"),
            false,
            None,
            &serde_json::json!({"id": "task-close-feedback-regression"}),
            reason,
            "test",
        );

        assert_ne!(
            telemetry["reason"],
            "feedback_deferred_for_canonical_close_status"
        );
        assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
    }

    #[test]
    fn task_close_commit_automation_requires_explicit_owned_files() {
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "audit-p1-task-close-release-options".to_string(),
                reason: "close bounded task".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
                stage_owned: false,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["dirty_ownership_ambiguous"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["dirty_ownership_ambiguous"]);
    }

    #[test]
    fn task_close_push_automation_requires_explicit_commit() {
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "audit-p1-task-close-release-options".to_string(),
                reason: "close bounded task".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: true,
                include_global_progress: false,
                stage_owned: false,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["push_requires_commit"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["push_requires_commit"]);
    }

    #[test]
    fn task_json_success_status_defaults_to_release_contract_vocabulary() {
        assert_eq!(task_json_success_status(), "pass");
    }

    #[test]
    fn normalize_task_json_contract_arrays_fail_closed_for_whitespace_only_entries() {
        let mut summary_json = serde_json::json!({
            "status": task_json_success_status(),
            "blocker_codes": ["   "],
            "next_actions": ["Run `vida task import-jsonl --json`"],
        });

        assert!(normalize_task_json_contract_arrays(&mut summary_json).is_err());
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["pending"])),
            Some(vec!["pending".to_string()])
        );
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }

    #[test]
    fn parse_label_values_accepts_repeated_and_comma_separated_forms() {
        let labels = parse_label_values(&[
            "alpha,beta".to_string(),
            " gamma ".to_string(),
            "delta, ,epsilon".to_string(),
        ]);
        assert_eq!(labels, vec!["alpha", "beta", "gamma", "delta", "epsilon"]);
    }

    #[test]
    fn proof_target_values_split_multi_filter_cargo_test_commands() {
        let proof_targets = parse_proof_target_values(&[
            "cargo test -p vida work_item_taxonomy operator_contracts development_flow_catalog -- --nocapture --test-threads=1".to_string(),
        ]);

        assert_eq!(
            proof_targets,
            vec![
                "cargo test -p vida work_item_taxonomy -- --nocapture --test-threads=1",
                "cargo test -p vida operator_contracts -- --nocapture --test-threads=1",
                "cargo test -p vida development_flow_catalog -- --nocapture --test-threads=1",
            ]
        );
    }

    #[test]
    fn proof_target_values_normalize_stale_diagnostics_and_docflow_flags() {
        let proof_targets = parse_proof_target_values(&[
            "vida diagnostics --json".to_string(),
            "vida docflow protocol-coverage-check --profile active-canon --format jsonl"
                .to_string(),
        ]);

        assert_eq!(
            proof_targets,
            vec![
                "vida diagnostics post-commit --json",
                "vida docflow protocol-coverage-check --profile active-canon",
            ]
        );
    }

    #[test]
    fn parse_optional_label_value_returns_none_for_absent_input() {
        assert_eq!(parse_optional_label_value(None), None);
        assert_eq!(
            parse_optional_label_value(Some("alpha, beta")),
            Some(vec!["alpha".to_string(), "beta".to_string()])
        );
    }

    #[test]
    fn adaptive_replan_finding_input_accepts_supported_finding_kinds() {
        for finding_kind in ADAPTIVE_REPLAN_FINDING_KINDS {
            let parsed = parse_adaptive_replan_finding_input(&serde_json::json!({
                "finding_kind": finding_kind,
                "source_task_id": "task-a",
                "summary": "bounded finding summary",
                "evidence_refs": ["receipt-b", " receipt-a ", "receipt-a"]
            }))
            .expect("supported finding kind should parse");

            assert_eq!(parsed.schema_version, "1");
            assert_eq!(parsed.input_kind, "adaptive_replan_finding_input");
            assert_eq!(parsed.finding_kind, *finding_kind);
            assert_eq!(parsed.source_task_id, "task-a");
            assert_eq!(
                parsed.evidence_refs,
                vec!["receipt-a".to_string(), "receipt-b".to_string()]
            );
            assert_eq!(parsed.operator_truth["parsing_and_validation_only"], true);
            assert_eq!(
                parsed.operator_truth["adaptive_mutation_execution_loop_implemented"],
                false
            );
            assert_eq!(
                parsed.operator_truth["adaptive_mutation_execution_loop_truth"],
                "not_implemented_in_this_slice"
            );
            assert_eq!(
                parsed.operator_truth["valid_input_does_not_mutate_task_graph"],
                true
            );
        }
    }

    #[test]
    fn adaptive_replan_finding_input_rejects_unsupported_kind() {
        let error = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "general_comment",
            "source_task_id": "task-a",
            "summary": "not actionable"
        }))
        .expect_err("unsupported finding kind should fail closed");

        assert_eq!(error.status, "blocked");
        assert_eq!(
            error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert_eq!(error.field.as_deref(), Some("finding_kind"));
        assert!(error
            .supported_finding_kinds
            .iter()
            .any(|kind| kind == "verification_finding"));
        assert_eq!(error.operator_truth["parsing_and_validation_only"], true);
    }

    #[test]
    fn adaptive_replan_finding_input_rejects_invalid_required_fields() {
        let missing_summary = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "proof_gap",
            "source_task_id": "task-a",
            "summary": "   "
        }))
        .expect_err("blank summary should fail closed");
        assert_eq!(missing_summary.field.as_deref(), Some("summary"));
        assert!(missing_summary.reason.contains("non-empty string"));

        let invalid_evidence = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "oversized_task",
            "source_task_id": "task-a",
            "summary": "task is too broad",
            "evidence_refs": ["ok", ""]
        }))
        .expect_err("blank evidence ref should fail closed");
        assert_eq!(invalid_evidence.field.as_deref(), Some("evidence_refs"));
        assert!(invalid_evidence.reason.contains("entries"));
    }

    #[test]
    fn adaptive_replan_finding_preview_maps_supported_kinds_without_mutation() {
        let cases = [
            (
                "verification_finding",
                "blocker_resolution",
                "spawn_blocker_task",
            ),
            ("proof_gap", "blocker_resolution", "spawn_blocker_task"),
            ("scope_drift", "scope_replan", "replan_scope_review"),
            ("oversized_task", "task_decomposition", "split_task"),
        ];

        for (finding_kind, expected_category, expected_kind) in cases {
            let preview = build_adaptive_replan_finding_preview(
                &serde_json::json!({
                    "finding_kind": finding_kind,
                    "source_task_id": "task-a",
                    "summary": "bounded adaptive replanner input",
                    "evidence_refs": ["receipt-a", "receipt-a", "receipt-b"]
                }),
                "vida task adaptive-preview",
            )
            .expect("supported finding kind should preview");

            assert_eq!(preview.status, task_json_success_status());
            assert_eq!(preview.planned_mutation_category, expected_category);
            assert_eq!(preview.planned_mutation_kind, expected_kind);
            assert_eq!(preview.source_task_id, "task-a");
            assert!(preview.dry_run);
            assert!(!preview.applied);
            assert_eq!(
                preview.finding.evidence_refs,
                vec!["receipt-a", "receipt-b"]
            );
            assert_eq!(preview.operator_truth["graph_state_opened"], false);
            assert_eq!(preview.operator_truth["graph_state_mutated"], false);
            assert_eq!(
                preview.operator_truth["adaptive_mutation_execution_loop_implemented"],
                false
            );
            assert_eq!(
                preview.preview_receipt.receipt_kind,
                "adaptive_replan_finding_preview_receipt"
            );
            assert_eq!(preview.preview_receipt.schema_version, "1");
            assert_eq!(
                preview.preview_receipt.receipt_id,
                format!(
                    "adaptive-replan-preview:task-a:{finding_kind}:{expected_category}:{expected_kind}:evidence=receipt-a+receipt-b"
                )
            );
            assert_eq!(preview.preview_receipt.source_task_id, "task-a");
            assert_eq!(preview.preview_receipt.finding_kind, finding_kind);
            assert_eq!(
                preview.preview_receipt.planned_mutation_category,
                expected_category
            );
            assert_eq!(preview.preview_receipt.planned_mutation_kind, expected_kind);
            assert!(preview.preview_receipt.dry_run);
            assert!(!preview.preview_receipt.applied);
            assert!(!preview.preview_receipt.graph_state_opened);
            assert!(!preview.preview_receipt.graph_state_mutated);
            assert_eq!(
                preview.preview_receipt.operator_truth["preview_receipt_emitted"],
                true
            );
        }
    }

    #[test]
    fn adaptive_replan_finding_preview_receipt_is_stable_without_evidence() {
        let preview = build_adaptive_replan_finding_preview(
            &serde_json::json!({
                "finding_kind": "oversized_task",
                "source_task_id": "task-b",
                "summary": "task is too broad"
            }),
            "vida task adaptive-preview",
        )
        .expect("valid finding should preview");

        assert_eq!(
            preview.preview_receipt.receipt_id,
            "adaptive-replan-preview:task-b:oversized_task:task_decomposition:split_task:evidence=none"
        );
        assert_eq!(
            preview.preview_receipt.surface,
            "vida task adaptive-preview"
        );
        assert_eq!(preview.preview_receipt.schema_version, "1");
        assert_eq!(preview.preview_receipt.planned_mutation_kind, "split_task");
        assert_eq!(
            preview.preview_receipt.planned_mutation_category,
            "task_decomposition"
        );
        assert!(!preview.preview_receipt.graph_state_mutated);
    }

    #[test]
    fn adaptive_replan_finding_preview_rejects_invalid_input() {
        let error = build_adaptive_replan_finding_preview(
            &serde_json::json!({
                "finding_kind": "general_comment",
                "source_task_id": "task-a",
                "summary": "not actionable"
            }),
            "vida task adaptive-preview",
        )
        .expect_err("unsupported finding kind should fail closed");

        assert_eq!(error.status, "blocked");
        assert_eq!(error.field.as_deref(), Some("finding_kind"));
        assert_eq!(
            error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
    }

    #[test]
    fn task_adaptive_preview_command_accepts_inline_json_without_state_store() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: Some(
                        serde_json::json!({
                            "finding_kind": "oversized_task",
                            "source_task_id": "task-a",
                            "summary": "task is too broad"
                        })
                        .to_string(),
                    ),
                    finding_file: None,
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn task_adaptive_preview_command_accepts_finding_file_without_state_store() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let finding_path = harness.path().join("adaptive-finding.json");
        fs::write(
            &finding_path,
            serde_json::json!({
                "finding_kind": "proof_gap",
                "source_task_id": "task-a",
                "summary": "proof artifact missing",
                "evidence_refs": ["receipt-b", "receipt-a"]
            })
            .to_string(),
        )
        .expect("finding file should write");

        let loaded = load_adaptive_preview_finding_json(None, Some(finding_path.as_path()))
            .expect("finding file input should load");
        let preview = build_adaptive_replan_finding_preview(&loaded, "vida task adaptive-preview")
            .expect("finding file input should preview");
        assert_eq!(preview.planned_mutation_category, "blocker_resolution");
        assert_eq!(preview.planned_mutation_kind, "spawn_blocker_task");
        assert_eq!(
            preview.preview_receipt.receipt_id,
            "adaptive-replan-preview:task-a:proof_gap:blocker_resolution:spawn_blocker_task:evidence=receipt-a+receipt-b"
        );
        assert_eq!(preview.operator_truth["preview_receipt_emitted"], true);
        assert_eq!(preview.operator_truth["graph_state_mutated"], false);

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: None,
                    finding_file: Some(finding_path),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn adaptive_preview_finding_file_input_fails_closed_for_missing_or_invalid_file() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let missing_path = harness.path().join("missing-finding.json");
        let missing_error = load_adaptive_preview_finding_json(None, Some(missing_path.as_path()))
            .expect_err("missing finding file should fail closed");
        assert_eq!(missing_error.status, "blocked");
        assert_eq!(missing_error.field.as_deref(), Some("finding_file"));
        assert_eq!(
            missing_error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert_eq!(
            missing_error.operator_truth["valid_input_does_not_mutate_task_graph"],
            true
        );

        let invalid_path = harness.path().join("invalid-finding.json");
        fs::write(&invalid_path, "{not-json").expect("invalid finding file should write");
        let invalid_error = load_adaptive_preview_finding_json(None, Some(invalid_path.as_path()))
            .expect_err("invalid finding file should fail closed");
        assert_eq!(invalid_error.status, "blocked");
        assert_eq!(invalid_error.field.as_deref(), Some("finding_file"));
        assert_eq!(
            invalid_error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert!(invalid_error.reason.contains("valid JSON"));

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: None,
                    finding_file: Some(missing_path),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(2)
        );
    }

    #[test]
    #[ignore = "covered by binary integration smoke; in-process sequential SurrealKv opens keep the lock longer than this unit test assumes"]
    fn task_command_round_trip_succeeds() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let jsonl_path = harness.path().join("issues.jsonl");
        fs::write(
            &jsonl_path,
            concat!(
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write sample task jsonl");

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "import-jsonl",
                    jsonl_path.to_str().expect("jsonl path should render"),
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "list",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "ready",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );
    }

    #[test]
    fn task_split_command_creates_children_and_blocks_source_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "dep-task",
                "Dependency",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .add_task_dependency("source-task", "dep-task", "depends-on", "test")
                .await
                .expect("dependency should create");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Split(crate::TaskSplitArgs {
                    task_id: "source-task".to_string(),
                    children: vec![
                        "source-task-a:First slice".to_string(),
                        "source-task-b:Second slice".to_string(),
                    ],
                    reason: "oversized task".to_string(),
                    dry_run: false,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            assert!(source.dependencies.iter().any(|dependency| {
                dependency.issue_id == "source-task"
                    && dependency.depends_on_id == "source-task-b"
                    && dependency.edge_type == "depends-on"
            }));

            let first_child = store
                .show_task("source-task-a")
                .await
                .expect("first split child should load");
            assert_eq!(
                first_child.description,
                "Split from `source-task`: oversized task"
            );
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task" && dependency.edge_type == "parent-child"
            }));
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "dep-task" && dependency.edge_type == "depends-on"
            }));

            let second_child = store
                .show_task("source-task-b")
                .await
                .expect("second split child should load");
            assert!(second_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task-a" && dependency.edge_type == "depends-on"
            }));
        });
    }

    #[test]
    fn task_split_command_reopens_closed_source_with_new_children() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "sibling-task",
                "Sibling",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("source-task", "completed")
                .await
                .expect("source task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Split(crate::TaskSplitArgs {
                    task_id: "source-task".to_string(),
                    children: vec![
                        "source-task-a:First reopened slice".to_string(),
                        "source-task-b:Second reopened slice".to_string(),
                    ],
                    reason: "new work found after closure".to_string(),
                    dry_run: false,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            assert_eq!(source.status, "in_progress");
            assert!(source.closed_at.is_none());
            assert!(source.close_reason.is_none());

            let first_child = store
                .show_task("source-task-a")
                .await
                .expect("first split child should load");
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task" && dependency.edge_type == "parent-child"
            }));
            assert!(store
                .validate_task_graph()
                .await
                .expect("validate")
                .is_empty());
        });
    }

    #[test]
    fn task_close_child_does_not_auto_close_parent_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "parent-task",
                "Parent",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "child-todo",
                "Child TODO",
                "todo",
                "in_progress",
                2,
                Some("parent-task"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Close(crate::TaskCloseArgs {
                    task_id: "child-todo".to_string(),
                    reason: "implementation proof passed".to_string(),
                    source: Some("task_close_child_regression".to_string()),
                    release: false,
                    install: false,
                    install_target: "current".to_string(),
                    skip_release_build: false,
                    source_binary: None,
                    install_root: None,
                    commit: false,
                    push: false,
                    include_global_progress: false,
                    stage_owned: false,
                    commit_files: vec![],
                    commit_message: None,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let parent = store
                .show_task("parent-task")
                .await
                .expect("parent task should still exist");
            let child = store
                .show_task("child-todo")
                .await
                .expect("child task should still exist");

            assert_eq!(child.status, "closed");
            assert_eq!(
                parent.status, "open",
                "closing a child TODO must not implicitly close its parent task"
            );
        });
    }

    #[test]
    fn task_close_returns_failure_when_requested_automation_is_blocked() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "root-epic", "Root epic", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "close-with-push-blocked",
                "Close with blocked push",
                "task",
                "in_progress",
                2,
                Some("root-epic"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Close(crate::TaskCloseArgs {
                    task_id: "close-with-push-blocked".to_string(),
                    reason: "implementation proof passed".to_string(),
                    source: Some("task_close_automation_regression".to_string()),
                    release: false,
                    install: false,
                    install_target: "current".to_string(),
                    skip_release_build: false,
                    source_binary: None,
                    install_root: None,
                    commit: false,
                    push: true,
                    include_global_progress: false,
                    stage_owned: false,
                    commit_files: vec![],
                    commit_message: None,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("close-with-push-blocked")
                .await
                .expect("task should still close even when automation is blocked");
            assert_eq!(task.status, "closed");
        });
    }

    #[test]
    fn task_spawn_blocker_command_creates_blocker_and_links_source() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "epic-root", "Epic", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "in_progress",
                2,
                Some("epic-root"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::SpawnBlocker(crate::TaskSpawnBlockerArgs {
                    task_id: "source-task".to_string(),
                    blocker_task_id: "blocker-task".to_string(),
                    title: "Blocker title".to_string(),
                    reason: "new dependency discovered".to_string(),
                    description: None,
                    issue_type: "task".to_string(),
                    status: "open".to_string(),
                    priority: None,
                    labels: Vec::new(),
                    dry_run: false,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            assert!(source.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "blocker-task" && dependency.edge_type == "blocks"
            }));

            let blocker = store
                .show_task("blocker-task")
                .await
                .expect("blocker task should load");
            assert_eq!(blocker.priority, 2);
            assert_eq!(
                blocker.description,
                "Blocker for `source-task`: new dependency discovered"
            );
            assert!(blocker.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "epic-root" && dependency.edge_type == "parent-child"
            }));
        });
    }

    #[test]
    fn split_preview_includes_first_class_graph_mutation_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "dep-task",
                "Dependency",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .add_task_dependency("source-task", "dep-task", "depends-on", "test")
                .await
                .expect("dependency should create");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            let rows = store.all_tasks().await.expect("task rows should load");
            let child_specs = parse_split_child_specs(&[
                "source-task-a:First slice".to_string(),
                "source-task-b:Second slice".to_string(),
            ])
            .expect("child specs should parse");

            let (result, _simulated_rows) = build_split_mutation_preview(
                &rows,
                &source,
                &child_specs,
                "oversized task",
                "vida task split",
                false,
            )
            .expect("split preview should build");

            let receipt = &result.graph_mutation_receipt;
            assert_eq!(receipt.receipt_kind, "task_graph_mutation_receipt");
            assert_eq!(receipt.schema_version, "1");
            assert_eq!(receipt.mutation_kind, "split_task");
            assert_eq!(receipt.source_task_id, "source-task");
            assert_eq!(receipt.dry_run, false);
            assert_eq!(receipt.applied, true);
            assert_eq!(receipt.before_validation.status, "pass");
            assert_eq!(receipt.after_validation.status, "pass");
            assert_eq!(receipt.before_task_count, rows.len());
            assert_eq!(receipt.after_task_count, rows.len() + 2);
            assert_eq!(
                receipt.planned_task_ids,
                vec!["source-task-a".to_string(), "source-task-b".to_string()]
            );
            assert_eq!(
                receipt.operator_truth["adaptive_replanner_loop_implemented"],
                false
            );
            assert_eq!(
                receipt.operator_truth["adaptive_replanner_loop_truth"],
                "not_implemented_in_this_slice"
            );
        });
    }

    #[test]
    fn spawn_blocker_preview_receipt_records_dry_run_truth() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            let rows = store.all_tasks().await.expect("task rows should load");
            let command = crate::TaskSpawnBlockerArgs {
                task_id: "source-task".to_string(),
                blocker_task_id: "blocker-task".to_string(),
                title: "Blocker title".to_string(),
                reason: "new dependency discovered".to_string(),
                description: None,
                issue_type: "task".to_string(),
                status: "open".to_string(),
                priority: None,
                labels: Vec::new(),
                dry_run: true,
                state_dir: Some(harness.path().to_path_buf()),
                render: crate::RenderMode::Plain,
                json: true,
            };

            let (result, _simulated_rows) =
                build_spawn_blocker_preview(&rows, &source, &command, "vida task spawn-blocker")
                    .expect("spawn blocker preview should build");

            let receipt = &result.graph_mutation_receipt;
            assert_eq!(receipt.receipt_kind, "task_graph_mutation_receipt");
            assert_eq!(receipt.mutation_kind, "spawn_blocker_task");
            assert_eq!(receipt.dry_run, true);
            assert_eq!(receipt.applied, false);
            assert_eq!(receipt.before_validation.status, "pass");
            assert_eq!(receipt.after_validation.status, "pass");
            assert_eq!(receipt.before_task_count, rows.len());
            assert_eq!(receipt.after_task_count, rows.len() + 1);
            assert_eq!(receipt.planned_task_ids, vec!["blocker-task".to_string()]);
            assert_eq!(
                receipt.planned_dependency_edges[0].reason,
                "spawn_blocker_dependency"
            );
            assert_eq!(
                receipt.operator_truth["records_before_after_validation"],
                true
            );
        });
    }

    #[test]
    fn taskflow_replan_split_defaults_to_dry_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(crate::taskflow_proxy::run_taskflow_proxy(
                crate::ProxyArgs {
                    args: vec![
                        "replan".to_string(),
                        "split".to_string(),
                        "source-task".to_string(),
                        "--child".to_string(),
                        "source-task-a:First slice".to_string(),
                        "--child".to_string(),
                        "source-task-b:Second slice".to_string(),
                        "--reason".to_string(),
                        "oversized task".to_string(),
                        "--state-dir".to_string(),
                        harness.path().display().to_string(),
                        "--json".to_string(),
                    ],
                }
            )),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            assert!(matches!(
                store.show_task("source-task-a").await,
                Err(crate::state_store::StateStoreError::MissingTask { .. })
            ));
            assert!(matches!(
                store.show_task("source-task-b").await,
                Err(crate::state_store::StateStoreError::MissingTask { .. })
            ));
        });
    }
}
