pub(crate) fn infer_runtime_task_class(
    selection: &super::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> String {
    use crate::runtime_contract_vocab::{
        RUNTIME_ROLE_BUSINESS_ANALYST, RUNTIME_ROLE_COACH, RUNTIME_ROLE_PM, RUNTIME_ROLE_PROVER,
        RUNTIME_ROLE_SOLUTION_ARCHITECT, RUNTIME_ROLE_VERIFIER, TASK_CLASS_ARCHITECTURE,
        TASK_CLASS_COACH, TASK_CLASS_SPECIFICATION, TASK_CLASS_VERIFICATION,
    };
    let normalized_request = selection.request.to_lowercase();
    let has_architecture_terms = super::contains_keywords(
        &normalized_request,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "topology".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "refactor".to_string(),
            "migration".to_string(),
            "security".to_string(),
            "hard conflict".to_string(),
            "meta-analysis".to_string(),
            "meta analysis".to_string(),
        ],
    )
    .len()
        >= 2;
    let coach_terms = super::coach_review_terms(&normalized_request);
    if selection.selected_role == RUNTIME_ROLE_SOLUTION_ARCHITECT || has_architecture_terms {
        return TASK_CLASS_ARCHITECTURE.to_string();
    }
    if selection.selected_role == RUNTIME_ROLE_COACH || !coach_terms.is_empty() {
        return TASK_CLASS_COACH.to_string();
    }
    if selection.selected_role == RUNTIME_ROLE_VERIFIER
        || selection.selected_role == RUNTIME_ROLE_PROVER
    {
        return TASK_CLASS_VERIFICATION.to_string();
    }
    if requires_design_gate
        || selection.selected_role == RUNTIME_ROLE_BUSINESS_ANALYST
        || selection.selected_role == RUNTIME_ROLE_PM
    {
        return TASK_CLASS_SPECIFICATION.to_string();
    }
    if !super::contains_keywords(
        &normalized_request,
        &[
            "verify".to_string(),
            "verification".to_string(),
            "proof".to_string(),
            "review".to_string(),
            "audit".to_string(),
            "test".to_string(),
        ],
    )
    .is_empty()
    {
        return TASK_CLASS_VERIFICATION.to_string();
    }
    crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION.to_string()
}

pub(crate) fn infer_execution_runtime_role(
    selection: &super::RuntimeConsumptionLaneSelection,
    task_class: &str,
    requires_design_gate: bool,
) -> String {
    use crate::runtime_contract_vocab::{
        RUNTIME_ROLE_BUSINESS_ANALYST, RUNTIME_ROLE_COACH, RUNTIME_ROLE_PM, RUNTIME_ROLE_WORKER,
        TASK_CLASS_COACH,
    };
    if selection.selected_role == RUNTIME_ROLE_PM {
        return RUNTIME_ROLE_PM.to_string();
    }
    if selection.selected_role == RUNTIME_ROLE_COACH || task_class == TASK_CLASS_COACH {
        return RUNTIME_ROLE_COACH.to_string();
    }
    if requires_design_gate || selection.selected_role == RUNTIME_ROLE_BUSINESS_ANALYST {
        return RUNTIME_ROLE_BUSINESS_ANALYST.to_string();
    }
    if selection.selected_role == RUNTIME_ROLE_WORKER {
        return RUNTIME_ROLE_WORKER.to_string();
    }
    runtime_role_for_task_class(task_class).to_string()
}

