use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const LEGACY_HOST_BRIDGE_SOURCE_CONTRACT_VERSION: &str = "legacy.host_bridge_result.v1";
pub const LEGACY_LANE_COMPLETION_SOURCE_CONTRACT_VERSION: &str = "legacy.lane_completion.v1";
pub const LEGACY_RUN_STATUS_SOURCE_CONTRACT_VERSION: &str = "legacy.run_status.v1";
pub const LEGACY_RECEIPT_SOURCE_CONTRACT_VERSION: &str = "legacy.receipt.v1";
pub const LEGACY_COMMAND_OPTIONS_SOURCE_CONTRACT_VERSION: &str = "legacy.command_options.v1";
pub const LEGACY_OUTCOME_CONTRADICTION: &str = "legacy_outcome_contradiction";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CompletionOutcome {
    Passed {
        #[serde(default)]
        evidence_refs: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reported_next_step: Option<FlowStepRef>,
    },
    Blocked {
        blockers: Vec<CompletionBlocker>,
        rework_target: FlowStepRef,
        #[serde(default)]
        evidence_refs: Vec<String>,
    },
}

impl CompletionOutcome {
    fn passed(reported_next_step: Option<FlowStepRef>) -> Self {
        Self::Passed {
            evidence_refs: Vec::new(),
            reported_next_step,
        }
    }

