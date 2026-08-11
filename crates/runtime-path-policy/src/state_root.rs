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

#[cfg(test)]
mod tests {
    use super::{StateRoot, path_is_rooted};
    use crate::safe_path::{ArtifactPathKind, PathPolicyError};
    use std::path::{Path, PathBuf};

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "runtime-path-policy-state-root-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn open_rejects_missing_and_non_directory_paths() {
        let missing = std::env::temp_dir().join(format!(
            "runtime-path-policy-state-root-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing);
        let missing_error = StateRoot::open(&missing).unwrap_err();
        assert!(matches!(
            missing_error,
            PathPolicyError::StateRootOpen { .. }
        ));

        let file = std::env::temp_dir().join(format!(
            "runtime-path-policy-state-root-file-{}",
            std::process::id()
        ));
        std::fs::write(&file, b"state").unwrap();
        let file_error = StateRoot::open(&file).unwrap_err();
        assert!(matches!(
            file_error,
            PathPolicyError::StateRootNotDirectory { .. }
        ));
        let _ = std::fs::remove_file(file);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn open_rejects_symlinked_state_roots() {
        let target = temp_root("symlink-target");
        let link = temp_root("symlink-link");
        let _ = std::fs::remove_dir_all(&link);

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("symlink creation");
        #[cfg(windows)]
        match std::os::windows::fs::symlink_dir(&target, &link) {
            Ok(()) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                let _ = std::fs::remove_dir_all(target);
                return;
            }
            Err(error) => panic!("symlink creation failed: {error}"),
        }

        let error = StateRoot::open(&link).unwrap_err();
        assert!(matches!(
            error,
            PathPolicyError::StateRootSymlink { path } if path == link
        ));

        let _ = std::fs::remove_dir_all(link);
        let _ = std::fs::remove_dir_all(target);
    }

    #[test]
    fn resolution_and_canonical_containment_preserve_root_boundaries() {
        let root_dir = temp_root("resolution");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let relative = Path::new("nested/state.json");
        let rooted = root_dir.join(relative);
        let sibling = root_dir.with_file_name(format!(
            "runtime-path-policy-state-root-resolution-sibling-{}",
            std::process::id()
        ));

        assert!(!path_is_rooted(relative));
        assert!(path_is_rooted(&rooted));
        assert_eq!(state_root.resolve_raw(relative), rooted);
        assert_eq!(state_root.resolve_raw(&rooted), rooted);
        assert!(state_root.contains_canonical(&state_root.canonical().join(relative)));
        assert!(!state_root.contains_canonical(&sibling));

        let _ = std::fs::remove_dir_all(root_dir);
    }

    #[test]
    fn open_preserves_raw_and_canonical_path_accessors() {
        let root_dir = temp_root("accessors");
        let expected_canonical = root_dir.canonicalize().unwrap();
        let state_root = StateRoot::open(&root_dir).unwrap();

        assert_eq!(state_root.raw(), root_dir.as_path());
        assert_eq!(state_root.canonical(), expected_canonical.as_path());

        let _ = std::fs::remove_dir_all(root_dir);
    }

    #[cfg(windows)]
    #[test]
    fn resolve_raw_preserves_windows_rooted_paths_without_a_drive_prefix() {
        let root_dir = temp_root("windows-rooted");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let rooted = PathBuf::from(r"\nested\state.json");

        assert!(path_is_rooted(&rooted));
        assert_eq!(state_root.resolve_raw(&rooted), rooted);

        let _ = std::fs::remove_dir_all(root_dir);
    }

    #[test]
    fn cap_relative_path_accepts_rooted_inside_and_rejects_outside() {
        let root_dir = temp_root("cap-relative");
        let state_root = StateRoot::open(&root_dir).unwrap();
        let inside = root_dir.join("nested.json");
        let outside = root_dir.with_file_name(format!(
            "runtime-path-policy-state-root-cap-relative-outside-{}",
            std::process::id()
        ));

        assert_eq!(
            state_root
                .cap_relative_path(&inside, ArtifactPathKind::GenericJson)
                .unwrap(),
            PathBuf::from("nested.json")
        );
        assert!(matches!(
            state_root.cap_relative_path(&outside, ArtifactPathKind::GenericJson),
            Err(PathPolicyError::OutsideStateRoot { .. })
        ));

        let _ = std::fs::remove_dir_all(root_dir);
    }
}
