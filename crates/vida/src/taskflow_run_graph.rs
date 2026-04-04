use crate::{
    build_runtime_execution_plan_from_snapshot, build_runtime_lane_selection_with_store,
    operator_contracts::canonical_release1_blocker_code_entries,
    print_surface_header, print_surface_line, read_or_sync_launcher_activation_snapshot,
    state_store::{RunGraphStatus, StateStore, StateStoreError},
    taskflow_layer4::print_taskflow_proxy_help,
    taskflow_routing::selected_backend_from_execution_plan_route,
    taskflow_task_bridge::proxy_state_dir,
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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
            match StateStore::open_existing(state_dir).await {
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

fn print_run_graph_json_error(
    surface: &str,
    run_id: &str,
    error: &str,
    evidence: Option<serde_json::Value>,
) {
    let mut payload = serde_json::json!({
        "surface": surface,
        "run_id": run_id,
        "error": error,
    });
    if let Some(evidence) = evidence {
        payload["incident"] = evidence["incident"].clone();
        payload["blockers"] = evidence["blockers"].clone();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("run-graph error should render as json")
    );
}

fn run_graph_blocker_code(status: &str) -> Option<&'static str> {
    match status {
        "denied" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewDenied,
        )),
        "expired" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewExpired,
        )),
        "review_findings" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewFindings,
        )),
        "changed_scope" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewChangedScope,
        )),
        _ => None,
    }
}

struct RunGraphBlockerEvidenceArgs<'a> {
    run_id: &'a str,
    active_node: &'a str,
    status: &'a str,
    route_task_class: &'a str,
    policy_gate: &'a str,
    resume_target: &'a str,
    next_node: Option<&'a str>,
    error: &'a str,
}

fn run_graph_blocker_evidence(
    args: RunGraphBlockerEvidenceArgs<'_>,
) -> Result<Option<serde_json::Value>, String> {
    let is_blocked_advance = args.error.starts_with("run-graph advance blocked:");
    if !is_blocked_advance {
        return Ok(None);
    }
    let blocker_code = run_graph_blocker_code(args.status).ok_or_else(|| {
        format!(
            "run-graph advance blocked without explicit blocker evidence for `{}` status `{}`; refusing to continue (fail-closed)",
            args.run_id, args.status
        )
    })?;
    let canonical_blocker_codes =
        canonical_release1_blocker_code_entries(&serde_json::json!([blocker_code])).ok_or_else(
            || {
                format!(
            "run-graph blocker code `{blocker_code}` is not canonical (must be lowercase/digits/_)"
        )
            },
        )?;
    let canonical_blocker_code = canonical_blocker_codes
        .first()
        .expect("canonical block list always non-empty")
        .clone();
    Ok(Some(serde_json::json!({
        "incident": {
            "code": "run_graph_advance_blocked",
            "run_id": args.run_id,
            "active_node": args.active_node,
            "status": args.status,
            "route_task_class": args.route_task_class,
        },
        "blockers": [{
            "code": canonical_blocker_code,
            "policy_gate": args.policy_gate,
            "resume_target": args.resume_target,
            "next_node": args.next_node,
            "source": "run_graph_state",
        }]
    })))
}

pub(crate) fn is_dispatch_resume_handoff_complete(status: &RunGraphStatus) -> bool {
    if !status.resume_target.starts_with("dispatch.") {
        return true;
    }
    status.next_node.is_some()
        && !status.policy_gate.trim().is_empty()
        && status.policy_gate != "none"
        && !status.handoff_state.trim().is_empty()
        && status.handoff_state != "none"
}

pub(crate) fn validate_run_graph_resume_gate(status: &RunGraphStatus) -> Result<(), String> {
    if !status.recovery_ready {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: recovery_ready is false",
            status.run_id
        ));
    }
    if status.resume_target == "none" || !status.resume_target.starts_with("dispatch.") {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: resume_target `{}` is not a dispatch target",
            status.run_id, status.resume_target
        ));
    }
    ensure_resume_target_handoff_consistency(status).map_err(|error| {
        format!(
            "Run-graph resume gate denied for `{}`: {error}",
            status.run_id
        )
    })?;
    if !is_dispatch_resume_handoff_complete(status) {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
            status.run_id,
            status.resume_target,
            status.next_node.as_deref().unwrap_or("none"),
            status.policy_gate,
            status.handoff_state
        ));
    }
    if !status.delegation_gate().delegated_cycle_open {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: delegated cycle is not open",
            status.run_id
        ));
    }
    Ok(())
}
fn resume_dispatch_node(resume_target: &str) -> Option<&str> {
    let resume_target = resume_target.trim();
    let stripped = resume_target.strip_prefix("dispatch.")?;
    let node = stripped.strip_suffix("_lane").unwrap_or(stripped);
    if node.is_empty() {
        return None;
    }
    Some(node)
}

