use crate::taskflow_run_graph::validate_run_graph_resume_gate;
use std::process::ExitCode;

const DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS: [&str; 3] = [
    ".vida/data/state/runtime-consumption",
    "docs/product/spec",
    "docs/process",
];

fn missing_dispatch_packet_path_error(latest: bool) -> String {
    let _ = super::blocker_code_str(super::BlockerCode::MissingPacket);
    if latest {
        "Latest persisted dispatch receipt is missing dispatch_packet_path".to_string()
    } else {
        "Persisted dispatch receipt is missing dispatch_packet_path".to_string()
    }
}

fn missing_dispatch_receipt_error(run_id: &str) -> String {
    let _ = super::blocker_code_str(super::BlockerCode::MissingLaneReceipt);
    format!("No persisted run-graph dispatch receipt exists for run_id `{run_id}`")
}

fn lane_status_pair_is_resume_compatible(
    packet_lane_status: super::LaneStatus,
    derived_lane_status: super::LaneStatus,
) -> bool {
    if packet_lane_status == derived_lane_status {
        return true;
    }
    matches!(
        (packet_lane_status, derived_lane_status),
        (super::LaneStatus::LaneRunning, super::LaneStatus::LaneOpen)
            | (super::LaneStatus::LaneOpen, super::LaneStatus::LaneRunning)
            | (
                super::LaneStatus::LaneRunning,
                super::LaneStatus::PacketReady
            )
            | (
                super::LaneStatus::PacketReady,
                super::LaneStatus::LaneRunning
            )
            | (super::LaneStatus::LaneOpen, super::LaneStatus::PacketReady)
            | (super::LaneStatus::PacketReady, super::LaneStatus::LaneOpen)
            | (
                super::LaneStatus::LaneRunning,
                super::LaneStatus::LaneBlocked
            )
            | (
                super::LaneStatus::LaneBlocked,
                super::LaneStatus::LaneRunning
            )
            | (
                super::LaneStatus::LaneExceptionRecorded,
                super::LaneStatus::LaneExceptionTakeover
            )
            | (
                super::LaneStatus::LaneExceptionTakeover,
                super::LaneStatus::LaneExceptionRecorded
            )
    )
}

fn validate_receipt_packet_pair(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet: &serde_json::Value,
    packet_path: &str,
    packet_label: &str,
) -> Result<(), String> {
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Persisted {packet_label} is missing run_id"))?;
    if packet_run_id != receipt.run_id {
        return Err(format!(
            "Persisted {packet_label} run_id `{packet_run_id}` does not match dispatch receipt run_id `{}`",
            receipt.run_id
        ));
    }
    if let Some(expected_dispatch_packet_path) = receipt.dispatch_packet_path.as_deref() {
        if expected_dispatch_packet_path != packet_path {
            let expected_downstream_packet_path =
                receipt.downstream_dispatch_packet_path.as_deref();
            if expected_downstream_packet_path != Some(packet_path) {
                return Err(format!(
                    "Persisted dispatch receipt expects dispatch_packet_path `{expected_dispatch_packet_path}` but resolved `{packet_path}`"
                ));
            }
        }
    }
    if let Some(packet_lane_status) = packet
        .get("lane_status")
        .and_then(serde_json::Value::as_str)
        .and_then(canonical_resume_lane_status)
    {
        let packet_dispatch_status = canonical_resume_dispatch_status(
            packet
                .get("dispatch_status")
                .and_then(serde_json::Value::as_str),
        );
        let mut derived_lane_status = super::derive_lane_status(
            packet_dispatch_status,
            packet
                .get("supersedes_receipt_id")
                .and_then(serde_json::Value::as_str),
            packet
                .get("exception_path_receipt_id")
                .and_then(serde_json::Value::as_str),
        );
        if packet_lane_status == super::LaneStatus::LaneCompleted
            && packet_dispatch_status == "executed"
        {
            derived_lane_status = super::LaneStatus::LaneCompleted;
        }
        if !lane_status_pair_is_resume_compatible(packet_lane_status, derived_lane_status) {
            return Err(format!(
                "Persisted {packet_label} lane_status `{}` conflicts with derived lane_status `{}` from lane evidence",
                packet_lane_status.as_str(),
                derived_lane_status.as_str()
            ));
        }
    }
    Ok(())
}

async fn validate_run_graph_resume_state(
    store: &super::StateStore,
    run_id: &str,
) -> Result<(), String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(error) => {
            let receipt_exists =
                matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)));
            if receipt_exists && resume_from_persisted_final_snapshot(store)? {
                return Ok(());
            }
            return Err(format!(
                "Failed to read persisted run-graph state for `{run_id}`: {error}"
            ));
        }
    };
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph state mismatch: requested run_id `{run_id}` resolved to `{}`",
            status.run_id
        ));
    }
    if status.lifecycle_stage == "closure_complete"
        && status.status == "completed"
        && status.resume_target == "none"
        && matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)))
    {
        return Ok(());
    }
    match validate_run_graph_resume_gate(&status) {
        Ok(()) => Ok(()),
        Err(_error) if resume_from_persisted_final_snapshot(store)? => Ok(()),
        Err(error) => Err(error),
    }
}

fn persisted_dispatch_packet_lineage_task_id(packet: &serde_json::Value) -> Option<&str> {
    packet
        .pointer("/run_graph_bootstrap/latest_status/task_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            packet
                .pointer("/run_graph_bootstrap/task_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            packet
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
}

async fn validate_explicit_task_graph_binding_lineage_for_resume(
    store: &super::StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let status = store
        .run_graph_status(run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read persisted run-graph state for `{run_id}` while reconciling explicit continuation binding: {error}"
            )
        })?;
    if status.status != "completed" {
        return Ok(());
    }

    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(());
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return Ok(());
    }

    let bound_task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(binding.task_id.as_str());
    let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return Ok(());
    };
    let packet = read_dispatch_packet(packet_path)?;
    validate_receipt_packet_pair(receipt, &packet, packet_path, "dispatch packet")?;
    let Some(lineage_task_id) = persisted_dispatch_packet_lineage_task_id(&packet) else {
        return Ok(());
    };
    if lineage_task_id == bound_task_id {
        return Ok(());
    }

    Err(format!(
        "Completed run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but persisted dispatch packet lineage at `{packet_path}` still points to task `{lineage_task_id}`. Resume must fail closed until a fresh dispatch packet is recorded for the bound task."
    ))
}

async fn validate_completed_run_downstream_resume_candidate(
    store: &super::StateStore,
    run_id: &str,
    candidate_target: &str,
    candidate_source: &str,
) -> Result<(), String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(()),
    };
    if status.status != "completed" {
        return Ok(());
    }

    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(());
    };
    if binding.status != "bound" {
        return Ok(());
    }

    match binding.active_bounded_unit["kind"].as_str() {
        Some("downstream_dispatch_target") => {
            let Some(bound_target) = binding.active_bounded_unit["dispatch_target"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(());
            };
            if bound_target == candidate_target {
                return Ok(());
            }
            Err(format!(
                "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted {candidate_source} still points to stale downstream target `{candidate_target}`. Resume must fail closed until lawful closure-bound evidence is refreshed."
            ))
        }
        Some("task_graph_task") => {
            let bound_task_id = binding.active_bounded_unit["task_id"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(binding.task_id.as_str());
            Err(format!(
                "Completed run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but persisted {candidate_source} still points to downstream target `{candidate_target}`. Resume must fail closed until a fresh dispatch packet is recorded for the bound task."
            ))
        }
        _ => Ok(()),
    }
}

