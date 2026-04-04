use std::process::ExitCode;

use crate::operator_contracts::{
    canonical_release1_operator_contract_status, release1_operator_contracts_consistency_error,
    shared_operator_output_contract_parity_error,
};
use crate::release1_contracts::{
    canonical_compatibility_class_str, classify_compatibility_boundary, CompatibilityBoundary,
    CompatibilityClass,
};

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str =
    "Clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

fn is_unsupported_architecture_reserved_workflow_boundary(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "unsupported" | "architecture_reserved" | "unsupported_architecture_reserved"
    )
}

fn retrieval_trust_signal(
    source: Option<&str>,
    citation: Option<&str>,
    freshness: Option<&str>,
    acl: Option<&str>,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "source": source?,
        "citation": citation?,
        "freshness": freshness?,
        "acl": acl?,
    }))
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
    let operator_contracts = match summary_json.get("operator_contracts") {
        Some(value) => value,
        None => return true,
    };
    let status_ok = canonical_release1_operator_contract_status(&summary_json["status"]).is_some();
    let operator_status_ok =
        canonical_release1_operator_contract_status(&operator_contracts["status"]).is_some();
    if !status_ok || !operator_status_ok {
        return true;
    }
    let blockers_ok = operator_contracts
        .get("blocker_codes")
        .and_then(|value| value.as_array())
        .is_some();
    let next_actions_ok = operator_contracts
        .get("next_actions")
        .and_then(|value| value.as_array())
        .is_some();
    let trust_signal_ok = operator_contracts
        .get("artifact_refs")
        .and_then(|refs| refs.get("retrieval_trust_signal"))
        .is_some_and(|value| value.is_object());
    !(blockers_ok && next_actions_ok && trust_signal_ok)
}

