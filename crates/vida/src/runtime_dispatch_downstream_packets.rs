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
    let selected_backend = crate::runtime_dispatch_state::downstream_selected_backend(
        role_selection,
        downstream_target,
        activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    );
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
    let host_runtime =
        crate::runtime_dispatch_state::runtime_host_execution_contract_for_root(&project_root);
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            downstream_target,
            selected_backend.as_deref(),
            activation_agent_type.as_deref(),
            Some(&host_runtime),
            crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(receipt),
        );
    let execution_truth = crate::runtime_dispatch_state::dispatch_execution_route_summary(
        role_selection,
        downstream_target,
        selected_backend.as_deref(),
    );
    let activation_evidence =
        crate::runtime_dispatch_state::dispatch_activation_evidence_summary(receipt);
    let mut body = serde_json::Map::new();
    body.insert(
        "packet_kind".to_string(),
        serde_json::json!("runtime_downstream_dispatch_packet"),
    );
    body.insert(
        "packet_template_kind".to_string(),
        serde_json::json!(packet_template_kind),
    );
    body.insert(
        "delivery_task_packet".to_string(),
        if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "execution_block_packet".to_string(),
        if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "coach_review_packet".to_string(),
        if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &receipt.run_id,
                downstream_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "verifier_proof_packet".to_string(),
        if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &receipt.run_id,
                downstream_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "escalation_packet".to_string(),
        if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "tracked_flow_packet".to_string(),
        if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(role_selection, &receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "prompt".to_string(),
        serde_json::json!(runtime_packet_prompt(
            &receipt.run_id,
            downstream_target,
            handoff_runtime_role,
            &role_selection.request,
            &role_selection.execution_plan["orchestration_contract"],
        )),
    );
    body.insert(
        "recorded_at".to_string(),
        serde_json::json!(receipt.recorded_at),
    );
    body.insert("run_id".to_string(), serde_json::json!(receipt.run_id));
    body.insert(
        "source_dispatch_target".to_string(),
        serde_json::json!(receipt.dispatch_target),
    );
    body.insert(
        "source_dispatch_status".to_string(),
        serde_json::json!(receipt.dispatch_status),
    );
    body.insert(
        "source_lane_status".to_string(),
        serde_json::json!(receipt.lane_status),
    );
    body.insert(
        "source_supersedes_receipt_id".to_string(),
        serde_json::json!(receipt.supersedes_receipt_id),
    );
    body.insert(
        "source_exception_path_receipt_id".to_string(),
        serde_json::json!(receipt.exception_path_receipt_id),
    );
    body.insert(
        "source_blocker_code".to_string(),
        serde_json::json!(receipt.blocker_code),
    );
    body.insert(
        "downstream_dispatch_target".to_string(),
        serde_json::json!(receipt.downstream_dispatch_target),
    );
    body.insert(
        "downstream_dispatch_command".to_string(),
        serde_json::json!(
            activation_command.or_else(|| receipt.downstream_dispatch_command.clone())
        ),
    );
    body.insert(
        "downstream_dispatch_note".to_string(),
        serde_json::json!(receipt.downstream_dispatch_note),
    );
    body.insert(
        "downstream_dispatch_ready".to_string(),
        serde_json::json!(receipt.downstream_dispatch_ready),
    );
    body.insert(
        "downstream_dispatch_blockers".to_string(),
        serde_json::json!(receipt.downstream_dispatch_blockers),
    );
    body.insert(
        "downstream_dispatch_status".to_string(),
        serde_json::json!(receipt.downstream_dispatch_status),
    );
    body.insert(
        "downstream_lane_status".to_string(),
        serde_json::json!(receipt.downstream_dispatch_status.as_deref().map(|status| {
            derive_lane_status(
                status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
            .as_str()
            .to_string()
        })),
    );
    body.insert(
        "downstream_supersedes_receipt_id".to_string(),
        serde_json::json!(receipt.supersedes_receipt_id),
    );
    body.insert(
        "downstream_exception_path_receipt_id".to_string(),
        serde_json::json!(receipt.exception_path_receipt_id),
    );
    body.insert(
        "downstream_dispatch_result_path".to_string(),
        serde_json::json!(receipt.downstream_dispatch_result_path),
    );
    body.insert(
        "downstream_dispatch_active_target".to_string(),
        serde_json::json!(receipt.downstream_dispatch_active_target),
    );
    body.insert(
        "activation_agent_type".to_string(),
        serde_json::json!(activation_agent_type),
    );
    body.insert(
        "activation_runtime_role".to_string(),
        serde_json::json!(activation_runtime_role),
    );
    body.insert(
        "selected_backend".to_string(),
        serde_json::json!(selected_backend),
    );
    body.insert(
        "effective_execution_posture".to_string(),
        effective_execution_posture.clone(),
    );
    body.insert("mixed_posture".to_string(), effective_execution_posture);
    body.insert("route_policy".to_string(), execution_truth.clone());
    body.insert(
        "activation_vs_execution_evidence".to_string(),
        activation_evidence.clone(),
    );
    body.insert(
        "activation_semantics".to_string(),
        activation_evidence["activation_semantics"].clone(),
    );
    body.insert(
        "execution_evidence".to_string(),
        activation_evidence["execution_evidence"].clone(),
    );
    body.insert("execution_truth".to_string(), execution_truth);
    body.insert("activation_evidence".to_string(), activation_evidence);
    body.insert("host_runtime".to_string(), host_runtime);
    body.insert(
        "role_selection_full".to_string(),
        serde_json::to_value(role_selection).expect("role selection should serialize"),
    );
    body.insert(
        "run_graph_bootstrap".to_string(),
        run_graph_bootstrap.clone(),
    );
    body.insert(
        "orchestration_contract".to_string(),
        role_selection.execution_plan["orchestration_contract"].clone(),
    );
    serde_json::Value::Object(body)
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
