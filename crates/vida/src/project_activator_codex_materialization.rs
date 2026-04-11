use super::*;

const CODEX_RUNTIME_LABEL: &str = "Codex";

pub(crate) fn render_codex_template_from_catalog(
    project_root: &Path,
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> Result<(), String> {
    crate::host_runtime_materialization::render_host_runtime_template_from_catalog(
        CODEX_RUNTIME_LABEL,
        project_root,
        &project_root.join(".codex"),
        template_root,
        agent_catalog,
        named_lane_catalog,
    )
}

pub(crate) fn read_codex_agent_catalog(codex_root: &Path) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::read_host_runtime_agent_catalog(codex_root)
}

pub(crate) fn overlay_codex_agent_catalog(config: &serde_yaml::Value) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::overlay_host_runtime_agent_catalog(config)
}

pub(crate) fn host_cli_entry_carrier_catalog(
    entry: Option<&serde_yaml::Value>,
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(entry)
}

pub(crate) fn materialize_codex_dispatch_alias_catalog(
    configured_aliases: &[serde_json::Value],
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::materialize_host_runtime_dispatch_alias_catalog(
        configured_aliases,
        agent_catalog,
    )
}

pub(crate) fn overlay_codex_dispatch_alias_catalog(
    config: &serde_yaml::Value,
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::overlay_host_runtime_dispatch_alias_catalog(
        config,
        agent_catalog,
    )
}

pub(crate) fn codex_dispatch_alias_catalog_for_root(
    config: &serde_yaml::Value,
    root: &Path,
    agent_catalog: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, String> {
    crate::host_runtime_materialization::host_runtime_dispatch_alias_catalog_for_root(
        config,
        root,
        agent_catalog,
    )
}

pub(crate) fn materialize_codex_template_with_catalog_render(
    project_root: &Path,
    cli_system: &str,
    registry_entry: &serde_yaml::Value,
) -> Result<PathBuf, String> {
    let source = super::resolve_host_cli_template_source(cli_system, Some(registry_entry))?;
    let runtime_root =
        super::host_cli_system_runtime_root(registry_entry, cli_system, project_root);
    let copy_tree_target = project_root.join(&runtime_root);
    super::copy_tree_if_missing(&source, &copy_tree_target)?;
    let overlay = super::read_yaml_file_checked(&project_root.join("vida.config.yaml"))
        .unwrap_or(serde_yaml::Value::Null);
    let scoring_policy = serde_json::to_value(
        yaml_lookup(&overlay, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let rendered_catalog_root = project_root.join(super::host_cli_system_runtime_surface(
        registry_entry,
        cli_system,
    ));
    let carrier_roles = {
        let overlay_roles = overlay_codex_agent_catalog(&overlay);
        if overlay_roles.is_empty() {
            read_codex_agent_catalog(&rendered_catalog_root)
        } else {
            overlay_roles
        }
    };
    let carrier_dispatch_aliases =
        codex_dispatch_alias_catalog_for_root(&overlay, project_root, &carrier_roles)?;
    if !carrier_roles.is_empty() {
        render_codex_template_from_catalog(
            project_root,
            &source,
            &carrier_roles,
            &carrier_dispatch_aliases,
        )?;
    }
    super::refresh_worker_strategy(project_root, &carrier_roles, &scoring_policy);
    Ok(runtime_root)
}

pub(crate) fn resolve_codex_agent_catalog_for_rendered_root(
    project_root: &Path,
    overlay: &serde_yaml::Value,
    catalog_entry: Option<&serde_yaml::Value>,
    selected_host_cli_system: &str,
) -> Vec<serde_json::Value> {
    let carrier_catalog_root = project_root.join(super::host_cli_system_runtime_surface(
        catalog_entry.unwrap_or(&serde_yaml::Value::Null),
        selected_host_cli_system,
    ));
    let overlay_rows = overlay_codex_agent_catalog(overlay);
    if overlay_rows.is_empty() {
        read_codex_agent_catalog(carrier_catalog_root.as_path())
    } else {
        overlay_rows
    }
}
