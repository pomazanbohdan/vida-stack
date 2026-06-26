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

pub(crate) fn authorized_dispatch_rework_route_from_receipt_fields(
    downstream_dispatch_result_path: Option<&str>,
    dispatch_result_path: Option<&str>,
    dispatch_packet_path: Option<&str>,
    completed_dispatch_target: &str,
) -> Option<DispatchReworkRoute> {
    let packet = dispatch_packet_path.and_then(read_dispatch_packet_json)?;
    let execution_plan = packet_role_selection_execution_plan(&packet)?;
    let completed_target = completed_result_target(&packet, completed_dispatch_target);
    for result_path in dispatch_result_path_candidates_from_receipt_fields(
        downstream_dispatch_result_path,
        dispatch_result_path,
        dispatch_packet_path,
    ) {
        if let Some(route) = dispatch_rework_route_from_result_path(&result_path) {
            if rework_route_is_authorized(&execution_plan, &completed_target, &route) {
                return Some(route);
            }
        }
    }
    None
}

const MAX_DISPATCH_EVIDENCE_JSON_BYTES: u64 = 1024 * 1024;

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
            if let Some(packet) = read_bounded_dispatch_evidence_json(packet_path) {
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

    paths
}

fn read_dispatch_packet_json(path: &str) -> Option<serde_json::Value> {
    read_bounded_dispatch_evidence_json(path)
}

