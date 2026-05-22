pub(crate) fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

pub(crate) fn run_graph_latest_snapshot_inconsistent_next_action() -> &'static str {
    "Rebuild the latest run-graph evidence by rerunning `vida taskflow consume continue --json` and then recheck `vida status --json` once status, recovery, checkpoint, gate, and dispatch receipt share the same run_id."
}

pub(crate) fn run_graph_latest_dispatch_receipt_signal_ambiguous_next_action() -> &'static str {
    "Rebuild the latest run-graph dispatch receipt with `vida taskflow consume continue --json` so lane_status and dispatch_status are canonical and aligned before trusting the operator signal."
}

pub(crate) fn continuation_binding_ambiguous_next_action() -> &'static str {
    "Do not continue by heuristic. Inspect `vida status --json`, then inspect the authoritative run with `vida taskflow run-graph status` using that concrete `run_id`; if user intent already names the next bounded unit, bind it explicitly with `vida taskflow continuation bind` using the cited `task_id` and `run_id` before further implementation."
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
                "Inspect the blocked run-graph status with `vida taskflow recovery status {run_id} --json` before writing."
            ),
            format!(
                "Run `{task_id}` is already closed for blocked run `{run_id}`; retire that stale blocked run with `vida lane retire {run_id} --receipt-id <concrete-receipt-id> --reason <reason> --json`, then refresh continuation evidence with `vida taskflow consume continue --json` before selecting the next bounded step."
            ),
        ],
        (Some(run_id), _, false) => vec![
            format!(
                "Inspect the blocked run-graph status with `vida taskflow recovery status {run_id} --json` and resolve the blocker before writing."
            ),
            "After the blocker is resolved, refresh continuation evidence with `vida taskflow consume continue --json` or bind the next bounded unit explicitly.".to_string(),
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
        Some((run_id, task_id)) => format!(
            "Runtime binding points to missing task `{task_id}` for run `{run_id}`. Inspect the concrete recovery state with `vida taskflow recovery status {run_id} --json`; only bind a new explicit task after the run reaches closure_complete, otherwise reconcile the recovery blocker or retire the stale run before continuing."
        ),
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
        (Some(run_id), Some(task_id)) => format!(
            "Recovery for run `{run_id}` has no dispatch resume_target for task `{task_id}`. Inspect `vida taskflow recovery status {run_id} --json`; if the run is still inside an open delegated cycle, resolve that blocker through lane recovery before any explicit task bind. Only bind a new explicit task after closure_complete."
        ),
        (Some(run_id), None) => format!(
            "Recovery for run `{run_id}` has no dispatch resume_target. Inspect `vida taskflow recovery status {run_id} --json`; if the run is still inside an open delegated cycle, resolve that blocker through lane recovery before any explicit task bind. Only bind a new explicit task after closure_complete."
        ),
        _ => continuation_binding_ambiguous_next_action().to_string(),
    }
}

pub(crate) fn terminal_next_action_requires_authoritative_run_state(
    run_id: Option<&str>,
) -> String {
    match run_id.filter(|value| !value.trim().is_empty()) {
        Some(run_id) => format!(
            "Do not continue by heuristic. First inspect the authoritative run state with `vida taskflow run-graph status {run_id} --json`, then either cite the explicit next bounded unit from the user and bind it with `vida taskflow continuation bind` using that concrete `run_id` and `task_id`, or stop and reconcile why the authoritative run state still lacks the next bounded unit before further implementation."
        , run_id = crate::shell_quote(run_id.trim())),
        None => "Do not continue by heuristic. First inspect the authoritative run state with `vida status --json`, then inspect the authoritative run with `vida taskflow run-graph status` using that concrete `run_id`; if user intent already names the next bounded unit, bind it explicitly with `vida taskflow continuation bind` using the cited `task_id` and `run_id` before further implementation.".to_string(),
    }
}

pub(crate) fn run_graph_latest_dispatch_receipt_summary_inconsistent_next_action() -> &'static str {
    "Run `vida status --json` to refresh the latest run-graph dispatch receipt summary, then inspect `vida taskflow recovery latest --json`; rerun the blocked TaskFlow command only after latest status and dispatch receipt share the same concrete run_id."
}

pub(crate) fn run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action() -> &'static str {
    "Refresh the latest checkpoint evidence for the run graph before rerunning `vida status --json` so checkpoint rows and dispatch receipt evidence share the same run_id."
}

pub(crate) const MISSING_RETRIEVAL_TRUST_SOURCE_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "Run `vida taskflow consume bundle check --json` so runtime consumption snapshots publish retrieval-trust source evidence.";
pub(crate) const MISSING_RETRIEVAL_TRUST_SIGNAL_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "Run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle check --json` to materialize retrieval-trust citation/freshness/ACL signal.";
pub(crate) const MISSING_RETRIEVAL_TRUST_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "Run `vida taskflow consume bundle check --json` to record retrieval-trust operator evidence.";

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
