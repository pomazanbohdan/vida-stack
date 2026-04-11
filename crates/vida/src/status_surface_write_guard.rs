use std::path::{Path, PathBuf};

use crate::release1_contracts::{blocker_code_str, BlockerCode};

fn looks_like_runtime_root_session_write_guard_candidate(value: &serde_json::Value) -> bool {
    matches!(
        value["status"].as_str(),
        Some("blocked_by_default" | "exception_takeover_active")
    ) && value["root_session_role"]
        .as_str()
        .is_some_and(|role| !role.trim().is_empty())
}

fn has_runtime_root_session_write_guard(value: &serde_json::Value) -> bool {
    looks_like_runtime_root_session_write_guard_candidate(value)
        && value["local_write_requires_exception_path"].is_boolean()
        && value["root_local_write_allowed"].is_boolean()
        && value["required_exception_evidence"]
            .as_str()
            .is_some_and(|evidence| !evidence.trim().is_empty())
        && value["pre_write_checkpoint_required"].is_boolean()
}

fn canonical_root_session_write_guard_defaults() -> serde_json::Value {
    serde_json::json!({
        "status": "blocked_by_default",
        "root_session_role": "orchestrator",
        "lawful_write_surface": "vida agent-init",
        "explicit_user_ordered_agent_mode_is_sticky": true,
        "saturation_recovery_required_before_local_fallback": true,
        "local_fallback_without_lane_recovery_forbidden": true,
        "host_local_write_capability_is_not_authority": true,
        "local_write_requires_exception_path": true,
        "root_local_write_allowed": false,
        "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
        "pre_write_checkpoint_required": true,
    })
}

fn has_nonempty_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn exception_takeover_state_label(
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> Option<&'static str> {
    let Some(receipt) = latest_receipt else {
        return None;
    };
    if !has_nonempty_value(receipt.exception_path_receipt_id.as_deref()) {
        return None;
    }
    let takeover_state = crate::release1_contracts::exception_takeover_state(
        receipt.exception_path_receipt_id.as_deref(),
        receipt.supersedes_receipt_id.as_deref(),
        latest_recovery.map(|recovery| {
            recovery
                .delegation_gate
                .local_exception_takeover_gate
                .as_str()
        }),
    );
    if receipt.lane_status == "lane_exception_takeover" && takeover_state.is_active() {
        return Some("active");
    }
    if takeover_state.is_active() {
        return Some("admissible_not_active");
    }
    Some("receipt_recorded")
}

fn exception_takeover_is_lawfully_active(
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    exception_takeover_state_label(latest_receipt, latest_recovery) == Some("active")
}

