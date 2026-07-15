use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

use crate::runtime_assignment_policy::DispatchContractLane;
use crate::runtime_lane_summary::summarize_execution_truth_for_route;
use crate::runtime_proof_scope::proof_scope_from_dispatch_packet_path;
use crate::{RuntimeConsumptionLaneSelection, StateStore, yaml_lookup};
use taskflow_host_bridge::{
    DispatchReceiptBindingInput, HostBridgeRequest, default_host_bridge_required_result_fields,
    host_bridge_artifact_has_retryable_completion_blocker,
    host_bridge_completed_artifact_status_is_admissible,
    host_bridge_completed_result_execution_state_is_admissible,
    host_bridge_completed_result_status_is_admissible,
    host_bridge_existing_request_status_is_admissible,
    host_bridge_result_verdict_contract_blockers, validate_dispatch_receipt_binding,
};

fn canonical_dispatch_target_for_admissibility(dispatch_target: &str) -> String {
    crate::runtime_assignment_policy::backend_admissibility_key_for_dispatch_target(
        dispatch_target,
        None,
    )
    .into_string()
}

/// Check whether a backend is admissible for a given dispatch target (lane).
/// When no admissibility matrix is present, keep fail-open behavior for backward
/// compatibility. Once a matrix exists, write-producing lanes fail closed if the
/// backend row, lane mapping, or canonical lane key is missing.
fn backend_is_admissible_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
) -> bool {
    let policy_dispatch_target =
        crate::runtime_dispatch_state::policy_dispatch_target_for_admissibility(
            execution_plan,
            dispatch_target,
        );
    let lane = crate::dispatch_contract_lane(execution_plan, &policy_dispatch_target)
        .map(DispatchContractLane::from_value);
    crate::runtime_assignment_policy::backend_is_admissible_for_dispatch_target(
        execution_plan,
        backend_id,
        &policy_dispatch_target,
        lane.as_ref(),
    )
}

fn execution_plan_backend_class(
    role_selection: &RuntimeConsumptionLaneSelection,
    backend_id: &str,
) -> Option<String> {
    role_selection.execution_plan["backend_admissibility_matrix"]
        .as_array()?
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))?
        .get("backend_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn configured_backend_class(
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> Option<String> {
    let entry =
        overlay.and_then(|overlay| configured_subagent_backend_entry(overlay, backend_id))?;
    crate::yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn backend_is_internal_host_bridge(
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> bool {
    execution_plan_backend_class(role_selection, backend_id)
        .or_else(|| configured_backend_class(overlay, backend_id))
        .as_deref()
        .is_some_and(|backend_class| matches!(backend_class, "internal" | "internal_cli"))
}

fn backend_is_external_cli_bridge(
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> bool {
    execution_plan_backend_class(role_selection, backend_id)
        .or_else(|| configured_backend_class(overlay, backend_id))
        .as_deref()
        .is_some_and(|backend_class| backend_class == "external_cli")
}

fn default_activation_view(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    serde_json::json!({
        "selection": {
            "mode": "dispatch_packet",
            "selected_role": receipt
                .activation_runtime_role
                .as_deref()
                .unwrap_or(&role_selection.selected_role),
        },
        "activation_semantics": {
            "activation_kind": "activation_view",
            "view_only": true,
        },
    })
}

const DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS: u64 = 1;
const DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS: u64 = 2;

async fn bounded_activation_view(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let open_store = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await;
    let Ok(Ok(store)) = open_store else {
        return default_activation_view(receipt, role_selection);
    };

    let rendered = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS),
        crate::init_surfaces::render_agent_init_packet_activation_with_store(
            &store,
            project_root,
            dispatch_packet_path,
            dispatch_packet_path_should_render_as_downstream(dispatch_packet_path),
        ),
    )
    .await;
    drop(store);

    match rendered {
        Ok(Ok(view)) => view,
        _ => default_activation_view(receipt, role_selection),
    }
}

fn dispatch_packet_path_should_render_as_downstream(dispatch_packet_path: &str) -> bool {
    let Ok(body) = std::fs::read_to_string(dispatch_packet_path) else {
        return false;
    };
    let Ok(packet) = serde_json::from_str::<serde_json::Value>(&body) else {
        return false;
    };
    packet["packet_kind"].as_str() == Some("runtime_downstream_dispatch_packet")
        || packet["downstream_dispatch_target"]
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn readiness_fallback_internal_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let fallback_backend = crate::taskflow_routing::fallback_executor_backend_from_route(route)?
        .trim()
        .to_string();
    if fallback_backend.is_empty()
        || fallback_backend == blocked_backend_id
        || !backend_is_internal_host_bridge(role_selection, None, &fallback_backend)
    {
        return None;
    }
    backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        &fallback_backend,
        dispatch_target,
    )
    .then_some(fallback_backend)
}

fn push_unique_backend_candidate(candidates: &mut Vec<String>, candidate: Option<String>) {
    let Some(candidate) = candidate
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn external_readiness_blocker_allows_default_profile_retry(
    readiness_verdict: &serde_json::Value,
) -> bool {
    readiness_verdict["blocker_code"].as_str()
        == Some(crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ModelNotPinned,
        ))
        || readiness_verdict["status"].as_str() == Some("pi_model_unavailable")
        || readiness_verdict["model_catalog"]["status"].as_str() == Some("model_not_found")
}

fn dispatch_profile_target_candidates(
    dispatch_target: &str,
) -> (Vec<&'static str>, Vec<&'static str>) {
    match canonical_dispatch_target_for_admissibility(dispatch_target).as_str() {
        "coach" => (vec!["coach"], vec!["coach", "review"]),
        "verification" | "verifier" => (vec!["verifier"], vec!["verification", "review"]),
        "analysis" => (
            vec!["business_analyst", "worker"],
            vec!["analysis", "planning", "specification"],
        ),
        "architecture" => (
            vec!["solution_architect"],
            vec!["architecture", "execution_preparation"],
        ),
        _ => (
            vec!["worker"],
            vec!["implementation", "delivery_task", "execution_block"],
        ),
    }
}

fn profile_list_allows(profile: &serde_json::Value, key: &str, candidates: &[&str]) -> bool {
    let Some(rows) = profile.get(key).and_then(serde_json::Value::as_array) else {
        return true;
    };
    if rows.is_empty() {
        return true;
    }
    rows.iter()
        .filter_map(serde_json::Value::as_str)
        .any(|row| {
            let row = row.trim();
            !row.is_empty() && candidates.iter().any(|candidate| row == *candidate)
        })
}

fn profile_supports_dispatch_target(profile: &serde_json::Value, dispatch_target: &str) -> bool {
    let (role_candidates, task_class_candidates) =
        dispatch_profile_target_candidates(dispatch_target);
    profile_list_allows(profile, "runtime_roles", &role_candidates)
        && profile_list_allows(profile, "task_classes", &task_class_candidates)
}

fn dispatch_target_requires_owned_scope(dispatch_target: &str) -> bool {
    canonical_dispatch_target_for_admissibility(dispatch_target) == "implementation"
}

fn profile_compatible_with_packet_scope(
    profile: &serde_json::Value,
    dispatch_target: &str,
    packet_has_concrete_owned_paths: bool,
) -> bool {
    if !crate::runtime_dispatch_state::selected_profile_requires_owned_path_guard(profile) {
        return true;
    }
    packet_has_concrete_owned_paths || dispatch_target_requires_owned_scope(dispatch_target)
}

fn profile_id(profile: &serde_json::Value) -> Option<&str> {
    profile["profile_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn selected_profile_for_backend(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    profile_id: Option<&str>,
) -> Option<serde_json::Value> {
    let profile_projection = crate::runtime_dispatch_state::external_backend_profile_projection(
        backend_id,
        backend_entry,
    );
    crate::model_profile_contract::selected_model_profile_from_json_row(
        &profile_projection,
        profile_id,
    )
}

fn ready_external_profile_for_dispatch_target(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    policy_dispatch_target: &str,
    packet_has_concrete_owned_paths: bool,
    excluded_profile_id: Option<&str>,
) -> Option<(serde_json::Value, String)> {
    let profile_projection = crate::runtime_dispatch_state::external_backend_profile_projection(
        backend_id,
        backend_entry,
    );
    let mut profiles =
        crate::model_profile_contract::model_profiles_from_json_row(&profile_projection);
    profiles.sort_by(|left, right| {
        let left_guard =
            crate::runtime_dispatch_state::selected_profile_requires_owned_path_guard(left);
        let right_guard =
            crate::runtime_dispatch_state::selected_profile_requires_owned_path_guard(right);
        left_guard.cmp(&right_guard).then_with(|| {
            profile_id(left)
                .unwrap_or_default()
                .cmp(profile_id(right).unwrap_or_default())
        })
    });

    for profile in profiles {
        let Some(candidate_profile_id) = profile_id(&profile) else {
            continue;
        };
        if excluded_profile_id == Some(candidate_profile_id) {
            continue;
        }
        if !profile_supports_dispatch_target(&profile, &policy_dispatch_target) {
            continue;
        }
        if !profile_compatible_with_packet_scope(
            &profile,
            &policy_dispatch_target,
            packet_has_concrete_owned_paths,
        ) {
            continue;
        }
        let readiness =
            crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
                backend_id,
                backend_entry,
                Some(candidate_profile_id),
            );
        if !readiness["blocked"].as_bool().unwrap_or(false) {
            return Some((readiness, candidate_profile_id.to_string()));
        }
    }
    None
}

fn external_cli_dispatch_readiness_verdict(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    selected_model_profile_id: Option<String>,
    policy_dispatch_target: &str,
    packet_has_concrete_owned_paths: bool,
) -> (serde_json::Value, Option<String>) {
    let selected_readiness =
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            backend_id,
            backend_entry,
            selected_model_profile_id.as_deref(),
        );
    let selected_profile = selected_model_profile_id.or_else(|| {
        selected_readiness["selected_model_profile"]
            .as_str()
            .map(str::to_string)
    });
    let selected_profile_guard_incompatible = selected_profile
        .as_deref()
        .and_then(|profile_id| {
            selected_profile_for_backend(backend_id, backend_entry, Some(profile_id))
        })
        .as_ref()
        .is_some_and(|profile| {
            !profile_compatible_with_packet_scope(
                profile,
                policy_dispatch_target,
                packet_has_concrete_owned_paths,
            )
        });
    if selected_profile_guard_incompatible {
        if let Some((mut fallback_readiness, fallback_profile)) =
            ready_external_profile_for_dispatch_target(
                backend_id,
                backend_entry,
                policy_dispatch_target,
                packet_has_concrete_owned_paths,
                selected_profile.as_deref(),
            )
        {
            if let Some(body) = fallback_readiness.as_object_mut() {
                body.insert(
                    "guarded_write_profile_retry".to_string(),
                    serde_json::json!({
                        "selected_model_profile": selected_profile,
                        "reason": "selected_profile_requires_owned_paths_but_packet_has_no_owned_scope",
                        "dispatch_target": policy_dispatch_target,
                    }),
                );
            }
            return (fallback_readiness, Some(fallback_profile));
        }

        let mut blocked_readiness = selected_readiness;
        if let Some(body) = blocked_readiness.as_object_mut() {
            body.insert("blocked".to_string(), serde_json::json!(true));
            body.insert(
                "status".to_string(),
                serde_json::json!("external_profile_requires_owned_paths"),
            );
            body.insert(
                "blocker_code".to_string(),
                serde_json::json!("missing_owned_write_scope"),
            );
            body.insert(
                "next_actions".to_string(),
                serde_json::json!([
                    "Select a read-only external model profile for this non-write lane or provide bounded owned paths for a write-producing lane."
                ]),
            );
        }
        return (blocked_readiness, selected_profile);
    }
    if !selected_readiness["blocked"].as_bool().unwrap_or(false)
        || selected_profile.is_none()
        || !external_readiness_blocker_allows_default_profile_retry(&selected_readiness)
    {
        return (selected_readiness, selected_profile);
    }

    if let Some((mut fallback_readiness, fallback_profile)) =
        ready_external_profile_for_dispatch_target(
            backend_id,
            backend_entry,
            policy_dispatch_target,
            packet_has_concrete_owned_paths,
            selected_profile.as_deref(),
        )
    {
        if let Some(body) = fallback_readiness.as_object_mut() {
            body.insert(
                "stale_selected_profile_retry".to_string(),
                serde_json::json!({
                    "selected_model_profile": selected_profile,
                    "selected_readiness_status": selected_readiness["status"].clone(),
                    "selected_blocker_code": selected_readiness["blocker_code"].clone(),
                }),
            );
        }
        return (fallback_readiness, Some(fallback_profile));
    }

    let mut default_readiness =
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            backend_id,
            backend_entry,
            None,
        );
    if default_readiness["blocked"].as_bool().unwrap_or(false) {
        return (selected_readiness, selected_profile);
    }

    if let Some(body) = default_readiness.as_object_mut() {
        body.insert(
            "stale_selected_profile_retry".to_string(),
            serde_json::json!({
                "selected_model_profile": selected_profile,
                "selected_readiness_status": selected_readiness["status"].clone(),
                "selected_blocker_code": selected_readiness["blocker_code"].clone(),
            }),
        );
    }
    let default_profile = default_readiness["selected_model_profile"]
        .as_str()
        .map(str::to_string);
    (default_readiness, default_profile)
}

pub(crate) fn internal_host_external_fallback_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
    overlay: &serde_yaml::Value,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let policy_dispatch_target =
        crate::runtime_dispatch_state::policy_dispatch_target_for_admissibility(
            &role_selection.execution_plan,
            dispatch_target,
        );
    let mut candidates = Vec::new();
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::fallback_executor_backend_from_route(route),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::runtime_assignment_backend_for_route(
            &role_selection.execution_plan,
            route,
        ),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::route_primary_backend_hint_from_route(route),
    );
    for candidate in crate::taskflow_routing::fanout_executor_backends_from_route(route) {
        push_unique_backend_candidate(&mut candidates, Some(candidate));
    }

    candidates.into_iter().find(|candidate| {
        if candidate == blocked_backend_id {
            return false;
        }
        if !backend_is_external_cli_bridge(role_selection, Some(overlay), candidate) {
            return false;
        }
        if !backend_is_admissible_for_dispatch_target(
            &role_selection.execution_plan,
            candidate,
            dispatch_target,
        ) {
            return false;
        }
        let Some(backend_entry) =
            crate::runtime_dispatch_state::configured_external_backend_entry(overlay, candidate)
        else {
            return false;
        };
        if crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            candidate,
            backend_entry,
        )
        .is_some()
        {
            return false;
        }
        let selected_model_profile_id =
            crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
                role_selection,
                dispatch_target,
                Some(candidate),
            );
        let (readiness, _) = external_cli_dispatch_readiness_verdict(
            candidate,
            backend_entry,
            selected_model_profile_id,
            &policy_dispatch_target,
            dispatch_target_requires_owned_scope(&policy_dispatch_target),
        );
        !readiness["blocked"].as_bool().unwrap_or(false)
    })
}

fn ready_external_readiness_fallback_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
    overlay: &serde_yaml::Value,
    inherited_selected_backend: Option<&str>,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let policy_dispatch_target =
        crate::runtime_dispatch_state::policy_dispatch_target_for_admissibility(
            &role_selection.execution_plan,
            dispatch_target,
        );
    let mut candidates = Vec::new();
    push_unique_backend_candidate(
        &mut candidates,
        inherited_selected_backend.map(str::to_string),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::runtime_assignment_backend_for_route(
            &role_selection.execution_plan,
            route,
        ),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::route_primary_backend_hint_from_route(route),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::fallback_executor_backend_from_route(route),
    );
    for candidate in crate::taskflow_routing::fanout_executor_backends_from_route(route) {
        push_unique_backend_candidate(&mut candidates, Some(candidate));
    }

    candidates.into_iter().find(|candidate| {
        if candidate == blocked_backend_id {
            return false;
        }
        if !backend_is_external_cli_bridge(role_selection, Some(overlay), candidate) {
            return false;
        }
        if !backend_is_admissible_for_dispatch_target(
            &role_selection.execution_plan,
            candidate,
            dispatch_target,
        ) {
            return false;
        }
        let Some(backend_entry) =
            crate::runtime_dispatch_state::configured_external_backend_entry(overlay, candidate)
        else {
            return false;
        };
        if crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            candidate,
            backend_entry,
        )
        .is_some()
        {
            return false;
        }
        let selected_model_profile_id =
            crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
                role_selection,
                dispatch_target,
                Some(candidate),
            );
        let (readiness, _) = external_cli_dispatch_readiness_verdict(
            candidate,
            backend_entry,
            selected_model_profile_id,
            &policy_dispatch_target,
            dispatch_target_requires_owned_scope(&policy_dispatch_target),
        );
        !readiness["blocked"].as_bool().unwrap_or(false)
    })
}

fn configured_external_dispatch_wall_timeout_seconds(
    backend_entry: &serde_yaml::Value,
) -> Option<u64> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    yaml_lookup(backend_entry, &["max_runtime_seconds"])
        .and_then(serde_yaml::Value::as_u64)
        .or_else(|| {
            yaml_lookup(dispatch, &["no_output_timeout_seconds"])
                .and_then(serde_yaml::Value::as_u64)
        })
        .filter(|seconds| *seconds > 0)
}

fn configured_internal_host_dispatch_wall_timeout_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    crate::runtime_dispatch_state::internal_host_runtime_window_seconds(
        project_root,
        role_selection,
        receipt,
    )
}

