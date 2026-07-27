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
        if let Some(backend_id) = canonical_lane_receipt_backend_id_for_result(receipt, body) {
            evidence.insert("backend_id".to_string(), serde_json::json!(backend_id));
        }
        if let Some(carrier_id) = canonical_lane_receipt_carrier_id_for_result(receipt, body) {
            evidence.insert("carrier_id".to_string(), serde_json::json!(carrier_id));
        }
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

fn canonical_lane_receipt_backend_id(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    receipt
        .selected_backend
        .clone()
        .filter(|value| !value.trim().is_empty() && value != "unknown")
}

fn canonical_lane_receipt_backend_id_for_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> Option<String> {
    for candidate in [
        body.get("backend_dispatch")
            .and_then(|value| value.get("backend_id")),
        body.get("execution_evidence")
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
            return Some(value);
        }
    }
    canonical_lane_receipt_backend_id(receipt)
}

fn canonical_lane_receipt_carrier_id_for_result(
    _receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> Option<String> {
    for candidate in [
        body.get("backend_dispatch")
            .and_then(|value| value.get("carrier_id")),
        body.get("execution_evidence")
            .and_then(|value| value.get("carrier_id")),
        body.get("selected_carrier_id"),
        body.get("backend_dispatch")
            .and_then(|value| value.get("selected_carrier_id")),
        body.get("runtime_assignment")
            .and_then(|value| value.get("selected_carrier_id")),
        body.pointer("/role_selection/execution_plan/runtime_assignment/selected_carrier_id"),
        body.pointer("/role_selection_full/execution_plan/runtime_assignment/selected_carrier_id"),
        body.get("carrier_runtime_assignment")
            .and_then(|value| value.get("carrier_id")),
    ] {
        if let Some(value) = crate::json_string(candidate)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && value != "unknown")
        {
            return Some(value);
        }
    }
    None
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
    pub(crate) receipt_backed: bool,
    pub(crate) outcome_blocker_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthorizedDispatchReworkContext {
    pub(crate) route: DispatchReworkRoute,
    pub(crate) role_selection: crate::RuntimeConsumptionLaneSelection,
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

#[cfg(test)]
pub(crate) fn authorized_dispatch_rework_route_from_receipt_fields(
    downstream_dispatch_result_path: Option<&str>,
    dispatch_result_path: Option<&str>,
    dispatch_packet_path: Option<&str>,
    completed_dispatch_target: &str,
) -> Option<DispatchReworkRoute> {
    let packet = dispatch_packet_path.and_then(read_dispatch_packet_json)?;
    let (authority, execution_plan) = packet_team_flow_authority(&packet).ok()?;
    let completed_target = completed_result_target(&packet, completed_dispatch_target);
    let packet_fallback_path = (downstream_dispatch_result_path.is_none()
        && dispatch_result_path.is_none())
    .then_some(dispatch_packet_path)
    .flatten();
    for result_path in dispatch_result_path_candidates_from_receipt_fields(
        downstream_dispatch_result_path,
        dispatch_result_path,
        packet_fallback_path,
    ) {
        if let Some(route) = dispatch_rework_route_from_result_path(&result_path) {
            if rework_route_is_authorized(&authority, &execution_plan, &completed_target, &route)
                .is_ok()
            {
                return Some(route);
            }
        }
    }
    None
}

pub(crate) async fn authorized_dispatch_rework_context_from_receipt_fields(
    store: &crate::state_store::StateStore,
    run_id: &str,
    task_id: &str,
    downstream_dispatch_result_path: Option<&str>,
    dispatch_result_path: Option<&str>,
    dispatch_packet_path: Option<&str>,
    completed_dispatch_target: &str,
) -> Result<
    Option<AuthorizedDispatchReworkContext>,
    crate::team_flow_authority_adapter::TeamFlowResolutionBlocker,
> {
    let run_id = run_id.trim();
    if run_id.is_empty() {
        return Err(rework_authority_blocker(
            "team_flow_rework_run_id_missing",
            "run_id",
            Vec::new(),
        ));
    }
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(rework_authority_blocker(
            "team_flow_rework_task_id_missing",
            "task_id",
            Vec::new(),
        ));
    }
    let packet_path = dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            rework_authority_blocker(
                "team_flow_rework_packet_missing",
                "dispatch_packet_path",
                Vec::new(),
            )
        })?;
    let packet = read_dispatch_packet_json(packet_path).ok_or_else(|| {
        rework_authority_blocker(
            "team_flow_rework_packet_unreadable",
            packet_path,
            Vec::new(),
        )
    })?;
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            rework_authority_blocker(
                "team_flow_rework_packet_run_id_missing",
                "run_id",
                Vec::new(),
            )
        })?;
    if packet_run_id != run_id {
        return Err(rework_authority_blocker(
            "team_flow_rework_packet_run_id_mismatch",
            packet_run_id,
            vec![run_id.to_string()],
        ));
    }
    let packet_fallback_path = (downstream_dispatch_result_path.is_none()
        && dispatch_result_path.is_none())
    .then_some(dispatch_packet_path)
    .flatten();
    let result_paths = dispatch_result_path_candidates_from_receipt_fields(
        downstream_dispatch_result_path,
        dispatch_result_path,
        packet_fallback_path,
    );
    if !result_paths
        .iter()
        .any(|path| dispatch_rework_route_from_result_path(path).is_some())
    {
        return Ok(None);
    }
    store.show_task(task_id).await.map_err(|error| {
        rework_authority_blocker(
            "team_flow_rework_task_missing",
            task_id,
            vec![error.to_string()],
        )
    })?;
    let role_selection_value = packet
        .get("role_selection_full")
        .or_else(|| packet.get("role_selection"))
        .cloned()
        .ok_or_else(|| {
            rework_authority_blocker(
                "team_flow_rework_role_selection_missing",
                "role_selection",
                Vec::new(),
            )
        })?;
    let role_selection = crate::taskflow_run_graph::rehydrate_persisted_role_selection_value(
        store,
        role_selection_value,
        Some(task_id),
    )
    .await
    .map_err(|error| {
        rework_authority_blocker(
            "team_flow_rework_role_selection_rehydrate_failed",
            task_id,
            vec![error],
        )
    })?;
    let (authority, execution_plan) =
        team_flow_authority_from_rehydrated_selection(&role_selection)?;
    let completed_target = completed_result_target(&packet, completed_dispatch_target);
    for result_path in result_paths {
        if let Some(route) = dispatch_rework_route_from_result_path(&result_path) {
            rework_route_is_authorized(&authority, &execution_plan, &completed_target, &route)?;
            return Ok(Some(AuthorizedDispatchReworkContext {
                route,
                role_selection,
            }));
        }
    }
    Ok(None)
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

