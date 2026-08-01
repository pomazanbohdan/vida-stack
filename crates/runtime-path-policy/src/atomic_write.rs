use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(all(test, windows))]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};

use atomic_write_file::AtomicWriteFile;
use cap_std::ambient_authority;
use cap_std::fs::{Dir, Metadata, OpenOptions, Permissions};
use serde::Serialize;

use crate::safe_path::{NewStateOutputPath, PathPolicyError};

pub const HARD_ATOMIC_REPLACE_MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const DEFAULT_ATOMIC_REPLACE_MAX_BYTES: u64 = HARD_ATOMIC_REPLACE_MAX_BYTES;
const MAX_TEMP_FILE_ATTEMPTS: usize = 128;
const STREAM_BUFFER_BYTES: usize = 64 * 1024;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, windows))]
static INJECT_WINDOWS_MOVE_FAILURE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicReplaceLimit(u64);

impl AtomicReplaceLimit {
    #[must_use]
    pub const fn new(requested_max_bytes: u64) -> Self {
        Self(if requested_max_bytes < HARD_ATOMIC_REPLACE_MAX_BYTES {
            requested_max_bytes
        } else {
            HARD_ATOMIC_REPLACE_MAX_BYTES
        })
    }

    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.0
    }
}

impl Default for AtomicReplaceLimit {
    fn default() -> Self {
        Self::new(DEFAULT_ATOMIC_REPLACE_MAX_BYTES)
    }
}

pub fn atomic_replace_bounded(
    destination: impl AsRef<Path>,
    bytes: &[u8],
) -> Result<(), PathPolicyError> {
    atomic_replace_bounded_with_limit(destination, bytes, AtomicReplaceLimit::default())
}

pub fn atomic_replace_bounded_with_limit(
    destination: impl AsRef<Path>,
    bytes: &[u8],
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    atomic_replace_bounded_from_reader(destination, Cursor::new(bytes), limit)
}

pub fn atomic_replace_bounded_from_reader(
    destination: impl AsRef<Path>,
    reader: impl Read,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    let destination = destination.as_ref();
    let kind = crate::safe_path::ArtifactPathKind::GenericJson;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = destination
        .file_name()
        .ok_or_else(|| PathPolicyError::Write {
            kind,
            path: destination.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "atomic replacement destination has no file name",
            ),
        })?;
    validate_atomic_replace_parent(parent, kind)?;
    let parent_dir = Dir::open_ambient_dir(parent, ambient_authority()).map_err(|source| {
        PathPolicyError::Metadata {
            kind,
            path: destination.to_path_buf(),
            source,
        }
    })?;
    let absolute_destination = commit_destination(parent, destination, Path::new(file_name), kind)?;
    atomic_replace_bounded_from_reader_at_impl(
        &parent_dir,
        Path::new(file_name),
        destination,
        &absolute_destination,
        reader,
        limit,
    )
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
        if ambient_metadata_is_link_like(&metadata) {
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

fn ambient_metadata_is_link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || ambient_metadata_is_reparse_point(metadata)
}

#[cfg(windows)]
fn ambient_metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn ambient_metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

pub fn atomic_replace_bounded_from_file(
    destination: impl AsRef<Path>,
    source: impl AsRef<Path>,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    let source = source.as_ref();
    let kind = crate::safe_path::ArtifactPathKind::GenericJson;
    let parent = source
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = source.file_name().ok_or_else(|| PathPolicyError::Read {
        kind,
        path: source.to_path_buf(),
        source: io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic replacement source has no file name",
        ),
    })?;
    let parent_dir = Dir::open_ambient_dir(parent, ambient_authority()).map_err(|error| {
        PathPolicyError::Metadata {
            kind,
            path: source.to_path_buf(),
            source: error,
        }
    })?;
    let source_leaf = Path::new(file_name);
    validate_single_component(source_leaf, kind, source)?;
    validate_atomic_replace_source(&parent_dir, source_leaf, kind, source, limit)?;

    let options = bounded_source_open_options();
    let source_file = parent_dir
        .open_with(source_leaf, &options)
        .map_err(|error| PathPolicyError::Read {
            kind,
            path: source.to_path_buf(),
            source: error,
        })?;
    validate_opened_atomic_replace_source(&source_file, kind, source, limit)?;

    atomic_replace_bounded_from_reader(destination, source_file, limit)
}

