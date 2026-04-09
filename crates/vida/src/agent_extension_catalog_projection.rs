use std::collections::HashMap;

pub(crate) struct AgentExtensionCatalogProjection {
    pub(crate) project_roles: Vec<serde_json::Value>,
    pub(crate) project_skills: Vec<serde_json::Value>,
    pub(crate) project_profiles: Vec<serde_json::Value>,
    pub(crate) project_flows: Vec<serde_json::Value>,
    pub(crate) project_role_map: HashMap<String, serde_json::Value>,
    pub(crate) project_skill_map: HashMap<String, serde_json::Value>,
    pub(crate) project_profile_map: HashMap<String, serde_json::Value>,
    pub(crate) project_flow_map: HashMap<String, serde_json::Value>,
    pub(crate) all_project_flow_map: HashMap<String, serde_json::Value>,
}

pub(crate) fn build_agent_extension_catalog_projection(
    roles_registry: &serde_yaml::Value,
    skills_registry: &serde_yaml::Value,
    profiles_registry: &serde_yaml::Value,
    flows_registry: &serde_yaml::Value,
    enabled_project_roles: &[String],
    enabled_project_skills: &[String],
    enabled_project_profiles: &[String],
    enabled_project_flows: &[String],
) -> AgentExtensionCatalogProjection {
    let project_roles =
        crate::registry_rows_by_key(roles_registry, "roles", "role_id", enabled_project_roles);
    let project_skills = crate::registry_rows_by_key(
        skills_registry,
        "skills",
        "skill_id",
        enabled_project_skills,
    );
    let project_profiles = crate::registry_rows_by_key(
        profiles_registry,
        "profiles",
        "profile_id",
        enabled_project_profiles,
    );
    let project_flows = crate::registry_rows_by_key(
        flows_registry,
        "flow_sets",
        "flow_id",
        enabled_project_flows,
    );
    let all_project_flows =
        crate::registry_rows_by_key(flows_registry, "flow_sets", "flow_id", &[]);

    AgentExtensionCatalogProjection {
        project_role_map: crate::registry_row_map_by_id(&project_roles, "role_id"),
        project_skill_map: crate::registry_row_map_by_id(&project_skills, "skill_id"),
        project_profile_map: crate::registry_row_map_by_id(&project_profiles, "profile_id"),
        project_flow_map: crate::registry_row_map_by_id(&project_flows, "flow_id"),
        all_project_flow_map: crate::registry_row_map_by_id(&all_project_flows, "flow_id"),
        project_roles,
        project_skills,
        project_profiles,
        project_flows,
    }
}
