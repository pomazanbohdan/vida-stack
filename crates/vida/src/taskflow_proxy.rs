use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use crate::taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use crate::taskflow_spec_bootstrap::run_taskflow_bootstrap_spec;
use crate::taskflow_task_bridge::{enforce_execution_preparation_contract_gate, proxy_state_dir};
use crate::{
    print_surface_header, print_surface_line, surface_render, taskflow_consume,
    taskflow_protocol_binding, Command, ProxyArgs, RenderMode, TaskCommand, TaskReadyArgs,
};
use clap::{CommandFactory, Parser};
use serde::Serialize;
use taskflow_cli::Cli as TaskflowCli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct GraphSummaryTaskRef {
    pub(crate) id: String,
    pub(crate) display_id: Option<String>,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) priority: u32,
    pub(crate) issue_type: String,
    pub(crate) conflict_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GraphSummaryWaveBucket {
    wave_id: String,
    task_count: usize,
    ready_count: usize,
    blocked_count: usize,
    primary_ready_task: Option<GraphSummaryTaskRef>,
    primary_blocked_task: Option<GraphSummaryTaskRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextAction {
    command: String,
    surface: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextWhyNotNow {
    category: String,
    summary: String,
    blocker_codes: Vec<String>,
    blocking_surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextCandidateContext {
    ready_head: Option<GraphSummaryTaskRef>,
    admissible_now: bool,
    admissibility_gate: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextDecision {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
    primary_ready_task: Option<GraphSummaryTaskRef>,
    candidate_task_context: TaskflowNextCandidateContext,
    why_not_now: Option<TaskflowNextWhyNotNow>,
    next_action: Option<TaskflowNextAction>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TaskflowSchedulerRejectedCandidate {
    pub(crate) task_id: String,
    pub(crate) task: GraphSummaryTaskRef,
    pub(crate) ready_now: bool,
    pub(crate) active_critical_path: bool,
    pub(crate) conflict_domain: Option<String>,
    pub(crate) reasons: Vec<String>,
    pub(crate) blocked_by: Vec<crate::state_store::TaskDependencyStatus>,
    pub(crate) parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TaskflowSchedulerReservationPreview {
    pub(crate) reservation_id: String,
    pub(crate) task_id: String,
    pub(crate) task: GraphSummaryTaskRef,
    pub(crate) launch_role: String,
    pub(crate) launch_index: usize,
    pub(crate) conflict_domain: Option<String>,
    pub(crate) command: String,
    pub(crate) state_dir: String,
    pub(crate) reservation_status: String,
    pub(crate) reservation_persisted: bool,
    pub(crate) execute_supported: bool,
    pub(crate) execution_attempted: bool,
    pub(crate) execute_status: String,
    pub(crate) activation_attempt_supported: bool,
    pub(crate) activation_attempted: bool,
    pub(crate) activation_status: String,
    pub(crate) activation_blocker_codes: Vec<String>,
    pub(crate) worker_execution_evidence_status: String,
    pub(crate) worker_completion_claimed: bool,
    pub(crate) receipt_id: Option<String>,
    pub(crate) receipt_path: Option<String>,
    pub(crate) preview_only_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TaskflowSchedulerDispatchReceiptPreview {
    receipt_id: Option<String>,
    receipt_path: Option<String>,
    receipt_status: String,
    receipt_persisted: bool,
    dispatch_surface: String,
    dispatch_command: String,
    dispatch_status: String,
    execute_requested: bool,
    execute_supported: bool,
    execution_attempted: bool,
    execute_status: String,
    activation_attempt_supported: bool,
    activation_attempted: bool,
    activation_status: String,
    activation_blocker_codes: Vec<String>,
    worker_execution_evidence_status: String,
    worker_completion_claimed: bool,
    packet_backed_execution_supported: bool,
    packet_backed_execution_status: String,
    packet_backed_execution_gate:
        Option<crate::taskflow_run_graph::RunGraphPacketBackedExecutionGate>,
    preview_only_reason: Option<String>,
    execution_blocker_codes: Vec<String>,
    selected_task_ids: Vec<String>,
    reservation_ids: Vec<String>,
    blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TaskflowSchedulerDispatchPlan {
    pub(crate) status: String,
    pub(crate) surface: String,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) dry_run: bool,
    pub(crate) execute_requested: bool,
    pub(crate) execute_supported: bool,
    pub(crate) execution_attempted: bool,
    pub(crate) execution_status: String,
    pub(crate) activation_attempt_supported: bool,
    pub(crate) activation_attempted: bool,
    pub(crate) activation_status: String,
    pub(crate) activation_blocker_codes: Vec<String>,
    pub(crate) worker_execution_evidence_status: String,
    pub(crate) worker_completion_claimed: bool,
    pub(crate) packet_backed_execution_supported: bool,
    pub(crate) packet_backed_execution_status: String,
    pub(crate) packet_backed_execution_gate:
        Option<crate::taskflow_run_graph::RunGraphPacketBackedExecutionGate>,
    pub(crate) configured_max_parallel_agents: u64,
    pub(crate) requested_parallel_limit: Option<u64>,
    pub(crate) scope_task_id: Option<String>,
    pub(crate) requested_current_task_id: Option<String>,
    pub(crate) selected_current_task_id: Option<String>,
    pub(crate) selection_source: String,
    pub(crate) max_parallel_agents: u64,
    pub(crate) ready_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) selected_primary_task: Option<GraphSummaryTaskRef>,
    pub(crate) selected_parallel_tasks: Vec<GraphSummaryTaskRef>,
    pub(crate) selected_task_ids: Vec<String>,
    pub(crate) reservations: Vec<TaskflowSchedulerReservationPreview>,
    pub(crate) dispatch_receipt: TaskflowSchedulerDispatchReceiptPreview,
    pub(crate) rejected_candidates: Vec<TaskflowSchedulerRejectedCandidate>,
    pub(crate) scheduling: crate::state_store::TaskSchedulingProjection,
}

fn graph_summary_task_ref(task: &crate::state_store::TaskRecord) -> GraphSummaryTaskRef {
    GraphSummaryTaskRef {
        id: task.id.clone(),
        display_id: task.display_id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
        conflict_domain: task.execution_semantics.conflict_domain.clone(),
    }
}

fn recovery_holds_active_bound_run(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|summary| summary.delegation_gate.delegated_cycle_open)
}

fn latest_run_graph_status_blocks_normal_continuation(
    status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    status.is_some_and(|status| {
        let normalized = status.status.trim().to_ascii_lowercase();
        normalized == "blocked" || normalized == "lane_blocked" || normalized.ends_with("_blocked")
    })
}

fn authoritative_dispatch_blocker_codes(
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let Some(dispatch) = dispatch else {
        return Vec::new();
    };
    let mut blocker_codes = Vec::new();
    if let Some(blocker_code) = dispatch
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if matches!(dispatch.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(
            dispatch.lane_status.as_str(),
            "lane_blocked" | "lane_failed"
        )
    {
        blocker_codes.extend(
            dispatch
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes)
}

fn normalize_scheduler_max_parallel_agents(activation_bundle: &serde_json::Value) -> u64 {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn scheduler_effective_parallel_limit(configured: u64, requested: Option<u64>) -> u64 {
    let configured = configured.max(1);
    requested
        .filter(|value| *value > 0)
        .map(|value| configured.min(value))
        .unwrap_or(configured)
        .max(1)
}

fn scheduler_rejection_reasons_from_blocked_by(
    blocked_by: &[crate::state_store::TaskDependencyStatus],
) -> Vec<String> {
    if blocked_by.is_empty() {
        return vec!["graph_blocked".to_string()];
    }

    blocked_by
        .iter()
        .map(|dependency| {
            format!(
                "blocked_by:{}:{}:{}",
                dependency.edge_type, dependency.depends_on_id, dependency.dependency_status
            )
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchedulerRuntimeGateBlockerSignals {
    blocker_codes: Vec<String>,
    open_delegated_cycle: bool,
    active_reservation: bool,
}

fn scheduler_reservation_preview(
    task: &GraphSummaryTaskRef,
    launch_role: &str,
    launch_index: usize,
    state_dir: &std::path::Path,
    execute_requested: bool,
) -> TaskflowSchedulerReservationPreview {
    let execute_status = if execute_requested {
        "execute_projection_not_executed"
    } else {
        "preview_not_executed"
    };
    let preview_only_reason = if execute_requested {
        None
    } else {
        Some("scheduler_dispatch_is_preview_only".to_string())
    };
    TaskflowSchedulerReservationPreview {
        reservation_id: format!("scheduler-preview-{launch_role}-{launch_index}-{}", task.id),
        task_id: task.id.clone(),
        task: task.clone(),
        launch_role: launch_role.to_string(),
        launch_index,
        conflict_domain: task.conflict_domain.clone(),
        command: format!(
            "vida agent-init --role worker {} --state-dir {} --json",
            task.id,
            state_dir.display()
        ),
        state_dir: state_dir.display().to_string(),
        reservation_status: if execute_requested {
            "execute_projection_unpersisted"
        } else {
            "preview_unpersisted"
        }
        .to_string(),
        reservation_persisted: false,
        execute_supported: false,
        execution_attempted: false,
        execute_status: execute_status.to_string(),
        activation_attempt_supported: false,
        activation_attempted: false,
        activation_status: execute_status.to_string(),
        activation_blocker_codes: Vec::new(),
        worker_execution_evidence_status: "not_received".to_string(),
        worker_completion_claimed: false,
        receipt_id: None,
        receipt_path: None,
        preview_only_reason,
    }
}

fn scheduler_execute_runtime_gate_blocker_codes(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> SchedulerRuntimeGateBlockerSignals {
    let mut blocker_codes = Vec::new();
    let mut open_delegated_cycle = false;
    let mut active_reservation = false;

    if recovery_holds_active_bound_run(recovery) {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::OpenDelegatedCycle,
        ) {
            open_delegated_cycle = true;
            blocker_codes.push(code);
        }
    }

    if let Some(dispatch) = dispatch {
        if matches!(dispatch.dispatch_status.as_str(), "executing")
            || dispatch.lane_status == "lane_running"
        {
            active_reservation = true;
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::ExecutionPreparationGateBlocked,
            ) {
                blocker_codes.push(code);
            }
        }
        blocker_codes.extend(authoritative_dispatch_blocker_codes(Some(dispatch)));
    }

    SchedulerRuntimeGateBlockerSignals {
        blocker_codes: crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes),
        open_delegated_cycle,
        active_reservation,
    }
}

fn apply_scheduler_execute_runtime_gate_blockers(
    plan: &mut TaskflowSchedulerDispatchPlan,
    signals: &SchedulerRuntimeGateBlockerSignals,
) {
    if signals.blocker_codes.is_empty() {
        return;
    }

    let blocker_codes = &signals.blocker_codes;
    if blocker_codes.is_empty() {
        return;
    }

    let execution_status = blocker_codes
        .first()
        .cloned()
        .unwrap_or_else(|| "scheduler_execute_blocked".to_string());
    let is_noop_projection = plan.selected_task_ids.is_empty() && plan.execute_requested;

    for blocker in blocker_codes {
        if !plan.blocker_codes.iter().any(|value| value == blocker) {
            plan.blocker_codes.push(blocker.clone());
        }
        if !plan
            .dispatch_receipt
            .blocker_codes
            .iter()
            .any(|value| value == blocker)
        {
            plan.dispatch_receipt.blocker_codes.push(blocker.clone());
        }
    }

    plan.status = "blocked".to_string();
    if !is_noop_projection {
        plan.execution_status = execution_status.clone();
        plan.activation_status = execution_status.clone();
        plan.activation_blocker_codes = blocker_codes.to_vec();
        plan.dispatch_receipt.execute_status = execution_status.clone();
        plan.dispatch_receipt.activation_status = execution_status.clone();
        plan.dispatch_receipt.preview_only_reason = Some(execution_status);
        plan.dispatch_receipt.dispatch_status = "blocked".to_string();
        plan.dispatch_receipt.execution_blocker_codes = blocker_codes.to_vec();
        plan.dispatch_receipt.activation_blocker_codes = blocker_codes.to_vec();
        plan.execute_supported = true;
        plan.execution_attempted = false;
        plan.activation_attempt_supported = true;
        plan.activation_attempted = false;
        for reservation in &mut plan.reservations {
            reservation.preview_only_reason = Some(plan.dispatch_receipt.execute_status.clone());
            reservation.activation_status = plan.dispatch_receipt.execute_status.clone();
            reservation.activation_blocker_codes = blocker_codes.to_vec();
            reservation.activation_attempt_supported = true;
            reservation.activation_attempted = false;
        }
    }
    if signals.open_delegated_cycle {
        plan.next_actions.push(
            "Resolve the open delegated-cycle gate before scheduler execute by inspecting `vida taskflow recovery latest --json` and the active continuation state.".to_string(),
        );
    }
    if signals.active_reservation {
        plan.next_actions.push(
            "An active scheduler reservation is already running; resume or close it with `vida taskflow consume continue --json` before creating new execution reservations.".to_string(),
        );
    }
    if !signals.open_delegated_cycle && !signals.active_reservation {
        plan.next_actions.push(
            "Resolve scheduler dispatch blockers and retry after `vida taskflow recovery latest --json` reports a clear gate.".to_string(),
        );
    }
}

fn scheduler_dispatch_receipt_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("scheduler-dispatch-{nanos}")
}

fn scheduler_remove_blocker_codes(codes: &mut Vec<String>, remove: &[&str]) {
    codes.retain(|code| !remove.iter().any(|candidate| candidate == code));
}

fn explicit_task_graph_continuation_task_id(
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
) -> Option<&str> {
    let binding = binding?;
    if binding.status != "bound" || binding.binding_source != "explicit_continuation_bind_task" {
        return None;
    }
    if binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
        != Some("task_graph_task")
    {
        return None;
    }
    binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
        .or_else(|| {
            let task_id = binding.task_id.trim();
            (!task_id.is_empty()).then_some(task_id)
        })
}

async fn build_scheduler_packet_backed_execution_gate(
    store: &crate::state_store::StateStore,
    plan: &TaskflowSchedulerDispatchPlan,
) -> Result<crate::taskflow_run_graph::RunGraphPacketBackedExecutionGate, String> {
    let selected_task_id = if plan.selected_task_ids.len() == 1 {
        plan.selected_task_ids.first().map(String::as_str)
    } else {
        None
    };
    let Some(selected_task_id) = selected_task_id else {
        return Ok(
            crate::taskflow_run_graph::evaluate_run_graph_packet_backed_execution_gate(
                None, None, None, None, None,
            ),
        );
    };
    build_scheduler_packet_backed_execution_gate_for_task(store, selected_task_id).await
}

async fn build_scheduler_packet_backed_execution_gate_for_task(
    store: &crate::state_store::StateStore,
    selected_task_id: &str,
) -> Result<crate::taskflow_run_graph::RunGraphPacketBackedExecutionGate, String> {
    let mut status = match store.run_graph_status(selected_task_id).await {
        Ok(status) => Some(status),
        Err(_) => None,
    };
    if status.is_none() {
        let task = store.show_task(selected_task_id).await.map_err(|error| {
            format!(
                "Failed to read selected scheduler task `{selected_task_id}` for packet-backed run-graph seed: {error}"
            )
        })?;
        let request_text = scheduler_task_request_text(&task).ok_or_else(|| {
            format!(
                "Selected scheduler task `{selected_task_id}` has no request text for packet-backed run-graph seed."
            )
        })?;
        let mut payload = crate::taskflow_run_graph::derive_seeded_run_graph_status(
            store,
            selected_task_id,
            &request_text,
        )
        .await?;
        payload.status.recovery_ready = true;
        crate::taskflow_run_graph::persist_seed_artifacts(store, &payload).await?;
        status = Some(payload.status);
    }
    let context = match status.as_ref() {
        Some(status) => store
            .run_graph_dispatch_context(&status.run_id)
            .await
            .map_err(|error| {
                format!("Failed to read scheduler packet-backed dispatch context: {error}")
            })?,
        None => None,
    };
    let mut binding = match status.as_ref() {
        Some(status) => store
            .run_graph_continuation_binding(&status.run_id)
            .await
            .map_err(|error| {
                format!("Failed to read scheduler packet-backed continuation binding: {error}")
            })?,
        None => None,
    };
    if let Some(status) = status.as_ref() {
        let binding_task_id = binding.as_ref().and_then(|binding| {
            (binding.status == "bound"
                && binding.binding_source == "explicit_continuation_bind_task"
                && binding.active_bounded_unit["kind"].as_str() == Some("task_graph_task"))
            .then(|| {
                binding.active_bounded_unit["task_id"]
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(binding.task_id.as_str())
            })
        });
        if binding_task_id != Some(selected_task_id) {
            let task = store.show_task(selected_task_id).await.map_err(|error| {
                format!(
                    "Failed to read selected scheduler task `{selected_task_id}` for packet-backed continuation binding: {error}"
                )
            })?;
            let request_text = context
                .as_ref()
                .map(|context| context.request_text.as_str());
            let why = format!(
                "Scheduler execute materialized packet-backed dispatch for selected task `{selected_task_id}`."
            );
            let new_binding = crate::taskflow_continuation::build_task_graph_continuation_binding(
                &status.run_id,
                request_text,
                &task,
                Some(&why),
            )
            .ok_or_else(|| {
                format!(
                    "Selected scheduler task `{selected_task_id}` did not yield a packet-backed continuation binding."
                )
            })?;
            store
                .record_run_graph_continuation_binding(&new_binding)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to record scheduler packet-backed continuation binding: {error}"
                    )
                })?;
            binding = Some(new_binding);
        }
    }
    let mut receipt = match status.as_ref() {
        Some(status) => store
            .run_graph_dispatch_receipt(&status.run_id)
            .await
            .map_err(|error| {
                format!("Failed to read scheduler packet-backed dispatch receipt: {error}")
            })?,
        None => None,
    };
    if let Some(status) = status.as_ref() {
        let receipt_missing_packet = receipt.as_ref().map_or(true, |receipt| {
            receipt
                .dispatch_packet_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        });
        if receipt_missing_packet {
            let artifacts = crate::taskflow_run_graph::prepare_run_graph_dispatch_init_artifacts(
                store,
                &status.run_id,
            )
            .await?;
            receipt = Some(artifacts.dispatch_receipt);
        }
    }
    Ok(
        crate::taskflow_run_graph::evaluate_run_graph_packet_backed_execution_gate(
            Some(selected_task_id),
            status.as_ref(),
            context.as_ref(),
            binding.as_ref(),
            receipt.as_ref(),
        ),
    )
}

fn scheduler_task_request_text(task: &crate::state_store::TaskRecord) -> Option<String> {
    task.description
        .trim()
        .strip_prefix('\n')
        .unwrap_or(task.description.trim())
        .trim()
        .chars()
        .next()
        .map(|_| task.description.trim().to_string())
        .or_else(|| {
            task.title
                .trim()
                .chars()
                .next()
                .map(|_| task.title.trim().to_string())
        })
}

#[cfg(test)]
fn scheduler_agent_init_activation_result_status(
    activation_kind: Option<&str>,
) -> (&'static str, &'static str, &'static str) {
    match activation_kind {
        Some("activation_view") => (
            "activation_view_only",
            "agent_init_activation_view_only",
            "scheduler_agent_init_activation_view_only",
        ),
        Some(_) => (
            "activation_evidence_returned",
            "agent_init_activation_evidence_only",
            "scheduler_agent_init_activation_evidence_only",
        ),
        None => (
            "activation_evidence_returned_unclassified",
            "agent_init_activation_evidence_unclassified",
            "scheduler_agent_init_activation_evidence_unclassified",
        ),
    }
}

fn scheduler_reservation_acquire_requests(
    plan: &TaskflowSchedulerDispatchPlan,
    receipt_id: &str,
    receipt_path: &str,
) -> Vec<crate::state_store::AcquireSchedulerDispatchReservationRequest> {
    plan.reservations
        .iter()
        .map(
            |reservation| crate::state_store::AcquireSchedulerDispatchReservationRequest {
                reservation_id: reservation.reservation_id.clone(),
                task_id: reservation.task_id.clone(),
                launch_role: reservation.launch_role.clone(),
                launch_index: reservation.launch_index as u64,
                conflict_domain: reservation.conflict_domain.clone(),
                scope_task_id: plan.scope_task_id.clone(),
                requested_current_task_id: plan.requested_current_task_id.clone(),
                selection_source: plan.selection_source.clone(),
                max_parallel_agents: plan.max_parallel_agents,
                command: reservation.command.clone(),
                state_dir: reservation.state_dir.clone(),
                lease_owner: "vida taskflow scheduler dispatch".to_string(),
                lease_token: receipt_id.to_string(),
                lease_seconds: 900,
                dispatch_receipt_id: Some(receipt_id.to_string()),
                receipt_path: Some(receipt_path.to_string()),
            },
        )
        .collect()
}

struct SchedulerPacketDispatchResult {
    run_id: String,
    command: String,
    dispatch_packet_path: String,
    dispatch_status: String,
    lane_status: String,
    execute_status: String,
    worker_execution_evidence_status: String,
    worker_completion_claimed: bool,
    preview_only_reason: Option<String>,
    blocker_codes: Vec<String>,
}

async fn scheduler_execute_packet_backed_dispatch(
    state_dir: &Path,
    task_id: &str,
    run_id: &str,
    expected_dispatch_packet_path: Option<&str>,
    reservation_id: &str,
) -> Result<SchedulerPacketDispatchResult, String> {
    let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
        .await
        .map_err(|error| {
            format!("failed to open scheduler packet dispatch store before launch: {error}")
        })?;
    let mut artifacts =
        crate::taskflow_run_graph::prepare_run_graph_dispatch_init_artifacts(&store, run_id)
            .await?;
    if artifacts.run_id != run_id || artifacts.dispatch_receipt.run_id != run_id {
        return Err(format!(
            "scheduler packet dispatch lineage mismatch for task `{task_id}`: expected run `{run_id}`, got artifacts run `{}` and receipt run `{}`",
            artifacts.run_id, artifacts.dispatch_receipt.run_id
        ));
    }
    let run_id = artifacts.run_id.clone();
    let dispatch_packet_path = artifacts.dispatch_packet_path.clone();
    if let Some(expected_dispatch_packet_path) = expected_dispatch_packet_path {
        if dispatch_packet_path != expected_dispatch_packet_path {
            return Err(format!(
                "scheduler packet dispatch path mismatch for task `{task_id}` run `{run_id}`: gate verified `{expected_dispatch_packet_path}`, materialized `{dispatch_packet_path}`"
            ));
        }
    }
    store
        .mark_scheduler_dispatch_reservation_executing(
            reservation_id,
            Some(run_id.as_str()),
            "packet_dispatch_launching",
        )
        .await
        .map_err(|error| {
            format!(
                "failed to mark scheduler packet reservation `{reservation_id}` executing: {error}"
            )
        })?;
    drop(store);

    let execution_error = crate::runtime_dispatch_state::execute_and_record_dispatch_receipt(
        state_dir,
        &artifacts.role_selection,
        &artifacts.run_graph_bootstrap,
        &mut artifacts.dispatch_receipt,
    )
    .await
    .err();
    let has_execution_evidence =
        crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(
            &artifacts.dispatch_receipt,
        );
    let worker_execution_evidence_status = if has_execution_evidence {
        "received"
    } else {
        "not_received"
    }
    .to_string();
    let mut blocker_codes = Vec::new();
    let (execute_status, preview_only_reason) = if let Some(error) = execution_error {
        blocker_codes.push("scheduler_packet_dispatch_failed".to_string());
        (
            "packet_dispatch_failed".to_string(),
            Some(format!("scheduler_packet_dispatch_failed: {error}")),
        )
    } else if has_execution_evidence {
        ("packet_dispatch_executed".to_string(), None)
    } else {
        blocker_codes.push("scheduler_packet_dispatch_no_execution_evidence".to_string());
        (
            "packet_dispatch_no_execution_evidence".to_string(),
            Some("scheduler_packet_dispatch_no_execution_evidence".to_string()),
        )
    };

    let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
        .await
        .map_err(|error| {
            format!("failed to open scheduler packet dispatch store after launch: {error}")
        })?;
    store
        .release_scheduler_dispatch_reservation_with_blockers(
            reservation_id,
            &execute_status,
            &blocker_codes,
        )
        .await
        .map_err(|error| {
            format!("failed to release scheduler packet reservation `{reservation_id}`: {error}")
        })?;

    Ok(SchedulerPacketDispatchResult {
        run_id,
        command: artifacts
            .dispatch_receipt
            .dispatch_command
            .clone()
            .unwrap_or_else(|| {
                crate::runtime_dispatch_state::agent_init_execute_command_for_packet_path(
                    &dispatch_packet_path,
                )
            }),
        dispatch_packet_path,
        dispatch_status: artifacts.dispatch_receipt.dispatch_status.clone(),
        lane_status: artifacts.dispatch_receipt.lane_status.clone(),
        execute_status,
        worker_execution_evidence_status,
        worker_completion_claimed: has_execution_evidence,
        preview_only_reason,
        blocker_codes,
    })
}

async fn persist_scheduler_execute_receipt(
    plan: &mut TaskflowSchedulerDispatchPlan,
    state_dir: &Path,
) -> Result<(), String> {
    if !plan.execute_requested || plan.selected_task_ids.is_empty() {
        return Ok(());
    }

    let gate = plan
        .packet_backed_execution_gate
        .clone()
        .unwrap_or_else(|| {
            crate::taskflow_run_graph::evaluate_run_graph_packet_backed_execution_gate(
                plan.selected_task_ids.first().map(String::as_str),
                None,
                None,
                None,
                None,
            )
        });
    plan.packet_backed_execution_supported = gate.supported;
    plan.packet_backed_execution_status = gate.status.clone();
    plan.dispatch_receipt.packet_backed_execution_supported = gate.supported;
    plan.dispatch_receipt.packet_backed_execution_status = gate.status.clone();
    plan.dispatch_receipt.packet_backed_execution_gate = Some(gate.clone());
    plan.packet_backed_execution_gate = Some(gate);
    let mut all_packet_backed_execution_supported = true;

    let receipt_id = scheduler_dispatch_receipt_id();
    let receipt_root = state_dir.join("scheduler-dispatch").join("receipts");
    std::fs::create_dir_all(&receipt_root).map_err(|error| {
        format!(
            "failed to create scheduler dispatch receipt directory `{}`: {error}",
            receipt_root.display()
        )
    })?;
    let receipt_path = receipt_root.join(format!("{receipt_id}.json"));
    let receipt_path_string = receipt_path.display().to_string();
    let mut launch_results = Vec::new();
    let mut launch_blockers = Vec::new();
    let acquire_requests =
        scheduler_reservation_acquire_requests(plan, &receipt_id, &receipt_path_string);
    {
        let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
            .await
            .map_err(|error| format!("failed to open scheduler reservation store: {error}"))?;
        store
            .acquire_scheduler_dispatch_reservations(&acquire_requests)
            .await
            .map_err(|error| format!("failed to acquire scheduler reservations: {error}"))?;
    }

    for reservation in &mut plan.reservations {
        reservation.reservation_persisted = true;
        reservation.execute_supported = true;
        reservation.execution_attempted = true;
        reservation.activation_attempt_supported = true;
        reservation.activation_attempted = true;
        reservation.reservation_status = "reservation_persisted".to_string();
        reservation.receipt_id = Some(receipt_id.clone());
        reservation.receipt_path = Some(receipt_path_string.clone());
        let reservation_gate = {
            let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "failed to open scheduler packet gate store for `{}`: {error}",
                        reservation.task_id
                    )
                })?;
            build_scheduler_packet_backed_execution_gate_for_task(&store, &reservation.task_id)
                .await?
        };
        if reservation_gate.supported {
            plan.packet_backed_execution_supported = true;
            plan.packet_backed_execution_status = reservation_gate.status.clone();
            plan.dispatch_receipt.packet_backed_execution_supported = true;
            plan.dispatch_receipt.packet_backed_execution_status = reservation_gate.status.clone();
            if plan.selected_task_ids.len() == 1 {
                plan.dispatch_receipt.packet_backed_execution_gate = Some(reservation_gate.clone());
                plan.packet_backed_execution_gate = Some(reservation_gate.clone());
            }
            let packet_backed_run_id = reservation_gate.run_id.as_deref().ok_or_else(|| {
                format!(
                    "scheduler packet-backed execution gate supported task `{}` without run_id",
                    reservation.task_id
                )
            })?;
            let packet_backed_dispatch_packet_path =
                reservation_gate.dispatch_packet_path.as_deref();
            let packet_result = scheduler_execute_packet_backed_dispatch(
                state_dir,
                &reservation.task_id,
                packet_backed_run_id,
                packet_backed_dispatch_packet_path,
                &reservation.reservation_id,
            )
            .await;
            match packet_result {
                Ok(packet_result) => {
                    reservation.command = packet_result.command.clone();
                    reservation.execute_status = packet_result.execute_status.clone();
                    reservation.activation_status = packet_result.execute_status.clone();
                    reservation.worker_execution_evidence_status =
                        packet_result.worker_execution_evidence_status.clone();
                    reservation.worker_completion_claimed = packet_result.worker_completion_claimed;
                    reservation.preview_only_reason = packet_result.preview_only_reason.clone();
                    reservation.activation_blocker_codes = packet_result.blocker_codes.clone();
                    launch_blockers.extend(packet_result.blocker_codes.clone());
                    if packet_result.worker_execution_evidence_status == "received" {
                        plan.worker_execution_evidence_status = "received".to_string();
                        plan.dispatch_receipt.worker_execution_evidence_status =
                            "received".to_string();
                    }
                    if packet_result.worker_completion_claimed {
                        plan.worker_completion_claimed = true;
                        plan.dispatch_receipt.worker_completion_claimed = true;
                    }
                    launch_results.push(serde_json::json!({
                        "task_id": reservation.task_id,
                        "reservation_id": reservation.reservation_id,
                        "status": packet_result.execute_status,
                        "activation_status": packet_result.execute_status,
                        "worker_execution_evidence_status": packet_result.worker_execution_evidence_status,
                        "worker_completion_claimed": packet_result.worker_completion_claimed,
                        "run_id": packet_result.run_id,
                        "dispatch_packet_path": packet_result.dispatch_packet_path,
                        "dispatch_status": packet_result.dispatch_status,
                        "lane_status": packet_result.lane_status,
                    }));
                }
                Err(error) => {
                    reservation.execute_status = "packet_dispatch_failed".to_string();
                    reservation.activation_status = "packet_dispatch_failed".to_string();
                    reservation.preview_only_reason =
                        Some("scheduler_packet_dispatch_failed".to_string());
                    reservation.activation_blocker_codes =
                        vec!["scheduler_packet_dispatch_failed".to_string()];
                    launch_blockers.push("scheduler_packet_dispatch_failed".to_string());
                    launch_results.push(serde_json::json!({
                        "task_id": reservation.task_id,
                        "reservation_id": reservation.reservation_id,
                        "status": "packet_dispatch_failed",
                        "activation_status": "packet_dispatch_failed",
                        "worker_execution_evidence_status": "not_received",
                        "worker_completion_claimed": false,
                        "error": error,
                    }));
                    let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
                        .await
                        .map_err(|error| {
                            format!("failed to open scheduler reservation store after packet dispatch failure: {error}")
                        })?;
                    store
                        .release_scheduler_dispatch_reservation_with_blockers(
                            &reservation.reservation_id,
                            "packet_dispatch_failed",
                            &reservation.activation_blocker_codes,
                        )
                        .await
                        .map_err(|error| {
                            format!(
                                "failed to release scheduler reservation `{}` after packet dispatch failure: {error}",
                                reservation.reservation_id
                            )
                        })?;
                }
            }
            continue;
        }
        all_packet_backed_execution_supported = false;
        {
            let store = crate::state_store::StateStore::open_existing(state_dir.to_path_buf())
                .await
                .map_err(|error| {
                    format!("failed to open scheduler reservation store after packet gate block: {error}")
                })?;
            let blocked_status = reservation_gate.status.clone();
            let mut blocker_codes = reservation_gate.blocker_codes.clone();
            if blocker_codes.is_empty() {
                blocker_codes.push(blocked_status.clone());
            }
            reservation.execute_status = blocked_status.clone();
            reservation.activation_status = blocked_status.clone();
            reservation.preview_only_reason = Some(blocked_status.clone());
            reservation.activation_blocker_codes = blocker_codes.clone();
            launch_blockers.extend(blocker_codes.clone());
            launch_results.push(serde_json::json!({
                "task_id": reservation.task_id,
                "reservation_id": reservation.reservation_id,
                "status": blocked_status,
                "activation_status": blocked_status,
                "worker_execution_evidence_status": "not_received",
                "worker_completion_claimed": false,
            }));
            store
                .release_scheduler_dispatch_reservation_with_blockers(
                    &reservation.reservation_id,
                    &blocked_status,
                    &reservation.activation_blocker_codes,
                )
                .await
                .map_err(|error| {
                    format!(
                        "failed to release scheduler reservation `{}` after packet gate block: {error}",
                        reservation.reservation_id
                    )
                })?;
        }
        continue;
    }

    launch_blockers.sort();
    launch_blockers.dedup();
    scheduler_remove_blocker_codes(
        &mut plan.blocker_codes,
        &["scheduler_execute_external_execution_unavailable"],
    );
    scheduler_remove_blocker_codes(
        &mut plan.dispatch_receipt.blocker_codes,
        &["scheduler_execute_external_execution_unavailable"],
    );
    plan.next_actions
        .retain(|action| !action.contains("external lane execution is not attempted"));
    plan.dispatch_receipt.execution_blocker_codes = launch_blockers.clone();
    plan.dispatch_receipt.activation_blocker_codes = launch_blockers.clone();
    plan.dispatch_receipt.receipt_id = Some(receipt_id.clone());
    plan.dispatch_receipt.receipt_path = Some(receipt_path_string.clone());
    plan.dispatch_receipt.receipt_persisted = true;
    plan.dispatch_receipt.receipt_status = "persisted".to_string();
    plan.dispatch_receipt.execute_supported = true;
    plan.dispatch_receipt.execution_attempted = true;
    plan.dispatch_receipt.activation_attempt_supported = true;
    plan.dispatch_receipt.activation_attempted = true;
    plan.execute_supported = true;
    plan.execution_attempted = true;
    plan.activation_attempt_supported = true;
    plan.activation_attempted = true;
    if all_packet_backed_execution_supported {
        plan.packet_backed_execution_supported = true;
        plan.dispatch_receipt.packet_backed_execution_supported = true;
        plan.packet_backed_execution_status = "packet_ready".to_string();
        plan.dispatch_receipt.packet_backed_execution_status = "packet_ready".to_string();
    }
    let packet_execution_evidence_received =
        launch_blockers.is_empty() && plan.worker_execution_evidence_status == "received";
    if packet_execution_evidence_received {
        plan.status = "pass".to_string();
        plan.dispatch_receipt.dispatch_status = "pass".to_string();
        plan.execution_status = "packet_dispatch_executed".to_string();
        plan.activation_status = plan.execution_status.clone();
        plan.activation_blocker_codes = Vec::new();
        plan.dispatch_receipt.execute_status = plan.execution_status.clone();
        plan.dispatch_receipt.activation_status = plan.execution_status.clone();
        plan.dispatch_receipt.preview_only_reason = None;
        plan.dispatch_receipt.activation_blocker_codes = Vec::new();
        plan.dispatch_receipt.execution_blocker_codes = Vec::new();
    } else if launch_blockers.is_empty() {
        plan.status = "blocked".to_string();
        plan.dispatch_receipt.dispatch_status = "blocked".to_string();
        plan.execution_status = "scheduler_execute_no_execution_evidence".to_string();
        plan.activation_status = plan.execution_status.clone();
        plan.dispatch_receipt.execute_status = plan.execution_status.clone();
        plan.dispatch_receipt.activation_status = plan.execution_status.clone();
        plan.dispatch_receipt.preview_only_reason = Some(plan.execution_status.clone());
        plan.blocker_codes.push(plan.execution_status.clone());
        plan.activation_blocker_codes = vec![plan.execution_status.clone()];
        plan.dispatch_receipt
            .blocker_codes
            .push(plan.execution_status.clone());
        plan.dispatch_receipt.activation_blocker_codes = vec![plan.execution_status.clone()];
        plan.dispatch_receipt
            .execution_blocker_codes
            .push(plan.execution_status.clone());
        plan.next_actions.push(
            "Scheduler execute did not receive receipt-backed worker execution evidence; continue through the lawful execution handoff before claiming completion."
                .to_string(),
        );
    } else {
        for blocker in &launch_blockers {
            if !plan.blocker_codes.iter().any(|code| code == blocker) {
                plan.blocker_codes.push(blocker.clone());
            }
            if !plan
                .dispatch_receipt
                .blocker_codes
                .iter()
                .any(|code| code == blocker)
            {
                plan.dispatch_receipt.blocker_codes.push(blocker.clone());
            }
        }
        if launch_blockers
            .iter()
            .any(|code| code == "scheduler_agent_init_activation_view_only")
        {
            plan.next_actions.push(
                "Scheduler selected lane activation returned view-only; continue through the lawful VIDA agent packet/dispatch path before claiming worker completion."
                    .to_string(),
            );
        }
        if launch_blockers
            .iter()
            .any(|code| code == "scheduler_agent_init_failed")
        {
            plan.next_actions.push(
                "Scheduler selected lane activation failed to launch; review scheduler reservation output and resolve the launch blocker before retrying."
                    .to_string(),
            );
        }
        plan.status = "blocked".to_string();
        plan.dispatch_receipt.dispatch_status = "blocked".to_string();
        plan.execution_status = launch_blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "scheduler_execute_blocked".to_string());
        plan.activation_status = plan.execution_status.clone();
        plan.activation_blocker_codes = launch_blockers.clone();
        plan.dispatch_receipt.execute_status = plan.execution_status.clone();
        plan.dispatch_receipt.activation_status = plan.execution_status.clone();
        plan.dispatch_receipt.activation_blocker_codes = launch_blockers.clone();
        plan.dispatch_receipt.preview_only_reason = Some(plan.execution_status.clone());
    }

    let receipt_body = serde_json::json!({
        "receipt_id": receipt_id,
        "surface": plan.surface,
        "dispatch_surface": plan.dispatch_receipt.dispatch_surface,
        "dispatch_command": plan.dispatch_receipt.dispatch_command,
        "status": plan.status,
        "execute_requested": plan.execute_requested,
        "execute_supported": plan.execute_supported,
        "execution_attempted": plan.execution_attempted,
        "execution_status": plan.execution_status,
        "activation_attempt_supported": plan.activation_attempt_supported,
        "activation_attempted": plan.activation_attempted,
        "activation_status": plan.activation_status,
        "activation_blocker_codes": plan.activation_blocker_codes,
        "worker_execution_evidence_status": plan.worker_execution_evidence_status,
        "worker_completion_claimed": plan.worker_completion_claimed,
        "packet_backed_execution_supported": plan.packet_backed_execution_supported,
        "packet_backed_execution_status": plan.packet_backed_execution_status,
        "packet_backed_execution_gate": plan.packet_backed_execution_gate,
        "selected_task_ids": plan.selected_task_ids,
        "reservations": plan.reservations,
        "reservation_ids": plan.dispatch_receipt.reservation_ids,
        "blocker_codes": plan.blocker_codes,
        "launch_results": launch_results,
    });
    let receipt_text = serde_json::to_string_pretty(&receipt_body)
        .map_err(|error| format!("failed to render scheduler dispatch receipt: {error}"))?;
    std::fs::write(&receipt_path, receipt_text).map_err(|error| {
        format!(
            "failed to write scheduler dispatch receipt `{}`: {error}",
            receipt_path.display()
        )
    })?;
    Ok(())
}

fn build_taskflow_scheduler_dispatch_plan(
    scheduling: crate::state_store::TaskSchedulingProjection,
    max_parallel_agents: u64,
    requested_parallel_limit: Option<u64>,
    scope_task_id: Option<&str>,
    requested_current_task_id: Option<&str>,
    explicit_bound_current_task_id: Option<&str>,
    state_dir: &std::path::Path,
    dry_run: bool,
    execute_requested: bool,
) -> TaskflowSchedulerDispatchPlan {
    let configured_max_parallel_agents = max_parallel_agents.max(1);
    let effective_parallel_limit = scheduler_effective_parallel_limit(
        configured_max_parallel_agents,
        requested_parallel_limit,
    );
    let selected_current_candidate = if let Some(task_id) = requested_current_task_id {
        scheduling
            .ready
            .iter()
            .find(|candidate| candidate.task.id == task_id)
    } else if let Some(task_id) = explicit_bound_current_task_id {
        scheduling
            .ready
            .iter()
            .find(|candidate| candidate.task.id == task_id)
    } else {
        scheduling
            .ready
            .iter()
            .find(|candidate| candidate.active_critical_path)
            .or_else(|| scheduling.ready.first())
    };
    let selected_current_task_id =
        selected_current_candidate.map(|candidate| candidate.task.id.clone());
    let selection_source = if requested_current_task_id.is_some() {
        if selected_current_task_id.is_some() {
            "requested_current_task"
        } else {
            "requested_current_task_not_ready"
        }
    } else if explicit_bound_current_task_id.is_some() {
        if selected_current_task_id.is_some() {
            "explicit_run_graph_continuation_binding"
        } else {
            "explicit_run_graph_continuation_binding_not_ready"
        }
    } else if selected_current_candidate.is_some_and(|candidate| candidate.active_critical_path) {
        "critical_path_ready_head"
    } else if selected_current_candidate.is_some() {
        "ready_head_fallback"
    } else {
        "no_ready_primary"
    }
    .to_string();

    let selected_primary_task =
        selected_current_candidate.map(|candidate| graph_summary_task_ref(&candidate.task));
    let parallel_capacity = effective_parallel_limit.saturating_sub(1) as usize;
    let mut selected_parallel_tasks = Vec::new();
    let mut rejected_candidates = Vec::new();
    let mut remaining_parallel_capacity = parallel_capacity;
    let mut selected_conflict_domains = BTreeSet::<String>::new();
    if let Some(domain) = selected_primary_task
        .as_ref()
        .and_then(|task| task.conflict_domain.as_ref())
    {
        selected_conflict_domains.insert(domain.clone());
    }

    for candidate in &scheduling.ready {
        if Some(candidate.task.id.as_str()) == selected_current_task_id.as_deref() {
            continue;
        }

        let task = graph_summary_task_ref(&candidate.task);
        let conflict_domain_blocker = task
            .conflict_domain
            .as_ref()
            .filter(|domain| selected_conflict_domains.contains(*domain))
            .map(|domain| format!("conflict_domain_already_selected:{domain}"));

        if candidate.ready_parallel_safe
            && conflict_domain_blocker.is_none()
            && remaining_parallel_capacity > 0
        {
            if let Some(domain) = task.conflict_domain.as_ref() {
                selected_conflict_domains.insert(domain.clone());
            }
            selected_parallel_tasks.push(task);
            remaining_parallel_capacity -= 1;
            continue;
        }

        let reasons = if let Some(blocker) = conflict_domain_blocker {
            let mut reasons = candidate.parallel_blockers.clone();
            if !reasons.iter().any(|reason| reason == &blocker) {
                reasons.push(blocker);
            }
            reasons
        } else if candidate.ready_parallel_safe {
            vec!["max_parallel_agents_cap_reached".to_string()]
        } else if candidate.parallel_blockers.is_empty() {
            vec!["max_parallel_agents_cap_reached".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        rejected_candidates.push(TaskflowSchedulerRejectedCandidate {
            task_id: task.id.clone(),
            conflict_domain: task.conflict_domain.clone(),
            task,
            ready_now: candidate.ready_now,
            active_critical_path: candidate.active_critical_path,
            reasons,
            blocked_by: candidate.blocked_by.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        });
    }

    for candidate in &scheduling.blocked {
        let task = graph_summary_task_ref(&candidate.task);
        rejected_candidates.push(TaskflowSchedulerRejectedCandidate {
            task_id: task.id.clone(),
            conflict_domain: task.conflict_domain.clone(),
            task,
            ready_now: candidate.ready_now,
            active_critical_path: candidate.active_critical_path,
            reasons: scheduler_rejection_reasons_from_blocked_by(&candidate.blocked_by),
            blocked_by: candidate.blocked_by.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        });
    }

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();
    if selected_primary_task.is_none() {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
        if requested_current_task_id.is_some() {
            blocker_codes.push("requested_current_task_not_ready".to_string());
        } else if explicit_bound_current_task_id.is_some() {
            blocker_codes.push("explicit_run_graph_continuation_binding_not_ready".to_string());
        }
        next_actions.push(
            "Inspect `vida taskflow graph-summary --json` before attempting scheduler dispatch."
                .to_string(),
        );
    }
    let execute_requested_with_selection = execute_requested && selected_primary_task.is_some();
    if let Some(task) = selected_primary_task.as_ref() {
        next_actions.push(format!(
            "Inspect the selected primary task with `vida task show {} --json` before delegated launch.",
            task.id
        ));
    }
    if let Some(task) = selected_parallel_tasks.first() {
        next_actions.push(format!(
            "Verify co-scheduling safety for `{}` with `vida taskflow graph-summary --json` before parallel launch.",
            task.id
        ));
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    }
    .to_string();
    let mut selected_task_ids = selected_primary_task
        .iter()
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    selected_task_ids.extend(selected_parallel_tasks.iter().map(|task| task.id.clone()));
    let mut reservations = Vec::new();
    if let Some(task) = selected_primary_task.as_ref() {
        reservations.push(scheduler_reservation_preview(
            task,
            "primary",
            0,
            state_dir,
            execute_requested,
        ));
    }
    reservations.extend(
        selected_parallel_tasks
            .iter()
            .enumerate()
            .map(|(index, task)| {
                scheduler_reservation_preview(
                    task,
                    "parallel",
                    index + 1,
                    state_dir,
                    execute_requested,
                )
            }),
    );
    if execute_requested_with_selection {
        for reservation in &mut reservations {
            reservation.execute_supported = true;
        }
    }
    let reservation_ids = reservations
        .iter()
        .map(|reservation| reservation.reservation_id.clone())
        .collect::<Vec<_>>();
    let blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let preview_only_reason = if execute_requested_with_selection {
        None
    } else if execute_requested {
        Some("scheduler_execute_external_execution_unavailable".to_string())
    } else {
        Some("scheduler_dispatch_is_preview_only".to_string())
    };
    let execute_supported = execute_requested_with_selection;
    let execution_status = if execute_requested {
        if selected_primary_task.is_some() {
            "execute_projection_not_executed"
        } else {
            "blocked_no_lawful_selection"
        }
    } else {
        "preview"
    }
    .to_string();
    let dispatch_receipt = TaskflowSchedulerDispatchReceiptPreview {
        receipt_id: None,
        receipt_path: None,
        receipt_status: if execute_requested {
            "execute_projection_not_persisted"
        } else {
            "preview_not_persisted"
        }
        .to_string(),
        receipt_persisted: false,
        dispatch_surface: "vida taskflow scheduler dispatch".to_string(),
        dispatch_command: if execute_requested {
            "vida taskflow scheduler dispatch --execute --json"
        } else {
            "vida taskflow scheduler dispatch --json"
        }
        .to_string(),
        dispatch_status: status.clone(),
        execute_requested,
        execute_supported,
        execution_attempted: false,
        execute_status: if execute_requested {
            execution_status.clone()
        } else {
            "preview_not_executed".to_string()
        },
        activation_attempt_supported: execute_supported,
        activation_attempted: false,
        activation_status: if execute_requested {
            execution_status.clone()
        } else {
            "preview_not_executed".to_string()
        },
        activation_blocker_codes: Vec::new(),
        worker_execution_evidence_status: "not_received".to_string(),
        worker_completion_claimed: false,
        packet_backed_execution_supported: false,
        packet_backed_execution_status: if execute_requested {
            "blocked_lineage_preconditions_not_verified".to_string()
        } else {
            "preview_not_requested".to_string()
        },
        packet_backed_execution_gate: None,
        preview_only_reason,
        execution_blocker_codes: Vec::new(),
        selected_task_ids: selected_task_ids.clone(),
        reservation_ids,
        blocker_codes: blocker_codes.clone(),
    };

    TaskflowSchedulerDispatchPlan {
        status,
        surface: "vida taskflow scheduler dispatch".to_string(),
        blocker_codes,
        next_actions,
        dry_run,
        execute_requested,
        execute_supported,
        execution_attempted: false,
        execution_status,
        activation_attempt_supported: execute_supported,
        activation_attempted: false,
        activation_status: if execute_requested {
            if selected_primary_task.is_some() {
                "execute_projection_not_executed"
            } else {
                "blocked_no_lawful_selection"
            }
        } else {
            "preview"
        }
        .to_string(),
        activation_blocker_codes: Vec::new(),
        worker_execution_evidence_status: "not_received".to_string(),
        worker_completion_claimed: false,
        packet_backed_execution_supported: false,
        packet_backed_execution_status: if execute_requested {
            "blocked_lineage_preconditions_not_verified".to_string()
        } else {
            "preview_not_requested".to_string()
        },
        packet_backed_execution_gate: None,
        configured_max_parallel_agents,
        requested_parallel_limit,
        scope_task_id: scope_task_id.map(str::to_string),
        requested_current_task_id: requested_current_task_id.map(str::to_string),
        selected_current_task_id,
        selection_source,
        max_parallel_agents: effective_parallel_limit,
        ready_count: scheduling.ready.len(),
        blocked_count: scheduling.blocked.len(),
        selected_primary_task,
        selected_parallel_tasks,
        selected_task_ids,
        reservations,
        dispatch_receipt,
        rejected_candidates,
        scheduling,
    }
}

pub(crate) async fn build_taskflow_scheduler_dispatch_plan_from_store(
    store: &crate::state_store::StateStore,
    state_dir: &Path,
    scope_task_id: Option<&str>,
    current_task_id: Option<&str>,
    requested_parallel_limit: Option<u64>,
    dry_run: bool,
    execute_requested: bool,
) -> Result<TaskflowSchedulerDispatchPlan, String> {
    let max_parallel_agents = crate::build_taskflow_consume_bundle_payload(store)
        .await
        .map(|payload| normalize_scheduler_max_parallel_agents(&payload.activation_bundle))
        .unwrap_or(1);
    let explicit_binding = if current_task_id.is_none() {
        Some(
            store
                .latest_explicit_run_graph_continuation_binding()
                .await
                .map_err(|error| {
                    format!("Failed to read latest explicit continuation binding: {error}")
                })?,
        )
    } else {
        None
    };
    let explicit_bound_current_task_id = explicit_task_graph_continuation_task_id(
        explicit_binding.as_ref().and_then(|value| value.as_ref()),
    );
    let effective_current_task_id = current_task_id.or(explicit_bound_current_task_id);

    let initial_projection = store
        .scheduling_projection_scoped(scope_task_id, effective_current_task_id)
        .await
        .map_err(|error| format!("Failed to compute scheduler projection: {error}"))?;
    let selected_primary_id = if let Some(task_id) = effective_current_task_id {
        initial_projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == task_id)
            .map(|candidate| candidate.task.id.clone())
    } else {
        initial_projection
            .ready
            .iter()
            .find(|candidate| candidate.active_critical_path)
            .or_else(|| initial_projection.ready.first())
            .map(|candidate| candidate.task.id.clone())
    };

    let scheduling = if let Some(primary_id) = selected_primary_id.as_deref() {
        if initial_projection.current_task_id.as_deref() == Some(primary_id) {
            initial_projection
        } else {
            store
                .scheduling_projection_scoped(scope_task_id, Some(primary_id))
                .await
                .map_err(|error| format!("Failed to recompute scheduler projection: {error}"))?
        }
    } else {
        initial_projection
    };

    Ok(build_taskflow_scheduler_dispatch_plan(
        scheduling,
        max_parallel_agents,
        requested_parallel_limit,
        scope_task_id,
        current_task_id,
        explicit_bound_current_task_id,
        state_dir,
        dry_run,
        execute_requested,
    ))
}

fn terminal_completed_without_next_unit(
    status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    status.is_some_and(|status| {
        status.status == "completed"
            && status.lifecycle_stage == "closure_complete"
            && status
                .next_node
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
    })
}

fn explicit_task_binding_matches_status(
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    let Some(binding) = binding else {
        return false;
    };
    let Some(status) = status else {
        return false;
    };
    binding.run_id == status.run_id
        && binding.binding_source == "explicit_continuation_bind_task"
        && binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str)
            == Some("task_graph_task")
}

fn active_exception_takeover_evidence_matches_status(
    status: Option<&crate::state_store::RunGraphStatus>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    terminal_continue_run_id: Option<&str>,
) -> bool {
    let Some(status) = status else {
        return false;
    };
    let Some(dispatch) = dispatch else {
        return false;
    };
    if terminal_continue_run_id == Some(status.run_id.as_str()) {
        return false;
    }
    dispatch.run_id == status.run_id
        && dispatch.lane_status == "lane_exception_takeover"
        && dispatch
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && dispatch
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn active_exception_takeover_binding_matches_status(
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    status: Option<&crate::state_store::RunGraphStatus>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    let Some(binding) = binding else {
        return false;
    };
    let Some(status) = status else {
        return false;
    };
    let binding_kind = binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str);

    binding.status == "bound"
        && binding.run_id == status.run_id
        && binding.task_id == status.task_id
        && binding.binding_source == "consume_continue_after_downstream_chain"
        && binding_kind == Some("run_graph_task")
        && active_exception_takeover_evidence_matches_status(Some(status), dispatch, None)
}

fn build_taskflow_next_decision(
    ready_head: Option<&crate::state_store::TaskRecord>,
    recovery_holds_active_bound_run: bool,
    recovery_present: bool,
    latest_runtime_consumption_kind: Option<&str>,
    scope_task_id: Option<&str>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    terminal_consume_continue_run_id: Option<&str>,
) -> TaskflowNextDecision {
    let ready_head = ready_head.map(graph_summary_task_ref);
    let latest_run_graph_status_blocked =
        latest_run_graph_status_blocks_normal_continuation(latest_run_graph_status);
    let active_exception_takeover_binding = active_exception_takeover_binding_matches_status(
        explicit_binding,
        latest_run_graph_status,
        dispatch,
    );
    let active_exception_takeover_evidence = active_exception_takeover_evidence_matches_status(
        latest_run_graph_status,
        dispatch,
        terminal_consume_continue_run_id,
    );
    let terminal_consume_continue_without_next_unit = latest_run_graph_status
        .zip(terminal_consume_continue_run_id)
        .is_some_and(|(status, run_id)| status.run_id == run_id);
    let latest_run_graph_status_blocks_admission = latest_run_graph_status_blocked
        && !active_exception_takeover_evidence
        && !terminal_consume_continue_without_next_unit;
    let completed_without_explicit_next_unit =
        terminal_completed_without_next_unit(latest_run_graph_status)
            && !explicit_task_binding_matches_status(explicit_binding, latest_run_graph_status);
    let admissibility_gate = if recovery_holds_active_bound_run {
        "delegated_cycle_runtime_gate".to_string()
    } else if active_exception_takeover_evidence {
        "active_exception_takeover_continuation".to_string()
    } else if terminal_consume_continue_without_next_unit {
        "terminal_continue_snapshot_without_next_bounded_unit".to_string()
    } else if latest_run_graph_status_blocks_admission {
        "latest_run_graph_status_blocked".to_string()
    } else if completed_without_explicit_next_unit {
        "completed_without_explicit_next_bounded_unit".to_string()
    } else if ready_head.is_some() {
        "ready_now".to_string()
    } else {
        "no_ready_task".to_string()
    };
    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();
    let candidate_task_context = TaskflowNextCandidateContext {
        ready_head: ready_head.clone(),
        admissible_now: !(recovery_holds_active_bound_run
            || active_exception_takeover_binding
            || (active_exception_takeover_evidence && latest_run_graph_status_blocked)
            || terminal_consume_continue_without_next_unit
            || latest_run_graph_status_blocks_admission
            || completed_without_explicit_next_unit),
        admissibility_gate,
    };

    let (recommended_command, recommended_surface, primary_ready_task, why_not_now, next_action) =
        if recovery_holds_active_bound_run {
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::OpenDelegatedCycle,
            ) {
                blocker_codes.push(code);
            }
            blocker_codes.extend(authoritative_dispatch_blocker_codes(dispatch));

            if latest_runtime_consumption_kind == Some("final") {
                let next_action = TaskflowNextAction {
                    command: "vida taskflow consume continue --json".to_string(),
                    surface: "vida taskflow consume continue".to_string(),
                    reason: "the delegated cycle is still open, so the next lawful step is continuation rather than selecting a new backlog slice".to_string(),
                };
                next_actions.push(format!(
                    "Continue the active bound run with `{}` before considering backlog ready-head work.",
                    next_action.command
                ));
                (
                    Some(next_action.command.clone()),
                    Some(next_action.surface.clone()),
                    None,
                    Some(TaskflowNextWhyNotNow {
                        category: "delegated_cycle_runtime_gate".to_string(),
                        summary: "A delegated execution cycle is still open, so backlog ready-head work is not admissible yet.".to_string(),
                        blocker_codes: blocker_codes.clone(),
                        blocking_surface: Some("vida taskflow recovery latest".to_string()),
                    }),
                    Some(next_action),
                )
            } else {
                let next_action = TaskflowNextAction {
                    command: "vida taskflow recovery latest --json".to_string(),
                    surface: "vida taskflow recovery latest".to_string(),
                    reason: "the delegated cycle is open and execution-preparation evidence is not yet finalized".to_string(),
                };
                next_actions.push(format!(
                    "Inspect the active bound recovery state with `{}` before considering backlog ready-head work.",
                    next_action.command
                ));
                (
                    Some(next_action.command.clone()),
                    Some(next_action.surface.clone()),
                    None,
                    Some(TaskflowNextWhyNotNow {
                        category: "delegated_cycle_runtime_gate".to_string(),
                        summary: "A delegated execution cycle is open and execution-preparation evidence is not finalized.".to_string(),
                        blocker_codes: blocker_codes.clone(),
                        blocking_surface: Some("vida taskflow recovery latest".to_string()),
                    }),
                    Some(next_action),
                )
            }
        } else if active_exception_takeover_evidence {
            let run_id = latest_run_graph_status
                .map(|status| status.run_id.as_str())
                .unwrap_or("<run-id>");
            let next_action = TaskflowNextAction {
                command: format!("vida taskflow consume continue --run-id {run_id} --json"),
                surface: "vida taskflow consume continue".to_string(),
                reason: "active exception-takeover evidence has resolved the dispatch blocker; continue the bound run before considering backlog ready-head work".to_string(),
            };
            next_actions.push(format!(
                "Continue the active exception-backed bounded unit with `{}` before selecting backlog work.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                None,
                Some(next_action),
            )
        } else if terminal_consume_continue_without_next_unit {
            let run_id = latest_run_graph_status
                .map(|status| status.run_id.as_str())
                .unwrap_or("<run-id>");
            let next_action = TaskflowNextAction {
                command: format!(
                    "vida taskflow continuation bind {run_id} --task-id <task-id> --json"
                ),
                surface: "vida taskflow continuation bind".to_string(),
                reason: "the latest consume-continue snapshot already completed with no further actions, so the same run must not be continued again without an explicit next bounded unit".to_string(),
            };
            blocker_codes.push("terminal_continue_snapshot_without_next_bounded_unit".to_string());
            next_actions.push(format!(
                "Do not repeat `vida taskflow consume continue --run-id {run_id} --json`; bind the next bounded unit explicitly with `{}` or reconcile the stale run-graph state.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "terminal_continue_snapshot_without_next_bounded_unit".to_string(),
                    summary: "The latest consume-continue snapshot completed with no next actions while the run graph still references the same blocked run, so `vida taskflow next` must fail closed instead of self-looping or admitting backlog ready-head work.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida taskflow continuation bind".to_string()),
                }),
                Some(next_action),
            )
        } else if completed_without_explicit_next_unit {
            let run_id = latest_run_graph_status
                .map(|status| status.run_id.as_str())
                .unwrap_or("<run-id>");
            let next_action = TaskflowNextAction {
                command: format!(
                    "vida taskflow continuation bind {run_id} --task-id <task-id> --json"
                ),
                surface: "vida taskflow continuation bind".to_string(),
                reason: "the latest run is already closure_complete and no explicit continuation binding is admissible yet".to_string(),
            };
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::NoReadyTasks,
            ) {
                blocker_codes.push(code);
            }
            next_actions.push(format!(
                "Do not continue by heuristic after closure; bind the next bounded unit explicitly with `{}`.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "completed_without_explicit_next_bounded_unit".to_string(),
                    summary: "The latest run is closure_complete with no explicit admissible continuation binding, so `vida taskflow next` must fail closed.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida taskflow continuation bind".to_string()),
                }),
                Some(next_action),
            )
        } else if latest_run_graph_status_blocks_admission {
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::LatestRunGraphStatusBlocked,
            ) {
                blocker_codes.push(code);
            }
            blocker_codes.extend(authoritative_dispatch_blocker_codes(dispatch));
            let run_id = latest_run_graph_status
                .map(|status| status.run_id.as_str())
                .unwrap_or("<run-id>");
            let next_action = TaskflowNextAction {
                command: format!("vida taskflow recovery status {run_id} --json"),
                surface: "vida taskflow recovery status".to_string(),
                reason: "the latest run graph is blocked, so backlog ready-head work is not lawful until recovery truth is resolved".to_string(),
            };
            next_actions.push(format!(
                "Inspect the blocked run-graph recovery state with `{}` before considering backlog ready-head work.",
                next_action.command
            ));
            next_actions.push(
                "After resolving the blocker, refresh continuation evidence with `vida taskflow consume continue --json` or bind the next bounded unit explicitly."
                    .to_string(),
            );
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "latest_run_graph_status_blocked".to_string(),
                    summary: "The latest run graph is blocked, so `vida taskflow next` must not present a ready backlog task as dispatchable.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida taskflow recovery status".to_string()),
                }),
                Some(next_action),
            )
        } else if let Some(task) = ready_head.clone() {
            let next_action = TaskflowNextAction {
                command: format!("vida task show {} --json", task.id),
                surface: "vida task show".to_string(),
                reason: "a backlog slice is ready now; inspect the canonical task record before dispatch".to_string(),
            };
            next_actions.push(format!(
                "Inspect the primary ready task with `{}` before dispatch.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                Some(task),
                None,
                Some(next_action),
            )
        } else if recovery_present && latest_runtime_consumption_kind == Some("final") {
            let next_action = TaskflowNextAction {
                command: "vida taskflow consume continue --json".to_string(),
                surface: "vida taskflow consume continue".to_string(),
                reason: "no ready backlog slice exists, but the latest lawful delegated chain can still continue".to_string(),
            };
            next_actions.push(format!(
                "Continue the latest lawful delegated chain with `{}`.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                None,
                Some(next_action),
            )
        } else {
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::NoReadyTasks,
            ) {
                blocker_codes.push(code);
            }
            let ready_command = if let Some(task_id) = scope_task_id {
                format!("vida task ready --scope {task_id} --json")
            } else {
                "vida task ready --json".to_string()
            };
            let next_action = TaskflowNextAction {
                command: ready_command.clone(),
                surface: "vida task ready".to_string(),
                reason: "no ready backlog slice is currently admissible".to_string(),
            };
            next_actions.push(format!(
                "No ready backlog slice is available right now; inspect `{ready_command}` and `vida taskflow recovery latest --json`."
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "backlog_no_ready_task".to_string(),
                    summary: "No ready backlog slice is currently admissible.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida task ready".to_string()),
                }),
                Some(next_action),
            )
        };

    if recovery_present {
        next_actions.push(
            "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
                .to_string(),
        );
    }

    if recovery_present && latest_runtime_consumption_kind != Some("final") {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::ExecutionPreparationGateBlocked,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "Materialize final execution-preparation evidence with `vida taskflow consume final \"<request>\" --json` before attempting continuation."
                .to_string(),
        );
    }

    blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let why_not_now = why_not_now.map(|mut summary| {
        summary.blocker_codes = blocker_codes.clone();
        summary
    });
    let status = if blocker_codes.is_empty() {
        "pass".to_string()
    } else {
        "blocked".to_string()
    };

    TaskflowNextDecision {
        status,
        blocker_codes,
        next_actions,
        recommended_command,
        recommended_surface,
        primary_ready_task,
        candidate_task_context,
        why_not_now,
        next_action,
    }
}

