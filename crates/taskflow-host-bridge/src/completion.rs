use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taskflow_contracts::{Release1ContractStatus, release1_contract_status_str};
use time::OffsetDateTime;

use crate::provenance::HostBridgeProvenanceDecision;
use crate::receipt_binding::DispatchReceiptBindingDecision;
use crate::request::HostBridgeRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeCompletionInput {
    pub request: HostBridgeRequest,
    pub provenance: HostBridgeProvenanceDecision,
    pub receipt_binding: DispatchReceiptBindingDecision,
    pub artifact_refs: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeCompletionEvidence {
    pub status: String,
    pub request_id: String,
    pub run_id: String,
    pub dispatch_target: String,
    pub completion_ready: bool,
    pub blocker_codes: Vec<String>,
    pub artifact_refs: Vec<PathBuf>,
    pub recorded_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeCompletionVerdict {
    pub status: String,
    pub execution_state: String,
    pub completion_verdict: String,
    pub completion_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeResultVerdictFields {
    pub decision: String,
    pub verdict: String,
    pub blocker_codes: Vec<String>,
    pub rework_target: Option<String>,
    pub allowed_next_node: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostBridgeQualityGateTransition {
    pub gate: &'static str,
    pub blocker_code: &'static str,
    pub rework_target: &'static str,
    pub blocked_allowed_next_node: &'static str,
}

const QUALITY_GATE_TRANSITIONS: &[(&[&str], HostBridgeQualityGateTransition)] = &[
    (
        &[
            "coach",
            "coach_lane",
            "coach_test_gate",
            "coach_implementation_gate",
            "coach_validator",
        ],
        HostBridgeQualityGateTransition {
            gate: "coach",
            blocker_code: "coach_rework_required",
            rework_target: "developer",
            blocked_allowed_next_node: "developer_rework",
        },
    ),
    (
        &[
            "tester",
            "tester_lane",
            "verification",
            "verification_lane",
            "verifier",
            "verifier_lane",
        ],
        HostBridgeQualityGateTransition {
            gate: "tester",
            blocker_code: "verification_rework_required",
            rework_target: "developer",
            blocked_allowed_next_node: "developer_rework",
        },
    ),
    (
        &[
            "reviewer",
            "reviewer_lane",
            "review",
            "review_lane",
            "duplication_reviewer",
        ],
        HostBridgeQualityGateTransition {
            gate: "reviewer",
            blocker_code: "review_rework_required",
            rework_target: "tester",
            blocked_allowed_next_node: "tester",
        },
    ),
];

#[must_use]
pub fn host_bridge_quality_gate_transition(
    completed_target: &str,
) -> Option<HostBridgeQualityGateTransition> {
    let normalized = completed_target.trim();
    QUALITY_GATE_TRANSITIONS
        .iter()
        .find(|(aliases, _)| aliases.iter().any(|alias| *alias == normalized))
        .map(|(_, transition)| *transition)
}

fn host_bridge_quality_gate_transition_for_blockers(
    blocker_codes: &[String],
) -> Option<HostBridgeQualityGateTransition> {
    QUALITY_GATE_TRANSITIONS
        .iter()
        .find(|(_, transition)| {
            blocker_codes
                .iter()
                .any(|blocker| blocker.trim() == transition.blocker_code)
        })
        .map(|(_, transition)| *transition)
}

pub fn materialize_host_bridge_completion_evidence(
    input: &HostBridgeCompletionInput,
) -> HostBridgeCompletionEvidence {
    let mut blocker_codes = Vec::new();
    if !input.provenance.accepted {
        blocker_codes.extend(input.provenance.blocker_codes.clone());
    }
    if !input.receipt_binding.accepted {
        blocker_codes.extend(input.receipt_binding.blocker_codes.clone());
    }

    HostBridgeCompletionEvidence {
        status: release1_contract_status_str(blocker_codes.is_empty()).to_string(),
        request_id: input.request.request_id.clone(),
        run_id: input.request.run_id.clone(),
        dispatch_target: input.request.dispatch_target.clone(),
        completion_ready: blocker_codes.is_empty(),
        blocker_codes,
        artifact_refs: input.artifact_refs.clone(),
        recorded_at: OffsetDateTime::now_utc(),
    }
}

#[must_use]
pub fn host_bridge_completion_retryable_blocker(blocker_code: &str) -> bool {
    matches!(
        blocker_code,
        "lane_completion_blocked_by_summary"
            | "verification_rework_required"
            | "coach_rework_required"
            | "review_rework_required"
            | "closure_evidence_blocked"
            | "host_bridge_request_task_mismatch"
    ) || matches!(
        taskflow_contracts::BlockerCode::try_from(blocker_code),
        Ok(taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactChangedFilesMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityInvalid)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactContractInvalid)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactReceiptMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactReceiptUnverified)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactsMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationAttemptScopeGuardViolation)
    )
}

#[must_use]
pub fn host_bridge_lane_completion_summary_blocker_code(
    completed_target: &str,
    summary: Option<&str>,
) -> Option<String> {
    let normalized = summary?.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let classifier_text = completion_summary_classifier_text(&normalized);
    let has_explicit_blocker_verdict = [
        "verdict: blocker",
        "verdict=blocker",
        "verdict: blocked",
        "verdict=blocked",
        "verdict: rework_required",
        "verdict=rework_required",
        "completion_verdict: blocker",
        "completion_verdict=blocker",
        "completion_verdict: blocked",
        "completion_verdict=blocked",
        "completion_verdict: rework_required",
        "completion_verdict=rework_required",
        "decision: blocked",
        "decision=blocked",
        "decision: blocker",
        "decision=blocker",
        "decision: rework_required",
        "decision=rework_required",
        "status: blocked",
        "status=blocked",
        "blocker: true",
        "blocked: true",
    ]
    .iter()
    .any(|needle| normalized.contains(needle));
    let has_negative_completion_semantics = [
        "not closure-ready",
        "not closure ready",
        "not approve",
        "not approved",
        "not closure_ready",
        "rework",
        "review_findings",
        "changed_scope",
        "implementation evidence absent",
        "implementation evidence missing",
        "product implementation evidence absent",
        "product implementation evidence missing",
        "not ready for closure",
        "closure not ready",
    ]
    .iter()
    .any(|needle| classifier_text.contains(needle));

    let has_explicit_blocker = has_explicit_blocker_verdict || has_negative_completion_semantics;
    if !has_explicit_blocker {
        return None;
    }

    let only_positive_blocker_context = [
        "no blocker",
        "no blockers",
        "without blockers",
        "blockers: []",
        "blocker_codes: []",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
        && ![
            "not closure-ready",
            "not closure ready",
            "not approve",
            "not approved",
            "rework",
            "review_findings",
            "changed_scope",
            "implementation evidence absent",
            "implementation evidence missing",
        ]
        .iter()
        .any(|needle| normalized.contains(needle));

    if only_positive_blocker_context {
        return None;
    }

    Some(blocker_code_for_completion_context(completed_target, &normalized).to_string())
}

fn blocker_code_for_completion_context(
    completed_target: &str,
    normalized_summary: &str,
) -> &'static str {
    for (_, transition) in QUALITY_GATE_TRANSITIONS {
        if quality_gate_decision_is_blocked(transition.gate, normalized_summary) {
            return transition.blocker_code;
        }
    }
    blocker_code_for_completed_target(completed_target)
}

fn blocker_code_for_completed_target(completed_target: &str) -> &'static str {
    if let Some(transition) = host_bridge_quality_gate_transition(completed_target) {
        return transition.blocker_code;
    }
    match completed_target.trim() {
        "closure" | "closure_lane" => "closure_evidence_blocked",
        _ => "lane_completion_blocked_by_summary",
    }
}

