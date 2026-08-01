//! Bounded requirement-analysis atom contract.
//!
//! Rust owns source/path bounds, redaction, class validation, and iteration;
//! Rhai receives only the scalar shadow context and can recommend, never own,
//! downstream TaskFlow authority.

use std::fmt;

pub const FACT_SCHEMA_VERSION: u16 = 1;
pub const POLICY_ID: &str = "rhai.runtime.requirement-analysis";
pub const POLICY_VERSION: u32 = 1;
pub const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SOURCE_LINES: usize = 512;
pub const MAX_SOURCE_COUNT: usize = 8;
pub const MAX_ATOM_CHARS: usize = 2048;
pub const REDACTION_PLACEHOLDER: &str = "[redacted requirement secret line]";
pub const POLICY_SOURCE: &str =
    include_str!("../../../vida/policies/builtin/v1/policies/vida.requirement-analysis.v1");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementClass {
    Feature,
    Bug,
    RuntimeDefect,
    Documentation,
    Research,
    Release,
    Cleanup,
}

impl RequirementClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Bug => "bug",
            Self::RuntimeDefect => "runtime_defect",
            Self::Documentation => "documentation",
            Self::Research => "research",
            Self::Release => "release",
            Self::Cleanup => "cleanup",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "feature" => Some(Self::Feature),
            "bug" => Some(Self::Bug),
            "runtime_defect" => Some(Self::RuntimeDefect),
            "documentation" => Some(Self::Documentation),
            "research" => Some(Self::Research),
            "release" => Some(Self::Release),
            "cleanup" => Some(Self::Cleanup),
            _ => None,
        }
    }
}

impl fmt::Display for RequirementClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    Quick,
    Standard,
    Critical,
}