fn taskflow_next_operator_contracts(
    decision: &TaskflowNextDecision,
    latest_run_graph: Option<&crate::state_store::RunGraphStatus>,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    let artifact_refs = serde_json::json!({
        "surface": "vida taskflow next",
        "latest_run_graph_run_id": latest_run_graph.map(|status| status.run_id.as_str()),
        "recovery_run_id": recovery.map(|summary| summary.run_id.as_str()),
        "dispatch_run_id": dispatch.map(|summary| summary.run_id.as_str()),
    });
    let shared_fields = serde_json::json!({
        "status": decision.status,
        "blocker_codes": decision.blocker_codes,
        "next_actions": decision.next_actions,
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
    (shared_fields, operator_contracts, artifact_refs)
}

fn task_wave_label(
    task_id: &str,
    by_id: &BTreeMap<String, crate::state_store::TaskRecord>,
    memo: &mut BTreeMap<String, Option<String>>,
    active: &mut BTreeSet<String>,
) -> Option<String> {
    if let Some(cached) = memo.get(task_id) {
        return cached.clone();
    }
    if !active.insert(task_id.to_string()) {
        return None;
    }

    let resolved = by_id.get(task_id).and_then(|task| {
        task.execution_semantics
            .order_bucket
            .clone()
            .or_else(|| {
                task.labels
                    .iter()
                    .find(|label| label.as_str() == "wave" || label.starts_with("wave-"))
                    .cloned()
            })
            .or_else(|| {
                task.dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type == "parent-child")
                    .find_map(|dependency| {
                        task_wave_label(&dependency.depends_on_id, by_id, memo, active)
                    })
            })
    });

    active.remove(task_id);
    memo.insert(task_id.to_string(), resolved.clone());
    resolved
}

fn wave_sort_key(wave_id: &str) -> (usize, usize, &str) {
    if wave_id == "unassigned" {
        return (2, usize::MAX, wave_id);
    }
    if wave_id == "wave" {
        return (0, 0, wave_id);
    }
    if let Some(index) = wave_id.strip_prefix("wave-") {
        if let Ok(parsed) = index.parse::<usize>() {
            return (0, parsed, wave_id);
        }
    }
    (1, usize::MAX, wave_id)
}

fn build_graph_summary_waves(
    all_tasks: &[crate::state_store::TaskRecord],
    ready_tasks: &[crate::state_store::TaskRecord],
    blocked_tasks: &[crate::state_store::BlockedTaskRecord],
) -> Vec<GraphSummaryWaveBucket> {
    #[derive(Default)]
    struct WaveAccumulator {
        task_ids: BTreeSet<String>,
        ready_count: usize,
        blocked_count: usize,
        primary_ready_task: Option<GraphSummaryTaskRef>,
        primary_blocked_task: Option<GraphSummaryTaskRef>,
    }

    let by_id = all_tasks
        .iter()
        .cloned()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    let mut memo = BTreeMap::<String, Option<String>>::new();
    let mut active = BTreeSet::<String>::new();
    let mut buckets = BTreeMap::<String, WaveAccumulator>::new();

    for task in all_tasks {
        if task.issue_type == "epic" || task.status == "closed" {
            continue;
        }
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        buckets
            .entry(wave_id)
            .or_default()
            .task_ids
            .insert(task.id.clone());
    }

    for task in ready_tasks {
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.ready_count += 1;
        if bucket.primary_ready_task.is_none() {
            bucket.primary_ready_task = Some(graph_summary_task_ref(task));
        }
    }

    for record in blocked_tasks {
        let wave_id = task_wave_label(&record.task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.blocked_count += 1;
        if bucket.primary_blocked_task.is_none() {
            bucket.primary_blocked_task = Some(graph_summary_task_ref(&record.task));
        }
    }

    let mut waves = buckets
        .into_iter()
        .map(|(wave_id, bucket)| GraphSummaryWaveBucket {
            wave_id,
            task_count: bucket.task_ids.len(),
            ready_count: bucket.ready_count,
            blocked_count: bucket.blocked_count,
            primary_ready_task: bucket.primary_ready_task,
            primary_blocked_task: bucket.primary_blocked_task,
        })
        .collect::<Vec<_>>();
    waves.sort_by(|left, right| {
        wave_sort_key(&left.wave_id)
            .cmp(&wave_sort_key(&right.wave_id))
            .then_with(|| right.ready_count.cmp(&left.ready_count))
            .then_with(|| right.blocked_count.cmp(&left.blocked_count))
            .then_with(|| left.wave_id.cmp(&right.wave_id))
    });
    waves
}

fn parse_taskflow_next_args(
    args: &[String],
) -> Result<(bool, Option<&str>, Option<PathBuf>), &'static str> {
    if !matches!(args.first().map(String::as_str), Some("next")) {
        return Err("Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]");
    }

    let mut as_json = false;
    let mut scope_task_id = None;
    let mut state_dir = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--scope" => {
                let Some(task_id) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                scope_task_id = Some(task_id.as_str());
                index += 2;
            }
            "--state-dir" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                state_dir = Some(PathBuf::from(path));
                index += 2;
            }
            "--help" | "-h" if index == 1 && args.len() == 2 => {
                return Ok((false, Some("__help__"), None));
            }
            _ => {
                return Err(
                    "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                );
            }
        }
    }

    Ok((as_json, scope_task_id, state_dir))
}

