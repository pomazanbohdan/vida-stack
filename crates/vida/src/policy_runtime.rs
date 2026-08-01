//! Rust-owned facade for versioned Rhai policy evaluation.
//!
//! This module deliberately keeps selection, pinning, mode semantics, fallback,
//! and receipt construction in Rust.  A Rhai bundle can only provide a typed
//! decision; it cannot choose a bundle or authorize an effect by itself.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vida_policy_rhai::{build_policy_engine, Limits, PolicyBundle};

pub const POLICY_RUNTIME_SCHEMA_VERSION: u16 = 1;
pub const POLICY_RUNTIME_BASELINE_DIGEST: &str = "rust-baseline-v1";
const QUALITY_GATE_POLICY_ID: &str = "rhai.runtime.quality-gate";
const SUPPORTED_POLICY_IDS: [&str; 7] = [
    "rhai.runtime.authority",
    "rhai.runtime.lifecycle",
    "rhai.runtime.failover",
    "rhai.runtime.promotion",
    "rhai.runtime.rollback",
    "rhai.runtime.pinned-resume",
    "rhai.runtime.quality-gate",
];
const QUALITY_GATE_PROFILE_IDS: [&str; 8] = [
    "contract",
    "security",
    "a11y",
    "visual",
    "performance",
    "resilience",
    "property",
    "observability",
];
const QUALITY_GATE_CHECK_IDS: [&str; 8] = QUALITY_GATE_PROFILE_IDS;
const QUALITY_GATE_HARD_PROFILES: [&str; 3] = ["contract", "security", "observability"];
const QUALITY_GATE_MAX_ITEMS: usize = 64;
const QUALITY_GATE_MAX_LIMITS: usize = 16;
const QUALITY_GATE_MAX_CONTEXT_BYTES: u64 = 4 * 1024 * 1024;
const QUALITY_GATE_MAX_EVALUATION_MS: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateBaselineRequest {
    pub(crate) schema_version: u16,
    pub(crate) hard_profiles: Vec<String>,
    pub(crate) config_profiles: Vec<String>,
    pub(crate) task_profiles: Vec<String>,
    pub(crate) path_profiles: Vec<String>,
    pub(crate) explicit_profiles: Vec<String>,
    pub(crate) explicit_check_ids: Vec<String>,
    pub(crate) limits: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateProfileResolution {
    pub(crate) baseline_profiles: Vec<String>,
    pub(crate) effective_profiles: Vec<String>,
    pub(crate) check_ids: Vec<String>,
    pub(crate) limits: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualityGateContextV1 {
    pub(crate) schema_version: u16,
    pub(crate) baseline_verdict: String,
    pub(crate) hard_profiles: Vec<String>,
    pub(crate) config_profiles: Vec<String>,
    pub(crate) task_profiles: Vec<String>,
    pub(crate) path_profiles: Vec<String>,
    pub(crate) explicit_profiles: Vec<String>,
    pub(crate) explicit_check_ids: Vec<String>,
    pub(crate) limits: BTreeMap<String, u64>,
}

impl QualityGateContextV1 {
    pub(crate) fn baseline_request(&self) -> QualityGateBaselineRequest {
        QualityGateBaselineRequest {
            schema_version: self.schema_version,
            hard_profiles: self.hard_profiles.clone(),
            config_profiles: self.config_profiles.clone(),
            task_profiles: self.task_profiles.clone(),
            path_profiles: self.path_profiles.clone(),
            explicit_profiles: self.explicit_profiles.clone(),
            explicit_check_ids: self.explicit_check_ids.clone(),
            limits: self.limits.clone(),
        }
    }

    pub(crate) fn redacted_input(&self) -> Result<Value, PolicyRuntimeError> {
        if self.baseline_verdict != "pass" && self.baseline_verdict != "blocked" {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_baseline_verdict_invalid".to_string(),
            ));
        }
        let request = self.baseline_request();
        validate_quality_gate_baseline_request(&request)?;
        let value = serde_json::to_value(self)
            .map_err(|error| PolicyRuntimeError::InvalidDecision(error.to_string()))?;
        let bytes = serde_json::to_vec(&value)
            .map_err(|error| PolicyRuntimeError::InvalidDecision(error.to_string()))?;
        if bytes.len() > QUALITY_GATE_MAX_CONTEXT_BYTES as usize {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_context_limit".to_string(),
            ));
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualityGateShadowDecision {
    pub(crate) schema_version: u16,
    pub(crate) additive_profiles: Vec<String>,
    pub(crate) check_ids: Vec<String>,
    pub(crate) rationale: String,
    pub(crate) risk: String,
}

impl QualityGateShadowDecision {
    fn from_value(value: &Value) -> Result<Self, PolicyRuntimeError> {
        let object = value.as_object().ok_or_else(|| {
            PolicyRuntimeError::InvalidDecision("quality_gate_output_not_object".to_string())
        })?;
        let allowed_fields = [
            "schema_version",
            "additive_profiles",
            "check_ids",
            "rationale",
            "risk",
        ];
        if object
            .keys()
            .any(|key| !allowed_fields.contains(&key.as_str()))
        {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_output_unknown_field".to_string(),
            ));
        }
        if object.get("schema_version").and_then(Value::as_u64) != Some(1) {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_output_schema_invalid".to_string(),
            ));
        }
        let additive_profiles =
            quality_gate_string_array(object.get("additive_profiles"), "profiles")?;
        let check_ids = quality_gate_string_array(object.get("check_ids"), "check_ids")?;
        for profile in &additive_profiles {
            validate_quality_gate_id(profile, &QUALITY_GATE_PROFILE_IDS, "profile")?;
        }
        for check in &check_ids {
            validate_quality_gate_id(check, &QUALITY_GATE_CHECK_IDS, "check")?;
        }
        let rationale = object
            .get("rationale")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PolicyRuntimeError::InvalidDecision("quality_gate_rationale_invalid".to_string())
            })?
            .to_string();
        if rationale.trim().is_empty() || rationale.len() > 512 {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_rationale_limit".to_string(),
            ));
        }
        let risk = object
            .get("risk")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PolicyRuntimeError::InvalidDecision("quality_gate_risk_invalid".to_string())
            })?
            .to_string();
        if !matches!(risk.as_str(), "low" | "medium" | "high") {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_risk_unknown".to_string(),
            ));
        }
        Ok(Self {
            schema_version: 1,
            additive_profiles,
            check_ids,
            rationale,
            risk,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateShadowRequest {
    pub(crate) run_id: String,
    pub(crate) bundle_id: String,
    pub(crate) context: QualityGateContextV1,
    pub(crate) pinned: Option<PolicyPin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateShadowOutcome {
    pub(crate) decision: Option<QualityGateShadowDecision>,
    pub(crate) resolution: QualityGateProfileResolution,
    pub(crate) receipt: PolicyReceipt,
}

impl QualityGateShadowOutcome {
    pub(crate) fn shadow_receipt(
        &self,
        bundle_id: impl Into<String>,
    ) -> crate::state_store::policy::PolicyShadowReceipt {
        crate::state_store::policy::PolicyShadowReceipt {
            receipt_id: self.receipt.receipt_id.clone(),
            run_id: self.receipt.run_id.clone(),
            bundle_id: bundle_id.into(),
            policy_id: self.receipt.policy_id.clone(),
            version: self.receipt.version,
            content_digest: self.receipt.content_digest.clone(),
            input_digest: self.receipt.input_digest.clone(),
            output_digest: self.receipt.output_digest.clone(),
            duration_ms: self.receipt.duration_ms,
            agreed: self.receipt.agreed,
            diff_code: self.receipt.diff_code.clone(),
            error_code: self.receipt.error_code.clone(),
            fallback_code: self.receipt.fallback_code.clone(),
        }
    }
}

pub(crate) fn quality_gate_baseline_profiles(
    request: &QualityGateBaselineRequest,
) -> Result<Vec<String>, PolicyRuntimeError> {
    validate_quality_gate_baseline_request(request)?;
    let hard_profiles = QUALITY_GATE_HARD_PROFILES
        .iter()
        .map(|profile| (*profile).to_string())
        .collect::<Vec<_>>();
    Ok(canonical_quality_gate_union([
        &hard_profiles,
        &request.hard_profiles,
        &request.config_profiles,
        &request.task_profiles,
        &request.path_profiles,
        &request.explicit_profiles,
    ]))
}

pub(crate) fn quality_gate_baseline_resolution(
    request: &QualityGateBaselineRequest,
) -> Result<QualityGateProfileResolution, PolicyRuntimeError> {
    let baseline_profiles = quality_gate_baseline_profiles(request)?;
    let check_ids = canonical_quality_gate_union([&request.explicit_check_ids]);
    Ok(QualityGateProfileResolution {
        baseline_profiles: baseline_profiles.clone(),
        effective_profiles: baseline_profiles,
        check_ids,
        limits: request.limits.clone(),
    })
}

pub(crate) fn quality_gate_final_profiles(
    request: &QualityGateBaselineRequest,
    rhai_output: &Value,
) -> Result<QualityGateProfileResolution, PolicyRuntimeError> {
    let baseline = quality_gate_baseline_resolution(request)?;
    let baseline_profiles = baseline.baseline_profiles;
    let object = rhai_output.as_object().ok_or_else(|| {
        PolicyRuntimeError::InvalidDecision("quality_gate_output_not_object".to_string())
    })?;
    let allowed_fields = [
        "schema_version",
        "additive_profiles",
        "check_ids",
        "rationale",
        "risk",
    ];
    if object
        .keys()
        .any(|key| !allowed_fields.contains(&key.as_str()))
    {
        return Err(PolicyRuntimeError::InvalidDecision(
            "quality_gate_output_unknown_field".to_string(),
        ));
    }
    if object.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(PolicyRuntimeError::InvalidDecision(
            "quality_gate_output_schema_invalid".to_string(),
        ));
    }
    if let Some(rationale) = object.get("rationale") {
        let rationale = rationale.as_str().ok_or_else(|| {
            PolicyRuntimeError::InvalidDecision("quality_gate_rationale_invalid".to_string())
        })?;
        if rationale.trim().is_empty() || rationale.len() > 512 {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_rationale_limit".to_string(),
            ));
        }
    }
    if let Some(risk) = object.get("risk") {
        let risk = risk.as_str().ok_or_else(|| {
            PolicyRuntimeError::InvalidDecision("quality_gate_risk_invalid".to_string())
        })?;
        if !matches!(risk, "low" | "medium" | "high") {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_risk_unknown".to_string(),
            ));
        }
    }
    let additions = quality_gate_string_array(object.get("additive_profiles"), "profiles")?;
    let rhai_checks = quality_gate_string_array(object.get("check_ids"), "check_ids")?;
    for profile in &additions {
        validate_quality_gate_id(profile, &QUALITY_GATE_PROFILE_IDS, "profile")?;
    }
    for check in &rhai_checks {
        validate_quality_gate_id(check, &QUALITY_GATE_CHECK_IDS, "check")?;
    }
    let mut checks = baseline.check_ids;
    checks.extend(rhai_checks);
    checks = canonical_quality_gate_union([&checks]);
    Ok(QualityGateProfileResolution {
        baseline_profiles: baseline_profiles.clone(),
        effective_profiles: canonical_quality_gate_union([&baseline_profiles, &additions]),
        check_ids: checks,
        limits: request.limits.clone(),
    })
}

