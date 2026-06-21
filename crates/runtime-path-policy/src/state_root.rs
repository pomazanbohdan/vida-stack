use std::path::{Path, PathBuf};
use std::sync::Arc;

use cap_std::fs::Dir;

use crate::safe_path::{ArtifactPathKind, PathPolicyError};

#[derive(Debug, Clone)]
pub struct StateRoot {
    raw: PathBuf,
    canonical: PathBuf,
    cap_dir: Arc<Dir>,
}

impl StateRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PathPolicyError> {
        let raw = path.as_ref().to_path_buf();
        let metadata =
            fs_err::symlink_metadata(&raw).map_err(|source| PathPolicyError::StateRootOpen {
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
        let cap_dir =
            Dir::open_ambient_dir(&canonical, cap_std::ambient_authority()).map_err(|source| {
                PathPolicyError::StateRootOpen {
                    path: canonical.clone(),
                    source,
                }
            })?;
        Ok(Self {
            raw,
            canonical,
            cap_dir: Arc::new(cap_dir),
        })
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
        if path_is_rooted(raw_path) {
            raw_path.to_path_buf()
        } else {
            self.raw.join(raw_path)
        }
    }

    pub(crate) fn cap_dir(&self) -> &Dir {
        &self.cap_dir
    }

    pub(crate) fn cap_relative_path(
        &self,
        raw_path: impl AsRef<Path>,
        kind: ArtifactPathKind,
    ) -> Result<PathBuf, PathPolicyError> {
        let raw_path = raw_path.as_ref();
        if path_is_rooted(raw_path) {
            raw_path
                .strip_prefix(&self.canonical)
                .or_else(|_| raw_path.strip_prefix(&self.raw))
                .map(Path::to_path_buf)
                .map_err(|_| PathPolicyError::OutsideStateRoot {
                    kind,
                    path: raw_path.to_path_buf(),
                    root: self.canonical.clone(),
                })
        } else {
            Ok(raw_path.to_path_buf())
        }
    }
}

fn path_is_rooted(path: &Path) -> bool {
    path.is_absolute() || path.has_root()
}
