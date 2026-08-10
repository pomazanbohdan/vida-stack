use std::path::Path;

use serde_yaml;

pub(crate) fn selected_host_cli_system_entry(
    overlay: &serde_yaml::Value,
) -> (String, Option<serde_yaml::Value>) {
    let registry =
        super::project_activator_surface::host_cli_system_registry_from_config(Some(overlay));
    let candidate = super::yaml_lookup(overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "__HOST_CLI_SYSTEM__")
        .and_then(super::project_activator_surface::normalize_host_cli_system);
    let normalized = candidate.unwrap_or_else(|| {
        let mut supported = registry
            .iter()
            .filter(|(_, entry)| super::yaml_bool(super::yaml_lookup(entry, &["enabled"]), true))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        supported.sort();
        supported
            .into_iter()
            .next()
            .or_else(|| {
                let mut fallback = registry.keys().cloned().collect::<Vec<_>>();
                fallback.sort();
                fallback.into_iter().next()
            })
            .unwrap_or_default()
    });
    let entry = registry.get(&normalized).cloned();
    (normalized, entry)
}

pub(crate) fn runtime_surface_for_selected_system(
    system: &str,
    entry: Option<&serde_yaml::Value>,
) -> String {
    entry
        .and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])))
        .unwrap_or_else(|| format!(".{system}"))
}

pub(crate) fn runtime_root_for_selected_system(
    project_root: &Path,
    system: &str,
    entry: Option<&serde_yaml::Value>,
) -> String {
    let configured =
        entry.and_then(|value| super::yaml_string(super::yaml_lookup(value, &["runtime_root"])));
    let relative = configured.unwrap_or_else(|| format!(".{system}"));
    project_root.join(relative).display().to_string()
}

pub(crate) fn default_host_cli_materialization_mode(
    entry: Option<&serde_yaml::Value>,
    _system: &str,
) -> String {
    let has_carrier_catalog = entry
        .and_then(|value| super::yaml_lookup(value, &["carriers"]))
        .and_then(serde_yaml::Value::as_mapping)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if has_carrier_catalog {
        crate::project_activator_surface::HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE.to_string()
    } else {
        "copy_tree_only".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_system_projects_runtime_root_and_catalog_mode() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            "host_environment:\n  cli_system: Codex\n  systems:\n    codex:\n      runtime_root: .vida/codex\n      carriers:\n        worker: {}\n",
        )
        .expect("host cli overlay should parse");

        let (system, entry) = selected_host_cli_system_entry(&overlay);
        let entry = entry.expect("selected system should have a registry entry");
        assert_eq!(system, "codex");
        assert_eq!(
            runtime_surface_for_selected_system(&system, Some(&entry)),
            ".vida/codex"
        );
        assert_eq!(
            runtime_root_for_selected_system(Path::new("project-root"), &system, Some(&entry)),
            Path::new("project-root")
                .join(".vida/codex")
                .display()
                .to_string()
        );
        assert_eq!(
            default_host_cli_materialization_mode(Some(&entry), &system),
            "codex_toml_catalog_render"
        );
    }

    #[test]
    fn selected_system_falls_back_to_first_enabled_sorted_entry() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            "host_environment:\n  systems:\n    zeta:\n      enabled: false\n    alpha:\n      enabled: true\n",
        )
        .expect("host cli overlay should parse");

        let (system, entry) = selected_host_cli_system_entry(&overlay);
        assert_eq!(system, "alpha");
        assert!(entry.is_some());
        assert_eq!(
            runtime_surface_for_selected_system(&system, entry.as_ref()),
            ".alpha"
        );
    }

    #[test]
    fn placeholder_selected_system_falls_back_to_enabled_entry() {
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            "host_environment:\n  cli_system: __HOST_CLI_SYSTEM__\n  systems:\n    zeta:\n      enabled: false\n    alpha:\n      enabled: true\n      runtime_root: .vida/alpha\n",
        )
        .expect("host cli overlay should parse");

        let (system, entry) = selected_host_cli_system_entry(&overlay);

        assert_eq!(system, "alpha");
        assert_eq!(
            runtime_surface_for_selected_system(&system, entry.as_ref()),
            ".vida/alpha"
        );
    }
}
