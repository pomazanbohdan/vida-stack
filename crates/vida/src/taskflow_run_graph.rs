use crate::{
    RenderMode, RuntimeConsumptionLaneSelection, build_runtime_execution_plan_from_snapshot,
    build_runtime_lane_selection_with_store, dispatch_contract_execution_lane_sequence,
    operator_contracts::canonical_release1_blocker_code_entries,
    print_surface_header, print_surface_line, read_or_sync_launcher_activation_snapshot,
    state_store::{
        RunGraphContinuationBinding, RunGraphDispatchReceipt, RunGraphStatus, StateStore,
        StateStoreError,
    },
    taskflow_layer4::print_taskflow_proxy_help,
    taskflow_task_bridge::proxy_state_dir,
};
use std::process::ExitCode;
use time::format_description::well_known::Rfc3339;

const STALE_PROJECTION_DISPATCH_TIMEOUT_SECONDS: i64 = 10;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct RecoveryNextAction {
    command: String,
    surface: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct RecoveryWhyNotNow {
    category: String,
    summary: String,
    blocker_codes: Vec<String>,
    blocking_surface: Option<String>,
}

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

#[derive(Debug, serde::Serialize)]
pub(crate) struct RunGraphProjectionTruth {
    pub(crate) projection_source: String,
    pub(crate) projection_reason: String,
    pub(crate) dispatch_receipt_present: bool,
    pub(crate) continuation_binding_present: bool,
    pub(crate) projection_vs_receipt_parity: String,
    pub(crate) stale_state_suspected: bool,
    pub(crate) next_lawful_operator_action: Option<String>,
    pub(crate) dispatch_receipt: Option<RunGraphDispatchReceipt>,
    pub(crate) continuation_binding: Option<RunGraphContinuationBinding>,
}

fn next_lawful_operator_action_for_status(status: &RunGraphStatus) -> Option<String> {
    if status.recovery_ready && status.resume_target != "none" {
        return Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            status.run_id
        ));
    }
    if status.status == "completed" {
        return None;
    }
    Some(format!(
        "vida taskflow run-graph status {} --json",
        status.run_id
    ))
}

