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

    #[test]
    fn resolve_optional_text_arg_returns_none_without_a_source() {
        assert_eq!(
            resolve_optional_text_arg("notes", None, None).expect("missing optional source"),
            None
        );
        assert_eq!(
            resolve_optional_text_arg("notes", Some(""), None).expect("empty direct source"),
            Some(String::new())
        );
    }

    #[test]
    fn resolve_optional_text_arg_accepts_exact_size_limit() {
        let path = std::env::temp_dir().join(format!(
            "taskflow-core-note-limit-{}.txt",
            uuid::Uuid::now_v7()
        ));
        let content = "x".repeat(MAX_OPTIONAL_TEXT_FILE_BYTES as usize);
        std::fs::write(&path, &content).expect("limit-sized note file should write");
        let result = resolve_optional_text_arg("notes", None, Some(&path))
            .expect("exact limit should be accepted");
        std::fs::remove_file(&path).expect("test note file should clean up");

        assert_eq!(result.as_deref(), Some(content.as_str()));
    }

    #[test]
    fn resolve_optional_text_arg_rejects_missing_and_non_regular_files() {
        let missing = std::env::temp_dir().join(format!(
            "taskflow-core-note-missing-{}.txt",
            uuid::Uuid::now_v7()
        ));
        let missing_error = resolve_optional_text_arg("notes", None, Some(&missing))
            .expect_err("missing note file should fail");
        assert!(missing_error.contains("Failed to inspect notes file"));

        let directory = std::env::temp_dir().join(format!(
            "taskflow-core-note-directory-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir(&directory).expect("test note directory should create");
        let directory_error = resolve_optional_text_arg("notes", None, Some(&directory))
            .expect_err("directory note source should fail");
        std::fs::remove_dir(&directory).expect("test note directory should clean up");

        assert!(directory_error.contains("expected a regular file"));
    }
}
