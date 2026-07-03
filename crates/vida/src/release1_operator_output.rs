use serde_json::Value;

pub(crate) use operator_output::operator_contracts::{
    OperatorContractSpec, OperatorSurfaceVerdict, canonical_blocker_code_entries,
    canonical_next_action_entries, canonical_operator_contract_status,
    canonical_operator_contract_status_str, canonical_pass_blocked_contract_status_str,
    finalize_operator_surface_verdict, is_canonical_blocker_code_entries,
    is_canonical_next_action_entries, is_canonical_operator_contract_status,
    normalize_blocker_codes, operator_contract_status_for_blockers,
    operator_contract_status_is_blocked, operator_contracts_consistency_error,
    operator_output_contract_parity_error, render_operator_contract_envelope,
};

pub(crate) const RELEASE1_OPERATOR_CONTRACT_SPEC: OperatorContractSpec = OperatorContractSpec {
    contract_id: "release-1-operator-contracts",
    schema_version: "release-1-v1",
    pass_status: "pass",
    blocked_status: "blocked",
    canonicalize_status: crate::release1_contracts::canonical_release1_contract_status_str,
    status_error_label: "canonical release-1 pass/blocked",
};

const DOCUMENTED_LOCAL_OPERATOR_BLOCKER_CODES: &[&str] = &[
    "canonical_gate_blocked",
    "close_feedback_canonical_status_blocked",
    "closeout_closure_not_ready",
    "closeout_proof_evidence_missing",
    "closeout_proof_targets_missing",
    "closeout_task_graph_invalid",
    "closeout_temp_scan_failed",
    "closeout_tracked_temp_artifacts",
    "foreign_claim_conflict_blocked",
    "invalid_task_title_input",
    "missing_structured_proof_evidence",
    "missing_requirement_identity",
    "proof_blocked_by_runtime",
    "requirement_source_unreadable",
    "state_reset_failed",
    "task_tree_traversal_failed",
    "untrusted_create_notes_file",
];

fn local_operator_blocker_code_is_documented(normalized: &str) -> bool {
    DOCUMENTED_LOCAL_OPERATOR_BLOCKER_CODES
        .iter()
        .any(|code| *code == normalized)
        || crate::release1_contracts::run_graph_operator_blocker_code_strings()
            .iter()
            .any(|code| *code == normalized)
}

fn canonical_release1_operator_blocker_codes(entries: &[String]) -> Vec<String> {
    let mut canonical = crate::contract_profile_adapter::canonical_blocker_codes(entries);
    for entry in entries {
        let normalized = entry.trim().to_ascii_lowercase();
        if local_operator_blocker_code_is_documented(normalized.as_str()) {
            canonical.push(normalized);
        }
    }
    canonical.sort();
    canonical.dedup();
    canonical
}

pub(crate) struct FinalizedRelease1OperatorTruth {
    pub(crate) status: &'static str,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) artifact_refs: Value,
    pub(crate) shared_fields: Value,
    pub(crate) operator_contracts: Value,
}

pub(crate) struct Release1OperatorOutputBuilder {
    surface: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Option<Value>,
    extra_fields: Value,
}

impl Release1OperatorOutputBuilder {
    pub(crate) fn new(surface: impl Into<String>) -> Self {
        Self {
            surface: surface.into(),
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
            artifact_refs: None,
            extra_fields: serde_json::json!({}),
        }
    }

    pub(crate) fn blocker_codes(mut self, blocker_codes: Vec<String>) -> Self {
        self.blocker_codes = blocker_codes;
        self
    }

    pub(crate) fn next_actions(mut self, next_actions: Vec<String>) -> Self {
        self.next_actions = next_actions;
        self
    }

    pub(crate) fn artifact_refs(mut self, artifact_refs: Value) -> Self {
        self.artifact_refs = Some(artifact_refs);
        self
    }

    pub(crate) fn extra_fields(mut self, extra_fields: Value) -> Self {
        self.extra_fields = extra_fields;
        self
    }

