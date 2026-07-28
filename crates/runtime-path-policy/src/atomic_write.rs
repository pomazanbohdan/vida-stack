#[cfg(unix)]
use std::fs::File;
use std::fs::{self, Metadata};
use std::io::Write;
use std::path::Path;

use atomic_write_file::AtomicWriteFile;
use serde::Serialize;

use crate::safe_path::{NewStateOutputPath, PathPolicyError};

pub const DEFAULT_ATOMIC_REPLACE_MAX_BYTES: u64 = 64 * 1024 * 1024;

pub fn atomic_replace_bounded(
    destination: impl AsRef<Path>,
    bytes: &[u8],
) -> Result<(), PathPolicyError> {
    atomic_replace_bounded_with_limit(destination, bytes, DEFAULT_ATOMIC_REPLACE_MAX_BYTES)
}

pub fn atomic_replace_bounded_with_limit(
    destination: impl AsRef<Path>,
    bytes: &[u8],
    max_bytes: u64,
) -> Result<(), PathPolicyError> {
    let destination = destination.as_ref();
    let kind = crate::safe_path::ArtifactPathKind::GenericJson;
    if bytes.len() as u64 > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind,
            path: destination.to_path_buf(),
            max_bytes,
        });
    }

    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    validate_atomic_replace_parent(parent, kind)?;
    let existing_permissions = validate_atomic_replace_destination(destination, kind)?
        .map(|metadata| metadata.permissions());

    #[cfg(unix)]
    let mut options = AtomicWriteFile::options();
    #[cfg(not(unix))]
    let options = AtomicWriteFile::options();
    #[cfg(unix)]
    atomic_write_file::unix::OpenOptionsExt::preserve_mode(&mut options, true);
    let mut file = options
        .open(destination)
        .map_err(|source| PathPolicyError::Write {
            kind,
            path: destination.to_path_buf(),
            source,
        })?;
    if let Some(permissions) = existing_permissions {
        file.set_permissions(permissions)
            .map_err(|source| PathPolicyError::Write {
                kind,
                path: destination.to_path_buf(),
                source,
            })?;
    }
    file.write_all(bytes)
        .map_err(|source| PathPolicyError::Write {
            kind,
            path: destination.to_path_buf(),
            source,
        })?;
    validate_atomic_replace_destination(destination, kind)?;
    file.commit().map_err(|source| PathPolicyError::Write {
        kind,
        path: destination.to_path_buf(),
        source,
    })?;
    sync_atomic_replace_parent(parent, kind, destination)
}

fn validate_atomic_replace_parent(
    parent: &Path,
    kind: crate::safe_path::ArtifactPathKind,
) -> Result<(), PathPolicyError> {
    let mut current = Some(parent);
    while let Some(path) = current {
        let metadata = fs::symlink_metadata(path).map_err(|source| PathPolicyError::Metadata {
            kind,
            path: path.to_path_buf(),
            source,
        })?;
        if metadata_is_link_like(&metadata) {
            return Err(PathPolicyError::Symlink {
                kind,
                path: path.to_path_buf(),
            });
        }
        if !metadata.is_dir() {
            return Err(PathPolicyError::NotRegularFile {
                kind,
                path: path.to_path_buf(),
            });
        }
        current = path.parent();
    }
    Ok(())
}

fn validate_atomic_replace_destination(
    destination: &Path,
    kind: crate::safe_path::ArtifactPathKind,
) -> Result<Option<Metadata>, PathPolicyError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if metadata_is_link_like(&metadata) {
                return Err(PathPolicyError::Symlink {
                    kind,
                    path: destination.to_path_buf(),
                });
            }
            if !metadata.is_file() {
                return Err(PathPolicyError::NotRegularFile {
                    kind,
                    path: destination.to_path_buf(),
                });
            }
            Ok(Some(metadata))
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(PathPolicyError::Metadata {
            kind,
            path: destination.to_path_buf(),
            source,
        }),
    }
}

