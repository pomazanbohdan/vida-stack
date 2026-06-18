use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoPathError {
    Empty,
    Absolute,
    DotSegment,
    EmptySegment,
}

impl RepoPathError {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "path is empty",
            Self::Absolute => "path must be repo-relative",
            Self::DotSegment => "path must not contain . or .. segments",
            Self::EmptySegment => "path must not contain empty segments",
        }
    }
}

#[must_use]
pub fn normalize_repo_relative_path(path: &str) -> Result<String, RepoPathError> {
    let normalized = path.trim().replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    if normalized.is_empty() {
        return Err(RepoPathError::Empty);
    }

    let path = Path::new(normalized);
    let has_absolute_component = path
        .components()
        .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir));
    let looks_like_windows_drive = normalized.as_bytes().get(1).copied() == Some(b':');
    if path.is_absolute()
        || has_absolute_component
        || looks_like_windows_drive
        || normalized.starts_with('/')
    {
        return Err(RepoPathError::Absolute);
    }

    for segment in normalized.split('/') {
        match segment {
            "" => return Err(RepoPathError::EmptySegment),
            "." | ".." => return Err(RepoPathError::DotSegment),
            _ => {}
        }
    }

    Ok(normalized.to_string())
}

#[must_use]
pub fn repo_relative_path_is_owned(changed_file: &str, owned_path: &str) -> bool {
    changed_file == owned_path || changed_file.starts_with(&format!("{owned_path}/"))
}

#[cfg(test)]
mod tests {
    use super::{RepoPathError, normalize_repo_relative_path, repo_relative_path_is_owned};

    #[test]
    fn repo_relative_path_normalization_trims_slashes_and_dot_prefix() {
        assert_eq!(
            normalize_repo_relative_path(" ./crates\\vida/src/task_surface.rs ").as_deref(),
            Ok("crates/vida/src/task_surface.rs")
        );
    }

    #[test]
    fn repo_relative_path_rejects_absolute_empty_dot_and_empty_segments() {
        assert_eq!(
            normalize_repo_relative_path("  "),
            Err(RepoPathError::Empty)
        );
        assert_eq!(
            normalize_repo_relative_path("/tmp/file"),
            Err(RepoPathError::Absolute)
        );
        assert_eq!(
            normalize_repo_relative_path("C:/tmp/file"),
            Err(RepoPathError::Absolute)
        );
        assert_eq!(
            normalize_repo_relative_path("crates/../vida"),
            Err(RepoPathError::DotSegment)
        );
        assert_eq!(
            normalize_repo_relative_path("crates//vida"),
            Err(RepoPathError::EmptySegment)
        );
    }

    #[test]
    fn repo_relative_ownership_checks_exact_path_or_child() {
        assert!(repo_relative_path_is_owned(
            "crates/vida/src/task_surface.rs",
            "crates/vida/src"
        ));
        assert!(repo_relative_path_is_owned(
            "crates/vida/src",
            "crates/vida/src"
        ));
        assert!(!repo_relative_path_is_owned(
            "crates/vida2/src/lib.rs",
            "crates/vida"
        ));
    }
}
