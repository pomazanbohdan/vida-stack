use serde_yaml;

use crate::status_surface_host_cli_system::default_host_cli_materialization_mode;

pub(crate) fn host_cli_system_entry_summary(
    entry: Option<&serde_yaml::Value>,
    system: &str,
) -> serde_json::Value {
    let enabled = entry
        .map(|value| super::yaml_bool(super::yaml_lookup(value, &["enabled"]), true))
        .unwrap_or(true);
    let template_root = entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["template_root"])))
        .unwrap_or_else(|| format!(".{system}"));
    let runtime_root = entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])))
        .unwrap_or_else(|| format!(".{system}"));
    let materialization_mode = entry
        .and_then(|value| super::yaml_lookup(value, &["materialization_mode"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| default_host_cli_materialization_mode(entry, system));
    let execution_class = entry
        .map(|value| {
            super::project_activator_surface::host_cli_system_execution_class(value, system)
        })
        .unwrap_or_else(|| "unknown".to_string());
    let carriers = entry
        .and_then(|value| super::yaml_lookup(value, &["carriers"]))
        .filter(|value| !value.is_null())
        .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})))
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "enabled": enabled,
        "execution_class": execution_class,
        "materialization_mode": materialization_mode,
        "template_root": template_root,
        "runtime_root": runtime_root,
        "carriers": carriers,
    })
}

pub(crate) fn host_cli_system_carrier_summary(
    entry: Option<&serde_yaml::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut agents = serde_json::Map::new();
    let Some(serde_yaml::Value::Mapping(carriers)) =
        entry.and_then(|value| super::yaml_lookup(value, &["carriers"]))
    else {
        return agents;
    };
    for (carrier_id, carrier_value) in carriers {
        let Some(carrier_id) = carrier_id
            .as_str()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        let carrier = serde_json::to_value(carrier_value).unwrap_or_else(|_| serde_json::json!({}));
        agents.insert(
            carrier_id.to_string(),
            serde_json::json!({
                "tier": carrier["tier"].clone(),
                "rate": carrier["rate"].clone(),
                "reasoning_band": carrier["reasoning_band"].clone(),
                "default_runtime_role": carrier["default_runtime_role"].clone(),
                "runtime_roles": carrier["runtime_roles"].clone(),
                "task_classes": carrier["task_classes"].clone(),
                "feedback_count": 0,
                "last_feedback_at": serde_json::Value::Null,
                "last_feedback_outcome": serde_json::Value::Null,
                "effective_score": serde_json::Value::Null,
                "lifecycle_state": serde_json::Value::Null,
            }),
        );
    }
    agents
}

#[cfg(test)]
mod tests {
    use super::{host_cli_system_carrier_summary, host_cli_system_entry_summary};

    #[test]
    fn host_cli_summary_defaults_missing_systems_to_safe_values() {
        let summary = host_cli_system_entry_summary(None, "codex");

        assert_eq!(summary["enabled"], true);
        assert_eq!(summary["execution_class"], "unknown");
        assert_eq!(summary["materialization_mode"], "copy_tree_only");
        assert_eq!(summary["template_root"], ".codex");
        assert_eq!(summary["runtime_root"], ".codex");
        assert_eq!(summary["carriers"], serde_json::json!({}));
        assert!(host_cli_system_carrier_summary(None).is_empty());
    }

    #[test]
    fn host_cli_summary_preserves_carrier_contract_and_skips_blank_ids() {
        let entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
enabled: false
template_root: templates/codex
runtime_root: runtime/codex
materialization_mode: " Explicit_Render "
execution_class: Internal
carriers:
  senior:
    tier: 3
    rate: 2.5
    reasoning_band: max
    default_runtime_role: verifier
    runtime_roles: [verifier, prover]
    task_classes: [implementation]
  " ":
    tier: 0
"#,
        )
        .expect("carrier fixture parses");

        let summary = host_cli_system_entry_summary(Some(&entry), "codex");
        assert_eq!(summary["enabled"], false);
        assert_eq!(summary["execution_class"], "internal");
        assert_eq!(summary["materialization_mode"], "explicit_render");
        assert_eq!(summary["template_root"], "templates/codex");
        assert_eq!(summary["runtime_root"], "runtime/codex");

        let carriers = host_cli_system_carrier_summary(Some(&entry));
        assert_eq!(carriers.len(), 1);
        assert_eq!(carriers["senior"]["tier"], 3);
        assert_eq!(carriers["senior"]["rate"], 2.5);
        assert_eq!(
            carriers["senior"]["runtime_roles"],
            serde_json::json!(["verifier", "prover"])
        );
        assert_eq!(carriers["senior"]["feedback_count"], 0);
        assert!(carriers["senior"]["last_feedback_at"].is_null());
    }

    #[test]
    fn host_cli_summary_uses_catalog_mode_when_materialization_is_blank() {
        let entry: serde_yaml::Value = serde_yaml::from_str(
            r#"
materialization_mode: "   "
carriers:
  worker:
    tier: 1
"#,
        )
        .expect("carrier fixture parses");

        let summary = host_cli_system_entry_summary(Some(&entry), "codex");

        assert_eq!(summary["materialization_mode"], "codex_toml_catalog_render");
        assert_eq!(summary["carriers"]["worker"]["tier"], 1);
    }
}