fn commit_destination(
    parent: &Path,
    destination: &Path,
    leaf: &Path,
    kind: crate::safe_path::ArtifactPathKind,
) -> Result<PathBuf, PathPolicyError> {
    #[cfg(windows)]
    {
        return parent
            .canonicalize()
            .map(|parent| parent.join(leaf))
            .map_err(|source| PathPolicyError::Metadata {
                kind,
                path: destination.to_path_buf(),
                source,
            });
    }
    #[cfg(not(windows))]
    {
        let _ = (parent, kind);
        Ok(destination.to_path_buf())
    }
}

fn atomic_replace_bounded_from_reader_at_impl(
    parent_dir: &Dir,
    destination: &Path,
    error_path: &Path,
    absolute_destination: &Path,
    mut reader: impl Read,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    let kind = crate::safe_path::ArtifactPathKind::GenericJson;
    validate_single_component(destination, kind, error_path)?;
    let existing_permissions =
        validate_atomic_replace_destination(parent_dir, destination, kind, error_path)?
            .map(|metadata| metadata.permissions());
    let mut temp = create_temp_file(parent_dir, destination, kind, error_path)?;

    let write_result = (|| {
        let mut buffer = [0_u8; STREAM_BUFFER_BYTES];
        let mut written = 0_u64;
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|source| PathPolicyError::Read {
                    kind,
                    path: error_path.to_path_buf(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            let next_written = written.saturating_add(read as u64);
            if next_written > limit.max_bytes() {
                return Err(PathPolicyError::TooLarge {
                    kind,
                    path: error_path.to_path_buf(),
                    max_bytes: limit.max_bytes(),
                });
            }
            temp.file_mut()
                .write_all(&buffer[..read])
                .map_err(|source| PathPolicyError::Write {
                    kind,
                    path: error_path.to_path_buf(),
                    source,
                })?;
            written = next_written;
        }
        if let Some(permissions) = existing_permissions.as_ref() {
            temp.file()
                .set_permissions(permissions.clone())
                .map_err(|source| PathPolicyError::Write {
                    kind,
                    path: error_path.to_path_buf(),
                    source,
                })?;
        }
        temp.file()
            .sync_all()
            .map_err(|source| PathPolicyError::Write {
                kind,
                path: error_path.to_path_buf(),
                source,
            })
    })();

    if let Err(error) = write_result {
        temp.cleanup(parent_dir);
        return Err(error);
    }

    let destination_exists =
        match validate_atomic_replace_destination(parent_dir, destination, kind, error_path) {
            Ok(metadata) => metadata.is_some(),
            Err(error) => {
                temp.cleanup(parent_dir);
                return Err(error);
            }
        };
    match temp.rename_into(
        parent_dir,
        destination,
        absolute_destination,
        destination_exists,
        existing_permissions.as_ref(),
    ) {
        Ok(()) => sync_atomic_replace_parent(parent_dir, kind, error_path),
        Err(failure) => {
            let source = failure.source;
            failure.temp.cleanup(parent_dir);
            Err(PathPolicyError::Write {
                kind,
                path: error_path.to_path_buf(),
                source,
            })
        }
    }
}

fn validate_single_component(
    destination: &Path,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
) -> Result<(), PathPolicyError> {
    let mut components = destination.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(PathPolicyError::Write {
            kind,
            path: error_path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cap-dir atomic replacement requires one relative file-name component",
            ),
        });
    }
    Ok(())
}

