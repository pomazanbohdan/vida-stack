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

#[derive(Debug, serde::Serialize, PartialEq, Eq, Clone)]
pub struct RunGraphPrincipalDelegationProjection {
    pub principal_id: String,
    pub principal_kind: String,
    pub principal_scope: String,
    pub delegator_ref: Option<String>,
    pub delegatee_ref: Option<String>,
    pub approval_receipt_id: Option<String>,
    pub audit_ref: Option<String>,
    pub execution_backend: Option<String>,
    pub delegation_state: String,
    pub enforcement_state: String,
    pub blocker_codes: Vec<String>,
}

impl RunGraphPrincipalDelegationProjection {
    pub fn as_display(&self) -> String {
        format!(
            "principal_id={} kind={} scope={} delegator={} delegatee={} approval_receipt_id={} audit_ref={} backend={} delegation_state={} enforcement_state={} blocker_codes={}",
            self.principal_id,
            self.principal_kind,
            self.principal_scope,
            self.delegator_ref.as_deref().unwrap_or("none"),
            self.delegatee_ref.as_deref().unwrap_or("none"),
            self.approval_receipt_id.as_deref().unwrap_or("none"),
            self.audit_ref.as_deref().unwrap_or("none"),
            self.execution_backend.as_deref().unwrap_or("none"),
            self.delegation_state,
            self.enforcement_state,
            if self.blocker_codes.is_empty() {
                "none".to_string()
            } else {
                self.blocker_codes.join(",")
            }
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, Clone)]
pub struct RunGraphMemoryGovernanceProjection {
    pub governance_required: bool,
    pub memory_class: Option<String>,
    pub sensitivity_level: Option<String>,
    pub consent_basis: Option<String>,
    pub ttl_policy: Option<String>,
    pub deletion_or_correction_ref: Option<String>,
    pub approval_receipt_id: Option<String>,
    pub enforcement_state: String,
    pub blocker_codes: Vec<String>,
}

impl RunGraphMemoryGovernanceProjection {
    pub fn as_display(&self) -> String {
        format!(
            "governance_required={} memory_class={} sensitivity={} consent_basis={} ttl_policy={} deletion_or_correction_ref={} approval_receipt_id={} enforcement_state={} blocker_codes={}",
            self.governance_required,
            self.memory_class.as_deref().unwrap_or("none"),
            self.sensitivity_level.as_deref().unwrap_or("none"),
            self.consent_basis.as_deref().unwrap_or("none"),
            self.ttl_policy.as_deref().unwrap_or("none"),
            self.deletion_or_correction_ref.as_deref().unwrap_or("none"),
            self.approval_receipt_id.as_deref().unwrap_or("none"),
            self.enforcement_state,
            if self.blocker_codes.is_empty() {
                "none".to_string()
            } else {
                self.blocker_codes.join(",")
            }
        )
    }
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

