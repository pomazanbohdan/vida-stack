use std::path::Path;
use std::process::ExitCode;

use crate::{
    release1_contracts::{
        blocker_code_str, canonical_compatibility_class_str, BlockerCode, CompatibilityClass,
    },
    state_store,
    state_store::StateStore,
    StatusArgs,
};

use serde_yaml;

use super::activation_status::canonical_activation_status;
use crate::operator_contracts::{
    release1_operator_contracts_consistency_error, shared_operator_output_contract_parity_error,
};

fn selected_host_cli_system_entry(
    overlay: &serde_yaml::Value,
) -> (String, Option<serde_yaml::Value>) {
    let registry =
        super::project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay));
    let candidate = super::yaml_lookup(overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "__HOST_CLI_SYSTEM__")
        .and_then(super::project_activator_surface::normalize_host_cli_system);
    let normalized = candidate.unwrap_or_else(|| {
        let mut supported = registry
            .iter()
            .filter(|(_, entry)| super::yaml_bool(super::yaml_lookup(entry, &["enabled"]), true))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        supported.sort();
        supported
            .into_iter()
            .next()
            .or_else(|| {
                let mut fallback = registry.keys().cloned().collect::<Vec<_>>();
                fallback.sort();
                fallback.into_iter().next()
            })
            .unwrap_or_default()
    });
    let entry = registry.get(&normalized).cloned();
    (normalized, entry)
}

fn runtime_surface_for_selected_system(system: &str, entry: Option<&serde_yaml::Value>) -> String {
    entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])))
        .unwrap_or_else(|| format!(".{system}"))
}

fn runtime_root_for_selected_system(
    project_root: &Path,
    system: &str,
    entry: Option<&serde_yaml::Value>,
) -> String {
    let configured =
        entry.and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])));
    let relative = configured.unwrap_or_else(|| format!(".{system}"));
    project_root.join(relative).display().to_string()
}

fn has_runtime_root_session_write_guard(value: &serde_json::Value) -> bool {
    value["status"].as_str() == Some("blocked_by_default")
        && value["root_session_role"]
            .as_str()
            .is_some_and(|role| !role.trim().is_empty())
        && value["local_write_requires_exception_path"].is_boolean()
        && value["required_exception_evidence"]
            .as_str()
            .is_some_and(|evidence| !evidence.trim().is_empty())
        && value["pre_write_checkpoint_required"].is_boolean()
}

fn migration_requires_action(migration_state: &str) -> bool {
    !matches!(migration_state, "none_required" | "no_migration_required")
}

fn run_graph_latest_snapshot_inconsistent_next_action() -> &'static str {
    "Rebuild the latest run-graph evidence by rerunning `vida taskflow consume continue --json` and then recheck `vida status --json` once status, recovery, checkpoint, gate, and dispatch receipt share the same run_id."
}

fn run_graph_latest_dispatch_receipt_signal_ambiguous_next_action() -> &'static str {
    "Rebuild the latest run-graph dispatch receipt with `vida taskflow consume continue --json` so lane_status and dispatch_status are canonical and aligned before trusting the operator signal."
}

fn run_graph_latest_dispatch_receipt_summary_inconsistent_next_action() -> &'static str {
    "Refresh the latest run-graph dispatch receipt summary before rerunning `vida status --json` so the latest status and dispatch receipt share the same run_id."
}

