use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub(crate) struct RuntimeStateDirGuard {
    previous: Option<OsString>,
    active: bool,
    previous_root: Option<OsString>,
    root_active: bool,
    previous_cwd: Option<PathBuf>,
    cwd_active: bool,
}

impl Drop for RuntimeStateDirGuard {
    fn drop(&mut self) {
        if self.active {
            if let Some(previous) = &self.previous {
                std::env::set_var("VIDA_STATE_DIR", previous);
            } else {
                std::env::remove_var("VIDA_STATE_DIR");
            }
        }
        if !self.root_active {
            return;
        }
        if let Some(previous_root) = &self.previous_root {
            std::env::set_var("VIDA_ROOT", previous_root);
        } else {
            std::env::remove_var("VIDA_ROOT");
        }
        if self.cwd_active {
            if let Some(previous_cwd) = &self.previous_cwd {
                let _ = std::env::set_current_dir(previous_cwd);
            }
        }
    }
}

pub(crate) fn bind_runtime_state_dir_for_project_bound_command(
) -> Result<Option<RuntimeStateDirGuard>, String> {
    match bind_runtime_state_dir_to_current_project() {
        Ok(guard) => Ok(guard),
        Err(error) => {
            if std::env::var_os("VIDA_STATE_DIR").is_some() {
                return Ok(preserve_runtime_state_dir_env_for_project_bound_command());
            }
            Err(error)
        }
    }
}

pub(crate) fn bind_runtime_state_dir_override_for_project_bound_command(
    state_dir: &Path,
) -> Result<Option<RuntimeStateDirGuard>, String> {
    let normalized =
        normalize_runtime_state_dir_override(state_dir).unwrap_or_else(|| state_dir.to_path_buf());
    let previous = std::env::var_os("VIDA_STATE_DIR");
    std::env::set_var("VIDA_STATE_DIR", &normalized);
    let mut guard = RuntimeStateDirGuard {
        previous,
        active: true,
        previous_root: None,
        root_active: false,
        previous_cwd: None,
        cwd_active: false,
    };
    bind_project_root_for_state_dir(&normalized, &mut guard);
    Ok(Some(guard))
}

pub(crate) fn normalize_runtime_state_dir_env_for_parse() -> Option<RuntimeStateDirGuard> {
    let existing = std::env::var_os("VIDA_STATE_DIR")?;
    let existing_path = PathBuf::from(&existing);
    let normalized = normalize_runtime_state_dir_override(&existing_path)?;
    if normalized == existing_path {
        return None;
    }
    std::env::set_var("VIDA_STATE_DIR", normalized);
    Some(RuntimeStateDirGuard {
        previous: Some(existing),
        active: true,
        previous_root: None,
        root_active: false,
        previous_cwd: None,
        cwd_active: false,
    })
}

pub(crate) fn preserve_runtime_state_dir_env_for_parse_only() -> Option<RuntimeStateDirGuard> {
    let previous = std::env::var_os("VIDA_STATE_DIR")?;
    if let Some(normalized) = normalize_runtime_state_dir_override(&PathBuf::from(&previous)) {
        std::env::set_var("VIDA_STATE_DIR", normalized);
    }
    Some(RuntimeStateDirGuard {
        previous: Some(previous),
        active: true,
        previous_root: None,
        root_active: false,
        previous_cwd: None,
        cwd_active: false,
    })
}

pub(crate) fn preserve_runtime_state_dir_env_for_project_bound_command(
) -> Option<RuntimeStateDirGuard> {
    let mut guard = normalize_runtime_state_dir_env_for_parse().unwrap_or(RuntimeStateDirGuard {
        previous: None,
        active: false,
        previous_root: None,
        root_active: false,
        previous_cwd: None,
        cwd_active: false,
    });
    let state_dir = std::env::var_os("VIDA_STATE_DIR").map(PathBuf::from)?;
    bind_project_root_for_state_dir(&state_dir, &mut guard);
    Some(guard)
}

pub(crate) fn normalize_runtime_state_dir_override(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name().and_then(OsStr::to_str)?;
    if file_name != ".vida" {
        return None;
    }
    let canonical_state = path.join("data").join("state");
    canonical_state.is_dir().then_some(canonical_state)
}

fn bind_runtime_state_dir_to_current_project() -> Result<Option<RuntimeStateDirGuard>, String> {
    match crate::resolve_runtime_project_root() {
        Ok(project_root) => {
            let previous = std::env::var_os("VIDA_STATE_DIR");
            std::env::set_var(
                "VIDA_STATE_DIR",
                project_root.join(crate::state_store::default_state_dir()),
            );
            Ok(Some(RuntimeStateDirGuard {
                previous,
                active: true,
                previous_root: None,
                root_active: false,
                previous_cwd: None,
                cwd_active: false,
            }))
        }
        Err(error) => Err(error),
    }
}

fn bind_project_root_for_state_dir(state_dir: &Path, guard: &mut RuntimeStateDirGuard) {
    let Some(project_root) =
        crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir)
    else {
        return;
    };
    let previous_root = std::env::var_os("VIDA_ROOT");
    let root_already_bound =
        previous_root.as_ref().map(PathBuf::from).as_ref() == Some(&project_root);
    if !root_already_bound {
        std::env::set_var("VIDA_ROOT", &project_root);
        guard.previous_root = previous_root;
        guard.root_active = true;
    }
    if let Ok(current_dir) = std::env::current_dir() {
        if !current_dir.starts_with(&project_root)
            && std::env::set_current_dir(&project_root).is_ok()
        {
            guard.previous_cwd = Some(current_dir);
            guard.cwd_active = true;
        }
    }
}