fn quality_gate_decision_is_blocked(gate: &str, normalized_summary: &str) -> bool {
    [
        format!("{gate} decision: blocked"),
        format!("{gate} decision=blocked"),
        format!("{gate} decision: blocker"),
        format!("{gate} decision=blocker"),
        format!("{gate} decision: rework_required"),
        format!("{gate} decision=rework_required"),
    ]
    .iter()
    .any(|needle| normalized_summary.contains(needle))
}

fn completion_summary_classifier_text(normalized_summary: &str) -> String {
    [
        "blocker_codes",
        "blocker code",
        "blocker codes",
        "blocker_code",
        "blockers field",
        "blockers array",
    ]
    .iter()
    .fold(normalized_summary.to_string(), |text, field_name| {
        text.replace(field_name, " ")
    })
}

#[must_use]
pub fn host_bridge_artifact_has_retryable_completion_blocker(artifact: &Value) -> bool {
    artifact
        .get("blocker_code")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(host_bridge_completion_retryable_blocker)
        || artifact
            .get("blocker_codes")
            .and_then(Value::as_array)
            .is_some_and(|blockers| {
                blockers.iter().any(|blocker| {
                    blocker
                        .as_str()
                        .map(str::trim)
                        .is_some_and(host_bridge_completion_retryable_blocker)
                })
            })
}

