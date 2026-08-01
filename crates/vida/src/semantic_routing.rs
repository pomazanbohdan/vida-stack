//! Standalone semantic-routing shadow contract.
//!
//! This module deliberately has no runtime authority.  The native router owns
//! tokenization, redaction, catalog lookup, and hard candidate constraints;
//! this file only projects bounded facts and classifies shadow differences.

use std::fmt;

pub const FACT_SCHEMA_VERSION: u16 = 1;
pub const POLICY_ID: &str = "rhai.runtime.semantic-routing";
pub const POLICY_VERSION: u32 = 1;
pub const MAX_TOKEN_COUNT: usize = 64;
pub const MAX_REDACTED_TOKEN_COUNT: usize = MAX_TOKEN_COUNT;
pub const MAX_HARD_CONSTRAINTS: usize = 8;
pub const VIDA_STACK_REPOSITORY_ALLOWLIST: &str = "pomazanbohdan/vida-stack";
pub const MAX_REPOSITORY_ID_CHARS: usize = 128;

/// The exact Rhai source staged beside this module.
pub const SHADOW_POLICY_SOURCE: &str =
    include_str!("../../../vida/policies/builtin/v1/policies/vida.semantic-routing.v1");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoutingDomain {
    General,
    Architecture,
    Security,
    Legal,
    Medical,
    Devops,
    Data,
    Implementation,
    Release,
}

impl RoutingDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Architecture => "architecture",
            Self::Security => "security",
            Self::Legal => "legal",
            Self::Medical => "medical",
            Self::Devops => "devops",
            Self::Data => "data",
            Self::Implementation => "implementation",
            Self::Release => "release",
        }
    }

    fn from_native(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "architecture" => Self::Architecture,
            "security" => Self::Security,
            "legal" => Self::Legal,
            "medical" => Self::Medical,
            "devops" => Self::Devops,
            "data" => Self::Data,
            "implementation" => Self::Implementation,
            "release" => Self::Release,
            _ => Self::General,
        }
    }
}

impl fmt::Display for RoutingDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplexityBand {
    Simple,
    Medium,
    MediumHigh,
    Complex,
}

impl ComplexityBand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Medium => "medium",
            Self::MediumHigh => "medium_high",
            Self::Complex => "complex",
        }
    }

    fn from_score(score: u8) -> Self {
        match score {
            0..=24 => Self::Simple,
            25..=49 => Self::Medium,
            50..=74 => Self::MediumHigh,
            _ => Self::Complex,
        }
    }
}

impl fmt::Display for ComplexityBand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingConstraint {
    PrivacyReview,
    QualityFloorHigh,
    VerificationRequired,
}

