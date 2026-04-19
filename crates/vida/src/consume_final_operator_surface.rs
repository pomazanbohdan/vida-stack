use crate::contract_profile_adapter::{
    blocker_code, canonical_blocker_code_list, operator_contract_status_is_blocked,
    render_operator_contract_envelope, BlockerCode,
};

pub(crate) fn emit_taskflow_consume_final_json(
    store: &crate::StateStore,
    payload: &crate::TaskflowDirectConsumptionPayload,
) -> Result<(), String> {
    let mut payload_json = serde_json::to_value(payload)
        .map_err(|error| format!("Failed to encode consume-final payload as json: {error}"))?;
    let runtime_dispatch_receipt_blocker_code =
        crate::runtime_consumption_final_dispatch_receipt_blocker_code(store, &payload_json)?;
    let mut consume_final_blocker_codes = consume_final_operator_blocker_codes(&payload_json);
    let mut consume_final_next_actions = consume_final_operator_next_actions(&payload_json);
    if let Some(blocker_code) = runtime_dispatch_receipt_blocker_code.as_deref() {
        crate::apply_runtime_consumption_final_dispatch_receipt_blocker(
            &mut payload_json,
            blocker_code,
        );
        if !consume_final_blocker_codes
            .iter()
            .any(|code| code == blocker_code)
        {
            consume_final_blocker_codes.push(blocker_code.to_string());
        }
        consume_final_next_actions.push(
            match blocker_code {
                crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER => {
                    crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION
                }
                _ => crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
            }
            .to_string(),
        );
    }
    consume_final_blocker_codes =
        canonical_blocker_code_list(consume_final_blocker_codes.iter().map(String::as_str));
    let consume_final_status = if consume_final_blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let failure_control_evidence = payload_json["dispatch_receipt"]["run_id"]
        .as_str()
        .zip(
            payload_json["dispatch_receipt"]["dispatch_packet_path"]
                .as_str()
                .filter(|value| !value.is_empty()),
        )
        .map(|(run_id, dispatch_packet_path)| {
            crate::taskflow_consume_resume::build_failure_control_evidence(
                run_id,
                dispatch_packet_path,
            )
        })
        .unwrap_or(serde_json::Value::Null);
    if !failure_control_evidence.is_null() {
        payload_json["failure_control_evidence"] = failure_control_evidence.clone();
    }
    let snapshot = serde_json::json!({
        "surface": "vida taskflow consume final",
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    let snapshot_path =
        crate::write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let mut operator_contracts = build_operator_contracts_envelope(
        consume_final_status,
        consume_final_blocker_codes.clone(),
        consume_final_next_actions.clone(),
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": payload_json["dispatch_receipt"]["run_id"].as_str(),
            "latest_task_reconciliation_receipt_id": payload_json["task_reconciliation_receipt"]["receipt_id"].as_str(),
            "retrieval_trust_signal": serde_json::json!({}),
            "consume_final_surface": "vida taskflow consume final",
        }),
    );
    let mut shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": operator_contracts["status"].clone(),
        "blocker_codes": operator_contracts["blocker_codes"].clone(),
        "next_actions": operator_contracts["next_actions"].clone(),
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
    });
    let mut snapshot_with_operator_contracts = serde_json::json!({
        "surface": "vida taskflow consume final",
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": consume_final_status,
        "blocker_codes": consume_final_blocker_codes,
        "next_actions": consume_final_next_actions,
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    let runtime_consumption = crate::runtime_consumption_summary(store.root())?;
    let latest_final_snapshot_path =
        crate::latest_final_runtime_consumption_snapshot_path(store.root())?;
    let protocol_binding_latest_receipt_id =
        crate::block_on_state_store(store.latest_protocol_binding_receipt())?
            .map(|receipt| receipt.receipt_id);
    let retrieval_trust_signal = crate::latest_admissible_retrieval_trust_signal(
        &runtime_consumption,
        latest_final_snapshot_path.as_deref(),
        protocol_binding_latest_receipt_id.as_deref(),
    )
    .unwrap_or_else(|| serde_json::json!({}));
    if let Some(runtime_bundle) = payload_json
        .get_mut("runtime_bundle")
        .and_then(serde_json::Value::as_object_mut)
    {
        runtime_bundle.insert(
            "retrieval_trust_evidence".to_string(),
            retrieval_trust_signal.clone(),
        );
        if let Some(cache_delivery_contract) = runtime_bundle
            .get_mut("cache_delivery_contract")
            .and_then(serde_json::Value::as_object_mut)
        {
            cache_delivery_contract.insert(
                "retrieval_trust_evidence".to_string(),
                retrieval_trust_signal.clone(),
            );
        }
    }
    operator_contracts["artifact_refs"]["retrieval_trust_signal"] = retrieval_trust_signal.clone();
    operator_contracts["artifact_refs"]["protocol_binding_latest_receipt_id"] =
        protocol_binding_latest_receipt_id
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
    shared_fields["artifact_refs"] = operator_contracts["artifact_refs"].clone();
    snapshot_with_operator_contracts["artifact_refs"] = operator_contracts["artifact_refs"].clone();
    snapshot_with_operator_contracts["shared_fields"] = shared_fields.clone();
    snapshot_with_operator_contracts["operator_contracts"] = operator_contracts.clone();
    snapshot_with_operator_contracts["payload"] = payload_json.clone();
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "surface": "vida taskflow consume final",
            "trace_id": operator_contracts["trace_id"].clone(),
            "workflow_class": operator_contracts["workflow_class"].clone(),
            "risk_tier": operator_contracts["risk_tier"].clone(),
            "status": consume_final_status,
            "blocker_codes": consume_final_blocker_codes,
            "next_actions": consume_final_next_actions,
            "artifact_refs": operator_contracts["artifact_refs"].clone(),
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts,
            "failure_control_evidence": failure_control_evidence,
            "payload": payload_json,
            "snapshot_path": snapshot_path,
        }))
        .expect("consume final should render as json")
    );
    Ok(())
}

