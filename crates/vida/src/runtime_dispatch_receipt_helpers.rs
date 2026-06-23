use std::path::Path;

pub(crate) fn dispatch_fields_are_materialization_only_blocked_task_ensure(
    dispatch_status: &str,
    dispatch_surface: Option<&str>,
    dispatch_target: &str,
    blocker_code: Option<&str>,
) -> bool {
    dispatch_status == "blocked"
        && dispatch_surface == Some("vida task ensure")
        && matches!(dispatch_target, "work-pool-pack" | "dev-pack")
        && blocker_code.is_none_or(|code| code == "internal_activation_view_only")
}

pub(crate) fn dispatch_summary_is_materialization_only_blocked_task_ensure(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> bool {
    dispatch_fields_are_materialization_only_blocked_task_ensure(
        &summary.dispatch_status,
        summary.dispatch_surface.as_deref(),
        &summary.dispatch_target,
        summary.blocker_code.as_deref(),
    )
}

pub(crate) fn dispatch_receipt_is_materialization_only_blocked_task_ensure(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    dispatch_fields_are_materialization_only_blocked_task_ensure(
        &receipt.dispatch_status,
        receipt.dispatch_surface.as_deref(),
        &receipt.dispatch_target,
        receipt.blocker_code.as_deref(),
    )
}

pub(crate) fn stored_dispatch_is_materialization_only_blocked_task_ensure(
    receipt: &crate::state_store::RunGraphDispatchReceiptStored,
) -> bool {
    dispatch_fields_are_materialization_only_blocked_task_ensure(
        &receipt.dispatch_status,
        receipt.dispatch_surface.as_deref(),
        &receipt.dispatch_target,
        receipt.blocker_code.as_deref(),
    )
}

pub(crate) fn dispatch_summary_has_clean_ready_downstream_handoff(
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    expected_run_id: Option<&str>,
) -> bool {
    receipt.is_some_and(|receipt| {
        expected_run_id.is_some_and(|run_id| receipt.run_id == run_id)
            && receipt.dispatch_status == "executed"
            && receipt.blocker_code.is_none()
            && receipt.downstream_dispatch_ready
            && receipt.downstream_dispatch_blockers.is_empty()
            && receipt
                .downstream_dispatch_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"))
            && receipt
                .downstream_dispatch_target
                .as_deref()
                .map(str::trim)
                .is_some_and(|target| !target.is_empty())
            && (receipt.downstream_dispatch_target.as_deref() == Some("closure")
                || receipt
                    .downstream_dispatch_packet_path
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty()))
            && downstream_dispatch_command_is_executable(
                receipt.downstream_dispatch_target.as_deref(),
                receipt.downstream_dispatch_command.as_deref(),
            )
    })
}

pub(crate) fn dispatch_receipt_has_clean_ready_downstream_handoff(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && receipt.dispatch_status == "executed"
        && receipt.blocker_code.is_none()
        && receipt.downstream_dispatch_ready
        && receipt.downstream_dispatch_blockers.is_empty()
        && receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"))
        && receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .is_some_and(|target| !target.is_empty())
        && (receipt.downstream_dispatch_target.as_deref() == Some("closure")
            || receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|path| !path.is_empty()))
        && downstream_dispatch_command_is_executable(
            receipt.downstream_dispatch_target.as_deref(),
            receipt.downstream_dispatch_command.as_deref(),
        )
}

fn downstream_dispatch_command_is_executable(target: Option<&str>, command: Option<&str>) -> bool {
    let Some(target) = target.map(str::trim).filter(|target| !target.is_empty()) else {
        return false;
    };
    if target == "closure" {
        return true;
    }
    let Some(command) = command.map(str::trim).filter(|command| !command.is_empty()) else {
        return false;
    };
    command.starts_with("vida agent-init")
        || (matches!(target, "work-pool-pack" | "dev-pack")
            && command.starts_with("vida task ensure"))
}

