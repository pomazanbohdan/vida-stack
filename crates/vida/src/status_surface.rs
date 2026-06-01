use std::process::ExitCode;
use std::time::Duration;

use crate::{state_store, state_store::StateStore, StatusArgs};

use crate::status_surface_json_report::{build_status_json_report, StatusJsonReportInputs};
use crate::status_surface_operator_contracts::{
    build_status_operator_contracts, StatusOperatorContractInputs,
};
use crate::status_surface_text_report::{emit_status_text_report, StatusTextReportInputs};
use crate::status_surface_truth_inputs::build_status_truth_inputs;

const STATUS_SURFACE_LOCK_TIMEOUT: Duration = Duration::from_secs(15);
const STATUS_SURFACE_RECENT_PROJECTION_MAX_AGE: Duration = Duration::from_secs(300);
pub(crate) fn degraded_read_lock_payload(
    surface: &str,
    state_dir: &std::path::Path,
    error: &str,
) -> serde_json::Value {
    let blocker_codes = vec!["state_store_read_lock_contention"];
    let next_actions = vec![
        "Retry the read surface after concurrent VIDA state readers finish; this degraded response avoided opening the locked datastore.",
    ];
    serde_json::json!({
        "surface": surface,
        "status": "blocked",
        "degraded": true,
        "state_access": {
            "mode": "degraded_lock_contention",
            "degraded": true,
            "state_dir": state_dir.display().to_string(),
            "detail": "authoritative datastore was locked by another process during a read-only surface",
            "error": error,
        },
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": {
            "state_dir": state_dir.display().to_string(),
            "read_fallback": "lock_contention_degraded_response",
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": {
                "state_dir": state_dir.display().to_string(),
                "read_fallback": "lock_contention_degraded_response",
            },
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": {
                "state_dir": state_dir.display().to_string(),
                "read_fallback": "lock_contention_degraded_response",
            },
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
    })
}

pub(crate) fn emit_degraded_read_lock_surface(
    surface: &str,
    state_dir: &std::path::Path,
    render: crate::RenderMode,
    as_json: bool,
    error: &str,
) -> ExitCode {
    let payload = degraded_read_lock_payload(surface, state_dir, error);
    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(render, surface);
        crate::print_surface_line(render, "status", "blocked");
        crate::print_surface_line(render, "state access", "degraded_lock_contention");
        crate::print_surface_line(render, "state dir", &state_dir.display().to_string());
    }
    ExitCode::from(1)
}

