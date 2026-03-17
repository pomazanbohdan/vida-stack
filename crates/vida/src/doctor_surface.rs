use std::process::ExitCode;

use crate::release1_contracts::{classify_compatibility_boundary, CompatibilityBoundary};

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

fn has_nonempty_string(value: &serde_json::Value) -> bool {
    value
        .as_str()
        .map(|entry| !entry.trim().is_empty())
        .unwrap_or(false)
}

fn retrieval_trust_signal(
    source: Option<&str>,
    latest_snapshot_path: Option<&str>,
    latest_kind: Option<&str>,
    protocol_binding_latest_receipt_id: Option<&str>,
) -> Option<serde_json::Value> {
    let source = source?;
    let citation = latest_snapshot_path?;
    let freshness = latest_kind?;
    let acl = protocol_binding_latest_receipt_id?;
    Some(serde_json::json!({
        "source": source,
        "citation": citation,
        "freshness": freshness,
        "acl": acl,
    }))
}

fn final_snapshot_missing_release_admission_evidence(snapshot_path: &str) -> bool {
    let snapshot_body = match std::fs::read_to_string(snapshot_path) {
        Ok(body) => body,
        Err(_) => return true,
    };
    let snapshot_json: serde_json::Value = match serde_json::from_str(&snapshot_body) {
        Ok(value) => value,
        Err(_) => return true,
    };
    let payload = &snapshot_json["payload"];
    if payload.is_null() {
        return true;
    }

    let canonical_registry_ok = payload["docflow_activation"]["evidence"]["registry"]["ok"]
        .as_bool()
        .is_some();
    let canonical_check_ok = payload["docflow_activation"]["evidence"]["check"]["ok"]
        .as_bool()
        .is_some();
    let canonical_readiness_verdict = has_nonempty_string(
        &payload["docflow_activation"]["evidence"]["readiness"]["verdict"],
    );
    let canonical_closure_status = has_nonempty_string(&payload["closure_admission"]["status"]);
    let canonical_closure_blockers =
        payload["closure_admission"]["blockers"].as_array().is_some();
    let canonical_top_level_operator_contract_parity = snapshot_json["status"]
        == snapshot_json["operator_contracts"]["status"]
        && snapshot_json["blocker_codes"] == snapshot_json["operator_contracts"]["blocker_codes"]
        && snapshot_json["next_actions"] == snapshot_json["operator_contracts"]["next_actions"]
        && snapshot_json["artifact_refs"] == snapshot_json["operator_contracts"]["artifact_refs"];
    let canonical_operator_contract = has_nonempty_string(&snapshot_json["operator_contracts"]["contract_id"])
        && has_nonempty_string(&snapshot_json["operator_contracts"]["schema_version"])
        && has_nonempty_string(&snapshot_json["operator_contracts"]["status"])
        && snapshot_json["operator_contracts"]["blocker_codes"]
            .as_array()
            .is_some()
        && snapshot_json["operator_contracts"]["next_actions"]
            .as_array()
            .is_some()
        && snapshot_json["operator_contracts"]["artifact_refs"].is_object()
        && has_nonempty_string(&snapshot_json["operator_contracts"]["artifact_refs"]["retrieval_trust_signal"]["source"])
        && has_nonempty_string(&snapshot_json["operator_contracts"]["artifact_refs"]["retrieval_trust_signal"]["citation"])
        && has_nonempty_string(&snapshot_json["operator_contracts"]["artifact_refs"]["retrieval_trust_signal"]["freshness"])
        && has_nonempty_string(&snapshot_json["operator_contracts"]["artifact_refs"]["retrieval_trust_signal"]["acl"]);

    !(canonical_registry_ok
        && canonical_check_ok
        && canonical_readiness_verdict
        && canonical_closure_status
        && canonical_closure_blockers
        && canonical_top_level_operator_contract_parity
        && canonical_operator_contract)
}

fn is_unsupported_architecture_reserved_workflow_boundary(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    normalized.contains("architecture_reserved")
        || normalized.contains("unsupported_boundary")
        || normalized.contains("unsupported_workflow_boundary")
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str =
    "Clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "Run `vida taskflow run-graph dispatch --json` to materialize run-graph dispatch receipt evidence before operator handoff.";

fn release1_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    match status {
        "ok" if !blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=ok must not include blocker_codes"
                .to_string(),
        ),
        "ok" if !next_actions.is_empty() => Some(
            "operator contract inconsistency: status=ok must not include next_actions".to_string(),
        ),
        "ok" => None,
        "blocked" if blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires blocker_codes".to_string(),
        ),
        "blocked" if next_actions.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires next_actions".to_string(),
        ),
        "blocked" => None,
        other => Some(format!(
            "operator contract inconsistency: unsupported status `{other}`"
        )),
    }
}

fn shared_operator_output_contract_parity_error(summary_json: &serde_json::Value) -> Option<&'static str> {
    let shared = &summary_json["shared_fields"];
    let contracts = &summary_json["operator_contracts"];
    if summary_json["status"] != contracts["status"]
        || summary_json["blocker_codes"] != contracts["blocker_codes"]
        || summary_json["next_actions"] != contracts["next_actions"]
        || summary_json["status"] != shared["status"]
        || summary_json["blocker_codes"] != shared["blocker_codes"]
        || summary_json["next_actions"] != shared["next_actions"]
    {
        return Some(
            "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch",
        );
    }
    None
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
                if runtime_consumption.latest_kind.as_deref() == Some("final")
                    && runtime_consumption
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
                    "ok"
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
                        "Normalize boot compatibility classification to release-1 values: compatible|incompatible|degraded|blocking.".to_string(),
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
                        "Normalize migration preflight compatibility classification to release-1 values: compatible|incompatible|degraded|blocking.".to_string(),
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
                        "Run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle-check --json` to materialize retrieval-trust citation/freshness/ACL signal."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "missing_retrieval_trust_source_operator_evidence")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow consume bundle-check --json` so runtime consumption snapshots publish retrieval-trust source evidence."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| code == "missing_retrieval_trust_operator_evidence")
                {
                    operator_next_actions.push(
                        "Run `vida taskflow consume bundle-check --json` to record retrieval-trust operator evidence."
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
                        "Run `vida taskflow run-graph recover --json` and confirm `recovery_ready=true` before resume/rollback handoff."
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| {
                        code == UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER
                    })
                {
                    operator_next_actions.push(
                        UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION
                            .to_string(),
                    );
                }
                if operator_blocker_codes
                    .iter()
                    .any(|code| {
                        code == MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER
                    })
                {
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
                        "status": operator_contracts["status"].clone(),
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
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
                        "compatibility_classification": migration_preflight.compatibility_classification,
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
                    migration_preflight.compatibility_classification,
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
