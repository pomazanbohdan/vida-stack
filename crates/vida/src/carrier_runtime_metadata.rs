use std::collections::HashMap;

pub(crate) const DEFAULT_CARRIER_RUNTIME_MATERIALIZATION_MODE: &str =
    "config_materialized_runtime_projection";
pub(crate) const DEFAULT_CARRIER_CATALOG_OWNER: &str =
    "vida.config.yaml -> configured host-system carrier surfaces";
pub(crate) const DEFAULT_DISPATCH_ALIAS_OWNER: &str =
    "vida.config.yaml -> agent_extensions.registries.dispatch_aliases";
pub(crate) const DEFAULT_AGENT_IDENTITY: &str = "execution_carrier";
pub(crate) const DEFAULT_RUNTIME_ROLE_IDENTITY: &str = "activation_state";
pub(crate) const DEFAULT_SELECTION_RULE: &str =
    "capability_first_then_score_guard_then_cheapest_tier";
pub(crate) const DEFAULT_PROMOTION_SCORE: u64 = 75;
pub(crate) const DEFAULT_DEMOTION_SCORE: u64 = 45;
pub(crate) const DEFAULT_CONSECUTIVE_FAILURE_LIMIT: u64 = 3;
pub(crate) const DEFAULT_PROBATION_TASK_RUNS: u64 = 2;
pub(crate) const DEFAULT_RETIREMENT_FAILURE_LIMIT: u64 = 8;

pub(crate) fn carrier_runtime_materialization_mode(
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
) -> String {
    selected_host_cli_system
        .and_then(|system| {
            host_cli_system_registry.get(system).map(|entry| {
                super::yaml_string(super::yaml_lookup(entry, &["materialization_mode"]))
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    crate::status_surface_host_cli_system::default_host_cli_materialization_mode(
                        Some(entry),
                        system,
                    )
                })
            })
        })
        .unwrap_or_else(|| DEFAULT_CARRIER_RUNTIME_MATERIALIZATION_MODE.to_string())
}

pub(crate) fn carrier_runtime_source_of_truth(
    selected_host_cli_system: Option<&str>,
    dispatch_alias_rows_empty: bool,
    dispatch_aliases_path: Option<&str>,
) -> serde_json::Value {
    let dispatch_alias_owner = if dispatch_alias_rows_empty {
        selected_host_cli_system
            .map(|system| {
                format!(
                    "vida.config.yaml -> host_environment.systems.{system}.dispatch_aliases overlay"
                )
            })
            .unwrap_or_else(|| {
                "vida.config.yaml -> configured host-system dispatch_aliases overlay".to_string()
            })
    } else {
        dispatch_aliases_path
            .map(|path| {
                format!("vida.config.yaml -> agent_extensions.registries.dispatch_aliases ({path})")
            })
            .unwrap_or_else(|| {
                "vida.config.yaml -> agent_extensions.registries.dispatch_aliases".to_string()
            })
    };
    serde_json::json!({
        "carrier_catalog_owner": selected_host_cli_system
            .map(|system| format!(
                "vida.config.yaml -> host_environment.systems.{system}.carriers"
            ))
            .unwrap_or_else(|| DEFAULT_CARRIER_CATALOG_OWNER.to_string()),
        "dispatch_alias_owner": dispatch_alias_owner,
    })
}

pub(crate) fn carrier_runtime_agent_model(
    config: &serde_yaml::Value,
    worker_strategy: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "agent_identity": super::non_empty_yaml_string(
            config,
            &["agent_system", "agent_identity"],
        )
        .unwrap_or_else(|| DEFAULT_AGENT_IDENTITY.to_string()),
        "runtime_role_identity": super::non_empty_yaml_string(
            config,
            &["agent_system", "runtime_role_identity"],
        )
        .unwrap_or_else(|| DEFAULT_RUNTIME_ROLE_IDENTITY.to_string()),
        "selection_rule": selection_policy_rule(
            &worker_strategy["selection_policy"],
        ),
    })
}

pub(crate) fn pricing_vendor_basis(config: &serde_yaml::Value) -> serde_json::Value {
    serde_json::to_value(
        super::yaml_lookup(config, &["agent_system", "pricing", "vendor_basis"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn selection_policy_rule(scoring_policy: &serde_json::Value) -> &str {
    scoring_policy["rule"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_SELECTION_RULE)
}

pub(crate) fn selection_policy_u64(
    scoring_policy: &serde_json::Value,
    key: &str,
    default: u64,
) -> u64 {
    scoring_policy[key].as_u64().unwrap_or(default)
}

pub(crate) fn snapshot_materialization_mode(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["materialization_mode"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_CARRIER_RUNTIME_MATERIALIZATION_MODE)
        .to_string()
}

pub(crate) fn snapshot_carrier_catalog_owner(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["source_of_truth"]["carrier_catalog_owner"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_CARRIER_CATALOG_OWNER)
        .to_string()
}

pub(crate) fn snapshot_dispatch_alias_owner(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["source_of_truth"]["dispatch_alias_owner"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_DISPATCH_ALIAS_OWNER)
        .to_string()
}

pub(crate) fn snapshot_agent_identity(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["agent_model"]["agent_identity"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_AGENT_IDENTITY)
        .to_string()
}

pub(crate) fn snapshot_runtime_role_identity(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["agent_model"]["runtime_role_identity"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_RUNTIME_ROLE_IDENTITY)
        .to_string()
}

pub(crate) fn snapshot_selection_rule(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["worker_strategy"]["selection_policy"]["rule"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            carrier_runtime["agent_model"]["selection_rule"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or(DEFAULT_SELECTION_RULE)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::carrier_runtime_source_of_truth;

    #[test]
    fn source_of_truth_uses_selected_system_for_overlay_dispatch_alias_owner() {
        let source = carrier_runtime_source_of_truth(Some("codex"), true, None);
        assert_eq!(
            source["dispatch_alias_owner"],
            "vida.config.yaml -> host_environment.systems.codex.dispatch_aliases overlay"
        );
        assert_eq!(
            source["carrier_catalog_owner"],
            "vida.config.yaml -> host_environment.systems.codex.carriers"
        );
    }

    #[test]
    fn source_of_truth_uses_neutral_overlay_fallback_without_selected_system() {
        let source = carrier_runtime_source_of_truth(None, true, None);
        assert_eq!(
            source["dispatch_alias_owner"],
            "vida.config.yaml -> configured host-system dispatch_aliases overlay"
        );
        assert_eq!(
            source["carrier_catalog_owner"],
            "vida.config.yaml -> configured host-system carrier surfaces"
        );
    }
}
