use std::path::Path;

fn configured_host_runtime_roots(root: &Path) -> Vec<String> {
    for candidate in [
        root.join("vida.config.yaml"),
        root.join("install/assets/vida.config.yaml.template"),
        root.join("docs/framework/templates/vida.config.yaml.template"),
    ] {
        let Ok(raw) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok(overlay) = serde_yaml::from_str::<serde_yaml::Value>(&raw) else {
            continue;
        };
        let registry =
            crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(&overlay));
        let mut roots = registry
            .iter()
            .filter_map(|(system, entry)| {
                let runtime_root =
                    crate::project_activator_surface::host_cli_system_runtime_surface(entry, system);
                let trimmed = runtime_root.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        if !roots.is_empty() {
            return roots;
        }
    }
    Vec::new()
}

pub(crate) fn looks_like_host_runtime_source_root(root: &Path) -> bool {
    crate::init_surfaces::resolve_init_agents_source(root).is_ok()
        && crate::init_surfaces::resolve_init_sidecar_source(root).is_ok()
        && crate::init_surfaces::resolve_init_config_template_source(root).is_ok()
        && configured_host_runtime_roots(root)
            .iter()
            .any(|relative| root.join(relative).is_dir())
}
