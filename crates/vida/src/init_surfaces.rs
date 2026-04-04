use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::state_store::StateStore;

use super::{
    build_runtime_lane_selection_with_store, ensure_launcher_bootstrap, normalize_root_arg,
    print_surface_header, print_surface_line, role_exists_in_lane_bundle, state_store,
    sync_launcher_activation_snapshot, AgentInitArgs, BootArgs, InitArgs, RenderMode,
};
use crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload;

pub(crate) fn resolve_init_bootstrap_source_root() -> PathBuf {
    if let Some(installed_root) = resolve_installed_runtime_root() {
        if looks_like_init_bootstrap_source_root(&installed_root) {
            return installed_root;
        }
    }
    super::repo_runtime_root()
}

pub(crate) fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    taskflow_binary_candidates_for_root(root)
        .into_iter()
        .next()
        .map(|_| root.to_path_buf())
}

pub(crate) fn taskflow_binary_candidates_for_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let bin_dir = root.join("bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
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

    if let Ok(entries) = std::fs::read_dir(root) {
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

pub(crate) fn looks_like_init_bootstrap_source_root(root: &Path) -> bool {
    resolve_init_agents_source(root).is_ok()
        && resolve_init_sidecar_source(root).is_ok()
        && resolve_init_config_template_source(root).is_ok()
        && [".codex", ".qwen", ".kilo", ".opencode"]
            .into_iter()
            .any(|relative| root.join(relative).is_dir())
}

pub(crate) fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

pub(crate) fn resolve_init_agents_source(root: &Path) -> Result<PathBuf, String> {
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

pub(crate) fn resolve_init_sidecar_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [root.join("AGENTS.sidecar.md")];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve project sidecar source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_config_template_source(root: &Path) -> Result<PathBuf, String> {
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
        super::ensure_dir(&project_root.join(relative))?;
    }
    Ok(())
}

fn copy_file_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|error| {
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
        super::ensure_dir(parent)?;
    }
    std::fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))
}

fn write_runtime_agent_extension_projections(project_root: &Path) -> Result<(), String> {
    let root = super::project_activator_surface::runtime_agent_extensions_root(project_root);
    super::ensure_dir(&root)?;
    write_file_if_missing(
        &root.join("README.md"),
        super::DEFAULT_RUNTIME_AGENT_EXTENSIONS_README,
    )?;
    write_file_if_missing(
        &root.join("roles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_YAML,
    )?;
    write_file_if_missing(
        &root.join("roles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_SIDECAR_YAML,
    )?;

    let receipt_path = project_root.join(".vida/receipts/agent-extensions-bootstrap.json");
    if !receipt_path.exists() {
        let generated_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
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
                ".vida/project/agent-extensions/flows.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.yaml"
            ],
            "sidecar_projection_files": [
                ".vida/project/agent-extensions/roles.sidecar.yaml",
                ".vida/project/agent-extensions/skills.sidecar.yaml",
                ".vida/project/agent-extensions/profiles.sidecar.yaml",
                ".vida/project/agent-extensions/flows.sidecar.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"
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

pub(crate) async fn run_init(args: super::BootArgs) -> ExitCode {
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

    if let Err(error) = materialize_framework_agents_and_sidecar(
        &project_root,
        &framework_agents,
        &sidecar_scaffold,
    )
    .and_then(|()| copy_file_if_missing(&config_template, &project_root.join("vida.config.yaml")))
    .and_then(|()| materialize_project_docs_scaffold(&project_root))
    .and_then(|()| ensure_runtime_home(&project_root))
    .and_then(|()| write_runtime_agent_extension_projections(&project_root))
    {
        eprintln!("{error}");
        return ExitCode::from(1);
    }

    let activation_view =
        super::project_activator_surface::build_project_activator_view(&project_root);
    print_init_summary(&project_root, &activation_view);
    ExitCode::SUCCESS
}

fn materialize_project_docs_scaffold(project_root: &Path) -> Result<(), String> {
    let project_id = super::project_activator_surface::inferred_project_id_candidate(project_root);
    let project_title = super::inferred_project_title(&project_id, None);
    let source_root = resolve_init_bootstrap_source_root();
    let feature_template_source =
        source_root.join("docs/product/spec/templates/feature-design-document.template.md");
    let feature_template = std::fs::read_to_string(&feature_template_source).map_err(|error| {
        format!(
            "Failed to read framework feature-design template source {}: {error}",
            feature_template_source.display()
        )
    })?;

    let generated_files = vec![
        (
            project_root.join("README.md"),
            render_project_readme(&project_title),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ROOT_MAP),
            render_project_root_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_INDEX),
            render_project_product_index(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_SPEC_README),
            render_project_product_spec_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE),
            feature_template,
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ARCHITECTURE_DOC),
            render_project_architecture_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PROCESS_README),
            render_project_process_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DECISIONS_DOC),
            with_scaffold_footer(
                "# Decisions\n\nRecord bounded architecture and product decisions here.\n",
                "process/decisions",
                "process_doc",
                "docs/process/decisions.md",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ENVIRONMENTS_DOC),
            render_project_environments_doc(project_root),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_OPERATIONS_DOC),
            render_project_operations_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_AGENT_SYSTEM_DOC),
            render_project_agent_system_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DOC_TOOLING_DOC),
            render_project_doc_tooling_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC),
            render_project_codex_guide(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_RESEARCH_README),
            render_project_research_readme(),
        ),
    ];

    for (path, content) in generated_files {
        write_file_if_missing(&path, &content)?;
        if let Ok(relative_source_path) = path.strip_prefix(project_root) {
            write_scaffold_changelog_if_missing(
                &path,
                relative_source_path,
                scaffold_artifact_path_for(relative_source_path),
                scaffold_artifact_type_for(relative_source_path),
            )?;
        }
    }

    Ok(())
}

