use taskflow_core::role_step::{
    RoleStepFlowDefinition, RoleStepStateError, TaskRoleStepState, TaskRoleStepStatus,
};
use thiserror::Error;

pub const MODULE: &str = "role_step";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleStepTransitionVerdict {
    pub allowed: bool,
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoleStepAuthorityError {
    #[error(transparent)]
    State(#[from] RoleStepStateError),
}

pub fn authorize_allowed_next_node(
    state: &TaskRoleStepState,
    flow: &RoleStepFlowDefinition,
    allowed_next_node: &str,
    current_flow_hash: &str,
) -> Result<RoleStepTransitionVerdict, RoleStepAuthorityError> {
    match state.accept_next(flow, allowed_next_node, current_flow_hash) {
        Ok(_) => Ok(RoleStepTransitionVerdict {
            allowed: true,
            blocker_codes: Vec::new(),
        }),
        Err(RoleStepStateError::UnknownAllowedNextNode(_, _)) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_allowed_next_node_not_configured".to_string()],
        }),
        Err(RoleStepStateError::FlowVersionDrift { .. }) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_flow_version_drift".to_string()],
        }),
        Err(RoleStepStateError::CurrentStepNotCompleted { .. }) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_current_step_not_completed".to_string()],
        }),
        Err(RoleStepStateError::UnresolvedBlockers { .. }) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_unresolved_blockers".to_string()],
        }),
        Err(RoleStepStateError::NonSequentialAllowedNextNode { .. }) => {
            Ok(RoleStepTransitionVerdict {
                allowed: false,
                blocker_codes: vec!["role_step_allowed_next_node_not_sequential".to_string()],
            })
        }
        Err(RoleStepStateError::NoNextStep { .. }) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_no_next_step".to_string()],
        }),
        Err(RoleStepStateError::TerminalState(_)) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_terminal_state".to_string()],
        }),
        Err(RoleStepStateError::EmptyFlow) => Ok(RoleStepTransitionVerdict {
            allowed: false,
            blocker_codes: vec!["role_step_empty_flow".to_string()],
        }),
    }
}

pub fn role_step_is_closure_ready(state: &TaskRoleStepState) -> bool {
    state.status == TaskRoleStepStatus::Completed && state.blockers.is_empty()
}

#[cfg(test)]
mod tests {
    use taskflow_core::role_step::{RoleStepDefinition, TaskRoleStepStatus};

    use super::*;

    fn flow() -> RoleStepFlowDefinition {
        RoleStepFlowDefinition {
            flow_id: "task".to_string(),
            schema_hash: "hash-1".to_string(),
            steps: vec![
                RoleStepDefinition {
                    role_id: "analyst".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "analysis".to_string(),
                    lifecycle_stage: "analysis".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
                RoleStepDefinition {
                    role_id: "developer".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "implementation".to_string(),
                    lifecycle_stage: "implementation".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
            ],
        }
    }

    #[test]
    fn accepts_configured_handoff() {
        let flow = flow();
        let mut state = TaskRoleStepState::from_first_step(&flow).unwrap();
        state.complete().unwrap();

        let verdict = authorize_allowed_next_node(&state, &flow, "developer", "hash-1").unwrap();

        assert!(verdict.allowed);
        assert!(verdict.blocker_codes.is_empty());
    }

    #[test]
    fn rejects_unconfigured_handoff_with_canonical_blocker() {
        let flow = flow();
        let mut state = TaskRoleStepState::from_first_step(&flow).unwrap();
        state.complete().unwrap();

        let verdict = authorize_allowed_next_node(&state, &flow, "tester", "hash-1").unwrap();

        assert!(!verdict.allowed);
        assert_eq!(
            verdict.blocker_codes,
            vec!["role_step_allowed_next_node_not_configured"]
        );
    }

    #[test]
    fn rejects_flow_version_drift_with_canonical_blocker() {
        let flow = flow();
        let state = TaskRoleStepState::from_first_step(&flow).unwrap();

        let verdict = authorize_allowed_next_node(&state, &flow, "developer", "hash-2").unwrap();

        assert!(!verdict.allowed);
        assert_eq!(verdict.blocker_codes, vec!["role_step_flow_version_drift"]);
    }

    #[test]
    fn closure_readiness_requires_completed_state_without_blockers() {
        let flow = flow();
        let mut state = TaskRoleStepState::from_first_step(&flow).unwrap();
        assert!(!role_step_is_closure_ready(&state));
        state.status = TaskRoleStepStatus::Accepted;
        assert!(!role_step_is_closure_ready(&state));
        state.status = TaskRoleStepStatus::Completed;
        assert!(role_step_is_closure_ready(&state));
        state.blockers.push("missing_proof".to_string());
        assert!(!role_step_is_closure_ready(&state));
    }
    #[test]
    fn rejects_incomplete_and_non_sequential_handoffs_with_canonical_blockers() {
        let flow = RoleStepFlowDefinition {
            flow_id: "task".to_string(),
            schema_hash: "hash-1".to_string(),
            steps: vec![
                RoleStepDefinition {
                    role_id: "analyst".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "analysis".to_string(),
                    lifecycle_stage: "analysis".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
                RoleStepDefinition {
                    role_id: "developer".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "implementation".to_string(),
                    lifecycle_stage: "implementation".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
                RoleStepDefinition {
                    role_id: "tester".to_string(),
                    runtime_role: "verification".to_string(),
                    task_class: "verification".to_string(),
                    lifecycle_stage: "verification".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
            ],
        };
        let state = TaskRoleStepState::from_first_step(&flow).unwrap();
        let verdict = authorize_allowed_next_node(&state, &flow, "developer", "hash-1").unwrap();
        assert!(!verdict.allowed);
        assert_eq!(
            verdict.blocker_codes,
            vec!["role_step_current_step_not_completed"]
        );

        let mut completed = state;
        completed.complete().unwrap();
        let verdict = authorize_allowed_next_node(&completed, &flow, "tester", "hash-1").unwrap();
        assert!(!verdict.allowed);
        assert_eq!(
            verdict.blocker_codes,
            vec!["role_step_allowed_next_node_not_sequential"]
        );
    }
}
