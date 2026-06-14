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
    if target_root.exists() {
        std::fs::remove_dir_all(target_root)
            .map_err(|error| format!("Failed to replace {}: {error}", target_root.display()))?;
    }
    copy_tree_recursive(source_root, target_root)
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
