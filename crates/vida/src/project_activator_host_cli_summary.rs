use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project_activator_surface::{
    host_cli_system_enabled, host_cli_system_execution_class, host_cli_system_materialization_mode,
    host_cli_system_runtime_root, host_cli_system_runtime_surface, list_host_cli_agent_templates,
    normalize_host_cli_system, resolve_host_cli_template_source, HOST_CLI_PLACEHOLDER,
};

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
        (Some(root), Some("codex_toml_catalog_render")) => {
            root.join("config.toml").is_file() && root.join("agents").is_dir()
        }
        (Some(root), Some("copy_tree_only")) => root.exists(),
        _ => false,
    };
    let host_cli_materialization_required =
        !host_cli_selection_required && !host_cli_template_materialized;
    let host_cli_template_source_root = selected_host_cli_system
        .as_deref()
        .and_then(|system| resolve_host_cli_template_source(system, host_cli_system_entry).ok())
        .or_else(|| {
            supported_host_cli_systems.first().and_then(|system| {
                host_cli_system_registry
                    .get(system)
                    .and_then(|entry| resolve_host_cli_template_source(system, Some(entry)).ok())
            })
        });
    let catalog_system = selected_host_cli_system
        .clone()
        .unwrap_or_else(|| host_cli_suggested_system.clone());
    let catalog_entry = selected_host_cli_system
        .as_deref()
        .and_then(|system| host_cli_system_registry.get(system))
        .or_else(|| host_cli_system_registry.get(&catalog_system));
    let default_host_agent_templates = host_cli_template_source_root
        .as_deref()
        .map(list_host_cli_agent_templates)
        .unwrap_or_default();
    let host_cli_agent_catalog =
        crate::project_activator_surface::host_cli_entry_carrier_catalog(catalog_entry);
    let default_agent_topology = host_cli_agent_catalog
        .iter()
        .filter_map(|row| row["role_id"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let mut carrier_tier_rates = serde_json::Map::new();
    for row in &host_cli_agent_catalog {
        if let (Some(tier), Some(rate)) = (row["tier"].as_str(), row["rate"].as_u64()) {
            carrier_tier_rates.insert(
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
    }
}