#[must_use]
pub fn host_bridge_request_status_allows_parent_completion(
    request_status: &str,
    retryable_completion_evidence: bool,
) -> bool {
    request_status == "pending" || retryable_completion_evidence
}

#[must_use]
pub fn host_bridge_existing_request_status_is_admissible(status: &str) -> bool {
    matches!(status, "pending" | "completed")
}

#[must_use]
pub fn host_bridge_completed_artifact_status_is_admissible(status: &str) -> bool {
    taskflow_contracts::canonical_release1_contract_status_str(status)
        == Some(Release1ContractStatus::Pass.as_str())
}

#[must_use]
pub fn host_bridge_completed_result_status_is_admissible(status: &str) -> bool {
    matches!(
        taskflow_contracts::canonical_release1_contract_status_str(status),
        Some(status)
            if status == Release1ContractStatus::Pass.as_str()
                || status == Release1ContractStatus::Blocked.as_str()
    )
}

#[must_use]
pub fn host_bridge_completed_result_execution_state_is_admissible(execution_state: &str) -> bool {
    matches!(execution_state, "executed" | "blocked")
}

#[must_use]
pub fn normalize_host_bridge_provenance_for_completion(
    provenance: &HostBridgeProvenanceDecision,
    retryable_completion_evidence: bool,
) -> HostBridgeProvenanceDecision {
    let mut blocker_codes = provenance.blocker_codes.clone();
    if retryable_completion_evidence {
        blocker_codes.retain(|code| code != "request_status_not_admissible");
    }
    HostBridgeProvenanceDecision {
        accepted: blocker_codes.is_empty(),
        blocker_codes,
        reason: if provenance.accepted || retryable_completion_evidence {
            provenance.reason.clone()
        } else {
            "host bridge request provenance rejected fail-closed".to_string()
        },
    }
}

#[must_use]
pub fn host_bridge_completion_verdict(blocker_codes: &[String]) -> HostBridgeCompletionVerdict {
    if blocker_codes.is_empty() {
        HostBridgeCompletionVerdict {
            status: Release1ContractStatus::Pass.as_str().to_string(),
            execution_state: "executed".to_string(),
            completion_verdict: Release1ContractStatus::Pass.as_str().to_string(),
            completion_ready: true,
        }
    } else {
        HostBridgeCompletionVerdict {
            status: Release1ContractStatus::Blocked.as_str().to_string(),
            execution_state: Release1ContractStatus::Blocked.as_str().to_string(),
            completion_verdict: "rework_required".to_string(),
            completion_ready: false,
        }
    }
}

