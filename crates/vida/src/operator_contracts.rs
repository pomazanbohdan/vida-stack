use serde_json::Value;

pub(crate) struct OperatorContractSpec {
    pub(crate) contract_id: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) pass_status: &'static str,
    pub(crate) blocked_status: &'static str,
    pub(crate) canonicalize_status: fn(&str) -> Option<&'static str>,
    pub(crate) status_error_label: &'static str,
}

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

pub(crate) fn render_operator_contract_envelope(
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

pub(crate) fn finalize_release1_operator_truth(
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Result<FinalizedRelease1OperatorTruth, String> {
    let blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let next_actions =
        canonical_next_action_entries(&serde_json::json!(next_actions)).unwrap_or(next_actions);
    let status = if blocker_codes.is_empty() {
        RELEASE1_OPERATOR_CONTRACT_SPEC.pass_status
    } else {
        RELEASE1_OPERATOR_CONTRACT_SPEC.blocked_status
    };
    let operator_contracts = render_operator_contract_envelope(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs.clone(),
    );
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
    if let Some(error) = release1_operator_contracts_consistency_error(
        operator_contracts["status"].as_str().unwrap_or(""),
        &blocker_codes,
        &next_actions,
    ) {
        return Err(error);
    }
    let shared_fields = serde_json::json!({
        "status": operator_contracts["status"].clone(),
        "blocker_codes": operator_contracts["blocker_codes"].clone(),
        "next_actions": operator_contracts["next_actions"].clone(),
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
    });
    Ok(FinalizedRelease1OperatorTruth {
        status,
        blocker_codes,
        next_actions,
        artifact_refs,
        shared_fields,
        operator_contracts,
    })
}

pub(crate) fn canonical_operator_contract_status_str<'a>(
    spec: &'a OperatorContractSpec,
    value: &str,
) -> Option<&'a str> {
    (spec.canonicalize_status)(value.trim())
}

pub(crate) fn canonical_operator_contract_status<'a>(
    spec: &'a OperatorContractSpec,
    value: &Value,
) -> Option<&'a str> {
    canonical_operator_contract_status_str(spec, value.as_str()?)
}

pub(crate) fn is_canonical_operator_contract_status(
    spec: &OperatorContractSpec,
    value: &Value,
) -> bool {
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
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    let canonical = canonicalize(&entries);
    if canonical.len() != entries.len() || canonical != entries {
        return None;
    }
    Some(entries)
}

pub(crate) fn canonical_blocker_code_entries(
    value: &Value,
    canonicalize: fn(&[String]) -> Vec<String>,
) -> Option<Vec<String>> {
    canonical_blocker_candidates(value, canonicalize)
}

pub(crate) fn is_canonical_blocker_code_entries(
    value: &Value,
    canonicalize: fn(&[String]) -> Vec<String>,
) -> bool {
    canonical_blocker_code_entries(value, canonicalize).is_some()
}

pub(crate) fn canonical_next_action_entries(value: &Value) -> Option<Vec<String>> {
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

pub(crate) fn is_canonical_next_action_entries(value: &Value) -> bool {
    canonical_next_action_entries(value).is_some()
}

pub(crate) fn normalize_blocker_codes(
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

pub(crate) fn operator_contract_status_for_blockers<'a>(
    spec: &'a OperatorContractSpec,
    blockers: &[String],
) -> &'a str {
    if blockers.is_empty() {
        spec.pass_status
    } else {
        spec.blocked_status
    }
}

pub(crate) fn operator_contract_status_is_blocked(
    spec: &OperatorContractSpec,
    value: &Value,
) -> bool {
    canonical_operator_contract_status(spec, value) == Some(spec.blocked_status)
}

pub(crate) fn operator_contracts_consistency_error(
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

pub(crate) fn canonical_release1_operator_contract_status(value: &Value) -> Option<&'static str> {
    canonical_operator_contract_status(&RELEASE1_OPERATOR_CONTRACT_SPEC, value)
}

fn canonical_release1_blocker_candidates(value: &Value) -> Option<Vec<String>> {
    canonical_blocker_candidates(value, |entries| {
        crate::release1_contracts::canonical_blocker_code_list(entries)
    })
}

pub(crate) fn canonical_release1_blocker_code_entries(value: &Value) -> Option<Vec<String>> {
    canonical_release1_blocker_candidates(value)
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

pub(crate) fn operator_output_contract_parity_error(
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
    {
        return None;
    }
    Some(
        "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
    )
}

pub(crate) fn shared_operator_output_contract_parity_error(
    summary_json: &Value,
) -> Option<&'static str> {
    operator_output_contract_parity_error(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        summary_json,
        |entries| crate::release1_contracts::canonical_blocker_code_list(entries),
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

#[cfg(test)]
mod tests {
    use super::{
        canonical_blocker_code_entries, canonical_next_action_entries,
        canonical_operator_contract_status, canonical_release1_blocker_code_entries,
        canonical_release1_operator_contract_status, finalize_release1_operator_truth,
        normalize_blocker_codes, operator_contract_status_for_blockers,
        operator_contracts_consistency_error, release1_operator_contracts_consistency_error,
        render_operator_contract_envelope, shared_operator_output_contract_parity_error,
        RELEASE1_OPERATOR_CONTRACT_SPEC,
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
            |entries| crate::release1_contracts::canonical_blocker_code_list(entries),
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
                crate::release1_contracts::canonical_blocker_code_list(entries)
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
}
