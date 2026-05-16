use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::{DocsArgs, DocsCommand, DocsUpdateArgs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopedDocWriteStatus {
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScopedDocWrite {
    pub(crate) path: String,
    pub(crate) status: ScopedDocWriteStatus,
}

pub(crate) async fn run_docs(args: DocsArgs) -> ExitCode {
    match args.command {
        DocsCommand::Update(args) => run_docs_update(args),
    }
}

fn run_docs_update(args: DocsUpdateArgs) -> ExitCode {
    let project_root = match super::resolve_runtime_project_root() {
        Ok(path) => path,
        Err(error) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "blocked",
                        "blocker": "project_root_not_found",
                        "error": error,
                    })
                );
            } else {
                eprintln!("{error}");
            }
            return ExitCode::from(1);
        }
    };
    let source_root = super::resolve_init_bootstrap_source_root();

    match update_current_docs_at_root(&project_root, &source_root) {
        Ok(writes) => {
            if args.json {
                print_docs_update_json(&project_root, &writes);
            } else {
                print_docs_update_text(&project_root, &writes);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "blocked",
                        "blocker": "scoped_docs_update_failed",
                        "project_root": project_root.display().to_string(),
                        "error": error,
                    })
                );
            } else {
                eprintln!("{error}");
            }
            ExitCode::from(1)
        }
    }
}

pub(crate) fn update_current_docs_at_root(
    project_root: &Path,
    bootstrap_source_root: &Path,
) -> Result<Vec<ScopedDocWrite>, String> {
    let agents_source = super::init_surfaces::resolve_init_agents_source(bootstrap_source_root)?;
    let agents_contents = std::fs::read_to_string(&agents_source)
        .map_err(|error| format!("Failed to read {}: {error}", agents_source.display()))?;

    let mut writes = vec![write_scoped_doc(
        project_root,
        Path::new("AGENTS.md"),
        &agents_contents,
    )?];

    let protocol_source_root = resolve_protocol_docs_source_root(bootstrap_source_root)?;
    let protocol_target_root = project_root.join("vida/config/instructions");
    collect_protocol_doc_writes(
        &protocol_source_root,
        &protocol_target_root,
        project_root,
        &mut writes,
    )?;
    Ok(writes)
}

fn resolve_protocol_docs_source_root(bootstrap_source_root: &Path) -> Result<PathBuf, String> {
    let canonical_instruction_root = bootstrap_source_root.join("vida/config/instructions");
    if protocol_doc_count(&canonical_instruction_root)? > 0 {
        return Ok(canonical_instruction_root);
    }

    let legacy_bundle_root =
        bootstrap_source_root.join(super::state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    if protocol_doc_count(&legacy_bundle_root)? > 0 {
        return Ok(legacy_bundle_root);
    }

    Err(format!(
        "No instruction protocol docs found under {} or {}",
        canonical_instruction_root.display(),
        legacy_bundle_root.display()
    ))
}

fn protocol_doc_count(root: &Path) -> Result<usize, String> {
    if !root.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(root)
        .map_err(|error| format!("Failed to read {}: {error}", root.display()))?
    {
        let entry =
            entry.map_err(|error| format!("Failed to iterate {}: {error}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            count += protocol_doc_count(&path)?;
        } else if file_type.is_file() && is_protocol_doc(&path) {
            count += 1;
        }
    }
    Ok(count)
}

fn collect_protocol_doc_writes(
    source_root: &Path,
    target_root: &Path,
    project_root: &Path,
    writes: &mut Vec<ScopedDocWrite>,
) -> Result<(), String> {
    if !source_root.is_dir() {
        return Err(format!(
            "Instruction protocol source root is missing: {}",
            source_root.display()
        ));
    }
    for entry in std::fs::read_dir(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to iterate {}: {error}", source_root.display()))?;
        let source_path = entry.path();
        let target_path = target_root.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect {}: {error}", source_path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_protocol_doc_writes(&source_path, &target_path, project_root, writes)?;
        } else if file_type.is_file() && is_protocol_doc(&source_path) {
            let contents = std::fs::read_to_string(&source_path)
                .map_err(|error| format!("Failed to read {}: {error}", source_path.display()))?;
            let relative_target = target_path.strip_prefix(project_root).map_err(|_| {
                format!(
                    "Instruction protocol target escaped project root: {}",
                    target_path.display()
                )
            })?;
            writes.push(write_scoped_doc(project_root, relative_target, &contents)?);
        }
    }
    Ok(())
}

fn is_protocol_doc(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("-protocol.md"))
}

