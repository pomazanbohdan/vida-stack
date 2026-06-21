use std::io::Write;

use atomic_write_file::AtomicWriteFile;
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
}
