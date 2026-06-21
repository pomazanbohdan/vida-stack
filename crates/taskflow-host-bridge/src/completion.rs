use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taskflow_contracts::{BlockerCode, Release1ContractStatus, release1_contract_status_str};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeResultContractDecision {
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub detail_blocker_codes: Vec<String>,
}

pub const HOST_AGENT_EXECUTION_RESULT_V2_SCHEMA_VERSION: &str = "host-agent-result-v2";
pub const HOST_AGENT_EXECUTION_RECEIPT_V2_SCHEMA_VERSION: &str = "host-agent-receipt-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostAgentExecutionResultV2 {
    pub artifact_kind: String,
    pub schema_version: String,
    pub request_id: String,
    pub run_id: String,
    pub task_id: String,
    pub dispatch_generation_id: String,
    pub lane_id: String,
    pub dispatch_target: String,
    pub execution_state: String,
    pub outcome: String,
    pub blocker_codes: Vec<String>,
    pub host_agent_id: Option<String>,
    pub backend_id: String,
    pub carrier_id: String,
    pub adapter_kind: String,
    pub adapter_capability_id: String,
    pub packet_path: String,
    pub packet_hash_blake3: String,
    pub summary: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostAgentExecutionReceiptV2 {
    pub artifact_kind: String,
    pub schema_version: String,
    pub receipt_id: String,
    pub request_id: String,
    pub run_id: String,
    pub task_id: String,
    pub dispatch_generation_id: String,
    pub lane_id: String,
    pub dispatch_target: String,
    pub result_path: String,
    pub result_hash_blake3: String,
    pub packet_hash_blake3: String,
    pub host_agent_id: String,
    pub adapter_kind: String,
    pub adapter_capability_id: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostAgentEvidenceContractDecision {
    pub accepted: bool,
    pub receipt_backed: bool,
    pub blocker_codes: Vec<String>,
    pub detail_blocker_codes: Vec<String>,
}

#[must_use]
pub fn host_agent_execution_result_v2_canonical_hash(result: &Value) -> String {
    let encoded = serde_json::to_vec(result).unwrap_or_default();
    blake3::hash(&encoded).to_hex().to_string()
}

#[must_use]
pub fn host_agent_execution_result_v2_contract_decision(
    result: &Value,
) -> HostAgentEvidenceContractDecision {
    let mut blockers = Vec::new();
    if string_field(result, "artifact_kind").as_deref() != Some("host_agent_execution_result") {
        push_unique_blocker(&mut blockers, "host_agent_result_artifact_kind_invalid");
    }
    if string_field(result, "schema_version").as_deref()
        != Some(HOST_AGENT_EXECUTION_RESULT_V2_SCHEMA_VERSION)
    {
        push_unique_blocker(&mut blockers, "host_agent_result_schema_version_invalid");
    }
    for field in [
        "request_id",
        "run_id",
        "task_id",
        "dispatch_generation_id",
        "lane_id",
        "dispatch_target",
        "execution_state",
        "outcome",
        "backend_id",
        "carrier_id",
        "adapter_kind",
        "adapter_capability_id",
        "packet_path",
        "packet_hash_blake3",
        "summary",
        "completed_at",
    ] {
        if string_field(result, field).is_none() {
            push_unique_blocker(
                &mut blockers,
                &format!("missing_host_agent_result_field_{field}"),
            );
        }
    }
    let host_agent_id = string_field(result, "host_agent_id");
    if string_field(result, "execution_state").as_deref() == Some("executed")
        && host_agent_id.is_none()
    {
        push_unique_blocker(&mut blockers, BlockerCode::HostAgentIdMissing.as_str());
    }
    if !matches!(
        string_field(result, "execution_state").as_deref(),
        Some("executed" | "not_started" | "failed" | "timed_out") | None
    ) {
        push_unique_blocker(&mut blockers, "host_agent_result_execution_state_invalid");
    }
    if !matches!(
        string_field(result, "outcome").as_deref(),
        Some("pass" | "rework_required" | "blocked") | None
    ) {
        push_unique_blocker(&mut blockers, "host_agent_result_outcome_invalid");
    }
    if result
        .get("blocker_codes")
        .and_then(Value::as_array)
        .is_none()
    {
        push_unique_blocker(
            &mut blockers,
            "missing_host_agent_result_field_blocker_codes",
        );
    } else if result
        .get("blocker_codes")
        .and_then(Value::as_array)
        .is_some_and(|codes| {
            codes
                .iter()
                .any(|code| code.as_str().map(str::trim).is_none_or(str::is_empty))
        })
    {
        push_unique_blocker(&mut blockers, "host_agent_result_blocker_codes_invalid");
    }
    if result.get("receipt_backed").is_some() {
        push_unique_blocker(
            &mut blockers,
            "host_agent_result_self_declared_receipt_backed",
        );
    }
    if string_field(result, "status")
        .as_deref()
        .is_some_and(|status| matches!(status, "pass" | "rework_required" | "blocked"))
    {
        push_unique_blocker(
            &mut blockers,
            "host_agent_result_status_must_not_be_workflow_outcome",
        );
    }
    host_agent_evidence_decision_from_detail_blockers(blockers, false)
}

#[must_use]
pub fn host_agent_execution_receipt_v2_contract_decision(
    receipt: &Value,
) -> HostAgentEvidenceContractDecision {
    let mut blockers = Vec::new();
    if string_field(receipt, "artifact_kind").as_deref() != Some("host_agent_execution_receipt") {
        push_unique_blocker(&mut blockers, "host_agent_receipt_artifact_kind_invalid");
    }
    if string_field(receipt, "schema_version").as_deref()
        != Some(HOST_AGENT_EXECUTION_RECEIPT_V2_SCHEMA_VERSION)
    {
        push_unique_blocker(&mut blockers, "host_agent_receipt_schema_version_invalid");
    }
    for field in [
        "receipt_id",
        "request_id",
        "run_id",
        "task_id",
        "dispatch_generation_id",
        "lane_id",
        "dispatch_target",
        "result_path",
        "result_hash_blake3",
        "packet_hash_blake3",
        "host_agent_id",
        "adapter_kind",
        "adapter_capability_id",
        "recorded_at",
    ] {
        if string_field(receipt, field).is_none() {
            push_unique_blocker(
                &mut blockers,
                &format!("missing_host_agent_receipt_field_{field}"),
            );
        }
    }
    if receipt.get("receipt_backed").is_some() {
        push_unique_blocker(
            &mut blockers,
            "host_agent_receipt_self_declared_receipt_backed",
        );
    }
    host_agent_evidence_decision_from_detail_blockers(blockers, false)
}

#[must_use]
pub fn host_agent_execution_evidence_v2_contract_decision(
    result: &Value,
    receipt: Option<&Value>,
) -> HostAgentEvidenceContractDecision {
    let result_decision = host_agent_execution_result_v2_contract_decision(result);
    let Some(receipt) = receipt else {
        return result_decision;
    };
    let receipt_decision = host_agent_execution_receipt_v2_contract_decision(receipt);
    let mut blockers = result_decision.detail_blocker_codes;
    for blocker in receipt_decision.detail_blocker_codes {
        push_unique_blocker(&mut blockers, &blocker);
    }
    for field in [
        "request_id",
        "run_id",
        "task_id",
        "dispatch_generation_id",
        "lane_id",
        "dispatch_target",
        "packet_hash_blake3",
        "host_agent_id",
        "adapter_kind",
        "adapter_capability_id",
    ] {
        if let (Some(result_value), Some(receipt_value)) =
            (string_field(result, field), string_field(receipt, field))
            && result_value != receipt_value
        {
            push_unique_blocker(
                &mut blockers,
                &format!("host_agent_evidence_identity_mismatch_{field}"),
            );
        }
    }
    if let Some(receipt_result_hash) = string_field(receipt, "result_hash_blake3") {
        let computed_hash = host_agent_execution_result_v2_canonical_hash(result);
        if receipt_result_hash != computed_hash {
            push_unique_blocker(&mut blockers, "host_agent_evidence_result_hash_mismatch");
        }
    }
    let receipt_backed = blockers.is_empty();
    host_agent_evidence_decision_from_detail_blockers(blockers, receipt_backed)
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn host_agent_evidence_decision_from_detail_blockers(
    detail_blocker_codes: Vec<String>,
    receipt_backed: bool,
) -> HostAgentEvidenceContractDecision {
    if detail_blocker_codes.is_empty() {
        HostAgentEvidenceContractDecision {
            accepted: true,
            receipt_backed,
            blocker_codes: Vec::new(),
            detail_blocker_codes,
        }
    } else {
        let mut blocker_codes = vec![
            BlockerCode::HostBridgeResultContractInvalid
                .as_str()
                .to_string(),
        ];
        if detail_blocker_codes
            .iter()
            .any(|blocker| blocker == BlockerCode::HostAgentIdMissing.as_str())
        {
            blocker_codes.push(BlockerCode::HostAgentIdMissing.as_str().to_string());
        }
        HostAgentEvidenceContractDecision {
            accepted: false,
            receipt_backed: false,
            blocker_codes,
            detail_blocker_codes,
        }
    }
}

#[must_use]
pub fn host_bridge_result_pass_allowed_next_node(completed_target: &str) -> &'static str {
    match completed_target.trim() {
        "developer" | "developer_rework" | "implementer" | "implementation" => "coach",
        "coach"
        | "coach_lane"
        | "coach_test_gate"
        | "coach_implementation_gate"
        | "coach_validator" => "tester",
        "tester" | "tester_lane" | "verification" | "verification_lane" | "verifier"
        | "verifier_lane" => "reviewer",
        "reviewer"
        | "reviewer_lane"
        | "review"
        | "review_lane"
        | "duplication_reviewer"
        | "closure"
        | "closure_lane" => "terminal_closure",
        _ => "next",
    }
}

fn host_bridge_next_node_family(next_node: &str) -> &str {
    match next_node.trim() {
        "coach"
        | "coach_lane"
        | "coach_test_gate"
        | "coach_implementation_gate"
        | "coach_validator" => "coach",
        "tester" | "tester_lane" | "verification" | "verification_lane" | "verifier"
        | "verifier_lane" => "tester",
        "reviewer" | "reviewer_lane" | "review" | "review_lane" | "duplication_reviewer" => {
            "reviewer"
        }
        "terminal_closure" | "closure" | "closure_lane" => "terminal_closure",
        "developer" | "developer_rework" | "implementer" | "implementation" => "developer",
        "next" | "next_lane" | "dispatch.next" | "dispatch.next_lane" => "next",
        value => value,
    }
}

#[must_use]
pub fn host_bridge_allowed_next_node_is_abstract_next(allowed_next_node: &str) -> bool {
    matches!(
        allowed_next_node.trim().replace('-', "_").as_str(),
        "next" | "next_lane" | "dispatch.next" | "dispatch.next_lane"
    )
}

fn host_bridge_allowed_next_node_matches_expected(expected: &str, allowed_next_node: &str) -> bool {
    expected.trim() == allowed_next_node.trim()
        || host_bridge_next_node_family(expected) == host_bridge_next_node_family(allowed_next_node)
}

#[must_use]
pub fn host_bridge_result_decision_is_pass(decision: &str) -> bool {
    matches!(decision.trim(), "pass" | "approve" | "approved")
}

#[must_use]
pub fn host_bridge_result_verdict_is_pass(verdict: &str) -> bool {
    matches!(verdict.trim(), "pass" | "implemented")
}

#[must_use]
pub fn host_bridge_result_decision_is_blocked(decision: &str) -> bool {
    matches!(
        decision.trim(),
        "blocked" | "blocker" | "rework" | "rework_required"
    )
}

#[must_use]
pub fn host_bridge_result_verdict_is_blocked(verdict: &str) -> bool {
    matches!(
        verdict.trim(),
        "blocked" | "blocker" | "rework" | "rework_required"
    )
}

#[must_use]
pub fn host_bridge_result_verdict_fields_from_typed_result(
    result: &Value,
    completed_target: &str,
) -> HostBridgeResultVerdictFields {
    let decision = result
        .get("decision")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("blocked");
    let verdict = result
        .get("verdict")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("blocked");
    let blocker_codes = result
        .get("blocker_codes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let rework_target = result
        .get("rework_target")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let allowed_next_node = result
        .get("allowed_next_node")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if host_bridge_result_decision_is_pass(decision)
                && host_bridge_result_verdict_is_pass(verdict)
            {
                host_bridge_result_pass_allowed_next_node(completed_target).to_string()
            } else {
                host_bridge_quality_gate_transition(completed_target)
                    .map(|transition| transition.blocked_allowed_next_node)
                    .unwrap_or("developer_rework")
                    .to_string()
            }
        });

    HostBridgeResultVerdictFields {
        decision: decision.to_string(),
        verdict: verdict.to_string(),
        blocker_codes,
        rework_target,
        allowed_next_node,
    }
}

