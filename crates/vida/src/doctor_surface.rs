use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::contract_profile_adapter::{
    blocker_code_str, canonical_blocker_code_list, canonical_compatibility_class_str,
    classify_compatibility_boundary, shared_operator_output_contract_parity_error, BlockerCode,
    CompatibilityBoundary, CompatibilityClass,
};
use crate::status_surface::{first_non_empty_artifact_ref, StatusRunGraphArtifactSource};

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    BlockerCode::UnsupportedArchitectureReservedWorkflowBoundary.as_str();
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str = "Clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const CLOSED_TASK_ACTIVE_RUN_PROJECTION_MISMATCH_BLOCKER: &str =
    "closed_task_active_run_projection_mismatch";
const DOCTOR_SURFACE_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
fn governance_projection_blocker_codes(
    principal_delegation: Option<&crate::state_store::RunGraphPrincipalDelegationProjection>,
    memory_governance: Option<&crate::state_store::RunGraphMemoryGovernanceProjection>,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if let Some(projection) = principal_delegation {
        blocker_codes.extend(projection.blocker_codes.iter().cloned());
    }
    if let Some(projection) = memory_governance {
        blocker_codes.extend(projection.blocker_codes.iter().cloned());
    }
    canonical_blocker_code_list(blocker_codes.iter().map(String::as_str))
}

fn governance_projection_next_actions(
    operator_blocker_codes: &[String],
    principal_delegation: Option<&crate::state_store::RunGraphPrincipalDelegationProjection>,
    memory_governance: Option<&crate::state_store::RunGraphMemoryGovernanceProjection>,
) -> Vec<String> {
    let mut next_actions = Vec::new();
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::DelegationChainBroken))
        && principal_delegation.is_some()
    {
        next_actions.push(
            "Refresh the active run-graph delegation projection so doctor can prove explicit delegator/delegatee linkage and audit evidence."
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::ApprovalRequired))
        && memory_governance.is_some_and(|projection| projection.governance_required)
    {
        next_actions.push(
            "Record approval linkage for the active memory-governed run before continuing correction/deletion work."
                .to_string(),
        );
    }
    if memory_governance.is_some_and(|projection| projection.enforcement_state == "blocked") {
        next_actions.push(
            "Materialize consent and TTL linkage in the active run-graph handoff before rerunning `vida doctor`."
                .to_string(),
        );
    }
    next_actions
}

fn is_unsupported_architecture_reserved_workflow_boundary(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "unsupported" | "architecture_reserved" | "unsupported_architecture_reserved"
    )
}

fn run_graph_status_has_unsupported_architecture_reserved_workflow_boundary(
    status: &crate::state_store::RunGraphStatus,
) -> bool {
    is_unsupported_architecture_reserved_workflow_boundary(&status.policy_gate)
        || is_unsupported_architecture_reserved_workflow_boundary(&status.context_state)
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
    !crate::release1_contracts::release_admission_operator_evidence_snapshot(&summary_json)
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

fn missing_instruction_runtime_state_bundle() -> crate::state_store::EffectiveInstructionBundle {
    crate::state_store::EffectiveInstructionBundle {
        root_artifact_id: "missing_instruction_runtime_state".to_string(),
        mandatory_chain_order: Vec::new(),
        source_version_tuple: Vec::new(),
        projected_artifacts: Vec::new(),
        receipt_id: String::new(),
    }
}

fn terminal_task_active_run_matches_effective_run(
    terminal: &crate::state_store::RunGraphStatus,
    current_session_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    current_session_run_graph_status
        .or(latest_run_graph_status)
        .is_some_and(|status| {
            crate::taskflow_run_graph_task_authority::terminal_task_active_status_matches_current_run(
                Some(status),
                terminal,
            )
        })
}

fn trace_evidence_blocker_codes(
    latest_task_reconciliation: Option<&crate::state_store::TaskReconciliationSummary>,
    runtime_consumption: &crate::runtime_consumption_state::RuntimeConsumptionSummary,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    protocol_binding: &crate::state_store::ProtocolBindingSummary,
    effective_instruction_bundle: &crate::state_store::EffectiveInstructionBundle,
    effective_bundle_receipt_id: &str,
    idle_terminal_run: bool,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();

    let missing_root_trace = if idle_terminal_run {
        runtime_consumption.total_snapshots == 0 || protocol_binding.total_receipts == 0
    } else {
        latest_task_reconciliation.is_none()
            || runtime_consumption.total_snapshots == 0
            || latest_run_graph_dispatch_receipt.is_none()
            || protocol_binding.total_receipts == 0
    };
    if missing_root_trace {
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
    idle_terminal_run: bool,
) -> (serde_json::Value, Vec<String>, Vec<String>) {
    let blocker_codes = trace_evidence_blocker_codes(
        latest_task_reconciliation,
        runtime_consumption,
        latest_run_graph_dispatch_receipt,
        protocol_binding,
        effective_instruction_bundle,
        effective_bundle_receipt_id,
        idle_terminal_run,
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
    latest_terminal_task_active_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_run_graph_snapshot_inconsistent: bool,
    latest_run_graph_dispatch_receipt_checkpoint_leakage: bool,
    principal_delegation: Option<&crate::state_store::RunGraphPrincipalDelegationProjection>,
    memory_governance: Option<&crate::state_store::RunGraphMemoryGovernanceProjection>,
    operator_session_projection: &serde_json::Value,
    no_active_taskflow_work: bool,
    latest_run_graph_task_missing: bool,
    latest_run_graph_task_closed: bool,
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
    if latest_final_snapshot_path.is_none()
        && latest_recorded_final_snapshot_path
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
    if !no_active_taskflow_work
        && !latest_run_graph_task_missing
        && latest_run_graph_recovery
            .as_ref()
            .is_some_and(|summary| !summary.recovery_ready && !summary.is_terminal_closure())
    {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::RecoveryReadinessBlocked).to_string());
    }
    let unsupported_architecture_reserved_workflow_boundary =
        latest_run_graph_gate.as_ref().is_some_and(|summary| {
            is_unsupported_architecture_reserved_workflow_boundary(&summary.policy_gate)
                || is_unsupported_architecture_reserved_workflow_boundary(&summary.context_state)
        }) || latest_terminal_task_active_run_graph_status
            .as_ref()
            .is_some_and(|status| {
                run_graph_status_has_unsupported_architecture_reserved_workflow_boundary(status)
            });
    if unsupported_architecture_reserved_workflow_boundary {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::UnsupportedArchitectureReservedWorkflowBoundary)
                .to_string(),
        );
    }
    if (latest_run_graph_gate.is_some() || unsupported_architecture_reserved_workflow_boundary)
        && latest_run_graph_dispatch_receipt.is_none()
    {
        operator_blocker_codes
            .push(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER.to_string());
    }
    if latest_run_graph_snapshot_inconsistent {
        operator_blocker_codes
            .push(blocker_code_str(BlockerCode::RunGraphLatestSnapshotInconsistent).to_string());
    }
    if latest_run_graph_dispatch_receipt_checkpoint_leakage {
        operator_blocker_codes.push(
            blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage)
                .to_string(),
        );
    }
    if latest_run_graph_task_closed {
        operator_blocker_codes.push(CLOSED_TASK_ACTIVE_RUN_PROJECTION_MISMATCH_BLOCKER.to_string());
    }
    operator_blocker_codes.extend(governance_projection_blocker_codes(
        principal_delegation,
        memory_governance,
    ));
    operator_blocker_codes.extend(
        crate::operator_session_projection::projection_operator_blocker_codes(
            operator_session_projection,
        ),
    );
    operator_blocker_codes.extend(trace_evidence_blocker_codes);
    canonical_blocker_code_list(operator_blocker_codes.iter().map(String::as_str))
}

