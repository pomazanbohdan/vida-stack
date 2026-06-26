use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::state_store;

pub(crate) fn copy_tree_replace(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "Expected directory source for tree materialization: {}",
            source_root.display()
        ));
    }
    let source_canonical = source_root
        .canonicalize()
        .map_err(|error| format!("Failed to canonicalize {}: {error}", source_root.display()))?;
    let target_canonical = match target_root.canonicalize() {
        Ok(path) => Some(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Failed to canonicalize {}: {error}",
                target_root.display()
            ));
        }
    };
    if target_canonical.as_ref() == Some(&source_canonical) {
        return Ok(());
    }
    if target_root.exists() {
        remove_tree_for_replacement(target_root)
            .map_err(|error| format!("Failed to replace {}: {error}", target_root.display()))?;
    }
    copy_tree_recursive(source_root, target_root)
}

fn remove_tree_for_replacement(target_root: &Path) -> Result<(), String> {
    match fs::remove_dir_all(target_root) {
        Ok(()) => Ok(()),
        Err(first_error) if !target_root.exists() => {
            let _ = first_error;
            Ok(())
        }
        Err(first_error) => {
            make_writable_for_replacement_root(target_root).map_err(|retry_error| {
                format!(
                    "{first_error}; retry after clearing replacement target failed: {retry_error}"
                )
            })?;
            fs::remove_dir_all(target_root).map_err(|retry_error| {
                format!(
                    "{first_error}; retry after clearing replacement target failed: {retry_error}"
                )
            })
        }
    }
}

fn make_writable_for_replacement_root(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn copy_tree_recursive(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if metadata.is_dir() {
        std::fs::create_dir_all(target_root)
            .map_err(|error| format!("Failed to create {}: {error}", target_root.display()))?;
        for entry in std::fs::read_dir(source_root)
            .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?
        {
            let entry = entry
                .map_err(|error| format!("Failed to iterate {}: {error}", source_root.display()))?;
            copy_tree_recursive(&entry.path(), &target_root.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = target_root.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source_root, target_root).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source_root.display(),
            target_root.display()
        )
    })?;
    Ok(())
}

pub(crate) fn default_init_instruction_bundle_source_roots(
    bootstrap_source_root: &Path,
) -> (PathBuf, PathBuf) {
    (
        bootstrap_source_root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT),
        bootstrap_source_root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT),
    )
}

pub(crate) fn materialize_framework_instruction_bundles(
    project_root: &Path,
    instruction_source_root: &Path,
    framework_memory_source_root: &Path,
) -> Result<(), String> {
    copy_tree_replace(
        instruction_source_root,
        &project_root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT),
    )?;
    copy_tree_replace(
        framework_memory_source_root,
        &project_root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vida-materialization-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn copy_tree_replace_removes_stale_nested_target_contents() {
        let root = unique_temp_root("stale-target");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(source.join("config")).expect("source config");
        fs::write(source.join("config").join("fresh.yaml"), "fresh").expect("fresh source");
        fs::create_dir_all(target.join("stale")).expect("stale target");
        fs::write(target.join("stale").join("old.yaml"), "old").expect("stale file");

        copy_tree_replace(&source, &target).expect("replace tree");

        assert!(target.join("config").join("fresh.yaml").is_file());
        assert!(!target.join("stale").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn copy_tree_replace_noops_when_source_and_target_are_same_directory() {
        let root = unique_temp_root("self-target");
        let source = root.join("source");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("framework.yaml"), "keep").expect("source file");

        copy_tree_replace(&source, &source).expect("same source and target should be a no-op");

        assert_eq!(
            fs::read_to_string(source.join("framework.yaml")).expect("source file remains"),
            "keep"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn copy_tree_replace_clears_readonly_target_files_before_retry_copy() {
        let root = unique_temp_root("readonly-target");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("fresh.yaml"), "fresh").expect("fresh source");
        fs::create_dir_all(&target).expect("target");
        let stale_file = target.join("old.yaml");
        fs::write(&stale_file, "old").expect("stale file");
        let mut permissions = fs::metadata(&stale_file)
            .expect("stale metadata")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&stale_file, permissions).expect("readonly stale file");

        copy_tree_replace(&source, &target).expect("replace tree with readonly target");

        assert_eq!(
            fs::read_to_string(target.join("fresh.yaml")).expect("fresh target"),
            "fresh"
        );
        assert!(!target.join("old.yaml").exists());
        let _ = fs::remove_dir_all(root);
    }
}
