use std::process::ExitCode;

use crate::contract_profile_adapter::{
    blocker_code_str, canonical_blocker_code_list, canonical_compatibility_class_str,
    classify_compatibility_boundary, operator_contracts_consistency_error,
    render_operator_contract_envelope, shared_operator_output_contract_parity_error, BlockerCode,
    CompatibilityBoundary, CompatibilityClass,
};

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    BlockerCode::UnsupportedArchitectureReservedWorkflowBoundary.as_str();
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str = "Clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

const MISSING_RETRIEVAL_TRUST_SOURCE_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "Run `vida taskflow consume bundle check --json` so runtime consumption snapshots publish retrieval-trust source evidence.";
const MISSING_RETRIEVAL_TRUST_SIGNAL_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "Run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle check --json` to materialize retrieval-trust citation/freshness/ACL signal.";
const MISSING_RETRIEVAL_TRUST_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "Run `vida taskflow consume bundle check --json` to record retrieval-trust operator evidence.";

fn is_unsupported_architecture_reserved_workflow_boundary(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "unsupported" | "architecture_reserved" | "unsupported_architecture_reserved"
    )
}

fn final_snapshot_missing_release_admission_evidence(snapshot_path: &str) -> bool {
    let payload = match std::fs::read_to_string(snapshot_path) {
        Ok(payload) => payload,
        Err(_) => return true,
    };
    let summary_json = match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(json) => json,
        Err(_) => return true,
    };
    if shared_operator_output_contract_parity_error(&summary_json).is_some() {
        return true;
    }
    !super::runtime_consumption_snapshot_has_release_admission_evidence(&summary_json)
}

fn trace_evidence_next_action() -> String {
    "Refresh task reconciliation, runtime consumption, run-graph dispatch receipt, protocol binding, and effective instruction bundle evidence before rerunning `vida doctor`.".to_string()
}

fn selected_effective_bundle_receipt_id(
    effective_instruction_bundle: &crate::state_store::EffectiveInstructionBundle,
    latest_effective_bundle_receipt: Option<&crate::state_store::EffectiveBundleReceiptSummary>,
) -> String {
    latest_effective_bundle_receipt
        .and_then(|receipt| {
            let receipt_id = receipt.receipt_id.trim();
            (!receipt_id.is_empty()).then(|| receipt_id.to_string())
        })
        .unwrap_or_else(|| effective_instruction_bundle.receipt_id.clone())
}

fn trace_evidence_blocker_codes(
    latest_task_reconciliation: Option<&crate::state_store::TaskReconciliationSummary>,
    runtime_consumption: &crate::runtime_consumption_state::RuntimeConsumptionSummary,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    protocol_binding: &crate::state_store::ProtocolBindingSummary,
    effective_instruction_bundle: &crate::state_store::EffectiveInstructionBundle,
    effective_bundle_receipt_id: &str,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();

    if latest_task_reconciliation.is_none()
        || runtime_consumption.total_snapshots == 0
        || latest_run_graph_dispatch_receipt.is_none()
        || protocol_binding.total_receipts == 0
    {
        blocker_codes.push(blocker_code_str(BlockerCode::TraceMissing).to_string());
    }

    if runtime_consumption.total_snapshots > 0 && runtime_consumption.latest_snapshot_path.is_none()
    {
        blocker_codes.push(blocker_code_str(BlockerCode::TraceIncomplete).to_string());
    }
    if protocol_binding.total_receipts > 0 && protocol_binding.latest_receipt_id.is_none() {
        blocker_codes.push(blocker_code_str(BlockerCode::TraceIncomplete).to_string());
    }
    if effective_bundle_receipt_id.trim().is_empty()
        || effective_instruction_bundle.projected_artifacts.is_empty()
        || effective_instruction_bundle
            .mandatory_chain_order
            .is_empty()
    {
        blocker_codes.push(blocker_code_str(BlockerCode::TraceIncomplete).to_string());
    }

    canonical_blocker_code_list(blocker_codes.iter().map(String::as_str))
}

fn build_trace_evidence_summary(
    latest_task_reconciliation: Option<&crate::state_store::TaskReconciliationSummary>,
    runtime_consumption: &crate::runtime_consumption_state::RuntimeConsumptionSummary,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    protocol_binding: &crate::state_store::ProtocolBindingSummary,
    effective_instruction_bundle: &crate::state_store::EffectiveInstructionBundle,
    effective_bundle_receipt_id: &str,
) -> (serde_json::Value, Vec<String>, Vec<String>) {
    let blocker_codes = trace_evidence_blocker_codes(
        latest_task_reconciliation,
        runtime_consumption,
        latest_run_graph_dispatch_receipt,
        protocol_binding,
        effective_instruction_bundle,
        effective_bundle_receipt_id,
    );
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        vec![trace_evidence_next_action()]
    };
    let trace_evidence = serde_json::json!({
        "contract_id": "release-1-trace-evidence",
        "schema_version": "release-1-v1",
        "status": if blocker_codes.is_empty() { "pass" } else { "blocked" },
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "root_trace": {
            "trace_id": serde_json::Value::Null,
            "latest_task_reconciliation_receipt_id": latest_task_reconciliation
                .map(|receipt| serde_json::Value::String(receipt.receipt_id.clone()))
                .unwrap_or(serde_json::Value::Null),
            "runtime_consumption_latest_snapshot_path": runtime_consumption
                .latest_snapshot_path
                .as_ref()
                .map(|path| serde_json::Value::String(path.clone()))
                .unwrap_or(serde_json::Value::Null),
            "latest_run_graph_dispatch_receipt_id": latest_run_graph_dispatch_receipt
                .map(|receipt| serde_json::Value::String(receipt.run_id.clone()))
                .unwrap_or(serde_json::Value::Null),
            "protocol_binding_latest_receipt_id": protocol_binding
                .latest_receipt_id
                .as_ref()
                .map(|receipt_id| serde_json::Value::String(receipt_id.clone()))
                .unwrap_or(serde_json::Value::Null),
            "effective_instruction_bundle_receipt_id": effective_bundle_receipt_id,
        },
        "lane_receipts": {
            "latest_task_reconciliation": latest_task_reconciliation,
            "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
        },
        "side_effect_evidence": {
            "runtime_consumption": runtime_consumption,
            "protocol_binding": protocol_binding,
        },
        "evaluation_evidence": {
            "effective_instruction_bundle": {
                "root_artifact_id": effective_instruction_bundle.root_artifact_id,
                "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
                "source_version_tuple": effective_instruction_bundle.source_version_tuple,
                "receipt_id": effective_bundle_receipt_id,
                "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
            }
        }
    });
    (trace_evidence, blocker_codes, next_actions)
}

