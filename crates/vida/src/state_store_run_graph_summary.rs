use super::*;
use crate::release1_contracts::lane_status_has_required_evidence;
use crate::state_store::state_store_task_models::{
    task_has_label, task_is_spec_pack_child, task_is_work_pool_pack_child,
};
use crate::taskflow_run_graph::{
    approval_delegation_transition_kind, clear_run_graph_dispatch_init_fast_cache,
    is_dispatch_resume_handoff_complete,
};
use crate::RuntimeConsumptionLaneSelection;

const MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES: u64 = 1024 * 1024;

fn reconcile_run_graph_status_with_dispatch_receipt(
    mut status: RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> Result<RunGraphStatus, StateStoreError> {
    let Some(receipt) = receipt else {
        return Ok(status);
    };
    let receipt = StateStore::validate_run_graph_dispatch_receipt_contract(receipt.clone())?;
    if terminal_closure_status(&status)
        && status.policy_gate == "historical_closed_task_stale_run_retired"
    {
        return Ok(status);
    }
    if terminal_closure_status_has_explicit_receipt_override(&status, &receipt) {
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    let stale_downstream_blockers_are_superseded_by_ready_handoff =
        ready_dispatch_handoff_matches_downstream_receipt(&status, &receipt);
    if active_exception_takeover_receipt_is_behind_status(&status, &receipt) {
        return Ok(status);
    }
    let executed_analysis_missing_owned_scope_handoff = receipt.dispatch_status == "executed"
        && receipt.dispatch_target == "analysis"
        && receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| blocker == "missing_owned_write_scope");
    let pre_execution_routed_handoff = receipt.dispatch_status == "routed"
        && receipt.blocker_code.as_deref().is_none_or(str::is_empty)
        && matches!(
            receipt.lane_status.as_deref(),
            Some("lane_open") | Some("lane_running") | Some("packet_ready") | None
        )
        && receipt.downstream_dispatch_status.is_none();
    let blocked_receipt = matches!(receipt.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(
            receipt.lane_status.as_deref(),
            Some("lane_blocked")
                | Some("lane_failed")
                | Some("lane_exception_recorded")
                | Some("lane_exception_takeover")
        )
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || (!pre_execution_routed_handoff
            && !stale_downstream_blockers_are_superseded_by_ready_handoff
            && !executed_analysis_missing_owned_scope_handoff
            && !receipt.downstream_dispatch_blockers.is_empty());
    let spec_post_design_gate_blocked = receipt.dispatch_status == "executed"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
    let retry_target = receipt.dispatch_target.replace('-', "_");
    let retry_ready_same_lane = receipt.dispatch_kind == "agent_lane"
        && receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("internal_activation_view_only")
        && status.status == "ready"
        && status.active_node == receipt.dispatch_target
        && status.next_node.as_deref() == Some(retry_target.as_str())
        && status.handoff_state == format!("awaiting_{retry_target}")
        && status.resume_target == format!("dispatch.{retry_target}_lane")
        && status.recovery_ready;
    if blocked_receipt {
        if retry_ready_same_lane {
            if let Some(selected_backend) = receipt
                .selected_backend
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                status.selected_backend = selected_backend.to_string();
            }
            return Ok(status);
        }
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        if spec_post_design_gate_blocked {
            let completed_target = receipt
                .downstream_dispatch_last_target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(receipt.dispatch_target.as_str());
            let lifecycle_target = completed_target.replace('-', "_");
            status.active_node = completed_target.to_string();
            status.next_node = None;
            status.lifecycle_stage = format!("{lifecycle_target}_complete");
            status.policy_gate = "not_required".to_string();
            status.handoff_state = "none".to_string();
            status.resume_target = "none".to_string();
            status.context_state = "sealed".to_string();
        } else {
            let blocked_target = receipt.dispatch_target.trim().replace('-', "_");
            status.active_node = receipt.dispatch_target.clone();
            status.next_node = None;
            status.lifecycle_stage = format!("{blocked_target}_blocked");
            status.handoff_state = "none".to_string();
            status.resume_target = if blocked_agent_lane_receipt_keeps_resume_target(&receipt) {
                format!("dispatch.{blocked_target}")
            } else {
                "none".to_string()
            };
            status.context_state = "sealed".to_string();
        }
        status.checkpoint_kind = "none".to_string();
        status.status = "blocked".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    let closure_dispatch_completed = receipt.dispatch_target == "closure"
        && receipt.dispatch_status == "executed"
        && receipt.blocker_code.is_none();
    if closure_dispatch_completed {
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.context_state = "sealed".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    if receipt.dispatch_status == "executed"
        && receipt.blocker_code.as_deref().is_none_or(str::is_empty)
        && receipt.downstream_dispatch_ready
        && receipt.downstream_dispatch_blockers.is_empty()
        && receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    {
        if status.status == "ready"
            && status.recovery_ready
            && status.active_node != receipt.dispatch_target
        {
            return Ok(status);
        }
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        let completed_target = receipt.dispatch_target.trim();
        let lifecycle_target = completed_target.replace('-', "_");
        let downstream_node = receipt
            .downstream_dispatch_target
            .as_deref()
            .expect("downstream target checked above")
            .replace('-', "_");
        status.active_node = completed_target.to_string();
        status.next_node = Some(downstream_node.clone());
        status.status = "ready".to_string();
        status.lifecycle_stage = format!("{lifecycle_target}_complete");
        status.policy_gate = "not_required".to_string();
        status.handoff_state = format!("awaiting_{downstream_node}");
        status.resume_target = format!("dispatch.{downstream_node}_lane");
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.recovery_ready = true;
        return Ok(status);
    }
    if status.status == "completed" {
        return Ok(status);
    }
    if receipt.dispatch_target == "analysis"
        && receipt.dispatch_status == "executed"
        && status.active_node == "analysis"
    {
        if let Some(next_target) = receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let next_node = next_target.replace('-', "_");
            if status.next_node.is_none() {
                status.next_node = Some(next_node.clone());
            }
            status.status = "ready".to_string();
            status.lifecycle_stage = "analysis_active".to_string();
            if status.policy_gate == "validation_report_required" {
                status.policy_gate = "targeted_verification".to_string();
            }
            status.handoff_state = format!("awaiting_{next_node}");
            status.resume_target = format!("dispatch.{next_node}_lane");
            status.recovery_ready = true;
        }
    }
    Ok(status)
}

fn blocked_agent_lane_receipt_keeps_resume_target(receipt: &RunGraphDispatchReceiptStored) -> bool {
    if receipt.dispatch_kind != "agent_lane"
        || !receipt
            .dispatch_result_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    {
        return false;
    }
    let blocker_code = receipt.blocker_code.as_deref().unwrap_or_default();
    !matches!(
        blocker_code,
        "internal_activation_view_only"
            | crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
    )
}

fn tracked_flow_materialization_result_passed(
    state_root: Option<&std::path::Path>,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    tracked_flow_materialization_result(state_root, receipt).is_some()
}

#[derive(Debug, Clone)]
struct TrackedFlowMaterializationResult {
    packet_key: String,
    epic_task_id: Option<String>,
    task_id: String,
}

fn tracked_flow_materialization_result(
    state_root: Option<&std::path::Path>,
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<TrackedFlowMaterializationResult> {
    if !stored_receipt_can_reconcile_tracked_flow_materialization(receipt) {
        return None;
    }
    let mut candidate_paths = Vec::new();
    if let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        candidate_paths.push(std::path::PathBuf::from(result_path));
    }
    if let Some(root) = state_root {
        let results_dir = root.join("runtime-consumption").join("dispatch-results");
        if let Ok(entries) = std::fs::read_dir(results_dir) {
            let mut result_paths = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(std::ffi::OsStr::to_str)
                        .is_some_and(|name| {
                            name.starts_with(&format!("{}-", receipt.run_id))
                                && name.ends_with(".json")
                        })
                })
                .collect::<Vec<_>>();
            result_paths.sort();
            candidate_paths.extend(result_paths.into_iter().rev());
        }
    }
    candidate_paths.into_iter().find_map(|result_path| {
        let Ok(result_body) = std::fs::read_to_string(&result_path) else {
            return None;
        };
        tracked_flow_materialization_result_from_body(&result_body, receipt)
    })
}

fn stored_receipt_can_reconcile_tracked_flow_materialization(
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    matches!(receipt.dispatch_status.as_str(), "blocked" | "executed")
        && receipt.dispatch_surface.as_deref() == Some("vida task ensure")
        && matches!(
            receipt.dispatch_target.as_str(),
            "work-pool-pack" | "dev-pack"
        )
        && receipt
            .blocker_code
            .as_deref()
            .is_none_or(|code| code == "internal_activation_view_only")
}

fn tracked_flow_materialization_result_body_passed(
    result_body: &str,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    tracked_flow_materialization_result_from_body(result_body, receipt).is_some()
}

fn tracked_flow_materialization_result_from_body(
    result_body: &str,
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<TrackedFlowMaterializationResult> {
    let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&result_body) else {
        return None;
    };
    if result_json["status"].as_str() != Some("pass")
        || result_json["surface"].as_str() != Some("vida task ensure")
    {
        return None;
    }
    let expected_packet_key = match receipt.dispatch_target.as_str() {
        "work-pool-pack" => "work_pool_task",
        "dev-pack" => "dev_task",
        _ => return None,
    };
    if result_json["packet_key"].as_str() != Some(expected_packet_key) {
        return None;
    }
    let task_id = result_json["task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let task_materialized = result_json["task"]["created"].as_bool() == Some(true)
        || result_json["task"]["reused_existing"].as_bool() == Some(true);
    if !task_materialized {
        return None;
    }
    Some(TrackedFlowMaterializationResult {
        packet_key: expected_packet_key.to_string(),
        epic_task_id: result_json["epic"]["task_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        task_id,
    })
}

fn ready_dispatch_handoff_matches_downstream_receipt(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if status.status != "ready"
        || !status.recovery_ready
        || !is_dispatch_resume_handoff_complete(status)
        || receipt.dispatch_status != "executed"
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || matches!(
            receipt.lane_status.as_deref(),
            Some("lane_blocked")
                | Some("lane_failed")
                | Some("lane_exception_recorded")
                | Some("lane_exception_takeover")
        )
    {
        return false;
    }
    let Some(downstream_target) = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if status.active_node != receipt.dispatch_target {
        return false;
    }
    let downstream_node = downstream_target.replace('-', "_");
    status.next_node.as_deref() == Some(downstream_node.as_str())
        && status.handoff_state == format!("awaiting_{downstream_node}")
        && status.resume_target == format!("dispatch.{downstream_node}_lane")
}

fn receipt_has_pending_spec_first_work_pool_handoff_gate(
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == "executed"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        })
}

fn has_receipt_evidence_id(value: Option<&str>) -> bool {
    value.map(str::trim).is_some_and(|value| !value.is_empty())
}

fn terminal_closure_status(status: &RunGraphStatus) -> bool {
    status.status == "completed"
        && status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
}

fn terminal_closure_supersedes_stale_handoff_receipt(
    status: &RunGraphStatus,
    receipt: &mut RunGraphDispatchReceipt,
) -> bool {
    if !terminal_closure_status(status)
        || receipt.dispatch_status != "blocked"
        || receipt.blocker_code.as_deref()
            != Some(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
            ))
        || receipt.exception_path_receipt_id.is_some()
        || receipt.supersedes_receipt_id.is_some()
    {
        return false;
    }
    receipt.dispatch_status = "executed".to_string();
    receipt.lane_status = "lane_completed".to_string();
    receipt.blocker_code = None;
    receipt.downstream_dispatch_target = Some("closure".to_string());
    receipt.downstream_dispatch_note =
        Some("terminal closure superseded stale developer handoff blocker".to_string());
    receipt.downstream_dispatch_ready = false;
    receipt.downstream_dispatch_blockers.clear();
    receipt.downstream_dispatch_status = Some("executed".to_string());
    receipt.downstream_dispatch_active_target = Some("closure".to_string());
    receipt.downstream_dispatch_last_target = Some("closure".to_string());
    true
}

fn task_status_is_terminal_for_continuation(status: &str) -> bool {
    matches!(status.trim(), "closed" | "completed")
}

fn terminal_closure_status_has_explicit_receipt_override(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if !terminal_closure_status(status) {
        return false;
    }
    let has_supersession = has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref());
    if !has_supersession {
        return false;
    }
    matches!(
        receipt.lane_status.as_deref(),
        Some("lane_exception_takeover") | Some("lane_superseded")
    )
}

fn stored_receipt_has_active_exception_takeover(receipt: &RunGraphDispatchReceiptStored) -> bool {
    receipt.lane_status.as_deref() == Some("lane_exception_takeover")
        && has_receipt_evidence_id(receipt.exception_path_receipt_id.as_deref())
        && has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref())
}

fn active_exception_takeover_receipt_is_behind_status(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if !stored_receipt_has_active_exception_takeover(receipt) {
        return false;
    }
    if terminal_closure_status(status) && status.resume_target == "none" {
        return true;
    }
    status.status == "ready"
        && status.recovery_ready
        && status.resume_target.starts_with("dispatch.")
        && status.active_node != receipt.dispatch_target
}

fn continuation_binding_active_kind(binding: &RunGraphContinuationBinding) -> Option<&str> {
    binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
}

fn continuation_binding_active_node(binding: &RunGraphContinuationBinding) -> Option<&str> {
    binding
        .active_bounded_unit
        .get("active_node")
        .and_then(serde_json::Value::as_str)
}

