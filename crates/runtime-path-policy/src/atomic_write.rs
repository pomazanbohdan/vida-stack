use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::safe_path::{NewStateOutputPath, PathPolicyError};

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
    let temp_path = temp_sibling_path(path);
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| PathPolicyError::Write {
                kind: path.kind(),
                path: temp_path.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .map_err(|source| PathPolicyError::Write {
                kind: path.kind(),
                path: temp_path.clone(),
                source,
            })?;
        file.write_all(b"\n")
            .map_err(|source| PathPolicyError::Write {
                kind: path.kind(),
                path: temp_path.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| PathPolicyError::Write {
            kind: path.kind(),
            path: temp_path.clone(),
            source,
        })?;
    }
    std::fs::rename(&temp_path, path.path()).map_err(|source| {
        let _ = std::fs::remove_file(&temp_path);
        PathPolicyError::Write {
            kind: path.kind(),
            path: path.path().to_path_buf(),
            source,
        }
    })
}

fn temp_sibling_path(path: &NewStateOutputPath) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .path()
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("artifact");
    path.path()
        .with_file_name(format!(".{file_name}.{}.{}.tmp", std::process::id(), nanos))
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

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
        std::fs::create_dir_all(&root).unwrap();
        let state_root = StateRoot::open(&root).unwrap();
        let output = new_output_path_under_root(
            &state_root,
            "results/result.json",
            ArtifactPathKind::HostBridgeResult,
            false,
        )
        .unwrap();

        write_json_new(&output, &Payload { value: "ok" }).unwrap();

        let written = std::fs::read_to_string(root.join("results").join("result.json")).unwrap();
        assert!(written.contains("\"value\": \"ok\""));
    }
}
