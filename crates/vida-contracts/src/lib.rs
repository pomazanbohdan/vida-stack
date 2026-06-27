use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

pub const VIDA_CONTRACTS_SCHEMA_VERSION: &str = "vida-contracts-v1";
pub const VIDA_COMMAND_PROTOCOL_VERSION: &str = "vida-command-v1";
pub const VIDA_RUNTIME_CONTRACTS_V1_SCHEMA_VERSION: &str = "vida-runtime-contracts-v1";
pub const VIDA_RUNTIME_ENGINE_CONTRACT_VERSION: &str = "vida-runtime-engine-v1";

pub mod operations {
    pub const SERVICE_HELLO: &str = "vida.service.hello";
    pub const SERVICE_STATUS: &str = "vida.service.status";
    pub const SERVICE_CAPABILITIES: &str = "vida.service.capabilities";
    pub const SERVICE_ENDPOINT_STATUS: &str = "vida.service.endpoint.status";
    pub const SERVICE_LIFECYCLE_PLAN: &str = "vida.service.lifecycle.plan";
    pub const SERVICE_LIFECYCLE_STATUS: &str = "vida.service.lifecycle.status";
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
    pub const WIZARD_SESSION_APPLY: &str = "vida.wizard.session.apply";
    pub const MATERIALIZATION_MANIFEST_GET: &str = "vida.materialization.manifest.get";
    pub const MATERIALIZATION_DRIFT_CLASSIFY: &str = "vida.materialization.drift.classify";
    pub const MATERIALIZATION_UPDATE_PLAN: &str = "vida.materialization.update.plan";
    pub const MATERIALIZATION_RECEIPTS_LIST: &str = "vida.materialization.receipts.list";
    pub const ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET: &str =
        "vida.orchestration.control_plane.summary.get";
    pub const JOBS_GET: &str = "vida.jobs.get";
    pub const TASK_APPLY: &str = "vida.task.apply";
    pub const RUN_ADVANCE: &str = "vida.run.advance";
    pub const COMPLETION_RECORD: &str = "vida.completion.record";
    pub const PACKET_DISPATCH: &str = "vida.packet.dispatch";
    pub const CLAIM_ACQUIRE: &str = "vida.claim.acquire";
    pub const PROJECTION_REBUILD: &str = "vida.projection.rebuild";
    pub const REPAIR_APPLY: &str = "vida.repair.apply";
    pub const SERVICE_LIFECYCLE_APPLY: &str = "vida.service.lifecycle.apply";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaLegacyOperationAlias {
    pub alias: String,
    pub canonical_operation: VidaOperation,
    pub deprecated_since: String,
    pub removal_target: Option<String>,
    pub receipt_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VidaOperationAliasResolution {
    Canonical {
        operation: VidaOperation,
    },
    Alias {
        alias: String,
        operation: VidaOperation,
        receipt: VidaLegacyOperationAlias,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VidaOperationAliasProblem {
    pub alias: String,
    pub blocker_code: String,
    pub message: String,
    pub candidates: Vec<VidaOperation>,
}

impl std::fmt::Display for VidaOperationAliasProblem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for VidaOperationAliasProblem {}

pub fn legacy_operation_aliases() -> Vec<VidaLegacyOperationAlias> {
    use operations::*;
    [
        ("service.hello", SERVICE_HELLO),
        ("service.status", SERVICE_STATUS),
        ("project.resolve", PROJECT_RESOLVE),
        ("project.status", PROJECT_STATUS),
        ("task.apply", TASK_APPLY),
        ("run.advance", RUN_ADVANCE),
        ("completion.record", COMPLETION_RECORD),
        ("packet.dispatch", PACKET_DISPATCH),
        ("claim.acquire", CLAIM_ACQUIRE),
        ("projection.rebuild", PROJECTION_REBUILD),
        ("repair.apply", REPAIR_APPLY),
    ]
    .into_iter()
    .map(|(alias, canonical)| VidaLegacyOperationAlias {
        alias: alias.to_string(),
        canonical_operation: VidaOperation(canonical.to_string()),
        deprecated_since: "vida-contracts-v1".to_string(),
        removal_target: Some("Use the vida.* canonical operation id.".to_string()),
        receipt_code: "legacy_operation_alias_used".to_string(),
    })
    .collect()
}

pub fn resolve_operation_alias(
    operation_id: &str,
) -> Result<VidaOperationAliasResolution, VidaOperationAliasProblem> {
    if mvp_operation_registry()
        .iter()
        .any(|spec| spec.operation.0 == operation_id)
    {
        return Ok(VidaOperationAliasResolution::Canonical {
            operation: VidaOperation(operation_id.to_string()),
        });
    }

    if operation_id == "status" {
        return Err(VidaOperationAliasProblem {
            alias: operation_id.to_string(),
            blocker_code: "ambiguous_legacy_operation_alias".to_string(),
            message:
                "Legacy operation alias `status` is ambiguous; use a canonical vida.* operation id."
                    .to_string(),
            candidates: vec![
                VidaOperation(operations::SERVICE_STATUS.to_string()),
                VidaOperation(operations::PROJECT_STATUS.to_string()),
            ],
        });
    }

    let matches: Vec<_> = legacy_operation_aliases()
        .into_iter()
        .filter(|alias| alias.alias == operation_id)
        .collect();

    match matches.as_slice() {
        [receipt] => Ok(VidaOperationAliasResolution::Alias {
            alias: operation_id.to_string(),
            operation: receipt.canonical_operation.clone(),
            receipt: receipt.clone(),
        }),
        [] => Ok(VidaOperationAliasResolution::Canonical {
            operation: VidaOperation(operation_id.to_string()),
        }),
        receipts => Err(VidaOperationAliasProblem {
            alias: operation_id.to_string(),
            blocker_code: "ambiguous_legacy_operation_alias".to_string(),
            message: format!(
                "Legacy operation alias `{operation_id}` resolves to multiple canonical operations."
            ),
            candidates: receipts
                .iter()
                .map(|receipt| receipt.canonical_operation.clone())
                .collect(),
        }),
    }
}

pub fn legacy_operation_alias_receipt(operation_id: &str) -> Option<VidaLegacyOperationAlias> {
    match resolve_operation_alias(operation_id).ok()? {
        VidaOperationAliasResolution::Alias { receipt, .. } => Some(receipt),
        VidaOperationAliasResolution::Canonical { .. } => None,
    }
}

pub fn mvp_operation_registry() -> Vec<VidaOperationSpec> {
    use VidaCapabilityScope::{
        ClaimWrite, CompletionRecord, MaterializationPlan, MaterializationRead,
        OrchestrationControlPlaneRead, PacketDispatch, ProjectRegistryRead, ProjectionRebuild,
        ReadEvents, ReadReceipts, ReadStatus, RepairApply, RunAdvance, ServiceAdmin,
        ServiceInstallApply, ServiceInstallPlan, TaskApply, WizardApply, WizardPlan, WizardRead,
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
        VidaOperationSpec::plan_with_capabilities(
            SERVICE_LIFECYCLE_PLAN,
            VidaOperationScope::Service,
            vec![ServiceInstallPlan],
        ),
        VidaOperationSpec::read_with_capabilities(
            SERVICE_LIFECYCLE_STATUS,
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
        VidaOperationSpec::apply_mutation(
            WIZARD_SESSION_APPLY,
            VidaOperationScope::Project,
            vec![WizardApply],
        ),
        VidaOperationSpec::read_with_capabilities(
            MATERIALIZATION_MANIFEST_GET,
            VidaOperationScope::Project,
            vec![MaterializationRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            MATERIALIZATION_DRIFT_CLASSIFY,
            VidaOperationScope::Project,
            vec![MaterializationRead],
        ),
        VidaOperationSpec::plan_with_capabilities(
            MATERIALIZATION_UPDATE_PLAN,
            VidaOperationScope::Project,
            vec![MaterializationPlan],
        ),
        VidaOperationSpec::read_with_capabilities(
            MATERIALIZATION_RECEIPTS_LIST,
            VidaOperationScope::Project,
            vec![ReadReceipts, MaterializationRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET,
            VidaOperationScope::Project,
            vec![OrchestrationControlPlaneRead],
        ),
        VidaOperationSpec::read_with_capabilities(
            JOBS_GET,
            VidaOperationScope::Service,
            vec![ReadEvents],
        ),
        VidaOperationSpec::apply_mutation(TASK_APPLY, VidaOperationScope::Project, vec![TaskApply]),
        VidaOperationSpec::apply_mutation(
            RUN_ADVANCE,
            VidaOperationScope::Project,
            vec![RunAdvance],
        ),
        VidaOperationSpec::apply_mutation(
            COMPLETION_RECORD,
            VidaOperationScope::Project,
            vec![CompletionRecord],
        ),
        VidaOperationSpec::automation_mutation(
            PACKET_DISPATCH,
            VidaOperationScope::Project,
            vec![PacketDispatch],
        ),
        VidaOperationSpec::apply_mutation(
            CLAIM_ACQUIRE,
            VidaOperationScope::Project,
            vec![ClaimWrite],
        ),
        VidaOperationSpec::admin_mutation(
            PROJECTION_REBUILD,
            VidaOperationScope::Project,
            vec![ProjectionRebuild],
        ),
        VidaOperationSpec::admin_mutation(
            REPAIR_APPLY,
            VidaOperationScope::Project,
            vec![RepairApply],
        ),
        VidaOperationSpec::apply_mutation(
            SERVICE_LIFECYCLE_APPLY,
            VidaOperationScope::Service,
            vec![ServiceInstallApply, ServiceAdmin],
        ),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaOperationCatalogEntry {
    pub operation: VidaOperation,
    pub scope: VidaOperationScope,
    pub posture: VidaOperationPosture,
    pub risk_tier: VidaRiskTier,
    pub allowed_client_kinds: Vec<VidaClientKind>,
    pub required_claim: VidaClaimKind,
    pub requires_project_ref: bool,
    pub requires_idempotency_key: bool,
    pub requires_apply_token: bool,
    pub required_capabilities: Vec<VidaCapabilityScope>,
    pub required_consistency: VidaConsistencyRequirement,
    pub automation_posture: VidaAutomationPosture,
    pub result_schema: VidaSchemaRef,
    pub input_schema: VidaOperationInputSchema,
}

impl VidaOperationCatalogEntry {
    #[must_use]
    pub fn from_spec(spec: VidaOperationSpec) -> Self {
        let input_schema = input_schema_for_spec(&spec);
        Self {
            operation: spec.operation,
            scope: spec.scope,
            posture: spec.posture,
            risk_tier: spec.risk_tier,
            allowed_client_kinds: spec.allowed_client_kinds,
            required_claim: spec.required_claim,
            requires_project_ref: spec.requires_project_ref,
            requires_idempotency_key: spec.requires_idempotency_key,
            requires_apply_token: spec.requires_apply_token,
            required_capabilities: spec.required_capabilities,
            required_consistency: spec.required_consistency,
            automation_posture: spec.automation_posture,
            result_schema: spec.result_schema,
            input_schema,
        }
    }
}

#[must_use]
pub fn mvp_operation_catalog() -> Vec<VidaOperationCatalogEntry> {
    mvp_operation_registry()
        .into_iter()
        .map(VidaOperationCatalogEntry::from_spec)
        .collect()
}

pub fn operation_spec(operation_id: &str) -> Option<VidaOperationSpec> {
    let operation_id = match resolve_operation_alias(operation_id) {
        Ok(VidaOperationAliasResolution::Canonical { operation })
        | Ok(VidaOperationAliasResolution::Alias { operation, .. }) => operation.0,
        Err(_) => return None,
    };
    mvp_operation_registry()
        .into_iter()
        .find(|spec| spec.operation.0 == operation_id)
}

#[must_use]
pub fn operation_input_schema(operation_id: &str) -> Option<VidaOperationInputSchema> {
    let operation_id = match resolve_operation_alias(operation_id) {
        Ok(VidaOperationAliasResolution::Canonical { operation })
        | Ok(VidaOperationAliasResolution::Alias { operation, .. }) => operation.0,
        Err(_) => return None,
    };
    mvp_operation_registry()
        .into_iter()
        .find(|spec| spec.operation.0 == operation_id)
        .map(|spec| input_schema_for_spec(&spec))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEngineCapability {
    DurableTimers,
    KeyedSerialization,
    Signals,
    Jobs,
    EventExport,
    StrongReads,
    OfflineMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeEngineCapabilitySupport {
    pub capability: RuntimeEngineCapability,
    pub supported: bool,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeEngineCapabilities {
    pub contract_version: String,
    pub engine_id: String,
    pub engine_kind: String,
    pub capabilities: Vec<RuntimeEngineCapabilitySupport>,
}

impl RuntimeEngineCapabilities {
    #[must_use]
    pub fn supports(&self, capability: RuntimeEngineCapability) -> bool {
        self.capabilities
            .iter()
            .any(|entry| entry.capability == capability && entry.supported)
    }

    #[must_use]
    pub fn unsupported(&self, capability: RuntimeEngineCapability) -> Option<&str> {
        self.capabilities
            .iter()
            .find(|entry| entry.capability == capability && !entry.supported)
            .and_then(|entry| entry.blocker_code.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeEngineHealth {
    pub engine_id: String,
    pub status: String,
    #[serde(default)]
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeQueryRequest {
    pub operation: VidaOperation,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeWatchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<VidaEventCursor>,
    pub required_capability: RuntimeEngineCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeWatchPlan {
    pub stream_kind: String,
    pub replayable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<VidaEventCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeEngineError {
    UnsupportedCapability {
        capability: RuntimeEngineCapability,
        blocker_code: String,
        remediation: String,
    },
    UnsupportedOperation {
        operation: VidaOperation,
        blocker_code: String,
    },
}

pub type RuntimeEngineResult<T> = Result<T, RuntimeEngineError>;

pub trait RuntimeEngine {
    fn capabilities(&self) -> RuntimeEngineCapabilities;
    fn health(&self) -> RuntimeEngineHealth;
    fn execute(&self, envelope: VidaCommandEnvelope) -> RuntimeEngineResult<VidaCommandResponse>;
    fn query(&self, request: RuntimeQueryRequest) -> RuntimeEngineResult<serde_json::Value>;
    fn watch(&self, request: RuntimeWatchRequest) -> RuntimeEngineResult<RuntimeWatchPlan>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaOperationSpec {
    pub operation: VidaOperation,
    pub scope: VidaOperationScope,
    pub posture: VidaOperationPosture,
    pub risk_tier: VidaRiskTier,
    pub allowed_client_kinds: Vec<VidaClientKind>,
    pub required_claim: VidaClaimKind,
    pub requires_project_ref: bool,
    pub requires_idempotency_key: bool,
    pub requires_apply_token: bool,
    pub required_capabilities: Vec<VidaCapabilityScope>,
    pub required_consistency: VidaConsistencyRequirement,
    pub automation_posture: VidaAutomationPosture,
    pub result_schema: VidaSchemaRef,
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
            VidaClaimKind::SharedRead,
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
            VidaClaimKind::SharedRead,
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
            VidaClaimKind::ExclusiveWrite,
            true,
            false,
        )
    }

    #[must_use]
    pub fn apply_mutation(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        Self::new(
            operation,
            scope,
            VidaOperationPosture::Apply,
            required_capabilities,
            VidaClaimKind::ExclusiveWrite,
            true,
            true,
        )
    }

    #[must_use]
    pub fn admin_mutation(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        Self::new(
            operation,
            scope,
            VidaOperationPosture::Admin,
            required_capabilities,
            VidaClaimKind::Admin,
            true,
            true,
        )
    }

    #[must_use]
    pub fn automation_mutation(
        operation: &str,
        scope: VidaOperationScope,
        required_capabilities: Vec<VidaCapabilityScope>,
    ) -> Self {
        let mut spec = Self::apply_mutation(operation, scope, required_capabilities);
        spec.required_claim = VidaClaimKind::Dispatch;
        spec.automation_posture = VidaAutomationPosture::Automation;
        spec
    }

    #[must_use]
    pub fn new(
        operation: &str,
        scope: VidaOperationScope,
        posture: VidaOperationPosture,
        required_capabilities: Vec<VidaCapabilityScope>,
        required_claim: VidaClaimKind,
        requires_idempotency_key: bool,
        requires_apply_token: bool,
    ) -> Self {
        Self {
            operation: VidaOperation(operation.to_string()),
            scope,
            posture,
            risk_tier: risk_tier_for_posture(posture),
            allowed_client_kinds: default_allowed_client_kinds(posture),
            required_claim,
            requires_project_ref: matches!(scope, VidaOperationScope::Project),
            requires_idempotency_key,
            requires_apply_token,
            required_capabilities,
            required_consistency: consistency_for_posture(posture),
            automation_posture: automation_for_posture(posture),
            result_schema: result_schema_for_posture(posture),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaOperationInputSchema {
    pub operation: VidaOperation,
    pub schema_ref: VidaSchemaRef,
    pub fields: Vec<VidaOperationInputField>,
}

impl VidaOperationInputSchema {
    #[must_use]
    pub fn field(&self, field_id: &str) -> Option<&VidaOperationInputField> {
        self.fields.iter().find(|field| field.field_id == field_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaOperationInputField {
    pub field_id: String,
    pub payload_key: String,
    pub label: String,
    pub value_kind: VidaOperationInputValueKind,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_flag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    pub help: String,
    pub tui_control: VidaOperationTuiControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationInputValueKind {
    String,
    Path,
    Boolean,
    EnumOne,
    JsonObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationTuiControl {
    TextInput,
    PathInput,
    Checkbox,
    Select,
    JsonEditor,
}

fn input_schema_for_spec(spec: &VidaOperationSpec) -> VidaOperationInputSchema {
    let mut fields = Vec::new();
    if spec.requires_project_ref {
        fields.push(project_ref_field());
    }
    fields.extend(input_fields_for_operation(&spec.operation.0));
    VidaOperationInputSchema {
        operation: spec.operation.clone(),
        schema_ref: VidaSchemaRef {
            schema_id: VidaSchemaId(format!("{}.input", spec.operation.0)),
            version: VidaSchemaVersion(1),
        },
        fields,
    }
}

fn input_fields_for_operation(operation_id: &str) -> Vec<VidaOperationInputField> {
    use operations::*;

    match operation_id {
        EVENTS_SINCE => vec![field(
            "cursor",
            "cursor",
            "Cursor",
            VidaOperationInputValueKind::String,
            false,
            Some("--cursor"),
            Some("latest"),
            "Event cursor to read from.",
            VidaOperationTuiControl::TextInput,
        )],
        SERVICE_LIFECYCLE_PLAN => vec![
            field(
                "mode",
                "mode",
                "Mode",
                VidaOperationInputValueKind::EnumOne,
                false,
                Some("--mode"),
                Some("dry_run"),
                "Lifecycle planning mode.",
                VidaOperationTuiControl::Select,
            )
            .with_enum_values(["dry_run"]),
        ],
        WIZARD_SCHEMA_GET
        | WIZARD_SESSION_START
        | WIZARD_SESSION_GET
        | WIZARD_SESSION_UPDATE_INPUT
        | WIZARD_SESSION_VALIDATE
        | WIZARD_SESSION_DIFF
        | WIZARD_SESSION_APPLY => {
            let mut fields = vec![
                field(
                    "wizard_kind",
                    "wizard_kind",
                    "Wizard kind",
                    VidaOperationInputValueKind::EnumOne,
                    false,
                    Some("--kind"),
                    Some("project_init"),
                    "Wizard schema kind.",
                    VidaOperationTuiControl::Select,
                )
                .with_enum_values(["project_init"]),
            ];
            if matches!(
                operation_id,
                WIZARD_SESSION_START | WIZARD_SESSION_VALIDATE | WIZARD_SESSION_DIFF
            ) {
                fields.push(field(
                    "dry_run",
                    "dry_run",
                    "Dry run",
                    VidaOperationInputValueKind::Boolean,
                    false,
                    Some("--dry-run"),
                    Some("true"),
                    "Plan or validate without applying project changes.",
                    VidaOperationTuiControl::Checkbox,
                ));
            }
            fields
        }
        JOBS_GET => vec![field(
            "job_id",
            "job_id",
            "Job id",
            VidaOperationInputValueKind::String,
            false,
            Some("--job"),
            Some("latest"),
            "Job id to inspect.",
            VidaOperationTuiControl::TextInput,
        )],
        RECEIPTS_GET | MATERIALIZATION_RECEIPTS_LIST => vec![field(
            "receipt_id",
            "receipt_id",
            "Receipt id",
            VidaOperationInputValueKind::String,
            false,
            Some("--receipt"),
            Some("latest"),
            "Receipt id to inspect.",
            VidaOperationTuiControl::TextInput,
        )],
        _ => Vec::new(),
    }
}

fn project_ref_field() -> VidaOperationInputField {
    field(
        "project",
        "project",
        "Project",
        VidaOperationInputValueKind::String,
        true,
        Some("--project"),
        Some("vida-stack"),
        "Project id or registry entry to resolve.",
        VidaOperationTuiControl::TextInput,
    )
}

fn field(
    field_id: &str,
    payload_key: &str,
    label: &str,
    value_kind: VidaOperationInputValueKind,
    required: bool,
    cli_flag: Option<&str>,
    default_value: Option<&str>,
    help: &str,
    tui_control: VidaOperationTuiControl,
) -> VidaOperationInputField {
    VidaOperationInputField {
        field_id: field_id.to_string(),
        payload_key: payload_key.to_string(),
        label: label.to_string(),
        value_kind,
        required,
        cli_flag: cli_flag.map(str::to_string),
        default_value: default_value.map(str::to_string),
        enum_values: Vec::new(),
        help: help.to_string(),
        tui_control,
    }
}

impl VidaOperationInputField {
    fn with_enum_values<const N: usize>(mut self, values: [&str; N]) -> Self {
        self.enum_values = values.into_iter().map(str::to_string).collect();
        self
    }
}

fn default_allowed_client_kinds(posture: VidaOperationPosture) -> Vec<VidaClientKind> {
    match posture {
        VidaOperationPosture::ReadOnly | VidaOperationPosture::PlanOnly => vec![
            VidaClientKind::Cli,
            VidaClientKind::Tui,
            VidaClientKind::Service,
            VidaClientKind::Dashboard,
            VidaClientKind::HostAgent,
        ],
        VidaOperationPosture::Apply | VidaOperationPosture::Admin => {
            vec![VidaClientKind::Service, VidaClientKind::HostAgent]
        }
    }
}

fn risk_tier_for_posture(posture: VidaOperationPosture) -> VidaRiskTier {
    match posture {
        VidaOperationPosture::ReadOnly => VidaRiskTier::Low,
        VidaOperationPosture::PlanOnly => VidaRiskTier::Medium,
        VidaOperationPosture::Apply | VidaOperationPosture::Admin => VidaRiskTier::High,
    }
}

fn consistency_for_posture(posture: VidaOperationPosture) -> VidaConsistencyRequirement {
    match posture {
        VidaOperationPosture::ReadOnly => VidaConsistencyRequirement::Eventual,
        VidaOperationPosture::PlanOnly => VidaConsistencyRequirement::Snapshot("plan".to_string()),
        VidaOperationPosture::Apply | VidaOperationPosture::Admin => {
            VidaConsistencyRequirement::Strong
        }
    }
}

fn automation_for_posture(posture: VidaOperationPosture) -> VidaAutomationPosture {
    match posture {
        VidaOperationPosture::ReadOnly | VidaOperationPosture::PlanOnly => {
            VidaAutomationPosture::HumanOrAutomation
        }
        VidaOperationPosture::Apply | VidaOperationPosture::Admin => VidaAutomationPosture::Human,
    }
}

fn result_schema_for_posture(posture: VidaOperationPosture) -> VidaSchemaRef {
    let schema_id = match posture {
        VidaOperationPosture::ReadOnly => "vida.command_response",
        VidaOperationPosture::PlanOnly => "vida.plan",
        VidaOperationPosture::Apply | VidaOperationPosture::Admin => "vida.receipt",
    };
    VidaSchemaRef {
        schema_id: VidaSchemaId(schema_id.to_string()),
        version: VidaSchemaVersion(1),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationScope {
    Service,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaOperationPosture {
    ReadOnly,
    PlanOnly,
    Apply,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaRiskTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaAutomationPosture {
    Human,
    Automation,
    HumanOrAutomation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    MaterializationRead,
    ConfigPlan,
    ConfigApply,
    MaterializationPlan,
    MaterializationApply,
    OrchestrationControlPlaneRead,
    ServiceInstallPlan,
    ServiceInstallApply,
    ServiceAdmin,
    DiagnosticDetail,
    TaskApply,
    RunAdvance,
    CompletionRecord,
    PacketDispatch,
    ClaimWrite,
    ProjectionRebuild,
    RepairApply,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaRequestId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaProjectId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaOperation(pub String);

impl<'de> Deserialize<'de> for VidaOperation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match resolve_operation_alias(&value) {
            Ok(VidaOperationAliasResolution::Canonical { operation })
            | Ok(VidaOperationAliasResolution::Alias { operation, .. }) => Ok(operation),
            Err(problem) => Err(serde::de::Error::custom(problem.message)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaIdempotencyKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaApplyToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaEventCursor(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaJobRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaPlanRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaCommandRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaEventRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaStreamRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaAggregateRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaProjectionRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaEffectRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaArtifactRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FlowStepRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaSchemaId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaSchemaVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaStreamVersion(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaTimestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct VidaReceiptId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WizardSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VidaProjectRef {
    ProjectId { project_id: VidaProjectId },
    RegistryEntry { registry_entry_id: String },
    RootPath { root_path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaClientKind {
    Cli,
    Tui,
    Service,
    Dashboard,
    HostAgent,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaClaimKind {
    Observe,
    SharedRead,
    ExclusiveWrite,
    Dispatch,
    Proof,
    Admin,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct VidaCommandEnvelope {
    pub schema_version: String,
    pub protocol_version: String,
    pub operation: VidaOperation,
    pub session_id: VidaSessionId,
    pub request_id: VidaRequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<VidaCommandRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<VidaCommandRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_stream_version: Option<VidaStreamVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency: Option<VidaConsistencyRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<VidaTimestamp>,
    pub client_kind: VidaClientKind,
    pub project_ref: Option<VidaProjectRef>,
    pub claim_kind: Option<VidaClaimKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub trusted_owned_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub trusted_owned_write_scopes: Vec<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<VidaIdempotencyKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_token: Option<VidaApplyToken>,
}

impl VidaCommandEnvelope {
    pub fn canonicalize_operation_alias(&mut self) -> Result<(), VidaOperationAliasProblem> {
        match resolve_operation_alias(&self.operation.0)? {
            VidaOperationAliasResolution::Canonical { operation } => {
                self.operation = operation;
                Ok(())
            }
            VidaOperationAliasResolution::Alias {
                operation, receipt, ..
            } => {
                self.operation = operation;
                self.correlation = Some(correlation_with_alias_receipt(
                    self.correlation.take(),
                    receipt,
                ));
                Ok(())
            }
        }
    }
}

fn correlation_with_alias_receipt(
    correlation: Option<serde_json::Value>,
    receipt: VidaLegacyOperationAlias,
) -> serde_json::Value {
    let receipt = serde_json::to_value(receipt)
        .expect("legacy operation alias receipt should serialize to JSON");
    match correlation {
        Some(serde_json::Value::Object(mut object)) => {
            object.insert("operation_alias_receipt".to_string(), receipt);
            serde_json::Value::Object(object)
        }
        Some(value) => serde_json::json!({
            "operation_alias_receipt": receipt,
            "original_correlation": value
        }),
        None => serde_json::json!({
            "operation_alias_receipt": receipt
        }),
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize)]
struct VidaCommandEnvelopeWire {
    schema_version: String,
    protocol_version: String,
    operation: String,
    session_id: VidaSessionId,
    request_id: VidaRequestId,
    #[serde(default)]
    command_id: Option<VidaCommandRef>,
    #[serde(default)]
    causation_id: Option<VidaCommandRef>,
    #[serde(default)]
    expected_stream_version: Option<VidaStreamVersion>,
    #[serde(default)]
    consistency: Option<VidaConsistencyRequirement>,
    #[serde(default)]
    deadline: Option<VidaTimestamp>,
    client_kind: VidaClientKind,
    project_ref: Option<VidaProjectRef>,
    claim_kind: Option<VidaClaimKind>,
    #[serde(default)]
    payload: serde_json::Value,
    #[serde(default)]
    correlation: Option<serde_json::Value>,
    #[serde(default)]
    idempotency_key: Option<VidaIdempotencyKey>,
    #[serde(default)]
    apply_token: Option<VidaApplyToken>,
}

impl<'de> Deserialize<'de> for VidaCommandEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VidaCommandEnvelopeWire::deserialize(deserializer)?;
        let (operation, correlation) = match resolve_operation_alias(&wire.operation) {
            Ok(VidaOperationAliasResolution::Canonical { operation }) => {
                (operation, wire.correlation)
            }
            Ok(VidaOperationAliasResolution::Alias {
                operation, receipt, ..
            }) => (
                operation,
                Some(correlation_with_alias_receipt(wire.correlation, receipt)),
            ),
            Err(problem) => return Err(serde::de::Error::custom(problem.message)),
        };

        Ok(Self {
            schema_version: wire.schema_version,
            protocol_version: wire.protocol_version,
            operation,
            session_id: wire.session_id,
            request_id: wire.request_id,
            command_id: wire.command_id,
            causation_id: wire.causation_id,
            expected_stream_version: wire.expected_stream_version,
            consistency: wire.consistency,
            deadline: wire.deadline,
            client_kind: wire.client_kind,
            project_ref: wire.project_ref,
            claim_kind: wire.claim_kind,
            trusted_owned_path: None,
            trusted_owned_write_scopes: Vec::new(),
            payload: wire.payload,
            correlation,
            idempotency_key: wire.idempotency_key,
            apply_token: wire.apply_token,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VidaContractParseError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for VidaContractParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for VidaContractParseError {}

impl VidaContractParseError {
    fn from_path_error(error: serde_path_to_error::Error<serde_json::Error>) -> Self {
        Self {
            path: error.path().to_string(),
            message: error.inner().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VidaContractValidationError {
    pub path: String,
    pub blocker_code: String,
    pub message: String,
}

impl std::fmt::Display for VidaContractValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.path, self.message, self.blocker_code
        )
    }
}

impl std::error::Error for VidaContractValidationError {}

impl VidaContractValidationError {
    fn new(
        path: impl Into<String>,
        blocker_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            blocker_code: blocker_code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaExternalPayloadKind {
    CommandEnvelope,
    DomainEventEnvelope,
    CompletionOutcome,
}

impl VidaExternalPayloadKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CommandEnvelope => "command_envelope",
            Self::DomainEventEnvelope => "domain_event_envelope",
            Self::CompletionOutcome => "completion_outcome",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaExternalPayloadValidationStage {
    JsonParse,
    Schema,
    Typed,
    Domain,
}

impl VidaExternalPayloadValidationStage {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JsonParse => "json_parse",
            Self::Schema => "schema",
            Self::Typed => "typed",
            Self::Domain => "domain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaExternalPayloadValidationError {
    pub payload_kind: VidaExternalPayloadKind,
    pub stage: VidaExternalPayloadValidationStage,
    pub path: String,
    pub blocker_code: String,
    pub message: String,
    pub schema_ref: VidaSchemaRef,
}

impl std::fmt::Display for VidaExternalPayloadValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {} validation failed at {}: {} ({})",
            self.payload_kind.as_str(),
            self.stage.as_str(),
            self.path,
            self.message,
            self.blocker_code
        )
    }
}

impl std::error::Error for VidaExternalPayloadValidationError {}

impl VidaExternalPayloadValidationError {
    fn new(
        payload_kind: VidaExternalPayloadKind,
        stage: VidaExternalPayloadValidationStage,
        path: impl Into<String>,
        blocker_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            payload_kind,
            stage,
            path: path.into(),
            blocker_code: blocker_code.into(),
            message: message.into(),
            schema_ref: external_payload_schema_ref(payload_kind),
        }
    }

    fn from_contract(
        payload_kind: VidaExternalPayloadKind,
        stage: VidaExternalPayloadValidationStage,
        error: VidaContractValidationError,
    ) -> Self {
        Self::new(
            payload_kind,
            stage,
            error.path,
            error.blocker_code,
            error.message,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VidaExternalPayload {
    CommandEnvelope(VidaCommandEnvelope),
    DomainEventEnvelope(VidaDomainEventEnvelope),
    CompletionOutcome(CompletionOutcome),
}

pub fn parse_command_envelope_json(
    input: &[u8],
) -> Result<VidaCommandEnvelope, VidaContractParseError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    serde_path_to_error::deserialize(&mut deserializer)
        .map_err(VidaContractParseError::from_path_error)
}

pub fn command_envelope_schema_json() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(VidaCommandEnvelope))
        .expect("VidaCommandEnvelope schema should serialize")
}

#[must_use]
pub fn external_payload_schema_ref(kind: VidaExternalPayloadKind) -> VidaSchemaRef {
    let (schema_id, version) = match kind {
        VidaExternalPayloadKind::CommandEnvelope => ("vida.command_envelope", 1),
        VidaExternalPayloadKind::DomainEventEnvelope => ("vida.domain_event", 2),
        VidaExternalPayloadKind::CompletionOutcome => ("vida.completion_outcome", 1),
    };
    VidaSchemaRef {
        schema_id: VidaSchemaId(schema_id.to_string()),
        version: VidaSchemaVersion(version),
    }
}

#[must_use]
pub fn external_payload_schema_json(kind: VidaExternalPayloadKind) -> serde_json::Value {
    match kind {
        VidaExternalPayloadKind::CommandEnvelope => command_envelope_schema_json(),
        VidaExternalPayloadKind::DomainEventEnvelope => {
            serde_json::to_value(schemars::schema_for!(VidaDomainEventEnvelope))
                .expect("VidaDomainEventEnvelope schema should serialize")
        }
        VidaExternalPayloadKind::CompletionOutcome => completion_outcome_schema_json(),
    }
}

pub fn validate_external_payload_schema_value(
    kind: VidaExternalPayloadKind,
    value: &serde_json::Value,
) -> Result<(), VidaExternalPayloadValidationError> {
    let schema = external_payload_schema_json(kind);
    let validator = jsonschema::validator_for(&schema).map_err(|error| {
        VidaExternalPayloadValidationError::new(
            kind,
            VidaExternalPayloadValidationStage::Schema,
            "$",
            "external_payload_schema_compile_failed",
            error.to_string(),
        )
    })?;
    if let Some(error) = validator.iter_errors(value).next() {
        return Err(VidaExternalPayloadValidationError::new(
            kind,
            VidaExternalPayloadValidationStage::Schema,
            jsonschema_instance_path(&error),
            "external_payload_schema_invalid",
            error.to_string(),
        ));
    }
    Ok(())
}

fn jsonschema_instance_path(error: &jsonschema::ValidationError<'_>) -> String {
    let pointer = error.instance_path().to_string();
    if pointer.is_empty() {
        "$".to_string()
    } else {
        format!("${pointer}")
    }
}

pub fn validate_command_envelope_domain(
    envelope: &VidaCommandEnvelope,
) -> Result<(), VidaContractValidationError> {
    if envelope.schema_version != VIDA_CONTRACTS_SCHEMA_VERSION {
        return Err(VidaContractValidationError::new(
            "$.schema_version",
            "command_envelope_schema_version_mismatch",
            format!("command schema version must be `{VIDA_CONTRACTS_SCHEMA_VERSION}`"),
        ));
    }
    if envelope.protocol_version != VIDA_COMMAND_PROTOCOL_VERSION {
        return Err(VidaContractValidationError::new(
            "$.protocol_version",
            "command_envelope_protocol_version_mismatch",
            format!("command protocol version must be `{VIDA_COMMAND_PROTOCOL_VERSION}`"),
        ));
    }
    if operation_spec(&envelope.operation.0).is_none() {
        return Err(VidaContractValidationError::new(
            "$.operation",
            "command_envelope_operation_unknown",
            format!("operation `{}` is not registered", envelope.operation.0),
        ));
    }
    Ok(())
}

pub fn parse_external_payload_json(
    kind: VidaExternalPayloadKind,
    input: &[u8],
    registry: &VidaSchemaRegistrySnapshot,
) -> Result<VidaExternalPayload, VidaExternalPayloadValidationError> {
    let value: serde_json::Value = serde_json::from_slice(input).map_err(|error| {
        VidaExternalPayloadValidationError::new(
            kind,
            VidaExternalPayloadValidationStage::JsonParse,
            "$",
            "external_payload_json_parse_failed",
            error.to_string(),
        )
    })?;
    validate_external_payload_schema_value(kind, &value)?;
    match kind {
        VidaExternalPayloadKind::CommandEnvelope => {
            let mut deserializer = serde_json::Deserializer::from_slice(input);
            let envelope: VidaCommandEnvelope = serde_path_to_error::deserialize(&mut deserializer)
                .map_err(|error| {
                    VidaExternalPayloadValidationError::new(
                        kind,
                        VidaExternalPayloadValidationStage::Typed,
                        error.path().to_string(),
                        "external_payload_typed_decode_failed",
                        error.inner().to_string(),
                    )
                })?;
            deserializer.end().map_err(|error| {
                VidaExternalPayloadValidationError::new(
                    kind,
                    VidaExternalPayloadValidationStage::Typed,
                    "$",
                    "external_payload_typed_decode_failed",
                    error.to_string(),
                )
            })?;
            validate_command_envelope_domain(&envelope).map_err(|error| {
                VidaExternalPayloadValidationError::from_contract(
                    kind,
                    VidaExternalPayloadValidationStage::Domain,
                    error,
                )
            })?;
            Ok(VidaExternalPayload::CommandEnvelope(envelope))
        }
        VidaExternalPayloadKind::DomainEventEnvelope => {
            let mut deserializer = serde_json::Deserializer::from_slice(input);
            let event: VidaDomainEventEnvelope =
                serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
                    VidaExternalPayloadValidationError::new(
                        kind,
                        VidaExternalPayloadValidationStage::Typed,
                        error.path().to_string(),
                        "external_payload_typed_decode_failed",
                        error.inner().to_string(),
                    )
                })?;
            deserializer.end().map_err(|error| {
                VidaExternalPayloadValidationError::new(
                    kind,
                    VidaExternalPayloadValidationStage::Typed,
                    "$",
                    "external_payload_typed_decode_failed",
                    error.to_string(),
                )
            })?;
            validate_domain_event(&event, registry).map_err(|error| {
                VidaExternalPayloadValidationError::from_contract(
                    kind,
                    VidaExternalPayloadValidationStage::Domain,
                    error,
                )
            })?;
            Ok(VidaExternalPayload::DomainEventEnvelope(event))
        }
        VidaExternalPayloadKind::CompletionOutcome => {
            let outcome = parse_completion_outcome_json(input).map_err(|error| {
                VidaExternalPayloadValidationError::from_contract(
                    kind,
                    VidaExternalPayloadValidationStage::Typed,
                    error,
                )
            })?;
            Ok(VidaExternalPayload::CompletionOutcome(outcome))
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, typed_builder::TypedBuilder,
)]
pub struct CompletionBlocker {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    pub scope: Option<String>,
    #[serde(default)]
    #[builder(default)]
    pub evidence_refs: Vec<VidaArtifactRef>,
    #[serde(default)]
    #[builder(default)]
    pub next_actions: Vec<String>,
}

impl CompletionBlocker {
    fn validate_at(&self, path: &str) -> Result<(), VidaContractValidationError> {
        if self.code.trim().is_empty() {
            return Err(VidaContractValidationError::new(
                format!("{path}.code"),
                "completion_blocker_code_empty",
                "completion blocker code must be non-empty",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompletionFailureCode {
    ContractViolation,
    ExecutionFailed,
    Timeout,
    Cancelled,
    Unknown,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompletionOutcome {
    Passed {
        #[serde(default)]
        evidence_refs: Vec<VidaArtifactRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reported_next_step: Option<FlowStepRef>,
    },
    Blocked {
        blockers: Vec<CompletionBlocker>,
        rework_target: FlowStepRef,
        #[serde(default)]
        evidence_refs: Vec<VidaArtifactRef>,
    },
    Failed {
        code: CompletionFailureCode,
        retryable: bool,
        #[serde(default)]
        evidence_refs: Vec<VidaArtifactRef>,
    },
}

impl CompletionOutcome {
    pub fn passed(
        evidence_refs: Vec<VidaArtifactRef>,
        reported_next_step: Option<FlowStepRef>,
    ) -> Self {
        Self::Passed {
            evidence_refs,
            reported_next_step,
        }
    }

    pub fn blocked(
        blockers: Vec<CompletionBlocker>,
        rework_target: FlowStepRef,
        evidence_refs: Vec<VidaArtifactRef>,
    ) -> Result<Self, VidaContractValidationError> {
        let outcome = Self::Blocked {
            blockers,
            rework_target,
            evidence_refs,
        };
        outcome.validate_contract()?;
        Ok(outcome)
    }

    pub fn failed(
        code: CompletionFailureCode,
        retryable: bool,
        evidence_refs: Vec<VidaArtifactRef>,
    ) -> Self {
        Self::Failed {
            code,
            retryable,
            evidence_refs,
        }
    }

    pub fn validate_contract(&self) -> Result<(), VidaContractValidationError> {
        match self {
            Self::Passed { .. } => Ok(()),
            Self::Blocked {
                blockers,
                rework_target,
                ..
            } => {
                if blockers.is_empty() {
                    return Err(VidaContractValidationError::new(
                        "$.blockers",
                        "completion_blocked_requires_blockers",
                        "blocked completion outcome requires at least one blocker",
                    ));
                }
                if rework_target.0.trim().is_empty() {
                    return Err(VidaContractValidationError::new(
                        "$.rework_target",
                        "completion_rework_target_empty",
                        "blocked completion outcome requires a rework target",
                    ));
                }
                for (index, blocker) in blockers.iter().enumerate() {
                    blocker.validate_at(&format!("$.blockers[{index}]"))?;
                }
                Ok(())
            }
            Self::Failed { .. } => Ok(()),
        }
    }
}

pub fn parse_completion_outcome_json(
    input: &[u8],
) -> Result<CompletionOutcome, VidaContractValidationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let outcome: CompletionOutcome =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
            VidaContractValidationError::new(
                error.path().to_string(),
                "completion_outcome_deserialize_failed",
                error.inner().to_string(),
            )
        })?;
    deserializer.end().map_err(|error| {
        VidaContractValidationError::new(
            ".",
            "completion_outcome_deserialize_failed",
            error.to_string(),
        )
    })?;
    outcome.validate_contract()?;
    Ok(outcome)
}

pub fn completion_outcome_schema_json() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(CompletionOutcome))
        .expect("CompletionOutcome schema should serialize")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaConsistencyRequirement {
    Strong,
    Eventual,
    Snapshot(String),
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaDomainEventEnvelope {
    pub schema_id: VidaSchemaId,
    pub event_version: VidaSchemaVersion,
    pub event_id: VidaEventRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<VidaCommandRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<VidaCommandRef>,
    pub stream_id: VidaStreamRef,
    pub stream_version: VidaStreamVersion,
    pub aggregate_id: VidaAggregateRef,
    pub occurred_at: VidaTimestamp,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub trace: serde_json::Value,
}

impl VidaDomainEventEnvelope {
    pub fn validate_known_version(
        &self,
        registry: &VidaSchemaRegistrySnapshot,
    ) -> Result<(), VidaContractValidationError> {
        let Some(entry) = registry.event_schema(&self.schema_id) else {
            return Err(VidaContractValidationError::new(
                "$.schema_id",
                "event_schema_id_unknown",
                format!("event schema id `{}` is not registered", self.schema_id.0),
            ));
        };
        if !entry.versions.contains(&self.event_version) {
            return Err(VidaContractValidationError::new(
                "$.event_version",
                "event_schema_version_unknown",
                format!(
                    "event schema `{}` does not register version {}",
                    self.schema_id.0, self.event_version.0
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaPlan {
    pub plan_id: VidaPlanRef,
    pub command_id: VidaCommandRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_stream_version: Option<VidaStreamVersion>,
    #[serde(default)]
    pub steps: Vec<FlowStepRef>,
    #[serde(default)]
    pub effects: Vec<VidaEffectIntent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaApplyRequest {
    pub request_id: VidaRequestId,
    pub command: VidaCommandEnvelope,
    pub plan: VidaPlan,
    pub idempotency_key: VidaIdempotencyKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaEffectIntent {
    pub effect_id: VidaEffectRef,
    pub operation: VidaOperation,
    pub command_id: VidaCommandRef,
    pub stream_id: VidaStreamRef,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaProjectionCheckpoint {
    pub projection_id: VidaProjectionRef,
    pub stream_id: VidaStreamRef,
    pub event_cursor: VidaEventCursor,
    pub stream_version: VidaStreamVersion,
    pub updated_at: VidaTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionDriftClass {
    #[serde(rename = "june_2026_pass_result_blocked_run")]
    June2026PassResultBlockedRun,
    ProjectionFailureRecorded,
    SafeProjectionLag,
    PassResultLegacyContradiction,
    UnrepairableProjectionFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionDriftFinding {
    pub drift_class: ProjectionDriftClass,
    pub blocker_code: String,
    pub projection_id: String,
    pub source_event_cursor: Option<VidaEventCursor>,
    pub failure_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionRepairPlan {
    pub plan_id: String,
    pub drift_class: ProjectionDriftClass,
    pub state_mutation_allowed: bool,
    pub required_existing_event_cursors: Vec<VidaEventCursor>,
    pub actions: Vec<String>,
    #[serde(default)]
    pub auto_repair_allowed: bool,
    #[serde(default)]
    pub canonical_passed_evidence_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionRepairReceipt {
    pub plan_id: String,
    pub applied: bool,
    pub idempotency_key: String,
    pub event_backing_cursors: Vec<VidaEventCursor>,
    #[serde(default)]
    pub applied_actions: Vec<String>,
    #[serde(default)]
    pub before_health_hash: String,
    #[serde(default)]
    pub after_health_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VidaReceipt {
    pub receipt_id: VidaReceiptId,
    pub command_id: VidaCommandRef,
    #[serde(default)]
    pub events: Vec<VidaEventRef>,
    #[serde(default)]
    pub effects: Vec<VidaEffectRef>,
    #[serde(default)]
    pub projection_checkpoints: Vec<VidaProjectionCheckpoint>,
    pub state_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaSchemaRef {
    pub schema_id: VidaSchemaId,
    pub version: VidaSchemaVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaSchemaRegistryEntry {
    pub schema_id: VidaSchemaId,
    pub kind: VidaSchemaKind,
    pub versions: Vec<VidaSchemaVersion>,
    pub latest_version: VidaSchemaVersion,
    pub artifact_ref: VidaArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VidaSchemaKind {
    Command,
    Event,
    Plan,
    Receipt,
    Effect,
    Projection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VidaSchemaRegistrySnapshot {
    pub schema_version: String,
    pub entries: Vec<VidaSchemaRegistryEntry>,
}

impl VidaSchemaRegistrySnapshot {
    pub fn event_schema(&self, schema_id: &VidaSchemaId) -> Option<&VidaSchemaRegistryEntry> {
        self.entries
            .iter()
            .find(|entry| entry.kind == VidaSchemaKind::Event && entry.schema_id == *schema_id)
    }
}

pub fn vida_runtime_schema_registry_snapshot() -> VidaSchemaRegistrySnapshot {
    VidaSchemaRegistrySnapshot {
        schema_version: VIDA_RUNTIME_CONTRACTS_V1_SCHEMA_VERSION.to_string(),
        entries: vec![
            VidaSchemaRegistryEntry {
                schema_id: VidaSchemaId("vida.command_envelope".to_string()),
                kind: VidaSchemaKind::Command,
                versions: vec![VidaSchemaVersion(1)],
                latest_version: VidaSchemaVersion(1),
                artifact_ref: VidaArtifactRef("schema://vida.command_envelope/v1".to_string()),
            },
            VidaSchemaRegistryEntry {
                schema_id: VidaSchemaId("vida.domain_event".to_string()),
                kind: VidaSchemaKind::Event,
                versions: vec![VidaSchemaVersion(1), VidaSchemaVersion(2)],
                latest_version: VidaSchemaVersion(2),
                artifact_ref: VidaArtifactRef("schema://vida.domain_event/v2".to_string()),
            },
            VidaSchemaRegistryEntry {
                schema_id: VidaSchemaId("vida.plan".to_string()),
                kind: VidaSchemaKind::Plan,
                versions: vec![VidaSchemaVersion(1)],
                latest_version: VidaSchemaVersion(1),
                artifact_ref: VidaArtifactRef("schema://vida.plan/v1".to_string()),
            },
            VidaSchemaRegistryEntry {
                schema_id: VidaSchemaId("vida.receipt".to_string()),
                kind: VidaSchemaKind::Receipt,
                versions: vec![VidaSchemaVersion(1)],
                latest_version: VidaSchemaVersion(1),
                artifact_ref: VidaArtifactRef("schema://vida.receipt/v1".to_string()),
            },
        ],
    }
}

pub fn runtime_schema_registry_snapshot_json() -> serde_json::Value {
    serde_json::to_value(vida_runtime_schema_registry_snapshot())
        .expect("runtime schema registry snapshot should serialize")
}

pub fn runtime_envelope_schema_bundle_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": VIDA_RUNTIME_CONTRACTS_V1_SCHEMA_VERSION,
        "command_envelope": command_envelope_schema_json(),
        "domain_event_envelope": schemars::schema_for!(VidaDomainEventEnvelope),
        "plan": schemars::schema_for!(VidaPlan),
        "apply_request": schemars::schema_for!(VidaApplyRequest),
        "effect_intent": schemars::schema_for!(VidaEffectIntent),
        "projection_checkpoint": schemars::schema_for!(VidaProjectionCheckpoint),
        "receipt": schemars::schema_for!(VidaReceipt),
        "registry_snapshot": runtime_schema_registry_snapshot_json()
    })
}

pub fn validate_domain_event(
    event: &VidaDomainEventEnvelope,
    registry: &VidaSchemaRegistrySnapshot,
) -> Result<(), VidaContractValidationError> {
    upcast_domain_event_to_latest(event, registry)?;
    Ok(())
}

pub fn upcast_domain_event_to_latest(
    event: &VidaDomainEventEnvelope,
    registry: &VidaSchemaRegistrySnapshot,
) -> Result<VidaDomainEventEnvelope, VidaContractValidationError> {
    event.validate_known_version(registry)?;
    let Some(entry) = registry.event_schema(&event.schema_id) else {
        return Err(VidaContractValidationError::new(
            "$.schema_id",
            "event_schema_id_unknown",
            format!("event schema id `{}` is not registered", event.schema_id.0),
        ));
    };
    if event.event_version == entry.latest_version {
        return Ok(event.clone());
    }
    if event.schema_id.0 == "vida.domain_event"
        && event.event_version == VidaSchemaVersion(1)
        && entry.latest_version == VidaSchemaVersion(2)
    {
        let mut upcasted = event.clone();
        upcasted.event_version = VidaSchemaVersion(2);
        return Ok(upcasted);
    }
    if event.event_version.0 > entry.latest_version.0 {
        return Err(VidaContractValidationError::new(
            "$.event_version",
            "event_schema_revision_not_latest",
            format!(
                "event schema `{}` must use latest revision {}",
                event.schema_id.0, entry.latest_version.0
            ),
        ));
    }
    Err(VidaContractValidationError::new(
        "$.event_version",
        "event_schema_upcaster_missing",
        format!(
            "event schema `{}` version {} has no upcaster to latest revision {}",
            event.schema_id.0, event.event_version.0, entry.latest_version.0
        ),
    ))
}

pub fn trace_links_are_conformant(
    command: &VidaCommandEnvelope,
    event: &VidaDomainEventEnvelope,
    receipt: &VidaReceipt,
) -> bool {
    let Some(command_id) = &command.command_id else {
        return false;
    };
    event.command_id.as_ref() == Some(command_id)
        && event.causation_id.as_ref() == command.causation_id.as_ref().or(Some(command_id))
        && receipt.command_id == *command_id
        && receipt
            .events
            .iter()
            .any(|event_id| event_id == &event.event_id)
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

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    garde::Validate,
    typed_builder::TypedBuilder,
)]
pub struct VidaReceiptRef {
    #[garde(length(min = 1))]
    pub receipt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    #[garde(skip)]
    pub project_id: Option<VidaProjectId>,
    #[garde(skip)]
    pub operation: VidaOperation,
    #[garde(skip)]
    pub scope: VidaReceiptScope,
    #[garde(length(min = 1))]
    pub state_root: String,
}

impl VidaReceiptRef {
    pub fn validate_contract(&self) -> Result<(), garde::Report> {
        garde::Validate::validate(self)
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    garde::Validate,
    typed_builder::TypedBuilder,
)]
pub struct VidaReceiptSummary {
    #[garde(dive)]
    pub receipt_ref: VidaReceiptRef,
    #[garde(skip)]
    pub status: VidaResponseStatus,
    #[garde(length(min = 1))]
    pub recorded_at: String,
}

impl VidaReceiptSummary {
    pub fn validate_contract(&self) -> Result<(), garde::Report> {
        garde::Validate::validate(self)
    }
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
    fn command_envelope_parse_error_reports_field_path() {
        let malformed = br#"{
          "schema_version": "vida-contracts-v1",
          "protocol_version": "vida-command-v1",
          "operation": "vida.wizard.schema.get",
          "session_id": 42,
          "request_id": "request-01",
          "client_kind": "tui",
          "payload": {}
        }"#;

        let error =
            parse_command_envelope_json(malformed).expect_err("malformed field should fail");
        assert_eq!(error.path, "session_id");
        assert!(
            error.message.contains("invalid type"),
            "parse message should preserve serde detail: {error}"
        );
    }

    #[test]
    fn command_envelope_schema_includes_packet_boundary_properties() {
        let schema = command_envelope_schema_json();
        assert_eq!(schema["title"], "VidaCommandEnvelope");
        let properties = schema["properties"]
            .as_object()
            .expect("schema should expose object properties");
        for key in [
            "schema_version",
            "protocol_version",
            "operation",
            "session_id",
            "request_id",
            "command_id",
            "causation_id",
            "expected_stream_version",
            "consistency",
            "deadline",
            "client_kind",
            "payload",
        ] {
            assert!(
                properties.contains_key(key),
                "schema should include `{key}`, got {schema}"
            );
        }
        for key in ["trusted_owned_path", "trusted_owned_write_scopes"] {
            assert!(
                !properties.contains_key(key),
                "schema must not expose internal trusted write-scope field `{key}`"
            );
        }
    }

    #[test]
    fn command_envelope_deserialize_ignores_client_supplied_trusted_scope_evidence() {
        let envelope: VidaCommandEnvelope = serde_json::from_str(
            r#"{
              "schema_version": "vida-contracts-v1",
              "protocol_version": "vida-command-v1",
              "operation": "vida.wizard.schema.get",
              "session_id": "session-01",
              "request_id": "request-01",
              "client_kind": "tui",
              "trusted_owned_path": "vida/config/policies",
              "trusted_owned_write_scopes": ["vida/config/policies"],
              "payload": {}
            }"#,
        )
        .expect("unknown trusted fields should not reject older clients or probes");

        assert_eq!(envelope.trusted_owned_path, None);
        assert!(envelope.trusted_owned_write_scopes.is_empty());
    }

    #[test]
    fn external_payload_validation_matrix_distinguishes_failure_stages() {
        let registry = vida_runtime_schema_registry_snapshot();

        let parse_error = parse_external_payload_json(
            VidaExternalPayloadKind::CommandEnvelope,
            br#"{"schema_version":"vida-contracts-v1","#,
            &registry,
        )
        .expect_err("invalid JSON should fail at parse stage");
        assert_eq!(
            parse_error.stage,
            VidaExternalPayloadValidationStage::JsonParse
        );
        assert_eq!(
            parse_error.blocker_code,
            "external_payload_json_parse_failed"
        );

        let schema_error = parse_external_payload_json(
            VidaExternalPayloadKind::CommandEnvelope,
            br#"{
              "schema_version": "vida-contracts-v1",
              "protocol_version": "vida-command-v1",
              "operation": "vida.wizard.schema.get",
              "session_id": "session-01",
              "client_kind": "tui",
              "payload": {}
            }"#,
            &registry,
        )
        .expect_err("missing request_id should fail schema validation");
        assert_eq!(
            schema_error.stage,
            VidaExternalPayloadValidationStage::Schema
        );
        assert_eq!(schema_error.blocker_code, "external_payload_schema_invalid");

        let typed_error = parse_external_payload_json(
            VidaExternalPayloadKind::DomainEventEnvelope,
            br#"{
              "schema_id": "vida.domain_event",
              "event_version": 1,
              "event_id": "event-typed-overflow",
              "command_id": "command-typed-overflow",
              "causation_id": "command-typed-overflow",
              "stream_id": "stream-typed-overflow",
              "stream_version": 18446744073709551616,
              "aggregate_id": "aggregate-typed-overflow",
              "occurred_at": "2026-06-24T10:00:00Z",
              "payload": {},
              "trace": {}
            }"#,
            &registry,
        )
        .expect_err("integer overflow should fail typed decode");
        assert_eq!(typed_error.stage, VidaExternalPayloadValidationStage::Typed);
        assert_eq!(
            typed_error.blocker_code,
            "external_payload_typed_decode_failed"
        );

        let domain_error = parse_external_payload_json(
            VidaExternalPayloadKind::CommandEnvelope,
            br#"{
              "schema_version": "wrong-contracts-v1",
              "protocol_version": "vida-command-v1",
              "operation": "vida.wizard.schema.get",
              "session_id": "session-01",
              "request_id": "request-01",
              "client_kind": "tui",
              "payload": {}
            }"#,
            &registry,
        )
        .expect_err("schema version mismatch should fail domain validation");
        assert_eq!(
            domain_error.stage,
            VidaExternalPayloadValidationStage::Domain
        );
        assert_eq!(
            domain_error.blocker_code,
            "command_envelope_schema_version_mismatch"
        );
    }

    #[test]
    fn external_payload_validation_accepts_valid_command_envelope() {
        let registry = vida_runtime_schema_registry_snapshot();
        let payload = parse_external_payload_json(
            VidaExternalPayloadKind::CommandEnvelope,
            include_bytes!("../fixtures/command_envelope.json"),
            &registry,
        )
        .expect("fixture should pass external validation");

        let VidaExternalPayload::CommandEnvelope(envelope) = payload else {
            panic!("expected command envelope payload");
        };
        assert_eq!(envelope.schema_version, VIDA_CONTRACTS_SCHEMA_VERSION);
        assert_eq!(envelope.protocol_version, VIDA_COMMAND_PROTOCOL_VERSION);
        assert_eq!(envelope.operation.0, operations::WIZARD_SCHEMA_GET);
    }

    #[test]
    fn runtime_schema_registry_snapshot_matches_fixture() {
        let fixture: VidaSchemaRegistrySnapshot = serde_json::from_str(include_str!(
            "../fixtures/runtime_schema_registry_snapshot.json"
        ))
        .expect("registry snapshot fixture should deserialize");
        assert_eq!(fixture, vida_runtime_schema_registry_snapshot());

        let bundle = runtime_envelope_schema_bundle_json();
        for key in [
            "command_envelope",
            "domain_event_envelope",
            "plan",
            "apply_request",
            "receipt",
            "registry_snapshot",
        ] {
            assert!(bundle.get(key).is_some(), "bundle should include `{key}`");
        }
    }

    #[test]
    fn event_replay_accepts_v1_and_v2_schema_versions() {
        let registry = vida_runtime_schema_registry_snapshot();
        let v1_event: VidaDomainEventEnvelope =
            serde_json::from_str(include_str!("../fixtures/domain_event_v1.json"))
                .expect("event fixture should deserialize");
        let v2_event: VidaDomainEventEnvelope =
            serde_json::from_str(include_str!("../fixtures/domain_event_v2.json"))
                .expect("v2 event fixture should deserialize");

        validate_domain_event(&v1_event, &registry).expect("v1 event should be registered");
        validate_domain_event(&v2_event, &registry).expect("v2 event should be registered");
        assert_eq!(v1_event.event_version, VidaSchemaVersion(1));
        assert_eq!(v2_event.event_version, VidaSchemaVersion(2));
    }

    #[test]
    fn domain_event_upcaster_is_pure_and_targets_latest_version() {
        let registry = vida_runtime_schema_registry_snapshot();
        let event: VidaDomainEventEnvelope =
            serde_json::from_str(include_str!("../fixtures/domain_event_v1.json"))
                .expect("event fixture should deserialize");

        let first = upcast_domain_event_to_latest(&event, &registry)
            .expect("v1 event should upcast to latest");
        let second = upcast_domain_event_to_latest(&event, &registry)
            .expect("v1 event should upcast deterministically");

        assert_eq!(event.event_version, VidaSchemaVersion(1));
        assert_eq!(first, second);
        assert_eq!(first.event_version, VidaSchemaVersion(2));
        assert_eq!(first.event_id, event.event_id);
        assert_eq!(first.stream_id, event.stream_id);
        assert_eq!(first.payload, event.payload);
    }

    #[test]
    fn event_replay_rejects_unknown_schema_revision() {
        let registry = vida_runtime_schema_registry_snapshot();
        let mut future_event: VidaDomainEventEnvelope =
            serde_json::from_str(include_str!("../fixtures/domain_event_v1.json"))
                .expect("event fixture should deserialize");
        future_event.event_version = VidaSchemaVersion(99);

        let error = future_event
            .validate_known_version(&registry)
            .expect_err("unknown event schema revision must fail closed");
        assert_eq!(error.path, "$.event_version");
        assert_eq!(error.blocker_code, "event_schema_version_unknown");
    }

    #[test]
    fn trace_link_conformance_requires_command_event_receipt_chain() {
        let command: VidaCommandEnvelope = serde_json::from_value(serde_json::json!({
            "schema_version": "vida-contracts-v1",
            "protocol_version": "vida-command-v1",
            "operation": "vida.wizard.schema.get",
            "session_id": "session-ldr-011",
            "request_id": "request-ldr-011",
            "command_id": "command-ldr-011-001",
            "causation_id": "command-ldr-011-001",
            "expected_stream_version": 1,
            "consistency": "strong",
            "deadline": "2026-06-22T08:00:00Z",
            "client_kind": "tui",
            "payload": {"task_id": "ldr-011"}
        }))
        .expect("command should deserialize");
        let event: VidaDomainEventEnvelope =
            serde_json::from_str(include_str!("../fixtures/domain_event_v1.json"))
                .expect("event fixture should deserialize");
        let receipt = VidaReceipt {
            receipt_id: VidaReceiptId("receipt-ldr-011-001".to_string()),
            command_id: VidaCommandRef("command-ldr-011-001".to_string()),
            events: vec![VidaEventRef("event-ldr-011-001".to_string())],
            effects: vec![VidaEffectRef("effect-ldr-011-001".to_string())],
            projection_checkpoints: vec![VidaProjectionCheckpoint {
                projection_id: VidaProjectionRef("projection-ldr-011".to_string()),
                stream_id: VidaStreamRef("stream-ldr-011".to_string()),
                event_cursor: VidaEventCursor("cursor-v1".to_string()),
                stream_version: VidaStreamVersion(1),
                updated_at: VidaTimestamp("2026-06-22T07:42:00Z".to_string()),
            }],
            state_root: "state-root-ldr-011".to_string(),
        };

        assert!(trace_links_are_conformant(&command, &event, &receipt));

        let broken_receipt = VidaReceipt {
            events: Vec::new(),
            ..receipt
        };
        assert!(!trace_links_are_conformant(
            &command,
            &event,
            &broken_receipt
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
    fn receipt_ref_builder_preserves_public_json_shape() {
        let receipt = VidaReceiptRef::builder()
            .receipt_id("receipt-1".to_string())
            .operation(VidaOperation(operations::SERVICE_HELLO.to_string()))
            .scope(VidaReceiptScope::Service)
            .state_root("state-root-1".to_string())
            .build();

        receipt
            .validate_contract()
            .expect("builder output should validate");
        let json = serde_json::to_value(&receipt).expect("receipt should serialize");

        assert_eq!(json["receipt_id"], "receipt-1");
        assert!(json.get("project_id").is_none());
        assert_eq!(json["operation"], "vida.service.hello");
        assert_eq!(json["scope"], "service");
        assert_eq!(json["state_root"], "state-root-1");
    }

    #[test]
    fn receipt_validation_rejects_empty_boundary_fields() {
        let receipt = VidaReceiptRef::builder()
            .receipt_id(String::new())
            .operation(VidaOperation(operations::SERVICE_HELLO.to_string()))
            .scope(VidaReceiptScope::Service)
            .state_root(String::new())
            .build();

        let report = receipt
            .validate_contract()
            .expect_err("empty receipt fields should fail validation");
        let report = report.to_string();

        assert!(report.contains("receipt_id"), "{report}");
        assert!(report.contains("state_root"), "{report}");
    }

    #[test]
    fn receipt_summary_builder_validates_nested_receipt() {
        let receipt_ref = VidaReceiptRef::builder()
            .receipt_id("receipt-1".to_string())
            .operation(VidaOperation(operations::SERVICE_HELLO.to_string()))
            .scope(VidaReceiptScope::Service)
            .state_root("state-root-1".to_string())
            .build();
        let summary = VidaReceiptSummary::builder()
            .receipt_ref(receipt_ref)
            .status(VidaResponseStatus::Pass)
            .recorded_at("2026-06-21T09:30:00Z".to_string())
            .build();

        summary
            .validate_contract()
            .expect("summary builder output should validate");
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
    fn mvp_registry_contains_runtime_mutation_operations() {
        let registry = mvp_operation_registry();
        for operation_id in [
            operations::TASK_APPLY,
            operations::RUN_ADVANCE,
            operations::COMPLETION_RECORD,
            operations::PACKET_DISPATCH,
            operations::CLAIM_ACQUIRE,
            operations::PROJECTION_REBUILD,
            operations::REPAIR_APPLY,
            operations::SERVICE_LIFECYCLE_APPLY,
        ] {
            assert!(
                registry.iter().any(|spec| spec.operation.0 == operation_id),
                "registry should include `{operation_id}`"
            );
        }
    }

    #[test]
    fn operation_input_schema_defines_project_field_once_for_cli_and_tui() {
        let schema = operation_input_schema(operations::WIZARD_SCHEMA_GET)
            .expect("wizard schema operation should expose input schema");
        let project = schema
            .fields
            .iter()
            .find(|field| field.field_id == "project")
            .expect("project field");
        assert_eq!(project.label, "Project");
        assert!(project.required);
        assert_eq!(project.payload_key, "project");
        assert_eq!(project.cli_flag.as_deref(), Some("--project"));
        assert_eq!(project.tui_control, VidaOperationTuiControl::TextInput);

        let wizard_kind = schema
            .fields
            .iter()
            .find(|field| field.field_id == "wizard_kind")
            .expect("wizard kind field");
        assert_eq!(wizard_kind.cli_flag.as_deref(), Some("--kind"));
        assert_eq!(wizard_kind.default_value.as_deref(), Some("project_init"));
        assert_eq!(wizard_kind.tui_control, VidaOperationTuiControl::Select);
    }

    #[test]
    fn operation_catalog_exposes_input_schemas_for_external_clients() {
        let catalog = mvp_operation_catalog();
        let wizard = catalog
            .iter()
            .find(|entry| entry.operation.0 == operations::WIZARD_SCHEMA_GET)
            .expect("wizard schema catalog entry");
        let field_ids = wizard
            .input_schema
            .fields
            .iter()
            .map(|field| field.field_id.as_str())
            .collect::<Vec<_>>();
        assert!(field_ids.contains(&"project"));
        assert!(field_ids.contains(&"wizard_kind"));
    }

    #[test]
    fn operation_catalog_exposes_authorization_metadata_for_external_clients() {
        for (spec, catalog) in mvp_operation_registry()
            .into_iter()
            .zip(mvp_operation_catalog().into_iter())
        {
            assert_eq!(catalog.operation, spec.operation);
            assert_eq!(catalog.scope, spec.scope);
            assert_eq!(catalog.posture, spec.posture);
            assert_eq!(catalog.risk_tier, spec.risk_tier);
            assert_eq!(catalog.allowed_client_kinds, spec.allowed_client_kinds);
            assert_eq!(catalog.required_claim, spec.required_claim);
            assert_eq!(catalog.requires_project_ref, spec.requires_project_ref);
            assert_eq!(
                catalog.requires_idempotency_key,
                spec.requires_idempotency_key
            );
            assert_eq!(catalog.requires_apply_token, spec.requires_apply_token);
            assert_eq!(catalog.required_capabilities, spec.required_capabilities);
            assert_eq!(catalog.required_consistency, spec.required_consistency);
            assert_eq!(catalog.automation_posture, spec.automation_posture);
            assert_eq!(catalog.result_schema, spec.result_schema);
        }
    }

    #[test]
    fn mutation_operation_input_schemas_do_not_expose_owned_write_evidence() {
        for entry in mvp_operation_catalog()
            .into_iter()
            .filter(|entry| entry.required_claim == VidaClaimKind::ExclusiveWrite)
        {
            assert!(entry.input_schema.field("owned_path").is_none());
            assert!(entry.input_schema.field("owned_write_scopes").is_none());
        }
    }

    #[test]
    fn retained_legacy_operation_aliases_reach_canonical_operation_specs() {
        for receipt in legacy_operation_aliases() {
            let canonical =
                operation_spec(&receipt.canonical_operation.0).expect("canonical operation spec");
            let alias = operation_spec(&receipt.alias).expect("legacy alias operation spec");

            assert_eq!(alias, canonical, "alias {}", receipt.alias);
        }
    }

    #[test]
    fn legacy_operation_alias_matrix_deserializes_to_canonical_command_envelopes() {
        for receipt in legacy_operation_aliases() {
            let canonical =
                operation_spec(&receipt.canonical_operation.0).expect("canonical operation spec");
            let canonical_json = command_envelope_json(&receipt.canonical_operation.0, &canonical);
            let alias_json = command_envelope_json(&receipt.alias, &canonical);

            let canonical: VidaCommandEnvelope =
                serde_json::from_value(canonical_json).expect("canonical command envelope");
            let alias: VidaCommandEnvelope =
                serde_json::from_value(alias_json).expect("alias command envelope");

            assert_eq!(
                alias.operation, canonical.operation,
                "alias {} should canonicalize",
                receipt.alias
            );
            validate_command_envelope_domain(&alias).expect("alias envelope should validate");
            assert_alias_receipt(&alias, &receipt);
        }
    }

    fn command_envelope_json(operation: &str, spec: &VidaOperationSpec) -> serde_json::Value {
        let mut value = serde_json::json!({
            "schema_version": VIDA_CONTRACTS_SCHEMA_VERSION,
            "protocol_version": VIDA_COMMAND_PROTOCOL_VERSION,
            "operation": operation,
            "session_id": "session-1",
            "request_id": "request-1",
            "client_kind": "cli",
            "project_ref": null,
            "claim_kind": serde_json::to_value(&spec.required_claim)
                .expect("claim kind serializes"),
            "payload": {}
        });

        if spec.requires_idempotency_key {
            value["idempotency_key"] = serde_json::json!("idem-1");
        }
        if spec.requires_apply_token {
            value["apply_token"] = serde_json::json!("apply-1");
        }
        value
    }

    fn assert_alias_receipt(envelope: &VidaCommandEnvelope, expected: &VidaLegacyOperationAlias) {
        let receipt = envelope
            .correlation
            .as_ref()
            .and_then(|value| value.get("operation_alias_receipt"))
            .expect("alias receipt should be observable in command correlation");

        assert_eq!(receipt["alias"], expected.alias);
        assert_eq!(
            receipt["canonical_operation"],
            expected.canonical_operation.0
        );
        assert_eq!(receipt["receipt_code"], "legacy_operation_alias_used");
    }

    #[test]
    fn legacy_operation_alias_receipt_is_measurable_and_serializable() {
        let receipt = legacy_operation_alias_receipt("task.apply").expect("alias receipt");
        let receipt_json = serde_json::to_value(&receipt).expect("receipt serializes");

        assert_eq!(receipt_json["alias"], "task.apply");
        assert_eq!(receipt_json["canonical_operation"], operations::TASK_APPLY);
        assert_eq!(receipt_json["receipt_code"], "legacy_operation_alias_used");
        assert!(receipt_json["deprecated_since"].is_string());
    }

    #[test]
    fn ambiguous_legacy_operation_alias_fails_before_command_envelope() {
        let error = serde_json::from_value::<VidaCommandEnvelope>(serde_json::json!({
            "schema_version": VIDA_CONTRACTS_SCHEMA_VERSION,
            "protocol_version": VIDA_COMMAND_PROTOCOL_VERSION,
            "operation": "status",
            "session_id": "session-1",
            "request_id": "request-1",
            "client_kind": "cli",
            "project_ref": null,
            "claim_kind": "observe",
            "payload": {}
        }))
        .expect_err("ambiguous alias should fail while parsing the envelope");

        assert!(
            error
                .to_string()
                .contains("Legacy operation alias `status` is ambiguous"),
            "{error}"
        );
    }

    #[test]
    fn mvp_registry_mutations_require_claim_and_replay_posture() {
        for spec in mvp_operation_registry() {
            match spec.posture {
                VidaOperationPosture::ReadOnly => {
                    assert_eq!(spec.required_claim, VidaClaimKind::SharedRead);
                    assert!(!spec.requires_idempotency_key);
                    assert!(!spec.requires_apply_token);
                }
                VidaOperationPosture::PlanOnly => {
                    assert!(!spec.requires_apply_token);
                }
                VidaOperationPosture::Apply | VidaOperationPosture::Admin => {
                    assert!(
                        spec.requires_idempotency_key,
                        "{} should require idempotency",
                        spec.operation.0
                    );
                    assert!(
                        spec.requires_apply_token,
                        "{} should require apply token",
                        spec.operation.0
                    );
                    assert!(
                        !spec.required_capabilities.is_empty(),
                        "{} should declare capability",
                        spec.operation.0
                    );
                    assert!(
                        spec.allowed_client_kinds.iter().all(|kind| matches!(
                            kind,
                            VidaClientKind::Service | VidaClientKind::HostAgent
                        )),
                        "{} should restrict write clients",
                        spec.operation.0
                    );
                }
            }
        }
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

    #[test]
    fn completion_outcome_builders_validate_blocked_invariants() {
        let blocker = CompletionBlocker::builder()
            .code("missing_required_input".to_string())
            .scope("ldr-010".to_string())
            .build();
        let blocked = CompletionOutcome::blocked(
            vec![blocker],
            FlowStepRef("developer_rework".to_string()),
            vec![VidaArtifactRef("artifact://receipt".to_string())],
        )
        .expect("blocked outcome with blocker should validate");

        assert!(matches!(blocked, CompletionOutcome::Blocked { .. }));

        let error = CompletionOutcome::blocked(
            Vec::new(),
            FlowStepRef("developer_rework".to_string()),
            vec![],
        )
        .expect_err("blocked outcome without blockers must fail");
        assert_eq!(error.path, "$.blockers");
        assert_eq!(error.blocker_code, "completion_blocked_requires_blockers");
    }

    #[test]
    fn completion_outcome_rejects_empty_blocker_with_json_path() {
        let payload = br#"{
            "outcome": "blocked",
            "blockers": [{"code": ""}],
            "rework_target": "developer_rework"
        }"#;
        let error = parse_completion_outcome_json(payload)
            .expect_err("empty blocker code must fail validation");

        assert_eq!(error.path, "$.blockers[0].code");
        assert_eq!(error.blocker_code, "completion_blocker_code_empty");
    }

    #[test]
    fn completion_outcome_rejects_blocked_with_empty_blockers_after_deserialization() {
        let payload = br#"{
            "outcome": "blocked",
            "blockers": [],
            "rework_target": "developer_rework"
        }"#;
        let error = parse_completion_outcome_json(payload)
            .expect_err("blocked outcome with empty blockers must fail");

        assert_eq!(error.path, "$.blockers");
        assert_eq!(error.blocker_code, "completion_blocked_requires_blockers");
    }

    #[test]
    fn completion_outcome_passed_rejects_wrong_variant_blocker_field_with_exact_path() {
        let payload = br#"{
            "outcome": "passed",
            "blockers": [{"code": "should_not_exist"}]
        }"#;
        let error = parse_completion_outcome_json(payload)
            .expect_err("passed outcome must not accept blockers");

        assert_eq!(error.path, ".");
        assert_eq!(error.blocker_code, "completion_outcome_deserialize_failed");
        assert!(error.message.contains("unknown field"));
    }

    #[test]
    fn completion_outcome_rejects_trailing_non_whitespace_after_valid_json() {
        let cases = [
            br#"{"outcome":"passed"} trailing text"#.as_slice(),
            br#"{"outcome":"passed"}{"outcome":"failed","code":"contract_violation","retryable":false}"#.as_slice(),
        ];

        for payload in cases {
            let error = parse_completion_outcome_json(payload)
                .expect_err("completion outcome parser must reject trailing data");
            assert_eq!(error.path, ".");
            assert_eq!(error.blocker_code, "completion_outcome_deserialize_failed");
            assert!(
                error.message.contains("trailing characters"),
                "unexpected error message: {}",
                error.message
            );
        }
    }

    #[test]
    fn completion_outcome_accepts_trailing_whitespace_after_valid_json() {
        let outcome = parse_completion_outcome_json(
            br#"{"outcome":"passed"}
	 "#,
        )
        .expect("completion outcome parser should allow JSON whitespace after the value");

        assert!(matches!(outcome, CompletionOutcome::Passed { .. }));
    }

    #[test]
    fn completion_outcome_round_trips_all_variants() {
        let cases = [
            include_str!("../fixtures/completion_outcome_passed.json"),
            include_str!("../fixtures/completion_outcome_blocked.json"),
            include_str!("../fixtures/completion_outcome_failed.json"),
        ];

        for case in cases {
            let outcome = parse_completion_outcome_json(case.as_bytes())
                .expect("fixture should parse and validate");
            let encoded = serde_json::to_vec(&outcome).expect("outcome should serialize");
            let decoded = parse_completion_outcome_json(&encoded)
                .expect("serialized outcome should parse and validate");
            assert_eq!(outcome, decoded);
        }
    }

    #[test]
    fn completion_outcome_schema_expresses_tagged_alternatives() {
        let schema = completion_outcome_schema_json();
        let schema_text = serde_json::to_string(&schema).expect("schema should serialize");
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/completion_outcome.schema.json"))
                .expect("golden schema metadata should parse");

        for alternative in golden["required_alternatives"]
            .as_array()
            .expect("alternatives should be an array")
        {
            assert!(schema_text.contains(alternative.as_str().unwrap()));
        }
        for field in golden["blocked_required_fields"]
            .as_array()
            .expect("blocked fields should be an array")
        {
            assert!(schema_text.contains(field.as_str().unwrap()));
        }
        for field in golden["failed_required_fields"]
            .as_array()
            .expect("failed fields should be an array")
        {
            assert!(schema_text.contains(field.as_str().unwrap()));
        }
    }
}