fn doctor_operator_next_actions(
    operator_blocker_codes: &[String],
    boot_compatibility: &crate::state_store::BootCompatibilitySummary,
    migration_preflight: &crate::state_store::MigrationPreflightSummary,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    principal_delegation: Option<&crate::state_store::RunGraphPrincipalDelegationProjection>,
    memory_governance: Option<&crate::state_store::RunGraphMemoryGovernanceProjection>,
) -> Vec<String> {
    let mut operator_next_actions: Vec<String> = Vec::new();
    if operator_blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::DependencyGraphIssues))
    {
        operator_next_actions
            .push(crate::status_surface_signals::task_validate_graph_next_action());
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
        .any(|code| code == blocker_code_str(BlockerCode::MissingRootSessionWriteGuard))
    {
        operator_next_actions
            .push(crate::status_surface_signals::missing_root_session_write_guard_next_action());
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
            crate::status_surface_signals::recovery_readiness_blocked_next_action_for_run(
                latest_run_graph_recovery.map(|summary| summary.run_id.as_str()),
            ),
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
        code == blocker_code_str(BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage)
    }) {
        operator_next_actions.push(
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()
                .to_string(),
        );
    }
    if operator_blocker_codes
        .iter()
        .any(|code| code == CLOSED_TASK_ACTIVE_RUN_PROJECTION_MISMATCH_BLOCKER)
    {
        operator_next_actions.push(
            crate::status_surface_signals::closed_task_active_run_projection_mismatch_next_action(),
        );
    }
    operator_next_actions.extend(governance_projection_next_actions(
        operator_blocker_codes,
        principal_delegation,
        memory_governance,
    ));
    operator_next_actions.extend(
        crate::operator_session_projection::projection_operator_next_actions(
            operator_blocker_codes,
        ),
    );
    operator_next_actions
}

fn recovery_readiness_target_evidence(
    recovery_readiness_blocked: bool,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> serde_json::Value {
    if !recovery_readiness_blocked {
        return serde_json::json!({
            "status": "not_blocked",
            "reason": "recovery_readiness_blocked is not present"
        });
    }

    match latest_run_graph_recovery {
        Some(summary) if !summary.run_id.trim().is_empty() => serde_json::json!({
            "status": "target_validated",
            "run_id": summary.run_id.clone(),
            "task_id": summary.task_id.clone(),
            "active_node": summary.active_node.clone(),
            "lifecycle_stage": summary.lifecycle_stage.clone(),
            "resume_status": summary.resume_status.clone(),
            "resume_target": summary.resume_target.clone(),
            "recovery_ready": summary.recovery_ready,
            "delegated_cycle_open": summary.delegation_gate.delegated_cycle_open,
            "delegated_cycle_state": summary.delegation_gate.delegated_cycle_state.clone(),
            "blocker_code": summary.delegation_gate.blocker_code.clone(),
        }),
        _ => serde_json::json!({
            "status": "no_target",
            "run_id": serde_json::Value::Null,
            "task_id": serde_json::Value::Null,
            "reason": "recovery_readiness_blocked has no validated run_id; recovery and continue commands require concrete run/task evidence"
        }),
    }
}

fn select_current_session_run_graph_dispatch_receipt<'a>(
    status_dispatch_receipt: Option<&'a crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_dispatch_receipt: Option<&'a crate::state_store::RunGraphDispatchReceiptSummary>,
    active_exception_takeover_dispatch_receipt: Option<
        &'a crate::state_store::RunGraphDispatchReceiptSummary,
    >,
) -> Option<&'a crate::state_store::RunGraphDispatchReceiptSummary> {
    status_dispatch_receipt
        .or(latest_dispatch_receipt)
        .or(active_exception_takeover_dispatch_receipt)
}

