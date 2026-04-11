use super::*;

#[derive(Debug)]
pub struct RunGraphSummary {
    pub execution_plan_count: usize,
    pub routed_run_count: usize,
    pub governance_count: usize,
    pub resumability_count: usize,
    pub reconciliation_count: usize,
}

impl RunGraphSummary {
    pub fn as_display(&self) -> String {
        format!(
            "execution_plans={}, routed_runs={}, governance={}, resumability={}, reconciliation={}",
            self.execution_plan_count,
            self.routed_run_count,
            self.governance_count,
            self.resumability_count,
            self.reconciliation_count
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct ExecutionPlanStateRow {
    pub(crate) run_id: String,
    pub(crate) task_id: String,
    pub(crate) task_class: String,
    pub(crate) active_node: String,
    pub(crate) next_node: Option<String>,
    pub(crate) status: String,
    pub(crate) updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct RoutedRunStateRow {
    pub(crate) run_id: String,
    pub(crate) route_task_class: String,
    pub(crate) selected_backend: String,
    pub(crate) lane_id: String,
    pub(crate) lifecycle_stage: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
pub(crate) struct RunGraphLatestRow {
    pub(crate) run_id: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct GovernanceStateRow {
    pub(crate) run_id: String,
    pub(crate) policy_gate: String,
    pub(crate) handoff_state: String,
    pub(crate) context_state: String,
    pub(crate) updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct ResumabilityCapsuleRow {
    pub(crate) run_id: String,
    pub(crate) checkpoint_kind: String,
    pub(crate) resume_target: String,
    pub(crate) recovery_ready: bool,
    pub(crate) updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct RunGraphDispatchReceipt {
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    #[serde(
        default = "default_run_graph_lane_status",
        deserialize_with = "deserialize_run_graph_lane_status"
    )]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct RunGraphContinuationBinding {
    pub run_id: String,
    pub task_id: String,
    pub status: String,
    pub active_bounded_unit: serde_json::Value,
    pub binding_source: String,
    pub why_this_unit: String,
    pub primary_path: String,
    pub sequential_vs_parallel_posture: String,
    pub request_text: Option<String>,
    pub recorded_at: String,
}

impl RunGraphContinuationBinding {
    pub(crate) fn validate(&self) -> Result<(), StateStoreError> {
        if self.run_id.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "run-graph continuation binding run_id must be non-empty".to_string(),
            });
        }
        if self.task_id.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph continuation binding for `{}` must have non-empty task_id",
                    self.run_id
                ),
            });
        }
        if self.status != "bound" {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph continuation binding for `{}` must have status `bound`, got `{}`",
                    self.run_id, self.status
                ),
            });
        }
        if !self.active_bounded_unit.is_object() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph continuation binding for `{}` must have object active_bounded_unit",
                    self.run_id
                ),
            });
        }
        for (field, value) in [
            ("binding_source", self.binding_source.as_str()),
            ("why_this_unit", self.why_this_unit.as_str()),
            ("primary_path", self.primary_path.as_str()),
            (
                "sequential_vs_parallel_posture",
                self.sequential_vs_parallel_posture.as_str(),
            ),
            ("recorded_at", self.recorded_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "run-graph continuation binding for `{}` has empty field `{field}`",
                        self.run_id
                    ),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct RunGraphDispatchContext {
    pub run_id: String,
    pub task_id: String,
    pub request_text: String,
    pub role_selection: serde_json::Value,
    pub recorded_at: String,
}

impl RunGraphDispatchContext {
    pub(crate) fn validate(&self) -> Result<(), StateStoreError> {
        if self.run_id.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "run-graph dispatch context run_id must be non-empty".to_string(),
            });
        }
        if self.task_id.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch context for `{}` must have non-empty task_id",
                    self.run_id
                ),
            });
        }
        if self.request_text.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch context for `{}` must have non-empty request_text",
                    self.run_id
                ),
            });
        }
        if self.recorded_at.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch context for `{}` must have non-empty recorded_at",
                    self.run_id
                ),
            });
        }
        if !self.role_selection.is_object() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch context for `{}` must have object role_selection",
                    self.run_id
                ),
            });
        }
        serde_json::from_value::<crate::RuntimeConsumptionLaneSelection>(
            self.role_selection.clone(),
        )
        .map_err(|error| StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph dispatch context for `{}` has invalid role_selection: {error}",
                self.run_id
            ),
        })?;
        Ok(())
    }

    pub(crate) fn role_selection(
        &self,
    ) -> Result<crate::RuntimeConsumptionLaneSelection, StateStoreError> {
        serde_json::from_value(self.role_selection.clone()).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch context for `{}` has invalid role_selection: {error}",
                    self.run_id
                ),
            }
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct RunGraphDispatchReceiptStored {
    pub(crate) run_id: String,
    pub(crate) dispatch_target: String,
    pub(crate) dispatch_status: String,
    pub(crate) lane_status: Option<String>,
    pub(crate) supersedes_receipt_id: Option<String>,
    pub(crate) exception_path_receipt_id: Option<String>,
    pub(crate) dispatch_kind: String,
    pub(crate) dispatch_surface: Option<String>,
    pub(crate) dispatch_command: Option<String>,
    pub(crate) dispatch_packet_path: Option<String>,
    pub(crate) dispatch_result_path: Option<String>,
    pub(crate) blocker_code: Option<String>,
    pub(crate) downstream_dispatch_target: Option<String>,
    pub(crate) downstream_dispatch_command: Option<String>,
    pub(crate) downstream_dispatch_note: Option<String>,
    pub(crate) downstream_dispatch_ready: bool,
    pub(crate) downstream_dispatch_blockers: Vec<String>,
    pub(crate) downstream_dispatch_packet_path: Option<String>,
    pub(crate) downstream_dispatch_status: Option<String>,
    pub(crate) downstream_dispatch_result_path: Option<String>,
    pub(crate) downstream_dispatch_trace_path: Option<String>,
    pub(crate) downstream_dispatch_executed_count: u32,
    pub(crate) downstream_dispatch_active_target: Option<String>,
    pub(crate) downstream_dispatch_last_target: Option<String>,
    pub(crate) activation_agent_type: Option<String>,
    pub(crate) activation_runtime_role: Option<String>,
    pub(crate) selected_backend: Option<String>,
    pub(crate) recorded_at: String,
}

