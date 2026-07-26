use super::*;
use crate::RuntimeConsumptionLaneSelection;
use crate::release1_contracts::lane_status_has_required_evidence;
use crate::state_store::state_store_task_models::{
    task_has_label, task_is_spec_pack_child, task_is_work_pool_pack_child,
};
use crate::taskflow_run_graph::{
    approval_delegation_transition_kind, clear_run_graph_dispatch_init_fast_cache,
    is_dispatch_resume_handoff_done,
};
use taskflow_authority::run_graph_evidence::{
    RunGraphBlockedSourceLane, RunGraphCompletionEvidence, RunGraphDownstreamPacketEvidence,
    RunGraphReworkEvidence, blocked_source_lane_from_packet_evidence,
    downstream_handoff_ready_from_completion_evidence, normalize_run_graph_node,
    rework_route_from_completion_evidence,
};
use taskflow_authority::run_graph_transition::{
    ReadyRunGraphTransitionInput, RunGraphAuthorityInput, RunGraphDispatchTargetFormat,
    admit_run_graph_transition, ready_run_graph_transition,
};
use taskflow_core::run_graph::model::{
    DispatchReceiptSnapshot as CoreDispatchReceiptSnapshot,
    RunGraphStatusSnapshot as CoreRunGraphStatusSnapshot,
    RunGraphTransitionKind as CoreRunGraphTransitionKind,
};

const MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES: u64 = 4 * 1024 * 1024;

fn run_graph_dispatch_lane_receipt_id(
    run_id: &str,
    dispatch_target: &str,
    dispatch_packet_path: &str,
) -> String {
    use std::hash::{Hash, Hasher};

    fn component(value: &str) -> String {
        let safe: String = value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '-'
                }
            })
            .collect();
        safe.trim_matches('-').chars().take(80).collect::<String>()
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    run_id.hash(&mut hasher);
    dispatch_target.hash(&mut hasher);
    dispatch_packet_path.hash(&mut hasher);
    format!(
        "{}-{}-{:016x}",
        component(run_id),
        component(dispatch_target),
        hasher.finish()
    )
}

fn dispatch_packet_path_matches_receipt(stored: Option<&str>, requested: &str) -> bool {
    let Some(stored) = stored else {
        return false;
    };
    taskflow_host_bridge::host_bridge_packet_paths_equivalent(stored, requested)
}

fn packet_path_has_dot_segment(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    })
}

fn invalid_reconciled_pack_dispatch_packet_error(
    packet_path: &std::path::Path,
    reason: impl Into<String>,
) -> StateStoreError {
    StateStoreError::InvalidTaskRecord {
        reason: format!(
            "Failed to load materialized pack dispatch packet `{}`: {}",
            packet_path.display(),
            reason.into()
        ),
    }
}

fn reconcile_run_graph_status_with_dispatch_receipt(
    status: RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> Result<RunGraphStatus, StateStoreError> {
    reconcile_run_graph_status_with_dispatch_receipt_and_rework_route(status, receipt, None)
}

fn reconcile_run_graph_status_with_dispatch_receipt_and_rework_route(
    mut status: RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
    authorized_rework_route: Option<&crate::runtime_dispatch_result_evidence::DispatchReworkRoute>,
) -> Result<RunGraphStatus, StateStoreError> {
    let Some(receipt) = receipt else {
        return Ok(status);
    };
    let receipt = StateStore::validate_run_graph_dispatch_receipt_contract(receipt.clone())?;
    if terminal_closure_status(&status)
        && status.policy_gate == "historical_closed_task_stale_run_retired"
    {
        return Ok(status);
    }
    if terminal_closure_status_has_explicit_receipt_override(&status, &receipt) {
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    if terminal_closure_supersedes_stale_handoff_receipt_fields(
        &status,
        &receipt.dispatch_kind,
        &receipt.dispatch_status,
        receipt.lane_status.as_deref(),
        receipt.blocker_code.as_deref(),
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    ) {
        if status.policy_gate != "historical_closed_task_stale_run_retired" {
            status.policy_gate = "closed_task_stale_run_retired".to_string();
        }
        return Ok(status);
    }
    let stale_downstream_blockers_are_superseded_by_ready_handoff =
        ready_dispatch_handoff_matches_downstream_receipt(&status, &receipt);
    if active_exception_takeover_receipt_is_behind_status(&status, &receipt) {
        return Ok(status);
    }
    let executed_analysis_missing_owned_scope_handoff = receipt.dispatch_status == "executed"
        && receipt.dispatch_target == "analysis"
        && receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| blocker == "missing_owned_write_scope");
    let pre_execution_packet_ready =
        crate::runtime_dispatch_receipt_helpers::stored_dispatch_has_pre_execution_packet_ready(
            &receipt, None,
        );
    let pre_execution_routed_handoff = receipt.dispatch_status == "routed"
        && receipt.blocker_code.as_deref().is_none_or(str::is_empty)
        && receipt.dispatch_kind == "agent_lane"
        && matches!(
            receipt.lane_status.as_deref(),
            Some("lane_open") | Some("lane_running") | Some("packet_ready") | None
        )
        && receipt.downstream_dispatch_status.is_none();
    let blocked_receipt = matches!(receipt.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(
            receipt.lane_status.as_deref(),
            Some("lane_blocked")
                | Some("lane_failed")
                | Some("lane_exception_recorded")
                | Some("lane_exception_takeover")
        )
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || (!pre_execution_routed_handoff
            && !stale_downstream_blockers_are_superseded_by_ready_handoff
            && !executed_analysis_missing_owned_scope_handoff
            && !receipt.downstream_dispatch_blockers.is_empty());
    let spec_post_design_gate_blocked = receipt.dispatch_status == "executed"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
    let retry_target = receipt.dispatch_target.replace('-', "_");
    let retry_ready_same_lane = receipt.dispatch_kind == "agent_lane"
        && receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("internal_activation_view_only")
        && status.status == "ready"
        && status.active_node == receipt.dispatch_target
        && status.next_node.as_deref() == Some(retry_target.as_str())
        && status.handoff_state == format!("awaiting_{retry_target}")
        && status.resume_target == format!("dispatch.{retry_target}_lane")
        && status.recovery_ready;
    if let Some(rework_route) = rework_route_from_completion_evidence(
        &run_graph_completion_evidence(&receipt, authorized_rework_route),
    ) {
        let rework_target = normalize_run_graph_node(&rework_route.allowed_next_node);
        let rework_policy_gate = rework_route
            .blocker_code
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| receipt.blocker_code.clone())
            .unwrap_or_else(|| "not_required".to_string());
        let transition = ready_transition_input(
            &status,
            receipt.dispatch_target.clone(),
            Some(rework_target.clone()),
            format!("{rework_target}_dispatch_ready"),
            rework_policy_gate,
            "execution_cursor".to_string(),
            RunGraphDispatchTargetFormat::Direct,
            true,
        );
        apply_ready_run_graph_transition(&mut status, transition);
        return Ok(status);
    }
    if blocked_receipt {
        if retry_ready_same_lane {
            if let Some(selected_backend) = receipt
                .selected_backend
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                status.selected_backend = selected_backend.to_string();
            }
            return Ok(status);
        }
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        let retryable_blocked_receipt = blocked_agent_lane_receipt_allows_recovery_retry(&receipt);
        if spec_post_design_gate_blocked {
            let completed_target = receipt
                .downstream_dispatch_last_target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(receipt.dispatch_target.as_str());
            let lifecycle_target = completed_target.replace('-', "_");
            status.active_node = completed_target.to_string();
            status.next_node = None;
            status.lifecycle_stage = format!("{lifecycle_target}_complete");
            status.policy_gate = "not_required".to_string();
            status.handoff_state = "none".to_string();
            status.resume_target = "none".to_string();
            status.context_state = "sealed".to_string();
        } else if let Some(rework_route) = rework_route_from_completion_evidence(
            &run_graph_completion_evidence(&receipt, authorized_rework_route),
        ) {
            let rework_target = normalize_run_graph_node(&rework_route.allowed_next_node);
            let rework_policy_gate = rework_route
                .blocker_code
                .clone()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| receipt.blocker_code.clone())
                .unwrap_or_else(|| "not_required".to_string());
            let transition = ready_transition_input(
                &status,
                receipt.dispatch_target.clone(),
                Some(rework_target.clone()),
                format!("{rework_target}_dispatch_ready"),
                rework_policy_gate,
                "execution_cursor".to_string(),
                RunGraphDispatchTargetFormat::Direct,
                true,
            );
            apply_ready_run_graph_transition(&mut status, transition);
            return Ok(status);
        } else if let Some(blocked_source) =
            blocked_source_lane_from_downstream_dispatch_packet(&receipt)
        {
            let blocked_target = normalize_run_graph_node(&blocked_source.dispatch_target);
            status.active_node = blocked_source.dispatch_target;
            status.next_node = None;
            status.lifecycle_stage = format!("{blocked_target}_blocked");
            status.policy_gate = blocked_source
                .blocker_code
                .unwrap_or_else(|| receipt.blocker_code.clone().unwrap_or_default());
            status.handoff_state = "none".to_string();
            status.resume_target = format!("dispatch.{blocked_target}");
            status.context_state = "sealed".to_string();
            status.recovery_ready = false;
        } else if retryable_blocked_receipt {
            let blocked_target = normalize_run_graph_node(&receipt.dispatch_target);
            status.active_node = receipt.dispatch_target.clone();
            status.next_node = Some(blocked_target.clone());
            status.lifecycle_stage = format!("{blocked_target}_blocked");
            status.policy_gate = receipt.blocker_code.clone().unwrap_or_default();
            status.handoff_state = format!("awaiting_{blocked_target}");
            status.resume_target = format!("dispatch.{blocked_target}");
            status.context_state = "sealed".to_string();
            status.recovery_ready = true;
        } else {
            let blocked_target = normalize_run_graph_node(&receipt.dispatch_target);
            status.active_node = receipt.dispatch_target.clone();
            status.next_node = None;
            status.lifecycle_stage = format!("{blocked_target}_blocked");
            status.handoff_state = "none".to_string();
            status.resume_target = if blocked_agent_lane_receipt_keeps_resume_target(&receipt) {
                format!("dispatch.{blocked_target}")
            } else {
                "none".to_string()
            };
            status.context_state = "sealed".to_string();
        }
        status.checkpoint_kind = "none".to_string();
        status.status = "blocked".to_string();
        if !blocked_agent_lane_receipt_allows_recovery_retry(&receipt) {
            status.recovery_ready = false;
        }
        return Ok(status);
    }
    if pre_execution_packet_ready {
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        // Dispatch-init materializes a packet before execution. Preserve the
        // seeded run-graph cursor (planning -> exact configured node) instead
        // of projecting the receipt's external dispatch alias into state.
        if crate::taskflow_run_graph::is_seeded_dispatch_ready(&status) {
            return Ok(status);
        }
        let dispatch_target = normalize_run_graph_node(&receipt.dispatch_target);
        let mut transition = ready_transition_input(
            &status,
            dispatch_target.clone(),
            Some(dispatch_target.clone()),
            format!("{dispatch_target}_dispatch_ready"),
            "not_required".to_string(),
            "execution_cursor".to_string(),
            RunGraphDispatchTargetFormat::Lane,
            true,
        );
        transition.lane_id = format!("{dispatch_target}_lane");
        apply_ready_run_graph_transition(&mut status, transition);
        return Ok(status);
    }
    let closure_dispatch_completed =
        crate::runtime_dispatch_state::canonical_terminal_closure_dispatch_target(
            &receipt.dispatch_target,
        )
        .is_some()
            && receipt.dispatch_status == "executed"
            && receipt.blocker_code.is_none();
    if closure_dispatch_completed {
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.context_state = "sealed".to_string();
        status.recovery_ready = false;
        return Ok(status);
    }
    let downstream_handoff_ready = downstream_handoff_ready_from_completion_evidence(
        &run_graph_downstream_handoff_evidence(&receipt),
    );
    let downstream_handoff_admitted = run_graph_authority_transition_kind(&status, &receipt)
        == Some(CoreRunGraphTransitionKind::DownstreamReadyHandoff)
        || (downstream_handoff_ready
            && (stale_status_can_accept_downstream_ready_handoff(&status, &receipt)
                || (status.status == "completed"
                    && receipt.dispatch_status == "executed"
                    && matches!(
                        receipt.dispatch_target.as_str(),
                        "work-pool-pack" | "dev-pack"
                    ))));
    if downstream_handoff_admitted && downstream_handoff_ready {
        let completed_target = receipt.dispatch_target.trim();
        let lifecycle_target = normalize_run_graph_node(completed_target);
        let downstream_node = receipt
            .downstream_dispatch_target
            .as_deref()
            .expect("downstream target checked above")
            .replace('-', "_");
        if status.status == "ready"
            && status.recovery_ready
            && status.active_node != receipt.dispatch_target
            && (status.active_node == downstream_node
                || status.next_node.as_deref() == Some(downstream_node.as_str()))
        {
            return Ok(status);
        }
        if let Some(selected_backend) = receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            status.selected_backend = selected_backend.to_string();
        }
        let transition = ready_transition_input(
            &status,
            completed_target.to_string(),
            Some(downstream_node),
            format!("{lifecycle_target}_complete"),
            "not_required".to_string(),
            "execution_cursor".to_string(),
            RunGraphDispatchTargetFormat::Lane,
            true,
        );
        apply_ready_run_graph_transition(&mut status, transition);
        return Ok(status);
    }
    if status.status == "completed" {
        return Ok(status);
    }
    if receipt.dispatch_target == "analysis"
        && receipt.dispatch_status == "executed"
        && status.active_node == "analysis"
    {
        if let Some(next_target) = receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let next_node = normalize_run_graph_node(next_target);
            if status.next_node.is_none() {
                status.next_node = Some(next_node.clone());
            }
            status.status = "ready".to_string();
            status.lifecycle_stage = "analysis_active".to_string();
            if status.policy_gate == "validation_report_required" {
                status.policy_gate = "targeted_verification".to_string();
            }
            let transition = ready_transition_input(
                &status,
                status.active_node.clone(),
                status.next_node.clone(),
                "analysis_active".to_string(),
                status.policy_gate.clone(),
                status.checkpoint_kind.clone(),
                RunGraphDispatchTargetFormat::Lane,
                true,
            );
            apply_ready_run_graph_transition(&mut status, transition);
        }
    }
    Ok(status)
}

fn ready_transition_input(
    status: &RunGraphStatus,
    active_node: String,
    next_node: Option<String>,
    lifecycle_stage: String,
    policy_gate: String,
    checkpoint_kind: String,
    target_format: RunGraphDispatchTargetFormat,
    recovery_ready: bool,
) -> ReadyRunGraphTransitionInput {
    ReadyRunGraphTransitionInput {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node,
        next_node,
        route_task_class: status.route_task_class.clone(),
        selected_backend: status.selected_backend.clone(),
        lane_id: status.lane_id.clone(),
        lifecycle_stage,
        policy_gate,
        checkpoint_kind,
        target_format,
        recovery_ready,
    }
}

fn apply_ready_run_graph_transition(
    status: &mut RunGraphStatus,
    input: ReadyRunGraphTransitionInput,
) {
    let transition = ready_run_graph_transition(input);
    status.run_id = transition.run_id;
    status.task_id = transition.task_id;
    status.task_class = transition.task_class;
    status.active_node = transition.active_node;
    status.next_node = transition.next_node;
    status.status = transition.status;
    status.route_task_class = transition.route_task_class;
    status.selected_backend = transition.selected_backend;
    status.lane_id = transition.lane_id;
    status.lifecycle_stage = transition.lifecycle_stage;
    status.policy_gate = transition.policy_gate;
    status.handoff_state = transition.handoff_state;
    status.context_state = transition.context_state;
    status.checkpoint_kind = transition.checkpoint_kind;
    status.resume_target = transition.resume_target;
    status.recovery_ready = transition.recovery_ready;
}