#[must_use]
pub fn host_bridge_completion_command_verdict_fields(
    result: Option<&Value>,
    completed_target: &str,
    packet_authorized_next_target: Option<&str>,
) -> HostBridgeResultVerdictFields {
    let result_has_allowed_next_node = result
        .and_then(|result| result.get("allowed_next_node"))
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let mut fields = result
        .map(|result| host_bridge_result_verdict_fields_from_typed_result(result, completed_target))
        .unwrap_or_else(|| {
            let mut fields =
                host_bridge_result_verdict_fields_for_gate(completed_target, &[], None);
            fields.decision = "pass".to_string();
            fields.verdict = "implemented".to_string();
            fields
        });

    if !result_has_allowed_next_node
        && host_bridge_result_decision_is_pass(&fields.decision)
        && host_bridge_result_verdict_is_pass(&fields.verdict)
        && let Some(next_target) = packet_authorized_next_target
            .map(str::trim)
            .filter(|value| !value.is_empty())
    {
        fields.allowed_next_node = next_target.to_string();
    }

    fields
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
        .find(|(aliases, _)| aliases.contains(&normalized))
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
            | "invalid_allowed_next_node"
            | "verification_rework_required"
            | "coach_rework_required"
            | "review_rework_required"
            | "closure_evidence_blocked"
            | "host_bridge_request_task_mismatch"
            | "host_bridge_result_blocker_codes_missing"
            | "host_bridge_result_blocker_codes_mismatch"
            | "host_bridge_result_decision_verdict_mismatch"
            | "host_bridge_result_invalid_blocker_codes"
            | "host_bridge_result_missing_verdict_field"
            | "host_bridge_result_rework_target_missing"
            | "host_agent_execution_failed"
    ) || matches!(
        taskflow_contracts::BlockerCode::try_from(blocker_code),
        Ok(taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactChangedFilesMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityInvalid)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactContractInvalid)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactReceiptMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactReceiptUnverified)
            | Ok(taskflow_contracts::BlockerCode::ImplementationArtifactsMissing)
            | Ok(taskflow_contracts::BlockerCode::ImplementationAttemptScopeGuardViolation)
    ) || blocker_code.starts_with("missing_required_result_field_")
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
        "rework_target",
        "rework target",
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
            allowed_next_node: host_bridge_result_pass_allowed_next_node(completed_target)
                .to_string(),
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
    host_bridge_result_verdict_contract_blockers_for_target(result, required_result_fields, "")
}

