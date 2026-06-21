use crate::contract_profile_adapter::{
    blocker_code, canonical_blocker_code_list, operator_contract_status_is_blocked,
    render_operator_contract_envelope, BlockerCode,
};
use crate::release1_operator_output::{
    build_release1_operator_output_payload, render_vida_gate_result_from_operator_contracts,
    replace_release1_operator_output_artifact_refs,
};

pub(crate) fn emit_taskflow_consume_final_json(
    store: &crate::StateStore,
    payload: &crate::TaskflowDirectConsumptionPayload,
) -> Result<String, String> {
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
    let mut snapshot_with_operator_contracts = build_release1_operator_output_payload(
        "vida taskflow consume final",
        consume_final_blocker_codes,
        consume_final_next_actions,
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": payload_json["dispatch_receipt"]["run_id"].as_str(),
            "latest_task_reconciliation_receipt_id": payload_json["task_reconciliation_receipt"]["receipt_id"].as_str(),
            "retrieval_trust_signal": serde_json::json!({}),
            "consume_final_surface": "vida taskflow consume final",
        }),
        serde_json::json!({
            "failure_control_evidence": failure_control_evidence.clone(),
            "payload": payload_json.clone(),
        }),
    )?;
    let consume_final_next_actions = snapshot_with_operator_contracts["next_actions"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut operator_contracts = snapshot_with_operator_contracts["operator_contracts"].clone();
    let mut vida_gate_result = consume_final_vida_gate_result(
        &operator_contracts,
        &consume_final_next_actions,
        &failure_control_evidence,
    );
    let docflow_vida_gate_result =
        docflow_verdict_vida_gate_result(&payload_json["docflow_verdict"]);
    snapshot_with_operator_contracts["vida_gate_result"] = vida_gate_result.clone();
    snapshot_with_operator_contracts["docflow_vida_gate_result"] = docflow_vida_gate_result.clone();
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
    let mut artifact_refs = snapshot_with_operator_contracts["artifact_refs"].clone();
    artifact_refs["retrieval_trust_signal"] = retrieval_trust_signal.clone();
    artifact_refs["protocol_binding_latest_receipt_id"] = protocol_binding_latest_receipt_id
        .clone()
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);
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
    for field in ["status", "blocker_codes", "next_actions"] {
        let canonical = snapshot_with_operator_contracts["operator_contracts"][field].clone();
        snapshot_with_operator_contracts[field] = canonical.clone();
        snapshot_with_operator_contracts["shared_fields"][field] = canonical;
    }
    replace_release1_operator_output_artifact_refs(
        &mut snapshot_with_operator_contracts,
        artifact_refs,
    )?;
    operator_contracts = snapshot_with_operator_contracts["operator_contracts"].clone();
    snapshot_with_operator_contracts["payload"] = payload_json.clone();
    vida_gate_result = consume_final_vida_gate_result(
        &operator_contracts,
        &consume_final_next_actions,
        &failure_control_evidence,
    );
    let docflow_vida_gate_result =
        docflow_verdict_vida_gate_result(&payload_json["docflow_verdict"]);
    snapshot_with_operator_contracts["vida_gate_result"] = vida_gate_result.clone();
    snapshot_with_operator_contracts["docflow_vida_gate_result"] = docflow_vida_gate_result.clone();
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
            "trace_id": snapshot_with_operator_contracts["trace_id"].clone(),
            "workflow_class": snapshot_with_operator_contracts["workflow_class"].clone(),
            "risk_tier": snapshot_with_operator_contracts["risk_tier"].clone(),
            "status": snapshot_with_operator_contracts["status"].clone(),
            "blocker_codes": snapshot_with_operator_contracts["blocker_codes"].clone(),
            "next_actions": consume_final_next_actions,
            "artifact_refs": operator_contracts["artifact_refs"].clone(),
            "shared_fields": snapshot_with_operator_contracts["shared_fields"].clone(),
            "operator_contracts": operator_contracts,
            "vida_gate_result": vida_gate_result,
            "docflow_vida_gate_result": docflow_vida_gate_result,
            "failure_control_evidence": failure_control_evidence,
            "payload": payload_json,
            "snapshot_path": snapshot_path,
        }))
        .expect("consume final should render as json")
    );
    Ok(snapshot_path)
}

