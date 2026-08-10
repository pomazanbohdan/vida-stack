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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(key: &str, id_field: &str, ids: &[&str]) -> serde_yaml::Value {
        let rows = ids
            .iter()
            .map(|id| format!("  - {id_field}: {id}\n"))
            .collect::<String>();
        serde_yaml::from_str(&format!("{key}:\n{rows}")).expect("registry fixture should parse")
    }

    #[test]
    fn projection_filters_enabled_rows_and_indexes_all_flow_definitions() {
        let roles = registry("roles", "role_id", &["worker", "reviewer"]);
        let skills = registry("skills", "skill_id", &["rust"]);
        let profiles = registry("profiles", "profile_id", &["default"]);
        let flows = registry("flow_sets", "flow_id", &["standard", "release"]);

        let projection = build_agent_extension_catalog_projection(
            &roles,
            &skills,
            &profiles,
            &flows,
            &["worker".to_string()],
            &["rust".to_string()],
            &["default".to_string()],
            &["release".to_string()],
        );

        assert_eq!(projection.project_roles.len(), 1);
        assert!(projection.project_role_map.contains_key("worker"));
        assert_eq!(projection.project_skills.len(), 1);
        assert_eq!(projection.project_profiles.len(), 1);
        assert_eq!(projection.project_flows.len(), 1);
        assert!(projection.project_flow_map.contains_key("release"));
        assert_eq!(projection.all_project_flow_map.len(), 2);
        assert!(projection.all_project_flow_map.contains_key("standard"));
    }
}