    fn blocked(
        blockers: Vec<CompletionBlocker>,
        rework_target: FlowStepRef,
    ) -> Result<Self, LegacyHostBridgeCompletionNormalizationError> {
        if blockers
            .iter()
            .any(|blocker| blocker.code.trim().is_empty())
        {
            return Err(normalization_error(
                "legacy_blocked_outcome_invalid",
                "blocked completion outcome requires non-empty blocker codes",
            ));
        }
        Ok(Self::Blocked {
            blockers,
            rework_target,
            evidence_refs: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionBlocker {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowStepRef(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyHostBridgeCompletionNormalization {
    pub source_contract_version: String,
    pub source_surface: String,
    pub outcome: CompletionOutcome,
    pub result_contract: Value,
    pub canonical_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyHostBridgeCompletionNormalizationError {
    pub blocker_code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyCompletionSurface {
    HostBridgeResult,
    LaneCompletion,
    RunStatus,
    Receipt,
    CommandOptions,
}

impl LegacyCompletionSurface {
    fn as_str(self) -> &'static str {
        match self {
            Self::HostBridgeResult => "host_bridge_result",
            Self::LaneCompletion => "lane_completion",
            Self::RunStatus => "run_status",
            Self::Receipt => "receipt",
            Self::CommandOptions => "command_options",
        }
    }

    fn default_source_contract_version(self) -> &'static str {
        match self {
            Self::HostBridgeResult => LEGACY_HOST_BRIDGE_SOURCE_CONTRACT_VERSION,
            Self::LaneCompletion => LEGACY_LANE_COMPLETION_SOURCE_CONTRACT_VERSION,
            Self::RunStatus => LEGACY_RUN_STATUS_SOURCE_CONTRACT_VERSION,
            Self::Receipt => LEGACY_RECEIPT_SOURCE_CONTRACT_VERSION,
            Self::CommandOptions => LEGACY_COMMAND_OPTIONS_SOURCE_CONTRACT_VERSION,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyTupleSignal {
    Pass,
    Blocked,
}

#[derive(Serialize)]
struct CanonicalLegacyCompletionPayload<'a> {
    source_contract_version: &'a str,
    source_surface: &'a str,
    outcome: &'a CompletionOutcome,
    result_contract: &'a Value,
}

pub fn normalize_legacy_host_bridge_completion_result(
    result: &Value,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    normalize_legacy_completion(result, LegacyCompletionSurface::HostBridgeResult)
}

pub fn normalize_legacy_lane_completion(
    result: &Value,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    normalize_legacy_completion(result, LegacyCompletionSurface::LaneCompletion)
}

pub fn normalize_legacy_run_status(
    result: &Value,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    normalize_legacy_completion(result, LegacyCompletionSurface::RunStatus)
}

pub fn normalize_legacy_receipt(
    result: &Value,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    normalize_legacy_completion(result, LegacyCompletionSurface::Receipt)
}

pub fn normalize_legacy_command_options(
    result: &Value,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    normalize_legacy_completion(result, LegacyCompletionSurface::CommandOptions)
}

fn normalize_legacy_completion(
    result: &Value,
    surface: LegacyCompletionSurface,
) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError> {
    let blocker_codes = legacy_blocker_codes(result);
    let blocker_evidence_present = legacy_blocker_evidence_present(result);
    let signals = legacy_tuple_signals(result);
    let pass_signal = signals
        .iter()
        .any(|signal| *signal == LegacyTupleSignal::Pass);
    let blocked_signal = signals
        .iter()
        .any(|signal| *signal == LegacyTupleSignal::Blocked)
        || !blocker_codes.is_empty()
        || string_field(result, "rework_target").is_some();

    if pass_signal && blocked_signal {
        return Err(normalization_error(
            LEGACY_OUTCOME_CONTRADICTION,
            "legacy result contains both pass/executed and blocked/rework evidence",
        ));
    }
    if canonical_result_tuple_present(result) {
        return Err(normalization_error(
            "legacy_outcome_not_required",
            "canonical result tuple should be validated by the result contract",
        ));
    }

    let source_contract_version = legacy_source_contract_version(result, surface);
    let next_step = string_field(result, "allowed_next_node")
        .or_else(|| string_field(result, "next_node"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value == "next" {
                Err(normalization_error(
                    "legacy_abstract_next_node",
                    "legacy result uses abstract allowed_next_node `next` instead of a concrete route",
                ))
            } else {
                Ok(FlowStepRef(value.to_string()))
            }
        })
        .transpose()?;
    let outcome = if pass_signal {
        if !blocker_evidence_present || !blocker_codes.is_empty() {
            return Err(normalization_error(
                "legacy_outcome_missing_empty_blocker_evidence",
                "legacy pass normalization requires explicit empty blocker evidence",
            ));
        }
        CompletionOutcome::passed(next_step.clone())
    } else if blocked_signal {
        if blocker_evidence_present && blocker_codes.is_empty() {
            return Err(normalization_error(
                "legacy_blocked_outcome_missing_blocker_codes",
                "blocked completion outcome with explicit blocker evidence requires non-empty blocker codes",
            ));
        }
        if result
            .get("rework_target")
            .is_some_and(|value| value.as_str().map(str::trim).is_none_or(str::is_empty))
        {
            return Err(normalization_error(
                "legacy_blocked_outcome_missing_rework_target",
                "blocked completion outcome with explicit rework target field requires a non-empty target",
            ));
        }
        let blockers = if blocker_codes.is_empty() {
            vec![CompletionBlocker {
                code: "legacy_result_blocked".to_string(),
                scope: None,
                evidence_refs: Vec::new(),
                next_actions: Vec::new(),
            }]
        } else {
            blocker_codes
                .into_iter()
                .map(|code| CompletionBlocker {
                    code,
                    scope: None,
                    evidence_refs: Vec::new(),
                    next_actions: Vec::new(),
                })
                .collect()
        };
        CompletionOutcome::blocked(
            blockers,
            FlowStepRef(
                string_field(result, "rework_target")
                    .or_else(|| string_field(result, "allowed_next_node"))
                    .unwrap_or("rework")
                    .to_string(),
            ),
        )?
    } else {
        return Err(normalization_error(
            "legacy_outcome_missing",
            "legacy result did not contain pass, executed, blocked, rework, or blocker evidence",
        ));
    };

    let result_contract = result_contract_for(&source_contract_version, &outcome);
    let canonical_json = serde_json::to_string(&CanonicalLegacyCompletionPayload {
        source_contract_version: &source_contract_version,
        source_surface: surface.as_str(),
        outcome: &outcome,
        result_contract: &result_contract,
    })
    .expect("canonical legacy completion payload should serialize");

    Ok(LegacyHostBridgeCompletionNormalization {
        source_contract_version,
        source_surface: surface.as_str().to_string(),
        outcome,
        result_contract,
        canonical_json,
    })
}

fn result_contract_for(source_contract_version: &str, outcome: &CompletionOutcome) -> Value {
    match outcome {
        CompletionOutcome::Passed {
            reported_next_step, ..
        } => json!({
            "source_contract_version": source_contract_version,
            "status": "pass",
            "execution_state": "executed",
            "decision": "approve",
            "verdict": "pass",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": reported_next_step.as_ref().map(|step| step.0.as_str()).unwrap_or("closure")
        }),
        CompletionOutcome::Blocked {
            blockers,
            rework_target,
            ..
        } => {
            let blocker_codes = blockers
                .iter()
                .map(|blocker| blocker.code.as_str())
                .collect::<Vec<_>>();
            json!({
                "source_contract_version": source_contract_version,
                "status": "blocked",
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": blocker_codes,
                "rework_target": rework_target.0,
                "allowed_next_node": rework_target.0
            })
        }
    }
}

fn legacy_tuple_signals(result: &Value) -> Vec<LegacyTupleSignal> {
    [
        signal_for_field(result, "status"),
        signal_for_field(result, "execution_state"),
        signal_for_field(result, "decision"),
        signal_for_field(result, "verdict"),
        signal_for_field(result, "completion_verdict"),
        signal_for_field(result, "outcome"),
        signal_for_field(result, "state"),
        signal_for_field(result, "lane_status"),
        signal_for_field(result, "run_status"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn canonical_result_tuple_present(result: &Value) -> bool {
    ["status", "execution_state", "decision", "verdict"]
        .iter()
        .all(|field| string_field(result, field).is_some())
}

fn signal_for_field(result: &Value, field: &str) -> Option<LegacyTupleSignal> {
    let value = string_field(result, field)?.to_ascii_lowercase();
    if matches!(
        value.as_str(),
        "pass"
            | "passed"
            | "approve"
            | "approved"
            | "accepted"
            | "ok"
            | "success"
            | "succeeded"
            | "executed"
            | "complete"
            | "completed"
            | "done"
    ) {
        return Some(LegacyTupleSignal::Pass);
    }
    if matches!(
        value.as_str(),
        "blocked"
            | "block"
            | "rework"
            | "rework_required"
            | "failed"
            | "failure"
            | "error"
            | "reject"
            | "rejected"
            | "denied"
    ) {
        return Some(LegacyTupleSignal::Blocked);
    }
    None
}

fn legacy_blocker_evidence_present(result: &Value) -> bool {
    result.get("blocker_codes").is_some()
        || result.get("blockers").is_some()
        || result.get("blocker_code").is_some()
}

fn legacy_blocker_codes(result: &Value) -> Vec<String> {
    let mut blockers = string_array_field(result, "blocker_codes");
    if blockers.is_empty() {
        blockers.extend(string_array_field(result, "blockers"));
    }
    if let Some(blocker) = string_field(result, "blocker_code") {
        push_unique(&mut blockers, blocker);
    }
    blockers
}

fn legacy_source_contract_version(result: &Value, surface: LegacyCompletionSurface) -> String {
    string_field(result, "source_contract_version")
        .or_else(|| string_field(result, "contract_version"))
        .map(ToOwned::to_owned)
        .or_else(|| {
            result
                .get("schema_version")
                .and_then(Value::as_u64)
                .map(|version| {
                    format!(
                        "{}.schema_v{version}",
                        surface.default_source_contract_version()
                    )
                })
        })
        .unwrap_or_else(|| surface.default_source_contract_version().to_string())
}

fn string_array_field(result: &Value, field: &str) -> Vec<String> {
    match result.get(field) {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::String(value)) if !value.trim().is_empty() => vec![value.trim().to_string()],
        _ => Vec::new(),
    }
}

fn string_field<'a>(result: &'a Value, field: &str) -> Option<&'a str> {
    result
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn normalization_error(
    blocker_code: &str,
    detail: &str,
) -> LegacyHostBridgeCompletionNormalizationError {
    LegacyHostBridgeCompletionNormalizationError {
        blocker_code: blocker_code.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct GoldenCase {
        name: String,
        surface: String,
        input: Value,
        expected_outcome: String,
        expected_source_contract_version: String,
        expected_blocker_code: Option<String>,
    }

    #[test]
    fn golden_compatibility_corpus_normalizes_current_rows() {
        let corpus: Vec<GoldenCase> =
            serde_json::from_str(include_str!("../fixtures/legacy_completion_corpus.json"))
                .expect("golden corpus parses");

        for row in corpus {
            let normalized = normalize_for_surface(&row.surface, &row.input)
                .unwrap_or_else(|error| panic!("{} should normalize: {error:?}", row.name));

            assert_eq!(
                normalized.source_contract_version, row.expected_source_contract_version,
                "{}",
                row.name
            );
            assert!(
                normalized
                    .canonical_json
                    .contains("\"source_contract_version\""),
                "{}",
                row.name
            );
            match (row.expected_outcome.as_str(), &normalized.outcome) {
                ("passed", CompletionOutcome::Passed { .. }) => {}
                ("blocked", CompletionOutcome::Blocked { blockers, .. }) => {
                    if let Some(expected) = row.expected_blocker_code {
                        assert!(
                            blockers.iter().any(|blocker| blocker.code == expected),
                            "{}",
                            row.name
                        );
                    }
                }
                _ => panic!(
                    "{} had unexpected outcome {:?}",
                    row.name, normalized.outcome
                ),
            }
        }
    }

    #[test]
    fn legacy_pass_executed_empty_blockers_is_byte_stable_passed() {
        let left = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "blocker_codes": [],
            "allowed_next_node": "coach"
        });
        let right = serde_json::json!({
            "allowed_next_node": "coach",
            "blocker_codes": [],
            "execution_state": "executed",
            "status": "pass"
        });

        let left = normalize_legacy_host_bridge_completion_result(&left).expect("left normalizes");
        let right =
            normalize_legacy_host_bridge_completion_result(&right).expect("right normalizes");

        assert!(matches!(left.outcome, CompletionOutcome::Passed { .. }));
        assert_eq!(left.canonical_json, right.canonical_json);
        assert_eq!(
            left.source_contract_version,
            LEGACY_HOST_BRIDGE_SOURCE_CONTRACT_VERSION
        );
    }

    #[test]
    fn june_2026_host_bridge_mixed_pass_blocked_tuple_is_rejected() {
        let legacy = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["host_agent_execution_failed"]
        });

        let error = normalize_legacy_host_bridge_completion_result(&legacy).unwrap_err();

        assert_eq!(error.blocker_code, LEGACY_OUTCOME_CONTRADICTION);
    }

    #[test]
    fn legacy_explicit_abstract_next_node_is_rejected() {
        let legacy = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "blocker_codes": [],
            "allowed_next_node": "next"
        });

        let error = normalize_legacy_host_bridge_completion_result(&legacy).unwrap_err();

        assert_eq!(error.blocker_code, "legacy_abstract_next_node");
    }

    #[test]
    fn normalizes_all_legacy_surface_kinds_with_source_versions() {
        let input = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["runtime_blocker"],
            "rework_target": "developer"
        });

        let cases = [
            (
                normalize_legacy_host_bridge_completion_result(&input).unwrap(),
                LEGACY_HOST_BRIDGE_SOURCE_CONTRACT_VERSION,
            ),
            (
                normalize_legacy_lane_completion(&input).unwrap(),
                LEGACY_LANE_COMPLETION_SOURCE_CONTRACT_VERSION,
            ),
            (
                normalize_legacy_run_status(&input).unwrap(),
                LEGACY_RUN_STATUS_SOURCE_CONTRACT_VERSION,
            ),
            (
                normalize_legacy_receipt(&input).unwrap(),
                LEGACY_RECEIPT_SOURCE_CONTRACT_VERSION,
            ),
            (
                normalize_legacy_command_options(&input).unwrap(),
                LEGACY_COMMAND_OPTIONS_SOURCE_CONTRACT_VERSION,
            ),
        ];

        for (normalized, source_contract_version) in cases {
            assert_eq!(normalized.source_contract_version, source_contract_version);
            assert!(matches!(
                normalized.outcome,
                CompletionOutcome::Blocked { .. }
            ));
        }
    }

    fn normalize_for_surface(
        surface: &str,
        value: &Value,
    ) -> Result<LegacyHostBridgeCompletionNormalization, LegacyHostBridgeCompletionNormalizationError>
    {
        match surface {
            "host_bridge_result" => normalize_legacy_host_bridge_completion_result(value),
            "lane_completion" => normalize_legacy_lane_completion(value),
            "run_status" => normalize_legacy_run_status(value),
            "receipt" => normalize_legacy_receipt(value),
            "command_options" => normalize_legacy_command_options(value),
            other => Err(normalization_error(
                "legacy_surface_unknown",
                &format!("unknown legacy surface `{other}`"),
            )),
        }
    }
}