fn configured_internal_host_dispatch_no_output_timeout_seconds(
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> Option<u64> {
    selected_cli_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch", "no_output_timeout_seconds"]))
        .and_then(serde_yaml::Value::as_u64)
        .filter(|seconds| *seconds > 0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandTimeoutWrapper {
    timeout_seconds: u64,
    kill_after_grace_seconds: u64,
    no_output_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WrappedCommand {
    command: String,
    args: Vec<String>,
    timeout_wrapper: Option<CommandTimeoutWrapper>,
}

#[derive(Debug)]
struct ObservedCommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

#[cfg(test)]
fn test_exit_status(code: i32) -> ExitStatus {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
}

#[cfg(test)]
fn emulated_test_shell_output(wrapped_command: &WrappedCommand) -> Option<ObservedCommandOutput> {
    if matches!(
        wrapped_command.command.as_str(),
        "qwen" | "hermes" | "opencode"
    ) {
        let stdout = serde_json::json!({
            "type": "result",
            "result": format!("external-dispatch:{}", wrapped_command.args.join(" ")),
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    let all_args = wrapped_command.args.join(" ");
    if all_args.contains("sleep 30") || all_args.contains("trap") {
        return Some(ObservedCommandOutput {
            status: test_exit_status(124),
            stdout: Vec::new(),
            stderr: b"test shell command timed out".to_vec(),
            timed_out: true,
        });
    }
    if wrapped_command.command != "sh" {
        return None;
    }
    let script = wrapped_command
        .args
        .windows(2)
        .find_map(|pair| (pair[0] == "-lc").then(|| pair[1].as_str()))
        .unwrap_or_default();
    if script.contains("sleep 30") || script.contains("trap") {
        return Some(ObservedCommandOutput {
            status: test_exit_status(124),
            stdout: Vec::new(),
            stderr: b"test shell command timed out".to_vec(),
            timed_out: true,
        });
    }
    if script.contains("external-dispatch:%s") {
        let prompt_args = wrapped_command
            .args
            .iter()
            .position(|arg| arg == "vida-dispatch")
            .map(|index| wrapped_command.args[index + 1..].to_vec())
            .unwrap_or_default();
        let rendered = if script.contains("\"$*\"") {
            prompt_args.join(" ")
        } else {
            prompt_args.first().cloned().unwrap_or_default()
        };
        let stdout = serde_json::json!({
            "type": "result",
            "result": format!("external-dispatch:{rendered}"),
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    if script.contains("input=$(cat)") {
        let stdout = serde_json::json!({
            "type": "result",
            "result": "STDIN_OK",
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    if script.contains("adapter boom") {
        let stdout = serde_json::json!({
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "error": {
                "message": "adapter boom"
            }
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(1),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    None
}

#[derive(Debug)]
enum TimeoutProgress {
    WaitingForDeadline(Instant),
    WaitingForKill(Instant),
    TimedOut,
}

#[cfg(unix)]
fn signal_process_group(process_group_id: u32, signal: libc::c_int) -> Result<(), String> {
    let result = unsafe { libc::killpg(process_group_id as libc::pid_t, signal) };
    if result == 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code) if code == libc::ESRCH => Ok(()),
        _ => Err(format!(
            "failed to signal process group {process_group_id} with signal {signal}: {error}"
        )),
    }
}

enum CommandOutputEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    StdoutDone,
    StderrDone,
}

fn spawn_reader_thread<T>(
    stream: Option<T>,
    sender: mpsc::Sender<CommandOutputEvent>,
    stdout: bool,
) -> std::thread::JoinHandle<()>
where
    T: Read + Send + 'static,
{
    std::thread::spawn(move || {
        if let Some(mut stream) = stream {
            let mut buffer = [0_u8; 8192];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let event = if stdout {
                            CommandOutputEvent::Stdout(buffer[..count].to_vec())
                        } else {
                            CommandOutputEvent::Stderr(buffer[..count].to_vec())
                        };
                        if sender.send(event).is_err() {
                            return;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        let _ = sender.send(if stdout {
            CommandOutputEvent::StdoutDone
        } else {
            CommandOutputEvent::StderrDone
        });
    })
}

fn drain_command_output_events(
    receiver: &mpsc::Receiver<CommandOutputEvent>,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
    stdout_done: &mut bool,
    stderr_done: &mut bool,
) -> bool {
    let mut observed_output = false;
    while let Ok(event) = receiver.try_recv() {
        match event {
            CommandOutputEvent::Stdout(bytes) => {
                observed_output |= !bytes.is_empty();
                stdout.extend(bytes);
            }
            CommandOutputEvent::Stderr(bytes) => {
                observed_output |= !bytes.is_empty();
                stderr.extend(bytes);
            }
            CommandOutputEvent::StdoutDone => *stdout_done = true,
            CommandOutputEvent::StderrDone => *stderr_done = true,
        }
    }
    observed_output
}

#[cfg(windows)]
fn trusted_taskkill_path() -> Option<std::path::PathBuf> {
    std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .map(|system_root| system_root.join("System32").join("taskkill.exe"))
        .filter(|taskkill| taskkill.is_file())
}

#[cfg(windows)]
fn terminate_windows_process_tree(
    child: &mut std::process::Child,
    reason: &str,
) -> Result<(), String> {
    let pid = child.id();
    if let Some(taskkill) = trusted_taskkill_path() {
        if let Ok(status) = std::process::Command::new(taskkill)
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                return Ok(());
            }
        }
    }

    if child
        .try_wait()
        .map_err(|error| format!("failed to inspect {reason} process after taskkill: {error}"))?
        .is_some()
    {
        return Ok(());
    }

    child
        .kill()
        .map_err(|error| format!("failed to kill {reason} process tree for pid {pid}: {error}"))
}

fn execute_wrapped_command(
    mut process: std::process::Command,
    wrapped_command: &WrappedCommand,
    stdin_payload: Option<Vec<u8>>,
) -> Result<ObservedCommandOutput, String> {
    process
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if stdin_payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    #[cfg(unix)]
    if wrapped_command.timeout_wrapper.is_some() {
        process.process_group(0);
    }

    let mut child = process
        .spawn()
        .map_err(|error| format!("spawn failed for `{}`: {error}", wrapped_command.command))?;
    if let Some(bytes) = stdin_payload {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes).map_err(|error| {
                format!(
                    "failed to write stdin for `{}`: {error}",
                    wrapped_command.command
                )
            })?;
        }
    }
    #[cfg(unix)]
    let process_group_id = child.id();
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();
    let (output_tx, output_rx) = mpsc::channel();
    let _stdout_reader = spawn_reader_thread(child_stdout, output_tx.clone(), true);
    let _stderr_reader = spawn_reader_thread(child_stderr, output_tx, false);

    let mut status = None;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut timed_out = false;
    let started_at = Instant::now();
    let mut timeout_progress = wrapped_command.timeout_wrapper.as_ref().map(|wrapper| {
        TimeoutProgress::WaitingForDeadline(
            Instant::now() + Duration::from_secs(wrapper.timeout_seconds),
        )
    });
    let no_output_timeout_seconds = wrapped_command
        .timeout_wrapper
        .as_ref()
        .and_then(|wrapper| {
            wrapper
                .no_output_timeout_seconds
                .filter(|seconds| *seconds > 0)
        });
    let mut no_output_deadline =
        no_output_timeout_seconds.map(|seconds| started_at + Duration::from_secs(seconds));

    loop {
        if status.is_none() {
            status = child.try_wait().map_err(|error| {
                format!("failed to wait on `{}`: {error}", wrapped_command.command)
            })?;
        }
        if drain_command_output_events(
            &output_rx,
            &mut stdout,
            &mut stderr,
            &mut stdout_done,
            &mut stderr_done,
        ) {
            if let Some(seconds) = no_output_timeout_seconds {
                no_output_deadline = Some(Instant::now() + Duration::from_secs(seconds));
            }
        }

        if status.is_some() && stdout_done && stderr_done {
            return Ok(ObservedCommandOutput {
                status: status.expect("status checked above"),
                stdout,
                stderr,
                timed_out,
            });
        }
        if status.is_none() && no_output_deadline.is_some_and(|deadline| Instant::now() >= deadline)
        {
            #[cfg(unix)]
            signal_process_group(process_group_id, libc::SIGTERM)?;
            #[cfg(windows)]
            terminate_windows_process_tree(&mut child, "no-output")?;
            #[cfg(all(not(unix), not(windows)))]
            child
                .kill()
                .map_err(|error| format!("failed to kill no-output process: {error}"))?;
            timed_out = true;
            let kill_deadline = Instant::now()
                + Duration::from_secs(
                    wrapped_command
                        .timeout_wrapper
                        .as_ref()
                        .map(|wrapper| wrapper.kill_after_grace_seconds)
                        .unwrap_or_default(),
                );
            timeout_progress = Some(TimeoutProgress::WaitingForKill(kill_deadline));
            no_output_deadline = None;
        }
        match timeout_progress.take() {
            Some(TimeoutProgress::WaitingForDeadline(deadline)) => {
                if Instant::now() >= deadline {
                    #[cfg(unix)]
                    signal_process_group(process_group_id, libc::SIGTERM)?;
                    #[cfg(windows)]
                    terminate_windows_process_tree(&mut child, "timed out")?;
                    #[cfg(all(not(unix), not(windows)))]
                    child
                        .kill()
                        .map_err(|error| format!("failed to kill timed out process: {error}"))?;
                    timed_out = true;
                    let kill_deadline = Instant::now()
                        + Duration::from_secs(
                            wrapped_command
                                .timeout_wrapper
                                .as_ref()
                                .map(|wrapper| wrapper.kill_after_grace_seconds)
                                .unwrap_or_default(),
                        );
                    timeout_progress = Some(TimeoutProgress::WaitingForKill(kill_deadline));
                } else {
                    timeout_progress = Some(TimeoutProgress::WaitingForDeadline(deadline));
                }
            }
            Some(TimeoutProgress::WaitingForKill(kill_deadline)) => {
                if Instant::now() >= kill_deadline {
                    #[cfg(unix)]
                    signal_process_group(process_group_id, libc::SIGKILL)?;
                    timeout_progress = Some(TimeoutProgress::TimedOut);
                } else {
                    timeout_progress = Some(TimeoutProgress::WaitingForKill(kill_deadline));
                }
            }
            Some(TimeoutProgress::TimedOut) => {
                return Ok(ObservedCommandOutput {
                    status: synthetic_timeout_exit_status(),
                    stdout,
                    stderr,
                    timed_out: true,
                });
            }
            None => {}
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn internal_host_activation_only_blocker_code(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timed_out: bool,
) -> String {
    if timed_out {
        crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT.to_string()
    } else {
        crate::runtime_dispatch_state::internal_host_activation_view_only_blocker_code(
            project_root,
            role_selection,
            receipt,
        )
        .to_string()
    }
}

#[cfg(unix)]
fn synthetic_timeout_exit_status() -> ExitStatus {
    ExitStatus::from_raw(libc::SIGKILL)
}

#[cfg(not(unix))]
fn synthetic_timeout_exit_status() -> ExitStatus {
    synthetic_timeout_exit_status_non_unix()
}

#[cfg(windows)]
fn synthetic_timeout_exit_status_non_unix() -> ExitStatus {
    ExitStatus::from_raw(124)
}

#[cfg(all(not(unix), not(windows)))]
fn synthetic_timeout_exit_status_non_unix() -> ExitStatus {
    panic!("synthetic timeout exit status is unsupported on this platform")
}

async fn execute_wrapped_command_async(
    process: std::process::Command,
    wrapped_command: WrappedCommand,
    stdin_payload: Option<Vec<u8>>,
) -> Result<ObservedCommandOutput, String> {
    tokio::task::spawn_blocking(move || {
        execute_wrapped_command(process, &wrapped_command, stdin_payload)
    })
    .await
    .map_err(|error| format!("wrapped command task join failed: {error}"))?
}

fn wrap_command_with_optional_timeout(
    command: String,
    args: Vec<String>,
    timeout_seconds: Option<u64>,
) -> WrappedCommand {
    wrap_command_with_optional_timeouts(command, args, timeout_seconds, None)
}

fn wrap_command_with_optional_timeouts(
    command: String,
    args: Vec<String>,
    timeout_seconds: Option<u64>,
    no_output_timeout_seconds: Option<u64>,
) -> WrappedCommand {
    if let Some(timeout_seconds) = timeout_seconds.filter(|seconds| *seconds > 0) {
        let kill_after_grace_seconds =
            DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS.min(timeout_seconds.max(1));
        WrappedCommand {
            command,
            args,
            timeout_wrapper: Some(CommandTimeoutWrapper {
                timeout_seconds,
                kill_after_grace_seconds,
                no_output_timeout_seconds: no_output_timeout_seconds
                    .filter(|seconds| *seconds > 0)
                    .map(|seconds| seconds.min(timeout_seconds)),
            }),
        }
    } else {
        WrappedCommand {
            command,
            args,
            timeout_wrapper: None,
        }
    }
}

#[derive(Debug)]
struct ParsedExternalProviderOutput {
    raw_json: serde_json::Value,
    result_text: Option<String>,
    usage: Option<serde_json::Value>,
    is_error: Option<bool>,
    error_message: Option<String>,
}

fn external_provider_output_indicates_error(output: &ParsedExternalProviderOutput) -> bool {
    if output.is_error.unwrap_or(false) {
        return true;
    }

    if external_provider_scope_guard_indicates_violation(&output.raw_json) {
        return true;
    }

    if external_provider_result_text_declares_blocker(output) {
        return true;
    }

    if output
        .error_message
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return true;
    }

    if output.is_error == Some(false)
        && output
            .raw_json
            .pointer("/raw_provider/provider")
            .and_then(serde_json::Value::as_str)
            == Some("pi")
        && output
            .raw_json
            .pointer("/raw_provider/terminal_event")
            .and_then(serde_json::Value::as_str)
            == Some("agent_end")
    {
        return false;
    }

    let Some(result_text) = output.result_text.as_ref() else {
        return false;
    };

    let normalized = result_text.trim().to_ascii_lowercase();
    if normalized.starts_with('[') && normalized.ends_with(']') {
        return normalized.contains("error") || normalized.contains("exception");
    }

    [
        "quota exceeded",
        "daily quota has been reached",
        "oauth quota exceeded",
        "auth failure",
        "authentication failed",
        "unauthorized",
        "invalid access token",
        "token expired",
        "invalid api key",
        "rate limit exceeded",
        "too many requests",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn external_provider_result_text_declares_blocker(output: &ParsedExternalProviderOutput) -> bool {
    output
        .result_text
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|normalized| {
            normalized.contains("dispatch blocked")
                || normalized.contains("blocked by vida pi write-scope guard")
                || normalized.contains("no execution receipt")
                || normalized.contains("no execution receipt/result artifact")
                || normalized.contains("refused in bash guarded-write mode")
        })
}

fn external_provider_scope_guard_indicates_violation(raw_json: &serde_json::Value) -> bool {
    external_provider_scope_guard(raw_json).is_some_and(|scope_guard| {
        scope_guard
            .get("valid")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
            || scope_guard
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| {
                    matches!(
                        status.trim().to_ascii_lowercase().as_str(),
                        "violation"
                            | "scope_violation"
                            | "owned_path_invalid"
                            | "missing_owned_paths"
                    )
                })
    })
}

fn external_provider_scope_guard(raw_json: &serde_json::Value) -> Option<&serde_json::Value> {
    match raw_json {
        serde_json::Value::Object(_) => raw_json.get("scope_guard"),
        serde_json::Value::Array(rows) => rows.iter().rev().find_map(|row| row.get("scope_guard")),
        _ => None,
    }
}

fn external_provider_reported_paths(raw_json: &serde_json::Value) -> Option<serde_json::Value> {
    let mut touched_paths = std::collections::BTreeSet::new();
    let mut changed_files = std::collections::BTreeSet::new();
    collect_external_provider_reported_paths(raw_json, &mut touched_paths, &mut changed_files);
    let mut body = serde_json::Map::new();
    if !touched_paths.is_empty() {
        body.insert(
            "touched_paths".to_string(),
            serde_json::json!(touched_paths.into_iter().collect::<Vec<_>>()),
        );
    }
    if !changed_files.is_empty() {
        body.insert(
            "changed_files".to_string(),
            serde_json::json!(changed_files.into_iter().collect::<Vec<_>>()),
        );
    }
    if body.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(body))
    }
}

fn collect_external_provider_reported_paths(
    raw_json: &serde_json::Value,
    touched_paths: &mut std::collections::BTreeSet<String>,
    changed_files: &mut std::collections::BTreeSet<String>,
) {
    match raw_json {
        serde_json::Value::Object(entries) => {
            if let Some(paths) = entries.get("touched_paths") {
                collect_external_provider_path_values(paths, touched_paths);
            }
            if let Some(paths) = entries.get("changed_files") {
                collect_external_provider_path_values(paths, changed_files);
            }
            for value in entries.values() {
                collect_external_provider_reported_paths(value, touched_paths, changed_files);
            }
        }
        serde_json::Value::Array(rows) => {
            for row in rows {
                collect_external_provider_reported_paths(row, touched_paths, changed_files);
            }
        }
        _ => {}
    }
}

fn collect_external_provider_path_values(
    value: &serde_json::Value,
    paths: &mut std::collections::BTreeSet<String>,
) {
    match value {
        serde_json::Value::String(path) => {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                paths.insert(trimmed.to_string());
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_external_provider_path_values(value, paths);
            }
        }
        serde_json::Value::Object(entries) => {
            for key in ["path", "file", "filename"] {
                if let Some(path) = entries.get(key).and_then(serde_json::Value::as_str) {
                    let trimmed = path.trim();
                    if !trimmed.is_empty() {
                        paths.insert(trimmed.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn external_provider_output_confirms_execution(
    output: Option<&ParsedExternalProviderOutput>,
) -> bool {
    output.is_some_and(|parsed| !external_provider_output_indicates_error(parsed))
}

fn configured_external_dispatch_output_mode(backend_entry: &serde_yaml::Value) -> String {
    yaml_lookup(backend_entry, &["dispatch", "output_mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("vida_result_json")
        .to_string()
}

fn external_provider_output_confirms_execution_for_mode(
    output_mode: &str,
    stdout: &str,
    output: Option<&ParsedExternalProviderOutput>,
) -> bool {
    match output {
        Some(parsed) => !external_provider_output_indicates_error(parsed),
        None => output_mode == "stdout" && !stdout.trim().is_empty(),
    }
}

fn external_provider_error_message(output: &ParsedExternalProviderOutput) -> Option<String> {
    if output
        .error_message
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return output.error_message.clone();
    }

    if output
        .result_text
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return output.result_text.clone();
    }

    None
}

fn external_provider_output_indicates_agent_end_timeout(
    output: &ParsedExternalProviderOutput,
) -> bool {
    let provider_is_pi = output
        .raw_json
        .pointer("/raw_provider/provider")
        .and_then(serde_json::Value::as_str)
        == Some("pi");
    if !provider_is_pi {
        return false;
    }
    let returned_agent_end = output
        .raw_json
        .pointer("/raw_provider/terminal_event")
        .and_then(serde_json::Value::as_str)
        == Some("agent_end");
    if returned_agent_end {
        return false;
    }
    external_provider_error_message(output)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .is_some_and(|message| message.contains("timed out waiting for pi agent_end event"))
}

fn parse_external_provider_output(stdout: &str) -> Option<ParsedExternalProviderOutput> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    let raw_json = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        parsed
    } else {
        let parsed_lines = trimmed
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>();
        match parsed_lines {
            Ok(rows) if !rows.is_empty() => serde_json::Value::Array(rows),
            _ => return None,
        }
    };
    let result_row = match &raw_json {
        serde_json::Value::Array(rows) => rows
            .iter()
            .rev()
            .find(|row| row.get("type").and_then(serde_json::Value::as_str) == Some("result")),
        serde_json::Value::Object(_) => Some(&raw_json),
        _ => None,
    }?;
    Some(ParsedExternalProviderOutput {
        result_text: result_row
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        usage: result_row.get("usage").cloned(),
        is_error: result_row
            .get("is_error")
            .and_then(serde_json::Value::as_bool),
        error_message: result_row
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        raw_json,
    })
}

#[derive(Debug)]
struct ParsedInternalCodexOutput {
    raw_json: serde_json::Value,
    result_text: Option<String>,
    error_messages: Vec<String>,
}

fn parse_internal_codex_exec_output(stdout: &str) -> ParsedInternalCodexOutput {
    let mut rows = Vec::new();
    let mut result_text = None;
    let mut error_messages = Vec::new();

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match row.get("type").and_then(serde_json::Value::as_str) {
            Some("item.completed") => {
                if let Some(item) = row.get("item") {
                    match item.get("type").and_then(serde_json::Value::as_str) {
                        Some("agent_message") => {
                            if let Some(text) = item
                                .get("text")
                                .and_then(serde_json::Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                            {
                                result_text = Some(text.to_string());
                            }
                        }
                        Some("error") => {
                            if let Some(message) =
                                item.get("message").and_then(serde_json::Value::as_str)
                            {
                                push_internal_codex_error_message(&mut error_messages, message);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some("error") => {
                if let Some(message) = row.get("message").and_then(serde_json::Value::as_str) {
                    push_internal_codex_error_message(&mut error_messages, message);
                }
            }
            Some("turn.failed") => {
                if let Some(message) = row
                    .get("error")
                    .and_then(|value| value.get("message"))
                    .and_then(serde_json::Value::as_str)
                {
                    push_internal_codex_error_message(&mut error_messages, message);
                }
            }
            _ => {}
        }
        rows.push(row);
    }

    ParsedInternalCodexOutput {
        raw_json: serde_json::Value::Array(rows),
        result_text,
        error_messages,
    }
}

fn push_internal_codex_error_message(error_messages: &mut Vec<String>, message: &str) {
    let message = message.trim();
    if message.is_empty() || internal_codex_message_is_benign_warning(message) {
        return;
    }
    if !error_messages.iter().any(|existing| existing == message) {
        error_messages.push(message.to_string());
    }
}

fn internal_codex_message_is_benign_warning(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.starts_with("under-development features enabled:")
        || normalized.contains("to suppress this warning")
}

fn internal_codex_stderr_line_is_benign_warning(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with("WARN ") || line.contains(" WARN ") {
        return true;
    }
    let normalized = line.to_ascii_lowercase();
    (line.starts_with("ERROR ") || line.contains(" ERROR "))
        && (normalized.contains("failed to stat skills path")
            || normalized.contains("failed to read skills dir"))
}

fn internal_codex_stderr_is_benign_warning(stderr: &str) -> bool {
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .all(internal_codex_stderr_line_is_benign_warning)
}

fn internal_codex_output_confirms_execution(
    parsed_output: &ParsedInternalCodexOutput,
    stderr: &str,
    exit_success: bool,
) -> bool {
    let result_text_present = parsed_output
        .result_text
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let error_stream_allows_result = parsed_output
        .error_messages
        .iter()
        .all(|message| internal_codex_error_message_allows_agent_result(message))
        && internal_codex_stderr_allows_agent_result(stderr);

    exit_success
        && result_text_present
        && (parsed_output.error_messages.is_empty()
            && internal_codex_stderr_is_benign_warning(stderr)
            || error_stream_allows_result)
}

fn internal_codex_error_message_allows_agent_result(message: &str) -> bool {
    message.contains("windows sandbox: spawn setup refresh")
}

fn internal_codex_stderr_allows_agent_result(stderr: &str) -> bool {
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .all(|line| {
            internal_codex_stderr_line_is_benign_warning(line)
                || line.contains("windows sandbox: spawn setup refresh")
        })
}

fn internal_host_provider_failure_blocker_code(
    stderr: &str,
    error_messages: &[String],
) -> Option<&'static str> {
    let windows_sandbox_spawn_failed = stderr.contains("windows sandbox: spawn setup refresh")
        || error_messages
            .iter()
            .any(|message| message.contains("windows sandbox: spawn setup refresh"));
    if windows_sandbox_spawn_failed {
        return Some("internal_codex_windows_sandbox_unavailable");
    }
    let usage_limit_reached = error_messages.iter().any(|message| {
        let normalized = message.to_ascii_lowercase();
        normalized.contains("usage limit")
            || normalized.contains("quota exceeded")
            || normalized.contains("daily quota has been reached")
            || normalized.contains("rate limit exceeded")
            || normalized.contains("too many requests")
    });
    usage_limit_reached.then_some("provider_usage_limit_exceeded")
}

fn internal_host_provider_failure_blocker_reason(
    blocker_code: &str,
    fallback_reason: String,
) -> String {
    if blocker_code == "internal_codex_windows_sandbox_unavailable" {
        return "Internal host carrier reached its configured dispatch command, but the Windows sandbox failed while spawning worker shell commands. Retry with a configured backend/runtime profile whose sandbox is supported on this host, or route through a configured external CLI backend before claiming receipt-backed execution.".to_string();
    }
    fallback_reason
}

fn should_render_store_backed_activation_view_for_internal_failure(
    activation_only: bool,
    success: bool,
) -> bool {
    !activation_only || success
}

fn dispatch_packet_prompt(dispatch_packet_path: &str) -> String {
    std::fs::read_to_string(dispatch_packet_path)
        .ok()
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .map(|packet| {
            crate::runtime_dispatch_packet_text::runtime_packet_prompt_from_packet(
                &packet,
                dispatch_packet_path,
            )
        })
        .unwrap_or_else(|| {
            format!(
                "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
                dispatch_packet_path
            )
        })
}

fn configured_subagent_backend_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|entries| {
            entries.iter().find_map(|(key, value)| {
                (key.as_str()?.trim() == backend_id
                    && crate::yaml_bool(yaml_lookup(value, &["enabled"]), false))
                .then_some(value)
            })
        })
}

fn exact_model_profile_from_backend_entry(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    profile_id: &str,
) -> Option<serde_json::Value> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return None;
    }
    let fallback_rate = crate::yaml_string(yaml_lookup(backend_entry, &["budget_cost_units"]))
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            crate::yaml_string(yaml_lookup(backend_entry, &["normalized_cost_units"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .or_else(|| {
            crate::yaml_string(yaml_lookup(backend_entry, &["rate"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        });
    let fallback_runtime_roles =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["runtime_roles"]));
    let fallback_task_classes =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["task_classes"]));
    let projection = crate::model_profile_contract::normalize_profile_projection_from_yaml(
        backend_id,
        backend_entry,
        fallback_rate,
        &fallback_runtime_roles,
        &fallback_task_classes,
    );
    projection["model_profiles"]
        .get(profile_id)
        .cloned()
        .filter(|profile| !profile.is_null())
}

fn apply_internal_subagent_profile_overlay(
    carrier: &serde_json::Value,
    backend_id: &str,
    backend_entry: Option<&serde_yaml::Value>,
    profile_id: Option<&str>,
) -> serde_json::Value {
    let Some(profile_id) = profile_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return carrier.clone();
    };
    let Some(profile) = backend_entry
        .and_then(|entry| exact_model_profile_from_backend_entry(backend_id, entry, profile_id))
    else {
        return carrier.clone();
    };
    let mut patched = carrier.clone();
    let object = patched
        .as_object_mut()
        .expect("internal carrier row should serialize to an object");
    object.insert(
        "selected_model_profile_id".to_string(),
        serde_json::json!(profile_id),
    );
    object.insert(
        "internal_subagent_backend_id".to_string(),
        serde_json::json!(backend_id),
    );
    object.insert(
        "internal_subagent_model_profile_id".to_string(),
        serde_json::json!(profile_id),
    );
    for (target_key, profile_key) in [
        ("model", "model_ref"),
        ("selected_model_ref", "model_ref"),
        ("model_provider", "provider"),
        ("selected_model_provider", "provider"),
        ("selected_reasoning_effort", "reasoning_effort"),
        (
            "selected_plan_mode_reasoning_effort",
            "plan_mode_reasoning_effort",
        ),
        ("selected_sandbox_mode", "sandbox_mode"),
        ("normalized_cost_units", "normalized_cost_units"),
        ("speed_tier", "speed_tier"),
        ("quality_tier", "quality_tier"),
        ("write_scope", "write_scope"),
    ] {
        if profile[profile_key]
            .as_str()
            .is_some_and(|value| value.trim().is_empty())
        {
            continue;
        }
        if !profile[profile_key].is_null() {
            object.insert(target_key.to_string(), profile[profile_key].clone());
        }
    }
    if let Some(reasoning) = profile["reasoning_effort"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "model_reasoning_effort".to_string(),
            serde_json::json!(reasoning),
        );
    }
    if let Some(sandbox) = profile["sandbox_mode"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| internal_profile_sandbox_from_write_scope(profile["write_scope"].as_str()))
    {
        object.insert(
            "selected_sandbox_mode".to_string(),
            serde_json::json!(sandbox),
        );
        object.insert("sandbox_mode".to_string(), serde_json::json!(sandbox));
    }
    patched
}

fn internal_profile_sandbox_from_write_scope(write_scope: Option<&str>) -> Option<&'static str> {
    match write_scope?.trim() {
        "orchestrator_native" | "workspace-write" | "scoped_write" => Some("workspace-write"),
        "read-only" | "read_or_review" | "none" => Some("read-only"),
        _ => None,
    }
}

fn selected_internal_host_carrier(
    selected_cli_entry: Option<&serde_yaml::Value>,
    preferred_backend: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
) -> Option<serde_json::Value> {
    let carriers =
        crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(selected_cli_entry);
    let find_carrier = |candidate_id: &str| {
        carriers
            .iter()
            .find(|row| row["role_id"].as_str() == Some(candidate_id))
            .cloned()
    };
    let effective_backend = preferred_backend.or(receipt.selected_backend.as_deref());
    let preferred_profile_id =
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            effective_backend,
        );

    let direct_ids = [preferred_backend, receipt.selected_backend.as_deref()];
    for candidate_id in direct_ids.into_iter().flatten() {
        if let Some(carrier) = find_carrier(candidate_id) {
            return Some(
                crate::model_profile_contract::apply_selected_model_profile_to_row(
                    &carrier,
                    preferred_profile_id.as_deref(),
                ),
            );
        }
    }

    let prefers_internal_backend = direct_ids
        .into_iter()
        .flatten()
        .any(|backend_id| backend_is_internal_host_bridge(role_selection, overlay, backend_id));
    if !prefers_internal_backend {
        return None;
    }

    let internal_backend_id = effective_backend?;
    let internal_bridge_ids = [
        receipt.activation_agent_type.as_deref(),
        role_selection
            .execution_plan
            .get("runtime_assignment")
            .and_then(|value| value.get("activation_agent_type"))
            .and_then(serde_json::Value::as_str),
        role_selection
            .execution_plan
            .get("runtime_assignment")
            .and_then(|value| value.get("selected_tier"))
            .and_then(serde_json::Value::as_str),
        Some(role_selection.selected_role.as_str()),
    ];
    let selected_backend_entry =
        overlay.and_then(|overlay| configured_subagent_backend_entry(overlay, internal_backend_id));
    internal_bridge_ids
        .into_iter()
        .flatten()
        .find_map(find_carrier)
        .map(|carrier| {
            let host_profile_carrier =
                crate::model_profile_contract::apply_selected_model_profile_to_row(
                    &carrier,
                    preferred_profile_id.as_deref(),
                );
            apply_internal_subagent_profile_overlay(
                &host_profile_carrier,
                internal_backend_id,
                selected_backend_entry,
                preferred_profile_id.as_deref(),
            )
        })
}

fn configured_internal_host_runtime_env(
    project_root: &Path,
    selected_cli_system: &str,
    carrier_id: &str,
) -> Result<Vec<(String, String)>, String> {
    let runtime_root = project_root
        .join(".vida")
        .join("data")
        .join("internal-host")
        .join(selected_cli_system)
        .join(carrier_id);
    let xdg_config_home = runtime_root.join("config");
    let xdg_data_home = runtime_root.join("data");
    let xdg_state_home = runtime_root.join("state");
    let xdg_cache_home = runtime_root.join("cache");
    let tmpdir = runtime_root.join("tmp");
    for dir in [
        &xdg_config_home,
        &xdg_data_home,
        &xdg_state_home,
        &xdg_cache_home,
        &tmpdir,
    ] {
        std::fs::create_dir_all(dir).map_err(|error| {
            format!(
                "Failed to prepare internal host runtime dir `{}`: {error}",
                dir.display()
            )
        })?;
    }

    Ok(vec![
        (
            "XDG_CONFIG_HOME".to_string(),
            xdg_config_home.display().to_string(),
        ),
        (
            "XDG_DATA_HOME".to_string(),
            xdg_data_home.display().to_string(),
        ),
        (
            "XDG_STATE_HOME".to_string(),
            xdg_state_home.display().to_string(),
        ),
        (
            "XDG_CACHE_HOME".to_string(),
            xdg_cache_home.display().to_string(),
        ),
        ("TMPDIR".to_string(), tmpdir.display().to_string()),
    ])
}

fn configured_internal_host_model_arg(dispatch: &serde_yaml::Value, model: &str) -> String {
    match crate::yaml_string(yaml_lookup(dispatch, &["model_arg_transform"]))
        .as_deref()
        .map(str::trim)
    {
        Some("provider_local_name") => model.rsplit('/').next().unwrap_or(model).to_string(),
        _ => model.to_string(),
    }
}

fn configured_host_execution_boundary(system_entry: Option<&serde_yaml::Value>) -> String {
    system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["execution_boundary"])))
        .unwrap_or_else(|| "parent_host_session".to_string())
}

fn configured_host_dispatch_transport(system_entry: Option<&serde_yaml::Value>) -> String {
    if let Some(transport) = system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["dispatch_transport"])))
    {
        return transport;
    }
    if system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["execution_class"])))
        .as_deref()
        == Some("internal")
    {
        return "host_tool_bridge".to_string();
    }
    if system_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch", "command"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return "codex_cli_exec".to_string();
    }
    "host_tool_bridge".to_string()
}

fn configured_host_receipt_mode(system_entry: Option<&serde_yaml::Value>) -> String {
    system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["receipt_mode"])))
        .unwrap_or_else(|| "host_bridge_receipt".to_string())
}

fn path_has_dot_segment(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    })
}

struct HostToolBridgeArtifactPaths {
    request_path: PathBuf,
    result_path: PathBuf,
    receipt_path: PathBuf,
}

fn host_tool_bridge_artifact_paths(
    project_root: &Path,
    state_root: &Path,
    system_entry: Option<&serde_yaml::Value>,
    request_id: &str,
) -> HostToolBridgeArtifactPaths {
    let request_dir = configured_host_tool_bridge_dir(
        project_root,
        state_root,
        system_entry,
        "request_dir",
        "host-tool-bridge/requests",
    );
    let result_dir = configured_host_tool_bridge_dir(
        project_root,
        state_root,
        system_entry,
        "result_dir",
        "host-tool-bridge/results",
    );
    let receipt_dir = configured_host_tool_bridge_dir(
        project_root,
        state_root,
        system_entry,
        "receipt_dir",
        "host-tool-bridge/receipts",
    );
    HostToolBridgeArtifactPaths {
        request_path: request_dir.join(format!("{request_id}.json")),
        result_path: result_dir.join(format!("{request_id}.json")),
        receipt_path: receipt_dir.join(format!("{request_id}.json")),
    }
}

fn configured_host_tool_bridge_dir(
    project_root: &Path,
    state_root: &Path,
    system_entry: Option<&serde_yaml::Value>,
    key: &str,
    fallback: &str,
) -> PathBuf {
    let configured = system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["host_tool_bridge", key])))
        .unwrap_or_else(|| fallback.to_string());
    let fallback_path = state_root.join(fallback);
    let path = PathBuf::from(configured);
    let candidate = if path.is_absolute() {
        path
    } else if let Ok(relative_to_state) = path.strip_prefix(".vida/data/state") {
        state_root.join(relative_to_state)
    } else {
        project_root.join(path)
    };
    if !path_has_dot_segment(&candidate) && candidate.starts_with(state_root) {
        candidate
    } else {
        fallback_path
    }
}

fn write_host_bridge_request_file(
    path: &Path,
    request: &serde_json::Value,
    replace_existing: bool,
    state_root: &Path,
) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Host bridge request path `{}` has no parent directory.",
            path.display()
        )
    })?;
    if path_has_dot_segment(path) {
        return Err(format!(
            "Host bridge request path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create host bridge request directory: {error}"))?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
        format!(
            "Failed to canonicalize host bridge request directory `{}`: {error}",
            parent.display()
        )
    })?;
    let canonical_state_root = std::fs::canonicalize(state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if !canonical_parent.starts_with(canonical_state_root) {
        return Err(format!(
            "Host bridge request directory `{}` escapes VIDA state root.",
            canonical_parent.display()
        ));
    }
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Host bridge request path `{}` is a symlink; refusing to follow it.",
                path.display()
            ));
        }
        if replace_existing {
            std::fs::remove_file(path).map_err(|error| {
                format!(
                    "Failed to remove existing host bridge request `{}` before replacement: {error}",
                    path.display()
                )
            })?;
        }
    }
    let encoded = serde_json::to_string_pretty(request)
        .map_err(|error| format!("Failed to encode host bridge request: {error}"))?;
    let mut open_options = std::fs::OpenOptions::new();
    open_options.write(true).create_new(true);
    let mut file = open_options.open(path).map_err(|error| {
        format!(
            "Failed to create host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    file.write_all(encoded.as_bytes()).map_err(|error| {
        format!(
            "Failed to write host bridge request `{}`: {error}",
            path.display()
        )
    })
}

fn ensure_host_bridge_state_parent(path: &Path, label: &str) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Host bridge {label} path `{}` has no parent directory.",
            path.display()
        )
    })?;
    let Some(state_root) = path
        .ancestors()
        .find(|ancestor| ancestor.ends_with(".vida/data/state"))
        .map(Path::to_path_buf)
    else {
        return Err(format!(
            "Host bridge {label} path `{}` is not under VIDA state root.",
            path.display()
        ));
    };
    let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
        format!(
            "Failed to canonicalize host bridge {label} directory `{}`: {error}",
            parent.display()
        )
    })?;
    let canonical_state_root = std::fs::canonicalize(&state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if !canonical_parent.starts_with(&canonical_state_root) {
        return Err(format!(
            "Host bridge {label} directory `{}` escapes VIDA state root.",
            canonical_parent.display()
        ));
    }
    Ok(())
}

fn remove_stale_host_bridge_artifact(path: &Path, label: &str) -> Result<(), String> {
    if std::fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    ensure_host_bridge_state_parent(path, label)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Failed to remove stale host bridge {label} `{}`: {error}",
            path.display()
        )),
    }
}

fn read_existing_host_bridge_request(path: &Path) -> Result<Option<serde_json::Value>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Host bridge request path `{}` is a symlink; refusing to read it.",
            path.display()
        ));
    }
    let raw = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read existing host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map(Some).map_err(|error| {
        format!(
            "Failed to decode existing host bridge request `{}`: {error}",
            path.display()
        )
    })
}

fn configured_host_tool_bridge_string(
    system_entry: Option<&serde_yaml::Value>,
    key: &str,
) -> Option<String> {
    system_entry
        .and_then(|entry| crate::yaml_string(yaml_lookup(entry, &["host_tool_bridge", key])))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| default_codex_host_tool_bridge_string(system_entry, key))
}

