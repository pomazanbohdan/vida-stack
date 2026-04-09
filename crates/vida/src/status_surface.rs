use std::process::ExitCode;

use crate::{state_store, state_store::StateStore, StatusArgs};

use crate::status_surface_json_report::{build_status_json_report, StatusJsonReportInputs};
use crate::status_surface_operator_contracts::{
    build_status_operator_contracts, StatusOperatorContractInputs,
};
use crate::status_surface_signals::final_snapshot_missing_release_admission_evidence;
use crate::status_surface_text_report::{emit_status_text_report, StatusTextReportInputs};
use crate::status_surface_truth_inputs::build_status_truth_inputs;

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
                let status_truth_inputs = build_status_truth_inputs(
                    store.root(),
                    runtime_consumption.latest_snapshot_path.as_deref(),
                );
                let host_agents = status_truth_inputs.host_agents;
                let latest_final_snapshot_path = status_truth_inputs.latest_final_snapshot_path;
                let latest_recorded_final_snapshot_path =
                    status_truth_inputs.latest_recorded_final_snapshot_path;
                let mut root_session_write_guard = status_truth_inputs.root_session_write_guard;
                let activation_truth = status_truth_inputs.activation_truth;
                let project_activation_status = status_truth_inputs.project_activation_status;
                let project_activation_pending = status_truth_inputs.project_activation_pending;
                root_session_write_guard =
                    crate::status_surface_write_guard::merge_live_exception_takeover_write_guard(
                        root_session_write_guard,
                        latest_run_graph_dispatch_receipt.as_ref(),
                        latest_run_graph_recovery.as_ref(),
                    );
                let mut host_agents = host_agents;
                if let Some(host_agents_value) = host_agents.as_mut() {
                    if let Some(object) = host_agents_value.as_object_mut() {
                        object.insert(
                            "root_session_write_guard".to_string(),
                            root_session_write_guard.clone(),
                        );
                    }
                }
                if as_json {
                    let incomplete_release_admission_operator_evidence =
                        latest_recorded_final_snapshot_path
                            .as_deref()
                            .map(final_snapshot_missing_release_admission_evidence)
                            .unwrap_or(true);
                    let operator_contracts =
                        match build_status_operator_contracts(StatusOperatorContractInputs {
                            boot_compatibility: boot_compatibility.as_ref(),
                            migration_state: migration_state.as_ref(),
                            protocol_binding: &protocol_binding,
                            runtime_consumption: &runtime_consumption,
                            latest_final_snapshot_path: latest_final_snapshot_path.as_deref(),
                            latest_run_graph_dispatch_receipt_id: latest_run_graph_dispatch_receipt
                                .as_ref()
                                .map(|receipt| receipt.run_id.as_str()),
                            latest_run_graph_gate_present: latest_run_graph_gate.is_some(),
                            latest_run_graph_dispatch_receipt_matches_status,
                            latest_run_graph_snapshot_inconsistent,
                            latest_run_graph_dispatch_receipt_signal_ambiguous,
                            latest_run_graph_dispatch_receipt_summary_inconsistent,
                            latest_run_graph_dispatch_receipt_checkpoint_leakage,
                            incomplete_release_admission_operator_evidence,
                            activation_truth: activation_truth.as_ref(),
                            project_activation_pending,
                            latest_task_reconciliation: latest_task_reconciliation.as_ref(),
                            effective_bundle_receipt: effective_bundle_receipt.as_ref(),
                            root_session_write_guard_status: root_session_write_guard["status"]
                                .as_str()
                                .unwrap_or(""),
                            root_local_write_allowed: root_session_write_guard
                                ["root_local_write_allowed"]
                                .as_bool()
                                .unwrap_or(false),
                            activation_view_only_dispatch_blocker_active: root_session_write_guard
                                ["activation_view_only_dispatch_blocker_active"]
                                .as_bool()
                                .unwrap_or(false),
                            blocking_dispatch_blocker_code: root_session_write_guard
                                ["blocking_dispatch_blocker_code"]
                                .as_str(),
                        }) {
                            Ok(value) => value,
                            Err(error) => {
                                eprintln!("Failed to render status json: {error}");
                                return ExitCode::from(1);
                            }
                        };
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
                    if let Some(error) =
                        crate::operator_contracts::release1_operator_contracts_consistency_error(
                            operator_contracts["status"].as_str().unwrap_or(""),
                            &blocker_codes,
                            &next_actions,
                        )
                    {
                        eprintln!("Failed to render status json: {error}");
                        return ExitCode::from(1);
                    }
                    let summary_json = match build_status_json_report(StatusJsonReportInputs {
                        summary_only,
                        operator_contracts,
                        backend_summary: &backend_summary,
                        state_dir: store.root(),
                        storage_metadata: &storage_metadata,
                        state_spine: &state_spine,
                        effective_bundle_receipt: effective_bundle_receipt.as_ref(),
                        boot_compatibility: boot_compatibility.as_ref(),
                        migration_state: migration_state.as_ref(),
                        migration_receipts: &migration_receipts,
                        latest_task_reconciliation: latest_task_reconciliation.as_ref(),
                        task_reconciliation_rollup: &task_reconciliation_rollup,
                        snapshot_bridge: &snapshot_bridge,
                        runtime_consumption: &runtime_consumption,
                        protocol_binding: &protocol_binding,
                        activation_truth: activation_truth.as_ref(),
                        project_activation_status: project_activation_status.as_deref(),
                        project_activation_pending,
                        host_agents: host_agents.as_ref(),
                        root_session_write_guard: &root_session_write_guard,
                        latest_run_graph_status: latest_run_graph_status.as_ref(),
                        latest_run_graph_recovery: latest_run_graph_recovery.as_ref(),
                        latest_run_graph_checkpoint: latest_run_graph_checkpoint.as_ref(),
                        latest_run_graph_gate: latest_run_graph_gate.as_ref(),
                        latest_run_graph_dispatch_receipt: latest_run_graph_dispatch_receipt
                            .as_ref(),
                    }) {
                        Ok(summary_json) => summary_json,
                        Err(error) => {
                            eprintln!("Failed to render status json: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary_json)
                            .expect("status summary should render as json")
                    );
                    return ExitCode::SUCCESS;
                }

                return emit_status_text_report(StatusTextReportInputs {
                    render,
                    backend_summary: &backend_summary,
                    state_dir: store.root(),
                    state_spine: &state_spine,
                    effective_bundle_receipt: effective_bundle_receipt.as_ref(),
                    boot_compatibility: boot_compatibility.as_ref(),
                    migration_state: migration_state.as_ref(),
                    migration_receipts: &migration_receipts,
                    latest_task_reconciliation: latest_task_reconciliation.as_ref(),
                    task_reconciliation_rollup: &task_reconciliation_rollup,
                    snapshot_bridge: &snapshot_bridge,
                    runtime_consumption: &runtime_consumption,
                    protocol_binding: &protocol_binding,
                    activation_truth: activation_truth.as_ref(),
                    project_activation_status: project_activation_status.as_deref(),
                    project_activation_pending,
                    latest_run_graph_status: latest_run_graph_status.as_ref(),
                    latest_run_graph_recovery: latest_run_graph_recovery.as_ref(),
                    latest_run_graph_checkpoint: latest_run_graph_checkpoint.as_ref(),
                    latest_run_graph_gate: latest_run_graph_gate.as_ref(),
                    latest_run_graph_snapshot_inconsistent,
                    latest_run_graph_dispatch_receipt_signal_ambiguous,
                    latest_run_graph_dispatch_receipt_summary_inconsistent,
                    latest_run_graph_dispatch_receipt_checkpoint_leakage,
                    host_agents: host_agents.as_ref(),
                });
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

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use crate::activation_status::canonical_activation_status;
    use crate::operator_contracts::release1_operator_contracts_consistency_error;
    use crate::operator_contracts::shared_operator_output_contract_parity_error;
    use crate::status_surface_external_cli::external_cli_preflight_summary;
    use crate::status_surface_host_cli_summary::host_cli_system_entry_summary;
    use crate::status_surface_host_cli_system::selected_host_cli_system_entry;
    use crate::status_surface_signals::{
        run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action,
        run_graph_latest_dispatch_receipt_signal_ambiguous_next_action,
        run_graph_latest_dispatch_receipt_summary_inconsistent_next_action,
        run_graph_latest_snapshot_inconsistent_next_action,
    };
    use crate::status_surface_write_guard::root_session_write_guard_summary_from_snapshot_path;
    use crate::{blocker_code_str, state_store, BlockerCode};

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
        let summary = external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["hybrid_external_cli_relevant"], false);
        assert_eq!(summary["selected_execution_class"], "internal");
        assert_eq!(summary["tool_contract"]["status"], "pass");
        assert_eq!(summary["tool_contract"]["artifact_type"], "tool_contract");
        assert_eq!(
            summary["tool_contract"]["auth_mode"],
            "project_runtime_internal"
        );
        assert_eq!(
            summary["tool_contract"]["idempotency_class"],
            "read_only_probe"
        );
    }

    #[test]
    fn external_cli_preflight_keeps_optional_external_subagents_non_blocking_for_internal_host() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        let summary = external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(summary["external_cli_subagents_present"], true);
        assert_eq!(summary["hybrid_external_cli_relevant"], true);
        assert_eq!(summary["selected_execution_class"], "internal");
        assert_eq!(summary["tool_contract"]["status"], "pass");
        assert_eq!(
            summary["tool_contract"]["policy_hook_ids"],
            serde_json::json!([
                "execution_class_gate",
                "runtime_root_resolution",
                "sandbox_network_gate"
            ])
        );
    }

    #[test]
    fn external_cli_preflight_requires_external_cli_for_external_host_with_configured_runtime_root() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: opencode
  systems:
    opencode:
      enabled: true
      execution_class: external
      runtime_root: .opencode
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        let summary = external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(summary["hybrid_external_cli_relevant"], false);
        assert_eq!(summary["selected_execution_class"], "external");
        assert_eq!(
            summary["tool_contract"]["auth_mode"],
            "delegated_host_session"
        );
        assert_eq!(summary["tool_contract"]["status"], "pass");
    }

    #[test]
    fn root_session_write_guard_summary_backfills_canonical_defaults_for_legacy_snapshot() {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-root-session-guard-legacy-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let snapshot_path = root.join("runtime-consumption/final-legacy.json");
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "payload": {
                    "role_selection": {
                        "execution_plan": {
                            "root_session_write_guard": {
                                "status": "blocked_by_default",
                                "root_session_role": "orchestrator",
                                "local_write_requires_exception_path": true,
                                "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
                                "pre_write_checkpoint_required": true
                            }
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("legacy snapshot should write");

        let summary = root_session_write_guard_summary_from_snapshot_path(snapshot_path.to_str());
        assert_eq!(summary["status"], "blocked_by_default");
        assert_eq!(summary["lawful_write_surface"], "vida agent-init");
        assert_eq!(
            summary["host_local_write_capability_is_not_authority"],
            true
        );
        assert_eq!(summary["root_local_write_allowed"], false);
        assert_eq!(
            summary["activation_view_only_dispatch_blocker_active"],
            false
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn root_session_write_guard_summary_marks_activation_view_only_dispatch_blocker() {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-root-session-guard-activation-view-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let snapshot_path = root.join("runtime-consumption/final-activation-view.json");
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "payload": {
                    "role_selection": {
                        "execution_plan": {
                            "root_session_write_guard": {
                                "status": "blocked_by_default",
                                "root_session_role": "orchestrator",
                                "local_write_requires_exception_path": true,
                                "root_local_write_allowed": false,
                                "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
                                "pre_write_checkpoint_required": true
                            }
                        }
                    }
                },
                "dispatch_receipt": {
                    "blocker_code": "internal_activation_view_only"
                }
            })
            .to_string(),
        )
        .expect("snapshot should write");

        let summary = root_session_write_guard_summary_from_snapshot_path(snapshot_path.to_str());
        assert_eq!(summary["root_local_write_allowed"], false);
        assert_eq!(
            summary["blocking_dispatch_blocker_code"],
            "internal_activation_view_only"
        );
        assert_eq!(
            summary["activation_view_only_dispatch_blocker_active"],
            true
        );

        let _ = fs::remove_dir_all(&root);
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
        assert_eq!(summary["tool_contract"]["status"], "blocked");
        assert_eq!(
            summary["tool_contract"]["blocker_code"],
            blocker_code_str(BlockerCode::ToolContractMissing)
        );
        assert_eq!(summary["status"], "blocked");
    }

    #[test]
    fn external_cli_preflight_marks_tool_contract_incomplete_when_runtime_root_is_missing() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: external
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        let summary = external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["selected_execution_class"], "external");
        assert_eq!(summary["tool_contract"]["status"], "blocked");
        assert_eq!(
            summary["tool_contract"]["blocker_code"],
            blocker_code_str(BlockerCode::ToolContractIncomplete)
        );
        assert_eq!(
            summary["tool_contract"]["auth_mode"],
            "delegated_host_session"
        );
        assert_eq!(summary["status"], "blocked");
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
        let next_action = run_graph_latest_snapshot_inconsistent_next_action().to_string();
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
            run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action().to_string();
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
                "next_actions": [run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
                "shared_fields": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_checkpoint_leakage"],
                    "next_actions": [run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
                    "artifact_refs": {}
                },
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_checkpoint_leakage"],
                    "next_actions": [run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action()],
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
                &[run_graph_latest_dispatch_receipt_signal_ambiguous_next_action().to_string()],
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
            run_graph_latest_dispatch_receipt_summary_inconsistent_next_action().to_string();
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
                "next_actions": [run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
                "shared_fields": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_summary_inconsistent"],
                    "next_actions": [run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
                    "artifact_refs": {}
                },
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["run_graph_latest_dispatch_receipt_summary_inconsistent"],
                    "next_actions": [run_graph_latest_dispatch_receipt_summary_inconsistent_next_action()],
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
