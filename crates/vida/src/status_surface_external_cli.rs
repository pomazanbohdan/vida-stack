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

fn external_cli_carrier_readiness(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
) -> serde_json::Value {
    let readiness = crate::yaml_lookup(backend_entry, &["readiness"]);
    if readiness.is_none() {
        return serde_json::json!({
            "backend_id": backend_id,
            "status": "carrier_ready",
            "blocked": false,
            "blocker_code": serde_json::Value::Null,
            "current_model_ref": serde_json::Value::Null,
            "expected_model_ref": serde_json::Value::Null,
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
            "expected_model_ref": serde_json::Value::Null,
            "next_actions": ["Complete carrier authentication outside sandbox, then rerun `vida status --json`."],
        });
    }

    let model_mode = crate::yaml_lookup(readiness, &["model", "mode"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or("none");
    let expected_model_ref = crate::yaml_lookup(readiness, &["model", "expected_ref"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            crate::yaml_lookup(backend_entry, &["default_model"])
                .and_then(serde_yaml::Value::as_str)
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

    if let Some(expected_model_ref) = expected_model_ref.clone() {
        if current_model_ref.as_deref() != Some(expected_model_ref.as_str()) {
            if allow_dispatch_override {
                return serde_json::json!({
                    "backend_id": backend_id,
                    "status": "carrier_ready_with_override",
                    "blocked": false,
                    "blocker_code": serde_json::Value::Null,
                    "current_model_ref": current_model_ref,
                    "expected_model_ref": expected_model_ref,
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
                "expected_model_ref": expected_model_ref,
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
        let provider_failure_status = crate::yaml_lookup(readiness, &["provider_failure", "status"])
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
        let provider_failure_next_actions =
            crate::yaml_string_list(crate::yaml_lookup(readiness, &["provider_failure", "next_actions"]));
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
            "expected_model_ref": expected_model_ref,
            "next_actions": next_actions,
        });
    }

    serde_json::json!({
        "backend_id": backend_id,
        "status": "carrier_ready",
        "blocked": false,
        "blocker_code": serde_json::Value::Null,
        "current_model_ref": current_model_ref,
        "expected_model_ref": expected_model_ref,
        "next_actions": [],
    })
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
                    Some(external_cli_carrier_readiness(&backend_id, entry))
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
        if let Some(routes) = crate::yaml_lookup(overlay, path).and_then(serde_yaml::Value::as_mapping)
        {
            collect_executor_backends_from_mapping(routes, &mut backends);
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
    let hybrid_external_cli_relevant = !selected_is_external && has_enabled_external_subagents;
    let requires_external_cli = selected_is_external || hybrid_external_cli_relevant;
    let sandbox_active = is_sandbox_active_from_env();
    let network_reachable = can_resolve_public_network();
    let tool_contract = external_cli_tool_contract_summary(
        selected_execution_class.as_str(),
        requires_external_cli,
        selected_cli_entry,
    );
    let tool_contract_blocked = tool_contract["status"].as_str() == Some("blocked");
    let carrier_readiness = external_cli_readiness_summaries(overlay);
    let route_primary_backends = route_primary_external_backends(overlay);
    let blocked_primary_backends = carrier_readiness["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|carrier| carrier["blocked"].as_bool() == Some(true))
        .filter_map(|carrier| carrier["backend_id"].as_str())
        .filter(|backend_id| route_primary_backends.iter().any(|backend| backend == backend_id))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let primary_blocker_next_actions = if blocked_primary_backends.is_empty() {
        serde_json::json!([])
    } else {
        serde_json::json!([
            format!(
                "One or more route-primary external backends are currently blocked: {}. Reroute, wait for recovery, or switch those routes to another carrier before relying on them.",
                blocked_primary_backends.join(", ")
            )
        ])
    };

    if tool_contract_blocked {
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": requires_external_cli,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "tool_contract": tool_contract,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "blocked_primary_backends": blocked_primary_backends,
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
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": true,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "tool_contract": tool_contract,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "blocked_primary_backends": blocked_primary_backends,
            "sandbox_active": true,
            "network_reachable": false,
            "blocker_code": crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::ExternalCliNetworkAccessUnavailableUnderSandbox
            ),
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
    if no_ready_carriers {
        let first_blocker = carrier_readiness["carriers"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["blocked"].as_bool() == Some(true))
                    .and_then(|row| row.get("blocker_code"))
                    .cloned()
            })
            .unwrap_or(serde_json::Value::Null);
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": requires_external_cli,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "tool_contract": tool_contract,
            "carrier_readiness": carrier_readiness,
            "route_primary_external_backends": route_primary_backends,
            "blocked_primary_backends": blocked_primary_backends,
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
        "tool_contract": tool_contract,
        "carrier_readiness": carrier_readiness,
        "route_primary_external_backends": route_primary_backends,
        "blocked_primary_backends": blocked_primary_backends,
        "sandbox_active": sandbox_active,
        "network_reachable": network_reachable,
        "blocker_code": serde_json::Value::Null,
        "next_actions": primary_blocker_next_actions
    })
}

#[cfg(test)]
mod tests {
    use super::external_cli_preflight_summary;
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
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["requires_external_cli"], true);
        assert_eq!(summary["hybrid_external_cli_relevant"], true);
        assert_eq!(summary["selected_execution_class"], "internal");
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
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["blocker_code"], "interactive_auth_required");
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "interactive_auth_required"
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
            r#"{"recent":[{"providerID":"zai","modelID":"glm-5.1"}]}"#,
        )
        .expect("model file should write");

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
      dispatch:
        command: opencode
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
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["blocker_code"], "provider_auth_failed");
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
    qwen_cli:
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
            - Wait for provider quota reset or switch qwen to API-key auth.
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["blocker_code"], "tool_execution_failed");
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["status"],
            "provider_failure_detected"
        );
        assert_eq!(
            summary["carrier_readiness"]["carriers"][0]["next_actions"][0],
            "Wait for provider quota reset or switch qwen to API-key auth."
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
    qwen_cli:
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
        assert_eq!(summary["status"], "blocked");
        assert_eq!(summary["blocker_code"], "tool_execution_failed");
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
      executor_backend: qwen_cli
agent_system:
  subagents:
    qwen_cli:
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
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
            log_dir.display()
        ))
        .expect("overlay yaml should parse");

        let entry = crate::yaml_lookup(&overlay, &["host_environment", "systems", "codex"]);
        let summary = external_cli_preflight_summary(&overlay, "codex", entry);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["blocked_primary_backends"][0], "qwen_cli");
        assert_eq!(summary["route_primary_external_backends"][0], "qwen_cli");
        assert!(summary["next_actions"][0]
            .as_str()
            .expect("next action should render")
            .contains("route-primary external backends are currently blocked"));
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
        assert!(
            backends.iter().any(|backend| backend == "qwen_cli"),
            "expected qwen_cli in route_primary_external_backends, got {backends:?}"
        );
    }
}
