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
pub fn host_bridge_completed_result_execution_state_is_admissible(execution_state: &str) -> bool {
    execution_state == "executed"
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
    fn completed_artifact_admissibility_is_shared() {
        assert!(host_bridge_existing_request_status_is_admissible("pending"));
        assert!(host_bridge_existing_request_status_is_admissible(
            "completed"
        ));
        assert!(!host_bridge_existing_request_status_is_admissible(
            "blocked"
        ));
        assert!(host_bridge_completed_artifact_status_is_admissible("pass"));
        assert!(host_bridge_completed_result_execution_state_is_admissible(
            "executed"
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
