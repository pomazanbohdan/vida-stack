use crate::contract_profile_adapter::{
    blocker_code_str, boot_compatibility_is_backward_compatible, canonical_blocker_codes,
    BlockerCode,
};

pub(crate) struct StatusOperatorContractInputs<'a> {
    pub(crate) boot_compatibility: Option<&'a crate::state_store::BootCompatibilitySummary>,
    pub(crate) migration_state: Option<&'a crate::state_store::MigrationPreflightSummary>,
    pub(crate) protocol_binding: &'a crate::state_store::ProtocolBindingSummary,
    pub(crate) runtime_consumption: &'a crate::runtime_consumption_state::RuntimeConsumptionSummary,
    pub(crate) latest_final_snapshot_path: Option<&'a str>,
    pub(crate) latest_run_graph_status_run_id: Option<&'a str>,
    pub(crate) latest_run_graph_status_task_id: Option<&'a str>,
    pub(crate) latest_run_graph_status_run_id_source: Option<&'a str>,
    pub(crate) latest_run_graph_status_task_id_source: Option<&'a str>,
    pub(crate) latest_run_graph_dispatch_receipt_id: Option<&'a str>,
    pub(crate) latest_run_graph_dispatch_packet_path: Option<&'a str>,
    pub(crate) latest_run_graph_gate_present: bool,
    pub(crate) latest_run_graph_dispatch_receipt_matches_status: bool,
    pub(crate) latest_run_graph_snapshot_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_signal_ambiguous: bool,
    pub(crate) latest_run_graph_dispatch_receipt_summary_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_checkpoint_leakage: bool,
    pub(crate) closed_task_active_run_projection_mismatch: bool,
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
    pub(crate) root_local_write_allowed_for_only_these_paths: &'a serde_json::Value,
    pub(crate) activation_view_only_dispatch_blocker_active: bool,
    pub(crate) blocking_dispatch_blocker_code: Option<&'a str>,
    pub(crate) operator_session_projection: &'a serde_json::Value,
}

pub(crate) struct LatestRunGraphArtifactRefsInputs<'a> {
    pub(crate) run_id: Option<&'a str>,
    pub(crate) task_id: Option<&'a str>,
    pub(crate) run_id_source: Option<&'a str>,
    pub(crate) task_id_source: Option<&'a str>,
    pub(crate) dispatch_receipt_id: Option<&'a str>,
    pub(crate) dispatch_packet_path: Option<&'a str>,
}