async fn completed_run_explicit_downstream_target_for_resume(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<String>, String> {
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph state for `{run_id}`: {error}")
    })?;
    if status.status != "completed" {
        return Ok(None);
    }

    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| format!("Failed to read explicit continuation binding for `{run_id}`: {error}"))?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("downstream_dispatch_target")
    {
        return Ok(None);
    }

    Ok(binding.active_bounded_unit["dispatch_target"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

fn missing_explicit_downstream_resume_evidence_error(run_id: &str, bound_target: &str) -> String {
    format!(
        "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but no lawful `{bound_target}` downstream packet or result is persisted. Resume must fail closed instead of replaying stale root dispatch lineage."
    )
}

pub(crate) fn build_failure_control_evidence(
    source_run_id: &str,
    source_dispatch_packet_path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "rollback": {
            "status": "recorded",
            "summary": "rollback posture recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
        "incident": {
            "status": "recorded",
            "summary": "incident evidence bundle recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
        "restore": {
            "status": "recorded",
            "summary": "restore trace recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
    })
}

fn failure_control_evidence_entry_is_complete(entry: Option<&serde_json::Value>) -> bool {
    let Some(entry) = entry.and_then(serde_json::Value::as_object) else {
        return false;
    };
    entry
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("source_run_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("source_dispatch_packet_path")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

fn runtime_consumption_snapshot_has_failure_control_evidence(snapshot: &serde_json::Value) -> bool {
    let Some(evidence) = snapshot
        .get("failure_control_evidence")
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("failure_control_evidence"))
        })
        .and_then(serde_json::Value::as_object)
    else {
        return false;
    };

    ["rollback", "incident", "restore"]
        .iter()
        .all(|key| failure_control_evidence_entry_is_complete(evidence.get(*key)))
}

fn final_snapshot_missing_failure_control_evidence(snapshot_path: &str) -> bool {
    let payload = match std::fs::read_to_string(snapshot_path) {
        Ok(payload) => payload,
        Err(_) => return true,
    };
    let summary_json = match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(json) => json,
        Err(_) => return true,
    };
    !runtime_consumption_snapshot_has_failure_control_evidence(&summary_json)
}

fn resume_from_persisted_final_snapshot(store: &super::StateStore) -> Result<bool, String> {
    let Some(snapshot_path) = super::latest_final_runtime_consumption_snapshot_path(store.root())?
    else {
        return Ok(false);
    };
    Ok(!final_snapshot_missing_failure_control_evidence(
        &snapshot_path,
    ))
}

async fn runtime_consumption_resume_blocker_code(
    store: &super::StateStore,
    payload_json: &serde_json::Value,
    explicit_run_id: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(run_id) = explicit_run_id {
        return crate::runtime_consumption_state::runtime_consumption_final_dispatch_receipt_blocker_code_for_run(
            store,
            payload_json,
            run_id,
        );
    }
    crate::runtime_consumption_state::runtime_consumption_final_dispatch_receipt_blocker_code(
        store,
        payload_json,
    )
}

async fn emit_runtime_consumption_resume_json(
    store: &super::StateStore,
    surface_name: &str,
    dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    explicit_run_id: Option<&str>,
    emit_output: bool,
    as_json: bool,
) -> Result<(), String> {
    let mut normalized_dispatch_receipt = dispatch_receipt.clone();
    if normalized_dispatch_receipt.dispatch_kind == "agent_lane" {
        normalized_dispatch_receipt.selected_backend =
            super::canonical_selected_backend_for_receipt(
                role_selection,
                &normalized_dispatch_receipt,
            );
    }
    let failure_control_evidence =
        build_failure_control_evidence(&normalized_dispatch_receipt.run_id, dispatch_packet_path);
    let mut payload_json = serde_json::json!({
        "dispatch_receipt": normalized_dispatch_receipt,
        "role_selection": role_selection,
        "source_dispatch_packet_path": dispatch_packet_path,
        "source_run_id": dispatch_receipt.run_id,
        "failure_control_evidence": failure_control_evidence.clone(),
    });
    let runtime_dispatch_receipt_blocker_code =
        runtime_consumption_resume_blocker_code(store, &payload_json, explicit_run_id).await?;
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    if let Some(blocker_code) = runtime_dispatch_receipt_blocker_code.as_deref() {
        super::apply_runtime_consumption_final_dispatch_receipt_blocker(
            &mut payload_json,
            blocker_code,
        );
        blocker_codes.push(blocker_code.to_string());
        next_actions.push(
            match blocker_code {
                super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER => {
                    super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION
                }
                _ => super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
            }
            .to_string(),
        );
    }
    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    payload_json["release_admission"] = serde_json::json!({});
    let snapshot = serde_json::json!({
        "surface": surface_name,
        "status": status,
        "release_admission": {},
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    let snapshot_path =
        super::write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let operator_contracts = super::build_release1_operator_contracts_envelope(
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": dispatch_receipt.run_id,
            "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
            "consume_final_surface": surface_name,
        }),
    );
    let shared_fields = serde_json::json!({
        "status": operator_contracts["status"].clone(),
        "blocker_codes": operator_contracts["blocker_codes"].clone(),
        "next_actions": operator_contracts["next_actions"].clone(),
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
    });
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": surface_name,
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
        "release_admission": {},
        "payload": payload_json,
        "failure_control_evidence": failure_control_evidence,
    });
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    if !emit_output {
        return Ok(());
    }
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": surface_name,
                "status": status,
                "blocker_codes": snapshot_with_operator_contracts["blocker_codes"].clone(),
                "next_actions": snapshot_with_operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "shared_fields": shared_fields,
                "operator_contracts": operator_contracts,
                "source_run_id": dispatch_receipt.run_id,
                "source_dispatch_packet_path": dispatch_packet_path,
                "dispatch_receipt": payload_json["dispatch_receipt"].clone(),
                "snapshot_path": snapshot_path,
                "failure_control_evidence": snapshot_with_operator_contracts["failure_control_evidence"].clone(),
            }))
            .expect("resume command should render as json")
        );
    } else {
        super::print_surface_header(super::RenderMode::Plain, surface_name);
        super::print_surface_line(super::RenderMode::Plain, "status", status);
        super::print_surface_line(
            super::RenderMode::Plain,
            "source run",
            &dispatch_receipt.run_id,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "source packet",
            dispatch_packet_path,
        );
        super::print_surface_line(super::RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    Ok(())
}

async fn validate_run_graph_resume_state_for_downstream_packet(
    store: &super::StateStore,
    run_id: &str,
) -> Result<(), String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(error) => {
            let receipt_exists =
                matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)));
            if receipt_exists && resume_from_persisted_final_snapshot(store)? {
                return Ok(());
            }
            return Err(format!(
                "Failed to read persisted run-graph state for `{run_id}`: {error}"
            ));
        }
    };
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph state mismatch: requested run_id `{run_id}` resolved to `{}`",
            status.run_id
        ));
    }
    if status.lifecycle_stage == "closure_complete"
        && status.status == "completed"
        && status.resume_target == "none"
        && matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)))
    {
        return Ok(());
    }
    if status.resume_target == "none" {
        if let Ok(Some(receipt)) = store.run_graph_dispatch_receipt(run_id).await {
            if receipt.downstream_dispatch_ready
                && receipt
                    .downstream_dispatch_packet_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty())
            {
                return Ok(());
            }
        }
    }
    validate_run_graph_resume_gate(&status)
}

fn packet_nonempty_string_array(packet: &serde_json::Value, key: &str) -> bool {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                })
        })
}

fn packet_has_owned_or_read_only_paths(packet: &serde_json::Value) -> bool {
    packet_nonempty_string_array(packet, "owned_paths")
        || packet_nonempty_string_array(packet, "read_only_paths")
}

fn packet_dispatch_target(packet: &serde_json::Value) -> Option<&str> {
    packet
        .get("dispatch_target")
        .or_else(|| packet.get("downstream_dispatch_target"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("scope_in"))
                .and_then(serde_json::Value::as_array)
                .and_then(|scope_in| {
                    scope_in.iter().find_map(|entry| {
                        entry
                            .as_str()
                            .map(str::trim)
                            .and_then(|value| value.strip_prefix("dispatch_target:"))
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                    })
                })
        })
}

fn packet_request_text(packet: &serde_json::Value) -> Option<&str> {
    packet
        .get("request_text")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("request_text"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

fn derive_legacy_owned_paths_from_request(packet: &serde_json::Value) -> Option<Vec<String>> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if packet_template_kind != "delivery_task_packet" {
        return None;
    }

    let dispatch_target = packet_dispatch_target(packet)?;
    if dispatch_target != "implementer" {
        return None;
    }

    let request_text = packet_request_text(packet)?;
    let owned_paths = crate::runtime_dispatch_packets::explicit_request_scope_paths(request_text);
    (!owned_paths.is_empty()).then_some(owned_paths)
}

fn normalize_runtime_dispatch_packet(packet: &mut serde_json::Value) -> bool {
    let Some(packet_template_kind) = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    else {
        return false;
    };
    let derived_owned_paths = derive_legacy_owned_paths_from_request(packet);
    let Some(active_packet) = packet.get_mut(&packet_template_kind) else {
        return false;
    };
    if active_packet.is_null() {
        return false;
    }
    let missing_owned_paths = !packet_nonempty_string_array(active_packet, "owned_paths");
    let missing_scope_paths = !packet_has_owned_or_read_only_paths(active_packet);
    let Some(active_packet_object) = active_packet.as_object_mut() else {
        return false;
    };
    let mut normalized = false;
    if missing_owned_paths {
        if let Some(owned_paths) = derived_owned_paths {
            active_packet_object.insert("owned_paths".to_string(), serde_json::json!(owned_paths));
            normalized = true;
        }
    }
    if missing_scope_paths {
        active_packet_object.insert(
            "read_only_paths".to_string(),
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS),
        );
        normalized = true;
    }
    normalized
}

pub(crate) fn read_dispatch_packet(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted dispatch packet: {error}"))?;
    let mut packet: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted dispatch packet: {error}"))?;
    if normalize_runtime_dispatch_packet(&mut packet) {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&packet)
                .map_err(|error| format!("Failed to encode normalized dispatch packet: {error}"))?,
        )
        .map_err(|error| format!("Failed to persist normalized dispatch packet: {error}"))?;
    }
    crate::validate_runtime_dispatch_packet_contract(&packet, "Persisted dispatch packet")?;
    Ok(packet)
}

pub(crate) struct ResumeInputs {
    pub(crate) dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    pub(crate) dispatch_packet_path: String,
    pub(crate) role_selection: super::RuntimeConsumptionLaneSelection,
    pub(crate) run_graph_bootstrap: serde_json::Value,
}

fn build_resume_inputs(
    dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: String,
    packet: serde_json::Value,
    role_selection: super::RuntimeConsumptionLaneSelection,
) -> ResumeInputs {
    let run_graph_bootstrap = packet
        .get("run_graph_bootstrap")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    ResumeInputs {
        dispatch_receipt,
        dispatch_packet_path,
        role_selection,
        run_graph_bootstrap,
    }
}

async fn recover_missing_first_dispatch_receipt(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<ResumeInputs>, String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };
    if status.status == "completed"
        || !status.lifecycle_stage.ends_with("_active")
        || status.active_node.trim().is_empty()
    {
        return Ok(None);
    }

    let context = store
        .run_graph_dispatch_context(run_id)
        .await
        .map_err(|error| format!("Failed to read persisted run-graph dispatch context: {error}"))?
        .ok_or_else(|| {
            format!(
                "No persisted run-graph dispatch receipt exists for run_id `{run_id}` and missing receipt recovery could not load dispatch context"
            )
        })?;
    let role_selection = context.role_selection().map_err(|error| {
        format!("Failed to decode persisted run-graph dispatch context for `{run_id}`: {error}")
    })?;
    let latest_status = serde_json::to_value(&status)
        .map_err(|error| format!("Failed to encode run-graph status for `{run_id}`: {error}"))?;
    let run_graph_bootstrap = serde_json::json!({
        "status": "dispatch_init_ready",
        "handoff_ready": true,
        "run_id": status.run_id,
        "latest_status": latest_status,
    });

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let dispatch_target = status.active_node.clone();
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, &dispatch_target);
    let selected_backend = super::downstream_selected_backend(
        &role_selection,
        &dispatch_target,
        activation_agent_type.as_deref(),
        None,
    )
    .filter(|value| !value.is_empty());
    let mut dispatch_receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: status.run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: "executed".to_string(),
        lane_status: super::derive_lane_status("executed", None, None)
            .as_str()
            .to_string(),
        supersedes_receipt_id: None,
        exception_path_receipt_id: None,
        dispatch_kind,
        dispatch_surface,
        dispatch_command: super::runtime_dispatch_command_for_target(
            &role_selection,
            &dispatch_target,
        ),
        dispatch_packet_path: None,
        dispatch_result_path: None,
        blocker_code: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_active_target: Some(dispatch_target.clone()),
        downstream_dispatch_last_target: Some(dispatch_target),
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    };
    super::refresh_downstream_dispatch_preview(
        store,
        &role_selection,
        &run_graph_bootstrap,
        &mut dispatch_receipt,
    )
    .await?;
    let taskflow_handoff_plan = super::build_taskflow_handoff_plan(&role_selection);
    let ctx = super::RuntimeDispatchPacketContext::new(
        store.root(),
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    );
    let dispatch_packet_path = super::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    store
        .record_run_graph_dispatch_receipt(&dispatch_receipt)
        .await
        .map_err(|error| {
            format!("Failed to record recovered run-graph dispatch receipt: {error}")
        })?;
    super::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &status,
        "consume_continue_missing_first_receipt_recovery",
    )
    .await?;
    let packet = read_dispatch_packet(&dispatch_packet_path)?;
    Ok(Some(build_resume_inputs(
        dispatch_receipt,
        dispatch_packet_path,
        packet,
        role_selection,
    )))
}