fn validate_atomic_replace_destination(
    parent_dir: &Dir,
    destination: &Path,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
) -> Result<Option<Metadata>, PathPolicyError> {
    match parent_dir.symlink_metadata(destination) {
        Ok(metadata) => {
            if metadata_is_link_like(&metadata) {
                return Err(PathPolicyError::Symlink {
                    kind,
                    path: error_path.to_path_buf(),
                });
            }
            if !metadata.is_file() {
                return Err(PathPolicyError::NotRegularFile {
                    kind,
                    path: error_path.to_path_buf(),
                });
            }
            Ok(Some(metadata))
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(PathPolicyError::Metadata {
            kind,
            path: error_path.to_path_buf(),
            source,
        }),
    }
}

fn validate_atomic_replace_source(
    parent_dir: &Dir,
    source: &Path,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    let metadata =
        parent_dir
            .symlink_metadata(source)
            .map_err(|source| PathPolicyError::Metadata {
                kind,
                path: error_path.to_path_buf(),
                source,
            })?;
    validate_atomic_replace_source_metadata(&metadata, kind, error_path, limit)
}

fn validate_opened_atomic_replace_source(
    source: &cap_std::fs::File,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    let metadata = source
        .metadata()
        .map_err(|source| PathPolicyError::Metadata {
            kind,
            path: error_path.to_path_buf(),
            source,
        })?;
    validate_atomic_replace_source_metadata(&metadata, kind, error_path, limit)
}

fn validate_atomic_replace_source_metadata(
    metadata: &Metadata,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
    limit: AtomicReplaceLimit,
) -> Result<(), PathPolicyError> {
    if metadata_is_link_like(metadata) {
        return Err(PathPolicyError::Symlink {
            kind,
            path: error_path.to_path_buf(),
        });
    }
    if !metadata.is_file() {
        return Err(PathPolicyError::NotRegularFile {
            kind,
            path: error_path.to_path_buf(),
        });
    }
    if metadata.len() > limit.max_bytes() {
        return Err(PathPolicyError::TooLarge {
            kind,
            path: error_path.to_path_buf(),
            max_bytes: limit.max_bytes(),
        });
    }
    Ok(())
}

fn bounded_source_open_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    apply_bounded_source_open_options(&mut options);
    options
}

#[cfg(unix)]
fn apply_bounded_source_open_options(options: &mut OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
}

#[cfg(windows)]
fn apply_bounded_source_open_options(options: &mut OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(not(any(unix, windows)))]
fn apply_bounded_source_open_options(_options: &mut OpenOptions) {}

struct OwnedTempFile {
    name: PathBuf,
    file: Option<cap_std::fs::File>,
}

struct AtomicRenameFailure {
    source: std::io::Error,
    temp: OwnedTempFile,
}

impl OwnedTempFile {
    fn file(&self) -> &cap_std::fs::File {
        self.file
            .as_ref()
            .expect("owned temp file handle is present")
    }

    fn file_mut(&mut self) -> &mut cap_std::fs::File {
        self.file
            .as_mut()
            .expect("owned temp file handle is present")
    }

