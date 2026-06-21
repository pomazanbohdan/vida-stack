#![allow(dead_code)]

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TEMP_DIR_ATTEMPTS: u32 = 32;

#[derive(Debug)]
pub struct TempStateHarness {
    root: PathBuf,
}

impl TempStateHarness {
    pub fn new() -> io::Result<Self> {
        let root = reserve_temp_root()?;
        Ok(Self { root })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempStateHarness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn reserve_temp_root() -> io::Result<PathBuf> {
    let base = temp_state_base_dir()?;
    fs::create_dir_all(&base)?;

    for attempt in 0..TEMP_DIR_ATTEMPTS {
        let candidate = base.join(unique_temp_dir_name(attempt));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "failed to reserve a unique VIDA temp-state directory",
    ))
}

pub(crate) fn temp_state_base_dir() -> io::Result<PathBuf> {
    let configured_temp = env::temp_dir();
    if !path_is_inside_repo(&configured_temp) {
        return Ok(configured_temp);
    }

    if let Some(fallback) = user_temp_fallback_dir().filter(|path| !path_is_inside_repo(path)) {
        return Ok(fallback);
    }

    Ok(configured_temp)
}

fn user_temp_fallback_dir() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Temp"))
        .or_else(|| {
            env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|path| path.join("AppData").join("Local").join("Temp"))
        })
}

fn path_is_inside_repo(path: &Path) -> bool {
    let repo_root = crate::state_store::repo_root();
    let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let normalized_repo = repo_root.canonicalize().unwrap_or(repo_root);
    normalized_path.starts_with(normalized_repo)
}

fn unique_temp_dir_name(attempt: u32) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!(
        "vida-temp-state-{}-{}-{}",
        std::process::id(),
        nanos,
        attempt
    )
}

#[cfg(test)]
mod tests {
    use super::TempStateHarness;

    #[test]
    fn temp_state_harness_creates_and_cleans_directory() {
        let path = {
            let harness = TempStateHarness::new().expect("temp state harness should initialize");
            let path = harness.path().to_path_buf();
            assert!(path.exists());
            path
        };

        assert!(!path.exists());
    }
}