pub(crate) fn runtime_role_for_task_class(task_class: &str) -> &'static str {
    use crate::runtime_contract_vocab::{
        RUNTIME_ROLE_BUSINESS_ANALYST, RUNTIME_ROLE_COACH, RUNTIME_ROLE_SOLUTION_ARCHITECT,
        RUNTIME_ROLE_VERIFIER, RUNTIME_ROLE_WORKER, TASK_CLASS_ARCHITECTURE, TASK_CLASS_COACH,
        TASK_CLASS_SPECIFICATION, TASK_CLASS_VERIFICATION,
    };
    match task_class {
        TASK_CLASS_ARCHITECTURE => RUNTIME_ROLE_SOLUTION_ARCHITECT,
        TASK_CLASS_VERIFICATION => RUNTIME_ROLE_VERIFIER,
        TASK_CLASS_COACH => RUNTIME_ROLE_COACH,
        TASK_CLASS_SPECIFICATION => RUNTIME_ROLE_BUSINESS_ANALYST,
        _ => RUNTIME_ROLE_WORKER,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispatchContractLane<'a> {
    pub(crate) task_class: Option<&'a str>,
}

impl<'a> DispatchContractLane<'a> {
    pub(crate) fn from_value(value: &'a serde_json::Value) -> Self {
        Self {
            task_class: value["task_class"].as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BackendAdmissibilityKey {
    Implementation,
    Verification,
    Architecture,
    Specification,
    Coach,
    Analysis,
    Review,
    Conservative(String),
}

impl BackendAdmissibilityKey {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Implementation => crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION,
            Self::Verification => crate::runtime_contract_vocab::TASK_CLASS_VERIFICATION,
            Self::Architecture => crate::runtime_contract_vocab::TASK_CLASS_ARCHITECTURE,
            Self::Specification => crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION,
            Self::Coach => crate::runtime_contract_vocab::TASK_CLASS_COACH,
            Self::Analysis => crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS,
            Self::Review => "review",
            Self::Conservative(value) => value.as_str(),
        }
    }

    pub(crate) fn into_string(self) -> String {
        self.as_str().to_string()
    }
}

pub(crate) fn backend_admissibility_key_for_task_class(
    task_class: &str,
) -> Option<BackendAdmissibilityKey> {
    match task_class.trim() {
        "implementation" | "delivery_task" | "execution_block" | "writer" => {
            Some(BackendAdmissibilityKey::Implementation)
        }
        "verification" | "test_authoring" | "quality_gate" | "release_readiness" => {
            Some(BackendAdmissibilityKey::Verification)
        }
        "architecture" | "execution_preparation" | "escalation" => {
            Some(BackendAdmissibilityKey::Architecture)
        }
        "specification" | "planning" | "analysis" => Some(BackendAdmissibilityKey::Specification),
        "coach" | "review" | "validation" => Some(BackendAdmissibilityKey::Coach),
        _ => None,
    }
}

pub(crate) fn backend_admissibility_key_for_dispatch_target(
    dispatch_target: &str,
    dispatch_contract_lane: Option<&DispatchContractLane<'_>>,
) -> BackendAdmissibilityKey {
    let canonical_target = canonical_dispatch_target_name(dispatch_target.trim());

    if let Some(task_class_key) = dispatch_contract_lane
        .and_then(|lane| lane.task_class)
        .and_then(backend_admissibility_key_for_task_class)
    {
        return task_class_key;
    }

    match canonical_target.as_str() {
        "implementer" | "writer" => BackendAdmissibilityKey::Implementation,
        "execution_preparation" => BackendAdmissibilityKey::Architecture,
        "implementation" | "developer" => BackendAdmissibilityKey::Implementation,
        "verification" | "tester" | "test_author" | "test_authoring" => {
            BackendAdmissibilityKey::Verification
        }
        "architecture" | "architect" | "solution_architect" | "escalation" => {
            BackendAdmissibilityKey::Architecture
        }
        "specification" | "business_analyst" | "pm" | "planning" => {
            BackendAdmissibilityKey::Specification
        }
        "coach" => BackendAdmissibilityKey::Coach,
        "analysis" => BackendAdmissibilityKey::Analysis,
        "review" => BackendAdmissibilityKey::Review,
        other => BackendAdmissibilityKey::Conservative(other.to_string()),
    }
}

pub(crate) fn canonical_dispatch_target_alias(value: &str) -> Option<&'static str> {
    use crate::runtime_contract_vocab::{
        DISPATCH_TARGET_ANALYSIS, DISPATCH_TARGET_CLOSURE, DISPATCH_TARGET_COACH,
        DISPATCH_TARGET_EXECUTION_PREPARATION, DISPATCH_TARGET_IMPLEMENTER,
        DISPATCH_TARGET_SPECIFICATION, DISPATCH_TARGET_VERIFICATION, RUNTIME_ROLE_BUSINESS_ANALYST,
        RUNTIME_ROLE_PM, RUNTIME_ROLE_PROVER, RUNTIME_ROLE_SOLUTION_ARCHITECT,
        RUNTIME_ROLE_VERIFIER, RUNTIME_ROLE_WORKER,
    };

