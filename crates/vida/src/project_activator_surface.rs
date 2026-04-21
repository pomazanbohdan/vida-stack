use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::activation_status::canonical_activation_status;
#[path = "project_activator_host_cli_materialization.rs"]
mod project_activator_host_cli_materialization;
use super::*;
pub(crate) use project_activator_host_cli_materialization::{
    host_cli_entry_carrier_catalog, materialize_host_cli_dispatch_alias_catalog,
    overlay_host_cli_agent_catalog, overlay_host_cli_dispatch_alias_catalog,
    read_host_cli_agent_catalog,
};

#[derive(Debug, Clone)]
pub(crate) struct ProjectActivationAnswers {
    pub(crate) project_id: String,
    pub(crate) project_title: String,
    pub(crate) user_communication_language: String,
    pub(crate) reasoning_language: String,
    pub(crate) documentation_language: String,
    pub(crate) todo_protocol_language: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectActivationStatusTruth {
    pub(crate) status: String,
    pub(crate) activation_pending: bool,
    pub(crate) next_steps: Vec<String>,
}

pub(crate) const HOST_CLI_PLACEHOLDER: &str = "__HOST_CLI_SYSTEM__";
pub(crate) const HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE: &str = "codex_toml_catalog_render";

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn set_yaml_scalar_in_top_level_section(
    contents: &str,
    section: &str,
    key: &str,
    value: &str,
) -> String {
    let rendered = yaml_scalar(value);
    let mut lines: Vec<String> = contents.lines().map(ToString::to_string).collect();
    let section_header = format!("{section}:");
    let key_prefix = format!("{key}:");
    let mut section_index = None;
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == section_header {
            section_index = Some(index);
            break;
        }
    }

    if let Some(section_index) = section_index {
        let mut section_end = lines.len();
        for (index, line) in lines.iter().enumerate().skip(section_index + 1) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !line.starts_with(' ') {
                section_end = index;
                break;
            }
        }
        for (index, line) in lines.iter().enumerate().skip(section_index + 1) {
            if index >= section_end {
                break;
            }
            if line.trim_start().starts_with(&key_prefix) && line.starts_with("  ") {
                lines[index] = format!("  {key}: {rendered}");
                return format!("{}\n", lines.join("\n"));
            }
        }
        lines.insert(section_end, format!("  {key}: {rendered}"));
        return format!("{}\n", lines.join("\n"));
    }

    if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.push(section_header);
    lines.push(format!("  {key}: {rendered}"));
    format!("{}\n", lines.join("\n"))
}

fn set_yaml_bool_in_top_level_section(
    contents: &str,
    section: &str,
    key: &str,
    value: bool,
) -> String {
    set_yaml_scalar_in_top_level_section(
        contents,
        section,
        key,
        if value { "true" } else { "false" },
    )
}

pub(crate) fn render_project_sidecar(project_title: &str) -> String {
    format!(
        "# Project Docs Map\n\n\
Repository: `{project_title}`\n\n\
1. Current project root map:\n\
   - `{DEFAULT_PROJECT_ROOT_MAP}`\n\
2. Project product index:\n\
   - `{DEFAULT_PROJECT_PRODUCT_INDEX}`\n\
3. Project product spec/readiness guide:\n\
   - `{DEFAULT_PROJECT_PRODUCT_SPEC_README}`\n\
4. Local feature/change design template:\n\
   - `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`\n\
5. Project process index:\n\
   - `{DEFAULT_PROJECT_PROCESS_README}`\n\
6. Project documentation tooling map:\n\
   - `{DEFAULT_PROJECT_DOC_TOOLING_DOC}`\n\
7. Project agent-system baseline:\n\
   - `{DEFAULT_PROJECT_AGENT_SYSTEM_DOC}`\n\
8. Project Codex agent guide:\n\
   - `{DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC}`\n\
9. Project research index:\n\
   - `{DEFAULT_PROJECT_RESEARCH_README}`\n\n\
Working rule:\n\
1. Use this sidecar as the project docs map after framework bootstrap.\n\
2. For bounded feature/change work that asks for research, specification, planning, and implementation, start with the local feature-design template and the documentation tooling path before code execution.\n\
3. While project activation is pending, prefer `vida project-activator` for activation mutations and `vida docflow` for documentation/readiness inspection.\n"
    )
}

fn detect_project_shape(project_root: &Path) -> &'static str {
    let bootstrap_markers = [
        project_root.join("AGENTS.md"),
        project_root.join("AGENTS.sidecar.md"),
        project_root.join("vida.config.yaml"),
        project_root.join(".vida/config"),
        project_root.join(".vida/db"),
        project_root.join(".vida/cache"),
        project_root.join(".vida/framework"),
        project_root.join(".vida/project"),
        project_root.join(".vida/project/agent-extensions"),
        project_root.join(".vida/receipts"),
        project_root.join(".vida/runtime"),
        project_root.join(".vida/scratchpad"),
    ];
    if bootstrap_markers.iter().all(|path| path.exists()) {
        return "bootstrapped";
    }

    let project_markers = [
        project_root.join("docs"),
        project_root.join("src"),
        project_root.join("README.md"),
        project_root.join("Cargo.toml"),
        project_root.join("package.json"),
        project_root.join("pubspec.yaml"),
    ];
    if project_markers.iter().any(|path| path.exists()) {
        "partial"
    } else {
        "empty"
    }
}

pub(crate) fn normalize_host_cli_system(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

pub(crate) fn host_cli_display_name(system: &str) -> String {
    if system.eq_ignore_ascii_case("codex") {
        "Codex".to_string()
    } else {
        system.to_string()
    }
}

fn host_cli_system_registry(config: &serde_yaml::Value) -> HashMap<String, serde_yaml::Value> {
    let mut registry = HashMap::new();
    let systems = yaml_lookup(config, &["host_environment", "systems"]);
    if let Some(serde_yaml::Value::Mapping(mapping)) = systems {
        for (key, value) in mapping {
            if let serde_yaml::Value::String(text) = key {
                let normalized = text.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    registry.insert(normalized, value.clone());
                }
            }
        }
    }
    registry
}

pub(crate) fn host_cli_system_registry_with_fallback(
    config: Option<&serde_yaml::Value>,
) -> HashMap<String, serde_yaml::Value> {
    config.map(host_cli_system_registry).unwrap_or_default()
}

fn load_host_cli_system_registry_from_root(root: &Path) -> HashMap<String, serde_yaml::Value> {
    let config_path = root.join("vida.config.yaml");
    read_yaml_file_checked(&config_path)
        .ok()
        .as_ref()
        .map(host_cli_system_registry)
        .unwrap_or_default()
}

pub(crate) fn host_cli_system_enabled(entry: &serde_yaml::Value) -> bool {
    yaml_bool(yaml_lookup(entry, &["enabled"]), true)
}

fn host_cli_system_template_root(entry: &serde_yaml::Value) -> Option<String> {
    yaml_string(yaml_lookup(entry, &["template_root"]))
}

pub(crate) fn host_cli_system_runtime_root(
    entry: &serde_yaml::Value,
    _system: &str,
    root: &Path,
) -> PathBuf {
    resolve_overlay_path(
        root,
        &yaml_string(yaml_lookup(entry, &["runtime_root"])).unwrap_or_default(),
    )
}

pub(crate) fn host_cli_system_runtime_surface(entry: &serde_yaml::Value, _system: &str) -> String {
    yaml_string(yaml_lookup(entry, &["runtime_root"])).unwrap_or_default()
}

pub(crate) fn host_cli_system_materialization_mode(
    entry: &serde_yaml::Value,
    _system: &str,
) -> String {
    yaml_string(yaml_lookup(entry, &["materialization_mode"]))
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub(crate) fn host_cli_system_execution_class(entry: &serde_yaml::Value, _system: &str) -> String {
    yaml_string(yaml_lookup(entry, &["execution_class"]))
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

fn host_cli_system_agent_only_defaults_enabled(entry: &serde_yaml::Value, system: &str) -> bool {
    host_cli_system_execution_class(entry, system) == "internal"
}

pub(crate) fn inferred_project_id_candidate(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify_project_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "vida-project".to_string())
}

pub(crate) fn resolve_overlay_path(root: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn registry_sidecar_path(registry_path: &Path) -> PathBuf {
    let Some(file_name) = registry_path.file_name().and_then(|value| value.to_str()) else {
        return registry_path.with_extension("sidecar");
    };
    if let Some(stripped) = file_name.strip_suffix(".yaml") {
        return registry_path.with_file_name(format!("{stripped}.sidecar.yaml"));
    }
    registry_path.with_file_name(format!("{file_name}.sidecar"))
}

pub(crate) fn collect_missing_registry_ids(
    existing_ids: &std::collections::HashSet<String>,
    enabled_ids: &[String],
) -> Vec<String> {
    enabled_ids
        .iter()
        .filter(|id| !existing_ids.contains(*id))
        .cloned()
        .collect()
}

fn yaml_key_matches(value: &serde_yaml::Value, expected: &str) -> bool {
    matches!(value, serde_yaml::Value::String(text) if text == expected)
}

fn merge_registry_projection(
    base_registry: &serde_yaml::Value,
    sidecar_registry: &serde_yaml::Value,
    key: &str,
    id_field: &str,
    registry_label: &str,
) -> Result<serde_yaml::Value, String> {
    let mut merged_mapping = match base_registry {
        serde_yaml::Value::Mapping(mapping) => mapping.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    if let serde_yaml::Value::Mapping(sidecar_mapping) = sidecar_registry {
        for (entry_key, entry_value) in sidecar_mapping {
            if yaml_key_matches(entry_key, key) {
                continue;
            }
            merged_mapping.insert(entry_key.clone(), entry_value.clone());
        }
    }

    let mut merged_rows = Vec::new();
    let mut index_by_id = std::collections::HashMap::new();
    for (source_name, registry) in [("base", base_registry), ("sidecar", sidecar_registry)] {
        let Some(serde_yaml::Value::Sequence(rows)) = super::yaml_lookup(registry, &[key]) else {
            continue;
        };
        for row in rows {
            let row_id = super::yaml_string(super::yaml_lookup(row, &[id_field])).ok_or_else(|| {
                format!(
                    "agent extension {registry_label} {source_name} projection contains a row without `{id_field}`"
                )
            })?;
            if let Some(index) = index_by_id.get(&row_id).copied() {
                merged_rows[index] = row.clone();
            } else {
                index_by_id.insert(row_id, merged_rows.len());
                merged_rows.push(row.clone());
            }
        }
    }

    merged_mapping.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::Sequence(merged_rows),
    );
    if !merged_mapping.contains_key(serde_yaml::Value::String("version".to_string())) {
        merged_mapping.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(1)),
        );
    }
    Ok(serde_yaml::Value::Mapping(merged_mapping))
}

