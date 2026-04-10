use crate::release1_contracts::{canonical_compatibility_class_str, CompatibilityClass};

pub(crate) struct StatusJsonReportInputs<'a> {
    pub(crate) summary_only: bool,
    pub(crate) operator_contracts: serde_json::Value,
    pub(crate) backend_summary: &'a str,
    pub(crate) state_dir: &'a std::path::Path,
    pub(crate) storage_metadata: &'a crate::state_store::StorageMetadataSummary,
    pub(crate) state_spine: &'a crate::state_store::StateSpineSummary,
    pub(crate) effective_bundle_receipt:
        Option<&'a crate::state_store::EffectiveBundleReceiptSummary>,
    pub(crate) boot_compatibility: Option<&'a crate::state_store::BootCompatibilitySummary>,
    pub(crate) migration_state: Option<&'a crate::state_store::MigrationPreflightSummary>,
    pub(crate) migration_receipts: &'a crate::state_store::MigrationReceiptSummary,
    pub(crate) latest_task_reconciliation:
        Option<&'a crate::state_store::TaskReconciliationSummary>,
    pub(crate) task_reconciliation_rollup: &'a crate::state_store::TaskReconciliationRollup,
    pub(crate) snapshot_bridge: &'a crate::state_store::TaskflowSnapshotBridgeSummary,
    pub(crate) runtime_consumption: &'a crate::runtime_consumption_state::RuntimeConsumptionSummary,
    pub(crate) protocol_binding: &'a crate::state_store::ProtocolBindingSummary,
    pub(crate) activation_truth:
        Option<&'a crate::project_activator_surface::ProjectActivationStatusTruth>,
    pub(crate) project_activation_status: Option<&'a str>,
    pub(crate) project_activation_pending: bool,
    pub(crate) host_agents: Option<&'a serde_json::Value>,
    pub(crate) root_session_write_guard: &'a serde_json::Value,
    pub(crate) continuation_binding: &'a serde_json::Value,
    pub(crate) latest_run_graph_status: Option<&'a crate::state_store::RunGraphStatus>,
    pub(crate) latest_run_graph_recovery: Option<&'a crate::state_store::RunGraphRecoverySummary>,
    pub(crate) latest_run_graph_checkpoint:
        Option<&'a crate::state_store::RunGraphCheckpointSummary>,
    pub(crate) latest_run_graph_gate: Option<&'a crate::state_store::RunGraphGateSummary>,
    pub(crate) latest_run_graph_dispatch_receipt:
        Option<&'a crate::state_store::RunGraphDispatchReceiptSummary>,
}

