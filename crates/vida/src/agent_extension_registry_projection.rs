use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) const RUNTIME_DISPATCH_ALIASES_PROJECTION: &str =
    ".vida/project/agent-extensions/dispatch-aliases.yaml";

#[derive(Clone, Copy, Debug)]
struct RegistryProjectionSpec {
    label: &'static str,
    config_key: &'static str,
    registry_key: &'static str,
    id_field: &'static str,
    runtime_projection_path: &'static str,
}

const REGISTRY_PROJECTION_SPECS: &[RegistryProjectionSpec] = &[
    RegistryProjectionSpec {
        label: "role",
        config_key: "roles",
        registry_key: "roles",
        id_field: "role_id",
        runtime_projection_path: ".vida/project/agent-extensions/roles.yaml",
    },
    RegistryProjectionSpec {
        label: "skill",
        config_key: "skills",
        registry_key: "skills",
        id_field: "skill_id",
        runtime_projection_path: ".vida/project/agent-extensions/skills.yaml",
    },
    RegistryProjectionSpec {
        label: "profile",
        config_key: "profiles",
        registry_key: "profiles",
        id_field: "profile_id",
        runtime_projection_path: ".vida/project/agent-extensions/profiles.yaml",
    },
    RegistryProjectionSpec {
        label: "flow",
        config_key: "flows",
        registry_key: "flow_sets",
        id_field: "flow_id",
        runtime_projection_path: ".vida/project/agent-extensions/flows.yaml",
    },
    RegistryProjectionSpec {
        label: "pack",
        config_key: "packs",
        registry_key: "packs",
        id_field: "pack_id",
        runtime_projection_path: ".vida/project/agent-extensions/packs.yaml",
    },
    RegistryProjectionSpec {
        label: "command",
        config_key: "commands",
        registry_key: "commands",
        id_field: "command_id",
        runtime_projection_path: ".vida/project/agent-extensions/commands.yaml",
    },
    RegistryProjectionSpec {
        label: "dispatch alias",
        config_key: "dispatch_aliases",
        registry_key: "dispatch_aliases",
        id_field: "alias_id",
        runtime_projection_path: RUNTIME_DISPATCH_ALIASES_PROJECTION,
    },
    RegistryProjectionSpec {
        label: "hook template",
        config_key: "hook_templates",
        registry_key: "hook_templates",
        id_field: "template_id",
        runtime_projection_path: ".vida/project/agent-extensions/hook-templates.yaml",
    },
];

#[derive(Clone, Debug)]
pub(crate) struct DispatchAliasProjectionParity {
    pub(crate) registry_label: String,
    pub(crate) config_key: String,
    pub(crate) configured_source_path: String,
    pub(crate) source_path: PathBuf,
    pub(crate) runtime_projection_path: PathBuf,
    pub(crate) source_alias_count: usize,
    pub(crate) runtime_alias_count: usize,
    pub(crate) missing_runtime_aliases: Vec<String>,
    pub(crate) extra_runtime_aliases: Vec<String>,
    pub(crate) content_matches: bool,
    pub(crate) in_sync: bool,
}

impl DispatchAliasProjectionParity {
    pub(crate) fn stale_summary(&self) -> String {
        let missing = if self.missing_runtime_aliases.is_empty() {
            "none".to_string()
        } else {
            self.missing_runtime_aliases.join(", ")
        };
        let extra = if self.extra_runtime_aliases.is_empty() {
            "none".to_string()
        } else {
            self.extra_runtime_aliases.join(", ")
        };
        format!(
            "{} runtime projection is stale: configured source `{}` has {} rows, runtime projection `{}` has {} rows; missing runtime ids: {missing}; extra runtime ids: {extra}; run `vida project-activator --repair` to refresh the generated base projection",
            self.registry_label,
            self.configured_source_path,
            self.source_alias_count,
            self.runtime_projection_path.display(),
            self.runtime_alias_count
        )
    }
}

