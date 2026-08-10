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
    crate::runtime_assignment_policy::canonical_dispatch_target_alias(value)
}

pub(crate) fn canonical_dispatch_target_name(value: &str) -> String {
    crate::runtime_assignment_policy::canonical_dispatch_target_name(value)
}

/// Return the backend-admissibility matrix key that represents a task class.
///
/// Configured development-team flows may keep human role labels (for example
/// `developer` or `tester`) as lane ids. Backend admissibility must still be
/// enforced against the canonical task-class lane, not the arbitrary label.
pub(crate) fn backend_admissibility_key_for_task_class(task_class: &str) -> Option<&'static str> {
    match task_class.trim() {
        "implementation" | "delivery_task" | "execution_block" | "writer" => Some("implementation"),
        "verification" | "test_authoring" | "quality_gate" | "release_readiness" => {
            Some("verification")
        }
        "architecture" | "execution_preparation" | "escalation" => Some("architecture"),
        "specification" | "planning" | "analysis" => Some("specification"),
        "coach" | "review" | "validation" => Some("coach"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_aliases_trim_known_targets_and_retain_unknown_names() {
        for (input, expected) in [
            (" coach ", DISPATCH_TARGET_COACH),
            ("implementer", DISPATCH_TARGET_IMPLEMENTER),
            ("verification", DISPATCH_TARGET_VERIFICATION),
        ] {
            assert_eq!(canonical_dispatch_target_alias(input), Some(expected));
            assert_eq!(canonical_dispatch_target_name(input), expected);
        }

        assert_eq!(canonical_dispatch_target_alias(""), None);
        assert_eq!(
            canonical_dispatch_target_name(" custom-lane "),
            "custom-lane"
        );
    }

    #[test]
    fn backend_admissibility_maps_aliases_to_canonical_task_classes() {
        for (input, expected) in [
            (" writer ", TASK_CLASS_IMPLEMENTATION),
            ("test_authoring", TASK_CLASS_VERIFICATION),
            ("execution_preparation", TASK_CLASS_ARCHITECTURE),
            ("planning", TASK_CLASS_SPECIFICATION),
            ("review", TASK_CLASS_COACH),
        ] {
            assert_eq!(
                backend_admissibility_key_for_task_class(input),
                Some(expected)
            );
        }

        assert_eq!(backend_admissibility_key_for_task_class("unknown"), None);
    }

    #[test]
    fn backend_admissibility_preserves_direct_classes_and_rejects_blank_values() {
        for (input, expected) in [
            (TASK_CLASS_IMPLEMENTATION, TASK_CLASS_IMPLEMENTATION),
            (TASK_CLASS_VERIFICATION, TASK_CLASS_VERIFICATION),
            (TASK_CLASS_ARCHITECTURE, TASK_CLASS_ARCHITECTURE),
            (TASK_CLASS_SPECIFICATION, TASK_CLASS_SPECIFICATION),
            (TASK_CLASS_COACH, TASK_CLASS_COACH),
        ] {
            assert_eq!(
                backend_admissibility_key_for_task_class(input),
                Some(expected)
            );
        }
        assert_eq!(backend_admissibility_key_for_task_class("   "), None);
    }
}
