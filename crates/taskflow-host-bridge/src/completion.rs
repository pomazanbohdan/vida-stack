use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
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
        status: if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
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
            | "implementation_artifact_authority_missing"
            | "implementation_artifact_changed_files_missing"
            | "implementation_artifact_authority_invalid"
            | "implementation_artifact_contract_invalid"
            | "implementation_artifact_receipt_missing"
            | "implementation_artifact_receipt_unverified"
            | "implementation_artifacts_missing"
            | "implementation_attempt_scope_guard_violation"
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
    status == "pass"
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
            status: "pass".to_string(),
            execution_state: "executed".to_string(),
            completion_verdict: "pass".to_string(),
            completion_ready: true,
        }
    } else {
        HostBridgeCompletionVerdict {
            status: "blocked".to_string(),
            execution_state: "blocked".to_string(),
            completion_verdict: "rework_required".to_string(),
            completion_ready: false,
        }
    }
}

#[must_use]
pub fn host_bridge_request_status_after_completion(blocker_codes: &[String]) -> String {
    if blocker_codes.is_empty() {
        "pass".to_string()
    } else if blocker_codes
        .iter()
        .all(|blocker| host_bridge_completion_retryable_blocker(blocker))
    {
        "retryable_blocked".to_string()
    } else {
        "blocked".to_string()
    }
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
}
