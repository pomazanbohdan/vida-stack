use crate::{
    build_runtime_execution_plan_from_snapshot, dispatch_contract_execution_lane_sequence,
    operator_contracts::{
        canonical_release1_blocker_code_entries, finalize_release1_operator_truth,
        shared_operator_output_contract_parity_error,
    },
    print_surface_header, print_surface_line, read_or_sync_launcher_activation_snapshot,
    shell_quote,
    state_store::{
        RunGraphContinuationBinding, RunGraphDispatchContext, RunGraphDispatchReceipt,
        RunGraphStatus, StateStore, StateStoreError,
    },
    taskflow_layer4::print_taskflow_proxy_help,
    taskflow_task_bridge::proxy_state_dir,
    RenderMode, RuntimeConsumptionLaneSelection,
};
use std::collections::BTreeSet;
use std::{process::ExitCode, time::Duration};
use time::format_description::well_known::Rfc3339;

const STALE_PROJECTION_DISPATCH_TIMEOUT_SECONDS: i64 = 10;
const RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS: u64 = 60;
const RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER: &str = "run_graph_dispatch_init_timeout";
const TASKFLOW_RECOVERY_RECENT_PROJECTION_MAX_AGE: Duration = Duration::from_secs(60 * 60);

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

fn recovery_status_projection_name(run_id: &str) -> String {
    format!("taskflow-recovery-status-{}", projection_component(run_id))
}

fn read_recovery_status_projection(
    state_dir: &std::path::Path,
    projection_name: &str,
    run_id: &str,
) -> Option<String> {
    crate::operator_projection_cache::read_fresh_json_projection(state_dir, projection_name)
        .filter(|cached| recovery_projection_has_complete_action_fields(cached))
        .or_else(|| {
            crate::operator_projection_cache::read_recent_json_projection(
                state_dir,
                projection_name,
                TASKFLOW_RECOVERY_RECENT_PROJECTION_MAX_AGE,
            )
            .filter(|cached| recovery_projection_has_complete_action_fields(cached))
        })
        .or_else(|| {
            let cached = crate::operator_projection_cache::read_state_stale_recent_json_projection(
                state_dir,
                projection_name,
                TASKFLOW_RECOVERY_RECENT_PROJECTION_MAX_AGE,
            )?;
            if recovery_projection_has_complete_action_fields(&cached)
                && stale_recovery_projection_matches_lane_truth(state_dir, run_id, &cached)
            {
                Some(cached)
            } else {
                None
            }
        })
}

fn recovery_projection_has_complete_action_fields(cached: &str) -> bool {
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

fn stale_recovery_projection_matches_lane_truth(
    state_dir: &std::path::Path,
    run_id: &str,
    cached: &str,
) -> bool {
    let Ok(recovery_payload) = serde_json::from_str::<serde_json::Value>(cached) else {
        return false;
    };
    let recovery_blocked_open_cycle = recovery_payload
        .get("blocker_codes")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|codes| {
            codes
                .iter()
                .any(|code| code.as_str() == Some("open_delegated_cycle"))
        });
    if !recovery_blocked_open_cycle {
        return true;
    }
    let _ = state_dir;
    let _ = run_id;
    false
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

fn run_graph_operator_artifact_refs(surface: &str, run_id: &str) -> serde_json::Value {
    serde_json::json!({
        "surface": surface,
        "run_id": run_id,
    })
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

fn blocked_status_signal(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "blocked" || normalized == "lane_blocked" || normalized.ends_with("_blocked")
}

fn fallback_dispatch_blocker_code() -> String {
    crate::contract_profile_adapter::blocker_code_str(
        crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
    )
    .to_string()
}

fn normalize_run_graph_blocker_codes(
    blocker_codes: &[String],
    blocked_evidence_present: bool,
) -> Vec<String> {
    if blocker_codes
        .iter()
        .any(|code| code.trim() == "missing_owned_write_scope")
    {
        return vec!["missing_owned_write_scope".to_string()];
    }
    let normalized = crate::operator_contracts::normalize_blocker_codes(
        blocker_codes,
        crate::release_contract_adapters::canonical_blocker_codes,
        None,
    );
    if normalized.is_empty() && (blocked_evidence_present || !blocker_codes.is_empty()) {
        vec![fallback_dispatch_blocker_code()]
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
    let finalized = finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        run_graph_operator_artifact_refs(surface, run_id),
    )?;
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
    build_run_graph_operator_surface_payload(
        surface,
        &summary.run_id,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "why_not_now": why_not_now,
            "next_action": next_action,
            "recommended_command": recommended_command,
            "recommended_surface": recommended_surface,
            "recovery": summary,
            "projection_truth": projection_truth,
        }),
    )
}

fn build_run_graph_diagnosis_json_payload_for_surface(
    surface: &str,
    diagnosis: &RunGraphDiagnosis,
) -> Result<serde_json::Value, String> {
    let next_actions = operator_next_actions_for_operator_surface(
        &diagnosis.blocker_codes,
        diagnosis.next_action.as_ref(),
        diagnosis.why_not_now.as_ref(),
        diagnosis.recommended_command.as_deref(),
    );
    build_run_graph_operator_surface_payload(
        surface,
        &diagnosis.run_id,
        diagnosis.blocker_codes.clone(),
        next_actions,
        serde_json::json!({
            "why_not_now": diagnosis.why_not_now,
            "next_action": diagnosis.next_action,
            "recommended_command": diagnosis.recommended_command,
            "recommended_surface": diagnosis.recommended_surface,
            "recovery": diagnosis.recovery,
            "projection_truth": diagnosis.projection_truth,
        }),
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
    build_recovery_json_payload(
        "vida taskflow recovery latest",
        summary,
        projection_truth,
        blocker_codes,
        why_not_now,
        next_action,
        recommended_command,
        recommended_surface,
    )
}

fn build_run_graph_diagnosis_json_payload(
    diagnosis: &RunGraphDiagnosis,
) -> Result<serde_json::Value, String> {
    build_run_graph_diagnosis_json_payload_for_surface(
        "vida taskflow run-graph diagnose-latest",
        diagnosis,
    )
}

fn build_run_graph_status_json_payload(
    surface: &str,
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Result<serde_json::Value, String> {
    let blocker_codes = run_graph_status_surface_blocker_codes(status, projection_truth);
    let next_actions = operator_next_actions_for_operator_surface(
        &blocker_codes,
        None,
        None,
        projection_truth.next_lawful_operator_action.as_deref(),
    );
    build_run_graph_operator_surface_payload(
        surface,
        &status.run_id,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "run_id": status.run_id,
            "run_graph_status": status,
            "delegation_gate": status.delegation_gate(),
            "projection_truth": projection_truth,
        }),
    )
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

fn next_lawful_operator_action_for_status(status: &RunGraphStatus) -> Option<String> {
    if status.recovery_ready && status.resume_target != "none" {
        return Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            status.run_id
        ));
    }
    if status.status == "completed" {
        return None;
    }
    Some(format!(
        "vida taskflow run-graph status {} --json",
        status.run_id
    ))
}

fn fail_closed_terminal_continue_followup(status: &RunGraphStatus) -> String {
    format!("vida taskflow run-graph status {} --json", status.run_id)
}

fn sanitized_placeholder_continuation_bind_command(
    run_id: Option<&str>,
    command: Option<String>,
) -> Option<String> {
    let has_placeholder_bind = command.as_deref().is_some_and(|command| {
        command.starts_with("vida taskflow continuation bind")
            && (command.contains("<task-id>") || command.contains("<run-id>"))
    });
    if !has_placeholder_bind {
        return command;
    }
    run_id
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("vida taskflow run-graph status {value} --json"))
        .or_else(|| Some("vida status --json".to_string()))
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
            || next_action.command == "vida status --json"
        {
            next_action.reason = crate::status_surface_signals::terminal_next_action_requires_authoritative_run_state(run_id);
        }
        next_action
    })
}

fn dispatch_receipt_resolution_reason_class(receipt: &RunGraphDispatchReceipt) -> Option<&str> {
    if receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
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
            .or_else(|| Some(format!("vida lane show {} --json", status.run_id)));
    }
    let _reason_class = dispatch_receipt_resolution_reason_class(receipt)?;
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
            return Some(format!("vida lane show {} --json", status.run_id));
        }
        if terminal_consume_continue_run_id == Some(status.run_id.as_str()) {
            return Some(fail_closed_terminal_continue_followup(status));
        }
        return (status.status != "completed").then(|| {
            format!(
                "vida taskflow consume continue --run-id {} --json",
                status.run_id
            )
        });
    }
    Some(format!("vida lane show {} --json", status.run_id))
}

fn dispatch_receipt_has_clean_ready_downstream_handoff(receipt: &RunGraphDispatchReceipt) -> bool {
    receipt.dispatch_status == "executed"
        && receipt.blocker_code.is_none()
        && receipt.downstream_dispatch_ready
        && receipt.downstream_dispatch_blockers.is_empty()
        && receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"))
}

fn downstream_dispatch_command_for_receipt(receipt: &RunGraphDispatchReceipt) -> Option<String> {
    crate::continuation_binding_summary::downstream_dispatch_command_from_parts(
        receipt.downstream_dispatch_command.as_deref(),
        receipt.downstream_dispatch_packet_path.as_deref(),
    )
}

