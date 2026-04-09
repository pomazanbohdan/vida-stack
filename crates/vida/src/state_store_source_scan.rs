use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct SourceMetadata {
    pub(crate) artifact_id: Option<String>,
    pub(crate) artifact_kind: Option<String>,
    pub(crate) version: Option<u32>,
    pub(crate) ownership_class: Option<String>,
    pub(crate) mutability_class: Option<String>,
    pub(crate) activation_class: Option<String>,
    pub(crate) required_follow_on: Vec<String>,
    pub(crate) hierarchy: Vec<String>,
}

pub(crate) fn collect_markdown_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

pub(crate) fn artifact_id_from_path(relative: &Path) -> String {
    relative
        .with_extension("")
        .to_string_lossy()
        .replace(['/', '\\'], "-")
}

pub(crate) fn parse_source_metadata(body: &str) -> SourceMetadata {
    let mut metadata = SourceMetadata::default();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "artifact_id" => metadata.artifact_id = Some(value),
            "artifact_kind" => metadata.artifact_kind = Some(value),
            "version" => metadata.version = value.parse::<u32>().ok(),
            "ownership_class" => metadata.ownership_class = Some(value),
            "mutability_class" => metadata.mutability_class = Some(value),
            "activation_class" => metadata.activation_class = Some(value),
            "required_follow_on" => {
                metadata.required_follow_on = value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            "hierarchy" => {
                metadata.hierarchy = value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            _ => {}
        }
    }
    metadata
}

pub(crate) fn infer_artifact_kind(slice: &str, relative: &Path) -> String {
    if slice == "framework_memory" {
        return "framework_memory_entry".to_string();
    }

    let normalized = relative.with_extension("").to_string_lossy().to_string();
    if normalized.ends_with("agent-definition") {
        "agent_definition".to_string()
    } else if normalized.ends_with("instruction-contract") {
        "instruction_contract".to_string()
    } else if normalized.ends_with("prompt-template-config") {
        "prompt_template_configuration".to_string()
    } else {
        "instruction_source".to_string()
    }
}

pub(crate) fn infer_ownership_class(slice: &str) -> &'static str {
    match slice {
        "framework_memory" => "framework",
        "instruction_memory" => "framework",
        _ => "project",
    }
}

pub(crate) fn infer_mutability_class(slice: &str) -> &'static str {
    match slice {
        "instruction_memory" => "immutable",
        "framework_memory" => "mutable",
        _ => "mutable",
    }
}

pub(crate) fn record_id_for_slice_source(slice: &str, relative: &Path) -> String {
    format!("{}-{}-source", slice, artifact_id_from_path(relative))
}

pub(crate) fn hierarchy_from_path(relative: &Path) -> Vec<String> {
    relative
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
