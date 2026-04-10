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
    "Do not continue by heuristic. Either bind the bounded unit explicitly from user intent or refresh runtime evidence with `vida taskflow consume continue --json` and recheck `vida status --json` before further implementation."
}

pub(crate) fn run_graph_latest_dispatch_receipt_summary_inconsistent_next_action() -> &'static str {
    "Refresh the latest run-graph dispatch receipt summary before rerunning `vida status --json` so the latest status and dispatch receipt share the same run_id."
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
