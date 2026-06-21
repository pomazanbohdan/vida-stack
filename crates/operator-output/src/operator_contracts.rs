use serde_json::Value;

pub struct OperatorContractSpec {
    pub contract_id: &'static str,
    pub schema_version: &'static str,
    pub pass_status: &'static str,
    pub blocked_status: &'static str,
    pub canonicalize_status: fn(&str) -> Option<&'static str>,
    pub status_error_label: &'static str,
}

pub const RELEASE1_OPERATOR_CONTRACT_SPEC: OperatorContractSpec = OperatorContractSpec {
    contract_id: "release-1-operator-contracts",
    schema_version: "release-1-v1",
    pass_status: "pass",
    blocked_status: "blocked",
    canonicalize_status: canonical_pass_blocked_contract_status_str,
    status_error_label: "canonical release-1 pass/blocked",
};

pub struct FinalizedRelease1OperatorTruth {
    pub status: &'static str,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub artifact_refs: Value,
    pub shared_fields: Value,
    pub operator_contracts: Value,
}

#[derive(Debug)]
pub struct OperatorSurfaceVerdict {
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub artifact_refs: Value,
    pub shared_fields: Value,
    pub operator_contracts: Value,
}

pub const VIDA_GATE_RESULT_SCHEMA_VERSION: &str = "vida-gate-result-v1";

pub fn canonical_pass_blocked_contract_status_str(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" | "ok" => Some("pass"),
        "blocked" | "block" => Some("blocked"),
        _ => None,
    }
}

