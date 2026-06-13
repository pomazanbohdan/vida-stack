use operator_output::next_actions;

pub(crate) fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

pub(crate) fn consume_continue_command(run_id: Option<&str>) -> String {
    next_actions::consume_continue_command(run_id)
}

pub(crate) fn recovery_latest_command() -> String {
    next_actions::recovery_latest_command()
}

pub(crate) fn open_delegated_cycle_continue_next_action(run_id: Option<&str>) -> String {
    let command = consume_continue_command(run_id);
    format!(
        "Continue the active bound run with `{command}` before considering backlog ready-head work."
    )
}

pub(crate) fn run_graph_latest_snapshot_inconsistent_next_action() -> String {
    let status_command = next_actions::status_command();
    let continue_command = next_actions::consume_continue_command(None);
    format!(
        "Inspect the concrete run/task/packet named by `{status_command}`; if the task or owned_paths are missing, repair or retire that stale run first. Only rerun `{continue_command}` after status, recovery, checkpoint, gate, and dispatch receipt can share one authoritative run_id, then recheck `{status_command}`."
    )
}

pub(crate) fn run_graph_latest_dispatch_receipt_signal_ambiguous_next_action() -> String {
    let continue_command = next_actions::consume_continue_command(None);
    format!(
        "Rebuild the latest run-graph dispatch receipt with `{continue_command}` so lane_status and dispatch_status are canonical and aligned before trusting the operator signal."
    )
}

pub(crate) fn continuation_binding_ambiguous_next_action() -> String {
    let status_command = next_actions::status_command();
    "Do not continue by heuristic. Inspect `".to_string()
        + &status_command
        + "`, then inspect the authoritative run with `vida taskflow run-graph status` using that concrete `run_id`; if user intent already names the next bounded unit, bind it explicitly with `vida taskflow continuation bind` using the cited `task_id` and `run_id` before further implementation."
}

pub(crate) fn blocked_run_graph_status_next_actions(
    run_id: Option<&str>,
    task_id: Option<&str>,
    task_closed: bool,
) -> Vec<String> {
    let run_id = run_id.map(str::trim).filter(|value| !value.is_empty());
    let task_id = task_id.map(str::trim).filter(|value| !value.is_empty());
    match (run_id, task_id, task_closed) {
        (Some(run_id), Some(task_id), true) => vec![
            format!(
                "Inspect the blocked run-graph status with `{}` before writing.",
                next_actions::human_recovery_status_command(run_id)
            ),
            format!(
                "Run `{task_id}` is already closed for blocked run `{run_id}`; retire that stale blocked run with `{}`, then refresh continuation evidence with `{}` before selecting the next bounded step.",
                next_actions::human_lane_retire_command(run_id),
                next_actions::consume_continue_command(None)
            ),
        ],
        (Some(run_id), _, false) => vec![
            format!(
                "Inspect the blocked run-graph status with `{}` and resolve the blocker before writing.",
                next_actions::human_recovery_status_command(run_id)
            ),
            format!(
                "After the blocker is resolved, refresh continuation evidence with `{}` or bind the next bounded unit explicitly.",
                next_actions::consume_continue_command(None)
            ),
        ],
        _ => vec![
            "Do not continue normal delivery while the latest run-graph status is blocked."
                .to_string(),
            continuation_binding_ambiguous_next_action().to_string(),
        ],
    }
}

pub(crate) fn runtime_binding_task_missing_next_action(
    run_id: Option<&str>,
    task_id: &str,
) -> String {
    let task_id = task_id.trim();
    match run_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .zip((!task_id.is_empty()).then_some(task_id))
    {
        Some((run_id, task_id)) => {
            let recovery_command = next_actions::human_recovery_status_command(run_id);
            format!(
                "Runtime binding points to missing task `{task_id}` for run `{run_id}`. Inspect the concrete recovery state with `{recovery_command}`; only bind a new explicit task after the run reaches closure_complete, otherwise reconcile the recovery blocker or retire the stale run before continuing."
            )
        }
        None => continuation_binding_ambiguous_next_action().to_string(),
    }
}

