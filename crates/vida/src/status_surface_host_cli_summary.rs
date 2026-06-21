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
