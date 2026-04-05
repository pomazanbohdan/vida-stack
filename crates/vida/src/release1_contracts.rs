use std::collections::BTreeSet;

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

pub(crate) fn derive_lane_status(
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> LaneStatus {
    if has_evidence_id(exception_path_receipt_id) {
        return LaneStatus::LaneExceptionTakeover;
    }
    if has_evidence_id(supersedes_receipt_id) {
        return LaneStatus::LaneSuperseded;
    }
    match dispatch_status {
        "packet_ready" => LaneStatus::PacketReady,
        "routed" => LaneStatus::LaneOpen,
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
    MissingRunGraphDispatchReceiptOperatorEvidence,
    RunGraphLatestSnapshotInconsistent,
    RunGraphLatestDispatchReceiptSignalAmbiguous,
    RunGraphLatestDispatchReceiptSummaryInconsistent,
    RunGraphLatestDispatchReceiptCheckpointLeakage,
    ProjectActivationUnknown,
    DependencyGraphIssues,
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
    PendingDesignPacket,
    PendingDeveloperHandoffPacket,
    MissingExecutionPreparationContract,
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
            Self::MissingRunGraphDispatchReceiptOperatorEvidence => {
                "missing_run_graph_dispatch_receipt_operator_evidence"
            }
            Self::RunGraphLatestSnapshotInconsistent => "run_graph_latest_snapshot_inconsistent",
            Self::RunGraphLatestDispatchReceiptSignalAmbiguous => {
                "run_graph_latest_dispatch_receipt_signal_ambiguous"
            }
            Self::RunGraphLatestDispatchReceiptSummaryInconsistent => {
                "run_graph_latest_dispatch_receipt_summary_inconsistent"
            }
            Self::RunGraphLatestDispatchReceiptCheckpointLeakage => {
                "run_graph_latest_dispatch_receipt_checkpoint_leakage"
            }
            Self::ProjectActivationUnknown => "project_activation_unknown",
            Self::DependencyGraphIssues => "dependency_graph_issues",
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
            Self::PendingDesignPacket => "pending_design_packet",
            Self::PendingDeveloperHandoffPacket => "pending_developer_handoff_packet",
            Self::MissingExecutionPreparationContract => "missing_execution_preparation_contract",
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
            "missing_run_graph_dispatch_receipt_operator_evidence" => {
                Some(Self::MissingRunGraphDispatchReceiptOperatorEvidence)
            }
            "run_graph_latest_snapshot_inconsistent" => {
                Some(Self::RunGraphLatestSnapshotInconsistent)
            }
            "run_graph_latest_dispatch_receipt_signal_ambiguous" => {
                Some(Self::RunGraphLatestDispatchReceiptSignalAmbiguous)
            }
            "run_graph_latest_dispatch_receipt_summary_inconsistent" => {
                Some(Self::RunGraphLatestDispatchReceiptSummaryInconsistent)
            }
            "run_graph_latest_dispatch_receipt_checkpoint_leakage" => {
                Some(Self::RunGraphLatestDispatchReceiptCheckpointLeakage)
            }
            "project_activation_unknown" => Some(Self::ProjectActivationUnknown),
            "dependency_graph_issues" => Some(Self::DependencyGraphIssues),
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
            "pending_design_packet" => Some(Self::PendingDesignPacket),
            "pending_developer_handoff_packet" => Some(Self::PendingDeveloperHandoffPacket),
            "missing_execution_preparation_contract" => {
                Some(Self::MissingExecutionPreparationContract)
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

pub(crate) fn canonical_blocker_code_str(value: &str) -> Option<&'static str> {
    BlockerCode::from_str(value).map(BlockerCode::as_str)
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
const RETRIEVAL_TRUST_EVIDENCE_KEYS: &[&str] = &["source", "citation", "freshness", "acl"];

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
        Some(LaneStatus::LaneExceptionTakeover)
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
    use std::collections::BTreeSet;

    use super::{
        blocker_code_str, blocker_code_value, canonical_approval_status_str,
        canonical_blocker_code_list, canonical_compatibility_class_str, canonical_gate_level_str,
        canonical_release1_contract_status_str, canonical_release1_contract_type_str,
        canonical_release1_schema_version_str, canonical_risk_tier_str,
        canonical_workflow_class_str, classify_compatibility_boundary,
        evaluate_policy_gate_protocol_binding, missing_downstream_lane_evidence_blocker,
        release1_contract_status_str, ApprovalStatus, BlockerCode, CompatibilityBoundary,
        CompatibilityClass, GateLevel, LaneStatus, Release1ContractStatus, Release1ContractType,
        Release1SchemaVersion, RiskTier, WorkflowClass,
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
    fn lane_superseded_requires_supersedes_receipt_evidence() {
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
            "pending_implementation_evidence",
            "pending_review_clean_evidence",
            "pending_verification_evidence",
            "pending_lane_evidence",
            "pending_review_findings",
            "missing_execution_preparation_contract",
        ]);
        assert_eq!(
            codes,
            vec![
                "missing_execution_preparation_contract".to_string(),
                "pending_approval_delegation_evidence".to_string(),
                "pending_execution_preparation_evidence".to_string(),
                "pending_implementation_evidence".to_string(),
                "pending_lane_evidence".to_string(),
                "pending_review_clean_evidence".to_string(),
                "pending_review_findings".to_string(),
                "pending_specification_evidence".to_string(),
                "pending_verification_evidence".to_string(),
            ]
        );
    }

    #[test]
    fn blocker_code_normalization_supports_parameterized_registry_families() {
        let codes = canonical_blocker_code_list([
            " missing_metadata_family ",
            "missing_cache_key_input:protocol_binding_revision",
            "invalid_invalidation_tuple_key:startup_bundle_revision",
            "cache_key_mismatch:protocol_binding_revision",
            "missing_retrieval_optional_boundary_entry:non_promoted_project_docs",
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