impl From<RunGraphDispatchReceiptStored> for RunGraphDispatchReceipt {
    fn from(stored: RunGraphDispatchReceiptStored) -> Self {
        let normalized_lane_status = normalize_run_graph_lane_status(
            stored.lane_status.as_deref(),
            &stored.dispatch_status,
            stored.supersedes_receipt_id.as_deref(),
            stored.exception_path_receipt_id.as_deref(),
        );
        Self {
            run_id: stored.run_id,
            dispatch_target: stored.dispatch_target,
            dispatch_status: stored.dispatch_status,
            lane_status: normalized_lane_status,
            supersedes_receipt_id: stored.supersedes_receipt_id,
            exception_path_receipt_id: stored.exception_path_receipt_id,
            dispatch_kind: stored.dispatch_kind,
            dispatch_surface: stored.dispatch_surface,
            dispatch_command: stored.dispatch_command,
            dispatch_packet_path: stored.dispatch_packet_path,
            dispatch_result_path: stored.dispatch_result_path,
            blocker_code: stored.blocker_code,
            downstream_dispatch_target: stored.downstream_dispatch_target,
            downstream_dispatch_command: stored.downstream_dispatch_command,
            downstream_dispatch_note: stored.downstream_dispatch_note,
            downstream_dispatch_ready: stored.downstream_dispatch_ready,
            downstream_dispatch_blockers: stored.downstream_dispatch_blockers,
            downstream_dispatch_packet_path: stored.downstream_dispatch_packet_path,
            downstream_dispatch_status: stored.downstream_dispatch_status,
            downstream_dispatch_result_path: stored.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: stored.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: stored.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: stored.downstream_dispatch_active_target,
            downstream_dispatch_last_target: stored.downstream_dispatch_last_target,
            activation_agent_type: stored.activation_agent_type,
            activation_runtime_role: stored.activation_runtime_role,
            selected_backend: stored.selected_backend,
            recorded_at: stored.recorded_at,
        }
    }
}

impl From<RunGraphDispatchReceipt> for RunGraphDispatchReceiptStored {
    fn from(receipt: RunGraphDispatchReceipt) -> Self {
        let lane_status = if receipt.lane_status.is_empty() {
            None
        } else {
            Some(receipt.lane_status)
        };
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
            blocker_code: receipt.blocker_code,
            downstream_dispatch_target: receipt.downstream_dispatch_target,
            downstream_dispatch_command: receipt.downstream_dispatch_command,
            downstream_dispatch_note: receipt.downstream_dispatch_note,
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_blockers: receipt.downstream_dispatch_blockers,
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
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunGraphStatus {
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub route_task_class: String,
    pub selected_backend: String,
    pub lane_id: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

#[allow(dead_code)]
impl RunGraphStatus {
    pub(crate) fn validate_memory_governance(&self) -> Result<(), StateStoreError> {
        if !requires_memory_governance_enforcement(&self.policy_gate) {
            return Ok(());
        }
        if self.context_state != "sealed" {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "memory governance evidence shaping required for policy_gate `{}`: context_state must be `sealed`, got `{}`",
                    self.policy_gate, self.context_state
                ),
            });
        }
        if !handoff_state_links_consent_ttl(&self.handoff_state) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "memory governance linkage required for policy_gate `{}`: handoff_state must link consent+ttl, got `{}`",
                    self.policy_gate, self.handoff_state
                ),
            });
        }
        Ok(())
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} class={} node={} status={} next={} route={} backend={} lane={} lifecycle={} gate={} handoff={} context={} checkpoint={} resume_target={} recovery_ready={}",
            self.run_id,
            self.task_id,
            self.task_class,
            self.active_node,
            self.status,
            self.next_node.as_deref().unwrap_or("none"),
            self.route_task_class,
            self.selected_backend,
            self.lane_id,
            self.lifecycle_stage,
            self.policy_gate,
            self.handoff_state,
            self.context_state,
            self.checkpoint_kind,
            self.resume_target,
            self.recovery_ready
        )
    }

    pub fn delegation_gate(&self) -> RunGraphDelegationGateSummary {
        RunGraphDelegationGateSummary::from_status(self)
    }
}
