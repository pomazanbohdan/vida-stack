use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project_activator_surface::{
    host_cli_system_enabled, host_cli_system_execution_class, host_cli_system_materialization_mode,
    host_cli_system_runtime_root, host_cli_system_runtime_surface, normalize_host_cli_system,
    resolve_host_cli_template_source, HOST_CLI_PLACEHOLDER, HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE,
    PI_AGENT_PROJECTION_RENDER_MODE,
};

fn pi_projection_path(
    project_root: &Path,
    entry: &serde_yaml::Value,
    key: &str,
    default_path: &str,
) -> PathBuf {
    let configured = crate::yaml_string(crate::yaml_lookup(entry, &["app", key]))
        .unwrap_or_else(|| default_path.to_string());
    project_root.join(configured)
}

fn pi_projection_materialized(
    project_root: &Path,
    runtime_surface: &str,
    entry: &serde_yaml::Value,
) -> bool {
    let base = runtime_surface.trim_end_matches('/');
    let settings_path = pi_projection_path(
        project_root,
        entry,
        "settings_path",
        &format!("{base}/settings.json"),
    );
    let agents_dir =
        pi_projection_path(project_root, entry, "agents_dir", &format!("{base}/agents"));
    settings_path.is_file()
        && agents_dir.is_dir()
        && agents_dir.read_dir().ok().is_some_and(|mut entries| {
            entries.any(|entry| {
                entry.is_ok_and(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("md")
                })
            })
        })
}

pub(crate) struct ProjectActivatorHostCliSummary {
    pub(crate) supported_host_cli_systems: Vec<String>,
    pub(crate) host_cli_suggested_system: String,
    pub(crate) host_cli_supported_list: String,
    pub(crate) selected_host_cli_system: Option<String>,
    pub(crate) host_cli_selection_required: bool,
    pub(crate) host_cli_runtime_template_root: String,
    pub(crate) host_cli_execution_class: Option<String>,
    pub(crate) host_cli_template_materialized: bool,
    pub(crate) host_cli_materialization_required: bool,
    pub(crate) host_cli_template_source_root: Option<PathBuf>,
    pub(crate) default_host_agent_templates: Vec<String>,
    pub(crate) default_agent_topology: Vec<String>,
    pub(crate) carrier_tier_rates: serde_json::Map<String, serde_json::Value>,
    pub(crate) concrete_tier_rates: serde_json::Map<String, serde_json::Value>,
}

