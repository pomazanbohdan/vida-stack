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