fn run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action() -> &'static str {
    "Refresh the latest checkpoint evidence for the run graph before rerunning `vida status --json` so checkpoint rows and dispatch receipt evidence share the same run_id."
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
    let operator_contracts = match summary_json.get("operator_contracts") {
        Some(value) => value,
        None => return true,
    };
    crate::operator_contracts::shared_operator_output_contract_parity_error(&summary_json)
        .is_some()
        || crate::operator_contracts::release1_operator_contracts_consistency_error(
            summary_json["status"].as_str().unwrap_or(""),
            &operator_contracts["blocker_codes"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            &operator_contracts["next_actions"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        .is_some()
}

fn is_sandbox_active_from_env() -> bool {
    let candidates = [
        std::env::var("CODEX_SANDBOX_MODE").ok(),
        std::env::var("SANDBOX_MODE").ok(),
        std::env::var("VIDA_SANDBOX_MODE").ok(),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_ascii_lowercase())
        .find(|value| !value.is_empty())
        .map(|value| {
            !matches!(
                value.as_str(),
                "danger-full-access" | "none" | "off" | "disabled" | "false"
            )
        })
        .unwrap_or(false)
}

fn can_resolve_public_network() -> bool {
    use std::net::ToSocketAddrs;
    if let Ok(override_value) = std::env::var("VIDA_NETWORK_PROBE_OVERRIDE") {
        let normalized = override_value.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "reachable" | "online" | "true" | "1") {
            return true;
        }
        if matches!(
            normalized.as_str(),
            "unreachable" | "offline" | "false" | "0"
        ) {
            return false;
        }
    }
    ("example.com", 443)
        .to_socket_addrs()
        .map(|mut rows| rows.next().is_some())
        .unwrap_or(false)
}

fn external_cli_preflight_summary(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> serde_json::Value {
    let selected_execution_class = selected_cli_entry
        .map(|entry| {
            super::project_activator_surface::host_cli_system_execution_class(
                entry,
                selected_cli_system,
            )
        })
        .unwrap_or_else(|| "unknown".to_string());
    let selected_is_external = selected_execution_class == "external";
    let has_enabled_external_subagents =
        super::yaml_lookup(overlay, &["agent_system", "subagents"])
            .and_then(serde_yaml::Value::as_mapping)
            .map(|mapping| {
                mapping.values().any(|entry| {
                    let enabled = super::yaml_bool(super::yaml_lookup(entry, &["enabled"]), false);
                    let backend = super::yaml_lookup(entry, &["subagent_backend_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .map(str::to_ascii_lowercase)
                        .unwrap_or_default();
                    enabled && backend == "external_cli"
                })
            })
            .unwrap_or(false);
    let requires_external_cli = selected_is_external || has_enabled_external_subagents;
    let sandbox_active = is_sandbox_active_from_env();
    let network_reachable = can_resolve_public_network();

    if requires_external_cli && sandbox_active && !network_reachable {
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": true,
            "selected_execution_class": selected_execution_class,
            "sandbox_active": true,
            "network_reachable": false,
            "blocker_code": "external_cli_network_access_unavailable_under_sandbox",
            "next_actions": [
                "Allow network access for this session or rerun outside sandbox before using external CLI agents.",
                "If sandbox must stay enabled, switch host and routing to an internal backend in `vida.config.yaml`.",
                "Rerun `vida status --json` and then retry the external CLI command."
            ]
        });
    }

    serde_json::json!({
        "status": "pass",
        "requires_external_cli": requires_external_cli,
        "selected_execution_class": selected_execution_class,
        "sandbox_active": sandbox_active,
        "network_reachable": network_reachable,
        "blocker_code": serde_json::Value::Null,
        "next_actions": []
    })
}

pub(crate) async fn run_status(args: StatusArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;
    let summary_only = args.summary;

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
                let mut latest_run_graph_dispatch_receipt_checkpoint_leakage = false;
                let latest_run_graph_dispatch_receipt =
                    match store.latest_run_graph_dispatch_receipt_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            if error
                                .to_string()
                                .contains("latest checkpoint evidence must share the same run_id")
                            {
                                latest_run_graph_dispatch_receipt_checkpoint_leakage = true;
                                None
                            } else {
                                eprintln!(
                                "Failed to read latest run graph dispatch receipt summary: {error}"
                            );
                                return ExitCode::from(1);
                            }
                        }
                    };
                let latest_run_graph_dispatch_receipt_matches_status =
                    latest_run_graph_dispatch_receipt_checkpoint_leakage
                        || state_store::latest_run_graph_dispatch_receipt_matches_status(
                            latest_run_graph_status
                                .as_ref()
                                .map(|status| status.run_id.as_str()),
                            latest_run_graph_dispatch_receipt
                                .as_ref()
                                .map(|receipt| receipt.run_id.as_str()),
                        );
                let latest_run_graph_dispatch_receipt_summary_inconsistent =
                    !latest_run_graph_dispatch_receipt_checkpoint_leakage
                        && state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                            latest_run_graph_status
                                .as_ref()
                                .map(|status| status.run_id.as_str()),
                            latest_run_graph_dispatch_receipt
                                .as_ref()
                                .map(|receipt| receipt.run_id.as_str()),
                        );
                let latest_run_graph_snapshot_inconsistent =
                    !latest_run_graph_dispatch_receipt_checkpoint_leakage
                        && !state_store::latest_run_graph_evidence_snapshot_is_consistent(
                            latest_run_graph_status
                                .as_ref()
                                .map(|status| status.run_id.as_str()),
                            latest_run_graph_recovery
                                .as_ref()
                                .map(|summary| summary.run_id.as_str()),
                            latest_run_graph_checkpoint
                                .as_ref()
                                .map(|summary| summary.run_id.as_str()),
                            latest_run_graph_gate
                                .as_ref()
                                .map(|summary| summary.run_id.as_str()),
                            latest_run_graph_dispatch_receipt
                                .as_ref()
                                .map(|receipt| receipt.run_id.as_str()),
                        );
                let latest_run_graph_dispatch_receipt_signal_ambiguous =
                    latest_run_graph_dispatch_receipt
                        .as_ref()
                        .is_some_and(|receipt| {
                            state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(
                                receipt,
                            )
                        });
                let status_project_root = super::resolve_status_project_root(store.root());
                let mut host_agents = status_project_root
                    .as_deref()
                    .and_then(build_host_agent_status_summary);
                let root_session_write_guard = root_session_write_guard_summary_from_snapshot_path(
                    runtime_consumption.latest_snapshot_path.as_deref(),
                );
                if let Some(host_agents_value) = host_agents.as_mut() {
                    if let Some(object) = host_agents_value.as_object_mut() {
                        object.insert(
                            "root_session_write_guard".to_string(),
                            root_session_write_guard.clone(),
                        );
                    }
                }
                let activation_truth = status_project_root.as_deref().map(|project_root| {
                    super::project_activator_surface::canonical_project_activation_status_truth(
                        project_root,
                    )
                });
                let project_activation_status = activation_truth.as_ref().map(|truth| {
                    canonical_activation_status(
                        Some(truth.status.as_str()),
                        truth.activation_pending,
                    )
                });
                let project_activation_pending = project_activation_status == Some("pending");
                if as_json {
                    let mut operator_blocker_codes: Vec<String> = Vec::new();
                    let incomplete_release_admission_operator_evidence =
                        runtime_consumption.latest_kind.as_deref() != Some("final")
                            || runtime_consumption
                                .latest_snapshot_path
                                .as_deref()
                                .is_some_and(final_snapshot_missing_release_admission_evidence);
                    if incomplete_release_admission_operator_evidence {
                        operator_blocker_codes.push(
                            blocker_code_str(BlockerCode::IncompleteReleaseAdmissionOperatorEvidence)
                                .to_string(),
                        );
                    } else if root_session_write_guard["status"].as_str() != Some("blocked_by_default")
                    {
                        operator_blocker_codes.push(
                            blocker_code_str(BlockerCode::MissingRootSessionWriteGuard).to_string(),
                        );
                    }
                    if boot_compatibility.as_ref().is_some_and(|compatibility| {
                        canonical_compatibility_class_str(&compatibility.classification)
                            != Some(CompatibilityClass::BackwardCompatible.as_str())
                    }) {
                        operator_blocker_codes.push(
                            crate::release1_contracts::blocker_code_str(
                                crate::release1_contracts::BlockerCode::BootCompatibilityNotCompatible,
                            )
                            .to_string(),
                        );
                    }
                    if migration_state.as_ref().is_some_and(|migration| {
                        canonical_compatibility_class_str(&migration.compatibility_classification)
                            != Some(CompatibilityClass::BackwardCompatible.as_str())
                    }) {
                        operator_blocker_codes.push(
                            crate::release1_contracts::blocker_code_str(
                                crate::release1_contracts::BlockerCode::MigrationPreflightNotReady,
                            )
                            .to_string(),
                        );
                    }
                    if migration_state.as_ref().is_some_and(|migration| {
                        migration_requires_action(&migration.migration_state)
                    }) {
                        operator_blocker_codes
                            .push(blocker_code_str(BlockerCode::MigrationRequired).to_string());
                    }
                    if protocol_binding.blocking_issue_count > 0 {
                        operator_blocker_codes.push(
                            blocker_code_str(BlockerCode::ProtocolBindingBlockingIssues)
                                .to_string(),
                        );
                    }
                    if latest_run_graph_gate.is_some()
                        && !latest_run_graph_dispatch_receipt_matches_status
                    {
                        operator_blocker_codes.push(
                            blocker_code_str(
                                BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence,
                            )
                            .to_string(),
                        );
                    }
                    if latest_run_graph_snapshot_inconsistent {
                        operator_blocker_codes.push(
                            blocker_code_str(BlockerCode::RunGraphLatestSnapshotInconsistent)
                                .to_string(),
                        );
                    }
                    if latest_run_graph_dispatch_receipt_signal_ambiguous {
                        operator_blocker_codes.push(
                            blocker_code_str(
                                BlockerCode::RunGraphLatestDispatchReceiptSignalAmbiguous,
                            )
                            .to_string(),
                        );
                    }
                    if latest_run_graph_dispatch_receipt_summary_inconsistent {
                        operator_blocker_codes.push(
                            blocker_code_str(
                                BlockerCode::RunGraphLatestDispatchReceiptSummaryInconsistent,
                            )
                            .to_string(),
                        );
                    }
                    if latest_run_graph_dispatch_receipt_checkpoint_leakage {
                        operator_blocker_codes.push(
                            blocker_code_str(
                                BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage,
                            )
                            .to_string(),
                        );
                    }
                    match activation_truth.as_ref() {
                        Some(_) if project_activation_pending => {
                            operator_blocker_codes.push(
                                crate::release1_contracts::blocker_code_str(
                                    crate::release1_contracts::BlockerCode::ActivationPending,
                                )
                                .to_string(),
                            );
                        }
                        None => {
                            operator_blocker_codes.push(
                                blocker_code_str(BlockerCode::ProjectActivationUnknown).to_string(),
                            );
                        }
                        _ => {}
                    }
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
                        if let Some(compatibility) = boot_compatibility.as_ref() {
                            operator_next_actions.push(compatibility.next_step.clone());
                        }
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "migration_not_ready")
                    {
                        if let Some(migration) = migration_state.as_ref() {
                            operator_next_actions.push(migration.next_step.clone());
                        }
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == blocker_code_str(BlockerCode::MigrationRequired))
                    {
                        operator_next_actions.push(
                            "Complete required migration before normal operation.".to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(BlockerCode::ProtocolBindingBlockingIssues)
                    }) {
                        operator_next_actions.push(
                            "Run `vida taskflow protocol-binding check --json` and clear blockers."
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes
                        .iter()
                        .any(|code| code == "activation_pending")
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
                        .any(|code| code == blocker_code_str(BlockerCode::ProjectActivationUnknown))
                    {
                        operator_next_actions.push(
                            "Resolve project root detection and run `vida project-activator --json` to surface canonical activation state."
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(
                            BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence,
                        )
                    }) {
                        operator_next_actions.push(
                            "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff."
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(BlockerCode::RunGraphLatestSnapshotInconsistent)
                    }) {
                        operator_next_actions
                            .push(run_graph_latest_snapshot_inconsistent_next_action().to_string());
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(
                            BlockerCode::RunGraphLatestDispatchReceiptSignalAmbiguous,
                        )
                    }) {
                        operator_next_actions.push(
                            run_graph_latest_dispatch_receipt_signal_ambiguous_next_action()
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(
                            BlockerCode::RunGraphLatestDispatchReceiptSummaryInconsistent,
                        )
                    }) {
                        operator_next_actions.push(
                            run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(
                            BlockerCode::RunGraphLatestDispatchReceiptCheckpointLeakage,
                        )
                    }) {
                        operator_next_actions.push(
                            run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(
                            BlockerCode::IncompleteReleaseAdmissionOperatorEvidence,
                        )
                    }) {
                        operator_next_actions.push(
                            "Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete."
                                .to_string(),
                        );
                    }
                    if operator_blocker_codes.iter().any(|code| {
                        code == blocker_code_str(BlockerCode::MissingRootSessionWriteGuard)
                    }) {
                        operator_next_actions.push(
                            "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard."
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
                        "root_session_write_guard_status": root_session_write_guard["status"].clone(),
                    });
                    let operator_contracts = super::build_release1_operator_contracts_envelope(
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
                        eprintln!("Failed to render status json: {error}");
                        return ExitCode::from(1);
                    }
                    let summary_json = if summary_only {
                        serde_json::json!({
                            "surface": "vida status",
                            "view": "summary",
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
                            "backend_summary": backend_summary,
                            "state_spine": {
                                "state_schema_version": state_spine.state_schema_version,
                                "entity_surface_count": state_spine.entity_surface_count,
                                "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                            },
                        "project_activation": activation_truth.as_ref().map(|truth| serde_json::json!({
                            "status": project_activation_status.unwrap_or("pending"),
                            "activation_pending": project_activation_pending,
                            "next_steps": truth.next_steps,
                        })).unwrap_or_else(|| serde_json::json!({
                                "status": "unknown",
                                "activation_pending": true,
                                "next_steps": [
                                    "run `vida project-activator --json` from the project root to load canonical activation state"
                                ],
                        })),
                        "protocol_binding": protocol_binding,
                        "root_session_write_guard": root_session_write_guard,
                        "latest_run_graph_status": latest_run_graph_status,
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "host_agents": host_agents_json_value(host_agents.as_ref()),
                    })
                    } else {
                        serde_json::json!({
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
                                "classification": canonical_compatibility_class_str(
                                    &compatibility.classification
                                ).unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
                                "reasons": compatibility.reasons,
                                "next_step": compatibility.next_step,
                            })),
                            "migration_state": migration_state.as_ref().map(|migration| serde_json::json!({
                                "compatibility_classification": canonical_compatibility_class_str(
                                    &migration.compatibility_classification
                                ).unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str()),
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
                                "status": project_activation_status.unwrap_or("pending"),
                                "activation_pending": project_activation_pending,
                                "next_steps": truth.next_steps,
                            })).unwrap_or_else(|| serde_json::json!({
                                "status": "unknown",
                                "activation_pending": true,
                                "next_steps": [
                                    "run `vida project-activator --json` from the project root to load canonical activation state"
                                ],
                            })),
                            "host_agents": host_agents_json_value(host_agents.as_ref()),
                            "root_session_write_guard": root_session_write_guard,
                            "latest_run_graph_status": latest_run_graph_status,
                            "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                            "latest_run_graph_recovery": latest_run_graph_recovery,
                            "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                            "latest_run_graph_gate": latest_run_graph_gate,
                            "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        })
                    };
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
                        let compatibility_classification =
                            canonical_compatibility_class_str(&compatibility.classification)
                                .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str());
                        super::print_surface_line(
                            render,
                            "boot compatibility",
                            &format!(
                                "{} ({})",
                                compatibility_classification, compatibility.next_step
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
                        .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str());
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
                if activation_truth.is_some() {
                    super::print_surface_line(
                        render,
                        "project activation",
                        &format!(
                            "{} (activation_pending={})",
                            project_activation_status.unwrap_or("pending"),
                            project_activation_pending
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
                if latest_run_graph_snapshot_inconsistent {
                    super::print_surface_line(
                        render,
                        "latest run graph next action",
                        run_graph_latest_snapshot_inconsistent_next_action(),
                    );
                }
                if latest_run_graph_dispatch_receipt_signal_ambiguous {
                    super::print_surface_line(
                        render,
                        "latest run graph dispatch receipt next action",
                        run_graph_latest_dispatch_receipt_signal_ambiguous_next_action(),
                    );
                }
                if latest_run_graph_dispatch_receipt_summary_inconsistent {
                    super::print_surface_line(
                        render,
                        "latest run graph dispatch receipt blocker",
                        "run_graph_latest_dispatch_receipt_summary_inconsistent",
                    );
                    super::print_surface_line(
                        render,
                        "latest run graph dispatch receipt summary next action",
                        run_graph_latest_dispatch_receipt_summary_inconsistent_next_action(),
                    );
                }
                if latest_run_graph_dispatch_receipt_checkpoint_leakage {
                    super::print_surface_line(
                        render,
                        "latest run graph dispatch receipt blocker",
                        "run_graph_latest_dispatch_receipt_checkpoint_leakage",
                    );
                    super::print_surface_line(
                        render,
                        "latest run graph dispatch receipt checkpoint leakage next action",
                        run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action(),
                    );
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
                    super::print_surface_line(
                        render,
                        "root session write guard",
                        host_agents["root_session_write_guard"]["status"]
                            .as_str()
                            .unwrap_or("missing"),
                    );
                    if host_agents["external_cli_preflight"]["status"]
                        .as_str()
                        .is_some_and(|value| value == "blocked")
                    {
                        super::print_surface_line(
                            render,
                            "external cli preflight",
                            host_agents["external_cli_preflight"]["blocker_code"]
                                .as_str()
                                .unwrap_or("blocked"),
                        );
                        if let Some(next_actions) =
                            host_agents["external_cli_preflight"]["next_actions"].as_array()
                        {
                            for action in next_actions {
                                if let Some(text) = action.as_str() {
                                    super::print_surface_line(
                                        render,
                                        "external cli next action",
                                        text,
                                    );
                                }
                            }
                        }
                    }
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
    let (selected_cli_system, host_cli_entry) = selected_host_cli_system_entry(&overlay);
    let runtime_surface =
        runtime_surface_for_selected_system(&selected_cli_system, host_cli_entry.as_ref());
    let observability =
        super::read_json_file_if_present(&super::host_agent_observability_state_path(project_root))
            .unwrap_or_else(|| {
                super::load_or_initialize_host_agent_observability_state(project_root)
            });
    let latest_events = observability["events"]
        .as_array()
        .map(|events| events.iter().rev().take(5).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let recent_events_value = serde_json::Value::Array(latest_events);
    let budget_value = observability["budget"].clone();
    let runtime_root = runtime_root_for_selected_system(
        project_root,
        &selected_cli_system,
        host_cli_entry.as_ref(),
    );

    let mut payload = serde_json::Map::new();
    payload.insert(
        "host_cli_system".to_string(),
        serde_json::Value::String(selected_cli_system.clone()),
    );
    payload.insert(
        "runtime_surface".to_string(),
        serde_json::Value::String(runtime_surface),
    );
    payload.insert(
        "runtime_root".to_string(),
        serde_json::Value::String(runtime_root),
    );
    payload.insert(
        "external_cli_preflight".to_string(),
        external_cli_preflight_summary(&overlay, &selected_cli_system, host_cli_entry.as_ref()),
    );
    payload.insert("budget".to_string(), budget_value);
    payload.insert("recent_events".to_string(), recent_events_value);
    payload.insert("selection_policy".to_string(), serde_json::Value::Null);
    payload.insert("agents".to_string(), serde_json::json!({}));
    payload.insert(
        "internal_dispatch_alias_count".to_string(),
        serde_json::Value::Null,
    );
    payload.insert(
        "internal_dispatch_alias_load_error".to_string(),
        serde_json::Value::Null,
    );
    payload.insert(
        "system_entry".to_string(),
        host_cli_system_entry_summary(host_cli_entry.as_ref(), &selected_cli_system),
    );

    let (carrier_system, carrier_catalog) =
        super::project_activator_surface::resolved_host_cli_agent_catalog_for_root(
            project_root,
            &overlay,
        )
        .unwrap_or_else(|_| (selected_cli_system.clone(), Vec::new()));
    let strategy =
        super::read_json_file_if_present(&super::worker_strategy_state_path(project_root))
            .unwrap_or(serde_json::Value::Null);
    let scorecards =
        super::read_json_file_if_present(&super::worker_scorecards_state_path(project_root))
            .unwrap_or(serde_json::Value::Null);

    let mut agents = serde_json::Map::new();
    for role in &carrier_catalog {
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
    if agents.is_empty() {
        agents = host_cli_system_carrier_summary(host_cli_entry.as_ref(), &selected_cli_system);
    }

    payload.insert(
        "selection_policy".to_string(),
        strategy["selection_policy"].clone(),
    );
    payload.insert(
        "agents".to_string(),
        serde_json::Value::Object(agents.clone()),
    );
    if let Some(system_entry) = payload
        .get_mut("system_entry")
        .and_then(serde_json::Value::as_object_mut)
    {
        let carriers_empty = system_entry
            .get("carriers")
            .is_none_or(|value| value.is_null() || value == &serde_json::json!({}));
        if carriers_empty {
            system_entry.insert(
                "carriers".to_string(),
                synthesized_host_cli_carrier_contract(&agents),
            );
        }
    }
    if carrier_system == "codex" && !carrier_catalog.is_empty() {
        let overlay_dispatch_aliases_result =
            super::project_activator_surface::codex_dispatch_alias_catalog_for_root(
                &overlay,
                project_root,
                &carrier_catalog,
            );
        let internal_dispatch_alias_load_error = overlay_dispatch_aliases_result
            .as_ref()
            .err()
            .map(std::string::ToString::to_string);
        let overlay_dispatch_aliases = overlay_dispatch_aliases_result.unwrap_or_default();
        payload.insert(
            "internal_dispatch_alias_count".to_string(),
            serde_json::json!(overlay_dispatch_aliases.len()),
        );
        payload.insert(
            "internal_dispatch_alias_load_error".to_string(),
            internal_dispatch_alias_load_error
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    payload.insert(
        "stores".to_string(),
        serde_json::json!({
            "scorecards": if strategy.is_null() { serde_json::Value::Null } else { serde_json::Value::String(super::WORKER_SCORECARDS_STATE.to_string()) },
            "strategy": if strategy.is_null() { serde_json::Value::Null } else { serde_json::Value::String(super::WORKER_STRATEGY_STATE.to_string()) },
            "observability": super::HOST_AGENT_OBSERVABILITY_STATE,
        }),
    );
    Some(serde_json::Value::Object(payload))
}

pub(crate) fn root_session_write_guard_summary_from_snapshot_path(
    snapshot_path: Option<&str>,
) -> serde_json::Value {
    let Some(path) = snapshot_path else {
        return serde_json::json!({
            "status": "missing",
            "reason": "runtime_consumption_snapshot_missing",
            "local_write_requires_exception_path": serde_json::Value::Null,
            "required_exception_evidence": serde_json::Value::Null,
        });
    };
    let snapshot = super::read_json_file_if_present(Path::new(path));
    let Some(snapshot) = snapshot else {
        return serde_json::json!({
            "status": "missing",
            "reason": "runtime_consumption_snapshot_unreadable",
            "local_write_requires_exception_path": serde_json::Value::Null,
            "required_exception_evidence": serde_json::Value::Null,
        });
    };
    let mut guard = runtime_root_session_write_guard_from_snapshot(&snapshot);
    if guard.is_none() {
        if let Some(path) = latest_final_runtime_snapshot_path(Path::new(path)) {
            if let Some(fallback_snapshot) = super::read_json_file_if_present(&path) {
                guard = runtime_root_session_write_guard_from_snapshot(&fallback_snapshot);
            }
        }
    }
    let guard = guard.unwrap_or(serde_json::Value::Null);
    let guard_ok = has_runtime_root_session_write_guard(&guard);
    serde_json::json!({
        "status": if guard_ok { "blocked_by_default" } else { "missing" },
        "reason": if guard_ok { serde_json::Value::Null } else { serde_json::Value::String("missing_root_session_write_guard".to_string()) },
        "root_session_role": guard["root_session_role"].clone(),
        "local_write_requires_exception_path": guard["local_write_requires_exception_path"].clone(),
        "required_exception_evidence": guard["required_exception_evidence"].clone(),
        "pre_write_checkpoint_required": guard["pre_write_checkpoint_required"].clone(),
    })
}

fn runtime_root_session_write_guard_from_snapshot(
    snapshot: &serde_json::Value,
) -> Option<serde_json::Value> {
    let direct_guard =
        &snapshot["payload"]["role_selection"]["execution_plan"]["root_session_write_guard"];
    if has_runtime_root_session_write_guard(direct_guard) {
        return Some(direct_guard.clone());
    }
    let execution_plan_contract_guard = &snapshot["payload"]["role_selection"]["execution_plan"]
        ["orchestration_contract"]["root_session_write_guard"];
    if has_runtime_root_session_write_guard(execution_plan_contract_guard) {
        return Some(execution_plan_contract_guard.clone());
    }

    let dispatch_packet_path = snapshot["source_dispatch_packet_path"]
        .as_str()
        .or_else(|| snapshot["dispatch_receipt"]["dispatch_packet_path"].as_str())?;
    let packet = super::read_json_file_if_present(Path::new(dispatch_packet_path))?;
    let packet_guard = &packet["root_session_write_guard"];
    if has_runtime_root_session_write_guard(packet_guard) {
        return Some(packet_guard.clone());
    }
    let packet_contract_guard = &packet["orchestration_contract"]["root_session_write_guard"];
    if has_runtime_root_session_write_guard(packet_contract_guard) {
        return Some(packet_contract_guard.clone());
    }

    None
}

fn latest_final_runtime_snapshot_path(latest_snapshot_path: &Path) -> Option<std::path::PathBuf> {
    let snapshot_dir = latest_snapshot_path.parent()?;
    let mut latest_final: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(snapshot_dir).ok()?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("final-") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &latest_final {
            Some((latest_modified, _)) if modified <= *latest_modified => {}
            _ => latest_final = Some((modified, path)),
        }
    }
    latest_final.map(|(_, path)| path)
}

fn host_cli_system_entry_summary(
    entry: Option<&serde_yaml::Value>,
    system: &str,
) -> serde_json::Value {
    let enabled = entry
        .map(|value| super::yaml_bool(super::yaml_lookup(value, &["enabled"]), true))
        .unwrap_or(true);
    let template_root = entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["template_root"])))
        .unwrap_or_else(|| format!(".{system}"));
    let runtime_root = entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])))
        .unwrap_or_else(|| format!(".{system}"));
    let materialization_mode = entry
        .and_then(|value| super::yaml_lookup(value, &["materialization_mode"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| default_host_cli_materialization_mode(system));
    let execution_class = entry
        .map(|value| {
            super::project_activator_surface::host_cli_system_execution_class(value, system)
        })
        .unwrap_or_else(|| "unknown".to_string());
    let carriers = entry
        .and_then(|value| super::yaml_lookup(value, &["carriers"]))
        .filter(|value| !value.is_null())
        .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})))
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "enabled": enabled,
        "execution_class": execution_class,
        "materialization_mode": materialization_mode,
        "template_root": template_root,
        "runtime_root": runtime_root,
        "carriers": carriers,
    })
}

