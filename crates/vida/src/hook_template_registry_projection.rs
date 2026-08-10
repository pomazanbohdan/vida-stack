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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn missing_hook_registry_path_preserves_configured_ids_and_strict_blocker() {
        let config: serde_yaml::Value =
            serde_yaml::from_str("agent_extensions:\n  enabled_hook_templates:\n    - preflight\n")
                .expect("config fixture should parse");
        let root = Path::new("project-root");

        let permissive = build_hook_template_registry_projection(&config, root, false);
        assert!(permissive.hook_templates_registry.is_null());
        assert_eq!(permissive.enabled_hook_templates, vec!["preflight"]);
        assert!(permissive.hook_templates_path.is_none());
        assert!(permissive.validation_errors.is_empty());

        let strict = build_hook_template_registry_projection(&config, root, true);
        assert_eq!(strict.validation_errors.len(), 1);
        assert!(strict.validation_errors[0].contains("registry path is required"));
    }

    #[test]
    fn configured_hook_registry_loads_rows_and_defaults_enabled_ids() {
        let root = std::env::temp_dir().join(format!(
            "vida-hook-registry-projection-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("config")).expect("create registry directory");
        fs::write(
            root.join("config/hooks.yaml"),
            "version: 1\nhook_templates:\n  - template_id: preflight\n    name: Preflight\n",
        )
        .expect("write hook registry");
        let config: serde_yaml::Value = serde_yaml::from_str(
            "agent_extensions:\n  registries:\n    hook_templates: config/hooks.yaml\n",
        )
        .expect("config fixture should parse");

        let projection = build_hook_template_registry_projection(&config, &root, true);

        assert!(projection.validation_errors.is_empty());
        assert_eq!(
            projection.hook_templates_path.as_deref(),
            Some("config/hooks.yaml")
        );
        assert_eq!(projection.enabled_hook_templates, vec!["preflight"]);
        assert_eq!(projection.hook_templates_registry["version"], 1);
        assert_eq!(
            projection.hook_templates_registry["hook_templates"][0]["template_id"],
            "preflight"
        );
        fs::remove_dir_all(&root).expect("remove registry directory");
    }
}