pub(crate) async fn run_status(args: StatusArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;
    let summary_only = args.summary;

    if as_json && summary_only {
        if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
            &state_dir,
            status_json_projection_name(summary_only),
        ) {
            println!(
                "{}",
                render_cached_status_projection_for_operator(summary_only, &cached)
            );
            return ExitCode::SUCCESS;
        }
        if summary_only {
            if let Some(cached) =
            crate::operator_projection_cache::read_state_fresh_json_projection_for_read_only_operator(
                &state_dir,
                status_json_projection_name(summary_only),
            )
            {
                println!(
                    "{}",
                    render_cached_status_projection_for_operator(summary_only, &cached)
                );
                return ExitCode::SUCCESS;
            }
        }
        if let Some(cached) = crate::operator_projection_cache::read_recent_json_projection(
            &state_dir,
            status_json_projection_name(summary_only),
            STATUS_SURFACE_RECENT_PROJECTION_MAX_AGE,
        )
        .filter(|cached| cached_status_projection_admissible(&state_dir, summary_only, cached))
        {
            println!(
                "{}",
                render_cached_status_projection_for_operator(summary_only, &cached)
            );
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                &state_dir,
                status_json_projection_name(summary_only),
                STATUS_SURFACE_RECENT_PROJECTION_MAX_AGE,
            )
            .filter(|cached| cached_status_projection_admissible(&state_dir, summary_only, cached))
        {
            println!(
                "{}",
                render_cached_status_projection_for_operator(summary_only, &cached)
            );
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_state_stale_recent_json_projection(
                &state_dir,
                status_json_projection_name(summary_only),
                STATUS_SURFACE_RECENT_PROJECTION_MAX_AGE,
            )
            .filter(|cached| cached_status_projection_admissible(&state_dir, summary_only, cached))
        {
            let rendered = if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay(
                    &state_dir,
                ) {
                crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_payload(
                    &state_dir,
                    &cached,
                    &overlay,
                )
                .unwrap_or_else(|| cached.clone())
            } else {
                cached.clone()
            };
            println!(
                "{}",
                render_cached_status_projection_for_operator(summary_only, &rendered)
            );
            return ExitCode::SUCCESS;
        }
    }

    match StateStore::open_existing_read_only_with_timeout(
        state_dir.clone(),
        STATUS_SURFACE_LOCK_TIMEOUT,
    )
    .await
    {
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
                let latest_run_graph_status =
                    match store.latest_run_graph_status_for_current_session().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph status: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_run_id = latest_run_graph_status
                    .as_ref()
                    .map(|status| status.run_id.as_str());
                let latest_run_graph_recovery = match latest_run_graph_run_id {
                    Some(run_id) => match store.run_graph_recovery_summary(run_id).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph recovery summary: {error}");
                            return ExitCode::from(1);
                        }
                    }
                    .into(),
                    None => None,
                };
                let latest_run_graph_checkpoint = match latest_run_graph_run_id {
                    Some(run_id) => match store.run_graph_checkpoint_summary(run_id).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!(
                                "Failed to read latest run graph checkpoint summary: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    }
                    .into(),
                    None => None,
                };
                let latest_run_graph_gate = match latest_run_graph_run_id {
                    Some(run_id) => match store.run_graph_gate_summary(run_id).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph gate summary: {error}");
                            return ExitCode::from(1);
                        }
                    }
                    .into(),
                    None => None,
                };
                let mut latest_run_graph_dispatch_receipt_checkpoint_leakage = false;
                let latest_run_graph_dispatch_receipt = match latest_run_graph_status.as_ref() {
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
                                latest_run_graph_dispatch_receipt_checkpoint_leakage = true;
                                None
                            } else {
                                eprintln!(
                                "Failed to read latest run graph dispatch receipt summary: {error}"
                            );
                                return ExitCode::from(1);
                            }
                        }
                    },
                    None => None,
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
                let task_store = match store.task_store_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read task store summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let no_active_taskflow_work = task_store.open_count == 0
                    && task_store.in_progress_count == 0
                    && task_store.ready_count == 0;
                let explicit_continuation_binding = match store
                    .latest_explicit_run_graph_continuation_binding_for_current_session()
                    .await
                {
                    Ok(binding) => binding,
                    Err(error) => {
                        eprintln!("Failed to read latest explicit continuation binding: {error}");
                        return ExitCode::from(1);
                    }
                };
                let all_tasks = match StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root())
                {
                    Ok(tasks) => tasks,
                    Err(_) => match store.list_tasks(None, true).await {
                        Ok(tasks) => tasks,
                        Err(error) => {
                            eprintln!(
                                "Failed to read tasks for TaskFlow active work truth: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    },
                };
                let (latest_run_graph_task_closed, latest_run_graph_task_missing) =
                    match latest_run_graph_status.as_ref() {
                        Some(status) => {
                            match all_tasks.iter().find(|task| task.id == status.task_id) {
                                Some(task) => (task.status == "closed", false),
                                None => (false, true),
                            }
                        }
                        None => (false, false),
                    };
                let continuation_binding =
                    crate::continuation_binding_summary::build_continuation_binding_summary_with_task_authority(
                        explicit_continuation_binding.as_ref(),
                        latest_run_graph_status.as_ref(),
                        latest_run_graph_recovery.as_ref(),
                        latest_run_graph_dispatch_receipt.as_ref(),
                        if latest_run_graph_dispatch_receipt.as_ref().is_some_and(|receipt| {
                            (receipt.supersedes_receipt_id.is_some()
                                && receipt.exception_path_receipt_id.is_some())
                                || crate::continuation_binding_summary::dispatch_summary_has_clean_ready_downstream_handoff(
                                    Some(receipt),
                                    receipt.run_id.as_str(),
                                )
                        }) {
                            crate::latest_terminal_consume_continue_snapshot_run_id(store.root())
                                .ok()
                                .flatten()
                        } else {
                            None
                        }
                        .as_deref(),
                        latest_run_graph_snapshot_inconsistent
                            || latest_run_graph_dispatch_receipt_signal_ambiguous
                            || latest_run_graph_dispatch_receipt_summary_inconsistent
                            || latest_run_graph_dispatch_receipt_checkpoint_leakage,
                        no_active_taskflow_work,
                        latest_run_graph_task_closed,
                        latest_run_graph_task_missing,
                    );
                let in_progress_tasks = all_tasks
                    .iter()
                    .filter(|task| task.status == "in_progress")
                    .cloned()
                    .collect::<Vec<_>>();
                let taskflow_active_candidates =
                    crate::continuation_binding_summary::taskflow_active_candidates_from_tasks(
                        &in_progress_tasks,
                    );
                let has_taskflow_active_candidates = !taskflow_active_candidates.is_empty();
                let continuation_binding =
                    crate::continuation_binding_summary::add_taskflow_active_work_truth(
                        continuation_binding,
                        taskflow_active_candidates,
                    );
                let continuation_binding_ambiguous = continuation_binding["status"].as_str()
                    == Some("ambiguous")
                    && (has_taskflow_active_candidates
                        || continuation_binding["continuation_required_now"]
                            .as_bool()
                            .unwrap_or(false));
                let status_truth_inputs = build_status_truth_inputs(
                    store.root(),
                    runtime_consumption.latest_snapshot_path.as_deref(),
                    summary_only,
                );
                let host_agents = status_truth_inputs.host_agents;
                let latest_release_admission_operator_evidence_snapshot_path =
                    status_truth_inputs.latest_release_admission_operator_evidence_snapshot_path;
                let latest_final_snapshot_path = if summary_only {
                    status_truth_inputs.latest_final_snapshot_path
                } else {
                    latest_release_admission_operator_evidence_snapshot_path
                        .clone()
                        .or(status_truth_inputs.latest_final_snapshot_path)
                };
                let mut root_session_write_guard = status_truth_inputs.root_session_write_guard;
                let activation_truth = status_truth_inputs.activation_truth;
                let project_activation_status = status_truth_inputs.project_activation_status;
                let project_activation_pending = status_truth_inputs.project_activation_pending;
                root_session_write_guard =
                    crate::status_surface_write_guard::merge_live_exception_takeover_write_guard_with_task_authority(
                        root_session_write_guard,
                        store.root(),
                        latest_run_graph_dispatch_receipt.as_ref(),
                        latest_run_graph_recovery.as_ref(),
                        latest_run_graph_task_missing || latest_run_graph_task_closed,
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
                let project_root =
                    crate::taskflow_task_bridge::infer_project_root_from_state_root(store.root())
                        .or_else(|| std::env::current_dir().ok())
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                let launcher_runtime_paths =
                    match crate::doctor_launcher_summary_for_root(&project_root) {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to resolve launcher/runtime paths: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_surface_truth = latest_run_graph_dispatch_receipt
                    .as_ref()
                    .and_then(|receipt| {
                        crate::runtime_dispatch_state::dispatch_surface_truth_from_packet_path(
                            &project_root,
                            receipt.dispatch_packet_path.as_deref(),
                            receipt,
                        )
                    });
                let latest_run_graph_mixed_posture = latest_run_graph_surface_truth
                    .as_ref()
                    .and_then(|value| value.get("mixed_posture"));
                let latest_run_graph_activation_vs_execution_evidence =
                    latest_run_graph_surface_truth
                        .as_ref()
                        .and_then(|value| value.get("activation_vs_execution_evidence"));
                if as_json {
                    let operator_session_projection =
                        match build_operator_session_projection_for_status(&store).await {
                            Ok(value) => value,
                            Err(error) => {
                                eprintln!("Failed to build operator session projection: {error}");
                                return ExitCode::from(1);
                            }
                        };
                    let incomplete_release_admission_operator_evidence =
                        match if latest_release_admission_operator_evidence_snapshot_path.is_some()
                        {
                            Ok(false)
                        } else if summary_only {
                            crate::runtime_consumption_state::release_admission_operator_evidence_incomplete_from_latest_snapshot(
                                runtime_consumption.latest_snapshot_path.as_deref(),
                            )
                        } else if latest_final_snapshot_path.is_some() {
                            crate::runtime_consumption_state::release_admission_operator_evidence_incomplete(
                                store.root(),
                            )
                        } else {
                            Ok(true)
                        } {
                            Ok(value) => value,
                            Err(error) => {
                                eprintln!("Failed to evaluate release-admission evidence: {error}");
                                return ExitCode::from(1);
                            }
                        };
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
                            continuation_binding_ambiguous,
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
                            root_local_write_allowed_for_only_these_paths:
                                &root_session_write_guard
                                    ["root_local_write_allowed_for_only_these_paths"],
                            activation_view_only_dispatch_blocker_active: root_session_write_guard
                                ["activation_view_only_dispatch_blocker_active"]
                                .as_bool()
                                .unwrap_or(false),
                            blocking_dispatch_blocker_code: root_session_write_guard
                                ["blocking_dispatch_blocker_code"]
                                .as_str(),
                            operator_session_projection: &operator_session_projection,
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
                        crate::contract_profile_adapter::operator_contracts_consistency_error(
                            operator_contracts["status"].as_str().unwrap_or(""),
                            &blocker_codes,
                            &next_actions,
                        )
                    {
                        eprintln!("Failed to render status json: {error}");
                        return ExitCode::from(1);
                    }
                    let mut summary_json = match build_status_json_report(StatusJsonReportInputs {
                        summary_only,
                        operator_contracts,
                        backend_summary: &backend_summary,
                        state_dir: store.root(),
                        launcher_runtime_paths: &launcher_runtime_paths,
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
                        operator_session_projection: &operator_session_projection,
                        continuation_binding: &continuation_binding,
                        latest_run_graph_status: latest_run_graph_status.as_ref(),
                        latest_run_graph_recovery: latest_run_graph_recovery.as_ref(),
                        latest_run_graph_checkpoint: latest_run_graph_checkpoint.as_ref(),
                        latest_run_graph_gate: latest_run_graph_gate.as_ref(),
                        latest_run_graph_dispatch_receipt: latest_run_graph_dispatch_receipt
                            .as_ref(),
                        latest_run_graph_mixed_posture,
                        latest_run_graph_activation_vs_execution_evidence,
                    }) {
                        Ok(summary_json) => summary_json,
                        Err(error) => {
                            eprintln!("Failed to render status json: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    if !summary_only {
                        compact_status_projection_for_fast_operator_render(&mut summary_json);
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary_json)
                            .expect("status summary should render as json")
                    );
                    crate::operator_projection_cache::write_json_projection(
                        store.root(),
                        status_json_projection_name(summary_only),
                        &summary_json,
                    );
                    return ExitCode::SUCCESS;
                }

                let operator_session_projection =
                    match build_operator_session_projection_for_status(&store).await {
                        Ok(value) => value,
                        Err(error) => {
                            eprintln!("Failed to build operator session projection: {error}");
                            return ExitCode::from(1);
                        }
                    };
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
                    operator_session_projection: &operator_session_projection,
                    continuation_binding: &continuation_binding,
                    host_agents: host_agents.as_ref(),
                    latest_run_graph_dispatch_receipt: latest_run_graph_dispatch_receipt.as_ref(),
                    latest_run_graph_mixed_posture,
                    latest_run_graph_activation_vs_execution_evidence,
                });
            }
            Err(error) => {
                eprintln!("Failed to read storage metadata: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            if StateStore::error_is_lock_contention(&error) {
                return emit_degraded_read_lock_surface(
                    "vida status",
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

fn status_json_projection_name(summary_only: bool) -> &'static str {
    if summary_only {
        "status-summary-v2-latest"
    } else {
        "status-full-latest"
    }
}

async fn build_operator_session_projection_for_status(
    store: &StateStore,
) -> Result<serde_json::Value, state_store::StateStoreError> {
    match crate::operator_session_projection::build_operator_session_projection(store).await {
        Ok(value) => Ok(value),
        Err(error)
            if crate::operator_session_projection::is_optional_task_worktree_assignment_missing_error(
                &error,
            ) =>
        {
            Ok(crate::operator_session_projection::degraded_operator_session_projection(
                store.root(),
                &error.to_string(),
            ))
        }
        Err(error) => Err(error),
    }
}

fn render_cached_status_projection_for_operator(summary_only: bool, cached: &str) -> String {
    if summary_only {
        return cached.to_string();
    }
    let Ok(mut payload) = serde_json::from_str::<serde_json::Value>(cached) else {
        return cached.to_string();
    };
    compact_status_projection_for_fast_operator_render(&mut payload);
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| cached.to_string())
}

fn compact_status_projection_for_fast_operator_render(payload: &mut serde_json::Value) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    object.insert("view".to_string(), serde_json::json!("operator_compact"));
    if let Some(host_agents) = object
        .get_mut("host_agents")
        .and_then(serde_json::Value::as_object_mut)
    {
        let agent_count = host_agents
            .get("agents")
            .and_then(serde_json::Value::as_object)
            .map(serde_json::Map::len)
            .unwrap_or(0);
        let backend_count = host_agents
            .get("subagent_backends")
            .and_then(serde_json::Value::as_object)
            .map(serde_json::Map::len)
            .unwrap_or(0);
        host_agents.insert(
            "agents".to_string(),
            serde_json::json!({
                "count": agent_count,
                "detail": "omitted_from_cached_operator_compact_status"
            }),
        );
        host_agents.insert(
            "subagent_backends".to_string(),
            serde_json::json!({
                "count": backend_count,
                "detail": "omitted_from_cached_operator_compact_status"
            }),
        );
    }
    if let Some(runtime_owner_evidence) = object
        .get_mut("operator_session_projection")
        .and_then(|projection| projection.get_mut("runtime_owner_evidence"))
        .and_then(serde_json::Value::as_object_mut)
    {
        if let Some(stale_sessions) = runtime_owner_evidence.get_mut("stale_sessions") {
            let stale_count = stale_sessions.as_array().map(Vec::len).unwrap_or_default();
            *stale_sessions = serde_json::json!({
                "count": stale_count,
                "detail": "omitted_from_cached_operator_compact_status"
            });
        }
    }
}

async fn refresh_cached_status_projection_runtime_fields(
    state_dir: &std::path::Path,
    cached: &str,
) -> Option<String> {
    let mut payload = serde_json::from_str::<serde_json::Value>(cached).ok()?;
    let store = StateStore::open_existing_read_only_with_timeout(
        state_dir.to_path_buf(),
        Duration::from_secs(2),
    )
    .await
    .ok()?;
    let latest_run_graph_status = match store.latest_run_graph_status_for_current_session().await {
        Ok(summary) => summary,
        Err(_) => return None,
    };
    let latest_run_graph_run_id = latest_run_graph_status
        .as_ref()
        .map(|status| status.run_id.as_str());
    let latest_run_graph_recovery = match latest_run_graph_run_id {
        Some(run_id) => store.run_graph_recovery_summary(run_id).await.ok(),
        None => None,
    };
    let latest_run_graph_checkpoint = match latest_run_graph_run_id {
        Some(run_id) => store.run_graph_checkpoint_summary(run_id).await.ok(),
        None => None,
    };
    let latest_run_graph_gate = match latest_run_graph_run_id {
        Some(run_id) => store.run_graph_gate_summary(run_id).await.ok(),
        None => None,
    };
    let mut dispatch_receipt_checkpoint_leakage = false;
    let latest_run_graph_dispatch_receipt = match latest_run_graph_status.as_ref() {
        Some(status) => match store
            .run_graph_dispatch_receipt_summary_for_status(status)
            .await
        {
            Ok(summary) => summary,
            Err(error)
                if error
                    .to_string()
                    .contains("latest checkpoint evidence must share the same run_id") =>
            {
                dispatch_receipt_checkpoint_leakage = true;
                None
            }
            Err(_) => return None,
        },
        None => None,
    };
    let latest_run_graph_snapshot_inconsistent = !dispatch_receipt_checkpoint_leakage
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
    let latest_run_graph_dispatch_receipt_signal_ambiguous = latest_run_graph_dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(receipt)
        });
    let latest_run_graph_dispatch_receipt_summary_inconsistent =
        !dispatch_receipt_checkpoint_leakage
            && state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                latest_run_graph_status
                    .as_ref()
                    .map(|status| status.run_id.as_str()),
                latest_run_graph_dispatch_receipt
                    .as_ref()
                    .map(|receipt| receipt.run_id.as_str()),
            );
    let task_store = store.task_store_summary().await.ok()?;
    let no_active_taskflow_work = task_store.open_count == 0
        && task_store.in_progress_count == 0
        && task_store.ready_count == 0;
    let explicit_continuation_binding = match store
        .latest_explicit_run_graph_continuation_binding_for_current_session()
        .await
    {
        Ok(binding) => binding,
        Err(_) => return None,
    };
    let all_tasks = store.list_tasks(None, true).await.ok()?;
    let (latest_run_graph_task_closed, latest_run_graph_task_missing) =
        match latest_run_graph_status.as_ref() {
            Some(status) => match all_tasks.iter().find(|task| task.id == status.task_id) {
                Some(task) => (task.status == "closed", false),
                None => (false, true),
            },
            None => (false, false),
        };
    let terminal_consume_continue_run_id = if latest_run_graph_dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            (receipt.supersedes_receipt_id.is_some() && receipt.exception_path_receipt_id.is_some())
                || crate::continuation_binding_summary::dispatch_summary_has_clean_ready_downstream_handoff(
                    Some(receipt),
                    receipt.run_id.as_str(),
                )
        }) {
        crate::latest_terminal_consume_continue_snapshot_run_id(store.root())
            .ok()
            .flatten()
    } else {
        None
    };
    let continuation_binding =
        crate::continuation_binding_summary::build_continuation_binding_summary_with_task_authority(
            explicit_continuation_binding.as_ref(),
            latest_run_graph_status.as_ref(),
            latest_run_graph_recovery.as_ref(),
            latest_run_graph_dispatch_receipt.as_ref(),
            terminal_consume_continue_run_id.as_deref(),
            latest_run_graph_snapshot_inconsistent
                || latest_run_graph_dispatch_receipt_signal_ambiguous
                || latest_run_graph_dispatch_receipt_summary_inconsistent
                || dispatch_receipt_checkpoint_leakage,
            no_active_taskflow_work,
            latest_run_graph_task_closed,
            latest_run_graph_task_missing,
        );
    let in_progress_tasks = all_tasks
        .iter()
        .filter(|task| task.status == "in_progress")
        .cloned()
        .collect::<Vec<_>>();
    let taskflow_active_candidates =
        crate::continuation_binding_summary::taskflow_active_candidates_from_tasks(
            &in_progress_tasks,
        );
    let continuation_binding = crate::continuation_binding_summary::add_taskflow_active_work_truth(
        continuation_binding,
        taskflow_active_candidates,
    );
    let mut root_session_write_guard = payload["root_session_write_guard"].clone();
    root_session_write_guard =
        crate::status_surface_write_guard::merge_live_exception_takeover_write_guard_with_task_authority(
            root_session_write_guard,
            store.root(),
            latest_run_graph_dispatch_receipt.as_ref(),
            latest_run_graph_recovery.as_ref(),
            latest_run_graph_task_missing || latest_run_graph_task_closed,
        );

    let project_root =
        crate::taskflow_task_bridge::infer_project_root_from_state_root(store.root())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
    let latest_run_graph_surface_truth =
        latest_run_graph_dispatch_receipt
            .as_ref()
            .and_then(|receipt| {
                crate::runtime_dispatch_state::dispatch_surface_truth_from_packet_path(
                    &project_root,
                    receipt.dispatch_packet_path.as_deref(),
                    receipt,
                )
            });
    let latest_run_graph_mixed_posture = latest_run_graph_surface_truth
        .as_ref()
        .and_then(|value| value.get("mixed_posture"));
    let latest_run_graph_activation_vs_execution_evidence = latest_run_graph_surface_truth
        .as_ref()
        .and_then(|value| value.get("activation_vs_execution_evidence"));
    let latest_run_graph_status_json = crate::status_surface_json_report::enrich_run_graph_status(
        latest_run_graph_status.as_ref(),
        latest_run_graph_mixed_posture,
        latest_run_graph_activation_vs_execution_evidence,
    );
    let latest_run_graph_dispatch_receipt_json =
        crate::status_surface_json_report::enrich_run_graph_dispatch_receipt(
            latest_run_graph_dispatch_receipt.as_ref(),
            latest_run_graph_mixed_posture,
            latest_run_graph_activation_vs_execution_evidence,
        );
    let latest_run_graph_dispatch_compact_summary =
        crate::taskflow_run_graph::build_run_graph_dispatch_compact_summary(
            store.root(),
            latest_run_graph_status.as_ref(),
            latest_run_graph_recovery.as_ref(),
            latest_run_graph_dispatch_receipt.as_ref(),
            Some(&continuation_binding),
            latest_run_graph_activation_vs_execution_evidence,
        );
    let latest_run_graph_dispatch_route_truth = latest_run_graph_dispatch_compact_summary
        .as_ref()
        .and_then(|summary| serde_json::to_value(&summary.route_truth).ok())
        .unwrap_or(serde_json::Value::Null);
    let latest_run_graph_downstream_dispatch_preview = latest_run_graph_dispatch_compact_summary
        .as_ref()
        .and_then(|summary| serde_json::to_value(&summary.downstream_dispatch_preview).ok())
        .unwrap_or(serde_json::Value::Null);
    let projection_name = payload
        .get("projection_cache")
        .and_then(|cache| cache.get("projection_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("status")
        .to_string();

    let object = payload.as_object_mut()?;
    object.insert("continuation_binding".to_string(), continuation_binding);
    object.insert(
        "root_session_write_guard".to_string(),
        root_session_write_guard.clone(),
    );
    if let Some(host_agents) = object
        .get_mut("host_agents")
        .and_then(serde_json::Value::as_object_mut)
    {
        host_agents.insert(
            "root_session_write_guard".to_string(),
            root_session_write_guard.clone(),
        );
    }
    object.insert(
        "latest_run_graph_status".to_string(),
        latest_run_graph_status_json,
    );
    object.insert(
        "latest_run_graph_delegation_gate".to_string(),
        latest_run_graph_status
            .as_ref()
            .map(|status| serde_json::to_value(status.delegation_gate()).ok())
            .flatten()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "latest_run_graph_recovery".to_string(),
        serde_json::to_value(&latest_run_graph_recovery).ok()?,
    );
    object.insert(
        "latest_run_graph_checkpoint".to_string(),
        serde_json::to_value(&latest_run_graph_checkpoint).ok()?,
    );
    object.insert(
        "latest_run_graph_gate".to_string(),
        serde_json::to_value(&latest_run_graph_gate).ok()?,
    );
    object.insert(
        "latest_run_graph_dispatch_receipt".to_string(),
        latest_run_graph_dispatch_receipt_json,
    );
    object.insert(
        "latest_run_graph_dispatch_route_truth".to_string(),
        latest_run_graph_dispatch_route_truth,
    );
    object.insert(
        "latest_run_graph_downstream_dispatch_preview".to_string(),
        latest_run_graph_downstream_dispatch_preview,
    );
    object.insert(
        "latest_run_graph_dispatch_compact_summary".to_string(),
        serde_json::to_value(&latest_run_graph_dispatch_compact_summary).ok()?,
    );
    object.insert(
        "projection_cache".to_string(),
        serde_json::json!({
            "status": "state_marker_stale_recent_projection_with_live_runtime_overlay",
            "projection_name": projection_name,
            "freshness_contract": "cached_structural_status_with_live_continuation_run_graph_and_write_guard_overlay"
        }),
    );
    serde_json::to_string_pretty(&payload).ok()
}

fn cached_status_projection_admissible(
    state_dir: &std::path::Path,
    _summary_only: bool,
    cached: &str,
) -> bool {
    serde_json::from_str::<serde_json::Value>(cached)
        .ok()
        .is_some_and(|payload| {
            payload
                .get("surface")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|surface| surface == "vida status")
                && payload
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .is_some()
                && cached_status_projection_matches_current_session(state_dir, &payload)
        })
}

