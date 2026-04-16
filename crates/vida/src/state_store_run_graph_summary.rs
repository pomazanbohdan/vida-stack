use super::*;
use crate::release1_contracts::lane_status_has_required_evidence;
use crate::taskflow_run_graph::{
    approval_delegation_transition_kind, is_dispatch_resume_handoff_complete,
};

fn reconcile_run_graph_status_with_dispatch_receipt(
    mut status: RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> Result<RunGraphStatus, StateStoreError> {
    let Some(receipt) = receipt else {
        return Ok(status);
    };
    let receipt = StateStore::validate_run_graph_dispatch_receipt_contract(receipt.clone())?;
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
        || !receipt.downstream_dispatch_blockers.is_empty();
    let spec_post_design_gate_blocked = receipt.dispatch_status == "executed"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
    if blocked_receipt {
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
        }
        status.status = "blocked".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    if status.status == "completed" {
        return Ok(status);
    }
    let closure_candidate = receipt.downstream_dispatch_target.as_deref() == Some("closure")
        && receipt.downstream_dispatch_ready
        && receipt.downstream_dispatch_blockers.is_empty()
        && matches!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready") | Some("executed")
        );
    if !closure_candidate {
        return Ok(status);
    }

    if let Some(last_target) = receipt.downstream_dispatch_last_target.as_deref() {
        if !last_target.trim().is_empty() {
            status.active_node = last_target.to_string();
        }
    }
    status.next_node = None;
    status.status = "completed".to_string();
    status.lifecycle_stage = "implementation_complete".to_string();
    status.policy_gate = "not_required".to_string();
    status.handoff_state = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    Ok(status)
}