pub(crate) fn recovery_resume_target_missing_next_action(
    run_id: Option<&str>,
    task_id: Option<&str>,
) -> String {
    let run_id = run_id.map(str::trim).filter(|value| !value.is_empty());
    let task_id = task_id.map(str::trim).filter(|value| !value.is_empty());
    match (run_id, task_id) {
        (Some(run_id), Some(task_id)) => {
            let recovery_command = next_actions::human_recovery_status_command(run_id);
            format!(
                "Recovery for run `{run_id}` has no dispatch resume_target for task `{task_id}`. Inspect `{recovery_command}`; if the run is still inside an open delegated cycle, resolve that blocker through lane recovery before any explicit task bind. Only bind a new explicit task after closure_complete."
            )
        }
        (Some(run_id), None) => {
            let recovery_command = next_actions::human_recovery_status_command(run_id);
            format!(
                "Recovery for run `{run_id}` has no dispatch resume_target. Inspect `{recovery_command}`; if the run is still inside an open delegated cycle, resolve that blocker through lane recovery before any explicit task bind. Only bind a new explicit task after closure_complete."
            )
        }
        _ => continuation_binding_ambiguous_next_action().to_string(),
    }
}

pub(crate) fn terminal_next_action_requires_authoritative_run_state(
    run_id: Option<&str>,
) -> String {
    match run_id.filter(|value| !value.trim().is_empty()) {
        Some(run_id) => {
            let run_graph_command = next_actions::human_run_graph_status_command(run_id.trim());
            format!(
                "Do not continue by heuristic. First inspect the authoritative run state with `{run_graph_command}`, then either cite the explicit next bounded unit from the user and bind it with `vida taskflow continuation bind` using that concrete `run_id` and `task_id`, or stop and reconcile why the authoritative run state still lacks the next bounded unit before further implementation."
            )
        }
        None => {
            let status_command = next_actions::status_command();
            "Do not continue by heuristic. First inspect the authoritative run state with `"
                .to_string()
                + &status_command
                + "`, then inspect the authoritative run with `vida taskflow run-graph status` using that concrete `run_id`; if user intent already names the next bounded unit, bind it explicitly with `vida taskflow continuation bind` using the cited `task_id` and `run_id` before further implementation."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        blocked_run_graph_status_next_actions, consume_continue_command,
        open_delegated_cycle_continue_next_action, recovery_latest_command,
        recovery_resume_target_missing_next_action, runtime_binding_task_missing_next_action,
        terminal_next_action_requires_authoritative_run_state,
    };

    #[test]
    fn string_runtime_status_signals_use_default_human_commands() {
        let actions = blocked_run_graph_status_next_actions(Some("run-1"), Some("task-1"), true);
        assert!(actions.iter().all(|action| !action.contains("--json")));
        assert!(actions
            .iter()
            .any(|action| action.contains("vida lane retire run-1")));

        let missing = runtime_binding_task_missing_next_action(Some("run-2"), "task-2");
        assert!(missing.contains("vida taskflow recovery status run-2"));
        assert!(!missing.contains("--json"));

        let recovery = recovery_resume_target_missing_next_action(Some("run-3"), Some("task-3"));
        assert!(recovery.contains("vida taskflow recovery status run-3"));
        assert!(!recovery.contains("--json"));

        let terminal = terminal_next_action_requires_authoritative_run_state(Some("run-4"));
        assert!(terminal.contains("vida taskflow run-graph status run-4"));
        assert!(!terminal.contains("--json"));

        assert_eq!(
            consume_continue_command(Some("run-5")),
            "vida taskflow consume continue --run-id run-5"
        );
        assert_eq!(
            open_delegated_cycle_continue_next_action(Some("run-6")),
            "Continue the active bound run with `vida taskflow consume continue --run-id run-6` before considering backlog ready-head work."
        );
        assert_eq!(recovery_latest_command(), "vida taskflow recovery latest");
    }

    #[test]
    fn catalog_runtime_status_signals_use_default_human_commands() {
        let actions = vec![
            super::run_graph_latest_snapshot_inconsistent_next_action(),
            super::run_graph_latest_dispatch_receipt_signal_ambiguous_next_action(),
            super::continuation_binding_ambiguous_next_action(),
            super::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action(),
            super::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action(),
            super::protocol_binding_check_next_action(),
            super::project_activation_next_action(),
            super::project_activation_unknown_next_action(),
            super::missing_run_graph_dispatch_receipt_operator_evidence_next_action(),
            super::closed_task_active_run_projection_mismatch_next_action(),
            super::missing_root_session_write_guard_next_action(),
            super::recovery_readiness_blocked_next_action(),
            super::task_validate_graph_next_action(),
            super::missing_retrieval_trust_source_operator_evidence_next_action(),
            super::missing_retrieval_trust_signal_operator_evidence_next_action(),
            super::missing_retrieval_trust_operator_evidence_next_action(),
        ];

        for action in actions {
            assert!(
                !action.contains("--json"),
                "human next-action must not force JSON-first default: {action}"
            );
        }
    }
}

pub(crate) fn run_graph_latest_dispatch_receipt_summary_inconsistent_next_action() -> String {
    let status_command = next_actions::status_command();
    let recovery_command = next_actions::recovery_latest_command();
    format!(
        "Run `{status_command}` to refresh the latest run-graph dispatch receipt summary, then inspect `{recovery_command}`; rerun the blocked TaskFlow command only after latest status and dispatch receipt share the same concrete run_id."
    )
}

pub(crate) fn run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action() -> String {
    let status_command = next_actions::status_command();
    format!(
        "Refresh the latest checkpoint evidence for the run graph before rerunning `{status_command}` so checkpoint rows and dispatch receipt evidence share the same run_id."
    )
}

pub(crate) fn protocol_binding_check_next_action() -> String {
    let command = next_actions::human_taskflow_protocol_binding_check_command();
    format!("Run `{command}` and clear blockers.")
}

pub(crate) fn project_activation_next_action() -> String {
    let command = next_actions::human_project_activator_command();
    format!("Complete project activation via `{command}` before normal work.")
}

pub(crate) fn project_activation_unknown_next_action() -> String {
    let command = next_actions::human_project_activator_command();
    format!(
        "Resolve project root detection and run `{command}` to surface canonical activation state."
    )
}

pub(crate) fn missing_run_graph_dispatch_receipt_operator_evidence_next_action() -> String {
    let command = next_actions::consume_continue_command(None);
    format!(
        "Run `{command}` to materialize or refresh run-graph dispatch receipt evidence before operator handoff."
    )
}

pub(crate) fn closed_task_active_run_projection_mismatch_next_action() -> String {
    let reconcile_command = next_actions::human_closed_run_reconcile_command();
    let inspect_command = next_actions::human_run_graph_status_command("<run-id>");
    format!(
        "Run `{reconcile_command}` and inspect skipped runs with `{inspect_command}`; closed tasks must not remain projected as active runtime work."
    )
}

pub(crate) fn missing_root_session_write_guard_next_action() -> String {
    let recovery_command = next_actions::recovery_latest_command();
    let continue_command = next_actions::consume_continue_command(None);
    format!(
        "Run `{recovery_command}` and `{continue_command}` to confirm runtime artifacts expose the canonical root-session pre-write guard."
    )
}

pub(crate) fn recovery_readiness_blocked_next_action() -> String {
    let recovery_command = next_actions::recovery_latest_command();
    let continue_command = next_actions::consume_continue_command(None);
    format!(
        "Inspect `{recovery_command}`, then run `{continue_command}` after `recovery_ready=true` is proven for resume/rollback handoff."
    )
}

pub(crate) fn task_validate_graph_next_action() -> String {
    let command = next_actions::human_taskflow_graph_summary_command();
    format!("Run `{command}` and resolve graph issues.")
}

pub(crate) fn missing_retrieval_trust_source_operator_evidence_next_action() -> String {
    let command = next_actions::human_bundle_check_command();
    format!(
        "Run `{command}` so runtime consumption snapshots publish retrieval-trust source evidence."
    )
}

pub(crate) fn missing_retrieval_trust_signal_operator_evidence_next_action() -> String {
    let sync_command = next_actions::human_taskflow_protocol_binding_sync_command();
    let bundle_command = next_actions::human_bundle_check_command();
    format!(
        "Run `{sync_command}` and `{bundle_command}` to materialize retrieval-trust citation/freshness/ACL signal."
    )
}

pub(crate) fn missing_retrieval_trust_operator_evidence_next_action() -> String {
    let command = next_actions::human_bundle_check_command();
    format!("Run `{command}` to record retrieval-trust operator evidence.")
}

pub(crate) fn final_snapshot_missing_release_admission_evidence(snapshot_path: &str) -> bool {
    let payload = match std::fs::read_to_string(snapshot_path) {
        Ok(payload) => payload,
        Err(_) => return true,
    };
    let summary_json = match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(json) => json,
        Err(_) => return true,
    };
    if crate::operator_contracts::shared_operator_output_contract_parity_error(&summary_json)
        .is_some()
    {
        return true;
    }
    !crate::runtime_consumption_snapshot_has_release_admission_evidence(&summary_json)
}