pub(crate) fn load_registry_projection(
    root: &Path,
    configured_path: Option<&str>,
    key: &str,
    id_field: &str,
    registry_label: &str,
    require_registry_files: bool,
) -> Result<serde_yaml::Value, String> {
    let Some(path) = configured_path else {
        return Ok(serde_yaml::Value::Null);
    };
    let registry_path = resolve_overlay_path(root, path);
    let sidecar_path = registry_sidecar_path(&registry_path);
    let base_registry = match read_yaml_file_checked(&registry_path) {
        Ok(value) => value,
        Err(error) => {
            if require_registry_files || sidecar_path.exists() {
                return Err(error);
            }
            return Ok(serde_yaml::Value::Null);
        }
    };
    let sidecar_registry = if sidecar_path.is_file() {
        read_yaml_file_checked(&sidecar_path)?
    } else {
        serde_yaml::Value::Null
    };
    merge_registry_projection(
        &base_registry,
        &sidecar_registry,
        key,
        id_field,
        registry_label,
    )
}

pub(crate) fn runtime_agent_extensions_root(project_root: &Path) -> PathBuf {
    project_root.join(".vida/project/agent-extensions")
}

pub(crate) fn read_yaml_file_checked(path: &Path) -> Result<serde_yaml::Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn resolve_host_cli_template_source(
    cli_system: &str,
    registry_entry: Option<&serde_yaml::Value>,
) -> Result<PathBuf, String> {
    let template_relative = registry_entry
        .and_then(host_cli_system_template_root)
        .ok_or_else(|| format!("No template_root configured for host CLI `{cli_system}`"))?;
    let primary_root = resolve_init_bootstrap_source_root();
    let fallback_root = super::repo_runtime_root();
    let candidates = if fallback_root == primary_root {
        vec![primary_root.join(&template_relative)]
    } else {
        vec![
            primary_root.join(&template_relative),
            fallback_root.join(&template_relative),
        ]
    };
    for candidate in &candidates {
        if candidate.is_dir() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "Missing framework host CLI template for `{cli_system}`. Checked: {}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub(crate) fn apply_host_cli_selection(
    project_root: &Path,
    cli_system: &str,
) -> Result<(), String> {
    let config_path = project_root.join("vida.config.yaml");
    if !config_path.is_file() {
        return Err(format!(
            "Missing project overlay; expected {} before host CLI activation",
            config_path.display()
        ));
    }
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?;
    let mut updated = if contents.contains(HOST_CLI_PLACEHOLDER) {
        contents.replace(HOST_CLI_PLACEHOLDER, cli_system)
    } else if contents.contains("host_environment:") && contents.contains("cli_system:") {
        let mut rewritten = String::new();
        let mut replaced = false;
        for line in contents.lines() {
            if line.trim_start().starts_with("cli_system:") && !replaced {
                rewritten.push_str(&format!("  cli_system: {cli_system}\n"));
                replaced = true;
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        rewritten
    } else {
        format!(
            "{}\nhost_environment:\n  cli_system: {cli_system}\n",
            contents.trim_end()
        )
    };
    let registry = load_host_cli_system_registry_from_root(project_root);
    if registry
        .get(&normalize_host_cli_system(cli_system).unwrap_or_default())
        .as_ref()
        .is_some_and(|entry| host_cli_system_agent_only_defaults_enabled(entry, cli_system))
    {
        updated = apply_agent_only_development_defaults(&updated);
    }
    fs::write(&config_path, updated)
        .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))
}

pub(crate) fn apply_agent_only_development_defaults(contents: &str) -> String {
    let mut updated = contents.to_string();
    updated =
        set_yaml_bool_in_top_level_section(&updated, "protocol_activation", "agent_system", true);
    updated = set_yaml_bool_in_top_level_section(
        &updated,
        "autonomous_execution",
        "agent_only_development",
        true,
    );
    updated = set_yaml_bool_in_top_level_section(&updated, "agent_system", "init_on_boot", true);
    updated = set_yaml_scalar_in_top_level_section(&updated, "agent_system", "mode", "native");
    updated = set_yaml_scalar_in_top_level_section(
        &updated,
        "agent_system",
        "state_owner",
        "orchestrator_only",
    );
    updated =
        set_yaml_scalar_in_top_level_section(&updated, "agent_system", "max_parallel_agents", "4");
    updated
}

pub(crate) fn materialize_host_cli_template(
    project_root: &Path,
    cli_system: &str,
    registry_entry: Option<&serde_yaml::Value>,
) -> Result<PathBuf, String> {
    let entry_value = registry_entry
        .cloned()
        .ok_or_else(|| format!("Registry entry required for host CLI `{cli_system}`"))?;
    let entry_ref = entry_value;
    let source = resolve_host_cli_template_source(cli_system, Some(&entry_ref))?;
    let runtime_root = host_cli_system_runtime_root(&entry_ref, cli_system, project_root);
    let mode = host_cli_system_materialization_mode(&entry_ref, cli_system);
    let copy_tree_target = project_root.join(&runtime_root);
    match mode.as_str() {
        HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE => {
            project_activator_host_cli_materialization::materialize_host_cli_template_with_catalog_render(
                project_root,
                cli_system,
                &entry_ref,
            )
        }
        "copy_tree_only" => {
            copy_tree_if_missing(&source, &copy_tree_target)?;
            Ok(runtime_root)
        }
        other => Err(format!(
            "Unsupported materialization_mode `{other}` for host CLI `{cli_system}`"
        )),
    }
}

fn copy_tree_if_missing(source_root: &Path, target_root: &Path) -> Result<(), String> {
    if target_root.exists() {
        return Ok(());
    }
    copy_tree_recursive(source_root, target_root)
}

fn copy_tree_recursive(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if metadata.is_dir() {
        fs::create_dir_all(target_root)
            .map_err(|error| format!("Failed to create {}: {error}", target_root.display()))?;
        for entry in fs::read_dir(source_root)
            .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?
        {
            let entry = entry
                .map_err(|error| format!("Failed to iterate {}: {error}", source_root.display()))?;
            let source_path = entry.path();
            let target_path = target_root.join(entry.file_name());
            copy_tree_recursive(&source_path, &target_path)?;
        }
        return Ok(());
    }

    if let Some(parent) = target_root.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(source_root, target_root).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source_root.display(),
            target_root.display()
        )
    })?;
    Ok(())
}

pub(crate) fn resolved_host_cli_agent_catalog_for_root(
    project_root: &Path,
    overlay: &serde_yaml::Value,
) -> Result<(String, Vec<serde_json::Value>), String> {
    let selected_host_cli_system = yaml_lookup(overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(normalize_host_cli_system)
        .ok_or_else(|| {
            "Host CLI system is missing or unsupported in vida.config.yaml.".to_string()
        })?;
    let registry = host_cli_system_registry_with_fallback(Some(overlay));
    let catalog_entry = registry.get(&selected_host_cli_system);
    let mut host_cli_agent_catalog =
        project_activator_host_cli_materialization::host_cli_entry_carrier_catalog(catalog_entry);
    if host_cli_agent_catalog.is_empty() {
        if catalog_entry
            .map(|entry| host_cli_system_materialization_mode(entry, &selected_host_cli_system))
            .as_deref()
            == Some(HOST_CLI_TEMPLATE_CATALOG_RENDER_MODE)
        {
            host_cli_agent_catalog = project_activator_host_cli_materialization::resolve_host_cli_agent_catalog_for_rendered_root(
                project_root,
                overlay,
                catalog_entry,
                &selected_host_cli_system,
            );
        }
    }
    if host_cli_agent_catalog.is_empty() {
        return Err(format!(
            "Host CLI `{selected_host_cli_system}` does not define any carriers in vida.config.yaml or the rendered runtime catalog."
        ));
    }
    Ok((selected_host_cli_system, host_cli_agent_catalog))
}

pub(crate) fn file_contains_placeholder(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|contents| {
            let lowercase = contents.to_ascii_lowercase();
            lowercase.contains("project documentation: docs/")
                || contents.contains(PROJECT_ID_PLACEHOLDER)
                || contents.contains(DOCS_ROOT_PLACEHOLDER)
                || contents.contains(PROCESS_ROOT_PLACEHOLDER)
                || contents.contains(RESEARCH_ROOT_PLACEHOLDER)
                || contents.contains(README_DOC_PLACEHOLDER)
                || contents.contains(ARCHITECTURE_DOC_PLACEHOLDER)
                || contents.contains(DECISIONS_DOC_PLACEHOLDER)
                || contents.contains(ENVIRONMENTS_DOC_PLACEHOLDER)
                || contents.contains(PROJECT_OPERATIONS_DOC_PLACEHOLDER)
                || contents.contains(AGENT_SYSTEM_DOC_PLACEHOLDER)
                || contents.contains(USER_COMMUNICATION_PLACEHOLDER)
                || contents.contains(REASONING_LANGUAGE_PLACEHOLDER)
                || contents.contains(DOCUMENTATION_LANGUAGE_PLACEHOLDER)
                || contents.contains(TODO_PROTOCOL_LANGUAGE_PLACEHOLDER)
                || contents.contains(HOST_CLI_PLACEHOLDER)
                || contents.contains("<fill-your-project-name>")
                || contents.contains("<project-root-map-path>")
                || contents.contains("<product-index-path>")
                || contents.contains("<product-spec-map-path>")
                || contents.contains("<project-documentation-law-path>")
                || contents.contains("<documentation-tooling-map-path>")
                || contents.contains("<project-extension-map-path>")
        })
        .unwrap_or(false)
}

