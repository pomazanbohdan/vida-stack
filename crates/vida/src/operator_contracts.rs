use serde_json::Value;

pub(crate) use operator_output::operator_contracts::{
    canonical_blocker_code_entries, canonical_next_action_entries,
    canonical_operator_contract_status, canonical_operator_contract_status_str,
    canonical_pass_blocked_contract_status_str, finalize_operator_surface_verdict,
    is_canonical_blocker_code_entries, is_canonical_next_action_entries,
    is_canonical_operator_contract_status, normalize_blocker_codes,
    operator_contract_status_for_blockers, operator_contract_status_is_blocked,
    operator_contracts_consistency_error, operator_output_contract_parity_error,
    render_operator_contract_envelope, OperatorContractSpec, OperatorSurfaceVerdict,
};

pub(crate) const RELEASE1_OPERATOR_CONTRACT_SPEC: OperatorContractSpec = OperatorContractSpec {
    contract_id: "release-1-operator-contracts",
    schema_version: "release-1-v1",
    pass_status: "pass",
    blocked_status: "blocked",
    canonicalize_status: crate::release1_contracts::canonical_release1_contract_status_str,
    status_error_label: "canonical release-1 pass/blocked",
};

pub(crate) struct FinalizedRelease1OperatorTruth {
    pub(crate) status: &'static str,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) artifact_refs: Value,
    pub(crate) shared_fields: Value,
    pub(crate) operator_contracts: Value,
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
        crate::contract_profile_adapter::canonical_blocker_codes,
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
        crate::contract_profile_adapter::canonical_blocker_codes,
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
    let mut payload = serde_json::json!({
        "surface": surface,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    let extra_object = extra_fields
        .as_object()
        .ok_or_else(|| "release-1 operator output extra_fields must be an object".to_string())?
        .clone();
    payload
        .as_object_mut()
        .ok_or_else(|| "release-1 operator output payload should be an object".to_string())?
        .extend(extra_object);
    if let Some(error) = shared_operator_output_contract_parity_error(&payload) {
        return Err(error.to_string());
    }
    Ok(payload)
}

pub(crate) fn replace_release1_operator_output_artifact_refs(
    payload: &mut Value,
    artifact_refs: Value,
) -> Result<(), String> {
    operator_output::operator_contracts::replace_release1_operator_output_artifact_refs(
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
    canonical_blocker_code_entries(value, |entries| {
        crate::release1_contracts::canonical_blocker_code_list(entries)
    })
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
        |entries| crate::release1_contracts::canonical_blocker_code_list(entries),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_release1_operator_output_payload, canonical_release1_blocker_code_entries,
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
            release1_operator_contracts_consistency_error("pass", &[], &[]),
            None
        );
    }
}
