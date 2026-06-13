use std::fmt;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::state_root::StateRoot;
use crate::symlink_policy::is_symlink;

#[derive(Debug, Clone)]
pub struct ExistingRegularFile {
    path: PathBuf,
    kind: ArtifactPathKind,
}

impl ExistingRegularFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> ArtifactPathKind {
        self.kind
    }
}

#[derive(Debug, Clone)]
pub struct NewStateOutputPath {
    path: PathBuf,
    kind: ArtifactPathKind,
}

impl NewStateOutputPath {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> ArtifactPathKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactPathKind {
    HostBridgeRequest,
    HostBridgePacket,
    HostBridgeResult,
    HostBridgeReceipt,
    DispatchPacket,
    DispatchResult,
    RuntimeSnapshot,
    TaskAttemptArtifact,
    DocflowChangedPath,
    GenericJson,
}

impl fmt::Display for ArtifactPathKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::HostBridgeRequest => "host bridge request",
            Self::HostBridgePacket => "host bridge packet",
            Self::HostBridgeResult => "host bridge result",
            Self::HostBridgeReceipt => "host bridge receipt",
            Self::DispatchPacket => "dispatch packet",
            Self::DispatchResult => "dispatch result",
            Self::RuntimeSnapshot => "runtime snapshot",
            Self::TaskAttemptArtifact => "task attempt artifact",
            Self::DocflowChangedPath => "docflow changed path",
            Self::GenericJson => "generic json",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Error)]
pub enum PathPolicyError {
    #[error("state root `{path}` could not be opened: {source}")]
    StateRootOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("state root `{path}` is a symlink")]
    StateRootSymlink { path: PathBuf },
    #[error("state root `{path}` is not a directory")]
    StateRootNotDirectory { path: PathBuf },
    #[error("{kind} path `{path}` contains a dot segment")]
    DotSegment {
        kind: ArtifactPathKind,
        path: PathBuf,
    },
    #[error("{kind} path `{path}` could not be inspected: {source}")]
    Metadata {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{kind} path `{path}` is a symlink")]
    Symlink {
        kind: ArtifactPathKind,
        path: PathBuf,
    },
    #[error("{kind} path `{path}` is not a regular file")]
    NotRegularFile {
        kind: ArtifactPathKind,
        path: PathBuf,
    },
    #[error("{kind} path `{path}` could not be canonicalized: {source}")]
    Canonicalize {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{kind} path `{path}` is outside state root `{root}`")]
    OutsideStateRoot {
        kind: ArtifactPathKind,
        path: PathBuf,
        root: PathBuf,
    },
    #[error("{kind} parent path `{path}` could not be created: {source}")]
    ParentCreate {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{kind} path `{path}` already exists")]
    AlreadyExists {
        kind: ArtifactPathKind,
        path: PathBuf,
    },
    #[error("{kind} path `{path}` has no parent directory")]
    MissingParent {
        kind: ArtifactPathKind,
        path: PathBuf,
    },
    #[error("{kind} path `{path}` exceeds {max_bytes} bytes")]
    TooLarge {
        kind: ArtifactPathKind,
        path: PathBuf,
        max_bytes: u64,
    },
    #[error("{kind} path `{path}` could not be read: {source}")]
    Read {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{kind} path `{path}` contains invalid json: {source}")]
    Json {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{kind} path `{path}` could not be written: {source}")]
    Write {
        kind: ArtifactPathKind,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn path_contains_dot_segment(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

pub fn existing_regular_file_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
) -> Result<ExistingRegularFile, PathPolicyError> {
    let raw_path = raw_path.as_ref();
    reject_dot_segment(raw_path, kind)?;
    let path = root.resolve_raw(raw_path);
    let metadata =
        std::fs::symlink_metadata(&path).map_err(|source| PathPolicyError::Metadata {
            kind,
            path: path.clone(),
            source,
        })?;
    if is_symlink(&metadata) {
        return Err(PathPolicyError::Symlink { kind, path });
    }
    if !metadata.is_file() {
        return Err(PathPolicyError::NotRegularFile { kind, path });
    }
    let canonical = path
        .canonicalize()
        .map_err(|source| PathPolicyError::Canonicalize {
            kind,
            path: path.clone(),
            source,
        })?;
    ensure_under_root(root, &canonical, kind)?;
    Ok(ExistingRegularFile {
        path: canonical,
        kind,
    })
}

pub fn new_output_path_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
    replace_existing: bool,
) -> Result<NewStateOutputPath, PathPolicyError> {
    let raw_path = raw_path.as_ref();
    reject_dot_segment(raw_path, kind)?;
    let path = root.resolve_raw(raw_path);
    let parent = path
        .parent()
        .ok_or_else(|| PathPolicyError::MissingParent {
            kind,
            path: path.clone(),
        })?;
    std::fs::create_dir_all(parent).map_err(|source| PathPolicyError::ParentCreate {
        kind,
        path: parent.to_path_buf(),
        source,
    })?;
    let parent_canonical =
        parent
            .canonicalize()
            .map_err(|source| PathPolicyError::Canonicalize {
                kind,
                path: parent.to_path_buf(),
                source,
            })?;
    ensure_under_root(root, &parent_canonical, kind)?;
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if is_symlink(&metadata) {
            return Err(PathPolicyError::Symlink { kind, path });
        }
        if !replace_existing {
            return Err(PathPolicyError::AlreadyExists { kind, path });
        }
        if !metadata.is_file() {
            return Err(PathPolicyError::NotRegularFile { kind, path });
        }
    }
    Ok(NewStateOutputPath { path, kind })
}

fn reject_dot_segment(path: &Path, kind: ArtifactPathKind) -> Result<(), PathPolicyError> {
    if path_contains_dot_segment(path) {
        return Err(PathPolicyError::DotSegment {
            kind,
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn ensure_under_root(
    root: &StateRoot,
    path: &Path,
    kind: ArtifactPathKind,
) -> Result<(), PathPolicyError> {
    if root.contains_canonical(path) {
        Ok(())
    } else {
        Err(PathPolicyError::OutsideStateRoot {
            kind,
            path: path.to_path_buf(),
            root: root.canonical().to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("runtime-path-policy-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn existing_regular_file_must_be_under_state_root() {
        let root_dir = temp_root("existing-under-root");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let file = root_dir.join("requests").join("request.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "{}").unwrap();

        let safe = existing_regular_file_under_root(
            &state_root,
            &file,
            ArtifactPathKind::HostBridgeRequest,
        )
        .unwrap();

        assert_eq!(safe.path(), file.canonicalize().unwrap());
    }

    #[test]
    fn existing_regular_file_rejects_dot_segments() {
        let root_dir = temp_root("dot-segment");
        let state_root = StateRoot::open(root_dir).unwrap();

        let err = existing_regular_file_under_root(
            &state_root,
            "requests/../request.json",
            ArtifactPathKind::HostBridgeRequest,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::DotSegment { .. }));
    }

    #[test]
    fn new_output_path_rejects_existing_without_replace() {
        let root_dir = temp_root("new-output-existing");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let file = root_dir.join("results").join("result.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "{}").unwrap();

        let err = new_output_path_under_root(
            &state_root,
            &file,
            ArtifactPathKind::HostBridgeResult,
            false,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::AlreadyExists { .. }));
    }
}