fn dispatch_receipt_retry_eligible(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    dispatch_receipt.dispatch_kind == "agent_lane"
        && dispatch_receipt.dispatch_status == "blocked"
        && matches!(
            dispatch_receipt.blocker_code.as_deref(),
            Some("configured_backend_dispatch_failed" | "timeout_without_takeover_authority")
        )
        && dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
}

fn retry_backend_for_dispatch_receipt(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    let route = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )?;
    let fallback = crate::taskflow_routing::fallback_executor_backend_from_route(route)?;
    if dispatch_receipt.selected_backend.as_deref() == Some(fallback.as_str()) {
        return None;
    }
    Some(fallback)
}

fn dispatch_receipt_primary_rebind_eligible(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
        || dispatch_receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return false;
    }
    let Some(route) = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    ) else {
        return false;
    };
    let Some(primary_backend) = crate::taskflow_routing::selected_backend_from_execution_plan_route(
        &role_selection.execution_plan,
        route,
    ) else {
        return false;
    };
    let Some(fallback_backend) =
        crate::taskflow_routing::fallback_executor_backend_from_route(route)
    else {
        return false;
    };
    dispatch_receipt.selected_backend.as_deref() == Some(fallback_backend.as_str())
        && primary_backend != fallback_backend
}

fn dispatch_receipt_internal_retry_eligible(
    project_root: &std::path::Path,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
        || dispatch_receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return false;
    }
    let overlay = match super::load_project_overlay_yaml_for_root(project_root) {
        Ok(overlay) => overlay,
        Err(_) => return false,
    };
    let (selected_cli_system, selected_cli_entry) =
        super::selected_host_cli_system_for_runtime_dispatch(&overlay);
    if selected_cli_system != "codex" {
        return false;
    }
    let execution_class = selected_cli_entry
        .as_ref()
        .and_then(|entry| super::yaml_lookup(entry, &["execution_class"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if execution_class != "internal" {
        return false;
    }
    let carriers = crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(
        selected_cli_entry.as_ref(),
    );
    [
        dispatch_receipt.selected_backend.as_deref(),
        dispatch_receipt.activation_agent_type.as_deref(),
        Some(role_selection.selected_role.as_str()),
    ]
    .iter()
    .flatten()
    .any(|backend_id| {
        carriers
            .iter()
            .any(|row| row["role_id"].as_str() == Some(*backend_id))
    })
}

fn primary_backend_for_dispatch_receipt(
    project_root: &std::path::Path,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if !dispatch_receipt_primary_rebind_eligible(role_selection, dispatch_receipt) {
        return None;
    }
    let route = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )?;
    let primary_backend = crate::taskflow_routing::selected_backend_from_execution_plan_route(
        &role_selection.execution_plan,
        route,
    )?;
    let overlay = super::load_project_overlay_yaml_for_root(project_root).ok()?;
    let (selected_cli_system, selected_cli_entry) =
        super::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preflight = crate::status_surface_external_cli::external_cli_preflight_summary(
        &overlay,
        &selected_cli_system,
        selected_cli_entry.as_ref(),
    );
    let carrier_ready = preflight["carrier_readiness"]["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|carrier| {
            carrier["backend_id"].as_str() == Some(primary_backend.as_str())
                && matches!(
                    carrier["status"].as_str(),
                    Some("carrier_ready" | "carrier_ready_with_override")
                )
        });
    carrier_ready.then_some(primary_backend)
}

fn decode_role_selection_from_packet(
    packet: &serde_json::Value,
    packet_kind: &str,
) -> Result<super::RuntimeConsumptionLaneSelection, String> {
    serde_json::from_value(
        packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| format!("Failed to decode role_selection from {packet_kind}: {error}"))
}

async fn resume_inputs_from_downstream_packet(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    packet_path: &str,
) -> Result<ResumeInputs, String> {
    let packet = read_dispatch_packet(packet_path)?;
    let run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?;
    if let Some(requested_run_id) = requested_run_id {
        if requested_run_id != run_id {
            return Err(format!(
                "Requested run_id `{requested_run_id}` does not match persisted downstream dispatch packet run_id `{run_id}`"
            ));
        }
    }
    let root_receipt = match store.run_graph_dispatch_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => return Err(missing_dispatch_receipt_error(run_id)),
        Err(error) => {
            return Err(format!(
                "Failed to read persisted run-graph dispatch receipt: {error}"
            ));
        }
    };
    validate_receipt_packet_pair(
        &root_receipt,
        &packet,
        packet_path,
        "downstream dispatch packet",
    )?;
    validate_run_graph_resume_state_for_downstream_packet(store, run_id).await?;
    let role_selection = decode_role_selection_from_packet(&packet, "downstream dispatch packet")?;
    let dispatch_target = packet
        .get("downstream_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Persisted downstream dispatch packet is missing downstream_dispatch_target".to_string()
        })?;
    validate_completed_run_downstream_resume_candidate(
        store,
        run_id,
        dispatch_target,
        "downstream dispatch packet",
    )
    .await?;
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, dispatch_target);
    let selected_backend = super::downstream_selected_backend(
        &role_selection,
        dispatch_target,
        activation_agent_type.as_deref(),
        root_receipt.selected_backend.as_deref(),
    )
    .filter(|value| !value.is_empty());
    let downstream_dispatch_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let dispatch_command = packet
        .get("downstream_dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_note = packet
        .get("downstream_dispatch_note")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_blockers = packet
        .get("downstream_dispatch_blockers")
        .map(|value| {
            canonical_resume_string_array_entries(value).ok_or_else(|| {
                "Persisted downstream dispatch packet has noncanonical downstream_dispatch_blockers"
                    .to_string()
            })
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(error) = super::downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let downstream_dispatch_status =
        if downstream_dispatch_ready && downstream_dispatch_blockers.is_empty() {
            Some("packet_ready".to_string())
        } else {
            packet
                .get("downstream_dispatch_status")
                .and_then(serde_json::Value::as_str)
                .map(|status| canonical_resume_dispatch_status(Some(status)))
                .map(str::to_string)
        };
    let downstream_dispatch_result_path = packet
        .get("downstream_dispatch_result_path")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if let Some(error) = resume_packet_ready_blocker_parity_error(
        downstream_dispatch_status.as_deref(),
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let supersedes_receipt_id = packet
        .get("downstream_supersedes_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let exception_path_receipt_id = packet
        .get("downstream_exception_path_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let parsed_downstream_lane_status = packet
        .get("downstream_lane_status")
        .and_then(serde_json::Value::as_str)
        .and_then(canonical_resume_lane_status);
    let missing_lane_evidence_blocker = super::missing_downstream_lane_evidence_blocker(
        parsed_downstream_lane_status,
        supersedes_receipt_id.as_deref(),
        exception_path_receipt_id.as_deref(),
    );
    if let Some(code) = missing_lane_evidence_blocker {
        let _ = super::blocker_code_value(code);
        return Err(match code {
            super::BlockerCode::ExceptionPathMissing => {
                "Persisted downstream dispatch packet is missing downstream_exception_path_receipt_id"
                    .to_string()
            }
            super::BlockerCode::MissingLaneReceipt => {
                "Persisted downstream dispatch packet is missing downstream_supersedes_receipt_id"
                    .to_string()
            }
            _ => "Persisted downstream dispatch packet is missing required lane evidence"
                .to_string(),
        });
    }
    let closure_completed = matches!(
        parsed_downstream_lane_status,
        Some(super::LaneStatus::LaneCompleted)
    ) && downstream_dispatch_status.as_deref() == Some("executed");
    let dispatch_status = if closure_completed {
        "executed".to_string()
    } else {
        downstream_dispatch_status
            .as_deref()
            .unwrap_or("blocked")
            .to_string()
    };
    let mut derived_lane_status = super::derive_lane_status(
        &dispatch_status,
        supersedes_receipt_id.as_deref(),
        exception_path_receipt_id.as_deref(),
    );
    if closure_completed {
        derived_lane_status = super::LaneStatus::LaneCompleted;
    }
    if let Some(packet_lane_status) = parsed_downstream_lane_status {
        if !lane_status_pair_is_resume_compatible(packet_lane_status, derived_lane_status) {
            return Err(format!(
                "Persisted downstream dispatch packet lane_status `{}` conflicts with derived lane_status `{}` from downstream lane evidence",
                packet_lane_status.as_str(),
                derived_lane_status.as_str()
            ));
        }
    }
    let receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.to_string(),
        dispatch_target: dispatch_target.to_string(),
        dispatch_status: dispatch_status.clone(),
        lane_status: derived_lane_status.as_str().to_string(),
        supersedes_receipt_id,
        exception_path_receipt_id,
        dispatch_kind,
        dispatch_surface,
        dispatch_command,
        dispatch_packet_path: Some(packet_path.to_string()),
        dispatch_result_path: None,
        blocker_code: if missing_lane_evidence_blocker
            == Some(super::BlockerCode::ExceptionPathMissing)
        {
            super::blocker_code_value(super::BlockerCode::ExceptionPathMissing)
        } else if missing_lane_evidence_blocker == Some(super::BlockerCode::MissingLaneReceipt) {
            super::blocker_code_value(super::BlockerCode::MissingLaneReceipt)
        } else if dispatch_status == "blocked" {
            super::blocker_code_value(super::BlockerCode::MissingPacket)
        } else {
            None
        },
        downstream_dispatch_target: packet
            .get("downstream_dispatch_target")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_command: packet
            .get("downstream_dispatch_command")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
        downstream_dispatch_packet_path: Some(packet_path.to_string()),
        downstream_dispatch_status,
        downstream_dispatch_result_path,
        downstream_dispatch_trace_path: packet
            .get("downstream_dispatch_trace_path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_executed_count: packet
            .get("downstream_dispatch_executed_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        downstream_dispatch_active_target: packet
            .get("downstream_dispatch_active_target")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_last_target: packet
            .get("downstream_dispatch_last_target")
            .or_else(|| packet.get("downstream_dispatch_target"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    };
    Ok(build_resume_inputs(
        receipt,
        packet_path.to_string(),
        packet,
        role_selection,
    ))
}

async fn maybe_resume_inputs_from_ready_downstream_packet(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let packet = read_dispatch_packet(packet_path)?;
    let packet_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !packet_ready {
        return Ok(None);
    }
    resume_inputs_from_downstream_packet(store, requested_run_id, packet_path)
        .await
        .map(Some)
}

fn prefer_ready_downstream_packet_over_active_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if !receipt.downstream_dispatch_ready {
        return false;
    }
    let ready_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let active_target = receipt
        .downstream_dispatch_active_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    matches!((ready_target, active_target), (Some(ready), Some(active)) if ready != active)
}

fn downstream_result_packet_path(result: &serde_json::Value) -> Option<String> {
    result
        .get("dispatch_packet_path")
        .or_else(|| result.get("source_dispatch_packet_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn read_downstream_dispatch_result(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted downstream dispatch result: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted downstream dispatch result: {error}"))
}

async fn maybe_resume_inputs_from_active_downstream_result(
    _store: &super::StateStore,
    requested_run_id: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    let Some(active_target) = receipt
        .downstream_dispatch_active_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(result_path) = receipt
        .downstream_dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let result = read_downstream_dispatch_result(result_path)?;
    let Some(packet_path) = downstream_result_packet_path(&result) else {
        return Ok(None);
    };
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "downstream dispatch packet")?;
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?;
    validate_completed_run_downstream_resume_candidate(
        _store,
        packet_run_id,
        active_target,
        "active downstream dispatch result",
    )
    .await?;
    if let Some(requested_run_id) = requested_run_id {
        if requested_run_id != packet_run_id {
            return Err(format!(
                "Requested run_id `{requested_run_id}` does not match persisted downstream dispatch packet run_id `{packet_run_id}`"
            ));
        }
    }
    let (dispatch_kind, derived_dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, active_target);
    let execution_state = result
        .get("execution_state")
        .and_then(serde_json::Value::as_str)
        .map(|value| canonical_resume_dispatch_status(Some(value)))
        .unwrap_or("blocked");
    let dispatch_surface = result
        .get("surface")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or(derived_dispatch_surface);
    let dispatch_command = result
        .get("activation_command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            packet
                .get("downstream_dispatch_command")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let blocker_code = result
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let selected_backend = result
        .get("backend_dispatch")
        .and_then(|value| value.get("backend_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            packet
                .get("selected_backend")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let stale_downstream_state = execution_state == "executed";
    let synthetic_receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: packet_run_id.to_string(),
        dispatch_target: active_target.to_string(),
        dispatch_status: execution_state.to_string(),
        lane_status: super::derive_lane_status(
            execution_state,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command,
        dispatch_packet_path: Some(packet_path.clone()),
        dispatch_result_path: Some(result_path.to_string()),
        blocker_code,
        downstream_dispatch_target: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_target.clone()
        },
        downstream_dispatch_command: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_command.clone()
        },
        downstream_dispatch_note: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_note.clone()
        },
        downstream_dispatch_ready: if stale_downstream_state {
            false
        } else {
            receipt.downstream_dispatch_ready
        },
        downstream_dispatch_blockers: if stale_downstream_state {
            Vec::new()
        } else {
            receipt.downstream_dispatch_blockers.clone()
        },
        downstream_dispatch_packet_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_packet_path.clone()
        },
        downstream_dispatch_status: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_status.clone()
        },
        downstream_dispatch_result_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_result_path.clone()
        },
        downstream_dispatch_trace_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_trace_path.clone()
        },
        downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
        downstream_dispatch_active_target: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_active_target.clone()
        },
        downstream_dispatch_last_target: if stale_downstream_state {
            Some(active_target.to_string())
        } else {
            receipt.downstream_dispatch_last_target.clone()
        },
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    };
    Ok(Some(build_resume_inputs(
        synthetic_receipt,
        packet_path,
        packet,
        role_selection,
    )))
}

async fn sync_run_graph_after_resumed_execution(
    store: &super::StateStore,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if dispatch_receipt.dispatch_status != "executed" {
        return Ok(());
    }
    let Some(run_id) = run_graph_bootstrap
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph state for resumed execution: {error}")
    })?;
    let executed_status =
        super::apply_first_handoff_execution_to_run_graph_status(&status, dispatch_receipt);
    store
        .record_run_graph_status(&executed_status)
        .await
        .map_err(|error| format!("Failed to record resumed executed run-graph status: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &executed_status,
        "resume_execution",
    )
    .await
    .map_err(|error| {
        format!("Failed to synchronize continuation binding after resumed execution: {error}")
    })?;
    Ok(())
}

