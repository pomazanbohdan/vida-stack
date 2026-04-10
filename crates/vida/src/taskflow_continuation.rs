use std::process::ExitCode;

use time::format_description::well_known::Rfc3339;

use crate::{
    print_surface_header, print_surface_line,
    state_store::{RunGraphContinuationBinding, RunGraphStatus, StateStore},
    taskflow_task_bridge::proxy_state_dir,
    RenderMode,
};

fn run_graph_active_bounded_unit(status: &RunGraphStatus) -> Option<serde_json::Value> {
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
        return Ok(None);
    };
    store
        .record_run_graph_continuation_binding(&binding)
        .await
        .map_err(|error| format!("Failed to record run-graph continuation binding: {error}"))?;
    Ok(Some(binding))
}

fn parse_bind_args(args: &[String]) -> Result<(String, Option<String>, bool), &'static str> {
    if !matches!(
        args,
        [head, subcommand, ..] if head == "continuation" && subcommand == "bind"
    ) {
        return Err("Usage: vida taskflow continuation bind <run-id> [--why <text>] [--json]");
    }

    let Some(run_id) = args.get(2) else {
        return Err("Usage: vida taskflow continuation bind <run-id> [--why <text>] [--json]");
    };
    let mut why = None;
    let mut as_json = false;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--why" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow continuation bind <run-id> [--why <text>] [--json]",
                    );
                };
                why = Some(value.clone());
                index += 2;
            }
            _ => {
                return Err(
                    "Usage: vida taskflow continuation bind <run-id> [--why <text>] [--json]",
                );
            }
        }
    }
    Ok((run_id.clone(), why, as_json))
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

    let (run_id, why, as_json) = match parse_bind_args(args) {
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
        print_surface_line(RenderMode::Plain, "why_this_unit", &binding.why_this_unit);
    }
    ExitCode::SUCCESS
}
