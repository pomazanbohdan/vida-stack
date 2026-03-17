use std::path::Path;
use std::process::ExitCode;

use crate::{
    release1_contracts::{canonical_compatibility_class_str, CompatibilityClass},
    state_store,
    state_store::StateStore,
    StatusArgs,
};

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

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

pub(crate) async fn run_status(args: StatusArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;

    match StateStore::open_existing(state_dir).await {
        Ok(store) => match store.storage_metadata_summary().await {
            Ok(storage_metadata) => {
                let backend_summary = format!(
                    "{} state-v{} instruction-v{}",
                    storage_metadata.backend,
                    storage_metadata.state_schema_version,
                    storage_metadata.instruction_schema_version
                );
                let state_spine = match store.state_spine_summary().await {
                    Ok(state_spine) => state_spine,
                    Err(error) => {
                        eprintln!("Failed to read authoritative state spine summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let effective_bundle_receipt =
                    match store.latest_effective_bundle_receipt_summary().await {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!("Failed to read effective bundle receipt summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let boot_compatibility = match store.latest_boot_compatibility_summary().await {
                    Ok(compatibility) => compatibility,
                    Err(error) => {
                        eprintln!("Failed to read boot compatibility summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let migration_state = match store.latest_migration_preflight_summary().await {
                    Ok(migration) => migration,
                    Err(error) => {
                        eprintln!("Failed to read migration preflight summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let migration_receipts = match store.migration_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read migration receipt summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_task_reconciliation =
                    match store.latest_task_reconciliation_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read task reconciliation summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let task_reconciliation_rollup = match store.task_reconciliation_rollup().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read task reconciliation rollup: {error}");
                        return ExitCode::from(1);
                    }
                };
                let snapshot_bridge = match store.taskflow_snapshot_bridge_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read taskflow snapshot bridge summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let runtime_consumption = match super::runtime_consumption_summary(store.root()) {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read runtime-consumption summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let protocol_binding = match store.protocol_binding_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read protocol-binding summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_status = match store.latest_run_graph_status().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph status: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_recovery =
                    match store.latest_run_graph_recovery_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph recovery summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_checkpoint = match store
                    .latest_run_graph_checkpoint_summary()
                    .await
                {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph checkpoint summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_gate = match store.latest_run_graph_gate_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph gate summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_dispatch_receipt =
                    match store.latest_run_graph_dispatch_receipt_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!(
                                "Failed to read latest run graph dispatch receipt summary: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                let status_project_root = super::resolve_status_project_root(store.root());
                let host_agents = status_project_root
                    .as_deref()
                    .and_then(build_host_agent_status_summary);
                let activation_truth = status_project_root.as_deref().map(|project_root| {
                    super::project_activator_surface::canonical_project_activation_status_truth(
                        project_root,
                    )
                });

                if as_json {
                    let mut operator_blocker_codes: Vec<String> = Vec::new();
                    if boot_compatibility
                        .as_ref()
                        .is_some_and(|compatibility| compatibility.classification != "compatible")
                    {
                        operator_blocker_codes
                            .push("boot_compatibility_not_compatible".to_string());
                    }
                    if migration_state.as_ref().is_some_and(|migration| {
                        migration.compatibility_classification != "compatible"
                    }) {
                        operator_blocker_codes
                            .push("migration_preflight_not_compatible".to_string());
                    }
                    if migration_state.as_ref().is_some_and(|migration| {
                        migration_requires_action(&migration.migration_state)
                    }) {
                        operator_blocker_codes.push("migration_required".to_string());
                    }
                    if protocol_binding.blocking_issue_count > 0 {
                        operator_blocker_codes.push("protocol_binding_blocking_issues".to_string());
                    }
                    if latest_run_graph_gate.is_some()
                        && latest_run_graph_dispatch_receipt.is_none()
                    {
                        operator_blocker_codes
                            .push("missing_run_graph_dispatch_receipt_operator_evidence".to_string());
                    }
                    match activation_truth.as_ref() {
                        Some(truth) if truth.activation_pending => {
                            operator_blocker_codes.push("project_activation_pending".to_string());
                        }
                        None => {
                            operator_blocker_codes.push("project_activation_unknown".to_string());
                        }
                        _ => {}
                    }
                    let operator_status = if operator_blocker_codes.is_empty() {
                        "ok"
                    } else {
                        "blocked"
                    };
                    let mut operator_next_actions: Vec<String> = Vec::new();
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "boot_compatibility_not_compatible")
                    {
                        if let Some(compatibility) = boot_compatibility.as_ref() {
                            operator_next_actions.push(compatibility.next_step.clone());
                        }
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "migration_preflight_not_compatible")
                    {
                        if let Some(migration) = migration_state.as_ref() {
                            operator_next_actions.push(migration.next_step.clone());
                        }
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "migration_required")
                    {
                        operator_next_actions.push(
                            "Complete required migration before normal operation.".to_string(),
                        );
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
                        .any(|code| code == "project_activation_pending")
                    {
                        if let Some(truth) = activation_truth.as_ref() {
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
                        .any(|code| code == "project_activation_unknown")
                    {
                        operator_next_actions.push(
                            "Resolve project root detection and run `vida project-activator --json` to surface canonical activation state."
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "missing_run_graph_dispatch_receipt_operator_evidence")
                    {
                        operator_next_actions.push(
                            "Run `vida taskflow run-graph dispatch --json` to materialize run-graph dispatch receipt evidence before operator handoff."
                                .to_string(),
                        );
                    }
                    let operator_artifact_refs = serde_json::json!({
                        "runtime_consumption_latest_snapshot_path": runtime_consumption.latest_snapshot_path,
                        "latest_run_graph_dispatch_receipt_id": latest_run_graph_dispatch_receipt
                            .as_ref()
                            .map(|receipt| receipt.run_id.clone()),
                        "protocol_binding_latest_receipt_id": protocol_binding.latest_receipt_id,
                        "latest_task_reconciliation_receipt_id": latest_task_reconciliation
                            .as_ref()
                            .map(|receipt| receipt.receipt_id.clone()),
                        "effective_instruction_bundle_receipt_id": effective_bundle_receipt
                            .as_ref()
                            .map(|receipt| receipt.receipt_id.clone()),
                    });
                    let operator_contracts = super::build_release1_operator_contracts_envelope(
                        operator_status,
                        operator_blocker_codes,
                        operator_next_actions,
                        operator_artifact_refs,
                    );
                    let blocker_codes = operator_contracts["blocker_codes"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>();
                    let next_actions = operator_contracts["next_actions"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>();
                    if let Some(error) = release1_operator_contracts_consistency_error(
                        operator_contracts["status"].as_str().unwrap_or(""),
                        &blocker_codes,
                        &next_actions,
                    ) {
                        eprintln!("Failed to render status json: {error}");
                        return ExitCode::from(1);
                    }
                    let summary_json = serde_json::json!({
                        "surface": "vida status",
                        "status": operator_contracts["status"].clone(),
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
                        "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        "shared_fields": {
                            "status": operator_contracts["status"].clone(),
                            "blocker_codes": operator_contracts["blocker_codes"].clone(),
                            "next_actions": operator_contracts["next_actions"].clone(),
                            "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        },
                        "operator_contracts": operator_contracts,
                        "state_dir": store.root().display().to_string(),
                        "storage_metadata": {
                            "engine": storage_metadata.engine,
                            "backend": storage_metadata.backend,
                            "namespace": storage_metadata.namespace,
                            "database": storage_metadata.database,
                            "state_schema_version": storage_metadata.state_schema_version,
                            "instruction_schema_version": storage_metadata.instruction_schema_version,
                        },
                        "backend_summary": backend_summary,
                        "state_spine": {
                            "state_schema_version": state_spine.state_schema_version,
                            "entity_surface_count": state_spine.entity_surface_count,
                            "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                        },
                        "latest_effective_bundle_receipt": effective_bundle_receipt,
                        "boot_compatibility": boot_compatibility.as_ref().map(|compatibility| serde_json::json!({
                            "classification": compatibility.classification,
                            "reasons": compatibility.reasons,
                            "next_step": compatibility.next_step,
                        })),
                        "migration_state": migration_state.as_ref().map(|migration| serde_json::json!({
                            "compatibility_classification": canonical_compatibility_class_str(
                                &migration.compatibility_classification
                            ).unwrap_or(CompatibilityClass::Incompatible.as_str()),
                            "migration_state": migration.migration_state,
                            "blockers": migration.blockers,
                            "source_version_tuple": migration.source_version_tuple,
                            "next_step": migration.next_step,
                        })),
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
                        "project_activation": activation_truth.as_ref().map(|truth| serde_json::json!({
                            "status": truth.status,
                            "activation_pending": truth.activation_pending,
                            "next_steps": truth.next_steps,
                        })).unwrap_or_else(|| serde_json::json!({
                            "status": "unknown",
                            "activation_pending": true,
                            "next_steps": [
                                "run `vida project-activator --json` from the project root to load canonical activation state"
                            ],
                        })),
                        "host_agents": host_agents_json_value(host_agents.clone()),
                        "latest_run_graph_status": latest_run_graph_status,
                        "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                    });
                    if let Some(error) = shared_operator_output_contract_parity_error(&summary_json)
                    {
                        eprintln!("Failed to render status json: {error}");
                        return ExitCode::from(1);
                    }
                    if summary_json["artifact_refs"]
                            != summary_json["operator_contracts"]["artifact_refs"]
                        || summary_json["artifact_refs"]
                            != summary_json["shared_fields"]["artifact_refs"]
                    {
                        eprintln!(
                            "Failed to render status json: top-level/operator_contracts/shared_fields mirror mismatch"
                        );
                        return ExitCode::from(1);
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary_json)
                        .expect("status summary should render as json")
                    );
                    return ExitCode::SUCCESS;
                }

                super::print_surface_header(render, "vida status");
                super::print_surface_line(render, "backend", &backend_summary);
                super::print_surface_line(render, "state dir", &store.root().display().to_string());
                super::print_surface_line(
                    render,
                    "state spine",
                    &format!(
                        "initialized (state-v{}, {} entity surfaces, mutation root {})",
                        state_spine.state_schema_version,
                        state_spine.entity_surface_count,
                        state_spine.authoritative_mutation_root
                    ),
                );
                match effective_bundle_receipt {
                    Some(receipt) => {
                        super::print_surface_line(
                            render,
                            "latest effective bundle receipt",
                            &receipt.receipt_id,
                        );
                        super::print_surface_line(
                            render,
                            "latest effective bundle root",
                            &receipt.root_artifact_id,
                        );
                        super::print_surface_line(
                            render,
                            "latest effective bundle artifact count",
                            &receipt.artifact_count.to_string(),
                        );
                    }
                    None => {
                        super::print_surface_line(
                            render,
                            "latest effective bundle receipt",
                            "none",
                        );
                    }
                }
                match boot_compatibility {
                    Some(compatibility) => {
                        super::print_surface_line(
                            render,
                            "boot compatibility",
                            &format!(
                                "{} ({})",
                                compatibility.classification, compatibility.next_step
                            ),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "boot compatibility", "none");
                    }
                }
                match migration_state {
                    Some(migration) => {
                        let compatibility_classification = canonical_compatibility_class_str(
                            &migration.compatibility_classification,
                        )
                        .unwrap_or(CompatibilityClass::Incompatible.as_str());
                        super::print_surface_line(
                            render,
                            "migration state",
                            &format!(
                                "{} / {} ({})",
                                compatibility_classification,
                                migration.migration_state,
                                migration.next_step
                            ),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "migration state", "none");
                    }
                }
                super::print_surface_line(
                    render,
                    "migration receipts",
                    &migration_receipts.as_display(),
                );
                match latest_task_reconciliation {
                    Some(receipt) => {
                        super::print_surface_line(
                            render,
                            "latest task reconciliation",
                            &receipt.as_display(),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "latest task reconciliation", "none");
                    }
                }
                super::print_surface_line(
                    render,
                    "task reconciliation rollup",
                    &task_reconciliation_rollup.as_display(),
                );
                super::print_surface_line(
                    render,
                    "taskflow snapshot bridge",
                    &snapshot_bridge.as_display(),
                );
                super::print_surface_line(
                    render,
                    "runtime consumption",
                    &runtime_consumption.as_display(),
                );
                super::print_surface_line(
                    render,
                    "protocol binding",
                    &protocol_binding.as_display(),
                );
                if let Some(truth) = activation_truth.as_ref() {
                    super::print_surface_line(
                        render,
                        "project activation",
                        &format!(
                            "{} (activation_pending={})",
                            truth.status, truth.activation_pending
                        ),
                    );
                } else {
                    super::print_surface_line(
                        render,
                        "project activation",
                        "unknown (fail-closed: activation_pending=true)",
                    );
                }
                match latest_run_graph_status {
                    Some(status) => {
                        super::print_surface_line(
                            render,
                            "latest run graph status",
                            &status.as_display(),
                        );
                        super::print_surface_line(
                            render,
                            "latest run graph delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "latest run graph status", "none");
                    }
                }
                match latest_run_graph_recovery {
                    Some(summary) => {
                        super::print_surface_line(
                            render,
                            "latest run graph recovery",
                            &summary.as_display(),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "latest run graph recovery", "none");
                    }
                }
                match latest_run_graph_checkpoint {
                    Some(summary) => {
                        super::print_surface_line(
                            render,
                            "latest run graph checkpoint",
                            &summary.as_display(),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "latest run graph checkpoint", "none");
                    }
                }
                match latest_run_graph_gate {
                    Some(summary) => {
                        super::print_surface_line(
                            render,
                            "latest run graph gate",
                            &summary.as_display(),
                        );
                    }
                    None => {
                        super::print_surface_line(render, "latest run graph gate", "none");
                    }
                }
                if let Some(host_agents) = host_agents {
                    super::print_surface_line(
                        render,
                        "host agents",
                        host_agents["host_cli_system"].as_str().unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        render,
                        "host agent budget units",
                        &host_agents["budget"]["total_estimated_units"]
                            .as_u64()
                            .unwrap_or_default()
                            .to_string(),
                    );
                    super::print_surface_line(
                        render,
                        "host agent events",
                        &host_agents["budget"]["event_count"]
                            .as_u64()
                            .unwrap_or_default()
                            .to_string(),
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Failed to read storage metadata: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn build_host_agent_status_summary(project_root: &Path) -> Option<serde_json::Value> {
    let overlay = super::project_activator_surface::read_yaml_file_checked(
        &project_root.join("vida.config.yaml"),
    )
    .ok()?;
    let selected_cli_system = super::yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(super::project_activator_surface::normalize_host_cli_system)?;
    match selected_cli_system {
        "codex" => {
            let overlay_roles =
                super::project_activator_surface::overlay_codex_agent_catalog(&overlay);
            let codex_roles = if overlay_roles.is_empty() {
                super::project_activator_surface::read_codex_agent_catalog(
                    &project_root.join(".codex"),
                )
            } else {
                overlay_roles
            };
            let strategy = super::read_json_file_if_present(
                &super::codex_worker_strategy_state_path(project_root),
            )
            .unwrap_or(serde_json::Value::Null);
            let scorecards = super::read_json_file_if_present(
                &super::codex_worker_scorecards_state_path(project_root),
            )
            .unwrap_or(serde_json::Value::Null);
            let observability = super::read_json_file_if_present(
                &super::host_agent_observability_state_path(project_root),
            )
            .unwrap_or_else(|| {
                super::load_or_initialize_host_agent_observability_state(project_root)
            });
            let latest_events = observability["events"]
                .as_array()
                .map(|events| events.iter().rev().take(5).cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            let mut agents = serde_json::Map::new();
            for role in &codex_roles {
                let Some(role_id) = role["role_id"].as_str() else {
                    continue;
                };
                let feedback_rows = scorecards["agents"][role_id]["feedback"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let last_feedback = feedback_rows
                    .last()
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                agents.insert(
                    role_id.to_string(),
                    serde_json::json!({
                        "tier": role["tier"],
                        "rate": role["rate"],
                        "reasoning_band": role["reasoning_band"],
                        "default_runtime_role": role["default_runtime_role"],
                        "runtime_roles": role["runtime_roles"],
                        "task_classes": role["task_classes"],
                        "feedback_count": feedback_rows.len(),
                        "last_feedback_at": last_feedback["recorded_at"],
                        "last_feedback_outcome": last_feedback["outcome"],
                        "effective_score": strategy["agents"][role_id]["effective_score"],
                        "lifecycle_state": strategy["agents"][role_id]["lifecycle_state"],
                    }),
                );
            }
            let overlay_dispatch_aliases_result =
                super::project_activator_surface::codex_dispatch_alias_catalog_for_root(
                    &overlay,
                    project_root,
                    &codex_roles,
                );
            let internal_dispatch_alias_load_error = overlay_dispatch_aliases_result
                .as_ref()
                .err()
                .map(std::string::ToString::to_string);
            let overlay_dispatch_aliases = overlay_dispatch_aliases_result.unwrap_or_default();

            Some(serde_json::json!({
                "host_cli_system": "codex",
                "runtime_surface": ".codex/**",
                "stores": {
                    "scorecards": super::CODEX_WORKER_SCORECARDS_STATE,
                    "strategy": super::CODEX_WORKER_STRATEGY_STATE,
                    "observability": super::HOST_AGENT_OBSERVABILITY_STATE,
                },
                "selection_policy": strategy["selection_policy"],
                "budget": observability["budget"],
                "agents": agents,
                "internal_dispatch_alias_count": overlay_dispatch_aliases.len(),
                "internal_dispatch_alias_load_error": internal_dispatch_alias_load_error,
                "recent_events": latest_events,
            }))
        }
        _ => None,
    }
}

fn host_agents_json_value(host_agents: Option<serde_json::Value>) -> serde_json::Value {
    host_agents.unwrap_or_else(|| serde_json::json!({}))
}
