pub(crate) fn normalized_dispatch_result_activation_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
    result_artifact_path: &str,
) -> serde_json::Value {
    let activation_kind = body["activation_semantics"]["activation_kind"]
        .as_str()
        .or_else(|| {
            if body["execution_evidence"]["status"].as_str() == Some("recorded")
                || body["execution_state"].as_str() == Some("executed")
            {
                Some("execution_evidence")
            } else if body["artifact_kind"].as_str() == Some("runtime_dispatch_result")
                || body["execution_state"].as_str() == Some("blocked")
                || body["execution_state"].as_str() == Some("executing")
            {
                Some("activation_view")
            } else {
                None
            }
        })
        .unwrap_or("activation_view");
    let activation_semantics = serde_json::json!({
        "activation_kind": activation_kind,
        "view_only": activation_kind != "execution_evidence",
        "executes_packet": activation_kind == "execution_evidence",
        "records_completion_receipt": activation_kind == "execution_evidence",
    });
    let execution_evidence = if activation_kind == "execution_evidence" {
        let mut evidence = match body.get("execution_evidence").cloned() {
            Some(serde_json::Value::Object(object)) => object,
            _ => serde_json::Map::new(),
        };
        evidence
            .entry("status".to_string())
            .or_insert_with(|| serde_json::json!("recorded"));
        evidence
            .entry("receipt_backed".to_string())
            .or_insert_with(|| serde_json::json!(true));
        evidence
            .entry("evidence_kind".to_string())
            .or_insert_with(|| serde_json::json!("lane_execution_receipt_artifact"));
        evidence
            .entry("result_path".to_string())
            .or_insert_with(|| serde_json::json!(result_artifact_path));
        evidence.entry("backend_id".to_string()).or_insert_with(|| {
            serde_json::json!(canonical_lane_receipt_carrier_id_for_result(receipt, body))
        });
        serde_json::Value::Object(evidence)
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "activation_semantics": activation_semantics,
        "execution_evidence": execution_evidence,
        "receipt_backed": activation_kind == "execution_evidence",
    })
}

fn canonical_lane_receipt_carrier_id(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> String {
    receipt
        .selected_backend
        .clone()
        .or_else(|| receipt.activation_agent_type.clone())
        .or_else(|| receipt.dispatch_surface.clone())
        .unwrap_or_else(|| "taskflow_state_store".to_string())
}

fn canonical_lane_receipt_carrier_id_for_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> String {
    for candidate in [
        body.get("execution_evidence")
            .and_then(|value| value.get("backend_id")),
        body.get("backend_dispatch")
            .and_then(|value| value.get("carrier_id")),
        body.get("backend_dispatch")
            .and_then(|value| value.get("backend_id")),
        body.get("execution_truth")
            .and_then(|value| value.get("effective_selected_backend")),
        body.get("effective_execution_posture")
            .and_then(|value| value.get("selected_backend")),
    ] {
        if let Some(value) = crate::json_string(candidate)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && value != "unknown")
        {
            return value;
        }
    }
    canonical_lane_receipt_carrier_id(receipt)
}

