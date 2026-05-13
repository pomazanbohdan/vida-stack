pub(crate) const TASK_CLASS_ARCHITECTURE: &str = "architecture";
pub(crate) const TASK_CLASS_COACH: &str = "coach";
pub(crate) const TASK_CLASS_IMPLEMENTATION: &str = "implementation";
pub(crate) const TASK_CLASS_SPECIFICATION: &str = "specification";
pub(crate) const TASK_CLASS_VERIFICATION: &str = "verification";

pub(crate) const RUNTIME_ROLE_BUSINESS_ANALYST: &str = "business_analyst";
pub(crate) const RUNTIME_ROLE_COACH: &str = "coach";
pub(crate) const RUNTIME_ROLE_PM: &str = "pm";
pub(crate) const RUNTIME_ROLE_PROVER: &str = "prover";
pub(crate) const RUNTIME_ROLE_SOLUTION_ARCHITECT: &str = "solution_architect";
pub(crate) const RUNTIME_ROLE_VERIFIER: &str = "verifier";
pub(crate) const RUNTIME_ROLE_WORKER: &str = "worker";

pub(crate) const DISPATCH_TARGET_COACH: &str = "coach";
pub(crate) const DISPATCH_TARGET_EXECUTION_PREPARATION: &str = "execution_preparation";
pub(crate) const DISPATCH_TARGET_IMPLEMENTER: &str = "implementer";
pub(crate) const DISPATCH_TARGET_SPECIFICATION: &str = "specification";
pub(crate) const DISPATCH_TARGET_VERIFICATION: &str = "verification";
pub(crate) const DISPATCH_TARGET_CLOSURE: &str = "closure";
pub(crate) const DISPATCH_TARGET_ANALYSIS: &str = "analysis";

pub(crate) fn canonical_dispatch_target_alias(value: &str) -> Option<&'static str> {
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