pub(crate) fn dispatch_summary_has_clean_completed_lane(
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    expected_run_id: Option<&str>,
) -> bool {
    receipt.is_some_and(|receipt| {
        expected_run_id.is_some_and(|run_id| receipt.run_id == run_id)
            && receipt.dispatch_status == "executed"
            && receipt.lane_status == "lane_completed"
            && receipt.blocker_code.is_none()
            && receipt.downstream_dispatch_blockers.is_empty()
    })
}

pub(crate) fn dispatch_receipt_has_pre_execution_packet_ready(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    dispatch_fields_have_pre_execution_packet_ready(
        &receipt.run_id,
        expected_run_id,
        &receipt.dispatch_kind,
        &receipt.dispatch_status,
        Some(receipt.lane_status.as_str()),
        receipt.blocker_code.as_deref(),
        receipt.dispatch_packet_path.as_deref(),
        receipt.downstream_dispatch_status.as_deref(),
        &receipt.downstream_dispatch_blockers,
    )
}

pub(crate) fn stored_dispatch_has_pre_execution_packet_ready(
    receipt: &crate::state_store::RunGraphDispatchReceiptStored,
    expected_run_id: Option<&str>,
) -> bool {
    dispatch_fields_have_pre_execution_packet_ready(
        &receipt.run_id,
        expected_run_id,
        &receipt.dispatch_kind,
        &receipt.dispatch_status,
        receipt.lane_status.as_deref(),
        receipt.blocker_code.as_deref(),
        receipt.dispatch_packet_path.as_deref(),
        receipt.downstream_dispatch_status.as_deref(),
        &receipt.downstream_dispatch_blockers,
    )
}

fn dispatch_fields_have_pre_execution_packet_ready(
    run_id: &str,
    expected_run_id: Option<&str>,
    dispatch_kind: &str,
    dispatch_status: &str,
    lane_status: Option<&str>,
    blocker_code: Option<&str>,
    dispatch_packet_path: Option<&str>,
    downstream_dispatch_status: Option<&str>,
    downstream_dispatch_blockers: &[String],
) -> bool {
    expected_run_id.is_none_or(|expected| run_id == expected)
        && dispatch_kind == "agent_lane"
        && dispatch_status == "routed"
        && blocker_code.is_none_or(|value| value.trim().is_empty())
        && matches!(
            lane_status.map(str::trim),
            Some("lane_open") | Some("lane_running") | Some("packet_ready") | None
        )
        && dispatch_packet_path
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && downstream_dispatch_status.is_none()
        && downstream_dispatch_blockers.is_empty()
}