fn recommended_surface_for_command(command: &str) -> String {
    if command.starts_with("vida taskflow consume continue") {
        return "vida taskflow consume continue".to_string();
    }
    if command.starts_with("vida taskflow recovery latest") {
        return "vida taskflow recovery latest".to_string();
    }
    if command.starts_with("vida taskflow run-graph status") {
        return "vida taskflow run-graph status".to_string();
    }
    command
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

fn recovery_surface_contract(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
) -> (
    Vec<String>,
    Option<RecoveryWhyNotNow>,
    Option<RecoveryNextAction>,
    Option<String>,
    Option<String>,
) {
    let blocker_codes = summary
        .delegation_gate
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![value.to_string()])
        .unwrap_or_default();
    let blocker_codes = crate::operator_contracts::normalize_blocker_codes(
        &blocker_codes,
        crate::release_contract_adapters::canonical_blocker_codes,
        None,
    );

    let next_action = projection_truth
        .next_lawful_operator_action
        .as_deref()
        .map(|command| RecoveryNextAction {
            command: command.to_string(),
            surface: recommended_surface_for_command(command),
            reason: if summary.recovery_ready {
                "recovery is ready; continue the lawful delegated chain".to_string()
            } else {
                "inspect the authoritative run-graph status for the bound recovery state"
                    .to_string()
            },
        });
    let why_not_now = (!blocker_codes.is_empty()).then(|| RecoveryWhyNotNow {
        category: "delegated_cycle_runtime_gate".to_string(),
        summary: format!(
            "The delegated cycle remains open in recovery state `{}`.",
            summary.delegation_gate.delegated_cycle_state
        ),
        blocker_codes: blocker_codes.clone(),
        blocking_surface: Some("vida taskflow recovery latest".to_string()),
    });
    let recommended_command = next_action.as_ref().map(|value| value.command.clone());
    let recommended_surface = next_action.as_ref().map(|value| value.surface.clone());

    (
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn projection_vs_receipt_parity(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return "no_receipt".to_string();
    };
    if receipt.dispatch_status == status.status
        || receipt.downstream_dispatch_status.as_deref() == Some(status.status.as_str())
    {
        return "aligned".to_string();
    }
    "reconciled_from_receipt".to_string()
}

fn projection_reason_for_status(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
    binding: Option<&RunGraphContinuationBinding>,
) -> String {
    if let Some(receipt) = receipt {
        if receipt.dispatch_status != status.status
            || receipt.downstream_dispatch_status.as_deref() == Some(status.status.as_str())
        {
            return "run-graph status was reconciled against persisted dispatch receipt evidence"
                .to_string();
        }
        if receipt.blocker_code.is_some() || !receipt.downstream_dispatch_blockers.is_empty() {
            return "run-graph status reflects persisted dispatch blocker evidence".to_string();
        }
    }
    if let Some(binding) = binding {
        return format!(
            "run-graph status is paired with explicit continuation binding from `{}`",
            binding.binding_source
        );
    }
    if status.status == "completed" {
        return "run-graph status reflects terminal state without additional projection inputs"
            .to_string();
    }
    "run-graph status reflects authoritative persisted state".to_string()
}

fn projection_stale_state_suspected(receipt: Option<&RunGraphDispatchReceipt>) -> bool {
    let Some(receipt) = receipt else {
        return false;
    };
    if receipt.dispatch_status != "executing" {
        return false;
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return false;
    };
    if result["execution_state"].as_str() != Some("executing") {
        return false;
    }
    let Some(recorded_at) = result["recorded_at"].as_str() else {
        return false;
    };
    let Ok(recorded_at) = time::OffsetDateTime::parse(recorded_at, &Rfc3339) else {
        return false;
    };
    let age_seconds = (time::OffsetDateTime::now_utc() - recorded_at).whole_seconds();
    age_seconds > STALE_PROJECTION_DISPATCH_TIMEOUT_SECONDS
}

pub(crate) async fn run_graph_projection_truth(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<RunGraphProjectionTruth, StateStoreError> {
    let dispatch_receipt = store.run_graph_dispatch_receipt(&status.run_id).await?;
    let continuation_binding = store.run_graph_continuation_binding(&status.run_id).await?;
    let stale_state_suspected = projection_stale_state_suspected(dispatch_receipt.as_ref());
    Ok(RunGraphProjectionTruth {
        projection_source: if dispatch_receipt.is_some() {
            "reconciled_run_graph_status".to_string()
        } else {
            "persisted_run_graph_status".to_string()
        },
        projection_reason: projection_reason_for_status(
            status,
            dispatch_receipt.as_ref(),
            continuation_binding.as_ref(),
        ),
        dispatch_receipt_present: dispatch_receipt.is_some(),
        continuation_binding_present: continuation_binding.is_some(),
        projection_vs_receipt_parity: projection_vs_receipt_parity(
            status,
            dispatch_receipt.as_ref(),
        ),
        stale_state_suspected,
        next_lawful_operator_action: next_lawful_operator_action_for_status(status),
        dispatch_receipt,
        continuation_binding,
    })
}

#[derive(Clone)]
struct CompiledRunGraphControl {
    implementation: serde_json::Value,
    verification: serde_json::Value,
    first_execution_lane: String,
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
    let first_execution_lane = dispatch_contract_execution_lane_sequence(
        &execution_plan["development_flow"]["dispatch_contract"],
    )
    .into_iter()
    .next()
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "implementer".to_string());
    if implementation.is_null() {
        return Err(
            "run-graph control is unavailable in the compiled activation snapshot.".to_string(),
        );
    }

    Ok(CompiledRunGraphControl {
        implementation,
        verification,
        first_execution_lane,
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
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!("Failed to build recovery projection truth: {error}");
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for projection truth: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract(&summary, &projection_truth);
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if !blocker_codes.is_empty() {
                            print_surface_line(
                                RenderMode::Plain,
                                "blocker_codes",
                                &blocker_codes.join(", "),
                            );
                        }
                        if let Some(summary) =
                            why_not_now.as_ref().map(|value| value.summary.as_str())
                        {
                            print_surface_line(RenderMode::Plain, "why_not_now", summary);
                        }
                        if let Some(next_action) = next_action.as_ref() {
                            print_surface_line(
                                RenderMode::Plain,
                                "next action",
                                &next_action.reason,
                            );
                        }
                        if let Some(command) = recommended_command.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_command", command);
                        }
                        if let Some(surface) = recommended_surface.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                        }
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
                        let projection_truth = match summary.as_ref() {
                            Some(summary) => match store.run_graph_status(&summary.run_id).await {
                                Ok(status) => {
                                    match run_graph_projection_truth(&store, &status).await {
                                        Ok(truth) => Some(truth),
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to build recovery projection truth: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read run-graph status for projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            },
                            None => None,
                        };
                        let contract = summary.as_ref().zip(projection_truth.as_ref()).map(
                            |(summary, projection_truth)| {
                                recovery_surface_contract(summary, projection_truth)
                            },
                        );
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery latest",
                                "blocker_codes": contract.as_ref().map(|value| value.0.clone()).unwrap_or_default(),
                                "why_not_now": contract.as_ref().and_then(|value| value.1.clone()),
                                "next_action": contract.as_ref().and_then(|value| value.2.clone()),
                                "recommended_command": contract.as_ref().and_then(|value| value.3.clone()),
                                "recommended_surface": contract.as_ref().and_then(|value| value.4.clone()),
                                "recovery": summary,
                                "projection_truth": projection_truth,
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
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!("Failed to build recovery projection truth: {error}");
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for projection truth: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract(&summary, &projection_truth);
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery status");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if !blocker_codes.is_empty() {
                            print_surface_line(
                                RenderMode::Plain,
                                "blocker_codes",
                                &blocker_codes.join(", "),
                            );
                        }
                        if let Some(summary) =
                            why_not_now.as_ref().map(|value| value.summary.as_str())
                        {
                            print_surface_line(RenderMode::Plain, "why_not_now", summary);
                        }
                        if let Some(next_action) = next_action.as_ref() {
                            print_surface_line(
                                RenderMode::Plain,
                                "next action",
                                &next_action.reason,
                            );
                        }
                        if let Some(command) = recommended_command.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_command", command);
                        }
                        if let Some(surface) = recommended_surface.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                        }
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
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!("Failed to build recovery projection truth: {error}");
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for projection truth: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract(&summary, &projection_truth);
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery status",
                                "run_id": summary.run_id,
                                "blocker_codes": blocker_codes,
                                "why_not_now": why_not_now,
                                "next_action": next_action,
                                "recommended_command": recommended_command,
                                "recommended_surface": recommended_surface,
                                "recovery": summary,
                                "projection_truth": projection_truth,
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
                        let projection_truth =
                            match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!(
                                        "Failed to build latest run-graph projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if let Some(next_action) =
                            projection_truth.next_lawful_operator_action.as_deref()
                        {
                            print_surface_line(RenderMode::Plain, "next action", next_action);
                        }
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
                        let projection_truth = match status.as_ref() {
                            Some(status) => {
                                match run_graph_projection_truth(&store, status).await {
                                    Ok(truth) => Some(truth),
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to build latest run-graph projection truth: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            None => None,
                        };
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph latest",
                                "status": status,
                                "delegation_gate": delegation_gate,
                                "projection_truth": projection_truth,
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
                        let projection_truth = match run_graph_projection_truth(&store, &status)
                            .await
                        {
                            Ok(truth) => truth,
                            Err(error) => {
                                eprintln!("Failed to build run-graph projection truth: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if let Some(next_action) =
                            projection_truth.next_lawful_operator_action.as_deref()
                        {
                            print_surface_line(RenderMode::Plain, "next action", next_action);
                        }
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
                        let projection_truth = match run_graph_projection_truth(&store, &status)
                            .await
                        {
                            Ok(truth) => truth,
                            Err(error) => {
                                eprintln!("Failed to build run-graph projection truth: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph status",
                                "run_id": status.run_id,
                                "status": status,
                                "delegation_gate": delegation_gate,
                                "projection_truth": projection_truth,
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
    let canonical_blocker_codes = canonical_release1_blocker_code_entries(&serde_json::json!([
        blocker_code
    ]))
    .ok_or_else(|| {
        format!(
            "run-graph blocker code `{blocker_code}` is not canonical (must be lowercase/digits/_)"
        )
    })?;
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

pub(crate) fn approval_delegation_transition_kind(status: &RunGraphStatus) -> Option<&'static str> {
    let route_bound_implementation =
        status.task_class == "implementation" && status.route_task_class == "implementation";

    if route_bound_implementation
        && status.status == "awaiting_approval"
        && status.lifecycle_stage == "approval_wait"
        && status.policy_gate
            == crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str()
        && matches!(status.next_node.as_deref(), Some("approval"))
        && status.handoff_state == "awaiting_approval"
        && status.resume_target == "dispatch.approval"
    {
        return Some("approval_wait");
    }

    if route_bound_implementation
        && status.status == "completed"
        && status.lifecycle_stage == "implementation_complete"
        && status.policy_gate == "not_required"
        && status.next_node.is_none()
        && status.handoff_state == "none"
        && status.resume_target == "none"
    {
        return Some("approval_complete");
    }

    None
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
    let lane_node = if is_conversation {
        selection.selected_role.clone()
    } else if selection.selected_role == "worker" {
        dispatch_contract_execution_lane_sequence(
            &execution_plan["development_flow"]["dispatch_contract"],
        )
        .into_iter()
        .next()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            json_string_field(route, "analysis_route_task_class").filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| selection.selected_role.clone())
    } else {
        json_string_field(route, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| selection.selected_role.clone())
    };
    let selected_backend =
        crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
            execution_plan,
            if is_conversation {
                lane_node.as_str()
            } else {
                "implementer"
            },
            json_string_field(route, "activation_agent_type").as_deref(),
            None,
        )
        .unwrap_or_else(|| "unknown".to_string());
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

pub(crate) fn run_graph_dispatch_context_from_seed_payload(
    payload: &TaskflowRunGraphSeedPayload,
) -> crate::state_store::RunGraphDispatchContext {
    crate::state_store::RunGraphDispatchContext {
        run_id: payload.status.run_id.clone(),
        task_id: payload.status.task_id.clone(),
        request_text: payload.request_text.clone(),
        role_selection: serde_json::to_value(&payload.role_selection)
            .unwrap_or(serde_json::Value::Null),
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    }
}

async fn persist_seed_artifacts(
    store: &StateStore,
    payload: &TaskflowRunGraphSeedPayload,
) -> Result<(), String> {
    store
        .clear_run_graph_dispatch_receipt(&payload.status.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to clear stale dispatch receipt before reseeding run `{}`: {error}",
                payload.status.run_id
            )
        })?;
    store
        .record_run_graph_status(&payload.status)
        .await
        .map_err(|error| format!("Failed to record seeded run-graph state: {error}"))?;
    store
        .record_run_graph_dispatch_context(&run_graph_dispatch_context_from_seed_payload(payload))
        .await
        .map_err(|error| format!("Failed to record seeded dispatch context: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &payload.status,
        "run_graph_seed",
    )
    .await?;
    Ok(())
}

pub(crate) fn run_graph_dispatch_bootstrap_from_status(
    status: &RunGraphStatus,
) -> Result<serde_json::Value, String> {
    validate_run_graph_resume_gate(status)?;
    let latest_status = serde_json::to_value(status)
        .map_err(|error| format!("Failed to encode status: {error}"))?;
    Ok(serde_json::json!({
        "status": "dispatch_init_ready",
        "handoff_ready": true,
        "run_id": status.run_id,
        "latest_status": latest_status,
    }))
}

fn dispatch_command_from_packet_path(packet_path: &str) -> Result<Option<String>, String> {
    let body = std::fs::read_to_string(packet_path).map_err(|error| {
        format!("Failed to read rendered dispatch packet `{packet_path}`: {error}")
    })?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
        format!("Failed to decode rendered dispatch packet `{packet_path}`: {error}")
    })?;
    Ok(json
        .get("dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

async fn reseed_explicit_task_graph_binding_for_dispatch_init(
    store: &StateStore,
    requested_run_id: &str,
) -> Result<Option<String>, String> {
    let binding = store
        .run_graph_continuation_binding(requested_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read explicit continuation binding for `{requested_run_id}`: {error}"
            )
        })?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return Ok(None);
    }

    let bound_task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(binding.task_id.as_str());
    if bound_task_id == requested_run_id {
        return Ok(None);
    }

    let request_text = if let Some(request_text) = binding
        .request_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        request_text.to_string()
    } else if let Some(context) = store
        .run_graph_dispatch_context(requested_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read persisted seeded dispatch context for `{requested_run_id}` while reseeding explicit continuation binding: {error}"
            )
        })?
    {
        context.request_text
    } else {
        return Err(format!(
            "Run `{requested_run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but no persisted request text is available to reseed dispatch-init for the bound task."
        ));
    };

    let payload = derive_seeded_run_graph_status(store, bound_task_id, &request_text).await?;
    persist_seed_artifacts(store, &payload).await?;

    let why = format!(
        "Explicit continuation binding for run `{requested_run_id}` reseeded bounded task `{bound_task_id}` into a fresh dispatch-ready run."
    );
    if let Some(binding) = crate::taskflow_continuation::build_run_graph_continuation_binding(
        &payload.status,
        Some(&request_text),
        "explicit_continuation_bind",
        Some(&why),
    ) {
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .map_err(|error| {
                format!(
                    "Failed to record reseeded explicit continuation binding for `{bound_task_id}`: {error}"
                )
            })?;
    }

    Ok(Some(bound_task_id.to_string()))
}

async fn run_graph_dispatch_init(
    store: &StateStore,
    run_id: &str,
) -> Result<serde_json::Value, String> {
    let effective_run_id = reseed_explicit_task_graph_binding_for_dispatch_init(store, run_id)
        .await?
        .unwrap_or_else(|| run_id.to_string());
    let status = store
        .run_graph_status(&effective_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read run-graph state for `{}`: {error}",
                effective_run_id
            )
        })?;
    let context = store
        .run_graph_dispatch_context(&effective_run_id)
        .await
        .map_err(|error| format!("Failed to read persisted seeded dispatch context: {error}"))?
        .ok_or_else(|| {
            format!(
                "No persisted seeded dispatch context exists for run_id `{}`; reseed the run with request text before dispatch-init.",
                effective_run_id
            )
        })?;
    let role_selection = context
        .role_selection()
        .map_err(|error| format!("Failed to decode persisted seeded dispatch context: {error}"))?;
    let run_graph_bootstrap = run_graph_dispatch_bootstrap_from_status(&status)?;
    let taskflow_handoff_plan = crate::build_taskflow_handoff_plan(&role_selection);
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    dispatch_receipt.dispatch_command = crate::runtime_dispatch_command_for_target(
        &role_selection,
        &dispatch_receipt.dispatch_target,
    );
    crate::refresh_downstream_dispatch_preview(
        store,
        &role_selection,
        &run_graph_bootstrap,
        &mut dispatch_receipt,
    )
    .await?;
    let ctx = crate::RuntimeDispatchPacketContext::new(
        store.root(),
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    );
    let dispatch_packet_path = crate::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    dispatch_receipt.dispatch_command = dispatch_command_from_packet_path(&dispatch_packet_path)?;
    // Refresh the run-graph status timestamps before persisting the receipt so every
    // latest_* projection resolves the same run after dispatch-init.
    store
        .record_run_graph_status(&status)
        .await
        .map_err(|error| {
            format!("Failed to refresh run-graph status for dispatch-init: {error}")
        })?;
    store
        .record_run_graph_dispatch_receipt(&dispatch_receipt)
        .await
        .map_err(|error| format!("Failed to record seeded dispatch receipt: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &status,
        "run_graph_dispatch_init",
    )
    .await?;
    Ok(serde_json::json!({
        "surface": "vida taskflow run-graph dispatch-init",
        "requested_run_id": run_id,
        "run_id": effective_run_id,
        "dispatch_receipt": dispatch_receipt,
        "dispatch_packet_path": dispatch_packet_path,
        "downstream_dispatch_packet_path": dispatch_receipt.downstream_dispatch_packet_path,
        "taskflow_handoff_plan": taskflow_handoff_plan,
        "run_graph_bootstrap": run_graph_bootstrap,
    }))
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
        let direct_writer_entry = compiled_control.first_execution_lane.clone();
        if existing.next_node.as_deref() == Some(direct_writer_entry.as_str()) {
            let coach_required =
                json_bool_field(&implementation, "coach_required").unwrap_or(false);
            let verification = compiled_control.verification.clone();
            let (next_node, policy_gate) =
                implementation_verification_gate(&implementation, &verification);
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_transition(
                    &existing,
                    RunGraphTransitionArgs {
                        active_node: direct_writer_entry.clone(),
                        next_node: if coach_required {
                            json_string_field(&implementation, "coach_route_task_class")
                                .filter(|value| !value.is_empty())
                                .or(next_node)
                        } else {
                            next_node
                        },
                        lane_id: format!("{direct_writer_entry}_lane"),
                        lifecycle_stage: "writer_active".to_string(),
                        policy_gate,
                        checkpoint_kind: "execution_cursor".to_string(),
                        target_format: DispatchTargetFormat::Lane,
                        recovery_ready: true,
                    },
                ),
            });
        }

        if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{analysis_node}` or `{direct_writer_entry}` for the seeded implementation run, got `{}`",
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
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &payload.status,
                            "run_graph_advance",
                        )
                        .await
                    {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
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
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &payload.status,
                            "run_graph_advance",
                        )
                        .await
                    {
                        let message = format!(
                            "Failed to synchronize continuation binding after advance: {error}"
                        );
                        eprintln!("{message}");
                        print_run_graph_json_error(
                            "vida taskflow run-graph advance",
                            task_id,
                            &message,
                            None,
                        );
                        return ExitCode::from(1);
                    }
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
            match persist_seed_artifacts(&store, &payload).await {
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
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            match run_graph_dispatch_init(&store, run_id).await {
                Ok(payload) => {
                    print_surface_header(
                        RenderMode::Plain,
                        "vida taskflow run-graph dispatch-init",
                    );
                    print_surface_line(RenderMode::Plain, "run", run_id);
                    print_surface_line(
                        RenderMode::Plain,
                        "dispatch_packet",
                        payload["dispatch_packet_path"].as_str().unwrap_or("none"),
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "dispatch_target",
                        payload["dispatch_receipt"]["dispatch_target"]
                            .as_str()
                            .unwrap_or("none"),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            match run_graph_dispatch_init(&store, run_id).await {
                Ok(payload) => {
                    crate::print_json_pretty(&payload);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    print_run_graph_json_error(
                        "vida taskflow run-graph dispatch-init",
                        run_id,
                        &error,
                        None,
                    );
                    eprintln!("{error}");
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
        [
            head,
            subcommand,
            task_id,
            task_class,
            node,
            status,
            route_task_class,
        ] if head == "run-graph" && subcommand == "update" => {
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
        [
            head,
            subcommand,
            task_id,
            task_class,
            node,
            status,
            route_task_class,
            meta_json,
        ] if head == "run-graph" && subcommand == "update" => {
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
        [head, subcommand, ..] if head == "run-graph" && subcommand == "dispatch-init" => {
            eprintln!("Usage: vida taskflow run-graph dispatch-init <task_id> [--json]");
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
    use crate::RuntimeConsumptionLaneSelection;
    use crate::build_compiled_agent_extension_bundle_for_root;
    use crate::launcher_activation_snapshot::config_file_digest;
    use crate::launcher_activation_snapshot::pack_router_keywords_json;
    use crate::runtime_dispatch_state::load_project_overlay_yaml_for_root;
    use crate::state_store::LauncherActivationSnapshot;
    use crate::temp_state::TempStateHarness;
    use serde_json::json;
    use std::path::Path;

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
    fn recovery_surface_contract_aligns_next_surface_vocabulary() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "planning".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            resume_node: Some("analysis".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.analysis_lane".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_analysis".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "planning".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "implementation_dispatch_ready".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "persisted_run_graph_status".to_string(),
            projection_reason: "paired with continuation binding".to_string(),
            dispatch_receipt_present: false,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "no_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-1 --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert_eq!(blocker_codes, vec!["open_delegated_cycle".to_string()]);
        assert_eq!(
            why_not_now.as_ref().map(|value| value.category.as_str()),
            Some("delegated_cycle_runtime_gate")
        );
        assert_eq!(
            next_action.as_ref().map(|value| value.surface.as_str()),
            Some("vida taskflow consume continue")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida taskflow consume continue --run-id run-1 --json")
        );
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida taskflow consume continue")
        );
    }

    async fn write_activation_snapshot_for_store(store: &StateStore) -> Result<(), String> {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let config = load_project_overlay_yaml_for_root(&project_root)?;
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, &project_root)
            .map_err(|error| format!("build compiled bundle: {error}"))?;
        let pack_router = pack_router_keywords_json(&config);
        let snapshot = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: project_root.join("vida.config.yaml").display().to_string(),
            source_config_digest: config_file_digest(&project_root.join("vida.config.yaml"))
                .map_err(|error| format!("read config digest: {error}"))?,
            captured_at: "2026-01-01T00:00:00Z".to_string(),
            compiled_bundle: bundle,
            pack_router_keywords: pack_router,
        };
        store
            .write_launcher_activation_snapshot(&snapshot)
            .await
            .map_err(|error| format!("write launcher activation snapshot: {error}"))?;
        Ok(())
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_bound_repair_with_file_scope_terms() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_status(
            &store,
            "task-repair-seed-1",
            "Repair scope and specification drift in crates/vida/src/runtime_lane_summary.rs, fix the file, add regression tests, and prove test coverage.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_ne!(payload.status.next_node.as_deref(), Some("spec-pack"));
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_keeps_design_spec_request_in_scope_discussion() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_status(
            &store,
            "task-design-seed-1",
            "Research the feature scope, write the specification and acceptance criteria.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.status.task_class, "scope_discussion");
        assert!(payload.role_selection.conversational_mode.is_some());
        assert_ne!(payload.status.route_task_class, "implementation");
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

    #[tokio::test]
    async fn dispatch_init_materializes_first_persisted_dispatch_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let status = RunGraphStatus {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "running".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "open".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "worker".to_string(),
            request: "Implement one bounded patch in crates/vida/src/taskflow_run_graph.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "orchestration_contract": {},
                "runtime_assignment": {
                    "selected_agent_id": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                },
                "development_flow": {
                    "lane_sequence": ["implementer", "coach", "verification"],
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "closure_class": "implementation"
                            },
                            "coach": {
                                "activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "coach"
                                },
                                "closure_class": "review"
                            },
                            "verification": {
                                "activation": {
                                    "activation_agent_type": "architect",
                                    "activation_runtime_role": "verifier"
                                },
                                "closure_class": "verification"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run status");
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: "task-1".to_string(),
                task_id: "task-1".to_string(),
                request_text: role_selection.request.clone(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("role selection should encode"),
                recorded_at: "2026-04-10T10:00:00Z".to_string(),
            })
            .await
            .expect("record dispatch context");

        let payload = run_graph_dispatch_init(&store, "task-1")
            .await
            .expect("dispatch init should succeed");
        let receipt = store
            .run_graph_dispatch_receipt("task-1")
            .await
            .expect("read receipt")
            .expect("receipt present");

        assert_eq!(receipt.dispatch_target, "implementer");
        assert!(receipt.dispatch_packet_path.is_some());
        assert!(payload["dispatch_packet_path"].as_str().is_some());
    }

    #[tokio::test]
    async fn reseed_clears_stale_blocked_dispatch_receipt_before_dispatch_init() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let stale_status = RunGraphStatus {
            run_id: "task-reseed-1".to_string(),
            task_id: "task-reseed-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "qwen_cli".to_string(),
            lane_id: "implementer_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer_lane".to_string(),
            recovery_ready: false,
        };
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale run status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "task-reseed-1".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("qwen".to_string()),
                dispatch_packet_path: Some("/tmp/stale-dispatch-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/stale-dispatch-result.json".to_string()),
                blocker_code: Some("stale_receipt".to_string()),
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale downstream note".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("qwen_cli".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale dispatch receipt");

        let payload = derive_seeded_run_graph_status(
            &store,
            "task-reseed-1",
            "Fix the exact in-process test hang in runtime_dispatch_state by removing nested EnvVarGuard acquisition and preserving harness-local state isolation. Owned paths: crates/vida/src/runtime_dispatch_state.rs.",
        )
        .await
        .expect("seed should be generated");
        assert_eq!(payload.status.selected_backend, "opencode_cli");
        assert!(payload.status.recovery_ready);

        persist_seed_artifacts(&store, &payload)
            .await
            .expect("persist seeded artifacts should succeed");

        let reconciled = store
            .run_graph_status("task-reseed-1")
            .await
            .expect("reseeded run status should load");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
        assert!(reconciled.recovery_ready);

        assert!(
            store
                .run_graph_dispatch_receipt("task-reseed-1")
                .await
                .expect("dispatch receipt lookup should succeed")
                .is_none(),
            "fresh reseed should clear stale pre-dispatch receipt lineage"
        );

        let dispatch_init = run_graph_dispatch_init(&store, "task-reseed-1")
            .await
            .expect("dispatch init should succeed after reseed");
        assert_eq!(
            dispatch_init["dispatch_receipt"]["selected_backend"].as_str(),
            Some("opencode_cli")
        );
    }

    #[tokio::test]
    async fn dispatch_init_reseeds_explicit_task_graph_binding_into_bound_task_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let mut stale_status = default_run_graph_status("run-old", "closure", "delivery");
        stale_status.task_id = "run-old".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.policy_gate = "validation_report_required".to_string();
        stale_status.context_state = "sealed".to_string();
        stale_status.checkpoint_kind = "execution_cursor".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = true;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-old".to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": "run-old",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "reseed onto task-new".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("Fix the runtime bridge for explicit task bindings in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/taskflow_packet.rs.".to_string()),
                    recorded_at: "2026-04-16T09:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit binding");

        let payload = run_graph_dispatch_init(&store, "run-old")
            .await
            .expect("dispatch init should reseed and succeed");

        assert_eq!(payload["requested_run_id"], "run-old");
        assert_eq!(payload["run_id"], "task-new");
        assert_eq!(payload["dispatch_receipt"]["run_id"], "task-new");

        let reseeded_status = store
            .run_graph_status("task-new")
            .await
            .expect("reseeded task run should exist");
        assert_eq!(reseeded_status.task_id, "task-new");
        assert_eq!(reseeded_status.run_id, "task-new");
        assert!(
            matches!(reseeded_status.status.as_str(), "ready" | "blocked"),
            "unexpected reseeded status: {}",
            reseeded_status.status
        );

        let reseeded_receipt = store
            .run_graph_dispatch_receipt("task-new")
            .await
            .expect("reseeded receipt lookup should succeed")
            .expect("reseeded receipt should exist");
        assert_eq!(reseeded_receipt.run_id, "task-new");
        assert!(reseeded_receipt.dispatch_packet_path.is_some());
    }

    #[tokio::test]
    async fn dispatch_init_refreshes_latest_run_graph_surfaces_to_effective_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let target_status = RunGraphStatus {
            run_id: "task-refresh-latest".to_string(),
            task_id: "task-refresh-latest".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("business_analyst".to_string()),
            status: "ready".to_string(),
            route_task_class: "spec-pack".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "dispatch_ready".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_business_analyst".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.business_analyst_lane".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&target_status)
            .await
            .expect("persist target run status");
        store
            .record_run_graph_dispatch_context(&run_graph_dispatch_context_from_seed_payload(
                &TaskflowRunGraphSeedPayload {
                    status: target_status.clone(),
                    request_text: "Repair fail-closed resume latest projection drift in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/state_store_run_graph_summary.rs.".to_string(),
                    role_selection: RuntimeConsumptionLaneSelection {
                        ok: true,
                        activation_source: "test".to_string(),
                        selection_mode: "fixed".to_string(),
                        fallback_role: "business_analyst".to_string(),
                        request: "Repair fail-closed resume latest projection drift in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/state_store_run_graph_summary.rs.".to_string(),
                        selected_role: "business_analyst".to_string(),
                        conversational_mode: None,
                        single_task_only: true,
                        tracked_flow_entry: Some("spec-pack".to_string()),
                        allow_freeform_chat: false,
                        confidence: "high".to_string(),
                        matched_terms: vec!["repair".to_string(), "resume".to_string()],
                        compiled_bundle: serde_json::Value::Null,
                        execution_plan: serde_json::json!({
                            "runtime_assignment": {
                                "selected_agent_id": "middle",
                                "activation_agent_type": "middle",
                                "activation_runtime_role": "business_analyst"
                            }
                        }),
                        reason: "test".to_string(),
                    },
                },
            ))
            .await
            .expect("persist target dispatch context");

        let mut stale_status = default_run_graph_status("run-stale-latest", "closure", "delivery");
        stale_status.task_id = "run-stale-latest".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.context_state = "sealed".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = false;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale latest run status");

        let payload = run_graph_dispatch_init(&store, "task-refresh-latest")
            .await
            .expect("dispatch init should succeed");
        assert_eq!(payload["run_id"], "task-refresh-latest");

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.run_id, "task-refresh-latest");

        let latest_recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("load latest recovery summary")
            .expect("latest recovery summary should exist");
        assert_eq!(latest_recovery.run_id, "task-refresh-latest");

        let latest_receipt = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary")
            .expect("latest dispatch receipt summary should exist");
        assert_eq!(latest_receipt.run_id, "task-refresh-latest");
    }

    #[tokio::test]
    async fn seeded_worker_run_can_advance_directly_into_implementer_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let existing = RunGraphStatus {
            run_id: "task-direct-implementer".to_string(),
            task_id: "task-direct-implementer".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let payload = derive_advanced_run_graph_status(&store, existing)
            .await
            .expect("seeded implementer run should advance");

        assert_eq!(payload.status.active_node, "implementer");
        assert_eq!(payload.status.lifecycle_stage, "writer_active");
        assert_eq!(payload.status.next_node.as_deref(), Some("coach"));
        assert_eq!(payload.status.handoff_state, "awaiting_coach");
    }

    #[test]
    fn implementation_verification_outcome_defaults_for_unexpected_status() {
        assert_eq!(
            implementation_verification_outcome("paused"),
            ImplementationVerificationOutcome::UnexpectedStatus
        );
    }

    #[test]
    fn approval_delegation_transition_kind_requires_route_bound_receipt_shape() {
        let mut awaiting_approval =
            default_run_graph_status("run-1", "implementation", "implementation");
        awaiting_approval.status = "awaiting_approval".to_string();
        awaiting_approval.active_node = "verification".to_string();
        awaiting_approval.next_node = Some("approval".to_string());
        awaiting_approval.lifecycle_stage = "approval_wait".to_string();
        awaiting_approval.policy_gate = crate::release1_contracts::ApprovalStatus::ApprovalRequired
            .as_str()
            .to_string();
        awaiting_approval.handoff_state = "awaiting_approval".to_string();
        awaiting_approval.resume_target = "dispatch.approval".to_string();

        assert_eq!(
            approval_delegation_transition_kind(&awaiting_approval),
            Some("approval_wait")
        );

        let mut completed = default_run_graph_status("run-1", "implementation", "implementation");
        completed.active_node = "verification".to_string();
        completed.status = "completed".to_string();
        completed.next_node = None;
        completed.lifecycle_stage = "implementation_complete".to_string();
        completed.policy_gate = "not_required".to_string();
        completed.handoff_state = "none".to_string();
        completed.resume_target = "none".to_string();

        assert_eq!(
            approval_delegation_transition_kind(&completed),
            Some("approval_complete")
        );

        let mut unstructured = completed;
        unstructured.status = "approved".to_string();
        assert_eq!(approval_delegation_transition_kind(&unstructured), None);
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

    #[test]
    fn projection_reason_prefers_persisted_dispatch_blocker_evidence() {
        let status = RunGraphStatus {
            run_id: "run-projection-1".to_string(),
            task_id: "task-projection-1".to_string(),
            task_class: "scope_discussion".to_string(),
            active_node: "specification".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "spec-pack".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "specification_lane".to_string(),
            lifecycle_stage: "specification_active".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/projection-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            projection_reason_for_status(&status, Some(&receipt), None),
            "run-graph status reflects persisted dispatch blocker evidence"
        );
        assert_eq!(
            projection_vs_receipt_parity(&status, Some(&receipt)),
            "aligned"
        );
    }

    #[test]
    fn projection_stale_state_suspected_for_old_executing_dispatch_result() {
        let root = std::env::temp_dir().join(format!(
            "vida-projection-stale-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let result_path = root.join("dispatch-result.json");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "execution_state": "executing",
                "recorded_at": "2026-04-18T00:00:00Z"
            }))
            .expect("dispatch result should encode"),
        )
        .expect("dispatch result should write");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-projection-stale".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(projection_stale_state_suspected(Some(&receipt)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn next_lawful_operator_action_prefers_continue_for_recovery_ready_status() {
        let status = RunGraphStatus {
            run_id: "run-projection-continue".to_string(),
            task_id: "task-projection-continue".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("verification".to_string()),
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_verification".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "artifact".to_string(),
            resume_target: "dispatch.verification_lane".to_string(),
            recovery_ready: true,
        };

        assert_eq!(
            next_lawful_operator_action_for_status(&status).as_deref(),
            Some("vida taskflow consume continue --run-id run-projection-continue --json")
        );
    }
}