pub(crate) async fn run_doctor(args: super::DoctorArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(super::state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;

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

            if as_json {
                let mut operator_blocker_codes: Vec<String> = Vec::new();
                if !dependency_graph.is_empty() {
                    operator_blocker_codes.push("dependency_graph_issues".to_string());
                }
                match classify_compatibility_boundary(&boot_compatibility.classification) {
                    CompatibilityBoundary::Compatible => {}
                    CompatibilityBoundary::BlockingSupported => {
                        operator_blocker_codes
                            .push("boot_compatibility_not_compatible".to_string());
                    }
                    CompatibilityBoundary::Unsupported => {
                        operator_blocker_codes
                            .push("boot_compatibility_unsupported_boundary".to_string());
                    }
                }
                match classify_compatibility_boundary(
                    &migration_preflight.compatibility_classification,
                ) {
                    CompatibilityBoundary::Compatible => {}
                    CompatibilityBoundary::BlockingSupported => {
                        operator_blocker_codes
                            .push("migration_preflight_not_compatible".to_string());
                    }
                    CompatibilityBoundary::Unsupported => {
                        operator_blocker_codes
                            .push("migration_preflight_unsupported_boundary".to_string());
                    }
                }
                if migration_requires_action(&migration_preflight.migration_state) {
                    operator_blocker_codes.push("migration_required".to_string());
                }
                if protocol_binding.blocking_issue_count > 0 {
                    operator_blocker_codes.push("protocol_binding_blocking_issues".to_string());
                }
                let retrieval_trust_source = runtime_consumption
                    .latest_snapshot_path
                    .as_deref()
                    .map(|_| "runtime_consumption_snapshot_index");
                let retrieval_trust_signal = retrieval_trust_signal(
                    retrieval_trust_source,
                    runtime_consumption.latest_snapshot_path.as_deref(),
                    runtime_consumption.latest_kind.as_deref(),
                    protocol_binding.latest_receipt_id.as_deref(),
                );
                if retrieval_trust_source.is_none() {
                    operator_blocker_codes
                        .push("missing_retrieval_trust_source_operator_evidence".to_string());
                }
                if retrieval_trust_signal.is_none() {
                    operator_blocker_codes
                        .push("missing_retrieval_trust_signal_operator_evidence".to_string());
                }
                if runtime_consumption.latest_snapshot_path.is_none() {
                    operator_blocker_codes
                        .push("missing_retrieval_trust_operator_evidence".to_string());
                }
                if runtime_consumption
                    .latest_snapshot_path
                    .as_deref()
                    .is_some_and(final_snapshot_missing_release_admission_evidence)
                {
                    operator_blocker_codes
                        .push("incomplete_release_admission_operator_evidence".to_string());
                }
                if latest_run_graph_recovery
                    .as_ref()
                    .is_some_and(|summary| !summary.recovery_ready)
                {
                    operator_blocker_codes.push("recovery_readiness_blocked".to_string());
                }
                if latest_run_graph_gate.as_ref().is_some_and(|summary| {
                    is_unsupported_architecture_reserved_workflow_boundary(&summary.policy_gate)
                        || is_unsupported_architecture_reserved_workflow_boundary(
                            &summary.context_state,
                        )
                }) {
                    operator_blocker_codes.push(
                        UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER.to_string(),
                    );
                }
                if latest_run_graph_gate.is_some() && latest_run_graph_dispatch_receipt.is_none() {
                    operator_blocker_codes.push(
                        MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER.to_string(),
                    );
                }
                let operator_status = if operator_blocker_codes.is_empty() {
                    "pass"
                } else {
                    "blocked"
                };
                let mut operator_next_actions: Vec<String> = Vec::new();
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "dependency_graph_issues")
                {
                    operator_next_actions.push(
                        "Run `vida task validate-graph --json` and resolve graph issues."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "boot_compatibility_not_compatible")
                {
                    operator_next_actions.push(boot_compatibility.next_step.clone());
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "boot_compatibility_unsupported_boundary")
                {
                    operator_next_actions.push(
                        "Normalize boot compatibility classification to release-1 values: backward_compatible|reader_upgrade_required.".to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "migration_preflight_not_compatible")
                {
                    operator_next_actions.push(migration_preflight.next_step.clone());
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "migration_preflight_unsupported_boundary")
                {
                    operator_next_actions.push(
                        "Normalize migration preflight compatibility classification to release-1 values: backward_compatible|reader_upgrade_required.".to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "migration_required")
                {
                    operator_next_actions
                        .push("Complete required migration before normal operation.".to_string());
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "protocol_binding_blocking_issues")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow protocol-binding check --json` and clear blockers."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "missing_retrieval_trust_signal_operator_evidence")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle check --json` to materialize retrieval-trust citation/freshness/ACL signal."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "missing_retrieval_trust_source_operator_evidence")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow consume bundle check --json` so runtime consumption snapshots publish retrieval-trust source evidence."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "missing_retrieval_trust_operator_evidence")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow consume bundle check --json` to record retrieval-trust operator evidence."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "incomplete_release_admission_operator_evidence")
                {
                    operator_next_actions.push(
                        "Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "recovery_readiness_blocked")
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
                    operator_next_actions.push(
                        UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION.to_string(),
                    );
                }
                if operator_blocker_codes.iter().any(|code| {
                    code == MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER
                }) {
                    operator_next_actions.push(
                        MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION
                            .to_string(),
                    );
                }
                if let Some(error) = release1_operator_contracts_consistency_error(
                    operator_status,
                    &operator_blocker_codes,
                    &operator_next_actions,
                ) {
                    eprintln!("doctor json contract: failed ({error})");
                    return ExitCode::from(1);
                }
                let operator_artifact_refs = serde_json::json!({
                    "runtime_consumption_latest_snapshot_path": runtime_consumption.latest_snapshot_path,
                    "latest_run_graph_dispatch_receipt_id": latest_run_graph_dispatch_receipt
                        .as_ref()
                        .map(|receipt| receipt.run_id.clone()),
                    "protocol_binding_latest_receipt_id": protocol_binding.latest_receipt_id,
                    "retrieval_trust_signal": retrieval_trust_signal,
                    "latest_task_reconciliation_receipt_id": latest_task_reconciliation
                        .as_ref()
                        .map(|receipt| receipt.receipt_id.clone()),
                    "effective_instruction_bundle_receipt_id": effective_instruction_bundle.receipt_id,
                });
                let operator_contracts = super::build_release1_operator_contracts_envelope(
                    operator_status,
                    operator_blocker_codes,
                    operator_next_actions,
                    operator_artifact_refs,
                );
                let summary_json = serde_json::json!({
                    "surface": "vida doctor",
                    "status": operator_contracts["status"].clone(),
                    "blocker_codes": operator_contracts["blocker_codes"].clone(),
                    "next_actions": operator_contracts["next_actions"].clone(),
                    "artifact_refs": operator_contracts["artifact_refs"].clone(),
                    "shared_fields": {
                        "contract_id": "release-1-shared-fields",
                        "schema_version": "release-1-v1",
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
                        "compatibility_classification": canonical_compatibility_class_str(
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
                    "protocol_binding": protocol_binding,
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
                        "receipt_id": effective_instruction_bundle.receipt_id,
                        "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
                    },
                    "storage_metadata_display": storage_metadata_display,
                });
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
            super::print_surface_ok(render, "protocol binding", &protocol_binding.as_display());
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
        final_snapshot_missing_release_admission_evidence,
        release1_operator_contracts_consistency_error,
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
            canonical_release1_operator_contract_status(&serde_json::json!("admit")),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!("block")),
            Some("blocked")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!(" admit ")),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_operator_contract_status(&serde_json::json!(" block ")),
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
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
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
                "next_actions": ["RUN `VIDA TASKFLOW RUN-GRAPH RECOVER --JSON` BEFORE RESUME."]
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
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
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
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
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
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
        );
    }
}