fn host_cli_system_carrier_summary(
    entry: Option<&serde_yaml::Value>,
    system: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut agents = serde_json::Map::new();
    let Some(serde_yaml::Value::Mapping(carriers)) =
        entry.and_then(|value| super::yaml_lookup(value, &["carriers"]))
    else {
        return synthesized_host_cli_carrier_summary(system);
    };
    for (carrier_id, carrier_value) in carriers {
        let Some(carrier_id) = carrier_id
            .as_str()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        let carrier = serde_json::to_value(carrier_value).unwrap_or_else(|_| serde_json::json!({}));
        agents.insert(
            carrier_id.to_string(),
            serde_json::json!({
                "tier": carrier["tier"].clone(),
                "rate": carrier["rate"].clone(),
                "reasoning_band": carrier["reasoning_band"].clone(),
                "default_runtime_role": carrier["default_runtime_role"].clone(),
                "runtime_roles": carrier["runtime_roles"].clone(),
                "task_classes": carrier["task_classes"].clone(),
                "feedback_count": 0,
                "last_feedback_at": serde_json::Value::Null,
                "last_feedback_outcome": serde_json::Value::Null,
                "effective_score": serde_json::Value::Null,
                "lifecycle_state": serde_json::Value::Null,
            }),
        );
    }
    if agents.is_empty() {
        synthesized_host_cli_carrier_summary(system)
    } else {
        agents
    }
}