#[must_use]
pub fn host_bridge_result_verdict_contract_blockers_for_target(
    result: &Value,
    required_result_fields: &[String],
    completed_target: &str,
) -> Vec<String> {
    host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
        result,
        required_result_fields,
        completed_target,
        None,
    )
}

#[must_use]
pub fn host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
    result: &Value,
    required_result_fields: &[String],
    completed_target: &str,
    authorized_next_target: Option<&str>,
) -> Vec<String> {
    let required_fields = crate::request::canonical_host_bridge_required_result_fields(
        required_result_fields.to_vec(),
    );
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
            push_unique_blocker(
                &mut blockers,
                &missing_required_result_field_blocker(&field),
            );
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
    let missing_required_result_fields = blockers
        .iter()
        .any(|blocker| blocker.starts_with("missing_required_result_field_"));

    let pass_result = result
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status == Release1ContractStatus::Pass.as_str())
        && result
            .get("execution_state")
            .and_then(Value::as_str)
            .is_some_and(|state| state == "executed");
    let pass_verdict = decision.is_some_and(host_bridge_result_decision_is_pass)
        && verdict.is_some_and(host_bridge_result_verdict_is_pass);
    if pass_result {
        if !pass_verdict && !missing_required_result_fields {
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
    let rework_verdict = decision.is_some_and(host_bridge_result_decision_is_blocked)
        || verdict.is_some_and(host_bridge_result_verdict_is_blocked);
    if pass_verdict && !pass_result {
        push_unique_blocker(
            &mut blockers,
            "host_bridge_result_decision_verdict_mismatch",
        );
    }
    if blocked_result && !rework_verdict && !missing_required_result_fields {
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
    if let Some(allowed_next_node) = result
        .get("allowed_next_node")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|_| !missing_required_result_fields)
    {
        let authorized_next_target = authorized_next_target
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let expected = if pass_verdict {
            authorized_next_target
                .or_else(|| Some(host_bridge_result_pass_allowed_next_node(completed_target)))
        } else if rework_verdict {
            host_bridge_quality_gate_transition(completed_target)
                .or_else(|| {
                    blocker_codes
                        .map(|codes| {
                            codes
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .and_then(|codes| host_bridge_quality_gate_transition_for_blockers(&codes))
                })
                .map(|transition| transition.blocked_allowed_next_node)
        } else {
            None
        };
        if let Some(expected) = expected {
            if pass_verdict
                && authorized_next_target.is_some()
                && host_bridge_allowed_next_node_is_abstract_next(allowed_next_node)
            {
                // Compatibility input only. Runtime callers with compiled-flow
                // context must persist the concrete resolved node instead.
            } else if expected != "next"
                && !host_bridge_allowed_next_node_matches_expected(expected, allowed_next_node)
            {
                push_unique_blocker(
                    &mut blockers,
                    if pass_verdict && authorized_next_target.is_some() {
                        BlockerCode::HostBridgeResultTransitionMismatch.as_str()
                    } else {
                        "invalid_allowed_next_node"
                    },
                );
            }
        }
    }
    blockers
}

#[must_use]
pub fn host_bridge_result_contract_decision_for_target(
    result: &Value,
    required_result_fields: &[String],
    completed_target: &str,
) -> HostBridgeResultContractDecision {
    host_bridge_result_contract_decision_for_target_with_authorized_next(
        result,
        required_result_fields,
        completed_target,
        None,
    )
}

#[must_use]
pub fn host_bridge_result_contract_decision_for_target_with_authorized_next(
    result: &Value,
    required_result_fields: &[String],
    completed_target: &str,
    authorized_next_target: Option<&str>,
) -> HostBridgeResultContractDecision {
    let detail_blocker_codes =
        host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
            result,
            required_result_fields,
            completed_target,
            authorized_next_target,
        );
    if detail_blocker_codes.is_empty() {
        HostBridgeResultContractDecision {
            accepted: true,
            blocker_codes: Vec::new(),
            detail_blocker_codes,
        }
    } else {
        HostBridgeResultContractDecision {
            accepted: false,
            blocker_codes: vec![
                BlockerCode::HostBridgeResultContractInvalid
                    .as_str()
                    .to_string(),
            ],
            detail_blocker_codes,
        }
    }
}

fn missing_required_result_field_blocker(field: &str) -> String {
    let suffix = field
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("missing_required_result_field_{suffix}")
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
    host_bridge_request_requires_implementation_artifacts(dispatch_target, None)
}

#[must_use]
pub fn host_bridge_request_requires_implementation_artifacts(
    dispatch_target: &str,
    task_class: Option<&str>,
) -> bool {
    matches!(dispatch_target.trim(), "implementer" | "implementation")
        || task_class.map(str::trim).is_some_and(|value| {
            matches!(
                value,
                "implementation" | "delivery_task" | "execution_block" | "writer"
            )
        })
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

    fn sample_host_agent_result_v2() -> Value {
        serde_json::json!({
            "artifact_kind": "host_agent_execution_result",
            "schema_version": HOST_AGENT_EXECUTION_RESULT_V2_SCHEMA_VERSION,
            "request_id": "request-1",
            "run_id": "run-1",
            "task_id": "task-1",
            "dispatch_generation_id": "generation-1",
            "lane_id": "coder_lane",
            "dispatch_target": "coder",
            "execution_state": "executed",
            "outcome": "pass",
            "blocker_codes": [],
            "host_agent_id": "host-agent-1",
            "backend_id": "internal_subagents",
            "carrier_id": "senior",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "packet_path": ".vida/data/state/runtime-consumption/dispatch-packets/run-1.json",
            "packet_hash_blake3": "packet-hash-1",
            "summary": "implemented",
            "completed_at": "2026-06-20T19:00:00Z"
        })
    }

    fn sample_host_agent_receipt_v2(result: &Value) -> Value {
        serde_json::json!({
            "artifact_kind": "host_agent_execution_receipt",
            "schema_version": HOST_AGENT_EXECUTION_RECEIPT_V2_SCHEMA_VERSION,
            "receipt_id": "receipt-1",
            "request_id": result["request_id"],
            "run_id": result["run_id"],
            "task_id": result["task_id"],
            "dispatch_generation_id": result["dispatch_generation_id"],
            "lane_id": result["lane_id"],
            "dispatch_target": result["dispatch_target"],
            "result_path": ".vida/data/state/runtime-consumption/dispatch-results/run-1.json",
            "result_hash_blake3": host_agent_execution_result_v2_canonical_hash(result),
            "packet_hash_blake3": result["packet_hash_blake3"],
            "host_agent_id": result["host_agent_id"],
            "adapter_kind": result["adapter_kind"],
            "adapter_capability_id": result["adapter_capability_id"],
            "recorded_at": "2026-06-20T19:00:01Z"
        })
    }

    #[test]
    fn host_agent_execution_evidence_v2_accepts_identity_and_hash_matched_receipt() {
        let result = sample_host_agent_result_v2();
        let receipt = sample_host_agent_receipt_v2(&result);

        let decision = host_agent_execution_evidence_v2_contract_decision(&result, Some(&receipt));

        assert!(decision.accepted);
        assert!(decision.receipt_backed);
        assert!(decision.blocker_codes.is_empty());
        assert!(decision.detail_blocker_codes.is_empty());
    }

    #[test]
    fn host_agent_execution_result_v2_rejects_executed_without_host_agent_id() {
        let mut result = sample_host_agent_result_v2();
        result
            .as_object_mut()
            .expect("sample result is object")
            .remove("host_agent_id");

        let decision = host_agent_execution_result_v2_contract_decision(&result);

        assert!(!decision.accepted);
        assert!(!decision.receipt_backed);
        assert!(
            decision
                .blocker_codes
                .contains(&BlockerCode::HostAgentIdMissing.as_str().to_string())
        );
        assert!(
            decision
                .detail_blocker_codes
                .contains(&BlockerCode::HostAgentIdMissing.as_str().to_string())
        );
    }

    #[test]
    fn host_agent_execution_result_v2_rejects_self_declared_receipt_backed() {
        let mut result = sample_host_agent_result_v2();
        result["receipt_backed"] = serde_json::json!(true);

        let decision = host_agent_execution_result_v2_contract_decision(&result);

        assert!(!decision.accepted);
        assert!(!decision.receipt_backed);
        assert!(
            decision
                .detail_blocker_codes
                .contains(&"host_agent_result_self_declared_receipt_backed".to_string())
        );
    }

    #[test]
    fn host_agent_execution_evidence_v2_rejects_result_hash_mismatch() {
        let result = sample_host_agent_result_v2();
        let mut receipt = sample_host_agent_receipt_v2(&result);
        receipt["result_hash_blake3"] = serde_json::json!("wrong-hash");

        let decision = host_agent_execution_evidence_v2_contract_decision(&result, Some(&receipt));

        assert!(!decision.accepted);
        assert!(!decision.receipt_backed);
        assert!(
            decision
                .detail_blocker_codes
                .contains(&"host_agent_evidence_result_hash_mismatch".to_string())
        );
    }

    #[test]
    fn host_agent_execution_evidence_v2_rejects_identity_mismatch() {
        let result = sample_host_agent_result_v2();
        let mut receipt = sample_host_agent_receipt_v2(&result);
        receipt["run_id"] = serde_json::json!("other-run");

        let decision = host_agent_execution_evidence_v2_contract_decision(&result, Some(&receipt));

        assert!(!decision.accepted);
        assert!(!decision.receipt_backed);
        assert!(
            decision
                .detail_blocker_codes
                .contains(&"host_agent_evidence_identity_mismatch_run_id".to_string())
        );
    }

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
            "retryable_blocked"
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
            vec![
                "missing_required_result_field_decision".to_string(),
                "missing_required_result_field_verdict".to_string(),
                "missing_required_result_field_blocker_codes".to_string(),
                "missing_required_result_field_rework_target".to_string(),
                "missing_required_result_field_allowed_next_node".to_string()
            ]
        );
    }

    #[test]
    fn result_verdict_contract_rejects_request_schema_downgrade() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "allowed_next_node": "next"
        });
        let downgraded_required_fields = vec!["allowed_next_node".to_string()];

        assert_eq!(
            host_bridge_result_verdict_contract_blockers(&result, &downgraded_required_fields),
            vec![
                "missing_required_result_field_decision".to_string(),
                "missing_required_result_field_verdict".to_string(),
                "missing_required_result_field_blocker_codes".to_string(),
                "missing_required_result_field_rework_target".to_string()
            ]
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

        let typed_pass_result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": "coach"
        });
        assert_eq!(
            host_bridge_result_verdict_contract_blockers_for_target(
                &typed_pass_result,
                &crate::request::default_host_bridge_required_result_fields(),
                "developer",
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
    fn completion_command_verdict_fields_use_packet_next_target_for_default_pass() {
        let fields =
            host_bridge_completion_command_verdict_fields(None, "analyst", Some("designer"));

        assert_eq!(fields.decision, "pass");
        assert_eq!(fields.verdict, "implemented");
        assert_eq!(fields.blocker_codes, Vec::<String>::new());
        assert_eq!(fields.rework_target, None);
        assert_eq!(fields.allowed_next_node, "designer");
    }

    #[test]
    fn result_contract_accepts_authorized_flow_next_target() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": "cleaner"
        });

        assert_eq!(
            host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
                &result,
                &crate::request::default_host_bridge_required_result_fields(),
                "developer",
                Some("cleaner"),
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn result_contract_accepts_abstract_next_only_as_authorized_flow_alias() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": "next"
        });

        assert_eq!(
            host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
                &result,
                &crate::request::default_host_bridge_required_result_fields(),
                "developer",
                Some("cleaner"),
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn result_contract_rejects_wrong_concrete_authorized_flow_next_target() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": "qa"
        });

        assert_eq!(
            host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next(
                &result,
                &crate::request::default_host_bridge_required_result_fields(),
                "developer",
                Some("cleaner"),
            ),
            vec![
                BlockerCode::HostBridgeResultTransitionMismatch
                    .as_str()
                    .to_string()
            ]
        );
    }

    #[test]
    fn result_contract_decision_wraps_detail_blockers_in_canonical_invalid_code() {
        let invalid_result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": ["unexpected_blocker"],
            "rework_target": null,
            "allowed_next_node": "coach"
        });

        let decision = host_bridge_result_contract_decision_for_target(
            &invalid_result,
            &crate::request::default_host_bridge_required_result_fields(),
            "developer",
        );

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["host_bridge_result_contract_invalid".to_string()]
        );
        assert_eq!(
            decision.detail_blocker_codes,
            vec!["host_bridge_result_blocker_codes_mismatch".to_string()]
        );
    }

    #[test]
    fn completion_command_verdict_fields_preserve_typed_blocked_result() {
        let result = serde_json::json!({
            "status": "blocked",
            "execution_state": "blocked",
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["implementation_artifacts_missing"],
            "rework_target": "developer",
            "allowed_next_node": "developer_rework"
        });

        let fields = host_bridge_completion_command_verdict_fields(
            Some(&result),
            "implementer",
            Some("coach"),
        );

        assert_eq!(fields.decision, "rework_required");
        assert_eq!(fields.verdict, "rework_required");
        assert_eq!(
            fields.blocker_codes,
            vec!["implementation_artifacts_missing".to_string()]
        );
        assert_eq!(fields.rework_target, Some("developer".to_string()));
        assert_eq!(fields.allowed_next_node, "developer_rework");
    }

    #[test]
    fn quality_gate_transition_matrix_routes_pass_and_blocked_decisions() {
        let cases = [
            (
                "coach",
                "tester",
                "coach decision=blocked; implementation acceptance gap",
                "coach_rework_required",
                "developer",
                "developer_rework",
            ),
            (
                "tester",
                "reviewer",
                "tester decision=blocked; focused proof failed",
                "verification_rework_required",
                "developer",
                "developer_rework",
            ),
            (
                "reviewer",
                "terminal_closure",
                "reviewer decision=blocked; proof review needs tester rework",
                "review_rework_required",
                "tester",
                "tester",
            ),
            (
                "closure",
                "terminal_closure",
                "closure not ready",
                "closure_evidence_blocked",
                "developer",
                "developer_rework",
            ),
        ];

        for (
            gate,
            pass_allowed_next_node,
            blocked_summary,
            blocker_code,
            rework_target,
            allowed_next_node,
        ) in cases
        {
            let pass_fields = host_bridge_result_verdict_fields_for_gate(gate, &[], None);
            assert_eq!(pass_fields.decision, "approve", "{gate}");
            assert_eq!(pass_fields.verdict, "pass", "{gate}");
            assert_eq!(pass_fields.blocker_codes, Vec::<String>::new(), "{gate}");
            assert_eq!(pass_fields.rework_target, None, "{gate}");
            assert_eq!(
                pass_fields.allowed_next_node, pass_allowed_next_node,
                "{gate}"
            );

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
            (
                serde_json::json!({
                    "status": "pass",
                    "execution_state": "executed",
                    "decision": "pass",
                    "verdict": "implemented",
                    "blocker_codes": [],
                    "rework_target": null,
                    "allowed_next_node": "tester"
                }),
                "invalid_allowed_next_node",
            ),
        ];

        for (result, expected_blocker) in cases {
            let blockers = host_bridge_result_verdict_contract_blockers_for_target(
                &result,
                &required_fields,
                "developer",
            );
            assert!(
                blockers.iter().any(|blocker| blocker == expected_blocker),
                "expected blocker `{expected_blocker}` for result {result}, got {blockers:?}"
            );
        }
    }

    #[test]
    fn result_verdict_contract_accepts_configured_coach_alias_after_developer_pass() {
        let required_fields = crate::request::default_host_bridge_required_result_fields();
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "pass",
            "verdict": "implemented",
            "blocker_codes": [],
            "rework_target": null,
            "allowed_next_node": "coach_implementation_gate"
        });

        let blockers = host_bridge_result_verdict_contract_blockers_for_target(
            &result,
            &required_fields,
            "developer",
        );

        assert!(
            !blockers
                .iter()
                .any(|blocker| blocker == "invalid_allowed_next_node"),
            "configured coach alias should be accepted after developer pass, got {blockers:?}"
        );
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
    fn summary_classifier_ignores_display_only_typed_result_fields() {
        let summary = "decision=pass; verdict=pass; blocker_codes=[]; rework_target=none; allowed_next_node=designer";

        assert_eq!(
            host_bridge_lane_completion_summary_blocker_code("analyst", Some(summary)),
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
    fn implementation_artifacts_are_required_for_implementation_targets_or_task_classes() {
        assert!(host_bridge_completion_requires_implementation_artifacts(
            "implementer"
        ));
        assert!(host_bridge_completion_requires_implementation_artifacts(
            " implementation "
        ));
        assert!(host_bridge_request_requires_implementation_artifacts(
            "developer",
            Some("implementation")
        ));
        assert!(host_bridge_request_requires_implementation_artifacts(
            "developer",
            Some(" delivery_task ")
        ));
        assert!(!host_bridge_request_requires_implementation_artifacts(
            "developer",
            Some("coach")
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