#[must_use]
pub fn host_bridge_result_verdict_fields(
    blocker_codes: &[String],
    rework_target: Option<&str>,
) -> HostBridgeResultVerdictFields {
    host_bridge_result_verdict_fields_for_gate("", blocker_codes, rework_target)
}

#[must_use]
pub fn host_bridge_result_verdict_fields_for_gate(
    completed_target: &str,
    blocker_codes: &[String],
    rework_target: Option<&str>,
) -> HostBridgeResultVerdictFields {
    if blocker_codes.is_empty() {
        HostBridgeResultVerdictFields {
            decision: "approve".to_string(),
            verdict: Release1ContractStatus::Pass.as_str().to_string(),
            blocker_codes: Vec::new(),
            rework_target: None,
            allowed_next_node: "next".to_string(),
        }
    } else {
        let transition = host_bridge_quality_gate_transition(completed_target)
            .or_else(|| host_bridge_quality_gate_transition_for_blockers(blocker_codes));
        HostBridgeResultVerdictFields {
            decision: "rework_required".to_string(),
            verdict: "rework_required".to_string(),
            blocker_codes: blocker_codes.to_vec(),
            rework_target: Some(
                rework_target
                    .map(str::trim)
                    .filter(|target| !target.is_empty())
                    .or_else(|| transition.map(|transition| transition.rework_target))
                    .unwrap_or("developer")
                    .to_string(),
            ),
            allowed_next_node: transition
                .map(|transition| transition.blocked_allowed_next_node)
                .unwrap_or("developer_rework")
                .to_string(),
        }
    }
}

#[must_use]
pub fn host_bridge_result_verdict_contract_blockers(
    result: &Value,
    required_result_fields: &[String],
) -> Vec<String> {
    let required_fields = if required_result_fields.is_empty() {
        crate::request::default_host_bridge_required_result_fields()
    } else {
        required_result_fields.to_vec()
    };
    let mut blockers = Vec::new();
    for field in required_fields {
        if !result.get(field.as_str()).is_some_and(|value| {
            if field == "blocker_codes" {
                value.as_array().is_some()
            } else if field == "rework_target" {
                value.is_null()
                    || value
                        .as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
            } else {
                value
                    .as_str()
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
            }
        }) {
            push_unique_blocker(&mut blockers, "host_bridge_result_missing_verdict_field");
        }
    }

    let decision = result
        .get("decision")
        .and_then(Value::as_str)
        .map(str::trim);
    let verdict = result.get("verdict").and_then(Value::as_str).map(str::trim);
    let blocker_codes = result.get("blocker_codes").and_then(Value::as_array);
    if blocker_codes.is_some_and(|codes| {
        codes
            .iter()
            .any(|code| code.as_str().map(str::trim).is_none_or(str::is_empty))
    }) {
        push_unique_blocker(&mut blockers, "host_bridge_result_invalid_blocker_codes");
    }

    let pass_result = result
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status == Release1ContractStatus::Pass.as_str())
        && result
            .get("execution_state")
            .and_then(Value::as_str)
            .is_some_and(|state| state == "executed");
    let pass_verdict =
        decision == Some("approve") && verdict == Some(Release1ContractStatus::Pass.as_str());
    if pass_result {
        if !pass_verdict {
            push_unique_blocker(
                &mut blockers,
                "host_bridge_result_decision_verdict_mismatch",
            );
        }
        if blocker_codes.is_some_and(|codes| !codes.is_empty()) {
            push_unique_blocker(&mut blockers, "host_bridge_result_blocker_codes_mismatch");
        }
    }

    let blocked_result = result
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status == Release1ContractStatus::Blocked.as_str())
        && result
            .get("execution_state")
            .and_then(Value::as_str)
            .is_some_and(|state| state == Release1ContractStatus::Blocked.as_str());
    let rework_verdict = matches!(decision, Some("rework_required") | Some("blocked"))
        || matches!(verdict, Some("rework_required") | Some("blocked"));
    if pass_verdict && !pass_result {
        push_unique_blocker(
            &mut blockers,
            "host_bridge_result_decision_verdict_mismatch",
        );
    }
    if blocked_result && !rework_verdict {
        push_unique_blocker(
            &mut blockers,
            "host_bridge_result_decision_verdict_mismatch",
        );
    }
    if rework_verdict {
        if !blocked_result {
            push_unique_blocker(
                &mut blockers,
                "host_bridge_result_decision_verdict_mismatch",
            );
        }
        if blocker_codes.is_none_or(|codes| codes.is_empty()) {
            push_unique_blocker(&mut blockers, "host_bridge_result_blocker_codes_missing");
        }
        if result
            .get("rework_target")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            push_unique_blocker(&mut blockers, "host_bridge_result_rework_target_missing");
        }
    }
    blockers
}

