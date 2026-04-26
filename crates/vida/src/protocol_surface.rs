use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Clone)]
pub(crate) struct ProtocolViewTarget {
    pub(crate) canonical_id: &'static str,
    pub(crate) source_path: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) aliases: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedProtocolViewTarget {
    pub(crate) canonical_id: String,
    pub(crate) source_path: String,
    pub(crate) kind: String,
    pub(crate) aliases: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct ProtocolViewRender {
    pub(crate) requested_name: String,
    pub(crate) resolved_id: String,
    pub(crate) resolved_path: String,
    pub(crate) resolved_kind: String,
    pub(crate) requested_fragment: Option<String>,
    pub(crate) aliases: Vec<String>,
    pub(crate) content: String,
}

pub(crate) async fn run_protocol(args: super::ProtocolArgs) -> ExitCode {
    match args.command {
        super::ProtocolCommand::View(view) => {
            let mut renders = Vec::with_capacity(view.names.len());
            for name in &view.names {
                let render = match render_protocol_view_target(name) {
                    Ok(render) => render,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                renders.push(render);
            }

            if view.json {
                if renders.len() == 1 {
                    let render = renders.pop().expect("single protocol render should exist");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida protocol view",
                            "requested_name": render.requested_name,
                            "resolved_id": render.resolved_id,
                            "resolved_path": render.resolved_path,
                            "resolved_kind": render.resolved_kind,
                            "requested_fragment": render.requested_fragment,
                            "aliases": render.aliases,
                            "content": render.content,
                        }))
                        .expect("protocol view json should render")
                    );
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida protocol view",
                            "requested_names": view.names,
                            "targets": renders,
                        }))
                        .expect("multi-target protocol view json should render")
                    );
                }
            } else {
                let multi_target = renders.len() > 1;
                for (index, render) in renders.iter().enumerate() {
                    if multi_target {
                        if index > 0 {
                            println!();
                        }
                        println!("===== {} =====", render.resolved_id);
                    } else if index > 0 {
                        println!();
                    }
                    print!("{}", render.content);
                    if !render.content.ends_with('\n') {
                        println!();
                    }
                }
            }
            ExitCode::SUCCESS
        }
    }
}

fn split_protocol_view_fragment(name: &str) -> (&str, Option<&str>) {
    match name.trim().split_once('#') {
        Some((base, fragment)) => (base.trim(), Some(fragment.trim())),
        None => (name.trim(), None),
    }
}

fn slugify_markdown_heading(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if (ch.is_ascii_whitespace() || ch == '-' || ch == '_') && !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

pub(crate) fn extract_protocol_view_fragment(
    content: &str,
    fragment: &str,
) -> Result<String, String> {
    let normalized_fragment = fragment.trim();
    if normalized_fragment.is_empty() {
        return Ok(content.to_string());
    }

    let requested_section = normalized_fragment
        .strip_prefix("section-")
        .unwrap_or(normalized_fragment);
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = None;
    let mut end_idx = lines.len();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(section_name) = trimmed.strip_prefix("## Section:") {
            let section_name = section_name.trim();
            if section_name == requested_section {
                start_idx = Some(idx);
                for (next_idx, next_line) in lines.iter().enumerate().skip(idx + 1) {
                    if next_line.trim().starts_with("## Section:") {
                        end_idx = next_idx;
                        break;
                    }
                }
                break;
            }
        }
        if trimmed.starts_with('#') {
            let heading = trimmed.trim_start_matches('#').trim();
            if slugify_markdown_heading(heading) == normalized_fragment {
                start_idx = Some(idx);
                let current_level = trimmed.chars().take_while(|ch| *ch == '#').count();
                for (next_idx, next_line) in lines.iter().enumerate().skip(idx + 1) {
                    let candidate = next_line.trim();
                    if candidate.starts_with('#') {
                        let next_level = candidate.chars().take_while(|ch| *ch == '#').count();
                        if next_level <= current_level {
                            end_idx = next_idx;
                            break;
                        }
                    }
                }
                break;
            }
        }
    }

    let Some(start_idx) = start_idx else {
        return Err(format!(
            "Unknown protocol view fragment `#{normalized_fragment}`."
        ));
    };
    Ok(lines[start_idx..end_idx].join("\n"))
}

