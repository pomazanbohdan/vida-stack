use std::path::{Path, PathBuf};

use crate::{yaml_lookup, RuntimeConsumptionLaneSelection, StateStore};

fn configured_external_dispatch_wall_timeout_seconds(
    backend_entry: &serde_yaml::Value,
) -> Option<u64> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    yaml_lookup(dispatch, &["no_output_timeout_seconds"])
        .and_then(serde_yaml::Value::as_u64)
        .or_else(|| yaml_lookup(backend_entry, &["max_runtime_seconds"]).and_then(serde_yaml::Value::as_u64))
        .filter(|seconds| *seconds > 0)
}

#[derive(Debug)]
struct ParsedExternalProviderOutput {
    raw_json: serde_json::Value,
    result_text: Option<String>,
    usage: Option<serde_json::Value>,
    is_error: Option<bool>,
    error_message: Option<String>,
}

fn parse_external_provider_output(stdout: &str) -> Option<ParsedExternalProviderOutput> {
    let raw_json: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let result_row = match &raw_json {
        serde_json::Value::Array(rows) => rows.iter().rev().find(|row| {
            row.get("type").and_then(serde_json::Value::as_str) == Some("result")
        }),
        serde_json::Value::Object(_) => Some(&raw_json),
        _ => None,
    }?;
    Some(ParsedExternalProviderOutput {
        result_text: result_row
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        usage: result_row.get("usage").cloned(),
        is_error: result_row.get("is_error").and_then(serde_json::Value::as_bool),
        error_message: result_row
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        raw_json,
    })
}

pub(crate) fn agent_lane_dispatch_result(
    mut activation_view: serde_json::Value,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    host_runtime: serde_json::Value,
) -> serde_json::Value {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let lane_dispatch = crate::runtime_dispatch_state::runtime_agent_lane_dispatch_for_root(
        &project_root,
        dispatch_packet_path,
        preferred_backend,
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

pub(crate) async fn execute_external_agent_lane_dispatch(
    store: &StateStore,
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
    let (command, args) = crate::runtime_dispatch_state::configured_external_activation_parts(
        &backend_entry,
        project_root,
        dispatch_packet_path,
    )?;
    let wall_timeout_seconds = configured_external_dispatch_wall_timeout_seconds(&backend_entry);
    let (effective_command, effective_args) = if let Some(timeout_seconds) = wall_timeout_seconds {
        let mut wrapped_args = vec![format!("{timeout_seconds}s"), command.clone()];
        wrapped_args.extend(args.clone());
        ("timeout".to_string(), wrapped_args)
    } else {
        (command.clone(), args.clone())
    };
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &effective_command,
        &effective_args,
    );

    let mut process = std::process::Command::new(&effective_command);
    process.args(&effective_args).current_dir(project_root);
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
    if let Some(selected_backend) = receipt.selected_backend.as_deref() {
        process.env("VIDA_SELECTED_BACKEND", selected_backend);
    }

    let output = process.output().map_err(|error| {
        format!(
            "Failed to execute configured external backend `{backend_id}` via `{effective_command}`: {error}"
        )
    })?;
    let activation_view = crate::init_surfaces::render_agent_init_packet_activation_with_store(
        store,
        project_root,
        dispatch_packet_path,
        false,
    )
    .await
    .unwrap_or_else(|_| {
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
    });
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
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
    let provider_reported_error = parsed_output
        .as_ref()
        .and_then(|parsed| parsed.is_error)
        .unwrap_or(false);
    let success = output.status.success() && !provider_reported_error;
    let exit_code = output.status.code();
    let timed_out = wall_timeout_seconds.is_some() && exit_code == Some(124);
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
    if let Some(parsed_output) = parsed_output {
        body.insert("provider_output_json".to_string(), parsed_output.raw_json);
        body.insert(
            "provider_result".to_string(),
            parsed_output
                .result_text
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
        body.insert(
            "provider_usage".to_string(),
            parsed_output.usage.unwrap_or(serde_json::Value::Null),
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
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    if success {
        body.insert("blocker_code".to_string(), serde_json::Value::Null);
        body.insert("blocker_reason".to_string(), serde_json::Value::Null);
    } else if timed_out {
        let timeout_seconds = wall_timeout_seconds.unwrap_or_default();
        body.insert(
            "provider_error".to_string(),
            serde_json::json!(format!(
                "configured external backend timed out after {timeout_seconds}s without receipt-backed completion"
            )),
        );
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::TimeoutWithoutTakeoverAuthority
                )
            ),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(
                "configured external backend exceeded the bounded runtime window before returning execution evidence"
            ),
        );
    } else {
        let provider_error_message = body
            .get("provider_error_message")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
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
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::parse_external_provider_output;

    #[test]
    fn parse_external_provider_output_extracts_qwen_json_success_result() {
        let parsed = parse_external_provider_output(
            r#"[{"type":"system"},{"type":"result","subtype":"success","is_error":false,"result":"OK","usage":{"total_tokens":42}}]"#,
        )
        .expect("qwen json output should parse");

        assert_eq!(parsed.result_text.as_deref(), Some("OK"));
        assert_eq!(parsed.is_error, Some(false));
        assert_eq!(parsed.usage.expect("usage should exist")["total_tokens"], 42);
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
}
