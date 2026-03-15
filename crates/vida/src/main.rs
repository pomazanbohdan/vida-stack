mod state_store;
mod taskflow_layer4;
mod taskflow_run_graph;
mod taskflow_runtime_bundle;
mod temp_state;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

use clap::{Args, CommandFactory, Parser, Subcommand};
use docflow_cli::{
    CheckArgs as DocflowCheckArgs, Cli as DocflowCli, Command as DocflowCommand,
    ProofcheckArgs as DocflowProofcheckArgs, RegistryScanArgs,
};
use state_store::{
    BlockedTaskRecord, LauncherActivationSnapshot, ProtocolBindingState, StateStore,
    StateStoreError, TaskCriticalPath, TaskDependencyRecord, TaskDependencyStatus,
    TaskDependencyTreeEdge, TaskDependencyTreeNode, TaskGraphIssue, TaskRecord,
};
use taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use taskflow_runtime_bundle::{
    blocking_runtime_bundle, build_taskflow_consume_bundle_payload, taskflow_consume_bundle_check,
};
use time::format_description::well_known::Rfc3339;

const TASKFLOW_PROTOCOL_BINDING_SCENARIO: &str = "v0.2.2-taskflow-wave1-primary";
const TASKFLOW_PROTOCOL_BINDING_AUTHORITY: &str = "taskflow_state_store";
const DEFAULT_AGENT_EXTENSION_ROLES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/roles.yaml");
const DEFAULT_AGENT_EXTENSION_SKILLS_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/skills.yaml");
const DEFAULT_AGENT_EXTENSION_PROFILES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/profiles.yaml");
const DEFAULT_AGENT_EXTENSION_FLOWS_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/flows.yaml");
const DEFAULT_RUNTIME_AGENT_EXTENSIONS_README: &str = r#"# Runtime Agent Extensions

This directory holds the active runtime-owned agent-extension projections for the project.

Runtime rule:

1. `.vida/project/agent-extensions/*.yaml` is the active project-local runtime projection family.
2. Matching `*.sidecar.yaml` files are the editable override surfaces for project-local changes.
3. Root `docs/process/agent-extensions/**` remains source/export/import lineage only; it is not the live runtime source.
4. Edited sidecars become active only through runtime validation and import-safe execution paths.
"#;
const DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML: &str = "version: 1\nroles: []\n";
const DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML: &str = "version: 1\nskills: []\n";
const DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML: &str = "version: 1\nprofiles: []\n";
const DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML: &str = "version: 1\nflow_sets: []\n";
const PROJECT_ID_PLACEHOLDER: &str = "__PROJECT_ID__";
const DOCS_ROOT_PLACEHOLDER: &str = "__DOCS_ROOT__";
const PROCESS_ROOT_PLACEHOLDER: &str = "__PROCESS_ROOT__";
const RESEARCH_ROOT_PLACEHOLDER: &str = "__RESEARCH_ROOT__";
const README_DOC_PLACEHOLDER: &str = "__README_DOC__";
const ARCHITECTURE_DOC_PLACEHOLDER: &str = "__ARCHITECTURE_DOC__";
const DECISIONS_DOC_PLACEHOLDER: &str = "__DECISIONS_DOC__";
const ENVIRONMENTS_DOC_PLACEHOLDER: &str = "__ENVIRONMENTS_DOC__";
const PROJECT_OPERATIONS_DOC_PLACEHOLDER: &str = "__PROJECT_OPERATIONS_DOC__";
const AGENT_SYSTEM_DOC_PLACEHOLDER: &str = "__AGENT_SYSTEM_DOC__";
const USER_COMMUNICATION_PLACEHOLDER: &str = "__USER_COMMUNICATION__";
const REASONING_LANGUAGE_PLACEHOLDER: &str = "__REASONING_LANGUAGE__";
const DOCUMENTATION_LANGUAGE_PLACEHOLDER: &str = "__DOCUMENTATION_LANGUAGE__";
const TODO_PROTOCOL_LANGUAGE_PLACEHOLDER: &str = "__TODO_PROTOCOL_LANGUAGE__";
const DEFAULT_PROJECT_DOCS_ROOT: &str = "docs";
const DEFAULT_PROJECT_PROCESS_ROOT: &str = "docs/process";
const DEFAULT_PROJECT_RESEARCH_ROOT: &str = "docs/research";
const DEFAULT_PROJECT_ROOT_MAP: &str = "docs/project-root-map.md";
const DEFAULT_PROJECT_PRODUCT_INDEX: &str = "docs/product/index.md";
const DEFAULT_PROJECT_PRODUCT_SPEC_README: &str = "docs/product/spec/README.md";
const DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE: &str =
    "docs/product/spec/templates/feature-design-document.template.md";
const DEFAULT_PROJECT_ARCHITECTURE_DOC: &str = "docs/product/architecture.md";
const DEFAULT_PROJECT_PROCESS_README: &str = "docs/process/README.md";
const DEFAULT_PROJECT_DECISIONS_DOC: &str = "docs/process/decisions.md";
const DEFAULT_PROJECT_ENVIRONMENTS_DOC: &str = "docs/process/environments.md";
const DEFAULT_PROJECT_OPERATIONS_DOC: &str = "docs/process/project-operations.md";
const DEFAULT_PROJECT_AGENT_SYSTEM_DOC: &str = "docs/process/agent-system.md";
const DEFAULT_PROJECT_CODEX_GUIDE_DOC: &str = "docs/process/codex-agent-configuration-guide.md";
const DEFAULT_PROJECT_DOC_TOOLING_DOC: &str = "docs/process/documentation-tooling-map.md";
const DEFAULT_PROJECT_RESEARCH_README: &str = "docs/research/README.md";
const PROJECT_ACTIVATION_RECEIPT_LATEST: &str = ".vida/receipts/project-activation.latest.json";
const SPEC_BOOTSTRAP_RECEIPT_LATEST: &str = ".vida/receipts/spec-bootstrap.latest.json";
const CODEX_WORKER_SCORECARDS_STATE: &str = ".vida/state/worker-scorecards.json";
const CODEX_WORKER_STRATEGY_STATE: &str = ".vida/state/worker-strategy.json";
const HOST_AGENT_OBSERVABILITY_STATE: &str = ".vida/state/host-agent-observability.json";

#[tokio::main]
async fn main() -> ExitCode {
    run(Cli::parse()).await
}

async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        None => {
            print_root_help();
            ExitCode::SUCCESS
        }
        Some(Command::Init(args)) => run_init(args).await,
        Some(Command::Boot(args)) => run_boot(args).await,
        Some(Command::OrchestratorInit(args)) => run_orchestrator_init(args).await,
        Some(Command::AgentInit(args)) => run_agent_init(args).await,
        Some(Command::Protocol(args)) => run_protocol(args).await,
        Some(Command::ProjectActivator(args)) => run_project_activator(args).await,
        Some(Command::AgentFeedback(args)) => run_agent_feedback(args).await,
        Some(Command::Task(args)) => run_task(args).await,
        Some(Command::Memory(args)) => run_memory(args).await,
        Some(Command::Status(args)) => run_status(args).await,
        Some(Command::Doctor(args)) => run_doctor(args).await,
        Some(Command::Taskflow(args)) => run_taskflow_proxy(args).await,
        Some(Command::Docflow(args)) => run_docflow_proxy(args),
        Some(Command::External(args)) => run_unknown(&args),
    }
}

fn run_unknown(args: &[String]) -> ExitCode {
    let command = args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unknown command family `{command}`. Use `vida --help` to inspect the frozen root surface."
    );
    ExitCode::from(2)
}

fn proxy_requested_help(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("help") | Some("--help") | Some("-h")
    )
}

fn print_docflow_proxy_help() {
    println!("VIDA DocFlow runtime family");
    println!();
    println!("Mode-scoped launcher contract:");
    println!("  repo/dev binary mode: vida routes the active DocFlow command map in-process through the Rust CLI.");
    println!("  installed mode: vida keeps the same in-process Rust DocFlow shell.");
    println!(
        "  unsupported commands fail closed instead of silently falling through to donor wrappers."
    );
    println!();
    println!("Implemented in-process command surface:");
    let mut command = DocflowCli::command();
    let help = command.render_long_help().to_string();
    print!("{help}");
    if !help.ends_with('\n') {
        println!();
    }
}

fn looks_like_project_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file()
        && path.join("vida.config.yaml").is_file()
        && path.join(".vida/config").is_dir()
        && path.join(".vida/db").is_dir()
        && path.join(".vida/project").is_dir()
}

fn resolve_source_repo_root_from_current_dir(current_dir: &Path) -> Option<PathBuf> {
    let repo_root = repo_runtime_root();
    if current_dir.starts_with(&repo_root) && looks_like_init_bootstrap_source_root(&repo_root) {
        return Some(repo_root);
    }
    None
}

fn resolve_env_repo_root() -> Result<Option<PathBuf>, String> {
    let Some(root) = std::env::var_os("VIDA_ROOT") else {
        return Ok(None);
    };
    let root = PathBuf::from(root);
    if !root.exists() {
        return Err(format!(
            "VIDA_ROOT points to a missing path: {}",
            root.display()
        ));
    }
    if looks_like_project_root(&root) || looks_like_init_bootstrap_source_root(&root) {
        return Ok(Some(root));
    }
    Err(format!(
        "VIDA_ROOT points to a path that is not a VIDA runtime or source root: {}",
        root.display()
    ))
}

fn resolve_repo_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    let mut candidates = current_dir
        .ancestors()
        .filter(|path| looks_like_project_root(path))
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => {
            if let Some(root) = resolve_source_repo_root_from_current_dir(&current_dir) {
                return Ok(root);
            }
            if let Some(root) = resolve_env_repo_root()? {
                return Ok(root);
            }
            Err(format!(
                "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
                current_dir.display()
            ))
        }
        _ => Err(format!(
            "Ambiguous VIDA project root from {}: {}. Set VIDA_ROOT explicitly.",
            current_dir.display(),
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn resolve_runtime_project_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    let mut candidates = current_dir
        .ancestors()
        .filter(|path| looks_like_project_root(path))
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(format!(
            "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
            current_dir.display()
        ))
        .or_else(|_| {
            if let Some(root) = resolve_source_repo_root_from_current_dir(&current_dir) {
                return Ok(root);
            }
            if let Some(root) = resolve_env_repo_root()? {
                return Ok(root);
            }
            Err(format!(
                "Unable to resolve VIDA project root from {}. Run inside a project tree or set VIDA_ROOT explicitly.",
                current_dir.display()
            ))
        }),
        _ => Err(format!(
            "Ambiguous VIDA project root from {}: {}. Set VIDA_ROOT explicitly.",
            current_dir.display(),
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

#[derive(Clone)]
struct ProtocolViewTarget {
    canonical_id: &'static str,
    source_path: &'static str,
    kind: &'static str,
    aliases: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedProtocolViewTarget {
    canonical_id: String,
    source_path: String,
    kind: String,
    aliases: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct ProtocolViewRender {
    requested_name: String,
    resolved_id: String,
    resolved_path: String,
    resolved_kind: String,
    requested_fragment: Option<String>,
    aliases: Vec<String>,
    content: String,
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

fn extract_protocol_view_fragment(content: &str, fragment: &str) -> Result<String, String> {
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
    if let Some(installed_root) = resolve_installed_runtime_root() {
        candidates.push(installed_root.join("current"));
        candidates.push(installed_root);
    }
    candidates.push(repo_runtime_root());
    if let Ok(root) = resolve_repo_root() {
        if !candidates.iter().any(|candidate| candidate == &root) {
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

fn resolve_protocol_view_target(
    name: &str,
) -> Result<(ResolvedProtocolViewTarget, PathBuf), String> {
    let (normalized, _) = split_protocol_view_fragment(name);
    if normalized.is_empty() {
        return Err("Protocol view target name must not be empty.".to_string());
    }

    if let Some(target) = protocol_view_targets().iter().find(|target| {
        target.canonical_id == normalized
            || target.source_path == normalized
            || target.aliases.iter().any(|alias| *alias == normalized)
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

fn render_protocol_view_target(name: &str) -> Result<ProtocolViewRender, String> {
    let (_, fragment) = split_protocol_view_fragment(name);
    let (target, path) = resolve_protocol_view_target(name)?;
    let content = fs::read_to_string(&path)
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

fn copy_tree_if_missing(source_root: &Path, target_root: &Path) -> Result<(), String> {
    if target_root.exists() {
        return Ok(());
    }
    copy_tree_recursive(source_root, target_root)
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("Failed to create {}: {error}", path.display()))
}

fn ensure_runtime_home(project_root: &Path) -> Result<(), String> {
    for relative in [
        ".vida/config",
        ".vida/db",
        ".vida/cache",
        ".vida/framework",
        ".vida/project",
        ".vida/project/agent-extensions",
        ".vida/receipts",
        ".vida/runtime",
        ".vida/scratchpad",
    ] {
        ensure_dir(&project_root.join(relative))?;
    }
    Ok(())
}

fn copy_tree_recursive(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if metadata.is_dir() {
        fs::create_dir_all(target_root)
            .map_err(|error| format!("Failed to create {}: {error}", target_root.display()))?;
        for entry in fs::read_dir(source_root)
            .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?
        {
            let entry = entry
                .map_err(|error| format!("Failed to iterate {}: {error}", source_root.display()))?;
            let source_path = entry.path();
            let target_path = target_root.join(entry.file_name());
            copy_tree_recursive(&source_path, &target_path)?;
        }
        return Ok(());
    }

    if let Some(parent) = target_root.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(source_root, target_root).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source_root.display(),
            target_root.display()
        )
    })?;
    Ok(())
}

fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    taskflow_binary_candidates_for_root(root)
        .into_iter()
        .next()
        .map(|_| root.to_path_buf())
}

fn looks_like_init_bootstrap_source_root(root: &Path) -> bool {
    resolve_init_agents_source(root).is_ok()
        && resolve_init_sidecar_source(root).is_ok()
        && resolve_init_config_template_source(root).is_ok()
        && root.join(".codex").is_dir()
}

fn resolve_init_bootstrap_source_root() -> PathBuf {
    if let Some(installed_root) = resolve_installed_runtime_root() {
        if looks_like_init_bootstrap_source_root(&installed_root) {
            return installed_root;
        }
    }
    repo_runtime_root()
}

fn resolve_init_agents_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.scaffold.md"),
        root.join("AGENTS.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve generated AGENTS scaffold. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn resolve_init_sidecar_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.sidecar.scaffold.md"),
        root.join("AGENTS.sidecar.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve project sidecar scaffold. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn resolve_init_config_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/vida.config.yaml.template"),
        root.join("docs/framework/templates/vida.config.yaml.template"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve vida.config.yaml template. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn taskflow_binary_candidates_for_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let bin_dir = root.join("bin");
    if let Ok(entries) = fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_binary = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_file() && is_taskflow_binary {
                candidates.push(path);
            }
        }
    }

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_runtime_dir = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_dir() && is_taskflow_runtime_dir {
                let candidate = path.join("src/vida");
                if candidate.exists() {
                    candidates.push(candidate);
                }
            }
        }
    }

    candidates
}

const SUPPORTED_HOST_CLI_SYSTEMS: &[&str] = &["codex"];
const HOST_CLI_PLACEHOLDER: &str = "__HOST_CLI_SYSTEM__";

fn normalize_host_cli_system(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    SUPPORTED_HOST_CLI_SYSTEMS
        .iter()
        .copied()
        .find(|candidate| *candidate == normalized)
}

fn trimmed_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn slugify_project_id(value: &str) -> String {
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

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn infer_feature_request_slug(request: &str) -> String {
    const STOPWORDS: &[&str] = &[
        "a",
        "an",
        "and",
        "build",
        "code",
        "containing",
        "create",
        "detailed",
        "develop",
        "file",
        "follow",
        "for",
        "full",
        "game",
        "html",
        "implementation",
        "implement",
        "mechanics",
        "page",
        "plan",
        "please",
        "research",
        "single",
        "specifications",
        "steps",
        "the",
        "these",
        "write",
    ];
    let filtered = request
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !STOPWORDS.iter().any(|stop| stop == token))
        .take(6)
        .collect::<Vec<_>>()
        .join("-");
    let slug = slugify_project_id(if filtered.is_empty() {
        request
    } else {
        &filtered
    });
    let trimmed = slug.trim_matches('-');
    let bounded = if trimmed.len() > 48 {
        &trimmed[..48]
    } else {
        trimmed
    };
    bounded.trim_matches('-').to_string()
}

fn infer_feature_request_title(request: &str) -> String {
    let trimmed = request.trim();
    let compact = trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if compact.is_empty() {
        "Feature request".to_string()
    } else if compact.chars().count() <= 72 {
        compact
    } else {
        let shortened = compact.chars().take(69).collect::<String>();
        format!("{shortened}...")
    }
}

fn inferred_project_id_candidate(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify_project_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "vida-project".to_string())
}

fn inferred_project_title(project_id: &str, explicit_name: Option<&str>) -> String {
    if let Some(name) = trimmed_non_empty(explicit_name) {
        return name;
    }
    project_id.to_string()
}

fn is_missing_or_placeholder(value: Option<&str>, placeholder: &str) -> bool {
    match value.map(str::trim) {
        None => true,
        Some("") => true,
        Some(current) if current == placeholder => true,
        _ => false,
    }
}

fn set_yaml_scalar_in_top_level_section(
    contents: &str,
    section: &str,
    key: &str,
    value: &str,
) -> String {
    let rendered = yaml_scalar(value);
    let mut lines: Vec<String> = contents.lines().map(ToString::to_string).collect();
    let section_header = format!("{section}:");
    let key_prefix = format!("{key}:");
    let mut section_index = None;
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == section_header {
            section_index = Some(index);
            break;
        }
    }

    if let Some(section_index) = section_index {
        let mut section_end = lines.len();
        for index in (section_index + 1)..lines.len() {
            let trimmed = lines[index].trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !lines[index].starts_with(' ') {
                section_end = index;
                break;
            }
        }
        for index in (section_index + 1)..section_end {
            if lines[index].trim_start().starts_with(&key_prefix) && lines[index].starts_with("  ")
            {
                lines[index] = format!("  {key}: {rendered}");
                return format!("{}\n", lines.join("\n"));
            }
        }
        lines.insert(section_end, format!("  {key}: {rendered}"));
        return format!("{}\n", lines.join("\n"));
    }

    if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.push(section_header);
    lines.push(format!("  {key}: {rendered}"));
    format!("{}\n", lines.join("\n"))
}

fn set_yaml_bool_in_top_level_section(
    contents: &str,
    section: &str,
    key: &str,
    value: bool,
) -> String {
    set_yaml_scalar_in_top_level_section(
        contents,
        section,
        key,
        if value { "true" } else { "false" },
    )
}

fn write_file_if_missing_or_placeholder(target: &Path, contents: &str) -> Result<bool, String> {
    if target.exists() && !file_contains_placeholder(target) {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))?;
    Ok(true)
}

fn list_host_cli_agent_templates(root: &Path) -> Vec<String> {
    let agents_dir = root.join("agents");
    let Ok(entries) = fs::read_dir(agents_dir) else {
        return Vec::new();
    };
    let mut names = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn escape_toml_basic_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rendered_codex_agent_catalog(
    agent_catalog: &[serde_json::Value],
    _named_lane_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    agent_catalog.to_vec()
}

fn render_codex_config_toml(
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> String {
    let template_config = read_simple_toml_sections(&template_root.join("config.toml"));
    let max_threads = template_config
        .get("agents")
        .and_then(|section| section.get("max_threads"))
        .cloned()
        .unwrap_or_else(|| "4".to_string());
    let max_depth = template_config
        .get("agents")
        .and_then(|section| section.get("max_depth"))
        .cloned()
        .unwrap_or_else(|| "2".to_string());
    let mut lines = vec![
        "[features]".to_string(),
        "multi_agent = true".to_string(),
        String::new(),
        "[agents]".to_string(),
        format!("max_threads = {max_threads}"),
        format!("max_depth = {max_depth}"),
        String::new(),
    ];
    for row in rendered_codex_agent_catalog(agent_catalog, named_lane_catalog) {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let description = row["description"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(escape_toml_basic_string)
            .unwrap_or_else(|| {
                escape_toml_basic_string(&format!(
                    "Rendered Codex executor lane for VIDA tier `{}`. Rate: {}.",
                    row["tier"].as_str().unwrap_or(role_id),
                    row["rate"].as_u64().unwrap_or(0)
                ))
            });
        lines.push(format!("[agents.{role_id}]"));
        lines.push(format!("description = \"{description}\""));
        lines.push(format!("config_file = \"agents/{role_id}.toml\""));
        lines.push(String::new());
    }
    format!("{}\n", lines.join("\n"))
}

fn set_toml_scalar_line(contents: &str, key: &str, rendered_value: &str) -> String {
    let replacement = format!("{key} = {rendered_value}");
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in contents.lines() {
        if line.trim_start().starts_with(&format!("{key} =")) && !replaced {
            lines.push(replacement.clone());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(replacement);
    }
    format!("{}\n", lines.join("\n"))
}

fn extract_toml_multiline_string(contents: &str, key: &str) -> Option<String> {
    let marker = format!("{key} = \"\"\"");
    let mut lines = contents.lines();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with(&marker) {
            continue;
        }
        let mut body = Vec::new();
        for next_line in &mut lines {
            if next_line.trim() == "\"\"\"" {
                return Some(body.join("\n"));
            }
            body.push(next_line.to_string());
        }
        return Some(body.join("\n"));
    }
    None
}

fn set_toml_multiline_string(contents: &str, key: &str, body: &str) -> String {
    let marker = format!("{key} = \"\"\"");
    let mut lines = Vec::new();
    let mut replaced = false;
    let mut source = contents.lines();

    while let Some(line) = source.next() {
        if line.trim_start().starts_with(&marker) && !replaced {
            lines.push(marker.clone());
            lines.extend(body.lines().map(ToString::to_string));
            lines.push("\"\"\"".to_string());
            replaced = true;
            for next_line in &mut source {
                if next_line.trim() == "\"\"\"" {
                    break;
                }
            }
            continue;
        }
        lines.push(line.to_string());
    }

    if !replaced {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(marker);
        lines.extend(body.lines().map(ToString::to_string));
        lines.push("\"\"\"".to_string());
    }

    format!("{}\n", lines.join("\n"))
}

fn compose_codex_lane_developer_instructions(
    base_instructions: Option<&str>,
    lane_override: Option<&str>,
) -> Option<String> {
    match (
        base_instructions.map(str::trim).filter(|value| !value.is_empty()),
        lane_override.map(str::trim).filter(|value| !value.is_empty()),
    ) {
        (Some(base), Some(overlay)) => Some(format!(
            "{base}\n\nLane activation overlay:\n{overlay}\n\nFollow both layers: keep the carrier-tier posture and boundaries, then apply the lane-specific mission as the active role for this packet."
        )),
        (Some(base), None) => Some(base.to_string()),
        (None, Some(overlay)) => Some(overlay.to_string()),
        (None, None) => None,
    }
}

fn render_codex_agent_toml(
    row: &serde_json::Value,
    template_contents: Option<&str>,
) -> Option<String> {
    let role_id = row["role_id"].as_str()?;
    let model = row["model"].as_str().unwrap_or("gpt-5.4");
    let reasoning_effort = row["model_reasoning_effort"].as_str().unwrap_or("medium");
    let sandbox_mode = row["sandbox_mode"].as_str().unwrap_or("workspace-write");
    let tier = row["tier"].as_str().unwrap_or(role_id);
    let rate = row["rate"].as_u64().unwrap_or(0);
    let reasoning_band = row["reasoning_band"].as_str().unwrap_or_default();
    let default_runtime_role = row["default_runtime_role"].as_str().unwrap_or_default();
    let runtime_roles = row["runtime_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(",");
    let task_classes = row["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(",");
    let developer_instructions_override = row["developer_instructions"]
        .as_str()
        .filter(|value| !value.trim().is_empty());
    if let Some(template) = template_contents.filter(|value| !value.trim().is_empty()) {
        let patched = set_toml_scalar_line(template, "model", &format!("\"{model}\""));
        let patched = set_toml_scalar_line(
            &patched,
            "model_reasoning_effort",
            &format!("\"{reasoning_effort}\""),
        );
        let patched =
            set_toml_scalar_line(&patched, "sandbox_mode", &format!("\"{sandbox_mode}\""));
        let patched = set_toml_scalar_line(&patched, "vida_tier", &format!("\"{tier}\""));
        let patched = set_toml_scalar_line(&patched, "vida_rate", &format!("\"{rate}\""));
        let patched = set_toml_scalar_line(
            &patched,
            "vida_reasoning_band",
            &format!("\"{reasoning_band}\""),
        );
        let patched = set_toml_scalar_line(
            &patched,
            "vida_default_runtime_role",
            &format!("\"{default_runtime_role}\""),
        );
        let patched = set_toml_scalar_line(
            &patched,
            "vida_runtime_roles",
            &format!("\"{runtime_roles}\""),
        );
        let patched = set_toml_scalar_line(
            &patched,
            "vida_task_classes",
            &format!("\"{task_classes}\""),
        );
        let patched = if let Some(instructions) = compose_codex_lane_developer_instructions(
            extract_toml_multiline_string(template, "developer_instructions").as_deref(),
            developer_instructions_override,
        ) {
            set_toml_multiline_string(&patched, "developer_instructions", &instructions)
        } else {
            patched
        };
        return Some(patched);
    }

    if let Some(instructions) =
        compose_codex_lane_developer_instructions(None, developer_instructions_override)
    {
        return Some(format!(
            "model = \"{model}\"\nmodel_reasoning_effort = \"{reasoning_effort}\"\nsandbox_mode = \"{sandbox_mode}\"\nvida_tier = \"{tier}\"\nvida_rate = \"{rate}\"\nvida_reasoning_band = \"{reasoning_band}\"\nvida_default_runtime_role = \"{default_runtime_role}\"\nvida_runtime_roles = \"{runtime_roles}\"\nvida_task_classes = \"{task_classes}\"\ndeveloper_instructions = \"\"\"\n{instructions}\n\"\"\"\n"
        ));
    }

    Some(format!(
        "model = \"{model}\"\nmodel_reasoning_effort = \"{reasoning_effort}\"\nsandbox_mode = \"{sandbox_mode}\"\nvida_tier = \"{tier}\"\nvida_rate = \"{rate}\"\nvida_reasoning_band = \"{reasoning_band}\"\nvida_default_runtime_role = \"{default_runtime_role}\"\nvida_runtime_roles = \"{runtime_roles}\"\nvida_task_classes = \"{task_classes}\"\n"
    ))
}

fn render_codex_template_from_catalog(
    project_root: &Path,
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> Result<(), String> {
    let codex_root = project_root.join(".codex");
    let agents_root = codex_root.join("agents");
    fs::create_dir_all(&agents_root)
        .map_err(|error| format!("Failed to create {}: {error}", agents_root.display()))?;

    let template_agents = read_codex_agent_catalog(template_root)
        .into_iter()
        .filter_map(|row| {
            Some((
                row["role_id"].as_str()?.to_string(),
                row["config_file"].as_str()?.to_string(),
            ))
        })
        .collect::<HashMap<_, _>>();

    for entry in fs::read_dir(&agents_root)
        .map_err(|error| format!("Failed to read {}: {error}", agents_root.display()))?
    {
        let path = entry
            .map_err(|error| format!("Failed to inspect {}: {error}", agents_root.display()))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            fs::remove_file(&path)
                .map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
        }
    }

    let config_body = render_codex_config_toml(template_root, agent_catalog, named_lane_catalog);
    fs::write(codex_root.join("config.toml"), config_body).map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            codex_root.join("config.toml").display()
        )
    })?;

    for row in rendered_codex_agent_catalog(agent_catalog, named_lane_catalog) {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let template_role_id = row["template_role_id"].as_str().unwrap_or(role_id);
        let template_contents = template_agents
            .get(template_role_id)
            .and_then(|config_file| fs::read_to_string(template_root.join(config_file)).ok());
        let Some(body) = render_codex_agent_toml(&row, template_contents.as_deref()) else {
            continue;
        };
        fs::write(agents_root.join(format!("{role_id}.toml")), body).map_err(|error| {
            format!(
                "Failed to write {}: {error}",
                agents_root.join(format!("{role_id}.toml")).display()
            )
        })?;
    }

    Ok(())
}

fn csv_string_list(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn read_codex_agent_catalog(codex_root: &Path) -> Vec<serde_json::Value> {
    let codex_config = read_simple_toml_sections(&codex_root.join("config.toml"));
    let mut roles = codex_config
        .iter()
        .filter_map(|(section, values)| {
            let role_id = section.strip_prefix("agents.")?;
            if role_id.is_empty() || role_id == "development" {
                return None;
            }
            let config_file = values.get("config_file").cloned().unwrap_or_default();
            let role_config = if config_file.is_empty() {
                HashMap::new()
            } else {
                read_simple_toml_sections(&codex_root.join(&config_file))
                    .remove("")
                    .unwrap_or_default()
            };
            let tier = role_config
                .get("vida_tier")
                .cloned()
                .unwrap_or_else(|| role_id.to_string());
            Some(serde_json::json!({
                "role_id": role_id,
                "description": values.get("description").cloned().unwrap_or_default(),
                "config_file": config_file,
                "model": role_config.get("model").cloned().unwrap_or_default(),
                "model_reasoning_effort": role_config.get("model_reasoning_effort").cloned().unwrap_or_default(),
                "sandbox_mode": role_config.get("sandbox_mode").cloned().unwrap_or_default(),
                "tier": tier,
                "rate": role_config
                    .get("vida_rate")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(0),
                "reasoning_band": role_config
                    .get("vida_reasoning_band")
                    .cloned()
                    .unwrap_or_else(|| role_config.get("model_reasoning_effort").cloned().unwrap_or_default()),
                "default_runtime_role": role_config.get("vida_default_runtime_role").cloned().unwrap_or_default(),
                "runtime_roles": csv_string_list(role_config.get("vida_runtime_roles")),
                "task_classes": csv_string_list(role_config.get("vida_task_classes")),
            }))
        })
        .collect::<Vec<_>>();
    roles.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["role_id"].as_str().unwrap_or_default())
            })
    });
    roles
}

fn codex_worker_scorecards_state_path(project_root: &Path) -> PathBuf {
    project_root.join(CODEX_WORKER_SCORECARDS_STATE)
}

fn codex_worker_strategy_state_path(project_root: &Path) -> PathBuf {
    project_root.join(CODEX_WORKER_STRATEGY_STATE)
}

fn host_agent_observability_state_path(project_root: &Path) -> PathBuf {
    project_root.join(HOST_AGENT_OBSERVABILITY_STATE)
}

fn read_json_file_if_present(path: &Path) -> Option<serde_json::Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn load_or_initialize_host_agent_observability_state(project_root: &Path) -> serde_json::Value {
    read_json_file_if_present(&host_agent_observability_state_path(project_root)).unwrap_or_else(
        || {
            serde_json::json!({
                "schema_version": 1,
                "updated_at": time::OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .expect("rfc3339 timestamp should render"),
                "store_path": HOST_AGENT_OBSERVABILITY_STATE,
                "budget": {
                    "total_estimated_units": 0,
                    "by_agent_id": {},
                    "by_task_class": {},
                    "event_count": 0,
                    "latest_event_at": serde_json::Value::Null,
                },
                "events": []
            })
        },
    )
}

#[derive(Debug, Clone)]
struct HostAgentFeedbackInput<'a> {
    agent_id: &'a str,
    score: u64,
    outcome: &'a str,
    task_class: &'a str,
    notes: Option<&'a str>,
    source: &'a str,
    task_id: Option<&'a str>,
    task_display_id: Option<&'a str>,
    task_title: Option<&'a str>,
    runtime_role: Option<&'a str>,
    selected_tier: Option<&'a str>,
    estimated_task_price_units: Option<u64>,
    lifecycle_state: Option<&'a str>,
    effective_score: Option<u64>,
    reason: Option<&'a str>,
}

fn increment_object_counter(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    delta: u64,
) {
    let current = object
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    object.insert(
        key.to_string(),
        serde_json::Value::Number(serde_json::Number::from(current + delta)),
    );
}

fn append_host_agent_observability_event(
    project_root: &Path,
    input: &HostAgentFeedbackInput<'_>,
) -> Result<serde_json::Value, String> {
    let mut ledger = load_or_initialize_host_agent_observability_state(project_root);
    if !ledger["events"].is_array() {
        ledger["events"] = serde_json::json!([]);
    }
    if !ledger["budget"].is_object() {
        ledger["budget"] = serde_json::json!({
            "total_estimated_units": 0,
            "by_agent_id": {},
            "by_task_class": {},
            "event_count": 0,
            "latest_event_at": serde_json::Value::Null,
        });
    }

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let estimated_units = input.estimated_task_price_units.unwrap_or(0);
    let event = serde_json::json!({
        "recorded_at": recorded_at,
        "event_kind": "feedback",
        "source": input.source,
        "agent_id": input.agent_id,
        "selected_tier": input.selected_tier.unwrap_or(input.agent_id),
        "runtime_role": input.runtime_role.unwrap_or(""),
        "task_id": input.task_id.unwrap_or(""),
        "task_display_id": input.task_display_id.unwrap_or(""),
        "task_title": input.task_title.unwrap_or(""),
        "task_class": input.task_class,
        "outcome": input.outcome,
        "score": input.score,
        "notes": input.notes.unwrap_or(""),
        "reason": input.reason.unwrap_or(""),
        "estimated_task_price_units": estimated_units,
        "effective_score": input.effective_score,
        "lifecycle_state": input.lifecycle_state.unwrap_or(""),
    });
    let event_count = {
        let events = ledger["events"]
            .as_array_mut()
            .expect("events array should initialize");
        events.push(event.clone());
        events.len() as u64
    };

    let budget = ledger["budget"]
        .as_object_mut()
        .expect("budget object should initialize");
    let total_estimated_units = budget
        .get("total_estimated_units")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    budget.insert(
        "total_estimated_units".to_string(),
        serde_json::Value::Number(serde_json::Number::from(
            total_estimated_units + estimated_units,
        )),
    );
    if !budget
        .get("by_agent_id")
        .and_then(serde_json::Value::as_object)
        .is_some()
    {
        budget.insert("by_agent_id".to_string(), serde_json::json!({}));
    }
    if !budget
        .get("by_task_class")
        .and_then(serde_json::Value::as_object)
        .is_some()
    {
        budget.insert("by_task_class".to_string(), serde_json::json!({}));
    }
    if let Some(by_agent_id) = budget
        .get_mut("by_agent_id")
        .and_then(serde_json::Value::as_object_mut)
    {
        increment_object_counter(by_agent_id, input.agent_id, estimated_units);
    }
    if let Some(by_task_class) = budget
        .get_mut("by_task_class")
        .and_then(serde_json::Value::as_object_mut)
    {
        increment_object_counter(by_task_class, input.task_class, estimated_units);
    }
    budget.insert(
        "event_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(event_count)),
    );
    budget.insert(
        "latest_event_at".to_string(),
        serde_json::Value::String(recorded_at.clone()),
    );
    ledger["updated_at"] = serde_json::Value::String(recorded_at.clone());

    let ledger_path = host_agent_observability_state_path(project_root);
    if let Some(parent) = ledger_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &ledger_path,
        serde_json::to_string_pretty(&ledger).expect("observability json should render"),
    )
    .map_err(|error| format!("Failed to write {}: {error}", ledger_path.display()))?;
    Ok(event)
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    })
}

fn clamp_score(value: f64) -> u64 {
    value.round().clamp(0.0, 100.0) as u64
}

fn codex_scorecard_feedback_rows(
    scorecards: &serde_json::Value,
    role_id: &str,
) -> Vec<serde_json::Value> {
    scorecards["agents"][role_id]["feedback"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn load_or_initialize_codex_worker_scorecards(
    project_root: &Path,
    codex_roles: &[serde_json::Value],
) -> serde_json::Value {
    let path = codex_worker_scorecards_state_path(project_root);
    let mut scorecards = read_json_file_if_present(&path).unwrap_or_else(|| {
        serde_json::json!({
            "schema_version": 1,
            "updated_at": time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("rfc3339 timestamp should render"),
            "agents": {}
        })
    });

    let Some(agents) = scorecards["agents"].as_object_mut() else {
        scorecards["agents"] = serde_json::json!({});
        let agents = scorecards["agents"]
            .as_object_mut()
            .expect("agents object should exist");
        for row in codex_roles {
            if let Some(role_id) = row["role_id"].as_str() {
                agents.insert(role_id.to_string(), serde_json::json!({"feedback": []}));
            }
        }
        let body =
            serde_json::to_string_pretty(&scorecards).expect("scorecards json should render");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, body);
        return scorecards;
    };

    let mut changed = false;
    for row in codex_roles {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        if !agents.contains_key(role_id) {
            agents.insert(role_id.to_string(), serde_json::json!({"feedback": []}));
            changed = true;
        }
    }
    if changed {
        scorecards["updated_at"] = serde_json::Value::String(
            time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("rfc3339 timestamp should render"),
        );
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let body =
            serde_json::to_string_pretty(&scorecards).expect("scorecards json should render");
        let _ = fs::write(&path, body);
    }
    scorecards
}

fn refresh_codex_worker_strategy(
    project_root: &Path,
    codex_roles: &[serde_json::Value],
    scoring_policy: &serde_json::Value,
) -> serde_json::Value {
    let scorecards = load_or_initialize_codex_worker_scorecards(project_root, codex_roles);
    let promotion_score = json_u64(json_lookup(scoring_policy, &["promotion_score"])).unwrap_or(75);
    let demotion_score = json_u64(json_lookup(scoring_policy, &["demotion_score"])).unwrap_or(45);
    let consecutive_failure_limit =
        json_u64(json_lookup(scoring_policy, &["consecutive_failure_limit"])).unwrap_or(3);
    let probation_task_runs =
        json_u64(json_lookup(scoring_policy, &["probation_task_runs"])).unwrap_or(2);
    let retirement_failure_limit =
        json_u64(json_lookup(scoring_policy, &["retirement_failure_limit"])).unwrap_or(8);

    let mut agents = serde_json::Map::new();
    for row in codex_roles {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let tier = row["tier"].as_str().unwrap_or(role_id);
        let rate = row["rate"].as_u64().unwrap_or(0);
        let feedback_rows = codex_scorecard_feedback_rows(&scorecards, role_id);
        let feedback_count = feedback_rows.len() as u64;
        let mut success_runs = 0u64;
        let mut failure_runs = 0u64;
        let mut last_feedback_score = None::<u64>;
        let mut total_feedback_score = 0f64;
        let mut consecutive_failures = 0u64;

        for item in &feedback_rows {
            let outcome = item["outcome"].as_str().unwrap_or("neutral");
            let score = json_u64(item.get("score")).unwrap_or(70);
            last_feedback_score = Some(score);
            total_feedback_score += score as f64;
            if outcome == "success" {
                success_runs += 1;
                consecutive_failures = 0;
            } else if outcome == "failure" {
                failure_runs += 1;
                consecutive_failures += 1;
            } else {
                consecutive_failures = 0;
            }
        }

        let average_feedback = if feedback_count > 0 {
            total_feedback_score / feedback_count as f64
        } else {
            70.0
        };
        let base_score = 70.0;
        let success_bonus = (success_runs as f64 * 2.5).min(15.0);
        let failure_penalty = (failure_runs as f64 * 4.0).min(28.0);
        let feedback_adjustment = (average_feedback - 70.0) * 0.45;
        let effective_score =
            clamp_score(base_score + success_bonus - failure_penalty + feedback_adjustment);
        let lifecycle_state = if consecutive_failures >= retirement_failure_limit {
            "retired"
        } else if consecutive_failures >= consecutive_failure_limit
            || effective_score < demotion_score
        {
            "degraded"
        } else if feedback_count < probation_task_runs {
            "probation"
        } else if effective_score >= promotion_score {
            "promoted"
        } else {
            "active"
        };

        agents.insert(
            role_id.to_string(),
            serde_json::json!({
                "tier": tier,
                "rate": rate,
                "reasoning_band": row["reasoning_band"],
                "default_runtime_role": row["default_runtime_role"],
                "task_classes": row["task_classes"],
                "feedback_count": feedback_count,
                "success_runs": success_runs,
                "failure_runs": failure_runs,
                "consecutive_failures": consecutive_failures,
                "average_feedback": (average_feedback * 100.0).round() / 100.0,
                "last_feedback_score": last_feedback_score,
                "effective_score": effective_score,
                "lifecycle_state": lifecycle_state,
            }),
        );
    }

    let strategy = serde_json::json!({
        "schema_version": 1,
        "updated_at": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "store_path": CODEX_WORKER_STRATEGY_STATE,
        "scorecards_path": CODEX_WORKER_SCORECARDS_STATE,
        "selection_policy": {
            "rule": "capability_first_then_score_guard_then_cheapest_tier",
            "promotion_score": promotion_score,
            "demotion_score": demotion_score,
            "consecutive_failure_limit": consecutive_failure_limit,
            "retirement_failure_limit": retirement_failure_limit
        },
        "agents": agents
    });
    let path = codex_worker_strategy_state_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(&strategy).expect("strategy json should render");
    let _ = fs::write(&path, body);
    strategy
}

fn append_host_agent_feedback(
    project_root: &Path,
    input: &HostAgentFeedbackInput<'_>,
) -> Result<serde_json::Value, String> {
    if input.score > 100 {
        return Err("Feedback score must be between 0 and 100.".to_string());
    }
    let overlay = read_yaml_file_checked(&project_root.join("vida.config.yaml"))
        .map_err(|error| format!("Failed to read project overlay: {error}"))?;
    let selected_cli_system = yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(normalize_host_cli_system)
        .ok_or_else(|| {
            "Host CLI system is missing or unsupported in vida.config.yaml.".to_string()
        })?;
    match selected_cli_system {
        "codex" => {
            let codex_roles = {
                let overlay_roles = overlay_codex_agent_catalog(&overlay);
                if overlay_roles.is_empty() {
                    read_codex_agent_catalog(&project_root.join(".codex"))
                } else {
                    overlay_roles
                }
            };
            if !codex_roles
                .iter()
                .any(|row| row["role_id"].as_str() == Some(input.agent_id))
            {
                return Err(format!(
                    "Unknown host agent `{}` for selected CLI system `codex`.",
                    input.agent_id
                ));
            }
            let scorecards_path = codex_worker_scorecards_state_path(project_root);
            let mut scorecards =
                load_or_initialize_codex_worker_scorecards(project_root, &codex_roles);
            if !scorecards["agents"][input.agent_id]["feedback"].is_array() {
                scorecards["agents"][input.agent_id]["feedback"] = serde_json::json!([]);
            }
            let feedback_rows = scorecards["agents"][input.agent_id]["feedback"]
                .as_array_mut()
                .expect("feedback array should initialize");
            feedback_rows.push(serde_json::json!({
                "recorded_at": time::OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .expect("rfc3339 timestamp should render"),
                "score": input.score,
                "outcome": input.outcome,
                "task_class": input.task_class,
                "notes": input.notes.unwrap_or(""),
                "source": input.source,
                "task_id": input.task_id.unwrap_or(""),
                "task_display_id": input.task_display_id.unwrap_or(""),
                "task_title": input.task_title.unwrap_or(""),
            }));
            scorecards["updated_at"] = serde_json::Value::String(
                time::OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .expect("rfc3339 timestamp should render"),
            );
            if let Some(parent) = scorecards_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(
                &scorecards_path,
                serde_json::to_string_pretty(&scorecards).expect("scorecards json should render"),
            )
            .map_err(|error| format!("Failed to write {}: {error}", scorecards_path.display()))?;
            let scoring_policy = serde_json::to_value(
                yaml_lookup(&overlay, &["agent_system", "scoring"])
                    .cloned()
                    .unwrap_or(serde_yaml::Value::Null),
            )
            .unwrap_or(serde_json::Value::Null);
            let strategy =
                refresh_codex_worker_strategy(project_root, &codex_roles, &scoring_policy);
            let observability_event = append_host_agent_observability_event(project_root, input)?;
            Ok(serde_json::json!({
                "surface": "vida agent-feedback",
                "host_cli_system": "codex",
                "agent_id": input.agent_id,
                "recorded_score": input.score,
                "recorded_outcome": input.outcome,
                "recorded_task_class": input.task_class,
                "recorded_notes": input.notes.unwrap_or(""),
                "scorecards_store": CODEX_WORKER_SCORECARDS_STATE,
                "strategy_store": CODEX_WORKER_STRATEGY_STATE,
                "observability_store": HOST_AGENT_OBSERVABILITY_STATE,
                "strategy_row": strategy["agents"][input.agent_id],
                "observability_event": observability_event
            }))
        }
        other => Err(format!(
            "Feedback writeback is not implemented for host CLI system `{other}`."
        )),
    }
}

fn build_codex_pricing_policy(
    codex_roles: &[serde_json::Value],
    worker_strategy: &serde_json::Value,
) -> serde_json::Value {
    let mut tiers = codex_roles.to_vec();
    tiers.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
    });
    let mut rate_map = serde_json::Map::new();
    for row in &tiers {
        if let (Some(tier), Some(rate)) = (row["tier"].as_str(), row["rate"].as_u64()) {
            rate_map.insert(tier.to_string(), serde_json::Value::Number(rate.into()));
        }
    }
    serde_json::json!({
        "selection_rule": "choose the cheapest configured agent that satisfies runtime-role admissibility, task-class fit, and the local score guard; escalate only when the cheaper candidate is degraded or score-insufficient",
        "rates": rate_map,
        "vendor_basis": {
            "openai": "structured role configs and reasoning-effort tuning in Codex project config; prefer explicit task/runtime settings over implicit chat heuristics",
            "anthropic": "structured prompt templates, explicit sections, and evaluation-backed refinement loops",
            "microsoft": "record one bounded decision/design artifact per change and route on explicit cost-quality tradeoffs instead of ad hoc escalation"
        },
        "local_strategy_store": worker_strategy["store_path"],
        "local_scorecards_store": worker_strategy["scorecards_path"],
        "tiers": tiers
    })
}

fn resolve_host_cli_template_source(cli_system: &str) -> Result<PathBuf, String> {
    let source_root = resolve_init_bootstrap_source_root();
    match cli_system {
        "codex" => {
            let template_root = source_root.join(".codex");
            if template_root.is_dir() {
                Ok(template_root)
            } else {
                Err(format!(
                    "Missing framework host CLI template for `{cli_system}`: {}",
                    template_root.display()
                ))
            }
        }
        other => Err(format!("Unsupported host CLI system `{other}`")),
    }
}

fn apply_host_cli_selection(project_root: &Path, cli_system: &str) -> Result<(), String> {
    let config_path = project_root.join("vida.config.yaml");
    if !config_path.is_file() {
        return Err(format!(
            "Missing project overlay; expected {} before host CLI activation",
            config_path.display()
        ));
    }
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?;
    let updated = if contents.contains(HOST_CLI_PLACEHOLDER) {
        contents.replace(HOST_CLI_PLACEHOLDER, cli_system)
    } else if contents.contains("host_environment:") && contents.contains("cli_system:") {
        let mut rewritten = String::new();
        let mut replaced = false;
        for line in contents.lines() {
            if line.trim_start().starts_with("cli_system:") && !replaced {
                rewritten.push_str(&format!("  cli_system: {cli_system}\n"));
                replaced = true;
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        rewritten
    } else {
        format!(
            "{}\nhost_environment:\n  cli_system: {cli_system}\n",
            contents.trim_end()
        )
    };
    fs::write(&config_path, updated)
        .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))
}

fn materialize_host_cli_template(project_root: &Path, cli_system: &str) -> Result<(), String> {
    match cli_system {
        "codex" => {
            let source = resolve_host_cli_template_source(cli_system)?;
            copy_tree_if_missing(&source, &project_root.join(".codex"))?;
            let overlay = read_yaml_file_checked(&project_root.join("vida.config.yaml"))
                .unwrap_or(serde_yaml::Value::Null);
            let scoring_policy = serde_json::to_value(
                yaml_lookup(&overlay, &["agent_system", "scoring"])
                    .cloned()
                    .unwrap_or(serde_yaml::Value::Null),
            )
            .unwrap_or(serde_json::Value::Null);
            let codex_root = project_root.join(".codex");
            let codex_roles = {
                let overlay_roles = overlay_codex_agent_catalog(&overlay);
                if overlay_roles.is_empty() {
                    read_codex_agent_catalog(&codex_root)
                } else {
                    overlay_roles
                }
            };
            let codex_dispatch_aliases =
                overlay_codex_dispatch_alias_catalog(&overlay, &codex_roles);
            if !codex_roles.is_empty() {
                render_codex_template_from_catalog(
                    project_root,
                    &source,
                    &codex_roles,
                    &codex_dispatch_aliases,
                )?;
            }
            refresh_codex_worker_strategy(project_root, &codex_roles, &scoring_policy);
            Ok(())
        }
        other => Err(format!("Unsupported host CLI system `{other}`")),
    }
}

fn copy_file_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(source, target).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn write_file_if_missing(target: &Path, contents: &str) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))
}

#[derive(Debug, Clone)]
struct ProjectActivationAnswers {
    project_id: String,
    project_title: String,
    user_communication_language: String,
    reasoning_language: String,
    documentation_language: String,
    todo_protocol_language: String,
}

fn resolve_project_activation_answers(
    project_root: &Path,
    args: &ProjectActivatorArgs,
) -> Option<ProjectActivationAnswers> {
    let config_path = project_root.join("vida.config.yaml");
    let current_overlay = if config_path.is_file() {
        read_yaml_file_checked(&config_path).ok()
    } else {
        None
    };
    let current_project_id = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["project", "id"])))
        .filter(|value| !is_missing_or_placeholder(Some(value.as_str()), PROJECT_ID_PLACEHOLDER));
    let current_user_communication_language = current_overlay
        .as_ref()
        .and_then(|config| {
            yaml_string(yaml_lookup(
                config,
                &["language_policy", "user_communication"],
            ))
        })
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), USER_COMMUNICATION_PLACEHOLDER)
        });
    let current_reasoning_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "reasoning"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), REASONING_LANGUAGE_PLACEHOLDER)
        });
    let current_documentation_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "documentation"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), DOCUMENTATION_LANGUAGE_PLACEHOLDER)
        });
    let current_todo_protocol_language = current_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "todo_protocol"])))
        .filter(|value| {
            !is_missing_or_placeholder(Some(value.as_str()), TODO_PROTOCOL_LANGUAGE_PLACEHOLDER)
        });
    let any_input_provided = args.project_id.is_some()
        || args.project_name.is_some()
        || args.language.is_some()
        || args.user_communication_language.is_some()
        || args.reasoning_language.is_some()
        || args.documentation_language.is_some()
        || args.todo_protocol_language.is_some();
    if !any_input_provided {
        return None;
    }

    let project_id = trimmed_non_empty(args.project_id.as_deref())
        .or_else(|| {
            trimmed_non_empty(args.project_name.as_deref())
                .map(|name| slugify_project_id(&name))
                .filter(|value| !value.is_empty())
        })
        .or(current_project_id)?;
    let shared_language = trimmed_non_empty(args.language.as_deref());
    let user_communication_language =
        trimmed_non_empty(args.user_communication_language.as_deref())
            .or_else(|| shared_language.clone())
            .or(current_user_communication_language)?;
    let reasoning_language = trimmed_non_empty(args.reasoning_language.as_deref())
        .or_else(|| shared_language.clone())
        .or(current_reasoning_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let documentation_language = trimmed_non_empty(args.documentation_language.as_deref())
        .or_else(|| shared_language.clone())
        .or(current_documentation_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let todo_protocol_language = trimmed_non_empty(args.todo_protocol_language.as_deref())
        .or_else(|| shared_language)
        .or(current_todo_protocol_language)
        .unwrap_or_else(|| user_communication_language.clone());
    let project_title = inferred_project_title(&project_id, args.project_name.as_deref());

    Some(ProjectActivationAnswers {
        project_id,
        project_title,
        user_communication_language,
        reasoning_language,
        documentation_language,
        todo_protocol_language,
    })
}

fn apply_project_activation_answers(
    project_root: &Path,
    answers: &ProjectActivationAnswers,
) -> Result<Vec<String>, String> {
    let config_path = project_root.join("vida.config.yaml");
    if !config_path.is_file() {
        return Err(format!(
            "Missing project overlay; expected {} before project activation writes",
            config_path.display()
        ));
    }
    let original_contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?;
    let mut updated_contents = original_contents
        .replace(PROJECT_ID_PLACEHOLDER, &answers.project_id)
        .replace(DOCS_ROOT_PLACEHOLDER, DEFAULT_PROJECT_DOCS_ROOT)
        .replace(PROCESS_ROOT_PLACEHOLDER, DEFAULT_PROJECT_PROCESS_ROOT)
        .replace(RESEARCH_ROOT_PLACEHOLDER, DEFAULT_PROJECT_RESEARCH_ROOT)
        .replace(README_DOC_PLACEHOLDER, "README.md")
        .replace(
            ARCHITECTURE_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_ARCHITECTURE_DOC,
        )
        .replace(DECISIONS_DOC_PLACEHOLDER, DEFAULT_PROJECT_DECISIONS_DOC)
        .replace(
            ENVIRONMENTS_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_ENVIRONMENTS_DOC,
        )
        .replace(
            PROJECT_OPERATIONS_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_OPERATIONS_DOC,
        )
        .replace(
            AGENT_SYSTEM_DOC_PLACEHOLDER,
            DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
        )
        .replace(
            USER_COMMUNICATION_PLACEHOLDER,
            &answers.user_communication_language,
        )
        .replace(REASONING_LANGUAGE_PLACEHOLDER, &answers.reasoning_language)
        .replace(
            DOCUMENTATION_LANGUAGE_PLACEHOLDER,
            &answers.documentation_language,
        )
        .replace(
            TODO_PROTOCOL_LANGUAGE_PLACEHOLDER,
            &answers.todo_protocol_language,
        );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project",
        "id",
        &answers.project_id,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "docs_root",
        DEFAULT_PROJECT_DOCS_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "process_root",
        DEFAULT_PROJECT_PROCESS_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "research_root",
        DEFAULT_PROJECT_RESEARCH_ROOT,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "readme_doc",
        "README.md",
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "architecture_doc",
        DEFAULT_PROJECT_ARCHITECTURE_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "decisions_doc",
        DEFAULT_PROJECT_DECISIONS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "environments_doc",
        DEFAULT_PROJECT_ENVIRONMENTS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "project_operations_doc",
        DEFAULT_PROJECT_OPERATIONS_DOC,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "agent_system_doc",
        DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
    );
    updated_contents = set_yaml_bool_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "allow_scaffold_missing",
        false,
    );
    updated_contents = set_yaml_bool_in_top_level_section(
        &updated_contents,
        "project_bootstrap",
        "require_launch_confirmation",
        false,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "user_communication",
        &answers.user_communication_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "reasoning",
        &answers.reasoning_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "documentation",
        &answers.documentation_language,
    );
    updated_contents = set_yaml_scalar_in_top_level_section(
        &updated_contents,
        "language_policy",
        "todo_protocol",
        &answers.todo_protocol_language,
    );

    let mut changed_files = Vec::new();
    if updated_contents != original_contents {
        fs::write(&config_path, updated_contents)
            .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))?;
        changed_files.push("vida.config.yaml".to_string());
    }

    let generated_files = vec![
        (
            project_root.join("AGENTS.sidecar.md"),
            render_project_sidecar(&answers.project_title),
            "AGENTS.sidecar.md",
        ),
        (
            project_root.join("README.md"),
            render_project_readme(&answers.project_title),
            "README.md",
        ),
        (
            project_root.join(DEFAULT_PROJECT_ROOT_MAP),
            render_project_root_map(),
            DEFAULT_PROJECT_ROOT_MAP,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PRODUCT_INDEX),
            render_project_product_index(),
            DEFAULT_PROJECT_PRODUCT_INDEX,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PRODUCT_SPEC_README),
            render_project_product_spec_readme(),
            DEFAULT_PROJECT_PRODUCT_SPEC_README,
        ),
        (
            project_root.join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE),
            fs::read_to_string(
                resolve_init_bootstrap_source_root()
                    .join("docs/framework/templates/feature-design-document.template.md"),
            )
            .map_err(|error| {
                format!("Failed to read framework feature-design template source: {error}")
            })?,
            DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
        ),
        (
            project_root.join(DEFAULT_PROJECT_ARCHITECTURE_DOC),
            render_project_architecture_doc(),
            DEFAULT_PROJECT_ARCHITECTURE_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_PROCESS_README),
            render_project_process_readme(),
            DEFAULT_PROJECT_PROCESS_README,
        ),
        (
            project_root.join(DEFAULT_PROJECT_DECISIONS_DOC),
            render_project_decisions_doc(answers),
            DEFAULT_PROJECT_DECISIONS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_ENVIRONMENTS_DOC),
            render_project_environments_doc(project_root),
            DEFAULT_PROJECT_ENVIRONMENTS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_OPERATIONS_DOC),
            render_project_operations_doc(),
            DEFAULT_PROJECT_OPERATIONS_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_AGENT_SYSTEM_DOC),
            render_project_agent_system_doc(),
            DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_CODEX_GUIDE_DOC),
            render_project_codex_guide(),
            DEFAULT_PROJECT_CODEX_GUIDE_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_DOC_TOOLING_DOC),
            render_project_doc_tooling_map(),
            DEFAULT_PROJECT_DOC_TOOLING_DOC,
        ),
        (
            project_root.join(DEFAULT_PROJECT_RESEARCH_README),
            render_project_research_readme(),
            DEFAULT_PROJECT_RESEARCH_README,
        ),
    ];

    for (path, contents, label) in generated_files {
        if write_file_if_missing_or_placeholder(&path, &contents)? {
            changed_files.push(label.to_string());
        }
    }

    Ok(changed_files)
}

fn write_project_activation_receipt(
    project_root: &Path,
    answers: Option<&ProjectActivationAnswers>,
    host_cli_system: Option<&str>,
    changed_files: &[String],
    host_template_materialized: bool,
) -> Result<Option<String>, String> {
    if changed_files.is_empty() && answers.is_none() && !host_template_materialized {
        return Ok(None);
    }
    let receipts_dir = project_root.join(".vida/receipts");
    fs::create_dir_all(&receipts_dir)
        .map_err(|error| format!("Failed to create {}: {error}", receipts_dir.display()))?;
    let now = time::OffsetDateTime::now_utc();
    let recorded_at = now
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt_name = format!("project-activation-{}.json", now.unix_timestamp());
    let receipt_path = receipts_dir.join(receipt_name);
    let host_template_root = host_cli_system
        .and_then(|system| normalize_host_cli_system(system))
        .and_then(|system| resolve_host_cli_template_source(system).ok())
        .map(|path| path.display().to_string());
    let default_agent_templates = host_template_root
        .as_deref()
        .map(Path::new)
        .map(list_host_cli_agent_templates)
        .unwrap_or_default();
    let receipt = serde_json::json!({
        "receipt_kind": "project_activation",
        "recorded_at": recorded_at,
        "surface": "vida project-activator",
        "project_root": project_root.display().to_string(),
        "activation_mode": "bounded_interview_then_materialize",
        "docflow_first": true,
        "taskflow_admitted_while_pending": false,
        "non_canonical_taskflow_surfaces_forbidden_while_pending": ["vida taskflow", "external_taskflow_runtime"],
        "answers": answers.map(|answers| serde_json::json!({
            "project_id": answers.project_id,
            "project_title": answers.project_title,
            "user_communication_language": answers.user_communication_language,
            "reasoning_language": answers.reasoning_language,
            "documentation_language": answers.documentation_language,
            "todo_protocol_language": answers.todo_protocol_language,
        })),
        "host_cli_system": host_cli_system,
        "host_template_materialized": host_template_materialized,
        "default_host_agent_templates": default_agent_templates,
        "changed_files": changed_files,
        "log_note": "Use `vida docflow` for subsequent documentation validation/readiness work; project activation itself is a bounded onboarding/configuration path, not TaskFlow execution.",
    });
    let body =
        serde_json::to_string_pretty(&receipt).expect("project activation receipt should render");
    fs::write(&receipt_path, &body)
        .map_err(|error| format!("Failed to write {}: {error}", receipt_path.display()))?;
    fs::write(project_root.join(PROJECT_ACTIVATION_RECEIPT_LATEST), &body).map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            project_root
                .join(PROJECT_ACTIVATION_RECEIPT_LATEST)
                .display()
        )
    })?;
    Ok(Some(
        receipt_path
            .strip_prefix(project_root)
            .unwrap_or(&receipt_path)
            .display()
            .to_string(),
    ))
}

fn render_project_sidecar(project_title: &str) -> String {
    format!(
        "# Project Docs Map\n\n\
Repository: `{project_title}`\n\n\
1. Current project root map:\n\
   - `{DEFAULT_PROJECT_ROOT_MAP}`\n\
2. Project product index:\n\
   - `{DEFAULT_PROJECT_PRODUCT_INDEX}`\n\
3. Project product spec/readiness guide:\n\
   - `{DEFAULT_PROJECT_PRODUCT_SPEC_README}`\n\
4. Local feature/change design template:\n\
   - `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`\n\
5. Project process index:\n\
   - `{DEFAULT_PROJECT_PROCESS_README}`\n\
6. Project documentation tooling map:\n\
   - `{DEFAULT_PROJECT_DOC_TOOLING_DOC}`\n\
7. Project agent-system baseline:\n\
   - `{DEFAULT_PROJECT_AGENT_SYSTEM_DOC}`\n\
8. Project Codex agent guide:\n\
   - `{DEFAULT_PROJECT_CODEX_GUIDE_DOC}`\n\
9. Project research index:\n\
   - `{DEFAULT_PROJECT_RESEARCH_README}`\n\n\
Working rule:\n\
1. Use this sidecar as the project docs map after framework bootstrap.\n\
2. For bounded feature/change work that asks for research, specification, planning, and implementation, start with the local feature-design template and the documentation tooling path before code execution.\n\
3. While project activation is pending, prefer `vida project-activator` for activation mutations and `vida docflow` for documentation/readiness inspection.\n"
    )
}

fn render_project_readme(project_title: &str) -> String {
    format!(
        "# {project_title}\n\n\
This repository contains a VIDA-initialized project scaffold.\n\n\
Use `AGENTS.md` for framework bootstrap, `AGENTS.sidecar.md` for project docs routing, and `docs/` for project-owned operating context.\n"
    )
}

fn render_project_root_map() -> String {
    format!(
        "# Project Root Map\n\n\
This project uses the following canonical documentation roots:\n\n\
- `docs/product/` for product-facing intent and architecture notes\n\
- `docs/process/` for project operations and working agreements\n\
- `docs/research/` for research notes and discovery artifacts\n\n\
Primary pointers:\n\n\
- Product index: `{DEFAULT_PROJECT_PRODUCT_INDEX}`\n\
- Product spec/readiness guide: `{DEFAULT_PROJECT_PRODUCT_SPEC_README}`\n\
- Feature design template: `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`\n\
- Process index: `{DEFAULT_PROJECT_PROCESS_README}`\n\
- Documentation tooling: `{DEFAULT_PROJECT_DOC_TOOLING_DOC}`\n\
- Codex agent guide: `{DEFAULT_PROJECT_CODEX_GUIDE_DOC}`\n\
- Research index: `{DEFAULT_PROJECT_RESEARCH_README}`\n\
- Repository overview: `README.md`\n"
    )
}

fn render_project_product_index() -> String {
    format!(
        "# Product Index\n\n\
Product documentation currently contains:\n\n\
- `{DEFAULT_PROJECT_ARCHITECTURE_DOC}` for the initial project architecture outline\n\
- `{DEFAULT_PROJECT_PRODUCT_SPEC_README}` for bounded feature/change design and ADR routing\n"
    )
}

fn render_project_product_spec_readme() -> String {
    format!(
        "# Product Spec Guide\n\n\
Use this directory for bounded product-facing feature/change design documents and linked ADRs.\n\n\
Default rule:\n\n\
1. If a request asks for research, detailed specifications, implementation planning, and then code, create or update one bounded design document before implementation.\n\
2. Start from the local template at `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before normal implementation work begins.\n\
4. Use `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check` to keep the document canonical.\n\
5. Close the spec-pack task only after the design artifact is finalized and validated, then hand off through the next TaskFlow packet.\n\
6. When one major decision needs durable standalone recording, add a linked ADR instead of overloading the design document.\n\
\n\
Suggested homes:\n\n\
- `docs/product/spec/<feature>-design.md` for committed feature/change designs\n\
- `docs/research/<topic>.md` for exploratory research before design closure\n"
    )
}

fn render_project_architecture_doc() -> String {
    "# Architecture\n\nCurrent project posture:\n\n- VIDA bootstrap scaffold is initialized\n- project documentation roots are materialized\n- project-specific implementation modules are not yet defined\n".to_string()
}

fn render_project_process_readme() -> String {
    format!(
        "# Process Docs\n\n\
This directory contains the minimum process documentation expected by VIDA activation.\n\n\
Available process docs:\n\n\
- `decisions.md`\n\
- `environments.md`\n\
- `project-operations.md`\n\
- `agent-system.md`\n\
- `documentation-tooling-map.md`\n\
- `codex-agent-configuration-guide.md`\n"
    )
}

fn render_project_decisions_doc(answers: &ProjectActivationAnswers) -> String {
    format!(
        "# Decisions\n\n\
Initial activation decisions:\n\n\
- project id: `{}`\n\
- host CLI system: selected through `vida project-activator`\n\
- language policy:\n  - user communication: `{}`\n  - reasoning: `{}`\n  - documentation: `{}`\n  - todo protocol: `{}`\n",
        answers.project_id,
        answers.user_communication_language,
        answers.reasoning_language,
        answers.documentation_language,
        answers.todo_protocol_language
    )
}

fn render_project_environments_doc(project_root: &Path) -> String {
    format!(
        "# Environments\n\n\
Initial environment assumptions:\n\n\
- local project root: `{}`\n\
- VIDA runtime directories are managed under `.vida/`\n\
- host CLI agent template is selected through `vida project-activator`\n",
        project_root.display()
    )
}

fn render_project_operations_doc() -> String {
    format!(
        "# Project Operations\n\n\
Current operating baseline:\n\n\
- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces\n\
- use `AGENTS.sidecar.md` as the project documentation map\n\
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`\n\
\n\
Default feature-delivery flow:\n\n\
1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.\n\
2. Use the local template at `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.\n\
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.\n\
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.\n\
6. When `.codex/**` is materialized, use the delegated Codex team surface instead of collapsing the root session directly into coding.\n\
7. Treat `vida.config.yaml` as the owner of carrier tiers and optional internal Codex aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.\n\
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.\n\
9. Keep the root session in orchestration posture unless an explicit exception path is recorded.\n"
    )
}

fn render_project_agent_system_doc() -> String {
    "# Agent System\n\nProject activation owns host CLI agent-template selection and runtime admission.\n\n- default framework agent templates become available only after the selected host CLI template is materialized\n- the current supported host CLI system list is framework-owned; the current supported value is `codex`\n- Codex carrier-tier metadata is owned by `vida.config.yaml -> host_environment.codex.agents`\n- `vida.config.yaml -> host_environment.codex.dispatch_aliases` is the canonical internal alias registry and is not the primary project-visible agent model\n- `.codex/**` is the rendered host executor surface, not the owner of tier/rate/task-class policy\n- project activation materializes the carrier tiers into `.codex/config.toml` and `.codex/agents/*.toml`\n- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`\n- project-local agent extensions remain under `.vida/project/agent-extensions/`\n- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists\n".to_string()
}

fn render_project_doc_tooling_map() -> String {
    format!(
        "# Documentation Tooling Map\n\n\
Use `vida docflow` for documentation inventory, mutation, validation, and readiness checks.\n\n\
Design-document rule:\n\n\
1. For bounded feature/change work that requires research, detailed specifications, planning, and implementation, begin with one design document before code execution.\n\
2. Start from `{DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE}`.\n\
3. Open one epic and one spec-pack task in `vida taskflow` before writing code.\n\
4. Suggested command sequence:\n\
   - `vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec \"initialize feature design\"`\n\
   - edit the document using the local template shape\n\
   - `vida docflow finalize-edit docs/product/spec/<feature>-design.md \"record bounded feature design\"`\n\
   - `vida docflow check --root . docs/product/spec/<feature>-design.md`\n\
   - `vida taskflow task close <spec-task-id> --reason \"design packet finalized and handed off\" --json`\n\
\n\
Activation rule:\n\n\
1. During project activation, `vida project-activator` owns bounded config/doc materialization.\n\
2. `vida taskflow` and any non-canonical external TaskFlow runtime are not lawful activation-entry surfaces while activation is pending.\n\
3. After activation writes, prefer `vida docflow` for documentation-oriented inspection and proof before multi-step implementation.\n"
    )
}

fn render_project_research_readme() -> String {
    "# Research Notes\n\nUse this directory for research artifacts, discovery notes, and external references that support future project work.\n".to_string()
}

fn render_project_codex_guide() -> String {
    "# Codex Agent Configuration Guide\n\nThis project uses framework-materialized `.codex/**` as the local Codex runtime surface.\n\nSource-of-truth rule:\n\n- `vida.config.yaml -> host_environment.codex.agents` owns carrier-tier metadata, rates, runtime-role fit, and task-class fit\n- `vida.config.yaml -> host_environment.codex.dispatch_aliases` is the canonical internal alias registry for executor-local overlays\n- `.codex/**` is the rendered executor surface used by Codex after activation\n- `.codex/config.toml` should expose the carrier tiers materialized from overlay\n\nCarrier rule:\n\n- the primary visible agent model is `junior`, `middle`, `senior`, `architect`\n- runtime role remains explicit activation state such as `worker`, `coach`, `verifier`, or `solution_architect`\n- internal alias ids may exist in overlay state, but they must not replace the carrier-tier model at the project surface\n\nWorking rule:\n\n1. The root session stays the orchestrator.\n2. Documentation/specification work should complete the bounded design document first.\n3. Before delegated implementation starts, open the feature epic/spec task in `vida taskflow` and close the spec task only after the design artifact is finalized.\n4. After a bounded packet exists, route research, specification, planning, implementation, review, and verification through the configured tier ladder instead of collapsing into root-session coding.\n5. Let runtime choose the cheapest capable configured carrier tier with a healthy local score from `.vida/state/worker-strategy.json` and pass the lawful runtime role explicitly.\n6. Use `.vida/project/agent-extensions/**` for project-local role and skill overlays; do not treat `.codex/**` as the owner of framework or product law.\n".to_string()
}

fn runtime_agent_extensions_root(project_root: &Path) -> PathBuf {
    project_root.join(".vida/project/agent-extensions")
}

fn registry_sidecar_path(registry_path: &Path) -> PathBuf {
    let Some(file_name) = registry_path.file_name().and_then(|value| value.to_str()) else {
        return registry_path.with_extension("sidecar");
    };
    if let Some(stripped) = file_name.strip_suffix(".yaml") {
        return registry_path.with_file_name(format!("{stripped}.sidecar.yaml"));
    }
    registry_path.with_file_name(format!("{file_name}.sidecar"))
}

fn write_runtime_agent_extension_projections(project_root: &Path) -> Result<(), String> {
    let root = runtime_agent_extensions_root(project_root);
    ensure_dir(&root)?;
    write_file_if_missing(
        &root.join("README.md"),
        DEFAULT_RUNTIME_AGENT_EXTENSIONS_README,
    )?;
    write_file_if_missing(&root.join("roles.yaml"), DEFAULT_AGENT_EXTENSION_ROLES_YAML)?;
    write_file_if_missing(
        &root.join("skills.yaml"),
        DEFAULT_AGENT_EXTENSION_SKILLS_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.yaml"),
        DEFAULT_AGENT_EXTENSION_PROFILES_YAML,
    )?;
    write_file_if_missing(&root.join("flows.yaml"), DEFAULT_AGENT_EXTENSION_FLOWS_YAML)?;
    write_file_if_missing(
        &root.join("roles.sidecar.yaml"),
        DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.sidecar.yaml"),
        DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.sidecar.yaml"),
        DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.sidecar.yaml"),
        DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML,
    )?;

    let receipt_path = project_root.join(".vida/receipts/agent-extensions-bootstrap.json");
    if !receipt_path.exists() {
        let generated_at = time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render");
        let receipt = serde_json::json!({
            "receipt_kind": "agent_extensions_bootstrap",
            "generated_at": generated_at,
            "project_root": project_root.display().to_string(),
            "runtime_projection_root": root.display().to_string(),
            "base_projection_files": [
                ".vida/project/agent-extensions/README.md",
                ".vida/project/agent-extensions/roles.yaml",
                ".vida/project/agent-extensions/skills.yaml",
                ".vida/project/agent-extensions/profiles.yaml",
                ".vida/project/agent-extensions/flows.yaml"
            ],
            "sidecar_projection_files": [
                ".vida/project/agent-extensions/roles.sidecar.yaml",
                ".vida/project/agent-extensions/skills.sidecar.yaml",
                ".vida/project/agent-extensions/profiles.sidecar.yaml",
                ".vida/project/agent-extensions/flows.sidecar.yaml"
            ],
            "source": "vida init default runtime projection bootstrap"
        });
        write_file_if_missing(
            &receipt_path,
            &serde_json::to_string_pretty(&receipt)
                .expect("agent extension bootstrap receipt should render"),
        )?;
    }

    Ok(())
}

fn preserve_existing_agents_as_sidecar(project_root: &Path) -> Result<(), String> {
    let agents = project_root.join("AGENTS.md");
    let sidecar = project_root.join("AGENTS.sidecar.md");
    if !agents.exists() || sidecar.exists() {
        return Ok(());
    }
    fs::rename(&agents, &sidecar).map_err(|error| {
        format!(
            "Failed to preserve existing {} as {}: {error}",
            agents.display(),
            sidecar.display()
        )
    })
}

fn print_init_summary(project_root: &Path, activation_view: &serde_json::Value) {
    println!("vida init project bootstrap ready");
    println!("project root: {}", project_root.display());
    println!(
        "materialized: AGENTS.md, AGENTS.sidecar.md, vida.config.yaml, .vida/config, .vida/db, .vida/cache, .vida/framework, .vida/project, .vida/project/agent-extensions/*, .vida/project/agent-extensions/*.sidecar.yaml, .vida/receipts, .vida/runtime, .vida/scratchpad"
    );
    println!(
        "activation status: {}",
        activation_view["status"].as_str().unwrap_or("unknown")
    );
    if activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true)
    {
        println!("next step: vida project-activator --json");
        if let Some(example) = activation_view["interview"]["one_shot_example"].as_str() {
            println!("activation example: {example}");
        }
        if let Some(step) = activation_view["next_steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .next()
        {
            println!("activation note: {step}");
        }
        println!("activation rule: while activation is pending, use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime");
    }
}

fn read_yaml_file_checked(path: &Path) -> Result<serde_yaml::Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn repo_runtime_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("repo root should exist two levels above crates/vida")
}

fn block_on_state_store<T>(
    future: impl std::future::Future<Output = Result<T, StateStoreError>>,
) -> Result<T, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future))
            .map_err(|error| error.to_string()),
        Err(_) => tokio::runtime::Runtime::new()
            .map_err(|error| format!("Failed to initialize Tokio runtime: {error}"))?
            .block_on(future)
            .map_err(|error| error.to_string()),
    }
}

fn taskflow_native_state_root(project_root: &Path) -> PathBuf {
    project_root.join(state_store::default_state_dir())
}

fn task_record_json(task: &TaskRecord) -> serde_json::Value {
    serde_json::to_value(task).expect("task record should serialize")
}

fn run_task_store_native_fallback(
    project_root: &Path,
    args: &[String],
) -> Result<serde_json::Value, String> {
    let state_root = taskflow_native_state_root(project_root);
    match args {
        [subcommand, tail @ ..] if subcommand == "list" => {
            let mut status = None::<String>;
            let mut include_all = false;
            let mut i = 0usize;
            while i < tail.len() {
                match tail[i].as_str() {
                    "--status" if i + 1 < tail.len() => {
                        status = Some(tail[i + 1].clone());
                        i += 2;
                    }
                    "--all" => {
                        include_all = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let tasks = block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.list_tasks(status.as_deref(), include_all).await
            })?;
            Ok(serde_json::to_value(tasks).expect("task list should serialize"))
        }
        [subcommand, task_id] if subcommand == "show" => {
            match block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.show_task(task_id).await
            }) {
                Ok(task) => Ok(task_record_json(&task)),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand] if subcommand == "ready" => {
            let tasks = block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.ready_tasks().await
            })?;
            Ok(serde_json::to_value(tasks).expect("ready task list should serialize"))
        }
        [subcommand, source] if subcommand == "import-jsonl" => {
            let summary = block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.import_tasks_from_jsonl(Path::new(source)).await
            })?;
            Ok(serde_json::json!({
                "status": "ok",
                "imported_count": summary.imported_count,
                "unchanged_count": summary.unchanged_count,
                "updated_count": summary.updated_count,
                "source_path": summary.source_path,
            }))
        }
        [subcommand, target] if subcommand == "export-jsonl" => {
            let exported_count = block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.export_tasks_to_jsonl(Path::new(target)).await
            })?;
            Ok(serde_json::json!({
                "status": "ok",
                "exported_count": exported_count,
                "target_path": target,
            }))
        }
        [subcommand, task_id, title, rest @ ..] if subcommand == "create" => {
            let mut issue_type = "task".to_string();
            let mut status = "open".to_string();
            let mut priority = 2u32;
            let mut parent_id = None::<String>;
            let mut description = String::new();
            let mut labels = Vec::new();
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--type" if i + 1 < rest.len() => {
                        issue_type = rest[i + 1].clone();
                        i += 2;
                    }
                    "--status" if i + 1 < rest.len() => {
                        status = rest[i + 1].clone();
                        i += 2;
                    }
                    "--priority" if i + 1 < rest.len() => {
                        priority = rest[i + 1].parse::<u32>().unwrap_or(2);
                        i += 2;
                    }
                    "--parent-id" if i + 1 < rest.len() => {
                        parent_id = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = rest[i + 1].clone();
                        i += 2;
                    }
                    "--labels" if i + 1 < rest.len() => {
                        labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--display-id" if i + 1 < rest.len() => {
                        i += 2;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            match block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store
                    .create_task(
                        task_id,
                        title,
                        &description,
                        &issue_type,
                        &status,
                        priority,
                        parent_id.as_deref(),
                        &labels,
                        "vida taskflow",
                        &project_root.display().to_string(),
                    )
                    .await
            }) {
                Ok(task) => Ok(task_record_json(&task)),
                Err(error) if error.contains("task already exists") => Ok(serde_json::json!({
                    "status": "error",
                    "reason": "task_already_exists",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand, task_id, rest @ ..] if subcommand == "update" => {
            let mut status = None::<String>;
            let mut notes = None::<String>;
            let mut description = None::<String>;
            let mut add_labels = Vec::new();
            let mut remove_labels = Vec::new();
            let mut set_labels = None::<Vec<String>>;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--status" if i + 1 < rest.len() => {
                        status = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--notes" if i + 1 < rest.len() => {
                        notes = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--add-label" if i + 1 < rest.len() => {
                        add_labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--remove-label" if i + 1 < rest.len() => {
                        remove_labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--set-labels" if i + 1 < rest.len() => {
                        set_labels = Some(
                            rest[i + 1]
                                .split(',')
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(|value| value.to_string())
                                .collect::<Vec<_>>(),
                        );
                        i += 2;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            match block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store
                    .update_task(
                        task_id,
                        status.as_deref(),
                        notes.as_deref(),
                        description.as_deref(),
                        &add_labels,
                        &remove_labels,
                        set_labels.as_deref(),
                    )
                    .await
            }) {
                Ok(task) => Ok(task_record_json(&task)),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand, task_id, flag, reason] if subcommand == "close" && flag == "--reason" => {
            match block_on_state_store(async {
                let store = StateStore::open(state_root.clone()).await?;
                store.close_task(task_id, reason).await
            }) {
                Ok(task) => Ok(serde_json::json!({
                    "status": "ok",
                    "task": task,
                })),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        _ => Err("unsupported_taskflow_task_bridge".to_string()),
    }
}

fn run_task_store_helper(
    project_root: &Path,
    args: &[String],
) -> Result<serde_json::Value, String> {
    run_task_store_native_fallback(project_root, args)
}

fn print_jsonl_value(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string(value).expect("jsonl payload should render")
    );
}

fn print_json_pretty(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("json payload should render")
    );
}

fn parse_display_path(display_id: &str) -> Option<(String, Vec<u32>)> {
    let trimmed = display_id.trim();
    if !trimmed.starts_with("vida-") {
        return None;
    }
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts[0].len() <= 5 {
        return None;
    }
    let mut levels = Vec::new();
    for part in parts.iter().skip(1) {
        levels.push(part.parse::<u32>().ok()?);
    }
    Some((parts[0].to_string(), levels))
}

fn next_display_id_payload(
    rows: &[serde_json::Value],
    parent_display_id: &str,
) -> serde_json::Value {
    let Some((parent_root, parent_levels)) = parse_display_path(parent_display_id) else {
        return serde_json::json!({
            "valid": false,
            "reason": "invalid_parent_display_id",
            "parent_display_id": parent_display_id,
        });
    };

    let mut max_child = 0u32;
    for row in rows {
        let display_id = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        let Some((child_root, child_levels)) = parse_display_path(display_id) else {
            continue;
        };
        if child_root != parent_root || child_levels.len() != parent_levels.len() + 1 {
            continue;
        }
        if !parent_levels.is_empty() && child_levels[..parent_levels.len()] != parent_levels[..] {
            continue;
        }
        max_child = max_child.max(*child_levels.last().unwrap_or(&0));
    }

    let next_index = max_child + 1;
    serde_json::json!({
        "valid": true,
        "parent_display_id": parent_display_id,
        "next_display_id": format!("{parent_display_id}.{next_index}"),
        "next_index": next_index,
    })
}

fn resolve_task_id_by_display_id(
    rows: &[serde_json::Value],
    display_id: &str,
) -> serde_json::Value {
    for row in rows {
        let current = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        if current == display_id {
            return serde_json::json!({
                "found": true,
                "display_id": display_id,
                "task_id": row.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(),
            });
        }
    }
    serde_json::json!({
        "found": false,
        "display_id": display_id,
        "reason": "parent_display_id_not_found",
    })
}

fn helper_value_is_missing(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(|status| status == "missing")
        .unwrap_or(false)
}

fn helper_value_is_ok(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(|status| status == "ok")
        .unwrap_or(true)
}

fn task_payload_by_id(
    project_root: &Path,
    task_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let payload = run_task_store_helper(project_root, &["show".to_string(), task_id.to_string()])?;
    if helper_value_is_missing(&payload)
        || (payload.get("reason").and_then(serde_json::Value::as_str)
            == Some("invalid_helper_output")
            && payload
                .get("output")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("no such table: tasks")))
    {
        Ok(None)
    } else if helper_value_is_ok(&payload) {
        Ok(Some(payload))
    } else {
        Err(format!(
            "Failed to inspect task `{task_id}` through the native TaskFlow state store."
        ))
    }
}

fn create_task_if_missing_with_store(
    store: &StateStore,
    project_root: &Path,
    task_id: &str,
    title: &str,
    issue_type: &str,
    status: &str,
    parent_id: Option<&str>,
    labels: &[&str],
    description: Option<&str>,
) -> Result<(serde_json::Value, bool), String> {
    if let Ok(existing) = block_on_state_store(store.show_task(task_id)) {
        return Ok((task_record_json(&existing), false));
    }
    let label_rows = labels
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    match block_on_state_store(store.create_task(
        task_id,
        title,
        description.unwrap_or_default(),
        issue_type,
        status,
        2,
        parent_id,
        &label_rows,
        "vida taskflow",
        &project_root.display().to_string(),
    )) {
        Ok(task) => Ok((task_record_json(&task), true)),
        Err(error) if error.contains("task already exists") => {
            let existing = block_on_state_store(store.show_task(task_id))?;
            Ok((task_record_json(&existing), false))
        }
        Err(error) => Err(error),
    }
}

fn run_docflow_cli_command(project_root: &Path, args: &[String]) -> Result<String, String> {
    let previous = std::env::var_os("VIDA_ROOT");
    std::env::set_var("VIDA_ROOT", project_root);
    let argv = std::iter::once("docflow".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let result = DocflowCli::try_parse_from(argv)
        .map_err(|error| error.to_string())
        .map(docflow_cli::run);
    match previous {
        Some(value) => std::env::set_var("VIDA_ROOT", value),
        None => std::env::remove_var("VIDA_ROOT"),
    }
    result
}

fn docflow_output_is_ok(output: &str) -> bool {
    !output.contains("verdict: blocking")
        && !output.contains("validation: blocking")
        && !output.contains("error:")
        && !output.contains("❌ BLOCKING:")
}

fn register_design_doc_in_spec_readme(
    project_root: &Path,
    design_doc_rel: &str,
) -> Result<bool, String> {
    let readme_path = project_root.join(DEFAULT_PROJECT_PRODUCT_SPEC_README);
    if !readme_path.is_file() {
        if let Some(parent) = readme_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
        }
        fs::write(&readme_path, render_project_product_spec_readme())
            .map_err(|error| format!("Failed to write {}: {error}", readme_path.display()))?;
    }
    let mut content = fs::read_to_string(&readme_path)
        .map_err(|error| format!("Failed to read {}: {error}", readme_path.display()))?;
    if content.contains(design_doc_rel) {
        return Ok(false);
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.contains("Active design docs:") {
        content.push_str("\nActive design docs:\n\n");
    }
    content.push_str(&format!("- `{design_doc_rel}`\n"));
    fs::write(&readme_path, content)
        .map_err(|error| format!("Failed to write {}: {error}", readme_path.display()))?;
    Ok(true)
}

fn ensure_project_docs_sidecar_pointers(project_root: &Path) -> Result<bool, String> {
    let sidecar_path = project_root.join("AGENTS.sidecar.md");
    if !sidecar_path.is_file() {
        return Err(format!(
            "Missing project docs sidecar: {}",
            sidecar_path.display()
        ));
    }
    let mut content = fs::read_to_string(&sidecar_path)
        .map_err(|error| format!("Failed to read {}: {error}", sidecar_path.display()))?;
    let mut changed = false;
    for pointer in [
        "docs/project-root-map.md",
        "docs/process/documentation-tooling-map.md",
    ] {
        if !content.contains(pointer) {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&format!("- `{pointer}`\n"));
            changed = true;
        }
    }
    if changed {
        fs::write(&sidecar_path, content)
            .map_err(|error| format!("Failed to write {}: {error}", sidecar_path.display()))?;
    }
    Ok(changed)
}

fn write_spec_bootstrap_receipt(
    project_root: &Path,
    request: &str,
    feature_slug: &str,
    epic_task_id: &str,
    spec_task_id: &str,
    design_doc_path: &str,
    changed_files: &[String],
) -> Result<String, String> {
    let receipts_dir = project_root.join(".vida/receipts");
    fs::create_dir_all(&receipts_dir)
        .map_err(|error| format!("Failed to create {}: {error}", receipts_dir.display()))?;
    let now = time::OffsetDateTime::now_utc();
    let recorded_at = now
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt_path = receipts_dir.join(format!("spec-bootstrap-{}.json", now.unix_timestamp()));
    let receipt = serde_json::json!({
        "receipt_kind": "spec_bootstrap",
        "recorded_at": recorded_at,
        "surface": "vida taskflow bootstrap-spec",
        "project_root": project_root.display().to_string(),
        "request": request,
        "feature_slug": feature_slug,
        "epic_task_id": epic_task_id,
        "spec_task_id": spec_task_id,
        "design_doc_path": design_doc_path,
        "changed_files": changed_files,
        "next_step": "finalize the design document through vida docflow, then close the spec task and continue through the next TaskFlow packet",
    });
    let body =
        serde_json::to_string_pretty(&receipt).expect("spec bootstrap receipt should render");
    fs::write(&receipt_path, &body)
        .map_err(|error| format!("Failed to write {}: {error}", receipt_path.display()))?;
    fs::write(project_root.join(SPEC_BOOTSTRAP_RECEIPT_LATEST), &body).map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            project_root.join(SPEC_BOOTSTRAP_RECEIPT_LATEST).display()
        )
    })?;
    Ok(receipt_path
        .strip_prefix(project_root)
        .unwrap_or(&receipt_path)
        .display()
        .to_string())
}

fn execute_taskflow_bootstrap_spec_with_store(
    project_root: &Path,
    store: &StateStore,
    request_text: &str,
    tracked: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let feature_slug = tracked["feature_slug"]
        .as_str()
        .unwrap_or("feature-request");
    let design_doc_path = tracked["design_doc_path"]
        .as_str()
        .unwrap_or("docs/product/spec/feature-request-design.md");
    let artifact_path = tracked["design_artifact_path"]
        .as_str()
        .unwrap_or("product/spec/feature-request-design");
    let epic_task_id = tracked["epic"]["task_id"]
        .as_str()
        .unwrap_or("feature-request");
    let epic_title = tracked["epic"]["title"].as_str().unwrap_or("Feature epic");
    let spec_task_id = tracked["spec_task"]["task_id"]
        .as_str()
        .unwrap_or("feature-request-spec");
    let spec_title = tracked["spec_task"]["title"]
        .as_str()
        .unwrap_or("Spec pack");

    let mut changed_files = Vec::new();
    let (_, epic_created) = create_task_if_missing_with_store(
        store,
        project_root,
        epic_task_id,
        epic_title,
        "epic",
        "open",
        None,
        &["feature-request", "spec-first"],
        Some(request_text),
    )?;
    if epic_created {
        changed_files.push(format!("taskflow:{epic_task_id}"));
    }

    let (_, spec_created) = create_task_if_missing_with_store(
        store,
        project_root,
        spec_task_id,
        spec_title,
        "task",
        "open",
        Some(epic_task_id),
        &["spec-pack", "documentation"],
        Some("bounded design/spec packet for the feature request"),
    )?;
    if spec_created {
        changed_files.push(format!("taskflow:{spec_task_id}"));
    }

    match ensure_project_docs_sidecar_pointers(project_root) {
        Ok(true) => changed_files.push("AGENTS.sidecar.md".to_string()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }

    match register_design_doc_in_spec_readme(project_root, design_doc_path) {
        Ok(true) => changed_files.push(DEFAULT_PROJECT_PRODUCT_SPEC_README.to_string()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }

    let design_doc_abs = project_root.join(design_doc_path);
    let design_doc_created = if design_doc_abs.exists() {
        false
    } else {
        let init_output = run_docflow_cli_command(
            project_root,
            &[
                "init".to_string(),
                design_doc_path.to_string(),
                artifact_path.to_string(),
                "product_spec".to_string(),
                "initialize bounded feature design".to_string(),
            ],
        )?;
        if !docflow_output_is_ok(&init_output) {
            return Err(init_output);
        }
        changed_files.push(design_doc_path.to_string());
        true
    };

    let check_output = run_docflow_cli_command(
        project_root,
        &[
            "check-file".to_string(),
            "--path".to_string(),
            design_doc_path.to_string(),
        ],
    )?;
    if !docflow_output_is_ok(&check_output) {
        return Err(check_output);
    }

    let receipt_path = write_spec_bootstrap_receipt(
        project_root,
        request_text,
        feature_slug,
        epic_task_id,
        spec_task_id,
        design_doc_path,
        &changed_files,
    )?;

    Ok(serde_json::json!({
        "surface": "vida taskflow bootstrap-spec",
        "status": "ok",
        "request": request_text,
        "feature_slug": feature_slug,
        "epic": {
            "task_id": epic_task_id,
            "created": epic_created,
        },
        "spec_task": {
            "task_id": spec_task_id,
            "created": spec_created,
        },
        "design_doc": {
            "path": design_doc_path,
            "created": design_doc_created,
            "registered_in": [DEFAULT_PROJECT_PRODUCT_SPEC_README],
        },
        "next": {
            "finalize_command": tracked["docflow"]["finalize_command"].clone(),
            "check_command": tracked["docflow"]["check_command"].clone(),
            "close_spec_task_command": tracked["spec_task"]["close_command"].clone(),
        },
        "receipt_path": receipt_path,
        "changed_files": changed_files,
    }))
}

fn execute_work_packet_create_with_store(
    project_root: &Path,
    store: &StateStore,
    request_text: &str,
    tracked: &serde_json::Value,
    packet_key: &str,
) -> Result<serde_json::Value, String> {
    let tracked = if tracked[packet_key]["task_id"].as_str().is_some() {
        tracked.clone()
    } else {
        build_design_first_tracked_flow_bootstrap(request_text)
    };
    let epic_task_id = tracked["epic"]["task_id"]
        .as_str()
        .unwrap_or("feature-request");
    let epic_title = tracked["epic"]["title"].as_str().unwrap_or("Feature epic");
    let task = &tracked[packet_key];
    let task_id = task["task_id"]
        .as_str()
        .ok_or_else(|| format!("missing task_id for `{packet_key}`"))?;
    let title = task["title"]
        .as_str()
        .ok_or_else(|| format!("missing title for `{packet_key}`"))?;
    let packet_label = if packet_key == "work_pool_task" {
        "work-pool-pack"
    } else {
        "dev-pack"
    };
    let packet_description = if packet_key == "work_pool_task" {
        "tracked work-pool packet created from runtime consumption"
    } else {
        "tracked dev packet created from runtime consumption"
    };
    let mut changed_files = Vec::new();

    let (_, epic_created) = create_task_if_missing_with_store(
        store,
        project_root,
        epic_task_id,
        epic_title,
        "epic",
        "open",
        None,
        &["feature-request"],
        Some("tracked feature epic for runtime-consumption dispatch"),
    )?;
    if epic_created {
        changed_files.push(format!("taskflow:{epic_task_id}"));
    }

    let (_, packet_created) = create_task_if_missing_with_store(
        store,
        project_root,
        task_id,
        title,
        "task",
        "open",
        Some(epic_task_id),
        &[packet_label],
        Some(packet_description),
    )?;
    if packet_created {
        changed_files.push(format!("taskflow:{task_id}"));
    }

    Ok(serde_json::json!({
        "surface": "vida taskflow task create",
        "status": "ok",
        "packet_key": packet_key,
        "epic": {
            "task_id": epic_task_id,
            "created": epic_created,
        },
        "task": {
            "task_id": task_id,
            "created": packet_created,
            "label": packet_label,
        },
        "changed_files": changed_files,
    }))
}

fn infer_codex_task_class_from_task_payload(task: &serde_json::Value) -> String {
    let labels = task["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut text = String::new();
    if let Some(title) = task["title"].as_str() {
        text.push_str(title);
        text.push(' ');
    }
    if let Some(description) = task["description"].as_str() {
        text.push_str(description);
    }
    let normalized = text.to_ascii_lowercase();

    if labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "architecture" | "solution-architect" | "hard-escalation" | "escalation"
        )
    }) || contains_keywords(
        &normalized,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "migration".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "hard escalation".to_string(),
        ],
    )
    .len()
        >= 1
    {
        return "architecture".to_string();
    }
    if labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "verification" | "review" | "proof" | "release-readiness"
        )
    }) || contains_keywords(
        &normalized,
        &[
            "verify".to_string(),
            "verification".to_string(),
            "review".to_string(),
            "audit".to_string(),
            "proof".to_string(),
            "release readiness".to_string(),
        ],
    )
    .len()
        >= 1
    {
        return "verification".to_string();
    }
    if labels
        .iter()
        .any(|label| matches!(label.as_str(), "spec-pack" | "documentation" | "planning"))
        || contains_keywords(
            &normalized,
            &[
                "spec".to_string(),
                "design".to_string(),
                "research".to_string(),
                "plan".to_string(),
                "requirements".to_string(),
            ],
        )
        .len()
            >= 1
    {
        return "specification".to_string();
    }
    "implementation".to_string()
}

fn infer_feedback_outcome_from_close_reason(reason: &str) -> &'static str {
    let normalized = reason.to_ascii_lowercase();
    if contains_keywords(
        &normalized,
        &[
            "fail".to_string(),
            "failed".to_string(),
            "blocked".to_string(),
            "abort".to_string(),
            "abandon".to_string(),
            "rejected".to_string(),
            "rollback".to_string(),
        ],
    )
    .len()
        >= 1
    {
        "failure"
    } else if contains_keywords(
        &normalized,
        &[
            "neutral".to_string(),
            "partial".to_string(),
            "handoff".to_string(),
            "handoff pending".to_string(),
        ],
    )
    .len()
        >= 1
    {
        "neutral"
    } else {
        "success"
    }
}

fn default_feedback_score(outcome: &str, task_class: &str) -> u64 {
    match outcome {
        "failure" => 35,
        "neutral" => 60,
        _ => match task_class {
            "architecture" => 90,
            "verification" => 88,
            "specification" => 84,
            _ => 82,
        },
    }
}

fn maybe_record_task_close_host_agent_feedback(
    project_root: &Path,
    task: &serde_json::Value,
    close_reason: &str,
) -> serde_json::Value {
    let overlay = match read_yaml_file_checked(&project_root.join("vida.config.yaml")) {
        Ok(overlay) => overlay,
        Err(error) => {
            return serde_json::json!({
                "status": "skipped",
                "reason": format!("overlay_unavailable: {error}")
            });
        }
    };
    let selected_cli_system = yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(normalize_host_cli_system);
    if selected_cli_system != Some("codex") {
        return serde_json::json!({
            "status": "skipped",
            "reason": "host_cli_not_selected_or_unsupported"
        });
    }

    let compiled_bundle =
        match build_compiled_agent_extension_bundle_for_root(&overlay, project_root) {
            Ok(bundle) => bundle,
            Err(error) => {
                return serde_json::json!({
                    "status": "error",
                    "reason": format!("compiled_bundle_failed: {error}")
                });
            }
        };
    let task_class = infer_codex_task_class_from_task_payload(task);
    let runtime_role = codex_runtime_role_for_task_class(&task_class);
    let assignment = build_codex_runtime_assignment_from_resolved_constraints(
        &compiled_bundle,
        "orchestrator",
        &task_class,
        runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        return serde_json::json!({
            "status": "skipped",
            "reason": assignment["reason"].as_str().unwrap_or("codex_runtime_assignment_disabled"),
            "task_class": task_class,
            "runtime_role": runtime_role,
        });
    }

    let agent_id = assignment["selected_agent_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if agent_id.is_empty() {
        return serde_json::json!({
            "status": "error",
            "reason": "selected_agent_id_missing",
            "task_class": task_class,
            "runtime_role": runtime_role,
        });
    }
    let outcome = infer_feedback_outcome_from_close_reason(close_reason);
    let score = default_feedback_score(outcome, &task_class);
    let input = HostAgentFeedbackInput {
        agent_id: &agent_id,
        score,
        outcome,
        task_class: &task_class,
        notes: Some("automatic task-close feedback"),
        source: "vida taskflow task close",
        task_id: task["id"].as_str(),
        task_display_id: task["display_id"].as_str(),
        task_title: task["title"].as_str(),
        runtime_role: assignment["runtime_role"].as_str(),
        selected_tier: assignment["selected_tier"].as_str(),
        estimated_task_price_units: assignment["estimated_task_price_units"].as_u64(),
        lifecycle_state: assignment["lifecycle_state"].as_str(),
        effective_score: assignment["effective_score"].as_u64(),
        reason: Some(close_reason),
    };
    match append_host_agent_feedback(project_root, &input) {
        Ok(view) => serde_json::json!({
            "status": "recorded",
            "task_class": task_class,
            "runtime_role": runtime_role,
            "assignment": assignment,
            "feedback": view,
        }),
        Err(error) => serde_json::json!({
            "status": "error",
            "reason": error,
            "task_class": task_class,
            "runtime_role": runtime_role,
            "assignment": assignment,
        }),
    }
}

fn infer_project_root_from_state_root(state_root: &Path) -> Option<PathBuf> {
    state_root
        .ancestors()
        .find(|path| looks_like_project_root(path))
        .map(Path::to_path_buf)
}

fn build_host_agent_status_summary(project_root: &Path) -> Option<serde_json::Value> {
    let overlay = read_yaml_file_checked(&project_root.join("vida.config.yaml")).ok()?;
    let selected_cli_system = yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(normalize_host_cli_system)?;
    match selected_cli_system {
        "codex" => {
            let overlay_roles = overlay_codex_agent_catalog(&overlay);
            let codex_roles = if overlay_roles.is_empty() {
                read_codex_agent_catalog(&project_root.join(".codex"))
            } else {
                overlay_roles
            };
            let strategy =
                read_json_file_if_present(&codex_worker_strategy_state_path(project_root))
                    .unwrap_or(serde_json::Value::Null);
            let scorecards =
                read_json_file_if_present(&codex_worker_scorecards_state_path(project_root))
                    .unwrap_or(serde_json::Value::Null);
            let observability =
                read_json_file_if_present(&host_agent_observability_state_path(project_root))
                    .unwrap_or_else(|| {
                        load_or_initialize_host_agent_observability_state(project_root)
                    });
            let latest_events = observability["events"]
                .as_array()
                .map(|events| events.iter().rev().take(5).cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            let mut agents = serde_json::Map::new();
            for role in &codex_roles {
                let Some(role_id) = role["role_id"].as_str() else {
                    continue;
                };
                let feedback_rows = scorecards["agents"][role_id]["feedback"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let last_feedback = feedback_rows
                    .last()
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                agents.insert(
                    role_id.to_string(),
                    serde_json::json!({
                        "tier": role["tier"],
                        "rate": role["rate"],
                        "reasoning_band": role["reasoning_band"],
                        "default_runtime_role": role["default_runtime_role"],
                        "runtime_roles": role["runtime_roles"],
                        "task_classes": role["task_classes"],
                        "feedback_count": feedback_rows.len(),
                        "last_feedback_at": last_feedback["recorded_at"],
                        "last_feedback_outcome": last_feedback["outcome"],
                        "effective_score": strategy["agents"][role_id]["effective_score"],
                        "lifecycle_state": strategy["agents"][role_id]["lifecycle_state"],
                    }),
                );
            }
            let overlay_dispatch_aliases =
                overlay_codex_dispatch_alias_catalog(&overlay, &codex_roles);

            Some(serde_json::json!({
                "host_cli_system": "codex",
                "runtime_surface": ".codex/**",
                "stores": {
                    "scorecards": CODEX_WORKER_SCORECARDS_STATE,
                    "strategy": CODEX_WORKER_STRATEGY_STATE,
                    "observability": HOST_AGENT_OBSERVABILITY_STATE,
                },
                "selection_policy": strategy["selection_policy"],
                "budget": observability["budget"],
                "agents": agents,
                "internal_dispatch_alias_count": overlay_dispatch_aliases.len(),
                "recent_events": latest_events,
            }))
        }
        _ => None,
    }
}

fn render_task_list_payload(payload: &serde_json::Value, as_json: bool) -> ExitCode {
    if as_json {
        print_json_pretty(payload);
    } else if let Some(rows) = payload.as_array() {
        for row in rows {
            print_jsonl_value(row);
        }
    } else {
        print_json_pretty(payload);
    }
    ExitCode::SUCCESS
}

fn run_taskflow_task_bridge(project_root: &Path, args: &[String]) -> Result<ExitCode, String> {
    match args {
        [head] if head == "task" => {
            print_taskflow_proxy_help(Some("task"));
            Ok(ExitCode::SUCCESS)
        }
        [head, flag] if head == "task" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("task"));
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, ..] if head == "task" && subcommand == "list" => {
            let mut helper_args = vec!["list".to_string()];
            let mut status = None::<String>;
            let mut include_all = false;
            let mut as_json = false;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--status" if i + 1 < args.len() => {
                        status = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--all" => {
                        include_all = true;
                        i += 1;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            if let Some(status) = status {
                helper_args.extend(["--status".to_string(), status]);
            }
            if include_all {
                helper_args.push("--all".to_string());
            }
            let payload = run_task_store_helper(project_root, &helper_args)?;
            Ok(render_task_list_payload(&payload, as_json))
        }
        [head, subcommand, task_id, tail @ ..] if head == "task" && subcommand == "show" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            let as_jsonl = tail.iter().any(|arg| arg == "--jsonl");
            if tail
                .iter()
                .any(|arg| !matches!(arg.as_str(), "--json" | "--jsonl"))
            {
                return Err("unsupported delegated task arguments".to_string());
            }
            let mut payload =
                run_task_store_helper(project_root, &["show".to_string(), task_id.clone()])?;
            if helper_value_is_missing(&payload) && task_id.starts_with("vida-") {
                let rows = run_task_store_helper(
                    project_root,
                    &["list".to_string(), "--all".to_string()],
                )?;
                if let Some(entries) = rows.as_array() {
                    let resolved = resolve_task_id_by_display_id(entries, task_id);
                    if resolved
                        .get("found")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        let resolved_id = resolved
                            .get("task_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        payload = run_task_store_helper(
                            project_root,
                            &["show".to_string(), resolved_id],
                        )?;
                    }
                }
            }
            if helper_value_is_missing(&payload) {
                if as_json {
                    print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            if as_json {
                print_json_pretty(&payload);
            } else if as_jsonl {
                print_jsonl_value(&payload);
            } else {
                print_json_pretty(&payload);
            }
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, ..] if head == "task" && subcommand == "ready" => {
            let as_json = args.iter().any(|arg| arg == "--json");
            if args
                .iter()
                .skip(2)
                .any(|arg| !matches!(arg.as_str(), "--json"))
            {
                return Err("unsupported delegated task arguments".to_string());
            }
            let payload = run_task_store_helper(project_root, &["ready".to_string()])?;
            Ok(render_task_list_payload(&payload, as_json))
        }
        [head, subcommand, source, tail @ ..] if head == "task" && subcommand == "import-jsonl" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let payload =
                run_task_store_helper(project_root, &["import-jsonl".to_string(), source.clone()])?;
            if as_json {
                print_json_pretty(&payload);
            } else {
                println!(
                    "{}: imported={} unchanged={} updated={}",
                    payload
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("error"),
                    payload
                        .get("imported_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("unchanged_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("updated_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                );
            }
            Ok(if helper_value_is_ok(&payload) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, target, tail @ ..] if head == "task" && subcommand == "export-jsonl" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let payload =
                run_task_store_helper(project_root, &["export-jsonl".to_string(), target.clone()])?;
            if as_json {
                print_json_pretty(&payload);
            } else {
                println!(
                    "{}: exported={} target={}",
                    payload
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("error"),
                    payload
                        .get("exported_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("target_path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(target)
                );
            }
            Ok(if helper_value_is_ok(&payload) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, parent_display_id, tail @ ..]
            if head == "task" && subcommand == "next-display-id" =>
        {
            let as_json = tail.iter().any(|arg| arg == "--json");
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let rows =
                run_task_store_helper(project_root, &["list".to_string(), "--all".to_string()])?;
            let entries = rows
                .as_array()
                .ok_or_else(|| "task list payload should be an array".to_string())?;
            let payload = next_display_id_payload(entries, parent_display_id);
            let valid = payload
                .get("valid")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if as_json {
                print_json_pretty(&payload);
            } else {
                print_json_pretty(&payload);
            }
            Ok(if valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, task_id, title, rest @ ..]
            if head == "task" && subcommand == "create" =>
        {
            let mut issue_type = "task".to_string();
            let mut status = "open".to_string();
            let mut priority = "2".to_string();
            let mut display_id = String::new();
            let mut parent_id = String::new();
            let mut parent_display_id = String::new();
            let mut auto_display_from = String::new();
            let mut description = String::new();
            let mut labels = Vec::new();
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--type" if i + 1 < rest.len() => {
                        issue_type = rest[i + 1].clone();
                        i += 2;
                    }
                    "--status" if i + 1 < rest.len() => {
                        status = rest[i + 1].clone();
                        i += 2;
                    }
                    "--priority" if i + 1 < rest.len() => {
                        priority = rest[i + 1].clone();
                        i += 2;
                    }
                    "--display-id" if i + 1 < rest.len() => {
                        display_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--parent-id" if i + 1 < rest.len() => {
                        parent_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--parent-display-id" if i + 1 < rest.len() => {
                        parent_display_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--auto-display-from" if i + 1 < rest.len() => {
                        auto_display_from = rest[i + 1].clone();
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = rest[i + 1].clone();
                        i += 2;
                    }
                    "--labels" if i + 1 < rest.len() => {
                        labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            if (display_id.is_empty() && !auto_display_from.is_empty())
                || (parent_id.is_empty() && !parent_display_id.is_empty())
            {
                let rows = run_task_store_helper(
                    project_root,
                    &["list".to_string(), "--all".to_string()],
                )?;
                let entries = rows
                    .as_array()
                    .ok_or_else(|| "task list payload should be an array".to_string())?;
                if display_id.is_empty() && !auto_display_from.is_empty() {
                    let next = next_display_id_payload(entries, &auto_display_from);
                    if !next
                        .get("valid")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        if as_json {
                            print_json_pretty(&next);
                        } else {
                            eprintln!(
                                "{}",
                                next.get("reason")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("invalid_parent_display_id")
                            );
                        }
                        return Ok(ExitCode::from(1));
                    }
                    display_id = next
                        .get("next_display_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
                if parent_id.is_empty() && !parent_display_id.is_empty() {
                    let resolved = resolve_task_id_by_display_id(entries, &parent_display_id);
                    if !resolved
                        .get("found")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        if as_json {
                            print_json_pretty(&resolved);
                        } else {
                            eprintln!(
                                "{}",
                                resolved
                                    .get("reason")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("parent_display_id_not_found")
                            );
                        }
                        return Ok(ExitCode::from(1));
                    }
                    parent_id = resolved
                        .get("task_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
            }
            let mut helper_args = vec![
                "create".to_string(),
                task_id.clone(),
                title.clone(),
                "--type".to_string(),
                issue_type,
                "--status".to_string(),
                status,
                "--priority".to_string(),
                priority,
            ];
            if !display_id.is_empty() {
                helper_args.extend(["--display-id".to_string(), display_id]);
            }
            if !parent_id.is_empty() {
                helper_args.extend(["--parent-id".to_string(), parent_id]);
            }
            if !description.is_empty() {
                helper_args.extend(["--description".to_string(), description]);
            }
            for label in labels {
                helper_args.extend(["--labels".to_string(), label]);
            }
            let payload = run_task_store_helper(project_root, &helper_args)?;
            if as_json {
                print_json_pretty(&payload);
            } else {
                print_json_pretty(&payload);
            }
            Ok(
                if helper_value_is_missing(&payload)
                    || payload.get("reason").and_then(serde_json::Value::as_str)
                        == Some("task_already_exists")
                {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                },
            )
        }
        [head, subcommand, task_id, rest @ ..] if head == "task" && subcommand == "update" => {
            let mut helper_args = vec!["update".to_string(), task_id.clone()];
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--status" | "--notes" | "--description" | "--add-label" | "--remove-label"
                    | "--set-labels"
                        if i + 1 < rest.len() =>
                    {
                        helper_args.push(rest[i].clone());
                        helper_args.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let payload = run_task_store_helper(project_root, &helper_args)?;
            if helper_value_is_missing(&payload) {
                if as_json {
                    print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            if as_json {
                print_json_pretty(&payload);
            } else {
                print_json_pretty(&payload);
            }
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, task_id, rest @ ..] if head == "task" && subcommand == "close" => {
            let mut reason = None::<String>;
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--reason" if i + 1 < rest.len() => {
                        reason = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let reason = reason.ok_or_else(|| {
                "Usage: vida taskflow task close <task_id> --reason <reason> [--json]".to_string()
            })?;
            let existing_task = task_payload_by_id(project_root, task_id).ok().flatten();
            let close_reason = reason.clone();
            let payload = run_task_store_helper(
                project_root,
                &[
                    "close".to_string(),
                    task_id.clone(),
                    "--reason".to_string(),
                    reason,
                ],
            )?;
            if helper_value_is_missing(&payload) {
                if as_json {
                    print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            let mut render_payload = payload.clone();
            let telemetry_task = existing_task
                .or_else(|| payload.get("task").cloned())
                .or_else(|| task_payload_by_id(project_root, task_id).ok().flatten());
            render_payload["host_agent_telemetry"] = match telemetry_task.as_ref() {
                Some(task) => {
                    maybe_record_task_close_host_agent_feedback(project_root, task, &close_reason)
                }
                None => serde_json::json!({
                    "status": "skipped",
                    "reason": "task_payload_unavailable_before_close"
                }),
            };
            if as_json {
                print_json_pretty(&render_payload);
            } else {
                print_json_pretty(&render_payload);
            }
            Ok(ExitCode::SUCCESS)
        }
        _ => Err("unsupported_taskflow_task_bridge".to_string()),
    }
}

async fn run_taskflow_bootstrap_spec(args: &[String]) -> ExitCode {
    let as_json = args.iter().any(|arg| arg == "--json");
    let request_text = args
        .iter()
        .skip(1)
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if request_text.is_empty() {
        eprintln!("Usage: vida taskflow bootstrap-spec <request_text> [--json]");
        return ExitCode::from(2);
    }

    let project_root = match resolve_repo_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    let state_dir = proxy_state_dir();
    let store = match open_existing_state_store_with_retry(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let role_selection = match build_runtime_lane_selection_with_store(&store, &request_text).await
    {
        Ok(selection) => selection,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if role_selection.execution_plan["status"] != "design_first" {
        eprintln!(
            "Spec bootstrap is allowed only for design-first feature requests. Use `vida taskflow consume final <request> --json` to inspect the routed intake posture first."
        );
        return ExitCode::from(1);
    }

    let tracked = &role_selection.execution_plan["tracked_flow_bootstrap"];
    let payload = match execute_taskflow_bootstrap_spec_with_store(
        &project_root,
        &store,
        &request_text,
        tracked,
    ) {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .expect("spec bootstrap payload should render as json")
        );
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow bootstrap-spec");
        print_surface_line(
            RenderMode::Plain,
            "feature slug",
            payload["feature_slug"]
                .as_str()
                .unwrap_or("feature-request"),
        );
        print_surface_line(
            RenderMode::Plain,
            "epic",
            payload["epic"]["task_id"]
                .as_str()
                .unwrap_or("feature-request"),
        );
        print_surface_line(
            RenderMode::Plain,
            "spec task",
            payload["spec_task"]["task_id"]
                .as_str()
                .unwrap_or("feature-request-spec"),
        );
        print_surface_line(
            RenderMode::Plain,
            "design doc",
            payload["design_doc"]["path"]
                .as_str()
                .unwrap_or("docs/product/spec/feature-request-design.md"),
        );
        print_surface_line(
            RenderMode::Plain,
            "receipt path",
            payload["receipt_path"].as_str().unwrap_or(""),
        );
    }

    ExitCode::SUCCESS
}

pub(crate) fn proxy_state_dir() -> PathBuf {
    std::env::var_os("VIDA_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(state_store::default_state_dir)
}

pub(crate) async fn open_existing_state_store_with_retry(
    state_dir: PathBuf,
) -> Result<StateStore, StateStoreError> {
    for attempt in 0..80 {
        match StateStore::open_existing(state_dir.clone()).await {
            Ok(store) => return Ok(store),
            Err(StateStoreError::Db(error)) if attempt < 79 => {
                let message = error.to_string();
                if message.contains("LOCK") || message.contains("lock") {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                    continue;
                }
                return Err(StateStoreError::Db(error));
            }
            Err(error) => return Err(error),
        }
    }

    StateStore::open_existing(state_dir).await
}

struct TaskflowProtocolBindingSeed {
    protocol_id: &'static str,
    source_path: &'static str,
    activation_class: &'static str,
    runtime_owner: &'static str,
    enforcement_type: &'static str,
    proof_surface: &'static str,
}

fn taskflow_protocol_binding_seeds() -> &'static [TaskflowProtocolBindingSeed] {
    &[
        TaskflowProtocolBindingSeed {
            protocol_id: "instruction-contracts/bridge.instruction-activation-protocol",
            source_path:
                "vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md",
            activation_class: "always_on",
            runtime_owner: "vida::taskflow::protocol_binding::activation_bridge",
            enforcement_type: "activation-resolution",
            proof_surface: "vida docflow activation-check --profile active-canon",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.taskflow-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.taskflow-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::taskflow::protocol_binding::taskflow_surface",
            enforcement_type: "execution-discipline",
            proof_surface: "vida taskflow consume bundle check --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/runtime.task-state-telemetry-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::state_store::task_state_telemetry",
            enforcement_type: "state-telemetry",
            proof_surface: "vida status --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.execution-health-check-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::doctor::execution_health",
            enforcement_type: "health-gate",
            proof_surface: "vida taskflow doctor --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.task-state-reconciliation-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md",
            activation_class: "closure_reflection",
            runtime_owner: "vida::state_store::task_reconciliation",
            enforcement_type: "state-reconciliation",
            proof_surface: "vida status --json",
        },
    ]
}

#[derive(Clone, serde::Serialize)]
struct ProtocolBindingCompiledPayloadImportEvidence {
    imported: bool,
    trusted: bool,
    source: String,
    source_config_path: String,
    source_config_digest: String,
    captured_at: String,
    effective_bundle_receipt_id: String,
    effective_bundle_root_artifact_id: String,
    effective_bundle_artifact_count: usize,
    compiled_payload_summary: serde_json::Value,
    blockers: Vec<String>,
}

impl ProtocolBindingCompiledPayloadImportEvidence {
    fn trusted(source: &str) -> bool {
        matches!(source, "state_store" | TASKFLOW_PROTOCOL_BINDING_AUTHORITY)
    }
}

async fn protocol_binding_compiled_payload_import_evidence(
    store: &StateStore,
) -> ProtocolBindingCompiledPayloadImportEvidence {
    let mut blockers = Vec::new();

    let activation_snapshot = match read_or_sync_launcher_activation_snapshot(store).await {
        Ok(snapshot) => Some(snapshot),
        Err(error) => {
            blockers.push(format!("launcher_activation_snapshot_unavailable:{error}"));
            None
        }
    };
    let effective_bundle_receipt = match store.latest_effective_bundle_receipt_summary().await {
        Ok(receipt) => receipt,
        Err(error) => {
            blockers.push(format!("effective_bundle_receipt_unavailable:{error}"));
            None
        }
    };

    let (source, source_config_path, source_config_digest, captured_at, compiled_payload_summary) =
        if let Some(snapshot) = activation_snapshot.as_ref() {
            (
                snapshot.source.clone(),
                snapshot.source_config_path.clone(),
                snapshot.source_config_digest.clone(),
                snapshot.captured_at.clone(),
                serde_json::json!({
                    "selection_mode": snapshot.compiled_bundle["role_selection"]["mode"],
                    "fallback_role": snapshot.compiled_bundle["role_selection"]["fallback_role"],
                    "agent_system_mode": snapshot.compiled_bundle["agent_system"]["mode"],
                    "agent_system_state_owner": snapshot.compiled_bundle["agent_system"]["state_owner"],
                }),
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                serde_json::json!({}),
            )
        };

    if source.is_empty() {
        blockers.push("missing_launcher_activation_snapshot".to_string());
    } else if !ProtocolBindingCompiledPayloadImportEvidence::trusted(&source) {
        blockers.push(format!("untrusted_compiled_payload_source:{source}"));
    }
    if let Some(receipt) = effective_bundle_receipt.as_ref() {
        if receipt.receipt_id.trim().is_empty() {
            blockers.push("missing_effective_bundle_receipt_id".to_string());
        }
        if receipt.root_artifact_id.trim().is_empty() {
            blockers.push("missing_effective_bundle_root_artifact_id".to_string());
        }
        if receipt.artifact_count == 0 {
            blockers.push("empty_effective_bundle_artifact_count".to_string());
        }
    } else {
        blockers.push("missing_effective_bundle_receipt".to_string());
    }

    ProtocolBindingCompiledPayloadImportEvidence {
        imported: activation_snapshot.is_some() && effective_bundle_receipt.is_some(),
        trusted: blockers.is_empty(),
        source,
        source_config_path,
        source_config_digest,
        captured_at,
        effective_bundle_receipt_id: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_id.clone())
            .unwrap_or_default(),
        effective_bundle_root_artifact_id: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.root_artifact_id.clone())
            .unwrap_or_default(),
        effective_bundle_artifact_count: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.artifact_count)
            .unwrap_or_default(),
        compiled_payload_summary,
        blockers,
    }
}

fn resolve_protocol_binding_source_root() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(root) = resolve_repo_root() {
        candidates.push(root);
    }
    if let Some(installed_root) = resolve_installed_runtime_root() {
        candidates.push(installed_root.join("current"));
        candidates.push(installed_root);
    }
    let repo_root = repo_runtime_root();
    if !candidates.iter().any(|root| root == &repo_root) {
        candidates.push(repo_root);
    }

    first_existing_path(
        &candidates
            .into_iter()
            .map(|root| root.join("vida/config/instructions/system-maps/protocol.index.md"))
            .collect::<Vec<_>>(),
    )
    .and_then(|path| {
        path.parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    })
    .ok_or_else(|| {
        "Unable to resolve protocol-binding source root with vida/config/instructions/system-maps/protocol.index.md"
            .to_string()
    })
}

fn build_taskflow_protocol_binding_rows(
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> Result<Vec<ProtocolBindingState>, String> {
    let repo_root = resolve_protocol_binding_source_root()?;
    let protocol_index_path =
        repo_root.join("vida/config/instructions/system-maps/protocol.index.md");
    let protocol_index = fs::read_to_string(&protocol_index_path).map_err(|error| {
        format!(
            "Failed to read protocol index {}: {error}",
            protocol_index_path.display()
        )
    })?;

    let mut rows = Vec::new();
    for seed in taskflow_protocol_binding_seeds() {
        let source = repo_root.join(seed.source_path);
        let mut blockers = Vec::new();
        if !source.exists() {
            blockers.push(format!("missing_source_path:{}", seed.source_path));
        }
        if !protocol_index.contains(&format!("`{}`", seed.protocol_id)) {
            blockers.push(format!(
                "missing_protocol_index_binding:{}",
                seed.protocol_id
            ));
        }
        blockers.extend(evidence.blockers.iter().cloned());

        rows.push(ProtocolBindingState {
            protocol_id: seed.protocol_id.to_string(),
            source_path: seed.source_path.to_string(),
            activation_class: seed.activation_class.to_string(),
            runtime_owner: seed.runtime_owner.to_string(),
            enforcement_type: seed.enforcement_type.to_string(),
            proof_surface: seed.proof_surface.to_string(),
            primary_state_authority: TASKFLOW_PROTOCOL_BINDING_AUTHORITY.to_string(),
            binding_status: if blockers.is_empty() {
                "fully-runtime-bound".to_string()
            } else {
                "unbound".to_string()
            },
            active: true,
            blockers,
            scenario: TASKFLOW_PROTOCOL_BINDING_SCENARIO.to_string(),
            synced_at: String::new(),
        });
    }
    Ok(rows)
}

fn protocol_binding_check_ok(
    summary: &state_store::ProtocolBindingSummary,
    rows: &[ProtocolBindingState],
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> bool {
    evidence.imported
        && evidence.trusted
        && summary.total_receipts > 0
        && summary.total_bindings == taskflow_protocol_binding_seeds().len()
        && summary.unbound_count == 0
        && summary.blocking_issue_count == 0
        && summary.script_bound_count == 0
        && summary.fully_runtime_bound_count == taskflow_protocol_binding_seeds().len()
        && rows.len() == taskflow_protocol_binding_seeds().len()
        && rows
            .iter()
            .all(|row| row.binding_status == "fully-runtime-bound" && row.blockers.is_empty())
}

async fn run_taskflow_protocol_binding(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "protocol-binding" => {
            print_taskflow_proxy_help(Some("protocol-binding"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "protocol-binding" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("protocol-binding"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "sync" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let rows = match build_taskflow_protocol_binding_rows(&evidence) {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    match store
                        .record_protocol_binding_snapshot(
                            TASKFLOW_PROTOCOL_BINDING_SCENARIO,
                            TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
                            &rows,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            print_surface_header(
                                RenderMode::Plain,
                                "vida taskflow protocol-binding sync",
                            );
                            print_surface_line(RenderMode::Plain, "receipt", &receipt.receipt_id);
                            print_surface_line(RenderMode::Plain, "scenario", &receipt.scenario);
                            print_surface_line(
                                RenderMode::Plain,
                                "authority",
                                &receipt.primary_state_authority,
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "bindings",
                                &receipt.total_bindings.to_string(),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "blocking issues",
                                &receipt.blocking_issue_count.to_string(),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "compiled payload import",
                                if evidence.trusted {
                                    "trusted"
                                } else {
                                    "blocked"
                                },
                            );
                            if receipt.unbound_count == 0
                                && receipt.blocking_issue_count == 0
                                && evidence.trusted
                            {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to record protocol-binding state: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "sync" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let rows = match build_taskflow_protocol_binding_rows(&evidence) {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    match store
                        .record_protocol_binding_snapshot(
                            TASKFLOW_PROTOCOL_BINDING_SCENARIO,
                            TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
                            &rows,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "surface": "vida taskflow protocol-binding sync",
                                    "compiled_payload_import_evidence": evidence,
                                    "receipt": receipt,
                                    "bindings": rows,
                                }))
                                .expect("protocol-binding sync should render as json")
                            );
                            if rows.iter().all(|row| row.blockers.is_empty()) && evidence.trusted {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to record protocol-binding state: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    print_surface_header(
                        RenderMode::Plain,
                        "vida taskflow protocol-binding status",
                    );
                    print_surface_line(RenderMode::Plain, "summary", &summary.as_display());
                    print_surface_line(
                        RenderMode::Plain,
                        "compiled payload import",
                        if evidence.trusted {
                            "trusted"
                        } else {
                            "blocked"
                        },
                    );
                    for row in rows {
                        print_surface_line(RenderMode::Plain, "binding", &row.as_display());
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow protocol-binding status",
                            "compiled_payload_import_evidence": evidence,
                            "summary": summary,
                            "bindings": rows,
                        }))
                        .expect("protocol-binding status should render as json")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "check" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let ok = protocol_binding_check_ok(&summary, &rows, &evidence);
                    print_surface_header(RenderMode::Plain, "vida taskflow protocol-binding check");
                    print_surface_line(RenderMode::Plain, "ok", if ok { "true" } else { "false" });
                    print_surface_line(RenderMode::Plain, "summary", &summary.as_display());
                    print_surface_line(
                        RenderMode::Plain,
                        "compiled payload import",
                        if evidence.trusted {
                            "trusted"
                        } else {
                            "blocked"
                        },
                    );
                    if ok {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "check" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let ok = protocol_binding_check_ok(&summary, &rows, &evidence);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow protocol-binding check",
                            "ok": ok,
                            "compiled_payload_import_evidence": evidence,
                            "summary": summary,
                            "bindings": rows,
                        }))
                        .expect("protocol-binding check should render as json")
                    );
                    if ok {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "sync" => {
            eprintln!("Usage: vida taskflow protocol-binding sync [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow protocol-binding status [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "check" => {
            eprintln!("Usage: vida taskflow protocol-binding check [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

async fn run_taskflow_proxy(args: ProxyArgs) -> ExitCode {
    if matches!(args.args.first().map(String::as_str), Some("query")) {
        return run_taskflow_query(&args.args);
    }

    if let Some(topic) = taskflow_help_topic(&args.args) {
        print_taskflow_proxy_help(topic);
        return ExitCode::SUCCESS;
    }

    if matches!(args.args.first().map(String::as_str), Some("recovery")) {
        return run_taskflow_recovery(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("task")) {
        return match resolve_repo_root() {
            Ok(root) => match run_taskflow_task_bridge(&root, &args.args) {
                Ok(code) => code,
                Err(error) if error == "unsupported_taskflow_task_bridge" => {
                    eprintln!(
                        "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to the external TaskFlow runtime."
                    );
                    ExitCode::from(2)
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        };
    }

    if matches!(args.args.first().map(String::as_str), Some("doctor")) {
        return route_taskflow_doctor(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("bootstrap-spec")
    ) {
        return run_taskflow_bootstrap_spec(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("protocol-binding")
    ) {
        return run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        if matches!(
            args.args.get(1).map(String::as_str),
            None | Some("bundle" | "final" | "continue" | "advance" | "--help" | "-h")
        ) {
            return run_taskflow_consume(&args.args).await;
        }
    }

    if matches!(args.args.first().map(String::as_str), Some("run-graph")) {
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("status" | "latest" | "--help" | "-h")
        ) {
            return run_taskflow_run_graph(&args.args).await;
        }
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("seed" | "advance" | "init" | "update")
        ) {
            return run_taskflow_run_graph_mutation(&args.args).await;
        }
    }

    let subcommand = args.args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unsupported `vida taskflow {subcommand}` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to the external TaskFlow runtime."
    );
    ExitCode::from(2)
}

async fn route_taskflow_doctor(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Doctor(doctor_args)) => run_doctor(doctor_args).await,
            _ => {
                eprintln!("Unsupported `vida taskflow doctor` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run_docflow_proxy(args: ProxyArgs) -> ExitCode {
    if proxy_requested_help(&args.args) {
        print_docflow_proxy_help();
        return ExitCode::SUCCESS;
    }

    let argv = std::iter::once("docflow".to_string())
        .chain(args.args.clone())
        .collect::<Vec<_>>();

    match DocflowCli::try_parse_from(argv.clone()) {
        Ok(_cli) => {
            let project_root = resolve_repo_root().unwrap_or_else(|_| repo_runtime_root());
            match run_docflow_cli_command(&project_root, &args.args) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn ensure_launcher_bootstrap(
    store: &StateStore,
    instruction_source_root: &Path,
    framework_memory_source_root: &Path,
) -> Result<(), String> {
    store
        .seed_framework_instruction_bundle()
        .await
        .map_err(|error| format!("Failed to seed framework instruction bundle: {error}"))?;
    store
        .source_tree_summary()
        .await
        .map_err(|error| format!("Failed to read source tree metadata: {error}"))?;
    store
        .ingest_instruction_source_tree(&normalize_root_arg(instruction_source_root))
        .await
        .map_err(|error| format!("Failed to ingest instruction source tree: {error}"))?;
    let compatibility = store
        .evaluate_boot_compatibility()
        .await
        .map_err(|error| format!("Failed to evaluate boot compatibility: {error}"))?;
    if compatibility.classification != "compatible" {
        return Err(format!(
            "Boot compatibility check failed: {}",
            compatibility.reasons.join(", ")
        ));
    }
    let migration = store
        .evaluate_migration_preflight()
        .await
        .map_err(|error| format!("Failed to evaluate migration preflight: {error}"))?;
    if !migration.blockers.is_empty() {
        return Err(format!(
            "Migration preflight failed: {}",
            migration.blockers.join(", ")
        ));
    }
    let root_artifact_id = store
        .active_instruction_root()
        .await
        .map_err(|error| format!("Failed to read active instruction root: {error}"))?;
    store
        .resolve_effective_instruction_bundle(&root_artifact_id)
        .await
        .map_err(|error| format!("Failed to resolve effective instruction bundle: {error}"))?;
    store
        .ingest_framework_memory_source_tree(&normalize_root_arg(framework_memory_source_root))
        .await
        .map_err(|error| format!("Failed to ingest framework memory source tree: {error}"))?;
    sync_launcher_activation_snapshot(store)
        .await
        .map_err(|error| format!("Failed to persist launcher activation snapshot: {error}"))?;
    let evidence = protocol_binding_compiled_payload_import_evidence(store).await;
    let rows = build_taskflow_protocol_binding_rows(&evidence)?;
    store
        .record_protocol_binding_snapshot(
            TASKFLOW_PROTOCOL_BINDING_SCENARIO,
            TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
            &rows,
        )
        .await
        .map_err(|error| format!("Failed to record protocol-binding snapshot: {error}"))?;
    Ok(())
}

async fn run_orchestrator_init(args: InitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);

    match StateStore::open(state_dir).await {
        Ok(store) => {
            if let Err(error) = ensure_launcher_bootstrap(
                &store,
                &instruction_source_root,
                &framework_memory_source_root,
            )
            .await
            {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            match build_taskflow_consume_bundle_payload(&store).await {
                Ok(bundle) => {
                    let project_activation_view = match std::env::current_dir() {
                        Ok(path) => build_project_activator_view(&path),
                        Err(error) => {
                            eprintln!("Failed to resolve current directory: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let init_view = merge_project_activation_into_init_view(
                        bundle.orchestrator_init_view,
                        &project_activation_view,
                    );
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida orchestrator-init",
                                "init": init_view,
                                "runtime_bundle_summary": {
                                    "bundle_id": bundle.metadata["bundle_id"],
                                    "root_artifact_id": bundle.control_core["root_artifact_id"],
                                    "activation_source": bundle.activation_source,
                                    "vida_root": bundle.vida_root,
                                    "state_dir": store.root().display().to_string(),
                                },
                            }))
                            .expect("orchestrator-init json should render")
                        );
                    } else {
                        print_surface_header(RenderMode::Plain, "vida orchestrator-init");
                        print_surface_line(
                            RenderMode::Plain,
                            "status",
                            init_view["status"].as_str().unwrap_or("unknown"),
                        );
                        print_surface_line(RenderMode::Plain, "boot surface", "vida boot");
                        print_surface_line(
                            RenderMode::Plain,
                            "bundle id",
                            bundle.metadata["bundle_id"].as_str().unwrap_or(""),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "state dir",
                            &store.root().display().to_string(),
                        );
                        if init_view["project_activation"]["activation_pending"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "next step",
                                "vida project-activator --json",
                            );
                            if let Some(example) = init_view["project_activation"]["interview"]
                                ["one_shot_example"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "activation example",
                                    example,
                                );
                            }
                            print_surface_line(
                                RenderMode::Plain,
                                "activation runtime",
                                "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                            );
                        } else if init_view["project_activation"]["normal_work_defaults"]
                            ["documentation_first_for_feature_requests"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "feature flow",
                                "for requests that combine research/specification/planning and implementation, start with one bounded design document before code execution",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "feature intake",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["intake_runtime"]
                                    .as_str()
                                    .unwrap_or("vida taskflow consume final <request> --json"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "design template",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["local_feature_design_template"]
                                    .as_str()
                                    .unwrap_or("docs/product/spec/templates/feature-design-document.template.md"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "documentation runtime",
                                "open one feature epic and one spec-pack task in `vida taskflow`, then use `vida docflow` to initialize, finalize, and validate the design document before shaping the execution packet",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "execution posture",
                                "after the bounded design document is ready, delegate normal write-producing work through the configured development team instead of collapsing directly into root-session coding",
                            );
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_agent_init(args: AgentInitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);

    match StateStore::open(state_dir).await {
        Ok(store) => {
            if let Err(error) = ensure_launcher_bootstrap(
                &store,
                &instruction_source_root,
                &framework_memory_source_root,
            )
            .await
            {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            let bundle = match build_taskflow_consume_bundle_payload(&store).await {
                Ok(bundle) => bundle,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            let selection = if let Some(role) = args.role.clone() {
                let compiled_bundle = &bundle.activation_bundle;
                if !role_exists_in_lane_bundle(compiled_bundle, &role) || role == "orchestrator" {
                    eprintln!(
                        "Agent init requires a non-orchestrator lane role present in the compiled activation bundle."
                    );
                    return ExitCode::from(2);
                }
                serde_json::json!({
                    "mode": "explicit_role",
                    "selected_role": role,
                    "request_text": args.request_text.clone().unwrap_or_default(),
                })
            } else {
                let request = match args.request_text.as_deref() {
                    Some(request) if !request.trim().is_empty() => request,
                    _ => {
                        eprintln!(
                            "Agent init requires either a non-orchestrator `--role` or a bounded request text."
                        );
                        return ExitCode::from(2);
                    }
                };
                match build_runtime_lane_selection_with_store(&store, request).await {
                    Ok(selection) => {
                        if selection.selected_role == "orchestrator" {
                            eprintln!(
                                "Agent init resolved to orchestrator posture; provide a non-orchestrator `--role` or a bounded worker request."
                            );
                            return ExitCode::from(2);
                        }
                        serde_json::to_value(selection).expect("lane selection should serialize")
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };

            let project_activation_view = match std::env::current_dir() {
                Ok(path) => build_project_activator_view(&path),
                Err(error) => {
                    eprintln!("Failed to resolve current directory: {error}");
                    return ExitCode::from(1);
                }
            };
            let init_view = merge_project_activation_into_init_view(
                bundle.agent_init_view,
                &project_activation_view,
            );

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": "vida agent-init",
                        "init": init_view,
                        "selection": selection,
                        "runtime_bundle_summary": {
                            "bundle_id": bundle.metadata["bundle_id"],
                            "activation_source": bundle.activation_source,
                            "vida_root": bundle.vida_root,
                            "state_dir": store.root().display().to_string(),
                        },
                    }))
                    .expect("agent-init json should render")
                );
            } else {
                print_surface_header(RenderMode::Plain, "vida agent-init");
                print_surface_line(
                    RenderMode::Plain,
                    "status",
                    init_view["status"].as_str().unwrap_or("unknown"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "selected role",
                    selection["selected_role"].as_str().unwrap_or("unknown"),
                );
                if let Some(fallback_surface) = init_view["source_mode_fallback_surface"].as_str() {
                    print_surface_line(RenderMode::Plain, "fallback surface", fallback_surface);
                }
                if init_view["project_activation"]["activation_pending"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    print_surface_line(
                        RenderMode::Plain,
                        "next step",
                        "vida project-activator --json",
                    );
                    if let Some(example) =
                        init_view["project_activation"]["interview"]["one_shot_example"].as_str()
                    {
                        print_surface_line(RenderMode::Plain, "activation example", example);
                    }
                    print_surface_line(
                        RenderMode::Plain,
                        "activation runtime",
                        "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_boot(args: BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida boot` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let render = args.render;
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = args
        .instruction_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT));
    let framework_memory_source_root = args
        .framework_memory_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT));

    match StateStore::open(state_dir).await {
        Ok(store) => match store.seed_framework_instruction_bundle().await {
            Ok(()) => match store.backend_summary().await {
                Ok(summary) => match store.source_tree_summary().await {
                    Ok(source_tree) => match store
                        .ingest_instruction_source_tree(&normalize_root_arg(
                            &instruction_source_root,
                        ))
                        .await
                    {
                        Ok(ingest) => {
                            print_surface_header(render, "vida boot scaffold ready");
                            print_surface_line(render, "authoritative state store", &summary);
                            match store.state_spine_summary().await {
                                Ok(state_spine) => print_surface_line(
                                    render,
                                    "authoritative state spine",
                                    &format!(
                                "initialized (state-v{}, {} entity surfaces, mutation root {})",
                                state_spine.state_schema_version,
                                state_spine.entity_surface_count,
                                state_spine.authoritative_mutation_root
                            ),
                                ),
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read authoritative state spine summary: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            print_surface_line(render, "framework instruction bundle", "seeded");
                            print_surface_line(render, "instruction source tree", &source_tree);
                            print_surface_line(render, "instruction ingest", &ingest.as_display());
                            match store.evaluate_boot_compatibility().await {
                                Ok(compatibility) => {
                                    print_surface_line(
                                        render,
                                        "boot compatibility",
                                        &format!(
                                            "{} ({})",
                                            compatibility.classification, compatibility.next_step
                                        ),
                                    );
                                    if compatibility.classification != "compatible" {
                                        eprintln!(
                                            "Boot compatibility check failed: {}",
                                            compatibility.reasons.join(", ")
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to evaluate boot compatibility: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.evaluate_migration_preflight().await {
                                Ok(migration) => {
                                    print_surface_line(
                                        render,
                                        "migration preflight",
                                        &format!(
                                            "{} / {} ({})",
                                            migration.compatibility_classification,
                                            migration.migration_state,
                                            migration.next_step
                                        ),
                                    );
                                    if !migration.blockers.is_empty() {
                                        eprintln!(
                                            "Migration preflight failed: {}",
                                            migration.blockers.join(", ")
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to evaluate migration preflight: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.migration_receipt_summary().await {
                                Ok(summary) => {
                                    print_surface_line(
                                        render,
                                        "migration receipts",
                                        &summary.as_display(),
                                    );
                                }
                                Err(error) => {
                                    eprintln!("Failed to read migration receipt summary: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.active_instruction_root().await {
                                Ok(root_artifact_id) => match store
                                    .resolve_effective_instruction_bundle(&root_artifact_id)
                                    .await
                                {
                                    Ok(bundle) => {
                                        print_surface_line(
                                            render,
                                            "effective instruction bundle",
                                            &bundle.mandatory_chain_order.join(" -> "),
                                        );
                                        print_surface_line(
                                            render,
                                            "effective instruction bundle receipt",
                                            &bundle.receipt_id,
                                        );
                                    }
                                    Err(error) => {
                                        eprintln!("Failed to resolve effective instruction bundle: {error}");
                                        return ExitCode::from(1);
                                    }
                                },
                                Err(error) => {
                                    eprintln!("Failed to read active instruction root: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store
                                .ingest_framework_memory_source_tree(&normalize_root_arg(
                                    &framework_memory_source_root,
                                ))
                                .await
                            {
                                Ok(framework_ingest) => {
                                    if let Err(error) =
                                        sync_launcher_activation_snapshot(&store).await
                                    {
                                        eprintln!(
                                            "Failed to persist launcher activation snapshot: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                    print_surface_line(
                                        render,
                                        "framework memory ingest",
                                        &framework_ingest.as_display(),
                                    );
                                    print_surface_line(
                                        render,
                                        "state dir",
                                        &store.root().display().to_string(),
                                    );
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to ingest framework memory source tree: {error}"
                                    );
                                    ExitCode::from(1)
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to ingest instruction source tree: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to read source tree metadata: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to read storage metadata: {error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("Failed to seed framework instruction bundle: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_init(args: BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida init` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let project_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let source_root = resolve_init_bootstrap_source_root();
    let framework_agents = match resolve_init_agents_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let sidecar_scaffold = match resolve_init_sidecar_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let config_template = match resolve_init_config_template_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if !framework_agents.is_file() {
        eprintln!(
            "Missing framework bootstrap carrier: {}",
            framework_agents.display()
        );
        return ExitCode::from(1);
    }

    if let Err(error) = preserve_existing_agents_as_sidecar(&project_root)
        .and_then(|()| {
            copy_file_if_missing(&sidecar_scaffold, &project_root.join("AGENTS.sidecar.md"))
        })
        .and_then(|()| copy_file_if_missing(&framework_agents, &project_root.join("AGENTS.md")))
        .and_then(|()| {
            copy_file_if_missing(&config_template, &project_root.join("vida.config.yaml"))
        })
        .and_then(|()| ensure_runtime_home(&project_root))
        .and_then(|()| write_runtime_agent_extension_projections(&project_root))
    {
        eprintln!("{error}");
        return ExitCode::from(1);
    }

    let activation_view = build_project_activator_view(&project_root);
    print_init_summary(&project_root, &activation_view);
    ExitCode::SUCCESS
}

fn detect_project_shape(project_root: &Path) -> &'static str {
    let bootstrap_markers = [
        project_root.join("AGENTS.md"),
        project_root.join("AGENTS.sidecar.md"),
        project_root.join("vida.config.yaml"),
        project_root.join(".vida/config"),
        project_root.join(".vida/db"),
        project_root.join(".vida/cache"),
        project_root.join(".vida/framework"),
        project_root.join(".vida/project"),
        project_root.join(".vida/project/agent-extensions"),
        project_root.join(".vida/receipts"),
        project_root.join(".vida/runtime"),
        project_root.join(".vida/scratchpad"),
    ];
    if bootstrap_markers.iter().all(|path| path.exists()) {
        return "bootstrapped";
    }

    let project_markers = [
        project_root.join("docs"),
        project_root.join("src"),
        project_root.join("README.md"),
        project_root.join("Cargo.toml"),
        project_root.join("package.json"),
        project_root.join("pubspec.yaml"),
    ];
    if project_markers.iter().any(|path| path.exists()) {
        "partial"
    } else {
        "empty"
    }
}

fn file_contains_placeholder(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|contents| {
            let lowercase = contents.to_ascii_lowercase();
            lowercase.contains("placeholder")
                || lowercase.contains("project documentation: docs/")
                || contents.contains(PROJECT_ID_PLACEHOLDER)
                || contents.contains(DOCS_ROOT_PLACEHOLDER)
                || contents.contains(PROCESS_ROOT_PLACEHOLDER)
                || contents.contains(RESEARCH_ROOT_PLACEHOLDER)
                || contents.contains(README_DOC_PLACEHOLDER)
                || contents.contains(ARCHITECTURE_DOC_PLACEHOLDER)
                || contents.contains(DECISIONS_DOC_PLACEHOLDER)
                || contents.contains(ENVIRONMENTS_DOC_PLACEHOLDER)
                || contents.contains(PROJECT_OPERATIONS_DOC_PLACEHOLDER)
                || contents.contains(AGENT_SYSTEM_DOC_PLACEHOLDER)
                || contents.contains(USER_COMMUNICATION_PLACEHOLDER)
                || contents.contains(REASONING_LANGUAGE_PLACEHOLDER)
                || contents.contains(DOCUMENTATION_LANGUAGE_PLACEHOLDER)
                || contents.contains(TODO_PROTOCOL_LANGUAGE_PLACEHOLDER)
                || contents.contains(HOST_CLI_PLACEHOLDER)
                || contents.contains("<fill-your-project-name>")
                || contents.contains("<project-root-map-path>")
                || contents.contains("<product-index-path>")
                || contents.contains("<product-spec-map-path>")
                || contents.contains("<project-documentation-law-path>")
                || contents.contains("<documentation-tooling-map-path>")
                || contents.contains("<project-extension-map-path>")
        })
        .unwrap_or(false)
}

fn build_project_activator_view(project_root: &Path) -> serde_json::Value {
    let agents_md = project_root.join("AGENTS.md");
    let agents_sidecar = project_root.join("AGENTS.sidecar.md");
    let codex_tree = project_root.join(".codex");
    let codex_config = codex_tree.join("config.toml");
    let codex_agents = codex_tree.join("agents");
    let vida_config = project_root.join("vida.config.yaml");
    let vida_home = project_root.join(".vida");
    let vida_config_dir = project_root.join(".vida/config");
    let vida_db_dir = project_root.join(".vida/db");
    let vida_cache_dir = project_root.join(".vida/cache");
    let vida_framework_dir = project_root.join(".vida/framework");
    let vida_project_dir = project_root.join(".vida/project");
    let vida_receipts_dir = project_root.join(".vida/receipts");
    let vida_runtime_dir = project_root.join(".vida/runtime");
    let vida_scratchpad_dir = project_root.join(".vida/scratchpad");
    let project_root_map = project_root.join("docs/project-root-map.md");
    let product_index = project_root.join("docs/product/index.md");
    let product_spec_readme = project_root.join(DEFAULT_PROJECT_PRODUCT_SPEC_README);
    let feature_design_template = project_root.join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE);
    let process_readme = project_root.join("docs/process/README.md");
    let codex_agent_guide = project_root.join(DEFAULT_PROJECT_CODEX_GUIDE_DOC);
    let documentation_tooling_map = project_root.join(DEFAULT_PROJECT_DOC_TOOLING_DOC);
    let runtime_agent_extensions = runtime_agent_extensions_root(project_root);
    let runtime_agent_extensions_readme = runtime_agent_extensions.join("README.md");
    let runtime_agent_extension_roles = runtime_agent_extensions.join("roles.yaml");
    let runtime_agent_extension_skills = runtime_agent_extensions.join("skills.yaml");
    let runtime_agent_extension_profiles = runtime_agent_extensions.join("profiles.yaml");
    let runtime_agent_extension_flows = runtime_agent_extensions.join("flows.yaml");
    let runtime_agent_extension_role_sidecar = runtime_agent_extensions.join("roles.sidecar.yaml");
    let runtime_agent_extension_skill_sidecar =
        runtime_agent_extensions.join("skills.sidecar.yaml");
    let runtime_agent_extension_profile_sidecar =
        runtime_agent_extensions.join("profiles.sidecar.yaml");
    let runtime_agent_extension_flow_sidecar = runtime_agent_extensions.join("flows.sidecar.yaml");

    let sidecar_missing = !agents_sidecar.is_file();
    let sidecar_has_placeholders =
        agents_sidecar.is_file() && file_contains_placeholder(&agents_sidecar);
    let config_has_placeholders = vida_config.is_file() && file_contains_placeholder(&vida_config);
    let runtime_home_missing = [
        &vida_config_dir,
        &vida_db_dir,
        &vida_cache_dir,
        &vida_framework_dir,
        &vida_project_dir,
        &vida_receipts_dir,
        &vida_runtime_dir,
        &vida_scratchpad_dir,
    ]
    .iter()
    .any(|path| !path.is_dir());
    let bootstrap_missing = !agents_md.is_file() || !vida_config.is_file() || runtime_home_missing;
    let docs_missing = !project_root_map.is_file()
        || !product_index.is_file()
        || !product_spec_readme.is_file()
        || !feature_design_template.is_file()
        || !process_readme.is_file()
        || !codex_agent_guide.is_file()
        || !documentation_tooling_map.is_file();

    let project_overlay = if vida_config.is_file() {
        read_yaml_file_checked(&vida_config).ok()
    } else {
        None
    };
    let current_project_id = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["project", "id"])));
    let current_user_communication_language = project_overlay.as_ref().and_then(|config| {
        yaml_string(yaml_lookup(
            config,
            &["language_policy", "user_communication"],
        ))
    });
    let current_reasoning_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "reasoning"])));
    let current_documentation_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "documentation"])));
    let current_todo_protocol_language = project_overlay
        .as_ref()
        .and_then(|config| yaml_string(yaml_lookup(config, &["language_policy", "todo_protocol"])));
    let selected_host_cli_system = project_overlay
        .as_ref()
        .and_then(|config| yaml_lookup(config, &["host_environment", "cli_system"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != HOST_CLI_PLACEHOLDER)
        .map(|value| value.to_ascii_lowercase());
    let host_cli_supported = selected_host_cli_system
        .as_deref()
        .and_then(normalize_host_cli_system)
        .is_some();
    let host_cli_selection_required = selected_host_cli_system.is_none() || !host_cli_supported;
    let host_cli_template_materialized = matches!(
        selected_host_cli_system
            .as_deref()
            .and_then(normalize_host_cli_system),
        Some("codex")
    ) && codex_config.is_file()
        && codex_agents.is_dir();
    let host_cli_materialization_required =
        !host_cli_selection_required && !host_cli_template_materialized;
    let host_cli_template_source_root = selected_host_cli_system
        .as_deref()
        .and_then(normalize_host_cli_system)
        .and_then(|system| resolve_host_cli_template_source(system).ok())
        .or_else(|| resolve_host_cli_template_source("codex").ok());
    let default_host_agent_templates = host_cli_template_source_root
        .as_deref()
        .map(list_host_cli_agent_templates)
        .unwrap_or_default();
    let host_cli_agent_catalog = project_overlay
        .as_ref()
        .map(overlay_codex_agent_catalog)
        .filter(|rows| !rows.is_empty())
        .or_else(|| {
            host_cli_template_source_root
                .as_deref()
                .map(read_codex_agent_catalog)
                .filter(|rows| !rows.is_empty())
        })
        .unwrap_or_default();
    let default_agent_topology = host_cli_agent_catalog
        .iter()
        .filter_map(|row| row["role_id"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let mut codex_tier_rates = serde_json::Map::new();
    for row in &host_cli_agent_catalog {
        if let (Some(tier), Some(rate)) = (row["tier"].as_str(), row["rate"].as_u64()) {
            codex_tier_rates.insert(tier.to_string(), serde_json::Value::Number(rate.into()));
        }
    }
    let agent_extensions_enabled = project_overlay
        .as_ref()
        .map(|config| yaml_bool(yaml_lookup(config, &["agent_extensions", "enabled"]), false))
        .unwrap_or(false);
    let agent_extension_bundle = project_overlay
        .as_ref()
        .filter(|_| agent_extensions_enabled)
        .map(|config| build_compiled_agent_extension_bundle_for_root(config, project_root));
    let agent_extensions_ready = agent_extension_bundle
        .as_ref()
        .map(|result| result.is_ok())
        .unwrap_or(true);
    let agent_extension_validation_error = agent_extension_bundle
        .as_ref()
        .and_then(|result| result.as_ref().err())
        .cloned();

    let runtime_agent_extensions_missing = [
        &runtime_agent_extensions_readme,
        &runtime_agent_extension_roles,
        &runtime_agent_extension_skills,
        &runtime_agent_extension_profiles,
        &runtime_agent_extension_flows,
        &runtime_agent_extension_role_sidecar,
        &runtime_agent_extension_skill_sidecar,
        &runtime_agent_extension_profile_sidecar,
        &runtime_agent_extension_flow_sidecar,
    ]
    .iter()
    .any(|path| !path.exists());

    let sidecar_or_project_docs_too_thin =
        sidecar_missing || sidecar_has_placeholders || docs_missing;
    let execution_posture_ambiguous = bootstrap_missing
        || sidecar_missing
        || config_has_placeholders
        || host_cli_selection_required
        || host_cli_materialization_required
        || sidecar_has_placeholders
        || docs_missing
        || !agent_extensions_ready;
    let activation_pending = bootstrap_missing
        || sidecar_missing
        || config_has_placeholders
        || host_cli_selection_required
        || host_cli_materialization_required
        || sidecar_has_placeholders
        || docs_missing
        || (agent_extensions_enabled
            && (runtime_agent_extensions_missing || !agent_extensions_ready));
    let project_id_missing =
        is_missing_or_placeholder(current_project_id.as_deref(), PROJECT_ID_PLACEHOLDER);
    let user_communication_language_missing = is_missing_or_placeholder(
        current_user_communication_language.as_deref(),
        USER_COMMUNICATION_PLACEHOLDER,
    );
    let reasoning_language_missing = is_missing_or_placeholder(
        current_reasoning_language.as_deref(),
        REASONING_LANGUAGE_PLACEHOLDER,
    );
    let documentation_language_missing = is_missing_or_placeholder(
        current_documentation_language.as_deref(),
        DOCUMENTATION_LANGUAGE_PLACEHOLDER,
    );
    let todo_protocol_language_missing = is_missing_or_placeholder(
        current_todo_protocol_language.as_deref(),
        TODO_PROTOCOL_LANGUAGE_PLACEHOLDER,
    );
    let inferred_project_id = inferred_project_id_candidate(project_root);
    let mut required_inputs = Vec::new();
    if project_id_missing {
        required_inputs.push(serde_json::json!({
            "id": "project_id",
            "prompt": "What project id should VIDA record for this repository?",
            "flag": "--project-id",
            "suggested_value": inferred_project_id,
            "required": true
        }));
    }
    if user_communication_language_missing
        || reasoning_language_missing
        || documentation_language_missing
        || todo_protocol_language_missing
    {
        required_inputs.push(serde_json::json!({
            "id": "language",
            "prompt": "Which language should VIDA use by default for user communication, reasoning, documentation, and todo protocol?",
            "flag": "--language",
            "suggested_value": current_user_communication_language
                .clone()
                .filter(|value| !is_missing_or_placeholder(Some(value.as_str()), USER_COMMUNICATION_PLACEHOLDER))
                .unwrap_or_else(|| "english".to_string()),
            "required": true,
            "covers": [
                "language_policy.user_communication",
                "language_policy.reasoning",
                "language_policy.documentation",
                "language_policy.todo_protocol"
            ]
        }));
    }
    if host_cli_selection_required {
        required_inputs.push(serde_json::json!({
            "id": "host_cli_system",
            "prompt": "Which supported host CLI system should VIDA activate for agents in this project?",
            "flag": "--host-cli-system",
            "suggested_value": "codex",
            "supported_values": SUPPORTED_HOST_CLI_SYSTEMS,
            "required": true
        }));
    }
    let mut one_shot_example_parts = vec!["vida project-activator".to_string()];
    if project_id_missing {
        one_shot_example_parts.push(format!("--project-id {}", inferred_project_id));
    }
    if user_communication_language_missing
        || reasoning_language_missing
        || documentation_language_missing
        || todo_protocol_language_missing
    {
        one_shot_example_parts.push("--language english".to_string());
    }
    if host_cli_selection_required {
        one_shot_example_parts.push("--host-cli-system codex".to_string());
    }
    one_shot_example_parts.push("--json".to_string());
    let one_shot_example = one_shot_example_parts.join(" ");

    let mut next_steps: Vec<String> = Vec::new();
    if bootstrap_missing || sidecar_missing {
        next_steps.push(
            "run `vida init` in the project root to materialize bootstrap carriers".to_string(),
        );
    }
    if config_has_placeholders {
        next_steps.push(
            "run `vida project-activator` with the bounded activation interview inputs to record project identity, language policy, docs roots, and host CLI setup before normal work"
                .to_string(),
        );
    }
    if host_cli_selection_required {
        next_steps.push(
            "choose the host CLI system from the supported framework list (`codex`) and run the one-shot `vida project-activator` activation command; project activation is not complete until the host agent template is selected"
                .to_string(),
        );
    } else if host_cli_materialization_required {
        next_steps.push(
            "materialize the selected host CLI template with `vida project-activator --host-cli-system codex`, then close and restart Codex so agent configuration becomes visible to the runtime environment"
                .to_string(),
        );
    }
    if sidecar_has_placeholders {
        next_steps.push(
            "replace placeholder project-doc pointers in `AGENTS.sidecar.md` before normal project work"
                .to_string(),
        );
    }
    if docs_missing {
        next_steps.push(
            "materialize the minimum project-doc roots (`docs/project-root-map.md`, `docs/product/index.md`, `docs/process/README.md`, `docs/process/documentation-tooling-map.md`) or record an explicit activation override"
                .to_string(),
        );
    }
    if agent_extensions_enabled && runtime_agent_extensions_missing {
        next_steps.push(
            "repair `.vida/project/agent-extensions/**` with `vida init` so runtime-owned role/skill/profile/flow projections and sidecars exist".to_string(),
        );
    }
    if let Some(error) = agent_extension_validation_error.as_deref() {
        next_steps.push(format!(
            "resolve agent-extension validation drift under `.vida/project/agent-extensions/**`: {error}"
        ));
    }
    if next_steps.is_empty() {
        next_steps.push(
            "activation looks ready enough for normal orchestrator and worker initialization"
                .to_string(),
        );
    }

    serde_json::json!({
        "surface": "vida project-activator",
        "status": if activation_pending { "pending_activation" } else { "ready_enough_for_normal_work" },
        "activation_pending": activation_pending,
        "project_root": project_root.display().to_string(),
        "project_shape": detect_project_shape(project_root),
        "triggers": {
            "initial_onboarding_missing": bootstrap_missing || sidecar_missing,
            "config_state_incomplete": !vida_config.is_file() || config_has_placeholders,
            "sidecar_or_project_docs_too_thin": sidecar_or_project_docs_too_thin,
            "execution_posture_ambiguous": execution_posture_ambiguous,
            "host_cli_unselected_or_unmaterialized": host_cli_selection_required || host_cli_materialization_required,
            "agent_extensions_invalid": agent_extensions_enabled && !agent_extensions_ready,
        },
        "bootstrap_surfaces": {
            "agents_md": agents_md.is_file(),
            "agents_sidecar_md": agents_sidecar.is_file(),
            "vida_config_yaml": vida_config.is_file(),
            "vida_home": vida_home.is_dir(),
            "vida_config_dir": vida_config_dir.is_dir(),
            "vida_db_dir": vida_db_dir.is_dir(),
            "vida_cache_dir": vida_cache_dir.is_dir(),
            "vida_framework_dir": vida_framework_dir.is_dir(),
            "vida_project_dir": vida_project_dir.is_dir(),
            "vida_receipts_dir": vida_receipts_dir.is_dir(),
            "vida_runtime_dir": vida_runtime_dir.is_dir(),
            "vida_scratchpad_dir": vida_scratchpad_dir.is_dir(),
        },
        "project_docs": {
            "project_root_map": project_root_map.is_file(),
            "product_index": product_index.is_file(),
            "product_spec_readme": product_spec_readme.is_file(),
            "feature_design_template": feature_design_template.is_file(),
            "process_readme": process_readme.is_file(),
            "codex_agent_configuration_guide": codex_agent_guide.is_file(),
            "documentation_tooling_map": documentation_tooling_map.is_file(),
            "sidecar_has_placeholders": sidecar_has_placeholders,
            "config_has_placeholders": config_has_placeholders,
        },
        "agent_extensions": {
            "enabled": agent_extensions_enabled,
            "runtime_projection_root": runtime_agent_extensions.display().to_string(),
            "runtime_readme": runtime_agent_extensions_readme.is_file(),
            "roles_registry": runtime_agent_extension_roles.is_file(),
            "skills_registry": runtime_agent_extension_skills.is_file(),
            "profiles_registry": runtime_agent_extension_profiles.is_file(),
            "flows_registry": runtime_agent_extension_flows.is_file(),
            "roles_sidecar": runtime_agent_extension_role_sidecar.is_file(),
            "skills_sidecar": runtime_agent_extension_skill_sidecar.is_file(),
            "profiles_sidecar": runtime_agent_extension_profile_sidecar.is_file(),
            "flows_sidecar": runtime_agent_extension_flow_sidecar.is_file(),
            "bundle_ready": agent_extensions_ready,
            "validation_error": agent_extension_validation_error,
        },
        "host_environment": {
            "supported_cli_systems": SUPPORTED_HOST_CLI_SYSTEMS,
            "selected_cli_system": selected_host_cli_system,
            "selection_required": host_cli_selection_required,
            "template_materialized": host_cli_template_materialized,
            "materialization_required": host_cli_materialization_required,
            "runtime_template_root": codex_tree.display().to_string(),
            "template_source_root": host_cli_template_source_root.map(|path| path.display().to_string()),
            "default_host_agent_templates": default_host_agent_templates,
            "configuration_protocols": [
                "runtime-instructions/work.host-cli-agent-setup-protocol"
            ],
        },
        "activation_algorithm": {
            "mode": "bounded_interview_then_materialize",
            "taskflow_admitted_while_pending": false,
            "non_canonical_taskflow_surfaces_forbidden_while_pending": [
                "vida taskflow",
                "external_taskflow_runtime"
            ],
            "docflow_first": true,
            "docflow_surface": "vida docflow",
            "allowed_activation_surfaces": [
                "vida project-activator",
                "vida docflow",
                "vida protocol view bootstrap/router",
                "vida protocol view runtime-instructions/work.host-cli-agent-setup-protocol"
            ],
            "activation_receipt_glob": ".vida/receipts/project-activation*.json"
        },
        "normal_work_defaults": {
            "documentation_first_for_feature_requests": true,
            "intake_runtime": "vida taskflow consume final <request> --json",
            "local_feature_design_template": DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            "local_product_spec_guide": DEFAULT_PROJECT_PRODUCT_SPEC_README,
            "local_documentation_tooling_map": DEFAULT_PROJECT_DOC_TOOLING_DOC,
            "local_codex_guide": DEFAULT_PROJECT_CODEX_GUIDE_DOC,
            "default_agent_topology": default_agent_topology,
            "codex_tier_rates": codex_tier_rates,
            "local_agent_score_state": {
                "strategy_store": CODEX_WORKER_STRATEGY_STATE,
                "scorecards_store": CODEX_WORKER_SCORECARDS_STATE
            },
            "recommended_flow": [
                "create or update one bounded design document before code execution when the request asks for research/specification/planning and implementation together",
                "open one feature epic and one spec-pack task in vida taskflow before delegated implementation begins",
                "use vida docflow to initialize, finalize, and validate the design document",
                "close the spec-pack task and shape the execution packet from the bounded file set and proof targets recorded in the design document",
                "delegate normal write-producing work through the default Codex tier ladder and let runtime pick the cheapest capable tier with a healthy local score instead of collapsing directly into root-session coding"
            ]
        },
        "interview": {
            "required_inputs": required_inputs,
            "safe_defaults": {
                "project_bootstrap.docs_root": DEFAULT_PROJECT_DOCS_ROOT,
                "project_bootstrap.process_root": DEFAULT_PROJECT_PROCESS_ROOT,
                "project_bootstrap.research_root": DEFAULT_PROJECT_RESEARCH_ROOT,
                "project_bootstrap.readme_doc": "README.md",
                "project_bootstrap.architecture_doc": DEFAULT_PROJECT_ARCHITECTURE_DOC,
                "project_bootstrap.decisions_doc": DEFAULT_PROJECT_DECISIONS_DOC,
                "project_bootstrap.environments_doc": DEFAULT_PROJECT_ENVIRONMENTS_DOC,
                "project_bootstrap.project_operations_doc": DEFAULT_PROJECT_OPERATIONS_DOC,
                "project_bootstrap.agent_system_doc": DEFAULT_PROJECT_AGENT_SYSTEM_DOC,
                "project_docs.documentation_tooling_doc": DEFAULT_PROJECT_DOC_TOOLING_DOC
            },
            "one_shot_example": one_shot_example
        },
        "current_activation_state": {
            "project_id": current_project_id,
            "user_communication_language": current_user_communication_language,
            "reasoning_language": current_reasoning_language,
            "documentation_language": current_documentation_language,
            "todo_protocol_language": current_todo_protocol_language
        },
        "next_steps": next_steps,
        "bounded_scope_note": "This runtime surface reports activation posture, required interview inputs, and bounded onboarding next steps. While activation remains pending it is a doc/config onboarding path, not tracked TaskFlow execution.",
    })
}

fn merge_project_activation_into_init_view(
    mut init_view: serde_json::Value,
    project_activation_view: &serde_json::Value,
) -> serde_json::Value {
    let activation_pending = project_activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true);
    if activation_pending {
        init_view["status"] = serde_json::Value::String("pending_activation".to_string());
        let mut minimum_commands = vec![
            serde_json::Value::String("vida project-activator --json".to_string()),
            serde_json::Value::String("vida docflow check --profile active-canon".to_string()),
        ];
        if let Some(example) = project_activation_view["interview"]["one_shot_example"].as_str() {
            minimum_commands.insert(0, serde_json::Value::String(example.to_string()));
        }
        init_view["minimum_commands"] = serde_json::Value::Array(minimum_commands);
        init_view["execution_gate"] = serde_json::json!({
            "activation_pending": true,
            "taskflow_admitted": false,
            "non_canonical_taskflow_surfaces_forbidden": ["vida taskflow", "external_taskflow_runtime"],
            "docflow_first": true
        });
        if init_view.get("source_mode_fallback_surface").is_some() {
            init_view["source_mode_fallback_surface"] =
                serde_json::Value::String("blocked_during_pending_activation".to_string());
        }
    }
    init_view["project_activation"] = serde_json::json!({
        "status": project_activation_view["status"],
        "activation_pending": project_activation_view["activation_pending"],
        "project_shape": project_activation_view["project_shape"],
        "triggers": project_activation_view["triggers"],
        "activation_algorithm": project_activation_view["activation_algorithm"],
        "normal_work_defaults": project_activation_view["normal_work_defaults"],
        "interview": project_activation_view["interview"],
        "host_environment": project_activation_view["host_environment"],
        "next_steps": project_activation_view["next_steps"],
    });
    init_view
}

async fn run_project_activator(args: ProjectActivatorArgs) -> ExitCode {
    let _ = args.state_dir;
    let project_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let mut host_cli_activated = None;
    let mut changed_files = Vec::new();
    let activation_answers = resolve_project_activation_answers(&project_root, &args);

    if let Some(requested_host_cli_system) = args.host_cli_system.as_deref() {
        let Some(normalized_host_cli_system) = normalize_host_cli_system(requested_host_cli_system)
        else {
            eprintln!(
                "Unsupported host CLI system `{requested_host_cli_system}`. Supported values: {}",
                SUPPORTED_HOST_CLI_SYSTEMS.join(", ")
            );
            return ExitCode::from(2);
        };
        if let Err(error) = apply_host_cli_selection(&project_root, normalized_host_cli_system)
            .and_then(|()| materialize_host_cli_template(&project_root, normalized_host_cli_system))
        {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
        host_cli_activated = Some(normalized_host_cli_system.to_string());
        changed_files.push("vida.config.yaml".to_string());
        changed_files.push(".codex/**".to_string());
    }

    if let Some(answers) = activation_answers.as_ref() {
        match apply_project_activation_answers(&project_root, answers) {
            Ok(mut files) => changed_files.append(&mut files),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }

    changed_files.sort();
    changed_files.dedup();
    let host_template_materialized = host_cli_activated.is_some();
    let activation_receipt_path = match write_project_activation_receipt(
        &project_root,
        activation_answers.as_ref(),
        host_cli_activated.as_deref(),
        &changed_files,
        host_template_materialized,
    ) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    let mut view = build_project_activator_view(&project_root);
    if let Some(path) = activation_receipt_path.as_deref() {
        view["activation_log"] = serde_json::json!({
            "receipt_path": path,
            "changed_files": changed_files,
        });
    }
    if args.json {
        let payload = if let Some(host_cli_activated) = host_cli_activated.as_deref() {
            serde_json::json!({
                "surface": "vida project-activator",
                "post_init_restart_required": true,
                "post_init_restart_note": format!(
                    "close and restart {} so the newly activated agent template becomes visible to the runtime execution environment",
                    if host_cli_activated == "codex" { "Codex" } else { host_cli_activated }
                ),
                "activation_log": view["activation_log"],
                "view": view,
            })
        } else if activation_receipt_path.is_some() {
            serde_json::json!({
                "surface": "vida project-activator",
                "activation_log": view["activation_log"],
                "view": view,
            })
        } else {
            view
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("project activator view should render")
        );
        return ExitCode::SUCCESS;
    }

    print_surface_header(RenderMode::Plain, "vida project-activator");
    print_surface_line(
        RenderMode::Plain,
        "status",
        view["status"].as_str().unwrap_or("unknown"),
    );
    print_surface_line(
        RenderMode::Plain,
        "project_root",
        view["project_root"].as_str().unwrap_or("unknown"),
    );
    print_surface_line(
        RenderMode::Plain,
        "project_shape",
        view["project_shape"].as_str().unwrap_or("unknown"),
    );
    print_surface_line(
        RenderMode::Plain,
        "activation_pending",
        if view["activation_pending"].as_bool().unwrap_or(true) {
            "true"
        } else {
            "false"
        },
    );
    print_surface_line(
        RenderMode::Plain,
        "host_cli_system",
        view["host_environment"]["selected_cli_system"]
            .as_str()
            .unwrap_or("unselected"),
    );
    print_surface_line(
        RenderMode::Plain,
        "taskflow_admitted_while_pending",
        if view["activation_algorithm"]["taskflow_admitted_while_pending"]
            .as_bool()
            .unwrap_or(false)
        {
            "true"
        } else {
            "false"
        },
    );
    println!("required_inputs");
    if let Some(inputs) = view["interview"]["required_inputs"].as_array() {
        if inputs.is_empty() {
            println!("  - none");
        } else {
            for input in inputs {
                let prompt = input["prompt"].as_str().unwrap_or("unspecified");
                let flag = input["flag"].as_str().unwrap_or("--unknown");
                let suggested_value = input["suggested_value"].as_str().unwrap_or("n/a");
                println!("  - {prompt} ({flag}, suggested: {suggested_value})");
            }
        }
    }
    print_surface_line(
        RenderMode::Plain,
        "one_shot_example",
        view["interview"]["one_shot_example"]
            .as_str()
            .unwrap_or("vida project-activator --json"),
    );
    println!("next_steps");
    if let Some(steps) = view["next_steps"].as_array() {
        for step in steps {
            if let Some(step) = step.as_str() {
                println!("  - {step}");
            }
        }
    }
    if let Some(path) = activation_receipt_path.as_deref() {
        println!("  - activation log: {path}");
    }
    if host_cli_activated.is_some() {
        println!(
            "  - close and restart Codex so the newly activated agent template becomes visible to the runtime environment"
        );
    }
    ExitCode::SUCCESS
}

async fn run_protocol(args: ProtocolArgs) -> ExitCode {
    match args.command {
        ProtocolCommand::View(view) => {
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

async fn run_memory(args: MemoryArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;

    match StateStore::open_existing(state_dir).await {
        Ok(store) => match store.active_instruction_root().await {
            Ok(root_artifact_id) => match store
                .inspect_effective_instruction_bundle(&root_artifact_id)
                .await
            {
                Ok(bundle) => {
                    print_surface_header(render, "vida memory");
                    print_surface_line(
                        render,
                        "effective instruction bundle root",
                        &bundle.root_artifact_id,
                    );
                    print_surface_line(
                        render,
                        "mandatory chain",
                        &bundle.mandatory_chain_order.join(" -> "),
                    );
                    print_surface_line(
                        render,
                        "source version tuple",
                        &bundle.source_version_tuple.join(", "),
                    );
                    print_surface_line(render, "receipt", &bundle.receipt_id);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to resolve effective instruction bundle: {error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("Failed to read active instruction root: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_agent_feedback(args: AgentFeedbackArgs) -> ExitCode {
    let project_root = match resolve_runtime_project_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let outcome = args.outcome.as_deref().unwrap_or("success");
    let task_class = args.task_class.as_deref().unwrap_or("unspecified");
    let input = HostAgentFeedbackInput {
        agent_id: &args.agent_id,
        score: args.score,
        outcome,
        task_class,
        notes: args.notes.as_deref(),
        source: "vida agent-feedback",
        task_id: None,
        task_display_id: None,
        task_title: None,
        runtime_role: None,
        selected_tier: Some(&args.agent_id),
        estimated_task_price_units: None,
        lifecycle_state: None,
        effective_score: None,
        reason: None,
    };
    match append_host_agent_feedback(&project_root, &input) {
        Ok(view) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&view).expect("agent feedback json should render")
                );
            } else {
                print_surface_header(RenderMode::Plain, "vida agent-feedback");
                println!(
                    "host cli system: {}",
                    view["host_cli_system"].as_str().unwrap_or("")
                );
                println!("agent id: {}", view["agent_id"].as_str().unwrap_or(""));
                println!(
                    "recorded score: {}",
                    view["recorded_score"].as_u64().unwrap_or_default()
                );
                println!(
                    "outcome: {}",
                    view["recorded_outcome"].as_str().unwrap_or("")
                );
                println!(
                    "task class: {}",
                    view["recorded_task_class"].as_str().unwrap_or("")
                );
                if let Some(notes) = view["recorded_notes"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                {
                    println!("notes: {notes}");
                }
                println!(
                    "effective score: {}",
                    view["strategy_row"]["effective_score"]
                        .as_u64()
                        .unwrap_or_default()
                );
                println!(
                    "lifecycle state: {}",
                    view["strategy_row"]["lifecycle_state"]
                        .as_str()
                        .unwrap_or("")
                );
                println!(
                    "scorecards store: {}",
                    view["scorecards_store"].as_str().unwrap_or("")
                );
                println!(
                    "strategy store: {}",
                    view["strategy_store"].as_str().unwrap_or("")
                );
                println!(
                    "observability store: {}",
                    view["observability_store"].as_str().unwrap_or("")
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::ImportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
                    Ok(summary) => {
                        if command.json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "status": "ok",
                                    "source_path": summary.source_path,
                                    "imported_count": summary.imported_count,
                                    "unchanged_count": summary.unchanged_count,
                                    "updated_count": summary.updated_count,
                                }))
                                .expect("json import summary should render")
                            );
                        } else {
                            print_surface_header(command.render, "vida task import-jsonl");
                            print_surface_line(command.render, "import", &summary.as_display());
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to import tasks from JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::List(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .list_tasks(command.status.as_deref(), command.all)
                    .await
                {
                    Ok(tasks) => {
                        print_task_list(command.render, &tasks, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to list tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.show_task(&command.task_id).await {
                    Ok(task) => {
                        print_task_show(command.render, &task, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to show task: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Ready(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.ready_tasks_scoped(command.scope.as_deref()).await {
                    Ok(tasks) => {
                        print_task_list(command.render, &tasks, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute ready tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Deps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.task_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReverseDeps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.reverse_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task reverse-deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read reverse dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Blocked(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.blocked_tasks().await {
                    Ok(tasks) => {
                        print_blocked_tasks(command.render, &tasks, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute blocked tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.task_dependency_tree(&command.task_id).await {
                    Ok(tree) => {
                        print_task_dependency_tree(command.render, &tree, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependency tree: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ValidateGraph(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.validate_task_graph().await {
                    Ok(issues) => {
                        print_task_graph_issues(command.render, &issues, command.json);
                        if issues.is_empty() {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to validate task graph: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Dep(command) => match command.command {
            TaskDependencyCommand::Add(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependency(
                            &add.task_id,
                            &add.depends_on_id,
                            &add.edge_type,
                            &add.created_by,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            print_task_dependency_mutation(
                                add.render,
                                "vida task dep add",
                                &dependency,
                                add.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Remove(remove) => {
                let state_dir = remove
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .remove_task_dependency(
                            &remove.task_id,
                            &remove.depends_on_id,
                            &remove.edge_type,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            print_task_dependency_mutation(
                                remove.render,
                                "vida task dep remove",
                                &dependency,
                                remove.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to remove task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::CriticalPath(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.critical_path().await {
                    Ok(path) => {
                        print_task_critical_path(command.render, &path, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute critical path: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

async fn run_status(args: StatusArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;

    match StateStore::open_existing(state_dir).await {
        Ok(store) => match store.storage_metadata_summary().await {
            Ok(storage_metadata) => {
                let backend_summary = format!(
                    "{} state-v{} instruction-v{}",
                    storage_metadata.backend,
                    storage_metadata.state_schema_version,
                    storage_metadata.instruction_schema_version
                );
                let state_spine = match store.state_spine_summary().await {
                    Ok(state_spine) => state_spine,
                    Err(error) => {
                        eprintln!("Failed to read authoritative state spine summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let effective_bundle_receipt =
                    match store.latest_effective_bundle_receipt_summary().await {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!("Failed to read effective bundle receipt summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let boot_compatibility = match store.latest_boot_compatibility_summary().await {
                    Ok(compatibility) => compatibility,
                    Err(error) => {
                        eprintln!("Failed to read boot compatibility summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let migration_state = match store.latest_migration_preflight_summary().await {
                    Ok(migration) => migration,
                    Err(error) => {
                        eprintln!("Failed to read migration preflight summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let migration_receipts = match store.migration_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read migration receipt summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_task_reconciliation =
                    match store.latest_task_reconciliation_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read task reconciliation summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let task_reconciliation_rollup = match store.task_reconciliation_rollup().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read task reconciliation rollup: {error}");
                        return ExitCode::from(1);
                    }
                };
                let snapshot_bridge = match store.taskflow_snapshot_bridge_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read taskflow snapshot bridge summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let runtime_consumption = match runtime_consumption_summary(store.root()) {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read runtime-consumption summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let protocol_binding = match store.protocol_binding_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read protocol-binding summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_status = match store.latest_run_graph_status().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph status: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_recovery =
                    match store.latest_run_graph_recovery_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph recovery summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_checkpoint = match store
                    .latest_run_graph_checkpoint_summary()
                    .await
                {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph checkpoint summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_gate = match store.latest_run_graph_gate_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("Failed to read latest run graph gate summary: {error}");
                        return ExitCode::from(1);
                    }
                };
                let latest_run_graph_dispatch_receipt =
                    match store.latest_run_graph_dispatch_receipt_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!(
                                "Failed to read latest run graph dispatch receipt summary: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                let host_agents = infer_project_root_from_state_root(store.root())
                    .or_else(|| resolve_runtime_project_root().ok())
                    .and_then(|project_root| build_host_agent_status_summary(&project_root));

                if as_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida status",
                            "state_dir": store.root().display().to_string(),
                            "storage_metadata": {
                                "engine": storage_metadata.engine,
                                "backend": storage_metadata.backend,
                                "namespace": storage_metadata.namespace,
                                "database": storage_metadata.database,
                                "state_schema_version": storage_metadata.state_schema_version,
                                "instruction_schema_version": storage_metadata.instruction_schema_version,
                            },
                            "backend_summary": backend_summary,
                            "state_spine": {
                                "state_schema_version": state_spine.state_schema_version,
                                "entity_surface_count": state_spine.entity_surface_count,
                                "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                            },
                            "latest_effective_bundle_receipt": effective_bundle_receipt,
                            "boot_compatibility": boot_compatibility.as_ref().map(|compatibility| serde_json::json!({
                                "classification": compatibility.classification,
                                "reasons": compatibility.reasons,
                                "next_step": compatibility.next_step,
                            })),
                            "migration_state": migration_state.as_ref().map(|migration| serde_json::json!({
                                "compatibility_classification": migration.compatibility_classification,
                                "migration_state": migration.migration_state,
                                "blockers": migration.blockers,
                                "source_version_tuple": migration.source_version_tuple,
                                "next_step": migration.next_step,
                            })),
                            "migration_receipts": {
                                "compatibility_receipts": migration_receipts.compatibility_receipts,
                                "application_receipts": migration_receipts.application_receipts,
                                "verification_receipts": migration_receipts.verification_receipts,
                                "cutover_readiness_receipts": migration_receipts.cutover_readiness_receipts,
                                "rollback_notes": migration_receipts.rollback_notes,
                            },
                            "latest_task_reconciliation": latest_task_reconciliation,
                            "task_reconciliation_rollup": task_reconciliation_rollup,
                            "taskflow_snapshot_bridge": snapshot_bridge,
                            "runtime_consumption": runtime_consumption,
                            "protocol_binding": protocol_binding,
                            "host_agents": host_agents,
                            "latest_run_graph_status": latest_run_graph_status,
                            "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                            "latest_run_graph_recovery": latest_run_graph_recovery,
                            "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                            "latest_run_graph_gate": latest_run_graph_gate,
                            "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        }))
                        .expect("status summary should render as json")
                    );
                    return ExitCode::SUCCESS;
                }

                print_surface_header(render, "vida status");
                print_surface_line(render, "backend", &backend_summary);
                print_surface_line(render, "state dir", &store.root().display().to_string());
                print_surface_line(
                    render,
                    "state spine",
                    &format!(
                        "initialized (state-v{}, {} entity surfaces, mutation root {})",
                        state_spine.state_schema_version,
                        state_spine.entity_surface_count,
                        state_spine.authoritative_mutation_root
                    ),
                );
                match effective_bundle_receipt {
                    Some(receipt) => {
                        print_surface_line(
                            render,
                            "latest effective bundle receipt",
                            &receipt.receipt_id,
                        );
                        print_surface_line(
                            render,
                            "latest effective bundle root",
                            &receipt.root_artifact_id,
                        );
                        print_surface_line(
                            render,
                            "latest effective bundle artifact count",
                            &receipt.artifact_count.to_string(),
                        );
                    }
                    None => {
                        print_surface_line(render, "latest effective bundle receipt", "none");
                    }
                }
                match boot_compatibility {
                    Some(compatibility) => {
                        print_surface_line(
                            render,
                            "boot compatibility",
                            &format!(
                                "{} ({})",
                                compatibility.classification, compatibility.next_step
                            ),
                        );
                    }
                    None => {
                        print_surface_line(render, "boot compatibility", "none");
                    }
                }
                match migration_state {
                    Some(migration) => {
                        print_surface_line(
                            render,
                            "migration state",
                            &format!(
                                "{} / {} ({})",
                                migration.compatibility_classification,
                                migration.migration_state,
                                migration.next_step
                            ),
                        );
                    }
                    None => {
                        print_surface_line(render, "migration state", "none");
                    }
                }
                print_surface_line(
                    render,
                    "migration receipts",
                    &migration_receipts.as_display(),
                );
                match latest_task_reconciliation {
                    Some(receipt) => {
                        print_surface_line(
                            render,
                            "latest task reconciliation",
                            &receipt.as_display(),
                        );
                    }
                    None => {
                        print_surface_line(render, "latest task reconciliation", "none");
                    }
                }
                print_surface_line(
                    render,
                    "task reconciliation rollup",
                    &task_reconciliation_rollup.as_display(),
                );
                print_surface_line(
                    render,
                    "taskflow snapshot bridge",
                    &snapshot_bridge.as_display(),
                );
                print_surface_line(
                    render,
                    "runtime consumption",
                    &runtime_consumption.as_display(),
                );
                print_surface_line(render, "protocol binding", &protocol_binding.as_display());
                match latest_run_graph_status {
                    Some(status) => {
                        print_surface_line(render, "latest run graph status", &status.as_display());
                        print_surface_line(
                            render,
                            "latest run graph delegation gate",
                            &status.delegation_gate().as_display(),
                        );
                    }
                    None => {
                        print_surface_line(render, "latest run graph status", "none");
                    }
                }
                match latest_run_graph_recovery {
                    Some(summary) => {
                        print_surface_line(
                            render,
                            "latest run graph recovery",
                            &summary.as_display(),
                        );
                    }
                    None => {
                        print_surface_line(render, "latest run graph recovery", "none");
                    }
                }
                match latest_run_graph_checkpoint {
                    Some(summary) => {
                        print_surface_line(
                            render,
                            "latest run graph checkpoint",
                            &summary.as_display(),
                        );
                    }
                    None => {
                        print_surface_line(render, "latest run graph checkpoint", "none");
                    }
                }
                match latest_run_graph_gate {
                    Some(summary) => {
                        print_surface_line(render, "latest run graph gate", &summary.as_display());
                    }
                    None => {
                        print_surface_line(render, "latest run graph gate", "none");
                    }
                }
                if let Some(host_agents) = host_agents {
                    print_surface_line(
                        render,
                        "host agents",
                        host_agents["host_cli_system"].as_str().unwrap_or("unknown"),
                    );
                    print_surface_line(
                        render,
                        "host agent budget units",
                        &host_agents["budget"]["total_estimated_units"]
                            .as_u64()
                            .unwrap_or_default()
                            .to_string(),
                    );
                    print_surface_line(
                        render,
                        "host agent events",
                        &host_agents["budget"]["event_count"]
                            .as_u64()
                            .unwrap_or_default()
                            .to_string(),
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Failed to read storage metadata: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_doctor(args: DoctorArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;

    match StateStore::open_existing(state_dir).await {
        Ok(store) => {
            let storage_metadata = match store.storage_metadata_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("storage metadata: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let storage_metadata_display = format!(
                "{} state-v{} instruction-v{}",
                storage_metadata.backend,
                storage_metadata.state_schema_version,
                storage_metadata.instruction_schema_version
            );
            let state_spine = match store.state_spine_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("authoritative state spine: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_store = match store.task_store_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task store: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let run_graph = match store.run_graph_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("run graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let launcher_runtime_paths = match doctor_launcher_summary_json() {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("launcher/runtime paths: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let dependency_graph = match store.validate_task_graph().await {
                Ok(issues) if issues.is_empty() => issues,
                Ok(issues) => {
                    let first = issues.first().expect("issues is not empty");
                    eprintln!(
                        "dependency graph: failed ({} issue(s), first={} on {})",
                        issues.len(),
                        first.issue_type,
                        first.issue_id
                    );
                    return ExitCode::from(1);
                }
                Err(error) => {
                    eprintln!("dependency graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let boot_compatibility = match store.evaluate_boot_compatibility().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("boot compatibility: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_preflight = match store.evaluate_migration_preflight().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration preflight: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_receipts = match store.migration_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration receipts: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_task_reconciliation = match store.latest_task_reconciliation_summary().await
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_reconciliation_rollup = match store.task_reconciliation_rollup().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation rollup: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let snapshot_bridge = match store.taskflow_snapshot_bridge_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("taskflow snapshot bridge: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let runtime_consumption = match runtime_consumption_summary(store.root()) {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("runtime consumption: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let protocol_binding = match store.protocol_binding_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("protocol binding: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_status = match store.latest_run_graph_status().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph status: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_recovery = match store.latest_run_graph_recovery_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph recovery: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_checkpoint =
                match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph checkpoint: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let latest_run_graph_gate = match store.latest_run_graph_gate_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph gate: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_dispatch_receipt =
                match store.latest_run_graph_dispatch_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph dispatch receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let effective_instruction_bundle = match store.active_instruction_root().await {
                Ok(root_artifact_id) => match store
                    .inspect_effective_instruction_bundle(&root_artifact_id)
                    .await
                {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        eprintln!("effective instruction bundle: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                Err(error) => {
                    eprintln!("active instruction root: failed ({error})");
                    return ExitCode::from(1);
                }
            };

            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": "vida doctor",
                        "storage_metadata": {
                            "engine": storage_metadata.engine,
                            "backend": storage_metadata.backend,
                            "namespace": storage_metadata.namespace,
                            "database": storage_metadata.database,
                            "state_schema_version": storage_metadata.state_schema_version,
                            "instruction_schema_version": storage_metadata.instruction_schema_version,
                        },
                        "state_spine": {
                            "state_schema_version": state_spine.state_schema_version,
                            "entity_surface_count": state_spine.entity_surface_count,
                            "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                        },
                        "task_store": {
                            "total_count": task_store.total_count,
                            "open_count": task_store.open_count,
                            "in_progress_count": task_store.in_progress_count,
                            "closed_count": task_store.closed_count,
                            "epic_count": task_store.epic_count,
                            "ready_count": task_store.ready_count,
                        },
                        "run_graph": {
                            "execution_plan_count": run_graph.execution_plan_count,
                            "routed_run_count": run_graph.routed_run_count,
                            "governance_count": run_graph.governance_count,
                            "resumability_count": run_graph.resumability_count,
                            "reconciliation_count": run_graph.reconciliation_count,
                        },
                        "launcher_runtime_paths": launcher_runtime_paths,
                        "dependency_graph": {
                            "issue_count": dependency_graph.len(),
                        },
                        "boot_compatibility": {
                            "classification": boot_compatibility.classification,
                            "reasons": boot_compatibility.reasons,
                            "next_step": boot_compatibility.next_step,
                        },
                        "migration_preflight": {
                            "compatibility_classification": migration_preflight.compatibility_classification,
                            "migration_state": migration_preflight.migration_state,
                            "blockers": migration_preflight.blockers,
                            "source_version_tuple": migration_preflight.source_version_tuple,
                            "next_step": migration_preflight.next_step,
                        },
                        "migration_receipts": {
                            "compatibility_receipts": migration_receipts.compatibility_receipts,
                            "application_receipts": migration_receipts.application_receipts,
                            "verification_receipts": migration_receipts.verification_receipts,
                            "cutover_readiness_receipts": migration_receipts.cutover_readiness_receipts,
                            "rollback_notes": migration_receipts.rollback_notes,
                        },
                        "latest_task_reconciliation": latest_task_reconciliation,
                        "task_reconciliation_rollup": task_reconciliation_rollup,
                        "taskflow_snapshot_bridge": snapshot_bridge,
                        "runtime_consumption": runtime_consumption,
                        "protocol_binding": protocol_binding,
                        "latest_run_graph_status": latest_run_graph_status,
                        "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        "effective_instruction_bundle": {
                            "root_artifact_id": effective_instruction_bundle.root_artifact_id,
                            "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
                            "source_version_tuple": effective_instruction_bundle.source_version_tuple,
                            "receipt_id": effective_instruction_bundle.receipt_id,
                            "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
                        },
                        "storage_metadata_display": storage_metadata_display,
                    }))
                    .expect("doctor summary should render as json")
                );
                return ExitCode::SUCCESS;
            }

            print_surface_header(render, "vida doctor");
            print_surface_ok(render, "storage metadata", &storage_metadata_display);
            print_surface_ok(
                render,
                "authoritative state spine",
                &format!(
                    "state-v{}, {} entity surfaces, mutation root {}",
                    state_spine.state_schema_version,
                    state_spine.entity_surface_count,
                    state_spine.authoritative_mutation_root
                ),
            );
            print_surface_ok(render, "task store", &task_store.as_display());
            print_surface_ok(render, "run graph", &run_graph.as_display());
            print_surface_ok(
                render,
                "launcher/runtime paths",
                &format!(
                    "vida={}, project_root={}, taskflow_surface={}",
                    launcher_runtime_paths.vida,
                    launcher_runtime_paths.project_root,
                    launcher_runtime_paths.taskflow_surface
                ),
            );
            print_surface_ok(render, "dependency graph", "0 issues");
            print_surface_ok(
                render,
                "boot compatibility",
                &format!(
                    "{} ({})",
                    boot_compatibility.classification, boot_compatibility.next_step
                ),
            );
            print_surface_ok(
                render,
                "migration preflight",
                &format!(
                    "{} / {} ({})",
                    migration_preflight.compatibility_classification,
                    migration_preflight.migration_state,
                    migration_preflight.next_step
                ),
            );
            print_surface_ok(
                render,
                "migration receipts",
                &migration_receipts.as_display(),
            );
            match latest_task_reconciliation {
                Some(receipt) => {
                    print_surface_ok(render, "task reconciliation", &receipt.as_display());
                }
                None => {
                    print_surface_ok(render, "task reconciliation", "none");
                }
            }
            print_surface_ok(
                render,
                "task reconciliation rollup",
                &task_reconciliation_rollup.as_display(),
            );
            print_surface_ok(
                render,
                "taskflow snapshot bridge",
                &snapshot_bridge.as_display(),
            );
            print_surface_ok(
                render,
                "runtime consumption",
                &runtime_consumption.as_display(),
            );
            print_surface_ok(render, "protocol binding", &protocol_binding.as_display());
            match latest_run_graph_status {
                Some(status) => {
                    print_surface_ok(render, "latest run graph status", &status.as_display());
                    print_surface_ok(
                        render,
                        "latest run graph delegation gate",
                        &status.delegation_gate().as_display(),
                    );
                }
                None => {
                    print_surface_ok(render, "latest run graph status", "none");
                }
            }
            match latest_run_graph_recovery {
                Some(summary) => {
                    print_surface_ok(render, "latest run graph recovery", &summary.as_display());
                }
                None => {
                    print_surface_ok(render, "latest run graph recovery", "none");
                }
            }
            match latest_run_graph_checkpoint {
                Some(summary) => {
                    print_surface_ok(render, "latest run graph checkpoint", &summary.as_display());
                }
                None => {
                    print_surface_ok(render, "latest run graph checkpoint", "none");
                }
            }
            match latest_run_graph_gate {
                Some(summary) => {
                    print_surface_ok(render, "latest run graph gate", &summary.as_display());
                }
                None => {
                    print_surface_ok(render, "latest run graph gate", "none");
                }
            }
            print_surface_ok(
                render,
                "effective instruction bundle",
                &effective_instruction_bundle
                    .mandatory_chain_order
                    .join(" -> "),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct DoctorLauncherSummary {
    vida: String,
    project_root: String,
    taskflow_surface: String,
}

fn doctor_launcher_summary_json() -> Result<DoctorLauncherSummary, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let project_root = resolve_repo_root()?;
    Ok(DoctorLauncherSummary {
        vida: current_exe.display().to_string(),
        project_root: project_root.display().to_string(),
        taskflow_surface: "vida taskflow".to_string(),
    })
}

fn doctor_launcher_summary_for_root(project_root: &Path) -> Result<DoctorLauncherSummary, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    Ok(DoctorLauncherSummary {
        vida: current_exe.display().to_string(),
        project_root: project_root.display().to_string(),
        taskflow_surface: "vida taskflow".to_string(),
    })
}

#[derive(Debug, serde::Serialize)]
struct TaskflowConsumeBundlePayload {
    artifact_name: String,
    artifact_type: String,
    generated_at: String,
    vida_root: String,
    config_path: String,
    activation_source: String,
    launcher_runtime_paths: DoctorLauncherSummary,
    metadata: serde_json::Value,
    control_core: serde_json::Value,
    activation_bundle: serde_json::Value,
    protocol_binding_registry: serde_json::Value,
    cache_delivery_contract: serde_json::Value,
    orchestrator_init_view: serde_json::Value,
    agent_init_view: serde_json::Value,
    boot_compatibility: serde_json::Value,
    migration_preflight: serde_json::Value,
    task_store: serde_json::Value,
    run_graph: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct TaskflowConsumeBundleCheck {
    ok: bool,
    blockers: Vec<String>,
    root_artifact_id: String,
    artifact_count: usize,
    boot_classification: String,
    migration_state: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct RuntimeConsumptionLaneSelection {
    pub(crate) ok: bool,
    pub(crate) activation_source: String,
    pub(crate) selection_mode: String,
    pub(crate) fallback_role: String,
    pub(crate) request: String,
    pub(crate) selected_role: String,
    pub(crate) conversational_mode: Option<String>,
    pub(crate) single_task_only: bool,
    pub(crate) tracked_flow_entry: Option<String>,
    pub(crate) allow_freeform_chat: bool,
    pub(crate) confidence: String,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) compiled_bundle: serde_json::Value,
    pub(crate) execution_plan: serde_json::Value,
    pub(crate) reason: String,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionEvidence {
    surface: String,
    ok: bool,
    row_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_path: Option<String>,
    output: String,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionOverview {
    surface: String,
    ok: bool,
    registry_rows: usize,
    check_rows: usize,
    readiness_rows: usize,
    proof_blocking: bool,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionDocflowActivation {
    activated: bool,
    runtime_family: String,
    owner_runtime: String,
    evidence: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionDocflowVerdict {
    status: String,
    ready: bool,
    blockers: Vec<String>,
    proof_surfaces: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionClosureAdmission {
    status: String,
    admitted: bool,
    blockers: Vec<String>,
    proof_surfaces: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct TaskflowDirectConsumptionPayload {
    artifact_name: String,
    artifact_type: String,
    generated_at: String,
    closure_authority: String,
    request_text: String,
    role_selection: RuntimeConsumptionLaneSelection,
    runtime_bundle: TaskflowConsumeBundlePayload,
    bundle_check: TaskflowConsumeBundleCheck,
    docflow_activation: RuntimeConsumptionDocflowActivation,
    docflow_verdict: RuntimeConsumptionDocflowVerdict,
    closure_admission: RuntimeConsumptionClosureAdmission,
    taskflow_handoff_plan: serde_json::Value,
    run_graph_bootstrap: serde_json::Value,
    dispatch_receipt: serde_json::Value,
    direct_consumption_ready: bool,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionSummary {
    total_snapshots: usize,
    bundle_snapshots: usize,
    bundle_check_snapshots: usize,
    final_snapshots: usize,
    latest_kind: Option<String>,
    latest_snapshot_path: Option<String>,
}

fn config_file_path() -> Result<PathBuf, String> {
    Ok(resolve_runtime_project_root()?.join("vida.config.yaml"))
}

fn resolve_overlay_path(root: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

pub(crate) fn load_project_overlay_yaml() -> Result<serde_yaml::Value, String> {
    let path = config_file_path()?;
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn json_lookup<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|node| match node {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

fn json_bool(value: Option<&serde_json::Value>, default: bool) -> bool {
    match value {
        Some(serde_json::Value::Bool(flag)) => *flag,
        Some(serde_json::Value::String(text)) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => true,
            "false" | "no" | "off" | "0" => false,
            _ => default,
        },
        _ => default,
    }
}

fn json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Some(serde_json::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

fn csv_json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(serde_json::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

fn read_simple_toml_sections(path: &Path) -> HashMap<String, HashMap<String, String>> {
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut sections = HashMap::<String, HashMap<String, String>>::new();
    let mut current = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current = trimmed
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            sections.entry(current.clone()).or_default();
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let normalized = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        sections
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), normalized);
    }
    sections
}

pub(crate) fn yaml_lookup<'a>(
    value: &'a serde_yaml::Value,
    path: &[&str],
) -> Option<&'a serde_yaml::Value> {
    let mut current = value;
    for segment in path {
        match current {
            serde_yaml::Value::Mapping(map) => {
                current = map.get(serde_yaml::Value::String((*segment).to_string()))?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn yaml_string(value: Option<&serde_yaml::Value>) -> Option<String> {
    value.and_then(|node| match node {
        serde_yaml::Value::String(text) => Some(text.clone()),
        serde_yaml::Value::Number(number) => Some(number.to_string()),
        serde_yaml::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

pub(crate) fn yaml_bool(value: Option<&serde_yaml::Value>, default: bool) -> bool {
    value
        .and_then(|node| match node {
            serde_yaml::Value::Bool(flag) => Some(*flag),
            serde_yaml::Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => Some(true),
                "false" | "no" | "off" | "0" => Some(false),
                _ => None,
            },
            serde_yaml::Value::Number(number) => number.as_i64().map(|value| value != 0),
            _ => None,
        })
        .unwrap_or(default)
}

fn split_csv_like(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
        .collect()
}

fn yaml_string_list(value: Option<&serde_yaml::Value>) -> Vec<String> {
    match value {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| match row {
                serde_yaml::Value::String(text) => Some(text.trim().to_string()),
                _ => None,
            })
            .filter(|value| !value.is_empty())
            .collect(),
        Some(serde_yaml::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

fn overlay_codex_agent_catalog(config: &serde_yaml::Value) -> Vec<serde_json::Value> {
    let Some(serde_yaml::Value::Mapping(agents)) =
        yaml_lookup(config, &["host_environment", "codex", "agents"])
    else {
        return Vec::new();
    };
    let mut rows = agents
        .iter()
        .filter_map(|(agent_id, value)| {
            let role_id = match agent_id {
                serde_yaml::Value::String(text) if !text.trim().is_empty() => text.trim(),
                _ => return None,
            };
            Some(serde_json::json!({
                "role_id": role_id,
                "description": yaml_string(yaml_lookup(value, &["description"])).unwrap_or_default(),
                "config_file": format!("agents/{role_id}.toml"),
                "model": yaml_string(yaml_lookup(value, &["model"])).unwrap_or_default(),
                "model_reasoning_effort": yaml_string(yaml_lookup(value, &["model_reasoning_effort"])).unwrap_or_default(),
                "sandbox_mode": yaml_string(yaml_lookup(value, &["sandbox_mode"])).unwrap_or_default(),
                "tier": yaml_string(yaml_lookup(value, &["tier"])).unwrap_or_else(|| role_id.to_string()),
                "rate": yaml_string(yaml_lookup(value, &["rate"]))
                    .and_then(|raw| raw.parse::<u64>().ok())
                    .unwrap_or(0),
                "reasoning_band": yaml_string(yaml_lookup(value, &["reasoning_band"])).unwrap_or_default(),
                "default_runtime_role": yaml_string(yaml_lookup(value, &["default_runtime_role"])).unwrap_or_default(),
                "runtime_roles": yaml_string_list(yaml_lookup(value, &["runtime_roles"])),
                "task_classes": yaml_string_list(yaml_lookup(value, &["task_classes"])),
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["role_id"].as_str().unwrap_or_default())
            })
    });
    rows
}

fn overlay_codex_dispatch_alias_catalog(
    config: &serde_yaml::Value,
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let Some(serde_yaml::Value::Mapping(configured_aliases)) =
        yaml_lookup(config, &["host_environment", "codex", "dispatch_aliases"])
    else {
        return Vec::new();
    };
    let carrier_rows = agent_catalog
        .iter()
        .filter_map(|row| Some((row["tier"].as_str()?.to_string(), row.clone())))
        .collect::<HashMap<_, _>>();
    let mut rows = configured_aliases
        .iter()
        .filter_map(|(lane_id, value)| {
            let lane_id = match lane_id {
                serde_yaml::Value::String(text) if !text.trim().is_empty() => text.trim(),
                _ => return None,
            };
            let carrier_tier = yaml_string(yaml_lookup(value, &["carrier_tier"]))?;
            let mut row = carrier_rows.get(&carrier_tier)?.clone();
            let runtime_role = yaml_string(yaml_lookup(value, &["runtime_role"]))
                .or_else(|| yaml_string(yaml_lookup(value, &["default_runtime_role"])))
                .unwrap_or_default();
            let runtime_roles = {
                let rows = yaml_string_list(yaml_lookup(value, &["runtime_roles"]));
                if rows.is_empty() && !runtime_role.is_empty() {
                    vec![runtime_role.clone()]
                } else {
                    rows
                }
            };
            row["role_id"] = serde_json::Value::String(lane_id.to_string());
            row["description"] = serde_json::Value::String(
                yaml_string(yaml_lookup(value, &["description"])).unwrap_or_default(),
            );
            row["config_file"] = serde_json::Value::String(format!("agents/{lane_id}.toml"));
            row["default_runtime_role"] = serde_json::Value::String(runtime_role);
            row["runtime_roles"] = serde_json::json!(runtime_roles);
            row["task_classes"] =
                serde_json::json!(yaml_string_list(yaml_lookup(value, &["task_classes"])));
            row["template_role_id"] = serde_json::Value::String(carrier_tier);
            row["carrier_tier"] = row["tier"].clone();
            row["developer_instructions"] = serde_json::Value::String(
                yaml_string(yaml_lookup(value, &["developer_instructions"])).unwrap_or_default(),
            );
            Some(row)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["role_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["role_id"].as_str().unwrap_or_default())
    });
    rows
}

fn registry_rows_by_key(
    registry: &serde_yaml::Value,
    key: &str,
    id_field: &str,
    enabled_ids: &[String],
) -> Vec<serde_json::Value> {
    let enabled = enabled_ids.iter().cloned().collect::<HashSet<_>>();
    match yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| {
                let row_id = yaml_string(yaml_lookup(row, &[id_field]))?;
                if !enabled.is_empty() && !enabled.contains(&row_id) {
                    return None;
                }
                serde_json::to_value(row).ok()
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn registry_all_ids_by_key(registry: &serde_yaml::Value, key: &str, id_field: &str) -> Vec<String> {
    match yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| yaml_string(yaml_lookup(row, &[id_field])))
            .collect(),
        _ => Vec::new(),
    }
}

fn effective_enabled_registry_ids(
    config: &serde_yaml::Value,
    config_path: &[&str],
    registry: &serde_yaml::Value,
    registry_key: &str,
    id_field: &str,
) -> Vec<String> {
    if yaml_lookup(config, config_path).is_some() {
        return yaml_string_list(yaml_lookup(config, config_path));
    }
    registry_all_ids_by_key(registry, registry_key, id_field)
}

fn registry_row_map_by_id(
    rows: &[serde_json::Value],
    id_field: &str,
) -> HashMap<String, serde_json::Value> {
    rows.iter()
        .filter_map(|row| Some((row[id_field].as_str()?.to_string(), row.clone())))
        .collect()
}

fn registry_ids_by_key(registry: &serde_yaml::Value, key: &str, id_field: &str) -> HashSet<String> {
    match yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| yaml_string(yaml_lookup(row, &[id_field])))
            .collect(),
        _ => HashSet::new(),
    }
}

fn pack_router_keywords_json(config: &serde_yaml::Value) -> serde_json::Value {
    serde_json::json!({
        "research": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "research"])).unwrap_or_default()),
        "spec": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "spec"])).unwrap_or_default()),
        "pool": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool"])).unwrap_or_default()),
        "pool_strong": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool_strong"])).unwrap_or_default()),
        "pool_dependency": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool_dependency"])).unwrap_or_default()),
        "dev": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "dev"])).unwrap_or_default()),
        "bug": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "bug"])).unwrap_or_default()),
        "reflect": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "reflect"])).unwrap_or_default()),
        "reflect_strong": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "reflect_strong"])).unwrap_or_default()),
    })
}

fn config_file_digest(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "Failed to read config for digest at {}: {error}",
            path.display()
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn capture_launcher_activation_snapshot() -> Result<LauncherActivationSnapshot, String> {
    let config = load_project_overlay_yaml()?;
    let config_path = config_file_path()?;
    let config_digest = config_file_digest(&config_path)?;
    let config_root = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let compiled_bundle = build_compiled_agent_extension_bundle_for_root(&config, &config_root)?;
    Ok(LauncherActivationSnapshot {
        source: "state_store".to_string(),
        source_config_path: config_path.display().to_string(),
        source_config_digest: config_digest,
        captured_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        compiled_bundle,
        pack_router_keywords: pack_router_keywords_json(&config),
    })
}

async fn sync_launcher_activation_snapshot(
    store: &StateStore,
) -> Result<LauncherActivationSnapshot, String> {
    let snapshot = capture_launcher_activation_snapshot()?;
    store
        .write_launcher_activation_snapshot(&snapshot)
        .await
        .map_err(|error| format!("Failed to write launcher activation snapshot: {error}"))?;
    Ok(snapshot)
}

pub(crate) async fn read_or_sync_launcher_activation_snapshot(
    store: &StateStore,
) -> Result<LauncherActivationSnapshot, String> {
    let current_config = config_file_path().ok().and_then(|path| {
        let digest = config_file_digest(&path).ok()?;
        Some((path.display().to_string(), digest))
    });
    match store.read_launcher_activation_snapshot().await {
        Ok(snapshot) => {
            let same_config = current_config
                .as_ref()
                .map(|(path, digest)| {
                    path == &snapshot.source_config_path && digest == &snapshot.source_config_digest
                })
                .unwrap_or(false);
            if same_config {
                Ok(snapshot)
            } else {
                sync_launcher_activation_snapshot(store).await
            }
        }
        Err(StateStoreError::MissingLauncherActivationSnapshot) => {
            sync_launcher_activation_snapshot(store).await
        }
        Err(error) => Err(format!(
            "Failed to read launcher activation snapshot: {error}"
        )),
    }
}

fn build_runtime_lane_selection_from_bundle(
    bundle: &serde_json::Value,
    activation_source: &str,
    pack_router_keywords: &serde_json::Value,
    request: &str,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let selection_mode = json_string(json_lookup(&bundle, &["role_selection", "mode"]))
        .unwrap_or_else(|| "fixed".to_string());
    let configured_fallback =
        json_string(json_lookup(&bundle, &["role_selection", "fallback_role"]))
            .unwrap_or_else(|| "orchestrator".to_string());
    if !role_exists_in_lane_bundle(&bundle, &configured_fallback) {
        return Err(format!(
            "Agent extension bundle validation failed: fallback role `{configured_fallback}` is unresolved."
        ));
    }
    let fallback_role = configured_fallback;
    let normalized_request = request.to_lowercase();
    let mut result = RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: activation_source.to_string(),
        selection_mode: selection_mode.clone(),
        fallback_role: fallback_role.clone(),
        request: request.to_string(),
        selected_role: fallback_role.clone(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "fallback".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: String::new(),
    };

    if selection_mode != "auto" {
        result.reason = "fixed_mode".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    }

    let Some(serde_json::Value::Object(conversation_modes)) =
        json_lookup(&bundle, &["role_selection", "conversation_modes"])
    else {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    };
    if normalized_request.trim().is_empty() {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    }

    let mut candidates = Vec::new();
    for (mode_key, mode_value) in conversation_modes {
        let mode_id = mode_key.as_str();
        let serde_json::Value::Object(_) = mode_value else {
            continue;
        };
        if !json_bool(json_lookup(mode_value, &["enabled"]), true) {
            continue;
        }

        let mut keywords = match mode_id {
            "scope_discussion" => vec![
                "scope",
                "scoping",
                "requirement",
                "requirements",
                "acceptance",
                "constraint",
                "constraints",
                "clarify",
                "clarification",
                "discover",
                "discovery",
                "spec",
                "specification",
                "user story",
                "ac",
            ]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
            "pbi_discussion" => vec![
                "pbi",
                "backlog",
                "priority",
                "prioritize",
                "prioritization",
                "task",
                "ticket",
                "delivery cut",
                "estimate",
                "estimation",
                "roadmap",
                "decompose",
                "decomposition",
                "work pool",
            ]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let extra_keys: &[&str] = match mode_id {
            "scope_discussion" => &["spec"],
            "pbi_discussion" => &["pool", "pool_strong", "pool_dependency"],
            _ => &[],
        };
        for key in extra_keys {
            for keyword in json_string_list(json_lookup(pack_router_keywords, &[*key])) {
                if !keywords.contains(&keyword) {
                    keywords.push(keyword);
                }
            }
        }

        let matched_terms = contains_keywords(&normalized_request, &keywords);
        let selected_role = json_string(json_lookup(mode_value, &["role"]))
            .unwrap_or_else(|| fallback_role.clone());
        if !role_exists_in_lane_bundle(&bundle, &selected_role) {
            return Err(format!(
                "Agent extension bundle validation failed: conversation mode `{mode_id}` references unresolved role `{selected_role}`."
            ));
        }
        let tracked_flow_entry = json_string(json_lookup(mode_value, &["tracked_flow_entry"]));
        if let Some(flow_id) = tracked_flow_entry.as_deref() {
            if !tracked_flow_target_exists(&bundle, flow_id) {
                return Err(format!(
                    "Agent extension bundle validation failed: conversation mode `{mode_id}` references unresolved tracked flow entry `{flow_id}`."
                ));
            }
        }
        candidates.push((
            mode_id.to_string(),
            selected_role,
            json_bool(json_lookup(mode_value, &["single_task_only"]), false),
            tracked_flow_entry,
            json_bool(json_lookup(mode_value, &["allow_freeform_chat"]), false),
            matched_terms,
        ));
    }

    if candidates.is_empty() {
        result.reason = "auto_no_enabled_modes".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    }

    candidates.sort_by(|a, b| b.5.len().cmp(&a.5.len()).then_with(|| a.0.cmp(&b.0)));
    let selected = &candidates[0];
    if selected.5.is_empty() {
        let feature_terms = feature_delivery_design_terms(&normalized_request);
        if !feature_terms.is_empty() {
            if let Some(scope_candidate) = candidates.iter().find(|row| row.0 == "scope_discussion")
            {
                result.selected_role = scope_candidate.1.clone();
                result.conversational_mode = Some(scope_candidate.0.clone());
                result.single_task_only = scope_candidate.2;
                result.tracked_flow_entry = scope_candidate.3.clone();
                result.allow_freeform_chat = scope_candidate.4;
                result.matched_terms = feature_terms.clone();
                result.confidence = if feature_terms.len() >= 4 {
                    "high".to_string()
                } else {
                    "medium".to_string()
                };
                result.reason = "auto_feature_design_request".to_string();
                result.execution_plan =
                    build_runtime_execution_plan_from_snapshot(&bundle, &result);
                return Ok(result);
            }
        }

        result.reason = "auto_no_keyword_match".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    }
    if !role_exists_in_lane_bundle(&bundle, &selected.1) {
        result.reason = "auto_selected_unknown_role".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
        return Ok(result);
    }

    result.selected_role = selected.1.clone();
    result.conversational_mode = Some(selected.0.clone());
    result.single_task_only = selected.2;
    result.tracked_flow_entry = selected.3.clone();
    result.allow_freeform_chat = selected.4;
    result.matched_terms = selected.5.clone();
    result.confidence = match selected.5.len() {
        0 => "fallback".to_string(),
        1 => "low".to_string(),
        2 => "medium".to_string(),
        _ => "high".to_string(),
    };
    result.reason = "auto_keyword_match".to_string();
    result.execution_plan = build_runtime_execution_plan_from_snapshot(&bundle, &result);
    Ok(result)
}

pub(crate) async fn build_runtime_lane_selection_with_store(
    store: &StateStore,
    request: &str,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
    build_runtime_lane_selection_from_bundle(
        &snapshot.compiled_bundle,
        &snapshot.source,
        &snapshot.pack_router_keywords,
        request,
    )
}

fn summarize_agent_route_from_snapshot(
    compiled_bundle: &serde_json::Value,
    agent_system: &serde_json::Value,
    route_id: &str,
) -> serde_json::Value {
    let Some(route) = json_lookup(agent_system, &["routing", route_id]) else {
        return serde_json::Value::Null;
    };
    let (runtime_role, task_class) = match route_id {
        "implementation" | "small_patch" | "small_patch_write" | "ui_patch" => {
            ("worker", "implementation")
        }
        "coach" => ("coach", "coach"),
        "verification" | "verification_ensemble" | "review_ensemble" => {
            ("verifier", "verification")
        }
        "architecture" => ("solution_architect", "architecture"),
        _ => ("", ""),
    };
    let codex_runtime_assignment = if runtime_role.is_empty() || task_class.is_empty() {
        serde_json::Value::Null
    } else {
        build_codex_runtime_assignment_from_resolved_constraints(
            compiled_bundle,
            route_id,
            task_class,
            runtime_role,
        )
    };
    serde_json::json!({
        "route_id": route_id,
        "subagents": json_string(json_lookup(route, &["subagents"])).unwrap_or_default(),
        "fanout_subagents": json_string(json_lookup(route, &["fanout_subagents"])).unwrap_or_default(),
        "preferred_agent_type": codex_runtime_assignment["selected_agent_id"],
        "preferred_agent_tier": codex_runtime_assignment["selected_tier"],
        "preferred_runtime_role": codex_runtime_assignment["runtime_role"],
        "codex_runtime_assignment": codex_runtime_assignment,
        "profiles": json_lookup(route, &["profiles"]).cloned().unwrap_or(serde_json::Value::Null),
        "write_scope": json_string(json_lookup(route, &["write_scope"])).unwrap_or_default(),
        "dispatch_required": json_string(json_lookup(route, &["dispatch_required"])).unwrap_or_default(),
        "verification_gate": json_string(json_lookup(route, &["verification_gate"])).unwrap_or_default(),
        "analysis_required": json_bool(json_lookup(route, &["analysis_required"]), false),
        "analysis_route_task_class": json_string(json_lookup(route, &["analysis_route_task_class"])).unwrap_or_default(),
        "coach_required": json_bool(json_lookup(route, &["coach_required"]), false),
        "coach_route_task_class": json_string(json_lookup(route, &["coach_route_task_class"])).unwrap_or_default(),
        "verification_route_task_class": json_string(json_lookup(route, &["verification_route_task_class"])).unwrap_or_default(),
        "independent_verification_required": json_bool(json_lookup(route, &["independent_verification_required"]), false),
        "graph_strategy": json_string(json_lookup(route, &["graph_strategy"])).unwrap_or_default(),
        "internal_escalation_trigger": json_string(json_lookup(route, &["internal_escalation_trigger"])).unwrap_or_default(),
    })
}

fn coach_review_terms(normalized_request: &str) -> Vec<String> {
    contains_keywords(
        normalized_request,
        &[
            "acceptance criteria".to_string(),
            "against the spec".to_string(),
            "against spec".to_string(),
            "definition of done".to_string(),
            "implementation drift".to_string(),
            "implemented result".to_string(),
            "matches the spec".to_string(),
            "rework".to_string(),
            "spec compliance".to_string(),
            "spec conformance".to_string(),
        ],
    )
}

fn build_design_first_tracked_flow_bootstrap(request: &str) -> serde_json::Value {
    let feature_slug = infer_feature_request_slug(request)
        .trim()
        .trim_matches('-')
        .to_string();
    let feature_slug = if feature_slug.is_empty() {
        "feature-request".to_string()
    } else {
        feature_slug
    };
    let feature_title = infer_feature_request_title(request);
    let design_doc_path = format!("docs/product/spec/{feature_slug}-design.md");
    let artifact_path = format!("product/spec/{feature_slug}-design");
    let epic_task_id = format!("feature-{feature_slug}");
    let spec_task_id = format!("{epic_task_id}-spec");
    let work_pool_task_id = format!("{epic_task_id}-work-pool");
    let dev_task_id = format!("{epic_task_id}-dev");
    let epic_title = format!("Feature epic: {feature_title}");
    let spec_title = format!("Spec pack: {feature_title}");
    let work_pool_title = format!("Work-pool pack: {feature_title}");
    let dev_title = format!("Dev pack: {feature_title}");
    let quoted_request = shell_quote(request);

    serde_json::json!({
        "required": true,
        "status": "pending",
        "bootstrap_command": format!(
            "vida taskflow bootstrap-spec {} --json",
            quoted_request,
        ),
        "feature_slug": feature_slug,
        "feature_title": feature_title,
        "design_doc_path": design_doc_path,
        "design_artifact_path": artifact_path,
        "epic": {
            "required": true,
            "task_id": epic_task_id,
            "title": epic_title,
            "runtime": "vida taskflow",
            "create_command": format!(
                "vida taskflow task create {} {} --type epic --status open --labels feature-request --labels spec-first --description {} --json",
                epic_task_id,
                shell_quote(&epic_title),
                quoted_request,
            ),
            "close_command": format!(
                "vida taskflow task close {} --reason {} --json",
                epic_task_id,
                shell_quote("feature delivery closed after proof and runtime handoff"),
            )
        },
        "spec_task": {
            "required": true,
            "task_id": spec_task_id,
            "title": spec_title,
            "runtime": "vida taskflow",
            "create_command": format!(
                "vida taskflow task create {} {} --type task --status open --parent-id {} --labels spec-pack --labels documentation --description {} --json",
                spec_task_id,
                shell_quote(&spec_title),
                epic_task_id,
                shell_quote("bounded design/spec packet for the feature request"),
            ),
            "close_command": format!(
                "vida taskflow task close {} --reason {} --json",
                spec_task_id,
                shell_quote("design packet finalized and handed off into tracked work-pool shaping"),
            )
        },
        "work_pool_task": {
            "required": true,
            "task_id": work_pool_task_id,
            "title": work_pool_title,
            "runtime": "vida taskflow",
            "create_command": format!(
                "vida taskflow task create {} {} --type task --status open --parent-id {} --labels work-pool-pack --json",
                work_pool_task_id,
                shell_quote(&work_pool_title),
                epic_task_id,
            ),
            "close_command": format!(
                "vida taskflow task close {} --reason {} --json",
                work_pool_task_id,
                shell_quote("work-pool packet closed after delegated execution packet was shaped"),
            )
        },
        "dev_task": {
            "required": false,
            "task_id": dev_task_id,
            "title": dev_title,
            "runtime": "vida taskflow",
            "create_command": format!(
                "vida taskflow task create {} {} --type task --status open --parent-id {} --labels dev-pack --json",
                dev_task_id,
                shell_quote(&dev_title),
                epic_task_id,
            ),
            "close_command": format!(
                "vida taskflow task close {} --reason {} --json",
                dev_task_id,
                shell_quote("delegated development packet reached proof-ready closure"),
            )
        },
        "docflow": {
            "required": true,
            "runtime": "vida docflow",
            "init_command": format!(
                "vida docflow init {} {} product_spec {}",
                design_doc_path,
                artifact_path,
                shell_quote("initialize bounded feature design"),
            ),
            "finalize_command": format!(
                "vida docflow finalize-edit {} {}",
                design_doc_path,
                shell_quote("record bounded feature design"),
            ),
            "check_command": format!(
                "vida docflow check --root . {}",
                design_doc_path,
            )
        },
        "handoff_sequence": [
            "create epic",
            "open spec task",
            "initialize bounded design document",
            "finalize and validate bounded design document",
            "close spec task",
            "open work-pool shaping task",
            "shape dev packet in TaskFlow before delegated implementation"
        ]
    })
}

fn infer_codex_task_class(
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> String {
    let normalized_request = selection.request.to_lowercase();
    let has_architecture_terms = contains_keywords(
        &normalized_request,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "topology".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "refactor".to_string(),
            "migration".to_string(),
            "security".to_string(),
            "hard conflict".to_string(),
            "meta-analysis".to_string(),
            "meta analysis".to_string(),
        ],
    )
    .len()
        >= 2;
    let coach_terms = coach_review_terms(&normalized_request);
    if selection.selected_role == "solution_architect" || has_architecture_terms {
        return "architecture".to_string();
    }
    if selection.selected_role == "coach" || !coach_terms.is_empty() {
        return "coach".to_string();
    }
    if selection.selected_role == "verifier"
        || selection.selected_role == "prover"
        || contains_keywords(
            &normalized_request,
            &[
                "verify".to_string(),
                "verification".to_string(),
                "proof".to_string(),
                "review".to_string(),
                "audit".to_string(),
                "test".to_string(),
            ],
        )
        .len()
            >= 1
    {
        return "verification".to_string();
    }
    if requires_design_gate
        || selection.selected_role == "business_analyst"
        || selection.selected_role == "pm"
    {
        return "specification".to_string();
    }
    "implementation".to_string()
}

fn infer_codex_execution_runtime_role(
    selection: &RuntimeConsumptionLaneSelection,
    task_class: &str,
    requires_design_gate: bool,
) -> String {
    if selection.selected_role == "pm" {
        return "pm".to_string();
    }
    if selection.selected_role == "coach" || task_class == "coach" {
        return "coach".to_string();
    }
    if requires_design_gate || selection.selected_role == "business_analyst" {
        return "business_analyst".to_string();
    }
    if selection.selected_role == "worker" {
        return "worker".to_string();
    }
    codex_runtime_role_for_task_class(task_class).to_string()
}

fn codex_runtime_role_for_task_class(task_class: &str) -> &'static str {
    match task_class {
        "architecture" => "solution_architect",
        "verification" => "verifier",
        "coach" => "coach",
        "specification" => "business_analyst",
        _ => "worker",
    }
}

fn codex_task_complexity_multiplier(task_class: &str) -> u64 {
    match task_class {
        "architecture" | "execution_preparation" | "hard_escalation" | "meta_analysis" => 4,
        "verification" | "review" | "quality_gate" | "release_readiness" => 2,
        "specification" | "planning" | "coach" | "implementation_medium" => 2,
        _ => 1,
    }
}

fn role_supports_runtime_role(role: &serde_json::Value, runtime_role: &str) -> bool {
    let runtime_roles = role["runtime_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    runtime_roles.is_empty() || runtime_roles.iter().any(|value| *value == runtime_role)
}

fn role_supports_codex_task_class(role: &serde_json::Value, task_class: &str) -> bool {
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    task_classes.is_empty() || task_classes.iter().any(|value| *value == task_class)
}

fn build_codex_runtime_assignment_from_resolved_constraints(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
) -> serde_json::Value {
    let Some(roles) = compiled_bundle["codex_multi_agent"]["roles"].as_array() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "codex_multi_agent_roles_missing"
        });
    };
    if roles.is_empty() {
        return serde_json::json!({
            "enabled": false,
            "reason": "codex_multi_agent_roles_missing"
        });
    }

    let demotion_score = json_u64(json_lookup(
        &compiled_bundle["codex_multi_agent"]["worker_strategy"],
        &["selection_policy", "demotion_score"],
    ))
    .unwrap_or(45);

    let mut candidates = roles
        .iter()
        .filter_map(|role| {
            let role_id = role["role_id"].as_str()?;
            let rate = role["rate"].as_u64().unwrap_or(0);
            if rate == 0 {
                return None;
            }
            let strategy =
                &compiled_bundle["codex_multi_agent"]["worker_strategy"]["agents"][role_id];
            let effective_score =
                json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
            let lifecycle_state = strategy["lifecycle_state"].as_str().unwrap_or("probation");
            let supports_runtime_role = role_supports_runtime_role(role, &execution_runtime_role);
            let supports_task_class = role_supports_codex_task_class(role, &task_class);
            Some((
                !supports_runtime_role,
                !supports_task_class,
                effective_score < demotion_score || lifecycle_state == "retired",
                rate,
                std::cmp::Reverse(effective_score),
                role.clone(),
                strategy.clone(),
            ))
        })
        .collect::<Vec<_>>();

    let has_exact_match =
        candidates
            .iter()
            .any(|(runtime_role_miss, task_class_miss, _, _, _, _, _)| {
                !*runtime_role_miss && !*task_class_miss
            });
    if !has_exact_match {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_codex_agent_declares_runtime_role_and_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role
        });
    }

    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
    });
    let Some((_, _, _, _, _, selected_role, strategy)) = candidates.first() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_codex_agent_satisfies_runtime_role_or_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role
        });
    };

    let tier = selected_role["tier"].as_str().unwrap_or_default();
    let rate = selected_role["rate"].as_u64().unwrap_or(0);
    let complexity_multiplier = codex_task_complexity_multiplier(&task_class);
    let effective_score = json_u64(json_lookup(&strategy, &["effective_score"])).unwrap_or(70);
    let lifecycle_state = strategy["lifecycle_state"].as_str().unwrap_or("probation");
    let rationale = vec![
        format!("task_class={task_class}"),
        format!("conversation_role={conversation_role}"),
        format!("execution_runtime_role={execution_runtime_role}"),
        format!("selected_tier={tier}"),
        format!("effective_score={effective_score}"),
        format!("lifecycle_state={lifecycle_state}"),
        "selection_rule=capability_first_then_score_guard_then_cheapest_tier".to_string(),
    ];

    serde_json::json!({
        "enabled": true,
        "task_class": task_class,
        "runtime_role": execution_runtime_role,
        "conversation_role": conversation_role,
        "activation_agent_type": selected_role["role_id"],
        "activation_runtime_role": execution_runtime_role,
        "selected_agent_id": selected_role["role_id"],
        "selected_carrier_agent_id": selected_role["role_id"],
        "selected_tier": selected_role["tier"],
        "selected_carrier_tier": selected_role["tier"],
        "selected_runtime_role": execution_runtime_role,
        "tier_default_runtime_role": selected_role["default_runtime_role"],
        "reasoning_band": selected_role["reasoning_band"],
        "model_reasoning_effort": selected_role["model_reasoning_effort"],
        "sandbox_mode": selected_role["sandbox_mode"],
        "rate": rate,
        "estimated_task_price_units": rate * complexity_multiplier,
        "complexity_multiplier": complexity_multiplier,
        "effective_score": effective_score,
        "lifecycle_state": lifecycle_state,
        "strategy_store": compiled_bundle["codex_multi_agent"]["worker_strategy"]["store_path"],
        "scorecards_store": compiled_bundle["codex_multi_agent"]["worker_strategy"]["scorecards_path"],
        "rationale": rationale
    })
}

fn build_codex_runtime_assignment(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let task_class = infer_codex_task_class(selection, requires_design_gate);
    let execution_runtime_role =
        infer_codex_execution_runtime_role(selection, &task_class, requires_design_gate);
    build_codex_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        &selection.selected_role,
        &task_class,
        &execution_runtime_role,
    )
}

pub(crate) fn build_runtime_execution_plan_from_snapshot(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let agent_system = &compiled_bundle["agent_system"];
    let implementation =
        summarize_agent_route_from_snapshot(compiled_bundle, agent_system, "implementation");
    let coach_route_id = implementation["coach_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("coach");
    let verification_route_id = implementation["verification_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("verification");
    let feature_design_terms = feature_delivery_design_terms(&selection.request.to_lowercase());
    let requires_design_gate = selection.tracked_flow_entry.as_deref() == Some("spec-pack")
        || !feature_design_terms.is_empty();
    let tracked_flow_bootstrap = if requires_design_gate {
        build_design_first_tracked_flow_bootstrap(&selection.request)
    } else {
        serde_json::Value::Null
    };
    let codex_runtime_assignment =
        build_codex_runtime_assignment(compiled_bundle, selection, requires_design_gate);
    let implementer_activation = build_codex_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        "implementation",
        "implementation",
        "worker",
    );
    let coach_activation = build_codex_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        "coach",
        "coach",
        "coach",
    );
    let verifier_activation = build_codex_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        "verification",
        "verification",
        "verifier",
    );
    let escalation_activation = build_codex_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        "architecture",
        "architecture",
        "solution_architect",
    );
    let lane_sequence = [
        implementer_activation["activation_agent_type"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        coach_activation["activation_agent_type"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        verifier_activation["activation_agent_type"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>();
    serde_json::json!({
        "status": if requires_design_gate {
            "design_first"
        } else {
            "ready_for_runtime_routing"
        },
        "system_mode": json_string(json_lookup(agent_system, &["mode"])).unwrap_or_default(),
        "state_owner": json_string(json_lookup(agent_system, &["state_owner"])).unwrap_or_default(),
        "max_parallel_agents": json_lookup(agent_system, &["max_parallel_agents"]).cloned().unwrap_or(serde_json::Value::Null),
        "default_route": summarize_agent_route_from_snapshot(compiled_bundle, agent_system, "default"),
        "conversation_stage": {
            "selected_role": selection.selected_role,
            "conversational_mode": selection.conversational_mode,
            "tracked_flow_entry": selection.tracked_flow_entry,
            "allow_freeform_chat": selection.allow_freeform_chat,
            "single_task_only": selection.single_task_only,
        },
        "pre_execution_design_gate": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "not_required"
            },
            "design_runtime": "vida docflow",
            "design_template": DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            "intake_runtime": if requires_design_gate {
                serde_json::Value::String("vida taskflow consume final <request> --json".to_string())
            } else {
                serde_json::Value::Null
            },
            "tracked_handoff": if requires_design_gate {
                serde_json::Value::String("spec-pack".to_string())
            } else {
                serde_json::Value::Null
            },
            "todo_sequence": if requires_design_gate {
                serde_json::json!([
                    "capture research, specification scope, and implementation plan in one bounded design document",
                    "create one epic and one spec task in vida taskflow before code execution",
                    "keep the design artifact canonical through vida docflow init/finalize-edit/check",
                    "close the spec task and shape one bounded execution packet from the approved design before delegated development"
                ])
            } else {
                serde_json::json!([])
            },
            "taskflow_sequence": if requires_design_gate {
                serde_json::json!(["spec-pack", "work-pool-pack", "dev-pack"])
            } else {
                serde_json::json!([])
            }
        },
        "pre_execution_todo": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "open"
            } else {
                "not_required"
            },
            "items": if requires_design_gate {
                serde_json::json!([
                    {
                        "id": "taskflow_epic_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one epic that will own the feature-level tracked flow before documentation or implementation begins"
                    },
                    {
                        "id": "taskflow_spec_task_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one spec-pack task under the epic before authoring the design artifact"
                    },
                    {
                        "id": "design_doc_scope",
                        "owner": "business_analyst",
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "capture research, specification scope, and implementation plan in one bounded design document"
                    },
                    {
                        "id": "design_doc_finalize",
                        "owner": "orchestrator",
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "finalize and validate the bounded design artifact canonically"
                    },
                    {
                        "id": "taskflow_spec_task_close",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "close the spec-pack task only after the design artifact is finalized and validated"
                    },
                    {
                        "id": "taskflow_packet_shape",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "shape TaskFlow handoff from spec-pack through work-pool-pack and dev-pack before delegated implementation dispatch"
                    }
                ])
            } else {
                serde_json::json!([])
            }
        },
        "tracked_flow_bootstrap": tracked_flow_bootstrap,
        "codex_runtime_assignment": codex_runtime_assignment,
        "development_flow": {
            "activation_status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "eligible_after_runtime_routing"
            },
            "lane_sequence": lane_sequence,
            "generic_single_worker_dispatch_forbidden": true,
            "dispatch_contract": {
                "root_session_must_remain_orchestrator": true,
                "packet_family_required": [
                    "delivery_task_packet",
                    "execution_block_packet",
                    "coach_review_packet",
                    "verifier_proof_packet"
                ],
                "implementer_activation": implementer_activation,
                "coach_activation": coach_activation,
                "verifier_activation": verifier_activation,
                "escalation_activation": escalation_activation
            },
            "timeout_policy": {
                "worker_wait_timeout_is_not_root_write_permission": true,
                "generic_internal_worker_fallback_forbidden": true,
                "root_session_takeover_requires_exception_receipt": true,
                "next_actions": [
                    "continue_lawful_waiting_or_polling",
                    "inspect_open_delegated_lane_state",
                    "reuse_or_reclaim_eligible_lane_if_lawful",
                    "dispatch_coach_or_verifier_or_escalation_when_route_requires_it",
                    "record_explicit_blocker_or_exception_path_before_any_root_session_write"
                ]
            },
            "implementation": implementation,
            "coach": summarize_agent_route_from_snapshot(compiled_bundle, agent_system, coach_route_id),
            "verification": summarize_agent_route_from_snapshot(compiled_bundle, agent_system, verification_route_id),
        },
    })
}

fn role_exists_in_lane_bundle(bundle: &serde_json::Value, role_id: &str) -> bool {
    if role_id.is_empty() {
        return false;
    }

    bundle["enabled_framework_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .any(|value| value == role_id)
        || bundle["project_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role_id"].as_str())
            .any(|value| value == role_id)
}

fn known_tracked_flow_targets() -> &'static [&'static str] {
    &[
        "research-pack",
        "spec-pack",
        "work-pool-pack",
        "dev-pack",
        "bug-pool-pack",
        "reflection-pack",
    ]
}

fn bundle_project_flow_exists(bundle: &serde_json::Value, flow_id: &str) -> bool {
    bundle["project_flows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["flow_id"].as_str())
        .any(|value| value == flow_id)
}

fn tracked_flow_target_exists(bundle: &serde_json::Value, flow_id: &str) -> bool {
    known_tracked_flow_targets().contains(&flow_id) || bundle_project_flow_exists(bundle, flow_id)
}

fn collect_missing_registry_ids(
    existing_ids: &HashSet<String>,
    enabled_ids: &[String],
) -> Vec<String> {
    enabled_ids
        .iter()
        .filter(|id| !existing_ids.contains(*id))
        .cloned()
        .collect()
}

fn yaml_key_matches(value: &serde_yaml::Value, expected: &str) -> bool {
    matches!(value, serde_yaml::Value::String(text) if text == expected)
}

fn merge_registry_projection(
    base_registry: &serde_yaml::Value,
    sidecar_registry: &serde_yaml::Value,
    key: &str,
    id_field: &str,
    registry_label: &str,
) -> Result<serde_yaml::Value, String> {
    let mut merged_mapping = match base_registry {
        serde_yaml::Value::Mapping(mapping) => mapping.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    if let serde_yaml::Value::Mapping(sidecar_mapping) = sidecar_registry {
        for (entry_key, entry_value) in sidecar_mapping {
            if yaml_key_matches(entry_key, key) {
                continue;
            }
            merged_mapping.insert(entry_key.clone(), entry_value.clone());
        }
    }

    let mut merged_rows = Vec::new();
    let mut index_by_id = HashMap::new();
    for (source_name, registry) in [("base", base_registry), ("sidecar", sidecar_registry)] {
        let Some(serde_yaml::Value::Sequence(rows)) = yaml_lookup(registry, &[key]) else {
            continue;
        };
        for row in rows {
            let row_id = yaml_string(yaml_lookup(row, &[id_field])).ok_or_else(|| {
                format!(
                    "agent extension {registry_label} {source_name} projection contains a row without `{id_field}`"
                )
            })?;
            if let Some(index) = index_by_id.get(&row_id).copied() {
                merged_rows[index] = row.clone();
            } else {
                index_by_id.insert(row_id, merged_rows.len());
                merged_rows.push(row.clone());
            }
        }
    }

    merged_mapping.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::Sequence(merged_rows),
    );
    if !merged_mapping.contains_key(&serde_yaml::Value::String("version".to_string())) {
        merged_mapping.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(1)),
        );
    }
    Ok(serde_yaml::Value::Mapping(merged_mapping))
}

fn load_registry_projection(
    root: &Path,
    configured_path: Option<&str>,
    key: &str,
    id_field: &str,
    registry_label: &str,
    require_registry_files: bool,
) -> Result<serde_yaml::Value, String> {
    let Some(path) = configured_path else {
        return Ok(serde_yaml::Value::Null);
    };
    let registry_path = resolve_overlay_path(root, path);
    let sidecar_path = registry_sidecar_path(&registry_path);
    let base_registry = match read_yaml_file_checked(&registry_path) {
        Ok(value) => value,
        Err(error) => {
            if require_registry_files || sidecar_path.exists() {
                return Err(error);
            }
            return Ok(serde_yaml::Value::Null);
        }
    };
    let sidecar_registry = if sidecar_path.is_file() {
        read_yaml_file_checked(&sidecar_path)?
    } else {
        serde_yaml::Value::Null
    };
    merge_registry_projection(
        &base_registry,
        &sidecar_registry,
        key,
        id_field,
        registry_label,
    )
}

fn build_compiled_agent_extension_bundle_for_root(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<serde_json::Value, String> {
    let configured_enabled_project_roles = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_roles"],
    ));
    let configured_enabled_project_profiles = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_profiles"],
    ));
    let configured_enabled_project_flows = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_flows"],
    ));
    let roles_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "roles"],
    ));
    let skills_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "skills"],
    ));
    let profiles_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "profiles"],
    ));
    let flows_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "flows"],
    ));
    let require_registry_files = yaml_bool(
        yaml_lookup(
            config,
            &["agent_extensions", "validation", "require_registry_files"],
        ),
        false,
    );
    let require_profile_resolution = yaml_bool(
        yaml_lookup(
            config,
            &[
                "agent_extensions",
                "validation",
                "require_profile_resolution",
            ],
        ),
        false,
    );
    let require_flow_resolution = yaml_bool(
        yaml_lookup(
            config,
            &["agent_extensions", "validation", "require_flow_resolution"],
        ),
        false,
    );
    let require_framework_role_compatibility = yaml_bool(
        yaml_lookup(
            config,
            &[
                "agent_extensions",
                "validation",
                "require_framework_role_compatibility",
            ],
        ),
        false,
    );
    let require_skill_role_compatibility = yaml_bool(
        yaml_lookup(
            config,
            &[
                "agent_extensions",
                "validation",
                "require_skill_role_compatibility",
            ],
        ),
        false,
    );
    let mut validation_errors = Vec::new();
    let roles_registry = match roles_path.as_deref() {
        Some(path) => match load_registry_projection(
            root,
            Some(path),
            "roles",
            "role_id",
            "roles",
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !configured_enabled_project_roles.is_empty() {
                validation_errors.push(
                    "agent extension roles registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let skills_registry = match skills_path.as_deref() {
        Some(path) => match load_registry_projection(
            root,
            Some(path),
            "skills",
            "skill_id",
            "skills",
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => serde_yaml::Value::Null,
    };
    let profiles_registry = match profiles_path.as_deref() {
        Some(path) => match load_registry_projection(
            root,
            Some(path),
            "profiles",
            "profile_id",
            "profiles",
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !configured_enabled_project_profiles.is_empty() {
                validation_errors.push(
                    "agent extension profiles registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let flows_registry = match flows_path.as_deref() {
        Some(path) => match load_registry_projection(
            root,
            Some(path),
            "flow_sets",
            "flow_id",
            "flows",
            require_registry_files,
        ) {
            Ok(value) => value,
            Err(error) => {
                validation_errors.push(error);
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !configured_enabled_project_flows.is_empty() {
                validation_errors.push(
                    "agent extension flows registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let enabled_project_roles = effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_roles"],
        &roles_registry,
        "roles",
        "role_id",
    );
    let enabled_project_skills = effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_skills"],
        &skills_registry,
        "skills",
        "skill_id",
    );
    let enabled_project_profiles = effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_profiles"],
        &profiles_registry,
        "profiles",
        "profile_id",
    );
    let enabled_project_flows = effective_enabled_registry_ids(
        config,
        &["agent_extensions", "enabled_project_flows"],
        &flows_registry,
        "flow_sets",
        "flow_id",
    );
    let codex_root = root.join(".codex");
    let codex_config = read_simple_toml_sections(&codex_root.join("config.toml"));
    let overlay_codex_roles = overlay_codex_agent_catalog(config);
    let codex_roles = if overlay_codex_roles.is_empty() {
        read_codex_agent_catalog(&codex_root)
    } else {
        overlay_codex_roles
    };
    let codex_validation_errors = codex_roles
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            let mut missing = Vec::new();
            if row["rate"].as_u64().unwrap_or(0) == 0 {
                missing.push("rate");
            }
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "codex agent `{role_id}` is missing required overlay/template metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect::<Vec<_>>();
    validation_errors.extend(codex_validation_errors);
    let codex_dispatch_aliases = overlay_codex_dispatch_alias_catalog(config, &codex_roles);
    let codex_dispatch_alias_validation_errors = codex_dispatch_aliases
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            let mut missing = Vec::new();
            if row["template_role_id"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("carrier_tier");
            }
            if row["default_runtime_role"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("runtime_role");
            }
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if row["developer_instructions"]
                .as_str()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                missing.push("developer_instructions");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "codex dispatch alias `{role_id}` is missing required overlay metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect::<Vec<_>>();
    validation_errors.extend(codex_dispatch_alias_validation_errors);
    let scoring_policy = serde_json::to_value(
        yaml_lookup(config, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let worker_strategy = if codex_roles.is_empty() {
        serde_json::json!({
            "schema_version": 1,
            "store_path": CODEX_WORKER_STRATEGY_STATE,
            "scorecards_path": CODEX_WORKER_SCORECARDS_STATE,
            "agents": {}
        })
    } else {
        refresh_codex_worker_strategy(root, &codex_roles, &scoring_policy)
    };
    let pricing_policy = build_codex_pricing_policy(&codex_roles, &worker_strategy);
    let project_roles =
        registry_rows_by_key(&roles_registry, "roles", "role_id", &enabled_project_roles);
    let project_skills = registry_rows_by_key(
        &skills_registry,
        "skills",
        "skill_id",
        &enabled_project_skills,
    );
    let project_profiles = registry_rows_by_key(
        &profiles_registry,
        "profiles",
        "profile_id",
        &enabled_project_profiles,
    );
    let project_flows = registry_rows_by_key(
        &flows_registry,
        "flow_sets",
        "flow_id",
        &enabled_project_flows,
    );
    let project_role_map = registry_row_map_by_id(&project_roles, "role_id");
    let project_skill_map = registry_row_map_by_id(&project_skills, "skill_id");
    let project_profile_map = registry_row_map_by_id(&project_profiles, "profile_id");
    let project_flow_map = registry_row_map_by_id(&project_flows, "flow_id");
    let enabled_framework_roles = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_framework_roles"],
    ));

    if require_framework_role_compatibility {
        for role in &project_roles {
            let role_id = role["role_id"].as_str().unwrap_or("<unknown>");
            let base_role = role["base_role"].as_str().unwrap_or_default();
            if base_role.is_empty() || !enabled_framework_roles.iter().any(|row| row == base_role) {
                validation_errors.push(format!(
                    "project role `{role_id}` references unresolved framework base role `{base_role}`"
                ));
            }
        }
    }

    if require_profile_resolution {
        for profile in &project_profiles {
            let profile_id = profile["profile_id"].as_str().unwrap_or("<unknown>");
            let role_ref = profile["role_ref"].as_str().unwrap_or_default();
            if role_ref.is_empty()
                || !(enabled_framework_roles.iter().any(|row| row == role_ref)
                    || project_role_map.contains_key(role_ref))
            {
                validation_errors.push(format!(
                    "project profile `{profile_id}` references unresolved role `{role_ref}`"
                ));
            }
        }
    }

    if require_skill_role_compatibility {
        for profile in &project_profiles {
            let profile_id = profile["profile_id"].as_str().unwrap_or("<unknown>");
            let role_ref = profile["role_ref"].as_str().unwrap_or_default();
            let Some(role) = project_role_map.get(role_ref) else {
                continue;
            };
            let base_role = role["base_role"].as_str().unwrap_or_default();
            for skill_ref in csv_json_string_list(profile.get("skill_refs")) {
                let Some(skill) = project_skill_map.get(&skill_ref) else {
                    validation_errors.push(format!(
                        "project profile `{profile_id}` references unresolved skill `{skill_ref}`"
                    ));
                    continue;
                };
                let compatible_roles = csv_json_string_list(skill.get("compatible_base_roles"));
                if !compatible_roles.is_empty()
                    && !compatible_roles.iter().any(|row| row == base_role)
                {
                    validation_errors.push(format!(
                        "project profile `{profile_id}` binds role `{role_ref}` with base role `{base_role}` to incompatible skill `{skill_ref}`"
                    ));
                }
            }
        }
    }

    if require_flow_resolution {
        for flow in &project_flows {
            let flow_id = flow["flow_id"].as_str().unwrap_or("<unknown>");
            for role_ref in csv_json_string_list(flow.get("role_chain")) {
                if !(enabled_framework_roles.iter().any(|row| row == &role_ref)
                    || project_role_map.contains_key(&role_ref))
                {
                    validation_errors.push(format!(
                        "project flow `{flow_id}` references unresolved role `{role_ref}`"
                    ));
                }
            }
        }
    }

    let bundle = serde_json::json!({
        "ok": true,
        "enabled": yaml_bool(yaml_lookup(config, &["agent_extensions", "enabled"]), false),
        "map_doc": yaml_string(yaml_lookup(config, &["agent_extensions", "map_doc"])).unwrap_or_default(),
        "enabled_framework_roles": enabled_framework_roles,
        "enabled_standard_flow_sets": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_standard_flow_sets"])),
        "enabled_shared_skills": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_shared_skills"])),
        "default_flow_set": yaml_string(yaml_lookup(config, &["agent_extensions", "default_flow_set"])).unwrap_or_default(),
        "runtime_projection_root": runtime_agent_extensions_root(root).display().to_string(),
        "project_roles": project_roles,
        "project_skills": project_skills,
        "project_profiles": project_profiles,
        "project_flows": project_flows,
        "project_role_catalog": project_role_map,
        "project_profile_catalog": project_profile_map,
        "project_flow_catalog": project_flow_map,
        "agent_system": serde_json::to_value(yaml_lookup(config, &["agent_system"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "autonomous_execution": serde_json::to_value(yaml_lookup(config, &["autonomous_execution"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "codex_multi_agent": serde_json::json!({
            "enabled": codex_config
                .get("features")
                .and_then(|section| section.get("multi_agent"))
                .map(|value| value == "true")
                .unwrap_or(false),
            "max_threads": codex_config
                .get("agents")
                .and_then(|section| section.get("max_threads"))
                .cloned()
                .unwrap_or_default(),
            "max_depth": codex_config
                .get("agents")
                .and_then(|section| section.get("max_depth"))
                .cloned()
                .unwrap_or_default(),
            "roles": codex_roles,
            "dispatch_aliases": codex_dispatch_aliases,
            "worker_strategy": worker_strategy,
            "pricing_policy": pricing_policy,
        }),
        "role_selection": serde_json::to_value(yaml_lookup(config, &["agent_extensions", "role_selection"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
    });

    let role_ids = registry_ids_by_key(&roles_registry, "roles", "role_id");
    let skill_ids = registry_ids_by_key(&skills_registry, "skills", "skill_id");
    let profile_ids = registry_ids_by_key(&profiles_registry, "profiles", "profile_id");
    let flow_ids = registry_ids_by_key(&flows_registry, "flow_sets", "flow_id");

    let missing_roles = collect_missing_registry_ids(&role_ids, &enabled_project_roles);
    if !missing_roles.is_empty() {
        validation_errors.push(format!(
            "agent extension roles registry is missing enabled role ids: {}",
            missing_roles.join(", ")
        ));
    }
    let missing_skills = collect_missing_registry_ids(&skill_ids, &enabled_project_skills);
    if !missing_skills.is_empty() {
        validation_errors.push(format!(
            "agent extension skills registry is missing enabled skill ids: {}",
            missing_skills.join(", ")
        ));
    }
    if require_profile_resolution {
        let missing_profiles =
            collect_missing_registry_ids(&profile_ids, &enabled_project_profiles);
        if !missing_profiles.is_empty() {
            validation_errors.push(format!(
                "agent extension profiles registry is missing enabled profile ids: {}",
                missing_profiles.join(", ")
            ));
        }
    }
    if require_flow_resolution {
        let missing_flows = collect_missing_registry_ids(&flow_ids, &enabled_project_flows);
        if !missing_flows.is_empty() {
            validation_errors.push(format!(
                "agent extension flows registry is missing enabled flow ids: {}",
                missing_flows.join(", ")
            ));
        }
    }

    if !validation_errors.is_empty() {
        return Err(format!(
            "Agent extension bundle validation failed: {}",
            validation_errors.join("; ")
        ));
    }

    Ok(bundle)
}

fn contains_keywords(request: &str, keywords: &[String]) -> Vec<String> {
    fn is_boundary(ch: Option<char>) -> bool {
        ch.map(|value| !value.is_alphanumeric() && value != '_')
            .unwrap_or(true)
    }

    fn bounded_match(request: &str, keyword: &str) -> bool {
        request.match_indices(keyword).any(|(start, _)| {
            let before = request[..start].chars().next_back();
            let after = request[start + keyword.len()..].chars().next();
            is_boundary(before) && is_boundary(after)
        })
    }

    keywords
        .iter()
        .filter(|keyword| {
            let keyword = keyword.as_str();
            if keyword.chars().count() <= 2 {
                return bounded_match(request, keyword);
            }
            if keyword.contains(' ') || keyword.contains('-') {
                return bounded_match(request, keyword);
            }
            if keyword
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return bounded_match(request, keyword);
            }
            request.contains(keyword)
        })
        .cloned()
        .collect()
}

fn feature_delivery_design_terms(request: &str) -> Vec<String> {
    let design_keywords = vec![
        "research".to_string(),
        "spec".to_string(),
        "specification".to_string(),
        "specifications".to_string(),
        "plan".to_string(),
        "planning".to_string(),
        "design".to_string(),
        "requirements".to_string(),
    ];
    let implementation_keywords = vec![
        "implement".to_string(),
        "implementation".to_string(),
        "write code".to_string(),
        "write the full code".to_string(),
        "full code".to_string(),
        "build".to_string(),
        "develop".to_string(),
    ];

    let design_matches = contains_keywords(request, &design_keywords);
    let implementation_matches = contains_keywords(request, &implementation_keywords);
    if design_matches.is_empty() || implementation_matches.is_empty() {
        return Vec::new();
    }

    let mut combined = Vec::new();
    for term in design_matches
        .into_iter()
        .chain(implementation_matches.into_iter())
    {
        if !combined.contains(&term) {
            combined.push(term);
        }
    }
    combined
}

impl RuntimeConsumptionSummary {
    fn as_display(&self) -> String {
        if self.total_snapshots == 0 {
            return "0 snapshots".to_string();
        }

        format!(
            "{} snapshots (bundle={}, bundle_check={}, final={}, latest_kind={}, latest_path={})",
            self.total_snapshots,
            self.bundle_snapshots,
            self.bundle_check_snapshots,
            self.final_snapshots,
            self.latest_kind.as_deref().unwrap_or("none"),
            self.latest_snapshot_path.as_deref().unwrap_or("none")
        )
    }
}

fn count_nonempty_lines(output: &str) -> usize {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .count()
}

fn build_docflow_runtime_evidence() -> (
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionOverview,
) {
    let registry_root = resolve_repo_root()
        .expect("docflow registry evidence should resolve the repo root")
        .display()
        .to_string();
    let registry_output = docflow_cli::run(DocflowCli {
        command: DocflowCommand::Registry(RegistryScanArgs {
            root: registry_root.clone(),
            exclude_globs: vec![],
        }),
    });
    let check_output = docflow_cli::run(DocflowCli {
        command: DocflowCommand::Check(DocflowCheckArgs {
            root: None,
            profile: "active-canon".to_string(),
            files: Vec::new(),
        }),
    });
    let readiness_output = docflow_cli::run(DocflowCli {
        command: DocflowCommand::ReadinessCheck(DocflowCheckArgs {
            root: None,
            profile: "active-canon".to_string(),
            files: Vec::new(),
        }),
    });
    let proof_output = docflow_cli::run(DocflowCli {
        command: DocflowCommand::Proofcheck(DocflowProofcheckArgs {
            layer: None,
            profile: "active-canon".to_string(),
        }),
    });

    let registry_rows = count_nonempty_lines(&registry_output);
    let check_rows = count_nonempty_lines(&check_output);
    let readiness_rows = count_nonempty_lines(&readiness_output);
    let proof_ok = proof_output.contains("✅ OK: proofcheck");
    let proof_blocking = !proof_ok;

    let registry = RuntimeConsumptionEvidence {
        surface: format!("vida docflow registry --root {}", registry_root),
        ok: registry_rows > 0 && !registry_output.contains("\"artifact_type\":\"inventory_error\""),
        row_count: registry_rows,
        verdict: None,
        artifact_path: None,
        output: registry_output,
    };
    let check = RuntimeConsumptionEvidence {
        surface: "vida docflow check --profile active-canon".to_string(),
        ok: check_output.trim().is_empty(),
        row_count: check_rows,
        verdict: None,
        artifact_path: None,
        output: check_output,
    };
    let readiness = RuntimeConsumptionEvidence {
        surface: "vida docflow readiness-check --profile active-canon".to_string(),
        ok: readiness_output.trim().is_empty(),
        row_count: readiness_rows,
        verdict: Some(if readiness_output.trim().is_empty() {
            "ready".to_string()
        } else {
            "blocked".to_string()
        }),
        artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
        output: readiness_output,
    };
    let proof = RuntimeConsumptionEvidence {
        surface: "vida docflow proofcheck --profile active-canon".to_string(),
        ok: proof_ok,
        row_count: count_nonempty_lines(&proof_output),
        verdict: None,
        artifact_path: None,
        output: proof_output,
    };
    let overview = RuntimeConsumptionOverview {
        surface: "vida taskflow direct runtime-consumption overview".to_string(),
        ok: registry.ok && check.ok && readiness.ok && proof.ok,
        registry_rows,
        check_rows,
        readiness_rows,
        proof_blocking,
    };

    (registry, check, readiness, proof, overview)
}

fn build_docflow_runtime_verdict(
    registry: &RuntimeConsumptionEvidence,
    check: &RuntimeConsumptionEvidence,
    readiness: &RuntimeConsumptionEvidence,
    proof: &RuntimeConsumptionEvidence,
) -> RuntimeConsumptionDocflowVerdict {
    let mut blockers = Vec::new();
    if !registry.ok {
        blockers.push("missing_docflow_activation".to_string());
    }
    if !check.ok {
        blockers.push("docflow_check_blocking".to_string());
    }
    if !readiness.ok {
        blockers.push("missing_readiness_verdict".to_string());
    }
    if !proof.ok {
        blockers.push("missing_proof_verdict".to_string());
    }

    RuntimeConsumptionDocflowVerdict {
        status: if blockers.is_empty() {
            "pass".to_string()
        } else {
            "block".to_string()
        },
        ready: blockers.is_empty(),
        blockers,
        proof_surfaces: vec![
            registry.surface.clone(),
            check.surface.clone(),
            readiness.surface.clone(),
            proof.surface.clone(),
        ],
    }
}

fn build_runtime_closure_admission(
    bundle_check: &TaskflowConsumeBundleCheck,
    docflow_verdict: &RuntimeConsumptionDocflowVerdict,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> RuntimeConsumptionClosureAdmission {
    let mut blockers = Vec::new();
    if !bundle_check.ok {
        blockers.push("missing_closure_proof".to_string());
    }
    if !docflow_verdict.ready {
        blockers.extend(docflow_verdict.blockers.iter().cloned());
    }
    if !docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"))
    {
        blockers.push("missing_closure_proof".to_string());
    }
    let has_readiness_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("readiness-check"));
    let has_proof_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"));
    if !(has_readiness_surface && has_proof_surface) {
        blockers.push("restore_reconcile_not_green".to_string());
    }
    if role_selection.execution_plan["status"] == "design_first" {
        blockers.push("pending_design_packet".to_string());
    }
    blockers.sort();
    blockers.dedup();

    let mut proof_surfaces = vec!["vida taskflow consume bundle check".to_string()];
    proof_surfaces.extend(docflow_verdict.proof_surfaces.iter().cloned());

    RuntimeConsumptionClosureAdmission {
        status: if blockers.is_empty() {
            "admit".to_string()
        } else {
            "block".to_string()
        },
        admitted: blockers.is_empty(),
        blockers,
        proof_surfaces,
    }
}

fn build_taskflow_handoff_plan(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let execution_plan = &role_selection.execution_plan;
    let development_flow = &execution_plan["development_flow"];
    let dispatch_contract = &development_flow["dispatch_contract"];
    if execution_plan["status"] == "design_first" {
        return serde_json::json!({
            "status": "spec_first_handoff_required",
            "tracked_flow_bootstrap": execution_plan["tracked_flow_bootstrap"],
            "design_packet_activation": execution_plan["codex_runtime_assignment"],
            "post_design_activation_chain": {
                "implementer": dispatch_contract["implementer_activation"],
                "coach": dispatch_contract["coach_activation"],
                "verifier": dispatch_contract["verifier_activation"],
                "escalation": dispatch_contract["escalation_activation"],
            },
            "handoff_ready": true,
        });
    }

    serde_json::json!({
        "status": "execution_handoff_ready",
        "activation_chain": {
            "implementer": dispatch_contract["implementer_activation"],
            "coach": dispatch_contract["coach_activation"],
            "verifier": dispatch_contract["verifier_activation"],
            "escalation": dispatch_contract["escalation_activation"],
        },
        "runtime_assignment": execution_plan["codex_runtime_assignment"],
        "lane_sequence": development_flow["lane_sequence"],
        "handoff_ready": true,
    })
}

fn runtime_consumption_run_id(role_selection: &RuntimeConsumptionLaneSelection) -> String {
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["spec_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    let slug = infer_feature_request_slug(&role_selection.request);
    if slug.trim().is_empty() {
        "runtime-consume-request".to_string()
    } else {
        format!("runtime-{slug}")
    }
}

fn build_runtime_consumption_dispatch_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
) -> crate::state_store::RunGraphDispatchReceipt {
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let run_id = json_string(run_graph_bootstrap.get("run_id"))
        .unwrap_or_else(|| runtime_consumption_run_id(role_selection));
    let latest_status = run_graph_bootstrap
        .get("latest_status")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let dispatch_target = json_string(latest_status.get("next_node"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| role_selection.selected_role.clone());
    let selected_backend = json_string(latest_status.get("selected_backend"))
        .or_else(|| {
            json_string(
                role_selection.execution_plan["development_flow"]["implementation"]
                    .get("subagents"),
            )
        })
        .filter(|value| !value.is_empty());
    let is_taskflow_pack = matches!(dispatch_target.as_str(), "spec-pack" | "work-pool-pack");
    let dispatch_surface = match dispatch_target.as_str() {
        "spec-pack" => Some("vida taskflow bootstrap-spec".to_string()),
        "work-pool-pack" => Some("vida taskflow task create".to_string()),
        _ if json_bool(run_graph_bootstrap.get("handoff_ready"), false) => {
            Some("vida agent-init".to_string())
        }
        _ => None,
    };
    let codex_runtime_assignment = &role_selection.execution_plan["development_flow"]
        ["implementation"]["codex_runtime_assignment"];
    let activation_agent_type = if is_taskflow_pack {
        None
    } else {
        json_string(codex_runtime_assignment.get("activation_agent_type"))
    };
    let activation_runtime_role = if is_taskflow_pack {
        None
    } else {
        json_string(codex_runtime_assignment.get("activation_runtime_role"))
    };

    crate::state_store::RunGraphDispatchReceipt {
        run_id,
        dispatch_target,
        dispatch_status: if json_bool(run_graph_bootstrap.get("handoff_ready"), false) {
            "routed".to_string()
        } else {
            "blocked".to_string()
        },
        dispatch_kind: if is_taskflow_pack {
            "taskflow_pack".to_string()
        } else {
            "agent_lane".to_string()
        },
        dispatch_surface,
        dispatch_command: None,
        dispatch_packet_path: None,
        dispatch_result_path: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    }
}

fn downstream_activation_fields(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let dispatch_contract = &role_selection.execution_plan["development_flow"]["dispatch_contract"];
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" => (
            "taskflow_pack".to_string(),
            match dispatch_target {
                "spec-pack" => Some("vida taskflow bootstrap-spec".to_string()),
                "work-pool-pack" => Some("vida taskflow task create".to_string()),
                "dev-pack" => Some("vida taskflow task create".to_string()),
                _ => None,
            },
            None,
            None,
        ),
        "implementer" => (
            "agent_lane".to_string(),
            Some("vida agent-init".to_string()),
            json_string(dispatch_contract["implementer_activation"].get("activation_agent_type")),
            json_string(dispatch_contract["implementer_activation"].get("activation_runtime_role")),
        ),
        "coach" => (
            "agent_lane".to_string(),
            Some("vida agent-init".to_string()),
            json_string(dispatch_contract["coach_activation"].get("activation_agent_type")),
            json_string(dispatch_contract["coach_activation"].get("activation_runtime_role")),
        ),
        "verification" => (
            "agent_lane".to_string(),
            Some("vida agent-init".to_string()),
            json_string(dispatch_contract["verifier_activation"].get("activation_agent_type")),
            json_string(dispatch_contract["verifier_activation"].get("activation_runtime_role")),
        ),
        "escalation" => (
            "agent_lane".to_string(),
            Some("vida agent-init".to_string()),
            json_string(dispatch_contract["escalation_activation"].get("activation_agent_type")),
            json_string(dispatch_contract["escalation_activation"].get("activation_runtime_role")),
        ),
        "closure" => ("closure".to_string(), None, None, None),
        _ => (
            "agent_lane".to_string(),
            Some("vida agent-init".to_string()),
            None,
            None,
        ),
    }
}

fn build_downstream_dispatch_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<crate::state_store::RunGraphDispatchReceipt> {
    let dispatch_target = receipt.downstream_dispatch_target.clone()?;
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(role_selection, &dispatch_target);
    Some(crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: if receipt.downstream_dispatch_ready {
            "routed".to_string()
        } else {
            "blocked".to_string()
        },
        dispatch_kind,
        dispatch_surface,
        dispatch_command: receipt.downstream_dispatch_command.clone(),
        dispatch_packet_path: receipt.downstream_dispatch_packet_path.clone(),
        dispatch_result_path: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend: receipt.selected_backend.clone(),
        recorded_at,
    })
}

fn root_receipt_fields_from_downstream_step(
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    step_receipt: &crate::state_store::RunGraphDispatchReceipt,
) {
    root_receipt.downstream_dispatch_target = step_receipt.downstream_dispatch_target.clone();
    root_receipt.downstream_dispatch_command = step_receipt.downstream_dispatch_command.clone();
    root_receipt.downstream_dispatch_note = step_receipt.downstream_dispatch_note.clone();
    root_receipt.downstream_dispatch_ready = step_receipt.downstream_dispatch_ready;
    root_receipt.downstream_dispatch_blockers = step_receipt.downstream_dispatch_blockers.clone();
    root_receipt.downstream_dispatch_packet_path =
        step_receipt.downstream_dispatch_packet_path.clone();
    root_receipt.downstream_dispatch_status = step_receipt.downstream_dispatch_status.clone();
    root_receipt.downstream_dispatch_result_path =
        step_receipt.downstream_dispatch_result_path.clone();
}

fn write_runtime_downstream_dispatch_trace(
    state_root: &Path,
    run_id: &str,
    trace: &[serde_json::Value],
) -> Result<String, String> {
    let trace_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-traces");
    std::fs::create_dir_all(&trace_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-traces directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let trace_path = trace_dir.join(format!("{run_id}-{ts}.json",));
    let body = serde_json::json!({
        "artifact_kind": "runtime_downstream_dispatch_trace",
        "run_id": run_id,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "step_count": trace.len(),
        "steps": trace,
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch trace: {error}"))?;
    std::fs::write(&trace_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch trace: {error}"))?;
    Ok(trace_path.display().to_string())
}

fn runtime_dispatch_command_for_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Option<String> {
    match dispatch_target {
        "spec-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"].get("bootstrap_command"),
        ),
        "work-pool-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                .get("create_command"),
        ),
        "dev-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                .get("create_command"),
        ),
        _ => Some("vida agent-init".to_string()),
    }
}

fn runtime_dispatch_packet_kind(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> &'static str {
    match receipt.dispatch_kind.as_str() {
        "taskflow_pack" => "tracked_flow_packet",
        _ => match receipt.activation_runtime_role.as_deref() {
            Some("coach") => "coach_review_packet",
            Some("verifier") => "verifier_proof_packet",
            Some("solution_architect") => "escalation_packet",
            _ => "delivery_task_packet",
        },
    }
}

fn derive_downstream_dispatch_preview(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Vec<String>,
) {
    match receipt.dispatch_target.as_str() {
        "spec-pack" => {
            let blockers = vec![
                "pending_design_finalize".to_string(),
                "pending_spec_task_close".to_string(),
            ];
            (
                Some("work-pool-pack".to_string()),
                json_string(
                    role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                        .get("create_command"),
                ),
                Some(
                    "after the design document is finalized and the spec task is closed, create the tracked work-pool packet"
                        .to_string(),
                ),
                false,
                blockers,
            )
        }
        "work-pool-pack" => (
            Some("dev-pack".to_string()),
            json_string(
                role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                    .get("create_command"),
            ),
            Some(
                "after the work-pool packet is shaped, create the bounded dev packet for delegated implementation"
                    .to_string(),
            ),
            receipt.dispatch_status == "executed",
            if receipt.dispatch_status == "executed" {
                Vec::new()
            } else {
                vec!["pending_work_pool_shape".to_string()]
            },
        ),
        "dev-pack" => (
            Some("implementer".to_string()),
            Some("vida agent-init".to_string()),
            Some(
                "after the dev packet is created, activate the selected implementer lane for bounded execution"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
        _ if receipt.dispatch_kind == "agent_lane" => {
            let coach_required = json_bool(
                role_selection.execution_plan["development_flow"]["implementation"]
                    .get("coach_required"),
                false,
            );
            let verification_required = json_bool(
                role_selection.execution_plan["development_flow"]["implementation"]
                    .get("independent_verification_required"),
                false,
            );
            if receipt.activation_runtime_role.as_deref() == Some("worker") && coach_required {
                (
                    Some("coach".to_string()),
                    Some("vida agent-init".to_string()),
                    Some(
                        "after implementer execution evidence exists, activate the coach lane for bounded review"
                            .to_string(),
                    ),
                    false,
                    vec!["pending_implementation_evidence".to_string()],
                )
            } else if verification_required {
                (
                    Some("verification".to_string()),
                    Some("vida agent-init".to_string()),
                    Some(
                        "after review-clean implementation evidence exists, activate the verifier lane for independent proof"
                            .to_string(),
                    ),
                    false,
                    vec!["pending_review_clean_evidence".to_string()],
                )
            } else {
                (
                    Some("closure".to_string()),
                    None,
                    Some(
                        "no additional downstream lane is required by the current execution plan after this handoff"
                            .to_string(),
                    ),
                    true,
                    Vec::new(),
                )
            }
        }
        _ => (None, None, None, false, Vec::new()),
    }
}

fn refresh_downstream_dispatch_preview(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let (
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
    ) = derive_downstream_dispatch_preview(role_selection, receipt);
    receipt.downstream_dispatch_target = downstream_dispatch_target;
    receipt.downstream_dispatch_command = downstream_dispatch_command;
    receipt.downstream_dispatch_note = downstream_dispatch_note;
    receipt.downstream_dispatch_ready = downstream_dispatch_ready;
    receipt.downstream_dispatch_blockers = downstream_dispatch_blockers;
    receipt.downstream_dispatch_status = None;
    receipt.downstream_dispatch_result_path = None;
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_last_target = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        state_root,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    Ok(())
}

fn write_runtime_dispatch_packet(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    taskflow_handoff_plan: &serde_json::Value,
    run_graph_bootstrap: &serde_json::Value,
) -> Result<String, String> {
    let packet_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-packets");
    std::fs::create_dir_all(&packet_dir)
        .map_err(|error| format!("Failed to create dispatch-packets directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", receipt.run_id));
    let body = serde_json::json!({
        "packet_kind": "runtime_dispatch_packet",
        "packet_template_kind": runtime_dispatch_packet_kind(receipt),
        "recorded_at": receipt.recorded_at,
        "run_id": receipt.run_id,
        "dispatch_target": receipt.dispatch_target,
        "dispatch_status": receipt.dispatch_status,
        "dispatch_kind": receipt.dispatch_kind,
        "dispatch_surface": receipt.dispatch_surface,
        "dispatch_command": runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target),
        "activation_agent_type": receipt.activation_agent_type,
        "activation_runtime_role": receipt.activation_runtime_role,
        "selected_backend": receipt.selected_backend,
        "request_text": role_selection.request,
        "role_selection": {
            "selected_role": role_selection.selected_role,
            "conversational_mode": role_selection.conversational_mode,
            "tracked_flow_entry": role_selection.tracked_flow_entry,
            "confidence": role_selection.confidence,
        },
        "role_selection_full": role_selection,
        "taskflow_handoff_plan": taskflow_handoff_plan,
        "run_graph_bootstrap": run_graph_bootstrap,
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode dispatch packet: {error}"))?;
    std::fs::write(&packet_path, encoded)
        .map_err(|error| format!("Failed to write dispatch packet: {error}"))?;
    Ok(packet_path.display().to_string())
}

async fn execute_runtime_dispatch_handoff(
    state_root: &Path,
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let project_root =
        infer_project_root_from_state_root(state_root).unwrap_or(std::env::current_dir().map_err(
            |error| format!("Failed to resolve current directory for dispatch execution: {error}"),
        )?);
    match receipt.dispatch_target.as_str() {
        "spec-pack" => execute_taskflow_bootstrap_spec_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
        ),
        "work-pool-pack" => execute_work_packet_create_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
            "work_pool_task",
        ),
        "dev-pack" => execute_work_packet_create_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
            "dev_task",
        ),
        "closure" => Ok(serde_json::json!({
            "surface": "vida taskflow closure-preview",
            "status": "ok",
            "closure_ready": true,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "note": "runtime downstream scheduler reached closure without additional lane activation",
        })),
        _ => {
            let bundle =
                crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload(store)
                    .await?;
            let project_activation_view = build_project_activator_view(&project_root);
            let init_view = merge_project_activation_into_init_view(
                bundle.agent_init_view,
                &project_activation_view,
            );
            Ok(serde_json::json!({
                "surface": "vida agent-init",
                "status": "ok",
                "init": init_view,
                "selection": serde_json::to_value(role_selection)
                    .expect("lane selection should serialize"),
                "runtime_bundle_summary": {
                    "bundle_id": bundle.metadata["bundle_id"],
                    "activation_source": bundle.activation_source,
                    "vida_root": bundle.vida_root,
                    "state_dir": store.root().display().to_string(),
                },
            }))
        }
    }
}

fn write_runtime_dispatch_result(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let result_path = result_dir.join(format!("{}-{ts}.json", receipt.run_id));
    let encoded = serde_json::to_string_pretty(body)
        .map_err(|error| format!("Failed to encode dispatch result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write dispatch result: {error}"))?;
    Ok(result_path.display().to_string())
}

async fn execute_and_record_dispatch_receipt(
    state_root: &Path,
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let execution_result =
        execute_runtime_dispatch_handoff(state_root, store, role_selection, receipt).await?;
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    receipt.dispatch_status = "executed".to_string();
    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        if let Ok(status) = store.run_graph_status(&run_id).await {
            let executed_status =
                apply_first_handoff_execution_to_run_graph_status(&status, receipt);
            store
                .record_run_graph_status(&executed_status)
                .await
                .map_err(|error| format!("Failed to record executed run-graph status: {error}"))?;
        }
    }
    Ok(())
}

async fn execute_downstream_dispatch_chain(
    state_root: &Path,
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if root_receipt.dispatch_status != "executed" || !root_receipt.downstream_dispatch_ready {
        return Ok(());
    }

    let mut downstream_source = root_receipt.clone();
    let mut downstream_trace = Vec::new();
    for _ in 0..4 {
        let Some(mut downstream_receipt) =
            build_downstream_dispatch_receipt(role_selection, &downstream_source)
        else {
            break;
        };
        let allow_downstream_execution = downstream_receipt.dispatch_kind != "taskflow_pack"
            || infer_project_root_from_state_root(state_root).is_some();
        if downstream_receipt.dispatch_status != "routed" || !allow_downstream_execution {
            root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
            break;
        }

        execute_and_record_dispatch_receipt(
            state_root,
            store,
            role_selection,
            run_graph_bootstrap,
            &mut downstream_receipt,
        )
        .await
        .map_err(|error| {
            format!("Failed to execute downstream runtime dispatch handoff: {error}")
        })?;

        let (next_target, next_command, next_note, next_ready, next_blockers) =
            derive_downstream_dispatch_preview(role_selection, &downstream_receipt);
        downstream_receipt.downstream_dispatch_target = next_target;
        downstream_receipt.downstream_dispatch_command = next_command;
        downstream_receipt.downstream_dispatch_note = next_note;
        downstream_receipt.downstream_dispatch_ready = next_ready;
        downstream_receipt.downstream_dispatch_blockers = next_blockers;
        downstream_receipt.downstream_dispatch_packet_path =
            write_runtime_downstream_dispatch_packet(
                state_root,
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
            )
            .map_err(|error| {
                format!("Failed to write chained downstream runtime dispatch packet: {error}")
            })?;
        downstream_receipt.downstream_dispatch_status = Some("executed".to_string());
        downstream_receipt.downstream_dispatch_result_path =
            downstream_receipt.dispatch_result_path.clone();
        if let Some(packet_path) = downstream_receipt
            .downstream_dispatch_packet_path
            .as_deref()
        {
            write_runtime_downstream_dispatch_packet_at(
                Path::new(packet_path),
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
            )
            .map_err(|error| {
                format!("Failed to refresh chained downstream runtime dispatch packet: {error}")
            })?;
        }

        downstream_trace
            .push(serde_json::to_value(&downstream_receipt).unwrap_or(serde_json::Value::Null));
        root_receipt.downstream_dispatch_executed_count += 1;
        root_receipt.downstream_dispatch_last_target =
            Some(downstream_receipt.dispatch_target.clone());
        root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
        if !downstream_receipt.downstream_dispatch_ready {
            break;
        }
        downstream_source = downstream_receipt;
    }

    if !downstream_trace.is_empty() {
        let trace_path = write_runtime_downstream_dispatch_trace(
            state_root,
            &root_receipt.run_id,
            &downstream_trace,
        )
        .map_err(|error| format!("Failed to write downstream runtime dispatch trace: {error}"))?;
        root_receipt.downstream_dispatch_trace_path = Some(trace_path);
    }
    Ok(())
}

fn downstream_dispatch_packet_body(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> serde_json::Value {
    let body = serde_json::json!({
        "packet_kind": "runtime_downstream_dispatch_packet",
        "recorded_at": receipt.recorded_at,
        "run_id": receipt.run_id,
        "source_dispatch_target": receipt.dispatch_target,
        "source_dispatch_status": receipt.dispatch_status,
        "downstream_dispatch_target": receipt.downstream_dispatch_target,
        "downstream_dispatch_command": receipt.downstream_dispatch_command,
        "downstream_dispatch_note": receipt.downstream_dispatch_note,
        "downstream_dispatch_ready": receipt.downstream_dispatch_ready,
        "downstream_dispatch_blockers": receipt.downstream_dispatch_blockers,
        "downstream_dispatch_status": receipt.downstream_dispatch_status,
        "downstream_dispatch_result_path": receipt.downstream_dispatch_result_path,
        "activation_agent_type": receipt.activation_agent_type,
        "activation_runtime_role": receipt.activation_runtime_role,
        "role_selection_full": role_selection,
        "run_graph_bootstrap": run_graph_bootstrap,
    });
    body
}

fn write_runtime_downstream_dispatch_packet_at(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let body = downstream_dispatch_packet_body(role_selection, run_graph_bootstrap, receipt);
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch packet: {error}"))?;
    std::fs::write(packet_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch packet: {error}"))?;
    Ok(())
}

fn write_runtime_downstream_dispatch_packet(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    let Some(target) = receipt.downstream_dispatch_target.as_deref() else {
        return Ok(None);
    };
    let packet_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-packets");
    std::fs::create_dir_all(&packet_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-packets directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", receipt.run_id));
    write_runtime_downstream_dispatch_packet_at(
        &packet_path,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    let _ = target;
    Ok(Some(packet_path.display().to_string()))
}

fn apply_first_handoff_execution_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    if receipt.dispatch_target == "closure" {
        return crate::state_store::RunGraphStatus {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_class: status.task_class.clone(),
            active_node: "closure".to_string(),
            next_node: None,
            status: "completed".to_string(),
            route_task_class: status.route_task_class.clone(),
            selected_backend: receipt
                .selected_backend
                .clone()
                .unwrap_or_else(|| status.selected_backend.clone()),
            lane_id: "closure_direct".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            policy_gate: status.policy_gate.clone(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: status.checkpoint_kind.clone(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
    }
    let dispatch_target = receipt.dispatch_target.replace('-', "_");
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node: None,
        status: "ready".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: if receipt.dispatch_kind == "taskflow_pack" {
            format!("{dispatch_target}_direct")
        } else {
            format!("{dispatch_target}_lane")
        },
        lifecycle_stage: format!("{dispatch_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state: "none".to_string(),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: "none".to_string(),
        recovery_ready: true,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}

fn fallback_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let conversational_mode = role_selection.conversational_mode.as_deref();
    let route_target = match conversational_mode {
        Some("scope_discussion") => "spec-pack".to_string(),
        Some("pbi_discussion") => "work-pool-pack".to_string(),
        _ if role_selection.execution_plan["status"] == "design_first" => "spec-pack".to_string(),
        _ => role_selection.selected_role.clone(),
    };
    let route_backend =
        json_string(role_selection.execution_plan["default_route"].get("subagents"))
            .or_else(|| {
                json_string(
                    role_selection.execution_plan["development_flow"]["implementation"]
                        .get("subagents"),
                )
            })
            .unwrap_or_else(|| "unknown".to_string());
    crate::state_store::RunGraphStatus {
        run_id: run_id.to_string(),
        task_id: run_id.to_string(),
        task_class: conversational_mode.unwrap_or("implementation").to_string(),
        active_node: if conversational_mode.is_some() {
            role_selection.selected_role.clone()
        } else {
            "planning".to_string()
        },
        next_node: Some(route_target.clone()),
        status: "ready".to_string(),
        route_task_class: if conversational_mode.is_some() {
            route_target.clone()
        } else {
            "implementation".to_string()
        },
        selected_backend: route_backend,
        lane_id: format!("{}_lane", role_selection.selected_role.replace('-', "_")),
        lifecycle_stage: if conversational_mode.is_some() {
            "conversation_active".to_string()
        } else {
            "implementation_dispatch_ready".to_string()
        },
        policy_gate: "not_required".to_string(),
        handoff_state: format!("awaiting_{route_target}"),
        context_state: "sealed".to_string(),
        checkpoint_kind: if conversational_mode.is_some() {
            "conversation_cursor".to_string()
        } else {
            "execution_cursor".to_string()
        },
        resume_target: format!("dispatch.{route_target}"),
        recovery_ready: true,
    }
}

async fn build_runtime_consumption_run_graph_bootstrap(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let run_id = runtime_consumption_run_id(role_selection);
    match crate::taskflow_run_graph::derive_seeded_run_graph_status(
        store,
        &run_id,
        &role_selection.request,
    )
    .await
    {
        Ok(seed_payload) => {
            let seed_payload_json =
                serde_json::to_value(&seed_payload).unwrap_or(serde_json::Value::Null);
            let seed_status_json =
                serde_json::to_value(&seed_payload.status).unwrap_or(serde_json::Value::Null);
            if let Err(error) = store.record_run_graph_status(&seed_payload.status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("record_seed_failed: {error}"),
                });
            }
            let mut latest_status = seed_status_json.clone();
            let mut advanced_payload = serde_json::Value::Null;

            if role_selection.conversational_mode.is_some() {
                match crate::taskflow_run_graph::derive_advanced_run_graph_status(
                    store,
                    seed_payload.status,
                )
                .await
                {
                    Ok(payload) => {
                        let advanced_status_json = serde_json::to_value(&payload.status)
                            .unwrap_or(serde_json::Value::Null);
                        if let Err(error) = store.record_run_graph_status(&payload.status).await {
                            return serde_json::json!({
                                "status": "blocked",
                                "handoff_ready": false,
                                "run_id": run_id,
                                "seed": seed_payload_json,
                                "reason": format!("record_advance_failed: {error}"),
                            });
                        }
                        advanced_payload =
                            serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
                        latest_status = advanced_status_json;
                    }
                    Err(error) => {
                        return serde_json::json!({
                            "status": "blocked",
                            "handoff_ready": false,
                            "run_id": run_id,
                            "seed": seed_payload_json,
                            "reason": format!("advance_failed: {error}"),
                        });
                    }
                }
            }

            serde_json::json!({
                "status": if advanced_payload.is_null() {
                    "seeded"
                } else {
                    "seeded_and_advanced"
                },
                "handoff_ready": true,
                "run_id": run_id,
                "seed": seed_payload_json,
                "advanced": advanced_payload,
                "latest_status": if advanced_payload.is_null() {
                    seed_status_json
                } else {
                    latest_status
                },
            })
        }
        Err(error) => {
            let status = fallback_runtime_consumption_run_graph_status(role_selection, &run_id);
            let latest_status = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
            if let Err(record_error) = store.record_run_graph_status(&status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("seed_failed: {error}; fallback_record_failed: {record_error}"),
                });
            }
            serde_json::json!({
                "status": "seeded_and_advanced",
                "handoff_ready": true,
                "run_id": run_id,
                "seed": serde_json::Value::Null,
                "advanced": serde_json::Value::Null,
                "latest_status": latest_status,
                "fallback_reason": format!("seed_failed: {error}"),
            })
        }
    }
}

fn blocking_lane_selection(request: &str, error: &str) -> RuntimeConsumptionLaneSelection {
    RuntimeConsumptionLaneSelection {
        ok: false,
        activation_source: "state_store".to_string(),
        selection_mode: "unresolved".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: request.to_string(),
        selected_role: "orchestrator".to_string(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "blocked".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: serde_json::Value::Null,
        execution_plan: serde_json::json!({
            "status": "blocked",
            "reason": error,
        }),
        reason: error.to_string(),
    }
}

fn blocking_docflow_activation(error: &str) -> RuntimeConsumptionDocflowActivation {
    RuntimeConsumptionDocflowActivation {
        activated: false,
        runtime_family: "docflow".to_string(),
        owner_runtime: "taskflow".to_string(),
        evidence: serde_json::json!({
            "error": error,
            "overview": {
                "surface": "vida taskflow direct runtime-consumption overview",
                "ok": false,
                "registry_rows": 0,
                "check_rows": 0,
                "readiness_rows": 0,
                "proof_blocking": true
            },
            "registry": {
                "surface": "vida docflow registry --root <repo-root>",
                "ok": false,
                "row_count": 0,
                "output": ""
            },
            "check": {
                "surface": "vida docflow check --profile active-canon",
                "ok": false,
                "row_count": 0,
                "output": error
            },
            "readiness": {
                "surface": "vida docflow readiness-check --profile active-canon",
                "ok": false,
                "row_count": 0,
                "verdict": "blocked",
                "artifact_path": "vida/config/docflow-readiness.current.jsonl",
                "output": error
            },
            "proof": {
                "surface": "vida docflow proofcheck --profile active-canon",
                "ok": false,
                "row_count": 0,
                "output": error
            }
        }),
    }
}

fn emit_taskflow_consume_final_json(
    store: &StateStore,
    payload: &TaskflowDirectConsumptionPayload,
) -> Result<(), String> {
    let snapshot = serde_json::json!({
        "surface": "vida taskflow consume final",
        "payload": payload,
    });
    let snapshot_path = write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "surface": "vida taskflow consume final",
            "payload": payload,
            "snapshot_path": snapshot_path,
        }))
        .expect("consume final should render as json")
    );
    Ok(())
}

fn write_runtime_consumption_snapshot(
    state_root: &Path,
    prefix: &str,
    payload: &serde_json::Value,
) -> Result<String, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    std::fs::create_dir_all(&snapshot_dir)
        .map_err(|error| format!("Failed to create runtime-consumption directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let snapshot_path = snapshot_dir.join(format!("{prefix}-{ts}.json"));
    let body = serde_json::to_string_pretty(payload)
        .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?;
    std::fs::write(&snapshot_path, body)
        .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    Ok(snapshot_path.display().to_string())
}

fn parse_taskflow_consume_continue_args(
    args: &[String],
) -> Result<(bool, Option<String>, Option<String>, Option<String>), String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut dispatch_packet_path = None;
    let mut downstream_packet_path = None;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--dispatch-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                dispatch_packet_path = Some(value.clone());
                index += 2;
            }
            "--downstream-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                downstream_packet_path = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]"
                ));
            }
        }
    }
    if dispatch_packet_path.is_some() && downstream_packet_path.is_some() {
        return Err(
            "Use only one packet source: --dispatch-packet <path> or --downstream-packet <path>"
                .to_string(),
        );
    }
    Ok((
        as_json,
        run_id,
        dispatch_packet_path,
        downstream_packet_path,
    ))
}

fn parse_taskflow_consume_advance_args(
    args: &[String],
) -> Result<(bool, Option<String>, usize), String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut max_rounds = 8usize;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--max-rounds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                max_rounds = value
                    .parse::<usize>()
                    .map_err(|_| "Expected a positive integer for --max-rounds".to_string())?;
                if max_rounds == 0 {
                    return Err("--max-rounds must be greater than zero".to_string());
                }
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                ));
            }
        }
    }
    Ok((as_json, run_id, max_rounds))
}

fn read_dispatch_packet(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted dispatch packet: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted dispatch packet: {error}"))
}

async fn resume_inputs_from_downstream_packet(
    store: &StateStore,
    requested_run_id: Option<&str>,
    packet_path: &str,
) -> Result<
    (
        crate::state_store::RunGraphDispatchReceipt,
        String,
        RuntimeConsumptionLaneSelection,
        serde_json::Value,
    ),
    String,
> {
    let packet = read_dispatch_packet(packet_path)?;
    let run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?;
    if let Some(requested_run_id) = requested_run_id {
        if requested_run_id != run_id {
            return Err(format!(
                "Requested run_id `{requested_run_id}` does not match persisted downstream dispatch packet run_id `{run_id}`"
            ));
        }
    }
    let root_receipt = match store.run_graph_dispatch_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => {
            return Err(format!(
                "No persisted run-graph dispatch receipt exists for run_id `{run_id}`"
            ))
        }
        Err(error) => {
            return Err(format!(
                "Failed to read persisted run-graph dispatch receipt: {error}"
            ))
        }
    };
    let role_selection: RuntimeConsumptionLaneSelection = serde_json::from_value(
        packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        format!("Failed to decode role_selection from downstream dispatch packet: {error}")
    })?;
    let dispatch_target = packet
        .get("downstream_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Persisted downstream dispatch packet is missing downstream_dispatch_target".to_string()
        })?;
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(&role_selection, dispatch_target);
    let dispatch_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let dispatch_command = packet
        .get("downstream_dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_note = packet
        .get("downstream_dispatch_note")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_blockers = packet
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.to_string(),
        dispatch_target: dispatch_target.to_string(),
        dispatch_status: if dispatch_ready {
            "routed".to_string()
        } else {
            "blocked".to_string()
        },
        dispatch_kind,
        dispatch_surface,
        dispatch_command,
        dispatch_packet_path: Some(packet_path.to_string()),
        dispatch_result_path: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers,
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: packet
            .get("downstream_dispatch_status")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_result_path: packet
            .get("downstream_dispatch_result_path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend: root_receipt.selected_backend.clone(),
        recorded_at,
    };
    Ok((
        receipt,
        packet_path.to_string(),
        role_selection,
        packet
            .get("run_graph_bootstrap")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    ))
}

async fn maybe_resume_inputs_from_ready_downstream_packet(
    store: &StateStore,
    requested_run_id: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<
    Option<(
        crate::state_store::RunGraphDispatchReceipt,
        String,
        RuntimeConsumptionLaneSelection,
        serde_json::Value,
    )>,
    String,
> {
    let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let packet = read_dispatch_packet(packet_path)?;
    let packet_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !packet_ready {
        return Ok(None);
    }
    resume_inputs_from_downstream_packet(store, requested_run_id, packet_path)
        .await
        .map(Some)
}

async fn resolve_runtime_consumption_resume_inputs(
    store: &StateStore,
    requested_run_id: Option<&str>,
    requested_dispatch_packet_path: Option<&str>,
    requested_downstream_packet_path: Option<&str>,
) -> Result<
    (
        crate::state_store::RunGraphDispatchReceipt,
        String,
        RuntimeConsumptionLaneSelection,
        serde_json::Value,
    ),
    String,
> {
    let dispatch_packet = if let Some(packet_path) = requested_dispatch_packet_path {
        let packet = read_dispatch_packet(packet_path)?;
        let run_id = packet
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Persisted dispatch packet is missing run_id".to_string())?;
        if let Some(requested_run_id) = requested_run_id {
            if requested_run_id != run_id {
                return Err(format!(
                    "Requested run_id `{requested_run_id}` does not match persisted dispatch packet run_id `{run_id}`"
                ));
            }
        }
        let receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                return Err(format!(
                    "No persisted run-graph dispatch receipt exists for run_id `{run_id}`"
                ))
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ))
            }
        };
        (receipt, packet_path.to_string(), packet)
    } else if let Some(packet_path) = requested_downstream_packet_path {
        return resume_inputs_from_downstream_packet(store, requested_run_id, packet_path).await;
    } else if let Some(run_id) = requested_run_id {
        let receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                return Err(format!(
                    "No persisted run-graph dispatch receipt exists for run_id `{run_id}`"
                ))
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ))
            }
        };
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, requested_run_id, &receipt)
                .await?
        {
            return Ok(resume);
        }
        let packet_path = receipt.dispatch_packet_path.clone().ok_or_else(|| {
            "Persisted dispatch receipt is missing dispatch_packet_path".to_string()
        })?;
        let packet = read_dispatch_packet(&packet_path)?;
        (receipt, packet_path, packet)
    } else {
        let receipt = match store.latest_run_graph_dispatch_receipt().await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                return Err("No persisted run-graph dispatch receipt is available".to_string())
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ))
            }
        };
        if let Some(resume) =
            maybe_resume_inputs_from_ready_downstream_packet(store, requested_run_id, &receipt)
                .await?
        {
            return Ok(resume);
        }
        let packet_path = receipt.dispatch_packet_path.clone().ok_or_else(|| {
            "Latest persisted dispatch receipt is missing dispatch_packet_path".to_string()
        })?;
        let packet = read_dispatch_packet(&packet_path)?;
        (receipt, packet_path, packet)
    };

    let (dispatch_receipt, dispatch_packet_path, dispatch_packet) = dispatch_packet;
    let role_selection: RuntimeConsumptionLaneSelection = serde_json::from_value(
        dispatch_packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| format!("Failed to decode role_selection from dispatch packet: {error}"))?;
    let run_graph_bootstrap = dispatch_packet
        .get("run_graph_bootstrap")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok((
        dispatch_receipt,
        dispatch_packet_path,
        role_selection,
        run_graph_bootstrap,
    ))
}

async fn run_taskflow_consume_resume_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    requested_dispatch_packet_path: Option<String>,
    requested_downstream_packet_path: Option<String>,
    surface_name: &str,
    emit_output: bool,
) -> ExitCode {
    match open_existing_state_store_with_retry(state_dir).await {
        Ok(store) => {
            let mut dispatch_receipt;
            let dispatch_packet_path;
            let role_selection;
            let run_graph_bootstrap;
            match resolve_runtime_consumption_resume_inputs(
                &store,
                requested_run_id.as_deref(),
                requested_dispatch_packet_path.as_deref(),
                requested_downstream_packet_path.as_deref(),
            )
            .await
            {
                Ok((receipt, packet_path, selection, bootstrap)) => {
                    dispatch_receipt = receipt;
                    dispatch_packet_path = packet_path;
                    role_selection = selection;
                    run_graph_bootstrap = bootstrap;
                }
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            }
            if dispatch_receipt.dispatch_status == "routed" {
                let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                    != "taskflow_pack"
                    || infer_project_root_from_state_root(store.root()).is_some();
                if allow_taskflow_pack_execution {
                    if let Err(error) = execute_and_record_dispatch_receipt(
                        store.root(),
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to execute resumed runtime dispatch handoff: {error}");
                        return ExitCode::from(1);
                    }
                    if let Err(error) = refresh_downstream_dispatch_preview(
                        store.root(),
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    ) {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
            }
            if let Err(error) = execute_downstream_dispatch_chain(
                store.root(),
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            if let Err(error) = store
                .record_run_graph_dispatch_receipt(&dispatch_receipt)
                .await
            {
                eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                return ExitCode::from(1);
            }
            let resume_snapshot = serde_json::json!({
                "surface": surface_name,
                "source_run_id": dispatch_receipt.run_id,
                "source_dispatch_packet_path": dispatch_packet_path,
                "dispatch_receipt": &dispatch_receipt,
            });
            let snapshot_path =
                match write_runtime_consumption_snapshot(store.root(), "final", &resume_snapshot) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            if !emit_output {
                return ExitCode::SUCCESS;
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": surface_name,
                        "source_run_id": dispatch_receipt.run_id,
                        "source_dispatch_packet_path": dispatch_packet_path,
                        "dispatch_receipt": dispatch_receipt,
                        "snapshot_path": snapshot_path,
                    }))
                    .expect("resume command should render as json")
                );
            } else {
                print_surface_header(RenderMode::Plain, surface_name);
                print_surface_line(RenderMode::Plain, "source run", &dispatch_receipt.run_id);
                print_surface_line(RenderMode::Plain, "source packet", &dispatch_packet_path);
                print_surface_line(RenderMode::Plain, "snapshot path", &snapshot_path);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_taskflow_consume_advance_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    max_rounds: usize,
) -> ExitCode {
    let mut rounds = 0usize;
    let mut last_result: Option<(String, crate::state_store::RunGraphDispatchReceipt, String)> =
        None;

    while rounds < max_rounds {
        let before_status = match open_existing_state_store_with_retry(state_dir.clone()).await {
            Ok(store) => match resolve_runtime_consumption_resume_inputs(
                &store,
                requested_run_id.as_deref(),
                None,
                None,
            )
            .await
            {
                Ok((receipt, packet_path, _, _)) => Some((receipt, packet_path)),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let exit = run_taskflow_consume_resume_command(
            state_dir.clone(),
            true,
            requested_run_id.clone(),
            None,
            None,
            "vida taskflow consume advance",
            false,
        )
        .await;
        if exit != ExitCode::SUCCESS {
            return exit;
        }

        let store = match open_existing_state_store_with_retry(state_dir.clone()).await {
            Ok(store) => store,
            Err(error) => {
                eprintln!("Failed to reopen authoritative state store after advance: {error}");
                return ExitCode::from(1);
            }
        };
        let after_receipt = match store.latest_run_graph_dispatch_receipt().await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                eprintln!("No persisted run-graph dispatch receipt is available after advance");
                return ExitCode::from(1);
            }
            Err(error) => {
                eprintln!(
                    "Failed to read persisted run-graph dispatch receipt after advance: {error}"
                );
                return ExitCode::from(1);
            }
        };
        let after_packet_path = after_receipt
            .dispatch_packet_path
            .clone()
            .or_else(|| after_receipt.downstream_dispatch_packet_path.clone())
            .unwrap_or_else(|| "none".to_string());
        let snapshot_path = match runtime_consumption_summary(store.root()) {
            Ok(summary) => summary
                .latest_snapshot_path
                .unwrap_or_else(|| "none".to_string()),
            Err(_) => "none".to_string(),
        };
        last_result = Some((
            after_packet_path.clone(),
            after_receipt.clone(),
            snapshot_path,
        ));
        rounds += 1;

        let progressed = match before_status {
            Some((before_receipt, before_packet_path)) => {
                before_packet_path != after_packet_path
                    || before_receipt.dispatch_status != after_receipt.dispatch_status
                    || before_receipt.downstream_dispatch_target
                        != after_receipt.downstream_dispatch_target
                    || before_receipt.downstream_dispatch_executed_count
                        != after_receipt.downstream_dispatch_executed_count
            }
            None => true,
        };

        let has_more_ready_work = after_receipt.downstream_dispatch_ready
            || (after_receipt.dispatch_status == "routed"
                && (after_receipt.dispatch_kind != "taskflow_pack"
                    || infer_project_root_from_state_root(store.root()).is_some()));
        if !progressed || !has_more_ready_work {
            break;
        }
    }

    let Some((source_dispatch_packet_path, dispatch_receipt, snapshot_path)) = last_result else {
        eprintln!("No advance step was executed");
        return ExitCode::from(1);
    };

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "vida taskflow consume advance",
                "source_run_id": dispatch_receipt.run_id,
                "source_dispatch_packet_path": source_dispatch_packet_path,
                "dispatch_receipt": dispatch_receipt,
                "snapshot_path": snapshot_path,
                "rounds_executed": rounds,
            }))
            .expect("advance should render as json")
        );
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow consume advance");
        print_surface_line(RenderMode::Plain, "source run", &dispatch_receipt.run_id);
        print_surface_line(
            RenderMode::Plain,
            "source packet",
            &source_dispatch_packet_path,
        );
        print_surface_line(RenderMode::Plain, "rounds executed", &rounds.to_string());
        print_surface_line(RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    ExitCode::SUCCESS
}

fn runtime_consumption_summary(state_root: &Path) -> Result<RuntimeConsumptionSummary, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    if !snapshot_dir.exists() {
        return Ok(RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: None,
        });
    }

    let mut total_snapshots = 0usize;
    let mut bundle_snapshots = 0usize;
    let mut bundle_check_snapshots = 0usize;
    let mut final_snapshots = 0usize;
    let mut latest: Option<(SystemTime, String, String)> = None;

    for entry in std::fs::read_dir(&snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        total_snapshots += 1;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let kind = if file_name.starts_with("bundle-check-") {
            bundle_check_snapshots += 1;
            "bundle-check".to_string()
        } else if file_name.starts_with("bundle-") {
            bundle_snapshots += 1;
            "bundle".to_string()
        } else if file_name.starts_with("final-") {
            final_snapshots += 1;
            "final".to_string()
        } else {
            "unknown".to_string()
        };

        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let path_display = path.display().to_string();
        match &latest {
            Some((latest_modified, _, _)) if modified <= *latest_modified => {}
            _ => latest = Some((modified, kind, path_display)),
        }
    }

    Ok(RuntimeConsumptionSummary {
        total_snapshots,
        bundle_snapshots,
        bundle_check_snapshots,
        final_snapshots,
        latest_kind: latest.as_ref().map(|(_, kind, _)| kind.clone()),
        latest_snapshot_path: latest.map(|(_, _, path)| path),
    })
}

async fn run_taskflow_consume(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "consume" => {
            print_taskflow_proxy_help(Some("consume"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "consume" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("consume"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "consume" && subcommand == "bundle" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => {
                        let snapshot_path = match write_runtime_consumption_snapshot(
                            store.root(),
                            "bundle",
                            &serde_json::json!({
                                "surface": "vida taskflow consume bundle",
                                "bundle": &payload,
                            }),
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        };
                        print_surface_header(RenderMode::Plain, "vida taskflow consume bundle");
                        print_surface_line(RenderMode::Plain, "artifact", &payload.artifact_name);
                        print_surface_line(
                            RenderMode::Plain,
                            "root artifact",
                            payload.control_core["root_artifact_id"]
                                .as_str()
                                .unwrap_or("unknown"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "bundle order",
                            &payload.control_core["mandatory_chain_order"]
                                .as_array()
                                .map(|rows| {
                                    rows.iter()
                                        .filter_map(serde_json::Value::as_str)
                                        .collect::<Vec<_>>()
                                        .join(" -> ")
                                })
                                .unwrap_or_else(|| "none".to_string()),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "boot compatibility",
                            payload.boot_compatibility["classification"]
                                .as_str()
                                .unwrap_or("unknown"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "migration state",
                            payload.migration_preflight["migration_state"]
                                .as_str()
                                .unwrap_or("unknown"),
                        );
                        print_surface_line(RenderMode::Plain, "snapshot path", &snapshot_path);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "consume" && subcommand == "bundle" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => {
                        let snapshot_path = match write_runtime_consumption_snapshot(
                            store.root(),
                            "bundle",
                            &serde_json::json!({
                                "surface": "vida taskflow consume bundle",
                                "bundle": &payload,
                            }),
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        };
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow consume bundle",
                                "bundle": payload,
                                "snapshot_path": snapshot_path,
                            }))
                            .expect("consume bundle should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, mode]
            if head == "consume" && subcommand == "bundle" && mode == "check" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => {
                        let check = taskflow_consume_bundle_check(&payload);
                        let snapshot_path = match write_runtime_consumption_snapshot(
                            store.root(),
                            "bundle-check",
                            &serde_json::json!({
                                "surface": "vida taskflow consume bundle check",
                                "check": &check,
                                "bundle": &payload,
                            }),
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        };
                        print_surface_header(
                            RenderMode::Plain,
                            "vida taskflow consume bundle check",
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "ok",
                            if check.ok { "true" } else { "false" },
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "root artifact",
                            &check.root_artifact_id,
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "artifact count",
                            &check.artifact_count.to_string(),
                        );
                        if !check.blockers.is_empty() {
                            print_surface_line(
                                RenderMode::Plain,
                                "blockers",
                                &check.blockers.join(", "),
                            );
                        }
                        print_surface_line(RenderMode::Plain, "snapshot path", &snapshot_path);
                        if check.ok {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, mode, flag]
            if head == "consume"
                && subcommand == "bundle"
                && mode == "check"
                && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => {
                        let check = taskflow_consume_bundle_check(&payload);
                        let snapshot_path = match write_runtime_consumption_snapshot(
                            store.root(),
                            "bundle-check",
                            &serde_json::json!({
                                "surface": "vida taskflow consume bundle check",
                                "check": &check,
                                "bundle": &payload,
                            }),
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        };
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow consume bundle check",
                                "check": check,
                                "snapshot_path": snapshot_path,
                            }))
                            .expect("consume bundle check should render as json")
                        );
                        if check.ok {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "continue" => {
            let (
                as_json,
                requested_run_id,
                requested_dispatch_packet_path,
                requested_downstream_packet_path,
            ) = match parse_taskflow_consume_continue_args(args) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            return run_taskflow_consume_resume_command(
                proxy_state_dir(),
                as_json,
                requested_run_id,
                requested_dispatch_packet_path,
                requested_downstream_packet_path,
                "vida taskflow consume continue",
                true,
            )
            .await;
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "advance" => {
            let (as_json, requested_run_id, max_rounds) =
                match parse_taskflow_consume_advance_args(args) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            return run_taskflow_consume_advance_command(
                proxy_state_dir(),
                as_json,
                requested_run_id,
                max_rounds,
            )
            .await;
        }
        [head, subcommand, request @ ..] if head == "consume" && subcommand == "final" => {
            let as_json = request.iter().any(|arg| arg == "--json");
            let request_text = request
                .iter()
                .filter(|arg| arg.as_str() != "--json")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if request_text.is_empty() {
                eprintln!("Usage: vida taskflow consume final <request_text> [--json]");
                return ExitCode::from(2);
            }

            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match build_taskflow_consume_bundle_payload(&store).await {
                    Ok(runtime_bundle) => {
                        let bundle_check = taskflow_consume_bundle_check(&runtime_bundle);
                        let (registry, check, readiness, proof, overview) =
                            build_docflow_runtime_evidence();
                        let docflow_verdict =
                            build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);
                        let role_selection =
                            match build_runtime_lane_selection_with_store(&store, &request_text)
                                .await
                            {
                                Ok(selection) => selection,
                                Err(error) => {
                                    if as_json {
                                        let payload = TaskflowDirectConsumptionPayload {
                                            artifact_name: "taskflow_direct_runtime_consumption"
                                                .to_string(),
                                            artifact_type: "runtime_consumption".to_string(),
                                            generated_at: time::OffsetDateTime::now_utc()
                                                .format(&Rfc3339)
                                                .expect("rfc3339 timestamp should render"),
                                            closure_authority: "taskflow".to_string(),
                                            role_selection: blocking_lane_selection(
                                                &request_text,
                                                &error,
                                            ),
                                            request_text: request_text.clone(),
                                            direct_consumption_ready: false,
                                            runtime_bundle,
                                            bundle_check,
                                            docflow_activation:
                                                RuntimeConsumptionDocflowActivation {
                                                    activated: true,
                                                    runtime_family: "docflow".to_string(),
                                                    owner_runtime: "taskflow".to_string(),
                                                    evidence: serde_json::json!({
                                                        "overview": overview,
                                                        "registry": registry,
                                                        "check": check,
                                                        "readiness": readiness,
                                                        "proof": proof,
                                                    }),
                                                },
                                            docflow_verdict,
                                            closure_admission: RuntimeConsumptionClosureAdmission {
                                                status: "block".to_string(),
                                                admitted: false,
                                                blockers: vec![
                                                    "unresolved_lane_selection".to_string()
                                                ],
                                                proof_surfaces: vec![
                                                    "vida taskflow consume bundle check"
                                                        .to_string(),
                                                ],
                                            },
                                            taskflow_handoff_plan: serde_json::json!({
                                                "status": "blocked",
                                                "handoff_ready": false,
                                                "reason": "unresolved_lane_selection",
                                            }),
                                            run_graph_bootstrap: serde_json::json!({
                                                "status": "blocked",
                                                "handoff_ready": false,
                                                "reason": "unresolved_lane_selection",
                                            }),
                                            dispatch_receipt: serde_json::json!({
                                                "status": "blocked",
                                                "reason": "unresolved_lane_selection",
                                            }),
                                        };
                                        if let Err(snapshot_error) =
                                            emit_taskflow_consume_final_json(&store, &payload)
                                        {
                                            eprintln!("{snapshot_error}");
                                        }
                                        return ExitCode::from(1);
                                    }
                                    eprintln!("{error}");
                                    return ExitCode::from(1);
                                }
                            };
                        let closure_admission = build_runtime_closure_admission(
                            &bundle_check,
                            &docflow_verdict,
                            &role_selection,
                        );
                        let taskflow_handoff_plan = build_taskflow_handoff_plan(&role_selection);
                        let run_graph_bootstrap =
                            build_runtime_consumption_run_graph_bootstrap(&store, &role_selection)
                                .await;
                        let mut dispatch_receipt = build_runtime_consumption_dispatch_receipt(
                            &role_selection,
                            &run_graph_bootstrap,
                        );
                        dispatch_receipt.dispatch_command = runtime_dispatch_command_for_target(
                            &role_selection,
                            &dispatch_receipt.dispatch_target,
                        );
                        if let Err(error) = refresh_downstream_dispatch_preview(
                            store.root(),
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut dispatch_receipt,
                        ) {
                            eprintln!(
                                "Failed to write downstream runtime dispatch packet: {error}"
                            );
                            return ExitCode::from(1);
                        }
                        let dispatch_packet_path = match write_runtime_dispatch_packet(
                            store.root(),
                            &role_selection,
                            &dispatch_receipt,
                            &taskflow_handoff_plan,
                            &run_graph_bootstrap,
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("Failed to write runtime dispatch packet: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path);
                        let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                            != "taskflow_pack"
                            || infer_project_root_from_state_root(store.root()).is_some();
                        if dispatch_receipt.dispatch_status == "routed"
                            && allow_taskflow_pack_execution
                        {
                            if let Err(error) = execute_and_record_dispatch_receipt(
                                store.root(),
                                &store,
                                &role_selection,
                                &run_graph_bootstrap,
                                &mut dispatch_receipt,
                            )
                            .await
                            {
                                eprintln!("Failed to execute runtime dispatch handoff: {error}");
                                return ExitCode::from(1);
                            }
                        }
                        if let Err(error) = execute_downstream_dispatch_chain(
                            store.root(),
                            &store,
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut dispatch_receipt,
                        )
                        .await
                        {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                        let dispatch_receipt_json = serde_json::to_value(&dispatch_receipt)
                            .unwrap_or(serde_json::Value::Null);
                        if let Err(error) = store
                            .record_run_graph_dispatch_receipt(&dispatch_receipt)
                            .await
                        {
                            eprintln!("Failed to record run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                        let direct_consumption_ready = bundle_check.ok
                            && docflow_verdict.ready
                            && !closure_admission
                                .blockers
                                .iter()
                                .any(|row| row == "pending_design_packet");
                        let payload = TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: time::OffsetDateTime::now_utc()
                                .format(&Rfc3339)
                                .expect("rfc3339 timestamp should render"),
                            closure_authority: "taskflow".to_string(),
                            role_selection,
                            request_text,
                            direct_consumption_ready,
                            runtime_bundle,
                            bundle_check,
                            docflow_activation: RuntimeConsumptionDocflowActivation {
                                activated: true,
                                runtime_family: "docflow".to_string(),
                                owner_runtime: "taskflow".to_string(),
                                evidence: serde_json::json!({
                                    "overview": overview,
                                    "registry": registry,
                                    "check": check,
                                    "readiness": readiness,
                                    "proof": proof,
                                }),
                            },
                            docflow_verdict,
                            closure_admission,
                            taskflow_handoff_plan,
                            run_graph_bootstrap,
                            dispatch_receipt: dispatch_receipt_json,
                        };
                        if as_json {
                            if let Err(error) = emit_taskflow_consume_final_json(&store, &payload) {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        } else {
                            let snapshot = serde_json::json!({
                                "surface": "vida taskflow consume final",
                                "payload": &payload,
                            });
                            let snapshot_path = match write_runtime_consumption_snapshot(
                                store.root(),
                                "final",
                                &snapshot,
                            ) {
                                Ok(path) => path,
                                Err(error) => {
                                    eprintln!("{error}");
                                    return ExitCode::from(1);
                                }
                            };
                            print_surface_header(RenderMode::Plain, "vida taskflow consume final");
                            print_surface_line(RenderMode::Plain, "request", &payload.request_text);
                            print_surface_line(
                                RenderMode::Plain,
                                "bundle ready",
                                if payload.bundle_check.ok {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "docflow ready",
                                if payload.docflow_verdict.ready {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "closure admitted",
                                if payload.closure_admission.admitted {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            if payload.role_selection.execution_plan["status"] == "design_first" {
                                if let Some(feature_slug) = payload.role_selection.execution_plan
                                    ["tracked_flow_bootstrap"]["feature_slug"]
                                    .as_str()
                                {
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "tracked flow",
                                        &format!("spec-first bootstrap for `{feature_slug}`"),
                                    );
                                }
                                if let Some(command) = payload.role_selection.execution_plan
                                    ["tracked_flow_bootstrap"]["epic"]["create_command"]
                                    .as_str()
                                {
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "next epic command",
                                        command,
                                    );
                                }
                                if let Some(command) = payload.role_selection.execution_plan
                                    ["tracked_flow_bootstrap"]["docflow"]["init_command"]
                                    .as_str()
                                {
                                    print_surface_line(
                                        RenderMode::Plain,
                                        "next design command",
                                        command,
                                    );
                                }
                            } else if let Some(agent_type) = payload.taskflow_handoff_plan
                                ["activation_chain"]["implementer"]["activation_agent_type"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "implementer carrier",
                                    agent_type,
                                );
                            }
                            print_surface_line(RenderMode::Plain, "snapshot path", &snapshot_path);
                        }

                        if payload.closure_admission.admitted {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        if as_json {
                            let runtime_bundle = blocking_runtime_bundle(&error);
                            let bundle_check = taskflow_consume_bundle_check(&runtime_bundle);
                            let docflow_verdict = RuntimeConsumptionDocflowVerdict {
                                status: "block".to_string(),
                                ready: false,
                                blockers: vec![
                                    "missing_docflow_activation".to_string(),
                                    "missing_readiness_verdict".to_string(),
                                    "missing_proof_verdict".to_string(),
                                ],
                                proof_surfaces: vec![],
                            };
                            let role_selection = blocking_lane_selection(&request_text, &error);
                            let closure_admission = build_runtime_closure_admission(
                                &bundle_check,
                                &docflow_verdict,
                                &role_selection,
                            );
                            let payload = TaskflowDirectConsumptionPayload {
                                artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                                artifact_type: "runtime_consumption".to_string(),
                                generated_at: time::OffsetDateTime::now_utc()
                                    .format(&Rfc3339)
                                    .expect("rfc3339 timestamp should render"),
                                closure_authority: "taskflow".to_string(),
                                request_text,
                                role_selection,
                                runtime_bundle,
                                bundle_check,
                                docflow_activation: blocking_docflow_activation(&error),
                                docflow_verdict,
                                closure_admission,
                                taskflow_handoff_plan: serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "reason": "docflow_activation_failed",
                                }),
                                run_graph_bootstrap: serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "reason": "docflow_activation_failed",
                                }),
                                dispatch_receipt: serde_json::json!({
                                    "status": "blocked",
                                    "reason": "docflow_activation_failed",
                                }),
                                direct_consumption_ready: false,
                            };
                            if let Err(snapshot_error) =
                                emit_taskflow_consume_final_json(&store, &payload)
                            {
                                eprintln!("{snapshot_error}");
                                return ExitCode::from(1);
                            }
                            return ExitCode::from(1);
                        }
                        eprintln!("{error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "bundle" => {
            eprintln!(
                "Usage: vida taskflow consume bundle [--json]\n       vida taskflow consume bundle check [--json]"
            );
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "final" => {
            eprintln!("Usage: vida taskflow consume final <request_text> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

fn print_surface_header(render: RenderMode, title: &str) {
    match render {
        RenderMode::Plain => println!("{title}"),
        RenderMode::Color => println!("\x1b[1;36m{title}\x1b[0m"),
        RenderMode::ColorEmoji => println!("\x1b[1;36m📘 {title}\x1b[0m"),
    }
}

fn print_surface_line(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: {value}"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: {value}"),
        RenderMode::ColorEmoji => println!("🔹 \x1b[1;34m{label}\x1b[0m: {value}"),
    }
}

fn print_surface_ok(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: ok ({value})"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: \x1b[1;32mok\x1b[0m ({value})"),
        RenderMode::ColorEmoji => {
            println!("✅ \x1b[1;34m{label}\x1b[0m: \x1b[1;32mok\x1b[0m ({value})")
        }
    }
}

fn print_task_list(render: RenderMode, tasks: &[TaskRecord], as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tasks).expect("task list should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task");
    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

fn print_task_show(render: RenderMode, task: &TaskRecord, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(task).expect("task should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task show");
    print_surface_line(render, "id", &task.id);
    print_surface_line(render, "status", &task.status);
    print_surface_line(render, "title", &task.title);
    print_surface_line(render, "priority", &task.priority.to_string());
    print_surface_line(render, "issue type", &task.issue_type);
    if !task.labels.is_empty() {
        print_surface_line(render, "labels", &task.labels.join(", "));
    }
    if !task.dependencies.is_empty() {
        let summary = task
            .dependencies
            .iter()
            .map(|dependency| format!("{}:{}", dependency.edge_type, dependency.depends_on_id))
            .collect::<Vec<_>>()
            .join(", ");
        print_surface_line(render, "dependencies", &summary);
    }
}

fn print_task_dependencies(
    render: RenderMode,
    title: &str,
    task_id: &str,
    dependencies: &[TaskDependencyStatus],
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(dependencies)
                .expect("task dependencies should render as json")
        );
        return;
    }

    print_surface_header(render, title);
    print_surface_line(render, "task", task_id);
    if dependencies.is_empty() {
        print_surface_line(render, "dependencies", "none");
        return;
    }

    for dependency in dependencies {
        let issue_type = dependency
            .dependency_issue_type
            .as_deref()
            .unwrap_or("unknown");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            dependency.issue_id,
            dependency.edge_type,
            dependency.depends_on_id,
            dependency.dependency_status,
            issue_type
        );
    }
}

fn print_blocked_tasks(render: RenderMode, tasks: &[BlockedTaskRecord], as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tasks).expect("blocked tasks should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task blocked");
    if tasks.is_empty() {
        print_surface_line(render, "blocked tasks", "none");
        return;
    }

    for blocked in tasks {
        println!(
            "{}\t{}\t{}",
            blocked.task.id, blocked.task.status, blocked.task.title
        );
        for blocker in &blocked.blockers {
            println!(
                "  blocked-by\t{}\t{}\t{}",
                blocker.edge_type, blocker.depends_on_id, blocker.dependency_status
            );
        }
    }
}

fn print_task_dependency_tree(render: RenderMode, tree: &TaskDependencyTreeNode, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tree).expect("task dependency tree should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task tree");
    print_surface_line(
        render,
        "root",
        &format!(
            "{}\t{}\t{}",
            tree.task.id, tree.task.status, tree.task.title
        ),
    );
    if tree.dependencies.is_empty() {
        print_surface_line(render, "dependencies", "none");
        return;
    }

    for edge in &tree.dependencies {
        print_task_dependency_tree_edge(edge, 0);
    }
}

fn print_task_dependency_tree_edge(edge: &TaskDependencyTreeEdge, depth: usize) {
    let indent = "  ".repeat(depth);
    let issue_type = edge.dependency_issue_type.as_deref().unwrap_or("unknown");
    let state = if edge.cycle {
        "cycle"
    } else if edge.missing {
        "missing"
    } else {
        edge.dependency_status.as_str()
    };
    println!(
        "{indent}{} -> {}\t{}\t{}\t{}",
        edge.edge_type, edge.depends_on_id, state, issue_type, edge.issue_id
    );

    if let Some(node) = &edge.node {
        for child in &node.dependencies {
            print_task_dependency_tree_edge(child, depth + 1);
        }
    }
}

fn print_task_graph_issues(render: RenderMode, issues: &[TaskGraphIssue], as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(issues).expect("task graph issues should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task validate-graph");
    if issues.is_empty() {
        print_surface_line(render, "graph", "ok");
        return;
    }

    for issue in issues {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            issue.issue_type,
            issue.issue_id,
            issue.depends_on_id.as_deref().unwrap_or("-"),
            issue.edge_type.as_deref().unwrap_or("-"),
            issue.detail
        );
    }
}

fn print_task_dependency_mutation(
    render: RenderMode,
    title: &str,
    dependency: &TaskDependencyRecord,
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(dependency)
                .expect("task dependency mutation should render as json")
        );
        return;
    }

    print_surface_header(render, title);
    print_surface_line(render, "task", &dependency.issue_id);
    print_surface_line(render, "depends_on", &dependency.depends_on_id);
    print_surface_line(render, "edge_type", &dependency.edge_type);
}

fn print_task_critical_path(render: RenderMode, path: &TaskCriticalPath, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(path).expect("critical path should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task critical-path");
    print_surface_line(render, "length", &path.length.to_string());
    print_surface_line(
        render,
        "root_task_id",
        path.root_task_id.as_deref().unwrap_or("none"),
    );
    print_surface_line(
        render,
        "terminal_task_id",
        path.terminal_task_id.as_deref().unwrap_or("none"),
    );
    for node in &path.nodes {
        println!(
            "{}\t{}\t{}\t{}",
            node.id, node.status, node.issue_type, node.title
        );
    }
}

fn normalize_root_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub(crate) enum RenderMode {
    #[default]
    Plain,
    Color,
    #[value(name = "color_emoji")]
    ColorEmoji,
}

#[derive(Parser, Debug)]
#[command(
    name = "vida",
    disable_help_subcommand = true,
    about = "VIDA Binary Foundation",
    long_about = "VIDA Binary Foundation\n\nRoot commands stay fail-closed. TaskFlow remains execution authority; DocFlow remains the documentation/readiness surface.",
    after_help = "Runtime-family help paths:\n  vida taskflow help\n  vida docflow help"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "bootstrap framework carriers into the current project")]
    Init(BootArgs),
    #[command(about = "initialize authoritative state and instruction/framework-memory surfaces")]
    Boot(BootArgs),
    #[command(about = "render the compiled startup view for the orchestrator lane")]
    OrchestratorInit(InitArgs),
    #[command(about = "render the bounded startup view for a worker/agent lane")]
    AgentInit(AgentInitArgs),
    #[command(about = "resolve and render framework protocol/guide surfaces")]
    Protocol(ProtocolArgs),
    #[command(about = "inspect project activation posture and bounded onboarding next steps")]
    ProjectActivator(ProjectActivatorArgs),
    #[command(about = "record host-agent feedback and refresh local strategy state")]
    AgentFeedback(AgentFeedbackArgs),
    #[command(about = "task import/list/show/ready over the authoritative state store")]
    Task(TaskArgs),
    #[command(about = "inspect the effective instruction bundle")]
    Memory(MemoryArgs),
    #[command(about = "inspect backend, state spine, and latest receipts")]
    Status(StatusArgs),
    #[command(about = "run bounded runtime integrity checks")]
    Doctor(DoctorArgs),
    #[command(about = "delegate to the TaskFlow runtime family")]
    Taskflow(ProxyArgs),
    #[command(about = "delegate to the DocFlow runtime family")]
    Docflow(ProxyArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Args, Debug, Clone, Default)]
struct ProxyArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
struct TaskArgs {
    #[command(subcommand)]
    command: TaskCommand,
}

#[derive(Subcommand, Debug, Clone)]
enum TaskCommand {
    ImportJsonl(TaskImportJsonlArgs),
    List(TaskListArgs),
    Show(TaskShowArgs),
    Ready(TaskReadyArgs),
    Deps(TaskDepsArgs),
    ReverseDeps(TaskDepsArgs),
    Blocked(TaskBlockedArgs),
    Tree(TaskDepsArgs),
    ValidateGraph(TaskBlockedArgs),
    Dep(TaskDepArgs),
    CriticalPath(TaskBlockedArgs),
}

#[derive(Args, Debug, Clone)]
struct TaskDepArgs {
    #[command(subcommand)]
    command: TaskDependencyCommand,
}

#[derive(Subcommand, Debug, Clone)]
enum TaskDependencyCommand {
    Add(TaskDependencyMutationCommandArgs),
    Remove(TaskDependencyTargetCommandArgs),
}

#[derive(Args, Debug, Clone, Default)]
struct TaskDependencyMutationCommandArgs {
    task_id: String,
    depends_on_id: String,
    edge_type: String,

    #[arg(long = "created-by", default_value = "vida")]
    created_by: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskDependencyTargetCommandArgs {
    task_id: String,
    depends_on_id: String,
    edge_type: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskImportJsonlArgs {
    path: PathBuf,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskListArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "status")]
    status: Option<String>,

    #[arg(long = "all")]
    all: bool,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskShowArgs {
    task_id: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskReadyArgs {
    #[arg(long = "scope")]
    scope: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskDepsArgs {
    task_id: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct TaskBlockedArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct BootArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "instruction-source-root", env = "VIDA_INSTRUCTION_SOURCE_ROOT")]
    instruction_source_root: Option<PathBuf>,

    #[arg(
        long = "framework-memory-source-root",
        env = "VIDA_FRAMEWORK_MEMORY_SOURCE_ROOT"
    )]
    framework_memory_source_root: Option<PathBuf>,

    #[arg(hide = true, trailing_var_arg = true, allow_hyphen_values = true)]
    extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
struct InitArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct ProjectActivatorArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "project-id")]
    project_id: Option<String>,

    #[arg(long = "project-name")]
    project_name: Option<String>,

    #[arg(long = "language")]
    language: Option<String>,

    #[arg(long = "user-communication-language")]
    user_communication_language: Option<String>,

    #[arg(long = "reasoning-language")]
    reasoning_language: Option<String>,

    #[arg(long = "documentation-language")]
    documentation_language: Option<String>,

    #[arg(long = "todo-protocol-language")]
    todo_protocol_language: Option<String>,

    #[arg(long = "host-cli-system")]
    host_cli_system: Option<String>,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct AgentFeedbackArgs {
    #[arg(long = "agent-id")]
    agent_id: String,

    #[arg(long = "score")]
    score: u64,

    #[arg(long = "outcome")]
    outcome: Option<String>,

    #[arg(long = "task-class")]
    task_class: Option<String>,

    #[arg(long = "notes")]
    notes: Option<String>,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct AgentInitArgs {
    request_text: Option<String>,

    #[arg(long = "role")]
    role: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone)]
struct ProtocolArgs {
    #[command(subcommand)]
    command: ProtocolCommand,
}

#[derive(Subcommand, Debug, Clone)]
enum ProtocolCommand {
    View(ProtocolViewArgs),
}

#[derive(Args, Debug, Clone)]
struct ProtocolViewArgs {
    #[arg(required = true, num_args = 1..)]
    names: Vec<String>,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct MemoryArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,
}

#[derive(Args, Debug, Clone, Default)]
struct StatusArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

#[derive(Args, Debug, Clone, Default)]
struct DoctorArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    render: RenderMode,

    #[arg(long = "json")]
    json: bool,
}

fn print_root_help() {
    println!("VIDA Binary Foundation");
    println!();
    println!("Usage:");
    println!("  vida <command>");
    println!("  vida taskflow <args...>");
    println!("  vida docflow <args...>");
    println!();
    println!("Root commands:");
    println!("  init      bootstrap framework carriers into the current project");
    println!(
        "  boot      initialize authoritative state and instruction/framework-memory surfaces"
    );
    println!("  orchestrator-init  render the compiled startup view for the orchestrator lane");
    println!("  agent-init         render the bounded startup view for a worker/agent lane");
    println!("  protocol  resolve and render framework protocol/guide surfaces");
    println!(
        "  project-activator  inspect project activation posture and bounded onboarding next steps"
    );
    println!("  agent-feedback  record host-agent feedback and refresh local strategy state");
    println!("  task      task import/list/show/ready over the authoritative state store");
    println!("  memory    inspect the effective instruction bundle");
    println!("  status    inspect backend, state spine, and latest receipts");
    println!("  doctor    run bounded runtime integrity checks");
    println!("  taskflow  delegate to the TaskFlow runtime family");
    println!("  docflow   delegate to the DocFlow runtime family");
    println!();
    println!("Notes:");
    println!("  - root commands stay fail-closed");
    println!("  - runtime-family help paths are `vida taskflow help` and `vida docflow help`");
    println!(
        "  - TaskFlow remains execution authority; DocFlow remains documentation/readiness surface"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;
    use clap::Parser;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    fn current_dir_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &Path) -> Self {
            let original = env::current_dir().expect("current dir should resolve");
            env::set_current_dir(path).expect("current dir should change");
            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.original).expect("current dir should restore");
        }
    }
    use std::fs;
    use std::thread;
    use std::time::{Duration, Instant};

    fn cli(args: &[&str]) -> Cli {
        let mut argv = vec!["vida"];
        argv.extend(args.iter().copied());
        Cli::parse_from(argv)
    }

    fn wait_for_state_unlock(state_dir: &std::path::Path) {
        let lock_path = state_dir.join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while lock_path.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn temp_state_harness_creates_and_cleans_directory() {
        let path = {
            let harness = TempStateHarness::new().expect("temp state harness should initialize");
            let path = harness.path().to_path_buf();
            assert!(path.exists());
            path
        };

        assert!(!path.exists());
    }

    #[test]
    fn boot_command_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        assert_eq!(
            runtime.block_on(run(Cli {
                command: Some(Command::Boot(BootArgs {
                    state_dir: Some(harness.path().to_path_buf()),
                    render: RenderMode::Plain,
                    instruction_source_root: None,
                    framework_memory_source_root: None,
                    extra_args: Vec::new(),
                })),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn init_command_succeeds() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());
        assert_eq!(
            runtime.block_on(run(Cli {
                command: Some(Command::Init(BootArgs {
                    state_dir: Some(harness.path().to_path_buf()),
                    render: RenderMode::Plain,
                    instruction_source_root: None,
                    framework_memory_source_root: None,
                    extra_args: Vec::new(),
                })),
            })),
            ExitCode::SUCCESS
        );
        assert!(harness.path().join("AGENTS.md").is_file());
        assert!(harness.path().join("AGENTS.sidecar.md").is_file());
        let agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("generated AGENTS should exist");
        assert!(
            agents.contains("VIDA Project Bootstrap Carrier"),
            "bare init should use the generated downstream bootstrap carrier"
        );
        assert!(
            !agents.contains("-v0"),
            "generated downstream bootstrap carrier should not leak legacy or historical runtime suffixes"
        );
        assert!(
            !harness.path().join(".codex").exists(),
            "host CLI templates should not materialize during bare `vida init`"
        );
        assert!(harness.path().join("vida.config.yaml").is_file());
        assert!(harness.path().join(".vida/config").is_dir());
        assert!(harness.path().join(".vida/db").is_dir());
        assert!(harness.path().join(".vida/cache").is_dir());
        assert!(harness.path().join(".vida/framework").is_dir());
        assert!(harness.path().join(".vida/project").is_dir());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/README.md")
            .is_file());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/roles.yaml")
            .is_file());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/roles.sidecar.yaml")
            .is_file());
        assert!(harness.path().join(".vida/receipts").is_dir());
        assert!(harness.path().join(".vida/runtime").is_dir());
        assert!(harness.path().join(".vida/scratchpad").is_dir());
        assert!(!harness.path().join("vida").exists());
    }

    #[test]
    #[ignore = "covered by binary integration smoke; in-process sequential SurrealKv opens keep the lock longer than this unit test assumes"]
    fn task_command_round_trip_succeeds() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let jsonl_path = harness.path().join("issues.jsonl");
        fs::write(
            &jsonl_path,
            concat!(
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write sample task jsonl");

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(run(cli(&[
                    "task",
                    "import-jsonl",
                    jsonl_path.to_str().expect("jsonl path should render"),
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(run(cli(&[
                    "task",
                    "list",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(run(cli(&[
                    "task",
                    "ready",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn unknown_root_command_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(runtime.block_on(run(cli(&["unknown"]))), ExitCode::from(2));
    }

    #[test]
    fn boot_with_extra_argument_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["boot", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn init_with_extra_argument_fails_closed() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["init", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn clap_help_lists_init_before_boot() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();
        let init_index = help.find("init").expect("init should be present in help");
        let boot_index = help.find("boot").expect("boot should be present in help");
        assert!(
            init_index < boot_index,
            "init should appear before boot in help"
        );
    }

    #[test]
    fn clap_help_lists_project_activator() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();
        assert!(
            help.contains("project-activator"),
            "project-activator should be present in help"
        );
    }

    #[test]
    fn clap_help_lists_protocol() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();
        assert!(
            help.contains("protocol"),
            "protocol should be present in help"
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

    #[test]
    fn protocol_view_command_accepts_json_output() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(
            runtime.block_on(run(cli(&["protocol", "view", "AGENTS", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn init_preserves_existing_agents_as_sidecar_when_missing() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());
        fs::write(
            harness.path().join("AGENTS.md"),
            "project documentation: docs/\n",
        )
        .expect("existing agents should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
                .expect("sidecar should exist"),
            "project documentation: docs/\n"
        );
        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );
    }

    #[test]
    fn project_activator_reports_pending_activation_for_partial_project() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        fs::write(harness.path().join("README.md"), "# demo\n").expect("readme should exist");

        let view = build_project_activator_view(harness.path());

        assert_eq!(view["status"], "pending_activation");
        assert_eq!(view["project_shape"], "partial");
        assert_eq!(view["activation_pending"], true);
        assert_eq!(
            view["triggers"]["initial_onboarding_missing"],
            serde_json::Value::Bool(true)
        );
    }

    #[test]
    fn project_activator_reports_ready_when_bootstrap_and_docs_exist() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".codex/agents")).expect(".codex agents dir should exist");
        fs::create_dir_all(root.join(".vida/config")).expect(".vida/config dir should exist");
        fs::create_dir_all(root.join(".vida/db")).expect(".vida/db dir should exist");
        fs::create_dir_all(root.join(".vida/cache")).expect(".vida/cache dir should exist");
        fs::create_dir_all(root.join(".vida/framework")).expect(".vida/framework dir should exist");
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect(".vida/project agent extensions dir should exist");
        fs::create_dir_all(root.join(".vida/receipts")).expect(".vida/receipts dir should exist");
        fs::create_dir_all(root.join(".vida/runtime")).expect(".vida/runtime dir should exist");
        fs::create_dir_all(root.join(".vida/scratchpad"))
            .expect(".vida/scratchpad dir should exist");
        fs::create_dir_all(root.join("docs/product")).expect("product docs dir should exist");
        fs::create_dir_all(root.join("docs/process")).expect("process docs dir should exist");
        fs::write(root.join("AGENTS.md"), "# framework\n").expect("agents should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "project docs map\n")
            .expect("sidecar should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "project:\n  id: demo\n",
                "language_policy:\n",
                "  user_communication: english\n",
                "  reasoning: english\n",
                "  documentation: english\n",
                "  todo_protocol: english\n",
                "host_environment:\n  cli_system: codex\n"
            ),
        )
        .expect("config should exist");
        fs::write(root.join(".codex/config.toml"), "[agents]\n")
            .expect("codex config should exist");
        fs::write(root.join("docs/project-root-map.md"), "# root map\n")
            .expect("project root map should exist");
        fs::write(root.join("docs/product/index.md"), "# product\n")
            .expect("product index should exist");
        fs::create_dir_all(root.join("docs/product/spec/templates"))
            .expect("product spec template dir should exist");
        fs::write(
            root.join("docs/product/spec/README.md"),
            "# product spec guide\n",
        )
        .expect("product spec guide should exist");
        fs::write(
            root.join("docs/product/spec/templates/feature-design-document.template.md"),
            "# feature design template\n",
        )
        .expect("feature design template should exist");
        fs::write(root.join("docs/process/README.md"), "# process\n")
            .expect("process readme should exist");
        fs::write(
            root.join("docs/process/codex-agent-configuration-guide.md"),
            "# codex guide\n",
        )
        .expect("codex guide should exist");
        fs::write(
            root.join("docs/process/documentation-tooling-map.md"),
            "# tooling\n",
        )
        .expect("documentation tooling map should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/README.md"),
            "# runtime agent extensions\n",
        )
        .expect("runtime readme should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");

        let view = build_project_activator_view(root);

        assert_eq!(view["status"], "ready_enough_for_normal_work");
        assert_eq!(view["project_shape"], "bootstrapped");
        assert_eq!(view["activation_pending"], false);
        assert_eq!(view["host_environment"]["selected_cli_system"], "codex");
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(
            view["next_steps"][0],
            "activation looks ready enough for normal orchestrator and worker initialization"
        );
    }

    #[test]
    fn project_activator_reports_pending_after_init_scaffold_without_docs() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let view = build_project_activator_view(harness.path());
        assert_eq!(view["status"], "pending_activation");
        assert_eq!(view["activation_pending"], true);
        assert_eq!(view["triggers"]["sidecar_or_project_docs_too_thin"], true);
        assert_eq!(
            view["triggers"]["host_cli_unselected_or_unmaterialized"],
            true
        );
        assert_eq!(view["project_docs"]["config_has_placeholders"], true);
        assert_eq!(view["agent_extensions"]["bundle_ready"], true);
        assert_eq!(
            view["activation_algorithm"]["taskflow_admitted_while_pending"],
            false
        );
        assert_eq!(view["activation_algorithm"]["docflow_first"], true);
        assert!(
            view["interview"]["required_inputs"]
                .as_array()
                .expect("required inputs should render")
                .len()
                >= 3,
            "activation interview should require project id, language, and host CLI selection"
        );
    }

    #[test]
    fn project_activator_accepts_host_cli_selection_and_materializes_codex_template() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        assert!(harness.path().join(".codex/config.toml").is_file());
        assert!(harness.path().join(".codex/agents").is_dir());
        assert!(harness.path().join(CODEX_WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(CODEX_WORKER_STRATEGY_STATE).is_file());
        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("cli_system: codex"));
        assert!(config.contains("host_environment:"));

        let view = build_project_activator_view(harness.path());
        assert_eq!(view["host_environment"]["selected_cli_system"], "codex");
        assert_eq!(view["host_environment"]["template_materialized"], true);
    }

    #[test]
    fn project_activator_can_complete_bounded_activation_in_one_command() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "ukrainian",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("id: vida-test"));
        assert!(config.contains("user_communication: ukrainian"));
        assert!(config.contains("documentation: ukrainian"));
        assert!(config.contains("cli_system: codex"));
        assert!(harness.path().join("docs/project-root-map.md").is_file());
        assert!(harness.path().join("docs/product/spec/README.md").is_file());
        assert!(harness
            .path()
            .join("docs/product/spec/templates/feature-design-document.template.md")
            .is_file());
        assert!(harness
            .path()
            .join("docs/process/documentation-tooling-map.md")
            .is_file());
        assert!(harness
            .path()
            .join("docs/process/codex-agent-configuration-guide.md")
            .is_file());
        assert!(harness.path().join(".codex/config.toml").is_file());
        assert!(harness.path().join(CODEX_WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(CODEX_WORKER_STRATEGY_STATE).is_file());
        assert!(
            harness
                .path()
                .join(".vida/receipts/project-activation.latest.json")
                .is_file(),
            "activation receipt should be written"
        );

        let view = build_project_activator_view(harness.path());
        assert_eq!(view["activation_pending"], false);
        assert_eq!(view["status"], "ready_enough_for_normal_work");
        assert_eq!(
            view["normal_work_defaults"]["documentation_first_for_feature_requests"],
            true
        );
        assert_eq!(
            view["normal_work_defaults"]["local_feature_design_template"],
            DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        );
    }

    #[test]
    fn project_activator_renders_codex_agent_files_from_overlay_and_keeps_template_contracts() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let config = fs::read_to_string(harness.path().join(".codex/config.toml"))
            .expect("rendered codex config should exist");
        assert!(config.contains("[agents.junior]"));
        assert!(config.contains("[agents.middle]"));
        assert!(config.contains("[agents.senior]"));
        assert!(config.contains("[agents.architect]"));
        assert!(!config.contains("[agents.development_implementer]"));
        assert!(!config.contains("[agents.development_coach]"));
        assert!(!config.contains("[agents.development_verifier]"));
        assert!(!config.contains("[agents.development_escalation]"));

        let junior = fs::read_to_string(harness.path().join(".codex/agents/junior.toml"))
            .expect("junior agent should exist");
        assert!(junior.contains("vida_rate = \"1\""));
        assert!(junior.contains("vida_runtime_roles = \"worker\""));
        assert!(junior.contains("Do not self-approve closure."));

        let middle = fs::read_to_string(harness.path().join(".codex/agents/middle.toml"))
            .expect("middle agent should exist");
        assert!(middle.contains("vida_rate = \"4\""));
        assert!(middle.contains("vida_runtime_roles = \"business_analyst,pm,coach,worker\""));
        assert!(middle
            .contains("matches the approved spec, acceptance criteria, and `definition_of_done`"));
        assert!(middle.contains("Do not replace independent verification."));

        let senior = fs::read_to_string(harness.path().join(".codex/agents/senior.toml"))
            .expect("senior agent should exist");
        assert!(senior.contains("vida_rate = \"16\""));
        assert!(senior.contains("vida_runtime_roles = \"verifier,prover\""));
        assert!(senior.contains("Fail closed when proof or evidence is missing."));

        let architect = fs::read_to_string(harness.path().join(".codex/agents/architect.toml"))
            .expect("architect agent should exist");
        assert!(architect.contains("vida_rate = \"32\""));
        assert!(architect.contains("vida_reasoning_band = \"xhigh\""));
        assert!(architect.contains("Do not become the normal default lane."));

        assert!(!harness
            .path()
            .join(".codex/agents/development_implementer.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_coach.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_verifier.toml")
            .exists());
        assert!(!harness
            .path()
            .join(".codex/agents/development_escalation.toml")
            .exists());
    }

    #[test]
    fn codex_runtime_assignment_uses_overlay_ladder_for_all_four_tiers() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);

        let assignment_for = |request: &str| {
            let selection = build_runtime_lane_selection_from_bundle(
                &bundle,
                "state_store",
                &pack_router,
                request,
            )
            .expect("selection should build");
            build_runtime_execution_plan_from_snapshot(&bundle, &selection)
                ["codex_runtime_assignment"]
                .clone()
        };
        let implementation = assignment_for("write one bounded implementation patch");
        assert_eq!(implementation["enabled"], true);
        assert_eq!(implementation["runtime_role"], "worker");
        assert_eq!(implementation["activation_agent_type"], "junior");
        assert_eq!(implementation["activation_runtime_role"], "worker");
        assert_eq!(implementation["selected_tier"], "junior");
        assert_eq!(implementation["selected_runtime_role"], "worker");
        assert_eq!(implementation["tier_default_runtime_role"], "worker");
        assert_eq!(implementation["rate"], 1);
        assert_eq!(implementation["estimated_task_price_units"], 1);

        let specification = assignment_for(
            "research the feature, write the specification, and develop an implementation plan",
        );
        assert_eq!(specification["enabled"], true);
        assert_eq!(specification["runtime_role"], "business_analyst");
        assert_eq!(specification["activation_agent_type"], "middle");
        assert_eq!(specification["activation_runtime_role"], "business_analyst");
        assert_eq!(specification["selected_tier"], "middle");
        assert_eq!(specification["selected_runtime_role"], "business_analyst");
        assert_eq!(specification["tier_default_runtime_role"], "coach");
        assert_eq!(specification["rate"], 4);
        assert_eq!(specification["estimated_task_price_units"], 8);

        let coach = assignment_for(
            "review the implemented result against the spec, acceptance criteria, and definition of done; request rework if it drifts",
        );
        assert_eq!(coach["enabled"], true);
        assert_eq!(coach["runtime_role"], "coach");
        assert_eq!(coach["activation_agent_type"], "middle");
        assert_eq!(coach["activation_runtime_role"], "coach");
        assert_eq!(coach["selected_tier"], "middle");
        assert_eq!(coach["selected_runtime_role"], "coach");
        assert_eq!(coach["tier_default_runtime_role"], "coach");
        assert_eq!(coach["rate"], 4);
        assert_eq!(coach["estimated_task_price_units"], 8);

        let verification = assignment_for("review one bounded patch and verify release readiness");
        assert_eq!(verification["enabled"], true);
        assert_eq!(verification["runtime_role"], "verifier");
        assert_eq!(verification["activation_agent_type"], "senior");
        assert_eq!(verification["activation_runtime_role"], "verifier");
        assert_eq!(verification["selected_tier"], "senior");
        assert_eq!(verification["selected_runtime_role"], "verifier");
        assert_eq!(verification["tier_default_runtime_role"], "verifier");
        assert_eq!(verification["rate"], 16);
        assert_eq!(verification["estimated_task_price_units"], 32);

        let architecture = assignment_for(
            "prepare the architecture and hard escalation plan for a cross cutting migration conflict",
        );
        assert_eq!(architecture["enabled"], true);
        assert_eq!(architecture["runtime_role"], "solution_architect");
        assert_eq!(architecture["activation_agent_type"], "architect");
        assert_eq!(
            architecture["activation_runtime_role"],
            "solution_architect"
        );
        assert_eq!(architecture["selected_tier"], "architect");
        assert_eq!(architecture["selected_runtime_role"], "solution_architect");
        assert_eq!(
            architecture["tier_default_runtime_role"],
            "solution_architect"
        );
        assert_eq!(architecture["rate"], 32);
        assert_eq!(architecture["estimated_task_price_units"], 128);
    }

    #[test]
    fn codex_dispatch_aliases_are_loaded_from_overlay_not_rust_catalog() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace("development_implementer:", "custom_impl_lane:");
        fs::write(&config_path, updated).expect("config should be rewritten");

        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let codex_config = fs::read_to_string(harness.path().join(".codex/config.toml"))
            .expect("rendered codex config should exist");
        assert!(!codex_config.contains("[agents.custom_impl_lane]"));
        assert!(!codex_config.contains("[agents.development_implementer]"));

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "write one bounded implementation patch",
        )
        .expect("selection should build");
        let plan = build_runtime_execution_plan_from_snapshot(&bundle, &selection);

        assert!(plan["codex_runtime_assignment"]
            .get("internal_named_lane_id")
            .is_none());
        assert_eq!(
            plan["development_flow"]["dispatch_contract"]["implementer_activation"]
                ["activation_agent_type"],
            "junior"
        );
        assert!(
            plan["development_flow"]["dispatch_contract"]["implementer_activation"]
                .get("internal_named_lane_id")
                .is_none()
        );
    }

    #[test]
    fn codex_dispatch_aliases_require_canonical_overlay_key() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace("dispatch_aliases:", "named_lanes:");
        fs::write(&config_path, updated).expect("config should be rewritten");

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let dispatch_aliases = bundle["codex_multi_agent"]["dispatch_aliases"]
            .as_array()
            .expect("dispatch aliases should still be an array");

        assert!(dispatch_aliases.is_empty());
    }

    #[test]
    fn agent_feedback_records_scorecard_and_refreshes_strategy() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-feedback",
                "--agent-id",
                "junior",
                "--score",
                "92",
                "--outcome",
                "success",
                "--task-class",
                "implementation",
                "--notes",
                "clean bounded closure",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let scorecards =
            read_json_file_if_present(&harness.path().join(CODEX_WORKER_SCORECARDS_STATE))
                .expect("scorecards should exist");
        let rows = scorecards["agents"]["junior"]["feedback"]
            .as_array()
            .expect("feedback rows should render");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["score"], 92);
        assert_eq!(rows[0]["outcome"], "success");
        assert_eq!(rows[0]["task_class"], "implementation");

        let strategy = read_json_file_if_present(&harness.path().join(CODEX_WORKER_STRATEGY_STATE))
            .expect("strategy should exist");
        assert!(
            strategy["agents"]["junior"]["effective_score"]
                .as_u64()
                .unwrap_or_default()
                >= 80
        );
        let observability =
            read_json_file_if_present(&harness.path().join(HOST_AGENT_OBSERVABILITY_STATE))
                .expect("observability ledger should exist");
        assert_eq!(
            observability["events"]
                .as_array()
                .expect("events should be an array")
                .len(),
            1
        );
        assert_eq!(observability["events"][0]["agent_id"], "junior");
    }

    #[test]
    fn merge_project_activation_marks_init_pending_when_activation_is_incomplete() {
        let init_view = serde_json::json!({
            "status": "ready"
        });
        let project_activation_view = serde_json::json!({
            "status": "pending_activation",
            "activation_pending": true,
            "project_shape": "bootstrapped",
            "triggers": {
                "config_state_incomplete": true
            },
            "activation_algorithm": {
                "taskflow_admitted_while_pending": false
            },
            "interview": {
                "required_inputs": []
            },
            "host_environment": {
                "selected_cli_system": serde_json::Value::Null
            },
            "next_steps": [
                "run `vida project-activator`"
            ]
        });

        let merged = merge_project_activation_into_init_view(init_view, &project_activation_view);

        assert_eq!(merged["status"], "pending_activation");
        assert_eq!(merged["project_activation"]["activation_pending"], true);
        assert_eq!(
            merged["project_activation"]["triggers"]["config_state_incomplete"],
            true
        );
        assert_eq!(
            merged["project_activation"]["activation_algorithm"]["taskflow_admitted_while_pending"],
            false
        );
    }

    #[test]
    fn orchestrator_init_succeeds_after_init_scaffold() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&["orchestrator-init", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn agent_init_succeeds_after_init_scaffold() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&["agent-init", "--role", "worker", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn compiled_agent_extension_bundle_merges_sidecar_overrides() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "  enabled_framework_roles:\n",
                "    - orchestrator\n",
                "    - worker\n",
                "  enabled_standard_flow_sets:\n",
                "    - minimal\n",
                "  enabled_project_roles:\n",
                "    - party_chat_facilitator\n",
                "  enabled_project_skills: []\n",
                "  enabled_project_profiles: []\n",
                "  enabled_project_flows: []\n",
                "  enabled_shared_skills: []\n",
                "  default_flow_set: minimal\n",
                "  validation:\n",
                "    require_registry_files: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: party_chat_facilitator\n",
                "    base_role: business_analyst\n",
                "    description: base\n",
            ),
        )
        .expect("base roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: party_chat_facilitator\n",
                "    base_role: business_analyst\n",
                "    description: overridden\n",
            ),
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile");
        assert_eq!(bundle["project_roles"][0]["description"], "overridden");
    }

    #[test]
    fn compiled_agent_extension_bundle_uses_registry_rows_when_enabled_lists_are_omitted() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "  enabled_framework_roles:\n",
                "    - orchestrator\n",
                "    - business_analyst\n",
                "    - coach\n",
                "    - verifier\n",
                "  validation:\n",
                "    require_registry_files: true\n",
                "    require_framework_role_compatibility: true\n",
                "    require_profile_resolution: true\n",
                "    require_flow_resolution: true\n",
                "    require_skill_role_compatibility: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: role_a\n",
                "    base_role: business_analyst\n",
                "    description: role a\n",
            ),
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            concat!(
                "version: 1\n",
                "skills:\n",
                "  - skill_id: skill_a\n",
                "    description: skill a\n",
                "    compatible_base_roles: business_analyst\n",
            ),
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            concat!(
                "version: 1\n",
                "profiles:\n",
                "  - profile_id: profile_a\n",
                "    role_ref: role_a\n",
                "    skill_refs: skill_a\n",
            ),
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            concat!(
                "version: 1\n",
                "flow_sets:\n",
                "  - flow_id: flow_a\n",
                "    role_chain: role_a\n",
            ),
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile from registries");
        assert_eq!(bundle["project_roles"][0]["role_id"], "role_a");
        assert_eq!(bundle["project_profiles"][0]["profile_id"], "profile_a");
        assert_eq!(bundle["project_flows"][0]["flow_id"], "flow_a");
    }

    #[test]
    fn compiled_agent_extension_bundle_fails_closed_on_invalid_profile_skill_and_flow_links() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "  enabled_framework_roles:\n",
                "    - business_analyst\n",
                "    - verifier\n",
                "  validation:\n",
                "    require_registry_files: true\n",
                "    require_framework_role_compatibility: true\n",
                "    require_profile_resolution: true\n",
                "    require_flow_resolution: true\n",
                "    require_skill_role_compatibility: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: role_a\n",
                "    base_role: business_analyst\n",
                "    description: role a\n",
            ),
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            concat!(
                "version: 1\n",
                "skills:\n",
                "  - skill_id: skill_a\n",
                "    description: skill a\n",
                "    compatible_base_roles: verifier\n",
            ),
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            concat!(
                "version: 1\n",
                "profiles:\n",
                "  - profile_id: profile_a\n",
                "    role_ref: role_a\n",
                "    skill_refs: skill_a,missing_skill\n",
            ),
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            concat!(
                "version: 1\n",
                "flow_sets:\n",
                "  - flow_id: flow_a\n",
                "    role_chain: role_a,missing_role\n",
            ),
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let error = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect_err("bundle should fail closed");
        assert!(error.contains("missing_skill"));
        assert!(error.contains("missing_role"));
        assert!(error.contains("incompatible skill `skill_a`"));
    }

    #[test]
    fn project_activator_command_accepts_json_output() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(
            runtime.block_on(run(cli(&["project-activator", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn orchestrator_init_view_exposes_protocol_view_targets() {
        let view = crate::taskflow_runtime_bundle::build_orchestrator_init_view(
            Path::new("/tmp/demo"),
            &serde_json::json!({"root_artifact_id": "root"}),
            &serde_json::json!({"startup_bundle": serde_json::Value::Null, "startup_capsules": []}),
            &serde_json::json!({"binding_status": "bound"}),
            &serde_json::json!({
                "always_on_core": [],
                "project_startup_bundle": [],
                "project_runtime_capsules": [],
                "task_specific_dynamic_context": [],
            }),
            "compatible",
            "no_migration_required",
        );
        assert_eq!(view["protocol_view_targets"][0], "bootstrap/router");
        assert_eq!(
            view["thinking_protocol_targets"][0],
            "instruction-contracts/overlay.step-thinking-runtime-capsule"
        );
        assert_eq!(view["allowed_thinking_modes"][0], "STC");
        assert_eq!(view["allowed_thinking_modes"][4], "META");
        assert!(
            view["minimum_commands"]
                .as_array()
                .expect("minimum commands should be an array")
                .iter()
                .any(|row| row == "vida protocol view agent-definitions/entry.orchestrator-entry"),
            "orchestrator init should surface protocol-view-friendly command hints"
        );
        assert!(
            view["minimum_commands"]
                .as_array()
                .expect("minimum commands should be an array")
                .iter()
                .any(|row| row
                    == "vida protocol view instruction-contracts/overlay.step-thinking-runtime-capsule"),
            "orchestrator init should surface the compact thinking bootstrap surface"
        );
    }

    #[test]
    fn agent_init_view_exposes_protocol_view_targets() {
        let view = crate::taskflow_runtime_bundle::build_agent_init_view(
            Path::new("/tmp/demo"),
            &serde_json::json!({"enabled_framework_roles": ["orchestrator", "worker"], "project_roles": []}),
            &serde_json::json!({"startup_capsules": []}),
            &serde_json::json!({"binding_status": "bound"}),
            "compatible",
            "no_migration_required",
        );
        assert_eq!(
            view["protocol_view_targets"][0],
            "agent-definitions/entry.worker-entry"
        );
        assert_eq!(
            view["thinking_protocol_targets"][0],
            "instruction-contracts/role.worker-thinking"
        );
        assert_eq!(view["allowed_thinking_modes"][0], "STC");
        assert_eq!(view["allowed_thinking_modes"][2], "MAR");
        assert!(
            view["minimum_commands"]
                .as_array()
                .expect("minimum commands should be an array")
                .iter()
                .any(|row| row == "vida protocol view instruction-contracts/role.worker-thinking"),
            "agent init should surface protocol-view-friendly command hints"
        );
    }

    #[test]
    fn init_bootstrap_source_requires_bootstrap_markers() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("bin")).expect("bin dir should exist");
        fs::write(root.join("bin/taskflow"), "#!/bin/sh\n").expect("taskflow marker should exist");
        assert!(
            !looks_like_init_bootstrap_source_root(root),
            "taskflow binary alone should not qualify as an init bootstrap source"
        );

        fs::create_dir_all(root.join("install/assets")).expect("install assets dir should exist");
        fs::create_dir_all(root.join(".codex")).expect(".codex dir should exist");
        fs::write(
            root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("generated AGENTS scaffold should exist");
        fs::write(
            root.join("install/assets/AGENTS.sidecar.scaffold.md"),
            "# sidecar\n",
        )
        .expect("sidecar scaffold should exist");
        fs::write(
            root.join("install/assets/vida.config.yaml.template"),
            "project:\n  id: demo\n",
        )
        .expect("config template should exist");
        assert!(
            looks_like_init_bootstrap_source_root(root),
            "bootstrap source should require actual init assets rather than runtime-only markers"
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_reports_pass_without_blockers() {
        let registry = RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: true,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let readiness = RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: true,
            row_count: 0,
            verdict: Some("ready".to_string()),
            artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
            output: String::new(),
        };
        let proof = RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "pass");
        assert!(verdict.ready);
        assert!(verdict.blockers.is_empty());
        assert_eq!(
            verdict.proof_surfaces,
            vec!["registry", "check", "readiness", "proof"]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_reports_explicit_blockers() {
        let registry = RuntimeConsumptionEvidence {
            surface: "registry".to_string(),
            ok: false,
            row_count: 0,
            verdict: None,
            artifact_path: None,
            output: String::new(),
        };
        let check = RuntimeConsumptionEvidence {
            surface: "check".to_string(),
            ok: false,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: "blocking check".to_string(),
        };
        let readiness = RuntimeConsumptionEvidence {
            surface: "readiness".to_string(),
            ok: false,
            row_count: 2,
            verdict: Some("blocked".to_string()),
            artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
            output: "blocking readiness".to_string(),
        };
        let proof = RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: false,
            row_count: 1,
            verdict: None,
            artifact_path: None,
            output: "❌ BLOCKING: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec![
                "missing_docflow_activation",
                "docflow_check_blocking",
                "missing_readiness_verdict",
                "missing_proof_verdict",
            ]
        );
        assert_eq!(
            verdict.proof_surfaces,
            vec!["registry", "check", "readiness", "proof"]
        );
    }

    #[test]
    fn taskflow_consume_final_closure_admission_reports_admit() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: true,
            blockers: vec![],
            root_artifact_id: "root".to_string(),
            artifact_count: 4,
            boot_classification: "compatible".to_string(),
            migration_state: "ready".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec![
                "vida docflow check --profile active-canon".to_string(),
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "status".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "ready_for_runtime_routing"
            }),
            reason: "test".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "admit");
        assert!(admission.admitted);
        assert!(admission.blockers.is_empty());
        assert_eq!(
            admission.proof_surfaces,
            vec![
                "vida taskflow consume bundle check",
                "vida docflow check --profile active-canon",
                "vida docflow readiness-check --profile active-canon",
                "vida docflow proofcheck --profile active-canon",
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_closure_admission_reports_fail_closed_blockers() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: false,
            blockers: vec!["boot_incompatible".to_string()],
            root_artifact_id: "root".to_string(),
            artifact_count: 0,
            boot_classification: "blocking".to_string(),
            migration_state: "blocked".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "block".to_string(),
            ready: false,
            blockers: vec![
                "missing_docflow_activation".to_string(),
                "missing_readiness_verdict".to_string(),
            ],
            proof_surfaces: vec!["vida docflow check --profile active-canon".to_string()],
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "status".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "blocked".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "blocked"
            }),
            reason: "test".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "block");
        assert!(!admission.admitted);
        assert_eq!(
            admission.blockers,
            vec![
                "missing_closure_proof",
                "missing_docflow_activation",
                "missing_readiness_verdict",
                "restore_reconcile_not_green",
            ]
        );
    }

    #[test]
    fn taskflow_consume_final_closure_admission_blocks_while_design_packet_is_pending() {
        let bundle_check = TaskflowConsumeBundleCheck {
            ok: true,
            blockers: vec![],
            root_artifact_id: "root".to_string(),
            artifact_count: 4,
            boot_classification: "compatible".to_string(),
            migration_state: "ready".to_string(),
        };
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec![
                "vida docflow check --profile active-canon".to_string(),
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "create a feature with research, specification, plan, and implementation"
                .to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec![
                "research".to_string(),
                "specification".to_string(),
                "implementation".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "design_first"
            }),
            reason: "auto_feature_design_request".to_string(),
        };

        let admission =
            build_runtime_closure_admission(&bundle_check, &docflow_verdict, &role_selection);

        assert_eq!(admission.status, "block");
        assert!(!admission.admitted);
        assert_eq!(admission.blockers, vec!["pending_design_packet"]);
    }
}
