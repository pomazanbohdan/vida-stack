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
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&overlay));
        let mut roots = registry
            .iter()
            .filter_map(|(system, entry)| {
                let runtime_root =
                    crate::project_activator_surface::host_cli_system_runtime_surface(
                        entry, system,
                    );
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

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use super::{configured_host_runtime_roots, looks_like_host_runtime_source_root};

    fn unique_test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "vida-host-runtime-registry-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ))
    }

    #[test]
    fn host_runtime_source_root_requires_bootstrap_markers_and_configured_runtime_dirs() {
        let root = unique_test_root();
        let assets = root.join("install/assets");
        std::fs::create_dir_all(&assets).expect("create bootstrap assets");
        std::fs::write(assets.join("AGENTS.scaffold.md"), "agents").expect("write agents scaffold");
        std::fs::write(assets.join("AGENTS.sidecar.scaffold.md"), "sidecar")
            .expect("write sidecar scaffold");
        std::fs::write(assets.join("vida.config.yaml.template"), "template")
            .expect("write config template");
        std::fs::write(
            root.join("vida.config.yaml"),
            "host_environment:\n  systems:\n    zeta:\n      runtime_root: runtime/z\n    alpha:\n      runtime_root: runtime/a\n",
        )
        .expect("write runtime registry");
        std::fs::create_dir_all(root.join("runtime/z")).expect("create zeta runtime root");
        std::fs::create_dir_all(root.join("runtime/a")).expect("create alpha runtime root");

        assert_eq!(
            configured_host_runtime_roots(&root),
            vec!["runtime/a".to_string(), "runtime/z".to_string()]
        );
        assert!(looks_like_host_runtime_source_root(&root));

        let missing_runtime = root.join("missing");
        assert!(!looks_like_host_runtime_source_root(&missing_runtime));
        std::fs::remove_dir_all(&root).expect("remove test root");
    }
}
