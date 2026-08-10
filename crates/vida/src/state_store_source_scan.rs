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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_source_metadata_trims_known_fields_and_lists() {
        let metadata = parse_source_metadata(
            "# source\n\
             artifact_id: source-1\n\
             artifact_kind: instruction_source\n\
             version: 7\n\
             ownership_class: project\n\
             mutability_class: mutable\n\
             activation_class: active\n\
             required_follow_on: first, , second\n\
             hierarchy: docs, process\n\
             ignored: value\n\
             malformed line",
        );

        assert_eq!(metadata.artifact_id.as_deref(), Some("source-1"));
        assert_eq!(
            metadata.artifact_kind.as_deref(),
            Some("instruction_source")
        );
        assert_eq!(metadata.version, Some(7));
        assert_eq!(metadata.ownership_class.as_deref(), Some("project"));
        assert_eq!(metadata.mutability_class.as_deref(), Some("mutable"));
        assert_eq!(metadata.activation_class.as_deref(), Some("active"));
        assert_eq!(
            metadata.required_follow_on,
            vec!["first".to_string(), "second".to_string()]
        );
        assert_eq!(
            metadata.hierarchy,
            vec!["docs".to_string(), "process".to_string()]
        );
    }

    #[test]
    fn parse_source_metadata_rejects_invalid_versions_and_empty_lists() {
        let metadata =
            parse_source_metadata("version: not-a-number\nrequired_follow_on: , \nhierarchy:  ,\n");

        assert_eq!(metadata.version, None);
        assert!(metadata.required_follow_on.is_empty());
        assert!(metadata.hierarchy.is_empty());
    }

    #[test]
    fn source_inference_preserves_slice_and_path_contracts() {
        assert_eq!(
            infer_artifact_kind("framework_memory", Path::new("agent-definition.md")),
            "framework_memory_entry"
        );
        assert_eq!(
            infer_artifact_kind("instruction_memory", Path::new("agent-definition.md")),
            "agent_definition"
        );
        assert_eq!(
            infer_artifact_kind("project", Path::new("instruction-contract.md")),
            "instruction_contract"
        );
        assert_eq!(
            infer_artifact_kind("project", Path::new("prompt-template-config.md")),
            "prompt_template_configuration"
        );
        assert_eq!(
            infer_artifact_kind("project", Path::new("notes.md")),
            "instruction_source"
        );

        assert_eq!(infer_ownership_class("framework_memory"), "framework");
        assert_eq!(infer_ownership_class("instruction_memory"), "framework");
        assert_eq!(infer_ownership_class("project"), "project");
        assert_eq!(infer_mutability_class("instruction_memory"), "immutable");
        assert_eq!(infer_mutability_class("framework_memory"), "mutable");
        assert_eq!(infer_mutability_class("project"), "mutable");

        let relative = Path::new("docs\\process\\source.md");
        assert_eq!(artifact_id_from_path(relative), "docs-process-source");
        assert_eq!(
            record_id_for_slice_source("instruction_memory", relative),
            "instruction_memory-docs-process-source-source"
        );
        assert_eq!(
            hierarchy_from_path(Path::new("docs/process/source.md")),
            vec!["docs".to_string(), "process".to_string()]
        );
        assert!(hierarchy_from_path(Path::new("source.md")).is_empty());
        assert_eq!(normalize_path(relative), "docs/process/source.md");
    }

    #[test]
    fn collect_markdown_files_returns_sorted_markdown_only() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-store-source-scan-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("nested/deep")).unwrap();
        fs::write(root.join("b.md"), "b").unwrap();
        fs::write(root.join("nested/a.md"), "a").unwrap();
        fs::write(root.join("nested/deep/c.md"), "c").unwrap();
        fs::write(root.join("nested/ignored.txt"), "ignored").unwrap();

        let files = collect_markdown_files(&root).unwrap();
        let relative = files
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            relative,
            vec![
                "b.md".to_string(),
                "nested/a.md".to_string(),
                "nested/deep/c.md".to_string()
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn collect_markdown_files_reports_missing_root() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-store-source-scan-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let error = collect_markdown_files(&root).expect_err("missing root should fail closed");

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
