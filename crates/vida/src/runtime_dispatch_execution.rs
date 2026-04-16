use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::runtime_lane_summary::summarize_execution_truth_for_route;
use crate::{yaml_lookup, RuntimeConsumptionLaneSelection, StateStore};

/// Check whether a backend is admissible for a given dispatch target (lane).
/// Returns `true` if the backend's lane_admissibility entry in the execution plan
/// allows the lane, or if no admissibility matrix is present (fail-open for backward
/// compatibility during the transition period).
fn backend_is_admissible_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
) -> bool {
    let Some(matrix) = execution_plan["backend_admissibility_matrix"].as_array() else {
        // No admissibility matrix present; fail open to avoid breaking existing flows.
        return true;
    };
    let Some(row) = matrix
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
    else {
        // Backend not in matrix; fail open.
        return true;
    };
    let Some(lane_admissibility) = row["lane_admissibility"].as_object() else {
        return true;
    };
    lane_admissibility
        .get(dispatch_target)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
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

const DEFAULT_INTERNAL_HOST_DISPATCH_TIMEOUT_SECONDS: u64 = 240;
const DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS: u64 = 1;
const DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS: u64 = 5;

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
            false,
        ),
    )
    .await;
    drop(store);

    match rendered {
        Ok(Ok(view)) => view,
        _ => default_activation_view(receipt, role_selection),
    }
}

fn configured_external_dispatch_wall_timeout_seconds(
    backend_entry: &serde_yaml::Value,
) -> Option<u64> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    yaml_lookup(dispatch, &["no_output_timeout_seconds"])
        .and_then(serde_yaml::Value::as_u64)
        .or_else(|| {
            yaml_lookup(backend_entry, &["max_runtime_seconds"]).and_then(serde_yaml::Value::as_u64)
        })
        .filter(|seconds| *seconds > 0)
}

