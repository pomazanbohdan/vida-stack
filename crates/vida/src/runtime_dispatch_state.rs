use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use time::format_description::well_known::Rfc3339;

use super::*;
use crate::release1_contracts::canonical_lane_status_str;
use crate::runtime_consumption_surface::RuntimeConsumptionClosureAdmissionEvidence;
use crate::runtime_contract_vocab::{
    canonical_dispatch_target_name, RUNTIME_ROLE_BUSINESS_ANALYST, RUNTIME_ROLE_COACH,
    RUNTIME_ROLE_PM, RUNTIME_ROLE_SOLUTION_ARCHITECT, RUNTIME_ROLE_VERIFIER,
    TASK_CLASS_ARCHITECTURE, TASK_CLASS_COACH, TASK_CLASS_IMPLEMENTATION, TASK_CLASS_SPECIFICATION,
    TASK_CLASS_VERIFICATION,
};
#[cfg(test)]
use crate::runtime_dispatch_downstream_packets::downstream_dispatch_packet_body;
use crate::runtime_dispatch_downstream_packets::{
    write_runtime_downstream_dispatch_packet,
    write_runtime_downstream_dispatch_packet_at_with_owned_paths,
    write_runtime_downstream_dispatch_packet_with_owned_paths,
};
use crate::runtime_dispatch_execution::{
    agent_lane_dispatch_result, execute_external_agent_lane_dispatch,
    execute_internal_agent_lane_dispatch, internal_host_external_fallback_backend,
};
use crate::runtime_dispatch_packet_text::{runtime_packet_prompt, runtime_tracked_flow_packet};
#[cfg(test)]
use crate::runtime_dispatch_packets::explicit_request_scope_paths;
#[cfg(test)]
use crate::runtime_dispatch_packets::runtime_delivery_task_packet;
use crate::runtime_dispatch_packets::{
    delivery_packet_owned_paths, is_runtime_consumption_fallback_owned_path,
    normalize_safe_owned_scope_path_candidate, request_has_explicit_owned_scope,
    runtime_coach_review_packet, runtime_delivery_task_packet_with_scope_context,
    runtime_escalation_packet, runtime_execution_block_packet, runtime_verifier_proof_packet,
    single_task_move_scope_paths,
};
use crate::taskflow_routing::{
    activation_backend_from_route, backend_selection_source, dispatch_target_for_runtime_role,
    fallback_executor_backend_from_route, fanout_executor_backends_from_route,
    route_primary_backend_hint_from_route, runtime_assignment_backend_for_route,
    runtime_assignment_from_execution_plan, runtime_assignment_from_route,
    runtime_assignment_source_from_execution_plan,
};

pub(crate) fn normalize_persisted_runtime_path(path: &str) -> std::path::PathBuf {
    let trimmed = path.trim();
    #[cfg(windows)]
    {
        if let Some(rest) = trimmed.strip_prefix("/mnt/") {
            let mut parts = rest.splitn(2, '/');
            if let (Some(drive), Some(tail)) = (parts.next(), parts.next()) {
                if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
                    let mut normalized = String::new();
                    normalized.push_str(&drive.to_ascii_uppercase());
                    normalized.push_str(":\\");
                    normalized.push_str(&tail.replace('/', "\\"));
                    return std::path::PathBuf::from(normalized);
                }
            }
        }
    }
    std::path::PathBuf::from(trimmed)
}

const DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_DISPATCH_HANDOFF_EXECUTION_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_INTERNAL_HOST_HANDOFF_TIMEOUT_SECONDS: u64 = 240;
const INTERNAL_DISPATCH_HANDOFF_TIMEOUT_GRACE_SECONDS: u64 = 2;
const LEGACY_STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS: i64 = 10;
pub(crate) const INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT: &str =
    "internal_dispatch_timeout_without_receipt";
pub(crate) const INTERNAL_CODEX_CARRIER_UNAVAILABLE: &str = "internal_codex_carrier_unavailable";

fn dispatch_state_reopen_failure_message(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    phase: &str,
    error: impl std::fmt::Display,
) -> String {
    format!(
        "Failed to reopen authoritative state store {phase} for run `{}` target `{}`: {error}",
        receipt.run_id, receipt.dispatch_target
    )
}

async fn reopen_authoritative_state_store_for_dispatch_phase(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    phase: &str,
) -> Result<StateStore, String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out reopening authoritative state store {phase} for run `{}` target `{}` after {}s",
            receipt.run_id,
            receipt.dispatch_target,
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })?
    .map_err(|error| dispatch_state_reopen_failure_message(receipt, phase, error))
}

fn configured_internal_host_handoff_timeout_seconds(project_root: &Path) -> Option<u64> {
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let (_system_id, system_entry) = selected_host_cli_system_for_runtime_dispatch(&overlay);
    system_entry
        .as_ref()
        .and_then(|entry| yaml_lookup(entry, &["max_runtime_seconds"]))
        .and_then(serde_yaml::Value::as_u64)
        .filter(|seconds| *seconds > 0)
}

fn configured_internal_host_no_output_timeout_seconds(project_root: &Path) -> Option<u64> {
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let (_system_id, system_entry) = selected_host_cli_system_for_runtime_dispatch(&overlay);
    system_entry
        .as_ref()
        .and_then(|entry| {
            yaml_lookup(entry, &["dispatch", "no_output_timeout_seconds"])
                .and_then(serde_yaml::Value::as_u64)
        })
        .filter(|seconds| *seconds > 0)
}

fn configured_internal_host_receipt_backed_completion_supported(
    project_root: &Path,
) -> Option<bool> {
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let (_system_id, system_entry) = selected_host_cli_system_for_runtime_dispatch(&overlay);
    system_entry.as_ref().and_then(|entry| {
        yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
            .and_then(serde_yaml::Value::as_bool)
    })
}

fn configured_external_backend_handoff_timeout_seconds(
    project_root: &Path,
    backend_id: &str,
) -> Option<u64> {
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let backend_entry = configured_external_backend_entry(&overlay, backend_id)?;
    yaml_lookup(backend_entry, &["max_runtime_seconds"])
        .and_then(serde_yaml::Value::as_u64)
        .or_else(|| {
            yaml_lookup(backend_entry, &["dispatch", "no_output_timeout_seconds"])
                .and_then(serde_yaml::Value::as_u64)
        })
        .filter(|seconds| *seconds > 0)
}

fn route_runtime_window_seconds(route: &serde_json::Value) -> Option<u64> {
    route["max_runtime_seconds"]
        .as_u64()
        .filter(|seconds| *seconds > 0)
}

fn compiled_bundle_route_runtime_window_seconds(
    compiled_bundle: &serde_json::Value,
    dispatch_target: &str,
) -> Option<u64> {
    let route_key = match dispatch_target {
        "implementer" | "writer" | "analysis" => "implementation",
        "execution_preparation" => "architecture",
        _ => dispatch_target,
    };
    compiled_bundle["agent_system"]["routing"][route_key]["max_runtime_seconds"]
        .as_u64()
        .filter(|seconds| *seconds > 0)
}

fn configured_route_runtime_window_seconds(
    project_root: &Path,
    dispatch_target: &str,
) -> Option<u64> {
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let route_key = match dispatch_target {
        "implementer" | "writer" | "analysis" => "implementation",
        "execution_preparation" => "architecture",
        _ => dispatch_target,
    };
    yaml_lookup(
        &overlay,
        &["agent_system", "routing", route_key, "max_runtime_seconds"],
    )
    .and_then(serde_yaml::Value::as_u64)
    .filter(|seconds| *seconds > 0)
}

pub(crate) fn internal_host_runtime_window_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &receipt.dispatch_target,
    )
    .and_then(route_runtime_window_seconds)
    .or_else(|| {
        compiled_bundle_route_runtime_window_seconds(
            &role_selection.compiled_bundle,
            &receipt.dispatch_target,
        )
    })
    .or_else(|| configured_route_runtime_window_seconds(project_root, &receipt.dispatch_target))
    .or_else(|| configured_internal_host_handoff_timeout_seconds(project_root))
    .unwrap_or(DEFAULT_INTERNAL_HOST_HANDOFF_TIMEOUT_SECONDS)
}

fn dispatch_handoff_timeout_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    let preferred_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    if dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        return internal_host_runtime_window_seconds(project_root, role_selection, receipt)
            .saturating_add(INTERNAL_DISPATCH_HANDOFF_TIMEOUT_GRACE_SECONDS);
    }
    preferred_backend
        .as_deref()
        .and_then(|backend_id| {
            configured_external_backend_handoff_timeout_seconds(project_root, backend_id)
        })
        .unwrap_or(DEFAULT_DISPATCH_HANDOFF_EXECUTION_TIMEOUT_SECONDS)
        .saturating_add(INTERNAL_DISPATCH_HANDOFF_TIMEOUT_GRACE_SECONDS)
}

pub(crate) fn dispatch_execution_started_stale_after_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    let handoff_timeout_seconds =
        dispatch_handoff_timeout_seconds(project_root, role_selection, receipt);
    if dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        return handoff_timeout_seconds;
    }
    if receipt
        .dispatch_surface
        .as_deref()
        .is_some_and(|surface| surface.trim() == "vida agent-init")
        || receipt
            .dispatch_command
            .as_deref()
            .is_some_and(|command| command.trim_start().starts_with("vida agent-init"))
    {
        return handoff_timeout_seconds;
    }
    handoff_timeout_seconds
}

fn dispatch_execution_timeout_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    dispatch_handoff_timeout_seconds(project_root, role_selection, receipt)
}

fn dispatch_handoff_requires_outer_timeout(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_kind != "agent_lane" {
        return true;
    }
    if dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        // Keep a secondary outer timeout around internal host dispatch. The inner
        // command wrapper remains the primary guard, but live delegated handoffs
        // have shown that pipe/process edge cases can still strand the orchestrator
        // after the bounded window unless the outer state-machine timeout also
        // remains active.
        return true;
    }
    false
}

pub(crate) fn sync_receipt_dispatch_handoff_surface(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) {
    if receipt.dispatch_kind != "agent_lane" {
        return;
    }
    let Some(dispatch_packet_path) = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let preferred_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    let preferred_model_profile_id = preferred_selected_model_profile_for_dispatch_target(
        role_selection,
        &receipt.dispatch_target,
        preferred_backend.as_deref(),
    );
    let lane_dispatch = runtime_agent_lane_dispatch_for_root(
        project_root,
        dispatch_packet_path,
        preferred_backend.as_deref(),
        preferred_model_profile_id.as_deref(),
    );
    receipt.dispatch_surface = Some(lane_dispatch.surface);
    receipt.dispatch_command = Some(lane_dispatch.activation_command);
}

fn dispatch_handoff_uses_internal_host(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_kind != "agent_lane" {
        return false;
    }
    let Some(dispatch_packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return false;
    };
    let overlay = load_project_overlay_yaml_for_root(project_root).ok();
    let overlay_host_selection = overlay
        .as_ref()
        .map(|overlay| selected_host_cli_system_for_runtime_dispatch(overlay));
    let host_runtime = runtime_host_execution_contract_for_root(project_root);
    let host_execution_class = json_string(host_runtime.get("selected_cli_execution_class"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            overlay_host_selection
                .as_ref()
                .and_then(|(_, entry)| entry.as_ref())
                .and_then(|entry| yaml_string(yaml_lookup(entry, &["execution_class"])))
        })
        .unwrap_or_default();
    let selected_cli_system = json_string(host_runtime.get("selected_cli_system"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            overlay_host_selection
                .as_ref()
                .map(|(system, _)| system.clone())
        })
        .unwrap_or_default();
    let preferred_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    let preferred_model_profile_id = preferred_selected_model_profile_for_dispatch_target(
        role_selection,
        &receipt.dispatch_target,
        preferred_backend.as_deref(),
    );
    let lane_dispatch = runtime_agent_lane_dispatch_for_root(
        project_root,
        dispatch_packet_path,
        preferred_backend.as_deref(),
        preferred_model_profile_id.as_deref(),
    );
    let lane_dispatch_backend_class = lane_dispatch.backend_dispatch["backend_class"].as_str();
    let lane_dispatch_execution_class =
        lane_dispatch.backend_dispatch["selected_execution_class"].as_str();
    let lane_dispatch_internal_surface =
        dispatch_surface_is_internal_host_surface(Some(lane_dispatch.surface.as_str()));
    let receipt_internal_surface =
        dispatch_surface_is_internal_host_surface(receipt.dispatch_surface.as_deref());
    let selected_backend_class = preferred_backend
        .as_deref()
        .or(receipt.selected_backend.as_deref())
        .and_then(|backend_id| {
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
        });
    let internal_host_carrier_backend = lane_dispatch_backend_class != Some("external_cli")
        && preferred_backend
            .as_deref()
            .or(receipt.selected_backend.as_deref())
            .is_some_and(|backend_id| {
                configured_internal_host_carrier_exists(
                    overlay.as_ref(),
                    &selected_cli_system,
                    backend_id,
                )
            });
    let internal_agent_backend = internal_host_carrier_backend
        || backend_class_is_internal(selected_backend_class.as_deref())
        || backend_class_is_internal(lane_dispatch_backend_class)
        || lane_dispatch_execution_class == Some("internal")
        || receipt_internal_surface;
    (lane_dispatch_internal_surface || receipt_internal_surface)
        && host_execution_class == "internal"
        && internal_agent_backend
}

pub(crate) fn internal_host_activation_view_only_requires_terminal_blocker(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if !dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        return false;
    }
    configured_internal_host_receipt_backed_completion_supported(project_root) == Some(false)
}

pub(crate) fn internal_host_activation_view_only_blocker_code(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> &'static str {
    if internal_host_app_bridge_requires_prelaunch_blocker(project_root, role_selection, receipt) {
        INTERNAL_CODEX_CARRIER_UNAVAILABLE
    } else {
        INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
    }
}

fn internal_host_receipt_backed_completion_is_enabled(entry: Option<&serde_yaml::Value>) -> bool {
    entry.and_then(|entry| {
        yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
            .and_then(serde_yaml::Value::as_bool)
    }) == Some(true)
}

fn dispatch_surface_is_internal_host_surface(surface: Option<&str>) -> bool {
    let Some(surface) = surface.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    surface == "vida agent-init"
        || surface
            .strip_prefix("internal_cli:")
            .is_some_and(|system| !system.trim().is_empty())
}

fn selected_backend_is_internal_host_bridge(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let selected_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    let is_internal = [
        selected_backend.as_deref(),
        receipt.selected_backend.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|backend_id| {
        backend_class_is_internal(
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
                .as_deref(),
        ) || configured_internal_host_carrier_exists(Some(overlay), selected_cli_system, backend_id)
            || configured_subagent_entry(overlay, backend_id).is_some_and(|entry| {
                yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
                    .as_deref()
                    .is_some_and(|backend_class| backend_class_is_internal(Some(backend_class)))
            })
    });
    is_internal
}

fn internal_host_app_bridge_requires_prelaunch_blocker(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_kind != "agent_lane" {
        return false;
    }
    let Ok(overlay) = load_project_overlay_yaml_for_root(project_root) else {
        return false;
    };
    let (selected_cli_system, selected_cli_entry) =
        selected_host_cli_system_for_runtime_dispatch(&overlay);
    let host_runtime = runtime_host_execution_contract_for_root(project_root);
    let host_execution_class = selected_cli_entry
        .as_ref()
        .and_then(|entry| yaml_string(yaml_lookup(entry, &["execution_class"])))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| json_string(host_runtime.get("selected_cli_execution_class")))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default();
    if host_execution_class != "internal" {
        return false;
    }
    if !selected_backend_is_internal_host_bridge(
        &overlay,
        &selected_cli_system,
        role_selection,
        receipt,
    ) {
        return false;
    }
    if selected_cli_entry
        .as_ref()
        .and_then(|entry| yaml_lookup(entry, &["dispatch"]))
        .is_none()
    {
        return false;
    }
    if internal_host_receipt_backed_completion_is_enabled(selected_cli_entry.as_ref()) {
        return false;
    }
    // A configured internal host command may be any carrier binary. Without an
    // explicit receipt-backed completion flag it is only an activation bridge,
    // so it must fail closed unless a route-admissible external CLI fallback can
    // execute the packet instead.
    let selected_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    if let Some(blocked_backend_id) = selected_backend
        .as_deref()
        .or(receipt.selected_backend.as_deref())
    {
        if internal_host_external_fallback_backend(
            role_selection,
            &receipt.dispatch_target,
            blocked_backend_id,
            &overlay,
        )
        .is_some()
        {
            return false;
        }
    }
    true
}

fn normalize_internal_host_timeout_result_blocker(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    execution_result: &mut serde_json::Value,
) {
    if json_string(execution_result.get("blocker_code")).as_deref()
        != Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
    {
        return;
    }
    if !dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        return;
    }
    execution_result["blocker_code"] =
        serde_json::Value::String(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT.to_string());
}

fn is_internal_activation_view_without_receipt_blocker(blocker_code: Option<&str>) -> bool {
    matches!(
        blocker_code,
        Some("internal_activation_view_only")
            | Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
            | Some(INTERNAL_CODEX_CARRIER_UNAVAILABLE)
    )
}

fn apply_dispatch_handoff_timeout_to_receipt(
    state_root: &Path,
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
) -> Result<(), String> {
    if dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        apply_internal_activation_timeout_to_receipt(
            state_root,
            project_root,
            role_selection,
            receipt,
            timeout_seconds,
        )
    } else {
        apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)
    }
}

pub(crate) fn runtime_dispatch_project_root_from_state_root<'a>(
    state_root: &'a Path,
) -> std::borrow::Cow<'a, Path> {
    if let Some(project_root) = crate::resolve_status_project_root(state_root) {
        return std::borrow::Cow::Owned(project_root);
    }
    std::borrow::Cow::Borrowed(state_root.parent().unwrap_or(state_root))
}

pub(crate) fn dispatch_handoff_timeout_seconds_for_state_root(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    let project_root = runtime_dispatch_project_root_from_state_root(state_root);
    dispatch_handoff_timeout_seconds(project_root.as_ref(), role_selection, receipt)
}

pub(crate) fn dispatch_handoff_uses_internal_host_for_state_root(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let project_root = runtime_dispatch_project_root_from_state_root(state_root);
    dispatch_handoff_uses_internal_host(project_root.as_ref(), role_selection, receipt)
}

pub(crate) fn apply_dispatch_handoff_timeout_to_receipt_for_state_root(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
) -> Result<(), String> {
    let project_root = runtime_dispatch_project_root_from_state_root(state_root);
    apply_dispatch_handoff_timeout_to_receipt(
        state_root,
        project_root.as_ref(),
        role_selection,
        receipt,
        timeout_seconds,
    )
}

fn downstream_preview_blockers_for_missing_lane_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    completion_blocker: &str,
) -> Vec<String> {
    if let Some(blocker_code) = receipt
        .blocker_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return vec![blocker_code.to_string()];
    }
    vec![completion_blocker.to_string()]
}

pub(crate) fn build_runtime_closure_admission(
    bundle_check: &TaskflowConsumeBundleCheck,
    docflow_verdict: &RuntimeConsumptionDocflowVerdict,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> RuntimeConsumptionClosureAdmission {
    let mut blockers = Vec::new();
    let mut evidence_table = Vec::new();
    if !bundle_check.ok {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::MissingClosureProof,
        ) {
            blockers.push(code);
        }
        blockers.extend(bundle_check.blockers.iter().cloned());
    }
    if !docflow_verdict.ready {
        blockers.extend(docflow_verdict.blockers.iter().cloned());
    }
    if !docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"))
    {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::MissingClosureProof,
        ) {
            blockers.push(code);
        }
    }
    let has_readiness_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("readiness-check"));
    let has_proof_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"));
    let mut bundle_blockers = Vec::new();
    if !bundle_check.ok {
        bundle_blockers.extend(bundle_check.blockers.iter().cloned());
    }
    evidence_table.push(RuntimeConsumptionClosureAdmissionEvidence {
        requirement: "taskflow_bundle_check".to_string(),
        status: if bundle_blockers.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        evidence_refs: vec![
            "vida taskflow consume bundle check".to_string(),
            format!("root_artifact_id={}", bundle_check.root_artifact_id),
        ],
        blockers: bundle_blockers,
    });
    let mut docflow_blockers = Vec::new();
    if !docflow_verdict.ready {
        docflow_blockers.extend(docflow_verdict.blockers.iter().cloned());
    }
    if !has_readiness_surface {
        docflow_blockers.push("missing_docflow_readiness_check".to_string());
    }
    if !has_proof_surface {
        docflow_blockers.push("missing_docflow_proofcheck".to_string());
    }
    docflow_blockers.sort();
    docflow_blockers.dedup();
    evidence_table.push(RuntimeConsumptionClosureAdmissionEvidence {
        requirement: "docflow_readiness".to_string(),
        status: if docflow_blockers.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        evidence_refs: docflow_verdict.proof_surfaces.clone(),
        blockers: docflow_blockers,
    });
    if !(has_readiness_surface && has_proof_surface) {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::RestoreReconcileNotGreen,
        ) {
            blockers.push(code);
        }
    }
    let design_first_flow = role_selection.execution_plan["status"] == "design_first";
    let design_packet_ready = !design_first_flow || tracked_design_doc_finalized(role_selection);
    if design_first_flow && !design_packet_ready {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::PendingDesignPacket,
        ) {
            blockers.push(code);
        }
    }
    if design_first_flow {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
        ) {
            blockers.push(code);
        }
    }
    let mut design_blockers = Vec::new();
    let mut handoff_blockers = Vec::new();
    if !design_packet_ready {
        design_blockers.push(
            crate::release_contract_adapters::blocker_code(
                crate::release1_contracts::BlockerCode::PendingDesignPacket,
            )
            .unwrap_or_else(|| "pending_design_packet".to_string()),
        );
    }
    if design_first_flow {
        handoff_blockers.push(
            crate::release_contract_adapters::blocker_code(
                crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
            )
            .unwrap_or_else(|| "pending_developer_handoff_packet".to_string()),
        );
    }
    evidence_table.push(RuntimeConsumptionClosureAdmissionEvidence {
        requirement: "approved_design_packet".to_string(),
        status: if design_blockers.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        evidence_refs: vec![role_selection.execution_plan["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string()],
        blockers: design_blockers,
    });
    evidence_table.push(RuntimeConsumptionClosureAdmissionEvidence {
        requirement: "spec_work_pool_dev_handoff".to_string(),
        status: if handoff_blockers.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        evidence_refs: vec![role_selection
            .tracked_flow_entry
            .clone()
            .unwrap_or_else(|| "untracked_flow".to_string())],
        blockers: handoff_blockers,
    });
    evidence_table.push(RuntimeConsumptionClosureAdmissionEvidence {
        requirement: "execution_preparation".to_string(),
        status: "pass".to_string(),
        evidence_refs: vec![role_selection.selected_role.clone()],
        blockers: Vec::new(),
    });
    blockers.sort();
    blockers.dedup();

    let mut proof_surfaces = vec!["vida taskflow consume bundle check".to_string()];
    proof_surfaces.extend(docflow_verdict.proof_surfaces.iter().cloned());

    RuntimeConsumptionClosureAdmission {
        status: if blockers.is_empty() {
            "admit".to_string()
        } else {
            "block".to_string()
        },
        admitted: blockers.is_empty(),
        blockers,
        proof_surfaces,
        evidence_table,
    }
}

pub(crate) fn build_taskflow_handoff_plan(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let execution_plan = &role_selection.execution_plan;
    let development_flow = &execution_plan["development_flow"];
    let dispatch_contract = &development_flow["dispatch_contract"];
    let lane_catalog = dispatch_contract["lane_catalog"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let activation_chain = lane_catalog
        .iter()
        .map(|(target, lane)| {
            (
                target.clone(),
                dispatch_contract_lane_activation(lane).clone(),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    if execution_plan["status"] == "design_first" {
        let execution_preparation_artifacts = taskflow_execution_preparation_artifacts(
            false,
            "blocked_pending_developer_handoff_packet",
        );
        return serde_json::json!({
            "status": "spec_first_handoff_required",
            "orchestration_contract": execution_plan["orchestration_contract"],
            "tracked_flow_bootstrap": execution_plan["tracked_flow_bootstrap"],
            "design_packet_activation": runtime_assignment_from_execution_plan(execution_plan),
            "design_packet_activation_source": runtime_assignment_source_from_execution_plan(execution_plan),
            "post_design_activation_chain": activation_chain,
            "post_design_lane_contract": lane_catalog,
            "handoff_ready": true,
            "execution_preparation_artifacts": execution_preparation_artifacts,
        });
    }

    let developer_handoff_status = execution_plan["pre_execution_design_gate"]
        ["developer_handoff_packet_status"]
        .as_str()
        .unwrap_or("blocked_pending_developer_handoff_packet");
    let execution_preparation_artifacts =
        taskflow_execution_preparation_artifacts(true, developer_handoff_status);
    serde_json::json!({
        "status": "execution_handoff_ready",
        "orchestration_contract": execution_plan["orchestration_contract"],
        "activation_chain": activation_chain,
        "lane_contract": lane_catalog,
        "runtime_assignment": runtime_assignment_from_execution_plan(execution_plan),
        "runtime_assignment_source": runtime_assignment_source_from_execution_plan(execution_plan),
        "lane_sequence": development_flow["lane_sequence"],
        "handoff_ready": true,
        "execution_preparation_artifacts": execution_preparation_artifacts,
    })
}

fn taskflow_execution_preparation_artifact(
    ready: bool,
    status: &str,
    path: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "ready": ready,
        "status": status,
        "path": path,
    })
}

fn taskflow_execution_preparation_artifacts(
    handoff_ready: bool,
    developer_handoff_status: &str,
) -> serde_json::Value {
    let blocked_prefix = if handoff_ready {
        "pending"
    } else {
        "blocked_pending"
    };
    serde_json::json!({
        "handoff_ready": handoff_ready,
        "required_artifacts": [
            "architecture_preparation_report",
            "developer_handoff_packet",
            "change_boundary",
            "dependency_impact_summary",
            "spec_alignment_summary",
        ],
        "architecture_preparation_report": taskflow_execution_preparation_artifact(
            false,
            &format!("{blocked_prefix}_architecture_preparation_report"),
            None,
        ),
        "developer_handoff_packet": taskflow_execution_preparation_artifact(
            false,
            developer_handoff_status,
            None,
        ),
        "change_boundary": taskflow_execution_preparation_artifact(
            false,
            &format!("{blocked_prefix}_change_boundary"),
            None,
        ),
        "dependency_impact_summary": taskflow_execution_preparation_artifact(
            false,
            &format!("{blocked_prefix}_dependency_impact_summary"),
            None,
        ),
        "spec_alignment_summary": taskflow_execution_preparation_artifact(
            false,
            &format!("{blocked_prefix}_spec_alignment_summary"),
            None,
        ),
        "execution_preparation_evidence": {
            "ready": false,
            "status": if handoff_ready {
                "pending_execution_preparation_evidence"
            } else {
                "blocked_pending_execution_preparation_evidence"
            },
        }
    })
}

pub(crate) fn runtime_consumption_run_id(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> String {
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["spec_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    let slug = infer_feature_request_slug(&role_selection.request);
    if slug.trim().is_empty() {
        "runtime-consume-request".to_string()
    } else {
        format!("runtime-{slug}")
    }
}

fn missing_agent_lane_dispatch_packet_error(dispatch_target: &str) -> String {
    let _ = blocker_code_str(BlockerCode::MissingPacket);
    format!("Agent lane dispatch for `{dispatch_target}` is missing dispatch_packet_path")
}

pub(crate) fn downstream_activation_fields(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" => (
            "taskflow_pack".to_string(),
            match dispatch_target {
                "spec-pack" => Some("vida taskflow bootstrap-spec".to_string()),
                "work-pool-pack" => Some("vida task ensure".to_string()),
                "dev-pack" => Some("vida task ensure".to_string()),
                _ => None,
            },
            None,
            None,
        ),
        "closure" => ("closure".to_string(), None, None, None),
        _ => {
            let lane = dispatch_contract_lane(&role_selection.execution_plan, dispatch_target);
            (
                "agent_lane".to_string(),
                Some("vida agent-init".to_string()),
                lane.and_then(|row| {
                    json_string(dispatch_contract_lane_activation(row).get("activation_agent_type"))
                }),
                lane.and_then(|row| {
                    json_string(
                        dispatch_contract_lane_activation(row).get("activation_runtime_role"),
                    )
                }),
            )
        }
    }
}

pub(crate) fn execution_plan_route_for_dispatch_target<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    let development_flow = &execution_plan["development_flow"];
    if dispatch_target == "analysis" {
        if let Some(route) = development_flow
            .get("analysis")
            .filter(|value| !value.is_null())
        {
            return Some(route);
        }
        return development_flow
            .get("implementation")
            .filter(|value| !value.is_null());
    }
    if let Some(canonical_target) =
        dispatch_target_for_runtime_role(execution_plan, dispatch_target)
            .filter(|target| target != dispatch_target)
    {
        if let Some(route) =
            execution_plan_route_for_dispatch_target(execution_plan, &canonical_target)
        {
            return Some(route);
        }
    }
    let canonical_route_key = match dispatch_target {
        "implementer" | "writer" => Some("implementation"),
        "execution_preparation" => Some("architecture"),
        _ => None,
    };
    if let Some(route_key) = canonical_route_key {
        if let Some(route) = development_flow
            .get(route_key)
            .filter(|value| !value.is_null())
        {
            return Some(route);
        }
    }
    if let Some(route) = development_flow
        .get(dispatch_target)
        .filter(|value| !value.is_null())
    {
        return Some(route);
    }
    dispatch_contract_lane(execution_plan, dispatch_target)
}

fn non_empty_assignment_string(assignment: &serde_json::Value, key: &str) -> bool {
    assignment
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

fn runtime_assignment_has_authoritative_truth(assignment: &serde_json::Value) -> bool {
    if !assignment
        .as_object()
        .is_some_and(|object| !object.is_empty())
    {
        return false;
    }
    if assignment
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        return true;
    }
    let has_backend_selector = [
        "selected_backend_id",
        "selected_carrier_id",
        "selected_agent_id",
        "selected_carrier_agent_id",
        "selected_tier",
        "activation_agent_type",
    ]
    .iter()
    .any(|key| non_empty_assignment_string(assignment, key));
    let has_model_selector = [
        "selected_model_profile_id",
        "selected_model_ref",
        "model_profile_id",
        "model_ref",
    ]
    .iter()
    .any(|key| non_empty_assignment_string(assignment, key));
    let has_legacy_tier_activation = non_empty_assignment_string(assignment, "selected_tier")
        && non_empty_assignment_string(assignment, "activation_agent_type");
    has_backend_selector && (has_model_selector || has_legacy_tier_activation)
}

fn authoritative_runtime_assignment_candidate(
    assignment: &serde_json::Value,
) -> Option<serde_json::Value> {
    runtime_assignment_has_authoritative_truth(assignment).then(|| assignment.clone())
}

fn legacy_dispatch_contract_activation_for_target<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<(&'a serde_json::Value, &'static str)> {
    let contract = &execution_plan["development_flow"]["dispatch_contract"];
    let activation_key = match dispatch_target {
        "specification" => "specification_activation",
        "implementer" | "implementation" => "implementer_activation",
        "coach" => "coach_activation",
        "verification" | "verifier" => "verifier_activation",
        "execution_preparation" | "escalation" => "escalation_activation",
        _ => return None,
    };
    contract
        .get(activation_key)
        .filter(|value| !value.is_null())
        .map(|value| {
            let source = match activation_key {
                "specification_activation" => "dispatch_contract_specification_activation",
                "implementer_activation" => "dispatch_contract_implementer_activation",
                "coach_activation" => "dispatch_contract_coach_activation",
                "verifier_activation" => "dispatch_contract_verifier_activation",
                "escalation_activation" => "dispatch_contract_escalation_activation",
                _ => "dispatch_contract_activation",
            };
            (value, source)
        })
}

pub(crate) fn dispatch_target_runtime_assignment(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> (serde_json::Value, &'static str) {
    if let Some((assignment, source)) =
        execution_plan_route_for_dispatch_target(execution_plan, dispatch_target).and_then(
            |route| {
                authoritative_runtime_assignment_candidate(runtime_assignment_from_route(route))
                    .map(|assignment| {
                        let source = if route.get("carrier_runtime_assignment").is_some() {
                            "route_carrier_runtime_assignment"
                        } else {
                            "route_runtime_assignment"
                        };
                        (assignment, source)
                    })
            },
        )
    {
        return (assignment, source);
    }

    if let Some((assignment, source)) =
        legacy_dispatch_contract_activation_for_target(execution_plan, dispatch_target).and_then(
            |(activation, source)| {
                authoritative_runtime_assignment_candidate(activation)
                    .map(|assignment| (assignment, source))
            },
        )
    {
        return (assignment, source);
    }

    if let Some((assignment, source)) = dispatch_contract_lane(execution_plan, dispatch_target)
        .map(dispatch_contract_lane_activation)
        .and_then(|activation| {
            authoritative_runtime_assignment_candidate(activation)
                .map(|assignment| (assignment, "dispatch_contract_lane_activation"))
        })
    {
        return (assignment, source);
    }

    if let Some(assignment) = authoritative_runtime_assignment_candidate(
        runtime_assignment_from_execution_plan(execution_plan),
    ) {
        return (
            assignment,
            runtime_assignment_source_from_execution_plan(execution_plan),
        );
    }

    (serde_json::Value::Null, "missing")
}

fn canonical_dispatch_target_for_backend_resolution(dispatch_target: &str) -> String {
    match canonical_dispatch_target_name(dispatch_target).as_str() {
        "implementer" | "writer" => "implementation".to_string(),
        "execution_preparation" => "architecture".to_string(),
        other => other.to_string(),
    }
}

fn dispatch_target_requires_strict_backend_admissibility(dispatch_target: &str) -> bool {
    matches!(
        canonical_dispatch_target_for_backend_resolution(dispatch_target).as_str(),
        "implementation" | "verification"
    )
}

pub(crate) fn backend_is_admissible_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
) -> bool {
    let canonical_target = canonical_dispatch_target_for_backend_resolution(dispatch_target);
    let strict_required = dispatch_target_requires_strict_backend_admissibility(dispatch_target);
    let Some(matrix) = execution_plan["backend_admissibility_matrix"].as_array() else {
        return !strict_required;
    };
    let Some(row) = matrix
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
    else {
        return !strict_required;
    };
    let Some(lane_admissibility) = row["lane_admissibility"].as_object() else {
        return !strict_required;
    };
    lane_admissibility
        .get(canonical_target.as_str())
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(!strict_required)
}

fn assignment_selects_backend(assignment: &serde_json::Value, backend_id: &str) -> bool {
    [
        "selected_backend_id",
        "selected_carrier_id",
        "selected_agent_id",
        "selected_carrier_agent_id",
        "selected_tier",
        "activation_agent_type",
    ]
    .iter()
    .filter_map(|key| json_string(assignment.get(*key)))
    .any(|value| value.trim() == backend_id)
}

fn assignment_selects_explicit_dispatch_backend(
    execution_plan: &serde_json::Value,
    assignment: &serde_json::Value,
    backend_id: &str,
) -> bool {
    let explicit_match = [
        "selected_backend_id",
        "selected_carrier_id",
        "selected_agent_id",
        "selected_carrier_agent_id",
    ]
    .iter()
    .filter_map(|key| json_string(assignment.get(*key)))
    .any(|value| value.trim() == backend_id);
    explicit_match
        && (runtime_assignment_has_authoritative_truth(assignment)
            || backend_has_execution_plan_dispatch_metadata(execution_plan, backend_id))
}

fn backend_has_execution_plan_dispatch_metadata(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> bool {
    backend_policy_from_execution_plan(execution_plan, backend_id).is_some()
        || backend_admissibility_row(execution_plan, backend_id).is_some_and(|row| {
            row["backend_class"]
                .as_str()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
                || row["lane_admissibility"].as_object().is_some()
        })
}

fn backend_admissibility_class<'a>(
    execution_plan: &'a serde_json::Value,
    backend_id: &str,
) -> Option<&'a str> {
    backend_admissibility_row(execution_plan, backend_id)
        .and_then(|row| row["backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn assignment_is_internal_host_carrier(
    execution_plan: &serde_json::Value,
    assignment: &serde_json::Value,
    backend_id: &str,
) -> bool {
    if backend_id.trim().is_empty() || !assignment_selects_backend(assignment, backend_id) {
        return false;
    }
    let backend_class = backend_admissibility_class(execution_plan, backend_id);
    if matches!(backend_class, Some("external_cli" | "external")) {
        return false;
    }
    if matches!(backend_class, Some("internal" | "internal_cli")) {
        return true;
    }
    if backend_class.is_none() {
        return false;
    }
    let provider = json_string(assignment.get("selected_model_provider"))
        .or_else(|| json_string(assignment.get("model_provider")))
        .unwrap_or_default();
    matches!(provider.as_str(), "openai" | "internal")
        || assignment.get("selected_model_profile_id").is_some()
}

pub(crate) fn backend_is_admissible_or_runtime_selected_carrier_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
) -> bool {
    if backend_is_admissible_for_dispatch_target(execution_plan, backend_id, dispatch_target) {
        return true;
    }
    let route_assignment_match =
        execution_plan_route_for_dispatch_target(execution_plan, dispatch_target)
            .map(runtime_assignment_from_route)
            .filter(|assignment| !assignment.is_null())
            .is_some_and(|assignment| {
                assignment_is_internal_host_carrier(execution_plan, assignment, backend_id)
                    || assignment_selects_explicit_dispatch_backend(
                        execution_plan,
                        assignment,
                        backend_id,
                    )
            });
    route_assignment_match || {
        let assignment = runtime_assignment_from_execution_plan(execution_plan);
        assignment_is_internal_host_carrier(execution_plan, assignment, backend_id)
            || assignment_selects_explicit_dispatch_backend(execution_plan, assignment, backend_id)
    }
}

#[cfg(test)]
fn route_selected_backend(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    selected_backend_from_execution_plan_route(execution_plan, route)
}

#[cfg(test)]
fn route_selected_backend_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> Option<String> {
    execution_plan_route_for_dispatch_target(execution_plan, dispatch_target)
        .and_then(|route| route_selected_backend(execution_plan, route))
}

fn route_has_backend_hints(execution_plan: &serde_json::Value, route: &serde_json::Value) -> bool {
    let _ = execution_plan;
    route_primary_backend_hint_from_route(route).is_some()
        || fallback_executor_backend_from_route(route).is_some()
        || !fanout_executor_backends_from_route(route).is_empty()
}

fn admissible_backend_candidates_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    route: &serde_json::Value,
    inherited_selected_backend: Option<&str>,
    activation_agent_type: Option<&str>,
) -> Vec<String> {
    let route_is_backend_agnostic = !route_has_backend_hints(execution_plan, route);
    let strict_required = dispatch_target_requires_strict_backend_admissibility(dispatch_target);
    let mut candidates = Vec::new();
    let inherited = inherited_selected_backend.map(str::to_string);
    let activation = activation_agent_type.map(str::to_string);
    let (target_assignment, target_assignment_source) =
        dispatch_target_runtime_assignment(execution_plan, dispatch_target);
    let target_assignment_is_route_scoped = matches!(
        target_assignment_source,
        "route_carrier_runtime_assignment" | "route_runtime_assignment"
    );
    let explicit_runtime_assignment_backend = runtime_assignment_selected_backend_for_target(
        execution_plan,
        dispatch_target,
    )
    .filter(|backend_id| {
        assignment_selects_explicit_dispatch_backend(execution_plan, &target_assignment, backend_id)
            || backend_has_execution_plan_dispatch_metadata(execution_plan, backend_id)
    });
    let has_explicit_runtime_assignment_backend =
        explicit_runtime_assignment_backend.is_some() && target_assignment_is_route_scoped;
    let runtime_assignment_backend = explicit_runtime_assignment_backend.or_else(|| {
        (!strict_required).then(|| runtime_assignment_backend_for_route(execution_plan, route))?
    });
    let route_primary = route_primary_backend_hint_from_route(route);
    let route_fallback = fallback_executor_backend_from_route(route);
    let route_fanout = fanout_executor_backends_from_route(route);
    let route_backends_have_dispatch_metadata = route_primary
        .iter()
        .chain(route_fallback.iter())
        .chain(route_fanout.iter())
        .any(|backend_id| {
            backend_policy_from_execution_plan(execution_plan, backend_id).is_some()
                || backend_has_execution_plan_dispatch_metadata(execution_plan, backend_id)
        });
    let prefer_route_backends_first = !route_is_backend_agnostic
        && (strict_required || route_backends_have_dispatch_metadata || inherited.is_none());
    if !strict_required && !prefer_route_backends_first {
        if let Some(inherited) = inherited.as_ref() {
            candidates.push(inherited.clone());
        }
    }
    if !strict_required && !prefer_route_backends_first {
        if let Some(runtime_assignment_backend) = runtime_assignment_backend.as_ref() {
            candidates.push(runtime_assignment_backend.clone());
        }
    }
    if prefer_route_backends_first {
        if !strict_required {
            if let Some(runtime_assignment_backend) = runtime_assignment_backend
                .as_ref()
                .filter(|_| has_explicit_runtime_assignment_backend)
            {
                candidates.push(runtime_assignment_backend.clone());
            }
        }
        if let Some(primary) = route_primary.as_ref() {
            candidates.push(primary.clone());
        }
        if strict_required {
            if let Some(runtime_assignment_backend) = runtime_assignment_backend.as_ref() {
                candidates.push(runtime_assignment_backend.clone());
            }
        }
        if let Some(fallback) = route_fallback.as_ref() {
            candidates.push(fallback.clone());
        }
        candidates.extend(route_fanout.iter().cloned());
    }
    if !strict_required && prefer_route_backends_first {
        if let Some(runtime_assignment_backend) = runtime_assignment_backend
            .as_ref()
            .filter(|_| !has_explicit_runtime_assignment_backend)
        {
            candidates.push(runtime_assignment_backend.clone());
        }
        if let Some(inherited) = inherited.as_ref() {
            candidates.push(inherited.clone());
        }
    }
    if strict_required {
        if let Some(activation) = activation.as_ref() {
            candidates.push(activation.clone());
        }
    }
    if strict_required {
        if let Some(runtime_assignment_backend) = runtime_assignment_backend.as_ref() {
            candidates.push(runtime_assignment_backend.clone());
        }
    }
    if strict_required {
        if let Some(inherited) = inherited {
            candidates.push(inherited);
        }
    }
    if !prefer_route_backends_first && !route_is_backend_agnostic {
        if let Some(primary) = route_primary.as_ref() {
            candidates.push(primary.clone());
        }
        if let Some(fallback) = route_fallback.as_ref() {
            candidates.push(fallback.clone());
        }
        candidates.extend(route_fanout.iter().cloned());
    }
    if let Some(activation) = activation.filter(|_| route_is_backend_agnostic && !strict_required) {
        candidates.push(activation);
    }
    let mut unique = std::collections::BTreeSet::new();
    candidates
        .into_iter()
        .filter(|candidate| !candidate.trim().is_empty())
        .filter(|candidate| unique.insert(candidate.clone()))
        .collect()
}

pub(crate) fn admissible_selected_backend_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    activation_agent_type: Option<&str>,
    inherited_selected_backend: Option<&str>,
) -> Option<String> {
    let strict_required = dispatch_target_requires_strict_backend_admissibility(dispatch_target);
    let route = execution_plan_route_for_dispatch_target(execution_plan, dispatch_target);
    let (candidates, route_is_backend_agnostic) = if let Some(route) = route {
        (
            admissible_backend_candidates_for_dispatch_target(
                execution_plan,
                dispatch_target,
                route,
                inherited_selected_backend,
                activation_agent_type,
            ),
            !route_has_backend_hints(execution_plan, route),
        )
    } else {
        let mut unique = std::collections::BTreeSet::new();
        (
            inherited_selected_backend
                .into_iter()
                .chain(activation_agent_type)
                .map(str::trim)
                .filter(|candidate| !candidate.is_empty())
                .filter(|candidate| unique.insert((*candidate).to_string()))
                .map(str::to_string)
                .collect::<Vec<_>>(),
            true,
        )
    };
    let route_activation_backend = route.and_then(activation_backend_from_route);
    if !strict_required {
        return candidates.into_iter().next();
    }
    let selected = candidates.into_iter().find(|candidate| {
        backend_is_admissible_or_runtime_selected_carrier_for_dispatch_target(
            execution_plan,
            candidate,
            dispatch_target,
        ) || route_activation_backend.as_deref() == Some(candidate.as_str())
            || (route_is_backend_agnostic
                && backend_class_from_execution_plan(execution_plan, candidate).as_deref()
                    == Some("internal"))
    });
    selected
        .or_else(|| admissible_matrix_backend_for_dispatch_target(execution_plan, dispatch_target))
}

fn admissible_matrix_backend_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> Option<String> {
    let matrix = execution_plan["backend_admissibility_matrix"].as_array()?;
    matrix
        .iter()
        .filter_map(|row| {
            row["backend_id"]
                .as_str()
                .map(|backend_id| (backend_id, row))
        })
        .filter(|(backend_id, _)| {
            backend_is_admissible_for_dispatch_target(execution_plan, backend_id, dispatch_target)
        })
        .max_by_key(|(_, row)| {
            (
                row["backend_class"].as_str() == Some("internal"),
                row["write_scope"].as_str() == Some("orchestrator_native"),
            )
        })
        .map(|(backend_id, _)| backend_id.to_string())
}

pub(crate) fn downstream_selected_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    activation_agent_type: Option<&str>,
    inherited_selected_backend: Option<&str>,
) -> Option<String> {
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" | "closure" => activation_agent_type
            .map(str::to_string)
            .or_else(|| inherited_selected_backend.map(str::to_string)),
        _ => admissible_selected_backend_for_dispatch_target(
            &role_selection.execution_plan,
            dispatch_target,
            activation_agent_type,
            inherited_selected_backend,
        ),
    }
}

fn backend_admissibility_row<'a>(
    execution_plan: &'a serde_json::Value,
    backend_id: &str,
) -> Option<&'a serde_json::Value> {
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["backend_id"].as_str() == Some(backend_id))
}

fn backend_class_for_execution_plan_backend(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> String {
    if backend_id.trim().is_empty() {
        return "unknown".to_string();
    }
    backend_admissibility_row(execution_plan, backend_id)
        .and_then(|row| row["backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if backend_id == "taskflow_state_store" {
                "taskflow_pack".to_string()
            } else {
                "unknown".to_string()
            }
        })
}

fn backend_execution_dimension(backend_class: &str) -> &'static str {
    match backend_class.trim() {
        "external_cli" | "external" => "external",
        "internal" | "internal_cli" => "internal",
        "taskflow_pack" => "taskflow_pack",
        _ => "unknown",
    }
}

pub(crate) fn effective_execution_posture_summary(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    selected_backend: Option<&str>,
    activation_agent_type: Option<&str>,
    host_runtime: Option<&serde_json::Value>,
    receipt_backed_execution_evidence: bool,
    selected_backend_override: Option<&str>,
) -> serde_json::Value {
    let route = execution_plan_route_for_dispatch_target(execution_plan, dispatch_target);
    let route_primary_backend = route.and_then(route_primary_backend_hint_from_route);
    let runtime_assignment_backend =
        route.and_then(|route| runtime_assignment_backend_for_route(execution_plan, route));
    let fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let normalized_selected_backend = selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let explicit_selected_backend_override = selected_backend_override
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let effective_selected_backend = explicit_selected_backend_override
        .map(str::to_string)
        .or_else(|| {
            admissible_selected_backend_for_dispatch_target(
                execution_plan,
                dispatch_target,
                activation_agent_type,
                normalized_selected_backend,
            )
        })
        .or_else(|| normalized_selected_backend.map(str::to_string));
    let selected_backend_source = backend_selection_source(
        effective_selected_backend.as_deref(),
        normalized_selected_backend,
        runtime_assignment_backend.as_deref(),
        route_primary_backend.as_deref(),
        fallback_backend.as_deref(),
        &fanout_backends,
        activation_agent_type,
        explicit_selected_backend_override,
    );
    let host_execution_class = host_runtime
        .and_then(|value| value.get("selected_cli_execution_class"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let selected_backend_class = effective_selected_backend.as_deref().map(|backend_id| {
        let resolved = backend_class_for_execution_plan_backend(execution_plan, backend_id);
        if resolved == "unknown"
            && host_execution_class == "internal"
            && activation_agent_type == Some(backend_id)
        {
            "internal".to_string()
        } else {
            resolved
        }
    });
    let route_primary_backend_class = route_primary_backend
        .as_deref()
        .map(|backend_id| backend_class_for_execution_plan_backend(execution_plan, backend_id));
    let fallback_backend_class = fallback_backend
        .as_deref()
        .map(|backend_id| backend_class_for_execution_plan_backend(execution_plan, backend_id));
    let fanout_backend_classes = fanout_backends
        .iter()
        .map(|backend_id| {
            (
                backend_id.clone(),
                serde_json::Value::String(backend_class_for_execution_plan_backend(
                    execution_plan,
                    backend_id,
                )),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();

    let route_dimensions = route_primary_backend_class
        .iter()
        .chain(fallback_backend_class.iter())
        .map(|value| backend_execution_dimension(value))
        .chain(
            fanout_backend_classes
                .values()
                .filter_map(serde_json::Value::as_str)
                .map(backend_execution_dimension),
        )
        .collect::<std::collections::BTreeSet<_>>();
    let route_contains_internal_backend = route_dimensions.contains("internal");
    let route_contains_external_backend = route_dimensions.contains("external");
    let mixed_route_backends = route_contains_internal_backend && route_contains_external_backend;

    let selected_execution_class = host_runtime
        .and_then(|value| value.get("selected_cli_execution_class"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_cli_system = host_runtime
        .and_then(|value| value.get("selected_cli_system"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let host_execution_dimension = host_execution_class;
    let selected_backend_dimension = selected_backend_class
        .as_deref()
        .map(backend_execution_dimension)
        .unwrap_or("unknown");
    let hybrid_host_backend_selection = matches!(
        (host_execution_dimension, selected_backend_dimension),
        ("internal", "external") | ("external", "internal")
    );
    let effective_posture_kind = if hybrid_host_backend_selection || mixed_route_backends {
        "mixed"
    } else if selected_backend_dimension == "external" || host_execution_dimension == "external" {
        "external"
    } else if selected_backend_dimension == "internal" || host_execution_dimension == "internal" {
        "internal"
    } else if selected_backend_dimension == "taskflow_pack" {
        "taskflow_pack"
    } else {
        "unknown"
    };

    serde_json::json!({
        "dispatch_target": dispatch_target,
        "selected_cli_system": selected_cli_system,
        "selected_execution_class": selected_execution_class,
        "selected_backend": effective_selected_backend,
        "selected_backend_source": selected_backend_source,
        "backend_selection_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "route_primary_backend": route_primary_backend,
        "route_primary_backend_class": route_primary_backend_class,
        "fallback_backend": fallback_backend,
        "fallback_backend_class": fallback_backend_class,
        "fanout_backends": fanout_backends,
        "fanout_backend_classes": fanout_backend_classes,
        "route_contains_internal_backend": route_contains_internal_backend,
        "route_contains_external_backend": route_contains_external_backend,
        "mixed_route_backends": mixed_route_backends,
        "hybrid_host_backend_selection": hybrid_host_backend_selection,
        "effective_posture_kind": effective_posture_kind,
        "activation_evidence_state": if receipt_backed_execution_evidence {
            "execution_evidence"
        } else {
            "activation_view_only"
        },
        "receipt_backed_execution_evidence": receipt_backed_execution_evidence,
    })
}

pub(crate) fn canonical_selected_backend_for_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    downstream_selected_backend(
        role_selection,
        &receipt.dispatch_target,
        receipt.activation_agent_type.as_deref(),
        None,
    )
}

pub(crate) fn sync_receipt_configured_activation_assignment(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) {
    if receipt.dispatch_kind != "agent_lane" {
        return;
    }
    let execution_plan = &role_selection.execution_plan;
    let canonical_target =
        dispatch_target_for_runtime_role(execution_plan, &receipt.dispatch_target)
            .unwrap_or_else(|| receipt.dispatch_target.clone());
    let lane_activation = dispatch_contract_lane(execution_plan, &canonical_target)
        .map(dispatch_contract_lane_activation);
    let (assignment, _) =
        dispatch_target_runtime_assignment(execution_plan, &receipt.dispatch_target);
    if receipt.activation_agent_type.is_none() {
        receipt.activation_agent_type = lane_activation
            .and_then(|activation| json_string(activation.get("activation_agent_type")))
            .or_else(|| json_string(assignment.get("activation_agent_type")))
            .or_else(|| json_string(assignment.get("selected_tier")))
            .or_else(|| json_string(assignment.get("selected_carrier_id")));
    }
    if receipt.activation_runtime_role.is_none() {
        receipt.activation_runtime_role = lane_activation
            .and_then(|activation| json_string(activation.get("activation_runtime_role")))
            .or_else(|| json_string(assignment.get("activation_runtime_role")))
            .or_else(|| json_string(assignment.get("runtime_role")));
    }
    let selected_backend = canonical_selected_backend_for_receipt(role_selection, receipt)
        .or_else(|| json_string(assignment.get("selected_backend_id")))
        .or_else(|| json_string(assignment.get("selected_carrier_id")));
    if let Some(selected_backend) = selected_backend {
        if receipt
            .selected_backend
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "unknown")
            .is_none()
        {
            receipt.selected_backend = Some(selected_backend);
        }
    }
}

fn persisted_selected_backend_override_for_packet_path(packet_path: &str) -> Option<String> {
    crate::read_json_file_if_present(Path::new(packet_path)).and_then(|packet| {
        packet
            .get("selected_backend_override")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn runtime_assignment_selected_backend_for_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> Option<String> {
    let (assignment, _) = dispatch_target_runtime_assignment(execution_plan, dispatch_target);
    json_string(assignment.get("selected_backend_id"))
        .or_else(|| json_string(assignment.get("selected_carrier_id")))
}

fn selected_backend_override_conflicts_with_runtime_assignment(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    selected_backend_override: &str,
) -> bool {
    let Some(assignment_backend) = runtime_assignment_selected_backend_for_target(
        &role_selection.execution_plan,
        dispatch_target,
    ) else {
        return false;
    };
    if assignment_backend == selected_backend_override {
        return false;
    }
    let override_backend_class = backend_class_for_execution_plan_backend(
        &role_selection.execution_plan,
        selected_backend_override,
    );
    matches!(
        backend_execution_dimension(&override_backend_class),
        "internal"
    ) || selected_backend_override == "internal_subagents"
}

fn current_selected_backend_override<'a>(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    selected_backend_override: Option<&'a str>,
) -> Option<&'a str> {
    selected_backend_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| {
            !selected_backend_override_conflicts_with_runtime_assignment(
                role_selection,
                dispatch_target,
                value,
            )
        })
}

pub(crate) fn preferred_selected_backend_for_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    let selected_backend_override = receipt
        .dispatch_packet_path
        .as_deref()
        .and_then(persisted_selected_backend_override_for_packet_path)
        .and_then(|value| {
            current_selected_backend_override(
                role_selection,
                &receipt.dispatch_target,
                Some(value.as_str()),
            )
            .map(str::to_string)
        });
    selected_backend_override
        .or_else(|| canonical_selected_backend_for_receipt(role_selection, receipt))
        .or_else(|| {
            runtime_assignment_selected_backend_for_target(
                &role_selection.execution_plan,
                &receipt.dispatch_target,
            )
        })
        .or_else(|| receipt.selected_backend.clone())
}

fn preferred_selected_model_profile_for_role_selection(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Option<&str> {
    role_selection
        .execution_plan
        .get("runtime_assignment")
        .and_then(|value| value.get("selected_model_profile_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn route_selected_model_profile_for_backend(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    backend_id: &str,
) -> Option<String> {
    let backend_id = backend_id.trim();
    if backend_id.is_empty() {
        return None;
    }
    if dispatch_target == "analysis" {
        if let Some(profile) = execution_plan["development_flow"]
            .get("analysis")
            .filter(|value| !value.is_null())
            .and_then(|route| route.get("profiles"))
            .and_then(|profiles| {
                profiles
                    .get(backend_id)
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        profiles
                            .as_str()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                    })
            })
        {
            return Some(profile);
        }
    }
    let route = execution_plan_route_for_dispatch_target(execution_plan, dispatch_target)?;
    let profiles = route.get("profiles")?;
    profiles
        .get(backend_id)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            profiles
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

pub(crate) fn preferred_selected_model_profile_for_dispatch_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    selected_backend: Option<&str>,
) -> Option<String> {
    selected_backend
        .and_then(|backend_id| {
            route_selected_model_profile_for_backend(
                &role_selection.execution_plan,
                dispatch_target,
                backend_id,
            )
        })
        .or_else(|| {
            preferred_selected_model_profile_for_role_selection(role_selection).map(str::to_string)
        })
}

fn backend_policy_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
    backend_id: &str,
) -> Option<&'a serde_json::Value> {
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
}

fn backend_class_from_execution_plan(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> Option<String> {
    backend_policy_from_execution_plan(execution_plan, backend_id)
        .and_then(|entry| entry["backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn backend_class_is_internal(backend_class: Option<&str>) -> bool {
    backend_class.is_some_and(|value| matches!(value.trim(), "internal" | "internal_cli"))
}

fn route_execution_posture_from_classes(classes: &[String]) -> &'static str {
    let has_internal = classes.iter().any(|value| value == "internal");
    let has_external = classes.iter().any(|value| value == "external_cli");
    match (has_internal, has_external) {
        (true, true) => "hybrid",
        (true, false) => "internal_only",
        (false, true) => "external_only",
        _ => "unknown",
    }
}

pub(crate) fn dispatch_execution_route_summary(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    selected_backend: Option<&str>,
    selected_backend_override: Option<&str>,
) -> serde_json::Value {
    let route =
        execution_plan_route_for_dispatch_target(&role_selection.execution_plan, dispatch_target);
    let route_primary_backend = route.and_then(route_primary_backend_hint_from_route);
    let runtime_assignment_backend = route.and_then(|route| {
        runtime_assignment_backend_for_route(&role_selection.execution_plan, route)
    });
    let route_fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let route_fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let normalized_selected_backend = selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let explicit_selected_backend_override = selected_backend_override
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let effective_selected_backend = explicit_selected_backend_override
        .map(str::to_string)
        .or_else(|| {
            admissible_selected_backend_for_dispatch_target(
                &role_selection.execution_plan,
                dispatch_target,
                None,
                normalized_selected_backend,
            )
        });
    let selected_backend_source = backend_selection_source(
        effective_selected_backend.as_deref(),
        normalized_selected_backend,
        runtime_assignment_backend.as_deref(),
        route_primary_backend.as_deref(),
        route_fallback_backend.as_deref(),
        &route_fanout_backends,
        None,
        explicit_selected_backend_override,
    );

    let mut execution_classes = Vec::new();
    for backend_id in effective_selected_backend
        .iter()
        .chain(route_primary_backend.iter())
        .chain(route_fallback_backend.iter())
        .chain(route_fanout_backends.iter())
    {
        if let Some(class) =
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
        {
            if !execution_classes.iter().any(|existing| existing == &class) {
                execution_classes.push(class);
            }
        }
    }
    let effective_execution_posture = route_execution_posture_from_classes(&execution_classes);
    let selected_backend_class = effective_selected_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
        });
    let route_primary_backend_class = route_primary_backend.as_deref().and_then(|backend_id| {
        backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
    });
    let route_fallback_backend_class = route_fallback_backend.as_deref().and_then(|backend_id| {
        backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
    });
    let route_fanout_backend_classes = route_fanout_backends
        .iter()
        .map(|backend_id| {
            (
                backend_id.clone(),
                serde_json::json!(backend_class_from_execution_plan(
                    &role_selection.execution_plan,
                    backend_id
                )),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let selected_backend_policy = effective_selected_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_primary_backend_policy = route_primary_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fallback_backend_policy = route_fallback_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fanout_backend_policies = route_fanout_backends
        .iter()
        .map(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "backend_id": backend_id }))
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "dispatch_target": dispatch_target,
        "effective_selected_backend": effective_selected_backend,
        "selected_backend_source": selected_backend_source,
        "backend_selection_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "route_primary_backend": route_primary_backend,
        "route_primary_backend_class": route_primary_backend_class,
        "route_fallback_backend": route_fallback_backend,
        "route_fallback_backend_class": route_fallback_backend_class,
        "route_fanout_backends": route_fanout_backends,
        "route_fanout_backend_classes": route_fanout_backend_classes,
        "selected_backend_policy": selected_backend_policy,
        "route_primary_backend_policy": route_primary_backend_policy,
        "route_fallback_backend_policy": route_fallback_backend_policy,
        "route_fanout_backend_policies": route_fanout_backend_policies,
        "selected_execution_class": selected_backend_class,
        "effective_execution_posture": effective_execution_posture,
        "mixed_posture": effective_execution_posture == "hybrid",
    })
}

fn activation_kind_from_dispatch_result_path(path: &str) -> Option<&'static str> {
    let result = crate::read_json_file_if_present(Path::new(path))?;
    dispatch_result_activation_kind(&result)
}

fn dispatch_result_activation_kind(result: &serde_json::Value) -> Option<&'static str> {
    if result["activation_vs_execution_evidence"]["evidence_state"].as_str()
        == Some("activation_view_only")
        || result["activation_semantics"]["view_only"].as_bool() == Some(true)
        || result["activation_semantics"]["activation_kind"].as_str() == Some("activation_view")
    {
        return Some("activation_view");
    }
    if result["activation_vs_execution_evidence"]["evidence_state"].as_str()
        == Some("execution_evidence_recorded")
        || result["activation_semantics"]["activation_kind"].as_str() == Some("execution_evidence")
        || result["execution_evidence"]["status"].as_str() == Some("recorded")
        || result["artifact_kind"].as_str() == Some("runtime_lane_completion_result")
    {
        return Some("execution_evidence");
    }
    if result["artifact_kind"].as_str() == Some("runtime_dispatch_result")
        || matches!(
            result["execution_state"].as_str(),
            Some("blocked" | "executing")
        )
    {
        return Some("activation_view");
    }
    None
}

fn receipt_result_path_has_execution_evidence(path: Option<&str>) -> bool {
    path.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(activation_kind_from_dispatch_result_path)
        == Some("execution_evidence")
}

fn receipt_result_path_activation_kind(path: Option<&str>) -> Option<&'static str> {
    path.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(activation_kind_from_dispatch_result_path)
}

fn canonical_activation_view_only_blocker_code(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> String {
    json_string(result.get("blocker_code"))
        .or_else(|| receipt.blocker_code.clone())
        .unwrap_or_else(|| "internal_activation_view_only".to_string())
}

pub(crate) fn normalize_activation_view_only_receipt_truth(
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if !matches!(
        receipt.dispatch_status.as_str(),
        "executed" | "packet_ready" | "executing"
    ) {
        return Ok(false);
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return Ok(false);
    };
    if dispatch_result_activation_kind(&result) != Some("activation_view") {
        return Ok(false);
    }
    if receipt.dispatch_status == "executing"
        && result["execution_state"].as_str() == Some("executing")
    {
        return Ok(false);
    }

    receipt.dispatch_status = "blocked".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = Some(canonical_activation_view_only_blocker_code(
        receipt, &result,
    ));
    receipt.downstream_dispatch_target = None;
    receipt.downstream_dispatch_command = None;
    receipt.downstream_dispatch_note = None;
    receipt.downstream_dispatch_ready = false;
    receipt.downstream_dispatch_blockers.clear();
    receipt.downstream_dispatch_packet_path = None;
    receipt.downstream_dispatch_status = None;
    receipt.downstream_dispatch_result_path = None;
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_active_target = Some(receipt.dispatch_target.clone());
    receipt.downstream_dispatch_last_target = Some(receipt.dispatch_target.clone());
    Ok(true)
}

fn resolve_project_artifact_path(
    project_root: &Path,
    raw_path: Option<&str>,
) -> Option<std::path::PathBuf> {
    let raw_path = raw_path.map(str::trim).filter(|value| !value.is_empty())?;
    let path = Path::new(raw_path);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    })
}

pub(crate) fn dispatch_activation_evidence_summary(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> serde_json::Value {
    let dispatch_result_path = nonempty_result_path(receipt.dispatch_result_path.as_deref());
    let downstream_result_path =
        nonempty_result_path(receipt.downstream_dispatch_result_path.as_deref());
    let evidence_path = dispatch_result_path
        .as_deref()
        .and_then(activation_kind_from_dispatch_result_path)
        .filter(|kind| *kind == "execution_evidence")
        .map(|_| {
            dispatch_result_path
                .clone()
                .expect("dispatch_result_path should exist")
        })
        .or_else(|| {
            downstream_result_path
                .as_deref()
                .and_then(activation_kind_from_dispatch_result_path)
                .filter(|kind| *kind == "execution_evidence")
                .map(|_| {
                    downstream_result_path
                        .clone()
                        .expect("downstream_result_path should exist")
                })
        });
    let activation_kind = if evidence_path.is_some() {
        "execution_evidence"
    } else {
        "activation_view"
    };
    let result_body = evidence_path
        .as_deref()
        .and_then(|path| crate::read_json_file_if_present(Path::new(path)));
    let activation_semantics = result_body
        .as_ref()
        .and_then(|value| value.get("activation_semantics"))
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "activation_kind": activation_kind,
                "view_only": activation_kind != "execution_evidence",
                "executes_packet": activation_kind == "execution_evidence",
                "records_completion_receipt": activation_kind == "execution_evidence",
            })
        });
    let execution_evidence = result_body
        .as_ref()
        .and_then(|value| value.get("execution_evidence"))
        .cloned()
        .unwrap_or_else(|| {
            if activation_kind == "execution_evidence" {
                serde_json::json!({
                    "status": "recorded",
                    "receipt_backed": true,
                    "result_path": evidence_path.clone(),
                })
            } else {
                serde_json::Value::Null
            }
        });

    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "execution_evidence_path": evidence_path,
        "receipt_backed": activation_kind == "execution_evidence",
        "activation_semantics": activation_semantics,
        "execution_evidence": execution_evidence,
    })
}

fn activation_evidence_from_result_body(result: &serde_json::Value) -> serde_json::Value {
    let activation_kind = result["activation_semantics"]["activation_kind"]
        .as_str()
        .or_else(|| {
            if result["execution_evidence"]["status"].as_str() == Some("recorded")
                || result["execution_state"].as_str() == Some("executed")
            {
                Some("execution_evidence")
            } else if result["artifact_kind"].as_str() == Some("runtime_dispatch_result")
                || result["execution_state"].as_str() == Some("blocked")
            {
                Some("activation_view")
            } else {
                None
            }
        })
        .unwrap_or("activation_view");
    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "activation_semantics": result["activation_semantics"].clone(),
        "execution_evidence": result["execution_evidence"].clone(),
        "receipt_backed": activation_kind == "execution_evidence",
    })
}

fn activation_evidence_from_receipt_result_paths(
    project_root: &Path,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<serde_json::Value> {
    for raw_path in [
        dispatch_receipt.dispatch_result_path.as_deref(),
        dispatch_receipt.downstream_dispatch_result_path.as_deref(),
    ] {
        let Some(resolved) = resolve_project_artifact_path(project_root, raw_path) else {
            continue;
        };
        let Some(result) = crate::read_json_file_if_present(&resolved) else {
            continue;
        };
        return Some(activation_evidence_from_result_body(&result));
    }
    None
}

pub(crate) fn dispatch_surface_truth_from_packet_path(
    project_root: &Path,
    packet_path: Option<&str>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<serde_json::Value> {
    let packet_path = resolve_project_artifact_path(project_root, packet_path)?;
    let packet = crate::read_json_file_if_present(&packet_path)?;
    let mut mixed_posture = packet
        .get("mixed_posture")
        .cloned()
        .or_else(|| packet.get("effective_execution_posture").cloned())
        .or_else(|| packet.get("execution_truth").cloned());
    if let Some(object) = mixed_posture
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        if object.get("effective_posture_kind").is_none() {
            if let Some(value) = object
                .get("effective_execution_posture")
                .cloned()
                .or_else(|| object.get("effective_posture_kind").cloned())
            {
                object.insert("effective_posture_kind".to_string(), value);
            }
        }
        if object.get("selected_backend").is_none() {
            if let Some(value) = object
                .get("effective_selected_backend")
                .cloned()
                .or_else(|| object.get("selected_backend").cloned())
            {
                object.insert("selected_backend".to_string(), value);
            }
        }
        if object.get("fanout_backends").is_none() {
            if let Some(value) = object
                .get("fanout_backends")
                .cloned()
                .or_else(|| object.get("fanout_executor_backends").cloned())
            {
                object.insert("fanout_backends".to_string(), value);
            }
        }
        if object.get("fallback_backend").is_none() {
            if let Some(value) = object
                .get("fallback_backend")
                .cloned()
                .or_else(|| object.get("fallback_executor_backend").cloned())
                .or_else(|| object.get("route_fallback_backend").cloned())
            {
                object.insert("fallback_backend".to_string(), value);
            }
        }
    }
    let activation_evidence =
        activation_evidence_from_receipt_result_paths(project_root, dispatch_receipt)
            .or_else(|| packet.get("activation_vs_execution_evidence").cloned())
            .or_else(|| packet.get("activation_evidence").cloned());
    Some(serde_json::json!({
        "mixed_posture": mixed_posture.unwrap_or(serde_json::Value::Null),
        "activation_vs_execution_evidence": activation_evidence.unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) fn fallback_backend_for_blocked_primary_dispatch_receipt(
    project_root: &std::path::Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return None;
    }
    let route = execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )?;
    let primary_backend =
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, route)?;
    let fallback_backend = fallback_executor_backend_from_route(route)?;
    if primary_backend == fallback_backend {
        return None;
    }
    let selected_backend = canonical_selected_backend_for_receipt(role_selection, dispatch_receipt)
        .or_else(|| dispatch_receipt.selected_backend.clone())?;
    if selected_backend != primary_backend {
        return None;
    }
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let (selected_cli_system, selected_cli_entry) =
        selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preflight = crate::status_surface_external_cli::external_cli_preflight_summary(
        &overlay,
        &selected_cli_system,
        selected_cli_entry.as_ref(),
    );
    let carrier_blocked = preflight["carrier_readiness"]["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|carrier| {
            carrier["backend_id"].as_str() == Some(primary_backend.as_str())
                && carrier["blocked"].as_bool() == Some(true)
        });
    let previous_primary_result_blocked =
        dispatch_result_blocks_primary_backend(dispatch_receipt, &primary_backend);
    (carrier_blocked || previous_primary_result_blocked).then_some(fallback_backend)
}

fn dispatch_result_blocks_primary_backend(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    primary_backend: &str,
) -> bool {
    let Some(result_path) = dispatch_receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return false;
    };
    let Ok(contents) = std::fs::read_to_string(result_path) else {
        return false;
    };
    let Ok(result) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return false;
    };
    let selected_backend = result["selected_backend"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(dispatch_receipt.selected_backend.as_deref())
        .unwrap_or_default();
    if selected_backend != primary_backend {
        return false;
    }
    let status_blocked = result["status"].as_str().map(str::trim) == Some("blocked");
    let blocker_present = result["blocker_code"]
        .as_str()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    status_blocked && blocker_present
}

pub(crate) fn build_downstream_dispatch_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<crate::state_store::RunGraphDispatchReceipt> {
    let dispatch_target = receipt.downstream_dispatch_target.clone()?;
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(role_selection, &dispatch_target);
    let selected_backend = downstream_selected_backend(
        role_selection,
        &dispatch_target,
        activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    )
    .filter(|value| !value.is_empty());
    let dispatch_status = if receipt.downstream_dispatch_ready {
        "routed".to_string()
    } else {
        "blocked".to_string()
    };
    // Downstream lanes must derive their truth from their own receipt state rather than
    // inheriting exception/supersession evidence that belongs to the upstream lane.
    let supersedes_receipt_id = None;
    let exception_path_receipt_id = None;
    Some(crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: dispatch_status.clone(),
        supersedes_receipt_id: supersedes_receipt_id.clone(),
        exception_path_receipt_id: exception_path_receipt_id.clone(),
        lane_status: derive_lane_status(
            &dispatch_status,
            supersedes_receipt_id.as_deref(),
            exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: receipt.downstream_dispatch_command.clone(),
        dispatch_packet_path: receipt.downstream_dispatch_packet_path.clone(),
        dispatch_result_path: None,
        blocker_code: if dispatch_status == "blocked" && receipt.dispatch_status != "executed" {
            blocker_code_value(BlockerCode::MissingLaneReceipt)
        } else if dispatch_status == "blocked" && receipt.downstream_dispatch_packet_path.is_none()
        {
            blocker_code_value(BlockerCode::MissingPacket)
        } else {
            None
        },
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
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    })
}

fn root_receipt_fields_from_downstream_step(
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    step_receipt: &crate::state_store::RunGraphDispatchReceipt,
) {
    root_receipt.downstream_dispatch_target = step_receipt.downstream_dispatch_target.clone();
    root_receipt.downstream_dispatch_command = step_receipt.downstream_dispatch_command.clone();
    root_receipt.downstream_dispatch_note = step_receipt.downstream_dispatch_note.clone();
    root_receipt.downstream_dispatch_ready = step_receipt.downstream_dispatch_ready;
    root_receipt.downstream_dispatch_blockers = step_receipt.downstream_dispatch_blockers.clone();
    root_receipt.downstream_dispatch_packet_path =
        step_receipt.downstream_dispatch_packet_path.clone();
    root_receipt.downstream_dispatch_status = step_receipt.downstream_dispatch_status.clone();
    root_receipt.downstream_dispatch_result_path =
        step_receipt.downstream_dispatch_result_path.clone();
    root_receipt.downstream_dispatch_active_target =
        step_receipt.downstream_dispatch_active_target.clone();
    root_receipt.supersedes_receipt_id = if same_evidence_id(
        root_receipt.supersedes_receipt_id.as_deref(),
        step_receipt.supersedes_receipt_id.as_deref(),
    ) {
        None
    } else {
        step_receipt.supersedes_receipt_id.clone()
    };
    root_receipt.exception_path_receipt_id = if same_evidence_id(
        root_receipt.exception_path_receipt_id.as_deref(),
        step_receipt.exception_path_receipt_id.as_deref(),
    ) {
        None
    } else {
        step_receipt.exception_path_receipt_id.clone()
    };
    root_receipt.blocker_code = step_receipt.blocker_code.clone();
}

fn same_evidence_id(lhs: Option<&str>, rhs: Option<&str>) -> bool {
    let lhs = lhs.map(str::trim).filter(|value| !value.is_empty());
    let rhs = rhs.map(str::trim).filter(|value| !value.is_empty());
    lhs.is_some() && lhs == rhs
}

pub(crate) fn active_downstream_dispatch_target(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if receipt.dispatch_kind == "agent_lane" && receipt.dispatch_status != "executed" {
        Some(receipt.dispatch_target.clone())
    } else {
        None
    }
}

fn agent_init_packet_flag_for_path(packet_path: &str) -> &'static str {
    if packet_path.contains("/downstream-dispatch-packets/")
        || packet_path.contains("downstream-dispatch-packets")
    {
        "--downstream-packet"
    } else {
        "--dispatch-packet"
    }
}

pub(crate) fn agent_init_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --json",
        agent_init_packet_flag_for_path(packet_path),
        shell_quote(packet_path)
    )
}

pub(crate) fn agent_init_execute_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --execute-dispatch --json",
        agent_init_packet_flag_for_path(packet_path),
        shell_quote(packet_path)
    )
}

pub(crate) fn runtime_host_execution_contract_for_root(project_root: &Path) -> serde_json::Value {
    let project_activation_view =
        project_activator_surface::build_project_activator_view(project_root);
    let host_environment = &project_activation_view["host_environment"];
    serde_json::json!({
        "selected_cli_system": host_environment["selected_cli_system"],
        "selected_cli_execution_class": host_environment["selected_cli_execution_class"],
        "runtime_template_root": host_environment["runtime_template_root"],
        "template_materialized": host_environment["template_materialized"],
    })
}

pub(crate) fn runtime_host_execution_contract_allows_automatic_dispatch_execution(
    project_root: &Path,
) -> bool {
    runtime_host_execution_contract_for_root(project_root)["selected_cli_execution_class"].as_str()
        == Some("internal")
}

pub(crate) fn load_project_overlay_yaml_for_root(
    project_root: &Path,
) -> Result<serde_yaml::Value, String> {
    let path = project_root.join("vida.config.yaml");
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) type ModelProfileCatalog = BTreeMap<String, BTreeSet<String>>;

fn collect_model_profiles_from_yaml(value: &serde_yaml::Value, profiles: &mut ModelProfileCatalog) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            if let Some(model_profiles) =
                mapping.get(serde_yaml::Value::String("model_profiles".to_string()))
            {
                if let serde_yaml::Value::Mapping(profile_mapping) = model_profiles {
                    for (profile_id, profile_value) in profile_mapping {
                        let Some(profile_id) = profile_id.as_str().map(str::trim) else {
                            continue;
                        };
                        if profile_id.is_empty() {
                            continue;
                        }
                        let model_ref = yaml_lookup(profile_value, &["model_ref"])
                            .and_then(serde_yaml::Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .unwrap_or(profile_id);
                        profiles
                            .entry(profile_id.to_string())
                            .or_default()
                            .insert(model_ref.to_string());
                    }
                }
            }
            for child in mapping.values() {
                collect_model_profiles_from_yaml(child, profiles);
            }
        }
        serde_yaml::Value::Sequence(values) => {
            for child in values {
                collect_model_profiles_from_yaml(child, profiles);
            }
        }
        _ => {}
    }
}

pub(crate) fn model_profile_catalog_from_overlay(
    overlay: &serde_yaml::Value,
) -> ModelProfileCatalog {
    let mut profiles = BTreeMap::new();
    collect_model_profiles_from_yaml(overlay, &mut profiles);
    profiles
}

pub(crate) fn current_project_model_profile_catalog_for_root(
    project_root: &Path,
) -> ModelProfileCatalog {
    let effective_project_root = if project_root == crate::state_store::repo_root() {
        crate::resolve_repo_root()
            .ok()
            .filter(|resolved| resolved != project_root)
            .filter(|resolved| resolved.join("vida.config.yaml").is_file())
            .unwrap_or_else(|| project_root.to_path_buf())
    } else {
        project_root.to_path_buf()
    };

    load_project_overlay_yaml_for_root(&effective_project_root)
        .map(|overlay| model_profile_catalog_from_overlay(&overlay))
        .unwrap_or_default()
}

pub(crate) fn route_assignment_catalog_drift_payload(
    route: &serde_json::Value,
    catalog: &ModelProfileCatalog,
) -> Option<serde_json::Value> {
    if catalog.is_empty() {
        return None;
    }
    let selected_profile = route["selected_model_profile_id"].as_str()?.trim();
    if selected_profile.is_empty() {
        return None;
    }
    let selected_model_ref = route["selected_model_ref"].as_str().map(str::trim);
    let Some(current_model_refs) = catalog.get(selected_profile) else {
        return Some(serde_json::json!({
            "status": "blocked",
            "reason": "selected_model_profile_not_in_current_config",
            "selected_model_profile_id": selected_profile,
            "selected_model_ref": selected_model_ref,
            "current_model_refs": serde_json::Value::Null,
        }));
    };
    if let Some(selected_model_ref) = selected_model_ref {
        if !selected_model_ref.is_empty() && !current_model_refs.contains(selected_model_ref) {
            return Some(serde_json::json!({
                "status": "blocked",
                "reason": "selected_model_ref_mismatch_current_config",
                "selected_model_profile_id": selected_profile,
                "selected_model_ref": selected_model_ref,
                "current_model_refs": current_model_refs,
            }));
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeAgentLaneDispatch {
    pub(crate) surface: String,
    pub(crate) activation_command: String,
    pub(crate) backend_dispatch: serde_json::Value,
}

pub(crate) fn selected_host_cli_system_for_runtime_dispatch(
    overlay: &serde_yaml::Value,
) -> (String, Option<serde_yaml::Value>) {
    let registry = project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay));
    let candidate = yaml_lookup(overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "__HOST_CLI_SYSTEM__")
        .and_then(project_activator_surface::normalize_host_cli_system);
    let selected = candidate.unwrap_or_else(|| {
        let mut supported = registry
            .iter()
            .filter(|(_, entry)| yaml_bool(yaml_lookup(entry, &["enabled"]), true))
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
    let entry = registry.get(&selected).cloned();
    (selected, entry)
}

pub(crate) fn configured_dispatch_backend_class(
    overlay: &serde_yaml::Value,
    system: &str,
) -> String {
    project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay))
        .get(system)
        .and_then(|entry| {
            yaml_string(yaml_lookup(entry, &["dispatch_backend_class"]))
                .or_else(|| yaml_string(yaml_lookup(entry, &["backend_class"])))
        })
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "external_cli".to_string())
}

fn configured_subagent_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    let entry = configured_subagent_entry_any(overlay, backend_id)?;
    yaml_bool(yaml_lookup(entry, &["enabled"]), false).then_some(entry)
}

fn configured_subagent_entry_any<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|entries| {
            entries.iter().find_map(|(key, value)| {
                let id = key.as_str()?.trim();
                if id == backend_id {
                    Some(value)
                } else {
                    None
                }
            })
        })
}

fn configured_internal_host_carrier_exists(
    overlay: Option<&serde_yaml::Value>,
    system: &str,
    backend_id: &str,
) -> bool {
    let registry = project_activator_surface::host_cli_system_registry_with_fallback(overlay);
    let Some(system_entry) = registry.get(system) else {
        return false;
    };
    let carriers =
        crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(Some(system_entry));
    carriers
        .iter()
        .any(|row| row["role_id"].as_str() == Some(backend_id))
        || overlay
            .and_then(|overlay| configured_subagent_entry(overlay, backend_id))
            .and_then(|entry| yaml_string(yaml_lookup(entry, &["subagent_backend_class"])))
            .as_deref()
            .is_some_and(|backend_class| backend_class_is_internal(Some(backend_class)))
}

pub(crate) fn configured_external_backend_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    let entry = configured_subagent_entry(overlay, backend_id)?;
    (yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref()
        == Some("external_cli"))
    .then_some(entry)
}

pub(crate) fn configured_external_backend_entry_any<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    let entry = configured_subagent_entry_any(overlay, backend_id)?;
    (yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref()
        == Some("external_cli"))
    .then_some(entry)
}

pub(crate) fn selected_external_backend_for_system(
    overlay: &serde_yaml::Value,
    system: &str,
    preferred_backend: Option<&str>,
) -> Option<(String, serde_yaml::Value)> {
    let subagents = yaml_lookup(overlay, &["agent_system", "subagents"])?;
    let entries = subagents.as_mapping()?;
    let backend_class = configured_dispatch_backend_class(overlay, system);
    let configured_backend_id =
        project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay))
            .get(system)
            .and_then(|entry| {
                yaml_string(yaml_lookup(entry, &["external_backend_id"]))
                    .or_else(|| yaml_string(yaml_lookup(entry, &["dispatch_backend_id"])))
            })
            .filter(|value| !value.trim().is_empty());
    if let Some(preferred_backend) = preferred_backend {
        for (key, value) in entries {
            let backend_id = key.as_str()?.trim();
            if backend_id != preferred_backend {
                continue;
            }
            if !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
                continue;
            }
            if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
                != Some(backend_class.as_str())
            {
                continue;
            }
            return Some((backend_id.to_string(), value.clone()));
        }
        return None;
    }
    if let Some(configured_backend_id) = configured_backend_id.as_deref() {
        for (key, value) in entries {
            let backend_id = key.as_str()?.trim();
            if backend_id != configured_backend_id {
                continue;
            }
            if !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
                continue;
            }
            if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
                != Some(backend_class.as_str())
            {
                continue;
            }
            return Some((backend_id.to_string(), value.clone()));
        }
    }
    let mut fallback = None;
    for (key, value) in entries {
        let backend_id = key.as_str()?.trim();
        if backend_id.is_empty() || !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
            continue;
        }
        if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
            != Some(backend_class.as_str())
        {
            continue;
        }
        let detect_command = yaml_string(yaml_lookup(value, &["detect_command"]));
        if detect_command.as_deref() == Some(system) {
            return Some((backend_id.to_string(), value.clone()));
        }
        if fallback.is_none() {
            fallback = Some((backend_id.to_string(), value.clone()));
        }
    }
    fallback
}

fn external_cli_activation_prompt(packet_path: &str) -> String {
    format!(
        "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
        packet_path
    )
}

fn dispatch_packet_prompt_text(packet_path: &str) -> Option<String> {
    std::fs::read_to_string(packet_path)
        .ok()
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|packet| {
            packet
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn configured_external_activation_prompt(
    backend_entry: &serde_yaml::Value,
    packet_path: &str,
) -> String {
    let packet_prompt = dispatch_packet_prompt_text(packet_path);
    yaml_lookup(backend_entry, &["dispatch", "prompt_template"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|template| {
            template
                .replace("{packet_path}", packet_path)
                .replace("{dispatch_packet_path}", packet_path)
                .replace("{prompt}", packet_prompt.as_deref().unwrap_or(""))
        })
        .or(packet_prompt)
        .unwrap_or_else(|| external_cli_activation_prompt(packet_path))
}

pub(crate) fn external_backend_profile_projection(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> serde_json::Value {
    let fallback_rate = yaml_string(yaml_lookup(backend_entry, &["budget_cost_units"]))
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            yaml_string(yaml_lookup(backend_entry, &["normalized_cost_units"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .or_else(|| {
            yaml_string(yaml_lookup(backend_entry, &["rate"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .unwrap_or(0);
    let fallback_runtime_roles =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["runtime_roles"]));
    let fallback_task_classes =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["task_classes"]));
    crate::model_profile_contract::normalize_profile_projection_from_yaml(
        backend_id,
        backend_entry,
        Some(fallback_rate),
        &fallback_runtime_roles,
        &fallback_task_classes,
    )
}

fn configured_external_dispatch_pin_args(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    preferred_profile_id: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();
    let dispatch = match yaml_lookup(backend_entry, &["dispatch"]) {
        Some(value) => value,
        None => return args,
    };
    let profile_projection = external_backend_profile_projection(backend_id, backend_entry);
    let selected_profile = crate::model_profile_contract::selected_model_profile_from_json_row(
        &profile_projection,
        preferred_profile_id,
    )
    .unwrap_or(serde_json::Value::Null);
    let selected_provider = selected_profile["provider"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains("provider-configured"))
        .map(str::to_string);
    let selected_model_ref = selected_profile["model_ref"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains("provider-configured"))
        .map(str::to_string)
        .or_else(|| {
            yaml_string(yaml_lookup(backend_entry, &["default_model"]))
                .filter(|value| !value.is_empty() && !value.contains("provider-configured"))
        });

    if let Some(provider_flag) = yaml_string(yaml_lookup(dispatch, &["provider_flag"])) {
        let provider_value = yaml_string(yaml_lookup(dispatch, &["provider_value"]))
            .or_else(|| selected_provider.clone())
            .filter(|value| !value.is_empty() && !value.contains("provider-configured"));
        if let Some(provider_value) = provider_value {
            args.push(provider_flag);
            args.push(provider_value);
        }
    }

    if let Some(model_flag) = yaml_string(yaml_lookup(dispatch, &["model_flag"])) {
        if let Some(default_model) = selected_model_ref {
            args.push(model_flag);
            args.push(default_model);
        }
    }

    if let Some(reasoning_effort_flag) =
        yaml_string(yaml_lookup(dispatch, &["reasoning_effort_flag"]))
    {
        let selected_reasoning_effort = selected_profile["reasoning_effort"]
            .as_str()
            .or_else(|| selected_profile["thinking_level"].as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "provider_default")
            .map(str::to_string);
        if let Some(reasoning_effort) = selected_reasoning_effort {
            args.push(reasoning_effort_flag);
            args.push(reasoning_effort);
        }
    }

    if selected_profile_requires_owned_path_guard(&selected_profile) {
        if let Some(scope_guard_mode_flag) =
            yaml_string(yaml_lookup(dispatch, &["scope_guard_mode_flag"]))
        {
            let scope_guard_mode_value = yaml_string(yaml_lookup(
                dispatch,
                &["scope_guard_mode_value_for_write_profiles"],
            ))
            .unwrap_or_else(|| "guarded-write".to_string());
            args.push(scope_guard_mode_flag);
            args.push(scope_guard_mode_value);
        }
    }

    if let Some(variant_flag) = yaml_string(yaml_lookup(dispatch, &["variant_flag"])) {
        if let Some(variant_value) =
            yaml_string(yaml_lookup(dispatch, &["variant_value"])).filter(|value| !value.is_empty())
        {
            args.push(variant_flag);
            args.push(variant_value);
        }
    }

    args
}

pub(crate) fn selected_profile_requires_owned_path_guard(
    selected_profile: &serde_json::Value,
) -> bool {
    matches!(
        selected_profile["write_scope"]
            .as_str()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .unwrap_or_default()
            .as_str(),
        "guard_required"
            | "guard-required"
            | "guard_required_owned_paths"
            | "guard-required-owned-paths"
            | "guard_required_packet_owned_paths"
            | "guard-required-packet-owned-paths"
    )
}

fn configured_external_activation_command(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    project_root: &Path,
    packet_path: &str,
    preferred_profile_id: Option<&str>,
) -> Option<String> {
    if external_backend_dispatch_blocker(backend_id, backend_entry).is_some() {
        return None;
    }
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    let command = yaml_string(yaml_lookup(dispatch, &["command"]))?;
    let mut parts = Vec::new();
    if let Some(env_map) = yaml_lookup(dispatch, &["env"]).and_then(serde_yaml::Value::as_mapping) {
        let mut env_pairs = env_map
            .iter()
            .filter_map(|(key, value)| {
                Some(format!(
                    "{}={}",
                    key.as_str()?.trim(),
                    shell_quote(value.as_str()?.trim())
                ))
            })
            .collect::<Vec<_>>();
        env_pairs.sort();
        parts.extend(env_pairs);
    }
    parts.push(command);
    parts.extend(yaml_string_list(yaml_lookup(dispatch, &["static_args"])));
    parts.extend(configured_external_dispatch_pin_args(
        backend_id,
        backend_entry,
        preferred_profile_id,
    ));
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        parts.push(workdir_flag);
        parts.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => parts.push(configured_external_activation_prompt(
            backend_entry,
            packet_path,
        )),
        "flag_value" => {
            if let Some(prompt_flag) = yaml_string(yaml_lookup(dispatch, &["prompt_flag"])) {
                parts.push(prompt_flag);
                parts.push(configured_external_activation_prompt(
                    backend_entry,
                    packet_path,
                ));
            }
        }
        "stdin" => {}
        _ => {}
    }
    Some(
        parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 || (index > 0 && part.contains('=') && !part.starts_with('-')) {
                    part
                } else {
                    shell_quote(&part)
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn configured_external_activation_parts(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    project_root: &Path,
    packet_path: &str,
    preferred_profile_id: Option<&str>,
) -> Result<(String, Vec<String>), String> {
    if let Some(blocker) = external_backend_dispatch_blocker(backend_id, backend_entry) {
        return Err(blocker);
    }
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])
        .ok_or_else(|| "Configured external backend is missing `dispatch`".to_string())?;
    let command = yaml_string(yaml_lookup(dispatch, &["command"]))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Configured external backend is missing non-empty `dispatch.command`".to_string()
        })?;
    if !external_dispatch_command_is_config_safe(&command) {
        return Err(format!(
            "Configured external backend `{backend_id}` uses unsafe `dispatch.command` `{command}`; external dispatch commands must be config-owned command tokens, not shell snippets or path-like invocations"
        ));
    }
    let mut args = yaml_string_list(yaml_lookup(dispatch, &["static_args"]));
    args.extend(configured_external_dispatch_pin_args(
        backend_id,
        backend_entry,
        preferred_profile_id,
    ));
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        args.push(workdir_flag);
        args.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => args.push(configured_external_activation_prompt(
            backend_entry,
            packet_path,
        )),
        "flag_value" => {
            let prompt_flag = yaml_string(yaml_lookup(dispatch, &["prompt_flag"]))
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    "Configured external backend uses prompt_mode `flag_value` without non-empty `prompt_flag`"
                        .to_string()
                })?;
            args.push(prompt_flag);
            args.push(configured_external_activation_prompt(
                backend_entry,
                packet_path,
            ));
        }
        "stdin" => {}
        other => {
            return Err(format!(
                "Configured external backend uses unsupported prompt_mode `{other}`"
            ));
        }
    }
    Ok((command, args))
}

pub(crate) fn configured_external_activation_stdin_payload(
    backend_entry: &serde_yaml::Value,
    packet_path: &str,
) -> Result<Option<String>, String> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])
        .ok_or_else(|| "Configured external backend is missing `dispatch`".to_string())?;
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" | "flag_value" => Ok(None),
        "stdin" => Ok(Some(configured_external_activation_prompt(
            backend_entry,
            packet_path,
        ))),
        other => Err(format!(
            "Configured external backend uses unsupported prompt_mode `{other}`"
        )),
    }
}

fn external_backend_dispatch_blocker(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> Option<String> {
    if yaml_lookup(backend_entry, &["enabled"]).is_some()
        && !yaml_bool(yaml_lookup(backend_entry, &["enabled"]), false)
    {
        return Some(format!(
            "external backend `{backend_id}` is disabled; external CLI execution path is forbidden"
        ));
    }
    if let Some(backend_class) =
        yaml_string(yaml_lookup(backend_entry, &["subagent_backend_class"]))
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
    {
        if backend_class != "external_cli" {
            return Some(format!(
                "backend `{backend_id}` has subagent_backend_class `{backend_class}` and cannot be dispatched through external CLI bridge"
            ));
        }
    }
    None
}

pub(crate) fn configured_external_backend_dispatch_blocker(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> Option<String> {
    external_backend_dispatch_blocker(backend_id, backend_entry)
}

fn external_dispatch_command_is_config_safe(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty()
        || trimmed.contains(std::path::MAIN_SEPARATOR)
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return false;
    }

    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

pub(crate) fn render_command_display(command: &str, args: &[String]) -> String {
    let mut rendered = Vec::with_capacity(args.len() + 1);
    rendered.push(shell_quote(command));
    rendered.extend(args.iter().map(|arg| shell_quote(arg)));
    rendered.join(" ")
}

#[cfg(test)]
mod runtime_dispatch_external_backend_tests {
    use super::*;

    #[test]
    fn selected_external_backend_prefers_system_configured_backend_id() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      external_backend_id: qwen_dispatch
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
    qwen_dispatch:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");
        assert_eq!(backend_id, "qwen_dispatch");
    }

    #[test]
    fn configured_external_activation_parts_uses_prompt_template_when_present() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: qwen
  static_args: ["run"]
  prompt_mode: positional
  prompt_template: "Process packet {packet_path} exactly once."
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "qwen_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "qwen");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "Process packet /tmp/project/.vida/dispatch.json exactly once.".to_string()
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_prefers_packet_prompt_by_default() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: vibe
  static_args: ["--output", "text"]
  prompt_mode: flag_value
  prompt_flag: -p
"#,
        )
        .expect("backend entry should parse");
        let root = std::env::temp_dir().join(format!(
            "vida-external-prompt-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let packet_path = root.join("dispatch.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({ "prompt": "Execute this concrete packet body." }).to_string(),
        )
        .expect("packet");

        let (_command, args) = configured_external_activation_parts(
            "vibe_cli",
            &backend_entry,
            &root,
            packet_path.to_str().expect("packet path"),
            None,
        )
        .expect("dispatch parts should render");

        assert_eq!(
            args,
            vec![
                "--output".to_string(),
                "text".to_string(),
                "-p".to_string(),
                "Execute this concrete packet body.".to_string()
            ]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn configured_external_activation_prompt_template_can_include_packet_prompt() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: vibe
  static_args: ["--output", "text"]
  prompt_mode: positional
  prompt_template: "Packet: {prompt}"
"#,
        )
        .expect("backend entry should parse");
        let root = std::env::temp_dir().join(format!(
            "vida-external-template-prompt-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let packet_path = root.join("dispatch.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({ "prompt": "Use template prompt body." }).to_string(),
        )
        .expect("packet");

        let (_command, args) = configured_external_activation_parts(
            "vibe_cli",
            &backend_entry,
            &root,
            packet_path.to_str().expect("packet path"),
            None,
        )
        .expect("dispatch parts should render");

        assert_eq!(
            args,
            vec![
                "--output".to_string(),
                "text".to_string(),
                "Packet: Use template prompt body.".to_string()
            ]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn configured_external_activation_parts_accepts_configured_command_without_binary_hardcode() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: newly-configured-carrier
  static_args: ["run"]
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "new_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("config-owned command token should be accepted without a binary-name allowlist");
        assert_eq!(command, "newly-configured-carrier");
        assert_eq!(args[0], "run");
    }

    #[test]
    fn configured_external_activation_parts_rejects_path_like_command_token() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: ./tools/configured-carrier
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let error = configured_external_activation_parts(
            "external_fixture",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect_err("path-like command should be rejected");

        assert!(error.contains("unsafe"));
        assert!(error.contains("./tools/configured-carrier"));
    }

    #[test]
    fn configured_external_activation_parts_accepts_configured_adapter_but_rejects_path_like_variant(
    ) {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: configured-adapter
  static_args: ["--mode", "rpc"]
  prompt_mode: stdin
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "external_fixture",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("configured adapter command should be trusted by config");
        assert_eq!(command, "configured-adapter");
        assert_eq!(args, vec!["--mode".to_string(), "rpc".to_string()]);

        let path_like: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: ./configured-adapter
  prompt_mode: stdin
"#,
        )
        .expect("backend entry should parse");
        let error = configured_external_activation_parts(
            "external_fixture",
            &path_like,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect_err("path-like configured adapter must remain rejected");
        assert!(error.contains("unsafe"));
        assert!(error.contains("./configured-adapter"));
    }

    #[test]
    fn configured_external_activation_parts_supports_stdin_prompt_mode_without_positional_prompt() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: vida-pi-agent
  static_args: ["--mode", "rpc", "--no-session"]
  workdir_flag: --workdir
  prompt_mode: stdin
  prompt_template: "Process packet {packet_path}."
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "pi_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("stdin dispatch parts should render");
        assert_eq!(command, "vida-pi-agent");
        assert_eq!(
            args,
            vec![
                "--mode".to_string(),
                "rpc".to_string(),
                "--no-session".to_string(),
                "--workdir".to_string(),
                "/tmp/project".to_string(),
            ]
        );
        assert_eq!(
            configured_external_activation_stdin_payload(
                &backend_entry,
                "/tmp/project/.vida/dispatch.json"
            )
            .expect("stdin payload should render")
            .as_deref(),
            Some("Process packet /tmp/project/.vida/dispatch.json.")
        );
    }

    #[test]
    fn configured_external_activation_parts_supports_prompt_flag_value_mode() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  command: vibe
  static_args: ["--output", "text", "--max-turns", "1"]
  workdir_flag: --workdir
  prompt_mode: flag_value
  prompt_flag: -p
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "vibe_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("prompt flag-value dispatch parts should render");
        assert_eq!(command, "vibe");
        assert_eq!(
            args,
            vec![
                "--output".to_string(),
                "text".to_string(),
                "--max-turns".to_string(),
                "1".to_string(),
                "--workdir".to_string(),
                "/tmp/project".to_string(),
                "-p".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
        assert_eq!(
            configured_external_activation_stdin_payload(
                &backend_entry,
                "/tmp/project/.vida/dispatch.json"
            )
            .expect("flag-value mode should not use stdin")
            .as_deref(),
            None
        );
    }

    #[test]
    fn configured_external_activation_parts_injects_pi_model_and_thinking_level() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
default_model_profile: pi_gpt55_medium_guarded
model_profiles:
  pi_gpt54_mini_low_guarded:
    provider: pi
    model_ref: openai-codex/gpt-5.4-mini
    reasoning_effort: low
    normalized_cost_units: 1
    write_scope: guard_required_owned_paths
    runtime_roles: [worker]
    task_classes: [implementation]
  pi_gpt55_medium_guarded:
    provider: pi
    model_ref: openai-codex/gpt-5.5
    reasoning_effort: medium
    normalized_cost_units: 4
    runtime_roles: [worker]
    task_classes: [implementation]
dispatch:
  command: vida-pi-agent
  static_args: ["--mode", "rpc"]
  model_flag: --model
  reasoning_effort_flag: --thinking-level
  scope_guard_mode_flag: --scope-guard-mode
  scope_guard_mode_value_for_write_profiles: guarded-write
  prompt_mode: stdin
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "pi_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("pi_gpt54_mini_low_guarded"),
        )
        .expect("pi dispatch parts should render");
        assert_eq!(command, "vida-pi-agent");
        assert_eq!(
            args,
            vec![
                "--mode".to_string(),
                "rpc".to_string(),
                "--model".to_string(),
                "openai-codex/gpt-5.4-mini".to_string(),
                "--thinking-level".to_string(),
                "low".to_string(),
                "--scope-guard-mode".to_string(),
                "guarded-write".to_string(),
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_rejects_disabled_backend_before_command_generation() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
enabled: false
subagent_backend_class: external_cli
default_model: gpt-5.5
dispatch:
  command: configured-carrier
  static_args: ["run", "--ephemeral", "-s", "workspace-write"]
  model_flag: -m
  workdir_flag: -C
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let error = configured_external_activation_parts(
            "disabled_external",
            &backend_entry,
            Path::new("C:/project/vida_mobile"),
            "C:/project/vida_mobile/.vida/dispatch.json",
            None,
        )
        .expect_err("disabled external backend must not render configured command");

        assert!(error.contains("disabled"));
        assert!(error.contains("external CLI execution path is forbidden"));
        assert!(
            configured_external_activation_command(
                "disabled_external",
                &backend_entry,
                Path::new("C:/project/vida_mobile"),
                "C:/project/vida_mobile/.vida/dispatch.json",
                None,
            )
            .is_none(),
            "disabled external backend must not produce a preview command"
        );
    }

    #[test]
    fn configured_external_activation_parts_rejects_internal_backend_before_external_bridge() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
enabled: true
subagent_backend_class: internal
default_model: gpt-5.5
dispatch:
  command: configured-carrier
  static_args: ["run", "--ephemeral", "-s", "workspace-write"]
  model_flag: -m
  workdir_flag: -C
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let error = configured_external_activation_parts(
            "internal_subagents",
            &backend_entry,
            Path::new("C:/project/vida_mobile"),
            "C:/project/vida_mobile/.vida/dispatch.json",
            None,
        )
        .expect_err("internal backend must not render external CLI bridge");

        assert!(error.contains("subagent_backend_class `internal`"));
        assert!(error.contains("external CLI bridge"));
        assert!(
            configured_external_activation_command(
                "internal_subagents",
                &backend_entry,
                Path::new("C:/project/vida_mobile"),
                "C:/project/vida_mobile/.vida/dispatch.json",
                None,
            )
            .is_none(),
            "internal backend must not produce a preview command"
        );
    }

    #[test]
    fn configured_external_activation_parts_injects_provider_and_model_flags() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
default_model: opencode/minimax-m2.5-free
dispatch:
  command: opencode
  static_args: ["run"]
  provider_flag: --provider
  provider_value: opencode
  model_flag: --model
  workdir_flag: --dir
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "opencode_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "opencode");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "--provider".to_string(),
                "opencode".to_string(),
                "--model".to_string(),
                "opencode/minimax-m2.5-free".to_string(),
                "--dir".to_string(),
                "/tmp/project".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_prefers_selected_model_profile_over_default_model() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
default_model: opencode/minimax-m2.5-free
default_model_profile: opencode_minimax_free_review
model_profiles:
  opencode_minimax_free_review:
    provider: opencode
    model_ref: opencode/minimax-m2.5-free
    reasoning_effort: provider_default
    normalized_cost_units: 0
    runtime_roles: [coach]
    task_classes: [review]
  opencode_codex_mini_review:
    provider: opencode
    model_ref: opencode/gpt-5.1-codex-mini
    reasoning_effort: low
    normalized_cost_units: 1
    runtime_roles: [coach]
    task_classes: [review]
dispatch:
  command: opencode
  static_args: ["run"]
  provider_flag: --provider
  model_flag: --model
  workdir_flag: --dir
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "opencode_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("opencode_codex_mini_review"),
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "opencode");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "--provider".to_string(),
                "opencode".to_string(),
                "--model".to_string(),
                "opencode/gpt-5.1-codex-mini".to_string(),
                "--dir".to_string(),
                "/tmp/project".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_skips_provider_configured_model_placeholders() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
default_model: hermes/provider-configured
dispatch:
  command: hermes
  static_args: ["chat", "-Q", "-q"]
  model_flag: --model
  provider_flag: --provider
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            "hermes_cli",
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "hermes");
        assert_eq!(
            args,
            vec![
                "chat".to_string(),
                "-Q".to_string(),
                "-q".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
    }

    #[test]
    fn selected_external_backend_uses_configured_backend_class() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      dispatch_backend_class: remote_cli
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
    qwen_remote:
      enabled: true
      subagent_backend_class: remote_cli
"#,
        )
        .expect("overlay should parse");

        let backend_class = configured_dispatch_backend_class(&overlay, "qwen");
        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");

        assert_eq!(backend_class, "remote_cli");
        assert_eq!(backend_id, "qwen_remote");
    }

    #[test]
    fn selected_external_backend_fails_closed_when_preferred_backend_is_unavailable() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        assert!(
            selected_external_backend_for_system(&overlay, "qwen", Some("hermes_cli")).is_none()
        );
    }

    #[test]
    fn configured_external_backend_entry_requires_enabled_true() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    qwen_dispatch:
      subagent_backend_class: external_cli
      dispatch:
        command: qwen
        prompt_mode: positional
    qwen_enabled:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        assert!(
            configured_external_backend_entry(&overlay, "qwen_dispatch").is_none(),
            "missing enabled must fail closed for external backend selection"
        );
        assert!(
            configured_external_backend_entry(&overlay, "qwen_enabled").is_some(),
            "enabled external backend should still resolve"
        );
    }

    #[test]
    fn selected_external_backend_does_not_prefer_name_pattern_without_config_or_detect_signal() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
agent_system:
  subagents:
    alpha_external:
      enabled: true
      subagent_backend_class: external_cli
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");

        assert_eq!(backend_id, "alpha_external");
    }

    #[test]
    fn configured_subagent_entry_resolves_enabled_internal_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let entry = configured_subagent_entry(&overlay, "internal_subagents")
            .expect("internal backend should resolve");

        assert_eq!(
            yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref(),
            Some("internal")
        );
    }

    #[test]
    fn internal_host_ignores_explicit_external_backend_and_stays_on_agent_init() {
        let overlay = serde_yaml::from_str(
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
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("hermes_cli"),
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(dispatch.backend_dispatch["backend_id"], "hermes_cli");
    }

    #[test]
    fn external_host_keeps_policy_selected_internal_backend_on_agent_init() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      runtime_root: .qwen
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "qwen",
            "external",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("internal_subagents"),
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }

    #[test]
    fn internal_host_without_preferred_backend_stays_on_agent_init() {
        let overlay = serde_yaml::from_str(
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
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn internal_host_carrier_role_id_is_classified_as_internal_backend() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
      carriers:
        junior:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: low
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("junior"),
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(dispatch.backend_dispatch["backend_id"], "junior");
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }
    #[test]
    fn internal_host_carrier_id_takes_precedence_over_external_backend_id_collision() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
      carriers:
        junior:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: low
agent_system:
  subagents:
    junior:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: hermes
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("junior"),
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(dispatch.backend_dispatch["backend_id"], "junior");
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }
}

fn runtime_agent_lane_dispatch_from_overlay(
    overlay: Option<&serde_yaml::Value>,
    selected_cli_system: &str,
    selected_execution_class: &str,
    project_root: &Path,
    packet_path: &str,
    preferred_backend: Option<&str>,
    preferred_model_profile_id: Option<&str>,
) -> RuntimeAgentLaneDispatch {
    let agent_init_command = agent_init_execute_command_for_packet_path(packet_path);
    let preferred_external_backend = preferred_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|backend_id| {
            configured_external_backend_entry(overlay?, backend_id)
                .cloned()
                .map(|entry| (backend_id.to_string(), entry))
        });
    let internal_host_backend_hint = preferred_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|backend_id| {
            let configured_backend = overlay
                .and_then(|overlay| configured_subagent_entry(overlay, backend_id))
                .is_some();
            let configured_internal_host_carrier =
                configured_internal_host_carrier_exists(overlay, selected_cli_system, backend_id);
            (!configured_backend && !configured_internal_host_carrier)
                .then(|| backend_id.to_string())
        });
    if selected_execution_class != "external" {
        if let Some(backend_id) = preferred_backend
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .filter(|backend_id| {
                configured_internal_host_carrier_exists(overlay, selected_cli_system, backend_id)
            })
        {
            return RuntimeAgentLaneDispatch {
                surface: "vida agent-init".to_string(),
                activation_command: agent_init_command,
                backend_dispatch: serde_json::json!({
                    "selected_cli_system": selected_cli_system,
                    "selected_execution_class": selected_execution_class,
                    "backend_class": "internal",
                    "backend_id": backend_id,
                    "executor_backend": "internal",
                    "selected_model_profile_id": preferred_model_profile_id,
                    "policy_selected_internal_backend": true,
                }),
            };
        }
        if let Some((backend_id, backend_entry)) = preferred_external_backend {
            return RuntimeAgentLaneDispatch {
                surface: format!("external_cli:{backend_id}"),
                activation_command: configured_external_activation_command(
                    &backend_id,
                    &backend_entry,
                    project_root,
                    packet_path,
                    preferred_model_profile_id,
                )
                .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path)),
                backend_dispatch: serde_json::json!({
                    "selected_cli_system": selected_cli_system,
                    "selected_execution_class": selected_execution_class,
                    "backend_class": "external_cli",
                    "backend_id": backend_id,
                    "selected_model_profile_id": preferred_model_profile_id,
                    "policy_selected_external_backend": true,
                }),
            };
        }
        if let Some(backend_id) = preferred_backend
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .filter(|backend_id| {
                configured_internal_host_carrier_exists(overlay, selected_cli_system, backend_id)
            })
        {
            return RuntimeAgentLaneDispatch {
                surface: "vida agent-init".to_string(),
                activation_command: agent_init_command,
                backend_dispatch: serde_json::json!({
                    "selected_cli_system": selected_cli_system,
                    "selected_execution_class": selected_execution_class,
                    "backend_class": "internal",
                    "backend_id": backend_id,
                    "executor_backend": "internal",
                    "selected_model_profile_id": preferred_model_profile_id,
                    "policy_selected_internal_backend": true,
                }),
            };
        }
        if let Some((backend_id, backend_class, _backend_entry)) = overlay.and_then(|overlay| {
            preferred_backend.and_then(|backend_id| {
                configured_subagent_entry(overlay, backend_id).and_then(|entry| {
                    yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
                        .map(|backend_class| (backend_id.to_string(), backend_class, entry))
                })
            })
        }) {
            if backend_class == "internal" {
                return RuntimeAgentLaneDispatch {
                    surface: "vida agent-init".to_string(),
                    activation_command: agent_init_command,
                    backend_dispatch: serde_json::json!({
                        "selected_cli_system": selected_cli_system,
                        "selected_execution_class": selected_execution_class,
                        "backend_class": backend_class,
                        "backend_id": backend_id,
                        "executor_backend": "internal",
                        "selected_model_profile_id": preferred_model_profile_id,
                        "policy_selected_internal_backend": true,
                    }),
                };
            }
        }
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": internal_host_backend_hint,
            }),
        };
    }
    if let Some((backend_id, backend_class, backend_entry)) = overlay.and_then(|overlay| {
        preferred_backend.and_then(|backend_id| {
            configured_external_backend_entry(overlay, backend_id)
                .map(|entry| (backend_id.to_string(), "external_cli".to_string(), entry))
        })
    }) {
        return RuntimeAgentLaneDispatch {
            surface: format!("external_cli:{backend_id}"),
            activation_command: configured_external_activation_command(
                &backend_id,
                backend_entry,
                project_root,
                packet_path,
                preferred_model_profile_id,
            )
            .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path)),
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_class": backend_class,
                "backend_id": backend_id,
                "selected_model_profile_id": preferred_model_profile_id,
                "policy_selected_external_backend": true,
            }),
        };
    }
    if let Some((backend_id, backend_class, backend_entry)) = overlay.and_then(|overlay| {
        preferred_backend.and_then(|backend_id| {
            configured_subagent_entry(overlay, backend_id).and_then(|entry| {
                yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
                    .map(|backend_class| (backend_id.to_string(), backend_class, entry))
            })
        })
    }) {
        if backend_class == "internal" {
            return RuntimeAgentLaneDispatch {
                surface: "vida agent-init".to_string(),
                activation_command: agent_init_command,
                backend_dispatch: serde_json::json!({
                    "selected_cli_system": selected_cli_system,
                    "selected_execution_class": selected_execution_class,
                    "backend_class": backend_class,
                    "backend_id": backend_id,
                    "executor_backend": "internal",
                    "selected_model_profile_id": preferred_model_profile_id,
                    "policy_selected_internal_backend": true,
                }),
            };
        }
        return RuntimeAgentLaneDispatch {
            surface: format!("external_cli:{backend_id}"),
            activation_command: configured_external_activation_command(
                &backend_id,
                backend_entry,
                project_root,
                packet_path,
                preferred_model_profile_id,
            )
            .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path)),
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_class": backend_class,
                "backend_id": backend_id,
                "selected_model_profile_id": preferred_model_profile_id,
                "policy_selected_external_backend": true,
            }),
        };
    }
    let Some(overlay) = overlay else {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": serde_json::Value::Null,
            }),
        };
    };
    let Some((backend_id, backend_entry)) =
        selected_external_backend_for_system(overlay, selected_cli_system, preferred_backend)
    else {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": serde_json::Value::Null,
            }),
        };
    };
    let backend_class = configured_dispatch_backend_class(overlay, selected_cli_system);
    let activation_command = configured_external_activation_command(
        &backend_id,
        &backend_entry,
        project_root,
        packet_path,
        preferred_model_profile_id,
    )
    .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path));
    RuntimeAgentLaneDispatch {
        surface: format!("{backend_class}:{backend_id}"),
        activation_command,
        backend_dispatch: serde_json::json!({
            "selected_cli_system": selected_cli_system,
            "selected_execution_class": selected_execution_class,
            "backend_class": backend_class,
            "backend_id": backend_id,
            "selected_model_profile_id": preferred_model_profile_id,
        }),
    }
}

pub(crate) fn runtime_agent_lane_dispatch_for_root(
    project_root: &Path,
    packet_path: &str,
    preferred_backend: Option<&str>,
    preferred_model_profile_id: Option<&str>,
) -> RuntimeAgentLaneDispatch {
    let host_runtime = runtime_host_execution_contract_for_root(project_root);
    let overlay = load_project_overlay_yaml_for_root(project_root).ok();
    let overlay_host_selection = overlay
        .as_ref()
        .map(|config| selected_host_cli_system_for_runtime_dispatch(config));
    let selected_cli_system = json_string(host_runtime.get("selected_cli_system"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            overlay_host_selection
                .as_ref()
                .map(|(system, _)| system.clone())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let selected_execution_class = json_string(host_runtime.get("selected_cli_execution_class"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            overlay_host_selection
                .as_ref()
                .and_then(|(_, entry)| entry.as_ref())
                .and_then(|entry| yaml_string(yaml_lookup(entry, &["execution_class"])))
        })
        .unwrap_or_else(|| "unknown".to_string());
    let effective_system = overlay_host_selection
        .as_ref()
        .map(|(system, _)| system.clone())
        .unwrap_or_else(|| selected_cli_system.clone());
    runtime_agent_lane_dispatch_from_overlay(
        overlay.as_ref(),
        &effective_system,
        &selected_execution_class,
        project_root,
        packet_path,
        preferred_backend,
        preferred_model_profile_id,
    )
}

pub(crate) fn dispatch_receipt_has_execution_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    match receipt.dispatch_status.as_str() {
        "executed" => {
            if receipt.blocker_code.is_some() {
                return false;
            }
            let dispatch_kind =
                receipt_result_path_activation_kind(receipt.dispatch_result_path.as_deref());
            let downstream_kind = receipt_result_path_activation_kind(
                receipt.downstream_dispatch_result_path.as_deref(),
            );
            if dispatch_kind.is_some() || downstream_kind.is_some() {
                return dispatch_kind == Some("execution_evidence")
                    || downstream_kind == Some("execution_evidence");
            }
            false
        }
        "packet_ready" => {
            receipt.blocker_code.is_none()
                && receipt_result_path_has_execution_evidence(
                    receipt.dispatch_result_path.as_deref(),
                )
        }
        _ => false,
    }
}

fn dispatch_receipt_allows_synthetic_lane_completion(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_status != "executed" || receipt.blocker_code.is_some() {
        return false;
    }
    let dispatch_kind =
        receipt_result_path_activation_kind(receipt.dispatch_result_path.as_deref());
    let downstream_kind =
        receipt_result_path_activation_kind(receipt.downstream_dispatch_result_path.as_deref());
    dispatch_kind.is_none() && downstream_kind.is_none()
}

fn nonempty_result_path(path: Option<&str>) -> Option<String> {
    path.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn readable_result_path(state_root: &Path, path: Option<&str>) -> Option<String> {
    let candidate = nonempty_result_path(path)?;
    let resolved = if Path::new(&candidate).is_absolute() {
        Path::new(&candidate).to_path_buf()
    } else {
        state_root.join(&candidate)
    };
    resolved.exists().then_some(candidate)
}

fn readable_verification_evidence_result_path(
    state_root: &Path,
    path: Option<&str>,
) -> Option<String> {
    let candidate = readable_result_path(state_root, path)?;
    let resolved = if Path::new(&candidate).is_absolute() {
        Path::new(&candidate).to_path_buf()
    } else {
        state_root.join(&candidate)
    };
    let result = crate::read_json_file_if_present(&resolved)?;
    let artifact_kind = result["artifact_kind"].as_str().unwrap_or_default();
    let status = result["status"].as_str().unwrap_or_default();
    let execution_state = result["execution_state"].as_str().unwrap_or_default();
    let is_verification_evidence = matches!(
        artifact_kind,
        "verification_evidence" | "runtime_lane_completion_result"
    ) || dispatch_result_activation_kind(&result)
        == Some("execution_evidence");
    let is_blocked_activation_view = dispatch_result_activation_kind(&result)
        == Some("activation_view")
        || matches!(execution_state, "blocked" | "executing");
    let receipt_backed = result
        .get("completion_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || result
            .get("receipt_backed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
    if is_verification_evidence
        && !is_blocked_activation_view
        && !matches!(status, "blocked" | "failed")
        && receipt_backed
    {
        return Some(candidate);
    }
    None
}

fn synthetic_execution_completion_receipt_id(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> String {
    format!(
        "receipt-executed-{}-{}",
        receipt.run_id, receipt.dispatch_target
    )
}

fn tracked_implementer_dev_task_id<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn tracked_specification_task_id<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["spec_task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn tracked_design_doc_path<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn tracked_implementer_task_closed(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let implementer_context = receipt.dispatch_target == "implementer"
        || receipt.downstream_dispatch_last_target.as_deref() == Some("implementer");
    if !implementer_context {
        return false;
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return false;
    };
    store
        .show_task(task_id)
        .await
        .map(|task| task.status == "closed")
        .unwrap_or(false)
}

const TRACKED_DESIGN_DOC_MAX_BYTES: u64 = 256 * 1024;

fn tracked_design_doc_finalized(role_selection: &RuntimeConsumptionLaneSelection) -> bool {
    let Some(path) = tracked_design_doc_path(role_selection) else {
        return false;
    };

    let resolved_path = normalize_persisted_runtime_path(path);

    let metadata = match std::fs::symlink_metadata(&resolved_path) {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    if metadata.len() > TRACKED_DESIGN_DOC_MAX_BYTES {
        return false;
    }

    std::fs::read_to_string(&resolved_path)
        .map(|contents| {
            contents
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case("Status: `approved`"))
        })
        .unwrap_or(false)
}

async fn tracked_specification_task_closed(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if !matches!(
        receipt.dispatch_target.as_str(),
        "specification" | "spec-pack"
    ) {
        return false;
    }
    let Some(task_id) = tracked_specification_task_id(role_selection) else {
        return false;
    };
    store
        .show_task(task_id)
        .await
        .map(|task| task.status == "closed")
        .unwrap_or(false)
}

async fn tracked_specification_gate_completion_ready(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_target == "specification"
        && receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && tracked_specification_task_id(role_selection).is_some()
        && tracked_design_doc_finalized(role_selection)
        && tracked_specification_task_closed(store, role_selection, receipt).await
}

async fn tracked_specification_gate_completion_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    if receipt.dispatch_target != "specification" {
        return Ok(None);
    }
    if !tracked_design_doc_finalized(role_selection) {
        return Ok(None);
    }
    if !tracked_specification_task_closed(store, role_selection, receipt).await {
        return Ok(None);
    }
    let Some(task_id) = tracked_specification_task_id(role_selection) else {
        return Ok(None);
    };
    let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let completion_receipt_id = format!("task-close-{task_id}");
    write_runtime_lane_completion_result(
        store.root(),
        &receipt.run_id,
        "specification",
        &completion_receipt_id,
        packet_path,
    )
    .map(Some)
}

async fn tracked_implementer_task_close_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    let implementer_context = receipt.dispatch_target == "implementer"
        || receipt.downstream_dispatch_last_target.as_deref() == Some("implementer");
    if !implementer_context {
        return Ok(None);
    }
    if !tracked_implementer_task_closed(store, role_selection, receipt).await {
        return Ok(None);
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return Ok(None);
    };
    let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let completion_receipt_id = format!("task-close-{task_id}");
    write_runtime_lane_completion_result(
        store.root(),
        &receipt.run_id,
        "implementer",
        &completion_receipt_id,
        packet_path,
    )
    .map(Some)
}

async fn verification_closure_admission_ready(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    admitted_override: Option<bool>,
) -> Result<bool, String> {
    if let Some(admitted) = admitted_override {
        return Ok(admitted);
    }
    let runtime_bundle = crate::build_taskflow_consume_bundle_payload(store)
        .await
        .map_err(|error| {
            format!("Failed to build runtime bundle while checking verification closure admission: {error}")
        })?;
    let bundle_check = crate::taskflow_consume_bundle_check(&runtime_bundle);
    let (registry, check, readiness, proof, _overview) = crate::build_docflow_runtime_evidence();
    let docflow_verdict =
        crate::build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);
    let closure_admission =
        build_runtime_closure_admission(&bundle_check, &docflow_verdict, role_selection);
    if closure_admission.admitted {
        return Ok(true);
    }
    let has_readiness_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("readiness-check"));
    let has_proof_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"));
    Ok(bundle_check.ok && docflow_verdict.ready && has_readiness_surface && has_proof_surface)
}

async fn tracked_verification_closure_evidence_path_with_admission(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    admitted_override: Option<bool>,
) -> Result<Option<String>, String> {
    let verification_context = receipt.dispatch_target == "verification"
        || receipt.downstream_dispatch_last_target.as_deref() == Some("verification");
    if !verification_context {
        return Ok(None);
    }
    if !verification_closure_admission_ready(store, role_selection, admitted_override).await? {
        return Ok(None);
    }
    let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let completion_receipt_id = format!("closure-admission-{}", receipt.run_id);
    write_runtime_lane_completion_result(
        store.root(),
        &receipt.run_id,
        "verification",
        &completion_receipt_id,
        packet_path,
    )
    .map(Some)
}

async fn tracked_verification_closure_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    tracked_verification_closure_evidence_path_with_admission(store, role_selection, receipt, None)
        .await
}

async fn receipt_backed_execution_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    if matches!(receipt.dispatch_status.as_str(), "routed" | "packet_ready")
        && receipt
            .dispatch_result_path
            .as_deref()
            .is_none_or(str::is_empty)
        && !dispatch_receipt_has_execution_evidence(receipt)
        && !dispatch_receipt_allows_synthetic_lane_completion(receipt)
    {
        return Ok(None);
    }
    if let Some(path) =
        tracked_specification_gate_completion_evidence_path(store, role_selection, receipt).await?
    {
        return Ok(Some(path));
    }
    if let Some(path) =
        tracked_implementer_task_close_evidence_path(store, role_selection, receipt).await?
    {
        return Ok(Some(path));
    }
    if dispatch_receipt_has_execution_evidence(receipt)
        || dispatch_receipt_allows_synthetic_lane_completion(receipt)
    {
        if let Some(path) =
            readable_result_path(store.root(), receipt.dispatch_result_path.as_deref())
        {
            return Ok(Some(path));
        }
        if let Some(packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return write_runtime_lane_completion_result(
                store.root(),
                &receipt.run_id,
                &receipt.dispatch_target,
                &synthetic_execution_completion_receipt_id(receipt),
                packet_path,
            )
            .map(Some);
        }
    }
    Ok(None)
}

fn decode_receipt_packet_context(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(RuntimeConsumptionLaneSelection, serde_json::Value), String> {
    let packet_path = receipt.dispatch_packet_path.as_deref().ok_or_else(|| {
        format!(
            "Persisted dispatch receipt for `{}` is missing dispatch_packet_path",
            receipt.run_id
        )
    })?;
    let normalized_packet_path = normalize_persisted_runtime_path(packet_path);
    let body = std::fs::read_to_string(&normalized_packet_path).map_err(|error| {
        format!("Failed to read persisted dispatch packet `{packet_path}`: {error}")
    })?;
    let packet = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
        format!("Failed to parse persisted dispatch packet `{packet_path}`: {error}")
    })?;
    let role_selection = serde_json::from_value::<RuntimeConsumptionLaneSelection>(
        packet
            .get("role_selection_full")
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Persisted dispatch packet `{packet_path}` is missing role_selection_full"
                )
            })?,
    )
    .map_err(|error| {
        format!(
            "Failed to decode role_selection_full from persisted dispatch packet `{packet_path}`: {error}"
        )
    })?;
    let run_graph_bootstrap = packet.get("run_graph_bootstrap").cloned().ok_or_else(|| {
        format!("Persisted dispatch packet `{packet_path}` is missing run_graph_bootstrap")
    })?;
    Ok((role_selection, run_graph_bootstrap))
}

pub(crate) async fn maybe_bridge_closed_implementer_task_into_receipt_with_context(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    closed_task_id: Option<&str>,
) -> Result<bool, String> {
    if receipt.dispatch_target == "implementer"
        && receipt.dispatch_kind == "agent_lane"
        && receipt.dispatch_status == "blocked"
        && is_internal_activation_view_without_receipt_blocker(receipt.blocker_code.as_deref())
    {
        let Some(result_path) =
            tracked_implementer_task_close_evidence_path(store, role_selection, receipt).await?
        else {
            return Ok(false);
        };
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = "lane_completed".to_string();
        receipt.dispatch_result_path = Some(result_path);
        receipt.blocker_code = None;
        receipt.exception_path_receipt_id = None;
        refresh_downstream_dispatch_preview(store, role_selection, run_graph_bootstrap, receipt)
            .await?;
        return Ok(true);
    }
    if receipt.downstream_dispatch_last_target.as_deref() != Some("implementer") {
        return Ok(false);
    }
    if receipt
        .downstream_dispatch_target
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Ok(false);
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return Ok(false);
    };
    if closed_task_id.is_some_and(|value| value != task_id) {
        return Ok(false);
    }
    let implementer_receipt = crate::state_store::RunGraphDispatchReceipt {
        dispatch_target: "implementer".to_string(),
        ..receipt.clone()
    };
    if !tracked_implementer_task_closed(store, role_selection, &implementer_receipt).await {
        return Ok(false);
    }
    try_bridge_bounded_implementer_completion_to_downstream_receipt(
        store,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )
    .await
}

pub(crate) async fn maybe_reconcile_blocked_verification_timeout_with_receipt_evidence_with_admission(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    _admitted_override: Option<bool>,
) -> Result<bool, String> {
    if receipt.dispatch_target != "verification"
        || receipt.dispatch_kind != "agent_lane"
        || receipt.dispatch_status != "blocked"
        || !is_internal_activation_view_without_receipt_blocker(receipt.blocker_code.as_deref())
    {
        return Ok(false);
    }
    let result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    let result_path = result_path.or_else(|| {
        readable_verification_evidence_result_path(
            store.root(),
            receipt.downstream_dispatch_result_path.as_deref(),
        )
    });
    let Some(result_path) = result_path else {
        return Ok(false);
    };
    receipt.dispatch_status = "executed".to_string();
    receipt.lane_status = "lane_completed".to_string();
    receipt.dispatch_result_path = Some(result_path);
    receipt.blocker_code = None;
    receipt.exception_path_receipt_id = None;
    refresh_downstream_dispatch_preview(store, role_selection, run_graph_bootstrap, receipt)
        .await?;
    Ok(true)
}

pub(crate) async fn maybe_reconcile_blocked_verification_timeout_with_receipt_evidence(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    maybe_reconcile_blocked_verification_timeout_with_receipt_evidence_with_admission(
        store,
        role_selection,
        run_graph_bootstrap,
        receipt,
        None,
    )
    .await
}

pub(crate) async fn maybe_bridge_closed_implementer_task_into_receipt(
    store: &StateStore,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    closed_task_id: Option<&str>,
) -> Result<bool, String> {
    let (role_selection, run_graph_bootstrap) = decode_receipt_packet_context(receipt)?;
    maybe_bridge_closed_implementer_task_into_receipt_with_context(
        store,
        &role_selection,
        &run_graph_bootstrap,
        receipt,
        closed_task_id,
    )
    .await
}

pub(crate) async fn maybe_bridge_closed_implementer_task_into_latest_receipt(
    store: &StateStore,
    closed_task_id: &str,
) -> Result<bool, String> {
    let Some(mut receipt) = store
        .latest_run_graph_dispatch_receipt()
        .await
        .map_err(|error| format!("Failed to load latest run-graph dispatch receipt: {error}"))?
    else {
        return Ok(false);
    };
    if !maybe_bridge_closed_implementer_task_into_receipt(store, &mut receipt, Some(closed_task_id))
        .await?
    {
        return Ok(false);
    }
    store
        .record_run_graph_dispatch_receipt(&receipt)
        .await
        .map_err(|error| {
            format!("Failed to persist bridged run-graph dispatch receipt: {error}")
        })?;
    Ok(true)
}

pub(crate) async fn maybe_bridge_closed_specification_task_into_receipt(
    store: &StateStore,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    closed_task_id: Option<&str>,
) -> Result<bool, String> {
    let (role_selection, run_graph_bootstrap) = decode_receipt_packet_context(receipt)?;
    if closed_task_id
        .is_some_and(|value| tracked_specification_task_id(&role_selection) != Some(value))
    {
        return Ok(false);
    }
    try_bridge_bounded_specification_completion_to_downstream_receipt(
        store,
        &role_selection,
        &run_graph_bootstrap,
        receipt,
    )
    .await
}

pub(crate) async fn maybe_bridge_closed_specification_task_into_latest_receipt(
    store: &StateStore,
    closed_task_id: &str,
) -> Result<bool, String> {
    let Some(mut receipt) = store
        .latest_run_graph_dispatch_receipt()
        .await
        .map_err(|error| format!("Failed to load latest run-graph dispatch receipt: {error}"))?
    else {
        return Ok(false);
    };
    if !maybe_bridge_closed_specification_task_into_receipt(
        store,
        &mut receipt,
        Some(closed_task_id),
    )
    .await?
    {
        return Ok(false);
    }
    store
        .record_run_graph_dispatch_receipt(&receipt)
        .await
        .map_err(|error| {
            format!("Failed to persist bridged run-graph dispatch receipt: {error}")
        })?;
    Ok(true)
}

fn receipt_waiting_on_implementer_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.downstream_dispatch_last_target.as_deref() == Some("implementer")
        && receipt.downstream_dispatch_target.as_deref() == Some("coach")
        && !receipt.downstream_dispatch_ready
        && receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|value| value == blocker_code_str(BlockerCode::PendingImplementationEvidence))
}

fn receipt_waiting_on_specification_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let specification_gate_blockers = [
        blocker_code_str(BlockerCode::PendingSpecificationEvidence),
        blocker_code_str(BlockerCode::PendingDesignFinalize),
        blocker_code_str(BlockerCode::PendingSpecTaskClose),
    ];
    receipt.dispatch_target == "specification"
        && receipt.downstream_dispatch_target.as_deref() == Some("work-pool-pack")
        && !receipt.downstream_dispatch_ready
        && receipt.downstream_dispatch_blockers.iter().any(|value| {
            specification_gate_blockers
                .iter()
                .any(|blocker| value == blocker)
        })
}

fn blocked_implementer_step_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphDispatchReceipt {
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(role_selection, "implementer");
    crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: "implementer".to_string(),
        dispatch_status: receipt
            .downstream_dispatch_status
            .clone()
            .unwrap_or_else(|| "blocked".to_string()),
        lane_status: derive_lane_status(
            receipt
                .downstream_dispatch_status
                .as_deref()
                .unwrap_or("blocked"),
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: Some("vida agent-init".to_string()),
        dispatch_packet_path: None,
        dispatch_result_path: None,
        blocker_code: receipt.blocker_code.clone(),
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
        activation_agent_type,
        activation_runtime_role,
        selected_backend: receipt.selected_backend.clone(),
        recorded_at,
    }
}

pub(crate) async fn try_bridge_bounded_specification_completion_to_downstream_receipt(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if !receipt_waiting_on_specification_evidence(receipt) {
        return Ok(false);
    }

    let Some(result_path) =
        tracked_specification_gate_completion_evidence_path(store, role_selection, receipt).await?
    else {
        return Ok(false);
    };

    receipt.dispatch_status = "executed".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.dispatch_result_path = Some(result_path);
    receipt.blocker_code = None;

    let (next_target, next_command, next_note, next_ready, next_blockers) =
        derive_downstream_dispatch_preview(store, role_selection, receipt).await;
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
    {
        return Err(error);
    }
    if !next_ready {
        return Ok(false);
    }

    let preview_result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        next_target,
        next_command,
        next_note,
        next_ready,
        next_blockers,
        preview_result_path,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        store.root(),
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    receipt.blocker_code = None;
    Ok(true)
}

pub(crate) async fn try_bridge_bounded_implementer_completion_to_downstream_receipt(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if !receipt_waiting_on_implementer_evidence(receipt) {
        return Ok(false);
    }

    let implementer_receipt = blocked_implementer_step_receipt(role_selection, receipt);
    let (next_target, next_command, next_note, next_ready, next_blockers) =
        derive_downstream_dispatch_preview(store, role_selection, &implementer_receipt).await;
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
    {
        return Err(error);
    }
    if !next_ready {
        return Ok(false);
    }

    let preview_result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        next_target,
        next_command,
        next_note,
        next_ready,
        next_blockers,
        preview_result_path,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        store.root(),
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    receipt.blocker_code = None;
    Ok(true)
}

fn receipt_backed_downstream_preview_result_path(
    preview_result_path: Option<String>,
) -> Option<String> {
    preview_result_path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn apply_downstream_dispatch_preview_to_receipt(
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    downstream_dispatch_target: Option<String>,
    downstream_dispatch_command: Option<String>,
    downstream_dispatch_note: Option<String>,
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: Vec<String>,
    preview_result_path: Option<String>,
) {
    let preview_result_path = receipt_backed_downstream_preview_result_path(preview_result_path);
    let closure_lineage_ready =
        downstream_dispatch_target.as_deref().map(str::trim) == Some("closure");
    let downstream_dispatch_ready = downstream_dispatch_ready || closure_lineage_ready;
    let packet_ready = downstream_dispatch_ready
        && downstream_dispatch_blockers.is_empty()
        && preview_result_path.is_some();
    receipt.downstream_dispatch_target = downstream_dispatch_target;
    receipt.downstream_dispatch_command = downstream_dispatch_command;
    receipt.downstream_dispatch_note = downstream_dispatch_note;
    receipt.downstream_dispatch_ready = downstream_dispatch_ready;
    receipt.downstream_dispatch_blockers = downstream_dispatch_blockers;
    receipt.downstream_dispatch_status = packet_ready.then(|| "packet_ready".to_string());
    receipt.downstream_dispatch_result_path = preview_result_path;
    receipt.downstream_dispatch_active_target = active_downstream_dispatch_target(receipt);
}

fn write_runtime_downstream_dispatch_trace(
    state_root: &Path,
    run_id: &str,
    trace: &[serde_json::Value],
) -> Result<String, String> {
    let trace_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-traces");
    std::fs::create_dir_all(&trace_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-traces directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let trace_path = trace_dir.join(format!("{run_id}-{ts}.json"));
    let body = serde_json::json!({
        "artifact_kind": "runtime_downstream_dispatch_trace",
        "run_id": run_id,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "step_count": trace.len(),
        "steps": trace,
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch trace: {error}"))?;
    std::fs::write(&trace_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch trace: {error}"))?;
    Ok(trace_path.display().to_string())
}

pub(crate) fn runtime_dispatch_command_for_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Option<String> {
    match dispatch_target {
        "spec-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"].get("bootstrap_command"),
        ),
        "work-pool-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                .get("ensure_command"),
        ),
        "dev-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                .get("ensure_command"),
        ),
        _ => Some("vida agent-init".to_string()),
    }
}

pub(crate) fn runtime_dispatch_packet_kind(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    dispatch_kind: &str,
) -> String {
    if dispatch_kind == "taskflow_pack" {
        return "tracked_flow_packet".to_string();
    }
    dispatch_contract_lane(execution_plan, dispatch_target)
        .and_then(|lane| json_string(lane.get("packet_template_kind")))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "delivery_task_packet".to_string())
}

pub(crate) async fn derive_downstream_dispatch_preview(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Vec<String>,
) {
    let agent_only_development =
        super::execution_plan_agent_only_development_required(&role_selection.execution_plan);
    let dispatch_contract = &role_selection.execution_plan["development_flow"]["dispatch_contract"];
    let lane_sequence = dispatch_contract_lane_sequence(dispatch_contract);
    let execution_lane_sequence = dispatch_contract_execution_lane_sequence(dispatch_contract);
    match receipt.dispatch_target.as_str() {
        "spec-pack" if agent_only_development => (
            Some(
                lane_sequence
                    .first()
                    .map(|value| value.as_str())
                    .unwrap_or("specification")
                    .to_string(),
            ),
            Some("vida agent-init".to_string()),
            Some(
                "after spec-pack task materialization, dispatch the business-analyst lane for bounded research/specification/planning before work-pool shaping"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
        "spec-pack" => {
            let design_doc_finalized = tracked_design_doc_finalized(role_selection);
            let spec_task_closed =
                tracked_specification_task_closed(store, role_selection, receipt).await;
            let ready = design_doc_finalized && spec_task_closed;
            let mut blockers = Vec::new();
            if !design_doc_finalized {
                blockers.push(
                    blocker_code_value(BlockerCode::PendingDesignFinalize)
                        .expect("pending design finalize should stay registry-backed"),
                );
            }
            if !spec_task_closed {
                blockers.push(
                    blocker_code_value(BlockerCode::PendingSpecTaskClose)
                        .expect("pending spec task close should stay registry-backed"),
                );
            }
            (
                Some("work-pool-pack".to_string()),
                json_string(
                    role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                        .get("ensure_command"),
                ),
                Some(
                    if ready {
                        "design document is finalized and the spec task is closed; ensure or reuse the tracked work-pool packet"
                    } else {
                        "after the design document is finalized and the spec task is closed, ensure or reuse the tracked work-pool packet"
                    }
                        .to_string(),
                ),
                ready,
                blockers,
            )
        }
        "work-pool-pack" => (
            Some("dev-pack".to_string()),
            json_string(
                role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                    .get("ensure_command"),
            ),
            Some(
                "after the work-pool packet is shaped, ensure or reuse the bounded dev packet for delegated implementation"
                    .to_string(),
            ),
            receipt.dispatch_status == "executed",
            if receipt.dispatch_status == "executed" {
                Vec::new()
            } else {
                vec!["pending_work_pool_shape".to_string()]
            },
        ),
        "dev-pack" => {
            let next_target = execution_lane_sequence
                .first()
                .map(|value| value.as_str())
                .unwrap_or("implementer")
                .to_string();
            let missing_owned_scope =
                request_missing_owned_write_scope_for_dispatch_target(
                    store,
                    role_selection,
                    receipt,
                    &next_target,
                )
                .await;
            (
                Some(next_target),
                Some("vida agent-init".to_string()),
                Some(
                    "after the dev packet is created, activate the selected implementer lane for bounded execution"
                        .to_string(),
                ),
                !missing_owned_scope,
                if missing_owned_scope {
                    vec![missing_owned_write_scope_blocker()]
                } else {
                    Vec::new()
                },
            )
        }
        "closure" => (
            None,
            None,
            Some("terminal closure is the active bounded runtime target".to_string()),
            false,
            Vec::new(),
        ),
        _ if receipt.dispatch_kind == "agent_lane" => {
            let current_index = execution_lane_sequence_index_for_target(
                &execution_lane_sequence,
                &receipt.dispatch_target,
                receipt.downstream_dispatch_last_target.as_deref(),
            );
            if current_index.is_some_and(|index| execution_lane_sequence.get(index + 1).is_none()) {
                if dispatch_contract_lane(&role_selection.execution_plan, &receipt.dispatch_target)
                    .is_some_and(|lane| {
                        lane["stage"].as_str() == Some("execution")
                            && lane["closure_class"].as_str() == Some("implementation")
                    })
                {
                    let has_lane_evidence = dispatch_receipt_has_execution_evidence(receipt)
                        || dispatch_receipt_allows_synthetic_lane_completion(receipt)
                        || tracked_implementer_task_closed(store, role_selection, receipt).await;
                    let blocker = dispatch_contract_lane(
                        &role_selection.execution_plan,
                        &receipt.dispatch_target,
                    )
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingLaneEvidence));
                    return (
                        Some("closure".to_string()),
                        None,
                        Some(
                            "no additional downstream lane is required by the current execution plan after this handoff"
                                .to_string(),
                        ),
                        has_lane_evidence,
                        if has_lane_evidence {
                            Vec::new()
                        } else {
                            downstream_preview_blockers_for_missing_lane_evidence(receipt, blocker)
                        },
                    );
                }
            }
            let current_lane =
                dispatch_contract_lane(&role_selection.execution_plan, &receipt.dispatch_target);
            if current_lane.and_then(|lane| lane["stage"].as_str()) == Some("design_gate")
                || (receipt.dispatch_target == "specification"
                    && current_lane.and_then(|lane| lane["stage"].as_str()).is_none()
                    && (dispatch_contract.get("specification_activation").is_some()
                        || role_selection.tracked_flow_entry.as_deref() == Some("spec-pack")))
            {
                let has_specification_evidence = dispatch_receipt_has_execution_evidence(receipt)
                    || tracked_specification_gate_completion_ready(store, role_selection, receipt)
                        .await;
                let spec_task_closed =
                    tracked_specification_task_closed(store, role_selection, receipt).await;
                let design_doc_finalized = tracked_design_doc_finalized(role_selection);
                let evidence_blocker = current_lane
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingSpecificationEvidence));
                return (
                    Some("work-pool-pack".to_string()),
                    json_string(
                        role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                        .get("ensure_command"),
                    ),
                    Some(
                        if receipt.dispatch_status == "executed"
                            && has_specification_evidence
                            && spec_task_closed
                            && design_doc_finalized
                        {
                            "specification/planning evidence is recorded and the spec-pack is closed; ensure or reuse the tracked work-pool packet"
                        } else if receipt.dispatch_status == "executed" {
                            "after specification/planning evidence is recorded, finalize the design doc and close spec-pack before work-pool shaping via tracked work-pool ensure/reuse"
                        } else {
                            "specification/planning lane is active; wait for bounded evidence return before design finalization, spec-pack closure, and tracked work-pool ensure/reuse"
                        }
                        .to_string(),
                    ),
                    has_specification_evidence && spec_task_closed && design_doc_finalized,
                    {
                        let mut blockers = Vec::new();
                        if !has_specification_evidence {
                            blockers.push(evidence_blocker.to_string());
                        }
                        if !spec_task_closed {
                            blockers.push(
                                blocker_code_value(BlockerCode::PendingSpecTaskClose)
                                    .expect("pending spec task close should stay registry-backed"),
                            );
                        }
                        if !design_doc_finalized {
                            blockers.push(
                                blocker_code_value(BlockerCode::PendingDesignFinalize).expect(
                                    "pending design finalize should stay registry-backed",
                                ),
                            );
                        }
                        blockers
                    },
                );
            }
            if matches!(
                receipt.dispatch_status.as_str(),
                "routed" | "executing"
            ) {
                return (
                    None,
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "`{}` dispatch is in flight; wait for terminal execution evidence before deriving downstream lane blockers",
                        receipt.dispatch_target
                    )),
                    false,
                    Vec::new(),
                );
            }
            let implementation = &role_selection.execution_plan["development_flow"]["implementation"];
            let analysis_target = implementation
                .get("analysis_route_task_class")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("analysis");
            if receipt.dispatch_target == analysis_target
                && (dispatch_receipt_has_execution_evidence(receipt)
                    || dispatch_receipt_allows_synthetic_lane_completion(receipt))
            {
                let writer_target = implementation
                    .get("writer_route_task_class")
                    .or_else(|| implementation.get("implementer_route_task_class"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("writer");
                let missing_owned_scope = request_missing_owned_write_scope_for_dispatch_target(
                    store,
                    role_selection,
                    receipt,
                    writer_target,
                )
                .await;
                return (
                    Some(writer_target.to_string()),
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "after `{}` validation evidence is recorded, activate `{}` for the first implementation lane",
                        receipt.dispatch_target, writer_target
                    )),
                    !missing_owned_scope,
                    if missing_owned_scope {
                        vec![missing_owned_write_scope_blocker()]
                    } else {
                        Vec::new()
                    },
                );
            }
            let effective_current_target = current_index
                .map(|_| receipt.dispatch_target.clone())
                .or_else(|| {
                    receipt
                        .activation_runtime_role
                        .as_deref()
                        .and_then(|runtime_role| {
                            dispatch_target_for_runtime_role(
                                &role_selection.execution_plan,
                                runtime_role,
                            )
                        })
                });
            let current_index = current_index.or_else(|| {
                receipt
                    .activation_runtime_role
                    .as_deref()
                    .and_then(|runtime_role| {
                        dispatch_target_for_runtime_role(
                            &role_selection.execution_plan,
                            runtime_role,
                        )
                    })
                    .and_then(|target| {
                        execution_lane_sequence_index_for_target(
                            &execution_lane_sequence,
                            &target,
                            receipt.downstream_dispatch_last_target.as_deref(),
                        )
                    })
            });
            let Some(current_index) = current_index else {
                return (None, None, None, false, Vec::new());
            };
            let next_target = execution_lane_sequence.get(current_index + 1);
            if let Some(next_target) = next_target {
                let blocker = effective_current_target
                    .as_deref()
                    .and_then(|target| dispatch_contract_lane(&role_selection.execution_plan, target))
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingLaneEvidence))
                    .to_string();
                let has_lane_evidence = dispatch_receipt_has_execution_evidence(receipt)
                    || dispatch_receipt_allows_synthetic_lane_completion(receipt)
                    || tracked_implementer_task_closed(store, role_selection, receipt).await;
                let missing_owned_scope = request_missing_owned_write_scope_for_dispatch_target(
                    store,
                    role_selection,
                    receipt,
                    next_target,
                )
                .await;
                (
                    Some(next_target.clone()),
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "after `{}` evidence is recorded, activate `{}` for the next bounded lane",
                        receipt.dispatch_target, next_target
                    )),
                    has_lane_evidence && !missing_owned_scope,
                    {
                        let mut blockers = Vec::new();
                        if !has_lane_evidence {
                            blockers.extend(
                                downstream_preview_blockers_for_missing_lane_evidence(
                                    receipt,
                                    &blocker,
                                ),
                            );
                        }
                        if missing_owned_scope {
                            blockers.push(missing_owned_write_scope_blocker());
                        }
                        blockers
                    },
                )
            } else {
                let blocker = effective_current_target
                    .as_deref()
                    .and_then(|target| dispatch_contract_lane(&role_selection.execution_plan, target))
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingLaneEvidence));
                let has_lane_evidence = dispatch_receipt_has_execution_evidence(receipt)
                    || dispatch_receipt_allows_synthetic_lane_completion(receipt)
                    || tracked_implementer_task_closed(store, role_selection, receipt).await;
                (
                    Some("closure".to_string()),
                    None,
                    Some(
                        "no additional downstream lane is required by the current execution plan after this handoff"
                            .to_string(),
                    ),
                    has_lane_evidence,
                    if has_lane_evidence {
                        Vec::new()
                    } else {
                        downstream_preview_blockers_for_missing_lane_evidence(receipt, &blocker)
                    },
                )
            }
        }
        _ => (None, None, None, false, Vec::new()),
    }
}

fn execution_lane_sequence_index_for_target(
    execution_lane_sequence: &[String],
    target: &str,
    previous_target: Option<&str>,
) -> Option<usize> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    let previous_target = previous_target
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(previous_target) = previous_target {
        let target_count = execution_lane_sequence
            .iter()
            .filter(|candidate| candidate.as_str() == target)
            .count();
        if target_count > 1 {
            if let Some(index) =
                execution_lane_sequence
                    .iter()
                    .enumerate()
                    .find_map(|(index, candidate)| {
                        (candidate.as_str() == target
                            && index > 0
                            && execution_lane_sequence
                                .get(index - 1)
                                .is_some_and(|previous| previous.as_str() == previous_target))
                        .then_some(index)
                    })
            {
                return Some(index);
            }
        }
    }

    execution_lane_sequence
        .iter()
        .position(|candidate| candidate.as_str() == target)
}

pub(crate) fn downstream_dispatch_ready_blocker_parity_error(
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: &[String],
) -> Option<String> {
    if downstream_dispatch_ready && !downstream_dispatch_blockers.is_empty() {
        return Some(
            "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                .to_string(),
        );
    }
    None
}

pub(crate) async fn refresh_downstream_dispatch_preview(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    refresh_downstream_dispatch_preview_with_owned_paths(
        store,
        role_selection,
        run_graph_bootstrap,
        receipt,
        &[],
    )
    .await
}

pub(crate) async fn refresh_downstream_dispatch_preview_with_owned_paths(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    implementation_owned_paths_override: &[String],
) -> Result<(), String> {
    let (
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
    ) = derive_downstream_dispatch_preview(store, role_selection, receipt).await;
    let mut downstream_dispatch_ready = downstream_dispatch_ready;
    let mut downstream_dispatch_blockers = downstream_dispatch_blockers;
    let mut implementation_owned_paths = Vec::new();
    append_unique_explicit_owned_scope_paths(
        &mut implementation_owned_paths,
        implementation_owned_paths_override,
    );
    if implementation_owned_paths.is_empty() {
        implementation_owned_paths =
            implementation_owned_paths_for_dispatch_context(store, role_selection, receipt).await;
    }
    if !implementation_owned_paths.is_empty()
        && downstream_dispatch_blockers
            .iter()
            .any(|blocker| blocker == "missing_owned_write_scope")
    {
        downstream_dispatch_blockers.retain(|blocker| blocker != "missing_owned_write_scope");
        if downstream_dispatch_blockers.is_empty() {
            downstream_dispatch_ready = true;
        }
    }
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let preview_result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
        preview_result_path,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_last_target = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_packet_path =
        if receipt.downstream_dispatch_status.as_deref() == Some("packet_ready") {
            write_runtime_downstream_dispatch_packet_with_owned_paths(
                store.root(),
                role_selection,
                run_graph_bootstrap,
                receipt,
                &implementation_owned_paths,
            )?
        } else {
            None
        };
    Ok(())
}

pub(crate) fn runtime_packet_handoff_task_class(
    dispatch_target: &str,
    handoff_runtime_role: &str,
) -> &'static str {
    match dispatch_target {
        "specification" => TASK_CLASS_SPECIFICATION,
        "analysis" => "analysis",
        "planning" => "planning",
        "coach" => TASK_CLASS_COACH,
        "verification" => TASK_CLASS_VERIFICATION,
        "escalation" => TASK_CLASS_ARCHITECTURE,
        "implementer" | "writer" => TASK_CLASS_IMPLEMENTATION,
        "orchestrator" => "analysis",
        _ => match handoff_runtime_role {
            RUNTIME_ROLE_BUSINESS_ANALYST => TASK_CLASS_SPECIFICATION,
            RUNTIME_ROLE_PM => "planning",
            RUNTIME_ROLE_COACH => TASK_CLASS_COACH,
            RUNTIME_ROLE_VERIFIER => TASK_CLASS_VERIFICATION,
            RUNTIME_ROLE_SOLUTION_ARCHITECT => TASK_CLASS_ARCHITECTURE,
            _ => TASK_CLASS_IMPLEMENTATION,
        },
    }
}

fn packet_nonempty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

fn packet_nonempty_string_array(packet: &serde_json::Value, key: &str) -> bool {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                })
        })
}

fn packet_string_array_is_runtime_consumption_fallback(
    packet: &serde_json::Value,
    key: &str,
) -> bool {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            rows.len() == 1
                && rows[0]
                    .as_str()
                    .map(str::trim)
                    .is_some_and(is_runtime_consumption_fallback_owned_path)
        })
}

fn packet_has_concrete_owned_paths(packet: &serde_json::Value) -> bool {
    packet_nonempty_string_array(packet, "owned_paths")
        && !packet_string_array_is_runtime_consumption_fallback(packet, "owned_paths")
}

pub(crate) fn runtime_dispatch_packet_has_concrete_owned_paths(packet: &serde_json::Value) -> bool {
    packet_has_concrete_owned_paths(packet)
        || active_runtime_packet(packet)
            .ok()
            .is_some_and(|(_, active_packet)| packet_has_concrete_owned_paths(active_packet))
}

fn packet_string_array(packet: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                })
                .collect::<Option<Vec<_>>>()
        })
        .flatten()
}

fn packet_has_owned_or_read_only_paths(packet: &serde_json::Value) -> bool {
    packet_nonempty_string_array(packet, "owned_paths")
        || packet_nonempty_string_array(packet, "read_only_paths")
}

fn packet_requires_owned_write_scope(
    packet_template_kind: &str,
    active_packet: &serde_json::Value,
) -> bool {
    if packet_template_kind != "delivery_task_packet" {
        return false;
    }
    active_packet
        .get("handoff_task_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some("implementation")
}

fn dispatch_target_requires_owned_write_scope(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> bool {
    let (_, _, _, activation_runtime_role) =
        downstream_activation_fields(role_selection, dispatch_target);
    let handoff_runtime_role = activation_runtime_role
        .as_deref()
        .unwrap_or(role_selection.selected_role.as_str());
    runtime_packet_handoff_task_class(dispatch_target, handoff_runtime_role) == "implementation"
}

fn missing_owned_write_scope_blocker() -> String {
    "missing_owned_write_scope".to_string()
}

pub(crate) fn planner_metadata_owned_paths_from_role_selection(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Vec<String> {
    let mut owned_paths = Vec::new();
    for value in role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
        ["planner_metadata"]["owned_paths"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(path) = value
            .as_str()
            .and_then(normalize_safe_owned_scope_path_candidate)
        else {
            continue;
        };
        if !owned_paths.iter().any(|existing| existing == &path) {
            owned_paths.push(path);
        }
    }
    owned_paths
}

pub(crate) fn implementation_owned_paths_for_role_selection(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Vec<String> {
    let derived_paths = delivery_packet_owned_paths(
        TASK_CLASS_IMPLEMENTATION,
        &role_selection.request,
        tracked_design_doc_path(role_selection),
    );
    if derived_paths.is_empty() {
        planner_metadata_owned_paths_from_role_selection(role_selection)
    } else {
        derived_paths
    }
}

fn append_unique_owned_paths(target: &mut Vec<String>, source: &[String]) {
    for path in source {
        let Some(normalized) = normalize_safe_owned_scope_path_candidate(path) else {
            continue;
        };
        if !target.iter().any(|existing| existing == &normalized) {
            target.push(normalized);
        }
    }
}

fn append_unique_explicit_owned_scope_paths(target: &mut Vec<String>, source: &[String]) {
    for path in source {
        let normalized = path.trim().trim_end_matches('/').to_string();
        if normalized.is_empty()
            || !normalized.contains('/')
            || normalized.starts_with('/')
            || normalized.starts_with("./")
            || normalized.starts_with("../")
            || normalized
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
        {
            continue;
        }
        if !target.iter().any(|existing| existing == &normalized) {
            target.push(normalized);
        }
    }
}

async fn planner_metadata_owned_paths_from_task(store: &StateStore, task_id: &str) -> Vec<String> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Vec::new();
    }
    store
        .show_task(task_id)
        .await
        .map(|task| {
            task.planner_metadata
                .owned_paths
                .into_iter()
                .filter_map(|path| normalize_safe_owned_scope_path_candidate(&path))
                .collect()
        })
        .unwrap_or_default()
}

async fn implementation_owned_paths_for_dispatch_context(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Vec<String> {
    let mut owned_paths = implementation_owned_paths_for_role_selection(role_selection);
    owned_paths.retain(|path| !is_runtime_consumption_fallback_owned_path(path));
    if !owned_paths.is_empty() {
        return owned_paths;
    }
    if let Some(task_id) = tracked_implementer_dev_task_id(role_selection) {
        let task_paths = planner_metadata_owned_paths_from_task(store, task_id).await;
        append_unique_owned_paths(&mut owned_paths, &task_paths);
    }
    let task_paths = planner_metadata_owned_paths_from_task(store, &receipt.run_id).await;
    append_unique_owned_paths(&mut owned_paths, &task_paths);
    owned_paths
}

pub(crate) fn apply_owned_paths_if_missing(
    packet: &mut serde_json::Value,
    owned_paths: &[String],
) -> bool {
    let concrete_owned_paths: Vec<String> = owned_paths
        .iter()
        .filter(|path| !is_runtime_consumption_fallback_owned_path(path))
        .cloned()
        .collect();
    if concrete_owned_paths.is_empty() {
        return false;
    }
    if packet_nonempty_string_array(packet, "owned_paths")
        && !packet_string_array_is_runtime_consumption_fallback(packet, "owned_paths")
    {
        return false;
    }
    let Some(object) = packet.as_object_mut() else {
        return false;
    };
    object.insert(
        "owned_paths".to_string(),
        serde_json::json!(concrete_owned_paths),
    );
    true
}

pub(crate) fn clear_runtime_consumption_fallback_owned_paths(
    packet: &mut serde_json::Value,
) -> bool {
    if !packet_string_array_is_runtime_consumption_fallback(packet, "owned_paths") {
        return false;
    }
    let Some(object) = packet.as_object_mut() else {
        return false;
    };
    object.insert("owned_paths".to_string(), serde_json::json!([]));
    true
}

async fn request_missing_owned_write_scope_for_dispatch_target(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    dispatch_target: &str,
) -> bool {
    dispatch_target_requires_owned_write_scope(role_selection, dispatch_target)
        && !request_has_explicit_owned_scope(&role_selection.request)
        && implementation_owned_paths_for_dispatch_context(store, role_selection, receipt)
            .await
            .is_empty()
}

fn single_task_move_scope_owned_paths(packet: &serde_json::Value) -> Option<Vec<String>> {
    let single_task_only = packet
        .get("role_selection_full")
        .and_then(|value| value.get("single_task_only"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !single_task_only {
        return None;
    }
    let request_text = packet
        .get("request_text")
        .and_then(serde_json::Value::as_str)?;
    single_task_move_scope_paths(request_text)
}

fn packet_tracked_design_doc_path<'a>(packet: &'a serde_json::Value) -> Option<&'a str> {
    packet["role_selection_full"]["execution_plan"]["tracked_flow_bootstrap"]["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn packet_request_text<'a>(packet: &'a serde_json::Value) -> Option<&'a str> {
    packet
        .get("request_text")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("request_text"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

fn active_runtime_packet<'a>(
    packet: &'a serde_json::Value,
) -> Result<(&'a str, &'a serde_json::Value), String> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted dispatch packet is missing packet_template_kind".to_string())?;
    let packet_value = packet.get(packet_template_kind).ok_or_else(|| {
        format!("Persisted dispatch packet is missing active packet body `{packet_template_kind}`")
    })?;
    if packet_value.is_null() {
        return Err(format!(
            "Persisted dispatch packet has null active packet body `{packet_template_kind}`"
        ));
    }
    Ok((packet_template_kind, packet_value))
}

pub(crate) fn validate_runtime_dispatch_packet_contract(
    packet: &serde_json::Value,
    packet_label: &str,
) -> Result<(), String> {
    let (packet_template_kind, active_packet) = active_runtime_packet(packet)?;
    for key in ["owned_paths", "read_only_paths"] {
        if let (Some(top_level), Some(active)) = (
            packet_string_array(packet, key),
            packet_string_array(active_packet, key),
        ) {
            if top_level != active {
                return Err(format!(
                    "{packet_label} `{packet_template_kind}` top-level {key} must mirror the active packet body; expected {:?}, got {:?}",
                    active, top_level
                ));
            }
        }
    }
    if let Some(expected_owned_paths) = single_task_move_scope_owned_paths(packet) {
        let actual_owned_paths = active_packet
            .get("owned_paths")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                format!(
                    "{packet_label} `{packet_template_kind}` is missing owned_paths for a single-task move request"
                )
            })?;
        let actual_owned_paths = actual_owned_paths
            .iter()
            .map(|value| {
                value.as_str().map(str::trim).filter(|value| !value.is_empty()).map(|value| {
                    value.to_string()
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                format!(
                    "{packet_label} `{packet_template_kind}` contains non-string owned_paths entries for a single-task move request"
                )
            })?;
        if actual_owned_paths != expected_owned_paths {
            return Err(format!(
                "{packet_label} `{packet_template_kind}` single-task move packet owned_paths must match the declared source/destination pair exactly; expected {:?}, got {:?}",
                expected_owned_paths, actual_owned_paths
            ));
        }
    }
    if packet_template_kind == "delivery_task_packet" {
        let handoff_task_class = active_packet
            .get("handoff_task_class")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if handoff_task_class == TASK_CLASS_SPECIFICATION {
            let expected_owned_paths = delivery_packet_owned_paths(
                handoff_task_class,
                packet_request_text(packet).unwrap_or_default(),
                packet_tracked_design_doc_path(packet),
            );
            let actual_owned_paths = active_packet
                .get("owned_paths")
                .and_then(serde_json::Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .map(|value| {
                            value
                                .as_str()
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(str::to_string)
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .flatten()
                .unwrap_or_default();
            if actual_owned_paths != expected_owned_paths {
                return Err(format!(
                    "{packet_label} `{packet_template_kind}` specification packet owned_paths must match the tracked design-doc scope exactly; expected {:?}, got {:?}",
                    expected_owned_paths, actual_owned_paths
                ));
            }
        }
    }
    let missing = match packet_template_kind {
        "delivery_task_packet" | "execution_block_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("goal")) {
                missing.push("goal");
            }
            if !packet_nonempty_string_array(active_packet, "scope_in") {
                missing.push("scope_in");
            }
            if packet_requires_owned_write_scope(packet_template_kind, active_packet) {
                if !packet_has_concrete_owned_paths(active_packet) {
                    missing.push("owned_paths");
                }
            } else if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string_array(active_packet, "definition_of_done") {
                missing.push("definition_of_done");
            }
            if !packet_nonempty_string(active_packet.get("verification_command")) {
                missing.push("verification_command");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_nonempty_string_array(active_packet, "stop_rules") {
                missing.push("stop_rules");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "coach_review_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("review_goal")) {
                missing.push("review_goal");
            }
            if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string_array(active_packet, "definition_of_done") {
                missing.push("definition_of_done");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "verifier_proof_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("proof_goal")) {
                missing.push("proof_goal");
            }
            if !packet_nonempty_string(active_packet.get("verification_command")) {
                missing.push("verification_command");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "escalation_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("decision_needed")) {
                missing.push("decision_needed");
            }
            if !packet_nonempty_string_array(active_packet, "options") {
                missing.push("options");
            }
            if !packet_nonempty_string_array(active_packet, "constraints") {
                missing.push("constraints");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "tracked_flow_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("dispatch_target")) {
                missing.push("dispatch_target");
            }
            if !packet_nonempty_string(active_packet.get("tracked_packet_key")) {
                missing.push("tracked_packet_key");
            }
            if !packet_nonempty_string(active_packet.get("task_id")) {
                missing.push("task_id");
            }
            if !packet_nonempty_string(active_packet.get("title")) {
                missing.push("title");
            }
            if !packet_nonempty_string(active_packet.get("runtime")) {
                missing.push("runtime");
            }
            if !packet_nonempty_string(active_packet.get("create_command")) {
                missing.push("create_command");
            }
            if !packet_nonempty_string(active_packet.get("ensure_command")) {
                missing.push("ensure_command");
            }
            if !packet_nonempty_string(active_packet.get("next_command")) {
                missing.push("next_command");
            }
            missing
        }
        other => {
            return Err(format!(
                "Persisted dispatch packet has unsupported packet_template_kind `{other}`"
            ));
        }
    };
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{packet_label} `{packet_template_kind}` is missing required packet fields: {}",
        missing.join(", ")
    ))
}

fn runtime_dispatch_command_for_packet_path(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: &str,
    preferred_backend: Option<&str>,
) -> Option<String> {
    match receipt.dispatch_kind.as_str() {
        "taskflow_pack" => {
            runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target)
        }
        "agent_lane" => Some({
            let preferred_model_profile_id = preferred_selected_model_profile_for_dispatch_target(
                role_selection,
                &receipt.dispatch_target,
                preferred_backend,
            );
            runtime_agent_lane_dispatch_for_root(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                packet_path,
                preferred_backend,
                preferred_model_profile_id.as_deref(),
            )
            .activation_command
        })
        .or_else(|| receipt.dispatch_command.clone())
        .or_else(|| runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target)),
        _ => runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target),
    }
}

pub(crate) struct RuntimeDispatchPacketContext<'a> {
    pub(crate) state_root: &'a Path,
    pub(crate) role_selection: &'a RuntimeConsumptionLaneSelection,
    pub(crate) receipt: &'a crate::state_store::RunGraphDispatchReceipt,
    pub(crate) taskflow_handoff_plan: &'a serde_json::Value,
    pub(crate) run_graph_bootstrap: &'a serde_json::Value,
    pub(crate) selected_backend_override: Option<String>,
}

impl<'a> RuntimeDispatchPacketContext<'a> {
    pub(crate) fn new(
        state_root: &'a Path,
        role_selection: &'a RuntimeConsumptionLaneSelection,
        receipt: &'a crate::state_store::RunGraphDispatchReceipt,
        taskflow_handoff_plan: &'a serde_json::Value,
        run_graph_bootstrap: &'a serde_json::Value,
    ) -> Self {
        Self {
            state_root,
            role_selection,
            receipt,
            taskflow_handoff_plan,
            run_graph_bootstrap,
            selected_backend_override: None,
        }
    }

    pub(crate) fn with_selected_backend_override(
        mut self,
        selected_backend_override: Option<String>,
    ) -> Self {
        self.selected_backend_override = selected_backend_override
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self
    }
}

fn build_runtime_dispatch_packet_body(
    ctx: &RuntimeDispatchPacketContext<'_>,
    dispatch_command: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(ctx.state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve project root for dispatch packet rendering: {error}")
        })?);
    let host_runtime = runtime_host_execution_contract_for_root(&project_root);
    let selected_backend_override = current_selected_backend_override(
        ctx.role_selection,
        &ctx.receipt.dispatch_target,
        ctx.selected_backend_override.as_deref(),
    );
    let receipt_selected_backend = ctx
        .receipt
        .selected_backend
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let runtime_assignment_selected_backend = runtime_assignment_selected_backend_for_target(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
    );
    let canonical_selected_backend = selected_backend_override
        .map(str::to_string)
        .or_else(|| canonical_selected_backend_for_receipt(ctx.role_selection, ctx.receipt))
        .or(runtime_assignment_selected_backend)
        .or_else(|| receipt_selected_backend.map(str::to_string));
    let posture_selected_backend = canonical_selected_backend
        .as_deref()
        .or(receipt_selected_backend);
    let effective_execution_posture = effective_execution_posture_summary(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
        posture_selected_backend,
        ctx.receipt.activation_agent_type.as_deref(),
        Some(&host_runtime),
        false,
        selected_backend_override,
    );
    let packet_template_kind = runtime_dispatch_packet_kind(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
        &ctx.receipt.dispatch_kind,
    );
    let handoff_runtime_role = ctx
        .receipt
        .activation_runtime_role
        .as_deref()
        .unwrap_or(ctx.role_selection.selected_role.as_str());
    let handoff_task_class =
        runtime_packet_handoff_task_class(&ctx.receipt.dispatch_target, handoff_runtime_role);
    let closure_class = dispatch_contract_lane(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
    )
    .and_then(|lane| lane["closure_class"].as_str())
    .unwrap_or("implementation");
    let mut execution_truth = dispatch_execution_route_summary(
        ctx.role_selection,
        &ctx.receipt.dispatch_target,
        posture_selected_backend,
        selected_backend_override,
    );
    if execution_truth["selected_backend_source"].as_str() == Some("dynamic_runtime_selection")
        && execution_truth["effective_selected_backend"].as_str()
            == execution_truth["route_fallback_backend"].as_str()
        && execution_truth["route_primary_backend"].as_str()
            != execution_truth["route_fallback_backend"].as_str()
    {
        if let Some(object) = execution_truth.as_object_mut() {
            object.insert(
                "selected_backend_source".to_string(),
                serde_json::json!("route_fallback_hint"),
            );
            object.insert(
                "backend_selection_source".to_string(),
                serde_json::json!("route_fallback_hint"),
            );
        }
    }
    let (runtime_assignment, runtime_assignment_source) = dispatch_target_runtime_assignment(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
    );
    let activation_evidence = dispatch_activation_evidence_summary(ctx.receipt);
    let mut delivery_task_packet = runtime_delivery_task_packet_with_scope_context(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &ctx.role_selection.request,
        tracked_design_doc_path(ctx.role_selection),
    );
    if handoff_task_class == TASK_CLASS_IMPLEMENTATION {
        let owned_paths = implementation_owned_paths_for_role_selection(ctx.role_selection);
        if !apply_owned_paths_if_missing(&mut delivery_task_packet, &owned_paths) {
            clear_runtime_consumption_fallback_owned_paths(&mut delivery_task_packet);
        }
    }
    let execution_block_packet = runtime_execution_block_packet(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    let mut packet = serde_json::json!({
        "packet_kind": "runtime_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet.clone()
        } else {
            serde_json::Value::Null
        },
        "execution_block_packet": if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
        "coach_review_packet": if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                None,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&ctx.receipt.run_id, &ctx.receipt.dispatch_target)
        } else {
            serde_json::Value::Null
        },
        "tracked_flow_packet": if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(
                ctx.role_selection,
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
            )
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &ctx.receipt.run_id,
            &ctx.receipt.dispatch_target,
            handoff_runtime_role,
            &ctx.role_selection.request,
            &ctx.role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": ctx.receipt.recorded_at,
        "run_id": ctx.receipt.run_id,
        "dispatch_target": ctx.receipt.dispatch_target,
        "dispatch_status": ctx.receipt.dispatch_status,
        "lane_status": ctx.receipt.lane_status,
        "blocker_code": ctx.receipt.blocker_code,
        "supersedes_receipt_id": ctx.receipt.supersedes_receipt_id,
        "exception_path_receipt_id": ctx.receipt.exception_path_receipt_id,
        "dispatch_kind": ctx.receipt.dispatch_kind,
        "dispatch_surface": ctx.receipt.dispatch_surface,
        "dispatch_command": dispatch_command,
        "activation_agent_type": ctx.receipt.activation_agent_type,
        "activation_runtime_role": ctx.receipt.activation_runtime_role,
        "selected_backend": canonical_selected_backend,
        "selected_backend_override": selected_backend_override,
        "mixed_posture": effective_execution_posture.clone(),
        "route_policy": execution_truth.clone(),
        "activation_vs_execution_evidence": activation_evidence.clone(),
        "activation_semantics": activation_evidence["activation_semantics"].clone(),
        "execution_evidence": activation_evidence["execution_evidence"].clone(),
        "effective_execution_posture": effective_execution_posture,
        "execution_truth": execution_truth,
        "activation_evidence": activation_evidence,
        "host_runtime": host_runtime,
        "request_text": ctx.role_selection.request,
        "role_selection": {
            "selected_role": ctx.role_selection.selected_role,
            "conversational_mode": ctx.role_selection.conversational_mode,
            "tracked_flow_entry": ctx.role_selection.tracked_flow_entry,
            "confidence": ctx.role_selection.confidence,
        },
        "role_selection_full": ctx.role_selection,
        "taskflow_handoff_plan": ctx.taskflow_handoff_plan,
        "run_graph_bootstrap": ctx.run_graph_bootstrap,
        "execution_preparation_artifacts": ctx.run_graph_bootstrap
            .get("execution_preparation_artifacts")
            .or_else(|| ctx.taskflow_handoff_plan.get("execution_preparation_artifacts"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "orchestration_contract": ctx.role_selection.execution_plan["orchestration_contract"],
    });
    if let Some(object) = packet.as_object_mut() {
        object.insert("runtime_assignment".to_string(), runtime_assignment.clone());
        object.insert("carrier_runtime_assignment".to_string(), runtime_assignment);
        object.insert(
            "runtime_assignment_source".to_string(),
            serde_json::Value::String(runtime_assignment_source.to_string()),
        );
    }
    Ok(packet)
}

pub(crate) fn runtime_dispatch_packet_preview(
    ctx: &RuntimeDispatchPacketContext<'_>,
) -> Result<serde_json::Value, String> {
    let dispatch_command =
        runtime_dispatch_command_for_target(ctx.role_selection, &ctx.receipt.dispatch_target);
    let packet = build_runtime_dispatch_packet_body(ctx, dispatch_command)?;
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let active_packet = packet
        .get(packet_template_kind.as_str())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let validation_error =
        validate_runtime_dispatch_packet_contract(&packet, "Runtime dispatch packet preview").err();
    let packet_contract_missing_fields = validation_error
        .as_deref()
        .and_then(|error| error.split("is missing required packet fields: ").nth(1))
        .map(|fields| {
            fields
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(serde_json::json!({
        "status": if validation_error.is_some() { "blocked" } else { "pass" },
        "packet_template_kind": packet_template_kind,
        "packet_contract_missing_fields": packet_contract_missing_fields,
        "contract_validation_error": validation_error,
        "owned_paths": active_packet
            .get("owned_paths")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "read_only_paths": active_packet
            .get("read_only_paths")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "packet": packet,
    }))
}

pub(crate) async fn preview_downstream_dispatch_receipt(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let (
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
    ) = derive_downstream_dispatch_preview(store, role_selection, receipt).await;
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
        None,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_last_target = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_packet_path = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::CreateTaskRequest;
    use crate::state_store::RunGraphDispatchReceipt;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::guard_current_dir;
    use crate::{run, Cli};
    use clap::Parser;
    use serde_json::json;
    use std::cell::Cell;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

    fn harness_state_root(harness: &TempStateHarness) -> PathBuf {
        harness.path().join(crate::state_store::default_state_dir())
    }

    fn set_yaml_u64(root: &mut serde_yaml::Value, path: &[&str], value: u64) {
        let Some((last, parents)) = path.split_last() else {
            panic!("yaml path must be non-empty");
        };
        let mut cursor = root;
        for key in parents {
            let mapping = cursor
                .as_mapping_mut()
                .unwrap_or_else(|| panic!("yaml path `{key}` parent is not a mapping"));
            cursor = mapping
                .get_mut(serde_yaml::Value::String((*key).to_string()))
                .unwrap_or_else(|| panic!("yaml path `{key}` is missing"));
        }
        let mapping = cursor
            .as_mapping_mut()
            .unwrap_or_else(|| panic!("yaml path `{last}` parent is not a mapping"));
        mapping.insert(
            serde_yaml::Value::String((*last).to_string()),
            serde_yaml::to_value(value).expect("u64 yaml value should serialize"),
        );
    }

    #[test]
    fn current_project_model_profile_catalog_prefers_active_project_root_over_static_root() {
        let root = std::env::temp_dir().join(format!(
            "vida-current-project-model-catalog-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join(".vida/config")).expect("config dir");
        fs::create_dir_all(root.join(".vida/db")).expect("db dir");
        fs::create_dir_all(root.join(".vida/project")).expect("project dir");
        fs::write(root.join("AGENTS.md"), "test").expect("agents");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  codex:
    agents:
      junior:
        default_model_profile: codex_gpt55_low_write
        model_profiles:
          codex_gpt55_low_write:
            provider: openai-codex
            model_ref: openai-codex/gpt-5.5
"#,
        )
        .expect("config");

        {
            let _cwd = guard_current_dir(&root);
            let catalog =
                current_project_model_profile_catalog_for_root(&crate::state_store::repo_root());
            let refs = catalog
                .get("codex_gpt55_low_write")
                .expect("active project profile should be collected");

            assert!(refs.contains("openai-codex/gpt-5.5"));
        }

        let _ = fs::remove_dir_all(&root);
    }

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn set(path: PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    struct HarnessStateRootGuards {
        _proxy_override: ProxyStateDirOverrideGuard,
        _env_guard: EnvVarGuard,
    }

    impl HarnessStateRootGuards {
        fn set(path: PathBuf) -> Self {
            let env_value = path.display().to_string();
            Self {
                _proxy_override: ProxyStateDirOverrideGuard::set(path),
                _env_guard: EnvVarGuard::set("VIDA_STATE_DIR", &env_value),
            }
        }
    }

    struct EnvVarGuard {
        lock: Option<MutexGuard<'static, ()>>,
        key: &'static str,
        original: Option<String>,
    }

    struct RecoveringMutex(Mutex<()>);

    impl RecoveringMutex {
        fn lock(&self) -> MutexGuard<'_, ()> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    fn env_var_lock() -> &'static RecoveringMutex {
        static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
        LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
    }

    thread_local! {
        static ENV_VAR_GUARD_DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let lock = ENV_VAR_GUARD_DEPTH.with(|depth| {
                let current = depth.get();
                depth.set(current + 1);
                (current == 0).then(|| env_var_lock().lock())
            });
            let original = env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                lock,
                key,
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = self.original.as_deref() {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
            ENV_VAR_GUARD_DEPTH.with(|depth| {
                let current = depth.get();
                depth.set(current.saturating_sub(1));
            });
            let _ = self.lock.take();
        }
    }

    fn cli(args: &[&str]) -> Cli {
        let mut argv = vec!["vida"];
        argv.extend(args.iter().copied());
        Cli::parse_from(argv)
    }

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn run_on_large_test_stack(name: &str, test: impl FnOnce() + Send + 'static) {
        let handle = thread::Builder::new()
            .name(name.to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(test)
            .expect("large-stack test thread should spawn");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    }

    fn fake_codex_path(fake_bin: &Path) -> PathBuf {
        #[cfg(windows)]
        {
            fake_bin.join("codex.ps1")
        }
        #[cfg(not(windows))]
        {
            fake_bin.join("codex")
        }
    }

    fn prepend_to_path(path: &Path) -> String {
        let original_path = env::var_os("PATH");
        let paths = original_path
            .as_deref()
            .map(env::split_paths)
            .into_iter()
            .flatten();
        env::join_paths(std::iter::once(path.to_path_buf()).chain(paths))
            .expect("test PATH should join")
            .to_string_lossy()
            .into_owned()
    }

    fn make_fake_codex_executable(_path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(_path)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(_path, perms).expect("fake codex should be executable");
        }
    }

    fn configure_fake_codex_dispatch(project_root: &Path, fake_codex: &Path) {
        let config_path = project_root.join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config should parse as yaml");
        let root = document
            .as_mapping_mut()
            .expect("config root should be a yaml mapping");
        let host_environment = root
            .get_mut(serde_yaml::Value::String("host_environment".to_string()))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("host_environment should exist");
        let systems = host_environment
            .get_mut(serde_yaml::Value::String("systems".to_string()))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("host systems should exist");
        let codex = systems
            .get_mut(serde_yaml::Value::String("codex".to_string()))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("codex system should exist");
        let dispatch = codex
            .get_mut(serde_yaml::Value::String("dispatch".to_string()))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("codex dispatch config should exist");
        let original_static_args = crate::yaml_string_list(
            dispatch.get(serde_yaml::Value::String("static_args".to_string())),
        );
        #[cfg(windows)]
        let (command, mut static_args) = (
            "pwsh".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                fake_codex.display().to_string(),
            ],
        );
        #[cfg(not(windows))]
        let (command, mut static_args) = (fake_codex.display().to_string(), Vec::<String>::new());
        static_args.extend(original_static_args);
        dispatch.insert(
            serde_yaml::Value::String("command".to_string()),
            serde_yaml::Value::String(command),
        );
        dispatch.insert(
            serde_yaml::Value::String("static_args".to_string()),
            serde_yaml::Value::Sequence(
                static_args
                    .into_iter()
                    .map(serde_yaml::Value::String)
                    .collect(),
            ),
        );
        dispatch.insert(
            serde_yaml::Value::String("receipt_backed_completion_supported".to_string()),
            serde_yaml::Value::Bool(true),
        );
        dispatch.insert(
            serde_yaml::Value::String("windows_sandbox_spawn_supported".to_string()),
            serde_yaml::Value::Bool(true),
        );
        fs::write(
            &config_path,
            serde_yaml::to_string(&document).expect("config should serialize as yaml"),
        )
        .expect("config should update fake codex dispatch");
    }

    fn write_fake_codex_success(path: &Path, message: &str) {
        #[cfg(windows)]
        let script = format!(
            "Write-Output '{{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}}'\r\nWrite-Output '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"{message}\"}}}}'\r\n"
        );
        #[cfg(not(windows))]
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}}'\nprintf '%s\\n' '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"{message}\"}}}}'\n"
        );
        fs::write(path, script).expect("fake codex should write");
        make_fake_codex_executable(path);
    }

    fn write_fake_codex_timeout(path: &Path) {
        #[cfg(windows)]
        let script = "Write-Output '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\r\nStart-Sleep -Seconds 30\r\n";
        #[cfg(not(windows))]
        let script = "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\ntrap '' TERM\nsleep 30\n";
        fs::write(path, script).expect("fake codex should write");
        make_fake_codex_executable(path);
    }

    fn write_fake_codex_delayed_success(path: &Path) {
        #[cfg(windows)]
        let script = "Write-Output '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\r\nStart-Sleep -Seconds 2\r\nWrite-Output '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\r\n";
        #[cfg(not(windows))]
        let script = "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nsleep 2\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n";
        fs::write(path, script).expect("fake codex should write");
        make_fake_codex_executable(path);
    }

    fn write_fake_codex_detached_timeout(path: &Path) {
        #[cfg(windows)]
        let script = "Write-Output '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\r\nStart-Process -WindowStyle Hidden pwsh -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30'\r\nStart-Sleep -Seconds 30\r\n";
        #[cfg(not(windows))]
        let script = "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nsetsid sh -c 'sleep 30' &\nexit 0\n";
        fs::write(path, script).expect("fake codex should write");
        make_fake_codex_executable(path);
    }

    fn write_fake_codex_env_capture(path: &Path, env_capture: &Path) {
        #[cfg(windows)]
        let script = format!(
            "Set-Content -LiteralPath '{capture}' -Value ([string]$env:HOME)\r\nAdd-Content -LiteralPath '{capture}' -Value ([string]$env:XDG_CONFIG_HOME)\r\nAdd-Content -LiteralPath '{capture}' -Value ([string]$env:XDG_DATA_HOME)\r\nAdd-Content -LiteralPath '{capture}' -Value ([string]$env:XDG_STATE_HOME)\r\nAdd-Content -LiteralPath '{capture}' -Value ([string]$env:XDG_CACHE_HOME)\r\nAdd-Content -LiteralPath '{capture}' -Value ([string]$env:TMPDIR)\r\nWrite-Output '{{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}}'\r\nWrite-Output '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}}}'\r\n",
            capture = env_capture.display()
        );
        #[cfg(not(windows))]
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$HOME\" > \"{capture}\"\nprintf '%s\\n' \"$XDG_CONFIG_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_DATA_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_STATE_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_CACHE_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$TMPDIR\" >> \"{capture}\"\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}}'\nprintf '%s\\n' '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}}}'\n",
            capture = env_capture.display()
        );
        fs::write(path, script).expect("fake codex should write");
        make_fake_codex_executable(path);
    }

    #[test]
    fn taskflow_consume_final_closure_admission_reports_admit() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: true,
            blockers: vec![],
            root_artifact_id: "root".to_string(),
            artifact_count: 4,
            boot_classification: "compatible".to_string(),
            migration_state: "ready".to_string(),
            activation_status: "ready_enough_for_normal_work".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec![
                "vida docflow check --profile active-canon".to_string(),
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };
        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "status".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "ready_for_runtime_routing"
            }),
            reason: "test".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "admit");
        assert!(admission.admitted);
        assert!(admission.blockers.is_empty());
        assert_eq!(
            admission.proof_surfaces,
            vec![
                "vida taskflow consume bundle check",
                "vida docflow check --profile active-canon",
                "vida docflow readiness-check --profile active-canon",
                "vida docflow proofcheck --profile active-canon",
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_closure_admission_reports_fail_closed_blockers() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: false,
            blockers: vec!["boot_incompatible".to_string()],
            root_artifact_id: "root".to_string(),
            artifact_count: 0,
            boot_classification: "blocking".to_string(),
            migration_state: "blocked".to_string(),
            activation_status: "pending".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "block".to_string(),
            ready: false,
            blockers: vec![
                "missing_docflow_activation".to_string(),
                "missing_readiness_verdict".to_string(),
            ],
            proof_surfaces: vec!["vida docflow check --profile active-canon".to_string()],
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "status".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "blocked".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "blocked"
            }),
            reason: "test".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "block");
        assert!(!admission.admitted);
        assert_eq!(
            admission.blockers,
            vec![
                "boot_incompatible",
                "missing_closure_proof",
                "missing_docflow_activation",
                "missing_readiness_verdict",
                "restore_reconcile_not_green",
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_closure_admission_blocks_while_design_packet_is_pending() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: true,
            blockers: vec![],
            root_artifact_id: "root".to_string(),
            artifact_count: 4,
            boot_classification: "compatible".to_string(),
            migration_state: "ready".to_string(),
            activation_status: "ready_enough_for_normal_work".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec![
                "vida docflow check --profile active-canon".to_string(),
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "create a feature with research, specification, plan, and implementation"
                .to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec![
                "research".to_string(),
                "specification".to_string(),
                "implementation".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "design_first"
            }),
            reason: "auto_feature_design_request".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "block");
        assert!(!admission.admitted);
        assert_eq!(
            admission.blockers,
            vec!["pending_design_packet", "pending_developer_handoff_packet"]
        );
    }

    #[test]
    fn runtime_host_execution_contract_reflects_external_qwen_selection() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let contract = runtime_host_execution_contract_for_root(harness.path());
        assert_eq!(contract["selected_cli_system"], "qwen");
        assert_eq!(contract["selected_cli_execution_class"], "external");
        assert_eq!(contract["runtime_template_root"], ".qwen");
        assert_eq!(contract["template_materialized"], true);
    }

    #[test]
    fn runtime_assignment_from_dispatch_alias_is_fail_closed_when_runtime_role_is_missing() {
        let compiled_bundle = serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "rate": 1,
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"]
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {
                        "junior": {
                            "effective_score": 90,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "dispatch_aliases": [
                    {
                        "role_id": "development_implementer",
                        "task_classes": ["implementation"]
                    }
                ]
            }
        });

        let assignment = build_runtime_assignment_from_dispatch_alias(
            &compiled_bundle,
            "development_implementer",
            "implementation",
        );
        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "dispatch_alias_runtime_role_missing");
    }

    fn install_external_cli_test_subagents(config_path: &Path) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config should parse as yaml");
        let root = document
            .as_mapping_mut()
            .expect("config root should be a yaml mapping");
        let agent_system_key = serde_yaml::Value::String("agent_system".to_string());
        let agent_system = root
            .entry(agent_system_key)
            .or_insert_with(|| serde_yaml::Value::Mapping(Default::default()));
        let agent_system = agent_system
            .as_mapping_mut()
            .expect("agent_system should be a yaml mapping");
        let subagents: serde_yaml::Value = serde_yaml::from_str(concat!(
            "internal_subagents:\n",
            "  enabled: true\n",
            "  subagent_backend_class: internal\n",
            "  runtime_roles: [worker, coach, verifier]\n",
            "  task_classes: [implementation, delivery_task, execution_block, coach, review, verification]\n",
            "opencode_cli:\n",
            "  enabled: true\n",
            "  subagent_backend_class: external_cli\n",
            "  runtime_roles: [worker, coach, verifier]\n",
            "  task_classes: [implementation, delivery_task, execution_block, coach, review, verification]\n",
            "  detect_command: cargo\n",
            "  dispatch:\n",
            "    command: qwen\n",
            "    static_args:\n",
            "      - -y\n",
            "      - -o\n",
            "      - text\n",
            "    model_flag: --model\n",
            "    prompt_mode: positional\n",
            "hermes_cli:\n",
            "  enabled: true\n",
            "  subagent_backend_class: external_cli\n",
            "  runtime_roles: [coach, verifier]\n",
            "  task_classes: [coach, review, verification]\n",
            "  detect_command: hermes\n",
            "  dispatch:\n",
            "    command: hermes\n",
            "    static_args:\n",
            "      - chat\n",
            "      - -Q\n",
            "      - -q\n",
            "    model_flag: --model\n",
            "    provider_flag: --provider\n",
            "    prompt_mode: positional\n",
        ))
        .expect("test subagents should parse as yaml");
        agent_system.insert(
            serde_yaml::Value::String("subagents".to_string()),
            subagents,
        );
        assert!(
            agent_system.contains_key(serde_yaml::Value::String("subagents".to_string())),
            "expected test subagents to be installed"
        );
        let updated = serde_yaml::to_string(&document).expect("config should serialize as yaml");
        fs::write(config_path, updated).expect("config should update");
    }

    fn install_external_cli_test_model_profiles(config_path: &Path) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config should parse as yaml");
        let root = document
            .as_mapping_mut()
            .expect("config root should be a yaml mapping");
        let opencode = root
            .get_mut(serde_yaml::Value::String("agent_system".to_string()))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|agent_system| {
                agent_system.get_mut(serde_yaml::Value::String("subagents".to_string()))
            })
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|subagents| {
                subagents.get_mut(serde_yaml::Value::String("opencode_cli".to_string()))
            })
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("opencode_cli test subagent should exist");
        opencode.insert(
            serde_yaml::Value::String("default_model".to_string()),
            serde_yaml::Value::String("opencode/minimax-m2.5-free".to_string()),
        );
        opencode.insert(
            serde_yaml::Value::String("default_model_profile".to_string()),
            serde_yaml::Value::String("opencode_minimax_free_review".to_string()),
        );
        opencode.insert(
            serde_yaml::Value::String("model_profiles".to_string()),
            serde_yaml::from_str(concat!(
                "opencode_minimax_free_review:\n",
                "  provider: opencode\n",
                "  model_ref: opencode/minimax-m2.5-free\n",
                "  reasoning_effort: provider_default\n",
                "  normalized_cost_units: 0\n",
                "  runtime_roles: [coach]\n",
                "  task_classes: [review]\n",
                "opencode_codex_mini_review:\n",
                "  provider: opencode\n",
                "  model_ref: opencode/gpt-5.1-codex-mini\n",
                "  reasoning_effort: low\n",
                "  normalized_cost_units: 1\n",
                "  runtime_roles: [coach]\n",
                "  task_classes: [review]\n",
            ))
            .expect("model profiles should parse"),
        );
        let updated = serde_yaml::to_string(&document).expect("config should serialize as yaml");
        fs::write(config_path, updated).expect("config should update");
    }

    fn set_test_subagent_dispatch_command(
        config_path: &Path,
        backend_id: &str,
        command: &str,
        static_args: &[&str],
    ) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config should parse as yaml");
        let agent_system_key = serde_yaml::Value::String("agent_system".to_string());
        let subagents_key = serde_yaml::Value::String("subagents".to_string());
        let backend_key = serde_yaml::Value::String(backend_id.to_string());
        let dispatch_key = serde_yaml::Value::String("dispatch".to_string());
        let dispatch = document
            .get_mut(&agent_system_key)
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|agent_system| agent_system.get_mut(&subagents_key))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|subagents| subagents.get_mut(&backend_key))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|subagent| subagent.get_mut(&dispatch_key))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("test subagent dispatch should exist");
        dispatch.insert(
            serde_yaml::Value::String("command".to_string()),
            serde_yaml::Value::String(command.to_string()),
        );
        dispatch.insert(
            serde_yaml::Value::String("static_args".to_string()),
            serde_yaml::Value::Sequence(
                static_args
                    .iter()
                    .map(|arg| serde_yaml::Value::String((*arg).to_string()))
                    .collect(),
            ),
        );
        let updated = serde_yaml::to_string(&document).expect("config should serialize as yaml");
        fs::write(config_path, updated).expect("config should update dispatch command");
    }

    fn set_test_subagent_dispatch_timeout(
        config_path: &Path,
        backend_id: &str,
        timeout_seconds: u64,
    ) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config should parse as yaml");
        let agent_system_key = serde_yaml::Value::String("agent_system".to_string());
        let subagents_key = serde_yaml::Value::String("subagents".to_string());
        let backend_key = serde_yaml::Value::String(backend_id.to_string());
        let dispatch_key = serde_yaml::Value::String("dispatch".to_string());
        let backend = document
            .get_mut(&agent_system_key)
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|agent_system| agent_system.get_mut(&subagents_key))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|subagents| subagents.get_mut(&backend_key))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("test subagent should exist");
        backend.insert(
            serde_yaml::Value::String("max_runtime_seconds".to_string()),
            serde_yaml::Value::Number(timeout_seconds.into()),
        );
        let dispatch = backend
            .get_mut(&dispatch_key)
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("test subagent dispatch should exist");
        dispatch.insert(
            serde_yaml::Value::String("no_output_timeout_seconds".to_string()),
            serde_yaml::Value::Number(timeout_seconds.into()),
        );
        let updated = serde_yaml::to_string(&document).expect("config should serialize as yaml");
        fs::write(config_path, updated).expect("config should update dispatch timeout");
    }

    fn bridge_test_role_selection(dev_task_id: &str) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": dev_task_id,
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
    }

    #[test]
    fn build_taskflow_handoff_plan_emits_canonical_execution_preparation_artifacts() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "status": "execution_ready",
                "pre_execution_design_gate": {
                    "developer_handoff_packet_status": "blocked_pending_developer_handoff_packet"
                },
                "development_flow": {
                    "lane_sequence": ["execution_preparation", "implementer"],
                    "dispatch_contract": {
                        "lane_catalog": {
                            "execution_preparation": {
                                "completion_blocker": "pending_execution_preparation_evidence"
                            }
                        }
                    }
                },
                "orchestration_contract": {},
                "runtime_assignment": {
                    "selected_tier": "junior"
                }
            }),
            reason: "test".to_string(),
        };

        let plan = build_taskflow_handoff_plan(&role_selection);

        assert_eq!(plan["status"], "execution_handoff_ready");
        assert_eq!(plan["handoff_ready"], true);
        assert_eq!(
            plan["execution_preparation_artifacts"]["handoff_ready"],
            true
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["developer_handoff_packet"]["ready"],
            false
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["developer_handoff_packet"]["status"],
            "blocked_pending_developer_handoff_packet"
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["required_artifacts"],
            serde_json::json!([
                "architecture_preparation_report",
                "developer_handoff_packet",
                "change_boundary",
                "dependency_impact_summary",
                "spec_alignment_summary",
            ])
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["architecture_preparation_report"]["status"],
            "pending_architecture_preparation_report"
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["change_boundary"]["status"],
            "pending_change_boundary"
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["dependency_impact_summary"]["status"],
            "pending_dependency_impact_summary"
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["spec_alignment_summary"]["status"],
            "pending_spec_alignment_summary"
        );
        assert_eq!(
            plan["execution_preparation_artifacts"]["execution_preparation_evidence"]["status"],
            "pending_execution_preparation_evidence"
        );
    }

    fn agent_lane_test_execution_plan(executor_backend: &str) -> serde_json::Value {
        let (model_profile_id, model_ref, reasoning_effort) = match executor_backend {
            "opencode_cli" => (
                "opencode_codex_mini_review",
                "opencode/gpt-5.1-codex-mini",
                "low",
            ),
            "internal_subagents" => ("internal_fast", "internal_fast", "low"),
            "middle" => ("codex_gpt54_medium_write", "gpt-5.5", "medium"),
            _ => ("codex_gpt54_low_write", "gpt-5.5", "low"),
        };
        json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "junior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "middle",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "opencode_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "implementer": {
                    "executor_backend": executor_backend
                }
            },
            "runtime_assignment": {
                "selected_carrier_id": executor_backend,
                "selected_backend_id": executor_backend,
                "selected_model_profile_id": model_profile_id,
                "selected_model_ref": model_ref,
                "selected_reasoning_effort": reasoning_effort,
                "selected_runtime_role": "worker",
                "task_class": "implementation"
            }
        })
    }

    fn mixed_backend_execution_plan() -> serde_json::Value {
        json!({
            "backend_admissibility_matrix": [
            {
                "backend_id": "opencode_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "implementation": false,
                    "coach": false,
                    "verification": true
                }
            },
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "implementation": false,
                        "coach": true,
                        "verification": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true,
                        "coach": true,
                        "verification": true
                    }
                }
            ],
            "development_flow": {
                "implementer": {
                    "executor_backend": "opencode_cli",
                    "fallback_executor_backend": "internal_subagents"
                },
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                },
                "verification": {
                    "executor_backend": "opencode_cli",
                    "fallback_executor_backend": "internal_subagents"
                },
                "review_ensemble": {
                    "executor_backend": "opencode_cli",
                    "fallback_executor_backend": "internal_subagents",
                    "fanout_executor_backends": ["opencode_cli", "hermes_cli", "kilo_cli"]
                },
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementer", "coach", "verification"],
                    "implementer_activation": {
                        "activation_agent_type": "junior",
                        "activation_runtime_role": "worker"
                    },
                    "coach_activation": {
                        "activation_agent_type": "middle",
                        "activation_runtime_role": "coach"
                    },
                    "verifier_activation": {
                        "activation_agent_type": "senior",
                        "activation_runtime_role": "verifier"
                    }
                }
            }
        })
    }

    fn mixed_backend_role_selection() -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: mixed_backend_execution_plan(),
            reason: "test".to_string(),
        }
    }

    fn pi_cli_analysis_role_selection() -> RuntimeConsumptionLaneSelection {
        let mut role_selection = bridge_test_role_selection("feature-x-dev");
        role_selection.execution_plan["backend_admissibility_matrix"] = json!([
            {
                "backend_id": "internal_subagents",
                "backend_class": "internal",
                "lane_admissibility": {
                    "analysis": true,
                    "verification": true
                }
            },
            {
                "backend_id": "pi_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "analysis": true,
                    "verification": true
                }
            }
        ]);
        role_selection.execution_plan["runtime_assignment"] = json!({
            "selected_backend_id": "pi_cli",
            "selected_carrier_id": "pi_cli",
            "selected_model_profile_id": "pi_gpt55_medium_guarded",
            "activation_agent_type": "pi_cli",
            "activation_runtime_role": "verifier",
            "task_class": "analysis"
        });
        role_selection
    }

    fn blocked_analysis_receipt(packet_path: Option<String>) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: "run-pi-cli-analysis".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --execute-dispatch --json".to_string()),
            dispatch_packet_path: packet_path,
            dispatch_result_path: None,
            blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
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
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-20T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn preferred_selected_backend_ignores_stale_internal_override_for_runtime_assignment() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let packet_path = harness.path().join("analysis-packet.json");
        fs::write(
            &packet_path,
            json!({
                "selected_backend": "internal_subagents",
                "selected_backend_override": "internal_subagents",
                "runtime_assignment": {
                    "selected_backend_id": "pi_cli",
                    "selected_carrier_id": "pi_cli",
                    "selected_model_profile_id": "pi_gpt55_medium_guarded"
                }
            })
            .to_string(),
        )
        .expect("packet should write");
        let role_selection = pi_cli_analysis_role_selection();
        let receipt = blocked_analysis_receipt(Some(packet_path.display().to_string()));

        assert_eq!(
            preferred_selected_backend_for_receipt(&role_selection, &receipt).as_deref(),
            Some("pi_cli")
        );
    }

    #[test]
    fn runtime_dispatch_packet_drops_stale_internal_override_for_pi_cli_assignment() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let role_selection = pi_cli_analysis_role_selection();
        let receipt = blocked_analysis_receipt(None);
        let taskflow_handoff_plan = build_taskflow_handoff_plan(&role_selection);
        let run_graph_bootstrap = json!({ "run_id": "run-pi-cli-analysis" });
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &taskflow_handoff_plan,
            &run_graph_bootstrap,
        )
        .with_selected_backend_override(Some("internal_subagents".to_string()));

        let packet = runtime_dispatch_packet_preview(&ctx)
            .expect("packet preview should render")
            .get("packet")
            .cloned()
            .expect("packet should exist");

        assert_eq!(packet["selected_backend"], "pi_cli");
        assert!(packet["selected_backend_override"].is_null());
        assert_eq!(
            packet["effective_execution_posture"]["selected_backend"],
            "pi_cli"
        );
        assert_ne!(
            packet["effective_execution_posture"]["selected_backend_source"],
            "explicit_retry_override"
        );
    }

    #[test]
    fn preferred_selected_backend_prefers_route_backend_over_carrier_assignment_and_stale_receipt()
    {
        let mut role_selection = pi_cli_analysis_role_selection();
        role_selection.execution_plan["development_flow"]["analysis"] = json!({
            "executor_backend": "pi_cli",
            "carrier_runtime_assignment": {
                "selected_backend_id": "internal_subagents",
                "selected_carrier_id": "internal_subagents",
                "activation_agent_type": "internal_subagents"
            }
        });
        let receipt = blocked_analysis_receipt(None);

        assert_eq!(
            canonical_selected_backend_for_receipt(&role_selection, &receipt).as_deref(),
            Some("pi_cli")
        );
        assert_eq!(
            preferred_selected_backend_for_receipt(&role_selection, &receipt).as_deref(),
            Some("pi_cli")
        );
    }

    #[test]
    fn preferred_selected_backend_uses_receipt_backend_only_as_terminal_fallback() {
        let mut role_selection = pi_cli_analysis_role_selection();
        role_selection.execution_plan = json!({});
        let mut receipt = blocked_analysis_receipt(None);
        receipt.dispatch_target = "unmapped-target".to_string();
        receipt.activation_agent_type = None;
        receipt.selected_backend = Some("internal_subagents".to_string());

        assert!(canonical_selected_backend_for_receipt(&role_selection, &receipt).is_none());
        assert_eq!(
            preferred_selected_backend_for_receipt(&role_selection, &receipt).as_deref(),
            Some("internal_subagents")
        );
    }

    fn executed_agent_lane_receipt(
        dispatch_target: &str,
        selected_backend: &str,
        activation_agent_type: &str,
        activation_runtime_role: &str,
        downstream_dispatch_target: Option<&str>,
    ) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: "run-mixed-backend-matrix".to_string(),
            dispatch_target: dispatch_target.to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_complete".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(format!("/tmp/{dispatch_target}-packet.json")),
            dispatch_result_path: Some(format!("/tmp/{dispatch_target}-result.json")),
            blocker_code: None,
            downstream_dispatch_target: downstream_dispatch_target.map(str::to_string),
            downstream_dispatch_command: downstream_dispatch_target
                .map(|_| "vida agent-init".to_string()),
            downstream_dispatch_note: downstream_dispatch_target.map(|target| {
                format!(
                    "after `{dispatch_target}` evidence is recorded, activate `{target}` for the next bounded lane"
                )
            }),
            downstream_dispatch_ready: downstream_dispatch_target.is_some(),
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: downstream_dispatch_target
                .map(|target| format!("/tmp/{target}-packet.json")),
            downstream_dispatch_status: downstream_dispatch_target
                .map(|_| "packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: downstream_dispatch_target.map(str::to_string),
            downstream_dispatch_last_target: downstream_dispatch_target.map(str::to_string),
            activation_agent_type: Some(activation_agent_type.to_string()),
            activation_runtime_role: Some(activation_runtime_role.to_string()),
            selected_backend: Some(selected_backend.to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn dispatch_state_reopen_failure_names_run_and_target() {
        let receipt = executed_agent_lane_receipt(
            "closure",
            "internal_subagents",
            "middle",
            "verifier",
            None,
        );
        let message = dispatch_state_reopen_failure_message(
            &receipt,
            "before dispatch execution",
            "os error 33",
        );

        assert!(message.contains("run `run-mixed-backend-matrix`"));
        assert!(message.contains("target `closure`"));
        assert!(message.contains("before dispatch execution"));
        assert!(message.contains("os error 33"));
    }

    fn agent_lane_test_request() -> &'static str {
        "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
    }

    #[test]
    fn route_profile_override_prefers_internal_review_over_runtime_assignment_default() {
        let mut execution_plan = agent_lane_test_execution_plan("internal_subagents");
        execution_plan["development_flow"]["coach"] = json!({
            "executor_backend": "internal_subagents",
            "profiles": {
                "internal_subagents": "internal_review"
            }
        });
        execution_plan["runtime_assignment"] = json!({
            "selected_model_profile_id": "codex_gpt54_low_write"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: format!(
                "{} in crates/vida/src/runtime_dispatch_state.rs",
                agent_lane_test_request()
            ),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        };

        let selected_profile = preferred_selected_model_profile_for_dispatch_target(
            &role_selection,
            "coach",
            Some("internal_subagents"),
        );

        assert_eq!(selected_profile.as_deref(), Some("internal_review"));
    }

    #[test]
    fn route_profile_override_for_analysis_uses_analysis_route_when_present() {
        let mut execution_plan = agent_lane_test_execution_plan("internal_subagents");
        execution_plan["development_flow"]["analysis"] = json!({
            "executor_backend": "analysis_cli",
            "profiles": {
                "internal_subagents": "analysis_profile"
            }
        });
        execution_plan["development_flow"]["implementation"] = json!({
            "executor_backend": "internal_subagents",
            "profiles": {
                "internal_subagents": "implementation_profile"
            }
        });
        execution_plan["runtime_assignment"] = json!({
            "selected_model_profile_id": "codex_gpt54_low_write"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        };

        let selected_profile = preferred_selected_model_profile_for_dispatch_target(
            &role_selection,
            "analysis",
            Some("internal_subagents"),
        );

        assert_eq!(selected_profile.as_deref(), Some("analysis_profile"));
    }

    fn specification_test_role_selection(
        spec_task_id: &str,
        design_doc_path: &str,
    ) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue specification".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "spec_task": {
                        "task_id": spec_task_id
                    },
                    "design_doc_path": design_doc_path,
                    "work_pool_task": {
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "completion_blocker": "pending_specification_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
    }

    fn bridge_waiting_root_receipt(run_id: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: Some("vida task ensure".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        }
    }

    async fn create_and_close_task(store: &crate::StateStore, task_id: &str) {
        let labels = vec!["dev-pack".to_string()];
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id,
                title: "Dev pack",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");
        store
            .close_task(task_id, "implemented and proven")
            .await
            .expect("task should close");
    }

    fn write_approved_design_doc(path: &Path) {
        fs::write(path, "# Test Design\n\nStatus: `approved`\n").expect("design doc should write");
    }

    fn read_json(project_root: &Path, path: &str) -> serde_json::Value {
        let resolved = if Path::new(path).is_absolute() {
            Path::new(path).to_path_buf()
        } else {
            project_root.join(path)
        };
        serde_json::from_str(
            &fs::read_to_string(&resolved).expect("json artifact should be readable"),
        )
        .expect("json artifact should decode")
    }

    #[test]
    fn runtime_dispatch_packet_contract_accepts_template_specific_minimums() {
        let delivery = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": runtime_delivery_task_packet(
                "run-1",
                "implementer",
                "worker",
                "implementation",
                "implementation",
                "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&delivery, "test packet").is_ok());

        let coach = serde_json::json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": runtime_coach_review_packet(
                "run-1",
                "coach",
                Some("implementer"),
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&coach, "test packet").is_ok());

        let verifier = serde_json::json!({
            "packet_template_kind": "verifier_proof_packet",
            "verifier_proof_packet": runtime_verifier_proof_packet(
                "run-1",
                "verification",
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&verifier, "test packet").is_ok());
    }

    #[test]
    fn runtime_delivery_task_packet_collects_explicit_owned_paths_from_request_text() {
        let packet = runtime_delivery_task_packet(
            "run-1",
            "implementer",
            "worker",
            "implementation",
            "implementation",
            "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
        );

        assert_eq!(
            packet["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
    }

    #[test]
    fn runtime_delivery_task_packet_uses_tracked_design_doc_scope_for_specification() {
        let packet = runtime_delivery_task_packet_with_scope_context(
            "run-1",
            "specification",
            "business_analyst",
            "specification",
            "specification",
            "Investigate scope and do not edit crates/vida/src/runtime_dispatch_state.rs directly.",
            Some("docs/product/spec/feature-x-design.md"),
        );

        assert_eq!(
            packet["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
        );
    }

    #[test]
    fn runtime_delivery_task_packet_uses_bounded_file_set_from_tracked_design_doc_for_implementation(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let design_doc_path = std::env::temp_dir().join(format!(
            "vida-implementation-bounded-file-set-{}-{}.md",
            std::process::id(),
            nanos
        ));
        std::fs::write(
            &design_doc_path,
            "### Bounded File Set\n- `crates/vida/src/runtime_dispatch_packets.rs`\n- `crates/vida/src/runtime_dispatch_state.rs`\n",
        )
        .expect("write tracked design doc");

        let packet = runtime_delivery_task_packet_with_scope_context(
            "run-1",
            "implementer",
            "worker",
            "implementation",
            "implementation",
            "Continue the bounded implementation packet and keep scope from the approved design.",
            Some(
                design_doc_path
                    .to_str()
                    .expect("design doc path should be utf-8"),
            ),
        );

        assert_eq!(
            packet["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let _ = std::fs::remove_file(design_doc_path);
    }

    #[test]
    fn explicit_request_scope_paths_stay_empty_without_file_scope_in_request_text() {
        assert!(explicit_request_scope_paths("continue development").is_empty());
    }

    #[test]
    fn runtime_dispatch_packet_contract_declares_and_enforces_single_task_move_scope() {
        let request_text = "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation.";
        let delivery_packet = runtime_delivery_task_packet(
            "run-1",
            "implementer",
            "worker",
            "implementation",
            "implementation",
            request_text,
        );
        assert_eq!(
            delivery_packet["owned_paths"],
            serde_json::json!([
                "crates/vida/src/main.rs",
                "crates/vida/src/project_activator_surface.rs"
            ])
        );

        let packet = serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": delivery_packet.clone(),
            "request_text": request_text,
            "role_selection_full": {
                "single_task_only": true
            }
        });
        assert!(validate_runtime_dispatch_packet_contract(&packet, "test packet").is_ok());

        let mut widened_packet = packet.clone();
        widened_packet["delivery_task_packet"]["owned_paths"] = serde_json::json!([
            "crates/vida/src/main.rs",
            "crates/vida/src/project_activator_surface.rs",
            "crates/vida/src/runtime_dispatch_state.rs"
        ]);
        let error = validate_runtime_dispatch_packet_contract(&widened_packet, "test packet")
            .expect_err("widened single-task move packet should fail closed");
        assert!(error.contains("single-task move packet owned_paths"));
        assert!(error.contains("expected"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_rejects_implementation_delivery_without_owned_paths() {
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer"],
                "owned_paths": [],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?",
                "handoff_task_class": "implementation"
            }
        });

        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("implementation delivery packet without owned scope should fail closed");
        assert!(error.contains("owned_paths"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_allows_analysis_delivery_without_owned_paths() {
        let packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::analysis::delivery",
                "goal": "Execute bounded analysis handoff",
                "scope_in": ["dispatch_target:analysis"],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?",
                "handoff_task_class": "analysis"
            }
        });

        validate_runtime_dispatch_packet_contract(&packet, "test packet")
            .expect("analysis delivery packet should remain read-only capable");
    }

    #[test]
    fn runtime_dispatch_packet_contract_rejects_top_level_scope_drift() {
        let packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["crates/vida/src/taskflow_run_graph.rs"],
            "read_only_paths": ["docs/process"],
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer"],
                "owned_paths": ["crates/vida/src/runtime_dispatch_state.rs"],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?",
                "handoff_task_class": "implementation"
            }
        });

        let error = validate_runtime_dispatch_packet_contract(&packet, "test packet")
            .expect_err("top-level owned_paths drift should fail closed");
        assert!(error.contains("top-level owned_paths must mirror"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_rejects_specification_delivery_with_code_owned_paths() {
        let role_selection = specification_test_role_selection(
            "feature-x-spec-task",
            "docs/product/spec/feature-x-design.md",
        );
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "request_text": "Investigate scope around crates/vida/src/runtime_dispatch_state.rs and capture the design update.",
            "role_selection_full": role_selection,
            "delivery_task_packet": {
                "packet_id": "run-1::specification::delivery",
                "goal": "Execute bounded specification handoff",
                "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                "owned_paths": ["crates/vida/src/runtime_dispatch_state.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                "definition_of_done": ["bounded specification result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for specification?",
                "handoff_task_class": "specification"
            }
        });

        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("specification delivery packet with code-owned scope should fail closed");
        assert!(error.contains("tracked design-doc scope"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_fails_closed_for_missing_required_fields() {
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "scope_in": ["dispatch_target:implementer"],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?"
            }
        });
        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("packet without goal should fail closed");
        assert!(error.contains("missing required packet fields"));
        assert!(error.contains("goal"));
    }

    #[test]
    fn execute_runtime_dispatch_handoff_executes_internal_codex_carrier() {
        run_on_large_test_stack(
            "execute_runtime_dispatch_handoff_executes_internal_codex_carrier",
            || {
                let runtime =
                    tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
                let _cwd = guard_current_dir(harness.path());
                let _vida_root_guard =
                    EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
                let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

                assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "project-activator",
                        "--project-id",
                        "test-project",
                        "--language",
                        "english",
                        "--host-cli-system",
                        "codex",
                        "--json"
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());

                let fake_bin = harness.path().join("fake-bin");
                fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
                let fake_codex = fake_codex_path(&fake_bin);
                write_fake_codex_success(&fake_codex, "internal-dispatch-ok");
                configure_fake_codex_dispatch(harness.path(), &fake_codex);
                let patched_path = prepend_to_path(&fake_bin);
                let _path_guard = EnvVarGuard::set("PATH", &patched_path);

                let state_root = harness_state_root(&harness);
                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should open");
                let labels: Vec<String> = Vec::new();
                runtime
                    .block_on(store.create_task_with_fixture_parent(
                        crate::state_store::CreateTaskRequest {
                            task_id: "run-agent-dispatch",
                            title: "Run agent dispatch",
                            display_id: None,
                            description: "test task backing the execute-dispatch run graph",
                            issue_type: "task",
                            status: "in_progress",
                            priority: 1,
                            parent_id: None,
                            labels: &labels,
                            execution_semantics:
                                crate::state_store::TaskExecutionSemantics::default(),
                            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                            created_by: "tester",
                            source_repo: ".",
                        },
                    ))
                    .expect("run graph task should exist");
                let dispatch_packet_path = harness.path().join("agent-dispatch.json");
                fs::write(
                    &dispatch_packet_path,
                    serde_json::to_string_pretty(&serde_json::json!({
                        "packet_kind": "runtime_dispatch_packet",
                        "packet_template_kind": "delivery_task_packet",
                        "delivery_task_packet": runtime_delivery_task_packet(
                            "run-agent-dispatch",
                            "implementer",
                            "worker",
                            "implementation",
                            "implementation",
                            "continue development"
                        ),
                        "dispatch_target": "implementer",
                        "request_text": "continue development",
                        "activation_runtime_role": "worker",
                        "role_selection": {
                            "selected_role": "worker"
                        }
                    }))
                    .expect("dispatch packet json should encode"),
                )
                .expect("dispatch packet should write");

                let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
                execution_plan["runtime_assignment"] = serde_json::json!({
                    "selected_model_profile_id": "opencode_codex_mini_review",
                    "selected_model_ref": "opencode/gpt-5.1-codex-mini",
                    "selected_reasoning_effort": "low"
                });
                let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
                let run_graph_bootstrap = serde_json::json!({
                    "run_id": "run-agent-dispatch"
                });
                let status = crate::state_store::RunGraphStatus {
                    run_id: "run-agent-dispatch".to_string(),
                    task_id: "run-agent-dispatch".to_string(),
                    task_class: "implementation".to_string(),
                    active_node: "planning".to_string(),
                    next_node: Some("worker".to_string()),
                    status: "ready".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "junior".to_string(),
                    lane_id: "worker_lane".to_string(),
                    lifecycle_stage: "dispatch_ready".to_string(),
                    policy_gate: "single_task_scope_required".to_string(),
                    handoff_state: "awaiting_worker".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "conversation_cursor".to_string(),
                    resume_target: "dispatch.worker_lane".to_string(),
                    recovery_ready: true,
                };
                runtime
                    .block_on(store.record_run_graph_status(&status))
                    .expect("run graph status should record");

                let receipt = crate::state_store::RunGraphDispatchReceipt {
                    run_id: "run-agent-dispatch".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "routed".to_string(),
                    lane_status: "lane_running".to_string(),
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
                    recorded_at: "2026-03-17T00:00:00Z".to_string(),
                };
                let handoff_plan = serde_json::json!({});
                let ctx = RuntimeDispatchPacketContext::new(
                    &state_root,
                    &role_selection,
                    &receipt,
                    &handoff_plan,
                    &run_graph_bootstrap,
                );
                let dispatch_packet_path =
                    write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
                let mut persisted_receipt = receipt.clone();
                persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
                runtime
                    .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
                    .expect("dispatch receipt should record");
                drop(store);
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "agent-init",
                        "--dispatch-packet",
                        dispatch_packet_path.as_str(),
                        "--execute-dispatch",
                        "--json",
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());

                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should reopen");
                let recorded_receipt = runtime
                    .block_on(store.latest_run_graph_dispatch_receipt())
                    .expect("latest dispatch receipt should load")
                    .expect("latest dispatch receipt should exist");
                let dispatch_result_path = recorded_receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should record");
                let rendered = fs::read_to_string(dispatch_result_path)
                    .expect("dispatch result artifact should load");
                let parsed: serde_json::Value =
                    serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
                assert_eq!(parsed["execution_state"], "executed");
                assert_eq!(parsed["status"], "pass");
                assert_eq!(
                    parsed["activation_semantics"]["activation_kind"],
                    "execution_evidence"
                );
                assert_eq!(parsed["activation_semantics"]["view_only"], false);
                assert_eq!(parsed["activation_semantics"]["executes_packet"], true);
                assert_eq!(parsed["execution_evidence"]["status"], "recorded");
                assert_eq!(
                    parsed["execution_evidence"]["evidence_kind"],
                    "internal_carrier_completion"
                );
                assert_eq!(parsed["provider_result"], "internal-dispatch-ok");
                assert_eq!(parsed["backend_dispatch"]["backend_id"], "junior");
            },
        );
    }

    #[test]
    fn execute_runtime_dispatch_handoff_sets_writable_runtime_env_for_internal_codex_carrier() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let original_home = env::var("HOME").unwrap_or_default();
        let original_xdg_data_home = env::var("XDG_DATA_HOME").unwrap_or_default();
        let original_xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or_default();

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let env_capture = harness.path().join("internal-host-env.txt");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_env_capture(&fake_codex, &env_capture);
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch-env.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch-env",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    "continue development"
                ),
                "dispatch_target": "implementer",
                "request_text": "continue development",
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch-env".to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("internal host should execute with writable runtime env");

        assert!(
            result["surface"]
                .as_str()
                .is_some_and(|value| value.starts_with("internal_cli:")),
            "expected internal host execution result, got {result}"
        );
        assert_eq!(result["execution_state"], "executed");
        let captured = fs::read_to_string(&env_capture).expect("env capture should exist");
        let rows: Vec<_> = captured.lines().collect();
        assert_eq!(
            rows.len(),
            6,
            "expected HOME, XDG config/data, state/cache, and TMPDIR"
        );
        assert_eq!(
            rows[0], original_home,
            "HOME should stay intact for auth/config discovery"
        );
        assert_ne!(
            rows[1], original_xdg_config_home,
            "XDG_CONFIG_HOME should move into the writable project runtime root"
        );
        assert_ne!(
            rows[2], original_xdg_data_home,
            "XDG_DATA_HOME should move into the writable project runtime root"
        );
        for row in &rows[1..] {
            let path = Path::new(row);
            assert!(
                path.starts_with(harness.path().join(".vida/data/internal-host/codex/junior")),
                "runtime env path should stay inside writable project runtime root: {}",
                row
            );
            assert!(path.is_dir(), "runtime env dir should exist: {}", row);
        }
    }

    #[test]
    fn agent_init_execute_dispatch_executes_internal_codex_carrier() {
        run_on_large_test_stack(
            "agent_init_execute_dispatch_executes_internal_codex_carrier",
            || {
                let runtime =
                    tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
                let _cwd = guard_current_dir(harness.path());
                let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

                assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "project-activator",
                        "--project-id",
                        "test-project",
                        "--language",
                        "english",
                        "--host-cli-system",
                        "codex",
                        "--json"
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());
                assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());

                let fake_bin = harness.path().join("fake-bin");
                fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
                let fake_codex = fake_codex_path(&fake_bin);
                write_fake_codex_success(&fake_codex, "internal-dispatch-ok");
                configure_fake_codex_dispatch(harness.path(), &fake_codex);
                let patched_path = prepend_to_path(&fake_bin);
                let _path_guard = EnvVarGuard::set("PATH", &patched_path);

                let state_root = harness_state_root(&harness);
                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should open");
                let labels: Vec<String> = Vec::new();
                runtime
                    .block_on(store.create_task_with_fixture_parent(
                        crate::state_store::CreateTaskRequest {
                            task_id: "run-agent-init-execute-dispatch",
                            title: "Run agent init execute-dispatch",
                            display_id: None,
                            description: "test task backing the execute-dispatch run graph",
                            issue_type: "task",
                            status: "in_progress",
                            priority: 1,
                            parent_id: None,
                            labels: &labels,
                            execution_semantics:
                                crate::state_store::TaskExecutionSemantics::default(),
                            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                            created_by: "tester",
                            source_repo: ".",
                        },
                    ))
                    .expect("run graph task should exist");
                let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
                let run_graph_bootstrap = serde_json::json!({
                    "run_id": "run-agent-init-execute-dispatch"
                });
                let status = crate::state_store::RunGraphStatus {
                    run_id: "run-agent-init-execute-dispatch".to_string(),
                    task_id: "run-agent-init-execute-dispatch".to_string(),
                    task_class: "implementation".to_string(),
                    active_node: "planning".to_string(),
                    next_node: Some("worker".to_string()),
                    status: "ready".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "junior".to_string(),
                    lane_id: "worker_lane".to_string(),
                    lifecycle_stage: "dispatch_ready".to_string(),
                    policy_gate: "single_task_scope_required".to_string(),
                    handoff_state: "awaiting_worker".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "conversation_cursor".to_string(),
                    resume_target: "dispatch.worker_lane".to_string(),
                    recovery_ready: true,
                };
                runtime
                    .block_on(store.record_run_graph_status(&status))
                    .expect("run graph status should record");

                let receipt = crate::state_store::RunGraphDispatchReceipt {
                    run_id: "run-agent-init-execute-dispatch".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "routed".to_string(),
                    lane_status: "lane_running".to_string(),
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
                    recorded_at: "2026-03-17T00:00:00Z".to_string(),
                };
                let handoff_plan = serde_json::json!({});
                let ctx = RuntimeDispatchPacketContext::new(
                    &state_root,
                    &role_selection,
                    &receipt,
                    &handoff_plan,
                    &run_graph_bootstrap,
                );
                let dispatch_packet_path =
                    write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
                let mut persisted_receipt = receipt.clone();
                persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
                runtime
                    .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
                    .expect("dispatch receipt should record");
                drop(store);
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "agent-init",
                        "--dispatch-packet",
                        dispatch_packet_path.as_str(),
                        "--execute-dispatch",
                        "--json",
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());

                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should reopen");
                let recorded_receipt = runtime
                    .block_on(store.latest_run_graph_dispatch_receipt())
                    .expect("latest dispatch receipt should load")
                    .expect("latest dispatch receipt should exist");
                let dispatch_result_path = recorded_receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should record");
                let rendered = fs::read_to_string(dispatch_result_path)
                    .expect("dispatch result artifact should load");
                let parsed: serde_json::Value =
                    serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
                assert_eq!(parsed["execution_state"], "executed");
                assert_eq!(parsed["status"], "pass");
                assert_eq!(
                    parsed["activation_semantics"]["activation_kind"],
                    "execution_evidence"
                );
                assert_eq!(parsed["activation_semantics"]["view_only"], false);
                assert_eq!(parsed["activation_semantics"]["executes_packet"], true);
                assert_eq!(parsed["execution_evidence"]["status"], "recorded");
                assert_eq!(
                    parsed["execution_evidence"]["evidence_kind"],
                    "internal_carrier_completion"
                );
                assert_eq!(parsed["provider_result"], "internal-dispatch-ok");
                assert_eq!(parsed["backend_dispatch"]["backend_id"], "junior");
            },
        );
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_internal_host_on_codex_when_receipt_backend_is_external(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_success(&fake_codex, "internal-dispatch-ok");
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-internal-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-internal-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-internal-dispatch".to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            preferred_selected_backend_for_receipt(&role_selection, &receipt).as_deref(),
            Some("junior"),
            "route/runtime assignment should win over a stale external receipt backend"
        );

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("internal host should ignore external receipt backend and execute on codex");

        assert!(
            result["surface"]
                .as_str()
                .is_some_and(|value| value.starts_with("internal_cli:")),
            "expected internal host execution result, got {result}"
        );
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_evidence"]["backend_id"], "junior");
        assert_eq!(result["backend_dispatch"]["backend_class"], "internal");
        assert_eq!(result["backend_dispatch"]["backend_id"], "junior");
        assert_eq!(result["provider_result"], "internal-dispatch-ok");
    }

    #[test]
    fn execute_and_record_dispatch_receipt_updates_surface_from_internal_execution_result() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_success(&fake_codex, "internal-dispatch-ok");
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch-record.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch-record",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-agent-dispatch-record"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch-record".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("dispatch receipt should record execution evidence");

        assert_eq!(receipt.dispatch_status, "executed");
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert!(receipt
            .dispatch_command
            .as_deref()
            .is_some_and(|value| value.contains("exec")));
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        let store = runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store should reopen");
        let persisted_receipt = runtime
            .block_on(store.run_graph_dispatch_receipt("run-agent-dispatch-record"))
            .expect("persisted dispatch receipt should load")
            .expect("persisted dispatch receipt should exist");
        assert_eq!(persisted_receipt.dispatch_status, "executed");
        assert_eq!(persisted_receipt.dispatch_target, "implementer");
        assert!(persisted_receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
    }

    #[test]
    fn execute_and_record_dispatch_receipt_closes_from_admitted_execution_packet() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-bundle-blocked",
            "closure",
            "delivery",
        );
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "closure_active".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should record");
        drop(store);
        wait_for_state_unlock(harness.path());

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "close the current runtime packet".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("closure".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["closure".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "ready_for_runtime_routing"
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-closure-bundle-blocked"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-closure-bundle-blocked".to_string(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "closure".to_string(),
            dispatch_surface: Some("vida taskflow closure-preview".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/closure-dispatch.json".to_string()),
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
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("closure dispatch should execute");

        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt.blocker_code.is_none());
        let result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("closure preview result should persist");
        let result = read_json(harness.path(), result_path);
        assert_eq!(result["surface"], "vida taskflow closure-preview");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["closure_ready"], true);
        assert!(result["blockers"]
            .as_array()
            .is_some_and(|blockers| blockers.is_empty()));

        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should reopen");
        let persisted_status = runtime
            .block_on(store.run_graph_status("run-closure-bundle-blocked"))
            .expect("run graph status should load");
        assert_eq!(persisted_status.status, "completed");
        assert_eq!(persisted_status.lifecycle_stage, "closure_complete");
    }

    #[test]
    fn execute_and_record_dispatch_receipt_advances_seeded_analysis_gate_after_execution_evidence()
    {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_success(&fake_codex, "analysis-validation-ok");
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-analysis-validation-bridge",
            "implementation",
            "implementation",
        );
        status.task_id = "run-analysis-validation-bridge".to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("analysis".to_string());
        status.status = "ready".to_string();
        status.route_task_class = "implementation".to_string();
        status.selected_backend = "internal_subagents".to_string();
        status.lane_id = "analysis_lane".to_string();
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "awaiting_analysis".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.analysis_lane".to_string();
        status.recovery_ready = true;
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run-graph status should record");
        drop(store);

        let dispatch_packet_path = harness
            .path()
            .join("analysis-validation-bridge-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-analysis-validation-bridge",
                    "analysis",
                    "coach",
                    "implementation",
                    "implementation",
                    "Analyze the bounded packet for crates/vida/src/runtime_dispatch_state.rs and return validation evidence."
                ),
                "dispatch_target": "analysis",
                "request_text": "Analyze the bounded packet for crates/vida/src/runtime_dispatch_state.rs and return validation evidence.",
                "activation_runtime_role": "coach",
                "role_selection": {
                    "selected_role": "coach"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Analyze the bounded packet for crates/vida/src/runtime_dispatch_state.rs and return validation evidence."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["analysis".to_string(), "validation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-analysis-validation-bridge"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-analysis-validation-bridge".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-04-23T00:00:00Z".to_string(),
        };

        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("analysis dispatch receipt should record execution evidence");
        let result_debug = receipt
            .dispatch_result_path
            .as_deref()
            .map(|path| read_json(harness.path(), path).to_string())
            .unwrap_or_else(|| "<missing dispatch result path>".to_string());
        assert_eq!(receipt.dispatch_status, "executed", "{result_debug}");
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("writer")
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());

        let store = runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store should reopen");
        let persisted_status = runtime
            .block_on(store.run_graph_status("run-analysis-validation-bridge"))
            .expect("run graph status should load");
        assert_eq!(persisted_status.active_node, "analysis");
        assert_eq!(persisted_status.next_node.as_deref(), Some("writer"));
        assert_eq!(persisted_status.lifecycle_stage, "analysis_active");
        assert_eq!(persisted_status.policy_gate, "targeted_verification");
        assert_eq!(persisted_status.handoff_state, "awaiting_writer");
        assert_eq!(persisted_status.resume_target, "dispatch.writer_lane");
    }

    #[test]
    fn execute_and_record_dispatch_receipt_persists_blocked_result_for_internal_codex_timeout() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 5\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_timeout(&fake_codex);
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("internal-host-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-internal-host-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::json!({
                "agent_system": {
                    "routing": {
                        "implementation": {
                            "max_runtime_seconds": 1
                        }
                    }
                }
            }),
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-internal-host-timeout"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("dispatch receipt should persist blocked timeout result");
        let elapsed = started.elapsed();

        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("dispatch result json should parse");
        assert!(
            elapsed < Duration::from_secs(15),
            "expected timeout wrapper to return within a bounded window, got {:?}",
            elapsed
        );
        assert_eq!(parsed["status"], "blocked");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(
            parsed["blocker_code"],
            INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert!(
            parsed["exit_code"].is_null() || parsed["exit_code"].as_i64().is_some(),
            "expected timeout path to record an exit code value or null signal exit, got {:?}",
            parsed["exit_code"]
        );
        assert!(parsed["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));
        assert_eq!(parsed["timeout_wrapper"]["timeout_seconds"], 1);
        assert_eq!(parsed["timeout_wrapper"]["kill_after_grace_seconds"], 1);
        assert_eq!(parsed["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn taskflow_consume_continue_returns_routed_receipt_for_internal_coach_handoff() {
        run_on_large_test_stack(
            "taskflow_consume_continue_returns_timeout_receipt_for_internal_coach_timeout",
            || {
                let runtime =
                    tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
                let _cwd = guard_current_dir(harness.path());
                let _vida_root_guard =
                    EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
                let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

                assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "project-activator",
                        "--project-id",
                        "test-project",
                        "--language",
                        "english",
                        "--host-cli-system",
                        "codex",
                        "--json"
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());
                assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());

                let config_path = harness.path().join("vida.config.yaml");
                let config = fs::read_to_string(&config_path).expect("config should exist");
                let updated = config.replace(
                    "      execution_class: internal\n",
                    "      execution_class: internal\n      max_runtime_seconds: 1\n",
                );
                fs::write(&config_path, updated).expect("config should update");

                let fake_bin = harness.path().join("fake-bin");
                fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
                let fake_codex = fake_codex_path(&fake_bin);
                write_fake_codex_timeout(&fake_codex);
                configure_fake_codex_dispatch(harness.path(), &fake_codex);
                let patched_path = prepend_to_path(&fake_bin);
                let _path_guard = EnvVarGuard::set("PATH", &patched_path);

                let run_id = "run-coach-timeout-continue";
                let state_root = harness_state_root(&harness);
                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should open");
                let mut status = crate::taskflow_run_graph::default_run_graph_status(
                    run_id, "coach", "delivery",
                );
                status.task_id = run_id.to_string();
                status.active_node = "coach".to_string();
                status.next_node = Some("verification".to_string());
                status.status = "ready".to_string();
                status.lifecycle_stage = "coach_active".to_string();
                status.policy_gate = "single_task_scope_required".to_string();
                status.handoff_state = "awaiting_coach".to_string();
                status.context_state = "sealed".to_string();
                status.checkpoint_kind = "execution_cursor".to_string();
                status.resume_target = "dispatch.coach_lane".to_string();
                status.recovery_ready = true;
                runtime
                    .block_on(store.record_run_graph_status(&status))
                    .expect("run graph status should record");
                let snapshot_dir = store.root().join("runtime-consumption");
                fs::create_dir_all(&snapshot_dir).expect("runtime-consumption dir should exist");
                let snapshot_path = snapshot_dir.join("final-2026-04-16T00-00-00Z.json");
                let snapshot_path_string = snapshot_path.display().to_string();
                let operator_contracts = crate::build_operator_contracts_envelope(
                    "pass",
                    Vec::new(),
                    Vec::new(),
                    serde_json::json!({
                        "runtime_consumption_latest_snapshot_path": snapshot_path_string.clone(),
                        "latest_run_graph_dispatch_receipt_id": run_id,
                        "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                        "consume_final_surface": "vida taskflow consume final",
                    }),
                );
                let failure_control_evidence =
                    crate::taskflow_consume_resume::build_failure_control_evidence(
                        run_id,
                        &snapshot_path_string,
                    );
                fs::write(
                    &snapshot_path,
                    serde_json::json!({
                        "surface": "vida taskflow consume final",
                        "status": operator_contracts["status"].clone(),
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
                        "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        "release_admission": {
                            "admitted": true,
                            "status": "pass",
                            "blockers": []
                        },
                        "operator_contracts": operator_contracts,
                        "payload": {
                            "dispatch_receipt": {
                                "run_id": run_id
                            },
                            "release_admission": {
                                "admitted": true,
                                "status": "pass",
                                "blockers": []
                            },
                            "failure_control_evidence": failure_control_evidence.clone()
                        },
                        "failure_control_evidence": failure_control_evidence
                    })
                    .to_string(),
                )
                .expect("final snapshot should write");

                let role_selection = RuntimeConsumptionLaneSelection {
                    ok: true,
                    activation_source: "test".to_string(),
                    selection_mode: "fixed".to_string(),
                    fallback_role: "orchestrator".to_string(),
                    request: "continue coach review".to_string(),
                    selected_role: "coach".to_string(),
                    conversational_mode: Some("development".to_string()),
                    single_task_only: true,
                    tracked_flow_entry: Some("dev-pack".to_string()),
                    allow_freeform_chat: false,
                    confidence: "high".to_string(),
                    matched_terms: vec!["continue".to_string(), "coach".to_string()],
                    compiled_bundle: serde_json::Value::Null,
                    execution_plan: json!({
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"],
                                "implementer_activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "coach_activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "verifier_activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    }),
                    reason: "test".to_string(),
                };
                runtime
                    .block_on(
                        store.record_run_graph_dispatch_context(
                            &crate::state_store::RunGraphDispatchContext {
                                run_id: run_id.to_string(),
                                task_id: run_id.to_string(),
                                request_text: "continue coach review".to_string(),
                                role_selection: serde_json::to_value(&role_selection)
                                    .expect("encode role selection"),
                                recorded_at: "2026-04-16T00:00:00Z".to_string(),
                            },
                        ),
                    )
                    .expect("dispatch context should record");

                let receipt = crate::state_store::RunGraphDispatchReceipt {
                    run_id: run_id.to_string(),
                    dispatch_target: "coach".to_string(),
                    dispatch_status: "routed".to_string(),
                    lane_status: "lane_running".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida taskflow consume continue".to_string()),
                    dispatch_command: Some(format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    )),
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: None,
                    downstream_dispatch_target: Some("verification".to_string()),
                    downstream_dispatch_command: Some("vida agent-init".to_string()),
                    downstream_dispatch_note: Some("after review".to_string()),
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: Some("coach".to_string()),
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("middle".to_string()),
                    activation_runtime_role: Some("coach".to_string()),
                    selected_backend: Some("middle".to_string()),
                    recorded_at: "2026-04-16T00:00:00Z".to_string(),
                };
                let run_graph_bootstrap = serde_json::json!({ "run_id": run_id });
                let handoff_plan = serde_json::json!({});
                let ctx = RuntimeDispatchPacketContext::new(
                    &state_root,
                    &role_selection,
                    &receipt,
                    &handoff_plan,
                    &run_graph_bootstrap,
                );
                let dispatch_packet_path =
                    write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
                let mut persisted_receipt = receipt.clone();
                persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path);
                runtime
                    .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
                    .expect("dispatch receipt should record");
                drop(store);

                let started = Instant::now();
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "taskflow", "consume", "continue", "--run-id", run_id, "--json",
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());
                let elapsed = started.elapsed();

                let store = runtime
                    .block_on(StateStore::open_existing(state_root.clone()))
                    .expect("state store should reopen");
                let persisted = runtime
                    .block_on(store.run_graph_dispatch_receipt(run_id))
                    .expect("dispatch receipt should load")
                    .expect("dispatch receipt should exist");
                assert!(
                    elapsed < Duration::from_secs(6),
                    "expected consume continue to return promptly on coach timeout, got {:?}",
                    elapsed
                );
                assert_eq!(persisted.dispatch_status, "routed");
                assert_eq!(persisted.lane_status, "lane_open");
                assert!(persisted.blocker_code.is_none());
                assert!(persisted.dispatch_result_path.is_none());
                assert_eq!(
                    persisted.downstream_dispatch_blockers,
                    vec!["pending_review_clean_evidence".to_string()]
                );
            },
        );
    }

    #[test]
    fn execute_and_record_dispatch_receipt_times_out_when_internal_detached_descendant_keeps_pipe_open(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let mut config_yaml: serde_yaml::Value =
            serde_yaml::from_str(&config).expect("config yaml should parse");
        set_yaml_u64(
            &mut config_yaml,
            &[
                "host_environment",
                "systems",
                "codex",
                "max_runtime_seconds",
            ],
            1,
        );
        set_yaml_u64(
            &mut config_yaml,
            &[
                "agent_system",
                "routing",
                "implementation",
                "max_runtime_seconds",
            ],
            1,
        );
        fs::write(
            &config_path,
            serde_yaml::to_string(&config_yaml).expect("config yaml should encode"),
        )
        .expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_detached_timeout(&fake_codex);
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness
            .path()
            .join("internal-host-detached-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-internal-host-detached-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-internal-host-detached-timeout"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-detached-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("detached descendant timeout should persist blocked result");
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(15),
            "expected detached descendant timeout wrapper to return within a bounded window, got {:?}",
            elapsed
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("dispatch result json should parse");
        assert_eq!(parsed["status"], "blocked");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(
            parsed["blocker_code"],
            INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert_eq!(parsed["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn execute_and_record_dispatch_receipt_releases_authoritative_lock_while_internal_codex_runs() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 5\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_delayed_success(&fake_codex);
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness
            .path()
            .join("internal-host-lock-release-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-internal-host-lock-release",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-internal-host-lock-release"
        });
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-lock-release".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let state_root_dispatch = state_root.clone();
        let run_graph_bootstrap_dispatch = run_graph_bootstrap.clone();
        let dispatch = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
            let mut receipt = receipt;
            runtime
                .block_on(execute_and_record_dispatch_receipt(
                    &state_root_dispatch,
                    &role_selection,
                    &run_graph_bootstrap_dispatch,
                    &mut receipt,
                ))
                .expect("dispatch receipt should execute without holding authoritative lock");
            receipt
        });

        thread::sleep(Duration::from_millis(250));
        let probe_runtime =
            tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let probe_started = Instant::now();
        let probe_store = probe_runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store reopen should succeed while dispatch is in flight");
        drop(probe_store);
        let probe_elapsed = probe_started.elapsed();

        let receipt = dispatch
            .join()
            .expect("dispatch thread should join successfully");
        assert!(
            probe_elapsed < Duration::from_secs(1),
            "expected concurrent store reopen during dispatch to finish quickly, got {:?}",
            probe_elapsed
        );
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_running");
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
    }

    #[test]
    fn execute_and_record_dispatch_receipt_persists_in_flight_runtime_truth_while_internal_codex_runs(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 5\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_codex_path(&fake_bin);
        write_fake_codex_delayed_success(&fake_codex);
        configure_fake_codex_dispatch(harness.path(), &fake_codex);
        let patched_path = prepend_to_path(&fake_bin);
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let run_graph_status = crate::state_store::RunGraphStatus {
            run_id: "run-in-flight-dispatch".to_string(),
            task_id: "task-in-flight-dispatch".to_string(),
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
        runtime
            .block_on(store.record_run_graph_status(&run_graph_status))
            .expect("run graph status should persist");
        drop(store);
        let dispatch_packet_path = harness.path().join("in-flight-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-in-flight-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-in-flight-dispatch"
        });
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-in-flight-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let state_root_dispatch = state_root.clone();
        let run_graph_bootstrap_dispatch = run_graph_bootstrap.clone();
        let dispatch = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
            let mut receipt = receipt;
            runtime
                .block_on(execute_and_record_dispatch_receipt(
                    &state_root_dispatch,
                    &role_selection,
                    &run_graph_bootstrap_dispatch,
                    &mut receipt,
                ))
                .expect("dispatch receipt should execute");
            receipt
        });

        let probe_runtime =
            tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let deadline = Instant::now() + Duration::from_secs(5);
        let (in_flight_receipt, in_flight_status) = loop {
            let probe_store = probe_runtime
                .block_on(StateStore::open_existing(state_root.clone()))
                .expect("state store reopen should succeed while dispatch is in flight");
            let receipt = probe_runtime
                .block_on(probe_store.run_graph_dispatch_receipt("run-in-flight-dispatch"))
                .expect("in-flight receipt should load");
            let status = probe_runtime
                .block_on(probe_store.run_graph_status("run-in-flight-dispatch"))
                .ok();
            drop(probe_store);
            if let (Some(receipt), Some(status)) = (receipt, status) {
                break (receipt, status);
            }
            assert!(
                Instant::now() < deadline,
                "in-flight receipt should exist before dispatch completes"
            );
            thread::sleep(Duration::from_millis(50));
        };

        assert_eq!(in_flight_receipt.dispatch_status, "executing");
        assert_eq!(in_flight_receipt.lane_status, "lane_running");
        assert!(in_flight_receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        assert_eq!(in_flight_status.active_node, "implementer");
        assert_eq!(in_flight_status.lifecycle_stage, "implementer_active");
        assert_eq!(in_flight_status.handoff_state, "none");
        assert_eq!(in_flight_status.status, "running");
        assert!(!in_flight_status.recovery_ready);

        let receipt = dispatch
            .join()
            .expect("dispatch thread should join successfully");
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_running");
    }

    #[test]
    fn execute_runtime_dispatch_handoff_times_out_configured_external_backend() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        set_test_subagent_dispatch_command(
            &config_path,
            "opencode_cli",
            "sh",
            &["-lc", "sleep 30", "vida-dispatch"],
        );
        set_test_subagent_dispatch_timeout(&config_path, "opencode_cli", 1);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-dispatch-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("opencode_cli"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-dispatch-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external timeout dispatch should render");
        let elapsed = started.elapsed();

        assert_eq!(result["surface"], "external_cli:opencode_cli");
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "timeout_without_takeover_authority");
        assert!(
            elapsed < Duration::from_secs(4),
            "expected external timeout wrapper to return promptly, got {:?}",
            elapsed
        );
        assert!(result["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));
        assert!(
            result["exit_code"].is_null() || result["exit_code"].as_i64().is_some(),
            "expected timeout path to record an exit code value or null signal exit, got {:?}",
            result["exit_code"]
        );
        assert_eq!(result["timeout_wrapper"]["timeout_seconds"], 1);
        assert_eq!(result["timeout_wrapper"]["kill_after_grace_seconds"], 1);
        assert_eq!(result["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn execute_runtime_dispatch_handoff_times_out_configured_external_backend_with_detached_descendant(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        set_test_subagent_dispatch_command(
            &config_path,
            "opencode_cli",
            "sh",
            &["-lc", "setsid sh -c 'sleep 30' & exit 0", "vida-dispatch"],
        );
        set_test_subagent_dispatch_timeout(&config_path, "opencode_cli", 1);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness
            .path()
            .join("external-agent-detached-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-detached-dispatch-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: {
                let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
                execution_plan["runtime_assignment"] = serde_json::json!({
                    "selected_model_profile_id": "opencode_codex_mini_review",
                    "selected_model_ref": "opencode/gpt-5.1-codex-mini",
                    "selected_reasoning_effort": "low"
                });
                execution_plan
            },
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-detached-dispatch-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external detached timeout dispatch should render");
        let elapsed = started.elapsed();

        assert_eq!(result["surface"], "external_cli:opencode_cli");
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "timeout_without_takeover_authority");
        assert!(
            elapsed < Duration::from_secs(4),
            "expected detached external timeout wrapper to return promptly, got {:?}",
            elapsed
        );
        assert_eq!(result["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_external_host_internal_backend_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-internal-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-internal-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("internal_subagents"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-internal-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("hybrid external-host internal-backend dispatch should stay on agent-init");

        assert_eq!(result["surface"], "vida agent-init");
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["host_runtime"]["selected_cli_system"], "qwen");
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(result["backend_dispatch"]["backend_class"], "internal");
        assert_eq!(
            result["backend_dispatch"]["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            result["backend_dispatch"]["policy_selected_internal_backend"],
            true
        );
        assert_eq!(result["blocker_code"], "internal_activation_view_only");
    }

    #[test]
    fn runtime_agent_lane_dispatch_prefers_receipt_selected_backend_for_external_hosts() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("hermes_cli"),
            None,
        );

        assert_eq!(dispatch.surface, "external_cli:hermes_cli");
        assert!(
            dispatch.activation_command.contains("hermes"),
            "expected hermes command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_id"], "hermes_cli");
    }

    #[test]
    fn runtime_agent_lane_dispatch_projects_selected_external_model_profile_into_activation_command(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        install_external_cli_test_model_profiles(&config_path);

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("opencode_cli"),
            Some("opencode_codex_mini_review"),
        );

        assert_eq!(dispatch.surface, "external_cli:opencode_cli");
        assert!(
            dispatch
                .activation_command
                .contains("opencode/gpt-5.1-codex-mini"),
            "expected selected profile model in activation command, got {}",
            dispatch.activation_command
        );
        assert!(
            !dispatch
                .activation_command
                .contains("opencode/minimax-m2.5-free"),
            "did not expect default model in activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(
            dispatch.backend_dispatch["selected_model_profile_id"],
            "opencode_codex_mini_review"
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_keeps_internal_hosts_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            None,
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert!(
            dispatch.activation_command.contains("vida agent-init"),
            "expected canonical internal activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_honors_preferred_external_backend_for_internal_hosts() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("hermes_cli"),
            None,
        );

        assert_eq!(dispatch.surface, "external_cli:hermes_cli");
        assert!(
            dispatch.activation_command.contains("hermes"),
            "expected hermes command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "external_cli");
        assert_eq!(dispatch.backend_dispatch["backend_id"], "hermes_cli");
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_external_backend"],
            true
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_surfaces_disabled_preferred_external_backend_for_internal_hosts()
    {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        fs::write(
            &config_path,
            config.replace(
                "hermes_cli:\n  enabled: true\n",
                "hermes_cli:\n  enabled: false\n",
            ),
        )
        .expect("config should disable hermes");

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("hermes_cli"),
            None,
        );

        assert_eq!(dispatch.surface, "external_cli:hermes_cli");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "external_cli");
        assert_eq!(dispatch.backend_dispatch["backend_id"], "hermes_cli");
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_external_backend"],
            true
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_keeps_policy_selected_internal_backend_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("internal_subagents"),
            Some("internal_review"),
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert!(
            dispatch.activation_command.contains("vida agent-init"),
            "expected canonical internal activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            dispatch.backend_dispatch["selected_model_profile_id"],
            "internal_review"
        );
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_does_not_project_internal_codex_carrier_to_codex_exec() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "    internal_subagents:\n      enabled: true\n      subagent_backend_class: internal\n",
            concat!(
                "    internal_subagents:\n",
                "      enabled: true\n",
                "      subagent_backend_class: internal\n",
                "      role: internal_primary_fixture\n",
                "      default_model: gpt-5.5\n",
                "    external_fixture:\n",
                "      enabled: false\n",
                "      subagent_backend_class: external_cli\n",
                "      role: bridge_fallback\n",
            ),
        );
        fs::write(&config_path, updated).expect("config should update");

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("internal_subagents"),
            Some("gpt-5.5"),
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            "internal_subagents"
        );
        assert_eq!(dispatch.backend_dispatch["executor_backend"], "internal");
        assert!(
            !dispatch.activation_command.contains("exec --json"),
            "internal host carrier must not be rendered as an external CLI bridge: {}",
            dispatch.activation_command
        );
        assert!(
            !dispatch.activation_command.contains("gpt-5.5"),
            "internal host model id must not be passed to a host CLI bridge: {}",
            dispatch.activation_command
        );
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_internal_host_external_backend_hint_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        install_external_cli_test_model_profiles(&config_path);
        set_test_subagent_dispatch_command(
            &config_path,
            "opencode_cli",
            "sh",
            &[
                "-lc",
                "printf 'external-dispatch:%s' \"$*\"",
                "vida-dispatch",
            ],
        );

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-external-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-external-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-external-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("internal host should honor explicit external backend hints");

        assert_eq!(result["surface"], "external_cli:opencode_cli");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["host_runtime"]["selected_cli_system"], "codex");
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "internal"
        );
        assert_eq!(result["backend_dispatch"]["backend_id"], "opencode_cli");
        assert_eq!(result["backend_dispatch"]["backend_class"], "external_cli");
        let activation_command = result["activation_command"]
            .as_str()
            .expect("activation command should render");
        assert!(!activation_command.trim().is_empty());
        assert!(!activation_command.contains("vida agent-init"));
        assert!(result["blocker_code"].is_null());
    }

    #[test]
    fn execute_runtime_dispatch_handoff_executes_configured_external_backend() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        install_external_cli_test_model_profiles(&config_path);
        set_test_subagent_dispatch_command(
            &config_path,
            "opencode_cli",
            "sh",
            &[
                "-lc",
                "printf 'external-dispatch:%s' \"$1\"",
                "vida-dispatch",
            ],
        );

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let mut execution_plan = agent_lane_test_execution_plan("opencode_cli");
        execution_plan["runtime_assignment"] = serde_json::json!({
            "selected_model_profile_id": "opencode_codex_mini_review",
            "selected_model_ref": "opencode/gpt-5.1-codex-mini",
            "selected_reasoning_effort": "low"
        });
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external agent-lane dispatch handoff should execute");

        assert_eq!(result["surface"], "external_cli:opencode_cli");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert!(result["blocker_code"].is_null());
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(result["backend_dispatch"]["backend_id"], "opencode_cli");
        assert!(result["activation_command"]
            .as_str()
            .expect("activation command should render")
            .contains("opencode/gpt-5.1-codex-mini"));
        assert!(!result["activation_command"]
            .as_str()
            .expect("activation command should render")
            .contains("opencode/minimax-m2.5-free"));
        assert_eq!(
            result["backend_dispatch"]["selected_model_profile_id"],
            "opencode_codex_mini_review"
        );
        assert!(result["provider_output"]
            .as_str()
            .expect("provider output should render")
            .contains("external-dispatch:--model"));
        assert_eq!(result["role_selection"]["selected_role"], "worker");
    }

    #[test]
    fn runtime_dispatch_packet_carries_external_host_runtime_contract() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let state_root = harness.path().join(".vida/data/state");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement backend execution in crates/vida/src/runtime_dispatch_state.rs"
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "qwen-primary",
                            "activation_runtime_role": "worker",
                            "closure_class": "implementation",
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-qwen-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
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
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen-primary".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let packet = crate::read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");
        assert_eq!(packet["host_runtime"]["selected_cli_system"], "qwen");
        assert_eq!(
            packet["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(packet["host_runtime"]["runtime_template_root"], ".qwen");
        assert_eq!(packet["selected_backend"], "qwen-primary");
        assert_eq!(
            packet["effective_execution_posture"]["selected_cli_system"],
            "qwen"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_execution_class"],
            "external"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_backend"],
            "qwen-primary"
        );
        assert_eq!(
            packet["effective_execution_posture"]["route_primary_backend"],
            "qwen-primary"
        );
        assert_eq!(
            packet["effective_execution_posture"]["activation_evidence_state"],
            "activation_view_only"
        );
    }

    #[test]
    fn downstream_receipt_backend_prefers_activation_agent_type() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "junior",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let root_receipt = RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
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
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let downstream = build_downstream_dispatch_receipt(&role_selection, &root_receipt)
            .expect("downstream receipt should build");
        assert_eq!(downstream.activation_agent_type.as_deref(), Some("junior"));
        assert_eq!(downstream.selected_backend.as_deref(), Some("junior"));
    }

    #[test]
    fn spec_pack_downstream_routes_to_specification_lane_when_agent_only_enabled() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
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
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("specification"));
        assert_eq!(command.as_deref(), Some("vida agent-init"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn spec_pack_downstream_canonicalizes_explicit_business_analyst_lane_alias() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_sequence": ["business_analyst", "implementer"],
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
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
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );

        assert_eq!(target.as_deref(), Some("specification"));
        assert_eq!(command.as_deref(), Some("vida agent-init"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn spec_pack_downstream_ready_when_tracked_design_and_spec_task_are_closed() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let design_doc_path = harness.path().join("feature-x-design.md");
        write_approved_design_doc(&design_doc_path);
        let role_selection = specification_test_role_selection(
            "feature-x-spec-task",
            &design_doc_path.display().to_string(),
        );
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec-pack-closed".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("/tmp/spec-pack-result.json".to_string()),
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
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-05-12T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(harness_state_root(&harness)))
            .expect("state store should initialize");
        runtime.block_on(create_and_close_task(&store, "feature-x-spec-task"));

        let (target, command, note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );

        assert_eq!(target.as_deref(), Some("work-pool-pack"));
        assert_eq!(
            command.as_deref(),
            Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
            )
        );
        assert!(ready);
        assert!(blockers.is_empty());
        assert!(note
            .as_deref()
            .unwrap_or_default()
            .contains("design document is finalized"));
    }

    #[test]
    fn packet_ready_specification_lane_stays_active_while_work_pool_handoff_remains_blocked() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("work-pool-pack"));
        assert_eq!(
            command.as_deref(),
            Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
            )
        );
        assert!(!ready);
        assert!(blockers.contains(&"pending_specification_evidence".to_string()));
        assert!(blockers.contains(&"pending_design_finalize".to_string()));
        assert!(blockers.contains(&"pending_spec_task_close".to_string()));
        assert_eq!(
            active_downstream_dispatch_target(&receipt).as_deref(),
            Some("specification")
        );
        assert!(note
            .as_deref()
            .unwrap_or_default()
            .contains("wait for bounded evidence return"));
    }

    #[test]
    fn specification_downstream_activation_uses_specification_contract() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let (_kind, surface, agent_type, runtime_role) =
            downstream_activation_fields(&role_selection, "specification");
        assert_eq!(surface.as_deref(), Some("vida agent-init"));
        assert_eq!(agent_type.as_deref(), Some("middle"));
        assert_eq!(runtime_role.as_deref(), Some("business_analyst"));
    }

    #[test]
    fn downstream_dispatch_packet_uses_tracked_design_doc_scope_for_specification() {
        let role_selection = specification_test_role_selection(
            "feature-x-spec-task",
            "docs/product/spec/feature-x-design.md",
        );
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("specification".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
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
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let packet = downstream_dispatch_packet_body(
            &role_selection,
            &serde_json::Value::Null,
            &receipt,
            None,
        );

        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
        );
    }

    #[test]
    fn runtime_downstream_dispatch_packet_uses_runtime_assignment_activation_fallback() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue writer lane".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {},
                "orchestration_contract": {},
                "runtime_assignment": {
                    "activation_agent_type": "configured_writer_backend",
                    "activation_runtime_role": "worker",
                    "selected_backend_id": "configured_writer_backend"
                },
                "development_flow": {
                    "dispatch_contract": {}
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "configured_writer_backend",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-writer".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/analysis.json".to_string()),
            dispatch_result_path: Some("/tmp/analysis-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("writer".to_string()),
            downstream_dispatch_last_target: Some("analysis".to_string()),
            activation_agent_type: Some("configured_analyst_backend".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("configured_analyst_backend".to_string()),
            recorded_at: "2026-05-12T00:00:00Z".to_string(),
        };

        let packet = downstream_dispatch_packet_body(
            &role_selection,
            &json!({ "run_id": "run-writer" }),
            &receipt,
            None,
        );

        assert_eq!(packet["activation_agent_type"], "configured_writer_backend");
        assert_eq!(packet["activation_runtime_role"], "worker");
        assert_eq!(
            packet["delivery_task_packet"]["handoff_runtime_role"],
            "worker"
        );
    }

    #[test]
    fn route_selected_backend_for_specification_prefers_contract_activation_tier() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "specification_activation": {
                        "activation_agent_type": "middle",
                    },
                }
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "specification");

        assert_eq!(backend.as_deref(), Some("middle"));
    }

    #[test]
    fn downstream_packet_preview_does_not_inherit_upstream_exception_or_supersession_evidence() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "activation": {
                            "activation_agent_type": "junior"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-impl".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: Some("sup-parent".to_string()),
            exception_path_receipt_id: Some("exc-parent".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("handoff to implementer".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/downstream-preview.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: Some("specification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let packet = downstream_dispatch_packet_body(
            &role_selection,
            &serde_json::json!({ "run_id": "run-impl" }),
            &receipt,
            None,
        );

        assert!(packet["downstream_supersedes_receipt_id"].is_null());
        assert!(packet["downstream_exception_path_receipt_id"].is_null());
        assert_eq!(packet["downstream_lane_status"], "packet_ready");
        assert_eq!(packet["source_supersedes_receipt_id"], "sup-parent");
        assert_eq!(packet["source_exception_path_receipt_id"], "exc-parent");
    }

    #[test]
    fn downstream_receipt_does_not_inherit_upstream_exception_or_supersession_evidence() {
        let role_selection = mixed_backend_role_selection();
        let mut implementer_receipt = executed_agent_lane_receipt(
            "implementer",
            "internal_subagents",
            "junior",
            "worker",
            Some("coach"),
        );
        implementer_receipt.lane_status = "lane_exception_recorded".to_string();
        implementer_receipt.supersedes_receipt_id = Some("sup-parent".to_string());
        implementer_receipt.exception_path_receipt_id = Some("exc-parent".to_string());

        let downstream_receipt =
            build_downstream_dispatch_receipt(&role_selection, &implementer_receipt)
                .expect("downstream receipt should build");

        assert_eq!(downstream_receipt.dispatch_target, "coach");
        assert_eq!(downstream_receipt.dispatch_status, "routed");
        assert!(downstream_receipt.supersedes_receipt_id.is_none());
        assert!(downstream_receipt.exception_path_receipt_id.is_none());
        assert_eq!(downstream_receipt.lane_status, "lane_open");
    }

    #[test]
    fn root_receipt_fields_from_downstream_step_clears_inherited_exception_or_supersession_ids() {
        let mut root_receipt = executed_agent_lane_receipt(
            "implementer",
            "internal_subagents",
            "junior",
            "worker",
            Some("closure"),
        );
        root_receipt.supersedes_receipt_id = Some("sup-parent".to_string());
        root_receipt.exception_path_receipt_id = Some("exc-parent".to_string());

        let mut downstream_receipt = executed_agent_lane_receipt(
            "closure",
            "internal_subagents",
            "senior",
            "verifier",
            None,
        );
        downstream_receipt.supersedes_receipt_id = Some("sup-parent".to_string());
        downstream_receipt.exception_path_receipt_id = Some("exc-parent".to_string());
        downstream_receipt.dispatch_status = "blocked".to_string();
        downstream_receipt.lane_status = "lane_exception_recorded".to_string();

        root_receipt_fields_from_downstream_step(&mut root_receipt, &downstream_receipt);

        assert!(root_receipt.supersedes_receipt_id.is_none());
        assert!(root_receipt.exception_path_receipt_id.is_none());
        assert_eq!(
            root_receipt.downstream_dispatch_active_target.as_deref(),
            downstream_receipt
                .downstream_dispatch_active_target
                .as_deref()
        );
    }

    #[test]
    fn route_selected_backend_for_implementer_keeps_explicit_route_hint() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "executor_backend": "opencode_cli",
                    "activation": {
                        "activation_agent_type": "middle",
                    },
                },
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "implementer");

        assert_eq!(backend.as_deref(), Some("opencode_cli"));
    }

    #[test]
    fn route_selected_backend_for_analysis_uses_execution_plan_runtime_assignment() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {}
            },
            "runtime_assignment": {
                "selected_tier": "junior",
                "activation_agent_type": "junior"
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "analysis");

        assert_eq!(backend.as_deref(), Some("junior"));
    }

    #[test]
    fn route_selected_backend_for_analysis_prefers_explicit_analysis_route() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "analysis": {
                    "executor_backend": "analysis_cli"
                },
                "implementation": {
                    "executor_backend": "opencode_cli"
                }
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "analysis");

        assert_eq!(backend.as_deref(), Some("analysis_cli"));
    }

    #[test]
    fn effective_execution_posture_keeps_backend_class_unknown_without_matrix_row() {
        let summary = effective_execution_posture_summary(
            &serde_json::json!({}),
            "coach",
            Some("opencode_cli"),
            None,
            None,
            false,
            None,
        );

        assert_eq!(summary["selected_backend"], "opencode_cli");
        assert_eq!(summary["selected_backend_class"], "unknown");
    }

    #[test]
    fn effective_execution_posture_infers_internal_backend_class_from_activation_agent_type_on_internal_host(
    ) {
        let summary = effective_execution_posture_summary(
            &serde_json::json!({}),
            "analysis",
            Some("junior"),
            Some("junior"),
            Some(&serde_json::json!({
                "selected_cli_system": "codex",
                "selected_cli_execution_class": "internal"
            })),
            false,
            None,
        );

        assert_eq!(summary["selected_backend"], "junior");
        assert_eq!(summary["selected_backend_class"], "internal");
        assert_eq!(summary["effective_posture_kind"], "internal");
    }

    #[test]
    fn effective_execution_posture_canonicalizes_inadmissible_implementer_backend_to_fallback() {
        let summary = effective_execution_posture_summary(
            &serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            "implementer",
            Some("opencode_cli"),
            Some("junior"),
            None,
            false,
            None,
        );

        assert_eq!(summary["selected_backend"], "internal_subagents");
        assert_eq!(summary["selected_backend_source"], "route_fallback_hint");
        assert_eq!(summary["selected_backend_class"], "internal");
        assert_eq!(summary["route_primary_backend"], "opencode_cli");
    }

    #[test]
    fn executed_worker_lane_sets_downstream_ready_without_evidence_blocker() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("coach"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn routed_analysis_lane_does_not_surface_evidence_blocker_before_execution() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "implementation": {
                        "analysis_route_task_class": "analysis",
                        "writer_route_task_class": "writer"
                    },
                    "dispatch_contract": {
                        "execution_lane_sequence": ["analysis", "writer", "coach"],
                        "analysis_activation": {
                            "completion_blocker": "pending_analysis_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "writer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-analysis-routed".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("analysis-packet.json".to_string()),
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
            recorded_at: "2026-05-12T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert!(target.is_none());
        assert!(!ready);
        assert!(note
            .as_deref()
            .unwrap_or_default()
            .contains("wait for terminal execution evidence"));
        assert!(blockers.is_empty());
    }

    #[test]
    fn activation_view_only_dispatch_result_surfaces_transport_blocker_and_does_not_unlock_the_next_lane(
    ) {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev-blocked".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("dispatch-result.json".to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-08T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_has_execution_evidence(&receipt));
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("coach"));
        assert!(!ready);
        assert_eq!(blockers, vec!["internal_activation_view_only".to_string()]);
    }

    #[test]
    fn blocked_coach_activation_view_surfaces_transport_blocker_instead_of_review_clean_evidence() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-coach-blocked".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after coach evidence, activate verification".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );

        assert_eq!(target.as_deref(), Some("verification"));
        assert!(!ready);
        assert_eq!(blockers, vec!["internal_activation_view_only".to_string()]);
    }

    #[test]
    fn duplicate_lane_sequence_uses_previous_target_to_advance_second_coach_to_verification() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["test_author", "coach", "implementer", "coach", "verification"],
                        "test_author_activation": {
                            "completion_blocker": "pending_test_author_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "worker"
                        },
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-duplicate-coach".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
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
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-25T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );

        assert_eq!(target.as_deref(), Some("verification"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn refresh_downstream_dispatch_preview_unblocks_dev_handoff_after_work_pool_execution() {
        let root = std::env::temp_dir().join(format!(
            "vida-refresh-downstream-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("work-pool-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-work-pool",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-work-pool".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("dev-pack".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_work_pool_shape".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                root.join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        runtime
            .block_on(refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("refresh should succeed");

        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(
            receipt.downstream_dispatch_command.as_deref(),
            Some("vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json")
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bounded_implementer_task_close_bridges_downstream_receipt_to_coach_ready() {
        let root = std::env::temp_dir().join(format!(
            "vida-implementer-bridge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("state store should open");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-bridge-dev",
                title: "Bridge dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");
        store
            .close_task("feature-bridge-dev", "implemented and proven")
            .await
            .expect("task should close");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-bridge-dev",
                        "ensure_command": "vida task ensure feature-bridge-dev \"Bridge dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-bridge",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(
            try_bridge_bounded_implementer_completion_to_downstream_receipt(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("bridge should succeed")
        );
        assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt.blocker_code.is_none());
        assert!(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bounded_implementer_bridge_stays_blocked_while_dev_task_is_open() {
        let root = std::env::temp_dir().join(format!(
            "vida-implementer-bridge-open-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("state store should open");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: "feature-bridge-open-dev",
                title: "Open bridge dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-bridge-open-dev",
                        "ensure_command": "vida task ensure feature-bridge-open-dev \"Open bridge dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-bridge-open",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge-open".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(
            !try_bridge_bounded_implementer_completion_to_downstream_receipt(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("bridge should evaluate cleanly")
        );
        assert!(!receipt.downstream_dispatch_ready);
        assert_eq!(
            receipt.downstream_dispatch_blockers,
            vec!["pending_implementation_evidence".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn downstream_preview_ready_blocker_parity_guard_detects_inconsistency() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            super::downstream_dispatch_ready_blocker_parity_error(true, &blockers),
            Some(
                "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                    .to_string()
            )
        );
        assert!(super::downstream_dispatch_ready_blocker_parity_error(false, &blockers).is_none());
    }

    #[test]
    fn context_preserves_inputs() {
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test-mode".to_string(),
            fallback_role: "junior".to_string(),
            request: "req".to_string(),
            selected_role: "junior".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: json!({}),
            execution_plan: json!({ "orchestration_contract": {}, "tracked_flow_bootstrap": {} }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
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
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let execution_plan_value = json!({"plan": "value"});
        let bootstrap_value = json!({"bootstrap": "value"});
        let ctx = RuntimeDispatchPacketContext::new(
            Path::new("/tmp"),
            &selection,
            &receipt,
            &execution_plan_value,
            &bootstrap_value,
        );
        assert_eq!(ctx.receipt.run_id, "run-test");
        assert_eq!(ctx.role_selection.request, "req");
    }

    #[test]
    fn downstream_packet_uses_next_lane_activation_for_dev_pack_handoff() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue bounded Release-1 work for task r1-04-a".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("pbi_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["task".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {},
                "orchestration_contract": {},
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "junior",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev-pack".to_string(),
            dispatch_target: "dev-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
            dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after the dev packet is created, activate the selected implementer lane for bounded execution"
                    .to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("executed".to_string()),
            downstream_dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("dev-pack".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-06T06:47:13Z".to_string(),
        };

        let packet = downstream_dispatch_packet_body(
            &role_selection,
            &json!({ "run_id": "run-dev-pack" }),
            &receipt,
            None,
        );

        assert_eq!(packet["packet_template_kind"], "delivery_task_packet");
        assert_eq!(packet["activation_agent_type"], "junior");
        assert_eq!(packet["activation_runtime_role"], "worker");
        assert_eq!(packet["selected_backend"], "junior");
        assert_eq!(packet["mixed_posture"]["route_primary_backend"], "junior");
        assert_eq!(packet["route_policy"]["route_primary_backend"], "junior");
        assert_eq!(
            packet["activation_vs_execution_evidence"]["evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            packet["activation_semantics"]["activation_kind"],
            "activation_view"
        );
        assert!(packet["execution_evidence"].is_null());
        assert_eq!(
            packet["effective_execution_posture"]["route_primary_backend"],
            "junior"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_backend"],
            "junior"
        );
        assert_eq!(
            packet["effective_execution_posture"]["mixed_route_backends"],
            false
        );
        assert_eq!(
            packet["effective_execution_posture"]["activation_evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            packet["delivery_task_packet"]["handoff_runtime_role"],
            "worker"
        );
    }

    #[test]
    fn dispatch_surface_truth_prefers_receipt_result_evidence_over_packet_activation_view() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-surface-truth-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("temp root should exist");
        let packet_path = root.join("dispatch-packet.json");
        let result_path = root.join("dispatch-result.json");

        fs::write(
            &packet_path,
            json!({
                "activation_vs_execution_evidence": {
                    "activation_kind": "activation_view",
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "mixed_posture": {
                    "effective_posture_kind": "external_only"
                }
            })
            .to_string(),
        )
        .expect("packet should write");
        fs::write(
            &result_path,
            json!({
                "artifact_kind": "runtime_dispatch_result",
                "activation_semantics": {
                    "activation_kind": "execution_evidence"
                },
                "execution_evidence": {
                    "status": "recorded",
                    "backend_id": "internal_subagents"
                },
                "execution_state": "executed"
            })
            .to_string(),
        )
        .expect("result should write");

        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-status-truth".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("implementer".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let truth = dispatch_surface_truth_from_packet_path(
            &root,
            Some(packet_path.to_str().expect("packet path should be utf8")),
            &receipt,
        )
        .expect("surface truth should resolve");

        assert_eq!(
            truth["activation_vs_execution_evidence"]["evidence_state"],
            "execution_evidence_recorded"
        );
        assert_eq!(
            truth["activation_vs_execution_evidence"]["receipt_backed"],
            true
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn closed_tracked_dev_task_counts_as_implementer_evidence_for_preview() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let receipt = RunGraphDispatchReceipt {
                run_id: "run-bridge".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: None,
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
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
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            let (target, command, _note, ready, blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;
            assert_eq!(target.as_deref(), Some("coach"));
            assert_eq!(command.as_deref(), Some("vida agent-init"));
            assert!(ready);
            assert!(blockers.is_empty());
        });
    }

    #[test]
    fn latest_receipt_bridge_persists_ready_coach_handoff_after_task_close() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-bridge" });
            let taskflow_handoff_plan = build_taskflow_handoff_plan(&role_selection);
            let mut receipt = bridge_waiting_root_receipt("run-bridge");
            let ctx = RuntimeDispatchPacketContext::new(
                &state_root,
                &role_selection,
                &receipt,
                &taskflow_handoff_plan,
                &run_graph_bootstrap,
            );
            let dispatch_packet_path =
                write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
            receipt.dispatch_packet_path = Some(dispatch_packet_path);
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "run-bridge".to_string(),
                    task_id: "run-bridge".to_string(),
                    task_class: "pbi_discussion".to_string(),
                    active_node: "dev-pack".to_string(),
                    next_node: None,
                    status: "ready".to_string(),
                    route_task_class: "work-pool-pack".to_string(),
                    selected_backend: "taskflow_state_store".to_string(),
                    lane_id: "dev_pack_direct".to_string(),
                    lifecycle_stage: "dev_pack_active".to_string(),
                    policy_gate: "single_task_scope_required".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "conversation_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: true,
                })
                .await
                .expect("run-graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&receipt)
                .await
                .expect("receipt should persist");
            let bridged = maybe_bridge_closed_implementer_task_into_receipt(
                &store,
                &mut receipt,
                Some("feature-x-dev"),
            )
            .await
            .expect("bridge should succeed");
            assert!(bridged);
            store
                .record_run_graph_dispatch_receipt(&receipt)
                .await
                .expect("bridged receipt should persist");

            let persisted = store
                .run_graph_dispatch_receipt("run-bridge")
                .await
                .expect("receipt should load")
                .expect("receipt should exist");
            assert_eq!(
                persisted.downstream_dispatch_target.as_deref(),
                Some("coach")
            );
            assert!(persisted.downstream_dispatch_ready);
            assert!(persisted.downstream_dispatch_blockers.is_empty());
            assert_eq!(
                persisted.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = persisted
                .downstream_dispatch_result_path
                .as_deref()
                .expect("bridged downstream evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "implementer");
            assert_eq!(
                evidence["completion_receipt_id"],
                "task-close-feature-x-dev"
            );
            assert!(persisted
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()));
            let packet_path = persisted
                .downstream_dispatch_packet_path
                .as_deref()
                .expect("downstream packet path should exist");
            let packet: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(packet_path).expect("downstream packet should be readable"),
            )
            .expect("downstream packet should decode");
            let prompt = packet["prompt"]
                .as_str()
                .expect("downstream packet prompt should be a string");
            assert!(prompt.contains("Runtime role=coach"));
            assert!(prompt.contains("Review/proof lane contract"));
            assert!(!prompt.contains(
                "First substantive response: publish a concise plan before edits or implementation."
            ));
            assert!(prompt.contains("Do not run root-only orchestration commands"));
            assert!(!prompt.contains("vida taskflow consume continue --json"));
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_marks_ready_packets_as_packet_ready_with_result_path() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-preview" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-preview".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
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
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
            assert!(receipt.downstream_dispatch_ready);
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = receipt
                .downstream_dispatch_result_path
                .as_deref()
                .expect("synthetic execution evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "implementer");
            assert_eq!(
                evidence["completion_receipt_id"],
                "receipt-executed-run-refresh-preview-implementer"
            );
            assert!(receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()));
        });
    }

    #[test]
    fn dev_pack_handoff_stays_blocked_without_owned_write_scope_for_implementer() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-missing-scope".to_string(),
                dispatch_target: "dev-pack".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "taskflow_pack".to_string(),
                dispatch_surface: Some("vida task ensure".to_string()),
                dispatch_command: Some("vida task ensure".to_string()),
                dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
                dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
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
                downstream_dispatch_active_target: Some("dev-pack".to_string()),
                downstream_dispatch_last_target: Some("dev-pack".to_string()),
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("taskflow_state_store".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            };

            let (next_target, _, _, next_ready, next_blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;

            assert_eq!(next_target.as_deref(), Some("implementer"));
            assert!(!next_ready);
            assert_eq!(next_blockers, vec!["missing_owned_write_scope".to_string()]);
        });
    }

    #[test]
    fn derive_downstream_dispatch_preview_routes_analysis_evidence_to_first_execution_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let mut role_selection = bridge_test_role_selection("feature-x-dev");
            role_selection.request =
                "Implement the bounded writer fix in crates/vida/src/runtime_dispatch_state.rs"
                    .to_string();
            role_selection.execution_plan["development_flow"]["implementation"] = json!({
                "analysis_route_task_class": "analysis",
                "writer_route_task_class": "writer"
            });
            let receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-analysis-preview".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            };

            let (next_target, command, note, next_ready, next_blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;

            assert_eq!(next_target.as_deref(), Some("writer"));
            assert_eq!(command.as_deref(), Some("vida agent-init"));
            assert!(next_ready);
            assert!(next_blockers.is_empty());
            assert!(note
                .as_deref()
                .unwrap_or_default()
                .contains("validation evidence is recorded"));
        });
    }

    #[test]
    fn derive_downstream_dispatch_preview_blocks_analysis_to_writer_without_owned_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let mut role_selection = bridge_test_role_selection("feature-x-dev");
            role_selection.request = "continue development".to_string();
            role_selection.execution_plan["development_flow"]["implementation"] = json!({
                "analysis_route_task_class": "analysis",
                "writer_route_task_class": "writer"
            });
            let receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-analysis-missing-scope-preview".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            };

            let (next_target, command, note, next_ready, next_blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;

            assert_eq!(next_target.as_deref(), Some("writer"));
            assert_eq!(command.as_deref(), Some("vida agent-init"));
            assert!(!next_ready);
            assert_eq!(next_blockers, vec!["missing_owned_write_scope".to_string()]);
            assert!(note
                .as_deref()
                .unwrap_or_default()
                .contains("validation evidence is recorded"));
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_uses_task_owned_paths_for_writer_handoff() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let owned_paths = vec!["crates/vida/src/runtime_dispatch_state.rs".to_string()];
            let labels = vec!["runtime-recovery".to_string()];
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: "run-analysis-task-metadata-preview",
                    title: "Runtime recovery",
                    display_id: None,
                    description: "runtime recovery",
                    issue_type: "task",
                    status: "open",
                    priority: 1,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata {
                        owned_paths: owned_paths.clone(),
                        ..Default::default()
                    },
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("task with planner metadata should be created");
            let mut role_selection = bridge_test_role_selection("unused-dev-task");
            role_selection.request = "continue development".to_string();
            role_selection.execution_plan["tracked_flow_bootstrap"] = serde_json::Value::Null;
            role_selection.execution_plan["development_flow"]["implementation"] = json!({
                "analysis_route_task_class": "analysis",
                "writer_route_task_class": "writer"
            });
            let run_graph_bootstrap = json!({ "run_id": "run-analysis-task-metadata-preview" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-analysis-task-metadata-preview".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should use task-owned paths");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("writer")
            );
            assert!(receipt.downstream_dispatch_ready);
            assert!(receipt.downstream_dispatch_blockers.is_empty());
            let packet_path = receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .expect("downstream packet should be written");
            let packet = read_json(harness.path(), packet_path);
            assert_eq!(
                packet["delivery_task_packet"]["owned_paths"],
                serde_json::json!(owned_paths)
            );
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_uses_takeover_owned_paths_for_writer_handoff() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let owned_paths = vec![
                "crates/vida/src".to_string(),
                "crates/vida/tests".to_string(),
            ];
            let mut role_selection = bridge_test_role_selection("unused-dev-task");
            role_selection.request = "continue development".to_string();
            role_selection.execution_plan["tracked_flow_bootstrap"] = serde_json::Value::Null;
            role_selection.execution_plan["development_flow"]["implementation"] = json!({
                "analysis_route_task_class": "analysis",
                "writer_route_task_class": "writer"
            });
            let run_graph_bootstrap = json!({ "run_id": "run-analysis-takeover-preview" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-analysis-takeover-preview".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview_with_owned_paths(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
                &owned_paths,
            )
            .await
            .expect("preview should use takeover-owned paths");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("writer")
            );
            assert!(receipt.downstream_dispatch_ready);
            assert!(receipt.downstream_dispatch_blockers.is_empty());
            let packet_path = receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .expect("downstream packet should be written");
            let packet = read_json(harness.path(), packet_path);
            assert_eq!(
                packet["delivery_task_packet"]["owned_paths"],
                serde_json::json!(owned_paths)
            );
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_filters_unsafe_task_owned_paths() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let labels = vec!["runtime-recovery".to_string()];
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: "run-analysis-task-metadata-unsafe-preview",
                    title: "Runtime recovery",
                    display_id: None,
                    description: "runtime recovery",
                    issue_type: "task",
                    status: "open",
                    priority: 1,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata {
                        owned_paths: vec![
                            "../../.ssh/config".to_string(),
                            "/home/user/.bashrc".to_string(),
                            "./local.rs".to_string(),
                            "crates/vida/src/runtime_dispatch_state.rs".to_string(),
                        ],
                        ..Default::default()
                    },
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("task with planner metadata should be created");
            let mut role_selection = bridge_test_role_selection("unused-dev-task");
            role_selection.request = "continue development".to_string();
            role_selection.execution_plan["tracked_flow_bootstrap"] = serde_json::Value::Null;
            role_selection.execution_plan["development_flow"]["implementation"] = json!({
                "analysis_route_task_class": "analysis",
                "writer_route_task_class": "writer"
            });
            let run_graph_bootstrap =
                json!({ "run_id": "run-analysis-task-metadata-unsafe-preview" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-analysis-task-metadata-unsafe-preview".to_string(),
                dispatch_target: "analysis".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
                downstream_dispatch_active_target: Some("analysis".to_string()),
                downstream_dispatch_last_target: Some("analysis".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-23T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should filter unsafe task-owned paths");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("writer")
            );
            assert!(receipt.downstream_dispatch_ready);
            let packet_path = receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .expect("downstream packet should be written");
            let packet = read_json(harness.path(), packet_path);
            assert_eq!(
                packet["delivery_task_packet"]["owned_paths"],
                serde_json::json!(["crates/vida/src/runtime_dispatch_state.rs"])
            );
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_does_not_mark_implementer_packet_ready_without_owned_scope(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-missing-scope" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-missing-scope".to_string(),
                dispatch_target: "dev-pack".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "taskflow_pack".to_string(),
                dispatch_surface: Some("vida task ensure".to_string()),
                dispatch_command: Some("vida task ensure".to_string()),
                dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
                dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
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
                downstream_dispatch_active_target: Some("dev-pack".to_string()),
                downstream_dispatch_last_target: Some("dev-pack".to_string()),
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("taskflow_state_store".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should fail closed into a blocked state, not an error");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("implementer")
            );
            assert!(!receipt.downstream_dispatch_ready);
            assert_eq!(receipt.downstream_dispatch_status, None);
            assert_eq!(
                receipt.downstream_dispatch_blockers,
                vec!["missing_owned_write_scope".to_string()]
            );
            assert!(receipt.downstream_dispatch_packet_path.is_none());
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_uses_task_close_completion_evidence_for_blocked_implementer(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-closed-task" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-closed-task".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/activation-view.json".to_string()),
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
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
            assert!(receipt.downstream_dispatch_ready);
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = receipt
                .downstream_dispatch_result_path
                .as_deref()
                .expect("task-close bridge evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "implementer");
            assert_eq!(
                evidence["completion_receipt_id"],
                "task-close-feature-x-dev"
            );
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maybe_bridge_closed_implementer_task_into_receipt_promotes_blocked_implementer_timeout(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(state_root.clone())
            .await
            .expect("state store should open");
        create_and_close_task(&store, "feature-x-dev").await;

        let role_selection = bridge_test_role_selection("feature-x-dev");
        let run_graph_bootstrap = json!({ "run_id": "run-bridge-blocked-implementer" });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-bridge-blocked-implementer".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/activation-view-only.json".to_string()),
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
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let bridged = maybe_bridge_closed_implementer_task_into_receipt_with_context(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
            Some("feature-x-dev"),
        )
        .await
        .expect("bridge should succeed");

        assert!(bridged);
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt.blocker_code.is_none());
        assert!(receipt.exception_path_receipt_id.is_none());
        assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
        assert!(receipt.downstream_dispatch_ready);
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        let evidence_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("bridged dispatch evidence path should exist");
        let evidence = read_json(harness.path(), evidence_path);
        assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
        assert_eq!(evidence["completed_target"], "implementer");
        assert_eq!(
            evidence["completion_receipt_id"],
            "task-close-feature-x-dev"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maybe_bridge_closure_ready_verification_into_receipt_requires_receipt_backed_evidence()
    {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(state_root.clone())
            .await
            .expect("state store should open");
        let verification_result_path = harness.path().join("verification-proof.json");
        fs::write(
            &verification_result_path,
            json!({
                "artifact_kind": "verification_evidence",
                "status": "clean"
            })
            .to_string(),
        )
        .expect("verification evidence should persist");

        let role_selection = bridge_test_role_selection("feature-x-dev");
        let run_graph_bootstrap = json!({ "run_id": "run-bridge-blocked-verification" });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-bridge-blocked-verification".to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/activation-view-only.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(verification_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let bridged =
            maybe_reconcile_blocked_verification_timeout_with_receipt_evidence_with_admission(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
                Some(true),
            )
            .await
            .expect("reconcile should return");

        assert!(!bridged);
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            receipt.exception_path_receipt_id.as_deref(),
            Some("exc-timeout")
        );
        assert_eq!(
            receipt.dispatch_result_path.as_deref(),
            Some("/tmp/activation-view-only.json")
        );
    }

    #[test]
    fn refresh_downstream_dispatch_preview_unblocks_work_pool_handoff_after_spec_task_closure() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-spec").await;
            let design_doc_path = harness.path().join("feature-x-spec-design.md");
            write_approved_design_doc(&design_doc_path);

            let role_selection = specification_test_role_selection(
                "feature-x-spec",
                design_doc_path
                    .to_str()
                    .expect("design doc path should be utf-8"),
            );
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-spec-closed-task" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-spec-closed-task".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
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
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("work-pool-pack")
            );
            assert!(receipt.downstream_dispatch_ready);
            assert!(receipt.downstream_dispatch_blockers.is_empty());
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = receipt
                .downstream_dispatch_result_path
                .as_deref()
                .expect("specification task-close evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "specification");
            assert_eq!(
                evidence["completion_receipt_id"],
                "task-close-feature-x-spec"
            );
            assert!(receipt
                .downstream_dispatch_note
                .as_deref()
                .unwrap_or_default()
                .contains("spec-pack is closed"));
        });
    }

    #[test]
    fn downstream_receipt_prefers_dynamic_runtime_backend_over_route_executor_hint() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "subagents": "legacy_hint_should_not_win"
                    },
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_complete".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `coach` evidence is recorded, activate `verification`".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        let downstream = build_downstream_dispatch_receipt(&role_selection, &receipt)
            .expect("downstream receipt should build");

        assert_eq!(downstream.activation_agent_type.as_deref(), Some("middle"));
        assert_eq!(downstream.activation_runtime_role.as_deref(), Some("coach"));
        assert_eq!(downstream.selected_backend.as_deref(), Some("middle"));
    }

    #[tokio::test]
    async fn try_bridge_bounded_specification_completion_to_downstream_receipt_sets_packet_ready() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-specification-bridge-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.join("state"))
            .await
            .expect("open state store");

        let spec_task_id = "feature-spec-bridge-spec";
        let design_doc_path = root.join("docs/specification-bridge-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Specification Bridge\n\nStatus: `approved`\n",
        )
        .expect("write approved design doc");

        let labels = vec!["spec-pack".to_string()];
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: spec_task_id,
                title: "Closed spec pack",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed spec task");

        let role_selection = specification_test_role_selection(
            spec_task_id,
            design_doc_path
                .to_str()
                .expect("design doc path should be utf-8"),
        );
        let run_graph_bootstrap = json!({ "run_id": "run-spec-bridge-ready" });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-spec-bridge-ready".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-spec-bridge".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/specification-started.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    .to_string(),
            ),
            downstream_dispatch_note: Some("waiting on specification evidence".to_string()),
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
            downstream_dispatch_last_target: Some("specification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let bridged = try_bridge_bounded_specification_completion_to_downstream_receipt(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .await
        .expect("specification bridge should succeed");

        assert!(bridged);
        assert_eq!(receipt.dispatch_status, "executed");
        assert!(receipt.dispatch_result_path.is_some());
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("work-pool-pack")
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(receipt.downstream_dispatch_packet_path.is_some());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn maybe_bridge_closed_specification_task_into_receipt_ignores_other_task_ids() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-specification-bridge-filter-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.join("state"))
            .await
            .expect("open state store");

        let spec_task_id = "feature-spec-bridge-filter-spec";
        let design_doc_path = root.join("docs/specification-bridge-filter-design.md");
        std::fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
            .expect("create design doc directory");
        std::fs::write(
            &design_doc_path,
            "# Specification Bridge Filter\n\nStatus: `approved`\n",
        )
        .expect("write approved design doc");

        let labels = vec!["spec-pack".to_string()];
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: spec_task_id,
                title: "Closed spec pack",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed spec task");

        let role_selection = specification_test_role_selection(
            spec_task_id,
            design_doc_path
                .to_str()
                .expect("design doc path should be utf-8"),
        );
        let packet_path = root.join("specification-dispatch-packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_string_pretty(&json!({
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": "run-spec-bridge-filter" }
            }))
            .expect("dispatch packet should encode"),
        )
        .expect("dispatch packet should write");
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-spec-bridge-filter".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-spec-bridge-filter".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some("/tmp/specification-started.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    .to_string(),
            ),
            downstream_dispatch_note: Some("waiting on specification evidence".to_string()),
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
            downstream_dispatch_last_target: Some("specification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let bridged = maybe_bridge_closed_specification_task_into_receipt(
            &store,
            &mut receipt,
            Some("other-task"),
        )
        .await
        .expect("bridge should evaluate cleanly");

        assert!(!bridged);
        assert_eq!(receipt.dispatch_status, "executing");
        assert!(!receipt.downstream_dispatch_ready);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn backend_agnostic_route_keeps_inherited_selected_backend() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(
                &role_selection,
                "implementer",
                Some("junior"),
                Some("junior")
            ),
            Some("junior".to_string())
        );
    }

    #[test]
    fn downstream_selected_backend_prefers_admissible_fallback_when_primary_is_inadmissible() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents"
                    },
                    "implementer": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(&role_selection, "implementer", Some("junior"), None),
            Some("internal_subagents".to_string())
        );
        assert_eq!(
            downstream_selected_backend(&role_selection, "writer", Some("junior"), None),
            Some("internal_subagents".to_string())
        );
    }

    #[test]
    fn downstream_selected_backend_prefers_admissible_fallback_for_verification_when_primary_is_inadmissible(
    ) {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue verification".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "verification": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "verification": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "verification": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(
                &role_selection,
                "verification",
                Some("senior"),
                Some("hermes_cli")
            ),
            Some("internal_subagents".to_string())
        );
    }

    #[test]
    fn downstream_selected_backend_prefers_explicit_runtime_assignment_over_internal_verification_fallback(
    ) {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue verification".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "verification": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "carrier_runtime_assignment": {
                            "selected_backend_id": "pi_cli",
                            "selected_carrier_id": "pi_cli",
                            "selected_model_profile_id": "pi_gpt55_high_readonly",
                            "activation_agent_type": "pi_cli",
                            "activation_runtime_role": "verifier"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "verification": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "verification": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(
                &role_selection,
                "verification",
                Some("senior"),
                Some("hermes_cli")
            ),
            Some("pi_cli".to_string())
        );
    }

    #[test]
    fn downstream_selected_backend_resolves_analysis_from_implementation_runtime_assignment() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "analysis": {}
                },
                "runtime_assignment": {
                    "selected_tier": "junior",
                    "activation_agent_type": "junior"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "junior",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(&role_selection, "analysis", Some("junior"), None),
            Some("junior".to_string())
        );
    }

    #[test]
    fn apply_first_handoff_execution_advances_executed_implementer_into_downstream_handoff() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-advance-implementer",
            "implementation",
            "implementation",
        );
        status.task_id = "run-advance-implementer".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-advance-implementer".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after implementer evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };

        let advanced = apply_first_handoff_execution_to_run_graph_status(&status, &receipt);

        assert_eq!(advanced.active_node, "implementer");
        assert_eq!(advanced.next_node.as_deref(), Some("coach"));
        assert_eq!(advanced.handoff_state, "awaiting_coach");
        assert_eq!(advanced.resume_target, "dispatch.coach");
        assert_eq!(advanced.lifecycle_stage, "implementer_active");
    }

    #[test]
    fn apply_first_handoff_execution_does_not_complete_exception_recorded_closure() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-exception",
            "closure",
            "delivery",
        );
        status.task_id = "run-closure-exception".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "closure_active".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-closure-exception".to_string(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-1".to_string()),
            dispatch_kind: "closure".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
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
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let advanced = apply_first_handoff_execution_to_run_graph_status(&status, &receipt);

        assert_eq!(advanced.active_node, "closure");
        assert_eq!(advanced.status, "blocked");
        assert_eq!(advanced.lifecycle_stage, "closure_active");
        assert_eq!(advanced.resume_target, "none");
        assert!(!advanced.recovery_ready);
    }

    #[test]
    fn write_runtime_dispatch_result_records_completion_evidence_for_executed_agent_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-completion-evidence".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:qwen".to_string()),
            dispatch_command: Some("configured-host run".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "internal_cli:qwen",
                "status": "pass",
                "execution_state": "executed",
                "provider_result": "implemented"
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(artifact["artifact_kind"], "runtime_lane_completion_result");
        assert_eq!(artifact["run_id"], "run-completion-evidence");
        assert_eq!(artifact["completed_target"], "implementer");
        assert_eq!(
            artifact["source_dispatch_packet_path"],
            "/tmp/implementer-packet.json"
        );
        assert_eq!(artifact["provider_result"], "implemented");
        assert!(artifact["completion_receipt_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("dispatch-completion-")));
        assert!(artifact["recorded_at"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty()));
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["artifact_type"],
            "lane_execution_receipt"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["owner_surface"],
            "runtime_dispatch_state"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["run_id"],
            "run-completion-evidence"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["packet_id"],
            "implementer-packet"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["lane_role"],
            "worker"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["carrier_id"],
            "junior"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["lane_status"],
            "lane_completed"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["evidence_status"],
            "recorded"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["result_artifact_ids"][0],
            path
        );
        assert_eq!(
            artifact["activation_vs_execution_evidence"]["evidence_state"],
            "execution_evidence_recorded"
        );
        assert_eq!(
            artifact["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(artifact["activation_semantics"]["view_only"], false);
        assert_eq!(artifact["activation_semantics"]["executes_packet"], true);
        assert_eq!(artifact["execution_evidence"]["status"], "recorded");
        assert_eq!(
            artifact["execution_evidence"]["evidence_kind"],
            "lane_execution_receipt_artifact"
        );
        assert_eq!(artifact["execution_evidence"]["backend_id"], "junior");
        assert_eq!(artifact["execution_evidence"]["result_path"], path);
    }

    #[test]
    fn write_runtime_dispatch_result_keeps_blocked_agent_lane_as_dispatch_artifact() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-blocked-dispatch".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only"
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(artifact["artifact_kind"], "runtime_dispatch_result");
        assert_eq!(artifact["run_id"], "run-blocked-dispatch");
        assert_eq!(artifact["dispatch_target"], "coach");
        assert_eq!(artifact["blocker_code"], "internal_activation_view_only");
        assert!(artifact.get("completion_receipt_id").is_none());
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["artifact_type"],
            "lane_execution_receipt"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["packet_id"],
            "coach-packet"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["carrier_id"],
            "middle"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["lane_status"],
            "lane_blocked"
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["exception_path_receipt_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["result_artifact_ids"][0],
            path
        );
        assert_eq!(
            artifact["activation_vs_execution_evidence"]["evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            artifact["activation_semantics"]["activation_kind"],
            "activation_view"
        );
        assert_eq!(artifact["activation_semantics"]["view_only"], true);
        assert_eq!(artifact["activation_semantics"]["executes_packet"], false);
        assert!(artifact["execution_evidence"].is_null());
    }

    #[test]
    fn write_runtime_dispatch_result_uses_effective_backend_for_lane_receipt_after_fallback() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-readiness-fallback-dispatch".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("configured-host run".to_string()),
            dispatch_packet_path: Some("/tmp/coach-fallback-packet.json".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "internal_cli:codex",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
                "backend_dispatch": {
                    "backend_class": "internal",
                    "backend_id": "middle",
                    "carrier_id": "middle"
                },
                "execution_truth": {
                    "effective_selected_backend": "middle",
                    "selected_backend_source": "dynamic_runtime_selection"
                }
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(
            artifact["lane_execution_receipt_artifact"]["carrier_id"],
            "middle"
        );
        assert_ne!(
            artifact["lane_execution_receipt_artifact"]["carrier_id"],
            "hermes_cli"
        );
        assert_eq!(artifact["backend_dispatch"]["backend_id"], "middle");
    }

    #[test]
    fn write_runtime_dispatch_result_omits_lane_receipt_for_in_flight_execution() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-executing-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "vida agent-init",
                "status": "pass",
                "execution_state": "executing",
                "note": "handoff started"
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(artifact["artifact_kind"], "runtime_dispatch_result");
        assert_eq!(artifact["execution_state"], "executing");
        assert!(artifact.get("lane_execution_receipt_artifact").is_none());
    }

    #[test]
    fn dispatch_receipt_has_execution_evidence_rejects_activation_view_only_result() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness
            .path()
            .join("runtime-consumption/dispatch-results/run-activation-view-only.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create dispatch result dir");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-activation-view-only".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale downstream preview".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        assert!(
            !dispatch_receipt_has_execution_evidence(&receipt),
            "activation-view-only result must not count as execution evidence"
        );
    }

    #[test]
    fn runtime_dispatch_execution_timeout_result_marks_blocked_timeout_receipt() {
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let result = runtime_dispatch_execution_timeout_result(&receipt, 10);
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(
            result["blocker_code"],
            blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority)
                .expect("timeout blocker should stay registry-backed")
        );
        assert!(result["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 10s"));
    }

    #[test]
    fn dispatch_result_stale_after_seconds_prefers_artifact_value_and_keeps_legacy_fallback() {
        let explicit = serde_json::json!({
            "stale_after_seconds": 39
        });
        assert_eq!(dispatch_result_stale_after_seconds(&explicit), 39);

        let legacy = serde_json::json!({});
        assert_eq!(dispatch_result_stale_after_seconds(&legacy), 10);
    }

    #[test]
    fn dispatch_packet_declares_activation_view_only_from_activation_semantics() {
        let root = std::env::temp_dir().join(format!(
            "vida-activation-view-only-packet-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let packet_path = root.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                }
            }))
            .expect("packet json should encode"),
        )
        .expect("packet should write");

        assert!(dispatch_packet_declares_activation_view_only(Some(
            &packet_path.display().to_string()
        )));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_packet_declares_activation_view_only_honors_executable_delivery_task_packet_evidence(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-executable-template-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let packet_path = root.join("delivery-task-packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "analysis",
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "delivery_task_packet": {
                    "goal": "Execute bounded analysis handoff"
                }
            }))
            .expect("packet json should encode"),
        )
        .expect("packet should write");

        assert!(dispatch_packet_declares_activation_view_only(Some(
            &packet_path.display().to_string()
        )));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_packet_declares_activation_view_only_allows_executable_packet_without_view_only_evidence(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-executable-template-ready-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let packet_path = root.join("delivery-task-packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "analysis",
                "delivery_task_packet": {
                    "goal": "Execute bounded analysis handoff"
                }
            }))
            .expect("packet json should encode"),
        )
        .expect("packet should write");

        assert!(!dispatch_packet_declares_activation_view_only(Some(
            &packet_path.display().to_string()
        )));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn execute_and_record_dispatch_receipt_blocks_internal_activation_view_only_packet_without_launch(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        std::fs::create_dir_all(harness.path().join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project")).expect("project dir");
        std::fs::write(harness.path().join("AGENTS.md"), "test").expect("agents marker");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 240
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config should write");
        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-activation-view-only-fast-block",
            "specification",
            "specification",
        );
        status.task_id = "run-activation-view-only-fast-block".to_string();
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run-graph status should record");
        drop(store);
        let dispatch_packet_path = harness.path().join("activation-view-only-spec-packet.json");
        std::fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "dispatch_target": "specification",
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "effective_execution_posture": {
                    "selected_execution_class": "internal"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Create the bounded TaskFlow case catalog.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "specification": true
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "specification": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-activation-view-only-fast-block"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-activation-view-only-fast-block".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("pm".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("activation-view-only internal dispatch should block without launching");
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "activation-view-only dispatch should not wait for the internal host window, got {:?}",
            elapsed
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let parsed: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dispatch_result_path).expect("dispatch result should read"),
        )
        .expect("dispatch result should parse");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(
            parsed["blocker_code"],
            INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert!(parsed["provider_error"]
            .as_str()
            .expect("provider_error should render")
            .contains("receipt-backed completion"));
    }

    #[test]
    fn internal_host_prelaunch_blocker_uses_configured_receipt_backed_capability() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        std::fs::create_dir_all(harness.path().join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project")).expect("project dir");
        std::fs::write(harness.path().join("AGENTS.md"), "test").expect("agents marker");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 240
      dispatch:
        command: configured-host-bridge
        receipt_backed_completion_supported: false
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config should write");
        let dispatch_packet_path = harness.path().join("coach-delivery-packet.json");
        std::fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "coach",
                "delivery_task_packet": {
                    "goal": "Execute bounded coach validation"
                },
                "effective_execution_posture": {
                    "selected_execution_class": "internal"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Validate bounded coach lane.".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "coach": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-configured-internal-prelaunch-block".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-18T00:00:00Z".to_string(),
        };

        assert!(internal_host_dispatch_requires_prelaunch_blocker(
            harness.path(),
            &role_selection,
            &receipt
        ));
        assert!(
            internal_host_activation_view_only_requires_terminal_blocker(
                harness.path(),
                &role_selection,
                &receipt
            )
        );
    }

    #[test]
    fn internal_host_bridge_blocks_before_launch_without_receipt_capability_flag() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        std::fs::create_dir_all(harness.path().join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project")).expect("project dir");
        std::fs::write(harness.path().join("AGENTS.md"), "test").expect("agents marker");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: configured_host
  systems:
    configured_host:
      enabled: true
      execution_class: internal
      dispatch:
        command: configured-host
        static_args:
          - run
          - --json
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
    external_fixture:
      enabled: false
      subagent_backend_class: external_cli
"#,
        )
        .expect("config should write");
        let dispatch_packet_path = harness.path().join("internal-host-packet.json");
        std::fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "analysis",
                "delivery_task_packet": {
                    "goal": "Execute bounded implementation analysis"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement bounded task.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "analysis": true,
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "analysis": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            },
                            "implementation": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-codex-prelaunch-block".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-20T00:00:00Z".to_string(),
        };

        assert!(internal_host_dispatch_requires_prelaunch_blocker(
            harness.path(),
            &role_selection,
            &receipt
        ));
        assert_eq!(
            internal_host_activation_view_only_blocker_code(
                harness.path(),
                &role_selection,
                &receipt
            ),
            INTERNAL_CODEX_CARRIER_UNAVAILABLE
        );
    }

    #[test]
    fn internal_host_prelaunch_yields_to_ready_external_fallback() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        std::fs::create_dir_all(harness.path().join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project")).expect("project dir");
        std::fs::write(harness.path().join("AGENTS.md"), "test").expect("agents marker");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: configured_host
  systems:
    configured_host:
      enabled: true
      execution_class: internal
      dispatch:
        command: configured-host
        static_args: ["run", "--json"]
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-lc", "printf '{\"type\":\"result\",\"is_error\":false,\"result\":\"ok\"}'"]
        prompt_mode: positional
"#,
        )
        .expect("config should write");
        let dispatch_packet_path = harness.path().join("internal-host-fallback-packet.json");
        std::fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "analysis",
                "delivery_task_packet": {
                    "goal": "Execute bounded implementation analysis"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement bounded task.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "analysis": true,
                            "implementation": true
                        }
                    },
                    {
                        "backend_id": "qwen_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "analysis": {
                        "executor_backend": "internal_subagents",
                        "fallback_executor_backend": "qwen_cli",
                        "fanout_executor_backends": ["qwen_cli"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-codex-ready-fallback".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-20T00:00:00Z".to_string(),
        };

        assert!(internal_host_external_fallback_backend(
            &role_selection,
            &receipt.dispatch_target,
            "internal_subagents",
            &load_project_overlay_yaml_for_root(harness.path()).expect("overlay")
        )
        .is_some());
        assert!(
            !internal_host_dispatch_requires_prelaunch_blocker(
                harness.path(),
                &role_selection,
                &receipt
            ),
            "ready external fallback should be executed instead of terminal prelaunch blocker"
        );
    }

    #[test]
    fn internal_host_prelaunch_allows_configured_receipt_backed_completion() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        std::fs::create_dir_all(harness.path().join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(harness.path().join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(harness.path().join(".vida/project")).expect("project dir");
        std::fs::write(harness.path().join("AGENTS.md"), "test").expect("agents marker");
        std::fs::write(
            harness.path().join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: configured_host
  systems:
    configured_host:
      enabled: true
      execution_class: internal
      dispatch:
        command: configured-host
        static_args: ["run", "--json"]
        receipt_backed_completion_supported: true
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config should write");
        let dispatch_packet_path = harness.path().join("internal-host-receipt-packet.json");
        std::fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "dispatch_target": "analysis",
                "delivery_task_packet": {
                    "goal": "Execute bounded implementation analysis"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement bounded task.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "analysis": true,
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "analysis": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            },
                            "implementation": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-receipt-backed".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-22T00:00:00Z".to_string(),
        };

        assert!(
            !internal_host_dispatch_requires_prelaunch_blocker(
                harness.path(),
                &role_selection,
                &receipt
            ),
            "receipt-backed dispatch support should allow configured internal carrier execution"
        );
    }

    #[test]
    fn stale_in_flight_dispatch_timeout_seconds_uses_internal_host_window_for_legacy_artifact() {
        let root = std::env::temp_dir().join(format!(
            "vida-legacy-internal-stale-timeout-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let state_root = root.join(crate::state_store::default_state_dir());
        std::fs::create_dir_all(&state_root).expect("state root");
        std::fs::create_dir_all(root.join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(root.join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(root.join(".vida/project")).expect("project dir");
        std::fs::write(root.join("AGENTS.md"), "test").expect("agents");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
"#,
        )
        .expect("config");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-legacy-internal-timeout".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        let timeout_seconds = stale_in_flight_dispatch_timeout_seconds_for_receipt(
            &state_root,
            &receipt,
            &serde_json::json!({
                "surface": "internal_cli:codex",
                "backend_dispatch": {
                    "backend_class": "internal"
                }
            }),
        );

        assert_eq!(timeout_seconds, 39);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_keeps_young_downstream_carrier_receipt_running() {
        let root = std::env::temp_dir().join(format!(
            "vida-young-downstream-inflight-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let state_root = root.join(crate::state_store::default_state_dir());
        let dispatch_results = state_root
            .join("runtime-consumption")
            .join("dispatch-results");
        let dispatch_packets = state_root
            .join("runtime-consumption")
            .join("downstream-dispatch-packets");
        std::fs::create_dir_all(&dispatch_results).expect("dispatch results");
        std::fs::create_dir_all(&dispatch_packets).expect("dispatch packets");
        let packet_path = dispatch_packets.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({ "packet_kind": "runtime_downstream_dispatch_packet" }).to_string(),
        )
        .expect("packet");
        let result_path = dispatch_results.join("result.json");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "execution_state": "executing",
                "recorded_at": time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .expect("timestamp"),
                "stale_after_seconds": 422,
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("result");
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-young-downstream-inflight".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        assert!(
            !normalize_stale_in_flight_dispatch_receipt(&state_root, &mut receipt)
                .expect("normalization should succeed"),
            "young downstream-carrier in-flight receipts must not be normalized to timeout"
        );
        assert_eq!(receipt.dispatch_status, "executing");
        assert_eq!(receipt.blocker_code, None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_dispatch_handoff_timeout_to_receipt_keeps_internal_activation_semantics_for_internal_host(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-timeout-classification-internal-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
      carriers:
        junior:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: low
"#,
        )
        .expect("config");
        let state_root = root.join(crate::state_store::default_state_dir());
        std::fs::create_dir_all(
            state_root
                .join("runtime-consumption")
                .join("dispatch-results"),
        )
        .expect("dispatch result dir");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Analyze the bounded runtime blocker.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["analysis".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "analysis": {
                        "executor_backend": "junior"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-timeout-internal-host".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        apply_dispatch_handoff_timeout_to_receipt(
            &state_root,
            &root,
            &role_selection,
            &mut receipt,
            39,
        )
        .expect("timeout classification should persist");

        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let artifact: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should persist"),
            )
            .expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should persist");
        assert_eq!(artifact["dispatch_result_path"], dispatch_result_path);
        assert_eq!(artifact["receipt_path"], dispatch_result_path);
        assert_eq!(artifact["receipt_status"], "blocked");
        assert_eq!(
            artifact["lane_execution_receipt_path"],
            dispatch_result_path
        );
        assert_eq!(
            artifact["blocker_code"],
            INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert!(artifact["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 39s"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_dispatch_handoff_timeout_to_receipt_keeps_generic_timeout_for_non_internal_handoff() {
        let root = std::env::temp_dir().join(format!(
            "vida-timeout-classification-generic-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: external
"#,
        )
        .expect("config");
        let state_root = root.join(crate::state_store::default_state_dir());
        std::fs::create_dir_all(
            state_root
                .join("runtime-consumption")
                .join("dispatch-results"),
        )
        .expect("dispatch result dir");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-timeout-generic".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        apply_dispatch_handoff_timeout_to_receipt(
            &state_root,
            &root,
            &role_selection,
            &mut receipt,
            10,
        )
        .expect("timeout classification should persist");

        assert_eq!(
            receipt.blocker_code.as_deref(),
            blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority).as_deref()
        );
        let artifact: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should persist"),
            )
            .expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");
        assert_eq!(
            artifact["blocker_code"],
            blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority)
                .expect("timeout blocker should stay registry-backed")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn coordinate_dispatch_timeout_state_best_effort_persists_receipt_and_rebinds_continuation() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let state_root = harness_state_root(&harness);

        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-timeout-coordinate",
            "implementation",
            "implementation",
        );
        status.task_id = "run-timeout-coordinate".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run-graph status should record");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Fix the bounded timeout coordination path.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["timeout".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        runtime
            .block_on(store.record_run_graph_dispatch_context(
                &crate::state_store::RunGraphDispatchContext {
                    run_id: "run-timeout-coordinate".to_string(),
                    task_id: "run-timeout-coordinate".to_string(),
                    request_text: "Fix the bounded timeout coordination path.".to_string(),
                    role_selection:
                        serde_json::to_value(&role_selection).expect("encode role selection"),
                    recorded_at: "2026-04-22T00:00:00Z".to_string(),
                },
            ))
            .expect("dispatch context should record");
        drop(store);

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-timeout-coordinate".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT.to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach`".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let warning = runtime.block_on(super::coordinate_dispatch_timeout_state_best_effort(
            &state_root,
            &serde_json::json!({ "run_id": "run-timeout-coordinate" }),
            &receipt,
        ));
        assert!(
            warning.is_none(),
            "timeout coordination should succeed: {warning:?}"
        );

        let store = runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store should reopen");
        let persisted_receipt = runtime
            .block_on(store.run_graph_dispatch_receipt("run-timeout-coordinate"))
            .expect("dispatch receipt should load")
            .expect("dispatch receipt should persist");
        assert_eq!(
            persisted_receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert_eq!(persisted_receipt.dispatch_status, "blocked");

        let binding = runtime
            .block_on(store.run_graph_continuation_binding("run-timeout-coordinate"))
            .expect("continuation binding should load")
            .expect("continuation binding should persist");
        assert_eq!(binding.binding_source, "dispatch_execution_timeout");
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");
        assert_eq!(
            binding.request_text.as_deref(),
            Some("Fix the bounded timeout coordination path.")
        );
    }

    #[test]
    fn normalize_activation_view_only_receipt_truth_blocks_terminal_executing_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness
            .path()
            .join("runtime-consumption/dispatch-results/run-terminal-activation-view-only.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create dispatch result dir");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-terminal-activation-view-only".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale coach handoff".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        assert!(normalize_activation_view_only_receipt_truth(&mut receipt)
            .expect("terminal activation-view receipt should normalize"));
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert!(!receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_status.is_none());
    }

    #[test]
    fn normalize_activation_view_only_receipt_truth_keeps_genuine_in_flight_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness
            .path()
            .join("runtime-consumption/dispatch-results/run-live-activation-view-only.json");
        fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create dispatch result dir");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true,
                    "executes_packet": false,
                    "records_completion_receipt": false
                },
                "execution_evidence": null
            }))
            .expect("encode result"),
        )
        .expect("write result");

        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-live-activation-view-only".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        assert!(!normalize_activation_view_only_receipt_truth(&mut receipt)
            .expect("live activation-view receipt should not normalize"));
        assert_eq!(receipt.dispatch_status, "executing");
        assert_eq!(receipt.lane_status, "lane_running");
        assert!(receipt.blocker_code.is_none());
    }

    #[test]
    fn apply_dispatch_handoff_timeout_to_receipt_keeps_generic_timeout_for_external_backend_on_internal_host(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-timeout-classification-external-on-internal-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        no_output_timeout_seconds: 8
"#,
        )
        .expect("config");
        let state_root = root.join(crate::state_store::default_state_dir());
        std::fs::create_dir_all(
            state_root
                .join("runtime-consumption")
                .join("dispatch-results"),
        )
        .expect("dispatch result dir");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded implementation result.".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["review".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "coach": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-timeout-external-on-internal".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        apply_dispatch_handoff_timeout_to_receipt(
            &state_root,
            &root,
            &role_selection,
            &mut receipt,
            10,
        )
        .expect("timeout classification should persist");

        assert_eq!(
            receipt.blocker_code.as_deref(),
            blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority).as_deref()
        );
        let artifact: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should persist"),
            )
            .expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");
        assert_eq!(
            artifact["blocker_code"],
            blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority)
                .expect("timeout blocker should stay registry-backed")
        );
        assert!(artifact["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("runtime dispatch handoff timed out"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_timeout_seconds_respects_internal_host_runtime_window() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-timeout-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "max_runtime_seconds": 41
                    },
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-internal-handoff-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            43
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_execution_started_stale_after_seconds_keeps_internal_worker_runtime_window() {
        let root = std::env::temp_dir().join(format!(
            "vida-inflight-stale-no-output-timeout-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch:
        no_output_timeout_seconds: 2
agent_system:
  routing:
    implementation:
      max_runtime_seconds: 420
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-internal-no-output-stale".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            422
        );
        assert_eq!(
            dispatch_execution_started_stale_after_seconds(&root, &role_selection, &receipt),
            422
        );
        assert_eq!(
            dispatch_execution_timeout_seconds(&root, &role_selection, &receipt),
            422
        );

        let mut direct_internal_receipt = receipt.clone();
        direct_internal_receipt.dispatch_surface = Some("internal_cli:codex".to_string());
        direct_internal_receipt.dispatch_command = Some("codex exec --json".to_string());
        assert_eq!(
            dispatch_execution_started_stale_after_seconds(
                &root,
                &role_selection,
                &direct_internal_receipt
            ),
            422
        );

        let mut direct_internal_middle_receipt = receipt.clone();
        direct_internal_middle_receipt.dispatch_target = "test_author".to_string();
        direct_internal_middle_receipt.dispatch_surface = Some("internal_cli:codex".to_string());
        direct_internal_middle_receipt.dispatch_command = Some("codex exec --json".to_string());
        direct_internal_middle_receipt.selected_backend = Some("middle".to_string());
        direct_internal_middle_receipt.activation_agent_type = Some("middle".to_string());
        assert_eq!(
            dispatch_execution_started_stale_after_seconds(
                &root,
                &role_selection,
                &direct_internal_middle_receipt
            ),
            242,
            "configured internal CLI no-output guards must not shrink the background worker stale window"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_timeout_seconds_uses_configured_route_window_when_selection_omits_it() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-timeout-config-route-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  routing:
    implementation:
      max_runtime_seconds: 420
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-internal-config-route-timeout".to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            422
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_timeout_seconds_treats_internal_host_carrier_role_id_as_internal() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-timeout-carrier-role-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
      carriers:
        junior:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: low
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "junior"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-internal-handoff-timeout-carrier-role".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            39
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_requires_outer_timeout_for_internal_host_agent_lane() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-outer-timeout-internal-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "backend_id": "internal_subagents",
                                "backend_class": "internal"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-internal-handoff-outer-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        assert!(dispatch_handoff_requires_outer_timeout(
            &root,
            &role_selection,
            &receipt,
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_requires_outer_timeout_is_disabled_for_external_agent_lane() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-outer-timeout-external-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded packet and return proof.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-external-handoff-outer-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        assert!(!dispatch_handoff_requires_outer_timeout(
            &root,
            &role_selection,
            &receipt,
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn execute_and_record_dispatch_receipt_times_out_external_backend_without_reverting_to_agent_init(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        set_test_subagent_dispatch_command(
            &config_path,
            "hermes_cli",
            "sh",
            &["-lc", "trap \"\" TERM; sleep 30", "vida-dispatch"],
        );
        set_test_subagent_dispatch_timeout(&config_path, "hermes_cli", 1);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-record-timeout.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-record-timeout",
                    "coach",
                    "coach",
                    "coach",
                    "coach",
                    "Review the bounded packet and return proof."
                ),
                "dispatch_target": "coach",
                "request_text": "Review the bounded packet and return proof.",
                "activation_runtime_role": "coach",
                "role_selection": {
                    "selected_role": "coach"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded packet and return proof.".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "coach": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-external-record-timeout"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-record-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("external timeout dispatch receipt should persist bounded result");
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(6),
            "expected external timeout wrapper to return within a bounded window, got {:?}",
            elapsed
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
        assert_eq!(
            receipt.dispatch_surface.as_deref(),
            Some("external_cli:hermes_cli")
        );
        assert!(receipt
            .dispatch_command
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty() && !value.contains("vida agent-init")));
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("dispatch result json should parse");
        assert_eq!(parsed["surface"], "external_cli:hermes_cli");
        assert_eq!(parsed["blocker_code"], "timeout_without_takeover_authority");
        assert_eq!(parsed["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn dispatch_handoff_timeout_seconds_respects_external_backend_runtime_window() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-timeout-external-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      max_runtime_seconds: 37
      dispatch:
        no_output_timeout_seconds: 37
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded packet and return proof.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-external-handoff-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            39
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatch_handoff_timeout_seconds_prefers_external_max_runtime_over_no_output_window() {
        let root = std::env::temp_dir().join(format!(
            "vida-handoff-timeout-external-max-runtime-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      max_runtime_seconds: 420
      dispatch:
        no_output_timeout_seconds: 180
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Analyze the bounded packet and return proof.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "analysis": {
                        "executor_backend": "pi_cli"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "pi_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true
                        }
                    }
                ],
                "runtime_assignment": {
                    "selected_backend_id": "pi_cli",
                    "selected_carrier_id": "pi_cli",
                    "selected_model_profile_id": "pi_gpt55_medium_guarded"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-external-pi-timeout".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida-pi-agent".to_string()),
            dispatch_packet_path: Some("/tmp/analysis-packet.json".to_string()),
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
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T00:00:00Z".to_string(),
        };

        assert_eq!(
            dispatch_handoff_timeout_seconds(&root, &role_selection, &receipt),
            422
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn runtime_dispatch_project_root_from_state_root_prefers_inferred_project_root() {
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-project-root-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let state_root = root.join(crate::state_store::default_state_dir());
        std::fs::create_dir_all(&state_root).expect("state root");
        std::fs::create_dir_all(root.join(".vida/config")).expect("config dir");
        std::fs::create_dir_all(root.join(".vida/db")).expect("db dir");
        std::fs::create_dir_all(root.join(".vida/project")).expect("project dir");
        std::fs::write(root.join("AGENTS.md"), "test").expect("agents");
        std::fs::write(root.join("vida.config.yaml"), "host_environment: {}\n").expect("config");

        let resolved = runtime_dispatch_project_root_from_state_root(&state_root);
        assert_eq!(resolved.as_ref(), root.as_path());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn admissible_selected_backend_prefers_explicit_route_over_inherited_backend_for_coach_lane() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "enabled": true,
                "selected_carrier_id": "senior",
                "selected_model_profile_id": "codex_gpt55_high_readonly",
                "selected_model_provider": "openai"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                }
            }
        });

        let selected = admissible_selected_backend_for_dispatch_target(
            &execution_plan,
            "coach",
            Some("middle"),
            Some("internal_subagents"),
        );

        assert_eq!(selected.as_deref(), Some("hermes_cli"));
    }

    #[test]
    fn admissible_selected_backend_prefers_runtime_assignment_over_route_hint_for_coach_lane() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "coach": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "internal_subagents",
                    "fanout_executor_backends": ["internal_subagents"],
                    "carrier_runtime_assignment": {
                        "enabled": true,
                        "selected_backend_id": "pi_cli",
                        "selected_carrier_id": "pi_cli",
                        "selected_model_profile_id": "pi_gpt55_medium_guarded",
                        "activation_agent_type": "pi_cli",
                        "activation_runtime_role": "coach"
                    }
                }
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true
                    }
                },
                {
                    "backend_id": "pi_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true
                    }
                }
            ]
        });

        let selected = admissible_selected_backend_for_dispatch_target(
            &execution_plan,
            "coach",
            Some("middle"),
            Some("internal_subagents"),
        );

        assert_eq!(selected.as_deref(), Some("pi_cli"));
    }

    #[test]
    fn admissible_selected_backend_prefers_activation_over_inadmissible_inherited_for_implementer()
    {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "senior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "verification": true,
                        "implementation": false
                    }
                },
                {
                    "backend_id": "junior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "implementation": {}
            }
        });

        let selected = admissible_selected_backend_for_dispatch_target(
            &execution_plan,
            "implementer",
            Some("junior"),
            Some("senior"),
        );

        assert_eq!(selected.as_deref(), Some("junior"));
    }

    #[test]
    fn runtime_role_dispatch_target_resolves_configured_runtime_assignment_backend() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "enabled": true,
                "selected_carrier_id": "middle",
                "selected_model_profile_id": "codex_gpt55_medium_write",
                "selected_model_provider": "openai"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "middle",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "specification": true
                    }
                }
            ],
            "development_flow": {
                "dispatch_contract": {
                    "specification_activation": {
                        "activation_agent_type": "middle",
                        "activation_runtime_role": "business_analyst"
                    }
                }
            }
        });

        let selected = admissible_selected_backend_for_dispatch_target(
            &execution_plan,
            "business_analyst",
            None,
            None,
        );

        assert_eq!(selected.as_deref(), Some("middle"));
    }

    #[test]
    fn receipt_sync_fills_runtime_role_activation_from_configured_dispatch_contract() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "spec task".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "enabled": true,
                    "selected_carrier_id": "middle",
                    "selected_model_profile_id": "codex_gpt55_medium_write",
                    "selected_model_provider": "openai"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "middle",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "specification": true
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "spec-run".to_string(),
            dispatch_target: "business_analyst".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            recorded_at: "2026-05-12T00:00:00Z".to_string(),
        };

        sync_receipt_configured_activation_assignment(&role_selection, &mut receipt);

        assert_eq!(receipt.activation_agent_type.as_deref(), Some("middle"));
        assert_eq!(
            receipt.activation_runtime_role.as_deref(),
            Some("business_analyst")
        );
        assert_eq!(receipt.selected_backend.as_deref(), Some("middle"));
    }

    #[test]
    fn mixed_backend_downstream_receipts_preserve_lane_specific_backend_lineage() {
        let role_selection = mixed_backend_role_selection();
        let implementer_receipt = executed_agent_lane_receipt(
            "implementer",
            "opencode_cli",
            "junior",
            "worker",
            Some("coach"),
        );

        let coach_receipt =
            build_downstream_dispatch_receipt(&role_selection, &implementer_receipt)
                .expect("coach downstream receipt should build");
        assert_eq!(coach_receipt.dispatch_target, "coach");
        assert_eq!(
            coach_receipt.selected_backend.as_deref(),
            Some("hermes_cli")
        );
        assert_eq!(
            coach_receipt.activation_agent_type.as_deref(),
            Some("middle")
        );
        assert_eq!(
            coach_receipt.activation_runtime_role.as_deref(),
            Some("coach")
        );
        assert_eq!(coach_receipt.dispatch_status, "routed");

        let mut executed_coach_receipt = coach_receipt.clone();
        executed_coach_receipt.dispatch_status = "executed".to_string();
        executed_coach_receipt.lane_status = "lane_complete".to_string();
        executed_coach_receipt.dispatch_packet_path = Some("/tmp/coach-packet.json".to_string());
        executed_coach_receipt.dispatch_result_path = Some("/tmp/coach-result.json".to_string());
        executed_coach_receipt.downstream_dispatch_target = Some("verification".to_string());
        executed_coach_receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        executed_coach_receipt.downstream_dispatch_note = Some(
            "after `coach` evidence is recorded, activate `verification` for the next bounded lane"
                .to_string(),
        );
        executed_coach_receipt.downstream_dispatch_ready = true;
        executed_coach_receipt.downstream_dispatch_blockers.clear();

        let verification_receipt =
            build_downstream_dispatch_receipt(&role_selection, &executed_coach_receipt)
                .expect("verification downstream receipt should build");
        assert_eq!(verification_receipt.dispatch_target, "verification");
        assert_eq!(
            verification_receipt.selected_backend.as_deref(),
            Some("opencode_cli")
        );
        assert_eq!(
            verification_receipt.activation_agent_type.as_deref(),
            Some("senior")
        );
        assert_eq!(
            verification_receipt.activation_runtime_role.as_deref(),
            Some("verifier")
        );
        assert_eq!(verification_receipt.dispatch_status, "routed");
    }

    #[test]
    fn mixed_backend_implementer_receipt_uses_internal_fallback_when_external_primary_is_inadmissible(
    ) {
        let mut execution_plan = mixed_backend_execution_plan();
        execution_plan["backend_admissibility_matrix"] = json!([
            {
                "backend_id": "opencode_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "implementation": false,
                    "coach": false,
                    "verification": true
                }
            },
            {
                "backend_id": "hermes_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "implementation": false,
                    "coach": true,
                    "verification": true
                }
            },
            {
                "backend_id": "internal_subagents",
                "backend_class": "internal",
                "lane_admissibility": {
                    "implementation": true,
                    "coach": true,
                    "verification": true
                }
            }
        ]);
        let mut role_selection = mixed_backend_role_selection();
        role_selection.execution_plan = execution_plan;

        let dev_pack_receipt = RunGraphDispatchReceipt {
            run_id: "run-mixed-backend-matrix".to_string(),
            dispatch_target: "dev-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_complete".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: Some("vida task ensure".to_string()),
            dispatch_packet_path: Some("/tmp/dev-pack-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `dev-pack` evidence is recorded, activate `implementer` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        let implementer_receipt =
            build_downstream_dispatch_receipt(&role_selection, &dev_pack_receipt)
                .expect("implementer downstream receipt should build");

        assert_eq!(implementer_receipt.dispatch_target, "implementer");
        assert_eq!(
            implementer_receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            implementer_receipt.activation_agent_type.as_deref(),
            Some("junior")
        );
        assert_eq!(
            implementer_receipt.activation_runtime_role.as_deref(),
            Some("worker")
        );
    }

    #[test]
    fn mixed_backend_coach_receipt_prefers_explicit_review_route_over_inherited_internal_fallback()
    {
        let mut execution_plan = mixed_backend_execution_plan();
        execution_plan["backend_admissibility_matrix"] = json!([
            {
                "backend_id": "opencode_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "implementation": false,
                    "coach": true,
                    "verification": true
                }
            },
            {
                "backend_id": "hermes_cli",
                "backend_class": "external_cli",
                "lane_admissibility": {
                    "implementation": false,
                    "coach": true,
                    "verification": true
                }
            },
            {
                "backend_id": "internal_subagents",
                "backend_class": "internal",
                "lane_admissibility": {
                    "implementation": true,
                    "coach": true,
                    "verification": true
                }
            }
        ]);
        let mut role_selection = mixed_backend_role_selection();
        role_selection.execution_plan = execution_plan;

        let implementer_receipt = executed_agent_lane_receipt(
            "implementer",
            "internal_subagents",
            "junior",
            "worker",
            Some("coach"),
        );

        let coach_receipt =
            build_downstream_dispatch_receipt(&role_selection, &implementer_receipt)
                .expect("coach downstream receipt should build");

        assert_eq!(coach_receipt.dispatch_target, "coach");
        assert_eq!(
            coach_receipt.selected_backend.as_deref(),
            Some("hermes_cli")
        );
        assert_eq!(
            coach_receipt.activation_agent_type.as_deref(),
            Some("middle")
        );
        assert_eq!(
            coach_receipt.activation_runtime_role.as_deref(),
            Some("coach")
        );
        assert_eq!(coach_receipt.dispatch_status, "routed");
    }

    #[test]
    fn apply_first_handoff_execution_keeps_selected_backend_across_mixed_lane_chain() {
        let status = crate::taskflow_run_graph::default_run_graph_status(
            "run-mixed-backend-matrix",
            "implementation",
            "implementation",
        );
        let implementer_receipt = executed_agent_lane_receipt(
            "implementer",
            "opencode_cli",
            "junior",
            "worker",
            Some("coach"),
        );
        let implemented_status =
            apply_first_handoff_execution_to_run_graph_status(&status, &implementer_receipt);
        assert_eq!(implemented_status.active_node, "implementer");
        assert_eq!(implemented_status.next_node.as_deref(), Some("coach"));
        assert_eq!(implemented_status.selected_backend, "opencode_cli");
        assert_eq!(implemented_status.resume_target, "dispatch.coach");

        let coach_receipt = executed_agent_lane_receipt(
            "coach",
            "hermes_cli",
            "middle",
            "coach",
            Some("verification"),
        );
        let coached_status =
            apply_first_handoff_execution_to_run_graph_status(&implemented_status, &coach_receipt);
        assert_eq!(coached_status.active_node, "coach");
        assert_eq!(coached_status.next_node.as_deref(), Some("verification"));
        assert_eq!(coached_status.selected_backend, "hermes_cli");
        assert_eq!(coached_status.resume_target, "dispatch.verification");
    }

    #[test]
    fn review_ensemble_route_summary_preserves_fanout_and_internal_fallback_matrix() {
        let role_selection = mixed_backend_role_selection();

        let summary = dispatch_execution_route_summary(
            &role_selection,
            "review_ensemble",
            Some("opencode_cli"),
            None,
        );

        assert_eq!(summary["effective_selected_backend"], "opencode_cli");
        assert_eq!(
            summary["selected_backend_source"],
            "dynamic_runtime_selection"
        );
        assert_eq!(summary["route_primary_backend"], "opencode_cli");
        assert_eq!(summary["route_fallback_backend"], "internal_subagents");
        assert_eq!(
            summary["route_fanout_backends"],
            serde_json::json!(["opencode_cli", "hermes_cli", "kilo_cli"])
        );
        assert_eq!(summary["effective_execution_posture"], "hybrid");
    }

    #[test]
    fn write_runtime_dispatch_packet_keeps_agent_init_command_for_mixed_implementer_route_before_execution(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue implementation in crates/vida/src/runtime_dispatch_state.rs"
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "write_scope": "none",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "write_scope": "repo",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "runtime_assignment": {
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-mixed-command".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );

        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let packet = crate::read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");

        assert_eq!(packet["dispatch_surface"], "vida agent-init");
        assert!(packet["dispatch_command"]
            .as_str()
            .expect("dispatch command should be present")
            .starts_with("vida agent-init --dispatch-packet "));
        assert_eq!(packet["selected_backend"], "internal_subagents");
        assert_eq!(
            packet["route_policy"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert_eq!(
            packet["route_policy"]["selected_backend_source"],
            "route_fallback_hint"
        );
        assert_eq!(
            packet["route_policy"]["route_primary_backend"],
            "opencode_cli"
        );
    }

    #[test]
    fn runtime_dispatch_packet_preview_exposes_template_and_scope_without_writing_packet() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue implementation in crates/vida/src/runtime_dispatch_state.rs"
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "write_scope": "none",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "write_scope": "repo",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "runtime_assignment": {
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-preview-command".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-20T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );

        let preview = runtime_dispatch_packet_preview(&ctx).expect("preview should render");

        assert_eq!(preview["status"], "pass");
        assert_eq!(preview["packet_template_kind"], "delivery_task_packet");
        assert_eq!(
            preview["owned_paths"],
            serde_json::json!(["crates/vida/src/runtime_dispatch_state.rs"])
        );
        assert!(
            state_root
                .join("runtime-consumption")
                .join("dispatch-packets")
                .read_dir()
                .map(|mut rows| rows.next().is_none())
                .unwrap_or(true),
            "preview helper must not write a dispatch packet to disk"
        );
    }

    #[test]
    fn runtime_dispatch_packet_carries_dispatch_target_runtime_assignment() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "write docs/product/spec/github-114-design.md".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "enabled": false,
                    "reason": "no_carrier_declares_runtime_role_and_task_class",
                    "runtime_role": "business_analyst",
                    "task_class": "verification"
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "specification": {
                                "activation": {
                                    "selected_tier": "middle",
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "business_analyst"
                                },
                                "closure_class": "law",
                                "packet_template_kind": "delivery_task_packet",
                                "task_class": "specification",
                                "runtime_role": "business_analyst"
                            }
                        },
                        "specification_activation": {
                            "enabled": true,
                            "selected_carrier_id": "middle",
                            "selected_backend_id": "middle",
                            "selected_model_profile_id": "codex_gpt55_medium_write",
                            "selected_tier": "middle",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "specification",
                            "selection_rule": "role_task_then_readiness_then_score_then_cost_quality"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "github-114-runtime-assignment".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-06T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );

        let preview = runtime_dispatch_packet_preview(&ctx).expect("preview should render");
        let packet = &preview["packet"];

        assert_eq!(
            packet["runtime_assignment_source"],
            "dispatch_contract_specification_activation"
        );
        assert_eq!(
            packet["runtime_assignment"]["selected_carrier_id"],
            "middle"
        );
        assert_eq!(
            packet["runtime_assignment"]["selected_model_profile_id"],
            "codex_gpt55_medium_write"
        );
        assert_eq!(packet["runtime_assignment"]["task_class"], "specification");
        assert_eq!(
            packet["carrier_runtime_assignment"],
            packet["runtime_assignment"]
        );
    }

    #[test]
    fn write_runtime_dispatch_packet_persists_blocked_implementer_packet_without_owned_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "probe closure".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["closure".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "internal_subagents",
                        "activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked-no-scope".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("missing_owned_write_scope".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-20T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );

        let preview = runtime_dispatch_packet_preview(&ctx).expect("preview should render");
        assert_eq!(preview["status"], "blocked");
        assert_eq!(
            preview["packet_contract_missing_fields"],
            serde_json::json!(["owned_paths"])
        );

        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("blocked dispatch packet should persist");
        let packet = crate::read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");

        assert_eq!(packet["packet_template_kind"], "delivery_task_packet");
        assert!(packet["dispatch_command"]
            .as_str()
            .expect("dispatch command should be present")
            .starts_with("vida agent-init --dispatch-packet "));
        assert_eq!(
            packet["delivery_task_packet"]["handoff_task_class"],
            "implementation"
        );
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([])
        );
    }
}

pub(crate) fn write_runtime_dispatch_packet(
    ctx: &RuntimeDispatchPacketContext<'_>,
) -> Result<String, String> {
    let packet_dir = ctx
        .state_root
        .join("runtime-consumption")
        .join("dispatch-packets");
    std::fs::create_dir_all(&packet_dir)
        .map_err(|error| format!("Failed to create dispatch-packets directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let safe_run_id = validate_dispatch_packet_run_id_component(&ctx.receipt.run_id)?;
    let packet_path = packet_dir.join(format!("{safe_run_id}-{ts}.json"));
    let packet_path_display = packet_path.display().to_string();
    let activation_command = runtime_dispatch_command_for_packet_path(
        ctx.role_selection,
        ctx.receipt,
        &packet_path_display,
        ctx.selected_backend_override.as_deref(),
    );
    let body = build_runtime_dispatch_packet_body(ctx, activation_command)?;
    let validation_error =
        validate_runtime_dispatch_packet_contract(&body, "Runtime dispatch packet").err();
    if ctx.receipt.dispatch_status != "blocked" {
        if let Some(error) = validation_error {
            return Err(error);
        }
    }
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode dispatch packet: {error}"))?;
    std::fs::write(&packet_path, encoded)
        .map_err(|error| format!("Failed to write dispatch packet: {error}"))?;
    Ok(packet_path.display().to_string())
}

fn validate_dispatch_packet_run_id_component(run_id: &str) -> Result<&str, String> {
    let value = run_id.trim();
    if value.is_empty() {
        return Err("Failed to write dispatch packet: receipt.run_id is empty".to_string());
    }
    if value.contains('/') || value.contains('\\') {
        return Err(format!(
            "Failed to write dispatch packet: receipt.run_id `{value}` contains path separators"
        ));
    }
    if value == "." || value == ".." {
        return Err(format!(
            "Failed to write dispatch packet: receipt.run_id `{value}` is not a valid filename segment"
        ));
    }
    Ok(value)
}

pub(crate) async fn execute_runtime_dispatch_handoff(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve current directory for dispatch execution: {error}")
        })?);
    match receipt.dispatch_target.as_str() {
        "spec-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for spec-pack dispatch: {error}"
                    )
                })?;
            execute_taskflow_bootstrap_spec_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
            )
        }
        "work-pool-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for work-pool dispatch: {error}"
                    )
                })?;
            execute_work_packet_create_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
                "work_pool_task",
            )
        }
        "dev-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for dev-pack dispatch: {error}"
                    )
                })?;
            execute_work_packet_create_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
                "dev_task",
            )
        }
        "closure" => {
            let bundle_check = crate::TaskflowConsumeBundleCheck {
                ok: true,
                blockers: Vec::new(),
                root_artifact_id: "runtime_dispatch_packet_closure_preview".to_string(),
                artifact_count: 0,
                boot_classification: "execution_packet_already_admitted".to_string(),
                migration_state: "execution_packet_already_admitted".to_string(),
                activation_status: "ready_enough_for_normal_work".to_string(),
            };
            let admitted_closure_packet = receipt.dispatch_kind == "closure"
                && receipt.selected_backend.as_deref() == Some("taskflow_state_store")
                || {
                    let store = StateStore::open_existing(state_root.to_path_buf())
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to reopen authoritative state store for closure admission: {error}"
                        )
                    })?;
                    let admitted = store
                        .run_graph_status(&receipt.run_id)
                        .await
                        .ok()
                        .is_some_and(|status| {
                            status.active_node == "closure"
                                && matches!(status.status.as_str(), "ready" | "running")
                                && status.lifecycle_stage == "closure_active"
                                && status.handoff_state == "none"
                                && status.resume_target == "none"
                        });
                    drop(store);
                    admitted
                };
            let docflow_verdict = if admitted_closure_packet {
                crate::RuntimeConsumptionDocflowVerdict {
                    status: "pass".to_string(),
                    ready: true,
                    blockers: Vec::new(),
                    proof_surfaces: vec![
                        "vida docflow readiness-check --profile active-canon".to_string(),
                        "vida docflow proofcheck --profile active-canon".to_string(),
                    ],
                }
            } else {
                let (registry, check, readiness, proof, _overview) =
                    crate::build_docflow_runtime_evidence();
                crate::build_docflow_runtime_verdict(&registry, &check, &readiness, &proof)
            };
            let closure_admission =
                build_runtime_closure_admission(&bundle_check, &docflow_verdict, role_selection);
            let closure_ready = closure_admission.admitted;
            let execution_state = if closure_ready { "executed" } else { "blocked" };
            let status = if closure_ready { "pass" } else { "blocked" };
            let note = if closure_ready {
                "runtime downstream scheduler reached closure without additional lane activation"
            } else {
                "runtime downstream scheduler blocked closure until consume-bundle-check and docflow admission blockers are cleared"
            };
            let blocker_code = closure_admission.blockers.first().cloned();
            let blockers = closure_admission.blockers.clone();
            Ok(serde_json::json!({
                "surface": "vida taskflow closure-preview",
                "execution_state": execution_state,
                "status": status,
                "closure_ready": closure_ready,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "note": note,
                "blocker_code": blocker_code,
                "blockers": blockers,
                "closure_admission": closure_admission,
                "bundle_check": bundle_check,
                "docflow_verdict": docflow_verdict,
            }))
        }
        _ => {
            let dispatch_packet_path =
                receipt.dispatch_packet_path.as_deref().ok_or_else(|| {
                    missing_agent_lane_dispatch_packet_error(&receipt.dispatch_target)
                })?;
            let canonical_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
            if canonical_backend.is_none() {
                return Err(format!(
                    "Dispatch target `{}` is routed to an agent lane but no lawful backend could be resolved from the execution route",
                    receipt.dispatch_target
                ));
            }
            let host_runtime = runtime_host_execution_contract_for_root(&project_root);
            let lane_dispatch = runtime_agent_lane_dispatch_for_root(
                &project_root,
                dispatch_packet_path,
                canonical_backend.as_deref(),
                preferred_selected_model_profile_for_dispatch_target(
                    role_selection,
                    &receipt.dispatch_target,
                    canonical_backend.as_deref(),
                )
                .as_deref(),
            );
            if lane_dispatch.surface != "vida agent-init" {
                return execute_external_agent_lane_dispatch(
                    state_root,
                    &project_root,
                    dispatch_packet_path,
                    canonical_backend.as_deref(),
                    role_selection,
                    receipt,
                    host_runtime,
                )
                .await;
            }
            let lane_backend_class = lane_dispatch
                .backend_dispatch
                .get("backend_class")
                .and_then(serde_json::Value::as_str);
            let lane_execution_class = lane_dispatch
                .backend_dispatch
                .get("selected_execution_class")
                .and_then(serde_json::Value::as_str)
                .or_else(|| host_runtime["selected_cli_execution_class"].as_str());
            if matches!(lane_backend_class, Some("internal" | "internal_cli"))
                && lane_execution_class != Some("internal")
            {
                let store = StateStore::open_existing(state_root.to_path_buf())
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to reopen authoritative state store for activation view: {error}"
                        )
                    })?;
                let activation_view =
                    crate::init_surfaces::render_agent_init_packet_activation_with_store(
                        &store,
                        &project_root,
                        dispatch_packet_path,
                        false,
                    )
                    .await?;
                drop(store);
                let mut result = agent_lane_dispatch_result(
                    activation_view,
                    dispatch_packet_path,
                    canonical_backend.as_deref(),
                    role_selection,
                    receipt,
                    host_runtime,
                );
                if let Some(body) = result.as_object_mut() {
                    body.insert("surface".to_string(), serde_json::json!("vida agent-init"));
                    body.insert("status".to_string(), serde_json::json!("blocked"));
                    body.insert("execution_state".to_string(), serde_json::json!("blocked"));
                    body.insert(
                        "blocker_code".to_string(),
                        serde_json::json!("internal_activation_view_only"),
                    );
                    body.insert(
                        "backend_dispatch".to_string(),
                        lane_dispatch.backend_dispatch,
                    );
                }
                return Ok(result);
            }
            if let Some(result) = execute_internal_agent_lane_dispatch(
                state_root,
                &project_root,
                dispatch_packet_path,
                canonical_backend.as_deref(),
                role_selection,
                receipt,
                host_runtime.clone(),
            )
            .await?
            {
                return Ok(result);
            }
            let activation_view = {
                let store = StateStore::open_existing(state_root.to_path_buf())
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to reopen authoritative state store for activation view: {error}"
                        )
                    })?;
                let activation_view =
                    crate::init_surfaces::render_agent_init_packet_activation_with_store(
                        &store,
                        &project_root,
                        dispatch_packet_path,
                        false,
                    )
                    .await?;
                drop(store);
                activation_view
            };
            Ok(agent_lane_dispatch_result(
                activation_view,
                dispatch_packet_path,
                canonical_backend.as_deref(),
                role_selection,
                receipt,
                host_runtime,
            ))
        }
    }
}

pub(crate) fn write_runtime_dispatch_result(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let result_path = receipt
        .dispatch_result_path
        .as_deref()
        .filter(|path| {
            receipt.dispatch_status == "executing"
                && body["execution_state"].as_str() == Some("executing")
                && !path.trim().is_empty()
        })
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let ts = time::OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("rfc3339 timestamp should render")
                .replace(':', "-");
            result_dir.join(format!("{}-{ts}.json", receipt.run_id))
        });
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let mut artifact_body = body.clone();
    let result_path_display = result_path.display().to_string();
    if let Some(object) = artifact_body.as_object_mut() {
        object.insert(
            "dispatch_result_path".to_string(),
            serde_json::json!(result_path_display),
        );
        object.insert("run_id".to_string(), serde_json::json!(receipt.run_id));
        object.insert(
            "recorded_at".to_string(),
            serde_json::json!(recorded_at.clone()),
        );
        if let Some(packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            object.insert(
                "source_dispatch_packet_path".to_string(),
                serde_json::json!(packet_path),
            );
        }
        if is_terminal_dispatch_execution_state(body) {
            let lane_receipt = canonical_lane_execution_receipt_artifact_json(
                receipt,
                body,
                &recorded_at,
                &result_path_display,
            );
            let receipt_status = json_string(body.get("status"))
                .filter(|value| value == "pass" || value == "blocked")
                .unwrap_or_else(|| {
                    if json_string(body.get("execution_state")).as_deref() == Some("blocked")
                        || receipt.dispatch_status == "blocked"
                    {
                        "blocked".to_string()
                    } else {
                        "pass".to_string()
                    }
                });
            object.insert(
                "receipt_status".to_string(),
                serde_json::json!(receipt_status),
            );
            object.insert(
                "receipt_path".to_string(),
                serde_json::json!(result_path_display),
            );
            object.insert(
                "lane_execution_receipt_path".to_string(),
                serde_json::json!(result_path_display),
            );
            object.insert("lane_execution_receipt_artifact".to_string(), lane_receipt);
        }
        let executed_agent_lane = receipt.dispatch_kind == "agent_lane"
            && json_string(body.get("execution_state")).as_deref() == Some("executed")
            && receipt
                .dispatch_packet_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty());
        if executed_agent_lane {
            let completion_receipt_id = format!(
                "dispatch-completion-{}",
                time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            );
            object.insert(
                "artifact_kind".to_string(),
                serde_json::json!("runtime_lane_completion_result"),
            );
            object.insert(
                "completed_target".to_string(),
                serde_json::json!(receipt.dispatch_target),
            );
            object.insert(
                "completion_receipt_id".to_string(),
                serde_json::json!(completion_receipt_id),
            );
        } else {
            object.insert(
                "artifact_kind".to_string(),
                serde_json::json!("runtime_dispatch_result"),
            );
            object.insert(
                "dispatch_target".to_string(),
                serde_json::json!(receipt.dispatch_target),
            );
        }
        let activation_evidence =
            normalized_dispatch_result_activation_evidence(receipt, body, &result_path_display);
        object.insert(
            "activation_vs_execution_evidence".to_string(),
            activation_evidence.clone(),
        );
        object.insert(
            "activation_semantics".to_string(),
            activation_evidence["activation_semantics"].clone(),
        );
        object.insert(
            "execution_evidence".to_string(),
            activation_evidence["execution_evidence"].clone(),
        );
    }
    let encoded = serde_json::to_string_pretty(&artifact_body)
        .map_err(|error| format!("Failed to encode dispatch result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write dispatch result: {error}"))?;
    Ok(result_path.display().to_string())
}

fn normalized_dispatch_result_activation_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
    result_artifact_path: &str,
) -> serde_json::Value {
    let activation_kind = body["activation_semantics"]["activation_kind"]
        .as_str()
        .or_else(|| {
            if body["execution_evidence"]["status"].as_str() == Some("recorded")
                || body["execution_state"].as_str() == Some("executed")
            {
                Some("execution_evidence")
            } else if body["artifact_kind"].as_str() == Some("runtime_dispatch_result")
                || body["execution_state"].as_str() == Some("blocked")
                || body["execution_state"].as_str() == Some("executing")
            {
                Some("activation_view")
            } else {
                None
            }
        })
        .unwrap_or("activation_view");
    let activation_semantics = serde_json::json!({
        "activation_kind": activation_kind,
        "view_only": activation_kind != "execution_evidence",
        "executes_packet": activation_kind == "execution_evidence",
        "records_completion_receipt": activation_kind == "execution_evidence",
    });
    let execution_evidence = if activation_kind == "execution_evidence" {
        let mut evidence = match body.get("execution_evidence").cloned() {
            Some(serde_json::Value::Object(object)) => object,
            _ => serde_json::Map::new(),
        };
        evidence
            .entry("status".to_string())
            .or_insert_with(|| serde_json::json!("recorded"));
        evidence
            .entry("receipt_backed".to_string())
            .or_insert_with(|| serde_json::json!(true));
        evidence
            .entry("evidence_kind".to_string())
            .or_insert_with(|| serde_json::json!("lane_execution_receipt_artifact"));
        evidence
            .entry("result_path".to_string())
            .or_insert_with(|| serde_json::json!(result_artifact_path));
        evidence.entry("backend_id".to_string()).or_insert_with(|| {
            serde_json::json!(canonical_lane_receipt_carrier_id_for_result(receipt, body))
        });
        serde_json::Value::Object(evidence)
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "activation_semantics": activation_semantics,
        "execution_evidence": execution_evidence,
        "receipt_backed": activation_kind == "execution_evidence",
    })
}

fn canonical_lane_receipt_carrier_id(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> String {
    receipt
        .selected_backend
        .clone()
        .or_else(|| receipt.activation_agent_type.clone())
        .or_else(|| receipt.dispatch_surface.clone())
        .unwrap_or_else(|| "taskflow_state_store".to_string())
}

fn canonical_lane_receipt_carrier_id_for_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> String {
    for candidate in [
        body.get("execution_evidence")
            .and_then(|value| value.get("backend_id")),
        body.get("backend_dispatch")
            .and_then(|value| value.get("carrier_id")),
        body.get("backend_dispatch")
            .and_then(|value| value.get("backend_id")),
        body.get("execution_truth")
            .and_then(|value| value.get("effective_selected_backend")),
        body.get("effective_execution_posture")
            .and_then(|value| value.get("selected_backend")),
    ] {
        if let Some(value) = json_string(candidate)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && value != "unknown")
        {
            return value;
        }
    }
    canonical_lane_receipt_carrier_id(receipt)
}

fn is_terminal_dispatch_execution_state(body: &serde_json::Value) -> bool {
    matches!(
        json_string(body.get("execution_state")).as_deref(),
        Some("executed" | "blocked")
    )
}

fn canonical_lane_execution_receipt_artifact_json(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
    finished_at: &str,
    result_artifact_path: &str,
) -> serde_json::Value {
    let packet_id = receipt
        .dispatch_packet_path
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_stem())
        .and_then(|stem| stem.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}-no-packet", receipt.run_id, receipt.dispatch_target));
    let lane_role = receipt
        .activation_runtime_role
        .clone()
        .unwrap_or_else(|| receipt.dispatch_target.clone());
    let carrier_id = canonical_lane_receipt_carrier_id_for_result(receipt, body);
    let status = match json_string(body.get("status")).as_deref() {
        Some("pass") => "pass".to_string(),
        Some("blocked") => "blocked".to_string(),
        _ if receipt.dispatch_status == "blocked" => "blocked".to_string(),
        _ => "pass".to_string(),
    };
    let lane_status = match json_string(body.get("execution_state")).as_deref() {
        Some("executed") => LaneStatus::LaneCompleted.as_str().to_string(),
        Some("blocked") => LaneStatus::LaneBlocked.as_str().to_string(),
        Some("executing") => LaneStatus::LaneRunning.as_str().to_string(),
        _ => receipt.lane_status.clone(),
    };
    serde_json::to_value(
        crate::release1_contracts::CanonicalLaneExecutionReceiptArtifact {
            lane_execution_receipt: crate::release1_contracts::CanonicalLaneExecutionReceipt {
                header: crate::release1_contracts::CanonicalArtifactHeader::new(
                    format!(
                        "lane-execution.{}.{}",
                        receipt.run_id, receipt.dispatch_target
                    ),
                    crate::release1_contracts::CanonicalArtifactType::LaneExecutionReceipt,
                    receipt.recorded_at.clone(),
                    finished_at.to_string(),
                    status,
                    "runtime_dispatch_state",
                    None,
                    Some(
                        crate::release1_contracts::WorkflowClass::DelegatedDevelopmentPacket
                            .as_str()
                            .to_string(),
                    ),
                ),
                run_id: receipt.run_id.clone(),
                packet_id,
                lane_id: format!("{}:{}", receipt.run_id, receipt.dispatch_target),
                lane_role,
                carrier_id,
                lane_status,
                evidence_status: "recorded".to_string(),
                started_at: receipt.recorded_at.clone(),
                finished_at: finished_at.to_string(),
                result_artifact_ids: vec![result_artifact_path.to_string()],
                supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
                exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
            },
        },
    )
    .expect("lane execution receipt artifact should serialize")
}

pub(crate) fn runtime_dispatch_execution_started_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    stale_after_seconds: u64,
) -> serde_json::Value {
    serde_json::json!({
        "surface": receipt.dispatch_surface,
        "activation_command": receipt.dispatch_command,
        "status": "pass",
        "execution_state": "executing",
        "dispatch_target": receipt.dispatch_target,
        "selected_backend": receipt.selected_backend,
        "stale_after_seconds": stale_after_seconds,
        "note": "runtime dispatch handoff started; terminal completion is still pending",
    })
}

pub(crate) async fn record_dispatch_execution_started(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let project_root = runtime_dispatch_project_root_from_state_root(state_root);
    let stale_after_seconds = dispatch_execution_started_stale_after_seconds(
        project_root.as_ref(),
        role_selection,
        receipt,
    );
    let in_flight_dispatch_result_path = write_runtime_dispatch_result(
        state_root,
        receipt,
        &runtime_dispatch_execution_started_result(receipt, stale_after_seconds),
    )?;
    receipt.dispatch_result_path = Some(in_flight_dispatch_result_path);
    receipt.dispatch_status = "executing".to_string();
    receipt.lane_status = LaneStatus::LaneRunning.as_str().to_string();
    receipt.blocker_code = None;
    let store = reopen_authoritative_state_store_for_dispatch_phase(
        state_root,
        receipt,
        "before dispatch execution",
    )
    .await?;
    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        if let Ok(status) = store.run_graph_status(&run_id).await {
            let executing_status =
                apply_dispatch_execution_started_to_run_graph_status(&status, receipt);
            store
                .record_run_graph_status(&executing_status)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to record in-flight run-graph status before dispatch execution: {error}"
                    )
                })?;
            crate::taskflow_continuation::sync_run_graph_continuation_binding(
                &store,
                &executing_status,
                "dispatch_execution_started",
            )
            .await
            .map_err(|error| {
                format!(
                    "Failed to synchronize continuation binding before dispatch execution: {error}"
                )
            })?;
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| {
            format!("Failed to persist in-flight dispatch receipt before execution: {error}")
        })?;
    Ok(())
}

fn runtime_dispatch_execution_timeout_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
) -> serde_json::Value {
    serde_json::json!({
        "surface": receipt.dispatch_surface,
        "activation_command": receipt.dispatch_command,
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_target": receipt.dispatch_target,
        "selected_backend": receipt.selected_backend,
        "blocker_code": blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority),
        "provider_error": format!(
            "runtime dispatch handoff timed out after {timeout_seconds}s before terminal completion evidence was recorded"
        ),
        "note": "runtime dispatch handoff timed out before terminal completion evidence was recorded",
    })
}

fn runtime_dispatch_internal_activation_timeout_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
    blocker_code: &str,
) -> serde_json::Value {
    serde_json::json!({
        "surface": receipt.dispatch_surface,
        "activation_command": receipt.dispatch_command,
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_target": receipt.dispatch_target,
        "selected_backend": receipt.selected_backend,
        "blocker_code": blocker_code,
        "provider_error": format!(
            "internal host carrier timed out after {timeout_seconds}s before receipt-backed completion evidence was recorded"
        ),
        "timeout_wrapper": {
            "timeout_seconds": timeout_seconds,
            "kill_after_grace_seconds": 1,
            "no_output_timeout_seconds": serde_json::Value::Null,
            "timed_out": true,
            "timeout_exit_code": serde_json::Value::Null,
        },
        "blocker_reason": "internal host carrier timed out before receipt-backed completion evidence was recorded",
        "note": "internal host carrier timed out before receipt-backed completion evidence was recorded",
    })
}

fn runtime_dispatch_internal_activation_view_only_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    blocker_code: &str,
) -> serde_json::Value {
    let (provider_error, blocker_reason, note) = if blocker_code
        == INTERNAL_CODEX_CARRIER_UNAVAILABLE
    {
        (
            "internal host carrier is unavailable from this CLI execution surface",
            "configured internal host backend would launch a non-receipted command bridge",
            "internal host handoff blocked before launch to avoid a non-receipted CLI bridge",
        )
    } else if blocker_code == INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT {
        (
            "internal host dispatch was blocked before launching nested carrier execution because the configured host bridge cannot provide receipt-backed completion evidence",
            "configured internal host bridge does not support receipt-backed completion; timeout avoided by recording a terminal blocker before launch",
            "internal host handoff blocked before launch to avoid waiting for a configured non-receipted carrier path",
        )
    } else {
        (
            "dispatch packet declares activation-view-only handoff without receipt-backed execution evidence",
            "internal host dispatch is not launched for activation-view-only packets without execution authority",
            "internal host activation-view-only handoff blocked before launching nested carrier execution",
        )
    };
    serde_json::json!({
        "surface": receipt.dispatch_surface,
        "activation_command": receipt.dispatch_command,
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_target": receipt.dispatch_target,
        "selected_backend": receipt.selected_backend,
        "blocker_code": blocker_code,
        "provider_error": provider_error,
        "blocker_reason": blocker_reason,
        "note": note,
    })
}

pub(crate) fn internal_host_dispatch_requires_prelaunch_blocker(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if internal_host_app_bridge_requires_prelaunch_blocker(project_root, role_selection, receipt) {
        return true;
    }
    if !dispatch_handoff_uses_internal_host(project_root, role_selection, receipt) {
        return false;
    }
    if dispatch_packet_declares_activation_view_only(receipt.dispatch_packet_path.as_deref()) {
        return configured_internal_host_receipt_backed_completion_supported(project_root)
            != Some(true);
    }
    internal_host_activation_view_only_requires_terminal_blocker(
        project_root,
        role_selection,
        receipt,
    )
}

fn dispatch_packet_declares_activation_view_only(dispatch_packet_path: Option<&str>) -> bool {
    let Some(dispatch_packet_path) = dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(packet) = crate::read_json_file_if_present(std::path::Path::new(dispatch_packet_path))
    else {
        return false;
    };
    let activation_semantics = packet
        .get("activation_semantics")
        .or_else(|| packet.pointer("/activation_evidence/activation_semantics"))
        .or_else(|| packet.pointer("/activation_vs_execution_evidence/activation_semantics"));
    let declares_view_only = activation_semantics.is_some_and(|semantics| {
        semantics["view_only"].as_bool().unwrap_or(false)
            && !semantics["executes_packet"].as_bool().unwrap_or(true)
            && !semantics["records_completion_receipt"]
                .as_bool()
                .unwrap_or(true)
    });
    let evidence_declares_view_only = packet["activation_evidence"]["evidence_state"].as_str()
        == Some("activation_view_only")
        || packet["activation_vs_execution_evidence"]["evidence_state"].as_str()
            == Some("activation_view_only");
    if declares_view_only || evidence_declares_view_only {
        return true;
    }
    if dispatch_packet_has_executable_template(&packet) {
        return false;
    }
    false
}

fn dispatch_packet_has_executable_template(packet: &serde_json::Value) -> bool {
    matches!(
        packet["packet_template_kind"].as_str(),
        Some(
            "delivery_task_packet"
                | "execution_block_packet"
                | "coach_review_packet"
                | "verifier_proof_packet"
        )
    )
}

pub(crate) fn dispatch_result_stale_after_seconds(result: &serde_json::Value) -> i64 {
    result["stale_after_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .unwrap_or(LEGACY_STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS)
}

pub(crate) fn stale_in_flight_dispatch_timeout_seconds_for_receipt(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> i64 {
    if let Some(seconds) = result["stale_after_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
    {
        return seconds;
    }
    if crate::runtime_dispatch_receipt_helpers::stale_in_flight_dispatch_preserves_internal_activation_view(receipt, result) {
        let project_root = runtime_dispatch_project_root_from_state_root(state_root);
        return configured_internal_host_handoff_timeout_seconds(project_root.as_ref())
            .unwrap_or(DEFAULT_INTERNAL_HOST_HANDOFF_TIMEOUT_SECONDS)
            .saturating_add(INTERNAL_DISPATCH_HANDOFF_TIMEOUT_GRACE_SECONDS) as i64;
    }
    LEGACY_STALE_IN_FLIGHT_DISPATCH_TIMEOUT_SECONDS
}

pub(crate) fn apply_dispatch_execution_timeout_to_receipt(
    state_root: &Path,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
) -> Result<(), String> {
    let execution_result = runtime_dispatch_execution_timeout_result(receipt, timeout_seconds);
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    receipt.dispatch_status = "blocked".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = blocker_code_value(BlockerCode::TimeoutWithoutTakeoverAuthority);
    if let Some(dispatch_surface) = json_string(execution_result.get("surface")) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    Ok(())
}

pub(crate) fn apply_internal_activation_timeout_to_receipt(
    state_root: &Path,
    _project_root: &Path,
    _role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    timeout_seconds: u64,
) -> Result<(), String> {
    let blocker_code = INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT;
    let execution_result =
        runtime_dispatch_internal_activation_timeout_result(receipt, timeout_seconds, blocker_code);
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    receipt.dispatch_status = "blocked".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = Some(blocker_code.to_string());
    if let Some(dispatch_surface) = json_string(execution_result.get("surface")) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    Ok(())
}

fn annotate_internal_host_timeout_surface(
    project_root: &Path,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) {
    if let Ok(overlay) = load_project_overlay_yaml_for_root(project_root) {
        let (selected_cli_system, _) = selected_host_cli_system_for_runtime_dispatch(&overlay);
        receipt.dispatch_surface = Some(format!("internal_cli:{selected_cli_system}"));
    }
}

pub(crate) fn apply_internal_activation_view_only_to_receipt(
    state_root: &Path,
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let blocker_code =
        internal_host_activation_view_only_blocker_code(project_root, role_selection, receipt);
    let execution_result =
        runtime_dispatch_internal_activation_view_only_result(receipt, blocker_code);
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    receipt.dispatch_status = "blocked".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = Some(blocker_code.to_string());
    if let Some(dispatch_surface) = json_string(execution_result.get("surface")) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    Ok(())
}

pub(crate) fn normalize_stale_in_flight_dispatch_receipt(
    state_root: &Path,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    let timeout_blocked_receipt = receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("timeout_without_takeover_authority");
    if receipt.dispatch_status != "executing" && !timeout_blocked_receipt {
        return Ok(false);
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return Ok(false);
    };
    let preserves_internal_activation_view =
        crate::runtime_dispatch_receipt_helpers::stale_in_flight_dispatch_preserves_internal_activation_view(receipt, &result);
    if timeout_blocked_receipt
        && preserves_internal_activation_view
        && result["blocker_code"].as_str() == Some("timeout_without_takeover_authority")
    {
        let timeout_seconds =
            stale_in_flight_dispatch_timeout_seconds_for_receipt(state_root, receipt, &result)
                as u64;
        let project_root = runtime_dispatch_project_root_from_state_root(state_root);
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "timeout-normalization".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: String::new(),
            selected_role: receipt
                .activation_runtime_role
                .clone()
                .unwrap_or_else(|| receipt.dispatch_target.clone()),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "timeout-normalization".to_string(),
        };
        apply_internal_activation_timeout_to_receipt(
            state_root,
            project_root.as_ref(),
            &role_selection,
            receipt,
            timeout_seconds,
        )?;
        return Ok(true);
    }
    if result["execution_state"].as_str() != Some("executing") {
        return Ok(false);
    }
    let Some(recorded_at) = result["recorded_at"].as_str() else {
        return Ok(false);
    };
    let Ok(recorded_at) =
        time::OffsetDateTime::parse(recorded_at, &time::format_description::well_known::Rfc3339)
    else {
        return Ok(false);
    };
    let stale_after_seconds = if preserves_internal_activation_view {
        stale_in_flight_dispatch_timeout_seconds_for_receipt(state_root, receipt, &result)
    } else {
        dispatch_result_stale_after_seconds(&result)
    };
    let age_seconds = (time::OffsetDateTime::now_utc() - recorded_at).whole_seconds();
    if age_seconds <= stale_after_seconds {
        return Ok(false);
    }
    let timeout_seconds = stale_after_seconds as u64;
    if crate::runtime_dispatch_receipt_helpers::dispatch_packet_uses_downstream_carrier(
        receipt.dispatch_packet_path.as_deref(),
        &result,
    ) {
        apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)?;
        return Ok(true);
    }
    if preserves_internal_activation_view {
        let project_root = runtime_dispatch_project_root_from_state_root(state_root);
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "stale-normalization".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: String::new(),
            selected_role: receipt
                .activation_runtime_role
                .clone()
                .unwrap_or_else(|| receipt.dispatch_target.clone()),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "stale-normalization".to_string(),
        };
        apply_internal_activation_timeout_to_receipt(
            state_root,
            project_root.as_ref(),
            &role_selection,
            receipt,
            timeout_seconds,
        )?;
    } else {
        apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)?;
    }
    Ok(true)
}

fn safe_dispatch_result_run_id(run_id: &str) -> String {
    let trimmed = run_id.trim();
    if trimmed.is_empty() {
        return "run".to_string();
    }
    let sanitized: String = trimmed
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();
    if sanitized.chars().any(|ch| ch != '_') {
        sanitized
    } else {
        "run".to_string()
    }
}

pub(crate) fn write_runtime_lane_completion_result(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let safe_run_id = safe_dispatch_result_run_id(run_id);
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let result_path = result_dir.join(format!("{safe_run_id}-{ts}.json"));
    let body = serde_json::json!({
        "artifact_kind": "runtime_lane_completion_result",
        "status": "pass",
        "execution_state": "executed",
        "run_id": run_id,
        "completed_target": completed_target,
        "completion_receipt_id": receipt_id,
        "source_dispatch_packet_path": source_dispatch_packet_path,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode lane completion result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write lane completion result: {error}"))?;
    Ok(result_path.display().to_string())
}

async fn coordinate_dispatch_timeout_state_best_effort(
    state_root: &Path,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    let store = match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    {
        Ok(Ok(store)) => store,
        Ok(Err(error)) => {
            return Some(format!(
                "authoritative timeout reconciliation deferred until next safe reopen: failed to reopen state store after dispatch timeout: {error}"
            ));
        }
        Err(_) => {
            return Some(format!(
                "authoritative timeout reconciliation deferred until next safe reopen: timed out reopening state store after dispatch timeout after {}s",
                DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
            ));
        }
    };

    if let Err(error) = store.record_run_graph_dispatch_receipt(receipt).await {
        return Some(format!(
            "authoritative timeout reconciliation deferred until next safe reopen: failed to persist timeout-blocked dispatch receipt after execution timeout: {error}"
        ));
    }

    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        match store.run_graph_status(&run_id).await {
            Ok(status) => {
                let mut blocked_status =
                    apply_first_handoff_execution_to_run_graph_status(&status, receipt);
                blocked_status.handoff_state = "blocked".to_string();
                if let Err(error) = store.record_run_graph_status(&blocked_status).await {
                    return Some(format!(
                        "authoritative timeout reconciliation deferred until next safe reopen: failed to record blocked run-graph status after dispatch timeout: {error}"
                    ));
                }
                if let Err(error) =
                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                        &store,
                        &blocked_status,
                        "dispatch_execution_timeout",
                    )
                    .await
                {
                    return Some(format!(
                        "authoritative timeout reconciliation deferred until next safe reopen: failed to synchronize continuation binding after dispatch timeout: {error}"
                    ));
                }
            }
            Err(error) => {
                return Some(format!(
                    "authoritative timeout reconciliation deferred until next safe reopen: failed to read run-graph status after dispatch timeout: {error}"
                ));
            }
        }
    }

    None
}

async fn persist_prelaunch_blocked_dispatch_state(
    state_root: &Path,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let store = reopen_authoritative_state_store_for_dispatch_phase(
        state_root,
        receipt,
        "after prelaunch dispatch block",
    )
    .await?;
    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        if let Ok(status) = store.run_graph_status(&run_id).await {
            let blocked_status =
                apply_first_handoff_execution_to_run_graph_status(&status, receipt);
            store
                .record_run_graph_status(&blocked_status)
                .await
                .map_err(|error| {
                    format!("Failed to record blocked run-graph status after prelaunch dispatch block: {error}")
                })?;
            crate::taskflow_continuation::sync_run_graph_continuation_binding(
                &store,
                &blocked_status,
                "dispatch_prelaunch_blocked",
            )
            .await
            .map_err(|error| {
                format!(
                    "Failed to synchronize continuation binding after prelaunch dispatch block: {error}"
                )
            })?;
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| {
            format!(
                "Failed to persist blocked dispatch receipt after prelaunch dispatch block: {error}"
            )
        })?;
    Ok(())
}

pub(crate) async fn execute_and_record_dispatch_receipt(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if receipt.dispatch_kind == "agent_lane" {
        receipt.selected_backend = preferred_selected_backend_for_receipt(role_selection, receipt);
    }
    let project_root = runtime_dispatch_project_root_from_state_root(state_root);
    sync_receipt_dispatch_handoff_surface(project_root.as_ref(), role_selection, receipt);
    if internal_host_dispatch_requires_prelaunch_blocker(
        project_root.as_ref(),
        role_selection,
        receipt,
    ) {
        apply_internal_activation_view_only_to_receipt(
            state_root,
            project_root.as_ref(),
            role_selection,
            receipt,
        )?;
        persist_prelaunch_blocked_dispatch_state(state_root, run_graph_bootstrap, receipt).await?;
        return Ok(());
    }
    record_dispatch_execution_started(state_root, role_selection, run_graph_bootstrap, receipt)
        .await?;
    let handoff_timeout_seconds =
        dispatch_execution_timeout_seconds(project_root.as_ref(), role_selection, receipt);
    let execution_result = if dispatch_handoff_requires_outer_timeout(
        project_root.as_ref(),
        role_selection,
        receipt,
    ) {
        tokio::time::timeout(
            std::time::Duration::from_secs(handoff_timeout_seconds),
            execute_runtime_dispatch_handoff(state_root, role_selection, receipt),
        )
        .await
        .map_err(|_| {
            format!(
                "Timed out executing runtime dispatch handoff after {}s",
                handoff_timeout_seconds
            )
        })
    } else {
        Ok(execute_runtime_dispatch_handoff(state_root, role_selection, receipt).await)
    };
    let mut execution_result = match execution_result {
        Ok(result) => match result {
            Ok(execution_result) => execution_result,
            Err(execution_error) => {
                persist_failed_dispatch_handoff_state(
                    state_root,
                    role_selection,
                    run_graph_bootstrap,
                    receipt,
                    &execution_error,
                )
                .await?;
                return Err(execution_error);
            }
        },
        Err(_timeout_error) => {
            // Generate timeout execution result with receipt-backed completion evidence
            let internal_host_timeout =
                dispatch_handoff_uses_internal_host(project_root.as_ref(), role_selection, receipt);
            let receipt_timeout_seconds = if internal_host_timeout {
                internal_host_runtime_window_seconds(project_root.as_ref(), role_selection, receipt)
            } else {
                handoff_timeout_seconds
            };
            let blocker_code = if internal_host_timeout {
                annotate_internal_host_timeout_surface(project_root.as_ref(), receipt);
                INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
            } else {
                "dispatch_handoff_timeout"
            };
            let execution_result = runtime_dispatch_internal_activation_timeout_result(
                receipt,
                receipt_timeout_seconds,
                blocker_code,
            );

            // Update receipt with timeout information
            if internal_host_timeout {
                apply_internal_activation_timeout_to_receipt(
                    state_root,
                    project_root.as_ref(),
                    role_selection,
                    receipt,
                    receipt_timeout_seconds,
                )?;
            } else {
                apply_dispatch_handoff_timeout_to_receipt(
                    state_root,
                    project_root.as_ref(),
                    role_selection,
                    receipt,
                    receipt_timeout_seconds,
                )?;
            }

            // Best-effort persist timeout receipt
            let deferred_coordination_warning = coordinate_dispatch_timeout_state_best_effort(
                state_root,
                run_graph_bootstrap,
                receipt,
            )
            .await;

            // Return timeout as a valid blocked execution result, not an error
            // This ensures the timeout receipt is properly recorded and persisted
            if let Some(warning) = deferred_coordination_warning {
                // Log warning but still return the execution result
                eprintln!("Timeout coordination warning: {warning}");
            }
            execution_result
        }
    };
    normalize_internal_host_timeout_result_blocker(
        project_root.as_ref(),
        role_selection,
        receipt,
        &mut execution_result,
    );
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    let execution_state = json_string(execution_result.get("execution_state"))
        .unwrap_or_else(|| "executed".to_string());
    receipt.dispatch_status = execution_state;
    let mut lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    );
    let closure_completed = receipt.dispatch_target == "closure"
        && receipt.dispatch_status == "executed"
        && json_bool(execution_result.get("closure_ready"), false)
        && lane_status == LaneStatus::LaneRunning;
    if closure_completed {
        lane_status = LaneStatus::LaneCompleted;
    }
    receipt.lane_status = lane_status.as_str().to_string();
    receipt.blocker_code =
        if receipt.dispatch_status == "blocked" && receipt.dispatch_packet_path.is_none() {
            blocker_code_value(BlockerCode::MissingPacket)
        } else if receipt.dispatch_status == "blocked" {
            json_string(execution_result.get("blocker_code"))
        } else {
            None
        };
    if let Some(dispatch_surface) = json_string(execution_result.get("surface")) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    let store = reopen_authoritative_state_store_for_dispatch_phase(
        state_root,
        receipt,
        "after dispatch execution",
    )
    .await?;
    tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        refresh_downstream_dispatch_preview(&store, role_selection, run_graph_bootstrap, receipt),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out refreshing downstream dispatch preview after dispatch execution after {}s",
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })??;
    if matches!(receipt.dispatch_status.as_str(), "executed" | "blocked") {
        if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
            if let Ok(status) = store.run_graph_status(&run_id).await {
                let receipt_matches_current_lane = status.active_node == receipt.dispatch_target
                    || status.next_node.as_deref() == Some(receipt.dispatch_target.as_str());
                let mut terminal_status =
                    if receipt.dispatch_status == "executed" && receipt_matches_current_lane {
                        match crate::taskflow_run_graph::derive_advanced_run_graph_status(
                            &store,
                            status.clone(),
                        )
                        .await
                        {
                            Ok(payload) => payload.status,
                            Err(_) => {
                                apply_first_handoff_execution_to_run_graph_status(&status, receipt)
                            }
                        }
                    } else {
                        apply_first_handoff_execution_to_run_graph_status(&status, receipt)
                    };
                if receipt.dispatch_status == "blocked" {
                    terminal_status.handoff_state = "blocked".to_string();
                }
                if let Some(selected_backend) = receipt
                    .selected_backend
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    terminal_status.selected_backend = selected_backend.to_string();
                }
                store
                    .record_run_graph_status(&terminal_status)
                    .await
                    .map_err(|error| {
                        format!("Failed to record terminal run-graph status: {error}")
                    })?;
                crate::taskflow_continuation::sync_run_graph_continuation_binding(
                    &store,
                    &terminal_status,
                    "dispatch_execution",
                )
                .await
                .map_err(|error| {
                    format!(
                        "Failed to synchronize continuation binding after dispatch execution: {error}"
                    )
                })?;
            }
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| format!("Failed to persist dispatch receipt after execution: {error}"))?;
    Ok(())
}

async fn persist_failed_dispatch_handoff_state(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    execution_error: &str,
) -> Result<(), String> {
    let failure_result = serde_json::json!({
        "surface": receipt.dispatch_surface.clone().unwrap_or_else(|| "vida agent-init".to_string()),
        "status": "blocked",
        "execution_state": "blocked",
        "blocker_code": "dispatch_execution_handoff_failed",
        "blocker_message": execution_error,
    });
    let dispatch_result_path = write_runtime_dispatch_result(state_root, receipt, &failure_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    receipt.dispatch_status = "blocked".to_string();
    receipt.lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = Some("dispatch_execution_handoff_failed".to_string());

    let store = reopen_authoritative_state_store_for_dispatch_phase(
        state_root,
        receipt,
        "after dispatch handoff failure",
    )
    .await?;
    tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        refresh_downstream_dispatch_preview(&store, role_selection, run_graph_bootstrap, receipt),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out refreshing downstream dispatch preview after dispatch handoff failure after {}s",
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })??;
    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        if let Ok(status) = store.run_graph_status(&run_id).await {
            let blocked_status =
                apply_first_handoff_execution_to_run_graph_status(&status, receipt);
            store
                .record_run_graph_status(&blocked_status)
                .await
                .map_err(|error| format!("Failed to record blocked run-graph status after dispatch handoff failure: {error}"))?;
            crate::taskflow_continuation::sync_run_graph_continuation_binding(
                &store,
                &blocked_status,
                "dispatch_execution_failed",
            )
            .await
            .map_err(|error| {
                format!(
                    "Failed to synchronize continuation binding after dispatch handoff failure: {error}"
                )
            })?;
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| {
            format!(
                "Failed to persist blocked dispatch receipt after dispatch handoff failure: {error}"
            )
        })?;
    Ok(())
}

pub(crate) async fn execute_downstream_dispatch_chain(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let root_lane_has_execution_evidence = if dispatch_receipt_has_execution_evidence(root_receipt)
    {
        true
    } else {
        let store = StateStore::open_existing(state_root.to_path_buf())
            .await
            .map_err(|error| {
                format!(
                    "Failed to reopen authoritative state store for downstream execution evidence: {error}"
                )
            })?;
        tracked_implementer_task_closed(&store, role_selection, root_receipt).await
    };
    if !root_lane_has_execution_evidence || !root_receipt.downstream_dispatch_ready {
        return Ok(());
    }

    let mut downstream_source = root_receipt.clone();
    let mut downstream_trace = Vec::new();
    for _ in 0..4 {
        let Some(mut downstream_receipt) =
            build_downstream_dispatch_receipt(role_selection, &downstream_source)
        else {
            break;
        };
        if downstream_receipt.dispatch_status != "routed"
            || (downstream_receipt.dispatch_kind == "taskflow_pack"
                && taskflow_task_bridge::infer_project_root_from_state_root(state_root).is_none())
        {
            root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
            break;
        }

        execute_and_record_dispatch_receipt(
            state_root,
            role_selection,
            run_graph_bootstrap,
            &mut downstream_receipt,
        )
        .await
        .map_err(|error| {
            format!("Failed to execute downstream runtime dispatch handoff: {error}")
        })?;

        let store = StateStore::open_existing(state_root.to_path_buf())
            .await
            .map_err(|error| {
                format!(
                    "Failed to reopen authoritative state store for downstream preview refresh: {error}"
                )
            })?;
        let (next_target, next_command, next_note, next_ready, next_blockers) =
            derive_downstream_dispatch_preview(&store, role_selection, &downstream_receipt).await;
        if let Some(error) =
            downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
        {
            return Err(error);
        }
        let preview_result_path = downstream_receipt.dispatch_result_path.clone();
        apply_downstream_dispatch_preview_to_receipt(
            &mut downstream_receipt,
            next_target,
            next_command,
            next_note,
            next_ready,
            next_blockers,
            preview_result_path,
        );
        let implementation_owned_paths = implementation_owned_paths_for_dispatch_context(
            &store,
            role_selection,
            &downstream_receipt,
        )
        .await;
        downstream_receipt.downstream_dispatch_packet_path =
            write_runtime_downstream_dispatch_packet_with_owned_paths(
                state_root,
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
                &implementation_owned_paths,
            )
            .map_err(|error| {
                format!("Failed to write chained downstream runtime dispatch packet: {error}")
            })?;
        if let Some(packet_path) = downstream_receipt
            .downstream_dispatch_packet_path
            .as_deref()
        {
            write_runtime_downstream_dispatch_packet_at_with_owned_paths(
                Path::new(packet_path),
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
                &implementation_owned_paths,
            )
            .map_err(|error| {
                format!("Failed to refresh chained downstream runtime dispatch packet: {error}")
            })?;
        }

        downstream_trace
            .push(serde_json::to_value(&downstream_receipt).unwrap_or(serde_json::Value::Null));
        if downstream_receipt.dispatch_status == "executed" {
            root_receipt.downstream_dispatch_executed_count += 1;
        }
        root_receipt.downstream_dispatch_last_target =
            Some(downstream_receipt.dispatch_target.clone());
        root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
        if !downstream_receipt.downstream_dispatch_ready {
            break;
        }
        downstream_source = downstream_receipt;
    }

    if !downstream_trace.is_empty() {
        let trace_path = write_runtime_downstream_dispatch_trace(
            state_root,
            &root_receipt.run_id,
            &downstream_trace,
        )
        .map_err(|error| format!("Failed to write downstream runtime dispatch trace: {error}"))?;
        root_receipt.downstream_dispatch_trace_path = Some(trace_path);
    }
    Ok(())
}

pub(crate) fn apply_first_handoff_execution_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    if receipt.dispatch_target == "closure" {
        if canonical_lane_status_str(&receipt.lane_status) != Some("lane_completed") {
            return crate::state_store::RunGraphStatus {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_class: status.task_class.clone(),
                active_node: "closure".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: status.route_task_class.clone(),
                selected_backend: receipt
                    .selected_backend
                    .clone()
                    .unwrap_or_else(|| status.selected_backend.clone()),
                lane_id: "closure_direct".to_string(),
                lifecycle_stage: "closure_active".to_string(),
                policy_gate: status.policy_gate.clone(),
                handoff_state: "none".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: status.checkpoint_kind.clone(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
        }
        return crate::state_store::RunGraphStatus {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_class: status.task_class.clone(),
            active_node: "closure".to_string(),
            next_node: None,
            status: "completed".to_string(),
            route_task_class: status.route_task_class.clone(),
            selected_backend: receipt
                .selected_backend
                .clone()
                .unwrap_or_else(|| status.selected_backend.clone()),
            lane_id: "closure_direct".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            policy_gate: status.policy_gate.clone(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: status.checkpoint_kind.clone(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
    }
    let dispatch_target = receipt.dispatch_target.replace('-', "_");
    let next_node =
        if receipt.downstream_dispatch_ready && receipt.downstream_dispatch_blockers.is_empty() {
            receipt
                .downstream_dispatch_target
                .as_deref()
                .map(|target| target.replace('-', "_"))
        } else {
            None
        };
    let (handoff_state, resume_target) = if let Some(next_target) = next_node.as_deref() {
        (
            format!("awaiting_{next_target}"),
            format!("dispatch.{next_target}"),
        )
    } else {
        ("none".to_string(), "none".to_string())
    };
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node,
        status: "ready".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: if receipt.dispatch_kind == "taskflow_pack" {
            format!("{dispatch_target}_direct")
        } else {
            format!("{dispatch_target}_lane")
        },
        lifecycle_stage: format!("{dispatch_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target,
        recovery_ready: true,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}

fn apply_dispatch_execution_started_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    let dispatch_target = receipt.dispatch_target.replace('-', "_");
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node: None,
        status: "running".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: if receipt.dispatch_kind == "taskflow_pack" {
            format!("{dispatch_target}_direct")
        } else {
            format!("{dispatch_target}_lane")
        },
        lifecycle_stage: format!("{dispatch_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state: "none".to_string(),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}