fn cached_status_projection_matches_current_session(
    state_dir: &std::path::Path,
    payload: &serde_json::Value,
) -> bool {
    let cached_worktree_environment_id = payload["current_session"]["worktree_environment_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(cached_session_id) = payload["current_session"]["session_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return cached_worktree_environment_id.is_some_and(|cached_id| {
            let Ok(owner_evidence) =
                crate::orchestrator_session_surface::build_runtime_owner_evidence(state_dir, false)
            else {
                return false;
            };
            owner_evidence["current_session"]["worktree_environment_id"]
                .as_str()
                .map(str::trim)
                .is_some_and(|current_id| current_id == cached_id)
        });
    };
    let Ok(owner_evidence) =
        crate::orchestrator_session_surface::build_runtime_owner_evidence(state_dir, false)
    else {
        return false;
    };
    if let Some(cached_id) = cached_worktree_environment_id {
        if owner_evidence["current_session"]["worktree_environment_id"]
            .as_str()
            .map(str::trim)
            .is_some_and(|current_id| current_id == cached_id)
        {
            return true;
        }
    }
    owner_evidence["current_session"]["session_id"]
        .as_str()
        .map(str::trim)
        .is_some_and(|current_session_id| current_session_id == cached_session_id)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Mutex, OnceLock},
        time::SystemTime,
    };

    use crate::activation_status::canonical_activation_status;
    use crate::contract_profile_adapter::operator_contracts_consistency_error;
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

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn restore_vida_session_id(saved: Option<String>) {
        unsafe {
            match saved {
                Some(value) => std::env::set_var("VIDA_SESSION_ID", value),
                None => std::env::remove_var("VIDA_SESSION_ID"),
            }
        }
    }

    #[test]
    fn status_summary_projection_cache_key_is_shape_versioned() {
        assert_eq!(
            super::status_json_projection_name(true),
            "status-summary-v2-latest"
        );
        assert_eq!(
            super::status_json_projection_name(false),
            "status-full-latest"
        );
    }

    #[test]
    fn status_projection_cache_is_session_identity_scoped() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-cache-session-scope-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temp state root");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "status-cache-session-a");
        }
        let payload = serde_json::json!({
            "surface": "vida status",
            "status": "pass",
            "current_session": {
                "session_id": "status-cache-session-a"
            }
        });
        assert!(super::cached_status_projection_admissible(
            &root,
            false,
            &payload.to_string()
        ));

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "status-cache-session-b");
        }
        assert!(!super::cached_status_projection_admissible(
            &root,
            false,
            &payload.to_string()
        ));

        let _ = fs::remove_dir_all(root);
        restore_vida_session_id(saved_session_id);
    }

    #[test]
    fn status_projection_cache_accepts_same_worktree_across_process_scoped_sessions() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-cache-worktree-scope-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temp state root");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "status-cache-worktree-a");
        }
        let owner_evidence =
            crate::orchestrator_session_surface::build_runtime_owner_evidence(&root, false)
                .expect("owner evidence should build");
        let worktree_environment_id = owner_evidence["current_session"]["worktree_environment_id"]
            .as_str()
            .expect("worktree id should be present")
            .to_string();
        let payload = serde_json::json!({
            "surface": "vida status",
            "status": "pass",
            "current_session": {
                "session_id": "status-cache-worktree-a",
                "worktree_environment_id": worktree_environment_id
            }
        });

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "status-cache-worktree-b");
        }
        assert!(super::cached_status_projection_admissible(
            &root,
            false,
            &payload.to_string()
        ));

        let _ = fs::remove_dir_all(root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn status_stale_projection_overlay_refreshes_continuation_binding() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::remove_var("VIDA_SESSION_ID");
        }
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-live-overlay-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open state store");
        store
            .create_task(state_store::CreateTaskRequest {
                task_id: "task-live-status-parent",
                title: "Live status overlay parent",
                display_id: None,
                description: "test parent",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: state_store::TaskExecutionSemantics::default(),
                planner_metadata: state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("create live parent task");
        store
            .create_task(state_store::CreateTaskRequest {
                task_id: "task-live-status",
                title: "Live status overlay task",
                display_id: None,
                description: "test task",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("task-live-status-parent"),
                labels: &[],
                execution_semantics: state_store::TaskExecutionSemantics::default(),
                planner_metadata: state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("create live task");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-live-status",
            "implementation",
            "implementation",
        );
        status.task_id = "task-live-status".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run graph status");
        store
            .record_run_graph_continuation_binding(&state_store::RunGraphContinuationBinding {
                run_id: "run-live-status".to_string(),
                task_id: "task-live-status".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "run_id": "run-live-status",
                    "task_id": "task-live-status",
                    "active_node": "coach"
                }),
                binding_source: "latest_run_graph_exception_takeover_dispatch".to_string(),
                why_this_unit: "test live binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_exception_takeover".to_string(),
                request_text: None,
                recorded_at: "2026-05-21T00:00:00Z".to_string(),
            })
            .await
            .expect("record continuation binding");
        assert_eq!(
            store
                .latest_run_graph_status()
                .await
                .expect("read latest run graph")
                .expect("latest run graph exists")
                .run_id,
            "run-live-status"
        );
        drop(store);

        let cached = serde_json::json!({
            "surface": "vida status",
            "status": "pass",
            "root_session_write_guard": {},
            "continuation_binding": {
                "status": "ambiguous",
                "active_bounded_unit": serde_json::Value::Null,
                "why_this_unit": serde_json::Value::Null,
                "sequential_vs_parallel_posture": "unknown_until_explicit_taskflow_binding"
            }
        });
        let refreshed =
            super::refresh_cached_status_projection_runtime_fields(&root, &cached.to_string())
                .await
                .expect("stale cached status should refresh live runtime fields");
        let payload: serde_json::Value =
            serde_json::from_str(&refreshed).expect("refreshed status should remain json");

        assert_eq!(
            payload["latest_run_graph_status"]["run_id"],
            "run-live-status"
        );
        assert_eq!(
            payload["projection_cache"]["status"],
            "state_marker_stale_recent_projection_with_live_runtime_overlay"
        );

        let _ = fs::remove_dir_all(root);
        restore_vida_session_id(saved_session_id);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn status_stale_projection_overlay_does_not_import_foreign_run_graph_evidence() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-status-foreign-overlay-{}-{nanos}",
            std::process::id()
        ));

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "foreign-status-overlay-session");
        }
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open state store");
        store
            .create_task(state_store::CreateTaskRequest {
                task_id: "task-foreign-status-parent",
                title: "Foreign status parent",
                display_id: None,
                description: "test parent",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: state_store::TaskExecutionSemantics::default(),
                planner_metadata: state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("create foreign parent task");
        store
            .create_task(state_store::CreateTaskRequest {
                task_id: "task-foreign-status",
                title: "Foreign status task",
                display_id: None,
                description: "test task",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("task-foreign-status-parent"),
                labels: &[],
                execution_semantics: state_store::TaskExecutionSemantics::default(),
                planner_metadata: state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("create foreign task");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-foreign-status",
            "implementation",
            "implementation",
        );
        status.task_id = "task-foreign-status".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("record foreign run graph status");
        store
            .record_run_graph_continuation_binding(&state_store::RunGraphContinuationBinding {
                run_id: "run-foreign-status".to_string(),
                task_id: "task-foreign-status".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "run_id": "run-foreign-status",
                    "task_id": "task-foreign-status",
                    "active_node": "coach"
                }),
                binding_source: "latest_run_graph_exception_takeover_dispatch".to_string(),
                why_this_unit: "foreign session binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_exception_takeover".to_string(),
                request_text: None,
                recorded_at: "2026-05-21T00:00:00Z".to_string(),
            })
            .await
            .expect("record foreign continuation binding");
        drop(store);

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "current-status-overlay-session");
        }
        let cached = serde_json::json!({
            "surface": "vida status",
            "status": "pass",
            "root_session_write_guard": {},
            "continuation_binding": {
                "status": "ambiguous",
                "active_bounded_unit": serde_json::Value::Null,
                "why_this_unit": serde_json::Value::Null,
                "sequential_vs_parallel_posture": "unknown_until_explicit_taskflow_binding"
            }
        });
        let refreshed =
            super::refresh_cached_status_projection_runtime_fields(&root, &cached.to_string())
                .await
                .expect("stale cached status should refresh without importing foreign evidence");
        let payload: serde_json::Value =
            serde_json::from_str(&refreshed).expect("refreshed status should remain json");

        assert!(payload["latest_run_graph_status"].is_null());
        assert!(payload["latest_run_graph_recovery"].is_null());
        assert!(payload["latest_run_graph_dispatch_receipt"].is_null());
        assert!(payload["continuation_binding"]["active_bounded_unit"].is_null());

        let _ = fs::remove_dir_all(root);
        restore_vida_session_id(saved_session_id);
    }

    #[test]
    fn release1_operator_contracts_consistency_accepts_pass_without_blockers() {
        assert_eq!(operator_contracts_consistency_error("pass", &[], &[]), None);
    }

    #[test]
    fn release1_operator_contracts_consistency_rejects_pass_with_blockers() {
        let blocker_codes = vec!["boot_incompatible".to_string()];
        assert_eq!(
            operator_contracts_consistency_error("pass", &blocker_codes, &[]),
            Some(
                "operator contract inconsistency: status=pass must not include blocker_codes"
                    .to_string()
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
                &["migration_required".to_string()],
                &["Complete required migration before normal operation.".to_string()]
            ),
            None
        );
        assert_eq!(operator_contracts_consistency_error(" Ok ", &[], &[]), None);
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
    fn selected_host_cli_system_entry_prefers_enabled_configured_system_without_legacy_fallback() {
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
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay yaml should parse");

        let (selected, entry) = selected_host_cli_system_entry(&overlay);
        let summary = external_cli_preflight_summary(&overlay, &selected, entry.as_ref());
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
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
    fn external_cli_preflight_requires_external_cli_for_external_host_with_configured_runtime_root()
    {
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
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
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
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
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
            Some(
                "top-level/operator_contracts/shared_fields status/blocker_codes/next_actions mirror mismatch"
            )
        );
    }

    #[test]
    fn latest_run_graph_snapshot_inconsistent_has_explicit_next_action_and_contracts_remain_valid()
    {
        let next_action = run_graph_latest_snapshot_inconsistent_next_action().to_string();
        assert!(next_action.contains("recheck `vida status --json`"));
        assert_eq!(
            operator_contracts_consistency_error(
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
            operator_contracts_consistency_error(
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
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };

        assert!(state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(&receipt));
        assert_eq!(
            operator_contracts_consistency_error(
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
            operator_contracts_consistency_error(
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