pub(crate) fn build_status_json_report(
    inputs: StatusJsonReportInputs<'_>,
) -> Result<serde_json::Value, String> {
    let project_activation = inputs
        .activation_truth
        .map(|truth| {
            serde_json::json!({
                "status": inputs.project_activation_status.unwrap_or("pending"),
                "activation_pending": inputs.project_activation_pending,
                "next_steps": truth.next_steps,
            })
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "status": "unknown",
                "activation_pending": true,
                "next_steps": [
                    "run `vida project-activator --json` from the project root to load canonical activation state"
                ],
            })
        });

    let summary_json = if inputs.summary_only {
        serde_json::json!({
            "surface": "vida status",
            "view": "summary",
            "status": inputs.operator_contracts["status"].clone(),
            "trace_id": inputs.operator_contracts["trace_id"].clone(),
            "workflow_class": inputs.operator_contracts["workflow_class"].clone(),
            "risk_tier": inputs.operator_contracts["risk_tier"].clone(),
            "blocker_codes": inputs.operator_contracts["blocker_codes"].clone(),
            "next_actions": inputs.operator_contracts["next_actions"].clone(),
            "artifact_refs": inputs.operator_contracts["artifact_refs"].clone(),
            "shared_fields": {
                "trace_id": inputs.operator_contracts["trace_id"].clone(),
                "workflow_class": inputs.operator_contracts["workflow_class"].clone(),
                "risk_tier": inputs.operator_contracts["risk_tier"].clone(),
                "status": inputs.operator_contracts["status"].clone(),
                "blocker_codes": inputs.operator_contracts["blocker_codes"].clone(),
                "next_actions": inputs.operator_contracts["next_actions"].clone(),
                "artifact_refs": inputs.operator_contracts["artifact_refs"].clone(),
            },
            "operator_contracts": inputs.operator_contracts,
            "backend_summary": inputs.backend_summary,
            "state_spine": {
                "state_schema_version": inputs.state_spine.state_schema_version,
                "entity_surface_count": inputs.state_spine.entity_surface_count,
                "authoritative_mutation_root": inputs.state_spine.authoritative_mutation_root,
            },
            "project_activation": project_activation,
            "protocol_binding": inputs.protocol_binding,
            "root_session_write_guard": inputs.root_session_write_guard,
            "continuation_binding": inputs.continuation_binding,
            "latest_run_graph_status": inputs.latest_run_graph_status,
            "latest_run_graph_recovery": inputs.latest_run_graph_recovery,
            "latest_run_graph_gate": inputs.latest_run_graph_gate,
            "host_agents": host_agents_json_value(inputs.host_agents),
        })
    } else {
        serde_json::json!({
            "surface": "vida status",
            "status": inputs.operator_contracts["status"].clone(),
            "trace_id": inputs.operator_contracts["trace_id"].clone(),
            "workflow_class": inputs.operator_contracts["workflow_class"].clone(),
            "risk_tier": inputs.operator_contracts["risk_tier"].clone(),
            "blocker_codes": inputs.operator_contracts["blocker_codes"].clone(),
            "next_actions": inputs.operator_contracts["next_actions"].clone(),
            "artifact_refs": inputs.operator_contracts["artifact_refs"].clone(),
            "shared_fields": {
                "trace_id": inputs.operator_contracts["trace_id"].clone(),
                "workflow_class": inputs.operator_contracts["workflow_class"].clone(),
                "risk_tier": inputs.operator_contracts["risk_tier"].clone(),
                "status": inputs.operator_contracts["status"].clone(),
                "blocker_codes": inputs.operator_contracts["blocker_codes"].clone(),
                "next_actions": inputs.operator_contracts["next_actions"].clone(),
                "artifact_refs": inputs.operator_contracts["artifact_refs"].clone(),
            },
            "operator_contracts": inputs.operator_contracts,
            "state_dir": inputs.state_dir.display().to_string(),
            "storage_metadata": {
                "engine": inputs.storage_metadata.engine,
                "backend": inputs.storage_metadata.backend,
                "namespace": inputs.storage_metadata.namespace,
                "database": inputs.storage_metadata.database,
                "state_schema_version": inputs.storage_metadata.state_schema_version,
                "instruction_schema_version": inputs.storage_metadata.instruction_schema_version,
            },
            "backend_summary": inputs.backend_summary,
            "state_spine": {
                "state_schema_version": inputs.state_spine.state_schema_version,
                "entity_surface_count": inputs.state_spine.entity_surface_count,
                "authoritative_mutation_root": inputs.state_spine.authoritative_mutation_root,
            },
            "latest_effective_bundle_receipt": inputs.effective_bundle_receipt,
            "boot_compatibility": inputs.boot_compatibility.map(|compatibility| serde_json::json!({
                "classification": canonical_compatibility_class_str(&compatibility.classification)
                    .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
                "reasons": compatibility.reasons,
                "next_step": compatibility.next_step,
            })),
            "migration_state": inputs.migration_state.map(|migration| serde_json::json!({
                "compatibility_class": canonical_compatibility_class_str(
                    &migration.compatibility_classification
                ).unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
                "migration_state": migration.migration_state,
                "blockers": migration.blockers,
                "source_version_tuple": migration.source_version_tuple,
                "next_step": migration.next_step,
            })),
            "migration_receipts": {
                "compatibility_receipts": inputs.migration_receipts.compatibility_receipts,
                "application_receipts": inputs.migration_receipts.application_receipts,
                "verification_receipts": inputs.migration_receipts.verification_receipts,
                "cutover_readiness_receipts": inputs.migration_receipts.cutover_readiness_receipts,
                "rollback_notes": inputs.migration_receipts.rollback_notes,
            },
            "latest_task_reconciliation": inputs.latest_task_reconciliation,
            "task_reconciliation_rollup": inputs.task_reconciliation_rollup,
            "taskflow_snapshot_bridge": inputs.snapshot_bridge,
            "runtime_consumption": inputs.runtime_consumption,
            "protocol_binding": inputs.protocol_binding,
            "project_activation": project_activation,
            "host_agents": host_agents_json_value(inputs.host_agents),
            "root_session_write_guard": inputs.root_session_write_guard,
            "continuation_binding": inputs.continuation_binding,
            "latest_run_graph_status": inputs.latest_run_graph_status,
            "latest_run_graph_delegation_gate": inputs.latest_run_graph_status.map(|status| status.delegation_gate()),
            "latest_run_graph_recovery": inputs.latest_run_graph_recovery,
            "latest_run_graph_checkpoint": inputs.latest_run_graph_checkpoint,
            "latest_run_graph_gate": inputs.latest_run_graph_gate,
            "latest_run_graph_dispatch_receipt": inputs.latest_run_graph_dispatch_receipt,
        })
    };

    if let Some(error) =
        crate::operator_contracts::shared_operator_output_contract_parity_error(&summary_json)
    {
        return Err(error.to_string());
    }
    if summary_json["artifact_refs"] != summary_json["operator_contracts"]["artifact_refs"]
        || summary_json["artifact_refs"] != summary_json["shared_fields"]["artifact_refs"]
    {
        return Err("top-level/operator_contracts/shared_fields mirror mismatch".to_string());
    }

    Ok(summary_json)
}

fn host_agents_json_value(host_agents: Option<&serde_json::Value>) -> serde_json::Value {
    host_agents
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}))
}