fn default_codex_host_tool_bridge_string(
    system_entry: Option<&serde_yaml::Value>,
    key: &str,
) -> Option<String> {
    let entry = system_entry?;
    if configured_host_dispatch_transport(Some(entry)) != "host_tool_bridge" {
        return None;
    }
    let execution_class = crate::yaml_string(yaml_lookup(entry, &["execution_class"]));
    let dispatch_command = yaml_lookup(entry, &["dispatch", "command"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim);
    if execution_class.as_deref() != Some("internal") || dispatch_command != Some("codex") {
        return None;
    }
    match key {
        "adapter_kind" => Some("codex_host_tools".to_string()),
        "adapter_capability_id" => Some("codex.multi_agent_v1".to_string()),
        "invocation_mode" => Some("parent_host_tool_api".to_string()),
        "tool_family" => Some("codex_multi_agent".to_string()),
        "spawn_tool" => Some("multi_agent_v1.spawn_agent".to_string()),
        "wait_tool" => Some("multi_agent_v1.wait_agent".to_string()),
        "close_tool" => Some("multi_agent_v1.close_agent".to_string()),
        "receipt_mode" => Some("host_bridge_receipt".to_string()),
        _ => None,
    }
}

fn dispatch_packet_string_list(dispatch_packet_path: &str, field: &str) -> Vec<String> {
    std::fs::read_to_string(dispatch_packet_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|packet| {
            packet
                .get(field)
                .or_else(|| {
                    packet
                        .get("delivery_task_packet")
                        .and_then(|value| value.get(field))
                })
                .cloned()
        })
        .and_then(|value| {
            value.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default()
}

fn dispatch_packet_string_field(dispatch_packet_path: &str, field: &str) -> Option<String> {
    std::fs::read_to_string(dispatch_packet_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|packet| {
            packet
                .get(field)
                .or_else(|| {
                    packet
                        .get("delivery_task_packet")
                        .and_then(|value| value.get(field))
                })
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn dispatch_packet_handoff_runtime_role(dispatch_packet_path: &str) -> Option<String> {
    dispatch_packet_string_field(dispatch_packet_path, "handoff_runtime_role")
        .or_else(|| dispatch_packet_string_field(dispatch_packet_path, "activation_runtime_role"))
        .or_else(|| dispatch_packet_string_field(dispatch_packet_path, "runtime_role"))
}

fn dispatch_packet_handoff_task_class(dispatch_packet_path: &str) -> Option<String> {
    dispatch_packet_string_field(dispatch_packet_path, "handoff_task_class")
        .or_else(|| dispatch_packet_string_field(dispatch_packet_path, "task_class"))
        .or_else(|| dispatch_packet_string_field(dispatch_packet_path, "route_task_class"))
}

fn dispatch_packet_value_field(
    dispatch_packet_path: &str,
    field: &str,
) -> Option<serde_json::Value> {
    std::fs::read_to_string(dispatch_packet_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|packet| {
            packet.get(field).cloned().or_else(|| {
                packet
                    .get("delivery_task_packet")
                    .and_then(|value| value.get(field))
                    .cloned()
            })
        })
        .filter(|value| !value.is_null())
}

fn add_proof_artifact_scope_to_implementation_isolation(
    implementation_isolation: &mut serde_json::Value,
    proof_artifact_paths: &[String],
) {
    if proof_artifact_paths.is_empty() || implementation_isolation.is_null() {
        return;
    }
    let Some(object) = implementation_isolation.as_object_mut() else {
        return;
    };
    object.insert(
        "proof_artifact_paths".to_string(),
        serde_json::json!(proof_artifact_paths),
    );
    object.insert(
        "proof_artifact_scope".to_string(),
        serde_json::json!(proof_artifact_paths),
    );
    let scope_policy = object
        .entry("scope_policy".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if let Some(scope_policy) = scope_policy.as_object_mut() {
        scope_policy.insert(
            "changed_files_must_be_subset_of_owned_or_proof_paths".to_string(),
            serde_json::json!(true),
        );
        scope_policy.insert(
            "patch_paths_must_be_subset_of_owned_or_proof_paths".to_string(),
            serde_json::json!(true),
        );
    }
}

fn configured_lane_rework_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Option<String> {
    crate::dispatch_contract_lane(&role_selection.execution_plan, dispatch_target)?
        .get("rework_transitions")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|transitions| transitions.values())
        .find_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(str::to_string)
}

fn host_bridge_blocked_result_contract(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> serde_json::Value {
    let rework_target = configured_lane_rework_target(role_selection, dispatch_target);
    serde_json::json!({
        "execution_state": "blocked",
        "decision": "rework_required",
        "verdict": "rework_required",
        "required_result_fields": default_host_bridge_required_result_fields(),
        "rework_target_required_when_blocked": true,
        "rework_target": rework_target,
        "allowed_next_node": rework_target,
        "blocker_codes": [
            "host_agent_capacity_unavailable",
            "host_tool_capability_missing",
            "host_agent_execution_failed"
        ],
        "allowed_blocker_codes": [
            "host_agent_capacity_unavailable",
            "host_tool_capability_missing",
            "host_agent_execution_failed"
        ]
    })
}

fn host_tool_bridge_request_id_segment(value: &str) -> String {
    let mut segment = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while segment.contains("--") {
        segment = segment.replace("--", "-");
    }
    segment
        .trim_matches('-')
        .chars()
        .take(96)
        .collect::<String>()
}

fn host_tool_bridge_request_id(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: &str,
) -> String {
    let run_segment = host_tool_bridge_request_id_segment(&receipt.run_id);
    let dispatch_target_segment = host_tool_bridge_request_id_segment(&receipt.dispatch_target);
    let packet_segment = Path::new(dispatch_packet_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(host_tool_bridge_request_id_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "packet".to_string());
    format!(
        "{}-{}-{}-host-tool-bridge",
        if run_segment.is_empty() {
            "run"
        } else {
            run_segment.as_str()
        },
        if dispatch_target_segment.is_empty() {
            "dispatch-target"
        } else {
            dispatch_target_segment.as_str()
        },
        packet_segment
    )
}

fn host_bridge_request_value_matches(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
    field: &str,
) -> bool {
    if matches!(
        field,
        "packet_path" | "request_path" | "result_path" | "receipt_path"
    ) {
        if existing.get(field) == expected.get(field) {
            return true;
        }
        return match (
            existing.get(field).and_then(serde_json::Value::as_str),
            expected.get(field).and_then(serde_json::Value::as_str),
        ) {
            (Some(existing), Some(expected)) => {
                let normalize = |path: &str| {
                    path.replace('\\', "/")
                        .trim_end_matches('/')
                        .to_ascii_lowercase()
                };
                normalize(existing) == normalize(expected)
                    || taskflow_core::runtime_packet_identity::runtime_packet_paths_equivalent(
                        existing, expected,
                    )
            }
            _ => existing.get(field) == expected.get(field),
        };
    }
    existing.get(field) == expected.get(field)
}

fn validate_existing_host_bridge_request_matches_expected(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
    request_path: &Path,
) -> Result<(), String> {
    let status = existing
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !host_bridge_existing_request_status_is_admissible(status) {
        return Err(format!(
            "Existing host bridge request `{}` has inadmissible status `{status}`.",
            request_path.display()
        ));
    }
    for field in [
        "schema_version",
        "request_id",
        "run_id",
        "task_id",
        "dispatch_target",
        "packet_path",
        "runtime_role",
        "task_class",
        "backend_id",
        "carrier_id",
        "execution_boundary",
        "dispatch_transport",
        "receipt_mode",
        "adapter_kind",
        "adapter_capability_id",
        "invocation_mode",
        "adapter_params",
        "implementation_isolation",
        "expected_implementation_artifact_kinds",
        "owned_paths",
        "proof_artifact_paths",
        "proof_artifact_scope",
        "read_only_paths",
        "proof_target",
        "request_path",
        "result_path",
        "receipt_path",
        "required_result_fields",
    ] {
        if !host_bridge_request_value_matches(existing, expected, field) {
            return Err(format!(
                "Existing host bridge request `{}` does not match expected `{field}` for the active dispatch lane.",
                request_path.display()
            ));
        }
    }
    Ok(())
}

fn existing_host_bridge_request_has_retryable_completion_evidence(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
) -> bool {
    let status = existing
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !matches!(status, "blocked" | "retryable_blocked") {
        return false;
    }

    ["result_path", "receipt_path"].iter().any(|field| {
        let paths_match = host_bridge_request_value_matches(existing, expected, field);
        if !paths_match {
            return false;
        }
        let Some(path) = existing
            .get(*field)
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
        else {
            return false;
        };
        let artifact = crate::read_json_file_if_present(&path);
        let retryable = artifact
            .as_ref()
            .is_some_and(|artifact| host_bridge_artifact_has_retryable_completion_blocker(artifact));
        retryable
    })
}

fn existing_host_bridge_request_needs_retryable_blocked_refresh(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
) -> bool {
    let status = existing
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    matches!(status, "blocked" | "retryable_blocked")
        && existing_host_bridge_request_has_retryable_completion_evidence(existing, expected)
}

fn validate_existing_host_bridge_request_identity_matches_expected(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
    request_path: &Path,
) -> Result<(), String> {
    let status = existing
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !host_bridge_existing_request_status_is_admissible(status)
        && !existing_host_bridge_request_has_retryable_completion_evidence(existing, expected)
    {
        return Err(format!(
            "Existing host bridge request `{}` has inadmissible status `{status}`.",
            request_path.display()
        ));
    }
    for field in [
        "schema_version",
        "request_id",
        "run_id",
        "task_id",
        "dispatch_target",
        "packet_path",
        "runtime_role",
        "task_class",
        "backend_id",
        "carrier_id",
        "execution_boundary",
        "dispatch_transport",
        "request_path",
        "result_path",
        "receipt_path",
    ] {
        if !host_bridge_request_value_matches(existing, expected, field) {
            return Err(format!(
                "Existing host bridge request `{}` does not match expected `{field}` for the active dispatch lane.",
                request_path.display()
            ));
        }
    }
    Ok(())
}

fn existing_host_bridge_request_needs_adapter_refresh(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
) -> bool {
    let legacy_unconfigured = existing
        .get("adapter_kind")
        .and_then(serde_json::Value::as_str)
        == Some("unconfigured_host_agent_adapter")
        || existing
            .get("adapter_capability_id")
            .and_then(serde_json::Value::as_str)
            == Some("unconfigured_host_agent_capability")
        || existing
            .get("invocation_mode")
            .and_then(serde_json::Value::as_str)
            == Some("configured_host_capability_required");
    let expected_configured = expected
        .get("adapter_kind")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| value != "unconfigured_host_agent_adapter")
        && expected
            .get("adapter_capability_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value != "unconfigured_host_agent_capability")
        && expected
            .get("invocation_mode")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value != "configured_host_capability_required");
    legacy_unconfigured
        && expected_configured
        && [
            "receipt_mode",
            "adapter_kind",
            "adapter_capability_id",
            "invocation_mode",
            "adapter_params",
        ]
        .iter()
        .any(|field| !host_bridge_request_value_matches(existing, expected, field))
}

fn existing_host_bridge_request_needs_pending_contract_refresh(
    existing: &serde_json::Value,
    expected: &serde_json::Value,
) -> bool {
    if existing
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        != "pending"
    {
        return false;
    }
    if [
        "schema_version",
        "request_id",
        "run_id",
        "task_id",
        "dispatch_target",
        "packet_path",
        "backend_id",
        "carrier_id",
        "execution_boundary",
        "dispatch_transport",
    ]
    .iter()
    .any(|field| !host_bridge_request_value_matches(existing, expected, field))
    {
        return false;
    }
    [
        "runtime_role",
        "task_class",
        "implementation_isolation",
        "expected_implementation_artifact_kinds",
        "owned_paths",
        "proof_artifact_paths",
        "proof_artifact_scope",
    ]
    .iter()
    .any(|field| !host_bridge_request_value_matches(existing, expected, field))
}

fn materialize_host_tool_bridge_request(
    project_root: &Path,
    state_root: &Path,
    selected_cli_entry: Option<&serde_yaml::Value>,
    dispatch_packet_path: &str,
    backend_id: &str,
    carrier_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Result<serde_json::Value, String> {
    let request_id = host_tool_bridge_request_id(receipt, dispatch_packet_path);
    let paths =
        host_tool_bridge_artifact_paths(project_root, state_root, selected_cli_entry, &request_id);
    let adapter_kind = configured_host_tool_bridge_string(selected_cli_entry, "adapter_kind")
        .unwrap_or_else(|| "unconfigured_host_agent_adapter".to_string());
    let adapter_capability_id =
        configured_host_tool_bridge_string(selected_cli_entry, "adapter_capability_id")
            .unwrap_or_else(|| "unconfigured_host_agent_capability".to_string());
    let invocation_mode = configured_host_tool_bridge_string(selected_cli_entry, "invocation_mode")
        .unwrap_or_else(|| "configured_host_capability_required".to_string());
    let receipt_mode = configured_host_tool_bridge_string(selected_cli_entry, "receipt_mode")
        .unwrap_or_else(|| configured_host_receipt_mode(selected_cli_entry));
    let adapter_params = selected_cli_entry
        .and_then(|entry| yaml_lookup(entry, &["host_tool_bridge", "adapter_params"]))
        .and_then(|params| serde_json::to_value(params).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "tool_family": configured_host_tool_bridge_string(selected_cli_entry, "tool_family"),
                "spawn_tool": configured_host_tool_bridge_string(selected_cli_entry, "spawn_tool"),
                "wait_tool": configured_host_tool_bridge_string(selected_cli_entry, "wait_tool"),
                "close_tool": configured_host_tool_bridge_string(selected_cli_entry, "close_tool"),
            })
        });
    let configured_runtime_role =
        crate::runtime_dispatch_downstream_packets::configured_lane_runtime_role(
            role_selection,
            &receipt.dispatch_target,
        );
    let request_runtime_role = configured_runtime_role
        .clone()
        .or_else(|| dispatch_packet_handoff_runtime_role(dispatch_packet_path))
        .or_else(|| receipt.activation_runtime_role.clone());
    let request_task_class = configured_runtime_role
        .as_deref()
        .map(|runtime_role| {
            crate::runtime_dispatch_state::runtime_packet_handoff_task_class_for_plan(
                &role_selection.execution_plan,
                &receipt.dispatch_target,
                runtime_role,
            )
        })
        .filter(|task_class| !task_class.trim().is_empty())
        .or_else(|| dispatch_packet_handoff_task_class(dispatch_packet_path))
        .unwrap_or_else(|| canonical_dispatch_target_for_admissibility(&receipt.dispatch_target));
    let mut request_owned_paths = dispatch_packet_string_list(dispatch_packet_path, "owned_paths");
    if crate::runtime_dispatch_downstream_packets::test_lane_requires_test_write_scope(
        &receipt.dispatch_target,
        None,
        &request_task_class,
    ) {
        for test_path in
            crate::runtime_dispatch_downstream_packets::project_test_write_scope_paths(project_root)
        {
            if !request_owned_paths.iter().any(|path| path == &test_path) {
                request_owned_paths.push(test_path);
            }
        }
    }
    let recomputed_implementation_isolation =
        crate::runtime_dispatch_packets::implementation_isolation_contract(
            &request_task_class,
            &request_owned_paths,
        );
    let mut implementation_isolation = if recomputed_implementation_isolation.is_null() {
        dispatch_packet_value_field(dispatch_packet_path, "implementation_isolation")
            .unwrap_or(serde_json::Value::Null)
    } else {
        recomputed_implementation_isolation
    };
    let proof_artifact_paths = proof_scope_from_dispatch_packet_path(dispatch_packet_path).paths;
    add_proof_artifact_scope_to_implementation_isolation(
        &mut implementation_isolation,
        &proof_artifact_paths,
    );
    let expected_implementation_artifact_kinds = if implementation_isolation.is_null() {
        serde_json::json!([])
    } else {
        serde_json::json!(["patch_proposal", "isolated_worktree_manifest"])
    };
    let request = serde_json::json!({
        "schema_version": 1,
        "status": "pending",
        "request_id": request_id,
        "run_id": receipt.run_id,
        "task_id": receipt.run_id,
        "dispatch_target": receipt.dispatch_target,
        "packet_path": dispatch_packet_path,
        "runtime_role": request_runtime_role,
        "task_class": request_task_class,
        "backend_id": backend_id,
        "carrier_id": carrier_id,
        "execution_boundary": "parent_host_session",
        "dispatch_transport": "host_tool_bridge",
        "receipt_mode": receipt_mode,
        "adapter_kind": adapter_kind,
        "adapter_capability_id": adapter_capability_id,
        "invocation_mode": invocation_mode,
        "adapter_params": adapter_params,
        "implementation_isolation": implementation_isolation,
        "expected_implementation_artifact_kinds": expected_implementation_artifact_kinds,
        "implementation_artifacts": [],
        "required_result_fields": default_host_bridge_required_result_fields(),
        "blocked_result_contract": host_bridge_blocked_result_contract(
            role_selection,
            &receipt.dispatch_target,
        ),
        "owned_paths": request_owned_paths,
        "proof_artifact_paths": proof_artifact_paths,
        "proof_artifact_scope": proof_artifact_paths,
        "read_only_paths": dispatch_packet_string_list(dispatch_packet_path, "read_only_paths"),
        "proof_target": dispatch_packet_string_field(dispatch_packet_path, "proof_target"),
        "request_path": paths.request_path.display().to_string(),
        "result_path": paths.result_path.display().to_string(),
        "receipt_path": paths.receipt_path.display().to_string(),
    });
    let mut replace_existing_request = false;
    if let Some(existing) = read_existing_host_bridge_request(&paths.request_path)? {
        if existing.get("run_id").and_then(serde_json::Value::as_str)
            != Some(receipt.run_id.as_str())
            || existing
                .get("dispatch_target")
                .and_then(serde_json::Value::as_str)
                != Some(receipt.dispatch_target.as_str())
            || existing
                .get("dispatch_transport")
                .and_then(serde_json::Value::as_str)
                != Some("host_tool_bridge")
        {
            return Err(format!(
                "Existing host bridge request `{}` does not match the active dispatch lane.",
                paths.request_path.display()
            ));
        }
        if existing
            .get("packet_path")
            .and_then(serde_json::Value::as_str)
            == Some(dispatch_packet_path)
        {
            if existing_host_bridge_request_needs_retryable_blocked_refresh(&existing, &request) {
                validate_existing_host_bridge_request_identity_matches_expected(
                    &existing,
                    &request,
                    &paths.request_path,
                )?;
                replace_existing_request = true;
            } else if existing_host_bridge_request_needs_pending_contract_refresh(&existing, &request) {
                replace_existing_request = true;
            } else if existing_host_bridge_request_needs_adapter_refresh(&existing, &request) {
                validate_existing_host_bridge_request_identity_matches_expected(
                    &existing,
                    &request,
                    &paths.request_path,
                )?;
                replace_existing_request = true;
            } else {
                validate_existing_host_bridge_request_matches_expected(
                    &existing,
                    &request,
                    &paths.request_path,
                )?;
                return Ok(existing);
            }
        } else {
            replace_existing_request = true;
        }
    }
    if replace_existing_request {
        remove_stale_host_bridge_artifact(&paths.result_path, "result")?;
        remove_stale_host_bridge_artifact(&paths.receipt_path, "receipt")?;
    }
    if let Some(parent) = paths.request_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create host bridge request directory: {error}"))?;
    }
    if let Some(parent) = paths.result_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create host bridge result directory: {error}"))?;
    }
    if let Some(parent) = paths.receipt_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create host bridge receipt directory: {error}"))?;
    }
    write_host_bridge_request_file(
        &paths.request_path,
        &request,
        replace_existing_request,
        state_root,
    )?;
    Ok(request)
}

fn compact_json_object_fields(source: &serde_json::Value, fields: &[&str]) -> serde_json::Value {
    let mut compact = serde_json::Map::new();
    for field in fields {
        if let Some(value) = source.get(*field) {
            if !value.is_null() {
                compact.insert((*field).to_string(), value.clone());
            }
        }
    }
    serde_json::Value::Object(compact)
}

fn compact_host_tool_bridge_request_for_dispatch_result(
    request: &serde_json::Value,
) -> serde_json::Value {
    let mut compact = compact_json_object_fields(
        request,
        &[
            "schema_version",
            "status",
            "request_id",
            "run_id",
            "task_id",
            "dispatch_target",
            "packet_path",
            "runtime_role",
            "task_class",
            "backend_id",
            "carrier_id",
            "execution_boundary",
            "dispatch_transport",
            "receipt_mode",
            "adapter_kind",
            "adapter_capability_id",
            "invocation_mode",
            "request_path",
            "result_path",
            "receipt_path",
        ],
    );
    let object = compact
        .as_object_mut()
        .expect("compact host bridge request should be an object");
    object.insert("compact_projection".to_string(), serde_json::json!(true));
    if let Some(count) = request
        .get("owned_paths")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
    {
        object.insert("owned_paths_count".to_string(), serde_json::json!(count));
    }
    if let Some(request_path) = request
        .get("request_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "full_request_artifact".to_string(),
            serde_json::json!({
                "kind": "host_tool_bridge_request",
                "path": request_path,
                "reason": "dispatch result stores compact projection; parent host adapter reads the full request artifact"
            }),
        );
    }
    compact
}

fn compact_role_selection_for_dispatch_result(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> serde_json::Value {
    serde_json::json!({
        "ok": role_selection.ok,
        "activation_source": &role_selection.activation_source,
        "selection_mode": &role_selection.selection_mode,
        "request": &role_selection.request,
        "selected_role": &role_selection.selected_role,
        "single_task_only": role_selection.single_task_only,
        "tracked_flow_entry": &role_selection.tracked_flow_entry,
        "confidence": &role_selection.confidence,
        "run_id": &receipt.run_id,
        "dispatch_target": &receipt.dispatch_target,
        "selected_backend": &receipt.selected_backend,
        "activation_runtime_role": &receipt.activation_runtime_role,
        "activation_agent_type": &receipt.activation_agent_type,
    })
}

fn compact_pending_host_bridge_dispatch_result(
    body: &mut serde_json::Map<String, serde_json::Value>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    bridge_request: &serde_json::Value,
) {
    let mut omitted_fields = Vec::new();
    for field in [
        "selection",
        "role_selection",
        "dev_team_readiness",
        "init",
        "backend_truth",
    ] {
        if body.remove(field).is_some() {
            omitted_fields.push(field);
        }
    }
    body.insert(
        "role_selection_summary".to_string(),
        compact_role_selection_for_dispatch_result(role_selection, receipt),
    );
    if !omitted_fields.is_empty() {
        body.insert(
            "omitted_heavy_fields".to_string(),
            serde_json::json!({
                "reason": "compact_default_dispatch_result",
                "fields": omitted_fields,
                "artifact_refs": {
                    "dispatch_packet_path": body.get("dispatch_packet_path").cloned().unwrap_or(serde_json::Value::Null),
                    "host_bridge_request_path": bridge_request.get("request_path").cloned().unwrap_or(serde_json::Value::Null),
                }
            }),
        );
    }
}

fn host_bridge_state_path_from_request(
    state_root: &Path,
    request: &serde_json::Value,
    field: &str,
) -> Result<PathBuf, String> {
    let raw = request
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Host bridge request is missing non-empty `{field}`."))?;
    let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(raw);
    if path_has_dot_segment(&path) {
        return Err(format!(
            "Host bridge `{field}` path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Host bridge `{field}` path `{}` has no parent directory.",
            path.display()
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create host bridge `{field}` directory `{}`: {error}",
            parent.display()
        )
    })?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
        format!(
            "Failed to canonicalize host bridge `{field}` directory `{}`: {error}",
            parent.display()
        )
    })?;
    let canonical_state_root = std::fs::canonicalize(state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if !canonical_parent.starts_with(&canonical_state_root) {
        return Err(format!(
            "Host bridge `{field}` path `{}` escapes VIDA state root `{}`.",
            path.display(),
            canonical_state_root.display()
        ));
    }
    Ok(path)
}

fn host_bridge_required_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
    artifact_label: &str,
) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Host bridge {artifact_label} is missing non-empty `{field}`."))
}

fn validate_host_bridge_request_dispatch_binding(
    request: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    backend_id: &str,
    carrier_id: &str,
    execution_boundary: &str,
    dispatch_transport: &str,
    receipt_mode: &str,
) -> Result<(), String> {
    let typed_request =
        HostBridgeRequest::from_value(request.clone()).map_err(|error| error.to_string())?;
    let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
        request: typed_request.clone(),
        receipt: Some(serde_json::json!({
            "receipt_backed": true,
            "dispatch_status": receipt.dispatch_status,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
        })),
        allow_active_packet_target_override: false,
    });
    if !decision.accepted {
        return Err(format!(
            "Host bridge request does not match active dispatch: {}.",
            decision.blocker_codes.join(",")
        ));
    }
    for (field, actual, expected) in [
        (
            "task_id",
            typed_request.task_id.as_str(),
            receipt.run_id.as_str(),
        ),
        ("backend_id", typed_request.backend_id.as_str(), backend_id),
        ("carrier_id", typed_request.carrier_id.as_str(), carrier_id),
        (
            "execution_boundary",
            typed_request.execution_boundary.as_str(),
            execution_boundary,
        ),
        (
            "dispatch_transport",
            typed_request.dispatch_transport.as_str(),
            dispatch_transport,
        ),
        (
            "receipt_mode",
            typed_request.receipt_mode.as_str(),
            receipt_mode,
        ),
    ] {
        if actual != expected {
            return Err(format!(
                "Host bridge request `{field}` does not match active dispatch."
            ));
        }
    }
    Ok(())
}

fn validate_completed_host_bridge_artifacts(
    request: &serde_json::Value,
    result: &serde_json::Value,
    bridge_receipt: &serde_json::Value,
    result_path: &Path,
    receipt_path: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    backend_id: &str,
) -> Result<(), String> {
    let typed_request = HostBridgeRequest::from_value(request.clone()).map_err(|error| {
        format!("Host bridge request cannot be parsed for completed result validation: {error}.")
    })?;
    let binding_decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
        request: typed_request.clone(),
        receipt: Some(serde_json::json!({
            "receipt_backed": true,
            "dispatch_status": receipt.dispatch_status,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
        })),
        allow_active_packet_target_override: false,
    });
    if !binding_decision.accepted {
        return Err(format!(
            "Host bridge request receipt binding failed shared validation: {}.",
            binding_decision.blocker_codes.join(",")
        ));
    }
    let request_id = host_bridge_required_string(request, "request_id", "request")?;
    let request_path = host_bridge_required_string(request, "request_path", "request")?;
    let request_result_path = host_bridge_required_string(request, "result_path", "request")?;
    let request_receipt_path = host_bridge_required_string(request, "receipt_path", "request")?;
    let packet_path = host_bridge_required_string(request, "packet_path", "request")?;
    let rendered_result_path = result_path.display().to_string();
    let rendered_receipt_path = receipt_path.display().to_string();
    if request_result_path != rendered_result_path {
        return Err(
            "Host bridge request result_path does not match the resolved result path.".into(),
        );
    }
    if request_receipt_path != rendered_receipt_path {
        return Err(
            "Host bridge request receipt_path does not match the resolved receipt path.".into(),
        );
    }
    if !taskflow_host_bridge::completion::host_bridge_completion_identity_matches(
        request,
        result,
        Some(bridge_receipt),
        &receipt.run_id,
        &receipt.dispatch_target,
        packet_path,
    ) {
        return Err(
            "Host bridge completion identity does not match the active request, run, packet, or lane."
                .into(),
        );
    }
    for (artifact, label) in [(result, "result"), (bridge_receipt, "receipt")] {
        if artifact.get("run_id").and_then(serde_json::Value::as_str)
            != Some(receipt.run_id.as_str())
        {
            return Err(format!(
                "Host bridge {label} run_id does not match active dispatch."
            ));
        }
        if artifact
            .get("dispatch_target")
            .and_then(serde_json::Value::as_str)
            != Some(receipt.dispatch_target.as_str())
        {
            return Err(format!(
                "Host bridge {label} dispatch_target does not match active dispatch."
            ));
        }
        if artifact
            .get("request_id")
            .and_then(serde_json::Value::as_str)
            != Some(request_id)
        {
            return Err(format!(
                "Host bridge {label} request_id does not match the active bridge request."
            ));
        }
        if artifact
            .get("source_dispatch_packet_path")
            .and_then(serde_json::Value::as_str)
            != Some(packet_path)
        {
            return Err(format!(
                "Host bridge {label} source_dispatch_packet_path does not match the active bridge request."
            ));
        }
    }
    if result
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
        != Some("host_tool_bridge_result")
    {
        return Err("Host bridge result artifact_kind is not host_tool_bridge_result.".into());
    }
    if bridge_receipt
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
        != Some("host_tool_bridge_receipt")
    {
        return Err("Host bridge receipt artifact_kind is not host_tool_bridge_receipt.".into());
    }
    if !result
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(host_bridge_completed_result_status_is_admissible)
    {
        return Err("Host bridge result status is not pass or blocked.".into());
    }
    if !bridge_receipt
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(host_bridge_completed_artifact_status_is_admissible)
    {
        return Err("Host bridge receipt status is not pass.".into());
    }
    if !result
        .get("execution_state")
        .and_then(serde_json::Value::as_str)
        .is_some_and(host_bridge_completed_result_execution_state_is_admissible)
    {
        return Err("Host bridge result execution_state is not executed.".into());
    }
    let verdict_blockers =
        host_bridge_result_verdict_contract_blockers(result, &typed_request.required_result_fields);
    if !verdict_blockers.is_empty() {
        return Err(format!(
            "Host bridge result verdict contract failed: {}.",
            verdict_blockers.join(",")
        ));
    }
    if bridge_receipt
        .get("receipt_backed")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err("Host bridge receipt is not receipt_backed=true.".into());
    }
    if result
        .get("execution_evidence")
        .and_then(|evidence| evidence.get("receipt_backed"))
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err("Host bridge result execution_evidence is not receipt_backed=true.".into());
    }
    if result
        .get("execution_evidence")
        .and_then(|evidence| evidence.get("backend_id"))
        .and_then(serde_json::Value::as_str)
        != Some(backend_id)
    {
        return Err(
            "Host bridge result execution_evidence backend_id does not match active dispatch."
                .into(),
        );
    }
    if request.get("status").and_then(serde_json::Value::as_str) != Some("completed") {
        return Err(
            "Host bridge request status is not completed for completed result ingestion.".into(),
        );
    }
    let completion_receipt_id =
        host_bridge_required_string(request, "completion_receipt_id", "completed request")?;
    for (artifact, label) in [(result, "result"), (bridge_receipt, "receipt")] {
        if artifact
            .get("completion_receipt_id")
            .and_then(serde_json::Value::as_str)
            != Some(completion_receipt_id)
        {
            return Err(format!(
                "Host bridge {label} completion_receipt_id does not match the completed bridge request."
            ));
        }
    }
    if bridge_receipt
        .get("request_path")
        .and_then(serde_json::Value::as_str)
        != Some(request_path)
    {
        return Err(
            "Host bridge receipt request_path does not match the active bridge request.".into(),
        );
    }
    if bridge_receipt
        .get("result_path")
        .and_then(serde_json::Value::as_str)
        != Some(rendered_result_path.as_str())
    {
        return Err(
            "Host bridge receipt result_path does not match the active bridge result.".into(),
        );
    }
    Ok(())
}

