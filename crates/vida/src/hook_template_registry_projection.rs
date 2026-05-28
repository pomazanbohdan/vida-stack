use std::path::Path;

pub(crate) struct HookTemplateRegistryProjection {
    pub(crate) hook_templates_registry: serde_yaml::Value,
    pub(crate) enabled_hook_templates: Vec<String>,
    pub(crate) hook_templates_path: Option<String>,
    pub(crate) validation_errors: Vec<String>,
}

pub(crate) fn build_hook_template_registry_projection(
    config: &serde_yaml::Value,
    root: &Path,
    require_registry_files: bool,
) -> HookTemplateRegistryProjection {
    let hook_templates_path = crate::yaml_string(crate::yaml_lookup(
        config,
        &["agent_extensions", "registries", "hook_templates"],
    ));
    let configured_enabled_hook_templates = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_hook_templates"],
    ));
    let mut validation_errors = Vec::new();
    let hook_templates_registry = match hook_templates_path.as_deref() {
        Some(path) => match crate::project_activator_surface::load_registry_projection(
            root,
            Some(path),
            "hook_templates",
            "template_id",
            "hook_templates",
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !configured_enabled_hook_templates.is_empty() {
                validation_errors.push(
                    "agent extension hook templates registry path is required but missing"
                        .to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let enabled_hook_templates = crate::effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_hook_templates"],
        &hook_templates_registry,
        "hook_templates",
        "template_id",
    );

    HookTemplateRegistryProjection {
        hook_templates_registry,
        enabled_hook_templates,
        hook_templates_path,
        validation_errors,
    }
}