fn registry_ids(
    registry: &serde_yaml::Value,
    registry_key: &str,
    id_field: &str,
) -> BTreeSet<String> {
    crate::registry_ids_by_key(registry, registry_key, id_field)
        .into_iter()
        .collect()
}

fn validate_registry_source_path(
    root: &Path,
    configured_path: &str,
    spec: RegistryProjectionSpec,
) -> Result<PathBuf, String> {
    let configured = Path::new(configured_path);
    if configured_path.trim().is_empty()
        || configured.is_absolute()
        || configured.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "refusing configured {} source `{configured_path}`: source path must be a project-relative safe path",
            spec.label
        ));
    }
    let source_path = root.join(configured);
    let metadata = std::fs::symlink_metadata(&source_path).map_err(|error| {
        format!(
            "failed to inspect configured {} source `{}`: {error}",
            spec.label, configured_path
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "refusing configured {} source `{configured_path}`: source must be a regular file",
            spec.label
        ));
    }
    let canonical_root = root.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize project root `{}`: {error}",
            root.display()
        )
    })?;
    let canonical_source = source_path.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize configured {} source `{configured_path}`: {error}",
            spec.label
        )
    })?;
    if !canonical_source.starts_with(&canonical_root) {
        return Err(format!(
            "refusing configured {} source `{configured_path}`: source resolves outside project root",
            spec.label
        ));
    }
    Ok(source_path)
}

fn validate_registry_document(
    registry: &serde_yaml::Value,
    spec: RegistryProjectionSpec,
    path: &Path,
) -> Result<(), String> {
    let Some(mapping) = registry.as_mapping() else {
        return Err(format!(
            "malformed {} registry `{}`: expected a YAML mapping",
            spec.label,
            path.display()
        ));
    };
    let key = serde_yaml::Value::String(spec.registry_key.to_string());
    let Some(serde_yaml::Value::Sequence(rows)) = mapping.get(&key) else {
        return Err(format!(
            "malformed {} registry `{}`: `{}` must be a YAML sequence",
            spec.label,
            path.display(),
            spec.registry_key
        ));
    };
    let mut ids = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(row_mapping) = row.as_mapping() else {
            return Err(format!(
                "malformed {} registry `{}`: row {index} must be a YAML mapping",
                spec.label,
                path.display()
            ));
        };
        let id = row_mapping
            .get(serde_yaml::Value::String(spec.id_field.to_string()))
            .and_then(|value| crate::yaml_string(Some(value)))
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                format!(
                    "malformed {} registry `{}`: row {index} requires non-empty `{}`",
                    spec.label,
                    path.display(),
                    spec.id_field
                )
            })?;
        if !ids.insert(id.clone()) {
            return Err(format!(
                "duplicate {} id `{id}` in registry `{}`",
                spec.label,
                path.display()
            ));
        }
    }
    Ok(())
}

fn same_configured_registry_projection_path(
    configured_path: &str,
    resolved_path: &Path,
    root: &Path,
    runtime_projection_path: &str,
) -> bool {
    let configured = configured_path.replace('\\', "/");
    if configured == runtime_projection_path {
        return true;
    }
    resolved_path == root.join(runtime_projection_path)
}

