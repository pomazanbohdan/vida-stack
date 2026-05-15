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

fn external_cli_carrier_readiness(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    let profile_projection = external_backend_profile_projection(backend_id, backend_entry);
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
    let readiness = crate::yaml_lookup(backend_entry, &["readiness"]);
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
    let sandbox_active = is_sandbox_active_from_env();
    let network_reachable = can_resolve_public_network();
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
        external_cli_backend_readiness_verdict_for_profile, external_cli_preflight_summary,
    };
    use std::fs;

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
        let summary = external_cli_preflight_summary(&overlay, "opencode", entry);
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
        std::env::set_var("CODEX_SANDBOX_MODE", "workspace-write");
        std::env::set_var("VIDA_NETWORK_PROBE_OVERRIDE", "offline");
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
        let summary = external_cli_preflight_summary(&overlay, "opencode", entry);
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

        std::env::remove_var("CODEX_SANDBOX_MODE");
        std::env::remove_var("VIDA_NETWORK_PROBE_OVERRIDE");
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
        assert!(
            summary["route_primary_external_required_backends"]
                .as_array()
                .expect("required backends should be an array")
                .is_empty()
        );
        assert_eq!(summary["blocker_code"], serde_json::Value::Null);
    }

    #[test]
    fn route_primary_external_backends_discovers_real_project_shape() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vida.config.yaml");
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(&config_path).expect("project config should read"),
        )
        .expect("project config should parse");

        let backends = super::route_primary_external_backends(&overlay);
        assert!(backends.iter().any(|backend| backend == "hermes_cli"));
        assert!(backends.iter().any(|backend| backend == "opencode_cli"));
        assert!(backends.iter().any(|backend| backend == "kilo_cli"));
        assert!(backends.iter().any(|backend| backend == "vibe_cli"));
        assert!(!backends.iter().any(|backend| backend == "qwen_cli"));
    }
}