fn write_scoped_doc(
    project_root: &Path,
    relative_path: &Path,
    contents: &str,
) -> Result<ScopedDocWrite, String> {
    let target = scoped_target_path(project_root, relative_path)?;
    reject_symlink_target(&target)?;
    reject_symlink_ancestors(project_root, &target)?;
    if let Some(parent) = target.parent() {
        super::ensure_dir(parent)?;
    }
    let status = match std::fs::read_to_string(&target) {
        Ok(existing) if existing == contents => ScopedDocWriteStatus::Unchanged,
        Ok(_) | Err(_) => {
            std::fs::write(&target, contents)
                .map_err(|error| format!("Failed to write {}: {error}", target.display()))?;
            ScopedDocWriteStatus::Updated
        }
    };
    Ok(ScopedDocWrite {
        path: relative_path_to_slash(relative_path),
        status,
    })
}

fn relative_path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn scoped_target_path(project_root: &Path, relative_path: &Path) -> Result<PathBuf, String> {
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "Scoped docs path is not project-relative: {}",
            relative_path.display()
        ));
    }
    Ok(project_root.join(relative_path))
}

fn reject_symlink_ancestors(project_root: &Path, target: &Path) -> Result<(), String> {
    let parent = target.parent().ok_or_else(|| {
        format!(
            "Scoped docs target has no parent directory: {}",
            target.display()
        )
    })?;
    let relative_parent = parent.strip_prefix(project_root).map_err(|_| {
        format!(
            "Scoped docs target escaped project root while validating parents: {}",
            parent.display()
        )
    })?;

    let mut cursor = project_root.to_path_buf();
    for component in relative_parent.components() {
        cursor.push(component.as_os_str());
        if let Ok(metadata) = std::fs::symlink_metadata(&cursor)
            && metadata.file_type().is_symlink()
        {
            return Err(format!(
                "Refusing to update scoped docs through symlinked parent: {}",
                cursor.display()
            ));
        }
    }
    Ok(())
}

fn reject_symlink_target(target: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Refusing to update scoped docs symlink target: {}",
            target.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect {}: {error}", target.display())),
    }
}

fn print_docs_update_json(project_root: &Path, writes: &[ScopedDocWrite]) {
    let updated: Vec<&str> = writes
        .iter()
        .filter(|write| write.status == ScopedDocWriteStatus::Updated)
        .map(|write| write.path.as_str())
        .collect();
    let unchanged: Vec<&str> = writes
        .iter()
        .filter(|write| write.status == ScopedDocWriteStatus::Unchanged)
        .map(|write| write.path.as_str())
        .collect();
    println!(
        "{}",
        serde_json::json!({
            "status": "ok",
            "scope": [
                "AGENTS.md",
                "vida/config/instructions/**/*-protocol.md"
            ],
            "excluded": [
                "AGENTS.sidecar.md",
                "vida.config.yaml",
                "README.md",
                "docs/**",
                "vida/config/instructions/**/*-contract.md",
                "vida/config/instructions/**/*-capsule.md",
                "vida/config/framework-memory/**",
                ".vida/**"
            ],
            "project_root": project_root.display().to_string(),
            "updated_paths": updated,
            "unchanged_paths": unchanged,
        })
    );
}