pub(crate) fn render_project_readme(project_title: &str) -> String {
    with_scaffold_footer(
        &format!(
        "# {project_title}\n\n\
This repository contains a VIDA-initialized project scaffold.\n\n\
Use `AGENTS.md` for framework bootstrap, `AGENTS.sidecar.md` for project docs routing, and `docs/` for project-owned operating context.\n"
        ),
        "project/readme",
        "document",
        "README.md",
    )
}

pub(crate) fn render_project_root_map() -> String {
    with_scaffold_footer(
        &format!(
            "# Project Root Map\n\n\
This project uses the following canonical documentation roots:\n\n\
- `docs/product/` for product-facing intent and architecture notes\n\
- `docs/process/` for project operations and working agreements\n\
- `docs/research/` for research notes and discovery artifacts\n\n\
Primary pointers:\n\n\
- Product index: `{}`\n\
- Product spec/readiness guide: `{}`\n\
- Feature design template: `{}`\n\
- Process index: `{}`\n\
- Documentation tooling: `{}`\n\
- Codex agent guide: `{}`\n\
- Research index: `{}`\n\
- Repository overview: `README.md`\n",
            super::DEFAULT_PROJECT_PRODUCT_INDEX,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README,
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            super::DEFAULT_PROJECT_PROCESS_README,
            super::DEFAULT_PROJECT_DOC_TOOLING_DOC,
            super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
            super::DEFAULT_PROJECT_RESEARCH_README
        ),
        "project/root-map",
        "document",
        "docs/project-root-map.md",
    )
}

pub(crate) fn render_project_product_index() -> String {
    with_scaffold_footer(
        &format!(
            "# Product Index\n\n\
Product documentation currently contains:\n\n\
- `{}` for the initial project architecture outline\n\
- `{}` for bounded feature/change design and ADR routing\n",
            super::DEFAULT_PROJECT_ARCHITECTURE_DOC,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README
        ),
        "product/index",
        "product_index",
        "docs/product/index.md",
    )
}