async fn route_taskflow_doctor(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Doctor(doctor_args)) => {
                crate::doctor_surface::run_doctor(doctor_args).await
            }
            _ => {
                eprintln!("Unsupported `vida taskflow doctor` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_status(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Status(mut status_args)) => {
                if status_args.state_dir.is_none() {
                    let state_dir = match resolve_taskflow_proxy_state_dir(None) {
                        Ok(state_dir) => state_dir,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    status_args.state_dir = Some(state_dir);
                }
                crate::status_surface::run_status(status_args).await
            }
            _ => {
                eprintln!("Unsupported `vida taskflow status` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_ready(command: TaskReadyArgs) -> ExitCode {
    let state_dir = match resolve_taskflow_proxy_state_dir(command.state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    match crate::task_surface::ready_tasks_scoped_read_only(state_dir, command.scope.as_deref())
        .await
    {
        Ok(tasks) => {
            let payload =
                serde_json::to_value(&tasks).expect("taskflow ready payload should serialize");
            if surface_render::print_surface_json(
                &payload,
                command.json,
                "taskflow ready payload should render as json",
            ) {
                return ExitCode::SUCCESS;
            }

            print_surface_header(command.render, "vida taskflow task ready");
            if tasks.is_empty() {
                print_surface_line(command.render, "ready tasks", "none");
                return ExitCode::SUCCESS;
            }

            for task in tasks {
                println!("{}\t{}\t{}", task.id, task.status, task.title);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to compute taskflow ready tasks: {error}");
            ExitCode::from(1)
        }
    }
}

fn taskflow_task_subcommand_supported(subcommand: &str) -> bool {
    matches!(
        subcommand,
        "help"
            | "import-jsonl"
            | "replace-jsonl"
            | "export-jsonl"
            | "list"
            | "show"
            | "ready"
            | "next"
            | "next-display-id"
            | "create"
            | "update"
            | "close"
            | "split"
            | "spawn-blocker"
            | "deps"
            | "reverse-deps"
            | "blocked"
            | "children"
            | "reparent-children"
            | "move-children"
            | "tree"
            | "subtree"
            | "validate-graph"
            | "dep"
            | "critical-path"
    )
}

async fn run_taskflow_replan_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow replan split <task-id> --child <task-id>:<title> --child <task-id>:<title> --reason <text> [--apply] [--json]\n       vida taskflow replan spawn-blocker <task-id> <blocker-task-id> <title> --reason <text> [--apply] [--json]";
    match args.get(1).map(String::as_str) {
        None | Some("--help" | "-h") => {
            eprintln!("{usage}");
            return ExitCode::SUCCESS;
        }
        Some("split" | "spawn-blocker") => {}
        Some(_) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    }

    let apply = args.iter().any(|value| value == "--apply");
    let mut argv = vec!["vida".to_string(), "task".to_string()];
    argv.push(
        args.get(1)
            .expect("validated replan subcommand should exist")
            .clone(),
    );
    argv.extend(
        args.iter()
            .skip(2)
            .filter(|value| value.as_str() != "--apply")
            .cloned(),
    );
    if !apply {
        argv.push("--dry-run".to_string());
    }

    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => crate::task_surface::run_task(task_args).await,
            _ => {
                eprintln!(
                    "Unsupported `vida taskflow replan` arguments. This preview surface must map to canonical `vida task` mutation commands."
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_task(args: &[String]) -> ExitCode {
    if let Some(subcommand) = args.get(1).map(String::as_str) {
        if !subcommand.starts_with('-') && !taskflow_task_subcommand_supported(subcommand) {
            eprintln!(
                "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to the external TaskFlow runtime."
            );
            return ExitCode::from(2);
        }
    }

    let mut argv = vec!["vida".to_string(), "task".to_string()];
    if matches!(args.get(1).map(String::as_str), Some("close")) {
        argv.push("close".to_string());
        argv.push("--source".to_string());
        argv.push("vida taskflow task close".to_string());
        argv.extend(args.iter().skip(2).cloned());
    } else {
        argv.extend(args.iter().skip(1).cloned());
    }
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => match task_args.command {
                TaskCommand::Ready(command) => route_taskflow_ready(command).await,
                _ => crate::task_surface::run_task(task_args).await,
            },
            _ => {
                eprintln!(
                    "Unsupported `vida taskflow task` subcommand. This compatibility alias must match the canonical `vida task` contract."
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn resolve_taskflow_proxy_state_dir(state_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    match state_dir {
        Some(state_dir) => Ok(state_dir),
        None => crate::resolve_runtime_project_root()
            .map(|project_root| project_root.join(crate::state_store::default_state_dir())),
    }
}

pub(crate) async fn run_taskflow_next_surface(args: &[String]) -> ExitCode {
    let (as_json, scope_task_id, state_dir) = match parse_taskflow_next_args(args) {
        Ok((_, Some("__help__"), _)) => {
            print_taskflow_proxy_help(Some("next"));
            return ExitCode::SUCCESS;
        }
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };

    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let runtime_consumption = match crate::runtime_consumption_summary(&state_dir) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to summarize runtime-consumption state: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks =
        match crate::task_surface::ready_tasks_scoped_read_only(state_dir.clone(), scope_task_id)
            .await
        {
            Ok(tasks) => tasks,
            Err(error) => {
                eprintln!("Failed to compute ready tasks: {error}");
                return ExitCode::from(1);
            }
        };

    let store = match crate::task_surface::open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => Some(store),
        Err(error) => {
            let message = error.to_string();
            if message.contains("LOCK") || message.contains("lock") {
                None
            } else {
                eprintln!("Failed to open authoritative state store: {error}");
                return ExitCode::from(1);
            }
        }
    };
    let latest_run_graph = match store.as_ref() {
        Some(store) => match store.latest_run_graph_status().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest run-graph status: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let recovery = match store.as_ref() {
        Some(store) => match store.latest_run_graph_recovery_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest recovery summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let gate = match store.as_ref() {
        Some(store) => match store.latest_run_graph_gate_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest gate summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let dispatch = match store.as_ref() {
        Some(store) => match store.latest_run_graph_dispatch_receipt_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest dispatch receipt summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let explicit_binding = match store.as_ref() {
        Some(store) => match store.latest_explicit_run_graph_continuation_binding().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest explicit continuation binding: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let recovery_holds_active_bound_run = recovery_holds_active_bound_run(recovery.as_ref());
    let decision = build_taskflow_next_decision(
        ready_tasks.first(),
        recovery_holds_active_bound_run,
        recovery.is_some(),
        runtime_consumption.latest_kind.as_deref(),
        scope_task_id,
        dispatch.as_ref(),
        latest_run_graph.as_ref(),
        explicit_binding.as_ref(),
        crate::latest_terminal_consume_continue_snapshot_run_id(&state_dir)
            .ok()
            .flatten()
            .as_deref(),
    );
    let (shared_fields, operator_contracts, artifact_refs) = taskflow_next_operator_contracts(
        &decision,
        latest_run_graph.as_ref(),
        recovery.as_ref(),
        dispatch.as_ref(),
    );
    let payload = serde_json::json!({
        "surface": "vida taskflow next",
        "status": decision.status,
        "artifact_refs": artifact_refs,
        "blocker_codes": decision.blocker_codes,
        "why_not_now": decision.why_not_now,
        "next_action": decision.next_action,
        "next_actions": decision.next_actions,
        "recommended_command": decision.recommended_command,
        "recommended_surface": decision.recommended_surface,
        "scope_task_id": scope_task_id,
        "ready_count": ready_tasks.len(),
        "primary_ready_task": decision.primary_ready_task,
        "candidate_task_context": decision.candidate_task_context,
        "latest_run_graph": latest_run_graph,
        "continuation_binding": explicit_binding,
        "recovery": recovery,
        "gate": gate,
        "dispatch": dispatch,
        "runtime_consumption": runtime_consumption,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow next");
        crate::print_surface_line(RenderMode::Plain, "status", &decision.status);
        if !decision.blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &decision.blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        if let Some(task_id) = scope_task_id {
            crate::print_surface_line(RenderMode::Plain, "scope_task_id", task_id);
        }
        if let Some(task) = payload
            .get("primary_ready_task")
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", task);
            if let Some(title) = payload
                .get("primary_ready_task")
                .and_then(|value| value.get("title"))
                .and_then(serde_json::Value::as_str)
            {
                crate::print_surface_line(RenderMode::Plain, "title", title);
            }
        } else {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", "none");
        }
        if let Some(task) = payload
            .get("candidate_task_context")
            .and_then(|value| value.get("ready_head"))
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "candidate_ready_task", task);
        }
        if let Some(gate) = payload
            .get("candidate_task_context")
            .and_then(|value| value.get("admissibility_gate"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "admissibility_gate", gate);
        }
        if let Some(summary) = payload
            .get("why_not_now")
            .and_then(|value| value.get("summary"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "why_not_now", summary);
        }
        if let Some(command) = payload["recommended_command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_command", command);
        }
        if let Some(surface) = payload["recommended_surface"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_surface", surface);
        }
        if let Some(next_action) = payload
            .get("next_action")
            .and_then(|value| value.get("reason"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if decision.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_graph_summary(args: &[String]) -> ExitCode {
    let as_json = match args {
        [head] if head == "graph-summary" => false,
        [head, flag] if head == "graph-summary" && flag == "--json" => true,
        [head, flag] if head == "graph-summary" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("graph-summary"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: vida taskflow graph-summary [--json]");
            return ExitCode::from(2);
        }
    };

    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks = match store.ready_tasks_scoped(None).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute ready tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let blocked_tasks = match store.blocked_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute blocked tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let all_tasks = match store.list_tasks(None, true).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to list tasks for wave summary: {error}");
            return ExitCode::from(1);
        }
    };
    let critical_path = match store.critical_path().await {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to compute critical path: {error}");
            return ExitCode::from(1);
        }
    };
    let scheduling = match store.scheduling_projection_scoped(None, None).await {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute scheduling projection: {error}");
            return ExitCode::from(1);
        }
    };
    let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);

    let primary_ready_task = scheduling.ready.first().map(|candidate| {
        serde_json::json!({
            "task": graph_summary_task_ref(&candidate.task),
            "ready_now": candidate.ready_now,
            "ready_parallel_safe": candidate.ready_parallel_safe,
            "active_critical_path": candidate.active_critical_path,
            "parallel_blockers": candidate.parallel_blockers,
            "execution_semantics": candidate.task.execution_semantics,
        })
    });
    let primary_blocked_task = blocked_tasks.first().map(|record| {
        serde_json::json!({
            "id": record.task.id,
            "display_id": record.task.display_id,
            "title": record.task.title,
            "status": record.task.status,
            "priority": record.task.priority,
            "issue_type": record.task.issue_type,
            "blocker_count": record.blockers.len(),
            "blockers": record.blockers,
        })
    });

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();

    if let Some(task) = ready_tasks.first() {
        next_actions.push(format!(
            "Inspect the primary ready task with `vida task show {} --json` before dispatch.",
            task.id
        ));
    }
    if let Some(record) = blocked_tasks.first() {
        next_actions.push(format!(
            "Inspect the highest-priority blocked task with `vida task deps {} --json` before resequencing.",
            record.task.id
        ));
    }
    if critical_path.length > 0 {
        next_actions.push(
            "Inspect the current graph bottleneck with `vida task critical-path --json` before parallelizing additional work."
                .to_string(),
        );
    }
    if ready_tasks.is_empty() {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
    }
    if blocked_tasks.is_empty() && critical_path.length == 0 {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::TaskGraphEmpty,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "No active execution graph is present; inspect `vida task list --all --json` before sequencing new work."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let payload = serde_json::json!({
        "surface": "vida taskflow graph-summary",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "ready_count": ready_tasks.len(),
        "blocked_count": blocked_tasks.len(),
        "critical_path_length": critical_path.length,
        "current_task_id": scheduling.current_task_id,
        "primary_ready_task": primary_ready_task,
        "primary_blocked_task": primary_blocked_task,
        "scheduling": scheduling,
        "waves": waves,
        "critical_path": critical_path,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph-summary");
        crate::print_surface_line(RenderMode::Plain, "status", status);
        if !blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "blocked_count",
            &blocked_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "critical_path_length",
            &critical_path.length.to_string(),
        );
        crate::print_surface_line(RenderMode::Plain, "wave_count", &waves.len().to_string());
        if let Some(task) = ready_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", &task.id);
        }
        if let Some(task_id) = payload["current_task_id"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "current_task_id", task_id);
        }
        if let Some(record) = blocked_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_blocked_task", &record.task.id);
        }
        if let Some(wave) = waves.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_wave", &wave.wave_id);
        }
        if let Some(next_action) = next_actions.first() {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn graph_explain_task_ref(task: &crate::state_store::TaskRecord) -> serde_json::Value {
    serde_json::json!({
        "id": task.id,
        "display_id": task.display_id,
        "title": task.title,
        "status": task.status,
        "priority": task.priority,
        "issue_type": task.issue_type,
        "execution_semantics": task.execution_semantics,
    })
}

fn build_taskflow_graph_explain_payload(
    projection: &crate::state_store::TaskSchedulingProjection,
    scope_task_id: Option<&str>,
    target_task_id: Option<&str>,
) -> serde_json::Value {
    let selected_target = target_task_id
        .map(ToOwned::to_owned)
        .or_else(|| projection.current_task_id.clone())
        .or_else(|| {
            projection
                .ready
                .first()
                .map(|candidate| candidate.task.id.clone())
        })
        .or_else(|| {
            projection
                .blocked
                .first()
                .map(|candidate| candidate.task.id.clone())
        });
    let candidate = selected_target.as_deref().and_then(|task_id| {
        projection
            .ready
            .iter()
            .chain(projection.blocked.iter())
            .find(|candidate| candidate.task.id == task_id)
    });
    let candidate_missing = selected_target.is_some() && candidate.is_none();

    let ready_now = candidate
        .map(|candidate| candidate.ready_now)
        .unwrap_or(false);
    let ready_parallel_safe = candidate
        .map(|candidate| candidate.ready_parallel_safe)
        .unwrap_or(false);
    let blocked_by = candidate
        .map(|candidate| serde_json::json!(candidate.blocked_by))
        .unwrap_or_else(|| serde_json::json!([]));
    let parallel_blockers = candidate
        .map(|candidate| serde_json::json!(candidate.parallel_blockers))
        .unwrap_or_else(|| serde_json::json!([]));
    let active_critical_path = candidate
        .map(|candidate| candidate.active_critical_path)
        .unwrap_or(false);
    let selected_as_current = candidate
        .map(|candidate| Some(candidate.task.id.as_str()) == projection.current_task_id.as_deref())
        .unwrap_or(false);
    let selected_as_parallel_after_current = candidate
        .map(|candidate| {
            projection
                .parallel_candidates_after_current
                .iter()
                .any(|task| task.id == candidate.task.id)
        })
        .unwrap_or(false);

    let mut blocker_codes = Vec::<String>::new();
    let mut blocked_reasons = Vec::<String>::new();
    let mut ready_reasons = Vec::<String>::new();
    if candidate_missing {
        blocker_codes.push("task_not_in_graph_projection".to_string());
        blocked_reasons
            .push("target task is not present in the scoped scheduling projection".to_string());
    } else if let Some(candidate) = candidate {
        if candidate.ready_now {
            ready_reasons.push("no open non-parent dependency blockers".to_string());
        } else {
            blocker_codes.push("graph_blocked".to_string());
            blocked_reasons.extend(candidate.blocked_by.iter().map(|blocker| {
                format!(
                    "{} dependency `{}` is `{}`",
                    blocker.edge_type, blocker.depends_on_id, blocker.dependency_status
                )
            }));
        }
        if candidate.ready_now && !candidate.ready_parallel_safe {
            blocker_codes.extend(candidate.parallel_blockers.iter().cloned());
        }
    } else {
        blocker_codes.push("task_graph_empty".to_string());
        blocked_reasons
            .push("no open tasks are present in the scoped scheduling projection".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    let next_lawful_action = if let Some(candidate) = candidate {
        if candidate.ready_now {
            serde_json::json!({
                "surface": "vida taskflow scheduler dispatch",
                "command": "vida taskflow scheduler dispatch --json",
                "reason": "task is graph-ready; use scheduler dispatch to select the next bounded launch set"
            })
        } else {
            serde_json::json!({
                "surface": "vida task deps",
                "command": format!("vida task deps {} --json", candidate.task.id),
                "reason": "task is blocked by open graph dependencies"
            })
        }
    } else {
        serde_json::json!({
            "surface": "vida task ready",
            "command": "vida task ready --json",
            "reason": "no explainable target was found in the scoped projection"
        })
    };

    let status = if candidate_missing || !blocker_codes.is_empty() || !ready_now {
        "blocked"
    } else {
        "pass"
    };
    serde_json::json!({
        "surface": "vida taskflow graph explain",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": [next_lawful_action["reason"].clone()],
        "scope_task_id": scope_task_id,
        "task_id": selected_target,
        "current_task_id": projection.current_task_id,
        "task": candidate.map(|candidate| graph_explain_task_ref(&candidate.task)),
        "ready_now": ready_now,
        "ready_reasons": ready_reasons,
        "blocked_by": blocked_by,
        "blocked_reasons": blocked_reasons,
        "ready_parallel_safe": ready_parallel_safe,
        "parallel_blockers": parallel_blockers,
        "active_critical_path": active_critical_path,
        "selected_as_current": selected_as_current,
        "selected_as_parallel_after_current": selected_as_parallel_after_current,
        "parallel_candidates_after_current": projection
            .parallel_candidates_after_current
            .iter()
            .map(graph_explain_task_ref)
            .collect::<Vec<_>>(),
        "next_lawful_action": next_lawful_action,
        "projection_source": "StateStore::scheduling_projection_scoped",
        "truth_source": "canonical_task_graph_scheduler_projection",
    })
}

async fn run_taskflow_graph_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow graph explain [task-id] [--scope <task-id>] [--current-task-id <task-id>] [--json]";
    if matches!(
        args,
        [head, flag] if head == "graph" && matches!(flag.as_str(), "--help" | "-h")
    ) || matches!(
        args,
        [head, subcommand, flag]
            if head == "graph"
                && subcommand == "explain"
                && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("graph"));
        return ExitCode::SUCCESS;
    }
    if !matches!(args.first().map(String::as_str), Some("graph"))
        || !matches!(args.get(1).map(String::as_str), Some("explain"))
    {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let mut target_task_id = None::<String>;
    let mut scope_task_id = None::<String>;
    let mut current_task_id = None::<String>;
    let mut as_json = false;
    let mut index = 2usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--scope" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                scope_task_id = Some(value.clone());
                index += 2;
            }
            "--current-task-id" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                current_task_id = Some(value.clone());
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--help" | "-h" => {
                print_taskflow_proxy_help(Some("graph"));
                return ExitCode::SUCCESS;
            }
            value if !value.starts_with('-') && target_task_id.is_none() => {
                target_task_id = Some(value.to_string());
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }

    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let projection = match store
        .scheduling_projection_scoped(scope_task_id.as_deref(), current_task_id.as_deref())
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute graph explain projection: {error}");
            return ExitCode::from(1);
        }
    };
    let payload = build_taskflow_graph_explain_payload(
        &projection,
        scope_task_id.as_deref(),
        target_task_id.as_deref(),
    );

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph explain");
        crate::print_surface_line(
            RenderMode::Plain,
            "status",
            payload["status"].as_str().unwrap_or("blocked"),
        );
        if let Some(task_id) = payload["task_id"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "task_id", task_id);
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_now",
            if payload["ready_now"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_parallel_safe",
            if payload["ready_parallel_safe"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
        );
        if let Some(action) = payload["next_lawful_action"]["command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "next_lawful_action", action);
        }
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_scheduler_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow scheduler dispatch [--scope <task-id>] [--current-task-id <task-id>] [--state-dir <path>] [--limit <n>] [--dry-run] [--execute] [--json]\n       vida taskflow scheduler reservations [--state-dir <path>] [--json]\n       vida taskflow scheduler reservation <reservation-id> [--state-dir <path>] [--json]";
    if matches!(
        args,
        [head, flag] if head == "scheduler" && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("scheduler"));
        return ExitCode::SUCCESS;
    }
    if matches!(
        args,
        [head, subcommand, flag]
            if head == "scheduler"
                && subcommand == "dispatch"
                && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("scheduler"));
        return ExitCode::SUCCESS;
    }
    if matches!(args.get(1).map(String::as_str), Some("reservations")) {
        return run_taskflow_scheduler_reservations_surface(args).await;
    }
    if matches!(args.get(1).map(String::as_str), Some("reservation")) {
        return run_taskflow_scheduler_reservation_surface(args).await;
    }
    if !matches!(args.first().map(String::as_str), Some("scheduler"))
        || !matches!(args.get(1).map(String::as_str), Some("dispatch"))
    {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let mut scope_task_id = None::<String>;
    let mut current_task_id = None::<String>;
    let mut state_dir = None::<PathBuf>;
    let mut as_json = false;
    let mut dry_run = false;
    let mut execute_requested = false;
    let mut requested_parallel_limit = None::<u64>;

    let mut index = 2usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--scope" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                scope_task_id = Some(value.clone());
                index += 2;
            }
            "--current-task-id" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                current_task_id = Some(value.clone());
                index += 2;
            }
            "--state-dir" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                state_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--limit" | "--max-parallel-agents" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                let Ok(parsed) = value.parse::<u64>() else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                if parsed == 0 {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                }
                requested_parallel_limit = Some(parsed);
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--execute" => {
                execute_requested = true;
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }

    if dry_run && execute_requested {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let store = match crate::task_surface::open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let mut plan = match build_taskflow_scheduler_dispatch_plan_from_store(
        &store,
        &state_dir,
        scope_task_id.as_deref(),
        current_task_id.as_deref(),
        requested_parallel_limit,
        dry_run || !execute_requested,
        execute_requested,
    )
    .await
    {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let recovery = if execute_requested && !dry_run {
        Some(match store.latest_run_graph_recovery_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest recovery summary: {error}");
                return ExitCode::from(1);
            }
        })
    } else {
        None
    };
    let dispatch = if execute_requested && !dry_run {
        Some(
            match store.latest_run_graph_dispatch_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("Failed to read latest dispatch receipt summary: {error}");
                    return ExitCode::from(1);
                }
            },
        )
    } else {
        None
    };

    let runtime_gate_blockers = scheduler_execute_runtime_gate_blocker_codes(
        recovery.as_ref().and_then(|summary| summary.as_ref()),
        dispatch.as_ref().and_then(|summary| summary.as_ref()),
    );
    if execute_requested && !dry_run {
        match build_scheduler_packet_backed_execution_gate(&store, &plan).await {
            Ok(gate) => {
                plan.packet_backed_execution_supported = gate.supported;
                plan.packet_backed_execution_status = gate.status.clone();
                plan.dispatch_receipt.packet_backed_execution_supported = gate.supported;
                plan.dispatch_receipt.packet_backed_execution_status = gate.status.clone();
                plan.dispatch_receipt.packet_backed_execution_gate = Some(gate.clone());
                plan.packet_backed_execution_gate = Some(gate);
            }
            Err(error) => {
                eprintln!("Failed to evaluate scheduler packet-backed execution gate: {error}");
                return ExitCode::from(1);
            }
        }
    }

    drop(store);
    if execute_requested && !dry_run && runtime_gate_blockers.blocker_codes.is_empty() {
        if let Err(error) = persist_scheduler_execute_receipt(&mut plan, &state_dir).await {
            plan.status = "blocked".to_string();
            let reservation_collision = error.contains("scheduler_task_already_reserved")
                || error.contains("scheduler_conflict_domain_reserved");
            plan.execution_status = if reservation_collision {
                "execution_preparation_gate_blocked".to_string()
            } else {
                "scheduler_execute_receipt_persistence_failed".to_string()
            };
            plan.execute_supported = true;
            plan.execution_attempted = false;
            if reservation_collision {
                plan.blocker_codes
                    .push("execution_preparation_gate_blocked".to_string());
                plan.dispatch_receipt
                    .execution_blocker_codes
                    .push("execution_preparation_gate_blocked".to_string());
                plan.next_actions.push(
                    "An active scheduler reservation already owns the selected task or conflict domain; inspect `vida taskflow scheduler reservations --json` before retrying."
                        .to_string(),
                );
            } else {
                plan.blocker_codes
                    .push("scheduler_execute_receipt_persistence_failed".to_string());
            }
            plan.next_actions.push(error);
        }
    } else if execute_requested && !dry_run {
        apply_scheduler_execute_runtime_gate_blockers(&mut plan, &runtime_gate_blockers);
    }

    if as_json {
        crate::print_json_pretty(
            &serde_json::to_value(&plan).expect("scheduler dispatch plan should serialize"),
        );
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow scheduler dispatch");
        print_surface_line(RenderMode::Plain, "status", &plan.status);
        print_surface_line(
            RenderMode::Plain,
            "max_parallel_agents",
            &plan.max_parallel_agents.to_string(),
        );
        print_surface_line(
            RenderMode::Plain,
            "selection_source",
            &plan.selection_source,
        );
        print_surface_line(
            RenderMode::Plain,
            "selected_task_count",
            &plan.selected_task_ids.len().to_string(),
        );
        if let Some(task) = plan.selected_primary_task.as_ref() {
            print_surface_line(RenderMode::Plain, "selected_primary_task", &task.id);
        }
        if !plan.selected_parallel_tasks.is_empty() {
            print_surface_line(
                RenderMode::Plain,
                "selected_parallel_tasks",
                &plan
                    .selected_parallel_tasks
                    .iter()
                    .map(|task| task.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !plan.blocker_codes.is_empty() {
            print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &plan.blocker_codes.join(", "),
            );
        }
        if let Some(next_action) = plan.next_actions.first() {
            print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if plan.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_scheduler_reservations_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow scheduler reservations [--state-dir <path>] [--json]";
    let mut state_dir = None::<PathBuf>;
    let mut as_json = false;
    let mut index = 2usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--state-dir" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                state_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }
    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let store = match crate::task_surface::open_read_only_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let reservations = match store.active_scheduler_dispatch_reservations().await {
        Ok(reservations) => reservations,
        Err(error) => {
            eprintln!("Failed to read scheduler reservations: {error}");
            return ExitCode::from(1);
        }
    };
    if as_json {
        crate::print_json_pretty(&serde_json::json!({
            "status": "pass",
            "surface": "vida taskflow scheduler reservations",
            "reservation_count": reservations.len(),
            "reservations": reservations,
            "blocker_codes": [],
            "next_actions": [],
        }));
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow scheduler reservations");
        print_surface_line(
            RenderMode::Plain,
            "reservation_count",
            &reservations.len().to_string(),
        );
        for reservation in reservations {
            print_surface_line(
                RenderMode::Plain,
                "reservation",
                &format!(
                    "{} task={} status={}",
                    reservation.reservation_id, reservation.task_id, reservation.lease_status
                ),
            );
        }
    }
    ExitCode::SUCCESS
}

async fn run_taskflow_scheduler_reservation_surface(args: &[String]) -> ExitCode {
    let usage =
        "Usage: vida taskflow scheduler reservation <reservation-id> [--state-dir <path>] [--json]";
    let Some(reservation_id) = args.get(2).cloned() else {
        eprintln!("{usage}");
        return ExitCode::from(2);
    };
    let mut state_dir = None::<PathBuf>;
    let mut as_json = false;
    let mut index = 3usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--state-dir" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                state_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }
    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let store = match crate::task_surface::open_read_only_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let reservation = match store.scheduler_dispatch_reservation(&reservation_id).await {
        Ok(reservation) => reservation,
        Err(error) => {
            eprintln!("Failed to read scheduler reservation: {error}");
            return ExitCode::from(1);
        }
    };
    let status = if reservation.is_some() {
        "pass"
    } else {
        "blocked"
    };
    let blocker_codes = if reservation.is_some() {
        Vec::<String>::new()
    } else {
        vec!["scheduler_reservation_not_found".to_string()]
    };
    if as_json {
        crate::print_json_pretty(&serde_json::json!({
            "status": status,
            "surface": "vida taskflow scheduler reservation",
            "reservation_id": reservation_id,
            "reservation": reservation,
            "blocker_codes": blocker_codes,
            "next_actions": [],
        }));
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow scheduler reservation");
        print_surface_line(RenderMode::Plain, "status", status);
        print_surface_line(RenderMode::Plain, "reservation_id", &reservation_id);
    }
    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteDiagnosticMode {
    Explain,
    ModelProfileReadinessAudit,
    ValidateRouting,
    ConfigActuationCensus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskflowRouteDiagnosticArgs {
    mode: RouteDiagnosticMode,
    run_id: Option<String>,
    dispatch_target: Option<String>,
    runtime_role: Option<String>,
    as_json: bool,
}

fn parse_taskflow_route_diagnostic_args(
    args: &[String],
) -> Result<TaskflowRouteDiagnosticArgs, &'static str> {
    let (mode, mut index) = match args {
        [head, subcommand, ..] if head == "route" && subcommand == "explain" => {
            (RouteDiagnosticMode::Explain, 2)
        }
        [head, subcommand, ..] if head == "route" && subcommand == "model-profile-readiness" => {
            (RouteDiagnosticMode::ModelProfileReadinessAudit, 2)
        }
        [head, ..] if head == "validate-routing" => (RouteDiagnosticMode::ValidateRouting, 1),
        [head, subcommand, ..] if head == "config-actuation" && subcommand == "census" => {
            (RouteDiagnosticMode::ConfigActuationCensus, 2)
        }
        _ => {
            return Err(
                "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow route model-profile-readiness [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
            );
        }
    };

    let mut parsed = TaskflowRouteDiagnosticArgs {
        mode,
        run_id: None,
        dispatch_target: None,
        runtime_role: None,
        as_json: false,
    };
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                parsed.as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --run-id");
                };
                parsed.run_id = Some(value.clone());
                index += 2;
            }
            "--dispatch-target" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --dispatch-target");
                };
                parsed.dispatch_target = Some(value.clone());
                index += 2;
            }
            "--runtime-role" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --runtime-role");
                };
                parsed.runtime_role = Some(value.clone());
                index += 2;
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow route model-profile-readiness [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
                );
            }
            _ => {
                return Err(
                    "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow route model-profile-readiness [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
                );
            }
        }
    }
    if parsed.dispatch_target.is_some() && parsed.runtime_role.is_some() {
        return Err("Use either --dispatch-target or --runtime-role, not both.");
    }
    if matches!(
        parsed.mode,
        RouteDiagnosticMode::ValidateRouting | RouteDiagnosticMode::ConfigActuationCensus
    ) && (parsed.dispatch_target.is_some() || parsed.runtime_role.is_some())
    {
        return Err(
            "vida taskflow validate-routing and config-actuation census inspect all routed lanes and do not accept --dispatch-target or --runtime-role.",
        );
    }
    Ok(parsed)
}

