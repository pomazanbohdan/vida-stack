/// Team-Flow State-Machine Ownership Boundary
///
/// This module is the canonical owner of team-flow continuation logic.
/// All crates must use these functions instead of hardcoded literals for:
/// - lane_sequence resolution
/// - execution_lane_sequence extraction
/// - allowed_next_node validation
/// - role-to-lane mapping
///
/// Related ADR: docs/product/spec/adr-team-flow-state-machine-owner.md
use serde_json::Value;

/// Verdict for state-machine transition validation
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionVerdict {
    /// The requested next node is lawful per the execution plan
    Allowed { next_lane: String },
    /// The requested next node violates the execution plan
    Blocked {
        blocker_code: String,
        allowed_next_node: String,
    },
}

/// Represents a single step in the team-flow state machine
#[derive(Debug, Clone)]
pub struct StateMachineStep {
    pub role_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub stage: String,
}

/// The canonical team-flow state machine definition
#[derive(Debug, Clone)]
pub struct TeamFlowStateMachine {
    pub flow_id: String,
    pub steps: Vec<StateMachineStep>,
}

impl TeamFlowStateMachine {
    /// Resolve the execution lane sequence from a dev_team flow configuration.
    /// This is the single source of truth for what lanes execute in order.
    pub fn resolve_execution_lane_sequence(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.role_id.clone()).collect()
    }

    /// Get the next lawful lane after a given role_id.
    /// Returns None if the current role is not in the state machine or is the last step.
    pub fn resolve_next_lane(&self, current_role: &str) -> Option<String> {
        let current_index = self.steps.iter().position(|s| s.role_id == current_role);
        current_index.and_then(|idx| self.steps.get(idx + 1).map(|next| next.role_id.clone()))
    }

    /// Validate whether a requested next node is lawful after the current role.
    pub fn validate_transition(
        &self,
        current_role: &str,
        requested_next_node: &str,
    ) -> TransitionVerdict {
        let expected_next = self.resolve_next_lane(current_role);

        match expected_next {
            Some(ref expected) if expected == requested_next_node => TransitionVerdict::Allowed {
                next_lane: expected.clone(),
            },
            Some(expected) => TransitionVerdict::Blocked {
                blocker_code: "invalid_allowed_next_node_for_execution_plan".to_string(),
                allowed_next_node: expected,
            },
            None => {
                // Current role is the last step — closure is expected
                if is_terminal_closure_ref(requested_next_node) {
                    TransitionVerdict::Allowed {
                        next_lane: requested_next_node.trim().to_string(),
                    }
                } else {
                    TransitionVerdict::Blocked {
                        blocker_code: "no_next_lane_after_last_step".to_string(),
                        allowed_next_node: "terminal_closure".to_string(),
                    }
                }
            }
        }
    }

    pub fn validate_rework_transition(
        &self,
        current_role: &str,
        requested_next_node: &str,
        rework_target: &str,
    ) -> TransitionVerdict {
        let current = normalize_step_ref(current_role);
        let requested = normalize_step_ref(requested_next_node);
        let rework = normalize_step_ref(rework_target);
        let current_index = self
            .steps
            .iter()
            .position(|step| normalize_step_ref(&step.role_id) == current);
        let rework_index = self
            .steps
            .iter()
            .position(|step| normalize_step_ref(&step.role_id) == rework);
        if matches!(
            (rework_index, current_index),
            (Some(rework_index), Some(current_index)) if rework_index < current_index
        ) {
            let rework_alias = format!("{rework}_rework");
            if requested == rework || requested == rework_alias {
                return TransitionVerdict::Allowed {
                    next_lane: requested_next_node.trim().to_string(),
                };
            }
        }
        TransitionVerdict::Blocked {
            blocker_code: "invalid_allowed_next_node_for_execution_plan".to_string(),
            allowed_next_node: self
                .resolve_next_lane(current_role)
                .unwrap_or_else(|| "closure".to_string()),
        }
    }

    /// Check if a role_id is a valid step in this state machine.
    pub fn is_valid_role(&self, role_id: &str) -> bool {
        self.steps.iter().any(|s| s.role_id == role_id)
    }

    /// Get the stage for a given role_id.
    pub fn get_stage_for_role(&self, role_id: &str) -> Option<&str> {
        self.steps
            .iter()
            .find(|s| s.role_id == role_id)
            .map(|s| s.stage.as_str())
    }

    /// Build a state machine from a dev_team flow configuration JSON.
    pub fn from_flow_config(flow_config: &Value) -> Option<Self> {
        let flow_id = flow_config.get("flow_id")?.as_str()?.to_string();
        let steps_value = flow_config.get("steps")?;
        let steps_array = steps_value.as_array()?;

        let steps: Vec<StateMachineStep> = steps_array
            .iter()
            .filter_map(|step| {
                let role_id = step.get("role_id")?.as_str()?.to_string();
                let runtime_role = step.get("runtime_role")?.as_str()?.to_string();
                let task_class = step.get("task_class")?.as_str()?.to_string();
                let stage = step.get("stage")?.as_str()?.to_string();

                Some(StateMachineStep {
                    role_id,
                    runtime_role,
                    task_class,
                    stage,
                })
            })
            .collect();

        if steps.is_empty() {
            None
        } else {
            Some(TeamFlowStateMachine { flow_id, steps })
        }
    }

    pub fn from_dispatch_contract(dispatch_contract: &Value, sequence_field: &str) -> Option<Self> {
        let sequence = dispatch_contract
            .get(sequence_field)
            .and_then(Value::as_array)
            .or_else(|| {
                dispatch_contract
                    .get("lane_sequence")
                    .and_then(Value::as_array)
            })?;
        let steps = sequence
            .iter()
            .filter_map(Value::as_str)
            .map(crate::runtime_assignment_policy::canonical_dispatch_target_name)
            .filter(|role_id| !role_id.trim().is_empty())
            .map(|role_id| {
                let lane = dispatch_contract
                    .get("lane_catalog")
                    .and_then(|catalog| catalog.get(role_id.as_str()));
                let activation = lane.and_then(|lane| lane.get("activation"));
                let runtime_role = activation
                    .and_then(|activation| activation.get("activation_runtime_role"))
                    .and_then(Value::as_str)
                    .or_else(|| {
                        lane.and_then(|lane| lane.get("runtime_role"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or(role_id.as_str())
                    .to_string();
                let task_class = lane
                    .and_then(|lane| lane.get("task_class"))
                    .and_then(Value::as_str)
                    .unwrap_or("implementation")
                    .to_string();
                let stage = lane
                    .and_then(|lane| lane.get("stage"))
                    .and_then(Value::as_str)
                    .unwrap_or("execution")
                    .to_string();
                StateMachineStep {
                    role_id,
                    runtime_role,
                    task_class,
                    stage,
                }
            })
            .collect::<Vec<_>>();
        if steps.is_empty() {
            None
        } else {
            Some(TeamFlowStateMachine {
                flow_id: sequence_field.to_string(),
                steps,
            })
        }
    }
}

fn normalize_step_ref(value: &str) -> String {
    value.trim().replace('-', "_")
}

fn is_terminal_closure_ref(value: &str) -> bool {
    matches!(
        normalize_step_ref(value).as_str(),
        "closure" | "terminal_closure" | "release_closure"
    )
}

/// Extract the team-flow state machine from an activation bundle.
/// This function reads from vida.config.yaml -> dev_team.flows and returns
/// the canonical state machine for the given work item type.
pub fn extract_team_flow_state_machine(
    activation_bundle: &Value,
    work_item_type: Option<&str>,
) -> Option<TeamFlowStateMachine> {
    let flows = activation_bundle
        .get("dev_team")?
        .get("flows")?
        .as_object()?;

    // Default flow for tasks is task_delivery_verified
    let flow_id = match work_item_type {
        Some("task") | None => "task_delivery_verified",
        Some("defect") => "defect_repair_verified",
        Some("runtime_defect") => "runtime_defect_remediation",
        Some("pull_request") | Some("pr_repair") => "pr_processing_team",
        Some("architecture") => "architecture_design",
        Some("release_readiness") => "release_readiness_gate",
        _ => "task_delivery_verified", // fallback
    };

    let flow_config = flows.get(flow_id)?;
    TeamFlowStateMachine::from_flow_config(flow_config)
}

/// Resolve the allowed next node for a given role using config-backed logic.
/// This replaces hardcoded lane ids that were previously scattered across
/// runtime_dispatch*, taskflow_routing*, etc.
pub fn resolve_allowed_next_node(activation_bundle: &Value, current_role: &str) -> Option<String> {
    extract_team_flow_state_machine(activation_bundle, None)
        .and_then(|sm| sm.resolve_next_lane(current_role))
}

/// Validate a transition using config-backed state machine logic.
pub fn validate_transition_config_backed(
    activation_bundle: &Value,
    current_role: &str,
    requested_next_node: &str,
) -> TransitionVerdict {
    match extract_team_flow_state_machine(activation_bundle, None) {
        Some(sm) => sm.validate_transition(current_role, requested_next_node),
        None => TransitionVerdict::Blocked {
            blocker_code: "team_flow_state_machine_not_configured".to_string(),
            allowed_next_node: "configured_next_lane_unavailable".to_string(),
        },
    }
}

pub fn resolve_dispatch_contract_lane_sequence(
    dispatch_contract: &Value,
    sequence_field: &str,
) -> Option<Vec<String>> {
    TeamFlowStateMachine::from_dispatch_contract(dispatch_contract, sequence_field)
        .map(|sm| sm.resolve_execution_lane_sequence())
}

pub fn validate_dispatch_contract_transition(
    dispatch_contract: &Value,
    current_role: &str,
    requested_next_node: &str,
) -> Option<TransitionVerdict> {
    TeamFlowStateMachine::from_dispatch_contract(dispatch_contract, "lane_sequence")
        .map(|sm| sm.validate_transition(current_role, requested_next_node))
}

pub fn validate_dispatch_contract_rework_transition(
    dispatch_contract: &Value,
    current_role: &str,
    requested_next_node: &str,
    rework_target: &str,
) -> Option<TransitionVerdict> {
    TeamFlowStateMachine::from_dispatch_contract(dispatch_contract, "execution_lane_sequence")
        .map(|sm| sm.validate_rework_transition(current_role, requested_next_node, rework_target))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_activation_bundle() -> Value {
        serde_json::json!({
            "dev_team": {
                "flows": {
                    "task_delivery_verified": {
                        "flow_id": "task_delivery_verified",
                        "steps": [
                            {"role_id": "alpha_spec", "runtime_role": "spec_runtime", "task_class": "specification", "stage": "design_gate"},
                            {"role_id": "beta_impl", "runtime_role": "impl_runtime", "task_class": "implementation", "stage": "execution"},
                            {"role_id": "gamma_gate", "runtime_role": "gate_runtime", "task_class": "quality_gate", "stage": "execution"},
                            {"role_id": "delta_verify", "runtime_role": "verify_runtime", "task_class": "verification", "stage": "execution"}
                        ]
                    }
                }
            }
        })
    }

    fn sample_dispatch_contract() -> Value {
        serde_json::json!({
            "lane_sequence": ["alpha_spec", "beta_design", "gamma_proof"],
            "execution_lane_sequence": ["alpha_spec", "gamma_proof"],
            "lane_catalog": {
                "alpha_spec": {
                    "task_class": "analysis",
                    "stage": "design_gate",
                    "activation": {
                        "activation_runtime_role": "spec_runtime"
                    }
                },
                "beta_design": {
                    "task_class": "design",
                    "stage": "design_gate",
                    "activation": {
                        "activation_runtime_role": "design_runtime"
                    }
                },
                "gamma_proof": {
                    "task_class": "verification",
                    "stage": "execution",
                    "activation": {
                        "activation_runtime_role": "proof_runtime"
                    }
                }
            }
        })
    }

    #[test]
    fn test_resolve_execution_lane_sequence() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let sequence = sm.resolve_execution_lane_sequence();
        assert_eq!(
            sequence,
            vec!["alpha_spec", "beta_impl", "gamma_gate", "delta_verify"]
        );
    }

    #[test]
    fn test_resolve_next_lane() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert_eq!(
            sm.resolve_next_lane("alpha_spec"),
            Some("beta_impl".to_string())
        );
        assert_eq!(
            sm.resolve_next_lane("beta_impl"),
            Some("gamma_gate".to_string())
        );
        assert_eq!(sm.resolve_next_lane("delta_verify"), None); // last step
    }

    #[test]
    fn test_validate_transition_allowed() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let verdict = sm.validate_transition("alpha_spec", "beta_impl");
        assert_eq!(
            verdict,
            TransitionVerdict::Allowed {
                next_lane: "beta_impl".to_string()
            }
        );
    }

    #[test]
    fn test_validate_transition_blocked() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let verdict = sm.validate_transition("alpha_spec", "delta_verify");
        match verdict {
            TransitionVerdict::Blocked {
                blocker_code,
                allowed_next_node,
            } => {
                assert_eq!(blocker_code, "invalid_allowed_next_node_for_execution_plan");
                assert_eq!(allowed_next_node, "beta_impl");
            }
            _ => panic!("Expected Blocked verdict"),
        }
    }

    #[test]
    fn test_validate_transition_closure_from_last_step() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let verdict = sm.validate_transition("delta_verify", "closure");
        assert_eq!(
            verdict,
            TransitionVerdict::Allowed {
                next_lane: "closure".to_string()
            }
        );
    }

    #[test]
    fn test_validate_transition_accepts_terminal_closure_from_last_step() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let verdict = sm.validate_transition("delta_verify", "terminal_closure");
        assert_eq!(
            verdict,
            TransitionVerdict::Allowed {
                next_lane: "terminal_closure".to_string()
            }
        );
    }

    #[test]
    fn test_validate_rework_transition_accepts_configured_back_edge_alias() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        let verdict = sm.validate_rework_transition("gamma_gate", "beta_impl_rework", "beta_impl");
        assert_eq!(
            verdict,
            TransitionVerdict::Allowed {
                next_lane: "beta_impl_rework".to_string()
            }
        );
    }

    #[test]
    fn test_validate_rework_transition_rejects_forward_target() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert_eq!(
            sm.validate_rework_transition("beta_impl", "delta_verify", "delta_verify"),
            TransitionVerdict::Blocked {
                blocker_code: "invalid_allowed_next_node_for_execution_plan".to_string(),
                allowed_next_node: "gamma_gate".to_string()
            }
        );
    }

    #[test]
    fn test_is_valid_role() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert!(sm.is_valid_role("alpha_spec"));
        assert!(sm.is_valid_role("beta_impl"));
        assert!(!sm.is_valid_role("unknown_role"));
    }

    #[test]
    fn test_from_flow_config_empty_steps() {
        let config = serde_json::json!({
            "flow_id": "empty",
            "steps": []
        });
        assert!(TeamFlowStateMachine::from_flow_config(&config).is_none());
    }

    #[test]
    fn test_from_flow_config_missing_steps() {
        let config = serde_json::json!({
            "flow_id": "no_steps"
        });
        assert!(TeamFlowStateMachine::from_flow_config(&config).is_none());
    }

    #[test]
    fn test_dispatch_contract_lane_sequence_uses_full_state_machine_order() {
        let contract = sample_dispatch_contract();
        assert_eq!(
            resolve_dispatch_contract_lane_sequence(&contract, "lane_sequence"),
            Some(vec![
                "alpha_spec".to_string(),
                "beta_design".to_string(),
                "gamma_proof".to_string()
            ])
        );
        assert_eq!(
            resolve_dispatch_contract_lane_sequence(&contract, "execution_lane_sequence"),
            Some(vec!["alpha_spec".to_string(), "gamma_proof".to_string()])
        );
    }

    #[test]
    fn test_dispatch_contract_transition_validates_against_full_lane_order() {
        let contract = sample_dispatch_contract();
        assert_eq!(
            validate_dispatch_contract_transition(&contract, "alpha_spec", "beta_design"),
            Some(TransitionVerdict::Allowed {
                next_lane: "beta_design".to_string()
            })
        );
        assert_eq!(
            validate_dispatch_contract_transition(&contract, "alpha_spec", "gamma_proof"),
            Some(TransitionVerdict::Blocked {
                blocker_code: "invalid_allowed_next_node_for_execution_plan".to_string(),
                allowed_next_node: "beta_design".to_string()
            })
        );
    }

    #[test]
    fn test_dispatch_contract_rework_transition_accepts_configured_back_edge_alias() {
        let contract = serde_json::json!({
            "lane_sequence": ["alpha_impl", "beta_gate", "gamma_verify"],
            "lane_catalog": {
                "alpha_impl": {"dispatch_target": "alpha_impl", "task_class": "implementation"},
                "beta_gate": {"dispatch_target": "beta_gate", "task_class": "quality_gate"},
                "gamma_verify": {"dispatch_target": "gamma_verify", "task_class": "verification"}
            }
        });

        assert_eq!(
            validate_dispatch_contract_rework_transition(
                &contract,
                "beta_gate",
                "alpha_impl_rework",
                "alpha_impl"
            ),
            Some(TransitionVerdict::Allowed {
                next_lane: "alpha_impl_rework".to_string()
            })
        );
    }
}
