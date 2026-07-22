//! Compatibility projection over the typed TeamFlow transition authority.
//!
//! This module intentionally contains no independent transition rules.  The
//! runtime adapter owns config parsing and `taskflow_authority` owns admission;
//! the legacy surface below only preserves the small API still used by older
//! consumers while delegating every decision to a `TeamFlowSnapshot`.

use crate::team_flow_authority_adapter::{
    team_flow_authority_availability, TeamFlowAuthorityAvailability,
};
use serde_json::Value;
use taskflow_authority::team_flow_transition::{
    admit_transition, TeamFlowReceipt, TeamFlowSnapshot, BLOCKER_REWORK_TARGET_NOT_CONFIGURED,
};

pub const DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE: &str =
    "dispatch_contract_lane_catalog_incomplete";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamFlowResolutionBlocker {
    pub code: &'static str,
    pub sequence_field: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionVerdict {
    Allowed {
        next_lane: String,
    },
    Blocked {
        blocker_code: String,
        allowed_next_node: String,
    },
}

#[derive(Debug, Clone)]
pub struct StateMachineStep {
    pub role_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub stage: String,
}

#[derive(Debug, Clone)]
pub struct TeamFlowStateMachine {
    pub flow_id: String,
    pub steps: Vec<StateMachineStep>,
    snapshot: TeamFlowSnapshot,
}

impl TeamFlowStateMachine {
    /// Typed execution boundary. Legacy callers may still use the `Vec` wrapper
    /// below for diagnostics, but execution must preserve a blocker instead of
    /// turning an unavailable or malformed sequence into an empty plan.
    pub(crate) fn resolve_execution_lane_sequence_status(
        &self,
    ) -> Result<Vec<String>, TeamFlowResolutionBlocker> {
        let sequence = self.snapshot.ordered_configured_nodes().to_vec();
        if sequence.is_empty() {
            return Err(TeamFlowResolutionBlocker {
                code: DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE,
                sequence_field: "execution_lane_sequence".to_string(),
            });
        }
        Ok(sequence)
    }

    /// Diagnostic-only compatibility projection. Executable callers must use
    /// `resolve_execution_lane_sequence_status` so blockers remain visible.
    pub fn resolve_execution_lane_sequence(&self) -> Vec<String> {
        self.resolve_execution_lane_sequence_status().map_or_else(
            |blocker| vec![format!("blocked:{}", blocker.code)],
            |sequence| sequence,
        )
    }

    pub fn resolve_next_lane(&self, current_role: &str) -> Option<String> {
        self.snapshot
            .node(current_role.trim())
            .and_then(|node| node.next_node.clone())
    }

    pub fn validate_transition(
        &self,
        current_role: &str,
        requested_next_node: &str,
    ) -> TransitionVerdict {
        self.admit(current_role, requested_next_node, "pass", &[])
    }

    pub fn validate_transition_with_evidence(
        &self,
        current_role: &str,
        requested_next_node: &str,
        evidence: &[String],
    ) -> TransitionVerdict {
        self.admit(current_role, requested_next_node, "pass", evidence)
    }

    pub fn validate_rework_transition(
        &self,
        current_role: &str,
        requested_next_node: &str,
        rework_target: &str,
    ) -> TransitionVerdict {
        let Some(node) = self.snapshot.node(current_role.trim()) else {
            return self.admit(current_role, requested_next_node, "rework", &[]);
        };
        let rework_target = rework_target.trim();
        let requested_next_node = requested_next_node.trim();
        if rework_target.is_empty()
            || requested_next_node != rework_target
            || !node
                .rework_targets
                .iter()
                .any(|target| target.trim() == rework_target)
        {
            return TransitionVerdict::Blocked {
                blocker_code: BLOCKER_REWORK_TARGET_NOT_CONFIGURED.to_string(),
                allowed_next_node: node.next_node.clone().unwrap_or_else(String::new),
            };
        }
        self.admit(current_role, rework_target, "rework", &[])
    }

    pub fn is_valid_role(&self, role_id: &str) -> bool {
        self.snapshot.node(role_id.trim()).is_some()
    }

    pub fn get_stage_for_role(&self, role_id: &str) -> Option<&str> {
        self.steps
            .iter()
            .find(|step| step.role_id == role_id.trim())
            .map(|step| step.stage.as_str())
    }

    /// Dispatch contracts do not carry the validated immutable TeamFlow snapshot.
    /// Preserve a typed blocker so callers cannot execute from raw JSON catalogs.
    pub fn resolve_dispatch_contract(
        _dispatch_contract: &Value,
        sequence_field: &str,
    ) -> Result<Self, TeamFlowResolutionBlocker> {
        Err(TeamFlowResolutionBlocker {
            code: DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE,
            sequence_field: sequence_field.to_string(),
        })
    }

    fn admit(
        &self,
        current: &str,
        requested: &str,
        status: &str,
        evidence: &[String],
    ) -> TransitionVerdict {
        let Some(node) = self.snapshot.node(current.trim()) else {
            return TransitionVerdict::Blocked {
                blocker_code: "team_flow_current_node_unknown".to_string(),
                // An unknown cursor has no lawful successor.  Never fabricate
                // the first ordered node as an executable recovery edge.
                allowed_next_node: String::new(),
            };
        };
        if evidence.is_empty() {
            return TransitionVerdict::Blocked {
                blocker_code: "team_flow_transition_evidence_missing".to_string(),
                allowed_next_node: node
                    .next_node
                    .clone()
                    .unwrap_or_else(|| "team_flow_next_node_unconfigured".to_string()),
            };
        }
        let receipt = TeamFlowReceipt {
            receipt_id: "compatibility-receipt".to_string(),
            node_id: current.trim().to_string(),
            status: status.to_string(),
            evidence: evidence.to_vec(),
            config_hash: self.snapshot.config_hash.clone(),
            snapshot_ref: self.snapshot.snapshot_ref.clone(),
        };
        let verdict = admit_transition(&self.snapshot, current, Some(&receipt), requested);
        if verdict.allowed {
            TransitionVerdict::Allowed {
                next_lane: verdict
                    .next_node
                    .unwrap_or_else(|| requested.trim().to_string()),
            }
        } else {
            TransitionVerdict::Blocked {
                blocker_code: verdict
                    .blocker
                    .unwrap_or_else(|| "team_flow_transition_blocked".to_string()),
                allowed_next_node: verdict.expected.unwrap_or_else(String::new),
            }
        }
    }
}

/// Canonical execution availability is owned by the runtime authority adapter.
/// This thin consumer helper exposes that status without re-parsing roles,
/// flows, or carrier defaults in the state-machine module.
pub(crate) fn team_flow_execution_authority_availability(
    activation_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> TeamFlowAuthorityAvailability {
    team_flow_authority_availability(activation_bundle, flow_ref, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_token(label: &str) -> String {
        format!("fixture-{label}")
    }

    fn canonical_state_machine() -> TeamFlowStateMachine {
        let bundle = crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle, None, None,
        )
        .expect("canonical persisted TeamFlow authority should compile");
        let snapshot = authority.projection().snapshot.clone();
        let steps = snapshot
            .nodes
            .iter()
            .map(|node| StateMachineStep {
                role_id: node.node_id.clone(),
                runtime_role: node.runtime_role.clone(),
                task_class: node.task_class.clone(),
                stage: node.inclusion_rule.clone(),
            })
            .collect();
        TeamFlowStateMachine {
            flow_id: snapshot.flow_ref.clone(),
            steps,
            snapshot,
        }
    }

    #[test]
    fn typed_snapshot_controls_next_terminal_and_rework() {
        let state_machine = canonical_state_machine();
        let sequence = state_machine.resolve_execution_lane_sequence();
        assert!(!sequence.is_empty());
        let first = sequence
            .first()
            .expect("configured flow must have a first node");
        let next = state_machine
            .resolve_next_lane(first)
            .expect("configured first node must point to its next node");
        let required_evidence = state_machine
            .snapshot
            .node(first)
            .expect("configured first node must exist in the typed snapshot")
            .evidence_requirements
            .clone();
        assert!(!required_evidence.is_empty());
        assert_eq!(
            state_machine.validate_transition_with_evidence(
                first,
                &next,
                &[fixture_token("evidence")],
            ),
            TransitionVerdict::Blocked {
                blocker_code: "team_flow_required_evidence_missing".to_string(),
                allowed_next_node: next.clone(),
            }
        );
        assert_eq!(
            state_machine.validate_transition_with_evidence(first, &next, &required_evidence),
            TransitionVerdict::Allowed {
                next_lane: next.clone()
            }
        );
        assert!(matches!(
            state_machine.validate_transition(first, &fixture_token("unknown-node")),
            TransitionVerdict::Blocked { .. }
        ));
        assert!(matches!(
            state_machine.validate_transition(&fixture_token("unknown-current"), first),
            TransitionVerdict::Blocked {
                blocker_code,
                allowed_next_node
            } if blocker_code == "team_flow_current_node_unknown" && allowed_next_node.is_empty()
        ));
        assert!(matches!(
            state_machine.validate_rework_transition(first, first, first),
            TransitionVerdict::Blocked { .. }
        ));
        assert!(matches!(
            state_machine.validate_rework_transition(
                first,
                &next,
                &fixture_token("unknown-rework-target")
            ),
            TransitionVerdict::Blocked { blocker_code, .. }
                if blocker_code == BLOCKER_REWORK_TARGET_NOT_CONFIGURED
        ));
        assert!(matches!(
            state_machine.validate_rework_transition(first, &next, &next),
            TransitionVerdict::Blocked { blocker_code, .. }
                if blocker_code == BLOCKER_REWORK_TARGET_NOT_CONFIGURED
        ));
        let terminal = sequence
            .last()
            .expect("configured flow must have a terminal node");
        assert!(matches!(
            state_machine.validate_transition(terminal, &fixture_token("closure")),
            TransitionVerdict::Blocked { .. }
        ));
    }

    #[test]
    fn raw_dispatch_contract_requires_typed_snapshot_authority() {
        let blocker =
            TeamFlowStateMachine::resolve_dispatch_contract(&Value::Null, "lane_sequence")
                .expect_err("raw dispatch contracts cannot provide execution authority");
        assert_eq!(blocker.code, DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE);
        assert_eq!(blocker.sequence_field, "lane_sequence");
    }
}