pub(crate) fn build_project_activator_view(project_root: &Path) -> serde_json::Value {
    let agents_md = project_root.join("AGENTS.md");
    let agents_sidecar = project_root.join("AGENTS.sidecar.md");
    let vida_config = project_root.join("vida.config.yaml");
    let vida_home = project_root.join(".vida");
    let vida_config_dir = project_root.join(".vida/config");
    let vida_db_dir = project_root.join(".vida/db");
    let vida_cache_dir = project_root.join(".vida/cache");
    let vida_framework_dir = project_root.join(".vida/framework");
    let vida_project_dir = project_root.join(".vida/project");
    let vida_receipts_dir = project_root.join(".vida/receipts");
    let vida_runtime_dir = project_root.join(".vida/runtime");
    let vida_scratchpad_dir = project_root.join(".vida/scratchpad");
    let project_root_map = project_root.join("docs/project-root-map.md");
    let product_index = project_root.join("docs/product/index.md");
    let product_spec_readme = project_root.join(DEFAULT_PROJECT_PRODUCT_SPEC_README);
    let feature_design_template = project_root.join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE);
    let process_readme = project_root.join("docs/process/README.md");
    let codex_agent_guide = project_root.join(DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC);
    let documentation_tooling_map = project_root.join(DEFAULT_PROJECT_DOC_TOOLING_DOC);
    let runtime_agent_extensions = runtime_agent_extensions_root(project_root);
    let runtime_agent_extensions_readme = runtime_agent_extensions.join("README.md");
    let runtime_agent_extension_roles = runtime_agent_extensions.join("roles.yaml");
    let runtime_agent_extension_skills = runtime_agent_extensions.join("skills.yaml");
    let runtime_agent_extension_profiles = runtime_agent_extensions.join("profiles.yaml");
    let runtime_agent_extension_flows = runtime_agent_extensions.join("flows.yaml");
    let runtime_agent_extension_dispatch_aliases =
        runtime_agent_extensions.join("dispatch-aliases.yaml");
    let runtime_agent_extension_role_sidecar = runtime_agent_extensions.join("roles.sidecar.yaml");
    let runtime_agent_extension_skill_sidecar =
        runtime_agent_extensions.join("skills.sidecar.yaml");
    let runtime_agent_extension_profile_sidecar =
        runtime_agent_extensions.join("profiles.sidecar.yaml");
    let runtime_agent_extension_flow_sidecar = runtime_agent_extensions.join("flows.sidecar.yaml");
    let runtime_agent_extension_dispatch_alias_sidecar =
        runtime_agent_extensions.join("dispatch-aliases.sidecar.yaml");

    let sidecar_missing = !agents_sidecar.is_file();
    let sidecar_has_placeholders =
        agents_sidecar.is_file() && file_contains_placeholder(&agents_sidecar);
    let config_has_placeholders = vida_config.is_file() && file_contains_placeholder(&vida_config);
    let runtime_home_missing = [
        &vida_config_dir,
        &vida_db_dir,
        &vida_cache_dir,
        &vida_framework_dir,
        &vida_project_dir,
        &vida_receipts_dir,
        &vida_runtime_dir,
        &vida_scratchpad_dir,
    ]
    .iter()
    .any(|path| !path.is_dir());
    let bootstrap_missing = !agents_md.is_file() || !vida_config.is_file() || runtime_home_missing;
    let docs_missing = !project_root_map.is_file()
        || !product_index.is_file()
        || !product_spec_readme.is_file()
        || !feature_design_template.is_file()
        || !process_readme.is_file()
        || !codex_agent_guide.is_file()
        || !documentation_tooling_map.is_file();

    let project_overlay = if vida_config.is_file() {
        read_yaml_file_checked(&vida_config).ok()
    } else {
        None
    };
    let host_cli_system_registry = host_cli_system_registry_with_fallback(project_overlay.as_ref());
    let host_cli_summary =
        crate::project_activator_host_cli_summary::build_project_activator_host_cli_summary(
            project_root,
            project_overlay.as_ref(),
            &host_cli_system_registry,
        );
    let supported_host_cli_systems = host_cli_summary.supported_host_cli_systems;
    let host_cli_suggested_system = host_cli_summary.host_cli_suggested_system;
    let host_cli_supported_list = host_cli_summary.host_cli_supported_list;
    let current_project_id = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["project", "id"])));
    let current_user_communication_language = project_overlay.as_ref().and_then(|config| {
        yaml_string(yaml_lookup(
            config,
            &["language_policy", "user_communication"],
        ))
    });
    let current_reasoning_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "reasoning"])));
    let current_documentation_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "documentation"])));
    let current_todo_protocol_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "todo_protocol"])));
    let selected_host_cli_system = host_cli_summary.selected_host_cli_system;
    let host_cli_selection_required = host_cli_summary.host_cli_selection_required;
    let host_cli_runtime_template_root = host_cli_summary.host_cli_runtime_template_root;
    let host_cli_execution_class = host_cli_summary.host_cli_execution_class;
    let host_cli_template_materialized = host_cli_summary.host_cli_template_materialized;
    let host_cli_materialization_required = host_cli_summary.host_cli_materialization_required;
    let host_cli_template_source_root = host_cli_summary.host_cli_template_source_root;
    let default_host_agent_templates = host_cli_summary.default_host_agent_templates;
    let default_agent_topology = host_cli_summary.default_agent_topology;
    let carrier_tier_rates = host_cli_summary.carrier_tier_rates;
    let agent_extensions_summary = crate::project_activator_agent_extensions_summary::
        build_project_activator_agent_extensions_summary(project_root, project_overlay.as_ref());
    let agent_extensions_enabled = agent_extensions_summary.agent_extensions_enabled;
    let agent_extensions_ready = agent_extensions_summary.agent_extensions_ready;
    let agent_extension_validation_error =
        agent_extensions_summary.agent_extension_validation_error;
    let execution_carrier_model = agent_extensions_summary.execution_carrier_model;

    let runtime_agent_extensions_missing = [
        &runtime_agent_extensions_readme,
        &runtime_agent_extension_roles,
        &runtime_agent_extension_skills,
        &runtime_agent_extension_profiles,
        &runtime_agent_extension_flows,
        &runtime_agent_extension_dispatch_aliases,
        &runtime_agent_extension_role_sidecar,
        &runtime_agent_extension_skill_sidecar,
        &runtime_agent_extension_profile_sidecar,
        &runtime_agent_extension_flow_sidecar,
        &runtime_agent_extension_dispatch_alias_sidecar,
    ]
    .iter()
    .any(|path| !path.exists());

    let activation_summary =
        crate::project_activator_activation_summary::build_project_activator_activation_summary(
            crate::project_activator_activation_summary::ProjectActivatorActivationInputs {
                project_root,
                bootstrap_missing,
                sidecar_missing,
                sidecar_has_placeholders,
                docs_missing,
                config_has_placeholders,
                current_project_id: current_project_id.as_deref(),
                current_user_communication_language: current_user_communication_language.as_deref(),
                current_reasoning_language: current_reasoning_language.as_deref(),
                current_documentation_language: current_documentation_language.as_deref(),
                current_todo_protocol_language: current_todo_protocol_language.as_deref(),
                host_cli_selection_required,
                host_cli_materialization_required,
                host_cli_suggested_system: &host_cli_suggested_system,
                host_cli_supported_list: &host_cli_supported_list,
                supported_host_cli_systems: &supported_host_cli_systems,
                selected_host_cli_system: selected_host_cli_system.as_deref(),
                agent_extensions_enabled,
                runtime_agent_extensions_missing,
                agent_extensions_ready,
                agent_extension_validation_error: agent_extension_validation_error.as_deref(),
            },
        );
    let activation_pending = activation_summary.activation_pending;
    let execution_posture_ambiguous = activation_summary.execution_posture_ambiguous;
    let sidecar_or_project_docs_too_thin = activation_summary.sidecar_or_project_docs_too_thin;
    let required_inputs = activation_summary.required_inputs;
    let one_shot_example = activation_summary.one_shot_example;
    let next_steps = activation_summary.next_steps;

    serde_json::json!({
        "surface": "vida project-activator",
        "status": if activation_pending { "pending" } else { "ready_enough_for_normal_work" },
        "activation_pending": activation_pending,
        "project_root": project_root.display().to_string(),
        "project_shape": detect_project_shape(project_root),
        "triggers": {
            "initial_onboarding_missing": bootstrap_missing || sidecar_missing,
            "config_state_incomplete": !vida_config.is_file() || config_has_placeholders,
            "sidecar_or_project_docs_too_thin": sidecar_or_project_docs_too_thin,
            "execution_posture_ambiguous": execution_posture_ambiguous,
            "host_cli_unselected_or_unmaterialized": host_cli_selection_required || host_cli_materialization_required,
            "agent_extensions_invalid": agent_extensions_enabled && !agent_extensions_ready,
        },
        "bootstrap_surfaces": {
            "agents_md": agents_md.is_file(),
            "agents_sidecar_md": agents_sidecar.is_file(),
            "vida_config_yaml": vida_config.is_file(),
            "vida_home": vida_home.is_dir(),
            "vida_config_dir": vida_config_dir.is_dir(),
            "vida_db_dir": vida_db_dir.is_dir(),
            "vida_cache_dir": vida_cache_dir.is_dir(),
            "vida_framework_dir": vida_framework_dir.is_dir(),
            "vida_project_dir": vida_project_dir.is_dir(),
            "vida_receipts_dir": vida_receipts_dir.is_dir(),
            "vida_runtime_dir": vida_runtime_dir.is_dir(),
            "vida_scratchpad_dir": vida_scratchpad_dir.is_dir(),
        },
        "project_docs": {
            "project_root_map": project_root_map.is_file(),
            "product_index": product_index.is_file(),
            "product_spec_readme": product_spec_readme.is_file(),
            "feature_design_template": feature_design_template.is_file(),
            "process_readme": process_readme.is_file(),
            "codex_agent_configuration_guide": codex_agent_guide.is_file(),
            "documentation_tooling_map": documentation_tooling_map.is_file(),
            "sidecar_has_placeholders": sidecar_has_placeholders,
            "config_has_placeholders": config_has_placeholders,
        },
        "agent_extensions": {
            "enabled": agent_extensions_enabled,
            "runtime_projection_root": runtime_agent_extensions.display().to_string(),
            "runtime_readme": runtime_agent_extensions_readme.is_file(),
            "roles_registry": runtime_agent_extension_roles.is_file(),
            "skills_registry": runtime_agent_extension_skills.is_file(),
            "profiles_registry": runtime_agent_extension_profiles.is_file(),
            "flows_registry": runtime_agent_extension_flows.is_file(),
            "dispatch_aliases_registry": runtime_agent_extension_dispatch_aliases.is_file(),
            "roles_sidecar": runtime_agent_extension_role_sidecar.is_file(),
            "skills_sidecar": runtime_agent_extension_skill_sidecar.is_file(),
            "profiles_sidecar": runtime_agent_extension_profile_sidecar.is_file(),
            "flows_sidecar": runtime_agent_extension_flow_sidecar.is_file(),
            "dispatch_aliases_sidecar": runtime_agent_extension_dispatch_alias_sidecar.is_file(),
            "bundle_ready": agent_extensions_ready,
            "validation_error": agent_extension_validation_error,
        },
        "host_environment": crate::project_activator_runtime_surface::build_project_activator_host_environment(
            supported_host_cli_systems,
            selected_host_cli_system,
            host_cli_execution_class,
            host_cli_selection_required,
            host_cli_template_materialized,
            host_cli_materialization_required,
            host_cli_runtime_template_root,
            host_cli_template_source_root,
            default_host_agent_templates,
        ),
        "activation_algorithm": crate::project_activator_runtime_surface::build_project_activator_activation_algorithm(),
        "normal_work_defaults": crate::project_activator_normal_work_defaults::build_project_activator_normal_work_defaults(
            default_agent_topology,
            carrier_tier_rates,
            execution_carrier_model,
        ),
        "interview": {
            "required_inputs": required_inputs,
            "safe_defaults": {
                "project_bootstrap.docs_root": DEFAULT_PROJECT_DOCS_ROOT,
                "project_bootstrap.process_root": DEFAULT_PROJECT_PROCESS_ROOT,
                "project_bootstrap.research_root": DEFAULT_PROJECT_RESEARCH_ROOT,
                "project_bootstrap.readme_doc": "README.md",
                "project_bootstrap.architecture_doc": DEFAULT_PROJECT_ARCHITECTURE_DOC,
                "project_bootstrap.decisions_doc": DEFAULT_PROJECT_DECISIONS_DOC,
                "project_bootstrap.environments_doc": DEFAULT_PROJECT_ENVIRONMENTS_DOC,
                "project_bootstrap.project_operations_doc": DEFAULT_PROJECT_OPERATIONS_DOC,
                "project_bootstrap.agent_system_doc": DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
                "project_docs.documentation_tooling_doc": DEFAULT_PROJECT_DOC_TOOLING_DOC
            },
            "one_shot_example": one_shot_example
        },
        "current_activation_state": {
            "project_id": current_project_id,
            "user_communication_language": current_user_communication_language,
            "reasoning_language": current_reasoning_language,
            "documentation_language": current_documentation_language,
            "todo_protocol_language": current_todo_protocol_language
        },
        "next_steps": next_steps,
        "bounded_scope_note": "This runtime surface reports activation posture, required interview inputs, and bounded onboarding next steps. While activation remains pending it is a doc/config onboarding path, not tracked TaskFlow execution.",
    })
}

