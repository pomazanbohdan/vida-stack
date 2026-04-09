use std::path::Path;

pub(crate) fn build_compiled_agent_extension_bundle_for_root(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<serde_json::Value, String> {
    let registry_projection = crate::build_agent_extension_registry_projection(config, root);
    let mut validation_errors = registry_projection.validation_errors;
    let roles_registry = registry_projection.roles_registry;
    let skills_registry = registry_projection.skills_registry;
    let profiles_registry = registry_projection.profiles_registry;
    let flows_registry = registry_projection.flows_registry;
    let dispatch_aliases_registry = registry_projection.dispatch_aliases_registry;
    let enabled_project_roles = registry_projection.enabled_project_roles;
    let enabled_project_skills = registry_projection.enabled_project_skills;
    let enabled_project_profiles = registry_projection.enabled_project_profiles;
    let enabled_project_flows = registry_projection.enabled_project_flows;
    let selected_host_cli_system = registry_projection.selected_host_cli_system;
    let host_cli_system_registry = registry_projection.host_cli_system_registry;
    let dispatch_aliases_path = registry_projection.dispatch_aliases_path;
    let require_profile_resolution = registry_projection.validation.require_profile_resolution;
    let require_flow_resolution = registry_projection.validation.require_flow_resolution;
    let require_framework_role_compatibility = registry_projection
        .validation
        .require_framework_role_compatibility;
    let require_skill_role_compatibility = registry_projection
        .validation
        .require_skill_role_compatibility;
    let carrier_runtime_projection = crate::build_carrier_runtime_projection(
        config,
        root,
        selected_host_cli_system.as_deref(),
        &host_cli_system_registry,
        &dispatch_aliases_registry,
        dispatch_aliases_path.as_deref(),
    );
    validation_errors.extend(carrier_runtime_projection.validation_errors);
    let catalog_projection = crate::build_agent_extension_catalog_projection(
        &roles_registry,
        &skills_registry,
        &profiles_registry,
        &flows_registry,
        &enabled_project_roles,
        &enabled_project_skills,
        &enabled_project_profiles,
        &enabled_project_flows,
    );
    let project_roles = catalog_projection.project_roles;
    let project_skills = catalog_projection.project_skills;
    let project_profiles = catalog_projection.project_profiles;
    let project_flows = catalog_projection.project_flows;
    let project_role_map = catalog_projection.project_role_map;
    let project_skill_map = catalog_projection.project_skill_map;
    let project_profile_map = catalog_projection.project_profile_map;
    let project_flow_map = catalog_projection.project_flow_map;
    let all_project_flow_map = catalog_projection.all_project_flow_map;
    let enabled_framework_roles = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_framework_roles"],
    ));

    let bundle = serde_json::json!({
        "ok": true,
        "enabled": crate::yaml_bool(crate::yaml_lookup(config, &["agent_extensions", "enabled"]), false),
        "map_doc": crate::yaml_string(crate::yaml_lookup(config, &["agent_extensions", "map_doc"])).unwrap_or_default(),
        "enabled_framework_roles": enabled_framework_roles,
        "enabled_standard_flow_sets": crate::yaml_string_list(crate::yaml_lookup(config, &["agent_extensions", "enabled_standard_flow_sets"])),
        "enabled_shared_skills": crate::yaml_string_list(crate::yaml_lookup(config, &["agent_extensions", "enabled_shared_skills"])),
        "default_flow_set": crate::yaml_string(crate::yaml_lookup(config, &["agent_extensions", "default_flow_set"])).unwrap_or_default(),
        "runtime_projection_root": crate::project_activator_surface::runtime_agent_extensions_root(root).display().to_string(),
        "project_roles": project_roles,
        "project_skills": project_skills,
        "project_profiles": project_profiles,
        "project_flows": project_flows,
        "project_role_catalog": project_role_map,
        "project_profile_catalog": project_profile_map,
        "project_flow_catalog": project_flow_map,
        "all_project_flow_catalog": all_project_flow_map,
        "agent_system": serde_json::to_value(crate::yaml_lookup(config, &["agent_system"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "autonomous_execution": serde_json::to_value(crate::yaml_lookup(config, &["autonomous_execution"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "carrier_runtime": carrier_runtime_projection.carrier_runtime,
        "role_selection": serde_json::to_value(crate::yaml_lookup(config, &["agent_extensions", "role_selection"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
    });

    let role_ids = crate::registry_ids_by_key(&roles_registry, "roles", "role_id");
    let skill_ids = crate::registry_ids_by_key(&skills_registry, "skills", "skill_id");
    let profile_ids = crate::registry_ids_by_key(&profiles_registry, "profiles", "profile_id");
    let flow_ids = crate::registry_ids_by_key(&flows_registry, "flow_sets", "flow_id");

    crate::extend_agent_extension_bundle_validation_errors(
        &mut validation_errors,
        crate::AgentExtensionBundleValidationInput {
            require_profile_resolution,
            require_flow_resolution,
            require_framework_role_compatibility,
            require_skill_role_compatibility,
            enabled_framework_roles: &enabled_framework_roles,
            project_roles: &project_roles,
            project_skills: &project_skills,
            project_profiles: &project_profiles,
            project_flows: &project_flows,
            project_role_map: &project_role_map,
            project_skill_map: &project_skill_map,
            enabled_project_roles: &enabled_project_roles,
            enabled_project_skills: &enabled_project_skills,
            enabled_project_profiles: &enabled_project_profiles,
            enabled_project_flows: &enabled_project_flows,
            role_ids: &role_ids,
            skill_ids: &skill_ids,
            profile_ids: &profile_ids,
            flow_ids: &flow_ids,
        },
    );

    if !validation_errors.is_empty() {
        return Err(format!(
            "Agent extension bundle validation failed: {}",
            validation_errors.join("; ")
        ));
    }

    Ok(bundle)
}