impl DepthMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Critical => "critical",
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "quick" => Self::Quick,
            "critical" => Self::Critical,
            _ => Self::Standard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementSource {
    pub path: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRequirementObservation {
    pub request_id: String,
    pub requirement_class: String,
    pub depth_mode: String,
    pub sources: Vec<RequirementSource>,
    pub party_chat_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementAnalysisError {
    MissingRequestId,
    UnknownClass(String),
    InvalidSourcePath(String),
    SourceTooLarge(usize),
}

impl fmt::Display for RequirementAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequestId => formatter.write_str("request_id is required"),
            Self::UnknownClass(class) => write!(formatter, "unknown requirement class: {class}"),
            Self::InvalidSourcePath(path) => write!(formatter, "invalid source path: {path}"),
            Self::SourceTooLarge(bytes) => write!(
                formatter,
                "source bytes {bytes} exceed {} byte bound",
                MAX_SOURCE_BYTES
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementFacts {
    pub schema_version: u16,
    pub request_id: String,
    pub requirement_class: RequirementClass,
    pub depth_mode: DepthMode,
    pub source_count: u8,
    pub source_bytes: usize,
    pub atom: String,
    pub ambiguity: bool,
    pub conflict: bool,
    pub party_chat_enabled: bool,
}

impl RequirementFacts {
    pub fn from_native(
        observation: NativeRequirementObservation,
    ) -> Result<Self, RequirementAnalysisError> {
        if observation.request_id.trim().is_empty() {
            return Err(RequirementAnalysisError::MissingRequestId);
        }
        let requirement_class = RequirementClass::parse(&observation.requirement_class)
            .ok_or_else(|| RequirementAnalysisError::UnknownClass(observation.requirement_class))?;
        let mut source_bytes = 0usize;
        let mut redacted_sources = Vec::new();
        for source in observation.sources.iter().take(MAX_SOURCE_COUNT) {
            if let Some(path) = &source.path {
                validate_source_path(path)?;
            }
            source_bytes = source_bytes.saturating_add(source.text.len());
            if source_bytes > MAX_SOURCE_BYTES {
                return Err(RequirementAnalysisError::SourceTooLarge(source_bytes));
            }
            redacted_sources.push(redact_source(&source.text));
        }
        let combined = redacted_sources.join("\n");
        let atom = combined
            .lines()
            .take(MAX_SOURCE_LINES)
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != REDACTION_PLACEHOLDER)
            .next()
            .unwrap_or("requirement_source_placeholder")
            .chars()
            .take(MAX_ATOM_CHARS)
            .collect::<String>();
        let lower = combined.to_ascii_lowercase();
        Ok(Self {
            schema_version: FACT_SCHEMA_VERSION,
            request_id: observation.request_id,
            requirement_class,
            depth_mode: DepthMode::parse(&observation.depth_mode),
            source_count: observation.sources.len().min(MAX_SOURCE_COUNT) as u8,
            source_bytes,
            atom,
            ambiguity: ["ambiguous", "unclear", "not sure", "tbd"]
                .iter()
                .any(|term| lower.contains(term)),
            conflict: ["conflict", "contradict", "without tests", "no tests"]
                .iter()
                .any(|term| lower.contains(term)),
            party_chat_enabled: observation.party_chat_enabled,
        })
    }

    pub fn context_literal(&self) -> String {
        format!(
            "#{{schema_version: {}, requirement_class: \"{}\", depth_mode: \"{}\", source_count: {}, source_bytes: {}, atom: \"{}\", ambiguity_hint: \"{}\", conflict_hint: \"{}\", party_chat_enabled: \"{}\"}}",
            self.schema_version,
            self.requirement_class,
            self.depth_mode.as_str(),
            self.source_count,
            self.source_bytes,
            rhai_escape(&self.atom),
            self.ambiguity,
            self.conflict,
            self.party_chat_enabled,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequirementDecision {
    pub requirement_class: RequirementClass,
    pub ambiguity: bool,
    pub conflict: bool,
    pub party_chat_recommended: bool,
    pub recommendation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReceipt {
    pub case_id: usize,
    pub facts_digest: String,
    pub decision_digest: String,
    pub native_effective: bool,
}

pub fn shadow_decision(facts: &RequirementFacts) -> RequirementDecision {
    let party_chat_recommended = facts.party_chat_enabled
        && (facts.depth_mode == DepthMode::Critical || facts.ambiguity || facts.conflict);
    let recommendation = if facts.conflict {
        "resolve_conflict"
    } else if facts.ambiguity {
        "clarify_scope"
    } else {
        "proceed_bounded"
    };
    RequirementDecision {
        requirement_class: facts.requirement_class,
        ambiguity: facts.ambiguity,
        conflict: facts.conflict,
        party_chat_recommended,
        recommendation,
    }
}

pub fn replay_shadow_cases(
    cases: &[NativeRequirementObservation],
) -> Vec<Result<ReplayReceipt, RequirementAnalysisError>> {
    cases
        .iter()
        .enumerate()
        .map(|(case_id, observation)| {
            let facts = RequirementFacts::from_native(observation.clone())?;
            let decision = shadow_decision(&facts);
            Ok(ReplayReceipt {
                case_id,
                facts_digest: stable_digest(&facts),
                decision_digest: decision_digest(&facts, decision),
                native_effective: true,
            })
        })
        .collect()
}

fn validate_source_path(path: &str) -> Result<(), RequirementAnalysisError> {
    let path = std::path::Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || path.as_os_str().is_empty()
    {
        return Err(RequirementAnalysisError::InvalidSourcePath(
            path.display().to_string(),
        ));
    }
    Ok(())
}

fn redact_source(source: &str) -> String {
    let mut secret_block = false;
    source
        .lines()
        .take(MAX_SOURCE_LINES)
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            let secret = secret_block
                || lower.contains("-----begin private key")
                || lower.contains("-----begin secret key")
                || lower.split(['=', ':']).next().is_some_and(|key| {
                    let key = key.trim_matches([' ', '"', '\'']);
                    key.contains("secret")
                        || key.contains("token")
                        || key.contains("password")
                        || key.contains("credential")
                        || key.contains("api_key")
                        || key.contains("private_key")
                });
            if secret {
                if lower.contains("-----begin private key")
                    || lower.contains("-----begin secret key")
                {
                    secret_block = true;
                }
                if lower.contains("-----end") {
                    secret_block = false;
                }
                REDACTION_PLACEHOLDER.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rhai_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn stable_digest(facts: &RequirementFacts) -> String {
    stable_digest_bytes(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        facts.schema_version,
        facts.request_id,
        facts.requirement_class,
        facts.depth_mode.as_str(),
        facts.source_count,
        facts.source_bytes,
        facts.atom,
        facts.ambiguity,
        facts.conflict,
    ))
}

fn decision_digest(facts: &RequirementFacts, decision: RequirementDecision) -> String {
    stable_digest_bytes(&format!(
        "{}|{}|{}|{}|{}",
        stable_digest(facts),
        decision.party_chat_recommended,
        decision.recommendation,
        decision.requirement_class,
        decision.ambiguity,
    ))
}

fn stable_digest_bytes(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(text: &str) -> RequirementSource {
        RequirementSource {
            path: Some("docs/request.md".to_string()),
            text: text.to_string(),
        }
    }

    fn observation(class: &str, text: &str) -> NativeRequirementObservation {
        NativeRequirementObservation {
            request_id: "req-1".to_string(),
            requirement_class: class.to_string(),
            depth_mode: "standard".to_string(),
            sources: vec![source(text)],
            party_chat_enabled: true,
        }
    }

    #[test]
    fn allowed_class_matrix_is_strict() {
        for class in [
            "feature",
            "bug",
            "runtime_defect",
            "documentation",
            "research",
            "release",
            "cleanup",
        ] {
            assert!(RequirementFacts::from_native(observation(class, "atom")).is_ok());
        }
        assert!(matches!(
            RequirementFacts::from_native(observation("unknown", "atom")),
            Err(RequirementAnalysisError::UnknownClass(_))
        ));
    }

    #[test]
    fn empty_and_many_sources_remain_bounded() {
        let mut empty = observation("feature", "ignored");
        empty.sources.clear();
        let facts = RequirementFacts::from_native(empty).expect("empty source placeholder");
        assert_eq!(facts.source_count, 0);
        assert_eq!(facts.atom, "requirement_source_placeholder");

        let mut many = observation("feature", "first atom");
        many.sources = (0..(MAX_SOURCE_COUNT + 3))
            .map(|index| source(&format!("atom-{index}")))
            .collect();
        let facts = RequirementFacts::from_native(many).expect("bounded sources");
        assert_eq!(facts.source_count, MAX_SOURCE_COUNT as u8);
        assert_eq!(facts.atom, "atom-0");
    }

    #[test]
    fn secret_lines_are_redacted_before_atom_and_context_projection() {
        let facts = RequirementFacts::from_native(observation(
            "feature",
            "Implement editable fields\nAPI_TOKEN=[REDACTED:API key param]",
        ))
        .expect("redacted source with normal line");
        assert_eq!(facts.atom, "Implement editable fields");
        assert!(!facts.context_literal().contains(REDACTION_PLACEHOLDER));
    }

    #[test]
    fn ambiguity_conflict_and_party_chat_are_advisory_shadow_fields() {
        let mut input = observation("runtime_defect", "ambiguous requirement conflict; no tests");
        input.depth_mode = "critical".to_string();
        let facts = RequirementFacts::from_native(input).expect("facts");
        let decision = shadow_decision(&facts);
        assert!(decision.ambiguity);
        assert!(decision.conflict);
        assert!(decision.party_chat_recommended);
        assert_eq!(decision.recommendation, "resolve_conflict");
    }

    #[test]
    fn invalid_paths_and_oversized_sources_fail_closed() {
        let mut path = observation("feature", "atom");
        path.sources[0].path = Some("../outside.md".to_string());
        assert!(matches!(
            RequirementFacts::from_native(path),
            Err(RequirementAnalysisError::InvalidSourcePath(_))
        ));
        let huge = observation("feature", &"x".repeat(MAX_SOURCE_BYTES + 1));
        assert!(matches!(
            RequirementFacts::from_native(huge),
            Err(RequirementAnalysisError::SourceTooLarge(_))
        ));
    }

    #[test]
    fn deterministic_shadow_replay_receipts() {
        let cases = (0..64)
            .map(|index| observation("feature", &format!("atom-{index}")))
            .collect::<Vec<_>>();
        let first = replay_shadow_cases(&cases);
        let second = replay_shadow_cases(&cases);
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first
            .iter()
            .all(|receipt| receipt.as_ref().is_ok_and(|row| row.native_effective)));
    }
}