fn sync_atomic_replace_parent(
    parent: &Path,
    kind: crate::safe_path::ArtifactPathKind,
    destination: &Path,
) -> Result<(), PathPolicyError> {
    #[cfg(unix)]
    {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| PathPolicyError::Write {
                kind,
                path: destination.to_path_buf(),
                source,
            })?;
    }
    #[cfg(not(unix))]
    let _ = (parent, kind, destination);
    Ok(())
}

fn metadata_is_link_like(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &Metadata) -> bool {
    false
}

pub fn write_json_new<T: Serialize>(
    path: &NewStateOutputPath,
    value: &T,
) -> Result<(), PathPolicyError> {
    write_json(path, value)
}

pub fn write_json_replace<T: Serialize>(
    path: &NewStateOutputPath,
    value: &T,
) -> Result<(), PathPolicyError> {
    write_json(path, value)
}

fn write_json<T: Serialize>(path: &NewStateOutputPath, value: &T) -> Result<(), PathPolicyError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| PathPolicyError::Json {
        kind: path.kind(),
        path: path.path().to_path_buf(),
        source,
    })?;
    let mut file = AtomicWriteFile::open(path.path()).map_err(|source| PathPolicyError::Write {
        kind: path.kind(),
        path: path.path().to_path_buf(),
        source,
    })?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|source| PathPolicyError::Write {
            kind: path.kind(),
            path: path.path().to_path_buf(),
            source,
        })?;
    file.commit().map_err(|source| PathPolicyError::Write {
        kind: path.kind(),
        path: path.path().to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use std::path::PathBuf;

    use super::*;
    use crate::safe_path::{ArtifactPathKind, new_output_path_under_root};
    use crate::state_root::StateRoot;

    #[derive(Serialize)]
    struct Payload<'a> {
        value: &'a str,
    }

    #[test]
    fn write_json_new_uses_validated_output_path() {
        let root =
            std::env::temp_dir().join(format!("runtime-path-policy-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        fs_err::create_dir_all(&root).unwrap();
        let state_root = StateRoot::open(&root).unwrap();
        let output = new_output_path_under_root(
            &state_root,
            "results/result.json",
            ArtifactPathKind::HostBridgeResult,
            false,
        )
        .unwrap();

        write_json_new(&output, &Payload { value: "ok" }).unwrap();

        let written = fs_err::read_to_string(root.join("results").join("result.json")).unwrap();
        assert!(written.contains("\"value\": \"ok\""));
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "runtime-path-policy-atomic-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn atomic_replace_bounded_writes_new_destination() {
        let root = temp_root("success");
        let destination = root.join("result.bin");

        atomic_replace_bounded(&destination, b"new contents").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"new contents");
    }

    #[test]
    fn atomic_replace_bounded_replaces_existing_destination() {
        let root = temp_root("replace");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"old contents").unwrap();

        atomic_replace_bounded(&destination, b"replacement").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_bounded_replaces_and_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("permissions");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"old").unwrap();
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o640)).unwrap();

        atomic_replace_bounded(&destination, b"replacement").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
        assert_eq!(
            std::fs::metadata(&destination)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[test]
    fn atomic_replace_bounded_rejects_oversize_without_touching_destination() {
        let root = temp_root("oversize");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"unchanged").unwrap();

        let error = atomic_replace_bounded_with_limit(&destination, b"too large", 3).unwrap_err();

        assert!(matches!(
            error,
            PathPolicyError::TooLarge { max_bytes: 3, .. }
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[test]
    fn atomic_replace_bounded_missing_parent_fails_without_temp_artifact() {
        let root = temp_root("missing-parent");
        let missing_parent = root.join("missing");
        let destination = missing_parent.join("result.bin");

        assert!(atomic_replace_bounded(&destination, b"payload").is_err());
        assert!(!missing_parent.exists());
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_bounded_rejects_symlink_destination_without_touching_target() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink");
        let target = root.join("target.bin");
        let destination = root.join("result.bin");
        std::fs::write(&target, b"target").unwrap();
        symlink(&target, &destination).unwrap();

        let error = atomic_replace_bounded(&destination, b"replacement").unwrap_err();

        assert!(matches!(error, PathPolicyError::Symlink { .. }));
        assert_eq!(std::fs::read(&target).unwrap(), b"target");
        assert!(std::fs::symlink_metadata(&destination)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