    pub fn principal_delegation_projection(
        &self,
        dispatch_summary: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
        approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
    ) -> RunGraphPrincipalDelegationProjection {
        let approval_receipt_id =
            approval_receipt.map(|receipt| receipt.receipt_id.clone()).and_then(non_empty_string);
        let dispatch_target = dispatch_summary
            .map(|summary| summary.dispatch_target.as_str())
            .filter(|value| !value.trim().is_empty());
        let delegatee_ref = self
            .next_node
            .as_deref()
            .map(|next_node| format!("node:{next_node}"))
            .or_else(|| dispatch_target.map(|target| format!("dispatch_target:{target}")))
            .or_else(|| {
                non_empty_string(self.lane_id.clone()).map(|lane_id| format!("lane:{lane_id}"))
            });
        let audit_ref = dispatch_summary
            .and_then(|summary| {
                summary
                    .dispatch_result_path
                    .clone()
                    .or_else(|| summary.dispatch_packet_path.clone())
                    .or_else(|| summary.dispatch_run_graph_trace_ref())
            })
            .or_else(|| Some(format!("run_graph:{}", self.run_id)));
        let execution_backend = dispatch_summary
            .and_then(|summary| summary.selected_backend.clone())
            .or_else(|| non_empty_string(self.selected_backend.clone()));
        let delegated_cycle_open = self.delegation_gate().delegated_cycle_open;
        let approval_transition_active = matches!(
            approval_receipt
                .as_ref()
                .map(|receipt| receipt.transition_kind.as_str()),
            Some("approval_wait" | "approval_complete")
        ) || matches!(self.status.as_str(), "awaiting_approval");
        let principal_delegation_required = delegated_cycle_open || approval_transition_active;
        let mut blocker_codes = Vec::new();
        if principal_delegation_required
            && (delegatee_ref.is_none() || audit_ref.as_deref().is_none())
        {
            blocker_codes.push(
                crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::DelegationChainBroken,
                )
                .to_string(),
            );
        }
        if approval_transition_active && approval_receipt_id.is_none() {
            blocker_codes.push(
                crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::ApprovalRequired,
                )
                .to_string(),
            );
        }
        blocker_codes =
            crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
        let delegation_state = if delegated_cycle_open {
            self.delegation_gate().delegated_cycle_state
        } else if approval_transition_active {
            "approval_gated".to_string()
        } else {
            "not_required".to_string()
        };
        let enforcement_state = if !principal_delegation_required {
            "not_required".to_string()
        } else if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        };

        RunGraphPrincipalDelegationProjection {
            principal_id: format!("run_graph:{}:task:{}", self.run_id, self.task_id),
            principal_kind: "runtime_local_bounded_principal".to_string(),
            principal_scope: self.route_task_class.clone(),
            delegator_ref: Some(format!("node:{}", self.active_node)),
            delegatee_ref,
            approval_receipt_id,
            audit_ref,
            execution_backend,
            delegation_state,
            enforcement_state,
            blocker_codes,
        }
    }

    pub fn memory_governance_projection(
        &self,
        approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
    ) -> RunGraphMemoryGovernanceProjection {
        let governance_required = requires_memory_governance_enforcement(&self.policy_gate);
        let normalized_gate = self.policy_gate.trim().to_ascii_lowercase();
        let consent_linked = self
            .handoff_state
            .trim()
            .to_ascii_lowercase()
            .contains("consent");
        let ttl_linked = self
            .handoff_state
            .trim()
            .to_ascii_lowercase()
            .contains("ttl");
        let approval_receipt_id =
            approval_receipt.map(|receipt| receipt.receipt_id.clone()).and_then(non_empty_string);
        let memory_class = if !governance_required {
            None
        } else if normalized_gate.contains("correction") {
            Some("memory_correction_request".to_string())
        } else if normalized_gate.contains("delete") || normalized_gate.contains("deletion") {
            Some("memory_deletion_request".to_string())
        } else {
            Some("governed_runtime_memory".to_string())
        };
        let sensitivity_level = governance_required.then(|| "internal_governed".to_string());
        let consent_basis = if !governance_required {
            None
        } else if consent_linked {
            Some("consent_linked_runtime_handoff".to_string())
        } else {
            None
        };
        let ttl_policy = if !governance_required {
            None
        } else if ttl_linked {
            Some("ttl_linked_runtime_handoff".to_string())
        } else {
            None
        };
        let deletion_or_correction_ref = if !governance_required {
            None
        } else if normalized_gate.contains("correction") {
            Some(format!("memory-correction-{}", self.run_id))
        } else if normalized_gate.contains("delete") || normalized_gate.contains("deletion") {
            Some(format!("memory-delete-{}", self.run_id))
        } else {
            None
        };

        let mut blocker_codes = Vec::new();
        if governance_required && self.context_state != "sealed" {
            blocker_codes.push(
                crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::PolicyContextMissing,
                )
                .to_string(),
            );
        }
        if governance_required && !handoff_state_links_consent_ttl(&self.handoff_state) {
            blocker_codes.push(
                crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::PolicyContextMissing,
                )
                .to_string(),
            );
        }
        if governance_required && approval_receipt_id.is_none() {
            blocker_codes.push(
                crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::ApprovalRequired,
                )
                .to_string(),
            );
        }
        blocker_codes =
            crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
        let enforcement_state = if !governance_required {
            "not_required".to_string()
        } else if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        };

        RunGraphMemoryGovernanceProjection {
            governance_required,
            memory_class,
            sensitivity_level,
            consent_basis,
            ttl_policy,
            deletion_or_correction_ref,
            approval_receipt_id,
            enforcement_state,
            blocker_codes,
        }
    }
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