fn ensure_resume_target_handoff_consistency(status: &RunGraphStatus) -> Result<(), String> {
    if let Some(node) = resume_dispatch_node(&status.resume_target) {
        let expected_handoff = format!("awaiting_{node}");
        if status.handoff_state != expected_handoff {
            return Err(format!(
                "run-graph resume metadata inconsistent for `{}`: resume_target `{}` requires handoff_state `{}`, not `{}`",
                status.run_id, status.resume_target, expected_handoff, status.handoff_state
            ));
        }
        if status.next_node.as_deref() != Some(node) {
            return Err(format!(
                "run-graph resume metadata inconsistent for `{}`: resume_target `{}` requires next_node `{}`",
                status.run_id, status.resume_target, node
            ));
        }
    } else if status.handoff_state.starts_with("awaiting_") {
        return Err(format!(
            "run-graph resume metadata inconsistent for `{}`: handoff_state `{}` requires a dispatch.* resume_target",
            status.run_id, status.handoff_state
        ));
    }
    Ok(())
}

fn canonicalize_resume_meta(status: &mut RunGraphStatus) {
    if let Some(node) = resume_dispatch_node(&status.resume_target) {
        status.next_node = Some(node.to_string());
        status.handoff_state = format!("awaiting_{node}");
    } else {
        status.next_node = None;
        status.handoff_state = "none".to_string();
    }
}

fn meta_string_field(meta: &serde_json::Value, key: &str) -> Option<Option<String>> {
    meta.get(key)?;
    Some(
        meta.get(key)
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
    )
}

pub(crate) fn merge_run_graph_meta(
    mut status: RunGraphStatus,
    meta: &serde_json::Value,
) -> RunGraphStatus {
    if let Some(selected_backend) = meta
        .get("selected_backend")
        .and_then(|value| value.as_str())
    {
        status.selected_backend = selected_backend.to_string();
    }
    if let Some(lane_id) = meta.get("lane_id").and_then(|value| value.as_str()) {
        status.lane_id = lane_id.to_string();
    }
    if let Some(lifecycle_stage) = meta.get("lifecycle_stage").and_then(|value| value.as_str()) {
        status.lifecycle_stage = lifecycle_stage.to_string();
    }
    if let Some(policy_gate) = meta.get("policy_gate").and_then(|value| value.as_str()) {
        status.policy_gate = policy_gate.to_string();
    }
    let resume_meta = meta_string_field(meta, "resume_target");
    if let Some(context_state) = meta.get("context_state").and_then(|value| value.as_str()) {
        status.context_state = context_state.to_string();
    }
    if let Some(checkpoint_kind) = meta.get("checkpoint_kind").and_then(|value| value.as_str()) {
        status.checkpoint_kind = checkpoint_kind.to_string();
    }
    if let Some(resume_field) = resume_meta {
        status.resume_target = resume_field.unwrap_or_else(|| "none".to_string());
        canonicalize_resume_meta(&mut status);
    } else {
        if let Some(next_node_field) = meta_string_field(meta, "next_node") {
            status.next_node = next_node_field;
        }
        if let Some(handoff_field) = meta_string_field(meta, "handoff_state") {
            status.handoff_state = handoff_field.unwrap_or_else(|| "none".to_string());
        }
    }
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

struct RunGraphTransitionArgs {
    active_node: String,
    next_node: Option<String>,
    lane_id: String,
    lifecycle_stage: String,
    policy_gate: String,
    checkpoint_kind: String,
    target_format: DispatchTargetFormat,
    recovery_ready: bool,
}

fn run_graph_transition(existing: &RunGraphStatus, args: RunGraphTransitionArgs) -> RunGraphStatus {
    let (handoff_state, resume_target) =
        governance_handoff(args.next_node.as_deref(), args.target_format);

    RunGraphStatus {
        run_id: existing.run_id.clone(),
        task_id: existing.task_id.clone(),
        task_class: existing.task_class.clone(),
        active_node: args.active_node,
        next_node: args.next_node,
        status: "ready".to_string(),
        route_task_class: existing.route_task_class.clone(),
        selected_backend: existing.selected_backend.clone(),
        lane_id: args.lane_id,
        lifecycle_stage: args.lifecycle_stage,
        policy_gate: args.policy_gate,
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: args.checkpoint_kind,
        resume_target,
        recovery_ready: args.recovery_ready,
    }
}

fn implementation_analysis_gate(
    implementation: &serde_json::Value,
) -> (Option<String>, String, bool) {
    let writer_node = implementation_writer_node(implementation);
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    let next_node = Some(writer_node);
    let policy_gate = if coach_required {
        json_string_field(implementation, "verification_gate")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "not_required".to_string())
    } else {
        "not_required".to_string()
    };
    let recovery_ready = next_node.is_some()
        || coach_required
        || json_bool_field(implementation, "independent_verification_required").unwrap_or(false);
    (next_node, policy_gate, recovery_ready)
}