pub(crate) fn dispatch_summary_has_active_exception_takeover(
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn dispatch_summary_has_exception_takeover_continuation_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && matches!(
            receipt.lane_status.as_str(),
            "lane_exception_takeover" | "lane_exception_recorded"
        )
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn dispatch_receipt_has_clean_completed_lane(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && receipt.dispatch_status == "executed"
        && receipt.lane_status == "lane_completed"
        && receipt.blocker_code.is_none()
        && receipt.downstream_dispatch_blockers.is_empty()
}

pub(crate) fn dispatch_receipt_has_active_exception_takeover(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn dispatch_receipt_has_exception_takeover_continuation_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    expected_run_id.is_none_or(|run_id| receipt.run_id == run_id)
        && matches!(
            receipt.lane_status.as_str(),
            "lane_exception_takeover" | "lane_exception_recorded"
        )
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn recovery_summary_is_terminal_retired_runtime_run(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|summary| {
        summary.resume_status == "completed"
            && summary.lifecycle_stage == "closure_complete"
            && !summary.delegation_gate.delegated_cycle_open
            && summary.delegation_gate.blocker_code.is_none()
            && summary.resume_target == "none"
            && summary.task_id == summary.run_id
    })
}

pub(crate) fn recovery_summary_is_reconciled_terminal_retired_runtime_run(
    status: Option<&crate::state_store::RunGraphStatus>,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    status.is_some_and(crate::state_store::RunGraphStatus::is_reconciled_terminal_closure)
        && recovery_summary_is_terminal_retired_runtime_run(recovery)
}

pub(crate) fn dispatch_receipt_downstream_blockers_superseded_by_ready_handoff(
    status: Option<&crate::state_store::RunGraphStatus>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let Some(status) = status else {
        return false;
    };
    dispatch_receipt_downstream_blockers_superseded_by_ready_handoff_fields(
        &status.run_id,
        &status.active_node,
        &status.status,
        status.recovery_ready,
        &status.resume_target,
        receipt,
    )
}

pub(crate) fn exception_takeover_dispatch_blocker_superseded_by_completed_node(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceiptStored,
) -> bool {
    receipt.run_id == status.run_id
        && matches!(
            receipt.lane_status.as_deref(),
            Some("lane_exception_takeover") | Some("lane_exception_recorded")
        )
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && status_has_sealed_completed_node(status)
        && status.active_node == receipt.dispatch_target
}

fn status_has_sealed_completed_node(status: &crate::state_store::RunGraphStatus) -> bool {
    status.status == "completed"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && status.lifecycle_stage.ends_with("_complete")
        && status.handoff_state == "none"
        && status.context_state == "sealed"
        && status.checkpoint_kind == "none"
        && status.resume_target == "none"
        && !status.recovery_ready
}

pub(crate) fn dispatch_receipt_downstream_blockers_superseded_by_ready_handoff_fields(
    run_id: &str,
    active_node: &str,
    status: &str,
    recovery_ready: bool,
    resume_target: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if run_id != receipt.run_id
        || status != "ready"
        || !recovery_ready
        || !resume_target.starts_with("dispatch.")
        || receipt.dispatch_status != "executed"
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || matches!(
            receipt.lane_status.as_str(),
            "lane_blocked" | "lane_failed" | "lane_exception_recorded" | "lane_exception_takeover"
        )
    {
        return false;
    }
    if receipt
        .downstream_dispatch_blockers
        .iter()
        .map(|value| value.trim())
        .any(|value| !value.is_empty() && value == "missing_owned_write_scope")
    {
        return false;
    }
    let Some(downstream_target) = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let downstream_node = downstream_target.replace('-', "_");
    let source_node = receipt.dispatch_target.replace('-', "_");
    if active_node != receipt.dispatch_target
        && active_node != source_node
        && active_node != downstream_node
    {
        return false;
    }
    resume_target == format!("dispatch.{downstream_node}_lane")
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn summary_for(run_id: &str) -> crate::state_store::RunGraphDispatchReceiptSummary {
        crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verifier".to_string()),
            downstream_dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
            downstream_dispatch_note: Some("continue".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: None,
            effective_execution_posture: serde_json::json!({}),
            route_policy: serde_json::json!({}),
            activation_evidence: serde_json::json!({}),
            recorded_at: "2026-06-05T00:00:00Z".to_string(),
        }
    }

    fn receipt_for(run_id: &str) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some("vida task ensure work-pool".to_string()),
            downstream_dispatch_note: Some("continue".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: None,
            recorded_at: "2026-06-05T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn materialization_only_task_ensure_classifier_rejects_preview_as_execution() {
        for target in ["work-pool-pack", "dev-pack"] {
            assert!(
                dispatch_fields_are_materialization_only_blocked_task_ensure(
                    "blocked",
                    Some("vida task ensure"),
                    target,
                    Some("internal_activation_view_only"),
                ),
                "blocked task ensure materialization must be classified for {target}"
            );
            assert!(
                dispatch_fields_are_materialization_only_blocked_task_ensure(
                    "blocked",
                    Some("vida task ensure"),
                    target,
                    None,
                ),
                "legacy materialization receipts without blocker_code still remain materialization-only for {target}"
            );
            assert!(
                !dispatch_fields_are_materialization_only_blocked_task_ensure(
                    "executed",
                    Some("vida task ensure"),
                    target,
                    None,
                ),
                "executed task ensure receipt is no longer preview/materialization-only for {target}"
            );
        }

        assert!(
            !dispatch_fields_are_materialization_only_blocked_task_ensure(
                "blocked",
                Some("vida agent-init"),
                "dev-pack",
                Some("internal_activation_view_only"),
            ),
            "agent-lane activation blockers are handled by the retry/exception gates, not the materialization-only classifier"
        );
        assert!(
            !dispatch_fields_are_materialization_only_blocked_task_ensure(
                "blocked",
                Some("vida task ensure"),
                "closure",
                Some("internal_activation_view_only"),
            ),
            "closure receipts must not be folded into tracked-flow materialization"
        );
        assert!(
            !dispatch_fields_are_materialization_only_blocked_task_ensure(
                "blocked",
                Some("vida task ensure"),
                "dev-pack",
                Some("tool_execution_failed"),
            ),
            "real task ensure failures are not safe to classify as materialization-only"
        );
    }

    fn ready_status_for(run_id: &str) -> crate::state_store::RunGraphStatus {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "spec-pack",
        );
        status.status = "ready".to_string();
        status.recovery_ready = true;
        status.active_node = "specification".to_string();
        status.resume_target = "dispatch.work_pool_pack_lane".to_string();
        status
    }

    #[test]
    fn clean_ready_downstream_requires_matching_run_and_packet_ready() {
        let mut summary = summary_for("run-1");
        assert!(dispatch_summary_has_clean_ready_downstream_handoff(
            Some(&summary),
            Some("run-1")
        ));
        assert!(!dispatch_summary_has_clean_ready_downstream_handoff(
            Some(&summary),
            Some("other-run")
        ));

        summary.downstream_dispatch_status = Some("blocked".to_string());
        assert!(!dispatch_summary_has_clean_ready_downstream_handoff(
            Some(&summary),
            Some("run-1")
        ));
    }

    #[test]
    fn clean_completed_lane_requires_no_blockers() {
        let mut summary = summary_for("run-1");
        assert!(dispatch_summary_has_clean_completed_lane(
            Some(&summary),
            Some("run-1")
        ));

        summary
            .downstream_dispatch_blockers
            .push("handoff_pending".to_string());
        assert!(!dispatch_summary_has_clean_completed_lane(
            Some(&summary),
            Some("run-1")
        ));
    }

    #[test]
    fn pre_execution_packet_ready_requires_routed_agent_lane_and_packet_path() {
        let mut receipt = receipt_for("run-1");
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_status = "routed".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.blocker_code = None;
        receipt.dispatch_packet_path = Some("packet.json".to_string());
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_blockers.clear();

        assert!(dispatch_receipt_has_pre_execution_packet_ready(
            &receipt,
            Some("run-1")
        ));
        assert!(!dispatch_receipt_has_pre_execution_packet_ready(
            &receipt,
            Some("other-run")
        ));

        receipt
            .downstream_dispatch_blockers
            .push("handoff_pending".to_string());
        assert!(!dispatch_receipt_has_pre_execution_packet_ready(
            &receipt,
            Some("run-1")
        ));
    }

    #[test]
    fn active_exception_takeover_requires_matching_run_and_complete_receipt_pair() {
        let mut receipt = receipt_for("run-1");
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());

        assert!(dispatch_receipt_has_active_exception_takeover(
            &receipt,
            Some("run-1")
        ));
        assert!(!dispatch_receipt_has_active_exception_takeover(
            &receipt,
            Some("other-run")
        ));

        receipt.supersedes_receipt_id = None;
        assert!(!dispatch_receipt_has_active_exception_takeover(
            &receipt,
            Some("run-1")
        ));
    }

    #[test]
    fn continuation_exception_takeover_evidence_accepts_recorded_or_active_lane() {
        let mut summary = summary_for("run-1");
        summary.lane_status = "lane_exception_recorded".to_string();
        summary.exception_path_receipt_id = Some("exception-1".to_string());
        summary.supersedes_receipt_id = Some("exception-1".to_string());

        assert!(
            dispatch_summary_has_exception_takeover_continuation_evidence(&summary, Some("run-1"))
        );
        assert!(!dispatch_summary_has_active_exception_takeover(
            &summary,
            Some("run-1")
        ));

        summary.lane_status = "lane_exception_takeover".to_string();
        assert!(
            dispatch_summary_has_exception_takeover_continuation_evidence(&summary, Some("run-1"))
        );

        summary.supersedes_receipt_id = None;
        assert!(
            !dispatch_summary_has_exception_takeover_continuation_evidence(&summary, Some("run-1"))
        );
    }

    #[test]
    fn full_receipt_continuation_exception_evidence_accepts_recorded_or_active_lane() {
        let mut receipt = receipt_for("run-1");
        receipt.lane_status = "lane_exception_recorded".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());

        assert!(
            dispatch_receipt_has_exception_takeover_continuation_evidence(&receipt, Some("run-1"))
        );
        assert!(!dispatch_receipt_has_active_exception_takeover(
            &receipt,
            Some("run-1")
        ));

        receipt.lane_status = "lane_exception_takeover".to_string();
        assert!(
            dispatch_receipt_has_exception_takeover_continuation_evidence(&receipt, Some("run-1"))
        );

        receipt.exception_path_receipt_id = None;
        assert!(
            !dispatch_receipt_has_exception_takeover_continuation_evidence(&receipt, Some("run-1"))
        );
    }

    #[test]
    fn full_receipt_clean_ready_and_completed_lane_require_no_stale_downstream_blockers() {
        let mut receipt = receipt_for("run-1");
        assert!(dispatch_receipt_has_clean_ready_downstream_handoff(
            &receipt,
            Some("run-1")
        ));
        assert!(dispatch_receipt_has_clean_completed_lane(
            &receipt,
            Some("run-1")
        ));

        receipt
            .downstream_dispatch_blockers
            .push("pending_design_finalize".to_string());
        assert!(!dispatch_receipt_has_clean_ready_downstream_handoff(
            &receipt,
            Some("run-1")
        ));
        assert!(!dispatch_receipt_has_clean_completed_lane(
            &receipt,
            Some("run-1")
        ));
    }

    #[test]
    fn ready_status_supersedes_stale_downstream_blockers_for_matching_next_handoff() {
        let mut receipt = receipt_for("run-1");
        receipt
            .downstream_dispatch_blockers
            .push("pending_design_finalize".to_string());
        let status = ready_status_for("run-1");

        assert!(
            dispatch_receipt_downstream_blockers_superseded_by_ready_handoff(
                Some(&status),
                &receipt
            )
        );

        let mut mismatched = status.clone();
        mismatched.resume_target = "dispatch.dev_pack_lane".to_string();
        assert!(
            !dispatch_receipt_downstream_blockers_superseded_by_ready_handoff(
                Some(&mismatched),
                &receipt
            )
        );
    }

    #[test]
    fn ready_status_supersedes_stale_downstream_blockers_when_active_node_is_downstream_target() {
        let mut receipt = receipt_for("run-1");
        receipt
            .downstream_dispatch_blockers
            .push("pending_design_finalize".to_string());
        let mut status = ready_status_for("run-1");
        status.active_node = "work_pool_pack".to_string();

        assert!(
            dispatch_receipt_downstream_blockers_superseded_by_ready_handoff(
                Some(&status),
                &receipt
            ),
            "recovery-ready downstream handoff should supersede stale source-lane blockers"
        );
    }

    fn recovery_summary_for(run_id: &str) -> crate::state_store::RunGraphRecoverySummary {
        crate::state_store::RunGraphRecoverySummary {
            run_id: run_id.to_string(),
            task_id: run_id.to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            handoff_state: "none".to_string(),
            checkpoint_kind: "none".to_string(),
            policy_gate: "closed_task_stale_run_retired".to_string(),
            resume_status: "completed".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                lifecycle_stage: "closure_complete".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                blocker_code: None,
                reporting_pause_gate: "closure_candidate".to_string(),
                continuation_signal: "continue_after_reports".to_string(),
            },
        }
    }

    fn terminal_status_for(run_id: &str) -> crate::state_store::RunGraphStatus {
        crate::state_store::RunGraphStatus {
            run_id: run_id.to_string(),
            task_id: run_id.to_string(),
            task_class: "runtime_diagnostic".to_string(),
            active_node: "closure".to_string(),
            next_node: None,
            status: "completed".to_string(),
            route_task_class: "runtime_diagnostic".to_string(),
            selected_backend: "local".to_string(),
            lane_id: "root".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            policy_gate: "closed_task_stale_run_retired".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        }
    }

    #[test]
    fn completed_same_node_supersedes_exception_takeover_blocker() {
        let mut status = terminal_status_for("run-exception");
        status.task_id = "task-exception".to_string();
        status.active_node = "analyst".to_string();
        status.lifecycle_stage = "analyst_complete".to_string();

        let mut receipt =
            crate::state_store::RunGraphDispatchReceiptStored::from(receipt_for("run-exception"));
        receipt.dispatch_target = "analyst".to_string();
        receipt.lane_status = Some("lane_exception_takeover".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());

        assert!(
            exception_takeover_dispatch_blocker_superseded_by_completed_node(&status, &receipt)
        );

        status.active_node = "developer".to_string();
        assert!(
            !exception_takeover_dispatch_blocker_superseded_by_completed_node(&status, &receipt)
        );
    }

    #[test]
    fn inconsistent_completed_status_does_not_supersede_exception_takeover_blocker() {
        let mut status = terminal_status_for("run-exception");
        status.task_id = "task-exception".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("coach".to_string());
        status.lifecycle_stage = "implementer_active".to_string();
        status.context_state = "open".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;

        let mut receipt =
            crate::state_store::RunGraphDispatchReceiptStored::from(receipt_for("run-exception"));
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = Some("lane_exception_takeover".to_string());
        receipt.blocker_code = Some("operator_exception_takeover".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("superseded-receipt".to_string());

        assert!(
            !exception_takeover_dispatch_blocker_superseded_by_completed_node(&status, &receipt),
            "a resumable or inconsistent completed status must keep the blocked receipt authoritative"
        );
    }

    #[test]
    fn terminal_retired_runtime_run_requires_closed_clear_self_task_recovery() {
        let terminal = recovery_summary_for("vida-scope");
        assert!(recovery_summary_is_terminal_retired_runtime_run(Some(
            &terminal
        )));

        let mut open_cycle = recovery_summary_for("vida-scope");
        open_cycle.delegation_gate.delegated_cycle_open = true;
        assert!(!recovery_summary_is_terminal_retired_runtime_run(Some(
            &open_cycle
        )));

        let mut task_backed = terminal;
        task_backed.task_id = "real-task".to_string();
        assert!(!recovery_summary_is_terminal_retired_runtime_run(Some(
            &task_backed
        )));
    }

    #[test]
    fn reconciled_terminal_retired_runtime_run_requires_retired_policy_gate() {
        let recovery = recovery_summary_for("vida-scope");
        let reconciled_status = terminal_status_for("vida-scope");
        assert!(recovery_summary_is_reconciled_terminal_retired_runtime_run(
            Some(&reconciled_status),
            Some(&recovery)
        ));

        let mut unreconciled_status = reconciled_status;
        unreconciled_status.policy_gate = "not_required".to_string();
        assert!(
            !recovery_summary_is_reconciled_terminal_retired_runtime_run(
                Some(&unreconciled_status),
                Some(&recovery)
            )
        );

        assert!(
            !recovery_summary_is_reconciled_terminal_retired_runtime_run(None, Some(&recovery))
        );
    }
}
