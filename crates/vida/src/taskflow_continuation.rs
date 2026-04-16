use std::process::ExitCode;

use time::format_description::well_known::Rfc3339;

use crate::{
    print_surface_header, print_surface_line,
    state_store::{RunGraphContinuationBinding, RunGraphStatus, StateStore, TaskRecord},
    taskflow_task_bridge::proxy_state_dir,
    RenderMode,
};

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

fn run_graph_active_bounded_unit(status: &RunGraphStatus) -> Option<serde_json::Value> {
    if terminal_completed_without_next_unit(status) {
        return None;
    }
    if status.status == "completed" {
        let dispatch_target = status
            .next_node
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("closure");
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

fn build_task_graph_continuation_binding(
    run_id: &str,
    request_text: Option<&str>,
    task: &TaskRecord,
    why_override: Option<&str>,
) -> Option<RunGraphContinuationBinding> {
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
        request_text: request_text
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    })
}

pub(crate) async fn sync_run_graph_continuation_binding(
    store: &StateStore,
    status: &RunGraphStatus,
    binding_source: &str,
) -> Result<Option<RunGraphContinuationBinding>, String> {
    let request_text = store
        .run_graph_dispatch_context(&status.run_id)
        .await
        .map_err(|error| format!("Failed to read persisted run-graph dispatch context: {error}"))?
        .map(|context| context.request_text);
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
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let store = match StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let status = match store.run_graph_status(&run_id).await {
        Ok(status) => status,
        Err(error) => {
            eprintln!("Failed to read run-graph state for `{run_id}`: {error}");
            return ExitCode::from(1);
        }
    };
    let request_text = match store.run_graph_dispatch_context(&run_id).await {
        Ok(context) => context.map(|row| row.request_text),
        Err(error) => {
            eprintln!("Failed to read run-graph dispatch context for `{run_id}`: {error}");
            return ExitCode::from(1);
        }
    };
    let binding = if let Some(task_id) = task_id.as_deref() {
        let task = match store.show_task(task_id).await {
            Ok(task) => task,
            Err(error) => {
                eprintln!(
                    "Failed to read task `{task_id}` for explicit continuation binding: {error}"
                );
                return ExitCode::from(1);
            }
        };
        if task.status == "closed" {
            eprintln!(
                "Task `{task_id}` is closed and cannot be recorded as the next lawful bounded unit."
            );
            return ExitCode::from(1);
        }
        match build_task_graph_continuation_binding(
            &run_id,
            request_text.as_deref(),
            &task,
            why.as_deref(),
        ) {
            Some(binding) => binding,
            None => {
                eprintln!(
                    "Task `{task_id}` did not yield a valid explicit continuation binding payload."
                );
                return ExitCode::from(1);
            }
        }
    } else {
        let Some(binding) = build_run_graph_continuation_binding(
            &status,
            request_text.as_deref(),
            "explicit_continuation_bind",
            why.as_deref(),
        ) else {
            eprintln!(
                "Run `{run_id}` does not expose a bindable active bounded unit; refresh run-graph evidence before binding."
            );
            return ExitCode::from(1);
        };
        binding
    };
    if let Err(error) = store.record_run_graph_continuation_binding(&binding).await {
        eprintln!("Failed to record continuation binding: {error}");
        return ExitCode::from(1);
    }

    if as_json {
        crate::print_json_pretty(&serde_json::json!({
            "surface": "vida taskflow continuation bind",
            "run_id": run_id,
            "binding": binding,
        }));
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow continuation bind");
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
        parse_bind_args, terminal_completed_without_next_unit,
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
            dependencies: Vec::new(),
            display_id: None,
        };

        let binding = build_task_graph_continuation_binding("run-1", Some("req"), &task, None)
            .expect("binding should build");

        assert_eq!(binding.task_id, "task-42");
        assert_eq!(binding.binding_source, "explicit_continuation_bind_task");
        assert_eq!(binding.active_bounded_unit["kind"], "task_graph_task");
        assert_eq!(binding.active_bounded_unit["task_status"], "in_progress");
    }

    #[test]
    fn completed_status_without_next_node_binds_closure_target() {
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

        let binding = build_run_graph_continuation_binding(&status, None, "test", None)
            .expect("completed status should bind closure");

        assert_eq!(binding.task_id, "feature-close-dev");
        assert_eq!(
            binding.active_bounded_unit["kind"],
            "downstream_dispatch_target"
        );
        assert_eq!(binding.active_bounded_unit["dispatch_target"], "closure");
        assert_eq!(binding.sequential_vs_parallel_posture, "sequential_only");
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
}
