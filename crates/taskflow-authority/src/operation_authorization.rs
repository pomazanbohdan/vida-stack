use std::{
    path::{Component, Path},
    str::FromStr,
};

use cedar_policy::{Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request};
use serde_json::json;
use vida_contracts::{
    VidaCapabilityScope, VidaClaimKind, VidaClientKind, VidaOperationPosture, VidaOperationScope,
    VidaOperationSpec, VidaProjectId,
};

pub const MODULE: &str = "operation_authorization";
pub const DEFAULT_POLICY: &str =
    include_str!("../../../vida/config/policies/operation_authorization.cedar");
pub const ENTITY_SNAPSHOT: &str =
    include_str!("../../../vida/config/policies/operation_authorization_entities.snapshot.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAuthorizationInput {
    pub session_id: String,
    pub project_id: Option<VidaProjectId>,
    pub client_kind: VidaClientKind,
    pub claim_kind: VidaClaimKind,
    pub capability: VidaCapabilityScope,
    pub operation: VidaOperationSpec,
    pub resource_project_id: Option<VidaProjectId>,
    pub owned_path: Option<String>,
    pub owned_write_scopes: Vec<String>,
    pub idempotency_key_present: bool,
    pub apply_token_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAuthorizationDecision {
    pub allowed: bool,
    pub blocker_codes: Vec<String>,
    pub remediation: Vec<String>,
}

impl OperationAuthorizationDecision {
    fn allow() -> Self {
        Self {
            allowed: true,
            blocker_codes: Vec::new(),
            remediation: Vec::new(),
        }
    }

    fn deny(code: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self {
            allowed: false,
            blocker_codes: vec![code.into()],
            remediation: vec![remediation.into()],
        }
    }
}

pub fn compile_default_policy() -> Result<PolicySet, String> {
    PolicySet::from_str(DEFAULT_POLICY).map_err(|error| error.to_string())
}

pub fn load_entity_snapshot() -> Result<Entities, String> {
    Entities::from_json_str(ENTITY_SNAPSHOT, None).map_err(|error| error.to_string())
}

pub fn authorize_operation(input: &OperationAuthorizationInput) -> OperationAuthorizationDecision {
    if !input
        .operation
        .allowed_client_kinds
        .contains(&input.client_kind)
    {
        return OperationAuthorizationDecision::deny(
            "operation_client_kind_denied",
            "Use a client kind admitted by the operation registry.",
        );
    }
    if !input
        .operation
        .required_capabilities
        .contains(&input.capability)
    {
        return OperationAuthorizationDecision::deny(
            "operation_capability_denied",
            "Request a capability admitted by the operation registry.",
        );
    }
    if input.operation.requires_project_ref && input.project_id.is_none() {
        return OperationAuthorizationDecision::deny(
            "operation_project_ref_required",
            "Bind the request to a project before authorization.",
        );
    }
    if input.operation.requires_idempotency_key && !input.idempotency_key_present {
        return OperationAuthorizationDecision::deny(
            "operation_idempotency_key_required",
            "Provide an idempotency key before mutation authorization.",
        );
    }
    if input.operation.requires_apply_token && !input.apply_token_present {
        return OperationAuthorizationDecision::deny(
            "operation_apply_token_required",
            "Provide an apply token before apply/admin authorization.",
        );
    }
    if writes_owned_path(&input.operation) && !owned_write_scope_contains(input) {
        return OperationAuthorizationDecision::deny(
            "operation_owned_write_scope_denied",
            "Restrict the write path to an owned write scope from the active lane or takeover receipt.",
        );
    }

    let Ok(policy_set) = compile_default_policy() else {
        return OperationAuthorizationDecision::deny(
            "operation_policy_invalid",
            "Repair the Cedar operation policy before accepting requests.",
        );
    };
    let Ok(entities) = entities_for_input(input) else {
        return OperationAuthorizationDecision::deny(
            "operation_entities_invalid",
            "Repair the Cedar entity projection before accepting requests.",
        );
    };
    let Ok(request) = cedar_request(input) else {
        return OperationAuthorizationDecision::deny(
            "operation_request_invalid",
            "Repair the Cedar request projection before accepting requests.",
        );
    };

    let response = Authorizer::new().is_authorized(&request, &policy_set, &entities);
    if response.decision() == Decision::Allow {
        OperationAuthorizationDecision::allow()
    } else {
        OperationAuthorizationDecision::deny(
            "operation_policy_denied",
            "Check claim, capability, client kind, project scope, and owned path before retrying.",
        )
    }
}

fn cedar_request(input: &OperationAuthorizationInput) -> Result<Request, String> {
    Request::new(
        principal_uid(&input.session_id)?,
        action_uid(&input.operation.operation.0)?,
        resource_uid(&input.operation.operation.0)?,
        Context::empty(),
        None,
    )
    .map_err(|error| error.to_string())
}

fn entities_for_input(input: &OperationAuthorizationInput) -> Result<Entities, String> {
    let project_id = project_id(input);
    let resource_project_id = resource_project_id(input);
    let owned_path = input.owned_path.clone().unwrap_or_default();
    let payload = json!([
        {
            "uid": {"type": "VidaPrincipal", "id": input.session_id},
            "attrs": {
                "claim_kind": claim_kind(&input.claim_kind),
                "capability": capability(&input.capability),
                "client_kind": client_kind(&input.client_kind),
                "project_id": project_id,
                "owned_path": owned_path
            },
            "parents": []
        },
        {
            "uid": {"type": "VidaAction", "id": input.operation.operation.0},
            "attrs": {
                "operation": input.operation.operation.0
            },
            "parents": []
        },
        {
            "uid": {"type": "VidaOperation", "id": input.operation.operation.0},
            "attrs": {
                "operation": input.operation.operation.0,
                "scope": operation_scope(&input.operation.scope),
                "required_claim": claim_kind(&input.operation.required_claim),
                "required_capability": capability(&input.capability),
                "allowed_client_kind": client_kind(&input.client_kind),
                "project_id": resource_project_id,
                "owned_path": owned_path
            },
            "parents": []
        }
    ]);
    Entities::from_json_value(payload, None).map_err(|error| error.to_string())
}

fn principal_uid(session_id: &str) -> Result<EntityUid, String> {
    EntityUid::from_str(&format!("VidaPrincipal::\"{}\"", escape_uid(session_id)))
        .map_err(|error| error.to_string())
}

fn action_uid(operation: &str) -> Result<EntityUid, String> {
    EntityUid::from_str(&format!("VidaAction::\"{}\"", escape_uid(operation)))
        .map_err(|error| error.to_string())
}

fn resource_uid(operation: &str) -> Result<EntityUid, String> {
    EntityUid::from_str(&format!("VidaOperation::\"{}\"", escape_uid(operation)))
        .map_err(|error| error.to_string())
}

fn escape_uid(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn project_id(input: &OperationAuthorizationInput) -> String {
    match (&input.operation.scope, &input.project_id) {
        (VidaOperationScope::Service, _) => "service".to_string(),
        (VidaOperationScope::Project, Some(project_id)) => project_id.0.clone(),
        (VidaOperationScope::Project, None) => String::new(),
    }
}

fn resource_project_id(input: &OperationAuthorizationInput) -> String {
    match (&input.operation.scope, &input.resource_project_id) {
        (VidaOperationScope::Service, _) => "service".to_string(),
        (VidaOperationScope::Project, Some(project_id)) => project_id.0.clone(),
        (VidaOperationScope::Project, None) => project_id(input),
    }
}

fn writes_owned_path(operation: &VidaOperationSpec) -> bool {
    matches!(
        operation.posture,
        VidaOperationPosture::Apply | VidaOperationPosture::Admin
    )
}

fn owned_write_scope_contains(input: &OperationAuthorizationInput) -> bool {
    let Some(owned_path) = input.owned_path.as_deref() else {
        return false;
    };
    let Some(owned_components) = safe_relative_components(owned_path) else {
        return false;
    };

    input.owned_write_scopes.iter().any(|scope| {
        let Some(scope_components) = safe_relative_components(scope) else {
            return false;
        };
        owned_components == scope_components
            || (owned_components.len() > scope_components.len()
                && owned_components.starts_with(&scope_components))
    })
}

fn safe_relative_components(path: &str) -> Option<Vec<&str>> {
    let mut components = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => components.push(value.to_str()?),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}

fn claim_kind(value: &VidaClaimKind) -> &'static str {
    match value {
        VidaClaimKind::Observe => "observe",
        VidaClaimKind::SharedRead => "shared_read",
        VidaClaimKind::ExclusiveWrite => "exclusive_write",
        VidaClaimKind::Dispatch => "dispatch",
        VidaClaimKind::Proof => "proof",
        VidaClaimKind::Admin => "admin",
    }
}

fn client_kind(value: &VidaClientKind) -> String {
    match value {
        VidaClientKind::Cli => "cli".to_string(),
        VidaClientKind::Tui => "tui".to_string(),
        VidaClientKind::Service => "service".to_string(),
        VidaClientKind::Dashboard => "dashboard".to_string(),
        VidaClientKind::HostAgent => "host_agent".to_string(),
        VidaClientKind::Other(value) => format!("other:{value}"),
    }
}

fn capability(value: &VidaCapabilityScope) -> &'static str {
    match value {
        VidaCapabilityScope::ReadStatus => "read_status",
        VidaCapabilityScope::ReadEvents => "read_events",
        VidaCapabilityScope::ReadReceipts => "read_receipts",
        VidaCapabilityScope::ReadConfig => "read_config",
        VidaCapabilityScope::ProjectRegistryRead => "project_registry_read",
        VidaCapabilityScope::ProjectRegistryWrite => "project_registry_write",
        VidaCapabilityScope::WizardRead => "wizard_read",
        VidaCapabilityScope::WizardPlan => "wizard_plan",
        VidaCapabilityScope::WizardApply => "wizard_apply",
        VidaCapabilityScope::MaterializationRead => "materialization_read",
        VidaCapabilityScope::ConfigPlan => "config_plan",
        VidaCapabilityScope::ConfigApply => "config_apply",
        VidaCapabilityScope::MaterializationPlan => "materialization_plan",
        VidaCapabilityScope::MaterializationApply => "materialization_apply",
        VidaCapabilityScope::OrchestrationControlPlaneRead => "orchestration_control_plane_read",
        VidaCapabilityScope::ServiceInstallPlan => "service_install_plan",
        VidaCapabilityScope::ServiceInstallApply => "service_install_apply",
        VidaCapabilityScope::ServiceAdmin => "service_admin",
        VidaCapabilityScope::DiagnosticDetail => "diagnostic_detail",
        VidaCapabilityScope::TaskApply => "task_apply",
        VidaCapabilityScope::RunAdvance => "run_advance",
        VidaCapabilityScope::CompletionRecord => "completion_record",
        VidaCapabilityScope::PacketDispatch => "packet_dispatch",
        VidaCapabilityScope::ClaimWrite => "claim_write",
        VidaCapabilityScope::ProjectionRebuild => "projection_rebuild",
        VidaCapabilityScope::RepairApply => "repair_apply",
    }
}