fn ingest_completed_host_bridge_result(
    state_root: &Path,
    request: &serde_json::Value,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    backend_id: &str,
    carrier_id: &str,
    execution_boundary: &str,
    dispatch_transport: &str,
    receipt_mode: &str,
    host_runtime: &serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let result_path = host_bridge_state_path_from_request(state_root, request, "result_path")?;
    let receipt_path = host_bridge_state_path_from_request(state_root, request, "receipt_path")?;
    if std::fs::symlink_metadata(&result_path).is_err()
        || std::fs::symlink_metadata(&receipt_path).is_err()
    {
        return Ok(None);
    }
    let result_raw = std::fs::read_to_string(&result_path).map_err(|error| {
        format!(
            "Failed to read host bridge result `{}`: {error}",
            result_path.display()
        )
    })?;
    let receipt_raw = std::fs::read_to_string(&receipt_path).map_err(|error| {
        format!(
            "Failed to read host bridge receipt `{}`: {error}",
            receipt_path.display()
        )
    })?;
    let mut result: serde_json::Value = serde_json::from_str(&result_raw).map_err(|error| {
        format!(
            "Failed to decode host bridge result `{}`: {error}",
            result_path.display()
        )
    })?;
    let bridge_receipt: serde_json::Value =
        serde_json::from_str(&receipt_raw).map_err(|error| {
            format!(
                "Failed to decode host bridge receipt `{}`: {error}",
                receipt_path.display()
            )
        })?;
    validate_host_bridge_request_dispatch_binding(
        request,
        receipt,
        backend_id,
        carrier_id,
        execution_boundary,
        dispatch_transport,
        receipt_mode,
    )?;
    validate_completed_host_bridge_artifacts(
        request,
        &result,
        &bridge_receipt,
        &result_path,
        &receipt_path,
        receipt,
        backend_id,
    )?;
    let body = result
        .as_object_mut()
        .ok_or_else(|| "Host bridge result must be a JSON object.".to_string())?;
    let blocker_code = body
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            body.get("blocker_codes")
                .and_then(serde_json::Value::as_array)
                .and_then(|codes| codes.iter().find_map(serde_json::Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    body.insert("surface".to_string(), serde_json::json!("vida agent-init"));
    body.entry("blocker_code".to_string())
        .or_insert_with(|| blocker_code.map_or(serde_json::Value::Null, serde_json::Value::String));
    body.insert("host_runtime".to_string(), host_runtime.clone());
    body.insert("host_tool_bridge_request".to_string(), request.clone());
    body.insert(
        "host_tool_bridge_receipt".to_string(),
        bridge_receipt.clone(),
    );
    body.insert(
        "backend_dispatch".to_string(),
        serde_json::json!({
            "backend_class": "internal",
            "backend_id": backend_id,
            "carrier_id": carrier_id,
            "execution_boundary": execution_boundary,
            "dispatch_transport": dispatch_transport,
            "receipt_mode": receipt_mode,
            "activation_view_is_execution_evidence": false,
            "host_tool_bridge_request": request,
            "host_tool_bridge_result_path": result_path.display().to_string(),
            "host_tool_bridge_receipt_path": receipt_path.display().to_string()
        }),
    );
    mark_dispatch_result_execution_evidence(body, "host_tool_bridge_receipt", backend_id);
    refresh_execution_truth(body, role_selection, receipt, Some(backend_id), "recorded");
    Ok(Some(result))
}

fn configured_internal_host_activation_parts(
    system_entry: Option<&serde_yaml::Value>,
    project_root: &Path,
    dispatch_packet_path: &str,
    carrier: &serde_json::Value,
) -> Result<(String, Vec<String>, Option<String>), String> {
    let dispatch = system_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch"]))
        .ok_or_else(|| "Configured internal host system is missing `dispatch`".to_string())?;
    let command = yaml_lookup(dispatch, &["command"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Configured internal host system is missing non-empty `dispatch.command`".to_string()
        })?
        .to_string();
    let model = carrier["model"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Configured internal host carrier is missing model".to_string())?;
    let sandbox_mode = carrier["sandbox_mode"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Configured internal host carrier is missing sandbox_mode".to_string())?;
    if sandbox_mode == "danger-full-access" {
        return Err(
            "Configured internal host carrier uses forbidden sandbox_mode `danger-full-access`"
                .to_string(),
        );
    }
    let reasoning_effort = carrier["model_reasoning_effort"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Configured internal host carrier is missing model_reasoning_effort; configure the carrier or materialize the configured missing_reasoning_effort_policy default before dispatch."
                .to_string()
        })?;
    let prompt = dispatch_packet_prompt(dispatch_packet_path);
    let mut args = crate::yaml_string_list(yaml_lookup(dispatch, &["static_args"]));
    args.extend(crate::yaml_string_list(yaml_lookup(
        dispatch,
        &["feature_args"],
    )));
    let mut stdin_payload = None;
    if let Some(workdir_flag) = crate::yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        args.push(workdir_flag);
        args.push(project_root.display().to_string());
    }
    if let Some(sandbox_flag) = crate::yaml_string(yaml_lookup(dispatch, &["sandbox_flag"])) {
        args.push(sandbox_flag);
        args.push(sandbox_mode.to_string());
    }
    if let Some(model_flag) = crate::yaml_string(yaml_lookup(dispatch, &["model_flag"])) {
        args.push(model_flag);
        args.push(configured_internal_host_model_arg(dispatch, model));
    }
    if let Some(reasoning_effort_flag) =
        crate::yaml_string(yaml_lookup(dispatch, &["reasoning_effort_flag"]))
    {
        let rendered_value =
            crate::yaml_string(yaml_lookup(dispatch, &["reasoning_effort_value_template"]))
                .map(|template| template.replace("{value}", reasoning_effort))
                .unwrap_or_else(|| reasoning_effort.to_string());
        args.push(reasoning_effort_flag);
        args.push(rendered_value);
    }
    let prompt_mode = crate::yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => args.push(prompt),
        "stdin" => {
            args.push("-".to_string());
            stdin_payload = Some(prompt);
        }
        other => {
            return Err(format!(
                "Configured internal host system uses unsupported prompt_mode `{other}`"
            ));
        }
    }
    Ok((command, args, stdin_payload))
}

fn configured_external_cli_fallback_enabled(overlay: &serde_yaml::Value) -> bool {
    configured_external_cli_backend_ids(overlay, Some(true))
        .into_iter()
        .next()
        .is_some()
}

fn internal_host_receipt_backed_completion_supported(
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> serde_json::Value {
    selected_cli_entry
        .and_then(|entry| {
            yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
                .and_then(serde_yaml::Value::as_bool)
        })
        .map_or(serde_json::Value::Null, serde_json::Value::Bool)
}

fn internal_host_receipt_backed_completion_is_enabled(
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> bool {
    selected_cli_entry.and_then(|entry| {
        yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
            .and_then(serde_yaml::Value::as_bool)
    }) == Some(true)
}

fn annotate_internal_host_completion_capability(
    dispatch: &mut serde_json::Map<String, serde_json::Value>,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    execution_evidence_available: bool,
) {
    dispatch.insert(
        "receipt_backed_completion_supported".to_string(),
        internal_host_receipt_backed_completion_supported(selected_cli_entry),
    );
    dispatch.insert(
        "receipt_backed_completion_source_path".to_string(),
        serde_json::json!(format!(
            "vida.config.yaml:host_environment.systems.{selected_cli_system}.dispatch.receipt_backed_completion_supported"
        )),
    );
    dispatch.insert(
        "execution_evidence_required".to_string(),
        serde_json::json!(true),
    );
    dispatch.insert(
        "execution_evidence_available".to_string(),
        serde_json::json!(execution_evidence_available),
    );
    dispatch.insert(
        "activation_view_is_execution_evidence".to_string(),
        serde_json::json!(false),
    );
}

fn internal_host_app_bridge_requires_fail_closed(
    selected_cli_entry: Option<&serde_yaml::Value>,
    overlay: &serde_yaml::Value,
) -> Option<&'static str> {
    if internal_host_receipt_backed_completion_is_enabled(selected_cli_entry) {
        return None;
    }
    if configured_external_cli_fallback_enabled(overlay) {
        return Some(
            "internal host carrier unavailable; refusing non-receipted internal bridge while an external CLI fallback is configured",
        );
    }
    Some("internal host carrier unavailable; external CLI fallback disabled")
}

fn internal_host_windows_sandbox_preflight_blocker(
    is_windows: bool,
    selected_cli_entry: Option<&serde_yaml::Value>,
    sandbox_mode: Option<&str>,
) -> Option<(&'static str, String)> {
    if !is_windows {
        return None;
    }
    let sandbox_mode = sandbox_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if sandbox_mode != "workspace-write" {
        return None;
    }
    let windows_sandbox_spawn_supported = selected_cli_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch", "windows_sandbox_spawn_supported"]))
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false);
    if windows_sandbox_spawn_supported {
        return None;
    }
    Some((
        "internal_codex_windows_sandbox_unavailable",
        format!(
            "Internal host carrier is configured with sandbox_mode `{sandbox_mode}` on Windows, but this host has not declared `dispatch.windows_sandbox_spawn_supported=true`; failing before process launch avoids a long no-receipt timeout. Route through a configured backend/runtime profile whose sandbox is supported on this host, or enable the support flag only after proving receipt-backed execution."
        ),
    ))
}

fn configured_external_cli_backend_ids(
    overlay: &serde_yaml::Value,
    enabled: Option<bool>,
) -> Vec<String> {
    let mut ids = yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(key, value)| {
                    let backend_id = key.as_str()?.trim();
                    if backend_id.is_empty() {
                        return None;
                    }
                    if crate::yaml_string(yaml_lookup(value, &["subagent_backend_class"]))
                        .as_deref()
                        != Some("external_cli")
                    {
                        return None;
                    }
                    if let Some(expected_enabled) = enabled {
                        let actual_enabled =
                            yaml_lookup(value, &["enabled"]).and_then(serde_yaml::Value::as_bool);
                        if actual_enabled != Some(expected_enabled) {
                            return None;
                        }
                    }
                    Some(backend_id.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

fn internal_host_windows_sandbox_recovery_actions(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    dispatch_target: &str,
    sandbox_mode: Option<&str>,
) -> Vec<String> {
    let disabled_external = configured_external_cli_backend_ids(overlay, Some(false));
    let enabled_external = configured_external_cli_backend_ids(overlay, Some(true));
    let sandbox_mode = sandbox_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let mut actions = vec![
        format!(
            "Preferred: enable a configured external CLI backend that is admissible for dispatch target `{dispatch_target}` (`agent_system.subagents.<backend>.enabled=true`, `subagent_backend_class=external_cli`, readiness satisfied), then route this lane to that backend through the configured runtime assignment/fallback fields."
        ),
        format!(
            "Alternative only after proof: if `{selected_cli_system}` has verified receipt-backed dispatch support for sandbox `{sandbox_mode}` on this Windows host, set `host_environment.systems.{selected_cli_system}.dispatch.windows_sandbox_spawn_supported=true` in `vida.config.yaml`."
        ),
        "Do not continue root-local implementation from this blocker; restore a receipt-backed backend route or record a separate configuration/readiness defect for the missing backend.".to_string(),
    ];
    if !disabled_external.is_empty() {
        actions.insert(
            1,
            format!(
                "Configured external CLI backends currently disabled in `agent_system.subagents`: {}.",
                disabled_external.join(", ")
            ),
        );
    } else if enabled_external.is_empty() {
        actions.insert(
            1,
            "No enabled external CLI backend is configured under `agent_system.subagents`; add or enable one before expecting external fallback dispatch.".to_string(),
        );
    }
    actions
}

pub(crate) fn agent_lane_dispatch_result(
    mut activation_view: serde_json::Value,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> serde_json::Value {
    let effective_selected_backend = preferred_backend
        .map(str::to_string)
        .or_else(|| receipt.selected_backend.clone())
        .or_else(|| {
            crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
                role_selection,
                receipt,
            )
        });
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let blocker_code =
        crate::runtime_dispatch_state::internal_host_activation_view_only_blocker_code(
            &project_root,
            role_selection,
            receipt,
        );
    let lane_dispatch = crate::runtime_dispatch_state::runtime_agent_lane_dispatch_for_root(
        &project_root,
        dispatch_packet_path,
        preferred_backend,
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            preferred_backend,
        )
        .as_deref(),
    );
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
            effective_selected_backend.as_deref(),
            receipt.activation_agent_type.as_deref(),
            Some(&host_runtime),
            false,
            None,
        );
    let execution_truth = summarize_execution_truth_for_route(
        &role_selection.execution_plan,
        crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
        ),
        host_runtime["selected_cli_execution_class"].as_str(),
        effective_selected_backend.as_deref(),
        Some("activation_view"),
        Some("missing"),
    );
    let body = activation_view
        .as_object_mut()
        .expect("agent-init activation view should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(lane_dispatch.surface),
    );
    body.insert("status".to_string(), serde_json::json!("blocked"));
    body.insert("execution_state".to_string(), serde_json::json!("blocked"));
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(lane_dispatch.activation_command),
    );
    body.insert(
        "dispatch_packet_path".to_string(),
        serde_json::json!(dispatch_packet_path),
    );
    body.insert("host_runtime".to_string(), host_runtime);
    body.insert(
        "effective_execution_posture".to_string(),
        effective_execution_posture,
    );
    body.insert("execution_truth".to_string(), execution_truth);
    body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
    body.insert(
        "blocker_reason".to_string(),
        serde_json::json!(
            "selected host/backend returned only an activation view without execution evidence"
        ),
    );
    body.insert(
        "backend_dispatch".to_string(),
        lane_dispatch.backend_dispatch,
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        let runtime_assignment = role_selection
            .execution_plan
            .get("runtime_assignment")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        for key in [
            "selected_carrier_id",
            "selected_backend_id",
            "selected_dispatch_backend_id",
            "selected_model_profile_id",
            "selected_model_ref",
            "selected_model_provider",
            "selected_reasoning_effort",
            "selected_sandbox_mode",
        ] {
            let dispatch_has_value = dispatch.get(key).is_some_and(|value| !value.is_null());
            if !dispatch_has_value && !runtime_assignment[key].is_null() {
                dispatch.insert(key.to_string(), runtime_assignment[key].clone());
            }
        }
    }
    body.insert(
        "role_selection".to_string(),
        serde_json::to_value(role_selection).expect("lane selection should serialize"),
    );
    activation_view
}

fn refresh_execution_truth(
    body: &mut serde_json::Map<String, serde_json::Value>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    effective_selected_backend: Option<&str>,
    execution_evidence_status: &str,
) {
    let host_runtime = body
        .get("host_runtime")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let activation_kind = body
        .get("activation_semantics")
        .and_then(|value| value.get("activation_kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    body.insert(
        "execution_truth".to_string(),
        summarize_execution_truth_for_route(
            &role_selection.execution_plan,
            crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
                &role_selection.execution_plan,
                &receipt.dispatch_target,
            ),
            host_runtime["selected_cli_execution_class"].as_str(),
            effective_selected_backend,
            Some(activation_kind),
            Some(execution_evidence_status),
        ),
    );
}

fn mark_dispatch_result_execution_evidence(
    body: &mut serde_json::Map<String, serde_json::Value>,
    evidence_kind: &str,
    backend_id: &str,
) {
    let completion_receipt_id = body
        .get("completion_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let activation_semantics = body
        .entry("activation_semantics".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let activation_semantics = activation_semantics
        .as_object_mut()
        .expect("activation_semantics should serialize to an object");
    activation_semantics.insert(
        "activation_kind".to_string(),
        serde_json::json!("execution_evidence"),
    );
    activation_semantics.insert("view_only".to_string(), serde_json::json!(false));
    activation_semantics.insert("executes_packet".to_string(), serde_json::json!(true));
    activation_semantics.insert(
        "records_completion_receipt".to_string(),
        serde_json::json!(true),
    );
    activation_semantics.insert(
        "transfers_root_session_write_authority".to_string(),
        serde_json::json!(false),
    );
    activation_semantics.insert(
        "root_session_write_guard_remains_authoritative".to_string(),
        serde_json::json!(true),
    );
    activation_semantics.insert(
        "next_lawful_action".to_string(),
        serde_json::json!(
            "treat this result as receipt-backed delegated-lane execution evidence and continue through runtime downstream progression"
        ),
    );
    let execution_evidence = body
        .entry("execution_evidence".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let execution_evidence = execution_evidence
        .as_object_mut()
        .expect("execution_evidence should serialize to an object");
    execution_evidence
        .entry("status".to_string())
        .or_insert_with(|| serde_json::json!("recorded"));
    execution_evidence.insert(
        "evidence_kind".to_string(),
        serde_json::json!(evidence_kind),
    );
    execution_evidence.insert("backend_id".to_string(), serde_json::json!(backend_id));
    execution_evidence.insert("receipt_backed".to_string(), serde_json::json!(true));
    if let Some(receipt_id) = completion_receipt_id {
        execution_evidence.insert("receipt_id".to_string(), serde_json::json!(receipt_id));
    }
    execution_evidence.insert(
        "records_dispatch_result".to_string(),
        serde_json::json!(true),
    );
    if let Some(posture) = body
        .get_mut("effective_execution_posture")
        .and_then(serde_json::Value::as_object_mut)
    {
        posture.insert(
            "activation_evidence_state".to_string(),
            serde_json::json!("execution_evidence"),
        );
        posture.insert(
            "receipt_backed_execution_evidence".to_string(),
            serde_json::json!(true),
        );
        posture.insert(
            "selected_backend".to_string(),
            serde_json::json!(backend_id),
        );
    }
    if let Some(posture) = body
        .get_mut("execution_truth")
        .and_then(serde_json::Value::as_object_mut)
    {
        posture.insert(
            "effective_selected_backend".to_string(),
            serde_json::json!(backend_id),
        );
        if let Some(activation_evidence) = posture
            .get_mut("activation_evidence")
            .and_then(serde_json::Value::as_object_mut)
        {
            activation_evidence.insert(
                "activation_kind".to_string(),
                serde_json::json!("execution_evidence"),
            );
            activation_evidence.insert(
                "execution_evidence_status".to_string(),
                serde_json::json!("recorded"),
            );
            activation_evidence.insert("receipt_backed".to_string(), serde_json::json!(true));
        }
    }
}

pub(crate) async fn execute_internal_agent_lane_dispatch(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    execute_internal_agent_lane_dispatch_with_fallback_policy(
        state_root,
        project_root,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
        true,
    )
    .await
}

async fn execute_internal_agent_lane_dispatch_with_fallback_policy(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
    allow_internal_codex_external_fallback: bool,
) -> Result<Option<serde_json::Value>, String> {
    let Some(backend_id) = preferred_backend.or(receipt.selected_backend.as_deref()) else {
        return Err(format!(
            "Dispatch target `{}` is routed to an internal agent lane but no backend id was resolved",
            receipt.dispatch_target
        ));
    };
    if !backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        backend_id,
        &receipt.dispatch_target,
    ) {
        return Err(format!(
            "Backend `{backend_id}` is not admissible for dispatch target `{}`",
            receipt.dispatch_target
        ));
    }

    let overlay = crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root)?;
    let (selected_cli_system, selected_cli_entry) =
        crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let execution_class = selected_cli_entry
        .as_ref()
        .and_then(|entry| yaml_lookup(entry, &["execution_class"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            host_runtime["selected_cli_execution_class"]
                .as_str()
                .unwrap_or("unknown")
        });
    if execution_class != "internal" {
        return Ok(None);
    }

    let Some(carrier) = selected_internal_host_carrier(
        selected_cli_entry.as_ref(),
        preferred_backend,
        receipt,
        role_selection,
        Some(&overlay),
    ) else {
        return Ok(None);
    };

    let carrier_id = carrier["role_id"]
        .as_str()
        .unwrap_or(selected_cli_system.as_str());
    let execution_boundary = configured_host_execution_boundary(selected_cli_entry.as_ref());
    let dispatch_transport = configured_host_dispatch_transport(selected_cli_entry.as_ref());
    let receipt_mode = configured_host_receipt_mode(selected_cli_entry.as_ref());
    if dispatch_transport == "host_tool_bridge" {
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            preferred_backend,
            role_selection,
            receipt,
            host_runtime.clone(),
        );
        let bridge_request = materialize_host_tool_bridge_request(
            project_root,
            state_root,
            selected_cli_entry.as_ref(),
            dispatch_packet_path,
            backend_id,
            carrier_id,
            receipt,
            role_selection,
        )?;
        let host_bridge_adapter_argv = bridge_request
            .get("request_path")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|request_path| {
                serde_json::json!([
                    "vida",
                    "agent",
                    "host-bridge",
                    "--request",
                    request_path,
                    "--json"
                ])
            });
        if let Some(result) = ingest_completed_host_bridge_result(
            state_root,
            &bridge_request,
            role_selection,
            receipt,
            backend_id,
            carrier_id,
            &execution_boundary,
            &dispatch_transport,
            &receipt_mode,
            &host_runtime,
        )? {
            return Ok(Some(result));
        }
        let body = result
            .as_object_mut()
            .expect("internal host bridge dispatch result should serialize to an object");
        body.insert("surface".to_string(), serde_json::json!("vida agent-init"));
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert(
            "execution_state".to_string(),
            serde_json::json!("bridge_request_pending"),
        );
        body.insert(
            "dispatch_mode".to_string(),
            serde_json::json!({
                "mode": "execution_dispatch",
                "requested_execute_dispatch": true,
                "has_packet_source": true,
                "activation_view_only": false,
                "execution_dispatch": true,
                "activation_view_is_execution_evidence": false,
                "activation_view_completes_delegated_work": false,
                "execution_evidence_required_for_completion": true,
                "completion_requires_receipt_backed_execution": true,
                "required_completion_evidence": "host_tool_bridge_receipt",
                "missing_execution_evidence_semantics": "non_executing_bridge_blocker",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false,
            }),
        );
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("host_tool_bridge_adapter_required"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(
                "internal_subagents require a configured parent host-agent bridge; vida.exe cannot call parent host adapter tools directly"
            ),
        );
        body.insert(
            "host_tool_bridge_request".to_string(),
            compact_host_tool_bridge_request_for_dispatch_result(&bridge_request),
        );
        if let Some(argv) = host_bridge_adapter_argv.as_ref() {
            body.insert("host_bridge_adapter_argv".to_string(), argv.clone());
        }
        body.insert(
            "next_actions".to_string(),
            serde_json::json!([
                "A configured parent host-agent adapter must read host_tool_bridge_request.request_path or host_bridge_adapter_argv, invoke the configured adapter capability without shell command interpolation, then submit a receipt-backed result through the host-bridge completion surface.",
                "Do not fall back to a child-process agent command for internal_subagents; use an explicit process carrier only when route policy selects that backend."
            ]),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert("backend_class".to_string(), serde_json::json!("internal"));
            dispatch.insert("backend_id".to_string(), serde_json::json!(backend_id));
            dispatch.insert("carrier_id".to_string(), serde_json::json!(carrier_id));
            dispatch.insert(
                "execution_boundary".to_string(),
                serde_json::json!(execution_boundary),
            );
            dispatch.insert(
                "dispatch_transport".to_string(),
                serde_json::json!(dispatch_transport),
            );
            dispatch.insert("receipt_mode".to_string(), serde_json::json!(receipt_mode));
            dispatch.insert(
                "activation_view_is_execution_evidence".to_string(),
                serde_json::json!(false),
            );
            dispatch.insert(
                "host_tool_bridge_request".to_string(),
                compact_host_tool_bridge_request_for_dispatch_result(&bridge_request),
            );
            if let Some(argv) = host_bridge_adapter_argv {
                dispatch.insert("host_bridge_adapter_argv".to_string(), argv);
            }
        }
        refresh_execution_truth(body, role_selection, receipt, Some(backend_id), "missing");
        compact_pending_host_bridge_dispatch_result(body, role_selection, receipt, &bridge_request);
        return Ok(Some(result));
    }
    let (command, args, stdin_payload) = configured_internal_host_activation_parts(
        selected_cli_entry.as_ref(),
        project_root,
        dispatch_packet_path,
        &carrier,
    )?;
    let preflight_blocker =
        internal_host_app_bridge_requires_fail_closed(selected_cli_entry.as_ref(), &overlay)
            .map(|reason| ("internal_codex_carrier_unavailable", reason.to_string()))
            .or_else(|| {
                internal_host_windows_sandbox_preflight_blocker(
                    cfg!(windows),
                    selected_cli_entry.as_ref(),
                    carrier["sandbox_mode"].as_str(),
                )
            });
    if let Some((blocker_code, blocker_reason)) = preflight_blocker {
        let preflight_recovery_actions =
            if blocker_code == "internal_codex_windows_sandbox_unavailable" {
                internal_host_windows_sandbox_recovery_actions(
                    &overlay,
                    &selected_cli_system,
                    &receipt.dispatch_target,
                    carrier["sandbox_mode"].as_str(),
                )
            } else {
                vec![blocker_reason.clone()]
            };
        if allow_internal_codex_external_fallback {
            if let Some(fallback_backend) = internal_host_external_fallback_backend(
                role_selection,
                &receipt.dispatch_target,
                backend_id,
                &overlay,
            ) {
                let mut result = Box::pin(execute_external_agent_lane_dispatch(
                    state_root,
                    project_root,
                    dispatch_packet_path,
                    Some(&fallback_backend),
                    role_selection,
                    receipt,
                    host_runtime.clone(),
                ))
                .await?;
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "internal_codex_external_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "blocker_code": blocker_code,
                            "blocker_reason": blocker_reason,
                            "fallback_backend": fallback_backend,
                            "fallback_source": "route_admissible_external_backend",
                            "selected_cli_system": selected_cli_system,
                        }),
                    );
                }
                return Ok(Some(result));
            }
        }
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            preferred_backend,
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("internal agent lane dispatch result should serialize to an object");
        body.insert("surface".to_string(), serde_json::json!("vida agent-init"));
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "activation_command".to_string(),
            serde_json::json!(
                crate::runtime_dispatch_state::agent_init_command_for_packet_path(
                    dispatch_packet_path
                )
            ),
        );
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason.clone()),
        );
        body.insert(
            "next_actions".to_string(),
            serde_json::json!(preflight_recovery_actions.clone()),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert("backend_class".to_string(), serde_json::json!("internal"));
            dispatch.insert("backend_id".to_string(), serde_json::json!(backend_id));
            dispatch.insert(
                "carrier_id".to_string(),
                serde_json::json!(carrier["role_id"].clone()),
            );
            dispatch.insert(
                "sandbox_mode".to_string(),
                serde_json::json!(carrier["sandbox_mode"].clone()),
            );
            dispatch.insert(
                "preflight_blocker_code".to_string(),
                serde_json::json!(blocker_code),
            );
            dispatch.insert(
                "preflight_blocker_reason".to_string(),
                serde_json::json!(blocker_reason),
            );
            dispatch.insert(
                "preflight_recovery_actions".to_string(),
                serde_json::json!(preflight_recovery_actions),
            );
            dispatch.insert(
                "configured_external_cli_candidates".to_string(),
                serde_json::json!({
                    "enabled": configured_external_cli_backend_ids(&overlay, Some(true)),
                    "disabled": configured_external_cli_backend_ids(&overlay, Some(false)),
                }),
            );
            dispatch.insert(
                "executor_backend".to_string(),
                serde_json::json!("internal"),
            );
            dispatch.insert(
                "external_cli_fallback_enabled".to_string(),
                serde_json::json!(configured_external_cli_fallback_enabled(&overlay)),
            );
            annotate_internal_host_completion_capability(
                dispatch,
                &selected_cli_system,
                selected_cli_entry.as_ref(),
                false,
            );
        }
        refresh_execution_truth(body, role_selection, receipt, Some(backend_id), "missing");
        return Ok(Some(result));
    }
    let wall_timeout_seconds = Some(configured_internal_host_dispatch_wall_timeout_seconds(
        project_root,
        role_selection,
        receipt,
    ));
    let no_output_timeout_seconds =
        configured_internal_host_dispatch_no_output_timeout_seconds(selected_cli_entry.as_ref());
    let wrapped_command = wrap_command_with_optional_timeouts(
        command.clone(),
        args.clone(),
        wall_timeout_seconds,
        no_output_timeout_seconds,
    );
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &wrapped_command.command,
        &wrapped_command.args,
    );
    let runtime_env =
        configured_internal_host_runtime_env(project_root, &selected_cli_system, carrier_id)?;

    let mut process = std::process::Command::new(&wrapped_command.command);
    process
        .args(&wrapped_command.args)
        .current_dir(project_root);
    for (key, value) in runtime_env {
        process.env(key, value);
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    process.env("VIDA_SELECTED_BACKEND", carrier_id);
    if let Some(profile_id) = carrier["selected_model_profile_id"].as_str() {
        process.env("VIDA_SELECTED_MODEL_PROFILE", profile_id);
    }
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }

    let output = execute_wrapped_command_async(
        process,
        wrapped_command.clone(),
        stdin_payload.map(String::into_bytes),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to execute internal host carrier `{carrier_id}` for `{selected_cli_system}` via `{}`: {error}",
            wrapped_command.command
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_output = parse_internal_codex_exec_output(&stdout);
    let exit_code = output.status.code();
    let timed_out = output.timed_out;
    let success =
        internal_codex_output_confirms_execution(&parsed_output, &stderr, output.status.success());
    let activation_only = timed_out
        || (output.status.success()
            && parsed_output.result_text.is_none()
            && parsed_output.error_messages.is_empty()
            && stderr.is_empty());
    let activation_view = if should_render_store_backed_activation_view_for_internal_failure(
        activation_only,
        success,
    ) {
        bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await
    } else {
        default_activation_view(receipt, role_selection)
    };
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
    );
    let body = result
        .as_object_mut()
        .expect("internal agent lane dispatch result should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(format!("internal_cli:{selected_cli_system}")),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch.insert("backend_class".to_string(), serde_json::json!("internal"));
        dispatch.insert("backend_id".to_string(), serde_json::json!(carrier_id));
        dispatch.insert(
            "carrier_id".to_string(),
            serde_json::json!(carrier["role_id"].clone()),
        );
        dispatch.insert(
            "model".to_string(),
            serde_json::json!(carrier["model"].clone()),
        );
        dispatch.insert(
            "model_reasoning_effort".to_string(),
            serde_json::json!(carrier["model_reasoning_effort"].clone()),
        );
        dispatch.insert(
            "sandbox_mode".to_string(),
            serde_json::json!(carrier["sandbox_mode"].clone()),
        );
        for key in [
            "selected_model_profile_id",
            "selected_model_ref",
            "selected_model_provider",
            "selected_reasoning_effort",
            "selected_sandbox_mode",
            "internal_subagent_backend_id",
            "internal_subagent_model_profile_id",
        ] {
            if !carrier[key].is_null() {
                dispatch.insert(key.to_string(), carrier[key].clone());
            }
        }
        annotate_internal_host_completion_capability(
            dispatch,
            &selected_cli_system,
            selected_cli_entry.as_ref(),
            success,
        );
    }

    body.insert(
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if let Some(timeout_wrapper) = &wrapped_command.timeout_wrapper {
        body.insert(
            "timeout_wrapper".to_string(),
            serde_json::json!({
                "command": wrapped_command.command,
                "timeout_seconds": timeout_wrapper.timeout_seconds,
                "kill_after_grace_seconds": timeout_wrapper.kill_after_grace_seconds,
                "no_output_timeout_seconds": timeout_wrapper.no_output_timeout_seconds,
                "timed_out": timed_out,
                "timeout_exit_code": exit_code,
            }),
        );
    }
    body.insert(
        "provider_output_json".to_string(),
        parsed_output.raw_json.clone(),
    );
    body.insert(
        "provider_result".to_string(),
        parsed_output
            .result_text
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    body.insert(
        "provider_error_items".to_string(),
        serde_json::to_value(parsed_output.error_messages.clone())
            .expect("internal host error items should serialize"),
    );
    if success {
        body.insert("blocker_code".to_string(), serde_json::Value::Null);
        body.insert("blocker_reason".to_string(), serde_json::Value::Null);
        mark_dispatch_result_execution_evidence(body, "internal_carrier_completion", carrier_id);
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "recorded");
    } else if activation_only {
        if timed_out {
            let timeout_seconds = wrapped_command
                .timeout_wrapper
                .as_ref()
                .map(|wrapper| wrapper.timeout_seconds)
                .unwrap_or_default();
            let kill_after_grace_seconds = wrapped_command
                .timeout_wrapper
                .as_ref()
                .map(|wrapper| wrapper.kill_after_grace_seconds)
                .unwrap_or_default();
            body.insert(
                "provider_error".to_string(),
                serde_json::json!(format!(
                    "internal host carrier for `{selected_cli_system}` timed out after {timeout_seconds}s and kill-after grace {kill_after_grace_seconds}s without receipt-backed completion"
                )),
            );
        }
        let blocker_reason = if timed_out {
            format!(
                "internal host carrier for `{selected_cli_system}` exceeded the bounded runtime window before returning execution evidence"
            )
        } else {
            format!(
                "internal host carrier for `{selected_cli_system}` completed without returning an agent_message result"
            )
        };
        let blocker_code = internal_host_activation_only_blocker_code(
            project_root,
            role_selection,
            receipt,
            timed_out,
        );
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "missing");
    } else {
        let effective_stderr = (!internal_codex_stderr_is_benign_warning(&stderr))
            .then_some(stderr.as_str())
            .unwrap_or("");
        let blocker_reason = if !effective_stderr.is_empty() {
            effective_stderr.to_string()
        } else if !parsed_output.error_messages.is_empty() {
            parsed_output.error_messages.join("\n")
        } else if output.status.success() {
            format!(
                "internal host carrier for `{selected_cli_system}` completed without returning an agent_message result"
            )
        } else {
            format!(
                "internal host carrier for `{selected_cli_system}` exited without returning receipt-backed completion"
            )
        };
        let blocker_code =
            internal_host_provider_failure_blocker_code(&stderr, &parsed_output.error_messages)
                .unwrap_or("configured_backend_dispatch_failed");
        let blocker_reason =
            internal_host_provider_failure_blocker_reason(blocker_code, blocker_reason);
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "missing");
    }

    Ok(Some(result))
}

