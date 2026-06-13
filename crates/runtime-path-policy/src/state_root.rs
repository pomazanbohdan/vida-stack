use std::path::{Path, PathBuf};

use crate::safe_path::PathPolicyError;

#[derive(Debug, Clone)]
pub struct StateRoot {
    raw: PathBuf,
    canonical: PathBuf,
}

impl StateRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PathPolicyError> {
        let raw = path.as_ref().to_path_buf();
        let metadata =
            std::fs::symlink_metadata(&raw).map_err(|source| PathPolicyError::StateRootOpen {
                path: raw.clone(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(PathPolicyError::StateRootSymlink { path: raw });
        }
        if !metadata.is_dir() {
            return Err(PathPolicyError::StateRootNotDirectory { path: raw });
        }
        let canonical = raw
            .canonicalize()
            .map_err(|source| PathPolicyError::StateRootOpen {
                path: raw.clone(),
                source,
            })?;
        Ok(Self { raw, canonical })
    }

    pub fn raw(&self) -> &Path {
        &self.raw
    }

    pub fn canonical(&self) -> &Path {
        &self.canonical
    }

    pub fn contains_canonical(&self, path: &Path) -> bool {
        path.starts_with(&self.canonical)
    }

    pub(crate) fn resolve_raw(&self, raw_path: impl AsRef<Path>) -> PathBuf {
        let raw_path = raw_path.as_ref();
        if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            self.raw.join(raw_path)
        }
    }
}
