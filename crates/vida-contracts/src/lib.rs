use serde::{Deserialize, Serialize};

pub const VIDA_CONTRACTS_SCHEMA_VERSION: &str = "vida-contracts-v1";
pub const VIDA_COMMAND_PROTOCOL_VERSION: &str = "vida-command-v1";

pub mod operations {
    pub const SERVICE_HELLO: &str = "vida.service.hello";
    pub const SERVICE_STATUS: &str = "vida.service.status";
    pub const SERVICE_CAPABILITIES: &str = "vida.service.capabilities";
    pub const SERVICE_ENDPOINT_STATUS: &str = "vida.service.endpoint.status";
    pub const EVENTS_SINCE: &str = "vida.events.since";
    pub const SESSION_RESOLVE: &str = "vida.session.resolve";
    pub const PROJECT_RESOLVE: &str = "vida.project.resolve";
    pub const PROJECT_STATUS: &str = "vida.project.status";
    pub const PROJECT_REGISTRY_LIST: &str = "vida.project.registry.list";
    pub const PROJECT_REGISTRY_GET: &str = "vida.project.registry.get";
    pub const PROJECT_REGISTRY_DISCOVER: &str = "vida.project.registry.discover";
    pub const RECEIPTS_GET: &str = "vida.receipts.get";
    pub const WIZARD_SCHEMA_GET: &str = "vida.wizard.schema.get";
    pub const WIZARD_SESSION_START: &str = "vida.wizard.session.start";
    pub const WIZARD_SESSION_GET: &str = "vida.wizard.session.get";
    pub const WIZARD_SESSION_UPDATE_INPUT: &str = "vida.wizard.session.update_input";
    pub const WIZARD_SESSION_VALIDATE: &str = "vida.wizard.session.validate";
    pub const WIZARD_SESSION_DIFF: &str = "vida.wizard.session.diff";
    pub const JOBS_GET: &str = "vida.jobs.get";
}