async fn latest_or_requested_dispatch_context(
    store: &crate::state_store::StateStore,
    run_id: Option<&str>,
) -> Result<Option<crate::state_store::RunGraphDispatchContext>, crate::state_store::StateStoreError>
{
    let run_id = match run_id {
        Some(run_id) => Some(run_id.to_string()),
        None => store.latest_run_graph_run_id().await?,
    };
    let Some(run_id) = run_id else {
        return Ok(None);
    };
    store.run_graph_dispatch_context(&run_id).await
}

fn string_array_from_payload(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn normalize_taskflow_diagnostic_operator_contract_payload(
    mut payload: serde_json::Value,
    fallback_blocked_next_action: &str,
) -> Result<serde_json::Value, String> {
    let blocker_codes = string_array_from_payload(&payload["blocker_codes"]);
    let blocker_codes = crate::operator_contracts::normalize_blocker_codes(
        &blocker_codes,
        crate::release_contract_adapters::canonical_blocker_codes,
        crate::contract_profile_adapter::blocker_code(
            crate::release1_contracts::BlockerCode::Unsupported,
        ),
    );
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        let mut next_actions = string_array_from_payload(&payload["next_actions"]);
        if next_actions.is_empty() {
            next_actions.push(fallback_blocked_next_action.to_string());
        }
        next_actions
    };
    let artifact_refs = payload
        .get("artifact_refs")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        artifact_refs,
    )?;
    let Some(object) = payload.as_object_mut() else {
        return Err("taskflow diagnostic payload must be a JSON object".to_string());
    };
    object.insert(
        "status".to_string(),
        serde_json::Value::String(finalized.status.to_string()),
    );
    object.insert(
        "blocker_codes".to_string(),
        serde_json::to_value(finalized.blocker_codes)
            .expect("normalized blocker_codes should serialize"),
    );
    object.insert(
        "next_actions".to_string(),
        serde_json::to_value(finalized.next_actions)
            .expect("normalized next_actions should serialize"),
    );
    object.insert("artifact_refs".to_string(), finalized.artifact_refs);
    object.insert("shared_fields".to_string(), finalized.shared_fields);
    object.insert(
        "operator_contracts".to_string(),
        finalized.operator_contracts,
    );
    Ok(payload)
}