    fn rename_into(
        self,
        parent_dir: &Dir,
        destination: &Path,
        absolute_destination: &Path,
        destination_exists: bool,
        original_permissions: Option<&Permissions>,
    ) -> Result<(), AtomicRenameFailure> {
        #[cfg(not(windows))]
        let _ = (
            absolute_destination,
            destination_exists,
            original_permissions,
        );
        #[cfg(windows)]
        {
            let restore_readonly =
                destination_exists && original_permissions.is_some_and(Permissions::readonly);
            if restore_readonly {
                let mut writable = original_permissions
                    .expect("readonly destination has original permissions")
                    .clone();
                writable.set_readonly(false);
                if let Err(source) = parent_dir.set_permissions(destination, writable) {
                    return Err(AtomicRenameFailure { source, temp: self });
                }
            }
            let temp_path = absolute_destination
                .parent()
                .and_then(|parent| parent.canonicalize().ok())
                .map(|parent| parent.join(&self.name));
            let result = match temp_path {
                Some(temp_path) => {
                    debug_assert!(self.file.is_some());
                    if take_windows_move_failure_injection() {
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            "injected Windows atomic move failure",
                        ))
                    } else {
                        replace_file_windows(&temp_path, absolute_destination)
                    }
                }
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "atomic replacement requires an absolute canonical parent",
                )),
            };
            return match result {
                Ok(()) => Ok(()),
                Err(mut source) => {
                    if restore_readonly {
                        if let Err(restore_error) = restore_windows_destination_permissions(
                            parent_dir,
                            destination,
                            original_permissions
                                .expect("readonly destination has original permissions")
                                .clone(),
                        ) {
                            source = io::Error::new(
                                restore_error.kind(),
                                format!(
                                    "Windows atomic move failed: {source}; permission restoration failed: {restore_error}"
                                ),
                            );
                        }
                    }
                    Err(AtomicRenameFailure { source, temp: self })
                }
            };
        }

        #[cfg(not(windows))]
        {
            match parent_dir.rename(&self.name, parent_dir, destination) {
                Ok(()) => Ok(()),
                Err(source) => Err(AtomicRenameFailure { source, temp: self }),
            }
        }
    }

    fn cleanup(self, parent_dir: &Dir) {
        drop(self.file);
        let _ = parent_dir.remove_file(&self.name);
    }
}

#[cfg(all(test, windows))]
fn take_windows_move_failure_injection() -> bool {
    INJECT_WINDOWS_MOVE_FAILURE.swap(false, Ordering::SeqCst)
}

#[cfg(all(not(test), windows))]
fn take_windows_move_failure_injection() -> bool {
    false
}

#[cfg(windows)]
fn restore_windows_destination_permissions(
    parent_dir: &Dir,
    destination: &Path,
    permissions: Permissions,
) -> io::Result<()> {
    match parent_dir.symlink_metadata(destination) {
        Ok(metadata) if !metadata_is_link_like(&metadata) && metadata.is_file() => {
            parent_dir.set_permissions(destination, permissions)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "refusing to restore permissions through a replaced destination",
        )),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(source),
    }
}

fn create_temp_file(
    parent_dir: &Dir,
    destination: &Path,
    kind: crate::safe_path::ArtifactPathKind,
    error_path: &Path,
) -> Result<OwnedTempFile, PathPolicyError> {
    let file_name = destination
        .file_name()
        .expect("single-component destination has a file name")
        .to_string_lossy();
    for _ in 0..MAX_TEMP_FILE_ATTEMPTS {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_name = PathBuf::from(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        match parent_dir.open_with(&temp_name, &options) {
            Ok(file) => {
                return Ok(OwnedTempFile {
                    name: temp_name,
                    file: Some(file),
                });
            }
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(PathPolicyError::Write {
                    kind,
                    path: error_path.to_path_buf(),
                    source,
                });
            }
        }
    }
    Err(PathPolicyError::Write {
        kind,
        path: error_path.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a collision-free atomic replacement temp file",
        ),
    })
}

fn sync_atomic_replace_parent(
    parent_dir: &Dir,
    kind: crate::safe_path::ArtifactPathKind,
    destination: &Path,
) -> Result<(), PathPolicyError> {
    #[cfg(unix)]
    {
        parent_dir
            .try_clone()
            .map(Dir::into_std_file)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| PathPolicyError::Write {
                kind,
                path: destination.to_path_buf(),
                source,
            })
    }
    #[cfg(windows)]
    {
        // Windows durability is completed by the successful
        // MoveFileExW WRITE_THROUGH rename immediately before this call.
        let _ = (parent_dir, kind, destination);
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (parent_dir, kind, destination);
        Ok(())
    }
}

fn metadata_is_link_like(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use cap_std::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &Metadata) -> bool {
    false
}

