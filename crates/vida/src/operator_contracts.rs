use serde_json::Value;

pub(crate) fn canonical_release1_operator_contract_status(value: &Value) -> Option<&'static str> {
    crate::release1_contracts::canonical_release1_contract_status_str(value.as_str()?)
}

pub(crate) fn is_canonical_release1_operator_contract_status(value: &Value) -> bool {
    canonical_release1_operator_contract_status(value).is_some()
}

fn canonical_release1_blocker_candidates(value: &Value) -> Option<Vec<String>> {
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
    let canonical = crate::release1_contracts::canonical_blocker_code_list(&entries);
    if canonical.len() != entries.len() || canonical != entries {
        return None;
    }
    Some(entries)
}

pub(crate) fn canonical_release1_blocker_code_entries(value: &Value) -> Option<Vec<String>> {
    canonical_release1_blocker_candidates(value)
}

pub(crate) fn is_canonical_release1_blocker_code_entries(value: &Value) -> bool {
    canonical_release1_blocker_code_entries(value).is_some()
}

pub(crate) fn canonical_release1_next_action_entries(value: &Value) -> Option<Vec<String>> {
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

pub(crate) fn is_canonical_release1_next_action_entries(value: &Value) -> bool {
    canonical_release1_next_action_entries(value).is_some()
}

pub(crate) fn release1_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    let canonical_status = status.trim().to_ascii_lowercase();
    match canonical_status.as_str() {
        "pass" | "ok" => {
            if !blocker_codes.is_empty() {
                return Some(
                    "operator contract inconsistency: status=pass must not include blocker_codes"
                        .to_string(),
                );
            }
            if !next_actions.is_empty() {
                return Some(
                    "operator contract inconsistency: status=pass must not include next_actions"
                        .to_string(),
                );
            }
            None
        }
        "blocked" => {
            if blocker_codes.is_empty() {
                return Some(
                    "operator contract inconsistency: status=blocked requires blocker_codes"
                        .to_string(),
                );
            }
            if next_actions.is_empty() {
                return Some(
                    "operator contract inconsistency: status=blocked requires next_actions"
                        .to_string(),
                );
            }
            None
        }
        other => Some(format!(
            "operator contract inconsistency: unsupported status `{other}`"
        )),
    }
}

pub(crate) fn shared_operator_output_contract_parity_error(
    summary_json: &Value,
) -> Option<&'static str> {
    let shared = &summary_json["shared_fields"];
    let contracts = &summary_json["operator_contracts"];
    let status_value = &summary_json["status"];
    let upper_blocker_codes = &summary_json["blocker_codes"];
    let upper_next_actions = &summary_json["next_actions"];
    let Some(top_status) = canonical_release1_operator_contract_status(status_value) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(shared_status) = canonical_release1_operator_contract_status(&shared["status"]) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_status) = canonical_release1_operator_contract_status(&contracts["status"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let status_has_canonical_mirror =
        shared_operator_has_canonical_status(summary_json, shared, contracts);
    let Some(top_blocker_codes) = canonical_release1_blocker_code_entries(upper_blocker_codes)
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let blocker_codes_has_canonical_mirror =
        shared_operator_has_canonical_blockers(summary_json, shared, contracts);
    let Some(top_next_actions) = canonical_release1_next_action_entries(upper_next_actions) else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let next_actions_has_canonical_mirror =
        shared_operator_has_canonical_next_actions(summary_json, shared, contracts);
    let Some(shared_blocker_codes) =
        canonical_release1_blocker_code_entries(&shared["blocker_codes"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(shared_next_actions) = canonical_release1_next_action_entries(&shared["next_actions"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_blocker_codes) =
        canonical_release1_blocker_code_entries(&contracts["blocker_codes"])
    else {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    };
    let Some(contract_next_actions) =
        canonical_release1_next_action_entries(&contracts["next_actions"])
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

fn shared_operator_has_canonical_status(top: &Value, shared: &Value, contract: &Value) -> bool {
    is_canonical_release1_operator_contract_status(&top["status"])
        || is_canonical_release1_operator_contract_status(&shared["status"])
        || is_canonical_release1_operator_contract_status(&contract["status"])
}

fn shared_operator_has_canonical_blockers(top: &Value, shared: &Value, contract: &Value) -> bool {
    is_canonical_release1_blocker_code_entries(&top["blocker_codes"])
        || is_canonical_release1_blocker_code_entries(&shared["blocker_codes"])
        || is_canonical_release1_blocker_code_entries(&contract["blocker_codes"])
}

fn shared_operator_has_canonical_next_actions(
    top: &Value,
    shared: &Value,
    contract: &Value,
) -> bool {
    is_canonical_release1_next_action_entries(&top["next_actions"])
        || is_canonical_release1_next_action_entries(&shared["next_actions"])
        || is_canonical_release1_next_action_entries(&contract["next_actions"])
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_release1_blocker_code_entries, canonical_release1_next_action_entries,
        canonical_release1_operator_contract_status, is_canonical_release1_blocker_code_entries,
        is_canonical_release1_next_action_entries, is_canonical_release1_operator_contract_status,
        release1_operator_contracts_consistency_error,
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
        assert!(!is_canonical_release1_operator_contract_status(&value));
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
        assert!(!is_canonical_release1_blocker_code_entries(&value));
    }

    #[test]
    fn canonical_blocker_codes_must_be_registry_backed() {
        let value = json!(["valid_code"]);
        assert!(canonical_release1_blocker_code_entries(&value).is_none());
        assert!(!is_canonical_release1_blocker_code_entries(&value));
    }

    #[test]
    fn canonical_next_actions_downcases_and_trims() {
        let value = json!([" Run `task` "]);
        assert_eq!(
            canonical_release1_next_action_entries(&value),
            Some(vec!["run `task`".into()])
        );
        assert!(is_canonical_release1_next_action_entries(&value));
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
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
        );
    }
}