fn inferred_gate_status(
    blocker_codes: &[String],
    warning_codes: &[String],
    failure_codes: &[String],
) -> &'static str {
    if !blocker_codes.is_empty() {
        RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status
    } else if !failure_codes.is_empty() {
        "fail"
    } else if !warning_codes.is_empty() {
        "warn"
    } else {
        RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_vida_gate_result_with_status(
    gate_id: &str,
    explicit_status: &str,
    blocker_codes: Vec<String>,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    let blocker_codes = canonical_gate_code_entries(blocker_codes);
    let warning_codes = canonical_gate_code_entries(warning_codes);
    let failure_codes = canonical_gate_code_entries(failure_codes);
    let next_actions =
        canonical_next_action_entries(&serde_json::json!(next_actions)).unwrap_or_default();
    let status = match explicit_status.trim() {
        "pass" | "warn" | "fail" | "blocked" | "insufficient_evidence" => explicit_status.trim(),
        _ => inferred_gate_status(&blocker_codes, &warning_codes, &failure_codes),
    };
    let operator_status = if matches!(status, "pass" | "warn") {
        RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status
    } else {
        RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status
    };
    let operator_next_actions = if operator_status == RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status {
        Vec::new()
    } else {
        next_actions.clone()
    };
    let mut operator_blocker_codes = blocker_codes.clone();
    operator_blocker_codes.extend(failure_codes.iter().cloned());
    operator_blocker_codes = canonical_gate_code_entries(operator_blocker_codes);
    let operator_contracts = render_operator_contract_envelope(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        operator_status,
        operator_blocker_codes,
        operator_next_actions,
        artifact_refs.clone(),
    );
    let trace_id = artifact_refs
        .get("trace_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let workflow_class = artifact_refs
        .get("workflow_class")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let risk_tier = artifact_refs
        .get("risk_tier")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let run_id = artifact_refs
        .get("run_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let task_id = artifact_refs
        .get("task_id")
        .cloned()
        .unwrap_or_else(|| run_id.clone());
    let packet_id = artifact_refs
        .get("packet_id")
        .or_else(|| artifact_refs.get("dispatch_packet_path"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let evidence_refs = artifact_refs
        .get("evidence_refs")
        .or_else(|| artifact_refs.get("proof_surfaces"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let affected_paths = artifact_refs
        .get("affected_paths")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "schema_version": VIDA_GATE_RESULT_SCHEMA_VERSION,
        "gate_id": gate_id.trim(),
        "status": status,
        "ready": matches!(status, "pass" | "warn"),
        "blocking": matches!(status, "fail" | "blocked" | "insufficient_evidence"),
        "trace_id": trace_id,
        "workflow_class": workflow_class,
        "risk_tier": risk_tier,
        "task_id": task_id,
        "run_id": run_id,
        "packet_id": packet_id,
        "evidence_refs": evidence_refs,
        "affected_paths": affected_paths,
        "blocker_codes": blocker_codes,
        "warning_codes": warning_codes,
        "failure_codes": failure_codes,
        "issues": issues,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "operator_contracts": operator_contracts,
    })
}

pub fn render_vida_gate_result(
    gate_id: &str,
    blocker_codes: Vec<String>,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    let blocker_codes = canonical_gate_code_entries(blocker_codes);
    let warning_codes = canonical_gate_code_entries(warning_codes);
    let failure_codes = canonical_gate_code_entries(failure_codes);
    let status = inferred_gate_status(&blocker_codes, &warning_codes, &failure_codes);
    render_vida_gate_result_with_status(
        gate_id,
        status,
        blocker_codes,
        warning_codes,
        failure_codes,
        issues,
        next_actions,
        artifact_refs,
    )
}

pub fn render_vida_gate_result_from_operator_contracts(
    gate_id: &str,
    mut operator_contracts: Value,
    warning_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<Value>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    let blocker_codes = operator_contracts["blocker_codes"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !failure_codes.is_empty()
        && operator_contracts["status"].as_str()
            == Some(RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status)
    {
        operator_contracts = render_operator_contract_envelope(
            &RELEASE1_OPERATOR_CONTRACT_SPEC,
            RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status,
            failure_codes.clone(),
            next_actions.clone(),
            artifact_refs.clone(),
        );
    }
    let mut gate_result = render_vida_gate_result(
        gate_id,
        blocker_codes,
        warning_codes,
        failure_codes,
        issues,
        next_actions,
        artifact_refs,
    );
    gate_result["operator_contracts"] = operator_contracts;
    gate_result["workflow_class"] = gate_result["operator_contracts"]["workflow_class"].clone();
    gate_result["risk_tier"] = gate_result["operator_contracts"]["risk_tier"].clone();
    gate_result["trace_id"] = gate_result["operator_contracts"]["trace_id"].clone();
    gate_result
}

fn canonical_default_blocker_codes(entries: &[String]) -> Vec<String> {
    const KNOWN_BLOCKERS: &[&str] = &[
        "host_tool_bridge_adapter_required",
        "closure_admission_block",
        "dispatch_packet_contract_invalid",
        "migration_required",
        "missing_artifact",
        "missing_gate_evidence",
        "unsupported_blocker_code",
    ];
    let mut canonical = entries
        .iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| KNOWN_BLOCKERS.contains(&entry.as_str()))
        .collect::<Vec<_>>();
    canonical.sort();
    canonical.dedup();
    canonical
}

fn canonical_gate_code_entries(entries: Vec<String>) -> Vec<String> {
    let mut canonical = entries
        .into_iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    canonical.sort();
    canonical.dedup();
    canonical
}

pub fn render_operator_contract_envelope(
    spec: &OperatorContractSpec,
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    let canonical_status =
        canonical_operator_contract_status_str(spec, status).unwrap_or(spec.blocked_status);
    serde_json::json!({
        "contract_id": spec.contract_id,
        "schema_version": spec.schema_version,
        "status": canonical_status,
        "trace_id": Value::Null,
        "workflow_class": Value::Null,
        "risk_tier": Value::Null,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
    })
}

pub fn finalize_operator_surface_verdict(
    spec: &OperatorContractSpec,
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> OperatorSurfaceVerdict {
    let operator_contracts =
        render_operator_contract_envelope(spec, status, blocker_codes, next_actions, artifact_refs);
    let status = operator_contracts["status"]
        .as_str()
        .unwrap_or(spec.blocked_status)
        .to_string();
    let blocker_codes = operator_contracts["blocker_codes"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let next_actions = operator_contracts["next_actions"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let artifact_refs = operator_contracts["artifact_refs"].clone();
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": operator_contracts["status"].clone(),
        "blocker_codes": operator_contracts["blocker_codes"].clone(),
        "next_actions": operator_contracts["next_actions"].clone(),
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
    });
    OperatorSurfaceVerdict {
        status,
        blocker_codes,
        next_actions,
        artifact_refs,
        shared_fields,
        operator_contracts,
    }
}

pub fn finalize_release1_operator_surface_verdict_with_status(
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
        canonical_default_blocker_codes,
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

pub fn finalize_release1_operator_truth(
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Result<FinalizedRelease1OperatorTruth, String> {
    let blocker_codes = normalize_blocker_codes(
        &blocker_codes,
        canonical_default_blocker_codes,
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

pub fn build_release1_operator_output_payload(
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

pub fn replace_release1_operator_output_artifact_refs(
    payload: &mut Value,
    artifact_refs: Value,
) -> Result<(), String> {
    write_release1_operator_output_artifact_refs(payload, artifact_refs)?;
    if let Some(error) = shared_operator_output_contract_parity_error(payload) {
        return Err(error.to_string());
    }
    Ok(())
}

pub fn write_release1_operator_output_artifact_refs(
    payload: &mut Value,
    artifact_refs: Value,
) -> Result<(), String> {
    {
        let object = payload
            .as_object_mut()
            .ok_or_else(|| "release-1 operator output payload should be an object".to_string())?;
        object.insert("artifact_refs".to_string(), artifact_refs.clone());
    }
    payload["shared_fields"]["artifact_refs"] = artifact_refs.clone();
    payload["operator_contracts"]["artifact_refs"] = artifact_refs;
    Ok(())
}

pub fn canonical_operator_contract_status_str<'a>(
    spec: &'a OperatorContractSpec,
    value: &str,
) -> Option<&'a str> {
    (spec.canonicalize_status)(value.trim())
}

pub fn canonical_operator_contract_status<'a>(
    spec: &'a OperatorContractSpec,
    value: &Value,
) -> Option<&'a str> {
    canonical_operator_contract_status_str(spec, value.as_str()?)
}

pub fn is_canonical_operator_contract_status(spec: &OperatorContractSpec, value: &Value) -> bool {
    canonical_operator_contract_status(spec, value).is_some()
}

fn canonical_blocker_candidates(
    value: &Value,
    canonicalize: fn(&[String]) -> Vec<String>,
) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    let canonical = canonicalize(&entries);
    if canonical.len() != entries.len() {
        return None;
    }
    Some(canonical)
}

pub fn canonical_blocker_code_entries(
    value: &Value,
    canonicalize: fn(&[String]) -> Vec<String>,
) -> Option<Vec<String>> {
    canonical_blocker_candidates(value, canonicalize)
}

pub fn is_canonical_blocker_code_entries(
    value: &Value,
    canonicalize: fn(&[String]) -> Vec<String>,
) -> bool {
    canonical_blocker_code_entries(value, canonicalize).is_some()
}

pub fn canonical_next_action_entries(value: &Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return None;
        }
        entries.push(trimmed.to_ascii_lowercase());
    }
    Some(entries)
}

pub fn is_canonical_next_action_entries(value: &Value) -> bool {
    canonical_next_action_entries(value).is_some()
}

pub fn normalize_blocker_codes(
    blockers: &[String],
    canonicalize: fn(&[String]) -> Vec<String>,
    unsupported_fallback: Option<String>,
) -> Vec<String> {
    let canonical = canonicalize(blockers);
    if canonical.is_empty() && !blockers.is_empty() {
        return unsupported_fallback.into_iter().collect();
    }
    canonical
}

pub fn operator_contract_status_for_blockers<'a>(
    spec: &'a OperatorContractSpec,
    blockers: &[String],
) -> &'a str {
    if blockers.is_empty() {
        spec.pass_status
    } else {
        spec.blocked_status
    }
}

pub fn operator_contract_status_is_blocked(spec: &OperatorContractSpec, value: &Value) -> bool {
    canonical_operator_contract_status(spec, value) == Some(spec.blocked_status)
}

pub fn operator_contracts_consistency_error(
    spec: &OperatorContractSpec,
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    let Some(canonical_status) = canonical_operator_contract_status_str(spec, status) else {
        return Some(format!(
            "operator contract inconsistency: status must be {}",
            spec.status_error_label
        ));
    };
    let string_is_canonical_nonempty = |value: &String| {
        let trimmed = value.trim();
        !trimmed.is_empty() && trimmed == value
    };

    if !blocker_codes.iter().all(string_is_canonical_nonempty)
        || !next_actions.iter().all(string_is_canonical_nonempty)
    {
        return Some(
            "operator contract inconsistency: shared string arrays must contain only canonical nonempty entries"
                .to_string(),
        );
    }

    match canonical_status {
        status if status == spec.pass_status && !blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=pass must not include blocker_codes"
                .to_string(),
        ),
        status if status == spec.pass_status && !next_actions.is_empty() => Some(
            "operator contract inconsistency: status=pass must not include next_actions"
                .to_string(),
        ),
        status if status == spec.pass_status => None,
        status if status == spec.blocked_status && blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires blocker_codes".to_string(),
        ),
        status if status == spec.blocked_status && next_actions.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires next_actions".to_string(),
        ),
        status if status == spec.blocked_status => None,
        _ => unreachable!("canonical operator contract status should match configured statuses"),
    }
}