pub(crate) fn canonical_project_activation_status_truth(
    project_root: &Path,
) -> ProjectActivationStatusTruth {
    let view = build_project_activator_view(project_root);
    let activation_pending = view["activation_pending"].as_bool().unwrap_or(true);
    let status =
        canonical_activation_status(view["status"].as_str(), activation_pending).to_string();
    let next_steps = view["next_steps"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ProjectActivationStatusTruth {
        status,
        activation_pending,
        next_steps,
    }
}

fn db_first_activation_truth_read_back_error(
    expected: &crate::state_store::LauncherActivationSnapshot,
    read_back: &crate::state_store::LauncherActivationSnapshot,
) -> Option<String> {
    if read_back.source != expected.source {
        return Some(
            "DB-first activation truth read-back mismatch in authoritative state store: source drift detected."
                .to_string(),
        );
    }
    if read_back.source_config_digest != expected.source_config_digest {
        return Some(
            "DB-first activation truth read-back mismatch in authoritative state store: source_config_digest drift detected."
                .to_string(),
        );
    }
    None
}

pub(crate) async fn run_project_activator(args: super::ProjectActivatorArgs) -> ExitCode {
    let project_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let pre_activation_view = build_project_activator_view(&project_root);
    let activation_pending = pre_activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true);
    let activation_mutation_requested = args.host_cli_system.is_some()
        || args.project_id.is_some()
        || args.project_name.is_some()
        || args.language.is_some()
        || args.user_communication_language.is_some()
        || args.reasoning_language.is_some()
        || args.documentation_language.is_some()
        || args.todo_protocol_language.is_some();
    let mut activation_store: Option<super::StateStore> = None;
    if activation_mutation_requested {
        let state_dir = args
            .state_dir
            .clone()
            .unwrap_or_else(super::state_store::default_state_dir);
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            super::StateStore::open(state_dir.clone()),
        )
        .await
        {
            Ok(Ok(store)) => {
                activation_store = Some(store);
            }
            Ok(Err(error)) => {
                eprintln!(
                    "Project activation failed closed before mutation: unable to initialize authoritative state store at {}: {error}",
                    state_dir.display()
                );
                return ExitCode::from(1);
            }
            Err(_) => {
                eprintln!(
                    "Project activation failed closed before mutation: timed out initializing authoritative state store at {} after 10s",
                    state_dir.display()
                );
                return ExitCode::from(1);
            }
        }
    }
    if activation_pending && activation_mutation_requested {
        let missing_inputs = missing_required_activation_inputs(&pre_activation_view, &args);
        if !missing_inputs.is_empty() {
            let missing_flags = pre_activation_view["interview"]["required_inputs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|input| {
                    input["id"]
                        .as_str()
                        .map(|id| missing_inputs.iter().any(|missing| missing == id))
                        .unwrap_or(false)
                })
                .filter_map(|input| input["flag"].as_str())
                .collect::<Vec<_>>();
            let mut message = format!(
                "Project activation is still pending and fails closed until the bounded activation interview is complete. Missing required inputs: {}.",
                if missing_flags.is_empty() {
                    missing_inputs.join(", ")
                } else {
                    missing_flags.join(", ")
                }
            );
            if let Some(example) = pre_activation_view["interview"]["one_shot_example"].as_str() {
                message.push_str(&format!(" Use `{example}`."));
            }
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    }
    let mut host_cli_activated = None;
    let mut changed_files = Vec::new();
    let activation_answers = resolve_project_activation_answers(&project_root, &args);

    if let Some(requested_host_cli_system) = args.host_cli_system.as_deref() {
        let registry = load_host_cli_system_registry_from_root(&project_root);
        let supported_values = registry
            .iter()
            .filter(|(_, entry)| host_cli_system_enabled(entry))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let normalized_host_cli_system = match normalize_host_cli_system(requested_host_cli_system)
        {
            Some(value) => value,
            None => {
                eprintln!(
                    "Unsupported host CLI system `{requested_host_cli_system}`. Supported values: {}",
                    supported_values.join(", ")
                );
                return ExitCode::from(2);
            }
        };
        let host_cli_entry = match registry.get(&normalized_host_cli_system) {
            Some(entry) if host_cli_system_enabled(entry) => entry,
            Some(_) => {
                eprintln!("Host CLI system `{normalized_host_cli_system}` is currently disabled.");
                return ExitCode::from(2);
            }
            None => {
                let supported_values = registry
                    .iter()
                    .filter(|(_, entry)| host_cli_system_enabled(entry))
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>();
                eprintln!(
                    "Unsupported host CLI system `{normalized_host_cli_system}`. Supported values: {}",
                    supported_values.join(", ")
                );
                return ExitCode::from(2);
            }
        };
        let runtime_root =
            match apply_host_cli_selection(&project_root, &normalized_host_cli_system).and_then(
                |()| {
                    materialize_host_cli_template(
                        &project_root,
                        &normalized_host_cli_system,
                        Some(host_cli_entry),
                    )
                },
            ) {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
        host_cli_activated = Some(normalized_host_cli_system.to_string());
        changed_files.push("vida.config.yaml".to_string());
        let relative_runtime_root = runtime_root
            .strip_prefix(&project_root)
            .unwrap_or_else(|_| runtime_root.as_path())
            .to_string_lossy()
            .to_string();
        changed_files.push(format!("{relative_runtime_root}/**"));
    }

    if let Some(answers) = activation_answers.as_ref() {
        match apply_project_activation_answers(&project_root, answers) {
            Ok(mut files) => changed_files.append(&mut files),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }

    changed_files.sort();
    changed_files.dedup();
    let host_template_materialized = host_cli_activated.is_some();
    let activation_receipt_path = match write_project_activation_receipt(
        &project_root,
        activation_answers.as_ref(),
        host_cli_activated.as_deref(),
        &changed_files,
        host_template_materialized,
    ) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let mut activation_truth_sync = None;
    if activation_mutation_requested {
        let store = activation_store
            .as_ref()
            .expect("activation store should be initialized before activation mutation");
        match super::sync_launcher_activation_snapshot(store).await {
            Ok(snapshot) => {
                let read_back = match store.read_launcher_activation_snapshot().await {
                    Ok(current) => current,
                    Err(error) => {
                        eprintln!(
                            "Project activation failed closed after mutation: unable to read back DB-first activation truth from authoritative state store: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                if let Some(error) =
                    db_first_activation_truth_read_back_error(&snapshot, &read_back)
                {
                    eprintln!("Project activation failed closed after mutation: {error}");
                    return ExitCode::from(1);
                }
                if read_back.source != "state_store" {
                    eprintln!(
                        "Project activation failed closed after mutation: DB-first activation truth source must be `state_store`."
                    );
                    return ExitCode::from(1);
                }
                if read_back.source_config_digest.trim().is_empty() {
                    eprintln!(
                        "Project activation failed closed after mutation: DB-first activation truth metadata is incomplete in authoritative state store."
                    );
                    return ExitCode::from(1);
                }
                activation_truth_sync = Some(serde_json::json!({
                    "source": snapshot.source,
                    "source_config_path": snapshot.source_config_path,
                    "source_config_digest": snapshot.source_config_digest,
                    "read_back_verified": true,
                }));
            }
            Err(error) => {
                eprintln!(
                    "Project activation failed closed after mutation: unable to persist DB-first activation truth in authoritative state store: {error}"
                );
                return ExitCode::from(1);
            }
        }
    }

    let mut view = build_project_activator_view(&project_root);
    if let Some(path) = activation_receipt_path.as_deref() {
        let mut activation_log = serde_json::json!({
            "receipt_path": path,
            "changed_files": changed_files,
        });
        if let Some(sync) = activation_truth_sync {
            activation_log["db_first_activation_truth"] = sync;
        }
        view["activation_log"] = activation_log;
    }
    let host_cli_restart_target = host_cli_activated.as_deref().map(host_cli_display_name);
    if args.json {
        let payload = if host_cli_activated.is_some() {
            serde_json::json!({
                "surface": "vida project-activator",
                "post_init_restart_required": true,
                "post_init_restart_note": format!(
                    "close and restart {} so the newly activated agent template becomes visible to the runtime execution environment",
                    host_cli_restart_target
                        .as_deref()
                        .unwrap_or("the host CLI system")
                ),
                "activation_log": view["activation_log"],
                "view": view,
            })
        } else if activation_receipt_path.is_some() {
            serde_json::json!({
                "surface": "vida project-activator",
                "activation_log": view["activation_log"],
                "view": view,
            })
        } else {
            view
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("project activator view should render")
        );
        return ExitCode::SUCCESS;
    }

    super::print_surface_header(super::RenderMode::Plain, "vida project-activator");
    super::print_surface_line(
        super::RenderMode::Plain,
        "status",
        view["status"].as_str().unwrap_or("unknown"),
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "project_root",
        view["project_root"].as_str().unwrap_or("unknown"),
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "project_shape",
        view["project_shape"].as_str().unwrap_or("unknown"),
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "activation_pending",
        if view["activation_pending"].as_bool().unwrap_or(true) {
            "true"
        } else {
            "false"
        },
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "host_cli_system",
        view["host_environment"]["selected_cli_system"]
            .as_str()
            .unwrap_or("unselected"),
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "taskflow_admitted_while_pending",
        if view["activation_algorithm"]["taskflow_admitted_while_pending"]
            .as_bool()
            .unwrap_or(false)
        {
            "true"
        } else {
            "false"
        },
    );
    println!("required_inputs");
    if let Some(inputs) = view["interview"]["required_inputs"].as_array() {
        if inputs.is_empty() {
            println!("  - none");
        } else {
            for input in inputs {
                let prompt = input["prompt"].as_str().unwrap_or("unspecified");
                let flag = input["flag"].as_str().unwrap_or("--unknown");
                let suggested_value = input["suggested_value"].as_str().unwrap_or("n/a");
                println!("  - {prompt} ({flag}, suggested: {suggested_value})");
            }
        }
    }
    super::print_surface_line(
        super::RenderMode::Plain,
        "one_shot_example",
        view["interview"]["one_shot_example"]
            .as_str()
            .unwrap_or("vida project-activator --json"),
    );
    println!("next_steps");
    if let Some(steps) = view["next_steps"].as_array() {
        for step in steps {
            if let Some(step) = step.as_str() {
                println!("  - {step}");
            }
        }
    }
    if let Some(path) = activation_receipt_path.as_deref() {
        println!("  - activation log: {path}");
    }
    if host_cli_activated.is_some() {
        println!(
            "  - close and restart {} so the newly activated agent template becomes visible to the runtime environment",
            host_cli_restart_target
                .as_deref()
                .unwrap_or("the host CLI system")
        );
    }
    ExitCode::SUCCESS
}

fn resolve_project_activation_answers(
    project_root: &Path,
    args: &ProjectActivatorArgs,
) -> Option<ProjectActivationAnswers> {
    let config_path = project_root.join("vida.config.yaml");
    let current_overlay = if config_path.is_file() {
        read_yaml_file_checked(&config_path).ok()
    } else {
        None
    };
    let current_project_id = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["project", "id"])))
        .filter(|value| !is_missing_or_placeholder(Some(value.as_str()), PROJECT_ID_PLACEHOLDER));
    let current_user_communication_language = current_overlay
        .as_ref()
        .and_then(|config| {
            yaml_string(yaml_lookup(
                config,
                &["language_policy", "user_communication"],
            ))
        })
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), USER_COMMUNICATION_PLACEHOLDER)
        });
    let current_reasoning_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "reasoning"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), REASONING_LANGUAGE_PLACEHOLDER)
        });
    let current_documentation_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "documentation"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), DOCUMENTATION_LANGUAGE_PLACEHOLDER)
        });
    let current_todo_protocol_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "todo_protocol"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), TODO_PROTOCOL_LANGUAGE_PLACEHOLDER)
        });
    let any_input_provided = args.project_id.is_some()
        || args.project_name.is_some()
        || args.language.is_some()
        || args.user_communication_language.is_some()
        || args.reasoning_language.is_some()
        || args.documentation_language.is_some()
        || args.todo_protocol_language.is_some();
    if !any_input_provided {
        return None;
    }

    let project_id = trimmed_non_empty(args.project_id.as_deref())
        .or_else(|| {
            trimmed_non_empty(args.project_name.as_deref())
                .map(|name| slugify_project_id(&name))
                .filter(|value| !value.is_empty())
        })
        .or(current_project_id)?;
    let shared_language = trimmed_non_empty(args.language.as_deref());
    let user_communication_language =
        trimmed_non_empty(args.user_communication_language.as_deref())
            .or(shared_language.clone())
            .or(current_user_communication_language)?;
    let reasoning_language = trimmed_non_empty(args.reasoning_language.as_deref())
        .or(shared_language.clone())
        .or(current_reasoning_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let documentation_language = trimmed_non_empty(args.documentation_language.as_deref())
        .or(shared_language.clone())
        .or(current_documentation_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let todo_protocol_language = trimmed_non_empty(args.todo_protocol_language.as_deref())
        .or(shared_language)
        .or(current_todo_protocol_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let project_title = inferred_project_title(&project_id, args.project_name.as_deref());

    Some(ProjectActivationAnswers {
        project_id,
        project_title,
        user_communication_language,
        reasoning_language,
        documentation_language,
        todo_protocol_language,
    })
}

fn project_activator_supplied_required_input(input_id: &str, args: &ProjectActivatorArgs) -> bool {
    match input_id {
        "project_id" => {
            trimmed_non_empty(args.project_id.as_deref()).is_some()
                || trimmed_non_empty(args.project_name.as_deref()).is_some()
        }
        "language" => {
            trimmed_non_empty(args.language.as_deref()).is_some()
                || trimmed_non_empty(args.user_communication_language.as_deref()).is_some()
                || trimmed_non_empty(args.reasoning_language.as_deref()).is_some()
                || trimmed_non_empty(args.documentation_language.as_deref()).is_some()
                || trimmed_non_empty(args.todo_protocol_language.as_deref()).is_some()
        }
        "host_cli_system" => trimmed_non_empty(args.host_cli_system.as_deref()).is_some(),
        _ => false,
    }
}

fn missing_required_activation_inputs(
    view: &serde_json::Value,
    args: &ProjectActivatorArgs,
) -> Vec<String> {
    view["interview"]["required_inputs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|input| input["id"].as_str())
        .filter(|id| !project_activator_supplied_required_input(id, args))
        .map(|id| id.to_string())
        .collect()
}

fn apply_project_activation_answers(
    project_root: &Path,
    answers: &ProjectActivationAnswers,
) -> Result<Vec<String>, String> {
    let config_path = project_root.join("vida.config.yaml");
    if !config_path.is_file() {
        return Err(format!(
            "Missing project overlay; expected {} before project activation writes",
            config_path.display()
        ));
    }
    let original_contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?;
    let mut updated_contents = original_contents
        .replace(PROJECT_ID_PLACEHOLDER, &answers.project_id)
        .replace(DOCS_ROOT_PLACEHOLDER, DEFAULT_PROJECT_DOCS_ROOT)
        .replace(PROCESS_ROOT_PLACEHOLDER, DEFAULT_PROJECT_PROCESS_ROOT)
        .replace(RESEARCH_ROOT_PLACEHOLDER, DEFAULT_PROJECT_RESEARCH_ROOT)
        .replace(README_DOC_PLACEHOLDER, "README.md")
        .replace(
            ARCHITECTURE_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_ARCHITECTURE_DOC,
        )
        .replace(DECISIONS_DOC_PLACEHOLDER, DEFAULT_PROJECT_DECISIONS_DOC)
        .replace(
            ENVIRONMENTS_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_ENVIRONMENTS_DOC,
        )
        .replace(
            PROJECT_OPERATIONS_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_OPERATIONS_DOC,
        )
        .replace(
            AGENT_SYSTEM_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
        )
        .replace(
            USER_COMMUNICATION_PLACEHOLDER,
            &answers.user_communication_language,
        )
        .replace(REASONING_LANGUAGE_PLACEHOLDER, &answers.reasoning_language)
        .replace(
            DOCUMENTATION_LANGUAGE_PLACEHOLDER,
            &answers.documentation_language,
        )
        .replace(
            TODO_PROTOCOL_LANGUAGE_PLACEHOLDER,
            &answers.todo_protocol_language,
        );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project",
        "id",
        &answers.project_id,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "docs_root",
        DEFAULT_PROJECT_DOCS_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "process_root",
        DEFAULT_PROJECT_PROCESS_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "research_root",
        DEFAULT_PROJECT_RESEARCH_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "readme_doc",
        "README.md",
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "architecture_doc",
        DEFAULT_PROJECT_ARCHITECTURE_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "decisions_doc",
        DEFAULT_PROJECT_DECISIONS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "environments_doc",
        DEFAULT_PROJECT_ENVIRONMENTS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "project_operations_doc",
        DEFAULT_PROJECT_OPERATIONS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "agent_system_doc",
        DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
    );
    updated_contents = set_yaml_bool_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "allow_scaffold_missing",
        false,
    );
    updated_contents = set_yaml_bool_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "require_launch_confirmation",
        false,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "user_communication",
        &answers.user_communication_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "reasoning",
        &answers.reasoning_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "documentation",
        &answers.documentation_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "todo_protocol",
        &answers.todo_protocol_language,
    );

    let mut changed_files = Vec::new();
    if updated_contents != original_contents {
        fs::write(&config_path, updated_contents)
            .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))?;
        changed_files.push("vida.config.yaml".to_string());
    }

    let generated_files = vec![
        (
            project_root.join("AGENTS.sidecar.md"),
            render_project_sidecar(&answers.project_title),
            "AGENTS.sidecar.md",
        ),
        (
            project_root.join("README.md"),
            super::init_surfaces::render_project_readme(&answers.project_title),
            "README.md",
        ),
        (
            project_root.join(DEFAULT_PROJECT_ROOT_MAP),
            super::init_surfaces::render_project_root_map(),
            DEFAULT_PROJECT_ROOT_MAP,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PRODUCT_INDEX),
            super::init_surfaces::render_project_product_index(),
            DEFAULT_PROJECT_PRODUCT_INDEX,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PRODUCT_SPEC_README),
            super::init_surfaces::render_project_product_spec_readme(),
            DEFAULT_PROJECT_PRODUCT_SPEC_README,
        ),
        (
            project_root.join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE),
            fs::read_to_string(
                resolve_init_bootstrap_source_root()
                    .join("docs/framework/templates/feature-design-document.template.md"),
            )
            .map_err(|error| {
                format!("Failed to read framework feature-design template source: {error}")
            })?,
            DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
        ),
        (
            project_root.join(DEFAULT_PROJECT_ARCHITECTURE_DOC),
            super::init_surfaces::render_project_architecture_doc(),
            DEFAULT_PROJECT_ARCHITECTURE_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PROCESS_README),
            super::init_surfaces::render_project_process_readme(),
            DEFAULT_PROJECT_PROCESS_README,
        ),
        (
            project_root.join(DEFAULT_PROJECT_DECISIONS_DOC),
            super::init_surfaces::render_project_decisions_doc(answers),
            DEFAULT_PROJECT_DECISIONS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_ENVIRONMENTS_DOC),
            super::init_surfaces::render_project_environments_doc(project_root),
            DEFAULT_PROJECT_ENVIRONMENTS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_OPERATIONS_DOC),
            super::init_surfaces::render_project_operations_doc(),
            DEFAULT_PROJECT_OPERATIONS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_AGENT_SYSTEM_DOC),
            super::init_surfaces::render_project_agent_system_doc(),
            DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC),
            super::init_surfaces::render_project_codex_guide(),
            DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_DOC_TOOLING_DOC),
            super::init_surfaces::render_project_doc_tooling_map(),
            DEFAULT_PROJECT_DOC_TOOLING_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_RESEARCH_README),
            super::init_surfaces::render_project_research_readme(),
            DEFAULT_PROJECT_RESEARCH_README,
        ),
    ];

    for (path, contents, label) in generated_files {
        if write_file_if_missing_or_placeholder(&path, &contents)? {
            changed_files.push(label.to_string());
        }
    }

    Ok(changed_files)
}

