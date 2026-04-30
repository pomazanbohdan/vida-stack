use std::collections::{BTreeMap, BTreeSet};

use surrealdb::types::SurrealValue;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowClass {
    InformationalAnswer,
    RetrievalGroundedAnswer,
    DocumentationMutation,
    InternalStateMutation,
    DelegatedDevelopmentPacket,
    ToolAssistedRead,
    ToolAssistedWrite,
    MemoryWrite,
    IdentityOrPolicyChange,
    IncidentResponseOrRecovery,
}

impl WorkflowClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::InformationalAnswer => "informational_answer",
            Self::RetrievalGroundedAnswer => "retrieval_grounded_answer",
            Self::DocumentationMutation => "documentation_mutation",
            Self::InternalStateMutation => "internal_state_mutation",
            Self::DelegatedDevelopmentPacket => "delegated_development_packet",
            Self::ToolAssistedRead => "tool_assisted_read",
            Self::ToolAssistedWrite => "tool_assisted_write",
            Self::MemoryWrite => "memory_write",
            Self::IdentityOrPolicyChange => "identity_or_policy_change",
            Self::IncidentResponseOrRecovery => "incident_response_or_recovery",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "informational_answer" => Some(Self::InformationalAnswer),
            "retrieval_grounded_answer" => Some(Self::RetrievalGroundedAnswer),
            "documentation_mutation" => Some(Self::DocumentationMutation),
            "internal_state_mutation" => Some(Self::InternalStateMutation),
            "delegated_development_packet" => Some(Self::DelegatedDevelopmentPacket),
            "tool_assisted_read" => Some(Self::ToolAssistedRead),
            "tool_assisted_write" => Some(Self::ToolAssistedWrite),
            "memory_write" => Some(Self::MemoryWrite),
            "identity_or_policy_change" => Some(Self::IdentityOrPolicyChange),
            "incident_response_or_recovery" => Some(Self::IncidentResponseOrRecovery),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn canonical_workflow_class_str(value: &str) -> Option<&'static str> {
    WorkflowClass::from_str(value).map(WorkflowClass::as_str)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RiskTier {
    R0,
    R1,
    R2,
    R3,
    R4,
}

impl RiskTier {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::R0 => "R0",
            Self::R1 => "R1",
            Self::R2 => "R2",
            Self::R3 => "R3",
            Self::R4 => "R4",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "R0" => Some(Self::R0),
            "R1" => Some(Self::R1),
            "R2" => Some(Self::R2),
            "R3" => Some(Self::R3),
            "R4" => Some(Self::R4),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn canonical_risk_tier_str(value: &str) -> Option<&'static str> {
    RiskTier::from_str(value).map(RiskTier::as_str)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalStatus {
    ApprovalNotRequired,
    ApprovalRequired,
    WaitingForApproval,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalNotRequired => "approval_not_required",
            Self::ApprovalRequired => "approval_required",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "approval_not_required" => Some(Self::ApprovalNotRequired),
            "approval_required" => Some(Self::ApprovalRequired),
            "waiting_for_approval" => Some(Self::WaitingForApproval),
            "approved" => Some(Self::Approved),
            "denied" => Some(Self::Denied),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn canonical_approval_status_str(value: &str) -> Option<&'static str> {
    ApprovalStatus::from_str(value).map(ApprovalStatus::as_str)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GateLevel {
    Block,
    Warn,
    Observe,
}

impl GateLevel {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Warn => "warn",
            Self::Observe => "observe",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "block" => Some(Self::Block),
            "warn" => Some(Self::Warn),
            "observe" => Some(Self::Observe),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn canonical_gate_level_str(value: &str) -> Option<&'static str> {
    GateLevel::from_str(value).map(GateLevel::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneStatus {
    PacketReady,
    LaneOpen,
    LaneRunning,
    LaneBlocked,
    LaneCompleted,
    LaneSuperseded,
    LaneExceptionRecorded,
    LaneExceptionTakeover,
}

impl LaneStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PacketReady => "packet_ready",
            Self::LaneOpen => "lane_open",
            Self::LaneRunning => "lane_running",
            Self::LaneBlocked => "lane_blocked",
            Self::LaneCompleted => "lane_completed",
            Self::LaneSuperseded => "lane_superseded",
            Self::LaneExceptionRecorded => "lane_exception_recorded",
            Self::LaneExceptionTakeover => "lane_exception_takeover",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "packet_ready" => Some(Self::PacketReady),
            "lane_open" => Some(Self::LaneOpen),
            "lane_running" => Some(Self::LaneRunning),
            "lane_blocked" => Some(Self::LaneBlocked),
            "lane_completed" => Some(Self::LaneCompleted),
            "lane_superseded" => Some(Self::LaneSuperseded),
            "lane_exception_recorded" => Some(Self::LaneExceptionRecorded),
            "lane_exception_takeover" => Some(Self::LaneExceptionTakeover),
            _ => None,
        }
    }
}

pub(crate) fn canonical_lane_status_str(value: &str) -> Option<&'static str> {
    LaneStatus::from_str(value).map(LaneStatus::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatibilityClass {
    BackwardCompatible,
    ReaderUpgradeRequired,
    MigrationRequired,
}

impl CompatibilityClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::BackwardCompatible => "backward_compatible",
            Self::ReaderUpgradeRequired => "reader_upgrade_required",
            Self::MigrationRequired => "migration_required",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "backward_compatible" | "compatible" => Some(Self::BackwardCompatible),
            "reader_upgrade_required" | "incompatible" | "degraded" | "blocking" => {
                Some(Self::ReaderUpgradeRequired)
            }
            "migration_required" => Some(Self::MigrationRequired),
            _ => None,
        }
    }
}

