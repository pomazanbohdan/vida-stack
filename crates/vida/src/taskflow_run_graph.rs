use crate::{
    build_runtime_execution_plan_from_snapshot, build_runtime_lane_selection_with_store,
    open_existing_state_store_with_retry, print_surface_header, print_surface_line,
    proxy_state_dir, read_or_sync_launcher_activation_snapshot,
    state_store::{RunGraphStatus, StateStore, StateStoreError},
    taskflow_layer4::print_taskflow_proxy_help,
    RenderMode, RuntimeConsumptionLaneSelection,
};
use std::process::ExitCode;

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowRunGraphSeedPayload {
    pub(crate) request_text: String,
    pub(crate) role_selection: RuntimeConsumptionLaneSelection,
    pub(crate) status: RunGraphStatus,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowRunGraphAdvancePayload {
    pub(crate) status: RunGraphStatus,
}

#[derive(Clone)]
struct CompiledRunGraphControl {
    implementation: serde_json::Value,
    verification: serde_json::Value,
    validation_report_required_before_implementation: bool,
}

async fn compiled_run_graph_control(store: &StateStore) -> Result<CompiledRunGraphControl, String> {
    let snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
    let selection = RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: snapshot.source,
        selection_mode: "compiled".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: String::new(),
        selected_role: "orchestrator".to_string(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "compiled".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: snapshot.compiled_bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: "compiled_snapshot".to_string(),
    };
    let execution_plan =
        build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, &selection);
    let implementation = execution_plan["development_flow"]["implementation"].clone();
    let verification = execution_plan["development_flow"]["verification"].clone();
    if implementation.is_null() {
        return Err(
            "run-graph control is unavailable in the compiled activation snapshot.".to_string(),
        );
    }

    Ok(CompiledRunGraphControl {
        implementation,
        verification,
        validation_report_required_before_implementation: selection.compiled_bundle
            ["autonomous_execution"]["validation_report_required_before_implementation"]
            .as_bool()
            .unwrap_or(false),
    })
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

pub(crate) fn default_run_graph_status(
    task_id: &str,
    task_class: &str,
    route_task_class: &str,
) -> RunGraphStatus {
    RunGraphStatus {
        run_id: task_id.to_string(),
        task_id: task_id.to_string(),
        task_class: task_class.to_string(),
        active_node: task_class.to_string(),
        next_node: None,
        status: "pending".to_string(),
        route_task_class: route_task_class.to_string(),
        selected_backend: "unknown".to_string(),
        lane_id: "unassigned".to_string(),
        lifecycle_stage: "initialized".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "none".to_string(),
        context_state: "open".to_string(),
        checkpoint_kind: "none".to_string(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    }
}

pub(crate) async fn run_taskflow_recovery(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "recovery" => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "recovery" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "recovery" && subcommand == "gate-latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery gate-latest",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery gate-latest",
                        );
                        print_surface_line(RenderMode::Plain, "gate", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "gate-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate-latest",
                                "gate": summary,
                            }))
                            .expect("latest gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "gate" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery gate");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "gate" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate",
                                "run_id": summary.run_id,
                                "gate": summary,
                            }))
                            .expect("gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "checkpoint-latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint-latest",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint-latest",
                        );
                        print_surface_line(RenderMode::Plain, "checkpoint", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "checkpoint-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint-latest",
                                "checkpoint": summary,
                            }))
                            .expect("latest checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "checkpoint" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "checkpoint" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint",
                                "run_id": summary.run_id,
                                "checkpoint": summary,
                            }))
                            .expect("checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "recovery", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery latest",
                                "recovery": summary,
                            }))
                            .expect("latest recovery summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery status");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery status",
                                "run_id": summary.run_id,
                                "recovery": summary,
                            }))
                            .expect("recovery summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate-latest" => {
            eprintln!("Usage: vida taskflow recovery gate-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate" => {
            eprintln!("Usage: vida taskflow recovery gate <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint-latest" => {
            eprintln!("Usage: vida taskflow recovery checkpoint-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint" => {
            eprintln!("Usage: vida taskflow recovery checkpoint <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow recovery latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow recovery status <run-id> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

pub(crate) async fn run_taskflow_run_graph(args: &[String]) -> ExitCode {
    match args {
        [head, flag] if head == "run-graph" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("run-graph"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "run-graph" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(Some(status)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "status", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "run-graph" && subcommand == "latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(status) => {
                        let delegation_gate =
                            status.as_ref().map(|status| status.delegation_gate());
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph latest",
                                "status": status,
                                "delegation_gate": delegation_gate,
                            }))
                            .expect("latest run-graph status should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        let delegation_gate = status.delegation_gate();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph status",
                                "run_id": status.run_id,
                                "status": status,
                                "delegation_gate": delegation_gate,
                            }))
                            .expect("run-graph status should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow run-graph latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow run-graph status <run-id> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

fn print_run_graph_json_error(surface: &str, run_id: &str, error: &str) {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "surface": surface,
            "run_id": run_id,
            "error": error,
        }))
        .expect("run-graph error should render as json")
    );
}

pub(crate) fn merge_run_graph_meta(
    mut status: RunGraphStatus,
    meta: &serde_json::Value,
) -> RunGraphStatus {
    status.next_node = json_string_field(meta, "next_node").or(status.next_node);
    status.selected_backend =
        json_string_field(meta, "selected_backend").unwrap_or(status.selected_backend);
    status.lane_id = json_string_field(meta, "lane_id").unwrap_or(status.lane_id);
    status.lifecycle_stage =
        json_string_field(meta, "lifecycle_stage").unwrap_or(status.lifecycle_stage);
    status.policy_gate = json_string_field(meta, "policy_gate").unwrap_or(status.policy_gate);
    status.handoff_state = json_string_field(meta, "handoff_state").unwrap_or(status.handoff_state);
    status.context_state = json_string_field(meta, "context_state").unwrap_or(status.context_state);
    status.checkpoint_kind =
        json_string_field(meta, "checkpoint_kind").unwrap_or(status.checkpoint_kind);
    status.resume_target = json_string_field(meta, "resume_target").unwrap_or(status.resume_target);
    status.recovery_ready =
        json_bool_field(meta, "recovery_ready").unwrap_or(status.recovery_ready);
    status
}

#[derive(Clone, Copy)]
enum DispatchTargetFormat {
    Lane,
    Direct,
}

fn governance_handoff(
    next_node: Option<&str>,
    target_format: DispatchTargetFormat,
) -> (String, String) {
    let handoff_state = next_node
        .map(|next| format!("awaiting_{next}"))
        .unwrap_or_else(|| "none".to_string());
    let resume_target = next_node
        .map(|next| match target_format {
            DispatchTargetFormat::Lane => format!("dispatch.{next}_lane"),
            DispatchTargetFormat::Direct => format!("dispatch.{next}"),
        })
        .unwrap_or_else(|| "none".to_string());
    (handoff_state, resume_target)
}

fn run_graph_transition(
    existing: &RunGraphStatus,
    active_node: String,
    next_node: Option<String>,
    lane_id: String,
    lifecycle_stage: String,
    policy_gate: String,
    checkpoint_kind: &str,
    target_format: DispatchTargetFormat,
    recovery_ready: bool,
) -> RunGraphStatus {
    let (handoff_state, resume_target) = governance_handoff(next_node.as_deref(), target_format);

    RunGraphStatus {
        run_id: existing.run_id.clone(),
        task_id: existing.task_id.clone(),
        task_class: existing.task_class.clone(),
        active_node,
        next_node,
        status: "ready".to_string(),
        route_task_class: existing.route_task_class.clone(),
        selected_backend: existing.selected_backend.clone(),
        lane_id,
        lifecycle_stage,
        policy_gate,
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: checkpoint_kind.to_string(),
        resume_target,
        recovery_ready,
    }
}

fn implementation_analysis_gate(
    implementation: &serde_json::Value,
) -> (Option<String>, String, bool) {
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    let coach_node = json_string_field(implementation, "coach_route_task_class")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "coach".to_string());
    let next_node = coach_required.then_some(coach_node);
    let policy_gate = if coach_required {
        json_string_field(implementation, "verification_gate")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "not_required".to_string())
    } else {
        "not_required".to_string()
    };
    let recovery_ready = next_node.is_some()
        || json_bool_field(implementation, "independent_verification_required").unwrap_or(false);
    (next_node, policy_gate, recovery_ready)
}

fn implementation_verification_gate(
    implementation: &serde_json::Value,
    verification: &serde_json::Value,
) -> (Option<String>, String) {
    let verification_route = json_string_field(implementation, "verification_route_task_class")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "verification".to_string());
    let next_node = json_bool_field(implementation, "independent_verification_required")
        .unwrap_or(false)
        .then_some(verification_route);
    let policy_gate = if next_node.is_some() {
        json_string_field(verification, "verification_gate")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification_summary".to_string())
    } else {
        "not_required".to_string()
    };
    (next_node, policy_gate)
}

pub(crate) async fn derive_seeded_run_graph_status(
    store: &StateStore,
    task_id: &str,
    request_text: &str,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    let selection = build_runtime_lane_selection_with_store(store, request_text).await?;
    let execution_plan = &selection.execution_plan;
    let compiled_control = compiled_run_graph_control(store).await?;
    let is_conversation = selection.conversational_mode.is_some();
    let task_class = if is_conversation {
        selection
            .conversational_mode
            .clone()
            .unwrap_or_else(|| "conversation".to_string())
    } else {
        "implementation".to_string()
    };
    let route = if is_conversation {
        &execution_plan["default_route"]
    } else {
        &execution_plan["development_flow"]["implementation"]
    };
    let selected_backend =
        json_string_field(route, "subagents").unwrap_or_else(|| "unknown".to_string());
    let lane_node = if is_conversation {
        selection.selected_role.clone()
    } else {
        json_string_field(route, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| selection.selected_role.clone())
    };
    let lane_id = format!("{lane_node}_lane");
    let next_node = Some(lane_node.clone());
    let lifecycle_stage = if is_conversation {
        "dispatch_ready".to_string()
    } else {
        "implementation_dispatch_ready".to_string()
    };
    let policy_gate = if is_conversation {
        if selection.single_task_only {
            "single_task_scope_required".to_string()
        } else {
            "not_required".to_string()
        }
    } else if execution_plan["state_owner"].as_str() == Some("orchestrator_only")
        && compiled_control.validation_report_required_before_implementation
    {
        "validation_report_required".to_string()
    } else {
        "not_required".to_string()
    };
    let handoff_state = if is_conversation {
        format!("awaiting_{}", selection.selected_role)
    } else {
        format!("awaiting_{lane_node}")
    };
    let checkpoint_kind = if is_conversation {
        "conversation_cursor".to_string()
    } else {
        "execution_cursor".to_string()
    };
    let recovery_ready = is_conversation
        || json_bool_field(route, "analysis_required").unwrap_or(false)
        || json_bool_field(route, "coach_required").unwrap_or(false)
        || json_bool_field(route, "independent_verification_required").unwrap_or(false);
    let seed_base = RunGraphStatus {
        run_id: task_id.to_string(),
        task_id: task_id.to_string(),
        task_class,
        active_node: "planning".to_string(),
        route_task_class: if is_conversation {
            selection
                .tracked_flow_entry
                .clone()
                .or_else(|| selection.conversational_mode.clone())
                .unwrap_or_else(|| selection.selected_role.clone())
        } else {
            "implementation".to_string()
        },
        selected_backend,
        ..default_run_graph_status(task_id, "planning", "implementation")
    };
    let mut status = run_graph_transition(
        &seed_base,
        "planning".to_string(),
        next_node,
        lane_id,
        lifecycle_stage,
        policy_gate,
        &checkpoint_kind,
        DispatchTargetFormat::Lane,
        recovery_ready,
    );
    status.task_class = seed_base.task_class;
    status.route_task_class = seed_base.route_task_class;
    status.selected_backend = seed_base.selected_backend;
    status.handoff_state = handoff_state;

    Ok(TaskflowRunGraphSeedPayload {
        request_text: request_text.to_string(),
        role_selection: selection,
        status,
    })
}

pub(crate) async fn derive_advanced_run_graph_status(
    store: &StateStore,
    existing: RunGraphStatus,
) -> Result<TaskflowRunGraphAdvancePayload, String> {
    let compiled_control = compiled_run_graph_control(store).await?;
    let implementation = compiled_control.implementation;

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "planning"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let analysis_node = json_string_field(&implementation, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "analysis".to_string());
        if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{analysis_node}` for the seeded implementation run, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                analysis_node.clone(),
                next_node,
                format!("{analysis_node}_lane"),
                "analysis_active".to_string(),
                policy_gate,
                "execution_cursor",
                DispatchTargetFormat::Lane,
                recovery_ready,
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "analysis"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let coach_node = json_string_field(&implementation, "coach_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "coach".to_string());
        if existing.next_node.as_deref() != Some(coach_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{coach_node}` for the implementation analysis handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let verification = compiled_control.verification.clone();
        let (next_node, policy_gate) =
            implementation_verification_gate(&implementation, &verification);
        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                coach_node.clone(),
                next_node,
                format!("{coach_node}_lane"),
                "coach_active".to_string(),
                policy_gate,
                "execution_cursor",
                DispatchTargetFormat::Direct,
                true,
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "coach"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let verification_node = json_string_field(&implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        if existing.next_node.as_deref() != Some(verification_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{verification_node}` for the implementation coach handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let verification = compiled_control.verification.clone();

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                verification_node.clone(),
                None,
                format!("{verification_node}_lane"),
                format!("{verification_node}_active"),
                json_string_field(&verification, "verification_gate")
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| existing.policy_gate.clone()),
                "execution_cursor",
                DispatchTargetFormat::Lane,
                false,
            ),
        });
    }

    if existing.task_class == "implementation" && existing.route_task_class == "implementation" {
        let verification_node = json_string_field(&implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        if existing.active_node != verification_node {
            // fall through
        } else {
            match existing.status.as_str() {
                "rework_ready" => {
                    let analysis_node =
                        json_string_field(&implementation, "analysis_route_task_class")
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| "analysis".to_string());
                    if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
                        return Err(format!(
                            "run-graph advance expected next node `{analysis_node}` for the explicit review rework loop, got `{}`",
                            existing.next_node.as_deref().unwrap_or("none")
                        ));
                    }

                    let (next_node, policy_gate, recovery_ready) =
                        implementation_analysis_gate(&implementation);

                    return Ok(TaskflowRunGraphAdvancePayload {
                        status: run_graph_transition(
                            &existing,
                            analysis_node.clone(),
                            next_node,
                            format!("{analysis_node}_lane"),
                            "analysis_active".to_string(),
                            policy_gate,
                            "execution_cursor",
                            DispatchTargetFormat::Lane,
                            recovery_ready,
                        ),
                    });
                }
                "clean" => {
                    let mut status = run_graph_transition(
                        &existing,
                        existing.active_node.clone(),
                        None,
                        existing.lane_id.clone(),
                        "implementation_complete".to_string(),
                        "not_required".to_string(),
                        &existing.checkpoint_kind,
                        DispatchTargetFormat::Lane,
                        false,
                    );
                    status.status = "completed".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                "review_findings" | "changed_scope" => {
                    return Err(format!(
                        "run-graph advance blocked: implementation review findings require explicit scope/rework resolution before completion; got status `{}`",
                        existing.status
                    ));
                }
                other => {
                    return Err(format!(
                        "run-graph advance expected `{verification_node}` status `clean` before completing implementation, got `{other}`"
                    ));
                }
            }
        }
    }

    if matches!(
        existing.task_class.as_str(),
        "scope_discussion" | "pbi_discussion"
    ) && existing.active_node == "planning"
    {
        let analyst_node = existing
            .next_node
            .clone()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "run-graph advance expected a seeded conversational next node, got `none`"
                    .to_string()
            })?;
        if existing.route_task_class.is_empty() || existing.route_task_class == existing.task_class
        {
            return Err(format!(
                "run-graph advance expected a seeded conversational route target for `{}`, got `{}`",
                existing.task_class, existing.route_task_class
            ));
        }
        let route_target = existing.route_task_class.clone();
        let next_node = Some(route_target.clone());

        return Ok(TaskflowRunGraphAdvancePayload {
            status: {
                let mut status = run_graph_transition(
                    &existing,
                    analyst_node.clone(),
                    next_node.clone(),
                    format!("{analyst_node}_lane"),
                    "conversation_active".to_string(),
                    existing.policy_gate.clone(),
                    "conversation_cursor",
                    DispatchTargetFormat::Lane,
                    true,
                );
                status.handoff_state = format!("awaiting_{route_target}");
                status.resume_target = format!("dispatch.{route_target}");
                status
            },
        });
    }

    Err(format!(
        "run-graph advance currently supports only seeded implementation, scope-discussion, or pbi-discussion runs; got class={} route={} node={}",
        existing.task_class, existing.route_task_class, existing.active_node
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_handoff_uses_lane_targets_for_execution() {
        let (handoff_state, resume_target) =
            governance_handoff(Some("coach"), DispatchTargetFormat::Lane);
        assert_eq!(handoff_state, "awaiting_coach");
        assert_eq!(resume_target, "dispatch.coach_lane");
    }

    #[test]
    fn governance_handoff_uses_direct_targets_for_conversation() {
        let (handoff_state, resume_target) =
            governance_handoff(Some("spec-pack"), DispatchTargetFormat::Direct);
        assert_eq!(handoff_state, "awaiting_spec-pack");
        assert_eq!(resume_target, "dispatch.spec-pack");
    }

    #[test]
    fn implementation_analysis_gate_tracks_coach_and_verification_requirements() {
        let implementation = serde_json::json!({
            "coach_required": true,
            "coach_route_task_class": "coach",
            "verification_gate": "targeted_verification",
            "independent_verification_required": true
        });

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);
        assert_eq!(next_node, Some("coach".to_string()));
        assert_eq!(policy_gate, "targeted_verification");
        assert!(recovery_ready);
    }

    #[test]
    fn implementation_verification_gate_falls_back_when_independent_review_is_disabled() {
        let implementation = serde_json::json!({
            "verification_route_task_class": "review_ensemble",
            "independent_verification_required": false
        });
        let verification = serde_json::json!({
            "verification_gate": "review_findings"
        });

        let (next_node, policy_gate) =
            implementation_verification_gate(&implementation, &verification);
        assert_eq!(next_node, None);
        assert_eq!(policy_gate, "not_required");
    }
}

pub(crate) async fn run_taskflow_run_graph_mutation(args: &[String]) -> ExitCode {
    let state_dir = proxy_state_dir();
    let store = match StateStore::open(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match args {
        [head, subcommand, task_id] if head == "run-graph" && subcommand == "advance" => {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let payload = match derive_advanced_run_graph_status(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            match store.record_run_graph_status(&payload.status).await {
                Ok(()) => {
                    print_surface_header(RenderMode::Plain, "vida taskflow run-graph advance");
                    print_surface_line(RenderMode::Plain, "run", task_id);
                    print_surface_line(
                        RenderMode::Plain,
                        "active node",
                        &payload.status.active_node,
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "next node",
                        payload.status.next_node.as_deref().unwrap_or("none"),
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "delegation gate",
                        &payload.status.delegation_gate().as_display(),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to advance run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, flag]
            if head == "run-graph" && subcommand == "advance" && flag == "--json" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(error) => {
                    let message = format!("Failed to read existing run-graph state: {error}");
                    eprintln!("{message}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &message,
                    );
                    return ExitCode::from(1);
                }
            };
            let payload = match derive_advanced_run_graph_status(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
                    print_run_graph_json_error("vida taskflow run-graph advance", task_id, &error);
                    return ExitCode::from(1);
                }
            };
            match store.record_run_graph_status(&payload.status).await {
                Ok(()) => {
                    let delegation_gate = payload.status.delegation_gate();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow run-graph advance",
                            "run_id": task_id,
                            "payload": payload,
                            "delegation_gate": delegation_gate,
                        }))
                        .expect("run-graph advance should render as json")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    let message = format!("Failed to advance run-graph state: {error}");
                    eprintln!("{message}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &message,
                    );
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, request @ ..]
            if head == "run-graph" && subcommand == "seed" =>
        {
            let as_json = request.iter().any(|arg| arg == "--json");
            let request_text = request
                .iter()
                .filter(|arg| arg.as_str() != "--json")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if request_text.is_empty() {
                eprintln!("Usage: vida taskflow run-graph seed <task_id> <request_text> [--json]");
                return ExitCode::from(2);
            }

            let payload = match derive_seeded_run_graph_status(&store, task_id, &request_text).await
            {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            match store.record_run_graph_status(&payload.status).await {
                Ok(()) => {
                    if as_json {
                        let delegation_gate = payload.status.delegation_gate();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph seed",
                                "run_id": task_id,
                                "payload": payload,
                                "delegation_gate": delegation_gate,
                            }))
                            .expect("run-graph seed should render as json")
                        );
                    } else {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph seed");
                        print_surface_line(RenderMode::Plain, "run", task_id);
                        print_surface_line(RenderMode::Plain, "request", &request_text);
                        print_surface_line(
                            RenderMode::Plain,
                            "selected role",
                            &payload.role_selection.selected_role,
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "next node",
                            payload.status.next_node.as_deref().unwrap_or("none"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "route",
                            &payload.status.route_task_class,
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &payload.status.delegation_gate().as_display(),
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to seed run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class] if head == "run-graph" && subcommand == "init" => {
            let status = default_run_graph_status(task_id, task_class, task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, route_task_class]
            if head == "run-graph" && subcommand == "init" =>
        {
            let status = default_run_graph_status(task_id, task_class, route_task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: existing.route_task_class,
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: route_task_class.to_string(),
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class, meta_json]
            if head == "run-graph" && subcommand == "update" =>
        {
            let meta: serde_json::Value = match serde_json::from_str(meta_json) {
                Ok(meta) => meta,
                Err(error) => {
                    eprintln!("[run-graph] meta_json must be valid JSON: {error}");
                    return ExitCode::from(2);
                }
            };
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = merge_run_graph_meta(
                RunGraphStatus {
                    run_id: task_id.to_string(),
                    task_id: task_id.to_string(),
                    task_class: task_class.to_string(),
                    active_node: node.to_string(),
                    next_node: existing.next_node,
                    status: status.to_string(),
                    route_task_class: route_task_class.to_string(),
                    selected_backend: existing.selected_backend,
                    lane_id: existing.lane_id,
                    lifecycle_stage: existing.lifecycle_stage,
                    policy_gate: existing.policy_gate,
                    handoff_state: existing.handoff_state,
                    context_state: existing.context_state,
                    checkpoint_kind: existing.checkpoint_kind,
                    resume_target: existing.resume_target,
                    recovery_ready: existing.recovery_ready,
                },
                &meta,
            );
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "init" => {
            eprintln!(
                "Usage: vida taskflow run-graph init <task_id> <task_class> [route_task_class]"
            );
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "seed" => {
            eprintln!("Usage: vida taskflow run-graph seed <task_id> <request_text> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "advance" => {
            eprintln!("Usage: vida taskflow run-graph advance <task_id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "update" => {
            eprintln!(
                "Usage: vida taskflow run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]"
            );
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}