fn protocol_view_targets() -> &'static [ProtocolViewTarget] {
    &[
        ProtocolViewTarget {
            canonical_id: "bootstrap/router",
            source_path: "vida/config/instructions/system-maps/bootstrap.router-guide.md",
            kind: "bootstrap_router_guide",
            aliases: &[
                "AGENTS",
                "AGENTS.md",
                "bootstrap-router",
                "bootstrap/router",
                "system-maps/bootstrap.router-guide",
                "system-maps/bootstrap.router-guide.md",
            ],
        },
        ProtocolViewTarget {
            canonical_id: "agent-definitions/entry.orchestrator-entry",
            source_path: "vida/config/instructions/agent-definitions/entry.orchestrator-entry.md",
            kind: "agent_definition",
            aliases: &[
                "agent-definitions/entry.orchestrator-entry",
                "agent-definitions/entry.orchestrator-entry.md",
            ],
        },
        ProtocolViewTarget {
            canonical_id: "agent-definitions/entry.worker-entry",
            source_path: "vida/config/instructions/agent-definitions/entry.worker-entry.md",
            kind: "agent_definition",
            aliases: &[
                "agent-definitions/entry.worker-entry",
                "agent-definitions/entry.worker-entry.md",
            ],
        },
        ProtocolViewTarget {
            canonical_id: "instruction-contracts/role.worker-thinking",
            source_path: "vida/config/instructions/instruction-contracts/role.worker-thinking.md",
            kind: "instruction_contract",
            aliases: &[
                "instruction-contracts/role.worker-thinking",
                "instruction-contracts/role.worker-thinking.md",
            ],
        },
        ProtocolViewTarget {
            canonical_id: "system-maps/bootstrap.worker-boot-flow",
            source_path: "vida/config/instructions/system-maps/bootstrap.worker-boot-flow.md",
            kind: "system_map",
            aliases: &[
                "system-maps/bootstrap.worker-boot-flow",
                "system-maps/bootstrap.worker-boot-flow.md",
            ],
        },
        ProtocolViewTarget {
            canonical_id: "system-maps/bootstrap.orchestrator-boot-flow",
            source_path: "vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md",
            kind: "system_map",
            aliases: &[
                "system-maps/bootstrap.orchestrator-boot-flow",
                "system-maps/bootstrap.orchestrator-boot-flow.md",
            ],
        },
    ]
}

fn resolve_protocol_view_source_root() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(installed_root) = super::init_surfaces::resolve_installed_runtime_root() {
        candidates.push(installed_root.join("current"));
        candidates.push(installed_root);
    }
    candidates.push(super::repo_runtime_root());
    if let Ok(root) = super::resolve_repo_root() {
        if !candidates.contains(&root) {
            candidates.push(root);
        }
    }

    candidates
        .into_iter()
        .find(|root| {
            root.join("AGENTS.md").is_file()
                && root
                    .join("vida/config/instructions/system-maps/protocol.index.md")
                    .is_file()
                && root
                    .join("vida/config/instructions/system-maps/bootstrap.router-guide.md")
                    .is_file()
        })
        .ok_or_else(|| {
            "Unable to resolve protocol-view source root with AGENTS.md and instruction maps"
                .to_string()
        })
}

fn infer_protocol_view_kind(canonical_id: &str) -> &'static str {
    match canonical_id.split('/').next().unwrap_or_default() {
        "agent-definitions" => "agent_definition",
        "instruction-contracts" => "instruction_contract",
        "prompt-templates" => "prompt_template",
        "runtime-instructions" => "runtime_instruction",
        "command-instructions" => "command_instruction",
        "diagnostic-instructions" => "diagnostic_instruction",
        "system-maps" => "system_map",
        "agent-backends" => "agent_backend",
        "references" => "reference",
        _ => "instruction_artifact",
    }
}