fn reconcile_run_graph_status_with_closed_task(
    mut status: RunGraphStatus,
    task: Option<&TaskRecord>,
) -> RunGraphStatus {
    let Some(task) = task else {
        return status;
    };
    if task.status != "closed"
        || matches!(status.status.as_str(), "completed" | "blocked" | "failed")
    {
        return status;
    }

    status.next_node = None;
    status.status = "completed".to_string();
    status.lifecycle_stage = "implementation_complete".to_string();
    status.policy_gate = "not_required".to_string();
    status.handoff_state = "none".to_string();
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
        let (delegated_cycle_open, delegated_cycle_state) = if handoff_pending {
            (true, "handoff_pending".to_string())
        } else if delegated_lane_active {
            (true, "delegated_lane_active".to_string())
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

#[allow(dead_code)]
impl RunGraphDispatchReceiptSummary {
    pub(crate) fn from_receipt(receipt: RunGraphDispatchReceipt) -> Self {
        let lane_status = if receipt.lane_status.trim().is_empty() {
            derive_lane_status(
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
            .as_str()
            .to_string()
        } else {
            normalize_run_graph_lane_status(
                Some(receipt.lane_status.as_str()),
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

#[allow(dead_code)]
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

impl StateStore {
    pub async fn run_graph_summary(&self) -> Result<RunGraphSummary, StateStoreError> {
        Ok(RunGraphSummary {
            execution_plan_count: self.count_table_rows("execution_plan_state").await?,
            routed_run_count: self.count_table_rows("routed_run_state").await?,
            governance_count: self.count_table_rows("governance_state").await?,
            resumability_count: self.count_table_rows("resumability_capsule").await?,
            reconciliation_count: self.count_table_rows("task_reconciliation_summary").await?,
        })
    }

    #[allow(dead_code)]
    pub async fn record_run_graph_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        status.validate_memory_governance()?;
        let updated_at = unix_timestamp_nanos().to_string();
        let receipt_recorded_at = updated_at.clone();
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
        if let Some(transition_kind) = approval_delegation_transition_kind(status) {
            let receipt = RunGraphApprovalDelegationReceipt::from_status(
                status,
                transition_kind,
                receipt_recorded_at,
            );
            self.record_run_graph_approval_delegation_receipt(&receipt)
                .await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn record_run_graph_dispatch_receipt(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        let receipt: RunGraphDispatchReceiptStored = receipt.clone().into();
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .upsert(("run_graph_dispatch_receipt", receipt.run_id.as_str()))
            .content(receipt)
            .await?;
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
        Ok(())
    }

    pub async fn record_run_graph_continuation_binding(
        &self,
        binding: &RunGraphContinuationBinding,
    ) -> Result<(), StateStoreError> {
        binding.validate()?;
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .upsert(("run_graph_continuation_binding", binding.run_id.as_str()))
            .content(binding.clone())
            .await?;
        Ok(())
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
                binding.validate()?;
                Ok(Some(binding))
            }
            None => Ok(None),
        }
    }

    pub async fn latest_explicit_run_graph_continuation_binding(
        &self,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_continuation_binding \
                 WHERE binding_source = 'explicit_continuation_bind' \
                    OR binding_source = 'explicit_continuation_bind_task' \
                 ORDER BY recorded_at DESC, run_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<RunGraphContinuationBinding> = query.take(0)?;
        match rows.into_iter().next() {
            Some(binding) => {
                binding.validate()?;
                Ok(Some(binding))
            }
            None => Ok(None),
        }
    }

    pub async fn clear_run_graph_continuation_binding(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .delete(("run_graph_continuation_binding", run_id))
            .await?;
        Ok(())
    }

    pub async fn record_run_graph_dispatch_context(
        &self,
        context: &RunGraphDispatchContext,
    ) -> Result<(), StateStoreError> {
        context.validate()?;
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

    #[allow(dead_code)]
    pub async fn run_graph_status(&self, run_id: &str) -> Result<RunGraphStatus, StateStoreError> {
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
        let task = self.show_task(&status.task_id).await.ok();
        let status = reconcile_run_graph_status_with_closed_task(status, task.as_ref());
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
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        Ok(Some(self.run_graph_status(&run_id).await?))
    }

    pub(crate) async fn latest_run_graph_run_id(&self) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
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

    async fn latest_run_graph_checkpoint_run_id(&self) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM resumability_capsule ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<RunGraphLatestRow> = query.take(0)?;
        Ok(rows.into_iter().next().map(|latest| latest.run_id))
    }

    async fn ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let latest_checkpoint_run_id = self.latest_run_graph_checkpoint_run_id().await?;
        if latest_checkpoint_run_id.as_deref() != Some(run_id) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/checkpoint summary is inconsistent for `{run_id}`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id={})",
                    latest_checkpoint_run_id.as_deref().unwrap_or("none")
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_recovery_surface_consistency(
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.resume_target.starts_with("dispatch.")
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
        let latest_checkpoint_run_id = self.latest_run_graph_checkpoint_run_id().await?;
        if latest_checkpoint_run_id.as_deref() != Some(status.run_id.as_str()) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id={})",
                    status.run_id,
                    latest_checkpoint_run_id.as_deref().unwrap_or("none")
                ),
            });
        }
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let receipt: RunGraphDispatchReceipt = receipt.into();
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
        let effective_execution_posture = {
            let mut summary = crate::runtime_dispatch_state::effective_execution_posture_summary(
                role_selection
                    .as_ref()
                    .map(|selection| &selection.execution_plan)
                    .unwrap_or(&serde_json::Value::Null),
                &receipt.dispatch_target,
                receipt.selected_backend.as_deref(),
                receipt.activation_agent_type.as_deref(),
                host_runtime.as_ref(),
                crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(&receipt),
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
                    receipt.selected_backend.as_deref(),
                )
            })
            .unwrap_or(serde_json::Value::Null);
        let activation_evidence =
            crate::runtime_dispatch_state::dispatch_activation_evidence_summary(&receipt);
        Ok(Some(
            RunGraphDispatchReceiptSummary::from_receipt(receipt)
                .with_effective_execution_posture(effective_execution_posture)
                .with_route_policy(route_policy)
                .with_activation_evidence(activation_evidence),
        ))
    }

    pub async fn latest_run_graph_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        let latest_checkpoint_run_id = self.latest_run_graph_checkpoint_run_id().await?;
        if latest_checkpoint_run_id.as_deref() != Some(status.run_id.as_str()) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt is inconsistent for `{}`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id={})",
                    status.run_id,
                    latest_checkpoint_run_id.as_deref().unwrap_or("none")
                ),
            });
        }
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        Ok(Some(receipt.into()))
    }

    pub async fn run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        self.run_graph_dispatch_receipt_stored(run_id)
            .await
            .map(|row| row.map(Into::into))
    }

    async fn run_graph_dispatch_receipt_stored(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceiptStored>, StateStoreError> {
        self.db
            .select(("run_graph_dispatch_receipt", run_id))
            .await
            .map_err(Into::into)
    }

    pub async fn latest_run_graph_recovery_summary(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_checkpoint_summary(
        &self,
    ) -> Result<Option<RunGraphCheckpointSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        Ok(Some(RunGraphCheckpointSummary::from_status(status)))
    }

    pub async fn latest_run_graph_gate_summary(
        &self,
    ) -> Result<Option<RunGraphGateSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
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
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(status)
    }

    pub async fn run_graph_checkpoint_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphCheckpointSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphCheckpointSummary::from_status(status))
    }

    pub async fn run_graph_gate_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphGateSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphGateSummary::from_status(status))
    }

    fn ensure_run_graph_dispatch_receipt_summary_consistency(
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
        let raw_lane_status = raw_lane_status.trim();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        let downstream_closure_completed = receipt.downstream_dispatch_status.as_deref()
            == Some("executed")
            && canonical_lane_status == "lane_completed";
        let effective_derived_lane_status = if downstream_closure_completed {
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

    fn validate_run_graph_dispatch_receipt_contract(
        receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn run_graph_status_reconciles_closed_active_task_into_completed_clear_cycle() {
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
            .create_task(CreateTaskRequest {
                task_id: "feature-close-dev",
                title: "Implement bounded fix",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
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
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.lifecycle_stage, "implementation_complete");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        assert!(!reconciled.delegation_gate().delegated_cycle_open);

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
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.resume_target, "none");
        assert_eq!(reconciled.selected_backend, "senior");
        assert!(!reconciled.recovery_ready);

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
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
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
            .create_task(CreateTaskRequest {
                task_id: "task-closed-exception",
                title: "Closed task with exception-backed closure receipt",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
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

        let _ = fs::remove_dir_all(&root);
    }
}