fn operation_scope(value: &VidaOperationScope) -> &'static str {
    match value {
        VidaOperationScope::Service => "service",
        VidaOperationScope::Project => "project",
    }
}

#[cfg(test)]
mod tests {
    use vida_contracts::{operation_spec, operations};

    use super::*;

    #[test]
    fn cedar_policy_and_entity_snapshot_compile() {
        compile_default_policy().expect("operation authorization policy must compile");
        load_entity_snapshot().expect("entity snapshot must parse");
    }

    #[test]
    fn permits_root_local_write_with_matching_claim_and_scope() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(decision.allowed, "{decision:?}");
    }

    #[test]
    fn permits_delegated_dispatch_write_with_dispatch_claim() {
        let operation =
            operation_spec(operations::PACKET_DISPATCH).expect("packet dispatch should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "agent-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: VidaClaimKind::Dispatch,
            capability: VidaCapabilityScope::PacketDispatch,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(decision.allowed, "{decision:?}");
    }

    #[test]
    fn permits_exception_takeover_write_with_owned_scope() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "exception-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: VidaClaimKind::ExclusiveWrite,
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/vida-contracts/src/lib.rs".to_string()),
            owned_write_scopes: vec!["crates/vida-contracts".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(decision.allowed, "{decision:?}");
    }

    #[test]
    fn denies_cross_project_write() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let mut input = OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        };
        input.project_id = Some(VidaProjectId("foreign-project".to_string()));

        let decision = authorize_operation(&input);
        assert!(!decision.allowed);
        assert_eq!(decision.blocker_codes, vec!["operation_policy_denied"]);

        let mut denied = input.clone();
        denied.operation.requires_project_ref = true;
        denied.project_id = None;
        let decision = authorize_operation(&denied);
        assert!(!decision.allowed);
        assert_eq!(
            decision.blocker_codes,
            vec!["operation_project_ref_required"]
        );
    }

    #[test]
    fn denies_out_of_scope_write_path() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/vida/src/cli.rs".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(!decision.allowed);
        assert_eq!(
            decision.blocker_codes,
            vec!["operation_owned_write_scope_denied"]
        );
    }

    #[test]
    fn denies_traversal_out_of_owned_write_scope() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority/../vida/src/lib.rs".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(!decision.allowed);
        assert_eq!(
            decision.blocker_codes,
            vec!["operation_owned_write_scope_denied"]
        );
    }

    #[test]
    fn denies_sibling_prefix_write_scope_match() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority-extra/src/lib.rs".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: true,
        });

        assert!(!decision.allowed);
        assert_eq!(
            decision.blocker_codes,
            vec!["operation_owned_write_scope_denied"]
        );
    }

    #[test]
    fn denies_missing_apply_token_posture() {
        let operation =
            operation_spec(operations::TASK_APPLY).expect("task apply operation should exist");
        let decision = authorize_operation(&OperationAuthorizationInput {
            session_id: "session-ldr-012".to_string(),
            project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            client_kind: VidaClientKind::HostAgent,
            claim_kind: operation.required_claim.clone(),
            capability: VidaCapabilityScope::TaskApply,
            operation,
            resource_project_id: Some(VidaProjectId("project-ldr-012".to_string())),
            owned_path: Some("crates/taskflow-authority".to_string()),
            owned_write_scopes: vec!["crates/taskflow-authority".to_string()],
            idempotency_key_present: true,
            apply_token_present: false,
        });

        assert!(!decision.allowed);
        assert_eq!(
            decision.blocker_codes,
            vec!["operation_apply_token_required"]
        );
    }
}
