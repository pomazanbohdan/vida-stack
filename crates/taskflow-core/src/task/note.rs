//! Pure task note and text-source policy helpers.

use std::path::Path;

pub const MAX_OPTIONAL_TEXT_FILE_BYTES: u64 = 64 * 1024;

pub fn resolve_optional_text_arg(
    label: &str,
    direct: Option<&str>,
    file_path: Option<&Path>,
) -> Result<Option<String>, String> {
    if direct.is_some() && file_path.is_some() {
        return Err(format!(
            "Use only one {label} source: --{label} <text> or --{label}-file <path>"
        ));
    }
    if let Some(path) = file_path {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            format!(
                "Failed to inspect {label} file `{}` metadata: {error}",
                path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Refusing to read {label} file `{}`: symlinks are not allowed",
                path.display()
            ));
        }
        if !metadata.is_file() {
            return Err(format!(
                "Refusing to read {label} file `{}`: expected a regular file",
                path.display()
            ));
        }
        if metadata.len() > MAX_OPTIONAL_TEXT_FILE_BYTES {
            return Err(format!(
                "Refusing to read {label} file `{}`: file is {} bytes, limit is {} bytes",
                path.display(),
                metadata.len(),
                MAX_OPTIONAL_TEXT_FILE_BYTES
            ));
        }
        let value = std::fs::read_to_string(path).map_err(|error| {
            format!("Failed to read {label} file `{}`: {error}", path.display())
        })?;
        return Ok(Some(value));
    }
    Ok(direct.map(ToOwned::to_owned))
}

#[cfg(test)]
mod tests {
    use super::{MAX_OPTIONAL_TEXT_FILE_BYTES, resolve_optional_text_arg};

    #[test]
    fn resolve_optional_text_arg_accepts_direct_or_file_source() {
        assert_eq!(
            resolve_optional_text_arg("notes", Some("inline"), None)
                .expect("inline text should resolve"),
            Some("inline".to_string())
        );

        let path =
            std::env::temp_dir().join(format!("taskflow-core-note-{}.txt", uuid::Uuid::now_v7()));
        std::fs::write(&path, "from file").expect("test note file should write");
        let result = resolve_optional_text_arg("notes", None, Some(&path))
            .expect("file text should resolve");
        std::fs::remove_file(&path).expect("test note file should clean up");

        assert_eq!(result, Some("from file".to_string()));
    }

    #[test]
    fn resolve_optional_text_arg_rejects_ambiguous_sources() {
        let error =
            resolve_optional_text_arg("notes", Some("inline"), Some(std::path::Path::new("x")))
                .expect_err("direct plus file should fail");

        assert!(error.contains("Use only one notes source"));
    }

    #[test]
    fn resolve_optional_text_arg_rejects_oversized_files() {
        let path = std::env::temp_dir().join(format!(
            "taskflow-core-note-big-{}.txt",
            uuid::Uuid::now_v7()
        ));
        std::fs::write(&path, "x".repeat(MAX_OPTIONAL_TEXT_FILE_BYTES as usize + 1))
            .expect("oversized note file should write");
        let error = resolve_optional_text_arg("notes", None, Some(&path))
            .expect_err("oversized file should fail");
        std::fs::remove_file(&path).expect("test note file should clean up");

        assert!(error.contains("limit is 65536 bytes"));
    }
}