pub(crate) async fn execute_external_agent_lane_dispatch(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let overlay = crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root)?;
    let (selected_cli_system, _) =
        crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preferred_external_backend = preferred_backend.and_then(|backend_id| {
        crate::runtime_dispatch_state::configured_external_backend_entry_any(&overlay, backend_id)
            .map(|entry| (backend_id.to_string(), entry.clone()))
    });
    let (backend_id, backend_entry, backend_class) = if let Some((backend_id, backend_entry)) =
        preferred_external_backend
    {
        (backend_id, backend_entry, "external_cli".to_string())
    } else {
        let backend_class = crate::runtime_dispatch_state::configured_dispatch_backend_class(
            &overlay,
            &selected_cli_system,
        );
        let (backend_id, backend_entry) =
            crate::runtime_dispatch_state::selected_external_backend_for_system(
                &overlay,
                &selected_cli_system,
                preferred_backend,
            )
            .ok_or_else(|| {
                format!(
                    "Configured host CLI system `{selected_cli_system}` has no enabled external backend dispatch adapter"
                )
            })?;
        (backend_id, backend_entry, backend_class)
    };

    if let Some(dispatch_blocker) =
        crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            &backend_id,
            &backend_entry,
        )
    {
        let readiness_verdict = serde_json::json!({
            "backend_id": backend_id,
            "status": "external_backend_dispatch_blocked",
            "blocked": true,
            "blocker_code": "configured_backend_dispatch_failed",
            "blocker_reason": dispatch_blocker,
            "next_actions": [
                format!("Enable and repair external backend `{backend_id}` in `vida.config.yaml`, or reroute this lane to a receipt-backed backend before dispatch.")
            ],
        });
        if let Some(fallback_backend) = ready_external_readiness_fallback_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
            &overlay,
            receipt.selected_backend.as_deref(),
        ) {
            let mut result = Box::pin(execute_external_agent_lane_dispatch(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
            ))
            .await?;
            if let Some(body) = result.as_object_mut() {
                body.insert(
                    "external_dispatch_blocker_external_fallback".to_string(),
                    serde_json::json!({
                        "blocked_backend": backend_id,
                        "fallback_backend": fallback_backend,
                        "readiness": readiness_verdict,
                    }),
                );
            }
            return Ok(result);
        }
        if let Some(fallback_backend) = readiness_fallback_internal_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
        ) {
            if let Some(mut result) = execute_internal_agent_lane_dispatch_with_fallback_policy(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
                false,
            )
            .await?
            {
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "external_dispatch_blocker_internal_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "fallback_backend": fallback_backend,
                            "readiness": readiness_verdict,
                        }),
                    );
                }
                return Ok(result);
            }
        }
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(dispatch_blocker),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "external_backend_readiness".to_string(),
            readiness_verdict.clone(),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert(
                "backend_class".to_string(),
                serde_json::json!(backend_class.clone()),
            );
            dispatch.insert("external_backend_readiness".to_string(), readiness_verdict);
        }
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }

    // Admissibility gate: refuse to dispatch to an external backend that is not
    // admissible for the target lane (e.g. a read-only backend for an implementer lane).
    if !backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        &backend_id,
        &receipt.dispatch_target,
    ) {
        let activation_view = match StateStore::open_existing(state_root.to_path_buf()).await {
            Ok(store) => {
                let rendered =
                    crate::init_surfaces::render_agent_init_packet_activation_with_store(
                        &store,
                        project_root,
                        dispatch_packet_path,
                        dispatch_packet_path_should_render_as_downstream(dispatch_packet_path),
                    )
                    .await
                    .unwrap_or_else(|_| default_activation_view(receipt, role_selection));
                drop(store);
                rendered
            }
            Err(_) => default_activation_view(receipt, role_selection),
        };
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("backend_inadmissible_for_lane"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(format!(
                "Backend `{backend_id}` is not admissible for dispatch target `{}` (lane_admissibility denies this lane); an implementation-capable backend is required",
                receipt.dispatch_target
            )),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }

    let route_selected_model_profile_id =
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            Some(&backend_id),
        );
    let packet_has_concrete_owned_paths =
        crate::taskflow_consume_resume::read_dispatch_packet(dispatch_packet_path)
            .ok()
            .as_ref()
            .is_some_and(
                crate::runtime_dispatch_state::runtime_dispatch_packet_has_concrete_owned_paths,
            );
    let policy_dispatch_target =
        crate::runtime_dispatch_state::policy_dispatch_target_for_admissibility(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
        );
    let (readiness_verdict, selected_model_profile_id) = external_cli_dispatch_readiness_verdict(
        &backend_id,
        &backend_entry,
        route_selected_model_profile_id,
        &policy_dispatch_target,
        packet_has_concrete_owned_paths,
    );
    if readiness_verdict["blocked"].as_bool().unwrap_or(false) {
        if let Some(fallback_backend) = ready_external_readiness_fallback_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
            &overlay,
            receipt.selected_backend.as_deref(),
        ) {
            let mut result = Box::pin(execute_external_agent_lane_dispatch(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
            ))
            .await?;
            if let Some(body) = result.as_object_mut() {
                body.insert(
                    "external_readiness_external_fallback".to_string(),
                    serde_json::json!({
                        "blocked_backend": backend_id,
                        "fallback_backend": fallback_backend,
                        "readiness": readiness_verdict,
                    }),
                );
            }
            return Ok(result);
        }
        if let Some(fallback_backend) = readiness_fallback_internal_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
        ) {
            if let Some(mut result) = execute_internal_agent_lane_dispatch_with_fallback_policy(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
                false,
            )
            .await?
            {
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "external_readiness_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "fallback_backend": fallback_backend,
                            "readiness": readiness_verdict,
                        }),
                    );
                }
                return Ok(result);
            }
        }
        let readiness_status = readiness_verdict["status"]
            .as_str()
            .unwrap_or("external_backend_blocked");
        let selected_model_profile = selected_model_profile_id
            .as_deref()
            .or_else(|| readiness_verdict["selected_model_profile"].as_str())
            .unwrap_or("unknown");
        let next_action = readiness_verdict["next_actions"]
            .as_array()
            .and_then(|actions| actions.iter().filter_map(serde_json::Value::as_str).next())
            .unwrap_or("Repair the external backend readiness blocker before dispatch.");
        let blocker_code = readiness_verdict
            .get("blocker_code")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or_else(|| serde_json::json!("external_backend_not_ready"));
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert("blocker_code".to_string(), blocker_code);
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(format!(
                "External backend `{backend_id}` is not dispatch-ready before launch: {readiness_status}; selected_model_profile={selected_model_profile}. {next_action}"
            )),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "external_backend_readiness".to_string(),
            readiness_verdict.clone(),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert(
                "backend_class".to_string(),
                serde_json::json!(backend_class.clone()),
            );
            dispatch.insert("external_backend_readiness".to_string(), readiness_verdict);
        }
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }
    let (command, args) = crate::runtime_dispatch_state::configured_external_activation_parts(
        &backend_id,
        &backend_entry,
        project_root,
        dispatch_packet_path,
        selected_model_profile_id.as_deref(),
    )?;
    let stdin_payload =
        crate::runtime_dispatch_state::configured_external_activation_stdin_payload(
            &backend_entry,
            dispatch_packet_path,
        )?;
    let wall_timeout_seconds = configured_external_dispatch_wall_timeout_seconds(&backend_entry);
    let wrapped_command =
        wrap_command_with_optional_timeout(command.clone(), args.clone(), wall_timeout_seconds);
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &wrapped_command.command,
        &wrapped_command.args,
    );

    let mut process = std::process::Command::new(&wrapped_command.command);
    process
        .args(&wrapped_command.args)
        .current_dir(project_root)
        .stdin(Stdio::null());
    if let Some(serde_yaml::Value::Mapping(env_map)) =
        yaml_lookup(&backend_entry, &["dispatch", "env"])
    {
        for (key, value) in env_map {
            if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                process.env(key, value);
            }
        }
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    if let Some(profile_id) = selected_model_profile_id.as_deref() {
        process.env("VIDA_SELECTED_MODEL_PROFILE", profile_id);
    }
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }
    let effective_selected_backend =
        crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
            role_selection,
            receipt,
        )
        .or_else(|| receipt.selected_backend.clone());
    if let Some(selected_backend) = effective_selected_backend.as_deref() {
        process.env("VIDA_SELECTED_BACKEND", selected_backend);
    }

    #[cfg(test)]
    let output = if let Some(output) = emulated_test_shell_output(&wrapped_command) {
        output
    } else {
        execute_wrapped_command_async(
            process,
            wrapped_command.clone(),
            stdin_payload.clone().map(String::into_bytes),
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to execute configured external backend `{backend_id}` via `{}`: {error}",
                wrapped_command.command
            )
        })?
    };
    #[cfg(not(test))]
    let output = execute_wrapped_command_async(
        process,
        wrapped_command.clone(),
        stdin_payload.map(String::into_bytes),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to execute configured external backend `{backend_id}` via `{}`: {error}",
            wrapped_command.command
        )
    })?;
    let activation_view = bounded_activation_view(
        state_root,
        project_root,
        dispatch_packet_path,
        receipt,
        role_selection,
    )
    .await;
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
    );
    let body = result
        .as_object_mut()
        .expect("agent lane dispatch result should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(format!("{backend_class}:{backend_id}")),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch.insert(
            "backend_class".to_string(),
            serde_json::json!(backend_class),
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_output = parse_external_provider_output(&stdout);
    let output_mode = configured_external_dispatch_output_mode(&backend_entry);
    let success = output.status.success()
        && external_provider_output_confirms_execution_for_mode(
            &output_mode,
            &stdout,
            parsed_output.as_ref(),
        );
    let exit_code = output.status.code();
    let timed_out = output.timed_out;
    body.insert(
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if let Some(timeout_wrapper) = &wrapped_command.timeout_wrapper {
        body.insert(
            "timeout_wrapper".to_string(),
            serde_json::json!({
                "command": wrapped_command.command,
                "timeout_seconds": timeout_wrapper.timeout_seconds,
                "kill_after_grace_seconds": timeout_wrapper.kill_after_grace_seconds,
                "no_output_timeout_seconds": timeout_wrapper.no_output_timeout_seconds,
                "timed_out": timed_out,
                "timeout_exit_code": exit_code,
            }),
        );
    }
    if let Some(parsed_output) = parsed_output.as_ref() {
        body.insert(
            "provider_output_json".to_string(),
            parsed_output.raw_json.clone(),
        );
        body.insert(
            "provider_result".to_string(),
            parsed_output
                .result_text
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
        body.insert(
            "provider_usage".to_string(),
            parsed_output
                .usage
                .clone()
                .unwrap_or(serde_json::Value::Null),
        );
        if let Some(scope_guard) = external_provider_scope_guard(&parsed_output.raw_json) {
            body.insert("provider_scope_guard".to_string(), scope_guard.clone());
        }
        if let Some(paths) = external_provider_reported_paths(&parsed_output.raw_json) {
            body.insert("provider_reported_paths".to_string(), paths);
        }
        body.insert(
            "provider_is_error".to_string(),
            parsed_output
                .is_error
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null),
        );
        body.insert(
            "provider_error_message".to_string(),
            parsed_output
                .error_message
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    if success {
        body.insert("blocker_code".to_string(), serde_json::Value::Null);
        body.insert("blocker_reason".to_string(), serde_json::Value::Null);
        mark_dispatch_result_execution_evidence(body, "external_backend_completion", &backend_id);
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "recorded");
    } else if timed_out
        || parsed_output
            .as_ref()
            .is_some_and(external_provider_output_indicates_agent_end_timeout)
    {
        let timeout_seconds = wrapped_command
            .timeout_wrapper
            .as_ref()
            .map(|wrapper| wrapper.timeout_seconds)
            .unwrap_or_default();
        let kill_after_grace_seconds = wrapped_command
            .timeout_wrapper
            .as_ref()
            .map(|wrapper| wrapper.kill_after_grace_seconds)
            .unwrap_or_default();
        body.insert(
            "provider_error".to_string(),
            serde_json::json!(
                parsed_output
                    .as_ref()
                    .and_then(external_provider_error_message)
                    .unwrap_or_else(|| format!(
                        "configured external backend timed out after {timeout_seconds}s and kill-after grace {kill_after_grace_seconds}s without receipt-backed completion"
                    ))
            ),
        );
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::TimeoutWithoutTakeoverAuthority
            )),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(
                "configured external backend exceeded the bounded runtime window before returning execution evidence"
            ),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
    } else {
        let provider_error_message = parsed_output
            .as_ref()
            .and_then(external_provider_error_message)
            .or_else(|| {
                output.status.success().then(|| {
                    "configured external backend exited successfully but did not return a parseable success payload"
                        .to_string()
                })
            })
            .or_else(|| {
                body.get("provider_error_message")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
            });
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(provider_error_message.unwrap_or_else(|| {
                "configured external backend exited without returning receipt-backed completion"
                    .to_string()
            })),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[cfg(any(unix, windows))]
    use super::execute_wrapped_command;
    use super::{
        CommandTimeoutWrapper, agent_lane_dispatch_result,
        configured_external_dispatch_output_mode,
        configured_external_dispatch_wall_timeout_seconds, configured_host_dispatch_transport,
        configured_host_execution_boundary, configured_host_receipt_mode,
        configured_host_tool_bridge_dir, configured_host_tool_bridge_string,
        configured_internal_host_activation_parts,
        configured_internal_host_dispatch_no_output_timeout_seconds,
        configured_internal_host_dispatch_wall_timeout_seconds,
        configured_internal_host_runtime_env, dispatch_packet_path_should_render_as_downstream,
        dispatch_packet_prompt, execute_external_agent_lane_dispatch,
        execute_internal_agent_lane_dispatch, external_provider_output_confirms_execution,
        external_provider_output_confirms_execution_for_mode, host_tool_bridge_artifact_paths,
        host_tool_bridge_request_id, internal_codex_output_confirms_execution,
        internal_host_activation_only_blocker_code, internal_host_app_bridge_requires_fail_closed,
        internal_host_windows_sandbox_preflight_blocker,
        internal_host_windows_sandbox_recovery_actions, mark_dispatch_result_execution_evidence,
        materialize_host_tool_bridge_request, parse_external_provider_output,
        parse_internal_codex_exec_output, ready_external_readiness_fallback_backend,
        should_render_store_backed_activation_view_for_internal_failure,
        wrap_command_with_optional_timeout, wrap_command_with_optional_timeouts,
    };
    use crate::RuntimeConsumptionLaneSelection;
    use std::path::{Path, PathBuf};
    #[cfg(any(unix, windows))]
    use std::process::Stdio;
    #[cfg(any(unix, windows))]
    use std::time::{Duration, Instant};

    #[test]
    fn parse_external_provider_output_extracts_qwen_json_success_result() {
        let parsed = parse_external_provider_output(
            r#"[{"type":"system"},{"type":"result","subtype":"success","is_error":false,"result":"OK","usage":{"total_tokens":42}}]"#,
        )
        .expect("qwen json output should parse");

        assert_eq!(parsed.result_text.as_deref(), Some("OK"));
        assert_eq!(parsed.is_error, Some(false));
        assert_eq!(
            parsed.usage.expect("usage should exist")["total_tokens"],
            42
        );
        assert_eq!(parsed.error_message, None);
    }

    #[test]
    fn parse_external_provider_output_extracts_qwen_json_error_message() {
        let parsed = parse_external_provider_output(
            r#"[{"type":"result","subtype":"error_during_execution","is_error":true,"error":{"message":"Missing API key"}}]"#,
        )
        .expect("qwen json error output should parse");

        assert_eq!(parsed.is_error, Some(true));
        assert_eq!(parsed.error_message.as_deref(), Some("Missing API key"));
        assert_eq!(parsed.result_text, None);
    }

    #[test]
    fn parse_external_provider_output_detects_bracketed_api_error() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","is_error":false,"result":"[API Error: 401 invalid access token or token expired]"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("[API Error: 401 invalid access token or token expired]")
        );
    }

    #[test]
    fn parse_external_provider_output_with_success_stays_success() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(!super::external_provider_output_indicates_error(&parsed));
        assert!(external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn stdout_output_mode_accepts_nonempty_zero_exit_text_without_provider_binary_hardcode() {
        let backend_entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
dispatch:
  output_mode: stdout
"#,
        )
        .expect("backend entry should parse");

        let output_mode = configured_external_dispatch_output_mode(&backend_entry);
        assert_eq!(output_mode, "stdout");
        assert!(external_provider_output_confirms_execution_for_mode(
            &output_mode,
            "Reviewed packet and found no blocking issues.",
            None,
        ));
        assert!(!external_provider_output_confirms_execution_for_mode(
            &output_mode,
            "   ",
            None,
        ));
    }

    #[test]
    fn parse_external_provider_output_trusts_pi_agent_end_success_even_when_result_mentions_auth_text()
     {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"raw_provider":{"mode":"rpc","provider":"pi","terminal_event":"agent_end"},"result":"packet text mentions authentication failed and invalid api key as configuration examples"}"#,
        )
        .expect("pi adapter json output should parse");

        assert!(!super::external_provider_output_indicates_error(&parsed));
        assert!(external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn parse_external_provider_output_blocks_pi_agent_end_when_result_declares_blocked_dispatch() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"raw_provider":{"mode":"rpc","provider":"pi","terminal_event":"agent_end"},"result":"Thinking mode: STC.\nBounded result: dispatch blocked by VIDA Pi write-scope guard; both the packet's `vida agent-init --execute-dispatch` path and the verification path are refused in bash guarded-write mode, so no execution receipt/result artifact was produced."}"#,
        )
        .expect("pi adapter blocked dispatch output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn parse_external_provider_output_classifies_pi_agent_end_timeout_as_runtime_timeout() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"raw_provider":{"mode":"rpc","provider":"pi"},"error":{"message":"Timed out waiting for Pi agent_end event"}}"#,
        )
        .expect("pi timeout output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(super::external_provider_output_indicates_agent_end_timeout(
            &parsed
        ));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("Timed out waiting for Pi agent_end event")
        );
    }

    #[test]
    fn configured_external_dispatch_wall_timeout_honors_backend_max_runtime() {
        let backend_entry = serde_yaml::from_str(
            r#"
max_runtime_seconds: 420
dispatch:
  no_output_timeout_seconds: 180
"#,
        )
        .expect("backend entry should parse");

        assert_eq!(
            configured_external_dispatch_wall_timeout_seconds(&backend_entry),
            Some(420)
        );
    }

    #[test]
    fn parse_external_provider_output_blocks_scope_guard_violation() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK","scope_guard":{"status":"violation","valid":false,"touched_paths":["docs/spec.md"]}}"#,
        )
        .expect("adapter json output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
        assert_eq!(
            super::external_provider_scope_guard(&parsed.raw_json)
                .expect("scope guard should be preserved")["status"],
            "violation"
        );
    }

    #[test]
    fn parse_external_provider_output_exposes_reported_paths() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK","raw_provider":{"provider_result_json":{"touched_paths":["src/lib.rs"],"changed_files":["src/main.rs"]}}}"#,
        )
        .expect("adapter json output should parse");

        let paths = super::external_provider_reported_paths(&parsed.raw_json)
            .expect("reported paths should be preserved");
        assert_eq!(paths["touched_paths"][0], "src/lib.rs");
        assert_eq!(paths["changed_files"][0], "src/main.rs");
    }

    #[test]
    fn parse_external_provider_output_detects_quota_exceeded_semantic_failure() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Qwen OAuth quota exceeded: Your free daily quota has been reached."}"#,
        )
        .expect("qwen json quota output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("Qwen OAuth quota exceeded: Your free daily quota has been reached.")
        );
    }

    #[test]
    fn parse_external_provider_output_bracketed_api_error_cannot_be_treated_as_executed() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","is_error":false,"result":"[API Error: 401 invalid access token or token expired]"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        let status_code_success = true;
        let execution_succeeded =
            status_code_success && !super::external_provider_output_indicates_error(&parsed);
        assert!(!execution_succeeded);
    }

    #[test]
    fn parse_external_provider_output_plain_text_success_stays_unparsable() {
        let parsed = parse_external_provider_output("external-dispatch:implemented");
        assert!(parsed.is_none());
        assert!(!external_provider_output_confirms_execution(
            parsed.as_ref()
        ));
    }

    #[test]
    fn parse_external_provider_output_plain_text_auth_failure_stays_unparsable() {
        let parsed =
            parse_external_provider_output("Authentication failed: invalid API key provided");
        assert!(parsed.is_none());
        assert!(!external_provider_output_confirms_execution(
            parsed.as_ref()
        ));
    }

    #[test]
    fn dispatch_packet_prompt_repairs_stale_downstream_empty_request_tail() {
        let packet_path = std::env::temp_dir().join(format!(
            "vida-stale-empty-request-packet-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": "run-1",
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "downstream_dispatch_target": "coach",
                "activation_runtime_role": "coach",
                "prompt": "Packet run_id=run-1\nTarget=coach\nRequest: ",
                "coach_review_packet": {
                    "review_goal": "Review test-author evidence",
                    "blocking_question": "Is the proof receipt-backed?",
                    "expected_output": ["decision=approve|rework|blocker"]
                },
                "orchestration_contract": {
                    "replanning": {
                        "checkpoints": ["after_review"]
                    }
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");

        let prompt =
            dispatch_packet_prompt(packet_path.to_str().expect("packet path should be utf-8"));

        assert!(prompt.contains("Request: review_goal: Review test-author evidence"));
        assert!(prompt.contains("blocking_question: Is the proof receipt-backed?"));
        assert!(!prompt.trim_end().ends_with("Request:"));

        let _ = std::fs::remove_file(packet_path);
    }

    #[test]
    fn unparsable_external_provider_stdout_cannot_confirm_execution() {
        assert!(!external_provider_output_confirms_execution(None));
    }

    #[test]
    fn parse_internal_codex_exec_output_extracts_last_agent_message() {
        let parsed = parse_internal_codex_exec_output(
            r#"{"type":"thread.started","thread_id":"abc"}
{"type":"item.completed","item":{"id":"1","type":"error","message":"warning"}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"first"}}
{"type":"item.completed","item":{"id":"3","type":"agent_message","text":"final"}}"#,
        );

        assert_eq!(parsed.result_text.as_deref(), Some("final"));
        assert_eq!(parsed.error_messages, vec!["warning".to_string()]);
        assert_eq!(parsed.raw_json.as_array().map(Vec::len), Some(4));
    }

    #[test]
    fn parse_internal_codex_exec_output_extracts_top_level_errors() {
        let parsed = parse_internal_codex_exec_output(
            r#"{"type":"thread.started","thread_id":"abc"}
{"type":"error","message":"You've hit your usage limit. Try again later."}
{"type":"turn.failed","error":{"message":"You've hit your usage limit. Try again later."}}"#,
        );

        assert_eq!(
            parsed.error_messages,
            vec!["You've hit your usage limit. Try again later.".to_string()]
        );
        assert_eq!(parsed.raw_json.as_array().map(Vec::len), Some(3));
        assert_eq!(parsed.result_text, None);
    }

    #[test]
    fn internal_codex_output_requires_clean_error_streams() {
        let parsed_with_error = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"error","message":"warning"}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"final"}}"#,
        );
        assert!(!internal_codex_output_confirms_execution(
            &parsed_with_error,
            "",
            true
        ));

        let parsed_with_unstable_feature_warning = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"error","message":"Under-development features enabled: memories. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set `suppress_unstable_features_warning = true` in config.toml."}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"final"}}"#,
        );
        assert!(
            parsed_with_unstable_feature_warning
                .error_messages
                .is_empty()
        );
        assert!(internal_codex_output_confirms_execution(
            &parsed_with_unstable_feature_warning,
            "",
            true
        ));

        let parsed_clean = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"agent_message","text":"final"}}"#,
        );
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "",
            true
        ));
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "2026-05-12T20:35:57Z WARN codex_core::features: unknown feature key in config: hooks",
            true
        ));
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "2026-05-22T22:05:56Z ERROR codex_core_skills::loader: failed to stat skills path C:\\Users\\pomaz\\.codex\\.tmp\\plugins\\plugins\\google-drive\\skills\\google-slides\\assets\\google-slides-small.svg: The system cannot find the path specified. (os error 3)\n2026-05-22T22:05:56Z ERROR codex_core_skills::loader: failed to read skills dir C:\\Users\\pomaz\\.codex\\.tmp\\plugins\\plugins\\google-drive\\skills\\google-slides\\references: The system cannot find the path specified. (os error 3)",
            true
        ));
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "2026-05-25T07:55:57Z ERROR codex_core::exec: exec error: windows sandbox: spawn setup refresh\n2026-05-25T07:55:57Z ERROR codex_core::tools::router: error=execution error: Io(Custom { kind: Other, error: \"windows sandbox: spawn setup refresh\" })",
            true
        ));
        assert!(!internal_codex_output_confirms_execution(
            &parsed_clean,
            "sandbox denied write to /workspace/secret",
            true
        ));
        assert!(!internal_codex_output_confirms_execution(
            &parsed_clean,
            "",
            false
        ));
    }

    #[test]
    fn internal_host_windows_sandbox_spawn_failure_gets_specific_blocker() {
        let stderr = "2026-05-22T02:27:30Z ERROR codex_core::exec: exec error: windows sandbox: spawn setup refresh";

        assert_eq!(
            super::internal_host_provider_failure_blocker_code(stderr, &[]),
            Some("internal_codex_windows_sandbox_unavailable")
        );
        assert!(
            super::internal_host_provider_failure_blocker_reason(
                "internal_codex_windows_sandbox_unavailable",
                stderr.to_string()
            )
            .contains("configured backend/runtime profile whose sandbox is supported")
        );
    }

    #[test]
    fn internal_codex_usage_limit_failure_gets_specific_blocker() {
        let errors = vec![
            "You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at May 26th, 2026 9:37 PM.".to_string(),
        ];

        assert_eq!(
            super::internal_host_provider_failure_blocker_code("", &errors),
            Some("provider_usage_limit_exceeded")
        );
    }

    #[test]
    fn internal_activation_only_failure_skips_store_backed_activation_render() {
        assert!(!should_render_store_backed_activation_view_for_internal_failure(true, false));
        assert!(should_render_store_backed_activation_view_for_internal_failure(false, false));
    }

    #[test]
    fn configured_internal_host_runtime_env_uses_selected_system_segment() {
        let harness = std::env::temp_dir().join(format!(
            "vida-runtime-dispatch-execution-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&harness).expect("create harness dir");
        let env = configured_internal_host_runtime_env(&harness, "qwen", "worker-a")
            .expect("internal host env");
        let xdg_config_home = env
            .iter()
            .find(|(key, _)| key == "XDG_CONFIG_HOME")
            .map(|(_, value)| value.clone())
            .expect("xdg config home");

        let expected = harness
            .join(".vida")
            .join("data")
            .join("internal-host")
            .join("qwen")
            .join("worker-a")
            .join("config");
        assert_eq!(PathBuf::from(xdg_config_home), expected);
        let _ = std::fs::remove_dir_all(&harness);
    }

    #[test]
    fn downstream_agent_init_backend_truth_detects_downstream_packet_path_for_activation_render() {
        let harness = std::env::temp_dir().join(format!(
            "vida-runtime-dispatch-downstream-detect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&harness).expect("create harness dir");
        let downstream_path = harness.join("downstream.json");
        let dispatch_path = harness.join("dispatch.json");
        std::fs::write(
            &downstream_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "downstream_dispatch_target": "implementer"
            })
            .to_string(),
        )
        .expect("downstream packet should write");
        std::fs::write(
            &dispatch_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "dispatch_target": "specification"
            })
            .to_string(),
        )
        .expect("dispatch packet should write");

        assert!(dispatch_packet_path_should_render_as_downstream(
            downstream_path
                .to_str()
                .expect("downstream path should render")
        ));
        assert!(!dispatch_packet_path_should_render_as_downstream(
            dispatch_path.to_str().expect("dispatch path should render")
        ));
        assert!(!dispatch_packet_path_should_render_as_downstream(
            harness
                .join("missing.json")
                .to_str()
                .expect("missing path should render")
        ));

        let _ = std::fs::remove_dir_all(&harness);
    }

    #[test]
    fn configured_internal_host_activation_parts_use_system_dispatch_config() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  feature_args: ["--enable", "multi_agent"]
  workdir_flag: -C
  sandbox_flag: -s
  model_flag: -m
  reasoning_effort_flag: -c
  reasoning_effort_value_template: 'model_reasoning_effort="{value}"'
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.5",
            "model_reasoning_effort": "high",
            "sandbox_mode": "workspace-write"
        });

        let (command, args, stdin_payload) = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect("internal host activation parts");

        assert_eq!(command, "codex");
        assert_eq!(
            args,
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "--enable".to_string(),
                "multi_agent".to_string(),
                "-C".to_string(),
                "/tmp/project".to_string(),
                "-s".to_string(),
                "workspace-write".to_string(),
                "-m".to_string(),
                "gpt-5.5".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"high\"".to_string(),
                dispatch_packet_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
        assert_eq!(stdin_payload, None);
    }

    #[test]
    fn configured_internal_host_activation_parts_reject_missing_reasoning_effort() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "configured-model",
            "sandbox_mode": "workspace-write"
        });

        let error = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect_err("missing reasoning effort must fail closed");

        assert!(error.contains("missing model_reasoning_effort"));
        assert!(error.contains("missing_reasoning_effort_policy"));
    }

    #[test]
    fn internal_host_tool_bridge_transport_does_not_require_codex_exec_dispatch() {
        let system_entry = serde_yaml::from_str(
            r#"
execution_boundary: parent_host_session
dispatch_transport: host_tool_bridge
receipt_mode: host_bridge_receipt
host_tool_bridge:
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
  spawn_tool: multi_agent_v1.spawn_agent
"#,
        )
        .expect("system entry should parse");

        assert_eq!(
            configured_host_execution_boundary(Some(&system_entry)),
            "parent_host_session"
        );
        assert_eq!(
            configured_host_dispatch_transport(Some(&system_entry)),
            "host_tool_bridge"
        );
        assert_eq!(
            configured_host_receipt_mode(Some(&system_entry)),
            "host_bridge_receipt"
        );
        assert_eq!(
            configured_host_tool_bridge_string(Some(&system_entry), "adapter_kind"),
            Some("codex_host_tools".to_string())
        );
    }

    #[test]
    fn internal_host_dispatch_command_defaults_to_host_tool_bridge_without_explicit_process_transport()
     {
        let system_entry = serde_yaml::from_str(
            r#"
execution_class: internal
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  prompt_mode: stdin
"#,
        )
        .expect("system entry should parse");

        assert_eq!(
            configured_host_dispatch_transport(Some(&system_entry)),
            "host_tool_bridge"
        );
        assert_eq!(
            configured_host_tool_bridge_string(Some(&system_entry), "adapter_kind"),
            Some("codex_host_tools".to_string())
        );
        assert_eq!(
            configured_host_tool_bridge_string(Some(&system_entry), "adapter_capability_id"),
            Some("codex.multi_agent_v1".to_string())
        );
        assert_eq!(
            configured_host_tool_bridge_string(Some(&system_entry), "spawn_tool"),
            Some("multi_agent_v1.spawn_agent".to_string())
        );
    }

    #[test]
    fn explicit_legacy_codex_selection_without_system_entry_uses_builtin_host_bridge_capability() {
        let overlay = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  cli_system: codex
  codex:
    agents:
      junior:
        tier: junior
        rate: 1
        runtime_roles: [developer]
        task_classes: [implementation]
"#,
        )
        .expect("legacy codex overlay should parse");

        let (selected, entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        assert_eq!(selected, "codex");
        let entry = entry.expect("explicit legacy codex selection should synthesize system entry");
        assert_eq!(
            configured_host_tool_bridge_string(Some(&entry), "adapter_capability_id"),
            Some("codex.multi_agent_v1".to_string())
        );
        assert_eq!(
            configured_host_tool_bridge_string(Some(&entry), "spawn_tool"),
            Some("multi_agent_v1.spawn_agent".to_string())
        );
        assert!(
            crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(Some(&entry))
                .iter()
                .any(|row| row["role_id"].as_str() == Some("junior"))
        );
    }

    #[test]
    fn partial_codex_system_selection_materializes_host_tool_bridge_capability() {
        let overlay = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  cli_system: codex
  codex:
    agents:
      junior:
        tier: junior
        rate: 1
        runtime_roles: [developer]
        task_classes: [implementation]
  systems:
    codex:
      enabled: true
      execution_class: internal
      materialization_mode: codex_toml_catalog_render
      runtime_root: .codex
      template_root: .codex
      dispatch:
        command: codex
        static_args: [exec]
      carriers:
        junior:
          tier: junior
          rate: 1
          runtime_roles: [developer]
          task_classes: [implementation]
"#,
        )
        .expect("partial codex overlay should parse");
        let (selected, entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        assert_eq!(selected, "codex");
        let entry = entry.expect("partial codex system should remain selected");
        let project_root = std::env::temp_dir().join("vida-partial-codex-host-bridge");
        let _ = std::fs::remove_dir_all(&project_root);
        std::fs::create_dir_all(project_root.join(".vida"))
            .expect("temp project root should be creatable");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "partial-codex-host-bridge".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(
                project_root
                    .join(".vida/dispatch.json")
                    .display()
                    .to_string(),
            ),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-01T00:00:00Z".to_string(),
        };
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&entry),
            &project_root
                .join(".vida/dispatch.json")
                .display()
                .to_string(),
            "internal_subagents",
            "junior",
            &receipt,
            &internal_codex_fallback_role_selection(serde_json::json!({})),
        )
        .expect("partial codex host bridge request should materialize");

        assert_eq!(request["adapter_kind"], "codex_host_tools");
        assert_eq!(request["adapter_capability_id"], "codex.multi_agent_v1");
        assert_eq!(request["invocation_mode"], "parent_host_tool_api");
        assert_eq!(
            request["adapter_params"]["spawn_tool"],
            "multi_agent_v1.spawn_agent"
        );
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn explicit_codex_cli_exec_transport_preserves_process_dispatch() {
        let system_entry = serde_yaml::from_str(
            r#"
execution_class: internal
dispatch_transport: codex_cli_exec
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  prompt_mode: stdin
"#,
        )
        .expect("system entry should parse");

        assert_eq!(
            configured_host_dispatch_transport(Some(&system_entry)),
            "codex_cli_exec"
        );
    }

    #[test]
    fn host_tool_bridge_request_uses_generic_unconfigured_adapter_defaults() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-defaults-{}-{nanos}",
            std::process::id()
        ));
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-host-bridge".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/project/.vida/dispatch.json".to_string()),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-01T00:00:00Z".to_string(),
        };

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            &project_root
                .join(".vida/dispatch.json")
                .display()
                .to_string(),
            "internal_subagents",
            "junior",
            &receipt,
            &internal_codex_fallback_role_selection(serde_json::json!({})),
        )
        .expect("host bridge request should materialize");

        assert_eq!(request["adapter_kind"], "unconfigured_host_agent_adapter");
        assert_eq!(
            request["adapter_capability_id"],
            "unconfigured_host_agent_capability"
        );
        assert_eq!(
            request["invocation_mode"],
            "configured_host_capability_required"
        );
        assert!(request.get("spawn_tool").is_none());
        assert!(request["adapter_params"].get("spawn_tool").is_some());
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn configured_host_tool_bridge_dir_accepts_state_root_subdirectories() {
        let project_root = std::env::temp_dir().join("vida-host-bridge-configured-dir");
        let state_root = project_root.join(".vida/data/state-alt");
        let configured = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_tool_bridge:
  result_dir: .vida/data/state/custom-agent-bridge/results
"#,
        )
        .expect("parse host bridge config");

        let resolved = configured_host_tool_bridge_dir(
            &project_root,
            &state_root,
            Some(&configured),
            "result_dir",
            "host-tool-bridge/results",
        );

        assert_eq!(resolved, state_root.join("custom-agent-bridge/results"));
    }

    #[test]
    fn configured_host_tool_bridge_dir_rejects_paths_outside_state_root() {
        let project_root = std::env::temp_dir().join("vida-host-bridge-dir-guard");
        let state_root = project_root.join(".vida/data/state-alt");
        let configured = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_tool_bridge:
  result_dir: C:\tmp\vida-host-bridge-escape
"#,
        )
        .expect("parse host bridge config");

        let resolved = configured_host_tool_bridge_dir(
            &project_root,
            &state_root,
            Some(&configured),
            "result_dir",
            "host-tool-bridge/results",
        );

        assert_eq!(resolved, state_root.join("host-tool-bridge/results"));
    }

    #[test]
    fn host_tool_bridge_request_sanitizes_dispatch_target_before_path_join() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-sanitize-target-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(&dispatch_packet_path, r#"{"owned_paths":["src/lib.rs"]}"#)
            .expect("write dispatch packet");

        let mut receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        receipt.run_id = "../../../../poc-outside-run".to_string();
        receipt.dispatch_target = "../../../../poc-outside-state/leaf".to_string();
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &internal_codex_fallback_role_selection(serde_json::json!({})),
        )
        .expect("host bridge request should sanitize dispatch target and materialize");

        let request_id = request["request_id"]
            .as_str()
            .expect("request id should render");
        let request_path = PathBuf::from(
            request["request_path"]
                .as_str()
                .expect("request path should render"),
        );
        let result_path = PathBuf::from(
            request["result_path"]
                .as_str()
                .expect("result path should render"),
        );
        let receipt_path = PathBuf::from(
            request["receipt_path"]
                .as_str()
                .expect("receipt path should render"),
        );
        assert!(!request_id.contains('/'));
        assert!(!request_id.contains('\\'));
        assert!(request_id.contains("poc-outside-run"));
        assert!(request_id.contains("poc-outside-state-leaf"));
        assert!(request_path.starts_with(&state_root));
        assert!(result_path.starts_with(&state_root));
        assert!(receipt_path.starts_with(&state_root));
        assert!(
            !project_root.join(".vida/data/poc-outside-state").exists(),
            "malicious dispatch target must not create directories outside the state root"
        );
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_tool_bridge_request_falls_back_to_default_segments_when_sanitized_segments_are_empty() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-empty-segment-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        let dispatch_packet_path = project_root.join(".vida/packet.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(&dispatch_packet_path, r#"{"owned_paths":["src/lib.rs"]}"#)
            .expect("write dispatch packet");

        let mut receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        receipt.run_id = "///".to_string();
        receipt.dispatch_target = "???".to_string();
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &internal_codex_fallback_role_selection(serde_json::json!({})),
        )
        .expect("host bridge request should fall back to default request-id segments");

        let request_id = request["request_id"]
            .as_str()
            .expect("request id should render");
        let request_path = PathBuf::from(
            request["request_path"]
                .as_str()
                .expect("request path should render"),
        );
        let result_path = PathBuf::from(
            request["result_path"]
                .as_str()
                .expect("result path should render"),
        );
        let receipt_path = PathBuf::from(
            request["receipt_path"]
                .as_str()
                .expect("receipt path should render"),
        );

        assert!(
            request_id.starts_with("run-dispatch-target-"),
            "expected empty sanitized run_id and dispatch_target to fall back to run-dispatch-target, got {request_id}"
        );
        assert!(request_path.starts_with(&state_root));
        assert!(result_path.starts_with(&state_root));
        assert!(receipt_path.starts_with(&state_root));
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_tool_bridge_request_fails_when_stale_request_file_does_not_match_lane() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-stale-request-{}-{nanos}",
            std::process::id()
        ));
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-host-bridge".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(
                project_root
                    .join(".vida/dispatch.json")
                    .display()
                    .to_string(),
            ),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-01T00:00:00Z".to_string(),
        };
        let stale_request_path = host_tool_bridge_artifact_paths(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            &host_tool_bridge_request_id(
                &receipt,
                dispatch_packet_path
                    .to_str()
                    .expect("dispatch packet path should render"),
            ),
        )
        .request_path;
        std::fs::create_dir_all(stale_request_path.parent().expect("stale request parent"))
            .expect("create stale request parent");
        std::fs::write(&stale_request_path, "{}").expect("write stale request");

        let error = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "junior",
            &receipt,
            &internal_codex_fallback_role_selection(serde_json::json!({})),
        )
        .expect_err("mismatched stale request file should fail closed");

        assert!(
            error.contains("does not match the active dispatch lane"),
            "unexpected error: {error}"
        );
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn retryable_blocked_host_bridge_request_can_be_rearmed_but_activation_only_cannot() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-retryable-blocked-request-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        let result_path = project_root.join(".vida/data/state/host-tool-bridge/result.json");
        let receipt_path = project_root.join(".vida/data/state/host-tool-bridge/receipt.json");
        std::fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create host bridge artifact directory");
        let expected = serde_json::json!({
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        let mut existing = expected.clone();
        existing["status"] = serde_json::json!("blocked");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "status": "blocked",
                "blocker_code": "host_tool_bridge_adapter_required"
            })
            .to_string(),
        )
        .expect("write retryable blocked result");
        std::fs::write(
            &receipt_path,
            serde_json::json!({
                "status": "blocked",
                "blocker_code": "host_tool_bridge_adapter_required"
            })
            .to_string(),
        )
        .expect("write retryable blocked receipt");

        assert!(super::existing_host_bridge_request_has_retryable_completion_evidence(
            &existing, &expected
        ));
        assert!(super::existing_host_bridge_request_needs_retryable_blocked_refresh(
            &existing, &expected
        ));

        std::fs::write(
            &result_path,
            serde_json::json!({
                "status": "blocked",
                "blocker_code": "activation_view_only"
            })
            .to_string(),
        )
        .expect("write activation-only result");
        std::fs::write(
            &receipt_path,
            serde_json::json!({
                "status": "blocked",
                "blocker_code": "activation_view_only"
            })
            .to_string(),
        )
        .expect("write activation-only receipt");
        assert!(!super::existing_host_bridge_request_has_retryable_completion_evidence(
            &existing, &expected
        ));
        assert!(!super::existing_host_bridge_request_needs_retryable_blocked_refresh(
            &existing, &expected
        ));

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_tool_bridge_materialize_rearms_retryable_blocked_request() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-materialize-retry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        let state_root = project_root.join(".vida/data/state");
        let packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({"owned_paths": ["src/lib.rs"]}).to_string(),
        )
        .expect("write dispatch packet");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));
        let receipt = internal_codex_fallback_receipt(
            packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let first_request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            None,
            packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("initial host bridge request should materialize");
        let request_path = PathBuf::from(
            first_request["request_path"]
                .as_str()
                .expect("request path should render"),
        );
        let result_path = PathBuf::from(
            first_request["result_path"]
                .as_str()
                .expect("result path should render"),
        );
        let receipt_path = PathBuf::from(
            first_request["receipt_path"]
                .as_str()
                .expect("receipt path should render"),
        );
        let mut blocked_request = first_request.clone();
        blocked_request["status"] = serde_json::json!("blocked");
        blocked_request["owned_paths"] = serde_json::json!(["src/stale.rs"]);
        std::fs::write(
            &request_path,
            serde_json::to_string_pretty(&blocked_request).expect("encode blocked request"),
        )
        .expect("write blocked request");
        let blocked_artifact = serde_json::json!({
            "status": "blocked",
            "blocker_code": "host_tool_bridge_adapter_required"
        });
        std::fs::write(
            &result_path,
            blocked_artifact.to_string(),
        )
        .expect("write blocked result");
        std::fs::write(&receipt_path, blocked_artifact.to_string())
            .expect("write blocked receipt");
        assert!(result_path.exists());
        assert!(super::host_bridge_artifact_has_retryable_completion_blocker(
            &blocked_artifact
        ));
        assert!(crate::read_json_file_if_present(&result_path).is_some());
        assert!(super::host_bridge_request_value_matches(
            &blocked_request,
            &first_request,
            "result_path"
        ));
        assert!(super::host_bridge_request_value_matches(
            &blocked_request,
            &first_request,
            "receipt_path"
        ));
        assert!(super::existing_host_bridge_request_has_retryable_completion_evidence(
            &blocked_request, &first_request
        ));
        assert!(super::existing_host_bridge_request_needs_retryable_blocked_refresh(
            &blocked_request, &first_request
        ));

        let rearmed_request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            None,
            packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("retryable blocked request should rearm");

        assert_eq!(rearmed_request["status"], "pending");
        assert_eq!(rearmed_request["owned_paths"], serde_json::json!(["src/lib.rs"]));
        assert!(!result_path.exists(), "old blocked result must not be reused");
        assert!(!receipt_path.exists(), "old blocked receipt must not be reused");
        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_projects_configured_rework_contract() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-rework-contract-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        let state_root = project_root.join(".vida/data/state");
        let packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({"owned_paths": ["src/lib.rs"]}).to_string(),
        )
        .expect("write dispatch packet");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_sequence": ["coder", "tester"],
                    "execution_lane_sequence": ["coder", "tester"],
                    "lane_catalog": {
                        "coder": {
                            "runtime_role": "worker",
                            "task_class": "implementation",
                            "stage": "execution",
                            "rework_transitions": {"rework": "coder"}
                        },
                        "tester": {
                            "runtime_role": "verifier",
                            "task_class": "verification",
                            "stage": "execution"
                        }
                    }
                }
            }
        }));
        let mut receipt = internal_codex_fallback_receipt(
            packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        receipt.dispatch_target = "coder".to_string();
        receipt.activation_runtime_role = Some("worker".to_string());

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            None,
            packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "junior",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");

        assert_eq!(
            request["blocked_result_contract"]["allowed_next_node"],
            "coder"
        );
        assert_eq!(
            request["blocked_result_contract"]["rework_target_required_when_blocked"],
            true
        );
        assert_eq!(
            request["blocked_result_contract"]["allowed_blocker_codes"][2],
            "host_agent_execution_failed"
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_tool_bridge_request_uses_distinct_paths_for_repeated_target_packets() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-rotate-request-{}-{nanos}",
            std::process::id()
        ));
        let first_packet_path = project_root.join(".vida/dispatch-a.json");
        let second_packet_path = project_root.join(".vida/dispatch-b.json");
        std::fs::create_dir_all(first_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(&first_packet_path, r#"{"owned_paths":["first.rs"]}"#)
            .expect("write first dispatch packet");
        std::fs::write(&second_packet_path, r#"{"owned_paths":["second.rs"]}"#)
            .expect("write second dispatch packet");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));
        let mut receipt = internal_codex_fallback_receipt(
            first_packet_path
                .to_str()
                .expect("first packet path should render"),
        );
        receipt.dispatch_target = "coach".to_string();

        let first_request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            first_packet_path
                .to_str()
                .expect("first packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("first bridge request should materialize");
        let result_path = PathBuf::from(
            first_request["result_path"]
                .as_str()
                .expect("result path should render"),
        );
        let receipt_path = PathBuf::from(
            first_request["receipt_path"]
                .as_str()
                .expect("receipt path should render"),
        );
        std::fs::write(&result_path, "{}").expect("write stale result");
        std::fs::write(&receipt_path, "{}").expect("write stale receipt");

        let second_request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            second_packet_path
                .to_str()
                .expect("second packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("second bridge request should materialize with a distinct packet id");

        assert_eq!(
            second_request["packet_path"],
            second_packet_path
                .to_str()
                .expect("second packet path should render")
        );
        assert_eq!(second_request["owned_paths"][0], "second.rs");
        assert_ne!(first_request["request_id"], second_request["request_id"]);
        assert_ne!(
            first_request["request_path"],
            second_request["request_path"]
        );
        assert!(
            first_request["request_id"]
                .as_str()
                .expect("first request id should render")
                .contains("dispatch-a")
        );
        assert!(
            second_request["request_id"]
                .as_str()
                .expect("second request id should render")
                .contains("dispatch-b")
        );
        assert!(
            result_path.exists(),
            "first result path should remain owned by first packet"
        );
        assert!(
            receipt_path.exists(),
            "first receipt path should remain owned by first packet"
        );
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn internal_host_tool_bridge_ingests_parent_written_receipt_backed_result() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-completed-result-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("create state root");
        let dispatch_packet_path =
            project_root.join(".vida/dispatch & echo CMD_INJECTION_POC &.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"],"proof_target":"cargo test -p vida host_bridge -- --nocapture"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: host_tool_bridge
      receipt_mode: host_bridge_receipt
      host_tool_bridge:
        adapter_kind: codex_host_tools
        adapter_capability_id: codex.multi_agent_v1
        invocation_mode: parent_host_tool_api
      carriers:
        middle:
          model: gpt-5.5
          model_reasoning_effort: medium
          sandbox_mode: read-only
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: openai
          model_ref: gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
"#,
        )
        .expect("write overlay");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
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
                "analysis": {
                    "executor_backend": "internal_subagents"
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "internal_fast"
            }
        }));
        let overlay =
            crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root)
                .expect("project overlay should load");
        let (selected_cli_system, selected_cli_entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        assert_eq!(selected_cli_system, "codex");
        let selected_cli_entry = selected_cli_entry.expect("codex cli entry should be selected");
        let receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&selected_cli_entry),
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");
        let result_path = PathBuf::from(
            request["result_path"]
                .as_str()
                .expect("request result path should render"),
        );
        let receipt_path = PathBuf::from(
            request["receipt_path"]
                .as_str()
                .expect("request receipt path should render"),
        );
        let request_id = request["request_id"]
            .as_str()
            .expect("request id should render");
        let request_path = request["request_path"]
            .as_str()
            .expect("request path should render");
        let packet_path = request["packet_path"]
            .as_str()
            .expect("packet path should render");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "pass",
                "execution_state": "executed",
                "decision": "approve",
                "verdict": "pass",
                "blocker_codes": [],
                "rework_target": serde_json::Value::Null,
                "allowed_next_node": "closure",
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-completion-test",
                "source_dispatch_packet_path": packet_path,
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                },
                "execution_evidence": {
                    "status": "recorded",
                    "backend_id": "internal_subagents",
                    "receipt_backed": true
                }
            }))
            .expect("encode host bridge result"),
        )
        .expect("write host bridge result");
        std::fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "schema_version": 1,
                "status": "pass",
                "receipt_backed": true,
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-completion-test",
                "request_path": request_path,
                "result_path": result_path.display().to_string(),
                "source_dispatch_packet_path": packet_path
            }))
            .expect("encode host bridge receipt"),
        )
        .expect("write host bridge receipt");
        let mut completed_request = request.clone();
        let completed_request_body = completed_request
            .as_object_mut()
            .expect("request should be an object");
        completed_request_body.insert("status".to_string(), serde_json::json!("completed"));
        completed_request_body.insert(
            "completion_receipt_id".to_string(),
            serde_json::json!("host-bridge-completion-test"),
        );
        std::fs::write(
            request_path,
            serde_json::to_string_pretty(&completed_request).expect("encode completed request"),
        )
        .expect("write completed bridge request");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    &state_root,
                    &project_root,
                    dispatch_packet_path
                        .to_str()
                        .expect("dispatch packet path should render"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal",
                        "selected_cli_system": "codex"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("completed bridge result should return execution evidence");

        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["decision"], "approve");
        assert_eq!(result["verdict"], "pass");
        assert_eq!(result["blocker_codes"], serde_json::json!([]));
        assert_eq!(result["blocker_code"], serde_json::Value::Null);
        assert_eq!(result["execution_evidence"]["receipt_backed"], true);
        assert_eq!(
            result["backend_dispatch"]["dispatch_transport"],
            "host_tool_bridge"
        );
        assert_eq!(
            result["backend_dispatch"]["backend_id"],
            "internal_subagents"
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn internal_host_tool_bridge_preserves_blocked_coach_product_rework_result() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-coach-blocked-result-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("create state root");
        let dispatch_packet_path = project_root.join(".vida/coach-dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"],"proof_target":"cargo test -p vida internal_host_tool_bridge_preserves_blocked_coach_product_rework_result"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: host_tool_bridge
      receipt_mode: host_bridge_receipt
      host_tool_bridge:
        adapter_kind: codex_host_tools
        adapter_capability_id: codex.multi_agent_v1
        invocation_mode: parent_host_tool_api
      carriers:
        middle:
          model: gpt-5.5
          model_reasoning_effort: medium
          sandbox_mode: read-only
          runtime_roles: [coach]
          task_classes: [coach]
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: coach_fast
      model_profiles:
        coach_fast:
          provider: openai
          model_ref: gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [coach]
          task_classes: [coach]
"#,
        )
        .expect("write overlay");
        let mut role_selection = internal_codex_fallback_role_selection(serde_json::json!({
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
                "coach": {
                    "executor_backend": "internal_subagents"
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "coach_fast"
            }
        }));
        role_selection.selected_role = "coach".to_string();
        let overlay =
            crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root)
                .expect("project overlay should load");
        let (selected_cli_system, selected_cli_entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        assert_eq!(selected_cli_system, "codex");
        let selected_cli_entry = selected_cli_entry.expect("codex cli entry should be selected");
        let mut receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        receipt.dispatch_target = "coach".to_string();
        receipt.activation_runtime_role = Some("coach".to_string());
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&selected_cli_entry),
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");
        let result_path = PathBuf::from(
            request["result_path"]
                .as_str()
                .expect("request result path should render"),
        );
        let receipt_path = PathBuf::from(
            request["receipt_path"]
                .as_str()
                .expect("request receipt path should render"),
        );
        let request_id = request["request_id"]
            .as_str()
            .expect("request id should render");
        let request_path = request["request_path"]
            .as_str()
            .expect("request path should render");
        let packet_path = request["packet_path"]
            .as_str()
            .expect("packet path should render");
        let blocker_text =
            "coach decision=blocked; Meeting scheduledAt missing for non-all-day meeting";
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "blocked",
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-coach-blocked-test",
                "blocker_code": "coach_rework_required",
                "blocker_codes": ["coach_rework_required"],
                "rework_target": "developer",
                "allowed_next_node": "developer_rework",
                "summary": blocker_text,
                "blocker_details": [{
                    "code": "coach_rework_required",
                    "message": blocker_text,
                    "completed_target": "coach"
                }],
                "source_dispatch_packet_path": packet_path,
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                },
                "execution_evidence": {
                    "status": "recorded",
                    "backend_id": "internal_subagents",
                    "receipt_backed": true
                }
            }))
            .expect("encode blocked host bridge result"),
        )
        .expect("write blocked host bridge result");
        std::fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "schema_version": 1,
                "status": "pass",
                "receipt_backed": true,
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-coach-blocked-test",
                "request_path": request_path,
                "result_path": result_path.display().to_string(),
                "source_dispatch_packet_path": packet_path
            }))
            .expect("encode host bridge receipt"),
        )
        .expect("write host bridge receipt");
        let mut completed_request = request.clone();
        let completed_request_body = completed_request
            .as_object_mut()
            .expect("request should be an object");
        completed_request_body.insert("status".to_string(), serde_json::json!("completed"));
        completed_request_body.insert(
            "completion_receipt_id".to_string(),
            serde_json::json!("host-bridge-coach-blocked-test"),
        );
        std::fs::write(
            request_path,
            serde_json::to_string_pretty(&completed_request).expect("encode completed request"),
        )
        .expect("write completed bridge request");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    &state_root,
                    &project_root,
                    dispatch_packet_path
                        .to_str()
                        .expect("dispatch packet path should render"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal",
                        "selected_cli_system": "codex"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("blocked bridge result should return execution evidence");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["decision"], "rework_required");
        assert_eq!(result["verdict"], "rework_required");
        assert_eq!(result["blocker_code"], "coach_rework_required");
        assert_eq!(
            result["blocker_codes"],
            serde_json::json!(["coach_rework_required"])
        );
        assert_eq!(result["rework_target"], "developer");
        assert_eq!(result["allowed_next_node"], "developer_rework");
        assert_ne!(result["allowed_next_node"], "verification");
        assert_eq!(result["summary"], blocker_text);
        assert_eq!(result["blocker_details"][0]["message"], blocker_text);
        assert_eq!(result["execution_evidence"]["receipt_backed"], true);
        assert_eq!(
            result["backend_dispatch"]["dispatch_transport"],
            "host_tool_bridge"
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_bridge_rejects_parent_result_missing_structured_verdict_fields() {
        let receipt = internal_codex_fallback_receipt("packet.json");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "completed",
            "request_id": "req-1",
            "run_id": receipt.run_id,
            "task_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json",
            "completion_receipt_id": "completion-1",
            "required_result_fields": taskflow_host_bridge::default_host_bridge_required_result_fields()
        });
        let result = serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "pass",
            "execution_state": "executed",
            "request_id": "req-1",
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "completion_receipt_id": "completion-1",
            "source_dispatch_packet_path": "packet.json",
            "execution_evidence": {
                "status": "recorded",
                "backend_id": "internal_subagents",
                "receipt_backed": true
            }
        });
        let bridge_receipt = serde_json::json!({
            "artifact_kind": "host_tool_bridge_receipt",
            "schema_version": 1,
            "status": "pass",
            "receipt_backed": true,
            "request_id": "req-1",
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "completion_receipt_id": "completion-1",
            "request_path": "request.json",
            "result_path": "result.json",
            "source_dispatch_packet_path": "packet.json"
        });

        let error = super::validate_completed_host_bridge_artifacts(
            &request,
            &result,
            &bridge_receipt,
            &PathBuf::from("result.json"),
            &PathBuf::from("receipt.json"),
            &receipt,
            "internal_subagents",
        )
        .expect_err("missing structured verdict fields must fail closed");

        assert!(
            error.contains("Host bridge result verdict contract failed"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("host_bridge_result_missing_verdict_field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn internal_host_tool_bridge_pending_result_preserves_execute_dispatch_mode() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-pending-result-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("create state root");
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"],"proof_target":"cargo test -p vida internal_host_tool_bridge_pending_result_preserves_execute_dispatch_mode"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: host_tool_bridge
      receipt_mode: host_bridge_receipt
      host_tool_bridge:
        adapter_kind: codex_host_tools
        adapter_capability_id: codex.multi_agent_v1
        invocation_mode: parent_host_tool_api
      carriers:
        middle:
          model: gpt-5.5
          model_reasoning_effort: medium
          sandbox_mode: read-only
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: openai
          model_ref: gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
"#,
        )
        .expect("write overlay");
        let mut role_selection = internal_codex_fallback_role_selection(serde_json::json!({
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
                "analysis": {
                    "executor_backend": "internal_subagents"
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "internal_fast"
            }
        }));
        role_selection.compiled_bundle = serde_json::json!({
            "large_default_output_regression_guard": "x".repeat(200_000)
        });
        role_selection.execution_plan["large_default_output_regression_guard"] =
            serde_json::json!("y".repeat(200_000));
        let receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_internal_agent_lane_dispatch(
                    &state_root,
                    &project_root,
                    dispatch_packet_path
                        .to_str()
                        .expect("dispatch packet path should render"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal",
                        "selected_cli_system": "codex"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("pending bridge result should be returned");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "bridge_request_pending");
        assert_eq!(result["blocker_code"], "host_tool_bridge_adapter_required");
        assert_eq!(result["dispatch_mode"]["mode"], "execution_dispatch");
        assert_eq!(result["dispatch_mode"]["requested_execute_dispatch"], true);
        assert_eq!(result["dispatch_mode"]["execution_dispatch"], true);
        assert_eq!(result["dispatch_mode"]["activation_view_only"], false);
        assert_eq!(
            result["dispatch_mode"]["required_completion_evidence"],
            "host_tool_bridge_receipt"
        );
        assert_eq!(
            result["backend_dispatch"]["host_tool_bridge_request"]["status"],
            "pending"
        );
        assert_eq!(
            result["host_tool_bridge_request"]["compact_projection"],
            true
        );
        assert_eq!(result["host_tool_bridge_request"]["owned_paths_count"], 1);
        assert!(result.get("selection").is_none());
        assert!(result.get("role_selection").is_none());
        assert!(result.get("dev_team_readiness").is_none());
        assert_eq!(
            result["role_selection_summary"]["dispatch_target"],
            receipt.dispatch_target
        );
        let serialized = serde_json::to_vec_pretty(&result)
            .expect("pending bridge result should serialize compactly");
        assert!(
            serialized.len() < 64 * 1024,
            "pending bridge result should stay compact; got {} bytes",
            serialized.len()
        );
        for request in [
            &result["host_tool_bridge_request"],
            &result["backend_dispatch"]["host_tool_bridge_request"],
        ] {
            assert_eq!(request["adapter_kind"], "codex_host_tools");
            assert_eq!(request["adapter_capability_id"], "codex.multi_agent_v1");
            assert_eq!(request["invocation_mode"], "parent_host_tool_api");
        }
        assert_eq!(
            result["host_bridge_adapter_command"],
            serde_json::Value::Null
        );
        assert_eq!(
            result["backend_dispatch"]["host_bridge_adapter_command"],
            serde_json::Value::Null
        );
        let adapter_argv = result["host_bridge_adapter_argv"]
            .as_array()
            .expect("adapter argv should render");
        assert_eq!(adapter_argv[0], "vida");
        assert_eq!(adapter_argv[1], "agent");
        assert_eq!(adapter_argv[2], "host-bridge");
        assert_eq!(adapter_argv[3], "--request");
        assert_eq!(
            adapter_argv[4]
                .as_str()
                .expect("request path arg should render"),
            result["host_tool_bridge_request"]["request_path"]
                .as_str()
                .expect("request path should render")
        );
        assert_eq!(adapter_argv[5], "--json");
        assert_eq!(
            result["backend_dispatch"]["host_bridge_adapter_argv"]
                .as_array()
                .expect("backend adapter argv should render"),
            adapter_argv
        );
        assert!(
            result["next_actions"]
                .as_array()
                .expect("next actions should render")
                .iter()
                .all(|action| !action
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("vida agent host-bridge"))
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_bridge_rejects_failed_parent_result() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-failed-result-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("create state root");
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"]}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: host_tool_bridge
      receipt_mode: host_bridge_receipt
      host_tool_bridge:
        adapter_kind: codex_host_tools
        adapter_capability_id: codex.multi_agent_v1
        invocation_mode: parent_host_tool_api
      carriers:
        middle:
          model: gpt-5.5
          model_reasoning_effort: medium
          sandbox_mode: read-only
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: openai
          model_ref: gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
"#,
        )
        .expect("write overlay");
        let role_selection =
            internal_codex_fallback_role_selection(serde_json::json!({ "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "internal_fast"
            }}));
        let overlay =
            crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root)
                .expect("project overlay should load");
        let (selected_cli_system, selected_cli_entry) =
            crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
        assert_eq!(selected_cli_system, "codex");
        let selected_cli_entry = selected_cli_entry.expect("codex cli entry should be selected");
        let receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&selected_cli_entry),
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");
        let result_path = PathBuf::from(
            request["result_path"]
                .as_str()
                .expect("request result path should render"),
        );
        let receipt_path = PathBuf::from(
            request["receipt_path"]
                .as_str()
                .expect("request receipt path should render"),
        );
        let request_id = request["request_id"]
            .as_str()
            .expect("request id should render");
        let request_path = request["request_path"]
            .as_str()
            .expect("request path should render");
        let packet_path = request["packet_path"]
            .as_str()
            .expect("packet path should render");
        std::fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "fail",
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-completion-test",
                "blocker_codes": ["host_agent_execution_failed"],
                "rework_target": "developer",
                "allowed_next_node": "developer_rework",
                "source_dispatch_packet_path": packet_path,
                "execution_evidence": {
                    "status": "failed",
                    "backend_id": "internal_subagents",
                    "receipt_backed": false
                }
            }))
            .expect("encode failed host bridge result"),
        )
        .expect("write failed host bridge result");
        std::fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "schema_version": 1,
                "status": "pass",
                "receipt_backed": true,
                "request_id": request_id,
                "run_id": receipt.run_id,
                "dispatch_target": receipt.dispatch_target,
                "completion_receipt_id": "host-bridge-completion-test",
                "request_path": request_path,
                "result_path": result_path.display().to_string(),
                "source_dispatch_packet_path": packet_path
            }))
            .expect("encode host bridge receipt"),
        )
        .expect("write host bridge receipt");
        let mut completed_request = request.clone();
        let completed_request_body = completed_request
            .as_object_mut()
            .expect("request should be an object");
        completed_request_body.insert("status".to_string(), serde_json::json!("completed"));
        completed_request_body.insert(
            "completion_receipt_id".to_string(),
            serde_json::json!("host-bridge-completion-test"),
        );
        std::fs::write(
            request_path,
            serde_json::to_string_pretty(&completed_request).expect("encode completed request"),
        )
        .expect("write completed bridge request");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let error = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    &state_root,
                    &project_root,
                    dispatch_packet_path
                        .to_str()
                        .expect("dispatch packet path should render"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal",
                        "selected_cli_system": "codex"
                    }),
                )
                .await
            })
            .expect_err("failed parent result must fail closed");

        assert!(
            error.contains("Host bridge result status is not pass"),
            "unexpected error: {error}"
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_bridge_rejects_existing_request_with_forged_backend_identity() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-forged-request-{}-{nanos}",
            std::process::id()
        ));
        let state_root = project_root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("create state root");
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"]}"#,
        )
        .expect("write dispatch packet");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "internal_fast"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");
        let mut forged_request = request.clone();
        forged_request
            .as_object_mut()
            .expect("request should be an object")
            .insert(
                "backend_id".to_string(),
                serde_json::json!("attacker_backend_not_internal_subagents"),
            );
        let request_path = PathBuf::from(
            request["request_path"]
                .as_str()
                .expect("request path should render"),
        );
        std::fs::write(
            &request_path,
            serde_json::to_string_pretty(&forged_request).expect("encode forged request"),
        )
        .expect("overwrite request with forged backend");

        let error = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect_err("forged same-lane request must fail closed");

        assert!(
            error.contains("does not match expected `backend_id`"),
            "unexpected error: {error}"
        );
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_bridge_refreshes_legacy_unconfigured_same_lane_request_when_codex_defaults_available() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-refresh-legacy-request-{}-{nanos}",
            std::process::id()
        ));
        let dispatch_packet_path = project_root.join(".vida/dispatch.json");
        std::fs::create_dir_all(dispatch_packet_path.parent().expect("dispatch parent"))
            .expect("create dispatch parent");
        std::fs::write(
            &dispatch_packet_path,
            r#"{"owned_paths":["crates/vida/src/runtime_dispatch_execution.rs"]}"#,
        )
        .expect("write dispatch packet");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle",
                "selected_model_profile_id": "internal_fast"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
        );
        let stale_request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            None,
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("legacy unconfigured host bridge request should materialize");
        assert_eq!(
            stale_request["adapter_capability_id"],
            "unconfigured_host_agent_capability"
        );
        let request_path = PathBuf::from(
            stale_request["request_path"]
                .as_str()
                .expect("request path should render"),
        );
        let result_path = PathBuf::from(
            stale_request["result_path"]
                .as_str()
                .expect("result path should render"),
        );
        let receipt_path = PathBuf::from(
            stale_request["receipt_path"]
                .as_str()
                .expect("receipt path should render"),
        );
        std::fs::write(&result_path, "{}").expect("write stale result");
        std::fs::write(&receipt_path, "{}").expect("write stale receipt");
        let legacy_codex_entry = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