fn implementation_writer_node(implementation: &serde_json::Value) -> String {
    json_string_field(implementation, "writer_route_task_class")
        .or_else(|| json_string_field(implementation, "implementer_route_task_class"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "writer".to_string())
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

fn implementation_writer_handoff(
    implementation: &serde_json::Value,
    verification: &serde_json::Value,
) -> (String, Option<String>, String, DispatchTargetFormat, bool) {
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    if coach_required {
        let coach_node = json_string_field(implementation, "coach_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "coach".to_string());
        let (next_node, policy_gate) =
            implementation_verification_gate(implementation, verification);
        return (
            coach_node,
            next_node,
            policy_gate,
            DispatchTargetFormat::Direct,
            true,
        );
    }

    let verification_required =
        json_bool_field(implementation, "independent_verification_required").unwrap_or(false);
    if verification_required {
        let verification_node = json_string_field(implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        return (
            verification_node,
            None,
            json_string_field(verification, "verification_gate")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "verification_summary".to_string()),
            DispatchTargetFormat::Lane,
            false,
        );
    }

    (
        implementation_writer_node(implementation),
        None,
        "not_required".to_string(),
        DispatchTargetFormat::Lane,
        false,
    )
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ImplementationVerificationOutcome {
    ReworkReady,
    Clean,
    Approved,
    FindingsBlocked,
    UnexpectedStatus,
}

fn implementation_verification_outcome(status: &str) -> ImplementationVerificationOutcome {
    const OUTCOME_TABLE: &[(&str, ImplementationVerificationOutcome)] = &[
        (
            "rework_ready",
            ImplementationVerificationOutcome::ReworkReady,
        ),
        ("clean", ImplementationVerificationOutcome::Clean),
        (
            crate::release1_contracts::ApprovalStatus::Approved.as_str(),
            ImplementationVerificationOutcome::Approved,
        ),
        (
            crate::release1_contracts::ApprovalStatus::Denied.as_str(),
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            crate::release1_contracts::ApprovalStatus::Expired.as_str(),
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "review_findings",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "changed_scope",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
    ];

    OUTCOME_TABLE
        .iter()
        .find_map(|(key, outcome)| (*key == status).then_some(*outcome))
        .unwrap_or(ImplementationVerificationOutcome::UnexpectedStatus)
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
    let selected_backend = selected_backend_from_execution_plan_route(execution_plan, route)
        .unwrap_or_else(|| "unknown".to_string());
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
        RunGraphTransitionArgs {
            active_node: "planning".to_string(),
            next_node,
            lane_id,
            lifecycle_stage,
            policy_gate,
            checkpoint_kind,
            target_format: DispatchTargetFormat::Lane,
            recovery_ready,
        },
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
                RunGraphTransitionArgs {
                    active_node: analysis_node.clone(),
                    next_node,
                    lane_id: format!("{analysis_node}_lane"),
                    lifecycle_stage: "analysis_active".to_string(),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready,
                },
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

        let writer_node = implementation_writer_node(&implementation);
        if existing.next_node.as_deref() != Some(writer_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{writer_node}` for the implementation analysis handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let coach_required = json_bool_field(&implementation, "coach_required").unwrap_or(false);
        let verification = compiled_control.verification.clone();
        let (next_node, policy_gate) =
            implementation_verification_gate(&implementation, &verification);
        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: writer_node.clone(),
                    next_node: if coach_required {
                        json_string_field(&implementation, "coach_route_task_class")
                            .filter(|value| !value.is_empty())
                            .or(next_node)
                    } else {
                        next_node
                    },
                    lane_id: format!("{writer_node}_lane"),
                    lifecycle_stage: "writer_active".to_string(),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: true,
                },
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == implementation_writer_node(&implementation)
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let verification = compiled_control.verification.clone();
        let (active_node, next_node, policy_gate, target_format, recovery_ready) =
            implementation_writer_handoff(&implementation, &verification);
        if existing.next_node.as_deref() != Some(active_node.as_str())
            && existing.next_node.is_some()
        {
            return Err(format!(
                "run-graph advance expected next node `{active_node}` for the implementation writer handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        if active_node == existing.active_node && next_node.is_none() {
            let mut status = run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: existing.active_node.clone(),
                    next_node: None,
                    lane_id: existing.lane_id.clone(),
                    lifecycle_stage: "implementation_complete".to_string(),
                    policy_gate: "not_required".to_string(),
                    checkpoint_kind: existing.checkpoint_kind.clone(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: false,
                },
            );
            status.status = "completed".to_string();
            status.context_state = existing.context_state;
            return Ok(TaskflowRunGraphAdvancePayload { status });
        }

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: active_node.clone(),
                    next_node,
                    lane_id: format!("{active_node}_lane"),
                    lifecycle_stage: format!("{active_node}_active"),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format,
                    recovery_ready,
                },
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
                RunGraphTransitionArgs {
                    active_node: verification_node.clone(),
                    next_node: None,
                    lane_id: format!("{verification_node}_lane"),
                    lifecycle_stage: format!("{verification_node}_active"),
                    policy_gate: json_string_field(&verification, "verification_gate")
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| existing.policy_gate.clone()),
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: false,
                },
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
            match implementation_verification_outcome(existing.status.as_str()) {
                ImplementationVerificationOutcome::ReworkReady => {
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
                            RunGraphTransitionArgs {
                                active_node: analysis_node.clone(),
                                next_node,
                                lane_id: format!("{analysis_node}_lane"),
                                lifecycle_stage: "analysis_active".to_string(),
                                policy_gate,
                                checkpoint_kind: "execution_cursor".to_string(),
                                target_format: DispatchTargetFormat::Lane,
                                recovery_ready,
                            },
                        ),
                    });
                }
                ImplementationVerificationOutcome::Clean => {
                    let mut status = run_graph_transition(
                        &existing,
                        RunGraphTransitionArgs {
                            active_node: existing.active_node.clone(),
                            next_node: Some("approval".to_string()),
                            lane_id: existing.lane_id.clone(),
                            lifecycle_stage: "approval_wait".to_string(),
                            policy_gate:
                                crate::release1_contracts::ApprovalStatus::ApprovalRequired
                                    .as_str()
                                    .to_string(),
                            checkpoint_kind: existing.checkpoint_kind.clone(),
                            target_format: DispatchTargetFormat::Direct,
                            recovery_ready: true,
                        },
                    );
                    status.status = "awaiting_approval".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                ImplementationVerificationOutcome::Approved => {
                    let mut status = run_graph_transition(
                        &existing,
                        RunGraphTransitionArgs {
                            active_node: existing.active_node.clone(),
                            next_node: None,
                            lane_id: existing.lane_id.clone(),
                            lifecycle_stage: "implementation_complete".to_string(),
                            policy_gate: "not_required".to_string(),
                            checkpoint_kind: existing.checkpoint_kind.clone(),
                            target_format: DispatchTargetFormat::Lane,
                            recovery_ready: false,
                        },
                    );
                    status.status = "completed".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                ImplementationVerificationOutcome::FindingsBlocked => {
                    return Err(format!(
                        "run-graph advance blocked: implementation review findings require explicit scope/rework resolution before completion; got status `{}`",
                        existing.status
                    ));
                }
                ImplementationVerificationOutcome::UnexpectedStatus => {
                    return Err(format!(
                        "run-graph advance expected `{verification_node}` status `clean` to enter approval wait or `approved` to complete implementation, got `{}`",
                        existing.status
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
                    RunGraphTransitionArgs {
                        active_node: analyst_node.clone(),
                        next_node: next_node.clone(),
                        lane_id: format!("{analyst_node}_lane"),
                        lifecycle_stage: "conversation_active".to_string(),
                        policy_gate: existing.policy_gate.clone(),
                        checkpoint_kind: "conversation_cursor".to_string(),
                        target_format: DispatchTargetFormat::Lane,
                        recovery_ready: true,
                    },
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
                        None,
                    );
                    return ExitCode::from(1);
                }
            };
            let blocker_run_id = existing.run_id.clone();
            let blocker_active_node = existing.active_node.clone();
            let blocker_status = existing.status.clone();
            let blocker_route_task_class = existing.route_task_class.clone();
            let blocker_policy_gate = existing.policy_gate.clone();
            let blocker_resume_target = existing.resume_target.clone();
            let blocker_next_node = existing.next_node.clone();
            let payload = match derive_advanced_run_graph_status(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    let evidence = match run_graph_blocker_evidence(RunGraphBlockerEvidenceArgs {
                        run_id: &blocker_run_id,
                        active_node: &blocker_active_node,
                        status: &blocker_status,
                        route_task_class: &blocker_route_task_class,
                        policy_gate: &blocker_policy_gate,
                        resume_target: &blocker_resume_target,
                        next_node: blocker_next_node.as_deref(),
                        error: &error,
                    }) {
                        Ok(evidence) => evidence,
                        Err(guard_error) => {
                            eprintln!("{guard_error}");
                            print_run_graph_json_error(
                                "vida taskflow run-graph advance",
                                task_id,
                                &guard_error,
                                None,
                            );
                            return ExitCode::from(1);
                        }
                    };
                    eprintln!("{error}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &error,
                        evidence,
                    );
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
                        None,
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
        assert_eq!(next_node, Some("writer".to_string()));
        assert_eq!(policy_gate, "targeted_verification");
        assert!(recovery_ready);
    }

    #[test]
    fn implementation_analysis_gate_keeps_writer_step_when_coach_is_disabled() {
        let implementation = serde_json::json!({
            "coach_required": false,
            "independent_verification_required": false
        });

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);
        assert_eq!(next_node, Some("writer".to_string()));
        assert_eq!(policy_gate, "not_required");
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

    #[test]
    fn implementation_verification_outcome_uses_expected_table_mappings() {
        assert_eq!(
            implementation_verification_outcome("rework_ready"),
            ImplementationVerificationOutcome::ReworkReady
        );
        assert_eq!(
            implementation_verification_outcome("clean"),
            ImplementationVerificationOutcome::Clean
        );
        assert_eq!(
            implementation_verification_outcome("approved"),
            ImplementationVerificationOutcome::Approved
        );
        assert_eq!(
            implementation_verification_outcome("denied"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("expired"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("review_findings"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("changed_scope"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
    }

    #[test]
    fn implementation_verification_outcome_defaults_for_unexpected_status() {
        assert_eq!(
            implementation_verification_outcome("paused"),
            ImplementationVerificationOutcome::UnexpectedStatus
        );
    }

    #[test]
    fn merge_run_graph_meta_allows_explicit_null_to_clear_handoff_fields() {
        let merged = merge_run_graph_meta(
            RunGraphStatus {
                run_id: "run-1".to_string(),
                task_id: "run-1".to_string(),
                task_class: "implementation".to_string(),
                active_node: "writer".to_string(),
                next_node: Some("coach".to_string()),
                status: "ready".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "middle".to_string(),
                lane_id: "writer_lane".to_string(),
                lifecycle_stage: "writer_active".to_string(),
                policy_gate: "targeted_verification".to_string(),
                handoff_state: "awaiting_coach".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.writer_lane".to_string(),
                recovery_ready: true,
            },
            &serde_json::json!({
                "next_node": null,
                "handoff_state": null,
                "resume_target": null,
                "recovery_ready": false
            }),
        );

        assert_eq!(merged.next_node, None);
        assert_eq!(merged.handoff_state, "none");
        assert_eq!(merged.resume_target, "none");
        assert!(!merged.recovery_ready);
    }

    #[test]
    fn merge_run_graph_meta_canonicalizes_resume_target_drifts() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };

        let merged = merge_run_graph_meta(
            status,
            &serde_json::json!({
                "resume_target": "dispatch.coach",
                "next_node": "writer",
                "handoff_state": "awaiting_writer"
            }),
        );

        assert_eq!(merged.resume_target, "dispatch.coach");
        assert_eq!(merged.next_node.as_deref(), Some("coach"));
        assert_eq!(merged.handoff_state, "awaiting_coach");
    }

    #[test]
    fn merge_run_graph_meta_resets_resume_fields_when_target_none() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let merged = merge_run_graph_meta(status, &serde_json::json!({ "resume_target": null }));

        assert_eq!(merged.resume_target, "none");
        assert_eq!(merged.next_node, None);
        assert_eq!(merged.handoff_state, "none");
    }

    #[test]
    fn validate_run_graph_resume_gate_requires_dispatch_resume_target() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("resume_target"));
    }

    #[test]
    fn validate_run_graph_resume_gate_accepts_open_delegation_cycle() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        validate_run_graph_resume_gate(&status).expect("should pass");
    }

    #[test]
    fn validate_run_graph_resume_gate_rejects_incomplete_dispatch_handoff_metadata() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: String::new(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("policy_gate"));
        assert!(error.contains("handoff metadata"));
    }

    #[test]
    fn validate_run_graph_resume_gate_rejects_resume_target_handoff_mismatch() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("resume_target"));
        assert!(error.contains("handoff_state"));
    }
}