pub(crate) fn render_project_product_spec_readme() -> String {
    with_scaffold_footer(
        &format!(
        "# Product Spec Guide\n\n\
Use this directory for bounded product-facing feature/change design documents and linked ADRs.\n\n\
Default rule:\n\n\
1. If a request asks for research, detailed specifications, implementation planning, and then code, create or update one bounded design document before implementation.\n\
2. Start from the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before normal implementation work begins.\n\
4. Use `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check` to keep the document canonical.\n\
5. Close the spec-pack task only after the design artifact is finalized and validated, then hand off through the next TaskFlow packet.\n\
6. When one major decision needs durable standalone recording, add a linked ADR instead of overloading the design document.\n\
\n\
Suggested homes:\n\n\
- `docs/product/spec/<feature>-design.md` for committed feature/change designs\n\
- `docs/research/<topic>.md` for exploratory research before design closure\n",
        super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "product/spec/readme",
        "product_spec",
        "docs/product/spec/README.md",
    )
}

pub(crate) fn render_project_architecture_doc() -> String {
    with_scaffold_footer(
        "# Architecture\n\nCurrent project posture:\n\n- VIDA bootstrap scaffold is initialized\n- project documentation roots are materialized\n- project-specific implementation modules are not yet defined\n",
        "product/architecture",
        "document",
        "docs/product/architecture.md",
    )
}

pub(crate) fn render_project_process_readme() -> String {
    with_scaffold_footer(
        "# Process Docs\n\nThis directory contains the minimum process documentation expected by VIDA activation.\n\nAvailable process docs:\n\n- `decisions.md`\n- `environments.md`\n- `project-operations.md`\n- `agent-system.md`\n- `documentation-tooling-map.md`\n- `codex-agent-configuration-guide.md`\n",
        "process/readme",
        "process_doc",
        "docs/process/README.md",
    )
}

pub(crate) fn render_project_decisions_doc(answers: &super::ProjectActivationAnswers) -> String {
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

pub(crate) fn render_project_environments_doc(project_root: &Path) -> String {
    with_scaffold_footer(
        &format!(
            "# Environments\n\n\
Initial environment assumptions:\n\n\
- local project root: `{}`\n\
- VIDA runtime directories are managed under `.vida/`\n\
- host CLI agent template is selected through `vida project-activator`\n",
            project_root.display()
        ),
        "process/environments",
        "process_doc",
        "docs/process/environments.md",
    )
}

pub(crate) fn render_project_operations_doc() -> String {
    with_scaffold_footer(
        &format!(
        "# Project Operations\n\n\
Current operating baseline:\n\n\
- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces\n\
- use `AGENTS.sidecar.md` as the project documentation map\n\
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`\n\
\n\
Default feature-delivery flow:\n\n\
1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.\n\
2. Use the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.\n\
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.\n\
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.\n\
6. When `.codex/**` is materialized, use the delegated Codex team surface instead of collapsing the root session directly into coding.\n\
7. Treat `vida.config.yaml` as the owner of carrier tiers and optional internal Codex aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.\n\
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.\n\
9. For normal write-producing work, treat project agent-first execution as the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the canonical project control surface.\n\
10. Keep the root session in orchestration posture unless an explicit exception path is recorded.\n",
        super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/project-operations",
        "process_doc",
        "docs/process/project-operations.md",
    )
}

pub(crate) fn render_project_agent_system_doc() -> String {
    with_scaffold_footer(
        "# Agent System\n\nProject activation owns host CLI agent-template selection and runtime admission.\n\n- default framework host templates become available only after the selected host CLI template is materialized\n- supported host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`\n- built-in template roots currently include `.codex/**`, `.qwen/**`, `.kilo/**`, and `.opencode/**`\n- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers` (Codex additionally keeps `vida.config.yaml -> host_environment.codex.agents` as the rendered tier-catalog source)\n- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model\n- the selected runtime surface is rendered under the configured runtime root and is not the owner of tier/rate/task-class policy\n- project activation materializes the selected host template using the configured `materialization_mode`; Codex renders `.codex/config.toml` and `.codex/agents/*.toml`, while external CLI systems use their own runtime root\n- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`\n- project-local agent extensions remain under `.vida/project/agent-extensions/`\n- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists\n- project \"agent-first\" development means the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier mechanics and not the canonical execution contract\n",
        "process/agent-system",
        "process_doc",
        "docs/process/agent-system.md",
    )
}

pub(crate) fn render_project_doc_tooling_map() -> String {
    with_scaffold_footer(
        &format!(
        "# Documentation Tooling Map\n\n\
Use `vida docflow` for documentation inventory, mutation, validation, and readiness checks.\n\n\
Design-document rule:\n\n\
1. For bounded feature/change work that requires research, detailed specifications, planning, and implementation, begin with one design document before code execution.\n\
2. Start from `{}`.\n\
3. Open one epic and one spec-pack task in `vida taskflow` before writing code.\n\
4. Suggested command sequence:\n\
   - `vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec \"initialize feature design\"`\n\
   - edit the document using the local template shape\n\
   - `vida docflow finalize-edit docs/product/spec/<feature>-design.md \"record bounded feature design\"`\n\
   - `vida docflow check --root . docs/product/spec/<feature>-design.md`\n\
   - `vida task close <spec-task-id> --reason \"design packet finalized and handed off\" --json`\n\
\n\
Activation rule:\n\n\
1. During project activation, `vida project-activator` owns bounded config/doc materialization.\n\
2. `vida taskflow` and any non-canonical external TaskFlow runtime are not lawful activation-entry surfaces while activation is pending.\n\
3. After activation writes, prefer `vida docflow` for documentation-oriented inspection and proof before multi-step implementation.\n",
        super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/documentation-tooling-map",
        "process_doc",
        "docs/process/documentation-tooling-map.md",
    )
}

pub(crate) fn render_project_research_readme() -> String {
    with_scaffold_footer(
        "# Research Notes\n\nUse this directory for research artifacts, discovery notes, and external references that support future project work.\n",
        "research/readme",
        "document",
        "docs/research/README.md",
    )
}

pub(crate) fn render_project_codex_guide() -> String {
    with_scaffold_footer(
        "# Codex Agent Configuration Guide\n\nThis project uses framework-materialized `.codex/**` as the local Codex runtime surface.\n\nSource-of-truth rule:\n\n- `vida.config.yaml -> host_environment.codex.agents` owns carrier-tier metadata, rates, runtime-role fit, and task-class fit\n- `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` owns the dispatch-alias registry for executor-local overlays\n- `.codex/**` is the rendered executor surface used by Codex after activation\n- `.codex/config.toml` should expose the carrier tiers materialized from overlay\n\nCarrier rule:\n\n- the primary visible agent model is `junior`, `middle`, `senior`, `architect`\n- runtime role remains explicit activation state such as `worker`, `coach`, `verifier`, or `solution_architect`\n- internal alias ids may exist in registry state, but they must not replace the carrier-tier model at the project surface\n\nWorking rule:\n\n1. The root session stays the orchestrator.\n2. Documentation/specification work should complete the bounded design document first.\n3. Before delegated implementation starts, open the feature epic/spec task in `vida taskflow` and close the spec task only after the design artifact is finalized.\n4. After a bounded packet exists, route research, specification, planning, implementation, review, and verification through the configured tier ladder instead of collapsing into root-session coding.\n5. Let runtime choose the cheapest capable configured carrier tier with a healthy local score from `.vida/state/worker-strategy.json` and pass the lawful runtime role explicitly.\n6. Canonical delegated execution still dispatches through `vida agent-init`; host-tool-specific Codex subagent APIs are optional executor details and not the primary project delegation surface.\n7. Use `.vida/project/agent-extensions/**` for project-local role and skill overlays; do not treat `.codex/**` as the owner of framework or product law.\n",
        "process/codex-agent-configuration-guide",
        "process_doc",
        "docs/process/codex-agent-configuration-guide.md",
    )
}

fn with_scaffold_footer(
    body: &str,
    artifact_path: &str,
    artifact_type: &str,
    source_path: &str,
) -> String {
    let changelog_ref = scaffold_changelog_ref_for(source_path);
    format!(
        "{body}\n-----\nartifact_path: {artifact_path}\nartifact_type: {artifact_type}\nartifact_version: '1'\nartifact_revision: '2026-04-04'\nschema_version: '1'\nstatus: scaffold\nsource_path: {source_path}\ncreated_at: '2026-04-04T00:00:00Z'\nupdated_at: '2026-04-04T00:00:00Z'\nchangelog_ref: {changelog_ref}\n"
    )
}

fn scaffold_changelog_ref_for(source_path: &str) -> String {
    let source_path = Path::new(source_path);
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    format!("{stem}.changelog.jsonl")
}

fn scaffold_artifact_path_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "README.md" => "project/readme",
        "docs/project-root-map.md" => "project/root-map",
        "docs/product/index.md" => "product/index",
        "docs/product/architecture.md" => "product/architecture",
        "docs/product/spec/README.md" => "product/spec/readme",
        "docs/product/spec/templates/feature-design-document.template.md" => {
            "product/spec/templates/feature-design-document.template"
        }
        "docs/process/README.md" => "process/readme",
        "docs/process/agent-system.md" => "process/agent-system",
        "docs/process/codex-agent-configuration-guide.md" => {
            "process/codex-agent-configuration-guide"
        }
        "docs/process/decisions.md" => "process/decisions",
        "docs/process/documentation-tooling-map.md" => "process/documentation-tooling-map",
        "docs/process/environments.md" => "process/environments",
        "docs/process/project-operations.md" => "process/project-operations",
        "docs/research/README.md" => "research/readme",
        _ => "project/scaffold-doc",
    }
}

fn scaffold_artifact_type_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "docs/process/README.md"
        | "docs/process/agent-system.md"
        | "docs/process/codex-agent-configuration-guide.md"
        | "docs/process/decisions.md"
        | "docs/process/documentation-tooling-map.md"
        | "docs/process/environments.md"
        | "docs/process/project-operations.md" => "process_doc",
        "docs/product/index.md" => "product_index",
        "docs/product/spec/README.md"
        | "docs/product/spec/templates/feature-design-document.template.md" => "product_spec",
        _ => "document",
    }
}

