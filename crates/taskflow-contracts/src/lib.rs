use serde::{Deserialize, Serialize};
use taskflow_core::{IssueType, TaskId, TaskStatus, Timestamp};

pub mod artifact_kind;
pub mod blocker_code;
pub mod decision_table;
pub mod status_code;

pub use vida_contracts::operations;

pub use artifact_kind::{ArtifactKind, UnknownArtifactKind};
pub use blocker_code::{
    BlockerCode, UnknownBlockerCode, blocker_code_list_preserving_legacy,
    canonical_blocker_code_list, canonical_blocker_code_str, canonical_blocker_code_value_from_str,
    is_selected_lane_assignment_guard_blocked, is_selected_lane_runtime_assignment_truth_missing,
    selected_lane_assignment_guard_blocked, selected_lane_runtime_assignment_truth_missing,
};
pub use decision_table::{
    DECISION_TABLE_SCHEMA_VERSION, DecisionTableCondition, DecisionTableEvaluationRequest,
    DecisionTableEvaluationResponse, DecisionTableEvaluationStatus, DecisionTableInput,
    DecisionTableOperator, DecisionTableOutput, DecisionTableRule, DecisionTableValue,
    TRANSITION_CONTRACT_SCHEMA_VERSION, TransitionContractBlocker, TransitionContractBlockerCode,
    TransitionContractDecision, TransitionContractOutcome, TransitionContractStatus,
    TransitionContractStatusCode,
};
pub use status_code::{
    ApprovalStatus, LaneStatus, Release1ContractStatus, UnknownStatusCode,
    canonical_approval_status_str, canonical_lane_status_str,
    canonical_release1_contract_status_str, release1_contract_status_str,
};
pub use vida_contracts::{
    CompletionBlocker, CompletionFailureCode, CompletionOutcome, FlowStepRef, VidaAggregateRef,
    VidaApplyRequest, VidaApplyToken, VidaArtifactRef, VidaAutomationPosture, VidaCapabilityScope,
    VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaCommandRef, VidaConsistencyRequirement,
    VidaContractValidationError, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
    VidaEventCursor, VidaEventRef, VidaExternalPayload, VidaExternalPayloadKind,
    VidaExternalPayloadValidationError, VidaExternalPayloadValidationStage, VidaIdempotencyKey,
    VidaOperation, VidaOperationPosture, VidaOperationScope, VidaOperationSpec, VidaPlan,
    VidaPlanRef, VidaProjectionCheckpoint, VidaProjectionRef, VidaReceipt, VidaReceiptId,
    VidaRiskTier, VidaSchemaId, VidaSchemaKind, VidaSchemaRef, VidaSchemaRegistryEntry,
    VidaSchemaRegistrySnapshot, VidaSchemaVersion, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
    completion_outcome_schema_json, external_payload_schema_json, external_payload_schema_ref,
    mvp_operation_registry, operation_spec, parse_completion_outcome_json,
    parse_external_payload_json, runtime_envelope_schema_bundle_json,
    runtime_schema_registry_snapshot_json, trace_links_are_conformant,
    upcast_domain_event_to_latest, validate_command_envelope_domain, validate_domain_event,
    validate_external_payload_schema_value, vida_runtime_schema_registry_snapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub issue_type: IssueType,
    pub updated_at: Timestamp,
}

impl TaskRecord {
    #[must_use]
    pub fn new(id: TaskId, title: impl Into<String>, issue_type: IssueType) -> Self {
        Self {
            id,
            title: title.into(),
            status: TaskStatus::Open,
            issue_type,
            updated_at: Timestamp::now_utc(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemKind {
    pub schema_version: u32,
    pub canonical_issue_type: String,
    pub original_issue_type: String,
    pub provider_issue_type: Option<String>,
    pub category: String,
    pub parent_required: bool,
    pub flow_bindable: bool,
    pub default_flow_binding: String,
    pub source_tiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemProviderMapping {
    pub schema_version: u32,
    pub provider: String,
    pub external_id: String,
    pub external_url: Option<String>,
    pub external_parent_id: Option<String>,
    pub provider_issue_type: Option<String>,
    pub provider_status: Option<String>,
    pub provider_priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub issue_id: TaskId,
    pub depends_on_id: TaskId,
    pub dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::TaskRecord;
    use taskflow_core::{IssueType, TaskId, TaskStatus};

    #[test]
    fn task_record_defaults_to_open() {
        let record = TaskRecord::new(TaskId::new("vida-rf1"), "program", IssueType::Epic);
        assert!(matches!(record.status, TaskStatus::Open));
    }

    #[test]
    fn task_record_constructor_preserves_public_identity_and_round_trips_json() {
        let record = TaskRecord::new(TaskId::new("vida-rf2"), "Ship contract", IssueType::Epic);
        assert_eq!(record.id.as_str(), "vida-rf2");
        assert_eq!(record.title, "Ship contract");
        assert!(matches!(record.issue_type, IssueType::Epic));

        let encoded = serde_json::to_value(&record).expect("task record serializes");
        let decoded: TaskRecord = serde_json::from_value(encoded).expect("task record decodes");
        assert_eq!(decoded.id.as_str(), "vida-rf2");
        assert_eq!(decoded.title, "Ship contract");
        assert!(matches!(decoded.status, TaskStatus::Open));
        assert!(matches!(decoded.issue_type, IssueType::Epic));
    }
}