fn registry_projection_parity(
    config: &serde_yaml::Value,
    root: &Path,
    spec: RegistryProjectionSpec,
) -> Result<Option<DispatchAliasProjectionParity>, String> {
    let Some(configured_source_path) = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", spec.config_key],
    )) else {
        return Ok(None);
    };
    let source_path = validate_registry_source_path(root, &configured_source_path, spec)?;
    if same_configured_registry_projection_path(
        &configured_source_path,
        &source_path,
        root,
        spec.runtime_projection_path,
    ) {
        return Ok(None);
    }

    let runtime_projection_path = root.join(spec.runtime_projection_path);
    let source_registry = crate::project_activator_surface::read_yaml_file_checked(&source_path)
        .map_err(|error| {
            format!(
                "failed to load configured {} source `{}`: {error}",
                spec.label, configured_source_path
            )
        })?;
    validate_registry_document(&source_registry, spec, &source_path)?;
    let (runtime_registry, runtime_raw) = match std::fs::read_to_string(&runtime_projection_path) {
        Ok(raw) => {
            let registry = serde_yaml::from_str(&raw).map_err(|error| {
                format!(
                    "failed to parse runtime {} projection `{}`: {error}",
                    spec.label, spec.runtime_projection_path
                )
            })?;
            validate_registry_document(&registry, spec, &runtime_projection_path)?;
            (registry, raw)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (serde_yaml::Value::Null, String::new())
        }
        Err(error) => {
            return Err(format!(
                "failed to read runtime {} projection `{}`: {error}",
                spec.label,
                runtime_projection_path.display()
            ));
        }
    };
    let source_aliases = registry_ids(&source_registry, spec.registry_key, spec.id_field);
    let runtime_aliases = registry_ids(&runtime_registry, spec.registry_key, spec.id_field);
    let source_raw = std::fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "failed to read configured {} source `{}`: {error}",
            spec.label,
            source_path.display()
        )
    })?;
    let missing_runtime_aliases = source_aliases
        .difference(&runtime_aliases)
        .cloned()
        .collect::<Vec<_>>();
    let extra_runtime_aliases = runtime_aliases
        .difference(&source_aliases)
        .cloned()
        .collect::<Vec<_>>();
    let content_matches = source_raw.trim_end() == runtime_raw.trim_end();
    let in_sync =
        content_matches && missing_runtime_aliases.is_empty() && extra_runtime_aliases.is_empty();

    Ok(Some(DispatchAliasProjectionParity {
        registry_label: spec.label.to_string(),
        config_key: spec.config_key.to_string(),
        configured_source_path,
        source_path,
        runtime_projection_path,
        source_alias_count: source_aliases.len(),
        runtime_alias_count: runtime_aliases.len(),
        missing_runtime_aliases,
        extra_runtime_aliases,
        content_matches,
        in_sync,
    }))
}

pub(crate) fn dispatch_alias_projection_parity(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<Option<DispatchAliasProjectionParity>, String> {
    registry_projection_parity(
        config,
        root,
        *REGISTRY_PROJECTION_SPECS
            .iter()
            .find(|spec| spec.config_key == "dispatch_aliases")
            .expect("dispatch alias projection spec should exist"),
    )
}

pub(crate) fn agent_extension_registry_projection_parities(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<Vec<DispatchAliasProjectionParity>, String> {
    let mut parities = Vec::new();
    for spec in REGISTRY_PROJECTION_SPECS {
        if let Some(parity) = registry_projection_parity(config, root, *spec)? {
            parities.push(parity);
        }
    }
    Ok(parities)
}

struct StagedProjectionFile {
    path: PathBuf,
    temporary_path: PathBuf,
    backup_path: Option<PathBuf>,
    committed: bool,
}

fn stage_runtime_projection_file(
    path: &Path,
    contents: &str,
    label: &str,
) -> Result<StagedProjectionFile, String> {
    if let Some(parent) = path.parent() {
        crate::ensure_dir(parent)?;
    }
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "refusing to refresh runtime {label} projection `{}` because it is a symlink",
                path.display()
            ));
        }
    }

    let parent = path.parent().ok_or_else(|| {
        format!(
            "runtime {label} projection `{}` has no parent",
            path.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "runtime {label} projection `{}` has no file name",
                path.display()
            )
        })?;
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));

    let write_result = (|| -> Result<StagedProjectionFile, String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "failed to create temporary runtime {label} projection `{}`: {error}",
                    temp_path.display()
                )
            })?;
        file.write_all(contents.as_bytes()).map_err(|error| {
            format!(
                "failed to write temporary runtime {label} projection `{}`: {error}",
                temp_path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "failed to flush temporary runtime {label} projection `{}`: {error}",
                temp_path.display()
            )
        })?;
        Ok(StagedProjectionFile {
            path: path.to_path_buf(),
            temporary_path: temp_path.clone(),
            backup_path: None,
            committed: false,
        })
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
}