fn normalize_taskflow_route_diagnostic_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    normalize_taskflow_diagnostic_operator_contract_payload(
        payload,
        "inspect route diagnostic blockers with `vida taskflow route explain --json` or `vida taskflow validate-routing --json`",
    )
}

fn execution_plan_from_dispatch_context(
    context: &crate::state_store::RunGraphDispatchContext,
) -> Option<&serde_json::Value> {
    context
        .role_selection
        .get("execution_plan")
        .filter(|value| value.is_object())
}

fn selected_backend_readiness_payload(
    selected_backend: &str,
    preferred_profile_id: Option<&str>,
) -> Option<serde_json::Value> {
    let project_root = crate::state_store::repo_root();
    let overlay =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root).ok()?;
    let backend_entry =
        crate::yaml_lookup(&overlay, &["agent_system", "subagents", selected_backend])?;
    let backend_class = crate::yaml_lookup(backend_entry, &["subagent_backend_class"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if backend_class != "external_cli" {
        return None;
    }
    Some(
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            selected_backend,
            backend_entry,
            preferred_profile_id,
        ),
    )
}

fn collect_model_profiles_from_yaml(
    value: &serde_yaml::Value,
    profiles: &mut BTreeMap<String, BTreeSet<String>>,
) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            if let Some(model_profiles) =
                mapping.get(serde_yaml::Value::String("model_profiles".to_string()))
            {
                if let serde_yaml::Value::Mapping(profile_mapping) = model_profiles {
                    for (profile_id, profile_value) in profile_mapping {
                        let Some(profile_id) = profile_id.as_str().map(str::trim) else {
                            continue;
                        };
                        if profile_id.is_empty() {
                            continue;
                        }
                        let model_ref = crate::yaml_lookup(profile_value, &["model_ref"])
                            .and_then(serde_yaml::Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .unwrap_or(profile_id);
                        profiles
                            .entry(profile_id.to_string())
                            .or_default()
                            .insert(model_ref.to_string());
                    }
                }
            }
            for child in mapping.values() {
                collect_model_profiles_from_yaml(child, profiles);
            }
        }
        serde_yaml::Value::Sequence(values) => {
            for child in values {
                collect_model_profiles_from_yaml(child, profiles);
            }
        }
        _ => {}
    }
}

fn current_project_model_profile_catalog() -> BTreeMap<String, BTreeSet<String>> {
    let project_root = crate::state_store::repo_root();
    let Ok(overlay) =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root)
    else {
        return BTreeMap::new();
    };
    let mut profiles = BTreeMap::new();
    collect_model_profiles_from_yaml(&overlay, &mut profiles);
    profiles
}

fn route_assignment_catalog_drift_payload(route: &serde_json::Value) -> Option<serde_json::Value> {
    let selected_profile = route["selected_model_profile_id"].as_str()?.trim();
    if selected_profile.is_empty() {
        return None;
    }
    let selected_model_ref = route["selected_model_ref"].as_str().map(str::trim);
    let catalog = current_project_model_profile_catalog();
    if catalog.is_empty() {
        return None;
    }
    let Some(current_model_refs) = catalog.get(selected_profile) else {
        return Some(serde_json::json!({
            "status": "blocked",
            "reason": "selected_model_profile_not_in_current_config",
            "selected_model_profile_id": selected_profile,
            "selected_model_ref": selected_model_ref,
            "current_model_refs": serde_json::Value::Null,
        }));
    };
    if let Some(selected_model_ref) = selected_model_ref {
        if !selected_model_ref.is_empty() && !current_model_refs.contains(selected_model_ref) {
            return Some(serde_json::json!({
                "status": "blocked",
                "reason": "selected_model_ref_mismatch_current_config",
                "selected_model_profile_id": selected_profile,
                "selected_model_ref": selected_model_ref,
                "current_model_refs": current_model_refs,
            }));
        }
    }
    None
}

fn route_payload_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> serde_json::Value {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        execution_plan,
        dispatch_target,
    );
    let mut payload =
        crate::taskflow_routing::route_explain_payload(execution_plan, dispatch_target, route);
    let preferred_profile_id = payload["selected_model_profile_id"].as_str();
    if let Some(selected_backend) = payload["selected_backend"].as_str() {
        if let Some(readiness) =
            selected_backend_readiness_payload(selected_backend, preferred_profile_id)
        {
            let readiness_blockers = if readiness["blocked"].as_bool().unwrap_or(false) {
                serde_json::json!([{
                    "backend_id": readiness["backend_id"].clone(),
                    "status": readiness["status"].clone(),
                    "blocker_code": readiness["blocker_code"].clone(),
                    "selected_model_profile": readiness["selected_model_profile"].clone(),
                    "next_actions": readiness["next_actions"].clone(),
                }])
            } else {
                serde_json::json!([])
            };
            if let Some(object) = payload.as_object_mut() {
                object.insert("selected_backend_readiness".to_string(), readiness);
                object.insert("readiness_blockers".to_string(), readiness_blockers);
            }
        }
    }
    let selected_backend = payload["selected_backend"].as_str();
    let admissible = selected_backend.map(|backend| {
        crate::runtime_dispatch_state::backend_is_admissible_or_runtime_selected_carrier_for_dispatch_target(
            execution_plan,
            backend,
            dispatch_target,
        )
    });
    let status = crate::taskflow_routing::route_explain_status(&payload, admissible);
    let blocker_codes = crate::taskflow_routing::route_explain_blocker_codes(&payload, admissible);
    if let Some(object) = payload.as_object_mut() {
        object.insert("status".to_string(), serde_json::Value::String(status));
        object.insert(
            "blocker_codes".to_string(),
            serde_json::to_value(blocker_codes)
                .expect("route explain blocker codes should serialize"),
        );
        object.insert(
            "selected_backend_admissible".to_string(),
            admissible.map_or(serde_json::Value::Null, serde_json::Value::Bool),
        );
    }
    if let Some(drift) = route_assignment_catalog_drift_payload(&payload) {
        let mut blocker_codes = string_array_from_payload(&payload["blocker_codes"]);
        blocker_codes.push("model_not_pinned".to_string());
        blocker_codes.sort();
        blocker_codes.dedup();
        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "status".to_string(),
                serde_json::Value::String("blocked".to_string()),
            );
            object.insert(
                "blocker_codes".to_string(),
                serde_json::to_value(blocker_codes)
                    .expect("route catalog drift blocker codes should serialize"),
            );
            object.insert("route_assignment_catalog_drift".to_string(), drift);
            object.insert(
                "next_actions".to_string(),
                serde_json::json!([
                    "refresh the run graph dispatch context or reseed the route assignment from the current carrier catalog before trusting routing diagnostics"
                ]),
            );
        }
    }
    payload
}

fn route_validate_targets(execution_plan: &serde_json::Value) -> Vec<String> {
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let mut targets =
        crate::taskflow_routing::dispatch_contract_execution_lane_sequence(dispatch_contract);
    if targets.is_empty() {
        targets.extend(
            ["implementation", "coach", "verification"]
                .into_iter()
                .map(str::to_string),
        );
    }
    let mut unique = BTreeSet::new();
    targets
        .into_iter()
        .map(|target| match target.as_str() {
            "implementer" | "analysis" => "implementation".to_string(),
            "execution_preparation" => "architecture".to_string(),
            _ => target,
        })
        .filter(|target| !target.trim().is_empty())
        .filter(|target| unique.insert(target.clone()))
        .collect()
}

fn value_configured(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(values) => !values.is_empty(),
    }
}

fn config_actuation_proof_status(
    route: &serde_json::Value,
    field: &str,
    configured: bool,
) -> &'static str {
    if !configured {
        return "not_configured";
    }
    if field.ends_with("model_selection_enabled")
        && route["model_selection_enabled"].as_bool() == Some(false)
    {
        return "validated_blocking";
    }
    if field.ends_with("candidate_scope")
        && !matches!(
            route["candidate_scope"].as_str(),
            Some("unified_carrier_model_profiles") | None
        )
    {
        return "validated_blocking";
    }
    if route["runtime_assignment_enabled"].as_bool() == Some(false)
        && field.starts_with("carrier_runtime_assignment.")
    {
        return "validated_blocking";
    }
    "actuated_or_validated"
}

fn config_actuation_census_row(
    route: &serde_json::Value,
    config_key: &str,
    value_key: &str,
    validator: &str,
    runtime_consumer: &str,
) -> serde_json::Value {
    let value = route
        .get(value_key)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let configured = value_configured(&value);
    serde_json::json!({
        "config_key": config_key,
        "value": value,
        "configured": configured,
        "validator": validator,
        "runtime_consumer": runtime_consumer,
        "operator_surface": "vida taskflow route explain / vida taskflow validate-routing",
        "proof_status": config_actuation_proof_status(route, config_key, configured),
    })
}

