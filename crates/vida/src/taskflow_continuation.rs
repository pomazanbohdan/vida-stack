use std::process::ExitCode;

use time::format_description::well_known::Rfc3339;

use crate::{
    print_surface_header, print_surface_line,
    state_store::{RunGraphContinuationBinding, RunGraphStatus, StateStore, TaskRecord},
    taskflow_task_bridge::proxy_state_dir,
    RenderMode,
};

pub(crate) const CONSUME_CONTINUE_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE: &str =
    "consume_continue_after_downstream_chain";
pub(crate) const CONSUME_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE: &str =
    "consume_after_downstream_chain";
const CONTINUATION_BIND_SURFACE: &str = "vida taskflow continuation bind";

pub(crate) fn is_downstream_chain_continuation_binding_source(binding_source: &str) -> bool {
    matches!(
        binding_source,
        CONSUME_CONTINUE_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE
            | CONSUME_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE
    )
}

fn terminal_completed_without_next_unit(status: &RunGraphStatus) -> bool {
    status.status == "completed"
        && status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
}

fn explicit_task_bind_allowed_for_status(status: &RunGraphStatus, task_id: &str) -> bool {
    terminal_completed_without_next_unit(status) || status.task_id.trim() == task_id.trim()
}

fn args_request_json(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json")
}

fn continuation_bind_blocked_payload(
    run_id: Option<&str>,
    task_id: Option<&str>,
    error: &str,
    blocker_code: &str,
) -> serde_json::Value {
    let next_actions = vec![
        "Refresh run-graph and task evidence before recording an explicit continuation binding."
            .to_string(),
        "Preserve fail-closed binding semantics until the active bounded unit is unambiguous."
            .to_string(),
    ];
    let artifact_refs = serde_json::json!({
        "surface": CONTINUATION_BIND_SURFACE,
        "run_id": run_id,
        "task_id": task_id,
    });
    let blocker_codes = vec![blocker_code.to_string()];
    serde_json::json!({
        "surface": CONTINUATION_BIND_SURFACE,
        "status": "blocked",
        "error": error,
        "run_id": run_id,
        "task_id": task_id,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
    })
}