    pub(crate) fn build(self) -> Result<Value, String> {
        let surface = self.surface;
        let artifact_refs = self.artifact_refs.unwrap_or_else(|| {
            serde_json::json!({
                "surface": surface.clone(),
            })
        });
        build_release1_operator_output_payload(
            &surface,
            self.blocker_codes,
            self.next_actions,
            artifact_refs,
            self.extra_fields,
        )
    }
}

pub(crate) const VIDA_GATE_RESULT_SCHEMA_VERSION: &str =
    operator_output::operator_contracts::VIDA_GATE_RESULT_SCHEMA_VERSION;

pub(crate) fn render_vida_gate_result_with_status(
    gate_id: &str,
    explicit_status: &str,
    blocker_codes: Vec<String>,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    operator_output::operator_contracts::render_vida_gate_result_with_status(
        gate_id,
        explicit_status,
        blocker_codes,
        warning_codes,
        failure_codes,
        issues,
        next_actions,
        artifact_refs,
    )
}

pub(crate) fn render_vida_gate_result(
    gate_id: &str,
    blocker_codes: Vec<String>,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    operator_output::operator_contracts::render_vida_gate_result(
        gate_id,
        blocker_codes,
        warning_codes,
        failure_codes,
        issues,
        next_actions,
        artifact_refs,
    )
}

pub(crate) fn render_vida_gate_result_from_operator_contracts(
    gate_id: &str,
    operator_contracts: Value,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    operator_output::operator_contracts::render_vida_gate_result_from_operator_contracts(
        gate_id,
        operator_contracts,
        warning_codes,
        failure_codes,
        issues,
        next_actions,
        artifact_refs,
    )
}

pub(crate) fn finalize_release1_operator_surface_verdict_with_status(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Result<OperatorSurfaceVerdict, String> {
    let blocker_codes = blocker_codes
        .into_iter()
        .map(|code| code.trim().to_ascii_lowercase())
        .filter(|code| !code.is_empty())
        .collect::<Vec<_>>();
    let blocker_codes = normalize_blocker_codes(
        &blocker_codes,
        canonical_release1_operator_blocker_codes,
        Some("unsupported_blocker_code".to_string()),
    );
    let next_actions =
        canonical_next_action_entries(&serde_json::json!(next_actions)).unwrap_or_default();
    let verdict = finalize_operator_surface_verdict(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        status,
        blocker_codes,
        next_actions,
        artifact_refs,
    );
    if let Some(error) = release1_operator_contracts_consistency_error(
        verdict.operator_contracts["status"].as_str().unwrap_or(""),
        &verdict.blocker_codes,
        &verdict.next_actions,
    ) {
        return Err(error);
    }
    Ok(verdict)
}

pub(crate) fn finalize_release1_operator_truth(
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Result<FinalizedRelease1OperatorTruth, String> {
    let blocker_codes = normalize_blocker_codes(
        &blocker_codes,
        canonical_release1_operator_blocker_codes,
        Some("unsupported_blocker_code".to_string()),
    );
    let next_actions =
        canonical_next_action_entries(&serde_json::json!(next_actions)).unwrap_or(next_actions);
    let status = if blocker_codes.is_empty() {
        RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status
    } else {
        RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status
    };
    let verdict = finalize_release1_operator_surface_verdict_with_status(
        status,
        blocker_codes,
        next_actions,
        artifact_refs,
    )?;
    Ok(FinalizedRelease1OperatorTruth {
        status,
        blocker_codes: verdict.blocker_codes,
        next_actions: verdict.next_actions,
        artifact_refs: verdict.artifact_refs,
        shared_fields: verdict.shared_fields,
        operator_contracts: verdict.operator_contracts,
    })
}

pub(crate) fn build_release1_operator_output_payload(
    surface: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
    extra_fields: Value,
) -> Result<Value, String> {
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)?;
    let extra_object = extra_fields
        .as_object()
        .ok_or_else(|| "release-1 operator output extra_fields must be an object".to_string())?
        .clone();
    let mut payload = Value::Object(extra_object);
    let payload_object = payload
        .as_object_mut()
        .ok_or_else(|| "release-1 operator output payload should be an object".to_string())?;
    payload_object.insert("surface".to_string(), serde_json::json!(surface));
    payload_object.insert("status".to_string(), serde_json::json!(finalized.status));
    payload_object.insert(
        "trace_id".to_string(),
        finalized.operator_contracts["trace_id"].clone(),
    );
    payload_object.insert(
        "workflow_class".to_string(),
        finalized.operator_contracts["workflow_class"].clone(),
    );
    payload_object.insert(
        "risk_tier".to_string(),
        finalized.operator_contracts["risk_tier"].clone(),
    );
    payload_object.insert(
        "blocker_codes".to_string(),
        serde_json::json!(finalized.blocker_codes),
    );
    payload_object.insert(
        "next_actions".to_string(),
        serde_json::json!(finalized.next_actions),
    );
    payload_object.insert("artifact_refs".to_string(), finalized.artifact_refs);
    payload_object.insert("shared_fields".to_string(), finalized.shared_fields);
    payload_object.insert(
        "operator_contracts".to_string(),
        finalized.operator_contracts,
    );
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    if let Some(error) = shared_operator_output_contract_parity_error(&payload) {
        return Err(error.to_string());
    }
    Ok(payload)
}

pub(crate) fn replace_release1_operator_output_artifact_refs(
    payload: &mut Value,
    artifact_refs: Value,
) -> Result<(), String> {
    operator_output::operator_contracts::write_release1_operator_output_artifact_refs(
        payload,
        artifact_refs,
    )?;
    if let Some(error) = shared_operator_output_contract_parity_error(payload) {
        return Err(error.to_string());
    }
    Ok(())
}

pub(crate) fn canonical_release1_operator_contract_status(value: &Value) -> Option<&'static str> {
    canonical_operator_contract_status(&RELEASE1_OPERATOR_CONTRACT_SPEC, value)
}

