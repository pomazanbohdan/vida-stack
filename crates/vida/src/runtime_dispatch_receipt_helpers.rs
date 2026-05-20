use std::path::Path;

pub(crate) fn dispatch_result_has_external_dispatch_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> bool {
    receipt
        .dispatch_surface
        .as_deref()
        .is_some_and(|value| value.starts_with("external_cli:"))
        || result["surface"]
            .as_str()
            .is_some_and(|value| value.starts_with("external_cli:"))
        || result["backend_dispatch"]["backend_class"].as_str() == Some("external_cli")
}

pub(crate) fn stale_in_flight_dispatch_preserves_internal_activation_view(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> bool {
    if dispatch_result_has_external_dispatch_evidence(receipt, result) {
        return false;
    }

    result_backend_class_is_internal(result)
        || receipt
            .selected_backend
            .as_deref()
            .is_some_and(|value| value == "internal_subagents" || value.starts_with("internal_"))
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

pub(crate) fn dispatch_packet_indicates_internal_activation_view(
    dispatch_packet_path: Option<&str>,
    result: &serde_json::Value,
) -> bool {
    let Some(packet) = dispatch_packet_from_receipt_or_result(dispatch_packet_path, result) else {
        return false;
    };

    packet["host_runtime"]["selected_cli_execution_class"].as_str() == Some("internal")
        || packet["effective_execution_posture"]["effective_posture_kind"].as_str()
            == Some("internal")
        || packet["mixed_posture"]["effective_posture_kind"].as_str() == Some("internal")
        || packet["effective_execution_posture"]["selected_execution_class"].as_str()
            == Some("internal")
}

pub(crate) fn dispatch_packet_uses_downstream_carrier(
    dispatch_packet_path: Option<&str>,
    result: &serde_json::Value,
) -> bool {
    let Some(packet) = dispatch_packet_from_receipt_or_result(dispatch_packet_path, result) else {
        return false;
    };

    packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        == Some("runtime_downstream_dispatch_packet")
}

fn dispatch_packet_from_receipt_or_result(
    dispatch_packet_path: Option<&str>,
    result: &serde_json::Value,
) -> Option<serde_json::Value> {
    let packet_path = dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            result
                .get("source_dispatch_packet_path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })?;

    crate::read_json_file_if_present(Path::new(packet_path))
}

fn result_backend_class_is_internal(result: &serde_json::Value) -> bool {
    let result_selected_backend_class = result["route_policy"]["selected_backend_class"]
        .as_str()
        .or_else(|| result["mixed_posture"]["selected_backend_class"].as_str())
        .or_else(|| result["effective_execution_posture"]["selected_backend_class"].as_str());

    backend_class_is_internal(result_selected_backend_class)
}

fn backend_class_is_internal(backend_class: Option<&str>) -> bool {
    backend_class.is_some_and(|value| matches!(value.trim(), "internal" | "internal_cli"))
}
