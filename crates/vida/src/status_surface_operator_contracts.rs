use crate::contract_profile_adapter::{
    blocker_code_str, boot_compatibility_is_backward_compatible, canonical_blocker_codes,
    BlockerCode,
};
use crate::operator_contracts::release1_operator_contracts_consistency_error;

pub(crate) struct StatusOperatorContractInputs<'a> {
    pub(crate) boot_compatibility: Option<&'a crate::state_store::BootCompatibilitySummary>,
    pub(crate) migration_state: Option<&'a crate::state_store::MigrationPreflightSummary>,
    pub(crate) protocol_binding: &'a crate::state_store::ProtocolBindingSummary,
    pub(crate) runtime_consumption: &'a crate::runtime_consumption_state::RuntimeConsumptionSummary,
    pub(crate) latest_final_snapshot_path: Option<&'a str>,
    pub(crate) latest_run_graph_dispatch_receipt_id: Option<&'a str>,
    pub(crate) latest_run_graph_gate_present: bool,
    pub(crate) latest_run_graph_dispatch_receipt_matches_status: bool,
    pub(crate) latest_run_graph_snapshot_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_signal_ambiguous: bool,
    pub(crate) latest_run_graph_dispatch_receipt_summary_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_checkpoint_leakage: bool,
    pub(crate) continuation_binding_ambiguous: bool,
    pub(crate) incomplete_release_admission_operator_evidence: bool,
    pub(crate) activation_truth:
        Option<&'a crate::project_activator_surface::ProjectActivationStatusTruth>,
    pub(crate) project_activation_pending: bool,
    pub(crate) latest_task_reconciliation:
        Option<&'a crate::state_store::TaskReconciliationSummary>,
    pub(crate) effective_bundle_receipt:
        Option<&'a crate::state_store::EffectiveBundleReceiptSummary>,
    pub(crate) root_session_write_guard_status: &'a str,
    pub(crate) root_local_write_allowed: bool,
    pub(crate) activation_view_only_dispatch_blocker_active: bool,
    pub(crate) blocking_dispatch_blocker_code: Option<&'a str>,
}

