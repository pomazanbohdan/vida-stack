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
const SUPPORTED_POLICY_IDS: [&str; 7] = [
    "rhai.runtime.authority",
    "rhai.runtime.lifecycle",
    "rhai.runtime.failover",
    "rhai.runtime.promotion",
    "rhai.runtime.rollback",
    "rhai.runtime.pinned-resume",
    "rhai.runtime.quality-gate",
];

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