fn print_docs_update_text(project_root: &Path, writes: &[ScopedDocWrite]) {
    println!("vida docs update complete");
    println!("project root: {}", project_root.display());
    for write in writes {
        let status = match write.status {
            ScopedDocWriteStatus::Updated => "updated",
            ScopedDocWriteStatus::Unchanged => "unchanged",
        };
        println!("{status}: {}", write.path);
    }
    println!("excluded: AGENTS.sidecar.md, vida.config.yaml, README.md, docs/**, non-protocol instruction files, vida/config/framework-memory/**, .vida/**");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn docs_update_only_touches_agents_and_instruction_protocol_docs() {
        let harness = crate::temp_state::TempStateHarness::new().expect("temp root should init");
        let project_root = harness.path().join("project");
        let source_root = harness.path().join("source");
        let source_instructions = source_root.join("vida/config/instructions");
        let project_instructions = project_root.join("vida/config/instructions");
        fs::create_dir_all(source_root.join("install/assets")).expect("source assets should exist");
        fs::create_dir_all(source_instructions.join("instruction-contracts"))
            .expect("source protocols should exist");
        fs::create_dir_all(project_instructions.join("instruction-contracts"))
            .expect("project protocols should exist");
        fs::create_dir_all(project_root.join("docs/product")).expect("product docs should exist");

        fs::write(
            source_root.join("install/assets/AGENTS.scaffold.md"),
            "# canonical agents\n",
        )
        .expect("agents source should write");
        fs::write(
            source_instructions.join("instruction-contracts/core.demo-protocol.md"),
            "# canonical protocol\n",
        )
        .expect("protocol source should write");
        fs::write(
            source_instructions.join("instruction-contracts/role.demo-contract.md"),
            "# canonical contract\n",
        )
        .expect("contract source should write");

        fs::write(project_root.join("AGENTS.md"), "# stale agents\n")
            .expect("agents target should write");
        fs::write(project_root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("sidecar should write");
        fs::write(
            project_root.join("vida.config.yaml"),
            "project:\n  id: demo\n",
        )
        .expect("config should write");
        fs::write(project_root.join("README.md"), "# readme\n").expect("readme should write");
        fs::write(project_root.join("docs/product/index.md"), "# product\n")
            .expect("product doc should write");
        fs::write(
            project_instructions.join("instruction-contracts/core.demo-protocol.md"),
            "# stale protocol\n",
        )
        .expect("protocol target should write");
        fs::write(
            project_instructions.join("instruction-contracts/role.demo-contract.md"),
            "# stale contract\n",
        )
        .expect("contract target should write");

        let writes = update_current_docs_at_root(&project_root, &source_root)
            .expect("docs update should succeed");

        assert!(writes.iter().any(
            |write| write.path == "AGENTS.md" && write.status == ScopedDocWriteStatus::Updated
        ));
        let protocol_write_path =
            "vida/config/instructions/instruction-contracts/core.demo-protocol.md";
        assert!(
            writes.iter().any(|write| write.path == protocol_write_path
                && write.status == ScopedDocWriteStatus::Updated),
            "writes: {writes:?}"
        );
        assert!(!writes
            .iter()
            .any(|write| write.path.ends_with("role.demo-contract.md")));
        assert_eq!(
            fs::read_to_string(project_root.join("AGENTS.md")).expect("agents should read"),
            "# canonical agents\n"
        );
        assert_eq!(
            fs::read_to_string(
                project_instructions.join("instruction-contracts/core.demo-protocol.md")
            )
            .expect("protocol should read"),
            "# canonical protocol\n"
        );
        assert_eq!(
            fs::read_to_string(
                project_instructions.join("instruction-contracts/role.demo-contract.md")
            )
            .expect("contract should read"),
            "# stale contract\n"
        );
        assert_eq!(
            fs::read_to_string(project_root.join("AGENTS.sidecar.md"))
                .expect("sidecar should read"),
            "# sidecar\n"
        );
        assert_eq!(
            fs::read_to_string(project_root.join("vida.config.yaml")).expect("config should read"),
            "project:\n  id: demo\n"
        );
        assert_eq!(
            fs::read_to_string(project_root.join("README.md")).expect("readme should read"),
            "# readme\n"
        );
        assert_eq!(
            fs::read_to_string(project_root.join("docs/product/index.md"))
                .expect("product doc should read"),
            "# product\n"
        );
    }

    #[test]
    fn docs_update_rejects_symlinked_instruction_parent_directory() {
        let harness = crate::temp_state::TempStateHarness::new().expect("temp root should init");
        let project_root = harness.path().join("project");
        let source_root = harness.path().join("source");
        let source_instructions = source_root.join("vida/config/instructions");
        let outside_root = harness.path().join("outside");

        fs::create_dir_all(source_root.join("install/assets")).expect("source assets should exist");
        fs::create_dir_all(source_instructions.join("instruction-contracts"))
            .expect("source protocols should exist");
        fs::create_dir_all(&outside_root).expect("outside root should exist");

        fs::write(
            source_root.join("install/assets/AGENTS.scaffold.md"),
            "# canonical agents\n",
        )
        .expect("agents source should write");
        fs::write(
            source_instructions.join("instruction-contracts/core.demo-protocol.md"),
            "# canonical protocol\n",
        )
        .expect("protocol source should write");

        fs::create_dir_all(project_root.join("vida/config")).expect("project config should exist");
        std::os::unix::fs::symlink(&outside_root, project_root.join("vida/config/instructions"))
            .expect("symlink should write");

        let error = update_current_docs_at_root(&project_root, &source_root)
            .expect_err("docs update should reject symlinked parent");

        assert!(
            error.contains("symlinked parent"),
            "expected symlinked parent error, got: {error}"
        );
        assert!(
            !outside_root
                .join("instruction-contracts/core.demo-protocol.md")
                .exists(),
            "protocol write should not escape project root"
        );
    }

    #[test]
    fn docs_command_does_not_request_runtime_state_dir() {
        let cli = <crate::Cli as clap::Parser>::try_parse_from(["vida", "docs", "update"])
            .expect("docs command should parse");
        assert!(!crate::root_command_router::command_needs_project_root_state_dir(&cli.command));
    }
}