pub(crate) fn is_terminal_dispatch_execution_state(body: &serde_json::Value) -> bool {
    matches!(
        crate::json_string(body.get("execution_state")).as_deref(),
        Some("executed" | "blocked")
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispatchReworkRoute {
    pub(crate) rework_target: String,
    pub(crate) allowed_next_node: String,
    pub(crate) blocker_code: Option<String>,
}

pub(crate) fn dispatch_rework_route_from_receipt_fields(
    downstream_dispatch_result_path: Option<&str>,
    dispatch_result_path: Option<&str>,
    dispatch_packet_path: Option<&str>,
) -> Option<DispatchReworkRoute> {
    for result_path in dispatch_result_path_candidates_from_receipt_fields(
        downstream_dispatch_result_path,
        dispatch_result_path,
        dispatch_packet_path,
    ) {
        if let Some(route) = dispatch_rework_route_from_result_path(&result_path) {
            return Some(route);
        }
    }
    None
}

pub(crate) fn dispatch_result_path_candidates_from_receipt_fields(
    downstream_dispatch_result_path: Option<&str>,
    dispatch_result_path: Option<&str>,
    dispatch_packet_path: Option<&str>,
) -> Vec<String> {
    let mut paths = Vec::new();
    push_non_empty_path(&mut paths, downstream_dispatch_result_path);
    push_non_empty_path(&mut paths, dispatch_result_path);

    if let Some(packet_path) = dispatch_packet_path {
        let packet_path = packet_path.trim();
        if !packet_path.is_empty() {
            let packet_path =
                crate::runtime_dispatch_state::normalize_persisted_runtime_path(packet_path);
            if let Ok(raw) = std::fs::read_to_string(packet_path) {
                if let Ok(packet) = serde_json::from_str::<serde_json::Value>(&raw) {
                    push_json_string_path(
                        &mut paths,
                        &packet,
                        &[
                            "downstream_dispatch_result_path",
                            "dispatch_result_path",
                            "result_path",
                        ],
                    );
                    if let Some(host_bridge_request) = packet.get("host_tool_bridge_request") {
                        push_json_string_path(
                            &mut paths,
                            host_bridge_request,
                            &["result_path", "dispatch_result_path"],
                        );
                    }
                }
            }
        }
    }

    paths
}

fn push_json_string_path(paths: &mut Vec<String>, value: &serde_json::Value, field_names: &[&str]) {
    for field_name in field_names {
        push_non_empty_path(
            paths,
            value
                .get(field_name)
                .and_then(serde_json::Value::as_str)
                .map(str::trim),
        );
    }
}

fn push_non_empty_path(paths: &mut Vec<String>, path: Option<&str>) {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !paths.iter().any(|existing| existing == path) {
        paths.push(path.to_string());
    }
}

pub(crate) fn dispatch_rework_route_from_result_path(
    result_path: &str,
) -> Option<DispatchReworkRoute> {
    let result_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(result_path);
    let raw = std::fs::read_to_string(result_path).ok()?;
    let result: serde_json::Value = serde_json::from_str(&raw).ok()?;
    dispatch_rework_route_from_result(&result)
}

pub(crate) fn dispatch_rework_route_from_result(
    result: &serde_json::Value,
) -> Option<DispatchReworkRoute> {
    let rework_verdict = matches!(
        result
            .get("decision")
            .and_then(serde_json::Value::as_str)
            .map(str::trim),
        Some("rework_required") | Some("blocked")
    ) || matches!(
        result
            .get("verdict")
            .and_then(serde_json::Value::as_str)
            .map(str::trim),
        Some("rework_required") | Some("blocked")
    );
    if !rework_verdict {
        return None;
    }
    let rework_target = result
        .get("rework_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let allowed_next_node = result
        .get("allowed_next_node")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(DispatchReworkRoute {
        rework_target: rework_target.replace('-', "_"),
        allowed_next_node: allowed_next_node.replace('-', "_"),
        blocker_code: result_blocker_code(result),
    })
}

fn result_blocker_code(result: &serde_json::Value) -> Option<String> {
    result
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            result
                .get("blocker_codes")
                .and_then(serde_json::Value::as_array)
                .and_then(|codes| codes.iter().find_map(serde_json::Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

pub(crate) fn canonical_lane_execution_receipt_artifact_json(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
    finished_at: &str,
    result_artifact_path: &str,
) -> serde_json::Value {
    let packet_id = receipt
        .dispatch_packet_path
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_stem())
        .and_then(|stem| stem.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}-no-packet", receipt.run_id, receipt.dispatch_target));
    let lane_role = receipt
        .activation_runtime_role
        .clone()
        .unwrap_or_else(|| receipt.dispatch_target.clone());
    let carrier_id = canonical_lane_receipt_carrier_id_for_result(receipt, body);
    let status = match crate::json_string(body.get("status")).as_deref() {
        Some("pass") => "pass".to_string(),
        Some("blocked") => "blocked".to_string(),
        _ if receipt.dispatch_status == "blocked" => "blocked".to_string(),
        _ => "pass".to_string(),
    };
    let lane_status = match crate::json_string(body.get("execution_state")).as_deref() {
        Some("executed") => crate::release1_contracts::LaneStatus::LaneCompleted
            .as_str()
            .to_string(),
        Some("blocked") => crate::release1_contracts::LaneStatus::LaneBlocked
            .as_str()
            .to_string(),
        Some("executing") => crate::release1_contracts::LaneStatus::LaneRunning
            .as_str()
            .to_string(),
        _ => receipt.lane_status.clone(),
    };
    serde_json::to_value(
        crate::release1_contracts::CanonicalLaneExecutionReceiptArtifact {
            lane_execution_receipt: crate::release1_contracts::CanonicalLaneExecutionReceipt {
                header: crate::release1_contracts::CanonicalArtifactHeader::new(
                    format!(
                        "lane-execution.{}.{}",
                        receipt.run_id, receipt.dispatch_target
                    ),
                    crate::release1_contracts::CanonicalArtifactType::LaneExecutionReceipt,
                    receipt.recorded_at.clone(),
                    finished_at.to_string(),
                    status,
                    "runtime_dispatch_state",
                    None,
                    Some(
                        crate::release1_contracts::WorkflowClass::DelegatedDevelopmentPacket
                            .as_str()
                            .to_string(),
                    ),
                ),
                run_id: receipt.run_id.clone(),
                packet_id,
                lane_id: format!("{}:{}", receipt.run_id, receipt.dispatch_target),
                lane_role,
                carrier_id,
                lane_status,
                evidence_status: "recorded".to_string(),
                started_at: receipt.recorded_at.clone(),
                finished_at: finished_at.to_string(),
                result_artifact_ids: vec![result_artifact_path.to_string()],
                supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
                exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
            },
        },
    )
    .expect("lane execution receipt artifact should serialize")
}