pub fn canonical_release1_operator_contract_status(value: &Value) -> Option<&'static str> {
    canonical_operator_contract_status(&RELEASE1_OPERATOR_CONTRACT_SPEC, value)
}

fn canonical_release1_blocker_candidates(value: &Value) -> Option<Vec<String>> {
    canonical_blocker_candidates(value, canonical_default_blocker_codes)
}

pub fn canonical_release1_blocker_code_entries(value: &Value) -> Option<Vec<String>> {
    canonical_release1_blocker_candidates(value)
}

pub fn release1_operator_contracts_consistency_error(
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

pub fn operator_output_contract_parity_error(
    spec: &OperatorContractSpec,
    summary_json: &Value,
    canonicalize_blockers: fn(&[String]) -> Vec<String>,
) -> Option<&'static str> {
    let shared = &summary_json["shared_fields"];
    let contracts = &summary_json["operator_contracts"];
    let status_value = &summary_json["status"];
    let upper_blocker_codes = &summary_json["blocker_codes"];
    let upper_next_actions = &summary_json["next_actions"];
    let Some(top_status) = canonical_operator_contract_status(spec, status_value) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(shared_status) = canonical_operator_contract_status(spec, &shared["status"]) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_status) = canonical_operator_contract_status(spec, &contracts["status"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let status_has_canonical_mirror =
        shared_operator_has_canonical_status(spec, summary_json, shared, contracts);
    let Some(top_blocker_codes) =
        canonical_blocker_code_entries(upper_blocker_codes, canonicalize_blockers)
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let blocker_codes_has_canonical_mirror = shared_operator_has_canonical_blockers(
        summary_json,
        shared,
        contracts,
        canonicalize_blockers,
    );
    let Some(top_next_actions) = canonical_next_action_entries(upper_next_actions) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let next_actions_has_canonical_mirror =
        shared_operator_has_canonical_next_actions(summary_json, shared, contracts);
    let Some(shared_blocker_codes) =
        canonical_blocker_code_entries(&shared["blocker_codes"], canonicalize_blockers)
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(shared_next_actions) = canonical_next_action_entries(&shared["next_actions"]) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_blocker_codes) =
        canonical_blocker_code_entries(&contracts["blocker_codes"], canonicalize_blockers)
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_next_actions) = canonical_next_action_entries(&contracts["next_actions"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    if top_status == shared_status
        && shared_status == contract_status
        && top_blocker_codes == shared_blocker_codes
        && shared_blocker_codes == contract_blocker_codes
        && top_next_actions == shared_next_actions
        && shared_next_actions == contract_next_actions
        && status_has_canonical_mirror
        && blocker_codes_has_canonical_mirror
        && next_actions_has_canonical_mirror
        && has_raw_canonical_blocker_entries(upper_blocker_codes, canonicalize_blockers)
        && has_raw_canonical_blocker_entries(&shared["blocker_codes"], canonicalize_blockers)
        && has_raw_canonical_blocker_entries(&contracts["blocker_codes"], canonicalize_blockers)
    {
        return None;
    }
    Some(
        "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
    )
}

pub fn shared_operator_output_contract_parity_error(summary_json: &Value) -> Option<&'static str> {
    operator_output_contract_parity_error(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        summary_json,
        canonical_default_blocker_codes,
    )
}

