use std::fmt;
use std::io::{self, Read};
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
    RequirementSourceFile,
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
            Self::RequirementSourceFile => "requirement source file",
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
    let path = path.as_ref();
    path.components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || path
            .as_os_str()
            .to_string_lossy()
            .split(['/', '\\'])
            .any(|segment| matches!(segment, "." | ".."))
}

pub fn existing_regular_file_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
) -> Result<ExistingRegularFile, PathPolicyError> {
    let raw_path = raw_path.as_ref();
    reject_dot_segment(raw_path, kind)?;
    let path = root.resolve_raw(raw_path);
    let cap_path = root.cap_relative_path(raw_path, kind)?;
    let metadata = root
        .cap_dir()
        .symlink_metadata(&cap_path)
        .map_err(|source| PathPolicyError::Metadata {
            kind,
            path: path.clone(),
            source,
        })?;
    if metadata.file_type().is_symlink() {
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

pub fn read_bounded_text_file_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
    max_bytes: u64,
) -> Result<String, PathPolicyError> {
    let raw_path = raw_path.as_ref();
    reject_dot_segment(raw_path, kind)?;
    let path = root.resolve_raw(raw_path);
    let cap_path = root.cap_relative_path(raw_path, kind)?;
    let metadata = root
        .cap_dir()
        .symlink_metadata(&cap_path)
        .map_err(|source| PathPolicyError::Metadata {
            kind,
            path: path.clone(),
            source,
        })?;
    if metadata.file_type().is_symlink() {
        return Err(PathPolicyError::Symlink { kind, path });
    }
    if !metadata.is_file() {
        return Err(PathPolicyError::NotRegularFile { kind, path });
    }
    if metadata.len() > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind,
            path,
            max_bytes,
        });
    }

    let options = bounded_regular_file_open_options();
    let mut file = root
        .cap_dir()
        .open_with(&cap_path, &options)
        .map_err(|source| PathPolicyError::Read {
            kind,
            path: path.clone(),
            source,
        })?;
    let opened_metadata = file
        .metadata()
        .map_err(|source| PathPolicyError::Metadata {
            kind,
            path: path.clone(),
            source,
        })?;
    if !opened_metadata.is_file() {
        return Err(PathPolicyError::NotRegularFile { kind, path });
    }
    if opened_metadata.len() > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind,
            path,
            max_bytes,
        });
    }

    let mut bytes = Vec::new();
    file.by_ref()
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| PathPolicyError::Read {
            kind,
            path: path.clone(),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind,
            path,
            max_bytes,
        });
    }

    String::from_utf8(bytes).map_err(|source| PathPolicyError::Read {
        kind,
        path,
        source: io::Error::new(io::ErrorKind::InvalidData, source),
    })
}

fn bounded_regular_file_open_options() -> cap_std::fs::OpenOptions {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true);
    apply_bounded_regular_file_open_options(&mut options);
    options
}

#[cfg(unix)]
fn apply_bounded_regular_file_open_options(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
}

#[cfg(windows)]
fn apply_bounded_regular_file_open_options(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(not(any(unix, windows)))]
fn apply_bounded_regular_file_open_options(_options: &mut cap_std::fs::OpenOptions) {}