async fn resolve_default_resume_run_id(store: &super::StateStore) -> Result<String, String> {
    let latest_status = store
        .latest_run_graph_status()
        .await
        .map_err(|error| format!("Failed to read latest persisted run-graph state: {error}"))?;
    let Some(status) = latest_status else {
        return Err("No persisted run-graph dispatch receipt is available".to_string());
    };
    let explicit_continuation_binding = store
        .run_graph_continuation_binding(&status.run_id)
        .await
        .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?;
    let latest_run_graph_recovery = store
        .latest_run_graph_recovery_summary()
        .await
        .map_err(|error| format!("Failed to read latest run graph recovery summary: {error}"))?;
    let latest_run_graph_dispatch_receipt =
        match store.latest_run_graph_dispatch_receipt_summary().await {
            Ok(summary) => summary,
            Err(_) => None,
        };
    let continuation_binding_evidence_ambiguous = latest_run_graph_dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            crate::state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(receipt)
                || crate::state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                    Some(status.run_id.as_str()),
                    Some(receipt.run_id.as_str()),
                )
        });
    let continuation_binding =
        crate::continuation_binding_summary::build_continuation_binding_summary(
            explicit_continuation_binding.as_ref(),
            Some(&status),
            latest_run_graph_recovery.as_ref(),
            latest_run_graph_dispatch_receipt.as_ref(),
            continuation_binding_evidence_ambiguous,
        );
    if continuation_binding["status"] != "bound" {
        return Err(format!(
            "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
            status.run_id, status.run_id, status.run_id
        ));
    }
    if status.status == "completed"
        && continuation_binding["active_bounded_unit"]["kind"] != "downstream_dispatch_target"
    {
        let unit_kind = continuation_binding["active_bounded_unit"]["kind"]
            .as_str()
            .unwrap_or("unknown");
        return Err(format!(
            "Latest continuation binding for run `{}` points to `{unit_kind}`, which is not resumeable through default `vida taskflow consume continue --json`. Pass `--run-id {}` to refresh the completed run explicitly or bind/shape the next bounded unit before continuing.",
            status.run_id, status.run_id
        ));
    }
    Ok(status.run_id)
}

async fn resolve_runtime_consumption_resume_inputs_for_run_id(
    store: &super::StateStore,
    run_id: &str,
) -> Result<ResumeInputs, String> {
    let receipt = match store.run_graph_dispatch_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => match recover_missing_first_dispatch_receipt(store, run_id).await? {
            Some(inputs) => return Ok(inputs),
            None => return Err(missing_dispatch_receipt_error(run_id)),
        },
        Err(error) => {
            return Err(format!(
                "Failed to read persisted run-graph dispatch receipt: {error}"
            ));
        }
    };
    validate_explicit_task_graph_binding_lineage_for_resume(store, run_id, &receipt).await?;
    let explicit_downstream_target =
        completed_run_explicit_downstream_target_for_resume(store, run_id).await?;
    if let Some(bound_target) = explicit_downstream_target.as_deref() {
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt).await?
        {
            if resume.dispatch_receipt.dispatch_target == bound_target {
                return Ok(resume);
            }
            return Err(format!(
                "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream packet lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                resume.dispatch_receipt.dispatch_target
            ));
        }
        if let Some(resume) =
            maybe_resume_inputs_from_active_downstream_result(store, Some(run_id), &receipt).await?
        {
            if resume.dispatch_receipt.dispatch_target == bound_target {
                return Ok(resume);
            }
            return Err(format!(
                "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream result lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                resume.dispatch_receipt.dispatch_target
            ));
        }
        return Err(missing_explicit_downstream_resume_evidence_error(
            run_id,
            bound_target,
        ));
    } else {
        if prefer_ready_downstream_packet_over_active_result(&receipt) {
            if let Some(resume) =
                maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt)
                    .await?
            {
                return Ok(resume);
            }
        }
        if let Some(resume) =
            maybe_resume_inputs_from_active_downstream_result(store, Some(run_id), &receipt).await?
        {
            return Ok(resume);
        }
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt).await?
        {
            return Ok(resume);
        }
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
    validate_run_graph_resume_state(store, run_id).await?;
    Ok(build_resume_inputs(
        receipt,
        packet_path,
        packet,
        role_selection,
    ))
}

pub(crate) async fn resolve_runtime_consumption_resume_inputs(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    requested_dispatch_packet_path: Option<&str>,
    requested_downstream_packet_path: Option<&str>,
) -> Result<ResumeInputs, String> {
    let dispatch_packet = if let Some(packet_path) = requested_dispatch_packet_path {
        let packet = read_dispatch_packet(packet_path)?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        let run_id = packet
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Persisted dispatch packet is missing run_id".to_string())?;
        if let Some(requested_run_id) = requested_run_id {
            if requested_run_id != run_id {
                return Err(format!(
                    "Requested run_id `{requested_run_id}` does not match persisted dispatch packet run_id `{run_id}`"
                ));
            }
        }
        let receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => return Err(missing_dispatch_receipt_error(run_id)),
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ));
            }
        };
        validate_receipt_packet_pair(&receipt, &packet, packet_path, "dispatch packet")?;
        validate_run_graph_resume_state(store, run_id).await?;
        build_resume_inputs(receipt, packet_path.to_string(), packet, role_selection)
    } else if let Some(packet_path) = requested_downstream_packet_path {
        return resume_inputs_from_downstream_packet(store, requested_run_id, packet_path).await;
    } else if let Some(run_id) = requested_run_id {
        return resolve_runtime_consumption_resume_inputs_for_run_id(store, run_id).await;
    } else {
        let run_id = resolve_default_resume_run_id(store).await?;
        return resolve_runtime_consumption_resume_inputs_for_run_id(store, &run_id).await;
    };
    Ok(dispatch_packet)
}