fn synthesized_host_cli_carrier_summary(
    system: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut agents = serde_json::Map::new();
    if system.trim().is_empty() || system.eq_ignore_ascii_case("codex") {
        return agents;
    }
    agents.insert(
        format!("{system}-primary"),
        serde_json::json!({
            "tier": system,
            "rate": 4,
            "reasoning_band": "medium",
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation", "research"],
            "feedback_count": 0,
            "last_feedback_at": serde_json::Value::Null,
            "last_feedback_outcome": serde_json::Value::Null,
            "effective_score": serde_json::Value::Null,
            "lifecycle_state": serde_json::Value::Null,
        }),
    );
    agents
}

fn synthesized_host_cli_carrier_contract(
    agents: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut carriers = serde_json::Map::new();
    for (carrier_id, summary) in agents {
        carriers.insert(
            carrier_id.clone(),
            serde_json::json!({
                "tier": summary["tier"].clone(),
                "rate": summary["rate"].clone(),
                "reasoning_band": summary["reasoning_band"].clone(),
                "default_runtime_role": summary["default_runtime_role"].clone(),
                "runtime_roles": summary["runtime_roles"].clone(),
                "task_classes": summary["task_classes"].clone(),
            }),
        );
    }
    serde_json::Value::Object(carriers)
}