fn write_file_if_missing_or_placeholder(target: &Path, contents: &str) -> Result<bool, String> {
    if target.exists() && !file_contains_placeholder(target) {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))?;
    Ok(true)
}

pub(crate) fn merge_project_activation_into_init_view(
    mut init_view: serde_json::Value,
    project_activation_view: &serde_json::Value,
) -> serde_json::Value {
    let activation_pending = project_activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true);
    let canonical_status = canonical_activation_status(
        project_activation_view["status"].as_str(),
        activation_pending,
    );
    if activation_pending {
        let mut minimum_commands = vec![
            serde_json::Value::String("vida project-activator --json".to_string()),
            serde_json::Value::String("vida docflow check --profile active-canon".to_string()),
        ];
        if let Some(example) = project_activation_view["interview"]["one_shot_example"].as_str() {
            minimum_commands.insert(0, serde_json::Value::String(example.to_string()));
        }
        init_view["minimum_commands"] = serde_json::Value::Array(minimum_commands);
        init_view["execution_gate"] = serde_json::json!({
            "activation_pending": true,
            "taskflow_admitted": false,
            "non_canonical_taskflow_surfaces_forbidden": ["vida taskflow", "external_taskflow_runtime"],
            "docflow_first": true
        });
        if init_view.get("source_mode_fallback_surface").is_some() {
            init_view["source_mode_fallback_surface"] =
                serde_json::Value::String("blocked_during_pending_activation".to_string());
        }
    }
    init_view["status"] = serde_json::Value::String(canonical_status.to_string());

    init_view["project_activation"] = serde_json::json!({
        "status": project_activation_view["status"],
        "activation_pending": project_activation_view["activation_pending"],
        "project_shape": project_activation_view["project_shape"],
        "triggers": project_activation_view["triggers"],
        "activation_algorithm": project_activation_view["activation_algorithm"],
        "normal_work_defaults": project_activation_view["normal_work_defaults"],
        "interview": project_activation_view["interview"],
        "host_environment": project_activation_view["host_environment"],
        "next_steps": project_activation_view["next_steps"],
    });
    init_view
}