fn validate_quality_gate_baseline_request(
    request: &QualityGateBaselineRequest,
) -> Result<(), PolicyRuntimeError> {
    if request.schema_version != 1 {
        return Err(PolicyRuntimeError::InvalidDecision(
            "quality_gate_schema_invalid".to_string(),
        ));
    }
    for (kind, values) in [
        ("hard", &request.hard_profiles),
        ("config", &request.config_profiles),
        ("task", &request.task_profiles),
        ("path", &request.path_profiles),
        ("explicit", &request.explicit_profiles),
    ] {
        if values.len() > QUALITY_GATE_MAX_ITEMS {
            return Err(PolicyRuntimeError::InvalidDecision(format!(
                "quality_gate_{kind}_profiles_limit"
            )));
        }
        for profile in values {
            validate_quality_gate_id(profile, &QUALITY_GATE_PROFILE_IDS, "profile")?;
        }
    }
    if request.explicit_check_ids.len() > QUALITY_GATE_MAX_ITEMS {
        return Err(PolicyRuntimeError::InvalidDecision(
            "quality_gate_check_ids_limit".to_string(),
        ));
    }
    for check in &request.explicit_check_ids {
        validate_quality_gate_id(check, &QUALITY_GATE_CHECK_IDS, "check")?;
    }
    if request.limits.len() > QUALITY_GATE_MAX_LIMITS {
        return Err(PolicyRuntimeError::InvalidDecision(
            "quality_gate_limits_limit".to_string(),
        ));
    }
    for (key, value) in &request.limits {
        let max = match key.as_str() {
            "max_profiles" | "max_checks" => QUALITY_GATE_MAX_ITEMS as u64,
            "max_context_bytes" => QUALITY_GATE_MAX_CONTEXT_BYTES,
            "max_evaluation_ms" => QUALITY_GATE_MAX_EVALUATION_MS,
            _ => {
                return Err(PolicyRuntimeError::InvalidDecision(
                    "quality_gate_limit_unknown".to_string(),
                ));
            }
        };
        if *value == 0 || *value > max {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_limit_invalid".to_string(),
            ));
        }
    }
    Ok(())
}

