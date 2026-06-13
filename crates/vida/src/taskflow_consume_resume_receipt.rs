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
        vec![crate::contract_profile_adapter::blocker_code_str(
            crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
        )
        .to_string()]
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
    crate::operator_contracts::canonical_next_action_entries(&serde_json::json!(next_actions))
        .unwrap_or_else(|| next_actions.to_vec())
}