fn write_scaffold_changelog_if_missing(
    absolute_source_path: &Path,
    relative_source_path: &Path,
    artifact_path: &str,
    artifact_type: &str,
) -> Result<(), String> {
    let parent = absolute_source_path.parent().ok_or_else(|| {
        format!(
            "Failed to determine scaffold parent directory for {}",
            absolute_source_path.display()
        )
    })?;
    let stem = absolute_source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    let changelog_path = parent.join(format!("{stem}.changelog.jsonl"));
    let entry = format!(
        "{{\"ts\":\"2026-04-04T00:00:00Z\",\"event\":\"metadata_initialized\",\"artifact_path\":\"{artifact_path}\",\"artifact_type\":\"{artifact_type}\",\"artifact_version\":\"1\",\"artifact_revision\":\"2026-04-04\",\"source_path\":\"{}\",\"reason\":\"initialize scaffold metadata for docflow-ready project bootstrap\",\"actor\":\"vida\",\"scope\":\"scaffold-init\",\"tags\":[\"scaffold\",\"docflow\"]}}\n",
        relative_source_path.display()
    );
    write_file_if_missing(&changelog_path, &entry)
}

fn materialize_framework_agents_and_sidecar(
    project_root: &Path,
    framework_agents: &Path,
    sidecar_scaffold: &Path,
) -> Result<(), String> {
    let agents = project_root.join("AGENTS.md");
    let sidecar = project_root.join("AGENTS.sidecar.md");
    let framework_contents = std::fs::read_to_string(framework_agents)
        .map_err(|error| format!("Failed to read {}: {error}", framework_agents.display()))?;

    if agents.is_file() {
        let existing_agents = std::fs::read_to_string(&agents)
            .map_err(|error| format!("Failed to read {}: {error}", agents.display()))?;
        if existing_agents != framework_contents {
            if !sidecar.is_file()
                || super::project_activator_surface::file_contains_placeholder(&sidecar)
            {
                if let Some(parent) = sidecar.parent() {
                    super::ensure_dir(parent)?;
                }
                std::fs::write(&sidecar, existing_agents).map_err(|error| {
                    format!(
                        "Failed to preserve existing {} as {}: {error}",
                        agents.display(),
                        sidecar.display()
                    )
                })?;
            } else {
                let backup_path = project_root.join(".vida/receipts/AGENTS.pre-init.backup.md");
                if let Some(parent) = backup_path.parent() {
                    super::ensure_dir(parent)?;
                }
                if !backup_path.exists() {
                    std::fs::write(&backup_path, existing_agents).map_err(|error| {
                        format!("Failed to write {} backup: {error}", backup_path.display())
                    })?;
                }
            }
        }
    }

    copy_file_if_missing(sidecar_scaffold, &sidecar)?;
    std::fs::write(&agents, framework_contents)
        .map_err(|error| format!("Failed to write {}: {error}", agents.display()))
}