fn rollback_staged_projection_files(staged: &[StagedProjectionFile]) {
    for item in staged.iter().rev() {
        if item.committed {
            let _ = std::fs::remove_file(&item.path);
        }
        if let Some(backup_path) = &item.backup_path {
            let _ = std::fs::rename(backup_path, &item.path);
        }
        let _ = std::fs::remove_file(&item.temporary_path);
    }
}

fn atomic_replace_runtime_projection_files(
    pending: &[(PathBuf, String, String)],
) -> Result<(), String> {
    let mut staged = Vec::new();
    for (path, contents, label) in pending {
        match stage_runtime_projection_file(path, contents, label) {
            Ok(file) => staged.push(file),
            Err(error) => {
                rollback_staged_projection_files(&staged);
                return Err(error);
            }
        }
    }

    for index in 0..staged.len() {
        let path = staged[index].path.clone();
        if std::fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            rollback_staged_projection_files(&staged);
            return Err(format!(
                "refusing to refresh runtime projection `{}` because it became a symlink",
                path.display()
            ));
        }
        if path.exists() {
            let backup_path = path.with_file_name(format!(
                ".{}.{}.backup",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("projection"),
                std::process::id()
            ));
            if let Err(error) = std::fs::rename(&path, &backup_path) {
                rollback_staged_projection_files(&staged);
                return Err(format!(
                    "failed to stage rollback backup for runtime projection `{}`: {error}",
                    path.display()
                ));
            }
            staged[index].backup_path = Some(backup_path);
        }
        let temporary_path = staged[index].temporary_path.clone();
        if let Err(error) = std::fs::rename(&temporary_path, &path) {
            rollback_staged_projection_files(&staged);
            return Err(format!(
                "failed to replace runtime projection `{}`: {error}",
                path.display()
            ));
        }
        staged[index].committed = true;
    }

    for item in &staged {
        if let Some(backup_path) = &item.backup_path {
            let _ = std::fs::remove_file(backup_path);
        }
    }
    Ok(())
}