/// Replaces a same-directory temporary file without deleting the destination first.
///
/// Safety is confined to this safe wrapper: callers provide validated regular paths, the
/// temporary file was created with `create_new`, and both paths are required to share a canonical
/// parent before the Windows API is called. The UTF-16 buffers remain alive for the duration of
/// the FFI calls and contain no embedded NUL code units.
#[cfg(windows)]
fn replace_file_windows(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    if !temporary_path.is_absolute() || !destination.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows atomic replacement requires absolute paths",
        ));
    }
    let temporary_parent = temporary_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "temporary path has no parent"))?
        .canonicalize()?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "destination path has no parent",
            )
        })?
        .canonicalize()?;
    if temporary_parent != destination_parent {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows atomic replacement paths are not in the same directory",
        ));
    }

    fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.is_empty() || wide.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Windows atomic replacement path is empty or contains NUL",
            ));
        }
        wide.push(0);
        Ok(wide)
    }

    let temporary = wide_path(temporary_path)?;
    let destination = wide_path(destination)?;
    let succeeded = unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) != 0
    };
    if succeeded {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
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

        let error = atomic_replace_bounded_with_limit(
            &destination,
            b"too large",
            AtomicReplaceLimit::new(3),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PathPolicyError::TooLarge { max_bytes: 3, .. }
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[test]
    fn atomic_replace_limit_never_exceeds_hard_cap() {
        assert_eq!(
            AtomicReplaceLimit::new(u64::MAX).max_bytes(),
            HARD_ATOMIC_REPLACE_MAX_BYTES
        );
        assert_eq!(AtomicReplaceLimit::new(7).max_bytes(), 7);
    }

    #[test]
    fn atomic_replace_reader_rejects_oversize_without_mutation_or_temp() {
        let root = temp_root("reader-oversize");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"unchanged").unwrap();

        let error = atomic_replace_bounded_from_reader(
            &destination,
            Cursor::new(b"streamed payload"),
            AtomicReplaceLimit::new(8),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PathPolicyError::TooLarge { max_bytes: 8, .. }
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[test]
    fn atomic_replace_cap_dir_skips_colliding_temp_name() {
        let root = temp_root("collision");
        let destination = root.join("result.bin");
        let next_sequence = TEMP_FILE_SEQUENCE.load(Ordering::Relaxed);
        let collision = format!(".result.bin.{}.{}.tmp", std::process::id(), next_sequence);
        std::fs::write(root.join(&collision), b"owned collision").unwrap();

        atomic_replace_bounded_from_reader(
            &destination,
            Cursor::new(b"replacement"),
            AtomicReplaceLimit::default(),
        )
        .unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
        assert_eq!(
            std::fs::read(root.join(&collision)).unwrap(),
            b"owned collision"
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    }

    #[test]
    fn atomic_replace_from_file_rejects_oversize_without_mutation_or_temp() {
        let root = temp_root("source-oversize");
        let source = root.join("source.bin");
        let destination = root.join("result.bin");
        std::fs::write(&source, b"streamed payload").unwrap();
        std::fs::write(&destination, b"unchanged").unwrap();

        let error =
            atomic_replace_bounded_from_file(&destination, &source, AtomicReplaceLimit::new(8))
                .unwrap_err();

        assert!(matches!(
            error,
            PathPolicyError::TooLarge { max_bytes: 8, .. }
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_from_file_rejects_symlink_source_without_mutation() {
        use std::os::unix::fs::symlink;

        let root = temp_root("source-symlink");
        let source_target = root.join("source-target.bin");
        let source_link = root.join("source-link.bin");
        let destination = root.join("result.bin");
        std::fs::write(&source_target, b"source contents").unwrap();
        symlink(&source_target, &source_link).unwrap();
        std::fs::write(&destination, b"unchanged").unwrap();

        let error = atomic_replace_bounded_from_file(
            &destination,
            &source_link,
            AtomicReplaceLimit::default(),
        )
        .unwrap_err();

        assert!(matches!(error, PathPolicyError::Symlink { .. }));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert!(
            std::fs::symlink_metadata(&source_link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_from_file_rejects_reparse_source_without_mutation() {
        use std::os::windows::fs::symlink_file;

        let root = temp_root("source-reparse");
        let source_target = root.join("source-target.bin");
        let source_link = root.join("source-link.bin");
        let destination = root.join("result.bin");
        std::fs::write(&source_target, b"source contents").unwrap();
        match symlink_file(&source_target, &source_link) {
            Ok(()) => {}
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                return;
            }
            Err(error) => panic!("source reparse point should be created: {error}"),
        }
        std::fs::write(&destination, b"unchanged").unwrap();

        let error = atomic_replace_bounded_from_file(
            &destination,
            &source_link,
            AtomicReplaceLimit::default(),
        )
        .unwrap_err();

        assert!(matches!(error, PathPolicyError::Symlink { .. }));
        assert_eq!(std::fs::read(&destination).unwrap(), b"unchanged");
        assert!(metadata_is_link_like(
            &Dir::open_ambient_dir(&root, ambient_authority())
                .unwrap()
                .symlink_metadata("source-link.bin")
                .unwrap()
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn atomic_replace_failed_rename_keeps_handle_until_identity_owned_cleanup() {
        let root = temp_root("failed-rename-handle");
        let parent_dir = Dir::open_ambient_dir(&root, ambient_authority()).unwrap();
        parent_dir.create_dir("occupied").unwrap();
        let temp = create_temp_file(
            &parent_dir,
            Path::new("result.bin"),
            ArtifactPathKind::GenericJson,
            Path::new("result.bin"),
        )
        .unwrap();
        let temp_name = temp.name.clone();
        let occupied = root.join("occupied");

        let failure = temp
            .rename_into(&parent_dir, Path::new("occupied"), &occupied, true, None)
            .expect_err("renaming a file over a directory must fail");

        assert!(failure.temp.file.is_some());
        assert!(failure.temp.file().metadata().is_ok());
        assert!(parent_dir.symlink_metadata(&temp_name).unwrap().is_file());
        failure.temp.cleanup(&parent_dir);
        assert!(
            parent_dir
                .symlink_metadata(&temp_name)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        );
        assert!(parent_dir.symlink_metadata("occupied").unwrap().is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_windows_failed_replace_cleans_temp() {
        let root = temp_root("windows-failed-replace");
        let parent_dir = Dir::open_ambient_dir(&root, ambient_authority()).unwrap();
        parent_dir.create_dir("occupied").unwrap();
        let temp = create_temp_file(
            &parent_dir,
            Path::new("result.bin"),
            ArtifactPathKind::GenericJson,
            Path::new("result.bin"),
        )
        .unwrap();
        let temp_name = temp.name.clone();
        let occupied = root.join("occupied");

        let failure = temp
            .rename_into(&parent_dir, Path::new("occupied"), &occupied, true, None)
            .expect_err("replacing a directory must fail");
        failure.temp.cleanup(&parent_dir);

        assert!(
            parent_dir
                .symlink_metadata(&temp_name)
                .is_err_and(|error| { error.kind() == std::io::ErrorKind::NotFound })
        );
        assert!(parent_dir.symlink_metadata("occupied").unwrap().is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_windows_write_through_replaces_with_live_temp_handle() {
        let root = temp_root("windows-write-through");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"old").unwrap();

        atomic_replace_bounded(&destination, b"replacement").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_windows_preserves_readonly_on_existing_destination() {
        let root = temp_root("windows-readonly");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"old").unwrap();
        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&destination, permissions).unwrap();

        atomic_replace_bounded(&destination, b"replacement").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        assert!(permissions.readonly());
        permissions.set_readonly(false);
        std::fs::set_permissions(&destination, permissions).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_windows_restores_readonly_after_move_failure() {
        let root = temp_root("windows-readonly-restore");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"old").unwrap();
        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&destination, permissions).unwrap();
        let parent_dir = Dir::open_ambient_dir(&root, ambient_authority()).unwrap();
        let original_permissions = parent_dir
            .symlink_metadata("result.bin")
            .unwrap()
            .permissions();
        let mut temp = create_temp_file(
            &parent_dir,
            Path::new("result.bin"),
            ArtifactPathKind::GenericJson,
            Path::new("result.bin"),
        )
        .unwrap();
        temp.file_mut().write_all(b"replacement").unwrap();
        temp.file()
            .set_permissions(original_permissions.clone())
            .unwrap();
        temp.file().sync_all().unwrap();
        INJECT_WINDOWS_MOVE_FAILURE.store(true, Ordering::SeqCst);

        let failure = temp
            .rename_into(
                &parent_dir,
                Path::new("result.bin"),
                &destination,
                true,
                Some(&original_permissions),
            )
            .expect_err("injected move failure must be returned");

        assert!(
            failure
                .source
                .to_string()
                .contains("injected Windows atomic move failure")
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"old");
        let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
        assert!(permissions.readonly());
        failure.temp.cleanup(&parent_dir);
        permissions.set_readonly(false);
        std::fs::set_permissions(&destination, permissions).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_windows_write_through_creates_with_live_temp_handle() {
        let root = temp_root("windows-write-through-new");
        let destination = root.join("result.bin");

        atomic_replace_bounded(&destination, b"new").unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_parent_handle_sync_is_supported() {
        let root = temp_root("parent-sync");
        let parent_dir = Dir::open_ambient_dir(&root, ambient_authority()).unwrap();

        sync_atomic_replace_parent(
            &parent_dir,
            ArtifactPathKind::GenericJson,
            Path::new("result.bin"),
        )
        .unwrap();
    }

    #[test]
    fn atomic_replace_reader_error_preserves_destination_and_cleans_temp() {
        struct InterruptedReader {
            emitted: bool,
        }

        impl Read for InterruptedReader {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if self.emitted {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "injected reader interruption",
                    ));
                }
                self.emitted = true;
                buffer[..4].copy_from_slice(b"part");
                Ok(4)
            }
        }

        let root = temp_root("reader-interruption");
        let destination = root.join("result.bin");
        std::fs::write(&destination, b"unchanged").unwrap();

        let error = atomic_replace_bounded_from_reader(
            &destination,
            InterruptedReader { emitted: false },
            AtomicReplaceLimit::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("injected reader interruption"));
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
        assert!(
            std::fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_bounded_rejects_symlink_parent_without_writing_target() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink-parent");
        let outside = temp_root("symlink-parent-outside");
        let linked_parent = root.join("state-link");
        let destination = linked_parent.join("journal.json");
        let outside_destination = outside.join("journal.json");
        symlink(&outside, &linked_parent).unwrap();

        let error = atomic_replace_bounded(&destination, b"redirected").unwrap_err();

        assert!(matches!(error, PathPolicyError::Symlink { path, .. } if path == linked_parent));
        assert!(!outside_destination.exists());
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_bounded_rejects_reparse_parent_without_writing_target() {
        use std::os::windows::fs::symlink_dir;

        let root = temp_root("reparse-parent");
        let outside = temp_root("reparse-parent-outside");
        let linked_parent = root.join("state-link");
        let destination = linked_parent.join("journal.json");
        let outside_destination = outside.join("journal.json");
        match symlink_dir(&outside, &linked_parent) {
            Ok(()) => {}
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                return;
            }
            Err(error) => panic!("parent reparse point should be created: {error}"),
        }

        let error = atomic_replace_bounded(&destination, b"redirected").unwrap_err();

        assert!(matches!(error, PathPolicyError::Symlink { path, .. } if path == linked_parent));
        assert!(!outside_destination.exists());
    }
}