fn trace_evidence_display(trace_evidence: &serde_json::Value) -> String {
    let status = trace_evidence["status"].as_str().unwrap_or("unknown");
    let task_reconciliation = trace_evidence["root_trace"]["latest_task_reconciliation_receipt_id"]
        .as_str()
        .unwrap_or("none");
    let dispatch_receipt = trace_evidence["root_trace"]["latest_run_graph_dispatch_receipt_id"]
        .as_str()
        .unwrap_or("none");
    let runtime_consumption = trace_evidence["root_trace"]
        ["runtime_consumption_latest_snapshot_path"]
        .as_str()
        .unwrap_or("none");
    let protocol_binding = trace_evidence["root_trace"]["protocol_binding_latest_receipt_id"]
        .as_str()
        .unwrap_or("none");
    let evaluation_bundle = trace_evidence["root_trace"]["effective_instruction_bundle_receipt_id"]
        .as_str()
        .unwrap_or("none");

    format!(
        "{status} (task_reconciliation={task_reconciliation}, dispatch_receipt={dispatch_receipt}, runtime_consumption={runtime_consumption}, protocol_binding={protocol_binding}, evaluation_bundle={evaluation_bundle})"
    )
}

fn doctor_operator_blocker_codes(
    dependency_graph_issues: &[crate::state_store::TaskGraphIssue],
    boot_compatibility: &crate::state_store::BootCompatibilitySummary,
    migration_preflight: &crate::state_store::MigrationPreflightSummary,
    protocol_binding: &crate::state_store::ProtocolBindingSummary,
    latest_final_snapshot_path: Option<&str>,
    runtime_consumption: &crate::runtime_consumption_state::RuntimeConsumptionSummary,
    latest_recorded_final_snapshot_path: Option<&str>,
    root_session_write_guard: &serde_json::Value,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_gate: Option<&crate::state_store::RunGraphGateSummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    trace_evidence_blocker_codes: Vec<String>,
) -> Vec<String> {
    let mut operator_blocker_codes: Vec<String> = Vec::new();

    if !dependency_graph_issues.is_empty() {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::DependencyGraphIssues).to_string());
    }
    match classify_compatibility_boundary(&boot_compatibility.classification) {
        CompatibilityBoundary::Compatible => {}
        CompatibilityBoundary::BlockingSupported => {
            operator_blocker_codes
                .push(blocker_code_str(BlockerCode::BootCompatibilityNotCompatible).to_string());
        }
        CompatibilityBoundary::Unsupported => {
            operator_blocker_codes.push(
                blocker_code_str(BlockerCode::BootCompatibilityUnsupportedBoundary).to_string(),
            );
        }
    }
    match classify_compatibility_boundary(&migration_preflight.compatibility_classification) {
        CompatibilityBoundary::Compatible => {}
        CompatibilityBoundary::BlockingSupported => {
            operator_blocker_codes
                .push(blocker_code_str(BlockerCode::MigrationPreflightNotReady).to_string());
        }
        CompatibilityBoundary::Unsupported => {
            operator_blocker_codes.push(
                blocker_code_str(BlockerCode::MigrationPreflightUnsupportedBoundary).to_string(),
            );
        }
    }
    if migration_requires_action(&migration_preflight.migration_state) {
        operator_blocker_codes.push(blocker_code_str(BlockerCode::MigrationRequired).to_string());
    }
    if protocol_binding.blocking_issue_count > 0 {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::ProtocolBindingBlockingIssues).to_string());
    }
    let retrieval_trust_signal =
        super::runtime_consumption_state::latest_admissible_retrieval_trust_signal(
            runtime_consumption,
            latest_final_snapshot_path,
            protocol_binding.latest_receipt_id.as_deref(),
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
    if latest_recorded_final_snapshot_path
        .is_some_and(final_snapshot_missing_release_admission_evidence)
    {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::IncompleteReleaseAdmissionOperatorEvidence).to_string(),
        );
    }
    if root_session_write_guard["activation_view_only_dispatch_blocker_active"].as_bool()
        == Some(true)
    {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::LocalTakeoverForbidden).to_string());
    }
    if !matches!(
        root_session_write_guard["status"].as_str(),
        Some("blocked_by_default" | "exception_takeover_active")
    ) {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::MissingRootSessionWriteGuard).to_string());
    }
    if latest_run_graph_recovery
        .as_ref()
        .is_some_and(|summary| !summary.recovery_ready)
    {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::RecoveryReadinessBlocked).to_string());
    }
    if latest_run_graph_gate.as_ref().is_some_and(|summary| {
        is_unsupported_architecture_reserved_workflow_boundary(&summary.policy_gate)
            || is_unsupported_architecture_reserved_workflow_boundary(&summary.context_state)
    }) {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::UnsupportedArchitectureReservedWorkflowBoundary)
                .to_string(),
        );
    }
    if latest_run_graph_gate.is_some() && latest_run_graph_dispatch_receipt.is_none() {
        operator_blocker_codes
            .push(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER.to_string());
    }
    operator_blocker_codes.extend(trace_evidence_blocker_codes);
    canonical_blocker_code_list(operator_blocker_codes.iter().map(String::as_str))
}