pub(crate) fn build_project_activator_host_cli_summary(
    project_root: &Path,
    project_overlay: Option<&serde_yaml::Value>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
) -> ProjectActivatorHostCliSummary {
    let mut supported_host_cli_systems = host_cli_system_registry
        .iter()
        .filter(|(_, entry)| host_cli_system_enabled(entry))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    supported_host_cli_systems.sort();
    let host_cli_suggested_system = supported_host_cli_systems
        .first()
        .cloned()
        .unwrap_or_default();
    let host_cli_supported_list = if supported_host_cli_systems.is_empty() {
        String::new()
    } else {
        supported_host_cli_systems.join(", ")
    };
    let selected_host_cli_system = project_overlay
        .and_then(|config| crate::yaml_lookup(config, &["host_environment", "cli_system"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != HOST_CLI_PLACEHOLDER)
        .and_then(normalize_host_cli_system);
    let host_cli_system_entry = selected_host_cli_system
        .as_deref()
        .and_then(|system| host_cli_system_registry.get(system));
    let host_cli_selection_required = selected_host_cli_system.is_none()
        || host_cli_system_entry.is_none()
        || !host_cli_system_entry
            .map(host_cli_system_enabled)
            .unwrap_or(false);
    let host_cli_runtime_root = selected_host_cli_system.as_deref().and_then(|system| {
        host_cli_system_entry.map(|entry| host_cli_system_runtime_root(entry, system, project_root))
    });
    let host_cli_runtime_template_root = host_cli_runtime_root
        .as_ref()
        .and_then(|_| {
            selected_host_cli_system.as_deref().and_then(|system| {
                host_cli_system_entry.map(|entry| host_cli_system_runtime_surface(entry, system))
            })
        })
        .or_else(|| {
            supported_host_cli_systems.first().and_then(|system| {
                host_cli_system_registry
                    .get(system)
                    .map(|entry| host_cli_system_runtime_surface(entry, system))
            })
        })
        .unwrap_or_default();
    let host_cli_materialization_mode = selected_host_cli_system.as_deref().and_then(|system| {
        host_cli_system_entry.map(|entry| host_cli_system_materialization_mode(entry, system))
    });
    let host_cli_execution_class = selected_host_cli_system.as_deref().and_then(|system| {
        host_cli_system_entry.map(|entry| host_cli_system_execution_class(entry, system))
    });
    let host_cli_template_materialized = match (
        host_cli_runtime_root.as_deref(),
        host_cli_materialization_mode.as_deref(),
    ) {
        (Some(root), Some(HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE)) => {
            root.join("config.toml").is_file() && root.join("agents").is_dir()
        }
        (Some(_root), Some(PI_AGENT_PROJECTION_RENDER_MODE)) => {
            host_cli_system_entry.is_some_and(|entry| {
                pi_projection_materialized(project_root, &host_cli_runtime_template_root, entry)
            })
        }
        (Some(root), Some("copy_tree_only")) => root.exists(),
        _ => false,
    };
    let host_cli_materialization_required =
        !host_cli_selection_required && !host_cli_template_materialized;
    let host_cli_template_source_root = selected_host_cli_system
        .as_deref()
        .and_then(|system| {
            host_cli_system_entry.and_then(|entry| {
                if host_cli_system_materialization_mode(entry, system)
                    == PI_AGENT_PROJECTION_RENDER_MODE
                {
                    None
                } else {
                    resolve_host_cli_template_source(system, Some(entry)).ok()
                }
            })
        })
        .or_else(|| {
            supported_host_cli_systems.first().and_then(|system| {
                host_cli_system_registry.get(system).and_then(|entry| {
                    if host_cli_system_materialization_mode(entry, system)
                        == PI_AGENT_PROJECTION_RENDER_MODE
                    {
                        None
                    } else {
                        resolve_host_cli_template_source(system, Some(entry)).ok()
                    }
                })
            })
        });
    let catalog_system = selected_host_cli_system
        .clone()
        .unwrap_or_else(|| host_cli_suggested_system.clone());
    let catalog_entry = selected_host_cli_system
        .as_deref()
        .and_then(|system| host_cli_system_registry.get(system))
        .or_else(|| host_cli_system_registry.get(&catalog_system));
    let host_cli_agent_catalog =
        crate::project_activator_surface::host_cli_entry_carrier_catalog(catalog_entry);
    let default_host_agent_templates = host_cli_agent_catalog
        .iter()
        .filter_map(|row| row["role_id"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let default_agent_topology = host_cli_agent_catalog
        .iter()
        .filter_map(|row| row["role_id"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let mut carrier_tier_rates = serde_json::Map::new();
    let mut concrete_tier_rates = serde_json::Map::new();
    for row in &host_cli_agent_catalog {
        if let (Some(tier), Some(rate)) = (
            crate::carrier_runtime_catalog::canonical_carrier_tier(row).as_deref(),
            row["rate"].as_u64(),
        ) {
            carrier_tier_rates.insert(
                tier.to_string(),
                serde_json::Value::Number(serde_json::Number::from(rate)),
            );
        }
        if let (Some(tier), Some(rate)) = (
            crate::carrier_runtime_catalog::concrete_carrier_tier(row).as_deref(),
            row["rate"].as_u64(),
        ) {
            concrete_tier_rates.insert(
                tier.to_string(),
                serde_json::Value::Number(serde_json::Number::from(rate)),
            );
        }
    }

    ProjectActivatorHostCliSummary {
        supported_host_cli_systems,
        host_cli_suggested_system,
        host_cli_supported_list,
        selected_host_cli_system,
        host_cli_selection_required,
        host_cli_runtime_template_root,
        host_cli_execution_class,
        host_cli_template_materialized,
        host_cli_materialization_required,
        host_cli_template_source_root,
        default_host_agent_templates,
        default_agent_topology,
        carrier_tier_rates,
        concrete_tier_rates,
    }
}

#[cfg(test)]
mod tests {
    use super::build_project_activator_host_cli_summary;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn host_cli_summary_uses_configured_carrier_catalog_as_template_source_of_truth() {
        let tempdir = std::env::temp_dir().join(format!(
            "vida-host-cli-summary-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&tempdir).expect("temp dir should initialize");
        let overlay: serde_yaml::Value = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      materialization_mode: codex_toml_catalog_render
      runtime_root: .codex
      template_root: .codex
      carriers:
        implementer-fast:
          tier: fast
          rate: 2
          runtime_roles: [worker]
          task_classes: [implementation]
        reviewer-proof:
          tier: proof
          rate: 8
          runtime_roles: [verifier]
          task_classes: [verification]
"#,
        )
        .expect("overlay yaml should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&overlay));

        let summary = build_project_activator_host_cli_summary(&tempdir, Some(&overlay), &registry);

        assert_eq!(
            summary.default_host_agent_templates,
            vec!["implementer-fast".to_string(), "reviewer-proof".to_string()]
        );
        assert_eq!(
            summary.default_agent_topology,
            vec!["implementer-fast".to_string(), "reviewer-proof".to_string()]
        );

        let _ = fs::remove_dir_all(tempdir);
    }
}
