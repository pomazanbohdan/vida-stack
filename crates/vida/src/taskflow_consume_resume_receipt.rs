pub(crate) fn blocker_codes(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    let blocked_evidence_present = matches!(
        dispatch_receipt.dispatch_status.as_str(),
        "blocked" | "failed"
    ) || matches!(
        dispatch_receipt.lane_status.as_str(),
        "lane_blocked" | "lane_failed"
    ) || dispatch_receipt
        .downstream_dispatch_status
        .as_deref()
        .is_some_and(|status| matches!(status, "blocked" | "failed"));
    if let Some(blocker_code) = dispatch_receipt
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    let downstream_blockers_apply = !dispatch_receipt.downstream_dispatch_blockers.is_empty()
        || !dispatch_receipt.downstream_dispatch_ready
        || dispatch_receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(|status| matches!(status, "blocked" | "failed"));
    if matches!(
        dispatch_receipt.dispatch_status.as_str(),
        "blocked" | "failed"
    ) || matches!(
        dispatch_receipt.lane_status.as_str(),
        "lane_blocked" | "lane_failed"
    ) || downstream_blockers_apply
    {
        blocker_codes.extend(
            dispatch_receipt
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    if blocker_codes
        .iter()
        .any(|code| code == "configured_backend_dispatch_failed")
    {
        blocker_codes.push(
            crate::contract_profile_adapter::blocker_code_str(
                crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
            )
            .to_string(),
        );
    }
    if blocker_codes.is_empty()
        && dispatch_receipt.dispatch_kind == "agent_lane"
        && dispatch_receipt.dispatch_status == "routed"
        && !dispatch_receipt.downstream_dispatch_ready
    {
        blocker_codes.push("open_delegated_cycle".to_string());
    }
    let normalized = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    if normalized.is_empty() && blocked_evidence_present {
        vec![
            crate::contract_profile_adapter::blocker_code_str(
                crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
            )
            .to_string(),
        ]
    } else {
        normalized
    }
}

pub(crate) fn ready_handoff_status_supersedes_blocked_dispatch_receipt(
    status: Option<&crate::state_store::RunGraphStatus>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if crate::runtime_dispatch_receipt_helpers::dispatch_receipt_downstream_blockers_superseded_by_ready_handoff(
        status,
        dispatch_receipt,
    ) {
        return true;
    }
    let Some(status) = status else {
        return false;
    };
    if status.run_id != dispatch_receipt.run_id
        || !status.status.eq_ignore_ascii_case("ready")
        || !status.recovery_ready
        || !status.resume_target.starts_with("dispatch.")
        || status.active_node == dispatch_receipt.dispatch_target
    {
        return false;
    }

    crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_exception_takeover_continuation_evidence(
        dispatch_receipt,
        Some(&status.run_id),
    )
}

pub(crate) fn next_actions(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    blocker_codes: &[String],
) -> Vec<String> {
    if blocker_codes.is_empty() {
        return Vec::new();
    }

    let mut next_actions = Vec::new();
    next_actions.push(
        "Inspect the latest recovery projection with `vida taskflow recovery latest`.".to_string(),
    );
    let current_lane_completed_without_blocker =
        dispatch_receipt.dispatch_status == "executed" && dispatch_receipt.blocker_code.is_none();
    if current_lane_completed_without_blocker
        && blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence")
    {
        next_actions.push(
            "Record the missing clean review evidence before activating the downstream verification lane."
                .to_string(),
        );
    }
    crate::release1_operator_output::canonical_next_action_entries(&serde_json::json!(next_actions))
        .unwrap_or_else(|| next_actions.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{
        blocker_codes, next_actions, ready_handoff_status_supersedes_blocked_dispatch_receipt,
    };
    use crate::state_store::{RunGraphDispatchReceipt, RunGraphStatus};

    fn receipt() -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "developer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_routed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
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
            selected_backend: None,
            policy_bundle_ref: None,
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn blocker_codes_normalize_blocked_receipt_and_ignore_blank_downstream_entries() {
        let mut receipt = receipt();
        receipt.dispatch_status = "failed".to_string();
        receipt.blocker_code = Some("configured_backend_dispatch_failed".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec![
            "pending_review_clean_evidence".to_string(),
            "  ".to_string(),
        ];

        let codes = blocker_codes(&receipt);

        assert!(codes.contains(&"pending_review_clean_evidence".to_string()));
        assert!(
            codes.contains(
                &crate::contract_profile_adapter::blocker_code_str(
                    crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
                )
                .to_string()
            )
        );
        assert!(!codes.iter().any(|code| code.trim().is_empty()));
    }

    #[test]
    fn next_actions_preserve_recovery_and_clean_review_guidance() {
        let mut receipt = receipt();
        receipt.dispatch_status = "executed".to_string();
        let actions = next_actions(&receipt, &["pending_review_clean_evidence".to_string()]);

        assert_eq!(
            actions[0],
            "inspect the latest recovery projection with `vida taskflow recovery latest`."
        );
        assert!(actions.iter().any(|action| action.contains(
            "record the missing clean review evidence before activating the downstream verification lane"
        )));
    }

    #[test]
    fn ready_handoff_requires_matching_status_and_exception_receipt_evidence() {
        let mut receipt = receipt();
        assert!(!ready_handoff_status_supersedes_blocked_dispatch_receipt(
            None, &receipt
        ));

        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("receipt-0".to_string());
        let mut status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "test".to_string(),
            active_node: "reviewer".to_string(),
            next_node: Some("reviewer".to_string()),
            status: "ready".to_string(),
            route_task_class: "test".to_string(),
            selected_backend: "internal".to_string(),
            lane_id: "lane-1".to_string(),
            lifecycle_stage: "dispatch".to_string(),
            policy_gate: "open".to_string(),
            handoff_state: "ready".to_string(),
            context_state: "open".to_string(),
            checkpoint_kind: "run".to_string(),
            resume_target: "dispatch.reviewer".to_string(),
            recovery_ready: true,
        };

        assert!(ready_handoff_status_supersedes_blocked_dispatch_receipt(
            Some(&status),
            &receipt,
        ));
        status.run_id = "other-run".to_string();
        assert!(!ready_handoff_status_supersedes_blocked_dispatch_receipt(
            Some(&status),
            &receipt,
        ));
    }
}