fn doctor_operator_next_actions(
    operator_blocker_codes: &[String],
    boot_compatibility: &crate::state_store::BootCompatibilitySummary,
    migration_preflight: &crate::state_store::MigrationPreflightSummary,
) -> Vec<String> {
    let mut operator_next_actions: Vec<String> = Vec::new();
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::DependencyGraphIssues))
    {
        operator_next_actions
            .push("Run `vida task validate-graph --json` and resolve graph issues.".to_string());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == "boot_incompatible")
    {
        operator_next_actions.push(boot_compatibility.next_step.clone());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::BootCompatibilityUnsupportedBoundary))
    {
        operator_next_actions.push(
            "Normalize boot compatibility classification to release-1 values: backward_compatible|reader_upgrade_required.".to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == "migration_not_ready")
    {
        operator_next_actions.push(migration_preflight.next_step.clone());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MigrationPreflightUnsupportedBoundary))
    {
        operator_next_actions.push(
            "Normalize migration preflight compatibility classification to release-1 values: backward_compatible|reader_upgrade_required.".to_string(),
        );
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
        operator_next_actions
            .push(MISSING_RETRIEVAL_TRUST_SOURCE_OPERATOR_EVIDENCE_NEXT_ACTION.to_string());
    }
    if operator_blocker_codes.iter().any(|code| {
        code == blocker_code_str(BlockerCode::MissingRetrievalTrustSignalOperatorEvidence)
    }) {
        operator_next_actions
            .push(MISSING_RETRIEVAL_TRUST_SIGNAL_OPERATOR_EVIDENCE_NEXT_ACTION.to_string());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::MissingRetrievalTrustOperatorEvidence))
    {
        operator_next_actions
            .push(MISSING_RETRIEVAL_TRUST_OPERATOR_EVIDENCE_NEXT_ACTION.to_string());
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
        .any(|code| code == blocker_code_str(BlockerCode::RecoveryReadinessBlocked))
    {
        operator_next_actions.push(
            "Inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER)
    {
        operator_next_actions
            .push(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION.to_string());
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER)
    {
        operator_next_actions
            .push(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION.to_string());
    }
    operator_next_actions
}

pub(crate) async fn run_doctor(args: super::DoctorArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(super::state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;
    let summary_only = args.summary;

    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => {
            let storage_metadata = match store.storage_metadata_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("storage metadata: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let storage_metadata_display = format!(
                "{} state-v{} instruction-v{}",
                storage_metadata.backend,
                storage_metadata.state_schema_version,
                storage_metadata.instruction_schema_version
            );
            let state_spine = match store.state_spine_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("authoritative state spine: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_store = match store.task_store_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task store: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let run_graph = match store.run_graph_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("run graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let launcher_runtime_paths = match super::resolve_repo_root()
                .and_then(|project_root| super::doctor_launcher_summary_for_root(&project_root))
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("launcher/runtime paths: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let dependency_graph = match store.validate_task_graph().await {
                Ok(issues) if issues.is_empty() => issues,
                Ok(issues) => {
                    let first = issues.first().expect("issues is not empty");
                    eprintln!(
                        "dependency graph: failed ({} issue(s), first={} on {})",
                        issues.len(),
                        first.issue_type,
                        first.issue_id
                    );
                    return ExitCode::from(1);
                }
                Err(error) => {
                    eprintln!("dependency graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let boot_compatibility = match store.evaluate_boot_compatibility().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("boot compatibility: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_preflight = match store.evaluate_migration_preflight().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration preflight: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_receipts = match store.migration_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration receipts: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_task_reconciliation = match store.latest_task_reconciliation_summary().await
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_reconciliation_rollup = match store.task_reconciliation_rollup().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation rollup: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let snapshot_bridge = match store.taskflow_snapshot_bridge_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("taskflow snapshot bridge: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let runtime_consumption = match super::runtime_consumption_summary(store.root()) {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("runtime consumption: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_final_snapshot_path =
                match super::latest_final_runtime_consumption_snapshot_path(store.root()) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("runtime consumption: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let latest_recorded_final_snapshot_path =
                match super::runtime_consumption_state::latest_recorded_final_runtime_consumption_snapshot_path(store.root()) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("runtime consumption: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let protocol_binding = match store.protocol_binding_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("protocol binding: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_status = match store.latest_run_graph_status().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph status: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_recovery = match store.latest_run_graph_recovery_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph recovery: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_checkpoint =
                match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph checkpoint: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let latest_run_graph_gate = match store.latest_run_graph_gate_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph gate: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_dispatch_receipt =
                match store.latest_run_graph_dispatch_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph dispatch receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let mut root_session_write_guard =
                crate::status_surface_write_guard::root_session_write_guard_summary_from_snapshot_path(
                    latest_final_snapshot_path
                        .as_deref()
                        .or(runtime_consumption.latest_snapshot_path.as_deref()),
                );
            root_session_write_guard =
                crate::status_surface_write_guard::merge_live_exception_takeover_write_guard(
                    root_session_write_guard,
                    latest_run_graph_dispatch_receipt.as_ref(),
                    latest_run_graph_recovery.as_ref(),
                );
            let effective_instruction_bundle = match store.active_instruction_root().await {
                Ok(root_artifact_id) => match store
                    .inspect_effective_instruction_bundle(&root_artifact_id)
                    .await
                {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        eprintln!("effective instruction bundle: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                Err(error) => {
                    eprintln!("active instruction root: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_effective_bundle_receipt =
                match store.latest_effective_bundle_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest effective bundle receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let effective_bundle_receipt_id = selected_effective_bundle_receipt_id(
                &effective_instruction_bundle,
                latest_effective_bundle_receipt.as_ref(),
            );
            let (trace_evidence, trace_evidence_blocker_codes, trace_evidence_next_actions) =
                build_trace_evidence_summary(
                    latest_task_reconciliation.as_ref(),
                    &runtime_consumption,
                    latest_run_graph_dispatch_receipt.as_ref(),
                    &protocol_binding,
                    &effective_instruction_bundle,
                    effective_bundle_receipt_id.as_str(),
                );
            let retrieval_trust_signal =
                super::runtime_consumption_state::latest_admissible_retrieval_trust_signal(
                    &runtime_consumption,
                    latest_final_snapshot_path.as_deref(),
                    protocol_binding.latest_receipt_id.as_deref(),
                );

            if as_json {
                let evidence_snapshot_path = latest_final_snapshot_path
                    .as_deref()
                    .or(runtime_consumption.latest_snapshot_path.as_deref());
                let operator_blocker_codes = doctor_operator_blocker_codes(
                    &dependency_graph,
                    &boot_compatibility,
                    &migration_preflight,
                    &protocol_binding,
                    latest_final_snapshot_path.as_deref(),
                    &runtime_consumption,
                    latest_recorded_final_snapshot_path.as_deref(),
                    &root_session_write_guard,
                    latest_run_graph_recovery.as_ref(),
                    latest_run_graph_gate.as_ref(),
                    latest_run_graph_dispatch_receipt.as_ref(),
                    trace_evidence_blocker_codes,
                );
                let operator_status = if operator_blocker_codes.is_empty() {
                    "pass"
                } else {
                    "blocked"
                };
                let mut operator_next_actions = doctor_operator_next_actions(
                    &operator_blocker_codes,
                    &boot_compatibility,
                    &migration_preflight,
                );
                operator_next_actions.extend(trace_evidence_next_actions);
                if let Some(error) = operator_contracts_consistency_error(
                    operator_status,
                    &operator_blocker_codes,
                    &operator_next_actions,
                ) {
                    eprintln!("doctor json contract: failed ({error})");
                    return ExitCode::from(1);
                }
                let operator_artifact_refs = serde_json::json!({
                    "runtime_consumption_latest_snapshot_path": evidence_snapshot_path,
                    "latest_run_graph_dispatch_receipt_id": latest_run_graph_dispatch_receipt
                        .as_ref()
                        .map(|receipt| receipt.run_id.clone()),
                    "protocol_binding_latest_receipt_id": protocol_binding.latest_receipt_id,
                    "retrieval_trust_signal": retrieval_trust_signal,
                    "latest_task_reconciliation_receipt_id": latest_task_reconciliation
                        .as_ref()
                        .map(|receipt| receipt.receipt_id.clone()),
                    "effective_instruction_bundle_receipt_id": effective_bundle_receipt_id,
                    "root_session_write_guard_status": root_session_write_guard["status"].clone(),
                });
                let operator_contracts = render_operator_contract_envelope(
                    operator_status,
                    operator_blocker_codes,
                    operator_next_actions,
                    operator_artifact_refs,
                );
                let summary_json = if summary_only {
                    serde_json::json!({
                        "surface": "vida doctor",
                        "view": "summary",
                        "status": operator_contracts["status"].clone(),
                        "trace_id": operator_contracts["trace_id"].clone(),
                        "workflow_class": operator_contracts["workflow_class"].clone(),
                        "risk_tier": operator_contracts["risk_tier"].clone(),
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
                        "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        "shared_fields": {
                            "contract_id": "release-1-shared-fields",
                            "schema_version": "release-1-v1",
                            "trace_id": operator_contracts["trace_id"].clone(),
                            "workflow_class": operator_contracts["workflow_class"].clone(),
                            "risk_tier": operator_contracts["risk_tier"].clone(),
                            "status": operator_contracts["status"].clone(),
                            "blocker_codes": operator_contracts["blocker_codes"].clone(),
                            "next_actions": operator_contracts["next_actions"].clone(),
                            "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        },
                        "operator_contracts": operator_contracts,
                        "storage_metadata_display": storage_metadata_display,
                        "dependency_graph": {
                            "issue_count": dependency_graph.len(),
                        },
                        "boot_compatibility": {
                            "classification": boot_compatibility.classification,
                            "next_step": boot_compatibility.next_step,
                        },
                        "runtime_consumption": runtime_consumption,
                        "root_session_write_guard": root_session_write_guard,
                        "protocol_binding": protocol_binding,
                        "trace_evidence": trace_evidence.clone(),
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "effective_instruction_bundle": {
                            "root_artifact_id": effective_instruction_bundle.root_artifact_id,
                            "receipt_id": effective_bundle_receipt_id,
                            "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
                        },
                    })
                } else {
                    serde_json::json!({
                        "surface": "vida doctor",
                        "status": operator_contracts["status"].clone(),
                        "trace_id": operator_contracts["trace_id"].clone(),
                        "workflow_class": operator_contracts["workflow_class"].clone(),
                        "risk_tier": operator_contracts["risk_tier"].clone(),
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
                        "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        "shared_fields": {
                            "contract_id": "release-1-shared-fields",
                            "schema_version": "release-1-v1",
                            "trace_id": operator_contracts["trace_id"].clone(),
                            "workflow_class": operator_contracts["workflow_class"].clone(),
                            "risk_tier": operator_contracts["risk_tier"].clone(),
                            "status": operator_contracts["status"].clone(),
                            "blocker_codes": operator_contracts["blocker_codes"].clone(),
                            "next_actions": operator_contracts["next_actions"].clone(),
                            "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        },
                        "operator_contracts": operator_contracts,
                        "storage_metadata": {
                            "engine": storage_metadata.engine,
                            "backend": storage_metadata.backend,
                            "namespace": storage_metadata.namespace,
                            "database": storage_metadata.database,
                            "state_schema_version": storage_metadata.state_schema_version,
                            "instruction_schema_version": storage_metadata.instruction_schema_version,
                        },
                        "state_spine": {
                            "state_schema_version": state_spine.state_schema_version,
                            "entity_surface_count": state_spine.entity_surface_count,
                            "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                        },
                        "task_store": {
                            "total_count": task_store.total_count,
                            "open_count": task_store.open_count,
                            "in_progress_count": task_store.in_progress_count,
                            "closed_count": task_store.closed_count,
                            "epic_count": task_store.epic_count,
                            "ready_count": task_store.ready_count,
                        },
                        "run_graph": {
                            "execution_plan_count": run_graph.execution_plan_count,
                            "routed_run_count": run_graph.routed_run_count,
                            "governance_count": run_graph.governance_count,
                            "resumability_count": run_graph.resumability_count,
                            "reconciliation_count": run_graph.reconciliation_count,
                        },
                        "launcher_runtime_paths": launcher_runtime_paths,
                        "dependency_graph": {
                            "issue_count": dependency_graph.len(),
                        },
                        "boot_compatibility": {
                            "classification": boot_compatibility.classification,
                            "reasons": boot_compatibility.reasons,
                            "next_step": boot_compatibility.next_step,
                        },
                        "migration_preflight": {
                            "compatibility_class": canonical_compatibility_class_str(
                                &migration_preflight.compatibility_classification
                            ).unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
                            "migration_state": migration_preflight.migration_state,
                            "blockers": migration_preflight.blockers,
                            "source_version_tuple": migration_preflight.source_version_tuple,
                            "next_step": migration_preflight.next_step,
                        },
                        "migration_receipts": {
                            "compatibility_receipts": migration_receipts.compatibility_receipts,
                            "application_receipts": migration_receipts.application_receipts,
                            "verification_receipts": migration_receipts.verification_receipts,
                            "cutover_readiness_receipts": migration_receipts.cutover_readiness_receipts,
                            "rollback_notes": migration_receipts.rollback_notes,
                        },
                        "latest_task_reconciliation": latest_task_reconciliation,
                        "task_reconciliation_rollup": task_reconciliation_rollup,
                        "taskflow_snapshot_bridge": snapshot_bridge,
                        "runtime_consumption": runtime_consumption,
                        "root_session_write_guard": root_session_write_guard,
                        "protocol_binding": protocol_binding,
                        "trace_evidence": trace_evidence.clone(),
                        "latest_run_graph_status": latest_run_graph_status,
                        "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        "effective_instruction_bundle": {
                            "root_artifact_id": effective_instruction_bundle.root_artifact_id,
                            "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
                            "source_version_tuple": effective_instruction_bundle.source_version_tuple,
                            "receipt_id": effective_bundle_receipt_id,
                            "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
                        },
                        "storage_metadata_display": storage_metadata_display,
                    })
                };
                if let Some(error) = shared_operator_output_contract_parity_error(&summary_json) {
                    eprintln!("doctor json contract: failed ({error})");
                    return ExitCode::from(1);
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary_json)
                        .expect("doctor summary should render as json")
                );
                return ExitCode::SUCCESS;
            }

            super::print_surface_header(render, "vida doctor");
            super::print_surface_ok(render, "storage metadata", &storage_metadata_display);
            super::print_surface_ok(
                render,
                "authoritative state spine",
                &format!(
                    "state-v{}, {} entity surfaces, mutation root {}",
                    state_spine.state_schema_version,
                    state_spine.entity_surface_count,
                    state_spine.authoritative_mutation_root
                ),
            );
            super::print_surface_ok(render, "task store", &task_store.as_display());
            super::print_surface_ok(render, "run graph", &run_graph.as_display());
            super::print_surface_ok(
                render,
                "launcher/runtime paths",
                &format!(
                    "vida={}, project_root={}, taskflow_surface={}",
                    launcher_runtime_paths.vida,
                    launcher_runtime_paths.project_root,
                    launcher_runtime_paths.taskflow_surface
                ),
            );
            super::print_surface_ok(render, "dependency graph", "0 issues");
            super::print_surface_ok(
                render,
                "boot compatibility",
                &format!(
                    "{} ({})",
                    boot_compatibility.classification, boot_compatibility.next_step
                ),
            );
            super::print_surface_ok(
                render,
                "migration preflight",
                &format!(
                    "{} / {} ({})",
                    canonical_compatibility_class_str(
                        &migration_preflight.compatibility_classification
                    )
                    .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
                    migration_preflight.migration_state,
                    migration_preflight.next_step
                ),
            );
            super::print_surface_ok(
                render,
                "migration receipts",
                &migration_receipts.as_display(),
            );
            match latest_task_reconciliation {
                Some(receipt) => {
                    super::print_surface_ok(render, "task reconciliation", &receipt.as_display());
                }
                None => {
                    super::print_surface_ok(render, "task reconciliation", "none");
                }
            }
            super::print_surface_ok(
                render,
                "task reconciliation rollup",
                &task_reconciliation_rollup.as_display(),
            );
            super::print_surface_ok(
                render,
                "taskflow snapshot bridge",
                &snapshot_bridge.as_display(),
            );
            super::print_surface_ok(
                render,
                "runtime consumption",
                &runtime_consumption.as_display(),
            );
            super::print_surface_ok(
                render,
                "root session write guard",
                &match root_session_write_guard["reason"].as_str() {
                    Some(reason) => format!(
                        "{} ({reason})",
                        root_session_write_guard["status"]
                            .as_str()
                            .unwrap_or("unknown")
                    ),
                    None => root_session_write_guard["status"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                },
            );
            super::print_surface_ok(render, "protocol binding", &protocol_binding.as_display());
            super::print_surface_ok(
                render,
                "trace evidence",
                &trace_evidence_display(&trace_evidence),
            );
            match latest_run_graph_status {
                Some(status) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph status",
                        &status.as_display(),
                    );
                    super::print_surface_ok(
                        render,
                        "latest run graph delegation gate",
                        &status.delegation_gate().as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph status", "none");
                }
            }
            match latest_run_graph_recovery {
                Some(summary) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph recovery",
                        &summary.as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph recovery", "none");
                }
            }
            match latest_run_graph_checkpoint {
                Some(summary) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph checkpoint",
                        &summary.as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph checkpoint", "none");
                }
            }
            match latest_run_graph_gate {
                Some(summary) => {
                    super::print_surface_ok(render, "latest run graph gate", &summary.as_display());
                }
                None => {
                    super::print_surface_ok(render, "latest run graph gate", "none");
                }
            }
            super::print_surface_ok(
                render,
                "effective instruction bundle",
                &effective_instruction_bundle
                    .mandatory_chain_order
                    .join(" -> "),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_trace_evidence_summary, final_snapshot_missing_release_admission_evidence,
        selected_effective_bundle_receipt_id,
    };
    use crate::contract_profile_adapter::{
        operator_contracts_consistency_error as release1_operator_contracts_consistency_error,
        shared_operator_output_contract_parity_error,
    };
    use crate::operator_contracts::canonical_release1_operator_contract_status;

    #[test]
    fn release1_operator_contracts_consistency_accepts_blocked_with_actions() {
        let blocker_codes = vec!["recovery_readiness_blocked".to_string()];
        let next_actions = vec![
            "Inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff.".to_string(),
        ];
        assert_eq!(
            release1_operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions),
            None
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_blocked_without_actions() {
        let blocker_codes = vec!["recovery_readiness_blocked".to_string()];
        assert_eq!(
            release1_operator_contracts_consistency_error("blocked", &blocker_codes, &[]),
            Some(
                "operator contract inconsistency: status=blocked requires next_actions".to_string()
            )
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_unknown_status() {
        assert_eq!(
            release1_operator_contracts_consistency_error("unknown", &[], &[]),
            Some("operator contract inconsistency: unsupported status `unknown`".to_string())
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_accepts_ok_compat_without_blockers() {
        assert_eq!(
            release1_operator_contracts_consistency_error("ok", &[], &[]),
            None
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_normalizes_case_and_whitespace_status_drift() {
        assert_eq!(
            release1_operator_contracts_consistency_error(" PASS ", &[], &[]),
            None
        );
        assert_eq!(
            release1_operator_contracts_consistency_error(
                " blocked ",
                &["recovery_readiness_blocked".to_string()],
                &["Inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff.".to_string()],
            ),
            None
        );
        assert_eq!(
            release1_operator_contracts_consistency_error(" Ok ", &[], &[]),
            None
        );
    }

    #[test]
    fn canonical_release1_operator_contract_status_accepts_release1_and_legacy_statuses() {
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!("pass")),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!("blocked")),
            Some("blocked")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!("ok")),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!("blockk")),
            None
        );
    }

    #[test]
    fn final_snapshot_missing_release_admission_evidence_accepts_canonical_blocked_snapshot() {
        let snapshot_path = std::env::temp_dir().join(format!(
            "vida-doctor-final-snapshot-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let snapshot_path_str = snapshot_path
            .to_str()
            .expect("temp snapshot path should be valid utf-8");
        let operator_contracts = serde_json::json!({
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": ["incomplete_release_admission_operator_evidence"],
            "next_actions": ["Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."],
            "artifact_refs": {
                "retrieval_trust_signal": {
                    "source": "runtime_consumption_snapshot_index",
                    "citation": "runtime-consumption/final-healthy.json",
                    "freshness": "final",
                    "acl": "protocol-binding-receipt-id"
                }
            }
        });
        std::fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "blocked",
                "blocker_codes": ["incomplete_release_admission_operator_evidence"],
                "next_actions": ["Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."],
                "shared_fields": {
                    "status": "blocked",
                    "blocker_codes": ["incomplete_release_admission_operator_evidence"],
                    "next_actions": ["Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."]
                },
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "payload": {
                    "docflow_activation": {
                        "evidence": {
                            "registry": {"ok": true},
                            "check": {"ok": true},
                            "readiness": {"verdict": "ready"},
                        }
                    },
                    "closure_admission": {
                        "status": "blocked",
                        "admitted": false,
                        "blockers": ["closure_admission_block"],
                        "proof_surfaces": ["vida taskflow consume final"],
                    }
                },
                "operator_contracts": operator_contracts,
            })
            .to_string(),
        )
        .expect("final snapshot should be writable");

        assert!(
            !final_snapshot_missing_release_admission_evidence(snapshot_path_str),
            "canonical blocked final snapshot should satisfy release-admission evidence"
        );

        let _ = std::fs::remove_file(snapshot_path);
    }

    fn sample_trace_evidence_inputs() -> (
        Option<crate::state_store::TaskReconciliationSummary>,
        crate::runtime_consumption_state::RuntimeConsumptionSummary,
        Option<crate::state_store::RunGraphDispatchReceiptSummary>,
        crate::state_store::ProtocolBindingSummary,
        crate::state_store::EffectiveInstructionBundle,
    ) {
        let task_reconciliation = crate::state_store::TaskReconciliationSummary {
            receipt_id: "task-reconciliation-1".to_string(),
            operation: "replace_snapshot".to_string(),
            source_kind: "canonical_snapshot_file".to_string(),
            source_path: Some("/tmp/project/tasks.snapshot.jsonl".to_string()),
            task_count: 3,
            dependency_count: 2,
            stale_removed_count: 1,
            recorded_at: "2026-03-08T00:00:00Z".to_string(),
        };
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 2,
            bundle_snapshots: 1,
            bundle_check_snapshots: 0,
            final_snapshots: 1,
            latest_kind: Some("final".to_string()),
            latest_snapshot_path: Some(
                "/tmp/project/.vida/data/state/runtime-consumption/final-1.json".to_string(),
            ),
        };
        let run_graph_dispatch_receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: Some("vida task ensure".to_string()),
            dispatch_packet_path: Some("/tmp/project/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/project/result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida taskflow run-graph advance".to_string()),
            downstream_dispatch_note: Some("continue".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(
                "/tmp/project/downstream-result.json".to_string(),
            ),
            downstream_dispatch_trace_path: Some("/tmp/project/downstream-trace.json".to_string()),
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-08T00:00:00Z".to_string(),
        };
        let protocol_binding = crate::state_store::ProtocolBindingSummary {
            total_receipts: 1,
            total_bindings: 1,
            active_bindings: 1,
            script_bound_count: 0,
            rust_bound_count: 1,
            fully_runtime_bound_count: 1,
            unbound_count: 0,
            blocking_issue_count: 0,
            latest_receipt_id: Some("protocol-binding-receipt-1".to_string()),
            latest_scenario: Some("runtime_assurance".to_string()),
            latest_recorded_at: Some("2026-03-08T00:00:00Z".to_string()),
            primary_state_authority: Some("state_store".to_string()),
        };
        let effective_instruction_bundle = crate::state_store::EffectiveInstructionBundle {
            root_artifact_id: "root-artifact".to_string(),
            mandatory_chain_order: vec!["prepare".to_string(), "verify".to_string()],
            source_version_tuple: vec!["1".to_string(), "0".to_string()],
            projected_artifacts: vec![crate::state_store::EffectiveInstructionArtifact {
                artifact_id: "artifact-1".to_string(),
                version: 1,
                source_hash: "source-hash".to_string(),
                projected_hash: "projected-hash".to_string(),
                body: "body".to_string(),
            }],
            receipt_id: "effective-bundle-receipt-1".to_string(),
        };

        (
            Some(task_reconciliation),
            runtime_consumption,
            Some(run_graph_dispatch_receipt),
            protocol_binding,
            effective_instruction_bundle,
        )
    }

    #[test]
    fn trace_evidence_links_available_sources_and_passes() {
        let (
            latest_task_reconciliation,
            runtime_consumption,
            latest_run_graph_dispatch_receipt,
            protocol_binding,
            effective_instruction_bundle,
        ) = sample_trace_evidence_inputs();
        let effective_bundle_receipt_id =
            selected_effective_bundle_receipt_id(&effective_instruction_bundle, None);

        let (trace_evidence, blocker_codes, next_actions) = build_trace_evidence_summary(
            latest_task_reconciliation.as_ref(),
            &runtime_consumption,
            latest_run_graph_dispatch_receipt.as_ref(),
            &protocol_binding,
            &effective_instruction_bundle,
            effective_bundle_receipt_id.as_str(),
        );

        assert_eq!(trace_evidence["status"], "pass");
        assert!(blocker_codes.is_empty());
        assert!(next_actions.is_empty());
        assert_eq!(
            trace_evidence["root_trace"]["latest_task_reconciliation_receipt_id"],
            "task-reconciliation-1"
        );
        assert_eq!(
            trace_evidence["root_trace"]["latest_run_graph_dispatch_receipt_id"],
            "run-1"
        );
        assert_eq!(
            trace_evidence["root_trace"]["runtime_consumption_latest_snapshot_path"],
            "/tmp/project/.vida/data/state/runtime-consumption/final-1.json"
        );
        assert_eq!(
            trace_evidence["root_trace"]["protocol_binding_latest_receipt_id"],
            "protocol-binding-receipt-1"
        );
        assert_eq!(
            trace_evidence["evaluation_evidence"]["effective_instruction_bundle"]["receipt_id"],
            "effective-bundle-receipt-1"
        );
    }

    #[test]
    fn trace_evidence_prefers_persisted_effective_bundle_receipt_id_when_available() {
        let (
            latest_task_reconciliation,
            runtime_consumption,
            latest_run_graph_dispatch_receipt,
            protocol_binding,
            effective_instruction_bundle,
        ) = sample_trace_evidence_inputs();
        let latest_effective_bundle_receipt = crate::state_store::EffectiveBundleReceiptSummary {
            receipt_id: "effective-bundle-receipt-persisted".to_string(),
            root_artifact_id: "root-artifact".to_string(),
            artifact_count: 1,
        };
        let effective_bundle_receipt_id = selected_effective_bundle_receipt_id(
            &effective_instruction_bundle,
            Some(&latest_effective_bundle_receipt),
        );

        let (trace_evidence, blocker_codes, next_actions) = build_trace_evidence_summary(
            latest_task_reconciliation.as_ref(),
            &runtime_consumption,
            latest_run_graph_dispatch_receipt.as_ref(),
            &protocol_binding,
            &effective_instruction_bundle,
            effective_bundle_receipt_id.as_str(),
        );

        assert_eq!(trace_evidence["status"], "pass");
        assert!(blocker_codes.is_empty());
        assert!(next_actions.is_empty());
        assert_eq!(
            trace_evidence["root_trace"]["effective_instruction_bundle_receipt_id"],
            "effective-bundle-receipt-persisted"
        );
        assert_eq!(
            trace_evidence["evaluation_evidence"]["effective_instruction_bundle"]["receipt_id"],
            "effective-bundle-receipt-persisted"
        );
    }

    #[test]
    fn trace_evidence_blocks_when_lane_receipt_is_missing() {
        let (
            latest_task_reconciliation,
            runtime_consumption,
            _latest_run_graph_dispatch_receipt,
            protocol_binding,
            effective_instruction_bundle,
        ) = sample_trace_evidence_inputs();
        let effective_bundle_receipt_id =
            selected_effective_bundle_receipt_id(&effective_instruction_bundle, None);

        let (trace_evidence, blocker_codes, next_actions) = build_trace_evidence_summary(
            latest_task_reconciliation.as_ref(),
            &runtime_consumption,
            None,
            &protocol_binding,
            &effective_instruction_bundle,
            effective_bundle_receipt_id.as_str(),
        );

        assert_eq!(trace_evidence["status"], "blocked");
        assert!(blocker_codes.iter().any(|code| code == "trace_missing"));
        assert!(!next_actions.is_empty());
        assert_eq!(
            trace_evidence["lane_receipts"]["latest_run_graph_dispatch_receipt"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_accepts_mirrored_payload() {
        let summary_json = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": []
            },
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": []
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            None
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_rejects_mismatch() {
        let summary_json = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": []
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["protocol_binding_blocking_issues"],
                "next_actions": ["Run `vida taskflow protocol-binding check --json` and clear blockers."]
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_accepts_status_case_and_whitespace_drift() {
        let summary_json = serde_json::json!({
            "status": " PASS ",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": " ok ",
                "blocker_codes": [],
                "next_actions": []
            },
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": []
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            None
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_accepts_next_actions_case_and_whitespace_drift() {
        let summary_json = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["recovery_readiness_blocked"],
            "next_actions": ["  Inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff.  "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["recovery_readiness_blocked"],
                "next_actions": ["inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff."]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["recovery_readiness_blocked"],
                "next_actions": ["INSPECT `VIDA TASKFLOW RECOVERY LATEST --JSON`, THEN RUN `VIDA TASKFLOW CONSUME CONTINUE --JSON` AFTER `RECOVERY_READY=TRUE` IS PROVEN FOR RESUME/ROLLBACK HANDOFF."]
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            None
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_rejects_noncanonical_mirrored_string_entries() {
        let summary_json = serde_json::json!({
            "status": "blocked",
            "blocker_codes": [" pending_lane_evidence "],
            "next_actions": [" "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": [" pending_lane_evidence "],
                "next_actions": [" "]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": [" pending_lane_evidence "],
                "next_actions": [" "]
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_rejects_case_drifted_blocker_codes() {
        let summary_json = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["MISSING_PROTOCOL_BINDING_RECEIPT"],
            "next_actions": ["Run `vida taskflow protocol-binding sync --json`"],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["MISSING_PROTOCOL_BINDING_RECEIPT"],
                "next_actions": ["Run `vida taskflow protocol-binding sync --json`"]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["MISSING_PROTOCOL_BINDING_RECEIPT"],
                "next_actions": ["Run `vida taskflow protocol-binding sync --json`"]
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_rejects_whitespace_only_mirrored_string_entries() {
        let summary_json = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["   "],
            "next_actions": ["   "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["   "],
                "next_actions": ["   "]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["   "],
                "next_actions": ["   "]
            }
        });

        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }
}