fn shared_operator_has_canonical_status(
    spec: &OperatorContractSpec,
    top: &Value,
    shared: &Value,
    contract: &Value,
) -> bool {
    is_canonical_operator_contract_status(spec, &top["status"])
        || is_canonical_operator_contract_status(spec, &shared["status"])
        || is_canonical_operator_contract_status(spec, &contract["status"])
}

fn shared_operator_has_canonical_blockers(
    top: &Value,
    shared: &Value,
    contract: &Value,
    canonicalize_blockers: fn(&[String]) -> Vec<String>,
) -> bool {
    is_canonical_blocker_code_entries(&top["blocker_codes"], canonicalize_blockers)
        || is_canonical_blocker_code_entries(&shared["blocker_codes"], canonicalize_blockers)
        || is_canonical_blocker_code_entries(&contract["blocker_codes"], canonicalize_blockers)
}

fn shared_operator_has_canonical_next_actions(
    top: &Value,
    shared: &Value,
    contract: &Value,
) -> bool {
    is_canonical_next_action_entries(&top["next_actions"])
        || is_canonical_next_action_entries(&shared["next_actions"])
        || is_canonical_next_action_entries(&contract["next_actions"])
}

fn has_raw_canonical_blocker_entries(
    value: &Value,
    canonicalize_blockers: fn(&[String]) -> Vec<String>,
) -> bool {
    let Some(rows) = value.as_array() else {
        return false;
    };
    let Some(canonical) = canonical_blocker_code_entries(value, canonicalize_blockers) else {
        return false;
    };
    let raw = rows
        .iter()
        .filter_map(|row| row.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    raw == canonical
}

#[cfg(test)]
mod tests {
    use super::{
        OperatorContractSpec, RELEASE1_OPERATOR_CONTRACT_SPEC, VIDA_GATE_RESULT_SCHEMA_VERSION,
        build_release1_operator_output_payload, canonical_blocker_code_entries,
        canonical_default_blocker_codes, canonical_next_action_entries,
        canonical_operator_contract_status, canonical_pass_blocked_contract_status_str,
        canonical_release1_blocker_code_entries, canonical_release1_operator_contract_status,
        finalize_operator_surface_verdict, finalize_release1_operator_surface_verdict_with_status,
        finalize_release1_operator_truth, normalize_blocker_codes,
        operator_contract_status_for_blockers, operator_contracts_consistency_error,
        release1_operator_contracts_consistency_error, render_operator_contract_envelope,
        render_vida_gate_result, render_vida_gate_result_from_operator_contracts,
        render_vida_gate_result_with_status, replace_release1_operator_output_artifact_refs,
        shared_operator_output_contract_parity_error,
    };
    use serde_json::json;

    #[test]
    fn canonical_operator_status_recognizes_pass_ok_and_blocked() {
        let value = json!("ok");
        assert_eq!(
            canonical_release1_operator_contract_status(&value),
            Some("pass")
        );
        let value = json!("blocked");
        assert_eq!(
            canonical_release1_operator_contract_status(&value),
            Some("blocked")
        );
    }

    #[test]
    fn canonical_operator_status_rejects_invalid_value() {
        let value = json!("unknown");
        assert_eq!(canonical_release1_operator_contract_status(&value), None);
        assert!(
            canonical_operator_contract_status(&RELEASE1_OPERATOR_CONTRACT_SPEC, &value).is_none()
        );
    }

    #[test]
    fn release1_operator_output_payload_builds_canonical_mirrors_once() {
        let payload = build_release1_operator_output_payload(
            "vida task ready",
            vec![
                " closure_admission_block ".to_string(),
                "dispatch_packet_contract_invalid".to_string(),
            ],
            vec![" Inspect task ".to_string()],
            json!({
                "surface": "vida task ready",
                "trace_id": "trace-1",
            }),
            json!({
                "task_count": 2,
            }),
        )
        .expect("release-1 operator payload should build");

        assert_eq!(payload["surface"], "vida task ready");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["task_count"], 2);
        assert_eq!(payload["trace_id"], serde_json::Value::Null);
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task ready");
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["surface"],
            "vida task ready"
        );
        assert_eq!(
            payload["blocker_codes"],
            json!([
                "closure_admission_block",
                "dispatch_packet_contract_invalid"
            ])
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn generic_operator_surface_verdict_builds_shared_and_contract_mirrors() {
        let spec = OperatorContractSpec {
            contract_id: "custom-operator-contract",
            schema_version: "1",
            pass_status: "pass",
            blocked_status: "blocked",
            canonicalize_status: canonical_pass_blocked_contract_status_str,
            status_error_label: "canonical pass/blocked",
        };
        let verdict = finalize_operator_surface_verdict(
            &spec,
            "block",
            vec!["custom_blocker".to_string()],
            vec!["repair custom surface".to_string()],
            json!({"surface": "vida custom"}),
        );

        assert_eq!(verdict.status, "blocked");
        assert_eq!(verdict.blocker_codes, vec!["custom_blocker".to_string()]);
        assert_eq!(
            verdict.next_actions,
            vec!["repair custom surface".to_string()]
        );
        assert_eq!(verdict.artifact_refs["surface"], "vida custom");
        assert_eq!(verdict.shared_fields["status"], "blocked");
        assert_eq!(
            verdict.shared_fields["blocker_codes"],
            verdict.operator_contracts["blocker_codes"]
        );
        assert_eq!(
            verdict.shared_fields["next_actions"],
            verdict.operator_contracts["next_actions"]
        );
        assert_eq!(
            verdict.shared_fields["artifact_refs"],
            verdict.operator_contracts["artifact_refs"]
        );
        assert_eq!(
            verdict.operator_contracts["contract_id"],
            "custom-operator-contract"
        );
    }

    #[test]
    fn release1_operator_surface_verdict_with_status_canonicalizes_and_validates() {
        let verdict = finalize_release1_operator_surface_verdict_with_status(
            "blocked",
            vec![" Migration_Required ".to_string()],
            vec![" Run migration ".to_string()],
            json!({"surface": "vida status"}),
        )
        .expect("release-1 verdict should build");

        assert_eq!(verdict.status, "blocked");
        assert_eq!(
            verdict.blocker_codes,
            vec!["migration_required".to_string()]
        );
        assert_eq!(verdict.next_actions, vec!["run migration".to_string()]);
        assert_eq!(
            verdict.shared_fields["artifact_refs"],
            verdict.operator_contracts["artifact_refs"]
        );

        let error = finalize_release1_operator_surface_verdict_with_status(
            "pass",
            Vec::new(),
            vec!["should not be present".to_string()],
            json!({}),
        )
        .expect_err("pass verdicts must not carry next actions");
        assert!(error.contains("status=pass must not include next_actions"));
    }

    #[test]
    fn release1_operator_output_payload_replaces_artifact_refs_in_all_mirrors() {
        let mut payload = build_release1_operator_output_payload(
            "vida task ready",
            Vec::new(),
            Vec::new(),
            json!({
                "surface": "vida task ready",
            }),
            json!({}),
        )
        .expect("release-1 operator payload should build");

        replace_release1_operator_output_artifact_refs(
            &mut payload,
            json!({
                "surface": "vida task ready",
                "snapshot": "updated",
            }),
        )
        .expect("artifact refs should replace in all mirrors");

        assert_eq!(payload["artifact_refs"]["snapshot"], "updated");
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["snapshot"],
            "updated"
        );
        assert_eq!(
            payload["operator_contracts"]["artifact_refs"]["snapshot"],
            "updated"
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn generic_operator_contract_envelope_preserves_release1_shape() {
        let envelope = render_operator_contract_envelope(
            &RELEASE1_OPERATOR_CONTRACT_SPEC,
            "ok",
            vec![],
            vec![],
            json!({"proof": "present"}),
        );
        assert_eq!(
            envelope["contract_id"],
            json!("release-1-operator-contracts")
        );
        assert_eq!(envelope["schema_version"], json!("release-1-v1"));
        assert_eq!(envelope["status"], json!("pass"));
    }

    #[test]
    fn vida_gate_result_explicit_insufficient_evidence_fails_closed() {
        let gate_result = render_vida_gate_result_with_status(
            "evidence",
            "insufficient_evidence",
            vec!["missing_gate_evidence".to_string()],
            vec![],
            vec![],
            vec![json!({"code": "insufficient_evidence"})],
            vec!["Provide evidence refs.".to_string()],
            json!({
                "evidence_refs": [],
                "affected_paths": [],
                "task_id": "task-1",
            }),
        );

        assert_eq!(gate_result["status"], json!("insufficient_evidence"));
        assert_eq!(gate_result["ready"], json!(false));
        assert_eq!(gate_result["blocking"], json!(true));
        assert_eq!(
            gate_result["blocker_codes"][0],
            json!("missing_gate_evidence")
        );
        assert_eq!(
            gate_result["operator_contracts"]["status"],
            json!("blocked")
        );
    }

    #[test]
    fn vida_gate_result_keeps_warning_only_gate_pass_without_operator_next_actions() {
        let gate_result = render_vida_gate_result(
            "docflow",
            vec![],
            vec![" Proof_Warning ".to_string()],
            vec![],
            vec![json!({"code": "proof_warning", "severity": "warning"})],
            vec![" Refresh docflow evidence. ".to_string()],
            json!({
                "proof": "present",
                "evidence_refs": ["docflow-proof"],
                "affected_paths": ["docs/process/docflow.md"],
                "task_id": "task-1",
                "run_id": "run-1",
                "packet_id": "packet-1",
                "risk_tier": "low",
                "workflow_class": "tool_assisted_read"
            }),
        );

        assert_eq!(
            gate_result["schema_version"],
            json!(VIDA_GATE_RESULT_SCHEMA_VERSION)
        );
        assert_eq!(gate_result["status"], json!("warn"));
        assert_eq!(gate_result["ready"], json!(true));
        assert_eq!(gate_result["blocking"], json!(false));
        assert_eq!(gate_result["warning_codes"], json!(["proof_warning"]));
        assert_eq!(gate_result["task_id"], json!("task-1"));
        assert_eq!(gate_result["run_id"], json!("run-1"));
        assert_eq!(gate_result["packet_id"], json!("packet-1"));
        assert_eq!(gate_result["risk_tier"], json!("low"));
        assert_eq!(gate_result["workflow_class"], json!("tool_assisted_read"));
        assert_eq!(gate_result["evidence_refs"], json!(["docflow-proof"]));
        assert_eq!(
            gate_result["affected_paths"],
            json!(["docs/process/docflow.md"])
        );
        assert_eq!(
            gate_result["next_actions"],
            json!(["refresh docflow evidence."])
        );
        assert_eq!(gate_result["operator_contracts"]["status"], json!("pass"));
        assert_eq!(gate_result["operator_contracts"]["next_actions"], json!([]));
    }

    #[test]
    fn vida_gate_result_mirrors_blockers_into_operator_contracts() {
        let gate_result = render_vida_gate_result(
            "consume-final",
            vec![" Migration_Required ".to_string()],
            vec![],
            vec![],
            vec![json!({"code": "migration_required", "severity": "blocker"})],
            vec![" Resolve migration. ".to_string()],
            json!({"proof": "present"}),
        );

        assert_eq!(gate_result["status"], json!("blocked"));
        assert_eq!(gate_result["ready"], json!(false));
        assert_eq!(gate_result["blocking"], json!(true));
        assert_eq!(gate_result["blocker_codes"], json!(["migration_required"]));
        assert_eq!(
            gate_result["operator_contracts"]["blocker_codes"],
            json!(["migration_required"])
        );
        assert_eq!(
            gate_result["operator_contracts"]["next_actions"],
            json!(["resolve migration."])
        );
    }

    #[test]
    fn vida_gate_result_reports_failure_while_legacy_projection_blocks() {
        let gate_result = render_vida_gate_result(
            "consume-final",
            vec![],
            vec![],
            vec![" Failure_Control_Evidence_Present ".to_string()],
            vec![json!({"code": "failure_control_evidence_present", "severity": "failure"})],
            vec![" Inspect dispatch receipt. ".to_string()],
            json!({
                "run_id": "run-2",
                "task_id": "task-2",
                "dispatch_packet_path": "packet.json",
                "evidence_refs": ["dispatch-result"],
                "affected_paths": ["crates/vida/src/operator_contracts.rs"]
            }),
        );

        assert_eq!(gate_result["status"], json!("fail"));
        assert_eq!(gate_result["ready"], json!(false));
        assert_eq!(gate_result["blocking"], json!(true));
        assert_eq!(
            gate_result["failure_codes"],
            json!(["failure_control_evidence_present"])
        );
        assert_eq!(gate_result["packet_id"], json!("packet.json"));
        assert_eq!(
            gate_result["operator_contracts"]["status"],
            json!("blocked")
        );
        assert_eq!(
            gate_result["operator_contracts"]["blocker_codes"],
            json!(["failure_control_evidence_present"])
        );
        assert_eq!(
            gate_result["operator_contracts"]["next_actions"],
            json!(["inspect dispatch receipt."])
        );
    }

    #[test]
    fn vida_gate_result_from_operator_contracts_preserves_projection() {
        let operator_contracts = render_operator_contract_envelope(
            &RELEASE1_OPERATOR_CONTRACT_SPEC,
            "blocked",
            vec!["migration_required".to_string()],
            vec!["resolve migration".to_string()],
            json!({"proof": "present"}),
        );
        let gate_result = render_vida_gate_result_from_operator_contracts(
            "consume-final",
            operator_contracts.clone(),
            vec![],
            vec![],
            vec![],
            vec!["resolve migration".to_string()],
            operator_contracts["artifact_refs"].clone(),
        );

        assert_eq!(gate_result["operator_contracts"], operator_contracts);
        assert_eq!(gate_result["status"], json!("blocked"));
        assert_eq!(gate_result["ready"], json!(false));
    }

    #[test]
    fn finalize_release1_operator_truth_derives_blocked_and_shared_fields() {
        let finalized = finalize_release1_operator_truth(
            vec!["migration_required".to_string()],
            vec![" Complete required migration before normal operation. ".to_string()],
            json!({"proof": "present"}),
        )
        .expect("finalization should succeed");

        assert_eq!(finalized.status, "blocked");
        assert_eq!(
            finalized.blocker_codes,
            vec!["migration_required".to_string()]
        );
        assert_eq!(
            finalized.next_actions,
            vec!["complete required migration before normal operation.".to_string()]
        );
        assert_eq!(finalized.shared_fields["status"], "blocked");
        assert_eq!(
            finalized.shared_fields["blocker_codes"],
            json!(["migration_required"])
        );
        assert_eq!(
            finalized.shared_fields["next_actions"],
            json!(["complete required migration before normal operation."])
        );
    }

    #[test]
    fn finalize_release1_operator_truth_preserves_host_tool_bridge_adapter_required_blocker() {
        let finalized = finalize_release1_operator_truth(
            vec!["host_tool_bridge_adapter_required".to_string()],
            vec!["materialize the host tool bridge adapter before dispatch".to_string()],
            json!({"surface": "run_graph_recovery"}),
        )
        .expect("finalization should preserve registered host bridge blocker");

        assert_eq!(finalized.status, "blocked");
        assert_eq!(
            finalized.blocker_codes,
            vec!["host_tool_bridge_adapter_required".to_string()]
        );
        assert_eq!(
            finalized.shared_fields["blocker_codes"],
            json!(["host_tool_bridge_adapter_required"])
        );
        assert_eq!(
            finalized.operator_contracts["blocker_codes"],
            json!(["host_tool_bridge_adapter_required"])
        );
    }

    #[test]
    fn finalize_release1_operator_truth_maps_unknown_only_blockers_to_unsupported() {
        let finalized = finalize_release1_operator_truth(
            vec!["unregistered_runtime_blocker".to_string()],
            vec!["repair the emitting surface before continuing".to_string()],
            json!({"surface": "unknown_blocker_regression"}),
        )
        .expect("unknown blocker should remain a blocked operator truth");

        assert_eq!(finalized.status, "blocked");
        assert_eq!(
            finalized.blocker_codes,
            vec!["unsupported_blocker_code".to_string()]
        );
        assert_eq!(
            finalized.shared_fields["blocker_codes"],
            json!(["unsupported_blocker_code"])
        );
        assert_eq!(
            finalized.operator_contracts["blocker_codes"],
            json!(["unsupported_blocker_code"])
        );
    }

    #[test]
    fn finalize_release1_operator_surface_with_status_maps_unknown_blockers_to_unsupported() {
        let verdict = finalize_release1_operator_surface_verdict_with_status(
            "blocked",
            vec!["unregistered_runtime_blocker".to_string()],
            vec!["repair the emitting surface before continuing".to_string()],
            json!({"surface": "unknown_blocker_regression"}),
        )
        .expect("unknown blocker should not collapse an explicitly blocked surface");

        assert_eq!(verdict.status, "blocked");
        assert_eq!(
            verdict.blocker_codes,
            vec!["unsupported_blocker_code".to_string()]
        );
        assert_eq!(
            verdict.operator_contracts["blocker_codes"],
            json!(["unsupported_blocker_code"])
        );
    }

    #[test]
    fn finalize_release1_operator_truth_derives_pass_without_blockers() {
        let finalized = finalize_release1_operator_truth(vec![], vec![], json!({}))
            .expect("finalization should succeed");

        assert_eq!(finalized.status, "pass");
        assert_eq!(finalized.blocker_codes, Vec::<String>::new());
        assert_eq!(finalized.next_actions, Vec::<String>::new());
        assert_eq!(finalized.shared_fields["status"], "pass");
    }

    #[test]
    fn canonical_blocker_codes_require_lower_snake_case() {
        let value = json!(["migration_required"]);
        assert_eq!(
            canonical_release1_blocker_code_entries(&value),
            Some(vec!["migration_required".into()])
        );
        let value = json!(["INVALID"]);
        assert!(canonical_release1_blocker_code_entries(&value).is_none());
    }

    #[test]
    fn canonical_blocker_codes_must_be_registry_backed() {
        let value = json!(["valid_code"]);
        assert!(canonical_release1_blocker_code_entries(&value).is_none());
    }

    #[test]
    fn generic_blocker_normalization_falls_back_to_unsupported_code() {
        let normalized = normalize_blocker_codes(
            &["unknown_code".to_string()],
            canonical_default_blocker_codes,
            Some("unsupported_blocker_code".to_string()),
        );
        assert_eq!(normalized, vec!["unsupported_blocker_code".to_string()]);
    }

    #[test]
    fn canonical_next_actions_downcases_and_trims() {
        let value = json!([" Run `task` "]);
        assert_eq!(
            canonical_next_action_entries(&value),
            Some(vec!["run `task`".into()])
        );
        assert!(canonical_next_action_entries(&value).is_some());
    }

    #[test]
    fn generic_helpers_match_release1_contract_semantics() {
        let value = json!("ok");
        assert_eq!(
            canonical_operator_contract_status(&RELEASE1_OPERATOR_CONTRACT_SPEC, &value),
            Some("pass")
        );
        assert_eq!(
            canonical_blocker_code_entries(&json!(["migration_required"]), |entries| {
                canonical_default_blocker_codes(entries)
            },),
            Some(vec!["migration_required".into()])
        );
        assert_eq!(
            canonical_next_action_entries(&json!([" Run check "])),
            Some(vec!["run check".into()])
        );
        assert_eq!(
            operator_contract_status_for_blockers(
                &RELEASE1_OPERATOR_CONTRACT_SPEC,
                &["migration_required".to_string()],
            ),
            "blocked"
        );
    }

    #[test]
    fn release1_consistency_accepts_valid_blocked_contract() {
        let blocker_codes = vec!["migration_required".into()];
        let next_actions = vec!["reconcile migration".into()];
        assert_eq!(
            release1_operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions,),
            None
        );
    }

    #[test]
    fn generic_consistency_matches_release1_error_contract() {
        let blocker_codes = vec!["migration_required".to_string()];
        let next_actions = vec!["reconcile migration".to_string()];
        assert_eq!(
            operator_contracts_consistency_error(
                &RELEASE1_OPERATOR_CONTRACT_SPEC,
                "blocked",
                &blocker_codes,
                &next_actions,
            ),
            None
        );
    }

    #[test]
    fn shared_parity_detects_mismatch() {
        let summary_json = json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["migration_required"],
                "next_actions": ["resolve migration"],
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }

    #[test]
    fn shared_parity_accepts_next_action_case_and_whitespace_drift() {
        let summary_json = json!({
            "status": "blocked",
            "blocker_codes": ["migration_required"],
            "next_actions": [" Run proofcheck "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["migration_required"],
                "next_actions": ["run proofcheck"],
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["migration_required"],
                "next_actions": ["run proofcheck"],
            }
        });

        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            None
        );
    }
}