fn config_actuation_census_rows_for_route(route: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut rows = vec![
        config_actuation_census_row(
            route,
            "executor_backend",
            "route_primary_backend",
            "selected_backend_from_execution_plan_route",
            "explicit_executor_backend_from_route",
        ),
        config_actuation_census_row(
            route,
            "fallback_executor_backend",
            "fallback_backend",
            "selected_backend_from_execution_plan_route",
            "fallback_executor_backend_from_route",
        ),
        config_actuation_census_row(
            route,
            "fanout_executor_backends",
            "fanout_backends",
            "selected_backend_from_execution_plan_route",
            "fanout_executor_backends_from_route",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_backend_id",
            "runtime_assignment_backend",
            "route_explain_status / route_explain_blocker_codes",
            "runtime_assignment_backend_for_route",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.model_selection_enabled",
            "model_selection_enabled",
            "route_explain_status / route_explain_blocker_codes",
            "route_explain_status",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.candidate_scope",
            "candidate_scope",
            "route_explain_status / route_explain_blocker_codes",
            "route_explain_status",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_model_profile_id",
            "selected_model_profile_id",
            "route_explain_payload",
            "selected_backend_readiness_payload",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_reasoning_effort",
            "selected_reasoning_effort",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.budget_policy",
            "budget_policy",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.max_budget_units",
            "max_budget_units",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
    ];
    rows.extend(
        route["route_field_truth"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|truth| {
                let field = truth["field"].as_str()?;
                Some(serde_json::json!({
                    "config_key": field,
                    "value": serde_json::Value::Null,
                    "configured": true,
                    "validator": "route_field_truth",
                    "runtime_consumer": serde_json::Value::Null,
                    "operator_surface": "vida taskflow validate-routing",
                    "proof_status": truth["truth"],
                    "effect": truth["effect"],
                }))
            }),
    );
    rows
}

pub(crate) fn model_profile_readiness_audit_payload_for_route(
    dispatch_target: &str,
    route: &serde_json::Value,
) -> serde_json::Value {
    let selected_model_profile_id = route["selected_model_profile_id"].clone();
    let selected_backend_readiness = route
        .get("selected_backend_readiness")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let readiness_blocked = selected_backend_readiness["blocked"].as_bool() == Some(true);
    let readiness_status = selected_backend_readiness["status"]
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            if selected_model_profile_id.as_str().is_some() {
                Some("unknown".to_string())
            } else {
                None
            }
        });
    let readiness_ready = selected_backend_readiness["blocked"]
        .as_bool()
        .map(|blocked| !blocked);
    let mut blocker_codes = Vec::new();
    if selected_model_profile_id.as_str().is_none() {
        blocker_codes.push("selected_model_profile_missing".to_string());
    }
    if readiness_blocked {
        blocker_codes.push("selected_model_profile_not_ready".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    let next_actions = if blocker_codes.is_empty() {
        Vec::<String>::new()
    } else {
        vec![
            "inspect selected_backend_readiness, selection_source_paths, and rejected_alternatives before enabling model-profile execution".to_string(),
        ]
    };
    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    serde_json::json!({
        "surface": "vida taskflow model-profile readiness audit",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "dispatch_target": dispatch_target,
        "route_status": route["status"],
        "selected_profile": {
            "profile_id": selected_model_profile_id,
            "model_ref": route["selected_model_ref"],
            "provider": route["selected_model_provider"],
            "reasoning_effort": route["selected_reasoning_effort"],
            "reasoning_control_mode": route["selected_reasoning_control_mode"],
            "selected_backend": route["selected_backend"],
            "selected_carrier_id": route["selected_carrier_id"],
            "readiness_status": readiness_status,
            "readiness_ready": readiness_ready,
            "readiness": selected_backend_readiness,
        },
        "source_paths": route["selection_source_paths"],
        "override_reasons": route["selection_override_reasons"],
        "selection_precedence": route["selection_precedence"],
        "selected_route_profile_mapping": route["selected_route_profile_mapping"],
        "selected_candidate": route["selected_candidate"],
        "candidate_pool": route["candidate_pool"],
        "rejected_alternatives": route["rejected_candidates"],
        "readiness_blockers": route["readiness_blockers"],
        "budget": {
            "policy": route["budget_policy"],
            "verdict": route["budget_verdict"],
            "max_budget_units": route["max_budget_units"],
            "selected_over_budget": route["selected_over_budget"],
            "scope": route["budget_scope"],
            "selection_budget": route["selection_budget"],
            "runtime_budget_ledger": route["runtime_budget_ledger"],
        },
    })
}

fn build_config_actuation_census_payload(
    context: &crate::state_store::RunGraphDispatchContext,
    execution_plan: &serde_json::Value,
) -> serde_json::Value {
    let routes = route_validate_targets(execution_plan)
        .into_iter()
        .map(|target| {
            let route = route_payload_for_dispatch_target(execution_plan, &target);
            let model_profile_readiness_audit =
                model_profile_readiness_audit_payload_for_route(&target, &route);
            let rows = config_actuation_census_rows_for_route(&route);
            serde_json::json!({
                "dispatch_target": target,
                "status": route["status"],
                "selected_backend": route["selected_backend"],
                "selection_source": route["selection_source"],
                "model_profile_readiness_audit": model_profile_readiness_audit,
                "rows": rows,
            })
        })
        .collect::<Vec<_>>();
    let row_count = routes
        .iter()
        .filter_map(|route| route["rows"].as_array().map(Vec::len))
        .sum::<usize>();
    serde_json::json!({
        "surface": "vida taskflow config-actuation census",
        "status": "pass",
        "blocker_codes": [],
        "run_id": context.run_id,
        "task_id": context.task_id,
        "scope": "routing_model_selection_keys",
        "route_count": routes.len(),
        "row_count": row_count,
        "routes": routes,
    })
}

async fn run_taskflow_route_diagnostic(args: &[String]) -> ExitCode {
    let parsed = match parse_taskflow_route_diagnostic_args(args) {
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let context = match latest_or_requested_dispatch_context(&store, parsed.run_id.as_deref()).await
    {
        Ok(Some(context)) => context,
        Ok(None) => {
            let payload = serde_json::json!({
                "surface": match parsed.mode {
                    RouteDiagnosticMode::Explain => "vida taskflow route explain",
                    RouteDiagnosticMode::ModelProfileReadinessAudit => {
                        "vida taskflow route model-profile-readiness"
                    }
                    RouteDiagnosticMode::ValidateRouting => "vida taskflow validate-routing",
                    RouteDiagnosticMode::ConfigActuationCensus => {
                        "vida taskflow config-actuation census"
                    }
                },
                "status": "blocked",
                "blocker_codes": ["run_graph_dispatch_context_missing"],
                "run_id": parsed.run_id,
            });
            let payload =
                normalize_taskflow_route_diagnostic_payload(payload).unwrap_or_else(|_| {
                    serde_json::json!({
                        "surface": "vida taskflow diagnostic",
                        "status": "blocked",
                        "blocker_codes": ["unsupported_blocker_code"],
                        "next_actions": ["inspect diagnostic blockers"]
                    })
                });
            crate::print_json_pretty(&payload);
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("Failed to read run-graph dispatch context: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(execution_plan) = execution_plan_from_dispatch_context(&context) else {
        eprintln!(
            "Run `{}` has no object role_selection.execution_plan in dispatch context.",
            context.run_id
        );
        return ExitCode::from(1);
    };

    let mode = parsed.mode;
    let payload = match mode {
        RouteDiagnosticMode::Explain => {
            let dispatch_target = match parsed.dispatch_target {
                Some(target) => target,
                None => match parsed.runtime_role.as_deref() {
                    Some(role) => match crate::taskflow_routing::dispatch_target_for_runtime_role(
                        execution_plan,
                        role,
                    ) {
                        Some(target) => target,
                        None => {
                            eprintln!(
                                "Unable to resolve dispatch target for runtime role `{role}`."
                            );
                            return ExitCode::from(1);
                        }
                    },
                    None => "implementation".to_string(),
                },
            };
            let explain = route_payload_for_dispatch_target(execution_plan, &dispatch_target);
            serde_json::json!({
                "surface": "vida taskflow route explain",
                "status": explain["status"],
                "blocker_codes": explain["blocker_codes"],
                "run_id": context.run_id,
                "task_id": context.task_id,
                "dispatch_target": dispatch_target,
                "route": explain,
            })
        }
        RouteDiagnosticMode::ModelProfileReadinessAudit => {
            let dispatch_target = match parsed.dispatch_target {
                Some(target) => target,
                None => match parsed.runtime_role.as_deref() {
                    Some(role) => match crate::taskflow_routing::dispatch_target_for_runtime_role(
                        execution_plan,
                        role,
                    ) {
                        Some(target) => target,
                        None => {
                            eprintln!(
                                "Unable to resolve dispatch target for runtime role `{role}`."
                            );
                            return ExitCode::from(1);
                        }
                    },
                    None => "implementation".to_string(),
                },
            };
            let route = route_payload_for_dispatch_target(execution_plan, &dispatch_target);
            let mut audit =
                model_profile_readiness_audit_payload_for_route(&dispatch_target, &route);
            if let Some(object) = audit.as_object_mut() {
                object.insert(
                    "run_id".to_string(),
                    serde_json::Value::String(context.run_id),
                );
                object.insert(
                    "task_id".to_string(),
                    serde_json::Value::String(context.task_id),
                );
            }
            audit
        }
        RouteDiagnosticMode::ValidateRouting => {
            let routes = route_validate_targets(execution_plan)
                .into_iter()
                .map(|target| route_payload_for_dispatch_target(execution_plan, &target))
                .collect::<Vec<_>>();
            let blocker_codes = routes
                .iter()
                .flat_map(|route| {
                    route["blocker_codes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let next_actions = routes
                .iter()
                .flat_map(|route| {
                    route["next_actions"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let status = if blocker_codes.is_empty() {
                "pass"
            } else {
                "blocked"
            };
            serde_json::json!({
                "surface": "vida taskflow validate-routing",
                "status": status,
                "blocker_codes": blocker_codes,
                "run_id": context.run_id,
                "task_id": context.task_id,
                "route_count": routes.len(),
                "routes": routes,
                "next_actions": next_actions,
            })
        }
        RouteDiagnosticMode::ConfigActuationCensus => {
            build_config_actuation_census_payload(&context, execution_plan)
        }
    };
    let payload = if matches!(mode, RouteDiagnosticMode::ModelProfileReadinessAudit) {
        payload
    } else {
        normalize_taskflow_route_diagnostic_payload(payload).unwrap_or_else(|_| {
            serde_json::json!({
                "surface": "vida taskflow diagnostic",
                "status": "blocked",
                "blocker_codes": ["unsupported_blocker_code"],
                "next_actions": ["inspect diagnostic blockers"]
            })
        })
    };

    if parsed.as_json {
        crate::print_json_pretty(&payload);
    } else {
        let surface = payload["surface"].as_str().unwrap_or("vida taskflow route");
        print_surface_header(RenderMode::Plain, surface);
        print_surface_line(
            RenderMode::Plain,
            "status",
            payload["status"].as_str().unwrap_or("unknown"),
        );
        if let Some(run_id) = payload["run_id"].as_str() {
            print_surface_line(RenderMode::Plain, "run_id", run_id);
        }
        if let Some(target) = payload["dispatch_target"].as_str() {
            print_surface_line(RenderMode::Plain, "dispatch_target", target);
        }
        if let Some(backend) = payload
            .get("route")
            .and_then(|route| route.get("selected_backend"))
            .and_then(serde_json::Value::as_str)
        {
            print_surface_line(RenderMode::Plain, "selected_backend", backend);
        }
        if let Some(source) = payload
            .get("route")
            .and_then(|route| route.get("selection_source"))
            .and_then(serde_json::Value::as_str)
        {
            print_surface_line(RenderMode::Plain, "selection_source", source);
        }
        if let Some(count) = payload["route_count"].as_u64() {
            print_surface_line(RenderMode::Plain, "route_count", &count.to_string());
        }
        if let Some(blockers) = payload["blocker_codes"]
            .as_array()
            .filter(|items| !items.is_empty())
        {
            let joined = blockers
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            print_surface_line(RenderMode::Plain, "blocker_codes", &joined);
        }
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_graph_summary_waves, build_taskflow_scheduler_dispatch_plan,
        taskflow_task_subcommand_supported, GraphSummaryWaveBucket,
    };
    use crate::state_store::{
        BlockedTaskRecord, TaskDependencyRecord, TaskDependencyStatus, TaskRecord,
        TaskSchedulingCandidate, TaskSchedulingProjection,
    };

    fn task(
        id: &str,
        issue_type: &str,
        status: &str,
        priority: u32,
        labels: &[&str],
        dependencies: Vec<TaskDependencyRecord>,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: format!("task {id}"),
            description: String::new(),
            issue_type: issue_type.to_string(),
            status: status.to_string(),
            priority,
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            updated_at: "0".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies,
        }
    }

    fn parent_dependency(issue_id: &str, depends_on_id: &str) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    fn scheduling_candidate(
        task: TaskRecord,
        ready_now: bool,
        ready_parallel_safe: bool,
        active_critical_path: bool,
        blocked_by: Vec<TaskDependencyStatus>,
        parallel_blockers: Vec<&str>,
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task,
            ready_now,
            ready_parallel_safe,
            blocked_by,
            active_critical_path,
            parallel_blockers: parallel_blockers.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn graph_summary_waves_follow_parent_chain_wave_labels() {
        let wave_epic = task("wave-0-epic", "epic", "open", 1, &["wave-0"], Vec::new());
        let child_ready = task(
            "r1-ready",
            "task",
            "open",
            1,
            &[],
            vec![parent_dependency("r1-ready", "wave-0-epic")],
        );
        let child_blocked = task(
            "r1-blocked",
            "task",
            "in_progress",
            2,
            &[],
            vec![parent_dependency("r1-blocked", "wave-0-epic")],
        );
        let unassigned = task("r1-unassigned", "task", "open", 3, &[], Vec::new());

        let all_tasks = vec![
            wave_epic.clone(),
            child_ready.clone(),
            child_blocked.clone(),
            unassigned.clone(),
        ];
        let ready_tasks = vec![child_ready.clone(), unassigned.clone()];
        let blocked_tasks = vec![BlockedTaskRecord {
            task: child_blocked.clone(),
            blockers: Vec::new(),
        }];

        let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);
        assert_eq!(
            waves,
            vec![
                GraphSummaryWaveBucket {
                    wave_id: "wave-0".to_string(),
                    task_count: 2,
                    ready_count: 1,
                    blocked_count: 1,
                    primary_ready_task: Some(super::graph_summary_task_ref(&child_ready)),
                    primary_blocked_task: Some(super::graph_summary_task_ref(&child_blocked)),
                },
                GraphSummaryWaveBucket {
                    wave_id: "unassigned".to_string(),
                    task_count: 1,
                    ready_count: 1,
                    blocked_count: 0,
                    primary_ready_task: Some(super::graph_summary_task_ref(&unassigned)),
                    primary_blocked_task: None,
                },
            ]
        );
    }

    #[test]
    fn graph_summary_waves_prefer_order_bucket_over_labels() {
        let mut explicit_bucket = task("bucket-task", "task", "open", 1, &["wave-99"], Vec::new());
        explicit_bucket.execution_semantics.order_bucket = Some("wave-2".to_string());

        let waves = build_graph_summary_waves(
            std::slice::from_ref(&explicit_bucket),
            std::slice::from_ref(&explicit_bucket),
            &[],
        );
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].wave_id, "wave-2");
        assert_eq!(waves[0].ready_count, 1);
    }

    #[test]
    fn scheduler_dispatch_plan_prefers_critical_path_and_respects_parallel_cap() {
        let mut primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        primary.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        primary.execution_semantics.order_bucket = Some("wave-a".to_string());
        primary.execution_semantics.parallel_group = Some("docs".to_string());
        primary.execution_semantics.conflict_domain = Some("critical".to_string());

        let mut sibling_a = task("parallel-a", "task", "open", 2, &[], Vec::new());
        sibling_a.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling_a.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling_a.execution_semantics.parallel_group = Some("docs".to_string());
        sibling_a.execution_semantics.conflict_domain = Some("parallel-a".to_string());

        let mut sibling_b = task("parallel-b", "task", "open", 3, &[], Vec::new());
        sibling_b.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling_b.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling_b.execution_semantics.parallel_group = Some("docs".to_string());
        sibling_b.execution_semantics.conflict_domain = Some("parallel-b".to_string());

        let mut unsafe_sibling = task("sequential-only", "task", "open", 4, &[], Vec::new());
        unsafe_sibling.execution_semantics.execution_mode = Some("sequential".to_string());

        let blocked_dependency = TaskDependencyStatus {
            issue_id: "blocked".to_string(),
            depends_on_id: "dep-1".to_string(),
            edge_type: "depends-on".to_string(),
            dependency_status: "open".to_string(),
            dependency_issue_type: Some("task".to_string()),
        };
        let blocked = task("blocked", "task", "open", 5, &[], Vec::new());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![
                scheduling_candidate(primary.clone(), true, false, true, Vec::new(), vec![]),
                scheduling_candidate(sibling_a.clone(), true, true, false, Vec::new(), vec![]),
                scheduling_candidate(sibling_b.clone(), true, true, false, Vec::new(), vec![]),
                scheduling_candidate(
                    unsafe_sibling.clone(),
                    true,
                    false,
                    false,
                    Vec::new(),
                    vec!["current_execution_mode_not_parallel_safe"],
                ),
            ],
            blocked: vec![scheduling_candidate(
                blocked.clone(),
                false,
                false,
                false,
                vec![blocked_dependency.clone()],
                vec!["graph_blocked"],
            )],
            parallel_candidates_after_current: vec![sibling_a.clone(), sibling_b.clone()],
        };

        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");
        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 2, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(plan.status, "pass");
        assert_eq!(plan.max_parallel_agents, 2);
        assert!(!plan.execute_requested);
        assert!(!plan.execute_supported);
        assert!(!plan.execution_attempted);
        assert_eq!(plan.execution_status, "preview");
        assert_eq!(plan.selection_source, "critical_path_ready_head");
        assert_eq!(
            plan.selected_primary_task
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("critical-ready")
        );
        assert_eq!(plan.selected_task_ids, vec!["critical-ready", "parallel-a"]);
        assert_eq!(plan.selected_parallel_tasks.len(), 1);
        assert_eq!(plan.selected_parallel_tasks[0].id, "parallel-a");
        assert_eq!(plan.reservations.len(), 2);
        assert_eq!(plan.reservations[0].task_id, "critical-ready");
        assert_eq!(plan.reservations[0].launch_role, "primary");
        assert_eq!(plan.reservations[0].launch_index, 0);
        assert_eq!(
            plan.reservations[0].command,
            "vida agent-init --role worker critical-ready --state-dir /tmp/vida-scheduler-state --json"
        );
        assert_eq!(plan.reservations[0].state_dir, "/tmp/vida-scheduler-state");
        assert_eq!(
            plan.reservations[0].conflict_domain.as_deref(),
            Some("critical")
        );
        assert_eq!(plan.reservations[0].execute_status, "preview_not_executed");
        assert_eq!(plan.reservations[0].receipt_id, None);
        assert_eq!(plan.reservations[0].receipt_path, None);
        assert_eq!(
            plan.reservations[0].preview_only_reason.as_deref(),
            Some("scheduler_dispatch_is_preview_only")
        );
        assert_eq!(
            plan.reservations[0].reservation_id,
            "scheduler-preview-primary-0-critical-ready"
        );
        assert_eq!(
            plan.reservations[0].reservation_status,
            "preview_unpersisted"
        );
        assert!(!plan.reservations[0].reservation_persisted);
        assert!(!plan.reservations[0].execution_attempted);
        assert_eq!(plan.reservations[1].task_id, "parallel-a");
        assert_eq!(plan.reservations[1].launch_role, "parallel");
        assert_eq!(plan.reservations[1].launch_index, 1);
        assert_eq!(
            plan.reservations[1].conflict_domain.as_deref(),
            Some("parallel-a")
        );
        assert_eq!(plan.dispatch_receipt.receipt_id, None);
        assert_eq!(plan.dispatch_receipt.receipt_path, None);
        assert_eq!(
            plan.dispatch_receipt.receipt_status,
            "preview_not_persisted"
        );
        assert_eq!(
            plan.dispatch_receipt.preview_only_reason.as_deref(),
            Some("scheduler_dispatch_is_preview_only")
        );
        assert_eq!(plan.dispatch_receipt.execute_status, "preview_not_executed");
        assert!(!plan.dispatch_receipt.receipt_persisted);
        assert_eq!(plan.dispatch_receipt.dispatch_status, "pass");
        assert_eq!(
            plan.dispatch_receipt.selected_task_ids,
            vec!["critical-ready", "parallel-a"]
        );
        assert_eq!(
            plan.dispatch_receipt.reservation_ids,
            vec![
                "scheduler-preview-primary-0-critical-ready",
                "scheduler-preview-parallel-1-parallel-a"
            ]
        );
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "parallel-b"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "sequential-only"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "current_execution_mode_not_parallel_safe")
        }));
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "blocked"
                && candidate.task_id == "blocked"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "blocked_by:depends-on:dep-1:open")
        }));
    }

    #[test]
    fn scheduler_dispatch_execute_projects_lawful_selection_with_real_reservation_attempt() {
        let primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![scheduling_candidate(
                primary,
                true,
                false,
                true,
                Vec::new(),
                vec![],
            )],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");
        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 1, None, None, None, None, state_dir, false, true,
        );

        assert_eq!(plan.status, "pass");
        assert!(plan.blocker_codes.is_empty());
        assert_eq!(plan.max_parallel_agents, 1);
        assert!(plan.execute_requested);
        assert!(plan.execute_supported);
        assert!(!plan.execution_attempted);
        assert_eq!(plan.execution_status, "execute_projection_not_executed");
        assert!(!plan.dry_run);
        assert_eq!(plan.selected_task_ids, vec!["critical-ready"]);
        assert_eq!(plan.reservations.len(), 1);
        assert_eq!(
            plan.reservations[0].reservation_status,
            "execute_projection_unpersisted"
        );
        assert!(plan.reservations[0].execute_supported);
        assert!(!plan.reservations[0].reservation_persisted);
        assert_eq!(
            plan.dispatch_receipt.receipt_status,
            "execute_projection_not_persisted"
        );
        assert_eq!(plan.dispatch_receipt.dispatch_status, "pass");
        assert!(plan.dispatch_receipt.execute_requested);
        assert!(plan.dispatch_receipt.execute_supported);
        assert!(!plan.dispatch_receipt.execution_attempted);
        assert_eq!(
            plan.dispatch_receipt.execute_status,
            "execute_projection_not_executed"
        );
        assert!(plan.dispatch_receipt.preview_only_reason.is_none());
        assert!(!plan.dispatch_receipt.receipt_persisted);
        assert_eq!(plan.dispatch_receipt.receipt_id, None);
        assert_eq!(plan.dispatch_receipt.receipt_path, None);
        assert!(plan.dispatch_receipt.execution_blocker_codes.is_empty());
        assert!(!plan.packet_backed_execution_supported);
        assert_eq!(
            plan.packet_backed_execution_status,
            "blocked_lineage_preconditions_not_verified"
        );
        assert!(!plan.dispatch_receipt.packet_backed_execution_supported);
        assert_eq!(
            plan.dispatch_receipt.packet_backed_execution_status,
            "blocked_lineage_preconditions_not_verified"
        );
        assert!(plan.reservations[0]
            .preview_only_reason
            .as_deref()
            .is_none());
        assert_eq!(
            plan.reservations[0].execute_status,
            "execute_projection_not_executed"
        );
        assert!(!plan
            .next_actions
            .iter()
            .any(|action| { action.contains("execution is not attempted") }));
    }

    #[test]
    fn scheduler_dispatch_plan_respects_requested_parallel_limit_under_configured_cap() {
        let mut primary = task("primary", "task", "open", 1, &[], Vec::new());
        primary.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        primary.execution_semantics.order_bucket = Some("wave-a".to_string());
        primary.execution_semantics.parallel_group = Some("docs".to_string());
        primary.execution_semantics.conflict_domain = Some("primary".to_string());

        let mut parallel = task("parallel", "task", "open", 2, &[], Vec::new());
        parallel.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        parallel.execution_semantics.order_bucket = Some("wave-a".to_string());
        parallel.execution_semantics.parallel_group = Some("docs".to_string());
        parallel.execution_semantics.conflict_domain = Some("parallel".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("primary".to_string()),
            ready: vec![
                scheduling_candidate(primary, true, true, true, Vec::new(), vec![]),
                scheduling_candidate(parallel, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");
        let plan = build_taskflow_scheduler_dispatch_plan(
            projection,
            4,
            Some(1),
            None,
            None,
            None,
            state_dir,
            true,
            false,
        );

        assert_eq!(plan.configured_max_parallel_agents, 4);
        assert_eq!(plan.requested_parallel_limit, Some(1));
        assert_eq!(plan.max_parallel_agents, 1);
        assert_eq!(plan.selected_task_ids, vec!["primary"]);
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "parallel"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_serializes_reservation_and_receipt_projection_fields() {
        let primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        let parallel = task("parallel-ready", "task", "open", 2, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![
                scheduling_candidate(primary, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(parallel, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");
        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 2, None, None, None, None, state_dir, true, false,
        );
        let payload = serde_json::to_value(&plan).expect("scheduler plan should serialize");

        assert_eq!(
            payload["reservations"][0]["reservation_id"],
            "scheduler-preview-primary-0-critical-ready"
        );
        assert_eq!(payload["reservations"][0]["task_id"], "critical-ready");
        assert_eq!(
            payload["reservations"][0]["command"],
            "vida agent-init --role worker critical-ready --state-dir /tmp/vida-scheduler-state --json"
        );
        assert_eq!(
            payload["reservations"][0]["state_dir"],
            "/tmp/vida-scheduler-state"
        );
        assert_eq!(
            payload["reservations"][0]["execute_status"],
            "preview_not_executed"
        );
        assert_eq!(
            payload["reservations"][0]["activation_status"],
            "preview_not_executed"
        );
        assert_eq!(
            payload["reservations"][0]["worker_execution_evidence_status"],
            "not_received"
        );
        assert_eq!(
            payload["reservations"][0]["worker_completion_claimed"],
            false
        );
        assert_eq!(
            payload["reservations"][0]["preview_only_reason"],
            "scheduler_dispatch_is_preview_only"
        );
        assert_eq!(
            payload["reservations"][0]["receipt_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            payload["reservations"][0]["receipt_path"],
            serde_json::Value::Null
        );
        assert_eq!(
            payload["reservations"][1]["reservation_id"],
            "scheduler-preview-parallel-1-parallel-ready"
        );
        assert_eq!(payload["reservations"][0]["reservation_persisted"], false);
        assert_eq!(
            payload["dispatch_receipt"]["receipt_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            payload["dispatch_receipt"]["receipt_path"],
            serde_json::Value::Null
        );
        assert_eq!(
            payload["dispatch_receipt"]["receipt_status"],
            "preview_not_persisted"
        );
        assert_eq!(payload["dispatch_receipt"]["receipt_persisted"], false);
        assert_eq!(
            payload["dispatch_receipt"]["execute_status"],
            "preview_not_executed"
        );
        assert_eq!(
            payload["dispatch_receipt"]["activation_status"],
            "preview_not_executed"
        );
        assert_eq!(
            payload["dispatch_receipt"]["worker_execution_evidence_status"],
            "not_received"
        );
        assert_eq!(
            payload["dispatch_receipt"]["worker_completion_claimed"],
            false
        );
        assert_eq!(
            payload["dispatch_receipt"]["preview_only_reason"],
            "scheduler_dispatch_is_preview_only"
        );
        assert_eq!(
            payload["dispatch_receipt"]["reservation_ids"],
            serde_json::json!([
                "scheduler-preview-primary-0-critical-ready",
                "scheduler-preview-parallel-1-parallel-ready"
            ])
        );
        assert_eq!(payload["dispatch_receipt"]["execution_attempted"], false);
        assert_eq!(payload["activation_attempted"], false);
        assert_eq!(payload["worker_execution_evidence_status"], "not_received");
        assert_eq!(payload["worker_completion_claimed"], false);
    }

    #[test]
    fn scheduler_dispatch_plan_documents_cap_one_and_two_selection() {
        let mut primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        primary.execution_semantics.conflict_domain = Some("critical".to_string());
        let mut parallel_a = task("parallel-a", "task", "open", 2, &[], Vec::new());
        parallel_a.execution_semantics.conflict_domain = Some("parallel-a".to_string());
        let mut parallel_b = task("parallel-b", "task", "open", 3, &[], Vec::new());
        parallel_b.execution_semantics.conflict_domain = Some("parallel-b".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![
                scheduling_candidate(primary, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(parallel_a, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(parallel_b, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let cap_one = build_taskflow_scheduler_dispatch_plan(
            projection.clone(),
            1,
            None,
            None,
            None,
            None,
            state_dir,
            true,
            false,
        );
        assert_eq!(cap_one.selected_task_ids, vec!["critical-ready"]);
        assert!(cap_one.selected_parallel_tasks.is_empty());
        assert!(cap_one.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "parallel-a"
                && candidate.ready_now
                && candidate.conflict_domain.as_deref() == Some("parallel-a")
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));

        let cap_two = build_taskflow_scheduler_dispatch_plan(
            projection, 2, None, None, None, None, state_dir, true, false,
        );
        assert_eq!(
            cap_two.selected_task_ids,
            vec!["critical-ready", "parallel-a"]
        );
        assert_eq!(cap_two.selected_parallel_tasks.len(), 1);
        assert!(cap_two.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "parallel-b"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_documents_conflict_domain_exclusion() {
        let mut current = task("current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.conflict_domain = Some("shared-docs".to_string());
        let mut conflicting = task("conflicting", "task", "open", 2, &[], Vec::new());
        conflicting.execution_semantics.conflict_domain = Some("shared-docs".to_string());
        let mut safe = task("safe-parallel", "task", "open", 3, &[], Vec::new());
        safe.execution_semantics.conflict_domain = Some("safe-docs".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("current".to_string()),
            ready: vec![
                scheduling_candidate(current, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(
                    conflicting,
                    true,
                    false,
                    false,
                    Vec::new(),
                    vec!["conflict_domain_already_selected:shared-docs"],
                ),
                scheduling_candidate(safe, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 2, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(plan.selected_task_ids, vec!["current", "safe-parallel"]);
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "conflicting"
                && candidate.ready_now
                && candidate.conflict_domain.as_deref() == Some("shared-docs")
                && candidate
                    .parallel_blockers
                    .iter()
                    .any(|reason| reason == "conflict_domain_already_selected:shared-docs")
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "conflict_domain_already_selected:shared-docs")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_enforces_conflict_domain_even_when_projection_is_permissive() {
        let mut current = task("current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.conflict_domain = Some("shared-docs".to_string());
        let mut conflicting = task("conflicting", "task", "open", 2, &[], Vec::new());
        conflicting.execution_semantics.conflict_domain = Some("shared-docs".to_string());
        let mut safe = task("safe-parallel", "task", "open", 3, &[], Vec::new());
        safe.execution_semantics.conflict_domain = Some("safe-docs".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("current".to_string()),
            ready: vec![
                scheduling_candidate(current, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(conflicting, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(safe, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 3, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(plan.selected_task_ids, vec!["current", "safe-parallel"]);
        assert_eq!(plan.max_parallel_agents, 3);
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "conflicting"
                && candidate.ready_now
                && candidate.conflict_domain.as_deref() == Some("shared-docs")
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "conflict_domain_already_selected:shared-docs")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_excludes_conflicts_between_parallel_candidates() {
        let mut current = task("current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.conflict_domain = Some("current-docs".to_string());
        let mut first = task("parallel-a", "task", "open", 2, &[], Vec::new());
        first.execution_semantics.conflict_domain = Some("shared-docs".to_string());
        let mut second = task("parallel-b", "task", "open", 3, &[], Vec::new());
        second.execution_semantics.conflict_domain = Some("shared-docs".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("current".to_string()),
            ready: vec![
                scheduling_candidate(current, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(first, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(second, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 3, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(plan.selected_task_ids, vec!["current", "parallel-a"]);
        assert_eq!(plan.selected_parallel_tasks.len(), 1);
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "parallel-b"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "conflict_domain_already_selected:shared-docs")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_prefers_critical_path_over_ready_order() {
        let mut fallback = task("ready-first", "task", "open", 1, &[], Vec::new());
        fallback.execution_semantics.order_bucket = Some("wave-2".to_string());
        fallback.execution_semantics.conflict_domain = Some("fallback".to_string());
        let mut critical = task("critical-second", "task", "open", 2, &[], Vec::new());
        critical.execution_semantics.order_bucket = Some("wave-1".to_string());
        critical.execution_semantics.conflict_domain = Some("critical".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-second".to_string()),
            ready: vec![
                scheduling_candidate(fallback, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(critical, true, false, true, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 1, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(plan.selection_source, "critical_path_ready_head");
        assert_eq!(plan.selected_task_ids, vec!["critical-second"]);
        assert_eq!(
            plan.selected_primary_task
                .as_ref()
                .and_then(|task| task.conflict_domain.as_deref()),
            Some("critical")
        );
    }

    #[test]
    fn scheduler_dispatch_plan_prefers_explicit_task_graph_binding_over_critical_path() {
        let mut critical = task("critical-ready", "task", "open", 1, &[], Vec::new());
        critical.execution_semantics.conflict_domain = Some("critical".to_string());
        let mut bound = task("explicit-bound", "task", "open", 2, &[], Vec::new());
        bound.execution_semantics.conflict_domain = Some("bound".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("explicit-bound".to_string()),
            ready: vec![
                scheduling_candidate(critical, true, true, true, Vec::new(), vec![]),
                scheduling_candidate(bound, true, false, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection,
            1,
            None,
            None,
            None,
            Some("explicit-bound"),
            state_dir,
            true,
            false,
        );

        assert_eq!(
            plan.selection_source,
            "explicit_run_graph_continuation_binding"
        );
        assert_eq!(plan.selected_task_ids, vec!["explicit-bound"]);
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "critical-ready"
                && candidate.active_critical_path
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
    }

    #[test]
    fn scheduler_agent_init_activation_evidence_status_stays_blocked_without_execution_evidence() {
        assert_eq!(
            super::scheduler_agent_init_activation_result_status(Some("activation_view")),
            (
                "activation_view_only",
                "agent_init_activation_view_only",
                "scheduler_agent_init_activation_view_only"
            )
        );
        assert_eq!(
            super::scheduler_agent_init_activation_result_status(Some("dispatch_packet")),
            (
                "activation_evidence_returned",
                "agent_init_activation_evidence_only",
                "scheduler_agent_init_activation_evidence_only"
            )
        );
        assert_ne!(
            super::scheduler_agent_init_activation_result_status(Some("dispatch_packet")).0,
            "agent_init_returned"
        );
        assert_ne!(
            super::scheduler_agent_init_activation_result_status(Some("activation_view")).0,
            "completed"
        );
    }

    #[test]
    fn scheduler_dispatch_plan_preserves_order_bucket_projection_boundary() {
        let mut current = task("wave-1-current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.order_bucket = Some("wave-1".to_string());
        current.execution_semantics.conflict_domain = Some("current".to_string());
        let mut same_bucket = task("wave-1-parallel", "task", "open", 2, &[], Vec::new());
        same_bucket.execution_semantics.order_bucket = Some("wave-1".to_string());
        same_bucket.execution_semantics.conflict_domain = Some("same-bucket".to_string());
        let mut later_bucket = task("wave-2-held", "task", "open", 3, &[], Vec::new());
        later_bucket.execution_semantics.order_bucket = Some("wave-2".to_string());
        later_bucket.execution_semantics.conflict_domain = Some("later-bucket".to_string());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("wave-1-current".to_string()),
            ready: vec![
                scheduling_candidate(current, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(same_bucket, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(
                    later_bucket,
                    true,
                    false,
                    false,
                    Vec::new(),
                    vec!["order_bucket_not_current:wave-2"],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let plan = build_taskflow_scheduler_dispatch_plan(
            projection, 3, None, None, None, None, state_dir, true, false,
        );

        assert_eq!(
            plan.selected_task_ids,
            vec!["wave-1-current", "wave-1-parallel"]
        );
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "wave-2-held"
                && candidate.ready_now
                && candidate.conflict_domain.as_deref() == Some("later-bucket")
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "order_bucket_not_current:wave-2")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_keeps_starvation_candidates_visible_for_rotation() {
        let primary = task("primary", "task", "open", 1, &[], Vec::new());
        let old_waiting = task("old-waiting", "task", "open", 2, &[], Vec::new());
        let still_waiting = task("still-waiting", "task", "open", 3, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("primary".to_string()),
            ready: vec![
                scheduling_candidate(primary, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(old_waiting, true, true, false, Vec::new(), vec![]),
                scheduling_candidate(still_waiting, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");

        let requested = build_taskflow_scheduler_dispatch_plan(
            projection,
            1,
            None,
            None,
            Some("old-waiting"),
            None,
            state_dir,
            true,
            false,
        );

        assert_eq!(requested.selection_source, "requested_current_task");
        assert_eq!(requested.selected_task_ids, vec!["old-waiting"]);
        assert!(requested.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "primary"
                && candidate.active_critical_path
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
        assert!(requested.rejected_candidates.iter().any(|candidate| {
            candidate.task_id == "still-waiting"
                && candidate.ready_now
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
    }

    #[test]
    fn graph_explain_payload_reports_ready_blocked_and_parallel_truth() {
        let mut current = task("current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        current.execution_semantics.order_bucket = Some("wave-a".to_string());
        current.execution_semantics.parallel_group = Some("docs".to_string());
        current.execution_semantics.conflict_domain = Some("current".to_string());

        let mut sibling = task("sibling", "task", "open", 2, &[], Vec::new());
        sibling.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling.execution_semantics.parallel_group = Some("docs".to_string());
        sibling.execution_semantics.conflict_domain = Some("sibling".to_string());

        let blocker = TaskDependencyStatus {
            issue_id: "blocked".to_string(),
            depends_on_id: "current".to_string(),
            edge_type: "blocks".to_string(),
            dependency_status: "open".to_string(),
            dependency_issue_type: Some("task".to_string()),
        };
        let blocked = task("blocked", "task", "open", 3, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("current".to_string()),
            ready: vec![
                scheduling_candidate(
                    current,
                    true,
                    false,
                    true,
                    Vec::new(),
                    vec!["parallel_conflict"],
                ),
                scheduling_candidate(sibling.clone(), true, true, false, Vec::new(), vec![]),
            ],
            blocked: vec![scheduling_candidate(
                blocked,
                false,
                false,
                false,
                vec![blocker],
                vec!["graph_blocked"],
            )],
            parallel_candidates_after_current: vec![sibling],
        };

        let ready_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("sibling"));
        assert_eq!(ready_payload["surface"], "vida taskflow graph explain");
        assert_eq!(ready_payload["status"], "pass");
        assert_eq!(ready_payload["ready_now"], true);
        assert_eq!(ready_payload["ready_parallel_safe"], true);
        assert_eq!(ready_payload["selected_as_parallel_after_current"], true);
        assert_eq!(
            ready_payload["next_lawful_action"]["surface"],
            "vida taskflow scheduler dispatch"
        );

        let blocked_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("blocked"));
        assert_eq!(blocked_payload["status"], "blocked");
        assert_eq!(blocked_payload["ready_now"], false);
        assert!(blocked_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.contains(&serde_json::json!("graph_blocked"))));
        assert_eq!(blocked_payload["blocked_by"][0]["depends_on_id"], "current");
        assert_eq!(
            blocked_payload["next_lawful_action"]["surface"],
            "vida task deps"
        );

        let missing_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("missing"));
        assert_eq!(missing_payload["status"], "blocked");
        assert_eq!(missing_payload["ready_now"], false);
        assert!(missing_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| {
                codes.contains(&serde_json::json!("task_not_in_graph_projection"))
            }));

        let parallel_blocked_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("current"));
        assert_eq!(parallel_blocked_payload["status"], "blocked");
        assert_eq!(parallel_blocked_payload["ready_now"], true);
        assert!(parallel_blocked_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
    }

    #[test]
    fn taskflow_task_subcommand_supports_replace_jsonl() {
        assert!(taskflow_task_subcommand_supported("replace-jsonl"));
    }

    #[test]
    fn taskflow_task_subcommand_supports_task_graph_parity_commands() {
        for subcommand in [
            "children",
            "reparent-children",
            "move-children",
            "tree",
            "subtree",
            "validate-graph",
            "critical-path",
        ] {
            assert!(
                taskflow_task_subcommand_supported(subcommand),
                "`vida taskflow task {subcommand}` should remain in parity with `vida task {subcommand}`"
            );
        }
    }

    #[test]
    fn route_diagnostic_parser_accepts_route_explain_json() {
        let args = vec![
            "route".to_string(),
            "explain".to_string(),
            "--dispatch-target".to_string(),
            "implementation".to_string(),
            "--json".to_string(),
        ];
        let parsed = super::parse_taskflow_route_diagnostic_args(&args).unwrap();
        assert_eq!(parsed.mode, super::RouteDiagnosticMode::Explain);
        assert_eq!(parsed.dispatch_target.as_deref(), Some("implementation"));
        assert!(parsed.as_json);
    }

    #[test]
    fn route_diagnostic_parser_accepts_config_actuation_census() {
        let args = vec![
            "config-actuation".to_string(),
            "census".to_string(),
            "--json".to_string(),
        ];
        let parsed = super::parse_taskflow_route_diagnostic_args(&args).unwrap();
        assert_eq!(
            parsed.mode,
            super::RouteDiagnosticMode::ConfigActuationCensus
        );
        assert!(parsed.as_json);
    }

    #[test]
    fn route_diagnostic_parser_accepts_model_profile_readiness_audit() {
        let args = vec![
            "route".to_string(),
            "model-profile-readiness".to_string(),
            "--runtime-role".to_string(),
            "worker".to_string(),
            "--json".to_string(),
        ];
        let parsed = super::parse_taskflow_route_diagnostic_args(&args).unwrap();
        assert_eq!(
            parsed.mode,
            super::RouteDiagnosticMode::ModelProfileReadinessAudit
        );
        assert_eq!(parsed.runtime_role.as_deref(), Some("worker"));
        assert!(parsed.as_json);
    }

    #[test]
    fn route_diagnostic_help_discovers_model_profile_readiness_audit() {
        let args = vec![
            "route".to_string(),
            "model-profile-readiness".to_string(),
            "--help".to_string(),
        ];
        let usage = super::parse_taskflow_route_diagnostic_args(&args)
            .expect_err("route help should return usage text");

        assert!(usage.contains("vida taskflow route model-profile-readiness"));
        assert!(usage.contains("--dispatch-target <target>|--runtime-role <role>"));
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_derives_blocked_truth() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "status": "pass",
            "blocker_codes": [" migration_required "],
            "next_actions": [" Run migration preflight "],
            "artifact_refs": {
                "source": "unit-test"
            }
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["migration_required"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!(["run migration preflight"])
        );
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
        assert_eq!(normalized["artifact_refs"]["source"], "unit-test");
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_derives_pass_truth() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "status": "blocked",
            "blocker_codes": [],
            "next_actions": [" stale action "]
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "pass");
        assert_eq!(normalized["blocker_codes"], serde_json::json!([]));
        assert_eq!(normalized["next_actions"], serde_json::json!([]));
        assert_eq!(normalized["shared_fields"]["status"], "pass");
        assert_eq!(normalized["operator_contracts"]["status"], "pass");
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_falls_back_for_unknown_blockers() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "blocker_codes": ["not_registry_backed"],
            "next_actions": []
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["unsupported_blocker_code"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!(["inspect diagnostic blockers"])
        );
    }

    #[test]
    fn route_diagnostic_payload_normalizer_applies_pass_contract_parity() {
        let payload = serde_json::json!({
            "surface": "vida taskflow route explain",
            "status": "blocked",
            "blocker_codes": [],
            "next_actions": [" stale action "],
            "route": {
                "status": "pass",
                "blocker_codes": []
            }
        });

        let normalized = super::normalize_taskflow_route_diagnostic_payload(payload)
            .expect("route diagnostic payload should normalize");

        assert_eq!(normalized["status"], "pass");
        assert_eq!(normalized["blocker_codes"], serde_json::json!([]));
        assert_eq!(normalized["next_actions"], serde_json::json!([]));
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
    }

    #[test]
    fn config_diagnostic_payload_normalizer_applies_blocked_contract_parity() {
        let payload = serde_json::json!({
            "surface": "vida taskflow config-actuation census",
            "status": "pass",
            "blocker_codes": ["route_missing"],
            "next_actions": [],
            "routes": []
        });

        let normalized = super::normalize_taskflow_route_diagnostic_payload(payload)
            .expect("config diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["unsupported_blocker_code"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!([
                "inspect route diagnostic blockers with `vida taskflow route explain --json` or `vida taskflow validate-routing --json`"
            ])
        );
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
    }

    #[test]
    fn route_payload_blocks_runtime_selected_carrier_without_matrix_row() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "opencode_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "implementation": false
                    }
                }
            ],
            "development_flow": {
                "implementation": {
                    "executor_backend": "opencode_cli",
                    "fallback_executor_backend": "internal_subagents",
                    "carrier_runtime_assignment": {
                        "selected_backend_id": "junior",
                        "selected_carrier_id": "junior",
                        "selected_agent_id": "junior",
                        "selected_tier": "junior"
                    }
                }
            }
        });

        let payload = super::route_payload_for_dispatch_target(&execution_plan, "implementation");

        assert_eq!(payload["selected_backend"].as_str(), Some("junior"));
        assert_eq!(
            payload["selected_backend_admissible"].as_bool(),
            Some(false)
        );
        assert_eq!(payload["status"].as_str(), Some("blocked"));
        assert!(payload["blocker_codes"].as_array().is_some_and(|codes| {
            codes.contains(&serde_json::json!(
                "selected_backend_not_admissible_for_dispatch_target"
            ))
        }));
    }

    #[test]
    fn config_actuation_census_reports_bounded_route_model_selection_rows() {
        let context = crate::state_store::RunGraphDispatchContext {
            run_id: "run-config-census".to_string(),
            task_id: "task-config-census".to_string(),
            request_text: "inspect config actuation".to_string(),
            role_selection: serde_json::json!({}),
            recorded_at: "0".to_string(),
        };
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementation"],
                    "lane_catalog": {
                        "implementation": {
                            "executor_backend": "opencode_cli",
                            "fallback_executor_backend": "internal_subagents",
                            "fanout_executor_backends": ["middle", "senior"],
                            "carrier_runtime_assignment": {
                                "enabled": true,
                                "selected_backend_id": "junior",
                                "selected_carrier_id": "junior",
                                "selected_model_profile_id": "configured_low_profile",
                                "selected_model_ref": "configured-model-a",
                                "selected_reasoning_effort": "low",
                                "model_selection_enabled": true,
                                "candidate_scope": "unified_carrier_model_profiles",
                                "budget_policy": "tier_budget_guard",
                                "max_budget_units": 4
                            }
                        }
                    }
                }
            }
        });

        let payload = super::build_config_actuation_census_payload(&context, &execution_plan);

        assert_eq!(payload["surface"], "vida taskflow config-actuation census");
        assert_eq!(payload["scope"], "routing_model_selection_keys");
        assert_eq!(payload["route_count"], 1);
        assert_eq!(
            payload["routes"][0]["model_profile_readiness_audit"]["surface"],
            "vida taskflow model-profile readiness audit"
        );
        assert!(payload["row_count"]
            .as_u64()
            .is_some_and(|count| count >= 10));
        let rows = payload["routes"][0]["rows"]
            .as_array()
            .expect("config actuation rows should render");
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.selected_model_profile_id"
                && row["runtime_consumer"] == "selected_backend_readiness_payload"
                && row["proof_status"] == "actuated_or_validated"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "executor_backend"
                && row["runtime_consumer"] == "explicit_executor_backend_from_route"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.candidate_scope"
                && row["validator"] == "route_explain_status / route_explain_blocker_codes"
                && row["proof_status"] == "actuated_or_validated"
        }));
    }

    #[test]
    fn config_actuation_census_marks_blocking_and_non_behavioral_route_fields() {
        let route = serde_json::json!({
            "runtime_assignment_enabled": true,
            "model_selection_enabled": false,
            "candidate_scope": "legacy_backend_pool",
            "route_field_truth": [
                {
                    "field": "external_first_required",
                    "truth": "rejected_no_runtime_consumer",
                    "effect": "validate-routing blocks the route until the field is removed or wired to a concrete consumer"
                }
            ]
        });

        let rows = super::config_actuation_census_rows_for_route(&route);

        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.model_selection_enabled"
                && row["proof_status"] == "validated_blocking"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.candidate_scope"
                && row["proof_status"] == "validated_blocking"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "external_first_required"
                && row["proof_status"] == "rejected_no_runtime_consumer"
                && row["operator_surface"] == "vida taskflow validate-routing"
        }));
    }

    #[test]
    fn model_profile_readiness_audit_payload_reports_ready_selection_truth() {
        let route = serde_json::json!({
            "status": "pass",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_profile_id": "configured_low_profile",
            "selected_model_ref": "configured-model-a",
            "selected_model_provider": "configured-provider",
            "selected_reasoning_effort": "low",
            "selected_reasoning_control_mode": "fixed",
            "selected_backend_readiness": {
                "backend_id": "junior",
                "blocked": false,
                "status": "pass",
                "blocker_code": null,
                "selected_model_profile": "configured_low_profile",
                "next_actions": []
            },
            "selection_source_paths": {
                "selected_model_profile_id": "carrier_runtime.roles[junior].model_profiles.configured_low_profile.profile_id"
            },
            "selection_override_reasons": ["route_profile_mapping"],
            "selection_precedence": ["route_profile_mapping", "role_default"],
            "selected_route_profile_mapping": {
                "runtime_role": "worker",
                "profile_id": "configured_low_profile"
            },
            "selected_candidate": {
                "profile_id": "configured_low_profile",
                "selected": true
            },
            "candidate_pool": [
                {
                    "profile_id": "configured_low_profile",
                    "selected": true
                },
                {
                    "profile_id": "configured_high_profile",
                    "selected": false
                }
            ],
            "rejected_candidates": [
                {
                    "profile_id": "configured_high_profile",
                    "reason": "write_scope_required"
                }
            ],
            "readiness_blockers": [],
            "budget_policy": "tier_budget_guard",
            "budget_verdict": "within_budget",
            "max_budget_units": 4,
            "selected_over_budget": false,
            "budget_scope": "task",
            "selection_budget": {
                "remaining_units": 3
            },
            "runtime_budget_ledger": {
                "spent_units": 1
            }
        });

        let payload =
            super::model_profile_readiness_audit_payload_for_route("implementation", &route);

        assert_eq!(
            payload["surface"],
            "vida taskflow model-profile readiness audit"
        );
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            payload["selected_profile"]["profile_id"],
            "configured_low_profile"
        );
        assert_eq!(payload["selected_profile"]["readiness_status"], "pass");
        assert_eq!(payload["selected_profile"]["readiness_ready"], true);
        assert_eq!(
            payload["source_paths"]["selected_model_profile_id"],
            "carrier_runtime.roles[junior].model_profiles.configured_low_profile.profile_id"
        );
        assert_eq!(
            payload["override_reasons"],
            serde_json::json!(["route_profile_mapping"])
        );
        assert_eq!(
            payload["rejected_alternatives"][0]["profile_id"],
            "configured_high_profile"
        );
        assert_eq!(payload["budget"]["policy"], "tier_budget_guard");
    }

    #[test]
    fn model_profile_readiness_audit_payload_blocks_unready_or_missing_selection() {
        let unready_route = serde_json::json!({
            "status": "blocked",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_profile_id": "configured_low_profile",
            "selected_model_ref": "configured-model-a",
            "selected_model_provider": "configured-provider",
            "selected_reasoning_effort": "low",
            "selected_reasoning_control_mode": "fixed",
            "selected_backend_readiness": {
                "backend_id": "junior",
                "blocked": true,
                "status": "blocked",
                "blocker_code": "external_cli_missing_api_key",
                "selected_model_profile": "configured_low_profile",
                "next_actions": ["configure OPENAI_API_KEY"]
            },
            "selection_source_paths": {},
            "selection_override_reasons": [],
            "selection_precedence": [],
            "selected_route_profile_mapping": null,
            "selected_candidate": null,
            "candidate_pool": [],
            "rejected_candidates": [],
            "readiness_blockers": [
                {
                    "blocker_code": "external_cli_missing_api_key"
                }
            ],
            "budget_policy": null,
            "budget_verdict": null,
            "max_budget_units": null,
            "selected_over_budget": null,
            "budget_scope": null,
            "selection_budget": null,
            "runtime_budget_ledger": null
        });
        let missing_route = serde_json::json!({
            "status": "blocked",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_ref": null,
            "selected_model_provider": null,
            "selected_reasoning_effort": null,
            "selected_reasoning_control_mode": null,
            "selection_source_paths": {},
            "selection_override_reasons": [],
            "selection_precedence": [],
            "selected_route_profile_mapping": null,
            "selected_candidate": null,
            "candidate_pool": [],
            "rejected_candidates": [],
            "readiness_blockers": [],
            "budget_policy": null,
            "budget_verdict": null,
            "max_budget_units": null,
            "selected_over_budget": null,
            "budget_scope": null,
            "selection_budget": null,
            "runtime_budget_ledger": null
        });

        let unready = super::model_profile_readiness_audit_payload_for_route(
            "implementation",
            &unready_route,
        );
        let missing = super::model_profile_readiness_audit_payload_for_route(
            "implementation",
            &missing_route,
        );

        assert_eq!(unready["status"], "blocked");
        assert_eq!(
            unready["blocker_codes"],
            serde_json::json!(["selected_model_profile_not_ready"])
        );
        assert_eq!(
            unready["selected_profile"]["readiness"]["blocker_code"],
            "external_cli_missing_api_key"
        );
        assert!(unready["next_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
        assert_eq!(missing["status"], "blocked");
        assert_eq!(
            missing["blocker_codes"],
            serde_json::json!(["selected_model_profile_missing"])
        );
        assert!(missing["selected_profile"]["readiness_status"].is_null());
    }

    #[test]
    fn validate_routing_targets_fall_back_when_contract_is_missing() {
        let targets = super::route_validate_targets(&serde_json::json!({}));
        assert_eq!(targets, vec!["implementation", "coach", "verification"]);
    }

    #[test]
    fn validate_routing_targets_canonicalize_legacy_dispatch_lane_names() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementer", "coach", "execution_preparation"]
                }
            }
        });

        let targets = super::route_validate_targets(&execution_plan);
        assert_eq!(targets, vec!["implementation", "coach", "architecture"]);
    }

    #[test]
    fn recovery_holds_active_bound_run_when_delegated_cycle_is_open() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
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

        assert!(super::recovery_holds_active_bound_run(Some(&recovery)));
    }

    #[test]
    fn recovery_does_not_hold_ready_head_when_delegated_cycle_is_clear() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            resume_node: None,
            resume_status: "completed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "none".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                reporting_pause_gate: "clear".to_string(),
                continuation_signal: "none".to_string(),
                blocker_code: None,
                lifecycle_stage: "closure_complete".to_string(),
            },
        };

        assert!(!super::recovery_holds_active_bound_run(Some(&recovery)));
        assert!(!super::recovery_holds_active_bound_run(None));
    }

    #[test]
    fn authoritative_dispatch_blocker_codes_include_primary_and_downstream_blockers() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
            activation_runtime_role: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            downstream_dispatch_trace_path: Some("/tmp/downstream-trace.json".to_string()),
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: Some("coach".to_string()),
            downstream_dispatch_note: Some("after coach, verify".to_string()),
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
        };

        let blocker_codes = super::authoritative_dispatch_blocker_codes(Some(&dispatch));
        assert_eq!(blocker_codes.len(), 2);
        assert!(blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence"));
    }

    #[test]
    fn scheduler_execute_runtime_gate_blockers_include_recovery_and_running_dispatch() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "worker".to_string(),
            lifecycle_stage: "worker_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "none".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "worker".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "worker_active".to_string(),
            },
        };
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            blocker_code: None,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_packet_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_active_target: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: None,
            dispatch_surface: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            selected_backend: None,
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
            activation_runtime_role: None,
            activation_agent_type: None,
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
        };
        let blocker_codes =
            super::scheduler_execute_runtime_gate_blocker_codes(Some(&recovery), Some(&dispatch));

        assert_eq!(blocker_codes.blocker_codes.len(), 2);
        assert!(blocker_codes
            .blocker_codes
            .iter()
            .any(|code| code == "open_delegated_cycle"));
        assert!(blocker_codes
            .blocker_codes
            .iter()
            .any(|code| code == "execution_preparation_gate_blocked"));
    }

    #[test]
    fn scheduler_execute_runtime_gate_blockers_apply_blocked_authoritative_dispatch() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_blockers: vec!["pending_lane_evidence".to_string()],
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_packet_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_active_target: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: None,
            dispatch_surface: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            selected_backend: None,
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
            activation_runtime_role: None,
            activation_agent_type: None,
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
        };

        let blocker_codes =
            super::scheduler_execute_runtime_gate_blocker_codes(None, Some(&dispatch));

        assert_eq!(blocker_codes.blocker_codes.len(), 2);
        assert!(blocker_codes
            .blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .blocker_codes
            .iter()
            .any(|code| code == "pending_lane_evidence"));
    }

    #[test]
    fn scheduler_execute_runtime_gate_blockers_mark_execute_projection_blocked() {
        let projection = crate::state_store::TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![scheduling_candidate(
                sample_task("critical-ready"),
                true,
                false,
                true,
                Vec::new(),
                Vec::new(),
            )],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let state_dir = std::path::Path::new("/tmp/vida-scheduler-state");
        let mut plan = super::build_taskflow_scheduler_dispatch_plan(
            projection, 1, None, None, None, None, state_dir, false, true,
        );

        super::apply_scheduler_execute_runtime_gate_blockers(
            &mut plan,
            &super::SchedulerRuntimeGateBlockerSignals {
                blocker_codes: vec!["open_delegated_cycle".to_string()],
                open_delegated_cycle: true,
                active_reservation: false,
            },
        );

        assert_eq!(plan.status, "blocked");
        assert_eq!(plan.execution_status, "open_delegated_cycle");
        assert_eq!(plan.activation_status, "open_delegated_cycle");
        assert_eq!(plan.activation_blocker_codes, vec!["open_delegated_cycle"]);
        assert_eq!(plan.worker_execution_evidence_status, "not_received");
        assert!(!plan.worker_completion_claimed);
        assert_eq!(plan.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(plan.dispatch_receipt.execute_status, "open_delegated_cycle");
        assert_eq!(
            plan.dispatch_receipt.activation_status,
            "open_delegated_cycle"
        );
        assert_eq!(
            plan.dispatch_receipt.worker_execution_evidence_status,
            "not_received"
        );
        assert!(!plan.dispatch_receipt.worker_completion_claimed);
        assert!(plan.next_actions.iter().any(|action| action
            .contains("Resolve the open delegated-cycle gate before scheduler execute")));
    }

    fn sample_task(task_id: &str) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: Some("vida-1".to_string()),
            title: "Sample Task".to_string(),
            description: "sample".to_string(),
            status: "open".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-18T00:00:00Z".to_string(),
            created_by: "tester".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies: Vec::new(),
        }
    }

    fn exception_takeover_dispatch(
        run_id: &str,
    ) -> crate::state_store::RunGraphDispatchReceiptSummary {
        crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            selected_backend: Some("internal_subagents".to_string()),
            exception_path_receipt_id: Some(
                "stream4-recovery-status-actionable-command-fix".to_string(),
            ),
            supersedes_receipt_id: Some(
                "stream4-recovery-status-actionable-command-fix".to_string(),
            ),
            recorded_at: "2026-04-24T18:50:54Z".to_string(),
            activation_runtime_role: Some("verifier".to_string()),
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_packet_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: None,
            downstream_dispatch_note: Some("exception takeover active".to_string()),
            downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
        }
    }

    #[test]
    fn taskflow_next_decision_surfaces_candidate_when_runtime_gate_blocks_ready_head() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            dispatch_surface: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            selected_backend: Some("internal_subagents".to_string()),
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
            activation_runtime_role: None,
            activation_agent_type: None,
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_packet_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_active_target: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: None,
            downstream_dispatch_note: None,
            downstream_dispatch_blockers: Vec::new(),
        };

        let decision = super::build_taskflow_next_decision(
            Some(&sample_task("task-1")),
            true,
            true,
            Some("bundle-check"),
            Some("epic-1"),
            Some(&dispatch),
            None,
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(
            decision
                .candidate_task_context
                .ready_head
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("task-1")
        );
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "delegated_cycle_runtime_gate"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("delegated_cycle_runtime_gate")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow recovery latest --json")
        );
    }

    #[test]
    fn taskflow_next_decision_blocks_ready_head_when_latest_run_graph_is_blocked() {
        let mut latest_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-blocked",
            "task-blocked",
            "analysis",
        );
        latest_status.status = "blocked".to_string();
        latest_status.lifecycle_stage = "analysis_blocked".to_string();

        let decision = super::build_taskflow_next_decision(
            Some(&sample_task("ready-head")),
            false,
            true,
            Some("final"),
            None,
            None,
            Some(&latest_status),
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(
            decision
                .candidate_task_context
                .ready_head
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("ready-head")
        );
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "latest_run_graph_status_blocked"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("latest_run_graph_status_blocked")
        );
        assert!(decision
            .blocker_codes
            .iter()
            .any(|code| code == "latest_run_graph_status_blocked"));
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow recovery status run-blocked --json")
        );

        let (shared_fields, operator_contracts, artifact_refs) =
            super::taskflow_next_operator_contracts(&decision, Some(&latest_status), None, None);
        assert_eq!(shared_fields["status"], serde_json::json!(decision.status));
        assert_eq!(artifact_refs["surface"], "vida taskflow next");
        assert_eq!(shared_fields["artifact_refs"], artifact_refs);
        assert_eq!(
            shared_fields["blocker_codes"],
            serde_json::json!(decision.blocker_codes.clone())
        );
        assert_eq!(
            shared_fields["next_actions"],
            serde_json::json!(decision.next_actions.clone())
        );
        assert_eq!(operator_contracts["status"], shared_fields["status"]);
        assert_eq!(
            operator_contracts["blocker_codes"],
            shared_fields["blocker_codes"]
        );
        assert_eq!(
            operator_contracts["next_actions"],
            shared_fields["next_actions"]
        );
        assert_eq!(
            operator_contracts["artifact_refs"],
            shared_fields["artifact_refs"]
        );
    }

    #[test]
    fn taskflow_next_decision_continues_active_exception_takeover_before_ready_head() {
        let mut latest_status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        latest_status.status = "blocked".to_string();
        latest_status.lifecycle_stage = "analysis_blocked".to_string();
        let stale_binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "feature-reconcile-qwen-cli-carrier-drift-across-config-code".to_string(),
            task_id: "feature-reconcile-qwen-cli-carrier-drift-across-config-code".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "downstream_dispatch_target",
                "task_id": "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
                "run_id": "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
                "dispatch_target": "closure"
            }),
            binding_source: "task_close_reconcile".to_string(),
            why_this_unit: "stale close reconcile binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: "2026-04-22T08:44:36Z".to_string(),
        };
        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");

        let decision = super::build_taskflow_next_decision(
            Some(&sample_task("ready-head")),
            false,
            true,
            Some("final"),
            None,
            Some(&dispatch),
            Some(&latest_status),
            Some(&stale_binding),
            None,
        );

        assert_eq!(decision.status, "pass");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(
            decision
                .candidate_task_context
                .ready_head
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("ready-head")
        );
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "active_exception_takeover_continuation"
        );
        assert!(decision.why_not_now.is_none());
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow consume continue --run-id runtime-audit-state-store-init-lock-timeout --json")
        );
        assert_eq!(
            decision.recommended_surface.as_deref(),
            Some("vida taskflow consume continue")
        );
        assert!(!decision
            .blocker_codes
            .iter()
            .any(|code| code == "latest_run_graph_status_blocked"));
    }

    #[test]
    fn taskflow_next_decision_stops_exception_takeover_self_loop_after_terminal_continue() {
        let mut latest_status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        latest_status.status = "blocked".to_string();
        latest_status.lifecycle_stage = "analysis_blocked".to_string();
        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");

        let decision = super::build_taskflow_next_decision(
            Some(&sample_task("ready-head")),
            false,
            true,
            Some("final"),
            None,
            Some(&dispatch),
            Some(&latest_status),
            None,
            Some("runtime-audit-state-store-init-lock-timeout"),
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(
            decision
                .candidate_task_context
                .ready_head
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("ready-head")
        );
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "terminal_continue_snapshot_without_next_bounded_unit"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("terminal_continue_snapshot_without_next_bounded_unit")
        );
        assert_ne!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow consume continue --run-id runtime-audit-state-store-init-lock-timeout --json")
        );
        assert_eq!(
            decision.recommended_surface.as_deref(),
            Some("vida taskflow continuation bind")
        );
        assert!(decision
            .blocker_codes
            .iter()
            .any(|code| code == "terminal_continue_snapshot_without_next_bounded_unit"));
    }

    #[test]
    fn taskflow_next_decision_reports_no_ready_task_with_explicit_next_action() {
        let decision = super::build_taskflow_next_decision(
            None,
            false,
            false,
            None,
            Some("epic-1"),
            None,
            None,
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(decision.candidate_task_context.ready_head, None);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "no_ready_task"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("backlog_no_ready_task")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida task ready --scope epic-1 --json")
        );
        assert!(decision
            .blocker_codes
            .iter()
            .any(|code| code == "no_ready_tasks"));
    }

    #[test]
    fn taskflow_next_decision_fails_closed_after_closure_without_explicit_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure",
            "task-closure",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        let decision = super::build_taskflow_next_decision(
            None,
            false,
            true,
            Some("final"),
            None,
            None,
            Some(&status),
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "completed_without_explicit_next_bounded_unit"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("completed_without_explicit_next_bounded_unit")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow continuation bind run-closure --task-id <task-id> --json")
        );
    }
}

