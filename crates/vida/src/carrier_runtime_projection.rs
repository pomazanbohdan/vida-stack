use std::{collections::HashMap, path::Path};

use crate::{read_simple_toml_sections, registry_rows_by_key};

pub(crate) struct CarrierRuntimeProjection {
    pub(crate) carrier_runtime: serde_json::Value,
    pub(crate) validation_errors: Vec<String>,
}

fn selected_runtime_root(
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
) -> std::path::PathBuf {
    if let Some(system) = selected_host_cli_system {
        if let Some(entry) = host_cli_system_registry.get(system) {
            return crate::project_activator_surface::host_cli_system_runtime_root(
                entry, system, root,
            );
        }
    }

    let mut enabled_entries = host_cli_system_registry
        .iter()
        .filter(|(_, entry)| crate::project_activator_surface::host_cli_system_enabled(entry))
        .collect::<Vec<_>>();
    enabled_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    if let Some((system, entry)) = enabled_entries.first() {
        return crate::project_activator_surface::host_cli_system_runtime_root(entry, system, root);
    }

    let mut entries = host_cli_system_registry.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    if let Some((system, entry)) = entries.first() {
        return crate::project_activator_surface::host_cli_system_runtime_root(entry, system, root);
    }

    root.join(".host-runtime")
}

pub(crate) fn build_carrier_runtime_projection(
    config: &serde_yaml::Value,
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
    dispatch_aliases_registry: &serde_yaml::Value,
    dispatch_aliases_path: Option<&str>,
) -> CarrierRuntimeProjection {
    let runtime_root =
        selected_runtime_root(root, selected_host_cli_system, host_cli_system_registry);
    let runtime_config = read_simple_toml_sections(&runtime_root.join("config.toml"));
    let carrier_roles =
        crate::carrier_runtime_catalog::resolved_carrier_roles(config, &runtime_root);
    let dispatch_alias_rows = registry_rows_by_key(
        dispatch_aliases_registry,
        "dispatch_aliases",
        "alias_id",
        &[],
    );
    let carrier_dispatch_aliases = crate::carrier_runtime_catalog::materialized_dispatch_aliases(
        config,
        &dispatch_alias_rows,
        &carrier_roles,
    );
    let scoring_policy = serde_json::to_value(
        crate::yaml_lookup(config, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let worker_strategy = crate::carrier_runtime_strategy::resolved_worker_strategy(
        root,
        &carrier_roles,
        &scoring_policy,
    );
    let pricing_policy = crate::carrier_runtime_strategy::resolved_pricing_policy(
        config,
        &carrier_roles,
        &worker_strategy,
    );

    let mut validation_errors =
        crate::carrier_runtime_catalog::carrier_role_validation_errors(&carrier_roles);
    validation_errors.extend(
        crate::carrier_runtime_catalog::carrier_dispatch_alias_validation_errors(
            &carrier_dispatch_aliases,
        ),
    );

    CarrierRuntimeProjection {
        carrier_runtime: serde_json::json!({
            "enabled": runtime_config
                .get("features")
                .and_then(|section| section.get("multi_agent"))
                .map(|value| value == "true")
                .unwrap_or(false),
            "max_threads": runtime_config
                .get("agents")
                .and_then(|section| section.get("max_threads"))
                .cloned()
                .unwrap_or_default(),
            "max_depth": runtime_config
                .get("agents")
                .and_then(|section| section.get("max_depth"))
                .cloned()
                .unwrap_or_default(),
            "materialization_mode": crate::carrier_runtime_metadata::carrier_runtime_materialization_mode(
                selected_host_cli_system,
                host_cli_system_registry,
            ),
            "roles": carrier_roles,
            "dispatch_aliases": carrier_dispatch_aliases,
            "source_of_truth": crate::carrier_runtime_metadata::carrier_runtime_source_of_truth(
                selected_host_cli_system,
                dispatch_alias_rows.is_empty(),
                dispatch_aliases_path,
            ),
            "agent_model": crate::carrier_runtime_metadata::carrier_runtime_agent_model(
                config,
                &worker_strategy,
            ),
            "worker_strategy": worker_strategy,
            "pricing_policy": pricing_policy,
        }),
        validation_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::selected_runtime_root;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn selected_runtime_root_prefers_explicit_system_from_registry() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    codex:
      enabled: true
      runtime_root: .codex
    hermes:
      enabled: true
      runtime_root: .hermes
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(&config));

        let root = selected_runtime_root(Path::new("/tmp/project"), Some("hermes"), &registry);

        assert_eq!(root, Path::new("/tmp/project/.hermes"));
    }

    #[test]
    fn selected_runtime_root_uses_first_enabled_system_without_hardcoded_codex_default() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    hermes:
      enabled: true
      runtime_root: .hermes
    opencode:
      enabled: false
      runtime_root: .opencode
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(&config));

        let root = selected_runtime_root(Path::new("/tmp/project"), None, &registry);

        assert_eq!(root, Path::new("/tmp/project/.hermes"));
    }

    #[test]
    fn selected_runtime_root_has_stable_neutral_fallback_without_registry() {
        let root = selected_runtime_root(Path::new("/tmp/project"), None, &HashMap::new());

        assert_eq!(root, Path::new("/tmp/project/.host-runtime"));
    }
}