pub(crate) fn build_operator_contracts_envelope(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
) -> serde_json::Value {
    render_operator_contract_envelope(status, blocker_codes, next_actions, artifact_refs)
}

fn consume_final_operator_blocker_codes(payload: &serde_json::Value) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if payload["bundle_check"]["activation_status"].as_str() != Some("ready_enough_for_normal_work")
    {
        if let Some(code) = blocker_code(BlockerCode::BundleActivationNotReady) {
            blocker_codes.push(code);
        }
    }
    if operator_contract_status_is_blocked(&payload["docflow_verdict"]["status"]) {
        if let Some(code) = blocker_code(BlockerCode::DocflowVerdictBlock) {
            blocker_codes.push(code);
        }
    }
    if operator_contract_status_is_blocked(&payload["closure_admission"]["status"]) {
        if let Some(code) = blocker_code(BlockerCode::ClosureAdmissionBlock) {
            blocker_codes.push(code);
        }
    }
    blocker_codes
}

fn consume_final_operator_next_actions(payload: &serde_json::Value) -> Vec<String> {
    let mut next_actions = Vec::new();
    if payload["bundle_check"]["activation_status"].as_str() != Some("ready_enough_for_normal_work")
    {
        next_actions.push("Resolve activation blockers before consume-final handoff.".to_string());
    }
    if operator_contract_status_is_blocked(&payload["docflow_verdict"]["status"]) {
        next_actions.push(
            "Run `vida docflow proofcheck --profile active-canon` and clear blockers.".to_string(),
        );
    }
    if operator_contract_status_is_blocked(&payload["closure_admission"]["status"]) {
        next_actions.push(
            "Run `vida taskflow consume bundle check --json` and resolve closure blockers."
                .to_string(),
        );
    }
    next_actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_contracts_envelope_normalizes_status_to_canonical_vocabulary() {
        let envelope = build_operator_contracts_envelope(
            " pass ",
            Vec::new(),
            Vec::new(),
            serde_json::json!({}),
        );

        assert_eq!(envelope["status"], "pass");
    }

    #[test]
    fn operator_contracts_envelope_accepts_ok_compat_status() {
        let envelope =
            build_operator_contracts_envelope("ok", Vec::new(), Vec::new(), serde_json::json!({}));

        assert_eq!(envelope["status"], "pass");
    }
}