pub(crate) fn canonical_compatibility_class_str(value: &str) -> Option<&'static str> {
    CompatibilityClass::from_str(value).map(CompatibilityClass::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatibilityBoundary {
    Compatible,
    BlockingSupported,
    Unsupported,
}

pub(crate) fn classify_compatibility_boundary(value: &str) -> CompatibilityBoundary {
    match canonical_compatibility_class_str(value) {
        Some("backward_compatible") => CompatibilityBoundary::Compatible,
        Some(_) => CompatibilityBoundary::BlockingSupported,
        None => CompatibilityBoundary::Unsupported,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1ContractType {
    OperatorContracts,
    SharedFields,
}

impl Release1ContractType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::OperatorContracts => "release-1-operator-contracts",
            Self::SharedFields => "release-1-shared-fields",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "release-1-operator-contracts" => Some(Self::OperatorContracts),
            "release-1-shared-fields" => Some(Self::SharedFields),
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_contract_type_str(value: &str) -> Option<&'static str> {
    Release1ContractType::from_str(value).map(Release1ContractType::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1SchemaVersion {
    V1,
}

impl Release1SchemaVersion {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "release-1-v1",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "release-1-v1" => Some(Self::V1),
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_schema_version_str(value: &str) -> Option<&'static str> {
    Release1SchemaVersion::from_str(value).map(Release1SchemaVersion::as_str)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonicalArtifactType {
    TraceEvent,
    PolicyDecision,
    ApprovalRecord,
    ToolContract,
    LaneExecutionReceipt,
    EvaluationRun,
    FeedbackEvent,
    IncidentEvidenceBundle,
    MemoryRecord,
    ClosureAdmissionRecord,
}

impl CanonicalArtifactType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::TraceEvent => "trace_event",
            Self::PolicyDecision => "policy_decision",
            Self::ApprovalRecord => "approval_record",
            Self::ToolContract => "tool_contract",
            Self::LaneExecutionReceipt => "lane_execution_receipt",
            Self::EvaluationRun => "evaluation_run",
            Self::FeedbackEvent => "feedback_event",
            Self::IncidentEvidenceBundle => "incident_evidence_bundle",
            Self::MemoryRecord => "memory_record",
            Self::ClosureAdmissionRecord => "closure_admission_record",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "trace_event" => Some(Self::TraceEvent),
            "policy_decision" => Some(Self::PolicyDecision),
            "approval_record" => Some(Self::ApprovalRecord),
            "tool_contract" => Some(Self::ToolContract),
            "lane_execution_receipt" => Some(Self::LaneExecutionReceipt),
            "evaluation_run" => Some(Self::EvaluationRun),
            "feedback_event" => Some(Self::FeedbackEvent),
            "incident_evidence_bundle" => Some(Self::IncidentEvidenceBundle),
            "memory_record" => Some(Self::MemoryRecord),
            "closure_admission_record" => Some(Self::ClosureAdmissionRecord),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn canonical_artifact_type_str(value: &str) -> Option<&'static str> {
    CanonicalArtifactType::from_str(value).map(CanonicalArtifactType::as_str)
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalArtifactHeader {
    pub artifact_id: String,
    pub artifact_type: String,
    pub schema_version: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub owner_surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_class: Option<String>,
}

#[allow(dead_code)]
impl CanonicalArtifactHeader {
    pub(crate) fn new(
        artifact_id: impl Into<String>,
        artifact_type: CanonicalArtifactType,
        created_at: impl Into<String>,
        updated_at: impl Into<String>,
        status: impl Into<String>,
        owner_surface: impl Into<String>,
        trace_id: Option<String>,
        workflow_class: Option<String>,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            artifact_type: artifact_type.as_str().to_string(),
            schema_version: Release1SchemaVersion::V1.as_str().to_string(),
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            status: status.into(),
            owner_surface: owner_surface.into(),
            trace_id,
            workflow_class,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalTraceEvent {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub span_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub workflow_run_id: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub event_type: String,
    pub started_at: String,
    pub ended_at: String,
    pub outcome: String,
    pub side_effect_class: String,
    #[serde(default)]
    pub related_artifact_ids: Vec<String>,
    #[serde(default)]
    pub policy_decision_ids: Vec<String>,
    #[serde(default)]
    pub approval_record_ids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalPolicyDecision {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub policy_id: String,
    pub policy_version: String,
    pub actor_id: String,
    pub subject_id: String,
    pub decision: String,
    #[serde(default)]
    pub reason_codes: Vec<String>,
    #[serde(default)]
    pub constraints_applied: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalApprovalRecord {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub approval_id: String,
    pub approval_scope: String,
    pub requested_by: String,
    pub approved_by: String,
    pub decision: String,
    pub decision_at: String,
    pub decision_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub related_policy_decision_ids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalToolContract {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub tool_id: String,
    pub tool_version: String,
    pub tool_name: String,
    pub operation_class: String,
    pub side_effect_class: String,
    pub auth_mode: String,
    pub approval_required: bool,
    pub idempotency_class: String,
    pub retry_posture: String,
    pub rollback_posture: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    #[serde(default)]
    pub policy_hook_ids: Vec<String>,
    #[serde(default)]
    pub observability_requirements: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalLaneExecutionReceipt {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub run_id: String,
    pub packet_id: String,
    pub lane_id: String,
    pub lane_role: String,
    pub carrier_id: String,
    pub lane_status: String,
    pub evidence_status: String,
    pub started_at: String,
    pub finished_at: String,
    #[serde(default)]
    pub result_artifact_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception_path_receipt_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalEvaluationRun {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub evaluation_id: String,
    pub evaluation_profile: String,
    pub target_surface: String,
    pub dataset_or_sample_window: String,
    #[serde(default)]
    pub metric_results: BTreeMap<String, f64>,
    pub regression_summary: String,
    pub decision: String,
    pub decision_reason: String,
    pub run_at: String,
    #[serde(default)]
    pub trace_sample_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalFeedbackEvent {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub feedback_id: String,
    pub source_kind: String,
    pub severity: String,
    pub feedback_type: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_defect_or_remediation_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalIncidentEvidenceBundle {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub incident_id: String,
    #[serde(default)]
    pub trace_ids: Vec<String>,
    pub trigger_reason: String,
    pub impact_summary: String,
    pub side_effect_summary: String,
    #[serde(default)]
    pub rollback_or_restore_actions: Vec<String>,
    pub recovery_outcome: String,
    pub root_cause_status: String,
    pub opened_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalMemoryRecord {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub memory_id: String,
    pub memory_class: String,
    pub subject_scope: String,
    pub origin_trace_id: String,
    pub origin_workflow_class: String,
    pub sensitivity_level: String,
    pub consent_basis: String,
    pub ttl_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_or_correction_ref: Option<String>,
    #[serde(default)]
    pub approval_record_ids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalClosureAdmissionRecord {
    #[serde(flatten)]
    pub header: CanonicalArtifactHeader,
    pub release_scope: String,
    #[serde(default)]
    pub supported_workflow_classes: Vec<String>,
    pub closure_decision: String,
    pub decision_at: String,
    pub decision_owner: String,
    #[serde(default)]
    pub evidence_bundle_refs: Vec<String>,
    #[serde(default)]
    pub open_risk_acceptance_ids: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
}

// Keep the older artifact-oriented names as explicit wrappers while the rest of
// the runtime migrates to the schema's canonical event/record/run/bundle nouns.
// `flatten` preserves the existing wire shape so operator and runtime surfaces
// can reuse either contract layer without carrier-specific branching.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalTraceArtifact {
    #[serde(flatten)]
    pub trace_event: CanonicalTraceEvent,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalPolicyDecisionArtifact {
    #[serde(flatten)]
    pub policy_decision: CanonicalPolicyDecision,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalApprovalArtifact {
    #[serde(flatten)]
    pub approval_record: CanonicalApprovalRecord,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalToolContractArtifact {
    #[serde(flatten)]
    pub tool_contract: CanonicalToolContract,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalLaneExecutionReceiptArtifact {
    #[serde(flatten)]
    pub lane_execution_receipt: CanonicalLaneExecutionReceipt,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalEvaluationArtifact {
    #[serde(flatten)]
    pub evaluation_run: CanonicalEvaluationRun,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalFeedbackArtifact {
    #[serde(flatten)]
    pub feedback_event: CanonicalFeedbackEvent,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalIncidentEvidenceArtifact {
    #[serde(flatten)]
    pub incident_evidence_bundle: CanonicalIncidentEvidenceBundle,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalMemoryArtifact {
    #[serde(flatten)]
    pub memory_record: CanonicalMemoryRecord,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub(crate) struct CanonicalClosureAdmissionArtifact {
    #[serde(flatten)]
    pub closure_admission_record: CanonicalClosureAdmissionRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1ContractStatus {
    Pass,
    Blocked,
}

impl Release1ContractStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Blocked => "blocked",
        }
    }

    pub(crate) const fn from_bool(ok: bool) -> Self {
        if ok {
            Self::Pass
        } else {
            Self::Blocked
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            value if value.eq_ignore_ascii_case("pass") || value.eq_ignore_ascii_case("ok") => {
                Some(Self::Pass)
            }
            value
                if value.eq_ignore_ascii_case("blocked") || value.eq_ignore_ascii_case("block") =>
            {
                Some(Self::Blocked)
            }
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_contract_status_str(value: &str) -> Option<&'static str> {
    Release1ContractStatus::from_str(value).map(Release1ContractStatus::as_str)
}

pub(crate) fn release1_contract_status_str(ok: bool) -> &'static str {
    Release1ContractStatus::from_bool(ok).as_str()
}

pub(crate) fn has_evidence_id(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExceptionTakeoverState {
    NotRecorded,
    ReceiptRecorded,
    ActiveTakeover,
}

impl ExceptionTakeoverState {
    pub(crate) const fn is_active(self) -> bool {
        matches!(self, Self::ActiveTakeover)
    }
}

pub(crate) fn exception_takeover_state(
    exception_path_receipt_id: Option<&str>,
    supersedes_receipt_id: Option<&str>,
    local_exception_takeover_gate: Option<&str>,
) -> ExceptionTakeoverState {
    if !has_evidence_id(exception_path_receipt_id) {
        return ExceptionTakeoverState::NotRecorded;
    }
    if has_evidence_id(supersedes_receipt_id) {
        return ExceptionTakeoverState::ActiveTakeover;
    }
    if local_exception_takeover_gate
        .is_some_and(|gate| gate.trim() != "blocked_open_delegated_cycle")
    {
        return ExceptionTakeoverState::ActiveTakeover;
    }
    ExceptionTakeoverState::ReceiptRecorded
}

pub(crate) fn lane_status_has_required_evidence(
    lane_status: LaneStatus,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> bool {
    match lane_status {
        LaneStatus::LaneSuperseded => has_evidence_id(supersedes_receipt_id),
        LaneStatus::LaneExceptionRecorded | LaneStatus::LaneExceptionTakeover => {
            has_evidence_id(exception_path_receipt_id)
        }
        _ => false,
    }
}

pub(crate) fn derive_lane_status(
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> LaneStatus {
    if has_evidence_id(exception_path_receipt_id) {
        return LaneStatus::LaneExceptionRecorded;
    }
    if has_evidence_id(supersedes_receipt_id) {
        return LaneStatus::LaneSuperseded;
    }
    match dispatch_status {
        "packet_ready" => LaneStatus::PacketReady,
        "routed" => LaneStatus::LaneOpen,
        "executing" => LaneStatus::LaneRunning,
        "executed" => LaneStatus::LaneRunning,
        "blocked" => LaneStatus::LaneBlocked,
        _ => LaneStatus::LaneOpen,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockerCode {
    MissingPacket,
    MissingLaneReceipt,
    OpenDelegatedCycle,
    ExceptionPathMissing,
    ClosureEvidenceIncomplete,
    OwnerSurfaceContradiction,
    PolicyDenied,
    PolicyContextMissing,
    ApprovalRequired,
    ApprovalDenied,
    ApprovalExpired,
    DelegationChainBroken,
    ToolContractMissing,
    ToolContractIncomplete,
    ExternalCliNetworkAccessUnavailableUnderSandbox,
    InteractiveAuthRequired,
    ProviderAuthFailed,
    ModelNotPinned,
    ToolExecutionFailed,
    ToolResultUnusable,
    CitationMissing,
    SourceUnregistered,
    FreshnessPolicyMissing,
    FreshnessViolation,
    AclContextMissing,
    TraceIncomplete,
    TraceMissing,
    IncidentEvidenceMissing,
    RollbackUnavailable,
    ProofVerdictMissing,
    MetricGateFailed,
    SchemaContractMissing,
    TimeoutWithoutTakeoverAuthority,
    SupersessionWithoutReceipt,
    LocalTakeoverForbidden,
    RecoveryNotTrustworthy,
    InvalidControlCoreKeys,
    MissingRootArtifactId,
    MissingMandatoryChainOrder,
    MissingEffectiveBundleArtifacts,
    MissingBundleId,
    MissingBundleSchemaVersion,
    MissingAuthoritativeProtocolBindingImportEvidence,
    MixedRuntimeRoot,
    MixedConfigPath,
    MissingCacheKeyInputs,
    MissingInvalidationTuple,
    BootCompatibilityNotCompatible,
    MigrationPreflightNotReady,
    MissingProtocolBindingRows,
    ProtocolBindingRowsNotRuntimeTrusted,
    ActivationPending,
    TaskflowBlockedDuringPendingActivation,
    MissingProtocolBindingReceipt,
    ProtocolBindingNotRuntimeReady,
    MissingRootSessionWriteGuard,
    MigrationRequired,
    ProtocolBindingBlockingIssues,
    ContinuationBindingAmbiguous,
    MissingRunGraphDispatchReceiptOperatorEvidence,
    RunGraphLatestSnapshotInconsistent,
    LatestRunGraphStatusBlocked,
    RunGraphLatestDispatchReceiptSignalAmbiguous,
    RunGraphLatestDispatchReceiptSummaryInconsistent,
    RunGraphLatestDispatchReceiptCheckpointLeakage,
    TerminalContinueSnapshotWithoutNextBoundedUnit,
    ProjectActivationUnknown,
    DependencyGraphIssues,
    DispatchPacketContractInvalid,
    ExecutionModeNotParallelSafe,
    CurrentExecutionModeNotParallelSafe,
    OrderBucketMismatchOrMissing,
    ConflictDomainCollision,
    MissingConflictDomain,
    BootCompatibilityUnsupportedBoundary,
    MigrationPreflightUnsupportedBoundary,
    MissingRetrievalTrustSourceOperatorEvidence,
    MissingRetrievalTrustSignalOperatorEvidence,
    MissingRetrievalTrustOperatorEvidence,
    IncompleteReleaseAdmissionOperatorEvidence,
    RecoveryReadinessBlocked,
    UnsupportedArchitectureReservedWorkflowBoundary,
    InvalidProtocolBindingRegistryKeys,
    InvalidCacheKeyInputsKeys,
    InvalidInvalidationTupleKeys,
    InvalidMetadataTupleKeys,
    CacheTupleProtocolBindingEvidenceUntrusted,
    CacheTupleProtocolBindingTokenMismatch,
    MissingLauncherActivationSnapshot,
    InvalidCompiledBundleRoleSelectionMode,
    InvalidCompiledBundleAgentSystemMode,
    InvalidCompiledBundleAgentSystemStateOwner,
    MissingEffectiveBundleReceiptId,
    MissingEffectiveBundleRootArtifactId,
    EmptyEffectiveBundleArtifactCount,
    MissingEffectiveBundleReceipt,
    NoReadyTasks,
    ExecutionPreparationGateBlocked,
    TaskGraphEmpty,
    MissingDocflowActivation,
    DocflowCheckBlocking,
    MissingReadinessVerdict,
    MissingInventoryOrProjectionEvidence,
    MissingProofVerdict,
    MissingClosureProof,
    RestoreReconcileNotGreen,
    PendingSpecificationEvidence,
    PendingExecutionPreparationEvidence,
    PendingApprovalDelegationEvidence,
    PendingImplementationEvidence,
    PendingReviewCleanEvidence,
    PendingVerificationEvidence,
    PendingLaneEvidence,
    PendingReviewFindings,
    PendingDesignFinalize,
    PendingSpecTaskClose,
    PendingDesignPacket,
    PendingDeveloperHandoffPacket,
    MissingExecutionPreparationContract,
    ExecutionPreparationArtifactsUnavailable,
    MissingExecutionPreparationArtifactQueryTarget,
    ImplementationReviewDenied,
    ImplementationReviewExpired,
    ImplementationReviewFindings,
    ImplementationReviewChangedScope,
    BundleActivationNotReady,
    DocflowVerdictBlock,
    ClosureAdmissionBlock,
    Unsupported,
}

impl BlockerCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::MissingPacket => "missing_packet",
            Self::MissingLaneReceipt => "missing_lane_receipt",
            Self::OpenDelegatedCycle => "open_delegated_cycle",
            Self::ExceptionPathMissing => "exception_path_missing",
            Self::ClosureEvidenceIncomplete => "closure_evidence_incomplete",
            Self::OwnerSurfaceContradiction => "owner_surface_contradiction",
            Self::PolicyDenied => "policy_denied",
            Self::PolicyContextMissing => "policy_context_missing",
            Self::ApprovalRequired => "approval_required",
            Self::ApprovalDenied => "approval_denied",
            Self::ApprovalExpired => "approval_expired",
            Self::DelegationChainBroken => "delegation_chain_broken",
            Self::ToolContractMissing => "tool_contract_missing",
            Self::ToolContractIncomplete => "tool_contract_incomplete",
            Self::ExternalCliNetworkAccessUnavailableUnderSandbox => {
                "external_cli_network_access_unavailable_under_sandbox"
            }
            Self::InteractiveAuthRequired => "interactive_auth_required",
            Self::ProviderAuthFailed => "provider_auth_failed",
            Self::ModelNotPinned => "model_not_pinned",
            Self::ToolExecutionFailed => "tool_execution_failed",
            Self::ToolResultUnusable => "tool_result_unusable",
            Self::CitationMissing => "citation_missing",
            Self::SourceUnregistered => "source_unregistered",
            Self::FreshnessPolicyMissing => "freshness_policy_missing",
            Self::FreshnessViolation => "freshness_violation",
            Self::AclContextMissing => "acl_context_missing",
            Self::TraceIncomplete => "trace_incomplete",
            Self::TraceMissing => "trace_missing",
            Self::IncidentEvidenceMissing => "incident_evidence_missing",
            Self::RollbackUnavailable => "rollback_unavailable",
            Self::ProofVerdictMissing => "proof_verdict_missing",
            Self::MetricGateFailed => "metric_gate_failed",
            Self::SchemaContractMissing => "schema_contract_missing",
            Self::TimeoutWithoutTakeoverAuthority => "timeout_without_takeover_authority",
            Self::SupersessionWithoutReceipt => "supersession_without_receipt",
            Self::LocalTakeoverForbidden => "local_takeover_forbidden",
            Self::RecoveryNotTrustworthy => "recovery_not_trustworthy",
            Self::InvalidControlCoreKeys => "invalid_control_core_keys",
            Self::MissingRootArtifactId => "missing_root_artifact_id",
            Self::MissingMandatoryChainOrder => "missing_mandatory_chain_order",
            Self::MissingEffectiveBundleArtifacts => "missing_effective_bundle_artifacts",
            Self::MissingBundleId => "missing_bundle_id",
            Self::MissingBundleSchemaVersion => "missing_bundle_schema_version",
            Self::MissingAuthoritativeProtocolBindingImportEvidence => {
                "missing_authoritative_protocol_binding_import_evidence"
            }
            Self::MixedRuntimeRoot => "mixed_runtime_root",
            Self::MixedConfigPath => "mixed_config_path",
            Self::MissingCacheKeyInputs => "missing_cache_key_inputs",
            Self::MissingInvalidationTuple => "missing_invalidation_tuple",
            Self::BootCompatibilityNotCompatible => "boot_incompatible",
            Self::MigrationPreflightNotReady => "migration_not_ready",
            Self::MissingProtocolBindingRows => "missing_protocol_binding_rows",
            Self::ProtocolBindingRowsNotRuntimeTrusted => {
                "protocol_binding_rows_not_runtime_trusted"
            }
            Self::ActivationPending => "activation_pending",
            Self::TaskflowBlockedDuringPendingActivation => {
                "taskflow_blocked_during_pending_activation"
            }
            Self::MissingProtocolBindingReceipt => "missing_protocol_binding_receipt",
            Self::ProtocolBindingNotRuntimeReady => "protocol_binding_not_runtime_ready",
            Self::MissingRootSessionWriteGuard => "missing_root_session_write_guard",
            Self::MigrationRequired => "migration_required",
            Self::ProtocolBindingBlockingIssues => "protocol_binding_blocking_issues",
            Self::ContinuationBindingAmbiguous => "continuation_binding_ambiguous",
            Self::MissingRunGraphDispatchReceiptOperatorEvidence => {
                "missing_run_graph_dispatch_receipt_operator_evidence"
            }
            Self::RunGraphLatestSnapshotInconsistent => "run_graph_latest_snapshot_inconsistent",
            Self::LatestRunGraphStatusBlocked => "latest_run_graph_status_blocked",
            Self::RunGraphLatestDispatchReceiptSignalAmbiguous => {
                "run_graph_latest_dispatch_receipt_signal_ambiguous"
            }
            Self::RunGraphLatestDispatchReceiptSummaryInconsistent => {
                "run_graph_latest_dispatch_receipt_summary_inconsistent"
            }
            Self::RunGraphLatestDispatchReceiptCheckpointLeakage => {
                "run_graph_latest_dispatch_receipt_checkpoint_leakage"
            }
            Self::TerminalContinueSnapshotWithoutNextBoundedUnit => {
                "terminal_continue_snapshot_without_next_bounded_unit"
            }
            Self::ProjectActivationUnknown => "project_activation_unknown",
            Self::DependencyGraphIssues => "dependency_graph_issues",
            Self::DispatchPacketContractInvalid => "dispatch_packet_contract_invalid",
            Self::ExecutionModeNotParallelSafe => "execution_mode_not_parallel_safe",
            Self::CurrentExecutionModeNotParallelSafe => "current_execution_mode_not_parallel_safe",
            Self::OrderBucketMismatchOrMissing => "order_bucket_mismatch_or_missing",
            Self::ConflictDomainCollision => "conflict_domain_collision",
            Self::MissingConflictDomain => "missing_conflict_domain",
            Self::BootCompatibilityUnsupportedBoundary => "boot_compatibility_unsupported_boundary",
            Self::MigrationPreflightUnsupportedBoundary => {
                "migration_preflight_unsupported_boundary"
            }
            Self::MissingRetrievalTrustSourceOperatorEvidence => {
                "missing_retrieval_trust_source_operator_evidence"
            }
            Self::MissingRetrievalTrustSignalOperatorEvidence => {
                "missing_retrieval_trust_signal_operator_evidence"
            }
            Self::MissingRetrievalTrustOperatorEvidence => {
                "missing_retrieval_trust_operator_evidence"
            }
            Self::IncompleteReleaseAdmissionOperatorEvidence => {
                "incomplete_release_admission_operator_evidence"
            }
            Self::RecoveryReadinessBlocked => "recovery_readiness_blocked",
            Self::UnsupportedArchitectureReservedWorkflowBoundary => {
                "unsupported_architecture_reserved_workflow_boundary"
            }
            Self::InvalidProtocolBindingRegistryKeys => "invalid_protocol_binding_registry_keys",
            Self::InvalidCacheKeyInputsKeys => "invalid_cache_key_inputs_keys",
            Self::InvalidInvalidationTupleKeys => "invalid_invalidation_tuple_keys",
            Self::InvalidMetadataTupleKeys => "invalid_metadata_tuple_keys",
            Self::CacheTupleProtocolBindingEvidenceUntrusted => {
                "cache_tuple_protocol_binding_evidence_untrusted"
            }
            Self::CacheTupleProtocolBindingTokenMismatch => {
                "cache_tuple_protocol_binding_token_mismatch"
            }
            Self::MissingLauncherActivationSnapshot => "missing_launcher_activation_snapshot",
            Self::InvalidCompiledBundleRoleSelectionMode => {
                "invalid_compiled_bundle_role_selection_mode"
            }
            Self::InvalidCompiledBundleAgentSystemMode => {
                "invalid_compiled_bundle_agent_system_mode"
            }
            Self::InvalidCompiledBundleAgentSystemStateOwner => {
                "invalid_compiled_bundle_agent_system_state_owner"
            }
            Self::MissingEffectiveBundleReceiptId => "missing_effective_bundle_receipt_id",
            Self::MissingEffectiveBundleRootArtifactId => {
                "missing_effective_bundle_root_artifact_id"
            }
            Self::EmptyEffectiveBundleArtifactCount => "empty_effective_bundle_artifact_count",
            Self::MissingEffectiveBundleReceipt => "missing_effective_bundle_receipt",
            Self::NoReadyTasks => "no_ready_tasks",
            Self::ExecutionPreparationGateBlocked => "execution_preparation_gate_blocked",
            Self::TaskGraphEmpty => "task_graph_empty",
            Self::MissingDocflowActivation => "missing_docflow_activation",
            Self::DocflowCheckBlocking => "docflow_check_blocking",
            Self::MissingReadinessVerdict => "missing_readiness_verdict",
            Self::MissingInventoryOrProjectionEvidence => {
                "missing_inventory_or_projection_evidence"
            }
            Self::MissingProofVerdict => "missing_proof_verdict",
            Self::MissingClosureProof => "missing_closure_proof",
            Self::RestoreReconcileNotGreen => "restore_reconcile_not_green",
            Self::PendingSpecificationEvidence => "pending_specification_evidence",
            Self::PendingExecutionPreparationEvidence => "pending_execution_preparation_evidence",
            Self::PendingApprovalDelegationEvidence => "pending_approval_delegation_evidence",
            Self::PendingImplementationEvidence => "pending_implementation_evidence",
            Self::PendingReviewCleanEvidence => "pending_review_clean_evidence",
            Self::PendingVerificationEvidence => "pending_verification_evidence",
            Self::PendingLaneEvidence => "pending_lane_evidence",
            Self::PendingReviewFindings => "pending_review_findings",
            Self::PendingDesignFinalize => "pending_design_finalize",
            Self::PendingSpecTaskClose => "pending_spec_task_close",
            Self::PendingDesignPacket => "pending_design_packet",
            Self::PendingDeveloperHandoffPacket => "pending_developer_handoff_packet",
            Self::MissingExecutionPreparationContract => "missing_execution_preparation_contract",
            Self::ExecutionPreparationArtifactsUnavailable => {
                "execution_preparation_artifacts_unavailable"
            }
            Self::MissingExecutionPreparationArtifactQueryTarget => {
                "missing_execution_preparation_artifact_query_target"
            }
            Self::ImplementationReviewDenied => "implementation_review_denied",
            Self::ImplementationReviewExpired => "implementation_review_expired",
            Self::ImplementationReviewFindings => "implementation_review_findings",
            Self::ImplementationReviewChangedScope => "implementation_review_changed_scope",
            Self::BundleActivationNotReady => "bundle_activation_not_ready",
            Self::DocflowVerdictBlock => "docflow_verdict_block",
            Self::ClosureAdmissionBlock => "closure_admission_block",
            Self::Unsupported => "unsupported_blocker_code",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "missing_packet" => Some(Self::MissingPacket),
            "missing_lane_receipt" => Some(Self::MissingLaneReceipt),
            "open_delegated_cycle" => Some(Self::OpenDelegatedCycle),
            "exception_path_missing" => Some(Self::ExceptionPathMissing),
            "closure_evidence_incomplete" => Some(Self::ClosureEvidenceIncomplete),
            "owner_surface_contradiction" => Some(Self::OwnerSurfaceContradiction),
            "policy_denied" => Some(Self::PolicyDenied),
            "policy_context_missing" => Some(Self::PolicyContextMissing),
            "approval_required" => Some(Self::ApprovalRequired),
            "approval_denied" => Some(Self::ApprovalDenied),
            "approval_expired" => Some(Self::ApprovalExpired),
            "delegation_chain_broken" => Some(Self::DelegationChainBroken),
            "tool_contract_missing" => Some(Self::ToolContractMissing),
            "tool_contract_incomplete" => Some(Self::ToolContractIncomplete),
            "external_cli_network_access_unavailable_under_sandbox" => {
                Some(Self::ExternalCliNetworkAccessUnavailableUnderSandbox)
            }
            "interactive_auth_required" => Some(Self::InteractiveAuthRequired),
            "provider_auth_failed" => Some(Self::ProviderAuthFailed),
            "model_not_pinned" => Some(Self::ModelNotPinned),
            "tool_execution_failed" => Some(Self::ToolExecutionFailed),
            "tool_result_unusable" => Some(Self::ToolResultUnusable),
            "citation_missing" => Some(Self::CitationMissing),
            "source_unregistered" => Some(Self::SourceUnregistered),
            "freshness_policy_missing" => Some(Self::FreshnessPolicyMissing),
            "freshness_violation" => Some(Self::FreshnessViolation),
            "acl_context_missing" => Some(Self::AclContextMissing),
            "trace_incomplete" => Some(Self::TraceIncomplete),
            "trace_missing" => Some(Self::TraceMissing),
            "incident_evidence_missing" => Some(Self::IncidentEvidenceMissing),
            "rollback_unavailable" => Some(Self::RollbackUnavailable),
            "proof_verdict_missing" => Some(Self::ProofVerdictMissing),
            "metric_gate_failed" => Some(Self::MetricGateFailed),
            "schema_contract_missing" => Some(Self::SchemaContractMissing),
            "timeout_without_takeover_authority" => Some(Self::TimeoutWithoutTakeoverAuthority),
            "supersession_without_receipt" => Some(Self::SupersessionWithoutReceipt),
            "local_takeover_forbidden" => Some(Self::LocalTakeoverForbidden),
            "recovery_not_trustworthy" => Some(Self::RecoveryNotTrustworthy),
            "invalid_control_core_keys" => Some(Self::InvalidControlCoreKeys),
            "missing_root_artifact_id" => Some(Self::MissingRootArtifactId),
            "missing_mandatory_chain_order" => Some(Self::MissingMandatoryChainOrder),
            "missing_effective_bundle_artifacts" => Some(Self::MissingEffectiveBundleArtifacts),
            "missing_bundle_id" => Some(Self::MissingBundleId),
            "missing_bundle_schema_version" => Some(Self::MissingBundleSchemaVersion),
            "missing_authoritative_protocol_binding_import_evidence" => {
                Some(Self::MissingAuthoritativeProtocolBindingImportEvidence)
            }
            "mixed_runtime_root" => Some(Self::MixedRuntimeRoot),
            "mixed_config_path" => Some(Self::MixedConfigPath),
            "missing_cache_key_inputs" => Some(Self::MissingCacheKeyInputs),
            "missing_invalidation_tuple" => Some(Self::MissingInvalidationTuple),
            "boot_incompatible" => Some(Self::BootCompatibilityNotCompatible),
            "migration_not_ready" => Some(Self::MigrationPreflightNotReady),
            "missing_protocol_binding_rows" => Some(Self::MissingProtocolBindingRows),
            "protocol_binding_rows_not_runtime_trusted" => {
                Some(Self::ProtocolBindingRowsNotRuntimeTrusted)
            }
            "activation_pending" => Some(Self::ActivationPending),
            "taskflow_blocked_during_pending_activation" => {
                Some(Self::TaskflowBlockedDuringPendingActivation)
            }
            "missing_protocol_binding_receipt" => Some(Self::MissingProtocolBindingReceipt),
            "protocol_binding_not_runtime_ready" => Some(Self::ProtocolBindingNotRuntimeReady),
            "missing_root_session_write_guard" => Some(Self::MissingRootSessionWriteGuard),
            "migration_required" => Some(Self::MigrationRequired),
            "protocol_binding_blocking_issues" => Some(Self::ProtocolBindingBlockingIssues),
            "continuation_binding_ambiguous" => Some(Self::ContinuationBindingAmbiguous),
            "missing_run_graph_dispatch_receipt_operator_evidence" => {
                Some(Self::MissingRunGraphDispatchReceiptOperatorEvidence)
            }
            "run_graph_latest_snapshot_inconsistent" => {
                Some(Self::RunGraphLatestSnapshotInconsistent)
            }
            "latest_run_graph_status_blocked" => Some(Self::LatestRunGraphStatusBlocked),
            "run_graph_latest_dispatch_receipt_signal_ambiguous" => {
                Some(Self::RunGraphLatestDispatchReceiptSignalAmbiguous)
            }
            "run_graph_latest_dispatch_receipt_summary_inconsistent" => {
                Some(Self::RunGraphLatestDispatchReceiptSummaryInconsistent)
            }
            "run_graph_latest_dispatch_receipt_checkpoint_leakage" => {
                Some(Self::RunGraphLatestDispatchReceiptCheckpointLeakage)
            }
            "terminal_continue_snapshot_without_next_bounded_unit" => {
                Some(Self::TerminalContinueSnapshotWithoutNextBoundedUnit)
            }
            "project_activation_unknown" => Some(Self::ProjectActivationUnknown),
            "dependency_graph_issues" => Some(Self::DependencyGraphIssues),
            "dispatch_packet_contract_invalid" => Some(Self::DispatchPacketContractInvalid),
            "execution_mode_not_parallel_safe" => Some(Self::ExecutionModeNotParallelSafe),
            "current_execution_mode_not_parallel_safe" => {
                Some(Self::CurrentExecutionModeNotParallelSafe)
            }
            "order_bucket_mismatch_or_missing" => Some(Self::OrderBucketMismatchOrMissing),
            "conflict_domain_collision" => Some(Self::ConflictDomainCollision),
            "missing_conflict_domain" => Some(Self::MissingConflictDomain),
            "boot_compatibility_unsupported_boundary" => {
                Some(Self::BootCompatibilityUnsupportedBoundary)
            }
            "migration_preflight_unsupported_boundary" => {
                Some(Self::MigrationPreflightUnsupportedBoundary)
            }
            "missing_retrieval_trust_source_operator_evidence" => {
                Some(Self::MissingRetrievalTrustSourceOperatorEvidence)
            }
            "missing_retrieval_trust_signal_operator_evidence" => {
                Some(Self::MissingRetrievalTrustSignalOperatorEvidence)
            }
            "missing_retrieval_trust_operator_evidence" => {
                Some(Self::MissingRetrievalTrustOperatorEvidence)
            }
            "incomplete_release_admission_operator_evidence" => {
                Some(Self::IncompleteReleaseAdmissionOperatorEvidence)
            }
            "recovery_readiness_blocked" => Some(Self::RecoveryReadinessBlocked),
            "unsupported_architecture_reserved_workflow_boundary" => {
                Some(Self::UnsupportedArchitectureReservedWorkflowBoundary)
            }
            "invalid_protocol_binding_registry_keys" => {
                Some(Self::InvalidProtocolBindingRegistryKeys)
            }
            "invalid_cache_key_inputs_keys" => Some(Self::InvalidCacheKeyInputsKeys),
            "invalid_invalidation_tuple_keys" => Some(Self::InvalidInvalidationTupleKeys),
            "invalid_metadata_tuple_keys" => Some(Self::InvalidMetadataTupleKeys),
            "cache_tuple_protocol_binding_evidence_untrusted" => {
                Some(Self::CacheTupleProtocolBindingEvidenceUntrusted)
            }
            "cache_tuple_protocol_binding_token_mismatch" => {
                Some(Self::CacheTupleProtocolBindingTokenMismatch)
            }
            "missing_launcher_activation_snapshot" => Some(Self::MissingLauncherActivationSnapshot),
            "invalid_compiled_bundle_role_selection_mode" => {
                Some(Self::InvalidCompiledBundleRoleSelectionMode)
            }
            "invalid_compiled_bundle_agent_system_mode" => {
                Some(Self::InvalidCompiledBundleAgentSystemMode)
            }
            "invalid_compiled_bundle_agent_system_state_owner" => {
                Some(Self::InvalidCompiledBundleAgentSystemStateOwner)
            }
            "missing_effective_bundle_receipt_id" => Some(Self::MissingEffectiveBundleReceiptId),
            "missing_effective_bundle_root_artifact_id" => {
                Some(Self::MissingEffectiveBundleRootArtifactId)
            }
            "empty_effective_bundle_artifact_count" => {
                Some(Self::EmptyEffectiveBundleArtifactCount)
            }
            "missing_effective_bundle_receipt" => Some(Self::MissingEffectiveBundleReceipt),
            "no_ready_tasks" => Some(Self::NoReadyTasks),
            "execution_preparation_gate_blocked" => Some(Self::ExecutionPreparationGateBlocked),
            "task_graph_empty" => Some(Self::TaskGraphEmpty),
            "missing_docflow_activation" => Some(Self::MissingDocflowActivation),
            "docflow_check_blocking" => Some(Self::DocflowCheckBlocking),
            "missing_readiness_verdict" => Some(Self::MissingReadinessVerdict),
            "missing_inventory_or_projection_evidence" => {
                Some(Self::MissingInventoryOrProjectionEvidence)
            }
            "missing_proof_verdict" => Some(Self::MissingProofVerdict),
            "missing_closure_proof" => Some(Self::MissingClosureProof),
            "restore_reconcile_not_green" => Some(Self::RestoreReconcileNotGreen),
            "pending_specification_evidence" => Some(Self::PendingSpecificationEvidence),
            "pending_execution_preparation_evidence" => {
                Some(Self::PendingExecutionPreparationEvidence)
            }
            "pending_approval_delegation_evidence" => Some(Self::PendingApprovalDelegationEvidence),
            "pending_implementation_evidence" => Some(Self::PendingImplementationEvidence),
            "pending_review_clean_evidence" => Some(Self::PendingReviewCleanEvidence),
            "pending_verification_evidence" => Some(Self::PendingVerificationEvidence),
            "pending_lane_evidence" => Some(Self::PendingLaneEvidence),
            "pending_review_findings" => Some(Self::PendingReviewFindings),
            "pending_design_finalize" => Some(Self::PendingDesignFinalize),
            "pending_spec_task_close" => Some(Self::PendingSpecTaskClose),
            "pending_design_packet" => Some(Self::PendingDesignPacket),
            "pending_developer_handoff_packet" => Some(Self::PendingDeveloperHandoffPacket),
            "missing_execution_preparation_contract" => {
                Some(Self::MissingExecutionPreparationContract)
            }
            "execution_preparation_artifacts_unavailable" => {
                Some(Self::ExecutionPreparationArtifactsUnavailable)
            }
            "missing_execution_preparation_artifact_query_target" => {
                Some(Self::MissingExecutionPreparationArtifactQueryTarget)
            }
            "implementation_review_denied" => Some(Self::ImplementationReviewDenied),
            "implementation_review_expired" => Some(Self::ImplementationReviewExpired),
            "implementation_review_findings" => Some(Self::ImplementationReviewFindings),
            "implementation_review_changed_scope" => Some(Self::ImplementationReviewChangedScope),
            "bundle_activation_not_ready" => Some(Self::BundleActivationNotReady),
            "docflow_verdict_block" => Some(Self::DocflowVerdictBlock),
            "closure_admission_block" => Some(Self::ClosureAdmissionBlock),
            "unsupported_blocker_code" => Some(Self::Unsupported),
            _ => None,
        }
    }
}

const EXTENDED_BLOCKER_CODE_STRINGS: &[&str] = &[
    "ambiguous_unsafe_parallel_candidates",
    "candidate_scope_not_supported",
    "conflicting_owned_paths",
    "cyclic_dependency",
    "dev_team_disabled",
    "duplicate_task_id",
    "existing_task_conflict",
    "explicit_run_graph_continuation_binding_not_ready",
    "graph_blocked",
    "invalid_lanes_requested",
    "missing_acceptance_targets",
    "missing_dependency",
    "missing_dev_team_roles",
    "missing_dev_team_sequence",
    "missing_edge_source",
    "missing_edge_type",
    "missing_execution_semantics",
    "missing_lane_hint",
    "missing_order_bucket",
    "missing_owned_paths",
    "missing_parent",
    "missing_proof_targets",
    "missing_title",
    "model_selection_disabled",
    "no_dispatch_lanes_selected",
    "no_ready_task_candidates",
    "release_build_failed",
    "requested_current_task_not_ready",
    "route_fields_not_behavioral",
    "route_missing",
    "scheduler_packet_dispatch_failed",
    "scheduler_packet_dispatch_no_execution_evidence",
    "selected_backend_missing",
    "selected_backend_not_admissible_for_dispatch_target",
    "selected_backend_not_ready",
    "selected_lane_runtime_assignment_truth_required",
    "selected_model_profile_missing",
    "selected_model_profile_not_ready",
    "self_dependency",
    "task_not_in_graph_projection",
];

pub(crate) fn canonical_blocker_code_str(value: &str) -> Option<&'static str> {
    let trimmed = value.trim();
    BlockerCode::from_str(trimmed)
        .map(BlockerCode::as_str)
        .or_else(|| {
            EXTENDED_BLOCKER_CODE_STRINGS
                .iter()
                .copied()
                .find(|code| *code == trimmed)
        })
}

const BLOCKER_FAMILY_NAMES: &[&str] = &[
    "metadata",
    "control_core",
    "activation_bundle",
    "protocol_binding_registry",
    "cache_delivery_contract",
    "orchestrator_init_view",
    "agent_init_view",
];
const CACHE_KEY_INPUT_KEYS: &[&str] = &[
    "source_version_tuple",
    "project_activation_revision",
    "protocol_binding_revision",
    "protocol_binding_cache_token",
    "startup_bundle_revision",
];
const INVALIDATION_TUPLE_KEYS: &[&str] = &[
    "framework_revision",
    "project_activation_revision",
    "protocol_binding_revision",
    "protocol_binding_cache_token",
    "startup_bundle_revision",
];
const METADATA_TUPLE_KEYS: &[&str] = &[
    "framework_revision",
    "project_activation_revision",
    "protocol_binding_revision",
    "protocol_binding_cache_token",
];
const CACHE_KEY_MISMATCH_KEYS: &[&str] = &[
    "project_activation_revision",
    "protocol_binding_revision",
    "protocol_binding_cache_token",
];
const RETRIEVAL_OPTIONAL_BOUNDARY_KEYS: &[&str] = &[
    "full_project_owner_protocols",
    "non_promoted_project_docs",
    "broad_repo_manual_scan",
];
const RETRIEVAL_TRUST_EVIDENCE_KEYS: &[&str] = &[
    "source",
    "source_registry_ref",
    "citation",
    "freshness",
    "freshness_posture",
    "acl",
    "acl_context",
    "acl_propagation",
];

fn canonical_parametric_blocker_code_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if BLOCKER_FAMILY_NAMES
        .iter()
        .any(|family| trimmed == format!("missing_{family}_family"))
    {
        return Some(trimmed.to_string());
    }
    if trimmed == "missing_triggered_domain_bundle_partition"
        || trimmed == "cache_registry_contract_missing_triggered_domain_binding"
        || trimmed == "missing_retrieval_only_optional_context_boundary"
        || trimmed == "missing_retrieval_trust_evidence"
    {
        return Some(trimmed.to_string());
    }
    canonical_parametric_blocker_code_with_suffix(
        trimmed,
        "missing_cache_key_input:",
        CACHE_KEY_INPUT_KEYS,
    )
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "missing_invalidation_tuple_key:",
            INVALIDATION_TUPLE_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "invalid_cache_key_input:",
            CACHE_KEY_INPUT_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "invalid_invalidation_tuple_key:",
            INVALIDATION_TUPLE_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "missing_metadata_tuple_key:",
            METADATA_TUPLE_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "invalid_metadata_tuple_key:",
            METADATA_TUPLE_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "cache_key_mismatch:",
            CACHE_KEY_MISMATCH_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "invalidation_tuple_mismatch:",
            INVALIDATION_TUPLE_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "missing_retrieval_optional_boundary_entry:",
            RETRIEVAL_OPTIONAL_BOUNDARY_KEYS,
        )
    })
    .or_else(|| {
        canonical_parametric_blocker_code_with_suffix(
            trimmed,
            "missing_retrieval_trust_evidence_field:",
            RETRIEVAL_TRUST_EVIDENCE_KEYS,
        )
    })
}

fn canonical_parametric_blocker_code_with_suffix(
    value: &str,
    prefix: &str,
    allowed_suffixes: &[&str],
) -> Option<String> {
    let suffix = value.strip_prefix(prefix)?;
    allowed_suffixes
        .contains(&suffix)
        .then(|| value.to_string())
}

pub(crate) fn canonical_blocker_code_value_from_str(value: &str) -> Option<String> {
    let trimmed = value.trim();
    canonical_blocker_code_str(trimmed)
        .map(str::to_string)
        .or_else(|| canonical_parametric_blocker_code_value(trimmed))
}

pub(crate) fn canonical_blocker_code_list<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .filter_map(|value| canonical_blocker_code_value_from_str(value.as_ref()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn blocker_code_str(code: BlockerCode) -> &'static str {
    canonical_blocker_code_str(code.as_str()).unwrap_or(code.as_str())
}

pub(crate) fn blocker_code_value(code: BlockerCode) -> Option<String> {
    canonical_blocker_code_list([code.as_str()])
        .into_iter()
        .next()
}

const CLI_PROBE_TOOL_CONTRACT_ARTIFACT_ID: &str = "status_surface.external_cli_preflight";
const CLI_PROBE_TOOL_CONTRACT_TOOL_ID: &str = "status_surface.external_cli_preflight";
const CLI_PROBE_TOOL_CONTRACT_TOOL_VERSION: &str = "release-1-v1";
const CLI_PROBE_TOOL_CONTRACT_TOOL_NAME: &str = "External CLI Preflight";
const CLI_PROBE_TOOL_CONTRACT_OPERATION_CLASS: &str = "preflight_probe";
const CLI_PROBE_TOOL_CONTRACT_SIDE_EFFECT_CLASS: &str = "read_only_status_probe";
const CLI_PROBE_TOOL_CONTRACT_IDEMPOTENCY_CLASS: &str = "read_only_probe";
const CLI_PROBE_TOOL_CONTRACT_ROLLBACK_POSTURE: &str = "not_applicable";
const CLI_PROBE_TOOL_CONTRACT_INPUT_SCHEMA_REF: &str =
    "status_surface.external_cli_preflight.input_schema.v1";
const CLI_PROBE_TOOL_CONTRACT_OUTPUT_SCHEMA_REF: &str =
    "status_surface.external_cli_preflight.output_schema.v1";
const CLI_PROBE_TRACE_BASELINE_ARTIFACT_ID: &str = "status_surface.external_cli_preflight.trace";
const CLI_PROBE_TRACE_BASELINE_TRACE_ID: &str = "status_surface.external_cli_preflight.trace";
const CLI_PROBE_TRACE_BASELINE_SPAN_ID: &str = "status_surface.external_cli_preflight.span";
const CLI_PROBE_TRACE_BASELINE_WORKFLOW_RUN_ID: &str = "status_surface.external_cli_preflight.run";
const CLI_PROBE_INCIDENT_BASELINE_ARTIFACT_ID: &str =
    "status_surface.external_cli_preflight.incident";
const CLI_PROBE_INCIDENT_BASELINE_INCIDENT_ID: &str =
    "status_surface.external_cli_preflight.incident";
const CLI_PROBE_TOOL_CONTRACT_POLICY_HOOK_IDS: &[&str] = &[
    "execution_class_gate",
    "runtime_root_resolution",
    "sandbox_network_gate",
];
const CLI_PROBE_TOOL_CONTRACT_OBSERVABILITY_REQUIREMENTS: &[&str] =
    &["status_snapshot", "blocker_code", "next_actions"];

fn cli_probe_tool_contract_auth_mode(selected_execution_class: &str) -> &'static str {
    match selected_execution_class.trim() {
        "external" => "delegated_host_session",
        "internal" => "project_runtime_internal",
        _ => "unknown",
    }
}

fn cli_probe_now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render")
}

pub(crate) fn cli_probe_tool_contract_blocker_code(
    selected_execution_class: &str,
    selected_cli_entry_present: bool,
    runtime_root_configured: bool,
) -> Option<BlockerCode> {
    if !selected_cli_entry_present {
        return Some(BlockerCode::ToolContractMissing);
    }
    if cli_probe_tool_contract_auth_mode(selected_execution_class) == "unknown"
        || !runtime_root_configured
    {
        return Some(BlockerCode::ToolContractIncomplete);
    }
    None
}

pub(crate) fn cli_probe_tool_contract_summary(
    selected_execution_class: &str,
    requires_external_cli: bool,
    selected_cli_entry_present: bool,
    runtime_root_configured: bool,
) -> serde_json::Value {
    let blocker_code = cli_probe_tool_contract_blocker_code(
        selected_execution_class,
        selected_cli_entry_present,
        runtime_root_configured,
    );
    let auth_mode = cli_probe_tool_contract_auth_mode(selected_execution_class);
    serde_json::json!({
        "artifact_id": CLI_PROBE_TOOL_CONTRACT_ARTIFACT_ID,
        "artifact_type": "tool_contract",
        "status": if blocker_code.is_some() { "blocked" } else { "pass" },
        "blocker_code": blocker_code
            .map(|code| serde_json::Value::String(blocker_code_str(code).to_string()))
            .unwrap_or(serde_json::Value::Null),
        "tool_id": CLI_PROBE_TOOL_CONTRACT_TOOL_ID,
        "tool_version": CLI_PROBE_TOOL_CONTRACT_TOOL_VERSION,
        "tool_name": CLI_PROBE_TOOL_CONTRACT_TOOL_NAME,
        "operation_class": CLI_PROBE_TOOL_CONTRACT_OPERATION_CLASS,
        "side_effect_class": CLI_PROBE_TOOL_CONTRACT_SIDE_EFFECT_CLASS,
        "auth_mode": auth_mode,
        "approval_required": false,
        "idempotency_class": CLI_PROBE_TOOL_CONTRACT_IDEMPOTENCY_CLASS,
        "retry_posture": if requires_external_cli {
            "retry_on_transient_external_cli_probe_failure"
        } else {
            "single_probe"
        },
        "rollback_posture": CLI_PROBE_TOOL_CONTRACT_ROLLBACK_POSTURE,
        "input_schema_ref": CLI_PROBE_TOOL_CONTRACT_INPUT_SCHEMA_REF,
        "output_schema_ref": CLI_PROBE_TOOL_CONTRACT_OUTPUT_SCHEMA_REF,
        "policy_hook_ids": CLI_PROBE_TOOL_CONTRACT_POLICY_HOOK_IDS,
        "observability_requirements": CLI_PROBE_TOOL_CONTRACT_OBSERVABILITY_REQUIREMENTS,
    })
}

pub(crate) fn cli_probe_trace_baseline_summary(
    status: Release1ContractStatus,
    blocker_code: Option<BlockerCode>,
    selected_execution_class: &str,
) -> serde_json::Value {
    let now = cli_probe_now_rfc3339();
    serde_json::to_value(CanonicalTraceArtifact {
        trace_event: CanonicalTraceEvent {
            header: CanonicalArtifactHeader::new(
                CLI_PROBE_TRACE_BASELINE_ARTIFACT_ID,
                CanonicalArtifactType::TraceEvent,
                now.clone(),
                now.clone(),
                status.as_str(),
                "status_surface_external_cli",
                Some(CLI_PROBE_TRACE_BASELINE_TRACE_ID.to_string()),
                Some(WorkflowClass::ToolAssistedRead.as_str().to_string()),
            ),
            span_id: CLI_PROBE_TRACE_BASELINE_SPAN_ID.to_string(),
            parent_span_id: None,
            workflow_run_id: CLI_PROBE_TRACE_BASELINE_WORKFLOW_RUN_ID.to_string(),
            actor_kind: "status_surface".to_string(),
            actor_id: selected_execution_class.trim().to_string(),
            event_type: "external_cli_preflight_probe".to_string(),
            started_at: now.clone(),
            ended_at: now,
            outcome: if status == Release1ContractStatus::Pass {
                "succeeded".to_string()
            } else {
                "blocked".to_string()
            },
            side_effect_class: CLI_PROBE_TOOL_CONTRACT_SIDE_EFFECT_CLASS.to_string(),
            related_artifact_ids: vec![CLI_PROBE_TOOL_CONTRACT_ARTIFACT_ID.to_string()],
            policy_decision_ids: CLI_PROBE_TOOL_CONTRACT_POLICY_HOOK_IDS
                .iter()
                .map(|entry| (*entry).to_string())
                .collect(),
            approval_record_ids: blocker_code
                .map(blocker_code_str)
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
    })
    .expect("trace baseline summary should serialize")
}

pub(crate) fn cli_probe_incident_baseline_summary(
    blocker_code: Option<BlockerCode>,
) -> serde_json::Value {
    let now = cli_probe_now_rfc3339();
    serde_json::to_value(CanonicalIncidentEvidenceArtifact {
        incident_evidence_bundle: CanonicalIncidentEvidenceBundle {
            header: CanonicalArtifactHeader::new(
                CLI_PROBE_INCIDENT_BASELINE_ARTIFACT_ID,
                CanonicalArtifactType::IncidentEvidenceBundle,
                now.clone(),
                now.clone(),
                "open",
                "status_surface_external_cli",
                Some(CLI_PROBE_TRACE_BASELINE_TRACE_ID.to_string()),
                Some(
                    WorkflowClass::IncidentResponseOrRecovery
                        .as_str()
                        .to_string(),
                ),
            ),
            incident_id: CLI_PROBE_INCIDENT_BASELINE_INCIDENT_ID.to_string(),
            trace_ids: vec![CLI_PROBE_TRACE_BASELINE_ARTIFACT_ID.to_string()],
            trigger_reason: blocker_code
                .map(blocker_code_str)
                .map(|code| format!("external_cli_preflight_gate:{code}"))
                .unwrap_or_else(|| "external_cli_preflight_baseline_ready".to_string()),
            impact_summary: "Bounded incident evidence bundle path for external CLI preflight."
                .to_string(),
            side_effect_summary:
                "Preflight remains read-only while preserving incident escalation evidence."
                    .to_string(),
            rollback_or_restore_actions: vec![
                "Repair the selected host CLI runtime/tool contract.".to_string(),
                "Rerun `vida status --json` to refresh the bounded preflight baseline.".to_string(),
            ],
            recovery_outcome: if blocker_code.is_some() {
                "pending_remediation".to_string()
            } else {
                "not_required".to_string()
            },
            root_cause_status: if blocker_code.is_some() {
                "suspected".to_string()
            } else {
                "not_started".to_string()
            },
            opened_at: now,
            closed_at: None,
        },
    })
    .expect("incident baseline summary should serialize")
}

pub(crate) fn missing_family_blocker_code(family: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("missing_{family}_family"))
}

pub(crate) fn missing_cache_key_input_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("missing_cache_key_input:{key}"))
}

pub(crate) fn missing_invalidation_tuple_key_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("missing_invalidation_tuple_key:{key}"))
}

pub(crate) fn invalid_cache_key_input_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("invalid_cache_key_input:{key}"))
}

pub(crate) fn invalid_invalidation_tuple_key_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("invalid_invalidation_tuple_key:{key}"))
}

pub(crate) fn missing_metadata_tuple_key_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("missing_metadata_tuple_key:{key}"))
}

pub(crate) fn invalid_metadata_tuple_key_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("invalid_metadata_tuple_key:{key}"))
}

pub(crate) fn cache_key_mismatch_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("cache_key_mismatch:{key}"))
}

pub(crate) fn invalidation_tuple_mismatch_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("invalidation_tuple_mismatch:{key}"))
}

pub(crate) fn missing_retrieval_optional_boundary_entry_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!(
        "missing_retrieval_optional_boundary_entry:{key}"
    ))
}

pub(crate) fn missing_retrieval_trust_evidence_field_blocker_code(key: &str) -> Option<String> {
    canonical_blocker_code_value_from_str(&format!("missing_retrieval_trust_evidence_field:{key}"))
}

struct DecisionGateRule {
    gate_id: &'static str,
    missing_receipt_blocker: BlockerCode,
    not_ready_blocker: BlockerCode,
}

const DECISION_GATE_TABLE: &[DecisionGateRule] = &[DecisionGateRule {
    gate_id: "retrieval_evidence",
    missing_receipt_blocker: BlockerCode::MissingProtocolBindingReceipt,
    not_ready_blocker: BlockerCode::ProtocolBindingNotRuntimeReady,
}];

pub(crate) fn evaluate_policy_gate_protocol_binding(
    gate_id: &str,
    protocol_binding_receipt_id: Option<&str>,
    runtime_ready: bool,
) -> Option<BlockerCode> {
    let Some(rule) = DECISION_GATE_TABLE
        .iter()
        .find(|rule| rule.gate_id == gate_id.trim())
    else {
        return Some(BlockerCode::Unsupported);
    };

    if !has_evidence_id(protocol_binding_receipt_id) {
        return Some(rule.missing_receipt_blocker);
    }
    if !runtime_ready {
        return Some(rule.not_ready_blocker);
    }
    None
}

pub(crate) fn missing_downstream_lane_evidence_blocker(
    parsed_downstream_lane_status: Option<LaneStatus>,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> Option<BlockerCode> {
    if matches!(
        parsed_downstream_lane_status,
        Some(LaneStatus::LaneExceptionRecorded | LaneStatus::LaneExceptionTakeover)
    ) && !has_evidence_id(exception_path_receipt_id)
    {
        return Some(BlockerCode::ExceptionPathMissing);
    }
    if matches!(
        parsed_downstream_lane_status,
        Some(LaneStatus::LaneSuperseded)
    ) && !has_evidence_id(supersedes_receipt_id)
    {
        return Some(BlockerCode::MissingLaneReceipt);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        blocker_code_str, blocker_code_value, canonical_approval_status_str,
        canonical_artifact_type_str, canonical_blocker_code_list,
        canonical_compatibility_class_str, canonical_gate_level_str,
        canonical_release1_contract_status_str, canonical_release1_contract_type_str,
        canonical_release1_schema_version_str, canonical_risk_tier_str,
        canonical_workflow_class_str, classify_compatibility_boundary,
        cli_probe_incident_baseline_summary, cli_probe_tool_contract_summary,
        cli_probe_trace_baseline_summary, evaluate_policy_gate_protocol_binding,
        exception_takeover_state, missing_downstream_lane_evidence_blocker,
        release1_contract_status_str, ApprovalStatus, BlockerCode, CanonicalApprovalArtifact,
        CanonicalApprovalRecord, CanonicalArtifactHeader, CanonicalArtifactType,
        CanonicalClosureAdmissionArtifact, CanonicalClosureAdmissionRecord,
        CanonicalEvaluationArtifact, CanonicalEvaluationRun, CanonicalFeedbackArtifact,
        CanonicalFeedbackEvent, CanonicalIncidentEvidenceArtifact, CanonicalIncidentEvidenceBundle,
        CanonicalLaneExecutionReceipt, CanonicalLaneExecutionReceiptArtifact,
        CanonicalMemoryArtifact, CanonicalMemoryRecord, CanonicalPolicyDecision,
        CanonicalPolicyDecisionArtifact, CanonicalToolContract, CanonicalToolContractArtifact,
        CanonicalTraceArtifact, CanonicalTraceEvent, CompatibilityBoundary, CompatibilityClass,
        ExceptionTakeoverState, GateLevel, LaneStatus, Release1ContractStatus,
        Release1ContractType, Release1SchemaVersion, RiskTier, WorkflowClass,
    };

    #[test]
    fn blocker_code_normalization_round_trips_to_canonical_values() {
        assert_eq!(
            blocker_code_str(BlockerCode::MissingPacket),
            "missing_packet"
        );
        assert_eq!(
            blocker_code_value(BlockerCode::MissingLaneReceipt),
            Some("missing_lane_receipt".to_string())
        );
    }

    #[test]
    fn workflow_class_round_trips_to_canonical_values() {
        assert_eq!(
            canonical_workflow_class_str(WorkflowClass::ToolAssistedWrite.as_str()),
            Some("tool_assisted_write")
        );
        assert_eq!(
            canonical_workflow_class_str("incident_response_or_recovery"),
            Some("incident_response_or_recovery")
        );
        assert_eq!(canonical_workflow_class_str("unknown"), None);
    }

    #[test]
    fn risk_tier_round_trips_to_canonical_values() {
        assert_eq!(canonical_risk_tier_str(RiskTier::R0.as_str()), Some("R0"));
        assert_eq!(canonical_risk_tier_str(RiskTier::R4.as_str()), Some("R4"));
        assert_eq!(canonical_risk_tier_str("r1"), None);
    }

    #[test]
    fn approval_status_round_trips_to_canonical_values() {
        assert_eq!(
            canonical_approval_status_str(ApprovalStatus::ApprovalRequired.as_str()),
            Some("approval_required")
        );
        assert_eq!(
            canonical_approval_status_str(ApprovalStatus::WaitingForApproval.as_str()),
            Some("waiting_for_approval")
        );
        assert_eq!(canonical_approval_status_str("pending"), None);
    }

    #[test]
    fn canonical_artifact_type_round_trips_to_canonical_values() {
        assert_eq!(
            canonical_artifact_type_str(CanonicalArtifactType::TraceEvent.as_str()),
            Some("trace_event")
        );
        assert_eq!(
            canonical_artifact_type_str(CanonicalArtifactType::MemoryRecord.as_str()),
            Some("memory_record")
        );
        assert_eq!(canonical_artifact_type_str("not_an_artifact"), None);
    }

    #[test]
    fn canonical_artifact_header_uses_release1_schema_version() {
        let header = CanonicalArtifactHeader::new(
            "trace-1",
            CanonicalArtifactType::TraceEvent,
            "2026-04-18T10:00:00Z",
            "2026-04-18T10:01:00Z",
            "pass",
            "runtime_surface",
            Some("trace-root".to_string()),
            Some(
                WorkflowClass::DelegatedDevelopmentPacket
                    .as_str()
                    .to_string(),
            ),
        );

        assert_eq!(header.artifact_type, "trace_event");
        assert_eq!(header.schema_version, Release1SchemaVersion::V1.as_str());
        assert_eq!(header.trace_id.as_deref(), Some("trace-root"));
        assert_eq!(
            header.workflow_class.as_deref(),
            Some("delegated_development_packet")
        );
    }

    #[test]
    fn canonical_trace_artifact_serializes_required_release1_fields() {
        let artifact = CanonicalTraceEvent {
            header: CanonicalArtifactHeader::new(
                "trace-evt-1",
                CanonicalArtifactType::TraceEvent,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:01:00Z",
                "pass",
                "operator_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::ToolAssistedWrite.as_str().to_string()),
            ),
            span_id: "span-1".to_string(),
            parent_span_id: Some("span-0".to_string()),
            workflow_run_id: "run-1".to_string(),
            actor_kind: "worker_lane".to_string(),
            actor_id: "worker-7".to_string(),
            event_type: "implementation_completed".to_string(),
            started_at: "2026-04-18T10:00:00Z".to_string(),
            ended_at: "2026-04-18T10:01:00Z".to_string(),
            outcome: "succeeded".to_string(),
            side_effect_class: "schema_write".to_string(),
            related_artifact_ids: vec!["evaluation-1".to_string()],
            policy_decision_ids: vec!["policy-1".to_string()],
            approval_record_ids: vec!["approval-1".to_string()],
        };

        let value = serde_json::to_value(&artifact).expect("trace artifact should serialize");
        assert_eq!(value["artifact_type"], "trace_event");
        assert_eq!(value["span_id"], "span-1");
        assert_eq!(value["parent_span_id"], "span-0");
        assert_eq!(value["workflow_run_id"], "run-1");
        assert_eq!(value["policy_decision_ids"][0], "policy-1");
        assert_eq!(value["approval_record_ids"][0], "approval-1");
    }

    #[test]
    fn canonical_schema_named_artifacts_remain_compatible_with_explicit_artifact_wrappers() {
        let trace = CanonicalTraceEvent {
            header: CanonicalArtifactHeader::new(
                "trace-evt-1",
                CanonicalArtifactType::TraceEvent,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:01:00Z",
                "pass",
                "operator_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::ToolAssistedWrite.as_str().to_string()),
            ),
            span_id: "span-1".to_string(),
            parent_span_id: Some("span-0".to_string()),
            workflow_run_id: "run-1".to_string(),
            actor_kind: "worker_lane".to_string(),
            actor_id: "worker-7".to_string(),
            event_type: "implementation_completed".to_string(),
            started_at: "2026-04-18T10:00:00Z".to_string(),
            ended_at: "2026-04-18T10:01:00Z".to_string(),
            outcome: "succeeded".to_string(),
            side_effect_class: "schema_write".to_string(),
            related_artifact_ids: vec!["evaluation-1".to_string()],
            policy_decision_ids: vec!["policy-1".to_string()],
            approval_record_ids: vec!["approval-1".to_string()],
        };
        let trace_artifact = CanonicalTraceArtifact {
            trace_event: trace.clone(),
        };

        let policy = CanonicalPolicyDecision {
            header: CanonicalArtifactHeader::new(
                "policy-1",
                CanonicalArtifactType::PolicyDecision,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "pass",
                "policy_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::ToolAssistedWrite.as_str().to_string()),
            ),
            policy_id: "approval_gate".to_string(),
            policy_version: "2026-04-18".to_string(),
            actor_id: "system".to_string(),
            subject_id: "packet-1".to_string(),
            decision: "allow".to_string(),
            reason_codes: vec!["policy_satisfied".to_string()],
            constraints_applied: vec!["trace_required".to_string()],
            expires_at: Some("2026-04-19T10:00:00Z".to_string()),
        };
        let policy_artifact = CanonicalPolicyDecisionArtifact {
            policy_decision: policy.clone(),
        };

        let approval = CanonicalApprovalRecord {
            header: CanonicalArtifactHeader::new(
                "approval-1",
                CanonicalArtifactType::ApprovalRecord,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:05:00Z",
                "approved",
                "approval_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            approval_id: "approval-1".to_string(),
            approval_scope: "runtime-add-canonical-trace-policy-approval-tool".to_string(),
            requested_by: "worker-7".to_string(),
            approved_by: "reviewer-1".to_string(),
            decision: "approved".to_string(),
            decision_at: "2026-04-18T10:05:00Z".to_string(),
            decision_reason: "bounded schema-only slice".to_string(),
            expires_at: None,
            related_policy_decision_ids: vec!["policy-1".to_string()],
        };
        let approval_artifact = CanonicalApprovalArtifact {
            approval_record: approval.clone(),
        };

        let tool = CanonicalToolContract {
            header: CanonicalArtifactHeader::new(
                "tool-1",
                CanonicalArtifactType::ToolContract,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "pass",
                "status_surface",
                None,
                Some(WorkflowClass::ToolAssistedRead.as_str().to_string()),
            ),
            tool_id: "status_surface.external_cli_preflight".to_string(),
            tool_version: "release-1-v1".to_string(),
            tool_name: "External CLI Preflight".to_string(),
            operation_class: "preflight_probe".to_string(),
            side_effect_class: "read_only_status_probe".to_string(),
            auth_mode: "delegated_host_session".to_string(),
            approval_required: false,
            idempotency_class: "read_only_probe".to_string(),
            retry_posture: "single_probe".to_string(),
            rollback_posture: "not_applicable".to_string(),
            input_schema_ref: "input.schema.v1".to_string(),
            output_schema_ref: "output.schema.v1".to_string(),
            policy_hook_ids: vec!["execution_class_gate".to_string()],
            observability_requirements: vec!["status_snapshot".to_string()],
        };
        let tool_artifact = CanonicalToolContractArtifact {
            tool_contract: tool.clone(),
        };

        let lane_execution_receipt = CanonicalLaneExecutionReceipt {
            header: CanonicalArtifactHeader::new(
                "lane-receipt-1",
                CanonicalArtifactType::LaneExecutionReceipt,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:12:00Z",
                "executed",
                "lane_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            run_id: "run-1".to_string(),
            packet_id: "packet-1".to_string(),
            lane_id: "lane-impl-1".to_string(),
            lane_role: "implementer".to_string(),
            carrier_id: "internal.codex.middle".to_string(),
            lane_status: "lane_completed".to_string(),
            evidence_status: "recorded".to_string(),
            started_at: "2026-04-18T10:00:00Z".to_string(),
            finished_at: "2026-04-18T10:12:00Z".to_string(),
            result_artifact_ids: vec!["trace-evt-1".to_string(), "evaluation-1".to_string()],
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
        };
        let lane_execution_receipt_artifact = CanonicalLaneExecutionReceiptArtifact {
            lane_execution_receipt: lane_execution_receipt.clone(),
        };

        let evaluation = CanonicalEvaluationRun {
            header: CanonicalArtifactHeader::new(
                "evaluation-1",
                CanonicalArtifactType::EvaluationRun,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:10:00Z",
                "pass",
                "evaluation_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            evaluation_id: "eval-1".to_string(),
            evaluation_profile: "post-r1-schema-contract".to_string(),
            target_surface: "release1_contracts".to_string(),
            dataset_or_sample_window: "unit-tests".to_string(),
            metric_results: BTreeMap::from([("artifacts_added".to_string(), 7.0)]),
            regression_summary: "no regressions observed".to_string(),
            decision: "promote".to_string(),
            decision_reason: "required schema artifacts now explicit".to_string(),
            run_at: "2026-04-18T10:10:00Z".to_string(),
            trace_sample_refs: vec!["trace-evt-1".to_string()],
        };
        let evaluation_artifact = CanonicalEvaluationArtifact {
            evaluation_run: evaluation.clone(),
        };

        let feedback = CanonicalFeedbackEvent {
            header: CanonicalArtifactHeader::new(
                "feedback-1",
                CanonicalArtifactType::FeedbackEvent,
                "2026-04-18T10:11:00Z",
                "2026-04-18T10:11:00Z",
                "recorded",
                "agent_feedback_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            feedback_id: "feedback-1".to_string(),
            source_kind: "manual_feedback".to_string(),
            severity: "low".to_string(),
            feedback_type: "agent_runtime_feedback".to_string(),
            summary: "bounded success feedback".to_string(),
            linked_defect_or_remediation_id: Some("task-1".to_string()),
        };
        let feedback_artifact = CanonicalFeedbackArtifact {
            feedback_event: feedback.clone(),
        };

        let incident = CanonicalIncidentEvidenceBundle {
            header: CanonicalArtifactHeader::new(
                "incident-1",
                CanonicalArtifactType::IncidentEvidenceBundle,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:20:00Z",
                "open",
                "incident_surface",
                None,
                Some(
                    WorkflowClass::IncidentResponseOrRecovery
                        .as_str()
                        .to_string(),
                ),
            ),
            incident_id: "incident-1".to_string(),
            trace_ids: vec!["trace-1".to_string()],
            trigger_reason: "schema gap detected".to_string(),
            impact_summary: "artifact consumers could drift".to_string(),
            side_effect_summary: "operator/runtime surfaces could diverge".to_string(),
            rollback_or_restore_actions: vec!["restore previous contract module".to_string()],
            recovery_outcome: "mitigated".to_string(),
            root_cause_status: "confirmed".to_string(),
            opened_at: "2026-04-18T10:00:00Z".to_string(),
            closed_at: Some("2026-04-18T10:20:00Z".to_string()),
        };
        let incident_artifact = CanonicalIncidentEvidenceArtifact {
            incident_evidence_bundle: incident.clone(),
        };

        let memory = CanonicalMemoryRecord {
            header: CanonicalArtifactHeader::new(
                "memory-1",
                CanonicalArtifactType::MemoryRecord,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "active",
                "memory_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::MemoryWrite.as_str().to_string()),
            ),
            memory_id: "memory-1".to_string(),
            memory_class: "operator_preference".to_string(),
            subject_scope: "release1_contracts".to_string(),
            origin_trace_id: "trace-1".to_string(),
            origin_workflow_class: WorkflowClass::MemoryWrite.as_str().to_string(),
            sensitivity_level: "internal".to_string(),
            consent_basis: "operator_action".to_string(),
            ttl_policy: "retain_until_corrected".to_string(),
            deletion_or_correction_ref: Some("memory-correction-1".to_string()),
            approval_record_ids: vec!["approval-1".to_string()],
        };
        let memory_artifact = CanonicalMemoryArtifact {
            memory_record: memory.clone(),
        };

        let closure_admission_record = CanonicalClosureAdmissionRecord {
            header: CanonicalArtifactHeader::new(
                "closure-admission-1",
                CanonicalArtifactType::ClosureAdmissionRecord,
                "2026-04-18T10:20:00Z",
                "2026-04-18T10:20:00Z",
                "admitted",
                "runtime_consumption_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            release_scope: "release-1-artifact-schema-slice".to_string(),
            supported_workflow_classes: vec![
                WorkflowClass::DelegatedDevelopmentPacket
                    .as_str()
                    .to_string(),
                WorkflowClass::IdentityOrPolicyChange.as_str().to_string(),
            ],
            closure_decision: "admit".to_string(),
            decision_at: "2026-04-18T10:20:00Z".to_string(),
            decision_owner: "closure_surface".to_string(),
            evidence_bundle_refs: vec!["bundle-check-1".to_string(), "proof-1".to_string()],
            open_risk_acceptance_ids: vec!["risk-acceptance-1".to_string()],
            blocked_by: Vec::new(),
        };
        let closure_admission_artifact = CanonicalClosureAdmissionArtifact {
            closure_admission_record: closure_admission_record.clone(),
        };

        assert_eq!(
            serde_json::to_value(&trace).unwrap(),
            serde_json::to_value(&trace_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&policy).unwrap(),
            serde_json::to_value(&policy_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&approval).unwrap(),
            serde_json::to_value(&approval_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&tool).unwrap(),
            serde_json::to_value(&tool_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&lane_execution_receipt).unwrap(),
            serde_json::to_value(&lane_execution_receipt_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&evaluation).unwrap(),
            serde_json::to_value(&evaluation_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&feedback).unwrap(),
            serde_json::to_value(&feedback_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&incident).unwrap(),
            serde_json::to_value(&incident_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&memory).unwrap(),
            serde_json::to_value(&memory_artifact).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&closure_admission_record).unwrap(),
            serde_json::to_value(&closure_admission_artifact).unwrap()
        );
    }

    #[test]
    fn canonical_release1_artifacts_serialize_required_schema_specific_fields() {
        let policy = CanonicalPolicyDecision {
            header: CanonicalArtifactHeader::new(
                "policy-1",
                CanonicalArtifactType::PolicyDecision,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "pass",
                "policy_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::ToolAssistedWrite.as_str().to_string()),
            ),
            policy_id: "approval_gate".to_string(),
            policy_version: "2026-04-18".to_string(),
            actor_id: "system".to_string(),
            subject_id: "packet-1".to_string(),
            decision: "allow".to_string(),
            reason_codes: vec!["policy_satisfied".to_string()],
            constraints_applied: vec!["trace_required".to_string()],
            expires_at: Some("2026-04-19T10:00:00Z".to_string()),
        };
        let approval = CanonicalApprovalRecord {
            header: CanonicalArtifactHeader::new(
                "approval-1",
                CanonicalArtifactType::ApprovalRecord,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:05:00Z",
                "approved",
                "approval_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            approval_id: "approval-1".to_string(),
            approval_scope: "runtime-add-canonical-trace-policy-approval-tool".to_string(),
            requested_by: "worker-7".to_string(),
            approved_by: "reviewer-1".to_string(),
            decision: "approved".to_string(),
            decision_at: "2026-04-18T10:05:00Z".to_string(),
            decision_reason: "bounded schema-only slice".to_string(),
            expires_at: None,
            related_policy_decision_ids: vec!["policy-1".to_string()],
        };
        let tool_contract = CanonicalToolContract {
            header: CanonicalArtifactHeader::new(
                "tool-1",
                CanonicalArtifactType::ToolContract,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "pass",
                "status_surface",
                None,
                Some(WorkflowClass::ToolAssistedRead.as_str().to_string()),
            ),
            tool_id: "status_surface.external_cli_preflight".to_string(),
            tool_version: "release-1-v1".to_string(),
            tool_name: "External CLI Preflight".to_string(),
            operation_class: "preflight_probe".to_string(),
            side_effect_class: "read_only_status_probe".to_string(),
            auth_mode: "delegated_host_session".to_string(),
            approval_required: false,
            idempotency_class: "read_only_probe".to_string(),
            retry_posture: "single_probe".to_string(),
            rollback_posture: "not_applicable".to_string(),
            input_schema_ref: "input.schema.v1".to_string(),
            output_schema_ref: "output.schema.v1".to_string(),
            policy_hook_ids: vec!["execution_class_gate".to_string()],
            observability_requirements: vec!["status_snapshot".to_string()],
        };
        let evaluation = CanonicalEvaluationRun {
            header: CanonicalArtifactHeader::new(
                "evaluation-1",
                CanonicalArtifactType::EvaluationRun,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:10:00Z",
                "pass",
                "evaluation_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            evaluation_id: "eval-1".to_string(),
            evaluation_profile: "post-r1-schema-contract".to_string(),
            target_surface: "release1_contracts".to_string(),
            dataset_or_sample_window: "unit-tests".to_string(),
            metric_results: BTreeMap::from([("artifacts_added".to_string(), 7.0)]),
            regression_summary: "no regressions observed".to_string(),
            decision: "promote".to_string(),
            decision_reason: "required schema artifacts now explicit".to_string(),
            run_at: "2026-04-18T10:10:00Z".to_string(),
            trace_sample_refs: vec!["trace-evt-1".to_string()],
        };
        let lane_execution_receipt = CanonicalLaneExecutionReceipt {
            header: CanonicalArtifactHeader::new(
                "lane-receipt-1",
                CanonicalArtifactType::LaneExecutionReceipt,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:12:00Z",
                "executed",
                "lane_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            run_id: "run-1".to_string(),
            packet_id: "packet-1".to_string(),
            lane_id: "lane-impl-1".to_string(),
            lane_role: "implementer".to_string(),
            carrier_id: "internal.codex.middle".to_string(),
            lane_status: "lane_completed".to_string(),
            evidence_status: "recorded".to_string(),
            started_at: "2026-04-18T10:00:00Z".to_string(),
            finished_at: "2026-04-18T10:12:00Z".to_string(),
            result_artifact_ids: vec!["trace-evt-1".to_string(), "evaluation-1".to_string()],
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
        };
        let incident = CanonicalIncidentEvidenceBundle {
            header: CanonicalArtifactHeader::new(
                "incident-1",
                CanonicalArtifactType::IncidentEvidenceBundle,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:20:00Z",
                "open",
                "incident_surface",
                None,
                Some(
                    WorkflowClass::IncidentResponseOrRecovery
                        .as_str()
                        .to_string(),
                ),
            ),
            incident_id: "incident-1".to_string(),
            trace_ids: vec!["trace-1".to_string()],
            trigger_reason: "schema gap detected".to_string(),
            impact_summary: "artifact consumers could drift".to_string(),
            side_effect_summary: "operator/runtime surfaces could diverge".to_string(),
            rollback_or_restore_actions: vec!["restore previous contract module".to_string()],
            recovery_outcome: "mitigated".to_string(),
            root_cause_status: "confirmed".to_string(),
            opened_at: "2026-04-18T10:00:00Z".to_string(),
            closed_at: Some("2026-04-18T10:20:00Z".to_string()),
        };
        let memory = CanonicalMemoryRecord {
            header: CanonicalArtifactHeader::new(
                "memory-1",
                CanonicalArtifactType::MemoryRecord,
                "2026-04-18T10:00:00Z",
                "2026-04-18T10:00:00Z",
                "active",
                "memory_surface",
                Some("trace-1".to_string()),
                Some(WorkflowClass::MemoryWrite.as_str().to_string()),
            ),
            memory_id: "memory-1".to_string(),
            memory_class: "operator_preference".to_string(),
            subject_scope: "release1_contracts".to_string(),
            origin_trace_id: "trace-1".to_string(),
            origin_workflow_class: WorkflowClass::MemoryWrite.as_str().to_string(),
            sensitivity_level: "internal".to_string(),
            consent_basis: "operator_action".to_string(),
            ttl_policy: "retain_until_corrected".to_string(),
            deletion_or_correction_ref: Some("memory-correction-1".to_string()),
            approval_record_ids: vec!["approval-1".to_string()],
        };
        let closure_admission = CanonicalClosureAdmissionRecord {
            header: CanonicalArtifactHeader::new(
                "closure-admission-1",
                CanonicalArtifactType::ClosureAdmissionRecord,
                "2026-04-18T10:20:00Z",
                "2026-04-18T10:20:00Z",
                "admitted",
                "runtime_consumption_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            release_scope: "release-1-artifact-schema-slice".to_string(),
            supported_workflow_classes: vec![
                WorkflowClass::DelegatedDevelopmentPacket
                    .as_str()
                    .to_string(),
                WorkflowClass::IdentityOrPolicyChange.as_str().to_string(),
            ],
            closure_decision: "admit".to_string(),
            decision_at: "2026-04-18T10:20:00Z".to_string(),
            decision_owner: "closure_surface".to_string(),
            evidence_bundle_refs: vec!["bundle-check-1".to_string(), "proof-1".to_string()],
            open_risk_acceptance_ids: vec!["risk-acceptance-1".to_string()],
            blocked_by: Vec::new(),
        };
        let feedback = CanonicalFeedbackEvent {
            header: CanonicalArtifactHeader::new(
                "feedback-1",
                CanonicalArtifactType::FeedbackEvent,
                "2026-04-18T10:11:00Z",
                "2026-04-18T10:11:00Z",
                "recorded",
                "agent_feedback_surface",
                Some("trace-1".to_string()),
                Some(
                    WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ),
            ),
            feedback_id: "feedback-1".to_string(),
            source_kind: "manual_feedback".to_string(),
            severity: "low".to_string(),
            feedback_type: "agent_runtime_feedback".to_string(),
            summary: "bounded success feedback".to_string(),
            linked_defect_or_remediation_id: Some("task-1".to_string()),
        };

        let policy_value = serde_json::to_value(&policy).expect("policy should serialize");
        let approval_value = serde_json::to_value(&approval).expect("approval should serialize");
        let tool_value =
            serde_json::to_value(&tool_contract).expect("tool contract should serialize");
        let lane_value =
            serde_json::to_value(&lane_execution_receipt).expect("lane receipt should serialize");
        let evaluation_value =
            serde_json::to_value(&evaluation).expect("evaluation should serialize");
        let feedback_value = serde_json::to_value(&feedback).expect("feedback should serialize");
        let incident_value = serde_json::to_value(&incident).expect("incident should serialize");
        let memory_value = serde_json::to_value(&memory).expect("memory should serialize");
        let closure_value =
            serde_json::to_value(&closure_admission).expect("closure admission should serialize");

        assert_eq!(policy_value["artifact_type"], "policy_decision");
        assert_eq!(policy_value["reason_codes"][0], "policy_satisfied");
        assert_eq!(approval_value["artifact_type"], "approval_record");
        assert_eq!(approval_value["requested_by"], "worker-7");
        assert_eq!(tool_value["artifact_type"], "tool_contract");
        assert_eq!(tool_value["approval_required"], false);
        assert_eq!(lane_value["artifact_type"], "lane_execution_receipt");
        assert_eq!(lane_value["run_id"], "run-1");
        assert_eq!(lane_value["packet_id"], "packet-1");
        assert_eq!(lane_value["lane_id"], "lane-impl-1");
        assert_eq!(lane_value["carrier_id"], "internal.codex.middle");
        assert_eq!(lane_value["lane_status"], "lane_completed");
        assert_eq!(lane_value["evidence_status"], "recorded");
        assert_eq!(lane_value["result_artifact_ids"][0], "trace-evt-1");
        assert_eq!(evaluation_value["artifact_type"], "evaluation_run");
        assert_eq!(evaluation_value["metric_results"]["artifacts_added"], 7.0);
        assert_eq!(feedback_value["artifact_type"], "feedback_event");
        assert_eq!(feedback_value["source_kind"], "manual_feedback");
        assert_eq!(feedback_value["severity"], "low");
        assert_eq!(incident_value["artifact_type"], "incident_evidence_bundle");
        assert_eq!(incident_value["trace_ids"][0], "trace-1");
        assert_eq!(memory_value["artifact_type"], "memory_record");
        assert_eq!(memory_value["approval_record_ids"][0], "approval-1");
        assert_eq!(closure_value["artifact_type"], "closure_admission_record");
        assert_eq!(
            closure_value["release_scope"],
            "release-1-artifact-schema-slice"
        );
        assert_eq!(closure_value["closure_decision"], "admit");
        assert_eq!(closure_value["decision_owner"], "closure_surface");
        assert_eq!(closure_value["evidence_bundle_refs"][0], "bundle-check-1");
        assert_eq!(
            closure_value["supported_workflow_classes"][0],
            WorkflowClass::DelegatedDevelopmentPacket.as_str()
        );
    }

    #[test]
    fn gate_level_round_trips_to_canonical_values() {
        assert_eq!(
            canonical_gate_level_str(GateLevel::Block.as_str()),
            Some("block")
        );
        assert_eq!(
            canonical_gate_level_str(GateLevel::Warn.as_str()),
            Some("warn")
        );
        assert_eq!(canonical_gate_level_str("deny"), None);
    }

    #[test]
    fn lane_exception_takeover_requires_exception_receipt_evidence() {
        let blocker = missing_downstream_lane_evidence_blocker(
            Some(LaneStatus::LaneExceptionTakeover),
            None,
            None,
        );
        assert_eq!(blocker, Some(BlockerCode::ExceptionPathMissing));
    }

    #[test]
    fn downstream_lane_exception_recorded_guard_requires_exception_receipt_evidence() {
        let blocker = missing_downstream_lane_evidence_blocker(
            Some(LaneStatus::LaneExceptionRecorded),
            None,
            None,
        );
        assert_eq!(blocker, Some(BlockerCode::ExceptionPathMissing));
    }

    #[test]
    fn derive_lane_status_marks_exception_receipts_as_recorded_until_takeover_is_explicit() {
        assert_eq!(
            super::derive_lane_status("executed", None, Some("receipt-1")),
            LaneStatus::LaneExceptionRecorded
        );
    }

    #[test]
    fn derive_lane_status_marks_executing_dispatch_as_running() {
        assert_eq!(
            super::derive_lane_status("executing", None, None),
            LaneStatus::LaneRunning
        );
    }

    #[test]
    fn exception_takeover_state_distinguishes_recorded_and_active_authority() {
        assert_eq!(
            exception_takeover_state(
                Some("receipt-1"),
                None,
                Some("blocked_open_delegated_cycle")
            ),
            ExceptionTakeoverState::ReceiptRecorded
        );
        assert_eq!(
            exception_takeover_state(Some("receipt-1"), None, Some("delegated_cycle_clear")),
            ExceptionTakeoverState::ActiveTakeover
        );
        assert_eq!(
            exception_takeover_state(
                Some("receipt-1"),
                Some("supersede-1"),
                Some("blocked_open_delegated_cycle")
            ),
            ExceptionTakeoverState::ActiveTakeover
        );
    }

    #[test]
    fn downstream_lane_superseded_requires_supersedes_receipt_evidence() {
        let blocker =
            missing_downstream_lane_evidence_blocker(Some(LaneStatus::LaneSuperseded), None, None);
        assert_eq!(blocker, Some(BlockerCode::MissingLaneReceipt));
    }

    #[test]
    fn compatibility_class_round_trips_to_canonical_values() {
        assert_eq!(
            CompatibilityClass::BackwardCompatible.as_str(),
            "backward_compatible"
        );
        assert_eq!(
            CompatibilityClass::ReaderUpgradeRequired.as_str(),
            "reader_upgrade_required"
        );
        assert_eq!(
            canonical_compatibility_class_str("degraded"),
            Some("reader_upgrade_required")
        );
        assert_eq!(
            canonical_compatibility_class_str("blocking"),
            Some("reader_upgrade_required")
        );
        assert_eq!(
            canonical_compatibility_class_str("migration_required"),
            Some("migration_required")
        );
    }

    #[test]
    fn compatibility_class_rejects_unknown_values() {
        assert_eq!(
            canonical_compatibility_class_str("reader-upgrade-required"),
            None
        );
        assert_eq!(canonical_compatibility_class_str("unknown"), None);
    }

    #[test]
    fn lane_status_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            super::canonical_lane_status_str(" lane_running "),
            Some("lane_running")
        );
        assert_eq!(super::canonical_lane_status_str("lane_block"), None);
    }

    #[test]
    fn blocker_code_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            super::canonical_blocker_code_str(" missing_packet "),
            Some("missing_packet")
        );
    }

    #[test]
    fn canonical_blocker_code_list_dedupes_and_sorts() {
        let codes = canonical_blocker_code_list([
            " missing_lane_receipt ",
            "open_delegated_cycle",
            "missing_lane_receipt",
            "exception_path_missing",
            "approval_required",
            "protocol_binding_not_runtime_ready",
            "open_delegated_cycle",
        ]);
        assert_eq!(
            codes,
            vec![
                "approval_required".to_string(),
                "exception_path_missing".to_string(),
                "missing_lane_receipt".to_string(),
                "open_delegated_cycle".to_string(),
                "protocol_binding_not_runtime_ready".to_string()
            ]
        );
    }

    #[test]
    fn canonical_blocker_code_list_ignores_unknown_and_empty_values() {
        let codes = canonical_blocker_code_list([
            "missing_packet",
            "",
            " ",
            "MISSING_PACKET",
            "not_a_real_code",
            " missing_packet ",
        ]);
        assert_eq!(codes, vec!["missing_packet".to_string()]);
    }

    #[test]
    fn blocker_code_normalization_supports_consume_bundle_protocol_binding_codes() {
        let codes = canonical_blocker_code_list([
            " missing_protocol_binding_receipt ",
            "protocol_binding_not_runtime_ready",
            "policy_denied",
            "unsupported_blocker_code",
        ]);
        assert_eq!(
            codes,
            vec![
                "missing_protocol_binding_receipt".to_string(),
                "policy_denied".to_string(),
                "protocol_binding_not_runtime_ready".to_string(),
                "unsupported_blocker_code".to_string()
            ]
        );
    }

    #[test]
    fn blocker_code_normalization_supports_implementation_review_codes() {
        let codes = canonical_blocker_code_list([
            " implementation_review_denied ",
            "implementation_review_expired",
            "implementation_review_findings",
            "implementation_review_changed_scope",
        ]);
        assert_eq!(
            codes,
            vec![
                "implementation_review_changed_scope".to_string(),
                "implementation_review_denied".to_string(),
                "implementation_review_expired".to_string(),
                "implementation_review_findings".to_string(),
            ]
        );
    }

    #[test]
    fn blocker_code_normalization_supports_development_flow_completion_codes() {
        let codes = canonical_blocker_code_list([
            " pending_specification_evidence ",
            "pending_execution_preparation_evidence",
            "pending_approval_delegation_evidence",
            "pending_design_finalize",
            "pending_implementation_evidence",
            "pending_review_clean_evidence",
            "pending_verification_evidence",
            "pending_lane_evidence",
            "pending_review_findings",
            "pending_spec_task_close",
            "missing_execution_preparation_contract",
        ]);
        assert_eq!(
            codes,
            vec![
                "missing_execution_preparation_contract".to_string(),
                "pending_approval_delegation_evidence".to_string(),
                "pending_design_finalize".to_string(),
                "pending_execution_preparation_evidence".to_string(),
                "pending_implementation_evidence".to_string(),
                "pending_lane_evidence".to_string(),
                "pending_review_clean_evidence".to_string(),
                "pending_review_findings".to_string(),
                "pending_spec_task_close".to_string(),
                "pending_specification_evidence".to_string(),
                "pending_verification_evidence".to_string(),
            ]
        );
    }

    #[test]
    fn cli_probe_tool_contract_summary_is_canonical_for_internal_probe_paths() {
        let contract = cli_probe_tool_contract_summary("internal", false, true, true);

        assert_eq!(
            contract["artifact_id"],
            "status_surface.external_cli_preflight"
        );
        assert_eq!(contract["artifact_type"], "tool_contract");
        assert_eq!(contract["status"], "pass");
        assert!(contract["blocker_code"].is_null());
        assert_eq!(contract["tool_id"], "status_surface.external_cli_preflight");
        assert_eq!(contract["tool_version"], "release-1-v1");
        assert_eq!(contract["tool_name"], "External CLI Preflight");
        assert_eq!(contract["operation_class"], "preflight_probe");
        assert_eq!(contract["side_effect_class"], "read_only_status_probe");
        assert_eq!(contract["auth_mode"], "project_runtime_internal");
        assert_eq!(contract["approval_required"], false);
        assert_eq!(contract["idempotency_class"], "read_only_probe");
        assert_eq!(contract["retry_posture"], "single_probe");
        assert_eq!(contract["rollback_posture"], "not_applicable");
        assert_eq!(
            contract["policy_hook_ids"],
            serde_json::json!([
                "execution_class_gate",
                "runtime_root_resolution",
                "sandbox_network_gate"
            ])
        );
        assert_eq!(
            contract["observability_requirements"],
            serde_json::json!(["status_snapshot", "blocker_code", "next_actions"])
        );
    }

    #[test]
    fn cli_probe_tool_contract_summary_blocks_when_required_inputs_are_missing_or_incomplete() {
        let missing = cli_probe_tool_contract_summary("unknown", true, false, true);
        assert_eq!(missing["status"], "blocked");
        assert_eq!(
            missing["blocker_code"],
            serde_json::Value::String(
                blocker_code_str(BlockerCode::ToolContractMissing).to_string()
            )
        );

        let incomplete = cli_probe_tool_contract_summary("external", true, true, false);
        assert_eq!(incomplete["status"], "blocked");
        assert_eq!(
            incomplete["blocker_code"],
            serde_json::Value::String(
                blocker_code_str(BlockerCode::ToolContractIncomplete).to_string()
            )
        );
        assert_eq!(incomplete["auth_mode"], "delegated_host_session");
        assert_eq!(
            incomplete["retry_posture"],
            "retry_on_transient_external_cli_probe_failure"
        );
    }

    #[test]
    fn cli_probe_trace_baseline_summary_projects_canonical_trace_event_shape() {
        let trace = cli_probe_trace_baseline_summary(
            Release1ContractStatus::Blocked,
            Some(BlockerCode::ToolContractIncomplete),
            "external",
        );

        assert_eq!(trace["artifact_type"], "trace_event");
        assert_eq!(trace["status"], "blocked");
        assert_eq!(trace["owner_surface"], "status_surface_external_cli");
        assert_eq!(
            trace["trace_id"],
            "status_surface.external_cli_preflight.trace"
        );
        assert_eq!(trace["workflow_class"], "tool_assisted_read");
        assert_eq!(trace["event_type"], "external_cli_preflight_probe");
        assert_eq!(trace["side_effect_class"], "read_only_status_probe");
        assert_eq!(trace["approval_record_ids"][0], "tool_contract_incomplete");
    }

    #[test]
    fn cli_probe_incident_baseline_summary_projects_canonical_incident_bundle_shape() {
        let incident =
            cli_probe_incident_baseline_summary(Some(BlockerCode::ToolContractIncomplete));

        assert_eq!(incident["artifact_type"], "incident_evidence_bundle");
        assert_eq!(incident["status"], "open");
        assert_eq!(incident["owner_surface"], "status_surface_external_cli");
        assert_eq!(incident["workflow_class"], "incident_response_or_recovery");
        assert_eq!(
            incident["trigger_reason"],
            "external_cli_preflight_gate:tool_contract_incomplete"
        );
        assert_eq!(incident["recovery_outcome"], "pending_remediation");
        assert_eq!(incident["root_cause_status"], "suspected");
    }

    #[test]
    fn blocker_code_normalization_supports_parameterized_registry_families() {
        let codes = canonical_blocker_code_list([
            " missing_metadata_family ",
            "missing_cache_key_input:protocol_binding_revision",
            "invalid_invalidation_tuple_key:startup_bundle_revision",
            "cache_key_mismatch:protocol_binding_revision",
            "missing_retrieval_optional_boundary_entry:non_promoted_project_docs",
            "missing_retrieval_trust_evidence_field:source_registry_ref",
            "missing_retrieval_trust_evidence_field:acl",
            "missing_retrieval_trust_evidence",
        ]);
        assert_eq!(
            codes,
            vec![
                "cache_key_mismatch:protocol_binding_revision".to_string(),
                "invalid_invalidation_tuple_key:startup_bundle_revision".to_string(),
                "missing_cache_key_input:protocol_binding_revision".to_string(),
                "missing_metadata_family".to_string(),
                "missing_retrieval_optional_boundary_entry:non_promoted_project_docs".to_string(),
                "missing_retrieval_trust_evidence".to_string(),
                "missing_retrieval_trust_evidence_field:acl".to_string(),
                "missing_retrieval_trust_evidence_field:source_registry_ref".to_string(),
            ]
        );
    }

    #[test]
    fn blocker_code_normalization_rejects_unknown_parameterized_suffixes() {
        let codes = canonical_blocker_code_list([
            "missing_cache_key_input:not_real",
            "missing_retrieval_trust_evidence_field:not_real",
            "missing_unknown_family",
        ]);
        assert!(codes.is_empty());
    }

    #[test]
    fn explicit_blocker_push_literals_are_registry_backed() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = BTreeSet::new();

        for entry in std::fs::read_dir(src_dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("read source");
            for needle in ["blockers.push(\"", "blocker_codes.push(\""] {
                let mut rest = source.as_str();
                while let Some(idx) = rest.find(needle) {
                    let after = &rest[idx + needle.len()..];
                    let Some(end) = after.find('"') else {
                        break;
                    };
                    let candidate = &after[..end];
                    let simple_literal = candidate
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
                    if simple_literal && super::canonical_blocker_code_str(candidate).is_none() {
                        missing.insert(format!(
                            "{}:{}",
                            path.file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("unknown"),
                            candidate
                        ));
                    }
                    rest = &after[end + 1..];
                }
            }
        }

        assert!(
            missing.is_empty(),
            "found explicit blocker push literals outside registry: {missing:?}"
        );
    }

    #[test]
    fn completion_blocker_literals_are_registry_backed() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = BTreeSet::new();

        for entry in std::fs::read_dir(src_dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("read source");
            let needle = "\"completion_blocker\": \"";
            let mut rest = source.as_str();
            while let Some(idx) = rest.find(needle) {
                let after = &rest[idx + needle.len()..];
                let Some(end) = after.find('"') else {
                    break;
                };
                let candidate = &after[..end];
                let simple_literal = candidate
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
                if simple_literal && super::canonical_blocker_code_str(candidate).is_none() {
                    missing.insert(format!(
                        "{}:{}",
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("unknown"),
                        candidate
                    ));
                }
                rest = &after[end + 1..];
            }
        }

        assert!(
            missing.is_empty(),
            "found completion_blocker literals outside registry: {missing:?}"
        );
    }

    #[test]
    fn compatibility_class_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            canonical_compatibility_class_str(" compatible "),
            Some("backward_compatible")
        );
    }

    #[test]
    fn compatibility_boundary_classifier_fails_closed_for_unknown_values() {
        assert_eq!(
            classify_compatibility_boundary("compatible"),
            CompatibilityBoundary::Compatible
        );
        assert_eq!(
            classify_compatibility_boundary("degraded"),
            CompatibilityBoundary::BlockingSupported
        );
        assert_eq!(
            classify_compatibility_boundary("migration_required"),
            CompatibilityBoundary::BlockingSupported
        );
        assert_eq!(
            classify_compatibility_boundary("unsupported_value"),
            CompatibilityBoundary::Unsupported
        );
    }

    #[test]
    fn release1_contract_type_round_trips_to_canonical_values() {
        assert_eq!(
            Release1ContractType::OperatorContracts.as_str(),
            "release-1-operator-contracts"
        );
        assert_eq!(
            Release1ContractType::SharedFields.as_str(),
            "release-1-shared-fields"
        );
        assert_eq!(
            canonical_release1_contract_type_str(" release-1-operator-contracts "),
            Some("release-1-operator-contracts")
        );
        assert_eq!(
            canonical_release1_contract_type_str(" release-1-shared-fields "),
            Some("release-1-shared-fields")
        );
        assert_eq!(
            canonical_release1_contract_type_str("unknown-contract"),
            None
        );
    }

    #[test]
    fn release1_schema_version_round_trips_to_canonical_values() {
        assert_eq!(Release1SchemaVersion::V1.as_str(), "release-1-v1");
        assert_eq!(
            canonical_release1_schema_version_str(" release-1-v1 "),
            Some("release-1-v1")
        );
        assert_eq!(canonical_release1_schema_version_str("release-2-v1"), None);
    }

    #[test]
    fn release1_contract_status_round_trips_to_canonical_values() {
        assert_eq!(Release1ContractStatus::Pass.as_str(), "pass");
        assert_eq!(Release1ContractStatus::Blocked.as_str(), "blocked");
        assert_eq!(
            canonical_release1_contract_status_str(" pass "),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_contract_status_str(" blocked "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str(" ok "), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str(" BLOCK "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("unknown"), None);
    }

    #[test]
    fn release1_contract_status_emission_maps_bool_to_canonical_values() {
        assert_eq!(release1_contract_status_str(true), "pass");
        assert_eq!(release1_contract_status_str(false), "blocked");
    }

    #[test]
    fn retrieval_decision_gate_blocks_when_receipt_or_runtime_readiness_missing() {
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", None, false),
            Some(BlockerCode::MissingProtocolBindingReceipt)
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), false),
            Some(BlockerCode::ProtocolBindingNotRuntimeReady)
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), true),
            None
        );
    }

    #[test]
    fn decision_gate_fails_closed_for_unknown_gate_id() {
        assert_eq!(
            evaluate_policy_gate_protocol_binding("unknown_gate", Some("pb-1"), true),
            Some(BlockerCode::Unsupported)
        );
    }
}
