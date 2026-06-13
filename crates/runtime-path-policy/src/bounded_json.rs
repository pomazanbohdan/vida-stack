use std::io::Read;

use serde::de::DeserializeOwned;

use crate::safe_path::{ExistingRegularFile, PathPolicyError};
use crate::size_limits::{
    DEFAULT_JSON_ARTIFACT_MAX_BYTES, HOST_BRIDGE_REQUEST_MAX_BYTES, HOST_BRIDGE_RESULT_MAX_BYTES,
    TASK_ATTEMPT_ARTIFACT_MAX_BYTES,
};

pub use crate::size_limits::{
    DEFAULT_JSON_ARTIFACT_MAX_BYTES as DEFAULT_JSON_ARTIFACT_LIMIT,
    HOST_BRIDGE_REQUEST_MAX_BYTES as HOST_BRIDGE_REQUEST_LIMIT,
    HOST_BRIDGE_RESULT_MAX_BYTES as HOST_BRIDGE_RESULT_LIMIT,
    TASK_ATTEMPT_ARTIFACT_MAX_BYTES as TASK_ATTEMPT_ARTIFACT_LIMIT,
};

pub fn read_json_file<T: DeserializeOwned>(
    file: &ExistingRegularFile,
    max_bytes: u64,
) -> Result<T, PathPolicyError> {
    let value = read_json_value_file(file, max_bytes)?;
    serde_json::from_value(value).map_err(|source| PathPolicyError::Json {
        kind: file.kind(),
        path: file.path().to_path_buf(),
        source,
    })
}

pub fn read_json_value_file(
    file: &ExistingRegularFile,
    max_bytes: u64,
) -> Result<serde_json::Value, PathPolicyError> {
    let metadata = std::fs::metadata(file.path()).map_err(|source| PathPolicyError::Metadata {
        kind: file.kind(),
        path: file.path().to_path_buf(),
        source,
    })?;
    if metadata.len() > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind: file.kind(),
            path: file.path().to_path_buf(),
            max_bytes,
        });
    }
    let mut handle = std::fs::File::open(file.path()).map_err(|source| PathPolicyError::Read {
        kind: file.kind(),
        path: file.path().to_path_buf(),
        source,
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    handle
        .by_ref()
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| PathPolicyError::Read {
            kind: file.kind(),
            path: file.path().to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(PathPolicyError::TooLarge {
            kind: file.kind(),
            path: file.path().to_path_buf(),
            max_bytes,
        });
    }
    serde_json::from_slice(&bytes).map_err(|source| PathPolicyError::Json {
        kind: file.kind(),
        path: file.path().to_path_buf(),
        source,
    })
}

pub const DEFAULT_JSON_ARTIFACT_MAX_BYTES_ALIAS: u64 = DEFAULT_JSON_ARTIFACT_MAX_BYTES;
pub const HOST_BRIDGE_REQUEST_MAX_BYTES_ALIAS: u64 = HOST_BRIDGE_REQUEST_MAX_BYTES;
pub const HOST_BRIDGE_RESULT_MAX_BYTES_ALIAS: u64 = HOST_BRIDGE_RESULT_MAX_BYTES;
pub const TASK_ATTEMPT_ARTIFACT_MAX_BYTES_ALIAS: u64 = TASK_ATTEMPT_ARTIFACT_MAX_BYTES;

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::safe_path::{ArtifactPathKind, existing_regular_file_under_root};
    use crate::state_root::StateRoot;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Payload {
        value: String,
    }

    #[test]
    fn read_json_file_enforces_size_limit() {
        let root = std::env::temp_dir().join(format!(
            "runtime-path-policy-json-limit-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("payload.json");
        std::fs::write(&path, r#"{"value":"abc"}"#).unwrap();
        let state_root = StateRoot::open(&root).unwrap();
        let file =
            existing_regular_file_under_root(&state_root, &path, ArtifactPathKind::GenericJson)
                .unwrap();

        let err = read_json_file::<Payload>(&file, 4).unwrap_err();
        assert!(matches!(err, PathPolicyError::TooLarge { .. }));
    }

    #[test]
    fn read_json_file_deserializes_bounded_json() {
        let root = std::env::temp_dir().join(format!(
            "runtime-path-policy-json-ok-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("payload.json");
        std::fs::write(&path, r#"{"value":"abc"}"#).unwrap();
        let state_root = StateRoot::open(&root).unwrap();
        let file =
            existing_regular_file_under_root(&state_root, &path, ArtifactPathKind::GenericJson)
                .unwrap();

        let payload = read_json_file::<Payload>(&file, DEFAULT_JSON_ARTIFACT_MAX_BYTES).unwrap();
        assert_eq!(
            payload,
            Payload {
                value: "abc".to_string()
            }
        );
    }
}