pub fn new_output_path_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
    replace_existing: bool,
) -> Result<NewStateOutputPath, PathPolicyError> {
    let raw_path = raw_path.as_ref();
    reject_dot_segment(raw_path, kind)?;
    let path = root.resolve_raw(raw_path);
    let cap_path = root.cap_relative_path(raw_path, kind)?;
    let parent = path
        .parent()
        .ok_or_else(|| PathPolicyError::MissingParent {
            kind,
            path: path.clone(),
        })?;
    ensure_parent_safe_to_create(root, parent, kind)?;
    let cap_parent = cap_path
        .parent()
        .ok_or_else(|| PathPolicyError::MissingParent {
            kind,
            path: path.clone(),
        })?;
    root.cap_dir()
        .create_dir_all(cap_parent)
        .map_err(|source| PathPolicyError::ParentCreate {
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
    if let Ok(metadata) = root.cap_dir().symlink_metadata(&cap_path) {
        if metadata.file_type().is_symlink() {
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

fn ensure_parent_safe_to_create(
    root: &StateRoot,
    parent: &Path,
    kind: ArtifactPathKind,
) -> Result<(), PathPolicyError> {
    let mut existing_ancestor = parent;
    while !existing_ancestor.exists() {
        existing_ancestor =
            existing_ancestor
                .parent()
                .ok_or_else(|| PathPolicyError::MissingParent {
                    kind,
                    path: parent.to_path_buf(),
                })?;
    }
    let metadata = fs_err::symlink_metadata(existing_ancestor).map_err(|source| {
        PathPolicyError::Metadata {
            kind,
            path: existing_ancestor.to_path_buf(),
            source,
        }
    })?;
    if is_symlink(&metadata) {
        return Err(PathPolicyError::Symlink {
            kind,
            path: existing_ancestor.to_path_buf(),
        });
    }
    if !metadata.is_dir() {
        return Err(PathPolicyError::NotRegularFile {
            kind,
            path: existing_ancestor.to_path_buf(),
        });
    }
    let canonical =
        existing_ancestor
            .canonicalize()
            .map_err(|source| PathPolicyError::Canonicalize {
                kind,
                path: existing_ancestor.to_path_buf(),
                source,
            })?;
    ensure_under_root(root, &canonical, kind)
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

    #[cfg(windows)]
    #[test]
    fn existing_regular_file_accepts_windows_rooted_path_under_raw_state_root() {
        let root_dir = PathBuf::from(format!(
            "/tmp/runtime-path-policy-rooted-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root_dir);
        std::fs::create_dir_all(&root_dir).unwrap();
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
        let _ = std::fs::remove_dir_all(&root_dir);
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
    fn bounded_text_file_reads_within_limit() {
        let root_dir = temp_root("bounded-text-file");
        let state_root = StateRoot::open(&root_dir).unwrap();
        std::fs::write(root_dir.join("requirements.md"), "Build the feature.").unwrap();

        let text = read_bounded_text_file_under_root(
            &state_root,
            "requirements.md",
            ArtifactPathKind::RequirementSourceFile,
            64,
        )
        .unwrap();

        assert_eq!(text, "Build the feature.");
    }

    #[test]
    fn bounded_text_file_enforces_limit_after_open() {
        let root_dir = temp_root("bounded-text-limit");
        let state_root = StateRoot::open(&root_dir).unwrap();
        std::fs::write(root_dir.join("requirements.md"), "12345").unwrap();

        let err = read_bounded_text_file_under_root(
            &state_root,
            "requirements.md",
            ArtifactPathKind::RequirementSourceFile,
            4,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::TooLarge { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_text_file_rejects_fifo_without_blocking() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let root_dir = temp_root("bounded-text-fifo");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let fifo = root_dir.join("requirements.md");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        let rc = unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) };
        assert_eq!(rc, 0);

        let err = read_bounded_text_file_under_root(
            &state_root,
            "requirements.md",
            ArtifactPathKind::RequirementSourceFile,
            64,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::NotRegularFile { .. }));
    }

    #[test]
    fn path_contains_dot_segment_rejects_middle_current_dir_segment() {
        assert!(path_contains_dot_segment("requests/./request.json"));
        assert!(path_contains_dot_segment("requests\\..\\request.json"));
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

    #[test]
    fn new_output_path_rejects_outside_parent_before_create() {
        let root_dir = temp_root("new-output-outside-parent");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let outside_root = root_dir.with_file_name(format!(
            "{}-outside",
            root_dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&outside_root);
        std::fs::create_dir_all(&outside_root).unwrap();
        let outside_child = outside_root.join("created-by-bug");
        let target = outside_child.join("result.json");

        let err = new_output_path_under_root(
            &state_root,
            &target,
            ArtifactPathKind::HostBridgeResult,
            true,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::OutsideStateRoot { .. }));
        assert!(
            !outside_child.exists(),
            "outside parent must not be created before state-root validation"
        );
        let _ = std::fs::remove_dir_all(&root_dir);
        let _ = std::fs::remove_dir_all(&outside_root);
    }

    #[cfg(unix)]
    #[test]
    fn new_output_path_rejects_symlink_parent_before_create() {
        let root_dir = temp_root("new-output-symlink-parent");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let outside_root = root_dir.with_file_name(format!(
            "{}-outside",
            root_dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&outside_root);
        std::fs::create_dir_all(&outside_root).unwrap();
        let link_parent = root_dir.join("linked-parent");
        std::os::unix::fs::symlink(&outside_root, &link_parent).unwrap();

        let err = new_output_path_under_root(
            &state_root,
            link_parent.join("result.json"),
            ArtifactPathKind::HostBridgeResult,
            true,
        )
        .unwrap_err();

        assert!(matches!(err, PathPolicyError::Symlink { .. }));
        let _ = std::fs::remove_dir_all(&root_dir);
        let _ = std::fs::remove_dir_all(&outside_root);
    }
}