fn quality_gate_string_array(
    value: Option<&Value>,
    field: &str,
) -> Result<Vec<String>, PolicyRuntimeError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or_else(|| {
        PolicyRuntimeError::InvalidDecision(format!("quality_gate_{field}_not_array"))
    })?;
    if values.len() > QUALITY_GATE_MAX_ITEMS {
        return Err(PolicyRuntimeError::InvalidDecision(format!(
            "quality_gate_{field}_limit"
        )));
    }
    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                PolicyRuntimeError::InvalidDecision(format!("quality_gate_{field}_id_invalid"))
            })
        })
        .collect()
}

fn validate_quality_gate_id(
    value: &str,
    allowed: &[&str],
    kind: &str,
) -> Result<(), PolicyRuntimeError> {
    if value.trim().is_empty() || value.len() > 128 || !allowed.contains(&value) {
        return Err(PolicyRuntimeError::InvalidDecision(format!(
            "quality_gate_{kind}_id_unknown"
        )));
    }
    Ok(())
}

fn canonical_quality_gate_union<const N: usize>(sources: [&[String]; N]) -> Vec<String> {
    QUALITY_GATE_PROFILE_IDS
        .iter()
        .filter(|profile| {
            sources
                .iter()
                .any(|source| source.iter().any(|value| value == *profile))
        })
        .map(|profile| profile.to_string())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Off,
    Shadow,
    Active,
}

impl Default for PolicyMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPin {
    pub policy_id: String,
    pub version: u32,
    pub content_digest: String,
}

impl PolicyPin {
    pub fn new(
        policy_id: impl Into<String>,
        version: u32,
        content_digest: impl Into<String>,
    ) -> Result<Self, PolicyRuntimeError> {
        let pin = Self {
            policy_id: policy_id.into(),
            version,
            content_digest: content_digest.into(),
        };
        pin.validate()?;
        Ok(pin)
    }

