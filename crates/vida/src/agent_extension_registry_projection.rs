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
    let source_path =
        crate::project_activator_surface::resolve_overlay_path(root, &configured_source_path);
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
    let runtime_registry =
        crate::project_activator_surface::read_yaml_file_checked(&runtime_projection_path)
            .map_err(|error| {
                format!(
                    "failed to load runtime {} projection `{}`: {error}",
                    spec.label, spec.runtime_projection_path
                )
            })?;
    let source_aliases = registry_ids(&source_registry, spec.registry_key, spec.id_field);
    let runtime_aliases = registry_ids(&runtime_registry, spec.registry_key, spec.id_field);
    let source_raw = std::fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "failed to read configured {} source `{}`: {error}",
            spec.label,
            source_path.display()
        )
    })?;
    let runtime_raw = std::fs::read_to_string(&runtime_projection_path).map_err(|error| {
        format!(
            "failed to read runtime {} projection `{}`: {error}",
            spec.label,
            runtime_projection_path.display()
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

fn write_runtime_projection_file(path: &Path, contents: &str, label: &str) -> Result<(), String> {
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

    let write_result = (|| -> Result<(), String> {
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
        drop(file);
        std::fs::rename(&temp_path, path).map_err(|error| {
            format!(
                "failed to replace runtime {label} projection `{}`: {error}",
                path.display()
            )
        })
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
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
    write_runtime_projection_file(&parity.runtime_projection_path, &source, "dispatch alias")?;
    dispatch_alias_projection_parity(config, root)
}

pub(crate) fn refresh_runtime_agent_extension_projections_from_configured_sources(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<Vec<DispatchAliasProjectionParity>, String> {
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
        write_runtime_projection_file(
            &parity.runtime_projection_path,
            &source,
            &parity.registry_label,
        )?;
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