pub(crate) fn latest_run_graph_artifact_refs(
    inputs: LatestRunGraphArtifactRefsInputs<'_>,
) -> serde_json::Value {
    serde_json::json!({
        "latest_run_graph_status_run_id": inputs.run_id,
        "latest_run_graph_status_task_id": inputs.task_id,
        "latest_run_graph_status_run_id_source": inputs.run_id_source,
        "latest_run_graph_status_task_id_source": inputs.task_id_source,
        "latest_run_graph_dispatch_receipt_id": inputs.dispatch_receipt_id,
        "latest_run_graph_dispatch_packet_path": inputs.dispatch_packet_path,
    })
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
    if inputs.closed_task_active_run_projection_mismatch {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::ClosedTaskActiveRunProjectionMismatch).to_string());
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
    operator_blocker_codes.extend(
        crate::operator_session_projection::projection_operator_blocker_codes(
            inputs.operator_session_projection,
        ),
    );
    operator_blocker_codes = canonical_blocker_codes(&operator_blocker_codes);
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
        operator_next_actions
            .push(crate::status_surface_signals::protocol_binding_check_next_action());
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRetrievalTrustSourceOperatorEvidence)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::missing_retrieval_trust_source_operator_evidence_next_action(),
        );
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRetrievalTrustSignalOperatorEvidence)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::missing_retrieval_trust_signal_operator_evidence_next_action(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MissingRetrievalTrustOperatorEvidence))
    {
        operator_next_actions.push(
            crate::status_surface_signals::missing_retrieval_trust_operator_evidence_next_action(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == "activation_pending")
    {
        if let Some(truth) = inputs.activation_truth {
            if truth.next_steps.is_empty() {
                operator_next_actions
                    .push(crate::status_surface_signals::project_activation_next_action());
            } else {
                operator_next_actions.extend(truth.next_steps.iter().cloned());
            }
        }
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::ProjectActivationUnknown))
    {
        operator_next_actions
            .push(crate::status_surface_signals::project_activation_unknown_next_action());
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::missing_run_graph_dispatch_receipt_operator_evidence_next_action(),
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
        .any(|code| code == blocker_code_str(BlockerCode::ClosedTaskActiveRunProjectionMismatch))
    {
        operator_next_actions.push(
            crate::status_surface_signals::closed_task_active_run_projection_mismatch_next_action(),
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
            "Regenerate consume-final evidence so canonical risk/register, closure/readiness, and operator-contract fields are complete."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MissingRootSessionWriteGuard))
    {
        operator_next_actions
            .push(crate::status_surface_signals::missing_root_session_write_guard_next_action());
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
    operator_next_actions.extend(
        crate::operator_session_projection::projection_operator_next_actions(
            &operator_blocker_codes,
        ),
    );
    let latest_run_graph_refs = latest_run_graph_artifact_refs(LatestRunGraphArtifactRefsInputs {
        run_id: inputs.latest_run_graph_status_run_id,
        task_id: inputs.latest_run_graph_status_task_id,
        run_id_source: inputs.latest_run_graph_status_run_id_source,
        task_id_source: inputs.latest_run_graph_status_task_id_source,
        dispatch_receipt_id: inputs.latest_run_graph_dispatch_receipt_id,
        dispatch_packet_path: inputs.latest_run_graph_dispatch_packet_path,
    });
    let mut operator_artifact_refs = serde_json::json!({
        "runtime_consumption_latest_snapshot_path": inputs.latest_final_snapshot_path
            .or(inputs.runtime_consumption.latest_snapshot_path.as_deref()),
        "principal_delegation_projection_state": if !inputs.latest_run_graph_gate_present {
            "absent"
        } else if inputs.latest_run_graph_dispatch_receipt_id.is_none() {
            "awaiting_dispatch_receipt"
        } else {
            "runtime_ready_for_projection"
        },
        "memory_governance_projection_state": if inputs.latest_run_graph_gate_present {
            "gate_visible"
        } else {
            "absent"
        },
        "protocol_binding_latest_receipt_id": inputs.protocol_binding.latest_receipt_id,
        "latest_task_reconciliation_receipt_id": inputs.latest_task_reconciliation.map(|receipt| receipt.receipt_id.clone()),
        "effective_instruction_bundle_receipt_id": inputs.effective_bundle_receipt.map(|receipt| receipt.receipt_id.clone()),
        "root_session_write_guard_status": inputs.root_session_write_guard_status,
        "root_local_write_allowed": inputs.root_local_write_allowed,
        "root_local_write_allowed_for_only_these_paths": inputs.root_local_write_allowed_for_only_these_paths,
        "blocking_dispatch_blocker_code": inputs.blocking_dispatch_blocker_code,
        "operator_session_projection": crate::operator_session_projection::projection_operator_artifact_refs(
            inputs.operator_session_projection
        ),
    });
    if let (Some(target), Some(source)) = (
        operator_artifact_refs.as_object_mut(),
        latest_run_graph_refs.as_object(),
    ) {
        target.extend(
            source
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    let finalized = crate::release1_operator_output::finalize_release1_operator_truth(
        operator_blocker_codes,
        operator_next_actions,
        operator_artifact_refs,
    )?;
    Ok(finalized.operator_contracts)
}

#[cfg(test)]
mod tests {
    use super::{build_status_operator_contracts, StatusOperatorContractInputs};
    use std::fs;

    fn protocol_binding_summary(
        latest_receipt_id: Option<&str>,
    ) -> crate::state_store::ProtocolBindingSummary {
        crate::state_store::ProtocolBindingSummary {
            active_bindings: 1,
            blocking_issue_count: 0,
            fully_runtime_bound_count: 1,
            latest_receipt_id: latest_receipt_id.map(str::to_string),
            latest_recorded_at: None,
            latest_scenario: None,
            primary_state_authority: None,
            rust_bound_count: 0,
            script_bound_count: 0,
            total_bindings: 1,
            total_receipts: usize::from(latest_receipt_id.is_some()),
            unbound_count: 0,
        }
    }

    fn ready_activation_truth() -> crate::project_activator_surface::ProjectActivationStatusTruth {
        crate::project_activator_surface::ProjectActivationStatusTruth {
            status: "ready_enough_for_normal_work".to_string(),
            activation_pending: false,
            next_steps: vec![],
        }
    }

    fn empty_operator_session_projection() -> serde_json::Value {
        serde_json::json!({
            "schema_version": "operator-session-projection-v1",
            "current_session": {"session_id": "session-current"},
            "project_foreign_runs": [],
            "project_foreign_blockers": [],
            "global_blockers": [],
            "claim_conflicts": [],
        })
    }

    #[test]
    fn bundle_check_retrieval_trust_signal_clears_missing_retrieval_trust_blockers() {
        let root = std::env::temp_dir().join(format!(
            "vida-status-operator-bundle-check-trust-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let snapshot_path = runtime_dir.join("bundle-check-pass.json");
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume bundle check",
                "check": { "ok": true },
                "blocker_codes": [],
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "artifact_refs": {}
                },
                "bundle": {
                    "cache_delivery_contract": {
                        "retrieval_trust_evidence": {
                            "source": crate::runtime_consumption_state::RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
                            "source_registry_ref": crate::runtime_consumption_state::RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
                            "citation": "runtime-consumption/final-recorded.json",
                            "freshness": "final",
                            "freshness_posture": crate::runtime_consumption_state::RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT,
                            "acl": "protocol-binding-current",
                            "acl_context": "protocol_binding_receipt:protocol-binding-current",
                            "acl_propagation": crate::runtime_consumption_state::RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("bundle-check snapshot should be writable");
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 1,
            bundle_snapshots: 0,
            bundle_check_snapshots: 1,
            final_snapshots: 0,
            latest_kind: Some("bundle-check".to_string()),
            latest_snapshot_path: Some(
                crate::runtime_consumption_state::runtime_consumption_snapshot_path_string(
                    &snapshot_path,
                ),
            ),
        };
        let protocol_binding = protocol_binding_summary(Some("protocol-binding-current"));
        let truth = ready_activation_truth();
        let operator_session_projection = empty_operator_session_projection();

        let contracts = build_status_operator_contracts(StatusOperatorContractInputs {
            boot_compatibility: None,
            migration_state: None,
            protocol_binding: &protocol_binding,
            runtime_consumption: &runtime_consumption,
            latest_final_snapshot_path: None,
            latest_run_graph_status_run_id: Some("run-1"),
            latest_run_graph_status_task_id: Some("task-1"),
            latest_run_graph_status_run_id_source: Some("status"),
            latest_run_graph_status_task_id_source: Some("status"),
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_dispatch_packet_path: Some("packet.json"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            closed_task_active_run_projection_mismatch: false,
            continuation_binding_ambiguous: false,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            root_local_write_allowed_for_only_these_paths: &serde_json::json!([]),
            activation_view_only_dispatch_blocker_active: false,
            blocking_dispatch_blocker_code: None,
            operator_session_projection: &operator_session_projection,
        })
        .expect("operator contracts should render");

        let blockers = contracts["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array");
        assert!(!blockers
            .iter()
            .any(|value| value == "missing_retrieval_trust_source_operator_evidence"));
        assert!(!blockers
            .iter()
            .any(|value| value == "missing_retrieval_trust_signal_operator_evidence"));
        assert!(!blockers
            .iter()
            .any(|value| value == "missing_retrieval_trust_operator_evidence"));

        let _ = fs::remove_dir_all(root);
    }

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
            latest_run_graph_status_run_id: Some("run-1"),
            latest_run_graph_status_task_id: Some("task-1"),
            latest_run_graph_status_run_id_source: Some("status"),
            latest_run_graph_status_task_id_source: Some("status"),
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_dispatch_packet_path: Some("packet.json"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            closed_task_active_run_projection_mismatch: false,
            continuation_binding_ambiguous: false,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            root_local_write_allowed_for_only_these_paths: &serde_json::json!([]),
            activation_view_only_dispatch_blocker_active: true,
            blocking_dispatch_blocker_code: Some("internal_activation_view_only"),
            operator_session_projection: &serde_json::json!({
                "schema_version": "operator-session-projection-v1",
                "current_session": {"session_id": "session-current"},
                "project_foreign_runs": [],
                "project_foreign_blockers": [],
                "global_blockers": [],
                "claim_conflicts": [],
            }),
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
            latest_run_graph_status_run_id: Some("run-1"),
            latest_run_graph_status_task_id: Some("task-1"),
            latest_run_graph_status_run_id_source: Some("status"),
            latest_run_graph_status_task_id_source: Some("status"),
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_dispatch_packet_path: Some("packet.json"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            closed_task_active_run_projection_mismatch: false,
            continuation_binding_ambiguous: true,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            root_local_write_allowed_for_only_these_paths: &serde_json::json!([]),
            activation_view_only_dispatch_blocker_active: false,
            blocking_dispatch_blocker_code: None,
            operator_session_projection: &serde_json::json!({
                "schema_version": "operator-session-projection-v1",
                "current_session": {"session_id": "session-current"},
                "project_foreign_runs": [],
                "project_foreign_blockers": [],
                "global_blockers": [],
                "claim_conflicts": [],
            }),
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
                .is_some_and(|text| text.contains("do not continue by heuristic"))
        }));
    }

    #[test]
    fn closed_task_active_run_projection_mismatch_blocks_status_operator_contracts() {
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: Some("snapshot.json".to_string()),
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            active_bindings: 0,
            blocking_issue_count: 0,
            fully_runtime_bound_count: 0,
            latest_receipt_id: Some("binding-receipt".to_string()),
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
            latest_final_snapshot_path: Some("snapshot.json"),
            latest_run_graph_status_run_id: Some("run-1"),
            latest_run_graph_status_task_id: Some("task-1"),
            latest_run_graph_status_run_id_source: Some("status"),
            latest_run_graph_status_task_id_source: Some("status"),
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_dispatch_packet_path: Some("packet.json"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            closed_task_active_run_projection_mismatch: true,
            continuation_binding_ambiguous: false,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            root_local_write_allowed_for_only_these_paths: &serde_json::json!([]),
            activation_view_only_dispatch_blocker_active: false,
            blocking_dispatch_blocker_code: None,
            operator_session_projection: &serde_json::json!({
                "schema_version": "operator-session-projection-v1",
                "current_session": {"session_id": "session-current"},
                "project_foreign_runs": [],
                "project_foreign_blockers": [],
                "global_blockers": [],
                "claim_conflicts": [],
            }),
        })
        .expect("operator contracts should render");

        let blockers = contracts["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array");
        assert!(blockers
            .iter()
            .any(|value| value == "closed_task_active_run_projection_mismatch"));
        let next_actions = contracts["next_actions"]
            .as_array()
            .expect("next_actions should be an array");
        assert!(next_actions.iter().any(|value| {
            value.as_str().is_some_and(|text| {
                text.contains("vida task reconcile-closed-runs --limit 25")
                    && !text.contains("--json")
                    && text
                        .contains("closed tasks must not remain projected as active runtime work")
            })
        }));
    }

    #[test]
    fn claim_conflicts_block_status_operator_contracts() {
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: Some("snapshot.json".to_string()),
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            active_bindings: 0,
            blocking_issue_count: 0,
            fully_runtime_bound_count: 0,
            latest_receipt_id: Some("binding-receipt".to_string()),
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
        let operator_session_projection = serde_json::json!({
            "schema_version": "operator-session-projection-v1",
            "current_session": {"session_id": "session-current"},
            "project_foreign_runs": [],
            "project_foreign_blockers": [],
            "global_blockers": [],
            "claim_conflicts": [{
                "claim_id": "claim-foreign",
                "orchestrator_session_id": "foreign-session",
                "task_id": "task-a",
                "run_id": "run-a",
                "conflict_domain": "path:crates/vida/src/status_surface.rs",
                "owned_paths": ["crates/vida/src/status_surface.rs"],
                "lease_mode": "exclusive",
                "status": "active",
                "blocker_codes": [],
            }],
        });

        let contracts = build_status_operator_contracts(StatusOperatorContractInputs {
            boot_compatibility: None,
            migration_state: None,
            protocol_binding: &protocol_binding,
            runtime_consumption: &runtime_consumption,
            latest_final_snapshot_path: Some("snapshot.json"),
            latest_run_graph_status_run_id: Some("run-1"),
            latest_run_graph_status_task_id: Some("task-1"),
            latest_run_graph_status_run_id_source: Some("status"),
            latest_run_graph_status_task_id_source: Some("status"),
            latest_run_graph_dispatch_receipt_id: Some("run-1"),
            latest_run_graph_dispatch_packet_path: Some("packet.json"),
            latest_run_graph_gate_present: false,
            latest_run_graph_dispatch_receipt_matches_status: true,
            latest_run_graph_snapshot_inconsistent: false,
            latest_run_graph_dispatch_receipt_signal_ambiguous: false,
            latest_run_graph_dispatch_receipt_summary_inconsistent: false,
            latest_run_graph_dispatch_receipt_checkpoint_leakage: false,
            closed_task_active_run_projection_mismatch: false,
            continuation_binding_ambiguous: false,
            incomplete_release_admission_operator_evidence: false,
            activation_truth: Some(&truth),
            project_activation_pending: false,
            latest_task_reconciliation: None,
            effective_bundle_receipt: None,
            root_session_write_guard_status: "blocked_by_default",
            root_local_write_allowed: false,
            root_local_write_allowed_for_only_these_paths: &serde_json::json!([]),
            activation_view_only_dispatch_blocker_active: false,
            blocking_dispatch_blocker_code: None,
            operator_session_projection: &operator_session_projection,
        })
        .expect("operator contracts should render");

        let blockers = contracts["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array");
        assert!(blockers
            .iter()
            .any(|value| value == "conflict_domain_collision"));
        assert_eq!(
            contracts["artifact_refs"]["operator_session_projection"]["claim_conflict_count"],
            1
        );
        let next_actions = contracts["next_actions"]
            .as_array()
            .expect("next_actions should be an array");
        assert!(next_actions.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|text| text.contains("claim_conflicts"))
        }));
    }
}