fn push_unique_blocker(blockers: &mut Vec<String>, blocker: &str) {
    if !blockers.iter().any(|value| value == blocker) {
        blockers.push(blocker.to_string());
    }
}

#[must_use]
pub fn host_bridge_request_status_after_completion(blocker_codes: &[String]) -> String {
    if blocker_codes.is_empty() {
        Release1ContractStatus::Pass.as_str().to_string()
    } else if blocker_codes
        .iter()
        .all(|blocker| host_bridge_completion_retryable_blocker(blocker))
    {
        "retryable_blocked".to_string()
    } else {
        Release1ContractStatus::Blocked.as_str().to_string()
    }
}

#[must_use]
pub fn host_bridge_completion_requires_implementation_artifacts(dispatch_target: &str) -> bool {
    matches!(dispatch_target.trim(), "implementer" | "implementation")
}

#[must_use]
pub fn host_bridge_request_artifacts_are_bare_completion_candidates(
    request_artifacts: &Value,
) -> bool {
    let Some(rows) = request_artifacts.as_array() else {
        return false;
    };
    !rows.is_empty()
        && rows.iter().all(|artifact| {
            let Some(object) = artifact.as_object() else {
                return false;
            };
            let receipt_backed = object
                .get("receipt_backed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let freshness = object
                .get("freshness")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let consolidation_receipt_id = object
                .get("consolidation_receipt_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            !receipt_backed && freshness.is_none() && consolidation_receipt_id.is_none()
        })
}

#[must_use]
pub fn host_bridge_completion_authorized_request_artifacts(
    request_artifacts: &Value,
    task_updated_at: &str,
    completion_receipt_id: &str,
) -> Value {
    let mut artifacts = request_artifacts.clone();
    if let Some(rows) = artifacts.as_array_mut() {
        for artifact in rows.iter_mut() {
            if let Some(object) = artifact.as_object_mut() {
                object.insert("freshness".to_string(), serde_json::json!(task_updated_at));
                object.insert("receipt_backed".to_string(), serde_json::json!(true));
                object.insert(
                    "consolidation_receipt_id".to_string(),
                    serde_json::json!(completion_receipt_id),
                );
            }
        }
    }
    artifacts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::HostBridgeProvenanceDecision;
    use crate::receipt_binding::DispatchReceiptBindingDecision;
    use crate::tests::minimal_request;

    #[test]
    fn completion_evidence_is_blocked_when_receipt_binding_rejected() {
        let evidence = materialize_host_bridge_completion_evidence(&HostBridgeCompletionInput {
            request: minimal_request(),
            provenance: HostBridgeProvenanceDecision {
                accepted: true,
                blocker_codes: Vec::new(),
                reason: "ok".to_string(),
            },
            receipt_binding: DispatchReceiptBindingDecision {
                accepted: false,
                blocker_codes: vec!["missing_dispatch_receipt".to_string()],
                reason: "blocked".to_string(),
            },
            artifact_refs: Vec::new(),
        });

        assert_eq!(evidence.status, "blocked");
        assert!(!evidence.completion_ready);
    }

    #[test]
    fn retryable_completion_status_normalizes_only_status_blocker() {
        let provenance = HostBridgeProvenanceDecision {
            accepted: false,
            blocker_codes: vec![
                "request_status_not_admissible".to_string(),
                "dispatch_target_mismatch".to_string(),
            ],
            reason: "blocked".to_string(),
        };

        let normalized = normalize_host_bridge_provenance_for_completion(&provenance, true);

        assert!(!normalized.accepted);
        assert_eq!(
            normalized.blocker_codes,
            vec!["dispatch_target_mismatch".to_string()]
        );
    }

    #[test]
    fn retryable_blocker_detection_accepts_known_completion_blocker() {
        let artifact = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["implementation_artifacts_missing"]
        });

        assert!(host_bridge_artifact_has_retryable_completion_blocker(
            &artifact
        ));
        assert!(host_bridge_request_status_allows_parent_completion(
            "blocked", true
        ));
        assert!(!host_bridge_request_status_allows_parent_completion(
            "blocked", false
        ));
    }

    #[test]
    fn completion_verdict_maps_blockers_to_blocked_execution() {
        let verdict = host_bridge_completion_verdict(&["rework".to_string()]);

        assert_eq!(verdict.status, "blocked");
        assert_eq!(verdict.execution_state, "blocked");
        assert_eq!(verdict.completion_verdict, "rework_required");
        assert!(!verdict.completion_ready);
        assert_eq!(
            host_bridge_request_status_after_completion(&[
                "implementation_artifacts_missing".to_string()
            ]),
            "retryable_blocked"
        );
        assert_eq!(
            host_bridge_request_status_after_completion(&[
                "host_agent_execution_failed".to_string()
            ]),
            "blocked"
        );
    }

    #[test]
    fn result_verdict_contract_rejects_missing_quality_gate_fields() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed"
        });

        assert_eq!(
            host_bridge_result_verdict_contract_blockers(
                &result,
                &crate::request::default_host_bridge_required_result_fields(),
            ),
            vec!["host_bridge_result_missing_verdict_field".to_string()]
        );
    }

    #[test]
    fn result_verdict_contract_accepts_pass_and_rework_shapes() {
        let pass_fields = host_bridge_result_verdict_fields(&[], None);
        let pass_result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": pass_fields.decision,
            "verdict": pass_fields.verdict,
            "blocker_codes": pass_fields.blocker_codes,
            "rework_target": pass_fields.rework_target,
            "allowed_next_node": pass_fields.allowed_next_node
        });
        assert_eq!(
            host_bridge_result_verdict_contract_blockers(
                &pass_result,
                &crate::request::default_host_bridge_required_result_fields(),
            ),
            Vec::<String>::new()
        );

        let rework_fields = host_bridge_result_verdict_fields(
            &["coach_rework_required".to_string()],
            Some("developer"),
        );
        let rework_result = serde_json::json!({
            "status": "blocked",
            "execution_state": "blocked",
            "execution_evidence": {
                "receipt_backed": true
            },
            "decision": rework_fields.decision,
            "verdict": rework_fields.verdict,
            "blocker_codes": rework_fields.blocker_codes,
            "rework_target": rework_fields.rework_target,
            "allowed_next_node": rework_fields.allowed_next_node
        });
        assert_eq!(
            host_bridge_result_verdict_contract_blockers(
                &rework_result,
                &crate::request::default_host_bridge_required_result_fields(),
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn quality_gate_transition_matrix_routes_pass_and_blocked_decisions() {
        let cases = [
            (
                "coach",
                "coach decision=blocked; implementation acceptance gap",
                "coach_rework_required",
                "developer",
                "developer_rework",
            ),
            (
                "tester",
                "tester decision=blocked; focused proof failed",
                "verification_rework_required",
                "developer",
                "developer_rework",
            ),
            (
                "reviewer",
                "reviewer decision=blocked; proof review needs tester rework",
                "review_rework_required",
                "tester",
                "tester",
            ),
        ];

        for (gate, blocked_summary, blocker_code, rework_target, allowed_next_node) in cases {
            let pass_fields = host_bridge_result_verdict_fields_for_gate(gate, &[], None);
            assert_eq!(pass_fields.decision, "approve", "{gate}");
            assert_eq!(pass_fields.verdict, "pass", "{gate}");
            assert_eq!(pass_fields.blocker_codes, Vec::<String>::new(), "{gate}");
            assert_eq!(pass_fields.rework_target, None, "{gate}");
            assert_eq!(pass_fields.allowed_next_node, "next", "{gate}");

            assert_eq!(
                host_bridge_lane_completion_summary_blocker_code(gate, Some(blocked_summary)),
                Some(blocker_code.to_string()),
                "{gate}"
            );
            let blocked_fields =
                host_bridge_result_verdict_fields_for_gate(gate, &[blocker_code.to_string()], None);
            assert_eq!(blocked_fields.decision, "rework_required", "{gate}");
            assert_eq!(blocked_fields.verdict, "rework_required", "{gate}");
            assert_eq!(
                blocked_fields.blocker_codes,
                vec![blocker_code.to_string()],
                "{gate}"
            );
            assert_eq!(
                blocked_fields.rework_target,
                Some(rework_target.to_string()),
                "{gate}"
            );
            assert_eq!(
                blocked_fields.allowed_next_node, allowed_next_node,
                "{gate}"
            );
        }
    }

    #[test]
    fn result_verdict_contract_rejects_malformed_mismatch_states() {
        let required_fields = crate::request::default_host_bridge_required_result_fields();
        let cases = [
            (
                serde_json::json!({
                    "status": "pass",
                    "execution_state": "blocked",
                    "decision": "approve",
                    "verdict": "pass",
                    "blocker_codes": [],
                    "rework_target": null,
                    "allowed_next_node": "next"
                }),
                "host_bridge_result_decision_verdict_mismatch",
            ),
            (
                serde_json::json!({
                    "status": "blocked",
                    "execution_state": "blocked",
                    "decision": "approve",
                    "verdict": "pass",
                    "blocker_codes": [],
                    "rework_target": null,
                    "allowed_next_node": "next"
                }),
                "host_bridge_result_decision_verdict_mismatch",
            ),
            (
                serde_json::json!({
                    "status": "pass",
                    "execution_state": "executed",
                    "decision": "rework_required",
                    "verdict": "pass",
                    "blocker_codes": [],
                    "rework_target": null,
                    "allowed_next_node": "next"
                }),
                "host_bridge_result_decision_verdict_mismatch",
            ),
            (
                serde_json::json!({
                    "status": "pass",
                    "execution_state": "executed",
                    "decision": "approve",
                    "verdict": "pass",
                    "blocker_codes": ["coach_rework_required"],
                    "rework_target": null,
                    "allowed_next_node": "next"
                }),
                "host_bridge_result_blocker_codes_mismatch",
            ),
            (
                serde_json::json!({
                    "status": "blocked",
                    "execution_state": "blocked",
                    "execution_evidence": {
                        "receipt_backed": true
                    },
                    "decision": "rework_required",
                    "verdict": "rework_required",
                    "blocker_codes": [],
                    "rework_target": "developer",
                    "allowed_next_node": "developer_rework"
                }),
                "host_bridge_result_blocker_codes_missing",
            ),
            (
                serde_json::json!({
                    "status": "blocked",
                    "execution_state": "blocked",
                    "execution_evidence": {
                        "receipt_backed": true
                    },
                    "decision": "rework_required",
                    "verdict": "rework_required",
                    "blocker_codes": ["coach_rework_required"],
                    "rework_target": null,
                    "allowed_next_node": "developer_rework"
                }),
                "host_bridge_result_rework_target_missing",
            ),
        ];

        for (result, expected_blocker) in cases {
            let blockers = host_bridge_result_verdict_contract_blockers(&result, &required_fields);
            assert!(
                blockers.iter().any(|blocker| blocker == expected_blocker),
                "expected blocker `{expected_blocker}` for result {result}, got {blockers:?}"
            );
        }
    }

    #[test]
    fn summary_classifier_ignores_positive_receipt_blocker_context() {
        let summary = "verifier proof passed focused host-bridge tests and confirmed pending receipt was the only closure blocker";

        assert_eq!(
            host_bridge_lane_completion_summary_blocker_code("verification", Some(summary)),
            None
        );
    }

    #[test]
    fn summary_classifier_keeps_explicit_blocker_verdicts() {
        let summary = "verdict: blocker; rework required; product implementation evidence missing";

        assert_eq!(
            host_bridge_lane_completion_summary_blocker_code("verification", Some(summary)),
            Some("verification_rework_required".to_string())
        );
    }

    #[test]
    fn summary_classifier_preserves_coach_decision_when_target_is_stale() {
        let summary = "coach decision=blocked; scheduledAt missing for non-all-day meeting";

        assert_eq!(
            host_bridge_lane_completion_summary_blocker_code("implementer", Some(summary)),
            Some("coach_rework_required".to_string())
        );
    }

    #[test]
    fn completed_artifact_admissibility_is_shared() {
        assert!(host_bridge_existing_request_status_is_admissible("pending"));
        assert!(host_bridge_existing_request_status_is_admissible(
            "completed"
        ));
        assert!(!host_bridge_existing_request_status_is_admissible(
            "blocked"
        ));
        assert!(host_bridge_completed_artifact_status_is_admissible("pass"));
        assert!(host_bridge_completed_result_status_is_admissible("blocked"));
        assert!(host_bridge_completed_result_execution_state_is_admissible(
            "executed"
        ));
        assert!(host_bridge_completed_result_execution_state_is_admissible(
            "blocked"
        ));
    }

    #[test]
    fn implementation_artifacts_are_required_only_for_implementation_targets() {
        assert!(host_bridge_completion_requires_implementation_artifacts(
            "implementer"
        ));
        assert!(host_bridge_completion_requires_implementation_artifacts(
            " implementation "
        ));
        assert!(!host_bridge_completion_requires_implementation_artifacts(
            "verification"
        ));
    }

    #[test]
    fn bare_request_artifacts_are_completion_authorizable() {
        let request_artifacts = serde_json::json!([
            {
                "artifact_path": ".vida/data/state/artifacts/impl.json",
                "changed_files": ["crates/vida/src/lane_surface.rs"]
            }
        ]);

        assert!(host_bridge_request_artifacts_are_bare_completion_candidates(&request_artifacts));

        let authorized = host_bridge_completion_authorized_request_artifacts(
            &request_artifacts,
            "2026-06-13T19:00:00Z",
            "receipt-123",
        );
        let row = authorized
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(Value::as_object)
            .expect("authorized artifact row");

        assert_eq!(
            row.get("freshness").and_then(Value::as_str),
            Some("2026-06-13T19:00:00Z")
        );
        assert_eq!(
            row.get("consolidation_receipt_id").and_then(Value::as_str),
            Some("receipt-123")
        );
        assert_eq!(
            row.get("receipt_backed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn stamped_request_artifacts_are_not_bare_completion_candidates() {
        let request_artifacts = serde_json::json!([
            {
                "artifact_path": ".vida/data/state/artifacts/impl.json",
                "receipt_backed": true
            }
        ]);

        assert!(!host_bridge_request_artifacts_are_bare_completion_candidates(&request_artifacts));
        assert!(
            !host_bridge_request_artifacts_are_bare_completion_candidates(&serde_json::json!([]))
        );
        assert!(
            !host_bridge_request_artifacts_are_bare_completion_candidates(
                &serde_json::json!({"artifact_path": "impl.json"})
            )
        );
    }
}
