pub(crate) fn resolved_carrier_roles(
    config: &serde_yaml::Value,
    catalog_root: &std::path::Path,
) -> Vec<serde_json::Value> {
    let overlay_roles = super::project_activator_surface::overlay_host_cli_agent_catalog(config);
    if overlay_roles.is_empty() {
        super::project_activator_surface::read_host_cli_agent_catalog(catalog_root)
    } else {
        overlay_roles
    }
}

pub(crate) fn carrier_role_validation_errors(roles: &[serde_json::Value]) -> Vec<String> {
    roles
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            let mut missing = Vec::new();
            if row["rate"].as_u64().unwrap_or(0) == 0 {
                missing.push("rate");
            }
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "carrier role `{role_id}` is missing required runtime metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect()
}

pub(crate) fn materialized_dispatch_aliases(
    config: &serde_yaml::Value,
    dispatch_alias_rows: &[serde_json::Value],
    carrier_roles: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    if dispatch_alias_rows.is_empty() {
        super::project_activator_surface::overlay_host_cli_dispatch_alias_catalog(
            config,
            carrier_roles,
        )
    } else {
        super::project_activator_surface::materialize_host_cli_dispatch_alias_catalog(
            dispatch_alias_rows,
            carrier_roles,
        )
    }
}

pub(crate) fn carrier_dispatch_alias_validation_errors(
    dispatch_aliases: &[serde_json::Value],
) -> Vec<String> {
    dispatch_aliases
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            let mut missing = Vec::new();
            if row["template_role_id"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("carrier_tier");
            }
            if row["default_runtime_role"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("runtime_role");
            }
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if row["developer_instructions"]
                .as_str()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                missing.push("developer_instructions");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "carrier dispatch alias `{role_id}` is missing required runtime metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{carrier_dispatch_alias_validation_errors, carrier_role_validation_errors};

    #[test]
    fn role_validation_errors_are_carrier_neutral() {
        let errors = carrier_role_validation_errors(&[serde_json::json!({
            "role_id": "junior"
        })]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("carrier role `junior`"));
        assert!(!errors[0].contains("codex"));
    }

    #[test]
    fn dispatch_alias_validation_errors_are_carrier_neutral() {
        let errors = carrier_dispatch_alias_validation_errors(&[serde_json::json!({
            "role_id": "development_implementer"
        })]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("carrier dispatch alias `development_implementer`"));
        assert!(!errors[0].contains("codex"));
    }
}
