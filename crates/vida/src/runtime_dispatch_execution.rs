use std::path::{Path, PathBuf};

use crate::{yaml_lookup, RuntimeConsumptionLaneSelection, StateStore};

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
    let activation_command = crate::runtime_dispatch_state::render_command_display(&command, &args);

    let mut process = std::process::Command::new(&command);
    process.args(&args).current_dir(project_root);
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
            "Failed to execute configured external backend `{backend_id}` via `{command}`: {error}"
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let success = output.status.success();
    let exit_code = output.status.code();
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
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch.insert(
            "backend_class".to_string(),
            serde_json::json!(backend_class),
        );
    }
    if !success {
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
    }
    Ok(result)
}
