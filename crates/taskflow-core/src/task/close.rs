//! Task closure command helpers.

#[must_use]
pub fn canonical_owned_paths(paths: Vec<String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for path in paths {
        let trimmed = path.trim().trim_end_matches('/').to_string();
        if !is_safe_literal_repo_path(&trimmed) {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
}

fn is_safe_literal_repo_path(path: &str) -> bool {
    if path.starts_with('\\') || path.starts_with(':') || path.contains(['*', '?', '[']) {
        return false;
    }

    for component in path.split('/') {
        if component.is_empty() || matches!(component, "." | "..") {
            return false;
        }
    }

    true
}

#[must_use]
pub fn task_close_commit_file_strings(
    commit_files: Vec<String>,
    stage_owned: bool,
    task_owned_paths: Option<Vec<String>>,
) -> Vec<String> {
    if !commit_files.is_empty() {
        return canonical_owned_paths(commit_files);
    }

    if stage_owned {
        if let Some(task_owned_paths) = task_owned_paths {
            return canonical_owned_paths(task_owned_paths);
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::{canonical_owned_paths, is_safe_literal_repo_path, task_close_commit_file_strings};

    #[test]
    fn canonical_owned_paths_trims_dedupes_and_drops_empty_values() {
        let paths = canonical_owned_paths(vec![
            " crates/vida/src/ ".to_string(),
            "crates/vida/src".to_string(),
            "crates/taskflow-core/src/task//".to_string(),
            " ".to_string(),
        ]);

        assert_eq!(
            paths,
            vec![
                "crates/vida/src".to_string(),
                "crates/taskflow-core/src/task".to_string()
            ]
        );
    }

    #[test]
    fn canonical_owned_paths_drops_broad_or_pathspec_like_values() {
        let paths = canonical_owned_paths(vec![
            ".".to_string(),
            "..".to_string(),
            "src/../secret.txt".to_string(),
            "/tmp/secret.txt".to_string(),
            ":(glob)*".to_string(),
            "src/*.rs".to_string(),
            "src/[a].rs".to_string(),
            "src/file?.rs".to_string(),
            "src/main.rs".to_string(),
        ]);

        assert_eq!(paths, vec!["src/main.rs".to_string()]);
    }

    #[test]
    fn safe_literal_repo_path_rejects_empty_components_and_magic() {
        assert!(is_safe_literal_repo_path("crates/vida/src/task_surface.rs"));
        assert!(is_safe_literal_repo_path("docs/process"));
        assert!(!is_safe_literal_repo_path(""));
        assert!(!is_safe_literal_repo_path("."));
        assert!(!is_safe_literal_repo_path("src//main.rs"));
        assert!(!is_safe_literal_repo_path(":(literal)src/main.rs"));
        assert!(!is_safe_literal_repo_path(r"\server\share\file.txt"));
        assert!(!is_safe_literal_repo_path("src/*.rs"));
    }

    #[test]
    fn task_close_commit_file_strings_prefers_explicit_files_over_stage_owned() {
        let files = task_close_commit_file_strings(
            vec![" crates/vida/src/task_surface.rs ".to_string()],
            true,
            Some(vec!["crates/taskflow-core/src/task/".to_string()]),
        );

        assert_eq!(files, vec!["crates/vida/src/task_surface.rs".to_string()]);
    }

    #[test]
    fn task_close_commit_file_strings_uses_owned_paths_for_stage_owned() {
        let files = task_close_commit_file_strings(
            Vec::new(),
            true,
            Some(vec![
                "crates/vida/src/".to_string(),
                "crates/vida/src".to_string(),
            ]),
        );

        assert_eq!(files, vec!["crates/vida/src".to_string()]);
    }

    #[test]
    fn task_close_commit_file_strings_blocks_empty_unowned_commit_scope() {
        let files = task_close_commit_file_strings(Vec::new(), false, None);

        assert!(files.is_empty());
    }

    #[test]
    fn task_close_commit_file_strings_ignores_owned_paths_without_stage_permission() {
        let files = task_close_commit_file_strings(
            Vec::new(),
            false,
            Some(vec!["crates/vida/src/main.rs".to_string()]),
        );

        assert!(files.is_empty());
    }

    #[test]
    fn task_close_commit_file_strings_does_not_fallback_when_explicit_scope_is_invalid() {
        let files = task_close_commit_file_strings(
            vec![r"\outside\file.txt".to_string()],
            true,
            Some(vec!["crates/vida/src/main.rs".to_string()]),
        );

        assert!(files.is_empty());
    }
}
