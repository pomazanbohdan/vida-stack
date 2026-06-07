use std::{path::Path, process::ExitCode};

use crate::dev_team_sequence_contract::{
    configured_dev_team_first_step_for_task, dev_team_sequence, dev_team_sequence_for_task,
    dev_team_sequence_for_work_item, selected_dev_team_flow_for_task, task_flow_lookup_keys,
    DevTeamSequenceStep,
};
use crate::launcher_activation_snapshot::capture_launcher_activation_snapshot_for_root;
use crate::operator_command_text::human_command;
use crate::{
    state_store, state_store::StateStore, AgentArgs, AgentCommand, AgentDispatchNextArgs,
    AgentHostBridgeArgs, AgentSelectArgs, AgentStatusArgs,
};

const AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(300);
const HOST_BRIDGE_PROVENANCE_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLaneSelectionTruth {
    selected_carrier: String,
    selected_backend: String,
    selected_model_profile: String,
    selected_model_ref: String,
    selected_reasoning_effort: String,
    rate: u64,
    estimated_task_price_units: u64,
    budget_verdict: String,
    selected_over_budget: bool,
    selected_model_profile_readiness_status: String,
    pricing_freshness_status: String,
    selected_external_backend_readiness_status: String,
    selection_source_paths: serde_json::Value,
    pricing_readiness: serde_json::Value,
    runtime_role: String,
    task_class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLanePreview {
    lane_index: usize,
    task_id: String,
    title: String,
    role_label: String,
    runtime_role: String,
    task_class: String,
    dispatch_command: String,
    dispatch_command_kind: String,
    receipt_backed_execution_command: String,
    ready_parallel_safe: bool,
    selection_reason: String,
    selection_truth: AgentDispatchLaneSelectionTruth,
    requires_user_approval: bool,
    approval_gate: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchBlockedCandidate {
    task_id: String,
    title: String,
    ready_now: bool,
    ready_parallel_safe: bool,
    reasons: Vec<String>,
    parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchNextPreview {
    status: String,
    mode: String,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    effective_max_parallel_agents: usize,
    lanes_selected: usize,
    selected_lanes: Vec<AgentDispatchLanePreview>,
    blocked_candidates: Vec<AgentDispatchBlockedCandidate>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    execute_supported: bool,
    execution_attempted: bool,
    parallelization_planner: serde_json::Value,
    packet_materialization: serde_json::Value,
    carrier_selection_api: serde_json::Value,
    fanout_guard: serde_json::Value,
    flow_projection: serde_json::Value,
    source_surfaces: Vec<String>,
}

fn agent_dispatch_source_surfaces() -> Vec<String> {
    vec![
        "vida agent dispatch-next".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida taskflow graph-summary --json".to_string(),
        "vida taskflow scheduler dispatch --json".to_string(),
        "vida agent select --runtime-role <role> --task-class <class> --json".to_string(),
        "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            .to_string(),
        "vida agent-init --role worker <task-id> --json".to_string(),
        "vida agent-init --role <runtime-role> <task-id> --json".to_string(),
    ]
}

fn host_bridge_required_string<'a>(
    request: &'a serde_json::Value,
    field: &str,
    missing: &mut Vec<String>,
) -> Option<&'a str> {
    let value = request
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if value.is_none() {
        missing.push(field.to_string());
    }
    value
}

fn read_host_bridge_request(path: &Path) -> Result<serde_json::Value, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Host bridge request `{}` is a symlink; refusing to follow it.",
            path.display()
        ));
    }
    let raw = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to decode host bridge request `{}` as JSON: {error}",
            path.display()
        )
    })
}

fn path_contains_dot_segment(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    })
}

fn canonical_state_artifact_path(
    state_root: &Path,
    raw_path: &str,
    require_existing_file: bool,
) -> Result<std::path::PathBuf, String> {
    let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(raw_path);
    if path_contains_dot_segment(&path) {
        return Err(format!(
            "Host bridge artifact path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    let canonical_state_root = std::fs::canonicalize(state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if require_existing_file {
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "Failed to inspect host bridge artifact `{}`: {error}",
                path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Host bridge artifact `{}` is a symlink; refusing to follow it.",
                path.display()
            ));
        }
        let canonical_path = std::fs::canonicalize(&path).map_err(|error| {
            format!(
                "Failed to canonicalize host bridge artifact `{}`: {error}",
                path.display()
            )
        })?;
        if !canonical_path.starts_with(&canonical_state_root) {
            return Err(format!(
                "Host bridge artifact `{}` escapes VIDA state root `{}`.",
                canonical_path.display(),
                canonical_state_root.display()
            ));
        }
        Ok(canonical_path)
    } else {
        let parent = path.parent().ok_or_else(|| {
            format!(
                "Host bridge artifact path `{}` has no parent directory.",
                path.display()
            )
        })?;
        let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
            format!(
                "Failed to canonicalize host bridge artifact directory `{}`: {error}",
                parent.display()
            )
        })?;
        if !canonical_parent.starts_with(&canonical_state_root) {
            return Err(format!(
                "Host bridge artifact path `{}` escapes VIDA state root `{}`.",
                path.display(),
                canonical_state_root.display()
            ));
        }
        Ok(path)
    }
}

fn host_bridge_request_string<'a>(request: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    request
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn host_bridge_request_provenance_blockers(
    request_path: &Path,
    request: &serde_json::Value,
    state_root: Option<&Path>,
) -> Vec<String> {
    let provided_state_root = state_root.map(Path::to_path_buf);
    let inferred_state_root = infer_host_bridge_state_root_from_request_path(request_path);
    let state_root = match (provided_state_root, inferred_state_root) {
        (Some(provided), Some(_inferred))
            if host_bridge_request_path_is_under_state_root(request_path, &provided) =>
        {
            provided
        }
        (Some(_provided), Some(inferred)) => inferred,
        (Some(provided), None) => provided,
        (None, Some(inferred)) => inferred,
        (None, None) => crate::taskflow_task_bridge::proxy_state_dir(),
    };
    host_bridge_request_provenance_blockers_for_state_root(&state_root, request_path, request).await
}