impl RoutingConstraint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrivacyReview => "privacy_review",
            Self::QualityFloorHigh => "quality_floor_high",
            Self::VerificationRequired => "verification_required",
        }
    }

    fn from_native(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "privacy_review" => Some(Self::PrivacyReview),
            "quality_floor_high" => Some(Self::QualityFloorHigh),
            "verification_required" => Some(Self::VerificationRequired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdvisoryWeights {
    pub cost: u8,
    pub speed: u8,
    pub quality: u8,
    pub reasoning: u8,
    pub reliability: u8,
}

impl AdvisoryWeights {
    pub const fn zero() -> Self {
        Self {
            cost: 0,
            speed: 0,
            quality: 0,
            reasoning: 0,
            reliability: 0,
        }
    }

    pub const fn for_band(band: ComplexityBand) -> Self {
        match band {
            ComplexityBand::Simple => Self {
                cost: 45,
                speed: 20,
                quality: 20,
                reasoning: 5,
                reliability: 10,
            },
            ComplexityBand::Medium => Self {
                cost: 25,
                speed: 15,
                quality: 30,
                reasoning: 15,
                reliability: 15,
            },
            ComplexityBand::MediumHigh | ComplexityBand::Complex => Self {
                cost: 15,
                speed: 10,
                quality: 35,
                reasoning: 25,
                reliability: 15,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRoutingObservation {
    pub detected_domain: String,
    pub domain_score_percent: u16,
    pub complexity_score_percent: u16,
    pub token_count: usize,
    pub redacted_token_count: usize,
    pub hard_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticRoutingFacts {
    pub schema_version: u16,
    pub domain: RoutingDomain,
    pub domain_score_percent: u8,
    pub complexity_score_percent: u8,
    pub complexity_band: ComplexityBand,
    pub token_count: u8,
    pub redacted_token_count: u8,
    pub advisory_weights: AdvisoryWeights,
    pub hard_constraints: Vec<RoutingConstraint>,
}

impl SemanticRoutingFacts {
    pub fn from_native(observation: NativeRoutingObservation) -> Self {
        let complexity_score_percent = observation.complexity_score_percent.min(100) as u8;
        let hard_constraints = observation
            .hard_constraints
            .iter()
            .filter_map(|value| RoutingConstraint::from_native(value))
            .take(MAX_HARD_CONSTRAINTS)
            .collect::<Vec<_>>();
        let mut hard_constraints = hard_constraints;
        hard_constraints.sort_unstable_by_key(|constraint| constraint.as_str());
        hard_constraints.dedup();
        let complexity_band = ComplexityBand::from_score(complexity_score_percent);
        Self {
            schema_version: FACT_SCHEMA_VERSION,
            domain: RoutingDomain::from_native(&observation.detected_domain),
            domain_score_percent: observation.domain_score_percent.min(100) as u8,
            complexity_score_percent,
            complexity_band,
            token_count: observation.token_count.min(MAX_TOKEN_COUNT) as u8,
            redacted_token_count: observation
                .redacted_token_count
                .min(MAX_REDACTED_TOKEN_COUNT) as u8,
            advisory_weights: AdvisoryWeights::for_band(complexity_band),
            hard_constraints,
        }
    }

    pub fn context_literal(&self) -> String {
        let constraints = self
            .hard_constraints
            .iter()
            .map(|constraint| format!("\"{}\"", constraint.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "#{{schema_version: {}, domain: \"{}\", domain_score_percent: {}, complexity_score_percent: {}, complexity_band: \"{}\", token_count: {}, redacted_token_count: {}, hard_constraints: [ {} ]}}",
            self.schema_version,
            self.domain,
            self.domain_score_percent,
            self.complexity_score_percent,
            self.complexity_band,
            self.token_count,
            self.redacted_token_count,
            constraints,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingDecision {
    pub domain: RoutingDomain,
    pub complexity_band: ComplexityBand,
    pub advisory_weights: AdvisoryWeights,
    pub recommendation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffClass {
    ExactParity,
    DomainDifference,
    ComplexityDifference,
    AdvisoryWeightsDifference,
    RecommendationDifference,
    MultipleDifferences,
}

impl DiffClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactParity => "exact_parity",
            Self::DomainDifference => "domain_difference",
            Self::ComplexityDifference => "complexity_difference",
            Self::AdvisoryWeightsDifference => "advisory_weights_difference",
            Self::RecommendationDifference => "recommendation_difference",
            Self::MultipleDifferences => "multiple_differences",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReceipt {
    pub case_id: usize,
    pub facts_digest: String,
    pub diff_class: DiffClass,
    pub native_effective: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRoutingPath {
    VidaStackCanary,
    NativeFallback,
}

impl ActiveRoutingPath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VidaStackCanary => "vida_stack_canary",
            Self::NativeFallback => "native_fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    RepositoryNotAllowlisted,
    RuntimeUncertain,
}

impl FallbackReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RepositoryNotAllowlisted => "repository_not_allowlisted",
            Self::RuntimeUncertain => "runtime_uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveRoutingDecision {
    pub decision: RoutingDecision,
    pub path: ActiveRoutingPath,
    pub fallback_reason: Option<FallbackReason>,
    pub native_effective: bool,
    pub advisory_adjustment_percent: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveReplayReceipt {
    pub case_id: usize,
    pub facts_digest: String,
    pub decision_digest: String,
    pub path: ActiveRoutingPath,
    pub fallback_reason: Option<FallbackReason>,
    pub native_effective: bool,
}

/// Normalize only common GitHub repository spellings before applying the
/// exact VIDA repository allowlist. Unknown hosts and paths remain rejected.
pub fn repository_is_allowlisted(repository: &str) -> bool {
    let mut normalized = repository.trim().to_ascii_lowercase();
    if normalized.len() > MAX_REPOSITORY_ID_CHARS {
        return false;
    }
    for prefix in [
        "https://github.com/",
        "http://github.com/",
        "github.com/",
        "git@github.com:",
    ] {
        if let Some(value) = normalized.strip_prefix(prefix) {
            normalized = value.to_string();
            break;
        }
    }
    while normalized.starts_with('/') {
        normalized.remove(0);
    }
    while normalized.ends_with('/') {
        normalized.pop();
    }
    if normalized.ends_with(".git") {
        normalized.truncate(normalized.len() - 4);
    }
    normalized == VIDA_STACK_REPOSITORY_ALLOWLIST
}

/// Native authority fallback: preserve native facts but apply no advisory
/// score or recommendation adjustment when canary activation is unsafe.
pub fn native_fallback_decision(facts: &SemanticRoutingFacts) -> RoutingDecision {
    RoutingDecision {
        domain: facts.domain,
        complexity_band: facts.complexity_band,
        advisory_weights: AdvisoryWeights::zero(),
        recommendation: "native_authority",
    }
}

/// Active native decision surface. It intentionally has no scoring formula;
/// the native router remains authoritative without advisory adjustment.
pub fn native_decision(facts: &SemanticRoutingFacts) -> RoutingDecision {
    native_fallback_decision(facts)
}

/// Legacy native formula retained only as a shadow-replay comparator.
fn native_shadow_reference_decision(facts: &SemanticRoutingFacts) -> RoutingDecision {
    let high_risk = facts.domain == RoutingDomain::Security
        || facts
            .hard_constraints
            .contains(&RoutingConstraint::QualityFloorHigh)
        || facts
            .hard_constraints
            .contains(&RoutingConstraint::VerificationRequired);
    let complex = matches!(
        facts.complexity_band,
        ComplexityBand::MediumHigh | ComplexityBand::Complex
    );
    RoutingDecision {
        domain: facts.domain,
        complexity_band: facts.complexity_band,
        advisory_weights: facts.advisory_weights,
        recommendation: if high_risk || complex {
            "native_candidate_constraints"
        } else {
            "balanced_cost_quality"
        },
    }
}

pub fn shadow_decision(facts: &SemanticRoutingFacts) -> RoutingDecision {
    let high_risk = facts.domain == RoutingDomain::Security
        || facts
            .hard_constraints
            .contains(&RoutingConstraint::QualityFloorHigh)
        || facts
            .hard_constraints
            .contains(&RoutingConstraint::VerificationRequired);
    let complex = matches!(
        facts.complexity_band,
        ComplexityBand::MediumHigh | ComplexityBand::Complex
    );
    RoutingDecision {
        domain: facts.domain,
        complexity_band: facts.complexity_band,
        advisory_weights: facts.advisory_weights,
        recommendation: if high_risk || complex {
            "quality_first"
        } else {
            "balanced_cost_quality"
        },
    }
}

/// Select the only active canary path. The Rhai-equivalent advisory decision
/// is admitted solely for the canonical VIDA repository and certain runtime;
/// every other case stays under native authority with zero adjustment.
pub fn active_canary_decision(
    facts: &SemanticRoutingFacts,
    repository: &str,
    runtime_certain: bool,
) -> ActiveRoutingDecision {
    let fallback_reason = if !runtime_certain {
        Some(FallbackReason::RuntimeUncertain)
    } else if !repository_is_allowlisted(repository) {
        Some(FallbackReason::RepositoryNotAllowlisted)
    } else {
        None
    };
    match fallback_reason {
        Some(reason) => ActiveRoutingDecision {
            decision: native_fallback_decision(facts),
            path: ActiveRoutingPath::NativeFallback,
            fallback_reason: Some(reason),
            native_effective: true,
            advisory_adjustment_percent: 0,
        },
        None => ActiveRoutingDecision {
            decision: shadow_decision(facts),
            path: ActiveRoutingPath::VidaStackCanary,
            fallback_reason: None,
            native_effective: true,
            advisory_adjustment_percent: 0,
        },
    }
}

pub fn active_decision_digest(
    facts: &SemanticRoutingFacts,
    repository: &str,
    runtime_certain: bool,
) -> String {
    let active = active_canary_decision(facts, repository, runtime_certain);
    let decision = active.decision;
    stable_digest_fields(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        stable_digest(facts),
        active.path.as_str(),
        active
            .fallback_reason
            .map(FallbackReason::as_str)
            .unwrap_or(""),
        active.native_effective,
        active.advisory_adjustment_percent,
        decision.domain,
        decision.complexity_band,
        decision.advisory_weights.cost,
        decision.advisory_weights.speed,
        decision.advisory_weights.quality,
        decision.advisory_weights.reasoning,
        decision.advisory_weights.reliability,
        decision.recommendation,
    ))
}

pub fn replay_active_canary_cases(
    cases: &[NativeRoutingObservation],
    repository: &str,
    runtime_certain: bool,
) -> Vec<ActiveReplayReceipt> {
    cases
        .iter()
        .enumerate()
        .map(|(case_id, observation)| {
            let facts = SemanticRoutingFacts::from_native(observation.clone());
            let active = active_canary_decision(&facts, repository, runtime_certain);
            ActiveReplayReceipt {
                case_id,
                facts_digest: stable_digest(&facts),
                decision_digest: active_decision_digest(&facts, repository, runtime_certain),
                path: active.path,
                fallback_reason: active.fallback_reason,
                native_effective: active.native_effective,
            }
        })
        .collect()
}

pub fn classify_diff(native: RoutingDecision, shadow: RoutingDecision) -> DiffClass {
    let domain = native.domain != shadow.domain;
    let complexity = native.complexity_band != shadow.complexity_band;
    let weights = native.advisory_weights != shadow.advisory_weights;
    let recommendation = native.recommendation != shadow.recommendation;
    match [domain, complexity, weights, recommendation]
        .into_iter()
        .filter(|changed| *changed)
        .count()
    {
        0 => DiffClass::ExactParity,
        1 if domain => DiffClass::DomainDifference,
        1 if complexity => DiffClass::ComplexityDifference,
        1 if weights => DiffClass::AdvisoryWeightsDifference,
        1 => DiffClass::RecommendationDifference,
        _ => DiffClass::MultipleDifferences,
    }
}

pub fn replay_parity_cases(cases: &[NativeRoutingObservation]) -> Vec<ReplayReceipt> {
    cases
        .iter()
        .enumerate()
        .map(|(case_id, observation)| {
            let facts = SemanticRoutingFacts::from_native(observation.clone());
            let native = native_shadow_reference_decision(&facts);
            let shadow = shadow_decision(&facts);
            ReplayReceipt {
                case_id,
                facts_digest: stable_digest(&facts),
                diff_class: classify_diff(native, shadow),
                native_effective: true,
            }
        })
        .collect()
}

fn stable_digest(facts: &SemanticRoutingFacts) -> String {
    let bytes = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        facts.schema_version,
        facts.domain,
        facts.domain_score_percent,
        facts.complexity_score_percent,
        facts.complexity_band,
        facts.token_count,
        facts.redacted_token_count,
        facts
            .hard_constraints
            .iter()
            .map(|constraint| constraint.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    stable_digest_fields(&bytes)
}

fn stable_digest_fields(bytes: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(index: usize) -> NativeRoutingObservation {
        let domains = [
            "general",
            "architecture",
            "security",
            "legal",
            "medical",
            "devops",
            "data",
            "implementation",
            "release",
            "unknown-domain",
        ];
        let mut hard_constraints = Vec::new();
        if index % 3 == 0 {
            hard_constraints.push("verification_required".to_string());
        }
        if index % 5 == 0 {
            hard_constraints.push("quality_floor_high".to_string());
        }
        if index % 7 == 0 {
            hard_constraints.push("privacy_review".to_string());
        }
        hard_constraints.push("untrusted-ignored-fact".to_string());
        NativeRoutingObservation {
            detected_domain: domains[index % domains.len()].to_string(),
            domain_score_percent: ((index * 17) % 140) as u16,
            complexity_score_percent: ((index * 13) % 125) as u16,
            token_count: (index * 11) % 96,
            redacted_token_count: (index * 7) % 96,
            hard_constraints,
        }
    }

    #[test]
    fn projection_is_bounded_typed_and_redacts_unknown_constraints() {
        let facts = SemanticRoutingFacts::from_native(NativeRoutingObservation {
            detected_domain: "SECURITY".to_string(),
            domain_score_percent: 999,
            complexity_score_percent: 999,
            token_count: 999,
            redacted_token_count: 999,
            hard_constraints: vec![
                "privacy_review".to_string(),
                "verification_required".to_string(),
                "untrusted".to_string(),
            ],
        });
        assert_eq!(facts.domain, RoutingDomain::Security);
        assert_eq!(facts.domain_score_percent, 100);
        assert_eq!(facts.complexity_score_percent, 100);
        assert_eq!(facts.complexity_band, ComplexityBand::Complex);
        assert_eq!(facts.token_count, MAX_TOKEN_COUNT as u8);
        assert_eq!(facts.redacted_token_count, MAX_REDACTED_TOKEN_COUNT as u8);
        assert_eq!(facts.hard_constraints.len(), 2);
        assert!(!facts.context_literal().contains("untrusted"));
    }

    #[test]
    fn shadow_is_advisory_and_diff_is_always_classified() {
        let facts = SemanticRoutingFacts::from_native(observation(3));
        let shadow = shadow_decision(&facts);
        assert_eq!(shadow.domain, facts.domain);
        assert_eq!(shadow.complexity_band, facts.complexity_band);
        assert_eq!(
            classify_diff(native_shadow_reference_decision(&facts), shadow),
            DiffClass::RecommendationDifference
        );
        assert_eq!(
            SHADOW_POLICY_SOURCE.contains("mode: \"active_canary\""),
            true
        );
    }

    #[test]
    fn classifier_names_each_single_field_drift() {
        let base = RoutingDecision {
            domain: RoutingDomain::General,
            complexity_band: ComplexityBand::Simple,
            advisory_weights: AdvisoryWeights::for_band(ComplexityBand::Simple),
            recommendation: "balanced_cost_quality",
        };
        assert_eq!(classify_diff(base, base), DiffClass::ExactParity);
        assert_eq!(
            classify_diff(
                base,
                RoutingDecision {
                    domain: RoutingDomain::Security,
                    ..base
                }
            ),
            DiffClass::DomainDifference
        );
        assert_eq!(
            classify_diff(
                base,
                RoutingDecision {
                    complexity_band: ComplexityBand::Complex,
                    ..base
                }
            ),
            DiffClass::ComplexityDifference
        );
        assert_eq!(
            classify_diff(
                base,
                RoutingDecision {
                    advisory_weights: AdvisoryWeights {
                        cost: 1,
                        ..base.advisory_weights
                    },
                    ..base
                }
            ),
            DiffClass::AdvisoryWeightsDifference
        );
        assert_eq!(
            classify_diff(
                base,
                RoutingDecision {
                    recommendation: "quality_first",
                    ..base
                }
            ),
            DiffClass::RecommendationDifference
        );
    }

    #[test]
    fn two_hundred_forty_case_replay_is_deterministic_and_native_effective() {
        let cases = (0..240).map(observation).collect::<Vec<_>>();
        let first = replay_parity_cases(&cases);
        let second = replay_parity_cases(&cases);
        assert_eq!(first, second);
        assert_eq!(first.len(), 240);
        assert!(first.iter().all(|receipt| receipt.native_effective));
        assert!(first
            .iter()
            .all(|receipt| !receipt.diff_class.as_str().is_empty()));
        assert!(first
            .iter()
            .any(|receipt| receipt.diff_class == DiffClass::ExactParity));
        assert!(first
            .iter()
            .any(|receipt| receipt.diff_class == DiffClass::RecommendationDifference));
    }

    #[test]
    fn vida_stack_allowlist_normalizes_only_canonical_github_spellings() {
        assert!(repository_is_allowlisted(VIDA_STACK_REPOSITORY_ALLOWLIST));
        assert!(repository_is_allowlisted(
            "https://github.com/pomazanbohdan/vida-stack.git/"
        ));
        assert!(repository_is_allowlisted(
            "git@github.com:pomazanbohdan/vida-stack.git"
        ));
        assert!(!repository_is_allowlisted("pomazanbohdan/other-repo"));
        assert!(!repository_is_allowlisted(
            "https://example.com/pomazanbohdan/vida-stack"
        ));
        assert!(!repository_is_allowlisted(&format!(
            "{}-suffix",
            "pomazanbohdan/vida-stack"
        )));
    }

    #[test]
    fn active_path_canaries_only_allowlisted_certain_runtime() {
        let facts = SemanticRoutingFacts::from_native(observation(3));
        let canary = active_canary_decision(&facts, VIDA_STACK_REPOSITORY_ALLOWLIST, true);
        assert_eq!(canary.path, ActiveRoutingPath::VidaStackCanary);
        assert_eq!(canary.fallback_reason, None);
        assert!(canary.native_effective);
        assert_eq!(canary.advisory_adjustment_percent, 0);
        assert_eq!(canary.decision, shadow_decision(&facts));

        for (repository, runtime_certain, reason) in [
            (
                "other-owner/other-repo",
                true,
                FallbackReason::RepositoryNotAllowlisted,
            ),
            (
                VIDA_STACK_REPOSITORY_ALLOWLIST,
                false,
                FallbackReason::RuntimeUncertain,
            ),
        ] {
            let fallback = active_canary_decision(&facts, repository, runtime_certain);
            assert_eq!(fallback.path, ActiveRoutingPath::NativeFallback);
            assert_eq!(fallback.fallback_reason, Some(reason));
            assert!(fallback.native_effective);
            assert_eq!(fallback.advisory_adjustment_percent, 0);
            assert_eq!(fallback.decision, native_fallback_decision(&facts));
            assert_eq!(fallback.decision.advisory_weights, AdvisoryWeights::zero());
            assert_eq!(fallback.decision.recommendation, "native_authority");
        }
    }

    #[test]
    fn sixty_four_allowlisted_canaries_have_stable_receipts() {
        let cases = (0..64).map(observation).collect::<Vec<_>>();
        let first = replay_active_canary_cases(&cases, VIDA_STACK_REPOSITORY_ALLOWLIST, true);
        let second = replay_active_canary_cases(&cases, VIDA_STACK_REPOSITORY_ALLOWLIST, true);
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first
            .iter()
            .all(|receipt| receipt.path == ActiveRoutingPath::VidaStackCanary));
        assert!(first.iter().all(|receipt| receipt.native_effective));
        assert!(first
            .iter()
            .all(|receipt| receipt.fallback_reason.is_none()));
    }

    #[test]
    fn ten_thousand_repeated_active_evaluation_digests_are_deterministic() {
        let facts = SemanticRoutingFacts::from_native(observation(51));
        let expected = active_decision_digest(&facts, VIDA_STACK_REPOSITORY_ALLOWLIST, true);
        for _ in 0..10_000 {
            assert_eq!(
                active_decision_digest(&facts, VIDA_STACK_REPOSITORY_ALLOWLIST, true),
                expected
            );
        }
    }
}