fn canonical_resume_dispatch_status(status: Option<&str>) -> &'static str {
    match status.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "executed" => "executed",
        Some(value) if value == "blocked" => "blocked",
        Some(value) if value == "routed" => "routed",
        Some(value) if value == "packet_ready" => "packet_ready",
        _ => "blocked",
    }
}

fn canonical_resume_lane_status(status: &str) -> Option<super::LaneStatus> {
    match status.trim().to_ascii_lowercase().as_str() {
        "packet_ready" => Some(super::LaneStatus::PacketReady),
        "lane_open" => Some(super::LaneStatus::LaneOpen),
        "lane_running" => Some(super::LaneStatus::LaneRunning),
        "lane_blocked" => Some(super::LaneStatus::LaneBlocked),
        "lane_completed" => Some(super::LaneStatus::LaneCompleted),
        "lane_superseded" => Some(super::LaneStatus::LaneSuperseded),
        "lane_exception_recorded" => Some(super::LaneStatus::LaneExceptionRecorded),
        "lane_exception_takeover" => Some(super::LaneStatus::LaneExceptionTakeover),
        _ => None,
    }
}

fn canonical_resume_string_array_entries(value: &serde_json::Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

fn resume_packet_ready_blocker_parity_error(
    downstream_dispatch_status: Option<&str>,
    downstream_dispatch_blockers: &[String],
) -> Option<String> {
    if downstream_dispatch_status == Some("packet_ready")
        && !downstream_dispatch_blockers.is_empty()
    {
        return Some(
            "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                .to_string(),
        );
    }
    None
}

fn should_refresh_resumed_downstream_preview(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == "executed"
        && (!receipt.downstream_dispatch_ready || !receipt.downstream_dispatch_blockers.is_empty())
}

fn prepare_explicit_resume_retry_artifact(
    project_root: Option<&std::path::Path>,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if dispatch_receipt_retry_eligible(dispatch_receipt) {
        if let Some(fallback_backend) =
            retry_backend_for_dispatch_receipt(role_selection, dispatch_receipt)
        {
            dispatch_receipt.selected_backend = Some(fallback_backend);
        }
        dispatch_receipt.dispatch_status = "packet_ready".to_string();
        dispatch_receipt.lane_status = super::LaneStatus::PacketReady.as_str().to_string();
        dispatch_receipt.blocker_code = None;
        return true;
    }

    let Some(project_root) = project_root else {
        return false;
    };
    if dispatch_receipt_internal_retry_eligible(project_root, role_selection, dispatch_receipt) {
        dispatch_receipt.dispatch_status = "packet_ready".to_string();
        dispatch_receipt.lane_status = super::LaneStatus::PacketReady.as_str().to_string();
        dispatch_receipt.blocker_code = None;
        return true;
    }
    if let Some(primary_backend) =
        primary_backend_for_dispatch_receipt(project_root, role_selection, dispatch_receipt)
    {
        dispatch_receipt.selected_backend = Some(primary_backend);
        dispatch_receipt.dispatch_status = "packet_ready".to_string();
        dispatch_receipt.lane_status = super::LaneStatus::PacketReady.as_str().to_string();
        dispatch_receipt.blocker_code = None;
        return true;
    }
    if let Some(fallback_backend) = super::fallback_backend_for_blocked_primary_dispatch_receipt(
        project_root,
        role_selection,
        dispatch_receipt,
    ) {
        dispatch_receipt.selected_backend = Some(fallback_backend);
        dispatch_receipt.dispatch_status = "packet_ready".to_string();
        dispatch_receipt.lane_status = super::LaneStatus::PacketReady.as_str().to_string();
        dispatch_receipt.blocker_code = None;
        return true;
    }
    false
}

type TaskflowConsumeContinueArgs = (bool, Option<String>, Option<String>, Option<String>);

pub(crate) fn parse_taskflow_consume_continue_args(
    args: &[String],
) -> Result<TaskflowConsumeContinueArgs, String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut dispatch_packet_path = None;
    let mut downstream_packet_path = None;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--dispatch-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                dispatch_packet_path = Some(value.clone());
                index += 2;
            }
            "--downstream-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                downstream_packet_path = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]"
                ));
            }
        }
    }
    if dispatch_packet_path.is_some() && downstream_packet_path.is_some() {
        return Err(
            "Use only one packet source: --dispatch-packet <path> or --downstream-packet <path>"
                .to_string(),
        );
    }
    Ok((
        as_json,
        run_id,
        dispatch_packet_path,
        downstream_packet_path,
    ))
}

pub(crate) fn parse_taskflow_consume_advance_args(
    args: &[String],
) -> Result<(bool, Option<String>, usize), String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut max_rounds = 8usize;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--max-rounds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                max_rounds = value
                    .parse::<usize>()
                    .map_err(|_| "Expected a positive integer for --max-rounds".to_string())?;
                if max_rounds == 0 {
                    return Err("--max-rounds must be greater than zero".to_string());
                }
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                ));
            }
        }
    }
    Ok((as_json, run_id, max_rounds))
}