fn infer_host_bridge_state_root_from_request_path(request_path: &Path) -> Option<std::path::PathBuf> {
    let request_path = std::fs::canonicalize(request_path).ok()?;
    for ancestor in request_path.ancestors() {
        let Some(state_name) = ancestor.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(data_dir) = ancestor.parent() else {
            continue;
        };
        let Some(data_name) = data_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(vida_dir) = data_dir.parent() else {
            continue;
        };
        let Some(vida_name) = vida_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if state_name == "state" && data_name == "data" && vida_name == ".vida" {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn host_bridge_request_path_is_under_state_root(request_path: &Path, state_root: &Path) -> bool {
    let Ok(request_path) = std::fs::canonicalize(request_path) else {
        return false;
    };
    let Ok(state_root) = std::fs::canonicalize(state_root) else {
        return false;
    };
    request_path.starts_with(state_root)
}

async fn host_bridge_request_provenance_blockers_for_state_root(
    state_root: &Path,
    request_path: &Path,
    request: &serde_json::Value,
) -> Vec<String> {
    let mut blockers = Vec::new();
    let canonical_state_root = match std::fs::canonicalize(&state_root) {
        Ok(path) => path,
        Err(_) => {
            blockers.push("host_bridge_state_root_missing".to_string());
            return blockers;
        }
    };
    let canonical_request_path =
        match canonical_state_artifact_path(&state_root, &request_path.display().to_string(), true)
        {
            Ok(path) => path,
            Err(_) => {
                blockers.push("host_bridge_request_untrusted_path".to_string());
                return blockers;
            }
        };
    let declared_request_path = match host_bridge_request_string(request, "request_path") {
        Some(path) => path,
        None => {
            blockers.push("host_bridge_request_path_missing".to_string());
            return blockers;
        }
    };
    match canonical_state_artifact_path(&state_root, declared_request_path, true) {
        Ok(path) if path == canonical_request_path => {}
        _ => blockers.push("host_bridge_request_path_mismatch".to_string()),
    }
    let packet_path = host_bridge_request_string(request, "packet_path");
    let canonical_packet_path =
        packet_path.and_then(
            |path| match canonical_state_artifact_path(&state_root, path, true) {
                Ok(path) => Some(path),
                Err(_) => {
                    blockers.push("host_bridge_packet_path_unbounded".to_string());
                    None
                }
            },
        );
    for (field, code) in [
        ("result_path", "host_bridge_result_path_unbounded"),
        ("receipt_path", "host_bridge_receipt_path_unbounded"),
    ] {
        if let Some(path) = host_bridge_request_string(request, field) {
            if canonical_state_artifact_path(&state_root, path, false).is_err() {
                blockers.push(code.to_string());
            }
        }
    }
    let Some(run_id) = host_bridge_request_string(request, "run_id") else {
        return blockers;
    };
    let store = match StateStore::open_existing_structural_read_only_with_timeout(
        canonical_state_root,
        HOST_BRIDGE_PROVENANCE_LOCK_TIMEOUT,
    )
    .await
    {
        Ok(store) => store,
        Err(_) => {
            blockers.push("host_bridge_dispatch_receipt_missing".to_string());
            return blockers;
        }
    };
    append_host_bridge_dispatch_receipt_blockers(
        &mut blockers,
        &store,
        state_root,
        request,
        run_id,
        canonical_packet_path.as_deref(),
    )
    .await;
    blockers
}

async fn append_host_bridge_dispatch_receipt_blockers(
    blockers: &mut Vec<String>,
    store: &StateStore,
    state_root: &Path,
    request: &serde_json::Value,
    run_id: &str,
    canonical_packet_path: Option<&Path>,
) {
    let receipt = match store.run_graph_dispatch_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Err(_) => {
            blockers.push("host_bridge_dispatch_receipt_missing".to_string());
            return;
        }
        Ok(None) => {
            blockers.push("host_bridge_dispatch_receipt_missing".to_string());
            return;
        }
    };
    if !matches!(
        receipt.dispatch_status.as_str(),
        "routed" | "executing" | "bridge_request_pending"
    ) {
        blockers.push("host_bridge_dispatch_receipt_inactive".to_string());
    }
    if host_bridge_request_string(request, "dispatch_target")
        != Some(receipt.dispatch_target.as_str())
        || host_bridge_request_string(request, "backend_id") != receipt.selected_backend.as_deref()
        || canonical_packet_path
            .as_ref()
            .map(|path| path.display().to_string())
            != receipt.dispatch_packet_path.as_ref().and_then(|path| {
                canonical_state_artifact_path(&state_root, path, true)
                    .ok()
                    .map(|path| path.display().to_string())
            })
    {
        blockers.push("host_bridge_dispatch_receipt_mismatch".to_string());
    }
}

fn host_bridge_operator_fields(
    status: &str,
    blocker_codes: Vec<String>,
    shared_next_actions: Vec<String>,
    operator_next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
) -> (serde_json::Value, serde_json::Value) {
    let spec = crate::operator_contracts::OperatorContractSpec {
        contract_id: "host-agent-bridge-adapter-v1",
        schema_version: "1",
        pass_status: "pass",
        blocked_status: "blocked",
        canonicalize_status: crate::operator_contracts::canonical_pass_blocked_contract_status_str,
        status_error_label: "canonical pass/blocked",
    };
    let mut verdict = crate::operator_contracts::finalize_operator_surface_verdict(
        &spec,
        status,
        blocker_codes,
        operator_next_actions,
        artifact_refs,
    );
    verdict.shared_fields["next_actions"] = serde_json::json!(shared_next_actions);
    (verdict.shared_fields, verdict.operator_contracts)
}

fn legacy_internal_subagents_host_bridge_request(request: &serde_json::Value) -> bool {
    request
        .get("backend_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some("internal_subagents")
        && request
            .get("dispatch_transport")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            == Some("host_tool_bridge")
        && (request
            .get("adapter_kind")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            == Some("unconfigured_host_agent_adapter")
            || request
                .get("adapter_capability_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                == Some("unconfigured_host_agent_capability")
            || request
                .get("invocation_mode")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                == Some("configured_host_capability_required"))
}

fn effective_host_bridge_request(request: &serde_json::Value) -> serde_json::Value {
    if !legacy_internal_subagents_host_bridge_request(request) {
        return request.clone();
    }
    let mut effective = request.clone();
    if let Some(object) = effective.as_object_mut() {
        object.insert(
            "adapter_kind".to_string(),
            serde_json::json!("codex_host_tools"),
        );
        object.insert(
            "adapter_capability_id".to_string(),
            serde_json::json!("codex.multi_agent_v1"),
        );
        object.insert(
            "invocation_mode".to_string(),
            serde_json::json!("parent_host_tool_api"),
        );
        object
            .entry("receipt_mode".to_string())
            .or_insert_with(|| serde_json::json!("host_bridge_receipt"));
        object.insert(
            "adapter_contract_source".to_string(),
            serde_json::json!("legacy_internal_subagents_default"),
        );
        let adapter_params = object
            .entry("adapter_params".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(params) = adapter_params.as_object_mut() {
            params.insert(
                "tool_family".to_string(),
                serde_json::json!("codex_multi_agent"),
            );
            params.insert(
                "spawn_tool".to_string(),
                serde_json::json!("multi_agent_v1.spawn_agent"),
            );
            params.insert(
                "wait_tool".to_string(),
                serde_json::json!("multi_agent_v1.wait_agent"),
            );
            params.insert(
                "close_tool".to_string(),
                serde_json::json!("multi_agent_v1.close_agent"),
            );
        }
    }
    effective
}

fn host_bridge_adapter_payload(
    request_path: &Path,
    request: &serde_json::Value,
    provenance_blockers: Vec<String>,
) -> serde_json::Value {
    let effective_request = effective_host_bridge_request(request);
    let request = &effective_request;
    let mut missing = Vec::new();
    let run_id = host_bridge_required_string(request, "run_id", &mut missing);
    let dispatch_target = host_bridge_required_string(request, "dispatch_target", &mut missing);
    let packet_path = host_bridge_required_string(request, "packet_path", &mut missing);
    let backend_id = host_bridge_required_string(request, "backend_id", &mut missing);
    let carrier_id = host_bridge_required_string(request, "carrier_id", &mut missing);
    let adapter_kind = host_bridge_required_string(request, "adapter_kind", &mut missing);
    let adapter_capability_id =
        host_bridge_required_string(request, "adapter_capability_id", &mut missing);
    let result_path = host_bridge_required_string(request, "result_path", &mut missing);
    let receipt_path = host_bridge_required_string(request, "receipt_path", &mut missing);
    let request_status = request
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or("unknown");
    let dispatch_transport = request
        .get("dispatch_transport")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let invocation_mode = request
        .get("invocation_mode")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("parent_host_tool_api");
    let adapter_contract_source = request
        .get("adapter_contract_source")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or("request");
    let mut blocker_codes = provenance_blockers;
    if !missing.is_empty() {
        blocker_codes.push("host_bridge_request_missing_fields".to_string());
    }
    if dispatch_transport != Some("host_tool_bridge") {
        blocker_codes.push("host_bridge_request_wrong_transport".to_string());
    }
    if request_status != "pending" {
        blocker_codes.push("host_bridge_request_not_pending".to_string());
    }
    if adapter_capability_id != Some("codex.multi_agent_v1") {
        blocker_codes.push("host_tool_capability_missing".to_string());
    }
    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let receipt_id = match (run_id, dispatch_target) {
        (Some(run_id), Some(dispatch_target)) => {
            format!("{run_id}-{dispatch_target}-host-bridge-receipt")
        }
        _ => "host-bridge-receipt".to_string(),
    };
    let host_agent_id_placeholder = "<host-agent-id>";
    let completion_command = if let Some(run_id) = run_id {
        format!(
            "vida lane complete {} --receipt-id {} --host-bridge-request {} --host-agent-id {} --host-bridge-summary {} --json",
            crate::shell_quote(run_id),
            crate::shell_quote(&receipt_id),
            crate::shell_quote(&request_path.display().to_string()),
            crate::shell_quote(host_agent_id_placeholder),
            crate::shell_quote("parent host adapter completed receipt-backed execution")
        )
    } else {
        "repair host bridge request run_id before completion".to_string()
    };
    let host_tool_calls = if status == "pass" {
        serde_json::json!([
            {
                "tool": "multi_agent_v1.spawn_agent",
                "purpose": "start the selected parent-host subagent for the bounded dispatch packet",
                "adapter_kind": adapter_kind,
                "adapter_capability_id": adapter_capability_id,
                "packet_path": packet_path,
                "backend_id": backend_id,
                "carrier_id": carrier_id
            },
            {
                "tool": "multi_agent_v1.wait_agent",
                "purpose": "wait for receipt-backed completion evidence from the spawned host agent"
            },
            {
                "tool": "multi_agent_v1.close_agent",
                "purpose": "release host thread capacity after completion or blocked result capture"
            }
        ])
    } else {
        serde_json::json!([])
    };
    let adapter_capacity_status = if status == "pass" {
        "ready_to_attempt"
    } else {
        "not_checked_due_request_blockers"
    };
    let adapter_capacity = serde_json::json!({
        "status": adapter_capacity_status,
        "capacity_observable": false,
        "capacity_source": "parent_host_tool_runtime",
        "active_agents_count": serde_json::Value::Null,
        "thread_limit_reached": serde_json::Value::Null,
        "blocked_result_code": "host_agent_capacity_unavailable",
        "next_actions": [
            "Invoke multi_agent_v1.spawn_agent from the parent host session when capacity is available.",
            "If the parent host tool reports thread or capacity exhaustion, close stale host agents or write a blocked host bridge result with blocker_code host_agent_capacity_unavailable."
        ]
    });
    let next_actions = if status == "pass" {
        vec![completion_command.clone()]
    } else {
        vec![
            "repair the host bridge request or selected host adapter capability before invoking parent host tools"
                .to_string(),
        ]
    };
    let artifact_refs = serde_json::json!({
        "request_path": request_path.display().to_string(),
        "packet_path": packet_path,
        "result_path": result_path,
        "receipt_path": receipt_path
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs,
    );
    serde_json::json!({
        "surface": "vida agent host-bridge",
        "status": status,
        "blocker_codes": blocker_codes,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "host_bridge": {
            "request_path": request_path.display().to_string(),
            "request_status": request_status,
            "run_id": run_id,
            "dispatch_target": dispatch_target,
            "packet_path": packet_path,
            "backend_id": backend_id,
            "carrier_id": carrier_id,
            "dispatch_transport": dispatch_transport,
            "adapter_kind": adapter_kind,
            "adapter_capability_id": adapter_capability_id,
            "invocation_mode": invocation_mode,
            "adapter_contract_source": adapter_contract_source,
            "missing_fields": missing,
            "result_path": result_path,
            "receipt_path": receipt_path,
            "receipt_id": receipt_id,
            "completion_command": completion_command,
            "host_tool_calls": host_tool_calls,
            "adapter_capacity": adapter_capacity,
            "blocked_result_contract": {
                "execution_state": "blocked",
                "allowed_blocker_codes": [
                    "host_agent_capacity_unavailable",
                    "host_tool_capability_missing",
                    "host_agent_execution_failed"
                ]
            },
            "binary_boundary": "vida.exe emits and validates bridge artifacts; parent host adapter invokes native host tools"
        }
    })
}

fn emit_host_bridge_payload(payload: &serde_json::Value, as_json: bool) -> ExitCode {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(payload)
                .expect("host bridge adapter payload should render")
        );
    } else {
        let mut fields = vec![crate::operator_toon_report::OperatorToonField::text(
            "status",
            payload["status"].as_str().unwrap_or("unknown"),
        )];
        if payload["status"].as_str() == Some("pass") {
            if let Some(command) = payload["host_bridge"]["completion_command"].as_str() {
                fields.push(crate::operator_toon_report::OperatorToonField::text(
                    "completion",
                    crate::operator_command_text::human_command(command),
                ));
            }
        }
        if let Some(blockers) = payload["blocker_codes"].as_array() {
            if !blockers.is_empty() {
                fields.push(crate::operator_toon_report::OperatorToonField::value(
                    "blocker_codes",
                    serde_json::Value::Array(blockers.clone()),
                ));
            }
        }
        crate::operator_toon_report::print("vida agent host-bridge", fields);
    }
    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn host_bridge_completion_lane_args(
    request_path: &Path,
    payload: &serde_json::Value,
    host_agent_id: &str,
    summary: Option<&str>,
    receipt_id_override: Option<&str>,
    state_dir: Option<&Path>,
) -> Result<Vec<String>, String> {
    let run_id = payload["host_bridge"]["run_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "host bridge request payload is missing run_id".to_string())?;
    let receipt_id = receipt_id_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| payload["host_bridge"]["receipt_id"].as_str())
        .ok_or_else(|| "host bridge request payload is missing receipt_id".to_string())?;
    let summary = summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("parent host adapter completed receipt-backed execution");
    let mut args = vec![
        "complete".to_string(),
        run_id.to_string(),
        "--receipt-id".to_string(),
        receipt_id.to_string(),
        "--host-bridge-request".to_string(),
        request_path.display().to_string(),
        "--host-agent-id".to_string(),
        host_agent_id.to_string(),
        "--host-bridge-summary".to_string(),
        summary.to_string(),
    ];
    if let Some(state_dir) = state_dir {
        args.push("--state-dir".to_string());
        args.push(state_dir.display().to_string());
    }
    args.push("--json".to_string());
    Ok(args)
}

fn build_parallelization_planner(
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
) -> serde_json::Value {
    let ready_parallel_safe = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .count();
    let independent_failures = projection
        .blocked
        .iter()
        .filter(|candidate| !candidate.ready_now)
        .count();
    let triggers = [
        (
            "coverage_or_test_expansion",
            projection.ready.iter().any(|candidate| {
                let title = candidate.task.title.to_ascii_lowercase();
                let work_item_keys = task_flow_lookup_keys(&candidate.task).join(" ");
                let labels = candidate
                    .task
                    .labels
                    .iter()
                    .map(|label| label.to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");
                title.contains("test")
                    || title.contains("coverage")
                    || work_item_keys.contains("verification")
                    || labels.contains("verification")
                    || labels.contains("quality")
            }),
        ),
        (
            "three_or_more_independent_failures",
            independent_failures >= 3,
        ),
        (
            "parallel_safe_ready_candidates",
            ready_parallel_safe >= 2 && configured_max_parallel_agents > 1,
        ),
    ];
    let active_triggers = triggers
        .into_iter()
        .filter_map(|(trigger, active)| active.then(|| trigger.to_string()))
        .collect::<Vec<_>>();
    let packet_proposals = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .take(lanes_requested.min(configured_max_parallel_agents.max(1)))
        .map(|candidate| {
            serde_json::json!({
                "task_id": candidate.task.id,
                "title": candidate.task.title,
                "proposal_kind": "parallel_safe_dispatch_packet_preview",
                "materializes_packet": false,
                "next_surface": "vida agent-init",
                "reason": "candidate is ready and parallel-safe under TaskFlow scheduling projection"
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if packet_proposals.is_empty() { "no_packet_proposals" } else { "proposals_available" },
        "mode": "preview_only",
        "triggers": active_triggers,
        "ready_parallel_safe_count": ready_parallel_safe,
        "independent_failure_count": independent_failures,
        "packet_proposals": packet_proposals,
        "materializes_packets": false,
        "next_action": if ready_parallel_safe > 0 {
            "review selected lanes and launch with the shown `vida agent-init` command only after operator approval"
        } else {
            "add or unblock parallel-safe execution semantics before expecting planner proposals"
        }
    })
}

fn no_packet_materialization() -> serde_json::Value {
    serde_json::json!({
        "status": "not_requested",
        "requested": false,
        "materializes_packets": false,
        "artifacts": [],
    })
}

fn build_carrier_selection_api_descriptor(
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let dev_team_roles = activation_bundle["dev_team_readiness"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let api_id = role["role_id"].as_str()?.trim();
            let runtime_role = role["runtime_role"].as_str()?.trim();
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .find(|value| !value.is_empty())?;
            if api_id.is_empty() || runtime_role.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "api_id": api_id,
                "runtime_role": runtime_role,
                "task_class": task_class,
                "selection_surface": "vida agent select",
                "selection_materialized": false,
                "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
            }))
        })
        .collect::<Vec<_>>();
    let first_class = if dev_team_roles.is_empty() {
        activation_bundle["carrier_runtime"]["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| {
                let api_id = role["role_id"].as_str()?.trim();
                let runtime_role = role["default_runtime_role"]
                    .as_str()
                    .or_else(|| {
                        role["runtime_roles"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .find(|value| !value.trim().is_empty())
                    })?
                    .trim();
                let task_class = role["task_classes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .find(|value| !value.trim().is_empty())?
                    .trim();
                if api_id.is_empty() || runtime_role.is_empty() || task_class.is_empty() {
                    return None;
                }
                Some(serde_json::json!({
                    "api_id": api_id,
                    "runtime_role": runtime_role,
                    "task_class": task_class,
                    "selection_surface": "vida agent select",
                    "selection_materialized": false,
                    "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                    "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
                }))
            })
            .collect::<Vec<_>>()
    } else {
        dev_team_roles
    };
    serde_json::json!({
        "surface": "vida agent select",
        "mode": "config_driven_runtime_assignment",
        "status": if first_class.is_empty() { "blocked" } else { "pass" },
        "blocker_codes": if first_class.is_empty() {
            vec!["carrier_selection_api_requires_configured_dev_team_roles"]
        } else {
            Vec::<&str>::new()
        },
        "first_class_carriers": first_class,
        "manual_host_tool_choice_required": false,
        "embedded_assignment_diagnostics": false,
        "diagnostics_note": "Run the listed `vida agent select` command for full carrier/model/cost assignment diagnostics.",
    })
}

fn non_dev_team_flow_projection() -> serde_json::Value {
    serde_json::json!({
        "status": "not_applicable",
        "reason": "dev_team_preview_not_enabled",
        "diagnostic_only": true,
    })
}

fn lifecycle_hook_event_stream(
    selected_flow: Option<&serde_json::Value>,
    sequence: &[DevTeamSequenceStep],
) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    if let Some(flow) = selected_flow {
        for hook in flow["lifecycle_hook_templates"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "flow",
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.lifecycle_hook_templates",
            }));
        }
    }
    for (index, step) in sequence.iter().enumerate() {
        for hook in step
            .lifecycle_hook_templates
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "step",
                "step_index": index,
                "role_label": step.role_label,
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.steps.lifecycle_hook_templates",
            }));
        }
    }
    events
}

