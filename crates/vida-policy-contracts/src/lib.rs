#![forbid(unsafe_code)]

//! Shared, versioned contracts for the bounded Rhai policy runtime.

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const POLICY_CONTRACTS_SCHEMA_VERSION: &str = "vida-policy-contracts-v1";
pub const POLICY_SCHEMA_VERSION: u16 = 1;
pub const MAX_POLICY_ARRAY_ITEMS: usize = 64;
pub const MAX_QUALITY_GATE_STRING_BYTES: usize = 128;
pub const MAX_QUALITY_GATE_RATIONALE_BYTES: usize = 256;
pub const MAX_QUALITY_GATE_RISK_BYTES: usize = 128;
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
    #[serde(rename = "rhai.runtime.quality-gate")]
    QualityGate,
}

impl PolicyId {
    pub const ALL: [Self; 7] = [
        Self::Authority,
        Self::Lifecycle,
        Self::Failover,
        Self::Promotion,
        Self::Rollback,
        Self::PinnedResume,
        Self::QualityGate,
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
            Self::QualityGate => "rhai.runtime.quality-gate",
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
            Self::QualityGate => "quality_gate",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGateProfileId {
    Contract,
    Security,
    #[serde(rename = "a11y", alias = "accessibility")]
    A11y,
    Visual,
    Performance,
    Resilience,
    Property,
    Observability,
}

impl QualityGateProfileId {
    pub const ALL: [Self; 8] = [
        Self::Contract,
        Self::Security,
        Self::A11y,
        Self::Visual,
        Self::Performance,
        Self::Resilience,
        Self::Property,
        Self::Observability,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Security => "security",
            Self::A11y => "a11y",
            Self::Visual => "visual",
            Self::Performance => "performance",
            Self::Resilience => "resilience",
            Self::Property => "property",
            Self::Observability => "observability",
        }
    }
}

impl fmt::Display for QualityGateProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown quality-gate profile id '{0}'")]
pub struct UnknownQualityGateProfileId(pub String);

impl FromStr for QualityGateProfileId {
    type Err = UnknownQualityGateProfileId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.as_str() == value)
            .ok_or_else(|| UnknownQualityGateProfileId(value.to_owned()))
    }
}

pub type QualityGateProfile = QualityGateProfileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGateCheckId {
    #[serde(alias = "schema_compatibility")]
    Contract,
    #[serde(alias = "capability_denial")]
    Security,
    #[serde(rename = "a11y", alias = "accessibility_fixtures")]
    A11y,
    #[serde(alias = "artifact_threshold")]
    Visual,
    #[serde(alias = "performance_budget")]
    Performance,
    #[serde(alias = "failure_recovery")]
    Resilience,
    #[serde(alias = "generated_cases")]
    Property,
    #[serde(alias = "receipt_telemetry")]
    Observability,
}

impl QualityGateCheckId {
    pub const ALL: [Self; 8] = [
        Self::Contract,
        Self::Security,
        Self::A11y,
        Self::Visual,
        Self::Performance,
        Self::Resilience,
        Self::Property,
        Self::Observability,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Security => "security",
            Self::A11y => "a11y",
            Self::Visual => "visual",
            Self::Performance => "performance",
            Self::Resilience => "resilience",
            Self::Property => "property",
            Self::Observability => "observability",
        }
    }
}

impl fmt::Display for QualityGateCheckId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown quality-gate check id '{0}'")]
pub struct UnknownQualityGateCheckId(pub String);

impl FromStr for QualityGateCheckId {
    type Err = UnknownQualityGateCheckId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|check| check.as_str() == value)
            .ok_or_else(|| UnknownQualityGateCheckId(value.to_owned()))
    }
}