fn reconcile_continuation_binding_with_exception_takeover_receipt(
    binding: RunGraphContinuationBinding,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> RunGraphContinuationBinding {
    let Some(receipt) = receipt else {
        return binding;
    };
    if binding.run_id != receipt.run_id
        || continuation_binding_active_kind(&binding) != Some("run_graph_task")
        || !stored_receipt_has_active_exception_takeover(receipt)
    {
        return binding;
    }
    let active_node = receipt.dispatch_target.trim();
    if active_node.is_empty() || continuation_binding_active_node(&binding) == Some(active_node) {
        return binding;
    }
    let recorded_at = if receipt.recorded_at.trim().is_empty() {
        binding.recorded_at.clone()
    } else {
        receipt.recorded_at.clone()
    };
    RunGraphContinuationBinding {
        run_id: binding.run_id.clone(),
        task_id: binding.task_id.clone(),
        status: "bound".to_string(),
        active_bounded_unit: serde_json::json!({
            "kind": "run_graph_task",
            "task_id": binding.task_id.clone(),
            "run_id": binding.run_id.clone(),
            "active_node": active_node,
        }),
        binding_source: "latest_run_graph_exception_takeover_dispatch".to_string(),
        why_this_unit: format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}` at node `{}`.",
            binding.task_id, active_node
        ),
        primary_path: "normal_delivery_path".to_string(),
        sequential_vs_parallel_posture: "sequential_only_exception_takeover".to_string(),
        request_text: binding.request_text,
        recorded_at,
    }
}

pub(crate) fn normalize_legacy_downstream_preview_drift(
    mut receipt: RunGraphDispatchReceiptStored,
) -> RunGraphDispatchReceiptStored {
    let active_dispatch_with_upstream_lane_evidence = receipt.dispatch_status != "executed"
        && (has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref())
            || has_receipt_evidence_id(receipt.exception_path_receipt_id.as_deref()));
    let stale_downstream_preview_present = receipt.downstream_dispatch_status.is_some()
        || receipt.downstream_dispatch_ready
        || receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
    if active_dispatch_with_upstream_lane_evidence && stale_downstream_preview_present {
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_note = None;
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_result_path = None;
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_executed_count = 0;
        receipt.downstream_dispatch_active_target = None;
    }
    receipt
}

fn reconcile_run_graph_status_with_closed_task(
    mut status: RunGraphStatus,
    task: Option<&TaskRecord>,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> RunGraphStatus {
    let Some(task) = task else {
        return status;
    };
    if !task_status_is_terminal_for_continuation(&task.status) {
        return status;
    }
    if receipt.and_then(|receipt| receipt.lane_status.as_deref()) == Some("lane_exception_recorded")
    {
        if status.active_node == "closure"
            && status.status == "blocked"
            && status.lifecycle_stage == "closure_blocked"
        {
            status.lifecycle_stage = "closure_complete".to_string();
        }
        return status;
    }
    let terminal_closure_status = status.active_node == "closure"
        && matches!(status.status.as_str(), "blocked" | "completed")
        && matches!(
            status.lifecycle_stage.as_str(),
            "closure_blocked" | "closure_complete"
        )
        && status.next_node.is_none()
        && status.handoff_state == "none"
        && status.resume_target == "none";
    if !StateStore::run_graph_status_allows_task_close_closure_binding(&status)
        && !terminal_closure_status
    {
        return status;
    }

    status.status = "completed".to_string();
    if status.active_node == "closure" {
        status.lifecycle_stage = "closure_complete".to_string();
    } else if status.lifecycle_stage != "closure_complete" {
        status.lifecycle_stage = "implementation_complete".to_string();
    }
    status.next_node = None;
    if !StateStore::run_graph_status_is_reconciled_terminal_closure(&status) {
        status.policy_gate = "not_required".to_string();
    }
    status.handoff_state = "none".to_string();
    status.context_state = "sealed".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}

pub(crate) fn requires_memory_governance_enforcement(policy_gate: &str) -> bool {
    let normalized = policy_gate.trim().to_ascii_lowercase();
    normalized.contains("consent")
        || normalized.contains("ttl")
        || normalized.contains("correction")
        || normalized.contains("delete")
        || normalized.contains("deletion")
}

pub(crate) fn handoff_state_links_consent_ttl(handoff_state: &str) -> bool {
    let normalized = handoff_state.trim().to_ascii_lowercase();
    normalized.contains("consent") && normalized.contains("ttl")
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, Clone)]
pub struct RunGraphDelegationGateSummary {
    pub active_node: String,
    pub lifecycle_stage: String,
    pub delegated_cycle_open: bool,
    pub delegated_cycle_state: String,
    pub local_exception_takeover_gate: String,
    pub blocker_code: Option<String>,
    pub reporting_pause_gate: String,
    pub continuation_signal: String,
}

impl RunGraphDelegationGateSummary {
    pub(crate) fn from_status(status: &RunGraphStatus) -> Self {
        let handoff_pending = status.next_node.is_some()
            || status.handoff_state != "none"
            || status.resume_target != "none";
        let delegated_lane_active = !handoff_pending
            && status.status != "completed"
            && status.active_node != "planning"
            && status.lifecycle_stage.ends_with("_active");
        let delegated_lane_blocked = !handoff_pending
            && status.status == "blocked"
            && status.active_node != "planning"
            && status.lifecycle_stage.ends_with("_blocked")
            && status.policy_gate != "not_required";
        let (delegated_cycle_open, delegated_cycle_state) = if handoff_pending {
            (true, "handoff_pending".to_string())
        } else if delegated_lane_active {
            (true, "delegated_lane_active".to_string())
        } else if delegated_lane_blocked {
            (true, "delegated_lane_blocked".to_string())
        } else {
            (false, "clear".to_string())
        };
        let local_exception_takeover_gate = if delegated_cycle_open {
            "blocked_open_delegated_cycle".to_string()
        } else {
            "delegated_cycle_clear".to_string()
        };
        let blocker_code = if local_exception_takeover_gate == "blocked_open_delegated_cycle" {
            Some(
                canonical_blocker_code_str(BlockerCode::OpenDelegatedCycle.as_str())
                    .unwrap_or(BlockerCode::OpenDelegatedCycle.as_str())
                    .to_string(),
            )
        } else {
            None
        };
        let reporting_pause_gate = if delegated_cycle_open {
            "non_blocking_only".to_string()
        } else if status.status == "completed" {
            "closure_candidate".to_string()
        } else {
            "continuation_check_required".to_string()
        };
        let continuation_signal = if delegated_cycle_open {
            "continue_routing_non_blocking".to_string()
        } else if status.status == "completed" {
            "continue_after_reports".to_string()
        } else {
            "continuation_check_required".to_string()
        };

        Self {
            active_node: status.active_node.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            delegated_cycle_open,
            delegated_cycle_state,
            local_exception_takeover_gate,
            blocker_code,
            reporting_pause_gate,
            continuation_signal,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "node={} lifecycle={} delegated_cycle_open={} delegated_cycle_state={} local_exception_takeover_gate={} blocker_code={} reporting_pause_gate={} continuation_signal={}",
            self.active_node,
            self.lifecycle_stage,
            self.delegated_cycle_open,
            self.delegated_cycle_state,
            self.local_exception_takeover_gate,
            self.blocker_code.as_deref().unwrap_or("none"),
            self.reporting_pause_gate,
            self.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphRecoverySummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub resume_node: Option<String>,
    pub resume_status: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub recovery_ready: bool,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphRecoverySummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            resume_node: status.next_node,
            resume_status: status.status,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            recovery_ready: status.recovery_ready,
            delegation_gate,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} resume_node={} resume_status={} checkpoint={} resume_target={} gate={} handoff={} recovery_ready={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.resume_node.as_deref().unwrap_or("none"),
            self.resume_status,
            self.checkpoint_kind,
            self.resume_target,
            self.policy_gate,
            self.handoff_state,
            self.recovery_ready,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphCheckpointSummary {
    pub run_id: String,
    pub task_id: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

impl RunGraphCheckpointSummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            recovery_ready: status.recovery_ready,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} checkpoint={} resume_target={} recovery_ready={}",
            self.run_id,
            self.task_id,
            self.checkpoint_kind,
            self.resume_target,
            self.recovery_ready
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphDispatchReceiptSummary {
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    pub lane_status: String,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub dispatch_kind: String,
    pub dispatch_surface: Option<String>,
    pub dispatch_command: Option<String>,
    pub dispatch_packet_path: Option<String>,
    pub dispatch_result_path: Option<String>,
    pub blocker_code: Option<String>,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_command: Option<String>,
    pub downstream_dispatch_note: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
    pub downstream_dispatch_packet_path: Option<String>,
    pub downstream_dispatch_status: Option<String>,
    pub downstream_dispatch_result_path: Option<String>,
    pub downstream_dispatch_trace_path: Option<String>,
    pub downstream_dispatch_executed_count: u32,
    pub downstream_dispatch_active_target: Option<String>,
    pub downstream_dispatch_last_target: Option<String>,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub selected_backend: Option<String>,
    pub effective_execution_posture: serde_json::Value,
    pub route_policy: serde_json::Value,
    pub activation_evidence: serde_json::Value,
    pub recorded_at: String,
}

impl RunGraphDispatchReceiptSummary {
    pub(crate) fn from_receipt(receipt: RunGraphDispatchReceipt) -> Self {
        let raw_lane_status = receipt.lane_status.trim();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        let lane_status = if raw_lane_status.is_empty() {
            derive_lane_status(
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
            .as_str()
            .to_string()
        } else if downstream_dispatch_allows_completed_lane_status(
            receipt.downstream_dispatch_status.as_deref(),
            canonical_lane_status,
        ) || (canonical_lane_status == LaneStatus::LaneCompleted.as_str()
            && receipt.dispatch_status == "executed")
        {
            canonical_lane_status.to_string()
        } else {
            normalize_run_graph_lane_status(
                Some(raw_lane_status),
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
        };
        let blocker_code = receipt
            .blocker_code
            .as_deref()
            .and_then(canonical_blocker_code_str)
            .map(str::to_string)
            .or(receipt.blocker_code.clone());
        let mut downstream_dispatch_blockers = receipt.downstream_dispatch_blockers;
        downstream_dispatch_blockers.sort_unstable();
        Self {
            run_id: receipt.run_id,
            dispatch_target: receipt.dispatch_target,
            dispatch_status: receipt.dispatch_status,
            lane_status,
            supersedes_receipt_id: receipt.supersedes_receipt_id,
            exception_path_receipt_id: receipt.exception_path_receipt_id,
            dispatch_kind: receipt.dispatch_kind,
            dispatch_surface: receipt.dispatch_surface,
            dispatch_command: receipt.dispatch_command,
            dispatch_packet_path: receipt.dispatch_packet_path,
            dispatch_result_path: receipt.dispatch_result_path,
            blocker_code,
            downstream_dispatch_target: receipt.downstream_dispatch_target,
            downstream_dispatch_command: receipt.downstream_dispatch_command,
            downstream_dispatch_note: receipt.downstream_dispatch_note,
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_blockers,
            downstream_dispatch_packet_path: receipt.downstream_dispatch_packet_path,
            downstream_dispatch_status: receipt.downstream_dispatch_status,
            downstream_dispatch_result_path: receipt.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: receipt.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: receipt.downstream_dispatch_active_target,
            downstream_dispatch_last_target: receipt.downstream_dispatch_last_target,
            activation_agent_type: receipt.activation_agent_type,
            activation_runtime_role: receipt.activation_runtime_role,
            selected_backend: receipt.selected_backend,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: receipt.recorded_at,
        }
    }

    pub(crate) fn with_effective_execution_posture(
        mut self,
        effective_execution_posture: serde_json::Value,
    ) -> Self {
        self.effective_execution_posture = effective_execution_posture;
        self
    }

    pub(crate) fn with_route_policy(mut self, route_policy: serde_json::Value) -> Self {
        self.route_policy = route_policy;
        self
    }

    pub(crate) fn with_activation_evidence(
        mut self,
        activation_evidence: serde_json::Value,
    ) -> Self {
        self.activation_evidence = activation_evidence;
        self
    }

    pub(crate) fn dispatch_run_graph_trace_ref(&self) -> Option<String> {
        self.downstream_dispatch_trace_path
            .clone()
            .or_else(|| {
                self.dispatch_result_path
                    .as_ref()
                    .map(|path| format!("dispatch-result:{path}"))
            })
            .or_else(|| {
                self.dispatch_packet_path
                    .as_ref()
                    .map(|path| format!("dispatch-packet:{path}"))
            })
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} target={} status={} lane_status={} supersedes_receipt_id={} exception_path_receipt_id={} blocker_code={} kind={} surface={} command={} packet={} result={} next_target={} next_command={} next_note={} next_ready={} next_blockers={} next_packet={} next_status={} next_result={} next_trace={} next_count={} next_last_target={} agent={} runtime_role={} backend={} posture={} route_backend={} evidence={} recorded_at={}",
            self.run_id,
            self.dispatch_target,
            self.dispatch_status,
            self.lane_status,
            self.supersedes_receipt_id.as_deref().unwrap_or("none"),
            self.exception_path_receipt_id.as_deref().unwrap_or("none"),
            self.blocker_code.as_deref().unwrap_or("none"),
            self.dispatch_kind,
            self.dispatch_surface.as_deref().unwrap_or("none"),
            self.dispatch_command.as_deref().unwrap_or("none"),
            self.dispatch_packet_path.as_deref().unwrap_or("none"),
            self.dispatch_result_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_target.as_deref().unwrap_or("none"),
            self.downstream_dispatch_command
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_note.as_deref().unwrap_or("none"),
            self.downstream_dispatch_ready,
            if self.downstream_dispatch_blockers.is_empty() {
                "none".to_string()
            } else {
                self.downstream_dispatch_blockers.join("|")
            },
            self.downstream_dispatch_packet_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_status.as_deref().unwrap_or("none"),
            self.downstream_dispatch_result_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_trace_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_executed_count,
            self.downstream_dispatch_last_target
                .as_deref()
                .unwrap_or("none"),
            self.activation_agent_type.as_deref().unwrap_or("none"),
            self.activation_runtime_role.as_deref().unwrap_or("none"),
            self.selected_backend.as_deref().unwrap_or("none"),
            self.effective_execution_posture["effective_posture_kind"]
                .as_str()
                .unwrap_or("unknown"),
            self.route_policy["route_primary_backend"]
                .as_str()
                .unwrap_or("none"),
            self.activation_evidence["activation_kind"]
                .as_str()
                .unwrap_or("unknown"),
            self.recorded_at
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, PartialEq, Eq, Clone)]
pub struct RunGraphApprovalDelegationReceipt {
    pub receipt_id: String,
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub route_task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub resume_target: String,
    pub transition_kind: String,
    pub recorded_at: String,
}

impl RunGraphApprovalDelegationReceipt {
    pub(crate) fn from_status(
        status: &RunGraphStatus,
        transition_kind: &str,
        recorded_at: String,
    ) -> Self {
        let receipt_id = format!(
            "run-graph-approval-delegation-{run_id}-{recorded_at}",
            run_id = status.run_id
        );
        Self {
            receipt_id,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_class: status.task_class.clone(),
            route_task_class: status.route_task_class.clone(),
            active_node: status.active_node.clone(),
            next_node: status.next_node.clone(),
            status: status.status.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            policy_gate: status.policy_gate.clone(),
            handoff_state: status.handoff_state.clone(),
            resume_target: status.resume_target.clone(),
            transition_kind: transition_kind.to_string(),
            recorded_at,
        }
    }
}

fn ensure_run_graph_approval_delegation_receipt_consistency(
    receipt: &RunGraphApprovalDelegationReceipt,
) -> Result<(), StateStoreError> {
    if receipt.receipt_id.trim().is_empty()
        || receipt.run_id.trim().is_empty()
        || receipt.task_id.trim().is_empty()
        || receipt.task_class.trim().is_empty()
        || receipt.route_task_class.trim().is_empty()
        || receipt.active_node.trim().is_empty()
        || receipt.status.trim().is_empty()
        || receipt.lifecycle_stage.trim().is_empty()
        || receipt.policy_gate.trim().is_empty()
        || receipt.handoff_state.trim().is_empty()
        || receipt.resume_target.trim().is_empty()
        || receipt.transition_kind.trim().is_empty()
        || receipt.recorded_at.trim().is_empty()
    {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph approval/delegation receipt summary is inconsistent for `{}`: all receipt fields must be non-empty",
                receipt.run_id
            ),
        });
    }

    let is_route_bound_implementation =
        receipt.task_class == "implementation" && receipt.route_task_class == "implementation";
    let approval_wait = receipt.transition_kind == "approval_wait";
    let approval_complete = receipt.transition_kind == "approval_complete";
    if !is_route_bound_implementation || (!approval_wait && !approval_complete) {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph approval/delegation receipt summary is inconsistent for `{}`: transition_kind `{}` must be route-bound to implementation",
                receipt.run_id, receipt.transition_kind
            ),
        });
    }

    match receipt.transition_kind.as_str() {
        "approval_wait" => {
            if receipt.status != "awaiting_approval"
                || receipt.lifecycle_stage != "approval_wait"
                || receipt.policy_gate
                    != crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str()
                || receipt.handoff_state != "awaiting_approval"
                || receipt.resume_target != "dispatch.approval"
                || receipt.next_node.as_deref() != Some("approval")
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "run-graph approval/delegation receipt summary is inconsistent for `{}`: approval_wait receipts must carry the approval route shape",
                        receipt.run_id
                    ),
                });
            }
        }
        "approval_complete" => {
            if receipt.status != "completed"
                || receipt.lifecycle_stage != "implementation_complete"
                || receipt.policy_gate != "not_required"
                || receipt.handoff_state != "none"
                || receipt.resume_target != "none"
                || receipt.next_node.is_some()
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "run-graph approval/delegation receipt summary is inconsistent for `{}`: approval_complete receipts must carry the completion route shape",
                        receipt.run_id
                    ),
                });
            }
        }
        _ => unreachable!("receipt.transition_kind is canonical above"),
    }

    Ok(())
}

pub(crate) fn latest_run_graph_dispatch_receipt_matches_status(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    matches!(
        (
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id
        ),
        (Some(status_run_id), Some(receipt_run_id)) if status_run_id == receipt_run_id
    )
}

pub(crate) fn latest_run_graph_dispatch_receipt_summary_is_inconsistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    latest_run_graph_status_run_id.is_some()
        && !latest_run_graph_dispatch_receipt_matches_status(
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id,
        )
}

pub(crate) fn latest_run_graph_dispatch_receipt_signal_is_ambiguous(
    receipt: &RunGraphDispatchReceiptSummary,
) -> bool {
    matches!(
        receipt.dispatch_status.as_str(),
        "packet_ready" | "routed" | "executing" | "executed" | "blocked"
    ) && receipt.lane_status.as_str()
        != normalize_run_graph_lane_status(
            Some(receipt.lane_status.as_str()),
            &receipt.dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        || !matches!(
            receipt.dispatch_status.as_str(),
            "packet_ready" | "routed" | "executing" | "executed" | "blocked"
        )
}

pub(crate) fn latest_run_graph_evidence_snapshot_is_consistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_recovery_run_id: Option<&str>,
    latest_run_graph_checkpoint_run_id: Option<&str>,
    latest_run_graph_gate_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    let Some(latest_run_graph_status_run_id) = latest_run_graph_status_run_id else {
        return latest_run_graph_recovery_run_id.is_none()
            && latest_run_graph_checkpoint_run_id.is_none()
            && latest_run_graph_gate_run_id.is_none()
            && latest_run_graph_dispatch_receipt_run_id.is_none();
    };
    [
        latest_run_graph_recovery_run_id,
        latest_run_graph_checkpoint_run_id,
        latest_run_graph_gate_run_id,
        latest_run_graph_dispatch_receipt_run_id,
    ]
    .into_iter()
    .flatten()
    .all(|run_id| run_id == latest_run_graph_status_run_id)
}

pub(crate) fn default_run_graph_lane_status() -> String {
    LaneStatus::LaneOpen.as_str().to_string()
}

pub(crate) fn normalize_run_graph_lane_status(
    value: Option<&str>,
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> String {
    let derived_lane_status = derive_lane_status(
        dispatch_status,
        supersedes_receipt_id,
        exception_path_receipt_id,
    )
    .as_str()
    .to_string();
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let canonical_lane_status = canonical_lane_status_str(raw).unwrap_or(raw).trim();
            if dispatch_status.trim() == "executed" && canonical_lane_status == "lane_completed" {
                return canonical_lane_status.to_string();
            }
            if canonical_lane_status == derived_lane_status {
                return canonical_lane_status.to_string();
            }
            if let Some(parsed_lane_status) = LaneStatus::from_str(canonical_lane_status) {
                if lane_status_has_required_evidence(
                    parsed_lane_status,
                    supersedes_receipt_id,
                    exception_path_receipt_id,
                ) {
                    return canonical_lane_status.to_string();
                }
            }
            derived_lane_status
        }
        _ => derived_lane_status,
    }
}

pub(crate) fn downstream_dispatch_allows_completed_lane_status(
    downstream_dispatch_status: Option<&str>,
    canonical_lane_status: &str,
) -> bool {
    matches!(
        downstream_dispatch_status,
        Some("executed" | "retired_closed_task_run")
    ) && canonical_lane_status == "lane_completed"
}

pub(crate) fn deserialize_run_graph_lane_status<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <Option<String> as serde::Deserialize>::deserialize(deserializer)?;
    match value.as_deref() {
        Some(raw) if !raw.trim().is_empty() => {
            Ok(canonical_lane_status_str(raw).unwrap_or(raw).to_string())
        }
        _ => Ok(default_run_graph_lane_status()),
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphGateSummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphGateSummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            context_state: status.context_state,
            delegation_gate,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} gate={} handoff={} context={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.policy_gate,
            self.handoff_state,
            self.context_state,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, Clone)]
struct CurrentSessionRunGraphClaimScope {
    run_ids: Vec<String>,
    task_ids: Vec<String>,
}

impl CurrentSessionRunGraphClaimScope {
    fn is_empty(&self) -> bool {
        self.run_ids.is_empty() && self.task_ids.is_empty()
    }

    fn push_run_id(&mut self, run_id: String) {
        let run_id = run_id.trim().to_string();
        if !run_id.is_empty() && !self.run_ids.contains(&run_id) {
            self.run_ids.push(run_id);
        }
    }

    fn push_task_id(&mut self, task_id: String) {
        let task_id = task_id.trim().to_string();
        if !task_id.is_empty() && !self.task_ids.contains(&task_id) {
            self.task_ids.push(task_id);
        }
    }

    fn contains_run_id(&self, run_id: &str) -> bool {
        let run_id = run_id.trim();
        !run_id.is_empty() && self.run_ids.iter().any(|value| value == run_id)
    }

    fn contains_task_id(&self, task_id: &str) -> bool {
        let task_id = task_id.trim();
        !task_id.is_empty() && self.task_ids.iter().any(|value| value == task_id)
    }

    fn matches_binding(&self, binding: &RunGraphContinuationBinding) -> bool {
        self.run_ids.contains(&binding.run_id)
            || self.task_ids.contains(&binding.task_id)
            || binding
                .active_bounded_unit
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|run_id| self.run_ids.iter().any(|value| value == run_id))
            || binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|task_id| self.task_ids.iter().any(|value| value == task_id))
    }
}

impl StateStore {
    fn run_graph_owner_evidence_record_id(run_id: &str, artifact_kind: &str) -> String {
        sanitize_record_id(&format!("{run_id}::{artifact_kind}"))
    }

    fn current_runtime_owner_evidence(&self) -> Result<serde_json::Value, StateStoreError> {
        static OWNER_EVIDENCE_CACHE: std::sync::OnceLock<
            std::sync::Mutex<
                Option<(
                    std::path::PathBuf,
                    Option<String>,
                    std::time::Instant,
                    serde_json::Value,
                )>,
            >,
        > = std::sync::OnceLock::new();

        let root = self.root().to_path_buf();
        let session_id = std::env::var("VIDA_SESSION_ID").ok();
        let cache = OWNER_EVIDENCE_CACHE.get_or_init(|| std::sync::Mutex::new(None));
        if let Ok(guard) = cache.lock() {
            if let Some((cached_root, cached_session_id, cached_at, evidence)) = guard.as_ref() {
                if cached_root == &root
                    && cached_session_id == &session_id
                    && cached_at.elapsed() < std::time::Duration::from_secs(10)
                {
                    return Ok(evidence.clone());
                }
            }
        }

        let evidence =
            crate::orchestrator_session_surface::build_runtime_owner_evidence(self.root(), true)
                .map_err(|reason| StateStoreError::InvalidTaskRecord {
                    reason: format!("runtime owner evidence unavailable: {reason}"),
                })?;

        if let Ok(mut guard) = cache.lock() {
            *guard = Some((
                root,
                session_id,
                std::time::Instant::now(),
                evidence.clone(),
            ));
        }
        Ok(evidence)
    }

    async fn current_session_run_graph_claim_scope(
        &self,
    ) -> Result<Option<CurrentSessionRunGraphClaimScope>, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        let Some(current_session_id) = evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            return Ok(None);
        };
        let current_stable_fallback = evidence["current_session"]
            ["fallback_replaces_legacy_stable_worktree_state_hash"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut scope = CurrentSessionRunGraphClaimScope {
            run_ids: Vec::new(),
            task_ids: Vec::new(),
        };
        for claim in self.active_orchestrator_claims().await? {
            if claim.orchestrator_session_id != current_session_id {
                continue;
            }
            if let Some(run_id) = claim.run_id {
                scope.push_run_id(run_id);
            }
            if let Some(task_id) = claim.task_id {
                scope.push_task_id(task_id);
            }
        }
        let scoped_task_ids = scope.task_ids.clone();
        for task_id in scoped_task_ids {
            let mut run_query = self
                .db
                .query(
                    "SELECT run_id FROM execution_plan_state \
                     WHERE task_id = $task_id \
                     ORDER BY run_id DESC;",
                )
                .bind(("task_id", task_id))
                .await?;
            let rows: Vec<serde_json::Value> = run_query.take(0)?;
            for row in rows {
                if let Some(run_id) = row
                    .get("run_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    scope.push_run_id(run_id.to_string());
                }
            }
        }
        let mut binding_query = self
            .db
            .query(
                "SELECT * FROM run_graph_continuation_binding \
                 WHERE binding_source = 'explicit_continuation_bind' \
                    OR binding_source = 'explicit_continuation_bind_task' \
                    OR binding_source = 'task_close_reconcile' \
                 ORDER BY recorded_at DESC, run_id DESC;",
            )
            .await?;
        let rows: Vec<RunGraphContinuationBinding> = binding_query.take(0)?;
        for binding in rows {
            if !scope.matches_binding(&binding) {
                continue;
            }
            scope.push_run_id(binding.run_id.clone());
            scope.push_task_id(binding.task_id.clone());
            if let Some(run_id) = binding
                .active_bounded_unit
                .get("run_id")
                .and_then(serde_json::Value::as_str)
            {
                scope.push_run_id(run_id.to_string());
            }
            if let Some(task_id) = binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
            {
                scope.push_task_id(task_id.to_string());
            }
        }

        if scope.is_empty() {
            let mut owner_query = self
                .db
                .query(
                    "SELECT * FROM run_graph_owner_evidence \
                     WHERE runtime_owner_evidence.current_session.session_id = $session_id \
                        OR runtime_owner_evidence.current_session.fallback_replaces_legacy_stable_worktree_state_hash = $stable_fallback \
                     ORDER BY recorded_at DESC, run_id DESC;",
                )
                .bind(("session_id", current_session_id.clone()))
                .bind((
                    "stable_fallback",
                    current_stable_fallback.clone().unwrap_or_default(),
                ))
                .await?;
            let owner_records: Vec<RunGraphOwnerEvidenceRecord> = owner_query.take(0)?;
            for record in owner_records {
                if Self::owner_evidence_matches_current_session(
                    &record.runtime_owner_evidence,
                    &evidence,
                    current_session_id.as_str(),
                    current_stable_fallback.as_deref(),
                ) {
                    scope.push_run_id(record.run_id);
                }
            }
        }
        if scope.is_empty() {
            Ok(None)
        } else {
            Ok(Some(scope))
        }
    }

    fn ensure_runtime_owner_mutation_allowed(
        evidence: &serde_json::Value,
    ) -> Result<(), StateStoreError> {
        if evidence["mutation_gate"] == "blocked_live_other_orchestrator" {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "runtime owner evidence blocks run-graph mutation: live_other_orchestrator_owner"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn owner_evidence_matches_current_session(
        runtime_owner_evidence: &serde_json::Value,
        current_runtime_owner_evidence: &serde_json::Value,
        current_session_id: &str,
        current_stable_fallback: Option<&str>,
    ) -> bool {
        let current_session = &runtime_owner_evidence["current_session"];
        current_session["session_id"]
            .as_str()
            .is_some_and(|session_id| session_id.trim() == current_session_id)
            || current_stable_fallback.is_some_and(|fallback| {
                let fallback = fallback.trim();
                current_session["session_id"]
                    .as_str()
                    .is_some_and(|session_id| !fallback.is_empty() && session_id.trim() == fallback)
                    || current_session["fallback_replaces_legacy_stable_worktree_state_hash"]
                        .as_str()
                        .is_some_and(|record_fallback| {
                            !fallback.is_empty() && record_fallback.trim() == fallback
                        })
            })
            || Self::legacy_same_worktree_owner_evidence_matches_current_session(
                runtime_owner_evidence,
                current_runtime_owner_evidence,
            )
    }

    fn legacy_same_worktree_owner_evidence_matches_current_session(
        runtime_owner_evidence: &serde_json::Value,
        current_runtime_owner_evidence: &serde_json::Value,
    ) -> bool {
        if current_runtime_owner_evidence["mutation_gate"] != "current_session_allowed" {
            return false;
        }
        if current_runtime_owner_evidence["live_other_sessions"]
            .as_array()
            .is_some_and(|sessions| !sessions.is_empty())
        {
            return false;
        }
        let recorded_session = &runtime_owner_evidence["current_session"];
        let recorded_identity_source = recorded_session["identity_source"]
            .as_str()
            .map(str::trim)
            .unwrap_or_default();
        if !matches!(
            recorded_identity_source,
            "stable_local_worktree_session_id"
                | "generated_local_session_token"
                | "synthesized_local_session_token"
                | "CODEX_SESSION_ID"
                | "CODEX_THREAD_ID"
        ) {
            return false;
        }
        let current_session = &current_runtime_owner_evidence["current_session"];
        Self::owner_path_field_matches(recorded_session, current_session, "worktree_environment_id")
            || Self::owner_path_field_matches(recorded_session, current_session, "project_root")
    }

    fn owner_path_field_matches(
        recorded_session: &serde_json::Value,
        current_session: &serde_json::Value,
        field: &str,
    ) -> bool {
        let Some(recorded) = recorded_session[field].as_str() else {
            return false;
        };
        let Some(current) = current_session[field].as_str() else {
            return false;
        };
        let recorded = Self::normalize_owner_path(recorded);
        let current = Self::normalize_owner_path(current);
        !recorded.is_empty() && recorded == current
    }

    fn normalize_owner_path(value: &str) -> String {
        value
            .trim()
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .to_ascii_lowercase()
    }

    async fn ensure_current_session_mutation_claim_for_run(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        let current_session_id = evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: "run-graph mutation requires an active current session id".to_string(),
            })?;
        let current_stable_fallback = evidence["current_session"]
            ["fallback_replaces_legacy_stable_worktree_state_hash"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let active_claims = self.active_orchestrator_claims().await?;
        let run_task_id = self
            .run_graph_task_id_for_mutation_claim(run_id)
            .await?
            .map(|task_id| task_id.trim().to_string())
            .filter(|task_id| !task_id.is_empty());
        for claim in active_claims {
            if claim.orchestrator_session_id == current_session_id
                && claim
                    .run_id
                    .as_deref()
                    .is_some_and(|claim_run_id| claim_run_id.trim() == run_id)
            {
                return Ok(());
            }
            if claim.orchestrator_session_id == current_session_id
                && run_task_id.as_deref().is_some_and(|task_id| {
                    claim
                        .task_id
                        .as_deref()
                        .is_some_and(|claim_task_id| claim_task_id.trim() == task_id)
                })
            {
                return Ok(());
            }
        }
        if let Some(scope) = self.current_session_run_graph_claim_scope().await? {
            if scope.contains_run_id(run_id)
                || scope.contains_task_id(run_id)
                || run_task_id
                    .as_deref()
                    .is_some_and(|task_id| scope.contains_task_id(task_id))
            {
                return Ok(());
            }
        }

        if self.run_graph_legacy_ownerless(run_id).await? {
            return Ok(());
        }

        let mut owner_query = self
            .db
            .query(
                "SELECT * FROM run_graph_owner_evidence \
                 WHERE run_id = $run_id \
                 ORDER BY recorded_at DESC, artifact_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let owner_records: Vec<RunGraphOwnerEvidenceRecord> = owner_query.take(0)?;
        if owner_records.iter().any(|record| {
            Self::owner_evidence_matches_current_session(
                &record.runtime_owner_evidence,
                &evidence,
                current_session_id,
                current_stable_fallback,
            )
        }) {
            return Ok(());
        }

        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph mutation blocked: current session does not own run `{run_id}`"
            ),
        })
    }

    pub(crate) async fn current_session_can_mutate_run_graph_run(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        match self
            .ensure_current_session_mutation_claim_for_run(run_id)
            .await
        {
            Ok(()) => Ok(true),
            Err(StateStoreError::InvalidTaskRecord { reason })
                if reason.contains("current session does not own run") =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    async fn run_graph_task_id_for_mutation_claim(
        &self,
        run_id: &str,
    ) -> Result<Option<String>, StateStoreError> {
        let execution: Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?;
        if let Some(execution) = execution {
            return Ok(Some(execution.task_id));
        }
        Ok(self
            .run_graph_dispatch_context(run_id)
            .await?
            .map(|context| context.task_id))
    }

    async fn record_run_graph_owner_evidence(
        &self,
        run_id: &str,
        artifact_kind: &str,
    ) -> Result<(), StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        Self::ensure_runtime_owner_mutation_allowed(&evidence)?;
        self.ensure_current_session_mutation_claim_for_run(run_id)
            .await?;
        let artifact_id = Self::run_graph_owner_evidence_record_id(run_id, artifact_kind);
        let record = RunGraphOwnerEvidenceRecord {
            run_id: run_id.to_string(),
            artifact_kind: artifact_kind.to_string(),
            artifact_id: artifact_id.clone(),
            runtime_owner_evidence: evidence,
            recorded_at: unix_timestamp().to_string(),
        };
        let _: Option<RunGraphOwnerEvidenceRecord> = self
            .db
            .upsert(("run_graph_owner_evidence", artifact_id.as_str()))
            .content(record)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn run_graph_owner_evidence_record(
        &self,
        run_id: &str,
        artifact_kind: &str,
    ) -> Result<Option<RunGraphOwnerEvidenceRecord>, StateStoreError> {
        self.db
            .select((
                "run_graph_owner_evidence",
                Self::run_graph_owner_evidence_record_id(run_id, artifact_kind).as_str(),
            ))
            .await
            .map_err(StateStoreError::from)
    }

    pub(crate) async fn run_graph_legacy_ownerless(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let mut owner_query = self
            .db
            .query(
                "SELECT artifact_id FROM run_graph_owner_evidence WHERE run_id = $run_id LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let owner_rows: Vec<serde_json::Value> = owner_query.take(0)?;
        if !owner_rows.is_empty() {
            return Ok(false);
        }

        let mut claim_query = self
            .db
            .query(
                "SELECT claim_id FROM orchestrator_claim \
                 WHERE run_id = $run_id \
                 AND status IN ['active', 'renewed', 'blocked'] \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let claim_rows: Vec<serde_json::Value> = claim_query.take(0)?;
        Ok(claim_rows.is_empty())
    }

    pub async fn run_graph_summary(&self) -> Result<RunGraphSummary, StateStoreError> {
        Ok(RunGraphSummary {
            execution_plan_count: self.count_table_rows("execution_plan_state").await?,
            routed_run_count: self.count_table_rows("routed_run_state").await?,
            governance_count: self.count_table_rows("governance_state").await?,
            resumability_count: self.count_table_rows("resumability_capsule").await?,
            reconciliation_count: self.count_table_rows("task_reconciliation_summary").await?,
        })
    }

    pub async fn record_run_graph_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        status.validate_memory_governance()?;
        self.ensure_current_session_mutation_claim_for_run(&status.run_id)
            .await?;
        let updated_at = unix_timestamp_nanos().to_string();
        let receipt_recorded_at = updated_at.clone();
        let checkpoint_record_updated_at = updated_at.clone();
        let _: Option<RoutedRunStateRow> = self
            .db
            .upsert(("routed_run_state", status.run_id.as_str()))
            .content(RoutedRunStateRow {
                run_id: status.run_id.clone(),
                route_task_class: status.route_task_class.clone(),
                selected_backend: status.selected_backend.clone(),
                lane_id: status.lane_id.clone(),
                lifecycle_stage: status.lifecycle_stage.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<GovernanceStateRow> = self
            .db
            .upsert(("governance_state", status.run_id.as_str()))
            .content(GovernanceStateRow {
                run_id: status.run_id.clone(),
                policy_gate: status.policy_gate.clone(),
                handoff_state: status.handoff_state.clone(),
                context_state: status.context_state.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<ResumabilityCapsuleRow> = self
            .db
            .upsert(("resumability_capsule", status.run_id.as_str()))
            .content(ResumabilityCapsuleRow {
                run_id: status.run_id.clone(),
                checkpoint_kind: status.checkpoint_kind.clone(),
                resume_target: status.resume_target.clone(),
                recovery_ready: status.recovery_ready,
                updated_at,
            })
            .await?;
        let _: Option<ExecutionPlanStateRow> = self
            .db
            .upsert(("execution_plan_state", status.run_id.as_str()))
            .content(ExecutionPlanStateRow {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_class: status.task_class.clone(),
                active_node: status.active_node.clone(),
                next_node: status.next_node.clone(),
                status: status.status.clone(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;
        if status.checkpoint_kind.trim().eq_ignore_ascii_case("none") {
            self.clear_run_graph_projection_checkpoint_records(&status.run_id)
                .await?;
        } else {
            let checkpoint_record = RunGraphProjectionCheckpointRecord::from_status(
                status,
                checkpoint_record_updated_at,
            );
            self.record_run_graph_projection_checkpoint_record(&checkpoint_record)
                .await?;
        }
        if let Some(transition_kind) = approval_delegation_transition_kind(status) {
            let receipt = RunGraphApprovalDelegationReceipt::from_status(
                status,
                transition_kind,
                receipt_recorded_at,
            );
            self.record_run_graph_approval_delegation_receipt(&receipt)
                .await?;
        }
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_dispatch_receipt(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        if receipt.dispatch_status != "routed" {
            clear_run_graph_dispatch_init_fast_cache(self.root(), &receipt.run_id);
        }
        self.record_run_graph_owner_evidence(&receipt.run_id, "dispatch_receipt")
            .await?;
        let receipt: RunGraphDispatchReceiptStored = receipt.clone().into();
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .upsert(("run_graph_dispatch_receipt", receipt.run_id.as_str()))
            .content(receipt)
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn clear_run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .delete(("run_graph_dispatch_receipt", run_id))
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_continuation_binding(
        &self,
        binding: &RunGraphContinuationBinding,
    ) -> Result<(), StateStoreError> {
        binding.validate()?;
        self.record_run_graph_owner_evidence(&binding.run_id, "continuation_binding")
            .await?;
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .upsert(("run_graph_continuation_binding", binding.run_id.as_str()))
            .content(binding.clone())
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    async fn effective_exception_takeover_continuation_binding(
        &self,
        binding: RunGraphContinuationBinding,
    ) -> Result<RunGraphContinuationBinding, StateStoreError> {
        let receipt: Option<RunGraphDispatchReceiptStored> = self
            .db
            .select(("run_graph_dispatch_receipt", binding.run_id.as_str()))
            .await?;
        Ok(
            reconcile_continuation_binding_with_exception_takeover_receipt(
                binding,
                receipt.as_ref(),
            ),
        )
    }

    pub async fn run_graph_continuation_binding(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let binding: Option<RunGraphContinuationBinding> = self
            .db
            .select(("run_graph_continuation_binding", run_id))
            .await?;
        match binding {
            Some(binding) => {
                let Some(binding) = self.normalize_task_close_reconcile_binding(binding).await?
                else {
                    return Ok(None);
                };
                let binding = self
                    .effective_exception_takeover_continuation_binding(binding)
                    .await?;
                if self
                    .continuation_binding_points_to_terminal_task_graph_task(&binding)
                    .await?
                {
                    self.clear_run_graph_continuation_binding(&binding.run_id)
                        .await?;
                    return Ok(None);
                }
                binding.validate()?;
                Ok(Some(binding))
            }
            None => Ok(None),
        }
    }

    pub async fn record_run_graph_replay_lineage_receipt(
        &self,
        receipt: &RunGraphReplayLineageReceipt,
    ) -> Result<(), StateStoreError> {
        receipt.validate()?;
        let _: Option<RunGraphReplayLineageReceipt> = self
            .db
            .upsert((
                "run_graph_replay_lineage_receipt",
                receipt.receipt_id.as_str(),
            ))
            .content(receipt.clone())
            .await?;
        Ok(())
    }

    pub async fn record_run_graph_projection_checkpoint_record(
        &self,
        record: &RunGraphProjectionCheckpointRecord,
    ) -> Result<(), StateStoreError> {
        record.validate()?;
        let _: Option<RunGraphProjectionCheckpointRecord> = self
            .db
            .upsert((
                "run_graph_projection_checkpoint_record",
                record.record_id.as_str(),
            ))
            .content(record.clone())
            .await?;
        Ok(())
    }

    pub async fn clear_run_graph_projection_checkpoint_records(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query(format!(
                "DELETE run_graph_projection_checkpoint_record WHERE run_id = '{}';",
                escape_surql_literal(run_id)
            ))
            .await?;
        Ok(())
    }

    pub async fn run_graph_projection_checkpoint_record(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphProjectionCheckpointRecord>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_projection_checkpoint_record \
                 WHERE run_id = $run_id \
                 ORDER BY updated_at DESC, record_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<RunGraphProjectionCheckpointRecord> = query.take(0)?;
        match rows.into_iter().next() {
            Some(record) => {
                record.validate()?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn latest_run_graph_projection_checkpoint_record(
        &self,
    ) -> Result<Option<RunGraphProjectionCheckpointRecord>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        match self
            .run_graph_projection_checkpoint_record(&status.run_id)
            .await?
        {
            Some(record) if record.run_id == status.run_id => Ok(Some(record)),
            Some(record) => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph projection checkpoint record is inconsistent for `{}`: latest projection checkpoint record run_id must share the same run_id (record_run_id={})",
                    status.run_id, record.run_id
                ),
            }),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn run_graph_replay_lineage_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphReplayLineageReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_replay_lineage_receipt \
                 WHERE run_id = $run_id \
                 ORDER BY recorded_at DESC, receipt_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<RunGraphReplayLineageReceipt> = query.take(0)?;
        match rows.into_iter().next() {
            Some(receipt) => {
                receipt.validate()?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn latest_run_graph_replay_lineage_receipt(
        &self,
    ) -> Result<Option<RunGraphReplayLineageReceipt>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        match self
            .run_graph_replay_lineage_receipt(&status.run_id)
            .await?
        {
            Some(receipt) if receipt.run_id == status.run_id => Ok(Some(receipt)),
            Some(receipt) => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph replay lineage receipt is inconsistent for `{}`: latest replay lineage receipt run_id must share the same run_id (receipt_run_id={})",
                    status.run_id, receipt.run_id
                ),
            }),
            None => Ok(None),
        }
    }

    pub async fn latest_explicit_run_graph_continuation_binding(
        &self,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        self.latest_explicit_run_graph_continuation_binding_matching(|_| true)
            .await
    }

    pub async fn latest_explicit_run_graph_continuation_binding_for_current_session(
        &self,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let Some(scope) = self.current_session_run_graph_claim_scope().await? else {
            return Ok(None);
        };
        self.latest_explicit_run_graph_continuation_binding_matching(|binding| {
            scope.matches_binding(binding)
        })
        .await
    }

    async fn latest_explicit_run_graph_continuation_binding_matching(
        &self,
        mut matches_scope: impl FnMut(&RunGraphContinuationBinding) -> bool,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_continuation_binding \
                 WHERE binding_source = 'explicit_continuation_bind' \
                    OR binding_source = 'explicit_continuation_bind_task' \
                    OR binding_source = 'task_close_reconcile' \
                 ORDER BY recorded_at DESC, run_id DESC;",
            )
            .await?;
        let rows: Vec<RunGraphContinuationBinding> = query.take(0)?;
        for binding in rows {
            let Some(mut binding) = self.normalize_task_close_reconcile_binding(binding).await?
            else {
                continue;
            };
            binding = self
                .effective_exception_takeover_continuation_binding(binding)
                .await?;
            binding.validate()?;
            if !matches_scope(&binding) {
                continue;
            }
            if binding.binding_source != "explicit_continuation_bind_task" {
                return Ok(Some(binding));
            }

            let Some(task_id) = binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            let task = match self.show_task(task_id).await {
                Ok(task) => task,
                Err(StateStoreError::MissingTask { .. }) => return Ok(Some(binding)),
                Err(error) => return Err(error),
            };
            if task_status_is_terminal_for_continuation(&task.status) {
                continue;
            }

            if let Some(active_bounded_unit) = binding.active_bounded_unit.as_object_mut() {
                active_bounded_unit.insert(
                    "task_status".to_string(),
                    serde_json::Value::String(task.status.clone()),
                );
                active_bounded_unit.insert(
                    "issue_type".to_string(),
                    serde_json::Value::String(task.issue_type.clone()),
                );
            }
            binding.validate()?;
            return Ok(Some(binding));
        }
        Ok(None)
    }

    async fn continuation_binding_points_to_terminal_task_graph_task(
        &self,
        binding: &RunGraphContinuationBinding,
    ) -> Result<bool, StateStoreError> {
        if binding.status != "bound"
            || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
        {
            return Ok(false);
        }
        let task_id = binding.active_bounded_unit["task_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(binding.task_id.as_str());
        let task = match self.show_task(task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        Ok(task_status_is_terminal_for_continuation(&task.status))
    }

    async fn normalize_task_close_reconcile_binding(
        &self,
        mut binding: RunGraphContinuationBinding,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        if binding.binding_source != "task_close_reconcile" {
            return Ok(Some(binding));
        }

        let binding_kind = binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str);
        if binding_kind != Some("run_graph_task") {
            return Ok(Some(binding));
        }

        let task = match self.show_task(&binding.task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => {
                self.clear_run_graph_continuation_binding(&binding.run_id)
                    .await?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if !task_status_is_terminal_for_continuation(&task.status) {
            return Ok(Some(binding));
        }
        if !self
            .task_close_reconcile_has_persisted_receipt_truth(&binding.run_id, &binding.task_id)
            .await?
        {
            self.clear_run_graph_continuation_binding(&binding.run_id)
                .await?;
            return Ok(None);
        }

        binding.active_bounded_unit = serde_json::json!({
            "kind": "downstream_dispatch_target",
            "task_id": binding.task_id,
            "run_id": binding.run_id,
            "dispatch_target": "closure",
        });
        binding.why_this_unit = "Closing the active task reconciled the run into a completed state and bound downstream closure as the next lawful bounded unit.".to_string();
        binding.sequential_vs_parallel_posture = "sequential_only".to_string();
        self.record_run_graph_continuation_binding(&binding).await?;
        Ok(Some(binding))
    }

    pub async fn clear_run_graph_continuation_binding(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .delete(("run_graph_continuation_binding", run_id))
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_dispatch_context(
        &self,
        context: &RunGraphDispatchContext,
    ) -> Result<(), StateStoreError> {
        context.validate()?;
        self.record_run_graph_owner_evidence(&context.run_id, "dispatch_context")
            .await?;
        let _: Option<RunGraphDispatchContext> = self
            .db
            .upsert(("run_graph_dispatch_context", context.run_id.as_str()))
            .content(context.clone())
            .await?;
        Ok(())
    }

    pub async fn run_graph_dispatch_context(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchContext>, StateStoreError> {
        let context: Option<RunGraphDispatchContext> = self
            .db
            .select(("run_graph_dispatch_context", run_id))
            .await?;
        match context {
            Some(context) => {
                context.validate()?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    pub async fn run_graph_status(&self, run_id: &str) -> Result<RunGraphStatus, StateStoreError> {
        self.run_graph_status_from_task_rows(run_id, &[]).await
    }

    async fn normalize_spec_first_work_pool_handoff_receipt_truth(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        if !receipt_has_pending_spec_first_work_pool_handoff_gate(receipt) {
            return Ok(false);
        }
        let tasks = self.list_tasks(None, true).await?;
        let identity = self
            .run_graph_dispatch_task_identity(&receipt.run_id)
            .await?;
        let feature_id = identity
            .as_ref()
            .and_then(|identity| identity.feature_epic_id.as_deref())
            .and_then(|feature_id| {
                Self::spec_first_work_pool_handoff_gate_satisfied_for_task(&tasks, feature_id)
            })
            .or_else(|| {
                Self::spec_first_work_pool_handoff_gate_satisfied_for_task(&tasks, &receipt.run_id)
            });
        let Some(feature_id) = feature_id else {
            return Ok(false);
        };
        let identity = Self::spec_first_dispatch_task_identity_from_tasks(
            &tasks,
            &receipt.run_id,
            &feature_id,
            identity
                .as_ref()
                .map(|_| "spec_first_work_pool_handoff_identity_reconciliation")
                .unwrap_or("spec_first_work_pool_handoff_reconciliation"),
        );
        self.record_run_graph_dispatch_task_identity(&identity)
            .await?;
        self.repair_stale_spec_first_parent_auto_close_for_work_pool_handoff(&tasks, &feature_id)
            .await?;

        receipt.downstream_dispatch_blockers.retain(|blocker| {
            !matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
        receipt.downstream_dispatch_ready = receipt.downstream_dispatch_blockers.is_empty();
        receipt.downstream_dispatch_status = Some(if receipt.downstream_dispatch_ready {
            "packet_ready".to_string()
        } else {
            "blocked".to_string()
        });
        receipt.downstream_dispatch_note = Some(format!(
            "spec-first feature `{feature_id}` has a closed spec-pack task; continue with work-pool handoff"
        ));
        receipt.downstream_dispatch_active_target = Some("specification".to_string());
        receipt.downstream_dispatch_last_target = Some("specification".to_string());
        Ok(true)
    }

    async fn normalize_materialized_pack_task_identity_and_receipt(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        if !matches!(receipt.dispatch_status.as_str(), "blocked" | "executed")
            || receipt.dispatch_surface.as_deref() != Some("vida task ensure")
            || !matches!(
                receipt.dispatch_target.as_str(),
                "work-pool-pack" | "dev-pack"
            )
            || !receipt
                .blocker_code
                .as_deref()
                .is_none_or(|code| code == "internal_activation_view_only")
        {
            return Ok(false);
        }
        let expected_packet_key = match receipt.dispatch_target.as_str() {
            "work-pool-pack" => "work_pool_task",
            "dev-pack" => "dev_task",
            _ => return Ok(false),
        };
        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt.clone());
        let Some(result) = tracked_flow_materialization_result(Some(self.root()), &stored_receipt)
        else {
            return Ok(false);
        };
        if result.packet_key != expected_packet_key {
            return Ok(false);
        }
        let tasks = self.list_tasks(None, true).await?;
        let materialized_task = match receipt.dispatch_target.as_str() {
            "work-pool-pack" => tasks
                .iter()
                .find(|task| task.id == result.task_id && task_is_work_pool_pack_child(task)),
            "dev-pack" => tasks
                .iter()
                .find(|task| task.id == result.task_id && task_has_label(task, "dev-pack")),
            _ => None,
        };
        let Some(materialized_task) = materialized_task else {
            return Ok(false);
        };
        let Some(feature_id) = Self::parent_id_for_task(materialized_task) else {
            return Ok(false);
        };
        if result
            .epic_task_id
            .as_deref()
            .is_some_and(|epic_task_id| epic_task_id != feature_id)
        {
            return Ok(false);
        }
        let child_tasks = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(feature_id.as_str()))
            .collect::<Vec<_>>();
        let spec_task_id = child_tasks
            .iter()
            .filter(|task| task_is_spec_pack_child(task))
            .map(|task| task.id.clone())
            .min();
        let identity = RunGraphDispatchTaskIdentity {
            run_id: receipt.run_id.clone(),
            feature_epic_id: Some(feature_id),
            spec_task_id,
            work_pool_task_id: if receipt.dispatch_target == "work-pool-pack" {
                Some(result.task_id.clone())
            } else {
                child_tasks
                    .iter()
                    .filter(|task| task_is_work_pool_pack_child(task))
                    .map(|task| task.id.clone())
                    .min()
            },
            dev_task_id: if receipt.dispatch_target == "dev-pack" {
                Some(result.task_id.clone())
            } else {
                child_tasks
                    .iter()
                    .filter(|task| task_has_label(task, "dev-pack"))
                    .map(|task| task.id.clone())
                    .min()
            },
            source: if receipt.dispatch_target == "work-pool-pack" {
                "work_pool_materialization_identity_reconciliation".to_string()
            } else {
                "dev_pack_materialization_identity_reconciliation".to_string()
            },
            updated_at: unix_timestamp_nanos().to_string(),
        };
        self.record_run_graph_dispatch_task_identity(&identity)
            .await?;
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.blocker_code = None;
        receipt.downstream_dispatch_target = Some(if receipt.dispatch_target == "work-pool-pack" {
            "dev-pack".to_string()
        } else {
            "closure".to_string()
        });
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        receipt.downstream_dispatch_note = Some(if receipt.dispatch_target == "work-pool-pack" {
            format!(
                "work-pool task `{}` is materialized; continue with the tracked dev packet",
                result.task_id
            )
        } else {
            format!(
                "dev task `{}` is materialized; continue with tracked flow closure",
                result.task_id
            )
        });
        receipt.downstream_dispatch_active_target = Some(receipt.dispatch_target.clone());
        receipt.downstream_dispatch_last_target = Some(receipt.dispatch_target.clone());
        if !self
            .materialize_reconciled_pack_downstream_packet(receipt)
            .await?
        {
            receipt.downstream_dispatch_ready = false;
            receipt.downstream_dispatch_status = Some("blocked".to_string());
            receipt.downstream_dispatch_blockers =
                vec!["missing_downstream_dispatch_packet".to_string()];
            receipt.downstream_dispatch_note = Some(format!(
                "materialized `{}` packet is reconciled, but no executable downstream packet could be produced",
                receipt.dispatch_target
            ));
        }
        Ok(true)
    }

    async fn materialize_reconciled_pack_downstream_packet(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        let Some(downstream_target) = receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            return Ok(false);
        };
        if receipt.dispatch_target != "work-pool-pack" {
            return Ok(true);
        }
        let Some((role_selection, run_graph_bootstrap)) =
            self.reconciled_pack_dispatch_context(receipt)?
        else {
            return Ok(false);
        };
        receipt.downstream_dispatch_command =
            crate::runtime_dispatch_state::runtime_dispatch_command_for_target(
                &role_selection,
                &downstream_target,
            );
        if let Some(packet_path) = receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            receipt.downstream_dispatch_command = Some(
                crate::runtime_dispatch_state::agent_init_command_for_packet_path(packet_path),
            );
            return Ok(true);
        }
        receipt.downstream_dispatch_packet_path =
            crate::runtime_dispatch_downstream_packets::write_runtime_downstream_dispatch_packet(
                self.root(),
                &role_selection,
                &run_graph_bootstrap,
                receipt,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        if let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() {
            receipt.downstream_dispatch_command = Some(
                crate::runtime_dispatch_state::agent_init_command_for_packet_path(packet_path),
            );
        }
        Ok(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|path| !path.is_empty()))
    }

    fn reconciled_pack_dispatch_context(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<Option<(RuntimeConsumptionLaneSelection, serde_json::Value)>, StateStoreError> {
        let Some(packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        let Some(packet_path) = self.canonical_reconciled_pack_dispatch_packet_path(packet_path)?
        else {
            return Ok(None);
        };
        let metadata = std::fs::metadata(&packet_path).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "Failed to inspect materialized pack dispatch packet `{}`: {error}",
                    packet_path.display()
                ),
            }
        })?;
        if !metadata.is_file() || metadata.len() > MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES {
            return Ok(None);
        }
        let body = std::fs::read_to_string(&packet_path).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "Failed to read materialized pack dispatch packet `{}`: {error}",
                    packet_path.display()
                ),
            }
        })?;
        let packet = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "Failed to parse materialized pack dispatch packet `{}`: {error}",
                    packet_path.display()
                ),
            }
        })?;
        let role_selection = serde_json::from_value::<RuntimeConsumptionLaneSelection>(
            packet
                .get("role_selection_full")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|error| StateStoreError::InvalidTaskRecord {
            reason: format!(
                "Failed to decode role_selection from materialized pack dispatch packet `{}`: {error}",
                packet_path.display()
            ),
        })?;
        let run_graph_bootstrap = packet
            .get("run_graph_bootstrap")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok(Some((role_selection, run_graph_bootstrap)))
    }

    fn canonical_reconciled_pack_dispatch_packet_path(
        &self,
        packet_path: &str,
    ) -> Result<Option<std::path::PathBuf>, StateStoreError> {
        let packet_path = packet_path.trim();
        if packet_path.is_empty() {
            return Ok(None);
        }
        let packet_path = std::path::PathBuf::from(packet_path);
        let candidate = if packet_path.is_absolute() {
            packet_path
        } else {
            self.root().join(packet_path)
        };
        let root = std::fs::canonicalize(self.root()).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "Failed to canonicalize VIDA state root `{}` while loading materialized pack dispatch packet: {error}",
                    self.root().display()
                ),
            }
        })?;
        let candidate = match std::fs::canonicalize(&candidate) {
            Ok(candidate) => candidate,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "Failed to canonicalize materialized pack dispatch packet `{}`: {error}",
                        candidate.display()
                    ),
                });
            }
        };
        if !candidate.starts_with(&root) {
            return Ok(None);
        }
        Ok(Some(candidate))
    }

    fn spec_first_dispatch_task_identity_from_tasks(
        tasks: &[TaskRecord],
        run_id: &str,
        feature_id: &str,
        source: &str,
    ) -> RunGraphDispatchTaskIdentity {
        let child_tasks = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(feature_id))
            .collect::<Vec<_>>();
        let spec_task_id = child_tasks
            .iter()
            .filter(|task| task_is_spec_pack_child(task))
            .map(|task| task.id.clone())
            .min();
        let work_pool_task_id = child_tasks
            .iter()
            .filter(|task| task_is_work_pool_pack_child(task))
            .map(|task| task.id.clone())
            .min();

        RunGraphDispatchTaskIdentity {
            run_id: run_id.to_string(),
            feature_epic_id: Some(feature_id.to_string()),
            spec_task_id,
            work_pool_task_id,
            dev_task_id: child_tasks
                .iter()
                .filter(|task| task_has_label(task, "dev-pack"))
                .map(|task| task.id.clone())
                .min(),
            source: source.to_string(),
            updated_at: unix_timestamp_nanos().to_string(),
        }
    }

    pub async fn record_run_graph_dispatch_task_identity(
        &self,
        identity: &RunGraphDispatchTaskIdentity,
    ) -> Result<(), StateStoreError> {
        if identity.run_id.trim().is_empty() || identity.source.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "run-graph dispatch task identity requires run_id and source".to_string(),
            });
        }
        let _: Option<RunGraphDispatchTaskIdentity> = self
            .db
            .upsert(("run_graph_dispatch_task_identity", identity.run_id.as_str()))
            .content(identity.clone())
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn run_graph_dispatch_task_identity(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchTaskIdentity>, StateStoreError> {
        let identity: Option<RunGraphDispatchTaskIdentity> = match self
            .db
            .select(("run_graph_dispatch_task_identity", run_id))
            .await
        {
            Ok(identity) => identity,
            Err(error) if error.to_string().contains("does not exist") => None,
            Err(error) => return Err(error.into()),
        };
        Ok(identity)
    }

    pub(crate) async fn run_graph_status_from_task_rows(
        &self,
        run_id: &str,
        task_rows: &[TaskRecord],
    ) -> Result<RunGraphStatus, StateStoreError> {
        let execution: Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?;
        let execution = execution.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph:{run_id}"),
        })?;
        let routed: Option<RoutedRunStateRow> =
            self.db.select(("routed_run_state", run_id)).await?;
        let routed = routed.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_route:{run_id}"),
        })?;
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let governance = governance.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_governance:{run_id}"),
        })?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        let resumability = resumability.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_resumability:{run_id}"),
        })?;

        let status = RunGraphStatus {
            run_id: execution.run_id,
            task_id: execution.task_id,
            task_class: execution.task_class,
            active_node: execution.active_node,
            next_node: execution.next_node,
            status: execution.status,
            route_task_class: routed.route_task_class,
            selected_backend: routed.selected_backend,
            lane_id: routed.lane_id,
            lifecycle_stage: routed.lifecycle_stage,
            policy_gate: governance.policy_gate,
            handoff_state: governance.handoff_state,
            context_state: governance.context_state,
            checkpoint_kind: resumability.checkpoint_kind,
            resume_target: resumability.resume_target,
            recovery_ready: resumability.recovery_ready,
        };
        let receipt = self.run_graph_dispatch_receipt_stored(run_id).await?;
        let status = reconcile_run_graph_status_with_dispatch_receipt(status, receipt.as_ref())?;
        let task = if task_rows.is_empty() {
            self.show_task(&status.task_id).await.ok()
        } else {
            task_rows
                .iter()
                .find(|task| task.id == status.task_id)
                .cloned()
        };
        let status =
            reconcile_run_graph_status_with_closed_task(status, task.as_ref(), receipt.as_ref());
        status.validate_memory_governance()?;
        Ok(status)
    }

    pub async fn record_run_graph_approval_delegation_receipt(
        &self,
        receipt: &RunGraphApprovalDelegationReceipt,
    ) -> Result<(), StateStoreError> {
        let receipt = receipt.clone();
        ensure_run_graph_approval_delegation_receipt_consistency(&receipt)?;
        let _: Option<RunGraphApprovalDelegationReceipt> = self
            .db
            .upsert((
                "run_graph_approval_delegation_receipt",
                receipt.run_id.as_str(),
            ))
            .content(receipt)
            .await?;
        Ok(())
    }

    pub async fn run_graph_approval_delegation_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphApprovalDelegationReceipt>, StateStoreError> {
        let receipt: Option<RunGraphApprovalDelegationReceipt> = self
            .db
            .select(("run_graph_approval_delegation_receipt", run_id))
            .await?;
        Ok(match receipt {
            Some(receipt) => Some(
                ensure_run_graph_approval_delegation_receipt_consistency(&receipt)
                    .map(|()| receipt)?,
            ),
            None => None,
        })
    }

    pub async fn latest_run_graph_status(&self) -> Result<Option<RunGraphStatus>, StateStoreError> {
        self.latest_run_graph_status_from_task_rows(&[]).await
    }

    pub async fn latest_run_graph_status_for_current_session(
        &self,
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let Some(scope) = self.current_session_run_graph_claim_scope().await? else {
            return Ok(None);
        };
        let mut seen_run_ids = std::collections::BTreeSet::new();
        for run_id in scope.run_ids {
            if !seen_run_ids.insert(run_id.clone()) {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_lane(&run_id)
                .await?
            {
                continue;
            }
            match self.run_graph_status_from_task_rows(&run_id, &[]).await {
                Ok(status) => {
                    if self
                        .run_graph_status_points_to_terminal_task_active(&status)
                        .await?
                    {
                        continue;
                    }
                    return Ok(Some(status));
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        for task_id in scope.task_ids {
            let Some(run_id) = self.latest_run_graph_run_id_for_task(&task_id).await? else {
                continue;
            };
            if !seen_run_ids.insert(run_id.clone()) {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_lane(&run_id)
                .await?
            {
                continue;
            }
            match self.run_graph_status_from_task_rows(&run_id, &[]).await {
                Ok(status) => {
                    if self
                        .run_graph_status_points_to_terminal_task_active(&status)
                        .await?
                    {
                        continue;
                    }
                    return Ok(Some(status));
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub(crate) async fn latest_run_graph_status_from_task_rows(
        &self,
        task_rows: &[TaskRecord],
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let Some(run_id) = self
            .latest_run_graph_run_id_from_task_rows(task_rows)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            self.run_graph_status_from_task_rows(&run_id, task_rows)
                .await?,
        ))
    }

    pub(crate) async fn latest_run_graph_run_id(&self) -> Result<Option<String>, StateStoreError> {
        self.latest_run_graph_run_id_from_task_rows(&[]).await
    }

    pub(crate) async fn latest_run_graph_run_id_from_task_rows(
        &self,
        _task_rows: &[TaskRecord],
    ) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphLatestStateRow> = query.take(0)?;
        for latest in rows {
            let terminal_task_active = self
                .run_graph_latest_row_points_to_terminal_task_active(&latest)
                .await?;
            if terminal_task_active {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_lane(&latest.run_id)
                .await?
            {
                continue;
            }
            return Ok(Some(latest.run_id));
        }
        Ok(None)
    }

    pub(crate) async fn latest_terminal_task_active_run_graph_status(
        &self,
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphLatestStateRow> = query.take(0)?;
        for latest in rows {
            if !self
                .run_graph_latest_row_points_to_terminal_task_active(&latest)
                .await?
            {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_lane(&latest.run_id)
                .await?
            {
                continue;
            }
            return Ok(Some(self.run_graph_status(&latest.run_id).await?));
        }
        Ok(None)
    }

    async fn run_graph_latest_receipt_row_supersedes_lane(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(false);
        };
        Ok((receipt.lane_status.as_deref() == Some("lane_superseded")
            && has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref()))
            || stored_receipt_has_active_exception_takeover(&receipt))
    }

    async fn run_graph_latest_row_points_to_terminal_task_active(
        &self,
        latest: &RunGraphLatestStateRow,
    ) -> Result<bool, StateStoreError> {
        if self.run_graph_latest_row_points_to_terminal_task_active_from_rows(latest, &[])? {
            return Ok(true);
        }
        if latest.status == "completed" {
            return Ok(false);
        }
        let status = self.run_graph_status(&latest.run_id).await?;
        Ok(
            crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                self, &status,
            )
            .await?
            .task_closed_stale_run(),
        )
    }

    async fn run_graph_status_points_to_terminal_task_active(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        if status.status == "completed" {
            return Ok(false);
        }
        Ok(
            crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                self, status,
            )
            .await?
            .task_closed_stale_run(),
        )
    }

    fn run_graph_latest_row_points_to_terminal_task_active_from_rows(
        &self,
        latest: &RunGraphLatestStateRow,
        task_rows: &[TaskRecord],
    ) -> Result<bool, StateStoreError> {
        if latest.status == "completed" {
            return Ok(false);
        }
        if task_rows.is_empty() {
            return Ok(false);
        }
        Ok(task_rows
            .iter()
            .find(|task| task.id == latest.task_id)
            .is_some_and(|task| task_status_is_terminal_for_continuation(&task.status)))
    }

    pub(crate) async fn latest_run_graph_run_id_for_task(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM execution_plan_state WHERE task_id = $task_id ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
            .bind(("task_id", task_id.to_string()))
            .await?;
        let rows: Vec<RunGraphLatestRow> = query.take(0)?;
        Ok(rows.into_iter().next().map(|latest| latest.run_id))
    }

    async fn ensure_run_graph_recovery_surface_rows_present(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        if governance.is_none() || resumability.is_none() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/checkpoint summary is inconsistent for `{run_id}`: latest status requires both governance and resumability rows (governance_present={}, resumability_present={})",
                    governance.is_some(),
                    resumability.is_some()
                ),
            });
        }
        Ok(())
    }

    async fn ensure_run_graph_recovery_surface_has_checkpoint_lineage(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.checkpoint_kind.trim().eq_ignore_ascii_case("none") {
            return Ok(());
        }
        match self
            .run_graph_projection_checkpoint_record(&status.run_id)
            .await?
        {
            Some(record) if record.run_id == status.run_id => Ok(()),
            Some(record) => {
                self.clear_run_graph_projection_checkpoint_records(&record.run_id)
                    .await?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn ensure_run_graph_recovery_surface_consistency(
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.recovery_ready
            && status.resume_target.starts_with("dispatch.")
            && !is_dispatch_resume_handoff_complete(status)
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/gate summary is inconsistent for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
                    status.run_id,
                    status.resume_target,
                    status.next_node.as_deref().unwrap_or("none"),
                    status.policy_gate,
                    status.handoff_state
                ),
            });
        }
        Ok(())
    }

    pub async fn ensure_memory_governance_guard(&self) -> Result<(), StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(());
        };
        status.validate_memory_governance()
    }

    pub async fn latest_run_graph_dispatch_receipt_summary(
        &self,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        self.run_graph_dispatch_receipt_summary_for_status(&status)
            .await
    }

    pub async fn latest_run_graph_dispatch_receipt_summary_for_current_session(
        &self,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status_for_current_session().await? else {
            return Ok(None);
        };
        self.run_graph_dispatch_receipt_summary_for_status(&status)
            .await
    }

    pub(crate) async fn run_graph_dispatch_receipt_summary_for_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        terminal_closure_supersedes_stale_handoff_receipt(&status, &mut receipt);
        let host_runtime = crate::taskflow_task_bridge::infer_project_root_from_state_root(
            self.root(),
        )
        .map(|project_root| {
            crate::runtime_dispatch_state::runtime_host_execution_contract_for_root(&project_root)
        });
        let role_selection = self
            .run_graph_dispatch_context(&status.run_id)
            .await?
            .map(|context| context.role_selection())
            .transpose()?;
        let canonical_selected_backend = role_selection
            .as_ref()
            .and_then(|selection| {
                crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
                    selection, &receipt,
                )
            })
            .or_else(|| receipt.selected_backend.clone());
        let effective_execution_posture = {
            let mut summary = crate::runtime_dispatch_state::effective_execution_posture_summary(
                role_selection
                    .as_ref()
                    .map(|selection| &selection.execution_plan)
                    .unwrap_or(&serde_json::Value::Null),
                &receipt.dispatch_target,
                canonical_selected_backend.as_deref(),
                receipt.activation_agent_type.as_deref(),
                host_runtime.as_ref(),
                crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(&receipt),
                None,
            );
            let activation_evidence =
                crate::runtime_dispatch_state::dispatch_activation_evidence_summary(&receipt);
            if let Some(object) = summary.as_object_mut() {
                object.insert(
                    "activation_kind".to_string(),
                    activation_evidence["activation_kind"].clone(),
                );
                object.insert(
                    "execution_evidence_path".to_string(),
                    activation_evidence["execution_evidence_path"].clone(),
                );
                object.insert(
                    "receipt_backed".to_string(),
                    activation_evidence["receipt_backed"].clone(),
                );
            }
            summary
        };
        let route_policy = role_selection
            .as_ref()
            .map(|selection| {
                crate::runtime_dispatch_state::dispatch_execution_route_summary(
                    selection,
                    &receipt.dispatch_target,
                    canonical_selected_backend.as_deref(),
                    None,
                )
            })
            .unwrap_or(serde_json::Value::Null);
        let activation_evidence =
            crate::runtime_dispatch_state::dispatch_activation_evidence_summary(&receipt);
        let mut summary = RunGraphDispatchReceiptSummary::from_receipt(receipt)
            .with_effective_execution_posture(effective_execution_posture)
            .with_route_policy(route_policy)
            .with_activation_evidence(activation_evidence);
        summary.selected_backend = canonical_selected_backend;
        Ok(Some(summary))
    }

    pub async fn latest_run_graph_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        terminal_closure_supersedes_stale_handoff_receipt(&status, &mut receipt);
        Ok(Some(receipt))
    }

    pub async fn latest_active_exception_takeover_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_dispatch_receipt \
                 WHERE lane_status = 'lane_exception_takeover' \
                 ORDER BY recorded_at DESC, run_id DESC \
                 LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphDispatchReceiptStored> = query.take(0)?;
        for receipt in rows {
            if !stored_receipt_has_active_exception_takeover(&receipt) {
                continue;
            }
            let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
            let status = match self.run_graph_status(&receipt.run_id).await {
                Ok(status) => status,
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            };
            if active_exception_takeover_receipt_is_behind_status(&status, &receipt) {
                continue;
            }
            return Ok(Some(receipt.into()));
        }
        Ok(None)
    }

    pub async fn run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let status = self.run_graph_status(run_id).await.ok();
        self.run_graph_dispatch_receipt_for_status(run_id, status.as_ref())
            .await
    }

    pub(crate) async fn run_graph_dispatch_receipt_for_status(
        &self,
        run_id: &str,
        status: Option<&RunGraphStatus>,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        if let Some(status) = status {
            terminal_closure_supersedes_stale_handoff_receipt(status, &mut receipt);
        }
        Ok(Some(receipt))
    }

    async fn run_graph_dispatch_receipt_stored(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceiptStored>, StateStoreError> {
        let receipt: Option<RunGraphDispatchReceiptStored> = self
            .db
            .select(("run_graph_dispatch_receipt", run_id))
            .await?;
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(&receipt)?;
        let receipt = normalize_legacy_downstream_preview_drift(receipt);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        if crate::runtime_dispatch_state::normalize_stale_in_flight_dispatch_receipt(
            self.root(),
            &mut receipt,
        )
        .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if crate::runtime_dispatch_state::normalize_activation_view_only_receipt_truth(&mut receipt)
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if self
            .normalize_spec_first_work_pool_handoff_receipt_truth(&mut receipt)
            .await?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if self
            .normalize_materialized_pack_task_identity_and_receipt(&mut receipt)
            .await?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        self.normalize_task_close_reconcile_dispatch_receipt(receipt.into())
            .await
            .map(Some)
    }

    async fn normalize_task_close_reconcile_dispatch_receipt(
        &self,
        mut receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        let Some(binding) = self.run_graph_continuation_binding(&receipt.run_id).await? else {
            return Ok(receipt);
        };
        if binding.binding_source != "task_close_reconcile"
            || binding.active_bounded_unit["kind"].as_str() != Some("downstream_dispatch_target")
            || binding.active_bounded_unit["dispatch_target"].as_str() != Some("closure")
        {
            return Ok(receipt);
        }

        let closure_receipt_is_already_materialized = receipt.downstream_dispatch_target.as_deref()
            == Some("closure")
            && matches!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready") | Some("executed")
            )
            && receipt.downstream_dispatch_blockers.is_empty()
            && receipt
                .downstream_dispatch_result_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
        if closure_receipt_is_already_materialized {
            return Ok(receipt);
        }

        let Some(dispatch_packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(receipt);
        };
        if !self
            .task_close_reconcile_has_persisted_receipt_truth(&receipt.run_id, &binding.task_id)
            .await?
        {
            return Ok(receipt);
        }

        let completion_receipt_id = format!("task-close-{}", binding.task_id);
        let completion_result_path =
            crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                self.root(),
                &receipt.run_id,
                "closure",
                &completion_receipt_id,
                dispatch_packet_path,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            receipt.run_id
        ));
        receipt.downstream_dispatch_note =
            Some("task close reconciled the run into lawful closure".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_result_path = Some(completion_result_path);
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_active_target = Some("closure".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.lane_status = Some("lane_completed".to_string());
        let public_receipt: RunGraphDispatchReceipt = receipt.clone().into();
        self.record_run_graph_dispatch_receipt(&public_receipt)
            .await?;
        Ok(receipt)
    }

    pub async fn latest_run_graph_recovery_summary(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_recovery_summary_for_current_session(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status_for_current_session().await? else {
            return Ok(None);
        };
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_checkpoint_summary(
        &self,
    ) -> Result<Option<RunGraphCheckpointSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphCheckpointSummary::from_status(status)))
    }

    pub async fn latest_run_graph_gate_summary(
        &self,
    ) -> Result<Option<RunGraphGateSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphGateSummary::from_status(status)))
    }

    pub async fn run_graph_recovery_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphRecoverySummary, StateStoreError> {
        let status = self.load_consistent_run_graph_status(run_id).await?;
        Ok(RunGraphRecoverySummary::from_status(status))
    }

    async fn load_consistent_run_graph_status(
        &self,
        run_id: &str,
    ) -> Result<RunGraphStatus, StateStoreError> {
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(status)
    }

    pub(crate) async fn run_graph_status_is_stale_after_release_admission_complete(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        self.run_graph_status_is_stale_after_release_admission_complete_from_task_rows(status, &[])
            .await
    }

    pub(crate) async fn run_graph_status_is_stale_after_release_admission_complete_from_task_rows(
        &self,
        status: &RunGraphStatus,
        task_rows: &[TaskRecord],
    ) -> Result<bool, StateStoreError> {
        let blocked_or_open_cycle = status.status == "blocked"
            || status.lifecycle_stage.ends_with("_blocked")
            || status.delegation_gate().delegated_cycle_open;
        if !blocked_or_open_cycle {
            return Ok(false);
        }
        let release_admission_complete =
            crate::runtime_consumption_state::release_admission_operator_evidence_complete_for_run(
                self.root(),
                &status.run_id,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        if !release_admission_complete {
            return Ok(false);
        }
        if task_rows.is_empty() {
            match self.show_task(&status.task_id).await {
                Ok(task) => Ok(task.status == "closed"),
                Err(StateStoreError::MissingTask { .. }) => Ok(true),
                Err(error) => Err(error),
            }
        } else {
            Ok(task_rows
                .iter()
                .find(|task| task.id == status.task_id)
                .map(|task| task.status == "closed")
                .unwrap_or(true))
        }
    }

    pub async fn run_graph_checkpoint_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphCheckpointSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphCheckpointSummary::from_status(status))
    }

    pub async fn run_graph_gate_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphGateSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphGateSummary::from_status(status))
    }

    fn ensure_run_graph_dispatch_receipt_summary_consistency(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(receipt)?;
        let Some(raw_lane_status) = receipt.lane_status.as_deref() else {
            return Ok(());
        };
        let raw_lane_status = raw_lane_status.trim();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        let effective_derived_lane_status = if downstream_dispatch_allows_completed_lane_status(
            receipt.downstream_dispatch_status.as_deref(),
            canonical_lane_status,
        ) {
            "lane_completed".to_string()
        } else {
            normalize_run_graph_lane_status(
                Some(raw_lane_status),
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
        };
        if receipt.downstream_dispatch_status.is_some()
            && canonical_lane_status != effective_derived_lane_status
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_status `{}` with lane_status `{}` conflicts with derived lane_status `{}` from dispatch_status `{}`",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none"),
                    canonical_lane_status,
                    effective_derived_lane_status,
                    receipt.dispatch_status
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_dispatch_receipt_required_fields_present(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        if receipt.dispatch_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: dispatch_status must be non-empty",
                    receipt.run_id
                ),
            });
        }
        let Some(raw_lane_status) = receipt.lane_status.as_deref() else {
            return Ok(());
        };
        if raw_lane_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: lane_status must be non-empty when present",
                    receipt.run_id
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        let Some(downstream_status) = receipt.downstream_dispatch_status.as_deref() else {
            return Ok(());
        };
        let downstream_status = downstream_status.trim().to_ascii_lowercase();
        let requires_blockers = downstream_status == "blocked";
        if requires_blockers && receipt.downstream_dispatch_blockers.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must be present and non-empty when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.is_empty() {
            return Ok(());
        }
        let mut canonical_blockers = std::collections::HashSet::new();
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let raw_blocker = blocker.as_str();
            let blocker = blocker.trim();
            let collapsed = blocker.split_whitespace().collect::<Vec<_>>().join(" ");
            raw_blocker != blocker
                || blocker.is_empty()
                || !blocker.is_ascii()
                || blocker.to_ascii_lowercase() != blocker
                || collapsed != blocker
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let canonical_blocker = blocker.trim().to_ascii_lowercase();
            !canonical_blockers.insert(canonical_blocker)
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must not contain duplicate canonical entries after lowercase canonicalization when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_run_graph_dispatch_receipt_contract(
        receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(&receipt)?;
        let receipt = normalize_legacy_downstream_preview_drift(receipt);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if matches!(status.trim(), "closed" | "completed") {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_run_graph_root(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn restore_vida_session_id(saved: Option<String>) {
        unsafe {
            match saved {
                Some(value) => std::env::set_var("VIDA_SESSION_ID", value),
                None => std::env::remove_var("VIDA_SESSION_ID"),
            }
        }
    }

    fn saved_runtime_session_env() -> Vec<(&'static str, Option<String>)> {
        [
            "VIDA_SESSION_ID",
            "VIDA_ORCHESTRATOR_SESSION_ID",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_REMOTE_SESSION_ID",
            "CODEX_SESSION_ID",
            "CODEX_THREAD_ID",
        ]
        .into_iter()
        .map(|name| (name, std::env::var(name).ok()))
        .collect()
    }

    fn clear_runtime_session_env() {
        unsafe {
            for name in [
                "VIDA_SESSION_ID",
                "VIDA_ORCHESTRATOR_SESSION_ID",
                "CLAUDE_CODE_SESSION_ID",
                "CLAUDE_CODE_REMOTE_SESSION_ID",
                "CODEX_SESSION_ID",
                "CODEX_THREAD_ID",
            ] {
                std::env::remove_var(name);
            }
        }
    }

    fn restore_runtime_session_env(saved: Vec<(&'static str, Option<String>)>) {
        unsafe {
            for (name, value) in saved {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    fn reconciled_pack_receipt_for_packet(packet_path: String) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: "run-materialized-pack-context".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "tracked_flow_materialization".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: Some("vida task ensure work-pool".to_string()),
            dispatch_packet_path: Some(packet_path),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("dev-pack".to_string()),
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
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-06T00:00:00Z".to_string(),
        }
    }

    fn write_reconciled_pack_packet(path: &std::path::Path, marker: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create packet parent");
        }
        fs::write(
            path,
            serde_json::to_string_pretty(&serde_json::json!({
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "orchestrator",
                    "request": marker,
                    "selected_role": "pm",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "work-pool-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["work-pool-pack"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {}
                        },
                        "orchestration_contract": {}
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "marker": marker
                }
            }))
            .expect("packet json should encode"),
        )
        .expect("write reconciled pack packet");
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_absolute_packet_outside_state_root() {
        let root = temp_run_graph_root("vida-reconciled-pack-external-packet");
        let external_root = temp_run_graph_root("vida-reconciled-pack-attacker-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let external_packet = external_root.join("outside-packet.json");
        write_reconciled_pack_packet(&external_packet, "outside-state-root");

        let receipt = reconciled_pack_receipt_for_packet(external_packet.display().to_string());
        let context = store
            .reconciled_pack_dispatch_context(&receipt)
            .expect("out-of-root packet should fail closed without read error");

        assert!(
            context.is_none(),
            "materialized pack reconciliation must not read packet paths outside the VIDA state root"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(external_root);
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_allows_absolute_packet_inside_state_root() {
        let root = temp_run_graph_root("vida-reconciled-pack-state-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("inside-packet.json");
        write_reconciled_pack_packet(&packet, "inside-state-root");

        let receipt = reconciled_pack_receipt_for_packet(packet.display().to_string());
        let (_role_selection, run_graph_bootstrap) = store
            .reconciled_pack_dispatch_context(&receipt)
            .expect("in-root packet should decode")
            .expect("in-root packet should be accepted");

        assert_eq!(run_graph_bootstrap["marker"], "inside-state-root");
        let _ = fs::remove_dir_all(root);
    }

    fn sample_run_graph_status() -> RunGraphStatus {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-vida-a",
            "implementation",
            "implementation",
        );
        status.task_id = "vida-a".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("coach".to_string());
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "awaiting_coach".to_string();
        status.context_state = "open".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        status
    }

    fn test_task_record(task_id: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-05-22T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-05-22T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn sample_explicit_binding(
        run_id: &str,
        task_id: &str,
        recorded_at: &str,
    ) -> RunGraphContinuationBinding {
        RunGraphContinuationBinding {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "run_id": run_id,
                "task_id": task_id,
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "test continuation".to_string(),
            primary_path: "crates/vida/src/state_store_run_graph_summary.rs".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: recorded_at.to_string(),
        }
    }

    fn sample_dispatch_receipt(run_id: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --execute-dispatch --json".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
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
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn terminal_closure_supersedes_stale_pending_developer_handoff_receipt() {
        let mut status = sample_run_graph_status();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.resume_target = "none".to_string();

        let mut receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
                )
                .to_string(),
            ),
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
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-17T00:00:00Z".to_string(),
        };

        assert!(terminal_closure_supersedes_stale_handoff_receipt(
            &status,
            &mut receipt
        ));
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert_eq!(receipt.blocker_code, None);
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("executed")
        );
    }

    fn sample_replay_lineage_receipt(
        receipt_id: &str,
        run_id: &str,
        recorded_at: &str,
    ) -> RunGraphReplayLineageReceipt {
        RunGraphReplayLineageReceipt {
            receipt_id: receipt_id.to_string(),
            run_id: run_id.to_string(),
            lineage_kind: "dispatch_packet_lineage".to_string(),
            replay_scope: "completed_resume".to_string(),
            origin_checkpoint_ref: format!("checkpoint:{run_id}"),
            fork_parent: None,
            source_dispatch_target: "coach".to_string(),
            source_dispatch_packet_path: Some(format!("/tmp/{run_id}-coach.json")),
            source_dispatch_result_path: Some(format!("/tmp/{run_id}-coach-result.json")),
            resolved_dispatch_target: "closure".to_string(),
            resolved_task_id: format!("task-{run_id}"),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            validation_outcome: "pass".to_string(),
            recorded_at: recorded_at.to_string(),
        }
    }

    #[test]
    fn dispatch_summary_preserves_executed_explicit_lane_completed_receipt() {
        let summary = RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
            run_id: "run-lane-complete".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida lane complete".to_string()),
            dispatch_command: Some("vida lane complete run-lane-complete".to_string()),
            dispatch_packet_path: Some("runtime-consumption/dispatch-packets/run.json".to_string()),
            dispatch_result_path: Some("runtime-consumption/dispatch-results/run.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("activate writer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(
                "runtime-consumption/dispatch-results/run.json".to_string(),
            ),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-15T08:00:00Z".to_string(),
        });

        assert_eq!(summary.lane_status, "lane_completed");
        assert_eq!(
            summary.downstream_dispatch_blockers,
            ["missing_owned_write_scope"]
        );
    }

    #[test]
    fn executed_lane_completed_dispatch_receipt_signal_is_not_ambiguous() {
        let summary = RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
            run_id: "run-lane-complete-signal".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida lane complete".to_string()),
            dispatch_command: Some("vida lane complete run-lane-complete-signal".to_string()),
            dispatch_packet_path: Some("runtime-consumption/dispatch-packets/run.json".to_string()),
            dispatch_result_path: Some("runtime-consumption/dispatch-results/run.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("activate writer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(
                "runtime-consumption/dispatch-results/run.json".to_string(),
            ),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-15T08:00:00Z".to_string(),
        });

        assert!(!latest_run_graph_dispatch_receipt_signal_is_ambiguous(
            &summary
        ));
    }

    #[test]
    fn routed_specification_receipt_with_pending_downstream_evidence_stays_dispatch_ready() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-spec-routed".to_string();
        status.task_id = "task-spec-routed".to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("pm".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "dispatch_ready".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_pm".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "dispatch.pm_lane".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --dispatch-packet packet --execute-dispatch --json".to_string(),
            ),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "pending_specification_evidence".to_string(),
                "pending_spec_task_close".to_string(),
                "pending_design_finalize".to_string(),
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("pm".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-02T06:52:54Z".to_string(),
        };

        let stored_receipt = receipt.into();
        let reconciled =
            reconcile_run_graph_status_with_dispatch_receipt(status, Some(&stored_receipt))
                .expect("routed pre-execution receipt should reconcile");

        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.lifecycle_stage, "dispatch_ready");
        assert_eq!(reconciled.resume_target, "dispatch.pm_lane");
        assert!(reconciled.recovery_ready);
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_and_dispatch_context_round_trip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-binding-roundtrip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let binding = RunGraphContinuationBinding {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "run-1",
                "active_node": "pm",
            }),
            binding_source: "test".to_string(),
            why_this_unit: "because".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("req".to_string()),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };
        let context = RunGraphDispatchContext {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            request_text: "req".to_string(),
            role_selection: serde_json::json!({
                "ok": true,
                "activation_source": "test",
                "selection_mode": "fixed",
                "fallback_role": "worker",
                "request": "req",
                "selected_role": "worker",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {},
                "reason": "test"
            }),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };

        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record binding");
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("record context");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-1".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "implementation".to_string(),
                dispatch_surface: Some("test".to_string()),
                dispatch_command: Some("test command".to_string()),
                dispatch_packet_path: Some("/tmp/run-1.json".to_string()),
                dispatch_result_path: Some("/tmp/run-1-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida taskflow consume continue".to_string()),
                downstream_dispatch_note: Some("test note".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/run-1-coach.json".to_string()),
                downstream_dispatch_status: Some("ready".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T10:00:01Z".to_string(),
            })
            .await
            .expect("record receipt");

        let stored_binding = store
            .run_graph_continuation_binding("run-1")
            .await
            .expect("read binding")
            .expect("binding present");
        let stored_context = store
            .run_graph_dispatch_context("run-1")
            .await
            .expect("read context")
            .expect("context present");

        assert_eq!(stored_binding.binding_source, "test");
        assert_eq!(stored_binding.active_bounded_unit["active_node"], "pm");
        assert_eq!(stored_context.request_text, "req");
        assert_eq!(
            stored_context
                .role_selection()
                .expect("role selection should decode")
                .selected_role,
            "worker"
        );
        for artifact_kind in [
            "continuation_binding",
            "dispatch_context",
            "dispatch_receipt",
        ] {
            let owner_record = store
                .run_graph_owner_evidence_record("run-1", artifact_kind)
                .await
                .expect("read owner evidence")
                .unwrap_or_else(|| panic!("missing owner evidence for {artifact_kind}"));
            assert_eq!(owner_record.run_id, "run-1");
            assert_eq!(owner_record.artifact_kind, artifact_kind);
            assert_eq!(
                owner_record.runtime_owner_evidence["mutation_gate"],
                "current_session_allowed"
            );
            assert!(
                owner_record.runtime_owner_evidence["current_session"]["session_id"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty())
            );
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_read_does_not_record_owner_evidence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-read-owner-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .record_run_graph_owner_evidence("seed-owner-evidence", "dispatch_context")
            .await
            .expect("seed owner evidence table");

        let mut status = sample_run_graph_status();
        status.run_id = "run-read-only-owner-evidence".to_string();
        status.task_id = "task-read-only-owner-evidence".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let loaded = store
            .run_graph_status("run-read-only-owner-evidence")
            .await
            .expect("read run graph status");
        assert_eq!(loaded.run_id, "run-read-only-owner-evidence");
        assert!(store
            .run_graph_owner_evidence_record("run-read-only-owner-evidence", "run_graph_status")
            .await
            .expect("read owner evidence")
            .is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_legacy_ownerless_tracks_owner_evidence_and_claims() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-legacy-ownerless-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut ownerless = sample_run_graph_status();
        ownerless.run_id = "legacy-ownerless-run".to_string();
        ownerless.task_id = "legacy-ownerless-task".to_string();
        store
            .record_run_graph_status(&ownerless)
            .await
            .expect("persist ownerless run graph status");
        assert!(store
            .run_graph_legacy_ownerless("legacy-ownerless-run")
            .await
            .expect("classify ownerless run"));

        store
            .record_run_graph_owner_evidence("legacy-ownerless-run", "dispatch_context")
            .await
            .expect("record owner evidence");
        assert!(!store
            .run_graph_legacy_ownerless("legacy-ownerless-run")
            .await
            .expect("owner evidence should make run non-ownerless"));

        let mut claimed = sample_run_graph_status();
        claimed.run_id = "legacy-claimed-run".to_string();
        claimed.task_id = "legacy-claimed-task".to_string();
        store
            .record_run_graph_status(&claimed)
            .await
            .expect("persist claim-backed run graph status");
        assert!(store
            .run_graph_legacy_ownerless("legacy-claimed-run")
            .await
            .expect("classify pre-claim run"));
        let claim = store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "legacy-claimed-run-write".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-a".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("legacy-claimed-task".to_string()),
                run_id: Some("legacy-claimed-run".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("legacy-claimed-domain".to_string()),
                owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire claim");
        assert!(!store
            .run_graph_legacy_ownerless("legacy-claimed-run")
            .await
            .expect("claim should make run non-ownerless"));
        store
            .release_orchestrator_claim(&claim.claim_id, claim.resource_revision, "test release")
            .await
            .expect("release claim");
        assert!(store
            .run_graph_legacy_ownerless("legacy-claimed-run")
            .await
            .expect("released claim should not block ownerless classification"));

        let mut expired = sample_run_graph_status();
        expired.run_id = "legacy-expired-claim-run".to_string();
        expired.task_id = "legacy-expired-claim-task".to_string();
        store
            .record_run_graph_status(&expired)
            .await
            .expect("persist expired-claim run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "legacy-expired-claim-run-write".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-a".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("legacy-expired-claim-task".to_string()),
                run_id: Some("legacy-expired-claim-run".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("legacy-expired-domain".to_string()),
                owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: -1,
            })
            .await
            .expect("acquire expiring claim");
        assert_eq!(
            store
                .expire_stale_orchestrator_claims()
                .await
                .expect("expire stale claims"),
            1
        );
        assert!(store
            .run_graph_legacy_ownerless("legacy-expired-claim-run")
            .await
            .expect("expired claim should not block ownerless classification"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_mutation_blocks_foreign_session_without_claim() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-claim-block");
        let owner_store = StateStore::open(root.clone())
            .await
            .expect("open owner store");
        owner_store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "owner-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-owner".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-owner".to_string()),
                run_id: Some("run-owned".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("owner-domain".to_string()),
                owned_paths: vec!["owner/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire owner claim");
        owner_store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-owned"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-foreign");
        }
        let binding_result = owner_store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owned",
                "task-owner",
                "2026-05-21T01:00:00Z",
            ))
            .await;
        assert!(binding_result.is_err());

        let mut foreign_status = sample_run_graph_status();
        foreign_status.run_id = "run-owned".to_string();
        foreign_status.task_id = "task-owner".to_string();
        let status_result = owner_store.record_run_graph_status(&foreign_status).await;
        assert!(status_result.is_err());

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_mutation_allows_current_session_task_claim_for_run_task() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-task-claim");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.run_id = "run-task-claim".to_string();
        status.task_id = "task-task-claim".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner status");
        store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-task-claim"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-task-only-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-task-claim".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-task-claim".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current task claim");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-task-claim",
                "task-task-claim",
                "2026-05-21T01:00:00Z",
            ))
            .await
            .expect("task claim should authorize run continuation mutation");

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_mutation_allows_claimed_parent_explicit_task_binding_child_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-explicit-task-child");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut parent_status = sample_run_graph_status();
        parent_status.run_id = "parent-run".to_string();
        parent_status.task_id = "parent-run".to_string();
        store
            .record_run_graph_status(&parent_status)
            .await
            .expect("persist parent status");
        store
            .persist_task_record(test_task_record("child-task-run", "open"))
            .await
            .expect("persist child task");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-parent-run-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("parent-run".to_string()),
                run_id: Some("parent-run".to_string()),
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:parent-run".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current parent claim");
        let mut binding =
            sample_explicit_binding("parent-run", "child-task-run", "2026-05-21T01:00:00Z");
        binding.binding_source = "explicit_continuation_bind_task".to_string();
        binding.active_bounded_unit = serde_json::json!({
            "kind": "task_graph_task",
            "run_id": "parent-run",
            "task_id": "child-task-run",
        });
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record explicit parent-child binding");

        let mut child_status = sample_run_graph_status();
        child_status.run_id = "child-task-run".to_string();
        child_status.task_id = "child-task-run".to_string();
        store
            .record_run_graph_status(&child_status)
            .await
            .expect("parent claim should authorize bound child task-as-run mutation");

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_explicit_binding_for_current_session_uses_task_claim_run_scope() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-current-session-task-claim-run-scope");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.run_id = "run-scope-from-task".to_string();
        status.task_id = "task-scope-from-task".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner status");
        store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-scope-from-task"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-task-scope-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-scope-from-task".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-scope-from-task".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current task claim");
        store
            .persist_task_record(test_task_record("next-bound-task", "open"))
            .await
            .expect("persist bound task");
        let mut binding = sample_explicit_binding(
            "run-scope-from-task",
            "next-bound-task",
            "2026-05-21T01:00:00Z",
        );
        binding.binding_source = "explicit_continuation_bind_task".to_string();
        binding.active_bounded_unit = serde_json::json!({
            "kind": "task_graph_task",
            "run_id": "run-scope-from-task",
            "task_id": "next-bound-task",
        });
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record explicit binding");

        let binding = store
            .latest_explicit_run_graph_continuation_binding_for_current_session()
            .await
            .expect("read current scoped binding")
            .expect("binding should be in current session scope");

        assert_eq!(binding.run_id, "run-scope-from-task");
        assert_eq!(binding.task_id, "next-bound-task");

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[test]
    fn owner_evidence_matches_prior_generated_local_session_by_stable_fallback() {
        let prior_owner_evidence = serde_json::json!({
            "current_session": {
                "session_id": "local-session-worktreehash-12345",
                "fallback_replaces_legacy_stable_worktree_state_hash": "local-worktree-worktreehash"
            }
        });
        let current_owner_evidence = serde_json::json!({
            "mutation_gate": "current_session_allowed",
            "live_other_sessions": [],
            "stale_sessions": [{
                "session_id": "stale-foreign-project",
                "project_root": "\\\\?\\C:\\project\\other",
                "worktree_environment_id": "\\\\?\\C:\\project\\other"
            }],
            "current_session": {
                "session_id": "local-session-worktreehash",
                "fallback_replaces_legacy_stable_worktree_state_hash": "local-worktree-worktreehash"
            }
        });

        assert!(StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "local-session-worktreehash",
            Some("local-worktree-worktreehash"),
        ));
        assert!(!StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "local-session-other",
            Some("local-worktree-other"),
        ));
    }

    #[test]
    fn owner_evidence_adopts_stale_codex_thread_owner_for_same_worktree() {
        let prior_owner_evidence = serde_json::json!({
            "current_session": {
                "session_id": "019e-old-codex-thread",
                "identity_source": "CODEX_THREAD_ID",
                "project_root": "\\\\?\\C:\\project\\vida_mobile",
                "worktree_environment_id": "\\\\?\\C:\\project\\vida_mobile"
            }
        });
        let current_owner_evidence = serde_json::json!({
            "mutation_gate": "current_session_allowed",
            "live_other_sessions": [],
            "current_session": {
                "session_id": "019e-new-codex-thread",
                "identity_source": "CODEX_THREAD_ID",
                "project_root": "\\\\?\\C:\\project\\vida_mobile",
                "worktree_environment_id": "\\\\?\\C:\\project\\vida_mobile"
            }
        });

        assert!(StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "019e-new-codex-thread",
            None,
        ));
    }

    #[tokio::test]
    async fn run_graph_mutation_adopts_legacy_same_worktree_owner_evidence_without_competing_owner()
    {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_env = saved_runtime_session_env();
        clear_runtime_session_env();

        let root = temp_run_graph_root("vida-run-graph-legacy-owner-evidence-adopt");
        let legacy_store = StateStore::open(root.clone())
            .await
            .expect("open legacy store");
        let mut status = sample_run_graph_status();
        status.run_id = "legacy-owner-evidence-run".to_string();
        status.task_id = "legacy-owner-evidence-task".to_string();
        legacy_store
            .record_run_graph_status(&status)
            .await
            .expect("persist legacy ownerless status");
        legacy_store
            .record_run_graph_owner_evidence("legacy-owner-evidence-run", "dispatch_context")
            .await
            .expect("record legacy local owner evidence");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "current-explicit-session");
        }
        let result = legacy_store.record_run_graph_status(&status).await;
        assert!(
            result.is_ok(),
            "legacy same-worktree owner evidence should be adoptable when no live/stale competing owner exists: {result:?}"
        );

        let _ = fs::remove_dir_all(&root);
        restore_runtime_session_env(saved_env);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_ignores_foreign_claims() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-latest");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut foreign_status = sample_run_graph_status();
        foreign_status.run_id = "run-foreign".to_string();
        foreign_status.task_id = "task-foreign".to_string();
        store
            .record_run_graph_status(&foreign_status)
            .await
            .expect("persist foreign run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "foreign-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-foreign".to_string(),
                process_id: None,
                task_id: Some("task-foreign".to_string()),
                run_id: Some("run-foreign".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("foreign-domain".to_string()),
                owned_paths: vec!["foreign/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire foreign claim");

        assert_eq!(
            store
                .latest_run_graph_status()
                .await
                .expect("read global latest")
                .expect("global latest present")
                .run_id,
            "run-foreign"
        );
        assert!(store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read scoped latest")
            .is_none());

        let mut current_status = sample_run_graph_status();
        current_status.run_id = "run-current".to_string();
        current_status.task_id = "task-current".to_string();
        store
            .record_run_graph_status(&current_status)
            .await
            .expect("persist current run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: None,
                task_id: Some("task-current".to_string()),
                run_id: Some("run-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-open-domain".to_string()),
                owned_paths: vec!["open-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire current claim");

        assert_eq!(
            store
                .latest_run_graph_status_for_current_session()
                .await
                .expect("read scoped latest")
                .expect("scoped latest present")
                .run_id,
            "run-current"
        );

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_skips_closed_task_active_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-closed-skip");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-closed-skip");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for (task_id, status) in [
            ("task-open-current-run", "open"),
            ("task-closed-current-run", "closed"),
        ] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: "Current session run graph task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status,
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create task");
        }

        let mut open_status = crate::taskflow_run_graph::default_run_graph_status(
            "task-open-current-run",
            "implementation",
            "implementation",
        );
        open_status.run_id = "run-open-current".to_string();
        store
            .record_run_graph_status(&open_status)
            .await
            .expect("persist open run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-open-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-closed-skip".to_string(),
                process_id: None,
                task_id: Some("task-open-current-run".to_string()),
                run_id: Some("run-open-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-open-domain".to_string()),
                owned_paths: vec!["open-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire open claim");

        let mut stale_closed_status = crate::taskflow_run_graph::default_run_graph_status(
            "task-closed-current-run",
            "implementation",
            "implementation",
        );
        stale_closed_status.run_id = "run-zzz-closed-current".to_string();
        stale_closed_status.status = "ready".to_string();
        stale_closed_status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        store
            .record_run_graph_status(&stale_closed_status)
            .await
            .expect("persist stale closed-task status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-closed-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-closed-skip".to_string(),
                process_id: None,
                task_id: Some("task-closed-current-run".to_string()),
                run_id: Some("run-zzz-closed-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-closed-domain".to_string()),
                owned_paths: vec!["closed-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire closed claim");

        let latest = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read scoped latest")
            .expect("open run should remain after closed-task run is skipped");
        assert_eq!(latest.run_id, "run-open-current");
        assert_eq!(latest.task_id, "task-open-current-run");

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_explicit_continuation_binding_for_current_session_skips_foreign_binding() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-binding");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-current-binding",
                "task-current-binding",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist current binding");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-binding-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-binding".to_string(),
                process_id: None,
                task_id: Some("task-current-binding".to_string()),
                run_id: Some("run-current-binding".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-binding-domain".to_string()),
                owned_paths: vec!["current/binding/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire current binding claim");
        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-foreign-binding",
                "task-foreign-binding",
                "2026-05-21T01:00:00Z",
            ))
            .await
            .expect("persist newer foreign binding");

        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding()
                .await
                .expect("read global binding")
                .expect("global binding present")
                .run_id,
            "run-foreign-binding"
        );
        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding_for_current_session()
                .await
                .expect("read scoped binding")
                .expect("scoped binding present")
                .run_id,
            "run-current-binding"
        );

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_explicit_continuation_binding_for_current_session_uses_current_owner_evidence_without_claim(
    ) {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner-evidence-binding");
        }
        let root = temp_run_graph_root("vida-run-graph-owner-evidence-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owner-evidence-binding",
                "task-owner-evidence-binding",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist owner-evidence binding");

        assert!(store
            .active_orchestrator_claims()
            .await
            .expect("read claims")
            .is_empty());
        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding_for_current_session()
                .await
                .expect("read scoped binding")
                .expect("scoped binding present")
                .run_id,
            "run-owner-evidence-binding"
        );

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_uses_owner_evidence_without_claim() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner-evidence-status");
        }
        let root = temp_run_graph_root("vida-run-graph-owner-evidence-status");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-owner-evidence-status".to_string();
        status.task_id = "task-owner-evidence-status".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner-evidence status");
        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owner-evidence-status",
                "task-owner-evidence-status",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist owner-evidence binding");

        assert!(store
            .active_orchestrator_claims()
            .await
            .expect("read claims")
            .is_empty());
        assert_eq!(
            store
                .latest_run_graph_status_for_current_session()
                .await
                .expect("read scoped status")
                .expect("scoped status present")
                .run_id,
            "run-owner-evidence-status"
        );

        let _ = fs::remove_dir_all(&root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_status_does_not_reconcile_closed_in_progress_task_into_completed() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-task-close-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "feature-close-dev",
                title: "Implement bounded fix",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create active task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-close-task",
            "implementation",
            "implementation",
        );
        status.task_id = "feature-close-dev".to_string();
        status.active_node = "implementer".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "none".to_string();
        status.checkpoint_kind = "active".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        store
            .close_task("feature-close-dev", "implemented and proven")
            .await
            .expect("close active task");

        let reconciled = store
            .run_graph_status("run-close-task")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.status, "in_progress");
        assert_eq!(reconciled.lifecycle_stage, "implementer_active");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.policy_gate, "targeted_verification");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.checkpoint_kind, "active");
        assert_eq!(reconciled.resume_target, "none");
        assert!(reconciled.recovery_ready);
        assert!(reconciled.delegation_gate().delegated_cycle_open);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_reconciles_closed_terminal_closure_blocker_into_completed() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closed-terminal-closure-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "feature-terminal-closure",
                title: "Closed terminal closure task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        for lifecycle_stage in ["closure_blocked", "closure_complete"] {
            let run_id = format!("run-{lifecycle_stage}");
            let mut status = crate::taskflow_run_graph::default_run_graph_status(
                &run_id, "delivery", "delivery",
            );
            status.task_id = "feature-terminal-closure".to_string();
            status.active_node = "closure".to_string();
            status.status = "blocked".to_string();
            status.lifecycle_stage = lifecycle_stage.to_string();
            status.next_node = None;
            status.policy_gate = "single_task_scope_required".to_string();
            status.handoff_state = "none".to_string();
            status.context_state = "ready".to_string();
            status.checkpoint_kind = "blocked".to_string();
            status.resume_target = "none".to_string();
            status.recovery_ready = false;
            store
                .record_run_graph_status(&status)
                .await
                .expect("persist terminal closure run-graph status");

            let reconciled = store
                .run_graph_status(&run_id)
                .await
                .expect("load reconciled terminal closure status");
            assert_eq!(reconciled.status, "completed");
            assert_eq!(reconciled.lifecycle_stage, "closure_complete");
            assert_eq!(reconciled.next_node, None);
            assert_eq!(reconciled.policy_gate, "not_required");
            assert_eq!(reconciled.handoff_state, "none");
            assert_eq!(reconciled.context_state, "sealed");
            assert_eq!(reconciled.checkpoint_kind, "none");
            assert_eq!(reconciled.resume_target, "none");
            assert!(!reconciled.recovery_ready);
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_heals_legacy_downstream_preview_drift_for_exception_recorded_active_dispatch(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-legacy-preview-drift-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-legacy-preview-drift",
            "implementer",
            "delivery",
        );
        status.task_id = "task-legacy-preview-drift".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-legacy-preview-drift".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: Some("sup-parent".to_string()),
                exception_path_receipt_id: Some("exc-parent".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("implementer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale retry preview".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/stale-preview.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/stale-preview-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist legacy drift receipt");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("summary should heal legacy drift")
            .expect("summary should exist");
        assert_eq!(summary.lane_status, "lane_exception_recorded");
        assert!(summary.downstream_dispatch_status.is_none());
        assert!(!summary.downstream_dispatch_ready);
        assert!(summary.downstream_dispatch_packet_path.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_keeps_advanced_handoff_after_exception_takeover_lane_moves_on() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-exception-takeover-advanced-handoff-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-exception-advanced",
            "implementation",
            "implementation",
        );
        status.task_id = "task-exception-advanced".to_string();
        status.active_node = "test_author".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run graph status");

        let mut receipt = sample_dispatch_receipt("run-exception-advanced");
        receipt.dispatch_target = "test_author".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");

        let mut advanced = status.clone();
        advanced.active_node = "coach".to_string();
        advanced.next_node = Some("review_ensemble".to_string());
        advanced.status = "ready".to_string();
        advanced.lane_id = "coach_lane".to_string();
        advanced.lifecycle_stage = "coach_active".to_string();
        advanced.policy_gate = "review_findings".to_string();
        advanced.handoff_state = "awaiting_review_ensemble".to_string();
        advanced.checkpoint_kind = "execution_cursor".to_string();
        advanced.resume_target = "dispatch.review_ensemble".to_string();
        advanced.recovery_ready = true;
        store
            .record_run_graph_status(&advanced)
            .await
            .expect("persist advanced handoff status");

        let reconciled = store
            .run_graph_status("run-exception-advanced")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "coach");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.resume_target, "dispatch.review_ensemble");
        assert!(reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn tracked_flow_materialization_pass_keeps_activation_view_only_receipt_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-tracked-flow-materialization-pass-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-tracked-flow-materialization",
            "spec-pack",
            "scope_discussion",
        );
        status.task_id = "task-tracked-flow-materialization".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "work_pool_pack_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked materialization status");

        let result_path = root.join("tracked-flow-materialization-result.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": false,
                    "reused_existing": true,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": []
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-tracked-flow-materialization");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked materialization receipt");

        let reconciled = store
            .run_graph_status("run-tracked-flow-materialization")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "work-pool-pack");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.lifecycle_stage, "work_pool_pack_blocked");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn tracked_flow_materialization_with_null_blocker_records_work_pool_identity() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-tracked-flow-materialization-null-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let tasks_path = root.join("tasks.jsonl");
        fs::write(
            &tasks_path,
            concat!(
                "{\"id\":\"feature-example\",\"title\":\"Feature\",\"description\":\"feature\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"spec-first-feature\"],\"dependencies\":[]}\n",
                "{\"id\":\"feature-example-spec\",\"title\":\"Spec\",\"description\":\"spec\",\"status\":\"closed\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"spec-pack\"],\"dependencies\":[{\"issue_id\":\"feature-example-spec\",\"depends_on_id\":\"feature-example\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"feature-example-work-pool\",\"title\":\"Work pool\",\"description\":\"work pool\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"work_pool\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"work-pool-pack\"],\"dependencies\":[{\"issue_id\":\"feature-example-work-pool\",\"depends_on_id\":\"feature-example\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write tasks");
        store
            .import_tasks_from_jsonl(&tasks_path)
            .await
            .expect("import tasks");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-tracked-flow-null-blocker",
            "feature-example",
            "scope_discussion",
        );
        status.task_id = "feature-example".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "work_pool_pack_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist materialization status");

        let result_path = root.join("tracked-flow-materialization-null-blocker-result.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": true,
                    "reused_existing": false,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": ["taskflow:feature-example-work-pool"]
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-tracked-flow-null-blocker");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.blocker_code = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist materialization receipt");

        let loaded = store
            .run_graph_dispatch_receipt("run-tracked-flow-null-blocker")
            .await
            .expect("load dispatch receipt")
            .expect("receipt should exist");
        assert_eq!(loaded.dispatch_surface.as_deref(), Some("vida task ensure"));
        let identity = store
            .run_graph_dispatch_task_identity("run-tracked-flow-null-blocker")
            .await
            .expect("load dispatch task identity")
            .expect("materialized work-pool identity should be recorded");
        assert_eq!(identity.feature_epic_id.as_deref(), Some("feature-example"));
        assert_eq!(
            identity.spec_task_id.as_deref(),
            Some("feature-example-spec")
        );
        assert_eq!(
            identity.work_pool_task_id.as_deref(),
            Some("feature-example-work-pool")
        );
        assert_eq!(
            identity.source,
            "work_pool_materialization_identity_reconciliation"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn work_pool_materialization_pass_without_result_path_keeps_receipt_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-work-pool-materialization-missing-result-path-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-work-pool-materialization",
            "spec-pack",
            "scope_discussion",
        );
        status.task_id = "task-work-pool-materialization".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "work_pool_pack_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked materialization status");

        let results_dir = root.join("runtime-consumption").join("dispatch-results");
        fs::create_dir_all(&results_dir).expect("create dispatch results dir");
        fs::write(
            results_dir.join("run-work-pool-materialization-2026-06-03T01-00-00Z.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": false,
                    "reused_existing": true,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": []
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-work-pool-materialization");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = None;
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked materialization receipt");

        let reconciled = store
            .run_graph_status("run-work-pool-materialization")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "work-pool-pack");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.lifecycle_stage, "work_pool_pack_blocked");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn completed_run_status_is_downgraded_by_newer_blocked_dispatch_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-completed-over-stale-blocked-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure",
            "closure",
            "delivery",
        );
        status.task_id = "task-closure".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure".to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale blocked coach lineage".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_closure_handoff".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale blocked dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure")
            .await
            .expect("load reconciled completed run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "verification");
        assert_eq!(reconciled.lifecycle_stage, "verification_blocked");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.resume_target, "none");
        assert_eq!(reconciled.selected_backend, "senior");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn terminal_closure_status_survives_blocked_receipt_after_explicit_takeover() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-terminal-closure-over-takeover-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-closure",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "awaiting_closure".to_string();
        status.context_state = "open".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.closure".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist terminal closure status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-terminal-closure".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("sup-terminal-closure".to_string()),
                exception_path_receipt_id: Some("exc-terminal-closure".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: Some("stale terminal blocker".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            })
            .await
            .expect("persist superseded blocked dispatch receipt");

        let reconciled = store
            .run_graph_status("run-terminal-closure")
            .await
            .expect("load reconciled terminal closure status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert!(!reconciled.recovery_ready);
        assert_eq!(reconciled.selected_backend, "internal_subagents");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_superseded_lane_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-superseded-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let active = crate::taskflow_run_graph::default_run_graph_status(
            "run-active",
            "task-active",
            "implementation",
        );
        store
            .record_run_graph_status(&active)
            .await
            .expect("persist active status");

        let stale = crate::taskflow_run_graph::default_run_graph_status(
            "run-stale-superseded",
            "task-stale",
            "implementation",
        );
        store
            .record_run_graph_status(&stale)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-stale-superseded".to_string(),
                dispatch_target: "business_analyst".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_superseded".to_string(),
                supersedes_receipt_id: Some("supersede-stale-run".to_string()),
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
                downstream_dispatch_note: Some("stale delegated cycle".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("business_analyst".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-05-15T19:40:00Z".to_string(),
            })
            .await
            .expect("persist superseded dispatch receipt");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("non-superseded status should remain");
        assert_eq!(latest.run_id, "run-active");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_active_run_for_closed_task() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-active-closed-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-active-run",
                title: "Closed task with stale active run",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let active = crate::taskflow_run_graph::default_run_graph_status(
            "run-active-open-task",
            "task-active-open",
            "implementation",
        );
        store
            .record_run_graph_status(&active)
            .await
            .expect("persist active open status");

        let mut stale = crate::taskflow_run_graph::default_run_graph_status(
            "run-closed-active-task",
            "task-closed-active-run",
            "implementation",
        );
        stale.task_id = "task-closed-active-run".to_string();
        stale.status = "ready".to_string();
        stale.lifecycle_stage = "implementation_dispatch_ready".to_string();
        store
            .record_run_graph_status(&stale)
            .await
            .expect("persist stale closed-task status");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("open-task run should remain latest after stale closed-task run is skipped");
        assert_eq!(latest.run_id, "run-active-open-task");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn completed_run_status_is_downgraded_by_exception_recorded_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-completed-over-exception-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-exception",
            "closure",
            "delivery",
        );
        status.task_id = "task-closure-exception".to_string();
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure-exception".to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-1".to_string()),
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: None,
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist exception-recorded dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure-exception")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_blocked");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn closure_dispatch_executed_receipt_reconciles_run_to_closure_complete() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closure-dispatch-executed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-direct",
            "closure",
            "delivery",
        );
        status.task_id = "task-closure-direct".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked closure status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure-direct".to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: Some(
                    "vida taskflow consume continue --run-id run-closure-direct --json".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist closure dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure-direct")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn executed_specification_receipt_with_design_gate_blockers_clears_fake_delegated_lane_active(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-spec-design-gate-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let status = RunGraphStatus {
            run_id: "run-spec-design-gate".to_string(),
            task_id: "task-spec-design-gate".to_string(),
            task_class: "implementation".to_string(),
            active_node: "specification".to_string(),
            next_node: None,
            status: "running".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "specification_lane".to_string(),
            lifecycle_stage: "specification_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist specification-active run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-spec-design-gate".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure".to_string()),
                downstream_dispatch_note: Some(
                    "finalize the design doc and close spec-pack before work-pool shaping"
                        .to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                downstream_dispatch_trace_path: Some("/tmp/specification-trace.json".to_string()),
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist executed specification receipt");

        let reconciled = store
            .run_graph_status("run-spec-design-gate")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "specification");
        assert_eq!(reconciled.lifecycle_stage, "specification_complete");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        assert!(!reconciled.delegation_gate().delegated_cycle_open);
        assert_eq!(
            reconciled.delegation_gate().delegated_cycle_state,
            "clear".to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn retry_ready_internal_activation_receipt_preserves_dispatch_ready_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-retry-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let status = RunGraphStatus {
            run_id: "run-retry-ready".to_string(),
            task_id: "task-retry-ready".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "implementer_lane".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer_lane".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-retry-ready".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-timeout".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: Some("internal_activation_view_only".to_string()),
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
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist blocked retry receipt");

        let reconciled = store
            .run_graph_status("run-retry-ready")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node.as_deref(), Some("implementer"));
        assert_eq!(reconciled.handoff_state, "awaiting_implementer");
        assert_eq!(reconciled.resume_target, "dispatch.implementer_lane");
        assert!(reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn stale_executing_receipt_is_normalized_on_read_and_clears_open_cycle() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-read-normalize-stale-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-read-normalize-stale",
            "verification",
            "delivery",
        );
        status.task_id = "task-read-normalize-stale".to_string();
        status.active_node = "verification".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "verification_active".to_string();
        status.next_node = None;
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "run_id": "run-read-normalize-stale",
                "dispatch_target": "verification"
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-read-normalize-stale".to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-timeout".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
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
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-19T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale receipt");

        let reconciled = store
            .run_graph_status("run-read-normalize-stale")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "verification");
        assert_eq!(reconciled.lifecycle_stage, "verification_blocked");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        assert!(!reconciled.delegation_gate().delegated_cycle_open);

        let persisted = store
            .run_graph_dispatch_receipt("run-read-normalize-stale")
            .await
            .expect("read persisted normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_dispatch_timeout_without_receipt")
        );
        assert_eq!(persisted.lane_status, "lane_exception_recorded");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_preserves_ready_handoff_against_stale_downstream_blockers() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-ready-handoff-stale-downstream-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-ready-handoff",
            "implementation",
            "implementation",
        );
        status.task_id = "task-ready-handoff".to_string();
        status.active_node = "analysis".to_string();
        status.next_node = Some("writer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "analysis_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "awaiting_writer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.writer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist ready writer handoff");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-ready-handoff".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("writer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some(
                    "stale preview before planner metadata repair".to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-12T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale downstream receipt");

        let reconciled = store
            .run_graph_status("run-ready-handoff")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "analysis");
        assert_eq!(reconciled.next_node.as_deref(), Some("writer"));
        assert_eq!(reconciled.lifecycle_stage, "analysis_active");
        assert_eq!(reconciled.handoff_state, "awaiting_writer");
        assert_eq!(reconciled.resume_target, "dispatch.writer_lane");
        assert!(reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn blocked_analysis_status_with_missing_owned_scope_reconciles_to_writer_handoff() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-missing-scope-handoff-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-missing-scope-handoff",
            "implementation",
            "implementation",
        );
        status.task_id = "task-missing-scope-handoff".to_string();
        status.active_node = "analysis".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked analysis status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-missing-scope-handoff".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida lane complete".to_string()),
                dispatch_command: Some("vida lane complete".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("writer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("activate writer".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist completed analysis receipt");

        let reconciled = store
            .run_graph_status("run-missing-scope-handoff")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "analysis");
        assert_eq!(reconciled.next_node.as_deref(), Some("writer"));
        assert_eq!(reconciled.lifecycle_stage, "analysis_active");
        assert_eq!(reconciled.handoff_state, "awaiting_writer");
        assert_eq!(reconciled.resume_target, "dispatch.writer_lane");
        assert!(reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn executed_activation_view_only_receipt_is_normalized_to_blocked_retry_truth() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-activation-view-only-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-activation-view-only",
            "implementation",
            "implementation",
        );
        status.task_id = "task-activation-view-only".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run graph status");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-activation-view-only.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "blocker_code": "internal_activation_view_only",
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-activation-view-only".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale coach handoff".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale executed receipt");

        let reconciled = store
            .run_graph_status("run-activation-view-only")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node.as_deref(), Some("implementer"));
        assert_eq!(reconciled.handoff_state, "awaiting_implementer");
        assert_eq!(reconciled.resume_target, "dispatch.implementer_lane");
        assert!(reconciled.recovery_ready);

        let persisted = store
            .run_graph_dispatch_receipt("run-activation-view-only")
            .await
            .expect("load normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert!(!persisted.downstream_dispatch_ready);
        assert!(persisted.downstream_dispatch_status.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn executing_activation_view_only_blocked_result_is_normalized_to_blocked_truth() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-terminal-activation-view-only-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-activation-view-only",
            "implementation",
            "implementation",
        );
        status.task_id = "task-terminal-activation-view-only".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run graph status");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-terminal-activation-view-only.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-terminal-activation-view-only".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale coach handoff".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale executing receipt");

        let reconciled = store
            .run_graph_status("run-terminal-activation-view-only")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let persisted = store
            .run_graph_dispatch_receipt("run-terminal-activation-view-only")
            .await
            .expect("load normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert_eq!(persisted.lane_status, "lane_blocked");
        assert!(!persisted.downstream_dispatch_ready);
        assert!(persisted.downstream_dispatch_status.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn closed_task_does_not_override_exception_recorded_run_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closed-task-exception-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-exception",
                title: "Closed task with exception-backed closure receipt",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closed-exception",
            "closure",
            "delivery",
        );
        status.task_id = "task-closed-exception".to_string();
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closed-exception".to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-closed-1".to_string()),
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: None,
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist exception-recorded dispatch receipt");

        store
            .close_task("task-closed-exception", "exception path recorded")
            .await
            .expect("close task");

        let reconciled = store
            .run_graph_status("run-closed-exception")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
        assert!(!reconciled.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn terminal_task_active_projection_ignores_exception_takeover_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-terminal-task-active-exception-takeover-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-exception-takeover",
                title: "Closed task with active exception takeover",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closed-exception-takeover",
            "implementation",
            "implementation",
        );
        status.task_id = "task-closed-exception-takeover".to_string();
        status.active_node = "test_author".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "dispatch.test_author".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run-graph status");

        let mut receipt = sample_dispatch_receipt("run-closed-exception-takeover");
        receipt.dispatch_target = "test_author".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("pending_test_author_evidence".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");

        store
            .close_task(
                "task-closed-exception-takeover",
                "superseded by exception takeover",
            )
            .await
            .expect("close task");

        let terminal_active = store
            .latest_terminal_task_active_run_graph_status()
            .await
            .expect("load terminal task active projection");
        assert!(
            terminal_active.is_none(),
            "exception takeover receipt should keep closed task out of active projection"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_ignores_newer_automatic_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-upstream",
                title: "Upstream task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create upstream task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-upstream".to_string(),
                task_id: "task-upstream".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-upstream",
                    "run_id": "run-upstream",
                    "task_status": "in_progress",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "operator rebound work to the upstream task".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T09:00:00Z".to_string(),
            })
            .await
            .expect("record explicit binding");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-child".to_string(),
                task_id: "run-child".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": "run-child",
                    "run_id": "run-child",
                    "active_node": "implementer"
                }),
                binding_source: "run_graph_advance".to_string(),
                why_this_unit: "stale automatic child continuation".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:00:00Z".to_string(),
            })
            .await
            .expect("record automatic binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("explicit binding should exist");

        assert_eq!(latest.run_id, "run-upstream");
        assert_eq!(latest.binding_source, "explicit_continuation_bind_task");
        assert_eq!(latest.active_bounded_unit["kind"], "task_graph_task");
        assert_eq!(latest.active_bounded_unit["task_status"], "in_progress");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_skips_completed_bound_task() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-completed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-completed-binding",
                title: "Completed explicit binding task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create completed task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-completed-binding".to_string(),
                task_id: "task-completed-binding".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-completed-binding",
                    "run_id": "run-completed-binding",
                    "task_status": "completed",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale explicit completed task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T09:00:00Z".to_string(),
            })
            .await
            .expect("record completed explicit binding");

        assert!(store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn active_exception_takeover_reconciles_stale_continuation_binding_for_next_lawful_sources(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-active-exception-takeover-reconciles-next-lawful-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "run-next-lawful-stale",
                title: "Next lawful stale binding task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-next-lawful-stale",
            "scope_discussion",
            "spec-pack",
        );
        status.task_id = "run-next-lawful-stale".to_string();
        status.active_node = "planning".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "planning_blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: status.run_id.clone(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("exc-next-lawful-stale".to_string()),
                exception_path_receipt_id: Some("exc-next-lawful-stale".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
                downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
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
                recorded_at: "2026-05-06T11:58:00Z".to_string(),
            })
            .await
            .expect("record exception takeover receipt");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": "planning"
                }),
                binding_source: "explicit_continuation_bind".to_string(),
                why_this_unit: "stale operator binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-05-06T11:50:00Z".to_string(),
            })
            .await
            .expect("record stale explicit binding");

        let current = store
            .run_graph_continuation_binding("run-next-lawful-stale")
            .await
            .expect("read current binding")
            .expect("current binding should exist");
        let explicit = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("latest explicit binding should exist");

        for binding in [current, explicit] {
            assert_eq!(
                binding.binding_source,
                "latest_run_graph_exception_takeover_dispatch"
            );
            assert_eq!(binding.active_bounded_unit["active_node"], "specification");
            assert_eq!(
                binding.sequential_vs_parallel_posture,
                "sequential_only_exception_takeover"
            );
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_skips_closed_task_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-skips-closed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed".to_string(),
                task_id: "task-closed".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-closed",
                    "run_id": "run-closed",
                    "task_status": "open",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale explicit task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:00:00Z".to_string(),
            })
            .await
            .expect("record closed explicit binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding");

        assert!(
            latest.is_none(),
            "closed explicit task binding must be skipped"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_clears_closed_task_graph_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-continuation-binding-clears-closed-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-direct",
                title: "Closed direct task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed direct task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed-direct".to_string(),
                task_id: "task-closed-direct".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-closed-direct",
                    "run_id": "run-closed-direct",
                    "task_status": "open",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale direct task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:30:00Z".to_string(),
            })
            .await
            .expect("record closed direct binding");

        let binding = store
            .run_graph_continuation_binding("run-closed-direct")
            .await
            .expect("read direct binding");

        assert!(
            binding.is_none(),
            "closed task_graph_task binding must not remain active"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_includes_task_close_reconcile() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-task-close-reconcile-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closure".to_string(),
                task_id: "task-closure".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "downstream_dispatch_target",
                    "task_id": "task-closure",
                    "run_id": "run-closure",
                    "dispatch_target": "closure"
                }),
                binding_source: "task_close_reconcile".to_string(),
                why_this_unit: "task close rebound the run to closure".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T11:00:00Z".to_string(),
            })
            .await
            .expect("record task-close reconcile binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("task-close reconcile binding should be returned");

        assert_eq!(latest.run_id, "run-closure");
        assert_eq!(latest.binding_source, "task_close_reconcile");
        assert_eq!(
            latest.active_bounded_unit["dispatch_target"],
            serde_json::Value::String("closure".to_string())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_keeps_task_close_reconcile_fail_closed_when_run_is_open(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-continuation-binding-normalizes-task-close-reconcile-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");
        let mut status = sample_run_graph_status();
        status.run_id = "run-closed".to_string();
        status.task_id = "task-closed".to_string();
        status.active_node = "analysis".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "analysis_active".to_string();
        status.policy_gate = "owned_scope_required".to_string();
        status.handoff_state = "awaiting_implementation_dispatch".to_string();
        status.resume_target = "dispatch.implementation_dispatch_ready".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("record open analysis run status");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed".to_string(),
                task_id: "task-closed".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": "task-closed",
                    "run_id": "run-closed",
                    "active_node": "implementer"
                }),
                binding_source: "task_close_reconcile".to_string(),
                why_this_unit: "stale task-close reconcile binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T12:00:00Z".to_string(),
            })
            .await
            .expect("record stale task-close reconcile binding");

        let binding = store
            .run_graph_continuation_binding("run-closed")
            .await
            .expect("read normalized binding");
        assert!(
            binding.is_none(),
            "stale task-close reconcile binding should fail closed when the run remains open"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_round_trips_latest_record_for_run_id() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-round-trip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let older = sample_replay_lineage_receipt(
            "receipt-replay-lineage-1",
            "run-replay-lineage",
            "2026-04-20T10:00:00Z",
        );
        let newer = sample_replay_lineage_receipt(
            "receipt-replay-lineage-2",
            "run-replay-lineage",
            "2026-04-20T10:05:00Z",
        );

        store
            .record_run_graph_replay_lineage_receipt(&older)
            .await
            .expect("persist older replay lineage receipt");
        store
            .record_run_graph_replay_lineage_receipt(&newer)
            .await
            .expect("persist newer replay lineage receipt");

        let loaded = store
            .run_graph_replay_lineage_receipt("run-replay-lineage")
            .await
            .expect("load latest replay lineage receipt")
            .expect("replay lineage receipt should exist");
        assert_eq!(loaded.receipt_id, "receipt-replay-lineage-2");
        assert_eq!(loaded.run_id, "run-replay-lineage");
        assert_eq!(loaded.resolved_dispatch_target, "closure");
        assert_eq!(loaded.validation_outcome, "pass");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_replay_lineage_receipt_uses_latest_status_run_id() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-replay-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-replay-lineage-older".to_string();
        older_status.task_id = "task-replay-lineage-older".to_string();
        older_status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older run graph status");
        store
            .record_run_graph_replay_lineage_receipt(&sample_replay_lineage_receipt(
                "receipt-replay-lineage-older",
                "run-replay-lineage-older",
                "2026-04-20T10:00:00Z",
            ))
            .await
            .expect("persist older replay lineage receipt");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-replay-lineage-latest".to_string();
        latest_status.task_id = "task-replay-lineage-latest".to_string();
        latest_status.resume_target = "dispatch.verification_lane".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest run graph status");
        store
            .record_run_graph_replay_lineage_receipt(&sample_replay_lineage_receipt(
                "receipt-replay-lineage-latest",
                "run-replay-lineage-latest",
                "2026-04-20T10:10:00Z",
            ))
            .await
            .expect("persist latest replay lineage receipt");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest replay lineage receipt")
            .expect("latest replay lineage receipt should exist");
        assert_eq!(latest.run_id, "run-replay-lineage-latest");
        assert_eq!(latest.receipt_id, "receipt-replay-lineage-latest");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_fails_closed_on_persisted_empty_required_field() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-invalid-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-replay-lineage-invalid".to_string();
        status.task_id = "task-replay-lineage-invalid".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<RunGraphReplayLineageReceipt> = store
            .db
            .upsert((
                "run_graph_replay_lineage_receipt",
                "receipt-replay-lineage-invalid",
            ))
            .content(RunGraphReplayLineageReceipt {
                resolved_task_id: String::new(),
                ..sample_replay_lineage_receipt(
                    "receipt-replay-lineage-invalid",
                    "run-replay-lineage-invalid",
                    "2026-04-20T10:15:00Z",
                )
            })
            .await
            .expect("persist invalid replay lineage receipt");

        let error = store
            .run_graph_replay_lineage_receipt("run-replay-lineage-invalid")
            .await
            .expect_err("invalid persisted replay lineage receipt should fail closed");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("run-graph replay lineage receipt"));
                assert!(reason.contains("resolved_task_id"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        let error = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect_err("latest replay lineage receipt should fail closed for invalid latest row");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("run-graph replay lineage receipt"));
                assert!(reason.contains("resolved_task_id"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_round_trips_for_latest_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-round-trip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-replay-lineage".to_string();
        status.task_id = "task-replay-lineage".to_string();
        status.resume_target = "resume.current_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = RunGraphReplayLineageReceipt {
            receipt_id: "replay-lineage-run-replay-lineage".to_string(),
            run_id: "run-replay-lineage".to_string(),
            lineage_kind: "root_dispatch_packet".to_string(),
            replay_scope: "resume_resolution".to_string(),
            origin_checkpoint_ref: "run-replay-lineage:execution_cursor:resume.current_lane"
                .to_string(),
            fork_parent: None,
            source_dispatch_target: "implementer".to_string(),
            source_dispatch_packet_path: Some("/tmp/run-replay-lineage-packet.json".to_string()),
            source_dispatch_result_path: Some("/tmp/run-replay-lineage-result.json".to_string()),
            resolved_dispatch_target: "implementer".to_string(),
            resolved_task_id: "task-replay-lineage".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "resume.current_lane".to_string(),
            validation_outcome: "lawful_resume".to_string(),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_replay_lineage_receipt(&receipt)
            .await
            .expect("persist replay lineage receipt");

        let by_run = store
            .run_graph_replay_lineage_receipt("run-replay-lineage")
            .await
            .expect("load replay lineage receipt by run")
            .expect("receipt exists");
        assert_eq!(by_run.receipt_id, receipt.receipt_id);
        assert_eq!(by_run.origin_checkpoint_ref, receipt.origin_checkpoint_ref);
        assert_eq!(by_run.resolved_task_id, "task-replay-lineage");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest replay lineage receipt")
            .expect("latest receipt exists");
        assert_eq!(latest.run_id, "run-replay-lineage");
        assert_eq!(latest.lineage_kind, "root_dispatch_packet");
        assert_eq!(latest.resume_target, "resume.current_lane");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_replay_lineage_receipt_ignores_older_run_receipts() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-latest-scope-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-replay-old".to_string();
        older_status.task_id = "task-replay-old".to_string();
        older_status.resume_target = "resume.old".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older status");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-replay-new".to_string();
        latest_status.task_id = "task-replay-new".to_string();
        latest_status.resume_target = "resume.new".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let older_receipt = RunGraphReplayLineageReceipt {
            receipt_id: "replay-lineage-run-replay-old".to_string(),
            run_id: "run-replay-old".to_string(),
            lineage_kind: "downstream_packet".to_string(),
            replay_scope: "resume_resolution".to_string(),
            origin_checkpoint_ref: "run-replay-old:execution_cursor:resume.old".to_string(),
            fork_parent: None,
            source_dispatch_target: "implementer".to_string(),
            source_dispatch_packet_path: Some("/tmp/run-replay-old-packet.json".to_string()),
            source_dispatch_result_path: None,
            resolved_dispatch_target: "coach".to_string(),
            resolved_task_id: "task-replay-old".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "resume.old".to_string(),
            validation_outcome: "lawful_resume".to_string(),
            recorded_at: "2026-04-20T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_replay_lineage_receipt(&older_receipt)
            .await
            .expect("persist older replay lineage receipt");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest scoped replay lineage receipt");
        assert!(
            latest.is_none(),
            "latest run should not inherit replay lineage receipt from older run"
        );

        let older = store
            .run_graph_replay_lineage_receipt("run-replay-old")
            .await
            .expect("load older replay lineage receipt")
            .expect("older receipt exists");
        assert_eq!(older.receipt_id, "replay-lineage-run-replay-old");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn record_run_graph_status_appends_projection_checkpoint_records_for_same_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-history-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-projection-checkpoint".to_string();
        status.task_id = "task-projection-checkpoint".to_string();
        status.resume_target = "dispatch.writer_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist first status");

        status.active_node = "coach".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist second status");

        let mut query = store
            .db
            .query(
                "SELECT * FROM run_graph_projection_checkpoint_record \
                 WHERE run_id = $run_id \
                 ORDER BY updated_at ASC, record_id ASC;",
            )
            .bind(("run_id", "run-projection-checkpoint".to_string()))
            .await
            .expect("load projection checkpoint history");
        let rows: Vec<RunGraphProjectionCheckpointRecord> =
            query.take(0).expect("decode projection checkpoint rows");
        assert_eq!(
            rows.len(),
            2,
            "status writes must append checkpoint records"
        );
        assert_ne!(rows[0].record_id, rows[1].record_id);
        assert_eq!(rows[0].projector_id, "taskflow.run_graph.status_projection");
        assert_eq!(
            rows[1].checkpoint_group,
            "run_graph_status:run-projection-checkpoint"
        );
        assert_eq!(rows[1].last_gapless_position, rows[1].updated_at);
        assert_eq!(rows[1].lineage_kind, "live_status_projection");

        let latest = store
            .run_graph_projection_checkpoint_record("run-projection-checkpoint")
            .await
            .expect("load latest projection checkpoint record")
            .expect("latest projection checkpoint record exists");
        assert_eq!(latest.last_gapless_position, latest.updated_at);
        assert_eq!(
            latest.origin_checkpoint_ref,
            "run-projection-checkpoint:execution_cursor:dispatch.coach_lane"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_projection_checkpoint_record_scopes_to_latest_status_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-projection-old".to_string();
        older_status.task_id = "task-projection-old".to_string();
        older_status.resume_target = "dispatch.old".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older status");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-projection-new".to_string();
        latest_status.task_id = "task-projection-new".to_string();
        latest_status.resume_target = "dispatch.new".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let latest = store
            .latest_run_graph_projection_checkpoint_record()
            .await
            .expect("load latest projection checkpoint record")
            .expect("latest projection checkpoint record exists");
        assert_eq!(latest.run_id, "run-projection-new");
        assert_eq!(latest.last_gapless_position, latest.updated_at);
        assert_eq!(latest.projector_id, "taskflow.run_graph.status_projection");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn record_run_graph_status_skips_projection_checkpoint_record_when_checkpoint_kind_is_none(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-skip-none-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-projection-none".to_string();
        status.task_id = "task-projection-none".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist status without checkpoint lineage");

        let latest = store
            .run_graph_projection_checkpoint_record("run-projection-none")
            .await
            .expect("load projection checkpoint record");
        assert!(
            latest.is_none(),
            "checkpoint placeholder state must not emit a projection checkpoint record"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