fn default_host_cli_materialization_mode(system: &str) -> String {
    if system == "codex" {
        "codex_toml_catalog_render".to_string()
    } else {
        "copy_tree_only".to_string()
    }
}

fn host_agents_json_value(host_agents: Option<&serde_json::Value>) -> serde_json::Value {
    host_agents
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_activation_status, external_cli_preflight_summary, host_cli_system_entry_summary,
        release1_operator_contracts_consistency_error, selected_host_cli_system_entry,
        shared_operator_output_contract_parity_error,
    };
    use crate::state_store;

    #[test]
    fn release1_operator_contracts_consistency_accepts_pass_without_blockers() {
        assert_eq!(
            release1_operator_contracts_consistency_error("pass", &[], &[]),
            None
        );
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_pass_with_blockers() {
        let blocker_codes = vec!["boot_incompatible".to_string()];
        assert_eq!(
            release1_operator_contracts_consistency_error("pass", &blocker_codes, &[]),
            Some(
                "operator contract inconsistency: status=pass must not include blocker_codes"
                    .to_string()
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
                &["migration_required".to_string()],
                &["Complete required migration before normal operation.".to_string()]
            ),
            None
        );
        assert_eq!(
            release1_operator_contracts_consistency_error(" Ok ", &[], &[]),
            None
        );
    }

    #[test]
    fn project_activation_status_normalizes_case_and_whitespace_drift() {
        assert_eq!(
            canonical_activation_status(Some(" PENDING_ACTIVATION "), false),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some(" ready_enough_for_normal_work "), false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(
            canonical_activation_status(Some(" unknown "), false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(
            canonical_activation_status(Some(" ready_enough_for_normal_work "), true),
            "pending"
        );
    }

    #[test]
    fn selected_host_cli_system_entry_prefers_enabled_configured_system_without_codex_fallback() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: "__HOST_CLI_SYSTEM__"
  systems:
    codex:
      enabled: false
    qwen:
      enabled: false
    acme:
      enabled: true
      runtime_root: .acme
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        assert_eq!(selected, "acme");
        assert_eq!(
            crate::yaml_string(crate::yaml_lookup(
                entry.as_ref().expect("entry should exist"),
                &["runtime_root"]
            ))
            .as_deref(),
            Some(".acme")
        );
    }

    #[test]
    fn selected_host_cli_system_entry_falls_back_to_sorted_registry_when_all_systems_disabled() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: ""
  systems:
    codex:
      enabled: false
    qwen:
      enabled: false
    acme:
      enabled: false
      runtime_root: .acme
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        assert_eq!(selected, "acme");
        assert_eq!(
            crate::yaml_string(crate::yaml_lookup(
                entry.as_ref().expect("entry should exist"),
                &["runtime_root"]
            ))
            .as_deref(),
            Some(".acme")
        );
    }

    #[test]
    fn external_cli_preflight_respects_configured_execution_class() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: acme
  systems:
    acme:
      enabled: true
      execution_class: internal
      runtime_root: .acme
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        let summary = super::external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["selected_execution_class"], "internal");
    }

    #[test]
    fn external_cli_preflight_defaults_to_unknown_without_registry_entry() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems: {}
