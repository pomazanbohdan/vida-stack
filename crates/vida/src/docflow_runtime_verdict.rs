use crate::contract_profile_adapter::{blocker_code, BlockerCode};
use crate::runtime_consumption_surface::{
    DOCFLOW_PROOF_CURRENT_PATH, DOCFLOW_READINESS_CURRENT_PATH,
};

pub(crate) fn build_docflow_runtime_verdict(
    registry: &crate::RuntimeConsumptionEvidence,
    check: &crate::RuntimeConsumptionEvidence,
    readiness: &crate::RuntimeConsumptionEvidence,
    proof: &crate::RuntimeConsumptionEvidence,
) -> crate::RuntimeConsumptionDocflowVerdict {
    let mut blockers = Vec::new();
    if !registry.ok {
        if let Some(code) = blocker_code(BlockerCode::MissingDocflowActivation) {
            blockers.push(code);
        }
    }
    if !check.ok {
        if let Some(code) = blocker_code(BlockerCode::DocflowCheckBlocking) {
            blockers.push(code);
        }
    }
    if !readiness.ok {
        if let Some(code) = blocker_code(BlockerCode::MissingReadinessVerdict) {
            blockers.push(code);
        }
    }
    if !matches!(readiness.verdict.as_deref(), Some("ready" | "blocked")) {
        if let Some(code) = blocker_code(BlockerCode::MissingReadinessVerdict) {
            blockers.push(code);
        }
    }
    if readiness
        .artifact_path
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        if let Some(code) = blocker_code(BlockerCode::MissingInventoryOrProjectionEvidence) {
            blockers.push(code);
        }
    }
    if !proof.ok {
        if let Some(code) = blocker_code(BlockerCode::MissingProofVerdict) {
            blockers.push(code);
        }
    }
    if !matches!(proof.verdict.as_deref(), Some("ready" | "blocked")) {
        if let Some(code) = blocker_code(BlockerCode::MissingProofVerdict) {
            blockers.push(code);
        }
    }
    if proof
        .artifact_path
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        if let Some(code) = blocker_code(BlockerCode::MissingClosureProof) {
            blockers.push(code);
        }
    }

    crate::RuntimeConsumptionDocflowVerdict {
        status: if blockers.is_empty() {
            "pass".to_string()
        } else {
            "block".to_string()
        },
        ready: blockers.is_empty(),
        blockers,
        proof_surfaces: vec![
            registry.surface.clone(),
            check.surface.clone(),
            readiness.surface.clone(),
            proof.surface.clone(),
        ],
    }
}

pub(crate) fn blocking_docflow_activation(
    error: &str,
) -> crate::RuntimeConsumptionDocflowActivation {
    crate::RuntimeConsumptionDocflowActivation {
        activated: false,
        runtime_family: "docflow".to_string(),
        owner_runtime: "taskflow".to_string(),
        evidence: serde_json::json!({
            "error": error,
            "overview": {
                "surface": "vida taskflow direct runtime-consumption overview",
                "ok": false,
                "registry_rows": 0,
                "check_rows": 0,
                "readiness_rows": 0,
                "proof_blocking": true
            },
            "registry": {
                "surface": "vida docflow registry --root <repo-root>",
                "ok": false,
                "row_count": 0,
                "output": ""
            },
            "check": {
                "surface": "vida docflow check --profile active-canon",
                "ok": false,
                "row_count": 0,
                "output": error
            },
            "readiness": {
                "surface": "vida docflow readiness-check --profile active-canon",
                "ok": false,
                "row_count": 0,
                "verdict": "blocked",
                "artifact_path": DOCFLOW_READINESS_CURRENT_PATH,
                "output": error
            },
            "proof": {
                "surface": "vida docflow proofcheck --profile active-canon",
                "ok": false,
                "row_count": 0,
                "verdict": "blocked",
                "artifact_path": DOCFLOW_PROOF_CURRENT_PATH,
                "output": error
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::build_docflow_runtime_verdict;
    use crate::runtime_consumption_surface::{
        DOCFLOW_PROOF_CURRENT_PATH, DOCFLOW_READINESS_CURRENT_PATH,
    };

    #[test]
    fn taskflow_consume_final_verdict_reports_pass_without_blockers() {
        let registry = crate::RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = crate::RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: true,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let readiness = crate::RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: true,
            row_count: 0,
            verdict: Some("ready".to_string()),
            artifact_path: Some(DOCFLOW_READINESS_CURRENT_PATH.to_string()),
            output: String::new(),
        };
        let proof = crate::RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: Some(DOCFLOW_PROOF_CURRENT_PATH.to_string()),
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "pass");
        assert!(verdict.ready);
        assert!(verdict.blockers.is_empty());
        assert_eq!(
            verdict.proof_surfaces,
            vec!["registry", "check", "readiness", "proof"]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_reports_explicit_blockers() {
        let registry = crate::RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: false,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = crate::RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: false,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: "blocking check".to_string(),
        };
        let readiness = crate::RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: false,
            row_count: 2,
            verdict: Some("blocked".to_string()),
            artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
            output: "blocking readiness".to_string(),
        };
        let proof = crate::RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: false,
            row_count: 1,
            verdict: Some("blocked".to_string()),
            artifact_path: Some("vida/config/docflow-proof.current.jsonl".to_string()),
            output: "❌ BLOCKING: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec![
                "missing_docflow_activation",
                "docflow_check_blocking",
                "missing_readiness_verdict",
                "missing_proof_verdict",
            ]
        );
        assert_eq!(
            verdict.proof_surfaces,
            vec!["registry", "check", "readiness", "proof"]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_blocks_when_readiness_artifact_path_missing() {
        let registry = crate::RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = crate::RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: true,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let readiness = crate::RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: true,
            row_count: 0,
            verdict: Some("ready".to_string()),
            artifact_path: None,
            output: String::new(),
        };
        let proof = crate::RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec![
                "missing_inventory_or_projection_evidence",
                "missing_closure_proof"
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_blocks_when_readiness_artifact_path_empty() {
        let registry = crate::RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = crate::RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: true,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let readiness = crate::RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: true,
            row_count: 0,
            verdict: Some("ready".to_string()),
            artifact_path: Some("   ".to_string()),
            output: String::new(),
        };
        let proof = crate::RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec![
                "missing_inventory_or_projection_evidence",
                "missing_closure_proof"
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_blocks_when_proof_verdict_is_missing() {
        let registry = crate::RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = crate::RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: true,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let readiness = crate::RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: true,
            row_count: 0,
            verdict: Some("ready".to_string()),
            artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
            output: String::new(),
        };
        let proof = crate::RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec!["missing_proof_verdict", "missing_closure_proof"]
        );
    }
}