pub type QualityGateCheck = QualityGateCheckId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGateBaselineVerdict {
    Pass,
    Fail,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGateRecommendation {
    NoChange,
    AdditiveProfile,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPin {
    pub policy_id: PolicyId,
    pub version: u32,
    pub content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityGateContextV1 {
    pub schema_version: u16,
    pub task_id: String,
    pub policy_id: PolicyId,
    pub policy_version: u32,
    pub content_digest: String,
    pub profile_id: QualityGateProfileId,
    pub mode: PolicyMode,
    pub baseline_verdict: QualityGateBaselineVerdict,
    pub inputs_digest: String,
    pub capability_snapshot: BTreeMap<String, bool>,
    pub limits: BTreeMap<String, u64>,
    pub pin: Option<PolicyPin>,
    pub receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityGateDecisionV1 {
    pub schema_version: u16,
    pub decision_id: String,
    pub policy_id: PolicyId,
    pub policy_version: u32,
    pub content_digest: String,
    pub profile_id: QualityGateProfileId,
    pub recommendation: QualityGateRecommendation,
    pub additive_profiles: Vec<QualityGateProfileId>,
    pub check_ids: Vec<QualityGateCheckId>,
    pub rationale: String,
    pub risk: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub receipt_id: String,
    pub deterministic_digest: String,
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
    #[serde(rename = "quality_gate")]
    QualityGate(QualityGateContextV1),
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
    #[serde(rename = "quality_gate")]
    QualityGate(QualityGateDecisionV1),
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
    #[error("quality-gate field '{field}' is empty")]
    QualityGateEmptyField { field: &'static str },
    #[error("quality-gate field '{field}' has {actual} bytes; maximum is {max}")]
    QualityGateStringTooLong {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("quality-gate map '{field}' has {actual} entries; maximum is {max}")]
    QualityGateMapTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("quality-gate digest '{field}' is not 64 lowercase hexadecimal characters")]
    QualityGateInvalidDigest { field: &'static str },
    #[error("quality-gate policy id must be rhai.runtime.quality-gate, got {0}")]
    QualityGatePolicyIdMismatch(PolicyId),
    #[error("quality-gate profile '{0}' is duplicated")]
    DuplicateQualityGateProfile(QualityGateProfileId),
    #[error("quality-gate check '{0}' is duplicated")]
    DuplicateQualityGateCheck(QualityGateCheckId),
}

impl QualityGateContextV1 {
    pub fn validate(&self) -> Result<(), PolicyValidationError> {
        if self.schema_version != POLICY_SCHEMA_VERSION {
            return Err(PolicyValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.policy_id != PolicyId::QualityGate {
            return Err(PolicyValidationError::QualityGatePolicyIdMismatch(
                self.policy_id,
            ));
        }
        validate_quality_gate_string("task_id", &self.task_id, true)?;
        validate_quality_gate_digest("content_digest", &self.content_digest)?;
        validate_quality_gate_digest("inputs_digest", &self.inputs_digest)?;
        validate_quality_gate_map("capability_snapshot", &self.capability_snapshot)?;
        validate_quality_gate_map("limits", &self.limits)?;
        validate_quality_gate_string("receipt_id", &self.receipt_id, true)?;
        if let Some(pin) = &self.pin {
            if pin.policy_id != PolicyId::QualityGate {
                return Err(PolicyValidationError::QualityGatePolicyIdMismatch(
                    pin.policy_id,
                ));
            }
            validate_quality_gate_digest("pin.content_digest", &pin.content_digest)?;
        }
        Ok(())
    }
}

impl QualityGateDecisionV1 {
    pub fn validate(&self) -> Result<(), PolicyValidationError> {
        if self.schema_version != POLICY_SCHEMA_VERSION {
            return Err(PolicyValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.policy_id != PolicyId::QualityGate {
            return Err(PolicyValidationError::QualityGatePolicyIdMismatch(
                self.policy_id,
            ));
        }
        validate_quality_gate_string("decision_id", &self.decision_id, true)?;
        validate_quality_gate_digest("content_digest", &self.content_digest)?;
        validate_quality_gate_string("rationale", &self.rationale, false)?;
        validate_quality_gate_string("risk", &self.risk, false)?;
        validate_quality_gate_string("receipt_id", &self.receipt_id, true)?;
        validate_quality_gate_digest("deterministic_digest", &self.deterministic_digest)?;
        validate_quality_gate_profiles(&self.additive_profiles)?;
        validate_quality_gate_checks(&self.check_ids)?;
        validate_quality_gate_strings("blockers", &self.blockers)?;
        validate_quality_gate_strings("evidence_refs", &self.evidence_refs)?;
        Ok(())
    }

    pub fn effective_profiles(
        &self,
        rust_required: &[QualityGateProfileId],
        explicit_profiles: &[QualityGateProfileId],
    ) -> Result<Vec<QualityGateProfileId>, PolicyValidationError> {
        effective_profiles(rust_required, explicit_profiles, &self.additive_profiles)
    }
}

pub fn effective_profiles(
    rust_required: &[QualityGateProfileId],
    explicit_profiles: &[QualityGateProfileId],
    rhai_additions: &[QualityGateProfileId],
) -> Result<Vec<QualityGateProfileId>, PolicyValidationError> {
    validate_quality_gate_profiles(rust_required)?;
    validate_quality_gate_profiles(explicit_profiles)?;
    validate_quality_gate_profiles(rhai_additions)?;

    let mut effective =
        Vec::with_capacity(rust_required.len() + explicit_profiles.len() + rhai_additions.len());
    for profile in rust_required
        .iter()
        .chain(explicit_profiles)
        .chain(rhai_additions)
    {
        if !effective.contains(profile) {
            effective.push(*profile);
        }
    }
    Ok(effective)
}

fn validate_quality_gate_string(
    field: &'static str,
    value: &str,
    required: bool,
) -> Result<(), PolicyValidationError> {
    if required && value.is_empty() {
        return Err(PolicyValidationError::QualityGateEmptyField { field });
    }
    let actual = value.len();
    let max = match field {
        "rationale" => MAX_QUALITY_GATE_RATIONALE_BYTES,
        "risk" => MAX_QUALITY_GATE_RISK_BYTES,
        _ => MAX_QUALITY_GATE_STRING_BYTES,
    };
    if actual > max {
        return Err(PolicyValidationError::QualityGateStringTooLong { field, actual, max });
    }
    Ok(())
}

fn validate_quality_gate_strings(
    field: &'static str,
    values: &[String],
) -> Result<(), PolicyValidationError> {
    validate_array_len(field, values.len())?;
    for value in values {
        validate_quality_gate_string(field, value, true)?;
    }
    Ok(())
}

fn validate_quality_gate_map<T>(
    field: &'static str,
    values: &BTreeMap<String, T>,
) -> Result<(), PolicyValidationError> {
    if values.len() > MAX_POLICY_ARRAY_ITEMS {
        return Err(PolicyValidationError::QualityGateMapTooLarge {
            field,
            actual: values.len(),
            max: MAX_POLICY_ARRAY_ITEMS,
        });
    }
    for key in values.keys() {
        validate_quality_gate_string(field, key, true)?;
    }
    Ok(())
}

fn validate_quality_gate_digest(
    field: &'static str,
    value: &str,
) -> Result<(), PolicyValidationError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        return Err(PolicyValidationError::QualityGateInvalidDigest { field });
    }
    Ok(())
}

fn validate_quality_gate_profiles(
    values: &[QualityGateProfileId],
) -> Result<(), PolicyValidationError> {
    validate_array_len("profiles", values.len())?;
    let mut seen = HashSet::with_capacity(values.len());
    for profile in values {
        if !seen.insert(*profile) {
            return Err(PolicyValidationError::DuplicateQualityGateProfile(*profile));
        }
    }
    Ok(())
}

fn validate_quality_gate_checks(
    values: &[QualityGateCheckId],
) -> Result<(), PolicyValidationError> {
    validate_array_len("check_ids", values.len())?;
    let mut seen = HashSet::with_capacity(values.len());
    for check in values {
        if !seen.insert(*check) {
            return Err(PolicyValidationError::DuplicateQualityGateCheck(*check));
        }
    }
    Ok(())
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
            Self::QualityGate(_) => "quality_gate",
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
            Self::QualityGate(context) => context.validate(),
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
            Self::QualityGate(_) => "quality_gate",
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
            Self::QualityGate(decision) => decision.validate(),
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

    fn quality_context() -> QualityGateContextV1 {
        QualityGateContextV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            task_id: "task-quality-gate".into(),
            policy_id: PolicyId::QualityGate,
            policy_version: 1,
            content_digest: "a".repeat(64),
            profile_id: QualityGateProfileId::Contract,
            mode: PolicyMode::Shadow,
            baseline_verdict: QualityGateBaselineVerdict::Pass,
            inputs_digest: "b".repeat(64),
            capability_snapshot: BTreeMap::from([(String::from("read_only"), true)]),
            limits: BTreeMap::from([(String::from("max_instructions"), 1000)]),
            pin: Some(PolicyPin {
                policy_id: PolicyId::QualityGate,
                version: 1,
                content_digest: "c".repeat(64),
            }),
            receipt_id: "receipt-quality-gate".into(),
        }
    }

    fn quality_decision() -> QualityGateDecisionV1 {
        QualityGateDecisionV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            decision_id: "decision-quality-gate".into(),
            policy_id: PolicyId::QualityGate,
            policy_version: 1,
            content_digest: "a".repeat(64),
            profile_id: QualityGateProfileId::Contract,
            recommendation: QualityGateRecommendation::AdditiveProfile,
            additive_profiles: vec![QualityGateProfileId::Security],
            check_ids: vec![QualityGateCheckId::Contract],
            rationale: "schema compatibility is required".into(),
            risk: "medium".into(),
            blockers: Vec::new(),
            evidence_refs: vec!["receipt://quality-gate".into()],
            receipt_id: "receipt-quality-gate".into(),
            deterministic_digest: "d".repeat(64),
        }
    }

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

    #[test]
    fn quality_gate_contracts_round_trip_and_validate() {
        let evaluation = PolicyEvaluationV1 {
            schema_version: POLICY_SCHEMA_VERSION,
            policy_id: PolicyId::QualityGate,
            version: 1,
            mode: PolicyMode::Shadow,
            context: PolicyContext::QualityGate(quality_context()),
            decision: PolicyDecision::QualityGate(quality_decision()),
        };
        evaluation
            .validate()
            .expect("valid quality-gate evaluation");
        let encoded = serde_json::to_vec(&evaluation).expect("serialize quality-gate");
        let decoded: PolicyEvaluationV1 =
            serde_json::from_slice(&encoded).expect("deserialize quality-gate");
        assert_eq!(evaluation, decoded);
        assert_eq!(
            PolicyId::QualityGate.to_string(),
            "rhai.runtime.quality-gate"
        );
        assert_eq!(
            "rhai.runtime.quality-gate".parse::<PolicyId>().unwrap(),
            PolicyId::QualityGate
        );
    }

    #[test]
    fn quality_gate_rejects_unknown_profile_check_and_fields() {
        let decision = quality_decision();
        let mut value = serde_json::to_value(decision).expect("serialize decision");
        value["additive_profiles"] = serde_json::json!(["unknown"]);
        assert!(serde_json::from_value::<QualityGateDecisionV1>(value).is_err());

        let mut value = serde_json::to_value(quality_decision()).expect("serialize decision");
        value["check_ids"] = serde_json::json!(["unknown"]);
        assert!(serde_json::from_value::<QualityGateDecisionV1>(value).is_err());

        let mut value = serde_json::to_value(quality_context()).expect("serialize context");
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<QualityGateContextV1>(value).is_err());
    }

    #[test]
    fn quality_gate_bounds_and_effective_profile_union_are_fail_closed() {
        let mut decision = quality_decision();
        decision.rationale = "x".repeat(MAX_QUALITY_GATE_RATIONALE_BYTES + 1);
        assert!(matches!(
            decision.validate(),
            Err(PolicyValidationError::QualityGateStringTooLong {
                field: "rationale",
                ..
            })
        ));

        let mut context = quality_context();
        context.capability_snapshot = (0..=MAX_POLICY_ARRAY_ITEMS)
            .map(|index| (format!("capability-{index}"), true))
            .collect();
        assert!(matches!(
            context.validate(),
            Err(PolicyValidationError::QualityGateMapTooLarge {
                field: "capability_snapshot",
                ..
            })
        ));

        let decision = quality_decision();
        assert_eq!(
            decision
                .effective_profiles(
                    &[QualityGateProfileId::Contract],
                    &[QualityGateProfileId::Visual],
                )
                .expect("effective profile union"),
            vec![
                QualityGateProfileId::Contract,
                QualityGateProfileId::Visual,
                QualityGateProfileId::Security,
            ]
        );

        let duplicate = effective_profiles(
            &[
                QualityGateProfileId::Contract,
                QualityGateProfileId::Contract,
            ],
            &[],
            &[],
        );
        assert!(matches!(
            duplicate,
            Err(PolicyValidationError::DuplicateQualityGateProfile(
                QualityGateProfileId::Contract
            ))
        ));
    }
}
