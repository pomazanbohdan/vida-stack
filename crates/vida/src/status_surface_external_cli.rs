fn expand_user_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(remainder) = trimmed.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{remainder}");
        }
    }
    trimmed.to_string()
}

fn file_exists(path: &str) -> bool {
    std::fs::metadata(expand_user_path(path))
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn path_candidate_exists(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn command_path_candidates(base: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut candidates = vec![base.to_path_buf()];
    if cfg!(windows) && base.extension().is_none() {
        let pathext =
            std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        for extension in pathext.split(';').map(str::trim) {
            if extension.is_empty() {
                continue;
            }
            candidates.push(base.with_extension(extension.trim_start_matches('.')));
        }
    }
    candidates
}

fn command_contains_path_separator(command: &str) -> bool {
    command.contains('/') || command.contains('\\')
}

fn command_is_resolvable(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    #[cfg(test)]
    if command == "sh" {
        return true;
    }
    let expanded = expand_user_path(command);
    let command_path = std::path::Path::new(&expanded);
    if command_path.is_absolute() || command_contains_path_separator(command) {
        return command_path_candidates(command_path)
            .iter()
            .any(|candidate| path_candidate_exists(candidate));
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| {
        command_path_candidates(&dir.join(command))
            .iter()
            .any(|candidate| path_candidate_exists(candidate))
    })
}

fn external_cli_command_probe<'a>(
    backend_entry: &'a serde_yaml::Value,
) -> Option<(&'static str, &'a str)> {
    crate::yaml_lookup(backend_entry, &["dispatch", "command"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|command| ("dispatch.command", command))
        .or_else(|| {
            crate::yaml_lookup(backend_entry, &["detect_command"])
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|command| ("detect_command", command))
        })
}

fn readiness_command<'a>(
    readiness: &'a serde_yaml::Value,
    section: &str,
) -> Option<(&'static str, &'a str)> {
    crate::yaml_lookup(readiness, &[section, "command"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|command| match section {
            "adapter" => ("readiness.adapter.command", command),
            "provider" => ("readiness.provider.command", command),
            _ => ("readiness.command", command),
        })
}

fn external_cli_probe_command_is_allowlisted(command: &str) -> bool {
    let trimmed = command.trim();
    #[cfg(test)]
    if std::path::Path::new(trimmed).is_file() {
        return true;
    }
    if trimmed.is_empty() || command_contains_path_separator(trimmed) {
        return false;
    }
    let normalized = trimmed.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "codex"
            | "qwen"
            | "claude"
            | "gemini"
            | "aider"
            | "cursor-agent"
            | "opencode"
            | "hermes"
            | "kilo"
            | "vibe"
            | "vida-pi-agent"
            | "pi"
    ) || (cfg!(test) && normalized == "sh")
}

fn profile_write_scope_requires_guard(profile: &serde_json::Value) -> bool {
    matches!(
        profile["write_scope"]
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

fn command_output_with_timeout(
    command: &str,
    args: &[&str],
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    #[cfg(test)]
    if !std::path::Path::new(command).is_absolute() && !command_contains_path_separator(command) {
        return Err(format!(
            "Unit tests do not execute live external readiness probe `{command}`; use an explicit fixture command path."
        ));
    }
    let mut child = std::process::Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to spawn `{command}`: {error}"))?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect `{command}` output: {error}"));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Timed out running `{command}` readiness probe"));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(25)),
            Err(error) => {
                return Err(format!(
                    "Failed to inspect `{command}` readiness probe: {error}"
                ))
            }
        }
    }
}

fn adapter_prewrite_guard_capabilities(adapter_command: &str) -> serde_json::Value {
    #[cfg(test)]
    if adapter_command == "sh" {
        return serde_json::json!({
            "status": "available",
            "pre_write_enforcement": true,
            "explicit_extension_arg": true,
            "source": "test-fixture"
        });
    }

    let output = match command_output_with_timeout(
        adapter_command,
        &["--capabilities-json"],
        std::time::Duration::from_secs(3),
    ) {
        Ok(output) => output,
        Err(error) => {
            return serde_json::json!({
                "status": "probe_failed",
                "pre_write_enforcement": false,
                "error": error,
            })
        }
    };
    if !output.status.success() {
        return serde_json::json!({
            "status": "probe_failed",
            "pre_write_enforcement": false,
            "exit_status": output.status.to_string(),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim()) else {
        return serde_json::json!({
            "status": "invalid_capabilities_json",
            "pre_write_enforcement": false,
        });
    };
    let scope_guard = &value["scope_guard"];
    let pre_write_enforcement = scope_guard["pre_write_enforcement"]
        .as_bool()
        .unwrap_or(false)
        && scope_guard["explicit_extension_arg"]
            .as_bool()
            .unwrap_or(false);
    serde_json::json!({
        "status": if pre_write_enforcement { "available" } else { "missing" },
        "pre_write_enforcement": pre_write_enforcement,
        "explicit_extension_arg": scope_guard["explicit_extension_arg"].as_bool().unwrap_or(false),
        "raw": value,
    })
}

fn pi_model_catalog_contains_model(
    provider_command: &str,
    expected_model_ref: &str,
) -> Result<bool, String> {
    let output = command_output_with_timeout(
        provider_command,
        &["--list-models"],
        std::time::Duration::from_secs(3),
    )?;
    if !output.status.success() {
        return Err(format!(
            "Pi model catalog probe failed with status {}",
            output.status
        ));
    }
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    let expected = expected_model_ref.trim();
    if expected.is_empty() {
        return Ok(true);
    }
    if combined.contains(expected) {
        return Ok(true);
    }
    if let Some((provider, model)) = expected.split_once('/') {
        return Ok(combined.contains(provider.trim()) && combined.contains(model.trim()));
    }
    Ok(combined.contains(expected))
}

fn read_text_file(path: &str) -> Option<String> {
    std::fs::read_to_string(expand_user_path(path)).ok()
}

fn read_json_file(path: &str) -> Option<serde_json::Value> {
    read_text_file(path).and_then(|text| serde_json::from_str(&text).ok())
}

fn file_contains(path: &str, needle: &str) -> bool {
    if needle.trim().is_empty() {
        return false;
    }
    read_text_file(path).is_some_and(|text| text.contains(needle))
}

fn latest_file_in_dir(path: &str) -> Option<std::path::PathBuf> {
    let dir = expand_user_path(path);
    let mut latest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let metadata = entry.metadata().ok()?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().ok()?;
        match latest.as_ref() {
            Some((current_modified, _)) if modified <= *current_modified => {}
            _ => latest = Some((modified, entry.path())),
        }
    }
    latest.map(|(_, path)| path)
}

fn latest_dir_file_contains(path: &str, needle: &str, max_age_seconds: Option<u64>) -> bool {
    if needle.trim().is_empty() {
        return false;
    }
    let Some(latest_file) = latest_file_in_dir(path) else {
        return false;
    };
    if let Some(max_age_seconds) = max_age_seconds {
        let Ok(metadata) = std::fs::metadata(&latest_file) else {
            return false;
        };
        let Ok(modified) = metadata.modified() else {
            return false;
        };
        let Ok(age) = std::time::SystemTime::now().duration_since(modified) else {
            return false;
        };
        if age.as_secs() > max_age_seconds {
            return false;
        }
    }
    std::fs::read_to_string(latest_file)
        .map(|text| text.contains(needle))
        .unwrap_or(false)
}

fn recent_dir_contains_any(path: &str, needle: &str, max_age_seconds: Option<u64>) -> bool {
    if needle.trim().is_empty() {
        return false;
    }
    let dir = expand_user_path(path);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((entry.path(), metadata))
        })
        .any(|(path, metadata)| {
            if let Some(max_age_seconds) = max_age_seconds {
                let Ok(modified) = metadata.modified() else {
                    return false;
                };
                let Ok(age) = std::time::SystemTime::now().duration_since(modified) else {
                    return false;
                };
                if age.as_secs() > max_age_seconds {
                    return false;
                }
            }
            std::fs::read_to_string(path)
                .map(|text| text.contains(needle))
                .unwrap_or(false)
        })
}

fn model_ref_from_json_state(mode: &str, path: &str) -> Option<String> {
    let value = read_json_file(path)?;
    match mode {
        "json_recent_ref" => {
            let first = value.get("recent")?.as_array()?.first()?;
            let provider = first.get("providerID")?.as_str()?.trim();
            let model = first.get("modelID")?.as_str()?.trim();
            if provider.is_empty() || model.is_empty() {
                None
            } else {
                Some(format!("{provider}/{model}"))
            }
        }
        "json_code_ref" => {
            let code = value.get("model")?.get("code")?;
            let provider = code.get("providerID")?.as_str()?.trim();
            let model = code.get("modelID")?.as_str()?.trim();
            if provider.is_empty() || model.is_empty() {
                None
            } else {
                Some(format!("{provider}/{model}"))
            }
        }
        _ => None,
    }
}

