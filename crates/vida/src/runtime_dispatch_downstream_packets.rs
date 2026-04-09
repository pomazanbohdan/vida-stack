use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use crate::runtime_dispatch_packet_text::{runtime_packet_prompt, runtime_tracked_flow_packet};
use crate::runtime_dispatch_packets::{
    runtime_coach_review_packet, runtime_delivery_task_packet, runtime_escalation_packet,
    runtime_execution_block_packet, runtime_verifier_proof_packet,
};
use crate::{
    derive_lane_status, dispatch_contract_lane, downstream_activation_fields,
    validate_runtime_dispatch_packet_contract, RuntimeConsumptionLaneSelection,
};

pub(crate) fn downstream_dispatch_packet_body(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: Option<&Path>,
) -> serde_json::Value {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let downstream_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .unwrap_or_default();
    let (
        downstream_dispatch_kind,
        _downstream_dispatch_surface,
        activation_agent_type,
        activation_runtime_role,
    ) = if downstream_target.is_empty() {
        (
            receipt.dispatch_kind.clone(),
            receipt.dispatch_surface.clone(),
            receipt.activation_agent_type.clone(),
            receipt.activation_runtime_role.clone(),
        )
    } else {
        downstream_activation_fields(role_selection, downstream_target)
    };
    let handoff_runtime_role = activation_runtime_role
        .as_deref()
        .or(receipt.activation_runtime_role.as_deref())
        .unwrap_or(role_selection.selected_role.as_str());
    let packet_template_kind = if downstream_target.is_empty() {
        "delivery_task_packet".to_string()
    } else {
        crate::runtime_dispatch_state::runtime_dispatch_packet_kind(
            &role_selection.execution_plan,
            downstream_target,
            &downstream_dispatch_kind,
        )
    };
    let activation_command = packet_path
        .and_then(|path| path.to_str())
        .map(crate::runtime_dispatch_state::agent_init_command_for_packet_path);
    let handoff_task_class = crate::runtime_dispatch_state::runtime_packet_handoff_task_class(
        downstream_target,
        handoff_runtime_role,
    );
    let closure_class = dispatch_contract_lane(&role_selection.execution_plan, downstream_target)
        .and_then(|lane| lane["closure_class"].as_str())
        .unwrap_or("implementation");
    let delivery_task_packet = runtime_delivery_task_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &role_selection.request,
    );
    let execution_block_packet = runtime_execution_block_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    serde_json::json!({
        "packet_kind": "runtime_downstream_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet
        } else {
            serde_json::Value::Null
        },
        "execution_block_packet": if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
        "coach_review_packet": if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &receipt.run_id,
                downstream_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &receipt.run_id,
                downstream_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
        "tracked_flow_packet": if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(role_selection, &receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &receipt.run_id,
            downstream_target,
            handoff_runtime_role,
            &role_selection.request,
            &role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": receipt.recorded_at,
        "run_id": receipt.run_id,
        "source_dispatch_target": receipt.dispatch_target,
        "source_dispatch_status": receipt.dispatch_status,
        "source_lane_status": receipt.lane_status,
        "source_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "source_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "source_blocker_code": receipt.blocker_code,
        "downstream_dispatch_target": receipt.downstream_dispatch_target,
        "downstream_dispatch_command": activation_command.or_else(|| receipt.downstream_dispatch_command.clone()),
        "downstream_dispatch_note": receipt.downstream_dispatch_note,
        "downstream_dispatch_ready": receipt.downstream_dispatch_ready,
        "downstream_dispatch_blockers": receipt.downstream_dispatch_blockers,
        "downstream_dispatch_status": receipt.downstream_dispatch_status,
        "downstream_lane_status": receipt
            .downstream_dispatch_status
            .as_deref()
            .map(|status| {
                derive_lane_status(
                    status,
                    receipt.supersedes_receipt_id.as_deref(),
                    receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string()
            }),
        "downstream_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "downstream_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "downstream_dispatch_result_path": receipt.downstream_dispatch_result_path,
        "downstream_dispatch_active_target": receipt.downstream_dispatch_active_target,
        "activation_agent_type": activation_agent_type.clone(),
        "activation_runtime_role": activation_runtime_role.clone(),
        "selected_backend": activation_agent_type.or_else(|| receipt.selected_backend.clone()),
        "host_runtime": crate::runtime_dispatch_state::runtime_host_execution_contract_for_root(&project_root),
        "role_selection_full": role_selection,
        "run_graph_bootstrap": run_graph_bootstrap,
        "orchestration_contract": role_selection.execution_plan["orchestration_contract"],
    })
}

pub(crate) fn write_runtime_downstream_dispatch_packet_at(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let body = downstream_dispatch_packet_body(
        role_selection,
        run_graph_bootstrap,
        receipt,
        Some(packet_path),
    );
    validate_runtime_dispatch_packet_contract(&body, "Runtime downstream dispatch packet")?;
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch packet: {error}"))?;
    std::fs::write(packet_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch packet: {error}"))?;
    Ok(())
}

pub(crate) fn write_runtime_downstream_dispatch_packet(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    let Some(target) = receipt.downstream_dispatch_target.as_deref() else {
        return Ok(None);
    };
    let packet_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-packets");
    std::fs::create_dir_all(&packet_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-packets directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", receipt.run_id));
    write_runtime_downstream_dispatch_packet_at(
        &packet_path,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    let _ = target;
    Ok(Some(packet_path.display().to_string()))
}
