//! Task closure command helpers.

#[must_use]
pub fn canonical_owned_paths(paths: Vec<String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for path in paths {
        let trimmed = path.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
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
    use super::{canonical_owned_paths, task_close_commit_file_strings};

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
}