execution_class: internal
dispatch:
  command: codex
"#,
        )
        .expect("parse legacy codex entry");

        let refreshed_request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&legacy_codex_entry),
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("same-lane legacy request should refresh to configured codex host bridge");

        assert_eq!(refreshed_request["adapter_kind"], "codex_host_tools");
        assert_eq!(
            refreshed_request["adapter_capability_id"],
            "codex.multi_agent_v1"
        );
        assert_eq!(refreshed_request["invocation_mode"], "parent_host_tool_api");
        assert_eq!(
            refreshed_request["adapter_params"]["spawn_tool"],
            "multi_agent_v1.spawn_agent"
        );
        let persisted_request =
            std::fs::read_to_string(&request_path).expect("refreshed request should be persisted");
        let persisted_request = serde_json::from_str::<serde_json::Value>(&persisted_request)
            .expect("persisted request should decode");
        assert_eq!(
            persisted_request["adapter_capability_id"],
            "codex.multi_agent_v1"
        );
        assert!(!result_path.exists());
        assert!(!receipt_path.exists());
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn host_bridge_rejects_receipt_result_path_not_bound_to_request_result() {
        let receipt = internal_codex_fallback_receipt("/tmp/dispatch.json");
        let request = serde_json::json!({
            "schema_version": 1,
            "request_id": "run-target-dispatch-host-tool-bridge",
            "run_id": receipt.run_id,
            "task_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "/tmp/.vida/data/state/host-bridge/requests/request.json",
            "result_path": "/tmp/.vida/data/state/host-bridge/results/result.json",
            "receipt_path": "/tmp/.vida/data/state/host-bridge/receipts/receipt.json",
            "packet_path": "/tmp/dispatch.json",
            "status": "completed",
            "completion_receipt_id": "host-bridge-completion-test",
            "required_result_fields": taskflow_host_bridge::default_host_bridge_required_result_fields()
        });
        let result = serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "pass",
            "execution_state": "executed",
            "decision": "approve",
            "verdict": "pass",
            "blocker_codes": [],
            "rework_target": serde_json::Value::Null,
            "allowed_next_node": "closure",
            "request_id": "run-target-dispatch-host-tool-bridge",
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "completion_receipt_id": "host-bridge-completion-test",
            "source_dispatch_packet_path": "/tmp/dispatch.json",
            "execution_evidence": {
                "status": "recorded",
                "backend_id": "internal_subagents",
                "receipt_backed": true
            }
        });
        let bridge_receipt = serde_json::json!({
            "artifact_kind": "host_tool_bridge_receipt",
            "schema_version": 1,
            "status": "pass",
            "receipt_backed": true,
            "request_id": "run-target-dispatch-host-tool-bridge",
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "completion_receipt_id": "host-bridge-completion-test",
            "request_path": "/tmp/.vida/data/state/host-bridge/requests/request.json",
            "result_path": "/tmp/.vida/data/state/host-bridge/results/attacker-result.json",
            "source_dispatch_packet_path": "/tmp/dispatch.json"
        });

        let error = super::validate_completed_host_bridge_artifacts(
            &request,
            &result,
            &bridge_receipt,
            Path::new("/tmp/.vida/data/state/host-bridge/results/result.json"),
            Path::new("/tmp/.vida/data/state/host-bridge/receipts/receipt.json"),
            &receipt,
            "internal_subagents",
        )
        .expect_err("receipt result path must be bound to the active request result");

        assert!(
            error.contains("receipt result_path does not match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn configured_internal_host_activation_parts_support_stdin_prompt_mode() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  workdir_flag: -C
  sandbox_flag: -s
  model_flag: -m
  reasoning_effort_flag: -c
  reasoning_effort_value_template: 'model_reasoning_effort="{value}"'
  prompt_mode: stdin
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.5",
            "model_reasoning_effort": "high",
            "sandbox_mode": "workspace-write"
        });

        let (command, args, stdin_payload) = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect("internal host activation parts");

        assert_eq!(command, "codex");
        assert_eq!(
            args,
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "-C".to_string(),
                "/tmp/project".to_string(),
                "-s".to_string(),
                "workspace-write".to_string(),
                "-m".to_string(),
                "gpt-5.5".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"high\"".to_string(),
                "-".to_string(),
            ]
        );
        assert_eq!(
            stdin_payload.as_deref(),
            Some(dispatch_packet_prompt("/tmp/project/.vida/dispatch.json").as_str())
        );
    }

    #[test]
    fn configured_internal_host_activation_parts_can_use_provider_local_model_arg() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  model_flag: -m
  model_arg_transform: provider_local_name
  prompt_mode: stdin
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "openai-codex/gpt-5.5",
            "model_reasoning_effort": "high",
            "sandbox_mode": "read-only"
        });

        let (_command, args, _stdin_payload) = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect("internal host activation parts");

        assert!(args.windows(2).any(|pair| pair == ["-m", "gpt-5.5"]));
        assert!(!args.iter().any(|arg| arg == "openai-codex/gpt-5.5"));
    }

    #[test]
    fn internal_host_app_bridge_fail_closes_when_external_fallback_disabled() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      role: internal_primary_fixture
      default_model: gpt-5.5
    external_fixture:
      enabled: false
      subagent_backend_class: external_cli
      role: bridge_fallback