pub(crate) fn root_session_write_guard_summary_from_snapshot_path(
    snapshot_path: Option<&str>,
) -> serde_json::Value {
    let Some(path) = snapshot_path else {
        return serde_json::json!({
            "status": "missing",
            "reason": "runtime_consumption_snapshot_missing",
            "lawful_write_surface": serde_json::Value::Null,
            "host_local_write_capability_is_not_authority": serde_json::Value::Null,
            "local_write_requires_exception_path": serde_json::Value::Null,
            "required_exception_evidence": serde_json::Value::Null,
        });
    };
    let snapshot = crate::read_json_file_if_present(Path::new(path));
    let Some(snapshot) = snapshot else {
        return serde_json::json!({
            "status": "missing",
            "reason": "runtime_consumption_snapshot_unreadable",
            "lawful_write_surface": serde_json::Value::Null,
            "host_local_write_capability_is_not_authority": serde_json::Value::Null,
            "local_write_requires_exception_path": serde_json::Value::Null,
            "required_exception_evidence": serde_json::Value::Null,
        });
    };
    let mut guard = runtime_root_session_write_guard_from_snapshot(&snapshot);
    if guard.is_none() {
        if let Some(path) = latest_final_runtime_snapshot_path(Path::new(path)) {
            if let Some(fallback_snapshot) = crate::read_json_file_if_present(&path) {
                guard = runtime_root_session_write_guard_from_snapshot(&fallback_snapshot);
            }
        }
    }
    let mut guard = guard.unwrap_or(serde_json::Value::Null);
    if guard["status"].as_str() == Some("blocked_by_default") {
        let defaults = canonical_root_session_write_guard_defaults();
        if let (Some(guard_obj), Some(defaults_obj)) = (guard.as_object_mut(), defaults.as_object())
        {
            for (key, value) in defaults_obj {
                let missing = guard_obj.get(key).is_none_or(|existing| existing.is_null());
                if missing {
                    guard_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
    let guard_ok = has_runtime_root_session_write_guard(&guard);
    let blocking_dispatch_blocker_code =
        latest_dispatch_blocker_code_from_snapshot_path(snapshot_path.map(Path::new))
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
    let activation_view_only_dispatch_blocker_active =
        blocking_dispatch_blocker_code.as_str() == Some("internal_activation_view_only");
    serde_json::json!({
        "status": if guard_ok { "blocked_by_default" } else { "missing" },
        "reason": if guard_ok {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(
                blocker_code_str(BlockerCode::MissingRootSessionWriteGuard).to_string()
            )
        },
        "root_session_role": guard["root_session_role"].clone(),
        "lawful_write_surface": guard["lawful_write_surface"].clone(),
        "explicit_user_ordered_agent_mode_is_sticky": guard["explicit_user_ordered_agent_mode_is_sticky"].clone(),
        "saturation_recovery_required_before_local_fallback": guard["saturation_recovery_required_before_local_fallback"].clone(),
        "local_fallback_without_lane_recovery_forbidden": guard["local_fallback_without_lane_recovery_forbidden"].clone(),
        "host_local_write_capability_is_not_authority": guard["host_local_write_capability_is_not_authority"].clone(),
        "local_write_requires_exception_path": guard["local_write_requires_exception_path"].clone(),
        "root_local_write_allowed": guard["root_local_write_allowed"].clone(),
        "required_exception_evidence": guard["required_exception_evidence"].clone(),
        "pre_write_checkpoint_required": guard["pre_write_checkpoint_required"].clone(),
        "blocking_dispatch_blocker_code": blocking_dispatch_blocker_code,
        "activation_view_only_dispatch_blocker_active": activation_view_only_dispatch_blocker_active,
    })
}

pub(crate) fn merge_live_exception_takeover_write_guard(
    mut guard: serde_json::Value,
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> serde_json::Value {
    let Some(guard_obj) = guard.as_object_mut() else {
        return guard;
    };
    guard_obj.insert(
        "exception_path_receipt_id".to_string(),
        latest_receipt
            .and_then(|receipt| receipt.exception_path_receipt_id.clone())
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    guard_obj.insert(
        "latest_lane_status".to_string(),
        latest_receipt
            .map(|receipt| serde_json::Value::String(receipt.lane_status.clone()))
            .unwrap_or(serde_json::Value::Null),
    );
    guard_obj.insert(
        "local_exception_takeover_gate".to_string(),
        latest_recovery
            .map(|recovery| {
                serde_json::Value::String(
                    recovery
                        .delegation_gate
                        .local_exception_takeover_gate
                        .clone(),
                )
            })
            .unwrap_or(serde_json::Value::Null),
    );
    guard_obj.insert(
        "latest_dispatch_blocker_code".to_string(),
        latest_receipt
            .and_then(|receipt| receipt.blocker_code.clone())
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    guard_obj.insert(
        "local_exception_takeover_state".to_string(),
        exception_takeover_state_label(latest_receipt, latest_recovery)
            .map(|state| serde_json::Value::String(state.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    if exception_takeover_is_lawfully_active(latest_receipt, latest_recovery) {
        guard_obj.insert(
            "status".to_string(),
            serde_json::Value::String("exception_takeover_active".to_string()),
        );
        guard_obj.insert(
            "root_local_write_allowed".to_string(),
            serde_json::Value::Bool(true),
        );
        if let Some(receipt_id) =
            latest_receipt.and_then(|receipt| receipt.exception_path_receipt_id.as_deref())
        {
            guard_obj.insert(
                "required_exception_evidence".to_string(),
                serde_json::Value::String(receipt_id.to_string()),
            );
        }
    }
    guard
}

fn runtime_root_session_write_guard_from_snapshot(
    snapshot: &serde_json::Value,
) -> Option<serde_json::Value> {
    let direct_guard =
        &snapshot["payload"]["role_selection"]["execution_plan"]["root_session_write_guard"];
    if looks_like_runtime_root_session_write_guard_candidate(direct_guard) {
        return Some(direct_guard.clone());
    }
    let execution_plan_contract_guard = &snapshot["payload"]["role_selection"]["execution_plan"]
        ["orchestration_contract"]["root_session_write_guard"];
    if looks_like_runtime_root_session_write_guard_candidate(execution_plan_contract_guard) {
        return Some(execution_plan_contract_guard.clone());
    }

    let dispatch_packet_path = snapshot["source_dispatch_packet_path"]
        .as_str()
        .or_else(|| snapshot["dispatch_receipt"]["dispatch_packet_path"].as_str())?;
    let packet = crate::read_json_file_if_present(Path::new(dispatch_packet_path))?;
    let packet_guard = &packet["root_session_write_guard"];
    if looks_like_runtime_root_session_write_guard_candidate(packet_guard) {
        return Some(packet_guard.clone());
    }
    let packet_contract_guard = &packet["orchestration_contract"]["root_session_write_guard"];
    if looks_like_runtime_root_session_write_guard_candidate(packet_contract_guard) {
        return Some(packet_contract_guard.clone());
    }

    None
}

fn latest_final_runtime_snapshot_path(latest_snapshot_path: &Path) -> Option<PathBuf> {
    let snapshot_dir = latest_snapshot_path.parent()?;
    let mut latest_final: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(snapshot_dir).ok()?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("final-") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &latest_final {
            Some((latest_modified, _)) if modified <= *latest_modified => {}
            _ => latest_final = Some((modified, path)),
        }
    }
    latest_final.map(|(_, path)| path)
}

fn latest_dispatch_blocker_code_from_snapshot_path(snapshot_path: Option<&Path>) -> Option<String> {
    let snapshot_path = snapshot_path?;
    let snapshot = crate::read_json_file_if_present(snapshot_path)?;
    latest_dispatch_blocker_code_from_snapshot(&snapshot).or_else(|| {
        latest_final_runtime_snapshot_path(snapshot_path).and_then(|path| {
            crate::read_json_file_if_present(&path)
                .and_then(|snapshot| latest_dispatch_blocker_code_from_snapshot(&snapshot))
        })
    })
}

fn latest_dispatch_blocker_code_from_snapshot(snapshot: &serde_json::Value) -> Option<String> {
    snapshot["dispatch_receipt"]["blocker_code"]
        .as_str()
        .or_else(|| snapshot["latest_run_graph_dispatch_receipt"]["blocker_code"].as_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_receipt() -> crate::state_store::RunGraphDispatchReceiptSummary {
        crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("receipt-1".to_string()),
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida lane exception-takeover".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
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
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-09T00:00:00Z".to_string(),
        }
    }

    fn sample_recovery(local_gate: &str) -> crate::state_store::RunGraphRecoverySummary {
        crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            active_node: "spec-pack".to_string(),
            lifecycle_stage: "spec_pack_active".to_string(),
            resume_node: None,
            resume_status: "ready".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "spec-pack".to_string(),
                lifecycle_stage: "spec_pack_active".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: local_gate.to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        }
    }

    #[test]
    fn merge_live_exception_takeover_write_guard_keeps_recorded_receipts_blocked() {
        let guard = canonical_root_session_write_guard_defaults();
        let merged = merge_live_exception_takeover_write_guard(
            guard,
            Some(&sample_receipt()),
            Some(&sample_recovery("blocked_open_delegated_cycle")),
        );

        assert_eq!(merged["status"], "blocked_by_default");
        assert_eq!(merged["root_local_write_allowed"], false);
        assert_eq!(merged["local_exception_takeover_state"], "receipt_recorded");
    }

    #[test]
    fn merge_live_exception_takeover_write_guard_exposes_admissible_but_inactive_state() {
        let guard = canonical_root_session_write_guard_defaults();
        let merged = merge_live_exception_takeover_write_guard(
            guard,
            Some(&sample_receipt()),
            Some(&sample_recovery("delegated_cycle_clear")),
        );

        assert_eq!(merged["status"], "blocked_by_default");
        assert_eq!(merged["root_local_write_allowed"], false);
        assert_eq!(
            merged["local_exception_takeover_state"],
            "admissible_not_active"
        );
    }

    #[test]
    fn merge_live_exception_takeover_write_guard_marks_explicit_takeover_active() {
        let guard = canonical_root_session_write_guard_defaults();
        let mut receipt = sample_receipt();
        receipt.lane_status = "lane_exception_takeover".to_string();

        let merged = merge_live_exception_takeover_write_guard(
            guard,
            Some(&receipt),
            Some(&sample_recovery("delegated_cycle_clear")),
        );

        assert_eq!(merged["status"], "exception_takeover_active");
        assert_eq!(merged["root_local_write_allowed"], true);
        assert_eq!(merged["required_exception_evidence"], "receipt-1");
        assert_eq!(merged["local_exception_takeover_state"], "active");
    }
}
