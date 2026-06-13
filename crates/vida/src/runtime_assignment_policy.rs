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
    crate::runtime_contract_vocab::backend_admissibility_key_for_task_class(task_class)
        .map(backend_admissibility_key_from_canonical)
}

pub(crate) fn backend_admissibility_key_for_dispatch_target(
    dispatch_target: &str,
    dispatch_contract_lane: Option<&DispatchContractLane<'_>>,
) -> BackendAdmissibilityKey {
    let canonical_target =
        crate::runtime_contract_vocab::canonical_dispatch_target_name(dispatch_target.trim());

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

fn backend_admissibility_key_from_canonical(value: &str) -> BackendAdmissibilityKey {
    match value {
        "implementation" => BackendAdmissibilityKey::Implementation,
        "verification" => BackendAdmissibilityKey::Verification,
        "architecture" => BackendAdmissibilityKey::Architecture,
        "specification" => BackendAdmissibilityKey::Specification,
        "coach" => BackendAdmissibilityKey::Coach,
        "analysis" => BackendAdmissibilityKey::Analysis,
        "review" => BackendAdmissibilityKey::Review,
        other => BackendAdmissibilityKey::Conservative(other.to_string()),
    }
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
}
