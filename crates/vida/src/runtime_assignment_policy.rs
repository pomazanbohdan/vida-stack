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
        || !super::contains_keywords(
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
    if requires_design_gate
        || selection.selected_role == RUNTIME_ROLE_BUSINESS_ANALYST
        || selection.selected_role == RUNTIME_ROLE_PM
    {
        return TASK_CLASS_SPECIFICATION.to_string();
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

pub(crate) fn task_complexity_multiplier(task_class: &str) -> u64 {
    match task_class {
        "architecture" | "execution_preparation" | "hard_escalation" | "meta_analysis" => 4,
        "verification" | "review" | "quality_gate" | "release_readiness" => 2,
        "specification" | "planning" | "coach" | "implementation_medium" => 2,
        _ => 1,
    }
}

pub(crate) fn role_supports_runtime_role(role: &serde_json::Value, runtime_role: &str) -> bool {
    let runtime_roles = role["runtime_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    runtime_roles.is_empty() || runtime_roles.contains(&runtime_role)
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