"#,
        )
        .expect("overlay should parse");
        assert_eq!(
            internal_host_app_bridge_requires_fail_closed(None, &overlay),
            Some("internal host carrier unavailable; external CLI fallback disabled")
        );
    }

    #[test]
    fn internal_host_app_bridge_allows_dispatch_when_receipt_backed_completion_is_configured() {
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
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  receipt_backed_completion_supported: true
"#,
        )
        .expect("selected cli entry should parse");

        assert_eq!(
            internal_host_app_bridge_requires_fail_closed(Some(&selected_cli_entry), &overlay),
            None
        );
    }

    #[test]
    fn internal_host_windows_workspace_write_preflight_fails_fast_without_support_flag() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
"#,
        )
        .expect("selected cli entry should parse");

        let blocker = internal_host_windows_sandbox_preflight_blocker(
            true,
            Some(&selected_cli_entry),
            Some("workspace-write"),
        )
        .expect("workspace-write dispatch should fail closed on Windows");

        assert_eq!(blocker.0, "internal_codex_windows_sandbox_unavailable");
        assert!(blocker.1.contains("failing before process launch"));
        assert_eq!(
            internal_host_windows_sandbox_preflight_blocker(
                false,
                Some(&selected_cli_entry),
                Some("workspace-write"),
            ),
            None
        );
        assert_eq!(
            internal_host_windows_sandbox_preflight_blocker(
                true,
                Some(&selected_cli_entry),
                Some("read-only"),
            ),
            None
        );
    }

    #[test]
    fn internal_host_windows_workspace_write_preflight_honors_support_flag() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  windows_sandbox_spawn_supported: true
"#,
        )
        .expect("selected cli entry should parse");

        assert_eq!(
            internal_host_windows_sandbox_preflight_blocker(
                true,
                Some(&selected_cli_entry),
                Some("workspace-write"),
            ),
            None
        );
    }

    #[test]
    fn internal_host_windows_sandbox_recovery_actions_are_actionable() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    disabled_external_fixture:
      enabled: false
      subagent_backend_class: external_cli
    internal_fixture:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let actions = internal_host_windows_sandbox_recovery_actions(
            &overlay,
            "codex",
            "implementation",
            Some("workspace-write"),
        );

        assert!(
            actions
                .iter()
                .any(|action| action.contains("agent_system.subagents.<backend>.enabled=true")),
            "actions should name the external backend enablement path"
        );
        assert!(
            actions
                .iter()
                .any(|action| action.contains("disabled_external_fixture")),
            "actions should list configured disabled external candidates"
        );
        assert!(
            actions.iter().any(|action| action.contains(
                "host_environment.systems.codex.dispatch.windows_sandbox_spawn_supported=true"
            )),
            "actions should name the exact Windows support flag path"
        );
        assert!(
            actions
                .iter()
                .any(|action| action.contains("receipt-backed")),
            "actions should keep receipt-backed execution as the recovery target"
        );
    }

    #[test]
    fn internal_codex_exec_falls_back_to_ready_admissible_external_route_backend() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-internal-codex-external-fallback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"FALLBACK_OK"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_external_fallback_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
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
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("internal dispatch should return fallback result");

        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["blocker_code"], serde_json::Value::Null);
        assert_eq!(result["backend_dispatch"]["backend_id"], "qwen_cli");
        assert_eq!(
            result["internal_codex_external_fallback"]["blocked_backend"],
            "internal_subagents"
        );
        assert_eq!(
            result["internal_codex_external_fallback"]["fallback_backend"],
            "qwen_cli"
        );
        assert_ne!(result["blocker_code"], "internal_codex_carrier_unavailable");

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn external_readiness_fallback_prefers_ready_inherited_external_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                }
            }
        }));

        assert_eq!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                Some("qwen_cli")
            )
            .as_deref(),
            Some("qwen_cli")
        );
    }

    #[test]
    fn external_readiness_fallback_prefers_ready_runtime_assignment_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "selected_backend_id": "qwen_cli",
                "selected_tier": "external_write_guarded",
                "activation_agent_type": "qwen_cli"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                }
            }
        }));

        assert_eq!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                None
            )
            .as_deref(),
            Some("qwen_cli")
        );
    }

    #[test]
    fn external_dispatch_readiness_retries_default_profile_when_route_profile_is_stale() {
        let root = std::env::temp_dir().join(format!(
            "vida-external-default-profile-retry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        let model_state_path = root.join("model-state.json");
        std::fs::write(
            &model_state_path,
            r#"{"model":{"code":{"providerID":"openai-codex","modelID":"gpt-5.5"}}}"#,
        )
        .expect("write model state");
        let model_state_path = model_state_path.display().to_string().replace('\\', "/");
        let overlay = serde_yaml::from_str::<serde_yaml::Value>(&format!(
            r#"
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      default_model_profile: qwen_gpt55_medium
      model_profiles:
        qwen_gpt54_low:
          provider: qwen
          model_ref: openai-codex/gpt-5.4-mini
          reasoning_effort: low
          normalized_cost_units: 1
          runtime_roles: [worker]
          task_classes: [implementation]
        qwen_gpt55_medium:
          provider: qwen
          model_ref: openai-codex/gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          runtime_roles: [worker]
          task_classes: [implementation]
      readiness:
        model:
          mode: json_code_ref
          path: "{model_state_path}"
"#,
        ))
        .expect("overlay should parse");
        let backend_entry =
            crate::yaml_lookup(&overlay, &["agent_system", "subagents", "qwen_cli"])
                .expect("backend should exist");

        let (readiness, selected_profile) = super::external_cli_dispatch_readiness_verdict(
            "qwen_cli",
            backend_entry,
            Some("qwen_gpt54_low".to_string()),
            "implementer",
            true,
        );

        assert_eq!(readiness["blocked"], false);
        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(readiness["selected_model_profile"], "qwen_gpt55_medium");
        assert_eq!(selected_profile.as_deref(), Some("qwen_gpt55_medium"));
        assert_eq!(
            readiness["stale_selected_profile_retry"]["selected_model_profile"],
            "qwen_gpt54_low"
        );
        assert_eq!(
            readiness["stale_selected_profile_retry"]["selected_blocker_code"],
            crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::ModelNotPinned
            )
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn external_dispatch_readiness_uses_readonly_profile_for_coach_without_owned_paths() {
        let root = std::env::temp_dir().join(format!(
            "vida-external-readonly-profile-retry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        let model_state_path = root.join("model-state.json");
        std::fs::write(
            &model_state_path,
            r#"{"model":{"code":{"providerID":"openai-codex","modelID":"gpt-5.5"}}}"#,
        )
        .expect("write model state");
        let model_state_path = model_state_path.display().to_string().replace('\\', "/");
        let overlay = serde_yaml::from_str::<serde_yaml::Value>(&format!(
            r#"
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      default_model_profile: qwen_gpt55_medium_guarded
      model_profiles:
        qwen_gpt55_high_readonly:
          provider: qwen
          model_ref: openai-codex/gpt-5.5
          reasoning_effort: high
          normalized_cost_units: 8
          runtime_roles: [coach]
          task_classes: [coach, review]
          write_scope: none
        qwen_gpt55_medium_guarded:
          provider: qwen
          model_ref: openai-codex/gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          runtime_roles: [coach]
          task_classes: [coach]
          write_scope: guard_required_owned_paths
      readiness:
        model:
          mode: json_code_ref
          path: "{model_state_path}"
"#,
        ))
        .expect("overlay should parse");
        let backend_entry =
            crate::yaml_lookup(&overlay, &["agent_system", "subagents", "qwen_cli"])
                .expect("backend should exist");

        let (readiness, selected_profile) = super::external_cli_dispatch_readiness_verdict(
            "qwen_cli",
            backend_entry,
            Some("qwen_gpt55_medium_guarded".to_string()),
            "coach",
            false,
        );

        assert_eq!(readiness["blocked"], false);
        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(
            readiness["selected_model_profile"],
            "qwen_gpt55_high_readonly"
        );
        assert_eq!(
            selected_profile.as_deref(),
            Some("qwen_gpt55_high_readonly")
        );
        assert_eq!(
            readiness["guarded_write_profile_retry"]["selected_model_profile"],
            "qwen_gpt55_medium_guarded"
        );
        assert_eq!(
            readiness["guarded_write_profile_retry"]["reason"],
            "selected_profile_requires_owned_paths_but_packet_has_no_owned_scope"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn external_readiness_fallback_rejects_disabled_external_candidate() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "selected_backend_id": "qwen_cli",
                "selected_tier": "external_write_guarded",
                "activation_agent_type": "qwen_cli"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "qwen_cli"
                }
            }
        }));

        assert!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                Some("qwen_cli")
            )
            .is_none(),
            "dispatch-blocked external fallback candidate must not be selected"
        );
    }

    #[test]
    fn internal_codex_exec_preserves_blocker_when_no_admissible_external_fallback_exists() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-internal-codex-no-external-fallback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"NO_FALLBACK"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_external_fallback_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
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
                        "analysis": false,
                        "implementation": false
                    }
                }
            ],
            "development_flow": {
                "analysis": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "qwen_cli",
                    "fanout_executor_backends": ["qwen_cli"]
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("internal dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "internal_codex_carrier_unavailable");
        assert!(result["internal_codex_external_fallback"].is_null());
        assert_eq!(
            result["backend_dispatch"]["receipt_backed_completion_supported"],
            false
        );
        assert_eq!(
            result["backend_dispatch"]["receipt_backed_completion_source_path"],
            "vida.config.yaml:host_environment.systems.codex.dispatch.receipt_backed_completion_supported"
        );
        assert_eq!(
            result["backend_dispatch"]["execution_evidence_required"],
            true
        );
        assert_eq!(
            result["backend_dispatch"]["execution_evidence_available"],
            false
        );
        assert_eq!(
            result["backend_dispatch"]["activation_view_is_execution_evidence"],
            false
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn external_readiness_internal_fallback_preserves_internal_codex_blocker() {
        let blocked_external_backend = "disabled_external_fixture";
        let internal_backend = "internal_fixture";
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-internal-codex-blocker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"EXTERNAL_INTERNAL_BLOCKER"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_disabled_external_primary_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": blocked_external_backend,
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                },
                {
                    "backend_id": internal_backend,
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "analysis": {
                    "executor_backend": blocked_external_backend,
                    "fallback_executor_backend": internal_backend,
                    "fanout_executor_backends": []
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let mut receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );
        receipt.selected_backend = Some(blocked_external_backend.to_string());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some(blocked_external_backend),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "internal_codex_carrier_unavailable");
        assert_eq!(result["backend_dispatch"]["backend_id"], internal_backend);
        assert_eq!(
            result["external_dispatch_blocker_internal_fallback"]["blocked_backend"],
            blocked_external_backend
        );
        assert_eq!(
            result["external_dispatch_blocker_internal_fallback"]["fallback_backend"],
            internal_backend
        );
        assert!(result["internal_codex_external_fallback"].is_null());

        let _ = std::fs::remove_dir_all(&project_root);
    }

    fn internal_codex_external_fallback_overlay() -> &'static str {
        r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: codex_cli_exec
      dispatch:
        command: codex
        receipt_backed_completion_supported: false
        static_args: ["exec", "--json"]
        prompt_mode: positional
      carriers:
        middle:
          model: gpt-5.5
          model_reasoning_effort: medium
          sandbox_mode: workspace-write
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: gpt-5.5
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      detect_command: cargo
      dispatch:
        command: cargo
        static_args:
          - --version
        prompt_mode: stdin
        output_mode: stdout
        prompt_template: "FALLBACK_OK"
"#
    }

    fn internal_codex_disabled_external_primary_overlay() -> &'static str {
        r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch_transport: codex_cli_exec
      dispatch:
        command: codex
        static_args: ["exec", "--json"]
        prompt_mode: positional
      carriers:
        middle:
          model: fixture-model
          model_reasoning_effort: medium
          sandbox_mode: workspace-write
agent_system:
  subagents:
    disabled_external_fixture:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: qwen
        prompt_mode: positional
    internal_fixture:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
"#
    }

    fn internal_codex_fallback_role_selection(
        execution_plan: serde_json::Value,
    ) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Analyze the bounded handoff".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        }
    }

    fn internal_codex_fallback_receipt(
        dispatch_packet_path: &str,
    ) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-codex-fallback".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(dispatch_packet_path.to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn internal_host_activation_only_timeout_uses_timeout_blocker() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-code".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        assert_eq!(
            internal_host_activation_only_blocker_code(
                Path::new("/tmp/project"),
                &role_selection,
                &receipt,
                true,
            ),
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert_eq!(
            internal_host_activation_only_blocker_code(
                Path::new("/tmp/project"),
                &role_selection,
                &receipt,
                false,
            ),
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
    }

    #[test]
    fn configured_internal_host_activation_parts_rejects_danger_full_access_sandbox() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  sandbox_flag: -s
  model_flag: -m
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.5",
            "sandbox_mode": "danger-full-access"
        });

        let error = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect_err("danger-full-access should be rejected");
        assert!(error.contains("forbidden sandbox_mode"));
    }

    #[test]
    fn mark_dispatch_result_execution_evidence_reclassifies_activation_view() {
        let mut body = serde_json::Map::from_iter([(
            "activation_semantics".to_string(),
            serde_json::json!({
                "activation_kind": "activation_view",
                "view_only": true,
                "executes_packet": false,
                "records_completion_receipt": false,
            }),
        )]);

        mark_dispatch_result_execution_evidence(&mut body, "internal_carrier_completion", "junior");

        assert_eq!(
            body["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(body["activation_semantics"]["view_only"], false);
        assert_eq!(body["activation_semantics"]["executes_packet"], true);
        assert_eq!(
            body["activation_semantics"]["records_completion_receipt"],
            true
        );
        assert_eq!(body["execution_evidence"]["status"], "recorded");
        assert_eq!(
            body["execution_evidence"]["evidence_kind"],
            "internal_carrier_completion"
        );
        assert_eq!(body["execution_evidence"]["backend_id"], "junior");
        assert_eq!(body["execution_evidence"]["receipt_backed"], true);
    }

    #[test]
    fn agent_lane_dispatch_result_emits_execution_truth() {
        let result = agent_lane_dispatch_result(
            serde_json::json!({
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true
                }
            }),
            "/tmp/dispatch-packet.json",
            Some("internal_subagents"),
            &RuntimeConsumptionLaneSelection {
                ok: true,
                activation_source: "test".to_string(),
                selection_mode: "fixed".to_string(),
                fallback_role: "orchestrator".to_string(),
                request: "Implement the task".to_string(),
                selected_role: "worker".to_string(),
                conversational_mode: None,
                single_task_only: false,
                tracked_flow_entry: None,
                allow_freeform_chat: false,
                confidence: "high".to_string(),
                matched_terms: vec![],
                compiled_bundle: serde_json::Value::Null,
                execution_plan: serde_json::json!({
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "opencode_cli",
                            "backend_class": "external_cli"
                        },
                        {
                            "backend_id": "internal_subagents",
                            "backend_class": "internal"
                        }
                    ],
                    "development_flow": {
                        "implementer": {
                            "executor_backend": "opencode_cli",
                            "fallback_executor_backend": "internal_subagents"
                        }
                    }
                }),
                reason: "test".to_string(),
            },
            &crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-1".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("worker".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-11T00:00:00Z".to_string(),
            },
            serde_json::json!({
                "selected_cli_execution_class": "internal"
            }),
        );

        assert_eq!(
            result["execution_truth"]["effective_execution_posture"],
            "hybrid"
        );
        assert_eq!(
            result["execution_truth"]["route_primary_backend"],
            "opencode_cli"
        );
        assert_eq!(
            result["execution_truth"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert_eq!(
            result["execution_truth"]["selected_backend_source"],
            "route_fallback_hint"
        );
        assert_eq!(
            result["execution_truth"]["activation_evidence"]["execution_evidence_status"],
            "missing"
        );
    }

    #[test]
    fn selected_internal_host_carrier_maps_internal_backend_alias_to_activation_tier() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  junior:
    model: gpt-5.5
    model_reasoning_effort: low
    sandbox_mode: workspace-write
  middle:
    model: gpt-5.5
    model_reasoning_effort: medium
    sandbox_mode: workspace-write
"#,
        )
        .expect("system entry should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true,
                            "review": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-carrier-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            None,
        )
        .expect("internal backend alias should bridge to activation tier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_selected_model_profile_fields() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  middle:
    model: gpt-5.5
    model_reasoning_effort: medium
    sandbox_mode: workspace-write
    default_model_profile: codex_gpt54_medium
    model_profiles:
      codex_gpt54_medium:
        model_ref: gpt-5.5
        reasoning_effort: medium
        sandbox_mode: workspace-write
        normalized_cost_units: 4
        runtime_roles: [coach]
        task_classes: [review]
      codex_spark_high_review:
        model_ref: gpt-5.3-codex-spark
        reasoning_effort: high
        sandbox_mode: read-only
        normalized_cost_units: 16
        runtime_roles: [coach]
        task_classes: [review]
"#,
        )
        .expect("system entry should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle",
                    "selected_model_profile_id": "codex_spark_high_review"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true,
                            "review": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-profile-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            None,
        )
        .expect("internal backend alias should bridge to activation tier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
        assert_eq!(
            carrier["selected_model_profile_id"].as_str(),
            Some("codex_spark_high_review")
        );
        assert_eq!(carrier["model"].as_str(), Some("gpt-5.3-codex-spark"));
        assert_eq!(carrier["model_reasoning_effort"].as_str(), Some("high"));
        assert_eq!(carrier["sandbox_mode"].as_str(), Some("read-only"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_internal_subagent_route_profile_overlay() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  middle:
    model: gpt-5.5
    model_reasoning_effort: high
    sandbox_mode: workspace-write
"#,
        )
        .expect("system entry should parse");
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: internal_fast
          reasoning_effort: low
          normalized_cost_units: 6
          speed_tier: fast
          quality_tier: medium_high
          write_scope: orchestrator_native
          runtime_roles: [worker]
          task_classes: [implementation]
        internal_review:
          provider: internal
          model_ref: internal_review
          reasoning_effort: medium
          normalized_cost_units: 8
          speed_tier: medium
          quality_tier: high
          write_scope: read_or_review
          runtime_roles: [coach]
          task_classes: [review]
"#,
        )
        .expect("overlay should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "internal_subagents",
                        "profiles": {
                            "internal_subagents": "internal_review"
                        }
                    }
                },
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle",
                    "selected_model_profile_id": "codex_gpt54_medium"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-route-profile-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            Some(&overlay),
        )
        .expect("internal route profile should bridge through host carrier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
        assert_eq!(carrier["model"].as_str(), Some("internal_review"));
        assert_eq!(
            carrier["selected_model_profile_id"].as_str(),
            Some("internal_review")
        );
        assert_eq!(
            carrier["internal_subagent_model_profile_id"].as_str(),
            Some("internal_review")
        );
        assert_eq!(
            carrier["selected_model_ref"].as_str(),
            Some("internal_review")
        );
        assert_eq!(carrier["model_reasoning_effort"].as_str(), Some("medium"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_internal_subagent_write_scope_sandbox() {
        let carrier = serde_json::json!({
            "role_id": "senior",
            "model": "gpt-5.5",
            "model_reasoning_effort": "high",
            "sandbox_mode": "read-only"
        });
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      write_scope: orchestrator_native
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: internal_fast
          reasoning_effort: low
          normalized_cost_units: 1
          write_scope: orchestrator_native
          runtime_roles: [worker]
          task_classes: [implementation]
"#,
        )
        .expect("overlay should parse");
        let backend_entry =
            super::configured_subagent_backend_entry(&overlay, "internal_subagents");

        let patched = super::apply_internal_subagent_profile_overlay(
            &carrier,
            "internal_subagents",
            backend_entry,
            Some("internal_fast"),
        );

        assert_eq!(patched["sandbox_mode"].as_str(), Some("workspace-write"));
        assert_eq!(
            patched["selected_sandbox_mode"].as_str(),
            Some("workspace-write")
        );
        assert_eq!(patched["write_scope"].as_str(), Some("orchestrator_native"));
    }

    #[test]
    fn wrap_command_with_optional_timeout_adds_kill_after_grace() {
        let wrapped = wrap_command_with_optional_timeout(
            "codex".to_string(),
            vec!["exec".to_string()],
            Some(5),
        );

        assert_eq!(wrapped.command, "codex");
        assert_eq!(wrapped.args, vec!["exec".to_string()]);
        assert_eq!(
            wrapped.timeout_wrapper,
            Some(CommandTimeoutWrapper {
                timeout_seconds: 5,
                kill_after_grace_seconds: 1,
                no_output_timeout_seconds: None,
            })
        );
    }

    #[test]
    fn internal_host_dispatch_wall_timeout_uses_configured_route_window() {
        let wrapped = wrap_command_with_optional_timeout(
            "codex".to_string(),
            vec!["exec".to_string()],
            Some(420),
        );

        assert_eq!(
            wrapped.timeout_wrapper,
            Some(CommandTimeoutWrapper {
                timeout_seconds: 420,
                kill_after_grace_seconds: 1,
                no_output_timeout_seconds: None,
            })
        );
    }

    #[test]
    fn wrap_command_with_optional_timeouts_caps_no_output_window_to_wall_timeout() {
        let wrapped = wrap_command_with_optional_timeouts(
            "local-bridge".to_string(),
            vec!["run".to_string()],
            Some(5),
            Some(10),
        );

        assert_eq!(
            wrapped.timeout_wrapper,
            Some(CommandTimeoutWrapper {
                timeout_seconds: 5,
                kill_after_grace_seconds: 1,
                no_output_timeout_seconds: Some(5),
            })
        );
    }

    #[test]
    fn internal_host_dispatch_wall_timeout_honors_configured_route_window() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-internal-host-timeout-cap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "development_flow": {
                "analysis": {
                    "executor_backend": "internal_subagents",
                    "max_runtime_seconds": 420
                }
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );

        assert_eq!(
            configured_internal_host_dispatch_wall_timeout_seconds(
                &project_root,
                &role_selection,
                &receipt
            ),
            420
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_carries_implementation_isolation_contract() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-implementation-isolation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        let state_root = project_root.join(".vida").join("data").join("state");
        std::fs::create_dir_all(
            state_root
                .join("runtime-consumption")
                .join("dispatch-packets"),
        )
        .expect("create packet dir");
        let packet_path = state_root
            .join("runtime-consumption")
            .join("dispatch-packets")
            .join("implementation.json");
        let implementation_isolation = serde_json::json!({
            "schema_version": "implementation-isolation-v1",
            "canonical_worktree_writes_allowed": false,
            "default_mode": "patch_proposal",
            "allowed_modes": ["patch_proposal", "isolated_worktree"],
            "artifact_contract": "stage_attempt_implementation_artifact_v1",
            "owned_paths": ["crates/vida/src/runtime_dispatch_execution.rs"],
            "required_result_fields": [
                "artifact_kind",
                "attempt_id",
                "task_id",
                "stage_id",
                "changed_files"
            ],
            "scope_policy": {
                "changed_files_must_be_subset_of_owned_paths": true,
                "patch_paths_must_be_subset_of_owned_paths": true
            }
        });
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "delivery_task_packet": {
                    "owned_paths": ["crates/vida/src/runtime_dispatch_execution.rs"],
                    "read_only_paths": ["docs/process"],
                    "proof_target": "implementation proof",
                    "implementation_isolation": implementation_isolation.clone()
                }
            })
            .to_string(),
        )
        .expect("write packet");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