fn configured_internal_host_dispatch_wall_timeout_seconds(
    system_entry: Option<&serde_yaml::Value>,
) -> u64 {
    system_entry
        .and_then(|entry| {
            yaml_lookup(entry, &["dispatch", "no_output_timeout_seconds"])
                .and_then(serde_yaml::Value::as_u64)
                .or_else(|| {
                    yaml_lookup(entry, &["max_runtime_seconds"]).and_then(serde_yaml::Value::as_u64)
                })
        })
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_INTERNAL_HOST_DISPATCH_TIMEOUT_SECONDS)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandTimeoutWrapper {
    timeout_seconds: u64,
    kill_after_grace_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WrappedCommand {
    command: String,
    args: Vec<String>,
    timeout_wrapper: Option<CommandTimeoutWrapper>,
}

impl WrappedCommand {
    fn timed_out(&self, exit_code: Option<i32>) -> bool {
        self.timeout_wrapper
            .as_ref()
            .is_some_and(|_| matches!(exit_code, Some(124) | Some(137)))
    }
}

fn wrap_command_with_optional_timeout(
    command: String,
    args: Vec<String>,
    timeout_seconds: Option<u64>,
) -> WrappedCommand {
    if let Some(timeout_seconds) = timeout_seconds.filter(|seconds| *seconds > 0) {
        let kill_after_grace_seconds =
            DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS.min(timeout_seconds.max(1));
        let mut wrapped_args = vec![
            format!("--kill-after={kill_after_grace_seconds}s"),
            format!("{timeout_seconds}s"),
            command.clone(),
        ];
        wrapped_args.extend(args);
        WrappedCommand {
            command: "timeout".to_string(),
            args: wrapped_args,
            timeout_wrapper: Some(CommandTimeoutWrapper {
                timeout_seconds,
                kill_after_grace_seconds,
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

    if output
        .error_message
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return true;
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

fn external_provider_output_confirms_execution(
    output: Option<&ParsedExternalProviderOutput>,
) -> bool {
    output.is_some_and(|parsed| !external_provider_output_indicates_error(parsed))
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

fn parse_external_provider_output(stdout: &str) -> Option<ParsedExternalProviderOutput> {
    let raw_json: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
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
        if row.get("type").and_then(serde_json::Value::as_str) == Some("item.completed") {
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
                        if let Some(message) = item
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                        {
                            error_messages.push(message.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        rows.push(row);
    }

    ParsedInternalCodexOutput {
        raw_json: serde_json::Value::Array(rows),
        result_text,
        error_messages,
    }
}

fn dispatch_packet_prompt(dispatch_packet_path: &str) -> String {
    std::fs::read_to_string(dispatch_packet_path)
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
        .unwrap_or_else(|| {
            format!(
                "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
                dispatch_packet_path
            )
        })
}

fn selected_internal_host_carrier(
    selected_cli_entry: Option<&serde_yaml::Value>,
    preferred_backend: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> Option<serde_json::Value> {
    let _ = role_selection;
    let preferred_ids = [preferred_backend, receipt.selected_backend.as_deref()];
    let carriers =
        crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(selected_cli_entry);
    preferred_ids.iter().flatten().find_map(|backend_id| {
        carriers
            .iter()
            .find(|row| row["role_id"].as_str() == Some(*backend_id))
            .cloned()
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

fn configured_internal_host_activation_parts(
    system_entry: Option<&serde_yaml::Value>,
    project_root: &Path,
    dispatch_packet_path: &str,
    carrier: &serde_json::Value,
) -> Result<(String, Vec<String>), String> {
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
    let reasoning_effort = carrier["model_reasoning_effort"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("medium");
    let prompt = dispatch_packet_prompt(dispatch_packet_path);
    let mut args = crate::yaml_string_list(yaml_lookup(dispatch, &["static_args"]));
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
        args.push(model.to_string());
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
        other => {
            return Err(format!(
                "Configured internal host system uses unsupported prompt_mode `{other}`"
            ));
        }
    }
    Ok((command, args))
}

pub(crate) fn agent_lane_dispatch_result(
    mut activation_view: serde_json::Value,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> serde_json::Value {
    let effective_selected_backend =
        crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
            role_selection,
            receipt,
        )
        .or_else(|| receipt.selected_backend.clone());
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let lane_dispatch = crate::runtime_dispatch_state::runtime_agent_lane_dispatch_for_root(
        &project_root,
        dispatch_packet_path,
        preferred_backend,
    );
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
            effective_selected_backend.as_deref(),
            receipt.activation_agent_type.as_deref(),
            Some(&host_runtime),
            false,
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
    body.insert(
        "blocker_code".to_string(),
        serde_json::json!("internal_activation_view_only"),
    );
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
    body.insert(
        "execution_evidence".to_string(),
        serde_json::json!({
            "status": "recorded",
            "evidence_kind": evidence_kind,
            "backend_id": backend_id,
            "receipt_backed": true,
            "records_dispatch_result": true,
        }),
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
    ) else {
        return Ok(None);
    };

    let carrier_id = carrier["role_id"]
        .as_str()
        .unwrap_or(selected_cli_system.as_str());
    let (command, args) = configured_internal_host_activation_parts(
        selected_cli_entry.as_ref(),
        project_root,
        dispatch_packet_path,
        &carrier,
    )?;
    let wall_timeout_seconds = Some(configured_internal_host_dispatch_wall_timeout_seconds(
        selected_cli_entry.as_ref(),
    ));
    let wrapped_command =
        wrap_command_with_optional_timeout(command.clone(), args.clone(), wall_timeout_seconds);
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &wrapped_command.command,
        &wrapped_command.args,
    );
    let runtime_env =
        configured_internal_host_runtime_env(project_root, &selected_cli_system, carrier_id)?;

    let mut process = std::process::Command::new(&wrapped_command.command);
    process
        .args(&wrapped_command.args)
        .current_dir(project_root)
        .stdin(Stdio::null());
    for (key, value) in runtime_env {
        process.env(key, value);
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    process.env("VIDA_SELECTED_BACKEND", carrier_id);
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }

    let output = process.output().map_err(|error| {
        format!(
            "Failed to execute internal host carrier `{carrier_id}` for `{selected_cli_system}` via `{}`: {error}",
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
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_output = parse_internal_codex_exec_output(&stdout);
    let exit_code = output.status.code();
    let timed_out = wrapped_command.timed_out(exit_code);
    let activation_only = timed_out
        || (output.status.success()
            && parsed_output.result_text.is_none()
            && parsed_output.error_messages.is_empty());
    let success = output.status.success() && parsed_output.result_text.is_some();

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
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("internal_activation_view_only"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "missing");
    } else {
        let blocker_reason = if !stderr.is_empty() {
            stderr
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
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
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
        crate::runtime_dispatch_state::configured_external_backend_entry(&overlay, backend_id)
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
                        false,
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

    let (command, args) = crate::runtime_dispatch_state::configured_external_activation_parts(
        &backend_entry,
        project_root,
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

    let output = process.output().map_err(|error| {
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
    let success =
        output.status.success() && external_provider_output_confirms_execution(parsed_output.as_ref());
    let exit_code = output.status.code();
    let timed_out = wrapped_command.timed_out(exit_code);
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
    } else if timed_out {
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
                "configured external backend timed out after {timeout_seconds}s and kill-after grace {kill_after_grace_seconds}s without receipt-backed completion"
            )),
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
    use super::{
        agent_lane_dispatch_result, configured_internal_host_activation_parts,
        configured_internal_host_runtime_env, dispatch_packet_prompt,
        external_provider_output_confirms_execution, mark_dispatch_result_execution_evidence,
        parse_external_provider_output,
        parse_internal_codex_exec_output, wrap_command_with_optional_timeout,
    };
    use crate::RuntimeConsumptionLaneSelection;
    use std::path::Path;

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

        assert!(xdg_config_home.contains("/.vida/data/internal-host/qwen/worker-a/config"));
        let _ = std::fs::remove_dir_all(&harness);
    }

    #[test]
    fn configured_internal_host_activation_parts_use_system_dispatch_config() {
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
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.4",
            "model_reasoning_effort": "high",
            "sandbox_mode": "workspace-write"
        });

        let (command, args) = configured_internal_host_activation_parts(
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
                "gpt-5.4".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"high\"".to_string(),
                dispatch_packet_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
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
            "opencode_cli"
        );
        assert_eq!(
            result["execution_truth"]["selected_backend_source"],
            "route_primary"
        );
        assert_eq!(
            result["execution_truth"]["activation_evidence"]["execution_evidence_status"],
            "missing"
        );
    }

    #[test]
    fn wrap_command_with_optional_timeout_adds_kill_after_grace() {
        let wrapped = wrap_command_with_optional_timeout(
            "codex".to_string(),
            vec!["exec".to_string()],
            Some(5),
        );

        assert_eq!(wrapped.command, "timeout");
        assert_eq!(
            wrapped.args,
            vec![
                "--kill-after=1s".to_string(),
                "5s".to_string(),
                "codex".to_string(),
                "exec".to_string()
            ]
        );
        assert!(wrapped.timed_out(Some(124)));
        assert!(wrapped.timed_out(Some(137)));
        assert!(!wrapped.timed_out(Some(1)));
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_denies_read_only_backend_for_implementer() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "qwen_cli",
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
                "qwen_cli",
                "implementation"
            ),
            "qwen_cli should be inadmissible for implementation lane"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "qwen_cli",
                "analysis"
            ),
            "qwen_cli should be admissible for analysis lane"
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
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "qwen_cli",
                "implementation"
            ),
            "should fail open when no admissibility matrix is present"
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
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "qwen_cli",
                "implementation"
            ),
            "should fail open when backend is not in the matrix"
        );
    }
}