pub(crate) fn dispatch_receipt_can_use_terminal_continue_evidence(
    receipt: &RunGraphDispatchReceipt,
) -> bool {
    receipt.supersedes_receipt_id.is_some() && receipt.exception_path_receipt_id.is_some()
}

fn blocked_external_dispatch_artifact_mismatched_as_internal_activation(
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
) -> Option<String> {
    if task_missing && status.status == "blocked" {
        return Some(format!(
            "vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\" --json",
            status.run_id, status.run_id
        ));
    }
    if receipt.is_some_and(blocked_external_dispatch_artifact_mismatched_as_internal_activation) {
        if terminal_consume_continue_run_id == Some(status.run_id.as_str()) {
            return Some(fail_closed_terminal_continue_followup(status));
        }
        return Some(format!("vida lane show {} --json", status.run_id));
    }
    if let Some(command) = receipt.and_then(|value| {
        if value
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| blocker == "missing_owned_write_scope")
        {
            return Some(format!(
                "vida taskflow packet render {} --json",
                status.run_id
            ));
        }
        next_lawful_operator_action_for_dispatch_resolution(
            status,
            value,
            terminal_consume_continue_run_id,
        )
    }) {
        return Some(command);
    }
    next_lawful_operator_action_for_status(status)
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
    command
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
    let projection_resolves_open_cycle =
        recovery_projection_resolves_persisted_open_cycle(summary, projection_truth);
    let mut blocker_codes = if projection_resolves_open_cycle {
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
    blocker_codes.extend(projection_truth_blocker_codes(projection_truth));
    let blocker_codes = normalize_run_graph_blocker_codes(&blocker_codes, false);

    let next_action = projection_truth
        .next_lawful_operator_action
        .as_deref()
        .map(|command| RecoveryNextAction {
            command: command.to_string(),
            surface: recommended_surface_for_command(command),
            reason: recovery_next_action_reason(command, summary, projection_truth),
        });
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
    if projection_truth.projection_vs_receipt_parity != "reconciled_from_receipt" {
        return false;
    }
    if !projection_truth_blocker_codes(projection_truth).is_empty() {
        return false;
    }
    projection_truth
        .dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            let ready_downstream_handoff = receipt.downstream_dispatch_ready
                && receipt
                    .downstream_dispatch_status
                    .as_deref()
                    .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"));
            let clean_resolved_receipt = receipt.dispatch_status == "executed"
                && (receipt.lane_status == "lane_completed"
                    || (receipt.lane_status == "lane_running" && ready_downstream_handoff))
                && receipt.blocker_code.is_none()
                && receipt.downstream_dispatch_blockers.is_empty();
            let downstream_has_no_blocking_state = !receipt.downstream_dispatch_ready
                || receipt
                    .downstream_dispatch_status
                    .as_deref()
                    .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"));
            clean_resolved_receipt && downstream_has_no_blocking_state
        })
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