host_tool_bridge:
  request_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  result_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  receipt_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
"#,
        )
        .expect("host bridge config should parse");
        let receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &project_root.join(".vida/data/state"),
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "junior",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");

        assert_eq!(
            request["implementation_isolation"]["schema_version"],
            "implementation-isolation-v1"
        );
        assert_eq!(
            request["expected_implementation_artifact_kinds"],
            serde_json::json!(["patch_proposal", "isolated_worktree_manifest"])
        );
        assert_eq!(
            request["required_result_fields"],
            serde_json::json!([
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ])
        );
        assert_eq!(request["implementation_artifacts"], serde_json::json!([]));
        assert_eq!(
            request["implementation_isolation"],
            implementation_isolation
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_carries_upstream_proof_artifact_scope() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-proof-artifact-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        let state_root = project_root.join(".vida").join("data").join("state");
        let packet_dir = state_root
            .join("runtime-consumption")
            .join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("implementation-proof-scope.json");
        let proof_paths = serde_json::json!([
            "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
            "src/test/features/list_view/data/record_chatter_repository_test.dart",
            "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
        ]);
        let normalized_proof_paths = serde_json::json!([
            "src/test/features/list_view/data/record_chatter_repository_test.dart",
            "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
            "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
        ]);
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "dispatch_target": "developer",
                "handoff_runtime_role": "worker",
                "handoff_task_class": "implementation",
                "owned_paths": ["src/lib/features/list_view"],
                "proof_artifact_paths": proof_paths,
                "proof_targets": [
                    "flutter test src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "flutter test src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "flutter test src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ],
                "delivery_task_packet": {
                    "handoff_runtime_role": "worker",
                    "handoff_task_class": "implementation",
                    "owned_paths": ["src/lib/features/list_view"],
                    "proof_artifact_paths": proof_paths,
                    "verification_commands": [
                        "flutter test src/test/features/list_view/domain/models/record_chatter_models_test.dart src/test/features/list_view/data/record_chatter_repository_test.dart src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                    ]
                }
            })
            .to_string(),
        )
        .expect("write packet");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
host_tool_bridge:
  request_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  result_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  receipt_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
"#,
        )
        .expect("host bridge config should parse");
        let mut receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );
        receipt.run_id = "activity-meeting-event-form-fields".to_string();
        receipt.dispatch_target = "developer".to_string();
        receipt.activation_runtime_role = Some("worker".to_string());
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");

        assert_eq!(request["proof_artifact_paths"], normalized_proof_paths);
        assert_eq!(request["proof_artifact_scope"], normalized_proof_paths);
        assert_eq!(
            request["implementation_isolation"]["scope_policy"]["changed_files_must_be_subset_of_owned_or_proof_paths"],
            true
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_carries_proof_scope_from_generated_downstream_packet() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-generated-proof-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        let state_root = project_root.join(".vida").join("data").join("state");
        let packet_dir = state_root
            .join("runtime-consumption")
            .join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("generated-developer.json");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement list view chatter and keep proof tests in scope".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {},
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "activation_runtime_role": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation",
                    "selected_backend_id": "internal_subagents"
                },
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "planner_metadata": {
                            "owned_paths": ["src/lib/features/list_view"],
                            "proof_targets": [
                                "flutter test src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                                "flutter test src/test/features/list_view/data/record_chatter_repository_test.dart",
                                "flutter test src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                            ]
                        }
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "developer": {
                                "dispatch_target": "developer",
                                "task_class": "implementation",
                                "runtime_role": "worker",
                                "closure_class": "implementation",
                                "packet_template_kind": "delivery_task_packet"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let mut upstream_receipt = internal_codex_fallback_receipt("upstream.json");
        upstream_receipt.run_id = "activity-meeting-event-form-fields".to_string();
        upstream_receipt.dispatch_target = "autotester".to_string();
        upstream_receipt.downstream_dispatch_target = Some("developer".to_string());
        crate::runtime_dispatch_downstream_packets::write_runtime_downstream_dispatch_packet_at_with_owned_paths(
            &packet_path,
            &role_selection,
            &serde_json::json!({ "run_id": "activity-meeting-event-form-fields" }),
            &upstream_receipt,
            &[],
        )
        .expect("generated downstream packet should write");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
host_tool_bridge:
  request_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  result_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  receipt_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
"#,
        )
        .expect("host bridge config should parse");
        let mut developer_receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );
        developer_receipt.run_id = "activity-meeting-event-form-fields".to_string();
        developer_receipt.dispatch_target = "developer".to_string();
        developer_receipt.activation_runtime_role = Some("worker".to_string());

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "middle",
            &developer_receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize from generated packet");

        assert_eq!(
            request["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/data/record_chatter_repository_test.dart",
                "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_carries_autotester_configured_role_and_task_class() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-autotester-contract-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        let state_root = project_root.join(".vida").join("data").join("state");
        let packet_dir = state_root
            .join("runtime-consumption")
            .join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("autotester.json");
        let owned_paths = serde_json::json!(["lib/src/activity_screen.dart", "test"]);
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "dispatch_target": "autotester",
                "handoff_runtime_role": "worker",
                "handoff_task_class": "test_authoring",
                "runtime_role": "business_analyst",
                "task_class": "specification",
                "owned_paths": owned_paths,
                "read_only_paths": ["docs/process"],
                "proof_target": "autotester proof",
                "delivery_task_packet": {
                    "handoff_runtime_role": "worker",
                    "handoff_task_class": "test_authoring",
                    "owned_paths": owned_paths,
                    "implementation_isolation": serde_json::Value::Null,
                    "proof_target": "autotester proof"
                }
            })
            .to_string(),
        )
        .expect("write packet");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
host_tool_bridge:
  request_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  result_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  receipt_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
"#,
        )
        .expect("host bridge config should parse");
        let mut receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );
        receipt.run_id = "activity-meeting-event-form-fields".to_string();
        receipt.dispatch_target = "autotester".to_string();
        receipt.activation_runtime_role = Some("business_analyst".to_string());
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));

        let request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &role_selection,
        )
        .expect("host bridge request should materialize");

        assert_eq!(request["runtime_role"], "worker");
        assert_eq!(request["task_class"], "test_authoring");
        assert_eq!(request["dispatch_target"], "autotester");
        assert_eq!(request["implementation_isolation"], serde_json::Value::Null);
        assert_eq!(
            request["expected_implementation_artifact_kinds"],
            serde_json::json!([])
        );
        assert_eq!(
            request["owned_paths"],
            serde_json::json!(["lib/src/activity_screen.dart", "test"])
        );
        assert_eq!(request["proof_target"], "autotester proof");

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_request_refreshes_pending_stale_autotester_contract() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-host-bridge-autotester-refresh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(project_root.join("test")).expect("create test root");
        let state_root = project_root.join(".vida").join("data").join("state");
        let packet_dir = state_root
            .join("runtime-consumption")
            .join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("autotester-stale.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "dispatch_target": "autotester",
                "handoff_runtime_role": "business_analyst",
                "handoff_task_class": "specification",
                "runtime_role": "business_analyst",
                "task_class": "specification",
                "owned_paths": ["lib/src/activity_screen.dart"],
                "read_only_paths": ["docs/process"],
                "proof_target": "autotester proof",
                "delivery_task_packet": {
                    "handoff_runtime_role": "business_analyst",
                    "handoff_task_class": "specification",
                    "owned_paths": ["lib/src/activity_screen.dart"],
                    "implementation_isolation": serde_json::Value::Null,
                    "proof_target": "autotester proof"
                }
            })
            .to_string(),
        )
        .expect("write stale packet");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
host_tool_bridge:
  request_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  result_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  receipt_dir: .vida/data/state/runtime-consumption/host-tool-bridge
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
 "#,
        )
        .expect("host bridge config should parse");
        let mut receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );
        receipt.run_id = "activity-meeting-event-form-fields".to_string();
        receipt.dispatch_target = "autotester".to_string();
        receipt.activation_runtime_role = Some("business_analyst".to_string());
        let stale_role_selection = internal_codex_fallback_role_selection(serde_json::json!({}));

        let stale_request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &stale_role_selection,
        )
        .expect("stale request should materialize");
        assert_eq!(stale_request["runtime_role"], "business_analyst");
        assert_eq!(stale_request["task_class"], "specification");
        let request_path = stale_request["request_path"]
            .as_str()
            .expect("request path should render");
        let mut persisted_request: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(request_path).expect("read stale request"),
        )
        .expect("decode stale request");
        for field in ["request_path", "result_path", "receipt_path"] {
            let slash_normalized = persisted_request[field]
                .as_str()
                .expect("path field should render")
                .replace('\\', "/");
            persisted_request[field] = serde_json::json!(slash_normalized);
        }
        std::fs::write(
            request_path,
            serde_json::to_string_pretty(&persisted_request).expect("encode stale request"),
        )
        .expect("rewrite stale request with path metadata drift");

        let refreshed_role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "autotester": {
                            "dispatch_target": "autotester",
                            "runtime_role": "worker",
                            "task_class": "implementation_medium",
                            "closure_class": "implementation",
                            "packet_template_kind": "delivery_task_packet"
                        }
                    }
                }
            }
        }));
        let refreshed_request = materialize_host_tool_bridge_request(
            &project_root,
            &state_root,
            Some(&selected_cli_entry),
            packet_path.to_str().expect("packet path should render"),
            "internal_subagents",
            "middle",
            &receipt,
            &refreshed_role_selection,
        )
        .expect("pending stale request should refresh from lane contract");

        assert_eq!(refreshed_request["runtime_role"], "worker");
        assert_eq!(refreshed_request["task_class"], "implementation_medium");
        assert_eq!(
            refreshed_request["implementation_isolation"]["canonical_worktree_writes_allowed"],
            false
        );
        assert!(
            refreshed_request["owned_paths"]
                .as_array()
                .expect("request owned paths")
                .iter()
                .any(|path| path == "test")
        );
        assert!(
            refreshed_request["implementation_isolation"]["owned_paths"]
                .as_array()
                .expect("isolation owned paths")
                .iter()
                .any(|path| path == "test")
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn internal_host_dispatch_no_output_timeout_reads_system_dispatch_config() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
execution_class: internal
dispatch:
  command: local-bridge
  no_output_timeout_seconds: 2
  prompt_mode: stdin
"#,
        )
        .expect("selected cli entry should parse");

        assert_eq!(
            configured_internal_host_dispatch_no_output_timeout_seconds(Some(&selected_cli_entry)),
            Some(2)
        );
    }

    #[test]
    fn internal_host_dispatch_no_output_timeout_applies_inside_worker_process() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
execution_class: internal
dispatch:
  command: local-bridge
  no_output_timeout_seconds: 2
  prompt_mode: stdin
"#,
        )
        .expect("selected cli entry should parse");
        std::env::set_var(
            crate::init_surfaces::AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV,
            "1",
        );
        let timeout =
            configured_internal_host_dispatch_no_output_timeout_seconds(Some(&selected_cli_entry));
        std::env::remove_var(crate::init_surfaces::AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV);

        assert_eq!(timeout, Some(2));
    }

    #[test]
    fn internal_host_dispatch_wall_timeout_uses_receipt_target_not_packet_handoff_class() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-receipt-route-window-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create temp root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
agent_system:
  routing:
    analysis:
      max_runtime_seconds: 60
    implementation:
      max_runtime_seconds: 420
"#,
        )
        .expect("write config");
        let packet_path = project_root.join("dispatch-packet.json");
        std::fs::write(
            &packet_path,
            r#"{"delivery_task_packet":{"handoff_task_class":"implementation"}}"#,
        )
        .expect("write packet");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "development_flow": {
                "analysis": {
                    "executor_backend": "internal_subagents",
                    "max_runtime_seconds": 60
                },
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "max_runtime_seconds": 420
                }
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            packet_path.to_str().expect("packet path should render"),
        );

        assert_eq!(
            configured_internal_host_dispatch_wall_timeout_seconds(
                &project_root,
                &role_selection,
                &receipt,
            ),
            60
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[cfg(unix)]
    #[test]
    fn execute_wrapped_command_times_out_when_descendant_keeps_pipe_open() {
        let wrapped = wrap_command_with_optional_timeout(
            "sh".to_string(),
            vec!["-c".to_string(), "(sleep 30) & exit 0".to_string()],
            Some(1),
        );
        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("timed command should complete");

        assert!(output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(windows)]
    fn windows_process_command_line_contains(token: &str) -> bool {
        let script = format!(
            "$token = '{}'; \
             Get-CimInstance Win32_Process | \
             Where-Object {{ $_.ProcessId -ne $PID -and $_.CommandLine -like \"*$token*\" }} | \
             Select-Object -First 1 -ExpandProperty ProcessId",
            token
        );
        let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
        else {
            return false;
        };
        !String::from_utf8_lossy(&output.stdout).trim().is_empty()
    }

    #[cfg(windows)]
    #[test]
    fn execute_wrapped_command_kills_windows_descendant_process_tree_on_timeout() {
        let token = format!(
            "vida-process-tree-timeout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        );
        let child_command = format!("Start-Sleep -Seconds 30 # {token}");
        let parent_command = format!(
            "Start-Process -WindowStyle Hidden -FilePath powershell -ArgumentList @('-NoProfile','-Command','{}'); Start-Sleep -Seconds 30",
            child_command.replace('\'', "''")
        );
        let wrapped = wrap_command_with_optional_timeouts(
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                parent_command,
            ],
            Some(1),
            Some(1),
        );
        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("timed command should complete");

        assert!(output.timed_out);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "expected Windows process-tree timeout wrapper to return promptly, got {:?}",
            started.elapsed()
        );
        std::thread::sleep(Duration::from_millis(500));
        assert!(
            !windows_process_command_line_contains(&token),
            "timeout wrapper should kill descendant process carrying token {token}"
        );
    }

    #[test]
    fn execute_wrapped_command_times_out_after_initial_output_goes_idle() {
        #[cfg(windows)]
        let wrapped = wrap_command_with_optional_timeouts(
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "[Console]::Out.WriteLine('started'); [Console]::Out.Flush(); Start-Sleep -Seconds 30"
                    .to_string(),
            ],
            Some(30),
            Some(1),
        );
        #[cfg(unix)]
        let wrapped = wrap_command_with_optional_timeouts(
            "sh".to_string(),
            vec!["-c".to_string(), "printf 'started\n'; sleep 30".to_string()],
            Some(30),
            Some(1),
        );
        #[cfg(not(any(unix, windows)))]
        return;

        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("idle command should complete through timeout wrapper");

        assert!(output.timed_out);
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("started"),
            "timeout wrapper should preserve output observed before idle timeout"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "expected idle-output timeout wrapper to return promptly, got {:?}",
            started.elapsed()
        );
    }

    #[cfg(unix)]
    #[test]
    fn execute_wrapped_command_times_out_when_detached_descendant_keeps_pipe_open() {
        let wrapped = wrap_command_with_optional_timeout(
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "setsid sh -c 'sleep 30' & exit 0".to_string(),
            ],
            Some(1),
        );
        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("detached timed command should complete");

        assert!(output.timed_out);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "expected detached descendant timeout wrapper to return within a bounded window, got {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_denies_read_only_backend_for_implementer() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true,
                        "execution_preparation": true,
                        "implementation": false,
                        "review": true,
                        "verification": false,
                        "policy_flags": {
                            "read_only_backend": true,
                            "review_only_backend": true,
                            "scoped_write_backend": false,
                            "internal_only_backend": false
                        }
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true,
                        "execution_preparation": true,
                        "implementation": true,
                        "review": true,
                        "verification": true,
                        "policy_flags": {
                            "read_only_backend": false,
                            "review_only_backend": false,
                            "scoped_write_backend": false,
                            "internal_only_backend": true
                        }
                    }
                }
            ]
        });

        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "hermes_cli should be inadmissible for implementer alias lane"
        );
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementation"
            ),
            "hermes_cli should be inadmissible for implementation lane"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "hermes_cli should be admissible for analysis lane"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "internal_subagents",
                "implementation"
            ),
            "internal_subagents should be admissible for implementation lane"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_open_without_matrix() {
        let execution_plan = serde_json::json!({});
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "write-producing implementer lane should fail closed when no admissibility matrix is present"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "read-only lanes should still fail open when no admissibility matrix is present"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_open_for_unknown_backend() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "other_backend",
                    "lane_admissibility": {
                        "implementation": false
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "implementer lane should fail closed when backend row is missing from the matrix"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "read-only lanes should continue failing open when backend is not in the matrix"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_closed_for_implementer_when_lane_key_missing()
     {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "implementer lane should fail closed when canonical implementation key is absent"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_uses_configured_lane_task_class_for_role_label() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "lane_admissibility": {
                        "implementation": false,
                        "verification": true
                    }
                }
            ],
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "developer": {
                            "dispatch_target": "developer",
                            "runtime_role": "worker",
                            "task_class": "implementation"
                        },
                        "tester": {
                            "dispatch_target": "tester",
                            "runtime_role": "verifier",
                            "task_class": "verification"
                        }
                    }
                }
            }
        });

        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "developer"
            ),
            "developer role label should enforce implementation denial"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "tester"
            ),
            "tester role label should enforce verification allowance"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_closed_for_execution_preparation_when_canonical_lane_key_missing()
     {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "lane_admissibility": {
                        "execution_preparation": false
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "execution_preparation"
            ),
            "execution_preparation lane should fail closed when canonical architecture key is absent"
        );
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_inadmissible_implementer_backend_before_launch()
    {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-admissibility-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      external_backend_id: hermes_cli
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
"#,
        )
        .expect("write overlay");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "coach": true,
                            "implementation": false
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("hermes_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "backend_inadmissible_for_lane");
        assert_eq!(result["backend_dispatch"]["backend_id"], "hermes_cli");
        assert_eq!(
            result["backend_dispatch"]["provider_error"],
            serde_json::Value::Null
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn preserves_lane_policy_for_dispatch_target_alias_before_launch() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-alias-policy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      external_backend_id: hermes_cli
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
"#,
        )
        .expect("write overlay");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "coach": true,
                            "implementation": false
                        }
                    }
                ],
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "dev",
                                "task_class": "implementation",
                                "activation": {
                                    "activation_agent_type": "worker",
                                    "activation_runtime_role": "worker"
                                }
                            }
                        }
                    },
                    "implementation": {
                        "executor_backend": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1-dev".to_string(),
            dispatch_target: "dev".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("hermes_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "backend_inadmissible_for_lane");
        assert_eq!(result["backend_dispatch"]["backend_id"], "hermes_cli");
        assert_eq!(
            result["backend_dispatch"]["provider_error"],
            serde_json::Value::Null
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_known_readiness_failure_before_launch() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-readiness-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        let missing_auth = project_root.join("missing-provider-auth.json");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            format!(
                r#"
host_environment:
  cli_system: opencode
  systems:
    opencode:
      enabled: true
      execution_class: external
      external_backend_id: opencode_cli
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
      readiness:
        auth:
          mode: file_present
          path: "{}"
"#,
                missing_auth.display().to_string().replace('\\', "/")
            ),
        )
        .expect("write overlay");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "coach": true,
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli"
                    }
                },
                "runtime_assignment": {
                    "selected_backend_id": "opencode_cli",
                    "selected_model_profile_id": "opencode_minimax_free_review"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("opencode_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "interactive_auth_required");
        assert_eq!(result["backend_dispatch"]["backend_id"], "opencode_cli");
        assert_eq!(
            result["external_backend_readiness"]["status"],
            "interactive_auth_required"
        );
        assert_eq!(
            result["backend_dispatch"]["external_backend_readiness"]["blocked"],
            true
        );
        assert_eq!(
            result["backend_dispatch"]["provider_error"],
            serde_json::Value::Null
        );
        assert!(
            !result["blocker_reason"]
                .as_str()
                .expect("blocker reason should render")
                .contains("SHOULD_NOT_LAUNCH")
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_executes_stdin_prompt_success_result() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-stdin-success-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: pi
  systems:
    pi:
      enabled: true
      execution_class: external
      external_backend_id: pi_cli
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      detect_command: sh
      dispatch:
        command: sh
        static_args:
          - -lc
          - |
            input=$(cat)
            printf '{"type":"result","is_error":false,"result":"%s"}' "$input"
        prompt_mode: stdin
        prompt_template: "STDIN_OK"
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("pi_cli");
        let receipt = external_test_receipt("pi_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("pi_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should execute");

        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["provider_result"], "STDIN_OK");
        assert_eq!(result["blocker_code"], serde_json::Value::Null);
        let activation_command = result["activation_command"]
            .as_str()
            .expect("activation command should render");
        assert!(!activation_command.contains("STDIN_OK"));

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_parseable_adapter_error_json() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-stdin-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: pi
  systems:
    pi:
      enabled: true
      execution_class: external
      external_backend_id: pi_cli
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      detect_command: sh
      dispatch:
        command: sh
        static_args:
          - -lc
          - |
            cat >/dev/null
            printf '{"type":"result","subtype":"error_during_execution","is_error":true,"error":{"message":"adapter boom"}}'
            exit 1
        prompt_mode: stdin
        prompt_template: "STDIN_ERROR"
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("pi_cli");
        let receipt = external_test_receipt("pi_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("pi_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "configured_backend_dispatch_failed");
        assert_eq!(result["provider_is_error"], true);
        assert_eq!(result["provider_error_message"], "adapter boom");
        assert_eq!(result["blocker_reason"], "adapter boom");

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_disabled_external_backend_before_launch() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-disabled-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
    hermes_cli:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("hermes_cli");
        let receipt = external_test_receipt("hermes_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("hermes_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "configured_backend_dispatch_failed");
        assert_eq!(
            result["external_backend_readiness"]["status"],
            "external_backend_dispatch_blocked"
        );
        assert!(
            result["blocker_reason"]
                .as_str()
                .expect("blocker reason should render")
                .contains("disabled")
        );
        assert!(
            !result["blocker_reason"]
                .as_str()
                .expect("blocker reason should render")
                .contains("SHOULD_NOT_LAUNCH")
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    fn external_test_role_selection(backend_id: &str) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Run the bounded external dispatch".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": backend_id,
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": backend_id
                    }
                },
                "runtime_assignment": {
                    "selected_backend_id": backend_id
                }
            }),
            reason: "test".to_string(),
        }
    }

    fn external_test_receipt(backend_id: &str) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some(backend_id.to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn readiness_fallback_internal_backend_uses_admissible_internal_fallback() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded implementation".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
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
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            super::readiness_fallback_internal_backend(&role_selection, "coach", "hermes_cli"),
            Some("internal_subagents".to_string())
        );
    }

    #[test]
    fn readiness_fallback_internal_backend_rejects_inadmissible_internal_fallback() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Verify the bounded implementation".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
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
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "verification": false
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            super::readiness_fallback_internal_backend(
                &role_selection,
                "verification",
                "hermes_cli"
            ),
            None
        );
    }
}