pub(crate) fn resolve_protocol_view_target(
    name: &str,
) -> Result<(ResolvedProtocolViewTarget, PathBuf), String> {
    let (normalized, _) = split_protocol_view_fragment(name);
    if normalized.is_empty() {
        return Err("Protocol view target name must not be empty.".to_string());
    }

    if let Some(target) = protocol_view_targets().iter().find(|target| {
        target.canonical_id == normalized
            || target.source_path == normalized
            || target.aliases.contains(&normalized)
    }) {
        let source_root = resolve_protocol_view_source_root()?;
        let resolved = ResolvedProtocolViewTarget {
            canonical_id: target.canonical_id.to_string(),
            source_path: target.source_path.to_string(),
            kind: target.kind.to_string(),
            aliases: target
                .aliases
                .iter()
                .map(|alias| (*alias).to_string())
                .collect(),
        };
        return Ok((resolved, source_root.join(target.source_path)));
    }

    let source_root = resolve_protocol_view_source_root()?;
    let relative = normalized
        .strip_prefix("vida/config/instructions/")
        .unwrap_or(normalized);
    let canonical_id = relative.strip_suffix(".md").unwrap_or(relative);
    if canonical_id.is_empty() || !canonical_id.contains('/') {
        return Err(format!("Unknown protocol view target `{normalized}`."));
    }
    let source_path = format!("vida/config/instructions/{canonical_id}.md");
    let resolved_path = source_root.join(&source_path);
    if !resolved_path.is_file() {
        return Err(format!("Unknown protocol view target `{normalized}`."));
    }
    let resolved = ResolvedProtocolViewTarget {
        canonical_id: canonical_id.to_string(),
        source_path,
        kind: infer_protocol_view_kind(canonical_id).to_string(),
        aliases: vec![
            canonical_id.to_string(),
            format!("{canonical_id}.md"),
            format!("vida/config/instructions/{canonical_id}.md"),
        ],
    };
    Ok((resolved, resolved_path))
}

pub(crate) fn render_protocol_view_target(name: &str) -> Result<ProtocolViewRender, String> {
    let (_, fragment) = split_protocol_view_fragment(name);
    let (target, path) = resolve_protocol_view_target(name)?;
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let rendered_content = match fragment {
        Some(fragment) => extract_protocol_view_fragment(&content, fragment)?,
        None => content,
    };

    Ok(ProtocolViewRender {
        requested_name: name.to_string(),
        resolved_id: target.canonical_id,
        resolved_path: target.source_path,
        resolved_kind: target.kind,
        requested_fragment: fragment.map(str::to_string),
        aliases: target.aliases,
        content: rendered_content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir};

    #[test]
    fn protocol_view_command_accepts_json_output() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(
            runtime.block_on(crate::run(cli(&["protocol", "view", "AGENTS", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_bootstrap_aliases() {
        let (target, path) =
            resolve_protocol_view_target("AGENTS").expect("AGENTS alias should resolve");
        assert_eq!(target.canonical_id, "bootstrap/router");
        assert!(
            path.ends_with("vida/config/instructions/system-maps/bootstrap.router-guide.md"),
            "bootstrap router guide path should resolve"
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_worker_entry_name() {
        let (target, path) = resolve_protocol_view_target("agent-definitions/entry.worker-entry")
            .expect("worker entry should resolve");
        assert_eq!(target.canonical_id, "agent-definitions/entry.worker-entry");
        assert!(
            path.ends_with("vida/config/instructions/agent-definitions/entry.worker-entry.md"),
            "worker entry path should resolve"
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_generic_canonical_ids_without_md() {
        let (target, path) =
            resolve_protocol_view_target("instruction-contracts/core.orchestration-protocol")
                .expect("generic canonical id should resolve");
        assert_eq!(
            target.canonical_id,
            "instruction-contracts/core.orchestration-protocol"
        );
        assert_eq!(target.kind, "instruction_contract");
        assert!(
            path.ends_with(
                "vida/config/instructions/instruction-contracts/core.orchestration-protocol.md"
            ),
            "generic protocol path should resolve"
        );
    }

    #[test]
    fn resolve_protocol_view_target_ignores_fragment_for_path_resolution() {
        let (target, path) = resolve_protocol_view_target(
            "instruction-contracts/overlay.step-thinking-protocol#section-web-search",
        )
        .expect("fragment target should resolve");
        assert_eq!(
            target.canonical_id,
            "instruction-contracts/overlay.step-thinking-protocol"
        );
        assert!(
            path.ends_with(
                "vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md"
            ),
            "fragment target path should resolve"
        );
    }

    #[test]
    fn extract_protocol_view_fragment_supports_section_markers() {
        let content = "intro\n## Section: web-search\n# Web Validation Integration\nbody\n## Section: other\nnext";
        let section = extract_protocol_view_fragment(content, "section-web-search")
            .expect("section marker should resolve");
        assert!(
            section.contains("Web Validation Integration"),
            "section content should include heading"
        );
        assert!(
            !section.contains("## Section: other"),
            "section content should stop at next marker"
        );
    }
}