fn continuation_bind_success_payload(
    run_id: &str,
    binding: &RunGraphContinuationBinding,
) -> serde_json::Value {
    let next_actions: Vec<String> = Vec::new();
    let blocker_codes: Vec<String> = Vec::new();
    let artifact_refs = serde_json::json!({
        "surface": CONTINUATION_BIND_SURFACE,
        "run_id": run_id,
        "task_id": binding.task_id,
    });
    serde_json::json!({
        "surface": CONTINUATION_BIND_SURFACE,
        "status": "ok",
        "run_id": run_id,
        "binding": binding,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "shared_fields": {
            "status": "ok",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "ok",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
    })
}

fn emit_continuation_bind_error(
    as_json: bool,
    run_id: Option<&str>,
    task_id: Option<&str>,
    error: String,
    blocker_code: &str,
    exit_code: u8,
) -> ExitCode {
    if as_json {
        crate::print_json_pretty(&continuation_bind_blocked_payload(
            run_id,
            task_id,
            &error,
            blocker_code,
        ));
    } else {
        eprintln!("{error}");
    }
    ExitCode::from(exit_code)
}

fn run_graph_active_bounded_unit(status: &RunGraphStatus) -> Option<serde_json::Value> {
    if terminal_completed_without_next_unit(status) {
        return None;
    }
    if status.status == "completed" {
        let dispatch_target = status
            .next_node
            .as_deref()
            .filter(|value| !value.trim().is_empty())?;
        return Some(serde_json::json!({
            "kind": "downstream_dispatch_target",
            "task_id": status.task_id,
            "run_id": status.run_id,
            "dispatch_target": dispatch_target,
        }));
    }

    Some(serde_json::json!({
        "kind": "run_graph_task",
        "task_id": status.task_id,
        "run_id": status.run_id,
        "active_node": status.active_node,
    }))
}

pub(crate) fn continuation_posture_for_status(status: &RunGraphStatus) -> String {
    if status.delegation_gate().delegated_cycle_open {
        "sequential_only_open_cycle".to_string()
    } else {
        "sequential_only".to_string()
    }
}

pub(crate) fn build_run_graph_continuation_binding(
    status: &RunGraphStatus,
    request_text: Option<&str>,
    binding_source: &str,
    why_override: Option<&str>,
) -> Option<RunGraphContinuationBinding> {
    let active_bounded_unit = run_graph_active_bounded_unit(status)?;
    let why_this_unit = if let Some(why_override) = why_override {
        why_override.trim().to_string()
    } else if active_bounded_unit["kind"] == "downstream_dispatch_target" {
        format!(
            "Explicit continuation binding records downstream target `{}` as the next lawful bounded unit for run `{}`.",
            active_bounded_unit["dispatch_target"]
                .as_str()
                .unwrap_or("unknown"),
            status.run_id
        )
    } else {
        format!(
            "Explicit continuation binding records task `{}` at node `{}` as the active bounded unit.",
            status.task_id, status.active_node
        )
    };
    if why_this_unit.trim().is_empty() {
        return None;
    }
    Some(RunGraphContinuationBinding {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        status: "bound".to_string(),
        active_bounded_unit,
        binding_source: binding_source.to_string(),
        why_this_unit,
        primary_path: "normal_delivery_path".to_string(),
        sequential_vs_parallel_posture: continuation_posture_for_status(status),
        request_text: request_text
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    })
}

pub(crate) fn build_task_graph_continuation_binding(
    run_id: &str,
    request_text: Option<&str>,
    task: &TaskRecord,
    why_override: Option<&str>,
) -> Option<RunGraphContinuationBinding> {
    let task_request_text = task
        .description
        .trim()
        .strip_prefix('\n')
        .unwrap_or(task.description.trim())
        .trim();
    let effective_request_text = if !task_request_text.is_empty() {
        Some(task_request_text.to_string())
    } else {
        task.title
            .trim()
            .chars()
            .next()
            .map(|_| task.title.trim().to_string())
            .or_else(|| {
                request_text
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
    };
    let why_this_unit = if let Some(why_override) = why_override {
        why_override.trim().to_string()
    } else {
        format!(
            "Explicit continuation binding records backlog task `{}` as the next lawful bounded unit for run `{}`.",
            task.id, run_id
        )
    };
    if why_this_unit.trim().is_empty() {
        return None;
    }

    Some(RunGraphContinuationBinding {
        run_id: run_id.to_string(),
        task_id: task.id.clone(),
        status: "bound".to_string(),
        active_bounded_unit: serde_json::json!({
            "kind": "task_graph_task",
            "task_id": task.id.clone(),
            "run_id": run_id,
            "task_status": task.status.clone(),
            "issue_type": task.issue_type.clone(),
        }),
        binding_source: "explicit_continuation_bind_task".to_string(),
        why_this_unit,
        primary_path: "normal_delivery_path".to_string(),
        sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
        request_text: effective_request_text,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    })
}

fn explicit_task_graph_bound_task_id(binding: &RunGraphContinuationBinding) -> Option<&str> {
    if binding.status != "bound"
        || binding.binding_source != "explicit_continuation_bind_task"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return None;
    }
    binding.active_bounded_unit["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            binding
                .task_id
                .trim()
                .chars()
                .next()
                .map(|_| binding.task_id.trim())
        })
}

pub(crate) async fn sync_run_graph_continuation_binding(
    store: &StateStore,
    status: &RunGraphStatus,
    binding_source: &str,
) -> Result<Option<RunGraphContinuationBinding>, String> {
    sync_run_graph_continuation_binding_with_request_text(store, status, binding_source, None).await
}

pub(crate) async fn sync_run_graph_continuation_binding_with_request_text(
    store: &StateStore,
    status: &RunGraphStatus,
    binding_source: &str,
    request_text_override: Option<&str>,
) -> Result<Option<RunGraphContinuationBinding>, String> {
    if let Some(existing) = store
        .run_graph_continuation_binding(&status.run_id)
        .await
        .map_err(|error| {
            format!("Failed to read existing run-graph continuation binding: {error}")
        })?
    {
        if let Some(bound_task_id) = explicit_task_graph_bound_task_id(&existing) {
            if bound_task_id != status.task_id.trim() {
                return Ok(Some(existing));
            }
        }
    }
    let request_text = if let Some(request_text) = request_text_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(request_text.to_string())
    } else {
        store
            .run_graph_dispatch_context(&status.run_id)
            .await
            .map_err(|error| {
                format!("Failed to read persisted run-graph dispatch context: {error}")
            })?
            .map(|context| context.request_text)
    };
    let Some(binding) =
        build_run_graph_continuation_binding(status, request_text.as_deref(), binding_source, None)
    else {
        store
            .clear_run_graph_continuation_binding(&status.run_id)
            .await
            .map_err(|error| {
                format!("Failed to clear stale run-graph continuation binding: {error}")
            })?;
        return Ok(None);
    };
    store
        .record_run_graph_continuation_binding(&binding)
        .await
        .map_err(|error| format!("Failed to record run-graph continuation binding: {error}"))?;
    Ok(Some(binding))
}

