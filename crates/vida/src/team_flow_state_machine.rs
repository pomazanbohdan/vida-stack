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

pub const DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE: &str =
    "dispatch_contract_lane_catalog_incomplete";

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
        self.steps
            .iter()
            .map(|step| normalize_step_ref(&step.role_id))
            .collect()
    }

    /// Get the next lawful lane after a given role_id.
    /// Returns None if the current role is not in the state machine or is the last step.
    pub fn resolve_next_lane(&self, current_role: &str) -> Option<String> {
        let current = normalize_step_ref(current_role);
        let current_index = self
            .steps
            .iter()
            .position(|step| normalize_step_ref(&step.role_id) == current);
        current_index
            .and_then(|idx| self.steps.get(idx + 1))
            .map(|next| normalize_step_ref(&next.role_id))
    }

    /// Validate whether a requested next node is lawful after the current role.
    pub fn validate_transition(
        &self,
        current_role: &str,
        requested_next_node: &str,
    ) -> TransitionVerdict {
        let current = normalize_step_ref(current_role);
        let requested = normalize_step_ref(requested_next_node);
        let Some(current_index) = self
            .steps
            .iter()
            .position(|step| normalize_step_ref(&step.role_id) == current)
        else {
            return TransitionVerdict::Blocked {
                blocker_code: "unknown_current_lane".to_string(),
                allowed_next_node: self
                    .steps
                    .first()
                    .map(|step| normalize_step_ref(&step.role_id))
                    .unwrap_or_else(|| "configured_next_lane_unavailable".to_string()),
            };
        };
        let expected_next = self
            .steps
            .get(current_index + 1)
            .map(|step| step.role_id.clone());

        match expected_next {
            Some(ref expected) if normalize_step_ref(expected) == requested => {
                TransitionVerdict::Allowed {
                    next_lane: normalize_step_ref(expected),
                }
            }
            Some(expected) => TransitionVerdict::Blocked {
                blocker_code: "invalid_allowed_next_node_for_execution_plan".to_string(),
                allowed_next_node: normalize_step_ref(&expected),
            },
            None => {
                // Current role is the last step — closure is expected
                if is_terminal_closure_ref(&requested) {
                    TransitionVerdict::Allowed {
                        next_lane: canonical_terminal_closure_ref(&requested),
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
        let rework = normalize_rework_target(rework_target);
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
                    next_lane: if requested == rework {
                        rework.clone()
                    } else {
                        rework_alias
                    },
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
        let role_id = normalize_step_ref(role_id);
        self.steps
            .iter()
            .any(|s| normalize_step_ref(&s.role_id) == role_id)
    }

    /// Get the stage for a given role_id.
    pub fn get_stage_for_role(&self, role_id: &str) -> Option<&str> {
        let role_id = normalize_step_ref(role_id);
        self.steps
            .iter()
            .find(|s| normalize_step_ref(&s.role_id) == role_id)
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
        let sequence_value = dispatch_contract.get(sequence_field).or_else(|| {
            (sequence_field != "lane_sequence")
                .then(|| dispatch_contract.get("lane_sequence"))
                .flatten()
        })?;
        let sequence = normalize_dispatch_sequence(sequence_value)?;
        let full_sequence = match dispatch_contract.get("lane_sequence") {
            Some(value) => Some(normalize_dispatch_sequence(value)?),
            None => None,
        };
        if let Some(full_sequence) = full_sequence.as_ref() {
            if sequence_field != "lane_sequence"
                && !is_ordered_subsequence(full_sequence, &sequence)
            {
                return None;
            }
        }
        let catalog = dispatch_contract.get("lane_catalog")?.as_object()?;
        let steps = sequence
            .into_iter()
            .map(|role_id| {
                let lane = catalog.iter().find_map(|(catalog_role_id, lane)| {
                    (normalize_step_ref(
                        &crate::runtime_assignment_policy::canonical_dispatch_target_name(
                            catalog_role_id,
                        ),
                    ) == role_id)
                        .then_some(lane)
                })?;
                if !lane.is_object() {
                    return None;
                }
                let activation = lane.get("activation").filter(|value| value.is_object());
                let runtime_role = required_nonempty_string(
                    activation
                        .and_then(|activation| activation.get("activation_runtime_role"))
                        .or_else(|| lane.get("activation_runtime_role"))
                        .or_else(|| lane.get("runtime_role")),
                )?;
                let task_class = required_nonempty_string(lane.get("task_class"))?;
                let stage = required_nonempty_string(lane.get("stage"))?;
                Some(StateMachineStep {
                    role_id,
                    runtime_role,
                    task_class,
                    stage,
                })
            })
            .collect::<Option<Vec<_>>>()?;
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

fn normalize_dispatch_sequence(value: &Value) -> Option<Vec<String>> {
    let sequence = value
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .map(|role_id| {
            normalize_step_ref(
                &crate::runtime_assignment_policy::canonical_dispatch_target_name(role_id),
            )
        })
        .collect::<Vec<_>>();
    if sequence.is_empty()
        || sequence.iter().any(|role_id| role_id.trim().is_empty())
        || sequence
            .iter()
            .enumerate()
            .any(|(index, role_id)| sequence[..index].iter().any(|seen| seen == role_id))
    {
        return None;
    }
    Some(sequence)
}

fn is_ordered_subsequence(full_sequence: &[String], candidate: &[String]) -> bool {
    let mut full_index = 0;
    for candidate_role in candidate {
        let Some(relative_index) = full_sequence[full_index..]
            .iter()
            .position(|role_id| role_id == candidate_role)
        else {
            return false;
        };
        full_index += relative_index + 1;
    }
    true
}

fn required_nonempty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_step_ref(value: &str) -> String {
    value.trim().replace('-', "_")
}

fn normalize_rework_target(value: &str) -> String {
    let normalized = normalize_step_ref(value);
    normalized
        .strip_suffix("_rework")
        .unwrap_or(normalized.as_str())
        .to_string()
}

fn canonical_terminal_closure_ref(value: &str) -> String {
    match normalize_step_ref(value).as_str() {
        "release_closure" => "release_closure".to_string(),
        "terminal_closure" => "terminal_closure".to_string(),
        _ => "closure".to_string(),
    }
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

pub fn validate_dispatch_contract(
    dispatch_contract: &Value,
    sequence_field: &str,
) -> Result<TeamFlowStateMachine, &'static str> {
    TeamFlowStateMachine::from_dispatch_contract(dispatch_contract, sequence_field)
        .ok_or(DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE)
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
    fn test_resolve_next_lane_canonicalizes_current_and_next_refs() {
        let sm = TeamFlowStateMachine {
            flow_id: "hyphenated".to_string(),
            steps: vec![
                StateMachineStep {
                    role_id: "alpha-spec".to_string(),
                    runtime_role: "spec_runtime".to_string(),
                    task_class: "specification".to_string(),
                    stage: "design_gate".to_string(),
                },
                StateMachineStep {
                    role_id: "beta-impl".to_string(),
                    runtime_role: "impl_runtime".to_string(),
                    task_class: "implementation".to_string(),
                    stage: "execution".to_string(),
                },
            ],
        };

        assert_eq!(
            sm.resolve_next_lane("alpha-spec"),
            Some("beta_impl".to_string())
        );
        assert_eq!(
            sm.resolve_next_lane("alpha_spec"),
            Some("beta_impl".to_string())
        );
        assert_eq!(
            sm.resolve_execution_lane_sequence(),
            vec!["alpha_spec", "beta_impl"]
        );
        assert_eq!(
            sm.validate_transition("alpha-spec", "beta-impl"),
            TransitionVerdict::Allowed {
                next_lane: "beta_impl".to_string()
            }
        );
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
    fn test_validate_transition_canonicalizes_lane_refs() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert_eq!(
            sm.validate_transition("alpha-spec", "beta-impl"),
            TransitionVerdict::Allowed {
                next_lane: "beta_impl".to_string()
            }
        );
    }

    #[test]
    fn test_validate_transition_rejects_unknown_current_lane_even_for_closure() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert_eq!(
            sm.validate_transition("missing", "closure"),
            TransitionVerdict::Blocked {
                blocker_code: "unknown_current_lane".to_string(),
                allowed_next_node: "alpha_spec".to_string()
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
    fn test_validate_rework_transition_canonicalizes_refs() {
        let bundle = sample_activation_bundle();
        let sm = extract_team_flow_state_machine(&bundle, None).unwrap();
        assert_eq!(
            sm.validate_rework_transition("gamma-gate", "beta-impl-rework", "beta-impl"),
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
                "alpha_impl": {"dispatch_target": "alpha_impl", "runtime_role": "worker", "task_class": "implementation", "stage": "execution"},
                "beta_gate": {"dispatch_target": "beta_gate", "runtime_role": "coach", "task_class": "quality_gate", "stage": "execution"},
                "gamma_verify": {"dispatch_target": "gamma_verify", "runtime_role": "verifier", "task_class": "verification", "stage": "verification"}
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

    #[test]
    fn test_dispatch_contract_rejects_missing_or_incomplete_lane_catalog() {
        let missing_catalog = serde_json::json!({
            "lane_sequence": ["alpha", "beta"]
        });
        assert!(
            resolve_dispatch_contract_lane_sequence(&missing_catalog, "lane_sequence").is_none()
        );

        let incomplete_catalog = serde_json::json!({
            "lane_sequence": ["alpha", "beta"],
            "lane_catalog": {"alpha": {"task_class": "analysis"}}
        });
        assert!(
            resolve_dispatch_contract_lane_sequence(&incomplete_catalog, "lane_sequence").is_none()
        );
    }

    #[test]
    fn test_dispatch_contract_rejects_missing_required_metadata_duplicates_and_mismatches() {
        for (role_id, field) in [("alpha_spec", "task_class"), ("beta_design", "stage")] {
            let mut contract = sample_dispatch_contract();
            contract["lane_catalog"][role_id]
                .as_object_mut()
                .expect("sample lane should be an object")
                .remove(field);
            assert!(
                resolve_dispatch_contract_lane_sequence(&contract, "lane_sequence").is_none(),
                "missing {field} on {role_id} must fail closed"
            );
        }

        let mut missing_runtime_role = sample_dispatch_contract();
        missing_runtime_role["lane_catalog"]["alpha_spec"]["activation"]
            .as_object_mut()
            .expect("sample activation should be an object")
            .remove("activation_runtime_role");
        assert!(
            resolve_dispatch_contract_lane_sequence(&missing_runtime_role, "lane_sequence")
                .is_none(),
            "missing runtime role must fail closed"
        );

        let mut duplicate_lanes = sample_dispatch_contract();
        duplicate_lanes["lane_sequence"] =
            serde_json::json!(["alpha_spec", "beta_design", "beta_design", "gamma_proof"]);
        assert!(
            resolve_dispatch_contract_lane_sequence(&duplicate_lanes, "lane_sequence").is_none(),
            "duplicate lane refs must fail closed"
        );

        let mut sequence_mismatch = sample_dispatch_contract();
        sequence_mismatch["execution_lane_sequence"] =
            serde_json::json!(["gamma_proof", "alpha_spec"]);
        assert!(
            resolve_dispatch_contract_lane_sequence(&sequence_mismatch, "execution_lane_sequence")
                .is_none(),
            "execution sequence outside canonical order must fail closed"
        );
    }

    #[test]
    fn validate_dispatch_contract_returns_canonical_blocker_for_incomplete_catalog() {
        let mut missing_catalog = sample_dispatch_contract();
        missing_catalog
            .as_object_mut()
            .expect("sample contract should be an object")
            .remove("lane_catalog");
        assert!(matches!(
            validate_dispatch_contract(&missing_catalog, "execution_lane_sequence"),
            Err(DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE)
        ));
    }
}
