use crate::release1_contracts::{canonical_compatibility_class_str, CompatibilityClass};

pub(crate) struct StatusJsonReportInputs<'a> {
    pub(crate) summary_only: bool,
    pub(crate) operator_contracts: serde_json::Value,
    pub(crate) backend_summary: &'a str,
    pub(crate) state_dir: &'a std::path::Path,
    pub(crate) launcher_runtime_paths: &'a crate::DoctorLauncherSummary,
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
    pub(crate) latest_run_graph_mixed_posture: Option<&'a serde_json::Value>,
    pub(crate) latest_run_graph_activation_vs_execution_evidence: Option<&'a serde_json::Value>,
}

pub(crate) fn build_status_json_report(
    inputs: StatusJsonReportInputs<'_>,
) -> Result<serde_json::Value, String> {
    let latest_run_graph_status = enrich_run_graph_status(
        inputs.latest_run_graph_status,
        inputs.latest_run_graph_mixed_posture,
        inputs.latest_run_graph_activation_vs_execution_evidence,
    );
    let latest_run_graph_dispatch_receipt = enrich_run_graph_dispatch_receipt(
        inputs.latest_run_graph_dispatch_receipt,
        inputs.latest_run_graph_mixed_posture,
        inputs.latest_run_graph_activation_vs_execution_evidence,
    );
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
            "launcher_runtime_paths": inputs.launcher_runtime_paths,
            "state_spine": {
                "state_schema_version": inputs.state_spine.state_schema_version,
                "entity_surface_count": inputs.state_spine.entity_surface_count,
                "authoritative_mutation_root": inputs.state_spine.authoritative_mutation_root,
            },
            "project_activation": project_activation,
            "protocol_binding": inputs.protocol_binding,
            "root_session_write_guard": inputs.root_session_write_guard,
            "continuation_binding": inputs.continuation_binding,
            "latest_run_graph_status": latest_run_graph_status.clone(),
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
            "launcher_runtime_paths": inputs.launcher_runtime_paths,
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
            "latest_run_graph_status": latest_run_graph_status.clone(),
            "latest_run_graph_delegation_gate": inputs.latest_run_graph_status.map(|status| status.delegation_gate()),
            "latest_run_graph_recovery": inputs.latest_run_graph_recovery,
            "latest_run_graph_checkpoint": inputs.latest_run_graph_checkpoint,
            "latest_run_graph_gate": inputs.latest_run_graph_gate,
            "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt.clone(),
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

fn enrich_run_graph_status(
    status: Option<&crate::state_store::RunGraphStatus>,
    mixed_posture: Option<&serde_json::Value>,
    activation_vs_execution_evidence: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(status) = status else {
        return serde_json::Value::Null;
    };
    let mut value = serde_json::to_value(status).expect("run graph status should serialize");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "mixed_posture".to_string(),
            mixed_posture.cloned().unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "activation_vs_execution_evidence".to_string(),
            activation_vs_execution_evidence
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
    }
    value
}

