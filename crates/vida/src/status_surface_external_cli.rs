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

    if tool_contract_blocked {
        return serde_json::json!({
            "status": "blocked",
            "requires_external_cli": requires_external_cli,
            "external_cli_subagents_present": has_enabled_external_subagents,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant,
            "selected_execution_class": selected_execution_class,
            "tool_contract": tool_contract,
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
            "sandbox_active": true,
            "network_reachable": false,
            "blocker_code": "external_cli_network_access_unavailable_under_sandbox",
            "next_actions": [
                "Allow network access for this session or rerun outside sandbox before using external CLI agents.",
                "If sandbox must stay enabled, switch host and routing to an internal backend in `vida.config.yaml`.",
                "Rerun `vida status --json` and then retry the external CLI command."
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
        "sandbox_active": sandbox_active,
        "network_reachable": network_reachable,
        "blocker_code": serde_json::Value::Null,
        "next_actions": []
    })
}

#[cfg(test)]
mod tests {
    use super::external_cli_preflight_summary;

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
}
