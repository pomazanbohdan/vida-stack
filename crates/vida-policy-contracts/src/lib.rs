#![forbid(unsafe_code)]

//! Shared, versioned contracts for the bounded Rhai policy runtime.

use std::{collections::HashSet, fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const POLICY_CONTRACTS_SCHEMA_VERSION: &str = "vida-policy-contracts-v1";
pub const POLICY_SCHEMA_VERSION: u16 = 1;
pub const MAX_POLICY_ARRAY_ITEMS: usize = 64;
pub const MIN_POLICY_SCORE: i64 = 0;
pub const MAX_POLICY_SCORE: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyId {
    #[serde(rename = "rhai.runtime.authority")]
    Authority,
    #[serde(rename = "rhai.runtime.lifecycle")]
    Lifecycle,
    #[serde(rename = "rhai.runtime.failover")]
    Failover,
    #[serde(rename = "rhai.runtime.promotion")]
    Promotion,
    #[serde(rename = "rhai.runtime.rollback")]
    Rollback,
    #[serde(rename = "rhai.runtime.pinned-resume")]
    PinnedResume,
}

impl PolicyId {
    pub const ALL: [Self; 6] = [
        Self::Authority,
        Self::Lifecycle,
        Self::Failover,
        Self::Promotion,
        Self::Rollback,
        Self::PinnedResume,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authority => "rhai.runtime.authority",
            Self::Lifecycle => "rhai.runtime.lifecycle",
            Self::Failover => "rhai.runtime.failover",
            Self::Promotion => "rhai.runtime.promotion",
            Self::Rollback => "rhai.runtime.rollback",
            Self::PinnedResume => "rhai.runtime.pinned-resume",
        }
    }

    const fn family_name(self) -> &'static str {
        match self {
            Self::Authority => "authority",
            Self::Lifecycle => "lifecycle",
            Self::Failover => "failover",
            Self::Promotion => "promotion",
            Self::Rollback => "rollback",
            Self::PinnedResume => "pinned_resume",
        }
    }
}

impl fmt::Display for PolicyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown Rhai policy id '{0}'")]
pub struct UnknownPolicyId(pub String);