fn parse_bind_args(
    args: &[String],
) -> Result<(String, Option<String>, Option<String>, bool), &'static str> {
    if !matches!(
        args,
        [head, subcommand, ..] if head == "continuation" && subcommand == "bind"
    ) {
        return Err(
            "Usage: vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]",
        );
    }

    let Some(run_id) = args.get(2) else {
        return Err(
            "Usage: vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]",
        );
    };
    let mut why = None;
    let mut task_id = None;
    let mut as_json = false;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--task-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]",
                    );
                };
                task_id = Some(value.clone());
                index += 2;
            }
            "--why" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]",
                    );
                };
                why = Some(value.clone());
                index += 2;
            }
            _ => {
                return Err(
                    "Usage: vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]",
                );
            }
        }
    }
    Ok((run_id.clone(), task_id, why, as_json))
}

pub(crate) async fn run_taskflow_continuation(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "continuation" => {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("continuation"));
            return ExitCode::SUCCESS;
        }
        [head, flag] if head == "continuation" && matches!(flag.as_str(), "--help" | "-h") => {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("continuation"));
            return ExitCode::SUCCESS;
        }
        [head, subcommand, flag]
            if head == "continuation"
                && subcommand == "bind"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("continuation"));
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    let (run_id, task_id, why, as_json) = match parse_bind_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            return emit_continuation_bind_error(
                args_request_json(args),
                None,
                None,
                error.to_string(),
                "invalid_continuation_bind_args",
                2,
            );
        }
    };

    let store = match StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                task_id.as_deref(),
                format!("Failed to open authoritative state store: {error}"),
                "taskflow_state_store_unavailable",
                1,
            );
        }
    };
    let status = match store.run_graph_status(&run_id).await {
        Ok(status) => status,
        Err(error) => {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                task_id.as_deref(),
                format!("Failed to read run-graph state for `{run_id}`: {error}"),
                "continuation_binding_run_graph_missing",
                1,
            );
        }
    };
    let request_text = match store.run_graph_dispatch_context(&run_id).await {
        Ok(context) => context.map(|row| row.request_text),
        Err(error) => {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                task_id.as_deref(),
                format!("Failed to read run-graph dispatch context for `{run_id}`: {error}"),
                "continuation_binding_dispatch_context_unavailable",
                1,
            );
        }
    };
    let binding = if let Some(task_id) = task_id.as_deref() {
        if !explicit_task_bind_allowed_for_status(&status, task_id) {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                Some(task_id),
                format!(
                    "Explicit --task-id continuation binding is only allowed for the active run task before closure_complete, or after run `{run_id}` reaches closure_complete with no downstream target."
                ),
                "continuation_binding_lifecycle_not_closed",
                1,
            );
        }
        let task = match store.show_task(task_id).await {
            Ok(task) => task,
            Err(error) => {
                return emit_continuation_bind_error(
                    as_json,
                    Some(&run_id),
                    Some(task_id),
                    format!(
                        "Failed to read task `{task_id}` for explicit continuation binding: {error}"
                    ),
                    "continuation_binding_task_missing",
                    1,
                );
            }
        };
        if StateStore::task_status_is_closed_like(&task.status) {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                Some(task_id),
                format!(
                    "Task `{task_id}` is closed and cannot be recorded as the next lawful bounded unit."
                ),
                "continuation_binding_task_closed",
                1,
            );
        }
        match build_task_graph_continuation_binding(
            &run_id,
            request_text.as_deref(),
            &task,
            why.as_deref(),
        ) {
            Some(binding) => binding,
            None => {
                return emit_continuation_bind_error(
                    as_json,
                    Some(&run_id),
                    Some(task_id),
                    format!(
                        "Task `{task_id}` did not yield a valid explicit continuation binding payload."
                    ),
                    "continuation_binding_payload_invalid",
                    1,
                );
            }
        }
    } else {
        let Some(binding) = build_run_graph_continuation_binding(
            &status,
            request_text.as_deref(),
            "explicit_continuation_bind",
            why.as_deref(),
        ) else {
            return emit_continuation_bind_error(
                as_json,
                Some(&run_id),
                None,
                format!(
                    "Run `{run_id}` does not expose a bindable active bounded unit; refresh run-graph evidence before binding."
                ),
                "continuation_binding_no_active_bounded_unit",
                1,
            );
        };
        binding
    };
    if let Err(error) = store.record_run_graph_continuation_binding(&binding).await {
        return emit_continuation_bind_error(
            as_json,
            Some(&run_id),
            Some(&binding.task_id),
            format!("Failed to record continuation binding: {error}"),
            "continuation_binding_record_failed",
            1,
        );
    }
    crate::operator_projection_cache::write_runtime_continuation_binding_overlay(
        store.root(),
        &binding,
    );

    if as_json {
        crate::print_json_pretty(&continuation_bind_success_payload(&run_id, &binding));
    } else {
        print_surface_header(RenderMode::Plain, CONTINUATION_BIND_SURFACE);
        print_surface_line(RenderMode::Plain, "run", &run_id);
        print_surface_line(RenderMode::Plain, "binding_source", &binding.binding_source);
        print_surface_line(
            RenderMode::Plain,
            "posture",
            &binding.sequential_vs_parallel_posture,
        );
        print_surface_line(RenderMode::Plain, "bound_task_id", &binding.task_id);
        print_surface_line(RenderMode::Plain, "why_this_unit", &binding.why_this_unit);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        build_run_graph_continuation_binding, build_task_graph_continuation_binding,
        continuation_bind_blocked_payload, continuation_bind_success_payload,
        explicit_task_bind_allowed_for_status, parse_bind_args, run_taskflow_continuation,
        sync_run_graph_continuation_binding, terminal_completed_without_next_unit,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parse_bind_args_accepts_task_id_flag() {
        let args = vec![
            "continuation".to_string(),
            "bind".to_string(),
            "run-1".to_string(),
            "--task-id".to_string(),
            "task-42".to_string(),
            "--why".to_string(),
            "explicit".to_string(),
            "--json".to_string(),
        ];

        let (run_id, task_id, why, as_json) = parse_bind_args(&args).expect("args should parse");

        assert_eq!(run_id, "run-1");
        assert_eq!(task_id.as_deref(), Some("task-42"));
        assert_eq!(why.as_deref(), Some("explicit"));
        assert!(as_json);
    }

    #[test]
    fn blocked_json_payload_uses_release_one_operator_envelope() {
        let payload = continuation_bind_blocked_payload(
            Some("run-1"),
            Some("task-42"),
            "blocked",
            "continuation_binding_lifecycle_not_closed",
        );

        assert_eq!(payload["surface"], "vida taskflow continuation bind");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "continuation_binding_lifecycle_not_closed"
        );
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "release-1-operator-contracts"
        );
        assert_eq!(
            payload["operator_contracts"]["schema_version"],
            "release-1-v1"
        );
    }

    #[test]
    fn success_json_payload_uses_release_one_operator_envelope() {
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status("run-1", "implementation", "task");
        status.task_id = "task-42".to_string();
        status.active_node = "implementation".to_string();
        let binding = build_run_graph_continuation_binding(&status, None, "test", None)
            .expect("binding should build");

        let payload = continuation_bind_success_payload("run-1", &binding);

        assert_eq!(payload["surface"], "vida taskflow continuation bind");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["binding"]["task_id"], "task-42");
        assert_eq!(payload["shared_fields"]["status"], "ok");
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "release-1-operator-contracts"
        );
        assert_eq!(payload["operator_contracts"]["status"], "ok");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn continuation_bind_json_rejects_stale_projection_cache_without_state_store() {
        let root = std::env::temp_dir().join(format!(
            "vida-continuation-bind-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status("run-1", "implementation", "task");
        status.task_id = "run-1".to_string();
        status.active_node = "implementation".to_string();
        let binding = build_run_graph_continuation_binding(
            &status,
            Some("continue run 1"),
            "explicit_continuation_bind",
            None,
        )
        .expect("binding should build");
        crate::operator_projection_cache::write_json_projection(
            &root,
            "lane-show-run-1",
            &serde_json::json!({
                "surface": "vida lane",
                "projection_truth": {
                    "continuation_binding": binding
                }
            }),
        );
        crate::operator_projection_cache::write_json_projection(
            &root,
            "orchestrator-init-summary-latest",
            &serde_json::json!({
                "surface": "vida orchestrator-init",
                "init": {
                    "continuation_binding": {
                        "status": "bound",
                        "binding_source": "explicit_continuation_bind",
                        "active_bounded_unit": {
                            "kind": "run_graph_task",
                            "run_id": "run-1",
                            "task_id": "run-1",
                            "active_node": "coach"
                        },
                        "why_this_unit": "Explicit continuation binding records run-1.",
                        "primary_path": "normal_delivery_path",
                        "sequential_vs_parallel_posture": "sequential_only_open_cycle"
                    }
                }
            }),
        );
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));

        let exit = run_taskflow_continuation(&[
            "continuation".to_string(),
            "bind".to_string(),
            "run-1".to_string(),
            "--json".to_string(),
        ])
        .await;

        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        assert_ne!(exit, std::process::ExitCode::SUCCESS);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_task_graph_binding_uses_task_payload() {
        let task = crate::state_store::TaskRecord {
            id: "task-42".to_string(),
            title: "Bounded task".to_string(),
            status: "in_progress".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "1776000000".to_string(),
            created_by: "test".to_string(),
            updated_at: "1776000000".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: String::new(),
            compaction_level: 0,
            original_size: 0,
            description: String::new(),
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
            display_id: None,
        };

        let binding = build_task_graph_continuation_binding("run-1", Some("req"), &task, None)
            .expect("binding should build");

        assert_eq!(binding.task_id, "task-42");
        assert_eq!(binding.binding_source, "explicit_continuation_bind_task");
        assert_eq!(binding.active_bounded_unit["kind"], "task_graph_task");
        assert_eq!(binding.active_bounded_unit["task_status"], "in_progress");
        assert_eq!(binding.request_text.as_deref(), Some("Bounded task"));
    }

    #[test]
    fn explicit_task_bind_allowed_for_active_same_run_task_before_terminal_closure() {
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status("run-1", "planning", "task-42");
        status.task_id = "task-42".to_string();
        status.active_node = "planning".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "analyst_dispatch_ready".to_string();

        assert!(explicit_task_bind_allowed_for_status(&status, "task-42"));
        assert!(!explicit_task_bind_allowed_for_status(
            &status,
            "other-task"
        ));
    }

    #[test]
    fn explicit_task_graph_binding_prefers_task_description_over_inherited_request() {
        let task = crate::state_store::TaskRecord {
            id: "task-42".to_string(),
            title: "Bounded task".to_string(),
            status: "in_progress".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "1776000000".to_string(),
            created_by: "test".to_string(),
            updated_at: "1776000000".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: String::new(),
            compaction_level: 0,
            original_size: 0,
            description: "Task-rooted scoped request".to_string(),
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
            display_id: None,
        };

        let binding = build_task_graph_continuation_binding(
            "run-1",
            Some("stale inherited request"),
            &task,
            None,
        )
        .expect("binding should build");

        assert_eq!(
            binding.request_text.as_deref(),
            Some("Task-rooted scoped request")
        );
    }

    #[test]
    fn completed_status_without_next_node_does_not_infer_closure_target() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-1",
            "implementation",
            "implementation",
        );
        status.task_id = "feature-close-dev".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "implementation_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();

        assert!(
            build_run_graph_continuation_binding(&status, None, "test", None).is_none(),
            "completed non-closure state without explicit next_node must not synthesize closure"
        );

        status.next_node = Some("closure".to_string());
        let binding = build_run_graph_continuation_binding(&status, None, "test", None)
            .expect("explicit closure next_node should bind closure");
        assert_eq!(binding.task_id, "feature-close-dev");
        assert_eq!(
            binding.active_bounded_unit["kind"],
            "downstream_dispatch_target"
        );
        assert_eq!(binding.active_bounded_unit["dispatch_target"], "closure");
        assert_eq!(
            binding.sequential_vs_parallel_posture,
            "sequential_only_open_cycle"
        );
    }

    #[test]
    fn closure_complete_without_next_node_does_not_build_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-1",
            "implementation",
            "implementation",
        );
        status.task_id = "feature-close-dev".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();

        assert!(terminal_completed_without_next_unit(&status));
        assert!(build_run_graph_continuation_binding(&status, None, "test", None).is_none());
    }

    #[tokio::test]
    async fn sync_run_graph_continuation_binding_preserves_explicit_task_graph_binding_for_stale_status(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-continuation-sync-preserve-explicit-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-1".to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": "run-1",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator rebound work to a different task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue bounded task".to_string()),
                    recorded_at: "2026-04-21T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit task binding");

        let mut stale_status =
            crate::taskflow_run_graph::default_run_graph_status("run-1", "closure", "delivery");
        stale_status.task_id = "task-old".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "blocked".to_string();
        stale_status.lifecycle_stage = "closure_blocked".to_string();
        stale_status.resume_target = "dispatch.closure".to_string();

        let binding = sync_run_graph_continuation_binding(
            &store,
            &stale_status,
            "consume_continue_after_downstream_chain",
        )
        .await
        .expect("stale status should preserve explicit binding")
        .expect("binding should remain present");

        assert_eq!(binding.binding_source, "explicit_continuation_bind_task");
        assert_eq!(binding.task_id, "task-new");
        assert_eq!(binding.active_bounded_unit["task_id"], "task-new");

        let persisted = store
            .run_graph_continuation_binding("run-1")
            .await
            .expect("reload binding")
            .expect("binding should stay persisted");
        assert_eq!(persisted.binding_source, "explicit_continuation_bind_task");
        assert_eq!(persisted.task_id, "task-new");
        assert_eq!(persisted.active_bounded_unit["task_id"], "task-new");

        let _ = fs::remove_dir_all(&root);
    }
}
