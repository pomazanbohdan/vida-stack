use std::collections::HashMap;
use std::path::Path;

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
    let configured_enabled_project_roles = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_roles"],
    ));
    let configured_enabled_project_profiles = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_profiles"],
    ));
    let configured_enabled_project_flows = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_flows"],
    ));
    let roles_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "roles"],
    ));
    let skills_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "skills"],
    ));
    let profiles_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "profiles"],
    ));
    let flows_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "flows"],
    ));
    let dispatch_aliases_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "dispatch_aliases"],
    ));
    let validation = AgentExtensionValidationConfig {
        require_registry_files: crate::yaml_bool(
            crate::yaml_lookup(
                config,
                &["agent_extensions", "validation", "require_registry_files"],
            ),
            false,
        ),
        require_profile_resolution: crate::yaml_bool(
            crate::yaml_lookup(
                config,
                &[
                    "agent_extensions",
                    "validation",
                    "require_profile_resolution",
                ],
            ),
            false,
        ),
        require_flow_resolution: crate::yaml_bool(
            crate::yaml_lookup(
                config,
                &["agent_extensions", "validation", "require_flow_resolution"],
            ),
            false,
        ),
        require_framework_role_compatibility: crate::yaml_bool(
            crate::yaml_lookup(
                config,
                &[
                    "agent_extensions",
                    "validation",
                    "require_framework_role_compatibility",
                ],
            ),
            false,
        ),
        require_skill_role_compatibility: crate::yaml_bool(
            crate::yaml_lookup(
                config,
                &[
                    "agent_extensions",
                    "validation",
                    "require_skill_role_compatibility",
                ],
            ),
            false,
        ),
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