pub(crate) fn refresh_runtime_dispatch_alias_projection_from_configured_source(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<Option<DispatchAliasProjectionParity>, String> {
    let Some(parity) = dispatch_alias_projection_parity(config, root)? else {
        return Ok(None);
    };
    if parity.in_sync {
        return Ok(Some(parity));
    }

    let source = std::fs::read_to_string(&parity.source_path).map_err(|error| {
        format!(
            "failed to read configured dispatch alias source `{}`: {error}",
            parity.source_path.display()
        )
    })?;
    atomic_replace_runtime_projection_files(&[(
        parity.runtime_projection_path.clone(),
        source,
        "dispatch alias".to_string(),
    )])?;
    dispatch_alias_projection_parity(config, root)
}

pub(crate) fn refresh_runtime_agent_extension_projections_from_configured_sources(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<Vec<DispatchAliasProjectionParity>, String> {
    let mut pending = Vec::new();
    for spec in REGISTRY_PROJECTION_SPECS {
        let Some(parity) = registry_projection_parity(config, root, *spec)? else {
            continue;
        };
        if parity.in_sync {
            continue;
        }
        let source = std::fs::read_to_string(&parity.source_path).map_err(|error| {
            format!(
                "failed to read configured {} source `{}`: {error}",
                parity.registry_label,
                parity.source_path.display()
            )
        })?;
        pending.push((
            parity.runtime_projection_path.clone(),
            source,
            parity.registry_label.clone(),
        ));
    }
    if !pending.is_empty() {
        atomic_replace_runtime_projection_files(&pending)?;
    }
    agent_extension_registry_projection_parities(config, root)
}

pub(crate) struct AgentExtensionValidationConfig {
    pub(crate) require_registry_files: bool,
    pub(crate) require_profile_resolution: bool,
    pub(crate) require_flow_resolution: bool,
    pub(crate) require_framework_role_compatibility: bool,
    pub(crate) require_skill_role_compatibility: bool,
}

pub(crate) struct AgentExtensionRegistryProjection {
    pub(crate) roles_registry: serde_yaml::Value,
    pub(crate) skills_registry: serde_yaml::Value,
    pub(crate) profiles_registry: serde_yaml::Value,
    pub(crate) flows_registry: serde_yaml::Value,
    pub(crate) packs_registry: serde_yaml::Value,
    pub(crate) commands_registry: serde_yaml::Value,
    pub(crate) dispatch_aliases_registry: serde_yaml::Value,
    pub(crate) enabled_project_roles: Vec<String>,
    pub(crate) enabled_project_skills: Vec<String>,
    pub(crate) enabled_project_profiles: Vec<String>,
    pub(crate) enabled_project_flows: Vec<String>,
    pub(crate) selected_host_cli_system: Option<String>,
    pub(crate) host_cli_system_registry: HashMap<String, serde_yaml::Value>,
    pub(crate) dispatch_aliases_path: Option<String>,
    pub(crate) validation: AgentExtensionValidationConfig,
    pub(crate) validation_errors: Vec<String>,
}

fn load_optional_registry_projection(
    root: &Path,
    path: Option<&str>,
    registry_key: &str,
    id_field: &str,
    registry_label: &str,
    require_registry_files: bool,
    missing_path_error: Option<&str>,
    validation_errors: &mut Vec<String>,
) -> serde_yaml::Value {
    match path {
        Some(path) => match crate::project_activator_surface::load_registry_projection(
            root,
            Some(path),
            registry_key,
            id_field,
            registry_label,
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => {
            if let Some(error) = missing_path_error {
                validation_errors.push(error.to_string());
            }
            serde_yaml::Value::Null
        }
    }
}

pub(crate) fn build_agent_extension_registry_projection(
    config: &serde_yaml::Value,
    root: &Path,
) -> AgentExtensionRegistryProjection {
    let overlay = crate::project_overlay_config(config).agent_extensions;
    let configured_enabled_project_roles = overlay.enabled_project_roles;
    let configured_enabled_project_profiles = overlay.enabled_project_profiles;
    let configured_enabled_project_flows = overlay.enabled_project_flows;
    let registry_paths = overlay.registries;
    let roles_path = registry_paths.roles;
    let skills_path = registry_paths.skills;
    let profiles_path = registry_paths.profiles;
    let flows_path = registry_paths.flows;
    let packs_path = registry_paths.packs;
    let commands_path = registry_paths.commands;
    let dispatch_aliases_path = registry_paths.dispatch_aliases;
    let validation_flags = overlay.validation;
    let validation = AgentExtensionValidationConfig {
        require_registry_files: validation_flags.require_registry_files,
        require_profile_resolution: validation_flags.require_profile_resolution,
        require_flow_resolution: validation_flags.require_flow_resolution,
        require_framework_role_compatibility: validation_flags.require_framework_role_compatibility,
        require_skill_role_compatibility: validation_flags.require_skill_role_compatibility,
    };
    let mut validation_errors = Vec::new();
    let roles_registry = load_optional_registry_projection(
        root,
        roles_path.as_deref(),
        "roles",
        "role_id",
        "roles",
        validation.require_registry_files,
        if validation.require_registry_files && !configured_enabled_project_roles.is_empty() {
            Some("agent extension roles registry path is required but missing")
        } else {
            None
        },
        &mut validation_errors,
    );
    let skills_registry = load_optional_registry_projection(
        root,
        skills_path.as_deref(),
        "skills",
        "skill_id",
        "skills",
        validation.require_registry_files,
        None,
        &mut validation_errors,
    );
    let profiles_registry = load_optional_registry_projection(
        root,
        profiles_path.as_deref(),
        "profiles",
        "profile_id",
        "profiles",
        validation.require_registry_files,
        if validation.require_registry_files && !configured_enabled_project_profiles.is_empty() {
            Some("agent extension profiles registry path is required but missing")
        } else {
            None
        },
        &mut validation_errors,
    );
    let flows_registry = load_optional_registry_projection(
        root,
        flows_path.as_deref(),
        "flow_sets",
        "flow_id",
        "flows",
        validation.require_registry_files,
        if validation.require_registry_files && !configured_enabled_project_flows.is_empty() {
            Some("agent extension flows registry path is required but missing")
        } else {
            None
        },
        &mut validation_errors,
    );
    let dispatch_aliases_registry = load_optional_registry_projection(
        root,
        dispatch_aliases_path.as_deref(),
        "dispatch_aliases",
        "alias_id",
        "dispatch_aliases",
        validation.require_registry_files,
        None,
        &mut validation_errors,
    );
    let packs_registry = load_optional_registry_projection(
        root,
        packs_path.as_deref(),
        "packs",
        "pack_id",
        "packs",
        validation.require_registry_files,
        None,
        &mut validation_errors,
    );
    let commands_registry = load_optional_registry_projection(
        root,
        commands_path.as_deref(),
        "commands",
        "command_id",
        "commands",
        validation.require_registry_files,
        None,
        &mut validation_errors,
    );
    let enabled_project_roles = crate::effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_roles"],
        &roles_registry,
        "roles",
        "role_id",
    );
    let enabled_project_skills = crate::effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_skills"],
        &skills_registry,
        "skills",
        "skill_id",
    );
    let enabled_project_profiles = crate::effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_profiles"],
        &profiles_registry,
        "profiles",
        "profile_id",
    );
    let enabled_project_flows = crate::effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_flows"],
        &flows_registry,
        "flow_sets",
        "flow_id",
    );
    let selected_host_cli_system = crate::yaml_lookup(config, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(crate::project_activator_surface::normalize_host_cli_system);
    let host_cli_system_registry =
        crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(config));

    AgentExtensionRegistryProjection {
        roles_registry,
        skills_registry,
        profiles_registry,
        flows_registry,
        packs_registry,
        commands_registry,
        dispatch_aliases_registry,
        enabled_project_roles,
        enabled_project_skills,
        enabled_project_profiles,
        enabled_project_flows,
        selected_host_cli_system,
        host_cli_system_registry,
        dispatch_aliases_path,
        validation,
        validation_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dispatch_alias_projection_parity,
        refresh_runtime_agent_extension_projections_from_configured_sources,
        refresh_runtime_dispatch_alias_projection_from_configured_source,
    };
    use crate::temp_state::TempStateHarness;
    use std::fs;

    fn write_dispatch_alias_projection_fixture(root: &std::path::Path) -> serde_yaml::Value {
        fs::create_dir_all(root.join("docs/process/agent-extensions"))
            .expect("docs agent extensions dir should exist");
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("docs/process/agent-extensions/dispatch-aliases.yaml"),
            concat!(
                "version: 1\n",
                "dispatch_aliases:\n",
                "  - alias_id: development_implementer\n",
                "    carrier_tier: junior\n",
                "  - alias_id: development_test_author\n",
                "    carrier_tier: middle\n",
            ),
        )
        .expect("source dispatch aliases should be written");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            concat!(
                "version: 1\n",
                "dispatch_aliases:\n",
                "  - alias_id: development_implementer\n",
                "    carrier_tier: junior\n",
            ),
        )
        .expect("runtime dispatch aliases should be written");
        serde_yaml::from_str(
            r#"
agent_extensions:
  registries:
    dispatch_aliases: docs/process/agent-extensions/dispatch-aliases.yaml
"#,
        )
        .expect("config should parse")
    }

    fn write_missing_command_projection_fixture(root: &std::path::Path) -> serde_yaml::Value {
        fs::create_dir_all(root.join("docs/process/agent-extensions"))
            .expect("docs agent extensions dir should exist");
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("docs/process/agent-extensions/commands.yaml"),
            concat!(
                "version: 1\n",
                "commands:\n",
                "  - command_id: agent-init-worker\n",
                "    command: vida agent-init --role worker --json\n",
            ),
        )
        .expect("source commands should be written");
        serde_yaml::from_str(
            r#"
agent_extensions:
  registries:
    commands: docs/process/agent-extensions/commands.yaml
"#,
        )
        .expect("config should parse")
    }

    #[test]
    fn agent_extension_projection_detects_stale_dispatch_alias_source_runtime_mismatch() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config = write_dispatch_alias_projection_fixture(harness.path());

        let parity = dispatch_alias_projection_parity(&config, harness.path())
            .expect("parity check should run")
            .expect("configured docs source should require parity");

        assert!(!parity.in_sync);
        assert_eq!(parity.source_alias_count, 2);
        assert_eq!(parity.runtime_alias_count, 1);
        assert_eq!(
            parity.missing_runtime_aliases,
            vec!["development_test_author".to_string()]
        );
    }

    #[test]
    fn agent_extension_projection_refreshes_runtime_dispatch_alias_base_from_source() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config = write_dispatch_alias_projection_fixture(harness.path());

        let parity = refresh_runtime_dispatch_alias_projection_from_configured_source(
            &config,
            harness.path(),
        )
        .expect("projection refresh should run")
        .expect("configured docs source should require parity");

        assert!(parity.in_sync);
        assert_eq!(parity.source_alias_count, parity.runtime_alias_count);
        let refreshed = fs::read_to_string(
            harness
                .path()
                .join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
        )
        .expect("runtime projection should be readable");
        assert!(refreshed.contains("alias_id: development_test_author"));

        let repeated = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect("repeated projection refresh should be deterministic");
        assert!(repeated.iter().all(|parity| parity.in_sync));
        assert_eq!(
            refreshed,
            fs::read_to_string(
                harness
                    .path()
                    .join(".vida/project/agent-extensions/dispatch-aliases.yaml")
            )
            .expect("repeated runtime projection should be readable")
        );
    }

    #[test]
    fn agent_extension_projection_refresh_creates_missing_runtime_command_projection() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config = write_missing_command_projection_fixture(harness.path());
        let runtime_projection = harness
            .path()
            .join(".vida/project/agent-extensions/commands.yaml");

        assert!(!runtime_projection.exists());

        let parities = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect("missing runtime command projection should be repairable");
        let command_parity = parities
            .iter()
            .find(|parity| parity.config_key == "commands")
            .expect("command projection parity should be reported");

        assert!(command_parity.in_sync);
        assert_eq!(command_parity.source_alias_count, 1);
        assert_eq!(command_parity.runtime_alias_count, 1);
        assert!(runtime_projection.is_file());
        assert!(
            fs::read_to_string(runtime_projection)
                .expect("runtime command projection should be readable")
                .contains("command_id: agent-init-worker")
        );
    }

    #[test]
    fn agent_extension_projection_fails_closed_on_duplicate_source_ids_without_writes() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        fs::create_dir_all(harness.path().join("docs/process/agent-extensions"))
            .expect("source registry dir should exist");
        fs::create_dir_all(harness.path().join(".vida/project/agent-extensions"))
            .expect("runtime registry dir should exist");
        fs::write(
            harness
                .path()
                .join("docs/process/agent-extensions/roles.yaml"),
            "version: 1\nroles:\n  - role_id: duplicate\n  - role_id: duplicate\n",
        )
        .expect("duplicate source registry should be written");
        let runtime_path = harness
            .path()
            .join(".vida/project/agent-extensions/roles.yaml");
        fs::write(&runtime_path, "version: 1\nroles:\n  - role_id: stable\n")
            .expect("runtime registry should be written");
        let config = serde_yaml::from_str(
            r#"
agent_extensions:
  registries:
    roles: docs/process/agent-extensions/roles.yaml
"#,
        )
        .expect("config should parse");

        let error = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect_err("duplicate ids should fail closed");

        assert!(error.contains("duplicate role id `duplicate`"));
        assert_eq!(
            fs::read_to_string(runtime_path).expect("runtime registry should remain readable"),
            "version: 1\nroles:\n  - role_id: stable\n"
        );
    }

    #[test]
    fn agent_extension_projection_rejects_unsafe_source_path_without_writes() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime_path = harness
            .path()
            .join(".vida/project/agent-extensions/roles.yaml");
        fs::create_dir_all(runtime_path.parent().expect("runtime parent should exist"))
            .expect("runtime registry dir should exist");
        fs::write(&runtime_path, "version: 1\nroles: []\n")
            .expect("runtime registry should be written");
        let config = serde_yaml::from_str(
            r#"
agent_extensions:
  registries:
    roles: ../outside-roles.yaml
"#,
        )
        .expect("config should parse");

        let error = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect_err("unsafe source path should fail closed");

        assert!(error.contains("project-relative safe path"));
        assert_eq!(
            fs::read_to_string(runtime_path).expect("runtime registry should remain readable"),
            "version: 1\nroles: []\n"
        );
    }

    #[test]
    fn agent_extension_projection_validates_all_families_before_any_write() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        fs::create_dir_all(harness.path().join("docs/process/agent-extensions"))
            .expect("source registry dir should exist");
        fs::create_dir_all(harness.path().join(".vida/project/agent-extensions"))
            .expect("runtime registry dir should exist");
        fs::write(
            harness
                .path()
                .join("docs/process/agent-extensions/roles.yaml"),
            "version: 1\nroles:\n  - role_id: refreshed\n",
        )
        .expect("valid roles source should be written");
        fs::write(
            harness
                .path()
                .join("docs/process/agent-extensions/skills.yaml"),
            "version: 1\nskills: malformed\n",
        )
        .expect("malformed skills source should be written");
        let roles_runtime = harness
            .path()
            .join(".vida/project/agent-extensions/roles.yaml");
        let skills_runtime = harness
            .path()
            .join(".vida/project/agent-extensions/skills.yaml");
        fs::write(&roles_runtime, "version: 1\nroles:\n  - role_id: stable\n")
            .expect("roles runtime should be written");
        fs::write(&skills_runtime, "version: 1\nskills: []\n")
            .expect("skills runtime should be written");
        let config = serde_yaml::from_str(
            r#"
agent_extensions:
  registries:
    roles: docs/process/agent-extensions/roles.yaml
    skills: docs/process/agent-extensions/skills.yaml
"#,
        )
        .expect("config should parse");

        let error = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect_err("malformed later family should fail closed");

        assert!(error.contains("malformed skill registry"));
        assert!(
            fs::read_to_string(roles_runtime)
                .expect("roles runtime should remain readable")
                .contains("role_id: stable")
        );
        assert_eq!(
            fs::read_to_string(skills_runtime).expect("skills runtime should remain readable"),
            "version: 1\nskills: []\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn agent_extension_projection_refresh_rejects_symlinked_runtime_projection() {
        use std::os::unix::fs::symlink;

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config = write_dispatch_alias_projection_fixture(harness.path());
        let outside_victim = harness.path().join("outside-victim.yaml");
        fs::write(&outside_victim, "original_victim_role\n")
            .expect("outside victim should be writable");
        let runtime_projection = harness
            .path()
            .join(".vida/project/agent-extensions/dispatch-aliases.yaml");
        fs::remove_file(&runtime_projection).expect("fixture runtime projection should be removed");
        symlink(&outside_victim, &runtime_projection)
            .expect("runtime projection symlink should be created");

        let error = refresh_runtime_agent_extension_projections_from_configured_sources(
            &config,
            harness.path(),
        )
        .expect_err("symlinked runtime projection should be rejected");

        assert!(error.contains("refusing to refresh runtime dispatch alias projection"));
        assert_eq!(
            fs::read_to_string(&outside_victim).expect("outside victim should remain readable"),
            "original_victim_role\n"
        );
    }
}