fn read_bounded_dispatch_evidence_json(path: &str) -> Option<serde_json::Value> {
    use std::io::Read;

    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_DISPATCH_EVIDENCE_JSON_BYTES {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;

    let mut raw = String::new();
    file.take(MAX_DISPATCH_EVIDENCE_JSON_BYTES + 1)
        .read_to_string(&mut raw)
        .ok()?;
    if raw.len() as u64 > MAX_DISPATCH_EVIDENCE_JSON_BYTES {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

fn packet_role_selection_execution_plan(packet: &serde_json::Value) -> Option<serde_json::Value> {
    packet
        .get("role_selection_full")
        .or_else(|| packet.get("role_selection"))
        .and_then(|role_selection| role_selection.get("execution_plan"))
        .cloned()
}

fn completed_result_target(packet: &serde_json::Value, fallback: &str) -> String {
    [
        packet.get("downstream_dispatch_target"),
        packet.get("dispatch_target"),
        packet.get("source_dispatch_target"),
    ]
    .into_iter()
    .find_map(|value| {
        value
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.replace('-', "_"))
    })
    .unwrap_or_else(|| fallback.trim().replace('-', "_"))
}

pub(crate) fn rework_route_is_authorized(
    execution_plan: &serde_json::Value,
    completed_dispatch_target: &str,
    route: &DispatchReworkRoute,
) -> bool {
    let Some(rework_resolution) = crate::runtime_dispatch_state::resolve_runtime_dispatch_target(
        execution_plan,
        &route.rework_target,
    ) else {
        return false;
    };
    let Some(completed_resolution) = crate::runtime_dispatch_state::resolve_runtime_dispatch_target(
        execution_plan,
        completed_dispatch_target,
    ) else {
        return false;
    };
    let allowed = route.allowed_next_node.trim();
    if allowed != rework_resolution.dispatch_target
        && allowed != format!("{}_rework", rework_resolution.dispatch_target)
    {
        return false;
    }
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let sequence = crate::dispatch_contract_execution_lane_sequence(dispatch_contract);
    let rework_index = sequence
        .iter()
        .position(|target| target == &rework_resolution.dispatch_target);
    let completed_index = sequence
        .iter()
        .position(|target| target == &completed_resolution.dispatch_target);
    matches!((rework_index, completed_index), (Some(rework_index), Some(completed_index)) if rework_index < completed_index)
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
    let result = read_bounded_dispatch_evidence_json(result_path)?;
    dispatch_rework_route_from_result(&result)
}

pub(crate) fn dispatch_rework_route_from_result(
    result: &serde_json::Value,
) -> Option<DispatchReworkRoute> {
    let rework_verdict = dispatch_result_has_rework_verdict(result);
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

fn dispatch_result_has_rework_verdict(result: &serde_json::Value) -> bool {
    dispatch_result_field_is_rework_verdict(result.get("decision"))
        || dispatch_result_field_is_rework_verdict(result.get("verdict"))
        || dispatch_result_field_is_rework_verdict(result.get("completion_verdict"))
        || result.get("execution_evidence").is_some_and(|evidence| {
            dispatch_result_field_is_rework_verdict(evidence.get("decision"))
                || dispatch_result_field_is_rework_verdict(evidence.get("verdict"))
                || dispatch_result_field_is_rework_verdict(evidence.get("completion_verdict"))
        })
}

fn dispatch_result_field_is_rework_verdict(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .map(|text| text.trim().to_ascii_lowercase())
        .is_some_and(|text| matches!(text.as_str(), "rework_required" | "blocked" | "blocker"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_rework_route_accepts_legacy_top_level_completion_verdict() {
        let result = serde_json::json!({
            "status": "blocked",
            "completion_verdict": "blocked",
            "rework_target": "developer",
            "allowed_next_node": "developer-rework",
            "blocker_codes": ["verification_rework_required"]
        });

        let route = dispatch_rework_route_from_result(&result)
            .expect("legacy completion_verdict should produce a rework route");
        assert_eq!(route.rework_target, "developer");
        assert_eq!(route.allowed_next_node, "developer_rework");
        assert_eq!(
            route.blocker_code.as_deref(),
            Some("verification_rework_required")
        );
    }

    #[test]
    fn dispatch_rework_route_accepts_nested_execution_completion_verdict() {
        let result = serde_json::json!({
            "status": "blocked",
            "execution_evidence": {
                "receipt_backed": true,
                "completion_verdict": "rework_required"
            },
            "rework_target": "tester",
            "allowed_next_node": "tester",
            "blocker_code": "review_rework_required"
        });

        let route = dispatch_rework_route_from_result(&result)
            .expect("nested completion_verdict should produce a rework route");
        assert_eq!(route.rework_target, "tester");
        assert_eq!(route.allowed_next_node, "tester");
        assert_eq!(
            route.blocker_code.as_deref(),
            Some("review_rework_required")
        );
    }

    #[test]
    fn dispatch_rework_route_rejects_pass_completion_verdict() {
        let result = serde_json::json!({
            "status": "pass",
            "completion_verdict": "pass",
            "rework_target": "developer",
            "allowed_next_node": "developer_rework"
        });

        assert!(dispatch_rework_route_from_result(&result).is_none());
    }

    fn execution_plan() -> serde_json::Value {
        serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "developer": {"dispatch_target": "developer", "task_class": "implementation"},
                        "tester": {"dispatch_target": "tester", "task_class": "verification"},
                        "release_admin": {"dispatch_target": "release_admin", "task_class": "release"}
                    },
                    "execution_lane_sequence": ["developer", "tester"]
                }
            }
        })
    }

    #[test]
    fn dispatch_rework_route_from_result_path_reads_regular_bounded_json() {
        let root = unique_test_dir("dispatch-result-regular");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let result_path = root.join("result.json");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "status": "blocked",
                "completion_verdict": "rework_required",
                "rework_target": "developer",
                "allowed_next_node": "developer-rework",
                "blocker_code": "verification_rework_required"
            })
            .to_string(),
        )
        .expect("result should write");

        let route = dispatch_rework_route_from_result_path(&result_path.display().to_string())
            .expect("bounded regular result json should parse");
        assert_eq!(route.rework_target, "developer");
        assert_eq!(route.allowed_next_node, "developer_rework");
        assert_eq!(
            route.blocker_code.as_deref(),
            Some("verification_rework_required")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_evidence_reader_rejects_oversized_json_file() {
        let root = unique_test_dir("dispatch-result-oversized");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let result_path = root.join("oversized.json");
        let oversized = format!(
            "{{{} }}",
            " ".repeat(MAX_DISPATCH_EVIDENCE_JSON_BYTES as usize)
        );
        std::fs::write(&result_path, oversized).expect("oversized result should write");

        assert!(
            dispatch_rework_route_from_result_path(&result_path.display().to_string()).is_none()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dispatch_evidence_reader_rejects_fifo_without_blocking() {
        let root = unique_test_dir("dispatch-result-fifo");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let fifo_path = root.join("result.fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status()
            .expect("mkfifo should run");
        assert!(status.success(), "mkfifo should create fifo");

        assert!(dispatch_rework_route_from_result_path(&fifo_path.display().to_string()).is_none());
        assert!(read_dispatch_packet_json(&fifo_path.display().to_string()).is_none());
        let candidates = dispatch_result_path_candidates_from_receipt_fields(
            None,
            None,
            Some(&fifo_path.display().to_string()),
        );
        assert!(candidates.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vida-{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn rework_route_authorization_accepts_configured_backward_rework_edge() {
        let route = DispatchReworkRoute {
            rework_target: "developer".to_string(),
            allowed_next_node: "developer_rework".to_string(),
            blocker_code: Some("verification_rework_required".to_string()),
        };

        assert!(rework_route_is_authorized(
            &execution_plan(),
            "tester",
            &route
        ));
    }

    #[test]
    fn rework_route_authorization_rejects_artifact_controlled_unknown_or_unsequenced_lane() {
        let unknown = DispatchReworkRoute {
            rework_target: "developer".to_string(),
            allowed_next_node: "release_admin".to_string(),
            blocker_code: Some("verification_rework_required".to_string()),
        };
        let unsequenced_target = DispatchReworkRoute {
            rework_target: "release_admin".to_string(),
            allowed_next_node: "release_admin".to_string(),
            blocker_code: Some("verification_rework_required".to_string()),
        };

        assert!(!rework_route_is_authorized(
            &execution_plan(),
            "tester",
            &unknown
        ));
        assert!(!rework_route_is_authorized(
            &execution_plan(),
            "tester",
            &unsequenced_target
        ));
    }
}
