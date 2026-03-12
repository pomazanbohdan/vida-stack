use docflow_contracts::RegistryRow;
use docflow_core::ArtifactPath;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct InventoryScope {
    pub root: PathBuf,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
}

impl InventoryScope {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            include_globs: vec!["**/*.md".to_string()],
            exclude_globs: vec![],
        }
    }
}

pub fn build_registry(scope: &InventoryScope) -> Result<Vec<RegistryRow>, globset::Error> {
    let includes = compile_globset(&scope.include_globs)?;
    let excludes = compile_globset(&scope.exclude_globs)?;

    let mut rows = Vec::new();
    for entry in WalkDir::new(&scope.root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = match entry.path().strip_prefix(&scope.root) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if !matches_scope(relative, &includes, &excludes) {
            continue;
        }
        rows.push(RegistryRow {
            artifact_path: ArtifactPath(path_to_string(relative)),
            artifact_type: artifact_type_for(relative),
        });
    }

    rows.sort_by(|left, right| left.artifact_path.0.cmp(&right.artifact_path.0));
    Ok(rows)
}

fn compile_globset(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    builder.build()
}

fn matches_scope(path: &Path, includes: &GlobSet, excludes: &GlobSet) -> bool {
    includes.is_match(path) && !excludes.is_match(path)
}

fn artifact_type_for(path: &Path) -> String {
    let text = path_to_string(path);
    if text.starts_with("docs/product/spec/") {
        "product_spec".to_string()
    } else if text.starts_with("docs/process/") {
        "process_doc".to_string()
    } else if text.starts_with("vida/config/instructions/") {
        "instruction_contract".to_string()
    } else {
        "document".to_string()
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{InventoryScope, build_registry};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should work")
            .as_nanos();
        std::env::temp_dir().join(format!("docflow-inventory-{nanos}"))
    }

    #[test]
    fn builds_sorted_registry_rows_from_markdown_tree() {
        let root = temp_root();
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir");
        fs::create_dir_all(root.join("docs/process")).expect("process dir");
        fs::write(root.join("docs/product/spec/a.md"), "# a\n").expect("write a");
        fs::write(root.join("docs/process/b.md"), "# b\n").expect("write b");
        fs::write(root.join("notes.txt"), "ignore\n").expect("write txt");

        let scope = InventoryScope::new(&root);
        let rows = build_registry(&scope).expect("registry should build");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].artifact_path.0, "docs/process/b.md");
        assert_eq!(rows[0].artifact_type, "process_doc");
        assert_eq!(rows[1].artifact_path.0, "docs/product/spec/a.md");
        assert_eq!(rows[1].artifact_type, "product_spec");

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn exclude_globs_remove_matching_files() {
        let root = temp_root();
        fs::create_dir_all(root.join("docs/process")).expect("process dir");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("write a");
        fs::write(root.join("docs/process/b.md"), "# b\n").expect("write b");

        let mut scope = InventoryScope::new(&root);
        scope.exclude_globs = vec!["**/b.md".to_string()];
        let rows = build_registry(&scope).expect("registry should build");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].artifact_path.0, "docs/process/a.md");

        fs::remove_dir_all(root).expect("temp root should be removed");
    }
}