pub fn mvp_operation_registry() -> Vec<VidaOperationSpec> {
    use VidaCapabilityScope::{
        ProjectRegistryRead, ReadEvents, ReadReceipts, ReadStatus, WizardPlan, WizardRead,
    };
    use operations::*;
    vec![
        VidaOperationSpec::read_with_capabilities(
            SERVICE_HELLO,
            VidaOperationScope::Service,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            SERVICE_STATUS,
            VidaOperationScope::Service,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            SERVICE_CAPABILITIES,
            VidaOperationScope::Service,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            SERVICE_ENDPOINT_STATUS,
            VidaOperationScope::Service,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            EVENTS_SINCE,
            VidaOperationScope::Service,
            vec![ReadEvents],
        ),
        VidaOperationSpec::read_with_capabilities(
            SESSION_RESOLVE,
            VidaOperationScope::Service,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            PROJECT_RESOLVE,
            VidaOperationScope::Project,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            PROJECT_STATUS,
            VidaOperationScope::Project,
            vec![ReadStatus],
        ),
        VidaOperationSpec::read_with_capabilities(
            PROJECT_REGISTRY_LIST,
            VidaOperationScope::Service,
            vec![ProjectRegistryRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            PROJECT_REGISTRY_GET,
            VidaOperationScope::Service,
            vec![ProjectRegistryRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            PROJECT_REGISTRY_DISCOVER,
            VidaOperationScope::Service,
            vec![ProjectRegistryRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            RECEIPTS_GET,
            VidaOperationScope::Project,
            vec![ReadReceipts],
        ),
        VidaOperationSpec::read_with_capabilities(
            WIZARD_SCHEMA_GET,
            VidaOperationScope::Project,
            vec![WizardRead],
        ),
        VidaOperationSpec::plan_mutation(
            WIZARD_SESSION_START,
            VidaOperationScope::Project,
            vec![WizardPlan],
        ),
        VidaOperationSpec::read_with_capabilities(
            WIZARD_SESSION_GET,
            VidaOperationScope::Project,
            vec![WizardRead],
        ),
        VidaOperationSpec::plan_mutation(
            WIZARD_SESSION_UPDATE_INPUT,
            VidaOperationScope::Project,
            vec![WizardPlan],
        ),
        VidaOperationSpec::plan_with_capabilities(
            WIZARD_SESSION_VALIDATE,
            VidaOperationScope::Project,
            vec![WizardPlan],
        ),
        VidaOperationSpec::plan_with_capabilities(
            WIZARD_SESSION_DIFF,
            VidaOperationScope::Project,
            vec![WizardPlan],
        ),
        VidaOperationSpec::read_with_capabilities(
            JOBS_GET,
            VidaOperationScope::Service,
            vec![ReadEvents],
        ),
    ]
}

pub fn operation_spec(operation_id: &str) -> Option<VidaOperationSpec> {
    mvp_operation_registry()
        .into_iter()
        .find(|spec| spec.operation.0 == operation_id)
}

pub fn unsupported_operation_problem(operation_id: &str) -> VidaProblem {
    VidaProblem {
        problem_type: "https://vida.dev/problems/unsupported-operation".to_string(),
        title: "Unsupported operation".to_string(),
        detail: format!(
            "Operation `{operation_id}` is not registered in the VIDA operation catalog."
        ),
        code: "unsupported_operation".to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: false,
        blockers: vec![VidaBlocker {
            code: "operation_not_registered".to_string(),
            scope: Some(operation_id.to_string()),
            next_actions: vec![
                "Use an operation id returned by the operation registry.".to_string(),
                "Add operation metadata before exposing a new command surface.".to_string(),
            ],
        }],
        remediation: vec![
            "Check the operation registry and retry with a supported operation id.".to_string(),
        ],
        instance: None,
        related_receipt: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaOperationSpec {
    pub operation: VidaOperation,
    pub scope: VidaOperationScope,
    pub posture: VidaOperationPosture,
    pub required_claim: VidaClaimKind,
    pub requires_project_ref: bool,
    pub requires_idempotency_key: bool,
    pub requires_apply_token: bool,
    pub required_capabilities: Vec<VidaCapabilityScope>,
}

impl VidaOperationSpec {
    #[must_use]
    pub fn read(operation: &str, scope: VidaOperationScope) -> Self {
        Self::read_with_capabilities(operation, scope, vec![VidaCapabilityScope::ReadStatus])
    }

    #[must_use]
    pub fn read_with_capabilities(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        Self::new(
            operation,
            scope,
            VidaOperationPosture::ReadOnly,
            required_capabilities,
            false,
            false,
        )
    }

    #[must_use]
    pub fn plan(operation: &str, scope: VidaOperationScope) -> Self {
        Self::plan_with_capabilities(operation, scope, vec![VidaCapabilityScope::WizardPlan])
    }

    #[must_use]
    pub fn plan_with_capabilities(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        Self::new(
            operation,
            scope,
            VidaOperationPosture::PlanOnly,
            required_capabilities,
            false,
            false,
        )
    }

    #[must_use]
    pub fn plan_mutation(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        Self::new(
            operation,
            scope,
            VidaOperationPosture::PlanOnly,
            required_capabilities,
            true,
            false,
        )
    }

    #[must_use]
    pub fn new(
        operation: &str,
        scope: VidaOperationScope,
        posture: VidaOperationPosture,
        required_capabilities: Vec<VidaCapabilityScope>,
        requires_idempotency_key: bool,
        requires_apply_token: bool,
    ) -> Self {
        Self {
            operation: VidaOperation(operation.to_string()),
            scope,
            posture,
            required_claim: VidaClaimKind::SharedRead,
            requires_project_ref: matches!(scope, VidaOperationScope::Project),
            requires_idempotency_key,
            requires_apply_token,
            required_capabilities,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationScope {
    Service,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationPosture {
    ReadOnly,
    PlanOnly,
    Apply,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaCapabilityScope {
    ReadStatus,
    ReadEvents,
    ReadReceipts,
    ReadConfig,
    ProjectRegistryRead,
    ProjectRegistryWrite,
    WizardRead,
    WizardPlan,
    WizardApply,
    ConfigPlan,
    ConfigApply,
    MaterializationPlan,
    MaterializationApply,
    ServiceInstallPlan,
    ServiceInstallApply,
    ServiceAdmin,
    DiagnosticDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaRequestId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaProjectId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaOperation(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaIdempotencyKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaApplyToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaEventCursor(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaJobRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VidaPlanRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WizardSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VidaProjectRef {
    ProjectId { project_id: VidaProjectId },
    RegistryEntry { registry_entry_id: String },
    RootPath { root_path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaClientKind {
    Cli,
    Tui,
    Service,
    Dashboard,
    HostAgent,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaClaimKind {
    Observe,
    SharedRead,
    ExclusiveWrite,
    Dispatch,
    Proof,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VidaCommandEnvelope {
    pub schema_version: String,
    pub protocol_version: String,
    pub operation: VidaOperation,
    pub session_id: VidaSessionId,
    pub request_id: VidaRequestId,
    pub client_kind: VidaClientKind,
    pub project_ref: Option<VidaProjectRef>,
    pub claim_kind: Option<VidaClaimKind>,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<VidaIdempotencyKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_token: Option<VidaApplyToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaResponseStatus {
    Pass,
    Blocked,
    Failed,
    Accepted,
    Running,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VidaCommandResponse {
    pub request_id: VidaRequestId,
    pub status: VidaResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<VidaProblem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_ref: Option<VidaReceiptRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_ref: Option<VidaJobRef>,
    #[serde(default)]
    pub blockers: Vec<VidaBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaBlocker {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaProblem {
    pub problem_type: String,
    pub title: String,
    pub detail: String,
    pub code: String,
    pub severity: VidaProblemSeverity,
    pub retryable: bool,
    #[serde(default)]
    pub blockers: Vec<VidaBlocker>,
    #[serde(default)]
    pub remediation: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_receipt: Option<VidaReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaProblemSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VidaEvent {
    pub event_id: String,
    pub request_id: VidaRequestId,
    pub session_id: VidaSessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<VidaProjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<VidaJobRef>,
    pub kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub cursor: VidaEventCursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaReceiptRef {
    pub receipt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<VidaProjectId>,
    pub operation: VidaOperation,
    pub scope: VidaReceiptScope,
    pub state_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaReceiptSummary {
    pub receipt_ref: VidaReceiptRef,
    pub status: VidaResponseStatus,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaReceiptScope {
    Service,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceProjectRegistryEntry {
    pub registry_entry_id: String,
    pub project_id: VidaProjectId,
    pub worktree_environment_id: String,
    pub root_path: String,
    pub registry_status: ProjectRegistryStatus,
    pub activation_status: ProjectActivationStatus,
    pub service_binding_status: ServiceBindingStatus,
    pub health: ProjectHealthSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRegistryStatus {
    Connected,
    Archived,
    Detached,
    UnhealthyMissingRoot,
    UnhealthyInaccessible,
    ConflictDuplicateProjectId,
    ConflictDuplicateRoot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectActivationStatus {
    NotActivated,
    ActivationPending,
    Activated,
    ReconfigurePending,
    ActivationBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceBindingStatus {
    NotBound,
    BoundCurrentService,
    BoundStaleService,
    BoundForeignService,
    BindingConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectHealthSummary {
    pub status: String,
    #[serde(default)]
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardKind {
    ProjectInit,
    ProjectRegister,
    Reconfigure,
    MaterializationUpdate,
    ServiceInstall,
    Repair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardSessionState {
    pub wizard_session_id: WizardSessionId,
    pub wizard_kind: WizardKind,
    pub session_id: VidaSessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<VidaProjectRef>,
    pub current_step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_preset: Option<String>,
    #[serde(default)]
    pub inputs: Vec<WizardOptionState>,
    #[serde(default)]
    pub validation_findings: Vec<WizardValidationFinding>,
    #[serde(default)]
    pub readiness_findings: Vec<WizardReadinessFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_ref: Option<VidaPlanRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_job_ref: Option<VidaJobRef>,
    #[serde(default)]
    pub receipt_refs: Vec<VidaReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardOptionSpec {
    pub option_id: String,
    pub label: String,
    pub value_type: WizardOptionValueType,
    pub required: bool,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardOptionValueType {
    String,
    Path,
    Boolean,
    Integer,
    Decimal,
    EnumOne,
    EnumMulti,
    OrderedList,
    Map,
    SecretRef,
    Computed,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardOptionState {
    pub option_id: String,
    pub value: WizardOptionValue,
    pub effective_value: WizardOptionValue,
    pub source: WizardOptionValueSource,
    pub visible: bool,
    pub enabled: bool,
    pub required: bool,
    pub dirty: bool,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning_reason: Option<String>,
    #[serde(default)]
    pub dependency_inputs: Vec<String>,
    #[serde(default)]
    pub affected_materialization_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum WizardOptionValue {
    String(String),
    Path(String),
    Boolean(bool),
    Integer(i64),
    Decimal(f64),
    List(Vec<String>),
    Map(serde_json::Map<String, serde_json::Value>),
    SecretRef(String),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardOptionValueSource {
    ConfigDefault,
    PresetDefault,
    InferredEnvironment,
    ExistingProject,
    OperatorInput,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardValidationFinding {
    pub option_id: String,
    pub code: String,
    pub message: String,
    pub severity: VidaProblemSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardReadinessFinding {
    pub code: String,
    pub message: String,
    pub blocker: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaConfigManifest {
    pub config_schema_version: String,
    pub config_generator_version: String,
    pub config_file_hash: String,
    pub config_semantic_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaProjectionManifest {
    pub artifact_id: String,
    pub path: String,
    pub artifact_kind: String,
    pub owner: MaterializationOwner,
    pub template_version: String,
    pub source_config_revision: String,
    pub last_generated_hash: String,
    pub current_hash: String,
    pub drift_status: MaterializationArtifactStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterializationOwner {
    VidaGenerated,
    UserOwned,
    Mixed,
    ExternalToolOwned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterializationArtifactStatus {
    Clean,
    Missing,
    GeneratedChangedByVersion,
    UserModified,
    MixedRegionChanged,
    Obsolete,
    Conflict,
    Untracked,
    UnsupportedFormat,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationUpdateReport {
    #[serde(default)]
    pub config_changes: Vec<String>,
    #[serde(default)]
    pub artifact_changes: Vec<String>,
    #[serde(default)]
    pub skipped_items: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VidaPlanSummary {
    pub plan_ref: VidaPlanRef,
    pub project_ref: VidaProjectRef,
    pub diff_summary: VidaDiffSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VidaDiffSummary {
    pub diff_hash: String,
    #[serde(default)]
    pub config_changes: Vec<String>,
    #[serde(default)]
    pub registry_changes: Vec<String>,
    #[serde(default)]
    pub materialization_changes: Vec<String>,
    #[serde(default)]
    pub service_changes: Vec<String>,
    #[serde(default)]
    pub runtime_impacts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardApplyJob {
    pub job_id: VidaJobRef,
    pub wizard_session_id: WizardSessionId,
    pub project_ref: VidaProjectRef,
    pub plan_id: VidaPlanRef,
    pub plan_hash: String,
    pub apply_token: VidaApplyToken,
    pub idempotency_key: VidaIdempotencyKey,
    pub status: VidaJobStatus,
    pub current_stage: String,
    #[serde(default)]
    pub receipt_refs: Vec<VidaReceiptRef>,
    pub event_cursor: VidaEventCursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VidaJobStatus {
    Queued,
    Running,
    Recovering,
    Resumable,
    Completed,
    FailedRecoverable,
    FailedTerminal,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_fixture_round_trips<T>(fixture: &str)
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let parsed: T = serde_json::from_str(fixture).expect("fixture should deserialize");
        let reparsed: serde_json::Value =
            serde_json::to_value(parsed).expect("fixture should serialize");
        let original: serde_json::Value =
            serde_json::from_str(fixture).expect("fixture should parse as JSON");
        assert_eq!(reparsed, original);
    }

    #[test]
    fn command_envelope_fixture_round_trips() {
        assert_fixture_round_trips::<VidaCommandEnvelope>(include_str!(
            "../fixtures/command_envelope.json"
        ));
    }

    #[test]
    fn command_response_problem_fixture_round_trips() {
        assert_fixture_round_trips::<VidaCommandResponse>(include_str!(
            "../fixtures/command_response_problem.json"
        ));
    }

    #[test]
    fn project_registry_fixture_round_trips() {
        assert_fixture_round_trips::<ServiceProjectRegistryEntry>(include_str!(
            "../fixtures/project_registry_entry.json"
        ));
    }

    #[test]
    fn wizard_session_fixture_round_trips() {
        assert_fixture_round_trips::<WizardSessionState>(include_str!(
            "../fixtures/wizard_session_state.json"
        ));
    }

    #[test]
    fn apply_job_fixture_round_trips() {
        assert_fixture_round_trips::<WizardApplyJob>(include_str!("../fixtures/apply_job.json"));
    }

    #[test]
    fn operation_registry_fixture_round_trips() {
        assert_fixture_round_trips::<Vec<VidaOperationSpec>>(include_str!(
            "../fixtures/operation_registry.json"
        ));
    }

    #[test]
    fn operation_registry_fixture_matches_mvp_registry() {
        let fixture: Vec<VidaOperationSpec> =
            serde_json::from_str(include_str!("../fixtures/operation_registry.json"))
                .expect("operation registry fixture should deserialize");
        assert_eq!(fixture, mvp_operation_registry());
        let fixture_value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/operation_registry.json"))
                .expect("operation registry fixture should parse as JSON");
        let registry_value =
            serde_json::to_value(mvp_operation_registry()).expect("registry should serialize");
        assert_eq!(fixture_value, registry_value);
    }

    #[test]
    fn operation_constants_are_registry_stable_strings() {
        assert_eq!(operations::SERVICE_HELLO, "vida.service.hello");
        assert_eq!(operations::WIZARD_SCHEMA_GET, "vida.wizard.schema.get");
        assert_eq!(operations::EVENTS_SINCE, "vida.events.since");
    }

    #[test]
    fn mvp_registry_marks_validation_as_plan_only_project_operation() {
        let spec = operation_spec(operations::WIZARD_SESSION_VALIDATE)
            .expect("wizard validate should be registered");
        assert_eq!(spec.posture, VidaOperationPosture::PlanOnly);
        assert_eq!(spec.scope, VidaOperationScope::Project);
        assert!(spec.requires_project_ref);
        assert!(!spec.requires_apply_token);
        assert!(
            spec.required_capabilities
                .contains(&VidaCapabilityScope::WizardPlan)
        );
    }

    #[test]
    fn mvp_registry_has_unique_operation_ids() {
        let registry = mvp_operation_registry();
        let mut ids = std::collections::BTreeSet::new();
        for spec in &registry {
            assert!(
                ids.insert(spec.operation.0.as_str()),
                "duplicate operation id `{}`",
                spec.operation.0
            );
        }
        assert_eq!(registry.len(), ids.len());
    }

    #[test]
    fn unsupported_operation_returns_problem_contract() {
        assert!(operation_spec("vida.unsupported.example").is_none());
        let problem = unsupported_operation_problem("vida.unsupported.example");
        assert_eq!(problem.code, "unsupported_operation");
        assert_eq!(problem.severity, VidaProblemSeverity::Error);
        assert!(!problem.retryable);
        assert_eq!(problem.blockers[0].code, "operation_not_registered");
    }
}