    pub fn validate(&self) -> Result<(), PolicyRuntimeError> {
        if self.policy_id.trim().is_empty()
            || self.policy_id.len() > 128
            || self.content_digest.trim().is_empty()
            || self.content_digest.len() > 128
        {
            return Err(PolicyRuntimeError::InvalidPin);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedPolicyDecision {
    pub schema_version: u16,
    pub allowed: bool,
    pub score: i64,
    pub recommendation: String,
    #[serde(default)]
    pub additive_profiles: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl TypedPolicyDecision {
    pub fn validate(&self) -> Result<(), PolicyRuntimeError> {
        if self.schema_version != POLICY_RUNTIME_SCHEMA_VERSION {
            return Err(PolicyRuntimeError::InvalidDecision(
                "unsupported decision schema".to_string(),
            ));
        }
        if !(0..=100).contains(&self.score) {
            return Err(PolicyRuntimeError::InvalidDecision(
                "score must be between 0 and 100".to_string(),
            ));
        }
        if self.recommendation.trim().is_empty() {
            return Err(PolicyRuntimeError::InvalidDecision(
                "recommendation must be non-empty".to_string(),
            ));
        }
        if !matches!(
            self.recommendation.as_str(),
            "no_change" | "additive_profile" | "block"
        ) {
            return Err(PolicyRuntimeError::InvalidDecision(
                "unsupported recommendation".to_string(),
            ));
        }
        if self.additive_profiles.len() > 64
            || self.blockers.len() > 64
            || self.evidence_refs.len() > 64
        {
            return Err(PolicyRuntimeError::InvalidDecision(
                "decision arrays exceed the bounded limit".to_string(),
            ));
        }
        for value in self
            .additive_profiles
            .iter()
            .chain(self.blockers.iter())
            .chain(self.evidence_refs.iter())
        {
            if value.trim().is_empty() || value.len() > 256 {
                return Err(PolicyRuntimeError::InvalidDecision(
                    "decision string exceeds the bounded limit".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<String, PolicyRuntimeError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| PolicyRuntimeError::InvalidDecision(error.to_string()))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTarget {
    LastKnownGood,
    RustBaseline,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyEvaluationRequest {
    pub run_id: String,
    pub input: Value,
    pub rust_decision: TypedPolicyDecision,
    #[serde(default)]
    pub pinned: Option<PolicyPin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyReceipt {
    pub schema_version: u16,
    pub receipt_id: String,
    pub run_id: String,
    pub policy_id: String,
    pub version: u32,
    pub content_digest: String,
    pub mode: PolicyMode,
    pub input_digest: String,
    pub output_digest: Option<String>,
    pub duration_ms: u64,
    pub agreed: Option<bool>,
    pub diff_code: Option<String>,
    pub error_code: Option<String>,
    pub fallback_code: Option<String>,
    pub authorizes_effects: bool,
}

impl PolicyReceipt {
    pub fn is_redacted(&self) -> bool {
        let raw = serde_json::to_string(self).unwrap_or_default();
        !raw.contains("context")
            && !raw.contains("source")
            && !raw.contains("secret")
            && !raw.contains("arbitrary")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluationOutcome {
    pub decision: TypedPolicyDecision,
    pub shadow_decision: Option<TypedPolicyDecision>,
    pub fallback: Option<FallbackTarget>,
    pub receipt: PolicyReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyRuntimeError {
    InvalidPin,
    BundleNotFound(PolicyPin),
    PinnedBundleMissing(PolicyPin),
    InvalidDecision(String),
    InvalidModeTransition { from: PolicyMode, to: PolicyMode },
    InvalidBundleDigest { expected: String, observed: String },
    InvalidBundlePolicy { expected: String, observed: String },
    Evaluation(String),
    FallbackUnavailable,
    InFlightPinConflict { run_id: String },
}

impl std::fmt::Display for PolicyRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPin => formatter.write_str("invalid policy pin"),
            Self::BundleNotFound(pin) => write!(formatter, "policy bundle not found: {pin:?}"),
            Self::PinnedBundleMissing(pin) => {
                write!(formatter, "policy pinned bundle missing: {pin:?}")
            }
            Self::InvalidDecision(detail) => write!(formatter, "invalid policy decision: {detail}"),
            Self::InvalidModeTransition { from, to } => {
                write!(
                    formatter,
                    "invalid policy mode transition: {from:?} -> {to:?}"
                )
            }
            Self::InvalidBundleDigest { expected, observed } => {
                write!(
                    formatter,
                    "policy digest mismatch: expected {expected}, observed {observed}"
                )
            }
            Self::InvalidBundlePolicy { expected, observed } => {
                write!(
                    formatter,
                    "policy id mismatch: expected {expected}, observed {observed}"
                )
            }
            Self::Evaluation(detail) => write!(formatter, "policy evaluation failed: {detail}"),
            Self::FallbackUnavailable => formatter.write_str("policy fallback unavailable"),
            Self::InFlightPinConflict { run_id } => {
                write!(
                    formatter,
                    "in-flight run {run_id} has an immutable policy pin"
                )
            }
        }
    }
}

impl std::error::Error for PolicyRuntimeError {}

#[derive(Debug)]
struct RegisteredPolicy {
    pin: PolicyPin,
    bundle: PolicyBundle,
    mode: PolicyMode,
}

#[derive(Debug)]
pub struct PolicyModeFacade {
    policies: BTreeMap<PolicyPin, RegisteredPolicy>,
    active: BTreeMap<String, PolicyPin>,
    last_known_good: BTreeMap<String, PolicyPin>,
    in_flight: BTreeMap<String, PolicyPin>,
    rust_baseline_available: bool,
    receipt_sequence: u64,
    limits: Limits,
}

impl Default for PolicyModeFacade {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

impl PolicyModeFacade {
    pub fn new(limits: Limits) -> Self {
        Self {
            policies: BTreeMap::new(),
            active: BTreeMap::new(),
            last_known_good: BTreeMap::new(),
            in_flight: BTreeMap::new(),
            rust_baseline_available: true,
            receipt_sequence: 0,
            limits,
        }
    }

    pub fn set_rust_baseline_available(&mut self, available: bool) {
        self.rust_baseline_available = available;
    }

    pub fn register(
        &mut self,
        bundle: PolicyBundle,
        content_digest: impl Into<String>,
        mode: PolicyMode,
    ) -> Result<PolicyPin, PolicyRuntimeError> {
        let digest = bundle
            .digest()
            .map_err(|error| PolicyRuntimeError::Evaluation(error.to_string()))?;
        let expected = content_digest.into();
        if !SUPPORTED_POLICY_IDS.contains(&bundle.policy_id.as_str()) {
            return Err(PolicyRuntimeError::InvalidBundlePolicy {
                expected: SUPPORTED_POLICY_IDS.join(","),
                observed: bundle.policy_id,
            });
        }
        if digest != expected {
            return Err(PolicyRuntimeError::InvalidBundleDigest {
                expected,
                observed: digest,
            });
        }
        let pin = PolicyPin::new(bundle.policy_id.clone(), bundle.version, digest)?;
        if let Some(existing) = self.policies.get(&pin) {
            if existing.bundle.source != bundle.source {
                return Err(PolicyRuntimeError::InvalidBundleDigest {
                    expected: pin.content_digest.clone(),
                    observed: "conflicting-duplicate".to_string(),
                });
            }
            return Ok(pin);
        }
        if mode == PolicyMode::Active {
            return Err(PolicyRuntimeError::InvalidModeTransition {
                from: PolicyMode::Off,
                to: mode,
            });
        }
        self.policies.insert(
            pin.clone(),
            RegisteredPolicy {
                pin: pin.clone(),
                bundle,
                mode,
            },
        );
        Ok(pin)
    }

    pub fn set_mode(
        &mut self,
        pin: &PolicyPin,
        mode: PolicyMode,
    ) -> Result<(), PolicyRuntimeError> {
        let current = self
            .policies
            .get(pin)
            .ok_or_else(|| PolicyRuntimeError::BundleNotFound(pin.clone()))?
            .mode;
        if current != mode
            && !matches!(
                (current, mode),
                (PolicyMode::Off, PolicyMode::Shadow) | (PolicyMode::Shadow, PolicyMode::Active)
            )
        {
            return Err(PolicyRuntimeError::InvalidModeTransition {
                from: current,
                to: mode,
            });
        }
        self.policies
            .get_mut(pin)
            .expect("validated policy pin")
            .mode = mode;
        if mode == PolicyMode::Active {
            let previous = self.active.insert(pin.policy_id.clone(), pin.clone());
            if let Some(previous) = previous {
                if previous != *pin {
                    self.last_known_good.insert(pin.policy_id.clone(), previous);
                }
            }
        }
        Ok(())
    }

    pub fn active_pin(&self, policy_id: &str) -> Option<&PolicyPin> {
        self.active.get(policy_id)
    }

    pub fn last_known_good_pin(&self, policy_id: &str) -> Option<&PolicyPin> {
        self.last_known_good.get(policy_id)
    }

    pub fn begin_run(
        &mut self,
        run_id: impl Into<String>,
        pinned: Option<PolicyPin>,
        policy_id: &str,
    ) -> Result<PolicyPin, PolicyRuntimeError> {
        let run_id = run_id.into();
        let selected = match pinned {
            Some(pin) => {
                self.require_exact_pin(&pin)?;
                pin
            }
            None => self.select_new_run(policy_id)?,
        };
        if let Some(existing) = self.in_flight.get(&run_id) {
            if existing != &selected {
                return Err(PolicyRuntimeError::InFlightPinConflict { run_id });
            }
        } else {
            self.in_flight.insert(run_id, selected.clone());
        }
        Ok(selected)
    }

    pub fn resolve_in_flight(&self, run_id: &str) -> Result<&PolicyPin, PolicyRuntimeError> {
        self.in_flight
            .get(run_id)
            .ok_or_else(|| PolicyRuntimeError::InFlightPinConflict {
                run_id: run_id.to_string(),
            })
    }

    pub fn finish_run(&mut self, run_id: &str) {
        self.in_flight.remove(run_id);
    }

    /// Evaluate a new run for an explicit policy family. This is the entry
    /// point used when no active bundle exists and Rust must select the
    /// immutable baseline rather than silently selecting another policy.
    pub fn evaluate_new_run(
        &mut self,
        policy_id: &str,
        mut request: PolicyEvaluationRequest,
    ) -> Result<PolicyEvaluationOutcome, PolicyRuntimeError> {
        if request.pinned.is_some() || self.active.contains_key(policy_id) {
            return self.evaluate(request);
        }
        let pin = self.begin_run(request.run_id.clone(), None, policy_id)?;
        if pin.version != 0 {
            request.pinned = Some(pin);
            return self.evaluate(request);
        }
        request.rust_decision.validate()?;
        let input_digest = digest_json(&request.input)?;
        let receipt = self.receipt(
            &request,
            &pin,
            PolicyMode::Off,
            input_digest,
            None,
            0,
            None,
            None,
            None,
            Some(&FallbackTarget::RustBaseline),
            false,
        );
        Ok(PolicyEvaluationOutcome {
            decision: request.rust_decision,
            shadow_decision: None,
            fallback: Some(FallbackTarget::RustBaseline),
            receipt,
        })
    }

    pub(crate) fn evaluate_quality_gate_shadow(
        &mut self,
        request: QualityGateShadowRequest,
    ) -> Result<QualityGateShadowOutcome, PolicyRuntimeError> {
        if request.run_id.trim().is_empty() || request.bundle_id.trim().is_empty() {
            return Err(PolicyRuntimeError::InvalidDecision(
                "quality_gate_shadow_identity_invalid".to_string(),
            ));
        }
        let input = request.context.redacted_input()?;
        let baseline_request = request.context.baseline_request();
        let baseline = quality_gate_baseline_resolution(&baseline_request)?;
        let pin = self.begin_run(
            request.run_id.clone(),
            request.pinned,
            QUALITY_GATE_POLICY_ID,
        )?;
        let mode = self
            .policies
            .get(&pin)
            .map(|registered| registered.mode)
            .unwrap_or(PolicyMode::Off);
        let input_digest = digest_json(&input)?;
        let started = std::time::Instant::now();
        if mode != PolicyMode::Shadow {
            let receipt = self.quality_gate_receipt(
                &request.run_id,
                &pin,
                mode,
                input_digest,
                None,
                started.elapsed().as_millis() as u64,
                None,
                None,
                Some("shadow_mode_required"),
                Some(&FallbackTarget::RustBaseline),
            );
            return Ok(QualityGateShadowOutcome {
                decision: None,
                resolution: baseline,
                receipt,
            });
        }
        let registered = self
            .policies
            .get(&pin)
            .ok_or_else(|| PolicyRuntimeError::BundleNotFound(pin.clone()))?;
        let engine = build_policy_engine(self.limits);
        let value = match engine.evaluate(&registered.bundle.source, input) {
            Ok(value) => value,
            Err(error) => {
                let receipt = self.quality_gate_receipt(
                    &request.run_id,
                    &pin,
                    PolicyMode::Shadow,
                    input_digest,
                    None,
                    started.elapsed().as_millis() as u64,
                    None,
                    None,
                    Some(error.code().as_str()),
                    Some(&FallbackTarget::RustBaseline),
                );
                return Ok(QualityGateShadowOutcome {
                    decision: None,
                    resolution: baseline,
                    receipt,
                });
            }
        };
        let decision = QualityGateShadowDecision::from_value(&value)?;
        let resolution = quality_gate_final_profiles(&baseline_request, &value)?;
        let agreed = resolution == baseline;
        let receipt = self.quality_gate_receipt(
            &request.run_id,
            &pin,
            PolicyMode::Shadow,
            input_digest,
            Some(digest_json(&value)?),
            started.elapsed().as_millis() as u64,
            Some(agreed),
            (!agreed).then_some("quality_gate_additive_diff"),
            None,
            None,
        );
        Ok(QualityGateShadowOutcome {
            decision: Some(decision),
            resolution,
            receipt,
        })
    }

    pub fn evaluate(
        &mut self,
        request: PolicyEvaluationRequest,
    ) -> Result<PolicyEvaluationOutcome, PolicyRuntimeError> {
        request.rust_decision.validate()?;
        let pin = if let Some(pin) = request.pinned.clone() {
            self.begin_run(request.run_id.clone(), Some(pin), "")?
        } else {
            let policy_id = self
                .active
                .keys()
                .next()
                .cloned()
                .ok_or(PolicyRuntimeError::FallbackUnavailable)?;
            self.begin_run(request.run_id.clone(), None, &policy_id)?
        };
        let baseline_pin = pin.version == 0
            && pin.content_digest == POLICY_RUNTIME_BASELINE_DIGEST
            && self.rust_baseline_available;
        let mode = self
            .policies
            .get(&pin)
            .map(|registered| registered.mode)
            .unwrap_or(PolicyMode::Off);
        let input_digest = digest_json(&request.input)?;
        let started = std::time::Instant::now();
        let mut shadow_decision = None;
        let mut output_digest = None;
        let mut error_code = None;
        let mut fallback = baseline_pin.then_some(FallbackTarget::RustBaseline);
        let decision = match mode {
            PolicyMode::Off => request.rust_decision.clone(),
            PolicyMode::Shadow | PolicyMode::Active => {
                let registered = self
                    .policies
                    .get(&pin)
                    .ok_or_else(|| PolicyRuntimeError::BundleNotFound(pin.clone()))?;
                let engine = build_policy_engine(self.limits);
                match engine.evaluate(&registered.bundle.source, request.input.clone()) {
                    Ok(value) => match serde_json::from_value::<TypedPolicyDecision>(value) {
                        Ok(candidate) => {
                            candidate.validate()?;
                            output_digest = Some(candidate.canonical_digest()?);
                            if mode == PolicyMode::Shadow {
                                shadow_decision = Some(candidate);
                                request.rust_decision.clone()
                            } else {
                                candidate
                            }
                        }
                        Err(_error) => {
                            error_code = Some("typed_decision_invalid".to_string());
                            fallback = Some(self.fallback_for(&pin));
                            request.rust_decision.clone()
                        }
                    },
                    Err(error) => {
                        error_code = Some(error.code().as_str().to_string());
                        fallback = Some(self.fallback_for(&pin));
                        request.rust_decision.clone()
                    }
                }
            }
        };
        let agreed = shadow_decision
            .as_ref()
            .map(|candidate| candidate == &request.rust_decision);
        let receipt = self.receipt(
            &request,
            &pin,
            mode,
            input_digest,
            output_digest,
            started.elapsed().as_millis() as u64,
            agreed,
            if agreed == Some(false) {
                Some("shadow_divergence".to_string())
            } else {
                None
            },
            error_code,
            fallback.as_ref(),
            mode == PolicyMode::Active && fallback.is_none(),
        );
        Ok(PolicyEvaluationOutcome {
            decision,
            shadow_decision,
            fallback,
            receipt,
        })
    }

    fn require_exact_pin(&self, pin: &PolicyPin) -> Result<(), PolicyRuntimeError> {
        if pin.version == 0
            && pin.content_digest == POLICY_RUNTIME_BASELINE_DIGEST
            && self.rust_baseline_available
        {
            return Ok(());
        }
        let Some(registered) = self.policies.get(pin) else {
            return Err(PolicyRuntimeError::PinnedBundleMissing(pin.clone()));
        };
        let observed = registered
            .bundle
            .digest()
            .map_err(|error| PolicyRuntimeError::Evaluation(error.to_string()))?;
        if observed != pin.content_digest {
            return Err(PolicyRuntimeError::InvalidBundleDigest {
                expected: pin.content_digest.clone(),
                observed,
            });
        }
        Ok(())
    }

    fn select_new_run(&self, policy_id: &str) -> Result<PolicyPin, PolicyRuntimeError> {
        if let Some(pin) = self.active.get(policy_id) {
            if self.require_exact_pin(pin).is_ok() {
                return Ok(pin.clone());
            }
        }
        if let Some(pin) = self.last_known_good.get(policy_id) {
            if self.require_exact_pin(pin).is_ok() {
                return Ok(pin.clone());
            }
        }
        if self.rust_baseline_available {
            return PolicyPin::new(policy_id, 0, POLICY_RUNTIME_BASELINE_DIGEST);
        }
        Err(PolicyRuntimeError::FallbackUnavailable)
    }

    fn fallback_for(&self, pin: &PolicyPin) -> FallbackTarget {
        if self
            .last_known_good
            .get(&pin.policy_id)
            .is_some_and(|candidate| self.require_exact_pin(candidate).is_ok())
        {
            return FallbackTarget::LastKnownGood;
        }
        if self.rust_baseline_available {
            return FallbackTarget::RustBaseline;
        }
        FallbackTarget::Block
    }

    fn receipt(
        &mut self,
        request: &PolicyEvaluationRequest,
        pin: &PolicyPin,
        mode: PolicyMode,
        input_digest: String,
        output_digest: Option<String>,
        duration_ms: u64,
        agreed: Option<bool>,
        diff_code: Option<String>,
        error_code: Option<String>,
        fallback: Option<&FallbackTarget>,
        authorizes_effects: bool,
    ) -> PolicyReceipt {
        self.receipt_sequence = self.receipt_sequence.saturating_add(1);
        PolicyReceipt {
            schema_version: POLICY_RUNTIME_SCHEMA_VERSION,
            receipt_id: format!("policy-receipt-{}", self.receipt_sequence),
            run_id: request.run_id.clone(),
            policy_id: pin.policy_id.clone(),
            version: pin.version,
            content_digest: pin.content_digest.clone(),
            mode,
            input_digest,
            output_digest,
            duration_ms,
            agreed,
            diff_code,
            error_code,
            fallback_code: fallback.map(|target| {
                match target {
                    FallbackTarget::LastKnownGood => "last_known_good",
                    FallbackTarget::RustBaseline => "rust_baseline",
                    FallbackTarget::Block => "block",
                }
                .to_string()
            }),
            authorizes_effects,
        }
    }

    fn quality_gate_receipt(
        &mut self,
        run_id: &str,
        pin: &PolicyPin,
        mode: PolicyMode,
        input_digest: String,
        output_digest: Option<String>,
        duration_ms: u64,
        agreed: Option<bool>,
        diff_code: Option<&str>,
        error_code: Option<&str>,
        fallback: Option<&FallbackTarget>,
    ) -> PolicyReceipt {
        self.receipt_sequence = self.receipt_sequence.saturating_add(1);
        PolicyReceipt {
            schema_version: POLICY_RUNTIME_SCHEMA_VERSION,
            receipt_id: format!("policy-receipt-{}", self.receipt_sequence),
            run_id: run_id.to_string(),
            policy_id: pin.policy_id.clone(),
            version: pin.version,
            content_digest: pin.content_digest.clone(),
            mode,
            input_digest,
            output_digest,
            duration_ms,
            agreed,
            diff_code: diff_code.map(str::to_string),
            error_code: error_code.map(str::to_string),
            fallback_code: fallback.map(|target| {
                match target {
                    FallbackTarget::LastKnownGood => "last_known_good",
                    FallbackTarget::RustBaseline => "rust_baseline",
                    FallbackTarget::Block => "block",
                }
                .to_string()
            }),
            authorizes_effects: false,
        }
    }
}

fn digest_json(value: &Value) -> Result<String, PolicyRuntimeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PolicyRuntimeError::Evaluation(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vida_policy_rhai::bundle::POLICY_ENGINE_ABI;

    fn bundle(policy_id: &str, version: u32, source: &str) -> PolicyBundle {
        PolicyBundle::from_json(
            &serde_json::json!({
                "schema": 1,
                "policy_id": policy_id,
                "version": version,
                "engine_abi": POLICY_ENGINE_ABI,
                "source": source,
            })
            .to_string(),
        )
        .unwrap()
    }

    fn decision() -> TypedPolicyDecision {
        TypedPolicyDecision {
            schema_version: 1,
            allowed: true,
            score: 100,
            recommendation: "no_change".to_string(),
            additive_profiles: Vec::new(),
            blockers: Vec::new(),
            evidence_refs: Vec::new(),
        }
    }

    fn quality_gate_request() -> QualityGateBaselineRequest {
        QualityGateBaselineRequest {
            schema_version: 1,
            hard_profiles: vec!["security".to_string()],
            config_profiles: vec!["visual".to_string()],
            task_profiles: vec!["contract".to_string()],
            path_profiles: vec!["a11y".to_string()],
            explicit_profiles: vec!["visual".to_string()],
            explicit_check_ids: vec!["contract".to_string()],
            limits: BTreeMap::from([(String::from("max_profiles"), 8)]),
        }
    }

    fn quality_gate_context(verdict: &str) -> QualityGateContextV1 {
        let request = quality_gate_request();
        QualityGateContextV1 {
            schema_version: request.schema_version,
            baseline_verdict: verdict.to_string(),
            hard_profiles: request.hard_profiles,
            config_profiles: request.config_profiles,
            task_profiles: request.task_profiles,
            path_profiles: request.path_profiles,
            explicit_profiles: request.explicit_profiles,
            explicit_check_ids: request.explicit_check_ids,
            limits: request.limits,
        }
    }

    fn quality_gate_shadow(source: &str, run_id: &str) -> QualityGateShadowOutcome {
        let mut facade = PolicyModeFacade::default();
        let policy = bundle(QUALITY_GATE_POLICY_ID, 1, source);
        let digest = policy.digest().unwrap();
        let pin = facade.register(policy, digest, PolicyMode::Off).unwrap();
        facade.set_mode(&pin, PolicyMode::Shadow).unwrap();
        facade
            .evaluate_quality_gate_shadow(QualityGateShadowRequest {
                run_id: run_id.to_string(),
                bundle_id: "bundle-quality-gate".to_string(),
                context: quality_gate_context("pass"),
                pinned: Some(pin),
            })
            .unwrap()
    }

    #[test]
    fn quality_gate_baseline_unions_sources_and_retains_hard_profiles() {
        let request = quality_gate_request();
        assert_eq!(
            quality_gate_baseline_profiles(&request).unwrap(),
            vec!["contract", "security", "a11y", "visual", "observability"]
        );
        let resolution = quality_gate_baseline_resolution(&request).unwrap();
        assert_eq!(resolution.effective_profiles, resolution.baseline_profiles);
        assert_eq!(resolution.check_ids, vec!["contract"]);
    }

    #[test]
    fn quality_gate_final_profiles_accepts_only_additive_rhai_changes() {
        let request = quality_gate_request();
        let output = serde_json::json!({
            "schema_version": 1,
            "additive_profiles": ["resilience"],
            "check_ids": ["property"]
        });
        let resolution = quality_gate_final_profiles(&request, &output).unwrap();
        assert_eq!(
            resolution.effective_profiles,
            vec![
                "contract",
                "security",
                "a11y",
                "visual",
                "resilience",
                "observability"
            ]
        );
        assert_eq!(resolution.check_ids, vec!["contract", "property"]);
        assert_eq!(resolution.limits["max_profiles"], 8);
    }

    #[test]
    fn quality_gate_rejects_unknown_schema_fields_and_limits() {
        let request = quality_gate_request();
        for output in [
            serde_json::json!({"schema_version": 2}),
            serde_json::json!({"schema_version": 1, "remove_profiles": ["security"]}),
            serde_json::json!({"schema_version": 1, "additive_profiles": ["unknown"]}),
        ] {
            assert!(matches!(
                quality_gate_final_profiles(&request, &output),
                Err(PolicyRuntimeError::InvalidDecision(_))
            ));
        }

        let mut oversized = quality_gate_request();
        oversized.task_profiles = vec!["contract".to_string(); QUALITY_GATE_MAX_ITEMS + 1];
        assert!(matches!(
            quality_gate_baseline_profiles(&oversized),
            Err(PolicyRuntimeError::InvalidDecision(_))
        ));
        oversized = quality_gate_request();
        oversized.limits.insert(
            "max_context_bytes".to_string(),
            QUALITY_GATE_MAX_CONTEXT_BYTES + 1,
        );
        assert!(matches!(
            quality_gate_baseline_profiles(&oversized),
            Err(PolicyRuntimeError::InvalidDecision(_))
        ));
    }

    #[test]
    fn quality_gate_shadow_records_agreement_and_additive_diff_without_authority() {
        let source = r#"#{schema_version: 1, additive_profiles: [], check_ids: ["contract"], rationale: "baseline", risk: "low"}"#;
        let agreed = quality_gate_shadow(source, "quality-agreement");
        assert_eq!(agreed.receipt.agreed, Some(true));
        assert!(!agreed.receipt.authorizes_effects);
        assert!(agreed.receipt.output_digest.is_some());

        let source = r#"#{schema_version: 1, additive_profiles: ["resilience"], check_ids: ["contract"], rationale: "add resilience", risk: "medium"}"#;
        let diff = quality_gate_shadow(source, "quality-diff");
        assert_eq!(diff.receipt.agreed, Some(false));
        assert_eq!(
            diff.receipt.diff_code.as_deref(),
            Some("quality_gate_additive_diff")
        );
        assert_eq!(
            diff.resolution
                .effective_profiles
                .last()
                .map(String::as_str),
            Some("observability")
        );
        assert!(!diff.receipt.authorizes_effects);
    }

    #[test]
    fn quality_gate_shadow_rejects_subtraction_unknown_ids_and_schema() {
        for source in [
            r#"#{schema_version: 1, additive_profiles: [], check_ids: ["contract"], rationale: "x", risk: "low", remove_profiles: ["security"]}"#,
            r#"#{schema_version: 1, additive_profiles: ["unknown"], check_ids: ["contract"], rationale: "x", risk: "low"}"#,
            r#"#{schema_version: 2, additive_profiles: [], check_ids: ["contract"], rationale: "x", risk: "low"}"#,
        ] {
            let mut facade = PolicyModeFacade::default();
            let policy = bundle(QUALITY_GATE_POLICY_ID, 1, source);
            let digest = policy.digest().unwrap();
            let pin = facade.register(policy, digest, PolicyMode::Off).unwrap();
            facade.set_mode(&pin, PolicyMode::Shadow).unwrap();
            assert!(matches!(
                facade.evaluate_quality_gate_shadow(QualityGateShadowRequest {
                    run_id: format!("quality-invalid-{}", facade.receipt_sequence),
                    bundle_id: "bundle-quality-gate".to_string(),
                    context: quality_gate_context("pass"),
                    pinned: Some(pin),
                }),
                Err(PolicyRuntimeError::InvalidDecision(_))
            ));
        }
    }

    #[test]
    fn quality_gate_shadow_falls_back_on_evaluator_error_and_receipt_is_redacted() {
        let outcome = quality_gate_shadow(r#"eval("forbidden")"#, "quality-error");
        assert_eq!(
            outcome.receipt.fallback_code.as_deref(),
            Some("rust_baseline")
        );
        assert!(outcome.receipt.error_code.is_some());
        assert!(!outcome.receipt.authorizes_effects);
        assert!(outcome.receipt.is_redacted());
        let serialized = serde_json::to_string(&outcome.receipt).unwrap();
        assert!(!serialized.contains("baseline_verdict"));
        assert!(!serialized.contains("forbidden"));
        let shadow = outcome.shadow_receipt("bundle-quality-gate");
        assert_eq!(shadow.policy_id, QUALITY_GATE_POLICY_ID);
        assert!(!serde_json::to_string(&shadow)
            .unwrap()
            .contains("baseline_verdict"));
    }

    #[test]
    fn off_shadow_active_are_explicit_and_shadow_never_authorizes() {
        let mut facade = PolicyModeFacade::default();
        let policy = bundle(
            "rhai.runtime.authority",
            1,
            r#"#{schema_version: 1, allowed: false, score: 0, recommendation: "block"}"#,
        );
        let digest = policy.digest().unwrap();
        let pin = facade.register(policy, digest, PolicyMode::Off).unwrap();
        facade.set_mode(&pin, PolicyMode::Shadow).unwrap();
        let shadow = facade
            .evaluate(PolicyEvaluationRequest {
                run_id: "run-shadow".to_string(),
                input: serde_json::json!({"claim":"x"}),
                rust_decision: decision(),
                pinned: Some(pin.clone()),
            })
            .unwrap();
        assert!(!shadow.receipt.authorizes_effects);
        assert_eq!(shadow.fallback, None);
        assert_eq!(shadow.shadow_decision.unwrap().allowed, false);
        facade.set_mode(&pin, PolicyMode::Active).unwrap();
        let active = facade
            .evaluate(PolicyEvaluationRequest {
                run_id: "run-active".to_string(),
                input: serde_json::json!({"claim":"x"}),
                rust_decision: decision(),
                pinned: Some(pin),
            })
            .unwrap();
        assert!(active.receipt.authorizes_effects);
        assert_eq!(active.decision.allowed, false);
    }

    #[test]
    fn missing_pinned_bundle_blocks_while_new_run_can_use_baseline() {
        let mut facade = PolicyModeFacade::default();
        let missing = PolicyPin::new("rhai.runtime.authority", 8, "missing").unwrap();
        assert!(matches!(
            facade.begin_run("pinned", Some(missing), "rhai.runtime.authority"),
            Err(PolicyRuntimeError::PinnedBundleMissing(_))
        ));
        facade.set_rust_baseline_available(true);
        let selected = facade
            .begin_run("new", None, "rhai.runtime.authority")
            .unwrap();
        assert_eq!(selected.version, 0);
        assert_eq!(selected.content_digest, POLICY_RUNTIME_BASELINE_DIGEST);
        let outcome = facade
            .evaluate_new_run(
                "rhai.runtime.lifecycle",
                PolicyEvaluationRequest {
                    run_id: "new-evaluated".to_string(),
                    input: serde_json::json!({"state":"candidate"}),
                    rust_decision: decision(),
                    pinned: None,
                },
            )
            .unwrap();
        assert_eq!(outcome.fallback, Some(FallbackTarget::RustBaseline));
        assert!(!outcome.receipt.authorizes_effects);
    }

    #[test]
    fn typed_boundary_rejects_unknown_recommendations_and_oversized_pin_fields() {
        let mut invalid = decision();
        invalid.recommendation = "grant_capability".to_string();
        assert!(matches!(
            invalid.validate(),
            Err(PolicyRuntimeError::InvalidDecision(_))
        ));
        assert!(matches!(
            PolicyPin::new("x".repeat(129), 1, "digest"),
            Err(PolicyRuntimeError::InvalidPin)
        ));
        let mut facade = PolicyModeFacade::default();
        let unknown = bundle(
            "rhai.runtime.unknown",
            1,
            "#{schema_version: 1, allowed: true, score: 1, recommendation: \"no_change\"}",
        );
        let digest = unknown.digest().unwrap();
        assert!(matches!(
            facade.register(unknown, digest, PolicyMode::Off),
            Err(PolicyRuntimeError::InvalidBundlePolicy { .. })
        ));
    }

    #[test]
    fn receipts_are_digest_only_and_in_flight_pins_are_immutable() {
        let mut facade = PolicyModeFacade::default();
        let policy = bundle(
            "rhai.runtime.authority",
            1,
            r#"#{schema_version: 1, allowed: true, score: 100, recommendation: "no_change"}"#,
        );
        let digest = policy.digest().unwrap();
        let pin = facade.register(policy, digest, PolicyMode::Shadow).unwrap();
        let selected = facade
            .begin_run("run", Some(pin.clone()), "ignored")
            .unwrap();
        assert_eq!(selected, pin);
        assert!(matches!(
            facade.begin_run(
                "run",
                Some(PolicyPin::new("rhai.runtime.authority", 2, "other").unwrap()),
                "ignored"
            ),
            Err(PolicyRuntimeError::PinnedBundleMissing(_))
        ));
        let outcome = facade
            .evaluate(PolicyEvaluationRequest {
                run_id: "run".to_string(),
                input: serde_json::json!({"secret":"never persist"}),
                rust_decision: decision(),
                pinned: Some(pin),
            })
            .unwrap();
        assert!(outcome.receipt.is_redacted());
        assert!(!serde_json::to_string(&outcome.receipt)
            .unwrap()
            .contains("never persist"));
    }

    #[test]
    fn evaluator_failure_without_lkg_or_baseline_returns_block_receipt() {
        let mut facade = PolicyModeFacade::default();
        facade.set_rust_baseline_available(false);
        let policy = bundle("rhai.runtime.authority", 1, r#"eval("forbidden")"#);
        let digest = policy.digest().unwrap();
        let pin = facade.register(policy, digest, PolicyMode::Shadow).unwrap();
        facade.set_mode(&pin, PolicyMode::Active).unwrap();
        let outcome = facade
            .evaluate(PolicyEvaluationRequest {
                run_id: "run-blocked".to_string(),
                input: serde_json::json!({"claim":"x"}),
                rust_decision: decision(),
                pinned: Some(pin),
            })
            .unwrap();
        assert_eq!(outcome.fallback, Some(FallbackTarget::Block));
        assert!(!outcome.receipt.authorizes_effects);
        assert_eq!(outcome.receipt.fallback_code.as_deref(), Some("block"));
    }
}