fn build_dev_team_flow_projection(
    activation_bundle: &serde_json::Value,
    selected_flow_id: Option<&str>,
    sequence: &[DevTeamSequenceStep],
    selected_lanes: &[AgentDispatchLanePreview],
    blocker_codes: &[String],
) -> serde_json::Value {
    let readiness = &activation_bundle["dev_team_readiness"];
    let selected_flow = selected_flow_id.and_then(|flow_id| {
        readiness["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
    });
    let current_lane = selected_lanes.first();
    let current_step = current_lane
        .map(|lane| {
            serde_json::json!({
                "role_label": lane.role_label,
                "runtime_role": lane.runtime_role,
                "task_class": lane.task_class,
                "task_id": lane.task_id,
                "dispatch_command": lane.dispatch_command,
                "dispatch_command_kind": lane.dispatch_command_kind,
                "receipt_status": {
                    "receipt_backed": false,
                    "receipt_path": null,
                    "status": "preview_only"
                },
                "proof_state": {
                    "status": "pending_dispatch",
                    "diagnostic_only": true
                },
                "approval_gate": lane.approval_gate,
            })
        })
        .or_else(|| {
            sequence.first().map(|step| {
                serde_json::json!({
                    "role_label": step.role_label,
                    "runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                    "task_id": null,
                    "dispatch_command": null,
                    "dispatch_command_kind": null,
                    "receipt_status": {
                        "receipt_backed": false,
                        "receipt_path": null,
                        "status": "not_selected"
                    },
                    "proof_state": {
                        "status": "not_started",
                        "diagnostic_only": true
                    },
                    "approval_gate": {
                        "required": step.requires_user_approval,
                        "status": if step.requires_user_approval {
                            "approval_required_after_step_completion"
                        } else {
                            "not_required"
                        },
                        "policy": step.approval_policy,
                    },
                })
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let approval_waits = selected_lanes
        .iter()
        .filter(|lane| lane.requires_user_approval)
        .map(|lane| {
            serde_json::json!({
                "task_id": lane.task_id,
                "role_label": lane.role_label,
                "status": "approval_required_after_step_completion",
                "policy": lane.approval_gate["policy"],
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if blocker_codes.is_empty() { "ready" } else { "blocked" },
        "flow_id": selected_flow.and_then(|flow| flow["flow_id"].as_str()),
        "flow_class": selected_flow.and_then(|flow| flow["flow_class"].as_str()),
        "work_item_bindings": selected_flow
            .map(|flow| flow["work_item_bindings"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection": selected_flow
            .map(|flow| flow["adapter_projection"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection_source": "dev_team.flows.adapter_projection",
        "adapter_projection_is_data_only": true,
        "proof_gates": selected_flow
            .map(|flow| flow["proof_gates"].clone())
            .unwrap_or(serde_json::Value::Null),
        "current_step": current_step,
        "steps": sequence.iter().enumerate().map(|(index, step)| {
            serde_json::json!({
                "index": index,
                "role_label": step.role_label,
                "runtime_role": step.runtime_role,
                "task_class": step.task_class,
                "requires_user_approval": step.requires_user_approval,
                "approval_policy": step.approval_policy,
                "lifecycle_hook_templates": step.lifecycle_hook_templates,
                "resume_transitions": step.resume_transitions,
                "rework_transitions": step.rework_transitions,
            })
        }).collect::<Vec<_>>(),
        "approval_waits": approval_waits,
        "lifecycle_hook_event_stream": lifecycle_hook_event_stream(selected_flow, sequence),
        "receipt_status": {
            "receipt_backed": false,
            "receipt_path": null,
            "status": "preview_only"
        },
        "proof_state": {
            "status": "pending_dispatch",
            "diagnostic_only": true
        },
        "diagnostic_only": true,
    })
}

fn single_in_progress_task_id_from_rows(rows: &[state_store::TaskRecord]) -> Option<&str> {
    let mut candidates = rows
        .iter()
        .filter(|task| task.status == "in_progress" && task.issue_type != "epic");
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate.id.as_str())
}

fn configured_max_parallel_agents_from_activation_bundle(
    activation_bundle: &serde_json::Value,
) -> usize {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1)
}

fn agent_init_command(
    task_id: &str,
    state_dir: Option<&std::path::Path>,
    runtime_role: &str,
) -> String {
    let runtime_role = if runtime_role.trim().is_empty() {
        "worker"
    } else {
        runtime_role
    };
    let mut command = format!(
        "vida agent-init --role {} {} --json",
        crate::shell_quote(runtime_role),
        crate::shell_quote(task_id)
    );
    if let Some(state_dir) = state_dir {
        command.push_str(" --state-dir ");
        command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    }
    command
}

fn receipt_backed_execution_command_hint(task_id: &str) -> String {
    format!(
        "vida taskflow run-graph dispatch-init {} --json, then vida agent-init --dispatch-packet <packet-path> --execute-dispatch --json",
        crate::shell_quote(task_id)
    )
}

fn required_string_field(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload[key]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn selection_truth_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    selection_truth_for_task_with_role_and_class(activation_bundle, task, "worker", None, None)
}

fn selection_truth_for_task_with_role_and_class(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
    conversation_role: &str,
    runtime_role_override: Option<&str>,
    task_class_override: Option<&str>,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    let task_value = serde_json::to_value(task)
        .map_err(|error| format!("task_record_serialization_failed:{error}"))?;
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let task_class = task_class_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or(inferred_task_class);
    let runtime_role = runtime_role_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| crate::runtime_role_for_task_class(&task_class).to_string());
    let assignment = crate::build_runtime_assignment_preview_from_resolved_constraints(
        activation_bundle,
        conversation_role,
        &task_class,
        &runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        let reason = required_string_field(&assignment, "reason")
            .unwrap_or_else(|| "runtime_assignment_disabled".to_string());
        return Err(reason);
    }

    let selected_carrier = required_string_field(&assignment, "selected_carrier_id")
        .ok_or_else(|| "selected_carrier_id_missing".to_string())?;
    let selected_backend = required_string_field(&assignment, "selected_backend_id")
        .ok_or_else(|| "selected_backend_id_missing".to_string())?;
    let selected_model_profile = required_string_field(&assignment, "selected_model_profile_id")
        .ok_or_else(|| "selected_model_profile_id_missing".to_string())?;
    let selected_model_ref = required_string_field(&assignment, "selected_model_ref")
        .ok_or_else(|| "selected_model_ref_missing".to_string())?;
    let selected_reasoning_effort = required_string_field(&assignment, "selected_reasoning_effort")
        .ok_or_else(|| "selected_reasoning_effort_missing".to_string())?;
    let budget_verdict = required_string_field(&assignment, "budget_verdict")
        .ok_or_else(|| "budget_verdict_missing".to_string())?;
    let selected_over_budget = assignment["selected_over_budget"]
        .as_bool()
        .unwrap_or(false);
    let selected_model_profile_readiness_status =
        required_string_field(&assignment, "selected_model_profile_readiness_status")
            .unwrap_or_else(|| "unknown".to_string());
    let pricing_freshness_status = assignment["pricing_readiness"]["pricing_freshness_status"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string();
    let selected_external_backend_readiness_status = assignment
        ["selected_external_backend_readiness"]["status"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("not_applicable")
        .to_string();
    let rate = assignment["rate"]
        .as_u64()
        .ok_or_else(|| "rate_missing".to_string())?;
    let estimated_task_price_units = assignment["estimated_task_price_units"]
        .as_u64()
        .ok_or_else(|| "estimated_task_price_units_missing".to_string())?;

    Ok(AgentDispatchLaneSelectionTruth {
        selected_carrier,
        selected_backend,
        selected_model_profile,
        selected_model_ref,
        selected_reasoning_effort,
        rate,
        estimated_task_price_units,
        budget_verdict,
        selected_over_budget,
        selected_model_profile_readiness_status,
        pricing_freshness_status,
        selected_external_backend_readiness_status,
        selection_source_paths: assignment["selection_source_paths"].clone(),
        pricing_readiness: assignment["pricing_readiness"].clone(),
        runtime_role,
        task_class,
    })
}

fn selection_truth_guard_blockers(truth: &AgentDispatchLaneSelectionTruth) -> Vec<String> {
    let mut blockers = Vec::new();
    if truth.selected_over_budget && truth.budget_verdict == "over_budget" {
        blockers.push("selected_model_profile_over_budget".to_string());
    }
    if truth.selected_model_profile_readiness_status == "blocked" {
        blockers.push("selected_model_profile_not_ready".to_string());
    }
    if matches!(
        truth.selected_external_backend_readiness_status.as_str(),
        "external_backend_dispatch_blocked" | "blocked"
    ) {
        blockers.push("selected_external_backend_not_ready".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn agent_dispatch_host_bridge_capacity_guard() -> serde_json::Value {
    serde_json::json!({
        "status": "parent_host_capacity_unobservable",
        "capacity_observable": false,
        "capacity_source": "parent_host_tool_runtime",
        "active_agents_count": serde_json::Value::Null,
        "thread_limit_reached": serde_json::Value::Null,
        "blocked_result_code": "host_agent_capacity_unavailable",
        "next_actions": [
            "Attempt the parent host bridge only after dispatch admission is otherwise clean.",
            "If the parent host tool reports thread or capacity exhaustion, close stale host agents or write a blocked host bridge result with blocker_code host_agent_capacity_unavailable."
        ]
    })
}

fn agent_dispatch_fanout_guard_from_projection(
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    selected_lanes: &[AgentDispatchLanePreview],
    blocked_candidates: &[AgentDispatchBlockedCandidate],
    blocker_codes: &[String],
) -> serde_json::Value {
    let effective_max_parallel_agents = if lanes_requested == 0 {
        0
    } else {
        lanes_requested.min(configured_max_parallel_agents.max(1))
    };
    let ready_parallel_safe_count = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .count();
    let cap_limited_count = blocked_candidates
        .iter()
        .filter(|candidate| {
            candidate.reasons.iter().any(|reason| {
                reason == "effective_max_parallel_agents_cap_reached"
                    || reason == "max_parallel_agents_cap_reached"
            })
        })
        .count();
    let conflict_rejected_count = blocked_candidates
        .iter()
        .filter(|candidate| {
            candidate.reasons.iter().any(|reason| {
                reason.starts_with("conflict_domain_already_selected:")
                    || reason.starts_with("owned_path_already_selected:")
            })
        })
        .count();
    let unsafe_ready_count = blocked_candidates
        .iter()
        .filter(|candidate| candidate.ready_now && !candidate.ready_parallel_safe)
        .count();
    let assignment_blockers = selected_lanes
        .iter()
        .flat_map(|lane| selection_truth_guard_blockers(&lane.selection_truth))
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if blocker_codes.is_empty() && assignment_blockers.is_empty() { "pass" } else { "blocked" },
        "configured_max_parallel_agents": configured_max_parallel_agents.max(1),
        "lanes_requested": lanes_requested,
        "effective_max_parallel_agents": effective_max_parallel_agents,
        "lanes_selected": selected_lanes.len(),
        "ready_parallel_safe_count": ready_parallel_safe_count,
        "cap_limited_rejected_count": cap_limited_count,
        "conflict_rejected_count": conflict_rejected_count,
        "unsafe_ready_rejected_count": unsafe_ready_count,
        "assignment_blocker_codes": assignment_blockers,
        "host_bridge_capacity": agent_dispatch_host_bridge_capacity_guard(),
        "blocker_codes": blocker_codes,
    })
}

fn agent_dispatch_fanout_guard_from_scheduler_plan(
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    selected_lanes: &[AgentDispatchLanePreview],
    blocked_candidates: &[AgentDispatchBlockedCandidate],
    blocker_codes: &[String],
) -> serde_json::Value {
    let mut guard = plan.fanout_guard.clone();
    if let Some(object) = guard.as_object_mut() {
        let assignment_blocker_codes = selected_lanes
            .iter()
            .flat_map(|lane| selection_truth_guard_blockers(&lane.selection_truth))
            .collect::<Vec<_>>();
        object.insert(
            "status".to_string(),
            serde_json::json!(
                if blocker_codes.is_empty() && assignment_blocker_codes.is_empty() {
                    "pass"
                } else {
                    "blocked"
                }
            ),
        );
        object.insert(
            "lanes_selected".to_string(),
            serde_json::json!(selected_lanes.len()),
        );
        object.insert(
            "assignment_blocker_codes".to_string(),
            serde_json::json!(assignment_blocker_codes),
        );
        object.insert(
            "agent_preview_blocker_codes".to_string(),
            serde_json::json!(blocker_codes),
        );
        object.insert(
            "agent_preview_rejected_count".to_string(),
            serde_json::json!(blocked_candidates.len()),
        );
        object.insert(
            "host_bridge_capacity".to_string(),
            agent_dispatch_host_bridge_capacity_guard(),
        );
    }
    guard
}

fn blocked_candidate(
    candidate: &state_store::TaskSchedulingCandidate,
    reasons: Vec<String>,
) -> AgentDispatchBlockedCandidate {
    AgentDispatchBlockedCandidate {
        task_id: candidate.task.id.clone(),
        title: candidate.task.title.clone(),
        ready_now: candidate.ready_now,
        ready_parallel_safe: candidate.ready_parallel_safe,
        reasons,
        parallel_blockers: candidate.parallel_blockers.clone(),
    }
}

fn explicit_task_graph_continuation_task_id(
    binding: Option<&state_store::RunGraphContinuationBinding>,
) -> Option<&str> {
    let binding = binding?;
    if binding.status != "bound" || binding.binding_source != "explicit_continuation_bind_task" {
        return None;
    }
    if !matches!(
        binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str),
        Some("task_graph_task" | "run_graph_task")
    ) {
        return None;
    }
    binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
        .or_else(|| {
            let task_id = binding.task_id.trim();
            (!task_id.is_empty()).then_some(task_id)
        })
}

fn materialization_owned_paths_for_lane_task(
    task: state_store::TaskRecord,
    lane: &AgentDispatchLanePreview,
) -> Vec<String> {
    if lane.task_class == crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION {
        Vec::new()
    } else {
        task.planner_metadata.owned_paths
    }
}

fn build_agent_dispatch_next_preview(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    dev_team: bool,
) -> AgentDispatchNextPreview {
    if dev_team {
        build_agent_dispatch_next_preview_dev_team(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    } else {
        build_agent_dispatch_next_preview_standard(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    }
}

fn build_agent_dispatch_next_preview_standard(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);

    let Some(primary) = projection.ready.first() else {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(format!(
            "Inspect `{}` and resolve blockers before previewing agent dispatch.",
            human_command("vida task ready --json")
        ));
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = non_dev_team_flow_projection();
        let fanout_guard = agent_dispatch_fanout_guard_from_projection(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            &selected_lanes,
            &blocked_candidates,
            &blocker_codes,
        );
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: build_parallelization_planner(
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            packet_materialization: no_packet_materialization(),
            carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
            fanout_guard,
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    };

    if effective_max_parallel_agents > 0 {
        match selection_truth_for_task(activation_bundle, &primary.task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: 1,
                task_id: primary.task.id.clone(),
                title: primary.task.title.clone(),
                role_label: "default".to_string(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &primary.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &primary.task.id,
                ),
                ready_parallel_safe: primary.ready_parallel_safe,
                selection_reason: "primary_ready_task".to_string(),
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                primary.task.id, reason
            )),
        }
    }

    let mut remaining = effective_max_parallel_agents.saturating_sub(selected_lanes.len());
    for candidate in projection.ready.iter().skip(1) {
        if candidate.ready_parallel_safe && remaining > 0 {
            match selection_truth_for_task(activation_bundle, &candidate.task) {
                Ok(selection_truth) => {
                    selected_lanes.push(AgentDispatchLanePreview {
                        lane_index: selected_lanes.len() + 1,
                        task_id: candidate.task.id.clone(),
                        title: candidate.task.title.clone(),
                        role_label: "parallel".to_string(),
                        runtime_role: selection_truth.runtime_role.clone(),
                        task_class: selection_truth.task_class.clone(),
                        dispatch_command: agent_init_command(
                            &candidate.task.id,
                            explicit_state_dir,
                            &selection_truth.runtime_role,
                        ),
                        dispatch_command_kind: "startup_activation_view_only".to_string(),
                        receipt_backed_execution_command: receipt_backed_execution_command_hint(
                            &candidate.task.id,
                        ),
                        ready_parallel_safe: candidate.ready_parallel_safe,
                        selection_reason: "parallel_safe_ready_task".to_string(),
                        selection_truth,
                        requires_user_approval: false,
                        approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
                    });
                    remaining -= 1;
                }
                Err(reason) => blocker_codes.push(format!(
                    "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                    candidate.task.id, reason
                )),
            }
            continue;
        }

        let reasons = if candidate.ready_parallel_safe {
            vec!["effective_max_parallel_agents_cap_reached".to_string()]
        } else if candidate.parallel_blockers.is_empty() {
            vec!["parallel_safety_not_established".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        blocked_candidates.push(blocked_candidate(candidate, reasons));
    }

    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    let unsafe_ready_candidates = blocked_candidates
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    if effective_max_parallel_agents > 1 && unsafe_ready_candidates && selected_lanes.is_empty() {
        blocker_codes.push("ambiguous_unsafe_parallel_candidates".to_string());
        next_actions.push(
            "Some ready candidates are not parallel-safe; reduce to `--lanes 1` or fix execution semantics/conflicts before multi-lane dispatch."
                .to_string(),
        );
    } else if effective_max_parallel_agents > 1 && unsafe_ready_candidates {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one chosen lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    let assignment_guard_blockers = selected_lanes
        .iter()
        .flat_map(|lane| {
            selection_truth_guard_blockers(&lane.selection_truth)
                .into_iter()
                .map(move |blocker| {
                    format!(
                        "selected_lane_assignment_guard_blocked:task={}:{}",
                        lane.task_id, blocker
                    )
                })
        })
        .collect::<Vec<_>>();
    if !assignment_guard_blockers.is_empty() {
        for blocker in assignment_guard_blockers {
            if !blocker_codes.iter().any(|code| code == &blocker) {
                blocker_codes.push(blocker);
            }
        }
        selected_lanes.clear();
        blocker_codes.push("selected_lane_assignment_guard_required".to_string());
        next_actions.push(
            "Selection truth has budget, readiness, or backend blockers; fix assignment guard evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let fanout_guard = agent_dispatch_fanout_guard_from_projection(
        projection,
        lanes_requested,
        configured_max_parallel_agents,
        &selected_lanes,
        &blocked_candidates,
        &blocker_codes,
    );

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: build_parallelization_planner(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        fanout_guard,
        flow_projection: non_dev_team_flow_projection(),
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn build_agent_dispatch_next_preview_dev_team(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();
    let current_task_matches = projection
        .current_task_id
        .as_deref()
        .map(|current_task_id| {
            projection
                .ready
                .iter()
                .filter(|candidate| candidate.task.id == current_task_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let all_ready_flow_ids = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now)
        .filter_map(|candidate| {
            selected_dev_team_flow_for_task(
                &activation_bundle["dev_team_readiness"],
                &candidate.task,
            )
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let has_unsafe_ready_candidates = projection
        .ready
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    let scoped_current_task_dev_team = projection.current_task_id.is_some()
        && current_task_matches.len() == 1
        && (lanes_requested <= 1
            || projection.ready.len() == 1
            || all_ready_flow_ids.len() > 1
            || has_unsafe_ready_candidates);
    let selected_ready_candidates = if scoped_current_task_dev_team {
        current_task_matches
    } else {
        projection.ready.iter().collect::<Vec<_>>()
    };
    let ready_flow_ids = selected_ready_candidates
        .iter()
        .filter(|candidate| candidate.ready_now)
        .filter_map(|candidate| {
            selected_dev_team_flow_for_task(
                &activation_bundle["dev_team_readiness"],
                &candidate.task,
            )
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let sequence = if ready_flow_ids.len() == 1 {
        selected_ready_candidates
            .iter()
            .find(|candidate| candidate.ready_now)
            .map(|candidate| dev_team_sequence_for_task(activation_bundle, &candidate.task))
            .unwrap_or_else(|| dev_team_sequence(activation_bundle))
    } else {
        dev_team_sequence(activation_bundle)
    };
    let selected_flow_id = if ready_flow_ids.len() == 1 {
        ready_flow_ids.iter().next().map(String::as_str)
    } else {
        activation_bundle["dev_team_readiness"]["default_flow_id"].as_str()
    };

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    if sequence.is_empty() {
        blocker_codes.push("configured_dev_team_sequence_required".to_string());
        next_actions.push(
            "Configure dev_team_readiness roles/sequence or dispatch_contract lanes before previewing dev-team dispatch."
                .to_string(),
        );
    }
    if projection.current_task_id.is_none() && ready_flow_ids.len() > 1 {
        blocker_codes.push("ambiguous_work_item_flow_selection".to_string());
        next_actions.push(
            "Ready task candidates map to multiple configured dev_team flows; narrow the task scope or dispatch one flow class at a time."
                .to_string(),
        );
    }

    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);
    let preview_step_limit = effective_max_parallel_agents;
    let steps_to_preview = sequence
        .iter()
        .cloned()
        .take(preview_step_limit)
        .collect::<Vec<_>>();
    if projection.ready.is_empty() {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(format!(
            "Inspect `{}` and resolve blockers before previewing dev-team dispatch.",
            human_command("vida task ready --json")
        ));
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = build_dev_team_flow_projection(
            activation_bundle,
            selected_flow_id,
            &sequence,
            &selected_lanes,
            &blocker_codes,
        );
        let fanout_guard = agent_dispatch_fanout_guard_from_projection(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            &selected_lanes,
            &blocked_candidates,
            &blocker_codes,
        );
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview-dev-team".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: build_parallelization_planner(
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            packet_materialization: no_packet_materialization(),
            carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
            fanout_guard,
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    }

    let mut ready_index = 0;
    for (step_index, step) in steps_to_preview.into_iter().enumerate() {
        if !step.requires_task {
            next_actions.push(format!(
                "dev-team step [{}] {} is closure-oriented and does not emit a runtime launch command.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
            continue;
        }
        if step.requires_user_approval {
            next_actions.push(format!(
                "dev-team step [{}] {} will pause after receipt-backed completion for configured user approval before the next role starts.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
        }
        let candidate = if scoped_current_task_dev_team {
            selected_ready_candidates.first().copied()
        } else {
            selected_ready_candidates.get(ready_index).copied()
        };
        if !scoped_current_task_dev_team {
            ready_index += usize::from(candidate.is_some());
        }
        let Some(candidate) = candidate else {
            blocker_codes.push(format!(
                "dev_team_step_missing_ready_task:position={}:{}",
                step_index + 1,
                step.role_label
            ));
            break;
        };
        if !candidate.ready_now {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["task_not_ready_for_dev_team_step".to_string()],
            ));
            continue;
        }
        if projection.current_task_id.is_none()
            && effective_max_parallel_agents > 1
            && !candidate.ready_parallel_safe
        {
            continue;
        }
        match selection_truth_for_task_with_role_and_class(
            activation_bundle,
            &candidate.task,
            &step.runtime_role,
            Some(&step.runtime_role),
            Some(&step.task_class),
        ) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: selected_lanes.len() + 1,
                task_id: candidate.task.id.clone(),
                title: candidate.task.title.clone(),
                role_label: step.role_label.clone(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &candidate.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &candidate.task.id,
                ),
                ready_parallel_safe: candidate.ready_parallel_safe,
                selection_reason: format!("dev_team_step_{}:{}", step_index + 1, step.role_label),
                selection_truth,
                requires_user_approval: step.requires_user_approval,
                approval_gate: serde_json::json!({
                    "required": step.requires_user_approval,
                    "status": if step.requires_user_approval {
                        "approval_required_after_step_completion"
                    } else {
                        "not_required"
                    },
                    "policy": step.approval_policy,
                    "lifecycle_hook_templates": step.lifecycle_hook_templates,
                    "resume_transitions": step.resume_transitions,
                    "rework_transitions": step.rework_transitions,
                    "prompt_template_source": if step.requires_user_approval {
                        "dev_team.flows.steps.approval_policy"
                    } else {
                        "none"
                    },
                }),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                candidate.task.id, reason
            )),
        }
    }

    let blocked_ready_parallel = projection
        .ready
        .iter()
        .filter(|candidate| {
            Some(candidate.task.id.as_str()) != projection.current_task_id.as_deref()
                && !candidate.ready_parallel_safe
        })
        .collect::<Vec<_>>();
    for candidate in blocked_ready_parallel {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["parallel_safety_not_established".to_string()],
        ));
    }
    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one configured dev-team step; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
        next_actions.push(
            "The shown `vida agent-init --role` command is startup activation view only; receipt-backed execution requires a dispatch packet and `--execute-dispatch`."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let flow_projection = build_dev_team_flow_projection(
        activation_bundle,
        selected_flow_id,
        &sequence,
        &selected_lanes,
        &blocker_codes,
    );
    let fanout_guard = agent_dispatch_fanout_guard_from_projection(
        projection,
        lanes_requested,
        configured_max_parallel_agents,
        &selected_lanes,
        &blocked_candidates,
        &blocker_codes,
    );

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview-dev-team".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: build_parallelization_planner(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        fanout_guard,
        flow_projection,
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn scheduler_task_record<'a>(
    plan: &'a crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> Option<&'a state_store::TaskRecord> {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .map(|candidate| &candidate.task)
}

fn scheduler_task_parallel_safety(
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> bool {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .is_some_and(|candidate| candidate.ready_parallel_safe)
}

fn build_agent_dispatch_next_preview_from_scheduler_plan(
    activation_bundle: &serde_json::Value,
    plan: crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    lanes_requested: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = plan.blocker_codes.clone();
    let mut next_actions = plan.next_actions.clone();
    let mut selected_lanes = Vec::new();
    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let blocked_candidates = plan
        .rejected_candidates
        .iter()
        .map(|candidate| AgentDispatchBlockedCandidate {
            task_id: candidate.task_id.clone(),
            title: candidate.task.title.clone(),
            ready_now: candidate.ready_now,
            ready_parallel_safe: candidate.ready_now && candidate.parallel_blockers.is_empty(),
            reasons: candidate.reasons.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        })
        .collect::<Vec<_>>();

    for (index, reservation) in plan.reservations.iter().enumerate() {
        let Some(task) = scheduler_task_record(&plan, &reservation.task_id) else {
            blocker_codes.push(format!(
                "selected_lane_task_record_missing:task={}",
                reservation.task_id
            ));
            continue;
        };
        match selection_truth_for_task(activation_bundle, task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: index + 1,
                task_id: reservation.task_id.clone(),
                title: reservation.task.title.clone(),
                role_label: if reservation.launch_role == "primary" {
                    "default".to_string()
                } else {
                    reservation.launch_role.clone()
                },
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &reservation.task_id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &reservation.task_id,
                ),
                ready_parallel_safe: scheduler_task_parallel_safety(&plan, &reservation.task_id),
                selection_reason: if reservation.launch_role == "primary" {
                    "scheduler_primary_ready_task".to_string()
                } else {
                    "scheduler_parallel_safe_ready_task".to_string()
                },
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                reservation.task_id, reason
            )),
        }
    }

    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
        || blocker_codes
            .iter()
            .any(|code| code.starts_with("selected_lane_task_record_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one scheduler-selected lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    let assignment_guard_blockers = selected_lanes
        .iter()
        .flat_map(|lane| {
            selection_truth_guard_blockers(&lane.selection_truth)
                .into_iter()
                .map(move |blocker| {
                    format!(
                        "selected_lane_assignment_guard_blocked:task={}:{}",
                        lane.task_id, blocker
                    )
                })
        })
        .collect::<Vec<_>>();
    if !assignment_guard_blockers.is_empty() {
        for blocker in assignment_guard_blockers {
            if !blocker_codes.iter().any(|code| code == &blocker) {
                blocker_codes.push(blocker);
            }
        }
        selected_lanes.clear();
        blocker_codes.push("selected_lane_assignment_guard_required".to_string());
        next_actions.push(
            "Selection truth has budget, readiness, or backend blockers; fix assignment guard evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if plan.max_parallel_agents > 1
        && blocked_candidates
            .iter()
            .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe)
    {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }
    if lanes_requested == 0 {
        selected_lanes.clear();
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    }
    .to_string();
    let configured_parallel =
        usize::try_from(plan.configured_max_parallel_agents).unwrap_or(usize::MAX);
    let effective_parallel = if lanes_requested == 0 {
        0
    } else {
        usize::try_from(plan.max_parallel_agents).unwrap_or(usize::MAX)
    };
    let mut parallelization_planner =
        build_parallelization_planner(&plan.scheduling, lanes_requested, effective_parallel);
    apply_scheduler_plan_continuation_gate_to_parallelization_planner(
        &mut parallelization_planner,
        &plan,
    );
    let fanout_guard = agent_dispatch_fanout_guard_from_scheduler_plan(
        &plan,
        &selected_lanes,
        &blocked_candidates,
        &blocker_codes,
    );
    AgentDispatchNextPreview {
        status,
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents: configured_parallel,
        effective_max_parallel_agents: effective_parallel,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner,
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        fanout_guard,
        flow_projection: non_dev_team_flow_projection(),
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn apply_scheduler_plan_continuation_gate_to_parallelization_planner(
    planner: &mut serde_json::Value,
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
) {
    let blocked_by_continuation_gate = plan.selected_task_ids.is_empty()
        && plan.blocker_codes.iter().any(|code| {
            matches!(
                code.as_str(),
                "continuation_binding_ambiguous"
                    | "open_delegated_cycle"
                    | "latest_run_graph_status_blocked"
            )
        });
    if !blocked_by_continuation_gate {
        return;
    }

    let proposals = plan
        .selected_parallel_tasks
        .iter()
        .map(|task| {
            serde_json::json!({
                "task_id": task.id,
                "title": task.title,
                "proposal_kind": "parallel_safe_dispatch_packet_preview",
                "materializes_packet": false,
                "next_surface": "vida agent-init",
                "reason": "candidate remains visible as diagnostic-only evidence while continuation gate blocks execution"
            })
        })
        .collect::<Vec<_>>();
    if let Some(object) = planner.as_object_mut() {
        object.insert(
            "status".to_string(),
            serde_json::json!(if proposals.is_empty() {
                "no_packet_proposals"
            } else {
                "proposals_available"
            }),
        );
        object.insert("packet_proposals".to_string(), serde_json::json!(proposals));
        object.insert("materializes_packets".to_string(), serde_json::json!(false));
        object.insert("diagnostic_only".to_string(), serde_json::json!(true));
        object.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        object.insert(
            "continuation_gate_scope".to_string(),
            serde_json::json!("task_scoped"),
        );
        object.insert(
            "independent_parallel_available".to_string(),
            serde_json::json!(!plan.selected_parallel_tasks.is_empty()),
        );
    }
}

fn apply_continuation_dispatch_gate_to_preview(
    preview: &mut AgentDispatchNextPreview,
    gate: &crate::taskflow_proxy::TaskflowContinuationDispatchGate,
) {
    if gate.admissible {
        return;
    }

    let blocked_task_ids = gate
        .blocked_task_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    preview.status = "blocked".to_string();
    preview.selected_lanes.clear();
    preview.lanes_selected = 0;
    if let Some(blocker) = crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::LatestRunGraphStatusBlocked,
    ) {
        if !preview.blocker_codes.iter().any(|value| value == &blocker) {
            preview.blocker_codes.push(blocker);
        }
    }
    for blocker in &gate.blocker_codes {
        if !preview.blocker_codes.iter().any(|value| value == blocker) {
            preview.blocker_codes.push(blocker.clone());
        }
    }
    preview.next_actions.clear();
    for action in &gate.next_actions {
        if !preview.next_actions.iter().any(|value| value == action) {
            preview.next_actions.push(action.clone());
        }
    }
    if preview.next_actions.is_empty() {
        preview.next_actions.push(
            crate::status_surface_signals::continuation_binding_ambiguous_next_action().to_string(),
        );
    }
    fail_closed_flow_projection_for_continuation_gate(preview);
    if let Some(planner) = preview.parallelization_planner.as_object_mut() {
        let mut proposals_available = false;
        if !blocked_task_ids.is_empty() {
            if let Some(proposals) = planner
                .get_mut("packet_proposals")
                .and_then(serde_json::Value::as_array_mut)
            {
                proposals.retain(|proposal| {
                    proposal
                        .get("task_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .is_some_and(|task_id| !blocked_task_ids.contains(task_id))
                });
                proposals_available = !proposals.is_empty();
            }
            planner.insert(
                "continuation_gate_blocked_task_ids".to_string(),
                serde_json::json!(blocked_task_ids.iter().cloned().collect::<Vec<_>>()),
            );
        } else {
            proposals_available = planner
                .get("packet_proposals")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|proposals| !proposals.is_empty());
        }
        planner.insert(
            "status".to_string(),
            serde_json::json!(if proposals_available {
                "proposals_available"
            } else {
                "no_packet_proposals"
            }),
        );
        planner.insert("materializes_packets".to_string(), serde_json::json!(false));
        planner.insert("diagnostic_only".to_string(), serde_json::json!(true));
        planner.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        planner.insert(
            "continuation_gate_scope".to_string(),
            serde_json::json!(if blocked_task_ids.is_empty() {
                "global"
            } else {
                "task_scoped"
            }),
        );
        planner.insert(
            "independent_parallel_available".to_string(),
            serde_json::json!(proposals_available),
        );
    }
}

fn fail_closed_flow_projection_for_continuation_gate(preview: &mut AgentDispatchNextPreview) {
    let blocked_proof_state = serde_json::json!({
        "status": "blocked_by_continuation_gate",
        "diagnostic_only": true
    });
    if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
        flow_projection.insert("status".to_string(), serde_json::json!("blocked"));
        flow_projection.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        flow_projection.insert(
            "blocker_codes".to_string(),
            serde_json::json!(preview.blocker_codes),
        );
        flow_projection.insert(
            "next_actions".to_string(),
            serde_json::json!(preview.next_actions),
        );
        flow_projection.insert("proof_state".to_string(), blocked_proof_state.clone());
        if let Some(current_step) = flow_projection
            .get_mut("current_step")
            .and_then(serde_json::Value::as_object_mut)
        {
            current_step.insert("dispatch_command".to_string(), serde_json::Value::Null);
            current_step.insert("dispatch_command_kind".to_string(), serde_json::Value::Null);
            current_step.insert("proof_state".to_string(), blocked_proof_state);
            current_step.insert(
                "blocked_by_continuation_gate".to_string(),
                serde_json::json!(true),
            );
        }
    }
}

pub(crate) fn dispatch_target_for_dev_team_task_class(task_class: &str) -> &'static str {
    match task_class {
        "specification" | "planning" | "analysis" => "specification",
        "execution_preparation" | "architecture" => "execution_preparation",
        "coach" | "review" | "validation" => "coach",
        "verification" | "quality_gate" | "release_readiness" => "verification",
        _ => "implementer",
    }
}

fn dispatch_target_for_agent_dispatch_lane(lane: &AgentDispatchLanePreview) -> &'static str {
    dispatch_target_for_dev_team_task_class(&lane.task_class)
}

fn validate_materialized_agent_dispatch_packet(
    lane: &AgentDispatchLanePreview,
    expected_dispatch_target: &str,
    dispatch_packet_path: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let packet_path = Path::new(dispatch_packet_path);
    if dispatch_packet_path.trim().is_empty() {
        return Err("dispatch packet path is empty".to_string());
    }
    if !packet_path.exists() {
        return Err(format!(
            "dispatch packet path does not exist: {dispatch_packet_path}"
        ));
    }
    let raw = std::fs::read_to_string(packet_path).map_err(|error| {
        format!("Failed to read materialized dispatch packet `{dispatch_packet_path}`: {error}")
    })?;
    let packet = serde_json::from_str::<serde_json::Value>(&raw).map_err(|error| {
        format!("Failed to parse materialized dispatch packet `{dispatch_packet_path}`: {error}")
    })?;
    if packet["run_id"].as_str() != Some(lane.task_id.as_str()) {
        return Err(format!(
            "materialized packet run_id mismatch: expected `{}`, got `{}`",
            lane.task_id,
            packet["run_id"].as_str().unwrap_or("<missing>")
        ));
    }
    if packet["dispatch_target"].as_str() != Some(expected_dispatch_target) {
        return Err(format!(
            "materialized packet dispatch_target mismatch: expected `{expected_dispatch_target}`, got `{}`",
            packet["dispatch_target"].as_str().unwrap_or("<missing>")
        ));
    }
    if receipt.run_id != lane.task_id {
        return Err(format!(
            "dispatch receipt run_id mismatch: expected `{}`, got `{}`",
            lane.task_id, receipt.run_id
        ));
    }
    if receipt.dispatch_target != expected_dispatch_target {
        return Err(format!(
            "dispatch receipt target mismatch: expected `{expected_dispatch_target}`, got `{}`",
            receipt.dispatch_target
        ));
    }
    if receipt.dispatch_status != "routed" {
        return Err(format!(
            "dispatch receipt is not routed: status `{}`",
            receipt.dispatch_status
        ));
    }
    Ok(packet)
}

async fn materialize_configured_agent_dispatch_lane(
    lane: &AgentDispatchLanePreview,
    state_dir: &Path,
    activation_bundle: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let expected_dispatch_target = dispatch_target_for_agent_dispatch_lane(lane);
    let mut role_selection = crate::RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: "vida.config.yaml".to_string(),
        selection_mode: "configured_dev_team_dispatch_next".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: lane.task_id.clone(),
        selected_role: lane.runtime_role.clone(),
        conversational_mode: None,
        single_task_only: true,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "explicit_configured_dev_team_lane".to_string(),
        matched_terms: vec![lane.role_label.clone(), lane.task_class.clone()],
        compiled_bundle: activation_bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: format!(
            "materialize configured dev-team lane `{}` as `{expected_dispatch_target}`",
            lane.role_label
        ),
    };
    role_selection.execution_plan =
        crate::development_flow_orchestration::build_runtime_execution_plan_from_snapshot(
            activation_bundle,
            &role_selection,
        );
    let run_graph_bootstrap = serde_json::json!({
        "status": "dispatch_init_ready",
        "handoff_ready": true,
        "run_id": lane.task_id,
        "latest_status": {
            "run_id": lane.task_id,
            "status": "pass",
            "active_node": expected_dispatch_target,
            "next_node": expected_dispatch_target,
            "task_class": lane.task_class,
            "route_task_class": lane.task_class,
            "dispatch_ready": true,
            "dispatch_blockers": [],
        }
    });
    let taskflow_handoff_plan = crate::build_taskflow_handoff_plan(&role_selection);
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    crate::runtime_dispatch_state::sync_receipt_configured_activation_assignment(
        &role_selection,
        &mut dispatch_receipt,
    );
    dispatch_receipt.dispatch_command =
        crate::runtime_dispatch_command_for_target(&role_selection, expected_dispatch_target);
    let owned_paths_override =
        match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
            Ok(store) => {
                let owned_paths = store
                    .show_task(&lane.task_id)
                    .await
                    .ok()
                    .map(|task| materialization_owned_paths_for_lane_task(task, lane))
                    .unwrap_or_default();
                store.close().await;
                owned_paths
            }
            Err(_) => Vec::new(),
        };
    let ctx = crate::RuntimeDispatchPacketContext::new(
        state_dir,
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    )
    .with_owned_paths_override(owned_paths_override);
    let dispatch_packet_path = crate::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    if let Ok(store) = StateStore::open_existing(state_dir.to_path_buf()).await {
        store
            .record_run_graph_dispatch_receipt(&dispatch_receipt)
            .await
            .map_err(|error| format!("Failed to record dev-team dispatch receipt: {error}"))?;
        store.close().await;
    }
    let packet = validate_materialized_agent_dispatch_packet(
        lane,
        expected_dispatch_target,
        &dispatch_packet_path,
        &dispatch_receipt,
    )?;
    let mut agent_init_execute_command = format!(
        "vida agent-init --dispatch-packet {} --execute-dispatch",
        crate::shell_quote(&dispatch_packet_path)
    );
    agent_init_execute_command.push_str(" --state-dir ");
    agent_init_execute_command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    Ok(serde_json::json!({
        "lane_index": lane.lane_index,
        "task_id": lane.task_id,
        "role_label": lane.role_label,
        "runtime_role": lane.runtime_role,
        "task_class": lane.task_class,
        "dispatch_packet_path": dispatch_packet_path,
        "dispatch_target": expected_dispatch_target,
        "packet_template_kind": packet["packet_template_kind"].clone(),
        "dispatch_receipt_id": dispatch_receipt.recorded_at,
        "dispatch_receipt": dispatch_receipt,
        "agent_init_execute_command": agent_init_execute_command,
        "machine_command": format!("{agent_init_execute_command} --json"),
        "receipt_backed": true,
        "status": "packet_ready",
    }))
}

async fn materialize_agent_dispatch_next_packets(
    mut preview: AgentDispatchNextPreview,
    state_dir: &std::path::Path,
    activation_bundle: &serde_json::Value,
) -> AgentDispatchNextPreview {
    if preview.status != "pass" {
        preview.packet_materialization = serde_json::json!({
            "status": "blocked",
            "requested": true,
            "materializes_packets": false,
            "reason": "dispatch preview is blocked",
            "blocker_codes": preview.blocker_codes,
            "artifacts": [],
        });
        return preview;
    }
    if preview.selected_lanes.is_empty() {
        preview.status = "blocked".to_string();
        preview
            .blocker_codes
            .push("no_dispatch_lanes_selected".to_string());
        preview.packet_materialization = serde_json::json!({
            "status": "blocked",
            "requested": true,
            "materializes_packets": false,
            "reason": "no selected lanes can be materialized",
            "artifacts": [],
        });
        return preview;
    }

    let mut artifacts = Vec::new();
    let mut errors = Vec::new();
    for lane in &preview.selected_lanes {
        match materialize_configured_agent_dispatch_lane(lane, state_dir, activation_bundle).await {
            Ok(artifact) => artifacts.push(artifact),
            Err(error) => {
                let blocker = format!("packet_materialization_failed:task={}", lane.task_id);
                if !preview.blocker_codes.iter().any(|value| value == &blocker) {
                    preview.blocker_codes.push(blocker);
                }
                errors.push(serde_json::json!({
                    "task_id": lane.task_id,
                    "role_label": lane.role_label,
                    "error": error,
                }));
            }
        }
    }

    if errors.is_empty() {
        preview.mode = if preview.mode == "preview-dev-team" {
            "materialized-dev-team".to_string()
        } else {
            "materialized".to_string()
        };
        preview.next_actions.retain(|action| {
            !action.contains("Preview only:") && !action.contains("startup activation view only")
        });
        if let Some(first) = artifacts
            .first()
            .and_then(|artifact| artifact["agent_init_execute_command"].as_str())
        {
            preview.next_actions.push(format!(
                "Run `{first}` to execute the first receipt-backed dispatch packet."
            ));
        }
        if let Some(planner) = preview.parallelization_planner.as_object_mut() {
            planner.insert(
                "mode".to_string(),
                serde_json::json!("materialized_packets"),
            );
            planner.insert("materializes_packets".to_string(), serde_json::json!(true));
            planner.insert(
                "packet_artifacts".to_string(),
                serde_json::json!(artifacts.clone()),
            );
        }
        if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
            flow_projection.insert("diagnostic_only".to_string(), serde_json::json!(false));
            flow_projection.insert(
                "receipt_status".to_string(),
                serde_json::json!({
                    "receipt_backed": true,
                    "status": "packet_ready",
                    "artifacts": artifacts,
                }),
            );
            if let Some(current_step) = flow_projection
                .get_mut("current_step")
                .and_then(serde_json::Value::as_object_mut)
            {
                if let Some(first) = artifacts.first() {
                    current_step.insert(
                        "dispatch_command".to_string(),
                        first["agent_init_execute_command"].clone(),
                    );
                    current_step.insert(
                        "dispatch_command_kind".to_string(),
                        serde_json::json!("receipt_backed_dispatch_packet"),
                    );
                    current_step.insert(
                        "receipt_status".to_string(),
                        serde_json::json!({
                            "receipt_backed": true,
                            "receipt_path": first["dispatch_packet_path"],
                            "status": "packet_ready",
                        }),
                    );
                    current_step.insert(
                        "proof_state".to_string(),
                        serde_json::json!({
                            "status": "pending_receipt_backed_execution",
                            "diagnostic_only": false,
                        }),
                    );
                }
            }
        }
        preview.packet_materialization = serde_json::json!({
            "status": "pass",
            "requested": true,
            "materializes_packets": true,
            "artifacts": preview.parallelization_planner["packet_artifacts"].clone(),
        });
    } else {
        preview.status = "blocked".to_string();
        preview.packet_materialization = serde_json::json!({
            "status": "blocked",
            "requested": true,
            "materializes_packets": false,
            "errors": errors,
            "artifacts": artifacts,
        });
    }
    preview
}

fn safe_agent_dispatch_projection_component(value: &str) -> String {
    let mut safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    safe.truncate(120);
    if safe.is_empty() {
        "none".to_string()
    } else {
        safe
    }
}

fn agent_dispatch_next_projection_name(
    command: &AgentDispatchNextArgs,
    materialize_packets: bool,
) -> String {
    let materialization_mode = if materialize_packets {
        "-materialized"
    } else {
        ""
    };
    format!(
        "agent-dispatch-next-mode-{}{}-lanes-{}-scope-{}-current-{}-latest",
        if command.dev_team {
            "dev-team"
        } else {
            "scheduler"
        },
        materialization_mode,
        command.lanes,
        safe_agent_dispatch_projection_component(command.scope.as_deref().unwrap_or("default")),
        safe_agent_dispatch_projection_component(
            command.current_task_id.as_deref().unwrap_or("default")
        ),
    )
}

fn dev_team_config_default_materializes_packets(activation_bundle: &serde_json::Value) -> bool {
    activation_bundle
        .pointer("/dev_team_readiness/orchestrator_command_contract/default_args")
        .or_else(|| {
            activation_bundle.pointer("/dev_team/orchestrator_command_contract/default_args")
        })
        .and_then(serde_json::Value::as_array)
        .map(|args| {
            args.iter()
                .any(|arg| arg.as_str() == Some("--materialize-packets"))
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentDispatchNextCurrentTaskIds<'a> {
    preview_current_task_id: Option<&'a str>,
    scheduler_current_task_id: Option<&'a str>,
}

fn resolve_agent_dispatch_next_current_task_ids<'a>(
    requested_current_task_id: Option<&'a str>,
    explicit_bound_current_task_id: Option<&'a str>,
    taskflow_single_in_progress_task_id: Option<&'a str>,
) -> AgentDispatchNextCurrentTaskIds<'a> {
    AgentDispatchNextCurrentTaskIds {
        preview_current_task_id: requested_current_task_id
            .or(explicit_bound_current_task_id)
            .or(taskflow_single_in_progress_task_id),
        scheduler_current_task_id: requested_current_task_id
            .or(explicit_bound_current_task_id)
            .or(taskflow_single_in_progress_task_id),
    }
}

fn emit_agent_dispatch_next_preview(
    command: &AgentDispatchNextArgs,
    state_dir: &std::path::Path,
    projection_name: &str,
    preview: AgentDispatchNextPreview,
) -> ExitCode {
    if command.json {
        let payload =
            serde_json::to_value(&preview).expect("agent dispatch-next preview should serialize");
        crate::print_json_pretty(&payload);
        crate::operator_projection_cache::write_json_projection(
            state_dir,
            projection_name,
            &payload,
        );
    } else {
        println!("agent dispatch-next: {}", preview.status);
        println!("lanes selected: {}", preview.lanes_selected);
        if preview.packet_materialization["requested"]
            .as_bool()
            .unwrap_or(false)
        {
            println!(
                "packet materialization: {}",
                preview.packet_materialization["status"]
                    .as_str()
                    .unwrap_or("unknown")
            );
        } else {
            println!(
                "preview only: review carrier/model/cost selection truth before launching any `vida agent-init` command"
            );
        }
        for lane in &preview.selected_lanes {
            println!(
                "lane {} [{}]: {} [{} / {} / rate={} / est_cost={}]",
                lane.lane_index,
                lane.role_label,
                lane.task_id,
                lane.selection_truth.selected_carrier,
                lane.selection_truth.selected_model_ref,
                lane.selection_truth.rate,
                lane.selection_truth.estimated_task_price_units
            );
        }
        if !preview.blocker_codes.is_empty() {
            println!("blockers: {}", preview.blocker_codes.join(", "));
        }
        if let Some(first_command) = preview
            .packet_materialization
            .get("artifacts")
            .and_then(serde_json::Value::as_array)
            .and_then(|artifacts| artifacts.first())
            .and_then(|artifact| artifact["agent_init_execute_command"].as_str())
        {
            println!("next: {first_command}");
        }
    }
    if preview.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(crate) async fn run_agent(args: AgentArgs) -> ExitCode {
    match args.command {
        AgentCommand::DispatchNext(command) => run_agent_dispatch_next(command).await,
        AgentCommand::Select(command) => run_agent_select(command).await,
        AgentCommand::HostBridge(command) => run_agent_host_bridge(command).await,
        AgentCommand::Status(command) => run_agent_status(command).await,
    }
}

async fn run_agent_status(command: AgentStatusArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let store = match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["project_activation_unknown".to_string()],
                vec![format!(
                    "open the authoritative state store before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };

    let latest_status = match store.latest_run_graph_status().await {
        Ok(status) => status,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_status_unreadable".to_string()],
                vec![format!(
                    "repair run-graph status evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };
    let latest_receipt = match store.latest_run_graph_dispatch_receipt_summary().await {
        Ok(receipt) => receipt,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_dispatch_receipt_unreadable".to_string()],
                vec![format!(
                    "repair dispatch receipt evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };
    let latest_recovery = match store.latest_run_graph_recovery_summary().await {
        Ok(recovery) => recovery,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_recovery_unreadable".to_string()],
                vec![format!(
                    "repair recovery evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };

    let current_run_id = latest_status
        .as_ref()
        .map(|status| status.run_id.clone())
        .or_else(|| {
            latest_receipt
                .as_ref()
                .map(|receipt| receipt.run_id.clone())
        });
    let latest_receipt = latest_receipt.filter(|receipt| {
        current_run_id
            .as_deref()
            .map(|run_id| run_id == receipt.run_id)
            .unwrap_or(true)
    });
    let latest_recovery = latest_recovery.filter(|summary| {
        current_run_id
            .as_deref()
            .map(|run_id| run_id == summary.run_id)
            .unwrap_or(true)
    });
    let active_lanes_count = latest_status
        .as_ref()
        .filter(|status| {
            !matches!(
                status.lifecycle_stage.as_str(),
                "closure_complete" | "completed" | "lane_completed"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let active_agents_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            matches!(
                receipt.dispatch_status.as_str(),
                "routed" | "pending" | "bridge_request_pending" | "blocked"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let handoff_pending_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt.downstream_dispatch_ready
                || matches!(
                    receipt.dispatch_status.as_str(),
                    "routed" | "bridge_request_pending"
                )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let view_only_dispatch_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt
                .effective_execution_posture
                .get("activation_evidence_state")
                .and_then(|value| value.as_str())
                == Some("activation_view_only")
        })
        .map(|_| 1)
        .unwrap_or(0);
    let blocked_dispatch_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt.dispatch_status == "blocked"
                || receipt
                    .blocker_code
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
        })
        .map(|_| 1)
        .unwrap_or(0);
    let reclaimable_lanes = latest_recovery
        .as_ref()
        .filter(|summary| {
            !summary.delegation_gate.delegated_cycle_open
                && matches!(
                    summary.lifecycle_stage.as_str(),
                    "closure_complete" | "completed" | "lane_completed"
                )
        })
        .map(|summary| vec![summary.run_id.clone()])
        .unwrap_or_default();
    let next_recovery_command = latest_recovery.as_ref().and_then(|summary| {
        if summary.delegation_gate.delegated_cycle_open {
            Some(format!(
                "vida taskflow recovery status {}",
                crate::shell_quote(&summary.run_id)
            ))
        } else if active_lanes_count > 0 {
            current_run_id.as_ref().map(|run_id| {
                format!(
                    "vida taskflow recovery status {}",
                    crate::shell_quote(run_id)
                )
            })
        } else if !reclaimable_lanes.is_empty() {
            Some(format!(
                "vida taskflow settle --run-id {}",
                crate::shell_quote(&summary.run_id)
            ))
        } else {
            None
        }
    });
    let mut blocker_codes = Vec::new();
    if blocked_dispatch_count > 0 {
        blocker_codes.push(
            latest_receipt
                .as_ref()
                .and_then(|receipt| receipt.blocker_code.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "blocked_dispatch".to_string()),
        );
    }
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        next_recovery_command
            .as_ref()
            .map(|command| vec![format!("run `{command}`")])
            .unwrap_or_default()
    };
    let artifact_refs = serde_json::json!({
        "surface": "vida agent status",
        "latest_run_id": current_run_id
            .clone()
            .or_else(|| latest_recovery.as_ref().map(|summary| summary.run_id.clone())),
        "latest_dispatch_packet_path": latest_receipt
            .as_ref()
            .and_then(|receipt| receipt.dispatch_packet_path.clone()),
        "state_dir": state_dir.display().to_string(),
    });
    let extra_fields = serde_json::json!({
        "view": if command.compact { "compact" } else { "compact" },
        "active_agents_count": active_agents_count,
        "active_lanes_count": active_lanes_count,
        "handoff_pending_count": handoff_pending_count,
        "view_only_dispatch_count": view_only_dispatch_count,
        "blocked_dispatch_count": blocked_dispatch_count,
        "reclaimable_lanes": reclaimable_lanes,
        "next_recovery_command": next_recovery_command,
    });
    let payload = agent_status_payload(blocker_codes, next_actions, artifact_refs, extra_fields);
    let success = payload["status"].as_str() == Some("pass");
    print_agent_status_payload(&payload, command.json);
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn agent_status_payload(
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    crate::operator_contracts::build_release1_operator_output_payload(
        "vida agent status",
        blocker_codes,
        next_actions,
        artifact_refs,
        extra_fields,
    )
    .expect("agent status operator payload should be valid")
}

fn print_agent_status_payload(payload: &serde_json::Value, json: bool) {
    if json {
        crate::print_json_pretty(payload);
        return;
    }
    crate::operator_toon_report::print(
        "vida agent status",
        vec![
            crate::operator_toon_report::OperatorToonField::value(
                "status",
                payload["status"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "active_agents_count",
                payload["active_agents_count"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "active_lanes_count",
                payload["active_lanes_count"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "handoff_pending_count",
                payload["handoff_pending_count"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "blocked_dispatch_count",
                payload["blocked_dispatch_count"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "reclaimable_lanes",
                payload["reclaimable_lanes"].clone(),
            ),
            crate::operator_toon_report::OperatorToonField::value(
                "next_recovery_command",
                payload["next_recovery_command"].clone(),
            ),
        ],
    );
}

async fn run_agent_host_bridge(command: AgentHostBridgeArgs) -> ExitCode {
    if command.complete
        && command
            .host_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        let blocker_codes = vec!["host_agent_id_missing".to_string()];
        let next_actions = vec![
            "provide --host-agent-id from the parent host adapter before completing the lane"
                .to_string(),
        ];
        let artifact_refs = serde_json::json!({
            "request_path": command.request.display().to_string()
        });
        let (shared_fields, operator_contracts) = host_bridge_operator_fields(
            "blocked",
            blocker_codes.clone(),
            next_actions.clone(),
            next_actions,
            artifact_refs,
        );
        let payload = serde_json::json!({
            "surface": "vida agent host-bridge",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts
        });
        return emit_host_bridge_payload(&payload, command.json);
    }
    match read_host_bridge_request(&command.request) {
        Ok(request) => {
            let payload = host_bridge_adapter_payload(
                &command.request,
                &request,
                host_bridge_request_provenance_blockers(
                    &command.request,
                    &request,
                    command.state_dir.as_deref(),
                )
                .await,
            );
            if command.complete {
                if payload["status"].as_str() != Some("pass") {
                    return emit_host_bridge_payload(&payload, command.json);
                }
                let Some(host_agent_id) = command
                    .host_agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    let blocker_codes = vec!["host_agent_id_missing".to_string()];
                    let next_actions = vec![
                        "provide --host-agent-id from the parent host adapter before completing the lane"
                            .to_string(),
                    ];
                    let artifact_refs = payload
                        .get("operator_contracts")
                        .and_then(|contracts| contracts.get("artifact_refs"))
                        .cloned()
                        .unwrap_or_else(|| {
                            serde_json::json!({
                                "request_path": command.request.display().to_string()
                            })
                        });
                    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                        "blocked",
                        blocker_codes.clone(),
                        next_actions.clone(),
                        next_actions,
                        artifact_refs,
                    );
                    let mut blocked = payload.clone();
                    if let Some(object) = blocked.as_object_mut() {
                        object.insert("status".to_string(), serde_json::json!("blocked"));
                        object.insert(
                            "blocker_codes".to_string(),
                            serde_json::json!(blocker_codes),
                        );
                        object.insert("shared_fields".to_string(), shared_fields);
                        object.insert("operator_contracts".to_string(), operator_contracts);
                    }
                    return emit_host_bridge_payload(&blocked, command.json);
                };
                let lane_args = match host_bridge_completion_lane_args(
                    &command.request,
                    &payload,
                    host_agent_id,
                    command.summary.as_deref(),
                    command.receipt_id.as_deref(),
                    command.state_dir.as_deref(),
                ) {
                    Ok(args) => args,
                    Err(error) => {
                        let blocker_codes = vec!["host_bridge_completion_args_invalid".to_string()];
                        let artifact_refs = serde_json::json!({
                            "request_path": command.request.display().to_string()
                        });
                        let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                            "blocked",
                            blocker_codes.clone(),
                            vec![error.clone()],
                            vec!["repair the host bridge request before completion".to_string()],
                            artifact_refs,
                        );
                        let blocked = serde_json::json!({
                            "surface": "vida agent host-bridge",
                            "status": "blocked",
                            "blocker_codes": blocker_codes,
                            "shared_fields": shared_fields,
                            "operator_contracts": operator_contracts
                        });
                        return emit_host_bridge_payload(&blocked, command.json);
                    }
                };
                return crate::lane_surface::run_lane(crate::ProxyArgs { args: lane_args }).await;
            }
            emit_host_bridge_payload(&payload, command.json)
        }
        Err(error) => {
            let blocker_codes = vec!["host_bridge_request_unreadable".to_string()];
            let next_actions =
                vec!["provide a readable host_tool_bridge_request JSON artifact".to_string()];
            let artifact_refs = serde_json::json!({
                "request_path": command.request.display().to_string()
            });
            let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                "blocked",
                blocker_codes.clone(),
                next_actions.clone(),
                next_actions,
                artifact_refs,
            );
            let payload = serde_json::json!({
                "surface": "vida agent host-bridge",
                "status": "blocked",
                "blocker_codes": blocker_codes,
                "shared_fields": shared_fields,
                "operator_contracts": operator_contracts,
                "error": error
            });
            emit_host_bridge_payload(&payload, command.json)
        }
    }
}

async fn run_agent_select(command: AgentSelectArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            let activation_bundle = match crate::build_taskflow_consume_bundle_payload(&store).await
            {
                Ok(payload) => payload.activation_bundle,
                Err(error) => {
                    eprintln!("Failed to load activation bundle for carrier selection: {error}");
                    return ExitCode::from(1);
                }
            };
            let selection = crate::build_runtime_assignment_from_resolved_constraints(
                &activation_bundle,
                &command.conversation_role,
                &command.task_class,
                &command.runtime_role,
            );
            let status = if selection["enabled"].as_bool().unwrap_or(false) {
                "pass"
            } else {
                "blocked"
            };
            let payload = serde_json::json!({
                "surface": "vida agent select",
                "status": status,
                "mode": "config_driven_runtime_assignment",
                "runtime_role": command.runtime_role,
                "task_class": command.task_class,
                "conversation_role": command.conversation_role,
                "selection": selection,
                "manual_host_tool_choice_required": false,
                "source_surfaces": [
                    "vida.config.yaml",
                    "build_runtime_assignment_from_resolved_constraints",
                    "carrier_runtime.roles"
                ],
            });
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                println!(
                    "agent select: {}",
                    payload["status"].as_str().unwrap_or("unknown")
                );
                if let Some(carrier) = payload["selection"]["selected_carrier_id"].as_str() {
                    println!("selected carrier: {carrier}");
                }
                if let Some(profile) = payload["selection"]["selected_model_profile_id"].as_str() {
                    println!("selected model profile: {profile}");
                }
            }
            if status == "pass" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_agent_dispatch_next(command: AgentDispatchNextArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let explicit_state_dir = command.state_dir.as_deref();
    let projection_name =
        agent_dispatch_next_projection_name(&command, command.materialize_packets);
    let cache_read_allowed =
        command.current_task_id.is_some() && !command.materialize_packets && !command.dev_team;
    if command.json && cache_read_allowed {
        if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
            &state_dir,
            &projection_name,
        ) {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_state_stale_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay(
                    &state_dir,
                )
            {
                if let Some(rendered) =
                    crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_payload(
                        &state_dir,
                        &cached,
                        &overlay,
                    )
                {
                    println!("{rendered}");
                    return ExitCode::SUCCESS;
                }
            }
        }
    }
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            let mut activation_bundle =
                match crate::read_or_sync_launcher_activation_snapshot(&store).await {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
            let explicit_binding = if command.current_task_id.is_none() {
                match store
                    .latest_explicit_run_graph_continuation_binding_for_current_session()
                    .await
                {
                    Ok(Some(binding)) => Some(binding),
                    Ok(None) => {
                        match store.latest_explicit_run_graph_continuation_binding().await {
                            Ok(binding) => binding,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read latest explicit continuation binding: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest explicit continuation binding: {error}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                None
            };
            let explicit_bound_current_task_id =
                explicit_task_graph_continuation_task_id(explicit_binding.as_ref())
                    .map(str::to_string);
            let taskflow_single_in_progress_task_id =
                if command.current_task_id.is_none() && explicit_bound_current_task_id.is_none() {
                    StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root())
                        .ok()
                        .and_then(|rows| {
                            single_in_progress_task_id_from_rows(&rows).map(str::to_string)
                        })
                } else {
                    None
                };
            let resolved_current_task_ids = resolve_agent_dispatch_next_current_task_ids(
                command.current_task_id.as_deref(),
                explicit_bound_current_task_id.as_deref(),
                taskflow_single_in_progress_task_id.as_deref(),
            );
            let preview = if command.dev_team {
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let projection =
                    match StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root()) {
                        Ok(rows) => {
                            let critical_path_ids = match StateStore::critical_path_from_rows(&rows)
                            {
                                Ok(path) => path
                                    .nodes
                                    .into_iter()
                                    .map(|node| node.id)
                                    .collect::<std::collections::BTreeSet<_>>(),
                                Err(_) => std::collections::BTreeSet::new(),
                            };
                            match StateStore::scheduling_projection_scoped_from_rows(
                                &rows,
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                                &critical_path_ids,
                            ) {
                                Ok(projection) => projection,
                                Err(error) => {
                                    eprintln!("Failed to compute agent dispatch preview: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(_) => match store
                            .scheduling_projection_scoped(
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                            )
                            .await
                        {
                            Ok(projection) => projection,
                            Err(error) => {
                                eprintln!("Failed to compute agent dispatch preview: {error}");
                                return ExitCode::from(1);
                            }
                        },
                    };
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let continuation_gate =
                    match crate::taskflow_proxy::build_taskflow_continuation_dispatch_gate_from_store(
                        &store,
                        &state_dir,
                        resolved_current_task_ids
                            .preview_current_task_id
                            .or(command.scope.as_deref()),
                    )
                    .await
                    {
                        Ok(gate) => gate,
                        Err(error) => {
                            eprintln!("Failed to compute agent continuation gate: {error}");
                            return ExitCode::from(1);
                        }
                    };
                drop(store);
                let mut preview = build_agent_dispatch_next_preview(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                );
                if let Some(gate) = continuation_gate {
                    apply_continuation_dispatch_gate_to_preview(&mut preview, &gate);
                }
                preview
            } else {
                let requested_parallel_limit = u64::try_from(command.lanes).ok();
                let plan =
                    match crate::taskflow_proxy::build_taskflow_scheduler_dispatch_plan_from_store(
                        &store,
                        &state_dir,
                        command.scope.as_deref(),
                        resolved_current_task_ids.scheduler_current_task_id,
                        requested_parallel_limit,
                        true,
                        false,
                    )
                    .await
                    {
                        Ok(plan) => plan,
                        Err(error) => {
                            eprintln!("Failed to compute agent dispatch preview: {error}");
                            return ExitCode::from(1);
                        }
                    };
                drop(store);
                build_agent_dispatch_next_preview_from_scheduler_plan(
                    &activation_bundle,
                    plan,
                    command.lanes,
                    explicit_state_dir,
                )
            };
            let effective_materialize_packets = if command.dev_team {
                command.materialize_packets
                    || dev_team_config_default_materializes_packets(&activation_bundle)
            } else {
                command.materialize_packets
            };
            let projection_name =
                agent_dispatch_next_projection_name(&command, effective_materialize_packets);
            let preview = if effective_materialize_packets {
                materialize_agent_dispatch_next_packets(preview, &state_dir, &activation_bundle)
                    .await
            } else {
                preview
            };
            emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
        }
        Err(error) => {
            if command.dev_team {
                let Some(current_task_id) = command.current_task_id.as_deref() else {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                };
                let project_root =
                    crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_dir)
                        .or_else(|| crate::resolve_runtime_project_root().ok());
                let Some(project_root) = project_root else {
                    eprintln!(
                        "Failed to resolve activation project root for state dir {}",
                        state_dir.display()
                    );
                    return ExitCode::from(1);
                };
                let mut activation_bundle = match capture_launcher_activation_snapshot_for_root(
                    &project_root,
                ) {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(snapshot_error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {snapshot_error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                let rows = match StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_dir) {
                    Ok(rows) => rows,
                    Err(fresh_error) => {
                        let snapshot_path =
                            StateStore::canonical_task_snapshot_path_for_state_root(&state_dir);
                        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
                            Ok(rows) => rows,
                            Err(snapshot_error) => {
                                eprintln!("Failed to open authoritative state store: {error}");
                                eprintln!(
                                    "Failed to read canonical task snapshot after authoritative open failure: {snapshot_error}; fresh snapshot error: {fresh_error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                };
                let critical_path_ids = match StateStore::critical_path_from_rows(&rows) {
                    Ok(path) => path
                        .nodes
                        .into_iter()
                        .map(|node| node.id)
                        .collect::<std::collections::BTreeSet<_>>(),
                    Err(_) => std::collections::BTreeSet::new(),
                };
                let projection = match StateStore::scheduling_projection_scoped_from_rows(
                    &rows,
                    command.scope.as_deref(),
                    Some(current_task_id),
                    &critical_path_ids,
                ) {
                    Ok(projection) => projection,
                    Err(projection_error) => {
                        eprintln!("Failed to compute agent dispatch preview: {projection_error}");
                        return ExitCode::from(1);
                    }
                };
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let effective_materialize_packets = command.materialize_packets
                    || dev_team_config_default_materializes_packets(&activation_bundle);
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let mut preview = build_agent_dispatch_next_preview(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                );
                preview.source_surfaces.push(
                    "StateStore::read_fresh_tasks_from_jsonl_snapshot(authoritative-open-fallback)"
                        .to_string(),
                );
                let projection_name =
                    agent_dispatch_next_projection_name(&command, effective_materialize_packets);
                let preview = if effective_materialize_packets {
                    materialize_agent_dispatch_next_packets(preview, &state_dir, &activation_bundle)
                        .await
                } else {
                    preview
                };
                emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
            } else {
                eprintln!("Failed to open authoritative state store: {error}");
                ExitCode::from(1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_continuation_dispatch_gate_to_preview, build_agent_dispatch_next_preview,
        configured_dev_team_first_step_for_task, dev_team_sequence, dev_team_sequence_for_task,
        dev_team_sequence_for_work_item, host_bridge_adapter_payload,
        host_bridge_completion_lane_args, host_bridge_request_provenance_blockers_for_state_root,
        infer_host_bridge_state_root_from_request_path, resolve_agent_dispatch_next_current_task_ids,
        single_in_progress_task_id_from_rows, state_store,
    };
    use crate::state_store::{
        CreateTaskRequest, RunGraphDispatchReceipt, TaskExecutionSemantics, TaskRecord,
        TaskSchedulingCandidate, TaskSchedulingProjection,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, EnvVarGuard};
    use crate::AgentDispatchNextArgs;
    use std::process::ExitCode;

    #[test]
    fn host_bridge_adapter_payload_renders_parent_host_tool_contract() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload =
            host_bridge_adapter_payload(std::path::Path::new("request.json"), &request, Vec::new());

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"].as_array().unwrap().len(), 0);
        assert_eq!(payload["shared_fields"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            payload["operator_contracts"]["next_actions"]
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            payload["operator_contracts"]["artifact_refs"]
        );
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "host-agent-bridge-adapter-v1"
        );
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .unwrap()
            .starts_with("vida lane complete run-1 "));
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .unwrap()
            .contains("--host-bridge-request request.json"));
        let calls = payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls should render");
        assert_eq!(calls[0]["tool"], "multi_agent_v1.spawn_agent");
        assert_eq!(calls[1]["tool"], "multi_agent_v1.wait_agent");
        assert_eq!(calls[2]["tool"], "multi_agent_v1.close_agent");
        assert_eq!(
            payload["host_bridge"]["blocked_result_contract"]["allowed_blocker_codes"][0],
            "host_agent_capacity_unavailable"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["status"],
            "ready_to_attempt"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["capacity_observable"],
            false
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["blocked_result_code"],
            "host_agent_capacity_unavailable"
        );
    }

    #[test]
    fn host_bridge_adapter_payload_normalizes_legacy_internal_subagents_adapter_contract() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "unconfigured_host_agent_adapter",
            "adapter_capability_id": "unconfigured_host_agent_capability",
            "invocation_mode": "configured_host_capability_required",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload =
            host_bridge_adapter_payload(std::path::Path::new("request.json"), &request, Vec::new());

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"].as_array().unwrap().len(), 0);
        assert_eq!(
            payload["host_bridge"]["adapter_capability_id"],
            "codex.multi_agent_v1"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_contract_source"],
            "legacy_internal_subagents_default"
        );
        let calls = payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls should render");
        assert_eq!(calls[0]["tool"], "multi_agent_v1.spawn_agent");
        assert_eq!(calls[0]["adapter_capability_id"], "codex.multi_agent_v1");
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_wrong_transport_and_capability() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "codex_cli_exec",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "missing.capability",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload =
            host_bridge_adapter_payload(std::path::Path::new("request.json"), &request, Vec::new());

        assert_eq!(payload["status"], "blocked");
        let blockers = payload["blocker_codes"]
            .as_array()
            .expect("blockers should render")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        assert!(blockers.contains(&"host_bridge_request_wrong_transport"));
        assert!(blockers.contains(&"host_tool_capability_missing"));
        assert_eq!(
            payload["host_bridge"]["host_tool_calls"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_untrusted_provenance() {
        let request = serde_json::json!({
            "status": "pending",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "/tmp/attacker-packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("/tmp/forged-request.json"),
            &request,
            vec!["host_bridge_request_untrusted_path".to_string()],
        );

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_untrusted_path"));
        assert!(payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("calls")
            .is_empty());
    }

    #[test]
    fn host_bridge_provenance_blocks_request_outside_state_root() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-forged-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("state root");
        let request_path = root.join("forged-request.json");
        std::fs::write(&request_path, b"{}").expect("request file");
        let request = serde_json::json!({
            "request_path": request_path.display().to_string(),
            "run_id": "run-1",
            "packet_path": "/tmp/attacker-packet.json"
        });
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        let blockers = runtime.block_on(host_bridge_request_provenance_blockers_for_state_root(
            &state_root,
            &request_path,
            &request,
        ));

        assert!(blockers.contains(&"host_bridge_request_untrusted_path".to_string()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_state_root_infers_from_project_state_request_path() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-state-root-infer-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should exist");
        std::fs::write(&request_path, b"{}").expect("request should write");

        let inferred = infer_host_bridge_state_root_from_request_path(&request_path)
            .expect("state root should infer from project state request path");

        assert_eq!(
            inferred,
            std::fs::canonicalize(&state_root).expect("state root should canonicalize")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_provenance_accepts_pending_bridge_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path();
        let request_path = state_root.join("host_bridge/request.json");
        let packet_path = state_root.join("packets/run-pending.json");
        let result_path = state_root.join("host_bridge/result.json");
        let receipt_path = state_root.join("host_bridge/receipt.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should be created");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("packet parent should be created");
        std::fs::write(&request_path, b"{}").expect("request file should be written");
        std::fs::write(&packet_path, b"{}").expect("packet file should be written");

        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-pending",
            "run_id": "run-pending",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let blockers = runtime.block_on(async {
            let store = crate::StateStore::open(state_root.to_path_buf())
                .await
                .expect("state store should open");
            store
                .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                    run_id: "run-pending".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "bridge_request_pending".to_string(),
                    lane_status: "lane_running".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "implementation".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: Some(packet_path.display().to_string()),
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
                    recorded_at: "2026-06-04T00:00:00Z".to_string(),
                })
                .await
                .expect("pending host bridge receipt should record");
            let canonical_packet_path =
                std::fs::canonicalize(&packet_path).expect("packet path should canonicalize");
            let mut blockers = Vec::new();
            super::append_host_bridge_dispatch_receipt_blockers(
                &mut blockers,
                &store,
                state_root,
                &request,
                "run-pending",
                Some(canonical_packet_path.as_path()),
            )
            .await;
            store.close().await;
            blockers
        });

        assert!(!blockers.contains(&"host_bridge_dispatch_receipt_inactive".to_string()));
        assert_eq!(blockers, Vec::<String>::new());
    }

    #[test]
    fn host_bridge_completion_lane_args_routes_through_lane_complete() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload =
            host_bridge_adapter_payload(std::path::Path::new("request.json"), &request, Vec::new());
        let args = host_bridge_completion_lane_args(
            std::path::Path::new("request.json"),
            &payload,
            "agent-1",
            Some("completed"),
            Some("receipt-1"),
            Some(std::path::Path::new("state-dir")),
        )
        .expect("completion lane args should render");

        assert_eq!(
            args,
            vec![
                "complete",
                "run-1",
                "--receipt-id",
                "receipt-1",
                "--host-bridge-request",
                "request.json",
                "--host-agent-id",
                "agent-1",
                "--host-bridge-summary",
                "completed",
                "--state-dir",
                "state-dir",
                "--json"
            ]
        );
    }

    #[test]
    fn agent_dispatch_next_scheduler_keeps_explicit_binding_implicit() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, Some("explicit-bound"), None);

        assert_eq!(resolved.preview_current_task_id, Some("explicit-bound"));
        assert_eq!(resolved.scheduler_current_task_id, Some("explicit-bound"));
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_operator_requested_current_task() {
        let resolved = resolve_agent_dispatch_next_current_task_ids(
            Some("requested"),
            Some("explicit-bound"),
            Some("single-in-progress"),
        );

        assert_eq!(resolved.preview_current_task_id, Some("requested"));
        assert_eq!(resolved.scheduler_current_task_id, Some("requested"));
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_single_in_progress_fallback() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, None, Some("single-in-progress"));

        assert_eq!(resolved.preview_current_task_id, Some("single-in-progress"));
        assert_eq!(
            resolved.scheduler_current_task_id,
            Some("single-in-progress")
        );
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

    fn task_with_labels(id: &str, title: &str, labels: &[&str]) -> TaskRecord {
        task_with_labels_and_type(id, title, labels, "task")
    }

    fn task_with_labels_and_type(
        id: &str,
        title: &str,
        labels: &[&str],
        issue_type: &str,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: title.to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 2,
            issue_type: issue_type.to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn candidate(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
    ) -> TaskSchedulingCandidate {
        candidate_with_labels(
            id,
            title,
            ready_now,
            ready_parallel_safe,
            parallel_blockers,
            &[],
        )
    }

    fn candidate_with_labels(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
        labels: &[&str],
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels(id, title, labels),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers,
        }
    }

    fn candidate_with_type(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        issue_type: &str,
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels_and_type(id, title, &[], issue_type),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers: Vec::new(),
        }
    }

    #[test]
    fn single_in_progress_task_id_from_rows_selects_only_non_epic_active_task() {
        let mut active = task_with_labels_and_type("task-active", "Active task", &[], "task");
        active.status = "in_progress".to_string();
        let mut epic = task_with_labels_and_type("epic-active", "Active epic", &[], "epic");
        epic.status = "in_progress".to_string();

        assert_eq!(
            single_in_progress_task_id_from_rows(&[epic, active]),
            Some("task-active")
        );
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_for_multiple_active_tasks() {
        let mut first = task_with_labels_and_type("task-first", "First task", &[], "task");
        first.status = "in_progress".to_string();
        let mut second = task_with_labels_and_type("task-second", "Second task", &[], "task");
        second.status = "in_progress".to_string();

        assert_eq!(single_in_progress_task_id_from_rows(&[first, second]), None);
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_without_active_task() {
        assert_eq!(
            single_in_progress_task_id_from_rows(&[task_with_labels_and_type(
                "task-open",
                "Open task",
                &[],
                "task",
            )]),
            None
        );
    }

    fn activation_bundle_with_worker_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation", "verification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "write_scope": "scoped_only",
                        "model_profiles": {
                            "gpt-5.5-low": {
                                "profile_id": "gpt-5.5-low",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation", "verification"],
                                "normalized_cost_units": 1
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "junior": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational"
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_dev_team_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "analyst-seat",
                        "tier": "senior",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["specification"],
                        "model_profiles": {
                            "analyst": {
                                "profile_id": "analyst-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["business_analyst"],
                                "task_classes": ["specification"],
                                "normalized_cost_units": 1
                            }
                        }
                    },
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    },
                    {
                        "role_id": "coach-seat",
                        "tier": "middle",
                        "default_runtime_role": "coach",
                        "runtime_roles": ["coach"],
                        "task_classes": ["coach"],
                        "normalized_cost_units": 3,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["coach"],
                        "model_profiles": {
                            "coach": {
                                "profile_id": "coach-profile",
                                "model_ref": "gpt-5.5-coach",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["coach"],
                                "task_classes": ["coach"],
                                "normalized_cost_units": 3
                            }
                        }
                    },
                    {
                        "role_id": "verifier-seat",
                        "tier": "middle",
                        "default_runtime_role": "verifier",
                        "runtime_roles": ["verifier", "prover"],
                        "task_classes": ["verification"],
                        "normalized_cost_units": 4,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["verification"],
                        "model_profiles": {
                            "prover": {
                                "profile_id": "verifier-profile",
                                "model_ref": "gpt-5.3",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["verifier", "prover"],
                                "task_classes": ["verification"],
                                "normalized_cost_units": 4
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "analyst-seat": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        },
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        },
                        "coach-seat": {
                            "effective_score": 74,
                            "lifecycle_state": "active"
                        },
                        "verifier-seat": {
                            "effective_score": 76,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_role_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_model_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "",
                                "model_ref": "",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_price_data_blocked() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": false
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_price_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn assertion_message_contains_actionable_blocker(blocker_codes: &[String], task_id: &str) {
        let expected_prefix =
            format!("selected_lane_runtime_assignment_truth_missing:task={task_id}:");
        assert!(blocker_codes
            .iter()
            .any(|code| code.starts_with(&expected_prefix)));
    }

    #[test]
    fn agent_dispatch_next_preview_selects_parallel_safe_lanes_with_commands() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 2);
        assert_eq!(preview.configured_max_parallel_agents, 4);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert_eq!(preview.selected_lanes[0].task_class, "implementation");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "junior"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert!(preview.selected_lanes[0]
            .selection_truth
            .selection_source_paths["selected_rate"]
            .as_str()
            .is_some_and(
                |path| path.starts_with("carrier_runtime.roles[junior].model_profiles.")
                    && path.ends_with(".normalized_cost_units")
            ));
        assert_eq!(
            preview.selected_lanes[0].selection_truth.pricing_readiness["pricing_freshness_status"],
            "missing"
        );
        assert!(preview.selected_lanes[1]
            .dispatch_command
            .contains("--state-dir /tmp/vida-state"));
        assert_eq!(
            preview.parallelization_planner["status"],
            "proposals_available"
        );
        assert_eq!(preview.fanout_guard["status"], "pass");
        assert_eq!(preview.fanout_guard["lanes_selected"], 2);
        assert_eq!(preview.fanout_guard["ready_parallel_safe_count"], 2);
        assert_eq!(
            preview.fanout_guard["host_bridge_capacity"]["blocked_result_code"],
            "host_agent_capacity_unavailable"
        );
        assert_eq!(
            preview.parallelization_planner["materializes_packets"],
            false
        );
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.len() == 2));
        assert_eq!(
            preview.carrier_selection_api["surface"],
            "vida agent select"
        );
        assert_eq!(preview.carrier_selection_api["status"], "pass");
        assert!(preview.carrier_selection_api["first_class_carriers"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["api_id"] == "junior")));
    }

    #[test]
    fn agent_dispatch_next_preview_blocks_no_ready_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: vec![candidate(
                "task-blocked",
                "Blocked",
                false,
                false,
                vec!["graph_blocked".to_string()],
            )],
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert_eq!(preview.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(preview.blocked_candidates[0].task_id, "task-blocked");
    }

    #[test]
    fn agent_dispatch_next_preview_selects_primary_and_reports_unsafe_parallel_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate(
                    "task-b",
                    "Task B",
                    true,
                    false,
                    vec!["execution_mode_not_parallel_safe".to_string()],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(
            preview.selected_lanes[0].dispatch_command_kind,
            "startup_activation_view_only"
        );
        assert!(preview.selected_lanes[0]
            .receipt_backed_execution_command
            .contains("--execute-dispatch"));
        assert!(preview.blocker_codes.is_empty());
        assert_eq!(preview.blocked_candidates[0].task_id, "task-b");
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("remain blocked candidates and are not selected")));
    }

    #[test]
    fn agent_dispatch_next_preview_clamps_requested_lanes_to_configured_max() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
                candidate("task-d", "Task D", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            2,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 4);
        assert_eq!(preview.configured_max_parallel_agents, 2);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert!(preview.blocked_candidates.iter().any(
            |candidate| candidate.reasons == vec!["effective_max_parallel_agents_cap_reached"]
        ));
        assert_eq!(preview.fanout_guard["effective_max_parallel_agents"], 2);
        assert_eq!(preview.fanout_guard["cap_limited_rejected_count"], 2);
    }

    #[test]
    fn agent_dispatch_next_preview_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({}),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn dev_team_sequence_uses_configured_flow_ordered_step_overrides() {
        let sequence = dev_team_sequence(&serde_json::json!({
            "dev_team_readiness": {
                "default_flow_id": "debug_flow",
                "roles": [
                    {
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    },
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "task_classes": ["implementation"]
                    }
                ],
                "sequence": ["developer"],
                "flows": [
                    {
                        "flow_id": "debug_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [
                            {
                                "role_id": "analyst",
                                "runtime_role": "solution_architect",
                                "task_class": "architecture"
                            },
                            {
                                "role_id": "developer"
                            }
                        ]
                    }
                ]
            }
        }));

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
        assert_eq!(sequence[1].role_label, "developer");
        assert_eq!(sequence[1].runtime_role, "worker");
        assert_eq!(sequence[1].task_class, "implementation");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_work_item_type() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "test_author", "runtime_role": "worker", "task_classes": ["test_authoring"]},
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [
                                {"role_id": "developer"}
                            ]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [
                                {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                                {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                            ]
                        }
                    ]
                }
            }),
            "defect",
        );

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].task_class, "specification");
        assert_eq!(sequence[1].role_label, "tester");
        assert_eq!(sequence[1].task_class, "verification");
    }

    #[test]
    fn configured_dev_team_route_selects_current_task_class_slice_for_generic_task() {
        let mut task = task_with_labels(
            "implementation-task",
            "Implement design-backed configured feature",
            &[],
        );
        task.planner_metadata.owned_paths = vec!["src/lib.rs".to_string()];
        let route = configured_dev_team_first_step_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery"
                    },
                    "roles": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "test_author", "runtime_role": "worker", "task_classes": ["test_authoring"]},
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "work_item_bindings": ["task"],
                            "ordered_steps": [
                                {"role_id": "analyst"},
                                {"role_id": "test_author"},
                                {"role_id": "developer"},
                                {"role_id": "tester"}
                            ]
                        }
                    ]
                }
            }),
            &task,
        )
        .expect("configured generic implementation task should resolve a route");

        assert_eq!(route.role_label, "test_author");
        assert_eq!(route.dispatch_target, "test_author");
        assert_eq!(route.runtime_role, "worker");
        assert_eq!(route.task_class, "test_authoring");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_canonical_work_item_alias() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair",
                        "bug": "bug_triage"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn development_flow_fallback_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn task_sequence_skips_default_on_inferred_key_miss_before_canonical_work_item() {
        let task = task_with_labels_and_type(
            "defect-review",
            "Verify defect remediation",
            &["verification"],
            "defect",
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].runtime_role, "verifier");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_task_class_for_generic_task_kind() {
        let task = task_with_labels(
            "architecture-task",
            "Architecture migration task",
            &["architecture"],
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "architecture": "architecture_design"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "architecture_design",
                            "enabled": true,
                            "work_item_bindings": ["architecture"],
                            "ordered_steps": [{"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "architect");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
    }

    #[test]
    fn development_flow_binding_selects_sequence_from_scalar_comma_bindings() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "minimal",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "coach", "runtime_role": "coach", "task_classes": ["coach"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "minimal",
                            "enabled": true,
                            "default": true,
                            "work_item_bindings": "task",
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "reviewed",
                            "enabled": true,
                            "work_item_bindings": "epic,task",
                            "ordered_steps": [{"role_id": "coach"}]
                        }
                    ]
                }
            }),
            "epic",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "coach");
        assert_eq!(sequence[0].task_class, "coach");
    }

    #[test]
    fn development_flow_binding_blocks_mixed_ready_flow_classes_without_current_task() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 2},
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: None,
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_uses_current_task_before_mixed_ready_flow_classes() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery",
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                },
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [{"role_id": "tester"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "tester");
        assert!(!preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_orders_current_task_first_with_same_ready_flow_class() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("task-active".to_string()),
                ready: vec![
                    candidate_with_type("task-other", "Other task", true, true, "task"),
                    candidate_with_type("task-active", "Active task", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "task-active");
        assert_eq!(preview.selected_lanes[0].role_label, "developer");
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_honors_current_task_for_same_flow_ready_candidates() {
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &TaskSchedulingProjection {
                current_task_id: Some("zzz-bound".to_string()),
                ready: vec![
                    candidate_with_type("aaa-other", "Other specification", true, true, "task"),
                    candidate_with_type("zzz-bound", "Bound specification", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "zzz-bound");
        assert!(preview.selected_lanes[0]
            .dispatch_command
            .contains("vida agent-init --role business_analyst zzz-bound --json"));
    }

    #[test]
    fn development_flow_binding_reuses_current_task_for_ordered_role_steps() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "defect_repair",
            "work_item_flow_bindings": {
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "flows": [
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![candidate_with_type(
                    "defect-a", "Defect A", true, false, "defect",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 2);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "analyst");
        assert_eq!(preview.selected_lanes[0].task_class, "specification");
        assert_eq!(preview.selected_lanes[1].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[1].role_label, "tester");
        assert_eq!(preview.selected_lanes[1].task_class, "verification");
        assert!(preview.blocker_codes.is_empty());
    }

    #[test]
    fn development_flow_binding_scopes_all_ordered_steps_to_current_task() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [
                        {"role_id": "developer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("task-active".to_string()),
                ready: vec![
                    candidate_with_type("task-active", "Active task", true, false, "task"),
                    candidate_with_type("task-other", "Other task", true, false, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 2);
        assert!(preview
            .selected_lanes
            .iter()
            .all(|lane| lane.task_id == "task-active"));
        assert!(!preview
            .selected_lanes
            .iter()
            .any(|lane| lane.task_id == "task-other"));
    }

    #[test]
    fn development_flow_binding_skips_unsafe_parallel_ready_candidates_without_current_task() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [
                        {"role_id": "developer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: None,
                ready: vec![
                    candidate_with_type("task-safe", "Safe task", true, true, "task"),
                    candidate_with_type("task-unsafe", "Unsafe task", true, false, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "task-safe");
        assert!(!preview
            .selected_lanes
            .iter()
            .any(|lane| lane.task_id == "task-unsafe"));
        assert!(preview
            .blocked_candidates
            .iter()
            .any(|candidate| candidate.task_id == "task-unsafe"
                && candidate
                    .reasons
                    .contains(&"parallel_safety_not_established".to_string())));
    }

    #[test]
    fn flow_projection_projects_user_approval_step_gate_and_rework_policy() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 1},
                "carrier_runtime": {
                    "roles": [{
                        "role_id": "middle",
                        "tier": "middle",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "rate": 4,
                        "model": "gpt-5.5",
                        "model_provider": "openai",
                        "model_reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "readiness": {"status": "ready"},
                        "lifecycle": {"state": "ready"}
                    }]
                },
                "dev_team_readiness": {
                    "default_flow_id": "approval_flow",
                    "roles": [{
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    }],
                    "flows": [{
                        "flow_id": "approval_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [{
                            "role_id": "analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "specification",
                            "requires_user_approval": true,
                            "approval_policy": {
                                "mode": "user_review_required",
                                "prompt_template": "review_document_before_next_role"
                            },
                            "lifecycle_hook_templates": ["approval_wait", "approval_complete"],
                            "resume_transitions": {"approved": "developer"},
                            "rework_transitions": {"rework": "analyst"}
                        }]
                    }]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: Some("task-approval".to_string()),
                ready: vec![candidate_with_type(
                    "task-approval",
                    "Approval task",
                    true,
                    true,
                    "task",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.selected_lanes.len(), 1);
        let lane = &preview.selected_lanes[0];
        assert!(lane.requires_user_approval);
        assert_eq!(
            lane.approval_gate["status"],
            "approval_required_after_step_completion"
        );
        assert_eq!(
            lane.approval_gate["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            lane.approval_gate["rework_transitions"]["rework"],
            "analyst"
        );
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("will pause after receipt-backed completion")));
        assert_eq!(preview.flow_projection["flow_id"], "approval_flow");
        assert_eq!(
            preview.flow_projection["current_step"]["role_label"],
            "analyst"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["receipt_status"]["status"],
            "preview_only"
        );
        assert_eq!(
            preview.flow_projection["approval_waits"][0]["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            preview.flow_projection["lifecycle_hook_event_stream"][0]["template_id"],
            "approval_wait"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_source"],
            "dev_team.flows.adapter_projection"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_is_data_only"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_preview_renders_configured_dev_team_sequence() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate_with_labels(
                    "task-analyst",
                    "Specification task",
                    true,
                    true,
                    Vec::new(),
                    &["documentation"],
                ),
                candidate_with_labels(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                    &[],
                ),
                candidate_with_labels(
                    "task-coach",
                    "Coach review task",
                    true,
                    true,
                    Vec::new(),
                    &["coach"],
                ),
                candidate_with_labels(
                    "task-tester",
                    "Tester verification",
                    true,
                    true,
                    Vec::new(),
                    &["tester"],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            4,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert_eq!(preview.selected_lanes[0].role_label, "analyst-seat");
        assert_eq!(preview.selected_lanes[1].role_label, "developer-seat");
        assert_eq!(preview.selected_lanes[2].role_label, "coach-seat");
        assert_eq!(preview.selected_lanes[3].role_label, "verifier-seat");
        assert_eq!(preview.selected_lanes[0].task_id, "task-analyst");
        assert_eq!(preview.selected_lanes[1].task_id, "task-developer");
        assert_eq!(preview.selected_lanes[2].task_id, "task-coach");
        assert_eq!(preview.selected_lanes[3].task_id, "task-tester");
        assert_eq!(preview.selected_lanes[0].runtime_role, "business_analyst");
        assert_eq!(preview.selected_lanes[1].runtime_role, "worker");
        assert_eq!(preview.selected_lanes[2].runtime_role, "coach");
        assert_eq!(preview.selected_lanes[3].runtime_role, "verifier");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.task_class,
            "specification"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.task_class,
            "implementation"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.task_class,
            "coach"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.task_class,
            "verification"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "analyst-seat"
        );
        assert_eq!(
            preview.selected_lanes[0]
                .selection_truth
                .selected_model_profile,
            "analyst-profile"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_carrier,
            "developer-seat"
        );
        assert_eq!(
            preview.selected_lanes[1]
                .selection_truth
                .selected_model_profile,
            "developer-profile"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[1].selection_truth.rate, 2);
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_carrier,
            "coach-seat"
        );
        assert_eq!(
            preview.selected_lanes[2]
                .selection_truth
                .selected_model_profile,
            "coach-profile"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_model_ref,
            "gpt-5.5-coach"
        );
        assert_eq!(preview.selected_lanes[2].selection_truth.rate, 3);
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_carrier,
            "verifier-seat"
        );
        assert_eq!(
            preview.selected_lanes[3]
                .selection_truth
                .selected_model_profile,
            "verifier-profile"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_model_ref,
            "gpt-5.3"
        );
        assert_eq!(preview.selected_lanes[3].selection_truth.rate, 4);
        assert!(
            preview.selected_lanes[0]
                .dispatch_command
                .contains("vida agent-init --role business_analyst task-analyst --json --state-dir /tmp/vida-state")
        );
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_uses_only_configured_registry_roles() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate("task-analyst", "Specification task", true, true, Vec::new()),
                candidate(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                ),
                candidate("task-coach", "Coach review task", true, true, Vec::new()),
                candidate("task-tester", "Tester verification", true, true, Vec::new()),
                candidate("task-unused", "Unused final task", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            5,
            5,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert!(!preview
            .next_actions
            .iter()
            .any(|action| action.contains("closure-oriented")));
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_role_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_role_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_carrier_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_model_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_model_profile_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_price_policy() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_price_data_blocked(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_rate_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_price_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_rate_missing")));
        assert!(preview.blocked_candidates.is_empty());
    }

    #[test]
    fn agent_dispatch_next_preview_exposes_dispatch_flow_discovery_surfaces() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert!(preview.source_surfaces.iter().any(|surface| {
            surface == "vida taskflow graph-summary --json"
                || surface == "vida taskflow scheduler dispatch --json"
        }));
        assert!(
            preview.source_surfaces.iter().any(
                |surface| surface
                    == "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            )
        );
        assert!(preview
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida agent-init --role worker <task-id> --json"));
    }

    #[test]
    fn agent_dispatch_next_preview_uses_default_ready_command_in_human_next_action() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"no_ready_task_candidates".to_string()));
        assert!(preview.next_actions.iter().any(|action| {
            action.contains("Inspect `vida task ready`") && !action.contains("ready --json")
        }));
    }

    #[test]
    fn dev_team_dispatch_preview_uses_default_ready_command_in_human_next_action() {
        let mut activation_bundle = activation_bundle_with_worker_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "implementation_flow",
            "roles": [{
                "role_id": "worker",
                "runtime_role": "worker",
                "task_classes": ["implementation"]
            }],
            "flows": [{
                "flow_id": "implementation_flow",
                "enabled": true,
                "default": true,
                "ordered_steps": [{
                    "role_id": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation"
                }]
            }]
        });
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: Vec::new(),
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview =
            build_agent_dispatch_next_preview(&activation_bundle, &projection, 1, 4, None, true);

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"no_ready_task_candidates".to_string()));
        assert!(preview.next_actions.iter().any(|action| {
            action.contains("Inspect `vida task ready`") && !action.contains("ready --json")
        }));
    }

    #[test]
    fn agent_dispatch_next_preview_terminal_gate_blocks_execution_but_preserves_diagnostic_proposals(
    ) {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, true, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            None,
            false,
        );
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.lanes_selected, 2);
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.len() == 2));

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "terminal_continue_snapshot_without_next_bounded_unit"
                    .to_string(),
                blocker_codes: vec![
                    "terminal_continue_snapshot_without_next_bounded_unit".to_string(),
                    "continuation_binding_ambiguous".to_string(),
                ],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: vec!["task-a".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert!(preview
            .blocker_codes
            .contains(&"terminal_continue_snapshot_without_next_bounded_unit".to_string()));
        assert!(preview
            .blocker_codes
            .contains(&"continuation_binding_ambiguous".to_string()));
        assert!(preview
            .next_actions
            .contains(&"bind an explicit next bounded unit".to_string()));
        assert_eq!(
            preview.parallelization_planner["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.parallelization_planner["continuation_gate_scope"],
            "task_scoped"
        );
        assert_eq!(
            preview.parallelization_planner["independent_parallel_available"],
            true
        );
        let proposals = preview.parallelization_planner["packet_proposals"]
            .as_array()
            .expect("diagnostic proposals should remain visible");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0]["task_id"], "task-b");
        assert_eq!(proposals[0]["materializes_packet"], false);
    }

    #[test]
    fn continuation_gate_preserves_disjoint_parallel_packet_proposals() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-current".to_string()),
            ready: vec![
                candidate("task-current", "Current task", true, true, Vec::new()),
                candidate("task-parallel-a", "Parallel A", true, true, Vec::new()),
                candidate("task-parallel-b", "Parallel B", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            3,
            4,
            None,
            false,
        );
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.lanes_selected, 3);

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "continuation_binding_ambiguous".to_string(),
                blocker_codes: vec!["continuation_binding_ambiguous".to_string()],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: vec!["task-current".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert_eq!(
            preview.flow_projection["current_step"]["dispatch_command"],
            serde_json::Value::Null
        );
        assert_eq!(
            preview.parallelization_planner["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.parallelization_planner["materializes_packets"],
            false
        );
        assert_eq!(preview.parallelization_planner["diagnostic_only"], true);
        let proposals = preview.parallelization_planner["packet_proposals"]
            .as_array()
            .expect("packet proposals should remain diagnostic");
        let proposal_task_ids = proposals
            .iter()
            .map(|proposal| proposal["task_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(
            proposal_task_ids,
            vec!["task-parallel-a", "task-parallel-b"]
        );
    }

    #[test]
    fn continuation_gate_blocks_flow_projection_dispatch_state() {
        let mut activation_bundle = activation_bundle_with_worker_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "implementation_flow",
            "roles": [{
                "role_id": "worker",
                "runtime_role": "worker",
                "task_classes": ["implementation"]
            }],
            "flows": [{
                "flow_id": "implementation_flow",
                "enabled": true,
                "default": true,
                "ordered_steps": [{
                    "role_id": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation"
                }]
            }]
        });
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview =
            build_agent_dispatch_next_preview(&activation_bundle, &projection, 1, 4, None, true);
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.flow_projection["status"], "ready");
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "pending_dispatch"
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_string());

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "continuation_binding_ambiguous".to_string(),
                blocker_codes: vec!["continuation_binding_ambiguous".to_string()],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: Vec::new(),
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert_eq!(
            preview.flow_projection["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.flow_projection["blocker_codes"],
            serde_json::json!([
                "latest_run_graph_status_blocked",
                "continuation_binding_ambiguous"
            ])
        );
        assert_eq!(
            preview.flow_projection["next_actions"],
            serde_json::json!(["bind an explicit next bounded unit"])
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_null());
        assert!(preview.flow_projection["current_step"]["dispatch_command_kind"].is_null());
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["blocked_by_continuation_gate"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_command_uses_configured_runtime_selection_truth() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: "task-ready",
                    title: "Ready task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 2,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: ".",
                })
                .await
                .expect("task should create");
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let code = runtime.block_on(crate::run(cli(&[
            "agent",
            "dispatch-next",
            "--lanes",
            "1",
            "--state-dir",
            harness.path().to_str().expect("state dir should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
    }
}