pub(crate) fn build_operator_contracts_envelope(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
) -> serde_json::Value {
    render_operator_contract_envelope(status, blocker_codes, next_actions, artifact_refs)
}

fn consume_final_vida_gate_result(
    operator_contracts: &serde_json::Value,
    next_actions: &[String],
    failure_control_evidence: &serde_json::Value,
) -> serde_json::Value {
    let failure_codes = consume_final_failure_codes(failure_control_evidence);
    let issues = failure_codes
        .iter()
        .map(|code| {
            serde_json::json!({
                "code": code,
                "severity": "failure",
                "source": "failure_control_evidence",
            })
        })
        .collect::<Vec<_>>();
    render_vida_gate_result_from_operator_contracts(
        "taskflow.consume_final",
        operator_contracts.clone(),
        Vec::new(),
        failure_codes,
        issues,
        next_actions.to_vec(),
        operator_contracts["artifact_refs"].clone(),
    )
}

fn consume_final_failure_codes(failure_control_evidence: &serde_json::Value) -> Vec<String> {
    if failure_control_evidence.is_null() {
        Vec::new()
    } else {
        vec!["failure_control_evidence_present".to_string()]
    }
}

fn docflow_verdict_vida_gate_result(docflow_verdict: &serde_json::Value) -> serde_json::Value {
    let blockers = docflow_verdict["blockers"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let status_is_blocked = operator_contract_status_is_blocked(&docflow_verdict["status"]);
    let next_actions = crate::docflow_runtime_verdict_next_actions(
        docflow_verdict["status"].as_str().unwrap_or_default(),
    );
    crate::release1_operator_output::render_vida_gate_result_with_status(
        "docflow.runtime_verdict",
        if status_is_blocked { "blocked" } else { "pass" },
        blockers
            .iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>(),
        Vec::new(),
        Vec::new(),
        blockers
            .iter()
            .map(|code| {
                serde_json::json!({
                    "code": code,
                    "severity": "blocker",
                    "source": "docflow_verdict",
                })
            })
            .collect(),
        next_actions,
        serde_json::json!({
            "proof_surfaces": docflow_verdict["proof_surfaces"].clone(),
        }),
    )
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
    if payload["dispatch_packet_preview"]["status"].as_str() == Some("blocked") {
        blocker_codes.push("dispatch_packet_contract_invalid".to_string());
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
        next_actions.extend(crate::docflow_runtime_verdict_next_actions(
            payload["docflow_verdict"]["status"]
                .as_str()
                .unwrap_or_default(),
        ));
    }
    if operator_contract_status_is_blocked(&payload["closure_admission"]["status"]) {
        next_actions.push(format!(
            "Run `{}` and resolve closure blockers.",
            operator_output::command_text::human_command("vida taskflow consume bundle check")
        ));
    }
    if payload["dispatch_packet_preview"]["status"].as_str() == Some("blocked") {
        let missing_fields = payload["dispatch_packet_preview"]["packet_contract_missing_fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        if missing_fields.is_empty() {
            next_actions.push(
                "Review the dispatch packet preview and resolve contract validation blockers before dispatch."
                    .to_string(),
            );
        } else {
            next_actions.push(format!(
                "Resolve dispatch packet preview contract gaps before dispatch: {}.",
                missing_fields.join(", ")
            ));
        }
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

    #[test]
    fn consume_final_operator_surface_adds_preview_contract_blocker_and_action() {
        let payload = serde_json::json!({
            "bundle_check": {
                "activation_status": "ready_enough_for_normal_work"
            },
            "docflow_verdict": {
                "status": "pass"
            },
            "closure_admission": {
                "status": "pass"
            },
            "dispatch_packet_preview": {
                "status": "blocked",
                "packet_contract_missing_fields": ["owned_paths", "proof_target"]
            }
        });

        let blocker_codes = consume_final_operator_blocker_codes(&payload);
        let next_actions = consume_final_operator_next_actions(&payload);

        assert!(blocker_codes
            .iter()
            .any(|code| code == "dispatch_packet_contract_invalid"));
        assert!(next_actions
            .iter()
            .any(|action| action.contains("owned_paths")));
        assert!(next_actions
            .iter()
            .any(|action| action.contains("proof_target")));
    }

    #[test]
    fn consume_final_operator_surface_uses_default_command_in_human_next_action() {
        let payload = serde_json::json!({
            "bundle_check": {
                "activation_status": "ready_enough_for_normal_work"
            },
            "docflow_verdict": {
                "status": "pass"
            },
            "closure_admission": {
                "status": "blocked"
            },
            "dispatch_packet_preview": {
                "status": "pass"
            }
        });

        let next_actions = consume_final_operator_next_actions(&payload);

        assert!(next_actions.iter().any(|action| {
            action.contains("vida taskflow consume bundle check")
                && !action.contains("bundle check --json")
        }));
    }

    #[test]
    fn consume_final_gate_result_is_sibling_and_preserves_operator_projection() {
        let operator_contracts = build_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({"snapshot": "present"}),
        );

        let gate_result =
            consume_final_vida_gate_result(&operator_contracts, &[], &serde_json::Value::Null);

        assert_eq!(gate_result["gate_id"], "taskflow.consume_final");
        assert_eq!(gate_result["status"], "pass");
        assert_eq!(gate_result["ready"], true);
        assert_eq!(gate_result["operator_contracts"], operator_contracts);
    }

    #[test]
    fn consume_final_gate_result_reports_failure_with_blocked_legacy_projection() {
        let operator_contracts = build_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "run_id": "run-1",
                "task_id": "task-1",
                "dispatch_packet_path": "packet.json",
                "evidence_refs": ["failure-control"],
                "affected_paths": ["crates/vida/src/consume_final_operator_surface.rs"]
            }),
        );
        let failure_control_evidence = serde_json::json!({
            "failure_control": "present"
        });

        let gate_result = consume_final_vida_gate_result(
            &operator_contracts,
            &["Inspect failure control evidence.".to_string()],
            &failure_control_evidence,
        );

        assert_eq!(gate_result["status"], "fail");
        assert_eq!(gate_result["blocking"], true);
        assert_eq!(gate_result["ready"], false);
        assert_eq!(
            gate_result["failure_codes"],
            serde_json::json!(["failure_control_evidence_present"])
        );
        assert_eq!(gate_result["operator_contracts"]["status"], "blocked");
        assert_eq!(
            gate_result["operator_contracts"]["blocker_codes"],
            serde_json::json!(["failure_control_evidence_present"])
        );
        assert_eq!(gate_result["run_id"], "run-1");
        assert_eq!(gate_result["task_id"], "task-1");
        assert_eq!(gate_result["packet_id"], "packet.json");
        assert_eq!(
            gate_result["evidence_refs"],
            serde_json::json!(["failure-control"])
        );
        assert_eq!(
            gate_result["affected_paths"],
            serde_json::json!(["crates/vida/src/consume_final_operator_surface.rs"])
        );
    }

    #[test]
    fn docflow_verdict_gate_result_adapts_block_verdict_without_changing_verdict_shape() {
        let docflow_verdict = serde_json::json!({
            "status": "block",
            "ready": false,
            "blockers": ["missing_proof_verdict"],
            "proof_surfaces": ["registry", "check", "readiness", "proof"]
        });

        let gate_result = docflow_verdict_vida_gate_result(&docflow_verdict);

        assert_eq!(gate_result["gate_id"], "docflow.runtime_verdict");
        assert_eq!(gate_result["status"], "blocked");
        assert_eq!(gate_result["ready"], false);
        assert_eq!(
            gate_result["blocker_codes"],
            serde_json::json!(["missing_proof_verdict"])
        );
        assert_eq!(
            gate_result["operator_contracts"]["blocker_codes"],
            serde_json::json!(["missing_proof_verdict"])
        );
    }
}