pub(crate) async fn run_taskflow_proxy(args: ProxyArgs) -> ExitCode {
    if matches!(args.args.first().map(String::as_str), Some("query")) {
        return run_taskflow_query(&args.args);
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("--version" | "-V")
    ) {
        let command = TaskflowCli::command();
        let version = command
            .get_version()
            .unwrap_or(env!("CARGO_PKG_VERSION"))
            .to_string();
        println!("taskflow {version}");
        return ExitCode::SUCCESS;
    }

    if let Some(topic) = taskflow_help_topic(&args.args) {
        print_taskflow_proxy_help(topic);
        return ExitCode::SUCCESS;
    }

    if matches!(args.args.first().map(String::as_str), Some("recovery")) {
        return run_taskflow_recovery(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("task")) {
        return route_taskflow_task(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("next")) {
        return run_taskflow_next_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph-summary")) {
        return run_taskflow_graph_summary(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph")) {
        return run_taskflow_graph_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("plan")) {
        return crate::taskflow_plan_graph::run_taskflow_plan(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("replan")) {
        return run_taskflow_replan_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("scheduler")) {
        return run_taskflow_scheduler_surface(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("route" | "validate-routing" | "config-actuation")
    ) {
        return run_taskflow_route_diagnostic(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("doctor")) {
        return route_taskflow_doctor(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("status")) {
        return route_taskflow_status(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("bootstrap-spec")
    ) {
        return run_taskflow_bootstrap_spec(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("protocol-binding")
    ) {
        return taskflow_protocol_binding::run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("continuation")) {
        return crate::taskflow_continuation::run_taskflow_continuation(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("packet")) {
        return crate::taskflow_packet::run_taskflow_packet(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("artifact" | "artifacts")
    ) {
        return crate::taskflow_artifacts::run_taskflow_artifacts(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        if taskflow_consume::try_print_taskflow_consume_nested_help(&args.args) {
            return ExitCode::SUCCESS;
        }
        let consume_subcommand = args.args.get(1).map(String::as_str);
        if matches!(consume_subcommand, Some("continue" | "advance")) {
            let state_root = proxy_state_dir();
            if let Err(error) = enforce_execution_preparation_contract_gate(&state_root) {
                eprintln!(
                    "{error}\nFail-closed: `vida taskflow consume {}` requires release-1 execution-preparation evidence/contract.",
                    consume_subcommand.unwrap_or("unknown")
                );
                return ExitCode::from(1);
            }
        }
        if matches!(
            consume_subcommand,
            None | Some(
                "bundle" | "agent-system" | "final" | "continue" | "advance" | "--help" | "-h"
            )
        ) {
            return taskflow_consume::run_taskflow_consume(&args.args).await;
        }
    }

    if matches!(args.args.first().map(String::as_str), Some("run-graph")) {
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("status" | "latest" | "diagnose" | "diagnose-latest" | "--help" | "-h")
        ) {
            return run_taskflow_run_graph(&args.args).await;
        }
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("seed" | "advance" | "dispatch-init" | "init" | "update")
        ) {
            return run_taskflow_run_graph_mutation(&args.args).await;
        }
    }

    let subcommand = args.args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unsupported `vida taskflow {subcommand}` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to the external TaskFlow runtime."
    );
    ExitCode::from(2)
}
