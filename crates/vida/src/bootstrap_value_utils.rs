use std::path::{Path, PathBuf};

pub(crate) fn trimmed_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn slugify_project_id(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

pub(crate) fn inferred_project_title(project_id: &str, explicit_name: Option<&str>) -> String {
    if let Some(name) = trimmed_non_empty(explicit_name) {
        return name;
    }
    project_id.to_string()
}

pub(crate) fn is_missing_or_placeholder(value: Option<&str>, placeholder: &str) -> bool {
    match value.map(str::trim) {
        None => true,
        Some("") => true,
        Some(current) if current == placeholder => true,
        _ => false,
    }
}

pub(crate) fn config_file_path() -> Result<PathBuf, String> {
    Ok(config_file_path_for_root(
        &crate::resolve_runtime_project_root()?,
    ))
}

pub(crate) fn config_file_path_for_root(root: &Path) -> PathBuf {
    root.join("vida.config.yaml")
}

pub(crate) fn normalize_root_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_helpers_cover_empty_explicit_and_placeholder_boundaries() {
        assert_eq!(trimmed_non_empty(None), None);
        assert_eq!(
            trimmed_non_empty(Some("  Vida  ")),
            Some("Vida".to_string())
        );
        assert_eq!(trimmed_non_empty(Some("   ")), None);

        assert_eq!(
            slugify_project_id(" Hello, VIDA_stack! "),
            "hello-vida-stack"
        );
        assert_eq!(slugify_project_id("---"), "");
        assert_eq!(
            inferred_project_title("project-id", Some("  Project Name  ")),
            "Project Name"
        );
        assert_eq!(inferred_project_title("project-id", None), "project-id");

        assert!(is_missing_or_placeholder(None, "placeholder"));
        assert!(is_missing_or_placeholder(
            Some(" placeholder "),
            "placeholder"
        ));
        assert!(is_missing_or_placeholder(Some(""), "placeholder"));
        assert!(!is_missing_or_placeholder(Some("other"), "placeholder"));
        assert!(!is_missing_or_placeholder(Some("real"), "placeholder"));
    }

    #[test]
    fn path_helpers_keep_root_and_config_file_contracts() {
        let root = Path::new("project-root");
        assert_eq!(
            config_file_path_for_root(root),
            root.join("vida.config.yaml")
        );
        assert_eq!(normalize_root_arg(root), root.to_string_lossy());
    }
}
