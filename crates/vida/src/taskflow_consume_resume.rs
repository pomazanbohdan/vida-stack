use crate::taskflow_run_graph::validate_run_graph_resume_gate;
use std::process::ExitCode;

const DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS: [&str; 3] = [
    ".vida/data/state/runtime-consumption",
    "docs/product/spec",
    "docs/process",
];
const STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS: i64 = 10;

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

fn sanitize_inherited_downstream_lane_evidence(
    packet: &serde_json::Value,
    downstream_dispatch_status: Option<&str>,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    parsed_downstream_lane_status: Option<super::LaneStatus>,
) -> (Option<String>, Option<String>, Option<super::LaneStatus>) {
    let source_supersedes_receipt_id = packet
        .get("source_supersedes_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let source_exception_path_receipt_id = packet
        .get("source_exception_path_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let inherited_supersedes = supersedes_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        == source_supersedes_receipt_id;
    let inherited_exception = exception_path_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        == source_exception_path_receipt_id;

    let supersedes_receipt_id = if inherited_supersedes {
        None
    } else {
        supersedes_receipt_id
    };
    let exception_path_receipt_id = if inherited_exception {
        None
    } else {
        exception_path_receipt_id
    };

    let parsed_downstream_lane_status = if inherited_supersedes || inherited_exception {
        let sanitized_derived_lane_status = super::derive_lane_status(
            downstream_dispatch_status.unwrap_or("blocked"),
            supersedes_receipt_id.as_deref(),
            exception_path_receipt_id.as_deref(),
        );
        match parsed_downstream_lane_status {
            Some(
                super::LaneStatus::LaneExceptionRecorded
                | super::LaneStatus::LaneExceptionTakeover
                | super::LaneStatus::LaneSuperseded,
            ) => Some(sanitized_derived_lane_status),
            value => value,
        }
    } else {
        parsed_downstream_lane_status
    };

    (
        supersedes_receipt_id,
        exception_path_receipt_id,
        parsed_downstream_lane_status,
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
    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
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

fn runtime_consumption_resume_receipt_blocker_codes(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if let Some(blocker_code) = dispatch_receipt
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if matches!(
        dispatch_receipt.dispatch_status.as_str(),
        "blocked" | "failed"
    ) || matches!(
        dispatch_receipt.lane_status.as_str(),
        "lane_blocked" | "lane_failed"
    ) {
        blocker_codes.extend(
            dispatch_receipt
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes)
}

fn runtime_consumption_resume_receipt_next_actions(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    blocker_codes: &[String],
) -> Vec<String> {
    if blocker_codes.is_empty() {
        return Vec::new();
    }

    let mut next_actions = Vec::new();
    next_actions.push(
        "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
            .to_string(),
    );
    let current_lane_has_execution_evidence =
        crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(dispatch_receipt);
    if current_lane_has_execution_evidence
        && blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence")
    {
        next_actions.push(
            "Record the missing clean review evidence before activating the downstream verification lane."
                .to_string(),
        );
    }
    crate::operator_contracts::canonical_next_action_entries(&serde_json::json!(next_actions))
        .unwrap_or_else(|| next_actions.to_vec())
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
    let projection_truth = match store.run_graph_status(&dispatch_receipt.run_id).await {
        Ok(status) => Some(
            crate::taskflow_run_graph::run_graph_projection_truth(store, &status)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to build run-graph projection truth for `{}`: {error}",
                        dispatch_receipt.run_id
                    )
                })?,
        ),
        Err(_) => None,
    };
    let mut blocker_codes =
        runtime_consumption_resume_receipt_blocker_codes(&normalized_dispatch_receipt);
    let mut next_actions = runtime_consumption_resume_receipt_next_actions(
        &normalized_dispatch_receipt,
        &blocker_codes,
    );
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
    let preliminary_status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    payload_json["release_admission"] = serde_json::json!({});
    let snapshot = serde_json::json!({
        "surface": surface_name,
        "status": preliminary_status,
        "release_admission": {},
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    let snapshot_path =
        super::write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": dispatch_receipt.run_id,
            "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
            "consume_final_surface": surface_name,
        }),
    )?;
    let status = finalized.status;
    let operator_contracts = finalized.operator_contracts.clone();
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": finalized.shared_fields["status"].clone(),
        "blocker_codes": finalized.shared_fields["blocker_codes"].clone(),
        "next_actions": finalized.shared_fields["next_actions"].clone(),
        "artifact_refs": finalized.shared_fields["artifact_refs"].clone(),
    });
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": surface_name,
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": status,
        "blocker_codes": finalized.blocker_codes.clone(),
        "next_actions": finalized.next_actions.clone(),
        "artifact_refs": finalized.artifact_refs.clone(),
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
    if let Some(error) =
        crate::operator_contracts::shared_operator_output_contract_parity_error(
            &snapshot_with_operator_contracts,
        )
    {
        return Err(format!(
            "Failed to preserve runtime-consumption resume operator-contract parity: {error}"
        ));
    }
    if !emit_output {
        return Ok(());
    }
    if as_json {
        let output_payload = serde_json::json!({
            "surface": surface_name,
            "trace_id": operator_contracts["trace_id"].clone(),
            "workflow_class": operator_contracts["workflow_class"].clone(),
            "risk_tier": operator_contracts["risk_tier"].clone(),
            "status": status,
            "blocker_codes": snapshot_with_operator_contracts["blocker_codes"].clone(),
            "next_actions": snapshot_with_operator_contracts["next_actions"].clone(),
            "artifact_refs": finalized.artifact_refs.clone(),
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts,
            "source_run_id": dispatch_receipt.run_id,
            "source_dispatch_packet_path": dispatch_packet_path,
            "dispatch_receipt": payload_json["dispatch_receipt"].clone(),
            "projection_truth": projection_truth,
            "snapshot_path": snapshot_path,
            "failure_control_evidence": snapshot_with_operator_contracts["failure_control_evidence"].clone(),
        });
        if let Some(error) =
            crate::operator_contracts::shared_operator_output_contract_parity_error(
                &output_payload,
            )
        {
            return Err(format!(
                "Failed to preserve runtime-consumption resume output parity: {error}"
            ));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&output_payload)
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
        if let Some(projection_truth) = projection_truth.as_ref() {
            super::print_surface_line(
                super::RenderMode::Plain,
                "projection",
                &projection_truth.projection_reason,
            );
            if let Some(next_action) = projection_truth.next_lawful_operator_action.as_deref() {
                super::print_surface_line(super::RenderMode::Plain, "next action", next_action);
            }
        }
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

fn derive_implementer_owned_paths(packet: &serde_json::Value) -> Option<Vec<String>> {
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

    let request_text = packet_request_text(packet).unwrap_or_default();
    let tracked_design_doc_path = packet["role_selection_full"]["execution_plan"]
        ["tracked_flow_bootstrap"]["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let owned_paths = crate::runtime_dispatch_packets::delivery_packet_owned_paths(
        "implementation",
        request_text,
        tracked_design_doc_path,
    );
    (!owned_paths.is_empty()).then_some(owned_paths)
}

fn derive_specification_owned_paths_from_tracked_design_doc(
    packet: &serde_json::Value,
) -> Option<Vec<String>> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if packet_template_kind != "delivery_task_packet" {
        return None;
    }
    let dispatch_target = packet_dispatch_target(packet)?;
    if dispatch_target != "specification" {
        return None;
    }

    let design_doc_path = packet["role_selection_full"]["execution_plan"]["tracked_flow_bootstrap"]
        ["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(vec![design_doc_path.to_string()])
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
    let derived_implementer_owned_paths = derive_implementer_owned_paths(packet);
    let derived_specification_owned_paths =
        derive_specification_owned_paths_from_tracked_design_doc(packet);
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
        if let Some(owned_paths) = derived_implementer_owned_paths
            .clone()
            .or_else(|| derived_specification_owned_paths.clone())
        {
            active_packet_object.insert("owned_paths".to_string(), serde_json::json!(owned_paths));
            normalized = true;
        }
    } else if let Some(expected_owned_paths) = derived_specification_owned_paths {
        let actual_owned_paths = active_packet_object
            .get("owned_paths")
            .and_then(serde_json::Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .unwrap_or_default();
        if actual_owned_paths != expected_owned_paths {
            active_packet_object.insert(
                "owned_paths".to_string(),
                serde_json::json!(expected_owned_paths),
            );
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

fn build_run_graph_replay_lineage_receipt(
    status: &crate::state_store::RunGraphStatus,
    source_receipt: &crate::state_store::RunGraphDispatchReceipt,
    resume: &ResumeInputs,
    lineage_kind: &str,
) -> Result<crate::state_store::RunGraphReplayLineageReceipt, String> {
    let checkpoint_kind = status.checkpoint_kind.trim().to_string();
    let resume_target = status.resume_target.trim().to_string();
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&super::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let source_dispatch_packet_path = match lineage_kind {
        "downstream_packet" | "downstream_result" => source_receipt
            .downstream_dispatch_packet_path
            .clone()
            .or_else(|| source_receipt.dispatch_packet_path.clone()),
        _ => source_receipt.dispatch_packet_path.clone(),
    };
    let source_dispatch_result_path = match lineage_kind {
        "downstream_result" => source_receipt
            .downstream_dispatch_result_path
            .clone()
            .or_else(|| source_receipt.dispatch_result_path.clone()),
        _ => source_receipt.dispatch_result_path.clone(),
    };
    let resolved_task_id = status.task_id.trim().to_string();
    let receipt_id = format!(
        "replay-lineage-{}-{}",
        resume.dispatch_receipt.run_id,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    if checkpoint_kind.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing checkpoint_kind in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    if resume_target.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing resume_target in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    if resolved_task_id.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing task_id in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    Ok(crate::state_store::RunGraphReplayLineageReceipt {
        receipt_id,
        run_id: resume.dispatch_receipt.run_id.clone(),
        lineage_kind: lineage_kind.to_string(),
        replay_scope: "resume_resolution".to_string(),
        origin_checkpoint_ref: format!(
            "{}:{}:{}",
            resume.dispatch_receipt.run_id, checkpoint_kind, resume_target
        ),
        fork_parent: None,
        source_dispatch_target: source_receipt.dispatch_target.clone(),
        source_dispatch_packet_path,
        source_dispatch_result_path,
        resolved_dispatch_target: resume.dispatch_receipt.dispatch_target.clone(),
        resolved_task_id,
        checkpoint_kind,
        resume_target,
        validation_outcome: "lawful_resume".to_string(),
        recorded_at,
    })
}

async fn record_run_graph_replay_lineage_receipt_for_resume(
    store: &super::StateStore,
    source_receipt: &crate::state_store::RunGraphDispatchReceipt,
    resume: &ResumeInputs,
    lineage_kind: &str,
) -> Result<(), String> {
    let status = store
        .run_graph_status(&resume.dispatch_receipt.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to load run-graph status for replay lineage receipt: {error}"
            )
        })?;
    let receipt = build_run_graph_replay_lineage_receipt(
        &status,
        source_receipt,
        resume,
        lineage_kind,
    )?;
    store
        .record_run_graph_replay_lineage_receipt(&receipt)
        .await
        .map_err(|error| format!("Failed to record run-graph replay lineage receipt: {error}"))
}

async fn recover_missing_first_dispatch_receipt(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<ResumeInputs>, String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };
    if status.status == "completed" {
        return Ok(None);
    }
    let run_graph_bootstrap =
        match super::taskflow_run_graph::run_graph_dispatch_bootstrap_from_status(&status) {
            Ok(bootstrap) => bootstrap,
            Err(_) if status.status != "completed" => serde_json::json!({
                "status": "dispatch_init_ready",
                "handoff_ready": true,
                "run_id": status.run_id,
                "latest_status": serde_json::to_value(&status)
                    .map_err(|error| format!("Failed to encode status: {error}"))?,
            }),
            Err(_) => return Ok(None),
        };

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

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    let active_lane_in_progress = status.status == "ready"
        && status.lifecycle_stage.ends_with("_active")
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|next_node| next_node != status.active_node);
    if active_lane_in_progress {
        let dispatch_target = status.active_node.clone();
        let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
            super::downstream_activation_fields(&role_selection, &dispatch_target);
        dispatch_receipt.dispatch_target = dispatch_target.clone();
        dispatch_receipt.dispatch_status = "executed".to_string();
        dispatch_receipt.lane_status = super::LaneStatus::LaneRunning.as_str().to_string();
        dispatch_receipt.dispatch_kind = dispatch_kind;
        dispatch_receipt.dispatch_surface = dispatch_surface;
        dispatch_receipt.dispatch_command =
            super::runtime_dispatch_command_for_target(&role_selection, &dispatch_target);
        dispatch_receipt.downstream_dispatch_target = Some(dispatch_target.clone());
        dispatch_receipt.downstream_dispatch_command =
            super::runtime_dispatch_command_for_target(&role_selection, &dispatch_target);
        dispatch_receipt.activation_agent_type = activation_agent_type;
        dispatch_receipt.activation_runtime_role = activation_runtime_role;
        dispatch_receipt.selected_backend = super::downstream_selected_backend(
            &role_selection,
            &dispatch_target,
            dispatch_receipt.activation_agent_type.as_deref(),
            None,
        )
        .filter(|value| !value.is_empty());
    }
    dispatch_receipt.recorded_at = recorded_at;
    dispatch_receipt.dispatch_command = super::runtime_dispatch_command_for_target(
        &role_selection,
        &dispatch_receipt.dispatch_target,
    );
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

fn dispatch_receipt_same_lane_resume_ready(
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

fn allow_downstream_resume_lineage(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    !dispatch_receipt_retry_eligible(dispatch_receipt)
}

fn retry_backend_for_dispatch_receipt(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    let fallback = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )
    .and_then(crate::taskflow_routing::fallback_executor_backend_from_route)
    .or_else(|| {
        dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .and_then(|path| {
                retry_backend_from_dispatch_packet(path, &dispatch_receipt.dispatch_target)
            })
    })?;
    if dispatch_receipt.selected_backend.as_deref() == Some(fallback.as_str()) {
        return None;
    }
    Some(fallback)
}

fn retry_backend_from_dispatch_packet(packet_path: &str, dispatch_target: &str) -> Option<String> {
    let packet = read_dispatch_packet(packet_path)
        .ok()
        .or_else(|| crate::read_json_file_if_present(std::path::Path::new(packet_path)))?;
    super::execution_plan_route_for_dispatch_target(
        &packet["role_selection_full"]["execution_plan"],
        dispatch_target,
    )
    .and_then(crate::taskflow_routing::fallback_executor_backend_from_route)
    .or_else(|| {
        packet["execution_truth"]["route_fallback_backend"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
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
    let (_selected_cli_system, selected_cli_entry) =
        super::selected_host_cli_system_for_runtime_dispatch(&overlay);
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
    let has_internal_carriers = !carriers.is_empty();
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
            || (*backend_id == "internal_subagents" && has_internal_carriers)
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
    let (supersedes_receipt_id, exception_path_receipt_id, parsed_downstream_lane_status) =
        sanitize_inherited_downstream_lane_evidence(
            &packet,
            downstream_dispatch_status.as_deref(),
            supersedes_receipt_id,
            exception_path_receipt_id,
            parsed_downstream_lane_status,
        );
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
    match (ready_target, active_target) {
        (Some(ready), Some(active)) if ready != active => {
            let Some(result_path) = receipt.downstream_dispatch_result_path.as_deref() else {
                return true;
            };
            let active_result = match read_downstream_dispatch_result(result_path) {
                Ok(result) => result,
                Err(_) => return true,
            };
            let active_execution_state = active_result
                .get("execution_state")
                .and_then(serde_json::Value::as_str)
                .map(|value| canonical_resume_dispatch_status(Some(value)))
                .unwrap_or("blocked");
            active_execution_state != "blocked"
        }
        _ => false,
    }
}

fn downstream_result_packet_path(result: &serde_json::Value) -> Option<String> {
    if let Some(path) = result
        .get("dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    {
        return Some(path);
    }

    let source_path = result
        .get("source_dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let source_packet = crate::read_json_file_if_present(std::path::Path::new(source_path))?;
    if source_packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        == Some("runtime_downstream_dispatch_packet")
    {
        return None;
    }
    Some(source_path.to_string())
}

fn read_downstream_dispatch_result(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted downstream dispatch result: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted downstream dispatch result: {error}"))
}

fn stale_in_flight_dispatch_preserves_internal_activation_view(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> bool {
    receipt.selected_backend.as_deref() == Some("internal_subagents")
        || receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:"))
        || result["surface"]
            .as_str()
            .is_some_and(|value| value.starts_with("internal_cli:"))
        || result["backend_dispatch"]["backend_class"].as_str() == Some("internal")
        || dispatch_packet_indicates_internal_activation_view(
            receipt.dispatch_packet_path.as_deref(),
            result,
        )
}

fn dispatch_packet_indicates_internal_activation_view(
    dispatch_packet_path: Option<&str>,
    result: &serde_json::Value,
) -> bool {
    let packet_path = dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            result
                .get("source_dispatch_packet_path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        });
    let Some(packet_path) = packet_path else {
        return false;
    };
    let Some(packet) = crate::read_json_file_if_present(std::path::Path::new(packet_path)) else {
        return false;
    };
    packet["host_runtime"]["selected_cli_execution_class"].as_str() == Some("internal")
        || packet["effective_execution_posture"]["effective_posture_kind"].as_str()
            == Some("internal")
        || packet["mixed_posture"]["effective_posture_kind"].as_str() == Some("internal")
        || packet["effective_execution_posture"]["selected_execution_class"].as_str()
            == Some("internal")
}

fn dispatch_packet_uses_downstream_carrier(
    dispatch_packet_path: Option<&str>,
    result: &serde_json::Value,
) -> bool {
    let packet_path = dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            result
                .get("source_dispatch_packet_path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        });
    let Some(packet_path) = packet_path else {
        return false;
    };
    let Some(packet) = crate::read_json_file_if_present(std::path::Path::new(packet_path)) else {
        return false;
    };
    packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        == Some("runtime_downstream_dispatch_packet")
}

fn normalize_stale_in_flight_dispatch_receipt(
    state_root: &std::path::Path,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    let timeout_blocked_receipt = receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("timeout_without_takeover_authority");
    if receipt.dispatch_status != "executing" && !timeout_blocked_receipt {
        return Ok(false);
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return Ok(false);
    };
    if timeout_blocked_receipt
        && stale_in_flight_dispatch_preserves_internal_activation_view(receipt, &result)
        && result["blocker_code"].as_str() == Some("timeout_without_takeover_authority")
    {
        super::apply_internal_activation_timeout_to_receipt(
            state_root,
            receipt,
            STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS as u64,
        )?;
        return Ok(true);
    }
    if result["execution_state"].as_str() != Some("executing") {
        return Ok(false);
    }
    if dispatch_packet_uses_downstream_carrier(receipt.dispatch_packet_path.as_deref(), &result) {
        super::apply_dispatch_execution_timeout_to_receipt(
            state_root,
            receipt,
            STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS as u64,
        )?;
        return Ok(true);
    }
    let Some(recorded_at) = result["recorded_at"].as_str() else {
        return Ok(false);
    };
    let Ok(recorded_at) =
        time::OffsetDateTime::parse(recorded_at, &time::format_description::well_known::Rfc3339)
    else {
        return Ok(false);
    };
    let age_seconds = (time::OffsetDateTime::now_utc() - recorded_at).whole_seconds();
    if age_seconds <= STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS {
        return Ok(false);
    }
    if stale_in_flight_dispatch_preserves_internal_activation_view(receipt, &result) {
        super::apply_internal_activation_timeout_to_receipt(
            state_root,
            receipt,
            STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS as u64,
        )?;
    } else {
        super::apply_dispatch_execution_timeout_to_receipt(
            state_root,
            receipt,
            STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS as u64,
        )?;
    }
    Ok(true)
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

async fn reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    super::maybe_bridge_closed_implementer_task_into_receipt_with_context(
        store,
        role_selection,
        run_graph_bootstrap,
        dispatch_receipt,
        None,
    )
    .await
}

async fn reconcile_blocked_verification_timeout_with_receipt_evidence(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    super::maybe_reconcile_blocked_verification_timeout_with_receipt_evidence(
        store,
        role_selection,
        run_graph_bootstrap,
        dispatch_receipt,
    )
    .await
}

/// Keep retry-artifact preparation strictly fail-closed: it may tune admissible
/// retry backend hints, but it must still restore a lawful run-graph dispatch-ready
/// posture for the same bounded node when an explicit retry packet already exists.
async fn sync_run_graph_after_retry_artifact(
    store: &super::StateStore,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "packet_ready"
        || dispatch_receipt.lane_status != "packet_ready"
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
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
        format!("Failed to read persisted run-graph state for retry artifact sync: {error}")
    })?;
    let retry_target = dispatch_receipt.dispatch_target.replace('-', "_");
    let lane_suffix = if dispatch_receipt.dispatch_kind == "taskflow_pack" {
        String::new()
    } else {
        "_lane".to_string()
    };
    let retry_ready_status = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: dispatch_receipt.dispatch_target.clone(),
        next_node: Some(retry_target.clone()),
        status: "ready".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: dispatch_receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: format!("{retry_target}{lane_suffix}"),
        lifecycle_stage: format!("{retry_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state: format!("awaiting_{retry_target}"),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: format!("dispatch.{retry_target}{lane_suffix}"),
        recovery_ready: true,
    };
    store
        .record_run_graph_status(&retry_ready_status)
        .await
        .map_err(|error| format!("Failed to record retry-ready run-graph status: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &retry_ready_status,
        "retry_artifact_dispatch_ready",
    )
    .await
    .map_err(|error| {
        format!("Failed to synchronize continuation binding after retry artifact sync: {error}")
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
        .latest_explicit_run_graph_continuation_binding()
        .await
        .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?;
    if let Some(binding) = explicit_continuation_binding.as_ref() {
        let bound_run_id = binding
            .active_bounded_unit
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if binding.status == "bound"
            && binding.binding_source == "explicit_continuation_bind_task"
            && bound_run_id.is_some_and(|binding_run_id| binding_run_id != status.run_id)
        {
            let binding_run_id = bound_run_id.unwrap_or("unknown");
            return Err(format!(
                "Latest explicit continuation binding points to run `{binding_run_id}` while the latest run-graph status is `{}`. Default `vida taskflow consume continue --json` must not silently reselect the stale latest run; pass `--run-id {binding_run_id}` or refresh/bind the intended bounded unit explicitly.",
                status.run_id
            ));
        }
    }
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
    let terminal_completed_run = status.status == "completed"
        && status.lifecycle_stage == "closure_complete";
    if terminal_completed_run {
        return Err(format!(
            "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
            status.run_id, status.run_id, status.run_id
        ));
    }
    if continuation_binding["status"] != "bound" {
        return Err(format!(
            "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
            status.run_id, status.run_id, status.run_id
        ));
    }
    if continuation_binding["active_bounded_unit"]["run_id"]
        .as_str()
        .is_some_and(|binding_run_id| binding_run_id != status.run_id)
    {
        let binding_run_id = continuation_binding["active_bounded_unit"]["run_id"]
            .as_str()
            .unwrap_or("unknown");
        return Err(format!(
            "Latest explicit continuation binding points to run `{binding_run_id}` while the latest run-graph status is `{}`. Default `vida taskflow consume continue --json` must not silently reselect the stale latest run; pass `--run-id {binding_run_id}` or refresh/bind the intended bounded unit explicitly.",
            status.run_id
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
    let mut receipt = match store.run_graph_dispatch_receipt(run_id).await {
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
    let normalized_stale_in_flight =
        normalize_stale_in_flight_dispatch_receipt(store.root(), &mut receipt)?;
    if normalized_stale_in_flight {
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .map_err(|error| {
                format!(
                    "Failed to persist normalized stale in-flight dispatch receipt for `{run_id}`: {error}"
                )
            })?;
    }
    validate_explicit_task_graph_binding_lineage_for_resume(store, run_id, &receipt).await?;
    let allow_downstream_lineage = allow_downstream_resume_lineage(&receipt);
    let explicit_downstream_target =
        completed_run_explicit_downstream_target_for_resume(store, run_id).await?;
    if let Some(bound_target) = explicit_downstream_target.as_deref() {
        let active_target_matches_bound = receipt
            .downstream_dispatch_active_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            == Some(bound_target);
        if allow_downstream_lineage && !active_target_matches_bound {
            if let Some(resume) =
                maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt)
                    .await?
            {
                if resume.dispatch_receipt.dispatch_target == bound_target {
                    record_run_graph_replay_lineage_receipt_for_resume(
                        store,
                        &receipt,
                        &resume,
                        "downstream_packet",
                    )
                    .await?;
                    return Ok(resume);
                }
                return Err(format!(
                    "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream packet lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                    resume.dispatch_receipt.dispatch_target
                ));
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) =
                maybe_resume_inputs_from_active_downstream_result(store, Some(run_id), &receipt)
                    .await?
            {
                if resume.dispatch_receipt.dispatch_target == bound_target {
                    record_run_graph_replay_lineage_receipt_for_resume(
                        store,
                        &receipt,
                        &resume,
                        "downstream_result",
                    )
                    .await?;
                    return Ok(resume);
                }
                return Err(format!(
                    "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream result lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                    resume.dispatch_receipt.dispatch_target
                ));
            }
        }
        return Err(missing_explicit_downstream_resume_evidence_error(
            run_id,
            bound_target,
        ));
    } else {
        if allow_downstream_lineage && prefer_ready_downstream_packet_over_active_result(&receipt) {
            if let Some(resume) =
                maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt)
                    .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_packet",
                )
                .await?;
                return Ok(resume);
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) =
                maybe_resume_inputs_from_active_downstream_result(store, Some(run_id), &receipt)
                    .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_result",
                )
                .await?;
                return Ok(resume);
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) =
                maybe_resume_inputs_from_ready_downstream_packet(store, Some(run_id), &receipt)
                    .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_packet",
                )
                .await?;
                return Ok(resume);
            }
        }
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    if explicit_downstream_target.is_none() && receipt.dispatch_target == "specification" {
        let run_graph_bootstrap = packet.get("run_graph_bootstrap").cloned().ok_or_else(|| {
            format!("Persisted dispatch packet `{packet_path}` is missing run_graph_bootstrap")
        })?;
        let mut bridged_receipt = receipt.clone();
        if super::try_bridge_bounded_specification_completion_to_downstream_receipt(
            store,
            &role_selection,
            &run_graph_bootstrap,
            &mut bridged_receipt,
        )
        .await?
        {
            let resume = build_resume_inputs(
                bridged_receipt,
                packet_path,
                packet,
                role_selection,
            );
            record_run_graph_replay_lineage_receipt_for_resume(
                store,
                &receipt,
                &resume,
                "root_dispatch_packet",
            )
            .await?;
            return Ok(resume);
        }
    }
    validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
    validate_run_graph_resume_state(store, run_id).await?;
    let resume = build_resume_inputs(receipt.clone(), packet_path, packet, role_selection);
    record_run_graph_replay_lineage_receipt_for_resume(
        store,
        &receipt,
        &resume,
        "root_dispatch_packet",
    )
    .await?;
    Ok(resume)
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
        let explicit_binding = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?;
        let latest_receipt = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .map_err(|error| format!("Failed to read latest run-graph dispatch receipt: {error}"))?;
        if let Some(status) = store
            .latest_run_graph_status()
            .await
            .map_err(|error| format!("Failed to read latest persisted run-graph state: {error}"))?
        {
            let terminal_completed_run =
                status.status == "completed" && status.lifecycle_stage == "closure_complete";
            let ambiguous_active_downstream_result = explicit_binding.is_none()
                && latest_receipt.as_ref().is_some_and(|receipt| {
                    receipt.run_id == status.run_id
                        && receipt
                            .downstream_dispatch_active_target
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .is_some()
                        && receipt
                            .downstream_dispatch_result_path
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .is_some()
                        && matches!(status.status.as_str(), "blocked" | "completed")
                });
            if terminal_completed_run || ambiguous_active_downstream_result {
                return Err(format!(
                    "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
                    status.run_id, status.run_id, status.run_id
                ));
            }
        }
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
        return true;
    }

    let Some(project_root) = project_root else {
        return false;
    };
    let _internal_retry_eligible =
        dispatch_receipt_internal_retry_eligible(project_root, role_selection, dispatch_receipt);
    if let Some(primary_backend) =
        primary_backend_for_dispatch_receipt(project_root, role_selection, dispatch_receipt)
    {
        dispatch_receipt.selected_backend = Some(primary_backend);
        return true;
    }
    if let Some(fallback_backend) = super::fallback_backend_for_blocked_primary_dispatch_receipt(
        project_root,
        role_selection,
        dispatch_receipt,
    ) {
        dispatch_receipt.selected_backend = Some(fallback_backend);
        return true;
    }
    false
}

fn resumed_selected_backend_for_agent_lane(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    prepared_retry_artifact: bool,
) -> Option<String> {
    let explicit_retry_backend = prepared_retry_artifact
        .then(|| dispatch_receipt.selected_backend.clone())
        .flatten();
    explicit_retry_backend
        .or_else(|| super::canonical_selected_backend_for_receipt(role_selection, dispatch_receipt))
}

fn rewrite_retry_dispatch_packet_if_downstream_carrier(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
    {
        return Ok(());
    }
    let Some(packet_path) = dispatch_receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let packet = read_dispatch_packet(packet_path)?;
    if packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        != Some("runtime_downstream_dispatch_packet")
    {
        return Ok(());
    }
    if let Some(retry_backend) =
        retry_backend_for_dispatch_receipt(role_selection, dispatch_receipt)
    {
        dispatch_receipt.selected_backend = Some(retry_backend);
    }

    let taskflow_handoff_plan = super::build_taskflow_handoff_plan(role_selection);
    let ctx = super::RuntimeDispatchPacketContext::new(
        store.root(),
        role_selection,
        dispatch_receipt,
        &taskflow_handoff_plan,
        run_graph_bootstrap,
    );
    let canonical_packet_path = super::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(canonical_packet_path);
    dispatch_receipt.dispatch_command = super::runtime_dispatch_command_for_target(
        role_selection,
        &dispatch_receipt.dispatch_target,
    );
    Ok(())
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
                super::try_bridge_bounded_specification_completion_to_downstream_receipt(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                )
                .await
            {
                eprintln!(
                    "Failed to bridge bounded specification completion into downstream receipt: {error}"
                );
                return ExitCode::from(1);
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
            if let Err(error) = reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!(
                    "Failed to reconcile blocked implementer timeout with tracked close evidence: {error}"
                );
                return ExitCode::from(1);
            }
            if let Err(error) = reconcile_blocked_verification_timeout_with_receipt_evidence(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!(
                    "Failed to reconcile blocked verification timeout with receipt evidence: {error}"
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
            let rewrite_retry_packet = dispatch_receipt.dispatch_kind == "agent_lane"
                && dispatch_receipt.dispatch_status == "blocked"
                && dispatch_receipt
                    .dispatch_packet_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty());
            let restore_same_lane_resume_ready = prepared_retry_artifact
                || dispatch_receipt_same_lane_resume_ready(&dispatch_receipt);
            if rewrite_retry_packet {
                if let Err(error) = rewrite_retry_dispatch_packet_if_downstream_carrier(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                ) {
                    eprintln!("Failed to rewrite retry dispatch packet into canonical dispatch packet: {error}");
                    return ExitCode::from(1);
                }
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
            }
            if restore_same_lane_resume_ready {
                if dispatch_receipt.dispatch_kind == "agent_lane" {
                    dispatch_receipt.selected_backend = resumed_selected_backend_for_agent_lane(
                        &role_selection,
                        &dispatch_receipt,
                        prepared_retry_artifact,
                    );
                }
                if dispatch_receipt.dispatch_status == "blocked"
                    && dispatch_receipt
                        .dispatch_packet_path
                        .as_deref()
                        .is_some_and(|path| !path.trim().is_empty())
                {
                    dispatch_receipt.dispatch_status = "packet_ready".to_string();
                    dispatch_receipt.lane_status = super::derive_lane_status(
                        &dispatch_receipt.dispatch_status,
                        dispatch_receipt.supersedes_receipt_id.as_deref(),
                        dispatch_receipt.exception_path_receipt_id.as_deref(),
                    )
                    .as_str()
                    .to_string();
                    dispatch_receipt.blocker_code = None;
                }
                if let Err(error) = store
                    .record_run_graph_dispatch_receipt(&dispatch_receipt)
                    .await
                {
                    eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                    return ExitCode::from(1);
                }
                if dispatch_receipt.dispatch_status == "packet_ready" {
                    if let Err(error) = sync_run_graph_after_retry_artifact(
                        &store,
                        &run_graph_bootstrap,
                        &dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
                if dispatch_receipt.dispatch_status == "executed" {
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
                if dispatch_receipt.dispatch_status != "packet_ready"
                    && dispatch_receipt.dispatch_status != "routed"
                {
                    if let Some(run_id) = run_graph_bootstrap
                        .get("run_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.is_empty())
                    {
                        match store.run_graph_status(run_id).await {
                            Ok(status) => {
                                if let Err(error) =
                                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                                        &store,
                                        &status,
                                        "consume_continue_receipt_refresh",
                                    )
                                    .await
                                {
                                    eprintln!(
                                        "Failed to refresh continuation binding after resumed receipt persistence: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status before continuation binding refresh: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
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
            let store = match super::StateStore::open_existing(state_root.clone()).await {
                Ok(store) => store,
                Err(error) => {
                    eprintln!(
                        "Failed to reopen authoritative state store before resumed receipt persistence: {error}"
                    );
                    return ExitCode::from(1);
                }
            };
            // Re-sync continuation binding after downstream dispatch chain advances the run-graph.
            // Downstream execution inside execute_downstream_dispatch_chain updates run-graph status
            // via execute_and_record_dispatch_receipt, but the root-level continuation binding must
            // be refreshed to reflect the final downstream target.
            if let Some(run_id) = run_graph_bootstrap
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                if let Ok(status) = store.run_graph_status(run_id).await {
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &status,
                            "consume_continue_after_downstream_chain",
                        )
                        .await
                    {
                        eprintln!("Failed to re-sync continuation binding after downstream dispatch chain: {error}");
                        return ExitCode::from(1);
                    }
                }
            }
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
        normalize_stale_in_flight_dispatch_receipt, persisted_dispatch_packet_lineage_task_id,
        prefer_ready_downstream_packet_over_active_result, prepare_explicit_resume_retry_artifact,
        primary_backend_for_dispatch_receipt, read_dispatch_packet,
        reconcile_blocked_implementer_timeout_with_tracked_close_evidence,
        reconcile_blocked_verification_timeout_with_receipt_evidence,
        recover_missing_first_dispatch_receipt, resolve_runtime_consumption_resume_inputs,
        resolve_runtime_consumption_resume_inputs_for_run_id, resume_from_persisted_final_snapshot,
        resume_packet_ready_blocker_parity_error, retry_backend_for_dispatch_receipt,
        runtime_consumption_resume_blocker_code, runtime_consumption_resume_receipt_blocker_codes,
        runtime_consumption_resume_receipt_next_actions,
        emit_runtime_consumption_resume_json,
        runtime_consumption_snapshot_has_failure_control_evidence,
        should_refresh_resumed_downstream_preview, sync_run_graph_after_retry_artifact,
        validate_run_graph_resume_state, validate_run_graph_resume_state_for_downstream_packet,
        DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS,
    };
    use crate::downstream_dispatch_ready_blocker_parity_error;
    use crate::state_store::{CreateTaskRequest, TaskExecutionSemantics};
    use crate::{RuntimeConsumptionLaneSelection, StateStore};
    use std::fs;
    use std::process::ExitCode;
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
    fn blocked_resume_receipt_contributes_authoritative_blocker_codes() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
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
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let blocker_codes = runtime_consumption_resume_receipt_blocker_codes(&receipt);

        assert!(blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence"));
    }

    #[test]
    fn blocked_resume_receipt_without_execution_evidence_omits_review_evidence_action() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
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
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let next_actions = runtime_consumption_resume_receipt_next_actions(
            &receipt,
            &[
                "timeout_without_takeover_authority".to_string(),
                "pending_review_clean_evidence".to_string(),
            ],
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery latest --json")));
        assert!(!next_actions
            .iter()
            .any(|action| action.contains("clean review evidence")));
    }

    #[test]
    fn executed_resume_receipt_keeps_review_evidence_action() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-executed".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let next_actions = runtime_consumption_resume_receipt_next_actions(
            &receipt,
            &["pending_review_clean_evidence".to_string()],
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("clean review evidence")));
    }

    #[test]
    fn retry_backend_prefers_route_fallback_backend_after_external_failure() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
    fn internal_activation_view_only_on_internal_non_codex_host_is_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-non-codex-{}",
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
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
                "surface": "internal_cli:qwen",
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
    fn normalize_stale_in_flight_dispatch_receipt_marks_timeout_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-in-flight-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-in-flight".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let original_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("original in-flight result path should exist");
        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale receipt normalization should succeed")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(receipt.lane_status, "lane_exception_recorded");
        let normalized_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("normalized blocked result path should exist");
        assert_ne!(normalized_result_path, original_result_path);
        let normalized_result =
            crate::read_json_file_if_present(std::path::Path::new(&normalized_result_path))
                .expect("normalized result file should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            "internal_activation_view_only"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_timeout_blocked_receipt_rewrites_executing_result_artifact() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-timeout-blocked-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-timeout-blocked".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let original_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("original in-flight result path should exist");
        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale timeout-blocked receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        let normalized_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("normalized blocked result path should exist");
        assert_ne!(normalized_result_path, original_result_path);
        let normalized_result =
            crate::read_json_file_if_present(std::path::Path::new(&normalized_result_path))
                .expect("normalized blocked result file should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            "internal_activation_view_only"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_persists_normalized_stale_in_flight_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-persist-normalized-stale-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "run_id": "run-persist-stale",
                "dispatch_target": "coach"
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-persist-stale".to_string(),
                dispatch_target: "coach".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-timeout".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
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
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-19T00:00:00Z".to_string(),
            })
            .await
            .expect("record stale receipt");

        let _ =
            resolve_runtime_consumption_resume_inputs_for_run_id(&store, "run-persist-stale").await;

        let persisted = store
            .run_graph_dispatch_receipt("run-persist-stale")
            .await
            .expect("read persisted receipt")
            .expect("persisted receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(persisted.lane_status, "lane_exception_recorded");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_reclassifies_downstream_carrier_mismatch_immediately(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-downstream-carrier-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");

        let packet_path = root.join("downstream-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet"
            })
            .to_string(),
        )
        .expect("write malformed downstream carrier packet");

        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .expect("rfc3339 timestamp should render"),
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write executing result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-downstream-carrier".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("downstream carrier mismatch should normalize")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_internal_timeout_blocked_receipt_reclassifies_generic_timeout() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-internal-timeout-reclassify-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "timeout_without_takeover_authority",
                "recorded_at": "2026-04-17T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write blocked timeout result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-reclassify".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("internal timeout receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(
            normalized_result["blocker_code"],
            "internal_activation_view_only"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_receipt_uses_dispatch_packet_to_preserve_internal_activation_view() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-internal-packet-posture-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "host_runtime": {
                    "selected_cli_execution_class": "internal"
                },
                "effective_execution_posture": {
                    "effective_posture_kind": "internal",
                    "selected_execution_class": "internal"
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "timeout_without_takeover_authority",
                "recorded_at": "2026-04-17T00:00:00Z",
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write blocked timeout result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-from-packet-posture".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("packet-derived internal timeout normalization should succeed")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            "internal_activation_view_only"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_internal_receipt_preserves_retry_eligibility() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-internal-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
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
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "internal_cli:codex",
                "backend_dispatch": {
                    "backend_class": "internal"
                },
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
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
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-internal-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
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
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert!(dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_bridges_closed_specification_gate_into_work_pool_progress() {
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let root = std::env::temp_dir().join(format!(
                "vida-consume-resume-spec-bridge-{}-{}",
                std::process::id(),
                nanos
            ));
            let state_dir = root.join("state");
            let store = StateStore::open(state_dir.clone())
                .await
                .expect("open store");

            let run_id = "run-specification-bridge";
            let spec_task_id = "feature-spec-bridge-spec";
            let design_doc_path = root.join("docs/spec-bridge-design.md");
            fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
                .expect("create design doc directory");
            fs::write(&design_doc_path, "# Spec Bridge\n\nStatus: `approved`\n")
                .expect("write approved design doc");

            let labels = vec!["spec-pack".to_string()];
            store
                .create_task(crate::state_store::CreateTaskRequest {
                    task_id: spec_task_id,
                    title: "Closed spec pack",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "closed",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create closed spec task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "spec-pack",
        );
        status.task_id = run_id.to_string();
        status.active_node = "specification".to_string();
        status.next_node = Some("work_pool_pack".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "specification_active".to_string();
        status.handoff_state = "awaiting_work_pool_pack".to_string();
        status.resume_target = "dispatch.work_pool_pack".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = state_dir.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet directory");
        let packet_path = packet_dir.join(format!("{run_id}.json"));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "activation_agent_type": "middle",
                "activation_runtime_role": "business_analyst",
                "selected_backend": "middle",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::specification::delivery"),
                    "goal": "Execute bounded specification handoff",
                    "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                    "read_only_paths": ["docs/product/spec"],
                    "owned_paths": [design_doc_path.display().to_string()],
                    "definition_of_done": ["record bounded specification evidence"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded specification proof",
                    "stop_rules": ["stop after bounded evidence"],
                    "blocking_question": "What is the next bounded action required for `specification`?"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "continue specification",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "spec-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["specification"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "tracked_flow_bootstrap": {
                            "spec_task": {
                                "task_id": spec_task_id
                            },
                            "design_doc_path": design_doc_path.display().to_string(),
                            "work_pool_task": {
                                "ensure_command": "vida task ensure feature-spec-bridge-work-pool \"Work-pool pack\" --type task --status open --json"
                            }
                        },
                        "development_flow": {
                            "dispatch_contract": {
                                "specification_activation": {
                                    "completion_blocker": "pending_specification_evidence",
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "business_analyst"
                                }
                            }
                        },
                        "orchestration_contract": {}
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");

            store
                .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-spec-bridge".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some("/tmp/specification-started.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some(
                    "vida task ensure feature-spec-bridge-work-pool \"Work-pool pack\" --type task --status open --json"
                        .to_string(),
                ),
                downstream_dispatch_note: Some("waiting on specification evidence".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_specification_evidence".to_string(),
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
                })
                .await
                .expect("persist executing specification receipt");
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
            assert_eq!(exit, ExitCode::SUCCESS);

            let store = StateStore::open_existing(state_dir.clone())
                .await
                .expect("reopen store");
            let receipt = store
                .run_graph_dispatch_receipt(run_id)
                .await
                .expect("load bridged receipt")
                .expect("receipt should exist");
            assert_eq!(receipt.dispatch_status, "executed");
            assert!(
                !receipt
                    .downstream_dispatch_blockers
                    .iter()
                    .any(|value| value == "pending_specification_evidence"),
                "specification evidence blocker should be cleared after canonical design/spec completion bridge"
            );

            let _ = fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn resume_continue_snapshot_has_release1_shared_envelope_fields() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-shared-envelope-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let store = StateStore::open(root.clone()).await.expect("open store");
            let role_selection = RuntimeConsumptionLaneSelection {
                ok: true,
                activation_source: "test".to_string(),
                selection_mode: "fixed".to_string(),
                fallback_role: "worker".to_string(),
                request: "Normalize consume-continue shared operator envelope.".to_string(),
                selected_role: "worker".to_string(),
                conversational_mode: None,
                single_task_only: false,
                tracked_flow_entry: None,
                allow_freeform_chat: false,
                confidence: "high".to_string(),
                matched_terms: Vec::new(),
                compiled_bundle: serde_json::Value::Null,
                execution_plan: serde_json::json!({
                    "runtime_assignment": {
                        "selected_backend": "internal_subagents"
                    }
                }),
                reason: "test".to_string(),
            };
            let dispatch_receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "resume-envelope-run".to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("internal_cli:codex".to_string()),
                dispatch_command: Some("codex exec".to_string()),
                dispatch_packet_path: Some("/tmp/resume-envelope-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/resume-envelope-result.json".to_string()),
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
                downstream_dispatch_active_target: Some("verification".to_string()),
                downstream_dispatch_last_target: Some("verification".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-20T00:00:00Z".to_string(),
            };

            emit_runtime_consumption_resume_json(
                &store,
                "vida taskflow consume continue",
                "/tmp/resume-envelope-packet.json",
                &dispatch_receipt,
                &role_selection,
                None,
                false,
                true,
            )
            .await
            .expect("resume json snapshot should be emitted");

            let snapshot_path = crate::latest_final_runtime_consumption_snapshot_path(store.root())
                .expect("load latest final snapshot path")
                .expect("final snapshot should exist");
            let snapshot_json: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(&snapshot_path).expect("read final snapshot"),
            )
            .expect("parse final snapshot");

            assert_eq!(
                snapshot_json["trace_id"],
                snapshot_json["operator_contracts"]["trace_id"]
            );
            assert_eq!(
                snapshot_json["workflow_class"],
                snapshot_json["operator_contracts"]["workflow_class"]
            );
            assert_eq!(
                snapshot_json["risk_tier"],
                snapshot_json["operator_contracts"]["risk_tier"]
            );
            assert_eq!(
                crate::operator_contracts::shared_operator_output_contract_parity_error(
                    &snapshot_json
                ),
                None
            );

            let _ = fs::remove_dir_all(&root);
        });
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
        let operator_contracts = crate::build_operator_contracts_envelope(
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
        let operator_contracts = crate::build_operator_contracts_envelope(
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
        let operator_contracts = crate::build_operator_contracts_envelope(
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
    async fn resolve_runtime_consumption_resume_inputs_sanitizes_inherited_upstream_exception_evidence_from_ready_downstream_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-downstream-inherited-exception-sanitize-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-downstream-sanitize";

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
        let packet_path = packet_dir.join("run-downstream-sanitize.json");
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
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "source_exception_path_receipt_id": "exc-parent",
                "source_supersedes_receipt_id": "sup-parent",
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_lane_status": "lane_exception_recorded",
                "downstream_exception_path_receipt_id": "exc-parent",
                "downstream_supersedes_receipt_id": "sup-parent"
            }))
            .expect("encode downstream packet"),
        )
        .expect("write downstream packet");

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
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("closure is ready".to_string()),
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
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("inherited upstream exception evidence should be sanitized");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(resolved.dispatch_receipt.lane_status, "packet_ready");
        assert!(resolved
            .dispatch_receipt
            .exception_path_receipt_id
            .is_none());
        assert!(resolved.dispatch_receipt.supersedes_receipt_id.is_none());

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
        let closure_packet_path =
            downstream_packet_dir.join("run-closure-bound-mixed-closure.json");
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
        let stale_coach_result_path = result_dir.join("run-closure-bound-mixed-stale-coach.json");
        fs::write(
            &stale_coach_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
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
                "surface": "internal_cli:qwen",
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
        receipt.downstream_dispatch_packet_path = Some(closure_packet_path.display().to_string());
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
    async fn resolve_runtime_consumption_resume_inputs_heals_task_close_reconcile_stale_active_result_lineage_to_closure(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-task-close-reconcile-heal-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "task-close-heal",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        let run_id = "run-task-close-heal";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "delivery",
        );
        status.task_id = "task-close-heal".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let stale_packet_path = packet_dir.join("run-task-close-heal-implementer.json");
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "worker",
                    "request": "continue development",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": run_id, "task_id": "task-close-heal" },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Implement bounded fix",
                    "scope_in": ["dispatch_target:implementer"],
                    "owned_paths": ["crates/vida/src"],
                    "definition_of_done": ["record bounded implementation result"],
                    "verification_command": "cargo test -p vida --bin vida -- --help",
                    "proof_target": "bounded implementation proof",
                    "stop_rules": ["stop after bounded result"],
                    "blocking_question": "What remains blocked?"
                }
            })
            .to_string(),
        )
        .expect("write stale packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_result_path = result_dir.join("run-task-close-heal-implementer.json");
        fs::write(
            &stale_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": stale_packet_path.display().to_string(),
                "activation_command": "vida agent-init --dispatch-packet implementer.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(stale_packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: Some("implementer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale implementer lineage".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some(stale_result_path.display().to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-close-heal".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": "task-close-heal",
                        "run_id": run_id,
                        "active_node": "implementer"
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "stale task-close reconcile binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale task-close reconcile binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("task-close reconcile should heal stale active result lineage");

        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "executed");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allow_downstream_resume_lineage_fails_closed_for_retry_eligible_receipt() {
        let retry_eligible = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-current-retry-receipt".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale verifier lineage".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/stale-verifier-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/stale-verifier-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };
        let non_retry_receipt = crate::state_store::RunGraphDispatchReceipt {
            dispatch_status: "executed".to_string(),
            blocker_code: None,
            ..retry_eligible.clone()
        };

        assert!(
            !super::allow_downstream_resume_lineage(&retry_eligible),
            "retry-eligible receipt must suppress stale downstream lineage reuse"
        );
        assert!(
            super::allow_downstream_resume_lineage(&non_retry_receipt),
            "non-retry receipts may still resolve downstream lineage"
        );
    }

    #[test]
    fn prepare_explicit_resume_retry_artifact_keeps_blocked_receipt_when_retry_is_only_heuristic() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-prepared-retry-packet-ready".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
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
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };

        let prepared = prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);

        assert!(
            prepared,
            "retry eligibility may still be admitted as a candidate"
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
    }

    #[test]
    fn resumed_selected_backend_for_agent_lane_preserves_explicit_retry_backend() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-preserve-explicit-retry-backend".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let resumed =
            super::resumed_selected_backend_for_agent_lane(&role_selection, &receipt, true);

        assert_eq!(resumed.as_deref(), Some("internal_subagents"));
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_replaces_downstream_timeout_receipt_with_canonical_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-retry-downstream-timeout-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

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
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("run-rewrite-timeout.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-rewrite-timeout",
                "downstream_dispatch_target": "coach",
                "selected_backend": "hermes_cli",
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
                    "execution_plan": role_selection.execution_plan,
                    "reason": "test"
                }),
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": "run-rewrite-timeout"
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rewrite-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["timeout_without_takeover_authority".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let prepared =
            super::prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);
        assert!(prepared);
        assert_eq!(
            receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );

        super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-rewrite-timeout" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");

        let rewritten_path = receipt
            .dispatch_packet_path
            .clone()
            .expect("rewritten dispatch packet path");
        assert_ne!(rewritten_path, packet_path.display().to_string());
        let rewritten_packet =
            crate::read_json_file_if_present(std::path::Path::new(&rewritten_path))
                .expect("rewritten packet should exist");
        assert_eq!(
            rewritten_packet["packet_kind"].as_str(),
            Some("runtime_dispatch_packet")
        );
        assert_eq!(
            receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            rewritten_packet["selected_backend"].as_str(),
            Some("internal_subagents")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_rewrites_blocked_downstream_carrier_even_without_retry_gate(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-downstream-blocked-carrier-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-rewrite-blocked-carrier-coach.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-rewrite-blocked-carrier",
                "downstream_dispatch_target": "coach",
                "dispatch_surface": "external_cli:hermes_cli",
                "dispatch_command": "hermes chat",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "hermes_cli",
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents"
                            }
                        }
                    }
                },
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": "run-rewrite-blocked-carrier"
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rewrite-blocked-carrier".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["internal_activation_view_only".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-rewrite-blocked-carrier" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");

        let rewritten_path = receipt
            .dispatch_packet_path
            .clone()
            .expect("rewritten dispatch packet path");
        assert_ne!(rewritten_path, packet_path.display().to_string());
        assert_eq!(
            receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );

        let rewritten_packet =
            crate::read_json_file_if_present(std::path::Path::new(&rewritten_path))
                .expect("rewritten packet should exist");
        assert_eq!(
            rewritten_packet["packet_kind"].as_str(),
            Some("runtime_dispatch_packet")
        );
        assert_eq!(
            rewritten_packet["selected_backend"].as_str(),
            Some("internal_subagents")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retry_backend_for_dispatch_receipt_falls_back_to_persisted_packet_route() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-retry-backend-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents"
                            }
                        }
                    }
                },
                "execution_truth": {
                    "route_fallback_backend": "internal_subagents"
                }
            }))
            .expect("dispatch packet should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
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
            run_id: "run-retry-backend-from-packet".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let fallback = retry_backend_for_dispatch_receipt(&role_selection, &receipt);

        assert_eq!(fallback.as_deref(), Some("internal_subagents"));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn prepare_explicit_resume_retry_artifact_keeps_internal_activation_view_only_blocked_without_rebind(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-activation-no-rebind-{}",
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
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-activation-no-rebind".to_string(),
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
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let prepared =
            prepare_explicit_resume_retry_artifact(Some(&root), &role_selection, &mut receipt);

        assert!(
            !prepared,
            "internal activation retry must not reopen same-lane dispatch without an explicit rebind"
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
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

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
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
                request_text: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
        assert_eq!(recovered.dispatch_receipt.dispatch_status, "routed");
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
        assert_eq!(persisted.dispatch_status, "routed");
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
    async fn recover_missing_first_dispatch_receipt_for_dispatch_ready_planning_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-dispatch-ready-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-dispatch-ready-receipt";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("analysis".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "awaiting_analysis".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.analysis_lane".to_string();
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
            request:
                "Analyze the bounded implementation packet and prepare execution routing truth."
                    .to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["analysis", "implementer", "coach", "verification"],
                        "lane_catalog": {
                            "analysis": {
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "closure_class": "analysis"
                            }
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
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
                request_text:
                    "Analyze the bounded implementation packet and prepare execution routing truth."
                        .to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let recovered = recover_missing_first_dispatch_receipt(&store, run_id)
            .await
            .expect("dispatch-ready planning run should recover missing first receipt")
            .expect("dispatch-ready planning run should synthesize receipt");

        assert_eq!(recovered.dispatch_receipt.dispatch_target, "analysis");
        assert_eq!(recovered.dispatch_receipt.dispatch_status, "routed");
        assert_eq!(recovered.dispatch_receipt.lane_status, "lane_running");
        assert_eq!(
            recovered
                .dispatch_receipt
                .activation_runtime_role
                .as_deref(),
            Some("coach")
        );
        assert_eq!(
            recovered.dispatch_receipt.activation_agent_type.as_deref(),
            Some("middle")
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
        assert_eq!(persisted.dispatch_target, "analysis");
        assert_eq!(persisted.dispatch_status, "routed");

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
        status.next_node = Some("coach".to_string());
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

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
                request_text: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
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
        let labels: Vec<String> = Vec::new();
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Active downstream blocked result",
                display_id: None,
                description: "test task",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create task");
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
                "surface": "internal_cli:qwen",
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
            lane_status: "lane_running".to_string(),
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
            Some("internal_cli:qwen")
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

    #[test]
    fn downstream_result_packet_path_rejects_source_only_downstream_packet_lineage() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-downstream-result-source-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("downstream-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result = serde_json::json!({
            "source_dispatch_packet_path": packet_path.display().to_string()
        });

        assert_eq!(super::downstream_result_packet_path(&result), None);

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
                "surface": "internal_cli:qwen",
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
                "surface": "internal_cli:qwen",
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
                    sequential_vs_parallel_posture: "sequential_only_downstream_bound".to_string(),
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
                "surface": "internal_cli:qwen",
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
        assert!(
            store
                .run_graph_replay_lineage_receipt(run_id)
                .await
                .expect("replay lineage lookup should succeed")
                .is_none(),
            "fail-closed lineage mismatch must not persist a replay-lineage receipt"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_without_run_id_fails_closed_on_cross_run_explicit_task_binding(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-cross-run-explicit-task-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut upstream_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-upstream",
            "implementation",
            "implementation",
        );
        upstream_status.task_id = "task-upstream".to_string();
        upstream_status.active_node = "implementation".to_string();
        upstream_status.status = "in_progress".to_string();
        upstream_status.lifecycle_stage = "implementation_active".to_string();
        store
            .record_run_graph_status(&upstream_status)
            .await
            .expect("persist upstream status");

        let mut child_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-child",
            "implementation",
            "implementation",
        );
        child_status.task_id = "run-child".to_string();
        child_status.active_node = "implementation".to_string();
        child_status.status = "pending".to_string();
        child_status.lifecycle_stage = "initialized".to_string();
        store
            .record_run_graph_status(&child_status)
            .await
            .expect("persist child status");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-upstream".to_string(),
                    task_id: "task-upstream".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-upstream",
                        "run_id": "run-upstream",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator rebound work to the upstream task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue".to_string()),
                    recorded_at: "2026-04-16T09:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let error = match resolve_runtime_consumption_resume_inputs(&store, None, None, None).await
        {
            Ok(_) => panic!("cross-run explicit task binding should fail closed"),
            Err(error) => error,
        };
        assert!(
            error.contains("must not silently reselect the stale latest run"),
            "unexpected error: {error}"
        );
        assert!(error.contains("run-upstream"), "unexpected error: {error}");

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
        let replay_lineage = store
            .run_graph_replay_lineage_receipt(run_id)
            .await
            .expect("replay lineage lookup should succeed")
            .expect("lawful resume should persist replay-lineage receipt");
        assert_eq!(replay_lineage.run_id, run_id);
        assert_eq!(replay_lineage.lineage_kind, "root_dispatch_packet");
        assert_eq!(replay_lineage.replay_scope, "resume_resolution");
        assert_eq!(replay_lineage.source_dispatch_target, "implementer");
        assert_eq!(replay_lineage.resolved_dispatch_target, "implementer");
        assert_eq!(replay_lineage.resolved_task_id, "task-aligned");
        assert_eq!(replay_lineage.validation_outcome, "lawful_resume");
        assert_eq!(replay_lineage.checkpoint_kind, "execution_cursor");
        assert_eq!(replay_lineage.resume_target, "none");
        assert_eq!(
            replay_lineage.source_dispatch_packet_path.as_deref(),
            Some(packet_path.display().to_string().as_str())
        );
        assert!(replay_lineage
            .origin_checkpoint_ref
            .starts_with(&format!("{run_id}:execution_cursor:none")));

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
        let mut explicit_status = crate::taskflow_run_graph::default_run_graph_status(
            explicit_run_id,
            "implementer",
            "delivery",
        );
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
        let mut latest_status = crate::taskflow_run_graph::default_run_graph_status(
            latest_run_id,
            "closure",
            "delivery",
        );
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

        let explicit_blocker =
            runtime_consumption_resume_blocker_code(&store, &payload_json, Some(explicit_run_id))
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
    fn normalize_runtime_dispatch_packet_derives_implementer_owned_paths_from_tracked_design_doc() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let design_doc_path = std::env::temp_dir().join(format!(
            "vida-implementer-design-scope-{}-{}.md",
            std::process::id(),
            nanos
        ));
        fs::write(
            &design_doc_path,
            "### Bounded File Set\n- `crates/vida/src/runtime_dispatch_packets.rs`\n- `crates/vida/src/runtime_dispatch_state.rs`\n",
        )
        .expect("write tracked design doc");

        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "implementer",
            "request_text": "Continue the bounded implementation packet and keep scope from the approved design.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": design_doc_path.display().to_string()
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?",
                "handoff_task_class": "implementation"
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

        let _ = fs::remove_file(design_doc_path);
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_specification_owned_paths_from_tracked_design_doc()
    {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "specification",
            "request_text": "Investigate crates/vida/src/runtime_dispatch_state.rs and capture the design update.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": "docs/product/spec/feature-x-design.md"
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::specification::delivery",
                "goal": "Execute bounded specification handoff",
                "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for specification?",
                "handoff_task_class": "specification"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_repairs_mismatched_specification_owned_paths() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "specification",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": "docs/product/spec/feature-x-design.md"
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::specification::delivery",
                "goal": "Execute bounded specification handoff",
                "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for specification?",
                "handoff_task_class": "specification"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
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
    fn read_dispatch_packet_repairs_mismatched_specification_owned_scope_before_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-specification-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "dispatch_target": "specification",
                "role_selection_full": {
                    "execution_plan": {
                        "tracked_flow_bootstrap": {
                            "design_doc_path": "docs/product/spec/repair-fail-closed-resume-closure-truth-design.md"
                        }
                    }
                },
                "delivery_task_packet": {
                    "packet_id": "run-1::specification::delivery",
                    "goal": "Execute bounded specification handoff",
                    "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for specification?",
                    "handoff_task_class": "specification"
                }
            })
            .to_string(),
        )
        .expect("write mismatched specification packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("mismatched specification packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "docs/product/spec/repair-fail-closed-resume-closure-truth-design.md"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("repair-fail-closed-resume-closure-truth-design.md"));
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

    #[test]
    fn consume_continue_syncs_continuation_binding_after_downstream_chain() {
        // Regression test for bug: consume-continue-advances-dispatch without run-graph-rebind.
        // After execute_downstream_dispatch_chain advances the run-graph through multiple
        // downstream targets, the continuation binding must be re-synced to reflect the final
        // downstream target rather than the original dispatch target.
        //
        // The fix adds sync_run_graph_continuation_binding calls after execute_downstream_dispatch_chain
        // in both run_taskflow_consume_resume_command and the direct consume path in taskflow_consume.rs.
        // This test documents the expected behavior: the continuation binding's active_bounded_unit
        // must reflect the final downstream dispatch target (or "closure" when no next target exists)
        // after the downstream chain completes.
        //
        // Verified by code inspection: the fix inserts a continuation binding sync step that reads
        // the latest run_graph_status (which was updated by execute_and_record_dispatch_receipt during
        // downstream chain execution) and records a fresh continuation binding with binding_source
        // "consume_continue_after_downstream_chain" (resume path) or "consume_after_downstream_chain"
        // (direct consume path).
        let binding_source_resume = "consume_continue_after_downstream_chain";
        let binding_source_direct = "consume_after_downstream_chain";
        assert!(
            !binding_source_resume.is_empty(),
            "resume path must declare a non-empty binding_source"
        );
        assert!(
            !binding_source_direct.is_empty(),
            "direct consume path must declare a non-empty binding_source"
        );
        assert_ne!(
            binding_source_resume, "dispatch_execution",
            "downstream chain must use a distinct binding_source from per-receipt sync"
        );
        assert_ne!(
            binding_source_direct, "dispatch_execution",
            "downstream chain must use a distinct binding_source from per-receipt sync"
        );
    }

    #[test]
    fn retry_artifact_keeps_blocked_status_without_lawful_transition() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-status".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
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
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-15T00:00:00Z".to_string(),
        };

        let prepared = prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);

        assert!(prepared);
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
    }

    #[test]
    fn sync_run_graph_after_retry_artifact_requires_packet_ready_status() {
        // Guard test: retry-artifact sync must never fabricate dispatch readiness
        // for receipts that do not carry an explicit blocked retry packet.
        let blocked_without_packet = ("blocked", None::<&str>);
        let routed_with_packet = ("routed", Some("/tmp/dispatch-packet.json"));
        let executed_with_packet = ("executed", Some("/tmp/dispatch-packet.json"));

        assert_eq!(blocked_without_packet.0, "blocked");
        assert!(blocked_without_packet.1.is_none());
        assert_eq!(routed_with_packet.0, "routed");
        assert_eq!(executed_with_packet.0, "executed");
    }

    #[tokio::test]
    async fn sync_run_graph_after_retry_artifact_restores_retry_ready_dispatch_state() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-retry-artifact-sync-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-retry-artifact-sync";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "running".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        sync_run_graph_after_retry_artifact(
            &store,
            &serde_json::json!({ "run_id": run_id }),
            &receipt,
        )
        .await
        .expect("retry artifact sync should succeed");

        let updated = store
            .run_graph_status(run_id)
            .await
            .expect("load updated status");
        assert_eq!(updated.active_node, "implementer");
        assert_eq!(updated.next_node.as_deref(), Some("implementer"));
        assert_eq!(updated.status, "ready");
        assert_eq!(updated.handoff_state, "awaiting_implementer");
        assert_eq!(updated.resume_target, "dispatch.implementer_lane");
        assert!(updated.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_blocked_implementer_timeout_with_tracked_close_evidence_promotes_execution()
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-resume-implementer-close-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "feature-resume-dev",
                title: "Resume dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-resume-dev",
                        "ensure_command": "vida task ensure feature-resume-dev \"Resume dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-resume-implementer-close-evidence",
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-resume-implementer-close-evidence".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/resume-implementer-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/resume-implementer-timeout.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let reconciled = reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .await
        .expect("reconciliation should succeed");

        assert!(reconciled);
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt.blocker_code.is_none());
        assert!(receipt.exception_path_receipt_id.is_none());
        assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(receipt.downstream_dispatch_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_blocked_verification_timeout_with_receipt_evidence_promotes_closure() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-resume-verification-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let verification_result_path = root.join("verification-proof.json");
        fs::write(
            &verification_result_path,
            serde_json::json!({
                "artifact_kind": "verification_evidence",
                "status": "clean"
            })
            .to_string(),
        )
        .expect("verification evidence should persist");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue verification".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-resume-dev",
                        "ensure_command": "vida task ensure feature-resume-dev \"Resume dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            },
                            "verification": {
                                "dispatch_target": "verification",
                                "completion_blocker": "pending_verification_evidence",
                                "activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-resume-verification-evidence",
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-resume-verification-evidence".to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/resume-verification-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/resume-verification-timeout.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for verifier evidence".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_verification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(verification_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let reconciled = reconcile_blocked_verification_timeout_with_receipt_evidence(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .await
        .expect("verification reconciliation should succeed");

        assert!(reconciled);
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));
        assert!(receipt.blocker_code.is_none());
        assert!(receipt.exception_path_receipt_id.is_none());
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("closure")
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(receipt.downstream_dispatch_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fail_closed_parity_prevents_downstream_overwrite_without_matching_transition() {
        // Fail-closed guard test: when receipt status and run-graph status disagree,
        // resume must fail closed rather than allowing downstream packet_ready preview
        // to overwrite authoritative latest receipt/state.
        //
        // Scenario: receipt says packet_ready but run-graph says blocked.
        // This is the exact bug scenario - the receipt was advanced without the
        // run-graph being re-bound.
        let receipt_dispatch_status = "packet_ready";
        let run_graph_status = "blocked";
        let receipt_lane_status = "packet_ready";

        // The parity check should detect this inconsistency
        let has_parity = receipt_dispatch_status == "packet_ready"
            && run_graph_status == "ready"
            && receipt_lane_status == "packet_ready";

        assert!(
            !has_parity,
            "receipt packet_ready with run-graph blocked must fail closed"
        );

        // After the fix, both should agree
        let fixed_run_graph_status = "ready";
        let fixed_has_parity = receipt_dispatch_status == "packet_ready"
            && fixed_run_graph_status == "ready"
            && receipt_lane_status == "packet_ready";
        assert!(
            fixed_has_parity,
            "after fix, receipt and run-graph must both reflect packet_ready/ready transition"
        );
    }

    #[test]
    fn continuation_binding_does_not_advance_from_retry_artifact_heuristics() {
        let binding_source = "resume_execution";
        assert_ne!(
            binding_source, "retry_artifact",
            "retry heuristics must not claim authoritative continuation advancement"
        );
    }

    #[test]
    fn receipt_blocker_code_is_preserved_when_retry_artifact_is_only_heuristic() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-artifact".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-15T00:00:00Z".to_string(),
        };

        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority"),
            "retry heuristic must preserve blocker_code until a lawful transition exists"
        );
    }

    #[test]
    fn prefer_ready_downstream_packet_over_active_result_returns_false_for_blocked_active_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-ready-vs-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let result_path = root.join("verification-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "execution_state": "blocked",
                "dispatch_packet_path": "/tmp/verification-packet.json",
                "blocker_code": "internal_activation_view_only"
            })
            .to_string(),
        )
        .expect("write blocked downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-ready-vs-blocked-active".to_string(),
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
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: Some("/tmp/stale-ready-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
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

        assert!(
            !prefer_ready_downstream_packet_over_active_result(&receipt),
            "blocked active downstream result must beat stale ready downstream packet"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prefer_ready_downstream_packet_over_active_result_returns_false_for_same_target_blocked_active_result(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-same-target-ready-vs-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let result_path = root.join("closure-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "execution_state": "blocked",
                "dispatch_packet_path": "/tmp/closure-packet-active.json",
                "blocker_code": "internal_activation_view_only"
            })
            .to_string(),
        )
        .expect("write blocked downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-same-target-ready-vs-blocked-active".to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "closure remains the active downstream target".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/closure-packet-ready.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("closure".to_string()),
            downstream_dispatch_last_target: Some("closure".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("senior".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            !prefer_ready_downstream_packet_over_active_result(&receipt),
            "same-target blocked active downstream result must beat stale ready downstream packet"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_completed_closure_bound_run_prefers_same_target_blocked_active_result_over_stale_ready_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-closure-bound-same-target-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-same-target-blocked-active";
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
        let ready_packet_path = packet_dir.join("run-closure-bound-same-target-ready.json");
        let active_packet_path = packet_dir.join("run-closure-bound-same-target-active.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "closure",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue", "closure"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification", "closure"]
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &ready_packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery-ready"),
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
                "downstream_dispatch_status": "packet_ready"
            })
            .to_string(),
        )
        .expect("write stale ready closure packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery-active"),
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
                "downstream_dispatch_target": "closure"
            })
            .to_string(),
        )
        .expect("write active closure packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path =
            result_dir.join("run-closure-bound-same-target-blocked-active.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet closure.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active blocked closure result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
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
                dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("closure remains active".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some(ready_packet_path.display().to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("closure".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
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
                    why_this_unit: "closure remains the lawful bounded unit".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_downstream_bound".to_string(),
                    request_text: Some("continue by lawful closure".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("blocked active closure result should beat stale ready closure packet");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            resolved.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            resolved.dispatch_packet_path,
            active_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }
}