    match value.trim() {
        "writer" | "implementation" | RUNTIME_ROLE_WORKER => Some(DISPATCH_TARGET_IMPLEMENTER),
        RUNTIME_ROLE_BUSINESS_ANALYST | RUNTIME_ROLE_PM => Some(DISPATCH_TARGET_SPECIFICATION),
        RUNTIME_ROLE_VERIFIER | RUNTIME_ROLE_PROVER => Some(DISPATCH_TARGET_VERIFICATION),
        "escalation" | "architecture" | RUNTIME_ROLE_SOLUTION_ARCHITECT => {
            Some(DISPATCH_TARGET_EXECUTION_PREPARATION)
        }
        "release" | "release/closure" => Some(DISPATCH_TARGET_CLOSURE),
        DISPATCH_TARGET_ANALYSIS => Some(DISPATCH_TARGET_ANALYSIS),
        DISPATCH_TARGET_COACH => Some(DISPATCH_TARGET_COACH),
        DISPATCH_TARGET_CLOSURE => Some(DISPATCH_TARGET_CLOSURE),
        DISPATCH_TARGET_EXECUTION_PREPARATION => Some(DISPATCH_TARGET_EXECUTION_PREPARATION),
        DISPATCH_TARGET_IMPLEMENTER => Some(DISPATCH_TARGET_IMPLEMENTER),
        DISPATCH_TARGET_SPECIFICATION => Some(DISPATCH_TARGET_SPECIFICATION),
        DISPATCH_TARGET_VERIFICATION => Some(DISPATCH_TARGET_VERIFICATION),
        _ => None,
    }
}

pub(crate) fn canonical_dispatch_target_name(value: &str) -> String {
    canonical_dispatch_target_alias(value)
        .unwrap_or_else(|| value.trim())
        .to_string()
}

#[derive(Debug, Clone)]
pub(crate) struct AgentInitResolvedRole {
    pub(crate) selected_role: String,
    pub(crate) mapping_source: Option<&'static str>,
}

pub(crate) fn agent_init_explicit_role_selection(
    resolved_role: &AgentInitResolvedRole,
    requested_role: &str,
    request_text: String,
) -> serde_json::Value {
    let role_mapping = resolved_role.mapping_source.map(|source| {
        serde_json::json!({
            "requested_role": requested_role,
            "selected_role": resolved_role.selected_role,
            "source": source,
        })
    });
    serde_json::json!({
        "mode": "explicit_role",
        "selected_role": resolved_role.selected_role,
        "requested_role": requested_role,
        "dispatch_target": requested_role,
        "role_mapping": role_mapping,
        "request_text": request_text,
    })
}