pub(crate) async fn run_doctor(args: super::DoctorArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(super::state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;
    let summary_only = args.summary;
    if crate::status_surface::state_store_lock_present(&state_dir) {
        return crate::status_surface::emit_degraded_read_lock_surface(
            "vida doctor",
            &state_dir,
            render,
            as_json,
            "another VIDA process still holds the authoritative datastore lock",
        );
    }

    match super::StateStore::open_existing_read_only_with_strict_timeout(
        state_dir.clone(),
        DOCTOR_SURFACE_LOCK_TIMEOUT,
    )
    .await
    {
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
            let dependency_graph = match doctor_dependency_graph_issues(&store).await {
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
            let boot_compatibility = match store.latest_boot_compatibility_summary().await {
                Ok(Some(summary)) => summary,
                Ok(None) => match store.evaluate_boot_compatibility().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("boot compatibility: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                Err(error) => {
                    eprintln!("boot compatibility: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_preflight = match store.latest_migration_preflight_summary().await {
                Ok(Some(summary)) => summary,
                Ok(None) => match store.evaluate_migration_preflight().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("migration preflight: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
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
                match crate::release1_contracts::latest_release_admission_operator_evidence_snapshot_path(store.root()) {
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
            let current_session_run_graph_status =
                match store.latest_run_graph_status_for_current_session().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("current-session run graph status: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let current_session_run_graph_run_id = current_session_run_graph_status
                .as_ref()
                .map(|status| status.run_id.as_str());
            let mut current_session_run_graph_dispatch_receipt_checkpoint_leakage = false;
            let current_session_run_graph_status_dispatch_receipt =
                match current_session_run_graph_status.as_ref() {
                    Some(status) => match store
                        .run_graph_dispatch_receipt_summary_for_status(status)
                        .await
                    {
                        Ok(summary) => summary,
                        Err(error) => {
                            if error
                                .to_string()
                                .contains("latest checkpoint evidence must share the same run_id")
                            {
                                current_session_run_graph_dispatch_receipt_checkpoint_leakage =
                                    true;
                                None
                            } else {
                                eprintln!(
                                    "current-session run graph dispatch receipt: failed ({error})"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    },
                    None => None,
                };
            let latest_run_graph_dispatch_receipt =
                match store.latest_run_graph_dispatch_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph dispatch receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let latest_run_graph_dispatch_receipt = if latest_run_graph_dispatch_receipt.is_none() {
                match crate::latest_final_runtime_consumption_dispatch_receipt_summary(store.root())
                {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!(
                            "latest runtime-consumption dispatch receipt fallback: failed ({error})"
                        );
                        return ExitCode::from(1);
                    }
                }
            } else {
                latest_run_graph_dispatch_receipt
            };
            let active_exception_takeover_dispatch_receipt =
                if current_session_run_graph_status_dispatch_receipt.is_none()
                    && latest_run_graph_dispatch_receipt.is_none()
                {
                    match store
                        .latest_active_exception_takeover_dispatch_receipt()
                        .await
                    {
                        Ok(receipt) => receipt
                            .map(crate::state_store::RunGraphDispatchReceiptSummary::from_receipt),
                        Err(error) => {
                            eprintln!(
                                "latest active exception takeover dispatch receipt: failed ({error})"
                            );
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    None
                };
            let current_session_run_graph_dispatch_receipt =
                select_current_session_run_graph_dispatch_receipt(
                    current_session_run_graph_status_dispatch_receipt.as_ref(),
                    latest_run_graph_dispatch_receipt.as_ref(),
                    active_exception_takeover_dispatch_receipt.as_ref(),
                );
            let current_session_effective_run_graph_run_id = current_session_run_graph_run_id
                .or_else(|| {
                    current_session_run_graph_dispatch_receipt
                        .map(|receipt| receipt.run_id.as_str())
                });
            let current_session_run_graph_checkpoint =
                match current_session_effective_run_graph_run_id {
                    Some(run_id) => match store.run_graph_checkpoint_summary(run_id).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("current-session run graph checkpoint: failed ({error})");
                            return ExitCode::from(1);
                        }
                    }
                    .into(),
                    None => None,
                };
            let current_session_run_graph_gate = match current_session_effective_run_graph_run_id {
                Some(run_id) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("current-session run graph gate: failed ({error})");
                        return ExitCode::from(1);
                    }
                }
                .into(),
                None => None,
            };
            let current_session_run_graph_recovery =
                match current_session_effective_run_graph_run_id {
                    Some(run_id) => match store.run_graph_recovery_summary(run_id).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("current-session run graph recovery: failed ({error})");
                            return ExitCode::from(1);
                        }
                    }
                    .into(),
                    None => None,
                };
            let latest_terminal_task_active_run_graph_status =
                match store.latest_terminal_task_active_run_graph_status().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest terminal-task run graph status: failed ({error})");
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
            let latest_run_graph_snapshot_inconsistent =
                !current_session_run_graph_dispatch_receipt_checkpoint_leakage
                    && !crate::state_store::latest_run_graph_evidence_snapshot_is_consistent(
                        current_session_effective_run_graph_run_id,
                        current_session_run_graph_recovery
                            .as_ref()
                            .map(|summary| summary.run_id.as_str()),
                        current_session_run_graph_checkpoint
                            .as_ref()
                            .map(|summary| summary.run_id.as_str()),
                        current_session_run_graph_gate
                            .as_ref()
                            .map(|summary| summary.run_id.as_str()),
                        current_session_run_graph_dispatch_receipt
                            .map(|receipt| receipt.run_id.as_str()),
                    );
            let (
                mut latest_run_graph_task_missing,
                mut latest_run_graph_task_closed,
                latest_run_graph_task_stale,
            ) = match latest_run_graph_status.as_ref() {
                Some(status) => match crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(&store, status).await {
                    Ok(verdict) => {
                        let missing = verdict.task_missing();
                        let closed = verdict.task_closed_stale_run();
                        (missing, closed, missing || closed)
                    }
                    Err(error) => {
                        eprintln!("latest run graph task authority: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                None => (false, false, false),
            };
            let terminal_task_active_run_graph_task_stale =
                match latest_terminal_task_active_run_graph_status.as_ref() {
                    Some(terminal)
                        if crate::taskflow_run_graph_task_authority::terminal_task_active_status_matches_current_run(
                            latest_run_graph_status.as_ref(),
                            terminal,
                        ) =>
                    {
                        match crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(&store, terminal).await {
                            Ok(verdict) => {
                                latest_run_graph_task_missing |= verdict.task_missing();
                                latest_run_graph_task_closed |= verdict.task_closed_stale_run();
                                verdict.task_missing() || verdict.task_closed_stale_run()
                            }
                            Err(error) => {
                                eprintln!(
                                    "latest terminal task-active run graph task authority: failed ({error})"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                    _ => false,
                };
            let latest_run_graph_task_stale =
                latest_run_graph_task_stale || terminal_task_active_run_graph_task_stale;
            let latest_run_graph_approval_receipt = match latest_run_graph_status.as_ref() {
                Some(status) => match store
                    .run_graph_approval_delegation_receipt(&status.run_id)
                    .await
                {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph approval/delegation receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                None => None,
            };
            let latest_principal_delegation = latest_run_graph_status.as_ref().map(|status| {
                status.principal_delegation_projection(
                    latest_run_graph_dispatch_receipt.as_ref(),
                    latest_run_graph_approval_receipt.as_ref(),
                )
            });
            let latest_memory_governance = latest_run_graph_status.as_ref().map(|status| {
                status.memory_governance_projection(latest_run_graph_approval_receipt.as_ref())
            });
            let mut root_session_write_guard =
                crate::status_surface_write_guard::root_session_write_guard_summary_from_snapshot_path(
                    latest_final_snapshot_path
                        .as_deref()
                        .or(runtime_consumption.latest_snapshot_path.as_deref()),
                );
            root_session_write_guard =
                crate::status_surface_write_guard::merge_live_exception_takeover_write_guard_with_task_authority(
                    root_session_write_guard,
                    store.root(),
                    latest_run_graph_dispatch_receipt.as_ref(),
                    latest_run_graph_recovery.as_ref(),
                    latest_run_graph_task_stale,
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
                Err(crate::state_store::StateStoreError::MissingInstructionRuntimeState) => {
                    missing_instruction_runtime_state_bundle()
                }
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
            let no_active_taskflow_work = task_store.open_count == 0
                && task_store.in_progress_count == 0
                && task_store.ready_count == 0;
            let idle_terminal_run = no_active_taskflow_work
                && latest_run_graph_status.as_ref().is_some_and(|status| {
                    status.status == "completed"
                        && status.lifecycle_stage == "closure_complete"
                        && status
                            .next_node
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .is_none()
                });
            let latest_run_graph_terminal_closure = match latest_run_graph_status.as_ref() {
                Some(status)
                    if crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(
                        status,
                    ) =>
                {
                    match store
                        .run_graph_terminal_closure_has_task_close_truth(status)
                        .await
                    {
                        Ok(has_truth) => has_truth,
                        Err(error) => {
                            eprintln!(
                                "latest run graph terminal closure evidence: failed ({error})"
                            );
                            return ExitCode::from(1);
                        }
                    }
                }
                _ => false,
            };
            let latest_run_graph_terminal_closure_without_receipt_truth =
                match latest_run_graph_status.as_ref() {
                    Some(status)
                        if crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(
                            status,
                        ) =>
                    {
                        let verdict = match crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(&store, status).await {
                            Ok(verdict) => verdict,
                            Err(error) => {
                                eprintln!("latest terminal run graph task authority: failed ({error})");
                                return ExitCode::from(1);
                            }
                        };
                        let task_closed = if verdict.task_closed_stale_run() {
                            true
                        } else {
                            match store.show_task(&status.task_id).await {
                                Ok(task) => {
                                    crate::state_store::StateStore::task_status_is_closed_like(
                                        &task.status,
                                    )
                                }
                                Err(crate::state_store::StateStoreError::MissingTask { .. }) => {
                                    false
                                }
                                Err(error) => {
                                    eprintln!(
                                        "latest terminal run graph task lookup: failed ({error})"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                        };
                        !latest_run_graph_terminal_closure && task_closed
                    }
                    _ => false,
                };
            let terminal_task_active_run_is_current =
                match latest_terminal_task_active_run_graph_status.as_ref() {
                    Some(terminal) if terminal_task_active_run_matches_effective_run(
                        terminal,
                        current_session_run_graph_status.as_ref(),
                        latest_run_graph_status.as_ref(),
                    ) =>
                    {
                        if crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(
                            terminal,
                        ) {
                            match store
                                .run_graph_terminal_closure_has_task_close_truth(terminal)
                                .await
                            {
                                Ok(has_truth) => !has_truth,
                                Err(error) => {
                                    eprintln!(
                                        "latest terminal task-active closure evidence: failed ({error})"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                        } else {
                            true
                        }
                    }
                    _ => false,
                };
            let unresolved_closed_task_active_run = !latest_run_graph_terminal_closure
                && latest_run_graph_task_closed
                || terminal_task_active_run_is_current
                || latest_run_graph_terminal_closure_without_receipt_truth;
            let (trace_evidence, trace_evidence_blocker_codes, trace_evidence_next_actions) =
                build_trace_evidence_summary(
                    latest_task_reconciliation.as_ref(),
                    &runtime_consumption,
                    latest_run_graph_dispatch_receipt.as_ref(),
                    &protocol_binding,
                    &effective_instruction_bundle,
                    effective_bundle_receipt_id.as_str(),
                    idle_terminal_run,
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
                let operator_session_projection =
                    match crate::operator_session_projection::build_operator_session_projection(
                        &store,
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(error) => {
                            eprintln!("doctor json: operator session projection failed ({error})");
                            return ExitCode::from(1);
                        }
                    };
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
                    latest_terminal_task_active_run_graph_status.as_ref(),
                    latest_run_graph_dispatch_receipt.as_ref(),
                    latest_run_graph_snapshot_inconsistent,
                    current_session_run_graph_dispatch_receipt_checkpoint_leakage,
                    latest_principal_delegation.as_ref(),
                    latest_memory_governance.as_ref(),
                    &operator_session_projection,
                    no_active_taskflow_work,
                    latest_run_graph_task_missing,
                    unresolved_closed_task_active_run,
                    trace_evidence_blocker_codes,
                );
                let recovery_readiness_blocked = operator_blocker_codes
                    .iter()
                    .any(|code| code == blocker_code_str(BlockerCode::RecoveryReadinessBlocked));
                let mut operator_next_actions = doctor_operator_next_actions(
                    &operator_blocker_codes,
                    &boot_compatibility,
                    &migration_preflight,
                    latest_run_graph_recovery.as_ref(),
                    latest_principal_delegation.as_ref(),
                    latest_memory_governance.as_ref(),
                );
                operator_next_actions.extend(trace_evidence_next_actions);
                let current_session_run_graph_dispatch_packet_path =
                    current_session_run_graph_dispatch_receipt
                        .and_then(|receipt| receipt.dispatch_packet_path.as_deref());
                let current_session_run_graph_packet_refs =
                    crate::status_surface::status_dispatch_packet_refs(
                        std::path::Path::new(&launcher_runtime_paths.project_root),
                        current_session_run_graph_dispatch_packet_path,
                    );
                let (
                    current_session_run_graph_artifact_run_id,
                    current_session_run_graph_artifact_run_id_source,
                ) = first_non_empty_artifact_ref(&[
                    (
                        current_session_run_graph_status
                            .as_ref()
                            .map(|status| status.run_id.as_str()),
                        StatusRunGraphArtifactSource::Status,
                    ),
                    (
                        current_session_run_graph_dispatch_receipt
                            .map(|receipt| receipt.run_id.as_str()),
                        StatusRunGraphArtifactSource::DispatchReceipt,
                    ),
                    (
                        current_session_run_graph_packet_refs.run_id.as_deref(),
                        StatusRunGraphArtifactSource::DispatchPacket,
                    ),
                ]);
                let (
                    current_session_run_graph_artifact_task_id,
                    current_session_run_graph_artifact_task_id_source,
                ) = first_non_empty_artifact_ref(&[
                    (
                        current_session_run_graph_status
                            .as_ref()
                            .map(|status| status.task_id.as_str()),
                        StatusRunGraphArtifactSource::Status,
                    ),
                    (
                        current_session_run_graph_packet_refs.task_id.as_deref(),
                        StatusRunGraphArtifactSource::DispatchPacket,
                    ),
                ]);
                let operator_artifact_refs = serde_json::json!({
                    "runtime_consumption_latest_snapshot_path": evidence_snapshot_path,
                    "current_session_run_graph_status_run_id": current_session_run_graph_artifact_run_id,
                    "current_session_run_graph_status_task_id": current_session_run_graph_artifact_task_id,
                    "current_session_run_graph_status_run_id_source": current_session_run_graph_artifact_run_id_source
                        .map(StatusRunGraphArtifactSource::as_str),
                    "current_session_run_graph_status_task_id_source": current_session_run_graph_artifact_task_id_source
                        .map(StatusRunGraphArtifactSource::as_str),
                    "current_session_run_graph_recovery_run_id": current_session_run_graph_recovery
                        .as_ref()
                        .map(|summary| summary.run_id.clone()),
                    "current_session_run_graph_checkpoint_run_id": current_session_run_graph_checkpoint
                        .as_ref()
                        .map(|summary| summary.run_id.clone()),
                    "current_session_run_graph_gate_run_id": current_session_run_graph_gate
                        .as_ref()
                        .map(|summary| summary.run_id.clone()),
                    "current_session_run_graph_dispatch_receipt_id": current_session_run_graph_dispatch_receipt
                        .map(|receipt| receipt.run_id.clone()),
                    "current_session_run_graph_dispatch_packet_path": current_session_run_graph_dispatch_packet_path,
                    "latest_run_graph_dispatch_receipt_id": latest_run_graph_dispatch_receipt
                        .as_ref()
                        .map(|receipt| receipt.run_id.clone()),
                    "protocol_binding_latest_receipt_id": protocol_binding.latest_receipt_id,
                    "retrieval_trust_signal": retrieval_trust_signal,
                    "recovery_readiness_target": recovery_readiness_target_evidence(
                        recovery_readiness_blocked,
                        latest_run_graph_recovery.as_ref()
                    ),
                    "latest_task_reconciliation_receipt_id": latest_task_reconciliation
                        .as_ref()
                        .map(|receipt| receipt.receipt_id.clone()),
                    "latest_run_graph_approval_receipt_id": latest_run_graph_approval_receipt
                        .as_ref()
                        .map(|receipt| receipt.receipt_id.clone()),
                    "latest_principal_delegation": latest_principal_delegation,
                    "latest_memory_governance": latest_memory_governance,
                    "effective_instruction_bundle_receipt_id": effective_bundle_receipt_id,
                    "root_session_write_guard_status": root_session_write_guard["status"].clone(),
                    "operator_session_projection": crate::operator_session_projection::projection_operator_artifact_refs(
                        &operator_session_projection
                    ),
                });
                let finalized =
                    match crate::release1_operator_output::finalize_release1_operator_truth(
                        operator_blocker_codes,
                        operator_next_actions,
                        operator_artifact_refs,
                    ) {
                        Ok(finalized) => finalized,
                        Err(error) => {
                            eprintln!("doctor json contract: failed ({error})");
                            return ExitCode::from(1);
                        }
                    };
                let operator_contracts = finalized.operator_contracts;
                let mut summary_json = if summary_only {
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
                        "shared_fields": finalized.shared_fields.clone(),
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
                        "latest_run_graph_approval_receipt": latest_run_graph_approval_receipt,
                        "latest_principal_delegation": latest_principal_delegation,
                        "latest_memory_governance": latest_memory_governance,
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
                        "shared_fields": finalized.shared_fields.clone(),
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
                        "latest_run_graph_approval_receipt": latest_run_graph_approval_receipt,
                        "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        "latest_principal_delegation": latest_principal_delegation,
                        "latest_memory_governance": latest_memory_governance,
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
                if let Some(object) = summary_json.as_object_mut() {
                    object.insert(
                        "operator_session_projection".to_string(),
                        operator_session_projection.clone(),
                    );
                    object.insert(
                        "current_session".to_string(),
                        operator_session_projection["current_session"].clone(),
                    );
                    object.insert(
                        "project_foreign_runs".to_string(),
                        operator_session_projection["project_foreign_runs"].clone(),
                    );
                    object.insert(
                        "project_foreign_blockers".to_string(),
                        operator_session_projection["project_foreign_blockers"].clone(),
                    );
                    object.insert(
                        "global_blockers".to_string(),
                        operator_session_projection["global_blockers"].clone(),
                    );
                    object.insert(
                        "claim_conflicts".to_string(),
                        operator_session_projection["claim_conflicts"].clone(),
                    );
                    object.insert(
                        "latest_terminal_task_active_run_graph_status".to_string(),
                        serde_json::to_value(&latest_terminal_task_active_run_graph_status)
                            .expect("terminal task active run graph status should serialize"),
                    );
                }
                if let Some(error) = shared_operator_output_contract_parity_error(&summary_json) {
                    eprintln!("doctor json contract: failed ({error})");
                    return ExitCode::from(1);
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary_json)
                        .expect("doctor summary should render as json")
                );
                crate::operator_projection_cache::write_json_projection(
                    store.root(),
                    doctor_json_projection_name(summary_only),
                    &summary_json,
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
                    if let Some(principal_delegation) = latest_principal_delegation.as_ref() {
                        super::print_surface_ok(
                            render,
                            "latest run graph principal delegation",
                            &principal_delegation.as_display(),
                        );
                    }
                    if let Some(memory_governance) = latest_memory_governance.as_ref() {
                        super::print_surface_ok(
                            render,
                            "latest run graph memory governance",
                            &memory_governance.as_display(),
                        );
                    }
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
            if super::StateStore::error_is_lock_contention(&error) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida doctor",
                    &state_dir,
                    render,
                    as_json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn doctor_json_projection_name(summary_only: bool) -> &'static str {
    if summary_only {
        "doctor-summary-v2-latest"
    } else {
        "doctor-full-latest"
    }
}

async fn doctor_dependency_graph_issues(
    store: &crate::StateStore,
) -> Result<Vec<crate::state_store::TaskGraphIssue>, crate::state_store::StateStoreError> {
    store.validate_task_graph().await
}

#[cfg(test)]
mod tests {
    use super::{
        build_trace_evidence_summary, doctor_json_projection_name, doctor_operator_blocker_codes,
        final_snapshot_missing_release_admission_evidence, selected_effective_bundle_receipt_id,
    };
    use crate::contract_profile_adapter::{
        operator_contracts_consistency_error, shared_operator_output_contract_parity_error,
    };
    use crate::release1_operator_output::canonical_release1_operator_contract_status;
    use std::fs;

    #[test]
    fn doctor_summary_projection_cache_key_is_shape_versioned() {
        assert_eq!(
            doctor_json_projection_name(true),
            "doctor-summary-v2-latest"
        );
        assert_eq!(doctor_json_projection_name(false), "doctor-full-latest");
    }

    #[test]
    fn doctor_current_session_dispatch_receipt_prefers_status_receipt() {
        let status_receipt = dispatch_receipt_summary_for_test("current-session-run");
        let final_receipt = dispatch_receipt_summary_for_test("final-runtime-consumption-run");
        let selected = super::select_current_session_run_graph_dispatch_receipt(
            Some(&status_receipt),
            Some(&final_receipt),
            None,
        )
        .expect("current-session status receipt should be selected");

        assert_eq!(selected.run_id, "current-session-run");
    }

    #[test]
    fn doctor_current_session_dispatch_receipt_prefers_latest_dispatch_before_exception() {
        let latest_receipt = dispatch_receipt_summary_for_test("latest-dispatch-run");
        let exception_receipt = dispatch_receipt_summary_for_test("stale-active-exception-run");
        let selected = super::select_current_session_run_graph_dispatch_receipt(
            None,
            Some(&latest_receipt),
            Some(&exception_receipt),
        )
        .expect("latest persisted dispatch receipt should be selected");

        assert_eq!(selected.run_id, "latest-dispatch-run");
    }

    fn dispatch_receipt_summary_for_test(
        run_id: &str,
    ) -> crate::state_store::RunGraphDispatchReceiptSummary {
        crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "orchestrator".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "runtime".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            dispatch_packet_path: None,
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
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "0".to_string(),
        }
    }

    #[test]
    fn release1_operator_contracts_consistency_accepts_blocked_with_actions() {
        let blocker_codes = vec!["recovery_readiness_blocked".to_string()];
        let next_actions = vec![
            "Inspect `vida taskflow recovery latest`, then run `vida taskflow consume continue` after `recovery_ready=true` is proven for resume/rollback handoff.".to_string(),
        ];
        assert_eq!(
            operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions),
            None
        );
    }

    #[tokio::test]
    async fn doctor_dependency_graph_uses_authoritative_store_over_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "vida-doctor-dependency-graph-authoritative-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("state store should open");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "authoritative-root",
                title: "Authoritative root",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("authoritative task should be created");
        store
            .refresh_task_snapshot()
            .await
            .expect("snapshot should be refreshed");
        let snapshot_path =
            crate::StateStore::canonical_task_snapshot_path_for_state_root(store.root());
        fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("canonical snapshot path should have a parent"),
        )
        .expect("snapshot parent should be writable");
        fs::write(
            &snapshot_path,
            r#"{"id":"snapshot-only","title":"Snapshot only","description":"","status":"open","priority":1,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"test","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":[],"dependencies":[{"issue_id":"snapshot-only","depends_on_id":"missing-target","type":"blocks","created_at":"2026-03-08T00:00:00Z","created_by":"test","metadata":"{}","thread_id":""}]}"#,
        )
        .expect("stale snapshot should be writable");

        let issues = super::doctor_dependency_graph_issues(&store)
            .await
            .expect("authoritative graph validation should run");

        assert!(issues.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_blocked_without_actions() {
        let blocker_codes = vec!["recovery_readiness_blocked".to_string()];
        assert_eq!(
            operator_contracts_consistency_error("blocked", &blocker_codes, &[]),
            Some(
                "operator contract inconsistency: status=blocked requires next_actions".to_string()
            )
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_unknown_status() {
        assert_eq!(
            operator_contracts_consistency_error("unknown", &[], &[]),
            Some("operator contract inconsistency: unsupported status `unknown`".to_string())
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_accepts_ok_compat_without_blockers() {
        assert_eq!(operator_contracts_consistency_error("ok", &[], &[]), None);
    }

    #[test]
    fn release1_operator_contracts_consistency_normalizes_case_and_whitespace_status_drift() {
        assert_eq!(
            operator_contracts_consistency_error(" PASS ", &[], &[]),
            None
        );
        assert_eq!(
            operator_contracts_consistency_error(
                " blocked ",
                &["recovery_readiness_blocked".to_string()],
                &["Inspect `vida taskflow recovery latest`, then run `vida taskflow consume continue` after `recovery_ready=true` is proven for resume/rollback handoff.".to_string()],
            ),
            None
        );
        assert_eq!(operator_contracts_consistency_error(" Ok ", &[], &[]), None);
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
                        "status": "pass",
                        "admitted": true,
                        "blockers": [],
                        "decision_owner": "doctor-test",
                        "decision_at": "2026-03-08T00:00:00Z",
                        "proof_surfaces": ["vida taskflow consume final"],
                        "evidence_table": [{
                            "requirement": "taskflow_bundle_check",
                            "status": "pass",
                            "evidence_refs": ["vida taskflow consume bundle check"],
                            "blockers": []
                        }, {
                            "requirement": "docflow_readiness",
                            "status": "pass",
                            "evidence_refs": ["vida taskflow consume final"],
                            "blockers": []
                        }, {
                            "requirement": "approved_design_packet",
                            "status": "pass",
                            "evidence_refs": ["vida docflow readiness"],
                            "blockers": []
                        }, {
                            "requirement": "spec_work_pool_dev_handoff",
                            "status": "pass",
                            "evidence_refs": ["vida taskflow run-graph dispatch-init"],
                            "blockers": []
                        }, {
                            "requirement": "execution_preparation",
                            "status": "pass",
                            "evidence_refs": ["vida agent-init"],
                            "blockers": []
                        }],
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
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
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
            false,
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
    fn claim_conflicts_block_doctor_operator_contracts() {
        let runtime_consumption = crate::runtime_consumption_state::RuntimeConsumptionSummary {
            total_snapshots: 1,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 1,
            latest_kind: Some("final".to_string()),
            latest_snapshot_path: Some("snapshot.json".to_string()),
        };
        let boot_compatibility = crate::state_store::BootCompatibilitySummary {
            classification: "backward_compatible".to_string(),
            reasons: vec![],
            next_step: "none".to_string(),
        };
        let migration_preflight = crate::state_store::MigrationPreflightSummary {
            contract_type: "operator_contracts".to_string(),
            schema_version: "release-1-v1".to_string(),
            compatibility_classification: "backward_compatible".to_string(),
            migration_state: "no_migration_required".to_string(),
            blockers: vec![],
            source_version_tuple: vec![],
            next_step: "none".to_string(),
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
        let root_session_write_guard = serde_json::json!({
            "status": "blocked_by_default",
            "activation_view_only_dispatch_blocker_active": false,
        });
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
                "conflict_domain": "path:crates/vida/src/doctor_surface.rs",
                "owned_paths": ["crates/vida/src/doctor_surface.rs"],
                "lease_mode": "exclusive",
                "status": "active",
                "blocker_codes": [],
            }],
        });

        let blockers = doctor_operator_blocker_codes(
            &[],
            &boot_compatibility,
            &migration_preflight,
            &protocol_binding,
            Some("snapshot.json"),
            &runtime_consumption,
            None,
            &root_session_write_guard,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            &operator_session_projection,
            true,
            false,
            false,
            vec![],
        );

        assert_eq!(blockers, vec!["conflict_domain_collision"]);
    }

    #[test]
    fn doctor_operator_contracts_block_on_latest_run_graph_snapshot_inconsistent() {
        let blocker_codes =
            vec![
                crate::blocker_code_str(crate::BlockerCode::RunGraphLatestSnapshotInconsistent)
                    .to_string(),
            ];
        let next_actions = super::doctor_operator_next_actions(
            &blocker_codes,
            &crate::state_store::BootCompatibilitySummary {
                classification: "backward_compatible".to_string(),
                reasons: vec![],
                next_step: "none".to_string(),
            },
            &crate::state_store::MigrationPreflightSummary {
                contract_type: "operator_contracts".to_string(),
                schema_version: "release-1-v1".to_string(),
                compatibility_classification: "backward_compatible".to_string(),
                migration_state: "no_migration_required".to_string(),
                blockers: vec![],
                source_version_tuple: vec![],
                next_step: "none".to_string(),
            },
            None,
            None,
            None,
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("concrete run/task/packet")));
        assert!(next_actions.iter().all(|action| !action.contains("--json")));
        assert_eq!(
            operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions),
            None
        );
    }

    #[test]
    fn terminal_task_active_run_matching_uses_current_session_before_global_latest() {
        let mut current = crate::taskflow_run_graph::default_run_graph_status(
            "run-current",
            "task-current",
            "implementation",
        );
        current.run_id = "run-current".to_string();
        let mut global = crate::taskflow_run_graph::default_run_graph_status(
            "run-global",
            "task-global",
            "implementation",
        );
        global.run_id = "run-global".to_string();
        let mut terminal = crate::taskflow_run_graph::default_run_graph_status(
            "run-terminal",
            "task-terminal",
            "implementation",
        );
        terminal.run_id = "run-terminal".to_string();

        assert!(!super::terminal_task_active_run_matches_effective_run(
            &terminal,
            Some(&current),
            Some(&global)
        ));

        terminal.run_id = "run-current".to_string();
        assert!(super::terminal_task_active_run_matches_effective_run(
            &terminal,
            Some(&current),
            Some(&global)
        ));

        terminal.run_id = "run-global".to_string();
        assert!(super::terminal_task_active_run_matches_effective_run(
            &terminal,
            None,
            Some(&global)
        ));
    }

    #[test]
    fn doctor_operator_contracts_explain_latest_run_graph_checkpoint_leakage() {
        let blocker_codes = vec![crate::blocker_code_str(
            crate::BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage,
        )
        .to_string()];
        let next_actions = super::doctor_operator_next_actions(
            &blocker_codes,
            &crate::state_store::BootCompatibilitySummary {
                classification: "backward_compatible".to_string(),
                reasons: vec![],
                next_step: "none".to_string(),
            },
            &crate::state_store::MigrationPreflightSummary {
                contract_type: "operator_contracts".to_string(),
                schema_version: "release-1-v1".to_string(),
                compatibility_classification: "backward_compatible".to_string(),
                migration_state: "no_migration_required".to_string(),
                blockers: vec![],
                source_version_tuple: vec![],
                next_step: "none".to_string(),
            },
            None,
            None,
            None,
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("checkpoint evidence")));
        assert!(next_actions.iter().all(|action| !action.contains("--json")));
        assert_eq!(
            operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions),
            None
        );
    }

    #[test]
    fn doctor_operator_next_actions_use_default_human_commands_from_shared_catalog() {
        let blocker_codes = vec![
            crate::blocker_code_str(crate::BlockerCode::ProtocolBindingBlockingIssues).to_string(),
            crate::blocker_code_str(
                crate::BlockerCode::MissingRetrievalTrustSourceOperatorEvidence,
            )
            .to_string(),
            crate::blocker_code_str(
                crate::BlockerCode::MissingRetrievalTrustSignalOperatorEvidence,
            )
            .to_string(),
            crate::blocker_code_str(crate::BlockerCode::MissingRetrievalTrustOperatorEvidence)
                .to_string(),
            crate::blocker_code_str(crate::BlockerCode::MissingRootSessionWriteGuard).to_string(),
            crate::blocker_code_str(crate::BlockerCode::RecoveryReadinessBlocked).to_string(),
            "missing_run_graph_dispatch_receipt_operator_evidence".to_string(),
            "closed_task_active_run_projection_mismatch".to_string(),
        ];
        let next_actions = super::doctor_operator_next_actions(
            &blocker_codes,
            &crate::state_store::BootCompatibilitySummary {
                classification: "backward_compatible".to_string(),
                reasons: vec![],
                next_step: "none".to_string(),
            },
            &crate::state_store::MigrationPreflightSummary {
                contract_type: "operator_contracts".to_string(),
                schema_version: "release-1-v1".to_string(),
                compatibility_classification: "backward_compatible".to_string(),
                migration_state: "no_migration_required".to_string(),
                blockers: vec![],
                source_version_tuple: vec![],
                next_step: "none".to_string(),
            },
            None,
            None,
            None,
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow protocol-binding check")));
        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow consume bundle check")));
        let no_target_recovery_action = next_actions
            .iter()
            .find(|action| action.contains("no validated run_id"))
            .expect("recovery readiness without target should produce no-target action");
        assert!(!no_target_recovery_action.contains("vida taskflow recovery latest"));
        assert!(!no_target_recovery_action.contains("vida taskflow consume continue"));
        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida task reconcile-closed-runs --limit 25")));
        assert!(next_actions.iter().all(|action| !action.contains("--json")));
        assert_eq!(
            operator_contracts_consistency_error("blocked", &blocker_codes, &next_actions),
            None
        );
    }

    #[test]
    fn doctor_operator_next_actions_target_recovery_readiness_run() {
        let blocker_codes =
            vec![crate::blocker_code_str(crate::BlockerCode::RecoveryReadinessBlocked).to_string()];
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-doctor".to_string(),
            task_id: "task-doctor".to_string(),
            active_node: "analyst".to_string(),
            lifecycle_stage: "analyst_blocked".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "dispatch.analyst".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analyst".to_string(),
                lifecycle_stage: "analyst_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let next_actions = super::doctor_operator_next_actions(
            &blocker_codes,
            &crate::state_store::BootCompatibilitySummary {
                classification: "backward_compatible".to_string(),
                reasons: vec![],
                next_step: "none".to_string(),
            },
            &crate::state_store::MigrationPreflightSummary {
                contract_type: "operator_contracts".to_string(),
                schema_version: "release-1-v1".to_string(),
                compatibility_classification: "backward_compatible".to_string(),
                migration_state: "no_migration_required".to_string(),
                blockers: vec![],
                source_version_tuple: vec![],
                next_step: "none".to_string(),
            },
            Some(&recovery),
            None,
            None,
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery status run-doctor")));
        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow consume continue --run-id run-doctor")));
        assert!(next_actions.iter().all(|action| !action.contains("--json")));
    }

    #[test]
    fn doctor_recovery_readiness_evidence_explains_missing_target() {
        let evidence = super::recovery_readiness_target_evidence(true, None);

        assert_eq!(evidence["status"], "no_target");
        assert_eq!(evidence["run_id"], serde_json::Value::Null);
        assert_eq!(evidence["task_id"], serde_json::Value::Null);
        assert!(evidence["reason"]
            .as_str()
            .expect("reason should be string")
            .contains("no validated run_id"));
    }

    #[test]
    fn doctor_recovery_readiness_evidence_exposes_validated_target() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-doctor".to_string(),
            task_id: "task-doctor".to_string(),
            active_node: "analyst".to_string(),
            lifecycle_stage: "analyst_blocked".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "dispatch.analyst".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analyst".to_string(),
                lifecycle_stage: "analyst_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let evidence = super::recovery_readiness_target_evidence(true, Some(&recovery));

        assert_eq!(evidence["status"], "target_validated");
        assert_eq!(evidence["run_id"], "run-doctor");
        assert_eq!(evidence["task_id"], "task-doctor");
        assert_eq!(evidence["resume_target"], "dispatch.analyst");
        assert_eq!(evidence["blocker_code"], "open_delegated_cycle");
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
            false,
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
            false,
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
                "next_actions": ["Run `vida taskflow protocol-binding check` and clear blockers."]
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
            "next_actions": ["  Inspect `vida taskflow recovery latest`, then run `vida taskflow consume continue` after `recovery_ready=true` is proven for resume/rollback handoff.  "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["recovery_readiness_blocked"],
                "next_actions": ["inspect `vida taskflow recovery latest`, then run `vida taskflow consume continue` after `recovery_ready=true` is proven for resume/rollback handoff."]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["recovery_readiness_blocked"],
                "next_actions": ["INSPECT `VIDA TASKFLOW RECOVERY LATEST`, THEN RUN `VIDA TASKFLOW CONSUME CONTINUE` AFTER `RECOVERY_READY=TRUE` IS PROVEN FOR RESUME/ROLLBACK HANDOFF."]
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
            "next_actions": ["Run `vida taskflow protocol-binding sync`"],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["MISSING_PROTOCOL_BINDING_RECEIPT"],
                "next_actions": ["Run `vida taskflow protocol-binding sync`"]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["MISSING_PROTOCOL_BINDING_RECEIPT"],
                "next_actions": ["Run `vida taskflow protocol-binding sync`"]
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