fn print_init_summary(project_root: &Path, activation_view: &serde_json::Value) {
    println!("vida init project bootstrap ready");
    println!("project root: {}", project_root.display());
    println!(
        "materialized: AGENTS.md, AGENTS.sidecar.md, vida.config.yaml, README.md, docs/project-root-map.md, docs/product/**, docs/process/**, docs/research/README.md, .vida/config, .vida/db, .vida/cache, .vida/framework, .vida/project, .vida/project/agent-extensions/*, .vida/project/agent-extensions/*.sidecar.yaml, .vida/receipts, .vida/runtime, .vida/scratchpad"
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
        println!(
            "activation rule: while activation is pending, use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime"
        );
    }
}

pub(crate) async fn run_boot(args: BootArgs) -> ExitCode {
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
                                    if crate::release1_contracts::canonical_compatibility_class_str(
                                        &compatibility.classification,
                                    ) != Some(
                                        crate::release1_contracts::CompatibilityClass::BackwardCompatible
                                            .as_str(),
                                    ) {
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
                                        eprintln!(
                                            "Failed to resolve effective instruction bundle: {error}"
                                        );
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

pub(crate) async fn run_orchestrator_init(args: InitArgs) -> ExitCode {
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
                        Ok(path) => {
                            super::project_activator_surface::build_project_activator_view(&path)
                        }
                        Err(error) => {
                            eprintln!("Failed to resolve current directory: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let init_view =
                        super::project_activator_surface::merge_project_activation_into_init_view(
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
                            if let Some(rule) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["selection_rule"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent model",
                                    "agent=execution carrier; role=runtime activation state",
                                );
                                print_surface_line(RenderMode::Plain, "carrier selection", rule);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["snapshot"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent snapshot cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["carrier_catalog"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier catalog cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["runtime_roles"]
                                .as_str()
                            {
                                print_surface_line(RenderMode::Plain, "runtime roles cmd", command);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["scores"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier scores cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["selection_preview"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "selection preview cmd",
                                    command,
                                );
                            }
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

pub(crate) async fn run_agent_init(args: AgentInitArgs) -> ExitCode {
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
            let packet_arg_count = usize::from(args.dispatch_packet.is_some())
                + usize::from(args.downstream_packet.is_some());
            if packet_arg_count > 1 {
                eprintln!(
                    "Agent init accepts at most one packet source: use either `--dispatch-packet` or `--downstream-packet`."
                );
                return ExitCode::from(2);
            }
            let selection = if let Some(packet_path) = args.dispatch_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                let selected_role = packet
                    .get("activation_runtime_role")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        packet
                            .get("role_selection")
                            .and_then(|value| value.get("selected_role"))
                            .and_then(serde_json::Value::as_str)
                    })
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unknown");
                if selected_role == "orchestrator" || selected_role == "unknown" {
                    eprintln!(
                        "Dispatch packet activation requires a non-orchestrator runtime role."
                    );
                    return ExitCode::from(2);
                }
                serde_json::json!({
                    "mode": "dispatch_packet",
                    "selected_role": selected_role,
                    "request_text": packet.get("request_text").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "dispatch_target": packet.get("dispatch_target").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "dispatch_packet_path": packet_path,
                    "packet_kind": packet.get("packet_kind").cloned().unwrap_or(serde_json::Value::Null),
                    "packet_template_kind": packet.get("packet_template_kind").cloned().unwrap_or(serde_json::Value::Null),
                    "packet": packet,
                })
            } else if let Some(packet_path) = args.downstream_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                let selected_role = packet
                    .get("activation_runtime_role")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        packet
                            .get("role_selection")
                            .and_then(|value| value.get("selected_role"))
                            .and_then(serde_json::Value::as_str)
                    })
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unknown");
                if selected_role == "orchestrator" || selected_role == "unknown" {
                    eprintln!(
                        "Downstream packet activation requires a non-orchestrator runtime role."
                    );
                    return ExitCode::from(2);
                }
                serde_json::json!({
                    "mode": "downstream_packet",
                    "selected_role": selected_role,
                    "request_text": packet.get("request_text").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "dispatch_target": packet.get("downstream_dispatch_target").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "downstream_packet_path": packet_path,
                    "packet_kind": packet.get("packet_kind").cloned().unwrap_or(serde_json::Value::Null),
                    "packet_template_kind": packet.get("packet_template_kind").cloned().unwrap_or(serde_json::Value::Null),
                    "packet": packet,
                })
            } else if let Some(role) = args.role.clone() {
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
                Ok(path) => super::project_activator_surface::build_project_activator_view(&path),
                Err(error) => {
                    eprintln!("Failed to resolve current directory: {error}");
                    return ExitCode::from(1);
                }
            };
            let init_view =
                super::project_activator_surface::merge_project_activation_into_init_view(
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
                if let Some(mode) = selection["mode"].as_str() {
                    print_surface_line(RenderMode::Plain, "mode", mode);
                }
                if let Some(path) = selection["dispatch_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "dispatch packet", path);
                }
                if let Some(path) = selection["downstream_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "downstream packet", path);
                }
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