fn sorted_unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn dev_team_role_runtime_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<String> {
    if let Some(runtime_role) = compiled_bundle["dev_team"]["roles"]
        .as_object()
        .and_then(|roles| {
            roles
                .get(requested_role)
                .and_then(|role| role["runtime_role"].as_str())
        })
        .map(ToOwned::to_owned)
    {
        return Some(runtime_role);
    }
    dev_team_readiness["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|role| role["role_id"].as_str() == Some(requested_role))
        .and_then(|role| role["runtime_role"].as_str())
        .map(ToOwned::to_owned)
}

fn dev_team_flow_step_runtime_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<String> {
    if let Some(runtime_role) = compiled_bundle["dev_team"]["flows"]
        .as_object()
        .into_iter()
        .flat_map(|flows| flows.values())
        .flat_map(|flow| flow["steps"].as_array().into_iter().flatten())
        .find_map(|step| {
            if step["role_id"].as_str() == Some(requested_role) {
                step["runtime_role"].as_str().map(ToOwned::to_owned)
            } else {
                None
            }
        })
    {
        return Some(runtime_role);
    }
    dev_team_readiness["flows"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|flow| flow["ordered_steps"].as_array().into_iter().flatten())
        .find_map(|step| {
            if step["role_id"].as_str() == Some(requested_role) {
                step["runtime_role"].as_str().map(ToOwned::to_owned)
            } else {
                None
            }
        })
}

fn dev_team_role_ids(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
) -> Vec<String> {
    let mut role_ids = Vec::new();
    role_ids.extend(
        compiled_bundle["dev_team"]["roles"]
            .as_object()
            .into_iter()
            .flat_map(|roles| roles.keys())
            .cloned(),
    );
    role_ids.extend(
        dev_team_readiness["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    role_ids.extend(
        compiled_bundle["dev_team"]["flows"]
            .as_object()
            .into_iter()
            .flat_map(|flows| flows.values())
            .flat_map(|flow| flow["steps"].as_array().into_iter().flatten())
            .filter_map(|step| step["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    role_ids.extend(
        dev_team_readiness["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|flow| flow["ordered_steps"].as_array().into_iter().flatten())
            .filter_map(|step| step["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    sorted_unique_strings(role_ids)
}

fn legacy_run_graph_role_runtime_role(requested_role: &str) -> Option<&'static str> {
    match requested_role {
        "implementer" => Some("worker"),
        _ => None,
    }
}

pub(crate) fn agent_init_selected_role_allowed(selected_role: &str) -> bool {
    selected_role != "orchestrator"
}

pub(crate) fn resolve_agent_init_explicit_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<AgentInitResolvedRole> {
    if requested_role.is_empty() || requested_role == "orchestrator" {
        return None;
    }
    if agent_init_selected_role_allowed(requested_role)
        && crate::role_exists_in_lane_bundle(compiled_bundle, requested_role)
    {
        return Some(AgentInitResolvedRole {
            selected_role: requested_role.to_string(),
            mapping_source: None,
        });
    }
    if let Some(runtime_role) =
        dev_team_role_runtime_role(compiled_bundle, dev_team_readiness, requested_role)
    {
        if agent_init_selected_role_allowed(&runtime_role)
            && crate::role_exists_in_lane_bundle(compiled_bundle, &runtime_role)
        {
            return Some(AgentInitResolvedRole {
                selected_role: runtime_role,
                mapping_source: Some("dev_team.roles.runtime_role"),
            });
        }
    }
    if let Some(runtime_role) =
        dev_team_flow_step_runtime_role(compiled_bundle, dev_team_readiness, requested_role)
    {
        if agent_init_selected_role_allowed(&runtime_role)
            && crate::role_exists_in_lane_bundle(compiled_bundle, &runtime_role)
        {
            return Some(AgentInitResolvedRole {
                selected_role: runtime_role,
                mapping_source: Some("dev_team.flows.steps.runtime_role"),
            });
        }
    }
    if let Some(runtime_role) = legacy_run_graph_role_runtime_role(requested_role) {
        if agent_init_selected_role_allowed(runtime_role)
            && crate::role_exists_in_lane_bundle(compiled_bundle, runtime_role)
        {
            return Some(AgentInitResolvedRole {
                selected_role: runtime_role.to_string(),
                mapping_source: Some("legacy_run_graph_node_alias"),
            });
        }
    }
    None
}

pub(crate) fn agent_init_role_candidates(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.extend(
        compiled_bundle["enabled_framework_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
    );
    candidates.extend(
        compiled_bundle["project_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    for row in crate::carrier_runtime_section(compiled_bundle)["roles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        candidates.extend(
            ["role_id", "runtime_role", "default_runtime_role"]
                .into_iter()
                .filter_map(|field| row[field].as_str())
                .map(ToOwned::to_owned),
        );
        candidates.extend(
            row["runtime_roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        );
        for profile in row["model_profiles"]
            .as_object()
            .into_iter()
            .flat_map(|profiles| profiles.values())
        {
            candidates.extend(
                profile["runtime_roles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
            );
        }
    }
    candidates.extend(dev_team_role_ids(compiled_bundle, dev_team_readiness));
    if crate::role_exists_in_lane_bundle(compiled_bundle, "worker") {
        candidates.push("implementer".to_string());
    }
    sorted_unique_strings(candidates)
}

pub(crate) fn task_complexity_multiplier(task_class: &str) -> u64 {
    match task_class {
        "architecture" | "execution_preparation" | "hard_escalation" | "meta_analysis" => 4,
        "verification" | "review" | "quality_gate" | "release_readiness" => 2,
        "specification" | "planning" | "coach" | "implementation_medium" => 2,
        _ => 1,
    }
}

pub(crate) fn role_supports_task_class(role: &serde_json::Value, task_class: &str) -> bool {
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    task_classes.is_empty() || task_classes.contains(&task_class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_contract_vocab::{
        RUNTIME_ROLE_PM, RUNTIME_ROLE_VERIFIER, RUNTIME_ROLE_WORKER, TASK_CLASS_SPECIFICATION,
        TASK_CLASS_VERIFICATION,
    };

    fn selection(selected_role: &str, request: &str) -> crate::RuntimeConsumptionLaneSelection {
        crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test".to_string(),
            fallback_role: RUNTIME_ROLE_WORKER.to_string(),
            request: request.to_string(),
            selected_role: selected_role.to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::json!({}),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        }
    }

    #[test]
    fn pm_specification_lane_keeps_specification_even_when_request_mentions_testing() {
        let selection = selection(
            RUNTIME_ROLE_PM,
            "create a full TaskFlow testing program and case matrix",
        );

        assert_eq!(
            infer_runtime_task_class(&selection, false),
            TASK_CLASS_SPECIFICATION
        );
    }

    #[test]
    fn design_gate_keeps_specification_before_keyword_verification() {
        let selection = selection(
            RUNTIME_ROLE_WORKER,
            "design proof and test coverage before implementation",
        );

        assert_eq!(
            infer_runtime_task_class(&selection, true),
            TASK_CLASS_SPECIFICATION
        );
    }

    #[test]
    fn verifier_role_still_maps_to_verification() {
        let selection = selection(RUNTIME_ROLE_VERIFIER, "review the bounded design");

        assert_eq!(
            infer_runtime_task_class(&selection, true),
            TASK_CLASS_VERIFICATION
        );
    }

    #[test]
    fn dispatch_contract_task_class_wins_over_human_target_label() {
        let lane = DispatchContractLane {
            task_class: Some("implementation"),
        };

        assert_eq!(
            backend_admissibility_key_for_dispatch_target("tester", Some(&lane)).as_str(),
            "implementation"
        );
    }

    #[test]
    fn tester_target_uses_verification_without_contract_lane() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("tester", None).as_str(),
            "verification"
        );
    }

    #[test]
    fn architecture_alias_uses_architecture_strictness() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("solution_architect", None).as_str(),
            "architecture"
        );
    }

    #[test]
    fn unknown_dispatch_target_is_conservative() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("custom-lane", None).as_str(),
            "custom-lane"
        );
    }

    #[test]
    fn dispatch_target_aliases_normalize_in_routing_policy() {
        assert_eq!(
            canonical_dispatch_target_name("business_analyst"),
            "specification"
        );
        assert_eq!(canonical_dispatch_target_name("prover"), "verification");
        assert_eq!(
            canonical_dispatch_target_name("escalation"),
            "execution_preparation"
        );
        assert_eq!(canonical_dispatch_target_name("release/closure"), "closure");
        assert_eq!(canonical_dispatch_target_name("custom-lane"), "custom-lane");
    }
}