pub(crate) fn write_project_activation_receipt(
    project_root: &Path,
    answers: Option<&ProjectActivationAnswers>,
    host_cli_system: Option<&str>,
    changed_files: &[String],
    host_template_materialized: bool,
) -> Result<Option<String>, String> {
    if changed_files.is_empty() && answers.is_none() && !host_template_materialized {
        return Ok(None);
    }
    let receipts_dir = project_root.join(".vida/receipts");
    std::fs::create_dir_all(&receipts_dir)
        .map_err(|error| format!("Failed to create {}: {error}", receipts_dir.display()))?;
    let now = time::OffsetDateTime::now_utc();
    let recorded_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt_name = format!("project-activation-{}.json", now.unix_timestamp());
    let receipt_path = receipts_dir.join(receipt_name);
    let normalized_host_cli_system = host_cli_system.and_then(normalize_host_cli_system);
    let registry = load_host_cli_system_registry_from_root(project_root);
    let host_cli_entry = normalized_host_cli_system
        .as_deref()
        .and_then(|system| registry.get(system));
    let default_agent_templates = host_cli_entry
        .map(|entry| {
            project_activator_host_cli_materialization::host_cli_entry_carrier_catalog(Some(entry))
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row["role_id"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let receipt = serde_json::json!({
        "receipt_kind": "project_activation",
        "recorded_at": recorded_at,
        "surface": "vida project-activator",
        "project_root": project_root.display().to_string(),
        "activation_mode": "bounded_interview_then_materialize",
        "docflow_first": true,
        "taskflow_admitted_while_pending": false,
        "non_canonical_taskflow_surfaces_forbidden_while_pending": ["vida taskflow", "external_taskflow_runtime"],
        "answers": answers.map(|answers| serde_json::json!({
            "project_id": answers.project_id,
            "project_title": answers.project_title,
            "user_communication_language": answers.user_communication_language,
            "reasoning_language": answers.reasoning_language,
            "documentation_language": answers.documentation_language,
            "todo_protocol_language": answers.todo_protocol_language,
        })),
        "host_cli_system": host_cli_system,
        "host_template_materialized": host_template_materialized,
        "default_host_agent_templates": default_agent_templates,
        "changed_files": changed_files,
        "log_note": "Use `vida docflow` for subsequent documentation validation/readiness work; project activation itself is a bounded onboarding/configuration path, not TaskFlow execution.",
    });
    let body =
        serde_json::to_string_pretty(&receipt).expect("project activation receipt should render");
    std::fs::write(&receipt_path, &body)
        .map_err(|error| format!("Failed to write {}: {error}", receipt_path.display()))?;
    std::fs::write(project_root.join(PROJECT_ACTIVATION_RECEIPT_LATEST), &body).map_err(
        |error| {
            format!(
                "Failed to write {}: {error}",
                project_root
                    .join(PROJECT_ACTIVATION_RECEIPT_LATEST)
                    .display()
            )
        },
    )?;
    Ok(Some(
        receipt_path
            .strip_prefix(project_root)
            .unwrap_or(&receipt_path)
            .display()
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::activation_status::canonical_activation_status;
    use super::db_first_activation_truth_read_back_error;
    use super::merge_project_activation_into_init_view;
    use crate::run;
    use crate::state_store::LauncherActivationSnapshot;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir, EnvVarGuard};
    use crate::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC;
    use crate::WORKER_SCORECARDS_STATE;
    use crate::WORKER_STRATEGY_STATE;
    use serde_json::json;
    use std::fs;
    use std::process::ExitCode;

    #[test]
    fn canonical_project_activation_status_normalizes_pending_compat_to_pending() {
        assert_eq!(
            canonical_activation_status(Some("pending_activation"), true),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some("pending_activation"), false),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some("pending"), false),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some("ready_enough_for_normal_work"), false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(
            canonical_activation_status(Some("unexpected-status"), false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(canonical_activation_status(None, true), "pending");
        assert_eq!(
            canonical_activation_status(None, false),
            "ready_enough_for_normal_work"
        );
    }

    #[test]
    fn canonical_project_activation_status_fails_closed_for_case_and_whitespace_drift() {
        assert_eq!(
            canonical_activation_status(Some(" PENDING_ACTIVATION "), false),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some(" READY_ENOUGH_FOR_NORMAL_WORK "), false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(
            canonical_activation_status(Some(" unexpected-status "), false),
            "ready_enough_for_normal_work"
        );
    }

    #[test]
    fn merge_project_activation_into_init_view_emits_canonical_status() {
        let init_view = serde_json::json!({
            "status": "ready_enough_for_normal_work",
            "minimum_commands": [],
        });
        let project_activation_view = json!({
            "status": "pending_activation",
            "activation_pending": true,
            "interview": {
                "one_shot_example": "vida docflow check --profile active-canon",
                "required_inputs": [],
            },
            "project_shape": {},
            "triggers": {},
            "activation_algorithm": {},
            "normal_work_defaults": {},
            "host_environment": {},
            "next_steps": [],
        });

        let merged = merge_project_activation_into_init_view(init_view, &project_activation_view);
        assert_eq!(merged["status"], "pending");
        assert!(merged["execution_gate"]["activation_pending"]
            .as_bool()
            .expect("execution gate activation_pending should exist"));
        assert_eq!(
            merged["execution_gate"]["taskflow_admitted"],
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            merged["minimum_commands"][0],
            "vida docflow check --profile active-canon"
        );
        assert_eq!(
            merged["project_activation"]["status"],
            project_activation_view["status"]
        );
        assert_eq!(
            merged["project_activation"]["activation_pending"],
            project_activation_view["activation_pending"]
        );
    }

    #[test]
    fn merge_project_activation_marks_init_pending_when_activation_is_incomplete() {
        let init_view = serde_json::json!({
            "status": "ready"
        });
        let project_activation_view = serde_json::json!({
            "status": "pending",
            "activation_pending": true,
            "project_shape": "bootstrapped",
            "triggers": {
                "config_state_incomplete": true
            },
            "activation_algorithm": {
                "taskflow_admitted_while_pending": false
            },
            "interview": {
                "required_inputs": []
            },
            "host_environment": {
                "selected_cli_system": serde_json::Value::Null
            },
            "next_steps": [
                "run `vida project-activator`"
            ]
        });

        let merged = merge_project_activation_into_init_view(init_view, &project_activation_view);

        assert_eq!(merged["status"], "pending");
        assert_eq!(merged["project_activation"]["activation_pending"], true);
        assert_eq!(
            merged["project_activation"]["triggers"]["config_state_incomplete"],
            true
        );
        assert_eq!(
            merged["project_activation"]["activation_algorithm"]["taskflow_admitted_while_pending"],
            false
        );
    }

    #[test]
    fn project_activator_view_uses_builtin_host_registry_without_overlay_systems() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let view = super::build_project_activator_view(harness.path());
        assert_eq!(
            view["host_environment"]["selected_cli_system"],
            serde_json::Value::Null
        );
        assert_eq!(view["host_environment"]["selection_required"], true);
        assert_eq!(view["host_environment"]["template_materialized"], false);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".codex");
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("codex")));
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
        assert!(view["host_environment"]["template_source_root"]
            .as_str()
            .expect("template source root should render")
            .ends_with("/.codex"));
    }

    #[test]
    fn project_activator_fails_closed_when_dispatch_alias_registry_is_configured_but_missing() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace(
            "dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml",
            "dispatch_aliases: .vida/project/agent-extensions/missing-dispatch-aliases.yaml",
        );
        fs::write(&config_path, updated).expect("config should be rewritten");

        assert_ne!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn db_first_activation_truth_read_back_allows_source_config_path_as_provenance_only() {
        let expected = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: "/tmp/project/vida.config.yaml".to_string(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({
                "role_selection": {
                    "fallback_role": "worker",
                    "mode": "native"
                },
                "agent_system": {}
            }),
            pack_router_keywords: serde_json::json!({}),
        };
        let read_back = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({
                "role_selection": {
                    "fallback_role": "worker",
                    "mode": "native"
                },
                "agent_system": {}
            }),
            pack_router_keywords: serde_json::json!({}),
        };

        assert_eq!(
            db_first_activation_truth_read_back_error(&expected, &read_back),
            None
        );
    }

    #[test]
    fn project_activator_reports_pending_activation_for_partial_project() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        fs::write(harness.path().join("README.md"), "# demo\n").expect("readme should exist");

        let view = super::build_project_activator_view(harness.path());

        assert_eq!(view["status"], "pending");
        assert_eq!(view["project_shape"], "partial");
        assert_eq!(view["activation_pending"], true);
        assert_eq!(
            view["triggers"]["initial_onboarding_missing"],
            serde_json::Value::Bool(true)
        );
    }

    #[test]
    fn project_activator_reports_pending_after_init_scaffold_without_docs() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let view = super::build_project_activator_view(harness.path());
        assert_eq!(view["status"], "pending");
        assert_eq!(view["activation_pending"], true);
        assert_eq!(view["triggers"]["sidecar_or_project_docs_too_thin"], false);
        assert_eq!(
            view["triggers"]["host_cli_unselected_or_unmaterialized"],
            true
        );
        assert_eq!(view["project_docs"]["config_has_placeholders"], true);
        assert_eq!(view["agent_extensions"]["bundle_ready"], true);
        assert_eq!(
            view["activation_algorithm"]["taskflow_admitted_while_pending"],
            false
        );
        assert_eq!(view["activation_algorithm"]["docflow_first"], true);
        assert!(
            view["interview"]["required_inputs"]
                .as_array()
                .expect("required inputs should render")
                .len()
                >= 3,
            "activation interview should require project id, language, and host CLI selection"
        );
    }

    #[test]
    fn project_activator_materializes_builtin_copy_tree_template_without_overlay_entry() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config = super::read_yaml_file_checked(&harness.path().join("vida.config.yaml"))
            .expect("project config should exist");
        let registry = super::host_cli_system_registry_with_fallback(Some(&config));
        let qwen_entry = registry
            .get("qwen")
            .expect("configured qwen template source should exist");
        let source = super::resolve_host_cli_template_source("qwen", Some(qwen_entry))
            .expect("configured qwen template source should resolve");
        assert!(source.ends_with(".qwen"));

        let runtime_root =
            super::materialize_host_cli_template(harness.path(), "qwen", Some(qwen_entry))
                .expect("configured qwen template should materialize");
        assert!(runtime_root.ends_with(".qwen"));
        assert!(harness.path().join(".qwen").is_dir());
    }

    #[test]
    fn project_activator_accepts_host_cli_selection_and_materializes_copy_tree_template() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        assert!(harness.path().join(".qwen").is_dir());
        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("cli_system: qwen"));

        let view = super::build_project_activator_view(harness.path());
        assert_eq!(view["host_environment"]["selected_cli_system"], "qwen");
        assert_eq!(
            view["host_environment"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".qwen");
        assert_eq!(
            view["normal_work_defaults"]["default_agent_topology"],
            serde_json::json!(["qwen-primary"])
        );
        assert_eq!(
            view["normal_work_defaults"]["carrier_tier_rates"]["qwen"],
            4
        );
        assert!(view["normal_work_defaults"]
            .get("local_codex_guide")
            .is_none());
        assert!(view["normal_work_defaults"]
            .get("codex_tier_rates")
            .is_none());
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("codex")));
    }

    #[test]
    fn project_activator_accepts_host_cli_selection_and_materializes_codex_template() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        assert!(harness.path().join(".codex/config.toml").is_file());
        assert!(harness.path().join(".codex/agents").is_dir());
        assert!(harness.path().join(WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(WORKER_STRATEGY_STATE).is_file());
        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("cli_system: codex"));
        assert!(config.contains("host_environment:"));
        assert!(config.contains("protocol_activation:\n  agent_system: true"));
        assert!(config.contains("agent_only_development: true"));
        assert!(config.contains("agent_system:\n  init_on_boot: true"));
        assert!(config.contains("mode: native"));
        assert!(config.contains("state_owner: orchestrator_only"));
        assert!(config.contains("max_parallel_agents: 4"));
        let view = super::build_project_activator_view(harness.path());
        let configured_agents = view["normal_work_defaults"]["default_agent_topology"]
            .as_array()
            .expect("default agent topology should render")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert!(!configured_agents.is_empty());

        for agent in configured_agents {
            let rendered =
                fs::read_to_string(harness.path().join(format!(".codex/agents/{agent}.toml")))
                    .unwrap_or_else(|_| panic!("{agent} agent should exist"));
            assert!(!rendered.contains("vida_tier"));
            assert!(!rendered.contains("vida_rate"));
            assert!(!rendered.contains("vida_reasoning_band"));
        }

        assert_eq!(view["host_environment"]["selected_cli_system"], "codex");
        assert_eq!(
            view["host_environment"]["selected_cli_execution_class"],
            "internal"
        );
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".codex");
        assert_eq!(
            view["normal_work_defaults"]["local_host_agent_guide"],
            DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC
        );
        assert!(view["normal_work_defaults"]
            .get("local_codex_guide")
            .is_none());
        assert!(view["normal_work_defaults"]
            .get("codex_tier_rates")
            .is_none());
    }

    #[test]
    fn project_activator_command_accepts_json_output() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::unset("VIDA_ROOT");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&["project-activator", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn project_activator_fails_closed_for_partial_activation_submission() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::unset("VIDA_ROOT");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::from(2)
        );

        assert!(!harness.path().join(".codex/config.toml").exists());
        let view = super::build_project_activator_view(harness.path());
        assert_eq!(view["activation_pending"], true);
        assert!(view["host_environment"]["selected_cli_system"].is_null());
    }

    #[test]
    fn project_activator_can_complete_bounded_activation_in_one_command() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::unset("VIDA_ROOT");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "ukrainian",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("id: vida-test"));
        assert!(config.contains("user_communication: ukrainian"));
        assert!(config.contains("documentation: ukrainian"));
        assert!(config.contains("cli_system: codex"));
        assert!(harness.path().join("docs/project-root-map.md").is_file());
        assert!(harness.path().join("docs/product/spec/README.md").is_file());
        assert!(harness
            .path()
            .join("docs/product/spec/templates/feature-design-document.template.md")
            .is_file());
        assert!(harness
            .path()
            .join("docs/process/documentation-tooling-map.md")
            .is_file());
        assert!(harness
            .path()
            .join("docs/process/codex-agent-configuration-guide.md")
            .is_file());
        assert!(harness.path().join(".codex/config.toml").is_file());
        assert!(harness.path().join(WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(WORKER_STRATEGY_STATE).is_file());
        assert!(
            harness
                .path()
                .join(".vida/receipts/project-activation.latest.json")
                .is_file(),
            "activation receipt should be written"
        );

        let view = super::build_project_activator_view(harness.path());
        assert_eq!(view["activation_pending"], false);
        assert_eq!(view["status"], "ready_enough_for_normal_work");
        assert_eq!(
            view["normal_work_defaults"]["documentation_first_for_feature_requests"],
            true
        );
        assert_eq!(
            view["normal_work_defaults"]["local_feature_design_template"],
            crate::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        );
    }

    #[test]
    fn project_activator_renders_codex_agent_files_from_overlay_and_keeps_template_contracts() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::unset("VIDA_ROOT");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let view = super::build_project_activator_view(harness.path());
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["agent_identity"],
            "execution_carrier"
        );
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["runtime_role_identity"],
            "activation_state"
        );
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["selection_rule"],
            "capability_first_then_score_guard_then_cheapest_tier"
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]["snapshot"]
                .as_str()
                .unwrap_or_default()
                .contains("vida taskflow consume agent-system --json")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["carrier_catalog"]
                .as_str()
                .unwrap_or_default()
                .contains(".snapshot.carriers")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["runtime_roles"]
                .as_str()
                .unwrap_or_default()
                .contains("roles")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]["scores"]
                .as_str()
                .unwrap_or_default()
                .contains(".snapshot.worker_strategy.agents")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["selection_preview"]
                .as_str()
                .unwrap_or_default()
                .contains(".payload.taskflow_handoff_plan.runtime_assignment")
        );

        let config = fs::read_to_string(harness.path().join(".codex/config.toml"))
            .expect("rendered codex config should exist");
        let configured_agents = super::build_project_activator_view(harness.path())
            ["normal_work_defaults"]["default_agent_topology"]
            .as_array()
            .expect("default agent topology should render")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert!(!configured_agents.is_empty());
        for agent in &configured_agents {
            assert!(config.contains(&format!("[agents.{agent}]")));
        }
        assert!(!config.contains("[agents.development_implementer]"));
        assert!(!config.contains("[agents.development_coach]"));
        assert!(!config.contains("[agents.development_verifier]"));
        assert!(!config.contains("[agents.development_escalation]"));

        for agent in &configured_agents {
            let rendered =
                fs::read_to_string(harness.path().join(format!(".codex/agents/{agent}.toml")))
                    .unwrap_or_else(|_| panic!("{agent} agent should exist"));
            assert!(!rendered.contains("vida_tier"));
            assert!(!rendered.contains("vida_rate"));
            assert!(!rendered.contains("vida_reasoning_band"));
            assert!(!rendered.contains("vida_default_runtime_role"));
            assert!(!rendered.contains("vida_runtime_roles"));
            assert!(!rendered.contains("vida_task_classes"));
        }

        assert!(!harness
            .path()
            .join(".codex/agents/development_implementer.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_coach.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_verifier.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_escalation.toml")
            .exists());
    }

    #[test]
    fn project_activator_reports_ready_when_bootstrap_and_docs_exist() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".codex/agents")).expect(".codex agents dir should exist");
        fs::create_dir_all(root.join(".vida/config")).expect(".vida/config dir should exist");
        fs::create_dir_all(root.join(".vida/db")).expect(".vida/db dir should exist");
        fs::create_dir_all(root.join(".vida/cache")).expect(".vida/cache dir should exist");
        fs::create_dir_all(root.join(".vida/framework")).expect(".vida/framework dir should exist");
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect(".vida/project agent extensions dir should exist");
        fs::create_dir_all(root.join(".vida/receipts")).expect(".vida/receipts dir should exist");
        fs::create_dir_all(root.join(".vida/runtime")).expect(".vida/runtime dir should exist");
        fs::create_dir_all(root.join(".vida/scratchpad"))
            .expect(".vida/scratchpad dir should exist");
        fs::create_dir_all(root.join("docs/product")).expect("product docs dir should exist");
        fs::create_dir_all(root.join("docs/process")).expect("process docs dir should exist");
        fs::write(root.join("AGENTS.md"), "# framework\n").expect("agents should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "project docs map\n")
            .expect("sidecar should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "project:\n  id: demo\n",
                "language_policy:\n",
                "  user_communication: english\n",
                "  reasoning: english\n",
                "  documentation: english\n",
                "  todo_protocol: english\n",
                "host_environment:\n",
                "  cli_system: codex\n",
                "  systems:\n",
                "    codex:\n",
                "      enabled: true\n",
                "      execution_class: internal\n",
                "      materialization_mode: codex_toml_catalog_render\n",
                "      template_root: .codex\n",
                "      runtime_root: .codex\n",
                "      carriers: {}\n",
            ),
        )
        .expect("config should exist");
        fs::write(root.join(".codex/config.toml"), "[agents]\n")
            .expect("codex config should exist");
        fs::write(root.join("docs/project-root-map.md"), "# root map\n")
            .expect("project root map should exist");
        fs::write(root.join("docs/product/index.md"), "# product\n")
            .expect("product index should exist");
        fs::create_dir_all(root.join("docs/product/spec/templates"))
            .expect("product spec template dir should exist");
        fs::write(
            root.join("docs/product/spec/README.md"),
            "# product spec guide\n",
        )
        .expect("product spec guide should exist");
        fs::write(
            root.join("docs/product/spec/templates/feature-design-document.template.md"),
            "# feature design template\n",
        )
        .expect("feature design template should exist");
        fs::write(root.join("docs/process/README.md"), "# process\n")
            .expect("process readme should exist");
        fs::write(
            root.join("docs/process/codex-agent-configuration-guide.md"),
            "# codex guide\n",
        )
        .expect("codex guide should exist");
        fs::write(
            root.join("docs/process/documentation-tooling-map.md"),
            "# tooling\n",
        )
        .expect("documentation tooling map should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/README.md"),
            "# runtime agent extensions\n",
        )
        .expect("runtime readme should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let view = super::build_project_activator_view(root);

        assert_eq!(view["status"], "ready_enough_for_normal_work");
        assert_eq!(view["project_shape"], "bootstrapped");
        assert_eq!(view["activation_pending"], false);
        assert_eq!(view["host_environment"]["selected_cli_system"], "codex");
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(
            view["next_steps"][0],
            "activation looks ready enough for normal orchestrator and worker initialization"
        );
    }
}
