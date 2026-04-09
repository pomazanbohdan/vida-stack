use super::*;

pub(super) fn reconcile_run_graph_status_with_dispatch_receipt(
    mut status: RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> Result<RunGraphStatus, StateStoreError> {
    let Some(receipt) = receipt else {
        return Ok(status);
    };
    let receipt = StateStore::validate_run_graph_dispatch_receipt_contract(receipt.clone())?;
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
            canonical_lane_status_str(&receipt.lane_status)
                .unwrap_or(receipt.lane_status.as_str())
                .to_string()
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
            recorded_at: receipt.recorded_at,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} target={} status={} lane_status={} supersedes_receipt_id={} exception_path_receipt_id={} blocker_code={} kind={} surface={} command={} packet={} result={} next_target={} next_command={} next_note={} next_ready={} next_blockers={} next_packet={} next_status={} next_result={} next_trace={} next_count={} next_last_target={} agent={} runtime_role={} backend={} recorded_at={}",
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
            self.downstream_dispatch_command.as_deref().unwrap_or("none"),
            self.downstream_dispatch_note.as_deref().unwrap_or("none"),
            self.downstream_dispatch_ready,
            if self.downstream_dispatch_blockers.is_empty() {
                "none".to_string()
            } else {
                self.downstream_dispatch_blockers.join("|")
            },
            self.downstream_dispatch_packet_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_status.as_deref().unwrap_or("none"),
            self.downstream_dispatch_result_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_trace_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_executed_count,
            self.downstream_dispatch_last_target.as_deref().unwrap_or("none"),
            self.activation_agent_type.as_deref().unwrap_or("none"),
            self.activation_runtime_role.as_deref().unwrap_or("none"),
            self.selected_backend.as_deref().unwrap_or("none"),
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

pub(crate) fn ensure_run_graph_approval_delegation_receipt_consistency(
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
        "packet_ready" | "routed" | "executed" | "blocked"
    ) && receipt.lane_status.as_str()
        != derive_lane_status(
            &receipt.dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        || !matches!(
            receipt.dispatch_status.as_str(),
            "packet_ready" | "routed" | "executed" | "blocked"
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
            let canonical_lane_status = canonical_lane_status_str(raw).unwrap_or(raw);
            if canonical_lane_status == derived_lane_status {
                canonical_lane_status.to_string()
            } else {
                derived_lane_status
            }
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
