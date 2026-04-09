use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn looks_like_project_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file()
        && path.join("vida.config.yaml").is_file()
        && path.join(".vida/config").is_dir()
        && path.join(".vida/db").is_dir()
        && path.join(".vida/project").is_dir()
}

pub(crate) fn resolve_source_repo_root_from_current_dir(current_dir: &Path) -> Option<PathBuf> {
    let repo_root = super::repo_runtime_root();
    if current_dir.starts_with(&repo_root)
        && super::init_surfaces::looks_like_init_bootstrap_source_root(&repo_root)
    {
        return Some(repo_root);
    }
    None
}

pub(crate) fn resolve_env_repo_root() -> Result<Option<PathBuf>, String> {
    let Some(root) = std::env::var_os("VIDA_ROOT") else {
        return Ok(None);
    };
    let root = PathBuf::from(root);
    if !root.exists() {
        return Err(format!(
            "VIDA_ROOT points to a missing path: {}",
            root.display()
        ));
    }
    if looks_like_project_root(&root)
        || super::init_surfaces::looks_like_init_bootstrap_source_root(&root)
    {
        return Ok(Some(root));
    }
    Err(format!(
        "VIDA_ROOT points to a path that is not a VIDA runtime or source root: {}",
        root.display()
    ))
}

pub(crate) fn resolve_repo_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    let mut candidates = current_dir
        .ancestors()
        .filter(|path| looks_like_project_root(path))
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => {
            if let Some(root) = resolve_source_repo_root_from_current_dir(&current_dir) {
                return Ok(root);
            }
            if let Some(root) = resolve_env_repo_root()? {
                return Ok(root);
            }
            Err(format!(
                "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
                current_dir.display()
            ))
        }
        _ => Err(format!(
            "Ambiguous VIDA project root from {}: {}. Set VIDA_ROOT explicitly.",
            current_dir.display(),
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub(crate) fn resolve_runtime_project_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    let mut candidates = current_dir
        .ancestors()
        .filter(|path| looks_like_project_root(path))
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(format!(
            "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
            current_dir.display()
        ))
        .or_else(|_| {
            if let Some(root) = resolve_source_repo_root_from_current_dir(&current_dir) {
                return Ok(root);
            }
            if let Some(root) = resolve_env_repo_root()? {
                return Ok(root);
            }
            Err(format!(
                "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
                current_dir.display()
            ))
        }),
        _ => Err(format!(
            "Ambiguous VIDA project root from {}: {}. Set VIDA_ROOT explicitly.",
            current_dir.display(),
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub(crate) fn resolve_status_project_root(state_root: &Path) -> Option<PathBuf> {
    super::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .or_else(|| resolve_runtime_project_root().ok())
}

pub(crate) fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("Failed to create {}: {error}", path.display()))
}