fn projection_reason_for_status(
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

fn continuation_binding_matches_reconciled_status(
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

fn active_exception_takeover_receipt_matches_status(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> bool {
    let Some(receipt) = receipt else {
        return false;
    };
    receipt.run_id == status.run_id
        && receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn status_derived_exception_takeover_binding(
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
        if continuation_binding_matches_reconciled_status(status, &binding) {
            return Some(binding);
        }
    }
    active_exception_takeover_receipt_matches_status(status, receipt)
        .then(|| status_derived_exception_takeover_binding(status))
}

fn continuation_binding_source_from_status_surface(
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

fn dispatch_receipt_from_status_surface(
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

fn downstream_dispatch_preview_from_status_snapshot(
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

fn dispatch_blocker_codes_from_status_surface(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    let mut blocked_evidence_present = false;
    if let Some(summary) = recovery {
        if let Some(blocker_code) = summary
            .delegation_gate
            .blocker_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            blocker_codes.push(blocker_code.to_string());
        }
        blocked_evidence_present |= blocked_status_signal(&summary.lifecycle_stage)
            || blocked_status_signal(&summary.resume_status);
    }
    if let Some(summary) = receipt {
        if let Some(blocker_code) = summary
            .blocker_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            blocker_codes.push(blocker_code.to_string());
        }
        blocker_codes.extend(
            summary
                .downstream_dispatch_blockers
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        );
        blocked_evidence_present |= blocked_status_signal(&summary.dispatch_status)
            || blocked_status_signal(&summary.lane_status)
            || summary
                .downstream_dispatch_status
                .as_deref()
                .is_some_and(blocked_status_signal);
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_blocker_codes(&blocker_codes, blocked_evidence_present)
}

fn dispatch_receipt_blocker_codes(
    receipt: &RunGraphDispatchReceipt,
    blocked_evidence_present: &mut bool,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
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
    *blocked_evidence_present |= blocked_status_signal(&receipt.dispatch_status)
        || blocked_status_signal(&receipt.lane_status)
        || receipt
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(blocked_status_signal);
    blocker_codes
}

fn projection_truth_blocker_codes(projection_truth: &RunGraphProjectionTruth) -> Vec<String> {
    let mut blocked_evidence_present = projection_truth.stale_state_suspected;
    let mut blocker_codes = Vec::new();
    if let Some(receipt) = projection_truth.dispatch_receipt.as_ref() {
        blocker_codes.extend(dispatch_receipt_blocker_codes(
            receipt,
            &mut blocked_evidence_present,
        ));
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_blocker_codes(&blocker_codes, blocked_evidence_present)
}

fn run_graph_status_surface_blocker_codes(
    status: &RunGraphStatus,
    projection_truth: &RunGraphProjectionTruth,
) -> Vec<String> {
    let mut blocked_evidence_present =
        blocked_status_signal(&status.status) || blocked_status_signal(&status.lifecycle_stage);
    let mut blocker_codes = projection_truth_blocker_codes(projection_truth);
    if let Some(receipt) = projection_truth.dispatch_receipt.as_ref() {
        blocker_codes.extend(dispatch_receipt_blocker_codes(
            receipt,
            &mut blocked_evidence_present,
        ));
    }
    blocker_codes.sort_unstable();
    blocker_codes.dedup();
    normalize_run_graph_blocker_codes(&blocker_codes, blocked_evidence_present)
}

fn projection_truth_from_status_surface(
    state_root: &std::path::Path,
    status: &RunGraphStatus,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    continuation_binding_source: Option<&str>,
) -> (RunGraphProjectionTruth, Vec<String>) {
    let status_surface_receipt = receipt.map(dispatch_receipt_from_status_surface);
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
    let stale_state_suspected = projection_stale_state_suspected(
        state_root,
        receipt
            .as_ref()
            .map(|summary| dispatch_receipt_from_status_surface(summary))
            .as_ref(),
    );
    let projection_truth = RunGraphProjectionTruth {
        projection_source: if receipt.is_some() {
            "reconciled_run_graph_status".to_string()
        } else {
            "persisted_run_graph_status".to_string()
        },
        projection_reason: projection_reason_for_status(
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
        ),
        dispatch_receipt: status_surface_receipt.clone(),
        continuation_binding: None,
    };
    let blocker_codes = if recovery.is_some_and(|summary| {
        recovery_projection_resolves_persisted_open_cycle(summary, &projection_truth)
    }) {
        projection_truth_blocker_codes(&projection_truth)
    } else {
        dispatch_blocker_codes_from_status_surface(recovery, receipt)
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
        continuation_binding_source_from_status_surface(continuation_binding);
    let evidence = activation_vs_execution_evidence.or_else(|| {
        receipt.and_then(|summary| {
            if summary.activation_evidence.is_null() {
                None
            } else {
                Some(&summary.activation_evidence)
            }
        })
    });
    let (projection_truth, blocker_codes) = projection_truth_from_status_surface(
        state_root,
        status,
        recovery,
        receipt,
        continuation_binding_source.as_deref(),
    );
    let (recommended_command, recommended_surface) = if let Some(summary) = recovery {
        let (_codes, _why_not_now, _next_action, command, surface) =
            recovery_surface_contract(summary, &projection_truth);
        if recovery_projection_resolves_persisted_open_cycle(summary, &projection_truth) {
            (command, surface)
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
    let (recommended_command, recommended_surface) =
        sanitize_placeholder_continuation_bind_recommendation(
            Some(&status.run_id),
            recommended_command,
            recommended_surface,
        );
    Some(RunGraphDispatchCompactSummary {
        route_truth: route_truth_from_projection_truth(&projection_truth, evidence),
        downstream_dispatch_preview: downstream_dispatch_preview_from_status_snapshot(
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
    if blocked_external_dispatch_artifact_mismatched_as_internal_activation(receipt) {
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

async fn run_graph_task_missing(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<bool, StateStoreError> {
    match store.show_task(&status.task_id).await {
        Ok(_) => Ok(false),
        Err(StateStoreError::MissingTask { .. }) => Ok(true),
        Err(error) => Err(error),
    }
}

pub(crate) async fn run_graph_projection_truth(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<RunGraphProjectionTruth, StateStoreError> {
    let dispatch_receipt = store.run_graph_dispatch_receipt(&status.run_id).await?;
    let persisted_continuation_binding =
        store.run_graph_continuation_binding(&status.run_id).await?;
    let task_missing = run_graph_task_missing(store, status).await?;
    let continuation_binding = if task_missing {
        None
    } else {
        effective_projection_continuation_binding(
            status,
            dispatch_receipt.as_ref(),
            persisted_continuation_binding,
        )
    };
    let terminal_consume_continue_run_id = if dispatch_receipt.as_ref().is_some_and(|receipt| {
        blocked_external_dispatch_artifact_mismatched_as_internal_activation(receipt)
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
    let stale_state_suspected =
        task_missing || projection_stale_state_suspected(store.root(), dispatch_receipt.as_ref());
    Ok(RunGraphProjectionTruth {
        projection_source: if dispatch_receipt.is_some() {
            "reconciled_run_graph_status".to_string()
        } else {
            "persisted_run_graph_status".to_string()
        },
        projection_reason: projection_reason_for_status(
            status,
            dispatch_receipt.as_ref(),
            continuation_binding.as_ref(),
        ),
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
            task_missing,
        ),
        dispatch_receipt,
        continuation_binding,
    })
}

#[derive(Clone)]
struct CompiledRunGraphControl {
    implementation: serde_json::Value,
    verification: serde_json::Value,
    first_execution_lane: String,
    validation_report_required_before_implementation: bool,
}

async fn compiled_run_graph_control(store: &StateStore) -> Result<CompiledRunGraphControl, String> {
    let snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
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
    let first_execution_lane = dispatch_contract_execution_lane_sequence(
        &execution_plan["development_flow"]["dispatch_contract"],
    )
    .into_iter()
    .next()
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "implementer".to_string());
    if implementation.is_null() {
        return Err(
            "run-graph control is unavailable in the compiled activation snapshot.".to_string(),
        );
    }

    Ok(CompiledRunGraphControl {
        implementation,
        verification,
        first_execution_lane,
        validation_report_required_before_implementation: selection.compiled_bundle
            ["autonomous_execution"]["validation_report_required_before_implementation"]
            .as_bool()
            .unwrap_or(false),
    })
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

pub(crate) fn default_run_graph_status(
    task_id: &str,
    task_class: &str,
    route_task_class: &str,
) -> RunGraphStatus {
    RunGraphStatus {
        run_id: task_id.to_string(),
        task_id: task_id.to_string(),
        task_class: task_class.to_string(),
        active_node: task_class.to_string(),
        next_node: None,
        status: "pending".to_string(),
        route_task_class: route_task_class.to_string(),
        selected_backend: "unknown".to_string(),
        lane_id: "unassigned".to_string(),
        lifecycle_stage: "initialized".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "none".to_string(),
        context_state: "open".to_string(),
        checkpoint_kind: "none".to_string(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    }
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
            match StateStore::open_existing_read_only(state_dir).await {
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
            match StateStore::open_existing_read_only(state_dir).await {
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
        [head, subcommand] if head == "recovery" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
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
                        ) = recovery_surface_contract(&summary, &projection_truth);
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
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
                    Ok(summary) => {
                        let projection_truth = match summary.as_ref() {
                            Some(summary) => match store.run_graph_status(&summary.run_id).await {
                                Ok(status) => {
                                    match run_graph_projection_truth(&store, &status).await {
                                        Ok(truth) => Some(truth),
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to build recovery projection truth: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read run-graph status for projection truth: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            },
                            None => None,
                        };
                        let contract = summary.as_ref().zip(projection_truth.as_ref()).map(
                            |(summary, projection_truth)| {
                                recovery_surface_contract(summary, projection_truth)
                            },
                        );
                        let payload = match (summary.as_ref(), projection_truth.as_ref(), contract)
                        {
                            (Some(summary), Some(projection_truth), Some(contract)) => {
                                build_recovery_latest_json_payload(
                                    summary,
                                    projection_truth,
                                    contract.0,
                                    contract.1,
                                    contract.2,
                                    contract.3,
                                    contract.4,
                                )
                            }
                            _ => Ok(serde_json::json!({
                                "surface": "vida taskflow recovery latest",
                                "status": null,
                            })),
                        };
                        match payload {
                            Ok(payload) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&payload)
                                        .expect("latest recovery summary should render as json")
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized recovery latest payload: {error}"
                                );
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
        [head, subcommand, run_id] if head == "recovery" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
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
                        ) = recovery_surface_contract(&summary, &projection_truth);
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery status");
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
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            let projection_name = recovery_status_projection_name(run_id);
            if let Some(cached) =
                read_recovery_status_projection(&state_dir, &projection_name, run_id)
            {
                println!("{cached}");
                return ExitCode::SUCCESS;
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
                        ) = recovery_surface_contract(&summary, &projection_truth);
                        match build_recovery_json_payload(
                            "vida taskflow recovery status",
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
                                        .expect("recovery summary should render as json")
                                );
                                crate::operator_projection_cache::write_json_projection(
                                    &proxy_state_dir(),
                                    &projection_name,
                                    &payload,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized recovery status payload: {error}"
                                );
                                ExitCode::from(1)
                            }
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
        [head, subcommand, ..] if head == "recovery" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow recovery status <run-id> [--json]");
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
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            if let Some(payload) = read_run_graph_dispatch_init_fast_cache(&state_dir, run_id) {
                crate::print_json_pretty(&payload);
                return ExitCode::SUCCESS;
            }
            run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, true).await
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            let state_dir = proxy_state_dir();
            if let Some(payload) = read_run_graph_dispatch_init_fast_cache(&state_dir, run_id) {
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
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(Some(status)) => {
                        match build_run_graph_diagnosis(&store, &status.run_id).await {
                            Ok(diagnosis) => {
                                print_surface_header(
                                    RenderMode::Plain,
                                    "vida taskflow run-graph diagnose-latest",
                                );
                                print_surface_line(RenderMode::Plain, "run", &diagnosis.run_id);
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
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "next action",
                                        &next_action.reason,
                                    );
                                }
                                if let Some(command) = diagnosis.recommended_command.as_deref() {
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "recommended_command",
                                        command,
                                    );
                                }
                                if let Some(surface) = diagnosis.recommended_surface.as_deref() {
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "recommended_surface",
                                        surface,
                                    );
                                }
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
        [head, subcommand] if head == "run-graph" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.latest_run_graph_status().await {
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
        [head, subcommand, flag]
            if head == "run-graph" && subcommand == "diagnose-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(Some(status)) => {
                        match build_run_graph_diagnosis(&store, &status.run_id).await {
                            Ok(diagnosis) => {
                                match build_run_graph_diagnosis_json_payload(&diagnosis) {
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
                Ok(store) => match store.latest_run_graph_status().await {
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
                                match build_run_graph_status_json_payload(
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
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        let projection_truth = match run_graph_projection_truth(&store, &status)
                            .await
                        {
                            Ok(truth) => truth,
                            Err(error) => {
                                eprintln!("Failed to build run-graph projection truth: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
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
                    Err(error) => {
                        if let Some(status) =
                            read_run_graph_status_json_from_dispatch_init_fast_cache(
                                &state_dir, run_id,
                            )
                        {
                            print_surface_header(
                                RenderMode::Plain,
                                "vida taskflow run-graph status",
                            );
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
                        } else {
                            eprintln!("Failed to read run-graph status: {error}");
                            ExitCode::from(1)
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "diagnose" => {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match build_run_graph_diagnosis(&store, run_id).await {
                    Ok(diagnosis) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph diagnose");
                        print_surface_line(RenderMode::Plain, "run", &diagnosis.run_id);
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
                            print_surface_line(
                                RenderMode::Plain,
                                "next action",
                                &next_action.reason,
                            );
                        }
                        if let Some(command) = diagnosis.recommended_command.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_command", command);
                        }
                        if let Some(surface) = diagnosis.recommended_surface.as_deref() {
                            print_surface_line(RenderMode::Plain, "recommended_surface", surface);
                        }
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
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        let projection_truth = match run_graph_projection_truth(&store, &status)
                            .await
                        {
                            Ok(truth) => truth,
                            Err(error) => {
                                eprintln!("Failed to build run-graph projection truth: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        match build_run_graph_status_json_payload(
                            "vida taskflow run-graph status",
                            &status,
                            &projection_truth,
                        ) {
                            Ok(payload) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&payload)
                                        .expect("run-graph status should render as json")
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to render normalized run-graph status payload: {error}"
                                );
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        if let Some(status) =
                            read_run_graph_status_json_from_dispatch_init_fast_cache(
                                &state_dir, run_id,
                            )
                        {
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
                            eprintln!("Failed to read run-graph status: {error}");
                            ExitCode::from(1)
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "diagnose" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match build_run_graph_diagnosis(&store, run_id).await {
                    Ok(diagnosis) => {
                        match build_run_graph_diagnosis_json_payload_for_surface(
                            "vida taskflow run-graph diagnose",
                            &diagnosis,
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
            eprintln!("Usage: vida taskflow run-graph latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "diagnose-latest" => {
            eprintln!("Usage: vida taskflow run-graph diagnose-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "diagnose" => {
            eprintln!("Usage: vida taskflow run-graph diagnose <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow run-graph status <run-id> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
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
        payload["incident"] = evidence["incident"].clone();
        payload["blockers"] = evidence["blockers"].clone();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("run-graph error should render as json")
    );
}

fn run_graph_dispatch_init_timeout_message(run_id: &str) -> String {
    format!(
        "Timed out preparing run-graph dispatch-init for `{run_id}` after {RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS}s; dispatch-init may have written a packet before timeout, but the surface failed closed instead of holding the state-store lock."
    )
}

fn run_graph_dispatch_init_error_evidence(error: &str) -> Option<serde_json::Value> {
    if !error.starts_with("Timed out preparing run-graph dispatch-init") {
        return None;
    }
    Some(serde_json::json!({
        "incident": {
            "status": "blocked",
            "summary": "run-graph dispatch-init timed out before returning a bounded dispatch receipt"
        },
        "blockers": [RUN_GRAPH_DISPATCH_INIT_TIMEOUT_BLOCKER]
    }))
}

async fn record_dispatch_init_timeout_blocker(
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
    record_run_graph_status_with_continuation_sync(
        store,
        &status,
        "run_graph_dispatch_init_timeout",
    )
    .await?;
    Ok(true)
}

fn run_graph_blocker_code(status: &str) -> Option<&'static str> {
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

fn run_graph_blocker_evidence(
    args: RunGraphBlockerEvidenceArgs<'_>,
) -> Result<Option<serde_json::Value>, String> {
    let is_blocked_advance = args.error.starts_with("run-graph advance blocked:");
    if !is_blocked_advance {
        return Ok(None);
    }
    let blocker_code = run_graph_blocker_code(args.status).ok_or_else(|| {
        format!(
            "run-graph advance blocked without explicit blocker evidence for `{}` status `{}`; refusing to continue (fail-closed)",
            args.run_id, args.status
        )
    })?;
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
        }]
    })))
}

pub(crate) fn is_dispatch_resume_handoff_complete(status: &RunGraphStatus) -> bool {
    if !status.resume_target.starts_with("dispatch.") {
        return true;
    }
    status.next_node.is_some()
        && !status.policy_gate.trim().is_empty()
        && status.policy_gate != "none"
        && !status.handoff_state.trim().is_empty()
        && status.handoff_state != "none"
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
    if !is_dispatch_resume_handoff_complete(status) {
        return Err(format!(
            "Run-graph resume gate denied for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
            status.run_id,
            status.resume_target,
            status.next_node.as_deref().unwrap_or("none"),
            status.policy_gate,
            status.handoff_state
        ));
    }
    if !status.delegation_gate().delegated_cycle_open {
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

pub(crate) fn status_with_active_exception_dispatch_replay(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceipt,
) -> Option<RunGraphStatus> {
    if !active_exception_takeover_receipt_matches_status(status, Some(receipt)) {
        return None;
    }
    let node = dispatch_replay_node_from_receipt(receipt)?;
    let (handoff_state, resume_target) =
        governance_handoff(Some(&node), DispatchTargetFormat::Lane);
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

async fn reconcile_dispatch_init_status_for_active_exception(
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
        .and_then(|receipt| status_with_active_exception_dispatch_replay(&status, receipt))
        .unwrap_or(status))
}

fn reconcile_dispatch_init_status_for_missing_receipt(
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
    let (handoff_state, resume_target) = governance_handoff(Some(&node), target_format);
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

async fn record_run_graph_status_with_continuation_sync(
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

#[derive(Clone, Copy)]
enum DispatchTargetFormat {
    Lane,
    Direct,
}

fn governance_handoff(
    next_node: Option<&str>,
    target_format: DispatchTargetFormat,
) -> (String, String) {
    let handoff_state = next_node
        .map(|next| format!("awaiting_{next}"))
        .unwrap_or_else(|| "none".to_string());
    let resume_target = next_node
        .map(|next| match target_format {
            DispatchTargetFormat::Lane => format!("dispatch.{next}_lane"),
            DispatchTargetFormat::Direct => format!("dispatch.{next}"),
        })
        .unwrap_or_else(|| "none".to_string());
    (handoff_state, resume_target)
}

struct RunGraphTransitionArgs {
    active_node: String,
    next_node: Option<String>,
    lane_id: String,
    lifecycle_stage: String,
    policy_gate: String,
    checkpoint_kind: String,
    target_format: DispatchTargetFormat,
    recovery_ready: bool,
}

fn run_graph_transition(existing: &RunGraphStatus, args: RunGraphTransitionArgs) -> RunGraphStatus {
    let (handoff_state, resume_target) =
        governance_handoff(args.next_node.as_deref(), args.target_format);

    RunGraphStatus {
        run_id: existing.run_id.clone(),
        task_id: existing.task_id.clone(),
        task_class: existing.task_class.clone(),
        active_node: args.active_node,
        next_node: args.next_node,
        status: "ready".to_string(),
        route_task_class: existing.route_task_class.clone(),
        selected_backend: existing.selected_backend.clone(),
        lane_id: args.lane_id,
        lifecycle_stage: args.lifecycle_stage,
        policy_gate: args.policy_gate,
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: args.checkpoint_kind,
        resume_target,
        recovery_ready: args.recovery_ready,
    }
}

fn implementation_analysis_gate(
    implementation: &serde_json::Value,
) -> (Option<String>, String, bool) {
    let writer_node = implementation_writer_node(implementation);
    let coach_required = json_bool_field(implementation, "coach_required").unwrap_or(false);
    let next_node = Some(writer_node);
    let policy_gate = if coach_required {
        json_string_field(implementation, "verification_gate")
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
    json_string_field(implementation, "writer_route_task_class")
        .or_else(|| json_string_field(implementation, "implementer_route_task_class"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "writer".to_string())
}

fn implementation_verification_gate(
    implementation: &serde_json::Value,
    verification: &serde_json::Value,
) -> (Option<String>, String) {
    let verification_route = json_string_field(implementation, "verification_route_task_class")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "verification".to_string());
    let next_node = json_bool_field(implementation, "independent_verification_required")
        .unwrap_or(false)
        .then_some(verification_route);
    let policy_gate = if next_node.is_some() {
        json_string_field(verification, "verification_gate")
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
        let coach_node = json_string_field(implementation, "coach_route_task_class")
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
        let verification_node = json_string_field(implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        return (
            verification_node,
            None,
            json_string_field(verification, "verification_gate")
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
    store: &StateStore,
    task_id: &str,
) -> Option<String> {
    let task = store.show_task(task_id).await.ok()?;
    if task.labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "spec-pack" | "documentation" | "work-pool-pack" | "dev-pack"
        )
    }) {
        return None;
    }
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
    execution_plan: &mut serde_json::Value,
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
    let Some(design_doc_path) = existing_design_backed_task_design_doc_path(store, task_id).await
    else {
        return Ok(());
    };
    let design_doc_has_bounded_scope =
        design_doc_has_bounded_file_set(std::path::Path::new(&design_doc_path)).unwrap_or(false);
    let existing_design_scope_discussion = selection.conversational_mode.as_deref()
        == Some("scope_discussion")
        && selection.tracked_flow_entry.as_deref() == Some("spec-pack");
    let existing_design_work_pool_discussion = selection.conversational_mode.as_deref()
        == Some("pbi_discussion")
        && selection.tracked_flow_entry.as_deref() == Some("work-pool-pack");
    let already_explicit_implementation = selection.conversational_mode.is_none()
        && selection.selected_role == "worker"
        && selection
            .reason
            .starts_with("auto_explicit_implementation_request");

    let normalized_request = request_text.to_lowercase();
    let implementation_terms =
        crate::runtime_lane_summary::explicit_implementation_request_terms(&normalized_request);
    let bounded_repair_terms =
        crate::runtime_lane_summary::explicit_bounded_code_repair_terms(&normalized_request);
    let matched_terms = if !implementation_terms.is_empty() {
        implementation_terms
    } else if !bounded_repair_terms.is_empty() {
        bounded_repair_terms
    } else if design_doc_has_bounded_scope
        && (existing_design_scope_discussion || already_explicit_implementation)
    {
        vec!["existing_design_backed_spec_override".to_string()]
    } else if existing_design_work_pool_discussion {
        vec!["existing_design_backed_work_pool_override".to_string()]
    } else if already_explicit_implementation {
        selection.execution_plan =
            build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, selection);
        inject_tracked_design_doc_path(&mut selection.execution_plan, &design_doc_path);
        return Ok(());
    } else {
        return Ok(());
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
    selection.reason = "auto_existing_design_backed_implementation_request_override".to_string();
    selection.execution_plan =
        build_runtime_execution_plan_from_snapshot(&selection.compiled_bundle, selection);
    inject_tracked_design_doc_path(&mut selection.execution_plan, &design_doc_path);
    Ok(())
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
    fn blocked(
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

fn terminal_completed_without_next_unit_for_packet_gate(status: &RunGraphStatus) -> bool {
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
        return RunGraphPacketBackedExecutionGate::blocked(
            "blocked_task_run_mapping_mismatch",
            None,
            None,
            vec!["scheduler_packet_execute_requires_single_selected_task"],
        );
    };
    let Some(status) = status else {
        return RunGraphPacketBackedExecutionGate::blocked(
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
        return RunGraphPacketBackedExecutionGate::blocked(
            "blocked_task_run_mapping_mismatch",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["blocked_task_run_mapping_mismatch"],
        );
    }
    if terminal_completed_without_next_unit_for_packet_gate(status) {
        return RunGraphPacketBackedExecutionGate::blocked(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["terminal_run_graph_status_without_next_unit"],
        );
    }

    let Some(context) = context else {
        return RunGraphPacketBackedExecutionGate::blocked(
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
        return RunGraphPacketBackedExecutionGate::blocked(
            "blocked_lineage_preconditions_not_verified",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["invalid_run_graph_dispatch_context"],
        );
    }

    let Some(binding) = binding else {
        return RunGraphPacketBackedExecutionGate::blocked(
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
        return RunGraphPacketBackedExecutionGate::blocked(
            "blocked_task_run_mapping_mismatch",
            Some(selected_task_id),
            Some(status.run_id.as_str()),
            vec!["blocked_task_run_mapping_mismatch"],
        );
    }

    let Some(receipt) = receipt else {
        return RunGraphPacketBackedExecutionGate::blocked(
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
        return RunGraphPacketBackedExecutionGate::blocked(
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
        return RunGraphPacketBackedExecutionGate::blocked(
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

pub(crate) async fn derive_seeded_run_graph_status(
    store: &StateStore,
    requested_run_id: &str,
    request_text: &str,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    let bounded_task_id =
        resolve_seed_task_id_for_runtime_run(store, requested_run_id, request_text).await?;
    let snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
    let mut payload = build_seeded_run_graph_status_from_activation_snapshot(
        requested_run_id,
        &bounded_task_id,
        request_text,
        &snapshot,
    )?;
    try_existing_design_backed_implementation_override(
        store,
        &bounded_task_id,
        request_text,
        &mut payload.role_selection,
    )
    .await?;
    if payload.role_selection.request != request_text {
        payload.request_text = request_text.to_string();
    }
    Ok(payload)
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

fn dispatch_init_route_targets(execution_plan: &serde_json::Value) -> Vec<String> {
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let mut targets =
        crate::taskflow_routing::dispatch_contract_execution_lane_sequence(dispatch_contract);
    targets.push("implementation".to_string());
    if targets.is_empty() {
        targets.extend(
            ["implementation", "coach", "verification"]
                .into_iter()
                .map(str::to_string),
        );
    }
    let mut unique = BTreeSet::new();
    targets
        .into_iter()
        .map(|target| match target.as_str() {
            "implementer" | "analysis" => "implementation".to_string(),
            "execution_preparation" => "architecture".to_string(),
            _ => target,
        })
        .filter(|target| !target.trim().is_empty())
        .filter(|target| unique.insert(target.clone()))
        .collect()
}

fn dispatch_context_route_assignment_catalog_drift(
    state_root: &std::path::Path,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Option<serde_json::Value> {
    let project_root =
        crate::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(state_root);
    let catalog = crate::runtime_dispatch_state::current_project_model_profile_catalog_for_root(
        project_root.as_ref(),
    );
    if catalog.is_empty() {
        return None;
    }
    for target in dispatch_init_route_targets(&role_selection.execution_plan) {
        let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &role_selection.execution_plan,
            &target,
        );
        let payload = crate::taskflow_routing::route_explain_payload(
            &role_selection.execution_plan,
            &target,
            route,
        );
        if let Some(drift) = crate::runtime_dispatch_state::route_assignment_catalog_drift_payload(
            &payload, &catalog,
        ) {
            return Some(serde_json::json!({
                "dispatch_target": target,
                "drift": drift,
            }));
        }
    }
    None
}

async fn reseed_dispatch_context_after_route_assignment_drift(
    store: &StateStore,
    status: &RunGraphStatus,
    context: &RunGraphDispatchContext,
) -> Result<TaskflowRunGraphSeedPayload, String> {
    let task_id = if status.task_id.trim().is_empty() {
        context.task_id.as_str()
    } else {
        status.task_id.as_str()
    };
    if let Ok(task) = store.show_task(task_id).await {
        if taskflow_task_status_is_terminal_for_dispatch_init(&task.status) {
            return Err(format!(
                "Dispatch-init cannot refresh stale route assignment for terminal TaskFlow task `{task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
                task.status
            ));
        }
    }
    derive_seeded_run_graph_status(store, task_id, &context.request_text).await
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
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &payload.status,
        "run_graph_seed",
    )
    .await?;
    Ok(())
}

pub(crate) fn run_graph_dispatch_bootstrap_from_status(
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
        serde_json::json!({
            "surface": "vida taskflow run-graph dispatch-init",
            "requested_run_id": self.requested_run_id,
            "run_id": self.run_id,
            "dispatch_receipt": self.dispatch_receipt,
            "dispatch_packet_path": self.dispatch_packet_path,
            "downstream_dispatch_packet_path": downstream_dispatch_packet_path,
            "taskflow_handoff_plan": self.taskflow_handoff_plan,
            "run_graph_bootstrap": self.run_graph_bootstrap,
        })
    }
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
    !packet_path.is_empty() && std::path::Path::new(packet_path).is_file()
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
        &payload,
        requested_run_id,
        current_config_digest.as_deref(),
    )
    .then_some(payload)
}

fn read_run_graph_status_json_from_dispatch_init_fast_cache(
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
    Some(packet_path.to_string())
}

async fn existing_routed_dispatch_init_artifacts(
    store: &StateStore,
    requested_run_id: &str,
    run_id: &str,
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
    let Some(dispatch_packet_path) = reusable_routed_dispatch_receipt(&dispatch_receipt) else {
        return Ok(None);
    };
    let role_selection = context
        .role_selection()
        .map_err(|error| format!("Failed to decode existing seeded dispatch context: {error}"))?;
    if dispatch_context_route_assignment_catalog_drift(store.root(), &role_selection).is_some() {
        return Ok(None);
    }
    let run_graph_bootstrap = run_graph_dispatch_bootstrap_from_status(&status)?;
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
    if let Ok(task) = store.show_task(bound_task_id).await {
        if taskflow_task_status_is_terminal_for_dispatch_init(&task.status) {
            return Err(format!(
                "Run `{requested_run_id}` has explicit continuation binding to terminal task_graph_task `{bound_task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
                task.status
            ));
        }
    }

    let request_text = if let Some(request_text) = binding
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

    let payload = derive_seeded_run_graph_status(store, bound_task_id, &request_text).await?;
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

fn taskflow_task_status_is_terminal_for_dispatch_init(status: &str) -> bool {
    matches!(status.trim(), "closed" | "completed")
}

fn build_seeded_run_graph_status_from_activation_snapshot(
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
    let lane_node = if is_conversation {
        selection.selected_role.clone()
    } else if selection.selected_role == "worker" {
        dispatch_contract_execution_lane_sequence(
            &execution_plan["development_flow"]["dispatch_contract"],
        )
        .into_iter()
        .next()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            json_string_field(route, "analysis_route_task_class").filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| selection.selected_role.clone())
    } else {
        json_string_field(route, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| selection.selected_role.clone())
    };
    let selected_backend =
        crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
            execution_plan,
            if is_conversation {
                lane_node.as_str()
            } else {
                "implementer"
            },
            json_string_field(route, "activation_agent_type").as_deref(),
            None,
        )
        .unwrap_or_else(|| "unknown".to_string());
    let lane_id = format!("{lane_node}_lane");
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
        ..default_run_graph_status(requested_run_id, "planning", "implementation")
    };
    let mut status = run_graph_transition(
        &seed_base,
        RunGraphTransitionArgs {
            active_node: "planning".to_string(),
            next_node,
            lane_id,
            lifecycle_stage,
            policy_gate,
            checkpoint_kind,
            target_format: DispatchTargetFormat::Lane,
            recovery_ready,
        },
    );
    status.task_class = seed_base.task_class;
    status.route_task_class = seed_base.route_task_class;
    status.selected_backend = seed_base.selected_backend;
    status.handoff_state = handoff_state;

    Ok(TaskflowRunGraphSeedPayload {
        request_text: request_text.to_string(),
        role_selection: selection,
        status,
    })
}

async fn seed_existing_task_payload_for_dispatch_init(
    store: &StateStore,
    task_id: &str,
) -> Result<Option<TaskflowRunGraphSeedPayload>, String> {
    let task = match store.show_task(task_id).await {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    if taskflow_task_status_is_terminal_for_dispatch_init(&task.status) {
        return Err(format!(
            "Dispatch-init cannot seed terminal TaskFlow task `{task_id}` with status `{}`; bind a non-terminal bounded unit before dispatch-init.",
            task.status
        ));
    }
    let request_text = task_record_dispatch_seed_request_text(&task);
    let payload = derive_seeded_run_graph_status(store, task_id, &request_text).await?;
    Ok(Some(payload))
}

async fn preview_run_graph_dispatch_init_artifacts(
    store: &StateStore,
    run_id: &str,
) -> Result<RunGraphDispatchInitPreview, String> {
    if let Some(artifacts) = existing_routed_dispatch_init_artifacts(store, run_id, run_id).await? {
        return Ok(RunGraphDispatchInitPreview::Existing(artifacts));
    }

    let effective_run_id = run_id.to_string();
    let mut seed_payload = None;
    let status = match store.run_graph_status(&effective_run_id).await {
        Ok(status) => status,
        Err(error) => {
            let original_error = format!(
                "Failed to read run-graph state for `{}`: {error}",
                effective_run_id
            );
            match seed_existing_task_payload_for_dispatch_init(store, &effective_run_id).await? {
                Some(payload) => {
                    let status = payload.status.clone();
                    seed_payload = Some(payload);
                    status
                }
                None => return Err(original_error),
            }
        }
    };
    let mut status = reconcile_dispatch_init_status_for_active_exception(store, status).await?;

    let context = if let Some(payload) = seed_payload.as_ref() {
        run_graph_dispatch_context_from_seed_payload(payload)
    } else {
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
            if let Some(payload) =
                seed_existing_task_payload_for_dispatch_init(store, &effective_run_id).await?
            {
                status = reconcile_dispatch_init_status_for_active_exception(
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

    let mut role_selection = context
        .role_selection()
        .map_err(|error| format!("Failed to decode persisted seeded dispatch context: {error}"))?;
    let route_assignment_drift =
        dispatch_context_route_assignment_catalog_drift(store.root(), &role_selection);
    if route_assignment_drift.is_some() {
        let payload =
            reseed_dispatch_context_after_route_assignment_drift(store, &status, &context).await?;
        status = reconcile_dispatch_init_status_for_active_exception(store, payload.status.clone())
            .await?;
        role_selection = payload.role_selection.clone();
        seed_payload = Some(payload);
    }
    let existing_dispatch_receipt = if route_assignment_drift.is_none() {
        store
            .run_graph_dispatch_receipt(&effective_run_id)
            .await
            .map_err(|error| format!("Failed to read existing dispatch receipt: {error}"))?
    } else {
        None
    };
    if route_assignment_drift.is_none() {
        status = reconcile_dispatch_init_status_for_missing_receipt(
            status,
            &role_selection,
            existing_dispatch_receipt.is_some(),
        );
    }
    let run_graph_bootstrap = run_graph_dispatch_bootstrap_from_status(&status)?;
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
    if let Ok(task) = store.show_task(&effective_run_id).await {
        inject_task_planner_metadata(&mut role_selection.execution_plan, &task.planner_metadata);
    }
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    crate::runtime_dispatch_state::sync_receipt_configured_activation_assignment(
        &role_selection,
        &mut dispatch_receipt,
    );
    dispatch_receipt.dispatch_command = crate::runtime_dispatch_command_for_target(
        &role_selection,
        &dispatch_receipt.dispatch_target,
    );
    crate::refresh_downstream_dispatch_preview(
        store,
        &role_selection,
        &run_graph_bootstrap,
        &mut dispatch_receipt,
    )
    .await?;
    let ctx = crate::RuntimeDispatchPacketContext::new(
        store.root(),
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    );
    let dispatch_packet_path = crate::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
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
                crate::taskflow_continuation::sync_run_graph_continuation_binding(
                    &store,
                    &prepared.status,
                    "run_graph_dispatch_init",
                )
                .await?;
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
    match preview_run_graph_dispatch_init_artifacts(store, run_id).await? {
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

async fn run_graph_dispatch_init_from_state_dir(
    state_dir: &std::path::Path,
    run_id: &str,
) -> Result<serde_json::Value, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(RUN_GRAPH_DISPATCH_INIT_TIMEOUT_SECONDS),
        async {
            let store = StateStore::open_existing_read_only(state_dir.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to open read-only state store for dispatch-init preparation: {error}"
                    )
                })?;
            let preview = preview_run_graph_dispatch_init_artifacts(&store, run_id).await?;
            store.close().await;
            commit_previewed_run_graph_dispatch_init_artifacts(state_dir, preview).await
        },
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            if let Ok(store) = StateStore::open_existing(state_dir.to_path_buf()).await {
                let _ = record_dispatch_init_timeout_blocker(&store, run_id).await;
            }
            Err(run_graph_dispatch_init_timeout_message(run_id))
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
            let _ = record_dispatch_init_timeout_blocker(store, run_id).await;
            Err(run_graph_dispatch_init_timeout_message(run_id))
        }
    }
}

pub(crate) async fn derive_advanced_run_graph_status(
    store: &StateStore,
    existing: RunGraphStatus,
) -> Result<TaskflowRunGraphAdvancePayload, String> {
    let compiled_control = compiled_run_graph_control(store).await?;
    let implementation = compiled_control.implementation;

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

        let analysis_node = json_string_field(&implementation, "analysis_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "analysis".to_string());
        let direct_writer_entry = compiled_control.first_execution_lane.clone();
        if existing.next_node.as_deref() == Some(direct_writer_entry.as_str()) {
            let coach_required =
                json_bool_field(&implementation, "coach_required").unwrap_or(false);
            let verification = compiled_control.verification.clone();
            let (next_node, policy_gate) =
                implementation_verification_gate(&implementation, &verification);
            return Ok(TaskflowRunGraphAdvancePayload {
                status: run_graph_transition(
                    &existing,
                    RunGraphTransitionArgs {
                        active_node: direct_writer_entry.clone(),
                        next_node: if coach_required {
                            json_string_field(&implementation, "coach_route_task_class")
                                .filter(|value| !value.is_empty())
                                .or(next_node)
                        } else {
                            next_node
                        },
                        lane_id: format!("{direct_writer_entry}_lane"),
                        lifecycle_stage: "writer_active".to_string(),
                        policy_gate,
                        checkpoint_kind: "execution_cursor".to_string(),
                        target_format: DispatchTargetFormat::Lane,
                        recovery_ready: true,
                    },
                ),
            });
        }

        if existing.next_node.as_deref() != Some(analysis_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{analysis_node}` or `{direct_writer_entry}` for the seeded implementation run, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let (next_node, policy_gate, recovery_ready) =
            implementation_analysis_gate(&implementation);

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: analysis_node.clone(),
                    next_node,
                    lane_id: format!("{analysis_node}_lane"),
                    lifecycle_stage: "analysis_active".to_string(),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready,
                },
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
                status: run_graph_transition(
                    &existing,
                    RunGraphTransitionArgs {
                        active_node: existing.active_node.clone(),
                        next_node,
                        lane_id: existing.lane_id.clone(),
                        lifecycle_stage: "analysis_active".to_string(),
                        policy_gate,
                        checkpoint_kind: "execution_cursor".to_string(),
                        target_format: DispatchTargetFormat::Lane,
                        recovery_ready,
                    },
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
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: writer_node.clone(),
                    next_node: if coach_required {
                        json_string_field(&implementation, "coach_route_task_class")
                            .filter(|value| !value.is_empty())
                            .or(next_node)
                    } else {
                        next_node
                    },
                    lane_id: format!("{writer_node}_lane"),
                    lifecycle_stage: "writer_active".to_string(),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: true,
                },
            ),
        });
    }

    if existing.task_class == "implementation"
        && existing.route_task_class == "implementation"
        && existing.active_node == implementation_writer_node(&implementation)
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
            let mut status = run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: existing.active_node.clone(),
                    next_node: None,
                    lane_id: existing.lane_id.clone(),
                    lifecycle_stage: "implementation_complete".to_string(),
                    policy_gate: "not_required".to_string(),
                    checkpoint_kind: existing.checkpoint_kind.clone(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: false,
                },
            );
            status.status = "completed".to_string();
            status.context_state = existing.context_state;
            return Ok(TaskflowRunGraphAdvancePayload { status });
        }

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: active_node.clone(),
                    next_node,
                    lane_id: format!("{active_node}_lane"),
                    lifecycle_stage: format!("{active_node}_active"),
                    policy_gate,
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format,
                    recovery_ready,
                },
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

        let verification_node = json_string_field(&implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        if existing.next_node.as_deref() != Some(verification_node.as_str()) {
            return Err(format!(
                "run-graph advance expected next node `{verification_node}` for the implementation coach handoff, got `{}`",
                existing.next_node.as_deref().unwrap_or("none")
            ));
        }

        let verification = compiled_control.verification.clone();

        return Ok(TaskflowRunGraphAdvancePayload {
            status: run_graph_transition(
                &existing,
                RunGraphTransitionArgs {
                    active_node: verification_node.clone(),
                    next_node: None,
                    lane_id: format!("{verification_node}_lane"),
                    lifecycle_stage: format!("{verification_node}_active"),
                    policy_gate: json_string_field(&verification, "verification_gate")
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| existing.policy_gate.clone()),
                    checkpoint_kind: "execution_cursor".to_string(),
                    target_format: DispatchTargetFormat::Lane,
                    recovery_ready: false,
                },
            ),
        });
    }

    if existing.task_class == "implementation" && existing.route_task_class == "implementation" {
        let verification_node = json_string_field(&implementation, "verification_route_task_class")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "verification".to_string());
        if existing.active_node != verification_node {
            // fall through
        } else {
            match implementation_verification_outcome(existing.status.as_str()) {
                ImplementationVerificationOutcome::ReworkReady => {
                    let analysis_node =
                        json_string_field(&implementation, "analysis_route_task_class")
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
                        status: run_graph_transition(
                            &existing,
                            RunGraphTransitionArgs {
                                active_node: analysis_node.clone(),
                                next_node,
                                lane_id: format!("{analysis_node}_lane"),
                                lifecycle_stage: "analysis_active".to_string(),
                                policy_gate,
                                checkpoint_kind: "execution_cursor".to_string(),
                                target_format: DispatchTargetFormat::Lane,
                                recovery_ready,
                            },
                        ),
                    });
                }
                ImplementationVerificationOutcome::Clean => {
                    let mut status = run_graph_transition(
                        &existing,
                        RunGraphTransitionArgs {
                            active_node: existing.active_node.clone(),
                            next_node: Some("approval".to_string()),
                            lane_id: existing.lane_id.clone(),
                            lifecycle_stage: "approval_wait".to_string(),
                            policy_gate:
                                crate::release1_contracts::ApprovalStatus::ApprovalRequired
                                    .as_str()
                                    .to_string(),
                            checkpoint_kind: existing.checkpoint_kind.clone(),
                            target_format: DispatchTargetFormat::Direct,
                            recovery_ready: true,
                        },
                    );
                    status.status = "awaiting_approval".to_string();
                    status.context_state = existing.context_state;
                    return Ok(TaskflowRunGraphAdvancePayload { status });
                }
                ImplementationVerificationOutcome::Approved => {
                    let mut status = run_graph_transition(
                        &existing,
                        RunGraphTransitionArgs {
                            active_node: existing.active_node.clone(),
                            next_node: None,
                            lane_id: existing.lane_id.clone(),
                            lifecycle_stage: "implementation_complete".to_string(),
                            policy_gate: "not_required".to_string(),
                            checkpoint_kind: existing.checkpoint_kind.clone(),
                            target_format: DispatchTargetFormat::Lane,
                            recovery_ready: false,
                        },
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
                let mut status = run_graph_transition(
                    &existing,
                    RunGraphTransitionArgs {
                        active_node: analyst_node.clone(),
                        next_node: next_node.clone(),
                        lane_id: format!("{analyst_node}_lane"),
                        lifecycle_stage: "conversation_active".to_string(),
                        policy_gate: existing.policy_gate.clone(),
                        checkpoint_kind: "conversation_cursor".to_string(),
                        target_format: DispatchTargetFormat::Lane,
                        recovery_ready: true,
                    },
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
    let store = match StateStore::open_existing(state_dir.to_path_buf()).await {
        Ok(store) => store,
        Err(error) => {
            let message = format!(
                "Failed to open authoritative state store for dispatch-init mutation: {error}"
            );
            if as_json {
                print_run_graph_json_error(
                    "vida taskflow run-graph dispatch-init",
                    run_id,
                    &message,
                    run_graph_dispatch_init_error_evidence(&message),
                );
            } else {
                eprintln!("{message}");
            }
            return ExitCode::from(1);
        }
    };

    match run_graph_dispatch_init(&store, run_id).await {
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
    match args {
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "dispatch-init" && flag == "--json" =>
        {
            if let Some(payload) = read_run_graph_dispatch_init_fast_cache(&state_dir, run_id) {
                crate::print_json_pretty(&payload);
                return ExitCode::SUCCESS;
            }
            return run_taskflow_run_graph_dispatch_init_mutation(&state_dir, run_id, true).await;
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "dispatch-init" => {
            if let Some(payload) = read_run_graph_dispatch_init_fast_cache(&state_dir, run_id) {
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
            let payload = match derive_advanced_run_graph_status(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("{error}");
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
            let payload = match derive_advanced_run_graph_status(&store, existing).await {
                Ok(payload) => payload,
                Err(error) => {
                    let evidence = match run_graph_blocker_evidence(RunGraphBlockerEvidenceArgs {
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

            let payload = match derive_seeded_run_graph_status(&store, task_id, &request_text).await
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
                                "payload": payload,
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
            let status = default_run_graph_status(task_id, task_class, task_class);
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
            let status = default_run_graph_status(task_id, task_class, route_task_class);
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
                    default_run_graph_status(task_id, task_class, task_class)
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
            match record_run_graph_status_with_continuation_sync(
                &store,
                &merged,
                "run_graph_update",
            )
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
                    default_run_graph_status(task_id, task_class, route_task_class)
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
            match record_run_graph_status_with_continuation_sync(
                &store,
                &merged,
                "run_graph_update",
            )
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
                    default_run_graph_status(task_id, task_class, route_task_class)
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
            match record_run_graph_status_with_continuation_sync(
                &store,
                &merged,
                "run_graph_update",
            )
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
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        }
    }

    #[test]
    fn dispatch_init_timeout_error_surfaces_blocker_evidence() {
        let message = run_graph_dispatch_init_timeout_message("run-timeout");
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
            &recovery_status_projection_name(run_id),
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

        assert!(read_recovery_status_projection(
            &root,
            &recovery_status_projection_name(run_id),
            run_id
        )
        .is_none());

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
                "next_lawful_operator_action": "vida lane show run-action-cache --json"
            }
        })
        .to_string();
        let complete = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_action": {
                "command": "vida lane show run-action-cache --json"
            },
            "recommended_command": "vida lane show run-action-cache --json",
            "projection_truth": {
                "next_lawful_operator_action": "vida lane show run-action-cache --json"
            }
        })
        .to_string();

        assert!(!recovery_projection_has_complete_action_fields(&incomplete));
        assert!(recovery_projection_has_complete_action_fields(&complete));
    }

    #[test]
    fn recovery_projection_rejects_lane_show_when_downstream_agent_init_packet_is_ready() {
        let stale_lane_show = serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_action": {
                "command": "vida lane show run-action-cache --json"
            },
            "recommended_command": "vida lane show run-action-cache --json",
            "projection_truth": {
                "next_lawful_operator_action": "vida lane show run-action-cache --json",
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

        assert!(!recovery_projection_has_complete_action_fields(
            &stale_lane_show
        ));
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
            &recovery_status_projection_name(run_id),
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

        assert!(read_recovery_status_projection(
            &root,
            &recovery_status_projection_name(run_id),
            run_id
        )
        .is_none());

        let _ = std::fs::remove_dir_all(&root);
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
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let run_id = "dispatch-init-timeout-missing-receipt";
        let mut status = default_run_graph_status(run_id, "implementation", "implementation");
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
            record_dispatch_init_timeout_blocker(&store, run_id)
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
        let mut status = default_run_graph_status(task_id, "implementation", "implementation");
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
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
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

    #[tokio::test]
    async fn projection_truth_recommends_retire_for_missing_task_stale_blocked_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        let run_id = "runtime-missing-task-stale-run";
        let task_id = "missing-authoritative-task";

        let mut status = default_run_graph_status(task_id, "implementation", "implementation");
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
    fn governance_handoff_uses_lane_targets_for_execution() {
        let (handoff_state, resume_target) =
            governance_handoff(Some("coach"), DispatchTargetFormat::Lane);
        assert_eq!(handoff_state, "awaiting_coach");
        assert_eq!(resume_target, "dispatch.coach_lane");
    }

    #[test]
    fn governance_handoff_uses_direct_targets_for_conversation() {
        let (handoff_state, resume_target) =
            governance_handoff(Some("spec-pack"), DispatchTargetFormat::Direct);
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
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };

        let reconciled =
            reconcile_dispatch_init_status_for_missing_receipt(status, &role_selection, false);

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

        assert_eq!(payload["status"], "blocked");
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
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false);

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
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false);

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
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false);

        assert_eq!(
            command.as_deref(),
            Some("vida lane show run-internal-codex-carrier --json")
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
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let command =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false);

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
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        let next_lawful_operator_action =
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false);
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

        let payload = build_run_graph_status_json_payload(
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
                .contains("vida taskflow packet render run-status-missing-owned-scope --json"))));
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
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false)
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
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false)
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
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false)
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
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        assert_eq!(
            next_lawful_operator_action_for_projection(
                &status,
                Some(&receipt),
                Some("run-active-exception"),
                false,
            )
            .as_deref(),
            Some("vida taskflow run-graph status run-active-exception --json")
        );
    }

    #[test]
    fn recovery_status_action_for_internal_timeout_points_to_lane_show() {
        let mut status = default_run_graph_status("run-timeout", "implementation", "coach");
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
            next_lawful_operator_action_for_projection(&status, Some(&receipt), None, false)
                .as_deref(),
            Some("vida lane show run-timeout --json")
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

        let payload = build_run_graph_status_json_payload(
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
                "vida taskflow run-graph status {} --json",
                status.run_id
            )),
            dispatch_receipt: Some(receipt),
            continuation_binding: None,
        };

        let payload = build_run_graph_status_json_payload(
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
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-26T00:00:00Z".to_string(),
        };

        let (_projection_truth, blocker_codes) = projection_truth_from_status_surface(
            std::path::Path::new("."),
            &status,
            None,
            Some(&receipt),
            None,
        );

        assert_eq!(blocker_codes, vec!["tool_execution_failed".to_string()]);
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

        let payload = build_run_graph_status_json_payload(
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

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_bound_repair_with_file_scope_terms() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");

        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open state store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let payload = derive_seeded_run_graph_status(
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
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "taskflow-runtime-run-binding-task-missing-actionability",
                title: "Runtime run binding task missing actionability",
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
            .expect("create active task");

        let payload = derive_seeded_run_graph_status(
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
        let dispatch_context = run_graph_dispatch_context_from_seed_payload(&payload);
        assert_eq!(
            dispatch_context.run_id,
            "runtime-fix-runtime-run-binding-task-missing"
        );
        assert_eq!(
            dispatch_context.task_id,
            "taskflow-runtime-run-binding-task-missing-actionability"
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
        let payload = derive_seeded_run_graph_status(
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
            .create_task(crate::state_store::CreateTaskRequest {
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
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
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

        let payload = derive_seeded_run_graph_status(
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
            "auto_existing_design_backed_implementation_request_override"
        );
        assert_eq!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_eq!(
            payload.role_selection.execution_plan["status"].as_str(),
            Some("ready_for_runtime_routing")
        );
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str(),
            Some("docs/product/spec/existing-design-route-fix-design.md")
        );
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_existing_design_backed_qwen_remediation_task(
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let payload = derive_seeded_run_graph_status(
            &store,
            "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
            "Bounded audit-remediation task. Remove qwen_cli from active runtime/config/code/test assumptions and retain it only in template/reference surfaces where it is intentionally documented as a non-active example carrier.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(
            payload.role_selection.reason,
            "auto_existing_design_backed_implementation_request_override"
        );
        assert!(payload
            .role_selection
            .matched_terms
            .iter()
            .any(|term| term == "existing_design_backed_work_pool_override"));
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_ne!(payload.status.next_node.as_deref(), Some("pm"));
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str()
                .map(|path| path.replace('\\', "/")),
            Some("docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md")
                .map(str::to_string)
        );
    }

    #[tokio::test]
    async fn derive_seeded_run_graph_prefers_worker_for_existing_design_backed_blocker_without_file_terms(
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let payload = derive_seeded_run_graph_status(
            &store,
            "feature-design-backed-reseed-blocker",
            "Bounded audit-remediation blocker. A finalized design-backed task is still reseeded into specification/planning instead of continuing into the implementation lane.",
        )
        .await
        .expect("seed should be generated");

        assert_eq!(payload.role_selection.selected_role, "worker");
        assert!(payload.role_selection.conversational_mode.is_none());
        assert_eq!(
            payload.role_selection.reason,
            "auto_existing_design_backed_implementation_request_override"
        );
        assert!(payload
            .role_selection
            .matched_terms
            .iter()
            .all(|term| term != ".rs" && term != "crates/" && term != "src/"));
        assert!(!payload.role_selection.matched_terms.is_empty());
        assert_eq!(
            payload.role_selection.tracked_flow_entry.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str(),
            Some("docs/product/spec/design-backed-reseed-blocker-design.md")
        );
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let payload = derive_seeded_run_graph_status(
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
            "auto_existing_design_backed_implementation_request_override"
        );
        assert_eq!(payload.status.task_class, "implementation");
        assert_eq!(payload.status.route_task_class, "implementation");
        assert_eq!(
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
                .as_str(),
            Some("docs/product/spec/direct-explicit-implementation-seed-design.md")
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
            compiled_bundle: serde_json::Value::Null,
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

        assert_eq!(receipt.dispatch_target, "implementer");
        assert!(receipt.dispatch_packet_path.is_some());
        assert!(payload["dispatch_packet_path"].as_str().is_some());
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
            .create_task(crate::state_store::CreateTaskRequest {
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
            compiled_bundle: serde_json::Value::Null,
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

        let artifacts =
            prepare_run_graph_dispatch_init_artifacts(&store, "task-writer-planner-scope")
                .await
                .expect("dispatch init artifacts should be prepared from planner metadata");
        assert_eq!(artifacts.dispatch_receipt.dispatch_target, "writer");
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
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale dispatch receipt");

        let payload = derive_seeded_run_graph_status(
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
        let payload = derive_seeded_run_graph_status(
            &store,
            run_id,
            "Fix run-graph dispatch-init replay after active exception takeover. Owned paths: crates/vida/src/taskflow_run_graph.rs.",
        )
        .await
        .expect("seed should be generated");
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

        let mut stale_status = default_run_graph_status("run-old", "closure", "delivery");
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let mut stale_status = default_run_graph_status("run-old-closed", "closure", "delivery");
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
            .create_task(crate::state_store::CreateTaskRequest {
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
            .create_task(crate::state_store::CreateTaskRequest {
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let payload = derive_seeded_run_graph_status(
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
            .create_task(crate::state_store::CreateTaskRequest {
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
        let payload = derive_seeded_run_graph_status(
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
            .create_task(crate::state_store::CreateTaskRequest {
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

        let first = run_graph_dispatch_init(&store, "task-dispatch-init-idempotent-fast-path")
            .await
            .expect("first dispatch-init should seed and route");
        let second = run_graph_dispatch_init(&store, "task-dispatch-init-idempotent-fast-path")
            .await
            .expect("second dispatch-init should reuse existing routed packet");

        assert_eq!(
            first["dispatch_packet_path"],
            second["dispatch_packet_path"]
        );
        assert_eq!(
            first["dispatch_receipt"]["recorded_at"],
            second["dispatch_receipt"]["recorded_at"]
        );
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
    async fn dispatch_init_refreshes_stale_route_assignment_after_model_catalog_change() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");

        store
            .create_task(crate::state_store::CreateTaskRequest {
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
        force_selected_model_ref(&mut context.role_selection, "gpt-5.5");
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
            "implementation",
            route,
        );
        assert_ne!(payload["selected_model_ref"], "gpt-5.5");
    }

    #[tokio::test]
    async fn dispatch_init_reseeds_design_backed_explicit_binding_into_implementer_lane() {
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

        let mut stale_status = default_run_graph_status(requested_run_id, "closure", "delivery");
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
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: bound_task_id,
                title: "Design-backed reseed canonicalization qwen blocker",
                display_id: None,
                description:
                    "Bounded audit-remediation blocker for design-backed reseed canonicalization.",
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

        let payload = run_graph_dispatch_init(&store, requested_run_id)
            .await
            .expect("dispatch init should reseed and produce an implementer dispatch");

        assert_eq!(payload["requested_run_id"], requested_run_id);
        assert_eq!(payload["run_id"], bound_task_id);
        assert_eq!(
            payload["run_graph_bootstrap"]["latest_status"]["task_class"].as_str(),
            Some("implementation")
        );
        assert_eq!(
            payload["run_graph_bootstrap"]["latest_status"]["next_node"].as_str(),
            Some("implementer")
        );
        assert_eq!(
            payload["run_graph_bootstrap"]["latest_status"]["route_task_class"].as_str(),
            Some("implementation")
        );
        assert_eq!(
            payload["dispatch_receipt"]["dispatch_target"].as_str(),
            Some("implementer")
        );
        assert_eq!(
            payload["dispatch_receipt"]["activation_agent_type"].as_str(),
            Some("junior")
        );
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
            Some("implementer")
        );
        assert_eq!(
            dispatch_packet["delivery_task_packet"]["handoff_runtime_role"].as_str(),
            Some("worker")
        );
        assert_eq!(
            dispatch_packet["activation_agent_type"].as_str(),
            Some("junior")
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
                "crates/vida/src/taskflow_run_graph.rs",
                "crates/vida/src/taskflow_consume.rs",
                "crates/vida/src/taskflow_consume_resume.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
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
                        compiled_bundle: serde_json::Value::Null,
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

        let mut stale_status = default_run_graph_status("run-stale-latest", "closure", "delivery");
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
    async fn seeded_worker_run_can_advance_directly_into_implementer_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let store = StateStore::open(harness.path().to_path_buf())
            .await
            .expect("open store");
        write_activation_snapshot_for_store(&store)
            .await
            .expect("activation snapshot should be written");
        let existing = RunGraphStatus {
            run_id: "task-direct-implementer".to_string(),
            task_id: "task-direct-implementer".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&existing)
            .await
            .expect("record run status");

        let payload = derive_advanced_run_graph_status(&store, existing)
            .await
            .expect("seeded implementer run should advance");

        assert_eq!(payload.status.active_node, "implementer");
        assert_eq!(payload.status.lifecycle_stage, "writer_active");
        assert_eq!(payload.status.next_node.as_deref(), Some("coach"));
        assert_eq!(payload.status.handoff_state, "awaiting_coach");
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
            default_run_graph_status("run-1", "implementation", "implementation");
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

        let mut completed = default_run_graph_status("run-1", "implementation", "implementation");
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
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale blocked receipt with explicit takeover");

        let mut status =
            default_run_graph_status("run-terminal-update", "implementation", "implementation");
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        record_run_graph_status_with_continuation_sync(&store, &status, "run_graph_update")
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
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            projection_reason_for_status(&status, Some(&receipt), None),
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

        assert_eq!(
            next_lawful_operator_action_for_status(&status).as_deref(),
            Some("vida taskflow consume continue --run-id run-projection-continue --json")
        );
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

        assert_eq!(
            next_lawful_operator_action_for_projection(
                &status,
                Some(&receipt),
                Some("run-terminal-ready-handoff"),
                false,
            )
            .as_deref(),
            Some(
                "vida agent-init --downstream-packet downstream-packet.json --execute-dispatch --json"
            )
        );
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
            "vida taskflow run-graph status run-terminal-bind --json"
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

        assert_eq!(next_action.command, "vida status --json");
        assert_eq!(next_action.surface, "vida status");
        assert!(next_action
            .reason
            .contains("inspect the authoritative run state"));
    }
}
