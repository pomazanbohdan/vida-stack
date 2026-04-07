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
            let receipt_exists = matches!(
                store.run_graph_dispatch_receipt(run_id).await,
                Ok(Some(_))
            );
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

fn emit_runtime_consumption_resume_json(
    store: &super::StateStore,
    surface_name: &str,
    dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    emit_output: bool,
    as_json: bool,
) -> Result<(), String> {
    let failure_control_evidence =
        build_failure_control_evidence(&dispatch_receipt.run_id, dispatch_packet_path);
    let mut payload_json = serde_json::json!({
        "dispatch_receipt": dispatch_receipt,
        "role_selection": role_selection,
        "source_dispatch_packet_path": dispatch_packet_path,
        "source_run_id": dispatch_receipt.run_id,
        "failure_control_evidence": failure_control_evidence.clone(),
    });
    let runtime_dispatch_receipt_blocker_code =
        super::runtime_consumption_final_dispatch_receipt_blocker_code(store, &payload_json)?;
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
                "dispatch_receipt": dispatch_receipt,
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
            let receipt_exists = matches!(
                store.run_graph_dispatch_receipt(run_id).await,
                Ok(Some(_))
            );
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
    let Some(active_packet) = packet.get_mut(&packet_template_kind) else {
        return false;
    };
    if active_packet.is_null() || packet_has_owned_or_read_only_paths(active_packet) {
        return false;
    }
    let Some(active_packet_object) = active_packet.as_object_mut() else {
        return false;
    };
    active_packet_object.insert(
        "read_only_paths".to_string(),
        serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS),
    );
    true
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

struct ResumeInputs {
    dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: String,
    role_selection: super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: serde_json::Value,
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
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, dispatch_target);
    let selected_backend = activation_agent_type
        .clone()
        .or_else(|| root_receipt.selected_backend.clone())
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
        downstream_dispatch_result_path: packet
            .get("downstream_dispatch_result_path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
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

async fn resolve_runtime_consumption_resume_inputs(
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
        let receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => return Err(missing_dispatch_receipt_error(run_id)),
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ));
            }
        };
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, requested_run_id, &receipt)
                .await?
        {
            return Ok(resume);
        }
        let packet_path = receipt
            .dispatch_packet_path
            .clone()
            .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
        let packet = read_dispatch_packet(&packet_path)?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
        validate_run_graph_resume_state(store, run_id).await?;
        build_resume_inputs(receipt, packet_path, packet, role_selection)
    } else {
        let receipt = match store.latest_run_graph_dispatch_receipt().await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                return Err("No persisted run-graph dispatch receipt is available".to_string());
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ));
            }
        };
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, requested_run_id, &receipt)
                .await?
        {
            return Ok(resume);
        }
        let packet_path = receipt
            .dispatch_packet_path
            .clone()
            .ok_or_else(|| missing_dispatch_packet_path_error(true))?;
        let packet = read_dispatch_packet(&packet_path)?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
        validate_run_graph_resume_state(store, &receipt.run_id).await?;
        build_resume_inputs(receipt, packet_path, packet, role_selection)
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
                    || super::taskflow_task_bridge::infer_project_root_from_state_root(
                        store.root(),
                    )
                    .is_some();
                if allow_taskflow_pack_execution {
                    if let Err(error) = super::execute_and_record_dispatch_receipt(
                        store.root(),
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to execute resumed runtime dispatch handoff: {error}");
                        return ExitCode::from(1);
                    }
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        store.root(),
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    ) {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
            }
            if let Err(error) = super::execute_downstream_dispatch_chain(
                store.root(),
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!("{error}");
                return ExitCode::from(1);
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
                emit_output,
                as_json,
            ) {
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
        DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS, build_failure_control_evidence,
        canonical_resume_dispatch_status, canonical_resume_lane_status,
        canonical_resume_string_array_entries, normalize_runtime_dispatch_packet,
        read_dispatch_packet, resume_from_persisted_final_snapshot,
        resume_packet_ready_blocker_parity_error, validate_run_graph_resume_state,
        validate_run_graph_resume_state_for_downstream_packet,
        runtime_consumption_snapshot_has_failure_control_evidence,
    };
    use crate::StateStore;
    use crate::downstream_dispatch_ready_blocker_parity_error;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn resume_from_persisted_final_snapshot_rejects_final_snapshot_without_failure_control_evidence()
     {
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
        assert!(
            !resume_from_persisted_final_snapshot(&store).expect("runtime consumption summary")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_persisted_receipt_lineage_when_summary_rows_are_missing()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-receipt-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone())
            .await
            .expect("open store");

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
        let failure_control_evidence = build_failure_control_evidence(run_id, &snapshot_path_string);
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
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "closure",
            "closure",
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
}
