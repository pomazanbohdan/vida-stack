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