"#,
        )
        .expect("overlay yaml should parse");

        let summary = external_cli_preflight_summary(&overlay, "codex", None);
        assert_eq!(summary["selected_execution_class"], "unknown");
    }

    #[test]
    fn host_cli_system_entry_summary_defaults_execution_class_to_unknown_without_entry() {
        let summary = host_cli_system_entry_summary(None, "codex");
        assert_eq!(summary["execution_class"], "unknown");
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
            "blocker_codes": ["missing_protocol_binding_receipt"],
            "next_actions": [" Run `vida taskflow protocol-binding sync --json` "],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["missing_protocol_binding_receipt"],
                "next_actions": ["run `vida taskflow protocol-binding sync --json`"]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["missing_protocol_binding_receipt"],
                "next_actions": ["RUN `VIDA TASKFLOW PROTOCOL-BINDING SYNC --JSON`"]
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
                "blocker_codes": ["migration_required"],
                "next_actions": ["Complete required migration before normal operation."]
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
        );
    }

    #[test]
    fn shared_operator_output_contract_parity_rejects_operator_ok_shared_blocked() {
        let summary_json = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["missing_protocol_binding_receipt"],
            "next_actions": ["Run `vida taskflow protocol-binding sync --json`"],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["missing_protocol_binding_receipt"],
                "next_actions": ["Run `vida taskflow protocol-binding sync --json`"]
            },
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": []
            }
        });
        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
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

    #[test]
    fn shared_operator_output_contract_parity_rejects_protocol_binding_surface_drift() {
        let summary_json = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": ["missing_protocol_binding_receipt"],
                "next_actions": ["Run `vida taskflow protocol-binding sync --json`"]
            },
            "operator_contracts": {
                "status": "blocked",
                "blocker_codes": ["missing_protocol_binding_receipt"],
                "next_actions": ["Run `vida taskflow protocol-binding sync --json`"]
            }
        });

        assert_eq!(
            shared_operator_output_contract_parity_error(&summary_json),
            Some("top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch")
        );
    }

    #[test]
    fn latest_run_graph_snapshot_inconsistent_has_explicit_next_action_and_contracts_remain_valid()
    {
        let next_action = super::run_graph_latest_snapshot_inconsistent_next_action().to_string();
        assert!(next_action.contains("recheck `vida status --json`"));
        assert_eq!(
            release1_operator_contracts_consistency_error(
                "blocked",
                &["run_graph_latest_snapshot_inconsistent".to_string()],
                &[next_action],
            ),
            None
        );
    }

    #[test]
    fn latest_run_graph_dispatch_receipt_checkpoint_leakage_has_explicit_next_action_and_contracts_remain_valid(
    ) {
        let next_action =
            super::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action().to_string();
        assert!(next_action.contains("checkpoint evidence"));
        assert!(next_action.contains("same run_id"));
        assert_eq!(
            release1_operator_contracts_consistency_error(
                "blocked",
                &["run_graph_latest_dispatch_receipt_checkpoint_leakage".to_string()],
                &[next_action],
            ),
            None
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["run_graph_latest_dispatch_receipt_checkpoint_leakage"],
                "next_actions": [super::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
                "shared_fields": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_checkpoint_leakage"],
                    "next_actions": [super::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
                    "artifact_refs": {}
                },
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_checkpoint_leakage"],
                    "next_actions": [super::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
                    "artifact_refs": {}
                }
            })),
            None
        );
    }

    #[test]
    fn latest_run_graph_dispatch_receipt_signal_ambiguous_blocks_drifted_lane_status() {
        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-vida-a".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
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
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };

        assert!(state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(&receipt));
        assert_eq!(
            release1_operator_contracts_consistency_error(
                "blocked",
                &["run_graph_latest_dispatch_receipt_signal_ambiguous".to_string()],
                &[
                    super::run_graph_latest_dispatch_receipt_signal_ambiguous_next_action()
                        .to_string()
                ],
            ),
            None
        );
    }

    #[test]
    fn latest_run_graph_dispatch_receipt_summary_inconsistent_blocks_missing_or_mismatched_run_id()
    {
        assert!(
            state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                Some("run-vida-a"),
                None
            )
        );
        assert!(
            state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                Some("run-vida-a"),
                Some("run-vida-b")
            )
        );
        assert!(
            !state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                Some("run-vida-a"),
                Some("run-vida-a")
            )
        );

        let next_action =
            super::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action().to_string();
        assert!(next_action.contains("run-graph dispatch receipt summary"));
        assert_eq!(
            release1_operator_contracts_consistency_error(
                "blocked",
                &["run_graph_latest_dispatch_receipt_summary_inconsistent".to_string()],
                &[next_action],
            ),
            None
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["run_graph_latest_dispatch_receipt_summary_inconsistent"],
                "next_actions": [super::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
                "shared_fields": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_summary_inconsistent"],
                    "next_actions": [super::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
                    "artifact_refs": {}
                },
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_summary_inconsistent"],
                    "next_actions": [super::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
                    "artifact_refs": {}
                }
            })),
            None
        );
    }

    #[test]
    fn latest_run_graph_dispatch_receipt_matches_status_accepts_matching_run_ids() {
        assert!(
            state_store::latest_run_graph_dispatch_receipt_matches_status(
                Some("run-vida-a"),
                Some("run-vida-a")
            )
        );
    }

    #[test]
    fn latest_run_graph_dispatch_receipt_matches_status_rejects_missing_or_mismatched_run_ids() {
        assert!(
            !state_store::latest_run_graph_dispatch_receipt_matches_status(
                Some("run-vida-a"),
                None
            )
        );
        assert!(
            !state_store::latest_run_graph_dispatch_receipt_matches_status(
                Some("run-vida-a"),
                Some("run-vida-b")
            )
        );
        assert!(
            !state_store::latest_run_graph_dispatch_receipt_matches_status(
                None,
                Some("run-vida-a")
            )
        );
    }

    #[test]
    fn latest_run_graph_evidence_snapshot_is_consistent_rejects_mismatched_gate_run_id() {
        assert!(
            !state_store::latest_run_graph_evidence_snapshot_is_consistent(
                Some("run-vida-a"),
                Some("run-vida-a"),
                Some("run-vida-a"),
                Some("run-vida-b"),
                Some("run-vida-a")
            )
        );
    }
}