pub(crate) fn read_bounded_dispatch_evidence_json(path: &str) -> Option<serde_json::Value> {
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

fn rework_authority_blocker(
    code: impl Into<String>,
    requested: impl Into<String>,
    candidates: Vec<String>,
) -> crate::team_flow_authority_adapter::TeamFlowResolutionBlocker {
    crate::team_flow_authority_adapter::TeamFlowResolutionBlocker {
        code: code.into(),
        requested: requested.into(),
        candidates,
    }
}

fn team_flow_authority_from_rehydrated_selection(
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> Result<
    (
        crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
        serde_json::Value,
    ),
    crate::team_flow_authority_adapter::TeamFlowResolutionBlocker,
> {
    if selection.compiled_bundle.is_null() {
        return Err(rework_authority_blocker(
            "team_flow_rework_compiled_bundle_missing_after_rehydrate",
            "compiled_bundle",
            Vec::new(),
        ));
    }
    let execution_plan = selection.execution_plan.clone();
    if !execution_plan.is_object() {
        return Err(rework_authority_blocker(
            "team_flow_authority_execution_plan_missing",
            "execution_plan",
            Vec::new(),
        ));
    }
    let flow_ref = crate::runtime_dispatch_state::validated_selected_flow_ref(
        selection,
        None,
        crate::runtime_dispatch_state::SelectedFlowIdentityMode::Replay,
    )?;
    let projection = crate::team_flow_authority_adapter::compile_team_flow_authority(
        &selection.compiled_bundle,
        flow_ref.as_deref(),
        None,
    )
    .map_err(|error| {
        rework_authority_blocker(
            "team_flow_rework_authority_compile_failed",
            flow_ref.as_deref().unwrap_or_default(),
            vec![error],
        )
    })?;
    if let Some(plan_authority_id) =
        execution_plan["development_flow"]["dispatch_contract"]["team_flow_authority_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    {
        if plan_authority_id != projection.authority_id {
            return Err(rework_authority_blocker(
                "team_flow_authority_plan_identity_mismatch",
                plan_authority_id,
                vec![projection.authority_id.clone()],
            ));
        }
    }
    Ok((projection, execution_plan))
}

#[cfg(test)]
fn packet_team_flow_authority(
    packet: &serde_json::Value,
) -> Result<
    (
        crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
        serde_json::Value,
    ),
    crate::team_flow_authority_adapter::TeamFlowResolutionBlocker,
> {
    let role_selection = packet
        .get("role_selection_full")
        .or_else(|| packet.get("role_selection"))
        .ok_or_else(|| {
            rework_authority_blocker(
                "team_flow_authority_role_selection_missing",
                "role_selection",
                Vec::new(),
            )
        })?;
    let execution_plan = role_selection
        .get("execution_plan")
        .cloned()
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            rework_authority_blocker(
                "team_flow_authority_execution_plan_missing",
                "execution_plan",
                Vec::new(),
            )
        })?;
    let compiled_bundle = role_selection.get("compiled_bundle").ok_or_else(|| {
        rework_authority_blocker(
            "team_flow_authority_bundle_missing",
            "compiled_bundle",
            Vec::new(),
        )
    })?;
    let flow_ref = execution_plan["development_flow"]["dispatch_contract"]["selected_flow_set"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let projection = crate::team_flow_authority_adapter::compile_team_flow_authority(
        compiled_bundle,
        flow_ref,
        None,
    )
    .map_err(|error| {
        rework_authority_blocker(
            "team_flow_authority_compile_failed",
            flow_ref.unwrap_or_default(),
            vec![error],
        )
    })?;
    if let Some(plan_authority_id) =
        execution_plan["development_flow"]["dispatch_contract"]["team_flow_authority_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    {
        if plan_authority_id != projection.authority_id {
            return Err(rework_authority_blocker(
                "team_flow_authority_plan_identity_mismatch",
                plan_authority_id,
                vec![projection.authority_id.clone()],
            ));
        }
    }
    Ok((projection, execution_plan))
}

fn completed_result_target(packet: &serde_json::Value, fallback: &str) -> String {
    [
        packet.get("dispatch_target"),
        packet.get("downstream_dispatch_target"),
        packet.get("source_dispatch_target"),
    ]
    .into_iter()
    .find_map(|value| {
        value
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
    .unwrap_or_else(|| fallback.trim().to_string())
}

pub(crate) fn rework_route_is_authorized(
    authority: &crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
    execution_plan: &serde_json::Value,
    completed_dispatch_target: &str,
    route: &DispatchReworkRoute,
) -> Result<(), crate::team_flow_authority_adapter::TeamFlowResolutionBlocker> {
    if !route.receipt_backed {
        return Err(rework_authority_blocker(
            taskflow_authority::team_flow_transition::BLOCKER_RECEIPT_REQUIRED,
            completed_dispatch_target,
            Vec::new(),
        ));
    }
    if !route.outcome_blocker_codes.is_empty() {
        return Err(rework_authority_blocker(
            taskflow_authority::team_flow_transition::BLOCKER_RECEIPT_NOT_COMPLETED,
            completed_dispatch_target,
            route.outcome_blocker_codes.clone(),
        ));
    }
    let source = crate::team_flow_authority_adapter::resolve_team_flow_node(
        authority,
        Some(execution_plan),
        completed_dispatch_target,
    )?;
    let target = crate::team_flow_authority_adapter::resolve_team_flow_node(
        authority,
        Some(execution_plan),
        &route.rework_target,
    )?;
    let allowed = crate::team_flow_authority_adapter::resolve_team_flow_node(
        authority,
        Some(execution_plan),
        &route.allowed_next_node,
    )?;
    if target.node_id != allowed.node_id {
        return Err(rework_authority_blocker(
            "team_flow_rework_route_target_mismatch",
            &route.allowed_next_node,
            vec![target.node_id, allowed.node_id],
        ));
    }
    if !source
        .rework_targets
        .iter()
        .any(|configured| configured == &target.node_id)
    {
        return Err(rework_authority_blocker(
            taskflow_authority::team_flow_transition::BLOCKER_REWORK_TARGET_NOT_CONFIGURED,
            target.node_id,
            source.rework_targets,
        ));
    }
    Ok(())
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
        rework_target: rework_target.to_string(),
        allowed_next_node: allowed_next_node.to_string(),
        blocker_code: result_blocker_code(result),
        receipt_backed: result
            .pointer("/execution_evidence/receipt_backed")
            .and_then(serde_json::Value::as_bool)
            == Some(true),
        outcome_blocker_codes: rework_result_contract_blockers(result),
    })
}

fn rework_result_contract_blockers(result: &serde_json::Value) -> Vec<String> {
    let mut required_fields = taskflow_host_bridge::default_host_bridge_required_result_fields();
    for field in ["status", "execution_state"] {
        if !required_fields.iter().any(|required| required == field) {
            required_fields.push(field.to_string());
        }
    }
    taskflow_host_bridge::host_bridge_result_verdict_contract_blockers(result, &required_fields)
}

fn dispatch_result_has_rework_verdict(result: &serde_json::Value) -> bool {
    if dispatch_result_has_authoritative_pass_verdict(result) {
        return false;
    }
    dispatch_result_field_is_rework_verdict(result.get("decision"))
        || dispatch_result_field_is_rework_verdict(result.get("verdict"))
        || dispatch_result_field_is_rework_verdict(result.get("completion_verdict"))
        || result.get("execution_evidence").is_some_and(|evidence| {
            dispatch_result_field_is_rework_verdict(evidence.get("decision"))
                || dispatch_result_field_is_rework_verdict(evidence.get("verdict"))
                || dispatch_result_field_is_rework_verdict(evidence.get("completion_verdict"))
        })
}

fn dispatch_result_has_authoritative_pass_verdict(result: &serde_json::Value) -> bool {
    let status = result
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let execution_state = result
        .get("execution_state")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let decision = result
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let verdict = result
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let blocker_codes_empty = result
        .get("blocker_codes")
        .and_then(serde_json::Value::as_array)
        .is_none_or(Vec::is_empty);
    status == Some("pass")
        && execution_state == Some("executed")
        && decision == Some("approve")
        && verdict == Some("pass")
        && blocker_codes_empty
}

fn dispatch_result_field_is_rework_verdict(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .map(|text| text.trim().to_ascii_lowercase())
        .is_some_and(|text| {
            matches!(
                text.as_str(),
                "rework" | "rework_required" | "blocked" | "blocker"
            )
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
    let backend_id = canonical_lane_receipt_backend_id_for_result(receipt, body);
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
    let mut artifact = serde_json::to_value(
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
                carrier_id: carrier_id.unwrap_or_else(|| "unknown".to_string()),
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
    .expect("lane execution receipt artifact should serialize");
    if let Some(object) = artifact.as_object_mut() {
        object.insert(
            "backend_id".to_string(),
            serde_json::Value::String(backend_id.unwrap_or_else(|| "unknown".to_string())),
        );
    }
    artifact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_dispatch_evidence_reader_rejects_oversized_files() {
        let root = unique_test_dir("oversized-dispatch-evidence");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let path = root.join("result.json");
        let file = std::fs::File::create(&path).expect("result file should be created");
        file.set_len(MAX_DISPATCH_EVIDENCE_JSON_BYTES + 1)
            .expect("result file should be oversized");

        assert!(read_bounded_dispatch_evidence_json(&path.display().to_string()).is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bounded_dispatch_evidence_reader_rejects_empty_missing_and_malformed_files() {
        let root = unique_test_dir("invalid-dispatch-evidence");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let malformed_path = root.join("malformed.json");
        std::fs::write(&malformed_path, "not-json").expect("malformed result should write");

        assert!(read_bounded_dispatch_evidence_json("").is_none());
        assert!(read_bounded_dispatch_evidence_json(
            &root.join("missing.json").display().to_string()
        )
        .is_none());
        assert!(read_bounded_dispatch_evidence_json(&malformed_path.display().to_string()).is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_dispatch_evidence_reader_rejects_special_files() {
        assert!(read_bounded_dispatch_evidence_json("/dev/zero").is_none());
    }

    fn identity_test_receipt(
        selected_backend: Option<&str>,
    ) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-identity".to_string(),
            dispatch_target: "lane-identity".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/identity-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: selected_backend.map(str::to_string),
            recorded_at: "2026-07-22T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn normalized_execution_evidence_keeps_backend_and_carrier_distinct() {
        let receipt = identity_test_receipt(Some("receipt-backend"));
        let body = serde_json::json!({
            "activation_semantics": {"activation_kind": "execution_evidence"},
            "execution_evidence": {"status": "recorded"},
            "backend_dispatch": {
                "backend_id": "executor-backend",
                "carrier_id": "carrier-token"
            },
            "execution_state": "executed"
        });

        let normalized = normalized_dispatch_result_activation_evidence(
            &receipt,
            &body,
            "/tmp/identity-result.json",
        );
        assert_eq!(
            normalized["execution_evidence"]["backend_id"],
            "executor-backend"
        );
        assert_eq!(
            normalized["execution_evidence"]["carrier_id"],
            "carrier-token"
        );
        assert_ne!(
            normalized["execution_evidence"]["backend_id"],
            normalized["execution_evidence"]["carrier_id"]
        );

        let artifact = canonical_lane_execution_receipt_artifact_json(
            &receipt,
            &body,
            "2026-07-22T00:01:00Z",
            "/tmp/identity-result.json",
        );
        assert_eq!(artifact["backend_id"], "executor-backend");
        assert_eq!(artifact["carrier_id"], "carrier-token");
    }

    #[test]
    fn carrier_field_never_populates_missing_backend_identity() {
        let receipt = identity_test_receipt(None);
        let body = serde_json::json!({
            "activation_semantics": {"activation_kind": "execution_evidence"},
            "backend_dispatch": {"carrier_id": "carrier-only"},
            "execution_state": "executed"
        });

        let normalized = normalized_dispatch_result_activation_evidence(
            &receipt,
            &body,
            "/tmp/carrier-only-result.json",
        );
        assert!(normalized["execution_evidence"].get("backend_id").is_none());
        assert_eq!(
            normalized["execution_evidence"]["carrier_id"],
            "carrier-only"
        );
    }

    #[test]
    fn dispatch_rework_route_accepts_legacy_top_level_completion_verdict() {
        let result = serde_json::json!({
            "status": "blocked",
            "completion_verdict": "blocked",
            "rework_target": "alpha_impl",
            "allowed_next_node": "alpha-impl-rework",
            "blocker_codes": ["verification_rework_required"]
        });

        let route = dispatch_rework_route_from_result(&result)
            .expect("legacy completion_verdict should produce a rework route");
        assert_eq!(route.rework_target, "alpha_impl");
        assert_eq!(route.allowed_next_node, "alpha-impl-rework");
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
            "rework_target": "gamma_verify",
            "allowed_next_node": "gamma_verify",
            "blocker_code": "review_rework_required"
        });

        let route = dispatch_rework_route_from_result(&result)
            .expect("nested completion_verdict should produce a rework route");
        assert_eq!(route.rework_target, "gamma_verify");
        assert_eq!(route.allowed_next_node, "gamma_verify");
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
            "rework_target": "alpha_impl",
            "allowed_next_node": "alpha_impl_rework"
        });

        assert!(dispatch_rework_route_from_result(&result).is_none());
    }

    #[test]
    fn dispatch_rework_route_rejects_stale_nested_rework_when_top_level_passes() {
        let result = serde_json::json!({
            "status": "pass",
            "execution_state": "executed",
            "decision": "approve",
            "verdict": "pass",
            "blocker_codes": [],
            "completion_verdict": "pass",
            "rework_target": null,
            "allowed_next_node": "gamma_verify",
            "execution_evidence": {
                "receipt_backed": true,
                "decision": "rework_required",
                "verdict": "rework_required",
                "completion_verdict": "rework_required"
            }
        });

        assert!(dispatch_rework_route_from_result(&result).is_none());
    }

    fn rework_authority_fixture(
        duplicate_target_alias: bool,
    ) -> (
        serde_json::Value,
        crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
        serde_json::Value,
    ) {
        let bundle = crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let mut authority =
            crate::team_flow_authority_adapter::compile_team_flow_authority(&bundle, None, None)
                .expect("canonical fixture authority should compile");
        let flow_id = authority.snapshot.flow_ref.clone();
        let selected_node_id = authority.entry_node_id.clone();
        let lane_catalog =
            authority
                .nodes
                .iter()
                .fold(serde_json::Map::new(), |mut catalog, node| {
                    catalog.insert(
                        node.node.node_id.clone(),
                        serde_json::json!({
                        "node_id": node.node.node_id.clone(),
                        "dispatch_target": node.dispatch_target.clone(),
                        "dispatch_alias": node.dispatch_alias.clone(),
                        "runtime_role": node.node.runtime_role.clone(),
                        "task_class": node.node.task_class.clone()
                        }),
                    );
                    catalog
                });
        let execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": flow_id.clone(),
            "team_flow_authority_selected_node_id": selected_node_id.clone(),
            "selected_flow_contract": {
                "flow_id": flow_id.clone(),
                "selected_node_id": selected_node_id.clone()
            },
            "development_flow": {
                "dispatch_contract": {
                    "selected_flow_set": flow_id.clone(),
                    "selected_node_id": selected_node_id.clone(),
                    "team_flow_authority_selected_node_id": selected_node_id.clone(),
                    "team_flow_authority_id": authority.authority_id.clone(),
                    "team_flow_config_hash": authority.config_authority_hash.clone(),
                    "team_flow_registry_hash": authority.registry_authority_hash.clone(),
                    "lane_catalog": lane_catalog
                }
            }
        });
        if duplicate_target_alias {
            let source_id = authority
                .nodes
                .iter()
                .find(|node| !node.node.rework_targets.is_empty())
                .map(|node| node.node.node_id.clone())
                .expect("canonical fixture should declare a rework source");
            let target_id = authority
                .node(&source_id)
                .and_then(|node| node.node.rework_targets.first())
                .cloned()
                .expect("canonical fixture should declare a rework target");
            let target_dispatch_target = authority
                .node(&target_id)
                .map(|node| node.dispatch_target.clone())
                .expect("canonical fixture target should resolve");
            authority
                .nodes
                .iter_mut()
                .find(|node| node.node.node_id != source_id && node.node.node_id != target_id)
                .expect("canonical fixture should expose a duplicate-alias candidate")
                .dispatch_alias = target_dispatch_target;
        }
        (bundle, authority, execution_plan)
    }

    #[test]
    fn rehydrated_result_authority_rejects_forged_conflicting_flow_identity() {
        let selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "resume rework".to_string(),
            selected_role: "stage-a".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("fixture-flow".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["dev_team_flow_id:forged-result-flow-c".to_string()],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({
                "team_flow_authority_selected_flow_id": "forged-result-flow-a",
                "development_flow": {
                    "dispatch_contract": {
                        "selected_flow_set": "forged-result-flow-b"
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let blocker = team_flow_authority_from_rehydrated_selection(&selection)
            .expect_err("forged result flow identity must fail closed");
        assert_eq!(blocker.code, "team_flow_selected_flow_identity_conflict");
    }

    fn configured_rework_nodes(
        authority: &crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
        execution_plan: &serde_json::Value,
    ) -> (
        crate::team_flow_authority_adapter::TeamFlowNodeResolution,
        crate::team_flow_authority_adapter::TeamFlowNodeResolution,
    ) {
        let source_id = authority
            .nodes
            .iter()
            .find(|node| !node.node.rework_targets.is_empty())
            .map(|node| node.node.node_id.as_str())
            .expect("fixture should declare a rework source");
        let source = crate::team_flow_authority_adapter::resolve_team_flow_node(
            authority,
            Some(execution_plan),
            source_id,
        )
        .expect("configured source should resolve");
        let target = crate::team_flow_authority_adapter::resolve_team_flow_node(
            authority,
            Some(execution_plan),
            source
                .rework_targets
                .first()
                .expect("fixture should declare a rework target"),
        )
        .expect("configured target should resolve");
        (source, target)
    }

    fn rework_route_result(
        status: &str,
        execution_state: &str,
        rework_target: &str,
        allowed_next_node: &str,
        receipt_backed: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "status": status,
            "execution_state": execution_state,
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["configured_rework_required"],
            "rework_target": rework_target,
            "allowed_next_node": allowed_next_node,
            "execution_evidence": {"receipt_backed": receipt_backed}
        })
    }

    fn receipt_backed_rework_route(
        rework_target: &str,
        allowed_next_node: &str,
    ) -> DispatchReworkRoute {
        let blocked = taskflow_contracts::Release1ContractStatus::Blocked.as_str();
        dispatch_rework_route_from_result(&rework_route_result(
            blocked,
            blocked,
            rework_target,
            allowed_next_node,
            true,
        ))
        .expect("configured rework result should produce a route")
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
                "rework_target": "alpha_impl",
                "allowed_next_node": "alpha-impl-rework",
                "blocker_code": "verification_rework_required"
            })
            .to_string(),
        )
        .expect("result should write");

        let route = dispatch_rework_route_from_result_path(&result_path.display().to_string())
            .expect("bounded regular result json should parse");
        assert_eq!(route.rework_target, "alpha_impl");
        assert_eq!(route.allowed_next_node, "alpha-impl-rework");
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
    fn completed_result_target_prefers_completed_lane_then_downstream_then_source() {
        assert_eq!(
            completed_result_target(
                &serde_json::json!({
                    "dispatch_target": "gamma-review",
                    "downstream_dispatch_target": "terminal_closure",
                    "source_dispatch_target": "beta_verify"
                }),
                "fallback_lane"
            ),
            "gamma-review"
        );
        assert_eq!(
            completed_result_target(
                &serde_json::json!({
                    "downstream_dispatch_target": "alpha-impl",
                    "source_dispatch_target": "beta_gate"
                }),
                "fallback_lane"
            ),
            "alpha-impl"
        );
        assert_eq!(
            completed_result_target(
                &serde_json::json!({
                    "source_dispatch_target": "beta-gate"
                }),
                "fallback_lane"
            ),
            "beta-gate"
        );
    }

    #[test]
    fn rework_route_authorization_accepts_only_explicit_configured_edge() {
        let (_, authority, execution_plan) = rework_authority_fixture(false);
        let (source, target) = configured_rework_nodes(&authority, &execution_plan);
        let route = receipt_backed_rework_route(&target.node_id, &target.dispatch_target);

        rework_route_is_authorized(&authority, &execution_plan, &source.dispatch_target, &route)
            .expect("explicit configured rework edge should authorize");
    }

    #[test]
    fn rework_route_authorization_rejects_unconfigured_node_even_when_resolvable() {
        let (_, authority, execution_plan) = rework_authority_fixture(false);
        let (source, configured_target) = configured_rework_nodes(&authority, &execution_plan);
        let unconfigured_target = authority
            .nodes
            .iter()
            .find_map(|node| {
                if node.node.node_id == source.node_id
                    || node.node.node_id == configured_target.node_id
                {
                    return None;
                }
                crate::team_flow_authority_adapter::resolve_team_flow_node(
                    &authority,
                    Some(&execution_plan),
                    &node.node.node_id,
                )
                .ok()
            })
            .expect("fixture should expose an unconfigured target");
        let route =
            receipt_backed_rework_route(&unconfigured_target.node_id, &unconfigured_target.node_id);

        let blocker =
            rework_route_is_authorized(&authority, &execution_plan, &source.node_id, &route)
                .expect_err("resolvable but undeclared edge must fail closed");
        assert_eq!(
            blocker.code,
            taskflow_authority::team_flow_transition::BLOCKER_REWORK_TARGET_NOT_CONFIGURED
        );
    }

    #[test]
    fn explicit_receipt_result_path_does_not_fall_back_to_stale_packet_rework() {
        let root = unique_test_dir("dispatch-result-explicit-path");
        std::fs::create_dir_all(&root).expect("test dir should be created");
        let packet_path = root.join("current-packet.json");
        let stale_result_path = root.join("stale-result.json");
        let current_result_path = root.join("current-result.json");
        let (bundle, authority, execution_plan) = rework_authority_fixture(false);
        let (source, target) = configured_rework_nodes(&authority, &execution_plan);
        let blocked = taskflow_contracts::Release1ContractStatus::Blocked.as_str();
        std::fs::write(
            &stale_result_path,
            rework_route_result(
                blocked,
                blocked,
                &target.node_id,
                &target.dispatch_target,
                true,
            )
            .to_string(),
        )
        .expect("stale result should write");
        std::fs::write(
            &current_result_path,
            serde_json::json!({
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "host_tool_bridge_adapter_required"
            })
            .to_string(),
        )
        .expect("current result should write");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-current",
                "dispatch_target": source.dispatch_target,
                "role_selection_full": {
                    "compiled_bundle": bundle,
                    "execution_plan": execution_plan
                },
                "downstream_dispatch_result_path": stale_result_path
            })
            .to_string(),
        )
        .expect("current packet should write");

        assert!(
            authorized_dispatch_rework_route_from_receipt_fields(
                None,
                Some(&current_result_path.display().to_string()),
                Some(&packet_path.display().to_string()),
                &source.node_id,
            )
            .is_none(),
            "stale packet rework must not supersede an explicit current result"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rework_route_authorization_reports_missing_and_ambiguous_typed_authority() {
        let (_, authority, execution_plan) = rework_authority_fixture(false);
        let (source, target) = configured_rework_nodes(&authority, &execution_plan);
        let missing = receipt_backed_rework_route(&target.node_id, "unconfigured-target");
        let missing_blocker =
            rework_route_is_authorized(&authority, &execution_plan, &source.node_id, &missing)
                .expect_err("missing authority must fail closed");
        assert_eq!(missing_blocker.code, "team_flow_node_resolution_missing");

        let (_, ambiguous_authority, ambiguous_plan) = rework_authority_fixture(true);
        let (ambiguous_source, ambiguous_target) =
            configured_rework_nodes(&ambiguous_authority, &ambiguous_plan);
        let ambiguous = receipt_backed_rework_route(
            &ambiguous_target.node_id,
            &ambiguous_target.dispatch_target,
        );
        let ambiguous_blocker = rework_route_is_authorized(
            &ambiguous_authority,
            &ambiguous_plan,
            &ambiguous_source.node_id,
            &ambiguous,
        )
        .expect_err("ambiguous authority must fail closed");
        assert_eq!(
            ambiguous_blocker.code,
            "team_flow_node_resolution_ambiguous"
        );
    }

    #[test]
    fn rework_route_authorization_requires_consistent_receipt_outcome() {
        let (_, authority, execution_plan) = rework_authority_fixture(false);
        let (source, target) = configured_rework_nodes(&authority, &execution_plan);
        let blocked = taskflow_contracts::Release1ContractStatus::Blocked.as_str();
        let pass = taskflow_contracts::Release1ContractStatus::Pass.as_str();

        let missing_receipt = dispatch_rework_route_from_result(&rework_route_result(
            blocked,
            blocked,
            &target.node_id,
            &target.dispatch_target,
            false,
        ))
        .expect("rework shape should parse before authorization");
        let missing_receipt_blocker = rework_route_is_authorized(
            &authority,
            &execution_plan,
            &source.node_id,
            &missing_receipt,
        )
        .expect_err("missing receipt evidence must fail closed");
        assert_eq!(
            missing_receipt_blocker.code,
            taskflow_authority::team_flow_transition::BLOCKER_RECEIPT_REQUIRED
        );

        let contradictory_outcome = dispatch_rework_route_from_result(&rework_route_result(
            pass,
            blocked,
            &target.node_id,
            &target.dispatch_target,
            true,
        ))
        .expect("contradictory rework shape should reach authorization blocker");
        let outcome_blocker = rework_route_is_authorized(
            &authority,
            &execution_plan,
            &source.node_id,
            &contradictory_outcome,
        )
        .expect_err("contradictory receipt outcome must fail closed");
        assert_eq!(
            outcome_blocker.code,
            taskflow_authority::team_flow_transition::BLOCKER_RECEIPT_NOT_COMPLETED
        );
        assert!(
            outcome_blocker
                .candidates
                .iter()
                .any(|code| code == "host_bridge_result_decision_verdict_mismatch")
        );
    }
}
