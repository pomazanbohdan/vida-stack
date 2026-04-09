use std::collections::HashMap;

pub(crate) struct AgentExtensionBundleValidationInput<'a> {
    pub(crate) require_profile_resolution: bool,
    pub(crate) require_flow_resolution: bool,
    pub(crate) require_framework_role_compatibility: bool,
    pub(crate) require_skill_role_compatibility: bool,
    pub(crate) enabled_framework_roles: &'a [String],
    pub(crate) project_roles: &'a [serde_json::Value],
    pub(crate) project_skills: &'a [serde_json::Value],
    pub(crate) project_profiles: &'a [serde_json::Value],
    pub(crate) project_flows: &'a [serde_json::Value],
    pub(crate) project_role_map: &'a HashMap<String, serde_json::Value>,
    pub(crate) project_skill_map: &'a HashMap<String, serde_json::Value>,
    pub(crate) enabled_project_roles: &'a [String],
    pub(crate) enabled_project_skills: &'a [String],
    pub(crate) enabled_project_profiles: &'a [String],
    pub(crate) enabled_project_flows: &'a [String],
    pub(crate) role_ids: &'a std::collections::HashSet<String>,
    pub(crate) skill_ids: &'a std::collections::HashSet<String>,
    pub(crate) profile_ids: &'a std::collections::HashSet<String>,
    pub(crate) flow_ids: &'a std::collections::HashSet<String>,
}

pub(crate) fn extend_agent_extension_bundle_validation_errors(
    validation_errors: &mut Vec<String>,
    input: AgentExtensionBundleValidationInput<'_>,
) {
    if input.require_framework_role_compatibility {
        for role in input.project_roles {
            let role_id = role["role_id"].as_str().unwrap_or("<unknown>");
            let base_role = role["base_role"].as_str().unwrap_or_default();
            if base_role.is_empty()
                || !input
                    .enabled_framework_roles
                    .iter()
                    .any(|row| row == base_role)
            {
                validation_errors.push(format!(
                    "project role `{role_id}` references unresolved framework base role `{base_role}`"
                ));
            }
        }
    }

    if input.require_profile_resolution {
        for profile in input.project_profiles {
            let profile_id = profile["profile_id"].as_str().unwrap_or("<unknown>");
            let role_ref = profile["role_ref"].as_str().unwrap_or_default();
            if role_ref.is_empty()
                || !(input
                    .enabled_framework_roles
                    .iter()
                    .any(|row| row == role_ref)
                    || input.project_role_map.contains_key(role_ref))
            {
                validation_errors.push(format!(
                    "project profile `{profile_id}` references unresolved role `{role_ref}`"
                ));
            }
        }
    }

    if input.require_skill_role_compatibility {
        for profile in input.project_profiles {
            let profile_id = profile["profile_id"].as_str().unwrap_or("<unknown>");
            let role_ref = profile["role_ref"].as_str().unwrap_or_default();
            let Some(role) = input.project_role_map.get(role_ref) else {
                continue;
            };
            let base_role = role["base_role"].as_str().unwrap_or_default();
            for skill_ref in crate::csv_json_string_list(profile.get("skill_refs")) {
                let Some(skill) = input.project_skill_map.get(&skill_ref) else {
                    validation_errors.push(format!(
                        "project profile `{profile_id}` references unresolved skill `{skill_ref}`"
                    ));
                    continue;
                };
                let compatible_roles =
                    crate::csv_json_string_list(skill.get("compatible_base_roles"));
                if !compatible_roles.is_empty()
                    && !compatible_roles.iter().any(|row| row == base_role)
                {
                    validation_errors.push(format!(
                        "project profile `{profile_id}` binds role `{role_ref}` with base role `{base_role}` to incompatible skill `{skill_ref}`"
                    ));
                }
            }
        }
    }

    if input.require_flow_resolution {
        for flow in input.project_flows {
            let flow_id = flow["flow_id"].as_str().unwrap_or("<unknown>");
            for role_ref in crate::csv_json_string_list(flow.get("role_chain")) {
                if !(input
                    .enabled_framework_roles
                    .iter()
                    .any(|row| row == &role_ref)
                    || input.project_role_map.contains_key(&role_ref))
                {
                    validation_errors.push(format!(
                        "project flow `{flow_id}` references unresolved role `{role_ref}`"
                    ));
                }
            }
        }
    }

    let missing_roles = crate::project_activator_surface::collect_missing_registry_ids(
        input.role_ids,
        input.enabled_project_roles,
    );
    if !missing_roles.is_empty() {
        validation_errors.push(format!(
            "agent extension roles registry is missing enabled role ids: {}",
            missing_roles.join(", ")
        ));
    }

    let missing_skills = crate::project_activator_surface::collect_missing_registry_ids(
        input.skill_ids,
        input.enabled_project_skills,
    );
    if !missing_skills.is_empty() {
        validation_errors.push(format!(
            "agent extension skills registry is missing enabled skill ids: {}",
            missing_skills.join(", ")
        ));
    }

    if input.require_profile_resolution {
        let missing_profiles = crate::project_activator_surface::collect_missing_registry_ids(
            input.profile_ids,
            input.enabled_project_profiles,
        );
        if !missing_profiles.is_empty() {
            validation_errors.push(format!(
                "agent extension profiles registry is missing enabled profile ids: {}",
                missing_profiles.join(", ")
            ));
        }
    }

    if input.require_flow_resolution {
        let missing_flows = crate::project_activator_surface::collect_missing_registry_ids(
            input.flow_ids,
            input.enabled_project_flows,
        );
        if !missing_flows.is_empty() {
            validation_errors.push(format!(
                "agent extension flows registry is missing enabled flow ids: {}",
                missing_flows.join(", ")
            ));
        }
    }

    let _ = input.project_skills;
}