pub(crate) fn canonical_release1_blocker_code_entries(value: &Value) -> Option<Vec<String>> {
    canonical_blocker_code_entries(value, canonical_release1_operator_blocker_codes)
}

pub(crate) fn release1_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    let normalized_status = status.trim().to_ascii_lowercase();
    if !matches!(
        normalized_status.as_str(),
        "pass" | "ok" | "blocked" | "block"
    ) {
        return Some(format!(
            "operator contract inconsistency: unsupported status `{}`",
            normalized_status
        ));
    }
    operator_contracts_consistency_error(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        status,
        blocker_codes,
        next_actions,
    )
}

pub(crate) fn shared_operator_output_contract_parity_error(
    summary_json: &Value,
) -> Option<&'static str> {
    operator_output::operator_contracts::operator_output_contract_parity_error(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        summary_json,
        canonical_release1_operator_blocker_codes,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Release1OperatorOutputBuilder, build_release1_operator_output_payload,
        canonical_release1_blocker_code_entries, canonical_release1_operator_blocker_codes,
        canonical_release1_operator_contract_status,
        finalize_release1_operator_surface_verdict_with_status, finalize_release1_operator_truth,
        release1_operator_contracts_consistency_error,
        shared_operator_output_contract_parity_error,
    };
    use serde_json::json;

    #[test]
    fn release1_operator_output_payload_builds_canonical_mirrors_once() {
        let payload = build_release1_operator_output_payload(
            "surface",
            vec!["missing_artifact".to_string()],
            vec!["inspect".to_string()],
            json!({"path": "artifact.json"}),
            json!({"extra": true}),
        )
        .expect("payload");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn release1_operator_output_builder_protects_contract_keys_from_extra_fields() {
        let payload = Release1OperatorOutputBuilder::new("surface")
            .blocker_codes(vec!["missing_structured_proof_evidence".to_string()])
            .next_actions(vec!["attach proof".to_string()])
            .artifact_refs(json!({"surface": "surface", "task_id": "task-1"}))
            .extra_fields(json!({
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "shared_fields": {"status": "pass"},
                "operator_contracts": {"status": "pass"},
                "extra": true,
            }))
            .build()
            .expect("payload");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            json!(["missing_structured_proof_evidence"])
        );
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(payload["extra"], true);
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn release1_operator_output_builder_uses_same_contract_skeleton_for_pass_and_blocked() {
        let pass_payload = Release1OperatorOutputBuilder::new("surface")
            .extra_fields(json!({"payload_kind": "pass"}))
            .build()
            .expect("pass payload");
        let blocked_payload = Release1OperatorOutputBuilder::new("surface")
            .blocker_codes(vec!["missing_artifact".to_string()])
            .next_actions(vec!["inspect artifact".to_string()])
            .extra_fields(json!({"payload_kind": "blocked"}))
            .build()
            .expect("blocked payload");
        let mut pass_keys = pass_payload
            .as_object()
            .expect("pass payload object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut blocked_keys = blocked_payload
            .as_object()
            .expect("blocked payload object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        pass_keys.sort();
        blocked_keys.sort();

        assert_eq!(pass_keys, blocked_keys);
        for key in [
            "status",
            "trace_id",
            "workflow_class",
            "risk_tier",
            "blocker_codes",
            "next_actions",
            "artifact_refs",
        ] {
            assert_eq!(
                pass_payload[key], pass_payload["shared_fields"][key],
                "pass shared_fields should mirror {key}"
            );
            assert_eq!(
                blocked_payload[key], blocked_payload["shared_fields"][key],
                "blocked shared_fields should mirror {key}"
            );
            assert_eq!(
                pass_payload[key], pass_payload["operator_contracts"][key],
                "pass operator_contracts should mirror {key}"
            );
            assert_eq!(
                blocked_payload[key], blocked_payload["operator_contracts"][key],
                "blocked operator_contracts should mirror {key}"
            );
        }
        assert_eq!(
            shared_operator_output_contract_parity_error(&pass_payload),
            None
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&blocked_payload),
            None
        );
    }

    #[test]
    fn release1_surface_verdict_normalizes_unknown_blockers() {
        let verdict = finalize_release1_operator_surface_verdict_with_status(
            "blocked",
            vec!["unknown blocker".to_string()],
            vec!["inspect".to_string()],
            json!({}),
        )
        .expect("verdict");

        assert_eq!(verdict.status, "blocked");
        assert_eq!(verdict.blocker_codes, vec!["unsupported_blocker_code"]);
    }

    #[test]
    fn run_graph_operator_blocker_codes_are_registry_backed() {
        let entries = crate::release1_contracts::run_graph_operator_blocker_code_strings()
            .iter()
            .map(|code| format!(" {code} "))
            .collect::<Vec<_>>();
        let normalized = canonical_release1_operator_blocker_codes(&entries);

        assert_eq!(
            normalized,
            crate::release1_contracts::run_graph_operator_blocker_code_strings()
                .iter()
                .map(|code| (*code).to_string())
                .collect::<Vec<_>>()
        );
        for code in crate::release1_contracts::run_graph_operator_blocker_code_strings() {
            assert_eq!(
                canonical_release1_blocker_code_entries(&json!([code])),
                Some(vec![(*code).to_string()])
            );
        }
    }

    #[test]
    fn release1_truth_derives_pass_without_blockers() {
        let finalized =
            finalize_release1_operator_truth(Vec::new(), Vec::new(), json!({})).expect("truth");

        assert_eq!(finalized.status, "pass");
        assert_eq!(
            canonical_release1_operator_contract_status(&json!("ok")),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_blocker_code_entries(&json!(["migration_required"])),
            Some(vec!["migration_required".to_string()])
        );
        assert_eq!(
            canonical_release1_blocker_code_entries(&json!(["missing_structured_proof_evidence"])),
            Some(vec!["missing_structured_proof_evidence".to_string()])
        );
        assert_eq!(
            canonical_release1_blocker_code_entries(&json!([
                "foreign_claim_conflict_blocked",
                "task_tree_traversal_failed"
            ])),
            Some(vec![
                "foreign_claim_conflict_blocked".to_string(),
                "task_tree_traversal_failed".to_string()
            ])
        );
        assert_eq!(
            release1_operator_contracts_consistency_error("pass", &[], &[]),
            None
        );
    }
}