fn enrich_run_graph_dispatch_receipt(
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    mixed_posture: Option<&serde_json::Value>,
    activation_vs_execution_evidence: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(receipt) = receipt else {
        return serde_json::Value::Null;
    };
    let mut value =
        serde_json::to_value(receipt).expect("run graph dispatch receipt should serialize");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "mixed_posture".to_string(),
            mixed_posture.cloned().unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "activation_vs_execution_evidence".to_string(),
            activation_vs_execution_evidence
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "activation_semantics".to_string(),
            activation_vs_execution_evidence
                .and_then(|value| value.get("activation_semantics"))
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "execution_evidence".to_string(),
            activation_vs_execution_evidence
                .and_then(|value| value.get("execution_evidence"))
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{enrich_run_graph_dispatch_receipt, enrich_run_graph_status};

    #[test]
    fn enrich_run_graph_json_surfaces_attach_truth_blocks() {
        let status = crate::state_store::RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "implementer_lane".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
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
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };
        let mixed_posture = serde_json::json!({
            "effective_execution_posture": "hybrid_external_cli",
            "route_primary_backend": "opencode_cli"
        });
        let activation_vs_execution_evidence = serde_json::json!({
            "evidence_state": "execution_evidence_recorded",
            "activation_semantics": {
                "activation_kind": "execution_evidence"
            },
            "execution_evidence": {
                "status": "recorded"
            }
        });

        let status_json = enrich_run_graph_status(
            Some(&status),
            Some(&mixed_posture),
            Some(&activation_vs_execution_evidence),
        );
        let receipt_json = enrich_run_graph_dispatch_receipt(
            Some(&receipt),
            Some(&mixed_posture),
            Some(&activation_vs_execution_evidence),
        );

        assert_eq!(
            status_json["mixed_posture"]["effective_execution_posture"],
            "hybrid_external_cli"
        );
        assert_eq!(
            receipt_json["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(receipt_json["execution_evidence"]["status"], "recorded");
    }

    #[test]
    fn build_status_json_report_keeps_enrichment_nested_and_drops_raw_top_level_mirrors() {
        let operator_contracts = serde_json::json!({
            "status": "ok",
            "trace_id": "trace-1",
            "workflow_class": "implementation",
            "risk_tier": "low",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": [],
        });
        let storage_metadata = crate::state_store::StorageMetadataSummary {
            engine: "surrealkv".to_string(),
            backend: "kv".to_string(),
            namespace: "vida".to_string(),
            database: "primary".to_string(),
            state_schema_version: 1,
            instruction_schema_version: 1,
        };
        let state_spine = crate::state_store::StateSpineSummary {
            authoritative_mutation_root: "state-root".to_string(),
            entity_surface_count: 2,
            state_schema_version: 1,
        };
        let boot_compatibility = crate::state_store::BootCompatibilitySummary {
            classification: "compatible".to_string(),
            reasons: vec!["ok".to_string()],
            next_step: "continue".to_string(),
        };
        let migration_state = crate::state_store::MigrationPreflightSummary {
            contract_type: "operator_contracts".to_string(),
            schema_version: "v1".to_string(),
            compatibility_classification: "compatible".to_string(),
            migration_state: "ready".to_string(),
            blockers: Vec::new(),
            source_version_tuple: vec!["1".to_string()],
            next_step: "continue".to_string(),
        };
        let migration_receipts = crate::state_store::MigrationReceiptSummary {
            compatibility_receipts: 1,
            application_receipts: 2,
            verification_receipts: 3,
            cutover_readiness_receipts: 4,
            rollback_notes: 5,
        };
        let task_reconciliation = crate::state_store::TaskReconciliationSummary {
            receipt_id: "recon-1".to_string(),
            operation: "import".to_string(),
            source_kind: "jsonl".to_string(),
            source_path: Some("/tmp/tasks.jsonl".to_string()),
            task_count: 7,
            dependency_count: 8,
            stale_removed_count: 9,
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };
        let task_reconciliation_rollup = crate::state_store::TaskReconciliationRollup {
            total_receipts: 1,
            latest_recorded_at: Some("2026-04-11T00:00:00Z".to_string()),
            latest_source_path: Some("/tmp/tasks.jsonl".to_string()),
            total_task_rows: 7,
            total_dependency_rows: 8,
            total_stale_removed: 9,
            by_operation: std::collections::BTreeMap::from([("import".to_string(), 1)]),
            by_source_kind: std::collections::BTreeMap::from([("jsonl".to_string(), 1)]),
            rows: Vec::new(),
        };
        let snapshot_bridge = crate::state_store::TaskflowSnapshotBridgeSummary {
            total_receipts: 0,
            export_receipts: 0,
            import_receipts: 0,
            replace_receipts: 0,
            object_export_receipts: 0,
            memory_export_receipts: 0,
            memory_import_receipts: 0,
            memory_replace_receipts: 0,
            file_export_receipts: 0,
            file_import_receipts: 0,
            file_replace_receipts: 0,
            total_task_rows: 0,
            total_dependency_rows: 0,
            total_stale_removed: 0,
            latest_operation: None,
            latest_source_kind: None,
            latest_source_path: None,
            latest_recorded_at: None,
        };
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: None,
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            total_receipts: 1,
            total_bindings: 2,
            active_bindings: 1,
            script_bound_count: 0,
            rust_bound_count: 1,
            fully_runtime_bound_count: 1,
            unbound_count: 0,
            blocking_issue_count: 0,
            latest_receipt_id: Some("protocol-binding-1".to_string()),
            latest_scenario: Some("default".to_string()),
            latest_recorded_at: Some("2026-04-11T00:00:00Z".to_string()),
            primary_state_authority: Some("state-root".to_string()),
        };
        let run_status = crate::state_store::RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-1".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
        let run_receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
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
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };
        let mixed_posture = serde_json::json!({
            "effective_execution_posture": "hybrid_external_cli",
            "route_primary_backend": "opencode_cli"
        });
        let activation_vs_execution_evidence = serde_json::json!({
            "evidence_state": "execution_evidence_recorded",
            "activation_semantics": {
                "activation_kind": "execution_evidence"
            },
            "execution_evidence": {
                "status": "recorded"
            }
        });
        let root_session_write_guard = serde_json::json!({"mode": "locked"});
        let continuation_binding = serde_json::json!({"status": "bound"});
        let launcher_runtime_paths = crate::doctor_launcher_summary_for_root(std::path::Path::new(
            "/tmp/project",
        ))
        .expect("launcher summary should build");

        let summary_json = super::build_status_json_report(super::StatusJsonReportInputs {
            summary_only: true,
            operator_contracts: operator_contracts.clone(),
            backend_summary: "backend summary",
            state_dir: std::path::Path::new("/tmp/state"),
            launcher_runtime_paths: &launcher_runtime_paths,
            storage_metadata: &storage_metadata,
            state_spine: &state_spine,
            effective_bundle_receipt: None,
            boot_compatibility: Some(&boot_compatibility),
            migration_state: Some(&migration_state),
            migration_receipts: &migration_receipts,
            latest_task_reconciliation: Some(&task_reconciliation),
            task_reconciliation_rollup: &task_reconciliation_rollup,
            snapshot_bridge: &snapshot_bridge,
            runtime_consumption: &runtime_consumption,
            protocol_binding: &protocol_binding,
            activation_truth: None,
            project_activation_status: Some("pending"),
            project_activation_pending: true,
            host_agents: None,
            root_session_write_guard: &root_session_write_guard,
            continuation_binding: &continuation_binding,
            latest_run_graph_status: Some(&run_status),
            latest_run_graph_recovery: None,
            latest_run_graph_checkpoint: None,
            latest_run_graph_gate: None,
            latest_run_graph_dispatch_receipt: Some(&run_receipt),
            latest_run_graph_mixed_posture: Some(&mixed_posture),
            latest_run_graph_activation_vs_execution_evidence: Some(
                &activation_vs_execution_evidence,
            ),
        })
        .expect("summary report should build");

        let full_json = super::build_status_json_report(super::StatusJsonReportInputs {
            summary_only: false,
            operator_contracts,
            backend_summary: "backend summary",
            state_dir: std::path::Path::new("/tmp/state"),
            launcher_runtime_paths: &launcher_runtime_paths,
            storage_metadata: &storage_metadata,
            state_spine: &state_spine,
            effective_bundle_receipt: None,
            boot_compatibility: Some(&boot_compatibility),
            migration_state: Some(&migration_state),
            migration_receipts: &migration_receipts,
            latest_task_reconciliation: Some(&task_reconciliation),
            task_reconciliation_rollup: &task_reconciliation_rollup,
            snapshot_bridge: &snapshot_bridge,
            runtime_consumption: &runtime_consumption,
            protocol_binding: &protocol_binding,
            activation_truth: None,
            project_activation_status: Some("pending"),
            project_activation_pending: true,
            host_agents: None,
            root_session_write_guard: &root_session_write_guard,
            continuation_binding: &continuation_binding,
            latest_run_graph_status: Some(&run_status),
            latest_run_graph_recovery: None,
            latest_run_graph_checkpoint: None,
            latest_run_graph_gate: None,
            latest_run_graph_dispatch_receipt: Some(&run_receipt),
            latest_run_graph_mixed_posture: Some(&mixed_posture),
            latest_run_graph_activation_vs_execution_evidence: Some(
                &activation_vs_execution_evidence,
            ),
        })
        .expect("full report should build");

        assert!(summary_json
            .as_object()
            .expect("summary JSON should be an object")
            .get("latest_run_graph_mixed_posture")
            .is_none());
        assert!(summary_json
            .as_object()
            .expect("summary JSON should be an object")
            .get("latest_run_graph_activation_vs_execution_evidence")
            .is_none());
        assert!(full_json
            .as_object()
            .expect("full JSON should be an object")
            .get("latest_run_graph_mixed_posture")
            .is_none());
        assert!(full_json
            .as_object()
            .expect("full JSON should be an object")
            .get("latest_run_graph_activation_vs_execution_evidence")
            .is_none());
        assert_eq!(
            full_json["latest_run_graph_status"]["mixed_posture"]["effective_execution_posture"],
            "hybrid_external_cli"
        );
        assert_eq!(
            full_json["latest_run_graph_dispatch_receipt"]["activation_semantics"]
                ["activation_kind"],
            "execution_evidence"
        );
    }
}