fn run_graph_authority_transition_kind(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<CoreRunGraphTransitionKind> {
    let decision = admit_run_graph_transition(RunGraphAuthorityInput {
        status: CoreRunGraphStatusSnapshot {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            active_node: status.active_node.clone(),
            next_node: status.next_node.clone(),
            status: status.status.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            handoff_state: status.handoff_state.clone(),
            resume_target: status.resume_target.clone(),
            recovery_ready: status.recovery_ready,
        },
        receipt: Some(CoreDispatchReceiptSnapshot {
            dispatch_target: receipt.dispatch_target.clone(),
            dispatch_status: receipt.dispatch_status.clone(),
            lane_status: receipt.lane_status.clone(),
            supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
            exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_target: receipt.downstream_dispatch_target.clone(),
            downstream_dispatch_blockers: receipt.downstream_dispatch_blockers.clone(),
        }),
        closure: None,
    });
    Some(decision.decision.kind)
}

fn run_graph_completion_evidence(
    receipt: &RunGraphDispatchReceiptStored,
    authorized_rework_route: Option<&crate::runtime_dispatch_result_evidence::DispatchReworkRoute>,
) -> RunGraphCompletionEvidence {
    RunGraphCompletionEvidence {
        dispatch_target: receipt.dispatch_target.clone(),
        dispatch_status: receipt.dispatch_status.clone(),
        blocker_code: receipt.blocker_code.clone(),
        rework: downstream_rework_evidence_from_completion_result(authorized_rework_route),
        source_lane: downstream_packet_evidence_from_receipt(receipt),
        downstream_dispatch_ready: receipt.downstream_dispatch_ready,
        downstream_dispatch_target: receipt.downstream_dispatch_target.clone(),
        downstream_dispatch_blockers: receipt.downstream_dispatch_blockers.clone(),
    }
}

fn run_graph_downstream_handoff_evidence(
    receipt: &RunGraphDispatchReceiptStored,
) -> RunGraphCompletionEvidence {
    RunGraphCompletionEvidence {
        dispatch_target: receipt.dispatch_target.clone(),
        dispatch_status: receipt.dispatch_status.clone(),
        blocker_code: receipt.blocker_code.clone(),
        rework: None,
        source_lane: None,
        downstream_dispatch_ready: receipt.downstream_dispatch_ready,
        downstream_dispatch_target: receipt.downstream_dispatch_target.clone(),
        downstream_dispatch_blockers: receipt.downstream_dispatch_blockers.clone(),
    }
}

fn stale_status_can_accept_downstream_ready_handoff(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    let dispatch_target = receipt.dispatch_target.trim();
    !dispatch_target.is_empty()
        && status.active_node == dispatch_target
        && status.next_node.is_none()
        && matches!(status.status.as_str(), "blocked" | "executing" | "ready")
}

fn downstream_rework_evidence_from_completion_result(
    authorized_rework_route: Option<&crate::runtime_dispatch_result_evidence::DispatchReworkRoute>,
) -> Option<RunGraphReworkEvidence> {
    let route = authorized_rework_route?;
    Some(RunGraphReworkEvidence {
        allowed_next_node: route.allowed_next_node.clone(),
        blocker_code: route.blocker_code.clone(),
    })
}

fn blocked_source_lane_from_downstream_dispatch_packet(
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<RunGraphBlockedSourceLane> {
    if !matches!(
        receipt.dispatch_status.as_str(),
        "bridge_request_pending" | "blocked"
    ) {
        return None;
    }
    let packet = downstream_packet_evidence_from_receipt(receipt)?;
    blocked_source_lane_from_packet_evidence(
        &receipt.dispatch_target,
        &receipt.dispatch_status,
        packet,
    )
}

fn path_is_under_vida_state_dir(path: &std::path::Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    components
        .windows(3)
        .any(|window| window == [".vida", "data", "state"])
}

fn downstream_packet_evidence_from_receipt(
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<RunGraphDownstreamPacketEvidence> {
    let packet_path = receipt.dispatch_packet_path.as_deref()?.trim();
    if packet_path.is_empty() {
        return None;
    }
    let packet_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(packet_path);
    if !path_is_under_vida_state_dir(&packet_path) {
        return None;
    }
    let metadata = std::fs::symlink_metadata(&packet_path).ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES
    {
        return None;
    }
    let raw = std::fs::read_to_string(packet_path).ok()?;
    let packet: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let source_dispatch_target = packet
        .get("source_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if source_dispatch_target == receipt.dispatch_target.trim() {
        return None;
    }
    let source_dispatch_status = packet
        .get("source_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let source_blocker_code = packet
        .get("source_blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let downstream_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let downstream_dispatch_blockers = packet
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .map(|blockers| {
            blockers
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(RunGraphDownstreamPacketEvidence {
        source_dispatch_target: source_dispatch_target.to_string(),
        source_dispatch_status: source_dispatch_status.to_string(),
        source_blocker_code,
        downstream_dispatch_ready: downstream_ready,
        downstream_dispatch_blockers,
    })
}

fn blocked_agent_lane_receipt_keeps_resume_target(receipt: &RunGraphDispatchReceiptStored) -> bool {
    if receipt.dispatch_kind != "agent_lane"
        || !receipt
            .dispatch_result_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    {
        return false;
    }
    let blocker_code = receipt.blocker_code.as_deref().unwrap_or_default();
    !matches!(
        blocker_code,
        "internal_activation_view_only"
            | "host_tool_bridge_adapter_required"
            | crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
    )
}

fn blocked_agent_lane_receipt_allows_recovery_retry(
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    blocked_agent_lane_receipt_keeps_resume_target(receipt)
        && receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("host_bridge_completion_result_blocked")
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
}

fn configured_dispatch_task_identity_field(dispatch_target: &str) -> Option<String> {
    let target = dispatch_target.trim().replace('-', "_");
    let config = crate::config_value_utils::load_project_overlay_yaml().ok()?;
    let groups = crate::config_value_utils::yaml_lookup(
        &config,
        &["run_graph", "dispatch_task_identity", "target_groups"],
    )?;
    groups.as_sequence()?.iter().find_map(|group| {
        let aliases = crate::config_value_utils::yaml_string_list(
            crate::config_value_utils::yaml_lookup(group, &["aliases"]),
        );
        let matches_target = aliases
            .iter()
            .map(|alias| alias.replace('-', "_"))
            .any(|alias| alias == target);
        matches_target.then(|| {
            crate::config_value_utils::yaml_string(crate::config_value_utils::yaml_lookup(
                group,
                &["identity_field"],
            ))
        })?
    })
}

fn dispatch_identity_task_id_for_target(
    identity: &RunGraphDispatchTaskIdentity,
    dispatch_target: &str,
) -> Option<String> {
    let candidate = match configured_dispatch_task_identity_field(dispatch_target).as_deref() {
        Some("spec_task_id") => identity.spec_task_id.as_ref(),
        Some("work_pool_task_id") => identity.work_pool_task_id.as_ref(),
        Some("dev_task_id") => identity.dev_task_id.as_ref(),
        _ => None,
    }
    .or(identity.work_pool_task_id.as_ref())
    .or(identity.spec_task_id.as_ref())
    .or(identity.feature_epic_id.as_ref())?;
    let candidate = candidate.trim();
    (!candidate.is_empty()).then(|| candidate.to_string())
}

fn tracked_flow_materialization_result_passed(
    state_root: Option<&std::path::Path>,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    tracked_flow_materialization_result(state_root, receipt).is_some()
}

#[derive(Debug, Clone)]
struct TrackedFlowMaterializationResult {
    packet_key: String,
    epic_task_id: Option<String>,
    task_id: String,
}

fn tracked_flow_materialization_result(
    state_root: Option<&std::path::Path>,
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<TrackedFlowMaterializationResult> {
    if !stored_receipt_can_reconcile_tracked_flow_materialization(receipt) {
        return None;
    }
    let mut candidate_paths = Vec::new();
    if let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        candidate_paths.push(std::path::PathBuf::from(result_path));
    }
    if let Some(root) = state_root {
        let results_dir = root.join("runtime-consumption").join("dispatch-results");
        if let Ok(entries) = std::fs::read_dir(results_dir) {
            let mut result_paths = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(std::ffi::OsStr::to_str)
                        .is_some_and(|name| {
                            name.starts_with(&format!("{}-", receipt.run_id))
                                && name.ends_with(".json")
                        })
                })
                .collect::<Vec<_>>();
            result_paths.sort();
            candidate_paths.extend(result_paths.into_iter().rev());
        }
    }
    candidate_paths.into_iter().find_map(|result_path| {
        let Ok(result_body) = std::fs::read_to_string(&result_path) else {
            return None;
        };
        tracked_flow_materialization_result_from_body(&result_body, receipt)
    })
}

fn stored_receipt_can_reconcile_tracked_flow_materialization(
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    matches!(receipt.dispatch_status.as_str(), "blocked" | "executed")
        && receipt.dispatch_surface.as_deref() == Some("vida task ensure")
        && matches!(
            receipt.dispatch_target.as_str(),
            "work-pool-pack" | "dev-pack"
        )
        && receipt
            .blocker_code
            .as_deref()
            .is_none_or(|code| code == "internal_activation_view_only")
}

fn tracked_flow_materialization_result_body_passed(
    result_body: &str,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    tracked_flow_materialization_result_from_body(result_body, receipt).is_some()
}

fn tracked_flow_materialization_result_from_body(
    result_body: &str,
    receipt: &RunGraphDispatchReceiptStored,
) -> Option<TrackedFlowMaterializationResult> {
    let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&result_body) else {
        return None;
    };
    if result_json["status"].as_str() != Some("pass")
        || result_json["surface"].as_str() != Some("vida task ensure")
    {
        return None;
    }
    let expected_packet_key = match receipt.dispatch_target.as_str() {
        "work-pool-pack" => "work_pool_task",
        "dev-pack" => "dev_task",
        _ => return None,
    };
    if result_json["packet_key"].as_str() != Some(expected_packet_key) {
        return None;
    }
    let task_id = result_json["task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let task_materialized = result_json["task"]["created"].as_bool() == Some(true)
        || result_json["task"]["reused_existing"].as_bool() == Some(true);
    if !task_materialized {
        return None;
    }
    Some(TrackedFlowMaterializationResult {
        packet_key: expected_packet_key.to_string(),
        epic_task_id: result_json["epic"]["task_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        task_id,
    })
}

fn ready_dispatch_handoff_matches_downstream_receipt(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if status.status != "ready"
        || !status.recovery_ready
        || !is_dispatch_resume_handoff_done(status)
        || receipt.dispatch_status != "executed"
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || matches!(
            receipt.lane_status.as_deref(),
            Some("lane_blocked")
                | Some("lane_failed")
                | Some("lane_exception_recorded")
                | Some("lane_exception_takeover")
        )
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
    if status.active_node != receipt.dispatch_target {
        return false;
    }
    let downstream_node = downstream_target.replace('-', "_");
    status.next_node.as_deref() == Some(downstream_node.as_str())
        && status.handoff_state == format!("awaiting_{downstream_node}")
        && status.resume_target == format!("dispatch.{downstream_node}_lane")
}

fn receipt_has_pending_spec_first_work_pool_handoff_gate(
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == "executed"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        })
}

fn has_receipt_evidence_id(value: Option<&str>) -> bool {
    value.map(str::trim).is_some_and(|value| !value.is_empty())
}

fn terminal_closure_status(status: &RunGraphStatus) -> bool {
    status.is_terminal_closure()
}

fn stale_host_bridge_handoff_receipt_fields(
    dispatch_kind: &str,
    dispatch_status: &str,
    lane_status: Option<&str>,
    blocker_code: Option<&str>,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> bool {
    dispatch_kind == "agent_lane"
        && dispatch_status == "bridge_request_pending"
        && blocker_code.map(str::trim)
            == Some(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::HostToolBridgeAdapterRequired,
            ))
        && matches!(
            lane_status.map(str::trim),
            Some("lane_open" | "lane_running" | "lane_blocked") | None
        )
        && !has_receipt_evidence_id(supersedes_receipt_id)
        && !has_receipt_evidence_id(exception_path_receipt_id)
}

fn terminal_closure_supersedes_stale_handoff_receipt_fields(
    status: &RunGraphStatus,
    dispatch_kind: &str,
    dispatch_status: &str,
    lane_status: Option<&str>,
    blocker_code: Option<&str>,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> bool {
    if !terminal_closure_status(status)
        || has_receipt_evidence_id(supersedes_receipt_id)
        || has_receipt_evidence_id(exception_path_receipt_id)
    {
        return false;
    }
    let blocker_code = blocker_code.map(str::trim);
    let stale_downstream_handoff = dispatch_status == "blocked"
        && blocker_code
            == Some(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
            ));
    let stale_host_bridge_handoff = stale_host_bridge_handoff_receipt_fields(
        dispatch_kind,
        dispatch_status,
        lane_status,
        blocker_code,
        supersedes_receipt_id,
        exception_path_receipt_id,
    );
    stale_downstream_handoff || stale_host_bridge_handoff
}

fn terminal_closure_supersedes_stale_handoff_receipt(
    status: &RunGraphStatus,
    receipt: &mut RunGraphDispatchReceipt,
) -> bool {
    if !terminal_closure_supersedes_stale_handoff_receipt_fields(
        status,
        &receipt.dispatch_kind,
        &receipt.dispatch_status,
        Some(receipt.lane_status.as_str()),
        receipt.blocker_code.as_deref(),
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    ) {
        return false;
    }
    let stale_kind = if receipt.dispatch_status == "bridge_request_pending" {
        "host bridge"
    } else {
        "downstream handoff"
    };
    receipt.dispatch_status = "executed".to_string();
    receipt.lane_status = "lane_completed".to_string();
    receipt.blocker_code = None;
    receipt.downstream_dispatch_target = Some("closure".to_string());
    receipt.downstream_dispatch_note = Some(format!(
        "terminal closure superseded stale {stale_kind} blocker"
    ));
    receipt.downstream_dispatch_ready = false;
    receipt.downstream_dispatch_blockers.clear();
    receipt.downstream_dispatch_status = Some("executed".to_string());
    receipt.downstream_dispatch_active_target = Some("closure".to_string());
    receipt.downstream_dispatch_last_target = Some("closure".to_string());
    true
}

fn terminal_closure_historicalizes_active_exception_takeover_receipt(
    status: &RunGraphStatus,
    receipt: &mut RunGraphDispatchReceipt,
) -> bool {
    if !terminal_closure_status(status)
        || !matches!(
            receipt.lane_status.as_str(),
            "lane_exception_takeover" | "lane_superseded"
        )
        || !has_receipt_evidence_id(receipt.exception_path_receipt_id.as_deref())
        || !has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref())
    {
        return false;
    }

    receipt.dispatch_status = "executed".to_string();
    receipt.lane_status = "lane_completed".to_string();
    receipt.blocker_code = None;
    receipt.exception_path_receipt_id = None;
    receipt.supersedes_receipt_id = None;
    receipt.downstream_dispatch_target = Some("closure".to_string());
    receipt.downstream_dispatch_command = None;
    receipt.downstream_dispatch_note =
        Some("terminal closure historicalized exception takeover".to_string());
    receipt.downstream_dispatch_ready = false;
    receipt.downstream_dispatch_blockers.clear();
    receipt.downstream_dispatch_packet_path = None;
    receipt.downstream_dispatch_status = Some("retired_closed_task_run".to_string());
    receipt.downstream_dispatch_result_path = None;
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_active_target = Some("closure".to_string());
    receipt.downstream_dispatch_last_target = Some("closure".to_string());
    true
}

fn task_status_is_terminal_for_continuation(status: &str) -> bool {
    taskflow_core::task_status_is_closed_like(status)
}

fn terminal_closure_status_has_explicit_receipt_override(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if !terminal_closure_status(status) {
        return false;
    }
    let has_supersession = has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref());
    if !has_supersession {
        return false;
    }
    matches!(
        receipt.lane_status.as_deref(),
        Some("lane_exception_takeover") | Some("lane_superseded")
    )
}

fn stored_receipt_has_active_exception_takeover(receipt: &RunGraphDispatchReceiptStored) -> bool {
    receipt.lane_status.as_deref() == Some("lane_exception_takeover")
        && has_receipt_evidence_id(receipt.exception_path_receipt_id.as_deref())
        && has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref())
}

fn stored_receipt_has_stale_host_bridge_handoff(receipt: &RunGraphDispatchReceiptStored) -> bool {
    stale_host_bridge_handoff_receipt_fields(
        &receipt.dispatch_kind,
        &receipt.dispatch_status,
        receipt.lane_status.as_deref(),
        receipt.blocker_code.as_deref(),
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
}

fn active_exception_takeover_receipt_is_behind_status(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceiptStored,
) -> bool {
    if !stored_receipt_has_active_exception_takeover(receipt) {
        return false;
    }
    if terminal_closure_status(status) && status.resume_target == "none" {
        return true;
    }
    if crate::runtime_dispatch_receipt_helpers::exception_takeover_dispatch_blocker_superseded_by_completed_node(
        status, receipt,
    ) {
        return true;
    }
    let dispatch_target = receipt.dispatch_target.trim();
    let next_node_matches_dispatch_target = status
        .next_node
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        == Some(dispatch_target);
    status.status == "ready"
        && status.recovery_ready
        && status.resume_target.starts_with("dispatch.")
        && status.active_node != dispatch_target
        && !next_node_matches_dispatch_target
}

fn lawful_current_exception_takeover(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> bool {
    receipt.is_some_and(|receipt| {
        stored_receipt_has_active_exception_takeover(receipt)
            && !active_exception_takeover_receipt_is_behind_status(status, receipt)
    })
}

fn receiptless_closed_task_operator_archive_sentinel(status: &RunGraphStatus) -> bool {
    if status.status == "completed"
        || terminal_closure_status(status)
        || [
            status.active_node.as_str(),
            status.next_node.as_deref().unwrap_or_default(),
            status.lifecycle_stage.as_str(),
            status.handoff_state.as_str(),
            status.resume_target.as_str(),
        ]
        .into_iter()
        .any(|value| value.to_ascii_lowercase().contains("exception"))
    {
        return false;
    }
    let Some(next_node) = status
        .next_node
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let delegation_gate = status.delegation_gate();
    delegation_gate.delegated_cycle_open
        && delegation_gate.delegated_cycle_state == "handoff_pending"
        && status.handoff_state == format!("awaiting_{next_node}")
        && matches!(
            status.resume_target.as_str(),
            value if value == format!("dispatch.{next_node}")
                || value == format!("dispatch.{next_node}_lane")
        )
        && status.recovery_ready
}

fn continuation_binding_active_kind(binding: &RunGraphContinuationBinding) -> Option<&str> {
    binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
}

fn continuation_binding_active_node(binding: &RunGraphContinuationBinding) -> Option<&str> {
    binding
        .active_bounded_unit
        .get("active_node")
        .and_then(serde_json::Value::as_str)
}

fn reconcile_continuation_binding_with_exception_takeover_receipt(
    binding: RunGraphContinuationBinding,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> RunGraphContinuationBinding {
    let Some(receipt) = receipt else {
        return binding;
    };
    if binding.run_id != receipt.run_id
        || continuation_binding_active_kind(&binding) != Some("run_graph_task")
        || !stored_receipt_has_active_exception_takeover(receipt)
    {
        return binding;
    }
    let active_node = receipt.dispatch_target.trim();
    if active_node.is_empty() || continuation_binding_active_node(&binding) == Some(active_node) {
        return binding;
    }
    let recorded_at = if receipt.recorded_at.trim().is_empty() {
        binding.recorded_at.clone()
    } else {
        receipt.recorded_at.clone()
    };
    RunGraphContinuationBinding {
        run_id: binding.run_id.clone(),
        task_id: binding.task_id.clone(),
        status: "bound".to_string(),
        active_bounded_unit: serde_json::json!({
            "kind": "run_graph_task",
            "task_id": binding.task_id.clone(),
            "run_id": binding.run_id.clone(),
            "active_node": active_node,
        }),
        binding_source: "latest_run_graph_exception_takeover_dispatch".to_string(),
        why_this_unit: format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}` at node `{}`.",
            binding.task_id, active_node
        ),
        primary_path: "normal_delivery_path".to_string(),
        sequential_vs_parallel_posture: "sequential_only_exception_takeover".to_string(),
        request_text: binding.request_text,
        recorded_at,
    }
}

pub(crate) fn normalize_legacy_downstream_preview_drift(
    mut receipt: RunGraphDispatchReceiptStored,
) -> RunGraphDispatchReceiptStored {
    let active_dispatch_with_upstream_lane_evidence = receipt.dispatch_status != "executed"
        && (has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref())
            || has_receipt_evidence_id(receipt.exception_path_receipt_id.as_deref()));
    let stale_downstream_preview_present = receipt.downstream_dispatch_status.is_some()
        || receipt.downstream_dispatch_ready
        || receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
    if active_dispatch_with_upstream_lane_evidence && stale_downstream_preview_present {
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_note = None;
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_result_path = None;
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_executed_count = 0;
        receipt.downstream_dispatch_active_target = None;
    }
    receipt
}

fn normalize_repairable_in_flight_receipt_lane_status_drift(
    mut receipt: RunGraphDispatchReceiptStored,
) -> (RunGraphDispatchReceiptStored, bool) {
    let Some(raw_lane_status) = receipt
        .lane_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return (receipt, false);
    };
    let Some(canonical_lane_status) = canonical_lane_status_str(raw_lane_status) else {
        return (receipt, false);
    };
    if receipt
        .downstream_dispatch_status
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        return (receipt, false);
    }
    if !matches!(
        receipt.dispatch_status.trim(),
        "packet_ready" | "routed" | "bridge_request_pending" | "executing"
    ) || !matches!(
        canonical_lane_status,
        "packet_ready" | "lane_open" | "lane_running"
    ) || receipt
        .supersedes_receipt_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return (receipt, false);
    }
    let effective_derived_lane_status = normalize_run_graph_lane_status(
        Some(raw_lane_status),
        &receipt.dispatch_status,
        None,
        None,
    );
    if canonical_lane_status == effective_derived_lane_status {
        return (receipt, false);
    }
    receipt.lane_status = Some(effective_derived_lane_status);
    (receipt, true)
}

fn reconcile_run_graph_status_with_closed_task(
    mut status: RunGraphStatus,
    task: Option<&TaskRecord>,
    receipt: Option<&RunGraphDispatchReceiptStored>,
) -> RunGraphStatus {
    let Some(task) = task else {
        return status;
    };
    if !task_status_is_terminal_for_continuation(&task.status) {
        return status;
    }
    if receipt.and_then(|receipt| receipt.lane_status.as_deref()) == Some("lane_exception_recorded")
    {
        if status.active_node == "closure"
            && status.status == "blocked"
            && status.lifecycle_stage == "closure_blocked"
        {
            status.lifecycle_stage = "closure_complete".to_string();
        }
        return status;
    }
    let terminal_closure_status = status.active_node == "closure"
        && matches!(status.status.as_str(), "blocked" | "completed")
        && matches!(
            status.lifecycle_stage.as_str(),
            "closure_blocked" | "closure_complete"
        )
        && status.next_node.is_none()
        && status.handoff_state == "none"
        && status.resume_target == "none";
    if !StateStore::run_graph_status_allows_task_close_closure_binding(&status)
        && !terminal_closure_status
    {
        return status;
    }

    status.status = "completed".to_string();
    if status.active_node == "closure" {
        status.lifecycle_stage = "closure_complete".to_string();
    } else if status.lifecycle_stage != "closure_complete" {
        status.lifecycle_stage = "implementation_complete".to_string();
    }
    status.next_node = None;
    if !StateStore::run_graph_status_is_reconciled_terminal_closure(&status) {
        status.policy_gate = "not_required".to_string();
    }
    status.handoff_state = "none".to_string();
    status.context_state = "sealed".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}

pub(crate) fn project_closed_task_scoped_run_graph_status(
    mut status: RunGraphStatus,
) -> RunGraphStatus {
    status.active_node = "closure".to_string();
    status.next_node = None;
    status.status = "completed".to_string();
    status.lifecycle_stage = "closure_complete".to_string();
    status.policy_gate = "closed_run_archived".to_string();
    status.handoff_state = "none".to_string();
    status.context_state = "sealed".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}

pub(crate) fn requires_memory_governance_enforcement(policy_gate: &str) -> bool {
    let normalized = policy_gate.trim().to_ascii_lowercase();
    normalized.contains("consent")
        || normalized.contains("ttl")
        || normalized.contains("correction")
        || normalized.contains("delete")
        || normalized.contains("deletion")
}

pub(crate) fn handoff_state_links_consent_ttl(handoff_state: &str) -> bool {
    let normalized = handoff_state.trim().to_ascii_lowercase();
    normalized.contains("consent") && normalized.contains("ttl")
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, Clone)]
pub struct RunGraphDelegationGateSummary {
    pub active_node: String,
    pub lifecycle_stage: String,
    pub delegated_cycle_open: bool,
    pub delegated_cycle_state: String,
    pub local_exception_takeover_gate: String,
    pub blocker_code: Option<String>,
    pub reporting_pause_gate: String,
    pub continuation_signal: String,
}

impl RunGraphDelegationGateSummary {
    pub(crate) fn from_status(status: &RunGraphStatus) -> Self {
        let dispatch_ready_resume = status.status == "ready"
            && status.lifecycle_stage.ends_with("_dispatch_ready")
            && status.resume_target.starts_with("dispatch.");
        let terminal_blocked = status.status == "blocked"
            && status.next_node.is_none()
            && status.handoff_state == "blocked"
            && status.resume_target == "none"
            && status.lifecycle_stage.ends_with("_terminal_blocked");
        let handoff_pending = !dispatch_ready_resume
            && !terminal_blocked
            && (status.next_node.is_some()
                || status.handoff_state != "none"
                || status.resume_target != "none");
        let delegated_lane_active = !handoff_pending
            && status.status != "completed"
            && status.active_node != "planning"
            && status.lifecycle_stage.ends_with("_active");
        let delegated_lane_blocked = !handoff_pending
            && status.status == "blocked"
            && status.active_node != "planning"
            && status.lifecycle_stage.ends_with("_blocked")
            && status.policy_gate != "not_required";
        let (delegated_cycle_open, delegated_cycle_state) = if terminal_blocked {
            (false, "terminal_blocked".to_string())
        } else if handoff_pending {
            (true, "handoff_pending".to_string())
        } else if delegated_lane_active {
            (true, "delegated_lane_active".to_string())
        } else if delegated_lane_blocked {
            (true, "delegated_lane_blocked".to_string())
        } else {
            (false, "clear".to_string())
        };
        let local_exception_takeover_gate = if delegated_cycle_open {
            "blocked_open_delegated_cycle".to_string()
        } else {
            "delegated_cycle_clear".to_string()
        };
        let blocker_code = if local_exception_takeover_gate == "blocked_open_delegated_cycle" {
            Some(
                canonical_blocker_code_str(BlockerCode::OpenDelegatedCycle.as_str())
                    .unwrap_or(BlockerCode::OpenDelegatedCycle.as_str())
                    .to_string(),
            )
        } else {
            None
        };
        let reporting_pause_gate = if delegated_cycle_open {
            "non_blocking_only".to_string()
        } else if status.status == "completed" {
            "closure_candidate".to_string()
        } else {
            "continuation_check_required".to_string()
        };
        let continuation_signal = if delegated_cycle_open {
            "continue_routing_non_blocking".to_string()
        } else if status.status == "completed" {
            "continue_after_reports".to_string()
        } else {
            "continuation_check_required".to_string()
        };

        Self {
            active_node: status.active_node.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            delegated_cycle_open,
            delegated_cycle_state,
            local_exception_takeover_gate,
            blocker_code,
            reporting_pause_gate,
            continuation_signal,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "node={} lifecycle={} delegated_cycle_open={} delegated_cycle_state={} local_exception_takeover_gate={} blocker_code={} reporting_pause_gate={} continuation_signal={}",
            self.active_node,
            self.lifecycle_stage,
            self.delegated_cycle_open,
            self.delegated_cycle_state,
            self.local_exception_takeover_gate,
            self.blocker_code.as_deref().unwrap_or("none"),
            self.reporting_pause_gate,
            self.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphRecoverySummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub resume_node: Option<String>,
    pub resume_status: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub recovery_ready: bool,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphRecoverySummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            resume_node: status.next_node,
            resume_status: status.status,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            recovery_ready: status.recovery_ready,
            delegation_gate,
        }
    }

    pub(crate) fn is_terminal_closure(&self) -> bool {
        self.resume_status == "completed"
            && self.lifecycle_stage == "closure_complete"
            && self.active_node == "closure"
            && self
                .resume_node
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            && self.handoff_state == "none"
            && self.checkpoint_kind == "none"
            && self.resume_target == "none"
            && !self.recovery_ready
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} resume_node={} resume_status={} checkpoint={} resume_target={} gate={} handoff={} recovery_ready={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.resume_node.as_deref().unwrap_or("none"),
            self.resume_status,
            self.checkpoint_kind,
            self.resume_target,
            self.policy_gate,
            self.handoff_state,
            self.recovery_ready,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphCheckpointSummary {
    pub run_id: String,
    pub task_id: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

impl RunGraphCheckpointSummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            recovery_ready: status.recovery_ready,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} checkpoint={} resume_target={} recovery_ready={}",
            self.run_id,
            self.task_id,
            self.checkpoint_kind,
            self.resume_target,
            self.recovery_ready
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphDispatchReceiptSummary {
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    pub lane_status: String,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub dispatch_kind: String,
    pub dispatch_surface: Option<String>,
    pub dispatch_command: Option<String>,
    pub dispatch_packet_path: Option<String>,
    pub dispatch_result_path: Option<String>,
    pub blocker_code: Option<String>,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_command: Option<String>,
    pub downstream_dispatch_note: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
    pub downstream_dispatch_packet_path: Option<String>,
    pub downstream_dispatch_status: Option<String>,
    pub downstream_dispatch_result_path: Option<String>,
    pub downstream_dispatch_trace_path: Option<String>,
    pub downstream_dispatch_executed_count: u32,
    pub downstream_dispatch_active_target: Option<String>,
    pub downstream_dispatch_last_target: Option<String>,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub selected_backend: Option<String>,
    pub effective_execution_posture: serde_json::Value,
    pub route_policy: serde_json::Value,
    pub activation_evidence: serde_json::Value,
    pub recorded_at: String,
}

impl RunGraphDispatchReceiptSummary {
    pub(crate) fn from_receipt(receipt: RunGraphDispatchReceipt) -> Self {
        let raw_lane_status = receipt.lane_status.trim();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        let lane_status = if raw_lane_status.is_empty() {
            derive_lane_status(
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
            .as_str()
            .to_string()
        } else if downstream_dispatch_allows_completed_lane_status(
            receipt.downstream_dispatch_status.as_deref(),
            canonical_lane_status,
        ) || (canonical_lane_status == LaneStatus::LaneCompleted.as_str()
            && receipt.dispatch_status == "executed")
        {
            canonical_lane_status.to_string()
        } else {
            normalize_run_graph_lane_status(
                Some(raw_lane_status),
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
        };
        let blocker_code = receipt
            .blocker_code
            .as_deref()
            .and_then(canonical_blocker_code_str)
            .map(str::to_string)
            .or(receipt.blocker_code.clone());
        let mut downstream_dispatch_blockers = receipt.downstream_dispatch_blockers;
        downstream_dispatch_blockers.sort_unstable();
        Self {
            run_id: receipt.run_id,
            dispatch_target: receipt.dispatch_target,
            dispatch_status: receipt.dispatch_status,
            lane_status,
            supersedes_receipt_id: receipt.supersedes_receipt_id,
            exception_path_receipt_id: receipt.exception_path_receipt_id,
            dispatch_kind: receipt.dispatch_kind,
            dispatch_surface: receipt.dispatch_surface,
            dispatch_command: receipt.dispatch_command,
            dispatch_packet_path: receipt.dispatch_packet_path,
            dispatch_result_path: receipt.dispatch_result_path,
            blocker_code,
            downstream_dispatch_target: receipt.downstream_dispatch_target,
            downstream_dispatch_command: receipt.downstream_dispatch_command,
            downstream_dispatch_note: receipt.downstream_dispatch_note,
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_blockers,
            downstream_dispatch_packet_path: receipt.downstream_dispatch_packet_path,
            downstream_dispatch_status: receipt.downstream_dispatch_status,
            downstream_dispatch_result_path: receipt.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: receipt.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: receipt.downstream_dispatch_active_target,
            downstream_dispatch_last_target: receipt.downstream_dispatch_last_target,
            activation_agent_type: receipt.activation_agent_type,
            activation_runtime_role: receipt.activation_runtime_role,
            selected_backend: receipt.selected_backend,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: receipt.recorded_at,
        }
    }

    pub(crate) fn with_effective_execution_posture(
        mut self,
        effective_execution_posture: serde_json::Value,
    ) -> Self {
        self.effective_execution_posture = effective_execution_posture;
        self
    }

    pub(crate) fn with_route_policy(mut self, route_policy: serde_json::Value) -> Self {
        self.route_policy = route_policy;
        self
    }

    pub(crate) fn with_activation_evidence(
        mut self,
        activation_evidence: serde_json::Value,
    ) -> Self {
        self.activation_evidence = activation_evidence;
        self
    }

    pub(crate) fn dispatch_run_graph_trace_ref(&self) -> Option<String> {
        self.downstream_dispatch_trace_path
            .clone()
            .or_else(|| {
                self.dispatch_result_path
                    .as_ref()
                    .map(|path| format!("dispatch-result:{path}"))
            })
            .or_else(|| {
                self.dispatch_packet_path
                    .as_ref()
                    .map(|path| format!("dispatch-packet:{path}"))
            })
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} target={} status={} lane_status={} supersedes_receipt_id={} exception_path_receipt_id={} blocker_code={} kind={} surface={} command={} packet={} result={} next_target={} next_command={} next_note={} next_ready={} next_blockers={} next_packet={} next_status={} next_result={} next_trace={} next_count={} next_last_target={} agent={} runtime_role={} backend={} posture={} route_backend={} evidence={} recorded_at={}",
            self.run_id,
            self.dispatch_target,
            self.dispatch_status,
            self.lane_status,
            self.supersedes_receipt_id.as_deref().unwrap_or("none"),
            self.exception_path_receipt_id.as_deref().unwrap_or("none"),
            self.blocker_code.as_deref().unwrap_or("none"),
            self.dispatch_kind,
            self.dispatch_surface.as_deref().unwrap_or("none"),
            self.dispatch_command.as_deref().unwrap_or("none"),
            self.dispatch_packet_path.as_deref().unwrap_or("none"),
            self.dispatch_result_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_target.as_deref().unwrap_or("none"),
            self.downstream_dispatch_command
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_note.as_deref().unwrap_or("none"),
            self.downstream_dispatch_ready,
            if self.downstream_dispatch_blockers.is_empty() {
                "none".to_string()
            } else {
                self.downstream_dispatch_blockers.join("|")
            },
            self.downstream_dispatch_packet_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_status.as_deref().unwrap_or("none"),
            self.downstream_dispatch_result_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_trace_path
                .as_deref()
                .unwrap_or("none"),
            self.downstream_dispatch_executed_count,
            self.downstream_dispatch_last_target
                .as_deref()
                .unwrap_or("none"),
            self.activation_agent_type.as_deref().unwrap_or("none"),
            self.activation_runtime_role.as_deref().unwrap_or("none"),
            self.selected_backend.as_deref().unwrap_or("none"),
            self.effective_execution_posture["effective_posture_kind"]
                .as_str()
                .unwrap_or("unknown"),
            self.route_policy["route_primary_backend"]
                .as_str()
                .unwrap_or("none"),
            self.activation_evidence["activation_kind"]
                .as_str()
                .unwrap_or("unknown"),
            self.recorded_at
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, PartialEq, Eq, Clone)]
pub struct RunGraphApprovalDelegationReceipt {
    pub receipt_id: String,
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub route_task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub resume_target: String,
    pub transition_kind: String,
    pub recorded_at: String,
}

impl RunGraphApprovalDelegationReceipt {
    pub(crate) fn from_status(
        status: &RunGraphStatus,
        transition_kind: &str,
        recorded_at: String,
    ) -> Self {
        let receipt_id = format!(
            "run-graph-approval-delegation-{run_id}-{recorded_at}",
            run_id = status.run_id
        );
        Self {
            receipt_id,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_class: status.task_class.clone(),
            route_task_class: status.route_task_class.clone(),
            active_node: status.active_node.clone(),
            next_node: status.next_node.clone(),
            status: status.status.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            policy_gate: status.policy_gate.clone(),
            handoff_state: status.handoff_state.clone(),
            resume_target: status.resume_target.clone(),
            transition_kind: transition_kind.to_string(),
            recorded_at,
        }
    }
}

fn ensure_run_graph_approval_delegation_receipt_consistency(
    receipt: &RunGraphApprovalDelegationReceipt,
) -> Result<(), StateStoreError> {
    if receipt.receipt_id.trim().is_empty()
        || receipt.run_id.trim().is_empty()
        || receipt.task_id.trim().is_empty()
        || receipt.task_class.trim().is_empty()
        || receipt.route_task_class.trim().is_empty()
        || receipt.active_node.trim().is_empty()
        || receipt.status.trim().is_empty()
        || receipt.lifecycle_stage.trim().is_empty()
        || receipt.policy_gate.trim().is_empty()
        || receipt.handoff_state.trim().is_empty()
        || receipt.resume_target.trim().is_empty()
        || receipt.transition_kind.trim().is_empty()
        || receipt.recorded_at.trim().is_empty()
    {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph approval/delegation receipt summary is inconsistent for `{}`: all receipt fields must be non-empty",
                receipt.run_id
            ),
        });
    }

    let is_route_bound_implementation =
        receipt.task_class == "implementation" && receipt.route_task_class == "implementation";
    let approval_wait = receipt.transition_kind == "approval_wait";
    let approval_complete = receipt.transition_kind == "approval_complete";
    if !is_route_bound_implementation || (!approval_wait && !approval_complete) {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph approval/delegation receipt summary is inconsistent for `{}`: transition_kind `{}` must be route-bound to implementation",
                receipt.run_id, receipt.transition_kind
            ),
        });
    }

    match receipt.transition_kind.as_str() {
        "approval_wait" => {
            if receipt.status != "awaiting_approval"
                || receipt.lifecycle_stage != "approval_wait"
                || receipt.policy_gate
                    != crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str()
                || receipt.handoff_state != "awaiting_approval"
                || receipt.resume_target != "dispatch.approval"
                || receipt.next_node.as_deref() != Some("approval")
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "run-graph approval/delegation receipt summary is inconsistent for `{}`: approval_wait receipts must carry the approval route shape",
                        receipt.run_id
                    ),
                });
            }
        }
        "approval_complete" => {
            if receipt.status != "completed"
                || receipt.lifecycle_stage != "implementation_complete"
                || receipt.policy_gate != "not_required"
                || receipt.handoff_state != "none"
                || receipt.resume_target != "none"
                || receipt.next_node.is_some()
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "run-graph approval/delegation receipt summary is inconsistent for `{}`: approval_complete receipts must carry the completion route shape",
                        receipt.run_id
                    ),
                });
            }
        }
        _ => unreachable!("receipt.transition_kind is canonical above"),
    }

    Ok(())
}

pub(crate) fn latest_run_graph_dispatch_receipt_matches_status(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    matches!(
        (
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id
        ),
        (Some(status_run_id), Some(receipt_run_id)) if status_run_id == receipt_run_id
    )
}

pub(crate) fn latest_run_graph_dispatch_receipt_summary_is_inconsistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    latest_run_graph_status_run_id.is_some()
        && !latest_run_graph_dispatch_receipt_matches_status(
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id,
        )
}

pub(crate) fn latest_run_graph_dispatch_receipt_signal_is_ambiguous(
    receipt: &RunGraphDispatchReceiptSummary,
) -> bool {
    dispatch_receipt_status_has_canonical_lane_signal(&receipt.dispatch_status)
        && receipt.lane_status.as_str()
            != normalize_run_graph_lane_status(
                Some(receipt.lane_status.as_str()),
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
        || !dispatch_receipt_status_has_canonical_lane_signal(&receipt.dispatch_status)
}

fn dispatch_receipt_status_has_canonical_lane_signal(dispatch_status: &str) -> bool {
    matches!(
        dispatch_status,
        "packet_ready" | "routed" | "bridge_request_pending" | "executing" | "executed" | "blocked"
    )
}

pub(crate) fn latest_run_graph_evidence_snapshot_is_consistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_recovery_run_id: Option<&str>,
    latest_run_graph_checkpoint_run_id: Option<&str>,
    latest_run_graph_gate_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    let Some(latest_run_graph_status_run_id) = latest_run_graph_status_run_id else {
        return latest_run_graph_recovery_run_id.is_none()
            && latest_run_graph_checkpoint_run_id.is_none()
            && latest_run_graph_gate_run_id.is_none()
            && latest_run_graph_dispatch_receipt_run_id.is_none();
    };
    [
        latest_run_graph_recovery_run_id,
        latest_run_graph_checkpoint_run_id,
        latest_run_graph_gate_run_id,
        latest_run_graph_dispatch_receipt_run_id,
    ]
    .into_iter()
    .flatten()
    .all(|run_id| run_id == latest_run_graph_status_run_id)
}

pub(crate) fn default_run_graph_lane_status() -> String {
    LaneStatus::LaneOpen.as_str().to_string()
}

pub(crate) fn normalize_run_graph_lane_status(
    value: Option<&str>,
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> String {
    let derived_lane_status = derive_lane_status(
        dispatch_status,
        supersedes_receipt_id,
        exception_path_receipt_id,
    )
    .as_str()
    .to_string();
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let canonical_lane_status = canonical_lane_status_str(raw).unwrap_or(raw).trim();
            if dispatch_status.trim() == "executed" && canonical_lane_status == "lane_completed" {
                return canonical_lane_status.to_string();
            }
            if canonical_lane_status == derived_lane_status {
                return canonical_lane_status.to_string();
            }
            if let Some(parsed_lane_status) = LaneStatus::from_str(canonical_lane_status) {
                if lane_status_has_required_evidence(
                    parsed_lane_status,
                    supersedes_receipt_id,
                    exception_path_receipt_id,
                ) {
                    return canonical_lane_status.to_string();
                }
            }
            derived_lane_status
        }
        _ => derived_lane_status,
    }
}

pub(crate) fn downstream_dispatch_allows_completed_lane_status(
    downstream_dispatch_status: Option<&str>,
    canonical_lane_status: &str,
) -> bool {
    matches!(
        downstream_dispatch_status,
        Some("executed" | "retired_closed_task_run")
    ) && canonical_lane_status == "lane_completed"
}

pub(crate) fn deserialize_run_graph_lane_status<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <Option<String> as serde::Deserialize>::deserialize(deserializer)?;
    match value.as_deref() {
        Some(raw) if !raw.trim().is_empty() => {
            Ok(canonical_lane_status_str(raw).unwrap_or(raw).to_string())
        }
        _ => Ok(default_run_graph_lane_status()),
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphGateSummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphGateSummary {
    pub(crate) fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            context_state: status.context_state,
            delegation_gate,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} gate={} handoff={} context={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.policy_gate,
            self.handoff_state,
            self.context_state,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, Clone)]
struct CurrentSessionRunGraphClaimScope {
    run_ids: Vec<String>,
    task_ids: Vec<String>,
}

impl CurrentSessionRunGraphClaimScope {
    fn is_empty(&self) -> bool {
        self.run_ids.is_empty() && self.task_ids.is_empty()
    }

    fn push_run_id(&mut self, run_id: String) {
        let run_id = run_id.trim().to_string();
        if !run_id.is_empty() && !self.run_ids.contains(&run_id) {
            self.run_ids.push(run_id);
        }
    }

    fn push_task_id(&mut self, task_id: String) {
        let task_id = task_id.trim().to_string();
        if !task_id.is_empty() && !self.task_ids.contains(&task_id) {
            self.task_ids.push(task_id);
        }
    }

    fn contains_run_id(&self, run_id: &str) -> bool {
        let run_id = run_id.trim();
        !run_id.is_empty() && self.run_ids.iter().any(|value| value == run_id)
    }

    fn contains_task_id(&self, task_id: &str) -> bool {
        let task_id = task_id.trim();
        !task_id.is_empty() && self.task_ids.iter().any(|value| value == task_id)
    }

    fn matches_binding(&self, binding: &RunGraphContinuationBinding) -> bool {
        self.run_ids.contains(&binding.run_id)
            || self.task_ids.contains(&binding.task_id)
            || binding
                .active_bounded_unit
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|run_id| self.run_ids.iter().any(|value| value == run_id))
            || binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|task_id| self.task_ids.iter().any(|value| value == task_id))
    }
}

impl StateStore {
    pub(crate) fn current_session_id(&self) -> Result<Option<String>, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        Ok(evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned))
    }

    pub(crate) fn current_session_identity_is_present(&self) -> Result<bool, StateStoreError> {
        Ok(self.current_session_id()?.is_some())
    }

    pub(crate) fn current_session_identity_is_explicit(&self) -> Result<bool, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        Ok(evidence["current_session"]["identity_source"]
            .as_str()
            .is_some_and(|source| source != "generated_local_session_token"))
    }

    fn run_graph_owner_evidence_record_id(run_id: &str, artifact_kind: &str) -> String {
        sanitize_record_id(&format!("{run_id}::{artifact_kind}"))
    }

    fn current_runtime_owner_evidence(&self) -> Result<serde_json::Value, StateStoreError> {
        static OWNER_EVIDENCE_CACHE: std::sync::OnceLock<
            std::sync::Mutex<
                Option<(
                    std::path::PathBuf,
                    Option<String>,
                    std::time::Instant,
                    serde_json::Value,
                )>,
            >,
        > = std::sync::OnceLock::new();

        let root = self.root().to_path_buf();
        let session_id = std::env::var("VIDA_SESSION_ID").ok();
        let cache = OWNER_EVIDENCE_CACHE.get_or_init(|| std::sync::Mutex::new(None));
        if let Ok(guard) = cache.lock() {
            if let Some((cached_root, cached_session_id, cached_at, evidence)) = guard.as_ref() {
                if cached_root == &root
                    && cached_session_id == &session_id
                    && cached_at.elapsed() < std::time::Duration::from_secs(10)
                {
                    return Ok(evidence.clone());
                }
            }
        }

        let evidence =
            crate::orchestrator_session_surface::build_runtime_owner_evidence(self.root(), true)
                .map_err(|reason| StateStoreError::InvalidTaskRecord {
                    reason: format!("runtime owner evidence unavailable: {reason}"),
                })?;

        if let Ok(mut guard) = cache.lock() {
            *guard = Some((
                root,
                session_id,
                std::time::Instant::now(),
                evidence.clone(),
            ));
        }
        Ok(evidence)
    }

    async fn current_session_run_graph_claim_scope(
        &self,
    ) -> Result<Option<CurrentSessionRunGraphClaimScope>, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        let Some(current_session_id) = evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            return Ok(None);
        };
        let current_stable_fallback =
            evidence["current_session"]["fallback_replaces_legacy_stable_worktree_state_hash"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        let mut scope = CurrentSessionRunGraphClaimScope {
            run_ids: Vec::new(),
            task_ids: Vec::new(),
        };
        for claim in self.active_orchestrator_claims().await? {
            let claim_matches_current_session = claim.orchestrator_session_id == current_session_id
                || current_stable_fallback.as_deref().is_some_and(|fallback| {
                    let fallback = fallback.trim();
                    !fallback.is_empty() && claim.orchestrator_session_id == fallback
                });
            if !claim_matches_current_session {
                continue;
            }
            if let Some(run_id) = claim.run_id {
                scope.push_run_id(run_id);
            }
            if let Some(task_id) = claim.task_id {
                scope.push_task_id(task_id);
            }
        }
        let scoped_task_ids = scope.task_ids.clone();
        for task_id in scoped_task_ids {
            let mut run_query = self
                .db
                .query(
                    "SELECT run_id FROM execution_plan_state \
                     WHERE task_id = $task_id \
                     ORDER BY run_id DESC;",
                )
                .bind(("task_id", task_id))
                .await?;
            let rows: Vec<serde_json::Value> = run_query.take(0)?;
            for row in rows {
                if let Some(run_id) = row
                    .get("run_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    scope.push_run_id(run_id.to_string());
                }
            }
        }
        let mut binding_query = self
            .db
            .query(
                "SELECT * FROM run_graph_continuation_binding \
                 WHERE binding_source = 'explicit_continuation_bind' \
                    OR binding_source = 'explicit_continuation_bind_task' \
                    OR binding_source = 'task_close_reconcile' \
                 ORDER BY recorded_at DESC, run_id DESC;",
            )
            .await?;
        let rows: Vec<RunGraphContinuationBinding> = binding_query.take(0)?;
        for binding in rows {
            let binding_session_id = binding
                .active_bounded_unit
                .get("orchestrator_session_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let binding_matches_current_session = binding_session_id.is_some_and(|session_id| {
                session_id == current_session_id
                    || current_stable_fallback
                        .as_deref()
                        .is_some_and(|fallback| fallback == session_id)
            });
            if (binding_session_id.is_some() && !binding_matches_current_session)
                || (binding_session_id.is_none() && !scope.matches_binding(&binding))
            {
                continue;
            }
            scope.push_run_id(binding.run_id.clone());
            scope.push_task_id(binding.task_id.clone());
            if let Some(run_id) = binding
                .active_bounded_unit
                .get("run_id")
                .and_then(serde_json::Value::as_str)
            {
                scope.push_run_id(run_id.to_string());
            }
            if let Some(task_id) = binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
            {
                scope.push_task_id(task_id.to_string());
            }
        }

        if scope.is_empty() {
            let mut owner_query = self
                .db
                .query(
                    "SELECT * FROM run_graph_owner_evidence \
                     WHERE runtime_owner_evidence.current_session.session_id = $session_id \
                        OR runtime_owner_evidence.current_session.fallback_replaces_legacy_stable_worktree_state_hash = $stable_fallback \
                     ORDER BY recorded_at DESC, run_id DESC;",
                )
                .bind(("session_id", current_session_id.clone()))
                .bind((
                    "stable_fallback",
                    current_stable_fallback.clone().unwrap_or_default(),
                ))
                .await?;
            let owner_records: Vec<RunGraphOwnerEvidenceRecord> = owner_query.take(0)?;
            for record in owner_records {
                if Self::owner_evidence_matches_current_session(
                    &record.runtime_owner_evidence,
                    &evidence,
                    current_session_id.as_str(),
                    current_stable_fallback.as_deref(),
                ) {
                    scope.push_run_id(record.run_id);
                }
            }
        }
        if scope.is_empty() {
            Ok(None)
        } else {
            Ok(Some(scope))
        }
    }

    #[cfg(test)]
    pub(crate) async fn acquire_current_session_run_graph_claim_for_test(
        &self,
        claim_id: &str,
        run_id: &str,
        task_id: &str,
        conflict_domain: &str,
        owned_path: &str,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        let owner_evidence = self.current_runtime_owner_evidence()?;
        let current_session = &owner_evidence["current_session"];
        let current_session_id = current_session["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: "test run-graph claim requires current session id".to_string(),
            })?;
        let claim_session_id =
            current_session["fallback_replaces_legacy_stable_worktree_state_hash"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(current_session_id);
        let worktree_environment_id = current_session["worktree_environment_id"]
            .as_str()
            .unwrap_or_else(|| self.root().to_str().unwrap_or_default())
            .to_string();
        self.acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
            claim_id: claim_id.to_string(),
            state_root_id: self.root().display().to_string(),
            worktree_environment_id,
            orchestrator_session_id: claim_session_id.to_string(),
            process_id: Some(std::process::id()),
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            lane_id: None,
            claim_kind: "active_task_session_claim".to_string(),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: vec![owned_path.to_string()],
            read_only_paths: Vec::new(),
            lease_mode: LeaseMode::Observe,
            lease_seconds: 3600,
        })
        .await
    }

    fn ensure_runtime_owner_mutation_allowed(
        evidence: &serde_json::Value,
    ) -> Result<(), StateStoreError> {
        if evidence["mutation_gate"] == "blocked_live_other_orchestrator" {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "runtime owner evidence blocks run-graph mutation: live_other_orchestrator_owner"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn owner_evidence_matches_current_session(
        runtime_owner_evidence: &serde_json::Value,
        current_runtime_owner_evidence: &serde_json::Value,
        current_session_id: &str,
        current_stable_fallback: Option<&str>,
    ) -> bool {
        let current_session = &runtime_owner_evidence["current_session"];
        current_session["session_id"]
            .as_str()
            .is_some_and(|session_id| session_id.trim() == current_session_id)
            || current_stable_fallback.is_some_and(|fallback| {
                let fallback = fallback.trim();
                current_session["session_id"]
                    .as_str()
                    .is_some_and(|session_id| !fallback.is_empty() && session_id.trim() == fallback)
                    || current_session["fallback_replaces_legacy_stable_worktree_state_hash"]
                        .as_str()
                        .is_some_and(|record_fallback| {
                            !fallback.is_empty() && record_fallback.trim() == fallback
                        })
            })
            || Self::legacy_same_worktree_owner_evidence_matches_current_session(
                runtime_owner_evidence,
                current_runtime_owner_evidence,
            )
    }

    fn legacy_same_worktree_owner_evidence_matches_current_session(
        runtime_owner_evidence: &serde_json::Value,
        current_runtime_owner_evidence: &serde_json::Value,
    ) -> bool {
        if current_runtime_owner_evidence["mutation_gate"] != "current_session_allowed" {
            return false;
        }
        if current_runtime_owner_evidence["live_other_sessions"]
            .as_array()
            .is_some_and(|sessions| !sessions.is_empty())
        {
            return false;
        }
        let recorded_session = &runtime_owner_evidence["current_session"];
        let recorded_identity_source = recorded_session["identity_source"]
            .as_str()
            .map(str::trim)
            .unwrap_or_default();
        if !matches!(
            recorded_identity_source,
            "stable_local_worktree_session_id"
                | "generated_local_session_token"
                | "synthesized_local_session_token"
                | "CODEX_SESSION_ID"
                | "CODEX_THREAD_ID"
        ) {
            return false;
        }
        let current_session = &current_runtime_owner_evidence["current_session"];
        Self::owner_path_field_matches(recorded_session, current_session, "worktree_environment_id")
            || Self::owner_path_field_matches(recorded_session, current_session, "project_root")
    }

    fn owner_path_field_matches(
        recorded_session: &serde_json::Value,
        current_session: &serde_json::Value,
        field: &str,
    ) -> bool {
        let Some(recorded) = recorded_session[field].as_str() else {
            return false;
        };
        let Some(current) = current_session[field].as_str() else {
            return false;
        };
        let recorded = Self::normalize_owner_path(recorded);
        let current = Self::normalize_owner_path(current);
        !recorded.is_empty() && recorded == current
    }

    fn normalize_owner_path(value: &str) -> String {
        value
            .trim()
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .to_ascii_lowercase()
    }

    async fn ensure_current_session_mutation_claim_for_run(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        let current_session_id = evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: "run-graph mutation requires an active current session id".to_string(),
            })?;
        let current_stable_fallback =
            evidence["current_session"]["fallback_replaces_legacy_stable_worktree_state_hash"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty());

        let active_claims = self.active_orchestrator_claims().await?;
        let run_task_id = self
            .run_graph_task_id_for_mutation_claim(run_id)
            .await?
            .map(|task_id| task_id.trim().to_string())
            .filter(|task_id| !task_id.is_empty());
        for claim in active_claims {
            let claim_matches_current_session = claim.orchestrator_session_id == current_session_id
                || current_stable_fallback.is_some_and(|fallback| {
                    let fallback = fallback.trim();
                    !fallback.is_empty() && claim.orchestrator_session_id == fallback
                });
            if claim_matches_current_session
                && claim
                    .run_id
                    .as_deref()
                    .is_some_and(|claim_run_id| claim_run_id.trim() == run_id)
            {
                return Ok(());
            }
            if claim_matches_current_session
                && run_task_id.as_deref().is_some_and(|task_id| {
                    claim
                        .task_id
                        .as_deref()
                        .is_some_and(|claim_task_id| claim_task_id.trim() == task_id)
                })
            {
                return Ok(());
            }
        }
        if let Some(scope) = self.current_session_run_graph_claim_scope().await? {
            if scope.contains_run_id(run_id)
                || scope.contains_task_id(run_id)
                || run_task_id
                    .as_deref()
                    .is_some_and(|task_id| scope.contains_task_id(task_id))
            {
                return Ok(());
            }
        }

        if self.run_graph_legacy_ownerless(run_id).await? {
            return Ok(());
        }

        let mut owner_query = self
            .db
            .query(
                "SELECT * FROM run_graph_owner_evidence \
                 WHERE run_id = $run_id \
                 ORDER BY recorded_at DESC, artifact_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let owner_records: Vec<RunGraphOwnerEvidenceRecord> = owner_query.take(0)?;
        if owner_records.iter().any(|record| {
            Self::owner_evidence_matches_current_session(
                &record.runtime_owner_evidence,
                &evidence,
                current_session_id,
                current_stable_fallback,
            )
        }) {
            return Ok(());
        }

        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "run-graph mutation blocked: current session does not own run `{run_id}`"
            ),
        })
    }

    pub(crate) async fn current_session_can_mutate_run_graph_run(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        match self
            .ensure_current_session_mutation_claim_for_run(run_id)
            .await
        {
            Ok(()) => Ok(true),
            Err(StateStoreError::InvalidTaskRecord { reason })
                if reason.contains("current session does not own run") =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn current_session_has_run_graph_claim(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        let Some(current_session_id) = evidence["current_session"]["session_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(false);
        };
        let current_stable_fallback =
            evidence["current_session"]["fallback_replaces_legacy_stable_worktree_state_hash"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty());
        Ok(self
            .active_orchestrator_claims()
            .await?
            .into_iter()
            .any(|claim| {
                let matches_session = claim.orchestrator_session_id == current_session_id
                    || current_stable_fallback
                        .is_some_and(|fallback| claim.orchestrator_session_id == fallback);
                matches_session
                    && claim
                        .run_id
                        .as_deref()
                        .is_some_and(|claim_run_id| claim_run_id.trim() == run_id.trim())
            }))
    }

    async fn run_graph_task_id_for_mutation_claim(
        &self,
        run_id: &str,
    ) -> Result<Option<String>, StateStoreError> {
        let execution: Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?;
        if let Some(execution) = execution {
            return Ok(Some(execution.task_id));
        }
        Ok(self
            .run_graph_dispatch_context(run_id)
            .await?
            .map(|context| context.task_id))
    }

    async fn prepare_run_graph_owner_evidence(
        &self,
        run_id: &str,
        artifact_kind: &str,
    ) -> Result<RunGraphOwnerEvidenceRecord, StateStoreError> {
        let evidence = self.current_runtime_owner_evidence()?;
        Self::ensure_runtime_owner_mutation_allowed(&evidence)?;
        self.ensure_current_session_mutation_claim_for_run(run_id)
            .await?;
        let artifact_id = Self::run_graph_owner_evidence_record_id(run_id, artifact_kind);
        Ok(RunGraphOwnerEvidenceRecord {
            run_id: run_id.to_string(),
            artifact_kind: artifact_kind.to_string(),
            artifact_id: artifact_id.clone(),
            runtime_owner_evidence: evidence,
            recorded_at: unix_timestamp().to_string(),
        })
    }

    async fn record_run_graph_owner_evidence(
        &self,
        run_id: &str,
        artifact_kind: &str,
    ) -> Result<(), StateStoreError> {
        let record = self
            .prepare_run_graph_owner_evidence(run_id, artifact_kind)
            .await?;
        let artifact_id = record.artifact_id.clone();
        let _: Option<RunGraphOwnerEvidenceRecord> = self
            .db
            .upsert(("run_graph_owner_evidence", artifact_id.as_str()))
            .content(record)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn run_graph_owner_evidence_record(
        &self,
        run_id: &str,
        artifact_kind: &str,
    ) -> Result<Option<RunGraphOwnerEvidenceRecord>, StateStoreError> {
        self.db
            .select((
                "run_graph_owner_evidence",
                Self::run_graph_owner_evidence_record_id(run_id, artifact_kind).as_str(),
            ))
            .await
            .map_err(StateStoreError::from)
    }

    pub(crate) async fn run_graph_legacy_ownerless(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let mut owner_query = self
            .db
            .query(
                "SELECT artifact_id FROM run_graph_owner_evidence WHERE run_id = $run_id LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let owner_rows: Vec<serde_json::Value> = owner_query.take(0)?;
        if !owner_rows.is_empty() {
            return Ok(false);
        }

        let mut claim_query = self
            .db
            .query(
                "SELECT claim_id FROM orchestrator_claim \
                 WHERE run_id = $run_id \
                 AND status IN ['active', 'renewed', 'blocked'] \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let claim_rows: Vec<serde_json::Value> = claim_query.take(0)?;
        Ok(claim_rows.is_empty())
    }

    pub async fn run_graph_summary(&self) -> Result<RunGraphSummary, StateStoreError> {
        Ok(RunGraphSummary {
            execution_plan_count: self.count_table_rows("execution_plan_state").await?,
            routed_run_count: self.count_table_rows("routed_run_state").await?,
            governance_count: self.count_table_rows("governance_state").await?,
            resumability_count: self.count_table_rows("resumability_capsule").await?,
            reconciliation_count: self.count_table_rows("task_reconciliation_summary").await?,
        })
    }

    pub async fn record_run_graph_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        status.validate_memory_governance()?;
        self.ensure_current_session_mutation_claim_for_run(&status.run_id)
            .await?;
        self.record_run_graph_status_after_admission(status).await
    }

    pub(crate) async fn record_reconciled_terminal_closure_run_graph_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        status.validate_memory_governance()?;
        Self::ensure_reconciled_terminal_closure_status(status)?;
        let evidence = self.current_runtime_owner_evidence()?;
        Self::ensure_runtime_owner_mutation_allowed(&evidence)?;
        self.record_run_graph_status_after_admission(status).await
    }

    fn ensure_reconciled_terminal_closure_status(
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        let is_reconciled_terminal_closure = status.active_node == "closure"
            && status.next_node.is_none()
            && status.status == "completed"
            && status.lifecycle_stage == "closure_complete"
            && status.handoff_state == "none"
            && status.checkpoint_kind == "none"
            && status.resume_target == "none"
            && !status.recovery_ready;
        if is_reconciled_terminal_closure {
            return Ok(());
        }
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "closed-run reconciliation can only bypass run ownership for reconciled terminal closure status `{}`",
                status.run_id
            ),
        })
    }

    async fn record_run_graph_status_after_admission(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        let updated_at = unix_timestamp_nanos().to_string();
        let receipt_recorded_at = updated_at.clone();
        let checkpoint_record_updated_at = updated_at.clone();
        let _: Option<RoutedRunStateRow> = self
            .db
            .upsert(("routed_run_state", status.run_id.as_str()))
            .content(RoutedRunStateRow {
                run_id: status.run_id.clone(),
                route_task_class: status.route_task_class.clone(),
                selected_backend: status.selected_backend.clone(),
                lane_id: status.lane_id.clone(),
                lifecycle_stage: status.lifecycle_stage.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<GovernanceStateRow> = self
            .db
            .upsert(("governance_state", status.run_id.as_str()))
            .content(GovernanceStateRow {
                run_id: status.run_id.clone(),
                policy_gate: status.policy_gate.clone(),
                handoff_state: status.handoff_state.clone(),
                context_state: status.context_state.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<ResumabilityCapsuleRow> = self
            .db
            .upsert(("resumability_capsule", status.run_id.as_str()))
            .content(ResumabilityCapsuleRow {
                run_id: status.run_id.clone(),
                checkpoint_kind: status.checkpoint_kind.clone(),
                resume_target: status.resume_target.clone(),
                recovery_ready: status.recovery_ready,
                updated_at,
            })
            .await?;
        let _: Option<ExecutionPlanStateRow> = self
            .db
            .upsert(("execution_plan_state", status.run_id.as_str()))
            .content(ExecutionPlanStateRow {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_class: status.task_class.clone(),
                active_node: status.active_node.clone(),
                next_node: status.next_node.clone(),
                status: status.status.clone(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;
        if status.checkpoint_kind.trim().eq_ignore_ascii_case("none") {
            self.clear_run_graph_projection_checkpoint_records(&status.run_id)
                .await?;
        } else {
            let checkpoint_record = RunGraphProjectionCheckpointRecord::from_status(
                status,
                checkpoint_record_updated_at,
            );
            self.record_run_graph_projection_checkpoint_record(&checkpoint_record)
                .await?;
        }
        if let Some(transition_kind) = approval_delegation_transition_kind(status) {
            let receipt = RunGraphApprovalDelegationReceipt::from_status(
                status,
                transition_kind,
                receipt_recorded_at,
            );
            self.record_run_graph_approval_delegation_receipt(&receipt)
                .await?;
        }
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_dispatch_receipt(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        if receipt.dispatch_status != "routed" {
            clear_run_graph_dispatch_init_fast_cache(self.root(), &receipt.run_id);
        }
        self.record_run_graph_owner_evidence(&receipt.run_id, "dispatch_receipt")
            .await?;
        let receipt: RunGraphDispatchReceiptStored = receipt.clone().into();
        let receipt = normalize_legacy_downstream_preview_drift(receipt);
        let (receipt, _) = normalize_repairable_in_flight_receipt_lane_status_drift(receipt);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .upsert(("run_graph_dispatch_receipt", receipt.run_id.as_str()))
            .content(receipt)
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_host_bridge_receipt_identity(
        &self,
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
    ) -> Result<(), StateStoreError> {
        let row = Self::host_bridge_receipt_identity_row(identity)?;
        let _: Option<HostBridgeReceiptIdentityStored> = self
            .db
            .upsert((
                "host_bridge_receipt_identity",
                identity.identity_key().as_str(),
            ))
            .content(row)
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    fn host_bridge_receipt_identity_row(
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
    ) -> Result<HostBridgeReceiptIdentityStored, StateStoreError> {
        identity
            .validate()
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        Ok(HostBridgeReceiptIdentityStored {
            schema_version: identity.schema_version.clone(),
            request_id: identity.request_id.clone(),
            run_id: identity.run_id.clone(),
            task_id: identity.task_id.clone(),
            attempt_id: identity.attempt_id.clone(),
            packet_id: identity.packet_id.clone(),
            dispatch_target: identity.dispatch_target.clone(),
            packet_path: identity.packet_path.clone(),
            backend_id: identity.backend_id.clone(),
            carrier_id: identity.carrier_id.clone(),
            adapter_kind: identity.adapter_kind.clone(),
            adapter_capability_id: identity.adapter_capability_id.clone(),
            invocation_mode: identity.invocation_mode.clone(),
            dispatch_transport: identity.dispatch_transport.clone(),
            receipt_mode: identity.receipt_mode.clone(),
            adapter_contract_source: identity.adapter_contract_source.clone(),
            adapter_contract_snapshot: identity.adapter_contract_snapshot.clone(),
            adapter_contract_hash: identity.adapter_contract_hash.clone(),
            adapter_operations: identity.adapter_operations.to_value(),
            request_path: identity.request_path.clone(),
            result_path: identity.result_path.clone(),
            receipt_path: identity.receipt_path.clone(),
            recorded_at: identity.recorded_at.clone(),
        })
    }

    pub async fn record_host_bridge_receipt_binding(
        &self,
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        self.record_host_bridge_receipt_binding_inner(identity, receipt, false)
            .await
    }

    #[cfg(test)]
    async fn record_host_bridge_receipt_binding_with_forced_rollback(
        &self,
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        self.record_host_bridge_receipt_binding_inner(identity, receipt, true)
            .await
    }

    async fn record_host_bridge_receipt_binding_inner(
        &self,
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
        receipt: &RunGraphDispatchReceipt,
        force_failure_after_binding_write: bool,
    ) -> Result<(), StateStoreError> {
        let identity_row = Self::host_bridge_receipt_identity_row(identity)?;
        if receipt.run_id != identity.run_id || receipt.dispatch_target != identity.dispatch_target
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "host_bridge_receipt_binding_identity_mismatch:run_or_target".to_string(),
            });
        }
        let compact_value =
            serde_json::to_value(receipt).map_err(|error| StateStoreError::InvalidTaskRecord {
                reason: format!("host_bridge_receipt_binding_compact_serialize_failed:{error}"),
            })?;
        let blockers = identity.compact_receipt_blockers(&compact_value);
        if !blockers.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: blockers.join(","),
            });
        }
        let compact: RunGraphDispatchReceiptStored = receipt.clone().into();
        let compact = normalize_legacy_downstream_preview_drift(compact);
        let (compact, _) = normalize_repairable_in_flight_receipt_lane_status_drift(compact);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&compact)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&compact)?;

        let identity_key = identity.identity_key();
        let owner_record = self
            .prepare_run_graph_owner_evidence(&compact.run_id, "dispatch_receipt")
            .await?;
        let owner_record_id = owner_record.artifact_id.clone();
        let compact_run_id = compact.run_id.clone();
        let response = self
            .db
            .query(
                "BEGIN TRANSACTION; \
                 LET $existing_identity = SELECT VALUE { schema_version: schema_version, request_id: request_id, run_id: run_id, task_id: task_id, attempt_id: attempt_id, packet_id: packet_id, dispatch_target: dispatch_target, packet_path: packet_path, backend_id: backend_id, carrier_id: carrier_id, adapter_kind: adapter_kind, adapter_capability_id: adapter_capability_id, invocation_mode: invocation_mode, dispatch_transport: dispatch_transport, receipt_mode: receipt_mode, adapter_contract_source: adapter_contract_source, adapter_contract_snapshot: adapter_contract_snapshot, adapter_contract_hash: adapter_contract_hash, adapter_operations: adapter_operations, request_path: request_path, result_path: result_path, receipt_path: receipt_path, recorded_at: recorded_at } FROM type::record('host_bridge_receipt_identity', $identity_key); \
                 IF array::len($existing_identity) > 0 AND $existing_identity[0] != $identity { \
                   THROW 'host_bridge_receipt_binding_conflict:identity_key=' + $identity_key; \
                 }; \
                 LET $existing_receipt = SELECT VALUE { run_id: run_id, dispatch_target: dispatch_target, dispatch_status: dispatch_status, lane_status: lane_status, supersedes_receipt_id: supersedes_receipt_id, exception_path_receipt_id: exception_path_receipt_id, dispatch_kind: dispatch_kind, dispatch_surface: dispatch_surface, dispatch_command: dispatch_command, dispatch_packet_path: dispatch_packet_path, dispatch_result_path: dispatch_result_path, blocker_code: blocker_code, downstream_dispatch_target: downstream_dispatch_target, downstream_dispatch_command: downstream_dispatch_command, downstream_dispatch_note: downstream_dispatch_note, downstream_dispatch_ready: downstream_dispatch_ready, downstream_dispatch_blockers: downstream_dispatch_blockers, downstream_dispatch_packet_path: downstream_dispatch_packet_path, downstream_dispatch_status: downstream_dispatch_status, downstream_dispatch_result_path: downstream_dispatch_result_path, downstream_dispatch_trace_path: downstream_dispatch_trace_path, downstream_dispatch_executed_count: downstream_dispatch_executed_count, downstream_dispatch_active_target: downstream_dispatch_active_target, downstream_dispatch_last_target: downstream_dispatch_last_target, activation_agent_type: activation_agent_type, activation_runtime_role: activation_runtime_role, selected_backend: selected_backend, recorded_at: recorded_at } FROM type::record('run_graph_dispatch_receipt', $run_id); \
                 LET $matching_in_flight_receipt = array::len($existing_receipt) > 0 \
                   AND $existing_receipt[0].run_id = $receipt.run_id \
                   AND $existing_receipt[0].dispatch_target = $receipt.dispatch_target \
                   AND $existing_receipt[0].dispatch_status = 'executing' \
                   AND $existing_receipt[0].lane_status = 'lane_running' \
                   AND $existing_receipt[0].supersedes_receipt_id = $receipt.supersedes_receipt_id \
                   AND $existing_receipt[0].exception_path_receipt_id = $receipt.exception_path_receipt_id \
                   AND $existing_receipt[0].dispatch_kind = $receipt.dispatch_kind \
                   AND $existing_receipt[0].dispatch_surface = $receipt.dispatch_surface \
                   AND $existing_receipt[0].dispatch_command = $receipt.dispatch_command \
                   AND $existing_receipt[0].dispatch_packet_path = $receipt.dispatch_packet_path \
                   AND $existing_receipt[0].activation_agent_type = $receipt.activation_agent_type \
                   AND $existing_receipt[0].activation_runtime_role = $receipt.activation_runtime_role \
                   AND $existing_receipt[0].selected_backend = $receipt.selected_backend; \
                 IF array::len($existing_receipt) > 0 AND array::len($existing_identity) = 0 AND $matching_in_flight_receipt = false { \
                   THROW 'host_bridge_receipt_binding_conflict:receipt_key=' + $run_id; \
                 }; \
                 UPSERT type::record('host_bridge_receipt_identity', $identity_key) CONTENT $identity; \
                 UPSERT type::record('run_graph_dispatch_receipt', $run_id) CONTENT $receipt; \
                 UPSERT type::record('run_graph_owner_evidence', $owner_record_id) CONTENT $owner_record; \
                 IF $force_failure_after_binding_write { \
                   THROW 'host_bridge_receipt_binding_test_atomic_rollback'; \
                 }; \
                 COMMIT TRANSACTION;",
            )
            .bind(("identity_key", identity_key.clone()))
            .bind(("identity", identity_row))
            .bind(("run_id", compact.run_id.clone()))
            .bind(("receipt", compact.clone()))
            .bind(("owner_record_id", owner_record_id))
            .bind(("owner_record", owner_record))
            .bind((
                "force_failure_after_binding_write",
                force_failure_after_binding_write,
            ))
            .await?;
        if let Err(error) = response.check() {
            if let Some(existing) = self
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await?
            {
                if existing != *identity {
                    return Err(StateStoreError::InvalidTaskRecord {
                        reason: format!(
                            "host_bridge_receipt_binding_conflict:identity_key={identity_key}"
                        ),
                    });
                }
            }
            let existing_receipt: Option<RunGraphDispatchReceiptStored> = self
                .db
                .select(("run_graph_dispatch_receipt", compact_run_id.as_str()))
                .await?;
            if existing_receipt.as_ref().is_some_and(|existing| {
                !Self::host_bridge_binding_matches_in_flight_receipt(existing, &compact)
            }) && self
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await?
                .is_none()
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "host_bridge_receipt_binding_conflict:receipt_key={}",
                        compact_run_id
                    ),
                });
            }
            return Err(error.into());
        }
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    fn host_bridge_binding_matches_in_flight_receipt(
        existing: &RunGraphDispatchReceiptStored,
        pending: &RunGraphDispatchReceiptStored,
    ) -> bool {
        existing.run_id == pending.run_id
            && existing.dispatch_target == pending.dispatch_target
            && existing.dispatch_status == "executing"
            && existing.lane_status.as_deref() == Some("lane_running")
            && existing.supersedes_receipt_id == pending.supersedes_receipt_id
            && existing.exception_path_receipt_id == pending.exception_path_receipt_id
            && existing.dispatch_kind == pending.dispatch_kind
            && existing.dispatch_surface == pending.dispatch_surface
            && existing.dispatch_command == pending.dispatch_command
            && existing.dispatch_packet_path == pending.dispatch_packet_path
            && existing.activation_agent_type == pending.activation_agent_type
            && existing.activation_runtime_role == pending.activation_runtime_role
            && existing.selected_backend == pending.selected_backend
    }

    pub async fn host_bridge_receipt_identity(
        &self,
        run_id: &str,
        dispatch_target: &str,
        packet_path: &str,
        request_id: &str,
    ) -> Result<Option<taskflow_host_bridge::HostBridgeReceiptIdentityV1>, StateStoreError> {
        let key = taskflow_host_bridge::host_bridge_receipt_identity_key(
            run_id,
            dispatch_target,
            packet_path,
            request_id,
        );
        let row: Option<HostBridgeReceiptIdentityStored> = self
            .db
            .select(("host_bridge_receipt_identity", key.as_str()))
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let adapter_operations =
            serde_json::from_value(row.adapter_operations).map_err(|error| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "host bridge receipt identity adapter_operations invalid: {error}"
                    ),
                }
            })?;
        let identity = taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
            schema_version: row.schema_version,
            request_id: row.request_id,
            run_id: row.run_id,
            task_id: row.task_id,
            attempt_id: row.attempt_id,
            packet_id: row.packet_id,
            dispatch_target: row.dispatch_target,
            packet_path: row.packet_path,
            backend_id: row.backend_id,
            carrier_id: row.carrier_id,
            adapter_kind: row.adapter_kind,
            adapter_capability_id: row.adapter_capability_id,
            invocation_mode: row.invocation_mode,
            dispatch_transport: row.dispatch_transport,
            receipt_mode: row.receipt_mode,
            adapter_contract_source: row.adapter_contract_source,
            adapter_contract_snapshot: row.adapter_contract_snapshot,
            adapter_contract_hash: row.adapter_contract_hash,
            adapter_operations,
            request_path: row.request_path,
            result_path: row.result_path,
            receipt_path: row.receipt_path,
            recorded_at: row.recorded_at,
        };
        identity
            .validate()
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        Ok(Some(identity))
    }

    pub async fn host_bridge_receipt_identities_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<taskflow_host_bridge::HostBridgeReceiptIdentityV1>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM host_bridge_receipt_identity WHERE run_id = $run_id ORDER BY recorded_at DESC;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<HostBridgeReceiptIdentityStored> = query.take(0)?;
        rows.into_iter()
            .map(|row| {
                let adapter_operations =
                    serde_json::from_value(row.adapter_operations).map_err(|error| {
                        StateStoreError::InvalidTaskRecord {
                            reason: format!(
                                "host bridge receipt identity adapter_operations invalid: {error}"
                            ),
                        }
                    })?;
                let identity = taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
                    schema_version: row.schema_version,
                    request_id: row.request_id,
                    run_id: row.run_id,
                    task_id: row.task_id,
                    attempt_id: row.attempt_id,
                    packet_id: row.packet_id,
                    dispatch_target: row.dispatch_target,
                    packet_path: row.packet_path,
                    backend_id: row.backend_id,
                    carrier_id: row.carrier_id,
                    adapter_kind: row.adapter_kind,
                    adapter_capability_id: row.adapter_capability_id,
                    invocation_mode: row.invocation_mode,
                    dispatch_transport: row.dispatch_transport,
                    receipt_mode: row.receipt_mode,
                    adapter_contract_source: row.adapter_contract_source,
                    adapter_contract_snapshot: row.adapter_contract_snapshot,
                    adapter_contract_hash: row.adapter_contract_hash,
                    adapter_operations,
                    request_path: row.request_path,
                    result_path: row.result_path,
                    receipt_path: row.receipt_path,
                    recorded_at: row.recorded_at,
                };
                identity
                    .validate()
                    .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
                Ok(identity)
            })
            .collect()
    }

    pub async fn host_bridge_receipt_identity_for_compact(
        &self,
        run_id: &str,
        dispatch_target: &str,
        packet_path: &str,
    ) -> Result<Option<taskflow_host_bridge::HostBridgeReceiptIdentityV1>, StateStoreError> {
        let matches = self
            .host_bridge_receipt_identities_for_run(run_id)
            .await?
            .into_iter()
            .filter(|identity| {
                identity.dispatch_target == dispatch_target
                    && taskflow_host_bridge::host_bridge_packet_paths_equivalent(
                        &identity.packet_path,
                        packet_path,
                    )
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Ok(None),
            [identity] => Ok(Some(identity.clone())),
            _ => Err(StateStoreError::InvalidTaskRecord {
                reason: "host_bridge_receipt_identity_ambiguous_compact_binding".to_string(),
            }),
        }
    }

    pub async fn clear_host_bridge_receipt_identity(
        &self,
        identity: &taskflow_host_bridge::HostBridgeReceiptIdentityV1,
    ) -> Result<(), StateStoreError> {
        identity
            .validate()
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        let key = identity.identity_key();
        let _: Option<HostBridgeReceiptIdentityStored> = self
            .db
            .delete(("host_bridge_receipt_identity", key.as_str()))
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_dispatch_lane_receipt(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        let Some(dispatch_packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch lane receipt for `{}` must include dispatch_packet_path",
                    receipt.run_id
                ),
            });
        };
        self.record_run_graph_owner_evidence(&receipt.run_id, "dispatch_lane_receipt")
            .await?;
        let receipt_id = run_graph_dispatch_lane_receipt_id(
            &receipt.run_id,
            &receipt.dispatch_target,
            dispatch_packet_path,
        );
        let receipt: RunGraphDispatchReceiptStored = receipt.clone().into();
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .upsert(("run_graph_dispatch_lane_receipt", receipt_id.as_str()))
            .content(receipt)
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn clear_run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .delete(("run_graph_dispatch_receipt", run_id))
            .await?;
        let _ = self
            .db
            .query("DELETE run_graph_dispatch_lane_receipt WHERE run_id = $run_id;")
            .bind(("run_id", run_id.to_string()))
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_continuation_binding(
        &self,
        binding: &RunGraphContinuationBinding,
    ) -> Result<(), StateStoreError> {
        binding.validate()?;
        self.record_run_graph_owner_evidence(&binding.run_id, "continuation_binding")
            .await?;
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .upsert(("run_graph_continuation_binding", binding.run_id.as_str()))
            .content(binding.clone())
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    async fn effective_exception_takeover_continuation_binding(
        &self,
        binding: RunGraphContinuationBinding,
    ) -> Result<RunGraphContinuationBinding, StateStoreError> {
        let receipt: Option<RunGraphDispatchReceiptStored> = self
            .db
            .select(("run_graph_dispatch_receipt", binding.run_id.as_str()))
            .await?;
        Ok(
            reconcile_continuation_binding_with_exception_takeover_receipt(
                binding,
                receipt.as_ref(),
            ),
        )
    }

    pub async fn run_graph_continuation_binding(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let binding: Option<RunGraphContinuationBinding> = self
            .db
            .select(("run_graph_continuation_binding", run_id))
            .await?;
        match binding {
            Some(binding) => {
                let Some(binding) = self.normalize_task_close_reconcile_binding(binding).await?
                else {
                    return Ok(None);
                };
                let binding = self
                    .effective_exception_takeover_continuation_binding(binding)
                    .await?;
                if self
                    .continuation_binding_points_to_terminal_task_graph_task(&binding)
                    .await?
                {
                    self.clear_run_graph_continuation_binding(&binding.run_id)
                        .await?;
                    return Ok(None);
                }
                binding.validate()?;
                Ok(Some(binding))
            }
            None => Ok(None),
        }
    }

    pub async fn record_run_graph_replay_lineage_receipt(
        &self,
        receipt: &RunGraphReplayLineageReceipt,
    ) -> Result<(), StateStoreError> {
        receipt.validate()?;
        let _: Option<RunGraphReplayLineageReceipt> = self
            .db
            .upsert((
                "run_graph_replay_lineage_receipt",
                receipt.receipt_id.as_str(),
            ))
            .content(receipt.clone())
            .await?;
        Ok(())
    }

    pub async fn record_run_graph_projection_checkpoint_record(
        &self,
        record: &RunGraphProjectionCheckpointRecord,
    ) -> Result<(), StateStoreError> {
        record.validate()?;
        let _: Option<RunGraphProjectionCheckpointRecord> = self
            .db
            .upsert((
                "run_graph_projection_checkpoint_record",
                record.record_id.as_str(),
            ))
            .content(record.clone())
            .await?;
        Ok(())
    }

    pub async fn clear_run_graph_projection_checkpoint_records(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query(format!(
                "DELETE run_graph_projection_checkpoint_record WHERE run_id = '{}';",
                escape_surql_literal(run_id)
            ))
            .await?;
        Ok(())
    }

    pub async fn run_graph_projection_checkpoint_record(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphProjectionCheckpointRecord>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_projection_checkpoint_record \
                 WHERE run_id = $run_id \
                 ORDER BY updated_at DESC, record_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<RunGraphProjectionCheckpointRecord> = query.take(0)?;
        match rows.into_iter().next() {
            Some(record) => {
                record.validate()?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn latest_run_graph_projection_checkpoint_record(
        &self,
    ) -> Result<Option<RunGraphProjectionCheckpointRecord>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        match self
            .run_graph_projection_checkpoint_record(&status.run_id)
            .await?
        {
            Some(record) if record.run_id == status.run_id => Ok(Some(record)),
            Some(record) => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph projection checkpoint record is inconsistent for `{}`: latest projection checkpoint record run_id must share the same run_id (record_run_id={})",
                    status.run_id, record.run_id
                ),
            }),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn run_graph_replay_lineage_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphReplayLineageReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_replay_lineage_receipt \
                 WHERE run_id = $run_id \
                 ORDER BY recorded_at DESC, receipt_id DESC \
                 LIMIT 1;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<RunGraphReplayLineageReceipt> = query.take(0)?;
        match rows.into_iter().next() {
            Some(receipt) => {
                receipt.validate()?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn latest_run_graph_replay_lineage_receipt(
        &self,
    ) -> Result<Option<RunGraphReplayLineageReceipt>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        match self
            .run_graph_replay_lineage_receipt(&status.run_id)
            .await?
        {
            Some(receipt) if receipt.run_id == status.run_id => Ok(Some(receipt)),
            Some(receipt) => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph replay lineage receipt is inconsistent for `{}`: latest replay lineage receipt run_id must share the same run_id (receipt_run_id={})",
                    status.run_id, receipt.run_id
                ),
            }),
            None => Ok(None),
        }
    }

    pub async fn latest_explicit_run_graph_continuation_binding(
        &self,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        self.latest_explicit_run_graph_continuation_binding_matching(|_| true)
            .await
    }

    pub async fn latest_explicit_run_graph_continuation_binding_for_current_session(
        &self,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let Some(scope) = self.current_session_run_graph_claim_scope().await? else {
            return Ok(None);
        };
        self.latest_explicit_run_graph_continuation_binding_matching(|binding| {
            scope.matches_binding(binding)
        })
        .await
    }

    async fn latest_explicit_run_graph_continuation_binding_matching(
        &self,
        mut matches_scope: impl FnMut(&RunGraphContinuationBinding) -> bool,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_continuation_binding \
                 WHERE binding_source = 'explicit_continuation_bind' \
                    OR binding_source = 'explicit_continuation_bind_task' \
                    OR binding_source = 'task_close_reconcile' \
                 ORDER BY recorded_at DESC, run_id DESC;",
            )
            .await?;
        let rows: Vec<RunGraphContinuationBinding> = query.take(0)?;
        for binding in rows {
            let Some(mut binding) = self.normalize_task_close_reconcile_binding(binding).await?
            else {
                continue;
            };
            binding = self
                .effective_exception_takeover_continuation_binding(binding)
                .await?;
            binding.validate()?;
            if !matches_scope(&binding) {
                continue;
            }
            if binding.binding_source != "explicit_continuation_bind_task" {
                return Ok(Some(binding));
            }

            let Some(task_id) = binding
                .active_bounded_unit
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            let task = match self.show_task(task_id).await {
                Ok(task) => task,
                Err(StateStoreError::MissingTask { .. }) => return Ok(Some(binding)),
                Err(error) => return Err(error),
            };
            if task_status_is_terminal_for_continuation(&task.status) {
                continue;
            }

            if let Some(active_bounded_unit) = binding.active_bounded_unit.as_object_mut() {
                active_bounded_unit.insert(
                    "task_status".to_string(),
                    serde_json::Value::String(task.status.clone()),
                );
                active_bounded_unit.insert(
                    "issue_type".to_string(),
                    serde_json::Value::String(task.issue_type.clone()),
                );
            }
            binding.validate()?;
            return Ok(Some(binding));
        }
        Ok(None)
    }

    async fn continuation_binding_points_to_terminal_task_graph_task(
        &self,
        binding: &RunGraphContinuationBinding,
    ) -> Result<bool, StateStoreError> {
        if binding.status != "bound"
            || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
        {
            return Ok(false);
        }
        let task_id = binding.active_bounded_unit["task_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(binding.task_id.as_str());
        let task = match self.show_task(task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        Ok(task_status_is_terminal_for_continuation(&task.status))
    }

    async fn normalize_task_close_reconcile_binding(
        &self,
        mut binding: RunGraphContinuationBinding,
    ) -> Result<Option<RunGraphContinuationBinding>, StateStoreError> {
        if binding.binding_source != "task_close_reconcile" {
            return Ok(Some(binding));
        }

        let binding_kind = binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str);
        if binding_kind != Some("run_graph_task") {
            return Ok(Some(binding));
        }

        let task = match self.show_task(&binding.task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => {
                self.clear_run_graph_continuation_binding(&binding.run_id)
                    .await?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if !task_status_is_terminal_for_continuation(&task.status) {
            return Ok(Some(binding));
        }
        if !self
            .task_close_reconcile_has_persisted_receipt_truth(&binding.run_id, &binding.task_id)
            .await?
        {
            self.clear_run_graph_continuation_binding(&binding.run_id)
                .await?;
            return Ok(None);
        }

        binding.active_bounded_unit = serde_json::json!({
            "kind": "downstream_dispatch_target",
            "task_id": binding.task_id,
            "run_id": binding.run_id,
            "dispatch_target": "closure",
        });
        binding.why_this_unit = "Closing the active task reconciled the run into a completed state and bound downstream closure as the next lawful bounded unit.".to_string();
        binding.sequential_vs_parallel_posture = "sequential_only".to_string();
        self.record_run_graph_continuation_binding(&binding).await?;
        Ok(Some(binding))
    }

    pub async fn clear_run_graph_continuation_binding(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let _: Option<RunGraphContinuationBinding> = self
            .db
            .delete(("run_graph_continuation_binding", run_id))
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn record_run_graph_dispatch_context(
        &self,
        context: &RunGraphDispatchContext,
    ) -> Result<(), StateStoreError> {
        context.validate()?;
        self.record_run_graph_owner_evidence(&context.run_id, "dispatch_context")
            .await?;
        let _: Option<RunGraphDispatchContext> = self
            .db
            .upsert(("run_graph_dispatch_context", context.run_id.as_str()))
            .content(context.clone())
            .await?;
        Ok(())
    }

    pub async fn run_graph_dispatch_context(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchContext>, StateStoreError> {
        let context: Option<RunGraphDispatchContext> = self
            .db
            .select(("run_graph_dispatch_context", run_id))
            .await?;
        match context {
            Some(context) => {
                context.validate()?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    pub async fn run_graph_status(&self, run_id: &str) -> Result<RunGraphStatus, StateStoreError> {
        self.run_graph_status_from_task_rows(run_id, &[]).await
    }

    async fn normalize_spec_first_work_pool_handoff_receipt_truth(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        if !receipt_has_pending_spec_first_work_pool_handoff_gate(receipt) {
            return Ok(false);
        }
        let tasks = self.list_tasks(None, true).await?;
        let identity = self
            .run_graph_dispatch_task_identity(&receipt.run_id)
            .await?;
        let mut candidates = Vec::new();
        if let Some(identity) = identity.as_ref() {
            candidates.extend(
                [
                    identity.dev_task_id.as_deref(),
                    identity.work_pool_task_id.as_deref(),
                    identity.spec_task_id.as_deref(),
                    identity.feature_epic_id.as_deref(),
                ]
                .into_iter()
                .flatten()
                .map(str::to_string),
            );
        }
        candidates.push(receipt.run_id.clone());
        let dev_gate = candidates.iter().find_map(|candidate| {
            Self::spec_first_dev_handoff_gate_satisfied_for_task(&tasks, candidate)
        });
        if let Some(gate) = dev_gate {
            let identity = Self::spec_first_dispatch_task_identity_from_tasks(
                &tasks,
                &receipt.run_id,
                &gate.feature_id,
                identity
                    .as_ref()
                    .map(|_| "spec_first_dev_handoff_identity_reconciliation")
                    .unwrap_or("spec_first_dev_handoff_reconciliation"),
            );
            self.record_run_graph_dispatch_task_identity(&identity)
                .await?;
            let dispatch_context = self.reconciled_pack_dispatch_context(receipt).await?;
            let resolved_dev_target = dispatch_context.as_ref().and_then(|(role_selection, _)| {
                crate::runtime_dispatch_state::first_runtime_dispatch_target_after_dev_pack(
                    role_selection,
                )
                .ok()
            });
            receipt.downstream_dispatch_blockers.retain(|blocker| {
                !matches!(
                    blocker.as_str(),
                    "pending_design_finalize"
                        | "pending_spec_task_close"
                        | "pending_specification_evidence"
                )
            });
            if let Some(resolved_dev_target) = resolved_dev_target {
                receipt.downstream_dispatch_target = Some(resolved_dev_target.dispatch_target);
                receipt.downstream_dispatch_command =
                    dispatch_context.as_ref().and_then(|(role_selection, _)| {
                        crate::runtime_dispatch_state::runtime_dispatch_command_for_target(
                            role_selection,
                            receipt
                                .downstream_dispatch_target
                                .as_deref()
                                .unwrap_or_default(),
                        )
                    });
            } else {
                receipt.downstream_dispatch_target = None;
                receipt.downstream_dispatch_command = None;
                if !receipt
                    .downstream_dispatch_blockers
                    .iter()
                    .any(|blocker| blocker == "missing_configured_runtime_dispatch_target")
                {
                    receipt
                        .downstream_dispatch_blockers
                        .push("missing_configured_runtime_dispatch_target".to_string());
                }
            }
            receipt.downstream_dispatch_ready = receipt.downstream_dispatch_blockers.is_empty()
                && receipt.downstream_dispatch_target.is_some();
            receipt.downstream_dispatch_status = Some(if receipt.downstream_dispatch_ready {
                "packet_ready".to_string()
            } else {
                "blocked".to_string()
            });
            receipt.downstream_dispatch_note = if let Some(target) =
                receipt.downstream_dispatch_target.as_deref()
            {
                Some(format!(
                    "spec-first feature `{}` has closed spec/work-pool tasks and open dev task `{}`; dispatch configured runtime target `{target}`",
                    gate.feature_id, gate.dev_task_id
                ))
            } else {
                Some(format!(
                    "spec-first feature `{}` has closed spec/work-pool tasks and open dev task `{}` but no configured runtime dispatch target could be resolved",
                    gate.feature_id, gate.dev_task_id
                ))
            };
            receipt.downstream_dispatch_active_target = Some("specification".to_string());
            receipt.downstream_dispatch_last_target = Some("specification".to_string());
            if receipt.downstream_dispatch_ready {
                if let Some((role_selection, run_graph_bootstrap)) = dispatch_context.as_ref() {
                    let dev_owned_paths = tasks
                        .iter()
                        .find(|task| task.id == gate.dev_task_id)
                        .map(|task| task.planner_metadata.owned_paths.clone())
                        .unwrap_or_default();
                    receipt.downstream_dispatch_packet_path =
                        crate::runtime_dispatch_downstream_packets::write_runtime_downstream_dispatch_packet_with_owned_paths(
                            self.root(),
                            role_selection,
                            run_graph_bootstrap,
                            receipt,
                            &dev_owned_paths,
                        )
                        .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
                    if let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() {
                        receipt.downstream_dispatch_command = Some(
                            crate::runtime_dispatch_state::agent_init_execute_command_for_packet_path(
                                packet_path,
                            ),
                        );
                    }
                }
                if receipt.downstream_dispatch_packet_path.is_none() {
                    receipt.downstream_dispatch_ready = false;
                    receipt.downstream_dispatch_status = Some("blocked".to_string());
                    receipt.downstream_dispatch_blockers =
                        vec!["missing_downstream_dispatch_packet".to_string()];
                    receipt.downstream_dispatch_note = Some(format!(
                        "spec-first feature `{}` resolved a dev runtime target, but no executable downstream packet could be produced",
                        gate.feature_id
                    ));
                }
            }
            return Ok(true);
        }
        let feature_id = identity
            .as_ref()
            .and_then(|identity| identity.feature_epic_id.as_deref())
            .and_then(|feature_id| {
                Self::spec_first_work_pool_handoff_gate_satisfied_for_task(&tasks, feature_id)
            })
            .or_else(|| {
                Self::spec_first_work_pool_handoff_gate_satisfied_for_task(&tasks, &receipt.run_id)
            });
        let Some(feature_id) = feature_id else {
            return Ok(false);
        };
        let identity = Self::spec_first_dispatch_task_identity_from_tasks(
            &tasks,
            &receipt.run_id,
            &feature_id,
            identity
                .as_ref()
                .map(|_| "spec_first_work_pool_handoff_identity_reconciliation")
                .unwrap_or("spec_first_work_pool_handoff_reconciliation"),
        );
        self.record_run_graph_dispatch_task_identity(&identity)
            .await?;
        self.repair_stale_spec_first_parent_auto_close_for_work_pool_handoff(&tasks, &feature_id)
            .await?;

        receipt.downstream_dispatch_blockers.retain(|blocker| {
            !matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
        receipt.downstream_dispatch_ready = receipt.downstream_dispatch_blockers.is_empty();
        receipt.downstream_dispatch_status = Some(if receipt.downstream_dispatch_ready {
            "packet_ready".to_string()
        } else {
            "blocked".to_string()
        });
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.downstream_dispatch_note = Some(format!(
            "spec-first feature `{feature_id}` has a closed spec-pack task; continue with work-pool handoff"
        ));
        receipt.downstream_dispatch_active_target = Some("specification".to_string());
        receipt.downstream_dispatch_last_target = Some("specification".to_string());
        Ok(true)
    }

    async fn normalize_materialized_pack_task_identity_and_receipt(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        if !matches!(receipt.dispatch_status.as_str(), "blocked" | "executed")
            || receipt.dispatch_surface.as_deref() != Some("vida task ensure")
            || !matches!(
                receipt.dispatch_target.as_str(),
                "work-pool-pack" | "dev-pack"
            )
            || !receipt
                .blocker_code
                .as_deref()
                .is_none_or(|code| code == "internal_activation_view_only")
        {
            return Ok(false);
        }
        let expected_packet_key = match receipt.dispatch_target.as_str() {
            "work-pool-pack" => "work_pool_task",
            "dev-pack" => "dev_task",
            _ => return Ok(false),
        };
        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt.clone());
        let Some(result) = tracked_flow_materialization_result(Some(self.root()), &stored_receipt)
        else {
            return Ok(false);
        };
        if result.packet_key != expected_packet_key {
            return Ok(false);
        }
        let tasks = self.list_tasks(None, true).await?;
        let materialized_task = match receipt.dispatch_target.as_str() {
            "work-pool-pack" => tasks
                .iter()
                .find(|task| task.id == result.task_id && task_is_work_pool_pack_child(task)),
            "dev-pack" => tasks
                .iter()
                .find(|task| task.id == result.task_id && task_has_label(task, "dev-pack")),
            _ => None,
        };
        let Some(materialized_task) = materialized_task else {
            return Ok(false);
        };
        let Some(feature_id) = Self::parent_id_for_task(materialized_task) else {
            return Ok(false);
        };
        if result
            .epic_task_id
            .as_deref()
            .is_some_and(|epic_task_id| epic_task_id != feature_id)
        {
            return Ok(false);
        }
        let child_tasks = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(feature_id.as_str()))
            .collect::<Vec<_>>();
        let spec_task_id = child_tasks
            .iter()
            .filter(|task| task_is_spec_pack_child(task))
            .map(|task| task.id.clone())
            .min();
        let identity = RunGraphDispatchTaskIdentity {
            run_id: receipt.run_id.clone(),
            feature_epic_id: Some(feature_id),
            spec_task_id,
            work_pool_task_id: if receipt.dispatch_target == "work-pool-pack" {
                Some(result.task_id.clone())
            } else {
                child_tasks
                    .iter()
                    .filter(|task| task_is_work_pool_pack_child(task))
                    .map(|task| task.id.clone())
                    .min()
            },
            dev_task_id: if receipt.dispatch_target == "dev-pack" {
                Some(result.task_id.clone())
            } else {
                child_tasks
                    .iter()
                    .filter(|task| task_has_label(task, "dev-pack"))
                    .map(|task| task.id.clone())
                    .min()
            },
            source: if receipt.dispatch_target == "work-pool-pack" {
                "work_pool_materialization_identity_reconciliation".to_string()
            } else {
                "dev_pack_materialization_identity_reconciliation".to_string()
            },
            updated_at: unix_timestamp_nanos().to_string(),
        };
        self.record_run_graph_dispatch_task_identity(&identity)
            .await?;
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.blocker_code = None;
        receipt.downstream_dispatch_target = Some(if receipt.dispatch_target == "work-pool-pack" {
            "dev-pack".to_string()
        } else {
            "closure".to_string()
        });
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        receipt.downstream_dispatch_note = Some(if receipt.dispatch_target == "work-pool-pack" {
            format!(
                "work-pool task `{}` is materialized; continue with the tracked dev packet",
                result.task_id
            )
        } else {
            format!(
                "dev task `{}` is materialized; continue with tracked flow closure",
                result.task_id
            )
        });
        receipt.downstream_dispatch_active_target = Some(receipt.dispatch_target.clone());
        receipt.downstream_dispatch_last_target = Some(receipt.dispatch_target.clone());
        if !self
            .materialize_reconciled_pack_downstream_packet(receipt)
            .await?
        {
            receipt.downstream_dispatch_ready = false;
            receipt.downstream_dispatch_status = Some("blocked".to_string());
            receipt.downstream_dispatch_blockers =
                vec!["missing_downstream_dispatch_packet".to_string()];
            receipt.downstream_dispatch_note = Some(format!(
                "materialized `{}` packet is reconciled, but no executable downstream packet could be produced",
                receipt.dispatch_target
            ));
        }
        Ok(true)
    }

    async fn materialize_reconciled_pack_downstream_packet(
        &self,
        receipt: &mut RunGraphDispatchReceipt,
    ) -> Result<bool, StateStoreError> {
        let Some(downstream_target) = receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            return Ok(false);
        };
        if receipt.dispatch_target != "work-pool-pack" {
            return Ok(true);
        }
        let Some((role_selection, run_graph_bootstrap)) =
            self.reconciled_pack_dispatch_context(receipt).await?
        else {
            return Ok(false);
        };
        receipt.downstream_dispatch_command =
            crate::runtime_dispatch_state::runtime_dispatch_command_for_target(
                &role_selection,
                &downstream_target,
            );
        if let Some(packet_path) = receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            receipt.downstream_dispatch_command = Some(
                crate::runtime_dispatch_state::agent_init_execute_command_for_packet_path(
                    packet_path,
                ),
            );
            return Ok(true);
        }
        receipt.downstream_dispatch_packet_path =
            crate::runtime_dispatch_downstream_packets::write_runtime_downstream_dispatch_packet(
                self.root(),
                &role_selection,
                &run_graph_bootstrap,
                receipt,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        if let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() {
            receipt.downstream_dispatch_command = Some(
                crate::runtime_dispatch_state::agent_init_execute_command_for_packet_path(
                    packet_path,
                ),
            );
        }
        Ok(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|path| !path.is_empty()))
    }

    async fn reconciled_pack_dispatch_context(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<Option<(RuntimeConsumptionLaneSelection, serde_json::Value)>, StateStoreError> {
        let Some(packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        let Some(packet_path) = self.canonical_reconciled_pack_dispatch_packet_path(packet_path)?
        else {
            return Ok(None);
        };
        let body = std::fs::read_to_string(&packet_path).map_err(|error| {
            invalid_reconciled_pack_dispatch_packet_error(
                &packet_path,
                format!("failed to read packet body: {error}"),
            )
        })?;
        let packet = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "Failed to parse materialized pack dispatch packet `{}`: {error}",
                    packet_path.display()
                ),
            }
        })?;
        let role_selection = serde_json::from_value::<RuntimeConsumptionLaneSelection>(
            packet
                .get("role_selection_full")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|error| StateStoreError::InvalidTaskRecord {
            reason: format!(
                "Failed to decode role_selection from materialized pack dispatch packet `{}`: {error}",
                packet_path.display()
            ),
        })?;
        let task_id = self
            .run_graph_dispatch_task_identity(&receipt.run_id)
            .await?
            .as_ref()
            .and_then(|identity| {
                dispatch_identity_task_id_for_target(identity, &receipt.dispatch_target)
            });
        let role_selection = crate::taskflow_run_graph::rehydrate_persisted_role_selection(
            self,
            role_selection,
            task_id.as_deref(),
        )
        .await
        .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        let run_graph_bootstrap = packet
            .get("run_graph_bootstrap")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok(Some((role_selection, run_graph_bootstrap)))
    }

    fn canonical_reconciled_pack_dispatch_packet_path(
        &self,
        packet_path: &str,
    ) -> Result<Option<std::path::PathBuf>, StateStoreError> {
        let packet_path =
            crate::runtime_dispatch_state::normalize_persisted_runtime_path(packet_path);
        if packet_path.as_os_str().is_empty() {
            return Ok(None);
        }
        if packet_path_has_dot_segment(&packet_path) {
            return Err(invalid_reconciled_pack_dispatch_packet_error(
                &packet_path,
                "dot-segment traversal is not admissible",
            ));
        }
        let candidate_path = if packet_path.is_absolute() {
            packet_path
        } else {
            self.root().join(packet_path)
        };
        let metadata = match std::fs::symlink_metadata(&candidate_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(invalid_reconciled_pack_dispatch_packet_error(
                    &candidate_path,
                    format!("failed to inspect packet path: {error}"),
                ));
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(invalid_reconciled_pack_dispatch_packet_error(
                &candidate_path,
                "symlink packet paths are not admissible; refusing to follow them",
            ));
        }
        let root = std::fs::canonicalize(self.root()).map_err(|error| {
            invalid_reconciled_pack_dispatch_packet_error(
                self.root(),
                format!("failed to canonicalize VIDA state root: {error}"),
            )
        })?;
        let candidate_path = std::fs::canonicalize(&candidate_path).map_err(|error| {
            invalid_reconciled_pack_dispatch_packet_error(
                &candidate_path,
                format!("failed to canonicalize packet path: {error}"),
            )
        })?;
        if !candidate_path.starts_with(&root) {
            return Err(invalid_reconciled_pack_dispatch_packet_error(
                &candidate_path,
                format!("escapes VIDA state root `{}`", root.display()),
            ));
        }
        if !metadata.is_file() {
            return Err(invalid_reconciled_pack_dispatch_packet_error(
                &candidate_path,
                "materialized pack dispatch packet is not a regular file",
            ));
        }
        if metadata.len() > MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES {
            return Err(invalid_reconciled_pack_dispatch_packet_error(
                &candidate_path,
                format!(
                    "materialized pack dispatch packet is {} bytes, exceeding the 4 MiB intake cap",
                    metadata.len()
                ),
            ));
        }
        Ok(Some(candidate_path))
    }

    fn spec_first_dispatch_task_identity_from_tasks(
        tasks: &[TaskRecord],
        run_id: &str,
        feature_id: &str,
        source: &str,
    ) -> RunGraphDispatchTaskIdentity {
        let child_tasks = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(feature_id))
            .collect::<Vec<_>>();
        let spec_task_id = child_tasks
            .iter()
            .filter(|task| task_is_spec_pack_child(task))
            .map(|task| task.id.clone())
            .min();
        let work_pool_task_id = child_tasks
            .iter()
            .filter(|task| task_is_work_pool_pack_child(task))
            .map(|task| task.id.clone())
            .min();

        RunGraphDispatchTaskIdentity {
            run_id: run_id.to_string(),
            feature_epic_id: Some(feature_id.to_string()),
            spec_task_id,
            work_pool_task_id,
            dev_task_id: child_tasks
                .iter()
                .filter(|task| task_has_label(task, "dev-pack"))
                .map(|task| task.id.clone())
                .min(),
            source: source.to_string(),
            updated_at: unix_timestamp_nanos().to_string(),
        }
    }

    pub async fn record_run_graph_dispatch_task_identity(
        &self,
        identity: &RunGraphDispatchTaskIdentity,
    ) -> Result<(), StateStoreError> {
        if identity.run_id.trim().is_empty() || identity.source.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "run-graph dispatch task identity requires run_id and source".to_string(),
            });
        }
        let _: Option<RunGraphDispatchTaskIdentity> = self
            .db
            .upsert(("run_graph_dispatch_task_identity", identity.run_id.as_str()))
            .content(identity.clone())
            .await?;
        crate::operator_projection_cache::touch_state_mutation_marker(self.root());
        Ok(())
    }

    pub async fn run_graph_dispatch_task_identity(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchTaskIdentity>, StateStoreError> {
        let identity: Option<RunGraphDispatchTaskIdentity> = match self
            .db
            .select(("run_graph_dispatch_task_identity", run_id))
            .await
        {
            Ok(identity) => identity,
            Err(error) if error.to_string().contains("does not exist") => None,
            Err(error) => return Err(error.into()),
        };
        Ok(identity)
    }

    pub(crate) async fn run_graph_raw_status_from_task_rows(
        &self,
        run_id: &str,
    ) -> Result<
        (
            RunGraphStatus,
            Option<RunGraphDispatchReceiptStored>,
            bool,
            bool,
            bool,
            bool,
        ),
        StateStoreError,
    > {
        let execution: Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?;
        let routed: Option<RoutedRunStateRow> =
            self.db.select(("routed_run_state", run_id)).await?;
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        let receipt = self.run_graph_dispatch_receipt_stored(run_id).await?;
        let identity = self.run_graph_dispatch_task_identity(run_id).await?;
        if execution.is_none() && routed.is_none() && receipt.is_none() && identity.is_none() {
            return Err(StateStoreError::MissingTask {
                task_id: format!("run_graph:{run_id}"),
            });
        }
        let missing_execution = execution.is_none();
        let missing_routed = routed.is_none();
        let missing_governance = governance.is_none();
        let missing_resumability = resumability.is_none();
        let fallback_dispatch_target = receipt
            .as_ref()
            .map(|receipt| receipt.dispatch_target.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown");
        let fallback_node = fallback_dispatch_target.replace('-', "_");
        let fallback_task_id = identity
            .as_ref()
            .and_then(|identity| {
                dispatch_identity_task_id_for_target(identity, fallback_dispatch_target)
            })
            .unwrap_or_else(|| run_id.to_string());
        let fallback_task_class = routed
            .as_ref()
            .map(|row| row.route_task_class.clone())
            .or_else(|| execution.as_ref().map(|row| row.task_class.clone()))
            .unwrap_or_else(|| "implementation".to_string());

        let status = RunGraphStatus {
            run_id: execution
                .as_ref()
                .map(|row| row.run_id.clone())
                .unwrap_or_else(|| run_id.to_string()),
            task_id: execution
                .as_ref()
                .map(|row| row.task_id.trim())
                .filter(|task_id| !task_id.is_empty())
                .map(str::to_string)
                .unwrap_or(fallback_task_id),
            task_class: execution
                .as_ref()
                .map(|row| row.task_class.clone())
                .unwrap_or_else(|| fallback_task_class.clone()),
            active_node: execution
                .as_ref()
                .map(|row| row.active_node.clone())
                .unwrap_or_else(|| fallback_dispatch_target.to_string()),
            next_node: execution.as_ref().and_then(|row| row.next_node.clone()),
            status: execution
                .as_ref()
                .map(|row| row.status.clone())
                .unwrap_or_else(|| "blocked".to_string()),
            route_task_class: routed
                .as_ref()
                .map(|row| row.route_task_class.clone())
                .unwrap_or(fallback_task_class),
            selected_backend: routed
                .as_ref()
                .map(|row| row.selected_backend.clone())
                .or_else(|| {
                    receipt
                        .as_ref()
                        .and_then(|receipt| receipt.selected_backend.clone())
                })
                .unwrap_or_else(|| "unknown".to_string()),
            lane_id: routed
                .as_ref()
                .map(|row| row.lane_id.clone())
                .unwrap_or_else(|| format!("{fallback_node}_lane")),
            lifecycle_stage: routed
                .as_ref()
                .map(|row| row.lifecycle_stage.clone())
                .unwrap_or_else(|| format!("{fallback_node}_blocked")),
            policy_gate: governance
                .as_ref()
                .map(|row| row.policy_gate.clone())
                .unwrap_or_else(|| {
                    if missing_execution {
                        "stale_missing_run_graph_execution".to_string()
                    } else if missing_routed {
                        "stale_missing_run_graph_route".to_string()
                    } else {
                        "stale_missing_run_graph_governance".to_string()
                    }
                }),
            handoff_state: governance
                .as_ref()
                .map(|row| row.handoff_state.clone())
                .unwrap_or_else(|| {
                    if missing_execution {
                        "blocked_missing_run_graph_execution".to_string()
                    } else if missing_routed {
                        "blocked_missing_run_graph_route".to_string()
                    } else {
                        "blocked_missing_run_graph_governance".to_string()
                    }
                }),
            context_state: governance
                .as_ref()
                .map(|row| row.context_state.clone())
                .unwrap_or_else(|| "stale_projection".to_string()),
            checkpoint_kind: resumability
                .as_ref()
                .map(|row| row.checkpoint_kind.clone())
                .unwrap_or_else(|| {
                    if missing_execution {
                        "missing_execution_plan_state".to_string()
                    } else if missing_routed {
                        "missing_routed_run_state".to_string()
                    } else {
                        "missing_resumability_capsule".to_string()
                    }
                }),
            resume_target: resumability
                .as_ref()
                .map(|row| row.resume_target.clone())
                .unwrap_or_else(|| "none".to_string()),
            recovery_ready: resumability
                .as_ref()
                .map(|row| row.recovery_ready)
                .unwrap_or(false),
        };
        Ok((
            status,
            receipt,
            missing_execution,
            missing_routed,
            missing_governance,
            missing_resumability,
        ))
    }

    pub(crate) async fn run_graph_status_from_task_rows(
        &self,
        run_id: &str,
        task_rows: &[TaskRecord],
    ) -> Result<RunGraphStatus, StateStoreError> {
        let (
            status,
            receipt,
            missing_execution,
            missing_routed,
            missing_governance,
            missing_resumability,
        ) = self.run_graph_raw_status_from_task_rows(run_id).await?;
        let authorized_rework_route = if let Some(receipt) = receipt.as_ref() {
            if receipt
                .dispatch_packet_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty())
            {
                crate::runtime_dispatch_result_evidence::
                    authorized_dispatch_rework_context_from_receipt_fields(
                        self,
                        &status.run_id,
                        &status.task_id,
                        receipt.downstream_dispatch_result_path.as_deref(),
                        receipt.dispatch_result_path.as_deref(),
                        receipt.dispatch_packet_path.as_deref(),
                        &receipt.dispatch_target,
                    )
                    .await
                    .map(|context| context.map(|context| context.route))
                    .map_err(|blocker| StateStoreError::InvalidTaskRecord {
                        reason: blocker.to_string(),
                    })?
            } else {
                None
            }
        } else {
            None
        };
        let status = reconcile_run_graph_status_with_dispatch_receipt_and_rework_route(
            status,
            receipt.as_ref(),
            authorized_rework_route.as_ref(),
        )?;
        let task = if task_rows.is_empty() {
            self.show_task(&status.task_id).await.ok()
        } else {
            task_rows
                .iter()
                .find(|task| task.id == status.task_id)
                .cloned()
        };
        let status =
            reconcile_run_graph_status_with_closed_task(status, task.as_ref(), receipt.as_ref());
        if missing_execution || missing_routed || missing_governance || missing_resumability {
            let receipt_reconciled_dispatch_resume = status.recovery_ready
                && status.resume_target.starts_with("dispatch.")
                && status.checkpoint_kind == "execution_cursor"
                && status.next_node.is_some()
                && status.handoff_state.starts_with("awaiting_")
                && status.policy_gate == "not_required";
            if receipt_reconciled_dispatch_resume {
                status.validate_memory_governance()?;
                return Ok(status);
            }
            let checkpoint_kind = if missing_execution {
                "missing_execution_plan_state"
            } else if missing_routed {
                "missing_routed_run_state"
            } else if missing_resumability {
                "missing_resumability_capsule"
            } else {
                status.checkpoint_kind.as_str()
            };
            return Ok(RunGraphStatus {
                status: "blocked".to_string(),
                checkpoint_kind: checkpoint_kind.to_string(),
                recovery_ready: false,
                resume_target: "none".to_string(),
                ..status
            });
        }
        status.validate_memory_governance()?;
        Ok(status)
    }

    pub async fn record_run_graph_approval_delegation_receipt(
        &self,
        receipt: &RunGraphApprovalDelegationReceipt,
    ) -> Result<(), StateStoreError> {
        let receipt = receipt.clone();
        ensure_run_graph_approval_delegation_receipt_consistency(&receipt)?;
        let _: Option<RunGraphApprovalDelegationReceipt> = self
            .db
            .upsert((
                "run_graph_approval_delegation_receipt",
                receipt.run_id.as_str(),
            ))
            .content(receipt)
            .await?;
        Ok(())
    }

    pub async fn run_graph_approval_delegation_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphApprovalDelegationReceipt>, StateStoreError> {
        let receipt: Option<RunGraphApprovalDelegationReceipt> = self
            .db
            .select(("run_graph_approval_delegation_receipt", run_id))
            .await?;
        Ok(match receipt {
            Some(receipt) => Some(
                ensure_run_graph_approval_delegation_receipt_consistency(&receipt)
                    .map(|()| receipt)?,
            ),
            None => None,
        })
    }

    pub async fn latest_run_graph_status(&self) -> Result<Option<RunGraphStatus>, StateStoreError> {
        self.latest_run_graph_status_from_task_rows(&[]).await
    }

    pub async fn latest_run_graph_status_for_current_session(
        &self,
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        if let Some(binding) = self
            .latest_explicit_run_graph_continuation_binding_for_current_session()
            .await?
        {
            let bound_run_id = binding
                .active_bounded_unit
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(binding.run_id.as_str());
            if !bound_run_id.is_empty()
                && !self
                    .run_graph_latest_receipt_row_supersedes_current_session_lane(bound_run_id)
                    .await?
            {
                match self
                    .run_graph_status_from_task_rows(bound_run_id, &[])
                    .await
                {
                    Ok(status)
                        if !self
                            .run_graph_status_has_completed_exception_takeover_supersession(&status)
                            .await?
                            && !self
                                .run_graph_status_points_to_terminal_task_active(&status)
                                .await? =>
                    {
                        return Ok(Some(status));
                    }
                    Ok(_) | Err(StateStoreError::MissingTask { .. }) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let Some(scope) = self.current_session_run_graph_claim_scope().await? else {
            return Ok(None);
        };
        let mut seen_run_ids = std::collections::BTreeSet::new();
        for run_id in scope.run_ids {
            if !seen_run_ids.insert(run_id.clone()) {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_current_session_lane(&run_id)
                .await?
            {
                continue;
            }
            match self.run_graph_status_from_task_rows(&run_id, &[]).await {
                Ok(status) => {
                    if self
                        .run_graph_status_has_completed_exception_takeover_supersession(&status)
                        .await?
                    {
                        continue;
                    }
                    if self
                        .run_graph_status_points_to_terminal_task_active(&status)
                        .await?
                    {
                        continue;
                    }
                    return Ok(Some(status));
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        for task_id in scope.task_ids {
            let Some(run_id) = self.latest_run_graph_run_id_for_task(&task_id).await? else {
                continue;
            };
            if !seen_run_ids.insert(run_id.clone()) {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_current_session_lane(&run_id)
                .await?
            {
                continue;
            }
            match self.run_graph_status_from_task_rows(&run_id, &[]).await {
                Ok(status) => {
                    if self
                        .run_graph_status_has_completed_exception_takeover_supersession(&status)
                        .await?
                    {
                        continue;
                    }
                    if self
                        .run_graph_status_points_to_terminal_task_active(&status)
                        .await?
                    {
                        continue;
                    }
                    return Ok(Some(status));
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub(crate) async fn latest_run_graph_status_from_task_rows(
        &self,
        task_rows: &[TaskRecord],
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let Some(run_id) = self
            .latest_run_graph_run_id_from_task_rows(task_rows)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            self.run_graph_status_from_task_rows(&run_id, task_rows)
                .await?,
        ))
    }

    pub(crate) async fn latest_run_graph_run_id(&self) -> Result<Option<String>, StateStoreError> {
        self.latest_run_graph_run_id_from_task_rows(&[]).await
    }

    pub(crate) async fn latest_run_graph_run_id_from_task_rows(
        &self,
        _task_rows: &[TaskRecord],
    ) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphLatestStateRow> = query.take(0)?;
        for latest in rows {
            let terminal_task_active = self
                .run_graph_latest_row_points_to_terminal_task_active(&latest)
                .await?;
            let task = self.show_task(&latest.task_id).await.ok();
            let task_exists = task.is_some();
            let receipt = self
                .run_graph_dispatch_receipt_stored(&latest.run_id)
                .await?;
            let active_receipt = self
                .run_graph_dispatch_receipt_has_active_lane_evidence(&latest.run_id)
                .await?;
            let task_is_closed = task
                .as_ref()
                .is_some_and(|task| StateStore::task_status_is_closed_like(&task.status));
            let lawful_takeover = if task_is_closed {
                let status = self.run_graph_status(&latest.run_id).await?;
                lawful_current_exception_takeover(&status, receipt.as_ref())
            } else {
                false
            };
            if task_is_closed && !lawful_takeover {
                continue;
            }
            if !task_is_closed && terminal_task_active && (task_exists || !active_receipt) {
                continue;
            }
            if !lawful_takeover
                && self
                    .run_graph_latest_receipt_row_supersedes_lane(&latest.run_id)
                    .await?
            {
                continue;
            }
            return Ok(Some(latest.run_id));
        }
        let mut receipt_query = self
            .db
            .query(
                "SELECT * FROM run_graph_dispatch_receipt ORDER BY recorded_at DESC, run_id DESC LIMIT 25;",
            )
            .await?;
        let receipts: Vec<RunGraphDispatchReceiptStored> = receipt_query.take(0)?;
        for receipt in receipts {
            Self::ensure_run_graph_dispatch_receipt_required_fields_present(&receipt)?;
            let run_id = receipt.run_id.trim();
            if run_id.is_empty() {
                continue;
            }
            match self.run_graph_status_from_task_rows(run_id, &[]).await {
                Ok(status) => {
                    let task = self.show_task(&status.task_id).await.ok();
                    let task_is_closed = task
                        .as_ref()
                        .is_some_and(|task| StateStore::task_status_is_closed_like(&task.status));
                    let lawful_takeover =
                        lawful_current_exception_takeover(&status, Some(&receipt));
                    if task_is_closed && !lawful_takeover {
                        continue;
                    }
                    if !lawful_takeover
                        && self
                            .run_graph_latest_receipt_row_supersedes_lane(run_id)
                            .await?
                    {
                        continue;
                    }
                    if lawful_takeover
                        || self
                            .run_graph_dispatch_receipt_has_active_lane_evidence(run_id)
                            .await?
                        || !self
                            .run_graph_status_points_to_terminal_task_active(&status)
                            .await?
                    {
                        return Ok(Some(run_id.to_string()));
                    }
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub(crate) async fn latest_terminal_task_active_run_graph_status(
        &self,
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphLatestStateRow> = query.take(0)?;
        for latest in rows {
            let terminal_task_active = self
                .run_graph_latest_row_points_to_terminal_task_active(&latest)
                .await?;
            if !terminal_task_active && self.show_task(&latest.task_id).await.is_ok() {
                continue;
            }
            if self
                .run_graph_latest_receipt_row_supersedes_lane(&latest.run_id)
                .await?
            {
                continue;
            }
            if self
                .show_task(&latest.task_id)
                .await
                .ok()
                .is_some_and(|task| StateStore::task_status_is_closed_like(&task.status))
            {
                continue;
            }
            let status = self.run_graph_status(&latest.run_id).await?;
            let active_dispatch =
                status.recovery_ready && status.resume_target.starts_with("dispatch.");
            let task_is_terminal = self
                .show_task(&latest.task_id)
                .await
                .ok()
                .is_some_and(|task| task_status_is_terminal_for_continuation(&task.status));
            let active_receipt = self
                .run_graph_dispatch_receipt_has_active_lane_evidence(&latest.run_id)
                .await?;
            let delegated_cycle_open = status.delegation_gate().delegated_cycle_open;
            if (!delegated_cycle_open && task_is_terminal)
                || (!delegated_cycle_open && !active_dispatch && !active_receipt)
            {
                continue;
            }
            return Ok(Some(status));
        }
        Ok(None)
    }

    async fn run_graph_latest_receipt_row_supersedes_lane(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(false);
        };
        Ok((receipt.lane_status.as_deref() == Some("lane_superseded")
            && has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref()))
            || stored_receipt_has_active_exception_takeover(&receipt))
    }

    async fn run_graph_latest_receipt_row_supersedes_current_session_lane(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(false);
        };
        Ok(receipt.lane_status.as_deref() == Some("lane_superseded")
            && has_receipt_evidence_id(receipt.supersedes_receipt_id.as_deref()))
    }

    async fn run_graph_dispatch_receipt_has_active_lane_evidence(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(false);
        };
        Ok(receipt.dispatch_status != "executed"
            && matches!(
                receipt.lane_status.as_deref(),
                Some("lane_open")
                    | Some("lane_running")
                    | Some("lane_blocked")
                    | Some("lane_exception_recorded")
                    | Some("lane_exception_takeover")
            ))
    }

    async fn run_graph_status_has_completed_exception_takeover_supersession(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(false);
        };
        Ok(crate::runtime_dispatch_receipt_helpers::exception_takeover_dispatch_blocker_superseded_by_completed_node(
            status, &receipt,
        ))
    }

    async fn run_graph_latest_row_points_to_terminal_task_active(
        &self,
        latest: &RunGraphLatestStateRow,
    ) -> Result<bool, StateStoreError> {
        if self.run_graph_latest_row_points_to_terminal_task_active_from_rows(latest, &[])? {
            return Ok(true);
        }
        if latest.status == "completed" {
            return Ok(false);
        }
        let status = self.run_graph_status(&latest.run_id).await?;

        Ok(
            crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                self, &status,
            )
            .await?
            .stale_for_active_projection(),
        )
    }

    async fn run_graph_status_points_to_terminal_task_active(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        if status.status == "completed" {
            return Ok(false);
        }
        Ok(
            crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                self, status,
            )
            .await?
            .stale_for_active_projection(),
        )
    }

    async fn run_graph_status_points_to_closed_task_active(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        if status.status == "completed" {
            return Ok(false);
        }
        let Some(task) = self.show_task(&status.task_id).await.ok() else {
            return Ok(false);
        };
        if !task_status_is_terminal_for_continuation(&task.status) {
            return Ok(false);
        }
        self.run_graph_status_points_to_terminal_task_active(status)
            .await
    }

    fn run_graph_latest_row_points_to_terminal_task_active_from_rows(
        &self,
        latest: &RunGraphLatestStateRow,
        task_rows: &[TaskRecord],
    ) -> Result<bool, StateStoreError> {
        if latest.status == "completed" {
            return Ok(false);
        }
        if task_rows.is_empty() {
            return Ok(false);
        }
        Ok(task_rows
            .iter()
            .find(|task| task.id == latest.task_id)
            .is_some_and(|task| task_status_is_terminal_for_continuation(&task.status)))
    }

    pub(crate) async fn latest_run_graph_run_id_for_task(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM execution_plan_state WHERE task_id = $task_id ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
            .bind(("task_id", task_id.to_string()))
            .await?;
        let rows: Vec<RunGraphLatestRow> = query.take(0)?;
        Ok(rows.into_iter().next().map(|latest| latest.run_id))
    }

    pub(crate) async fn latest_run_graph_status_for_task(
        &self,
        task_id: &str,
    ) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state WHERE task_id = $task_id ORDER BY updated_at DESC, run_id DESC LIMIT 25;",
            )
            .bind(("task_id", task_id.to_string()))
            .await?;
        let rows: Vec<RunGraphLatestStateRow> = query.take(0)?;
        for latest in rows {
            if self
                .run_graph_latest_receipt_row_supersedes_current_session_lane(&latest.run_id)
                .await?
            {
                continue;
            }
            match self
                .run_graph_status_from_task_rows(&latest.run_id, &[])
                .await
            {
                Ok(status) => {
                    if status.task_id.trim() != task_id.trim()
                        || self
                            .run_graph_status_points_to_terminal_task_active(&status)
                            .await?
                    {
                        continue;
                    }
                    return Ok(Some(status));
                }
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        if let Ok(status) = self.run_graph_status(task_id).await {
            if status.task_id.trim() == task_id.trim()
                && !self
                    .run_graph_status_points_to_terminal_task_active(&status)
                    .await?
            {
                return Ok(Some(status));
            }
        }
        Ok(None)
    }

    pub(crate) async fn run_graph_status_for_operator_selector(
        &self,
        selector: &str,
    ) -> Result<RunGraphStatus, StateStoreError> {
        let selector = selector.trim();
        if selector.is_empty() {
            return Err(StateStoreError::MissingTask {
                task_id: "run_graph:<empty>".to_string(),
            });
        }

        let (status, resolved_from_task) = match self.run_graph_status(selector).await {
            Ok(status) => (status, false),
            Err(StateStoreError::MissingTask { .. }) => {
                if let Some(status) = self.latest_run_graph_status_for_task(selector).await? {
                    (status, true)
                } else {
                    let Some(run_id) = self.latest_run_graph_run_id_for_task(selector).await?
                    else {
                        return Err(StateStoreError::MissingTask {
                            task_id: format!("run_graph:{selector}"),
                        });
                    };
                    (self.run_graph_status(&run_id).await?, true)
                }
            }
            Err(error) => return Err(error),
        };

        if resolved_from_task && status.task_id.trim() != selector {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "scoped run-graph selector resolved to a different task: requested `{selector}`, status task `{}`",
                    status.task_id
                ),
            });
        }

        let task = self.show_task(&status.task_id).await.ok();
        let task_is_closed = task
            .as_ref()
            .is_some_and(|task| task_status_is_terminal_for_continuation(&task.status));
        let receipt = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?;
        let active_lawful_exception_takeover =
            lawful_current_exception_takeover(&status, receipt.as_ref());
        let stale_host_bridge_archive = receipt
            .as_ref()
            .is_some_and(stored_receipt_has_stale_host_bridge_handoff);
        let receiptless_host_bridge_archive =
            receipt.is_none() && receiptless_closed_task_operator_archive_sentinel(&status);
        if task_is_closed
            && !active_lawful_exception_takeover
            && (terminal_closure_status(&status)
                || stale_host_bridge_archive
                || receiptless_host_bridge_archive)
        {
            return Ok(project_closed_task_scoped_run_graph_status(status));
        }
        Ok(status)
    }

    async fn ensure_run_graph_recovery_surface_rows_present(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        if governance.is_none() || resumability.is_none() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/checkpoint summary is inconsistent for `{run_id}`: latest status requires both governance and resumability rows (governance_present={}, resumability_present={})",
                    governance.is_some(),
                    resumability.is_some()
                ),
            });
        }
        Ok(())
    }

    async fn ensure_run_graph_recovery_surface_has_checkpoint_lineage(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.checkpoint_kind.trim().eq_ignore_ascii_case("none") {
            return Ok(());
        }
        match self
            .run_graph_projection_checkpoint_record(&status.run_id)
            .await?
        {
            Some(record) if record.run_id == status.run_id => Ok(()),
            Some(record) => {
                self.clear_run_graph_projection_checkpoint_records(&record.run_id)
                    .await?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn ensure_run_graph_recovery_surface_consistency(
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.recovery_ready
            && status.resume_target.starts_with("dispatch.")
            && !is_dispatch_resume_handoff_done(status)
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/gate summary is inconsistent for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
                    status.run_id,
                    status.resume_target,
                    status.next_node.as_deref().unwrap_or("none"),
                    status.policy_gate,
                    status.handoff_state
                ),
            });
        }
        Ok(())
    }

    pub async fn ensure_memory_governance_guard(&self) -> Result<(), StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(());
        };
        status.validate_memory_governance()
    }

    pub async fn latest_run_graph_dispatch_receipt_summary(
        &self,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        self.run_graph_dispatch_receipt_summary_for_status(&status)
            .await
    }

    pub async fn latest_run_graph_dispatch_receipt_summary_for_current_session(
        &self,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status_for_current_session().await? else {
            return Ok(None);
        };
        self.run_graph_dispatch_receipt_summary_for_status(&status)
            .await
    }

    pub(crate) async fn run_graph_dispatch_receipt_summary_for_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        terminal_closure_supersedes_stale_handoff_receipt(&status, &mut receipt);
        terminal_closure_historicalizes_active_exception_takeover_receipt(&status, &mut receipt);
        let host_runtime = crate::taskflow_task_bridge::infer_project_root_from_state_root(
            self.root(),
        )
        .map(|project_root| {
            crate::runtime_dispatch_state::runtime_host_execution_contract_for_root(&project_root)
        });
        let role_selection = self
            .run_graph_dispatch_context(&status.run_id)
            .await?
            .map(|context| context.role_selection())
            .transpose()?;
        let canonical_selected_backend = role_selection
            .as_ref()
            .and_then(|selection| {
                crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
                    selection, &receipt,
                )
            })
            .or_else(|| receipt.selected_backend.clone());
        let effective_execution_posture = {
            let mut summary = crate::runtime_dispatch_state::effective_execution_posture_summary(
                role_selection
                    .as_ref()
                    .map(|selection| &selection.execution_plan)
                    .unwrap_or(&serde_json::Value::Null),
                &receipt.dispatch_target,
                canonical_selected_backend.as_deref(),
                receipt.activation_agent_type.as_deref(),
                host_runtime.as_ref(),
                crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(&receipt),
                None,
            );
            let activation_evidence =
                crate::runtime_dispatch_state::dispatch_activation_evidence_summary(&receipt);
            if let Some(object) = summary.as_object_mut() {
                object.insert(
                    "activation_kind".to_string(),
                    activation_evidence["activation_kind"].clone(),
                );
                object.insert(
                    "execution_evidence_path".to_string(),
                    activation_evidence["execution_evidence_path"].clone(),
                );
                object.insert(
                    "receipt_backed".to_string(),
                    activation_evidence["receipt_backed"].clone(),
                );
            }
            summary
        };
        let route_policy = role_selection
            .as_ref()
            .map(|selection| {
                crate::runtime_dispatch_state::dispatch_execution_route_summary(
                    selection,
                    &receipt.dispatch_target,
                    canonical_selected_backend.as_deref(),
                    None,
                )
            })
            .unwrap_or(serde_json::Value::Null);
        let activation_evidence =
            crate::runtime_dispatch_state::dispatch_activation_evidence_summary(&receipt);
        let mut summary = RunGraphDispatchReceiptSummary::from_receipt(receipt)
            .with_effective_execution_posture(effective_execution_posture)
            .with_route_policy(route_policy)
            .with_activation_evidence(activation_evidence);
        summary.selected_backend = canonical_selected_backend;
        Ok(Some(summary))
    }

    pub async fn latest_run_graph_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        terminal_closure_supersedes_stale_handoff_receipt(&status, &mut receipt);
        Ok(Some(receipt))
    }

    pub async fn latest_active_exception_takeover_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_dispatch_receipt \
                 WHERE lane_status = 'lane_exception_takeover' \
                 ORDER BY recorded_at DESC, run_id DESC \
                 LIMIT 25;",
            )
            .await?;
        let rows: Vec<RunGraphDispatchReceiptStored> = query.take(0)?;
        for receipt in rows {
            if !stored_receipt_has_active_exception_takeover(&receipt) {
                continue;
            }
            let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
            let status = match self.run_graph_status(&receipt.run_id).await {
                Ok(status) => status,
                Err(StateStoreError::MissingTask { .. }) => continue,
                Err(error) => return Err(error),
            };
            if active_exception_takeover_receipt_is_behind_status(&status, &receipt) {
                continue;
            }
            return Ok(Some(receipt.into()));
        }
        Ok(None)
    }

    pub async fn run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let status = self.run_graph_status(run_id).await.ok();
        self.run_graph_dispatch_receipt_for_status(run_id, status.as_ref())
            .await
    }

    pub async fn run_graph_dispatch_receipt_for_packet(
        &self,
        run_id: &str,
        dispatch_packet_path: &str,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        if let Some(receipt) = self.run_graph_dispatch_receipt(run_id).await? {
            if dispatch_packet_path_matches_receipt(
                receipt.dispatch_packet_path.as_deref(),
                dispatch_packet_path,
            ) || dispatch_packet_path_matches_receipt(
                receipt.downstream_dispatch_packet_path.as_deref(),
                dispatch_packet_path,
            ) {
                return Ok(Some(receipt));
            }
        }
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_dispatch_lane_receipt \
                 WHERE run_id = $run_id \
                 ORDER BY recorded_at DESC, dispatch_target DESC \
                 LIMIT 100;",
            )
            .bind(("run_id", run_id.to_string()))
            .await?;
        let rows: Vec<RunGraphDispatchReceiptStored> = query.take(0)?;
        let Some(receipt) = rows.into_iter().find(|receipt| {
            dispatch_packet_path_matches_receipt(
                receipt.dispatch_packet_path.as_deref(),
                dispatch_packet_path,
            ) || dispatch_packet_path_matches_receipt(
                receipt.downstream_dispatch_packet_path.as_deref(),
                dispatch_packet_path,
            )
        }) else {
            return Ok(None);
        };
        let status = self.run_graph_status(run_id).await.ok();
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        if let Some(status) = status.as_ref() {
            terminal_closure_supersedes_stale_handoff_receipt(status, &mut receipt);
        }
        Ok(Some(receipt))
    }

    pub(crate) async fn run_graph_dispatch_receipt_for_status(
        &self,
        run_id: &str,
        status: Option<&RunGraphStatus>,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let Some(receipt) = self.run_graph_dispatch_receipt_stored(run_id).await? else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        if let Some(status) = status {
            terminal_closure_supersedes_stale_handoff_receipt(status, &mut receipt);
            terminal_closure_historicalizes_active_exception_takeover_receipt(status, &mut receipt);
        }
        Ok(Some(receipt))
    }

    async fn run_graph_dispatch_receipt_stored(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceiptStored>, StateStoreError> {
        let receipt: Option<RunGraphDispatchReceiptStored> = self
            .db
            .select(("run_graph_dispatch_receipt", run_id))
            .await?;
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(&receipt)?;
        let receipt = normalize_legacy_downstream_preview_drift(receipt);
        let (receipt, lane_status_repaired) =
            normalize_repairable_in_flight_receipt_lane_status_drift(receipt);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        if lane_status_repaired {
            let public_receipt: RunGraphDispatchReceipt = receipt.clone().into();
            self.record_run_graph_dispatch_receipt(&public_receipt)
                .await?;
        }
        let mut receipt: RunGraphDispatchReceipt = receipt.into();
        let pre_execution_packet_ready =
            crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_pre_execution_packet_ready(
                &receipt,
                Some(run_id),
            );
        if !pre_execution_packet_ready
            && crate::runtime_dispatch_state::normalize_stale_in_flight_dispatch_receipt(
                self.root(),
                &mut receipt,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if crate::runtime_dispatch_state::normalize_activation_view_only_receipt_truth(&mut receipt)
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if self
            .normalize_spec_first_work_pool_handoff_receipt_truth(&mut receipt)
            .await?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        if self
            .normalize_materialized_pack_task_identity_and_receipt(&mut receipt)
            .await?
        {
            self.record_run_graph_dispatch_receipt(&receipt).await?;
        }
        self.normalize_task_close_reconcile_dispatch_receipt(receipt.into())
            .await
            .map(Some)
    }

    async fn normalize_task_close_reconcile_dispatch_receipt(
        &self,
        mut receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        let Some(binding) = self.run_graph_continuation_binding(&receipt.run_id).await? else {
            return Ok(receipt);
        };
        if binding.binding_source != "task_close_reconcile"
            || binding.active_bounded_unit["kind"].as_str() != Some("downstream_dispatch_target")
            || binding.active_bounded_unit["dispatch_target"].as_str() != Some("closure")
        {
            return Ok(receipt);
        }

        let closure_receipt_is_already_materialized = receipt.downstream_dispatch_target.as_deref()
            == Some("closure")
            && matches!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready") | Some("executed")
            )
            && receipt.downstream_dispatch_blockers.is_empty()
            && receipt
                .downstream_dispatch_result_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
        if closure_receipt_is_already_materialized {
            return Ok(receipt);
        }

        let Some(dispatch_packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(receipt);
        };
        if !self
            .task_close_reconcile_has_persisted_receipt_truth(&receipt.run_id, &binding.task_id)
            .await?
        {
            return Ok(receipt);
        }

        let completion_receipt_id = format!("task-close-{}", binding.task_id);
        let completion_result_path =
            crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                self.root(),
                &receipt.run_id,
                "closure",
                &completion_receipt_id,
                dispatch_packet_path,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            receipt.run_id
        ));
        receipt.downstream_dispatch_note =
            Some("task close reconciled the run into lawful closure".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_result_path = Some(completion_result_path);
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_active_target = Some("closure".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.lane_status = Some("lane_completed".to_string());
        let public_receipt: RunGraphDispatchReceipt = receipt.clone().into();
        self.record_run_graph_dispatch_receipt(&public_receipt)
            .await?;
        Ok(receipt)
    }

    pub async fn latest_run_graph_recovery_summary(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_recovery_summary_for_current_session(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status_for_current_session().await? else {
            return Ok(None);
        };
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_checkpoint_summary(
        &self,
    ) -> Result<Option<RunGraphCheckpointSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphCheckpointSummary::from_status(status)))
    }

    pub async fn latest_run_graph_gate_summary(
        &self,
    ) -> Result<Option<RunGraphGateSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        if self
            .run_graph_status_is_stale_after_release_admission_complete(&status)
            .await?
        {
            return Ok(None);
        }
        Ok(Some(RunGraphGateSummary::from_status(status)))
    }

    pub async fn run_graph_recovery_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphRecoverySummary, StateStoreError> {
        let status = self.load_consistent_run_graph_status(run_id).await?;
        Ok(RunGraphRecoverySummary::from_status(status))
    }

    async fn load_consistent_run_graph_status(
        &self,
        run_id: &str,
    ) -> Result<RunGraphStatus, StateStoreError> {
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(status)
    }

    pub(crate) async fn run_graph_status_is_stale_after_release_admission_complete(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        self.run_graph_status_is_stale_after_release_admission_complete_from_task_rows(status, &[])
            .await
    }

    pub(crate) async fn run_graph_status_is_stale_for_task_continuation_binding(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        self.run_graph_status_is_stale_for_task_continuation_binding_from_task_rows(status, &[])
            .await
    }

    pub(crate) async fn run_graph_status_is_stale_for_task_continuation_binding_from_task_rows(
        &self,
        status: &RunGraphStatus,
        task_rows: &[TaskRecord],
    ) -> Result<bool, StateStoreError> {
        if Self::run_graph_status_is_reconciled_terminal_closure(status) {
            return Ok(true);
        }
        if self
            .run_graph_status_points_to_closed_task_active(status)
            .await?
        {
            return Ok(true);
        }
        self.run_graph_status_is_stale_after_release_admission_complete_from_task_rows(
            status, task_rows,
        )
        .await
    }

    pub(crate) async fn run_graph_status_is_stale_after_release_admission_complete_from_task_rows(
        &self,
        status: &RunGraphStatus,
        task_rows: &[TaskRecord],
    ) -> Result<bool, StateStoreError> {
        let blocked_or_open_cycle = status.status == "blocked"
            || status.lifecycle_stage.ends_with("_blocked")
            || status.delegation_gate().delegated_cycle_open;
        if !blocked_or_open_cycle {
            return Ok(false);
        }
        let release_admission_complete =
            crate::runtime_consumption_state::release_admission_operator_evidence_complete_for_run(
                self.root(),
                &status.run_id,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        if !release_admission_complete {
            return Ok(false);
        }
        if task_rows.is_empty() {
            match self.show_task(&status.task_id).await {
                Ok(task) => Ok(Self::task_status_is_closed_like(&task.status)),
                Err(StateStoreError::MissingTask { .. }) => Ok(true),
                Err(error) => Err(error),
            }
        } else {
            Ok(task_rows
                .iter()
                .find(|task| task.id == status.task_id)
                .map(|task| Self::task_status_is_closed_like(&task.status))
                .unwrap_or(true))
        }
    }

    pub async fn run_graph_checkpoint_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphCheckpointSummary, StateStoreError> {
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphCheckpointSummary::from_status(status))
    }

    pub async fn run_graph_gate_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphGateSummary, StateStoreError> {
        let status = self.run_graph_status(run_id).await?;
        self.ensure_run_graph_recovery_surface_has_checkpoint_lineage(&status)
            .await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphGateSummary::from_status(status))
    }

    fn ensure_run_graph_dispatch_receipt_summary_consistency(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(receipt)?;
        let Some(raw_lane_status) = receipt.lane_status.as_deref() else {
            return Ok(());
        };
        let raw_lane_status = raw_lane_status.trim();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        let effective_derived_lane_status = if downstream_dispatch_allows_completed_lane_status(
            receipt.downstream_dispatch_status.as_deref(),
            canonical_lane_status,
        ) {
            "lane_completed".to_string()
        } else {
            normalize_run_graph_lane_status(
                Some(raw_lane_status),
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
        };
        let stale_blocked_lane_label = receipt.dispatch_status == "blocked"
            && receipt.downstream_dispatch_status.as_deref() == Some("blocked")
            && raw_lane_status.ends_with("_lane")
            && receipt.blocker_code.as_deref().is_some_and(|value| {
                matches!(
                    value.trim(),
                    "host_bridge_completion_result_blocked" | "host_bridge_completion_blocked"
                )
            });
        if receipt.downstream_dispatch_status.is_some()
            && canonical_lane_status != effective_derived_lane_status
            && !stale_blocked_lane_label
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_status `{}` with lane_status `{}` conflicts with derived lane_status `{}` from dispatch_status `{}`",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none"),
                    canonical_lane_status,
                    effective_derived_lane_status,
                    receipt.dispatch_status
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_dispatch_receipt_required_fields_present(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        if receipt.dispatch_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: dispatch_status must be non-empty",
                    receipt.run_id
                ),
            });
        }
        let Some(raw_lane_status) = receipt.lane_status.as_deref() else {
            return Ok(());
        };
        if raw_lane_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: lane_status must be non-empty when present",
                    receipt.run_id
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        let Some(downstream_status) = receipt.downstream_dispatch_status.as_deref() else {
            return Ok(());
        };
        let downstream_status = downstream_status.trim().to_ascii_lowercase();
        let requires_blockers = downstream_status == "blocked";
        if requires_blockers && receipt.downstream_dispatch_blockers.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must be present and non-empty when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.is_empty() {
            return Ok(());
        }
        let mut canonical_blockers = std::collections::HashSet::new();
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let raw_blocker = blocker.as_str();
            let blocker = blocker.trim();
            raw_blocker != blocker
                || blocker.is_empty()
                || !blocker.is_ascii()
                || blocker.bytes().any(|byte| byte.is_ascii_uppercase())
                || blocker.bytes().any(|byte| byte.is_ascii_whitespace())
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let canonical_blocker = blocker.trim();
            !canonical_blockers.insert(canonical_blocker)
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must not contain duplicate canonical entries after lowercase canonicalization when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .unwrap_or("none")
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_run_graph_dispatch_receipt_contract(
        receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_required_fields_present(&receipt)?;
        let receipt = normalize_legacy_downstream_preview_drift(receipt);
        let (receipt, _) = normalize_repairable_in_flight_receipt_lane_status_drift(receipt);
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;
    use std::fs;

    #[cfg(windows)]
    #[test]
    fn resume_packet_selector_matches_normal_extended_and_mixed_path_spellings() {
        let root = std::env::temp_dir().join(format!(
            "vida-resume-selector-path-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet directory");
        let packet = packet_dir.join("current.json");
        let other = packet_dir.join("other.json");
        fs::write(&packet, "{}").expect("write packet");
        fs::write(&other, "{}").expect("write other packet");
        let normal = packet.display().to_string();
        let extended = format!(r"\\?\{}", normal);
        let mixed = normal.replace('\\', "/");

        assert!(super::dispatch_packet_path_matches_receipt(
            Some(&normal),
            &extended
        ));
        assert!(super::dispatch_packet_path_matches_receipt(
            Some(&normal),
            &mixed
        ));
        assert!(!super::dispatch_packet_path_matches_receipt(
            Some(&normal),
            &other.display().to_string()
        ));

        let _ = fs::remove_dir_all(root);
    }
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if StateStore::task_status_is_closed_like(status) {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_run_graph_root(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    async fn close_store_and_remove_root(store: StateStore, root: std::path::PathBuf) {
        store.close().await;
        let _ = fs::remove_dir_all(root);
    }

    fn restore_vida_session_id(saved: Option<String>) {
        unsafe {
            match saved {
                Some(value) => std::env::set_var("VIDA_SESSION_ID", value),
                None => std::env::remove_var("VIDA_SESSION_ID"),
            }
        }
    }

    fn saved_runtime_session_env() -> Vec<(&'static str, Option<String>)> {
        [
            "VIDA_SESSION_ID",
            "VIDA_ORCHESTRATOR_SESSION_ID",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_REMOTE_SESSION_ID",
            "CODEX_SESSION_ID",
            "CODEX_THREAD_ID",
        ]
        .into_iter()
        .map(|name| (name, std::env::var(name).ok()))
        .collect()
    }

    fn clear_runtime_session_env() {
        unsafe {
            for name in [
                "VIDA_SESSION_ID",
                "VIDA_ORCHESTRATOR_SESSION_ID",
                "CLAUDE_CODE_SESSION_ID",
                "CLAUDE_CODE_REMOTE_SESSION_ID",
                "CODEX_SESSION_ID",
                "CODEX_THREAD_ID",
            ] {
                std::env::remove_var(name);
            }
        }
    }

    fn restore_runtime_session_env(saved: Vec<(&'static str, Option<String>)>) {
        unsafe {
            for (name, value) in saved {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    fn sample_run_graph_status() -> RunGraphStatus {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-vida-a",
            "implementation",
            "implementation",
        );
        status.task_id = "vida-a".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("coach".to_string());
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "awaiting_coach".to_string();
        status.context_state = "open".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        status
    }

    #[test]
    fn dispatch_identity_target_uses_configured_target_group() {
        let identity = RunGraphDispatchTaskIdentity {
            run_id: "run".to_string(),
            feature_epic_id: Some("epic".to_string()),
            spec_task_id: Some("spec".to_string()),
            work_pool_task_id: Some("pool".to_string()),
            dev_task_id: Some("dev".to_string()),
            source: "test".to_string(),
            updated_at: "now".to_string(),
        };

        assert_eq!(
            dispatch_identity_task_id_for_target(&identity, "developer"),
            Some("dev".to_string())
        );
        assert_eq!(
            dispatch_identity_task_id_for_target(&identity, "work-pool-pack"),
            Some("pool".to_string())
        );
    }

    #[test]
    fn terminal_blocked_dispatch_closes_delegated_cycle_gate() {
        let mut status = sample_run_graph_status();
        status.active_node = "coder".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coder_terminal_blocked".to_string();
        status.handoff_state = "blocked".to_string();
        status.resume_target = "none".to_string();

        let gate = RunGraphDelegationGateSummary::from_status(&status);

        assert!(!gate.delegated_cycle_open);
        assert_eq!(gate.delegated_cycle_state, "terminal_blocked");
        assert_eq!(gate.local_exception_takeover_gate, "delegated_cycle_clear");
        assert_eq!(gate.blocker_code, None);
    }

    #[test]
    fn non_terminal_blocked_dispatch_keeps_delegated_cycle_gate_open() {
        let mut status = sample_run_graph_status();
        status.active_node = "coder".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coder_blocked".to_string();
        status.handoff_state = "blocked".to_string();
        status.resume_target = "none".to_string();

        let gate = RunGraphDelegationGateSummary::from_status(&status);

        assert!(gate.delegated_cycle_open);
        assert_eq!(gate.delegated_cycle_state, "handoff_pending");
        assert_eq!(
            gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        assert_eq!(gate.blocker_code.as_deref(), Some("open_delegated_cycle"));
    }

    fn mark_terminal_closure_status(status: &mut RunGraphStatus) {
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
    }

    fn test_task_record(task_id: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-05-22T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-05-22T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn continuation_terminal_task_status_uses_canonical_closed_aliases() {
        for alias in ["closed", "completed", "done", "resolved", "merged"] {
            assert!(
                task_status_is_terminal_for_continuation(alias),
                "{alias} should be terminal for run-graph continuation"
            );
        }
        for non_terminal in ["open", "in_progress", "paused", "cancelled"] {
            assert!(
                !task_status_is_terminal_for_continuation(non_terminal),
                "{non_terminal} should not be terminal for run-graph continuation"
            );
        }
    }

    #[test]
    fn receiptless_operator_archive_sentinel_requires_exact_open_handoff_geometry() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-receiptless-archive",
            "implementation",
            "implementation",
        );
        status.active_node = "coder".to_string();
        status.next_node = Some("tester".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coder_active".to_string();
        status.handoff_state = "awaiting_tester".to_string();
        status.resume_target = "dispatch.tester_lane".to_string();
        status.recovery_ready = true;
        assert!(receiptless_closed_task_operator_archive_sentinel(&status));

        let mut generic = status.clone();
        generic.next_node = None;
        generic.handoff_state = "none".to_string();
        generic.resume_target = "none".to_string();
        assert!(!receiptless_closed_task_operator_archive_sentinel(&generic));

        let mut malformed_handoff = status.clone();
        malformed_handoff.handoff_state = "awaiting_coach".to_string();
        assert!(!receiptless_closed_task_operator_archive_sentinel(
            &malformed_handoff
        ));

        let mut malformed_resume = status.clone();
        malformed_resume.resume_target = "dispatch.coach_lane".to_string();
        assert!(!receiptless_closed_task_operator_archive_sentinel(
            &malformed_resume
        ));

        let mut recovery_closed = status.clone();
        recovery_closed.recovery_ready = false;
        assert!(!receiptless_closed_task_operator_archive_sentinel(
            &recovery_closed
        ));

        status.lifecycle_stage = "lane_exception_takeover".to_string();
        assert!(!receiptless_closed_task_operator_archive_sentinel(&status));
    }

    #[test]
    fn terminal_closure_status_requires_sealed_terminal_fields() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal",
            "implementation",
            "implementation",
        );
        mark_terminal_closure_status(&mut status);

        assert!(terminal_closure_status(&status));

        status.resume_target = "dispatch.verification".to_string();

        assert!(!terminal_closure_status(&status));
        status.resume_target = "none".to_string();

        status.next_node = Some("verification".to_string());

        assert!(!terminal_closure_status(&status));
    }

    fn write_release_admission_snapshot(state_root: &std::path::Path, run_id: &str) {
        let runtime_dir = state_root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        fs::write(
            runtime_dir.join("final-recorded-with-release-admission.json"),
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "source_run_id": run_id,
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": {
                        "status": "pass",
                        "admitted": true,
                        "closure_decision": "closed",
                        "decision_owner": "release-owner",
                        "decision_at": "2026-05-19T00:00:00Z",
                        "evidence_bundle_refs": ["evidence-bundle-case10"],
                        "open_risk_acceptance_ids": ["risk-acceptance-case10"],
                        "blockers": [],
                        "proof_surfaces": ["vida taskflow consume final"],
                        "evidence_table": [
                            {
                                "evidence_class": "closure_decision_record",
                                "status": "pass",
                                "evidence_refs": ["closure-record-case10"]
                            },
                            {
                                "evidence_class": "runtime_consumption_final_snapshot",
                                "status": "pass",
                                "evidence_refs": ["final-snapshot-case10"]
                            },
                            {
                                "evidence_class": "docflow_readiness_and_proof_receipts",
                                "status": "pass",
                                "evidence_refs": ["docflow-readiness-case10", "docflow-proof-case10"]
                            },
                            {
                                "evidence_class": "lane_execution_and_handoff_receipts",
                                "status": "pass",
                                "evidence_refs": ["lane-execution-case10", "handoff-case10"]
                            },
                            {
                                "evidence_class": "replay_checkpoint_lineage_artifacts",
                                "status": "pass",
                                "evidence_refs": ["checkpoint-case10", "replay-case10"]
                            },
                            {
                                "evidence_class": "risk_acceptance_artifacts",
                                "status": "pass",
                                "evidence_refs": ["risk-acceptance-case10"]
                            },
                            {
                                "evidence_class": "evidence_bundle_linkage",
                                "status": "pass",
                                "evidence_refs": ["evidence-bundle-case10"]
                            }
                        ]
                    }
                }
            })
            .to_string(),
        )
        .expect("release-admission snapshot should be writable");
    }

    #[tokio::test]
    async fn stale_after_release_admission_treats_closed_alias_task_as_closed() {
        let root = temp_run_graph_root("vida-release-admission-closed-alias");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("alias-task", "resolved"))
            .await
            .expect("seed alias task");
        write_release_admission_snapshot(store.root(), "run-alias");

        let mut status = sample_run_graph_status();
        status.run_id = "run-alias".to_string();
        status.task_id = "alias-task".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();

        assert!(
            store
                .run_graph_status_is_stale_after_release_admission_complete_from_task_rows(
                    &status,
                    &[],
                )
                .await
                .expect("live task lookup should succeed")
        );

        let rows = vec![test_task_record("alias-task", "merged")];
        assert!(
            store
                .run_graph_status_is_stale_after_release_admission_complete_from_task_rows(
                    &status, &rows,
                )
                .await
                .expect("task row lookup should succeed")
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn task_continuation_binding_stale_classifier_retires_reconciled_terminal_closure() {
        let root = temp_run_graph_root("vida-terminal-closure-stale-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.run_id = "terminal-closure-run".to_string();
        status.task_id = "closed-runtime-task".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        status.policy_gate = "historical_closed_task_stale_run_retired".to_string();

        assert!(
            store
                .run_graph_status_is_stale_for_task_continuation_binding(&status)
                .await
                .expect("terminal closure classifier should succeed")
        );

        let mut malicious_status = status.clone();
        malicious_status.run_id = "malicious-retired-run".to_string();
        malicious_status.task_id = "still-active-runtime-task".to_string();
        malicious_status.active_node = "implementation".to_string();
        malicious_status.handoff_state = "awaiting_review".to_string();
        malicious_status.context_state = "open".to_string();
        malicious_status.checkpoint_kind = "execution_cursor".to_string();
        malicious_status.recovery_ready = true;

        assert!(
            !store
                .run_graph_status_is_stale_for_task_continuation_binding(&malicious_status)
                .await
                .expect("contradictory retired closure classifier should fail closed")
        );

        let mut open_status = sample_run_graph_status();
        open_status.run_id = "active-run".to_string();
        open_status.task_id = "active-runtime-task".to_string();
        assert!(
            !store
                .run_graph_status_is_stale_for_task_continuation_binding(&open_status)
                .await
                .expect("active status classifier should succeed")
        );

        close_store_and_remove_root(store, root).await;
    }

    #[test]
    fn run_graph_status_fails_closed_when_governance_and_resumability_rows_are_missing() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = runtime
            .block_on(StateStore::open(harness.path().join(".vida/data/state")))
            .expect("state store should open");
        let status = sample_run_graph_status();
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should persist");
        let _: Option<GovernanceStateRow> = runtime
            .block_on(async {
                store
                    .db
                    .delete(("governance_state", status.run_id.as_str()))
                    .await
            })
            .expect("governance row should delete");
        let _: Option<ResumabilityCapsuleRow> = runtime
            .block_on(async {
                store
                    .db
                    .delete(("resumability_capsule", status.run_id.as_str()))
                    .await
            })
            .expect("resumability row should delete");

        let loaded = runtime
            .block_on(store.run_graph_status(&status.run_id))
            .expect("stale run graph status should remain inspectable");

        assert_eq!(loaded.run_id, status.run_id);
        assert_eq!(loaded.task_id, status.task_id);
        assert_eq!(loaded.policy_gate, "stale_missing_run_graph_governance");
        assert_eq!(loaded.handoff_state, "blocked_missing_run_graph_governance");
        assert_eq!(loaded.context_state, "stale_projection");
        assert_eq!(loaded.checkpoint_kind, "missing_resumability_capsule");
        assert_eq!(loaded.resume_target, "none");
        assert!(!loaded.recovery_ready);
    }

    #[test]
    fn routed_materialized_dispatch_receipt_reconciles_stale_status_to_recovery_ready() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-materialized-dispatch".to_string();
        status.task_id = "task-materialized-dispatch".to_string();
        status.active_node = "analyst".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analyst_blocked".to_string();
        status.policy_gate = "stale_missing_run_graph_execution".to_string();
        status.handoff_state = "blocked_missing_run_graph_execution".to_string();
        status.context_state = "stale_projection".to_string();
        status.checkpoint_kind = "missing_execution_plan_state".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = "analyst".to_string();
        receipt.dispatch_status = "routed".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.dispatch_packet_path = Some(
            "C:/project/vida-stack/.vida/data/state/runtime-consumption/dispatch-packets/run-materialized-dispatch.json"
                .to_string(),
        );
        receipt.dispatch_result_path = None;
        receipt.blocker_code = None;
        receipt.downstream_dispatch_target = Some("analyst".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_status = None;

        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt);
        let reconciled =
            reconcile_run_graph_status_with_dispatch_receipt(status, Some(&stored_receipt))
                .expect("materialized routed receipt should reconcile");

        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "analyst");
        assert_eq!(reconciled.next_node.as_deref(), Some("analyst"));
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "awaiting_analyst");
        assert_eq!(reconciled.resume_target, "dispatch.analyst_lane");
        assert!(reconciled.recovery_ready);
        crate::taskflow_run_graph::validate_run_graph_resume_gate(&reconciled)
            .expect("reconciled materialized dispatch should pass resume gate");
    }

    #[tokio::test]
    async fn run_graph_status_preserves_receipt_reconciled_materialized_dispatch_resume() {
        let root = temp_run_graph_root("vida-materialized-dispatch-recovery-ready");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-materialized-store", "in_progress"))
            .await
            .expect("seed task");
        let mut receipt = sample_dispatch_receipt("run-materialized-store");
        receipt.dispatch_target = "coach_implementation_gate".to_string();
        receipt.dispatch_status = "routed".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.dispatch_packet_path = Some(root.join("packet.json").display().to_string());
        receipt.dispatch_result_path = None;
        receipt.blocker_code = None;
        receipt.downstream_dispatch_target = Some("coach_implementation_gate".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_status = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("record receipt");

        let status = store
            .run_graph_status("run-materialized-store")
            .await
            .expect("status should reconcile from receipt");

        assert_eq!(status.status, "ready");
        assert_eq!(
            status.next_node.as_deref(),
            Some("coach_implementation_gate")
        );
        assert_eq!(status.handoff_state, "awaiting_coach_implementation_gate");
        assert_eq!(
            status.resume_target,
            "dispatch.coach_implementation_gate_lane"
        );
        assert_eq!(status.checkpoint_kind, "execution_cursor");
        assert!(status.recovery_ready);
        crate::taskflow_run_graph::validate_run_graph_resume_gate(&status)
            .expect("state-store reconciled status should pass resume gate");

        close_store_and_remove_root(store, root).await;
    }

    fn sample_explicit_binding(
        run_id: &str,
        task_id: &str,
        recorded_at: &str,
    ) -> RunGraphContinuationBinding {
        RunGraphContinuationBinding {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "run_id": run_id,
                "task_id": task_id,
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "test continuation".to_string(),
            primary_path: "crates/vida/src/state_store_run_graph_summary.rs".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: recorded_at.to_string(),
        }
    }

    fn sample_project_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should have a repository root parent")
            .to_path_buf()
    }

    fn sample_project_overlay() -> serde_yaml::Value {
        let project_root = sample_project_root();
        crate::config_value_utils::load_project_overlay_yaml_for_root(&project_root)
            .expect("repository project overlay should resolve for dispatch fixture")
    }

    fn configured_sample_dispatch_command(run_id: &str, runtime_role: &str) -> (String, String) {
        let project_root = sample_project_root();
        let overlay = sample_project_overlay();
        let commands_path = crate::yaml_string(crate::yaml_lookup(
            &overlay,
            &["agent_extensions", "registries", "commands"],
        ))
        .map(|path| crate::project_activator_surface::resolve_overlay_path(&project_root, &path))
        .expect("dispatch fixture should have a configured command registry path");
        let registry = crate::project_activator_surface::read_yaml_file_checked(&commands_path)
            .expect("dispatch fixture command registry should resolve");
        let role_token = format!("--role {runtime_role}");
        let command = crate::yaml_lookup(&registry, &["commands"])
            .and_then(serde_yaml::Value::as_sequence)
            .into_iter()
            .flatten()
            .find(|entry| {
                crate::yaml_string(crate::yaml_lookup(entry, &["args"]))
                    .is_some_and(|args| args.contains(&role_token))
            })
            .expect("dispatch fixture command registry should map the configured runtime role");
        let surface = crate::yaml_string(crate::yaml_lookup(command, &["surface"]))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .expect("configured dispatch command should have a surface");
        let args = crate::yaml_string(crate::yaml_lookup(command, &["args"]))
            .map(|value| {
                value
                    .replace("{{task_id}}", &format!("task-{run_id}"))
                    .trim()
                    .to_string()
            })
            .filter(|value| !value.is_empty())
            .expect("configured dispatch command should have args");
        (surface.clone(), format!("{surface} {args}"))
    }

    fn configured_sample_dispatch_identity_fields() -> (String, String, String, String) {
        let overlay = sample_project_overlay();
        let mut dispatch_targets = crate::yaml_lookup(
            &overlay,
            &["run_graph", "dispatch_task_identity", "target_groups"],
        )
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .flat_map(|group| crate::yaml_string_list(crate::yaml_lookup(group, &["aliases"])))
        .map(|target| target.trim().to_string())
        .filter(|target| !target.is_empty())
        .collect::<Vec<_>>();
        dispatch_targets.sort();
        dispatch_targets.dedup();
        let dispatch_target = dispatch_targets
            .into_iter()
            .next()
            .expect("dispatch fixture should have a configured target alias");

        let backend_id = crate::yaml_string(crate::yaml_lookup(
            &overlay,
            &["party_chat", "single_agent", "backend"],
        ))
        .map(|backend| backend.trim().to_string())
        .filter(|backend| !backend.is_empty())
        .expect("dispatch fixture should have a configured backend");
        let default_profile = crate::yaml_string(crate::yaml_lookup(
            &overlay,
            &[
                "agent_system",
                "subagents",
                backend_id.as_str(),
                "default_model_profile",
            ],
        ))
        .map(|profile| profile.trim().to_string())
        .filter(|profile| !profile.is_empty())
        .expect("dispatch fixture backend should have a default profile");
        let runtime_role = crate::yaml_string(crate::yaml_lookup(
            &overlay,
            &[
                "agent_system",
                "subagents",
                backend_id.as_str(),
                "default_runtime_role",
            ],
        ))
        .map(|role| role.trim().to_string())
        .filter(|role| !role.is_empty())
        .expect("dispatch fixture backend should have a default runtime role");
        let mut carrier_ids =
            crate::yaml_lookup(&overlay, &["host_environment", "codex", "agents"])
                .and_then(serde_yaml::Value::as_mapping)
                .into_iter()
                .flatten()
                .filter_map(|(key, entry)| {
                    let carrier_id = key.as_str()?.trim();
                    let configured_profile =
                        crate::yaml_string(crate::yaml_lookup(entry, &["default_model_profile"]))?;
                    (configured_profile.trim() == default_profile).then_some(carrier_id.to_string())
                })
                .filter(|carrier_id| !carrier_id.is_empty())
                .collect::<Vec<_>>();
        carrier_ids.sort();
        carrier_ids.dedup();
        assert_eq!(
            carrier_ids.len(),
            1,
            "dispatch fixture backend profile should resolve to one configured carrier"
        );
        (
            dispatch_target,
            backend_id,
            carrier_ids
                .into_iter()
                .next()
                .expect("carrier id should remain after uniqueness check"),
            runtime_role,
        )
    }

    fn sample_dispatch_receipt(run_id: &str) -> RunGraphDispatchReceipt {
        let (dispatch_target, backend_id, carrier_id, runtime_role) =
            configured_sample_dispatch_identity_fields();
        let (dispatch_surface, dispatch_command) =
            configured_sample_dispatch_command(run_id, &runtime_role);
        RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target,
            dispatch_status: "routed".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            // `dispatch_kind` is the persisted routing semantic; command surface/args are registry-derived above.
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some(dispatch_surface),
            dispatch_command: Some(dispatch_command),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
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
            activation_agent_type: Some(carrier_id),
            activation_runtime_role: Some(runtime_role),
            selected_backend: Some(backend_id),
            recorded_at: "2026-05-21T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn dispatch_receipt_for_packet_ignores_mismatching_primary_receipt_on_fresh_store() {
        let root = temp_run_graph_root("vida-dispatch-receipt-for-packet-fresh-store");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-dispatch-receipt-for-packet-fresh-store";
        let requested_packet_path = "/tmp/current-packet.json";

        let mut primary_receipt = sample_dispatch_receipt(run_id);
        primary_receipt.dispatch_packet_path = Some("/tmp/stale-packet.json".to_string());
        store
            .record_run_graph_dispatch_receipt(&primary_receipt)
            .await
            .expect("persist primary dispatch receipt");

        let mismatch = store
            .run_graph_dispatch_receipt_for_packet(run_id, requested_packet_path)
            .await
            .expect("mismatching packet lookup should succeed");
        assert!(mismatch.is_none());

        let mut lane_receipt = sample_dispatch_receipt(run_id);
        lane_receipt.dispatch_packet_path = Some(requested_packet_path.to_string());
        store
            .record_run_graph_dispatch_lane_receipt(&lane_receipt)
            .await
            .expect("persist matching lane receipt");

        let matching = store
            .run_graph_dispatch_receipt_for_packet(run_id, requested_packet_path)
            .await
            .expect("matching lane packet lookup should succeed")
            .expect("matching lane receipt should be returned");
        assert_eq!(
            matching.dispatch_packet_path.as_deref(),
            Some(requested_packet_path)
        );
        close_store_and_remove_root(store, root).await;
    }

    fn sample_host_bridge_receipt_identity(
        run_id: &str,
        packet_path: &str,
        receipt: &RunGraphDispatchReceipt,
    ) -> taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
        let overlay = sample_project_overlay();
        let (_, selected_cli_entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        let registry = serde_json::to_value(
            selected_cli_entry.expect("configured host CLI registry entry should resolve"),
        )
        .expect("configured host CLI registry should serialize");
        let adapter_operations =
            taskflow_host_bridge::HostBridgeAdapterOperations::from_registry_value(&registry)
                .expect("test adapter registry should resolve");
        let adapter_contract_snapshot = adapter_operations.to_value();
        let adapter_contract_hash = blake3::hash(
            &serde_json::to_vec(&adapter_contract_snapshot).expect("test adapter snapshot"),
        )
        .to_hex()
        .to_string();
        taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
            schema_version: taskflow_host_bridge::HOST_BRIDGE_RECEIPT_IDENTITY_SCHEMA_VERSION
                .to_string(),
            request_id: format!("request-{run_id}"),
            run_id: run_id.to_string(),
            task_id: format!("task-{run_id}"),
            attempt_id: format!("attempt-{run_id}"),
            packet_id: format!("packet-{run_id}"),
            dispatch_target: receipt.dispatch_target.clone(),
            packet_path: packet_path.to_string(),
            backend_id: receipt
                .selected_backend
                .clone()
                .expect("dispatch fixture should select a backend"),
            carrier_id: receipt
                .activation_agent_type
                .clone()
                .expect("dispatch fixture should select a carrier"),
            adapter_kind: adapter_operations.adapter_kind.clone(),
            adapter_capability_id: adapter_operations.adapter_capability_id.clone(),
            invocation_mode: adapter_operations.invocation_mode.clone(),
            dispatch_transport: adapter_operations.dispatch_transport.clone(),
            receipt_mode: adapter_operations.receipt_mode.clone(),
            adapter_contract_source: sample_project_root()
                .join("vida.config.yaml")
                .display()
                .to_string(),
            adapter_contract_snapshot,
            adapter_contract_hash,
            adapter_operations,
            request_path: format!("host-tool-bridge/requests/{run_id}.json"),
            result_path: format!("host-tool-bridge/results/{run_id}.json"),
            receipt_path: format!("host-tool-bridge/receipts/{run_id}.json"),
            recorded_at: "2026-07-18T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_is_idempotent_and_rejects_conflicts() {
        let root = temp_run_graph_root("vida-host-bridge-receipt-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-binding";
        let packet_path = "/tmp/host-bridge-binding.json";
        let mut receipt = sample_dispatch_receipt(run_id);
        receipt.dispatch_packet_path = Some(packet_path.to_string());
        let identity = sample_host_bridge_receipt_identity(run_id, packet_path, &receipt);

        store
            .record_host_bridge_receipt_binding(&identity, &receipt)
            .await
            .expect("first host bridge binding should persist");
        store
            .record_host_bridge_receipt_binding(&identity, &receipt)
            .await
            .expect("same host bridge binding should be idempotent");

        let mut progressed = receipt.clone();
        progressed.dispatch_result_path = Some(identity.result_path.clone());
        store
            .record_host_bridge_receipt_binding(&identity, &progressed)
            .await
            .expect("same identity may advance its compact receipt idempotently");

        let mut competing_identity = identity.clone();
        competing_identity.request_id = format!("{}-competing", identity.request_id);
        competing_identity.backend_id = format!("{}-competing", identity.backend_id);
        let mut conflicting = progressed.clone();
        conflicting.selected_backend = Some(competing_identity.backend_id.clone());
        let error = store
            .record_host_bridge_receipt_binding(&competing_identity, &conflicting)
            .await
            .expect_err("conflicting compact receipt must fail closed");
        assert!(
            error.to_string().contains(
                "host_bridge_receipt_binding_conflict:receipt_key=run-host-bridge-binding"
            ),
            "error={error:?}"
        );

        assert!(
            store
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await
                .expect("identity lookup should succeed")
                .is_some()
        );
        let stored = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("receipt lookup should succeed")
            .expect("receipt should remain persisted");
        assert_eq!(stored.dispatch_result_path, progressed.dispatch_result_path);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_accepts_matching_in_flight_receipt() {
        let root = temp_run_graph_root("vida-host-bridge-receipt-binding-in-flight");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-binding-in-flight";
        let packet_path = "/tmp/host-bridge-binding-in-flight.json";
        let mut in_flight = sample_dispatch_receipt(run_id);
        in_flight.dispatch_packet_path = Some(packet_path.to_string());
        in_flight.dispatch_status = "executing".to_string();
        in_flight.lane_status = "lane_running".to_string();
        store
            .record_run_graph_dispatch_receipt(&in_flight)
            .await
            .expect("in-flight dispatch receipt should persist");

        let identity = sample_host_bridge_receipt_identity(run_id, packet_path, &in_flight);
        let mut pending = in_flight.clone();
        pending.dispatch_status = "bridge_request_pending".to_string();
        pending.dispatch_result_path = Some(identity.result_path.clone());
        store
            .record_host_bridge_receipt_binding(&identity, &pending)
            .await
            .expect("matching in-flight receipt should advance to host bridge pending");

        assert!(
            store
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await
                .expect("identity lookup should succeed")
                .is_some()
        );
        let stored = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("receipt lookup should succeed")
            .expect("pending receipt should remain persisted");
        assert_eq!(stored.dispatch_status, "bridge_request_pending");
        assert_eq!(stored.dispatch_result_path, pending.dispatch_result_path);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_rejects_mismatched_in_flight_receipt() {
        let root = temp_run_graph_root("vida-host-bridge-receipt-binding-in-flight-mismatch");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-binding-in-flight-mismatch";
        let packet_path = "/tmp/host-bridge-binding-in-flight-mismatch.json";
        let mut in_flight = sample_dispatch_receipt(run_id);
        in_flight.dispatch_packet_path = Some(packet_path.to_string());
        in_flight.dispatch_status = "executing".to_string();
        in_flight.lane_status = "lane_running".to_string();
        store
            .record_run_graph_dispatch_receipt(&in_flight)
            .await
            .expect("in-flight dispatch receipt should persist");

        let identity = sample_host_bridge_receipt_identity(run_id, packet_path, &in_flight);
        let mut mismatched = in_flight.clone();
        mismatched.dispatch_status = "bridge_request_pending".to_string();
        mismatched.dispatch_command = Some("different-command".to_string());
        let error = store
            .record_host_bridge_receipt_binding(&identity, &mismatched)
            .await
            .expect_err("mismatched in-flight receipt must fail closed");
        assert!(
            error
                .to_string()
                .contains("host_bridge_receipt_binding_conflict:receipt_key="),
            "error={error:?}"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_atomic_rollback_leaves_no_partial_rows() {
        let root = temp_run_graph_root("vida-host-bridge-receipt-binding-rollback");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-binding-rollback";
        let packet_path = "/tmp/host-bridge-binding-rollback.json";
        let mut receipt = sample_dispatch_receipt(run_id);
        receipt.dispatch_packet_path = Some(packet_path.to_string());
        let identity = sample_host_bridge_receipt_identity(run_id, packet_path, &receipt);

        let _error = store
            .record_host_bridge_receipt_binding_with_forced_rollback(&identity, &receipt)
            .await
            .expect_err("forced in-transaction fault should roll back every binding row");
        assert!(
            store
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await
                .expect("identity lookup should succeed")
                .is_none()
        );
        assert!(
            store
                .run_graph_dispatch_receipt(run_id)
                .await
                .expect("receipt lookup should succeed")
                .is_none()
        );
        assert!(
            store
                .run_graph_owner_evidence_record(run_id, "dispatch_receipt")
                .await
                .expect("owner evidence lookup should succeed")
                .is_none()
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_concurrent_distinct_identities_have_one_winner() {
        for iteration in 0..10 {
            let root = temp_run_graph_root(&format!(
                "vida-host-bridge-receipt-binding-concurrent-{iteration}"
            ));
            let store = StateStore::open(root.clone()).await.expect("open store");
            let second_store = StateStore {
                db: store.db.clone(),
                root: store.root.clone(),
                _lifecycle_guard: std::sync::Arc::clone(&store._lifecycle_guard),
            };
            let run_id = format!("run-host-bridge-binding-concurrent-{iteration}");
            let packet_path = format!("/tmp/host-bridge-binding-concurrent-{iteration}.json");
            let mut receipt = sample_dispatch_receipt(&run_id);
            receipt.dispatch_packet_path = Some(packet_path.clone());
            let first_identity =
                sample_host_bridge_receipt_identity(&run_id, &packet_path, &receipt);
            let mut second_identity = first_identity.clone();
            second_identity.request_id = format!("{}-second", second_identity.request_id);

            let (first_result, second_result) = tokio::join!(
                store.record_host_bridge_receipt_binding(&first_identity, &receipt),
                second_store.record_host_bridge_receipt_binding(&second_identity, &receipt),
            );
            assert_eq!(
                usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
                1,
                "exactly one distinct identity may win: first={first_result:?} second={second_result:?}"
            );

            let identities = store
                .host_bridge_receipt_identities_for_run(&run_id)
                .await
                .expect("identity listing should succeed");
            assert_eq!(identities.len(), 1, "losing identity must roll back");
            let winner_request_id = if first_result.is_ok() {
                &first_identity.request_id
            } else {
                &second_identity.request_id
            };
            assert_eq!(&identities[0].request_id, winner_request_id);
            assert!(
                store
                    .run_graph_dispatch_receipt(&run_id)
                    .await
                    .expect("receipt lookup should succeed")
                    .is_some()
            );
            assert!(
                store
                    .run_graph_owner_evidence_record(&run_id, "dispatch_receipt")
                    .await
                    .expect("owner evidence lookup should succeed")
                    .is_some()
            );

            drop(second_store);
            close_store_and_remove_root(store, root).await;
        }
    }

    #[tokio::test]
    async fn host_bridge_receipt_binding_rejects_identity_mismatch_without_writes() {
        let root = temp_run_graph_root("vida-host-bridge-receipt-binding-mismatch");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let identity_run_id = "run-host-bridge-identity";
        let identity_packet_path = "/tmp/host-bridge-identity.json";
        let identity_receipt = sample_dispatch_receipt(identity_run_id);
        let identity = sample_host_bridge_receipt_identity(
            identity_run_id,
            identity_packet_path,
            &identity_receipt,
        );
        let receipt = sample_dispatch_receipt("run-host-bridge-other");
        let error = store
            .record_host_bridge_receipt_binding(&identity, &receipt)
            .await
            .expect_err("run mismatch must fail closed");
        assert!(
            error
                .to_string()
                .contains("host_bridge_receipt_binding_identity_mismatch:run_or_target")
        );
        assert!(
            store
                .host_bridge_receipt_identity(
                    &identity.run_id,
                    &identity.dispatch_target,
                    &identity.packet_path,
                    &identity.request_id,
                )
                .await
                .expect("identity lookup should succeed")
                .is_none()
        );

        close_store_and_remove_root(store, root).await;
    }

    #[test]
    fn downstream_ready_receipt_without_expected_next_node_cannot_select_handoff_target() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-untrusted-downstream-target".to_string();
        status.task_id = "task-untrusted-downstream-target".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;

        let mut receipt =
            RunGraphDispatchReceiptStored::from(sample_dispatch_receipt(&status.run_id));
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = Some(crate::LaneStatus::LaneCompleted.as_str().to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_blockers.clear();

        let reconciled = reconcile_run_graph_status_with_dispatch_receipt(status, Some(&receipt))
            .expect("receipt reconciliation should not fail");

        assert_eq!(reconciled.status, "in_progress");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "dispatch.implementer_lane");
    }

    #[test]
    fn downstream_handoff_evidence_uses_receipt_fields_without_packet_read() {
        let mut receipt = RunGraphDispatchReceiptStored::from(sample_dispatch_receipt(
            "run-ready-no-packet-read",
        ));
        receipt.dispatch_status = "executed".to_string();
        receipt.dispatch_packet_path = Some("/dev/zero".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_target = Some("verification".to_string());

        let evidence = run_graph_downstream_handoff_evidence(&receipt);

        assert!(downstream_handoff_ready_from_completion_evidence(&evidence));
        assert!(evidence.rework.is_none());
        assert!(evidence.source_lane.is_none());
    }

    #[test]
    fn blocked_source_lane_skips_packet_read_for_ineligible_receipt_status() {
        let mut receipt = RunGraphDispatchReceiptStored::from(sample_dispatch_receipt(
            "run-ineligible-no-packet-read",
        ));
        receipt.dispatch_status = "executed".to_string();
        receipt.dispatch_packet_path = Some("/dev/zero".to_string());

        assert!(blocked_source_lane_from_downstream_dispatch_packet(&receipt).is_none());
    }

    #[test]
    fn downstream_packet_evidence_rejects_non_regular_packet_path() {
        let mut receipt =
            RunGraphDispatchReceiptStored::from(sample_dispatch_receipt("run-device-rejected"));
        receipt.dispatch_status = "blocked".to_string();
        receipt.dispatch_packet_path = Some("/dev/zero".to_string());

        assert!(downstream_packet_evidence_from_receipt(&receipt).is_none());
    }

    #[test]
    fn host_bridge_completion_result_blocker_keeps_lawful_dispatch_retry() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-host-bridge-completion-retry".to_string();
        status.task_id = "task-host-bridge-completion-retry".to_string();
        status.active_node = "designer".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "designer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        let root = temp_run_graph_root("host-bridge-completion-retry");
        let result_path = root
            .join(".vida/data/state/runtime-consumption/dispatch-results/designer-blocked.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(
            &result_path,
            serde_json::to_string(&serde_json::json!({
                "status": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_code": "host_bridge_completion_result_blocked",
                "blocker_codes": ["host_bridge_completion_result_blocked"],
                "allowed_next_node": null,
                "completed_target": "designer"
            }))
            .expect("serialize result"),
        )
        .expect("write result");

        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = "designer".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("host_bridge_completion_result_blocked".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;

        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt);
        let projected =
            reconcile_run_graph_status_with_dispatch_receipt(status, Some(&stored_receipt))
                .expect("host bridge completion blocker should reconcile");

        assert_eq!(projected.status, "blocked");
        assert_eq!(projected.active_node, "designer");
        assert_eq!(projected.lifecycle_stage, "designer_blocked");
        assert_eq!(projected.next_node.as_deref(), Some("designer"));
        assert_eq!(projected.handoff_state, "awaiting_designer");
        assert_eq!(projected.resume_target, "dispatch.designer");
        assert!(projected.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn blocked_host_bridge_receipt_does_not_promote_from_stale_packet_rework_result() {
        let root = temp_run_graph_root("host-bridge-stale-packet-rework");
        let state_root = root.join(".vida/data/state");
        let packet_dir = state_root.join("runtime-consumption/dispatch-packets");
        let result_dir = state_root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let run_id = "run-host-bridge-stale-packet-rework";
        let packet_path = packet_dir.join("current.json");
        let stale_result_path = result_dir.join("stale-rework.json");
        let current_result_path = result_dir.join("current-blocked.json");
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "coder": {"dispatch_target": "coder", "task_class": "implementation"},
                        "tester": {"dispatch_target": "tester", "task_class": "verification"}
                    },
                    "execution_lane_sequence": ["coder", "tester"]
                }
            }
        });
        fs::write(
            &stale_result_path,
            serde_json::json!({
                "status": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "rework_target": "coder",
                "allowed_next_node": "coder",
                "execution_evidence": {"receipt_backed": true}
            })
            .to_string(),
        )
        .expect("write stale rework result");
        fs::write(
            &current_result_path,
            serde_json::json!({
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "host_tool_bridge_adapter_required"
            })
            .to_string(),
        )
        .expect("write current blocked result");
        fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "coder",
                "role_selection_full": {"execution_plan": execution_plan},
                "downstream_dispatch_result_path": stale_result_path
            })
            .to_string(),
        )
        .expect("write current packet");

        let mut status = sample_run_graph_status();
        status.run_id = run_id.to_string();
        status.task_id = run_id.to_string();
        status.active_node = "coder".to_string();
        status.next_node = Some("coder".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coder_dispatch_ready".to_string();
        status.handoff_state = "awaiting_coder".to_string();
        status.resume_target = "dispatch.coder".to_string();
        status.recovery_ready = true;

        let mut receipt = RunGraphDispatchReceiptStored::from(sample_dispatch_receipt(run_id));
        receipt.dispatch_target = "coder".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = Some("lane_blocked".to_string());
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path = Some(current_result_path.display().to_string());
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_blockers =
            vec!["host_tool_bridge_adapter_required".to_string()];

        let reconciled = reconcile_run_graph_status_with_dispatch_receipt(status, Some(&receipt))
            .expect("blocked host bridge receipt should reconcile");

        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "coder");
        assert_eq!(reconciled.lifecycle_stage, "coder_blocked");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn recovery_summary_exposes_host_bridge_completion_retry_target() {
        let root = temp_run_graph_root("host-bridge-completion-retry-recovery");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record(
                "task-host-bridge-completion-retry-recovery",
                "in_progress",
            ))
            .await
            .expect("seed task");
        let mut status = sample_run_graph_status();
        status.run_id = "run-host-bridge-completion-retry-recovery".to_string();
        status.task_id = "task-host-bridge-completion-retry-recovery".to_string();
        status.active_node = "designer".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "designer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked status");

        let result_path = root
            .join(".vida/data/state/runtime-consumption/dispatch-results/designer-blocked.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(&result_path, "{}").expect("write result placeholder");
        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = "designer".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("host_bridge_completion_result_blocked".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked receipt");

        let recovery = store
            .run_graph_recovery_summary(&status.run_id)
            .await
            .expect("load recovery summary");

        assert_eq!(recovery.resume_status, "blocked");
        assert_eq!(recovery.active_node, "designer");
        assert_eq!(recovery.lifecycle_stage, "designer_blocked");
        assert_eq!(recovery.resume_node.as_deref(), Some("designer"));
        assert_eq!(recovery.resume_target, "dispatch.designer");
        assert!(recovery.recovery_ready);
        assert!(recovery.delegation_gate.delegated_cycle_open);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn host_bridge_completion_retry_preserves_blocked_packet_source_lane() {
        let root = temp_run_graph_root("host-bridge-retry-blocked-packet-source");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record(
                "task-host-bridge-retry-blocked-packet-source",
                "in_progress",
            ))
            .await
            .expect("seed task");
        let mut status = sample_run_graph_status();
        status.run_id = "run-host-bridge-retry-blocked-packet-source".to_string();
        status.task_id = "task-host-bridge-retry-blocked-packet-source".to_string();
        status.active_node = "designer".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "designer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked status");

        let packet_path = root.join(
            ".vida/data/state/runtime-consumption/downstream-dispatch-packets/stale-designer-source.json",
        );
        fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        fs::write(
            &packet_path,
            serde_json::to_string(&serde_json::json!({
                "source_dispatch_target": "designer",
                "source_dispatch_status": "blocked",
                "source_blocker_code": "host_bridge_completion_result_blocked",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["host_bridge_completion_result_blocked"]
            }))
            .expect("serialize packet"),
        )
        .expect("write packet");

        let result_path = root
            .join(".vida/data/state/runtime-consumption/dispatch-results/autotester-blocked.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(&result_path, "{}").expect("write result placeholder");
        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = "autotester".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "autotester_lane".to_string();
        receipt.blocker_code = Some("host_bridge_completion_result_blocked".to_string());
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_blockers =
            vec!["host_bridge_completion_result_blocked".to_string()];
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked receipt");

        let recovery = store
            .run_graph_recovery_summary(&status.run_id)
            .await
            .expect("load recovery summary");

        assert_eq!(recovery.resume_status, "blocked");
        assert_eq!(recovery.active_node, "designer");
        assert_eq!(recovery.lifecycle_stage, "designer_blocked");
        assert_eq!(recovery.resume_node, None);
        assert_eq!(recovery.resume_target, "dispatch.designer");
        assert!(!recovery.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[test]
    fn blocked_downstream_completion_result_routes_to_developer_rework() {
        let scenario = crate::team_flow_authority_adapter::test_support::canonical_scenario_spec(
            "coach-blocked-developer-rework-routing",
        );
        let bundle = scenario.compiled_bundle;
        let authority =
            crate::team_flow_authority_adapter::compile_team_flow_authority(&bundle, None, None)
                .expect("canonical fixture authority should compile");
        let (source_node_id, target_node_id) = authority
            .nodes
            .iter()
            .find_map(|source| {
                source.node.rework_targets.iter().find_map(|target_id| {
                    authority
                        .node(target_id)
                        .filter(|target| target.node.task_class == "implementation")
                        .map(|_| (source.node.node_id.clone(), target_id.clone()))
                })
            })
            .expect("canonical authority should expose an implementation rework edge");
        let source = crate::team_flow_authority_adapter::resolve_team_flow_node(
            &authority,
            None,
            &source_node_id,
        )
        .expect("canonical rework source should resolve");
        let target = crate::team_flow_authority_adapter::resolve_team_flow_node(
            &authority,
            None,
            &target_node_id,
        )
        .expect("canonical implementation rework target should resolve");
        let execution_plan = serde_json::json!({
            "team_flow_authority_selected_node_id": source.node_id.clone(),
            "development_flow": {
                "dispatch_contract": {
                    "selected_flow_set": authority.snapshot.flow_ref,
                    "team_flow_authority_id": authority.authority_id,
                    "team_flow_config_hash": authority.config_authority_hash,
                    "team_flow_registry_hash": authority.registry_authority_hash,
                    "selected_node_id": source.node_id.clone(),
                    "team_flow_authority_selected_node_id": source.node_id.clone(),
                    "lane_catalog": scenario
                        .lane_catalog_override
                        .expect("canonical lane catalog"),
                    "execution_lane_sequence": scenario
                        .lane_sequence_override
                        .expect("canonical lane sequence")
                }
            }
        });
        let mut status = sample_run_graph_status();
        status.run_id = "run-coach-rework-route".to_string();
        status.task_id = "coach-blocked-developer-rework-routing".to_string();
        status.active_node = source.node_id.clone();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = format!("{}_blocked", source.node_id);
        status.recovery_ready = false;

        let root = temp_run_graph_root("coach-rework-route");
        let result_path =
            root.join(".vida/data/state/runtime-consumption/dispatch-results/coach-rework.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(
            &result_path,
            serde_json::to_string(&serde_json::json!({
                "status": "blocked",
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_code": "coach_rework_required",
                "blocker_codes": ["coach_rework_required"],
                "rework_target": target.node_id.clone(),
                "allowed_next_node": target.node_id.clone(),
                "completion_verdict": "rework_required",
                "execution_evidence": {"receipt_backed": true},
                "summary": "coach decision=blocked; Meeting scheduledAt missing for non-all-day meeting",
                "blocker_details": [{
                    "code": "coach_rework_required",
                    "message": "coach decision=blocked; Meeting scheduledAt missing for non-all-day meeting",
                    "completed_target": source.node_id.clone()
                }]
            }))
            .expect("serialize result"),
        )
        .expect("write result");
        let packet_path = root.join(
            ".vida/data/state/runtime-consumption/downstream-dispatch-packets/coach-rework.json",
        );
        fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        fs::write(
            &packet_path,
            serde_json::to_string(&serde_json::json!({
                "source_dispatch_target": source.node_id.clone(),
                "source_dispatch_status": "blocked",
                "source_blocker_code": "coach_rework_required",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["coach_rework_required"],
                "downstream_dispatch_target": source.node_id.clone(),
                "downstream_dispatch_result_path": result_path.display().to_string(),
                "role_selection_full": {
                    "compiled_bundle": bundle,
                    "execution_plan": execution_plan
                }
            }))
            .expect("serialize packet"),
        )
        .expect("write packet");

        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = source.node_id.clone();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("coach_rework_required".to_string());
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());

        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt);
        let authorized_rework_route =
            crate::runtime_dispatch_result_evidence::
                authorized_dispatch_rework_route_from_receipt_fields(
                    stored_receipt.downstream_dispatch_result_path.as_deref(),
                    stored_receipt.dispatch_result_path.as_deref(),
                    stored_receipt.dispatch_packet_path.as_deref(),
                    &stored_receipt.dispatch_target,
                )
                .expect("canonical receipt-backed rework route should authorize");
        let projected = reconcile_run_graph_status_with_dispatch_receipt_and_rework_route(
            status,
            Some(&stored_receipt),
            Some(&authorized_rework_route),
        )
        .expect("rework route should reconcile");

        assert_eq!(projected.status, "ready");
        assert!(projected.recovery_ready);
        assert_eq!(projected.active_node, source.node_id);
        assert_eq!(
            projected.next_node.as_deref(),
            Some(target.node_id.as_str())
        );
        assert_ne!(projected.next_node.as_deref(), Some("verification"));
        assert_eq!(projected.policy_gate, "coach_rework_required");
        assert_eq!(
            projected.handoff_state,
            format!("awaiting_{}", target.node_id)
        );
        assert_eq!(
            projected.resume_target,
            format!("dispatch.{}", target.node_id)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pass_downstream_completion_result_ignores_stale_nested_rework_evidence() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-analyst-pass-route".to_string();
        status.task_id = "analyst-pass-designer-routing".to_string();
        status.active_node = "analyst".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analyst_blocked".to_string();
        status.recovery_ready = false;

        let root = temp_run_graph_root("analyst-pass-stale-rework");
        let result_path =
            root.join(".vida/data/state/runtime-consumption/dispatch-results/analyst-pass.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(
            &result_path,
            serde_json::to_string(&serde_json::json!({
                "status": "pass",
                "execution_state": "executed",
                "decision": "approve",
                "verdict": "pass",
                "blocker_codes": [],
                "allowed_next_node": "designer",
                "completion_verdict": "pass",
                "execution_evidence": {
                    "decision": "rework_required",
                    "verdict": "rework_required",
                    "completion_verdict": "rework_required"
                }
            }))
            .expect("serialize result"),
        )
        .expect("write result");
        let packet_path = root.join(
            ".vida/data/state/runtime-consumption/downstream-dispatch-packets/analyst-pass.json",
        );
        fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        fs::write(
            &packet_path,
            serde_json::to_string(&serde_json::json!({
                "source_dispatch_target": "analyst",
                "source_dispatch_status": "executed",
                "downstream_dispatch_result_path": result_path.display().to_string(),
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "lane_catalog": {
                                    "analyst": {
                                        "dispatch_target": "analyst",
                                        "task_class": "analysis"
                                    },
                                    "designer": {
                                        "dispatch_target": "designer",
                                        "task_class": "design"
                                    },
                                    "developer": {
                                        "dispatch_target": "developer",
                                        "task_class": "implementation"
                                    }
                                },
                                "execution_lane_sequence": ["analyst", "designer", "developer"]
                            }
                        }
                    }
                }
            }))
            .expect("serialize packet"),
        )
        .expect("write packet");

        let mut receipt = sample_dispatch_receipt(&status.run_id);
        receipt.dispatch_target = "analyst".to_string();
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.blocker_code = None;
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());

        let stored_receipt = RunGraphDispatchReceiptStored::from(receipt);
        let projected =
            reconcile_run_graph_status_with_dispatch_receipt(status, Some(&stored_receipt))
                .expect("pass route should reconcile without rework");

        assert_ne!(projected.next_node.as_deref(), Some("developer_rework"));
        assert_ne!(projected.policy_gate, "host_bridge_completion_blocked");
        assert_eq!(projected.active_node, "analyst");

        let _ = fs::remove_dir_all(&root);
    }

    fn reconciled_pack_dispatch_receipt_for_path(packet_path: String) -> RunGraphDispatchReceipt {
        let mut receipt = sample_dispatch_receipt("run-reconciled-pack-context");
        receipt.dispatch_packet_path = Some(packet_path);
        receipt
    }

    fn write_reconciled_pack_packet(
        path: &std::path::Path,
        marker: &str,
        padding_bytes: Option<usize>,
    ) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create packet parent");
        }
        let mut packet = serde_json::json!({
            "role_selection_full": {
                "ok": true,
                "activation_source": "test",
                "selection_mode": "runtime",
                "fallback_role": "orchestrator",
                "request": marker,
                "selected_role": "pm",
                "conversational_mode": null,
                "single_task_only": true,
                "tracked_flow_entry": "work-pool-pack",
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": ["work-pool-pack"],
                "compiled_bundle": null,
                "execution_plan": {
                    "development_flow": {
                        "dispatch_contract": {}
                    },
                    "orchestration_contract": {}
                },
                "reason": "test"
            },
            "run_graph_bootstrap": {
                "marker": marker
            }
        });
        if let Some(padding_bytes) = padding_bytes {
            packet
                .as_object_mut()
                .expect("packet should be an object")
                .insert(
                    "padding".to_string(),
                    serde_json::Value::String("x".repeat(padding_bytes)),
                );
        }
        fs::write(
            path,
            serde_json::to_string_pretty(&packet).expect("packet json should encode"),
        )
        .expect("write reconciled pack packet");
    }

    #[test]
    fn terminal_closure_supersedes_stale_pending_developer_handoff_receipt() {
        let mut status = sample_run_graph_status();
        mark_terminal_closure_status(&mut status);

        let mut receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
                )
                .to_string(),
            ),
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
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-17T00:00:00Z".to_string(),
        };

        assert!(terminal_closure_supersedes_stale_handoff_receipt(
            &status,
            &mut receipt
        ));
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert_eq!(receipt.blocker_code, None);
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("executed")
        );
    }

    #[test]
    fn terminal_closure_supersedes_stale_host_bridge_pending_receipt() {
        let mut status = sample_run_graph_status();
        mark_terminal_closure_status(&mut status);

        let mut receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analyst".to_string(),
            dispatch_status: "bridge_request_pending".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::HostToolBridgeAdapterRequired,
                )
                .to_string(),
            ),
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
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-25T00:00:00Z".to_string(),
        };

        assert!(terminal_closure_supersedes_stale_handoff_receipt(
            &status,
            &mut receipt
        ));
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert_eq!(receipt.blocker_code, None);
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("closure")
        );
        assert_eq!(
            receipt.downstream_dispatch_note.as_deref(),
            Some("terminal closure superseded stale host bridge blocker")
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("executed")
        );
    }

    #[tokio::test]
    async fn terminal_closure_recovery_ignores_stale_host_bridge_pending_receipt() {
        let root = temp_run_graph_root("vida-terminal-closure-stale-hostbridge");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "feature-terminal-hostbridge",
                title: "Closed terminal closure host bridge task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-hostbridge",
            "delivery",
            "delivery",
        );
        status.task_id = "feature-terminal-hostbridge".to_string();
        mark_terminal_closure_status(&mut status);
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist terminal closure run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-terminal-hostbridge".to_string(),
                dispatch_target: "analyst".to_string(),
                dispatch_status: "bridge_request_pending".to_string(),
                lane_status: "lane_open".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analyst-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analyst-result.json".to_string()),
                blocker_code: Some(
                    crate::release1_contracts::blocker_code_str(
                        crate::release1_contracts::BlockerCode::HostToolBridgeAdapterRequired,
                    )
                    .to_string(),
                ),
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
                downstream_dispatch_active_target: Some("analyst".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-25T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale host bridge receipt");

        let reconciled = store
            .run_graph_status("run-terminal-hostbridge")
            .await
            .expect("load reconciled terminal closure status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.policy_gate, "closed_task_stale_run_retired");
        assert_eq!(reconciled.resume_target, "none");
        assert!(reconciled.is_reconciled_terminal_closure());
        assert!(!reconciled.delegation_gate().delegated_cycle_open);

        let recovery = store
            .run_graph_recovery_summary("run-terminal-hostbridge")
            .await
            .expect("load recovery summary");
        assert_eq!(recovery.resume_status, "completed");
        assert_eq!(recovery.active_node, "closure");
        assert_eq!(recovery.lifecycle_stage, "closure_complete");
        assert_eq!(recovery.delegation_gate.blocker_code, None);

        let receipt = store
            .run_graph_dispatch_receipt_summary_for_status(&reconciled)
            .await
            .expect("load receipt summary")
            .expect("receipt summary should exist");
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert_eq!(receipt.blocker_code, None);
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("closure")
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_absolute_packet_outside_state_root() {
        let root = temp_run_graph_root("vida-reconciled-pack-external-packet");
        let external_root = temp_run_graph_root("vida-reconciled-pack-attacker-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let external_packet = external_root.join("outside-packet.json");
        write_reconciled_pack_packet(&external_packet, "outside-state-root", None);

        let receipt =
            reconciled_pack_dispatch_receipt_for_path(external_packet.display().to_string());
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("out-of-root packet should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("escapes VIDA state root"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
        let _ = fs::remove_dir_all(external_root);
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_relative_packet_traversal() {
        let root = temp_run_graph_root("vida-reconciled-pack-traversal");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let outside_packet = root.parent().unwrap().join(format!(
            "vida-reconciled-pack-outside-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        write_reconciled_pack_packet(&outside_packet, "traversal-outside-root", None);

        let receipt = reconciled_pack_dispatch_receipt_for_path(format!(
            "../{}",
            outside_packet.file_name().unwrap().to_string_lossy()
        ));
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("dot-segment traversal should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("dot-segment traversal"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        let _ = fs::remove_file(outside_packet);
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_existing_directory_packet_path() {
        let root = temp_run_graph_root("vida-reconciled-pack-directory-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet_dir = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("packet-dir");
        fs::create_dir_all(&packet_dir).expect("create packet directory");

        let receipt = reconciled_pack_dispatch_receipt_for_path(packet_dir.display().to_string());
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("directory packet path should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("not a regular file"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_symlink_packet_path() {
        use std::os::unix::fs::symlink;

        let root = temp_run_graph_root("vida-reconciled-pack-symlink");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("packet.json");
        write_reconciled_pack_packet(&packet, "symlink-target", None);
        let packet_link = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("packet-link.json");
        if let Some(parent) = packet_link.parent() {
            fs::create_dir_all(parent).expect("create symlink parent");
        }
        symlink(&packet, &packet_link).expect("create packet symlink");

        let receipt = reconciled_pack_dispatch_receipt_for_path(packet_link.display().to_string());
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("symlink packet path should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("symlink"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_windows_symlink_packet_path() {
        use std::os::windows::fs::symlink_file;

        let root = temp_run_graph_root("vida-reconciled-pack-windows-symlink");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("packet.json");
        write_reconciled_pack_packet(&packet, "windows-symlink-target", None);
        let packet_link = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("packet-link.json");
        if let Some(parent) = packet_link.parent() {
            fs::create_dir_all(parent).expect("create symlink parent");
        }
        match symlink_file(&packet, &packet_link) {
            Ok(()) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                eprintln!("skipping Windows symlink packet path test: {error}");
                close_store_and_remove_root(store, root).await;
                return;
            }
            Err(error) => panic!("create packet symlink: {error}"),
        }

        let receipt = reconciled_pack_dispatch_receipt_for_path(packet_link.display().to_string());
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("symlink packet path should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("symlink"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_allows_absolute_packet_inside_state_root() {
        let root = temp_run_graph_root("vida-reconciled-pack-state-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("inside-packet.json");
        write_reconciled_pack_packet(&packet, "inside-state-root", None);

        let receipt = reconciled_pack_dispatch_receipt_for_path(packet.display().to_string());
        let (_role_selection, run_graph_bootstrap) = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect("in-root packet should decode")
            .expect("in-root packet should be accepted");

        assert_eq!(run_graph_bootstrap["marker"], "inside-state-root");
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_allows_relative_packet_inside_state_root() {
        let root = temp_run_graph_root("vida-reconciled-pack-relative-packet");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("relative-packet.json");
        write_reconciled_pack_packet(&packet, "relative-state-root", None);

        let relative_packet_path = packet
            .strip_prefix(&root)
            .expect("packet should live under state root")
            .display()
            .to_string();
        let receipt = reconciled_pack_dispatch_receipt_for_path(relative_packet_path);
        let (_role_selection, run_graph_bootstrap) = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect("relative in-root packet should decode")
            .expect("relative in-root packet should be accepted");

        assert_eq!(run_graph_bootstrap["marker"], "relative-state-root");
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconciled_pack_dispatch_context_rejects_oversized_packet() {
        let root = temp_run_graph_root("vida-reconciled-pack-oversized");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let packet = root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("oversized-packet.json");
        write_reconciled_pack_packet(
            &packet,
            "oversized-packet",
            Some(MAX_RECONCILED_PACK_DISPATCH_PACKET_BYTES as usize + 1),
        );

        let receipt = reconciled_pack_dispatch_receipt_for_path(packet.display().to_string());
        let error = store
            .reconciled_pack_dispatch_context(&receipt)
            .await
            .expect_err("oversized packet should be rejected");

        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("4 MiB intake cap"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
    }

    fn sample_replay_lineage_receipt(
        receipt_id: &str,
        run_id: &str,
        recorded_at: &str,
    ) -> RunGraphReplayLineageReceipt {
        RunGraphReplayLineageReceipt {
            receipt_id: receipt_id.to_string(),
            run_id: run_id.to_string(),
            lineage_kind: "dispatch_packet_lineage".to_string(),
            replay_scope: "completed_resume".to_string(),
            origin_checkpoint_ref: format!("checkpoint:{run_id}"),
            fork_parent: None,
            source_dispatch_target: "coach".to_string(),
            source_dispatch_packet_path: Some(format!("/tmp/{run_id}-coach.json")),
            source_dispatch_result_path: Some(format!("/tmp/{run_id}-coach-result.json")),
            resolved_dispatch_target: "closure".to_string(),
            resolved_task_id: format!("task-{run_id}"),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            validation_outcome: "pass".to_string(),
            recorded_at: recorded_at.to_string(),
        }
    }

    #[test]
    fn dispatch_summary_preserves_executed_explicit_lane_completed_receipt() {
        let summary = RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
            run_id: "run-lane-complete".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida lane complete".to_string()),
            dispatch_command: Some("vida lane complete run-lane-complete".to_string()),
            dispatch_packet_path: Some("runtime-consumption/dispatch-packets/run.json".to_string()),
            dispatch_result_path: Some("runtime-consumption/dispatch-results/run.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("activate writer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(
                "runtime-consumption/dispatch-results/run.json".to_string(),
            ),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-15T08:00:00Z".to_string(),
        });

        assert_eq!(summary.lane_status, "lane_completed");
        assert_eq!(
            summary.downstream_dispatch_blockers,
            ["missing_owned_write_scope"]
        );
    }

    #[test]
    fn executed_lane_completed_dispatch_receipt_signal_is_not_ambiguous() {
        let summary = RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
            run_id: "run-lane-complete-signal".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida lane complete".to_string()),
            dispatch_command: Some("vida lane complete run-lane-complete-signal".to_string()),
            dispatch_packet_path: Some("runtime-consumption/dispatch-packets/run.json".to_string()),
            dispatch_result_path: Some("runtime-consumption/dispatch-results/run.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("activate writer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(
                "runtime-consumption/dispatch-results/run.json".to_string(),
            ),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-15T08:00:00Z".to_string(),
        });

        assert!(!latest_run_graph_dispatch_receipt_signal_is_ambiguous(
            &summary
        ));
    }

    #[test]
    fn bridge_request_pending_lane_open_dispatch_receipt_signal_is_not_ambiguous() {
        let summary = RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
            run_id: "run-bridge-request-pending-signal".to_string(),
            dispatch_target: "analyst".to_string(),
            dispatch_status: "bridge_request_pending".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --execute-dispatch --json".to_string()),
            dispatch_packet_path: Some("runtime-consumption/dispatch-packets/run.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "bridge request is pending host execution evidence".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analyst".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-15T08:00:00Z".to_string(),
        });

        assert!(!latest_run_graph_dispatch_receipt_signal_is_ambiguous(
            &summary
        ));
    }

    #[test]
    fn routed_specification_receipt_with_pending_downstream_evidence_stays_dispatch_ready() {
        let mut status = sample_run_graph_status();
        status.run_id = "run-spec-routed".to_string();
        status.task_id = "task-spec-routed".to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("pm".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "dispatch_ready".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_pm".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "dispatch.pm_lane".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --dispatch-packet packet --execute-dispatch --json".to_string(),
            ),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "pending_specification_evidence".to_string(),
                "pending_spec_task_close".to_string(),
                "pending_design_finalize".to_string(),
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("pm".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-02T06:52:54Z".to_string(),
        };

        let stored_receipt = receipt.into();
        let reconciled =
            reconcile_run_graph_status_with_dispatch_receipt(status, Some(&stored_receipt))
                .expect("routed pre-execution receipt should reconcile");

        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.lifecycle_stage, "dispatch_ready");
        assert_eq!(reconciled.resume_target, "dispatch.pm_lane");
        assert!(reconciled.recovery_ready);
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_and_dispatch_context_round_trip() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-binding-roundtrip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let binding = RunGraphContinuationBinding {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "run-1",
                "active_node": "pm",
            }),
            binding_source: "test".to_string(),
            why_this_unit: "because".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("req".to_string()),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };
        let context = RunGraphDispatchContext {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            request_text: "req".to_string(),
            role_selection: serde_json::json!({
                "ok": true,
                "activation_source": "test",
                "selection_mode": "fixed",
                "fallback_role": "worker",
                "request": "req",
                "selected_role": "worker",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {},
                "reason": "test"
            }),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };

        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record binding");
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("record context");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-1".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "implementation".to_string(),
                dispatch_surface: Some("test".to_string()),
                dispatch_command: Some("test command".to_string()),
                dispatch_packet_path: Some("/tmp/run-1.json".to_string()),
                dispatch_result_path: Some("/tmp/run-1-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida taskflow consume continue".to_string()),
                downstream_dispatch_note: Some("test note".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/run-1-coach.json".to_string()),
                downstream_dispatch_status: Some("ready".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T10:00:01Z".to_string(),
            })
            .await
            .expect("record receipt");

        let stored_binding = store
            .run_graph_continuation_binding("run-1")
            .await
            .expect("read binding")
            .expect("binding present");
        let stored_context = store
            .run_graph_dispatch_context("run-1")
            .await
            .expect("read context")
            .expect("context present");

        assert_eq!(stored_binding.binding_source, "test");
        assert_eq!(stored_binding.active_bounded_unit["active_node"], "pm");
        assert_eq!(stored_context.request_text, "req");
        assert_eq!(
            stored_context
                .role_selection()
                .expect("role selection should decode")
                .selected_role,
            "worker"
        );
        for artifact_kind in [
            "continuation_binding",
            "dispatch_context",
            "dispatch_receipt",
        ] {
            let owner_record = store
                .run_graph_owner_evidence_record("run-1", artifact_kind)
                .await
                .expect("read owner evidence")
                .unwrap_or_else(|| panic!("missing owner evidence for {artifact_kind}"));
            assert_eq!(owner_record.run_id, "run-1");
            assert_eq!(owner_record.artifact_kind, artifact_kind);
            assert_eq!(
                owner_record.runtime_owner_evidence["mutation_gate"],
                "current_session_allowed"
            );
            assert!(
                owner_record.runtime_owner_evidence["current_session"]["session_id"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty())
            );
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_status_read_does_not_record_owner_evidence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-read-owner-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .record_run_graph_owner_evidence("seed-owner-evidence", "dispatch_context")
            .await
            .expect("seed owner evidence table");

        let mut status = sample_run_graph_status();
        status.run_id = "run-read-only-owner-evidence".to_string();
        status.task_id = "task-read-only-owner-evidence".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let loaded = store
            .run_graph_status("run-read-only-owner-evidence")
            .await
            .expect("read run graph status");
        assert_eq!(loaded.run_id, "run-read-only-owner-evidence");
        assert!(
            store
                .run_graph_owner_evidence_record("run-read-only-owner-evidence", "run_graph_status")
                .await
                .expect("read owner evidence")
                .is_none()
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_legacy_ownerless_tracks_owner_evidence_and_claims() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-legacy-ownerless-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut ownerless = sample_run_graph_status();
        ownerless.run_id = "legacy-ownerless-run".to_string();
        ownerless.task_id = "legacy-ownerless-task".to_string();
        store
            .record_run_graph_status(&ownerless)
            .await
            .expect("persist ownerless run graph status");
        assert!(
            store
                .run_graph_legacy_ownerless("legacy-ownerless-run")
                .await
                .expect("classify ownerless run")
        );

        store
            .record_run_graph_owner_evidence("legacy-ownerless-run", "dispatch_context")
            .await
            .expect("record owner evidence");
        assert!(
            !store
                .run_graph_legacy_ownerless("legacy-ownerless-run")
                .await
                .expect("owner evidence should make run non-ownerless")
        );

        let mut claimed = sample_run_graph_status();
        claimed.run_id = "legacy-claimed-run".to_string();
        claimed.task_id = "legacy-claimed-task".to_string();
        store
            .record_run_graph_status(&claimed)
            .await
            .expect("persist claim-backed run graph status");
        assert!(
            store
                .run_graph_legacy_ownerless("legacy-claimed-run")
                .await
                .expect("classify pre-claim run")
        );
        let claim = store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "legacy-claimed-run-write".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-a".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("legacy-claimed-task".to_string()),
                run_id: Some("legacy-claimed-run".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("legacy-claimed-domain".to_string()),
                owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire claim");
        assert!(
            !store
                .run_graph_legacy_ownerless("legacy-claimed-run")
                .await
                .expect("claim should make run non-ownerless")
        );
        store
            .release_orchestrator_claim(&claim.claim_id, claim.resource_revision, "test release")
            .await
            .expect("release claim");
        assert!(
            store
                .run_graph_legacy_ownerless("legacy-claimed-run")
                .await
                .expect("released claim should not block ownerless classification")
        );

        let mut expired = sample_run_graph_status();
        expired.run_id = "legacy-expired-claim-run".to_string();
        expired.task_id = "legacy-expired-claim-task".to_string();
        store
            .record_run_graph_status(&expired)
            .await
            .expect("persist expired-claim run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "legacy-expired-claim-run-write".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-a".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("legacy-expired-claim-task".to_string()),
                run_id: Some("legacy-expired-claim-run".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("legacy-expired-domain".to_string()),
                owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: -1,
            })
            .await
            .expect("acquire expiring claim");
        assert_eq!(
            store
                .expire_stale_orchestrator_claims()
                .await
                .expect("expire stale claims"),
            1
        );
        assert!(
            store
                .run_graph_legacy_ownerless("legacy-expired-claim-run")
                .await
                .expect("expired claim should not block ownerless classification")
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_mutation_blocks_foreign_session_without_claim() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-claim-block");
        let owner_store = StateStore::open(root.clone())
            .await
            .expect("open owner store");
        owner_store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "owner-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-owner".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-owner".to_string()),
                run_id: Some("run-owned".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("owner-domain".to_string()),
                owned_paths: vec!["owner/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire owner claim");
        owner_store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-owned"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-foreign");
        }
        let binding_result = owner_store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owned",
                "task-owner",
                "2026-05-21T01:00:00Z",
            ))
            .await;
        assert!(binding_result.is_err());

        let mut foreign_status = sample_run_graph_status();
        foreign_status.run_id = "run-owned".to_string();
        foreign_status.task_id = "task-owner".to_string();
        let status_result = owner_store.record_run_graph_status(&foreign_status).await;
        assert!(status_result.is_err());

        close_store_and_remove_root(owner_store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_mutation_allows_current_session_task_claim_for_run_task() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-task-claim");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.run_id = "run-task-claim".to_string();
        status.task_id = "task-task-claim".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner status");
        store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-task-claim"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-task-only-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-task-claim".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-task-claim".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current task claim");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-task-claim",
                "task-task-claim",
                "2026-05-21T01:00:00Z",
            ))
            .await
            .expect("task claim should authorize run continuation mutation");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_mutation_allows_claimed_parent_explicit_task_binding_child_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-run-graph-mutation-explicit-task-child");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut parent_status = sample_run_graph_status();
        parent_status.run_id = "parent-run".to_string();
        parent_status.task_id = "parent-run".to_string();
        store
            .record_run_graph_status(&parent_status)
            .await
            .expect("persist parent status");
        store
            .persist_task_record(test_task_record("child-task-run", "open"))
            .await
            .expect("persist child task");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-parent-run-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("parent-run".to_string()),
                run_id: Some("parent-run".to_string()),
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:parent-run".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current parent claim");
        let mut binding =
            sample_explicit_binding("parent-run", "child-task-run", "2026-05-21T01:00:00Z");
        binding.binding_source = "explicit_continuation_bind_task".to_string();
        binding.active_bounded_unit = serde_json::json!({
            "kind": "task_graph_task",
            "run_id": "parent-run",
            "task_id": "child-task-run",
        });
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record explicit parent-child binding");

        let mut child_status = sample_run_graph_status();
        child_status.run_id = "child-task-run".to_string();
        child_status.task_id = "child-task-run".to_string();
        store
            .record_run_graph_status(&child_status)
            .await
            .expect("parent claim should authorize bound child task-as-run mutation");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_explicit_binding_for_current_session_uses_task_claim_run_scope() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner");
        }
        let root = temp_run_graph_root("vida-current-session-task-claim-run-scope");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.run_id = "run-scope-from-task".to_string();
        status.task_id = "task-scope-from-task".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner status");
        store
            .record_run_graph_dispatch_receipt(&sample_dispatch_receipt("run-scope-from-task"))
            .await
            .expect("persist owner receipt");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-task-scope-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-scope-from-task".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-scope-from-task".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("current task claim");
        store
            .persist_task_record(test_task_record("next-bound-task", "open"))
            .await
            .expect("persist bound task");
        let mut binding = sample_explicit_binding(
            "run-scope-from-task",
            "next-bound-task",
            "2026-05-21T01:00:00Z",
        );
        binding.binding_source = "explicit_continuation_bind_task".to_string();
        binding.active_bounded_unit = serde_json::json!({
            "kind": "task_graph_task",
            "run_id": "run-scope-from-task",
            "task_id": "next-bound-task",
        });
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record explicit binding");

        let binding = store
            .latest_explicit_run_graph_continuation_binding_for_current_session()
            .await
            .expect("read current scoped binding")
            .expect("binding should be in current session scope");

        assert_eq!(binding.run_id, "run-scope-from-task");
        assert_eq!(binding.task_id, "next-bound-task");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_prefers_explicit_binding_over_current_session_claim() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-explicit-priority");
        }
        let root = temp_run_graph_root("vida-explicit-binding-status-priority");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-claimed", "in_progress"))
            .await
            .expect("persist claimed task");
        store
            .persist_task_record(test_task_record("task-bound", "in_progress"))
            .await
            .expect("persist explicitly bound task");

        let mut claimed_status = sample_run_graph_status();
        claimed_status.run_id = "run-claimed".to_string();
        claimed_status.task_id = "task-claimed".to_string();
        store
            .record_run_graph_status(&claimed_status)
            .await
            .expect("persist claimed status");
        let mut bound_status = sample_run_graph_status();
        bound_status.run_id = "run-bound".to_string();
        bound_status.task_id = "task-bound".to_string();
        store
            .record_run_graph_status(&bound_status)
            .await
            .expect("persist bound status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "claimed-run-session-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: root.display().to_string(),
                orchestrator_session_id: "session-explicit-priority".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("task-claimed".to_string()),
                run_id: Some("run-claimed".to_string()),
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-claimed".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("acquire current session claim");
        let mut binding =
            sample_explicit_binding("run-bound", "task-bound", "2026-05-21T01:00:00Z");
        binding.binding_source = "explicit_continuation_bind_task".to_string();
        binding.active_bounded_unit = serde_json::json!({
            "kind": "task_graph_task",
            "run_id": "run-bound",
            "task_id": "task-bound",
            "orchestrator_session_id": "session-explicit-priority",
        });
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record explicit binding");

        let status = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read current-session status")
            .expect("explicit binding should resolve a status");
        assert_eq!(status.run_id, "run-bound");
        assert_eq!(status.task_id, "task-bound");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[test]
    fn owner_evidence_matches_prior_generated_local_session_by_stable_fallback() {
        let prior_owner_evidence = serde_json::json!({
            "current_session": {
                "session_id": "local-session-worktreehash-12345",
                "fallback_replaces_legacy_stable_worktree_state_hash": "local-worktree-worktreehash"
            }
        });
        let current_owner_evidence = serde_json::json!({
            "mutation_gate": "current_session_allowed",
            "live_other_sessions": [],
            "stale_sessions": [{
                "session_id": "stale-foreign-project",
                "project_root": "\\\\?\\C:\\project\\other",
                "worktree_environment_id": "\\\\?\\C:\\project\\other"
            }],
            "current_session": {
                "session_id": "local-session-worktreehash",
                "fallback_replaces_legacy_stable_worktree_state_hash": "local-worktree-worktreehash"
            }
        });

        assert!(StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "local-session-worktreehash",
            Some("local-worktree-worktreehash"),
        ));
        assert!(!StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "local-session-other",
            Some("local-worktree-other"),
        ));
    }

    #[test]
    fn owner_evidence_adopts_stale_codex_thread_owner_for_same_worktree() {
        let prior_owner_evidence = serde_json::json!({
            "current_session": {
                "session_id": "019e-old-codex-thread",
                "identity_source": "CODEX_THREAD_ID",
                "project_root": "\\\\?\\C:\\project\\vida_mobile",
                "worktree_environment_id": "\\\\?\\C:\\project\\vida_mobile"
            }
        });
        let current_owner_evidence = serde_json::json!({
            "mutation_gate": "current_session_allowed",
            "live_other_sessions": [],
            "current_session": {
                "session_id": "019e-new-codex-thread",
                "identity_source": "CODEX_THREAD_ID",
                "project_root": "\\\\?\\C:\\project\\vida_mobile",
                "worktree_environment_id": "\\\\?\\C:\\project\\vida_mobile"
            }
        });

        assert!(StateStore::owner_evidence_matches_current_session(
            &prior_owner_evidence,
            &current_owner_evidence,
            "019e-new-codex-thread",
            None,
        ));
    }

    #[tokio::test]
    async fn run_graph_mutation_adopts_legacy_same_worktree_owner_evidence_without_competing_owner()
    {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_env = saved_runtime_session_env();
        clear_runtime_session_env();

        let root = temp_run_graph_root("vida-run-graph-legacy-owner-evidence-adopt");
        let legacy_store = StateStore::open(root.clone())
            .await
            .expect("open legacy store");
        let mut status = sample_run_graph_status();
        status.run_id = "legacy-owner-evidence-run".to_string();
        status.task_id = "legacy-owner-evidence-task".to_string();
        legacy_store
            .record_run_graph_status(&status)
            .await
            .expect("persist legacy ownerless status");
        legacy_store
            .record_run_graph_owner_evidence("legacy-owner-evidence-run", "dispatch_context")
            .await
            .expect("record legacy local owner evidence");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "current-explicit-session");
        }
        let result = legacy_store.record_run_graph_status(&status).await;
        assert!(
            result.is_ok(),
            "legacy same-worktree owner evidence should be adoptable when no live/stale competing owner exists: {result:?}"
        );

        close_store_and_remove_root(legacy_store, root).await;
        restore_runtime_session_env(saved_env);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_ignores_foreign_claims() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-latest");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for task_id in ["task-foreign", "task-current"] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: "Current session latest task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "in_progress",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create current session latest task");
        }

        let mut foreign_status = sample_run_graph_status();
        foreign_status.run_id = "run-foreign".to_string();
        foreign_status.task_id = "task-foreign".to_string();
        store
            .record_run_graph_status(&foreign_status)
            .await
            .expect("persist foreign run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "foreign-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-foreign".to_string(),
                process_id: None,
                task_id: Some("task-foreign".to_string()),
                run_id: Some("run-foreign".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("foreign-domain".to_string()),
                owned_paths: vec!["foreign/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire foreign claim");

        assert_eq!(
            store
                .latest_run_graph_status()
                .await
                .expect("read global latest")
                .expect("global latest present")
                .run_id,
            "run-foreign"
        );
        assert!(
            store
                .latest_run_graph_status_for_current_session()
                .await
                .expect("read scoped latest")
                .is_none()
        );

        let mut current_status = sample_run_graph_status();
        current_status.run_id = "run-current".to_string();
        current_status.task_id = "task-current".to_string();
        store
            .record_run_graph_status(&current_status)
            .await
            .expect("persist current run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current".to_string(),
                process_id: None,
                task_id: Some("task-current".to_string()),
                run_id: Some("run-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-open-domain".to_string()),
                owned_paths: vec!["open-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire current claim");

        assert_eq!(
            store
                .latest_run_graph_status_for_current_session()
                .await
                .expect("read scoped latest")
                .expect("scoped latest present")
                .run_id,
            "run-current"
        );

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_skips_closed_task_active_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-closed-skip");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-closed-skip");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for (task_id, status) in [
            ("task-open-current-run", "open"),
            ("task-closed-current-run", "closed"),
        ] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: "Current session run graph task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status,
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create task");
        }

        let mut open_status = crate::taskflow_run_graph::default_run_graph_status(
            "task-open-current-run",
            "implementation",
            "implementation",
        );
        open_status.run_id = "run-open-current".to_string();
        store
            .record_run_graph_status(&open_status)
            .await
            .expect("persist open run graph status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-open-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-closed-skip".to_string(),
                process_id: None,
                task_id: Some("task-open-current-run".to_string()),
                run_id: Some("run-open-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-open-domain".to_string()),
                owned_paths: vec!["open-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire open claim");

        let mut stale_closed_status = crate::taskflow_run_graph::default_run_graph_status(
            "task-closed-current-run",
            "implementation",
            "implementation",
        );
        stale_closed_status.run_id = "run-zzz-closed-current".to_string();
        stale_closed_status.status = "ready".to_string();
        stale_closed_status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        store
            .record_run_graph_status(&stale_closed_status)
            .await
            .expect("persist stale closed-task status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-closed-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-closed-skip".to_string(),
                process_id: None,
                task_id: Some("task-closed-current-run".to_string()),
                run_id: Some("run-zzz-closed-current".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-closed-domain".to_string()),
                owned_paths: vec!["closed-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire closed claim");

        let latest = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read scoped latest")
            .expect("open run should remain after closed-task run is skipped");
        assert_eq!(latest.run_id, "run-open-current");
        assert_eq!(latest.task_id, "task-open-current-run");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn orchestrator_init_does_not_reconcile_open_blocked_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-open-blocked-run");
        }
        let root = temp_run_graph_root("vida-run-graph-open-blocked-run");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "open-blocked-task",
                title: "Open blocked task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create open blocked task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "open-blocked-run",
            "implementation",
            "implementation",
        );
        status.task_id = "open-blocked-task".to_string();
        status.active_node = "developer".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "developer_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist open blocked run");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "open-blocked-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-open-blocked-run".to_string(),
                process_id: None,
                task_id: Some("open-blocked-task".to_string()),
                run_id: Some("open-blocked-run".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("open-blocked-domain".to_string()),
                owned_paths: vec!["blocked-scope/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire open blocked run claim");

        assert!(
            !store
                .run_graph_status_points_to_terminal_task_active(&status)
                .await
                .expect("classify open blocked run"),
            "open blocked runs must not be classified as closed-task projection mismatches"
        );
        let latest = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read latest status")
            .expect("open blocked run remains active");
        assert_eq!(latest.run_id, "open-blocked-run");
        assert_eq!(latest.task_id, "open-blocked-task");
        assert_eq!(latest.lifecycle_stage, "developer_blocked");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_recovery_for_current_session_keeps_active_exception_takeover_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-exception-takeover");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-exception-takeover");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-current-exception-takeover",
                title: "Current session exception takeover task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create current exception takeover task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-current-exception-takeover",
            "task-current-exception-takeover",
            "implementation",
        );
        status.run_id = "run-current-exception-takeover".to_string();
        status.task_id = "task-current-exception-takeover".to_string();
        status.active_node = "analyst".to_string();
        status.lifecycle_stage = "analyst_blocked".to_string();
        status.status = "blocked".to_string();
        status.resume_target = "dispatch.analyst".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist current exception takeover status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-exception-takeover-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-exception-takeover".to_string(),
                process_id: None,
                task_id: Some("task-current-exception-takeover".to_string()),
                run_id: Some("run-current-exception-takeover".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-exception-domain".to_string()),
                owned_paths: vec!["crates/vida/src/completed_exception.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire current exception takeover claim");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-current-exception-takeover".to_string(),
                dispatch_target: "analyst".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("exception-receipt-1".to_string()),
                exception_path_receipt_id: Some("exception-receipt-1".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida lane complete --host-bridge-request".to_string()),
                dispatch_command: Some(
                    "vida lane complete run-current-exception-takeover --json".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/current-exception-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/current-exception-result.json".to_string()),
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
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-12T08:00:00Z".to_string(),
            })
            .await
            .expect("persist active exception takeover receipt");

        let latest = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read current-session latest")
            .expect("active exception takeover run remains current-session latest");
        assert_eq!(latest.run_id, "run-current-exception-takeover");

        let recovery = store
            .latest_run_graph_recovery_summary_for_current_session()
            .await
            .expect("read current-session recovery")
            .expect("active exception takeover recovery remains current-session latest");
        assert_eq!(recovery.run_id, "run-current-exception-takeover");

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_skips_completed_exception_takeover_run() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-completed-exception-skip");
        }
        let root = temp_run_graph_root("vida-run-graph-completed-exception-skip");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let current_session_id = "session-completed-exception-skip".to_string();
        let labels = Vec::new();
        for task_id in ["task-completed-exception", "task-next-open"] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: "Current session exception takeover task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "in_progress",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create task");
        }

        let next_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-next-open",
            "task-next-open",
            "implementation",
        );
        store
            .record_run_graph_status(&next_status)
            .await
            .expect("persist next open status");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "next-open-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: current_session_id.clone(),
                process_id: None,
                task_id: Some("task-next-open".to_string()),
                run_id: Some("run-next-open".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("next-open-domain".to_string()),
                owned_paths: vec!["crates/vida/src/next_open.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire next open claim");

        let mut completed_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-completed-exception",
            "task-completed-exception",
            "specification",
        );
        completed_status.active_node = "analyst".to_string();
        completed_status.status = "completed".to_string();
        completed_status.lifecycle_stage = "analyst_complete".to_string();
        completed_status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&completed_status)
            .await
            .expect("persist completed exception status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-completed-exception".to_string(),
                dispatch_target: "analyst".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("exception-receipt-1".to_string()),
                exception_path_receipt_id: Some("exception-receipt-1".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida lane exception-takeover".to_string()),
                dispatch_command: Some(
                    "vida lane exception-takeover run-completed-exception".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/completed-exception-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/completed-exception-result.json".to_string()),
                blocker_code: Some("old_blocker".to_string()),
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
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-12T08:00:00Z".to_string(),
            })
            .await
            .expect("persist completed exception receipt");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "completed-exception-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: current_session_id,
                process_id: None,
                task_id: Some("task-completed-exception".to_string()),
                run_id: Some("run-completed-exception".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("completed-exception-domain".to_string()),
                owned_paths: vec!["crates/vida/src/completed_exception.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire completed exception claim");

        let latest = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("read current-session latest");
        assert!(
            latest
                .as_ref()
                .is_none_or(|status| status.run_id != "run-completed-exception"),
            "completed exception takeover must not remain current-session latest"
        );

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_task_ignores_newer_foreign_run() {
        let root = temp_run_graph_root("vida-run-graph-task-scoped-latest");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for task_id in ["task-requested", "task-foreign"] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: task_id,
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "in_progress",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create run graph task");
        }

        let mut requested = sample_run_graph_status();
        requested.run_id = "run-requested-task".to_string();
        requested.task_id = "task-requested".to_string();
        store
            .record_run_graph_status(&requested)
            .await
            .expect("persist requested status");

        let mut foreign = sample_run_graph_status();
        foreign.run_id = "run-foreign-newer".to_string();
        foreign.task_id = "task-foreign".to_string();
        store
            .record_run_graph_status(&foreign)
            .await
            .expect("persist foreign status");

        let status = store
            .latest_run_graph_status_for_task("task-requested")
            .await
            .expect("read task-scoped latest")
            .expect("requested task status should resolve");
        assert_eq!(status.run_id, "run-requested-task");
        assert_eq!(status.task_id, "task-requested");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_task_keeps_active_exception_takeover_run() {
        let root = temp_run_graph_root("vida-run-graph-task-scoped-exception-takeover");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-requested-exception-takeover",
                title: "Requested exception takeover task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create requested exception takeover task");

        let mut status = sample_run_graph_status();
        status.run_id = "run-requested-exception-takeover".to_string();
        status.task_id = "task-requested-exception-takeover".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "bridge_request_pending".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist requested exception takeover status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-requested-exception-takeover".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("exception-takeover-receipt".to_string()),
                exception_path_receipt_id: Some("exception-takeover-receipt".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
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
                activation_runtime_role: Some("implementer".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist active exception takeover receipt");

        let scoped = store
            .latest_run_graph_status_for_task("task-requested-exception-takeover")
            .await
            .expect("read task-scoped latest")
            .expect("active exception takeover status should resolve by task id");
        assert_eq!(scoped.run_id, "run-requested-exception-takeover");
        assert_eq!(scoped.task_id, "task-requested-exception-takeover");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_explicit_continuation_binding_for_current_session_skips_foreign_binding() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-binding");
        }
        let root = temp_run_graph_root("vida-run-graph-current-session-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-current-binding",
                "task-current-binding",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist current binding");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "current-binding-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-binding".to_string(),
                process_id: None,
                task_id: Some("task-current-binding".to_string()),
                run_id: Some("run-current-binding".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("current-binding-domain".to_string()),
                owned_paths: vec!["current/binding/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire current binding claim");
        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-foreign-binding",
                "task-foreign-binding",
                "2026-05-21T01:00:00Z",
            ))
            .await
            .expect("persist newer foreign binding");

        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding()
                .await
                .expect("read global binding")
                .expect("global binding present")
                .run_id,
            "run-foreign-binding"
        );
        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding_for_current_session()
                .await
                .expect("read scoped binding")
                .expect("scoped binding present")
                .run_id,
            "run-current-binding"
        );

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_explicit_continuation_binding_for_current_session_uses_current_owner_evidence_without_claim()
     {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner-evidence-binding");
        }
        let root = temp_run_graph_root("vida-run-graph-owner-evidence-binding");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owner-evidence-binding",
                "task-owner-evidence-binding",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist owner-evidence binding");

        assert!(
            store
                .active_orchestrator_claims()
                .await
                .expect("read claims")
                .is_empty()
        );
        assert_eq!(
            store
                .latest_explicit_run_graph_continuation_binding_for_current_session()
                .await
                .expect("read scoped binding")
                .expect("scoped binding present")
                .run_id,
            "run-owner-evidence-binding"
        );

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_for_current_session_uses_owner_evidence_without_claim() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-owner-evidence-status");
        }
        let root = temp_run_graph_root("vida-run-graph-owner-evidence-status");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-owner-evidence-status",
                title: "Owner evidence task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create owner evidence task");

        let mut status = sample_run_graph_status();
        status.run_id = "run-owner-evidence-status".to_string();
        status.task_id = "task-owner-evidence-status".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist owner-evidence status");
        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-owner-evidence-status",
                "task-owner-evidence-status",
                "2026-05-21T00:00:00Z",
            ))
            .await
            .expect("persist owner-evidence binding");

        assert!(
            store
                .active_orchestrator_claims()
                .await
                .expect("read claims")
                .is_empty()
        );
        assert_eq!(
            store
                .latest_run_graph_status_for_current_session()
                .await
                .expect("read scoped status")
                .expect("scoped status present")
                .run_id,
            "run-owner-evidence-status"
        );

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn run_graph_status_does_not_reconcile_closed_in_progress_task_into_completed() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-task-close-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "feature-close-dev",
                title: "Implement bounded fix",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create active task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-close-task",
            "implementation",
            "implementation",
        );
        status.task_id = "feature-close-dev".to_string();
        status.active_node = "implementer".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "none".to_string();
        status.checkpoint_kind = "active".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        store
            .close_task("feature-close-dev", "implemented and proven")
            .await
            .expect("close active task");

        let reconciled = store
            .run_graph_status("run-close-task")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.status, "in_progress");
        assert_eq!(reconciled.lifecycle_stage, "implementer_active");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.policy_gate, "targeted_verification");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.checkpoint_kind, "active");
        assert_eq!(reconciled.resume_target, "none");
        assert!(reconciled.recovery_ready);
        assert!(reconciled.delegation_gate().delegated_cycle_open);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn operator_run_graph_selector_preserves_open_closed_task_run_without_mutating_raw_status()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-operator-run-graph-closed-selector-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-scoped-diagnostic",
                title: "Closed task with archived scoped run",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let mut raw = crate::taskflow_run_graph::default_run_graph_status(
            "task-closed-scoped-diagnostic",
            "implementation",
            "implementation",
        );
        raw.run_id = "run-closed-scoped-diagnostic".to_string();
        raw.status = "in_progress".to_string();
        raw.active_node = "implementer".to_string();
        raw.lifecycle_stage = "implementer_active".to_string();
        raw.policy_gate = "targeted_verification".to_string();
        raw.checkpoint_kind = "active".to_string();
        raw.recovery_ready = true;
        store
            .record_run_graph_status(&raw)
            .await
            .expect("persist raw closed-task run status");

        let scoped = store
            .run_graph_status_for_operator_selector("task-closed-scoped-diagnostic")
            .await
            .expect("resolve task-scoped run-graph status");
        assert_eq!(scoped.run_id, "run-closed-scoped-diagnostic");
        assert_eq!(scoped.task_id, "task-closed-scoped-diagnostic");
        assert_eq!(scoped.status, "in_progress");
        assert_eq!(scoped.active_node, "implementer");
        assert_eq!(scoped.lifecycle_stage, "implementer_active");
        assert_eq!(scoped.policy_gate, "targeted_verification");
        assert!(scoped.recovery_ready);

        let raw_after = store
            .run_graph_status("run-closed-scoped-diagnostic")
            .await
            .expect("read raw run-graph status");
        assert_eq!(raw_after.status, "in_progress");
        assert_eq!(raw_after.active_node, "implementer");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn operator_run_graph_selector_archives_latest_receiptless_open_handoff_without_mutating_raw_status()
     {
        let root = temp_run_graph_root("vida-operator-receiptless-archive");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let task_id = "task-receiptless-archive";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id,
                title: "Closed task with receiptless open handoff",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let mut older = crate::taskflow_run_graph::default_run_graph_status(
            "run-a-receiptless-archive",
            "implementation",
            "implementation",
        );
        older.task_id = task_id.to_string();
        older.status = "in_progress".to_string();
        older.next_node = None;
        older.handoff_state = "none".to_string();
        older.resume_target = "none".to_string();
        store
            .record_run_graph_status(&older)
            .await
            .expect("persist older generic run");

        let mut current = crate::taskflow_run_graph::default_run_graph_status(
            "run-z-receiptless-archive",
            "implementation",
            "implementation",
        );
        current.task_id = task_id.to_string();
        current.active_node = "coder".to_string();
        current.next_node = Some("tester".to_string());
        current.status = "ready".to_string();
        current.lifecycle_stage = "coder_active".to_string();
        current.handoff_state = "awaiting_tester".to_string();
        current.context_state = "sealed".to_string();
        current.checkpoint_kind = "execution_cursor".to_string();
        current.resume_target = "dispatch.tester_lane".to_string();
        current.recovery_ready = true;
        store
            .record_run_graph_status(&current)
            .await
            .expect("persist current receiptless handoff");

        let scoped = store
            .run_graph_status_for_operator_selector(task_id)
            .await
            .expect("resolve latest task-scoped run");
        assert_eq!(scoped.run_id, current.run_id);
        assert_eq!(scoped.status, "completed");
        assert_eq!(scoped.active_node, "closure");
        assert_eq!(scoped.lifecycle_stage, "closure_complete");
        assert_eq!(scoped.policy_gate, "closed_run_archived");

        let raw_after = store
            .run_graph_status(&current.run_id)
            .await
            .expect("read raw current run");
        assert_eq!(raw_after.status, "ready");
        assert_eq!(raw_after.active_node, "coder");
        assert_eq!(raw_after.next_node.as_deref(), Some("tester"));
        assert_eq!(raw_after.handoff_state, "awaiting_tester");
        assert_eq!(raw_after.resume_target, "dispatch.tester_lane");
        assert!(raw_after.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn operator_run_graph_selector_keeps_lawful_exception_takeover_active() {
        let root = temp_run_graph_root("vida-operator-active-exception-takeover");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let task_id = "task-operator-active-exception";
        let run_id = "run-operator-active-exception";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id,
                title: "Closed task with active exception takeover",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = task_id.to_string();
        status.active_node = "tester".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "tester_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.tester".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist active exception status");

        let mut receipt = sample_dispatch_receipt(run_id);
        receipt.dispatch_target = "tester".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("pending_test_evidence".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("superseded-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist active exception receipt");

        let scoped = store
            .run_graph_status_for_operator_selector(run_id)
            .await
            .expect("resolve exception-takeover run");
        assert_ne!(scoped.status, "completed");
        assert_ne!(scoped.active_node, "closure");
        assert_ne!(scoped.policy_gate, "closed_run_archived");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_status_reconciles_closed_terminal_closure_blocker_into_completed() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closed-terminal-closure-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "feature-terminal-closure",
                title: "Closed terminal closure task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        for lifecycle_stage in ["closure_blocked", "closure_complete"] {
            let run_id = format!("run-{lifecycle_stage}");
            let mut status = crate::taskflow_run_graph::default_run_graph_status(
                &run_id, "delivery", "delivery",
            );
            status.task_id = "feature-terminal-closure".to_string();
            status.active_node = "closure".to_string();
            status.status = "blocked".to_string();
            status.lifecycle_stage = lifecycle_stage.to_string();
            status.next_node = None;
            status.policy_gate = "single_task_scope_required".to_string();
            status.handoff_state = "none".to_string();
            status.context_state = "ready".to_string();
            status.checkpoint_kind = "blocked".to_string();
            status.resume_target = "none".to_string();
            status.recovery_ready = false;
            store
                .record_run_graph_status(&status)
                .await
                .expect("persist terminal closure run-graph status");

            let reconciled = store
                .run_graph_status(&run_id)
                .await
                .expect("load reconciled terminal closure status");
            assert_eq!(reconciled.status, "completed");
            assert_eq!(reconciled.lifecycle_stage, "closure_complete");
            assert_eq!(reconciled.next_node, None);
            assert_eq!(reconciled.policy_gate, "not_required");
            assert_eq!(reconciled.handoff_state, "none");
            assert_eq!(reconciled.context_state, "sealed");
            assert_eq!(reconciled.checkpoint_kind, "none");
            assert_eq!(reconciled.resume_target, "none");
            assert!(!reconciled.recovery_ready);
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_heals_legacy_downstream_preview_drift_for_exception_recorded_active_dispatch()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-legacy-preview-drift-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-legacy-preview-drift",
            "implementer",
            "delivery",
        );
        status.task_id = "task-legacy-preview-drift".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-legacy-preview-drift".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: Some("sup-parent".to_string()),
                exception_path_receipt_id: Some("exc-parent".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("implementer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale retry preview".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/stale-preview.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/stale-preview-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist legacy drift receipt");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("summary should heal legacy drift")
            .expect("summary should exist");
        assert_eq!(summary.lane_status, "lane_exception_recorded");
        assert!(summary.downstream_dispatch_status.is_none());
        assert!(!summary.downstream_dispatch_ready);
        assert!(summary.downstream_dispatch_packet_path.is_none());

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_status_keeps_advanced_handoff_after_exception_takeover_lane_moves_on() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-exception-takeover-advanced-handoff-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-exception-advanced",
            "implementation",
            "implementation",
        );
        status.task_id = "task-exception-advanced".to_string();
        status.active_node = "test_author".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run graph status");

        let mut receipt = sample_dispatch_receipt("run-exception-advanced");
        receipt.dispatch_target = "test_author".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");
        store
            .acquire_current_session_run_graph_claim_for_test(
                "advanced-exception-handoff-claim",
                "run-exception-advanced",
                "task-exception-advanced",
                "run-graph-continuation-ownership",
                "crates/vida/src/state_store_run_graph_summary.rs",
            )
            .await
            .expect("current session should claim advanced exception fixture");

        let mut advanced = status.clone();
        advanced.active_node = "coach".to_string();
        advanced.next_node = Some("review_ensemble".to_string());
        advanced.status = "ready".to_string();
        advanced.lane_id = "coach_lane".to_string();
        advanced.lifecycle_stage = "coach_active".to_string();
        advanced.policy_gate = "review_findings".to_string();
        advanced.handoff_state = "awaiting_review_ensemble".to_string();
        advanced.checkpoint_kind = "execution_cursor".to_string();
        advanced.resume_target = "dispatch.review_ensemble".to_string();
        advanced.recovery_ready = true;
        store
            .record_run_graph_status(&advanced)
            .await
            .expect("persist advanced handoff status");

        let reconciled = store
            .run_graph_status("run-exception-advanced")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "coach");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.resume_target, "dispatch.review_ensemble");
        assert!(reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_materialization_pass_keeps_activation_view_only_receipt_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-tracked-flow-materialization-pass-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-tracked-flow-materialization",
            "spec-pack",
            "scope_discussion",
        );
        status.task_id = "task-tracked-flow-materialization".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "work_pool_pack_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked materialization status");

        let result_path = root.join("tracked-flow-materialization-result.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": false,
                    "reused_existing": true,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": []
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-tracked-flow-materialization");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked materialization receipt");

        let reconciled = store
            .run_graph_status("run-tracked-flow-materialization")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "work-pool-pack");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.lifecycle_stage, "work_pool_pack_blocked");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_materialization_with_null_blocker_records_work_pool_identity() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-tracked-flow-materialization-null-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let tasks_path = root.join("tasks.jsonl");
        fs::write(
            &tasks_path,
            concat!(
                "{\"id\":\"feature-example\",\"title\":\"Feature\",\"description\":\"feature\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"spec-first-feature\"],\"dependencies\":[]}\n",
                "{\"id\":\"feature-example-spec\",\"title\":\"Spec\",\"description\":\"spec\",\"status\":\"closed\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"spec-pack\"],\"dependencies\":[{\"issue_id\":\"feature-example-spec\",\"depends_on_id\":\"feature-example\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"feature-example-work-pool\",\"title\":\"Work pool\",\"description\":\"work pool\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"work_pool\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"work-pool-pack\"],\"dependencies\":[{\"issue_id\":\"feature-example-work-pool\",\"depends_on_id\":\"feature-example\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write tasks");
        store
            .import_tasks_from_jsonl(&tasks_path)
            .await
            .expect("import tasks");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-tracked-flow-null-blocker",
            "feature-example",
            "scope_discussion",
        );
        status.task_id = "feature-example".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "work_pool_pack_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist materialization status");

        let result_path = root.join("tracked-flow-materialization-null-blocker-result.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": true,
                    "reused_existing": false,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": ["taskflow:feature-example-work-pool"]
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-tracked-flow-null-blocker");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        receipt.blocker_code = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist materialization receipt");

        let loaded = store
            .run_graph_dispatch_receipt("run-tracked-flow-null-blocker")
            .await
            .expect("load dispatch receipt")
            .expect("receipt should exist");
        assert_eq!(loaded.dispatch_surface.as_deref(), Some("vida task ensure"));
        let identity = store
            .run_graph_dispatch_task_identity("run-tracked-flow-null-blocker")
            .await
            .expect("load dispatch task identity")
            .expect("materialized work-pool identity should be recorded");
        assert_eq!(identity.feature_epic_id.as_deref(), Some("feature-example"));
        assert_eq!(
            identity.spec_task_id.as_deref(),
            Some("feature-example-spec")
        );
        assert_eq!(
            identity.work_pool_task_id.as_deref(),
            Some("feature-example-work-pool")
        );
        assert_eq!(
            identity.source,
            "work_pool_materialization_identity_reconciliation"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn work_pool_materialization_pass_without_result_path_keeps_receipt_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-work-pool-materialization-missing-result-path-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-work-pool-materialization",
            "spec-pack",
            "scope_discussion",
        );
        status.task_id = "task-work-pool-materialization".to_string();
        status.active_node = "work-pool-pack".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "work_pool_pack_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked materialization status");

        let results_dir = root.join("runtime-consumption").join("dispatch-results");
        fs::create_dir_all(&results_dir).expect("create dispatch results dir");
        fs::write(
            results_dir.join("run-work-pool-materialization-2026-06-03T01-00-00Z.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida task ensure",
                "status": "pass",
                "packet_key": "work_pool_task",
                "task": {
                    "task_id": "feature-example-work-pool",
                    "created": false,
                    "reused_existing": true,
                    "label": "work-pool-pack"
                },
                "epic": {
                    "task_id": "feature-example",
                    "created": false
                },
                "changed_files": []
            }))
            .expect("render materialization result"),
        )
        .expect("write materialization result");

        let mut receipt = sample_dispatch_receipt("run-work-pool-materialization");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.dispatch_result_path = None;
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked materialization receipt");

        let reconciled = store
            .run_graph_status("run-work-pool-materialization")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.active_node, "work-pool-pack");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.lifecycle_stage, "work_pool_pack_blocked");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn completed_run_status_is_downgraded_by_newer_blocked_dispatch_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-completed-over-stale-blocked-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure",
            "closure",
            "delivery",
        );
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
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure".to_string(),
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
                downstream_dispatch_note: Some("stale blocked coach lineage".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_closure_handoff".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale blocked dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure")
            .await
            .expect("load reconciled completed run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "verification");
        assert_eq!(reconciled.lifecycle_stage, "verification_blocked");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.resume_target, "none");
        assert_eq!(reconciled.selected_backend, "senior");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn terminal_closure_status_survives_blocked_receipt_after_explicit_takeover() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-terminal-closure-over-takeover-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-closure",
            "implementation",
            "implementation",
        );
        mark_terminal_closure_status(&mut status);
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist terminal closure status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-terminal-closure".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("sup-terminal-closure".to_string()),
                exception_path_receipt_id: Some("exc-terminal-closure".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: Some("stale terminal blocker".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            })
            .await
            .expect("persist superseded blocked dispatch receipt");

        let reconciled = store
            .run_graph_status("run-terminal-closure")
            .await
            .expect("load reconciled terminal closure status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert!(!reconciled.recovery_ready);
        assert_eq!(reconciled.selected_backend, "internal_subagents");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_superseded_lane_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-superseded-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "run-active",
                title: "Active task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create active task");

        let active = crate::taskflow_run_graph::default_run_graph_status(
            "run-active",
            "task-active",
            "implementation",
        );
        store
            .record_run_graph_status(&active)
            .await
            .expect("persist active status");

        let stale = crate::taskflow_run_graph::default_run_graph_status(
            "run-stale-superseded",
            "task-stale",
            "implementation",
        );
        store
            .record_run_graph_status(&stale)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-stale-superseded".to_string(),
                dispatch_target: "business_analyst".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_superseded".to_string(),
                supersedes_receipt_id: Some("supersede-stale-run".to_string()),
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
                downstream_dispatch_note: Some("stale delegated cycle".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("business_analyst".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-05-15T19:40:00Z".to_string(),
            })
            .await
            .expect("persist superseded dispatch receipt");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("non-superseded status should remain");
        assert_eq!(latest.run_id, "run-active");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_active_run_for_closed_task() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-active-closed-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-active-open",
                title: "Active open task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create active open task");
        store
            .show_task("task-active-open")
            .await
            .expect("active open task should be readable");

        let active = crate::taskflow_run_graph::default_run_graph_status(
            "task-active-open",
            "implementation",
            "implementation",
        );
        let mut active = active;
        active.run_id = "run-active-open-task".to_string();
        store
            .record_run_graph_status(&active)
            .await
            .expect("persist active open status");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-active-run",
                title: "Closed task with stale active run",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let mut stale = crate::taskflow_run_graph::default_run_graph_status(
            "task-closed-active-run",
            "implementation",
            "implementation",
        );
        stale.run_id = "run-closed-active-task".to_string();
        stale.task_id = "task-closed-active-run".to_string();
        stale.status = "ready".to_string();
        stale.lifecycle_stage = "implementation_dispatch_ready".to_string();
        store
            .record_run_graph_status(&stale)
            .await
            .expect("persist stale closed-task status");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-requested-active-run");
        }
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "requested-active-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-requested-active-run".to_string(),
                process_id: None,
                task_id: Some("task-active-open".to_string()),
                run_id: Some("run-active-open-task".to_string()),
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:task-active-open".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 60,
            })
            .await
            .expect("acquire requested active run claim");
        store
            .record_run_graph_continuation_binding(&sample_explicit_binding(
                "run-active-open-task",
                "task-active-open",
                "2026-07-14T00:00:00Z",
            ))
            .await
            .expect("persist requested active run binding");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("open-task run should remain latest after stale closed-task run is skipped");
        assert_eq!(latest.run_id, "run-active-open-task");

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("latest recovery should load")
            .expect("latest recovery should remain on active run");
        assert_eq!(recovery.run_id, "run-active-open-task");

        let scoped = store
            .latest_run_graph_status_for_task("task-active-open")
            .await
            .expect("scoped latest status should load")
            .expect("scoped latest status should remain on active run");
        assert_eq!(scoped.run_id, "run-active-open-task");

        let current_session = store
            .latest_run_graph_status_for_current_session()
            .await
            .expect("current-session latest status should load")
            .expect("explicit requested active run should win current-session selection");
        assert_eq!(current_session.run_id, "run-active-open-task");

        let current_session_recovery = store
            .latest_run_graph_recovery_summary_for_current_session()
            .await
            .expect("current-session recovery should load")
            .expect("current-session recovery should remain on active run");
        assert_eq!(current_session_recovery.run_id, "run-active-open-task");

        let binding = store
            .latest_explicit_run_graph_continuation_binding_for_current_session()
            .await
            .expect("current-session binding should load")
            .expect("requested active run binding should remain present");
        assert_eq!(binding.run_id, "run-active-open-task");

        let terminal_evidence = store
            .latest_terminal_task_active_run_graph_status()
            .await
            .expect("terminal-task-active evidence should load");
        assert!(
            terminal_evidence.is_none(),
            "terminal closed-task run must not remain active projection evidence"
        );

        let graph_summary = store
            .run_graph_summary()
            .await
            .expect("graph summary should load");
        assert_eq!(graph_summary.execution_plan_count, 2);

        close_store_and_remove_root(store, root).await;
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_archived_closed_task_with_stale_blocked_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-archived-closed-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for (task_id, status) in [("task-open", "in_progress"), ("task-closed", "closed")] {
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id,
                    title: "Run graph projection task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status,
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create task fixture");
        }

        let mut open_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-open",
            "task-open",
            "implementation",
        );
        open_status.task_id = "task-open".to_string();
        store
            .record_run_graph_status(&open_status)
            .await
            .expect("persist open run graph status");

        let mut archived_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-z-archived-closed",
            "task-closed",
            "closure",
        );
        archived_status.task_id = "task-closed".to_string();
        archived_status.status = "completed".to_string();
        archived_status.lifecycle_stage = "closure_complete".to_string();
        archived_status.next_node = None;
        archived_status.resume_target = "none".to_string();
        archived_status.recovery_ready = false;
        store
            .record_run_graph_status(&archived_status)
            .await
            .expect("persist archived closed-task status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-z-archived-closed".to_string(),
                dispatch_target: "coder".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
                dispatch_packet_path: Some("packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
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
                downstream_dispatch_active_target: Some("coder".to_string()),
                downstream_dispatch_last_target: Some("coder".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-07-15T23:00:08Z".to_string(),
            })
            .await
            .expect("persist stale closed-task receipt");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("open run should remain after archived closed-task run is skipped");
        assert_eq!(latest.run_id, "run-open");
        assert_eq!(latest.task_id, "task-open");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_status_skips_active_run_for_missing_task() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-run-graph-skips-active-missing-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "run-active-present-task",
                title: "Active present task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create active present task");
        store
            .show_task("run-active-present-task")
            .await
            .expect("active present task should be readable");

        let active = crate::taskflow_run_graph::default_run_graph_status(
            "run-active-present-task",
            "task-active-present",
            "implementation",
        );
        store
            .record_run_graph_status(&active)
            .await
            .expect("persist active present-task status");
        let active_status = store
            .run_graph_status("run-active-present-task")
            .await
            .expect("active present run should load");
        let active_verdict =
            crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                &store,
                &active_status,
            )
            .await
            .expect("active present run should have task authority verdict");
        assert_eq!(
            active_verdict.kind,
            crate::taskflow_run_graph_task_authority::RunGraphTaskAuthorityKind::AuthoritativeTaskPresent
        );

        let mut stale = crate::taskflow_run_graph::default_run_graph_status(
            "run-missing-active-task",
            "task-missing-active-run",
            "implementation",
        );
        stale.task_id = "task-missing-active-run".to_string();
        stale.status = "ready".to_string();
        stale.lifecycle_stage = "implementation_dispatch_ready".to_string();
        store
            .record_run_graph_status(&stale)
            .await
            .expect("persist stale missing-task status");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect(
                "present-task run should remain latest after stale missing-task run is skipped",
            );
        assert_eq!(latest.run_id, "run-active-present-task");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_status_retains_active_receipt_only_missing_execution_as_global_stale()
    {
        let root = temp_run_graph_root("vida-run-graph-receipt-only-missing-execution");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-receipt-only-missing-execution";
        let packet_path = root.join("receipt-only-packet.json").display().to_string();
        let receipt = RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "synthetic_target".to_string(),
            dispatch_status: "bridge_request_pending".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            dispatch_packet_path: Some(packet_path),
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
            selected_backend: None,
            recorded_at: "2026-07-19T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist active receipt-only run");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("read latest status")
            .expect("active receipt-only stale run should remain global");
        assert_eq!(latest.run_id, run_id);
        assert_eq!(latest.checkpoint_kind, "missing_execution_plan_state");
        assert_eq!(latest.status, "blocked");

        let mut executed = receipt;
        executed.dispatch_status = "executed".to_string();
        executed.recorded_at = "2026-07-19T00:00:01Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&executed)
            .await
            .expect("persist executed receipt-only run");
        assert!(
            store
                .latest_run_graph_status()
                .await
                .expect("read latest status after receipt closure")
                .is_none(),
            "missing execution without active receipt must be dropped"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn completed_run_status_is_downgraded_by_exception_recorded_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-completed-over-exception-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-exception",
            "closure",
            "delivery",
        );
        status.task_id = "task-closure-exception".to_string();
        status.active_node = "closure".to_string();
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
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure-exception".to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-1".to_string()),
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: None,
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist exception-recorded dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure-exception")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_blocked");
        assert_eq!(reconciled.checkpoint_kind, "none");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn terminal_closure_dispatch_executed_receipt_reconciles_run_to_closure_complete() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closure-dispatch-executed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-direct",
            "terminal_closure",
            "delivery",
        );
        status.task_id = "task-closure-direct".to_string();
        status.active_node = "terminal_closure".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "terminal_closure_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked closure status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closure-direct".to_string(),
                dispatch_target: "terminal_closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: Some(
                    "vida taskflow consume continue --run-id run-closure-direct --json".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist closure dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure-direct")
            .await
            .expect("load reconciled run graph status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn executed_specification_receipt_with_design_gate_blockers_clears_fake_delegated_lane_active()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-spec-design-gate-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let status = RunGraphStatus {
            run_id: "run-spec-design-gate".to_string(),
            task_id: "task-spec-design-gate".to_string(),
            task_class: "implementation".to_string(),
            active_node: "specification".to_string(),
            next_node: None,
            status: "running".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "specification_lane".to_string(),
            lifecycle_stage: "specification_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist specification-active run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-spec-design-gate".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure".to_string()),
                downstream_dispatch_note: Some(
                    "finalize the design doc and close spec-pack before work-pool shaping"
                        .to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                downstream_dispatch_trace_path: Some("/tmp/specification-trace.json".to_string()),
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist executed specification receipt");

        let reconciled = store
            .run_graph_status("run-spec-design-gate")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "specification");
        assert_eq!(reconciled.lifecycle_stage, "specification_complete");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        assert!(!reconciled.delegation_gate().delegated_cycle_open);
        assert_eq!(
            reconciled.delegation_gate().delegated_cycle_state,
            "clear".to_string()
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn retry_ready_internal_activation_receipt_preserves_dispatch_ready_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-retry-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let status = RunGraphStatus {
            run_id: "run-retry-ready".to_string(),
            task_id: "task-retry-ready".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "implementer_lane".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer_lane".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-retry-ready".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
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
            })
            .await
            .expect("persist blocked retry receipt");

        let reconciled = store
            .run_graph_status("run-retry-ready")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node.as_deref(), Some("implementer"));
        assert_eq!(reconciled.handoff_state, "awaiting_implementer");
        assert_eq!(reconciled.resume_target, "dispatch.implementer_lane");
        assert!(reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn stale_executing_receipt_is_normalized_on_read_and_clears_open_cycle() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-read-normalize-stale-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-read-normalize-stale",
            "verification",
            "delivery",
        );
        status.task_id = "task-read-normalize-stale".to_string();
        status.active_node = "verification".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "verification_active".to_string();
        status.next_node = None;
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "run_id": "run-read-normalize-stale",
                "dispatch_target": "verification"
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
        .expect("write stale result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-read-normalize-stale".to_string(),
                dispatch_target: "verification".to_string(),
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
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-19T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale receipt");

        let reconciled = store
            .run_graph_status("run-read-normalize-stale")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "verification");
        assert_eq!(reconciled.lifecycle_stage, "verification_blocked");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);
        assert!(!reconciled.delegation_gate().delegated_cycle_open);

        let persisted = store
            .run_graph_dispatch_receipt("run-read-normalize-stale")
            .await
            .expect("read persisted normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_dispatch_timeout_without_receipt")
        );
        assert_eq!(persisted.lane_status, "lane_exception_recorded");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_status_preserves_ready_handoff_against_stale_downstream_blockers() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-ready-handoff-stale-downstream-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-ready-handoff",
            "implementation",
            "implementation",
        );
        status.task_id = "task-ready-handoff".to_string();
        status.active_node = "analysis".to_string();
        status.next_node = Some("writer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "analysis_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "awaiting_writer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.writer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist ready writer handoff");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-ready-handoff".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("writer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some(
                    "stale preview before planner metadata repair".to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-12T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale downstream receipt");

        let reconciled = store
            .run_graph_status("run-ready-handoff")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "analysis");
        assert_eq!(reconciled.next_node.as_deref(), Some("writer"));
        assert_eq!(reconciled.lifecycle_stage, "analysis_active");
        assert_eq!(reconciled.handoff_state, "awaiting_writer");
        assert_eq!(reconciled.resume_target, "dispatch.writer_lane");
        assert!(reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn blocked_analysis_status_with_missing_owned_scope_reconciles_to_writer_handoff() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-missing-scope-handoff-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-missing-scope-handoff",
            "implementation",
            "implementation",
        );
        status.task_id = "task-missing-scope-handoff".to_string();
        status.active_node = "analysis".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked analysis status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-missing-scope-handoff".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida lane complete".to_string()),
                dispatch_command: Some("vida lane complete".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("writer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("activate writer".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist completed analysis receipt");

        let reconciled = store
            .run_graph_status("run-missing-scope-handoff")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "analysis");
        assert_eq!(reconciled.next_node.as_deref(), Some("writer"));
        assert_eq!(reconciled.lifecycle_stage, "analysis_active");
        assert_eq!(reconciled.handoff_state, "awaiting_writer");
        assert_eq!(reconciled.resume_target, "dispatch.writer_lane");
        assert!(reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn executed_activation_view_only_receipt_is_normalized_to_blocked_retry_truth() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-activation-view-only-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-activation-view-only",
            "implementation",
            "implementation",
        );
        status.task_id = "task-activation-view-only".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run graph status");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-activation-view-only.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "blocker_code": "internal_activation_view_only",
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-activation-view-only".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale coach handoff".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale executed receipt");

        let reconciled = store
            .run_graph_status("run-activation-view-only")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node.as_deref(), Some("implementer"));
        assert_eq!(reconciled.handoff_state, "awaiting_implementer");
        assert_eq!(reconciled.resume_target, "dispatch.implementer_lane");
        assert!(reconciled.recovery_ready);

        let persisted = store
            .run_graph_dispatch_receipt("run-activation-view-only")
            .await
            .expect("load normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert!(!persisted.downstream_dispatch_ready);
        assert!(persisted.downstream_dispatch_status.is_none());

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn executing_activation_view_only_blocked_result_is_normalized_to_blocked_truth() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-terminal-activation-view-only-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-activation-view-only",
            "implementation",
            "implementation",
        );
        status.task_id = "task-terminal-activation-view-only".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist retry-ready run graph status");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-terminal-activation-view-only.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-terminal-activation-view-only".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale coach handoff".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale executing receipt");

        let reconciled = store
            .run_graph_status("run-terminal-activation-view-only")
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "implementer");
        assert_eq!(reconciled.next_node, None);
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let persisted = store
            .run_graph_dispatch_receipt("run-terminal-activation-view-only")
            .await
            .expect("load normalized receipt")
            .expect("normalized receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert_eq!(persisted.lane_status, "lane_blocked");
        assert!(!persisted.downstream_dispatch_ready);
        assert!(persisted.downstream_dispatch_status.is_none());

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn closed_task_does_not_override_exception_recorded_run_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closed-task-exception-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-exception",
                title: "Closed task with exception-backed closure receipt",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closed-exception",
            "closure",
            "delivery",
        );
        status.task_id = "task-closed-exception".to_string();
        status.active_node = "closure".to_string();
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
            .expect("persist completed run-graph status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: "run-closed-exception".to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_exception_recorded".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-closed-1".to_string()),
                dispatch_kind: "closure".to_string(),
                dispatch_surface: None,
                dispatch_command: None,
                dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist exception-recorded dispatch receipt");

        store
            .close_task("task-closed-exception", "exception path recorded")
            .await
            .expect("close task");

        let reconciled = store
            .run_graph_status("run-closed-exception")
            .await
            .expect("load reconciled run-graph status");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(reconciled.selected_backend, "opencode_cli");
        assert!(!reconciled.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn terminal_task_active_projection_ignores_exception_takeover_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-terminal-task-active-exception-takeover-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-exception-takeover",
                title: "Closed task with active exception takeover",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closed-exception-takeover",
            "implementation",
            "implementation",
        );
        status.task_id = "task-closed-exception-takeover".to_string();
        status.active_node = "test_author".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "dispatch.test_author".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run-graph status");

        let mut receipt = sample_dispatch_receipt("run-closed-exception-takeover");
        receipt.dispatch_target = "test_author".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("pending_test_author_evidence".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");

        store
            .close_task(
                "task-closed-exception-takeover",
                "superseded by exception takeover",
            )
            .await
            .expect("close task");

        let terminal_active = store
            .latest_terminal_task_active_run_graph_status()
            .await
            .expect("load terminal task active projection");
        assert!(
            terminal_active.is_none(),
            "exception takeover receipt should keep closed task out of active projection"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn terminal_closure_historicalizes_active_exception_takeover_projection() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-terminal-closure-exception-takeover-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-terminal-exception-takeover",
                title: "Closed task with terminal exception takeover",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal-exception-takeover",
            "closure",
            "delivery",
        );
        status.task_id = "task-terminal-exception-takeover".to_string();
        mark_terminal_closure_status(&mut status);
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist terminal closure status");

        let mut receipt = sample_dispatch_receipt("run-terminal-exception-takeover");
        receipt.dispatch_target = "coder".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
        receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist active exception takeover receipt");

        store
            .close_task("task-terminal-exception-takeover", "terminal closure proof")
            .await
            .expect("close task");

        let reconciled = store
            .run_graph_status("run-terminal-exception-takeover")
            .await
            .expect("load terminal closure status");
        let projected = store
            .run_graph_dispatch_receipt_for_status(
                "run-terminal-exception-takeover",
                Some(&reconciled),
            )
            .await
            .expect("project terminal exception takeover receipt")
            .expect("projected receipt should exist");

        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert_eq!(projected.dispatch_status, "executed");
        assert_eq!(projected.lane_status, "lane_completed");
        assert_eq!(projected.blocker_code, None);
        assert_eq!(projected.exception_path_receipt_id, None);
        assert_eq!(projected.supersedes_receipt_id, None);
        assert_eq!(
            projected.downstream_dispatch_target.as_deref(),
            Some("closure")
        );
        assert_eq!(
            projected.downstream_dispatch_status.as_deref(),
            Some("retired_closed_task_run")
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_ignores_newer_automatic_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-upstream",
                title: "Upstream task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create upstream task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
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
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T09:00:00Z".to_string(),
            })
            .await
            .expect("record explicit binding");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-child".to_string(),
                task_id: "run-child".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": "run-child",
                    "run_id": "run-child",
                    "active_node": "implementer"
                }),
                binding_source: "run_graph_advance".to_string(),
                why_this_unit: "stale automatic child continuation".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:00:00Z".to_string(),
            })
            .await
            .expect("record automatic binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("explicit binding should exist");

        assert_eq!(latest.run_id, "run-upstream");
        assert_eq!(latest.binding_source, "explicit_continuation_bind_task");
        assert_eq!(latest.active_bounded_unit["kind"], "task_graph_task");
        assert_eq!(latest.active_bounded_unit["task_status"], "in_progress");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_skips_completed_bound_task() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-completed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-completed-binding",
                title: "Completed explicit binding task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create completed task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-completed-binding".to_string(),
                task_id: "task-completed-binding".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-completed-binding",
                    "run_id": "run-completed-binding",
                    "task_status": "completed",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale explicit completed task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T09:00:00Z".to_string(),
            })
            .await
            .expect("record completed explicit binding");

        assert!(
            store
                .latest_explicit_run_graph_continuation_binding()
                .await
                .expect("read latest explicit binding")
                .is_none()
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn active_exception_takeover_reconciles_stale_continuation_binding_for_next_lawful_sources()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-active-exception-takeover-reconciles-next-lawful-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "run-next-lawful-stale",
                title: "Next lawful stale binding task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-next-lawful-stale",
            "scope_discussion",
            "spec-pack",
        );
        status.task_id = "run-next-lawful-stale".to_string();
        status.active_node = "planning".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "planning_blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run status");

        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: status.run_id.clone(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("exc-next-lawful-stale".to_string()),
                exception_path_receipt_id: Some("exc-next-lawful-stale".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
                downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-05-06T11:58:00Z".to_string(),
            })
            .await
            .expect("record exception takeover receipt");
        store
            .acquire_current_session_run_graph_claim_for_test(
                "next-lawful-stale-binding-claim",
                "run-next-lawful-stale",
                "task-next-lawful-stale",
                "run-graph-continuation-ownership",
                "crates/vida/src/state_store_run_graph_summary.rs",
            )
            .await
            .expect("current session should claim next-lawful stale fixture");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": "planning"
                }),
                binding_source: "explicit_continuation_bind".to_string(),
                why_this_unit: "stale operator binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-05-06T11:50:00Z".to_string(),
            })
            .await
            .expect("record stale explicit binding");

        let current = store
            .run_graph_continuation_binding("run-next-lawful-stale")
            .await
            .expect("read current binding")
            .expect("current binding should exist");
        let explicit = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("latest explicit binding should exist");

        for binding in [current, explicit] {
            assert_eq!(
                binding.binding_source,
                "latest_run_graph_exception_takeover_dispatch"
            );
            assert_eq!(binding.active_bounded_unit["active_node"], "specification");
            assert_eq!(
                binding.sequential_vs_parallel_posture,
                "sequential_only_exception_takeover"
            );
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_skips_closed_task_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-skips-closed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed".to_string(),
                task_id: "task-closed".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-closed",
                    "run_id": "run-closed",
                    "task_status": "open",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale explicit task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:00:00Z".to_string(),
            })
            .await
            .expect("record closed explicit binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding");

        assert!(
            latest.is_none(),
            "closed explicit task binding must be skipped"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_clears_closed_task_graph_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-continuation-binding-clears-closed-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed-direct",
                title: "Closed direct task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed direct task");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed-direct".to_string(),
                task_id: "task-closed-direct".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "task-closed-direct",
                    "run_id": "run-closed-direct",
                    "task_status": "open",
                    "issue_type": "task"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "stale direct task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T10:30:00Z".to_string(),
            })
            .await
            .expect("record closed direct binding");

        let binding = store
            .run_graph_continuation_binding("run-closed-direct")
            .await
            .expect("read direct binding");

        assert!(
            binding.is_none(),
            "closed task_graph_task binding must not remain active"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_explicit_run_graph_continuation_binding_includes_task_close_reconcile() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-explicit-run-graph-continuation-binding-task-close-reconcile-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closure".to_string(),
                task_id: "task-closure".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "downstream_dispatch_target",
                    "task_id": "task-closure",
                    "run_id": "run-closure",
                    "dispatch_target": "closure"
                }),
                binding_source: "task_close_reconcile".to_string(),
                why_this_unit: "task close rebound the run to closure".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T11:00:00Z".to_string(),
            })
            .await
            .expect("record task-close reconcile binding");

        let latest = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .expect("read latest explicit binding")
            .expect("task-close reconcile binding should be returned");

        assert_eq!(latest.run_id, "run-closure");
        assert_eq!(latest.binding_source, "task_close_reconcile");
        assert_eq!(
            latest.active_bounded_unit["dispatch_target"],
            serde_json::Value::String("closure".to_string())
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_keeps_task_close_reconcile_fail_closed_when_run_is_open()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-continuation-binding-normalizes-task-close-reconcile-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: "task-closed",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");
        let mut status = sample_run_graph_status();
        status.run_id = "run-closed".to_string();
        status.task_id = "task-closed".to_string();
        status.active_node = "analysis".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "analysis_active".to_string();
        status.policy_gate = "owned_scope_required".to_string();
        status.handoff_state = "awaiting_implementation_dispatch".to_string();
        status.resume_target = "dispatch.implementation_dispatch_ready".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("record open analysis run status");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-closed".to_string(),
                task_id: "task-closed".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": "task-closed",
                    "run_id": "run-closed",
                    "active_node": "implementer"
                }),
                binding_source: "task_close_reconcile".to_string(),
                why_this_unit: "stale task-close reconcile binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                request_text: Some("continue".to_string()),
                recorded_at: "2026-04-16T12:00:00Z".to_string(),
            })
            .await
            .expect("record stale task-close reconcile binding");

        let binding = store
            .run_graph_continuation_binding("run-closed")
            .await
            .expect("read normalized binding");
        assert!(
            binding.is_none(),
            "stale task-close reconcile binding should fail closed when the run remains open"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_round_trips_latest_record_for_run_id() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-round-trip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let older = sample_replay_lineage_receipt(
            "receipt-replay-lineage-1",
            "run-replay-lineage",
            "2026-04-20T10:00:00Z",
        );
        let newer = sample_replay_lineage_receipt(
            "receipt-replay-lineage-2",
            "run-replay-lineage",
            "2026-04-20T10:05:00Z",
        );

        store
            .record_run_graph_replay_lineage_receipt(&older)
            .await
            .expect("persist older replay lineage receipt");
        store
            .record_run_graph_replay_lineage_receipt(&newer)
            .await
            .expect("persist newer replay lineage receipt");

        let loaded = store
            .run_graph_replay_lineage_receipt("run-replay-lineage")
            .await
            .expect("load latest replay lineage receipt")
            .expect("replay lineage receipt should exist");
        assert_eq!(loaded.receipt_id, "receipt-replay-lineage-2");
        assert_eq!(loaded.run_id, "run-replay-lineage");
        assert_eq!(loaded.resolved_dispatch_target, "closure");
        assert_eq!(loaded.validation_outcome, "pass");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_replay_lineage_receipt_uses_latest_status_run_id() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-latest-replay-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-replay-lineage-older", "open"))
            .await
            .expect("seed older replay lineage task");
        store
            .persist_task_record(test_task_record("task-replay-lineage-latest", "open"))
            .await
            .expect("seed latest replay lineage task");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-replay-lineage-older".to_string();
        older_status.task_id = "task-replay-lineage-older".to_string();
        older_status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older run graph status");
        store
            .record_run_graph_replay_lineage_receipt(&sample_replay_lineage_receipt(
                "receipt-replay-lineage-older",
                "run-replay-lineage-older",
                "2026-04-20T10:00:00Z",
            ))
            .await
            .expect("persist older replay lineage receipt");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-replay-lineage-latest".to_string();
        latest_status.task_id = "task-replay-lineage-latest".to_string();
        latest_status.resume_target = "dispatch.verification_lane".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest run graph status");
        store
            .record_run_graph_replay_lineage_receipt(&sample_replay_lineage_receipt(
                "receipt-replay-lineage-latest",
                "run-replay-lineage-latest",
                "2026-04-20T10:10:00Z",
            ))
            .await
            .expect("persist latest replay lineage receipt");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest replay lineage receipt")
            .expect("latest replay lineage receipt should exist");
        assert_eq!(latest.run_id, "run-replay-lineage-latest");
        assert_eq!(latest.receipt_id, "receipt-replay-lineage-latest");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_fails_closed_on_persisted_empty_required_field() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-invalid-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-replay-lineage-invalid", "open"))
            .await
            .expect("seed invalid replay lineage task");

        let mut status = sample_run_graph_status();
        status.run_id = "run-replay-lineage-invalid".to_string();
        status.task_id = "task-replay-lineage-invalid".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<RunGraphReplayLineageReceipt> = store
            .db
            .upsert((
                "run_graph_replay_lineage_receipt",
                "receipt-replay-lineage-invalid",
            ))
            .content(RunGraphReplayLineageReceipt {
                resolved_task_id: String::new(),
                ..sample_replay_lineage_receipt(
                    "receipt-replay-lineage-invalid",
                    "run-replay-lineage-invalid",
                    "2026-04-20T10:15:00Z",
                )
            })
            .await
            .expect("persist invalid replay lineage receipt");

        let error = store
            .run_graph_replay_lineage_receipt("run-replay-lineage-invalid")
            .await
            .expect_err("invalid persisted replay lineage receipt should fail closed");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("run-graph replay lineage receipt"));
                assert!(reason.contains("resolved_task_id"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        let error = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect_err("latest replay lineage receipt should fail closed for invalid latest row");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("run-graph replay lineage receipt"));
                assert!(reason.contains("resolved_task_id"));
            }
            other => panic!("expected InvalidTaskRecord, got {other:?}"),
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn run_graph_replay_lineage_receipt_round_trips_for_latest_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-round-trip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-replay-lineage", "open"))
            .await
            .expect("seed replay lineage task");

        let mut status = sample_run_graph_status();
        status.run_id = "run-replay-lineage".to_string();
        status.task_id = "task-replay-lineage".to_string();
        status.resume_target = "resume.current_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = RunGraphReplayLineageReceipt {
            receipt_id: "replay-lineage-run-replay-lineage".to_string(),
            run_id: "run-replay-lineage".to_string(),
            lineage_kind: "root_dispatch_packet".to_string(),
            replay_scope: "resume_resolution".to_string(),
            origin_checkpoint_ref: "run-replay-lineage:execution_cursor:resume.current_lane"
                .to_string(),
            fork_parent: None,
            source_dispatch_target: "implementer".to_string(),
            source_dispatch_packet_path: Some("/tmp/run-replay-lineage-packet.json".to_string()),
            source_dispatch_result_path: Some("/tmp/run-replay-lineage-result.json".to_string()),
            resolved_dispatch_target: "implementer".to_string(),
            resolved_task_id: "task-replay-lineage".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "resume.current_lane".to_string(),
            validation_outcome: "lawful_resume".to_string(),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_replay_lineage_receipt(&receipt)
            .await
            .expect("persist replay lineage receipt");

        let by_run = store
            .run_graph_replay_lineage_receipt("run-replay-lineage")
            .await
            .expect("load replay lineage receipt by run")
            .expect("receipt exists");
        assert_eq!(by_run.receipt_id, receipt.receipt_id);
        assert_eq!(by_run.origin_checkpoint_ref, receipt.origin_checkpoint_ref);
        assert_eq!(by_run.resolved_task_id, "task-replay-lineage");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest replay lineage receipt")
            .expect("latest receipt exists");
        assert_eq!(latest.run_id, "run-replay-lineage");
        assert_eq!(latest.lineage_kind, "root_dispatch_packet");
        assert_eq!(latest.resume_target, "resume.current_lane");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_replay_lineage_receipt_ignores_older_run_receipts() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-replay-lineage-latest-scope-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-replay-old".to_string();
        older_status.task_id = "task-replay-old".to_string();
        older_status.resume_target = "resume.old".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older status");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-replay-new".to_string();
        latest_status.task_id = "task-replay-new".to_string();
        latest_status.resume_target = "resume.new".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let older_receipt = RunGraphReplayLineageReceipt {
            receipt_id: "replay-lineage-run-replay-old".to_string(),
            run_id: "run-replay-old".to_string(),
            lineage_kind: "downstream_packet".to_string(),
            replay_scope: "resume_resolution".to_string(),
            origin_checkpoint_ref: "run-replay-old:execution_cursor:resume.old".to_string(),
            fork_parent: None,
            source_dispatch_target: "implementer".to_string(),
            source_dispatch_packet_path: Some("/tmp/run-replay-old-packet.json".to_string()),
            source_dispatch_result_path: None,
            resolved_dispatch_target: "coach".to_string(),
            resolved_task_id: "task-replay-old".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "resume.old".to_string(),
            validation_outcome: "lawful_resume".to_string(),
            recorded_at: "2026-04-20T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_replay_lineage_receipt(&older_receipt)
            .await
            .expect("persist older replay lineage receipt");

        let latest = store
            .latest_run_graph_replay_lineage_receipt()
            .await
            .expect("load latest scoped replay lineage receipt");
        assert!(
            latest.is_none(),
            "latest run should not inherit replay lineage receipt from older run"
        );

        let older = store
            .run_graph_replay_lineage_receipt("run-replay-old")
            .await
            .expect("load older replay lineage receipt")
            .expect("older receipt exists");
        assert_eq!(older.receipt_id, "replay-lineage-run-replay-old");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn record_run_graph_status_appends_projection_checkpoint_records_for_same_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-history-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-projection-checkpoint".to_string();
        status.task_id = "task-projection-checkpoint".to_string();
        status.resume_target = "dispatch.writer_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist first status");

        status.active_node = "coach".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist second status");

        let mut query = store
            .db
            .query(
                "SELECT * FROM run_graph_projection_checkpoint_record \
                 WHERE run_id = $run_id \
                 ORDER BY updated_at ASC, record_id ASC;",
            )
            .bind(("run_id", "run-projection-checkpoint".to_string()))
            .await
            .expect("load projection checkpoint history");
        let rows: Vec<RunGraphProjectionCheckpointRecord> =
            query.take(0).expect("decode projection checkpoint rows");
        assert_eq!(
            rows.len(),
            2,
            "status writes must append checkpoint records"
        );
        assert_ne!(rows[0].record_id, rows[1].record_id);
        assert_eq!(rows[0].projector_id, "taskflow.run_graph.status_projection");
        assert_eq!(
            rows[1].checkpoint_group,
            "run_graph_status:run-projection-checkpoint"
        );
        assert_eq!(rows[1].last_gapless_position, rows[1].updated_at);
        assert_eq!(rows[1].lineage_kind, "live_status_projection");

        let latest = store
            .run_graph_projection_checkpoint_record("run-projection-checkpoint")
            .await
            .expect("load latest projection checkpoint record")
            .expect("latest projection checkpoint record exists");
        assert_eq!(latest.last_gapless_position, latest.updated_at);
        assert_eq!(
            latest.origin_checkpoint_ref,
            "run-projection-checkpoint:execution_cursor:dispatch.coach_lane"
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn latest_run_graph_projection_checkpoint_record_scopes_to_latest_status_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("task-projection-old", "open"))
            .await
            .expect("seed older projection task");
        store
            .persist_task_record(test_task_record("task-projection-new", "open"))
            .await
            .expect("seed latest projection task");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-projection-old".to_string();
        older_status.task_id = "task-projection-old".to_string();
        older_status.resume_target = "dispatch.old".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older status");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-projection-new".to_string();
        latest_status.task_id = "task-projection-new".to_string();
        latest_status.resume_target = "dispatch.new".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let latest = store
            .latest_run_graph_projection_checkpoint_record()
            .await
            .expect("load latest projection checkpoint record")
            .expect("latest projection checkpoint record exists");
        assert_eq!(latest.run_id, "run-projection-new");
        assert_eq!(latest.last_gapless_position, latest.updated_at);
        assert_eq!(latest.projector_id, "taskflow.run_graph.status_projection");

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn record_run_graph_status_skips_projection_checkpoint_record_when_checkpoint_kind_is_none()
     {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-checkpoint-skip-none-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-projection-none".to_string();
        status.task_id = "task-projection-none".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist status without checkpoint lineage");

        let latest = store
            .run_graph_projection_checkpoint_record("run-projection-none")
            .await
            .expect("load projection checkpoint record");
        assert!(
            latest.is_none(),
            "checkpoint placeholder state must not emit a projection checkpoint record"
        );

        close_store_and_remove_root(store, root).await;
    }
}
