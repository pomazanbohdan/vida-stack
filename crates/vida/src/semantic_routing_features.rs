use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct SemanticRoutingFeatureInput<'a> {
    pub(crate) request_text: &'a str,
    pub(crate) task_title: Option<&'a str>,
    pub(crate) task_description: Option<&'a str>,
    pub(crate) runtime_role: Option<&'a str>,
    pub(crate) task_class: Option<&'a str>,
    pub(crate) route_key: Option<&'a str>,
    pub(crate) max_text_chars: usize,
}

impl<'a> SemanticRoutingFeatureInput<'a> {
    pub(crate) fn request(request_text: &'a str) -> Self {
        Self {
            request_text,
            task_title: None,
            task_description: None,
            runtime_role: None,
            task_class: None,
            route_key: None,
            max_text_chars: 12_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct SemanticRoutingFeatureVector {
    pub(crate) schema_version: u8,
    pub(crate) feature_source: &'static str,
    pub(crate) complexity_score: f64,
    pub(crate) complexity_band: &'static str,
    pub(crate) detected_domain: &'static str,
    pub(crate) domain_score: f64,
    pub(crate) signals: SemanticRoutingSignals,
    pub(crate) matched_terms: MatchedSemanticTerms,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct SemanticRoutingSignals {
    pub(crate) has_code: bool,
    pub(crate) has_math: bool,
    pub(crate) requires_reasoning: bool,
    pub(crate) is_multilingual: bool,
    pub(crate) is_translation: bool,
    pub(crate) is_creative: bool,
    pub(crate) is_security: bool,
    pub(crate) is_devops: bool,
    pub(crate) is_data: bool,
    pub(crate) has_multi_step: bool,
    pub(crate) has_specifics: bool,
    pub(crate) has_qualifiers: bool,
    pub(crate) expert_verb_score: f64,
    pub(crate) avg_word_length: f64,
    pub(crate) word_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub(crate) struct MatchedSemanticTerms {
    pub(crate) domain: Vec<String>,
    pub(crate) expert_verbs: Vec<String>,
    pub(crate) multi_step: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SemanticScoreInputs {
    pub(crate) quality: f64,
    pub(crate) reasoning: f64,
    pub(crate) reliability: f64,
    pub(crate) speed: f64,
    pub(crate) cost: f64,
    pub(crate) domain_fit: f64,
    pub(crate) complexity_mismatch_penalty: f64,
    pub(crate) lifecycle_penalty: f64,
    pub(crate) over_budget_penalty: f64,
    pub(crate) readiness_penalty: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct SemanticScoreBreakdown {
    pub(crate) quality_component: f64,
    pub(crate) reasoning_component: f64,
    pub(crate) reliability_component: f64,
    pub(crate) speed_component: f64,
    pub(crate) cost_component: f64,
    pub(crate) domain_fit_component: f64,
    pub(crate) penalties: Vec<SemanticScorePenalty>,
    pub(crate) semantic_route_score: f64,
    pub(crate) advisory_only: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct SemanticScorePenalty {
    pub(crate) code: &'static str,
    pub(crate) value: f64,
}

#[derive(Debug, Clone, Copy)]
struct SemanticCostWeights {
    cost: f64,
    speed: f64,
    quality: f64,
    reasoning: f64,
    reliability: f64,
}

pub(crate) fn extract_semantic_routing_features(
    input: &SemanticRoutingFeatureInput<'_>,
) -> SemanticRoutingFeatureVector {
    let text = joined_input_text(input);
    let normalized = text.to_lowercase();
    let tokens = tokenize(&normalized);
    let word_count = tokens.len();
    let avg_word_length = if word_count == 0 {
        0.0
    } else {
        round2(
            tokens
                .iter()
                .map(|token| token.chars().count() as f64)
                .sum::<f64>()
                / word_count as f64,
        )
    };

    let code_terms = [
        "```",
        "fn ",
        "impl ",
        "class ",
        "cargo ",
        "serde",
        "json",
        "yaml",
        "stack trace",
        "exception",
        ".rs",
        ".ts",
        ".dart",
        "sql",
    ];
    let math_terms = [
        "equation", "proof", "formula", "derive", "integral", "matrix",
    ];
    let translation_terms = [
        "translate",
        "translation",
        "localize",
        "localization",
        "i18n",
    ];
    let creative_terms = ["story", "poem", "copywriting", "slogan", "brand voice"];
    let security_terms = [
        "security",
        "vulnerability",
        "cve",
        "injection",
        "auth",
        "secret",
        "pii",
        "threat",
        "exploit",
    ];
    let devops_terms = [
        "docker",
        "kubernetes",
        "deploy",
        "release",
        "pipeline",
        "ci",
        "cd",
        "terraform",
        "runtime",
    ];
    let data_terms = [
        "dataset",
        "dataframe",
        "analytics",
        "warehouse",
        "query",
        "index",
        "retrieval",
    ];
    let multi_step_terms = [
        "first", "then", "after", "before", "next", "step", "sequence", "plan", "phase",
    ];
    let qualifier_terms = [
        "must",
        "should",
        "cannot",
        "only",
        "deterministic",
        "fail closed",
        "without network",
        "receipt",
    ];
    let expert_verbs = [
        "analyze",
        "design",
        "diagnose",
        "optimize",
        "validate",
        "verify",
        "prove",
        "implement",
        "refactor",
        "orchestrate",
    ];

    let has_code = contains_any(&normalized, &code_terms);
    let has_math = contains_any(&normalized, &math_terms)
        || normalized.contains('+')
        || normalized.contains('=');
    let is_translation = contains_any(&normalized, &translation_terms);
    let is_creative = contains_any(&normalized, &creative_terms);
    let is_security = contains_any(&normalized, &security_terms);
    let is_devops = contains_any(&normalized, &devops_terms);
    let is_data = contains_any(&normalized, &data_terms);
    let has_multi_step = contains_any(&normalized, &multi_step_terms);
    let has_specifics = has_code
        || text.contains('`')
        || text.contains('/')
        || text.contains('\\')
        || text.chars().any(|ch| ch.is_ascii_digit());
    let has_qualifiers = contains_any(&normalized, &qualifier_terms);
    let expert_matches = matched_terms(&normalized, &expert_verbs);
    let expert_verb_score = round2((expert_matches.len() as f64 / 10.0).min(1.0));
    let is_multilingual = has_latin(&text) && has_cyrillic(&text);

    let mut matched = MatchedSemanticTerms {
        domain: Vec::new(),
        expert_verbs: expert_matches,
        multi_step: matched_terms(&normalized, &multi_step_terms),
    };
    let (detected_domain, domain_score) = detect_domain(&normalized, input, &mut matched);

    let requires_reasoning = has_multi_step
        || has_qualifiers
        || matches!(
            detected_domain,
            "architecture" | "security" | "legal" | "medical"
        )
        || matched.expert_verbs.iter().any(|term| {
            matches!(
                term.as_str(),
                "analyze" | "design" | "diagnose" | "optimize" | "validate" | "verify" | "prove"
            )
        });

    let signals = SemanticRoutingSignals {
        has_code,
        has_math,
        requires_reasoning,
        is_multilingual,
        is_translation,
        is_creative,
        is_security,
        is_devops,
        is_data,
        has_multi_step,
        has_specifics,
        has_qualifiers,
        expert_verb_score,
        avg_word_length,
        word_count,
    };
    let complexity_score = complexity_score(&signals, domain_score);

    SemanticRoutingFeatureVector {
        schema_version: 1,
        feature_source: "vida_native_deterministic_rules",
        complexity_score,
        complexity_band: complexity_band(complexity_score),
        detected_domain,
        domain_score,
        signals,
        matched_terms: matched,
    }
}

pub(crate) fn score_semantic_route(
    feature_vector: &SemanticRoutingFeatureVector,
    inputs: SemanticScoreInputs,
) -> SemanticScoreBreakdown {
    let weights = weights_for_band(feature_vector.complexity_band);
    let quality_component = round2(inputs.quality.max(0.0) * weights.quality);
    let reasoning_component = round2(inputs.reasoning.max(0.0) * weights.reasoning);
    let reliability_component = round2(inputs.reliability.max(0.0) * weights.reliability);
    let speed_component = round2(inputs.speed.max(0.0) * weights.speed);
    let cost_component = round2(-inputs.cost.max(0.0) * weights.cost);
    let domain_fit_component = round2(inputs.domain_fit.max(0.0) * 0.10);

    let penalties = [
        (
            "complexity_mismatch_penalty",
            inputs.complexity_mismatch_penalty,
        ),
        ("lifecycle_penalty", inputs.lifecycle_penalty),
        ("over_budget_penalty", inputs.over_budget_penalty),
        ("readiness_penalty", inputs.readiness_penalty),
    ]
    .into_iter()
    .filter_map(|(code, value)| {
        (value > 0.0).then_some(SemanticScorePenalty {
            code,
            value: round2(value),
        })
    })
    .collect::<Vec<_>>();

    let penalty_total = penalties.iter().map(|penalty| penalty.value).sum::<f64>();
    let semantic_route_score = round2(
        quality_component
            + reasoning_component
            + reliability_component
            + speed_component
            + cost_component
            + domain_fit_component
            - penalty_total,
    );

    SemanticScoreBreakdown {
        quality_component,
        reasoning_component,
        reliability_component,
        speed_component,
        cost_component,
        domain_fit_component,
        penalties,
        semantic_route_score,
        advisory_only: true,
    }
}

fn joined_input_text(input: &SemanticRoutingFeatureInput<'_>) -> String {
    [
        Some(input.request_text),
        input.task_title,
        input.task_description,
        input.runtime_role,
        input.task_class,
        input.route_key,
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
    .chars()
    .take(input.max_text_chars.max(1))
    .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn matched_terms(text: &str, terms: &[&str]) -> Vec<String> {
    terms
        .iter()
        .filter(|term| text.contains(**term))
        .map(|term| (*term).to_string())
        .collect()
}

fn detect_domain(
    text: &str,
    input: &SemanticRoutingFeatureInput<'_>,
    matched: &mut MatchedSemanticTerms,
) -> (&'static str, f64) {
    let mut scores = BTreeMap::from([
        ("architecture", 0_u8),
        ("security", 0),
        ("legal", 0),
        ("medical", 0),
        ("devops", 0),
        ("data", 0),
        ("implementation", 0),
        ("release", 0),
    ]);
    let domain_terms = [
        (
            "architecture",
            [
                "architecture",
                "system design",
                "runtime law",
                "orchestration",
                "scheduler",
                "contract",
            ]
            .as_slice(),
        ),
        (
            "security",
            [
                "security",
                "vulnerability",
                "injection",
                "threat",
                "auth",
                "secret",
            ]
            .as_slice(),
        ),
        (
            "legal",
            ["legal", "contract review", "compliance", "policy"].as_slice(),
        ),
        (
            "medical",
            ["medical", "clinical", "diagnosis", "patient"].as_slice(),
        ),
        (
            "devops",
            ["docker", "deploy", "pipeline", "release", "ci", "runtime"].as_slice(),
        ),
        (
            "data",
            ["dataset", "query", "index", "retrieval", "analytics"].as_slice(),
        ),
        (
            "implementation",
            ["implement", "code", "refactor", "test", "cargo"].as_slice(),
        ),
        (
            "release",
            ["release", "install", "archive", "artifact", "version"].as_slice(),
        ),
    ];

    for (domain, terms) in domain_terms {
        for term in terms {
            if text.contains(term) {
                *scores.entry(domain).or_default() += 1;
                matched.domain.push(term.to_string());
            }
        }
    }

    for value in [
        input.runtime_role,
        input.task_class,
        input.route_key,
        input.task_title,
    ]
    .into_iter()
    .flatten()
    {
        let value = value.to_ascii_lowercase();
        for domain in scores.clone().into_keys() {
            if value.contains(domain) {
                *scores.entry(domain).or_default() += 1;
            }
        }
    }

    let (domain, score) = scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .unwrap_or(("general", 0));
    if score == 0 {
        ("general", 0.0)
    } else {
        (domain, round2((score as f64 / 8.0).min(1.0)))
    }
}

fn complexity_score(signals: &SemanticRoutingSignals, domain_score: f64) -> f64 {
    let word_component = match signals.word_count {
        0..=8 => 0.05,
        9..=24 => 0.12,
        25..=64 => 0.20,
        _ => 0.28,
    };
    let mut score = 0.10 + word_component + domain_score * 0.20;
    if signals.has_code {
        score += 0.10;
    }
    if signals.has_math {
        score += 0.08;
    }
    if signals.requires_reasoning {
        score += 0.16;
    }
    if signals.has_multi_step {
        score += 0.10;
    }
    if signals.has_specifics {
        score += 0.07;
    }
    if signals.has_qualifiers {
        score += 0.06;
    }
    if signals.is_security {
        score += 0.08;
    }
    if signals.is_multilingual {
        score += 0.04;
    }
    round2(score.min(1.0))
}

fn complexity_band(score: f64) -> &'static str {
    match score {
        value if value < 0.25 => "simple",
        value if value < 0.50 => "medium",
        value if value < 0.75 => "medium_high",
        _ => "complex",
    }
}

fn weights_for_band(band: &str) -> SemanticCostWeights {
    match band {
        "simple" => SemanticCostWeights {
            cost: 0.45,
            speed: 0.20,
            quality: 0.20,
            reasoning: 0.05,
            reliability: 0.10,
        },
        "medium" => SemanticCostWeights {
            cost: 0.25,
            speed: 0.15,
            quality: 0.30,
            reasoning: 0.15,
            reliability: 0.15,
        },
        _ => SemanticCostWeights {
            cost: 0.15,
            speed: 0.10,
            quality: 0.35,
            reasoning: 0.25,
            reliability: 0.15,
        },
    }
}

fn has_latin(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn has_cyrillic(text: &str) -> bool {
    text.chars()
        .any(|ch| ('\u{0400}'..='\u{04FF}').contains(&ch))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(text: &str) -> SemanticRoutingFeatureVector {
        extract_semantic_routing_features(&SemanticRoutingFeatureInput::request(text))
    }

    #[test]
    fn simple_request_stays_low_complexity_and_cost_sensitive() {
        let feature = features("Summarize this note.");
        assert_eq!(feature.feature_source, "vida_native_deterministic_rules");
        assert_eq!(feature.complexity_band, "simple");
        assert!(!feature.signals.requires_reasoning);

        let score = score_semantic_route(
            &feature,
            SemanticScoreInputs {
                quality: 60.0,
                reasoning: 10.0,
                reliability: 50.0,
                speed: 80.0,
                cost: 40.0,
                domain_fit: 0.0,
                complexity_mismatch_penalty: 0.0,
                lifecycle_penalty: 0.0,
                over_budget_penalty: 0.0,
                readiness_penalty: 0.0,
            },
        );

        assert_eq!(score.cost_component, -18.0);
        assert!(score.advisory_only);
    }

    #[test]
    fn architecture_code_request_detects_reasoning_domain_and_multistep_shape() {
        let feature = extract_semantic_routing_features(&SemanticRoutingFeatureInput {
            request_text: "First design the runtime architecture, then implement Rust code with serde json receipts and validate without network calls.",
            task_title: Some("runtime routing architecture"),
            task_description: None,
            runtime_role: Some("solution_architect"),
            task_class: Some("architecture"),
            route_key: Some("architecture"),
            max_text_chars: 12_000,
        });

        assert_eq!(feature.detected_domain, "architecture");
        assert!(feature.signals.has_code);
        assert!(feature.signals.requires_reasoning);
        assert!(feature.signals.has_multi_step);
        assert!(matches!(feature.complexity_band, "medium_high" | "complex"));
        assert!(feature
            .matched_terms
            .expert_verbs
            .contains(&"design".to_string()));
    }

    #[test]
    fn security_request_raises_risk_domain_without_authority() {
        let feature = features(
            "Analyze authentication injection risk, secret handling, and verification gates before release.",
        );
        assert_eq!(feature.detected_domain, "security");
        assert!(feature.signals.is_security);
        assert!(feature.signals.requires_reasoning);

        let score = score_semantic_route(
            &feature,
            SemanticScoreInputs {
                quality: 90.0,
                reasoning: 90.0,
                reliability: 75.0,
                speed: 20.0,
                cost: 30.0,
                domain_fit: 80.0,
                complexity_mismatch_penalty: 0.0,
                lifecycle_penalty: 0.0,
                over_budget_penalty: 50.0,
                readiness_penalty: 25.0,
            },
        );

        assert!(score.semantic_route_score < 0.0);
        assert_eq!(score.penalties.len(), 2);
        assert!(score.advisory_only);
    }

    #[test]
    fn mixed_script_text_is_detected_without_external_services() {
        let text = "Implement runtime plan for \u{043F}\u{0440}\u{043E}\u{0454}\u{043A}\u{0442} with exact validation steps.";
        let feature = features(text);
        assert!(feature.signals.is_multilingual);
        assert!(feature.signals.requires_reasoning);
        assert_eq!(feature.feature_source, "vida_native_deterministic_rules");
    }

    #[test]
    fn devops_data_terms_are_visible_as_diagnostics() {
        let feature = features(
            "Plan Docker release pipeline for retrieval index data, then validate deployment telemetry.",
        );
        assert!(feature.signals.is_devops);
        assert!(feature.signals.is_data);
        assert!(feature.domain_score > 0.0);
        assert!(!feature.matched_terms.domain.is_empty());
    }
}