impl FromStr for PolicyId {
    type Err = UnknownPolicyId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|policy_id| policy_id.as_str() == value)
            .ok_or_else(|| UnknownPolicyId(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Shadow,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPin {
    pub policy_id: PolicyId,
    pub version: u32,
    pub content_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Candidate,
    Shadow,
    Promotable,
    Active,
    Retired,
    RolledBack,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityContext {
    pub claim: String,
    pub requested_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleContext {
    pub current_state: LifecycleState,
    pub requested_state: LifecycleState,
    pub dependency_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverContext {
    pub failure_code: String,
    pub last_known_good: Option<PolicyPin>,
    pub rust_baseline_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateResult {
    pub gate: String,
    pub passed: bool,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionContext {
    pub replay_score: i64,
    pub parity_score: i64,
    pub gate_results: Vec<GateResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackContext {
    pub failed_version: PolicyPin,
    pub restore_version: PolicyPin,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedResumeContext {
    pub persisted_pin: PolicyPin,
    pub active_pin: PolicyPin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum PolicyContext {
    #[serde(rename = "authority")]
    Authority(AuthorityContext),
    #[serde(rename = "lifecycle")]
    Lifecycle(LifecycleContext),
    #[serde(rename = "failover")]
    Failover(FailoverContext),
    #[serde(rename = "promotion")]
    Promotion(PromotionContext),
    #[serde(rename = "rollback")]
    Rollback(RollbackContext),
    #[serde(rename = "pinned_resume")]
    PinnedResume(PinnedResumeContext),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDecision {
    pub allowed: bool,
    pub score: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleDecision {
    pub admitted: bool,
    pub next_state: LifecycleState,
    pub score: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTarget {
    LastKnownGood,
    RustBaseline,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverDecision {
    pub target: FallbackTarget,
    pub score: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionDecision {
    pub eligible: bool,
    pub score: i64,
    pub gate_results: Vec<GateResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackDecision {
    pub rollback: bool,
    pub quarantine: bool,
    pub score: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedResumeDecision {
    pub compatible: bool,
    pub score: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum PolicyDecision {
    #[serde(rename = "authority")]
    Authority(AuthorityDecision),
    #[serde(rename = "lifecycle")]
    Lifecycle(LifecycleDecision),
    #[serde(rename = "failover")]
    Failover(FailoverDecision),
    #[serde(rename = "promotion")]
    Promotion(PromotionDecision),
    #[serde(rename = "rollback")]
    Rollback(RollbackDecision),
    #[serde(rename = "pinned_resume")]
    PinnedResume(PinnedResumeDecision),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyEvaluationV1 {
    pub schema_version: u16,
    pub policy_id: PolicyId,
    pub version: u32,
    pub mode: PolicyMode,
    pub context: PolicyContext,
    pub decision: PolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyCatalogV1 {
    pub schema_version: u16,
    pub evaluations: Vec<PolicyEvaluationV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyValidationError {
    #[error("unsupported schema version {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("policy array '{field}' has {actual} items; maximum is {max}")]
    ArrayTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error(
        "policy score '{field}' value {value} is outside {MIN_POLICY_SCORE}..={MAX_POLICY_SCORE}"
    )]
    ScoreOutOfRange { field: &'static str, value: i64 },
    #[error("policy {policy_id} has context family '{actual}', expected '{expected}'")]
    MismatchedContext {
        policy_id: PolicyId,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("policy {policy_id} has decision family '{actual}', expected '{expected}'")]
    MismatchedDecision {
        policy_id: PolicyId,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("catalog contains duplicate policy id {0}")]
    DuplicatePolicyId(PolicyId),
}

impl PolicyContext {
    const fn family_name(&self) -> &'static str {
        match self {
            Self::Authority(_) => "authority",
            Self::Lifecycle(_) => "lifecycle",
            Self::Failover(_) => "failover",
            Self::Promotion(_) => "promotion",
            Self::Rollback(_) => "rollback",
            Self::PinnedResume(_) => "pinned_resume",
        }
    }

    fn validate(&self) -> Result<(), PolicyValidationError> {
        match self {
            Self::Authority(context) => validate_array_len(
                "requested_capabilities",
                context.requested_capabilities.len(),
            ),
            Self::Lifecycle(context) => {
                validate_array_len("dependency_ids", context.dependency_ids.len())
            }
            Self::Failover(_) | Self::Rollback(_) | Self::PinnedResume(_) => Ok(()),
            Self::Promotion(context) => {
                validate_gate_results(&context.gate_results, "gate_results")?;
                validate_score("replay_score", context.replay_score)?;
                validate_score("parity_score", context.parity_score)
            }
        }
    }
}

impl PolicyDecision {
    const fn family_name(&self) -> &'static str {
        match self {
            Self::Authority(_) => "authority",
            Self::Lifecycle(_) => "lifecycle",
            Self::Failover(_) => "failover",
            Self::Promotion(_) => "promotion",
            Self::Rollback(_) => "rollback",
            Self::PinnedResume(_) => "pinned_resume",
        }
    }

    fn validate(&self) -> Result<(), PolicyValidationError> {
        match self {
            Self::Authority(decision) => validate_score("score", decision.score),
            Self::Lifecycle(decision) => validate_score("score", decision.score),
            Self::Failover(decision) => validate_score("score", decision.score),
            Self::Promotion(decision) => {
                validate_score("score", decision.score)?;
                validate_gate_results(&decision.gate_results, "gate_results")
            }
            Self::Rollback(decision) => validate_score("score", decision.score),
            Self::PinnedResume(decision) => validate_score("score", decision.score),
        }
    }
}

impl PolicyEvaluationV1 {
    pub fn validate(&self) -> Result<(), PolicyValidationError> {
        if self.schema_version != POLICY_SCHEMA_VERSION {
            return Err(PolicyValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }

        let expected = self.policy_id.family_name();
        let actual_context = self.context.family_name();
        if actual_context != expected {
            return Err(PolicyValidationError::MismatchedContext {
                policy_id: self.policy_id,
                expected,
                actual: actual_context,
            });
        }
        let actual_decision = self.decision.family_name();
        if actual_decision != expected {
            return Err(PolicyValidationError::MismatchedDecision {
                policy_id: self.policy_id,
                expected,
                actual: actual_decision,
            });
        }

        self.context.validate()?;
        self.decision.validate()
    }

    pub fn canonical_blake3_digest(&self) -> Result<String, serde_json::Error> {
        canonical_blake3_digest(self)
    }
}

impl PolicyCatalogV1 {
    pub fn validate(&self) -> Result<(), PolicyValidationError> {
        if self.schema_version != POLICY_SCHEMA_VERSION {
            return Err(PolicyValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        validate_array_len("evaluations", self.evaluations.len())?;

        let mut seen = HashSet::with_capacity(self.evaluations.len());
        for evaluation in &self.evaluations {
            evaluation.validate()?;
            if !seen.insert(evaluation.policy_id) {
                return Err(PolicyValidationError::DuplicatePolicyId(
                    evaluation.policy_id,
                ));
            }
        }
        Ok(())
    }
}

pub fn canonical_blake3_digest(
    evaluation: &PolicyEvaluationV1,
) -> Result<String, serde_json::Error> {
    let canonical_bytes = serde_json::to_vec(evaluation)?;
    Ok(blake3::hash(&canonical_bytes).to_hex().to_string())
}

fn validate_array_len(field: &'static str, actual: usize) -> Result<(), PolicyValidationError> {
    if actual > MAX_POLICY_ARRAY_ITEMS {
        return Err(PolicyValidationError::ArrayTooLarge {
            field,
            actual,
            max: MAX_POLICY_ARRAY_ITEMS,
        });
    }
    Ok(())
}

fn validate_gate_results(
    values: &[GateResult],
    field: &'static str,
) -> Result<(), PolicyValidationError> {
    validate_array_len(field, values.len())?;
    for value in values {
        validate_score("gate_result.score", value.score)?;
    }
    Ok(())
}

fn validate_score(field: &'static str, value: i64) -> Result<(), PolicyValidationError> {
    if !(MIN_POLICY_SCORE..=MAX_POLICY_SCORE).contains(&value) {
        return Err(PolicyValidationError::ScoreOutOfRange { field, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_evaluation() -> PolicyEvaluationV1 {
        PolicyEvaluationV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            policy_id: PolicyId::Authority,
            version: 1,
            mode: PolicyMode::Shadow,
            context: PolicyContext::Authority(AuthorityContext {
                claim: "task-owner".into(),
                requested_capabilities: vec!["read-config".into()],
            }),
            decision: PolicyDecision::Authority(AuthorityDecision {
                allowed: true,
                score: 100,
                reason: "owner-match".into(),
            }),
        }
    }

    #[test]
    fn evaluation_round_trips_and_hashes_deterministically() {
        let evaluation = authority_evaluation();
        let encoded = serde_json::to_vec(&evaluation).expect("serialize evaluation");
        let decoded: PolicyEvaluationV1 =
            serde_json::from_slice(&encoded).expect("deserialize evaluation");

        assert_eq!(evaluation, decoded);
        evaluation.validate().expect("valid evaluation");
        assert_eq!(
            evaluation.canonical_blake3_digest().expect("digest"),
            decoded.canonical_blake3_digest().expect("digest")
        );
    }

    #[test]
    fn serde_rejects_unknown_policy_and_unknown_fields() {
        let unknown_policy = r#"{
            "schema_version": 1,
            "policy_id": "rhai.runtime.unknown",
            "version": 1,
            "mode": "shadow",
            "context": {"kind":"authority","data":{"claim":"x","requested_capabilities":[]}},
            "decision": {"kind":"authority","data":{"allowed":true,"score":1,"reason":"ok"}}
        }"#;
        assert!(serde_json::from_str::<PolicyEvaluationV1>(unknown_policy).is_err());

        let mut value = serde_json::to_value(authority_evaluation()).expect("serialize");
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<PolicyEvaluationV1>(value).is_err());
    }

    #[test]
    fn validation_rejects_mismatched_family_and_score() {
        let mut evaluation = authority_evaluation();
        evaluation.context = PolicyContext::Lifecycle(LifecycleContext {
            current_state: LifecycleState::Candidate,
            requested_state: LifecycleState::Shadow,
            dependency_ids: Vec::new(),
        });
        assert!(matches!(
            evaluation.validate(),
            Err(PolicyValidationError::MismatchedContext { .. })
        ));

        let mut evaluation = authority_evaluation();
        evaluation.decision = PolicyDecision::Authority(AuthorityDecision {
            allowed: true,
            score: 101,
            reason: "invalid".into(),
        });
        assert!(matches!(
            evaluation.validate(),
            Err(PolicyValidationError::ScoreOutOfRange { .. })
        ));
    }

    #[test]
    fn catalog_validation_rejects_oversized_arrays() {
        let evaluation = PolicyEvaluationV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            policy_id: PolicyId::Promotion,
            version: 1,
            mode: PolicyMode::Shadow,
            context: PolicyContext::Promotion(PromotionContext {
                replay_score: 50,
                parity_score: 50,
                gate_results: (0..=MAX_POLICY_ARRAY_ITEMS)
                    .map(|index| GateResult {
                        gate: format!("gate-{index}"),
                        passed: true,
                        score: 50,
                    })
                    .collect(),
            }),
            decision: PolicyDecision::Promotion(PromotionDecision {
                eligible: false,
                score: 50,
                gate_results: Vec::new(),
            }),
        };
        let catalog = PolicyCatalogV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            evaluations: vec![evaluation],
        };

        assert!(matches!(
            catalog.validate(),
            Err(PolicyValidationError::ArrayTooLarge {
                field: "gate_results",
                ..
            })
        ));
    }
}