fn external_backend_profile_projection(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> serde_json::Value {
    let fallback_rate =
        crate::yaml_string(crate::yaml_lookup(backend_entry, &["budget_cost_units"]))
            .and_then(|raw| raw.parse::<u64>().ok())
            .or_else(|| {
                crate::yaml_string(crate::yaml_lookup(
                    backend_entry,
                    &["normalized_cost_units"],
                ))
                .and_then(|raw| raw.parse::<u64>().ok())
            })
            .or_else(|| {
                crate::yaml_string(crate::yaml_lookup(backend_entry, &["rate"]))
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

fn profile_id_matching_model_ref(
    profile_projection: &serde_json::Value,
    model_ref: Option<&str>,
) -> Option<String> {
    let model_ref = model_ref.map(str::trim).filter(|value| !value.is_empty())?;
    crate::model_profile_contract::model_profiles_from_json_row(profile_projection)
        .into_iter()
        .find(|profile| profile["model_ref"].as_str().map(str::trim) == Some(model_ref))
        .and_then(|profile| profile["profile_id"].as_str().map(str::to_string))
}

fn selected_external_cli_profile(
    profile_projection: &serde_json::Value,
    current_model_ref: Option<&str>,
    preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    if preferred_profile_id.is_some() {
        if let Some(selected_profile) =
            crate::model_profile_contract::selected_model_profile_from_json_row(
                profile_projection,
                preferred_profile_id,
            )
        {
            return selected_profile["profile_id"].clone();
        }
    }
    profile_id_matching_model_ref(profile_projection, current_model_ref)
        .map(serde_json::Value::String)
        .or_else(|| {
            profile_projection["current_model_profile"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| serde_json::Value::String(value.to_string()))
        })
        .unwrap_or_else(|| profile_projection["default_model_profile"].clone())
}

fn selected_profile_id_value(
    profile_projection: &serde_json::Value,
    selected_profile: &serde_json::Value,
) -> serde_json::Value {
    selected_profile["profile_id"]
        .as_str()
        .map(|value| serde_json::Value::String(value.to_string()))
        .unwrap_or_else(|| profile_projection["default_model_profile"].clone())
}

fn pi_style_readiness_payload(
    backend_id: &str,
    status: &str,
    blocked: bool,
    blocker_code: serde_json::Value,
    profile_projection: &serde_json::Value,
    selected_profile: &serde_json::Value,
    expected_model_ref: Option<String>,
    adapter_status: serde_json::Value,
    provider_status: serde_json::Value,
    model_catalog_status: serde_json::Value,
    write_scope_guard_status: serde_json::Value,
    next_actions: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "backend_id": backend_id,
        "status": status,
        "blocked": blocked,
        "blocker_code": blocker_code,
        "current_model_ref": serde_json::Value::Null,
        "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
        "expected_model_ref": expected_model_ref,
        "default_model_profile": profile_projection["default_model_profile"].clone(),
        "selected_model_profile": selected_profile_id_value(profile_projection, selected_profile),
        "model_profiles": profile_projection["model_profiles"].clone(),
        "adapter": adapter_status,
        "provider": provider_status,
        "model_catalog": model_catalog_status,
        "write_scope_guard": write_scope_guard_status,
        "next_actions": next_actions,
    })
}

fn command_ready_status(source: &str, command: &str) -> serde_json::Value {
    serde_json::json!({
        "source": source,
        "status": "command_found",
        "command": command,
    })
}

fn command_missing_status(source: &str, command: &str) -> serde_json::Value {
    serde_json::json!({
        "source": source,
        "status": "command_not_found",
        "command": command,
    })
}

fn pi_style_external_cli_carrier_readiness(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    readiness: &serde_yaml::Value,
    profile_projection: &serde_json::Value,
    selected_profile: &serde_json::Value,
    _preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    let expected_model_ref = selected_profile["model_ref"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            profile_projection["model"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    let selected_profile_id = selected_profile_id_value(profile_projection, selected_profile);
    let adapter_command = readiness_command(readiness, "adapter")
        .or_else(|| external_cli_command_probe(backend_entry));
    let Some((adapter_source, adapter_command)) = adapter_command else {
        return pi_style_readiness_payload(
            backend_id,
            "pi_adapter_command_not_configured",
            true,
            serde_json::Value::String(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                )
                .to_string(),
            ),
            profile_projection,
            selected_profile,
            expected_model_ref,
            serde_json::json!({"status":"command_not_configured"}),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec!["Configure readiness.adapter.command for this Pi external carrier.".to_string()],
        );
    };
    if !command_is_resolvable(adapter_command) {
        return pi_style_readiness_payload(
            backend_id,
            "pi_adapter_command_not_found",
            true,
            serde_json::Value::String(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                )
                .to_string(),
            ),
            profile_projection,
            selected_profile,
            expected_model_ref,
            command_missing_status(adapter_source, adapter_command),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec![format!(
                "Install or expose Pi adapter command `{adapter_command}` on PATH before dispatch."
            )],
        );
    }
    if !external_cli_probe_command_is_allowlisted(adapter_command) {
        return pi_style_readiness_payload(
            backend_id,
            "pi_adapter_command_not_allowlisted",
            true,
            serde_json::Value::String(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                )
                .to_string(),
            ),
            profile_projection,
            selected_profile,
            expected_model_ref,
            serde_json::json!({
                "status": "command_not_allowlisted",
                "source": adapter_source,
                "command": adapter_command,
            }),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec![format!(
                "Use an allowlisted command for Pi adapter readiness probes; `{adapter_command}` is not allowed."
            )],
        );
    }
    let adapter_status = command_ready_status(adapter_source, adapter_command);

    let provider_command = readiness_command(readiness, "provider");
    let Some((provider_source, provider_command)) = provider_command else {
        return pi_style_readiness_payload(
            backend_id,
            "pi_provider_command_not_configured",
            true,
            serde_json::Value::String(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                )
                .to_string(),
            ),
            profile_projection,
            selected_profile,
            expected_model_ref,
            adapter_status,
            serde_json::json!({"status":"command_not_configured"}),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec!["Configure readiness.provider.command for this Pi external carrier.".to_string()],
        );
    };
    if !command_is_resolvable(provider_command) {
        return pi_style_readiness_payload(
            backend_id,
            "pi_provider_command_not_found",
            true,
            serde_json::Value::String(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::ToolExecutionFailed,
            ).to_string()),
            profile_projection,
            selected_profile,
            expected_model_ref,
            adapter_status,
            command_missing_status(provider_source, provider_command),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec![format!("Install or expose Pi provider command `{provider_command}` on PATH before dispatch.")],
        );
    }
    if !external_cli_probe_command_is_allowlisted(provider_command) {
        return pi_style_readiness_payload(
            backend_id,
            "pi_provider_command_not_allowlisted",
            true,
            serde_json::Value::String(
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                )
                .to_string(),
            ),
            profile_projection,
            selected_profile,
            expected_model_ref,
            adapter_status,
            serde_json::json!({
                "status": "command_not_allowlisted",
                "source": provider_source,
                "command": provider_command,
            }),
            serde_json::json!({"status":"not_checked"}),
            serde_json::json!({"status":"not_checked"}),
            vec![format!(
                "Use an allowlisted command for Pi provider readiness probes; `{provider_command}` is not allowed."
            )],
        );
    }
    let provider_status = command_ready_status(provider_source, provider_command);

    let model_catalog_mode = crate::yaml_lookup(readiness, &["model_catalog", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or("none");
    let model_catalog_status = if matches!(
        model_catalog_mode,
        "pi_rpc_get_available_models" | "pi_list_models" | "pi_get_available_models"
    ) {
        match expected_model_ref.as_deref() {
            Some(expected) => match pi_model_catalog_contains_model(provider_command, expected) {
                Ok(true) => serde_json::json!({
                    "mode": model_catalog_mode,
                    "status": "model_available",
                    "expected_model_ref": expected,
                }),
                Ok(false) => {
                    return pi_style_readiness_payload(
                        backend_id,
                        "pi_model_unavailable",
                        true,
                        serde_json::Value::String(crate::release1_contracts::blocker_code_str(
                            crate::release1_contracts::BlockerCode::ModelNotPinned,
                        ).to_string()),
                        profile_projection,
                        selected_profile,
                        expected_model_ref.clone(),
                        adapter_status,
                        provider_status,
                        serde_json::json!({
                            "mode": model_catalog_mode,
                            "status": "model_not_found",
                            "expected_model_ref": expected,
                        }),
                        serde_json::json!({"status":"not_checked"}),
                        vec![format!("Pi model catalog does not include selected model `{expected}` for profile `{}`.", selected_profile_id.as_str().unwrap_or("unknown"))],
                    );
                }
                Err(error) => {
                    return pi_style_readiness_payload(
                        backend_id,
                        "pi_model_catalog_unavailable",
                        true,
                        serde_json::Value::String(
                            crate::release1_contracts::blocker_code_str(
                                crate::release1_contracts::BlockerCode::ToolExecutionFailed,
                            )
                            .to_string(),
                        ),
                        profile_projection,
                        selected_profile,
                        expected_model_ref.clone(),
                        adapter_status,
                        provider_status,
                        serde_json::json!({
                            "mode": model_catalog_mode,
                            "status": "probe_failed",
                            "error": error,
                        }),
                        serde_json::json!({"status":"not_checked"}),
                        vec![
                            "Repair Pi model catalog access, then rerun `vida status --json`."
                                .to_string(),
                        ],
                    );
                }
            },
            None => serde_json::json!({
                "mode": model_catalog_mode,
                "status": "not_checked_no_expected_model",
            }),
        }
    } else {
        serde_json::json!({
            "mode": model_catalog_mode,
            "status": "not_required",
        })
    };

    let guard_required_for_write = crate::yaml_bool(
        crate::yaml_lookup(
            readiness,
            &["write_scope_guard", "required_for_write_profiles"],
        ),
        false,
    );
    let fail_closed_until_available = crate::yaml_bool(
        crate::yaml_lookup(
            readiness,
            &["write_scope_guard", "fail_closed_until_available"],
        ),
        false,
    );
    let guard_required_by_profile = profile_write_scope_requires_guard(selected_profile);
    let guard_mode = crate::yaml_lookup(readiness, &["write_scope_guard", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or("none");
    let adapter_capabilities = if guard_required_for_write || guard_required_by_profile {
        adapter_prewrite_guard_capabilities(adapter_command)
    } else {
        serde_json::json!({"status":"not_required", "pre_write_enforcement": false})
    };
    let prewrite_guard_active = adapter_capabilities["pre_write_enforcement"]
        .as_bool()
        .unwrap_or(false);
    let write_scope_guard_status = serde_json::json!({
        "mode": guard_mode,
        "required_for_write_profiles": guard_required_for_write,
        "fail_closed_until_available": fail_closed_until_available,
        "selected_profile_requires_guard": guard_required_by_profile,
        "pre_write_enforcement": prewrite_guard_active,
        "adapter_capabilities": adapter_capabilities,
        "status": if guard_required_by_profile && guard_required_for_write && prewrite_guard_active {
            "active"
        } else if guard_required_by_profile && guard_required_for_write && fail_closed_until_available {
            "guard_required_not_active"
        } else {
            "not_blocking"
        },
    });
    if guard_required_by_profile
        && guard_required_for_write
        && fail_closed_until_available
        && !prewrite_guard_active
    {
        return pi_style_readiness_payload(
            backend_id,
            "pi_write_scope_guard_required",
            true,
            serde_json::Value::String("write_scope_guard_required".to_string()),
            profile_projection,
            selected_profile,
            expected_model_ref,
            adapter_status,
            provider_status,
            model_catalog_status,
            write_scope_guard_status,
            vec!["Install an updated vida-pi-agent that reports Pi pre-write guard capabilities before admitting write-capable Pi profiles.".to_string()],
        );
    }

    pi_style_readiness_payload(
        backend_id,
        "carrier_ready",
        false,
        serde_json::Value::Null,
        profile_projection,
        selected_profile,
        expected_model_ref,
        adapter_status,
        provider_status,
        model_catalog_status,
        write_scope_guard_status,
        Vec::new(),
    )
}

fn external_cli_carrier_readiness(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    let profile_projection = external_backend_profile_projection(backend_id, backend_entry);
    let readiness = crate::yaml_lookup(backend_entry, &["readiness"]);
    let preferred_profile = crate::model_profile_contract::selected_model_profile_from_json_row(
        &profile_projection,
        preferred_profile_id,
    )
    .unwrap_or(serde_json::Value::Null);
    let selected_profile = if preferred_profile.is_null() {
        crate::model_profile_contract::selected_model_profile_from_json_row(
            &profile_projection,
            profile_projection["default_model_profile"].as_str(),
        )
        .unwrap_or(serde_json::Value::Null)
    } else {
        preferred_profile.clone()
    };

    if let Some(readiness) = readiness {
        let has_pi_style_readiness = readiness_command(readiness, "adapter").is_some()
            || readiness_command(readiness, "provider").is_some()
            || crate::yaml_lookup(readiness, &["model_catalog", "mode"]).is_some()
            || crate::yaml_lookup(readiness, &["write_scope_guard", "mode"]).is_some();
        if has_pi_style_readiness {
            return pi_style_external_cli_carrier_readiness(
                backend_id,
                backend_entry,
                readiness,
                &profile_projection,
                &selected_profile,
                preferred_profile_id,
            );
        }
    }

    if let Some((command_source, command)) = external_cli_command_probe(backend_entry) {
        if !command_is_resolvable(command) {
            return serde_json::json!({
                "backend_id": backend_id,
                "status": "external_cli_command_not_found",
                "blocked": true,
                "blocker_code": crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ToolExecutionFailed
                ),
                "current_model_ref": serde_json::Value::Null,
                "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
                "expected_model_ref": profile_projection["model"].clone(),
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "selected_model_profile": profile_projection["default_model_profile"].clone(),
                "model_profiles": profile_projection["model_profiles"].clone(),
                "detect_command": command,
                "command_resolution": {
                    "source": command_source,
                    "status": "command_not_found",
                    "command": command,
                },
                "next_actions": [
                    format!("Install or expose `{command}` on PATH, or reroute this external CLI backend before dispatch."),
                    "Rerun `vida status --json` after restoring the external CLI command."
                ],
            });
        }
    }
    if readiness.is_none() {
        return serde_json::json!({
            "backend_id": backend_id,
            "status": "carrier_ready",
            "blocked": false,
            "blocker_code": serde_json::Value::Null,
            "current_model_ref": serde_json::Value::Null,
            "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
            "expected_model_ref": profile_projection["model"].clone(),
            "default_model_profile": profile_projection["default_model_profile"].clone(),
            "selected_model_profile": profile_projection["default_model_profile"].clone(),
            "model_profiles": profile_projection["model_profiles"].clone(),
            "next_actions": [],
        });
    }
    let readiness = readiness.expect("checked is_some");

    let auth_mode = crate::yaml_lookup(readiness, &["auth", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or("none");
    let auth_ok = match auth_mode {
        "none" => true,
        "file_present" => crate::yaml_lookup(readiness, &["auth", "path"])
            .and_then(serde_yaml::Value::as_str)
            .is_some_and(file_exists),
        "env_present" => crate::yaml_lookup(readiness, &["auth", "env_var"])
            .and_then(serde_yaml::Value::as_str)
            .and_then(|name| std::env::var(name.trim()).ok())
            .is_some_and(|value| !value.trim().is_empty()),
        _ => true,
    };
    if !auth_ok {
        return serde_json::json!({
            "backend_id": backend_id,
            "status": "interactive_auth_required",
            "blocked": true,
            "blocker_code": crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::InteractiveAuthRequired
            ),
            "current_model_ref": serde_json::Value::Null,
            "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
            "expected_model_ref": profile_projection["model"].clone(),
            "default_model_profile": profile_projection["default_model_profile"].clone(),
            "selected_model_profile": profile_projection["default_model_profile"].clone(),
            "model_profiles": profile_projection["model_profiles"].clone(),
            "next_actions": ["Complete carrier authentication outside sandbox, then rerun `vida status --json`."],
        });
    }

    let model_mode = crate::yaml_lookup(readiness, &["model", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or("none");
    let preferred_profile = crate::model_profile_contract::selected_model_profile_from_json_row(
        &profile_projection,
        preferred_profile_id,
    )
    .unwrap_or(serde_json::Value::Null);
    let expected_model_ref = crate::yaml_lookup(readiness, &["model", "expected_ref"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            preferred_profile["model_ref"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty() && !value.contains("provider-configured"))
                .map(str::to_string)
        })
        .or_else(|| {
            profile_projection["model"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty() && !value.contains("provider-configured"))
                .map(str::to_string)
        });
    let dispatch_can_override_model =
        crate::yaml_lookup(backend_entry, &["dispatch", "model_flag"])
            .and_then(serde_yaml::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
            && expected_model_ref.is_some();
    let allow_dispatch_override = crate::yaml_bool(
        crate::yaml_lookup(readiness, &["model", "allow_dispatch_override"]),
        dispatch_can_override_model,
    );

    let current_model_ref = match model_mode {
        "none" => None,
        "json_recent_ref" | "json_code_ref" => crate::yaml_lookup(readiness, &["model", "path"])
            .and_then(serde_yaml::Value::as_str)
            .and_then(|path| model_ref_from_json_state(model_mode, path)),
        "text_contains" => {
            let path = crate::yaml_lookup(readiness, &["model", "path"])
                .and_then(serde_yaml::Value::as_str);
            let expected_substring =
                crate::yaml_lookup(readiness, &["model", "expected_substring"])
                    .and_then(serde_yaml::Value::as_str)
                    .map(str::trim);
            match (path, expected_substring) {
                (Some(path), Some(expected_substring))
                    if read_text_file(path)
                        .is_some_and(|text| text.contains(expected_substring)) =>
                {
                    expected_model_ref
                        .clone()
                        .or_else(|| Some(expected_substring.to_string()))
                }
                _ => None,
            }
        }
        _ => None,
    };
    let selected_model_profile = selected_external_cli_profile(
        &profile_projection,
        current_model_ref.as_deref(),
        preferred_profile_id,
    );

    if let Some(expected_model_ref) = expected_model_ref.clone() {
        if current_model_ref.as_deref() != Some(expected_model_ref.as_str()) {
            if allow_dispatch_override {
                return serde_json::json!({
                    "backend_id": backend_id,
                    "status": "carrier_ready_with_override",
                    "blocked": false,
                    "blocker_code": serde_json::Value::Null,
                    "current_model_ref": current_model_ref,
                    "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
                    "expected_model_ref": expected_model_ref,
                    "default_model_profile": profile_projection["default_model_profile"].clone(),
                    "selected_model_profile": selected_model_profile,
                    "model_profiles": profile_projection["model_profiles"].clone(),
                    "next_actions": ["Carrier-local model state differs from project intent, but dispatch-level model pinning will override it."],
                });
            }
            return serde_json::json!({
                "backend_id": backend_id,
                "status": "model_not_pinned",
                "blocked": true,
                "blocker_code": crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ModelNotPinned
                ),
                "current_model_ref": current_model_ref,
                "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
                "expected_model_ref": expected_model_ref,
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "selected_model_profile": selected_model_profile,
                "model_profiles": profile_projection["model_profiles"].clone(),
                "next_actions": ["Fix carrier-local model selection or add dispatch-level model pinning before external dispatch."],
            });
        }
    }

    let provider_failure_mode = crate::yaml_lookup(readiness, &["provider_failure", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or("none");
    let provider_failure_substring =
        crate::yaml_lookup(readiness, &["provider_failure", "substring"])
            .and_then(serde_yaml::Value::as_str)
            .map(str::trim)
            .unwrap_or("");
    let provider_failure_detected = match provider_failure_mode {
        "file_contains" => crate::yaml_lookup(readiness, &["provider_failure", "path"])
            .and_then(serde_yaml::Value::as_str)
            .is_some_and(|path| file_contains(path, provider_failure_substring)),
        "recent_dir_contains" => {
            let max_age_seconds =
                crate::yaml_lookup(readiness, &["provider_failure", "max_age_seconds"])
                    .and_then(serde_yaml::Value::as_u64);
            crate::yaml_lookup(readiness, &["provider_failure", "path"])
                .and_then(serde_yaml::Value::as_str)
                .is_some_and(|path| {
                    latest_dir_file_contains(path, provider_failure_substring, max_age_seconds)
                })
        }
        "recent_dir_contains_any" => {
            let max_age_seconds =
                crate::yaml_lookup(readiness, &["provider_failure", "max_age_seconds"])
                    .and_then(serde_yaml::Value::as_u64);
            crate::yaml_lookup(readiness, &["provider_failure", "path"])
                .and_then(serde_yaml::Value::as_str)
                .is_some_and(|path| {
                    recent_dir_contains_any(path, provider_failure_substring, max_age_seconds)
                })
        }
        _ => false,
    };
    if provider_failure_detected {
        let provider_failure_status =
            crate::yaml_lookup(readiness, &["provider_failure", "status"])
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("provider_auth_failed");
        let provider_failure_blocker_code =
            crate::yaml_lookup(readiness, &["provider_failure", "blocker_code"])
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    crate::release1_contracts::blocker_code_str(
                        crate::release1_contracts::BlockerCode::ProviderAuthFailed,
                    )
                });
        let provider_failure_next_actions = crate::yaml_string_list(crate::yaml_lookup(
            readiness,
            &["provider_failure", "next_actions"],
        ));
        let next_actions = if provider_failure_next_actions.is_empty() {
            vec![
                "Repair the provider credential or provider-specific auth path, then rerun `vida status --json`."
                    .to_string(),
            ]
        } else {
            provider_failure_next_actions
        };
        return serde_json::json!({
            "backend_id": backend_id,
            "status": provider_failure_status,
            "blocked": true,
            "blocker_code": provider_failure_blocker_code,
            "current_model_ref": current_model_ref,
            "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
            "expected_model_ref": expected_model_ref,
            "default_model_profile": profile_projection["default_model_profile"].clone(),
            "selected_model_profile": selected_model_profile,
            "model_profiles": profile_projection["model_profiles"].clone(),
            "next_actions": next_actions,
        });
    }

    serde_json::json!({
        "backend_id": backend_id,
        "status": "carrier_ready",
        "blocked": false,
        "blocker_code": serde_json::Value::Null,
        "current_model_ref": current_model_ref,
        "current_reasoning_effort": profile_projection["current_reasoning_effort"].clone(),
        "expected_model_ref": expected_model_ref,
        "default_model_profile": profile_projection["default_model_profile"].clone(),
        "selected_model_profile": selected_model_profile,
        "model_profiles": profile_projection["model_profiles"].clone(),
        "next_actions": [],
    })
}

pub(crate) fn external_cli_backend_readiness_verdict(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> serde_json::Value {
    external_cli_carrier_readiness(backend_id, backend_entry, None)
}

pub(crate) fn external_cli_backend_readiness_verdict_for_profile(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    external_cli_carrier_readiness(backend_id, backend_entry, preferred_profile_id)
}

fn external_cli_readiness_summaries(overlay: &serde_yaml::Value) -> serde_json::Value {
    let carrier_rows = crate::yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(key, entry)| {
                    let backend_id = key.as_str()?.trim().to_string();
                    if backend_id.is_empty() {
                        return None;
                    }
                    let enabled = crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false);
                    let backend_class = crate::yaml_lookup(entry, &["subagent_backend_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    if !enabled || backend_class != "external_cli" {
                        return None;
                    }
                    Some(external_cli_backend_readiness_verdict(&backend_id, entry))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ready_like_count = carrier_rows
        .iter()
        .filter(|row| {
            matches!(
                row["status"].as_str(),
                Some("carrier_ready" | "carrier_ready_with_override")
            )
        })
        .count();
    let blocked_count = carrier_rows
        .iter()
        .filter(|row| row["blocked"].as_bool() == Some(true))
        .count();
    serde_json::json!({
        "total": carrier_rows.len(),
        "ready_like_count": ready_like_count,
        "blocked_count": blocked_count,
        "carriers": carrier_rows,
    })
}

fn route_primary_external_backends(overlay: &serde_yaml::Value) -> Vec<String> {
    fn collect_executor_backends_from_mapping(
        routes: &serde_yaml::Mapping,
        backends: &mut Vec<String>,
    ) {
        for route in routes.values() {
            if let Some(executor_backend) = crate::yaml_lookup(route, &["executor_backend"])
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                backends.push(executor_backend.to_string());
                continue;
            }
            if let Some(nested_routes) = crate::yaml_lookup(route, &["development_flow"])
                .and_then(serde_yaml::Value::as_mapping)
            {
                collect_executor_backends_from_mapping(nested_routes, backends);
            }
        }
    }

    let mut backends = Vec::new();
    for path in [
        ["agent_system", "routing", "development_flow"].as_slice(),
        ["agent_system", "routing"].as_slice(),
        ["routing", "development_flow"].as_slice(),
        ["routing"].as_slice(),
        ["development_flow"].as_slice(),
    ] {
        if let Some(routes) =
            crate::yaml_lookup(overlay, path).and_then(serde_yaml::Value::as_mapping)
        {
            collect_executor_backends_from_mapping(routes, &mut backends);
        }
    }
    backends.sort();
    backends.dedup();
    backends
}

fn external_cli_backend_ids(overlay: &serde_yaml::Value) -> std::collections::BTreeSet<String> {
    crate::yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(key, entry)| {
                    let backend_id = key.as_str()?.trim();
                    if backend_id.is_empty() {
                        return None;
                    }
                    let enabled = crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false);
                    let backend_class = crate::yaml_lookup(entry, &["subagent_backend_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    (enabled && backend_class == "external_cli").then(|| backend_id.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn backend_is_internal(overlay: &serde_yaml::Value, backend_id: &str) -> bool {
    let backend_id = backend_id.trim();
    if backend_id == "internal_subagents" {
        return true;
    }
    crate::yaml_lookup(
        overlay,
        &[
            "agent_system",
            "subagents",
            backend_id,
            "subagent_backend_class",
        ],
    )
    .and_then(serde_yaml::Value::as_str)
    .map(str::trim)
        == Some("internal")
}

fn route_has_internal_fallback(overlay: &serde_yaml::Value, route: &serde_yaml::Value) -> bool {
    [
        "fallback_executor_backend",
        "route_fallback_backend",
        "fallback_backend",
        "bridge_fallback_subagent",
    ]
    .into_iter()
    .filter_map(|field| {
        crate::yaml_lookup(route, &[field])
            .and_then(serde_yaml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
    .any(|backend_id| backend_is_internal(overlay, backend_id))
        || crate::yaml_string_list(crate::yaml_lookup(route, &["fanout_executor_backends"]))
            .iter()
            .any(|backend_id| backend_is_internal(overlay, backend_id))
}

fn route_primary_external_backends_without_internal_fallback(
    overlay: &serde_yaml::Value,
) -> Vec<String> {
    fn collect_required_external_backends_from_mapping(
        overlay: &serde_yaml::Value,
        external_backend_ids: &std::collections::BTreeSet<String>,
        routes: &serde_yaml::Mapping,
        backends: &mut Vec<String>,
    ) {
        for route in routes.values() {
            if let Some(executor_backend) = crate::yaml_lookup(route, &["executor_backend"])
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if external_backend_ids.contains(executor_backend)
                    && !route_has_internal_fallback(overlay, route)
                {
                    backends.push(executor_backend.to_string());
                }
                continue;
            }
            if let Some(nested_routes) = crate::yaml_lookup(route, &["development_flow"])
                .and_then(serde_yaml::Value::as_mapping)
            {
                collect_required_external_backends_from_mapping(
                    overlay,
                    external_backend_ids,
                    nested_routes,
                    backends,
                );
            }
        }
    }

    let external_backend_ids = external_cli_backend_ids(overlay);
    let mut backends = Vec::new();
    for path in [
        ["agent_system", "routing", "development_flow"].as_slice(),
        ["agent_system", "routing"].as_slice(),
        ["routing", "development_flow"].as_slice(),
        ["routing"].as_slice(),
        ["development_flow"].as_slice(),
    ] {
        if let Some(routes) =
            crate::yaml_lookup(overlay, path).and_then(serde_yaml::Value::as_mapping)
        {
            collect_required_external_backends_from_mapping(
                overlay,
                &external_backend_ids,
                routes,
                &mut backends,
            );
        }
    }
    backends.sort();
    backends.dedup();
    backends
}

pub(crate) fn is_sandbox_active_from_env() -> bool {
    let candidates = [
        std::env::var("CODEX_SANDBOX_MODE").ok(),
        std::env::var("SANDBOX_MODE").ok(),
        std::env::var("VIDA_SANDBOX_MODE").ok(),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_ascii_lowercase())
        .find(|value| !value.is_empty())
        .map(|value| {
            !matches!(
                value.as_str(),
                "danger-full-access" | "none" | "off" | "disabled" | "false"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn can_resolve_public_network() -> bool {
    use std::net::ToSocketAddrs;
    if let Ok(override_value) = std::env::var("VIDA_NETWORK_PROBE_OVERRIDE") {
        let normalized = override_value.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "reachable" | "online" | "true" | "1") {
            return true;
        }
        if matches!(
            normalized.as_str(),
            "unreachable" | "offline" | "false" | "0"
        ) {
            return false;
        }
    }
    ("example.com", 443)
        .to_socket_addrs()
        .map(|mut rows| rows.next().is_some())
        .unwrap_or(false)
}

pub(crate) fn external_cli_tool_contract_summary(
    selected_execution_class: &str,
    requires_external_cli: bool,
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> serde_json::Value {
    let runtime_root_configured = selected_cli_entry
        .and_then(|entry| crate::yaml_lookup(entry, &["runtime_root"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    crate::release1_contracts::cli_probe_tool_contract_summary(
        selected_execution_class,
        requires_external_cli,
        selected_cli_entry.is_some(),
        runtime_root_configured,
    )
}

pub(crate) fn external_cli_preflight_summary(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> serde_json::Value {
    external_cli_preflight_summary_with_probe(
        overlay,
        selected_cli_system,
        selected_cli_entry,
        None,
    )
}

#[cfg(test)]
fn external_cli_preflight_summary_with_probe_override(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    sandbox_active: bool,
    network_reachable: bool,
) -> serde_json::Value {
    external_cli_preflight_summary_with_probe(
        overlay,
        selected_cli_system,
        selected_cli_entry,
        Some((sandbox_active, network_reachable)),
    )
}

fn external_cli_preflight_summary_with_probe(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    probe_override: Option<(bool, bool)>,
) -> serde_json::Value {
    let selected_execution_class = selected_cli_entry
        .map(|entry| {
            crate::project_activator_surface::host_cli_system_execution_class(
                entry,
                selected_cli_system,
            )
        })
        .unwrap_or_else(|| "unknown".to_string());
    let selected_is_external = selected_execution_class == "external";
    let has_enabled_external_subagents =
        crate::yaml_lookup(overlay, &["agent_system", "subagents"])
            .and_then(serde_yaml::Value::as_mapping)
            .map(|mapping| {
                mapping.values().any(|entry| {
                    let enabled = crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false);
                    let backend = crate::yaml_lookup(entry, &["subagent_backend_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .map(str::to_ascii_lowercase)
                        .unwrap_or_default();
                    enabled && backend == "external_cli"
                })
            })
            .unwrap_or(false);
    let route_primary_backends = route_primary_external_backends(overlay);
    let route_primary_required_backends =
        route_primary_external_backends_without_internal_fallback(overlay);
    let route_primary_external_required = !route_primary_required_backends.is_empty();
    let hybrid_external_cli_relevant = !selected_is_external && has_enabled_external_subagents;
    let requires_external_cli = selected_is_external || route_primary_external_required;
    let effective_execution_posture = if selected_is_external {
        "external"
    } else if hybrid_external_cli_relevant {
        "mixed"
    } else if selected_execution_class == "internal" {
        "internal"
    } else {
        "unknown"
    };
    let (sandbox_active, network_reachable) = match probe_override {
        Some(value) => value,
        None => {
            let sandbox_active = is_sandbox_active_from_env();
            let network_reachable = if requires_external_cli && sandbox_active {
                can_resolve_public_network()
            } else {
                true
            };
            (sandbox_active, network_reachable)
        }
    };
    let tool_contract = external_cli_tool_contract_summary(
        selected_execution_class.as_str(),
        requires_external_cli,
        selected_cli_entry,
    );
    let tool_contract_blocked = tool_contract["status"].as_str() == Some("blocked");
    let tool_contract_blocker = crate::release1_contracts::cli_probe_tool_contract_blocker_code(
        selected_execution_class.as_str(),
        selected_cli_entry.is_some(),
        selected_cli_entry
            .and_then(|entry| crate::yaml_lookup(entry, &["runtime_root"]))
            .and_then(serde_yaml::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
    );
    let baseline_for_blocker = |blocker_code: Option<crate::release1_contracts::BlockerCode>| {
        let trace_baseline = crate::release1_contracts::cli_probe_trace_baseline_summary(
            if blocker_code.is_some() {
                crate::release1_contracts::Release1ContractStatus::Blocked
            } else {
                crate::release1_contracts::Release1ContractStatus::Pass
            },
            blocker_code,
            selected_execution_class.as_str(),
        );
        let incident_baseline =
            crate::release1_contracts::cli_probe_incident_baseline_summary(blocker_code);
        (trace_baseline, incident_baseline)
    };
    let (trace_baseline, incident_baseline) = baseline_for_blocker(tool_contract_blocker);
    let carrier_readiness = external_cli_readiness_summaries(overlay);
    let blocked_primary_backends = carrier_readiness["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|carrier| carrier["blocked"].as_bool() == Some(true))
        .filter_map(|carrier| carrier["backend_id"].as_str())
        .filter(|backend_id| {
            route_primary_backends
                .iter()
                .any(|backend| backend == backend_id)
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    let blocked_required_primary_backends = blocked_primary_backends
        .iter()
        .filter(|backend_id| {
            route_primary_required_backends
                .iter()
                .any(|required| required == *backend_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let primary_blocker_next_actions = if blocked_primary_backends.is_empty() {
        serde_json::json!([])
    } else {
        serde_json::json!([format!(
            "One or more route-primary external backends are currently blocked: {}. Reroute, wait for recovery, or switch those routes to another carrier before relying on them.",
            blocked_primary_backends.join(", ")
        )])
    };

    if tool_contract_blocked {
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": requires_external_cli,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "effective_execution_posture": effective_execution_posture,
            "mixed_posture": effective_execution_posture == "mixed",
            "tool_contract": tool_contract,
            "trace_baseline": trace_baseline,
            "incident_baseline": incident_baseline,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "route_primary_external_required_backends": route_primary_required_backends,
            "blocked_primary_backends": blocked_primary_backends,
            "blocked_required_primary_backends": blocked_required_primary_backends,
            "sandbox_active": sandbox_active,
            "network_reachable": network_reachable,
            "blocker_code": tool_contract["blocker_code"].clone(),
            "next_actions": [
                "Fix the selected host CLI system entry or runtime root in `vida.config.yaml`.",
                "Rerun `vida status --json` after restoring the canonical tool contract fields.",
            ]
        });
    }

    if requires_external_cli && sandbox_active && !network_reachable {
        let blocker_code =
            crate::release1_contracts::BlockerCode::ExternalCliNetworkAccessUnavailableUnderSandbox;
        let (trace_baseline, incident_baseline) = baseline_for_blocker(Some(blocker_code));
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": true,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "effective_execution_posture": effective_execution_posture,
            "mixed_posture": effective_execution_posture == "mixed",
            "tool_contract": tool_contract,
            "trace_baseline": trace_baseline,
            "incident_baseline": incident_baseline,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "route_primary_external_required_backends": route_primary_required_backends,
            "blocked_primary_backends": blocked_primary_backends,
            "blocked_required_primary_backends": blocked_required_primary_backends,
            "sandbox_active": true,
            "network_reachable": false,
            "blocker_code": crate::release1_contracts::blocker_code_str(blocker_code),
            "next_actions": [
                "Allow network access for this session or rerun outside sandbox before using external CLI agents.",
                "If sandbox must stay enabled, switch host and routing to an internal backend in `vida.config.yaml`.",
                "Rerun `vida status --json` and then retry the external CLI command."
            ]
        });
    }

    let no_ready_carriers = requires_external_cli
        && carrier_readiness["total"].as_u64().unwrap_or(0) > 0
        && carrier_readiness["ready_like_count"].as_u64().unwrap_or(0) == 0;
    let blocked_required_primary =
        requires_external_cli && !blocked_required_primary_backends.is_empty();
    if no_ready_carriers || blocked_required_primary {
        let first_blocker = carrier_readiness["carriers"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| {
                        row["blocked"].as_bool() == Some(true)
                            && (!blocked_required_primary
                                || row["backend_id"].as_str().is_some_and(|backend_id| {
                                    blocked_required_primary_backends
                                        .iter()
                                        .any(|blocked| blocked == backend_id)
                                }))
                    })
                    .and_then(|row| row.get("blocker_code"))
                    .cloned()
            })
            .unwrap_or(serde_json::Value::Null);
        let blocker_code = first_blocker
            .as_str()
            .and_then(crate::release1_contracts::BlockerCode::from_str);
        let (trace_baseline, incident_baseline) = baseline_for_blocker(blocker_code);
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": requires_external_cli,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "effective_execution_posture": effective_execution_posture,
            "mixed_posture": effective_execution_posture == "mixed",
            "tool_contract": tool_contract,
            "trace_baseline": trace_baseline,
            "incident_baseline": incident_baseline,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "route_primary_external_required_backends": route_primary_required_backends,
            "blocked_primary_backends": blocked_primary_backends,
            "blocked_required_primary_backends": blocked_required_primary_backends,
            "sandbox_active": sandbox_active,
            "network_reachable": network_reachable,
            "blocker_code": first_blocker,
            "next_actions": [
                "Repair carrier auth or model state for at least one enabled external CLI backend.",
                "Rerun `vida status --json` after the bounded carrier fix."
            ]
        });
    }

    serde_json::json!({
        "status": "pass",
        "requires_external_cli": requires_external_cli,
        "external_cli_subagents_present": has_enabled_external_subagents,
        "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
        "selected_execution_class": selected_execution_class,
        "effective_execution_posture": effective_execution_posture,
        "mixed_posture": effective_execution_posture == "mixed",
        "tool_contract": tool_contract,
        "trace_baseline": trace_baseline,
        "incident_baseline": incident_baseline,
        "carrier_readiness": carrier_readiness,
        "route_primary_external_backends": route_primary_backends,
        "route_primary_external_required_backends": route_primary_required_backends,
        "blocked_primary_backends": blocked_primary_backends,
        "blocked_required_primary_backends": blocked_required_primary_backends,
        "sandbox_active": sandbox_active,
        "network_reachable": network_reachable,
        "blocker_code": serde_json::Value::Null,
        "next_actions": primary_blocker_next_actions
    })
}

#[cfg(test)]
mod tests {
    use super::{
        adapter_prewrite_guard_capabilities, external_cli_backend_readiness_verdict_for_profile,
        external_cli_preflight_summary, external_cli_preflight_summary_with_probe_override,
        external_cli_probe_command_is_allowlisted,
    };
    use std::fs;

    #[test]
    fn adapter_prewrite_guard_capabilities_reports_test_fixture_active() {
        let capabilities = adapter_prewrite_guard_capabilities("sh");
        assert_eq!(capabilities["status"], "available");
        assert_eq!(capabilities["pre_write_enforcement"], true);
        assert_eq!(capabilities["explicit_extension_arg"], true);
    }

    #[test]
    fn external_cli_probe_allowlist_rejects_path_like_commands_but_keeps_pi_provider() {
        assert!(external_cli_probe_command_is_allowlisted("pi"));
        assert!(external_cli_probe_command_is_allowlisted("vida-pi-agent"));
        assert!(!external_cli_probe_command_is_allowlisted("./pi"));
        assert!(!external_cli_probe_command_is_allowlisted("tools\\pi.exe"));
        assert!(!external_cli_probe_command_is_allowlisted("not-a-carrier"));
    }

    fn current_exe_command() -> String {
        std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string()
    }

    fn fake_pi_list_models_command(models: &[&str]) -> String {
        let root = std::env::temp_dir().join(format!(
            "vida-fake-pi-list-models-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should support unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("fake pi command dir should exist");
        #[cfg(windows)]
        {
            let path = root.join("fake-pi.cmd");
            let mut body = String::from("@echo off\r\n");
            body.push_str("if \"%1\"==\"--list-models\" (\r\n");
            body.push_str("  echo provider model\r\n");
            for model in models {
                if let Some((provider, model_id)) = model.split_once('/') {
                    body.push_str(&format!("  echo {provider} {model_id}\r\n"));
                } else {
                    body.push_str(&format!("  echo {model}\r\n"));
                }
            }
            body.push_str("  exit /b 0\r\n)\r\nexit /b 0\r\n");
            fs::write(&path, body).expect("fake pi command should write");
            path.display().to_string()
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = root.join("fake-pi.sh");
            let mut body = String::from(
                "#!/bin/sh\nif [ \"$1\" = \"--list-models\" ]; then\n  echo 'provider model'\n",
            );
            for model in models {
                if let Some((provider, model_id)) = model.split_once('/') {
                    body.push_str(&format!("  echo '{provider} {model_id}'\n"));
                } else {
                    body.push_str(&format!("  echo '{model}'\n"));
                }
            }
            body.push_str("  exit 0\nfi\nexit 0\n");
            fs::write(&path, body).expect("fake pi command should write");
            let mut permissions = fs::metadata(&path)
                .expect("fake pi command metadata should read")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("fake pi command should be executable");
            path.display().to_string()
        }
    }

    fn pi_backend_entry(adapter_command: &str, provider_command: &str) -> serde_yaml::Value {
        serde_yaml::to_value(serde_json::json!({
            "enabled": true,
            "subagent_backend_class": "external_cli",
            "detect_command": "vida-pi-agent",
            "default_model": "openai-codex/gpt-5.5",
            "default_model_profile": "pi_gpt55_medium_guarded",
            "model_profiles": {
                "pi_gpt55_medium_guarded": {
                    "profile_id": "pi_gpt55_medium_guarded",
                    "provider": "pi",
                    "model_ref": "openai-codex/gpt-5.5",
                    "reasoning_effort": "medium",
                    "normalized_cost_units": 4,
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "write_scope": "guard_required_owned_paths"
                },
                "pi_gpt55_high_readonly": {
                    "profile_id": "pi_gpt55_high_readonly",
                    "provider": "pi",
                    "model_ref": "openai-codex/gpt-5.5",
                    "reasoning_effort": "high",
                    "normalized_cost_units": 16,
                    "runtime_roles": ["verifier"],
                    "task_classes": ["review", "verification"],
                    "write_scope": "none"
                }
            },
            "dispatch": {
                "command": "vida-pi-agent",
                "model_flag": "--model"
            },
            "readiness": {
                "adapter": {"mode": "command_found", "command": adapter_command},
                "provider": {"mode": "command_found", "command": provider_command},
                "model_catalog": {"mode": "pi_rpc_get_available_models", "required": true},
                "write_scope_guard": {
                    "mode": "adapter_feature_required",
                    "required_for_write_profiles": true,
                    "fail_closed_until_available": true
                }
            }
        }))
        .expect("pi backend yaml value should render")
    }

    #[test]
    fn pi_readiness_blocks_when_adapter_command_missing_distinctly() {
        let provider = fake_pi_list_models_command(&["openai-codex/gpt-5.5"]);
        let entry = pi_backend_entry(
            "vida-definitely-missing-pi-adapter-command-for-test",
            &provider,
        );

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_high_readonly"),
        );

        assert_eq!(readiness["status"], "pi_adapter_command_not_found");
        assert_eq!(readiness["blocked"], true);
        assert_eq!(readiness["adapter"]["status"], "command_not_found");
        assert_eq!(readiness["adapter"]["source"], "readiness.adapter.command");
        assert_eq!(readiness["provider"]["status"], "not_checked");
    }

    #[test]
    fn pi_readiness_blocks_when_provider_command_missing_after_adapter_found() {
        let entry = pi_backend_entry(
            &current_exe_command(),
            "vida-definitely-missing-pi-provider-command-for-test",
        );

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_high_readonly"),
        );

        assert_eq!(readiness["status"], "pi_provider_command_not_found");
        assert_eq!(readiness["blocked"], true);
        assert_eq!(readiness["adapter"]["status"], "command_found");
        assert_eq!(readiness["provider"]["status"], "command_not_found");
        assert_eq!(
            readiness["provider"]["source"],
            "readiness.provider.command"
        );
    }

    #[test]
    fn pi_readiness_reports_model_catalog_ready_for_readonly_profile() {
        let provider = fake_pi_list_models_command(&["openai-codex/gpt-5.5"]);
        let entry = pi_backend_entry(&current_exe_command(), &provider);

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_high_readonly"),
        );

        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(readiness["blocked"], false);
        assert_eq!(readiness["model_catalog"]["status"], "model_available");
        assert_eq!(readiness["write_scope_guard"]["status"], "not_blocking");
    }

    #[test]
    fn pi_readiness_blocks_when_model_catalog_is_missing_selected_model() {
        let provider = fake_pi_list_models_command(&["openai-codex/gpt-5.4"]);
        let entry = pi_backend_entry(&current_exe_command(), &provider);

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_high_readonly"),
        );

        assert_eq!(readiness["status"], "pi_model_unavailable");
        assert_eq!(readiness["blocked"], true);
        assert_eq!(readiness["blocker_code"], "model_not_pinned");
        assert_eq!(readiness["model_catalog"]["status"], "model_not_found");
    }

    #[test]
    fn pi_readiness_blocks_guarded_write_profile_until_write_guard_exists() {
        let provider = fake_pi_list_models_command(&["openai-codex/gpt-5.5"]);
        let entry = pi_backend_entry(&current_exe_command(), &provider);

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_medium_guarded"),
        );

        assert_eq!(readiness["status"], "pi_write_scope_guard_required");
        assert_eq!(readiness["blocked"], true);
        assert_eq!(readiness["blocker_code"], "write_scope_guard_required");
        assert_eq!(readiness["model_catalog"]["status"], "model_available");
        assert_eq!(
            readiness["write_scope_guard"]["status"],
            "guard_required_not_active"
        );
    }

    #[test]
    fn pi_readiness_does_not_block_readonly_profile_on_write_guard() {
        let provider = fake_pi_list_models_command(&["openai-codex/gpt-5.5"]);
        let entry = pi_backend_entry(&current_exe_command(), &provider);

        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "pi_cli",
            &entry,
            Some("pi_gpt55_high_readonly"),
        );

        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(readiness["blocked"], false);
        assert_eq!(readiness["write_scope_guard"]["status"], "not_blocking");
        assert_eq!(
            readiness["write_scope_guard"]["selected_profile_requires_guard"],
            false
        );
    }

    #[test]
    fn internal_host_without_enabled_external_backends_does_not_require_external_cli() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
"#,
        )
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["hybrid_external_cli_relevant"], false);
        assert_eq!(summary["selected_execution_class"], "internal");
        assert_eq!(summary["effective_execution_posture"], "internal");
        assert_eq!(summary["mixed_posture"], false);
    }

    #[test]
    fn internal_host_with_enabled_external_backends_is_hybrid_aware() {
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

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["hybrid_external_cli_relevant"], true);
        assert_eq!(summary["selected_execution_class"], "internal");
        assert_eq!(summary["effective_execution_posture"], "mixed");
        assert_eq!(summary["mixed_posture"], true);
    }

    #[test]
    fn external_cli_detect_command_missing_blocks_carrier_readiness() {
        let entry = serde_yaml::to_value(serde_json::json!({
            "enabled": true,
            "subagent_backend_class": "external_cli",
            "detect_command": "vida-definitely-missing-external-cli-command-for-test",
            "default_model_profile": "hermes_provider_configured_review",
            "model_profiles": {
                "hermes_provider_configured_review": {
                    "profile_id": "hermes_provider_configured_review",
                    "model_ref": "hermes/provider-configured",
                    "provider": "hermes",
                    "reasoning_effort": "provider_default",
                    "normalized_cost_units": 0,
                    "runtime_roles": ["coach"],
                    "task_classes": ["review"],
                    "write_scope": "none"
                }
            }
        }))
        .expect("yaml value should render");

        let readiness =
            external_cli_backend_readiness_verdict_for_profile("hermes_cli", &entry, None);

        assert_eq!(readiness["status"], "external_cli_command_not_found");
        assert_eq!(readiness["blocked"], true);
        assert_eq!(readiness["blocker_code"], "tool_execution_failed");
        assert_eq!(
            readiness["command_resolution"]["status"],
            "command_not_found"
        );
    }

    #[test]
    fn external_cli_detect_command_present_preserves_ready_status() {
        let current_exe = std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string();
        let entry = serde_yaml::to_value(serde_json::json!({
            "enabled": true,
            "subagent_backend_class": "external_cli",
            "detect_command": current_exe,
            "default_model_profile": "hermes_provider_configured_review",
            "model_profiles": {
                "hermes_provider_configured_review": {
                    "profile_id": "hermes_provider_configured_review",
                    "model_ref": "hermes/provider-configured",
                    "provider": "hermes",
                    "reasoning_effort": "provider_default",
                    "normalized_cost_units": 0,
                    "runtime_roles": ["coach"],
                    "task_classes": ["review"],
                    "write_scope": "none"
                }
            }
        }))
        .expect("yaml value should render");

        let readiness =
            external_cli_backend_readiness_verdict_for_profile("hermes_cli", &entry, None);

        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(readiness["blocked"], false);
    }

    #[test]
    fn external_cli_dispatch_command_is_authoritative_probe_when_present() {
        let current_exe = std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string();
        let entry = serde_yaml::to_value(serde_json::json!({
            "enabled": true,
            "subagent_backend_class": "external_cli",
            "detect_command": "vida-definitely-missing-external-cli-command-for-test",
            "dispatch": {
                "command": current_exe
            },
            "default_model_profile": "hermes_provider_configured_review",
            "model_profiles": {
                "hermes_provider_configured_review": {
                    "profile_id": "hermes_provider_configured_review",
                    "model_ref": "hermes/provider-configured",
                    "provider": "hermes",
                    "reasoning_effort": "provider_default",
                    "normalized_cost_units": 0,
                    "runtime_roles": ["coach"],
                    "task_classes": ["review"],
                    "write_scope": "none"
                }
            }
        }))
        .expect("yaml value should render");

        let readiness =
            external_cli_backend_readiness_verdict_for_profile("hermes_cli", &entry, None);

        assert_eq!(readiness["status"], "carrier_ready");
        assert_eq!(readiness["blocked"], false);
    }

    #[test]
    fn external_cli_preflight_keeps_missing_external_candidate_diagnostic_for_internal_route() {
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
      detect_command: vida-definitely-missing-external-cli-command-for-test
"#,
        )
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);

        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(summary["carrier_readiness"]["ready_like_count"], 0);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "external_cli_command_not_found"
        );
    }

    #[test]
    fn external_cli_preflight_blocks_missing_external_route_without_internal_fallback() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
routing:
  development_flow:
    coach:
      executor_backend: hermes_cli
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      detect_command: vida-definitely-missing-external-cli-command-for-test
"#,
        )
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);

        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(
            summary["route_primary_external_required_backends"][0],
            "hermes_cli"
        );
        assert_eq!(summary["blocker_code"], "tool_execution_failed");
    }

    #[test]
    fn external_host_preserves_external_requirement_behavior() {
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

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "opencode"]);
        let summary = external_cli_preflight_summary_with_probe_override(
            &overlay, "opencode", entry, false, true,
        );
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(summary["hybrid_external_cli_relevant"], false);
        assert_eq!(summary["selected_execution_class"], "external");
        assert_eq!(summary["effective_execution_posture"], "external");
        assert_eq!(summary["mixed_posture"], false);
        assert_eq!(summary["trace_baseline"]["artifact_type"], "trace_event");
        assert_eq!(
            summary["incident_baseline"]["artifact_type"],
            "incident_evidence_bundle"
        );
    }

    #[test]
    fn external_cli_preflight_projects_trace_and_incident_baselines_when_tool_contract_blocks() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: opencode
  systems:
    opencode:
      enabled: true
      execution_class: external
"#,
        )
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "opencode"]);
        let summary = external_cli_preflight_summary(&overlay, "opencode", entry);
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["tool_contract"]["status"], "blocked");
        assert_eq!(summary["trace_baseline"]["status"], "blocked");
        assert_eq!(summary["trace_baseline"]["artifact_type"], "trace_event");
        assert_eq!(summary["incident_baseline"]["status"], "open");
        assert_eq!(
            summary["incident_baseline"]["artifact_type"],
            "incident_evidence_bundle"
        );
        assert_eq!(
            summary["incident_baseline"]["trigger_reason"],
            "external_cli_preflight_gate:tool_contract_incomplete"
        );
    }

    #[test]
    fn external_cli_preflight_blocks_when_only_external_carrier_needs_auth() {
        let temp_root =
            std::env::temp_dir().join(format!("vida-external-cli-auth-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).expect("temp root should exist");
        let missing_auth = temp_root.join("missing-auth.json");
        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
      readiness:
        auth:
          mode: file_present
          path: {}
"#,
            missing_auth.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "interactive_auth_required"
        );
    }

    #[test]
    fn external_cli_preflight_sets_blocked_baselines_for_sandbox_network_gate() {
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

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "opencode"]);
        let summary = external_cli_preflight_summary_with_probe_override(
            &overlay, "opencode", entry, true, false,
        );
        assert_eq!(summary["status"], "blocked");
        assert_eq!(
            summary["blocker_code"],
            "external_cli_network_access_unavailable_under_sandbox"
        );
        assert_eq!(summary["trace_baseline"]["status"], "blocked");
        assert_eq!(summary["trace_baseline"]["outcome"], "blocked");
        assert_eq!(
            summary["incident_baseline"]["recovery_outcome"],
            "pending_remediation"
        );
    }

    #[test]
    fn external_cli_preflight_reports_ready_with_override_for_model_drift() {
        let temp_root =
            std::env::temp_dir().join(format!("vida-external-cli-model-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).expect("temp root should exist");
        let auth_path = temp_root.join("auth.json");
        let model_path = temp_root.join("model.json");
        fs::write(&auth_path, "{}").expect("auth file should write");
        fs::write(
            &model_path,
            r#"{"recent":[{"providerID":"opencode","modelID":"gpt-5.1-codex-mini"}]}"#,
        )
        .expect("model file should write");
        let command_path = std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string()
            .replace('\'', "''");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
        command: '{}'
        static_args: ["run"]
        model_flag: --model
      readiness:
        auth:
          mode: file_present
          path: {}
        model:
          mode: json_recent_ref
          path: {}
          expected_ref: opencode/minimax-m2.5-free
          allow_dispatch_override: true
"#,
            command_path,
            auth_path.display(),
            model_path.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "carrier_ready_with_override"
        );
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["selected_model_profile"],
            "opencode_codex_mini_review"
        );
    }

    #[test]
    fn external_cli_preflight_projects_current_nondefault_profile_when_model_is_not_pinned() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-model-unpinned-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).expect("temp root should exist");
        let auth_path = temp_root.join("auth.json");
        let model_path = temp_root.join("model.json");
        fs::write(&auth_path, "{}").expect("auth file should write");
        fs::write(
            &model_path,
            r#"{"recent":[{"providerID":"opencode","modelID":"gpt-5.1-codex-mini"}]}"#,
        )
        .expect("model file should write");
        let command_path = std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string()
            .replace('\'', "''");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
        command: '{}'
        static_args: ["run"]
      readiness:
        auth:
          mode: file_present
          path: {}
        model:
          mode: json_recent_ref
          path: {}
          expected_ref: opencode/minimax-m2.5-free
