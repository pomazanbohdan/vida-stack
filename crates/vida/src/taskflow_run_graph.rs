use crate::{
    build_runtime_execution_plan_from_snapshot, print_surface_header, print_surface_line,
    read_or_sync_launcher_activation_snapshot,
    release1_operator_output::{
        canonical_release1_blocker_code_entries, finalize_release1_operator_truth,
        shared_operator_output_contract_parity_error,
    },
    shell_quote,
    state_store::{
        RunGraphContinuationBinding, RunGraphDispatchContext, RunGraphDispatchReceipt,
        RunGraphDispatchTaskIdentity, RunGraphStatus, StateStore, StateStoreError,
    },
    taskflow_layer4::print_taskflow_proxy_help,
    taskflow_task_bridge::proxy_state_dir,
    RenderMode, RuntimeConsumptionLaneSelection,
};
use std::collections::BTreeSet;
use std::{path::PathBuf, process::ExitCode, time::Duration};
use taskflow_authority::run_graph_transition::{
    ready_run_graph_transition, run_graph_handoff, ReadyRunGraphTransitionInput,
    RunGraphDispatchTargetFormat as DispatchTargetFormat,
};
use time::format_description::well_known::Rfc3339;

const STALE_PROJECTION_DISPATCH_TIMEOUT_SECONDS: i64 = 10;
const RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS: u64 = 60;
const RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER: &str = "run_graph_dispatch_init_timeout";
const TASKFLOW_RECOVERY_RECENT_PROJECTION_MAX_AGE: Duration = Duration::from_secs(60 * 60);
const DISPATCH_INIT_FAST_CACHE_SCHEMA_VERSION: u64 = 6;
const DISPATCH_INIT_IDENTITY_BACKFILL_OPEN_TIMEOUT: Duration = Duration::from_secs(15);

fn projection_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn recovery_projection_name(run_id: &str) -> String {
    format!("taskflow-recovery-status-{}", projection_component(run_id))
}

fn read_recovery_projection(
    _state_dir: &std::path::Path,
    _projection_name: &str,
    _run_id: &str,
) -> Option<String> {
    // Security hardening: recovery status JSON must be rendered from authoritative
    // state instead of repository-provided projection cache payloads.
    None
}

fn recovery_projection_has_done_action_fields(cached: &str) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(cached) else {
        return false;
    };
    let expected_command = recovery_projection_expected_action_command(&payload).or_else(|| {
        payload
            .get("projection_truth")
            .and_then(|truth| truth.get("next_lawful_operator_action"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    });
    let Some(expected_command) = expected_command else {
        return true;
    };
    let expected_command = expected_command.trim();
    let next_action_command = payload
        .get("next_action")
        .and_then(|next_action| next_action.get("command"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let recommended_command = payload
        .get("recommended_command")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    next_action_command == Some(expected_command) && recommended_command == Some(expected_command)
}

fn recovery_projection_expected_action_command(payload: &serde_json::Value) -> Option<String> {
    let receipt = payload
        .get("projection_truth")
        .and_then(|truth| truth.get("dispatch_receipt"))?;
    let dispatch_status = receipt
        .get("dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some("executed");
    let blocker_clear = receipt
        .get("blocker_code")
        .is_none_or(serde_json::Value::is_null);
    let downstream_ready = receipt
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let downstream_packet_ready = receipt
        .get("downstream_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"));
    let downstream_blockers_empty = receipt
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .is_none_or(|values| values.is_empty());
    if !(dispatch_status
        && blocker_clear
        && downstream_ready
        && downstream_packet_ready
        && downstream_blockers_empty)
    {
        return None;
    }
    let command = receipt
        .get("downstream_dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let packet_path = receipt
        .get("downstream_dispatch_packet_path")
        .and_then(serde_json::Value::as_str);
    crate::continuation_binding_summary::downstream_dispatch_command_from_parts(
        command,
        packet_path,
    )
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct RecoveryNextAction {
    command: String,
    surface: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct RecoveryWhyNotNow {
    category: String,
    summary: String,
    blocker_codes: Vec<String>,
    blocking_surface: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct RunGraphDiagnosis {
    run_id: String,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
    recovery: crate::state_store::RunGraphRecoverySummary,
    projection_truth: RunGraphProjectionTruth,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct ActiveRunRepairSummary {
    run_id: String,
    task_id: String,
    active_node: String,
    lifecycle_stage: String,
    resume_target: String,
    policy_gate: String,
    recovery_ready: bool,
    blocker_codes: Vec<String>,
    dispatch_target: Option<String>,
    dispatch_status: Option<String>,
    lane_status: Option<String>,
    downstream_dispatch_target: Option<String>,
    downstream_dispatch_ready: bool,
    dispatch_packet_path: Option<String>,
    dispatch_result_path: Option<String>,
    downstream_dispatch_packet_path: Option<String>,
    downstream_dispatch_result_path: Option<String>,
    host_bridge_request_path: Option<String>,
    result_allowed_next_node: Option<String>,
    validated_next_command: Option<String>,
    recommended_surface: Option<String>,
    stale_state_suspected: bool,
    projection_vs_receipt_parity: String,
}

fn compact_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn run_graph_operator_artifact_refs(surface: &str, run_id: &str) -> serde_json::Value {
    serde_json::json!({
        "surface": surface,
        "run_id": run_id,
    })
}

fn insert_string_artifact_ref(
    refs: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        refs.insert(key.to_string(), serde_json::json!(value));
    }
}

fn host_bridge_string_from_result(result: &serde_json::Value, result_key: &str) -> Option<String> {
    result
        .get("host_tool_bridge_request")
        .and_then(|request| request.get(result_key))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            result
                .get("host_bridge")
                .and_then(|request| request.get(result_key))
                .and_then(serde_json::Value::as_str)
        })
        .and_then(|value| compact_optional_string(Some(value)))
}

fn state_artifact_path_in_root(
    state_root: &std::path::Path,
    value: &str,
) -> Option<std::path::PathBuf> {
    fn path_without_extended_prefix(path: &std::path::Path) -> std::path::PathBuf {
        let value = path.to_string_lossy();
        std::path::PathBuf::from(value.strip_prefix(r"\\?\").unwrap_or(&value))
    }

    fn path_is_under_root(path: &std::path::Path, root: &std::path::Path) -> bool {
        path_without_extended_prefix(path).starts_with(path_without_extended_prefix(root))
    }

    let state_root = std::fs::canonicalize(state_root).ok()?;
    let raw = std::path::Path::new(value.trim());
    if raw.as_os_str().is_empty() {
        return None;
    }
    let lexical_candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        state_root.join(raw)
    };
    if lexical_candidate
        .components()
        .any(|component| component == std::path::Component::ParentDir)
        || !path_is_under_root(&lexical_candidate, &state_root)
    {
        return None;
    }
    match std::fs::canonicalize(&lexical_candidate) {
        Ok(candidate) if path_is_under_root(&candidate, &state_root) => Some(candidate),
        Ok(_) => None,
        Err(_) => Some(lexical_candidate),
    }
}

fn host_bridge_artifact_ref_from_result(
    refs: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    result: &serde_json::Value,
    result_key: &str,
    state_root: Option<&std::path::Path>,
    expected_key: &str,
) {
    let value = host_bridge_string_from_result(result, result_key);
    let Some(value) = value else {
        return;
    };
    let Some(state_root) = state_root else {
        insert_string_artifact_ref(refs, key, Some(&value));
        return;
    };
    if let Some(path) = state_artifact_path_in_root(state_root, &value)
        .filter(|path| std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()))
    {
        insert_string_artifact_ref(refs, key, path.to_str());
    } else if state_artifact_path_in_root(state_root, &value).is_some() {
        insert_string_artifact_ref(refs, expected_key, Some(&value));
    }
}

fn active_repair_summary_result_json(
    state_root: Option<&std::path::Path>,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> Option<serde_json::Value> {
    let root = state_root?;
    let receipt = receipt?;
    receipt
        .downstream_dispatch_result_path
        .as_deref()
        .and_then(|path| safe_read_dispatch_result_json(root, path))
        .or_else(|| {
            receipt
                .dispatch_result_path
                .as_deref()
                .and_then(|path| safe_read_dispatch_result_json(root, path))
        })
}

fn active_repair_blocker_codes(diagnosis: &RunGraphDiagnosis) -> Vec<String> {
    let mut values = diagnosis.blocker_codes.clone();
    if values.is_empty() {
        if let Some(receipt) = diagnosis.projection_truth.dispatch_receipt.as_ref() {
            if let Some(blocker_code) = compact_optional_string(receipt.blocker_code.as_deref()) {
                values.push(blocker_code);
            }
            values.extend(receipt.downstream_dispatch_blockers.iter().cloned());
        }
    }
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter_map(|value| compact_optional_string(Some(&value)))
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn active_run_repair_summary(
    diagnosis: &RunGraphDiagnosis,
    state_root: Option<&std::path::Path>,
) -> ActiveRunRepairSummary {
    let receipt = diagnosis.projection_truth.dispatch_receipt.as_ref();
    let result = active_repair_summary_result_json(state_root, receipt);
    let validated_next_command = diagnosis
        .recommended_command
        .clone()
        .or_else(|| {
            diagnosis
                .projection_truth
                .next_lawful_operator_action
                .clone()
        })
        .or_else(|| {
            diagnosis
                .next_action
                .as_ref()
                .map(|action| action.command.clone())
        })
        .or_else(|| receipt.and_then(|receipt| receipt.downstream_dispatch_command.clone()));
    ActiveRunRepairSummary {
        run_id: diagnosis.run_id.clone(),
        task_id: diagnosis.recovery.task_id.clone(),
        active_node: diagnosis.recovery.active_node.clone(),
        lifecycle_stage: diagnosis.recovery.lifecycle_stage.clone(),
        resume_target: diagnosis.recovery.resume_target.clone(),
        policy_gate: diagnosis.recovery.policy_gate.clone(),
        recovery_ready: diagnosis.recovery.recovery_ready,
        blocker_codes: active_repair_blocker_codes(diagnosis),
        dispatch_target: receipt.map(|receipt| receipt.dispatch_target.clone()),
        dispatch_status: receipt.map(|receipt| receipt.dispatch_status.clone()),
        lane_status: receipt.map(|receipt| receipt.lane_status.clone()),
        downstream_dispatch_target: receipt
            .and_then(|receipt| receipt.downstream_dispatch_target.clone()),
        downstream_dispatch_ready: receipt
            .map(|receipt| receipt.downstream_dispatch_ready)
            .unwrap_or(false),
        dispatch_packet_path: receipt.and_then(|receipt| receipt.dispatch_packet_path.clone()),
        dispatch_result_path: receipt.and_then(|receipt| receipt.dispatch_result_path.clone()),
        downstream_dispatch_packet_path: receipt
            .and_then(|receipt| receipt.downstream_dispatch_packet_path.clone()),
        downstream_dispatch_result_path: receipt
            .and_then(|receipt| receipt.downstream_dispatch_result_path.clone()),
        host_bridge_request_path: result
            .as_ref()
            .and_then(|result| host_bridge_string_from_result(result, "request_path")),
        result_allowed_next_node: result
            .as_ref()
            .and_then(|result| result.get("allowed_next_node"))
            .and_then(serde_json::Value::as_str)
            .and_then(|value| compact_optional_string(Some(value))),
        validated_next_command,
        recommended_surface: diagnosis.recommended_surface.clone(),
        stale_state_suspected: diagnosis.projection_truth.stale_state_suspected,
        projection_vs_receipt_parity: diagnosis
            .projection_truth
            .projection_vs_receipt_parity
            .clone(),
    }
}

fn compact_display_value(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none")
}

fn active_run_repair_summary_display(summary: &ActiveRunRepairSummary) -> String {
    let blocker_codes = if summary.blocker_codes.is_empty() {
        "none".to_string()
    } else {
        summary.blocker_codes.join(",")
    };
    format!(
        "run={} task={} active_node={} lifecycle={} blockers={} downstream_ready={} next={}",
        summary.run_id,
        summary.task_id,
        summary.active_node,
        summary.lifecycle_stage,
        blocker_codes,
        summary.downstream_dispatch_ready,
        compact_display_value(summary.validated_next_command.as_deref())
    )
}

fn active_run_repair_artifacts_display(summary: &ActiveRunRepairSummary) -> String {
    format!(
        "dispatch_packet={} dispatch_result={} downstream_packet={} downstream_result={} host_bridge_request={} result_allowed_next_node={}",
        compact_display_value(summary.dispatch_packet_path.as_deref()),
        compact_display_value(summary.dispatch_result_path.as_deref()),
        compact_display_value(summary.downstream_dispatch_packet_path.as_deref()),
        compact_display_value(summary.downstream_dispatch_result_path.as_deref()),
        compact_display_value(summary.host_bridge_request_path.as_deref()),
        compact_display_value(summary.result_allowed_next_node.as_deref())
    )
}

fn print_run_graph_diagnosis_plain(
    surface: &'static str,
    diagnosis: &RunGraphDiagnosis,
    state_root: Option<&std::path::Path>,
) {
    let active_repair_summary = active_run_repair_summary(diagnosis, state_root);
    print_surface_header(RenderMode::Plain, surface);
    print_surface_line(RenderMode::Plain, "run", &diagnosis.run_id);
    print_surface_line(
        RenderMode::Plain,
        "active_repair",
        &active_run_repair_summary_display(&active_repair_summary),
    );
    print_surface_line(
        RenderMode::Plain,
        "artifacts",
        &active_run_repair_artifacts_display(&active_repair_summary),
    );
    print_surface_line(
        RenderMode::Plain,
        "recovery",
        &diagnosis.recovery.as_display(),
    );
    print_surface_line(
        RenderMode::Plain,
        "projection",
        &diagnosis.projection_truth.projection_reason,
    );
    if !diagnosis.blocker_codes.is_empty() {
        print_surface_line(
            RenderMode::Plain,
            "blocker_codes",
            &diagnosis.blocker_codes.join(", "),
        );
    }
    if let Some(summary) = diagnosis
        .why_not_now
        .as_ref()
        .map(|value| value.summary.as_str())
    {
        print_surface_line(RenderMode::Plain, "why_not_now", summary);
    }
    if let Some(next_action) = diagnosis.next_action.as_ref() {
        print_surface_line(RenderMode::Plain, "next action", &next_action.reason);
    }
    if let Some(command) = diagnosis.recommended_command.as_deref() {
        print_surface_line(RenderMode::Plain, "recommended_command", command);
    }
    if let Some(surface) = diagnosis.recommended_surface.as_deref() {
        print_surface_line(RenderMode::Plain, "recommended_surface", surface);
    }
}

fn run_graph_repair_target(
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Option<String> {
    projection_truth
        .dispatch_receipt
        .as_ref()
        .filter(|receipt| !receipt_has_inflight_downstream_projection(receipt))
        .and_then(|receipt| {
            receipt
                .downstream_dispatch_target
                .as_deref()
                .or(receipt.downstream_dispatch_active_target.as_deref())
                .or(receipt.downstream_dispatch_last_target.as_deref())
                .or(Some(receipt.dispatch_target.as_str()))
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != "none")
                .map(str::to_string)
        })
        .or_else(|| {
            status
                .next_node
                .as_deref()
                .or(Some(status.active_node.as_str()))
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != "none")
                .map(str::to_string)
        })
}

fn receipt_has_inflight_downstream_projection(receipt: &RunGraphDispatchReceipt) -> bool {
    receipt.dispatch_kind == "agent_lane"
        && matches!(
            receipt.dispatch_status.as_str(),
            "routed" | "executing" | "bridge_request_pending"
        )
}

fn run_graph_state_operator_artifact_refs(
    surface: &str,
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
    state_root: Option<&std::path::Path>,
) -> serde_json::Value {
    let mut refs = run_graph_operator_artifact_refs(surface, &status.run_id)
        .as_object()
        .cloned()
        .unwrap_or_default();
    refs.insert("task_id".to_string(), serde_json::json!(status.task_id));
    refs.insert(
        "status_lifecycle_stage".to_string(),
        serde_json::json!(status.lifecycle_stage),
    );
    insert_string_artifact_ref(
        &mut refs,
        "repair_target",
        run_graph_repair_target(status, projection_truth).as_deref(),
    );
    if let Some(receipt) = projection_truth.dispatch_receipt.as_ref() {
        let inflight_downstream_projection = receipt_has_inflight_downstream_projection(receipt);
        insert_string_artifact_ref(
            &mut refs,
            "dispatch_target",
            Some(receipt.dispatch_target.as_str()),
        );
        insert_string_artifact_ref(
            &mut refs,
            "dispatch_status",
            Some(receipt.dispatch_status.as_str()),
        );
        insert_string_artifact_ref(&mut refs, "lane_status", Some(receipt.lane_status.as_str()));
        insert_string_artifact_ref(
            &mut refs,
            "dispatch_packet_path",
            receipt.dispatch_packet_path.as_deref(),
        );
        insert_string_artifact_ref(
            &mut refs,
            "dispatch_result_path",
            receipt.dispatch_result_path.as_deref(),
        );
        insert_string_artifact_ref(&mut refs, "blocker_code", receipt.blocker_code.as_deref());
        if !inflight_downstream_projection {
            insert_string_artifact_ref(
                &mut refs,
                "downstream_dispatch_target",
                receipt.downstream_dispatch_target.as_deref(),
            );
            insert_string_artifact_ref(
                &mut refs,
                "downstream_dispatch_status",
                receipt.downstream_dispatch_status.as_deref(),
            );
            insert_string_artifact_ref(
                &mut refs,
                "downstream_dispatch_result_path",
                receipt.downstream_dispatch_result_path.as_deref(),
            );
            refs.insert(
                "downstream_dispatch_ready".to_string(),
                serde_json::json!(receipt.downstream_dispatch_ready),
            );
            if !receipt.downstream_dispatch_blockers.is_empty() {
                refs.insert(
                    "downstream_dispatch_blockers".to_string(),
                    serde_json::json!(receipt.downstream_dispatch_blockers),
                );
            }
        }
        if let Some(root) = state_root {
            let result = receipt
                .downstream_dispatch_result_path
                .as_deref()
                .and_then(|path| safe_read_dispatch_result_json(root, path))
                .or_else(|| {
                    receipt
                        .dispatch_result_path
                        .as_deref()
                        .and_then(|path| safe_read_dispatch_result_json(root, path))
                });
            if let Some(result) = result.as_ref() {
                host_bridge_artifact_ref_from_result(
                    &mut refs,
                    "host_bridge_request_path",
                    result,
                    "request_path",
                    Some(root),
                    "expected_host_bridge_request_path",
                );
                host_bridge_artifact_ref_from_result(
                    &mut refs,
                    "host_bridge_result_path",
                    result,
                    "result_path",
                    Some(root),
                    "expected_host_bridge_result_path",
                );
                host_bridge_artifact_ref_from_result(
                    &mut refs,
                    "host_bridge_receipt_path",
                    result,
                    "receipt_path",
                    Some(root),
                    "expected_host_bridge_receipt_path",
                );
                insert_string_artifact_ref(
                    &mut refs,
                    "result_rework_target",
                    result
                        .get("rework_target")
                        .and_then(serde_json::Value::as_str),
                );
                insert_string_artifact_ref(
                    &mut refs,
                    "result_allowed_next_node",
                    result
                        .get("allowed_next_node")
                        .and_then(serde_json::Value::as_str),
                );
            }
        }
    }
    serde_json::Value::Object(refs)
}

fn operator_next_actions_for_operator_surface(
    blocker_codes: &[String],
    next_action: Option<&RecoveryNextAction>,
    why_not_now: Option<&RecoveryWhyNotNow>,
    recommended_command: Option<&str>,
) -> Vec<String> {
    if blocker_codes.is_empty() {
        return Vec::new();
    }
    if let Some(reason) = next_action
        .map(|value| value.reason.trim())
        .filter(|value| !value.is_empty())
    {
        return vec![reason.to_string()];
    }
    if let Some(command) = recommended_command
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return vec![format!("run `{command}`")];
    }
    if let Some(summary) = why_not_now
        .map(|value| value.summary.trim())
        .filter(|value| !value.is_empty())
    {
        return vec![summary.to_string()];
    }
    vec!["inspect authoritative run-graph state".to_string()]
}

fn halted_state_signal(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "blocked" || normalized == "lane_blocked" || normalized.ends_with("_blocked")
}

fn done_state_signal(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "completed" || normalized == "complete" || normalized.ends_with("_complete")
}

pub(crate) fn terminal_run_graph_state_resolved(status: &RunGraphStatus) -> bool {
    crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(status)
}

pub(crate) fn missing_task_run_graph_requires_stale_cleanup(
    status: Option<&RunGraphStatus>,
    task_missing: bool,
) -> bool {
    task_missing
        && status
            .map(|status| !terminal_run_graph_state_resolved(status))
            .unwrap_or(false)
}

fn stale_guard_status(
    status: &RunGraphStatus,
) -> taskflow_authority::stale_guard::StaleRunGraphStatus<'_> {
    taskflow_authority::stale_guard::StaleRunGraphStatus {
        status: status.status.as_str(),
        lifecycle_stage: status.lifecycle_stage.as_str(),
        next_node: status.next_node.as_deref(),
        resume_target: status.resume_target.as_str(),
    }
}

fn stale_guard_receipt(
    receipt: &RunGraphDispatchReceipt,
) -> taskflow_authority::stale_guard::StaleRunGraphReceipt<'_> {
    taskflow_authority::stale_guard::StaleRunGraphReceipt {
        dispatch_status: receipt.dispatch_status.as_str(),
        lane_status: receipt.lane_status.as_str(),
        downstream_dispatch_status: receipt.downstream_dispatch_status.as_deref(),
    }
}

fn recovery_lane_retire_admissibility(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
    task_state: taskflow_authority::stale_guard::StaleRunTaskState,
) -> taskflow_authority::stale_guard::StaleRunRetireAdmissibility {
    let status = stale_guard_status(status);
    let receipt = receipt.map(stale_guard_receipt);
    taskflow_authority::stale_guard::stale_run_retire_admissibility(
        &status,
        receipt.as_ref(),
        task_state,
    )
}

fn closed_task_active_run_projection_mismatch_command() -> String {
    "vida task reconcile-closed-runs --limit 25".to_string()
}

fn closed_task_run_graph_requires_stale_cleanup(
    status: Option<&RunGraphStatus>,
    task_closed_stale_run: bool,
) -> bool {
    task_closed_stale_run
        && status
            .map(|status| !terminal_run_graph_state_resolved(status))
            .unwrap_or(false)
}

fn terminal_recovery_summary_resolved(
    summary: &crate::state_store::RunGraphRecoverySummary,
) -> bool {
    done_state_signal(&summary.resume_status)
        && done_state_signal(&summary.lifecycle_stage)
        && !summary.recovery_ready
        && summary.resume_target == "none"
        && !summary.delegation_gate.delegated_cycle_open
        && summary.delegation_gate.blocker_code.is_none()
}

fn fallback_dispatch_issue_code() -> String {
    crate::contract_profile_adapter::blocker_code_str(
        crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
    )
    .to_string()
}

fn normalize_run_graph_issue_codes(
    blocker_codes: &[String],
    blocked_evidence_present: bool,
) -> Vec<String> {
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "stale_missing_task_run_graph")
    {
        return vec!["stale_missing_task_run_graph".to_string()];
    }
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "closed_task_active_run_projection_mismatch")
    {
        return vec!["closed_task_active_run_projection_mismatch".to_string()];
    }
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "missing_owned_write_scope")
    {
        return vec!["missing_owned_write_scope".to_string()];
    }
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "host_tool_bridge_adapter_required")
    {
        return vec!["host_tool_bridge_adapter_required".to_string()];
    }
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "internal_activation_view_only")
    {
        return vec!["internal_activation_view_only".to_string()];
    }
    let normalized = crate::release1_operator_output::normalize_blocker_codes(
        blocker_codes,
        crate::release_contract_adapters::canonical_blocker_codes,
        None,
    );
    if normalized.is_empty() && (blocked_evidence_present || !blocker_codes.is_empty()) {
        vec![fallback_dispatch_issue_code()]
    } else {
        normalized
    }
}

fn build_run_graph_operator_surface_payload(
    surface: &str,
    run_id: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    extra_fields: serde_json::Value,
) -> Result<serde_json::Value, String> {
    build_run_graph_operator_surface_payload_with_artifact_refs(
        surface,
        run_id,
        blocker_codes,
        next_actions,
        run_graph_operator_artifact_refs(surface, run_id),
        extra_fields,
    )
}

fn build_run_graph_operator_surface_payload_with_artifact_refs(
    surface: &str,
    run_id: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    extra_fields: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)?;
    let mut payload = serde_json::json!({
        "surface": surface,
        "run_id": run_id,
        "status": finalized.status,
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
    });
    let extra_object = extra_fields
        .as_object()
        .ok_or_else(|| "run-graph operator payload extras must be an object".to_string())?
        .clone();
    payload
        .as_object_mut()
        .expect("run-graph operator payload should serialize to an object")
        .extend(extra_object);
    if let Some(error) = shared_operator_output_contract_parity_error(&payload) {
        return Err(error.to_string());
    }
    Ok(payload)
}

fn build_recovery_json_payload_with_task_identity(
    surface: &str,
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    task_identity: Option<&RunGraphDispatchTaskIdentity>,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    let why_not_now = why_not_now.map(|mut value| {
        value.blocking_surface = Some(surface.to_string());
        value
    });
    let next_actions = operator_next_actions_for_operator_surface(
        &blocker_codes,
        next_action.as_ref(),
        why_not_now.as_ref(),
        recommended_command.as_deref(),
    );
    let dispatch_receipt = projection_truth
        .dispatch_receipt
        .as_ref()
        .map(|receipt| serde_json::to_value(receipt).expect("dispatch receipt should serialize"))
        .unwrap_or(serde_json::Value::Null);
    build_run_graph_operator_surface_payload(
        surface,
        &summary.run_id,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "dispatch_receipt": dispatch_receipt,
            "why_not_now": why_not_now,
            "next_action": next_action,
            "recommended_command": recommended_command,
            "recommended_surface": recommended_surface,
            "recovery": summary,
            "projection_truth": projection_truth,
            "task_identity": task_identity,
        }),
    )
}

fn build_recovery_json_payload(
    surface: &str,
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    build_recovery_json_payload_with_task_identity(
        surface,
        summary,
        projection_truth,
        None,
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn recovery_json_error_payload(
    surface: &str,
    run_id: &str,
    state_dir: &std::path::Path,
    error_kind: &str,
    error: &str,
) -> serde_json::Value {
    let status_command = format!(
        "vida taskflow run-graph status {} --json",
        shell_quote(run_id)
    );
    let next_actions = vec![format!(
        "Inspect run-graph state with `{}`; recovery status failed: {}",
        status_command, error
    )];
    build_run_graph_operator_surface_payload(
        surface,
        run_id,
        vec![error_kind.to_string()],
        next_actions,
        serde_json::json!({
            "error_kind": error_kind,
            "error": error,
            "state_dir": state_dir.display().to_string(),
            "recommended_surface": "vida taskflow run-graph status",
            "recommended_command": status_command,
        }),
    )
    .unwrap_or_else(|payload_error| {
        serde_json::json!({
            "surface": surface,
            "run_id": run_id,
            "status": "blocked",
            "blocker_codes": [fallback_dispatch_issue_code()],
            "next_actions": [
                format!("Inspect run-graph state for `{}`; recovery status failed: {}", run_id, error)
            ],
            "artifact_refs": run_graph_operator_artifact_refs(surface, run_id),
            "error_kind": error_kind,
            "error": error,
            "payload_error": payload_error,
            "state_dir": state_dir.display().to_string(),
        })
    })
}

fn run_graph_status_error_payload(
    state_dir: &std::path::Path,
    run_id: &str,
    error_kind: &str,
    error: &str,
) -> serde_json::Value {
    let latest_command = "vida taskflow run-graph latest".to_string();
    let next_actions = vec![
        format!("Inspect the latest run-graph state with `{latest_command}`."),
        format!(
            "Validate the run id `{}` before retrying `vida taskflow run-graph status {}`.",
            run_id,
            shell_quote(run_id)
        ),
    ];
    build_run_graph_operator_surface_payload(
        "vida taskflow run-graph status",
        run_id,
        vec![error_kind.to_string()],
        next_actions,
        serde_json::json!({
            "error_kind": error_kind,
            "error": error,
            "state_dir": state_dir.display().to_string(),
            "recommended_surface": "vida taskflow run-graph latest",
            "recommended_command": latest_command,
        }),
    )
    .unwrap_or_else(|payload_error| {
        serde_json::json!({
            "surface": "vida taskflow run-graph status",
            "run_id": run_id,
            "status": "blocked",
            "blocker_codes": [fallback_dispatch_issue_code()],
            "next_actions": [
                format!("Inspect latest run-graph state before retrying `{}`.", run_id)
            ],
            "artifact_refs": run_graph_operator_artifact_refs("vida taskflow run-graph status", run_id),
            "error_kind": error_kind,
            "error": error,
            "payload_error": payload_error,
            "state_dir": state_dir.display().to_string(),
        })
    })
}

fn emit_run_graph_status_error(
    state_dir: &std::path::Path,
    run_id: &str,
    error_kind: &str,
    error: &str,
    as_json: bool,
) -> ExitCode {
    let payload = run_graph_status_error_payload(state_dir, run_id, error_kind, error);
    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
        print_surface_line(RenderMode::Plain, "run", run_id);
        print_surface_line(RenderMode::Plain, "status", "blocked");
        print_surface_line(
            RenderMode::Plain,
            "blocker_codes",
            &payload["blocker_codes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
        print_surface_line(RenderMode::Plain, "error", error);
        if let Some(command) = payload["recommended_command"].as_str() {
            print_surface_line(RenderMode::Plain, "next", command);
        }
    }
    exit_code_for_operator_payload(&payload)
}

fn run_graph_latest_error_payload(
    state_dir: &std::path::Path,
    error_kind: &str,
    error: &str,
) -> serde_json::Value {
    let latest_command = "vida taskflow run-graph latest".to_string();
    let next_actions = vec![
        format!("Retry latest run-graph inspection with `{latest_command}` after resolving the reported state error."),
        "Inspect broader runtime readiness with `vida status` if latest run-graph state remains unavailable.".to_string(),
    ];
    build_run_graph_operator_surface_payload(
        "vida taskflow run-graph latest",
        "latest",
        vec![fallback_dispatch_issue_code()],
        next_actions,
        serde_json::json!({
            "error_kind": error_kind,
            "error": error,
            "state_dir": state_dir.display().to_string(),
            "recommended_surface": "vida taskflow run-graph latest",
            "recommended_command": latest_command,
        }),
    )
    .unwrap_or_else(|payload_error| {
        serde_json::json!({
            "surface": "vida taskflow run-graph latest",
            "run_id": "latest",
            "status": "blocked",
            "blocker_codes": [fallback_dispatch_issue_code()],
            "next_actions": [
                "Inspect broader runtime readiness with `vida status` before retrying latest run-graph state."
            ],
            "artifact_refs": run_graph_operator_artifact_refs("vida taskflow run-graph latest", "latest"),
            "error_kind": error_kind,
            "error": error,
            "payload_error": payload_error,
            "state_dir": state_dir.display().to_string(),
        })
    })
}

fn emit_run_graph_latest_error(
    state_dir: &std::path::Path,
    error_kind: &str,
    error: &str,
    as_json: bool,
) -> ExitCode {
    let payload = run_graph_latest_error_payload(state_dir, error_kind, error);
    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
        print_surface_line(RenderMode::Plain, "status", "blocked");
        print_surface_line(
            RenderMode::Plain,
            "blocker_codes",
            &payload["blocker_codes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
        print_surface_line(RenderMode::Plain, "error", error);
        if let Some(command) = payload["recommended_command"].as_str() {
            print_surface_line(RenderMode::Plain, "next", command);
        }
    }
    exit_code_for_operator_payload(&payload)
}

fn emit_recovery_json_error(
    state_dir: &std::path::Path,
    run_id: &str,
    error_kind: &str,
    error: &str,
) -> ExitCode {
    let payload = recovery_json_error_payload(
        "vida taskflow recovery status",
        run_id,
        state_dir,
        error_kind,
        error,
    );
    crate::print_json_pretty(&payload);
    exit_code_for_operator_payload(&payload)
}

fn exit_code_for_operator_payload(payload: &serde_json::Value) -> ExitCode {
    if payload["status"].as_str() == Some("blocked") {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn build_recovery_explain_json_payload_with_task_identity(
    surface: &str,
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    task_identity: Option<&RunGraphDispatchTaskIdentity>,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    let why_not_now = why_not_now.map(|mut value| {
        value.blocking_surface = Some(surface.to_string());
        value
    });
    let diagnosis_summary = next_action
        .as_ref()
        .map(|value| value.reason.clone())
        .or_else(|| why_not_now.as_ref().map(|value| value.summary.clone()))
        .or_else(|| {
            recommended_command
                .as_ref()
                .map(|command| format!("Run `{command}`."))
        })
        .unwrap_or_else(|| "No recovery blocker is currently actionable.".to_string());

    // Classify into one of four primary diagnosis types
    let diagnosis_type = categorize_recovery_diagnosis(&blocker_codes, summary, projection_truth);

    let diagnosis_evidence = serde_json::json!({
        "recovery_ready": summary.recovery_ready,
        "resume_target": summary.resume_target,
        "delegated_cycle_open": summary.delegation_gate.delegated_cycle_open,
        "projection_reason": projection_truth.projection_reason,
        "projection_vs_receipt_parity": projection_truth.projection_vs_receipt_parity,
        "stale_state_suspected": projection_truth.stale_state_suspected,
    });
    let next_actions = operator_next_actions_for_operator_surface(
        &blocker_codes,
        next_action.as_ref(),
        why_not_now.as_ref(),
        recommended_command.as_deref(),
    );
    build_run_graph_operator_surface_payload(
        surface,
        &summary.run_id,
        blocker_codes.clone(),
        next_actions,
        serde_json::json!({
            "diagnosis": diagnosis_type,
            "diagnosis_detail": {
                "summary": diagnosis_summary,
                "blocker_codes": blocker_codes,
                "evidence": diagnosis_evidence,
            },
            "next_action": next_action,
            "recommended_command": recommended_command,
            "recommended_surface": recommended_surface,
            "recovery": summary,
            "projection_truth": projection_truth,
            "task_identity": task_identity,
        }),
    )
}

fn build_recovery_explain_json_payload(
    surface: &str,
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    build_recovery_explain_json_payload_with_task_identity(
        surface,
        summary,
        projection_truth,
        None,
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn build_run_graph_diagnosis_json_payload_for_surface(
    surface: &str,
    diagnosis: &RunGraphDiagnosis,
) -> Result<serde_json::Value, String> {
    build_run_graph_diagnosis_json_payload_for_surface_with_state_root(surface, diagnosis, None)
}

fn build_run_graph_diagnosis_json_payload_for_surface_with_state_root(
    surface: &str,
    diagnosis: &RunGraphDiagnosis,
    state_root: Option<&std::path::Path>,
) -> Result<serde_json::Value, String> {
    let next_actions = operator_next_actions_for_operator_surface(
        &diagnosis.blocker_codes,
        diagnosis.next_action.as_ref(),
        diagnosis.why_not_now.as_ref(),
        diagnosis.recommended_command.as_deref(),
    );
    let active_repair_summary = active_run_repair_summary(diagnosis, state_root);
    build_run_graph_operator_surface_payload(
        surface,
        &diagnosis.run_id,
        diagnosis.blocker_codes.clone(),
        next_actions,
        serde_json::json!({
            "active_repair_summary": active_repair_summary,
            "why_not_now": diagnosis.why_not_now,
            "next_action": diagnosis.next_action,
            "recommended_command": diagnosis.recommended_command,
            "recommended_surface": diagnosis.recommended_surface,
            "recovery": diagnosis.recovery,
            "projection_truth": diagnosis.projection_truth,
        }),
    )
}

fn build_recovery_latest_json_payload_with_task_identity(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    task_identity: Option<&RunGraphDispatchTaskIdentity>,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    build_recovery_json_payload_with_task_identity(
        "vida taskflow recovery latest",
        summary,
        projection_truth,
        task_identity,
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn build_recovery_latest_json_payload(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    blocker_codes: Vec<String>,
    why_not_now: Option<RecoveryWhyNotNow>,
    next_action: Option<RecoveryNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> Result<serde_json::Value, String> {
    build_recovery_latest_json_payload_with_task_identity(
        summary,
        projection_truth,
        None,
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

async fn render_latest_recovery_json_payload(surface: &'static str) -> ExitCode {
    let state_dir = proxy_state_dir();
    match StateStore::open_existing_read_only(state_dir).await {
        Ok(store) => match latest_recovery_summary_for_operator_surface(&store).await {
            Ok(summary) => {
                let summary = match summary {
                    Some(summary) => {
                        let status = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => status,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for release-admission stale recovery check: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        match store
                            .run_graph_status_is_stale_after_release_admission_complete(&status)
                            .await
                        {
                            Ok(true) => None,
                            Ok(false) => Some(summary),
                            Err(error) => {
                                eprintln!(
                                    "Failed to classify release-admitted stale recovery: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                    None => None,
                };
                let projection_truth = match summary.as_ref() {
                    Some(summary) => match store.run_graph_status(&summary.run_id).await {
                        Ok(status) => match run_graph_projection_truth(&store, &status).await {
                            Ok(truth) => Some(truth),
                            Err(error) => {
                                eprintln!("Failed to build recovery projection truth: {error}");
                                return ExitCode::from(1);
                            }
                        },
                        Err(error) => {
                            eprintln!(
                                "Failed to read run-graph status for projection truth: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    },
                    None => None,
                };
                let owned_write_scope_hint = match summary.as_ref() {
                    Some(summary) => recovery_owned_write_scope_for_summary(&store, summary).await,
                    None => Vec::new(),
                };
                let task_identity = match summary.as_ref() {
                    Some(summary) => match store
                        .run_graph_dispatch_task_identity(&summary.run_id)
                        .await
                    {
                        Ok(identity) => identity,
                        Err(error) => {
                            eprintln!("Failed to read run-graph task identity: {error}");
                            return ExitCode::from(1);
                        }
                    },
                    None => None,
                };
                let contract = summary.as_ref().zip(projection_truth.as_ref()).map(
                    |(summary, projection_truth)| {
                        recovery_surface_contract_with_owned_scope(
                            summary,
                            projection_truth,
                            &owned_write_scope_hint,
                        )
                    },
                );
                let payload = match (summary.as_ref(), projection_truth.as_ref(), contract) {
                    (Some(summary), Some(projection_truth), Some(contract)) => {
                        build_recovery_json_payload_with_task_identity(
                            surface,
                            summary,
                            projection_truth,
                            task_identity.as_ref(),
                            contract.0,
                            contract.1,
                            contract.2,
                            contract.3,
                            contract.4,
                        )
                    }
                    _ => Ok(serde_json::json!({
                        "surface": surface,
                        "status": null,
                    })),
                };
                match payload {
                    Ok(payload) => {
                        let exit_code = exit_code_for_operator_payload(&payload);
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&payload)
                                .expect("latest recovery summary should render as json")
                        );
                        exit_code
                    }
                    Err(error) => {
                        eprintln!("Failed to render normalized recovery latest payload: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            Err(error) => {
                eprintln!("Failed to read latest recovery status: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn latest_recovery_summary_for_operator_surface(
    store: &StateStore,
) -> Result<Option<crate::state_store::RunGraphRecoverySummary>, crate::state_store::StateStoreError>
{
    let current_session_scope_is_present = store.current_session_identity_is_present()?;
    let scoped = store
        .latest_run_graph_recovery_summary_for_current_session()
        .await?;
    if scoped.is_some() || current_session_scope_is_present {
        return Ok(scoped);
    }
    store.latest_run_graph_recovery_summary().await
}

async fn latest_run_graph_status_for_operator_surface(
    store: &StateStore,
) -> Result<Option<RunGraphStatus>, crate::state_store::StateStoreError> {
    if store.current_session_identity_is_present()? {
        store.latest_run_graph_status_for_current_session().await
    } else {
        store.latest_run_graph_status().await
    }
}

fn build_run_graph_diagnosis_json_payload(
    diagnosis: &RunGraphDiagnosis,
) -> Result<serde_json::Value, String> {
    build_run_graph_diagnosis_json_payload_for_surface(
        "vida taskflow run-graph diagnose-latest",
        diagnosis,
    )
}

fn build_run_graph_state_json_payload_with_task_identity(
    surface: &str,
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
    task_identity: Option<&RunGraphDispatchTaskIdentity>,
) -> Result<serde_json::Value, String> {
    build_run_graph_state_json_payload_with_task_identity_and_state_root(
        surface,
        status,
        projection_truth,
        task_identity,
        None,
    )
}

fn run_graph_state_recommended_command_is_self_loop(
    status: &RunGraphStatus,
    command: Option<&str>,
) -> bool {
    let Some(command) = command.map(default_operator_command_text) else {
        return false;
    };
    command == format!("vida taskflow run-graph status {}", status.run_id)
}

fn run_graph_state_actionable_next_action(
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Option<RecoveryNextAction> {
    let receipt = projection_truth.dispatch_receipt.as_ref()?;
    let blocked_receipt_evidence = halted_state_signal(&receipt.dispatch_status)
        || halted_state_signal(&receipt.lane_status)
        || receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(halted_state_signal)
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|value| !value.trim().is_empty());
    if !blocked_receipt_evidence {
        return None;
    }
    let command = if run_graph_state_recommended_command_is_self_loop(
        status,
        projection_truth.next_lawful_operator_action.as_deref(),
    ) {
        format!("vida lane show {}", shell_quote(&status.run_id))
    } else {
        projection_truth
            .next_lawful_operator_action
            .as_deref()
            .map(default_operator_command_text)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("vida lane show {}", shell_quote(&status.run_id)))
    };
    Some(RecoveryNextAction {
        surface: recommended_surface_for_command(&command),
        command: command.clone(),
        reason: format!(
            "inspect dispatch receipt and host-bridge artifacts with `{command}`; artifact_refs identify result paths, downstream dispatch status, and repair target."
        ),
    })
}

fn run_graph_state_operator_next_actions(
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
    blocker_codes: &[String],
) -> Vec<String> {
    let next_action = run_graph_state_actionable_next_action(status, projection_truth);
    let recommended_command = if next_action.is_some()
        && run_graph_state_recommended_command_is_self_loop(
            status,
            projection_truth.next_lawful_operator_action.as_deref(),
        ) {
        None
    } else {
        projection_truth.next_lawful_operator_action.as_deref()
    };
    operator_next_actions_for_operator_surface(
        blocker_codes,
        next_action.as_ref(),
        None,
        recommended_command,
    )
}

fn build_run_graph_state_json_payload_with_task_identity_and_state_root(
    surface: &str,
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
    task_identity: Option<&RunGraphDispatchTaskIdentity>,
    state_root: Option<&std::path::Path>,
) -> Result<serde_json::Value, String> {
    let blocker_codes = run_graph_state_surface_issue_codes(status, projection_truth);
    let next_actions =
        run_graph_state_operator_next_actions(status, projection_truth, &blocker_codes);
    build_run_graph_operator_surface_payload_with_artifact_refs(
        surface,
        &status.run_id,
        blocker_codes,
        next_actions,
        run_graph_state_operator_artifact_refs(surface, status, projection_truth, state_root),
        serde_json::json!({
            "run_id": status.run_id,
            "run_graph_status": status,
            "delegation_gate": status.delegation_gate(),
            "projection_truth": public_run_graph_projection_truth(projection_truth),
            "task_identity": task_identity,
        }),
    )
}

fn public_run_graph_projection_truth(
    projection_truth: &RunGraphProjectionTruth,
) -> serde_json::Value {
    let mut value = serde_json::to_value(projection_truth).unwrap_or(serde_json::Value::Null);
    let Some(receipt) = projection_truth.dispatch_receipt.as_ref() else {
        return value;
    };
    if !receipt_has_inflight_downstream_projection(receipt) {
        return value;
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "stale_downstream_projection_suppressed".to_string(),
            serde_json::json!(true),
        );
        if let Some(receipt_object) = object
            .get_mut("dispatch_receipt")
            .and_then(serde_json::Value::as_object_mut)
        {
            receipt_object.remove("downstream_dispatch_target");
            receipt_object.remove("downstream_dispatch_command");
            receipt_object.remove("downstream_dispatch_note");
            receipt_object.remove("downstream_dispatch_ready");
            receipt_object.remove("downstream_dispatch_blockers");
            receipt_object.remove("downstream_dispatch_packet_path");
            receipt_object.remove("downstream_dispatch_status");
            receipt_object.remove("downstream_dispatch_result_path");
            receipt_object.remove("downstream_dispatch_trace_path");
        }
    }
    value
}

fn build_run_graph_state_json_payload(
    surface: &str,
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Result<serde_json::Value, String> {
    build_run_graph_state_json_payload_with_task_identity(surface, status, projection_truth, None)
}

fn run_graph_task_identity_compact(identity: &RunGraphDispatchTaskIdentity) -> String {
    format!(
        "run={} feature={} spec={} work_pool={} source={}",
        identity.run_id,
        identity.feature_epic_id.as_deref().unwrap_or("none"),
        identity.spec_task_id.as_deref().unwrap_or("none"),
        identity.work_pool_task_id.as_deref().unwrap_or("none"),
        identity.source
    )
}

fn run_graph_task_identity_payload(
    surface: &str,
    run_id: &str,
    identity: Option<&RunGraphDispatchTaskIdentity>,
) -> Result<serde_json::Value, String> {
    let blocker_codes = if identity.is_some() {
        Vec::new()
    } else {
        vec!["missing_run_graph_task_identity".to_string()]
    };
    let next_actions = if identity.is_some() {
        Vec::new()
    } else {
        vec![format!(
            "Run `vida taskflow run-graph task-identity repair {run_id} --from-task <feature-or-spec-task>`."
        )]
    };
    build_run_graph_operator_surface_payload(
        surface,
        run_id,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "task_identity": identity,
        }),
    )
}

fn print_run_graph_task_identity(
    surface: &str,
    run_id: &str,
    identity: Option<&RunGraphDispatchTaskIdentity>,
) {
    print_surface_header(RenderMode::Plain, surface);
    print_surface_line(RenderMode::Plain, "run", run_id);
    match identity {
        Some(identity) => {
            print_surface_line(
                RenderMode::Plain,
                "task_identity",
                &run_graph_task_identity_compact(identity),
            );
        }
        None => {
            print_surface_line(RenderMode::Plain, "status", "missing_task_identity");
            print_surface_line(
                RenderMode::Plain,
                "next_action",
                &format!(
                    "vida taskflow run-graph task-identity repair {run_id} --from-task <feature-or-spec-task>"
                ),
            );
        }
    }
}

#[derive(Debug, Default)]
struct RunGraphTaskIdentityCommand {
    action: String,
    run_id: Option<String>,
    feature_epic_id: Option<String>,
    spec_task_id: Option<String>,
    work_pool_task_id: Option<String>,
    from_task_id: Option<String>,
    state_dir: Option<PathBuf>,
    as_json: bool,
    help: bool,
}

fn parse_run_graph_task_identity_command(
    args: &[String],
) -> Result<RunGraphTaskIdentityCommand, String> {
    let mut command = RunGraphTaskIdentityCommand {
        action: "show".to_string(),
        ..RunGraphTaskIdentityCommand::default()
    };
    let mut index = 0;
    if args
        .first()
        .is_some_and(|value| value == "--help" || value == "-h")
    {
        command.help = true;
        return Ok(command);
    }
    if let Some(action) = args
        .first()
        .filter(|value| matches!(value.as_str(), "show" | "seed" | "repair"))
    {
        command.action = action.clone();
        index = 1;
    }
    if let Some(run_id) = args.get(index).filter(|value| !value.starts_with("--")) {
        command.run_id = Some(run_id.clone());
        index += 1;
    }
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                command.as_json = true;
                index += 1;
            }
            "--help" | "-h" => {
                command.help = true;
                index += 1;
            }
            "--state-dir" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--state-dir requires a path".to_string())?;
                command.state_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--feature-epic" | "--feature-task" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("{} requires a task id", args[index]))?;
                command.feature_epic_id = Some(value.clone());
                index += 2;
            }
            "--spec-task" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--spec-task requires a task id".to_string())?;
                command.spec_task_id = Some(value.clone());
                index += 2;
            }
            "--work-pool-task" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--work-pool-task requires a task id".to_string())?;
                command.work_pool_task_id = Some(value.clone());
                index += 2;
            }
            "--from-task" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--from-task requires a task id".to_string())?;
                command.from_task_id = Some(value.clone());
                index += 2;
            }
            value => {
                return Err(format!("unsupported task-identity option `{value}`"));
            }
        }
    }
    Ok(command)
}

fn print_run_graph_task_identity_help() {
    println!("Usage:");
    println!("  vida taskflow run-graph task-identity <run-id> [--state-dir <path>] [--json]");
    println!("  vida taskflow run-graph task-identity show <run-id> [--state-dir <path>] [--json]");
    println!(
        "  vida taskflow run-graph task-identity repair <run-id> --from-task <feature-or-spec-task> [--state-dir <path>] [--json]"
    );
    println!(
        "  vida taskflow run-graph task-identity seed <run-id> --feature-epic <id> --spec-task <id> [--work-pool-task <id>] [--state-dir <path>] [--json]"
    );
}

fn task_has_label(task: &crate::state_store::TaskRecord, label: &str) -> bool {
    task.labels.iter().any(|value| value.trim() == label)
}

fn task_is_spec_first_feature_parent(task: &crate::state_store::TaskRecord) -> bool {
    task_has_label(task, "feature-request") && task_has_label(task, "spec-first")
}

fn task_is_spec_pack_child(task: &crate::state_store::TaskRecord) -> bool {
    task_has_label(task, "spec-pack")
}

fn task_is_work_pool_pack_child(task: &crate::state_store::TaskRecord) -> bool {
    task_has_label(task, "work-pool-pack")
}

fn task_is_dev_pack_child(task: &crate::state_store::TaskRecord) -> bool {
    task_has_label(task, "dev-pack")
}

fn parent_id_for_task(task: &crate::state_store::TaskRecord) -> Option<String> {
    StateStore::parent_id_for_task(task)
}

fn derive_run_graph_task_identity_from_feature(
    tasks: &[crate::state_store::TaskRecord],
    run_id: &str,
    feature_epic_id: &str,
    source: &str,
) -> Result<RunGraphDispatchTaskIdentity, String> {
    let feature = tasks
        .iter()
        .find(|task| task.id == feature_epic_id)
        .ok_or_else(|| format!("feature task `{feature_epic_id}` is missing"))?;
    if !task_is_spec_first_feature_parent(feature) {
        return Err(format!(
            "feature task `{feature_epic_id}` is not a spec-first feature parent"
        ));
    }
    let children = tasks
        .iter()
        .filter(|task| parent_id_for_task(task).as_deref() == Some(feature_epic_id))
        .collect::<Vec<_>>();
    let spec_task_id = children
        .iter()
        .filter(|task| task_is_spec_pack_child(task))
        .map(|task| task.id.clone())
        .min()
        .ok_or_else(|| format!("feature task `{feature_epic_id}` has no spec-pack child"))?;
    let work_pool_task_id = children
        .iter()
        .filter(|task| task_is_work_pool_pack_child(task))
        .map(|task| task.id.clone())
        .min();
    let dev_task_id = children
        .iter()
        .filter(|task| task_is_dev_pack_child(task))
        .map(|task| task.id.clone())
        .min();
    Ok(RunGraphDispatchTaskIdentity {
        run_id: run_id.to_string(),
        feature_epic_id: Some(feature_epic_id.to_string()),
        spec_task_id: Some(spec_task_id),
        work_pool_task_id,
        dev_task_id,
        source: source.to_string(),
        updated_at: time::OffsetDateTime::now_utc()
            .unix_timestamp_nanos()
            .to_string(),
    })
}

fn derive_run_graph_task_identity_from_task(
    tasks: &[crate::state_store::TaskRecord],
    run_id: &str,
    from_task_id: &str,
) -> Result<RunGraphDispatchTaskIdentity, String> {
    let task = tasks
        .iter()
        .find(|task| task.id == from_task_id)
        .ok_or_else(|| format!("source task `{from_task_id}` is missing"))?;
    let feature_id = if task_is_spec_first_feature_parent(task) {
        task.id.clone()
    } else if task_is_spec_pack_child(task) || task_is_work_pool_pack_child(task) {
        parent_id_for_task(task).ok_or_else(|| {
            format!("source task `{from_task_id}` has no parent feature relationship")
        })?
    } else {
        return Err(format!(
            "source task `{from_task_id}` is not a spec-first feature, spec-pack, or work-pool-pack task"
        ));
    };
    derive_run_graph_task_identity_from_feature(
        tasks,
        run_id,
        &feature_id,
        "operator_task_identity_repair",
    )
}

fn dispatch_init_task_identity_from_task(
    run_id: &str,
    task: &crate::state_store::TaskRecord,
) -> RunGraphDispatchTaskIdentity {
    RunGraphDispatchTaskIdentity {
        run_id: run_id.to_string(),
        feature_epic_id: parent_id_for_task(task),
        spec_task_id: None,
        work_pool_task_id: None,
        dev_task_id: Some(task.id.clone()),
        source: "dispatch_init_existing_task".to_string(),
        updated_at: time::OffsetDateTime::now_utc()
            .unix_timestamp_nanos()
            .to_string(),
    }
}

async fn preview_dispatch_init_task_identity(
    store: &StateStore,
    run_id: &str,
) -> Result<Option<RunGraphDispatchTaskIdentity>, String> {
    if let Some(identity) = store
        .run_graph_dispatch_task_identity(run_id)
        .await
        .map_err(|error| format!("Failed to read dispatch-init task identity: {error}"))?
    {
        return Ok(Some(identity));
    }
    let task = match store.show_task(run_id).await {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    Ok(Some(dispatch_init_task_identity_from_task(run_id, &task)))
}

async fn ensure_dispatch_init_task_identity(
    store: &StateStore,
    run_id: &str,
) -> Result<Option<RunGraphDispatchTaskIdentity>, String> {
    let Some(identity) = preview_dispatch_init_task_identity(store, run_id).await? else {
        return Ok(None);
    };
    store
        .record_run_graph_dispatch_task_identity(&identity)
        .await
        .map_err(|error| format!("Failed to record dispatch-init task identity: {error}"))?;
    Ok(Some(identity))
}

async fn try_backfill_dispatch_init_task_identity(state_dir: &std::path::Path, run_id: &str) {
    if let Ok(store) = StateStore::open_existing_with_timeout(
        state_dir.to_path_buf(),
        DISPATCH_INIT_IDENTITY_BACKFILL_OPEN_TIMEOUT,
    )
    .await
    {
        let _ = ensure_dispatch_init_task_identity(&store, run_id).await;
        store.close().await;
    }
}

fn seed_run_graph_task_identity_from_options(
    tasks: &[crate::state_store::TaskRecord],
    run_id: &str,
    feature_epic_id: &str,
    spec_task_id: &str,
    work_pool_task_id: Option<&str>,
) -> Result<RunGraphDispatchTaskIdentity, String> {
    let feature = tasks
        .iter()
        .find(|task| task.id == feature_epic_id)
        .ok_or_else(|| format!("feature task `{feature_epic_id}` is missing"))?;
    if !task_is_spec_first_feature_parent(feature) {
        return Err(format!(
            "feature task `{feature_epic_id}` is not a spec-first feature parent"
        ));
    }
    let spec = tasks
        .iter()
        .find(|task| task.id == spec_task_id)
        .ok_or_else(|| format!("spec task `{spec_task_id}` is missing"))?;
    if !task_is_spec_pack_child(spec)
        || parent_id_for_task(spec).as_deref() != Some(feature_epic_id)
    {
        return Err(format!(
            "spec task `{spec_task_id}` is not a spec-pack child of `{feature_epic_id}`"
        ));
    }
    if let Some(work_pool_task_id) = work_pool_task_id {
        let work_pool = tasks
            .iter()
            .find(|task| task.id == work_pool_task_id)
            .ok_or_else(|| format!("work-pool task `{work_pool_task_id}` is missing"))?;
        if !task_is_work_pool_pack_child(work_pool)
            || parent_id_for_task(work_pool).as_deref() != Some(feature_epic_id)
        {
            return Err(format!(
                "work-pool task `{work_pool_task_id}` is not a work-pool-pack child of `{feature_epic_id}`"
            ));
        }
    }
    Ok(RunGraphDispatchTaskIdentity {
        run_id: run_id.to_string(),
        feature_epic_id: Some(feature_epic_id.to_string()),
        spec_task_id: Some(spec_task_id.to_string()),
        work_pool_task_id: work_pool_task_id.map(str::to_string),
        dev_task_id: None,
        source: "operator_task_identity_seed".to_string(),
        updated_at: time::OffsetDateTime::now_utc()
            .unix_timestamp_nanos()
            .to_string(),
    })
}

async fn run_taskflow_run_graph_task_identity(args: &[String]) -> ExitCode {
    let command = match parse_run_graph_task_identity_command(args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            print_run_graph_task_identity_help();
            return ExitCode::from(2);
        }
    };
    if command.help {
        print_run_graph_task_identity_help();
        return ExitCode::SUCCESS;
    }
    let Some(run_id) = command.run_id.as_deref() else {
        eprintln!("Missing required <run-id>.");
        print_run_graph_task_identity_help();
        return ExitCode::from(2);
    };
    let state_dir = command.state_dir.unwrap_or_else(proxy_state_dir);
    match command.action.as_str() {
        "show" => match StateStore::open_existing_read_only(state_dir).await {
            Ok(store) => match store.run_graph_dispatch_task_identity(run_id).await {
                Ok(identity) => {
                    if command.as_json {
                        match run_graph_task_identity_payload(
                            "vida taskflow run-graph task-identity",
                            run_id,
                            identity.as_ref(),
                        ) {
                            Ok(payload) => crate::print_json_pretty(&payload),
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized run-graph task identity payload: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        print_run_graph_task_identity(
                            "vida taskflow run-graph task-identity",
                            run_id,
                            identity.as_ref(),
                        );
                    }
                    if identity.is_some() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to read run-graph task identity: {error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("Failed to open authoritative state store: {error}");
                ExitCode::from(1)
            }
        },
        "repair" | "seed" => match StateStore::open_existing(state_dir).await {
            Ok(store) => {
                let tasks = match store.list_tasks(None, true).await {
                    Ok(tasks) => tasks,
                    Err(error) => {
                        eprintln!("Failed to read tasks for task identity repair: {error}");
                        return ExitCode::from(1);
                    }
                };
                let identity = if command.action == "repair" {
                    let Some(from_task_id) = command.from_task_id.as_deref() else {
                        eprintln!("repair requires --from-task <task-id>.");
                        return ExitCode::from(2);
                    };
                    derive_run_graph_task_identity_from_task(&tasks, run_id, from_task_id)
                } else {
                    let Some(feature_epic_id) = command.feature_epic_id.as_deref() else {
                        eprintln!("seed requires --feature-epic <task-id>.");
                        return ExitCode::from(2);
                    };
                    let Some(spec_task_id) = command.spec_task_id.as_deref() else {
                        eprintln!("seed requires --spec-task <task-id>.");
                        return ExitCode::from(2);
                    };
                    seed_run_graph_task_identity_from_options(
                        &tasks,
                        run_id,
                        feature_epic_id,
                        spec_task_id,
                        command.work_pool_task_id.as_deref(),
                    )
                };
                let identity = match identity {
                    Ok(identity) => identity,
                    Err(error) => {
                        eprintln!("Failed to build run-graph task identity: {error}");
                        return ExitCode::from(1);
                    }
                };
                if let Err(error) = store
                    .record_run_graph_dispatch_task_identity(&identity)
                    .await
                {
                    eprintln!("Failed to persist run-graph task identity: {error}");
                    return ExitCode::from(1);
                }
                if command.as_json {
                    match run_graph_task_identity_payload(
                        "vida taskflow run-graph task-identity",
                        run_id,
                        Some(&identity),
                    ) {
                        Ok(payload) => crate::print_json_pretty(&payload),
                        Err(error) => {
                            eprintln!(
                                "Failed to render normalized run-graph task identity payload: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    print_run_graph_task_identity(
                        "vida taskflow run-graph task-identity",
                        run_id,
                        Some(&identity),
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Failed to open authoritative state store: {error}");
                ExitCode::from(1)
            }
        },
        _ => {
            print_run_graph_task_identity_help();
            ExitCode::from(2)
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowRunGraphSeedPayload {
    pub(crate) request_text: String,
    pub(crate) role_selection: RuntimeConsumptionLaneSelection,
    pub(crate) status: RunGraphStatus,
}

#[derive(Debug)]
enum RunGraphDispatchInitPreview {
    Existing(RunGraphDispatchInitArtifacts),
    Prepared(PreparedRunGraphDispatchInit),
}

#[derive(Debug)]
struct PreparedRunGraphDispatchInit {
    requested_run_id: String,
    run_id: String,
    status: RunGraphStatus,
    role_selection: RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: serde_json::Value,
    taskflow_handoff_plan: serde_json::Value,
    dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: String,
    seed_payload: Option<TaskflowRunGraphSeedPayload>,
    task_identity: Option<RunGraphDispatchTaskIdentity>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowRunGraphAdvancePayload {
    pub(crate) status: RunGraphStatus,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RunGraphProjectionTruth {
    pub(crate) projection_source: String,
    pub(crate) projection_reason: String,
    pub(crate) dispatch_receipt_present: bool,
    pub(crate) continuation_binding_present: bool,
    pub(crate) projection_vs_receipt_parity: String,
    pub(crate) stale_state_suspected: bool,
    pub(crate) next_lawful_operator_action: Option<String>,
    pub(crate) dispatch_receipt: Option<RunGraphDispatchReceipt>,
    pub(crate) continuation_binding: Option<RunGraphContinuationBinding>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct RunGraphDispatchRouteTruthSummary {
    pub(crate) projection_source: String,
    pub(crate) projection_reason: String,
    pub(crate) projection_vs_receipt_parity: String,
    pub(crate) dispatch_receipt_present: bool,
    pub(crate) continuation_binding_present: bool,
    pub(crate) evidence_state: String,
    pub(crate) activation_kind: String,
    pub(crate) receipt_backed_execution_evidence: bool,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct RunGraphDownstreamDispatchPreviewSummary {
    pub(crate) dispatch_target: String,
    pub(crate) dispatch_status: String,
    pub(crate) lane_status: String,
    pub(crate) selected_backend: String,
    pub(crate) activation_agent_type: String,
    pub(crate) activation_runtime_role: String,
    pub(crate) downstream_dispatch_target: String,
    pub(crate) downstream_dispatch_status: String,
    pub(crate) downstream_dispatch_ready: bool,
    pub(crate) downstream_dispatch_executed_count: u32,
    pub(crate) downstream_dispatch_active_target: String,
    pub(crate) downstream_dispatch_last_target: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct RunGraphDispatchCompactSummary {
    pub(crate) route_truth: RunGraphDispatchRouteTruthSummary,
    pub(crate) downstream_dispatch_preview: RunGraphDownstreamDispatchPreviewSummary,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) stale_state_suspected: bool,
    pub(crate) recommended_command: Option<String>,
    pub(crate) recommended_surface: Option<String>,
}

fn parse_dispatch_target_from_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return None;
    }
    let dispatch_path = trimmed.strip_prefix("dispatch.")?;
    trimmed
        .split('.')
        .next_back()
        .map(str::trim)
        .filter(|value| {
            !dispatch_path.is_empty()
                && !value.is_empty()
                && *value != "none"
                && *value != "unknown"
        })
        .map(str::to_string)
}

fn next_lawful_operator_action_for_snapshot(status: &RunGraphStatus) -> Option<String> {
    if status.recovery_ready && status.resume_target != "none" {
        return Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            status.run_id
        ));
    }
    if status.status == "completed" {
        return None;
    }
    Some(machine_json_command(format!(
        "vida taskflow run-graph status {}",
        status.run_id
    )))
}

fn guard_terminal_continue_followup(status: &RunGraphStatus) -> String {
    machine_json_command(format!("vida taskflow run-graph status {}", status.run_id))
}

fn default_operator_command_text(command: &str) -> String {
    command
        .replace(" --json", "")
        .replace("--json ", "")
        .replace("--json", "")
        .trim()
        .to_string()
}

fn machine_json_command(command: impl Into<String>) -> String {
    let command = command.into();
    if command.split_whitespace().any(|token| token == "--json") {
        command
    } else {
        format!("{command} --json")
    }
}

fn sanitized_placeholder_continuation_bind_command(
    run_id: Option<&str>,
    command: Option<String>,
) -> Option<String> {
    let has_unsafe_task_bind = command.as_deref().is_some_and(|command| {
        command.starts_with("vida taskflow continuation bind")
            && (command.contains("--task-id")
                || command.contains("<task-id>")
                || command.contains("<run-id>"))
    });
    if !has_unsafe_task_bind {
        return command;
    }
    run_id
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("vida taskflow run-graph status {value}"))
        .or_else(|| Some("vida status".to_string()))
}

pub(crate) fn sanitize_placeholder_continuation_bind_recommendation(
    run_id: Option<&str>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
) -> (Option<String>, Option<String>) {
    let recommended_command =
        sanitized_placeholder_continuation_bind_command(run_id, recommended_command);
    let recommended_surface = recommended_command
        .as_deref()
        .map(recommended_surface_for_command)
        .or(recommended_surface);
    (recommended_command, recommended_surface)
}

fn sanitize_placeholder_continuation_bind_next_action(
    run_id: Option<&str>,
    next_action: Option<RecoveryNextAction>,
) -> Option<RecoveryNextAction> {
    next_action.map(|mut next_action| {
        next_action.command = sanitized_placeholder_continuation_bind_command(
            run_id,
            Some(next_action.command),
        )
        .expect("sanitized continuation command should remain present");
        next_action.surface = recommended_surface_for_command(&next_action.command);
        if next_action.command.starts_with("vida taskflow run-graph status")
            || next_action.command == "vida status"
        {
            next_action.reason = crate::status_surface_signals::terminal_next_action_requires_authoritative_run_state(run_id);
        }
        next_action
    })
}

fn dispatch_receipt_resolution_reason_class(receipt: &RunGraphDispatchReceipt) -> Option<&str> {
    if crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_exception_takeover_continuation_evidence(receipt, None)
    {
        return Some("active_exception_takeover");
    }
    let has_downstream_dispatch_blockers = receipt
        .downstream_dispatch_blockers
        .iter()
        .any(|value| !value.trim().is_empty());
    if receipt.dispatch_status != "blocked"
        && receipt.lane_status != "lane_blocked"
        && !has_downstream_dispatch_blockers
    {
        return None;
    }
    if receipt.blocker_code.as_deref() == Some("configured_backend_dispatch_failed") {
        return Some("configured_backend_dispatch_failed");
    }
    if receipt.blocker_code.as_deref() == Some("internal_activation_view_only") {
        return Some("internal_activation_view_only");
    }
    if receipt.blocker_code.as_deref() == Some("internal_dispatch_timeout_without_receipt") {
        return Some("internal_dispatch_timeout_without_receipt");
    }
    if receipt.blocker_code.as_deref() == Some("internal_codex_carrier_unavailable") {
        return Some("internal_codex_carrier_unavailable");
    }
    if receipt.blocker_code.as_deref() == Some("internal_codex_windows_sandbox_unavailable") {
        return Some("internal_codex_windows_sandbox_unavailable");
    }
    if receipt
        .downstream_dispatch_blockers
        .iter()
        .any(|value| value == "pending_terminal_write_evidence")
    {
        return Some("pending_terminal_write_evidence");
    }
    if receipt
        .downstream_dispatch_blockers
        .iter()
        .any(|value| value == "missing_owned_write_scope")
    {
        return Some("missing_owned_write_scope");
    }
    if receipt
        .downstream_dispatch_blockers
        .iter()
        .any(|value| value == "internal_dispatch_timeout_without_receipt")
    {
        return Some("internal_dispatch_timeout_without_receipt");
    }
    None
}

fn next_lawful_operator_action_for_dispatch_resolution(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceipt,
    terminal_consume_continue_run_id: Option<&str>,
) -> Option<String> {
    if dispatch_receipt_has_clean_ready_downstream_handoff(receipt)
        && terminal_consume_continue_run_id == Some(status.run_id.as_str())
    {
        return downstream_dispatch_command_for_receipt(receipt)
            .map(machine_json_command)
            .or_else(|| {
                Some(machine_json_command(format!(
                    "vida lane show {}",
                    status.run_id
                )))
            });
    }
    if dispatch_receipt_has_clean_routed_agent_handoff(receipt, Some(&status.run_id)) {
        return routed_dispatch_command_for_receipt(receipt)
            .map(machine_json_command)
            .or_else(|| {
                Some(machine_json_command(format!(
                    "vida lane show {}",
                    status.run_id
                )))
            });
    }
    let _reason_class = dispatch_receipt_resolution_reason_class(receipt)?;
    if receipt.blocker_code.as_deref() == Some("internal_dispatch_timeout_without_receipt") {
        if status.recovery_ready
            && status.resume_target != "none"
            && receipt
                .dispatch_packet_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
        {
            return Some(format!(
                "vida taskflow consume continue --run-id {} --json",
                shell_quote(&status.run_id)
            ));
        }
        return Some(machine_json_command(format!(
            "vida lane show {}",
            shell_quote(&status.run_id)
        )));
    }
    if let Some(receipt_id) = receipt
        .exception_path_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|_| receipt.supersedes_receipt_id.is_none())
    {
        return Some(format!(
            "vida lane supersede {} --receipt-id {} --json",
            shell_quote(&status.run_id),
            shell_quote(receipt_id)
        ));
    }
    if receipt.supersedes_receipt_id.is_some() && receipt.exception_path_receipt_id.is_some() {
        if !status.recovery_ready || status.resume_target == "none" {
            return Some(machine_json_command(format!(
                "vida lane show {}",
                status.run_id
            )));
        }
        if terminal_consume_continue_run_id == Some(status.run_id.as_str()) {
            return Some(guard_terminal_continue_followup(status));
        }
        return (status.status != "completed").then(|| {
            format!(
                "vida taskflow consume continue --run-id {} --json",
                status.run_id
            )
        });
    }
    Some(machine_json_command(format!(
        "vida lane show {}",
        status.run_id
    )))
}

fn dispatch_receipt_has_clean_ready_downstream_handoff(receipt: &RunGraphDispatchReceipt) -> bool {
    crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_ready_downstream_handoff(
        receipt, None,
    )
}

fn dispatch_receipt_has_clean_routed_agent_handoff(
    receipt: &RunGraphDispatchReceipt,
    expected_run_id: Option<&str>,
) -> bool {
    crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_routed_agent_handoff(
        receipt,
        expected_run_id,
    )
}

fn downstream_dispatch_command_for_receipt(receipt: &RunGraphDispatchReceipt) -> Option<String> {
    crate::continuation_binding_summary::downstream_dispatch_command_from_parts(
        receipt.downstream_dispatch_command.as_deref(),
        receipt.downstream_dispatch_packet_path.as_deref(),
    )
    .map(|command| machine_json_command(default_operator_command_text(&command)))
    .filter(|command| !command.is_empty())
}

fn routed_dispatch_command_for_receipt(receipt: &RunGraphDispatchReceipt) -> Option<String> {
    crate::continuation_binding_summary::routed_dispatch_command_from_parts(
        receipt.dispatch_command.as_deref(),
        receipt.dispatch_packet_path.as_deref(),
    )
    .map(|command| default_operator_command_text(&command))
    .filter(|command| !command.is_empty())
}

pub(crate) fn dispatch_receipt_can_use_terminal_continue_evidence(
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    receipt.supersedes_receipt_id.is_some() && receipt.exception_path_receipt_id.is_some()
}

fn halted_external_dispatch_artifact_mismatched_as_internal_activation(
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_status != "blocked"
        || receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
    {
        return false;
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return false;
    };
    if result["execution_state"].as_str() != Some("blocked")
        || result["blocker_code"].as_str() != Some("internal_activation_view_only")
    {
        return false;
    }
    let selected_backend = receipt
        .selected_backend
        .as_deref()
        .or_else(|| result["selected_backend"].as_str());
    let Some(selected_backend) = selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if selected_backend == "internal_subagents" {
        return false;
    }
    receipt
        .dispatch_surface
        .as_deref()
        .is_some_and(|value| value.starts_with("external_cli:"))
        || result["surface"]
            .as_str()
            .is_some_and(|value| value.starts_with("external_cli:"))
        || result["backend_dispatch"]["backend_class"].as_str() == Some("external_cli")
        || (selected_backend.ends_with("_cli")
            && result["lane_execution_receipt_artifact"]["carrier_id"].as_str()
                == Some(selected_backend))
}

fn next_lawful_operator_action_for_projection(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
    terminal_consume_continue_run_id: Option<&str>,
    task_missing: bool,
    task_closed_stale_run: bool,
) -> Option<String> {
    if missing_task_run_graph_requires_stale_cleanup(Some(status), task_missing) {
        if recovery_lane_retire_admissibility(
            status,
            receipt,
            taskflow_authority::stale_guard::StaleRunTaskState::Missing,
        )
        .is_allowed()
        {
            return Some(format!(
                "vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\" --json",
                status.run_id, status.run_id
            ));
        }
    }
    if task_missing {
        return None;
    }
    if closed_task_run_graph_requires_stale_cleanup(Some(status), task_closed_stale_run) {
        return Some(closed_task_active_run_projection_mismatch_command());
    }
    if receipt.is_some_and(halted_external_dispatch_artifact_mismatched_as_internal_activation) {
        if terminal_consume_continue_run_id == Some(status.run_id.as_str()) {
            return Some(guard_terminal_continue_followup(status));
        }
        return Some(machine_json_command(format!(
            "vida lane show {}",
            status.run_id
        )));
    }
    if let Some(command) = receipt.and_then(|value| {
        if value
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| blocker == "missing_owned_write_scope")
        {
            return Some(machine_json_command(format!(
                "vida taskflow packet render {}",
                status.run_id
            )));
        }
        next_lawful_operator_action_for_dispatch_resolution(
            status,
            value,
            terminal_consume_continue_run_id,
        )
    }) {
        return Some(command);
    }
    next_lawful_operator_action_for_snapshot(status)
}

fn recommended_surface_for_command(command: &str) -> String {
    if command.starts_with("vida status") {
        return "vida status".to_string();
    }
    if command.starts_with("vida agent-init") {
        return "vida agent-init".to_string();
    }
    if command.starts_with("vida taskflow consume continue") {
        return "vida taskflow consume continue".to_string();
    }
    if command.starts_with("vida taskflow recovery latest") {
        return "vida taskflow recovery latest".to_string();
    }
    if command.starts_with("vida taskflow run-graph status") {
        return "vida taskflow run-graph status".to_string();
    }
    if command.starts_with("vida task show") {
        return "vida task show".to_string();
    }
    if command.starts_with("vida task ready") {
        return "vida task ready".to_string();
    }
    if command.starts_with("vida task deps") {
        return "vida task deps".to_string();
    }
    if command.starts_with("vida task critical-path") {
        return "vida task critical-path".to_string();
    }
    if command.starts_with("vida orchestrator-session show") {
        return "vida orchestrator-session show".to_string();
    }
    if command.starts_with("vida taskflow continuation bind") {
        return "vida taskflow continuation bind".to_string();
    }
    if command.starts_with("vida lane show") {
        return "vida lane show".to_string();
    }
    if command.starts_with("vida lane exception-takeover") {
        return "vida lane exception-takeover".to_string();
    }
    if command.starts_with("vida lane retire") {
        return "vida lane retire".to_string();
    }
    if command.starts_with("vida lane supersede") {
        return "vida lane supersede".to_string();
    }
    operator_output::command_text::human_command(command)
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

fn recovery_next_action_reason(
    command: &str,
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
) -> String {
    if command.starts_with("vida lane exception-takeover") {
        return "record bounded exception-path evidence for the dispatch blocker before local recovery work".to_string();
    }
    if command.starts_with("vida lane retire") {
        return "retire the stale missing-task run graph through the bounded lane repair surface before rebinding continuation".to_string();
    }
    if command.starts_with("vida lane supersede") {
        return "activate the recorded exception-path receipt before treating local recovery as lawful".to_string();
    }
    if command.starts_with("vida taskflow continuation bind") {
        return "the latest consume-continue snapshot already completed without a next action, so confirm the authoritative run state and bind the next bounded unit with the concrete run/task ids instead of repeating continuation".to_string();
    }
    if command.starts_with("vida lane show") {
        return "inspect the lane envelope for the dispatch blocker, then record structured exception-takeover evidence and supersession before any local recovery work".to_string();
    }
    if command.starts_with("vida task show") {
        return "inspect the active task owned paths before recording structured exception-takeover evidence for the terminal dispatch blocker".to_string();
    }
    if command.starts_with("vida agent-init") {
        return "execute the ready materialized agent dispatch packet without granting root-local product write authority".to_string();
    }
    if command.starts_with("vida taskflow consume continue")
        && projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                receipt.blocker_code.as_deref() == Some("internal_dispatch_timeout_without_receipt")
                    && receipt
                        .dispatch_packet_path
                        .as_deref()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
            })
    {
        return "retry the same bounded agent lane from persisted dispatch packet evidence after a terminal internal timeout without granting root-local product write authority".to_string();
    }
    if command.starts_with("vida taskflow consume continue")
        && projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                receipt.exception_path_receipt_id.is_some()
                    && receipt.supersedes_receipt_id.is_some()
                    && dispatch_receipt_resolution_reason_class(receipt).is_some()
            })
    {
        return "continue the TaskFlow run after active exception-takeover evidence resolved the dispatch blocker".to_string();
    }
    if projection_truth.stale_state_suspected {
        "stale delegated execution is suspected; inspect the authoritative run-graph status before re-dispatch".to_string()
    } else if summary.recovery_ready {
        "recovery is ready; continue the lawful delegated chain".to_string()
    } else {
        "inspect the authoritative run-graph status for the bound recovery state".to_string()
    }
}

async fn recovery_owned_write_scope_for_summary(
    store: &StateStore,
    summary: &crate::state_store::RunGraphRecoverySummary,
) -> Vec<String> {
    match store.show_task(&summary.task_id).await {
        Ok(task) => task
            .planner_metadata
            .owned_paths
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn recovery_exception_takeover_next_action(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    owned_write_scope_hint: &[String],
) -> Option<RecoveryNextAction> {
    if projection_truth.stale_state_suspected {
        return None;
    }
    if !summary.delegation_gate.delegated_cycle_open {
        return None;
    }
    if recovery_projection_resolves_persisted_open_cycle(summary, projection_truth)
        || projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                dispatch_receipt_has_clean_routed_agent_handoff(receipt, Some(&summary.run_id))
            })
        || recovery_ready_handoff_resolves_open_cycle(summary)
    {
        return None;
    }
    let receipt = projection_truth.dispatch_receipt.as_ref();
    if receipt.is_some_and(|receipt| {
        receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }) {
        return None;
    }

    let task_id = summary.task_id.trim();
    let task_id = if task_id.is_empty() {
        summary.run_id.trim()
    } else {
        task_id
    };
    if owned_write_scope_hint
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .all(str::is_empty)
    {
        return None;
    }

    let reason_class = receipt
        .and_then(|receipt| receipt.blocker_code.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            receipt.and_then(|receipt| {
                receipt
                    .downstream_dispatch_blockers
                    .iter()
                    .map(String::as_str)
                    .map(str::trim)
                    .find(|value| !value.is_empty())
            })
        })
        .or_else(|| summary.delegation_gate.blocker_code.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("blocked_open_delegated_cycle");
    let active_node = summary.active_node.trim();
    let active_node = if active_node.is_empty() {
        "delegated-lane"
    } else {
        active_node
    };
    let active_bounded_unit = format!("{task_id}:{active_node}:exception-takeover");
    let owned_write_scope_args = owned_write_scope_hint
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("--owned-write-scope {}", shell_quote(value)))
        .collect::<Vec<_>>();
    let receipt_id = format!("{}-exception-takeover", summary.run_id.trim());
    let why_delegated_not_lawful = format!(
        "delegated lane is blocked for run {} by {}",
        summary.run_id.trim(),
        reason_class
    );
    let why_local_safe = format!(
        "bounded exception recovery is limited to the active {} unit and declared owned write scope",
        active_node
    );
    let verification_step = format!(
        "run focused proof for {} before local write closure",
        active_bounded_unit
    );
    let command = format!(
        "vida lane exception-takeover {} --receipt-id {} --reason-class {} --active-bounded-unit {} {} --why-delegated-path-not-lawful {} --why-local-write-safe {} --return-to-normal-when {} --verification-step {} --json",
        shell_quote(summary.run_id.trim()),
        shell_quote(&receipt_id),
        shell_quote(reason_class),
        shell_quote(&active_bounded_unit),
        owned_write_scope_args.join(" "),
        shell_quote(&why_delegated_not_lawful),
        shell_quote(&why_local_safe),
        shell_quote("after focused proof, release install, task closure, and lane completion"),
        shell_quote(&verification_step),
    );
    Some(RecoveryNextAction {
        surface: recommended_surface_for_command(&command),
        command,
        reason: "record bounded exception-path evidence for the dispatch blocker before local recovery work".to_string(),
    })
}

/// Classify blocker codes into one of four primary diagnosis types.
/// Returns the diagnosis type as a string: runtime_defect, carrier_unavailable, packet_invalid, or user_action_needed.
fn categorize_recovery_diagnosis(
    blocker_codes: &[String],
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
) -> String {
    if blocker_codes.is_empty()
        && recovery_projection_has_active_exception_takeover(projection_truth)
    {
        return "runtime_defect".to_string();
    }

    let blocker_classification = classify_recovery_blocker_codes(blocker_codes);

    if blocker_classification.carrier_unavailable {
        return "carrier_unavailable".to_string();
    }

    if blocker_classification.packet_invalid {
        return "packet_invalid".to_string();
    }

    if summary.delegation_gate.delegated_cycle_open || blocker_classification.user_action_needed {
        return "user_action_needed".to_string();
    }

    // Default to runtime_defect for any other blocker
    if !blocker_codes.is_empty() {
        return "runtime_defect".to_string();
    }

    // If no blocker codes but still blocked (e.g., stale state), it's a runtime defect
    if projection_truth.stale_state_suspected {
        return "runtime_defect".to_string();
    }

    // If recovery is ready and not blocked, no diagnosis needed
    // But per task requirements, we should still emit one of the four types
    // Default to user_action_needed for non-blocked but not-terminal states
    if !summary.recovery_ready {
        return "runtime_defect".to_string();
    }

    "user_action_needed".to_string()
}

#[derive(Debug, Default)]
struct RecoveryBlockerCodeClassification {
    carrier_unavailable: bool,
    packet_invalid: bool,
    user_action_needed: bool,
}

fn classify_recovery_blocker_codes(blocker_codes: &[String]) -> RecoveryBlockerCodeClassification {
    let mut classification = RecoveryBlockerCodeClassification::default();
    for code in blocker_codes {
        classification.record(code);
    }
    classification
}

impl RecoveryBlockerCodeClassification {
    fn record(&mut self, code: &str) {
        let code = code.to_lowercase();
        self.carrier_unavailable |= recovery_blocker_code_is_carrier_unavailable(&code);
        self.packet_invalid |= recovery_blocker_code_is_packet_invalid(&code);
        self.user_action_needed |= recovery_blocker_code_is_user_action_needed(&code);
    }
}

fn recovery_blocker_code_is_carrier_unavailable(code: &str) -> bool {
    code.contains("carrier_unavailable")
        || code.contains("codex_carrier_unavailable")
        || code == "internal_codex_carrier_unavailable"
}

fn recovery_blocker_code_is_packet_invalid(code: &str) -> bool {
    code.contains("packet") && code.contains("invalid")
        || code == "packet_invalid"
        || code.contains("receipt") && code.contains("invalid")
}

fn recovery_blocker_code_is_user_action_needed(code: &str) -> bool {
    code == "open_delegated_cycle"
        || code.contains("user_action")
        || code.contains("delegate")
        || code == "internal_activation_view_only"
}

fn recovery_surface_contract(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
) -> (
    Vec<String>,
    Option<RecoveryWhyNotNow>,
    Option<RecoveryNextAction>,
    Option<String>,
    Option<String>,
) {
    recovery_surface_contract_with_owned_scope(summary, projection_truth, &[])
}

fn recovery_surface_contract_with_owned_scope(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    owned_write_scope_hint: &[String],
) -> (
    Vec<String>,
    Option<RecoveryWhyNotNow>,
    Option<RecoveryNextAction>,
    Option<String>,
    Option<String>,
) {
    if projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            crate::runtime_dispatch_receipt_helpers::dispatch_receipt_is_materialization_only_blocked_task_ensure(receipt)
        })
    {
        let blocker_codes = vec!["internal_activation_view_only".to_string()];
        return (
            blocker_codes.clone(),
            Some(RecoveryWhyNotNow {
                category: "run_graph_blocked_state".to_string(),
                summary: "The persisted dispatch receipt is only a task-materialization result and remains blocked by internal activation view-only evidence.".to_string(),
                blocker_codes,
                blocking_surface: Some("vida taskflow recovery latest".to_string()),
            }),
            None,
            None,
            None,
        );
    }
    if terminal_recovery_summary_resolved(summary) {
        return (Vec::new(), None, None, None, None);
    }
    let task_authority_blockers = projection_truth_issue_codes(projection_truth);
    if task_authority_blockers.iter().any(|code| {
        matches!(
            code.as_str(),
            "stale_missing_task_run_graph" | "closed_task_active_run_projection_mismatch"
        )
    }) {
        let blocker_codes = normalize_run_graph_issue_codes(
            &task_authority_blockers,
            projection_truth.stale_state_suspected,
        );
        let recommended_command = projection_truth.next_lawful_operator_action.clone();
        let recommended_surface = recommended_command.as_ref().map(|command| {
            if command.trim() == closed_task_active_run_projection_mismatch_command() {
                "vida task reconcile-closed-runs".to_string()
            } else if command.starts_with("vida lane retire") {
                "vida lane retire".to_string()
            } else {
                "vida taskflow run-graph status".to_string()
            }
        });
        let next_action = recommended_command
            .as_ref()
            .map(|command| RecoveryNextAction {
                command: command.clone(),
                surface: recommended_surface
                    .clone()
                    .unwrap_or_else(|| "vida taskflow run-graph status".to_string()),
                reason: format!(
                    "stale TaskFlow task authority requires projection cleanup before recovery; run `{command}`"
                ),
            });
        return (
            blocker_codes.clone(),
            Some(RecoveryWhyNotNow {
                category: "stale_run_graph_blocked_state".to_string(),
                summary:
                    "The run graph is stale because its active TaskFlow task identity is not actionable."
                        .to_string(),
                blocker_codes,
                blocking_surface: Some("vida taskflow recovery latest".to_string()),
            }),
            next_action,
            recommended_command,
            recommended_surface,
        );
    }

    let projection_resolves_open_cycle =
        recovery_projection_resolves_persisted_open_cycle(summary, projection_truth);
    let active_exception_takeover_resolves_open_cycle =
        recovery_active_exception_takeover_resolves_persisted_open_cycle(
            summary,
            projection_truth,
            owned_write_scope_hint,
        );
    let ready_handoff_resolves_open_cycle = recovery_ready_handoff_resolves_open_cycle(summary);
    let routed_agent_handoff_resolves_open_cycle = summary.delegation_gate.delegated_cycle_open
        && summary.delegation_gate.blocker_code.as_deref() == Some("open_delegated_cycle")
        && summary.recovery_ready
        && summary.resume_status == "ready"
        && summary.resume_target.starts_with("dispatch.")
        && projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                dispatch_receipt_has_clean_routed_agent_handoff(receipt, Some(&summary.run_id))
            });
    let projection_ready_handoff_resolves_open_cycle = summary.delegation_gate.delegated_cycle_open
        && summary.delegation_gate.blocker_code.as_deref() == Some("open_delegated_cycle")
        && summary.recovery_ready
        && summary.resume_status == "ready"
        && summary.resume_target.starts_with("dispatch.")
        && projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                summary.active_node == receipt.dispatch_target
                    && matches!(receipt.dispatch_status.as_str(), "routed" | "packet_ready")
                    && receipt.blocker_code.as_deref().is_none_or(str::is_empty)
            });
    let downstream_ready_handoff_resolves_open_cycle = summary.delegation_gate.delegated_cycle_open
        && summary.delegation_gate.blocker_code.as_deref() == Some("open_delegated_cycle")
        && summary.recovery_ready
        && summary.resume_status == "ready"
        && summary.resume_target.starts_with("dispatch.")
        && projection_truth
            .dispatch_receipt
            .as_ref()
            .is_some_and(|receipt| {
                receipt
                    .downstream_dispatch_target
                    .as_deref()
                    .is_some_and(|target| target == summary.active_node)
                    && receipt
                        .downstream_dispatch_status
                        .as_deref()
                        .is_some_and(|status| status == "packet_ready")
                    && receipt.downstream_dispatch_blockers.is_empty()
            });
    let mut blocker_codes = if projection_resolves_open_cycle
        || active_exception_takeover_resolves_open_cycle
        || ready_handoff_resolves_open_cycle
        || routed_agent_handoff_resolves_open_cycle
        || projection_ready_handoff_resolves_open_cycle
        || downstream_ready_handoff_resolves_open_cycle
    {
        Vec::new()
    } else {
        summary
            .delegation_gate
            .blocker_code
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| vec![value.to_string()])
            .unwrap_or_default()
    };
    if !active_exception_takeover_resolves_open_cycle
        && !routed_agent_handoff_resolves_open_cycle
        && !downstream_ready_handoff_resolves_open_cycle
    {
        blocker_codes.extend(projection_truth_issue_codes_for_ready_handoff(
            &summary.active_node,
            &summary.resume_status,
            summary.recovery_ready,
            &summary.resume_target,
            projection_truth,
        ));
    }
    let blocker_codes = normalize_run_graph_issue_codes(&blocker_codes, false);

    let next_action = projection_truth
        .next_lawful_operator_action
        .as_deref()
        .map(|command| RecoveryNextAction {
            command: command.to_string(),
            surface: recommended_surface_for_command(command),
            reason: recovery_next_action_reason(command, summary, projection_truth),
        });
    let next_action =
        recovery_exception_takeover_next_action(summary, projection_truth, owned_write_scope_hint)
            .or(next_action);
    let why_not_now = (!blocker_codes.is_empty()).then(|| {
        let delegated_cycle_open = summary.delegation_gate.delegated_cycle_open;
        let stale_state_suspected = projection_truth.stale_state_suspected;
        RecoveryWhyNotNow {
            category: if delegated_cycle_open {
                "delegated_cycle_runtime_gate".to_string()
            } else if stale_state_suspected {
                "stale_run_graph_blocked_state".to_string()
            } else {
                "run_graph_blocked_state".to_string()
            },
            summary: if delegated_cycle_open && stale_state_suspected {
                format!(
                    "The delegated cycle remains open in recovery state `{}`, and the persisted delegated execution now looks stale.",
                    summary.delegation_gate.delegated_cycle_state
                )
            } else if delegated_cycle_open {
                format!(
                    "The delegated cycle remains open in recovery state `{}`.",
                    summary.delegation_gate.delegated_cycle_state
                )
            } else if stale_state_suspected {
                format!(
                    "The run graph is blocked while delegated cycle state is `{}`; persisted dispatch evidence looks stale.",
                    summary.delegation_gate.delegated_cycle_state
                )
            } else {
                format!(
                    "The run graph is blocked while delegated cycle state is `{}`.",
                    summary.delegation_gate.delegated_cycle_state
                )
            },
            blocker_codes: blocker_codes.clone(),
            blocking_surface: Some("vida taskflow recovery latest".to_string()),
        }
    });
    let recommended_command = next_action.as_ref().map(|value| value.command.clone());
    let recommended_surface = next_action.as_ref().map(|value| value.surface.clone());

    (
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn recovery_projection_resolves_persisted_open_cycle(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
) -> bool {
    if !summary.delegation_gate.delegated_cycle_open {
        return false;
    }
    if summary.delegation_gate.blocker_code.as_deref() != Some("open_delegated_cycle") {
        return false;
    }
    if projection_truth.projection_vs_receipt_parity != "reconciled_from_receipt"
        && projection_truth.projection_vs_receipt_parity != "aligned"
    {
        return false;
    }
    projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            let upstream_lane_completed =
                crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_completed_lane(
                    receipt, Some(&summary.run_id),
                )
                    || crate::runtime_dispatch_receipt_helpers::dispatch_receipt_downstream_blockers_superseded_by_ready_handoff_fields(
                        &summary.run_id,
                        &summary.active_node,
                        &summary.resume_status,
                        summary.recovery_ready,
                        &summary.resume_target,
                        receipt,
                    );
            let ready_running_handoff = receipt.lane_status == "lane_running"
                && crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_ready_downstream_handoff(
                    receipt,
                    Some(&summary.run_id),
                );
            upstream_lane_completed || ready_running_handoff
        })
}

fn recovery_active_exception_takeover_resolves_persisted_open_cycle(
    summary: &crate::state_store::RunGraphRecoverySummary,
    projection_truth: &RunGraphProjectionTruth,
    owned_write_scope_hint: &[String],
) -> bool {
    if !summary.delegation_gate.delegated_cycle_open {
        return false;
    }
    if summary.delegation_gate.blocker_code.as_deref() != Some("open_delegated_cycle") {
        return false;
    }
    if owned_write_scope_hint
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .all(str::is_empty)
    {
        return false;
    }
    recovery_projection_has_active_exception_takeover(projection_truth)
}

fn recovery_projection_has_active_exception_takeover(
    projection_truth: &RunGraphProjectionTruth,
) -> bool {
    projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_active_exception_takeover(
                receipt, None,
            )
        })
}

fn recovery_ready_handoff_resolves_open_cycle(
    summary: &crate::state_store::RunGraphRecoverySummary,
) -> bool {
    summary.delegation_gate.delegated_cycle_open
        && summary.delegation_gate.delegated_cycle_state == "handoff_pending"
        && summary.delegation_gate.blocker_code.is_none()
        && summary.recovery_ready
        && summary.resume_status == "ready"
        && summary.resume_target.starts_with("dispatch.")
}

async fn build_run_graph_diagnosis(
    store: &StateStore,
    run_id: &str,
) -> Result<RunGraphDiagnosis, StateStoreError> {
    let summary = store.run_graph_recovery_summary(run_id).await?;
    let status = store.run_graph_status(&summary.run_id).await?;
    let projection_truth = run_graph_projection_truth(store, &status).await?;
    let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
        recovery_surface_contract(&summary, &projection_truth);
    let next_action =
        sanitize_placeholder_continuation_bind_next_action(Some(&summary.run_id), next_action);
    let (recommended_command, recommended_surface) =
        sanitize_placeholder_continuation_bind_recommendation(
            Some(&summary.run_id),
            recommended_command,
            recommended_surface,
        );
    Ok(RunGraphDiagnosis {
        run_id: summary.run_id.clone(),
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
        recovery: summary,
        projection_truth,
    })
}

fn projection_vs_receipt_parity(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return "no_receipt".to_string();
    };
    if receipt.dispatch_status == status.status
        || receipt.downstream_dispatch_status.as_deref() == Some(status.status.as_str())
    {
        return "aligned".to_string();
    }
    "reconciled_from_receipt".to_string()
}

fn projection_reason_for_snapshot(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
    binding: Option<&RunGraphContinuationBinding>,
) -> String {
    if let Some(receipt) = receipt {
        if receipt.dispatch_status != status.status
            || receipt.downstream_dispatch_status.as_deref() == Some(status.status.as_str())
        {
            return "run-graph status was reconciled against persisted dispatch receipt evidence"
                .to_string();
        }
        if receipt.blocker_code.is_some() || !receipt.downstream_dispatch_blockers.is_empty() {
            return "run-graph status reflects persisted dispatch blocker evidence".to_string();
        }
    }
    if let Some(binding) = binding {
        return format!(
            "run-graph status is paired with explicit continuation binding from `{}`",
            binding.binding_source
        );
    }
    if status.status == "completed" {
        return "run-graph status reflects terminal state without additional projection inputs"
            .to_string();
    }
    "run-graph status reflects authoritative persisted state".to_string()
}

fn continuation_binding_matches_reconciled_snapshot(
    status: &RunGraphStatus,
    binding: &RunGraphContinuationBinding,
) -> bool {
    if binding.run_id != status.run_id || binding.task_id != status.task_id {
        return false;
    }
    if binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
        != Some("run_graph_task")
    {
        return true;
    }
    if status.status == "completed" {
        return true;
    }
    binding
        .active_bounded_unit
        .get("active_node")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|active_node| active_node == status.active_node)
}

fn active_exception_takeover_receipt_matches_snapshot(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> bool {
    let Some(receipt) = receipt else {
        return false;
    };
    crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_active_exception_takeover(
        receipt,
        Some(&status.run_id),
    )
}

fn snapshot_derived_exception_takeover_binding(
    status: &RunGraphStatus,
) -> RunGraphContinuationBinding {
    RunGraphContinuationBinding {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        status: "bound".to_string(),
        active_bounded_unit: serde_json::json!({
            "kind": "run_graph_task",
            "task_id": status.task_id,
            "run_id": status.run_id,
            "active_node": status.active_node,
        }),
        binding_source: "latest_run_graph_exception_takeover_dispatch".to_string(),
        why_this_unit: format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}` at node `{}`.",
            status.task_id, status.active_node
        ),
        primary_path: "normal_delivery_path".to_string(),
        sequential_vs_parallel_posture: "sequential_only_exception_takeover".to_string(),
        request_text: None,
        recorded_at: String::new(),
    }
}

fn effective_projection_continuation_binding(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
    binding: Option<RunGraphContinuationBinding>,
) -> Option<RunGraphContinuationBinding> {
    if let Some(binding) = binding {
        if continuation_binding_matches_reconciled_snapshot(status, &binding) {
            return Some(binding);
        }
    }
    active_exception_takeover_receipt_matches_snapshot(status, receipt)
        .then(|| snapshot_derived_exception_takeover_binding(status))
}

fn continuation_binding_source_from_state_surface(
    continuation_binding: Option<&serde_json::Value>,
) -> Option<String> {
    let binding = continuation_binding?;
    let status = binding["status"].as_str().unwrap_or("unknown");
    if matches!(status, "unknown" | "none") {
        return None;
    }
    binding["primary_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unknown")
        .map(str::to_string)
}

fn continuation_binding_exposes_closed_task_active_run_projection_mismatch(
    continuation_binding: Option<&serde_json::Value>,
) -> bool {
    continuation_binding.is_some_and(|binding| {
        binding["ambiguity_reason"].as_str() == Some("closed_task_active_run_projection_mismatch")
            || binding["blocker_codes"].as_array().is_some_and(|codes| {
                codes
                    .iter()
                    .any(|code| code.as_str() == Some("closed_task_active_run_projection_mismatch"))
            })
    })
}

fn dispatch_receipt_from_state_surface(
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> RunGraphDispatchReceipt {
    RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: receipt.dispatch_target.clone(),
        dispatch_status: receipt.dispatch_status.clone(),
        lane_status: receipt.lane_status.clone(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind: receipt.dispatch_kind.clone(),
        dispatch_surface: receipt.dispatch_surface.clone(),
        dispatch_command: receipt.dispatch_command.clone(),
        dispatch_packet_path: receipt.dispatch_packet_path.clone(),
        dispatch_result_path: receipt.dispatch_result_path.clone(),
        blocker_code: receipt.blocker_code.clone(),
        downstream_dispatch_target: receipt.downstream_dispatch_target.clone(),
        downstream_dispatch_command: receipt.downstream_dispatch_command.clone(),
        downstream_dispatch_note: receipt.downstream_dispatch_note.clone(),
        downstream_dispatch_ready: receipt.downstream_dispatch_ready,
        downstream_dispatch_blockers: receipt.downstream_dispatch_blockers.clone(),
        downstream_dispatch_packet_path: receipt.downstream_dispatch_packet_path.clone(),
        downstream_dispatch_status: receipt.downstream_dispatch_status.clone(),
        downstream_dispatch_result_path: receipt.downstream_dispatch_result_path.clone(),
        downstream_dispatch_trace_path: receipt.downstream_dispatch_trace_path.clone(),
        downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
        downstream_dispatch_active_target: receipt.downstream_dispatch_active_target.clone(),
        downstream_dispatch_last_target: receipt.downstream_dispatch_last_target.clone(),
        activation_agent_type: receipt.activation_agent_type.clone(),
        activation_runtime_role: receipt.activation_runtime_role.clone(),
        selected_backend: receipt.selected_backend.clone(),
        policy_bundle_ref: receipt.policy_bundle_ref.clone(),
        recorded_at: receipt.recorded_at.clone(),
    }
}

fn activation_string_field(evidence: Option<&serde_json::Value>, key: &str) -> Option<String> {
    evidence
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "none" && *value != "unknown")
        .map(str::to_string)
}

fn activation_kind_from_evidence(evidence: Option<&serde_json::Value>) -> String {
    evidence
        .and_then(|value| value.get("activation_kind"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            evidence
                .and_then(|value| value.get("activation_semantics"))
                .and_then(|value| value.get("activation_kind"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("unknown")
        .to_string()
}

fn receipt_backed_execution_evidence_from_evidence(evidence: Option<&serde_json::Value>) -> bool {
    evidence
        .and_then(|value| value.get("receipt_backed"))
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            evidence
                .and_then(|value| value.get("execution_evidence"))
                .and_then(|value| value.get("receipt_backed"))
                .and_then(serde_json::Value::as_bool)
        })
        .unwrap_or(false)
}

fn route_truth_from_projection_truth(
    projection_truth: &RunGraphProjectionTruth,
    evidence: Option<&serde_json::Value>,
) -> RunGraphDispatchRouteTruthSummary {
    let mut projection_reason = projection_truth.projection_reason.clone();
    if projection_truth.stale_state_suspected {
        projection_reason =
            format!("{projection_reason}; persisted delegated execution now looks stale");
    }
    RunGraphDispatchRouteTruthSummary {
        projection_source: projection_truth.projection_source.clone(),
        projection_reason,
        projection_vs_receipt_parity: projection_truth.projection_vs_receipt_parity.clone(),
        dispatch_receipt_present: projection_truth.dispatch_receipt_present,
        continuation_binding_present: projection_truth.continuation_binding_present,
        evidence_state: evidence
            .and_then(|value| value.get("evidence_state"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        activation_kind: activation_kind_from_evidence(evidence),
        receipt_backed_execution_evidence: receipt_backed_execution_evidence_from_evidence(
            evidence,
        ),
    }
}

fn downstream_dispatch_preview_from_run_snapshot(
    status: &RunGraphStatus,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    continuation_binding_source: Option<&str>,
    evidence: Option<&serde_json::Value>,
) -> RunGraphDownstreamDispatchPreviewSummary {
    let derived_downstream_target = continuation_binding_source
        .and_then(parse_dispatch_target_from_path)
        .or_else(|| parse_dispatch_target_from_path(&status.resume_target))
        .or_else(|| status.next_node.clone())
        .unwrap_or_else(|| "none".to_string());
    let downstream_dispatch_ready = receipt
        .map(|value| value.downstream_dispatch_ready)
        .unwrap_or_else(|| {
            derived_downstream_target != "none"
                && status.recovery_ready
                && status.resume_target != "none"
                && status.status != "completed"
        });
    let derived_downstream_status = if derived_downstream_target == "none" {
        "none".to_string()
    } else if status.status == "completed" {
        "not_required".to_string()
    } else if downstream_dispatch_ready {
        "resume_ready".to_string()
    } else {
        "pending_receipt".to_string()
    };

    RunGraphDownstreamDispatchPreviewSummary {
        dispatch_target: receipt
            .map(|value| value.dispatch_target.clone())
            .unwrap_or_else(|| status.active_node.clone()),
        dispatch_status: receipt
            .map(|value| value.dispatch_status.clone())
            .unwrap_or_else(|| status.status.clone()),
        lane_status: receipt
            .map(|value| value.lane_status.clone())
            .unwrap_or_else(|| status.lifecycle_stage.clone()),
        selected_backend: receipt
            .and_then(|value| value.selected_backend.clone())
            .or_else(|| activation_string_field(evidence, "selected_backend"))
            .or_else(|| {
                evidence
                    .and_then(|value| value.get("execution_evidence"))
                    .and_then(|value| value.get("selected_backend"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| status.selected_backend.clone()),
        activation_agent_type: receipt
            .and_then(|value| value.activation_agent_type.clone())
            .or_else(|| activation_string_field(evidence, "agent_type"))
            .or_else(|| activation_string_field(evidence, "selected_agent_type"))
            .unwrap_or_else(|| "none".to_string()),
        activation_runtime_role: receipt
            .and_then(|value| value.activation_runtime_role.clone())
            .or_else(|| activation_string_field(evidence, "runtime_role"))
            .or_else(|| activation_string_field(evidence, "selected_runtime_role"))
            .unwrap_or_else(|| "none".to_string()),
        downstream_dispatch_target: receipt
            .and_then(|value| value.downstream_dispatch_target.clone())
            .unwrap_or(derived_downstream_target),
        downstream_dispatch_status: receipt
            .and_then(|value| value.downstream_dispatch_status.clone())
            .unwrap_or(derived_downstream_status),
        downstream_dispatch_ready,
        downstream_dispatch_executed_count: receipt
            .map(|value| value.downstream_dispatch_executed_count)
            .unwrap_or_default(),
        downstream_dispatch_active_target: receipt
            .and_then(|value| value.downstream_dispatch_active_target.clone())
            .unwrap_or_else(|| status.active_node.clone()),
        downstream_dispatch_last_target: receipt
            .and_then(|value| value.downstream_dispatch_last_target.clone())
            .unwrap_or_else(|| {
                parse_dispatch_target_from_path(&status.resume_target)
                    .or_else(|| status.next_node.clone())
                    .unwrap_or_else(|| status.active_node.clone())
            }),
    }
}

fn dispatch_issue_codes_from_state_surface(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    let mut blocked_evidence_present = false;
    if let Some(summary) = recovery {
        if !recovery_ready_handoff_resolves_open_cycle(summary) {
            if let Some(blocker_code) = summary
                .delegation_gate
                .blocker_code
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                blocker_codes.push(blocker_code.to_string());
            }
        }
        blocked_evidence_present |= halted_state_signal(&summary.lifecycle_stage)
            || halted_state_signal(&summary.resume_status);
    }
    if let Some(summary) = receipt {
        if crate::runtime_dispatch_receipt_helpers::dispatch_summary_is_materialization_only_blocked_task_ensure(summary)
        {
            blocker_codes.push("internal_activation_view_only".to_string());
        }
        if let Some(blocker_code) = summary
            .blocker_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            blocker_codes.push(blocker_code.to_string());
        }
        if !recovery.is_some_and(|recovery| {
            downstream_summary_issues_are_superseded_by_ready_handoff(recovery, summary)
        }) {
            blocker_codes.extend(
                summary
                    .downstream_dispatch_blockers
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
            );
        }
        blocked_evidence_present |= halted_state_signal(&summary.dispatch_status)
            || halted_state_signal(&summary.lane_status)
            || summary
                .downstream_dispatch_status
                .as_deref()
                .is_some_and(halted_state_signal);
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_issue_codes(&blocker_codes, blocked_evidence_present)
}

fn dispatch_receipt_issue_codes(
    receipt: &RunGraphDispatchReceipt,
    blocked_evidence_present: &mut bool,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if crate::runtime_dispatch_receipt_helpers::dispatch_receipt_is_materialization_only_blocked_task_ensure(receipt)
    {
        blocker_codes.push("internal_activation_view_only".to_string());
    }
    if let Some(blocker_code) = receipt
        .blocker_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    blocker_codes.extend(
        receipt
            .downstream_dispatch_blockers
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    );
    *blocked_evidence_present |= halted_state_signal(&receipt.dispatch_status)
        || halted_state_signal(&receipt.lane_status)
        || receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(halted_state_signal);
    blocker_codes
}

fn projection_truth_issue_codes(projection_truth: &RunGraphProjectionTruth) -> Vec<String> {
    let mut blocked_evidence_present = projection_truth.stale_state_suspected;
    let mut blocker_codes = Vec::new();
    if projection_truth.stale_state_suspected
        && (projection_truth
            .projection_reason
            .contains("missing TaskFlow task stale run")
            || projection_truth
                .next_lawful_operator_action
                .as_deref()
                .is_some_and(|action| {
                    action.starts_with("vida lane retire")
                        && action.contains("missing TaskFlow task stale run")
                }))
    {
        blocker_codes.push("stale_missing_task_run_graph".to_string());
    }
    if projection_truth.stale_state_suspected
        && projection_truth
            .next_lawful_operator_action
            .as_deref()
            .is_some_and(|action| {
                action.trim() == closed_task_active_run_projection_mismatch_command()
            })
    {
        blocker_codes.push("closed_task_active_run_projection_mismatch".to_string());
    }
    if let Some(receipt) = projection_truth.dispatch_receipt.as_ref() {
        blocker_codes.extend(dispatch_receipt_issue_codes(
            receipt,
            &mut blocked_evidence_present,
        ));
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_issue_codes(&blocker_codes, blocked_evidence_present)
}

fn exception_takeover_receipt_is_behind_ready_handoff(
    active_node: &str,
    status: &str,
    recovery_ready: bool,
    resume_target: &str,
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && status == "ready"
        && recovery_ready
        && resume_target.starts_with("dispatch.")
        && active_node != receipt.dispatch_target
}

fn projection_truth_issue_codes_for_ready_handoff(
    active_node: &str,
    status: &str,
    recovery_ready: bool,
    resume_target: &str,
    projection_truth: &RunGraphProjectionTruth,
) -> Vec<String> {
    let task_authority_blockers = projection_truth_issue_codes(projection_truth);
    if task_authority_blockers.iter().any(|code| {
        matches!(
            code.as_str(),
            "closed_task_active_run_projection_mismatch" | "stale_missing_task_run_graph"
        )
    }) {
        return task_authority_blockers;
    }
    if projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            rework_receipt_issues_are_superseded_by_ready_handoff(
                active_node,
                status,
                recovery_ready,
                resume_target,
                receipt,
            )
        })
    {
        return Vec::new();
    }
    if projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            exception_takeover_receipt_is_behind_ready_handoff(
                active_node,
                status,
                recovery_ready,
                resume_target,
                receipt,
            )
        })
    {
        return normalize_run_graph_issue_codes(&[], projection_truth.stale_state_suspected);
    }
    if projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            downstream_receipt_issues_are_superseded_by_ready_handoff(
                active_node,
                status,
                recovery_ready,
                resume_target,
                receipt,
            )
        })
    {
        return normalize_run_graph_issue_codes(&[], projection_truth.stale_state_suspected);
    }
    if projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            status == "ready"
                && recovery_ready
                && resume_target.starts_with("dispatch.")
                && active_node == receipt.dispatch_target
                && matches!(receipt.dispatch_status.as_str(), "routed" | "packet_ready")
                && receipt.blocker_code.as_deref().is_none_or(str::is_empty)
        })
    {
        return Vec::new();
    }
    projection_truth_issue_codes(projection_truth)
}

fn downstream_receipt_issues_are_superseded_by_ready_handoff(
    active_node: &str,
    status: &str,
    recovery_ready: bool,
    resume_target: &str,
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    if status != "ready"
        || !recovery_ready
        || !resume_target.starts_with("dispatch.")
        || active_node != receipt.dispatch_target
        || receipt.dispatch_status != "executed"
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || matches!(
            receipt.lane_status.as_str(),
            "lane_blocked" | "lane_failed" | "lane_exception_recorded" | "lane_exception_takeover"
        )
    {
        return false;
    }
    if receipt
        .downstream_dispatch_blockers
        .iter()
        .map(|value| value.trim())
        .any(|value| !value.is_empty() && value == "missing_owned_write_scope")
    {
        return false;
    }
    let Some(downstream_target) = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let downstream_node = downstream_target.replace('-', "_");
    resume_target == format!("dispatch.{downstream_node}_lane")
}

fn rework_receipt_issues_are_superseded_by_ready_handoff(
    active_node: &str,
    status: &str,
    recovery_ready: bool,
    resume_target: &str,
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    let _ = (active_node, status, recovery_ready, resume_target, receipt);
    false
}

fn run_graph_state_surface_issue_codes(
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Vec<String> {
    if terminal_run_graph_state_resolved(status)
        && !projection_truth.dispatch_receipt.as_ref().is_some_and(|receipt| {
            crate::runtime_dispatch_receipt_helpers::dispatch_receipt_is_materialization_only_blocked_task_ensure(receipt)
        })
    {
        return Vec::new();
    }
    if active_exception_takeover_receipt_matches_snapshot(
        status,
        projection_truth.dispatch_receipt.as_ref(),
    ) {
        return Vec::new();
    }

    let mut blocked_evidence_present =
        halted_state_signal(&status.status) || halted_state_signal(&status.lifecycle_stage);
    let mut blocker_codes = projection_truth_issue_codes_for_ready_handoff(
        &status.active_node,
        &status.status,
        status.recovery_ready,
        &status.resume_target,
        projection_truth,
    );
    if let Some(receipt) = projection_truth
        .dispatch_receipt
        .as_ref()
        .filter(|receipt| {
            !rework_receipt_issues_are_superseded_by_ready_handoff(
                &status.active_node,
                &status.status,
                status.recovery_ready,
                &status.resume_target,
                receipt,
            )
        })
        .filter(|receipt| {
            !exception_takeover_receipt_is_behind_ready_handoff(
                &status.active_node,
                &status.status,
                status.recovery_ready,
                &status.resume_target,
                receipt,
            )
        })
        .filter(|receipt| {
            !downstream_receipt_issues_are_superseded_by_ready_handoff(
                &status.active_node,
                &status.status,
                status.recovery_ready,
                &status.resume_target,
                receipt,
            )
        })
    {
        blocker_codes.extend(dispatch_receipt_issue_codes(
            receipt,
            &mut blocked_evidence_present,
        ));
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_issue_codes(&blocker_codes, blocked_evidence_present)
}

fn downstream_summary_issues_are_superseded_by_ready_handoff(
    recovery: &crate::state_store::RunGraphRecoverySummary,
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> bool {
    if recovery.resume_status != "ready"
        || !recovery.recovery_ready
        || !recovery.resume_target.starts_with("dispatch.")
        || recovery.active_node != receipt.dispatch_target
        || receipt.dispatch_status != "executed"
        || receipt
            .blocker_code
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || matches!(
            receipt.lane_status.as_str(),
            "lane_blocked" | "lane_failed" | "lane_exception_recorded" | "lane_exception_takeover"
        )
    {
        return false;
    }
    let Some(downstream_target) = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let downstream_node = downstream_target.replace('-', "_");
    recovery.resume_target == format!("dispatch.{downstream_node}_lane")
}

fn projection_truth_from_state_surface(
    state_root: &std::path::Path,
    status: &RunGraphStatus,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    continuation_binding_source: Option<&str>,
) -> (RunGraphProjectionTruth, Vec<String>) {
    let status_surface_receipt = receipt.map(dispatch_receipt_from_state_surface);
    let status_surface_binding =
        continuation_binding_source.map(|binding_source| RunGraphContinuationBinding {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({}),
            primary_path: binding_source.to_string(),
            sequential_vs_parallel_posture: "unknown".to_string(),
            binding_source: "status_surface".to_string(),
            why_this_unit: String::new(),
            request_text: None,
            recorded_at: String::new(),
        });
    let status_surface_receipt_for_stale = receipt
        .as_ref()
        .map(|summary| dispatch_receipt_from_state_surface(summary));
    let stale_state_suspected =
        projection_stale_state_suspected(state_root, status_surface_receipt_for_stale.as_ref())
            || status_surface_receipt_for_stale
                .as_ref()
                .is_some_and(|receipt| {
                    stale_blocked_dispatch_receipt_mismatches_active_lane(status, receipt)
                });
    let projection_truth = RunGraphProjectionTruth {
        projection_source: if receipt.is_some() {
            "reconciled_run_graph_status".to_string()
        } else {
            "persisted_run_graph_status".to_string()
        },
        projection_reason: projection_reason_for_snapshot(
            status,
            status_surface_receipt.as_ref(),
            status_surface_binding.as_ref(),
        ),
        dispatch_receipt_present: receipt.is_some(),
        continuation_binding_present: continuation_binding_source.is_some(),
        projection_vs_receipt_parity: projection_vs_receipt_parity(
            status,
            status_surface_receipt.as_ref(),
        ),
        stale_state_suspected,
        next_lawful_operator_action: next_lawful_operator_action_for_projection(
            status,
            status_surface_receipt.as_ref(),
            None,
            false,
            false,
        ),
        dispatch_receipt: status_surface_receipt.clone(),
        continuation_binding: None,
    };
    let blocker_codes = if recovery.is_some_and(|summary| {
        recovery_projection_resolves_persisted_open_cycle(summary, &projection_truth)
    }) {
        projection_truth_issue_codes_for_ready_handoff(
            &status.active_node,
            &status.status,
            status.recovery_ready,
            &status.resume_target,
            &projection_truth,
        )
    } else {
        dispatch_issue_codes_from_state_surface(recovery, receipt)
    };
    (projection_truth, blocker_codes)
}

pub(crate) fn build_run_graph_dispatch_compact_summary(
    state_root: &std::path::Path,
    status: Option<&RunGraphStatus>,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    continuation_binding: Option<&serde_json::Value>,
    activation_vs_execution_evidence: Option<&serde_json::Value>,
) -> Option<RunGraphDispatchCompactSummary> {
    let status = status?;
    let continuation_binding_source =
        continuation_binding_source_from_state_surface(continuation_binding);
    let evidence = activation_vs_execution_evidence.or_else(|| {
        receipt.and_then(|summary| {
            if summary.activation_evidence.is_null() {
                None
            } else {
                Some(&summary.activation_evidence)
            }
        })
    });
    let (projection_truth, mut blocker_codes) = projection_truth_from_state_surface(
        state_root,
        status,
        recovery,
        receipt,
        continuation_binding_source.as_deref(),
    );
    let (mut recommended_command, mut recommended_surface) = if let Some(summary) = recovery {
        let (_codes, _why_not_now, _next_action, command, surface) =
            recovery_surface_contract(summary, &projection_truth);
        if recovery_projection_resolves_persisted_open_cycle(summary, &projection_truth)
            && blocker_codes.is_empty()
        {
            (None, None)
        } else {
            (
                command.or_else(|| projection_truth.next_lawful_operator_action.clone()),
                surface.or_else(|| {
                    projection_truth
                        .next_lawful_operator_action
                        .as_deref()
                        .map(recommended_surface_for_command)
                }),
            )
        }
    } else {
        (
            projection_truth.next_lawful_operator_action.clone(),
            projection_truth
                .next_lawful_operator_action
                .as_deref()
                .map(recommended_surface_for_command),
        )
    };
    if continuation_binding_exposes_closed_task_active_run_projection_mismatch(continuation_binding)
    {
        blocker_codes.push("closed_task_active_run_projection_mismatch".to_string());
        blocker_codes = normalize_run_graph_issue_codes(&blocker_codes, true);
        recommended_command = Some(closed_task_active_run_projection_mismatch_command());
        recommended_surface = Some("vida task reconcile-closed-runs".to_string());
    }
    let (recommended_command, recommended_surface) =
        sanitize_placeholder_continuation_bind_recommendation(
            Some(&status.run_id),
            recommended_command,
            recommended_surface,
        );
    Some(RunGraphDispatchCompactSummary {
        route_truth: route_truth_from_projection_truth(&projection_truth, evidence),
        downstream_dispatch_preview: downstream_dispatch_preview_from_run_snapshot(
            status,
            receipt,
            continuation_binding_source.as_deref(),
            evidence,
        ),
        blocker_codes,
        stale_state_suspected: projection_truth.stale_state_suspected,
        recommended_command,
        recommended_surface,
    })
}

const MAX_DISPATCH_RESULT_BYTES: u64 = 1024 * 1024;

fn safe_read_dispatch_result_json(
    state_root: &std::path::Path,
    result_path: &str,
) -> Option<serde_json::Value> {
    let trimmed = result_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = std::path::Path::new(trimmed);
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        state_root.join(candidate)
    };
    let Ok(state_root_canonical) = std::fs::canonicalize(state_root) else {
        return None;
    };
    let Ok(candidate_canonical) = std::fs::canonicalize(&candidate) else {
        return None;
    };
    if !candidate_canonical.starts_with(&state_root_canonical) {
        return None;
    }
    let Ok(metadata) = std::fs::metadata(&candidate_canonical) else {
        return None;
    };
    if !metadata.is_file() || metadata.len() > MAX_DISPATCH_RESULT_BYTES {
        return None;
    }
    crate::read_json_file_if_present(&candidate_canonical)
}

fn projection_stale_state_suspected(
    state_root: &std::path::Path,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> bool {
    let Some(receipt) = receipt else {
        return false;
    };
    if halted_external_dispatch_artifact_mismatched_as_internal_activation(receipt) {
        return true;
    }
    if receipt.dispatch_status != "executing" {
        return false;
    }
    let Some(result_path) = receipt.dispatch_result_path.as_deref() else {
        return false;
    };
    let Some(result) = safe_read_dispatch_result_json(state_root, result_path) else {
        return false;
    };
    if result["execution_state"].as_str() != Some("executing") {
        return false;
    }
    let Some(recorded_at) = result["recorded_at"].as_str() else {
        return false;
    };
    let Ok(recorded_at) = time::OffsetDateTime::parse(recorded_at, &Rfc3339) else {
        return false;
    };
    let stale_after_seconds = result["stale_after_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .unwrap_or(STALE_PROJECTION_DISPATCH_TIMEOUT_SECONDS);
    let age_seconds = (time::OffsetDateTime::now_utc() - recorded_at).whole_seconds();
    age_seconds > stale_after_seconds
}

fn stale_blocked_dispatch_receipt_mismatches_active_lane(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    if receipt.exception_path_receipt_id.is_some() || receipt.supersedes_receipt_id.is_some() {
        return false;
    }
    let receipt_blocked = receipt.dispatch_status == "blocked"
        || receipt.blocker_code.as_deref().is_some_and(|value| {
            matches!(
                value.trim(),
                "host_bridge_completion_result_blocked" | "host_bridge_completion_blocked"
            )
        });
    if !receipt_blocked {
        return false;
    }
    let active_node = status.active_node.trim();
    let dispatch_target = receipt.dispatch_target.trim();
    if active_node.is_empty() || dispatch_target.is_empty() {
        return false;
    }
    let lifecycle = status.lifecycle_stage.trim();
    let lifecycle_mismatch = !lifecycle.is_empty() && !lifecycle.starts_with(dispatch_target);
    let active_node_mismatch = active_node != dispatch_target;
    let historical_lane_label = receipt.lane_status.trim().ends_with("_lane");
    active_node_mismatch && (lifecycle_mismatch || historical_lane_label)
}

pub(crate) async fn run_graph_projection_truth(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<RunGraphProjectionTruth, StateStoreError> {
    let dispatch_receipt = store.run_graph_dispatch_receipt(&status.run_id).await?;
    let persisted_continuation_binding =
        store.run_graph_continuation_binding(&status.run_id).await?;
    let task_authority =
        crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(store, status)
            .await?;
    let continuation_binding = if task_authority.stale_for_active_projection() {
        None
    } else {
        effective_projection_continuation_binding(
            status,
            dispatch_receipt.as_ref(),
            persisted_continuation_binding,
        )
    };
    let terminal_consume_continue_run_id = if dispatch_receipt.as_ref().is_some_and(|receipt| {
        halted_external_dispatch_artifact_mismatched_as_internal_activation(receipt)
            || dispatch_receipt_has_clean_ready_downstream_handoff(receipt)
            || (dispatch_receipt_resolution_reason_class(receipt).is_some()
                && dispatch_receipt_can_use_terminal_continue_evidence(receipt)
                && status.recovery_ready
                && status.resume_target != "none")
    }) {
        crate::latest_terminal_consume_continue_snapshot_run_id(store.root())
            .ok()
            .flatten()
    } else {
        None
    };
    let stale_state_suspected = task_authority.stale_for_active_projection()
        || projection_stale_state_suspected(store.root(), dispatch_receipt.as_ref())
        || dispatch_receipt.as_ref().is_some_and(|receipt| {
            stale_blocked_dispatch_receipt_mismatches_active_lane(status, receipt)
        });
    Ok(RunGraphProjectionTruth {
        projection_source: if dispatch_receipt.is_some() {
            "reconciled_run_graph_status".to_string()
        } else {
            "persisted_run_graph_status".to_string()
        },
        projection_reason: if task_authority.task_missing() {
            "missing TaskFlow task stale run".to_string()
        } else {
            projection_reason_for_snapshot(
                status,
                dispatch_receipt.as_ref(),
                continuation_binding.as_ref(),
            )
        },
        dispatch_receipt_present: dispatch_receipt.is_some(),
        continuation_binding_present: continuation_binding.is_some(),
        projection_vs_receipt_parity: projection_vs_receipt_parity(
            status,
            dispatch_receipt.as_ref(),
        ),
        stale_state_suspected,
        next_lawful_operator_action: next_lawful_operator_action_for_projection(
            status,
            dispatch_receipt.as_ref(),
            terminal_consume_continue_run_id.as_deref(),
            task_authority.task_missing(),
            task_authority.task_closed_stale_run(),
        ),
        dispatch_receipt,
        continuation_binding,
    })
}

#[derive(Clone)]
struct CompiledRunGraphControl {
    implementation: serde_json::Value,
    verification: serde_json::Value,
    /// Configured TeamFlow entry identity used by executable run-graph state.
    entry_execution_node_id: String,
    validation_report_required_before_implementation: bool,
}

async fn compiled_run_graph_control(store: &StateStore) -> Result<CompiledRunGraphControl, String> {
    compiled_run_graph_control_with_persistence(store, true).await
}

async fn compiled_run_graph_control_with_persistence(
    store: &StateStore,
    persist_launcher_snapshot: bool,
) -> Result<CompiledRunGraphControl, String> {
    let snapshot = if persist_launcher_snapshot {
        read_or_sync_launcher_activation_snapshot(store).await?
    } else {
        crate::launcher_activation_snapshot::read_or_capture_launcher_activation_snapshot(store)
            .await?
    };
    compiled_run_graph_control_from_bundle(&snapshot.compiled_bundle, &snapshot.source)
}

fn compiled_run_graph_control_from_bundle(
    compiled_bundle: &serde_json::Value,
    activation_source: &str,
) -> Result<CompiledRunGraphControl, String> {
    let selection = RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: activation_source.to_string(),
        selection_mode: "compiled".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: String::new(),
        selected_role: "orchestrator".to_string(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "compiled".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: compiled_bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: "compiled_snapshot".to_string(),
    };
    let execution_plan =
        build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, &selection);
    let implementation = execution_plan["development_flow"]["implementation"].clone();
    let verification = execution_plan["development_flow"]["verification"].clone();
    let authority =
        crate::runtime_dispatch_state::require_team_flow_authority_for_selection(&selection)
            .map_err(|blocker| blocker.to_string())?;
    let entry_execution_node_id = authority.entry_node_id.trim().to_string();
    if entry_execution_node_id.is_empty() {
        return Err("team_flow_authority_entry_node_missing".to_string());
    }
    authority
        .resolve_target(None, &entry_execution_node_id)
        .map_err(|blocker| blocker.to_string())?;
    if implementation.is_null() {
        return Err(
            "run-graph control is unavailable in the compiled activation snapshot.".to_string(),
        );
    }

    Ok(CompiledRunGraphControl {
        implementation,
        verification,
        entry_execution_node_id,
        validation_report_required_before_implementation: selection.compiled_bundle
            ["autonomous_execution"]["validation_report_required_before_implementation"]
            .as_bool()
            .unwrap_or(false),
    })
}

fn json_raw_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

pub(crate) fn default_run_graph_state(
    task_id: &str,
    task_class: &str,
    route_task_class: &str,
) -> RunGraphStatus {
    let fields = taskflow_core::run_graph::model::default_run_graph_status_fields(
        task_id,
        task_class,
        route_task_class,
    );
    RunGraphStatus {
        run_id: fields.run_id,
        task_id: fields.task_id,
        task_class: fields.task_class,
        active_node: fields.active_node,
        next_node: fields.next_node,
        status: fields.status,
        route_task_class: fields.route_task_class,
        selected_backend: fields.selected_backend,
        lane_id: fields.lane_id,
        lifecycle_stage: fields.lifecycle_stage,
        policy_gate: fields.policy_gate,
        handoff_state: fields.handoff_state,
        context_state: fields.context_state,
        checkpoint_kind: fields.checkpoint_kind,
        resume_target: fields.resume_target,
        recovery_ready: fields.recovery_ready,
    }
}

pub(crate) fn default_run_graph_status(
    task_id: &str,
    task_class: &str,
    route_task_class: &str,
) -> RunGraphStatus {
    // compatibility_adapter: one-way alias while callers move to state naming.
    default_run_graph_state(task_id, task_class, route_task_class)
}

pub(crate) fn default_run_graph_recovery_summary(
    task_id: &str,
    run_id: &str,
) -> crate::state_store::RunGraphRecoverySummary {
    crate::state_store::RunGraphRecoverySummary {
        run_id: run_id.to_string(),
        task_id: task_id.to_string(),
        active_node: "implementation".to_string(),
        lifecycle_stage: "implementation_active".to_string(),
        resume_node: None,
        resume_status: "running".to_string(),
        checkpoint_kind: "execution_cursor".to_string(),
        resume_target: "dispatch.implementation_lane".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "awaiting_implementation".to_string(),
        recovery_ready: false,
        delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
            active_node: "implementation".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            delegated_cycle_open: false,
            delegated_cycle_state: "none".to_string(),
            local_exception_takeover_gate: "admissible_not_active".to_string(),
            blocker_code: None,
            reporting_pause_gate: "allowed".to_string(),
            continuation_signal: "continue_when_bound".to_string(),
        },
    }
}

async fn run_taskflow_run_graph_state(
    state_dir: &std::path::Path,
    run_id: &str,
    as_json: bool,
) -> ExitCode {
    match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
        Ok(store) => match store.run_graph_status_for_operator_selector(run_id).await {
            Ok(status) => {
                let projection_truth = match run_graph_projection_truth(&store, &status).await {
                    Ok(truth) => truth,
                    Err(error) => {
                        return emit_run_graph_status_error(
                            &state_dir,
                            run_id,
                            "projection_truth_unavailable",
                            &error.to_string(),
                            as_json,
                        );
                    }
                };
                let task_identity =
                    match store.run_graph_dispatch_task_identity(&status.run_id).await {
                        Ok(identity) => identity,
                        Err(error) => {
                            return emit_run_graph_status_error(
                                state_dir,
                                run_id,
                                "task_identity_unavailable",
                                &error.to_string(),
                                as_json,
                            );
                        }
                    };
                if as_json {
                    match build_run_graph_state_json_payload_with_task_identity_and_state_root(
                        "vida taskflow run-graph status",
                        &status,
                        &projection_truth,
                        task_identity.as_ref(),
                        Some(state_dir),
                    ) {
                        Ok(payload) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&payload)
                                    .expect("run-graph status should render as json")
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => emit_run_graph_status_error(
                            &state_dir,
                            run_id,
                            "payload_render_failed",
                            &error,
                            as_json,
                        ),
                    }
                } else {
                    let blocker_codes =
                        run_graph_state_surface_issue_codes(&status, &projection_truth);
                    let next_actions = run_graph_state_operator_next_actions(
                        &status,
                        &projection_truth,
                        &blocker_codes,
                    );
                    print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
                    print_surface_line(RenderMode::Plain, "run", &status.run_id);
                    print_surface_line(RenderMode::Plain, "status", &status.status);
                    print_surface_line(RenderMode::Plain, "status_detail", &status.as_display());
                    print_surface_line(
                        RenderMode::Plain,
                        "delegation gate",
                        &status.delegation_gate().as_display(),
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "projection",
                        &projection_truth.projection_reason,
                    );
                    if !blocker_codes.is_empty() {
                        print_surface_line(
                            RenderMode::Plain,
                            "blocker_codes",
                            &blocker_codes.join(", "),
                        );
                    }
                    if let Some(identity) = task_identity.as_ref() {
                        print_surface_line(
                            RenderMode::Plain,
                            "task_identity",
                            &run_graph_task_identity_compact(identity),
                        );
                    }
                    if next_actions.is_empty() {
                        if let Some(next_action) =
                            projection_truth.next_lawful_operator_action.as_deref()
                        {
                            print_surface_line(RenderMode::Plain, "next action", next_action);
                        }
                    } else {
                        for next_action in next_actions {
                            print_surface_line(RenderMode::Plain, "next action", &next_action);
                        }
                    }
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                if let Some(status) =
                    read_run_graph_state_json_from_dispatch_init_fast_cache(state_dir, run_id)
                {
                    if as_json {
                        let payload = serde_json::json!({
                            "surface": "vida taskflow run-graph status",
                            "run_id": run_id,
                            "status": "cache_backed_dispatch_init_status",
                            "latest_status": status,
                            "projection_truth": {
                                "projection_reason": "run-graph status restored from dispatch-init fast-cache because authoritative state was unavailable"
                            }
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&payload)
                                .expect("cache-backed run-graph status should render as json")
                        );
                        ExitCode::SUCCESS
                    } else {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
                        print_surface_line(RenderMode::Plain, "run", run_id);
                        print_surface_line(
                            RenderMode::Plain,
                            "status",
                            status["lifecycle_stage"].as_str().unwrap_or("cache_backed"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            "run-graph status restored from dispatch-init fast-cache because authoritative state was unavailable",
                        );
                        ExitCode::SUCCESS
                    }
                } else {
                    emit_run_graph_status_error(
                        &state_dir,
                        run_id,
                        "run_graph_status_unavailable",
                        &error.to_string(),
                        as_json,
                    )
                }
            }
        },
        Err(error) => emit_run_graph_status_error(
            state_dir,
            run_id,
            "state_store_unavailable",
            &error.to_string(),
            as_json,
        ),
    }
}

async fn run_taskflow_run_graph_latest(state_dir: &std::path::Path, as_json: bool) -> ExitCode {
    match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
        Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
            Ok(status) => {
                let projection_truth = match status.as_ref() {
                    Some(status) => match run_graph_projection_truth(&store, status).await {
                        Ok(truth) => Some(truth),
                        Err(error) => {
                            return emit_run_graph_latest_error(
                                state_dir,
                                "projection_truth_unavailable",
                                &error.to_string(),
                                as_json,
                            );
                        }
                    },
                    None => None,
                };
                match (status.as_ref(), projection_truth.as_ref(), as_json) {
                    (Some(status), Some(projection_truth), true) => {
                        match build_run_graph_state_json_payload(
                            "vida taskflow run-graph latest",
                            status,
                            projection_truth,
                        ) {
                            Ok(payload) => {
                                crate::print_json_pretty(&payload);
                                ExitCode::SUCCESS
                            }
                            Err(error) => emit_run_graph_latest_error(
                                state_dir,
                                "payload_render_failed",
                                &error,
                                as_json,
                            ),
                        }
                    }
                    (Some(status), Some(projection_truth), false) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if let Some(next_action) =
                            projection_truth.next_lawful_operator_action.as_deref()
                        {
                            print_surface_line(RenderMode::Plain, "next action", next_action);
                        }
                        ExitCode::SUCCESS
                    }
                    _ if as_json => {
                        crate::print_json_pretty(&serde_json::json!({
                            "surface": "vida taskflow run-graph latest",
                            "status": null,
                        }));
                        ExitCode::SUCCESS
                    }
                    _ => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "status", "none");
                        ExitCode::SUCCESS
                    }
                }
            }
            Err(error) => emit_run_graph_latest_error(
                state_dir,
                "run_graph_latest_unavailable",
                &error.to_string(),
                as_json,
            ),
        },
        Err(error) => {
            if StateStore::error_is_lock_contention(&error) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida taskflow run-graph latest",
                    state_dir,
                    RenderMode::Plain,
                    as_json,
                    &error.to_string(),
                );
            }
            emit_run_graph_latest_error(
                state_dir,
                "state_store_unavailable",
                &error.to_string(),
                as_json,
            )
        }
    }
}

async fn run_taskflow_run_graph_diagnose(
    state_dir: &std::path::Path,
    run_id: &str,
    surface: &'static str,
    as_json: bool,
) -> ExitCode {
    match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
        Ok(store) => match build_run_graph_diagnosis(&store, run_id).await {
            Ok(diagnosis) => {
                if as_json {
                    match build_run_graph_diagnosis_json_payload_for_surface_with_state_root(
                        surface,
                        &diagnosis,
                        Some(state_dir),
                    ) {
                        Ok(payload) => {
                            crate::print_json_pretty(&payload);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!(
                                "Failed to render normalized run-graph diagnose payload: {error}"
                            );
                            ExitCode::from(1)
                        }
                    }
                } else {
                    print_run_graph_diagnosis_plain(surface, &diagnosis, Some(state_dir));
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("Failed to diagnose run-graph dispatch state: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            if StateStore::error_is_lock_contention(&error) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    surface,
                    state_dir,
                    RenderMode::Plain,
                    as_json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_taskflow_run_graph_diagnose_latest(
    state_dir: &std::path::Path,
    as_json: bool,
) -> ExitCode {
    match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
        Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
            Ok(Some(status)) => {
                let run_id = status.run_id.clone();
                drop(store);
                run_taskflow_run_graph_diagnose(
                    state_dir,
                    &run_id,
                    "vida taskflow run-graph diagnose-latest",
                    as_json,
                )
                .await
            }
            Ok(None) if as_json => {
                crate::print_json_pretty(&serde_json::json!({
                    "surface": "vida taskflow run-graph diagnose-latest",
                    "status": null,
                }));
                ExitCode::SUCCESS
            }
            Ok(None) => {
                print_surface_header(RenderMode::Plain, "vida taskflow run-graph diagnose-latest");
                print_surface_line(RenderMode::Plain, "status", "none");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Failed to read latest run-graph status: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            if StateStore::error_is_lock_contention(&error) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida taskflow run-graph diagnose-latest",
                    state_dir,
                    RenderMode::Plain,
                    as_json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_taskflow_recovery_from_state_dir(
    state_dir: PathBuf,
    run_id: &str,
    as_json: bool,
) -> ExitCode {
    if as_json {
        let projection_name = recovery_projection_name(run_id);
        if let Some(cached) = read_recovery_projection(&state_dir, &projection_name, run_id) {
            let exit_code = serde_json::from_str::<serde_json::Value>(&cached)
                .ok()
                .as_ref()
                .map(exit_code_for_operator_payload)
                .unwrap_or(ExitCode::SUCCESS);
            println!("{cached}");
            return exit_code;
        }
        return match StateStore::open_existing_read_only(state_dir.clone()).await {
            Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                Ok(summary) => {
                    let projection_truth = match store.run_graph_status(&summary.run_id).await {
                        Ok(status) => match run_graph_projection_truth(&store, &status).await {
                            Ok(truth) => truth,
                            Err(error) => {
                                return emit_recovery_json_error(
                                    &state_dir,
                                    run_id,
                                    "recovery_projection_truth_unreadable",
                                    &error.to_string(),
                                );
                            }
                        },
                        Err(error) => {
                            return emit_recovery_json_error(
                                &state_dir,
                                run_id,
                                "run_graph_status_unreadable",
                                &error.to_string(),
                            );
                        }
                    };
                    let (
                        blocker_codes,
                        why_not_now,
                        next_action,
                        recommended_command,
                        recommended_surface,
                    ) = recovery_surface_contract_with_owned_scope(
                        &summary,
                        &projection_truth,
                        &recovery_owned_write_scope_for_summary(&store, &summary).await,
                    );
                    let task_identity = match store
                        .run_graph_dispatch_task_identity(&summary.run_id)
                        .await
                    {
                        Ok(identity) => identity,
                        Err(error) => {
                            return emit_recovery_json_error(
                                &state_dir,
                                run_id,
                                "run_graph_task_identity_unreadable",
                                &error.to_string(),
                            );
                        }
                    };
                    match build_recovery_json_payload_with_task_identity(
                        "vida taskflow recovery status",
                        &summary,
                        &projection_truth,
                        task_identity.as_ref(),
                        blocker_codes,
                        why_not_now,
                        next_action,
                        recommended_command,
                        recommended_surface,
                    ) {
                        Ok(payload) => {
                            let exit_code = exit_code_for_operator_payload(&payload);
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&payload)
                                    .expect("recovery summary should render as json")
                            );
                            crate::operator_projection_cache::write_json_projection(
                                &state_dir,
                                &projection_name,
                                &payload,
                            );
                            exit_code
                        }
                        Err(error) => emit_recovery_json_error(
                            &state_dir,
                            run_id,
                            "recovery_status_payload_render_failed",
                            &error,
                        ),
                    }
                }
                Err(error) => emit_recovery_json_error(
                    &state_dir,
                    run_id,
                    "run_graph_recovery_unreadable",
                    &error.to_string(),
                ),
            },
            Err(error) => emit_recovery_json_error(
                &state_dir,
                run_id,
                "state_store_unreadable",
                &error.to_string(),
            ),
        };
    }

    match StateStore::open_existing_read_only(state_dir).await {
        Ok(store) => match store.run_graph_recovery_summary(run_id).await {
            Ok(summary) => {
                let projection_truth = match store.run_graph_status(&summary.run_id).await {
                    Ok(status) => match run_graph_projection_truth(&store, &status).await {
                        Ok(truth) => truth,
                        Err(error) => {
                            eprintln!("Failed to build recovery projection truth: {error}");
                            return ExitCode::from(1);
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to read run-graph status for projection truth: {error}");
                        return ExitCode::from(1);
                    }
                };
                let (
                    blocker_codes,
                    why_not_now,
                    next_action,
                    recommended_command,
                    recommended_surface,
                ) = recovery_surface_contract_with_owned_scope(
                    &summary,
                    &projection_truth,
                    &recovery_owned_write_scope_for_summary(&store, &summary).await,
                );
                let task_identity = match store
                    .run_graph_dispatch_task_identity(&summary.run_id)
                    .await
                {
                    Ok(identity) => identity,
                    Err(error) => {
                        eprintln!("Failed to read run-graph task identity: {error}");
                        return ExitCode::from(1);
                    }
                };
                print_surface_header(RenderMode::Plain, "vida taskflow recovery status");
                print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                print_surface_line(
                    RenderMode::Plain,
                    "projection",
                    &projection_truth.projection_reason,
                );
                if let Some(identity) = task_identity.as_ref() {
                    print_surface_line(
                        RenderMode::Plain,
                        "task_identity",
                        &run_graph_task_identity_compact(identity),
                    );
                }
                if !blocker_codes.is_empty() {
                    print_surface_line(
                        RenderMode::Plain,
                        "blocker_codes",
                        &blocker_codes.join(", "),
                    );
                }
                if let Some(summary) = why_not_now.as_ref().map(|value| value.summary.as_str()) {
                    print_surface_line(RenderMode::Plain, "why_not_now", summary);
                }
                if let Some(next_action) = next_action.as_ref() {
                    print_surface_line(RenderMode::Plain, "next action", &next_action.reason);
                }
                if let Some(command) = recommended_command.as_deref() {
                    print_surface_line(RenderMode::Plain, "recommended_command", command);
                }
                if let Some(surface) = recommended_surface.as_deref() {
                    print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                }
                if blocker_codes.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(error) => {
                eprintln!("Failed to read recovery status: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_taskflow_recovery(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "recovery" => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "recovery" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "recovery" && subcommand == "gate-latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery gate-latest",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery gate-latest",
                        );
                        print_surface_line(RenderMode::Plain, "gate", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "gate-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate-latest",
                                "gate": summary,
                            }))
                            .expect("latest gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "gate" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery gate");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "gate" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate",
                                "run_id": summary.run_id,
                                "gate": summary,
                            }))
                            .expect("gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "checkpoint-latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint-latest",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint-latest",
                        );
                        print_surface_line(RenderMode::Plain, "checkpoint", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "checkpoint-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint-latest",
                                "checkpoint": summary,
                            }))
                            .expect("latest checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "checkpoint" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow recovery checkpoint",
                        );
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "checkpoint" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint",
                                "run_id": summary.run_id,
                                "checkpoint": summary,
                            }))
                            .expect("checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        let payload = recovery_json_error_payload(
                            "vida taskflow recovery checkpoint",
                            run_id,
                            &state_dir,
                            "run_graph_checkpoint_unreadable",
                            &error.to_string(),
                        );
                        crate::print_json_pretty(&payload);
                        exit_code_for_operator_payload(&payload)
                    }
                },
                Err(error) => {
                    let payload = recovery_json_error_payload(
                        "vida taskflow recovery checkpoint",
                        run_id,
                        &state_dir,
                        "state_store_unreadable",
                        &error.to_string(),
                    );
                    crate::print_json_pretty(&payload);
                    exit_code_for_operator_payload(&payload)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match latest_recovery_summary_for_operator_surface(&store).await {
                    Ok(Some(summary)) => {
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!("Failed to build recovery projection truth: {error}");
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for projection truth: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract_with_owned_scope(
                            &summary,
                            &projection_truth,
                            &recovery_owned_write_scope_for_summary(&store, &summary).await,
                        );
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if !blocker_codes.is_empty() {
                            print_surface_line(
                                RenderMode::Plain,
                                "blocker_codes",
                                &blocker_codes.join(", "),
                            );
                        }
                        if let Some(summary) =
                            why_not_now.as_ref().map(|value| value.summary.as_str())
                        {
                            print_surface_line(RenderMode::Plain, "why_not_now", summary);
                        }
                        if let Some(next_action) = next_action.as_ref() {
                            print_surface_line(
                                RenderMode::Plain,
                                "next action",
                                &next_action.reason,
                            );
                        }
                        if let Some(command) = recommended_command.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_command", command);
                        }
                        if let Some(surface) = recommended_surface.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                        }
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "recovery", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "latest" && flag == "--json" =>
        {
            render_latest_recovery_json_payload("vida taskflow recovery latest").await
        }
        [head, subcommand, flag]
            if head == "recovery" && subcommand == "status" && flag == "--json" =>
        {
            render_latest_recovery_json_payload("vida taskflow recovery status").await
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "explain" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!(
                                        "Failed to build recovery explain projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for recovery explain: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract_with_owned_scope(
                            &summary,
                            &projection_truth,
                            &recovery_owned_write_scope_for_summary(&store, &summary).await,
                        );
                        let diagnosis_type = categorize_recovery_diagnosis(
                            &blocker_codes,
                            &summary,
                            &projection_truth,
                        );
                        let diagnosis_summary = next_action
                            .as_ref()
                            .map(|value| value.reason.as_str())
                            .or_else(|| why_not_now.as_ref().map(|value| value.summary.as_str()))
                            .unwrap_or("No recovery blocker is currently actionable.");
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery explain");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "diagnosis", &diagnosis_type);
                        print_surface_line(
                            RenderMode::Plain,
                            "diagnosis_summary",
                            diagnosis_summary,
                        );
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "evidence",
                            &projection_truth.projection_reason,
                        );
                        if let Some(next_action) = next_action.as_ref() {
                            print_surface_line(
                                RenderMode::Plain,
                                "next_action",
                                &next_action.reason,
                            );
                        }
                        if let Some(command) = recommended_command.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_command", command);
                        }
                        if let Some(surface) = recommended_surface.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                        }
                        if !blocker_codes.is_empty() {
                            print_surface_line(
                                RenderMode::Plain,
                                "blocker_codes",
                                &blocker_codes.join(", "),
                            );
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to explain recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "explain" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        let projection_truth = match store.run_graph_status(&summary.run_id).await {
                            Ok(status) => match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!(
                                        "Failed to build recovery explain projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status for recovery explain: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let (
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) = recovery_surface_contract_with_owned_scope(
                            &summary,
                            &projection_truth,
                            &recovery_owned_write_scope_for_summary(&store, &summary).await,
                        );
                        match build_recovery_explain_json_payload(
                            "vida taskflow recovery explain",
                            &summary,
                            &projection_truth,
                            blocker_codes,
                            why_not_now,
                            next_action,
                            recommended_command,
                            recommended_surface,
                        ) {
                            Ok(payload) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&payload)
                                        .expect("recovery explain should render as json")
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized recovery explain payload: {error}"
                                );
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to explain recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "recovery"
                && subcommand == "status"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            eprintln!(
                "Usage: vida taskflow recovery status <run-id> [--state-dir <path>] [--json]\n\nField/view/detail selection:\n  Recovery surfaces use fixed diagnostic projections.\n  Use default output for compact operator text or --json for full machine-readable detail.\n  Recovery does not expose ad-hoc --fields, --view, or --details selectors.\n\nOutput:\n  default              Emit compact TOON operator output.\n  --json               Emit machine-readable JSON output."
            );
            ExitCode::SUCCESS
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "status" => {
            run_taskflow_recovery_from_state_dir(proxy_state_dir(), run_id, false).await
        }
        [head, subcommand, run_id, state_flag, state_value]
            if head == "recovery" && subcommand == "status" && state_flag == "--state-dir" =>
        {
            run_taskflow_recovery_from_state_dir(PathBuf::from(state_value), run_id, false).await
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "status" && flag == "--json" =>
        {
            run_taskflow_recovery_from_state_dir(proxy_state_dir(), run_id, true).await
        }
        [head, subcommand, run_id, state_flag, state_value, flag]
            if head == "recovery"
                && subcommand == "status"
                && state_flag == "--state-dir"
                && flag == "--json" =>
        {
            run_taskflow_recovery_from_state_dir(PathBuf::from(state_value), run_id, true).await
        }
        [head, subcommand, run_id, flag, state_flag, state_value]
            if head == "recovery"
                && subcommand == "status"
                && flag == "--json"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_recovery_from_state_dir(PathBuf::from(state_value), run_id, true).await
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate-latest" => {
            eprintln!("Usage: vida taskflow recovery gate-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate" => {
            eprintln!("Usage: vida taskflow recovery gate <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint-latest" => {
            eprintln!("Usage: vida taskflow recovery checkpoint-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint" => {
            eprintln!("Usage: vida taskflow recovery checkpoint <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow recovery latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "explain" => {
            eprintln!("Usage: vida taskflow recovery explain <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "status" => {
            eprintln!(
                "Usage: vida taskflow recovery status <run-id> [--state-dir <path>] [--json]\nRecovery status is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

pub(crate) async fn run_taskflow_run_graph(args: &[String]) -> ExitCode {
    match args {
        [head, flag] if head == "run-graph" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("run-graph"));
            ExitCode::SUCCESS
        }
        [head, subcommand, tail @ ..] if head == "run-graph" && subcommand == "task-identity" => {
            run_taskflow_run_graph_task_identity(tail).await
        }
        [head, subcommand, flag]
            if head == "run-graph"
                && subcommand == "status"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            eprintln!(
                "Usage: vida taskflow run-graph status <run-id> [--state-dir <path>] [--json]\nRun-graph status is a fixed diagnostic projection; use --json for machine-readable detail.\nIt does not expose ad-hoc --fields, --view, or --details selectors."
            );
            ExitCode::SUCCESS
        }
        [head, subcommand, flag]
            if head == "run-graph"
                && subcommand == "latest"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            eprintln!("Usage: vida taskflow run-graph latest [--state-dir <path>] [--json]");
            ExitCode::SUCCESS
        }
        [head, subcommand, flag]
            if head == "run-graph"
                && subcommand == "diagnose-latest"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            eprintln!(
                "Usage: vida taskflow run-graph diagnose-latest [--state-dir <path>] [--json]\nRun-graph diagnose is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::SUCCESS
        }
        [head, subcommand, flag]
            if head == "run-graph"
                && subcommand == "diagnose"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            eprintln!(
                "Usage: vida taskflow run-graph diagnose <run-id> [--state-dir <path>] [--json]\nRun-graph diagnose is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::SUCCESS
        }
        [head, subcommand, flag]
            if head == "run-graph" && subcommand == "status" && flag == "--json" =>
        {
            print_run_graph_missing_run_id_json();
            ExitCode::from(2)
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            if let Some(payload) =
                read_run_graph_dispatch_init_fast_cache_for_dispatch_init(&state_dir, run_id).await
            {
                crate::print_json_pretty(&payload);
                return ExitCode::SUCCESS;
            }
            run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, true).await
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            let state_dir = proxy_state_dir();
            if let Some(payload) =
                read_run_graph_dispatch_init_fast_cache_for_dispatch_init(&state_dir, run_id).await
            {
                print_surface_header(RenderMode::Plain, "vida taskflow run-graph dispatch-init");
                print_surface_line(RenderMode::Plain, "run", run_id);
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_packet",
                    payload["dispatch_packet_path"].as_str().unwrap_or("none"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_target",
                    payload["dispatch_receipt"]["dispatch_target"]
                        .as_str()
                        .unwrap_or("none"),
                );
                return ExitCode::SUCCESS;
            }
            run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, false).await
        }
        [head, subcommand] if head == "run-graph" && subcommand == "diagnose-latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
                    Ok(Some(status)) => {
                        match build_run_graph_diagnosis(&store, &status.run_id).await {
                            Ok(diagnosis) => {
                                print_run_graph_diagnosis_plain(
                                    "vida taskflow run-graph diagnose-latest",
                                    &diagnosis,
                                    Some(&state_dir),
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to diagnose latest run-graph dispatch state: {error}"
                                );
                                ExitCode::from(1)
                            }
                        }
                    }
                    Ok(None) => {
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow run-graph diagnose-latest",
                        );
                        print_surface_line(RenderMode::Plain, "status", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    if StateStore::error_is_lock_contention(&error) {
                        return crate::status_surface::emit_degraded_read_lock_surface(
                            "vida taskflow run-graph diagnose-latest",
                            &state_dir,
                            RenderMode::Plain,
                            false,
                            &error.to_string(),
                        );
                    }
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, state_flag, state_value]
            if head == "run-graph"
                && subcommand == "diagnose-latest"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_diagnose_latest(&PathBuf::from(state_value), false).await
        }
        [head, subcommand, state_flag, state_value, flag]
            if head == "run-graph"
                && subcommand == "diagnose-latest"
                && state_flag == "--state-dir"
                && flag == "--json" =>
        {
            run_taskflow_run_graph_diagnose_latest(&PathBuf::from(state_value), true).await
        }
        [head, subcommand, flag, state_flag, state_value]
            if head == "run-graph"
                && subcommand == "diagnose-latest"
                && flag == "--json"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_diagnose_latest(&PathBuf::from(state_value), true).await
        }
        [head, subcommand] if head == "run-graph" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
                    Ok(Some(status)) => {
                        let projection_truth =
                            match run_graph_projection_truth(&store, &status).await {
                                Ok(truth) => truth,
                                Err(error) => {
                                    eprintln!(
                                        "Failed to build latest run-graph projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "projection",
                            &projection_truth.projection_reason,
                        );
                        if let Some(next_action) =
                            projection_truth.next_lawful_operator_action.as_deref()
                        {
                            print_surface_line(RenderMode::Plain, "next action", next_action);
                        }
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "status", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    if StateStore::error_is_lock_contention(&error) {
                        return crate::status_surface::emit_degraded_read_lock_surface(
                            "vida taskflow run-graph latest",
                            &state_dir,
                            RenderMode::Plain,
                            false,
                            &error.to_string(),
                        );
                    }
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, state_flag, state_value]
            if head == "run-graph" && subcommand == "latest" && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_latest(&PathBuf::from(state_value), false).await
        }
        [head, subcommand, state_flag, state_value, flag]
            if head == "run-graph"
                && subcommand == "latest"
                && state_flag == "--state-dir"
                && flag == "--json" =>
        {
            run_taskflow_run_graph_latest(&PathBuf::from(state_value), true).await
        }
        [head, subcommand, flag, state_flag, state_value]
            if head == "run-graph"
                && subcommand == "latest"
                && flag == "--json"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_latest(&PathBuf::from(state_value), true).await
        }
        [head, subcommand, flag]
            if head == "run-graph" && subcommand == "diagnose-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
                    Ok(Some(status)) => {
                        match build_run_graph_diagnosis(&store, &status.run_id).await {
                            Ok(diagnosis) => {
                                match build_run_graph_diagnosis_json_payload_for_surface_with_state_root(
                                    "vida taskflow run-graph diagnose-latest",
                                    &diagnosis,
                                    Some(&state_dir),
                                ) {
                                    Ok(payload) => {
                                        println!(
                                            "{}",
                                            serde_json::to_string_pretty(&payload).expect(
                                                "run-graph diagnose-latest should render as json"
                                            )
                                        );
                                        ExitCode::SUCCESS
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to render normalized run-graph diagnose payload: {error}"
                                        );
                                        ExitCode::from(1)
                                    }
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to diagnose latest run-graph dispatch state: {error}"
                                );
                                ExitCode::from(1)
                            }
                        }
                    }
                    Ok(None) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph diagnose-latest",
                                "status": null,
                            }))
                            .expect("run-graph diagnose-latest should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    if StateStore::error_is_lock_contention(&error) {
                        return crate::status_surface::emit_degraded_read_lock_surface(
                            "vida taskflow run-graph diagnose-latest",
                            &state_dir,
                            RenderMode::Plain,
                            true,
                            &error.to_string(),
                        );
                    }
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "run-graph" && subcommand == "latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match latest_run_graph_status_for_operator_surface(&store).await {
                    Ok(status) => {
                        let projection_truth = match status.as_ref() {
                            Some(status) => {
                                match run_graph_projection_truth(&store, status).await {
                                    Ok(truth) => Some(truth),
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to build latest run-graph projection truth: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            None => None,
                        };
                        match (status.as_ref(), projection_truth.as_ref()) {
                            (Some(status), Some(projection_truth)) => {
                                match build_run_graph_state_json_payload(
                                    "vida taskflow run-graph latest",
                                    status,
                                    projection_truth,
                                ) {
                                    Ok(payload) => {
                                        println!(
                                            "{}",
                                            serde_json::to_string_pretty(&payload).expect(
                                                "latest run-graph status should render as json"
                                            )
                                        );
                                        ExitCode::SUCCESS
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to render normalized latest run-graph payload: {error}"
                                        );
                                        ExitCode::from(1)
                                    }
                                }
                            }
                            _ => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&serde_json::json!({
                                        "surface": "vida taskflow run-graph latest",
                                        "status": null,
                                    }))
                                    .expect("latest run-graph status should render as json")
                                );
                                ExitCode::SUCCESS
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    if StateStore::error_is_lock_contention(&error) {
                        return crate::status_surface::emit_degraded_read_lock_surface(
                            "vida taskflow run-graph latest",
                            &state_dir,
                            RenderMode::Plain,
                            true,
                            &error.to_string(),
                        );
                    }
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "status" => {
            run_taskflow_run_graph_state(&proxy_state_dir(), run_id, false).await
        }
        [head, subcommand, run_id, state_flag, state_value]
            if head == "run-graph" && subcommand == "status" && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_state(&PathBuf::from(state_value), run_id, false).await
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "diagnose" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match build_run_graph_diagnosis(&store, run_id).await {
                    Ok(diagnosis) => {
                        print_run_graph_diagnosis_plain(
                            "vida taskflow run-graph diagnose",
                            &diagnosis,
                            Some(&state_dir),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to diagnose run-graph dispatch state: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, state_flag, state_value]
            if head == "run-graph" && subcommand == "diagnose" && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_diagnose(
                &PathBuf::from(state_value),
                run_id,
                "vida taskflow run-graph diagnose",
                false,
            )
            .await
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "status" && flag == "--json" =>
        {
            run_taskflow_run_graph_state(&proxy_state_dir(), run_id, true).await
        }
        [head, subcommand, run_id, state_flag, state_value, flag]
            if head == "run-graph"
                && subcommand == "status"
                && state_flag == "--state-dir"
                && flag == "--json" =>
        {
            run_taskflow_run_graph_state(&PathBuf::from(state_value), run_id, true).await
        }
        [head, subcommand, run_id, flag, state_flag, state_value]
            if head == "run-graph"
                && subcommand == "status"
                && flag == "--json"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_state(&PathBuf::from(state_value), run_id, true).await
        }
        [head, subcommand, run_id, state_flag, state_value, flag]
            if head == "run-graph"
                && subcommand == "diagnose"
                && state_flag == "--state-dir"
                && flag == "--json" =>
        {
            run_taskflow_run_graph_diagnose(
                &PathBuf::from(state_value),
                run_id,
                "vida taskflow run-graph diagnose",
                true,
            )
            .await
        }
        [head, subcommand, run_id, flag, state_flag, state_value]
            if head == "run-graph"
                && subcommand == "diagnose"
                && flag == "--json"
                && state_flag == "--state-dir" =>
        {
            run_taskflow_run_graph_diagnose(
                &PathBuf::from(state_value),
                run_id,
                "vida taskflow run-graph diagnose",
                true,
            )
            .await
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "diagnose" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match build_run_graph_diagnosis(&store, run_id).await {
                    Ok(diagnosis) => {
                        match build_run_graph_diagnosis_json_payload_for_surface_with_state_root(
                            "vida taskflow run-graph diagnose",
                            &diagnosis,
                            Some(&state_dir),
                        ) {
                            Ok(payload) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&payload)
                                        .expect("run-graph diagnose should render as json")
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized run-graph diagnose payload: {error}"
                                );
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to diagnose run-graph dispatch state: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow run-graph latest [--state-dir <path>] [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "diagnose-latest" => {
            eprintln!(
                "Usage: vida taskflow run-graph diagnose-latest [--state-dir <path>] [--json]\nRun-graph diagnose is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "diagnose" => {
            eprintln!(
                "Usage: vida taskflow run-graph diagnose <run-id> [--state-dir <path>] [--json]\nRun-graph diagnose is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "status" => {
            eprintln!(
                "Usage: vida taskflow run-graph status <run-id> [--state-dir <path>] [--json]\nRun-graph status is a fixed diagnostic projection; use --json for machine-readable detail."
            );
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

fn print_run_graph_missing_run_id_json() {
    let payload = run_graph_missing_run_id_json_payload();
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .expect("missing run id payload should render as json")
    );
}

fn run_graph_missing_run_id_json_payload() -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow run-graph status",
        "status": "blocked",
        "blocker_codes": ["missing_run_id"],
        "error": "Missing required <run-id> for `vida taskflow run-graph status`.",
        "next_actions": [
            format!(
                "Run `{}` with the concrete run id.",
                operator_output::command_text::human_command(
                    "vida taskflow run-graph status <run-id> --json"
                )
            ),
            format!(
                "Use `{}` to inspect the latest run when the run id is unknown.",
                operator_output::command_text::human_command("vida taskflow run-graph latest --json")
            )
        ],
    })
}

fn print_run_graph_json_error(
    surface: &str,
    run_id: &str,
    error: &str,
    evidence: Option<serde_json::Value>,
) {
    let mut payload = serde_json::json!({
        "surface": surface,
        "run_id": run_id,
        "error": error,
    });
    if let Some(evidence) = evidence {
        if evidence.get("surface").is_some() {
            let mut evidence = evidence;
            evidence["error"] = serde_json::Value::String(error.to_string());
            println!(
                "{}",
                serde_json::to_string_pretty(&evidence)
                    .expect("run-graph error should render as json")
            );
            return;
        }
        payload["incident"] = evidence["incident"].clone();
        payload["blockers"] = evidence["blockers"].clone();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("run-graph error should render as json")
    );
}

fn run_graph_dispatch_init_timeout_message(run_id: &str, stage: &str) -> String {
    format!(
        "Timed out preparing run-graph dispatch-init for `{run_id}` after {RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS}s during `{stage}`; dispatch-init may have written a packet before timeout, but the surface failed closed instead of holding the state-store lock."
    )
}

fn set_dispatch_init_timeout_stage(
    stage_slot: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    stage: &'static str,
) {
    if let Ok(trace_path) = std::env::var("VIDA_DISPATCH_INIT_STAGE_TRACE") {
        if !trace_path.trim().is_empty() {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(trace_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "{stage}");
            }
        }
    }
    if let Some(stage_slot) = stage_slot {
        *stage_slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = stage;
    }
}

fn run_graph_dispatch_init_error_evidence(error: &str) -> Option<serde_json::Value> {
    if error.starts_with("Timed out preparing run-graph dispatch-init") {
        return Some(serde_json::json!({
            "incident": {
                "status": "blocked",
                "summary": "run-graph dispatch-init timed out before returning a bounded dispatch receipt"
            },
            "blockers": [RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER]
        }));
    }
    if error.contains("recovery_ready is false") {
        let run_id = run_graph_resume_gate_error_run_id(error)?;
        let command =
            machine_json_command(format!("vida lane show {} --json", shell_quote(&run_id)));
        let next_action = RecoveryNextAction {
            command: command.clone(),
            surface: recommended_surface_for_command(&command),
            reason: "inspect the lane envelope for the dispatch blocker and follow its recommended recovery command".to_string(),
        };
        let next_actions = vec![
            next_action.reason.clone(),
            format!(
                "Inspect recovery details with `{}`.",
                operator_output::command_text::human_command(&format!(
                    "vida taskflow recovery status {} --json",
                    shell_quote(&run_id)
                ))
            ),
        ];
        return build_run_graph_operator_surface_payload(
            "vida taskflow run-graph dispatch-init",
            &run_id,
            vec![fallback_dispatch_issue_code()],
            next_actions,
            serde_json::json!({
                "error": error,
                "blocker_code": "run_graph_recovery_not_ready",
                "next_action": next_action,
                "recommended_command": command,
                "recommended_surface": recommended_surface_for_command(&command),
                "incident": {
                    "status": "blocked",
                    "summary": "run-graph dispatch-init resume gate denied because recovery_ready is false",
                    "run_id": run_id,
                },
            }),
        )
        .ok();
    }
    None
}

fn run_graph_resume_gate_error_run_id(error: &str) -> Option<String> {
    let marker = "Run-graph resume gate denied for `";
    let start = error.find(marker)? + marker.len();
    let rest = &error[start..];
    let end = rest.find('`')?;
    let run_id = rest[..end].trim();
    (!run_id.is_empty()).then(|| run_id.to_string())
}

async fn record_dispatch_init_timeout_issue(
    store: &StateStore,
    run_id: &str,
) -> Result<bool, String> {
    if matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_))) {
        return Ok(false);
    }
    let mut status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(false),
    };
    if status.status == "completed" {
        return Ok(false);
    }

    status.next_node = None;
    status.status = "blocked".to_string();
    status.lifecycle_stage = "dispatch_init_timeout".to_string();
    status.policy_gate = RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER.to_string();
    status.handoff_state = "blocked_dispatch_init_timeout".to_string();
    status.context_state = "blocked".to_string();
    status.checkpoint_kind = "dispatch_init_timeout".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    record_run_graph_state_with_continuation_sync(
        store,
        &status,
        "run_graph_dispatch_init_timeout",
    )
    .await?;
    Ok(true)
}

async fn record_dispatch_init_timeout_issue_bounded(
    store: &StateStore,
    run_id: &str,
) -> Result<bool, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        record_dispatch_init_timeout_issue(store, run_id),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Ok(false),
    }
}

async fn record_dispatch_init_timeout_issue_from_state_dir_bounded(
    state_dir: &std::path::Path,
    run_id: &str,
) -> Result<bool, String> {
    match tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let store = StateStore::open_existing(state_dir.to_path_buf())
            .await
            .map_err(|error| {
                format!("Failed to open state store for dispatch-init timeout blocker: {error}")
            })?;
        let recorded = record_dispatch_init_timeout_issue(&store, run_id).await?;
        store.close().await;
        Ok::<bool, String>(recorded)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Ok(false),
    }
}

fn run_graph_issue_code(status: &str) -> Option<&'static str> {
    match status {
        "denied" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewDenied,
        )),
        "expired" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewExpired,
        )),
        "review_findings" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewFindings,
        )),
        "changed_scope" => Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ImplementationReviewChangedScope,
        )),
        _ => None,
    }
}

struct RunGraphBlockerEvidenceArgs<'a> {
    run_id: &'a str,
    active_node: &'a str,
    status: &'a str,
    route_task_class: &'a str,
    policy_gate: &'a str,
    resume_target: &'a str,
    next_node: Option<&'a str>,
    error: &'a str,
}

fn run_graph_issue_evidence(
    args: RunGraphBlockerEvidenceArgs<'_>,
) -> Result<Option<serde_json::Value>, String> {
    let is_blocked_advance = args.error.starts_with("run-graph advance blocked:");
    if !is_blocked_advance {
        return Ok(None);
    }
    let is_active_exception_takeover = args.error.contains("active exception takeover");
    let blocker_code = if is_active_exception_takeover {
        crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::OpenDelegatedCycle,
        )
    } else {
        run_graph_issue_code(args.status).ok_or_else(|| {
            format!(
                "run-graph advance blocked without explicit blocker evidence for `{}` status `{}`; refusing to continue (fail-closed)",
                args.run_id, args.status
            )
        })?
    };
    let canonical_blocker_codes =
        canonical_release1_blocker_code_entries(&serde_json::json!([blocker_code])).ok_or_else(
            || {
                format!(
            "run-graph blocker code `{blocker_code}` is not canonical (must be lowercase/digits/_)"
        )
            },
        )?;
    let canonical_blocker_code = canonical_blocker_codes
        .first()
        .expect("canonical block list always non-empty")
        .clone();
    Ok(Some(serde_json::json!({
        "incident": {
            "code": "run_graph_advance_blocked",
            "run_id": args.run_id,
            "active_node": args.active_node,
            "status": args.status,
            "route_task_class": args.route_task_class,
        },
        "blockers": [{
            "code": canonical_blocker_code,
            "policy_gate": args.policy_gate,
            "resume_target": args.resume_target,
            "next_node": args.next_node,
            "source": "run_graph_state",
            "evidence_kind": if is_active_exception_takeover {
                "active_exception_takeover"
            } else {
                "run_graph_status_blocker"
            },
        }]
    })))
}

pub(crate) fn is_dispatch_resume_handoff_done(status: &RunGraphStatus) -> bool {
    if !status.resume_target.starts_with("dispatch.") {
        return true;
    }
    status.next_node.is_some()
        && !status.policy_gate.trim().is_empty()
        && status.policy_gate != "none"
        && !status.handoff_state.trim().is_empty()
        && status.handoff_state != "none"
}

fn is_receipt_backed_materialized_dispatch_ready(status: &RunGraphStatus) -> bool {
    let Some(next_node) = status.next_node.as_deref().map(str::trim) else {
        return false;
    };
    if next_node.is_empty() || next_node == "none" || next_node == "unknown" {
        return false;
    }
    let same_lane_materialized = status.active_node == next_node
        && status.lane_id == format!("{next_node}_lane")
        && status.policy_gate == "not_required"
        && status.resume_target == format!("dispatch.{next_node}_lane");
    let direct_rework_materialized = !status.active_node.trim().is_empty()
        && status.active_node != "planning"
        && !matches!(status.policy_gate.as_str(), "" | "none" | "not_required")
        && status.resume_target == format!("dispatch.{next_node}");
    status.status == "ready"
        && (same_lane_materialized || direct_rework_materialized)
        && status.lifecycle_stage == format!("{next_node}_dispatch_ready")
        && status.handoff_state == format!("awaiting_{next_node}")
        && status.context_state == "sealed"
        && status.checkpoint_kind == "execution_cursor"
        && status.recovery_ready
}

pub(crate) fn validate_run_graph_resume_gate(status: &RunGraphStatus) -> Result<(), String> {
    if !status.recovery_ready {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: recovery_ready is false",
            status.run_id
        ));
    }
    if status.resume_target == "none" || !status.resume_target.starts_with("dispatch.") {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: resume_target `{}` is not a dispatch target",
            status.run_id, status.resume_target
        ));
    }
    ensure_resume_target_handoff_consistency(status).map_err(|error| {
        format!(
            "Run-graph resume gate denied for `{}`: {error}",
            status.run_id
        )
    })?;
    if !is_dispatch_resume_handoff_done(status) {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
            status.run_id,
            status.resume_target,
            status.next_node.as_deref().unwrap_or("none"),
            status.policy_gate,
            status.handoff_state
        ));
    }
    if !status.delegation_gate().delegated_cycle_open
        && !is_receipt_backed_materialized_dispatch_ready(status)
        && !is_seeded_dispatch_ready(status)
    {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: delegated cycle is not open",
            status.run_id
        ));
    }
    Ok(())
}
fn resume_dispatch_node(resume_target: &str) -> Option<&str> {
    let resume_target = resume_target.trim();
    let stripped = resume_target.strip_prefix("dispatch.")?;
    let node = stripped.strip_suffix("_lane").unwrap_or(stripped);
    if node.is_empty() {
        return None;
    }
    Some(node)
}

fn ensure_resume_target_handoff_consistency(status: &RunGraphStatus) -> Result<(), String> {
    if let Some(node) = resume_dispatch_node(&status.resume_target) {
        let expected_handoff = format!("awaiting_{node}");
        if status.handoff_state != expected_handoff {
            return Err(format!(
                "run-graph resume metadata inconsistent for `{}`: resume_target `{}` requires handoff_state `{}`, not `{}`",
                status.run_id, status.resume_target, expected_handoff, status.handoff_state
            ));
        }
        if status.next_node.as_deref() != Some(node) {
            return Err(format!(
                "run-graph resume metadata inconsistent for `{}`: resume_target `{}` requires next_node `{}`",
                status.run_id, status.resume_target, node
            ));
        }
    } else if status.handoff_state.starts_with("awaiting_") {
        return Err(format!(
            "run-graph resume metadata inconsistent for `{}`: handoff_state `{}` requires a dispatch.* resume_target",
            status.run_id, status.handoff_state
        ));
    }
    Ok(())
}

fn canonicalize_resume_meta(status: &mut RunGraphStatus) {
    if let Some(node) = resume_dispatch_node(&status.resume_target) {
        status.next_node = Some(node.to_string());
        status.handoff_state = format!("awaiting_{node}");
    } else {
        status.next_node = None;
        status.handoff_state = "none".to_string();
    }
}

fn dispatch_replay_node_from_receipt(receipt: &RunGraphDispatchReceipt) -> Option<String> {
    receipt
        .downstream_dispatch_active_target
        .as_deref()
        .or(receipt.downstream_dispatch_last_target.as_deref())
        .or(Some(receipt.dispatch_target.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "none" && *value != "unknown")
        .map(|value| value.strip_suffix("_lane").unwrap_or(value).to_string())
}

pub(crate) fn state_with_active_exception_dispatch_replay(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceipt,
) -> Option<RunGraphStatus> {
    if !active_exception_takeover_receipt_matches_snapshot(status, Some(receipt)) {
        return None;
    }
    let node = dispatch_replay_node_from_receipt(receipt)?;
    let (handoff_state, resume_target) = run_graph_handoff(Some(&node), DispatchTargetFormat::Lane);
    let mut replay = status.clone();
    replay.next_node = Some(node);
    replay.handoff_state = handoff_state;
    replay.resume_target = resume_target;
    replay.recovery_ready = true;
    if replay.policy_gate.trim().is_empty() || replay.policy_gate == "none" {
        replay.policy_gate = "exception_takeover_dispatch_replay".to_string();
    }
    Some(replay)
}

async fn reconcile_dispatch_init_state_for_active_exception(
    store: &StateStore,
    status: RunGraphStatus,
) -> Result<RunGraphStatus, String> {
    if status.recovery_ready && status.resume_target.starts_with("dispatch.") {
        return Ok(status);
    }
    let receipt = store
        .run_graph_dispatch_receipt(&status.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read run-graph dispatch receipt for `{}` while reconciling dispatch-init recovery: {error}",
                status.run_id
            )
        })?;
    Ok(receipt
        .as_ref()
        .and_then(|receipt| state_with_active_exception_dispatch_replay(&status, receipt))
        .unwrap_or(status))
}

fn reconcile_dispatch_init_state_for_missing_receipt(
    mut status: RunGraphStatus,
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_receipt_present: bool,
) -> RunGraphStatus {
    if dispatch_receipt_present
        || !status.recovery_ready
        || status.resume_target.starts_with("dispatch.")
        || status.next_node.is_some()
        || status.active_node == "planning"
        || !status.delegation_gate().delegated_cycle_open
    {
        return status;
    }
    let target_format = if role_selection.conversational_mode.is_some() {
        DispatchTargetFormat::Direct
    } else {
        DispatchTargetFormat::Lane
    };
    let node = status.active_node.clone();
    let (handoff_state, resume_target) = run_graph_handoff(Some(&node), target_format);
    status.next_node = Some(node);
    status.handoff_state = handoff_state;
    status.resume_target = resume_target;
    status
}

fn meta_string_field(meta: &serde_json::Value, key: &str) -> Option<Option<String>> {
    meta.get(key)?;
    Some(
        meta.get(key)
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
    )
}

pub(crate) fn merge_run_graph_meta(
    mut status: RunGraphStatus,
    meta: &serde_json::Value,
) -> RunGraphStatus {
    if let Some(selected_backend) = meta
        .get("selected_backend")
        .and_then(|value| value.as_str())
    {
        status.selected_backend = selected_backend.to_string();
    }
    if let Some(lane_id) = meta.get("lane_id").and_then(|value| value.as_str()) {
        status.lane_id = lane_id.to_string();
    }
    if let Some(lifecycle_stage) = meta.get("lifecycle_stage").and_then(|value| value.as_str()) {
        status.lifecycle_stage = lifecycle_stage.to_string();
    }
    if let Some(policy_gate) = meta.get("policy_gate").and_then(|value| value.as_str()) {
        status.policy_gate = policy_gate.to_string();
    }
    let resume_meta = meta_string_field(meta, "resume_target");
    if let Some(context_state) = meta.get("context_state").and_then(|value| value.as_str()) {
        status.context_state = context_state.to_string();
    }
    if let Some(checkpoint_kind) = meta.get("checkpoint_kind").and_then(|value| value.as_str()) {
        status.checkpoint_kind = checkpoint_kind.to_string();
    }
    if let Some(resume_field) = resume_meta {
        status.resume_target = resume_field.unwrap_or_else(|| "none".to_string());
        canonicalize_resume_meta(&mut status);
    } else {
        if let Some(next_node_field) = meta_string_field(meta, "next_node") {
            status.next_node = next_node_field;
        }
        if let Some(handoff_field) = meta_string_field(meta, "handoff_state") {
            status.handoff_state = handoff_field.unwrap_or_else(|| "none".to_string());
        }
    }
    status.recovery_ready =
        json_bool_field(meta, "recovery_ready").unwrap_or(status.recovery_ready);
    status
}

async fn record_run_graph_state_with_continuation_sync(
    store: &StateStore,
    status: &RunGraphStatus,
    binding_source: &str,
) -> Result<(), String> {
    store
        .record_run_graph_status(status)
        .await
        .map_err(|error| format!("Failed to update run-graph state: {error}"))?;
    let reconciled = store
        .run_graph_status(&status.run_id)
        .await
        .map_err(|error| {
            format!("Failed to read reconciled run-graph state after update: {error}")
        })?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &reconciled,
        binding_source,
    )
    .await
    .map_err(|error| format!("Failed to synchronize continuation binding: {error}"))?;
    Ok(())
}

fn run_graph_state_from_authority_ready_transition(
    existing: &RunGraphStatus,
    active_node: String,
    next_node: Option<String>,
    lane_id: String,
    lifecycle_stage: String,
    policy_gate: String,
    checkpoint_kind: String,
    target_format: DispatchTargetFormat,
    recovery_ready: bool,
) -> RunGraphStatus {
    let transition = ready_run_graph_transition(ReadyRunGraphTransitionInput {
        run_id: existing.run_id.clone(),
        task_id: existing.task_id.clone(),
        task_class: existing.task_class.clone(),
        active_node,
        next_node,
        route_task_class: existing.route_task_class.clone(),
        selected_backend: existing.selected_backend.clone(),
        lane_id,
        lifecycle_stage,
        policy_gate,
        checkpoint_kind,
        target_format,
        recovery_ready,
    });
    RunGraphStatus {
        run_id: transition.run_id,
        task_id: transition.task_id,
        task_class: transition.task_class,
        active_node: transition.active_node,
        next_node: transition.next_node,
        status: transition.status,
        route_task_class: transition.route_task_class,
        selected_backend: transition.selected_backend,
        lane_id: transition.lane_id,
        lifecycle_stage: transition.lifecycle_stage,
        policy_gate: transition.policy_gate,
        handoff_state: transition.handoff_state,
        context_state: transition.context_state,
        checkpoint_kind: transition.checkpoint_kind,
        resume_target: transition.resume_target,
        recovery_ready: transition.recovery_ready,
    }
}

fn implementation_analysis_gate(
    implementation: &serde_json::Value,
) -> (Option<String>, String, bool) {
    let writer_node = implementation_writer_node(implementation);
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    let next_node = Some(writer_node);
    let policy_gate = if coach_required {
        json_raw_string_field(implementation, "verification_gate")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "not_required".to_string())
    } else {
        "not_required".to_string()
    };
    let recovery_ready = next_node.is_some()
        || coach_required
        || json_bool_field(implementation, "independent_verification_required").unwrap_or(false);
    (next_node, policy_gate, recovery_ready)
}

fn implementation_writer_node(implementation: &serde_json::Value) -> String {
    json_raw_string_field(implementation, "writer_route_task_class")
        .or_else(|| json_raw_string_field(implementation, "implementer_route_task_class"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "writer".to_string())
}

async fn seeded_implementation_lane_sequence(
    store: &StateStore,
    run_id: &str,
) -> Result<Option<Vec<crate::team_flow_authority_adapter::TeamFlowNodeResolution>>, String> {
    seeded_implementation_lane_sequence_with_persistence(store, run_id, true).await
}

async fn seeded_implementation_lane_sequence_with_persistence(
    store: &StateStore,
    run_id: &str,
    persist_launcher_snapshot: bool,
) -> Result<Option<Vec<crate::team_flow_authority_adapter::TeamFlowNodeResolution>>, String> {
    let context = store
        .run_graph_dispatch_context(run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "run_graph_dispatch_context_missing".to_string())?;
    let selection = rehydrate_dispatch_context_role_selection_with_persistence(
        store,
        &context,
        persist_launcher_snapshot,
    )
    .await?;
    let mut sequence = crate::runtime_dispatch_state::typed_lane_node_sequence(&selection, true)?;
    let selected_node = selection.execution_plan["development_flow"]["implementation"]
        .get("team_flow_selected_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(selected_node) = selected_node {
        let selected_index = sequence
            .iter()
            .position(|node| node.node_id == selected_node)
            .ok_or_else(|| format!("team_flow_selected_node_id_unknown:{selected_node}"))?;
        sequence.drain(..selected_index);
    }
    Ok(Some(sequence))
}

fn ensure_configured_lane_advance_allowed(
    existing: &RunGraphStatus,
    transition_kind: &str,
) -> Result<(), String> {
    if existing.task_class != "implementation" || existing.route_task_class != "implementation" {
        return Err(format!(
            "run-graph advance blocked: configured execution lane `{}` belongs to task_class=`{}` route_task_class=`{}`; configured lane advancement is limited to implementation runs",
            existing.active_node, existing.task_class, existing.route_task_class
        ));
    }

    if existing.lifecycle_stage.contains("blocked")
        || existing.lifecycle_stage.ends_with("_blocked")
        || existing.status == "blocked"
        || existing.status == "failed"
    {
        return Err(format!(
            "run-graph advance blocked: configured execution lane `{}` is not advanceable while status=`{}` lifecycle_stage=`{}`",
            existing.active_node, existing.status, existing.lifecycle_stage
        ));
    }

    match transition_kind {
        "dispatch_ready" => {
            if existing.status != "ready" {
                return Err(format!(
                    "run-graph advance blocked: configured dispatch-ready lane `{}` requires status=`ready`, got `{}`",
                    existing.active_node, existing.status
                ));
            }
            if existing.policy_gate != "not_required" {
                return Err(format!(
                    "run-graph advance blocked: configured dispatch-ready lane `{}` still requires policy_gate=`{}`",
                    existing.active_node, existing.policy_gate
                ));
            }
        }
        "handoff" | "complete" => {
            if !matches!(
                existing.status.as_str(),
                "clean" | "completed" | "completed_success" | "completed-success"
            ) {
                return Err(format!(
                    "run-graph advance blocked: configured execution lane `{}` requires completed lane evidence before `{transition_kind}`, got status=`{}`",
                    existing.active_node, existing.status
                ));
            }
            if existing.policy_gate.trim().is_empty() || existing.policy_gate == "none" {
                return Err(format!(
                    "run-graph advance blocked: configured execution lane `{}` has invalid policy_gate=`{}`",
                    existing.active_node, existing.policy_gate
                ));
            }
        }
        _ => {
            return Err(format!(
                "run-graph advance blocked: unknown configured lane transition `{transition_kind}` for `{}`",
                existing.active_node
            ));
        }
    }

    Ok(())
}

fn next_seeded_implementation_lane(
    sequence: &[crate::team_flow_authority_adapter::TeamFlowNodeResolution],
    current_node: &str,
) -> Option<String> {
    let current_index = sequence
        .iter()
        .position(|node| node.node_id == current_node.trim())?;
    sequence
        .get(current_index + 1)
        .map(|node| node.node_id.clone())
}

pub(crate) fn is_seeded_dispatch_ready(status: &RunGraphStatus) -> bool {
    let Some(next_node) = status.next_node.as_deref().map(str::trim) else {
        return false;
    };
    if next_node.is_empty() || next_node == "none" || next_node == "unknown" {
        return false;
    }
    status.active_node == "planning"
        && status.status == "ready"
        && status.lane_id == format!("{next_node}_lane")
        && status.lifecycle_stage == format!("{next_node}_dispatch_ready")
        && status.policy_gate == "not_required"
        && status.handoff_state == format!("awaiting_{next_node}")
        && status.context_state == "ready"
        && status.checkpoint_kind == "execution_cursor"
        && status.resume_target == format!("dispatch.{next_node}")
        && status.recovery_ready
}

fn is_seeded_implementation_dispatch_ready(status: &RunGraphStatus) -> bool {
    is_seeded_dispatch_ready(status)
        && status.task_class == "implementation"
        && status.route_task_class == "implementation"
}

fn implementation_verification_gate(
    implementation: &serde_json::Value,
    verification: &serde_json::Value,
) -> (Option<String>, String) {
    let verification_route = json_raw_string_field(implementation, "verification_route_task_class")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "verification".to_string());
    let next_node = json_bool_field(implementation, "independent_verification_required")
        .unwrap_or(false)
        .then_some(verification_route);
    let policy_gate = if next_node.is_some() {
        json_raw_string_field(verification, "verification_gate")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification_summary".to_string())
    } else {
        "not_required".to_string()
    };
    (next_node, policy_gate)
}

fn implementation_writer_handoff(
    implementation: &serde_json::Value,
    verification: &serde_json::Value,
) -> (String, Option<String>, String, DispatchTargetFormat, bool) {
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    if coach_required {
        let coach_node = json_raw_string_field(implementation, "coach_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "coach".to_string());
        let (next_node, policy_gate) =
            implementation_verification_gate(implementation, verification);
        return (
            coach_node,
            next_node,
            policy_gate,
            DispatchTargetFormat::Direct,
            true,
        );
    }

    let verification_required =
        json_bool_field(implementation, "independent_verification_required").unwrap_or(false);
    if verification_required {
        let verification_node =
            json_raw_string_field(implementation, "verification_route_task_class")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "verification".to_string());
        return (
            verification_node,
            None,
            json_raw_string_field(verification, "verification_gate")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "verification_summary".to_string()),
            DispatchTargetFormat::Lane,
            false,
        );
    }

    (
        implementation_writer_node(implementation),
        None,
        "not_required".to_string(),
        DispatchTargetFormat::Lane,
        false,
    )
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ImplementationVerificationOutcome {
    ReworkReady,
    Clean,
    Approved,
    FindingsBlocked,
    UnexpectedStatus,
}

fn implementation_verification_outcome(status: &str) -> ImplementationVerificationOutcome {
    const OUTCOME_TABLE: &[(&str, ImplementationVerificationOutcome)] = &[
        (
            "rework_ready",
            ImplementationVerificationOutcome::ReworkReady,
        ),
        ("clean", ImplementationVerificationOutcome::Clean),
        (
            crate::release1_contracts::ApprovalStatus::Approved.as_str(),
            ImplementationVerificationOutcome::Approved,
        ),
        (
            crate::release1_contracts::ApprovalStatus::Denied.as_str(),
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            crate::release1_contracts::ApprovalStatus::Expired.as_str(),
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "review_findings",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "blocker",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "blocked",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        (
            "rework_required",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
        ("rework", ImplementationVerificationOutcome::FindingsBlocked),
        (
            "changed_scope",
            ImplementationVerificationOutcome::FindingsBlocked,
        ),
    ];

    OUTCOME_TABLE
        .iter()
        .find_map(|(key, outcome)| (*key == status).then_some(*outcome))
        .unwrap_or(ImplementationVerificationOutcome::UnexpectedStatus)
}

fn inferred_design_doc_path_for_task(task_id: &str) -> Option<String> {
    let slug = task_id
        .trim()
        .strip_prefix("feature-")
        .unwrap_or(task_id.trim());
    if slug.is_empty() {
        return None;
    }
    Some(format!("docs/product/spec/{slug}-design.md"))
}

fn design_doc_has_ready_markers(path: &std::path::Path) -> Option<bool> {
    let contents = std::fs::read_to_string(path).ok()?;
    let has_status_marker = contents.contains("Status:");
    let has_bounded_file_set = contents.contains("## Bounded File Set");
    Some(has_status_marker || has_bounded_file_set)
}

fn design_doc_has_bounded_file_set(path: &std::path::Path) -> Option<bool> {
    let contents = std::fs::read_to_string(path).ok()?;
    Some(contents.contains("## Bounded File Set"))
}

fn registered_design_doc_path_for_task(task_id: &str) -> Option<String> {
    let task_slug = task_id
        .trim()
        .strip_prefix("feature-")
        .unwrap_or(task_id.trim())
        .trim();
    if task_slug.is_empty() {
        return None;
    }

    let spec_root = std::path::Path::new("docs/product/spec");
    let entries = std::fs::read_dir(spec_root).ok()?;
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let candidate_slug = file_name.strip_suffix("-design.md")?;
            if candidate_slug.is_empty()
                || (!task_slug.contains(candidate_slug) && !candidate_slug.contains(task_slug))
            {
                return None;
            }
            if !design_doc_has_ready_markers(&path)? {
                return None;
            }
            Some((candidate_slug.len(), path.to_string_lossy().to_string()))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, path)| path)
}

async fn existing_design_backed_task_design_doc_path(
    _store: &StateStore,
    task_id: &str,
) -> Option<String> {
    let inferred = inferred_design_doc_path_for_task(task_id);
    let design_doc_path = inferred
        .as_deref()
        .and_then(|path| {
            let design_doc = std::path::Path::new(path);
            design_doc.is_file().then_some(path.to_string())
        })
        .or_else(|| registered_design_doc_path_for_task(task_id))?;
    let design_doc = std::path::Path::new(&design_doc_path);
    if !design_doc.is_file() {
        return None;
    }
    design_doc_has_ready_markers(design_doc)?.then_some(design_doc_path)
}

fn inject_tracked_design_doc_path(execution_plan: &mut serde_json::Value, design_doc_path: &str) {
    let Some(plan) = execution_plan.as_object_mut() else {
        return;
    };
    let tracked_flow_bootstrap = plan
        .entry("tracked_flow_bootstrap".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if tracked_flow_bootstrap.is_null() {
        *tracked_flow_bootstrap = serde_json::json!({});
    }
    let Some(tracked_flow_bootstrap) = tracked_flow_bootstrap.as_object_mut() else {
        return;
    };
    tracked_flow_bootstrap.insert(
        "design_doc_path".to_string(),
        serde_json::Value::String(design_doc_path.to_string()),
    );
}

fn inject_task_planner_metadata(
    selection: &mut RuntimeConsumptionLaneSelection,
    planner_metadata: &crate::state_store::TaskPlannerMetadata,
) {
    if planner_metadata.owned_paths.is_empty()
        && planner_metadata.acceptance_targets.is_empty()
        && planner_metadata.proof_targets.is_empty()
        && planner_metadata.risk.is_none()
        && planner_metadata.estimate.is_none()
        && planner_metadata.lane_hint.is_none()
    {
        return;
    }
    if !planner_metadata.owned_paths.is_empty() {
        let owned_clause = format!("Owned paths: {}.", planner_metadata.owned_paths.join(", "));
        if !selection.request.contains(&owned_clause) {
            if selection.request.trim().is_empty() {
                selection.request = owned_clause;
            } else {
                selection.request = format!("{}\n\n{owned_clause}", selection.request.trim());
            }
        }
    }
    let Some(plan) = selection.execution_plan.as_object_mut() else {
        return;
    };
    let tracked_flow_bootstrap = plan
        .entry("tracked_flow_bootstrap".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if tracked_flow_bootstrap.is_null() {
        *tracked_flow_bootstrap = serde_json::json!({});
    }
    let Some(tracked_flow_bootstrap) = tracked_flow_bootstrap.as_object_mut() else {
        return;
    };
    let dev_task = tracked_flow_bootstrap
        .entry("dev_task".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if dev_task.is_null() {
        *dev_task = serde_json::json!({});
    }
    let Some(dev_task) = dev_task.as_object_mut() else {
        return;
    };
    dev_task.insert(
        "planner_metadata".to_string(),
        serde_json::to_value(planner_metadata).unwrap_or_else(|_| serde_json::json!({})),
    );
}

async fn try_existing_design_backed_implementation_override(
    store: &StateStore,
    task_id: &str,
    request_text: &str,
    selection: &mut RuntimeConsumptionLaneSelection,
) -> Result<(), String> {
    let normalized_request = request_text.to_lowercase();
    let implementation_terms =
        crate::runtime_lane_summary::explicit_implementation_request_terms(&normalized_request);
    let bounded_repair_terms =
        crate::runtime_lane_summary::explicit_bounded_code_repair_terms(&normalized_request);
    let design_doc_path = existing_design_backed_task_design_doc_path(store, task_id).await;
    let design_doc_has_bounded_scope = design_doc_path
        .as_ref()
        .map(|path| design_doc_has_bounded_file_set(std::path::Path::new(path)).unwrap_or(false))
        .unwrap_or(false);

    let request_has_explicit_owned_scope =
        !crate::runtime_dispatch_packets::explicit_request_scope_paths(request_text).is_empty();
    if (!implementation_terms.is_empty() || !bounded_repair_terms.is_empty())
        && (design_doc_has_bounded_scope || request_has_explicit_owned_scope)
    {
        let matched_terms = if !implementation_terms.is_empty() {
            implementation_terms
        } else {
            bounded_repair_terms
        };
        selection.selected_role = "worker".to_string();
        selection.conversational_mode = None;
        selection.tracked_flow_entry = Some("dev-pack".to_string());
        selection.allow_freeform_chat = false;
        selection.matched_terms = matched_terms.clone();
        selection.confidence = if matched_terms.len() >= 3 {
            "high".to_string()
        } else {
            "medium".to_string()
        };
        selection.reason = "auto_explicit_implementation_request".to_string();
        selection.execution_plan =
            build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, selection);
        if let Some(ref path) = design_doc_path {
            inject_tracked_design_doc_path(&mut selection.execution_plan, path);
        }
        return Ok(());
    }
    if let Ok(task) = store.show_task(task_id).await {
        if task.issue_type == "defect" || !task.planner_metadata.owned_paths.is_empty() {
            let matched_terms = if task.issue_type == "defect" {
                vec!["task_issue_type_defect".to_string()]
            } else {
                vec!["task_planner_owned_paths".to_string()]
            };
            selection.selected_role = "worker".to_string();
            selection.conversational_mode = None;
            selection.tracked_flow_entry = Some("dev-pack".to_string());
            selection.allow_freeform_chat = false;
            selection.matched_terms = matched_terms;
            selection.confidence = "medium".to_string();
            selection.reason = "auto_task_metadata_bounded_implementation_request".to_string();
            selection.execution_plan =
                build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, selection);
            inject_task_planner_metadata(selection, &task.planner_metadata);
            if let Some(ref path) = design_doc_path {
                inject_tracked_design_doc_path(&mut selection.execution_plan, path);
            }
            return Ok(());
        }
    }

    let Some(design_doc_path) = design_doc_path else {
        return Ok(());
    };
    let already_explicit_implementation = selection.conversational_mode.is_none()
        && selection.selected_role == "worker"
        && selection
            .reason
            .starts_with("auto_explicit_implementation_request");

    if design_doc_has_bounded_scope && already_explicit_implementation {
        selection.execution_plan =
            build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, selection);
        inject_tracked_design_doc_path(&mut selection.execution_plan, &design_doc_path);
        return Ok(());
    }
    Ok(())
}

fn selection_is_bounded_implementation_seed(selection: &RuntimeConsumptionLaneSelection) -> bool {
    selection.conversational_mode.is_none()
        && selection.selected_role == "worker"
        && selection.tracked_flow_entry.as_deref() == Some("dev-pack")
        && matches!(
            selection.reason.as_str(),
            "auto_explicit_implementation_request"
                | "auto_task_metadata_bounded_implementation_request"
        )
}

fn request_has_explicit_implementation_or_repair_terms(request_text: &str) -> bool {
    let normalized_request = request_text.to_lowercase();
    !crate::runtime_lane_summary::explicit_implementation_request_terms(&normalized_request)
        .is_empty()
        || !crate::runtime_lane_summary::explicit_bounded_code_repair_terms(&normalized_request)
            .is_empty()
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct RunGraphPacketBackedExecutionGate {
    pub(crate) status: String,
    pub(crate) supported: bool,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) selected_task_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) dispatch_packet_path: Option<String>,
    pub(crate) request_text_present: bool,
}

impl RunGraphPacketBackedExecutionGate {
    fn not_ready(
        status: &str,
        selected_task_id: Option<&str>,
        run_id: Option<&str>,
        blocker_codes: Vec<&str>,
    ) -> Self {
        let blocker_codes = blocker_codes
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            status: status.to_string(),
            supported: false,
            blocker_codes,
            next_actions: vec![
                "Verify selected task, run-graph status, dispatch context, explicit continuation binding, request text, and dispatch packet lineage before enabling packet-backed execute."
                    .to_string(),
            ],
            selected_task_id: selected_task_id.map(str::to_string),
            run_id: run_id.map(str::to_string),
            dispatch_packet_path: None,
            request_text_present: false,
        }
    }

    fn ready(selected_task_id: &str, run_id: &str, dispatch_packet_path: &str) -> Self {
        Self {
            status: "packet_ready".to_string(),
            supported: true,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
            selected_task_id: Some(selected_task_id.to_string()),
            run_id: Some(run_id.to_string()),
            dispatch_packet_path: Some(dispatch_packet_path.to_string()),
            request_text_present: true,
        }
    }
}

fn run_graph_packet_gate_binding_task_id(binding: &RunGraphContinuationBinding) -> Option<&str> {
    binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn terminal_done_without_next_unit_for_packet_gate(status: &RunGraphStatus) -> bool {
    status.status == "completed"
        && status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
}

pub(crate) fn evaluate_run_graph_packet_backed_execution_gate(
    selected_task_id: Option<&str>,
    status: Option<&RunGraphStatus>,
    context: Option<&RunGraphDispatchContext>,
    binding: Option<&RunGraphContinuationBinding>,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> RunGraphPacketBackedExecutionGate {
    let Some(selected_task_id) = selected_task_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_task_run_mapping_mismatch",
            None,
            None,
            vec!["scheduler_packet_execute_requires_single_selected_task"],
        );
    };
    let Some(status) = status else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            None,
            vec!["missing_run_graph_status"],
        );
    };
    if status.run_id.trim().is_empty()
        || status.task_id.trim().is_empty()
        || status.task_id != selected_task_id
    {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_task_run_mapping_mismatch",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["blocked_task_run_mapping_mismatch"],
        );
    }
    if terminal_done_without_next_unit_for_packet_gate(status) {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["terminal_run_graph_status_without_next_unit"],
        );
    }

    let Some(context) = context else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["missing_run_graph_dispatch_context"],
        );
    };
    if context.run_id != status.run_id
        || context.task_id != selected_task_id
        || context.request_text.trim().is_empty()
        || !context.role_selection.is_object()
    {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["invalid_run_graph_dispatch_context"],
        );
    }

    let Some(binding) = binding else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["missing_run_graph_continuation_binding"],
        );
    };
    if binding.run_id != status.run_id
        || binding.task_id != selected_task_id
        || binding.status != "bound"
        || binding.binding_source != "explicit_continuation_bind_task"
        || binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str)
            != Some("task_graph_task")
        || run_graph_packet_gate_binding_task_id(binding) != Some(selected_task_id)
        || binding.why_this_unit.trim().is_empty()
        || binding.sequential_vs_parallel_posture.trim().is_empty()
    {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_task_run_mapping_mismatch",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["blocked_task_run_mapping_mismatch"],
        );
    }

    let Some(receipt) = receipt else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["missing_run_graph_dispatch_receipt"],
        );
    };
    let Some(dispatch_packet_path) = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["missing_dispatch_packet_path"],
        );
    };
    if receipt.run_id != status.run_id
        || receipt
            .blocker_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
    {
        return RunGraphPacketBackedExecutionGate::not_ready(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["invalid_run_graph_dispatch_receipt"],
        );
    }

    RunGraphPacketBackedExecutionGate::ready(
        selected_task_id,
        status.run_id.as_str(),
        dispatch_packet_path,
    )
}

pub(crate) fn approval_delegation_transition_kind(status: &RunGraphStatus) -> Option<&'static str> {
    let route_bound_implementation =
        status.task_class == "implementation" && status.route_task_class == "implementation";

    if route_bound_implementation
        && status.status == "awaiting_approval"
        && status.lifecycle_stage == "approval_wait"
        && status.policy_gate
            == crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str()
        && matches!(status.next_node.as_deref(), Some("approval"))
        && status.handoff_state == "awaiting_approval"
        && status.resume_target == "dispatch.approval"
    {
        return Some("approval_wait");
    }

    if route_bound_implementation
        && status.status == "completed"
        && status.lifecycle_stage == "implementation_complete"
        && status.policy_gate == "not_required"
        && status.next_node.is_none()
        && status.handoff_state == "none"
        && status.resume_target == "none"
    {
        return Some("approval_complete");
    }

    None
}

pub(crate) fn implementation_lane_allows_terminal_completion(active_node: &str) -> bool {
    matches!(
        active_node,
        "implementer" | "verification" | "approval" | "closure"
    )
}

pub(crate) fn implementation_lane_is_diagnostic(active_node: &str) -> bool {
    !implementation_lane_allows_terminal_completion(active_node)
}

fn is_task_id_boundary_char(value: char) -> bool {
    !(value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
}

fn request_text_mentions_task_id(request_text: &str, task_id: &str) -> bool {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return false;
    }

    let mut search_from = 0;
    while let Some(offset) = request_text[search_from..].find(task_id) {
        let start = search_from + offset;
        let end = start + task_id.len();
        let before_boundary = start == 0
            || request_text[..start]
                .chars()
                .next_back()
                .is_some_and(is_task_id_boundary_char);
        let after_boundary = end >= request_text.len()
            || request_text[end..]
                .chars()
                .next()
                .is_some_and(is_task_id_boundary_char);
        if before_boundary && after_boundary {
            return true;
        }
        search_from = end;
    }

    false
}

async fn resolve_seed_task_id_for_runtime_run(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
) -> Result<String, String> {
    if store.show_task(requested_run_id).await.is_ok() {
        return Ok(requested_run_id.to_string());
    }

    let mut matches = store
        .list_tasks(None, true)
        .await
        .map_err(|error| {
            format!(
                "Failed to resolve TaskFlow task id for runtime run `{requested_run_id}`: {error}"
            )
        })?
        .into_iter()
        .filter(|task| task.status != "closed")
        .filter(|task| request_text_mentions_task_id(request_text, &task.id))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        return Ok(requested_run_id.to_string());
    }
    if matches.len() == 1 {
        return Ok(matches.remove(0).id);
    }

    let active_matches = matches
        .iter()
        .filter(|task| task.status == "in_progress")
        .collect::<Vec<_>>();
    if active_matches.len() == 1 {
        return Ok(active_matches[0].id.clone());
    }

    matches.sort_by(|left, right| left.id.cmp(&right.id));
    let ids = matches
        .iter()
        .map(|task| task.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Runtime run `{requested_run_id}` request text references multiple open TaskFlow task ids ({ids}); cite a single bounded task before dispatch."
    ))
}

pub(crate) async fn derive_seeded_run_graph_state(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    derive_seeded_run_graph_state_with_persistence(store, requested_run_id, request_text, true)
        .await
}

pub(crate) async fn derive_seeded_run_graph_state_read_only(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    derive_seeded_run_graph_state_with_persistence(store, requested_run_id, request_text, false)
        .await
}

pub(crate) async fn derive_seeded_run_graph_state_with_persistence(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
    persist_launcher_snapshot: bool,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    derive_seeded_run_graph_state_with_stage(
        store,
        requested_run_id,
        request_text,
        None,
        false,
        persist_launcher_snapshot,
    )
    .await
}

async fn derive_seeded_run_graph_state_with_stage(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    skip_design_override: bool,
    persist_launcher_snapshot: bool,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_resolve_task_id");
    let bounded_task_id =
        resolve_seed_task_id_for_runtime_run(store, requested_run_id, request_text).await?;
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_read_launcher_snapshot");
    let snapshot = read_seed_launcher_activation_snapshot(store, persist_launcher_snapshot).await?;
    let bounded_task = store.show_task(&bounded_task_id).await.ok();
    if let Some(task) = bounded_task
        .as_ref()
        .filter(|task| task_has_configured_dev_team_dispatch_identity(task))
    {
        let activation_bundle = activation_bundle_with_dev_team_readiness(&snapshot);
        if let Some(route) =
            crate::dev_team_sequence_contract::configured_dev_team_first_step_for_task(
                &activation_bundle,
                task,
            )
        {
            set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_dev_team_route_selection");
            let mut selection = configured_dev_team_lane_selection_from_snapshot(
                &snapshot,
                activation_bundle,
                request_text,
                &route,
            );
            selection.execution_plan =
                build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, &selection);
            validate_configured_dev_team_route_against_authority(&selection, &route)?;
            inject_task_planner_metadata(&mut selection, &task.planner_metadata);
            if let Some(path) =
                existing_design_backed_task_design_doc_path(store, &bounded_task_id).await
            {
                inject_tracked_design_doc_path(&mut selection.execution_plan, &path);
            }
            set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_dev_team_route_status");
            let mut status = seeded_run_graph_state_from_role_selection(
                requested_run_id,
                &bounded_task_id,
                &selection,
                &snapshot,
            )?;
            apply_configured_dev_team_route_to_state(&mut status, &selection, &route)?;
            return Ok(TaskflowRunGraphSeedPayload {
                request_text: request_text.to_string(),
                role_selection: selection,
                status,
            });
        }
    }
    if skip_design_override {
        if bounded_task
            .as_ref()
            .is_some_and(|task| task.issue_type.trim() != "task")
        {
            if let Some(payload) = configured_dev_team_seed_payload_from_task(
                store,
                &snapshot,
                &bounded_task_id,
                requested_run_id,
                request_text,
                bounded_task.as_ref(),
                timeout_stage,
                false,
            )
            .await?
            {
                return Ok(payload);
            }
        }
        set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_task_metadata_selection");
        let mut selection =
            bounded_implementation_lane_selection_from_snapshot(&snapshot, request_text);
        set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_task_metadata_execution_plan");
        selection.execution_plan =
            build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, &selection);
        if let Some(task) = bounded_task.as_ref() {
            inject_task_planner_metadata(&mut selection, &task.planner_metadata);
        }
        if let Some(path) =
            existing_design_backed_task_design_doc_path(store, &bounded_task_id).await
        {
            inject_tracked_design_doc_path(&mut selection.execution_plan, &path);
        }
        set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_task_metadata_status");
        let status = seeded_run_graph_state_from_role_selection(
            requested_run_id,
            &bounded_task_id,
            &selection,
            &snapshot,
        )?;
        return Ok(TaskflowRunGraphSeedPayload {
            request_text: request_text.to_string(),
            role_selection: selection,
            status,
        });
    }
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_build_from_snapshot");
    let mut design_backed_payload = build_seeded_run_graph_state_from_activation_snapshot(
        requested_run_id,
        &bounded_task_id,
        request_text,
        &snapshot,
    )?;
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_existing_design_override");
    try_existing_design_backed_implementation_override(
        store,
        &bounded_task_id,
        request_text,
        &mut design_backed_payload.role_selection,
    )
    .await?;
    if bounded_task
        .as_ref()
        .is_some_and(|task| task.issue_type.trim() != "task")
    {
        if let Some(payload) = configured_dev_team_seed_payload_from_task(
            store,
            &snapshot,
            &bounded_task_id,
            requested_run_id,
            request_text,
            bounded_task.as_ref(),
            timeout_stage,
            false,
        )
        .await?
        {
            return Ok(payload);
        }
    }
    if selection_is_bounded_implementation_seed(&design_backed_payload.role_selection) {
        set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_status_from_role_selection");
        design_backed_payload.status = seeded_run_graph_state_from_role_selection(
            requested_run_id,
            &bounded_task_id,
            &design_backed_payload.role_selection,
            &snapshot,
        )?;
        if design_backed_payload.role_selection.request != request_text {
            design_backed_payload.request_text = request_text.to_string();
        }
        return Ok(design_backed_payload);
    }
    if let Some(payload) = configured_dev_team_seed_payload_from_task(
        store,
        &snapshot,
        &bounded_task_id,
        requested_run_id,
        request_text,
        bounded_task.as_ref(),
        timeout_stage,
        true,
    )
    .await?
    {
        return Ok(payload);
    }
    let mut payload = design_backed_payload;
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_status_from_role_selection");
    payload.status = seeded_run_graph_state_from_role_selection(
        requested_run_id,
        &bounded_task_id,
        &payload.role_selection,
        &snapshot,
    )?;
    if payload.role_selection.request != request_text {
        payload.request_text = request_text.to_string();
    }
    Ok(payload)
}

async fn read_seed_launcher_activation_snapshot(
    store: &StateStore,
    persist_launcher_snapshot: bool,
) -> Result<crate::state_store::LauncherActivationSnapshot, String> {
    if !persist_launcher_snapshot {
        return crate::launcher_activation_snapshot::read_or_capture_launcher_activation_snapshot(
            store,
        )
        .await;
    }
    match crate::launcher_activation_snapshot::read_or_sync_launcher_activation_snapshot(store)
        .await
    {
        Ok(snapshot) => Ok(snapshot),
        Err(error) if error.starts_with("Unable to resolve launcher activation project root") => {
            match store.read_launcher_activation_snapshot().await {
                Ok(snapshot) => Ok(snapshot),
                Err(_) => {
                    crate::launcher_activation_snapshot::capture_launcher_activation_snapshot()
                }
            }
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn run_graph_dispatch_context_from_seed_payload(
    payload: &TaskflowRunGraphSeedPayload,
) -> crate::state_store::RunGraphDispatchContext {
    let mut role_selection =
        serde_json::to_value(&payload.role_selection).unwrap_or(serde_json::Value::Null);
    if let Some(object) = role_selection.as_object_mut() {
        object.insert("compiled_bundle".to_string(), serde_json::Value::Null);
    }
    crate::state_store::RunGraphDispatchContext {
        run_id: payload.status.run_id.clone(),
        task_id: payload.status.task_id.clone(),
        request_text: payload.request_text.clone(),
        role_selection,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    }
}

pub(crate) async fn rehydrate_persisted_role_selection(
    store: &StateStore,
    selection: RuntimeConsumptionLaneSelection,
    task_id: Option<&str>,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    rehydrate_persisted_role_selection_with_persistence(store, selection, task_id, true).await
}

async fn rehydrate_persisted_role_selection_with_persistence(
    store: &StateStore,
    mut selection: RuntimeConsumptionLaneSelection,
    task_id: Option<&str>,
    persist_launcher_snapshot: bool,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let snapshot = read_seed_launcher_activation_snapshot(store, persist_launcher_snapshot).await?;
    let compiled_bundle = snapshot.compiled_bundle;
    selection.compiled_bundle = compiled_bundle.clone();

    let persisted_selected_node_id =
        crate::runtime_dispatch_state::selected_flow_node_ref(&selection).map(str::to_string);
    let task_flow_ref =
        if let Some(task_id) = task_id.map(str::trim).filter(|value| !value.is_empty()) {
            if let Ok(task) = store.show_task(task_id).await {
                Some(
                    crate::dev_team_sequence_contract::selected_dev_team_flow_id_for_task(
                        &compiled_bundle,
                        &task,
                    )?,
                )
            } else {
                None
            }
        } else {
            None
        };
    let flow_ref = crate::runtime_dispatch_state::validated_selected_flow_ref(
        &selection,
        task_flow_ref.as_deref(),
        crate::runtime_dispatch_state::SelectedFlowIdentityMode::Replay,
    )
    .map_err(|blocker| blocker.to_string())?;
    if let Some(flow_id) = flow_ref.as_deref() {
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &compiled_bundle,
            Some(flow_id),
            None,
        )
        .map_err(|blocker| blocker.to_string())?;
        if persisted_selected_node_id.is_none() {
            return Err("team_flow_authority_selected_node_id_missing".to_string());
        }
        let has_dispatch_contract = selection
            .execution_plan
            .get("development_flow")
            .and_then(|flow| flow.get("dispatch_contract"))
            .or_else(|| selection.execution_plan.get("dispatch_contract"))
            .is_some();
        if has_dispatch_contract {
            let validation_target = persisted_selected_node_id
                .as_deref()
                .ok_or_else(|| "team_flow_authority_selected_node_id_missing".to_string())?;
            authority
                .resolve_target(Some(&selection.execution_plan), validation_target)
                .map_err(|blocker| blocker.to_string())?;
        }
        crate::development_flow_orchestration::normalize_selected_flow_for_execution_plan_with_selected_node(
            &mut selection,
            &compiled_bundle,
            flow_id,
            persisted_selected_node_id.as_deref(),
        )?;
    } else {
        selection.execution_plan =
            build_runtime_execution_plan_from_snapshot(&compiled_bundle, &selection);
    }
    if let Some(flow_id) = flow_ref.as_deref() {
        let plan = selection
            .execution_plan
            .as_object_mut()
            .ok_or_else(|| "team_flow_authority_rehydrated_execution_plan_missing".to_string())?;
        plan.insert(
            "team_flow_authority_selected_flow_id".to_string(),
            serde_json::Value::String(flow_id.to_string()),
        );
        let dispatch_contract = plan["development_flow"]["dispatch_contract"].clone();
        if dispatch_contract["status"] == "blocked" {
            let blockers = dispatch_contract["blocker_codes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            if !blockers.is_empty() {
                return Err(format!(
                    "team_flow_authority_rehydrated_dispatch_contract_blocked:{}",
                    blockers.join(",")
                ));
            }
            return Err("team_flow_authority_rehydrated_execution_plan_blocked".to_string());
        }
        let contract_flow =
            plan["development_flow"]["dispatch_contract"]["selected_flow_set"].as_str();
        if contract_flow != Some(flow_id) {
            return Err(format!(
                "team_flow_authority_rehydrated_flow_identity_mismatch:{flow_id}:{}",
                contract_flow.unwrap_or("<missing>")
            ));
        }
    }
    crate::team_flow_authority_adapter::require_team_flow_execution_authority(
        &selection.compiled_bundle,
        flow_ref.as_deref(),
        None,
    )
    .map_err(|blocker| blocker.to_string())?;
    Ok(selection)
}

pub(crate) async fn rehydrate_persisted_role_selection_value(
    store: &StateStore,
    value: serde_json::Value,
    task_id: Option<&str>,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let selection = serde_json::from_value(value)
        .map_err(|error| format!("Failed to decode persisted role selection: {error}"))?;
    rehydrate_persisted_role_selection(store, selection, task_id).await
}

pub(crate) async fn rehydrate_dispatch_context_role_selection(
    store: &StateStore,
    context: &RunGraphDispatchContext,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    rehydrate_dispatch_context_role_selection_with_persistence(store, context, true).await
}

pub(crate) async fn rehydrate_dispatch_context_role_selection_read_only(
    store: &StateStore,
    context: &RunGraphDispatchContext,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    rehydrate_dispatch_context_role_selection_with_persistence(store, context, false).await
}

async fn rehydrate_dispatch_context_role_selection_with_persistence(
    store: &StateStore,
    context: &RunGraphDispatchContext,
    persist_launcher_snapshot: bool,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let selection = context
        .role_selection()
        .map_err(|error| format!("Failed to decode persisted seeded dispatch context: {error}"))?;
    rehydrate_persisted_role_selection_with_persistence(
        store,
        selection,
        Some(&context.task_id),
        persist_launcher_snapshot,
    )
    .await
}

fn seed_payload_operator_surface_json(payload: &TaskflowRunGraphSeedPayload) -> serde_json::Value {
    let mut payload_json = serde_json::to_value(payload)
        .expect("run-graph seed payload should render as operator-surface json");
    if let Some(object) = payload_json["role_selection"].as_object_mut() {
        object.insert("compiled_bundle".to_string(), serde_json::Value::Null);
    }
    payload_json
}

fn dispatch_init_route_targets(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Result<Vec<String>, String> {
    Ok(
        crate::runtime_dispatch_state::typed_lane_node_sequence(role_selection, true)?
            .into_iter()
            .map(|node| node.node_id)
            .collect(),
    )
}

const ACTUATABLE_SELECTED_BACKEND_KEYS: &[&str] = &[
    "selected_backend",
    "selected_backend_id",
    "selected_carrier_id",
    "selected_carrier_agent_id",
    "selected_agent_id",
    "activation_agent_type",
    "selected_tier",
];

fn disabled_external_backend_ref_from_overlay(
    overlay: &serde_yaml::Value,
    backend_id: &str,
    source: &str,
) -> Option<serde_json::Value> {
    let backend_id = backend_id.trim();
    if backend_id.is_empty() {
        return None;
    }
    let backend_entry = crate::yaml_lookup(overlay, &["agent_system", "subagents", backend_id])?;
    let backend_class = crate::yaml_lookup(backend_entry, &["subagent_backend_class"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if backend_class != "external_cli" {
        return None;
    }
    let blocker_reason =
        crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            backend_id,
            backend_entry,
        )?;
    Some(serde_json::json!({
        "source": source,
        "backend_id": backend_id,
        "blocking": true,
        "readiness": {
            "backend_id": backend_id,
            "status": "external_backend_dispatch_blocked",
            "blocked": true,
            "blocker_code": "configured_backend_dispatch_failed",
            "blocker_reason": blocker_reason,
        },
    }))
}

fn collect_disabled_external_backend_refs_from_value(
    overlay: &serde_yaml::Value,
    value: &serde_json::Value,
    path: &str,
    refs: &mut Vec<serde_json::Value>,
    seen: &mut BTreeSet<String>,
) {
    const MAX_BACKEND_REF_SCAN_DEPTH: usize = 96;
    const MAX_BACKEND_REF_SCAN_NODES: usize = 20_000;

    let mut stack = vec![(value, path.to_string(), 0usize)];
    let mut visited_nodes = 0usize;
    while let Some((value, path, depth)) = stack.pop() {
        visited_nodes += 1;
        if visited_nodes > MAX_BACKEND_REF_SCAN_NODES || depth > MAX_BACKEND_REF_SCAN_DEPTH {
            continue;
        }
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    if ACTUATABLE_SELECTED_BACKEND_KEYS.contains(&key.as_str()) {
                        if let Some(backend_id) = child.as_str().map(str::trim) {
                            let seen_key = format!("{child_path}\u{1f}{backend_id}");
                            if seen.insert(seen_key) {
                                if let Some(reference) = disabled_external_backend_ref_from_overlay(
                                    overlay,
                                    backend_id,
                                    &child_path,
                                ) {
                                    refs.push(reference);
                                }
                            }
                        }
                    }
                    stack.push((child, child_path, depth + 1));
                }
            }
            serde_json::Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    stack.push((child, format!("{path}[{index}]"), depth + 1));
                }
            }
            _ => {}
        }
    }
}

fn disabled_external_backend_refs_payload_for_value_from_overlay(
    overlay: &serde_yaml::Value,
    value: &serde_json::Value,
) -> Option<serde_json::Value> {
    let mut refs = Vec::new();
    let mut seen = BTreeSet::new();
    collect_disabled_external_backend_refs_from_value(overlay, value, "", &mut refs, &mut seen);
    if refs.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "status": "blocked",
        "blocking": true,
        "refs": refs,
        "next_actions": [
            "reseed the route assignment from the current carrier config before trusting persisted dispatch artifacts",
            "remove disabled external backends from actuatable selected-backend fields or enable the backend with receipt-backed readiness evidence",
        ],
    }))
}

fn dispatch_context_route_assignment_catalog_drift(
    state_root: &std::path::Path,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Option<serde_json::Value> {
    set_dispatch_init_timeout_stage(None, "drift_resolve_project_root");
    let project_root =
        crate::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(state_root);
    set_dispatch_init_timeout_stage(None, "drift_load_model_profile_catalog");
    let catalog = crate::runtime_dispatch_state::current_project_model_profile_catalog_for_root(
        project_root.as_ref(),
    );
    set_dispatch_init_timeout_stage(None, "drift_load_project_overlay");
    let overlay =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root.as_ref())
            .ok();
    if let Some(overlay) = overlay.as_ref() {
        set_dispatch_init_timeout_stage(None, "drift_scan_execution_plan_disabled_backends");
        if let Some(drift) = disabled_external_backend_refs_payload_for_value_from_overlay(
            overlay,
            &role_selection.execution_plan,
        ) {
            return Some(serde_json::json!({
                "dispatch_target": "execution_plan",
                "drift": {
                    "kind": "disabled_external_backend_ref",
                    "status": "blocked",
                    "route_disabled_external_backend_refs": drift,
                },
            }));
        }
    }
    set_dispatch_init_timeout_stage(None, "drift_collect_dispatch_targets");
    let route_targets = match dispatch_init_route_targets(role_selection) {
        Ok(targets) => targets,
        Err(blocker_code) => {
            return Some(serde_json::json!({
                "dispatch_target": "execution_plan",
                "drift": {
                    "kind": "team_flow_authority_blocked",
                    "status": "blocked",
                    "blocker_codes": [blocker_code],
                },
            }));
        }
    };
    for target in route_targets {
        set_dispatch_init_timeout_stage(None, "drift_lookup_route_for_target");
        let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &role_selection.execution_plan,
            &target,
        );
        set_dispatch_init_timeout_stage(None, "drift_build_route_explain_payload");
        let payload = crate::taskflow_routing::route_explain_payload(
            &role_selection.execution_plan,
            &role_selection.compiled_bundle,
            &target,
            route,
        );
        if !catalog.is_empty() {
            set_dispatch_init_timeout_stage(None, "drift_check_catalog_drift");
            if let Some(drift) =
                crate::runtime_dispatch_state::route_assignment_catalog_drift_payload(
                    &payload, &catalog,
                )
            {
                return Some(serde_json::json!({
                    "dispatch_target": target,
                    "drift": drift,
                }));
            }
        }
        if let Some(overlay) = overlay.as_ref() {
            set_dispatch_init_timeout_stage(None, "drift_check_overlay_disabled_backends");
            if let Some(drift) =
                crate::taskflow_proxy::disabled_external_backend_refs_payload_from_overlay(
                    overlay, &payload,
                )
                .filter(|drift| drift["blocking"].as_bool() == Some(true))
            {
                return Some(serde_json::json!({
                    "dispatch_target": target,
                    "drift": {
                        "kind": "disabled_external_backend_ref",
                        "status": "blocked",
                        "route_disabled_external_backend_refs": drift,
                    },
                }));
            }
        }
    }
    None
}

fn execution_plan_dev_team_route_signature(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Result<serde_json::Value, String> {
    let execution_nodes =
        crate::runtime_dispatch_state::typed_lane_node_sequence(role_selection, true)?;
    let allowed_nodes =
        crate::runtime_dispatch_state::typed_lane_node_sequence(role_selection, false)?;
    let execution_lane_sequence = execution_nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let allowed_next_lane_sequence = allowed_nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let mut route_catalog = serde_json::Map::new();
    for node in allowed_nodes.iter().chain(execution_nodes.iter()) {
        if route_catalog.contains_key(&node.node_id) {
            continue;
        }
        route_catalog.insert(
            node.node_id.clone(),
            serde_json::json!({
                "resolution": {
                    "node_id": node.node_id.as_str(),
                    "dispatch_target": node.dispatch_target.as_str(),
                    "dispatch_alias": node.dispatch_alias.as_str(),
                    "lane_id": node.lane_id.as_str(),
                },
                "runtime_role": node.runtime_role.as_str(),
                "task_class": node.task_class.as_str(),
                "packet_template_kind": node.packet_template_kind.as_str(),
                "activation_runtime_role": node
                    .activation
                    .get("activation_runtime_role")
                    .and_then(serde_json::Value::as_str),
            }),
        );
    }
    Ok(serde_json::json!({
        "allowed_next_lane_sequence": allowed_next_lane_sequence,
        "execution_lane_sequence": execution_lane_sequence,
        "route_catalog": route_catalog,
    }))
}

fn dispatch_receipt_disabled_external_backend_drift(
    state_root: &std::path::Path,
    receipt: &RunGraphDispatchReceipt,
) -> Option<serde_json::Value> {
    let project_root =
        crate::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(state_root);
    let overlay =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root.as_ref())
            .ok()?;
    let receipt_payload = serde_json::to_value(receipt).ok()?;
    let drift =
        disabled_external_backend_refs_payload_for_value_from_overlay(&overlay, &receipt_payload)?;
    Some(serde_json::json!({
        "dispatch_target": receipt.dispatch_target,
        "drift": {
            "kind": "disabled_external_backend_ref",
            "status": "blocked",
            "route_disabled_external_backend_refs": drift,
        },
    }))
}

async fn reseed_dispatch_context_after_route_assignment_drift(
    store: &StateStore,
    status: &RunGraphStatus,
    context: &RunGraphDispatchContext,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    reseed_dispatch_context_after_route_assignment_drift_with_persistence(
        store, status, context, true,
    )
    .await
}

async fn reseed_dispatch_context_after_route_assignment_drift_with_persistence(
    store: &StateStore,
    status: &RunGraphStatus,
    context: &RunGraphDispatchContext,
    persist_launcher_snapshot: bool,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    let task_id = if status.task_id.trim().is_empty() {
        context.task_id.as_str()
    } else {
        status.task_id.as_str()
    };
    if let Ok(task) = store.show_task(task_id).await {
        if taskflow_task_state_is_terminal_for_dispatch_init(&task.status) {
            return Err(format!(
                "Dispatch-init cannot refresh stale route assignment for terminal TaskFlow task `{task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
                task.status
            ));
        }
    }
    derive_seeded_run_graph_state_with_persistence(
        store,
        task_id,
        &context.request_text,
        persist_launcher_snapshot,
    )
    .await
}

pub(crate) async fn persist_seed_artifacts(
    store: &StateStore,
    payload: &TaskflowRunGraphSeedPayload,
) -> Result<(), String> {
    store
        .clear_run_graph_dispatch_receipt(&payload.status.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to clear stale dispatch receipt before reseeding run `{}`: {error}",
                payload.status.run_id
            )
        })?;
    store
        .record_run_graph_dispatch_context(&run_graph_dispatch_context_from_seed_payload(payload))
        .await
        .map_err(|error| format!("Failed to record seeded dispatch context: {error}"))?;
    store
        .record_run_graph_status(&payload.status)
        .await
        .map_err(|error| format!("Failed to record seeded run-graph state: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding_with_request_text(
        store,
        &payload.status,
        "run_graph_seed",
        Some(&payload.request_text),
    )
    .await?;
    Ok(())
}

pub(crate) async fn persist_selected_node_for_run_graph_transition(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<(), String> {
    let Some(mut context) = store
        .run_graph_dispatch_context(&status.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read run-graph dispatch context before selected-node transition: {error}"
            )
        })?
    else {
        return Ok(());
    };
    let selected_node_id = status
        .next_node
        .as_deref()
        .or_else(|| (!status.active_node.trim().is_empty()).then_some(status.active_node.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "team_flow_authority_selected_node_id_missing_after_transition".to_string()
        })?;
    let mut progressed = rehydrate_dispatch_context_role_selection(store, &context).await?;
    project_selected_node_for_run_graph_status(&mut progressed, status, Some(selected_node_id))?;
    let mut role_selection = serde_json::to_value(&progressed)
        .map_err(|error| format!("Failed to encode progressed TeamFlow role selection: {error}"))?;
    role_selection["compiled_bundle"] = serde_json::Value::Null;
    context.role_selection = role_selection;
    context.recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| {
            format!("Failed to timestamp progressed TeamFlow role selection: {error}")
        })?;
    store
        .record_run_graph_dispatch_context(&context)
        .await
        .map_err(|error| format!("Failed to persist progressed TeamFlow selected node: {error}"))
}

fn is_external_run_graph_dispatch_target(status: &RunGraphStatus, target: &str) -> bool {
    let target = target.trim();
    if target.is_empty() {
        return false;
    }
    if matches!(
        target,
        "research-pack"
            | "spec-pack"
            | "work-pool-pack"
            | "dev-pack"
            | "bug-pool-pack"
            | "reflection-pack"
    ) {
        return matches!(
            status.task_class.as_str(),
            "scope_discussion" | "pbi_discussion"
        );
    }
    status.task_class == "implementation"
        && matches!(
            target,
            "review_ensemble" | "verification" | "verification_ensemble" | "approval"
        )
}

pub(crate) fn project_selected_node_for_run_graph_status(
    selection: &mut RuntimeConsumptionLaneSelection,
    status: &RunGraphStatus,
    receipt_dispatch_target: Option<&str>,
) -> Result<(), String> {
    let selected_node_id = status
        .next_node
        .as_deref()
        .filter(|target| !is_external_run_graph_dispatch_target(status, target))
        .or_else(|| (!status.active_node.trim().is_empty()).then_some(status.active_node.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "team_flow_authority_selected_node_id_missing_after_transition".to_string()
        })?;
    let authority =
        crate::runtime_dispatch_state::require_persisted_team_flow_authority_for_selection(
            selection,
        )
        .map_err(|blocker| blocker.to_string())?;
    let status_node = crate::runtime_dispatch_state::resolve_team_flow_target_for_selection(
        &authority,
        Some(&selection.execution_plan),
        selected_node_id,
    )
    .or_else(|blocker| {
        let verification_alias = matches!(
            selected_node_id,
            "review_ensemble" | "verification" | "verification_ensemble"
        );
        if !verification_alias || status.task_class != "implementation" {
            return Err(blocker);
        }
        let candidates = authority
            .ordered_nodes()
            .filter(|projection| projection.node.task_class == "verification")
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(blocker);
        }
        crate::runtime_dispatch_state::resolve_team_flow_target_for_selection(
            &authority,
            Some(&selection.execution_plan),
            &candidates[0].node.node_id,
        )
    })
    .map_err(|blocker| blocker.to_string())?;
    if let Some(receipt_dispatch_target) = receipt_dispatch_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !is_external_run_graph_dispatch_target(status, receipt_dispatch_target) {
            let receipt_node =
                crate::runtime_dispatch_state::resolve_team_flow_target_for_selection(
                    &authority,
                    Some(&selection.execution_plan),
                    receipt_dispatch_target,
                )
                .map_err(|blocker| blocker.to_string())?;
            if receipt_node.node_id != status_node.node_id {
                return Err(format!(
                    "team_flow_selected_node_status_receipt_mismatch:{}:{}:{}",
                    status_node.node_id, status_node.dispatch_target, receipt_node.node_id
                ));
            }
        }
    }
    let plan = selection
        .execution_plan
        .as_object_mut()
        .ok_or_else(|| "team_flow_authority_execution_plan_missing".to_string())?;
    plan.insert(
        "team_flow_authority_selected_node_id".to_string(),
        serde_json::Value::String(status_node.node_id.clone()),
    );
    if let Some(contract) = plan
        .get_mut("development_flow")
        .and_then(|flow| flow.get_mut("dispatch_contract"))
        .and_then(serde_json::Value::as_object_mut)
    {
        contract.insert(
            "team_flow_authority_selected_node_id".to_string(),
            serde_json::Value::String(status_node.node_id.clone()),
        );
        contract.insert(
            "selected_node_id".to_string(),
            serde_json::Value::String(status_node.node_id.clone()),
        );
    }
    if let Some(contract) = plan
        .get_mut("selected_flow_contract")
        .and_then(serde_json::Value::as_object_mut)
    {
        if contract.contains_key("selected_node_id") {
            contract.insert(
                "selected_node_id".to_string(),
                serde_json::Value::String(status_node.node_id),
            );
        }
    }
    Ok(())
}

pub(crate) fn run_graph_dispatch_bootstrap_from_state(
    status: &RunGraphStatus,
) -> Result<serde_json::Value, String> {
    validate_run_graph_resume_gate(status)?;
    let latest_status = serde_json::to_value(status)
        .map_err(|error| format!("Failed to encode status: {error}"))?;
    Ok(serde_json::json!({
        "status": "dispatch_init_ready",
        "handoff_ready": true,
        "run_id": status.run_id,
        "latest_status": latest_status,
    }))
}

#[derive(Debug)]
pub(crate) struct RunGraphDispatchInitArtifacts {
    pub(crate) requested_run_id: String,
    pub(crate) run_id: String,
    #[allow(dead_code)]
    pub(crate) role_selection: crate::RuntimeConsumptionLaneSelection,
    pub(crate) run_graph_bootstrap: serde_json::Value,
    pub(crate) taskflow_handoff_plan: serde_json::Value,
    pub(crate) dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    pub(crate) dispatch_packet_path: String,
}

impl RunGraphDispatchInitArtifacts {
    fn into_json_payload(self) -> serde_json::Value {
        let downstream_dispatch_packet_path = self
            .dispatch_receipt
            .downstream_dispatch_packet_path
            .clone();
        let taskflow_handoff_plan =
            compact_taskflow_handoff_plan_for_dispatch_init(&self.taskflow_handoff_plan);
        let dispatch_init_dev_team_route_signature =
            execution_plan_dev_team_route_signature(&self.role_selection).unwrap_or_else(|code| {
                serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": [code],
                })
            });
        serde_json::json!({
            "surface": "vida taskflow run-graph dispatch-init",
            "requested_run_id": self.requested_run_id,
            "run_id": self.run_id,
            "dispatch_receipt": self.dispatch_receipt,
            "dispatch_packet_path": self.dispatch_packet_path,
            "downstream_dispatch_packet_path": downstream_dispatch_packet_path,
            "taskflow_handoff_plan": taskflow_handoff_plan,
            "run_graph_bootstrap": self.run_graph_bootstrap,
            "full_handoff_plan_location": "dispatch_packet",
            "dispatch_init_dev_team_route_signature": dispatch_init_dev_team_route_signature,
        })
    }
}

fn compact_taskflow_handoff_plan_for_dispatch_init(plan: &serde_json::Value) -> serde_json::Value {
    let mut summary = serde_json::Map::new();
    for key in [
        "status",
        "handoff_ready",
        "design_packet_activation_source",
        "required_artifacts",
        "execution_preparation_artifacts",
    ] {
        if let Some(value) = plan.get(key) {
            summary.insert(key.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(summary)
}

fn dispatch_init_cache_record_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

pub(crate) fn run_graph_dispatch_init_fast_cache_path(
    state_root: &std::path::Path,
    run_id: &str,
) -> std::path::PathBuf {
    state_root
        .join("runtime-consumption")
        .join("dispatch-init-cache")
        .join(format!("{}.json", dispatch_init_cache_record_id(run_id)))
}

fn current_dispatch_init_cache_config_digest(state_root: &std::path::Path) -> Option<String> {
    let project_root =
        crate::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(state_root);
    crate::launcher_activation_snapshot::config_file_digest(&project_root.join("vida.config.yaml"))
        .ok()
}

fn dispatch_init_fast_cache_payload_is_reusable(
    state_root: &std::path::Path,
    payload: &serde_json::Value,
    requested_run_id: &str,
    current_config_digest: Option<&str>,
) -> bool {
    let requested_matches = payload["requested_run_id"].as_str() == Some(requested_run_id)
        || payload["run_id"].as_str() == Some(requested_run_id);
    if !requested_matches
        || payload["surface"].as_str() != Some("vida taskflow run-graph dispatch-init")
    {
        return false;
    }
    if payload["dispatch_init_fast_cache_schema_version"].as_u64()
        != Some(DISPATCH_INIT_FAST_CACHE_SCHEMA_VERSION)
    {
        return false;
    }
    if let Some(current_config_digest) = current_config_digest {
        if payload["source_config_digest"].as_str() != Some(current_config_digest) {
            return false;
        }
    }
    if payload
        .get("authoritative_persistence")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status != "recorded")
    {
        return false;
    }
    if payload["dispatch_receipt"]["dispatch_status"].as_str() != Some("routed") {
        return false;
    }
    if !payload
        .get("dispatch_init_dev_team_route_signature")
        .is_some_and(serde_json::Value::is_object)
    {
        return false;
    }
    let project_root =
        crate::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(state_root);
    if let Ok(overlay) =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root.as_ref())
    {
        if disabled_external_backend_refs_payload_for_value_from_overlay(&overlay, payload)
            .is_some()
        {
            return false;
        }
    }
    if !payload["dispatch_receipt"]["dispatch_command"]
        .as_str()
        .map(str::trim)
        .is_some_and(|command| !command.is_empty())
    {
        return false;
    }
    let Some(packet_path) = payload["dispatch_packet_path"].as_str().map(str::trim) else {
        return false;
    };
    if packet_path.is_empty() || !std::path::Path::new(packet_path).is_file() {
        return false;
    }
    let Some(packet) = crate::read_json_file_if_present(std::path::Path::new(packet_path)) else {
        return false;
    };
    let packet_selected_backend = dispatch_init_packet_selected_backend(packet_path);
    if packet
        .get("runtime_assignment")
        .and_then(|assignment| assignment.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
        && packet_selected_backend.is_none()
    {
        return false;
    }
    if packet_selected_backend.is_none() {
        return false;
    }
    if dispatch_init_packet_template_kind(packet_path).as_deref() == Some("delivery_task_packet") {
        let run_id = payload["run_id"].as_str().unwrap_or(requested_run_id);
        if dispatch_init_delivery_packet_string_field(packet_path, "task_id").as_deref()
            != Some(run_id)
        {
            return false;
        }
        if let (Some(owned_paths), Some(implementation_isolation_owned_paths)) = (
            dispatch_init_delivery_packet_string_array(packet_path, "owned_paths"),
            dispatch_init_delivery_packet_implementation_isolation_owned_paths(packet_path),
        ) {
            if implementation_isolation_owned_paths != owned_paths {
                return false;
            }
        }
    }
    true
}

pub(crate) fn read_run_graph_dispatch_init_fast_cache(
    state_root: &std::path::Path,
    requested_run_id: &str,
) -> Option<serde_json::Value> {
    let path = run_graph_dispatch_init_fast_cache_path(state_root, requested_run_id);
    let body = std::fs::read_to_string(path).ok()?;
    let payload = serde_json::from_str::<serde_json::Value>(&body).ok()?;
    let current_config_digest = current_dispatch_init_cache_config_digest(state_root);
    dispatch_init_fast_cache_payload_is_reusable(
        state_root,
        &payload,
        requested_run_id,
        current_config_digest.as_deref(),
    )
    .then_some(payload)
}

async fn read_run_graph_dispatch_init_fast_cache_for_dispatch_init(
    state_root: &std::path::Path,
    requested_run_id: &str,
) -> Option<serde_json::Value> {
    let payload = read_run_graph_dispatch_init_fast_cache(state_root, requested_run_id)?;
    let run_id = payload["run_id"].as_str().unwrap_or(requested_run_id);
    let store = tokio::time::timeout(
        DISPATCH_INIT_IDENTITY_BACKFILL_OPEN_TIMEOUT,
        StateStore::open_existing_read_only(state_root.to_path_buf()),
    )
    .await
    .ok()?
    .ok()?;
    let current_seed = tokio::time::timeout(
        DISPATCH_INIT_IDENTITY_BACKFILL_OPEN_TIMEOUT,
        seed_existing_task_payload_for_dispatch_init(&store, run_id, None, false),
    )
    .await
    .ok()?
    .ok()
    .flatten();
    store.close().await;
    let current_seed = current_seed?;
    dispatch_init_fast_cache_payload_matches_current_dev_team_route_signature(
        &payload,
        &current_seed.role_selection,
    )
    .then_some(payload)
}

fn dispatch_init_fast_cache_payload_matches_current_dev_team_route_signature(
    payload: &serde_json::Value,
    current_role_selection: &RuntimeConsumptionLaneSelection,
) -> bool {
    let current_signature = match execution_plan_dev_team_route_signature(current_role_selection) {
        Ok(signature) => signature,
        Err(_) => return false,
    };
    payload["dispatch_init_dev_team_route_signature"] == current_signature
}

async fn dispatch_context_configured_dev_team_route_drift(
    store: &StateStore,
    run_id: &str,
    role_selection: &RuntimeConsumptionLaneSelection,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
) -> Result<Option<serde_json::Value>, String> {
    dispatch_context_configured_dev_team_route_drift_with_persistence(
        store,
        run_id,
        role_selection,
        timeout_stage,
        true,
    )
    .await
}

async fn dispatch_context_configured_dev_team_route_drift_with_persistence(
    store: &StateStore,
    run_id: &str,
    role_selection: &RuntimeConsumptionLaneSelection,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    persist_launcher_snapshot: bool,
) -> Result<Option<serde_json::Value>, String> {
    let Some(current_seed) = seed_existing_task_payload_for_dispatch_init(
        store,
        run_id,
        timeout_stage,
        persist_launcher_snapshot,
    )
    .await?
    else {
        return Ok(None);
    };
    let persisted_signature = execution_plan_dev_team_route_signature(role_selection)?;
    let current_signature = execution_plan_dev_team_route_signature(&current_seed.role_selection)?;
    if persisted_signature == current_signature {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "dispatch_target": "execution_plan",
        "drift": {
            "kind": "configured_dev_team_route_sequence_drift",
            "status": "blocked",
            "persisted_signature": persisted_signature,
            "current_signature": current_signature,
            "next_actions": [
                "reseed the dispatch context from the current dev-team route before trusting persisted dispatch artifacts"
            ],
        },
    })))
}

fn read_run_graph_state_json_from_dispatch_init_fast_cache(
    state_root: &std::path::Path,
    run_id: &str,
) -> Option<serde_json::Value> {
    let payload = read_run_graph_dispatch_init_fast_cache(state_root, run_id)?;
    payload.get("latest_status").cloned().or_else(|| {
        payload
            .pointer("/run_graph_bootstrap/latest_status")
            .cloned()
    })
}

fn write_run_graph_dispatch_init_fast_cache(
    state_root: &std::path::Path,
    requested_run_id: &str,
    run_id: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let cache_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-init-cache");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Failed to create dispatch-init fast cache dir: {error}"))?;
    let mut payload = payload.clone();
    payload["dispatch_init_fast_cache_schema_version"] =
        serde_json::Value::Number(DISPATCH_INIT_FAST_CACHE_SCHEMA_VERSION.into());
    if let Some(digest) = current_dispatch_init_cache_config_digest(state_root) {
        payload["source_config_digest"] = serde_json::Value::String(digest);
    }
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Failed to encode dispatch-init fast cache: {error}"))?;
    for cache_id in [requested_run_id, run_id] {
        let path = run_graph_dispatch_init_fast_cache_path(state_root, cache_id);
        std::fs::write(&path, &body).map_err(|error| {
            format!(
                "Failed to write dispatch-init fast cache `{}`: {error}",
                path.display()
            )
        })?;
    }
    Ok(())
}

pub(crate) fn clear_run_graph_dispatch_init_fast_cache(state_root: &std::path::Path, run_id: &str) {
    let path = run_graph_dispatch_init_fast_cache_path(state_root, run_id);
    let _ = std::fs::remove_file(path);
    let cache_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-init-cache");
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(&body) else {
            continue;
        };
        if payload["run_id"].as_str() == Some(run_id)
            || payload["requested_run_id"].as_str() == Some(run_id)
        {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn reusable_routed_dispatch_receipt(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if receipt.dispatch_status != "routed" {
        return None;
    }
    if !receipt
        .dispatch_command
        .as_deref()
        .map(str::trim)
        .is_some_and(|command| !command.is_empty())
    {
        return None;
    }
    let packet_path = receipt.dispatch_packet_path.as_deref()?.trim();
    if packet_path.is_empty() || !std::path::Path::new(packet_path).is_file() {
        return None;
    }
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    if !crate::runtime_dispatch_state::runtime_dispatch_packet_has_top_level_task_scope_mirror(
        &packet,
    ) {
        return None;
    }
    let packet_selected_backend = dispatch_init_packet_selected_backend(packet_path);
    if packet
        .get("runtime_assignment")
        .and_then(|assignment| assignment.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
        && packet_selected_backend.is_none()
    {
        return None;
    }
    if packet_selected_backend.is_none() {
        return None;
    }
    Some(packet_path.to_string())
}

fn dispatch_init_packet_selected_backend(packet_path: &str) -> Option<String> {
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    [
        "/runtime_assignment/selected_backend_id",
        "/runtime_assignment/selected_carrier_id",
        "/carrier_runtime_assignment/selected_backend_id",
        "/carrier_runtime_assignment/selected_carrier_id",
        "/selected_backend",
        "/execution_truth/effective_selected_backend",
        "/route_policy/effective_selected_backend",
    ]
    .into_iter()
    .find_map(|pointer| {
        packet
            .pointer(pointer)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn dispatch_init_packet_template_kind(packet_path: &str) -> Option<String> {
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn dispatch_init_delivery_packet_string_field(packet_path: &str, key: &str) -> Option<String> {
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    packet
        .get("delivery_task_packet")?
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn dispatch_init_delivery_packet_string_array(packet_path: &str, key: &str) -> Option<Vec<String>> {
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    packet
        .get("delivery_task_packet")?
        .get(key)?
        .as_array()?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .collect::<Option<Vec<_>>>()
}

fn dispatch_init_delivery_packet_implementation_isolation_owned_paths(
    packet_path: &str,
) -> Option<Vec<String>> {
    let packet = crate::read_json_file_if_present(std::path::Path::new(packet_path))?;
    packet
        .get("delivery_task_packet")?
        .get("implementation_isolation")?
        .get("owned_paths")?
        .as_array()?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .collect::<Option<Vec<_>>>()
}

async fn existing_dispatch_receipt_matches_current_seed(
    store: &StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
) -> Result<bool, String> {
    existing_dispatch_receipt_matches_current_seed_with_persistence(
        store,
        run_id,
        receipt,
        timeout_stage,
        true,
    )
    .await
}

async fn existing_dispatch_receipt_matches_current_seed_with_persistence(
    store: &StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    persist_launcher_snapshot: bool,
) -> Result<bool, String> {
    let Some(packet_path) = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Ok(true);
    };
    let Some(current_seed) = seed_existing_task_payload_for_dispatch_init(
        store,
        run_id,
        timeout_stage,
        persist_launcher_snapshot,
    )
    .await?
    else {
        return Ok(true);
    };
    let current_bootstrap = run_graph_dispatch_bootstrap_from_state(&current_seed.status)?;
    let current_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &current_seed.role_selection,
        &current_bootstrap,
    )?;
    if receipt.dispatch_target != current_receipt.dispatch_target {
        return Ok(false);
    }
    if receipt.activation_runtime_role != current_receipt.activation_runtime_role {
        return Ok(false);
    }
    if receipt.activation_agent_type != current_receipt.activation_agent_type {
        return Ok(false);
    }
    let current_packet_kind = crate::runtime_dispatch_state::runtime_dispatch_packet_kind(
        &current_seed.role_selection.execution_plan,
        &current_receipt.dispatch_target,
        &current_receipt.dispatch_kind,
    );
    if dispatch_init_packet_template_kind(packet_path).as_deref()
        != Some(current_packet_kind.as_str())
    {
        return Ok(false);
    }
    if current_packet_kind == "delivery_task_packet" {
        let packet_task_id = dispatch_init_delivery_packet_string_field(packet_path, "task_id");
        if packet_task_id.as_deref() != Some(current_receipt.run_id.as_str()) {
            return Ok(false);
        }
        let handoff_runtime_role = current_receipt
            .activation_runtime_role
            .as_deref()
            .unwrap_or(current_seed.role_selection.selected_role.as_str());
        let handoff_task_class =
            crate::runtime_dispatch_state::runtime_packet_handoff_task_class_for_plan(
                &current_seed.role_selection.execution_plan,
                &current_receipt.dispatch_target,
                handoff_runtime_role,
            );
        if crate::runtime_dispatch_packets::delivery_packet_task_class_requires_owned_paths(
            handoff_task_class.as_str(),
        ) {
            let expected_owned_paths =
                crate::runtime_dispatch_state::owned_paths_for_required_delivery_task_class(
                    &current_seed.role_selection,
                    handoff_task_class.as_str(),
                );
            if !expected_owned_paths.is_empty() {
                if dispatch_init_delivery_packet_string_array(packet_path, "owned_paths").as_deref()
                    != Some(expected_owned_paths.as_slice())
                {
                    return Ok(false);
                }
                if crate::runtime_dispatch_packets::delivery_packet_task_class_requires_implementation_isolation(handoff_task_class.as_str())
                    && dispatch_init_delivery_packet_implementation_isolation_owned_paths(packet_path)
                        .as_deref()
                        != Some(expected_owned_paths.as_slice())
                {
                    return Ok(false);
                }
            }
        }
    }
    let current_backend =
        crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
            &current_seed.role_selection.execution_plan,
            &current_receipt.dispatch_target,
            current_receipt.activation_agent_type.as_deref(),
            None,
        );
    let packet_backend = dispatch_init_packet_selected_backend(packet_path);
    Ok(match (packet_backend, current_backend) {
        (Some(packet_backend), Some(current_backend)) => packet_backend == current_backend,
        _ => true,
    })
}

async fn existing_routed_dispatch_init_artifacts(
    store: &StateStore,
    requested_run_id: &str,
    run_id: &str,
) -> Result<Option<RunGraphDispatchInitArtifacts>, String> {
    existing_routed_dispatch_init_artifacts_with_persistence(store, requested_run_id, run_id, true)
        .await
}

async fn existing_routed_dispatch_init_artifacts_with_persistence(
    store: &StateStore,
    requested_run_id: &str,
    run_id: &str,
    persist_launcher_snapshot: bool,
) -> Result<Option<RunGraphDispatchInitArtifacts>, String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };
    if !status.recovery_ready || !status.resume_target.starts_with("dispatch.") {
        return Ok(None);
    }
    let Some(context) = store
        .run_graph_dispatch_context(run_id)
        .await
        .map_err(|error| format!("Failed to read existing dispatch context: {error}"))?
    else {
        return Ok(None);
    };
    let Some(dispatch_receipt) = store
        .run_graph_dispatch_receipt(run_id)
        .await
        .map_err(|error| format!("Failed to read existing dispatch receipt: {error}"))?
    else {
        return Ok(None);
    };
    if dispatch_receipt_disabled_external_backend_drift(store.root(), &dispatch_receipt).is_some() {
        return Ok(None);
    }
    let Some(dispatch_packet_path) = reusable_routed_dispatch_receipt(&dispatch_receipt) else {
        return Ok(None);
    };
    let role_selection = context
        .role_selection()
        .map_err(|error| format!("Failed to decode existing seeded dispatch context: {error}"))?;
    if dispatch_context_route_assignment_catalog_drift(store.root(), &role_selection).is_some() {
        return Ok(None);
    }
    if dispatch_context_configured_dev_team_route_drift_with_persistence(
        store,
        run_id,
        &role_selection,
        None,
        persist_launcher_snapshot,
    )
    .await?
    .is_some()
    {
        return Ok(None);
    }
    if !existing_dispatch_receipt_matches_current_seed_with_persistence(
        store,
        run_id,
        &dispatch_receipt,
        None,
        persist_launcher_snapshot,
    )
    .await?
    {
        return Ok(None);
    }
    let run_graph_bootstrap = run_graph_dispatch_bootstrap_from_state(&status)?;
    let taskflow_handoff_plan = crate::build_taskflow_handoff_plan(&role_selection);
    Ok(Some(RunGraphDispatchInitArtifacts {
        requested_run_id: requested_run_id.to_string(),
        run_id: run_id.to_string(),
        role_selection,
        run_graph_bootstrap,
        taskflow_handoff_plan,
        dispatch_receipt,
        dispatch_packet_path,
    }))
}

pub(crate) fn dispatch_command_from_packet_path(
    packet_path: &str,
) -> Result<Option<String>, String> {
    let body = std::fs::read_to_string(packet_path).map_err(|error| {
        format!("Failed to read rendered dispatch packet `{packet_path}`: {error}")
    })?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
        format!("Failed to decode rendered dispatch packet `{packet_path}`: {error}")
    })?;
    Ok(json
        .get("dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

async fn reseed_explicit_task_graph_binding_for_dispatch_init(
    store: &StateStore,
    requested_run_id: &str,
    persist_launcher_snapshot: bool,
) -> Result<Option<String>, String> {
    let binding = store
        .run_graph_continuation_binding(requested_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read explicit continuation binding for `{requested_run_id}`: {error}"
            )
        })?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return Ok(None);
    }

    let bound_task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(binding.task_id.as_str());
    if bound_task_id == requested_run_id {
        return Ok(None);
    }
    let bound_task = store.show_task(bound_task_id).await.ok();
    if let Some(task) = bound_task.as_ref() {
        if taskflow_task_state_is_terminal_for_dispatch_init(&task.status) {
            return Err(format!(
                "Run `{requested_run_id}` has explicit continuation binding to terminal task_graph_task `{bound_task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
                task.status
            ));
        }
    }

    let request_text = if let Some(task) = bound_task.as_ref() {
        task_record_dispatch_seed_request_text(task)
    } else if let Some(request_text) = binding
        .request_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        request_text.to_string()
    } else if let Some(context) = store
        .run_graph_dispatch_context(requested_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read persisted seeded dispatch context for `{requested_run_id}` while reseeding explicit continuation binding: {error}"
            )
        })?
    {
        context.request_text
    } else {
        return Err(format!(
            "Run `{requested_run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but no persisted request text is available to reseed dispatch-init for the bound task."
        ));
    };

    if !persist_launcher_snapshot {
        return Ok(Some(bound_task_id.to_string()));
    }
    let payload = derive_seeded_run_graph_state(store, bound_task_id, &request_text).await?;
    persist_seed_artifacts(store, &payload).await?;

    let why = format!(
        "Explicit continuation binding for run `{requested_run_id}` reseeded bounded task `{bound_task_id}` into a fresh dispatch-ready run."
    );
    if let Some(binding) = crate::taskflow_continuation::build_run_graph_continuation_binding(
        &payload.status,
        Some(&request_text),
        "explicit_continuation_bind",
        Some(&why),
    ) {
        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .map_err(|error| {
                format!(
                    "Failed to record reseeded explicit continuation binding for `{bound_task_id}`: {error}"
                )
            })?;
    }

    Ok(Some(bound_task_id.to_string()))
}

fn task_record_dispatch_seed_request_text(task: &crate::state_store::TaskRecord) -> String {
    let mut parts = Vec::new();
    let title = task.title.trim();
    if !title.is_empty() {
        parts.push(title.to_string());
    }
    let description = task.description.trim();
    if !description.is_empty() && description != title {
        parts.push(description.to_string());
    }
    if !task.planner_metadata.owned_paths.is_empty() {
        parts.push(format!(
            "Owned paths: {}.",
            task.planner_metadata.owned_paths.join(", ")
        ));
    }
    if !task.planner_metadata.proof_targets.is_empty() {
        parts.push(format!(
            "Proof targets: {}.",
            task.planner_metadata.proof_targets.join(", ")
        ));
    }
    if parts.is_empty() {
        task.id.clone()
    } else {
        parts.join("\n\n")
    }
}

fn taskflow_task_state_is_terminal_for_dispatch_init(status: &str) -> bool {
    matches!(status.trim(), "closed" | "completed")
}

async fn configured_dev_team_seed_payload_from_task(
    store: &StateStore,
    snapshot: &crate::state_store::LauncherActivationSnapshot,
    bounded_task_id: &str,
    requested_run_id: &str,
    request_text: &str,
    task: Option<&crate::state_store::TaskRecord>,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    allow_design_backed: bool,
) -> Result<Option<TaskflowRunGraphSeedPayload>, String> {
    let Some(task) = task.filter(|task| task_has_configured_dev_team_dispatch_identity(task))
    else {
        return Ok(None);
    };
    let design_doc_path = existing_design_backed_task_design_doc_path(store, bounded_task_id).await;
    if design_doc_path.is_some() && !allow_design_backed {
        return Ok(None);
    }
    let activation_bundle = activation_bundle_with_dev_team_readiness(snapshot);
    let Some(route) = crate::dev_team_sequence_contract::configured_dev_team_first_step_for_task(
        &activation_bundle,
        task,
    ) else {
        return Ok(None);
    };
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_dev_team_route_selection");
    let mut selection = configured_dev_team_lane_selection_from_snapshot(
        snapshot,
        activation_bundle,
        request_text,
        &route,
    );
    selection.execution_plan =
        build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, &selection);
    validate_configured_dev_team_route_against_authority(&selection, &route)?;
    inject_task_planner_metadata(&mut selection, &task.planner_metadata);
    if let Some(path) = design_doc_path {
        inject_tracked_design_doc_path(&mut selection.execution_plan, &path);
    }
    set_dispatch_init_timeout_stage(timeout_stage, "derive_seed_dev_team_route_status");
    let mut status = seeded_run_graph_state_from_role_selection(
        requested_run_id,
        bounded_task_id,
        &selection,
        snapshot,
    )?;
    apply_configured_dev_team_route_to_state(&mut status, &selection, &route)?;
    Ok(Some(TaskflowRunGraphSeedPayload {
        request_text: request_text.to_string(),
        role_selection: selection,
        status,
    }))
}

fn activation_bundle_with_dev_team_readiness(
    snapshot: &crate::state_store::LauncherActivationSnapshot,
) -> serde_json::Value {
    let mut activation_bundle = snapshot.compiled_bundle.clone();
    let source_config_path = snapshot.source_config_path.trim();
    let primary_config_path = if source_config_path.is_empty() {
        "vida.config.yaml"
    } else {
        source_config_path
    };
    let mut readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
        primary_config_path,
        &activation_bundle,
    );
    if !dev_team_readiness_has_route_truth(&readiness) {
        let config_path = std::path::Path::new(primary_config_path);
        if let Some(project_root) = config_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            if let Ok(snapshot) =
                crate::launcher_activation_snapshot::capture_launcher_activation_snapshot_for_root(
                    project_root,
                )
            {
                activation_bundle = snapshot.compiled_bundle;
                readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    &snapshot.source_config_path,
                    &activation_bundle,
                );
            }
        } else if let Ok(snapshot) =
            crate::launcher_activation_snapshot::capture_launcher_activation_snapshot()
        {
            activation_bundle = snapshot.compiled_bundle;
            readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                &snapshot.source_config_path,
                &activation_bundle,
            );
        }
    }
    if readiness
        .get("flows")
        .and_then(serde_json::Value::as_array)
        .is_none_or(Vec::is_empty)
        && primary_config_path != "vida.config.yaml"
    {
        readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
            "vida.config.yaml",
            &activation_bundle,
        );
    }
    if let Some(object) = activation_bundle.as_object_mut() {
        object.insert("dev_team_readiness".to_string(), readiness);
    }
    activation_bundle
}

fn dev_team_readiness_has_route_truth(readiness: &serde_json::Value) -> bool {
    readiness
        .get("roles")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|roles| !roles.is_empty())
        && readiness
            .get("flows")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|flows| !flows.is_empty())
}

fn configured_dev_team_lane_selection_from_snapshot(
    snapshot: &crate::state_store::LauncherActivationSnapshot,
    activation_bundle: serde_json::Value,
    request_text: &str,
    route: &crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute,
) -> RuntimeConsumptionLaneSelection {
    let fallback_role = snapshot
        .compiled_bundle
        .get("role_selection")
        .and_then(|value| value.get("fallback_role"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("orchestrator")
        .to_string();
    let selection_mode = snapshot
        .compiled_bundle
        .get("role_selection")
        .and_then(|value| value.get("mode"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string();
    RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: snapshot.source.clone(),
        selection_mode,
        fallback_role,
        request: request_text.to_string(),
        selected_role: route.runtime_role.clone(),
        conversational_mode: None,
        single_task_only: true,
        tracked_flow_entry: Some(
            match route.dispatch_target.as_str() {
                "specification" => "spec-pack",
                "coach" => "coach-pack",
                "verification" => "verification-pack",
                "execution_preparation" => "execution-preparation-pack",
                _ => "dev-pack",
            }
            .to_string(),
        ),
        allow_freeform_chat: false,
        confidence: "explicit_configured_dev_team_dispatch_init".to_string(),
        matched_terms: route
            .flow_id
            .as_ref()
            .map(|flow_id| {
                vec![
                    route.role_label.clone(),
                    route.runtime_role.clone(),
                    route.task_class.clone(),
                    format!("dev_team_flow_id:{flow_id}"),
                ]
            })
            .unwrap_or_else(|| {
                vec![
                    route.role_label.clone(),
                    route.runtime_role.clone(),
                    route.task_class.clone(),
                ]
            }),
        compiled_bundle: activation_bundle,
        execution_plan: serde_json::Value::Null,
        reason: "configured_dev_team_first_step_dispatch_init".to_string(),
    }
}

fn configured_dev_team_route_blocker(
    route: &crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute,
    requested_target: &str,
    runtime_role: &str,
    task_class: &str,
    blocker: &crate::team_flow_authority_adapter::TeamFlowResolutionBlocker,
    projected_node: Option<&crate::team_flow_authority_adapter::TeamFlowNodeProjection>,
) -> String {
    let flow_id = route.flow_id.as_deref().unwrap_or("<default>");
    let node_id = projected_node
        .map(|node| node.node.node_id.as_str())
        .unwrap_or("<unknown>");
    let included = projected_node
        .map(|node| node.node.included.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let inclusion_rule = projected_node
        .map(|node| node.node.inclusion_rule.as_str())
        .unwrap_or("<unknown>");
    let candidates = if blocker.candidates.is_empty() {
        "<none>".to_string()
    } else {
        blocker.candidates.join(",")
    };
    format!(
        "{}: flow_id={flow_id}:requested_target={requested_target}:runtime_role={runtime_role}:task_class={task_class}:node_id={node_id}:included={included}:inclusion_rule={inclusion_rule}:requested={}:candidates={candidates}",
        blocker.code, blocker.requested,
    )
}

fn validate_configured_dev_team_route_against_authority(
    role_selection: &RuntimeConsumptionLaneSelection,
    route: &crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute,
) -> Result<(), String> {
    let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
        &role_selection.compiled_bundle,
        route.flow_id.as_deref(),
        None,
    )
    .map_err(|blocker| {
        configured_dev_team_route_blocker(
            route,
            route.dispatch_target.as_str(),
            route.runtime_role.as_str(),
            route.task_class.as_str(),
            &blocker,
            None,
        )
    })?;
    let requested_target = route.dispatch_target.trim();
    if requested_target.is_empty() {
        return Err("team_flow_route_dispatch_target_missing".to_string());
    }
    let projected_node_for_target = |target: &str| {
        authority.projection().nodes.iter().find(|node| {
            node.node.node_id == target
                || node.dispatch_target == target
                || node.dispatch_alias == target
        })
    };
    let route_projected_node = projected_node_for_target(requested_target);
    let route_node = authority
        .resolve_target(Some(&role_selection.execution_plan), requested_target)
        .map_err(|blocker| {
            configured_dev_team_route_blocker(
                route,
                requested_target,
                route.runtime_role.as_str(),
                route.task_class.as_str(),
                &blocker,
                route_projected_node,
            )
        })?;
    let route_node_id = route.node_id.trim();
    if route_node_id.is_empty() {
        return Err("team_flow_route_node_id_missing".to_string());
    }
    let identity_node = authority
        .resolve_target(Some(&role_selection.execution_plan), route_node_id)
        .map_err(|blocker| {
            configured_dev_team_route_blocker(
                route,
                route_node_id,
                route.runtime_role.as_str(),
                route.task_class.as_str(),
                &blocker,
                authority.projection().node(route_node_id),
            )
        })?;
    if identity_node.node_id != route_node.node_id {
        return Err(format!(
            "team_flow_route_node_id_target_mismatch:{}:{}",
            route_node_id, route_node.node_id
        ));
    }
    if route_node.runtime_role != route.runtime_role.trim() {
        return Err(format!(
            "team_flow_route_runtime_role_mismatch:{}:{}",
            requested_target, route_node.runtime_role
        ));
    }
    if route_node.task_class != route.task_class.trim() {
        return Err(format!(
            "team_flow_route_task_class_mismatch:{}:configured={}:authority={}",
            requested_target,
            route.task_class.trim(),
            route_node.task_class
        ));
    }

    let sequence = if route.sequence.is_empty() {
        vec![(
            route.node_id.as_str(),
            route.runtime_role.as_str(),
            route.task_class.as_str(),
        )]
    } else {
        route
            .sequence
            .iter()
            .map(|step| {
                (
                    step.node_id.as_str(),
                    step.runtime_role.as_str(),
                    step.task_class.as_str(),
                )
            })
            .collect::<Vec<_>>()
    };
    if sequence.is_empty() {
        return Err("team_flow_route_sequence_missing".to_string());
    }
    let mut route_target_seen = false;
    for (requested_target, runtime_role, task_class) in sequence {
        let requested_target = requested_target.trim();
        let runtime_role = runtime_role.trim();
        let task_class = task_class.trim();
        if requested_target.is_empty() {
            return Err("team_flow_route_step_target_missing".to_string());
        }
        let projected_node = projected_node_for_target(requested_target);
        let node = authority
            .resolve_target(Some(&role_selection.execution_plan), requested_target)
            .map_err(|blocker| {
                configured_dev_team_route_blocker(
                    route,
                    requested_target,
                    runtime_role,
                    task_class,
                    &blocker,
                    projected_node,
                )
            })?;
        if node.runtime_role != runtime_role {
            return Err(format!(
                "team_flow_route_runtime_role_mismatch:{}:{}",
                requested_target, node.runtime_role
            ));
        }
        if node.task_class != task_class {
            return Err(format!(
                "team_flow_route_task_class_mismatch:{}:configured={}:authority={}",
                requested_target, task_class, node.task_class
            ));
        }
        route_target_seen |= requested_target == route.node_id.trim();
    }
    if !route_target_seen {
        return Err(format!(
            "team_flow_route_dispatch_target_not_in_sequence:{}",
            route.node_id.trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
fn inject_configured_dev_team_route_into_execution_plan(
    execution_plan: &mut serde_json::Value,
    route: &crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute,
) {
    let Some(root) = execution_plan.as_object_mut() else {
        return;
    };
    let development_flow = root
        .entry("development_flow".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(development_flow) = development_flow.as_object_mut() else {
        return;
    };
    let dispatch_contract = development_flow
        .entry("dispatch_contract".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(dispatch_contract) = dispatch_contract.as_object_mut() else {
        return;
    };
    let sequence = if route.sequence.is_empty() {
        vec![crate::dev_team_sequence_contract::DevTeamSequenceStep {
            node_id: route.node_id.clone(),
            dispatch_target: route.dispatch_target.clone(),
            role_label: route.role_label.clone(),
            runtime_role: route.runtime_role.clone(),
            task_class: route.task_class.clone(),
            packet_template_kind: None,
            closure_class: None,
            stage: None,
            completion_blocker: None,
            inclusion_rule: None,
            requires_task: true,
            requires_user_approval: false,
            approval_policy: serde_json::Value::Null,
            lifecycle_hook_templates: serde_json::Value::Null,
            resume_transitions: serde_json::Value::Null,
            rework_transitions: serde_json::Value::Null,
        }]
    } else {
        route.sequence.clone()
    };
    let lane_sequence = sequence
        .iter()
        .map(|step| step.node_id.trim())
        .filter(|target| !target.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !lane_sequence.is_empty() {
        dispatch_contract.insert(
            "lane_sequence".to_string(),
            serde_json::json!(lane_sequence.clone()),
        );
        dispatch_contract.insert(
            "execution_lane_sequence".to_string(),
            serde_json::json!(lane_sequence),
        );
    }
    let lane_catalog = dispatch_contract
        .entry("lane_catalog".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(lane_catalog) = lane_catalog.as_object_mut() else {
        return;
    };
    for step in sequence {
        let node_id = step.node_id.trim();
        let dispatch_target = step.dispatch_target.trim();
        if node_id.is_empty() || dispatch_target.is_empty() {
            continue;
        }
        lane_catalog.insert(
            node_id.to_string(),
            serde_json::json!({
                "node_id": step.node_id,
                "dispatch_target": dispatch_target,
                "runtime_role": step.runtime_role,
                "task_class": step.task_class,
                "packet_template_kind": step.packet_template_kind,
                "closure_class": step.closure_class,
                "stage": step.stage,
                "completion_blocker": step.completion_blocker,
                "inclusion_rule": step.inclusion_rule,
                "requires_task": step.requires_task,
                "requires_user_approval": step.requires_user_approval,
                "approval_policy": step.approval_policy,
                "lifecycle_hook_templates": step.lifecycle_hook_templates,
                "resume_transitions": step.resume_transitions,
                "rework_transitions": step.rework_transitions,
                "runtime_assignment": {
                    "enabled": true,
                    "runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                    "activation_runtime_role": step.runtime_role,
                    "selected_backend_id": "internal_subagents",
                    "selected_dispatch_backend_id": "internal_subagents",
                },
                "carrier_runtime_assignment": {
                    "enabled": true,
                    "runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                    "activation_runtime_role": step.runtime_role,
                    "selected_backend_id": "internal_subagents",
                    "selected_dispatch_backend_id": "internal_subagents",
                },
                "activation": {
                    "activation_runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                },
            }),
        );
    }
}

fn apply_configured_dev_team_route_to_state(
    status: &mut RunGraphStatus,
    selection: &RuntimeConsumptionLaneSelection,
    route: &crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute,
) -> Result<(), String> {
    validate_configured_dev_team_route_against_authority(selection, route)?;
    let node_id = route.node_id.trim();
    if node_id.is_empty() {
        return Err("team_flow_route_node_id_missing".to_string());
    }
    let node_id = node_id.to_string();
    let dispatch_target = route.dispatch_target.trim();
    if dispatch_target.is_empty() {
        return Err("team_flow_route_dispatch_target_missing".to_string());
    }
    let dispatch_target = dispatch_target.to_string();
    status.status = "ready".to_string();
    status.context_state = "ready".to_string();
    status.task_class = route.task_class.clone();
    status.route_task_class = route.task_class.clone();
    // Run-graph progression persists the exact TeamFlow node id. Human-facing
    // dispatch targets remain in the receipt/command projection, while strict
    // rehydration resolves this field against the selected authority.
    status.next_node = Some(node_id);
    status.lane_id = format!("{}_lane", route.node_id);
    status.lifecycle_stage = format!("{}_dispatch_ready", route.node_id);
    status.policy_gate = "not_required".to_string();
    status.handoff_state = format!("awaiting_{}", route.node_id);
    status.resume_target = format!("dispatch.{}", route.node_id);
    status.recovery_ready = true;
    if let Some(selected_backend) =
        crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
            &selection.execution_plan,
            &dispatch_target,
            None,
            None,
        )
    {
        status.selected_backend = selected_backend;
    }
    Ok(())
}

fn task_has_configured_dev_team_dispatch_identity(task: &crate::state_store::TaskRecord) -> bool {
    task.issue_type.trim() != "task"
        || !task.planner_metadata.owned_paths.is_empty()
        || !task.labels.is_empty()
}

pub(crate) async fn run_graph_state_has_configured_dev_team_route_mismatch(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<bool, String> {
    run_graph_state_has_configured_dev_team_route_mismatch_with_persistence(store, status, true)
        .await
}

async fn run_graph_state_has_configured_dev_team_route_mismatch_with_persistence(
    store: &StateStore,
    status: &RunGraphStatus,
    persist_launcher_snapshot: bool,
) -> Result<bool, String> {
    let task = match store.show_task(&status.task_id).await {
        Ok(task) => task,
        Err(_) => return Ok(false),
    };
    if !task_has_configured_dev_team_dispatch_identity(&task) {
        return Ok(false);
    }
    let snapshot = read_seed_launcher_activation_snapshot(store, persist_launcher_snapshot).await?;
    let activation_bundle = activation_bundle_with_dev_team_readiness(&snapshot);
    let Some(route) = crate::dev_team_sequence_contract::configured_dev_team_first_step_for_task(
        &activation_bundle,
        &task,
    ) else {
        return Ok(false);
    };
    let current_dispatch_target = status
        .next_node
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(status.active_node.trim());
    Ok(current_dispatch_target != route.node_id
        || status.task_class != route.task_class
        || status.route_task_class != route.task_class)
}

fn bounded_implementation_lane_selection_from_snapshot(
    snapshot: &crate::state_store::LauncherActivationSnapshot,
    request_text: &str,
) -> RuntimeConsumptionLaneSelection {
    let fallback_role = snapshot
        .compiled_bundle
        .get("role_selection")
        .and_then(|value| value.get("fallback_role"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("orchestrator")
        .to_string();
    let selection_mode = snapshot
        .compiled_bundle
        .get("role_selection")
        .and_then(|value| value.get("mode"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string();
    RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: snapshot.source.clone(),
        selection_mode,
        fallback_role,
        request: request_text.to_string(),
        selected_role: "worker".to_string(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: Some("dev-pack".to_string()),
        allow_freeform_chat: false,
        confidence: "medium".to_string(),
        matched_terms: vec!["task_metadata_bounded_implementation".to_string()],
        compiled_bundle: snapshot.compiled_bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: "auto_task_metadata_bounded_implementation_request".to_string(),
    }
}

fn build_seeded_run_graph_state_from_activation_snapshot(
    requested_run_id: &str,
    bounded_task_id: &str,
    request_text: &str,
    snapshot: &crate::state_store::LauncherActivationSnapshot,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    let selection = crate::runtime_lane_summary::build_runtime_lane_selection_from_bundle(
        &snapshot.compiled_bundle,
        &snapshot.source,
        &snapshot.pack_router_keywords,
        request_text,
    )?;
    let status = seeded_run_graph_state_from_role_selection(
        requested_run_id,
        bounded_task_id,
        &selection,
        snapshot,
    )?;
    Ok(TaskflowRunGraphSeedPayload {
        request_text: request_text.to_string(),
        role_selection: selection,
        status,
    })
}

fn seeded_run_graph_state_from_role_selection(
    requested_run_id: &str,
    bounded_task_id: &str,
    selection: &RuntimeConsumptionLaneSelection,
    snapshot: &crate::state_store::LauncherActivationSnapshot,
) -> Result<RunGraphStatus, String> {
    let execution_plan = &selection.execution_plan;
    let compiled_control =
        compiled_run_graph_control_from_bundle(&snapshot.compiled_bundle, &snapshot.source)?;
    let is_conversation = selection.conversational_mode.is_some();
    let task_class = if is_conversation {
        selection
            .conversational_mode
            .clone()
            .unwrap_or_else(|| "conversation".to_string())
    } else {
        "implementation".to_string()
    };
    let route = if is_conversation {
        &execution_plan["default_route"]
    } else {
        &execution_plan["development_flow"]["implementation"]
    };
    let first_execution_node = if is_conversation {
        None
    } else {
        execution_plan["development_flow"]["implementation"]
            .get("team_flow_selected_node_id")
            .and_then(serde_json::Value::as_str)
            .filter(|node_id| !node_id.trim().is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| Some(compiled_control.entry_execution_node_id.clone()))
    };
    let lane_node = first_execution_node
        .clone()
        .unwrap_or_else(|| selection.selected_role.clone());
    let selected_backend = if is_conversation {
        let configured_route = execution_plan["default_route"]
            .as_object()
            .filter(|route| !route.is_empty())
            .cloned()
            .map(serde_json::Value::Object)
            .or_else(|| {
                selection
                    .tracked_flow_entry
                    .as_deref()
                    .and_then(|route_id| {
                        selection.compiled_bundle["agent_system"]["routing"][route_id]
                            .as_object()
                            .filter(|route| !route.is_empty())
                            .map(|route| serde_json::Value::Object(route.clone()))
                    })
            })
            .or_else(|| {
                selection.compiled_bundle["agent_system"]["routing"]["default"]
                    .as_object()
                    .filter(|route| !route.is_empty())
                    .map(|route| serde_json::Value::Object(route.clone()))
            });
        configured_route
            .as_ref()
            .and_then(|route| {
                crate::taskflow_routing::selected_backend_from_execution_plan_route(
                    execution_plan,
                    route,
                )
            })
            .filter(|backend| {
                selection.compiled_bundle["agent_system"]["subagents"][backend]["enabled"]
                    .as_bool()
                    .unwrap_or(false)
            })
            .or_else(|| {
                crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
                    execution_plan,
                    lane_node.as_str(),
                    json_raw_string_field(route, "activation_agent_type").as_deref(),
                    None,
                )
            })
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
            execution_plan,
            lane_node.as_str(),
            json_raw_string_field(route, "activation_agent_type").as_deref(),
            None,
        )
        .unwrap_or_else(|| "unknown".to_string())
    };
    let lane_id = if is_conversation {
        format!("{lane_node}_lane")
    } else {
        crate::runtime_dispatch_state::typed_lane_node_sequence(selection, true)?
            .into_iter()
            .find(|node| node.node_id == lane_node)
            .map(|node| node.lane_id)
            .ok_or_else(|| format!("team_flow_node_identity_unknown:{lane_node}"))?
    };
    let next_node = Some(lane_node.clone());
    let lifecycle_stage = if is_conversation {
        "dispatch_ready".to_string()
    } else {
        "implementation_dispatch_ready".to_string()
    };
    let policy_gate = if is_conversation {
        if selection.single_task_only {
            "single_task_scope_required".to_string()
        } else {
            "not_required".to_string()
        }
    } else if execution_plan["state_owner"].as_str() == Some("orchestrator_only")
        && compiled_control.validation_report_required_before_implementation
    {
        "validation_report_required".to_string()
    } else {
        "not_required".to_string()
    };
    let handoff_state = if is_conversation {
        format!("awaiting_{}", selection.selected_role)
    } else {
        format!("awaiting_{lane_node}")
    };
    let checkpoint_kind = if is_conversation {
        "conversation_cursor".to_string()
    } else {
        "execution_cursor".to_string()
    };
    let recovery_ready = is_conversation
        || json_bool_field(route, "analysis_required").unwrap_or(false)
        || json_bool_field(route, "coach_required").unwrap_or(false)
        || json_bool_field(route, "independent_verification_required").unwrap_or(false);
    let seed_base = RunGraphStatus {
        run_id: requested_run_id.to_string(),
        task_id: bounded_task_id.to_string(),
        task_class,
        active_node: "planning".to_string(),
        route_task_class: if is_conversation {
            selection
                .tracked_flow_entry
                .clone()
                .or_else(|| selection.conversational_mode.clone())
                .unwrap_or_else(|| selection.selected_role.clone())
        } else {
            "implementation".to_string()
        },
        selected_backend,
        ..default_run_graph_state(requested_run_id, "planning", "implementation")
    };
    let mut status = run_graph_state_from_authority_ready_transition(
        &seed_base,
        "planning".to_string(),
        next_node,
        lane_id,
        lifecycle_stage,
        policy_gate,
        checkpoint_kind,
        DispatchTargetFormat::Lane,
        recovery_ready,
    );
    status.task_class = seed_base.task_class;
    status.route_task_class = seed_base.route_task_class;
    status.selected_backend = seed_base.selected_backend;
    status.handoff_state = handoff_state;
    if !is_conversation {
        let authority_sequence =
            crate::runtime_dispatch_state::typed_lane_node_sequence(selection, false);
        if let Err(blocker) = authority_sequence {
            status.lifecycle_stage = blocker.clone();
            status.policy_gate = blocker.clone();
            status.handoff_state = format!("blocked_{blocker}");
            status.next_node = None;
            status.status = "blocked".to_string();
            status.context_state = "blocked".to_string();
            status.recovery_ready = false;
        }
    }
    if status.status == "blocked" {
        status.next_node = None;
        status.context_state = "blocked".to_string();
        status.recovery_ready = false;
    }
    Ok(status)
}

async fn seed_existing_task_payload_for_dispatch_init(
    store: &StateStore,
    task_id: &str,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    persist_launcher_snapshot: bool,
) -> Result<Option<TaskflowRunGraphSeedPayload>, String> {
    set_dispatch_init_timeout_stage(timeout_stage, "seed_existing_task_show_task");
    let task = match store.show_task(task_id).await {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    if taskflow_task_state_is_terminal_for_dispatch_init(&task.status) {
        return Err(format!(
            "Dispatch-init cannot seed terminal TaskFlow task `{task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
            task.status
        ));
    }
    set_dispatch_init_timeout_stage(timeout_stage, "seed_existing_task_request_text");
    let request_text = task_record_dispatch_seed_request_text(&task);
    let design_doc_has_bounded_scope = existing_design_backed_task_design_doc_path(store, task_id)
        .await
        .as_ref()
        .map(|path| design_doc_has_bounded_file_set(std::path::Path::new(path)).unwrap_or(false))
        .unwrap_or(false);
    let skip_design_override = task.issue_type == "defect"
        || !task.planner_metadata.owned_paths.is_empty()
        || (task.issue_type == "runtime_defect"
            && design_doc_has_bounded_scope
            && request_has_explicit_implementation_or_repair_terms(&request_text));
    set_dispatch_init_timeout_stage(timeout_stage, "seed_existing_task_derive_seeded_status");
    let payload = derive_seeded_run_graph_state_with_stage(
        store,
        task_id,
        &request_text,
        timeout_stage,
        skip_design_override,
        persist_launcher_snapshot,
    )
    .await?;
    Ok(Some(payload))
}

async fn preview_run_graph_dispatch_init_artifacts(
    store: &StateStore,
    run_id: &str,
    timeout_stage: Option<&std::sync::Arc<std::sync::Mutex<&'static str>>>,
    persist_launcher_snapshot: bool,
) -> Result<RunGraphDispatchInitPreview, String> {
    set_dispatch_init_timeout_stage(timeout_stage, "read_existing_routed_dispatch_artifacts");
    if let Some(artifacts) = existing_routed_dispatch_init_artifacts_with_persistence(
        store,
        run_id,
        run_id,
        persist_launcher_snapshot,
    )
    .await?
    {
        return Ok(RunGraphDispatchInitPreview::Existing(artifacts));
    }

    set_dispatch_init_timeout_stage(timeout_stage, "reseed_explicit_task_graph_binding");
    let effective_run_id = if let Some(bound_run_id) =
        reseed_explicit_task_graph_binding_for_dispatch_init(
            store,
            run_id,
            persist_launcher_snapshot,
        )
        .await?
    {
        set_dispatch_init_timeout_stage(timeout_stage, "read_bound_routed_dispatch_artifacts");
        if let Some(artifacts) = existing_routed_dispatch_init_artifacts_with_persistence(
            store,
            run_id,
            &bound_run_id,
            persist_launcher_snapshot,
        )
        .await?
        {
            return Ok(RunGraphDispatchInitPreview::Existing(artifacts));
        }
        bound_run_id
    } else {
        run_id.to_string()
    };
    let mut seed_payload = None;
    set_dispatch_init_timeout_stage(timeout_stage, "read_or_seed_run_graph_status");
    let status = match store.run_graph_status(&effective_run_id).await {
        Ok(status) => status,
        Err(error) => {
            let original_error = format!(
                "Failed to read run-graph state for `{}`: {error}",
                effective_run_id
            );
            match seed_existing_task_payload_for_dispatch_init(
                store,
                &effective_run_id,
                timeout_stage,
                persist_launcher_snapshot,
            )
            .await?
            {
                Some(payload) => {
                    let status = payload.status.clone();
                    seed_payload = Some(payload);
                    status
                }
                None => return Err(original_error),
            }
        }
    };
    set_dispatch_init_timeout_stage(timeout_stage, "reconcile_active_exception_status");
    let mut status = reconcile_dispatch_init_state_for_active_exception(store, status).await?;
    if run_graph_state_has_configured_dev_team_route_mismatch_with_persistence(
        store,
        &status,
        persist_launcher_snapshot,
    )
    .await?
    {
        set_dispatch_init_timeout_stage(timeout_stage, "reseed_configured_dev_team_route_mismatch");
        if let Some(payload) = seed_existing_task_payload_for_dispatch_init(
            store,
            &effective_run_id,
            timeout_stage,
            persist_launcher_snapshot,
        )
        .await?
        {
            status =
                reconcile_dispatch_init_state_for_active_exception(store, payload.status.clone())
                    .await?;
            seed_payload = Some(payload);
        }
    }

    let context = if let Some(payload) = seed_payload.as_ref() {
        run_graph_dispatch_context_from_seed_payload(payload)
    } else {
        set_dispatch_init_timeout_stage(timeout_stage, "read_persisted_dispatch_context");
        let mut persisted = store
            .run_graph_dispatch_context(&effective_run_id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to read persisted seeded dispatch context while preparing dispatch-init preview: {error}"
                )
            })?;
        if persisted.is_none()
            && status.active_node == "planning"
            && status.resume_target.starts_with("dispatch.")
        {
            if let Some(payload) = seed_existing_task_payload_for_dispatch_init(
                store,
                &effective_run_id,
                timeout_stage,
                persist_launcher_snapshot,
            )
            .await?
            {
                status = reconcile_dispatch_init_state_for_active_exception(
                    store,
                    payload.status.clone(),
                )
                .await?;
                persisted = Some(run_graph_dispatch_context_from_seed_payload(&payload));
                seed_payload = Some(payload);
            }
        }
        persisted.ok_or_else(|| {
            format!(
                "No persisted seeded dispatch context exists for run_id `{}`; reseed the run with request text before dispatch-init.",
                effective_run_id
            )
        })?
    };

    set_dispatch_init_timeout_stage(timeout_stage, "decode_role_selection");
    let mut role_selection = rehydrate_dispatch_context_role_selection_with_persistence(
        store,
        &context,
        persist_launcher_snapshot,
    )
    .await?;
    set_dispatch_init_timeout_stage(timeout_stage, "check_route_assignment_catalog_drift");
    let route_assignment_drift =
        match dispatch_context_configured_dev_team_route_drift_with_persistence(
            store,
            &effective_run_id,
            &role_selection,
            timeout_stage,
            persist_launcher_snapshot,
        )
        .await?
        {
            Some(drift) => Some(drift),
            None => dispatch_context_route_assignment_catalog_drift(store.root(), &role_selection),
        };
    if route_assignment_drift.is_some() {
        set_dispatch_init_timeout_stage(timeout_stage, "reseed_route_assignment_drift");
        let payload = reseed_dispatch_context_after_route_assignment_drift_with_persistence(
            store,
            &status,
            &context,
            persist_launcher_snapshot,
        )
        .await?;
        status = reconcile_dispatch_init_state_for_active_exception(store, payload.status.clone())
            .await?;
        role_selection = payload.role_selection.clone();
        seed_payload = Some(payload);
    }
    let mut existing_dispatch_receipt = if route_assignment_drift.is_none() {
        set_dispatch_init_timeout_stage(timeout_stage, "read_existing_dispatch_receipt");
        store
            .run_graph_dispatch_receipt(&effective_run_id)
            .await
            .map_err(|error| format!("Failed to read existing dispatch receipt: {error}"))?
            .filter(|receipt| {
                dispatch_receipt_disabled_external_backend_drift(store.root(), receipt).is_none()
            })
    } else {
        None
    };
    let stale_seeded_packet = if let Some(receipt) = existing_dispatch_receipt.as_ref() {
        !existing_dispatch_receipt_matches_current_seed_with_persistence(
            store,
            &effective_run_id,
            receipt,
            timeout_stage,
            persist_launcher_snapshot,
        )
        .await?
    } else {
        false
    };
    if stale_seeded_packet {
        set_dispatch_init_timeout_stage(timeout_stage, "reseed_stale_dispatch_packet");
        let payload = reseed_dispatch_context_after_route_assignment_drift_with_persistence(
            store,
            &status,
            &context,
            persist_launcher_snapshot,
        )
        .await?;
        status = reconcile_dispatch_init_state_for_active_exception(store, payload.status.clone())
            .await?;
        role_selection = payload.role_selection.clone();
        seed_payload = Some(payload);
        existing_dispatch_receipt = None;
    }
    if route_assignment_drift.is_none() {
        status = reconcile_dispatch_init_state_for_missing_receipt(
            status,
            &role_selection,
            existing_dispatch_receipt.is_some(),
        );
    }
    set_dispatch_init_timeout_stage(timeout_stage, "build_run_graph_dispatch_bootstrap");
    let run_graph_bootstrap = run_graph_dispatch_bootstrap_from_state(&status)?;
    let runtime_bundle =
        crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload_with_persistence(
            store,
            persist_launcher_snapshot,
        )
        .await
        .ok();
    let assignment_bundle = runtime_bundle
        .as_ref()
        .map(|bundle| bundle.activation_bundle.clone())
        .unwrap_or_else(|| role_selection.compiled_bundle.clone());
    crate::apply_run_graph_runtime_assignment_to_selection(
        &mut role_selection,
        &assignment_bundle,
        &run_graph_bootstrap,
        "run-graph dispatch-init execution_plan is not an object",
    )?;
    set_dispatch_init_timeout_stage(timeout_stage, "build_taskflow_handoff_plan");
    let taskflow_handoff_plan = crate::build_taskflow_handoff_plan(&role_selection);
    if route_assignment_drift.is_none() {
        if let Some(existing_receipt) = existing_dispatch_receipt.as_ref() {
            if let Some(dispatch_packet_path) = reusable_routed_dispatch_receipt(&existing_receipt)
            {
                return Ok(RunGraphDispatchInitPreview::Existing(
                    RunGraphDispatchInitArtifacts {
                        requested_run_id: run_id.to_string(),
                        run_id: effective_run_id,
                        role_selection,
                        run_graph_bootstrap,
                        taskflow_handoff_plan,
                        dispatch_receipt: existing_receipt.clone(),
                        dispatch_packet_path,
                    },
                ));
            }
        }
    }
    let task_identity = if let Ok(task) = store.show_task(&effective_run_id).await {
        inject_task_planner_metadata(&mut role_selection, &task.planner_metadata);
        preview_dispatch_init_task_identity(store, &effective_run_id).await?
    } else {
        None
    };
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    )?;
    crate::runtime_dispatch_state::sync_receipt_configured_activation_assignment(
        &role_selection,
        &mut dispatch_receipt,
    );
    set_dispatch_init_timeout_stage(timeout_stage, "build_dispatch_command");
    dispatch_receipt.dispatch_command = crate::runtime_dispatch_command_for_target(
        &role_selection,
        &dispatch_receipt.dispatch_target,
    );
    set_dispatch_init_timeout_stage(timeout_stage, "refresh_downstream_dispatch_preview");
    crate::refresh_downstream_dispatch_preview(
        store,
        &role_selection,
        &run_graph_bootstrap,
        &mut dispatch_receipt,
    )
    .await?;
    set_dispatch_init_timeout_stage(timeout_stage, "write_runtime_dispatch_packet");
    let ctx = crate::RuntimeDispatchPacketContext::new(
        store.root(),
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    );
    let dispatch_packet_path = crate::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    set_dispatch_init_timeout_stage(timeout_stage, "read_dispatch_command_from_packet");
    dispatch_receipt.dispatch_command = dispatch_command_from_packet_path(&dispatch_packet_path)?;
    Ok(RunGraphDispatchInitPreview::Prepared(
        PreparedRunGraphDispatchInit {
            requested_run_id: run_id.to_string(),
            run_id: effective_run_id,
            status,
            role_selection,
            run_graph_bootstrap,
            taskflow_handoff_plan,
            dispatch_receipt,
            dispatch_packet_path,
            seed_payload,
            task_identity,
        },
    ))
}

async fn commit_previewed_run_graph_dispatch_init_artifacts(
    state_dir: &std::path::Path,
    preview: RunGraphDispatchInitPreview,
) -> Result<serde_json::Value, String> {
    match preview {
        RunGraphDispatchInitPreview::Existing(artifacts) => {
            let requested_run_id = artifacts.requested_run_id.clone();
            let run_id = artifacts.run_id.clone();
            let payload = artifacts.into_json_payload();
            try_backfill_dispatch_init_task_identity(state_dir, &run_id).await;
            write_run_graph_dispatch_init_fast_cache(
                state_dir,
                &requested_run_id,
                &run_id,
                &payload,
            )?;
            Ok(payload)
        }
        RunGraphDispatchInitPreview::Prepared(prepared) => {
            let artifacts = RunGraphDispatchInitArtifacts {
                requested_run_id: prepared.requested_run_id.clone(),
                run_id: prepared.run_id.clone(),
                role_selection: prepared.role_selection.clone(),
                run_graph_bootstrap: prepared.run_graph_bootstrap.clone(),
                taskflow_handoff_plan: prepared.taskflow_handoff_plan.clone(),
                dispatch_receipt: prepared.dispatch_receipt.clone(),
                dispatch_packet_path: prepared.dispatch_packet_path.clone(),
            };
            let requested_run_id = artifacts.requested_run_id.clone();
            let run_id = artifacts.run_id.clone();
            let mut payload = artifacts.into_json_payload();

            let authoritative_commit = async {
                let store = StateStore::open_existing(state_dir.to_path_buf())
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to reopen authoritative state store for dispatch-init commit: {error}"
                        )
                    })?;
                if let Some(seed_payload) = prepared.seed_payload.as_ref() {
                    persist_seed_artifacts(&store, seed_payload).await?;
                }
                store
                    .record_run_graph_status(&prepared.status)
                    .await
                    .map_err(|error| {
                        format!("Failed to refresh run-graph status for dispatch-init: {error}")
                    })?;
                store
                    .record_run_graph_dispatch_receipt(&prepared.dispatch_receipt)
                    .await
                    .map_err(|error| {
                        format!("Failed to record seeded dispatch receipt: {error}")
                    })?;
                ensure_dispatch_init_task_identity(&store, &prepared.run_id).await?;
                crate::taskflow_continuation::sync_run_graph_continuation_binding(
                    &store,
                    &prepared.status,
                    "run_graph_dispatch_init",
                )
                .await?;
                store.close().await;
                Ok::<(), String>(())
            };
            match authoritative_commit.await {
                Ok(()) => {
                    payload["authoritative_persistence"] =
                        serde_json::json!({"status": "recorded"});
                }
                Err(error) if StateStore::message_is_lock_contention(&error) => {
                    payload["authoritative_persistence"] = serde_json::json!({
                        "status": "deferred_lock_contention",
                        "reason": format!("dispatch-init fast-cache receipt was not written because authoritative state-store commit could not acquire the datastore lock within the bounded operator window: {error}"),
                        "retry_surface": "vida taskflow run-graph dispatch-init"
                    });
                }
                Err(error) => return Err(error),
            }
            write_run_graph_dispatch_init_fast_cache(
                state_dir,
                &requested_run_id,
                &run_id,
                &payload,
            )?;
            Ok(payload)
        }
    }
}

pub(crate) async fn prepare_run_graph_dispatch_init_artifacts(
    store: &StateStore,
    run_id: &str,
) -> Result<RunGraphDispatchInitArtifacts, String> {
    match preview_run_graph_dispatch_init_artifacts(store, run_id, None, true).await? {
        RunGraphDispatchInitPreview::Existing(artifacts) => Ok(artifacts),
        RunGraphDispatchInitPreview::Prepared(prepared) => {
            if let Some(seed_payload) = prepared.seed_payload.as_ref() {
                persist_seed_artifacts(store, seed_payload).await?;
            }
            store
                .record_run_graph_status(&prepared.status)
                .await
                .map_err(|error| {
                    format!("Failed to refresh run-graph status for dispatch-init: {error}")
                })?;
            store
                .record_run_graph_dispatch_receipt(&prepared.dispatch_receipt)
                .await
                .map_err(|error| format!("Failed to record seeded dispatch receipt: {error}"))?;
            ensure_dispatch_init_task_identity(store, &prepared.run_id).await?;
            crate::taskflow_continuation::sync_run_graph_continuation_binding(
                store,
                &prepared.status,
                "run_graph_dispatch_init",
            )
            .await?;
            Ok(RunGraphDispatchInitArtifacts {
                requested_run_id: prepared.requested_run_id,
                run_id: prepared.run_id,
                role_selection: prepared.role_selection,
                run_graph_bootstrap: prepared.run_graph_bootstrap,
                taskflow_handoff_plan: prepared.taskflow_handoff_plan,
                dispatch_receipt: prepared.dispatch_receipt,
                dispatch_packet_path: prepared.dispatch_packet_path,
            })
        }
    }
}

pub(crate) async fn run_graph_dispatch_init_from_state_dir(
    state_dir: &std::path::Path,
    run_id: &str,
) -> Result<serde_json::Value, String> {
    let timeout_stage = std::sync::Arc::new(std::sync::Mutex::new("open_read_only_state_store"));
    let timeout_stage_for_task = timeout_stage.clone();
    match tokio::time::timeout(
        std::time::Duration::from_secs(RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS),
        async {
            *timeout_stage_for_task
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = "open_read_only_state_store";
            let store = StateStore::open_existing_read_only(state_dir.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to open read-only state store for dispatch-init preparation: {error}"
                    )
                })?;
            *timeout_stage_for_task
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = "preview_dispatch_init_artifacts";
            let preview = preview_run_graph_dispatch_init_artifacts(
                &store,
                run_id,
                Some(&timeout_stage_for_task),
                false,
            )
            .await?;
            *timeout_stage_for_task
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = "close_read_only_state_store";
            store.close().await;
            *timeout_stage_for_task
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                "commit_previewed_dispatch_init_artifacts";
            commit_previewed_run_graph_dispatch_init_artifacts(state_dir, preview).await
        },
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let _ = record_dispatch_init_timeout_issue_from_state_dir_bounded(state_dir, run_id)
                .await;
            let stage = *timeout_stage
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Err(run_graph_dispatch_init_timeout_message(run_id, stage))
        }
    }
}

async fn run_graph_dispatch_init(
    store: &StateStore,
    run_id: &str,
) -> Result<serde_json::Value, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS),
        prepare_run_graph_dispatch_init_artifacts(store, run_id),
    )
    .await
    {
        Ok(result) => result.and_then(|artifacts| {
            let requested_run_id = artifacts.requested_run_id.clone();
            let run_id = artifacts.run_id.clone();
            let payload = artifacts.into_json_payload();
            write_run_graph_dispatch_init_fast_cache(
                store.root(),
                &requested_run_id,
                &run_id,
                &payload,
            )?;
            Ok(payload)
        }),
        Err(_) => {
            let _ = record_dispatch_init_timeout_issue_bounded(store, run_id).await;
            Err(run_graph_dispatch_init_timeout_message(
                run_id,
                "prepare_dispatch_init_artifacts",
            ))
        }
    }
}

pub(crate) async fn derive_advanced_run_graph_state(
    store: &StateStore,
    existing: RunGraphStatus,
) -> Result<TaskflowRunGraphAdvancePayload, String> {
    derive_advanced_run_graph_state_with_persistence(store, existing, true).await
}

pub(crate) async fn derive_advanced_run_graph_state_read_only(
    store: &StateStore,
    existing: RunGraphStatus,
) -> Result<TaskflowRunGraphAdvancePayload, String> {
    derive_advanced_run_graph_state_with_persistence(store, existing, false).await
}

pub(crate) async fn derive_advanced_run_graph_state_with_persistence(
    store: &StateStore,
    existing: RunGraphStatus,
    persist_launcher_snapshot: bool,
) -> Result<TaskflowRunGraphAdvancePayload, String> {
    let compiled_control =
        compiled_run_graph_control_with_persistence(store, persist_launcher_snapshot).await?;
    let implementation = compiled_control.implementation;
    let compiled_route_uses_seeded_sequence = implementation["team_flow_selected_node_id"]
        .as_str()
        .is_none_or(|value| value.trim().is_empty());
    let seeded_lane_sequence = if compiled_route_uses_seeded_sequence
        && existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
    {
        seeded_implementation_lane_sequence_with_persistence(
            store,
            &existing.run_id,
            persist_launcher_snapshot,
        )
        .await?
    } else {
        None
    };
    let dispatch_receipt = store
        .run_graph_dispatch_receipt(&existing.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read run-graph dispatch receipt for `{}` before advance: {error}",
                existing.run_id
            )
        })?;
    if active_exception_takeover_receipt_matches_snapshot(&existing, dispatch_receipt.as_ref()) {
        return Err(format!(
            "run-graph advance blocked: run `{}` is in active exception takeover for `{}`; finish the scoped local work allowed by `vida lane takeover-ready {}`, then close the bounded task before advancing another runtime lane.",
            existing.run_id, existing.active_node, existing.run_id
        ));
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "planning"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let analysis_node = json_raw_string_field(&implementation, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "analysis".to_string());
        let direct_writer_entry = compiled_control.entry_execution_node_id.clone();
        if let Some(sequence) = seeded_lane_sequence.as_ref() {
            let expected_entry = sequence
                .first()
                .expect("seeded lane sequence should be non-empty");
            let actual_entry = existing.next_node.as_deref().unwrap_or("none");
            if actual_entry != expected_entry.node_id {
                return Err(format!(
                    "run-graph advance expected configured execution node `{}`, got `{actual_entry}`",
                    expected_entry.node_id
                ));
            }
            let active_entry = expected_entry.node_id.clone();
            let next_node = next_seeded_implementation_lane(sequence, &active_entry);
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_state_from_authority_ready_transition(
                    &existing,
                    active_entry.clone(),
                    next_node.clone(),
                    expected_entry.lane_id.clone(),
                    format!("{active_entry}_active"),
                    "not_required".to_string(),
                    "execution_cursor".to_string(),
                    DispatchTargetFormat::Lane,
                    next_node.is_some(),
                ),
            });
        }
        let configured_writer_entry = implementation["team_flow_selected_node_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| implementation_writer_node(&implementation));
        if existing.next_node.as_deref() == Some(direct_writer_entry.as_str())
            || existing.next_node.as_deref() == Some(configured_writer_entry.as_str())
        {
            let active_entry = existing
                .next_node
                .clone()
                .unwrap_or_else(|| direct_writer_entry.clone());
            let coach_required =
                json_bool_field(&implementation, "coach_required").unwrap_or(false);
            let verification = compiled_control.verification.clone();
            let (next_node, policy_gate) =
                implementation_verification_gate(&implementation, &verification);
            let active_lane_id = existing.lane_id.clone();
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_state_from_authority_ready_transition(
                    &existing,
                    active_entry.clone(),
                    if coach_required {
                        json_raw_string_field(&implementation, "coach_route_task_class")
                            .filter(|value| !value.is_empty())
                            .or(next_node)
                    } else {
                        next_node
                    },
                    active_lane_id,
                    format!("{active_entry}_active"),
                    policy_gate,
                    "execution_cursor".to_string(),
                    DispatchTargetFormat::Lane,
                    true,
                ),
            });
        }

        if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{analysis_node}`, `{direct_writer_entry}`, or `{configured_writer_entry}` for the seeded implementation run, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_state_from_authority_ready_transition(
                &existing,
                analysis_node.clone(),
                next_node,
                format!("{analysis_node}_lane"),
                "analysis_active".to_string(),
                policy_gate,
                "execution_cursor".to_string(),
                DispatchTargetFormat::Lane,
                recovery_ready,
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "analysis"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        if existing.next_node.is_none() {
            let (next_node, policy_gate, recovery_ready) =
                implementation_analysis_gate(&implementation);
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_state_from_authority_ready_transition(
                    &existing,
                    existing.active_node.clone(),
                    next_node,
                    existing.lane_id.clone(),
                    "analysis_active".to_string(),
                    policy_gate,
                    "execution_cursor".to_string(),
                    DispatchTargetFormat::Lane,
                    recovery_ready,
                ),
            });
        }

        let writer_node = implementation_writer_node(&implementation);
        if existing.next_node.as_deref() != Some(writer_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{writer_node}` for the implementation analysis handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let coach_required = json_bool_field(&implementation, "coach_required").unwrap_or(false);
        let verification = compiled_control.verification.clone();
        let (next_node, policy_gate) =
            implementation_verification_gate(&implementation, &verification);
        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_state_from_authority_ready_transition(
                &existing,
                writer_node.clone(),
                if coach_required {
                    json_raw_string_field(&implementation, "coach_route_task_class")
                        .filter(|value| !value.is_empty())
                        .or(next_node)
                } else {
                    next_node
                },
                format!("{writer_node}_lane"),
                "writer_active".to_string(),
                policy_gate,
                "execution_cursor".to_string(),
                DispatchTargetFormat::Lane,
                true,
            ),
        });
    }

    if let Some(sequence) = seeded_lane_sequence.as_ref() {
        if sequence
            .iter()
            .any(|node| node.node_id == existing.active_node)
        {
            if existing.lifecycle_stage.ends_with("_dispatch_ready")
                && existing.next_node.as_deref() == Some(existing.active_node.as_str())
            {
                ensure_configured_lane_advance_allowed(&existing, "dispatch_ready")?;
                let next_node = next_seeded_implementation_lane(sequence, &existing.active_node);
                return Ok(TaskflowRunGraphAdvancePayload {
                    status: run_graph_state_from_authority_ready_transition(
                        &existing,
                        existing.active_node.clone(),
                        next_node.clone(),
                        sequence
                            .iter()
                            .find(|node| node.node_id == existing.active_node)
                            .map(|node| node.lane_id.clone())
                            .unwrap_or_else(|| existing.lane_id.clone()),
                        format!("{}_active", existing.active_node),
                        "not_required".to_string(),
                        "execution_cursor".to_string(),
                        DispatchTargetFormat::Lane,
                        next_node.is_some(),
                    ),
                });
            }
            let expected_next_node =
                next_seeded_implementation_lane(sequence, &existing.active_node);
            match (existing.next_node.as_deref(), expected_next_node.as_deref()) {
                (None, None) => {
                    ensure_configured_lane_advance_allowed(&existing, "complete")?;
                    let mut status = run_graph_state_from_authority_ready_transition(
                        &existing,
                        existing.active_node.clone(),
                        None,
                        existing.lane_id.clone(),
                        "implementation_complete".to_string(),
                        "not_required".to_string(),
                        existing.checkpoint_kind.clone(),
                        DispatchTargetFormat::Lane,
                        false,
                    );
                    status.status = "completed".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                (Some(actual), Some(expected)) if actual == expected => {
                    ensure_configured_lane_advance_allowed(&existing, "handoff")?;
                    let next_node = next_seeded_implementation_lane(sequence, actual);
                    return Ok(TaskflowRunGraphAdvancePayload {
                        status: run_graph_state_from_authority_ready_transition(
                            &existing,
                            actual.to_string(),
                            next_node.clone(),
                            sequence
                                .iter()
                                .find(|node| node.node_id == actual)
                                .map(|node| node.lane_id.clone())
                                .unwrap_or_else(|| existing.lane_id.clone()),
                            format!("{actual}_active"),
                            "not_required".to_string(),
                            "execution_cursor".to_string(),
                            DispatchTargetFormat::Lane,
                            next_node.is_some(),
                        ),
                    });
                }
                (actual, expected) => {
                    return Err(format!(
                        "run-graph advance expected configured execution lane `{}` after active node `{}`, got `{}`",
                        expected.unwrap_or("none"),
                        existing.active_node,
                        actual.unwrap_or("none")
                    ));
                }
            }
        }
    }

    let writer_node = implementation_writer_node(&implementation);
    let direct_writer_entry = compiled_control.entry_execution_node_id.clone();
    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && (existing.active_node == writer_node || existing.active_node == direct_writer_entry)
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let verification = compiled_control.verification.clone();
        let (active_node, next_node, policy_gate, target_format, recovery_ready) =
            implementation_writer_handoff(&implementation, &verification);
        if existing.next_node.as_deref() != Some(active_node.as_str())
            && existing.next_node.is_some()
        {
            return Err(format!(
                "run-graph advance expected next node `{active_node}` for the implementation writer handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        if active_node == existing.active_node && next_node.is_none() {
            let mut status = run_graph_state_from_authority_ready_transition(
                &existing,
                existing.active_node.clone(),
                None,
                existing.lane_id.clone(),
                "implementation_complete".to_string(),
                "not_required".to_string(),
                existing.checkpoint_kind.clone(),
                DispatchTargetFormat::Lane,
                false,
            );
            status.status = "completed".to_string();
            status.context_state = existing.context_state;
            return Ok(TaskflowRunGraphAdvancePayload { status });
        }

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_state_from_authority_ready_transition(
                &existing,
                active_node.clone(),
                next_node,
                format!("{active_node}_lane"),
                format!("{active_node}_active"),
                policy_gate,
                "execution_cursor".to_string(),
                target_format,
                recovery_ready,
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == "coach"
    {
        if implementation.is_null() {
            return Err(
                "run-graph advance failed: implementation route is unavailable in the compiled activation snapshot."
                    .to_string(),
            );
        }

        let verification_node =
            json_raw_string_field(&implementation, "verification_route_task_class")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "verification".to_string());
        if existing.next_node.as_deref() == Some("review_ensemble") && existing.status == "ready" {
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_state_from_authority_ready_transition(
                    &existing,
                    "review_ensemble".to_string(),
                    Some(verification_node),
                    "review_ensemble_lane".to_string(),
                    "review_ensemble_active".to_string(),
                    "review_findings".to_string(),
                    "execution_cursor".to_string(),
                    DispatchTargetFormat::Direct,
                    true,
                ),
            });
        }
        if existing.next_node.as_deref() != Some(verification_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{verification_node}` for the implementation coach handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        match implementation_verification_outcome(existing.status.as_str()) {
            ImplementationVerificationOutcome::Clean
            | ImplementationVerificationOutcome::Approved => {}
            ImplementationVerificationOutcome::ReworkReady
            | ImplementationVerificationOutcome::FindingsBlocked => {
                return Err(format!(
                    "run-graph advance blocked: coach review requires developer_rework before verification; got status `{}`",
                    existing.status
                ));
            }
            ImplementationVerificationOutcome::UnexpectedStatus => {
                return Err(format!(
                    "run-graph advance expected coach status `clean` or `approved` before verification handoff, got `{}`",
                    existing.status
                ));
            }
        }

        let verification = compiled_control.verification.clone();

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_state_from_authority_ready_transition(
                &existing,
                verification_node.clone(),
                None,
                format!("{verification_node}_lane"),
                format!("{verification_node}_active"),
                json_raw_string_field(&verification, "verification_gate")
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| existing.policy_gate.clone()),
                "execution_cursor".to_string(),
                DispatchTargetFormat::Lane,
                false,
            ),
        });
    }

    if existing.task_class == "implementation" && existing.route_task_class == "implementation" {
        let verification_node =
            json_raw_string_field(&implementation, "verification_route_task_class")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "verification".to_string());
        if existing.active_node != verification_node {
            // fall through
        } else {
            match implementation_verification_outcome(existing.status.as_str()) {
                ImplementationVerificationOutcome::ReworkReady => {
                    let analysis_node =
                        json_raw_string_field(&implementation, "analysis_route_task_class")
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| "analysis".to_string());
                    if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
                        return Err(format!(
                            "run-graph advance expected next node `{analysis_node}` for the explicit review rework loop, got `{}`",
                            existing.next_node.as_deref().unwrap_or("none")
                        ));
                    }

                    let (next_node, policy_gate, recovery_ready) =
                        implementation_analysis_gate(&implementation);

                    return Ok(TaskflowRunGraphAdvancePayload {
                        status: run_graph_state_from_authority_ready_transition(
                            &existing,
                            analysis_node.clone(),
                            next_node,
                            format!("{analysis_node}_lane"),
                            "analysis_active".to_string(),
                            policy_gate,
                            "execution_cursor".to_string(),
                            DispatchTargetFormat::Lane,
                            recovery_ready,
                        ),
                    });
                }
                ImplementationVerificationOutcome::Clean => {
                    let mut status = run_graph_state_from_authority_ready_transition(
                        &existing,
                        existing.active_node.clone(),
                        Some("approval".to_string()),
                        existing.lane_id.clone(),
                        "approval_wait".to_string(),
                        crate::release1_contracts::ApprovalStatus::ApprovalRequired
                            .as_str()
                            .to_string(),
                        existing.checkpoint_kind.clone(),
                        DispatchTargetFormat::Direct,
                        true,
                    );
                    status.status = "awaiting_approval".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                ImplementationVerificationOutcome::Approved => {
                    let mut status = run_graph_state_from_authority_ready_transition(
                        &existing,
                        existing.active_node.clone(),
                        None,
                        existing.lane_id.clone(),
                        "implementation_complete".to_string(),
                        "not_required".to_string(),
                        existing.checkpoint_kind.clone(),
                        DispatchTargetFormat::Lane,
                        false,
                    );
                    status.status = "completed".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                ImplementationVerificationOutcome::FindingsBlocked => {
                    return Err(format!(
                        "run-graph advance blocked: implementation review findings require explicit scope/rework resolution before completion; got status `{}`",
                        existing.status
                    ));
                }
                ImplementationVerificationOutcome::UnexpectedStatus => {
                    return Err(format!(
                        "run-graph advance expected `{verification_node}` status `clean` to enter approval wait or `approved` to complete implementation, got `{}`",
                        existing.status
                    ));
                }
            }
        }
    }

    if matches!(
        existing.task_class.as_str(),
        "scope_discussion" | "pbi_discussion"
    ) && existing.active_node == "planning"
    {
        let analyst_node = existing
            .next_node
            .clone()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "run-graph advance expected a seeded conversational next node, got `none`"
                    .to_string()
            })?;
        if existing.route_task_class.is_empty() || existing.route_task_class == existing.task_class
        {
            return Err(format!(
                "run-graph advance expected a seeded conversational route target for `{}`, got `{}`",
                existing.task_class, existing.route_task_class
            ));
        }
        let route_target = existing.route_task_class.clone();
        let next_node = Some(route_target.clone());

        return Ok(TaskflowRunGraphAdvancePayload {
            status: {
                let mut status = run_graph_state_from_authority_ready_transition(
                    &existing,
                    analyst_node.clone(),
                    next_node.clone(),
                    format!("{analyst_node}_lane"),
                    "conversation_active".to_string(),
                    existing.policy_gate.clone(),
                    "conversation_cursor".to_string(),
                    DispatchTargetFormat::Lane,
                    true,
                );
                status.handoff_state = format!("awaiting_{route_target}");
                status.resume_target = format!("dispatch.{route_target}");
                status
            },
        });
    }

    Err(format!(
        "run-graph advance currently supports only seeded implementation, scope-discussion, or pbi-discussion runs; got class={} route={} node={}",
        existing.task_class, existing.route_task_class, existing.active_node
    ))
}

async fn run_taskflow_run_graph_dispatch_init_mutation(
    state_dir: &std::path::Path,
    run_id: &str,
    as_json: bool,
) -> ExitCode {
    match run_graph_dispatch_init_from_state_dir(state_dir, run_id).await {
        Ok(payload) => {
            if as_json {
                crate::print_json_pretty(&payload);
            } else {
                print_surface_header(RenderMode::Plain, "vida taskflow run-graph dispatch-init");
                print_surface_line(RenderMode::Plain, "run", run_id);
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_packet",
                    payload["dispatch_packet_path"].as_str().unwrap_or("none"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_target",
                    payload["dispatch_receipt"]["dispatch_target"]
                        .as_str()
                        .unwrap_or("none"),
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if as_json {
                print_run_graph_json_error(
                    "vida taskflow run-graph dispatch-init",
                    run_id,
                    &error,
                    run_graph_dispatch_init_error_evidence(&error),
                );
            } else {
                eprintln!("{error}");
            }
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_taskflow_run_graph_mutation(args: &[String]) -> ExitCode {
    let state_dir = proxy_state_dir();
    if matches!(
        args.get(1).map(String::as_str),
        Some("seed" | "advance" | "dispatch-init" | "init" | "update")
    ) && !crate::taskflow_runtime::taskflow_dispatch_enabled_for_state_root(&state_dir)
    {
        crate::print_json_pretty(&crate::taskflow_runtime::dispatch_runtime_disabled_payload(
            "vida taskflow run-graph",
            crate::taskflow_runtime::TaskRuntimeMode::ManagementOnly,
        ));
        return ExitCode::from(1);
    }
    match args {
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            if let Some(payload) =
                read_run_graph_dispatch_init_fast_cache_for_dispatch_init(&state_dir, run_id).await
            {
                try_backfill_dispatch_init_task_identity(&state_dir, run_id).await;
                crate::print_json_pretty(&payload);
                return ExitCode::SUCCESS;
            }
            return run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, true).await;
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            if let Some(payload) =
                read_run_graph_dispatch_init_fast_cache_for_dispatch_init(&state_dir, run_id).await
            {
                try_backfill_dispatch_init_task_identity(&state_dir, run_id).await;
                print_surface_header(RenderMode::Plain, "vida taskflow run-graph dispatch-init");
                print_surface_line(RenderMode::Plain, "run", run_id);
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_packet",
                    payload["dispatch_packet_path"].as_str().unwrap_or("none"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "dispatch_target",
                    payload["dispatch_receipt"]["dispatch_target"]
                        .as_str()
                        .unwrap_or("none"),
                );
                return ExitCode::SUCCESS;
            }
            return run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, false).await;
        }
        _ => {}
    }
    let store = match StateStore::open(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match args {
        [head, subcommand, task_id] if head == "run-graph" && subcommand == "advance" => {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let payload = match derive_advanced_run_graph_state(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            match store.record_run_graph_status(&payload.status).await {
                Ok(()) => {
                    if let Err(error) =
                        persist_selected_node_for_run_graph_transition(&store, &payload.status)
                            .await
                    {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &payload.status,
                            "run_graph_advance",
                        )
                        .await
                    {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                    print_surface_header(RenderMode::Plain, "vida taskflow run-graph advance");
                    print_surface_line(RenderMode::Plain, "run", task_id);
                    print_surface_line(
                        RenderMode::Plain,
                        "active node",
                        &payload.status.active_node,
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "next node",
                        payload.status.next_node.as_deref().unwrap_or("none"),
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "delegation gate",
                        &payload.status.delegation_gate().as_display(),
                    );
                    store.close().await;
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to advance run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, flag]
            if head == "run-graph" && subcommand == "advance" && flag == "--json" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(error) => {
                    let message = format!("Failed to read existing run-graph state: {error}");
                    eprintln!("{message}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &message,
                        None,
                    );
                    return ExitCode::from(1);
                }
            };
            let blocker_run_id = existing.run_id.clone();
            let blocker_active_node = existing.active_node.clone();
            let blocker_status = existing.status.clone();
            let blocker_route_task_class = existing.route_task_class.clone();
            let blocker_policy_gate = existing.policy_gate.clone();
            let blocker_resume_target = existing.resume_target.clone();
            let blocker_next_node = existing.next_node.clone();
            let payload = match derive_advanced_run_graph_state(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    let evidence = match run_graph_issue_evidence(RunGraphBlockerEvidenceArgs {
                        run_id: &blocker_run_id,
                        active_node: &blocker_active_node,
                        status: &blocker_status,
                        route_task_class: &blocker_route_task_class,
                        policy_gate: &blocker_policy_gate,
                        resume_target: &blocker_resume_target,
                        next_node: blocker_next_node.as_deref(),
                        error: &error,
                    }) {
                        Ok(evidence) => evidence,
                        Err(guard_error) => {
                            eprintln!("{guard_error}");
                            print_run_graph_json_error(
                                "vida taskflow run-graph advance",
                                task_id,
                                &guard_error,
                                None,
                            );
                            return ExitCode::from(1);
                        }
                    };
                    eprintln!("{error}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &error,
                        evidence,
                    );
                    return ExitCode::from(1);
                }
            };
            match store.record_run_graph_status(&payload.status).await {
                Ok(()) => {
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &payload.status,
                            "run_graph_advance",
                        )
                        .await
                    {
                        let message = format!(
                            "Failed to synchronize continuation binding after advance: {error}"
                        );
                        eprintln!("{message}");
                        print_run_graph_json_error(
                            "vida taskflow run-graph advance",
                            task_id,
                            &message,
                            None,
                        );
                        return ExitCode::from(1);
                    }
                    let delegation_gate = payload.status.delegation_gate();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow run-graph advance",
                            "run_id": task_id,
                            "payload": payload,
                            "delegation_gate": delegation_gate,
                        }))
                        .expect("run-graph advance should render as json")
                    );
                    store.close().await;
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    let message = format!("Failed to advance run-graph state: {error}");
                    eprintln!("{message}");
                    print_run_graph_json_error(
                        "vida taskflow run-graph advance",
                        task_id,
                        &message,
                        None,
                    );
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, request @ ..]
            if head == "run-graph" && subcommand == "seed" =>
        {
            let as_json = request.iter().any(|arg| arg == "--json");
            let request_text = request
                .iter()
                .filter(|arg| arg.as_str() != "--json")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if request_text.is_empty() {
                eprintln!("Usage: vida taskflow run-graph seed <task_id> <request_text> [--json]");
                return ExitCode::from(2);
            }

            let payload = match derive_seeded_run_graph_state(&store, task_id, &request_text).await
            {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            match persist_seed_artifacts(&store, &payload).await {
                Ok(()) => {
                    if as_json {
                        let delegation_gate = payload.status.delegation_gate();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph seed",
                                "run_id": task_id,
                                "payload": seed_payload_operator_surface_json(&payload),
                                "delegation_gate": delegation_gate,
                            }))
                            .expect("run-graph seed should render as json")
                        );
                    } else {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph seed");
                        print_surface_line(RenderMode::Plain, "run", task_id);
                        print_surface_line(RenderMode::Plain, "request", &request_text);
                        print_surface_line(
                            RenderMode::Plain,
                            "selected role",
                            &payload.role_selection.selected_role,
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "next node",
                            payload.status.next_node.as_deref().unwrap_or("none"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "route",
                            &payload.status.route_task_class,
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "delegation gate",
                            &payload.status.delegation_gate().as_display(),
                        );
                    }
                    store.close().await;
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            match run_graph_dispatch_init(&store, run_id).await {
                Ok(payload) => {
                    print_surface_header(
                        RenderMode::Plain,
                        "vida taskflow run-graph dispatch-init",
                    );
                    print_surface_line(RenderMode::Plain, "run", run_id);
                    print_surface_line(
                        RenderMode::Plain,
                        "dispatch_packet",
                        payload["dispatch_packet_path"].as_str().unwrap_or("none"),
                    );
                    print_surface_line(
                        RenderMode::Plain,
                        "dispatch_target",
                        payload["dispatch_receipt"]["dispatch_target"]
                            .as_str()
                            .unwrap_or("none"),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            match run_graph_dispatch_init(&store, run_id).await {
                Ok(payload) => {
                    crate::print_json_pretty(&payload);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    print_run_graph_json_error(
                        "vida taskflow run-graph dispatch-init",
                        run_id,
                        &error,
                        run_graph_dispatch_init_error_evidence(&error),
                    );
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class] if head == "run-graph" && subcommand == "init" => {
            let status = default_run_graph_state(task_id, task_class, task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, route_task_class]
            if head == "run-graph" && subcommand == "init" =>
        {
            let status = default_run_graph_state(task_id, task_class, route_task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_state(task_id, task_class, task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: existing.route_task_class,
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match record_run_graph_state_with_continuation_sync(&store, &merged, "run_graph_update")
                .await
            {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_state(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: route_task_class.to_string(),
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match record_run_graph_state_with_continuation_sync(&store, &merged, "run_graph_update")
                .await
            {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class, meta_json]
            if head == "run-graph" && subcommand == "update" =>
        {
            let meta: serde_json::Value = match serde_json::from_str(meta_json) {
                Ok(meta) => meta,
                Err(error) => {
                    eprintln!("[run-graph] meta_json must be valid JSON: {error}");
                    return ExitCode::from(2);
                }
            };
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_state(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = merge_run_graph_meta(
                RunGraphStatus {
                    run_id: task_id.to_string(),
                    task_id: task_id.to_string(),
                    task_class: task_class.to_string(),
                    active_node: node.to_string(),
                    next_node: existing.next_node,
                    status: status.to_string(),
                    route_task_class: route_task_class.to_string(),
                    selected_backend: existing.selected_backend,
                    lane_id: existing.lane_id,
                    lifecycle_stage: existing.lifecycle_stage,
                    policy_gate: existing.policy_gate,
                    handoff_state: existing.handoff_state,
                    context_state: existing.context_state,
                    checkpoint_kind: existing.checkpoint_kind,
                    resume_target: existing.resume_target,
                    recovery_ready: existing.recovery_ready,
                },
                &meta,
            );
            match record_run_graph_state_with_continuation_sync(&store, &merged, "run_graph_update")
                .await
            {
                Ok(()) => {
                    println!(
                        "{}",
                        store
                            .root()
                            .join("run-graph")
                            .join(format!("{task_id}.json"))
                            .display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "init" => {
            eprintln!(
                "Usage: vida taskflow run-graph init <task_id> <task_class> [route_task_class]"
            );
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "seed" => {
            eprintln!("Usage: vida taskflow run-graph seed <task_id> <request_text> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "advance" => {
            eprintln!("Usage: vida taskflow run-graph advance <task_id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "dispatch-init" => {
            eprintln!("Usage: vida taskflow run-graph dispatch-init <task_id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "update" => {
            eprintln!(
                "Usage: vida taskflow run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]"
            );
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_temp_run_graph_root(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos));
        crate::test_cli_support::canonical_team_flow_test_project_root(&root);
        root
    }

    fn recovery_classifier_summary(
        recovery_ready: bool,
        delegated_cycle_open: bool,
    ) -> crate::state_store::RunGraphRecoverySummary {
        let mut summary = default_run_graph_recovery_summary(
            "task-recovery-classifier",
            "run-recovery-classifier",
        );
        summary.recovery_ready = recovery_ready;
        summary.delegation_gate.delegated_cycle_open = delegated_cycle_open;
        summary
    }

    fn recovery_classifier_projection_truth(
        stale_state_suspected: bool,
        active_exception_takeover: bool,
    ) -> RunGraphProjectionTruth {
        let dispatch_receipt = active_exception_takeover.then(|| {
            let mut receipt = clean_ready_downstream_dispatch_receipt("run-recovery-classifier");
            receipt.lane_status = "lane_exception_takeover".to_string();
            receipt.exception_path_receipt_id = Some("exception-receipt".to_string());
            receipt.supersedes_receipt_id = Some("exception-receipt".to_string());
            receipt
        });
        RunGraphProjectionTruth {
            projection_source: "test".to_string(),
            projection_reason: "classifier test projection".to_string(),
            dispatch_receipt_present: dispatch_receipt.is_some(),
            continuation_binding_present: false,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected,
            next_lawful_operator_action: None,
            dispatch_receipt,
            continuation_binding: None,
        }
    }

    fn blocker_codes(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|code| (*code).to_string()).collect()
    }

    #[test]
    fn run_graph_specification_dispatch_convergence() {
        let mut status = default_run_graph_state("task-specification-dispatch", "defect", "defect");
        status.active_node = "reviewer".to_string();
        status.next_node = Some("coder".to_string());
        status.status = "ready".to_string();
        status.lane_id = "coder_lane".to_string();
        status.lifecycle_stage = "coder_dispatch_ready".to_string();
        status.handoff_state = "awaiting_coder".to_string();
        status.context_state = "ready".to_string();
        status.checkpoint_kind = "dispatch_ready".to_string();
        status.resume_target = "dispatch.coder".to_string();
        status.recovery_ready = true;

        assert!(validate_run_graph_resume_gate(&status).is_ok());

        status.lane_id = "reviewer_lane".to_string();
        assert!(validate_run_graph_resume_gate(&status).is_err());

        status.lane_id = "coder_lane".to_string();
        status.context_state = "sealed".to_string();
        assert!(validate_run_graph_resume_gate(&status).is_err());
    }

    #[test]
    fn recommended_surface_for_command_uses_default_human_surface_without_json_bias() {
        assert_eq!(
            recommended_surface_for_command("vida task ready --json"),
            "vida task ready"
        );
        assert_eq!(
            recommended_surface_for_command("vida task ready --scope epic-1 --json"),
            "vida task ready"
        );
        assert_eq!(
            recommended_surface_for_command("vida task show task-1 --json"),
            "vida task show"
        );
        assert_eq!(
            recommended_surface_for_command("vida custom surface --json --details"),
            "vida custom surface --details"
        );
    }

    #[test]
    fn categorize_recovery_diagnosis_preserves_blocker_precedence() {
        let projection_truth = recovery_classifier_projection_truth(false, false);
        let delegated_summary = recovery_classifier_summary(true, true);
        let ready_summary = recovery_classifier_summary(true, false);

        assert_eq!(
            categorize_recovery_diagnosis(
                &blocker_codes(&[
                    "receipt_invalid",
                    "open_delegated_cycle",
                    "internal_codex_carrier_unavailable",
                ]),
                &delegated_summary,
                &projection_truth,
            ),
            "carrier_unavailable"
        );
        assert_eq!(
            categorize_recovery_diagnosis(
                &blocker_codes(&["open_delegated_cycle", "receipt_invalid"]),
                &delegated_summary,
                &projection_truth,
            ),
            "packet_invalid"
        );
        assert_eq!(
            categorize_recovery_diagnosis(
                &blocker_codes(&["unexpected_runtime_fault"]),
                &delegated_summary,
                &projection_truth,
            ),
            "user_action_needed"
        );
        assert_eq!(
            categorize_recovery_diagnosis(
                &blocker_codes(&["internal_activation_view_only", "unexpected_runtime_fault"]),
                &ready_summary,
                &projection_truth,
            ),
            "user_action_needed"
        );
    }

    #[test]
    fn categorize_recovery_diagnosis_preserves_fallback_categories() {
        let ready_summary = recovery_classifier_summary(true, false);
        let blocked_summary = recovery_classifier_summary(false, false);
        let active_exception_projection = recovery_classifier_projection_truth(false, true);
        let stale_projection = recovery_classifier_projection_truth(true, false);
        let clean_projection = recovery_classifier_projection_truth(false, false);

        assert_eq!(
            categorize_recovery_diagnosis(&[], &ready_summary, &active_exception_projection),
            "runtime_defect"
        );
        assert_eq!(
            categorize_recovery_diagnosis(
                &blocker_codes(&["unexpected_runtime_fault"]),
                &ready_summary,
                &clean_projection,
            ),
            "runtime_defect"
        );
        assert_eq!(
            categorize_recovery_diagnosis(&[], &ready_summary, &stale_projection),
            "runtime_defect"
        );
        assert_eq!(
            categorize_recovery_diagnosis(&[], &blocked_summary, &clean_projection),
            "runtime_defect"
        );
        assert_eq!(
            categorize_recovery_diagnosis(&[], &ready_summary, &clean_projection),
            "user_action_needed"
        );
    }

    fn dev_team_step(
        role_label: &str,
        runtime_role: &str,
        task_class: &str,
    ) -> crate::dev_team_sequence_contract::DevTeamSequenceStep {
        crate::dev_team_sequence_contract::DevTeamSequenceStep {
            node_id: role_label.to_string(),
            dispatch_target: role_label.to_string(),
            role_label: role_label.to_string(),
            runtime_role: runtime_role.to_string(),
            task_class: task_class.to_string(),
            packet_template_kind: None,
            closure_class: None,
            stage: None,
            completion_blocker: None,
            inclusion_rule: None,
            requires_task: true,
            requires_user_approval: false,
            approval_policy: serde_json::Value::Null,
            lifecycle_hook_templates: serde_json::Value::Null,
            resume_transitions: serde_json::Value::Null,
            rework_transitions: serde_json::Value::Null,
        }
    }

    fn configured_authority_test_selection(
        compiled_bundle: serde_json::Value,
    ) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "configured TeamFlow authority test".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        }
    }

    fn canonical_default_flow_id(compiled_bundle: &serde_json::Value) -> String {
        compiled_bundle["team_flow_authority"]["selected_config"]["authority_selection"]
            ["default_flow_id"]
            .as_str()
            .expect("canonical bundle must persist default flow id")
            .to_string()
    }

    #[test]
    fn configured_dev_team_route_injects_full_sequence_into_dispatch_contract() {
        let mut execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_sequence": ["analyst", "developer"],
                    "execution_lane_sequence": ["analyst", "developer"],
                    "lane_catalog": {
                        "analyst": {
                            "dispatch_target": "analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "specification"
                        }
                    }
                }
            }
        });
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: Some("meeting_event_form_flow".to_string()),
            node_id: "analyst".to_string(),
            role_label: "analyst".to_string(),
            runtime_role: "business_analyst".to_string(),
            task_class: "specification".to_string(),
            dispatch_target: "analyst".to_string(),
            sequence: vec![
                dev_team_step("analyst", "business_analyst", "specification"),
                dev_team_step("designer", "designer", "design"),
                dev_team_step("autotester", "tester", "verification"),
            ],
        };

        inject_configured_dev_team_route_into_execution_plan(&mut execution_plan, &route);

        let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
        let expected_sequence: Vec<String> = route
            .sequence
            .iter()
            .map(|step| step.role_label.clone())
            .collect();
        assert_eq!(
            dispatch_contract["lane_sequence"],
            serde_json::json!(expected_sequence)
        );
        assert_eq!(
            dispatch_contract["execution_lane_sequence"],
            serde_json::json!(expected_sequence)
        );
        for step in &route.sequence {
            let lane = &dispatch_contract["lane_catalog"][step.role_label.as_str()];
            assert_eq!(
                lane["dispatch_target"].as_str(),
                Some(step.role_label.as_str())
            );
            assert_eq!(
                lane["runtime_role"].as_str(),
                Some(step.runtime_role.as_str())
            );
            assert_eq!(lane["task_class"].as_str(), Some(step.task_class.as_str()));
        }
        assert_eq!(
            crate::taskflow_routing::dispatch_contract_execution_lane_sequence(dispatch_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
    }

    #[test]
    fn configured_dev_team_route_gate_retains_persisted_authority_blocker_path() {
        let mut compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        compiled_bundle["team_flow_authority"]
            .as_object_mut()
            .expect("canonical materialized authority must be an object")
            .remove("authority_id")
            .expect("canonical materialized authority must persist authority_id");
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: String::new(),
            selection_mode: String::new(),
            fallback_role: String::new(),
            request: String::new(),
            selected_role: String::new(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: String::new(),
            matched_terms: Vec::new(),
            compiled_bundle,
            execution_plan: serde_json::Value::Null,
            reason: String::new(),
        };
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: None,
            node_id: String::new(),
            role_label: String::new(),
            runtime_role: String::new(),
            task_class: String::new(),
            dispatch_target: String::new(),
            sequence: Vec::new(),
        };

        let error = validate_configured_dev_team_route_against_authority(&selection, &route)
            .expect_err("missing persisted authority field must fail closed");

        assert!(
            error.starts_with("team_flow_authority_persisted_field_missing: "),
            "blocker must retain its stable code prefix: {error}"
        );
        assert!(
            error.contains("team_flow_authority.authority_id"),
            "blocker must retain its requested JSON path: {error}"
        );
    }

    #[test]
    fn configured_dev_team_route_applies_exact_configured_target_for_implementation() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let (flow_id, included_nodes) = compiled_bundle["team_flow_authority"]
            ["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("canonical bundle must persist configured flows")
            .iter()
            .filter(|flow| flow["flow_policy"]["enabled"].as_bool() == Some(true))
            .filter_map(|flow| flow["flow_id"].as_str().map(str::to_string))
            .find_map(|flow_id| {
                let authority =
                    crate::team_flow_authority_adapter::require_team_flow_execution_authority(
                        &compiled_bundle,
                        Some(flow_id.as_str()),
                        None,
                    )
                    .ok()?;
                let included_nodes = authority
                    .ordered_nodes()
                    .filter(|node| node.node.included)
                    .collect::<Vec<_>>();
                (included_nodes.len() >= 2).then_some((flow_id, included_nodes))
            })
            .expect("an enabled configured flow must expose two included authority nodes");
        let first = &included_nodes[0];
        let route_node = &included_nodes[1];
        assert!(
            !first.node.task_class.trim().is_empty()
                && !route_node.node.task_class.trim().is_empty(),
            "canonical authority nodes must expose configured task classes"
        );
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: Some(flow_id),
            node_id: route_node.node.node_id.clone(),
            role_label: route_node.node.node_id.clone(),
            runtime_role: route_node.node.runtime_role.clone(),
            task_class: route_node.node.task_class.clone(),
            dispatch_target: route_node.node.node_id.clone(),
            sequence: included_nodes
                .iter()
                .take(2)
                .map(|node| {
                    dev_team_step(
                        &node.node.node_id,
                        &node.node.runtime_role,
                        &node.node.task_class,
                    )
                })
                .collect(),
        };
        let selection = configured_authority_test_selection(compiled_bundle);
        let mut status = default_run_graph_state(
            "configured-route-target",
            &route.task_class,
            &route.task_class,
        );
        apply_configured_dev_team_route_to_state(&mut status, &selection, &route)
            .expect("configured route must apply");
        assert_eq!(
            status.next_node.as_deref(),
            Some(route.dispatch_target.as_str())
        );
        assert_ne!(
            status.next_node.as_deref(),
            Some(first.node.node_id.as_str())
        );
    }

    #[test]
    fn configured_dev_team_route_rejects_task_class_tampering() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let flow_id = canonical_default_flow_id(&compiled_bundle);
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &compiled_bundle,
            Some(flow_id.as_str()),
            None,
        )
        .expect("default configured flow must compile");
        let node = authority
            .ordered_nodes()
            .find(|node| node.node.included)
            .expect("configured flow must expose an included node");
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: Some(flow_id),
            node_id: node.node.node_id.clone(),
            role_label: node.node.node_id.clone(),
            runtime_role: node.node.runtime_role.clone(),
            task_class: "tampered_task_class".to_string(),
            dispatch_target: node.node.node_id.clone(),
            sequence: vec![dev_team_step(
                &node.node.node_id,
                &node.node.runtime_role,
                "tampered_task_class",
            )],
        };
        let selection = configured_authority_test_selection(compiled_bundle);
        let error = validate_configured_dev_team_route_against_authority(&selection, &route)
            .expect_err("tampered task class must fail closed");
        assert!(error.starts_with("team_flow_route_task_class_mismatch:"));
    }

    #[test]
    fn configured_dev_team_route_uses_included_sequence_for_default_and_alternate_flows() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let default_flow_id = canonical_default_flow_id(&compiled_bundle);
        let alternate_flow_id = compiled_bundle["team_flow_authority"]["resolved_all_flow_payload"]
            ["flows"]
            .as_array()
            .expect("canonical bundle must persist flows")
            .iter()
            .find(|flow| {
                flow["flow_id"].as_str() != Some(default_flow_id.as_str())
                    && flow["flow_policy"]["enabled"].as_bool() == Some(true)
                    && flow["lanes"].as_array().is_some_and(|lanes| {
                        lanes
                            .iter()
                            .any(|lane| lane["included"].as_bool() == Some(true))
                    })
            })
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string);
        let flow_ids = alternate_flow_id
            .map(|alternate| vec![default_flow_id.clone(), alternate])
            .unwrap_or_else(|| vec![default_flow_id.clone()]);

        for flow_id in flow_ids {
            let authority =
                crate::team_flow_authority_adapter::require_team_flow_execution_authority(
                    &compiled_bundle,
                    Some(flow_id.as_str()),
                    None,
                )
                .expect("selected configured flow must compile");
            let included_nodes: Vec<_> = authority
                .ordered_nodes()
                .filter(|node| node.node.included)
                .collect();
            assert!(
                !included_nodes.is_empty(),
                "selected configured flow must expose an included execution sequence"
            );
            let first = &included_nodes[0];
            let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
                flow_id: Some(flow_id.clone()),
                node_id: first.node.node_id.clone(),
                role_label: first.node.node_id.clone(),
                runtime_role: first.node.runtime_role.clone(),
                task_class: first.node.task_class.clone(),
                dispatch_target: first.node.node_id.clone(),
                sequence: included_nodes
                    .iter()
                    .map(|node| {
                        dev_team_step(
                            &node.node.node_id,
                            &node.node.runtime_role,
                            &node.node.task_class,
                        )
                    })
                    .collect(),
            };
            let selection = configured_authority_test_selection(compiled_bundle.clone());
            validate_configured_dev_team_route_against_authority(&selection, &route)
                .unwrap_or_else(|error| panic!("configured flow {flow_id} must validate: {error}"));
        }
    }

    #[test]
    fn configured_dev_team_route_rejects_explicitly_excluded_node_with_context() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let flow_id = canonical_default_flow_id(&compiled_bundle);
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &compiled_bundle,
            Some(flow_id.as_str()),
            None,
        )
        .expect("default configured flow must compile");
        let excluded = authority
            .ordered_nodes()
            .find(|node| !node.node.included)
            .expect("canonical flow must retain a conditional excluded node");
        let excluded_id = excluded.node.node_id.clone();
        let runtime_role = excluded.node.runtime_role.clone();
        let task_class = excluded.node.task_class.clone();
        let inclusion_rule = excluded.node.inclusion_rule.clone();
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: Some(flow_id.clone()),
            node_id: excluded_id.clone(),
            role_label: excluded_id.clone(),
            runtime_role: runtime_role.clone(),
            task_class: task_class.clone(),
            dispatch_target: excluded_id.clone(),
            sequence: vec![dev_team_step(&excluded_id, &runtime_role, &task_class)],
        };
        let selection = configured_authority_test_selection(compiled_bundle);
        let error = validate_configured_dev_team_route_against_authority(&selection, &route)
            .expect_err("explicitly excluded node must remain fail-closed");
        assert!(
            error.contains("team_flow_node_excluded"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("flow_id={flow_id}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("requested_target={excluded_id}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("runtime_role={runtime_role}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("task_class={task_class}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("included=false"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("inclusion_rule={inclusion_rule}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("requested={excluded_id}:candidates={excluded_id}")),
            "excluded blocker must preserve the original target request and candidates: {error}"
        );
    }

    #[test]
    fn configured_dev_team_route_rejects_unknown_flow_with_context() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let default_flow_id = canonical_default_flow_id(&compiled_bundle);
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &compiled_bundle,
            Some(default_flow_id.as_str()),
            None,
        )
        .expect("default configured flow must compile");
        let first = authority
            .ordered_nodes()
            .find(|node| node.node.included)
            .expect("default configured flow must expose an included node");
        let route_flow_id = format!("{default_flow_id}-unknown");
        let route = crate::dev_team_sequence_contract::ConfiguredDevTeamTaskRoute {
            flow_id: Some(route_flow_id.clone()),
            node_id: first.node.node_id.clone(),
            role_label: first.node.node_id.clone(),
            runtime_role: first.node.runtime_role.clone(),
            task_class: first.node.task_class.clone(),
            dispatch_target: first.node.node_id.clone(),
            sequence: vec![dev_team_step(
                &first.node.node_id,
                &first.node.runtime_role,
                &first.node.task_class,
            )],
        };
        let selection = configured_authority_test_selection(compiled_bundle);
        let error = validate_configured_dev_team_route_against_authority(&selection, &route)
            .expect_err("unknown configured flow must fail closed");
        assert!(
            error.contains("team_flow_authority_unknown_flow"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("flow_id={route_flow_id}")),
            "unexpected error: {error}"
        );
        assert!(
            error.contains(&format!("requested={route_flow_id}")),
            "unexpected error: {error}"
        );
        assert!(error.contains("candidates="), "unexpected error: {error}");
    }

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if matches!(status.trim(), "closed" | "completed") {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

    #[test]
    fn inject_task_planner_metadata_carries_owned_paths_into_tracked_dev_task() {
        let mut selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue active bounded task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut planner_metadata = crate::state_store::TaskPlannerMetadata::default();
        planner_metadata.owned_paths = vec![
            "crates/vida/src/runtime_dispatch_execution.rs".to_string(),
            "docs/product/spec/codex-host-agent-boundary-and-cli-bridge-design.md".to_string(),
        ];

        inject_task_planner_metadata(&mut selection, &planner_metadata);

        assert_eq!(
            selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]["planner_metadata"]
                ["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_execution.rs",
                "docs/product/spec/codex-host-agent-boundary-and-cli-bridge-design.md"
            ])
        );
        assert!(selection
            .request
            .contains("Owned paths: crates/vida/src/runtime_dispatch_execution.rs"));
    }

    fn clean_ready_downstream_dispatch_receipt(run_id: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("developer".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn closed_task_ready_downstream_handoff_blocks_projection_action() {
        let root = test_temp_run_graph_root("vida-closed-task-ready-downstream");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        let run_id = "closed-ready-downstream-run";

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Closed ready downstream run",
                display_id: None,
                description: "closed task with stale run graph handoff",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create closed task");

        let mut status = default_run_graph_state(run_id, "implementation", "implementation");
        status.run_id = run_id.to_string();
        status.task_id = run_id.to_string();
        status.active_node = "architect".to_string();
        status.next_node = Some("internal_subagents".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "architect_complete".to_string();
        status.handoff_state = "awaiting_internal_subagents".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.internal_subagents_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale closed-task status");

        let mut receipt = clean_ready_downstream_dispatch_receipt(run_id);
        receipt.dispatch_target = "architect".to_string();
        receipt.lane_status = "lane_completed".to_string();
        receipt.downstream_dispatch_target = Some("internal_subagents".to_string());
        receipt.downstream_dispatch_command = Some(
            "vida agent-init --downstream-packet downstream.json --execute-dispatch --json"
                .to_string(),
        );
        receipt.downstream_dispatch_packet_path = Some("downstream.json".to_string());
        receipt.downstream_dispatch_result_path = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist ready downstream receipt");

        let projection_truth = run_graph_projection_truth(&store, &status)
            .await
            .expect("build projection truth");
        assert!(projection_truth.stale_state_suspected);
        assert_eq!(
            projection_truth.next_lawful_operator_action.as_deref(),
            Some("vida task reconcile-closed-runs --limit 25")
        );
        assert!(projection_truth.continuation_binding.is_none());
        assert_eq!(
            run_graph_state_surface_issue_codes(&status, &projection_truth),
            vec!["closed_task_active_run_projection_mismatch".to_string()]
        );

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
        )
        .expect("status payload renders");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["closed_task_active_run_projection_mismatch"])
        );
        assert!(
            !payload.to_string().contains("consume continue"),
            "closed task projection must not recommend downstream consume: {payload}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_init_timeout_error_surfaces_blocker_evidence() {
        let message = run_graph_dispatch_init_timeout_message("run-timeout", "test_stage");
        let evidence = run_graph_dispatch_init_error_evidence(&message)
            .expect("dispatch-init timeout should expose blocker evidence");

        assert_eq!(evidence["incident"]["status"], "blocked");
        assert_eq!(
            evidence["blockers"],
            serde_json::json!([RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER])
        );
        assert!(run_graph_dispatch_init_error_evidence("unrelated dispatch-init error").is_none());
    }

    #[test]
    fn dispatch_init_recovery_not_ready_error_surfaces_lane_recovery_action() {
        let evidence = run_graph_dispatch_init_error_evidence(
            "Run-graph resume gate denied for `run-recovery-false`: recovery_ready is false",
        )
        .expect("recovery_ready=false should expose operator action evidence");

        assert_eq!(evidence["surface"], "vida taskflow run-graph dispatch-init");
        assert_eq!(evidence["status"], "blocked");
        assert_eq!(
            evidence["blocker_codes"],
            serde_json::json!(["tool_execution_failed"])
        );
        assert_eq!(
            evidence["recommended_command"],
            "vida lane show run-recovery-false --json"
        );
        assert_eq!(evidence["recommended_surface"], "vida lane show");
        assert_eq!(
            evidence["next_action"]["command"],
            "vida lane show run-recovery-false --json"
        );
        assert!(evidence["next_actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().any(|action| action
                .as_str()
                .is_some_and(|text| text.contains("recommended recovery command")))));
        assert_eq!(
            shared_operator_output_contract_parity_error(&evidence),
            None
        );
    }

    #[test]
    fn run_graph_issue_evidence_accepts_active_exception_takeover() {
        let evidence = run_graph_issue_evidence(RunGraphBlockerEvidenceArgs {
            run_id: "run-active-exception",
            active_node: "analyst",
            status: "blocked",
            route_task_class: "specification",
            policy_gate: "not_required",
            resume_target: "dispatch.analyst",
            next_node: None,
            error: "run-graph advance blocked: run `run-active-exception` is in active exception takeover for `analyst`; finish the scoped local work allowed by `vida lane takeover-ready run-active-exception --json`, then close the bounded task before advancing another runtime lane.",
        })
        .expect("active exception takeover should be explicit blocker evidence")
        .expect("active exception takeover should render blocker evidence");

        assert_eq!(
            evidence["blockers"][0]["code"],
            serde_json::json!("open_delegated_cycle")
        );
        assert_eq!(
            evidence["blockers"][0]["evidence_kind"],
            serde_json::json!("active_exception_takeover")
        );
        assert_eq!(
            evidence["incident"]["active_node"],
            serde_json::json!("analyst")
        );
    }

    #[test]
    fn recovery_status_rejects_stale_blocked_projection_after_clean_lane() {
        let root = std::env::temp_dir().join(format!(
            "vida-recovery-stale-projection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let run_id = "run-1";
        crate::operator_projection_cache::write_json_projection(
            &root,
            &recovery_projection_name(run_id),
            &serde_json::json!({
                "surface": "vida taskflow recovery status",
                "status": "blocked",
                "blocker_codes": ["open_delegated_cycle", "tool_execution_failed"]
            }),
        );
        crate::operator_projection_cache::write_json_projection(
            &root,
            "lane-show-run-1",
            &serde_json::json!({
                "surface": "vida lane",
                "status": "pass",
                "blocker_codes": [],
                "exception_path_receipt_id": null,
                "lane_status": "lane_completed"
            }),
        );
        crate::operator_projection_cache::touch_state_mutation_marker(&root);

        assert!(
            read_recovery_projection(&root, &recovery_projection_name(run_id), run_id).is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_projection_rejects_missing_action_fields_when_projection_has_next_command() {
        let incomplete = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_action": null,
            "recommended_command": null,
            "projection_truth": {
                "next_lawful_operator_action": "vida lane show run-action-cache"
            }
        })
        .to_string();
        let complete = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_action": {
                "command": "vida lane show run-action-cache"
            },
            "recommended_command": "vida lane show run-action-cache",
            "projection_truth": {
                "next_lawful_operator_action": "vida lane show run-action-cache"
            }
        })
        .to_string();

        assert!(!recovery_projection_has_done_action_fields(&incomplete));
        assert!(recovery_projection_has_done_action_fields(&complete));
    }

    #[test]
    fn recovery_projection_rejects_lane_show_when_downstream_agent_init_packet_is_ready() {
        let stale_lane_show = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_action": {
                "command": "vida lane show run-action-cache"
            },
            "recommended_command": "vida lane show run-action-cache",
            "projection_truth": {
                "next_lawful_operator_action": "vida lane show run-action-cache",
                "dispatch_receipt": {
                    "dispatch_status": "executed",
                    "blocker_code": null,
                    "downstream_dispatch_ready": true,
                    "downstream_dispatch_status": "packet_ready",
                    "downstream_dispatch_blockers": [],
                    "downstream_dispatch_command": "vida agent-init",
                    "downstream_dispatch_packet_path": "packet.json"
                }
            }
        })
        .to_string();

        assert!(!recovery_projection_has_done_action_fields(
            &stale_lane_show
        ));
    }

    #[test]
    fn recovery_status_rejects_state_marker_stale_pass_projection_with_next_action() {
        let root = std::env::temp_dir().join(format!(
            "vida-recovery-stale-pass-projection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let run_id = "run-stale-pass";
        crate::operator_projection_cache::write_json_projection(
            &root,
            &recovery_projection_name(run_id),
            &serde_json::json!({
                "surface": "vida taskflow recovery status",
                "status": "pass",
                "blocker_codes": [],
                "next_action": {
                    "command": "vida agent-init --downstream-packet packet.json --execute-dispatch --json"
                },
                "recommended_command": "vida agent-init --downstream-packet packet.json --execute-dispatch --json",
                "projection_truth": {
                    "next_lawful_operator_action": "vida agent-init --downstream-packet packet.json --execute-dispatch --json"
                }
            }),
        );
        crate::operator_projection_cache::touch_state_mutation_marker(&root);

        assert!(
            read_recovery_projection(&root, &recovery_projection_name(run_id), run_id).is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_status_rejects_stale_blocked_projection_after_stale_blocked_lane() {
        let root = std::env::temp_dir().join(format!(
            "vida-recovery-stale-blocked-projection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let run_id = "run-1";
        crate::operator_projection_cache::write_json_projection(
            &root,
            &recovery_projection_name(run_id),
            &serde_json::json!({
                "surface": "vida taskflow recovery status",
                "status": "blocked",
                "blocker_codes": ["open_delegated_cycle", "tool_execution_failed"]
            }),
        );
        crate::operator_projection_cache::write_json_projection(
            &root,
            "lane-show-run-1",
            &serde_json::json!({
                "surface": "vida lane",
                "status": "blocked",
                "blocker_codes": ["open_delegated_cycle"],
                "exception_path_receipt_id": null,
                "lane_status": "lane_blocked"
            }),
        );
        crate::operator_projection_cache::touch_state_mutation_marker(&root);

        assert!(
            read_recovery_projection(&root, &recovery_projection_name(run_id), run_id).is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn recovery_latest_prefers_current_session_run_over_global_stale_run() {
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-recovery-latest");
        }
        let root = std::env::temp_dir().join(format!(
            "vida-recovery-current-session-latest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::new();
        for task_id in ["run-current-recovery", "run-global-stale"] {
            store
                .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                    task_id,
                    title: "Recovery latest task",
                    display_id: None,
                    description: "test task",
                    issue_type: "task",
                    status: "in_progress",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create task");
        }

        let mut current_status = crate::taskflow_run_graph::default_run_graph_state(
            "run-current-recovery",
            "run-current-recovery",
            "analysis",
        );
        current_status.status = "blocked".to_string();
        current_status.lifecycle_stage = "analysis_blocked".to_string();
        current_status.policy_gate = "review_findings".to_string();
        current_status.recovery_ready = false;
        store
            .record_run_graph_status(&current_status)
            .await
            .expect("persist current status");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "current-recovery-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-current-recovery-latest".to_string(),
                process_id: None,
                task_id: Some("run-current-recovery".to_string()),
                run_id: Some("run-current-recovery".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("runtime-delegated-cycle".to_string()),
                owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("claim current run");

        let mut stale_status = crate::taskflow_run_graph::default_run_graph_state(
            "run-global-stale",
            "run-global-stale",
            "planning",
        );
        stale_status.status = "ready".to_string();
        stale_status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        stale_status.next_node = Some("analysis".to_string());
        stale_status.policy_gate = "validation_report_required".to_string();
        stale_status.handoff_state = "awaiting_analysis".to_string();
        stale_status.resume_target = "dispatch.analysis_lane".to_string();
        stale_status.recovery_ready = true;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist newer global status");

        assert_eq!(
            store
                .latest_run_graph_recovery_summary()
                .await
                .expect("read global recovery")
                .expect("global recovery present")
                .run_id,
            "run-global-stale"
        );
        assert_eq!(
            latest_recovery_summary_for_operator_surface(&store)
                .await
                .expect("read operator recovery")
                .expect("operator recovery present")
                .run_id,
            "run-current-recovery"
        );

        let _ = std::fs::remove_dir_all(&root);
        match saved_session_id {
            Some(value) => unsafe {
                std::env::set_var("VIDA_SESSION_ID", value);
            },
            None => unsafe {
                std::env::remove_var("VIDA_SESSION_ID");
            },
        }
    }

    #[test]
    fn dispatch_init_timeout_window_covers_live_activation_snapshot_seed() {
        assert!(
            RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS >= 60,
            "dispatch-init seeds open TaskFlow tasks by compiling the live activation snapshot before a run row exists; keep the bounded window above the observed Windows live seed path"
        );
    }

    #[tokio::test]
    async fn dispatch_init_timeout_marks_missing_receipt_status_blocked() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
agent_system:
  subagents:
    pi_cli:
      enabled: false
      subagent_backend_class: external_cli
"#,
        )
        .expect("write disabled backend overlay");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let run_id = "dispatch-init-timeout-missing-receipt";
        let mut status = default_run_graph_state(run_id, "implementation", "implementation");
        status.run_id = run_id.to_string();
        status.task_id = run_id.to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("implementer".to_string());
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist timeout candidate status");

        assert!(
            record_dispatch_init_timeout_issue(&store, run_id)
                .await
                .expect("timeout blocker should record"),
            "missing receipt timeout should mutate status"
        );

        let blocked = store
            .run_graph_status(run_id)
            .await
            .expect("read blocked status");
        assert_eq!(blocked.status, "blocked");
        assert_eq!(blocked.policy_gate, RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER);
        assert_eq!(blocked.resume_target, "none");
        assert!(!blocked.recovery_ready);
    }

    fn packet_gate_status(task_id: &str) -> RunGraphStatus {
        let mut status = default_run_graph_state(task_id, "implementation", "implementation");
        status.run_id = task_id.to_string();
        status.task_id = task_id.to_string();
        status.status = "running".to_string();
        status.next_node = Some("implementer".to_string());
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = true;
        status
    }

    fn packet_gate_context(task_id: &str) -> RunGraphDispatchContext {
        RunGraphDispatchContext {
            run_id: task_id.to_string(),
            task_id: task_id.to_string(),
            request_text: "Implement one bounded packet-backed scheduler task.".to_string(),
            role_selection: serde_json::json!({"selected_role": "worker"}),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    fn packet_gate_binding(task_id: &str) -> RunGraphContinuationBinding {
        RunGraphContinuationBinding {
            run_id: task_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "task_id": task_id,
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "packet-backed scheduler execute test".to_string(),
            primary_path: "crates/vida/src/taskflow_proxy.rs".to_string(),
            sequential_vs_parallel_posture: "sequential".to_string(),
            request_text: Some("Implement one bounded packet-backed scheduler task.".to_string()),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    fn packet_gate_receipt(task_id: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: task_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --execute-dispatch /tmp/packet.json".to_string(),
            ),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
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
            selected_backend: Some("codex".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn run_graph_status_json_suppresses_inflight_stale_downstream_projection() {
        let mut status = default_run_graph_status("run-designer-pending", "designer", "design");
        status.task_id = "task-designer-pending".to_string();
        status.active_node = "designer".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "designer_blocked".to_string();
        status.recovery_ready = false;

        let mut receipt = packet_gate_receipt("run-designer-pending");
        receipt.dispatch_target = "designer".to_string();
        receipt.dispatch_status = "bridge_request_pending".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_note = Some(
            "specification evidence is recorded and no tracked design-first gate is required; close the bounded lane"
                .to_string(),
        );
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers =
            vec!["host_tool_bridge_adapter_required".to_string()];

        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-designer-pending --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload_with_task_identity_and_state_root(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
            None,
            None,
        )
        .expect("run-graph status payload should build");

        assert_eq!(payload["artifact_refs"]["repair_target"], "designer");
        assert!(payload["artifact_refs"]
            .get("downstream_dispatch_target")
            .is_none());
        assert!(payload["artifact_refs"]
            .get("downstream_dispatch_ready")
            .is_none());
        assert_eq!(
            payload["projection_truth"]["stale_downstream_projection_suppressed"],
            true
        );
        let receipt = &payload["projection_truth"]["dispatch_receipt"];
        assert!(receipt.get("downstream_dispatch_target").is_none());
        assert!(receipt.get("downstream_dispatch_ready").is_none());
        assert_eq!(receipt["dispatch_status"], "bridge_request_pending");
    }

    #[test]
    fn packet_backed_execute_gate_blocks_without_lineage() {
        let gate = evaluate_run_graph_packet_backed_execution_gate(
            Some("sched-primary"),
            None,
            None,
            None,
            None,
        );

        assert!(!gate.supported);
        assert_eq!(gate.status, "blocked_lineage_preconditions_not_verified");
        assert!(gate
            .blocker_codes
            .iter()
            .any(|code| code == "missing_run_graph_status"));
    }

    #[test]
    fn packet_backed_execute_gate_admits_verified_packet_ready_tuple() {
        let status = packet_gate_status("sched-primary");
        let context = packet_gate_context("sched-primary");
        let binding = packet_gate_binding("sched-primary");
        let receipt = packet_gate_receipt("sched-primary");

        let gate = evaluate_run_graph_packet_backed_execution_gate(
            Some("sched-primary"),
            Some(&status),
            Some(&context),
            Some(&binding),
            Some(&receipt),
        );

        assert!(gate.supported);
        assert_eq!(gate.status, "packet_ready");
        assert_eq!(gate.run_id.as_deref(), Some("sched-primary"));
        assert_eq!(
            gate.dispatch_packet_path.as_deref(),
            Some("/tmp/packet.json")
        );
        assert!(gate.blocker_codes.is_empty());
    }

    #[test]
    fn packet_backed_execute_gate_allows_downstream_blockers_for_first_lane_packet() {
        let status = packet_gate_status("sched-primary");
        let context = packet_gate_context("sched-primary");
        let binding = packet_gate_binding("sched-primary");
        let mut receipt = packet_gate_receipt("sched-primary");
        receipt
            .downstream_dispatch_blockers
            .push("pending_implementation_evidence".to_string());

        let gate = evaluate_run_graph_packet_backed_execution_gate(
            Some("sched-primary"),
            Some(&status),
            Some(&context),
            Some(&binding),
            Some(&receipt),
        );

        assert!(gate.supported);
        assert_eq!(gate.status, "packet_ready");
        assert!(gate.blocker_codes.is_empty());
    }

    #[test]
    fn packet_backed_execute_gate_rejects_mismatched_task_run_mapping() {
        let mut status = packet_gate_status("other-task");
        status.task_id = "other-task".to_string();
        let context = packet_gate_context("other-task");
        let binding = packet_gate_binding("other-task");
        let receipt = packet_gate_receipt("other-task");

        let gate = evaluate_run_graph_packet_backed_execution_gate(
            Some("sched-primary"),
            Some(&status),
            Some(&context),
            Some(&binding),
            Some(&receipt),
        );

        assert!(!gate.supported);
        assert_eq!(gate.status, "blocked_task_run_mapping_mismatch");
        assert!(gate
            .blocker_codes
            .iter()
            .any(|code| code == "blocked_task_run_mapping_mismatch"));
    }
    use crate::build_compiled_agent_extension_bundle_for_root;
    use crate::launcher_activation_snapshot::config_file_digest;
    use crate::launcher_activation_snapshot::pack_router_keywords_json;
    use crate::runtime_dispatch_state::load_project_overlay_yaml_for_root;
    use crate::state_store::LauncherActivationSnapshot;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::guard_current_dir;
    use crate::RuntimeConsumptionLaneSelection;
    use serde_json::json;
    use std::path::Path;

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn install(path: std::path::PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    #[tokio::test]
    async fn projection_truth_recommends_retire_for_missing_task_stale_blocked_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let run_id = "runtime-missing-task-stale-run";
        let task_id = "missing-authoritative-task";

        let mut status = default_run_graph_state(task_id, "implementation", "implementation");
        status.run_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: crate::LaneStatus::LaneBlocked.as_str().to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(
                    "runtime-consumption/dispatch-packets/missing.json".to_string(),
                ),
                dispatch_result_path: Some(
                    "runtime-consumption/dispatch-results/missing.json".to_string(),
                ),
                blocker_code: Some("tool_execution_failed".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                policy_bundle_ref: None,
                recorded_at: "2026-05-21T00:00:00Z".to_string(),
            })
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: task_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "run_id": run_id,
                        "task_id": task_id,
                        "active_node": "analysis"
                    }),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only".to_string(),
                    binding_source: "consume_continue_after_downstream_chain".to_string(),
                    why_this_unit: "stale missing-task binding".to_string(),
                    request_text: Some("stale missing task repro".to_string()),
                    recorded_at: "2026-05-21T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale continuation binding");

        let truth = run_graph_projection_truth(&store, &status)
            .await
            .expect("projection truth should build");

        assert!(truth.stale_state_suspected);
        assert_eq!(
            truth.next_lawful_operator_action.as_deref(),
            Some(
                "vida lane retire runtime-missing-task-stale-run --receipt-id runtime-missing-task-stale-run --reason \"missing TaskFlow task stale run\" --json"
            )
        );
        assert!(truth.continuation_binding.is_none());
    }

    #[test]
    fn run_graph_authority_handoff_uses_lane_targets_for_execution() {
        let (handoff_state, resume_target) =
            run_graph_handoff(Some("coach"), DispatchTargetFormat::Lane);
        assert_eq!(handoff_state, "awaiting_coach");
        assert_eq!(resume_target, "dispatch.coach_lane");
    }

    #[test]
    fn run_graph_authority_handoff_uses_direct_targets_for_conversation() {
        let (handoff_state, resume_target) =
            run_graph_handoff(Some("spec-pack"), DispatchTargetFormat::Direct);
        assert_eq!(handoff_state, "awaiting_spec-pack");
        assert_eq!(resume_target, "dispatch.spec-pack");
    }

    #[test]
    fn dispatch_init_replays_active_seeded_lane_when_receipt_is_missing() {
        let status = RunGraphStatus {
            run_id: "run-missing-receipt-active-lane".to_string(),
            task_id: "run-missing-receipt-active-lane".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue implementation".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };

        let reconciled =
            reconcile_dispatch_init_state_for_missing_receipt(status, &role_selection, false);

        assert_eq!(reconciled.next_node.as_deref(), Some("analysis"));
        assert_eq!(reconciled.handoff_state, "awaiting_analysis");
        assert_eq!(reconciled.resume_target, "dispatch.analysis_lane");
        validate_run_graph_resume_gate(&reconciled)
            .expect("replayed active lane should be dispatch-init ready");
    }

    #[test]
    fn recovery_surface_contract_aligns_next_surface_vocabulary() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "planning".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            resume_node: Some("analysis".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.analysis_lane".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_analysis".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "planning".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "implementation_dispatch_ready".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "persisted_run_graph_status".to_string(),
            projection_reason: "paired with continuation binding".to_string(),
            dispatch_receipt_present: false,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "no_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-1 --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert_eq!(blocker_codes, vec!["open_delegated_cycle".to_string()]);
        assert_eq!(
            why_not_now.as_ref().map(|value| value.category.as_str()),
            Some("delegated_cycle_runtime_gate")
        );
        assert_eq!(
            next_action.as_ref().map(|value| value.surface.as_str()),
            Some("vida taskflow consume continue")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida taskflow consume continue --run-id run-1 --json")
        );
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida taskflow consume continue")
        );
    }

    #[test]
    fn recovery_surface_contract_keeps_open_cycle_blocker_for_lane_running_receipt() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-complete".to_string(),
            task_id: "run-lane-complete".to_string(),
            active_node: "pm".to_string(),
            lifecycle_stage: "pm_active".to_string(),
            resume_node: None,
            resume_status: "ready".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "pm".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "pm_active".to_string(),
            },
        };
        let mut receipt = packet_gate_receipt("run-lane-complete");
        receipt.dispatch_target = "pm".to_string();
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.dispatch_result_path = Some("/tmp/run-lane-complete-result.json".to_string());
        receipt.downstream_dispatch_result_path =
            Some("/tmp/run-lane-complete-result.json".to_string());
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "receipt-backed lane completion reconciled status".to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-lane-complete --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert_eq!(blocker_codes, vec!["open_delegated_cycle".to_string()]);
        assert_eq!(
            why_not_now.as_ref().map(|value| value.category.as_str()),
            Some("delegated_cycle_runtime_gate")
        );
        assert_eq!(
            next_action.as_ref().map(|value| value.surface.as_str()),
            Some("vida taskflow run-graph status")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida taskflow run-graph status run-lane-complete --json")
        );
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida taskflow run-graph status")
        );
    }

    #[test]
    fn recovery_latest_json_payload_keeps_operator_contract_parity() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-recovery-json".to_string(),
            task_id: "task-recovery-json".to_string(),
            active_node: "planning".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            resume_node: Some("analysis".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.analysis_lane".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_analysis".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "planning".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "implementation_dispatch_ready".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "persisted_run_graph_status".to_string(),
            projection_reason: "paired with continuation binding".to_string(),
            dispatch_receipt_present: false,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "no_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-recovery-json --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };
        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        let payload = build_recovery_latest_json_payload(
            &summary,
            &projection_truth,
            blocker_codes,
            why_not_now,
            next_action,
            recommended_command,
            recommended_surface,
        )
        .expect("recovery payload should render");

        assert!(payload["dispatch_receipt"].is_null());
        assert_eq!(
            payload["projection_truth"]["dispatch_receipt_present"],
            serde_json::json!(false)
        );
        assert!(payload["projection_truth"]["dispatch_receipt"].is_null());
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["artifact_refs"]["run_id"],
            serde_json::json!("run-recovery-json")
        );
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow recovery latest")
        );
        assert_eq!(
            payload["why_not_now"]["blocking_surface"],
            serde_json::json!("vida taskflow recovery latest")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn recovery_status_json_payload_keeps_operator_contract_parity() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-recovery-status-json".to_string(),
            task_id: "task-recovery-status-json".to_string(),
            active_node: "verification".to_string(),
            lifecycle_stage: "verification_active".to_string(),
            resume_node: Some("closure".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.closure".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_closure".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "verification".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "verification_active".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-recovery-status-json --json"
                    .to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };
        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);
        let payload = build_recovery_json_payload(
            "vida taskflow recovery status",
            &summary,
            &projection_truth,
            blocker_codes,
            why_not_now,
            next_action,
            recommended_command,
            recommended_surface,
        )
        .expect("recovery status payload should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow recovery status")
        );
        assert_eq!(
            payload["why_not_now"]["blocking_surface"],
            serde_json::json!("vida taskflow recovery status")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn recovery_status_json_failure_contract() {
        let state_dir = std::path::Path::new(".vida/data/state");
        let payload = recovery_json_error_payload(
            "vida taskflow recovery status",
            "run-missing-recovery-json",
            state_dir,
            "run_graph_recovery_unreadable",
            "task is missing: run_graph:run-missing-recovery-json",
        );

        assert_eq!(payload["surface"], "vida taskflow recovery status");
        assert_eq!(payload["run_id"], "run-missing-recovery-json");
        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
        assert_eq!(
            payload["artifact_refs"]["surface"],
            "vida taskflow recovery status"
        );
        assert_eq!(
            payload["artifact_refs"]["run_id"],
            "run-missing-recovery-json"
        );
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(payload["error_kind"], "run_graph_recovery_unreadable");
        assert!(payload["next_actions"][0]
            .as_str()
            .is_some_and(|action| action.contains("vida taskflow run-graph status")));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn recovery_explain_json_payload_keeps_operator_contract_parity() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-recovery-explain-json".to_string(),
            task_id: "task-recovery-explain-json".to_string(),
            active_node: "writer".to_string(),
            lifecycle_stage: "implementation_writer_active".to_string(),
            resume_node: Some("verification".to_string()),
            resume_status: "blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.verifier".to_string(),
            policy_gate: "writer_result_required".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "writer".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "implementation_writer_active".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-recovery-explain-json --json"
                    .to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };
        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);
        let payload = build_recovery_explain_json_payload(
            "vida taskflow recovery explain",
            &summary,
            &projection_truth,
            blocker_codes,
            why_not_now,
            next_action,
            recommended_command,
            recommended_surface,
        )
        .expect("recovery explain payload should render");

        assert_eq!(payload["surface"], "vida taskflow recovery explain");
        assert_eq!(payload["status"], "blocked");
        assert!(payload.get("diagnosis").is_some());
        // diagnosis is now a string with one of four types
        let diagnosis_value = payload["diagnosis"]
            .as_str()
            .expect("diagnosis should be a string");
        assert!([
            "runtime_defect",
            "carrier_unavailable",
            "packet_invalid",
            "user_action_needed"
        ]
        .contains(&diagnosis_value));
        // diagnosis_detail contains the old diagnosis object
        assert!(payload.get("diagnosis_detail").is_some());
        assert_eq!(
            payload["diagnosis_detail"]["blocker_codes"],
            payload["blocker_codes"]
        );
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow recovery explain")
        );
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[tokio::test]
    async fn recovery_status_json_without_run_id_uses_latest_recovery_summary() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _proxy_state = ProxyStateDirOverrideGuard::install(harness.path().to_path_buf());
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let mut status = default_run_graph_state(
            "task-recovery-status-json-latest",
            "implementation",
            "implementation",
        );
        status.run_id = "run-recovery-status-json-latest".to_string();
        status.active_node = "analysis".to_string();
        status.lifecycle_stage = "analysis_active".to_string();
        status.next_node = Some("writer".to_string());
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "awaiting_writer".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.writer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("record latest run status");
        drop(store);

        let args = vec![
            "recovery".to_string(),
            "status".to_string(),
            "--json".to_string(),
        ];
        assert_eq!(run_taskflow_recovery(&args).await, ExitCode::SUCCESS);
    }

    #[test]
    fn recovery_status_action_for_configured_backend_failure_points_to_lane_show() {
        let status = RunGraphStatus {
            run_id: "run-configured-backend-blocked".to_string(),
            task_id: "task-configured-backend-blocked".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);

        assert!(command
            .as_deref()
            .is_some_and(|value| value == "vida lane show run-configured-backend-blocked --json"));
    }

    #[test]
    fn recovery_status_action_for_internal_activation_view_only_points_to_lane_show() {
        let status = RunGraphStatus {
            run_id: "run-internal-activation-view-only".to_string(),
            task_id: "task-internal-activation-view-only".to_string(),
            task_class: "verification".to_string(),
            active_node: "verification".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "verification".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "verification_lane".to_string(),
            lifecycle_stage: "verification_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("middle".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);

        assert_eq!(
            command.as_deref(),
            Some("vida lane show run-internal-activation-view-only --json")
        );
    }

    #[test]
    fn recovery_status_action_for_internal_codex_carrier_unavailable_points_to_lane_show() {
        let mut status = packet_gate_status("run-internal-codex-carrier");
        status.status = "blocked".to_string();
        status.recovery_ready = false;
        status.resume_target = "dispatch.coach".to_string();
        status.active_node = "coach".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();

        let mut receipt = packet_gate_receipt("run-internal-codex-carrier");
        receipt.dispatch_target = "coach".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("internal_codex_carrier_unavailable".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);

        assert_eq!(
            command.as_deref(),
            Some("vida lane show run-internal-codex-carrier --json")
        );
    }

    #[test]
    fn recovery_status_action_for_internal_timeout_with_running_lane_points_to_lane_show() {
        let mut status = packet_gate_status("run-internal-timeout");
        status.task_id = "task-internal-timeout".to_string();
        status.status = "blocked".to_string();
        status.recovery_ready = false;
        status.resume_target = "none".to_string();
        status.active_node = "test_author".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();

        let mut receipt = packet_gate_receipt("run-internal-timeout");
        receipt.dispatch_target = "test_author".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);

        assert_eq!(
            command.as_deref(),
            Some("vida lane show run-internal-timeout --json")
        );
    }

    #[test]
    fn recovery_status_action_for_downstream_missing_owned_scope_points_to_packet_render() {
        let status = RunGraphStatus {
            run_id: "run-missing-owned-scope".to_string(),
            task_id: "task-missing-owned-scope".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: Some("writer".to_string()),
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some("/tmp/result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);

        assert_eq!(
            command.as_deref(),
            Some("vida taskflow packet render run-missing-owned-scope --json")
        );
    }

    #[test]
    fn run_graph_status_payload_preserves_missing_owned_scope_blocker() {
        let status = RunGraphStatus {
            run_id: "run-status-missing-owned-scope".to_string(),
            task_id: "task-status-missing-owned-scope".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: Some("writer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            recovery_ready: true,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some("/tmp/result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        let next_lawful_operator_action =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false);
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects persisted dispatch blocker evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action,
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
        )
        .expect("status payload should render");

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array")
            .iter()
            .any(|value| value == "missing_owned_write_scope"));
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .any(|value| value.as_str().is_some_and(|action| action
                .contains("vida taskflow packet render run-status-missing-owned-scope"))));
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .all(|value| value
                .as_str()
                .is_some_and(|action| !action.contains("--json"))));
    }

    #[test]
    fn exception_takeover_projection_replaces_stale_dispatch_init_binding() {
        let status = RunGraphStatus {
            run_id: "run-stale-binding".to_string(),
            task_id: "run-stale-binding".to_string(),
            task_class: "scope_discussion".to_string(),
            active_node: "specification".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "spec-pack".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "business_analyst_lane".to_string(),
            lifecycle_stage: "specification_blocked".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("exc-stale-binding".to_string()),
            exception_path_receipt_id: Some("exc-stale-binding".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
            downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "pending_specification_evidence".to_string(),
                "pending_design_finalize".to_string(),
                "pending_spec_task_close".to_string(),
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-06T10:10:26Z".to_string(),
        };
        let stale_binding = RunGraphContinuationBinding {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": status.task_id,
                "run_id": status.run_id,
                "active_node": "planning"
            }),
            binding_source: "run_graph_dispatch_init".to_string(),
            why_this_unit: "stale dispatch-init binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
            request_text: None,
            recorded_at: "2026-05-06T10:10:26Z".to_string(),
        };

        let effective =
            effective_projection_continuation_binding(&status, Some(&receipt), Some(stale_binding))
                .expect("exception takeover should synthesize current binding");

        assert_eq!(
            effective.binding_source,
            "latest_run_graph_exception_takeover_dispatch"
        );
        assert_eq!(
            effective.active_bounded_unit["active_node"],
            "specification"
        );
        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false)
                .as_deref(),
            Some("vida lane show run-stale-binding --json")
        );
    }

    #[test]
    fn recovery_status_payload_for_terminal_write_blocker_is_actionable_and_parity_safe() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-terminal-write-blocked".to_string(),
            task_id: "task-terminal-write-blocked".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                reporting_pause_gate: "continuation_check_required".to_string(),
                continuation_signal: "continuation_check_required".to_string(),
                blocker_code: None,
                lifecycle_stage: "analysis_blocked".to_string(),
            },
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: summary.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("terminal evidence missing".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects persisted dispatch blocker evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: next_lawful_operator_action_for_projection(
                &RunGraphStatus {
                    run_id: summary.run_id.clone(),
                    task_id: summary.task_id.clone(),
                    task_class: "implementation".to_string(),
                    active_node: summary.active_node.clone(),
                    next_node: None,
                    status: "blocked".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "internal_subagents".to_string(),
                    lane_id: "analysis_lane".to_string(),
                    lifecycle_stage: summary.lifecycle_stage.clone(),
                    policy_gate: summary.policy_gate.clone(),
                    handoff_state: summary.handoff_state.clone(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: summary.checkpoint_kind.clone(),
                    resume_target: summary.resume_target.clone(),
                    recovery_ready: summary.recovery_ready,
                },
                Some(&receipt),
                None,
                false,
                false,
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };
        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);
        let payload = build_recovery_json_payload(
            "vida taskflow recovery status",
            &summary,
            &projection_truth,
            blocker_codes,
            why_not_now,
            next_action,
            recommended_command,
            recommended_surface,
        )
        .expect("recovery status payload should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["recommended_surface"],
            serde_json::json!("vida lane show")
        );
        assert!(payload["recommended_command"]
            .as_str()
            .is_some_and(|value| value == "vida lane show run-terminal-write-blocked --json"));
        assert_eq!(
            payload["why_not_now"]["category"],
            serde_json::json!("run_graph_blocked_state")
        );
        assert_eq!(
            payload["why_not_now"]["blocking_surface"],
            serde_json::json!("vida taskflow recovery status")
        );
        assert!(payload["why_not_now"]["summary"].as_str().is_some_and(
            |value| value == "The run graph is blocked while delegated cycle state is `clear`."
        ));
        assert_eq!(
            payload["next_action"]["surface"],
            serde_json::json!("vida lane show")
        );
        assert!(payload["next_actions"].as_array().is_some_and(|actions| {
            actions.iter().any(|value| {
                value.as_str().is_some_and(|text| {
                    text.contains("lane envelope")
                        && text.contains("exception-takeover evidence")
                        && text.contains("supersession")
                })
            })
        }));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn recovery_status_action_for_recorded_exception_points_to_supersede() {
        let status = RunGraphStatus {
            run_id: "run-recorded-exception".to_string(),
            task_id: "task-recorded-exception".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exception-receipt-1".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false)
                .as_deref(),
            Some(
                "vida lane supersede run-recorded-exception --receipt-id exception-receipt-1 --json"
            )
        );
    }

    #[test]
    fn recovery_status_action_for_active_exception_returns_to_continue() {
        let status = RunGraphStatus {
            run_id: "run-active-exception".to_string(),
            task_id: "task-active-exception".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "dispatch.analysis_lane".to_string(),
            recovery_ready: true,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("exception-receipt-1".to_string()),
            exception_path_receipt_id: Some("exception-receipt-1".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false)
                .as_deref(),
            Some("vida taskflow consume continue --run-id run-active-exception --json")
        );
    }

    #[test]
    fn recovery_status_action_for_terminal_consumed_exception_requires_bind() {
        let status = RunGraphStatus {
            run_id: "run-active-exception".to_string(),
            task_id: "task-active-exception".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "dispatch.analysis_lane".to_string(),
            recovery_ready: true,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("exception-receipt-1".to_string()),
            exception_path_receipt_id: Some("exception-receipt-1".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(
                &status,
                Some(&receipt),
                Some("run-active-exception"),
                false,
                false,
            )
            .as_deref(),
            Some("vida taskflow run-graph status run-active-exception --json")
        );
    }

    #[test]
    fn recovery_status_action_for_internal_timeout_without_ready_recovery_points_to_lane_show() {
        let mut status = default_run_graph_state("run-timeout", "implementation", "coach");
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.active_node = "implementer".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        let mut receipt = packet_gate_receipt("run-timeout");
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt
            .downstream_dispatch_blockers
            .push("internal_dispatch_timeout_without_receipt".to_string());

        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false)
                .as_deref(),
            Some("vida lane show run-timeout --json")
        );
    }

    #[test]
    fn missing_task_stale_cleanup_is_not_actionable_for_terminal_resolved_status() {
        let mut terminal =
            default_run_graph_state("run-terminal-missing", "implementation", "closure");
        terminal.active_node = "closure".to_string();
        terminal.status = "completed".to_string();
        terminal.lifecycle_stage = "closure_complete".to_string();
        terminal.handoff_state = "none".to_string();
        terminal.context_state = "sealed".to_string();
        terminal.checkpoint_kind = "none".to_string();
        terminal.resume_target = "none".to_string();
        terminal.recovery_ready = false;
        terminal.next_node = None;
        let mut completed_like =
            default_run_graph_state("run-completed-like-missing", "implementation", "coach");
        completed_like.status = "completed".to_string();
        completed_like.lifecycle_stage = "implementation_complete".to_string();
        completed_like.next_node = Some("verification".to_string());
        completed_like.resume_target = "dispatch.implementation_lane".to_string();
        completed_like.recovery_ready = true;
        let mut active = default_run_graph_state("run-active-missing", "implementation", "coach");
        active.status = "blocked".to_string();
        active.lifecycle_stage = "coach_blocked".to_string();

        assert!(!missing_task_run_graph_requires_stale_cleanup(
            Some(&terminal),
            true
        ));
        assert!(missing_task_run_graph_requires_stale_cleanup(
            Some(&active),
            true
        ));
        assert!(missing_task_run_graph_requires_stale_cleanup(
            Some(&completed_like),
            true
        ));
        assert!(!missing_task_run_graph_requires_stale_cleanup(
            Some(&active),
            false
        ));
        assert!(
            next_lawful_operator_action_for_projection(&terminal, None, None, true, false)
                .is_none()
        );
        assert!(
            next_lawful_operator_action_for_projection(&active, None, None, true, false)
                .as_deref()
                .is_some_and(|action| action.starts_with("vida lane retire run-active-missing"))
        );
    }

    #[test]
    fn run_graph_status_json_payload_keeps_operator_contract_parity() {
        let status = RunGraphStatus {
            run_id: "run-status-json".to_string(),
            task_id: "task-status-json".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: Some("verification".to_string()),
            status: "executing".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "lane-status-json".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_verification".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.verification_lane".to_string(),
            recovery_ready: true,
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-status-json --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
        )
        .expect("run-graph status payload should render");

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph status")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_status_error_payload_is_actionable_and_parity_safe() {
        let state_dir = std::env::temp_dir().join("vida-run-graph-status-error-payload-test");
        let payload = run_graph_status_error_payload(
            &state_dir,
            "missing-run",
            "run_graph_status_unavailable",
            "task is missing: run_graph:missing-run",
        );

        assert_eq!(payload["surface"], "vida taskflow run-graph status");
        assert_eq!(payload["run_id"], "missing-run");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array")
            .iter()
            .any(|code| code
                .as_str()
                .is_some_and(|code| code == "run_graph_status_unavailable")));
        assert_eq!(payload["error_kind"], "run_graph_status_unavailable");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph status")
        );
        assert_eq!(
            payload["artifact_refs"]["run_id"],
            serde_json::json!("missing-run")
        );
        assert_eq!(
            payload["recommended_command"],
            serde_json::json!("vida taskflow run-graph latest")
        );
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|action| action.contains("vida taskflow run-graph latest"))));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_latest_error_payload_is_actionable_and_parity_safe() {
        let state_dir = std::env::temp_dir().join("vida-run-graph-latest-error-payload-test");
        let payload = run_graph_latest_error_payload(
            &state_dir,
            "projection_truth_unavailable",
            "latest projection failed",
        );

        assert_eq!(payload["surface"], "vida taskflow run-graph latest");
        assert_eq!(payload["run_id"], "latest");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array")
            .iter()
            .any(|code| code
                .as_str()
                .is_some_and(|code| code == fallback_dispatch_issue_code())));
        assert_eq!(payload["error_kind"], "projection_truth_unavailable");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph latest")
        );
        assert_eq!(
            payload["artifact_refs"]["run_id"],
            serde_json::json!("latest")
        );
        assert_eq!(
            payload["recommended_command"],
            serde_json::json!("vida taskflow run-graph latest")
        );
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|action| action.contains("vida status"))));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_status_json_payload_surfaces_blocked_dispatch_truth() {
        let status = RunGraphStatus {
            run_id: "runtime-audit-state-store-init-lock-timeout".to_string(),
            task_id: "runtime-audit-state-store-init-lock-timeout".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "analysis-lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/runtime-audit-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/runtime-audit-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects persisted dispatch blocker evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(format!(
                "vida taskflow run-graph status {}",
                status.run_id
            )),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
        )
        .expect("blocked run-graph status payload should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["tool_execution_failed"])
        );
        assert_eq!(
            payload["projection_truth"]["dispatch_receipt"]["blocker_code"],
            serde_json::json!("configured_backend_dispatch_failed")
        );
        assert!(payload["next_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_status_json_payload_does_not_block_on_stale_exception_receipt_after_handoff() {
        let status = RunGraphStatus {
            run_id: "run-advanced-after-exception".to_string(),
            task_id: "task-advanced-after-exception".to_string(),
            task_class: "implementation".to_string(),
            active_node: "coach".to_string(),
            next_node: Some("review_ensemble".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "coach_lane".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            policy_gate: "review_findings".to_string(),
            handoff_state: "awaiting_review_ensemble".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.review_ensemble".to_string(),
            recovery_ready: true,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "test_author".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("exception-receipt".to_string()),
            exception_path_receipt_id: Some("exception-receipt".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/test-author-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/test-author-result.json".to_string()),
            blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "internal_dispatch_timeout_without_receipt".to_string()
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("test_author".to_string()),
            downstream_dispatch_last_target: Some("test_author".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-05-24T00:00:00Z".to_string(),
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-advanced-after-exception --json"
                    .to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
        )
        .expect("advanced handoff payload should render");

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            payload["projection_truth"]["dispatch_receipt"]["blocker_code"],
            serde_json::json!("internal_dispatch_timeout_without_receipt")
        );
    }

    #[test]
    fn status_surface_projection_truth_keeps_blocked_receipt_actionable() {
        let status = RunGraphStatus {
            run_id: "run-status-surface-blocked".to_string(),
            task_id: "task-status-surface-blocked".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "analysis-lane".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: status.run_id.clone(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/status-surface-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/status-surface-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            policy_bundle_ref: None,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        let (_projection_truth, blocker_codes) = projection_truth_from_state_surface(
            std::path::Path::new("."),
            &status,
            None,
            Some(&receipt),
            None,
        );

        assert_eq!(blocker_codes, vec!["tool_execution_failed".to_string()]);
    }

    #[test]
    fn internal_timeout_without_ready_recovery_recommends_lane_inspection_not_continue() {
        let status = RunGraphStatus {
            run_id: "run-timeout-not-ready".to_string(),
            task_id: "task-timeout-not-ready".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementation".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "lane-timeout-not-ready".to_string(),
            lifecycle_stage: "implementation_blocked".to_string(),
            policy_gate: "agent_init_execute_dispatch_timeout".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "implementation".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --dispatch-packet packet.json --execute-dispatch --json"
                    .to_string(),
            ),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "internal_dispatch_timeout_without_receipt".to_string()
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementation".to_string()),
            downstream_dispatch_last_target: Some("implementation".to_string()),
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-06-01T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_dispatch_resolution(&status, &receipt, None).as_deref(),
            Some("vida lane show run-timeout-not-ready --json")
        );
    }

    #[test]
    fn run_graph_latest_json_payload_keeps_operator_contract_parity() {
        let status = RunGraphStatus {
            run_id: "run-latest-json".to_string(),
            task_id: "task-latest-json".to_string(),
            task_class: "specification".to_string(),
            active_node: "business_analyst".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "specification".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-latest-json".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "analysis_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "persisted_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects authoritative persisted state"
                .to_string(),
            dispatch_receipt_present: false,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "no_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow consume continue --run-id run-latest-json --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload(
            "vida taskflow run-graph latest",
            &status,
            &projection_truth,
        )
        .expect("latest run-graph payload should render");

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph latest")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_status_host_bridge_projection_blocker_next_actions() {
        let root = test_temp_run_graph_root("run-graph-status-host-bridge-actionable");
        let result_path =
            root.join("runtime-consumption/dispatch-results/run-host-bridge-blocked.json");
        let request_path =
            root.join("host-tool-bridge/requests/run-host-bridge-blocked-request.json");
        let receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-blocked-receipt.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "blocked",
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["host_tool_bridge_adapter_required"],
                "rework_target": "developer",
                "allowed_next_node": "developer_rework",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": receipt_path.display().to_string()
                }
            }))
            .expect("result should encode"),
        )
        .expect("write result");
        let status = RunGraphStatus {
            run_id: "run-host-bridge-blocked".to_string(),
            task_id: "task-host-bridge-blocked".to_string(),
            task_class: "implementation".to_string(),
            active_node: "coach".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "coach_lane".to_string(),
            lifecycle_stage: "coach_blocked".to_string(),
            policy_gate: "host_bridge_completion_result_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
            dispatch_packet_path: Some(
                "runtime-consumption/dispatch-packets/coach.json".to_string(),
            ),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
            downstream_dispatch_target: Some("developer".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some(
                "host bridge completion requires developer rework".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["host_tool_bridge_adapter_required".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-06-24T00:00:00Z".to_string(),
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-host-bridge-blocked --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_state_json_payload_with_task_identity_and_state_root(
            "vida taskflow run-graph status",
            &status,
            &projection_truth,
            None,
            Some(&root),
        )
        .expect("run-graph status payload should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["host_tool_bridge_adapter_required"])
        );
        let next_actions = payload["next_actions"]
            .as_array()
            .expect("next_actions should render");
        assert!(next_actions.iter().any(|action| {
            action
                .as_str()
                .is_some_and(|value| value.contains("vida lane show run-host-bridge-blocked"))
        }));
        assert!(next_actions.iter().all(|action| {
            action.as_str().is_some_and(|value| {
                !value.contains("vida taskflow run-graph status run-host-bridge-blocked")
            })
        }));
        assert_eq!(
            payload["artifact_refs"]["dispatch_result_path"],
            serde_json::json!(result_path.display().to_string())
        );
        assert!(
            state_artifact_path_in_root(&root, &receipt_path.display().to_string()).is_some(),
            "missing expected artifact path scope root={} receipt={}",
            root.display(),
            receipt_path.display()
        );
        assert!(payload["artifact_refs"]["host_bridge_receipt_path"].is_null());
        assert_eq!(
            payload["artifact_refs"]["expected_host_bridge_receipt_path"],
            serde_json::json!(receipt_path.display().to_string())
        );
        assert_eq!(
            payload["artifact_refs"]["downstream_dispatch_status"],
            serde_json::json!("blocked")
        );
        assert_eq!(
            payload["artifact_refs"]["repair_target"],
            serde_json::json!("developer")
        );
        assert_eq!(
            payload["artifact_refs"]["result_rework_target"],
            serde_json::json!("developer")
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            payload["operator_contracts"]["artifact_refs"]
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn compact_dispatch_summary_reuses_projection_and_downstream_preview_semantics() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: Some("verifier".to_string()),
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-1".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_verifier".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.verifier".to_string(),
            recovery_ready: true,
        };
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "implementer".to_string(),
            lifecycle_stage: "implementer_active".to_string(),
            resume_node: Some("verifier".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.verifier".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_verifier".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "implementer".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "implementer_active".to_string(),
            },
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
            downstream_dispatch_target: Some("verifier".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("proof".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: vec!["missing_review_receipt".to_string()],
            downstream_dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_trace_path: Some("/tmp/downstream-trace.json".to_string()),
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verifier".to_string()),
            downstream_dispatch_last_target: Some("verifier".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            policy_bundle_ref: None,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::json!({
                "activation_kind": "activation_view",
                "evidence_state": "activation_view_only",
                "receipt_backed": false,
            }),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };
        let continuation_binding = serde_json::json!({
            "status": "bound",
            "primary_path": "dispatch.verifier",
        });

        let summary = build_run_graph_dispatch_compact_summary(
            std::path::Path::new("."),
            Some(&status),
            Some(&recovery),
            Some(&receipt),
            Some(&continuation_binding),
            None,
        )
        .expect("compact summary should exist");

        assert_eq!(
            summary.route_truth.projection_source,
            "reconciled_run_graph_status"
        );
        assert_eq!(
            summary.route_truth.projection_vs_receipt_parity,
            "aligned".to_string()
        );
        assert_eq!(summary.route_truth.evidence_state, "activation_view_only");
        assert_eq!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_target,
            "verifier"
        );
        assert_eq!(
            summary.blocker_codes,
            vec!["open_delegated_cycle".to_string()]
        );
        assert_eq!(
            summary.recommended_surface.as_deref(),
            Some("vida taskflow consume continue")
        );
    }

    #[test]
    fn compact_dispatch_summary_suppresses_stale_open_cycle_after_completed_lane_receipt() {
        let status = RunGraphStatus {
            run_id: "run-completed-cycle".to_string(),
            task_id: "task-completed-cycle".to_string(),
            task_class: "implementation".to_string(),
            active_node: "coach".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-completed-cycle".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-completed-cycle".to_string(),
            task_id: "task-completed-cycle".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: Some("coach".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_active".to_string(),
            },
        };
        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-completed-cycle".to_string(),
            dispatch_target: "coach".to_string(),
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
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("proof".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_trace_path: Some("/tmp/downstream-trace.json".to_string()),
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            policy_bundle_ref: None,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::json!({
                "activation_kind": "execution_receipt",
                "evidence_state": "receipt_backed_execution",
                "receipt_backed": true,
            }),
            recorded_at: "2026-05-16T00:00:00Z".to_string(),
        };
        let continuation_binding = serde_json::json!({
            "status": "bound",
            "primary_path": "dispatch.coach",
        });

        let summary = build_run_graph_dispatch_compact_summary(
            std::path::Path::new("."),
            Some(&status),
            Some(&recovery),
            Some(&receipt),
            Some(&continuation_binding),
            None,
        )
        .expect("compact summary should exist");

        assert_eq!(
            summary.route_truth.projection_vs_receipt_parity,
            "reconciled_from_receipt"
        );
        assert_eq!(summary.blocker_codes, Vec::<String>::new());
        assert_eq!(summary.recommended_command, None);
        assert_eq!(summary.recommended_surface, None);
    }

    #[test]
    fn compact_dispatch_summary_falls_back_to_status_truth_without_receipt() {
        let status = RunGraphStatus {
            run_id: "run-2".to_string(),
            task_id: "task-2".to_string(),
            task_class: "specification".to_string(),
            active_node: "business_analyst".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "specification".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-2".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "analysis_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        let continuation_binding = serde_json::json!({
            "status": "bound",
            "primary_path": "dispatch.implementer",
        });
        let activation_vs_execution_evidence = serde_json::json!({
            "evidence_state": "activation_view_only",
            "activation_kind": "activation_view",
            "receipt_backed": false,
        });

        let summary = build_run_graph_dispatch_compact_summary(
            std::path::Path::new("."),
            Some(&status),
            None,
            None,
            Some(&continuation_binding),
            Some(&activation_vs_execution_evidence),
        )
        .expect("compact summary should exist without receipt");

        assert_eq!(
            summary.route_truth.projection_source,
            "persisted_run_graph_status"
        );
        assert_eq!(
            summary.route_truth.projection_vs_receipt_parity,
            "no_receipt"
        );
        assert!(!summary.route_truth.dispatch_receipt_present);
        assert!(summary.route_truth.continuation_binding_present);
        assert_eq!(summary.route_truth.evidence_state, "activation_view_only");
        assert_eq!(summary.route_truth.activation_kind, "activation_view");
        assert!(!summary.route_truth.receipt_backed_execution_evidence);
        assert_eq!(
            summary.downstream_dispatch_preview.dispatch_target,
            "business_analyst"
        );
        assert_eq!(summary.downstream_dispatch_preview.dispatch_status, "ready");
        assert_eq!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_target,
            "implementer"
        );
        assert_eq!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_status,
            "resume_ready"
        );
        assert!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_ready
        );
        assert_eq!(
            summary.downstream_dispatch_preview.lane_status,
            "analysis_active"
        );
        assert_eq!(
            summary.downstream_dispatch_preview.selected_backend,
            "opencode_cli"
        );
        assert_eq!(
            summary.recommended_command.as_deref(),
            Some("vida taskflow consume continue --run-id run-2 --json")
        );
    }

    #[test]
    fn compact_dispatch_summary_ignores_non_dispatch_continuation_primary_path() {
        let status = RunGraphStatus {
            run_id: "run-2b".to_string(),
            task_id: "task-2b".to_string(),
            task_class: "delivery".to_string(),
            active_node: "closure".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "delivery".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-2b".to_string(),
            lifecycle_stage: "closure_pending".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        let continuation_binding = serde_json::json!({
            "status": "bound",
            "primary_path": "normal_delivery_path",
        });

        let summary = build_run_graph_dispatch_compact_summary(
            std::path::Path::new("."),
            Some(&status),
            None,
            None,
            Some(&continuation_binding),
            None,
        )
        .expect("compact summary should exist");

        assert_eq!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_target,
            "implementer"
        );
        assert_ne!(
            summary
                .downstream_dispatch_preview
                .downstream_dispatch_target,
            "normal_delivery_path"
        );
    }

    #[test]
    fn compact_dispatch_summary_reuses_recovery_semantics_for_stale_dispatch() {
        let root = std::env::temp_dir().join(format!(
            "vida-compact-dispatch-stale-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let result_path = root.join("dispatch-result.json");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "execution_state": "executing",
                "recorded_at": "2026-04-18T00:00:00Z"
            }))
            .expect("dispatch result should encode"),
        )
        .expect("dispatch result should write");

        let status = RunGraphStatus {
            run_id: "run-stale".to_string(),
            task_id: "task-stale".to_string(),
            task_class: "implementation".to_string(),
            active_node: "implementer".to_string(),
            next_node: Some("reviewer".to_string()),
            status: "executing".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "lane-stale".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-stale".to_string(),
            task_id: "task-stale".to_string(),
            active_node: "implementer".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            resume_status: "executing".to_string(),
            recovery_ready: false,
            handoff_state: "none".to_string(),
            policy_gate: "not_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "implementer".to_string(),
                lifecycle_stage: "implementation_active".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-stale".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/stale-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            policy_bundle_ref: None,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::json!({
                "activation_kind": "activation_view",
                "evidence_state": "activation_view_only",
                "receipt_backed": false,
            }),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let summary = build_run_graph_dispatch_compact_summary(
            &root,
            Some(&status),
            Some(&recovery),
            Some(&receipt),
            None,
            None,
        )
        .expect("compact summary should exist");

        assert!(summary.stale_state_suspected);
        assert!(summary
            .route_truth
            .projection_reason
            .contains("looks stale"));
        assert_eq!(
            summary.recommended_command.as_deref(),
            Some("vida taskflow run-graph status run-stale --json")
        );
        assert_eq!(
            summary.recommended_surface.as_deref(),
            Some("vida taskflow run-graph status")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    async fn write_activation_snapshot_for_store(store: &StateStore) -> Result<(), String> {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let config = load_project_overlay_yaml_for_root(&project_root)?;
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, &project_root)
            .map_err(|error| format!("build compiled bundle: {error}"))?;
        let pack_router = pack_router_keywords_json(&config);
        let snapshot = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: project_root.join("vida.config.yaml").display().to_string(),
            source_config_digest: config_file_digest(&project_root.join("vida.config.yaml"))
                .map_err(|error| format!("read config digest: {error}"))?,
            captured_at: "2026-01-01T00:00:00Z".to_string(),
            compiled_bundle: bundle,
            pack_router_keywords: pack_router,
        };
        store
            .write_launcher_activation_snapshot(&snapshot)
            .await
            .map_err(|error| format!("write launcher activation snapshot: {error}"))?;
        Ok(())
    }

    #[tokio::test]
    async fn read_only_seed_snapshot_does_not_persist_refresh() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let before = store
            .read_launcher_activation_snapshot()
            .await
            .expect("read persisted activation snapshot");
        store.close().await;
        let read_only_store = StateStore::open_existing_read_only(harness.path().to_path_buf())
            .await
            .expect("open read-only store");
        let _ = crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload_read_only(
            &read_only_store,
        )
        .await;
        let observed = read_seed_launcher_activation_snapshot(&read_only_store, false)
            .await
            .expect("read-only seed snapshot should use persisted state");
        assert_eq!(observed, before);
        read_only_store.close().await;
        let after_store = StateStore::open_existing(harness.path().to_path_buf())
            .await
            .expect("reopen store after read-only consume");
        let after = after_store
            .read_launcher_activation_snapshot()
            .await
            .expect("read persisted activation snapshot after read-only consume");
        assert_eq!(after, before);
        after_store.close().await;
    }

    fn force_selected_model_ref(value: &mut serde_json::Value, model_ref: &str) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(selected_model_ref) = map.get_mut("selected_model_ref") {
                    if selected_model_ref.is_string() {
                        *selected_model_ref = serde_json::Value::String(model_ref.to_string());
                    }
                }
                for child in map.values_mut() {
                    force_selected_model_ref(child, model_ref);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    force_selected_model_ref(child, model_ref);
                }
            }
            _ => {}
        }
    }

    fn force_selected_backend_assignment(
        value: &mut serde_json::Value,
        backend_id: &str,
        model_profile_id: &str,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for key in [
                    "selected_backend",
                    "selected_backend_id",
                    "selected_carrier_id",
                    "selected_carrier_agent_id",
                    "selected_agent_id",
                    "activation_agent_type",
                    "selected_tier",
                ] {
                    if let Some(field) = map.get_mut(key) {
                        if field.is_string() {
                            *field = serde_json::Value::String(backend_id.to_string());
                        }
                    }
                }
                for key in ["selected_model_profile_id", "selected_model_profile"] {
                    if let Some(field) = map.get_mut(key) {
                        if field.is_string() {
                            *field = serde_json::Value::String(model_profile_id.to_string());
                        }
                    }
                }
                for child in map.values_mut() {
                    force_selected_backend_assignment(child, backend_id, model_profile_id);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    force_selected_backend_assignment(child, backend_id, model_profile_id);
                }
            }
            _ => {}
        }
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_bound_repair_with_file_scope_terms() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_state(
            &store,
            "task-repair-seed-1",
            "Repair scope and specification drift in crates/vida/src/runtime_lane_summary.rs, fix the file, add regression tests, and prove test coverage.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_ne!(payload.status.next_node.as_deref(), Some("spec-pack"));
    }

    #[test]
    fn request_text_task_id_match_requires_token_boundaries() {
        assert!(request_text_mentions_task_id(
            "Fix bounded task `taskflow-runtime-run-binding-task-missing-actionability` now.",
            "taskflow-runtime-run-binding-task-missing-actionability"
        ));
        assert!(request_text_mentions_task_id(
            "Fix taskflow-runtime-run-binding-task-missing-actionability.",
            "taskflow-runtime-run-binding-task-missing-actionability"
        ));
        assert!(!request_text_mentions_task_id(
            "Fix taskflow-runtime-run-binding-task-missing-actionability now.",
            "taskflow-runtime-run-binding-task-missing"
        ));
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_binds_generated_runtime_run_to_open_task_mentioned_in_request()
    {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let activation_snapshot = store
            .read_launcher_activation_snapshot()
            .await
            .expect("activation snapshot should round-trip");
        let authority = crate::team_flow_authority_adapter::TeamFlowExecutionAuthority::require(
            &activation_snapshot.compiled_bundle,
            None,
            None,
        )
        .expect("canonical persisted TeamFlow authority should validate at seed boundary");
        let projection = authority.projection();
        for (field, value) in [
            ("team_flow_authority_id", projection.authority_id.as_str()),
            (
                "team_flow_config_hash",
                projection.config_authority_hash.as_str(),
            ),
            (
                "team_flow_registry_hash",
                projection.registry_authority_hash.as_str(),
            ),
        ] {
            assert!(!value.trim().is_empty(), "{field} must be persisted");
        }
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "taskflow-runtime-run-binding-task-missing-actionability",
                title: "Runtime run binding task missing actionability",
                display_id: None,
                description: "Implement or adapt repo-owned WORKFLOW.md policy loading with typed config defaults and TUI-visible validation errors.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "crates/vida/src/taskflow_run_graph.rs".to_string(),
                        "crates/vida/src/runtime_dispatch_state.rs".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_reseeds_design_backed_explicit_binding_into_implementer_lane -- --nocapture".to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create active task");

        let payload = derive_seeded_run_graph_state(
            &store,
            "runtime-fix-runtime-run-binding-task-missing",
            "Fix runtime run binding task-missing actionability for taskflow-runtime-run-binding-task-missing-actionability.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(
            payload.status.run_id,
            "runtime-fix-runtime-run-binding-task-missing"
        );
        assert_eq!(
            payload.status.task_id,
            "taskflow-runtime-run-binding-task-missing-actionability"
        );
        let seeded_target = payload
            .status
            .next_node
            .as_deref()
            .expect("seeded run must expose an initial dispatch target");
        let selected_flow =
            crate::runtime_dispatch_state::selected_flow_ref(&payload.role_selection)
                .expect("seeded selection must expose the selected flow");
        let seeded_authority =
            crate::team_flow_authority_adapter::TeamFlowExecutionAuthority::require(
                &payload.role_selection.compiled_bundle,
                Some(selected_flow),
                None,
            )
            .expect("seeded selection flow must resolve through TeamFlow authority");
        let seeded_node = seeded_authority
            .resolve_target(None, seeded_target)
            .expect("seeded dispatch target must belong to the selected flow");
        let task = store
            .show_task("taskflow-runtime-run-binding-task-missing-actionability")
            .await
            .expect("seeded task should remain readable");
        let expected_task_class = crate::infer_task_class_from_task_payload(
            &serde_json::to_value(task).expect("seeded task should serialize"),
        );
        assert!(
            seeded_node.included,
            "seeded dispatch target must be included"
        );
        assert_eq!(seeded_node.task_class, expected_task_class);
        let dispatch_context = run_graph_dispatch_context_from_seed_payload(&payload);
        assert_eq!(
            dispatch_context.run_id,
            "runtime-fix-runtime-run-binding-task-missing"
        );
        assert_eq!(
            dispatch_context.task_id,
            "taskflow-runtime-run-binding-task-missing-actionability"
        );
        assert!(
            dispatch_context.role_selection["compiled_bundle"].is_null(),
            "persisted dispatch context intentionally stores no executable bundle"
        );
        let rehydrated = rehydrate_dispatch_context_role_selection(&store, &dispatch_context)
            .await
            .expect("restart rehydration must restore authoritative TeamFlow bundle");
        assert!(
            rehydrated.compiled_bundle["team_flow_authority"].is_object(),
            "rehydration must restore persisted TeamFlow authority"
        );
        assert!(
            rehydrated.execution_plan["development_flow"]["dispatch_contract"]["selected_flow_set"]
                .as_str()
                .is_some(),
            "rehydration must rebuild the selected flow contract"
        );

        let default_flow = rehydrated.compiled_bundle["default_flow_set"]
            .as_str()
            .expect("configured default flow id");
        let alternate_flow = rehydrated.compiled_bundle["team_flow_authority"]
            ["resolved_all_flow_payload"]["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| {
                flow["flow_id"].as_str() != Some(default_flow)
                    && flow["flow_policy"]["enabled"].as_bool() == Some(true)
            })
            .and_then(|flow| flow["flow_id"].as_str())
            .expect("canonical config should expose an enabled alternate flow")
            .to_string();
        let mut persisted_alternate = payload.role_selection.clone();
        persisted_alternate.compiled_bundle = serde_json::Value::Null;
        persisted_alternate.execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": alternate_flow.clone(),
        });
        persisted_alternate.matched_terms = vec![format!("dev_team_flow_id:{alternate_flow}")];
        let error = rehydrate_persisted_role_selection(&store, persisted_alternate, None)
            .await
            .expect_err("legacy initial state without selected node must fail closed");
        assert!(error.contains("team_flow_authority_selected_node_id_missing"));

        let mut tampered = payload.role_selection;
        tampered.compiled_bundle = serde_json::Value::Null;
        tampered.execution_plan = serde_json::json!({});
        tampered.matched_terms = vec!["dev_team_flow_id:unknown_persisted_flow".to_string()];
        let error = rehydrate_persisted_role_selection(&store, tampered, None)
            .await
            .expect_err("unknown persisted flow must fail closed");
        assert!(error.contains("team_flow_authority_unknown_flow"));
    }

    #[tokio::test]
    async fn rehydrate_persisted_role_selection_preserves_progressed_selected_node_identity() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_state(
            &store,
            "runtime-rehydrate-progressed-node",
            "Continue the configured implementation flow from its persisted execution cursor with verification proof.",
        )
        .await
        .expect("seed should be generated");
        let selected_flow =
            crate::runtime_dispatch_state::selected_flow_ref(&payload.role_selection)
                .expect("seeded selection must expose selected flow");
        let authority = crate::team_flow_authority_adapter::TeamFlowExecutionAuthority::require(
            &payload.role_selection.compiled_bundle,
            Some(selected_flow),
            None,
        )
        .expect("selected flow authority must compile");
        let progressed_node = authority
            .ordered_nodes()
            .filter(|node| node.node.included)
            .nth(1)
            .expect("configured flow must expose a progressed node")
            .node;
        let progressed_node_id = progressed_node.node_id.clone();
        let progressed_role = progressed_node.runtime_role.clone();
        let mut persisted = payload.role_selection.clone();
        persisted.selected_role = progressed_role;
        persisted.execution_plan["team_flow_authority_selected_node_id"] =
            serde_json::json!(progressed_node_id);
        persisted.execution_plan["development_flow"]["dispatch_contract"]["selected_node_id"] =
            serde_json::json!(progressed_node_id);
        persisted.execution_plan["development_flow"]["dispatch_contract"]
            ["team_flow_authority_selected_node_id"] = serde_json::json!(progressed_node_id);
        persisted.compiled_bundle = serde_json::Value::Null;

        let rehydrated = rehydrate_persisted_role_selection(&store, persisted, None)
            .await
            .expect("rehydrate must preserve the progressed selected node");
        assert_eq!(
            rehydrated.execution_plan["team_flow_authority_selected_node_id"],
            serde_json::json!(progressed_node_id)
        );
        assert_eq!(
            rehydrated.execution_plan["development_flow"]["dispatch_contract"]["selected_node_id"],
            serde_json::json!(progressed_node_id)
        );
        assert_eq!(
            rehydrated.execution_plan["development_flow"]["dispatch_contract"]
                ["team_flow_authority_selected_node_id"],
            serde_json::json!(progressed_node_id)
        );
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_keeps_design_spec_request_in_scope_discussion() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_state(
            &store,
            "task-design-seed-1",
            "Research the feature scope, write the specification and acceptance criteria.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.status.task_class, "scope_discussion");
        assert!(payload.role_selection.conversational_mode.is_some());
        assert_ne!(payload.status.route_task_class, "implementation");
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_existing_design_backed_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-existing-design-route-fix",
                title: "Existing design route fix",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "crates/vida/src/taskflow_run_graph.rs".to_string(),
                        "crates/vida/src/taskflow_consume.rs".to_string(),
                        "crates/vida/src/taskflow_consume_resume.rs".to_string(),
                        "crates/vida/src/runtime_dispatch_state.rs".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida design_backed -- --nocapture".to_string()
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create implementation-ready task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/existing-design-route-fix-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Existing Design Route Fix\n\nStatus: `proposed`\n\n## Bounded File Set\n- `crates/vida/src/taskflow_run_graph.rs`\n",
        )
        .expect("write existing design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "feature-existing-design-route-fix",
            "Review the existing design document, keep the specification context, and then implement the bounded current-release code fix without opening a new spec pack.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(
            payload.role_selection.reason,
            "configured_dev_team_first_step_dispatch_init"
        );
        assert_eq!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        // With design doc injected, execution plan status may differ
        let exec_status = payload.role_selection.execution_plan["status"].as_str();
        assert!(
            exec_status == Some("ready_for_runtime_routing") || exec_status == Some("design_first")
        );
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str(),
            Some("docs/product/spec/existing-design-route-fix-design.md")
        );
    }

    #[tokio::test]
    async fn dispatch_init_routes_unscoped_implementation_wording_through_analysis() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-workflow-md-policy-loader",
                title: "Add WORKFLOW policy loader",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "crates/vida/src/taskflow_run_graph.rs".to_string(),
                        "crates/vida/src/taskflow_consume.rs".to_string(),
                        "crates/vida/src/taskflow_consume_resume.rs".to_string(),
                        "crates/vida/src/runtime_dispatch_state.rs".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida design_backed -- --nocapture".to_string()
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create unscoped implementation task");

        let payload = run_graph_dispatch_init(&store, "feature-workflow-md-policy-loader")
            .await
            .expect("dispatch init should materialize an analysis packet");

        assert_eq!(payload["dispatch_receipt"]["dispatch_status"], "routed");
        assert_eq!(payload["dispatch_receipt"]["dispatch_target"], "developer");
        assert!(payload["dispatch_receipt"]["blocker_code"].is_null());
        let dispatch_packet_path = payload["dispatch_packet_path"]
            .as_str()
            .expect("dispatch packet path should be present");
        let dispatch_packet =
            crate::read_json_file_if_present(std::path::Path::new(dispatch_packet_path))
                .expect("dispatch packet should load");
        assert_eq!(dispatch_packet["dispatch_status"], "routed");
        assert_eq!(dispatch_packet["dispatch_target"], "developer");
        assert_eq!(
            dispatch_packet["delivery_task_packet"]["handoff_task_class"],
            "implementation"
        );
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_keeps_design_backed_qwen_remediation_out_of_worker_without_explicit_terms(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
                title: "Qwen carrier drift remediation",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create qwen remediation task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Qwen Remediation\n\nStatus: `approved`\n\n## Bounded File Set\n- `docs/process/agent-system.md`\n- `crates/vida/src/taskflow_run_graph.rs`\n- `crates/vida/src/taskflow_consume.rs`\n",
        )
        .expect("write qwen design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
            "Bounded audit-remediation task. Remove qwen_cli from active runtime/config/code/test assumptions and retain it only in template/reference surfaces where it is intentionally documented as a non-active example carrier.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "pm");
        assert_ne!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_ne!(
            payload.role_selection.reason,
            "auto_existing_design_backed_implementation_request_override"
        );
        assert!(!payload
            .role_selection
            .matched_terms
            .iter()
            .any(|term| term == "existing_design_backed_work_pool_override"
                || term == "existing_design_backed_generic_override"));
        assert_ne!(payload.status.route_task_class, "implementation");
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_keeps_design_backed_blocker_out_of_worker_without_explicit_terms(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-design-backed-reseed-blocker",
                title: "Design-backed reseed blocker",
                display_id: None,
                description: "Bounded audit-remediation blocker. A finalized design-backed task is still reseeded into specification/planning instead of continuing into the implementation lane.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create design-backed blocker task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/design-backed-reseed-blocker-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Design-backed reseed blocker\n\nStatus: `approved`\n\n## Bounded File Set\n- `crates/vida/src/taskflow_run_graph.rs`\n",
        )
        .expect("write blocker design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "feature-design-backed-reseed-blocker",
            "Bounded audit-remediation blocker. A finalized design-backed task is still reseeded into specification/planning instead of continuing into the implementation lane.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "pm");
        assert_ne!(
            payload.role_selection.reason,
            "auto_existing_design_backed_implementation_request_override"
        );
        assert!(payload
            .role_selection
            .matched_terms
            .iter()
            .all(|term| term != ".rs"
                && term != "crates/"
                && term != "src/"
                && term != "existing_design_backed_generic_override"));
        assert_ne!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_ne!(payload.status.route_task_class, "implementation");
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_does_not_promote_generic_design_doc_to_worker() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-generic-design-scope",
                title: "Generic design scope",
                display_id: None,
                description: "Clarify the product scope for an existing design document.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create generic design task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/generic-design-scope-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Generic Design Scope\n\nStatus: `approved`\n\n## Bounded File Set\n- `docs/product/spec/generic-design-scope-design.md`\n",
        )
        .expect("write generic design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "feature-generic-design-scope",
            "Clarify the product scope for an existing design document.",
        )
        .await
        .expect("seed should be generated");

        assert_ne!(
            payload.role_selection.reason,
            "auto_existing_design_backed_implementation_request_override"
        );
        assert!(!payload
            .role_selection
            .matched_terms
            .iter()
            .any(|term| term == "existing_design_backed_generic_override"));
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_injects_design_doc_for_direct_explicit_implementation_seed() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-direct-explicit-implementation-seed",
                title: "Direct explicit implementation seed",
                display_id: None,
                description: "A design-backed implementation task that should seed directly into worker implementation.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create direct explicit implementation task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/direct-explicit-implementation-seed-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Direct Explicit Implementation Seed\n\nStatus: `approved`\n\n## Bounded File Set\n- `crates/vida/src/taskflow_run_graph.rs`\n- `crates/vida/src/runtime_dispatch_state.rs`\n",
        )
        .expect("write approved design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "feature-direct-explicit-implementation-seed",
            "Implement the bounded fix for the design-backed dispatch-init regression and keep the registered design scope.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(
            payload.role_selection.reason,
            "auto_explicit_implementation_request"
        );
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str(),
            Some("docs/product/spec/direct-explicit-implementation-seed-design.md")
        );
    }

    #[tokio::test]
    async fn runtime_defect_design_backed_seed_uses_configured_first_step() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "runtime-defect-design-backed-dispatch-init-routing",
                title: "Runtime defect design-backed dispatch-init routing",
                display_id: None,
                description: "Implement the bounded runtime defect fix after the approved design doc and owned scope are ready.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create runtime defect task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/design-backed-dispatch-init-routing-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Design-backed Dispatch Init Routing\n\nStatus: `approved`\n\n## Bounded File Set\n- `crates/vida/src/taskflow_run_graph.rs`\n",
        )
        .expect("write approved design doc");

        let payload = derive_seeded_run_graph_state(
            &store,
            "runtime-defect-design-backed-dispatch-init-routing",
            "Implement the bounded runtime defect fix from the approved design doc for crates/vida/src/taskflow_run_graph.rs.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "business_analyst");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(
            payload.role_selection.reason,
            "configured_dev_team_first_step_dispatch_init"
        );
        assert!(payload
            .role_selection
            .matched_terms
            .iter()
            .any(|term| term == "dev_team_flow_id:runtime_defect_remediation"));
        assert_eq!(payload.status.task_class, "specification");
        assert_eq!(payload.status.route_task_class, "specification");
        assert_eq!(payload.status.next_node.as_deref(), Some("specifier"));
        assert_eq!(payload.status.handoff_state, "awaiting_specifier");
        let design_doc_path = payload.role_selection.execution_plan["tracked_flow_bootstrap"]
            ["design_doc_path"]
            .as_str()
            .expect("design doc path should be injected")
            .replace('\\', "/");
        assert_eq!(
            design_doc_path,
            "docs/product/spec/design-backed-dispatch-init-routing-design.md"
        );
    }

    #[test]
    fn implementation_analysis_gate_tracks_coach_and_verification_requirements() {
        let implementation = serde_json::json!({
            "coach_required": true,
            "coach_route_task_class": "coach",
            "verification_gate": "targeted_verification",
            "independent_verification_required": true
        });

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);
        assert_eq!(next_node, Some("writer".to_string()));
        assert_eq!(policy_gate, "targeted_verification");
        assert!(recovery_ready);
    }

    #[test]
    fn implementation_analysis_gate_keeps_writer_step_when_coach_is_disabled() {
        let implementation = serde_json::json!({
            "coach_required": false,
            "independent_verification_required": false
        });

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);
        assert_eq!(next_node, Some("writer".to_string()));
        assert_eq!(policy_gate, "not_required");
        assert!(recovery_ready);
    }

    #[test]
    fn implementation_verification_gate_falls_back_when_independent_review_is_disabled() {
        let implementation = serde_json::json!({
            "verification_route_task_class": "review_ensemble",
            "independent_verification_required": false
        });
        let verification = serde_json::json!({
            "verification_gate": "review_findings"
        });

        let (next_node, policy_gate) =
            implementation_verification_gate(&implementation, &verification);
        assert_eq!(next_node, None);
        assert_eq!(policy_gate, "not_required");
    }

    #[test]
    fn implementation_verification_outcome_uses_expected_table_mappings() {
        assert_eq!(
            implementation_verification_outcome("rework_ready"),
            ImplementationVerificationOutcome::ReworkReady
        );
        assert_eq!(
            implementation_verification_outcome("clean"),
            ImplementationVerificationOutcome::Clean
        );
        assert_eq!(
            implementation_verification_outcome("approved"),
            ImplementationVerificationOutcome::Approved
        );
        assert_eq!(
            implementation_verification_outcome("denied"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("expired"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("review_findings"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("blocker"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("rework_required"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
        assert_eq!(
            implementation_verification_outcome("changed_scope"),
            ImplementationVerificationOutcome::FindingsBlocked
        );
    }

    #[tokio::test]
    async fn dispatch_init_materializes_first_persisted_dispatch_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-1",
                title: "Task 1",
                display_id: None,
                description: "dispatch-init identity backing task",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create backing task");
        let status = RunGraphStatus {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "running".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "open".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "worker".to_string(),
            request: "Implement one bounded patch in crates/vida/src/taskflow_run_graph.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: json!({
                "orchestration_contract": {},
                "runtime_assignment": {
                    "selected_agent_id": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                },
                "development_flow": {
                    "lane_sequence": ["implementer", "coach", "verification"],
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "closure_class": "implementation"
                            },
                            "coach": {
                                "activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "coach"
                                },
                                "closure_class": "review"
                            },
                            "verification": {
                                "activation": {
                                    "activation_agent_type": "architect",
                                    "activation_runtime_role": "verifier"
                                },
                                "closure_class": "verification"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run status");
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: "task-1".to_string(),
                task_id: "task-1".to_string(),
                request_text: role_selection.request.clone(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("role selection should encode"),
                recorded_at: "2026-04-10T10:00:00Z".to_string(),
            })
            .await
            .expect("record dispatch context");

        let artifacts = prepare_run_graph_dispatch_init_artifacts(&store, "task-1")
            .await
            .expect("dispatch init artifacts should be prepared");
        assert_eq!(artifacts.requested_run_id, "task-1");
        assert_eq!(artifacts.run_id, "task-1");
        assert_eq!(artifacts.role_selection.selected_role, "worker");
        assert!(artifacts.dispatch_receipt.dispatch_packet_path.is_some());
        let payload = artifacts.into_json_payload();
        let receipt = store
            .run_graph_dispatch_receipt("task-1")
            .await
            .expect("read receipt")
            .expect("receipt present");

        assert_eq!(
            receipt.dispatch_target, "junior",
            "dispatch target follows the current configured worker lane mapping"
        );
        assert!(receipt.dispatch_packet_path.is_some());
        assert!(payload["dispatch_packet_path"].as_str().is_some());
        let identity = store
            .run_graph_dispatch_task_identity("task-1")
            .await
            .expect("read dispatch task identity")
            .expect("non-cli dispatch-init should record task identity");
        assert_eq!(identity.run_id, "task-1");
        assert_eq!(identity.dev_task_id.as_deref(), Some("task-1"));
        assert_eq!(identity.source, "dispatch_init_existing_task");
    }

    #[tokio::test]
    async fn dispatch_init_fast_cache_backfill_helper_records_missing_task_identity() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "cache-epic",
                title: "Cache Epic",
                display_id: None,
                description: "identity parent",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent epic");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "cache-task",
                title: "Cache Task",
                display_id: None,
                description: "identity child",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("cache-epic"),
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child task");
        store.close().await;

        try_backfill_dispatch_init_task_identity(harness.path(), "cache-task").await;
        let store = StateStore::open_existing(harness.path().to_path_buf())
            .await
            .expect("reopen store");
        let identity = store
            .run_graph_dispatch_task_identity("cache-task")
            .await
            .expect("read dispatch task identity")
            .expect("backfill helper should persist identity");
        assert_eq!(identity.run_id, "cache-task");
        assert_eq!(identity.feature_epic_id.as_deref(), Some("cache-epic"));
        assert_eq!(identity.dev_task_id.as_deref(), Some("cache-task"));
        assert_eq!(identity.source, "dispatch_init_existing_task");
        store.close().await;
    }

    #[tokio::test]
    async fn dispatch_init_preview_derives_missing_task_identity_without_persisting() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "preview-epic",
                title: "Preview Epic",
                display_id: None,
                description: "identity parent",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent epic");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "preview-task",
                title: "Preview Task",
                display_id: None,
                description: "identity child",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("preview-epic"),
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child task");
        store.close().await;

        let read_only_store = StateStore::open_existing_read_only(harness.path().to_path_buf())
            .await
            .expect("open read-only store");
        let identity = preview_dispatch_init_task_identity(&read_only_store, "preview-task")
            .await
            .expect("preview identity should derive")
            .expect("preview identity should be present");
        assert_eq!(identity.run_id, "preview-task");
        assert_eq!(identity.feature_epic_id.as_deref(), Some("preview-epic"));
        assert_eq!(identity.dev_task_id.as_deref(), Some("preview-task"));
        read_only_store.close().await;

        let store = StateStore::open_existing(harness.path().to_path_buf())
            .await
            .expect("reopen store");
        let persisted_identity = store
            .run_graph_dispatch_task_identity("preview-task")
            .await
            .expect("read dispatch task identity");
        assert!(
            persisted_identity.is_none(),
            "dispatch-init preview must not persist task identity through a read-only state handle"
        );
        store.close().await;
    }

    #[tokio::test]
    async fn dispatch_init_uses_task_planner_metadata_owned_paths_for_writer_packet() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let owned_paths = vec![
            "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            "crates/vida/src/runtime_dispatch_downstream_packets.rs".to_string(),
        ];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-writer-planner-scope",
                title: "Writer planner scope",
                display_id: None,
                description: "dev packet with planner-owned paths",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["dev-pack".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: owned_paths.clone(),
                    acceptance_targets: vec![
                        "writer dispatch packet has owned scope".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_uses_task_planner_metadata_owned_paths_for_writer_packet -- --nocapture".to_string(),
                    ],
                    risk: None,
                    estimate: None,
                    lane_hint: None,
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create scoped task");
        let status = RunGraphStatus {
            run_id: "task-writer-planner-scope".to_string(),
            task_id: "task-writer-planner-scope".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: Some("writer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            recovery_ready: true,
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "worker".to_string(),
            request: "tracked dev packet created from runtime consumption".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "fallback".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: json!({
                "orchestration_contract": {},
                "runtime_assignment": {
                    "selected_agent_id": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                },
                "development_flow": {
                    "implementation": {
                        "analysis_route_task_class": "analysis",
                        "writer_route_task_class": "writer"
                    },
                    "lane_sequence": ["analysis", "writer", "coach", "verification"],
                    "dispatch_contract": {
                        "lane_catalog": {
                            "analysis": {
                                "activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                },
                                "closure_class": "analysis"
                            },
                            "writer": {
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "closure_class": "implementation"
                            }
                        }
                    }
                },
                "tracked_flow_bootstrap": null
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("record run status");
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: "task-writer-planner-scope".to_string(),
                task_id: "task-writer-planner-scope".to_string(),
                request_text: role_selection.request.clone(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("role selection should encode"),
                recorded_at: "2026-04-10T10:00:00Z".to_string(),
            })
            .await
            .expect("record dispatch context");
        store
            .record_run_graph_dispatch_task_identity(
                &crate::state_store::RunGraphDispatchTaskIdentity {
                    run_id: "task-writer-planner-scope".to_string(),
                    feature_epic_id: Some("feature-writer".to_string()),
                    spec_task_id: Some("feature-writer-spec".to_string()),
                    work_pool_task_id: Some("feature-writer-work-pool".to_string()),
                    dev_task_id: Some("feature-writer-dev-pack".to_string()),
                    source: "seeded_richer_identity".to_string(),
                    updated_at: "2026-06-05T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("seed richer identity");

        let artifacts =
            prepare_run_graph_dispatch_init_artifacts(&store, "task-writer-planner-scope")
                .await
                .expect("dispatch init artifacts should be prepared from planner metadata");
        assert_eq!(artifacts.dispatch_receipt.dispatch_target, "developer");
        assert!(artifacts
            .dispatch_receipt
            .downstream_dispatch_blockers
            .is_empty());

        let packet =
            crate::read_json_file_if_present(std::path::Path::new(&artifacts.dispatch_packet_path))
                .expect("dispatch packet should load");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_downstream_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
        assert_eq!(packet["task_id"], "task-writer-planner-scope");
        assert_eq!(
            packet["owned_paths"],
            packet["delivery_task_packet"]["owned_paths"]
        );
        assert_eq!(
            packet["implementation_isolation"]["owned_paths"],
            packet["delivery_task_packet"]["owned_paths"]
        );
        let identity = store
            .run_graph_dispatch_task_identity("task-writer-planner-scope")
            .await
            .expect("read preserved dispatch task identity")
            .expect("richer identity should remain present");
        assert_eq!(
            identity.spec_task_id.as_deref(),
            Some("feature-writer-spec")
        );
        assert_eq!(
            identity.work_pool_task_id.as_deref(),
            Some("feature-writer-work-pool")
        );
        assert_eq!(
            identity.dev_task_id.as_deref(),
            Some("feature-writer-dev-pack")
        );
        assert_eq!(identity.source, "seeded_richer_identity");
    }

    #[tokio::test]
    async fn reseed_clears_stale_blocked_dispatch_receipt_before_dispatch_init() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let stale_status = RunGraphStatus {
            run_id: "task-reseed-1".to_string(),
            task_id: "task-reseed-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "legacy_removed_backend".to_string(),
            lane_id: "implementer_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer_lane".to_string(),
            recovery_ready: false,
        };
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale run status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "task-reseed-1".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("qwen".to_string()),
                dispatch_packet_path: Some("/tmp/stale-dispatch-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/stale-dispatch-result.json".to_string()),
                blocker_code: Some("stale_receipt".to_string()),
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale downstream note".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("legacy_removed_backend".to_string()),
                policy_bundle_ref: None,
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale dispatch receipt");

        let payload = derive_seeded_run_graph_state(
            &store,
            "task-reseed-1",
            "Fix the exact in-process test hang in runtime_dispatch_state by removing nested EnvVarGuard acquisition and preserving harness-local state isolation. Owned paths: crates/vida/src/runtime_dispatch_state.rs.",
        )
        .await
        .expect("seed should be generated");
        let reseeded_backend = payload.status.selected_backend.clone();
        assert_ne!(
            reseeded_backend, "legacy_removed_backend",
            "fresh reseed should not preserve stale blocked dispatch backend lineage"
        );
        assert!(payload.status.recovery_ready);

        persist_seed_artifacts(&store, &payload)
            .await
            .expect("persist seeded artifacts should succeed");

        let reconciled = store
            .run_graph_status("task-reseed-1")
            .await
            .expect("reseeded run status should load");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.selected_backend, reseeded_backend);
        assert!(reconciled.recovery_ready);

        assert!(
            store
                .run_graph_dispatch_receipt("task-reseed-1")
                .await
                .expect("dispatch receipt lookup should succeed")
                .is_none(),
            "fresh reseed should clear stale pre-dispatch receipt lineage"
        );

        let dispatch_init = run_graph_dispatch_init(&store, "task-reseed-1")
            .await
            .expect("dispatch init should succeed after reseed");
        assert_eq!(
            dispatch_init["dispatch_receipt"]["selected_backend"].as_str(),
            Some(reseeded_backend.as_str())
        );
    }

    #[tokio::test]
    async fn dispatch_init_reconciles_active_exception_takeover_status_for_replay() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let run_id = "task-exception-replay";
        let payload = derive_seeded_run_graph_state(
            &store,
            run_id,
            "Fix run-graph dispatch-init replay after active exception takeover. Owned paths: crates/vida/src/taskflow_run_graph.rs.",
        )
        .await
        .expect("seed should be generated");
        store
            .acquire_current_session_run_graph_claim_for_test(
                "dispatch-init-exception-replay-claim",
                run_id,
                run_id,
                "runtime-recovery-contract",
                "crates/vida/src/taskflow_run_graph.rs",
            )
            .await
            .expect("current session should claim dispatch-init exception replay");
        persist_seed_artifacts(&store, &payload)
            .await
            .expect("persist seeded artifacts should succeed");

        let mut blocked_status = payload.status.clone();
        blocked_status.active_node = "implementer".to_string();
        blocked_status.next_node = None;
        blocked_status.handoff_state = "none".to_string();
        blocked_status.resume_target = "none".to_string();
        blocked_status.recovery_ready = false;
        store
            .record_run_graph_status(&blocked_status)
            .await
            .expect("persist blocked exception status");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("sup-exception-replay".to_string()),
                exception_path_receipt_id: Some("exc-exception-replay".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/stale-exception-replay-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/stale-exception-replay-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: Some("implementer".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some(blocked_status.selected_backend.clone()),
                policy_bundle_ref: None,
                recorded_at: "2026-05-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist active exception receipt");

        let dispatch_init = run_graph_dispatch_init(&store, run_id)
            .await
            .expect("dispatch init should reconcile active exception takeover");
        assert_eq!(
            dispatch_init["run_graph_bootstrap"]["latest_status"]["recovery_ready"],
            serde_json::json!(true)
        );
        assert_eq!(
            dispatch_init["run_graph_bootstrap"]["latest_status"]["resume_target"],
            serde_json::json!("dispatch.implementer_lane")
        );
        assert_eq!(
            dispatch_init["dispatch_receipt"]["dispatch_target"].as_str(),
            Some("implementer")
        );
    }

    #[tokio::test]
    async fn dispatch_init_reseeds_explicit_task_graph_binding_into_bound_task_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let mut stale_status = default_run_graph_state("run-old", "closure", "delivery");
        stale_status.task_id = "run-old".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.policy_gate = "validation_report_required".to_string();
        stale_status.context_state = "sealed".to_string();
        stale_status.checkpoint_kind = "execution_cursor".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = true;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-old".to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": "run-old",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "reseed onto task-new".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("Fix the runtime bridge for explicit task bindings in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/taskflow_packet.rs.".to_string()),
                    recorded_at: "2026-04-16T09:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit binding");

        let payload = run_graph_dispatch_init(&store, "run-old")
            .await
            .expect("dispatch init should reseed and succeed");

        assert_eq!(payload["requested_run_id"], "run-old");
        assert_eq!(payload["run_id"], "task-new");
        assert_eq!(payload["dispatch_receipt"]["run_id"], "task-new");

        let reseeded_status = store
            .run_graph_status("task-new")
            .await
            .expect("reseeded task run should exist");
        assert_eq!(reseeded_status.task_id, "task-new");
        assert_eq!(reseeded_status.run_id, "task-new");
        assert!(
            matches!(reseeded_status.status.as_str(), "ready" | "blocked"),
            "unexpected reseeded status: {}",
            reseeded_status.status
        );

        let reseeded_receipt = store
            .run_graph_dispatch_receipt("task-new")
            .await
            .expect("reseeded receipt lookup should succeed")
            .expect("reseeded receipt should exist");
        assert_eq!(reseeded_receipt.run_id, "task-new");
        assert!(reseeded_receipt.dispatch_packet_path.is_some());
    }

    #[tokio::test]
    async fn dispatch_init_rejects_explicit_task_graph_binding_to_closed_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let labels = vec![String::from("runtime-recovery")];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-closed-bound",
                title: "Closed bound task",
                display_id: None,
                description: "dispatch-init must not reseed terminal continuation targets",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed bound task");

        let mut stale_status = default_run_graph_state("run-old-closed", "closure", "delivery");
        stale_status.task_id = "run-old-closed".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.next_node = None;
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = false;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale closed status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-old-closed".to_string(),
                    task_id: "task-closed-bound".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-closed-bound",
                        "run_id": "run-old-closed",
                        "task_status": "closed",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "stale closed task binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("closed task must not reseed".to_string()),
                    recorded_at: "2026-04-16T09:30:00Z".to_string(),
                },
            )
            .await
            .expect("persist closed explicit binding");

        let error = run_graph_dispatch_init(&store, "run-old-closed")
            .await
            .expect_err("dispatch-init must reject terminal task_graph_task binding");

        assert!(
            error.contains(
                "No persisted seeded dispatch context exists for run_id `run-old-closed`"
            ),
            "unexpected dispatch-init error: {error}"
        );
    }

    #[tokio::test]
    async fn dispatch_init_seeds_existing_task_without_prior_run_graph_state() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "taskflow-graph-summary-open-cycle-gate",
                title: "Graph summary must respect open delegated cycle gate",
                display_id: None,
                description: "Fix graph-summary so operator status and next actions account for active run/recovery gates before backlog ready-head guidance.",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
                    proof_targets: vec![
                        "vida taskflow graph-summary --json reports open_delegated_cycle when an active delegated cycle is open."
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create taskflow task");

        let payload = run_graph_dispatch_init(&store, "taskflow-graph-summary-open-cycle-gate")
            .await
            .expect("dispatch-init should seed the existing task and succeed");

        assert_eq!(
            payload["requested_run_id"],
            "taskflow-graph-summary-open-cycle-gate"
        );
        assert_eq!(payload["run_id"], "taskflow-graph-summary-open-cycle-gate");
        assert_eq!(
            payload["dispatch_receipt"]["run_id"],
            "taskflow-graph-summary-open-cycle-gate"
        );
        assert!(payload["dispatch_packet_path"].as_str().is_some());

        let status = store
            .run_graph_status("taskflow-graph-summary-open-cycle-gate")
            .await
            .expect("seeded run-graph status should exist");
        assert_eq!(status.task_id, "taskflow-graph-summary-open-cycle-gate");

        let context = store
            .run_graph_dispatch_context("taskflow-graph-summary-open-cycle-gate")
            .await
            .expect("seeded dispatch context lookup should succeed")
            .expect("seeded dispatch context should exist");
        assert_eq!(
            context.role_selection["compiled_bundle"],
            serde_json::Value::Null
        );
        assert!(context
            .request_text
            .contains("Graph summary must respect open delegated cycle gate"));
        assert!(context
            .request_text
            .contains("crates/vida/src/taskflow_proxy.rs"));
    }

    #[tokio::test]
    async fn dispatch_init_mutation_seeds_existing_task_via_read_mostly_path() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-read-mostly",
                title: "Dispatch init should not hold writable store during preparation",
                display_id: None,
                description: "The CLI mutation path should preview from a read-only store and commit only the prepared artifacts.",
                issue_type: "defect",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &[String::from("runtime-recovery")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "vida taskflow run-graph dispatch-init <task-id> --json returns a bounded dispatch receipt without timing out on normal existing TaskFlow tasks."
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create dispatch-init task");
        store.close().await;

        let exit = run_taskflow_run_graph_dispatch_init_mutation(
            harness.path(),
            "task-dispatch-init-read-mostly",
            true,
        )
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("reopen store");
        let status = store
            .run_graph_status("task-dispatch-init-read-mostly")
            .await
            .expect("status lookup should succeed");
        assert_eq!(status.run_id, "task-dispatch-init-read-mostly");
        assert!(store
            .run_graph_dispatch_receipt("task-dispatch-init-read-mostly")
            .await
            .expect("receipt lookup should succeed")
            .is_some());
    }

    #[tokio::test]
    async fn dispatch_init_mutation_captures_stale_launcher_snapshot_without_preview_write() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let mut snapshot = store
            .read_launcher_activation_snapshot()
            .await
            .expect("snapshot should load");
        snapshot.source_config_digest = "stale-test-digest".to_string();
        store
            .write_launcher_activation_snapshot(&snapshot)
            .await
            .expect("stale snapshot should persist");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-stale-snapshot",
                title: "Dispatch init should capture stale launcher snapshot read-only",
                display_id: None,
                description: "A stale launcher snapshot must not force a write during dispatch-init preview.",
                issue_type: "defect",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &[String::from("runtime-recovery")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "dispatch-init succeeds after release install refreshes activation assets and makes the persisted launcher snapshot stale."
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create dispatch-init task");
        store.close().await;

        let exit = run_taskflow_run_graph_dispatch_init_mutation(
            harness.path(),
            "task-dispatch-init-stale-snapshot",
            true,
        )
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("reopen store");
        assert!(store
            .run_graph_dispatch_receipt("task-dispatch-init-stale-snapshot")
            .await
            .expect("receipt lookup should succeed")
            .is_some());
    }

    #[tokio::test]
    async fn dispatch_init_rejects_closed_existing_task_without_prior_run_graph_state() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let labels = vec![String::from("runtime-recovery")];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-closed-dispatch-init",
                title: "Closed dispatch init task",
                display_id: None,
                description: "dispatch-init must fail closed for terminal tasks",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed dispatch-init task");

        let error = run_graph_dispatch_init(&store, "task-closed-dispatch-init")
            .await
            .expect_err("dispatch-init must reject closed task");

        assert!(error.contains("cannot seed terminal TaskFlow task `task-closed-dispatch-init`"));
    }

    #[tokio::test]
    async fn dispatch_init_repairs_seeded_status_missing_dispatch_context() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-context-repair",
                title: "Repair dispatch-init context gap",
                display_id: None,
                description: "dispatch-init must recover a seeded status that was persisted without context",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_repairs_seeded_status_missing_dispatch_context -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create repair task");

        let payload = derive_seeded_run_graph_state(
            &store,
            "task-dispatch-context-repair",
            "Fix dispatch-init partial seed recovery. Owned paths: crates/vida/src/taskflow_run_graph.rs.",
        )
        .await
        .expect("seed status should derive");
        store
            .record_run_graph_status(&payload.status)
            .await
            .expect("persist status without context");
        assert!(store
            .run_graph_dispatch_context("task-dispatch-context-repair")
            .await
            .expect("context lookup should succeed")
            .is_none());

        let dispatch_init = run_graph_dispatch_init(&store, "task-dispatch-context-repair")
            .await
            .expect("dispatch-init should repair missing context and succeed");

        assert_eq!(dispatch_init["run_id"], "task-dispatch-context-repair");
        assert!(dispatch_init["dispatch_packet_path"].as_str().is_some());
        assert!(store
            .run_graph_dispatch_context("task-dispatch-context-repair")
            .await
            .expect("repaired context lookup should succeed")
            .is_some());
        assert!(store
            .run_graph_dispatch_receipt("task-dispatch-context-repair")
            .await
            .expect("receipt lookup should succeed")
            .is_some());
    }

    #[tokio::test]
    async fn dispatch_init_reuses_existing_seeded_context_without_reseeding() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-context-fast-path",
                title: "Task record title should not replace seeded context",
                display_id: None,
                description: "Task record description should not replace seeded context",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_reuses_existing_seeded_context_without_reseeding -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create fast-path task");

        let seeded_request_text = "preseeded dispatch context fast path";
        let payload = derive_seeded_run_graph_state(
            &store,
            "task-dispatch-context-fast-path",
            seeded_request_text,
        )
        .await
        .expect("seed status should derive");
        persist_seed_artifacts(&store, &payload)
            .await
            .expect("seed artifacts should persist");

        let dispatch_init = run_graph_dispatch_init(&store, "task-dispatch-context-fast-path")
            .await
            .expect("dispatch-init should reuse existing context and succeed");

        assert_eq!(dispatch_init["run_id"], "task-dispatch-context-fast-path");
        let context = store
            .run_graph_dispatch_context("task-dispatch-context-fast-path")
            .await
            .expect("context lookup should succeed")
            .expect("seeded context should remain present");
        assert_eq!(context.request_text, seeded_request_text);
    }

    #[tokio::test]
    async fn dispatch_init_reuses_existing_routed_receipt_packet() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-idempotent-fast-path",
                title: "Dispatch init should reuse an existing routed packet",
                display_id: None,
                description: "Repeated dispatch-init must be read-mostly once routed packet evidence exists.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_reuses_existing_routed_receipt_packet -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create idempotent task");

        let _first = run_graph_dispatch_init(&store, "task-dispatch-init-idempotent-fast-path")
            .await
            .expect("first dispatch-init should seed and route");
        let second = run_graph_dispatch_init(&store, "task-dispatch-init-idempotent-fast-path")
            .await
            .expect("second dispatch-init should reuse existing routed packet");

        assert_eq!(second["dispatch_receipt"]["dispatch_status"], "routed");
        let second_packet_path = second["dispatch_packet_path"]
            .as_str()
            .expect("second packet path should be present");
        assert!(
            std::path::Path::new(second_packet_path).is_file(),
            "second dispatch-init should return a valid routed packet path"
        );
        let mut disabled_backend_cache_payload = second.clone();
        disabled_backend_cache_payload["dispatch_receipt"]["selected_backend"] =
            serde_json::Value::String("pi_cli".to_string());
        let current_config_digest = current_dispatch_init_cache_config_digest(store.root());
        assert!(!dispatch_init_fast_cache_payload_is_reusable(
            store.root(),
            &disabled_backend_cache_payload,
            "task-dispatch-init-idempotent-fast-path",
            current_config_digest.as_deref()
        ));
        assert!(read_run_graph_dispatch_init_fast_cache(
            store.root(),
            "task-dispatch-init-idempotent-fast-path"
        )
        .is_some());
        let cache_path = run_graph_dispatch_init_fast_cache_path(
            store.root(),
            "task-dispatch-init-idempotent-fast-path",
        );
        let mut legacy_cache_payload: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&cache_path).expect("cache should be readable"),
        )
        .expect("cache should decode");
        legacy_cache_payload
            .as_object_mut()
            .expect("cache payload should be object")
            .remove("dispatch_init_fast_cache_schema_version");
        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&legacy_cache_payload).expect("cache should encode"),
        )
        .expect("legacy cache payload should write");
        assert!(read_run_graph_dispatch_init_fast_cache(
            store.root(),
            "task-dispatch-init-idempotent-fast-path"
        )
        .is_none());
        write_run_graph_dispatch_init_fast_cache(
            store.root(),
            "task-dispatch-init-idempotent-fast-path",
            "task-dispatch-init-idempotent-fast-path",
            &second,
        )
        .expect("cache should rewrite with current schema");
        assert!(read_run_graph_dispatch_init_fast_cache(
            store.root(),
            "task-dispatch-init-idempotent-fast-path"
        )
        .is_some());

        let mut receipt = store
            .run_graph_dispatch_receipt("task-dispatch-init-idempotent-fast-path")
            .await
            .expect("receipt lookup should succeed")
            .expect("receipt should exist");
        receipt.dispatch_status = "executing".to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("non-routed receipt should persist and invalidate fast cache");
        assert!(read_run_graph_dispatch_init_fast_cache(
            store.root(),
            "task-dispatch-init-idempotent-fast-path"
        )
        .is_none());
    }

    #[tokio::test]
    async fn dispatch_init_rebuilds_routed_packet_with_stale_selected_backend() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-stale-backend-packet",
                title: "Dispatch init refreshes stale packet backend",
                display_id: None,
                description: "Repeated dispatch-init must not reuse a routed packet whose embedded backend no longer matches current config.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_rebuilds_routed_packet_with_stale_selected_backend -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create stale packet task");

        let first = run_graph_dispatch_init(&store, "task-dispatch-init-stale-backend-packet")
            .await
            .expect("first dispatch-init should seed and route");
        let first_packet_path = first["dispatch_packet_path"]
            .as_str()
            .expect("first packet path")
            .to_string();
        let mut stale_packet =
            crate::read_json_file_if_present(std::path::Path::new(&first_packet_path))
                .expect("stale packet should be readable");
        stale_packet["runtime_assignment"]["selected_backend_id"] =
            serde_json::Value::String("legacy_middle".to_string());
        stale_packet["runtime_assignment"]["selected_carrier_id"] =
            serde_json::Value::String("legacy_middle".to_string());
        std::fs::write(
            &first_packet_path,
            serde_json::to_string_pretty(&stale_packet).expect("stale packet should encode"),
        )
        .expect("stale packet should write");
        let receipt = store
            .run_graph_dispatch_receipt("task-dispatch-init-stale-backend-packet")
            .await
            .expect("receipt lookup should succeed")
            .expect("receipt should exist");
        assert!(!existing_dispatch_receipt_matches_current_seed(
            &store,
            "task-dispatch-init-stale-backend-packet",
            &receipt,
            None
        )
        .await
        .expect("stale packet check should succeed"));

        let second = run_graph_dispatch_init(&store, "task-dispatch-init-stale-backend-packet")
            .await
            .expect("second dispatch-init should rebuild stale routed packet");
        assert_ne!(
            first["dispatch_packet_path"],
            second["dispatch_packet_path"]
        );
        let second_packet_path = second["dispatch_packet_path"]
            .as_str()
            .expect("second packet path");
        assert_ne!(
            dispatch_init_packet_selected_backend(second_packet_path).as_deref(),
            Some("legacy_middle")
        );
    }

    #[tokio::test]
    async fn dispatch_init_rebuilds_routed_packet_with_stale_owned_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-stale-owned-scope-packet",
                title: "Dispatch init refreshes stale packet scope",
                display_id: None,
                description: "Repeated dispatch-init must not reuse a routed implementation packet whose task_id or owned_paths no longer match current planner metadata.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "Cargo.toml".to_string(),
                        "crates/taskflow-state".to_string(),
                        "crates/taskflow-state-redb".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_rebuilds_routed_packet_with_stale_owned_scope -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create stale owned-scope packet task");

        let first = run_graph_dispatch_init(&store, "task-dispatch-init-stale-owned-scope-packet")
            .await
            .expect("first dispatch-init should seed and route");
        let first_packet_path = first["dispatch_packet_path"]
            .as_str()
            .expect("first packet path")
            .to_string();
        let mut stale_packet =
            crate::read_json_file_if_present(std::path::Path::new(&first_packet_path))
                .expect("stale packet should be readable");
        let docs_only_scope = serde_json::json!([
            "docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md",
            "docs/product/decisions/ldr-002-redb-operational-journal-adr.md"
        ]);
        if let Some(delivery_packet) = stale_packet["delivery_task_packet"].as_object_mut() {
            delivery_packet.remove("task_id");
            delivery_packet.insert("owned_paths".to_string(), docs_only_scope.clone());
            delivery_packet["implementation_isolation"]["owned_paths"] = docs_only_scope;
        }
        std::fs::write(
            &first_packet_path,
            serde_json::to_string_pretty(&stale_packet).expect("stale packet should encode"),
        )
        .expect("stale packet should write");
        let receipt = store
            .run_graph_dispatch_receipt("task-dispatch-init-stale-owned-scope-packet")
            .await
            .expect("receipt lookup should succeed")
            .expect("receipt should exist");
        assert!(!existing_dispatch_receipt_matches_current_seed(
            &store,
            "task-dispatch-init-stale-owned-scope-packet",
            &receipt,
            None
        )
        .await
        .expect("stale owned-scope packet check should succeed"));

        let second = run_graph_dispatch_init(&store, "task-dispatch-init-stale-owned-scope-packet")
            .await
            .expect("second dispatch-init should rebuild stale owned-scope packet");
        assert_ne!(
            first["dispatch_packet_path"],
            second["dispatch_packet_path"]
        );
        let second_packet_path = second["dispatch_packet_path"]
            .as_str()
            .expect("second packet path");
        let expected_owned_paths = vec![
            "Cargo.toml".to_string(),
            "crates/taskflow-state".to_string(),
            "crates/taskflow-state-redb".to_string(),
        ];
        assert_eq!(
            dispatch_init_delivery_packet_string_field(second_packet_path, "task_id").as_deref(),
            Some("task-dispatch-init-stale-owned-scope-packet")
        );
        assert_eq!(
            dispatch_init_delivery_packet_string_array(second_packet_path, "owned_paths")
                .expect("owned paths should be present"),
            expected_owned_paths
        );
        assert_eq!(
            dispatch_init_delivery_packet_implementation_isolation_owned_paths(second_packet_path)
                .expect("implementation isolation should carry owned paths"),
            expected_owned_paths
        );
    }

    #[tokio::test]
    async fn dispatch_init_does_not_reuse_routed_receipt_with_disabled_external_backend() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-disabled-receipt-refresh",
                title: "Dispatch init ignores disabled selected backend receipts",
                display_id: None,
                description: "Repeated dispatch-init must not reuse a routed receipt whose selected backend is disabled in current config.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_does_not_reuse_routed_receipt_with_disabled_external_backend -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create disabled receipt refresh task");

        let first = run_graph_dispatch_init(&store, "task-dispatch-init-disabled-receipt-refresh")
            .await
            .expect("first dispatch-init should seed and route");
        let mut receipt = store
            .run_graph_dispatch_receipt("task-dispatch-init-disabled-receipt-refresh")
            .await
            .expect("receipt lookup should succeed")
            .expect("receipt should exist");
        receipt.dispatch_status = "routed".to_string();
        receipt.dispatch_packet_path = first["dispatch_packet_path"].as_str().map(str::to_string);
        receipt.selected_backend = Some("pi_cli".to_string());
        receipt.activation_agent_type = Some("pi_cli".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("stale disabled-backend receipt should persist");
        std::fs::write(harness.path().join("AGENTS.md"), "# test project\n")
            .expect("write test project marker");
        std::fs::create_dir_all(harness.path().join(".vida/config"))
            .expect("create test project config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db"))
            .expect("create test project db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project"))
            .expect("create test project metadata dir");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
agent_extensions:
  role_selection:
    mode: auto
    fallback_role: orchestrator
agent_system:
  subagents:
    pi_cli:
      enabled: false
      subagent_backend_class: external_cli
"#,
        )
        .expect("write disabled backend overlay");
        assert!(dispatch_receipt_disabled_external_backend_drift(store.root(), &receipt).is_some());

        let second = run_graph_dispatch_init(&store, "task-dispatch-init-disabled-receipt-refresh")
            .await
            .expect("second dispatch-init should rebuild disabled-backend receipt");
        assert_ne!(
            second["dispatch_receipt"]["selected_backend"].as_str(),
            Some("pi_cli")
        );
    }

    #[tokio::test]
    async fn dispatch_init_refreshes_stale_route_assignment_after_model_catalog_change() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-route-drift-refresh",
                title: "Dispatch init refreshes stale route assignment",
                display_id: None,
                description: "Repeated dispatch-init must rebuild stale model route assignments from the current carrier catalog.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_refreshes_stale_route_assignment_after_model_catalog_change -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create route drift refresh task");

        let first = run_graph_dispatch_init(&store, "task-dispatch-init-route-drift-refresh")
            .await
            .expect("first dispatch-init should seed and route");
        assert_eq!(first["run_id"], "task-dispatch-init-route-drift-refresh");

        let mut context = store
            .run_graph_dispatch_context("task-dispatch-init-route-drift-refresh")
            .await
            .expect("context lookup should succeed")
            .expect("dispatch context should exist");
        force_selected_model_ref(
            &mut context.role_selection,
            "stale-model-ref-for-catalog-drift-test",
        );
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("stale dispatch context should persist");
        let stale_selection = context
            .role_selection()
            .expect("stale role selection should still decode");
        assert!(
            dispatch_context_route_assignment_catalog_drift(store.root(), &stale_selection)
                .is_some()
        );

        let second = run_graph_dispatch_init(&store, "task-dispatch-init-route-drift-refresh")
            .await
            .expect("second dispatch-init should refresh stale route assignment");
        assert_eq!(second["run_id"], "task-dispatch-init-route-drift-refresh");

        let refreshed_context = store
            .run_graph_dispatch_context("task-dispatch-init-route-drift-refresh")
            .await
            .expect("refreshed context lookup should succeed")
            .expect("refreshed dispatch context should exist");
        let refreshed_selection = refreshed_context
            .role_selection()
            .expect("refreshed role selection should decode");
        assert!(dispatch_context_route_assignment_catalog_drift(
            store.root(),
            &refreshed_selection
        )
        .is_none());

        let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &refreshed_selection.execution_plan,
            "implementation",
        );
        let payload = crate::taskflow_routing::route_explain_payload(
            &refreshed_selection.execution_plan,
            &refreshed_selection.compiled_bundle,
            "implementation",
            route,
        );
        assert_ne!(
            payload["selected_model_ref"],
            "stale-model-ref-for-catalog-drift-test"
        );
    }

    #[tokio::test]
    async fn dispatch_init_refreshes_stale_configured_dev_team_lane_sequence() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-dev-team-sequence-drift-refresh",
                title: "Dispatch init refreshes stale dev-team sequence",
                display_id: None,
                description: "Repeated dispatch-init must rebuild stale configured dev-team lane sequences before validating allowed_next_node evidence.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_refreshes_stale_configured_dev_team_lane_sequence -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create dev-team route drift refresh task");

        let first =
            run_graph_dispatch_init(&store, "task-dispatch-init-dev-team-sequence-drift-refresh")
                .await
                .expect("first dispatch-init should seed current dev-team sequence");
        assert_eq!(
            first["run_id"],
            "task-dispatch-init-dev-team-sequence-drift-refresh"
        );
        let cache_path = run_graph_dispatch_init_fast_cache_path(
            store.root(),
            "task-dispatch-init-dev-team-sequence-drift-refresh",
        );
        let mut stale_cache_payload: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&cache_path).expect("dispatch-init cache should be readable"),
        )
        .expect("dispatch-init cache should decode");
        let stale_route_sequence = |sequence: &serde_json::Value| {
            let mut sequence = sequence
                .as_array()
                .cloned()
                .expect("configured route sequence should be an array");
            assert!(
                sequence.len() > 1,
                "configured route sequence should contain enough lanes to model drift"
            );
            let original = sequence.clone();
            sequence.rotate_left(1);
            assert_ne!(
                sequence, original,
                "configured route sequence should contain distinct lanes to model drift"
            );
            serde_json::Value::Array(sequence)
        };
        let stale_allowed_next_lane_sequence = stale_route_sequence(
            &stale_cache_payload["dispatch_init_dev_team_route_signature"]
                ["allowed_next_lane_sequence"],
        );
        let stale_execution_lane_sequence = stale_route_sequence(
            &stale_cache_payload["dispatch_init_dev_team_route_signature"]
                ["execution_lane_sequence"],
        );
        stale_cache_payload["dispatch_init_dev_team_route_signature"]
            ["allowed_next_lane_sequence"] = stale_allowed_next_lane_sequence.clone();
        stale_cache_payload["dispatch_init_dev_team_route_signature"]["execution_lane_sequence"] =
            stale_execution_lane_sequence.clone();
        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&stale_cache_payload).expect("cache should encode"),
        )
        .expect("stale signature cache should write");
        assert!(
            read_run_graph_dispatch_init_fast_cache(
                store.root(),
                "task-dispatch-init-dev-team-sequence-drift-refresh",
            )
            .is_some(),
            "structurally valid cache still passes the cheap sync schema gate"
        );
        let mut context = store
            .run_graph_dispatch_context("task-dispatch-init-dev-team-sequence-drift-refresh")
            .await
            .expect("context lookup should succeed")
            .expect("dispatch context should exist");
        let mut stale_selection = context
            .role_selection()
            .expect("role selection should decode before mutation");
        assert!(
            !dispatch_init_fast_cache_payload_matches_current_dev_team_route_signature(
                &stale_cache_payload,
                &stale_selection,
            ),
            "dispatch-init must not return a cache whose route signature is stale"
        );
        stale_selection.execution_plan["development_flow"]["dispatch_contract"]["lane_sequence"] =
            stale_allowed_next_lane_sequence;
        stale_selection.execution_plan["development_flow"]["dispatch_contract"]
            ["execution_lane_sequence"] = stale_execution_lane_sequence;
        context.role_selection =
            serde_json::to_value(&stale_selection).expect("selection should serialize");
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("stale dispatch context should persist");

        assert!(dispatch_context_configured_dev_team_route_drift(
            &store,
            "task-dispatch-init-dev-team-sequence-drift-refresh",
            &stale_selection,
            None,
        )
        .await
        .expect("drift check should run")
        .is_some());

        let second =
            run_graph_dispatch_init(&store, "task-dispatch-init-dev-team-sequence-drift-refresh")
                .await
                .expect("second dispatch-init should refresh stale dev-team sequence");
        assert_eq!(
            second["run_id"],
            "task-dispatch-init-dev-team-sequence-drift-refresh"
        );

        let refreshed_context = store
            .run_graph_dispatch_context("task-dispatch-init-dev-team-sequence-drift-refresh")
            .await
            .expect("refreshed context lookup should succeed")
            .expect("refreshed dispatch context should exist");
        let refreshed_selection = refreshed_context
            .role_selection()
            .expect("refreshed role selection should decode");
        assert!(dispatch_context_configured_dev_team_route_drift(
            &store,
            "task-dispatch-init-dev-team-sequence-drift-refresh",
            &refreshed_selection,
            None,
        )
        .await
        .expect("drift check should run after refresh")
        .is_none());
        let dispatch_contract =
            &refreshed_selection.execution_plan["development_flow"]["dispatch_contract"];
        assert_eq!(
            crate::dispatch_contract_allowed_next_lane_sequence(dispatch_contract),
            crate::dispatch_contract_execution_lane_sequence(dispatch_contract)
        );
        assert!(
            crate::dispatch_contract_allowed_next_lane_sequence(dispatch_contract).len() > 2,
            "refresh should restore the full configured dev-team sequence"
        );
    }

    #[tokio::test]
    async fn dispatch_init_refreshes_stale_route_assignment_after_disabled_external_backend_change()
    {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-dispatch-init-disabled-backend-drift-refresh",
                title: "Dispatch init refreshes disabled backend route drift",
                display_id: None,
                description: "Repeated dispatch-init must rebuild stale route assignments that still reference a disabled external backend.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &["runtime-recovery".to_string()],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "cargo test -p vida dispatch_init_refreshes_stale_route_assignment_after_disabled_external_backend_change -- --nocapture"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create disabled backend route drift refresh task");

        let first =
            run_graph_dispatch_init(&store, "task-dispatch-init-disabled-backend-drift-refresh")
                .await
                .expect("first dispatch-init should seed and route");
        assert_eq!(
            first["run_id"],
            "task-dispatch-init-disabled-backend-drift-refresh"
        );

        let mut context = store
            .run_graph_dispatch_context("task-dispatch-init-disabled-backend-drift-refresh")
            .await
            .expect("context lookup should succeed")
            .expect("dispatch context should exist");
        force_selected_backend_assignment(
            &mut context.role_selection,
            "pi_cli",
            "pi_gpt55_medium_guarded",
        );
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("stale dispatch context should persist");
        let stale_selection = context
            .role_selection()
            .expect("stale role selection should still decode");
        if let Some(drift) =
            dispatch_context_route_assignment_catalog_drift(store.root(), &stale_selection)
                .filter(|drift| drift["drift"]["kind"].is_string())
        {
            assert_eq!(drift["drift"]["kind"], "disabled_external_backend_ref");
            assert_eq!(
                drift["drift"]["route_disabled_external_backend_refs"]["blocking"],
                true
            );
        }

        let second =
            run_graph_dispatch_init(&store, "task-dispatch-init-disabled-backend-drift-refresh")
                .await
                .expect("second dispatch-init should refresh disabled backend route assignment");
        assert_eq!(
            second["run_id"],
            "task-dispatch-init-disabled-backend-drift-refresh"
        );

        let refreshed_context = store
            .run_graph_dispatch_context("task-dispatch-init-disabled-backend-drift-refresh")
            .await
            .expect("refreshed context lookup should succeed")
            .expect("refreshed dispatch context should exist");
        let refreshed_selection = refreshed_context
            .role_selection()
            .expect("refreshed role selection should decode");
        assert!(dispatch_context_route_assignment_catalog_drift(
            store.root(),
            &refreshed_selection
        )
        .is_none());

        let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &refreshed_selection.execution_plan,
            "implementation",
        );
        let payload = crate::taskflow_routing::route_explain_payload(
            &refreshed_selection.execution_plan,
            &refreshed_selection.compiled_bundle,
            "implementation",
            route,
        );
        assert_ne!(payload["selected_backend"].as_str(), Some("pi_cli"));
    }

    #[tokio::test]
    async fn dispatch_init_reseeds_design_backed_explicit_binding_into_worker_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let requested_run_id = "feature-reconcile-autonomous-execution-flag-runtime-drift";
        let bound_task_id =
            "feature-repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen";
        let request_text = "Bounded audit-remediation blocker. After fixing explicit continuation-bind preservation, `vida taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json` now lawfully reseeds the explicit qwen task into a fresh run `feature-reconcile-qwen-cli-carrier-drift-across-config-code`. But that fresh run is shaped as `task_class=pbi_discussion`, `next_node=pm`, `tracked_flow_entry=work-pool-pack`, while the rendered dispatch packet canonicalizes to `dispatch_target=specification`, `handoff_runtime_role=pm`, `activation_agent_type=null`, `selected_backend=null`; `vida agent-init --dispatch-packet ... --execute-dispatch --json` then fails closed with `Dispatch target `specification` is routed to an agent lane but no lawful backend could be resolved from the execution route`.";

        let mut stale_status = default_run_graph_state(requested_run_id, "closure", "delivery");
        stale_status.task_id = requested_run_id.to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.policy_gate = "validation_report_required".to_string();
        stale_status.context_state = "sealed".to_string();
        stale_status.checkpoint_kind = "execution_cursor".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = true;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: requested_run_id.to_string(),
                    task_id: bound_task_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": bound_task_id,
                        "run_id": requested_run_id,
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "reseed explicit qwen remediation blocker onto the bounded implementation task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some(request_text.to_string()),
                    recorded_at: "2026-04-21T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: bound_task_id,
                title: "Implement design-backed reseed canonicalization qwen blocker",
                display_id: None,
                description:
                    "Implement the bounded audit-remediation blocker for design-backed reseed canonicalization.",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "crates/vida/src/taskflow_run_graph.rs".to_string(),
                        "crates/vida/src/taskflow_consume.rs".to_string(),
                        "crates/vida/src/taskflow_consume_resume.rs".to_string(),
                        "crates/vida/src/runtime_dispatch_state.rs".to_string(),
                    ],
                    proof_targets: vec![
                        "cargo test -p vida design_backed -- --nocapture".to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create bound task");

        let design_doc_path = harness
            .path()
            .join("docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Design-backed reseed canonicalization qwen blocker\n\nStatus: `approved`\n\n## Bounded File Set\n- `crates/vida/src/taskflow_run_graph.rs`\n- `crates/vida/src/taskflow_consume.rs`\n- `crates/vida/src/taskflow_consume_resume.rs`\n- `crates/vida/src/runtime_dispatch_state.rs`\n",
        )
        .expect("write approved design doc");
        store
            .acquire_current_session_run_graph_claim_for_test(
                "dispatch-init-reseed-bound-worker-claim",
                bound_task_id,
                bound_task_id,
                "runtime-recovery-contract",
                "crates/vida/src/taskflow_run_graph.rs",
            )
            .await
            .expect("current session should claim reseeded bound run");

        let payload = run_graph_dispatch_init(&store, requested_run_id)
            .await
            .expect("dispatch init should reseed and produce an implementation dispatch");

        assert_eq!(payload["requested_run_id"], requested_run_id);
        assert_eq!(payload["run_id"], bound_task_id);
        assert_eq!(
            payload["run_graph_bootstrap"]["latest_status"]["task_class"].as_str(),
            Some("implementation")
        );
        assert_eq!(
            payload["run_graph_bootstrap"]["latest_status"]["route_task_class"].as_str(),
            Some("implementation")
        );
        let next_node = payload["run_graph_bootstrap"]["latest_status"]["next_node"]
            .as_str()
            .expect("implementation dispatch should have a next node");
        let dispatch_target = payload["dispatch_receipt"]["dispatch_target"]
            .as_str()
            .expect("implementation dispatch should have a target");
        assert_eq!(next_node, dispatch_target);
        assert_ne!(dispatch_target, "specification");
        assert_ne!(dispatch_target, "analyst");
        let activation_agent_type = payload["dispatch_receipt"]["activation_agent_type"]
            .as_str()
            .expect("implementation dispatch should select an agent type");
        assert_eq!(
            payload["dispatch_receipt"]["activation_runtime_role"].as_str(),
            Some("worker")
        );
        assert_eq!(
            payload["dispatch_receipt"]["selected_backend"].as_str(),
            Some("internal_subagents")
        );

        let dispatch_packet_path = payload["dispatch_packet_path"]
            .as_str()
            .expect("dispatch packet path should be present");
        let dispatch_packet =
            crate::read_json_file_if_present(std::path::Path::new(dispatch_packet_path))
                .expect("dispatch packet should load");
        assert_eq!(
            dispatch_packet["dispatch_target"].as_str(),
            Some(dispatch_target)
        );
        assert_eq!(
            dispatch_packet["delivery_task_packet"]["handoff_runtime_role"].as_str(),
            Some("worker")
        );
        assert_eq!(
            dispatch_packet["activation_agent_type"].as_str(),
            Some(activation_agent_type)
        );
        assert_eq!(
            dispatch_packet["selected_backend"].as_str(),
            Some("internal_subagents")
        );
        assert_ne!(
            dispatch_packet["dispatch_target"].as_str(),
            Some("specification")
        );
        assert_eq!(
            dispatch_packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_state.rs",
                "crates/vida/src/taskflow_consume.rs",
                "crates/vida/src/taskflow_consume_resume.rs",
                "crates/vida/src/taskflow_run_graph.rs"
            ])
        );
    }

    #[tokio::test]
    async fn configured_execution_sequence_drives_seed_advance_and_dispatch_init() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let task_id = "task-run-graph-configured-sequence-contract";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Preserve configured execution sequence",
                display_id: None,
                description: "Seed, advance, and dispatch-init must preserve configured ordering.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "seed -> advance -> dispatch-init preserves configured ordering"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured sequence task");

        let request_text = "Repair the configured run-graph seed/advance/dispatch-init contract.";
        let seeded = derive_seeded_run_graph_state(&store, task_id, request_text)
            .await
            .expect("configured sequence seed should derive");
        let dispatch_contract = seeded
            .role_selection
            .execution_plan
            .get("development_flow")
            .and_then(|flow| flow.get("dispatch_contract"))
            .expect("seed should retain the dispatch contract");
        let configured_sequence =
            crate::dispatch_contract_execution_lane_sequence(dispatch_contract);
        assert!(
            configured_sequence.len() >= 2,
            "configured sequence needs two steps"
        );
        let first = configured_sequence[0].clone();
        let second = configured_sequence[1].clone();

        assert_eq!(seeded.status.task_class, "implementation");
        assert_eq!(seeded.status.route_task_class, "implementation");
        assert_eq!(seeded.status.active_node, "planning");
        assert_eq!(seeded.status.next_node.as_deref(), Some(first.as_str()));
        let bootstrap = run_graph_dispatch_bootstrap_from_state(&seeded.status);
        bootstrap.expect("unexpected bootstrap error");

        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let dispatch_init = run_graph_dispatch_init(&store, task_id)
            .await
            .expect("dispatch-init should accept the configured handoff");
        assert_eq!(
            dispatch_init["dispatch_receipt"]["dispatch_target"],
            serde_json::json!(first)
        );

        let advanced = derive_advanced_run_graph_state(
            &store,
            store
                .run_graph_status(task_id)
                .await
                .expect("seeded status lookup should succeed"),
        )
        .await
        .expect("advance should preserve configured ordering");
        assert_eq!(advanced.status.active_node, first);
        assert_eq!(advanced.status.next_node.as_deref(), Some(second.as_str()));
        assert_eq!(
            advanced.status.resume_target,
            format!("dispatch.{second}_lane")
        );
    }

    #[tokio::test]
    async fn configured_execution_sequence_rejects_skipped_seed_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let task_id = "task-run-graph-configured-sequence-skip";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Reject skipped configured execution sequence",
                display_id: None,
                description:
                    "Advance must fail closed when persisted planning state skips a configured predecessor lane.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "advance rejects out-of-order configured execution lanes".to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured sequence task");

        let request_text = "Repair the configured run-graph seed/advance/dispatch-init contract.";
        let seeded = derive_seeded_run_graph_state(&store, task_id, request_text)
            .await
            .expect("configured sequence seed should derive");
        let dispatch_contract = seeded
            .role_selection
            .execution_plan
            .get("development_flow")
            .and_then(|flow| flow.get("dispatch_contract"))
            .expect("seed should retain the dispatch contract");
        let configured_sequence =
            crate::dispatch_contract_execution_lane_sequence(dispatch_contract);
        assert!(
            configured_sequence.len() >= 2,
            "configured sequence needs two steps"
        );
        let first = configured_sequence[0].clone();
        let second = configured_sequence[1].clone();
        assert_eq!(seeded.status.next_node.as_deref(), Some(first.as_str()));

        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let mut corrupted_status = store
            .run_graph_status(task_id)
            .await
            .expect("seeded status lookup should succeed");
        corrupted_status.next_node = Some(second.clone());
        corrupted_status.resume_target = format!("dispatch.{second}_lane");
        store
            .record_run_graph_status(&corrupted_status)
            .await
            .expect("corrupted status should persist for regression proof");

        let error = derive_advanced_run_graph_state(&store, corrupted_status)
            .await
            .expect_err("advance must reject skipped configured predecessor lanes");
        assert!(
            error.contains(&format!(
                "expected configured execution lane `{first}`, got `{second}`"
            )),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn dispatch_init_refreshes_latest_run_graph_surfaces_to_effective_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let labels = Vec::new();
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "task-refresh-latest",
                title: "Refresh latest run graph task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("seed dispatch-init target task");

        let target_status = RunGraphStatus {
            run_id: "task-refresh-latest".to_string(),
            task_id: "task-refresh-latest".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("business_analyst".to_string()),
            status: "ready".to_string(),
            route_task_class: "spec-pack".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "dispatch_ready".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_business_analyst".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.business_analyst_lane".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&target_status)
            .await
            .expect("persist target run status");
        store
            .record_run_graph_dispatch_context(&run_graph_dispatch_context_from_seed_payload(
                &TaskflowRunGraphSeedPayload {
                    status: target_status.clone(),
                    request_text: "Repair fail-closed resume latest projection drift in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/state_store_run_graph_summary.rs.".to_string(),
                    role_selection: RuntimeConsumptionLaneSelection {
                        ok: true,
                        activation_source: "test".to_string(),
                        selection_mode: "fixed".to_string(),
                        fallback_role: "business_analyst".to_string(),
                        request: "Repair fail-closed resume latest projection drift in crates/vida/src/taskflow_run_graph.rs and crates/vida/src/state_store_run_graph_summary.rs.".to_string(),
                        selected_role: "business_analyst".to_string(),
                        conversational_mode: None,
                        single_task_only: true,
                        tracked_flow_entry: Some("spec-pack".to_string()),
                        allow_freeform_chat: false,
                        confidence: "high".to_string(),
                        matched_terms: vec!["repair".to_string(), "resume".to_string()],
                        compiled_bundle: crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
                        execution_plan: serde_json::json!({
                            "runtime_assignment": {
                                "selected_agent_id": "middle",
                                "activation_agent_type": "middle",
                                "activation_runtime_role": "business_analyst"
                            }
                        }),
                        reason: "test".to_string(),
                    },
                },
            ))
            .await
            .expect("persist target dispatch context");

        let mut stale_status = default_run_graph_state("run-stale-latest", "closure", "delivery");
        stale_status.task_id = "run-stale-latest".to_string();
        stale_status.active_node = "closure".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "closure_complete".to_string();
        stale_status.context_state = "sealed".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = false;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale latest run status");
        store
            .acquire_current_session_run_graph_claim_for_test(
                "dispatch-init-refresh-current-run-claim",
                "task-refresh-latest",
                "task-refresh-latest",
                "runtime-recovery-contract",
                "crates/vida/src/taskflow_run_graph.rs",
            )
            .await
            .expect("current session should claim dispatch-init target run");

        let payload = run_graph_dispatch_init(&store, "task-refresh-latest")
            .await
            .expect("dispatch init should succeed");
        assert_eq!(payload["run_id"], "task-refresh-latest");

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.run_id, "task-refresh-latest");

        let latest_recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("load latest recovery summary")
            .expect("latest recovery summary should exist");
        assert_eq!(latest_recovery.run_id, "task-refresh-latest");

        let latest_receipt = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary")
            .expect("latest dispatch receipt summary should exist");
        assert_eq!(latest_receipt.run_id, "task-refresh-latest");
    }

    #[tokio::test]
    async fn configured_lane_advance_rejects_blocked_seeded_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let task_id = "task-configured-blocked-coder";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Reject blocked configured lane advance",
                display_id: None,
                description:
                    "Configured lane advancement must fail closed for blocked seeded lanes.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec!["blocked configured lane advance fails closed".to_string()],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured blocked-lane task");
        let seeded = derive_seeded_run_graph_state(
            &store,
            task_id,
            "Repair configured run-graph lane advance guards.",
        )
        .await
        .expect("seeded configured lane state should derive");
        let first_node =
            crate::runtime_dispatch_state::typed_lane_node_sequence(&seeded.role_selection, true)
                .expect("configured TeamFlow node sequence should resolve")
                .first()
                .cloned()
                .expect("configured lane sequence should have a first lane");
        let first_lane = first_node.node_id.clone();
        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let mut existing = seeded.status;
        existing.active_node = first_lane.clone();
        existing.next_node = Some(first_lane.clone());
        existing.status = "blocked".to_string();
        existing.lane_id = first_node.lane_id;
        existing.lifecycle_stage = format!("{first_lane}_dispatch_ready");
        existing.policy_gate = "targeted_verification".to_string();
        existing.handoff_state = format!("awaiting_{first_lane}");
        existing.resume_target = format!("dispatch.{first_lane}_lane");
        existing.recovery_ready = true;
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let error = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect_err("blocked configured lane must not advance");

        assert!(
            error.contains("configured execution lane `coder` is not advanceable"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn configured_dispatch_ready_lane_requires_cleared_policy_gate() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let task_id = "task-configured-policy-gated-coder";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Reject policy-gated configured lane advance",
                display_id: None,
                description: "Configured dispatch-ready lanes must reject uncleared policy gates.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "policy-gated configured dispatch-ready lane fails closed".to_string()
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured policy-gated task");
        let seeded = derive_seeded_run_graph_state(
            &store,
            task_id,
            "Repair configured run-graph lane policy guards.",
        )
        .await
        .expect("seeded configured policy state should derive");
        let first_node =
            crate::runtime_dispatch_state::typed_lane_node_sequence(&seeded.role_selection, true)
                .expect("configured TeamFlow node sequence should resolve")
                .first()
                .cloned()
                .expect("configured lane sequence should have a first lane");
        let first_lane = first_node.node_id.clone();
        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let mut existing = seeded.status;
        existing.active_node = first_lane.clone();
        existing.next_node = Some(first_lane.clone());
        existing.status = "ready".to_string();
        existing.lane_id = first_node.lane_id;
        existing.lifecycle_stage = format!("{first_lane}_dispatch_ready");
        existing.policy_gate = "targeted_verification".to_string();
        existing.handoff_state = format!("awaiting_{first_lane}");
        existing.resume_target = format!("dispatch.{first_lane}_lane");
        existing.recovery_ready = true;
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let error = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect_err("policy-gated configured dispatch-ready lane must not advance");

        assert!(
            error.contains("still requires policy_gate=`targeted_verification`"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn configured_final_lane_requires_completed_evidence_before_completion() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let task_id = "task-configured-final-ready";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Reject incomplete configured final lane",
                display_id: None,
                description: "The final configured lane must require completed evidence.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "final configured lane requires completed evidence".to_string()
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured final-lane task");
        let seeded = derive_seeded_run_graph_state(
            &store,
            task_id,
            "Repair configured run-graph final-lane completion guards.",
        )
        .await
        .expect("seeded configured final-lane state should derive");
        let final_node =
            crate::runtime_dispatch_state::typed_lane_node_sequence(&seeded.role_selection, true)
                .expect("configured TeamFlow node sequence should resolve")
                .last()
                .cloned()
                .expect("configured lane sequence should have a final lane");
        let final_lane = final_node.node_id.clone();
        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let mut existing = seeded.status;
        existing.active_node = final_lane.clone();
        existing.next_node = None;
        existing.status = "ready".to_string();
        existing.lane_id = final_node.lane_id;
        existing.lifecycle_stage = format!("{final_lane}_active");
        existing.policy_gate = "not_required".to_string();
        existing.handoff_state = "none".to_string();
        existing.resume_target = "none".to_string();
        existing.recovery_ready = false;
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let error = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect_err("final configured lane must not complete without completed evidence");

        assert!(
            error.contains("requires completed lane evidence"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn seeded_worker_run_can_advance_directly_into_configured_writer_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let task_id = "task-direct-configured-writer";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Advance seeded run into configured writer lane",
                display_id: None,
                description:
                    "Seeded implementation runs must preserve configured writer node identity.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "seeded worker advances into configured writer lane".to_string()
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured writer task");
        let seeded = derive_seeded_run_graph_state(
            &store,
            task_id,
            "Advance seeded implementation directly into the configured writer lane.",
        )
        .await
        .expect("seeded configured-writer state should derive");
        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let compiled_control = compiled_run_graph_control(&store)
            .await
            .expect("compiled run-graph control should be available");
        let writer_node = compiled_control.entry_execution_node_id.clone();
        let configured_sequence =
            crate::runtime_dispatch_state::typed_lane_node_sequence(&seeded.role_selection, true)
                .expect("configured TeamFlow node sequence should resolve");
        let writer_index = configured_sequence
            .iter()
            .position(|node| node.node_id == writer_node)
            .expect("configured writer node should be in the execution sequence");
        let expected_next_node = configured_sequence
            .get(writer_index + 1)
            .map(|node| node.node_id.clone());
        let existing = RunGraphStatus {
            run_id: task_id.to_string(),
            task_id: task_id.to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some(writer_node.clone()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: format!("awaiting_{writer_node}"),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: format!("dispatch.{writer_node}_lane"),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let payload = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect("seeded writer run should advance");

        assert_eq!(payload.status.active_node, writer_node);
        assert_eq!(
            payload.status.lifecycle_stage,
            format!("{writer_node}_active")
        );
        assert_eq!(payload.status.next_node, expected_next_node);
        assert_eq!(
            payload.status.handoff_state,
            expected_next_node
                .as_deref()
                .map(|node| format!("awaiting_{node}"))
                .unwrap_or_else(|| "none".to_string())
        );
    }

    #[tokio::test]
    async fn configured_runtime_defect_seed_advance_and_dispatch_init_share_lane_contract() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        let task_id = "task-run-graph-configured-coder-contract";
        let labels = vec!["runtime-recovery".to_string()];
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Repair configured run-graph coder contract",
                display_id: None,
                description:
                    "Seed, advance, and dispatch-init must preserve the configured coder lane.",
                issue_type: "runtime_defect",
                status: "in_progress",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/taskflow_run_graph.rs".to_string()],
                    proof_targets: vec![
                        "seed -> advance -> dispatch-init preserves the configured coder next-node contract"
                            .to_string(),
                    ],
                    ..crate::state_store::TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create configured runtime-defect task");

        let request_text =
            "Repair the configured run-graph seed/advance/dispatch-init next-node contract.";
        let seeded = derive_seeded_run_graph_state(&store, task_id, request_text)
            .await
            .expect("configured runtime-defect seed should derive");
        let configured_sequence =
            crate::runtime_dispatch_state::typed_lane_node_sequence(&seeded.role_selection, true)
                .expect("seed-selected TeamFlow node authority must compile");
        let first_node = configured_sequence
            .first()
            .cloned()
            .expect("configured node sequence must have a first node");
        let second_node = configured_sequence
            .get(1)
            .cloned()
            .expect("configured node sequence must have a second node");
        let first = first_node.node_id.clone();
        let second = second_node.node_id.clone();
        assert_eq!(seeded.status.task_class, "implementation");
        assert_eq!(seeded.status.route_task_class, "implementation");
        assert_eq!(seeded.status.active_node, "planning");
        assert_eq!(seeded.status.next_node.as_deref(), Some(first.as_str()));
        assert_eq!(
            seeded.status.lifecycle_stage,
            format!("{first}_dispatch_ready")
        );
        run_graph_dispatch_bootstrap_from_state(&seeded.status)
            .expect("unexpected bootstrap error");

        persist_seed_artifacts(&store, &seeded)
            .await
            .expect("seed artifacts should persist");
        let dispatch_init = run_graph_dispatch_init(&store, task_id)
            .await
            .expect("dispatch-init should accept the seeded configured handoff");
        assert_eq!(
            dispatch_init["dispatch_receipt"]["dispatch_target"],
            serde_json::json!(first_node.dispatch_target)
        );
        assert!(store
            .run_graph_dispatch_receipt(task_id)
            .await
            .expect("dispatch receipt lookup should succeed")
            .is_some());
        let persisted_after_dispatch = store
            .run_graph_status(task_id)
            .await
            .expect("dispatch-init status lookup should succeed");
        assert_eq!(persisted_after_dispatch.active_node, "planning");
        assert_eq!(
            persisted_after_dispatch.next_node.as_deref(),
            Some(first.as_str())
        );
        let advanced = derive_advanced_run_graph_state(
            &store,
            store
                .run_graph_status(task_id)
                .await
                .expect("seeded status lookup should succeed"),
        )
        .await
        .expect("advance should accept the configured handoff");
        assert_eq!(advanced.status.active_node, first);
        assert_eq!(advanced.status.next_node.as_deref(), Some(second.as_str()));
        assert_eq!(advanced.status.lifecycle_stage, format!("{first}_active"));
        assert_eq!(
            advanced.status.resume_target,
            format!("dispatch.{second}_lane")
        );
    }

    #[tokio::test]
    async fn run_graph_advance_reports_active_exception_takeover_before_route_support_error() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let mut existing = default_run_graph_state(
            "run-active-exception-advance",
            "specification",
            "specification",
        );
        existing.active_node = "analyst".to_string();
        existing.status = "blocked".to_string();
        existing.lifecycle_stage = "analyst_blocked".to_string();
        existing.lane_id = "analyst_lane".to_string();
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-active-exception-advance");
        receipt.dispatch_target = "analyst".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_packet_path = None;
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("record active exception takeover receipt");

        let error = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect_err("active exception takeover should block route advance");

        assert!(error.contains("active exception takeover"));
        assert!(error.contains("vida lane takeover-ready run-active-exception-advance"));
        assert!(
            !error.contains("currently supports only seeded implementation"),
            "advance should not mask active exception takeover behind route support errors"
        );
    }

    #[tokio::test]
    async fn seeded_worker_coach_lane_can_advance_to_review_ensemble() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let existing = RunGraphStatus {
            run_id: "task-test-author-to-coach".to_string(),
            task_id: "task-test-author-to-coach".to_string(),
            task_class: "implementation".to_string(),
            active_node: "coach".to_string(),
            next_node: Some("review_ensemble".to_string()),
            status: "clean".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "senior".to_string(),
            lane_id: "coach_lane".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            policy_gate: "review_findings".to_string(),
            handoff_state: "awaiting_review_ensemble".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.review_ensemble".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let payload = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect("coach lane should advance to review ensemble");

        assert_eq!(payload.status.active_node, "review_ensemble");
        assert_eq!(payload.status.lifecycle_stage, "review_ensemble_active");
        assert_eq!(payload.status.next_node, None);
        assert_eq!(payload.status.handoff_state, "none");
    }

    #[tokio::test]
    async fn blocked_coach_lane_cannot_advance_to_verification() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let mut existing = default_run_graph_state(
            "task-blocked-coach-no-verification",
            "implementation",
            "implementation",
        );
        existing.active_node = "coach".to_string();
        existing.next_node = Some("review_ensemble".to_string());
        existing.status = "blocked".to_string();
        existing.lane_id = "coach_lane".to_string();
        existing.lifecycle_stage = "coach_blocked".to_string();
        existing.handoff_state = "awaiting_verification".to_string();
        existing.resume_target = "dispatch.review_ensemble".to_string();
        existing.recovery_ready = true;
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let error = derive_advanced_run_graph_state(&store, existing)
            .await
            .expect_err("blocked coach should not advance to verification");

        assert!(
            error.contains("developer_rework before verification"),
            "unexpected error: {error}"
        );
        assert!(
            !error.contains("verification_active"),
            "blocked coach must not enter verification: {error}"
        );
    }

    #[test]
    fn implementation_verification_outcome_defaults_for_unexpected_status() {
        assert_eq!(
            implementation_verification_outcome("paused"),
            ImplementationVerificationOutcome::UnexpectedStatus
        );
    }

    #[test]
    fn approval_delegation_transition_kind_requires_route_bound_receipt_shape() {
        let mut awaiting_approval =
            default_run_graph_state("run-1", "implementation", "implementation");
        awaiting_approval.status = "awaiting_approval".to_string();
        awaiting_approval.active_node = "verification".to_string();
        awaiting_approval.next_node = Some("approval".to_string());
        awaiting_approval.lifecycle_stage = "approval_wait".to_string();
        awaiting_approval.policy_gate = crate::release1_contracts::ApprovalStatus::ApprovalRequired
            .as_str()
            .to_string();
        awaiting_approval.handoff_state = "awaiting_approval".to_string();
        awaiting_approval.resume_target = "dispatch.approval".to_string();

        assert_eq!(
            approval_delegation_transition_kind(&awaiting_approval),
            Some("approval_wait")
        );

        let mut completed = default_run_graph_state("run-1", "implementation", "implementation");
        completed.active_node = "verification".to_string();
        completed.status = "completed".to_string();
        completed.next_node = None;
        completed.lifecycle_stage = "implementation_complete".to_string();
        completed.policy_gate = "not_required".to_string();
        completed.handoff_state = "none".to_string();
        completed.resume_target = "none".to_string();

        assert_eq!(
            approval_delegation_transition_kind(&completed),
            Some("approval_complete")
        );

        let mut unstructured = completed;
        unstructured.status = "approved".to_string();
        assert_eq!(approval_delegation_transition_kind(&unstructured), None);
    }

    #[test]
    fn merge_run_graph_meta_allows_explicit_null_to_clear_handoff_fields() {
        let merged = merge_run_graph_meta(
            RunGraphStatus {
                run_id: "run-1".to_string(),
                task_id: "run-1".to_string(),
                task_class: "implementation".to_string(),
                active_node: "writer".to_string(),
                next_node: Some("coach".to_string()),
                status: "ready".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "middle".to_string(),
                lane_id: "writer_lane".to_string(),
                lifecycle_stage: "writer_active".to_string(),
                policy_gate: "targeted_verification".to_string(),
                handoff_state: "awaiting_coach".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.writer_lane".to_string(),
                recovery_ready: true,
            },
            &serde_json::json!({
                "next_node": null,
                "handoff_state": null,
                "resume_target": null,
                "recovery_ready": false
            }),
        );

        assert_eq!(merged.next_node, None);
        assert_eq!(merged.handoff_state, "none");
        assert_eq!(merged.resume_target, "none");
        assert!(!merged.recovery_ready);
    }

    #[tokio::test]
    async fn run_graph_terminal_update_sync_clears_stale_continuation_binding() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("state store should open");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-terminal-update".to_string(),
                    task_id: "run-terminal-update".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": "run-terminal-update",
                        "run_id": "run-terminal-update",
                        "active_node": "analysis"
                    }),
                    binding_source: "consume_after_downstream_chain".to_string(),
                    why_this_unit: "stale generated proof run binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only".to_string(),
                    request_text: Some("proof run".to_string()),
                    recorded_at: "2026-04-23T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale continuation binding");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-terminal-update".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("sup-terminal-update".to_string()),
                exception_path_receipt_id: Some("exc-terminal-update".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: Some("stale terminal blocker".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                policy_bundle_ref: None,
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale blocked receipt with explicit takeover");

        let mut status =
            default_run_graph_state("run-terminal-update", "implementation", "implementation");
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        record_run_graph_state_with_continuation_sync(&store, &status, "run_graph_update")
            .await
            .expect("terminal update should sync continuation binding");

        let reconciled = store
            .run_graph_status("run-terminal-update")
            .await
            .expect("reconciled status should load");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert!(store
            .run_graph_continuation_binding("run-terminal-update")
            .await
            .expect("continuation binding lookup should succeed")
            .is_none());
    }

    #[test]
    fn merge_run_graph_meta_canonicalizes_resume_target_drifts() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };

        let merged = merge_run_graph_meta(
            status,
            &serde_json::json!({
                "resume_target": "dispatch.coach",
                "next_node": "writer",
                "handoff_state": "awaiting_writer"
            }),
        );

        assert_eq!(merged.resume_target, "dispatch.coach");
        assert_eq!(merged.next_node.as_deref(), Some("coach"));
        assert_eq!(merged.handoff_state, "awaiting_coach");
    }

    #[test]
    fn merge_run_graph_meta_resets_resume_fields_when_target_none() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let merged = merge_run_graph_meta(status, &serde_json::json!({ "resume_target": null }));

        assert_eq!(merged.resume_target, "none");
        assert_eq!(merged.next_node, None);
        assert_eq!(merged.handoff_state, "none");
    }

    #[test]
    fn validate_run_graph_resume_gate_requires_dispatch_resume_target() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: None,
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("resume_target"));
    }

    #[test]
    fn validate_run_graph_resume_gate_accepts_open_delegation_cycle() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        validate_run_graph_resume_gate(&status).expect("should pass");
    }

    #[test]
    fn validate_run_graph_resume_gate_accepts_seeded_configured_route() {
        let status = RunGraphStatus {
            run_id: "run-configured-route".to_string(),
            task_id: "task-configured-route".to_string(),
            task_class: "verification".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("tester".to_string()),
            status: "ready".to_string(),
            route_task_class: "verification".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "tester_lane".to_string(),
            lifecycle_stage: "tester_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_tester".to_string(),
            context_state: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.tester".to_string(),
            recovery_ready: true,
        };

        validate_run_graph_resume_gate(&status).expect("seeded configured route should pass");
    }

    #[test]
    fn validate_run_graph_resume_gate_rejects_incomplete_dispatch_handoff_metadata() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: String::new(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("policy_gate"));
        assert!(error.contains("handoff metadata"));
    }

    #[test]
    fn validate_run_graph_resume_gate_rejects_resume_target_handoff_mismatch() {
        let status = RunGraphStatus {
            run_id: "run-1".to_string(),
            task_id: "run-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "writer_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            recovery_ready: true,
        };

        let error = validate_run_graph_resume_gate(&status).expect_err("should fail");
        assert!(error.contains("resume_target"));
        assert!(error.contains("handoff_state"));
    }

    #[test]
    fn projection_reason_prefers_persisted_dispatch_blocker_evidence() {
        let status = RunGraphStatus {
            run_id: "run-projection-1".to_string(),
            task_id: "task-projection-1".to_string(),
            task_class: "scope_discussion".to_string(),
            active_node: "specification".to_string(),
            next_node: None,
            status: "blocked".to_string(),
            route_task_class: "spec-pack".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "specification_lane".to_string(),
            lifecycle_stage: "specification_active".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/projection-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            projection_reason_for_snapshot(&status, Some(&receipt), None),
            "run-graph status reflects persisted dispatch blocker evidence"
        );
        assert_eq!(
            projection_vs_receipt_parity(&status, Some(&receipt)),
            "aligned"
        );
    }

    #[test]
    fn projection_stale_state_suspected_for_old_executing_dispatch_result() {
        let root = std::env::temp_dir().join(format!(
            "vida-projection-stale-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let result_path = root.join("dispatch-result.json");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "execution_state": "executing",
                "recorded_at": "2026-04-18T00:00:00Z"
            }))
            .expect("dispatch result should encode"),
        )
        .expect("dispatch result should write");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-projection-stale".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(projection_stale_state_suspected(&root, Some(&receipt)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn projection_stale_state_suspected_respects_artifact_stale_after_seconds() {
        let root = std::env::temp_dir().join(format!(
            "vida-projection-stale-window-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let result_path = root.join("dispatch-result.json");
        let recorded_at = (time::OffsetDateTime::now_utc() - time::Duration::seconds(15))
            .format(&Rfc3339)
            .expect("timestamp should render");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "execution_state": "executing",
                "recorded_at": recorded_at,
                "stale_after_seconds": 39
            }))
            .expect("dispatch result should encode"),
        )
        .expect("dispatch result should write");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-projection-stale-window".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(!projection_stale_state_suspected(&root, Some(&receipt)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn projection_stale_state_suspected_for_blocked_external_internal_activation_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "vida-projection-blocked-mismatch-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let result_path = root.join("dispatch-result.json");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "selected_backend": "hermes_cli",
                "lane_execution_receipt_artifact": {
                    "carrier_id": "hermes_cli"
                },
                "recorded_at": "2026-04-21T12:39:12Z"
            }))
            .expect("dispatch result should encode"),
        )
        .expect("dispatch result should write");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-projection-blocked-mismatch".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/projection-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            policy_bundle_ref: None,
            recorded_at: "2026-04-21T12:14:39Z".to_string(),
        };

        assert!(projection_stale_state_suspected(&root, Some(&receipt)));
        assert_eq!(
            next_lawful_operator_action_for_projection(
                &RunGraphStatus {
                    run_id: "run-projection-blocked-mismatch".to_string(),
                    task_id: "task-projection-blocked-mismatch".to_string(),
                    task_class: "implementation".to_string(),
                    active_node: "coach".to_string(),
                    next_node: None,
                    status: "blocked".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "hermes_cli".to_string(),
                    lane_id: "coach_lane".to_string(),
                    lifecycle_stage: "coach_blocked".to_string(),
                    policy_gate: "validation_report_required".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "execution_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: false,
                },
                Some(&receipt),
                None,
                false,
                false,
            )
            .as_deref(),
            Some("vida lane show run-projection-blocked-mismatch --json")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn next_lawful_operator_action_prefers_continue_for_recovery_ready_status() {
        let status = RunGraphStatus {
            run_id: "run-projection-continue".to_string(),
            task_id: "task-projection-continue".to_string(),
            task_class: "implementation".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("verification".to_string()),
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_verification".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "artifact".to_string(),
            resume_target: "dispatch.verification_lane".to_string(),
            recovery_ready: true,
        };

        let next_action = next_lawful_operator_action_for_snapshot(&status)
            .expect("recovery-ready status should recommend consume continue");
        assert_eq!(
            next_action,
            "vida taskflow consume continue --run-id run-projection-continue --json"
        );
        assert!(next_action.contains("--json"));
    }

    #[test]
    fn next_lawful_operator_action_uses_downstream_execute_command_after_terminal_ready_downstream_handoff(
    ) {
        let status = RunGraphStatus {
            run_id: "run-terminal-ready-handoff".to_string(),
            task_id: "task-terminal-ready-handoff".to_string(),
            task_class: "implementation".to_string(),
            active_node: "analysis".to_string(),
            next_node: Some("writer".to_string()),
            status: "blocked".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "middle".to_string(),
            lane_id: "analysis_lane".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            recovery_ready: true,
        };
        let receipt = clean_ready_downstream_dispatch_receipt("run-terminal-ready-handoff");

        let next_action = next_lawful_operator_action_for_projection(
            &status,
            Some(&receipt),
            Some("run-terminal-ready-handoff"),
            false,
            false,
        )
        .expect("terminal ready downstream handoff should expose downstream command");
        assert_eq!(
            next_action,
            "vida agent-init --downstream-packet downstream-packet.json --execute-dispatch --json"
        );
        assert!(next_action.contains("--json"));
    }

    #[test]
    fn next_lawful_operator_action_uses_dispatch_packet_for_routed_agent_handoff() {
        let status = RunGraphStatus {
            run_id: "run-routed-agent-handoff".to_string(),
            task_id: "task-routed-agent-handoff".to_string(),
            task_class: "implementation".to_string(),
            active_node: "coach_implementation_gate".to_string(),
            next_node: Some("coach_implementation_gate".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "vibe_cli".to_string(),
            lane_id: "coach_implementation_gate_lane".to_string(),
            lifecycle_stage: "coach_implementation_gate_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_coach_implementation_gate".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach_implementation_gate_lane".to_string(),
            recovery_ready: true,
        };
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-routed-agent-handoff");
        receipt.dispatch_target = "coach_implementation_gate".to_string();
        receipt.dispatch_status = "routed".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.dispatch_packet_path = Some("coach-packet.json".to_string());
        receipt.dispatch_result_path = None;
        receipt.downstream_dispatch_target = Some("coach_implementation_gate".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_result_path = None;

        let next_action =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false, false)
                .expect("routed agent handoff should expose dispatch packet command");
        assert_eq!(
            next_action,
            "vida agent-init --dispatch-packet coach-packet.json --execute-dispatch --json"
        );
        assert!(!next_action.contains("exception-takeover"));
    }

    #[test]
    fn recovery_surface_contract_pass_retains_projection_next_action() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-recovery-pass-action".to_string(),
            task_id: "run-recovery-pass-action".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            resume_node: Some("writer".to_string()),
            resume_status: "ready".to_string(),
            recovery_ready: true,
            handoff_state: "awaiting_writer".to_string(),
            policy_gate: "targeted_verification".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                lifecycle_stage: "analysis_active".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida agent-init --downstream-packet packet.json --execute-dispatch --json"
                    .to_string(),
            ),
            dispatch_receipt: Some(clean_ready_downstream_dispatch_receipt(
                "run-recovery-pass-action",
            )),
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert!(blocker_codes.is_empty());
        assert!(why_not_now.is_none());
        assert_eq!(
            next_action.as_ref().map(|value| value.command.as_str()),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json")
        );
        assert_eq!(recommended_surface.as_deref(), Some("vida agent-init"));
    }

    #[test]
    fn recovery_surface_contract_does_not_recommend_takeover_for_routed_agent_handoff() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-routed-recovery-handoff".to_string(),
            task_id: "task-routed-recovery-handoff".to_string(),
            active_node: "coach_implementation_gate".to_string(),
            lifecycle_stage: "coach_implementation_gate_dispatch_ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach_implementation_gate_lane".to_string(),
            resume_node: Some("coach_implementation_gate".to_string()),
            resume_status: "ready".to_string(),
            recovery_ready: true,
            handoff_state: "awaiting_coach_implementation_gate".to_string(),
            policy_gate: "not_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach_implementation_gate".to_string(),
                lifecycle_stage: "coach_implementation_gate_dispatch_ready".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-routed-recovery-handoff");
        receipt.dispatch_target = "coach_implementation_gate".to_string();
        receipt.dispatch_status = "routed".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.dispatch_packet_path = Some("coach-packet.json".to_string());
        receipt.dispatch_result_path = None;
        receipt.downstream_dispatch_target = Some("coach_implementation_gate".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_result_path = None;
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida agent-init --dispatch-packet coach-packet.json --execute-dispatch --json"
                    .to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract_with_owned_scope(
                &summary,
                &projection_truth,
                &["crates/vida/src/taskflow_run_graph.rs".to_string()],
            );

        assert!(blocker_codes.is_empty());
        assert!(why_not_now.is_none());
        assert_eq!(
            next_action.as_ref().map(|action| action.command.as_str()),
            Some("vida agent-init --dispatch-packet coach-packet.json --execute-dispatch --json")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida agent-init --dispatch-packet coach-packet.json --execute-dispatch --json")
        );
        assert_eq!(recommended_surface.as_deref(), Some("vida agent-init"));
        assert!(recommended_command
            .as_deref()
            .is_some_and(|command| !command.contains("exception-takeover")));
    }

    #[test]
    fn recovery_surface_contract_keeps_materialization_only_receipt_blocked_after_terminal_completion(
    ) {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-terminal-materialization".to_string(),
            task_id: "run-terminal-materialization".to_string(),
            active_node: "work-pool-pack".to_string(),
            lifecycle_stage: "work_pool_pack_complete".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            resume_status: "completed".to_string(),
            recovery_ready: false,
            handoff_state: "none".to_string(),
            policy_gate: "not_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "work-pool-pack".to_string(),
                lifecycle_stage: "work_pool_pack_complete".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                blocker_code: None,
                reporting_pause_gate: "closure_candidate".to_string(),
                continuation_signal: "continue_after_reports".to_string(),
            },
        };
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-terminal-materialization");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["internal_activation_view_only".to_string()];
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-terminal-materialization --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert!(blocker_codes.contains(&"internal_activation_view_only".to_string()));
        assert!(why_not_now.is_some());
        assert!(next_action.is_none());
        assert!(recommended_command.is_none());
        assert!(recommended_surface.is_none());
    }

    #[test]
    fn run_graph_status_surface_keeps_materialization_only_receipt_blocked_after_terminal_completion(
    ) {
        let mut status = default_run_graph_state(
            "run-terminal-materialization-status",
            "implementation",
            "implementation",
        );
        status.active_node = "work-pool-pack".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "work_pool_pack_complete".to_string();
        status.recovery_ready = false;
        status.resume_target = "none".to_string();
        status.checkpoint_kind = "none".to_string();
        status.handoff_state = "none".to_string();
        status.policy_gate = "not_required".to_string();
        status.context_state = "sealed".to_string();

        let mut receipt =
            clean_ready_downstream_dispatch_receipt("run-terminal-materialization-status");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["internal_activation_view_only".to_string()];
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-terminal-materialization-status --json"
                    .to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let blocker_codes = run_graph_state_surface_issue_codes(&status, &projection_truth);

        assert!(blocker_codes.contains(&"internal_activation_view_only".to_string()));
    }

    #[test]
    fn recovery_surface_contract_open_cycle_emits_exception_takeover_when_owned_scope_known() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-open-cycle".to_string(),
            task_id: "task-open-cycle".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            recovery_ready: false,
            handoff_state: "blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                lifecycle_stage: "coach_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "blocking".to_string(),
                continuation_signal: "record_exception_takeover".to_string(),
            },
        };
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-open-cycle");
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_blocked".to_string();
        receipt.blocker_code = Some("timeout_without_takeover_authority".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_blockers =
            vec!["timeout_without_takeover_authority".to_string()];
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects persisted dispatch blocker evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some("vida lane show run-open-cycle --json".to_string()),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (_codes, _why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract_with_owned_scope(
                &summary,
                &projection_truth,
                &[
                    "crates/vida/src/agent_dispatch_surface.rs".to_string(),
                    "vida.config.yaml".to_string(),
                ],
            );
        let command = recommended_command.expect("recovery should recommend takeover command");

        assert_eq!(
            next_action.as_ref().map(|action| action.command.as_str()),
            Some(command.as_str())
        );
        assert!(command.starts_with("vida lane exception-takeover run-open-cycle"));
        assert!(command.contains("--reason-class timeout_without_takeover_authority"));
        assert!(command.contains("--active-bounded-unit task-open-cycle:coach:exception-takeover"));
        assert!(command.contains("--owned-write-scope crates/vida/src/agent_dispatch_surface.rs"));
        assert!(command.contains("--owned-write-scope vida.config.yaml"));
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida lane exception-takeover")
        );
    }

    #[test]
    fn recovery_surface_contract_open_cycle_with_owned_scope_defers_to_stale_projection_action() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-open-cycle-stale".to_string(),
            task_id: "task-open-cycle-stale".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.coach".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            recovery_ready: false,
            handoff_state: "blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                lifecycle_stage: "coach_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "blocking".to_string(),
                continuation_signal: "record_exception_takeover".to_string(),
            },
        };
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-open-cycle-stale");
        receipt.dispatch_status = "executing".to_string();
        receipt.lane_status = "lane_active".to_string();
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_blockers = vec!["stale_executing_dispatch".to_string()];
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects stale persisted dispatch evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: true,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-open-cycle-stale --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (_codes, _why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract_with_owned_scope(
                &summary,
                &projection_truth,
                &["crates/vida/src/agent_dispatch_surface.rs".to_string()],
            );

        assert_eq!(
            next_action.as_ref().map(|action| action.command.as_str()),
            Some("vida taskflow run-graph status run-open-cycle-stale --json")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida taskflow run-graph status run-open-cycle-stale --json")
        );
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida taskflow run-graph status")
        );
        assert!(recommended_command
            .as_deref()
            .is_some_and(|command| !command.contains("exception-takeover")));
    }

    #[test]
    fn recovery_surface_contract_suppresses_stale_open_cycle_after_active_exception_takeover() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-active-exception-stale-recovery".to_string(),
            task_id: "task-active-exception-stale-recovery".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            recovery_ready: false,
            handoff_state: "blocked".to_string(),
            policy_gate: "validation_report_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                lifecycle_stage: "coach_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "blocking".to_string(),
                continuation_signal: "record_exception_takeover".to_string(),
            },
        };
        let mut receipt =
            clean_ready_downstream_dispatch_receipt("run-active-exception-stale-recovery");
        receipt.dispatch_target = "coach".to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_blockers = vec!["tool_execution_failed".to_string()];
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects stale persisted recovery evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: true,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-active-exception-stale-recovery --json"
                    .to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, next_action, recommended_command, recommended_surface) =
            recovery_surface_contract_with_owned_scope(
                &summary,
                &projection_truth,
                &["crates/vida/src/taskflow_run_graph.rs".to_string()],
            );

        assert!(blocker_codes.is_empty());
        assert!(why_not_now.is_none());
        assert_eq!(
            next_action.as_ref().map(|action| action.command.as_str()),
            Some("vida taskflow run-graph status run-active-exception-stale-recovery --json")
        );
        assert_eq!(
            recommended_command.as_deref(),
            Some("vida taskflow run-graph status run-active-exception-stale-recovery --json")
        );
        assert_eq!(
            recommended_surface.as_deref(),
            Some("vida taskflow run-graph status")
        );

        let payload = build_recovery_explain_json_payload(
            "vida taskflow recovery explain",
            &summary,
            &projection_truth,
            blocker_codes,
            why_not_now,
            next_action,
            recommended_command,
            recommended_surface,
        )
        .expect("recovery explain payload should render");

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["diagnosis"], "runtime_defect");
        assert_eq!(
            payload["diagnosis_detail"]["blocker_codes"],
            serde_json::json!([])
        );
    }

    #[test]
    fn run_graph_status_surface_suppresses_open_cycle_after_active_exception_takeover() {
        let mut status = default_run_graph_state(
            "run-active-exception-status",
            "specification",
            "specification",
        );
        status.active_node = "analyst".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analyst_blocked".to_string();
        status.lane_id = "analyst_lane".to_string();
        status.recovery_ready = false;
        status.resume_target = "dispatch.analyst".to_string();
        let mut receipt = clean_ready_downstream_dispatch_receipt("run-active-exception-status");
        receipt.dispatch_target = "analyst".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_packet_path = None;
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: false,
            next_lawful_operator_action: Some(
                "vida lane show run-active-exception-status --json".to_string(),
            ),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let blocker_codes = run_graph_state_surface_issue_codes(&status, &projection_truth);

        assert!(
            blocker_codes.is_empty(),
            "active exception takeover receipt should supersede stale open delegated cycle blockers"
        );
    }

    #[test]
    fn recovery_surface_contract_mentions_stale_state_when_projection_flags_it() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-stale-summary".to_string(),
            task_id: "run-stale-summary".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            recovery_ready: false,
            handoff_state: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                lifecycle_stage: "analysis_active".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason:
                "run-graph status was reconciled against persisted dispatch receipt evidence"
                    .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: true,
            projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
            stale_state_suspected: true,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-stale-summary --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let (_codes, why_not_now, next_action, _command, _surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert!(why_not_now
            .as_ref()
            .map(|value| value.summary.contains("looks stale"))
            .unwrap_or(false));
        assert!(next_action
            .as_ref()
            .map(|value| value
                .reason
                .contains("stale delegated execution is suspected"))
            .unwrap_or(false));
    }

    #[test]
    fn recovery_surface_contract_distinguishes_stale_blocked_state_from_open_cycle() {
        let summary = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-stale-clear-summary".to_string(),
            task_id: "run-stale-clear-summary".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            recovery_ready: false,
            handoff_state: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                lifecycle_stage: "analysis_blocked".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                blocker_code: None,
                reporting_pause_gate: "continuation_check_required".to_string(),
                continuation_signal: "continuation_check_required".to_string(),
            },
        };
        let projection_truth = RunGraphProjectionTruth {
            projection_source: "reconciled_run_graph_status".to_string(),
            projection_reason: "run-graph status reflects persisted dispatch blocker evidence"
                .to_string(),
            dispatch_receipt_present: true,
            continuation_binding_present: false,
            projection_vs_receipt_parity: "aligned".to_string(),
            stale_state_suspected: true,
            next_lawful_operator_action: Some(
                "vida taskflow run-graph status run-stale-clear-summary --json".to_string(),
            ),
            dispatch_receipt: None,
            continuation_binding: None,
        };

        let (blocker_codes, why_not_now, _next_action, _command, _surface) =
            recovery_surface_contract(&summary, &projection_truth);

        assert_eq!(blocker_codes, vec!["tool_execution_failed".to_string()]);
        assert_eq!(
            why_not_now.as_ref().map(|value| value.category.as_str()),
            Some("stale_run_graph_blocked_state")
        );
        assert!(why_not_now
            .as_ref()
            .and_then(|value| Some(value.summary.as_str()))
            .is_some_and(|value| value.contains("persisted dispatch evidence looks stale")));
        assert!(why_not_now
            .as_ref()
            .and_then(|value| Some(value.summary.as_str()))
            .is_some_and(|value| !value.contains("delegated cycle remains open")));
        assert!(why_not_now
            .as_ref()
            .and_then(|value| Some(value.summary.as_str()))
            .is_some_and(|value| !value.contains("actively open")));
    }

    #[test]
    fn run_graph_diagnose_json_payload_keeps_operator_contract_parity() {
        let diagnosis = RunGraphDiagnosis {
            run_id: "run-diagnose-json".to_string(),
            blocker_codes: vec!["open_delegated_cycle".to_string()],
            why_not_now: Some(RecoveryWhyNotNow {
                category: "delegated_cycle_runtime_gate".to_string(),
                summary: "The delegated cycle remains open.".to_string(),
                blocker_codes: vec!["open_delegated_cycle".to_string()],
                blocking_surface: Some("vida taskflow recovery latest".to_string()),
            }),
            next_action: Some(RecoveryNextAction {
                command: "vida taskflow consume continue --run-id run-diagnose-json --json"
                    .to_string(),
                surface: "vida taskflow consume continue".to_string(),
                reason: "recovery is ready; continue the lawful delegated chain".to_string(),
            }),
            recommended_command: Some(
                "vida taskflow consume continue --run-id run-diagnose-json --json".to_string(),
            ),
            recommended_surface: Some("vida taskflow consume continue".to_string()),
            recovery: crate::state_store::RunGraphRecoverySummary {
                run_id: "run-diagnose-json".to_string(),
                task_id: "task-diagnose-json".to_string(),
                active_node: "implementer".to_string(),
                lifecycle_stage: "implementer_active".to_string(),
                resume_node: Some("verification".to_string()),
                resume_status: "ready".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.verification_lane".to_string(),
                policy_gate: "not_required".to_string(),
                handoff_state: "awaiting_verification".to_string(),
                recovery_ready: true,
                delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                    active_node: "implementer".to_string(),
                    delegated_cycle_open: true,
                    delegated_cycle_state: "handoff_pending".to_string(),
                    local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                    reporting_pause_gate: "non_blocking_only".to_string(),
                    continuation_signal: "continue_routing_non_blocking".to_string(),
                    blocker_code: Some("open_delegated_cycle".to_string()),
                    lifecycle_stage: "implementer_active".to_string(),
                },
            },
            projection_truth: RunGraphProjectionTruth {
                projection_source: "reconciled_run_graph_status".to_string(),
                projection_reason:
                    "run-graph status was reconciled against persisted dispatch receipt evidence"
                        .to_string(),
                dispatch_receipt_present: true,
                continuation_binding_present: false,
                projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
                stale_state_suspected: false,
                next_lawful_operator_action: Some(
                    "vida taskflow consume continue --run-id run-diagnose-json --json".to_string(),
                ),
                dispatch_receipt: None,
                continuation_binding: None,
            },
        };

        let payload =
            build_run_graph_diagnosis_json_payload(&diagnosis).expect("diagnosis should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph diagnose-latest")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn run_graph_diagnose_json_payload_for_surface_keeps_operator_contract_parity() {
        let diagnosis = RunGraphDiagnosis {
            run_id: "run-diagnose-surface-json".to_string(),
            blocker_codes: vec!["open_delegated_cycle".to_string()],
            why_not_now: Some(RecoveryWhyNotNow {
                category: "delegated_cycle_runtime_gate".to_string(),
                summary: "The delegated cycle remains open.".to_string(),
                blocker_codes: vec!["open_delegated_cycle".to_string()],
                blocking_surface: Some("vida taskflow recovery status".to_string()),
            }),
            next_action: Some(RecoveryNextAction {
                command: "vida taskflow consume continue --run-id run-diagnose-surface-json --json"
                    .to_string(),
                surface: "vida taskflow consume continue".to_string(),
                reason: "recovery is ready; continue the lawful delegated chain".to_string(),
            }),
            recommended_command: Some(
                "vida taskflow consume continue --run-id run-diagnose-surface-json --json"
                    .to_string(),
            ),
            recommended_surface: Some("vida taskflow consume continue".to_string()),
            recovery: crate::state_store::RunGraphRecoverySummary {
                run_id: "run-diagnose-surface-json".to_string(),
                task_id: "task-diagnose-surface-json".to_string(),
                active_node: "implementer".to_string(),
                lifecycle_stage: "implementer_active".to_string(),
                resume_node: Some("verification".to_string()),
                resume_status: "ready".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.verification_lane".to_string(),
                policy_gate: "not_required".to_string(),
                handoff_state: "awaiting_verification".to_string(),
                recovery_ready: true,
                delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                    active_node: "implementer".to_string(),
                    delegated_cycle_open: true,
                    delegated_cycle_state: "handoff_pending".to_string(),
                    local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                    reporting_pause_gate: "non_blocking_only".to_string(),
                    continuation_signal: "continue_routing_non_blocking".to_string(),
                    blocker_code: Some("open_delegated_cycle".to_string()),
                    lifecycle_stage: "implementer_active".to_string(),
                },
            },
            projection_truth: RunGraphProjectionTruth {
                projection_source: "reconciled_run_graph_status".to_string(),
                projection_reason:
                    "run-graph status was reconciled against persisted dispatch receipt evidence"
                        .to_string(),
                dispatch_receipt_present: true,
                continuation_binding_present: false,
                projection_vs_receipt_parity: "reconciled_from_receipt".to_string(),
                stale_state_suspected: false,
                next_lawful_operator_action: Some(
                    "vida taskflow consume continue --run-id run-diagnose-surface-json --json"
                        .to_string(),
                ),
                dispatch_receipt: None,
                continuation_binding: None,
            },
        };

        let payload = build_run_graph_diagnosis_json_payload_for_surface(
            "vida taskflow run-graph diagnose",
            &diagnosis,
        )
        .expect("diagnosis should render");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            serde_json::json!("vida taskflow run-graph diagnose")
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn sanitize_placeholder_terminal_bind_next_action_fails_closed_to_run_graph_status() {
        let next_action = RecoveryNextAction {
            command: "vida taskflow continuation bind <task-id> --run-id <run-id> --json"
                .to_string(),
            surface: "vida taskflow continuation bind".to_string(),
            reason: "placeholder bind".to_string(),
        };

        let next_action = sanitize_placeholder_continuation_bind_next_action(
            Some("run-terminal-bind"),
            Some(next_action),
        )
        .expect("next action should remain present");

        assert_eq!(
            next_action.command,
            "vida taskflow run-graph status run-terminal-bind"
        );
        assert_eq!(next_action.surface, "vida taskflow run-graph status");
        assert!(next_action
            .reason
            .contains("inspect the authoritative run state"));
    }

    #[test]
    fn sanitize_placeholder_terminal_bind_next_action_without_run_id_fails_closed_to_status() {
        let next_action = RecoveryNextAction {
            command: "vida taskflow continuation bind <task-id> --run-id <run-id> --json"
                .to_string(),
            surface: "vida taskflow continuation bind".to_string(),
            reason: "placeholder bind".to_string(),
        };

        let next_action =
            sanitize_placeholder_continuation_bind_next_action(None, Some(next_action))
                .expect("next action should remain present");

        assert_eq!(next_action.command, "vida status");
        assert_eq!(next_action.surface, "vida status");
        assert!(next_action
            .reason
            .contains("inspect the authoritative run state"));
    }

    #[test]
    fn sanitize_concrete_task_bind_next_action_fails_closed_to_run_graph_status() {
        let next_action = RecoveryNextAction {
            command: "vida taskflow continuation bind run-open --task-id task-open --json"
                .to_string(),
            surface: "vida taskflow continuation bind".to_string(),
            reason: "illegal lifecycle bind".to_string(),
        };

        let next_action =
            sanitize_placeholder_continuation_bind_next_action(Some("run-open"), Some(next_action))
                .expect("next action should remain present");

        assert_eq!(
            next_action.command,
            "vida taskflow run-graph status run-open"
        );
        assert_eq!(next_action.surface, "vida taskflow run-graph status");
        assert!(next_action
            .reason
            .contains("inspect the authoritative run state"));
    }

    #[test]
    fn run_graph_status_missing_run_id_json_is_actionable() {
        let payload = run_graph_missing_run_id_json_payload();

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "missing_run_id");
        assert!(payload["error"]
            .as_str()
            .expect("error should be a string")
            .contains("<run-id>"));
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .any(|value| value
                .as_str()
                .is_some_and(|action| action.contains("run-graph latest"))));
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .iter()
            .all(|value| value
                .as_str()
                .is_some_and(|action| !action.contains("--json"))));
    }
}