pub(crate) async fn run_taskflow_consume_resume_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    requested_dispatch_packet_path: Option<String>,
    requested_downstream_packet_path: Option<String>,
    surface_name: &str,
    emit_output: bool,
) -> ExitCode {
    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => {
            let mut dispatch_receipt;
            let dispatch_packet_path;
            let role_selection;
            let run_graph_bootstrap;
            let state_root = store.root().to_path_buf();
            match resolve_runtime_consumption_resume_inputs(
                &store,
                requested_run_id.as_deref(),
                requested_dispatch_packet_path.as_deref(),
                requested_downstream_packet_path.as_deref(),
            )
            .await
            {
                Ok(ResumeInputs {
                    dispatch_receipt: receipt,
                    dispatch_packet_path: packet_path,
                    role_selection: selection,
                    run_graph_bootstrap: bootstrap,
                }) => {
                    dispatch_receipt = receipt;
                    dispatch_packet_path = packet_path;
                    role_selection = selection;
                    run_graph_bootstrap = bootstrap;
                }
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            }
            if let Err(error) =
                super::try_bridge_bounded_implementer_completion_to_downstream_receipt(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                )
                .await
            {
                eprintln!(
                    "Failed to bridge bounded implementer completion into downstream receipt: {error}"
                );
                return ExitCode::from(1);
            }
            let project_root =
                super::taskflow_task_bridge::infer_project_root_from_state_root(store.root());
            let prepared_retry_artifact = prepare_explicit_resume_retry_artifact(
                project_root.as_deref(),
                &role_selection,
                &mut dispatch_receipt,
            );
            if prepared_retry_artifact {
                if should_refresh_resumed_downstream_preview(&dispatch_receipt) {
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
                if let Err(error) = sync_run_graph_after_resumed_execution(
                    &store,
                    &run_graph_bootstrap,
                    &dispatch_receipt,
                )
                .await
                {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                if dispatch_receipt.dispatch_kind == "agent_lane" {
                    dispatch_receipt.selected_backend =
                        super::canonical_selected_backend_for_receipt(
                            &role_selection,
                            &dispatch_receipt,
                        );
                }
                if let Err(error) = store
                    .record_run_graph_dispatch_receipt(&dispatch_receipt)
                    .await
                {
                    eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                    return ExitCode::from(1);
                }
                return match emit_runtime_consumption_resume_json(
                    &store,
                    surface_name,
                    &dispatch_packet_path,
                    &dispatch_receipt,
                    &role_selection,
                    requested_run_id.as_deref(),
                    emit_output,
                    as_json,
                )
                .await
                {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                };
            }
            if dispatch_receipt.dispatch_status == "packet_ready" {
                dispatch_receipt.dispatch_status = "routed".to_string();
                dispatch_receipt.lane_status = super::derive_lane_status(
                    &dispatch_receipt.dispatch_status,
                    dispatch_receipt.supersedes_receipt_id.as_deref(),
                    dispatch_receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string();
                dispatch_receipt.blocker_code = None;
            }
            if dispatch_receipt.dispatch_status == "routed" {
                let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                    != "taskflow_pack"
                    || super::taskflow_task_bridge::infer_project_root_from_state_root(&state_root)
                        .is_some();
                if allow_taskflow_pack_execution {
                    drop(store);
                    if let Err(error) = super::execute_and_record_dispatch_receipt(
                        &state_root,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to execute resumed runtime dispatch handoff: {error}");
                        return ExitCode::from(1);
                    }
                    let store = match super::StateStore::open_existing(state_root.clone()).await {
                        Ok(store) => store,
                        Err(error) => {
                            eprintln!(
                                "Failed to reopen authoritative state store after resumed runtime dispatch: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                    drop(store);
                }
            } else {
                if should_refresh_resumed_downstream_preview(&dispatch_receipt) {
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
                if let Err(error) = sync_run_graph_after_resumed_execution(
                    &store,
                    &run_graph_bootstrap,
                    &dispatch_receipt,
                )
                .await
                {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                drop(store);
            }
            if let Err(error) = super::execute_downstream_dispatch_chain(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            let store = match super::StateStore::open_existing(state_root).await {
                Ok(store) => store,
                Err(error) => {
                    eprintln!(
                        "Failed to reopen authoritative state store before resumed receipt persistence: {error}"
                    );
                    return ExitCode::from(1);
                }
            };
            if dispatch_receipt.dispatch_kind == "agent_lane" {
                dispatch_receipt.selected_backend = super::canonical_selected_backend_for_receipt(
                    &role_selection,
                    &dispatch_receipt,
                );
            }
            if let Err(error) = store
                .record_run_graph_dispatch_receipt(&dispatch_receipt)
                .await
            {
                eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                return ExitCode::from(1);
            }
            match emit_runtime_consumption_resume_json(
                &store,
                surface_name,
                &dispatch_packet_path,
                &dispatch_receipt,
                &role_selection,
                requested_run_id.as_deref(),
                emit_output,
                as_json,
            )
            .await
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_taskflow_consume_advance_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    max_rounds: usize,
) -> ExitCode {
    let mut rounds = 0usize;
    let mut last_result: Option<(String, crate::state_store::RunGraphDispatchReceipt, String)> =
        None;

    while rounds < max_rounds {
        let before_status = match super::StateStore::open_existing(state_dir.clone()).await {
            Ok(store) => match resolve_runtime_consumption_resume_inputs(
                &store,
                requested_run_id.as_deref(),
                None,
                None,
            )
            .await
            {
                Ok(ResumeInputs {
                    dispatch_receipt: receipt,
                    dispatch_packet_path: packet_path,
                    ..
                }) => Some((receipt, packet_path)),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let exit = run_taskflow_consume_resume_command(
            state_dir.clone(),
            true,
            requested_run_id.clone(),
            None,
            None,
            "vida taskflow consume advance",
            false,
        )
        .await;
        if exit != ExitCode::SUCCESS {
            return exit;
        }

        let store = match super::StateStore::open_existing(state_dir.clone()).await {
            Ok(store) => store,
            Err(error) => {
                eprintln!("Failed to reopen authoritative state store after advance: {error}");
                return ExitCode::from(1);
            }
        };
        let after_receipt = match store.latest_run_graph_dispatch_receipt().await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                eprintln!("No persisted run-graph dispatch receipt is available after advance");
                return ExitCode::from(1);
            }
            Err(error) => {
                eprintln!(
                    "Failed to read persisted run-graph dispatch receipt after advance: {error}"
                );
                return ExitCode::from(1);
            }
        };
        let after_packet_path = after_receipt
            .dispatch_packet_path
            .clone()
            .or_else(|| after_receipt.downstream_dispatch_packet_path.clone())
            .unwrap_or_else(|| "none".to_string());
        let snapshot_path =
            match super::latest_final_runtime_consumption_snapshot_path(store.root()) {
                Ok(Some(path)) => path,
                Ok(None) => "none".to_string(),
                Err(_) => "none".to_string(),
            };
        last_result = Some((
            after_packet_path.clone(),
            after_receipt.clone(),
            snapshot_path,
        ));
        rounds += 1;

        let progressed = match before_status {
            Some((before_receipt, before_packet_path)) => {
                before_packet_path != after_packet_path
                    || before_receipt.dispatch_status != after_receipt.dispatch_status
                    || before_receipt.downstream_dispatch_target
                        != after_receipt.downstream_dispatch_target
                    || before_receipt.downstream_dispatch_executed_count
                        != after_receipt.downstream_dispatch_executed_count
            }
            None => true,
        };

        let has_more_ready_work = after_receipt.downstream_dispatch_ready
            || (after_receipt.dispatch_status == "routed"
                && (after_receipt.dispatch_kind != "taskflow_pack"
                    || super::taskflow_task_bridge::infer_project_root_from_state_root(
                        store.root(),
                    )
                    .is_some()));
        if !progressed || !has_more_ready_work {
            break;
        }
    }

    let Some((source_dispatch_packet_path, dispatch_receipt, snapshot_path)) = last_result else {
        eprintln!("No advance step was executed");
        return ExitCode::from(1);
    };

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida taskflow consume advance",
                "source_run_id": dispatch_receipt.run_id,
                "source_dispatch_packet_path": source_dispatch_packet_path,
                "dispatch_receipt": dispatch_receipt,
                "snapshot_path": snapshot_path,
                "rounds_executed": rounds,
            }))
            .expect("advance should render as json")
        );
    } else {
        super::print_surface_header(super::RenderMode::Plain, "vida taskflow consume advance");
        super::print_surface_line(
            super::RenderMode::Plain,
            "source run",
            &dispatch_receipt.run_id,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "source packet",
            &source_dispatch_packet_path,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "rounds executed",
            &rounds.to_string(),
        );
        super::print_surface_line(super::RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        build_failure_control_evidence, canonical_resume_dispatch_status,
        canonical_resume_lane_status, canonical_resume_string_array_entries,
        dispatch_receipt_internal_retry_eligible, dispatch_receipt_primary_rebind_eligible,
        dispatch_receipt_retry_eligible, normalize_runtime_dispatch_packet,
        persisted_dispatch_packet_lineage_task_id, primary_backend_for_dispatch_receipt,
        read_dispatch_packet, recover_missing_first_dispatch_receipt,
        resolve_runtime_consumption_resume_inputs, resume_from_persisted_final_snapshot,
        resume_packet_ready_blocker_parity_error, retry_backend_for_dispatch_receipt,
        runtime_consumption_resume_blocker_code,
        runtime_consumption_snapshot_has_failure_control_evidence,
        should_refresh_resumed_downstream_preview, validate_run_graph_resume_state,
        validate_run_graph_resume_state_for_downstream_packet,
        DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS,
    };
    use crate::downstream_dispatch_ready_blocker_parity_error;
    use crate::StateStore;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn configured_backend_dispatch_failure_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn timeout_without_takeover_authority_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-timeout-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:qwen_cli".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn retry_backend_prefers_route_fallback_backend_after_external_failure() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "subagents": "qwen_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            retry_backend_for_dispatch_receipt(&role_selection, &receipt).as_deref(),
            Some("internal_subagents")
        );
    }

    #[test]
    fn internal_activation_view_only_on_fallback_is_rebind_eligible() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_primary_rebind_eligible(
            &role_selection,
            &receipt
        ));
    }

    #[test]
    fn internal_activation_view_only_on_internal_codex_host_is_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.4
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn internal_activation_view_only_on_external_host_is_not_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-external-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      carriers:
        qwen-primary:
          default_runtime_role: worker
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-retry-external".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn primary_backend_rebind_prefers_ready_external_carrier() {
        let root = std::env::temp_dir().join(format!(
            "vida-primary-rebind-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: none
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            primary_backend_for_dispatch_receipt(&root, &role_selection, &receipt).as_deref(),
            Some("qwen_cli")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn primary_backend_rebind_stays_blocked_when_external_carrier_is_not_ready() {
        let root = std::env::temp_dir().join(format!(
            "vida-primary-rebind-blocked-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: file_present
          path: /tmp/vida-missing-qwen-auth
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            primary_backend_for_dispatch_receipt(&root, &role_selection, &receipt),
            None
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn blocked_primary_backend_prefers_route_fallback_before_dispatch_execution() {
        let root = std::env::temp_dir().join(format!(
            "vida-blocked-primary-fallback-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: file_present
          path: /tmp/vida-missing-qwen-auth
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked-primary".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            crate::runtime_dispatch_state::fallback_backend_for_blocked_primary_dispatch_receipt(
                &root,
                &role_selection,
                &receipt,
            )
            .as_deref(),
            Some("internal_subagents")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn canonical_resume_dispatch_status_preserves_release1_vocabulary() {
        assert_eq!(
            canonical_resume_dispatch_status(Some("executed")),
            "executed"
        );
        assert_eq!(canonical_resume_dispatch_status(Some("routed")), "routed");
        assert_eq!(
            canonical_resume_dispatch_status(Some("packet_ready")),
            "packet_ready"
        );
        assert_eq!(canonical_resume_dispatch_status(Some("blocked")), "blocked");
    }

    #[test]
    fn canonical_resume_dispatch_status_fails_closed_for_unknown_or_drifted_values() {
        assert_eq!(canonical_resume_dispatch_status(Some("block")), "blocked");
        assert_eq!(canonical_resume_dispatch_status(Some("unknown")), "blocked");
        assert_eq!(
            canonical_resume_dispatch_status(Some(" packet_ready ")),
            "packet_ready"
        );
        assert_eq!(canonical_resume_dispatch_status(None), "blocked");
    }

    #[test]
    fn canonical_resume_dispatch_and_lane_status_normalize_case_and_whitespace_drift() {
        assert_eq!(
            canonical_resume_dispatch_status(Some("  PACKET_READY  ")),
            "packet_ready"
        );
        assert_eq!(
            canonical_resume_dispatch_status(Some("  BLOCKED  ")),
            "blocked"
        );
        assert_eq!(
            canonical_resume_lane_status("  LANE_COMPLETED  "),
            Some(crate::LaneStatus::LaneCompleted)
        );
        assert_eq!(
            canonical_resume_lane_status("  lane_open  "),
            Some(crate::LaneStatus::LaneOpen)
        );
        assert_eq!(canonical_resume_lane_status("lane_block"), None);
    }

    #[test]
    fn canonical_resume_string_array_entries_fail_closed_for_whitespace_only_entries() {
        assert_eq!(
            canonical_resume_string_array_entries(&serde_json::json!(["pending_lane_evidence"])),
            Some(vec!["pending_lane_evidence".to_string()])
        );
        assert_eq!(
            canonical_resume_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }

    #[test]
    fn resume_packet_ready_blocker_parity_fails_closed_for_drifted_blocker_evidence() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            resume_packet_ready_blocker_parity_error(Some("packet_ready"), &blockers),
            Some(
                "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                    .to_string()
            )
        );
        assert_eq!(
            resume_packet_ready_blocker_parity_error(Some("packet_ready"), &[]),
            None
        );
    }

    #[test]
    fn downstream_dispatch_ready_blocker_parity_fails_closed_for_drifted_blocker_evidence() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("ready"), &blockers),
            None
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("ready"), &[]),
            None
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("packet_ready"), &blockers),
            Some(
                "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                    .to_string()
            )
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("blocked"), &blockers),
            None
        );
    }

    #[test]
    fn downstream_dispatch_ready_guard_message_matches_main_surface() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            downstream_dispatch_ready_blocker_parity_error(true, &blockers),
            crate::downstream_dispatch_ready_blocker_parity_error(true, &blockers)
        );
    }

    #[test]
    fn should_refresh_resumed_downstream_preview_for_executed_receipt_with_stale_blockers() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-refresh".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/spec-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/spec-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some("vida task ensure".to_string()),
            downstream_dispatch_note: Some("stale blockers".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "pending_specification_evidence".to_string(),
                "pending_design_finalize".to_string(),
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some("/tmp/spec-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: Some("specification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-12T00:00:00Z".to_string(),
        };

        assert!(should_refresh_resumed_downstream_preview(&receipt));

        let mut settled = receipt.clone();
        settled.downstream_dispatch_ready = true;
        settled.downstream_dispatch_blockers.clear();
        assert!(!should_refresh_resumed_downstream_preview(&settled));

        let mut blocked = receipt.clone();
        blocked.dispatch_status = "blocked".to_string();
        assert!(!should_refresh_resumed_downstream_preview(&blocked));
    }

    #[tokio::test]
    async fn resolve_resume_inputs_clears_stale_downstream_state_for_executed_active_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-stale-executed-downstream-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-stale-executed-downstream";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = Some("verification".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-stale-executed-downstream-coach.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-stale-executed-downstream-coach.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "executed",
                "dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "middle"
                }
            })
            .to_string(),
        )
        .expect("write downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after implementer evidence, activate coach".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("executed".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from executed downstream result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "executed");
        assert!(!inputs.dispatch_receipt.downstream_dispatch_ready);
        assert!(inputs.dispatch_receipt.downstream_dispatch_target.is_none());
        assert!(inputs
            .dispatch_receipt
            .downstream_dispatch_active_target
            .is_none());
        assert_eq!(
            inputs
                .dispatch_receipt
                .downstream_dispatch_last_target
                .as_deref(),
            Some("coach")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_from_persisted_final_snapshot_detects_final_snapshot_evidence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-final-snapshot-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(StateStore::open(root.clone()))
            .expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-00Z.json");
        let operator_contracts = crate::build_release1_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-final-snapshot",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        let failure_control_evidence = build_failure_control_evidence(
            "run-final-snapshot",
            &snapshot_path.display().to_string(),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": "run-final-snapshot"
                    },
                    "release_admission": {},
                    "failure_control_evidence": failure_control_evidence.clone()
                },
                "failure_control_evidence": failure_control_evidence
            })
            .to_string(),
        )
        .expect("write final snapshot");

        assert!(resume_from_persisted_final_snapshot(&store).expect("runtime consumption summary"),);
        let snapshot_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read final snapshot"))
                .expect("parse final snapshot");
        assert!(runtime_consumption_snapshot_has_failure_control_evidence(
            &snapshot_json
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_from_persisted_final_snapshot_rejects_final_snapshot_without_failure_control_evidence(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-final-snapshot-missing-control-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(StateStore::open(root.clone()))
            .expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-01Z.json");
        let operator_contracts = crate::build_release1_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-final-snapshot",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": "run-final-snapshot"
                    },
                    "release_admission": {}
                }
            })
            .to_string(),
        )
        .expect("write final snapshot");

        let snapshot_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read final snapshot"))
                .expect("parse final snapshot");
        assert!(!runtime_consumption_snapshot_has_failure_control_evidence(
            &snapshot_json
        ));
        assert!(!resume_from_persisted_final_snapshot(&store).expect("runtime consumption summary"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_persisted_receipt_lineage_when_summary_rows_are_missing(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-receipt-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-02Z.json");
        let run_id = "run-receipt-lineage";
        let snapshot_path_string = snapshot_path.display().to_string();
        let operator_contracts = crate::build_release1_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path_string.clone(),
                "latest_run_graph_dispatch_receipt_id": run_id,
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        let failure_control_evidence =
            build_failure_control_evidence(run_id, &snapshot_path_string);
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": run_id
                    },
                    "release_admission": {},
                    "failure_control_evidence": failure_control_evidence.clone()
                },
                "failure_control_evidence": failure_control_evidence
            })
            .to_string(),
        )
        .expect("write final snapshot");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_run".to_string(),
            dispatch_surface: Some("vida taskflow consume continue".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("receipt lineage should allow resume validation");
        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect("receipt lineage should allow downstream resume validation");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_closure_complete_receipt_backed_lineage() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-complete";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "closure");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        store
            .record_run_graph_status(&status)
            .await
            .expect("persist closure-complete status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_run".to_string(),
            dispatch_surface: Some("vida taskflow consume continue".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("closure-complete receipt lineage should allow resume validation");
        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect("closure-complete receipt lineage should allow downstream resume validation");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_for_downstream_packet_accepts_receipt_backed_packet_ready(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-downstream-packet-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-downstream-packet-ready";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "dev-pack", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "dev-pack".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "dev_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-downstream-packet-ready.json");
        fs::write(&packet_path, "{}").expect("write downstream packet placeholder");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after implementer evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect(
                "receipt-backed downstream packet_ready should allow downstream resume validation",
            );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_accepts_runtime_style_downstream_packet_ready_without_result_path(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-runtime-downstream-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-runtime-downstream-ready";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-runtime-downstream-ready.json");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream packet".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": "vida taskflow consume continue --run-id run-runtime-downstream-ready --json",
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_dispatch_result_path": "/tmp/verification-result.json"
            }))
            .expect("encode downstream packet"),
        )
        .expect("write downstream packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("previous-verifier-packet".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("no additional downstream lane is required".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("runtime-style downstream packet_ready with result path should resume");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn completed_closure_bound_run_prefers_lawful_closure_packet_over_stale_blocked_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-bound-mixed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-mixed";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let downstream_packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&downstream_packet_dir).expect("create downstream packet dir");
        let closure_packet_path = downstream_packet_dir.join("run-closure-bound-mixed-closure.json");
        let stale_coach_packet_path =
            downstream_packet_dir.join("run-closure-bound-mixed-stale-coach.json");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream packet".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["closure".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &closure_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready"
            }))
            .expect("encode closure packet"),
        )
        .expect("write closure packet");
        fs::write(
            &stale_coach_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            }))
            .expect("encode stale coach packet"),
        )
        .expect("write stale coach packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_coach_result_path =
            result_dir.join("run-closure-bound-mixed-stale-coach.json");
        fs::write(
            &stale_coach_result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": stale_coach_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale coach result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "runtime reached closure; no additional downstream lane is required".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(closure_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(stale_coach_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": run_id,
                        "run_id": run_id,
                        "dispatch_target": "closure"
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "task closure rebound the next lawful bounded unit to closure"
                        .to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only".to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-14T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("closure-bound run should prefer lawful closure packet");

        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            closure_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_completed_closure_bound_run_prefers_lawful_closure_packet_over_stale_blocked_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-bound-mixed-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-mixed-lineage";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-closure-bound-mixed-lineage-coach.json");
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_result_path = result_dir.join("run-closure-bound-mixed-lineage-coach.json");
        fs::write(
            &stale_result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": stale_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale downstream result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("bounded closure handoff is required".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_closure_handoff".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(stale_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("senior".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist stale receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "dispatch_target": "closure",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_downstream".to_string(),
                    why_this_unit: "completed run is explicitly closure-bound".to_string(),
                    primary_path: "lawful_closure_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_downstream_bound"
                        .to_string(),
                    request_text: Some("continue by lawful closure".to_string()),
                    recorded_at: "2026-04-14T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit downstream binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale blocked coach lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicitly bound to downstream target `closure`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("stale downstream target `coach`"),
            "unexpected error: {error}"
        );

        let closure_packet_path = packet_dir.join("run-closure-bound-mixed-lineage-closure.json");
        fs::write(
            &closure_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "worker",
                    "request": "resume downstream packet",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "closure",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["closure"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    ),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_dispatch_result_path": "/tmp/closure-result.json"
            }))
            .expect("encode closure packet"),
        )
        .expect("write closure packet");

        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers = Vec::new();
        receipt.downstream_dispatch_packet_path =
            Some(closure_packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt with lawful closure packet");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("lawful closure packet_ready should win over stale blocked coach lineage");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            closure_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_taskflow_consume_resume_preserves_packet_ready_when_retry_artifact_is_prepared() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-prepared-retry-packet-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let state_dir = root.join(".vida/data/state");
        let store = StateStore::open(state_dir.clone())
            .await
            .expect("open store");

        let run_id = "run-prepared-retry-packet-ready";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = store.root().join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("run-prepared-retry-packet-ready.json");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "dispatch_status": "blocked",
                "lane_status": "lane_blocked",
                "dispatch_kind": "agent_lane",
                "dispatch_surface": "vida agent-init",
                "dispatch_command": "vida agent-init",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "junior",
                "recorded_at": "2026-04-13T00:00:00Z",
                "request_text": "continue development",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id,
                    "latest_status": {
                        "run_id": run_id,
                        "task_id": run_id
                    }
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);

        let exit = super::run_taskflow_consume_resume_command(
            state_dir.clone(),
            true,
            Some(run_id.to_string()),
            None,
            None,
            "vida taskflow consume continue",
            false,
        )
        .await;
        assert_eq!(exit, std::process::ExitCode::SUCCESS);

        let store = StateStore::open_existing(state_dir)
            .await
            .expect("reopen store");
        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should exist");
        assert_eq!(persisted.dispatch_status, "packet_ready");
        assert_eq!(persisted.lane_status, "packet_ready");
        assert_eq!(persisted.blocker_code, None);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn recover_missing_first_dispatch_receipt_for_active_implementer_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-first-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-first-receipt";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text: "continue development".to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let recovered = recover_missing_first_dispatch_receipt(&store, run_id)
            .await
            .expect("active implementer run should recover missing first receipt")
            .expect("active implementer run should synthesize receipt");

        assert_eq!(recovered.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(recovered.dispatch_receipt.dispatch_status, "executed");
        assert_eq!(recovered.dispatch_receipt.lane_status, "lane_running");
        assert_eq!(
            recovered
                .dispatch_receipt
                .activation_runtime_role
                .as_deref(),
            Some("worker")
        );
        assert_eq!(
            recovered.dispatch_receipt.activation_agent_type.as_deref(),
            Some("junior")
        );
        assert!(
            recovered.dispatch_receipt.dispatch_packet_path.is_some(),
            "recovered receipt should materialize a dispatch packet path"
        );
        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should be persisted");
        assert_eq!(persisted.dispatch_target, "implementer");
        assert_eq!(persisted.dispatch_status, "executed");
        assert_eq!(
            recovered.dispatch_packet_path,
            recovered
                .dispatch_receipt
                .dispatch_packet_path
                .clone()
                .expect("dispatch packet path should exist")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_without_run_id_recovers_missing_first_receipt_for_active_implementer_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-first-receipt-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-first-receipt-latest";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text: "continue development".to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, None, None, None)
            .await
            .expect("latest continuation path should recover missing first receipt");

        assert_eq!(resolved.dispatch_receipt.run_id, run_id);
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "executed");
        assert_eq!(resolved.dispatch_receipt.lane_status, "lane_running");
        assert!(
            resolved.dispatch_receipt.dispatch_packet_path.is_some(),
            "resolved receipt should materialize a dispatch packet path"
        );
        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should be persisted");
        assert_eq!(persisted.dispatch_target, "implementer");
        assert_eq!(persisted.dispatch_status, "executed");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_prefers_active_downstream_blocked_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-active-downstream-result-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-active-downstream-result";
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-active-downstream-result-verification.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "verifier_proof_packet",
                "run_id": run_id,
                "verifier_proof_packet": {
                    "proof_goal": "verify the bounded packet",
                    "verification_command": "cargo test -p vida verifier-smoke",
                    "proof_target": "bounded verifier proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "verification",
                "activation_agent_type": "senior",
                "activation_runtime_role": "verifier",
                "selected_backend": "senior",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"],
                                "implementer_activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "coach_activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "verifier_activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-active-downstream-result-verification.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet verification.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for verifier evidence".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_verification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from active downstream result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "verification");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            inputs.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            inputs.dispatch_receipt.dispatch_surface.as_deref(),
            Some("internal_cli:codex")
        );
        assert_eq!(
            inputs.dispatch_receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            inputs.dispatch_receipt.activation_agent_type.as_deref(),
            Some("senior")
        );
        assert_eq!(
            inputs.dispatch_receipt.activation_runtime_role.as_deref(),
            Some("verifier")
        );
        assert_eq!(
            inputs.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_prefers_active_downstream_result_over_stale_ready_packet_for_coach_active_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-coach-active-precedence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-coach-active-precedence";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-coach-active-precedence-stale.json");
        let active_packet_path = packet_dir.join("run-coach-active-precedence-active.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "dev-pack",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path = result_dir.join("run-coach-active-precedence-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: Some("receipt-implementer-current".to_string()),
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after implementer evidence, activate coach".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(stale_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let active_result_resume = super::maybe_resume_inputs_from_active_downstream_result(
            &store,
            Some(run_id),
            &receipt,
        )
        .await
        .expect("active downstream result probe should not fail");
        let active_result_resume =
            active_result_resume.expect("active downstream result should be visible");
        assert_eq!(
            active_result_resume.dispatch_receipt.dispatch_target,
            "coach"
        );
        assert_eq!(
            active_result_resume.dispatch_receipt.dispatch_status,
            "blocked"
        );

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from active downstream coach result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            inputs.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            inputs.dispatch_receipt.dispatch_packet_path.as_deref(),
            Some(active_packet_path.display().to_string().as_str())
        );
        assert_eq!(
            inputs.dispatch_packet_path,
            active_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_for_completed_closure_bound_run_rejects_stale_active_and_ready_downstream_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-completed-closure-bound-stale-downstream-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-completed-closure-bound-stale-downstream";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-closure".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-completed-closure-bound-stale-coach.json");
        let active_packet_path = packet_dir.join("run-completed-closure-bound-active-coach.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "dev-pack",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"]
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream coach packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream coach packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path =
            result_dir.join("run-completed-closure-bound-stale-downstream-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: Some("receipt-implementer-current".to_string()),
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale downstream coach evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(stale_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-closure".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": "task-closure",
                        "run_id": run_id,
                        "dispatch_target": "closure",
                    }),
                    binding_source: "latest_run_graph_dispatch_receipt".to_string(),
                    why_this_unit: "closure is the only lawful next bounded unit".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_downstream_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit closure binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale downstream coach lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicitly bound to downstream target `closure`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("stale downstream target `coach`"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_without_run_id_fails_closed_for_ambiguous_completed_run_with_active_downstream_result(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-ambiguous-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-ambiguous-latest";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "delivery",
        );
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let active_packet_path = packet_dir.join("run-ambiguous-latest-active.json");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"],
                                "implementer_activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "coach_activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "verifier_activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path = result_dir.join("run-ambiguous-latest-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after coach evidence, activate verification".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: Some(active_packet_path.display().to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let error = match resolve_runtime_consumption_resume_inputs(&store, None, None, None).await
        {
            Ok(_) => {
                panic!("ambiguous completed run should fail closed without --run-id");
            }
            Err(error) => error,
        };
        assert!(
            error.contains(
                "Latest continuation binding for run `run-ambiguous-latest` is ambiguous"
            ),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_fails_closed_when_explicit_task_graph_binding_mismatches_dispatch_packet_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-explicit-binding-mismatch";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-old".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("run-explicit-binding-mismatch.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "dispatch_status": "executed",
                "lane_status": "lane_completed",
                "dispatch_kind": "taskflow_pack",
                "dispatch_surface": "vida taskflow consume",
                "dispatch_command": "vida taskflow consume continue --run-id run-explicit-binding-mismatch --json",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "taskflow_state_store",
                "recorded_at": "2026-04-13T00:00:00Z",
                "request_text": "continue development",
                "role_selection": {
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "tracked_flow_entry": "dev-pack",
                    "confidence": "high"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "taskflow_handoff_plan": null,
                "run_graph_bootstrap": {
                    "run_id": run_id,
                    "latest_status": {
                        "run_id": run_id,
                        "task_id": "task-old"
                    }
                },
                "orchestration_contract": null
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow consume".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "test mismatch".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale packet lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicit continuation binding to task_graph_task `task-new`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("still points to task `task-old`"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_allows_matching_explicit_task_graph_binding_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-match-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-explicit-binding-match";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-aligned".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("run-explicit-binding-match.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "dispatch_status": "executed",
                "lane_status": "lane_completed",
                "dispatch_kind": "taskflow_pack",
                "dispatch_surface": "vida taskflow consume",
                "dispatch_command": "vida taskflow consume continue --run-id run-explicit-binding-match --json",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "taskflow_state_store",
                "recorded_at": "2026-04-13T00:00:00Z",
                "request_text": "continue development",
                "role_selection": {
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "tracked_flow_entry": "dev-pack",
                    "confidence": "high"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "taskflow_handoff_plan": null,
                "run_graph_bootstrap": {
                    "run_id": run_id,
                    "latest_status": {
                        "run_id": run_id,
                        "task_id": "task-aligned"
                    }
                },
                "orchestration_contract": null
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("dispatch packet should validate");
        assert_eq!(
            persisted_dispatch_packet_lineage_task_id(&packet),
            Some("task-aligned")
        );

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow consume".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-aligned".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-aligned",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "test match".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("matching explicit binding should keep current resume path admissible");
        assert_eq!(resolved.dispatch_receipt.run_id, run_id);
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(
            resolved.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn runtime_consumption_resume_blocker_code_uses_explicit_run_receipt_lineage_when_run_id_is_requested(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-run-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let explicit_run_id = "run-explicit";
        let mut explicit_status =
            crate::taskflow_run_graph::default_run_graph_status(explicit_run_id, "implementer", "delivery");
        explicit_status.task_id = "task-explicit".to_string();
        explicit_status.status = "running".to_string();
        explicit_status.lifecycle_stage = "execution_active".to_string();
        explicit_status.resume_target = "current_lane".to_string();
        store
            .record_run_graph_status(&explicit_status)
            .await
            .expect("persist explicit status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: explicit_run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some(format!(
                    "vida taskflow consume continue --run-id {explicit_run_id} --json"
                )),
                dispatch_packet_path: Some("/tmp/explicit-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/explicit-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-15T00:00:00Z".to_string(),
            })
            .await
            .expect("persist explicit receipt");

        let latest_run_id = "run-latest";
        let mut latest_status =
            crate::taskflow_run_graph::default_run_graph_status(latest_run_id, "closure", "delivery");
        latest_status.task_id = "task-latest".to_string();
        latest_status.status = "completed".to_string();
        latest_status.lifecycle_stage = "closure_complete".to_string();
        latest_status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: latest_run_id.to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/latest-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/latest-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-15T00:00:01Z".to_string(),
            })
            .await
            .expect("persist latest receipt");

        let payload_json = serde_json::json!({
            "dispatch_receipt": {
                "run_id": explicit_run_id
            }
        });

        let explicit_blocker = runtime_consumption_resume_blocker_code(
            &store,
            &payload_json,
            Some(explicit_run_id),
        )
        .await
        .expect("explicit blocker lookup should succeed");
        assert_eq!(explicit_blocker, None);

        let latest_blocker = runtime_consumption_resume_blocker_code(&store, &payload_json, None)
            .await
            .expect("latest blocker lookup should succeed");
        assert_eq!(
            latest_blocker.as_deref(),
            Some(super::super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_runtime_dispatch_packet_backfills_read_only_paths_for_legacy_packets() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": {
                "packet_id": "run-1::coach::coach-review",
                "review_goal": "review bounded packet",
                "owned_paths": [],
                "definition_of_done": ["return bounded review evidence"],
                "proof_target": "bounded proof target",
                "blocking_question": "is it aligned?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["coach_review_packet"]["read_only_paths"],
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS)
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_owned_paths_for_legacy_implementer_delivery_packet(
    ) {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "implementer",
            "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
        assert_eq!(
            packet["delivery_task_packet"]["read_only_paths"],
            serde_json::json!([".vida/data/state/runtime-consumption"])
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_owned_paths_from_delivery_packet_request_text() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_packet_scope_before_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-dispatch-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "coach_review_packet": {
                    "packet_id": "run-1::coach::coach-review",
                    "review_goal": "review bounded packet",
                    "owned_paths": [],
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded proof target",
                    "blocking_question": "is it aligned?"
                }
            })
            .to_string(),
        )
        .expect("write legacy packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy packet should normalize and validate");
        assert_eq!(
            packet["coach_review_packet"]["read_only_paths"],
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS)
        );

        let persisted = fs::read_to_string(&packet_path).expect("normalized packet should persist");
        assert!(persisted.contains("\"read_only_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_implementer_delivery_owned_scope_from_request_text() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-implementer-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "dispatch_target": "implementer",
                "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                }
            })
            .to_string(),
        )
        .expect("write legacy implementer packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy implementer packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("\"owned_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_implementer_scope_from_delivery_packet_request_text() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-implementer-delivery-body-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                }
            })
            .to_string(),
        )
        .expect("write legacy implementer packet with nested request");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy implementer packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("\"owned_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_rejects_widened_single_task_move_scope() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-widened-single-task-move-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "request_text": "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation.",
                "role_selection_full": {
                    "single_task_only": true
                },
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded `implementer` handoff for the active runtime request",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": [
                        "crates/vida/src/main.rs",
                        "crates/vida/src/project_activator_surface.rs",
                        "crates/vida/src/runtime_dispatch_state.rs"
                    ],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for `implementer`?"
                }
            })
            .to_string(),
        )
        .expect("write widened single-task move packet");

        let error =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect_err("widened packet should fail closed");
        assert!(error.contains("single-task move packet owned_paths"));
        let _ = fs::remove_file(packet_path);
    }
}