"#,
            command_path,
            auth_path.display(),
            model_path.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "model_not_pinned"
        );
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["selected_model_profile"],
            "opencode_codex_mini_review"
        );
    }

    #[test]
    fn external_cli_readiness_can_be_pinned_to_preferred_profile() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-profile-aware-readiness-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).expect("temp root should exist");
        let auth_path = temp_root.join("auth.json");
        let model_path = temp_root.join("model.json");
        fs::write(&auth_path, "{}").expect("auth file should write");
        fs::write(
            &model_path,
            r#"{"recent":[{"providerID":"opencode","modelID":"minimax-m2.5-free"}]}"#,
        )
        .expect("model file should write");
        let command_path = std::env::current_exe()
            .expect("current executable path should be available")
            .display()
            .to_string()
            .replace('\'', "''");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
            r#"
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
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
        command: '{}'
        static_args: ["run"]
        model_flag: --model
      readiness:
        auth:
          mode: file_present
          path: {}
        model:
          mode: json_recent_ref
          path: {}
          allow_dispatch_override: true
"#,
            command_path,
            auth_path.display(),
            model_path.display()
        ))
        .expect("overlay yaml should parse");

        let backend_entry =
            crate::yaml_lookup(&overlay, &["agent_system", "subagents", "opencode_cli"])
                .expect("backend entry should exist");
        let readiness = external_cli_backend_readiness_verdict_for_profile(
            "opencode_cli",
            backend_entry,
            Some("opencode_codex_mini_review"),
        );

        assert_eq!(readiness["status"], "carrier_ready_with_override");
        assert_eq!(
            readiness["selected_model_profile"],
            "opencode_codex_mini_review"
        );
        assert_eq!(
            readiness["expected_model_ref"],
            "opencode/gpt-5.1-codex-mini"
        );
    }

    #[test]
    fn external_cli_preflight_reports_provider_auth_failed_from_recent_log_signal() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-provider-auth-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        let log_dir = temp_root.join("logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        let auth_path = temp_root.join("auth.json");
        fs::write(&auth_path, "{}").expect("auth file should write");
        fs::write(
            log_dir.join("latest.log"),
            "ERROR provider returned Authentication Failed",
        )
        .expect("log file should write");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
      readiness:
        auth:
          mode: file_present
          path: {}
        provider_failure:
          mode: recent_dir_contains
          path: {}
          substring: Authentication Failed
          max_age_seconds: 3600
"#,
            auth_path.display(),
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "provider_auth_failed"
        );
    }

    #[test]
    fn external_cli_preflight_reports_configured_provider_failure_blocker() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-provider-quota-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        let log_dir = temp_root.join("logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("latest.log"),
            "ERROR 429 You exceeded your current quota",
        )
        .expect("log file should write");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
      readiness:
        provider_failure:
          mode: recent_dir_contains
          path: {}
          substring: exceeded your current quota
          max_age_seconds: 3600
          status: provider_failure_detected
          blocker_code: tool_execution_failed
          next_actions:
            - Wait for provider quota reset or refresh the configured backend credentials.
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "provider_failure_detected"
        );
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["next_actions"][0],
            "Wait for provider quota reset or refresh the configured backend credentials."
        );
    }

    #[test]
    fn external_cli_preflight_scans_any_recent_provider_failure_file() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-provider-any-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        let log_dir = temp_root.join("logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("older-quota.log"),
            "ERROR 429 You exceeded your current quota",
        )
        .expect("older quota log should write");
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(log_dir.join("latest-success.log"), "INFO all good")
            .expect("latest success log should write");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
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
      readiness:
        provider_failure:
          mode: recent_dir_contains_any
          path: {}
          substring: exceeded your current quota
          max_age_seconds: 3600
          status: provider_failure_detected
          blocker_code: tool_execution_failed
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "provider_failure_detected"
        );
    }

    #[test]
    fn external_cli_preflight_surfaces_blocked_route_primary_backends() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-primary-blocked-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        let log_dir = temp_root.join("logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("quota.log"),
            "ERROR 429 You exceeded your current quota",
        )
        .expect("quota log should write");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