pub(crate) fn build_status_operator_contracts(
    inputs: StatusOperatorContractInputs<'_>,
) -> Result<serde_json::Value, String> {
    let mut operator_blocker_codes: Vec<String> = Vec::new();

    if inputs.boot_compatibility.is_some_and(|compatibility| {
        !boot_compatibility_is_backward_compatible(&compatibility.classification)
    }) {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::BootCompatibilityNotCompatible).to_string());
    }
    if inputs.migration_state.is_some_and(|migration| {
        !boot_compatibility_is_backward_compatible(&migration.compatibility_classification)
    }) {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::MigrationPreflightNotReady).to_string());
    }
    if inputs.migration_state.is_some_and(|migration| {
        crate::status_surface_signals::migration_requires_action(&migration.migration_state)
    }) {
        operator_blocker_codes.push(blocker_code_str(BlockerCode::MigrationRequired).to_string());
    }
    if inputs.protocol_binding.blocking_issue_count > 0 {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::ProtocolBindingBlockingIssues).to_string());
    }
    let retrieval_trust_signal =
        crate::runtime_consumption_state::latest_admissible_retrieval_trust_signal(
            inputs.runtime_consumption,
            inputs.latest_final_snapshot_path,
            inputs.protocol_binding.latest_receipt_id.as_deref(),
        );
    if retrieval_trust_signal.is_none() {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::MissingRetrievalTrustSourceOperatorEvidence).to_string(),
        );
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::MissingRetrievalTrustSignalOperatorEvidence).to_string(),
        );
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::MissingRetrievalTrustOperatorEvidence).to_string());
    }
    if inputs.latest_run_graph_gate_present
        && !inputs.latest_run_graph_dispatch_receipt_matches_status
    {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence)
                .to_string(),
        );
    }
    if inputs.latest_run_graph_snapshot_inconsistent {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::RunGraphLatestSnapshotInconsistent).to_string());
    }
    if inputs.latest_run_graph_dispatch_receipt_signal_ambiguous {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptSignalAmbiguous).to_string(),
        );
    }
    if inputs.latest_run_graph_dispatch_receipt_summary_inconsistent {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptSummaryInconsistent)
                .to_string(),
        );
    }
    if inputs.latest_run_graph_dispatch_receipt_checkpoint_leakage {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage)
                .to_string(),
        );
    }
    if inputs.continuation_binding_ambiguous {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::ContinuationBindingAmbiguous).to_string());
    }
    if inputs.incomplete_release_admission_operator_evidence {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::IncompleteReleaseAdmissionOperatorEvidence).to_string(),
        );
    } else if inputs.activation_view_only_dispatch_blocker_active {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::LocalTakeoverForbidden).to_string());
    } else if !matches!(
        inputs.root_session_write_guard_status,
        "blocked_by_default" | "exception_takeover_active"
    ) {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::MissingRootSessionWriteGuard).to_string());
    }
    match inputs.activation_truth {
        Some(_) if inputs.project_activation_pending => {
            operator_blocker_codes
                .push(blocker_code_str(BlockerCode::ActivationPending).to_string());
        }
        None => {
            operator_blocker_codes
                .push(blocker_code_str(BlockerCode::ProjectActivationUnknown).to_string());
        }
        _ => {}
    }
    operator_blocker_codes = canonical_blocker_codes(&operator_blocker_codes);
    let operator_status = if operator_blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let mut operator_next_actions: Vec<String> = Vec::new();
    if operator_blocker_codes
        .iter()
        .any(|code| code == "boot_incompatible")
    {
        if let Some(compatibility) = inputs.boot_compatibility {
            operator_next_actions.push(compatibility.next_step.clone());
        }
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == "migration_not_ready")
    {
        if let Some(migration) = inputs.migration_state {
            operator_next_actions.push(migration.next_step.clone());
        }
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MigrationRequired))
    {
        operator_next_actions
            .push("Complete required migration before normal operation.".to_string());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::ProtocolBindingBlockingIssues))
    {
        operator_next_actions.push(
            "Run `vida taskflow protocol-binding check --json` and clear blockers.".to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRetrievalTrustSourceOperatorEvidence)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::MISSING_RETRIEVAL_TRUST_SOURCE_OPERATOR_EVIDENCE_NEXT_ACTION
                .to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRetrievalTrustSignalOperatorEvidence)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::MISSING_RETRIEVAL_TRUST_SIGNAL_OPERATOR_EVIDENCE_NEXT_ACTION
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MissingRetrievalTrustOperatorEvidence))
    {
        operator_next_actions.push(
            crate::status_surface_signals::MISSING_RETRIEVAL_TRUST_OPERATOR_EVIDENCE_NEXT_ACTION
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == "activation_pending")
    {
        if let Some(truth) = inputs.activation_truth {
            if truth.next_steps.is_empty() {
                operator_next_actions.push(
                    "Complete project activation via `vida project-activator --json` before normal work."
                        .to_string(),
                );
            } else {
                operator_next_actions.extend(truth.next_steps.iter().cloned());
            }
        }
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::ProjectActivationUnknown))
    {
        operator_next_actions.push(
            "Resolve project root detection and run `vida project-activator --json` to surface canonical activation state."
                .to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence)
    }) {
        operator_next_actions.push(
            "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::RunGraphLatestSnapshotInconsistent))
    {
        operator_next_actions.push(
            crate::status_surface_signals::run_graph_latest_snapshot_inconsistent_next_action()
                .to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptSignalAmbiguous)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_signal_ambiguous_next_action()
                .to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptSummaryInconsistent)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()
                .to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::ContinuationBindingAmbiguous))
    {
        operator_next_actions.push(
            crate::status_surface_signals::continuation_binding_ambiguous_next_action().to_string(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::IncompleteReleaseAdmissionOperatorEvidence)
    }) {
        operator_next_actions.push(
            "Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MissingRootSessionWriteGuard))
    {
        operator_next_actions.push(
            "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::LocalTakeoverForbidden))
    {
        operator_next_actions.push(
            "The latest delegated handoff returned only an activation view without execution evidence; keep root-local writes blocked, continue bounded read-only diagnosis or reroute `vida agent-init`, and record an explicit exception-path receipt before any local mutation."
                .to_string(),
        );
    }
    let operator_artifact_refs = serde_json::json!({
        "runtime_consumption_latest_snapshot_path": inputs.latest_final_snapshot_path
            .or(inputs.runtime_consumption.latest_snapshot_path.as_deref()),
        "latest_run_graph_dispatch_receipt_id": inputs.latest_run_graph_dispatch_receipt_id,
        "protocol_binding_latest_receipt_id": inputs.protocol_binding.latest_receipt_id,
        "latest_task_reconciliation_receipt_id": inputs.latest_task_reconciliation.map(|receipt| receipt.receipt_id.clone()),
        "effective_instruction_bundle_receipt_id": inputs.effective_bundle_receipt.map(|receipt| receipt.receipt_id.clone()),
        "root_session_write_guard_status": inputs.root_session_write_guard_status,
        "root_local_write_allowed": inputs.root_local_write_allowed,
        "blocking_dispatch_blocker_code": inputs.blocking_dispatch_blocker_code,
    });
    let operator_contracts = crate::operator_contracts::render_operator_contract_envelope(
        &crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC,
        operator_status,
        operator_blocker_codes,
        operator_next_actions,
        operator_artifact_refs,
    );
    let blocker_codes = operator_contracts["blocker_codes"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let next_actions = operator_contracts["next_actions"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(error) = release1_operator_contracts_consistency_error(
        operator_contracts["status"].as_str().unwrap_or(""),
        &blocker_codes,
        &next_actions,
    ) {
        return Err(error);
    }
    Ok(operator_contracts)
}

#[cfg(test)]
mod tests {
    use super::{build_status_operator_contracts, StatusOperatorContractInputs};

    #[test]
    fn activation_view_only_dispatch_blocks_local_takeover_in_operator_contracts() {
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: None,
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            active_bindings: 0,
            blocking_issue_count: 0,
            fully_runtime_bound_count: 0,
            latest_receipt_id: None,
            latest_recorded_at: None,
            latest_scenario: None,
            primary_state_authority: None,
            rust_bound_count: 0,
            script_bound_count: 0,
            total_bindings: 0,
            total_receipts: 0,
            unbound_count: 0,
        };
        let truth = crate::project_activator_surface::ProjectActivationStatusTruth {
            status: "ready_enough_for_normal_work".to_string(),
            activation_pending: false,
            next_steps: vec![],
        };

        let contracts = build_status_operator_contracts(StatusOperatorContractInputs {
            boot_compatibility: None,
            migration_state: None,
            protocol_binding: &protocol_binding,
            runtime_consumption: &runtime_consumption,
            latest_final_snapshot_path: None,
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            continuation_binding_ambiguous: false,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            activation_view_only_dispatch_blocker_active: true,
            blocking_dispatch_blocker_code: Some("internal_activation_view_only"),
        })
        .expect("operator contracts should render");

        let blockers = contracts["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array");
        assert!(blockers
            .iter()
            .any(|value| value == "local_takeover_forbidden"));
        let next_actions = contracts["next_actions"]
            .as_array()
            .expect("next_actions should be an array");
        assert!(next_actions.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|text| text.contains("activation view without execution evidence"))
        }));
        assert_eq!(
            contracts["artifact_refs"]["root_local_write_allowed"],
            false
        );
        assert_eq!(
            contracts["artifact_refs"]["blocking_dispatch_blocker_code"],
            "internal_activation_view_only"
        );
    }

    #[test]
    fn continuation_binding_ambiguous_blocks_operator_contracts() {
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: None,
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            active_bindings: 0,
            blocking_issue_count: 0,
            fully_runtime_bound_count: 0,
            latest_receipt_id: None,
            latest_recorded_at: None,
            latest_scenario: None,
            primary_state_authority: None,
            rust_bound_count: 0,
            script_bound_count: 0,
            total_bindings: 0,
            total_receipts: 0,
            unbound_count: 0,
        };
        let truth = crate::project_activator_surface::ProjectActivationStatusTruth {
            status: "ready_enough_for_normal_work".to_string(),
            activation_pending: false,
            next_steps: vec![],
        };

        let contracts = build_status_operator_contracts(StatusOperatorContractInputs {
            boot_compatibility: None,
            migration_state: None,
            protocol_binding: &protocol_binding,
            runtime_consumption: &runtime_consumption,
            latest_final_snapshot_path: None,
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            continuation_binding_ambiguous: true,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            activation_view_only_dispatch_blocker_active: false,
            blocking_dispatch_blocker_code: None,
        })
        .expect("operator contracts should render");

        let blockers = contracts["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array");
        assert!(blockers
            .iter()
            .any(|value| value == "continuation_binding_ambiguous"));
        let next_actions = contracts["next_actions"]
            .as_array()
            .expect("next_actions should be an array");
        assert!(next_actions.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|text| text.contains("Do not continue by heuristic"))
        }));
    }
}
