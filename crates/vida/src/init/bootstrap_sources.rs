use std::path::{Path, PathBuf};

use crate::host_runtime_registry::looks_like_host_runtime_source_root;

pub(crate) fn resolve_init_bootstrap_source_root() -> PathBuf {
    if let Some(installed_root) = resolve_installed_runtime_root() {
        for candidate in installed_runtime_source_root_candidates(&installed_root) {
            if looks_like_init_bootstrap_source_root(&candidate) {
                return candidate;
            }
        }
    }
    crate::repo_runtime_root()
}

pub(crate) fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    taskflow_binary_candidates_for_root(root)
        .into_iter()
        .next()
        .map(|_| root.to_path_buf())
}

pub(crate) fn installed_runtime_source_root_candidates(root: &Path) -> Vec<PathBuf> {
    let current_root = root.join("current");
    if current_root == root {
        vec![root.to_path_buf()]
    } else {
        vec![current_root, root.to_path_buf()]
    }
}

pub(crate) fn taskflow_binary_candidates_for_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let bin_dir = root.join("bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_binary = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_file() && is_taskflow_binary {
                candidates.push(path);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_runtime_dir = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_dir() && is_taskflow_runtime_dir {
                let candidate = path.join("src/vida");
                if candidate.exists() {
                    candidates.push(candidate);
                }
            }
        }
    }

    candidates
}

pub(crate) fn looks_like_init_bootstrap_source_root(root: &Path) -> bool {
    resolve_init_agents_source(root).is_ok()
        && resolve_init_sidecar_source(root).is_ok()
        && resolve_init_config_template_source(root).is_ok()
        && root
            .join(crate::state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .is_dir()
        && root
            .join(crate::state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT)
            .is_dir()
        && looks_like_host_runtime_source_root(root)
}

pub(crate) fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

pub(crate) fn resolve_init_agents_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.scaffold.md"),
        root.join("AGENTS.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve generated AGENTS scaffold. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_sidecar_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.sidecar.scaffold.md"),
        root.join("AGENTS.sidecar.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve project sidecar source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_config_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/vida.config.yaml.template"),
        root.join("docs/framework/templates/vida.config.yaml.template"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve vida.config.yaml template. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_feature_design_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/feature-design-document.template.md"),
        root.join("docs/framework/templates/feature-design-document.template.md"),
        root.join("docs/product/spec/templates/feature-design-document.template.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve framework feature-design template source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        first_existing_path, resolve_init_agents_source, resolve_init_config_template_source,
        resolve_init_sidecar_source, taskflow_binary_candidates_for_root,
    };
    use crate::temp_state::TempStateHarness;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn first_existing_path_returns_the_first_existing_candidate() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let first = harness.path().join("first");
        let second = harness.path().join("second");
        fs::write(&second, "second").expect("second candidate should be writable");

        assert_eq!(first_existing_path(&[first, second.clone()]), Some(second));
        assert_eq!(first_existing_path(&[PathBuf::from("missing")]), None);
    }

    #[test]
    fn bootstrap_source_resolvers_use_project_fallbacks_and_report_missing_inputs() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::write(root.join("AGENTS.md"), "agents").expect("agents fallback should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "sidecar")
            .expect("sidecar fallback should exist");
        let config_fallback = root.join("docs/framework/templates");
        fs::create_dir_all(&config_fallback).expect("config fallback directory should exist");
        fs::write(config_fallback.join("vida.config.yaml.template"), "config")
            .expect("config fallback should exist");

        assert_eq!(
            resolve_init_agents_source(root).expect("agents fallback should resolve"),
            root.join("AGENTS.md")
        );
        assert_eq!(
            resolve_init_sidecar_source(root).expect("sidecar fallback should resolve"),
            root.join("AGENTS.sidecar.md")
        );
        assert_eq!(
            resolve_init_config_template_source(root).expect("config fallback should resolve"),
            config_fallback.join("vida.config.yaml.template")
        );

        fs::remove_file(root.join("AGENTS.md")).expect("agents fallback should be removable");
        let error = resolve_init_agents_source(root).expect_err("missing agents should block");
        assert!(error.contains("install/assets/AGENTS.scaffold.md"));
        assert!(error.contains("AGENTS.md"));
    }

    #[test]
    fn taskflow_binary_candidates_include_files_and_runtime_source_directories() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("bin")).expect("bin directory should exist");
        fs::write(root.join("bin/taskflow-helper"), "binary")
            .expect("taskflow helper should exist");
        fs::write(root.join("bin/other"), "other").expect("other file should exist");
        fs::create_dir_all(root.join("taskflow-runtime/src/vida"))
            .expect("runtime source directory should exist");
        fs::create_dir_all(root.join("not-taskflow/src/vida"))
            .expect("unrelated source directory should exist");

        let candidates = taskflow_binary_candidates_for_root(root);
        assert!(
            candidates
                .iter()
                .any(|path| path == &root.join("bin/taskflow-helper"))
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == &root.join("taskflow-runtime/src/vida"))
        );
        assert!(!candidates.iter().any(|path| path.ends_with("bin/other")));
        assert!(
            !candidates
                .iter()
                .any(|path| path.ends_with("not-taskflow/src/vida"))
        );
    }
}