routing:
  development_flow:
    coach:
      executor_backend: hermes_cli
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        provider_failure:
          mode: recent_dir_contains_any
          path: {}
          substring: exceeded your current quota
          max_age_seconds: 3600
          status: provider_failure_detected
          blocker_code: tool_execution_failed
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(summary["blocked_primary_backends"][0], "hermes_cli");
        assert_eq!(summary["route_primary_external_backends"][0], "hermes_cli");
        assert_eq!(
            summary["route_primary_external_required_backends"][0],
            "hermes_cli"
        );
        assert_eq!(summary["blocker_code"], "tool_execution_failed");
    }

    #[test]
    fn external_cli_preflight_keeps_route_primary_with_internal_fallback_diagnostic() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-external-cli-primary-fallback-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_root);
        let log_dir = temp_root.join("logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("quota.log"),
            "ERROR 429 You exceeded your current quota",
        )
        .expect("quota log should write");

        let overlay: serde_yaml::Value = serde_yaml::from_str(&format!(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
routing:
  development_flow:
    coach:
      executor_backend: hermes_cli
      fallback_executor_backend: internal_subagents
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        provider_failure:
          mode: recent_dir_contains_any
          path: {}
          substring: exceeded your current quota
          max_age_seconds: 3600
          status: provider_failure_detected
          blocker_code: tool_execution_failed
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], false);
        assert_eq!(summary["blocked_primary_backends"][0], "hermes_cli");
        assert!(summary["route_primary_external_required_backends"]
            .as_array()
            .expect("required backends should be an array")
            .is_empty());
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
    }

    #[test]
    fn route_primary_backends_discovers_real_project_shape_without_requiring_legacy_external_routes(
    ) {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vida.config.yaml");
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(&config_path).expect("project config should read"),
        )
        .expect("project config should parse");

        let backends = super::route_primary_external_backends(&overlay);
        assert!(backends
            .iter()
            .any(|backend| backend == "internal_subagents"));
        assert!(!backends.iter().any(|backend| backend == "qwen_cli"));
        let required_external =
            super::route_primary_external_backends_without_internal_fallback(&overlay);
        assert!(required_external.is_empty());
    }
}
