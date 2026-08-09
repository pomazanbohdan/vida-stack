//! Canonical, fail-closed TeamFlow inclusion authority.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::team_flow_transition::TeamFlowSnapshot;

pub const MODULE: &str = "team_flow_inclusion";
pub const SCHEMA_VERSION: u16 = 1;
pub const DECISION_KIND: &str = "team_flow_inclusion_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InclusionRule {
    Always,
    Never,
    Optional,
    WhenProofRequired,
    WhenReviewTriggered,
    WhenArchitectureTriggered,
    WhenReworkRequired,
}

impl InclusionRule {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::Optional => "optional",
            Self::WhenProofRequired => "when_proof_required",
            Self::WhenReviewTriggered => "when_review_triggered",
            Self::WhenArchitectureTriggered => "when_architecture_triggered",
            Self::WhenReworkRequired => "when_rework_required",
        }
    }
}

impl FromStr for InclusionRule {
    type Err = InclusionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            "optional" => Ok(Self::Optional),
            "when_proof_required" => Ok(Self::WhenProofRequired),
            "when_review_triggered" => Ok(Self::WhenReviewTriggered),
            "when_architecture_triggered" => Ok(Self::WhenArchitectureTriggered),
            "when_rework_required" => Ok(Self::WhenReworkRequired),
            _ => Err(InclusionError::InvalidRule(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContextV1 {
    pub schema_version: u16,
    pub optional_requested: bool,
    pub proof_required: bool,
    pub review_triggered: bool,
    pub architecture_triggered: bool,
    pub rework_required: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionAuditV1 {
    pub task_id: String,
    pub run_id: String,
    pub flow_ref: String,
    pub authority_id: String,
    pub authority_content_hash: String,
    pub config_authority_hash: String,
    pub registry_authority_hash: String,
    pub source_snapshot_ref: String,
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_content_digest: String,
    pub optional_nodes: Vec<String>,
    pub proof_required: bool,
    pub review_triggered: bool,
    pub architecture_triggered: bool,
    pub rework_required: bool,
    pub evidence_refs: Vec<String>,
}

impl InclusionAuditV1 {
    pub fn canonical_digest(&self) -> Result<String, InclusionError> {
        digest(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionRequestV1 {
    pub schema_version: u16,
    pub decision_kind: String,
    pub task_id: String,
    pub run_id: String,
    pub flow_ref: String,
    pub node_id: String,
    pub inclusion_rule: InclusionRule,
    pub required: bool,
    pub terminal: bool,
    pub authority_id: String,
    pub authority_content_hash: String,
    pub config_authority_hash: String,
    pub registry_authority_hash: String,
    pub source_snapshot_ref: String,
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_content_digest: String,
    pub audit_digest: String,
    pub evidence: EvidenceContextV1,
}

impl InclusionRequestV1 {
    pub fn canonical_digest(&self) -> Result<String, InclusionError> {
        self.validate()?;
        digest(self)
    }

    fn validate(&self) -> Result<(), InclusionError> {
        if self.schema_version != SCHEMA_VERSION
            || self.evidence.schema_version != SCHEMA_VERSION
            || self.decision_kind != DECISION_KIND
        {
            return Err(InclusionError::Schema);
        }
        for value in [
            &self.task_id,
            &self.run_id,
            &self.flow_ref,
            &self.node_id,
            &self.authority_id,
            &self.authority_content_hash,
            &self.config_authority_hash,
            &self.registry_authority_hash,
            &self.source_snapshot_ref,
            &self.policy_id,
            &self.policy_content_digest,
            &self.audit_digest,
        ] {
            if value.trim().is_empty() || value.len() > 256 || value.trim() != value {
                return Err(InclusionError::Identity);
            }
        }
        if self.evidence.evidence_refs.len() > 64
            || self
                .evidence
                .evidence_refs
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 256 || value.trim() != value)
        {
            return Err(InclusionError::Evidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionDecisionV1 {
    pub schema_version: u16,
    pub decision_kind: String,
    pub request_digest: String,
    pub evidence_digest: String,
    pub included: bool,
    pub reason_code: String,
}

impl InclusionDecisionV1 {
    pub fn canonical_digest(&self) -> Result<String, InclusionError> {
        if self.schema_version != SCHEMA_VERSION
            || self.decision_kind != DECISION_KIND
            || self.request_digest.len() != 64
            || self.evidence_digest.len() != 64
            || self.reason_code.trim().is_empty()
        {
            return Err(InclusionError::Decision);
        }
        digest(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionReceiptV1 {
    pub schema_version: u16,
    pub decision_kind: String,
    pub receipt_id: String,
    pub activation_snapshot_ref: String,
    pub request: InclusionRequestV1,
    pub decision: InclusionDecisionV1,
    pub decision_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionSnapshotOverlayV1 {
    pub schema_version: u16,
    pub decision_kind: String,
    pub source_snapshot_ref: String,
    pub activation_snapshot_ref: String,
    pub snapshot: TeamFlowSnapshot,
    pub receipts: Vec<InclusionReceiptV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InclusionError {
    #[error("team_flow_inclusion_rule_invalid:{0}")]
    InvalidRule(String),
    #[error("team_flow_inclusion_schema_invalid")]
    Schema,
    #[error("team_flow_inclusion_identity_invalid")]
    Identity,
    #[error("team_flow_inclusion_evidence_invalid")]
    Evidence,
    #[error("team_flow_inclusion_decision_invalid")]
    Decision,
    #[error("team_flow_inclusion_receipt_invalid")]
    Receipt,
    #[error("team_flow_inclusion_request_set_invalid")]
    RequestSet,
}

pub fn evaluate(request: &InclusionRequestV1) -> Result<InclusionDecisionV1, InclusionError> {
    let request_digest = request.canonical_digest()?;
    let evidence_digest = digest(&request.evidence)?;
    let included = request.terminal
        || match request.inclusion_rule {
            InclusionRule::Always => true,
            InclusionRule::Never => false,
            InclusionRule::Optional => request.evidence.optional_requested,
            InclusionRule::WhenProofRequired => request.evidence.proof_required,
            InclusionRule::WhenReviewTriggered => request.evidence.review_triggered,
            InclusionRule::WhenArchitectureTriggered => request.evidence.architecture_triggered,
            InclusionRule::WhenReworkRequired => request.evidence.rework_required,
        };
    Ok(InclusionDecisionV1 {
        schema_version: SCHEMA_VERSION,
        decision_kind: DECISION_KIND.to_string(),
        request_digest,
        evidence_digest,
        included,
        reason_code: if request.terminal {
            "terminal_safety"
        } else {
            request.inclusion_rule.as_str()
        }
        .to_string(),
    })
}

pub fn build_snapshot_overlay(
    snapshot: &TeamFlowSnapshot,
    requests: Vec<InclusionRequestV1>,
) -> Result<InclusionSnapshotOverlayV1, InclusionError> {
    let requests = validated_requests(snapshot, requests)?;
    let decisions = requests
        .iter()
        .map(evaluate)
        .collect::<Result<Vec<_>, _>>()?;
    let activation_snapshot_ref = digest(&(
        snapshot.snapshot_ref.as_str(),
        decisions
            .iter()
            .map(InclusionDecisionV1::canonical_digest)
            .collect::<Result<Vec<_>, _>>()?,
    ))?;
    let receipts = requests
        .into_iter()
        .zip(decisions)
        .map(|(request, decision)| make_receipt(request, decision, &activation_snapshot_ref))
        .collect::<Result<Vec<_>, _>>()?;
    overlay(snapshot, receipts, activation_snapshot_ref)
}

pub fn replay_snapshot_overlay(
    snapshot: &TeamFlowSnapshot,
    requests: Vec<InclusionRequestV1>,
    receipts: Vec<InclusionReceiptV1>,
) -> Result<InclusionSnapshotOverlayV1, InclusionError> {
    let requests = validated_requests(snapshot, requests)?;
    let receipts = exact_node_order(snapshot, &receipts, |receipt| &receipt.request.node_id)?;
    let activation = receipts
        .first()
        .map(|receipt| receipt.activation_snapshot_ref.clone())
        .ok_or(InclusionError::RequestSet)?;
    for (receipt, request) in receipts.iter().zip(&requests) {
        validate_replay(receipt, request, &activation)?;
    }
    let expected = digest(&(
        snapshot.snapshot_ref.as_str(),
        receipts
            .iter()
            .map(|receipt| receipt.decision_digest.clone())
            .collect::<Vec<_>>(),
    ))?;
    if expected != activation {
        return Err(InclusionError::Receipt);
    }
    overlay(snapshot, receipts, activation)
}

pub fn validate_replay(
    receipt: &InclusionReceiptV1,
    request: &InclusionRequestV1,
    activation_snapshot_ref: &str,
) -> Result<(), InclusionError> {
    let expected_decision = evaluate(request)?;
    let decision_digest = receipt.decision.canonical_digest()?;
    if receipt.schema_version != SCHEMA_VERSION
        || receipt.decision_kind != DECISION_KIND
        || receipt.request != *request
        || receipt.decision != expected_decision
        || receipt.decision.request_digest != request.canonical_digest()?
        || receipt.decision.evidence_digest != digest(&request.evidence)?
        || receipt.decision_digest != decision_digest
        || receipt.activation_snapshot_ref != activation_snapshot_ref
        || receipt.receipt_id != receipt_id(request, &decision_digest, activation_snapshot_ref)?
    {
        return Err(InclusionError::Receipt);
    }
    Ok(())
}

fn validated_requests(
    snapshot: &TeamFlowSnapshot,
    requests: Vec<InclusionRequestV1>,
) -> Result<Vec<InclusionRequestV1>, InclusionError> {
    if !snapshot.has_valid_identity() {
        return Err(InclusionError::RequestSet);
    }
    let requests = exact_node_order(snapshot, &requests, |request| &request.node_id)?;
    let identity = requests.first().ok_or(InclusionError::RequestSet)?;
    for (node, request) in snapshot.nodes.iter().zip(&requests) {
        request.validate()?;
        if request.flow_ref != snapshot.flow_ref
            || request.node_id != node.node_id
            || request.inclusion_rule.as_str() != node.inclusion_rule
            || request.required != node.required
            || request.terminal != node.terminal
            || request.source_snapshot_ref != snapshot.snapshot_ref
            || request.config_authority_hash != snapshot.config_hash
            || request.registry_authority_hash != snapshot.registry_hash
            || request.task_id != identity.task_id
            || request.run_id != identity.run_id
            || request.authority_id != identity.authority_id
            || request.authority_content_hash != identity.authority_content_hash
            || request.policy_id != identity.policy_id
            || request.policy_version != identity.policy_version
            || request.policy_content_digest != identity.policy_content_digest
            || request.audit_digest != identity.audit_digest
            || request.evidence.evidence_refs != identity.evidence.evidence_refs
            || request.evidence.proof_required != identity.evidence.proof_required
            || request.evidence.review_triggered != identity.evidence.review_triggered
            || request.evidence.architecture_triggered != identity.evidence.architecture_triggered
            || request.evidence.rework_required != identity.evidence.rework_required
        {
            return Err(InclusionError::RequestSet);
        }
    }
    let audit_ref = format!("audit:{}", identity.audit_digest);
    if identity
        .evidence
        .evidence_refs
        .iter()
        .filter(|reference| reference.starts_with("audit:"))
        .count()
        != 1
    {
        return Err(InclusionError::RequestSet);
    }
    let audit = InclusionAuditV1 {
        task_id: identity.task_id.clone(),
        run_id: identity.run_id.clone(),
        flow_ref: identity.flow_ref.clone(),
        authority_id: identity.authority_id.clone(),
        authority_content_hash: identity.authority_content_hash.clone(),
        config_authority_hash: identity.config_authority_hash.clone(),
        registry_authority_hash: identity.registry_authority_hash.clone(),
        source_snapshot_ref: identity.source_snapshot_ref.clone(),
        policy_id: identity.policy_id.clone(),
        policy_version: identity.policy_version,
        policy_content_digest: identity.policy_content_digest.clone(),
        optional_nodes: requests
            .iter()
            .filter(|request| request.evidence.optional_requested)
            .map(|request| request.node_id.clone())
            .collect(),
        proof_required: identity.evidence.proof_required,
        review_triggered: identity.evidence.review_triggered,
        architecture_triggered: identity.evidence.architecture_triggered,
        rework_required: identity.evidence.rework_required,
        evidence_refs: identity
            .evidence
            .evidence_refs
            .iter()
            .filter(|reference| !reference.starts_with("audit:"))
            .cloned()
            .collect(),
    };
    if !identity.evidence.evidence_refs.contains(&audit_ref)
        || audit.canonical_digest()? != identity.audit_digest
    {
        return Err(InclusionError::RequestSet);
    }
    Ok(requests)
}

fn exact_node_order<T: Clone>(
    snapshot: &TeamFlowSnapshot,
    values: &[T],
    node_id: impl Fn(&T) -> &str,
) -> Result<Vec<T>, InclusionError> {
    if values.len() != snapshot.nodes.len() {
        return Err(InclusionError::RequestSet);
    }
    snapshot
        .nodes
        .iter()
        .map(|node| {
            let mut matches = values.iter().filter(|value| node_id(value) == node.node_id);
            let value = matches.next().ok_or(InclusionError::RequestSet)?;
            if matches.next().is_some() {
                return Err(InclusionError::RequestSet);
            }
            Ok(value.clone())
        })
        .collect()
}

fn make_receipt(
    request: InclusionRequestV1,
    decision: InclusionDecisionV1,
    activation_snapshot_ref: &str,
) -> Result<InclusionReceiptV1, InclusionError> {
    let decision_digest = decision.canonical_digest()?;
    Ok(InclusionReceiptV1 {
        schema_version: SCHEMA_VERSION,
        decision_kind: DECISION_KIND.to_string(),
        receipt_id: receipt_id(&request, &decision_digest, activation_snapshot_ref)?,
        activation_snapshot_ref: activation_snapshot_ref.to_string(),
        request,
        decision,
        decision_digest,
    })
}

fn overlay(
    snapshot: &TeamFlowSnapshot,
    receipts: Vec<InclusionReceiptV1>,
    activation_snapshot_ref: String,
) -> Result<InclusionSnapshotOverlayV1, InclusionError> {
    let mut activated = snapshot.clone();
    for (node, receipt) in activated.nodes.iter_mut().zip(&receipts) {
        node.included = receipt.decision.included;
    }
    activated.snapshot_ref.clear();
    activated.snapshot_ref = crate::team_flow_transition::hash_json(
        &serde_json::to_value(&activated).map_err(|_| InclusionError::Identity)?,
    );
    Ok(InclusionSnapshotOverlayV1 {
        schema_version: SCHEMA_VERSION,
        decision_kind: DECISION_KIND.to_string(),
        source_snapshot_ref: snapshot.snapshot_ref.clone(),
        activation_snapshot_ref,
        snapshot: activated,
        receipts,
    })
}

fn receipt_id(
    request: &InclusionRequestV1,
    decision_digest: &str,
    activation_snapshot_ref: &str,
) -> Result<String, InclusionError> {
    Ok(format!(
        "team-flow-inclusion:{}",
        digest(&(
            request.canonical_digest()?,
            decision_digest,
            activation_snapshot_ref
        ))?
    ))
}

fn digest<T: Serialize>(value: &T) -> Result<String, InclusionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| InclusionError::Identity)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team_flow_transition::{TeamFlowNode, TeamFlowSnapshot};

    fn request(rule: InclusionRule) -> InclusionRequestV1 {
        InclusionRequestV1 {
            schema_version: 1,
            decision_kind: DECISION_KIND.to_string(),
            task_id: "task-a".to_string(),
            run_id: "run-a".to_string(),
            flow_ref: "flow-a".to_string(),
            node_id: "node-a".to_string(),
            inclusion_rule: rule,
            required: false,
            terminal: false,
            authority_id: "authority-a".to_string(),
            authority_content_hash: "authority-hash".to_string(),
            config_authority_hash: "config-hash".to_string(),
            registry_authority_hash: "registry-hash".to_string(),
            source_snapshot_ref: "snapshot-a".to_string(),
            policy_id: "rhai.runtime.authority".to_string(),
            policy_version: 0,
            policy_content_digest: "rust-baseline-v1".to_string(),
            audit_digest: "audit-a".to_string(),
            evidence: EvidenceContextV1 {
                schema_version: 1,
                optional_requested: false,
                proof_required: false,
                review_triggered: false,
                architecture_triggered: false,
                rework_required: false,
                evidence_refs: vec!["evidence-a".to_string()],
            },
        }
    }

    fn snapshot() -> TeamFlowSnapshot {
        let mut snapshot = TeamFlowSnapshot {
            config_id: "cfg".to_string(),
            profile: "profile".to_string(),
            flow_ref: "flow-a".to_string(),
            config_hash: "config-hash".to_string(),
            registry_hash: "registry-hash".to_string(),
            snapshot_ref: String::new(),
            entry_node_id: "node-a".to_string(),
            ordered_nodes: vec!["node-a".to_string()],
            nodes: vec![TeamFlowNode {
                node_id: "node-a".to_string(),
                runtime_role: "worker".to_string(),
                task_class: "implementation".to_string(),
                inclusion_rule: "always".to_string(),
                included: true,
                required: true,
                next_node: None,
                rework_targets: vec![],
                evidence_requirements: vec![],
                command_ref: None,
                command_mapping_hash: None,
                requires_user_approval: false,
                terminal: true,
            }],
        };
        snapshot.snapshot_ref =
            crate::team_flow_transition::hash_json(&serde_json::to_value(&snapshot).unwrap());
        snapshot
    }

    fn bind_audit(requests: &mut [InclusionRequestV1]) {
        let source_refs = vec![
            format!("task:{}", requests[0].task_id),
            format!("run:{}", requests[0].run_id),
            format!("flow:{}", requests[0].flow_ref),
            format!("authority:{}", requests[0].authority_id),
            format!("snapshot:{}", requests[0].source_snapshot_ref),
            "selection:test".to_string(),
        ];
        for request in requests.iter_mut() {
            request.evidence.evidence_refs = source_refs.clone();
        }
        let identity = &requests[0];
        let audit = InclusionAuditV1 {
            task_id: identity.task_id.clone(),
            run_id: identity.run_id.clone(),
            flow_ref: identity.flow_ref.clone(),
            authority_id: identity.authority_id.clone(),
            authority_content_hash: identity.authority_content_hash.clone(),
            config_authority_hash: identity.config_authority_hash.clone(),
            registry_authority_hash: identity.registry_authority_hash.clone(),
            source_snapshot_ref: identity.source_snapshot_ref.clone(),
            policy_id: identity.policy_id.clone(),
            policy_version: identity.policy_version,
            policy_content_digest: identity.policy_content_digest.clone(),
            optional_nodes: requests
                .iter()
                .filter(|request| request.evidence.optional_requested)
                .map(|request| request.node_id.clone())
                .collect(),
            proof_required: identity.evidence.proof_required,
            review_triggered: identity.evidence.review_triggered,
            architecture_triggered: identity.evidence.architecture_triggered,
            rework_required: identity.evidence.rework_required,
            evidence_refs: source_refs,
        };
        let audit_digest = audit.canonical_digest().unwrap();
        for request in requests {
            request.audit_digest = audit_digest.clone();
            request
                .evidence
                .evidence_refs
                .push(format!("audit:{audit_digest}"));
        }
    }

    #[test]
    fn seven_rule_truth_matrix_and_malformed_values_fail_closed() {
        let cases = [
            (InclusionRule::Always, true),
            (InclusionRule::Never, false),
            (InclusionRule::Optional, false),
            (InclusionRule::WhenProofRequired, false),
            (InclusionRule::WhenReviewTriggered, false),
            (InclusionRule::WhenArchitectureTriggered, false),
            (InclusionRule::WhenReworkRequired, false),
        ];
        for (rule, expected) in cases {
            assert_eq!(evaluate(&request(rule)).unwrap().included, expected);
            let mut triggered = request(rule);
            triggered.evidence.optional_requested = true;
            triggered.evidence.proof_required = true;
            triggered.evidence.review_triggered = true;
            triggered.evidence.architecture_triggered = true;
            triggered.evidence.rework_required = true;
            assert_eq!(
                evaluate(&triggered).unwrap().included,
                rule != InclusionRule::Never
            );
        }
        for malformed in [
            "",
            "Always",
            " always",
            "always ",
            "when_review_or_architecture",
        ] {
            assert!(malformed.parse::<InclusionRule>().is_err(), "{malformed:?}");
        }
    }

    #[test]
    fn replay_binds_snapshot_request_and_recomputed_decision() {
        let snapshot = snapshot();
        let mut req = request(InclusionRule::Always);
        req.required = true;
        req.terminal = true;
        req.source_snapshot_ref = snapshot.snapshot_ref.clone();
        let mut requests = vec![req];
        bind_audit(&mut requests);
        let req = requests.remove(0);
        let overlay = build_snapshot_overlay(&snapshot, vec![req.clone()]).unwrap();
        assert!(
            replay_snapshot_overlay(&snapshot, vec![req.clone()], overlay.receipts.clone()).is_ok()
        );
        for mutate in 0..17 {
            let mut forged = req.clone();
            match mutate {
                0 => forged.task_id.push('x'),
                1 => forged.run_id.push('x'),
                2 => forged.flow_ref.push('x'),
                3 => forged.node_id.push('x'),
                4 => forged.authority_id.push('x'),
                5 => forged.authority_content_hash.push('x'),
                6 => forged.config_authority_hash.push('x'),
                7 => forged.registry_authority_hash.push('x'),
                8 => forged.source_snapshot_ref.push('x'),
                9 => forged.policy_id.push('x'),
                10 => forged.policy_version += 1,
                11 => forged.policy_content_digest.push('x'),
                12 => forged.audit_digest.push('x'),
                13 => forged.inclusion_rule = InclusionRule::Never,
                14 => forged.required = !forged.required,
                15 => forged.terminal = !forged.terminal,
                _ => forged.evidence.review_triggered = !forged.evidence.review_triggered,
            }
            assert!(
                replay_snapshot_overlay(&snapshot, vec![forged], overlay.receipts.clone()).is_err()
            );
        }
        for mutate in 0..5 {
            let mut receipts = overlay.receipts.clone();
            match mutate {
                0 => receipts[0].receipt_id.push('x'),
                1 => receipts[0].activation_snapshot_ref.push('x'),
                2 => receipts[0].decision_digest.push('x'),
                3 => receipts[0].decision.request_digest.push('x'),
                _ => receipts[0].decision.evidence_digest.push('x'),
            }
            assert!(replay_snapshot_overlay(&snapshot, vec![req.clone()], receipts).is_err());
        }

        let mut decision = evaluate(&req).unwrap();
        decision.included = false;
        decision.reason_code = "forged".to_string();
        let decision_digest = decision.canonical_digest().unwrap();
        let activation = digest(&(
            snapshot.snapshot_ref.as_str(),
            vec![decision_digest.clone()],
        ))
        .unwrap();
        let forged = make_receipt(req.clone(), decision, &activation).unwrap();
        assert!(replay_snapshot_overlay(&snapshot, vec![req.clone()], vec![forged]).is_err());

        let mut forged_request = request(InclusionRule::Never);
        forged_request.source_snapshot_ref = snapshot.snapshot_ref.clone();
        let forged_decision = evaluate(&forged_request).unwrap();
        let forged_digest = forged_decision.canonical_digest().unwrap();
        let forged_activation =
            digest(&(snapshot.snapshot_ref.as_str(), vec![forged_digest])).unwrap();
        let forged_receipt =
            make_receipt(forged_request.clone(), forged_decision, &forged_activation).unwrap();
        assert!(
            replay_snapshot_overlay(&snapshot, vec![forged_request], vec![forged_receipt]).is_err()
        );

        let mut forged_request = req;
        forged_request.audit_digest = "f".repeat(64);
        forged_request
            .evidence
            .evidence_refs
            .retain(|reference| !reference.starts_with("audit:"));
        forged_request
            .evidence
            .evidence_refs
            .push(format!("audit:{}", forged_request.audit_digest));
        let forged_decision = evaluate(&forged_request).unwrap();
        let forged_digest = forged_decision.canonical_digest().unwrap();
        let forged_activation =
            digest(&(snapshot.snapshot_ref.as_str(), vec![forged_digest.clone()])).unwrap();
        let forged_receipt =
            make_receipt(forged_request.clone(), forged_decision, &forged_activation).unwrap();
        assert!(
            replay_snapshot_overlay(&snapshot, vec![forged_request], vec![forged_receipt]).is_err()
        );
    }

    #[test]
    fn duplicate_requests_fail_closed_before_node_map_collapse() {
        let mut snapshot = snapshot();
        snapshot.nodes[0].terminal = false;
        snapshot.nodes[0].next_node = Some("node-b".to_string());
        let mut terminal = snapshot.nodes[0].clone();
        terminal.node_id = "node-b".to_string();
        terminal.next_node = None;
        terminal.terminal = true;
        snapshot.nodes.push(terminal);
        snapshot.ordered_nodes.push("node-b".to_string());
        snapshot.snapshot_ref.clear();
        snapshot.snapshot_ref =
            crate::team_flow_transition::hash_json(&serde_json::to_value(&snapshot).unwrap());
        let mut duplicate = request(InclusionRule::Always);
        duplicate.source_snapshot_ref = snapshot.snapshot_ref.clone();
        duplicate.terminal = false;
        duplicate.required = true;
        assert!(build_snapshot_overlay(&snapshot, vec![duplicate.clone(), duplicate]).is_err());

        let mut first = request(InclusionRule::Always);
        first.source_snapshot_ref = snapshot.snapshot_ref.clone();
        first.required = true;
        let mut second = first.clone();
        second.node_id = "node-b".to_string();
        second.terminal = true;
        let mut heterogeneous = vec![first, second];
        bind_audit(&mut heterogeneous);
        heterogeneous[1].evidence.review_triggered = true;
        assert!(build_snapshot_overlay(&snapshot, heterogeneous).is_err());
    }
}
