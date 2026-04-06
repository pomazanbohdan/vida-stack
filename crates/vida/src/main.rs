mod activation_status;
mod agent_feedback_surface;
mod cli;
mod config_value_utils;
mod docflow_proxy;
mod doctor_surface;
mod host_agent_state;
mod init_surfaces;
mod memory_surface;
mod operator_contracts;
mod project_activator_surface;
mod protocol_surface;
mod release1_contracts;
mod root_command_router;
mod runtime_consumption_state;
mod runtime_dispatch_state;
mod state_store;
mod status_surface;
mod surface_render;
mod task_cli_render;
mod task_surface;
mod taskflow_consume;
mod taskflow_consume_bundle;
mod taskflow_consume_resume;
mod taskflow_layer4;
mod taskflow_protocol_binding;
mod taskflow_proxy;
mod taskflow_routing;
mod taskflow_run_graph;
mod taskflow_runtime_bundle;
mod taskflow_spec_bootstrap;
mod taskflow_task_bridge;
mod temp_state;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

use clap::Parser;
pub(crate) use cli::*;
pub(crate) use config_value_utils::{
    csv_json_string_list, json_bool, json_lookup, json_string, json_string_list,
    load_project_overlay_yaml, split_csv_like, yaml_bool, yaml_lookup, yaml_string,
    yaml_string_list,
};
pub(crate) use host_agent_state::{
    append_host_agent_observability_event, build_carrier_pricing_policy,
    host_agent_observability_state_path, load_or_initialize_host_agent_observability_state,
    load_or_initialize_worker_scorecards, read_json_file_if_present, refresh_worker_strategy,
    worker_scorecards_state_path, worker_strategy_state_path, HostAgentFeedbackInput,
};
pub(crate) use init_surfaces::resolve_init_bootstrap_source_root;
pub(crate) use project_activator_surface::build_project_activator_view;
pub(crate) use project_activator_surface::merge_project_activation_into_init_view;
pub(crate) use project_activator_surface::ProjectActivationAnswers;
use release1_contracts::{
    blocker_code_str, blocker_code_value, canonical_release1_contract_status_str,
    derive_lane_status, missing_downstream_lane_evidence_blocker, BlockerCode, LaneStatus,
};
use root_command_router::run_root_command;
#[cfg(test)]
use runtime_consumption_state::runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result;
use runtime_consumption_state::{
    apply_runtime_consumption_final_dispatch_receipt_blocker,
    runtime_consumption_final_dispatch_receipt_blocker_code,
};
pub(crate) use runtime_consumption_state::{
    latest_final_runtime_consumption_snapshot_path,
    runtime_consumption_snapshot_has_release_admission_evidence, runtime_consumption_summary,
    write_runtime_consumption_snapshot,
};
pub(crate) use runtime_dispatch_state::*;
use state_store::{LauncherActivationSnapshot, StateStore, StateStoreError};
pub(crate) use surface_render::{
    print_root_help, print_surface_header, print_surface_line, print_surface_ok,
};
use task_cli_render::{
    print_blocked_tasks, print_task_critical_path, print_task_dependencies,
    print_task_dependency_mutation, print_task_dependency_tree, print_task_export_summary,
    print_task_graph_issues, print_task_list, print_task_mutation, print_task_next_display_id,
    print_task_ready, print_task_show,
};
use taskflow_layer4::print_taskflow_proxy_help;
use taskflow_proxy::run_taskflow_proxy;
pub(crate) use taskflow_routing::{
    dispatch_contract_execution_lane_sequence, dispatch_contract_lane,
    dispatch_contract_lane_activation, dispatch_contract_lane_sequence,
    selected_backend_from_execution_plan_route,
};
use taskflow_runtime_bundle::{
    blocking_runtime_bundle, build_taskflow_consume_bundle_payload, taskflow_consume_bundle_check,
};
use taskflow_spec_bootstrap::{
    execute_taskflow_bootstrap_spec_with_store, execute_work_packet_create_with_store,
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
const DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/dispatch-aliases.yaml");
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
const DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_SIDECAR_YAML: &str =
    "version: 1\ndispatch_aliases: []\n";
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
const DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC: &str =
    "docs/process/codex-agent-configuration-guide.md";
const DEFAULT_PROJECT_DOC_TOOLING_DOC: &str = "docs/process/documentation-tooling-map.md";
const DEFAULT_PROJECT_RESEARCH_README: &str = "docs/research/README.md";
const PROJECT_ACTIVATION_RECEIPT_LATEST: &str = ".vida/receipts/project-activation.latest.json";
const SPEC_BOOTSTRAP_RECEIPT_LATEST: &str = ".vida/receipts/spec-bootstrap.latest.json";
const WORKER_SCORECARDS_STATE: &str = ".vida/state/worker-scorecards.json";
const WORKER_STRATEGY_STATE: &str = ".vida/state/worker-strategy.json";
const HOST_AGENT_OBSERVABILITY_STATE: &str = ".vida/state/host-agent-observability.json";
const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER: &str =
    "run_graph_latest_dispatch_receipt_summary_inconsistent";
const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION: &str =
    "Refresh the latest run-graph dispatch receipt summary before rerunning consume-final.";
const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER: &str =
    "run_graph_latest_dispatch_receipt_checkpoint_leakage";
const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION: &str =
    "Refresh the latest checkpoint evidence before rerunning consume-final so the latest status and checkpoint rows share the same run_id.";

#[tokio::main]
async fn main() -> ExitCode {
    run_root_command(Cli::parse()).await
}

#[cfg(test)]
pub(crate) async fn run(cli: Cli) -> ExitCode {
    run_root_command(cli).await
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
    if current_dir.starts_with(&repo_root)
        && init_surfaces::looks_like_init_bootstrap_source_root(&repo_root)
    {
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
    if looks_like_project_root(&root) || init_surfaces::looks_like_init_bootstrap_source_root(&root)
    {
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

pub(crate) fn resolve_status_project_root(state_root: &Path) -> Option<PathBuf> {
    taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .or_else(|| resolve_runtime_project_root().ok())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("Failed to create {}: {error}", path.display()))
}

fn trimmed_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
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

fn build_task_create_command(
    task_id: &str,
    title: &str,
    task_type: &str,
    parent_id: Option<&str>,
    labels: &[&str],
    description_quoted: Option<&str>,
) -> String {
    let mut command = format!(
        "vida task create {} {} --type {} --status open",
        task_id,
        shell_quote(title),
        task_type
    );
    if let Some(parent_id) = parent_id {
        command.push_str(&format!(" --parent-id {parent_id}"));
    }
    for label in labels {
        command.push_str(&format!(" --labels {label}"));
    }
    if let Some(description_quoted) = description_quoted {
        command.push_str(&format!(" --description {description_quoted}"));
    }
    command.push_str(" --json");
    command
}

fn build_task_ensure_command(
    task_id: &str,
    title: &str,
    task_type: &str,
    parent_id: Option<&str>,
    labels: &[&str],
    description_quoted: Option<&str>,
) -> String {
    let mut command = format!(
        "vida task ensure {} {} --type {} --status open",
        task_id,
        shell_quote(title),
        task_type
    );
    if let Some(parent_id) = parent_id {
        command.push_str(&format!(" --parent-id {parent_id}"));
    }
    for label in labels {
        command.push_str(&format!(" --labels {label}"));
    }
    if let Some(description_quoted) = description_quoted {
        command.push_str(&format!(" --description {description_quoted}"));
    }
    command.push_str(" --json");
    command
}

fn build_task_show_command(task_id: &str) -> String {
    format!("vida task show {task_id} --json")
}

fn build_task_close_command(task_id: &str, reason: &str) -> String {
    format!(
        "vida task close {} --reason {} --json",
        task_id,
        shell_quote(reason)
    )
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

fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    })
}

pub(crate) fn carrier_runtime_section<'a>(
    compiled_bundle: &'a serde_json::Value,
) -> &'a serde_json::Value {
    compiled_bundle
        .get("carrier_runtime")
        .or_else(|| compiled_bundle.get("codex_multi_agent"))
        .unwrap_or(&serde_json::Value::Null)
}

fn runtime_assignment_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
) -> &'a serde_json::Value {
    execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("carrier_runtime_assignment"))
        .or_else(|| execution_plan.get("codex_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
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

fn print_json_pretty(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("json payload should render")
    );
}

fn runtime_assignment_alias_fields(
    runtime_assignment: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "carrier_runtime_assignment".to_string(),
        runtime_assignment.clone(),
    );
    fields.insert("runtime_assignment".to_string(), runtime_assignment.clone());
    fields
}

fn infer_task_class_from_task_payload(task: &serde_json::Value) -> String {
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
    }) || !contains_keywords(
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
    .is_empty()
    {
        return "architecture".to_string();
    }
    if labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "verification" | "review" | "proof" | "release-readiness"
        )
    }) || !contains_keywords(
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
    .is_empty()
    {
        return "verification".to_string();
    }
    if labels
        .iter()
        .any(|label| matches!(label.as_str(), "spec-pack" | "documentation" | "planning"))
        || !contains_keywords(
            &normalized,
            &[
                "spec".to_string(),
                "design".to_string(),
                "research".to_string(),
                "plan".to_string(),
                "requirements".to_string(),
            ],
        )
        .is_empty()
    {
        return "specification".to_string();
    }
    "implementation".to_string()
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
    if crate::release1_contracts::canonical_compatibility_class_str(&compatibility.classification)
        != Some(crate::release1_contracts::CompatibilityClass::BackwardCompatible.as_str())
    {
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
    taskflow_protocol_binding::sync_taskflow_protocol_binding_snapshot(store).await?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct DoctorLauncherSummary {
    vida: String,
    project_root: String,
    taskflow_surface: String,
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
    activation_status: String,
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

fn config_file_path() -> Result<PathBuf, String> {
    Ok(resolve_runtime_project_root()?.join("vida.config.yaml"))
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
    let selection_mode = json_string(json_lookup(bundle, &["role_selection", "mode"]))
        .unwrap_or_else(|| "fixed".to_string());
    let configured_fallback =
        json_string(json_lookup(bundle, &["role_selection", "fallback_role"]))
            .unwrap_or_else(|| "orchestrator".to_string());
    if !role_exists_in_lane_bundle(bundle, &configured_fallback) {
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
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }

    let Some(serde_json::Value::Object(conversation_modes)) =
        json_lookup(bundle, &["role_selection", "conversation_modes"])
    else {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    };
    if normalized_request.trim().is_empty() {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
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
        if !role_exists_in_lane_bundle(bundle, &selected_role) {
            return Err(format!(
                "Agent extension bundle validation failed: conversation mode `{mode_id}` references unresolved role `{selected_role}`."
            ));
        }
        let tracked_flow_entry = json_string(json_lookup(mode_value, &["tracked_flow_entry"]));
        if let Some(flow_id) = tracked_flow_entry.as_deref() {
            if !tracked_flow_target_exists(bundle, flow_id) {
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
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
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
                result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
                return Ok(result);
            }
        }

        result.reason = "auto_no_keyword_match".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }
    if !role_exists_in_lane_bundle(bundle, &selected.1) {
        result.reason = "auto_selected_unknown_role".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
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
    result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
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
    let runtime_assignment = if runtime_role.is_empty() || task_class.is_empty() {
        serde_json::Value::Null
    } else {
        build_runtime_assignment_from_resolved_constraints(
            compiled_bundle,
            route_id,
            task_class,
            runtime_role,
        )
    };
    let mut route_summary = serde_json::json!({
        "route_id": route_id,
        "carrier_backend_hint": json_string(json_lookup(route, &["subagents"])).unwrap_or_default(),
        "subagents": json_string(json_lookup(route, &["subagents"])).unwrap_or_default(),
        "fanout_subagents": json_string(json_lookup(route, &["fanout_subagents"])).unwrap_or_default(),
        "preferred_agent_type": runtime_assignment["selected_agent_id"],
        "preferred_agent_tier": runtime_assignment["selected_tier"],
        "preferred_runtime_role": runtime_assignment["runtime_role"],
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
    });
    if let Some(summary) = route_summary.as_object_mut() {
        summary.extend(runtime_assignment_alias_fields(&runtime_assignment));
    }
    route_summary
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
            "create_command": build_task_create_command(
                &epic_task_id,
                &epic_title,
                "epic",
                None,
                &["feature-request", "spec-first"],
                Some(&quoted_request),
            ),
            "close_command": build_task_close_command(
                &epic_task_id,
                "feature delivery closed after proof and runtime handoff",
            )
        },
        "spec_task": {
            "required": true,
            "task_id": spec_task_id,
            "title": spec_title,
            "runtime": "vida taskflow",
            "inspect_command": build_task_show_command(&spec_task_id),
            "ensure_command": build_task_ensure_command(
                &spec_task_id,
                &spec_title,
                "task",
                Some(&epic_task_id),
                &["spec-pack", "documentation"],
                Some(&shell_quote("bounded design/spec packet for the feature request")),
            ),
            "create_command": build_task_create_command(
                &spec_task_id,
                &spec_title,
                "task",
                Some(&epic_task_id),
                &["spec-pack", "documentation"],
                Some(&shell_quote("bounded design/spec packet for the feature request")),
            ),
            "close_command": build_task_close_command(
                &spec_task_id,
                "design packet finalized and handed off into tracked work-pool shaping",
            )
        },
        "work_pool_task": {
            "required": true,
            "task_id": work_pool_task_id,
            "title": work_pool_title,
            "runtime": "vida taskflow",
            "inspect_command": build_task_show_command(&work_pool_task_id),
            "ensure_command": build_task_ensure_command(
                &work_pool_task_id,
                &work_pool_title,
                "task",
                Some(&epic_task_id),
                &["work-pool-pack"],
                None,
            ),
            "create_command": build_task_create_command(
                &work_pool_task_id,
                &work_pool_title,
                "task",
                Some(&epic_task_id),
                &["work-pool-pack"],
                None,
            ),
            "close_command": build_task_close_command(
                &work_pool_task_id,
                "work-pool packet closed after delegated execution packet was shaped",
            )
        },
        "dev_task": {
            "required": false,
            "task_id": dev_task_id,
            "title": dev_title,
            "runtime": "vida taskflow",
            "inspect_command": build_task_show_command(&dev_task_id),
            "ensure_command": build_task_ensure_command(
                &dev_task_id,
                &dev_title,
                "task",
                Some(&epic_task_id),
                &["dev-pack"],
                None,
            ),
            "create_command": build_task_create_command(
                &dev_task_id,
                &dev_title,
                "task",
                Some(&epic_task_id),
                &["dev-pack"],
                None,
            ),
            "close_command": build_task_close_command(
                &dev_task_id,
                "delegated development packet reached proof-ready closure",
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

fn infer_runtime_task_class(
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
        || !contains_keywords(
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
        .is_empty()
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

fn infer_execution_runtime_role(
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
    runtime_role_for_task_class(task_class).to_string()
}

fn runtime_role_for_task_class(task_class: &str) -> &'static str {
    match task_class {
        "architecture" => "solution_architect",
        "verification" => "verifier",
        "coach" => "coach",
        "specification" => "business_analyst",
        _ => "worker",
    }
}

fn task_complexity_multiplier(task_class: &str) -> u64 {
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
    runtime_roles.is_empty() || runtime_roles.contains(&runtime_role)
}

fn role_supports_task_class(role: &serde_json::Value, task_class: &str) -> bool {
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    task_classes.is_empty() || task_classes.contains(&task_class)
}

fn dispatch_alias_row<'a>(
    compiled_bundle: &'a serde_json::Value,
    alias_id: &str,
) -> Option<&'a serde_json::Value> {
    carrier_runtime_section(compiled_bundle)["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["role_id"].as_str() == Some(alias_id))
}

fn build_runtime_assignment_from_dispatch_alias(
    compiled_bundle: &serde_json::Value,
    alias_id: &str,
    fallback_task_class: &str,
) -> serde_json::Value {
    let Some(alias) = dispatch_alias_row(compiled_bundle, alias_id) else {
        return serde_json::json!({
            "enabled": false,
            "reason": "dispatch_alias_missing",
            "dispatch_alias_id": alias_id,
            "task_class": fallback_task_class,
        });
    };
    let runtime_role = json_string(alias.get("default_runtime_role"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| runtime_role_for_task_class(fallback_task_class).to_string());
    let task_class = alias["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .find(|value| !value.is_empty())
        .unwrap_or(fallback_task_class)
        .to_string();
    let mut assignment = build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        alias_id,
        &task_class,
        &runtime_role,
    );
    if let Some(map) = assignment.as_object_mut() {
        map.insert(
            "dispatch_alias_id".to_string(),
            serde_json::Value::String(alias_id.to_string()),
        );
        map.insert(
            "dispatch_alias_runtime_role".to_string(),
            serde_json::Value::String(runtime_role),
        );
        map.insert(
            "dispatch_alias_task_class".to_string(),
            serde_json::Value::String(task_class),
        );
        map.insert(
            "dispatch_alias_description".to_string(),
            alias
                .get("description")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "preferred_carrier_tier".to_string(),
            alias
                .get("carrier_tier")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "developer_instructions".to_string(),
            alias
                .get("developer_instructions")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
    }
    assignment
}

fn resolve_dispatch_alias_id(
    compiled_bundle: &serde_json::Value,
    preferred_alias_id: &str,
    task_class: &str,
) -> Option<String> {
    if !preferred_alias_id.is_empty()
        && dispatch_alias_row(compiled_bundle, preferred_alias_id).is_some()
    {
        return Some(preferred_alias_id.to_string());
    }
    let runtime_role = runtime_role_for_task_class(task_class);
    carrier_runtime_section(compiled_bundle)["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|alias| {
            role_supports_runtime_role(alias, runtime_role)
                && role_supports_task_class(alias, task_class)
        })
        .and_then(|alias| alias["role_id"].as_str().map(str::to_string))
}

fn request_requires_execution_preparation(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
) -> bool {
    let selected_flow = compiled_bundle["default_flow_set"]
        .as_str()
        .and_then(|flow_id| compiled_bundle["all_project_flow_catalog"].get(flow_id));
    if let Some(policy) = selected_flow.and_then(|flow| flow.get("execution_preparation_policy")) {
        let mode = policy["mode"].as_str().unwrap_or_default();
        let gated_task_classes = policy["task_classes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        let task_class = runtime_assignment_from_execution_plan(&selection.execution_plan)
            ["task_class"]
            .as_str()
            .unwrap_or("implementation");
        let validation_gate = if json_bool(policy.get("honor_validation_gate"), false) {
            json_bool(
                compiled_bundle["autonomous_execution"]
                    .get("validation_report_required_before_implementation"),
                false,
            )
        } else {
            false
        };
        match mode {
            "always" => return true,
            "never" => return false,
            "required_for_task_classes" => {
                return gated_task_classes.contains(&task_class);
            }
            "required_for_code_shaped_work" => {
                if gated_task_classes.contains(&task_class) {
                    return validation_gate || task_class == "implementation";
                }
                return false;
            }
            _ => {}
        }
    }
    let normalized_request = selection.request.to_lowercase();
    let architecture_signals = contains_keywords(
        &normalized_request,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "migration".to_string(),
            "refactor".to_string(),
            "topology".to_string(),
            "boundary".to_string(),
            "cross-scope".to_string(),
            "cross scope".to_string(),
        ],
    );
    let write_signals = contains_keywords(
        &normalized_request,
        &[
            "implement".to_string(),
            "implementation".to_string(),
            "write code".to_string(),
            "write the code".to_string(),
            "patch".to_string(),
            "refactor".to_string(),
            "build".to_string(),
        ],
    );
    let task_class = json_string(
        compiled_bundle["role_selection"]
            .get("selected_task_class")
            .or_else(|| {
                runtime_assignment_from_execution_plan(&selection.execution_plan).get("task_class")
            }),
    )
    .unwrap_or_default();
    let validation_gate = json_bool(
        compiled_bundle["autonomous_execution"]
            .get("validation_report_required_before_implementation"),
        false,
    );
    task_class == "implementation"
        && (validation_gate || !architecture_signals.is_empty() || !write_signals.is_empty())
}

fn legacy_development_flow_templates() -> Vec<serde_json::Value> {
    let pending_specification_evidence =
        blocker_code_str(BlockerCode::PendingSpecificationEvidence);
    let pending_execution_preparation_evidence =
        blocker_code_str(BlockerCode::PendingExecutionPreparationEvidence);
    let pending_implementation_evidence =
        blocker_code_str(BlockerCode::PendingImplementationEvidence);
    let pending_review_clean_evidence = blocker_code_str(BlockerCode::PendingReviewCleanEvidence);
    let pending_verification_evidence = blocker_code_str(BlockerCode::PendingVerificationEvidence);
    vec![
        serde_json::json!({
            "lane_id": "specification",
            "dispatch_target": "specification",
            "dispatch_alias": "development_specification",
            "task_class": "specification",
            "packet_template_kind": "delivery_task_packet",
            "closure_class": "law",
            "stage": "design_gate",
            "inclusion_rule": "when_design_gate",
            "completion_blocker": pending_specification_evidence,
        }),
        serde_json::json!({
            "lane_id": "execution_preparation",
            "dispatch_target": "execution_preparation",
            "dispatch_alias": "development_execution_preparation",
            "task_class": "execution_preparation",
            "packet_template_kind": "escalation_packet",
            "closure_class": "refactor",
            "stage": "execution",
            "inclusion_rule": "when_execution_preparation_required",
            "completion_blocker": pending_execution_preparation_evidence,
        }),
        serde_json::json!({
            "lane_id": "implementation",
            "dispatch_target": "implementer",
            "dispatch_alias": "development_implementer",
            "task_class": "implementation",
            "packet_template_kind": "delivery_task_packet",
            "closure_class": "implementation",
            "stage": "execution",
            "inclusion_rule": "always",
            "completion_blocker": pending_implementation_evidence,
        }),
        serde_json::json!({
            "lane_id": "coach",
            "dispatch_target": "coach",
            "dispatch_alias": "development_coach",
            "task_class": "coach",
            "packet_template_kind": "coach_review_packet",
            "closure_class": "proof",
            "stage": "execution",
            "inclusion_rule": "when_flow_requires_coach",
            "completion_blocker": pending_review_clean_evidence,
        }),
        serde_json::json!({
            "lane_id": "verification",
            "dispatch_target": "verification",
            "dispatch_alias": "development_verifier",
            "task_class": "verification",
            "packet_template_kind": "verifier_proof_packet",
            "closure_class": "proof",
            "stage": "execution",
            "inclusion_rule": "when_flow_requires_verification",
            "completion_blocker": pending_verification_evidence,
        }),
    ]
}

fn resolved_development_flow_templates(
    compiled_bundle: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let flow_id = compiled_bundle["default_flow_set"]
        .as_str()
        .unwrap_or_default();
    if let Some(flow) = compiled_bundle["all_project_flow_catalog"]
        .get(flow_id)
        .or_else(|| compiled_bundle["project_flow_catalog"].get(flow_id))
    {
        if flow["flow_class"].as_str() == Some("development") {
            let templates = flow["lane_templates"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if !templates.is_empty() {
                return templates;
            }
        }
    }
    legacy_development_flow_templates()
}

fn lane_template_included(
    lane_template: &serde_json::Value,
    requires_design_gate: bool,
    requires_execution_preparation: bool,
) -> bool {
    match lane_template["inclusion_rule"].as_str().unwrap_or("always") {
        "when_design_gate" => requires_design_gate,
        "when_execution_preparation_required" => requires_execution_preparation,
        "when_flow_requires_coach" => true,
        "when_flow_requires_verification" => true,
        _ => true,
    }
}

fn build_resolved_development_dispatch_contract(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let flow_id = compiled_bundle["default_flow_set"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let requires_execution_preparation =
        request_requires_execution_preparation(compiled_bundle, selection);
    let resolved_lanes = resolved_development_flow_templates(compiled_bundle)
        .into_iter()
        .filter(|lane| {
            lane_template_included(lane, requires_design_gate, requires_execution_preparation)
        })
        .map(|lane_template| {
            let preferred_dispatch_alias = lane_template["dispatch_alias"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let task_class = lane_template["task_class"]
                .as_str()
                .unwrap_or("implementation");
            let dispatch_alias =
                resolve_dispatch_alias_id(compiled_bundle, &preferred_dispatch_alias, task_class)
                    .unwrap_or_default();
            let activation = if dispatch_alias.is_empty() {
                serde_json::json!({
                    "enabled": false,
                    "reason": "dispatch_alias_missing_from_lane_template",
                })
            } else {
                build_runtime_assignment_from_dispatch_alias(
                    compiled_bundle,
                    &dispatch_alias,
                    task_class,
                )
            };
            serde_json::json!({
                "lane_id": lane_template["lane_id"],
                "dispatch_target": lane_template["dispatch_target"],
                "dispatch_alias": dispatch_alias,
                "task_class": task_class,
                "runtime_role": activation["activation_runtime_role"],
                "packet_template_kind": lane_template["packet_template_kind"],
                "closure_class": lane_template["closure_class"],
                "stage": lane_template["stage"],
                "completion_blocker": lane_template["completion_blocker"],
                "activation": activation,
            })
        })
        .collect::<Vec<_>>();
    let lane_sequence = resolved_lanes
        .iter()
        .filter_map(|lane| lane["dispatch_target"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let execution_lane_sequence = resolved_lanes
        .iter()
        .filter(|lane| lane["stage"].as_str() != Some("design_gate"))
        .filter_map(|lane| lane["dispatch_target"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let lane_catalog = resolved_lanes
        .iter()
        .fold(serde_json::Map::new(), |mut acc, lane| {
            if let Some(dispatch_target) = lane["dispatch_target"].as_str() {
                acc.insert(dispatch_target.to_string(), lane.clone());
            }
            acc
        });
    serde_json::json!({
        "selected_flow_set": flow_id,
        "execution_preparation_required": requires_execution_preparation,
        "root_session_must_remain_orchestrator": true,
        "packet_family_required": [
            "delivery_task_packet",
            "execution_block_packet",
            "coach_review_packet",
            "verifier_proof_packet",
            "escalation_packet"
        ],
        "resolved_lanes": resolved_lanes,
        "lane_sequence": lane_sequence,
        "execution_lane_sequence": execution_lane_sequence,
        "lane_catalog": lane_catalog,
        "specification_activation": dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "specification"
        ).map(dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "implementer_activation": dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "implementer"
        ).map(dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "coach_activation": dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "coach"
        ).map(dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "verifier_activation": dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "verification"
        ).map(dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "escalation_activation": dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "execution_preparation"
        ).map(dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn canonical_lane_target_for_runtime_role(runtime_role: &str) -> Option<&'static str> {
    match runtime_role {
        "business_analyst" | "pm" => Some("specification"),
        "worker" => Some("implementer"),
        "coach" => Some("coach"),
        "verifier" | "prover" => Some("verification"),
        "solution_architect" => Some("execution_preparation"),
        _ => None,
    }
}

fn orchestration_lane_step_label(dispatch_target: &str) -> &'static str {
    match dispatch_target {
        "specification" => "delegate_specification_or_research_lane",
        "implementer" => "delegate_implementer_lane",
        "coach" => "delegate_coach_lane",
        "verification" => "delegate_verifier_lane",
        "execution_preparation" => "delegate_execution_preparation_lane",
        _ => "delegate_lane",
    }
}

fn orchestration_checkpoint_label(dispatch_target: &str) -> &'static str {
    match dispatch_target {
        "implementer" => "after_implementation_evidence",
        "coach" => "after_review_evidence",
        "verification" => "after_verification_evidence",
        "specification" => "after_design_gate",
        "execution_preparation" => "after_execution_preparation_evidence",
        _ => "after_lane_evidence",
    }
}

fn display_lane_label(dispatch_target: &str) -> String {
    match dispatch_target {
        "implementer" => "implementation".to_string(),
        "specification" => "specification".to_string(),
        "coach" => "coach".to_string(),
        "verification" => "verification".to_string(),
        "execution_preparation" => "execution_preparation".to_string(),
        _ => dispatch_target.to_string(),
    }
}

fn build_runtime_assignment_from_resolved_constraints(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
) -> serde_json::Value {
    let carrier_runtime = carrier_runtime_section(compiled_bundle);
    let Some(roles) = carrier_runtime["roles"].as_array() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing"
        });
    };
    if roles.is_empty() {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing"
        });
    }

    let demotion_score = json_u64(json_lookup(
        &carrier_runtime["worker_strategy"],
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
            let strategy = &carrier_runtime["worker_strategy"]["agents"][role_id];
            let effective_score =
                json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
            let lifecycle_state = strategy["lifecycle_state"].as_str().unwrap_or("probation");
            let supports_runtime_role = role_supports_runtime_role(role, execution_runtime_role);
            let supports_task_class = role_supports_task_class(role, task_class);
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
            "reason": "no_carrier_declares_runtime_role_and_task_class",
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
            "reason": "no_carrier_satisfies_runtime_role_or_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role
        });
    };

    let tier = selected_role["tier"].as_str().unwrap_or_default();
    let rate = selected_role["rate"].as_u64().unwrap_or(0);
    let complexity_multiplier = task_complexity_multiplier(task_class);
    let effective_score = json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
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
        "strategy_store": carrier_runtime["worker_strategy"]["store_path"],
        "scorecards_store": carrier_runtime["worker_strategy"]["scorecards_path"],
        "rationale": rationale
    })
}

fn build_runtime_assignment(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let task_class = infer_runtime_task_class(selection, requires_design_gate);
    let execution_runtime_role =
        infer_execution_runtime_role(selection, &task_class, requires_design_gate);
    build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        &selection.selected_role,
        &task_class,
        &execution_runtime_role,
    )
}

fn execution_plan_agent_only_development_required(execution_plan: &serde_json::Value) -> bool {
    json_bool(
        execution_plan["autonomous_execution"].get("agent_only_development"),
        false,
    )
}

fn build_runtime_orchestration_contract(
    requires_design_gate: bool,
    agent_only_development: bool,
    dispatch_contract: &serde_json::Value,
) -> serde_json::Value {
    let execution_lane_sequence = dispatch_contract["execution_lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let active_cycle = if requires_design_gate {
        let mut cycle = vec![
            "publish_initial_execution_plan".to_string(),
            "delegate_specification_or_research_lane".to_string(),
            "replan_after_design_gate".to_string(),
            "shape_work_pool_and_dev_packets".to_string(),
        ];
        cycle.extend(
            execution_lane_sequence
                .iter()
                .map(|lane| orchestration_lane_step_label(lane).to_string()),
        );
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    } else {
        let mut cycle = vec!["publish_initial_execution_plan".to_string()];
        cycle.extend(
            execution_lane_sequence
                .iter()
                .map(|lane| orchestration_lane_step_label(lane).to_string()),
        );
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    };
    let replanning_checkpoints = if requires_design_gate {
        let mut checkpoints = vec![
            "after_design_gate".to_string(),
            "after_work_pool_shape".to_string(),
            "after_dev_packet_shape".to_string(),
        ];
        checkpoints.extend(
            execution_lane_sequence
                .iter()
                .map(|lane| orchestration_checkpoint_label(lane).to_string()),
        );
        serde_json::json!(checkpoints)
    } else {
        let mut checkpoints = vec!["after_packet_shape".to_string()];
        checkpoints.extend(
            execution_lane_sequence
                .iter()
                .map(|lane| orchestration_checkpoint_label(lane).to_string()),
        );
        serde_json::json!(checkpoints)
    };

    serde_json::json!({
        "mode": "delegated_orchestration_cycle",
        "root_session_role": "orchestrator",
        "root_session_must_remain_orchestrator": true,
        "root_session_write_guard": build_root_session_write_guard(),
        "initial_response": {
            "plan_required_before_substantive_execution": true,
            "plan_scope": "one bounded active cycle",
            "must_happen_before": [
                "design_doc_mutation",
                "packet_dispatch",
                "implementation_work"
            ],
            "minimum_fields": [
                "active_bounded_unit",
                "next_steps",
                "delegation_targets",
                "proof_target"
            ],
            "operator_message": "publish a concise execution plan before mutating docs, dispatching work, or entering implementation"
        },
        "delegation_policy": {
            "normal_write_producing_work": "delegated_by_default",
            "agent_only_development_required": agent_only_development,
            "canonical_project_delegated_execution_surface": "vida agent-init",
            "host_subagent_apis_are_backend_details": true,
            "host_local_write_capability_is_not_authority": true,
            "generic_single_worker_dispatch_forbidden": true,
            "local_implementation_without_exception_path_forbidden": true,
            "required_lanes": dispatch_contract["lane_sequence"]
        },
        "replanning": {
            "required": true,
            "checkpoints": replanning_checkpoints,
            "trigger_rule": "replan after each bounded gate or delegated evidence return before the next write-producing step"
        },
        "active_cycle": active_cycle
    })
}

fn build_root_session_write_guard() -> serde_json::Value {
    serde_json::json!({
        "status": "blocked_by_default",
        "root_session_role": "orchestrator",
        "local_write_requires_exception_path": true,
        "lawful_write_surface": "vida agent-init",
        "host_local_write_capability_is_not_authority": true,
        "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
        "pre_write_checkpoint_required": true,
    })
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
    let agent_only_development = json_bool(
        compiled_bundle["autonomous_execution"].get("agent_only_development"),
        false,
    );
    let dispatch_contract = build_resolved_development_dispatch_contract(
        compiled_bundle,
        selection,
        requires_design_gate,
    );
    let orchestration_contract = build_runtime_orchestration_contract(
        requires_design_gate,
        agent_only_development,
        &dispatch_contract,
    );
    let runtime_assignment =
        build_runtime_assignment(compiled_bundle, selection, requires_design_gate);
    let lane_sequence = dispatch_contract["lane_sequence"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut execution_plan = serde_json::json!({
        "status": if requires_design_gate {
            "design_first"
        } else {
            "ready_for_runtime_routing"
        },
        "system_mode": json_string(json_lookup(agent_system, &["mode"])).unwrap_or_default(),
        "state_owner": json_string(json_lookup(agent_system, &["state_owner"])).unwrap_or_default(),
        "max_parallel_agents": json_lookup(agent_system, &["max_parallel_agents"]).cloned().unwrap_or(serde_json::Value::Null),
        "autonomous_execution": {
            "agent_only_development": agent_only_development,
        },
        "orchestration_contract": orchestration_contract,
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
            "developer_handoff_packet_required": requires_design_gate,
            "developer_handoff_packet_status": if requires_design_gate {
                "blocked_pending_developer_handoff_packet"
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
        "development_flow": {
            "activation_status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "eligible_after_runtime_routing"
            },
            "lane_sequence": lane_sequence,
            "generic_single_worker_dispatch_forbidden": true,
            "dispatch_contract": dispatch_contract,
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
    });
    if let Some(plan) = execution_plan.as_object_mut() {
        plan.insert(
            "root_session_write_guard".to_string(),
            build_root_session_write_guard(),
        );
        plan.extend(runtime_assignment_alias_fields(&runtime_assignment));
    }
    execution_plan
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
    let dispatch_aliases_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "dispatch_aliases"],
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
        Some(path) => match project_activator_surface::load_registry_projection(
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
        Some(path) => match project_activator_surface::load_registry_projection(
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
        Some(path) => match project_activator_surface::load_registry_projection(
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
        Some(path) => match project_activator_surface::load_registry_projection(
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
    let dispatch_aliases_registry = match dispatch_aliases_path.as_deref() {
        Some(path) => match project_activator_surface::load_registry_projection(
            root,
            Some(path),
            "dispatch_aliases",
            "alias_id",
            "dispatch_aliases",
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
    let overlay_codex_roles = project_activator_surface::overlay_codex_agent_catalog(config);
    let codex_roles = if overlay_codex_roles.is_empty() {
        project_activator_surface::read_codex_agent_catalog(&codex_root)
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
    let dispatch_alias_rows = registry_rows_by_key(
        &dispatch_aliases_registry,
        "dispatch_aliases",
        "alias_id",
        &[],
    );
    let codex_dispatch_aliases = if dispatch_alias_rows.is_empty() {
        project_activator_surface::overlay_codex_dispatch_alias_catalog(config, &codex_roles)
    } else {
        project_activator_surface::materialize_codex_dispatch_alias_catalog(
            &dispatch_alias_rows,
            &codex_roles,
        )
    };
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
            "store_path": WORKER_STRATEGY_STATE,
            "scorecards_path": WORKER_SCORECARDS_STATE,
            "agents": {}
        })
    } else {
        refresh_worker_strategy(root, &codex_roles, &scoring_policy)
    };
    let pricing_policy = build_carrier_pricing_policy(&codex_roles, &worker_strategy);
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
    let all_project_flows = registry_rows_by_key(&flows_registry, "flow_sets", "flow_id", &[]);
    let project_role_map = registry_row_map_by_id(&project_roles, "role_id");
    let project_skill_map = registry_row_map_by_id(&project_skills, "skill_id");
    let project_profile_map = registry_row_map_by_id(&project_profiles, "profile_id");
    let project_flow_map = registry_row_map_by_id(&project_flows, "flow_id");
    let all_project_flow_map = registry_row_map_by_id(&all_project_flows, "flow_id");
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

    let carrier_runtime = serde_json::json!({
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
    });

    let bundle = serde_json::json!({
        "ok": true,
        "enabled": yaml_bool(yaml_lookup(config, &["agent_extensions", "enabled"]), false),
        "map_doc": yaml_string(yaml_lookup(config, &["agent_extensions", "map_doc"])).unwrap_or_default(),
        "enabled_framework_roles": enabled_framework_roles,
        "enabled_standard_flow_sets": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_standard_flow_sets"])),
        "enabled_shared_skills": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_shared_skills"])),
        "default_flow_set": yaml_string(yaml_lookup(config, &["agent_extensions", "default_flow_set"])).unwrap_or_default(),
        "runtime_projection_root": project_activator_surface::runtime_agent_extensions_root(root).display().to_string(),
        "project_roles": project_roles,
        "project_skills": project_skills,
        "project_profiles": project_profiles,
        "project_flows": project_flows,
        "project_role_catalog": project_role_map,
        "project_profile_catalog": project_profile_map,
        "project_flow_catalog": project_flow_map,
        "all_project_flow_catalog": all_project_flow_map,
        "agent_system": serde_json::to_value(yaml_lookup(config, &["agent_system"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "autonomous_execution": serde_json::to_value(yaml_lookup(config, &["autonomous_execution"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "carrier_runtime": carrier_runtime,
        "role_selection": serde_json::to_value(yaml_lookup(config, &["agent_extensions", "role_selection"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
    });

    let role_ids = registry_ids_by_key(&roles_registry, "roles", "role_id");
    let skill_ids = registry_ids_by_key(&skills_registry, "skills", "skill_id");
    let profile_ids = registry_ids_by_key(&profiles_registry, "profiles", "profile_id");
    let flow_ids = registry_ids_by_key(&flows_registry, "flow_sets", "flow_id");

    let missing_roles =
        project_activator_surface::collect_missing_registry_ids(&role_ids, &enabled_project_roles);
    if !missing_roles.is_empty() {
        validation_errors.push(format!(
            "agent extension roles registry is missing enabled role ids: {}",
            missing_roles.join(", ")
        ));
    }
    let missing_skills = project_activator_surface::collect_missing_registry_ids(
        &skill_ids,
        &enabled_project_skills,
    );
    if !missing_skills.is_empty() {
        validation_errors.push(format!(
            "agent extension skills registry is missing enabled skill ids: {}",
            missing_skills.join(", ")
        ));
    }
    if require_profile_resolution {
        let missing_profiles = project_activator_surface::collect_missing_registry_ids(
            &profile_ids,
            &enabled_project_profiles,
        );
        if !missing_profiles.is_empty() {
            validation_errors.push(format!(
                "agent extension profiles registry is missing enabled profile ids: {}",
                missing_profiles.join(", ")
            ));
        }
    }
    if require_flow_resolution {
        let missing_flows = project_activator_surface::collect_missing_registry_ids(
            &flow_ids,
            &enabled_project_flows,
        );
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
    let registry_root = std::env::current_dir()
        .ok()
        .filter(|cwd| looks_like_project_root(cwd))
        .or_else(|| resolve_repo_root().ok())
        .expect("docflow registry evidence should resolve the repo root");
    let registry_root = registry_root.display().to_string();
    let registry_root_path = std::path::PathBuf::from(&registry_root);
    let registry_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "registry".to_string(),
            "--root".to_string(),
            registry_root.clone(),
        ],
    )
    .expect("docflow registry evidence should render");
    let check_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "check".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow check evidence should render");
    let readiness_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "readiness-check".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow readiness evidence should render");
    let proof_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "proofcheck".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow proof evidence should render");

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
        verdict: Some(if proof_ok {
            "ready".to_string()
        } else {
            "blocked".to_string()
        }),
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
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingDocflowActivation,
        ) {
            blockers.push(code);
        }
    }
    if !check.ok {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DocflowCheckBlocking,
        ) {
            blockers.push(code);
        }
    }
    if !readiness.ok {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingReadinessVerdict,
        ) {
            blockers.push(code);
        }
    }
    if !matches!(readiness.verdict.as_deref(), Some("ready" | "blocked")) {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingReadinessVerdict,
        ) {
            blockers.push(code);
        }
    }
    if readiness
        .artifact_path
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingInventoryOrProjectionEvidence,
        ) {
            blockers.push(code);
        }
    }
    if !proof.ok {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingProofVerdict,
        ) {
            blockers.push(code);
        }
    }
    if !matches!(proof.verdict.as_deref(), Some("ready" | "blocked")) {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::MissingProofVerdict,
        ) {
            blockers.push(code);
        }
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
                "verdict": "blocked",
                "output": error
            }
        }),
    }
}

fn emit_taskflow_consume_final_json(
    store: &StateStore,
    payload: &TaskflowDirectConsumptionPayload,
) -> Result<(), String> {
    let mut payload_json = serde_json::to_value(payload)
        .map_err(|error| format!("Failed to encode consume-final payload as json: {error}"))?;
    let runtime_dispatch_receipt_blocker_code =
        runtime_consumption_final_dispatch_receipt_blocker_code(store, &payload_json)?;
    let mut consume_final_blocker_codes = consume_final_operator_blocker_codes(&payload_json);
    let mut consume_final_next_actions = consume_final_operator_next_actions(&payload_json);
    if let Some(blocker_code) = runtime_dispatch_receipt_blocker_code.as_deref() {
        apply_runtime_consumption_final_dispatch_receipt_blocker(&mut payload_json, blocker_code);
        if !consume_final_blocker_codes
            .iter()
            .any(|code| code == blocker_code)
        {
            consume_final_blocker_codes.push(blocker_code.to_string());
        }
        consume_final_next_actions.push(
            match blocker_code {
                RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER => {
                    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION
                }
                _ => RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
            }
            .to_string(),
        );
    }
    consume_final_blocker_codes = crate::release1_contracts::canonical_blocker_code_list(
        consume_final_blocker_codes.iter().map(String::as_str),
    );
    let consume_final_status = if consume_final_blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let snapshot = serde_json::json!({
        "surface": "vida taskflow consume final",
        "payload": payload_json,
    });
    let snapshot_path = write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let operator_contracts = build_release1_operator_contracts_envelope(
        consume_final_status,
        consume_final_blocker_codes.clone(),
        consume_final_next_actions.clone(),
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": payload_json["dispatch_receipt"]["run_id"].as_str(),
            "latest_task_reconciliation_receipt_id": payload_json["task_reconciliation_receipt"]["receipt_id"].as_str(),
            "retrieval_trust_signal": payload_json["runtime_bundle"]["cache_delivery_contract"]["retrieval_trust_evidence"].clone(),
            "consume_final_surface": "vida taskflow consume final",
        }),
    );
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": operator_contracts["status"].clone(),
        "blocker_codes": operator_contracts["blocker_codes"].clone(),
        "next_actions": operator_contracts["next_actions"].clone(),
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
    });
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": "vida taskflow consume final",
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": consume_final_status,
        "blocker_codes": consume_final_blocker_codes,
        "next_actions": consume_final_next_actions,
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
        "payload": payload_json,
    });
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "surface": "vida taskflow consume final",
            "trace_id": operator_contracts["trace_id"].clone(),
            "workflow_class": operator_contracts["workflow_class"].clone(),
            "risk_tier": operator_contracts["risk_tier"].clone(),
            "status": consume_final_status,
            "blocker_codes": consume_final_blocker_codes,
            "next_actions": consume_final_next_actions,
            "artifact_refs": operator_contracts["artifact_refs"].clone(),
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts,
            "payload": payload_json,
            "snapshot_path": snapshot_path,
        }))
        .expect("consume final should render as json")
    );
    Ok(())
}

pub(crate) fn build_release1_operator_contracts_envelope(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
) -> serde_json::Value {
    let canonical_status =
        crate::release1_contracts::canonical_release1_contract_status_str(status)
            .unwrap_or(crate::release1_contracts::Release1ContractStatus::Blocked.as_str());
    serde_json::json!({
        "contract_id": "release-1-operator-contracts",
        "schema_version": "release-1-v1",
        "status": canonical_status,
        "trace_id": serde_json::Value::Null,
        "workflow_class": serde_json::Value::Null,
        "risk_tier": serde_json::Value::Null,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
    })
}

fn release1_status_from_value(value: &serde_json::Value) -> Option<&'static str> {
    value
        .as_str()
        .and_then(canonical_release1_contract_status_str)
}

fn release1_status_is_blocked(value: &serde_json::Value) -> bool {
    release1_status_from_value(value) == Some("blocked")
}

fn consume_final_operator_blocker_codes(payload: &serde_json::Value) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if payload["bundle_check"]["activation_status"].as_str() != Some("ready_enough_for_normal_work")
    {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::BundleActivationNotReady,
        ) {
            blocker_codes.push(code);
        }
    }
    if release1_status_is_blocked(&payload["docflow_verdict"]["status"]) {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DocflowVerdictBlock,
        ) {
            blocker_codes.push(code);
        }
    }
    if release1_status_is_blocked(&payload["closure_admission"]["status"]) {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::ClosureAdmissionBlock,
        ) {
            blocker_codes.push(code);
        }
    }
    blocker_codes
}

fn consume_final_operator_next_actions(payload: &serde_json::Value) -> Vec<String> {
    let mut next_actions = Vec::new();
    if payload["bundle_check"]["activation_status"].as_str() != Some("ready_enough_for_normal_work")
    {
        next_actions.push("Resolve activation blockers before consume-final handoff.".to_string());
    }
    if release1_status_is_blocked(&payload["docflow_verdict"]["status"]) {
        next_actions.push(
            "Run `vida docflow proofcheck --profile active-canon` and clear blockers.".to_string(),
        );
    }
    if release1_status_is_blocked(&payload["closure_admission"]["status"]) {
        next_actions.push(
            "Run `vida taskflow consume bundle check --json` and resolve closure blockers."
                .to_string(),
        );
    }
    next_actions
}

fn normalize_root_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release1_contracts::canonical_lane_status_str;
    use crate::temp_state::TempStateHarness;
    use clap::{CommandFactory, Parser};
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

    fn guard_current_dir(path: &Path) -> CurrentDirGuard {
        let guard = {
            let _lock = current_dir_lock().lock().expect("lock should succeed");
            CurrentDirGuard::change_to(path)
        };
        guard
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
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn install_external_cli_test_subagents(config_path: &Path) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let updated = config.replace(
            "agent_system:\n  init_on_boot: true\n  mode: native\n  state_owner: orchestrator_only\n  max_parallel_agents: 4\n  routing: {}\n  scoring: {}\n",
            concat!(
                "agent_system:\n",
                "  init_on_boot: true\n",
                "  mode: native\n",
                "  state_owner: orchestrator_only\n",
                "  max_parallel_agents: 4\n",
                "  subagents:\n",
                "    qwen_cli:\n",
                "      enabled: true\n",
                "      subagent_backend_class: external_cli\n",
                "      detect_command: qwen\n",
                "      dispatch:\n",
                "        command: qwen\n",
                "        static_args:\n",
                "          - -y\n",
                "          - -o\n",
                "          - text\n",
                "        prompt_mode: positional\n",
                "    kilo_cli:\n",
                "      enabled: true\n",
                "      subagent_backend_class: external_cli\n",
                "      detect_command: kilo\n",
                "      dispatch:\n",
                "        command: kilo\n",
                "        static_args:\n",
                "          - run\n",
                "          - --auto\n",
                "        workdir_flag: --dir\n",
                "        prompt_mode: positional\n",
                "  routing: {}\n",
                "  scoring: {}\n",
            ),
        );
        assert_ne!(
            updated, config,
            "expected agent_system scaffold replacement"
        );
        fs::write(config_path, updated).expect("config should update");
    }

    #[test]
    fn latest_final_runtime_consumption_snapshot_path_prefers_newest_valid_final_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "vida-valid-final-snapshot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let valid_path = runtime_dir.join("final-valid.json");
        fs::write(
            &valid_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": []
                },
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {
                        "retrieval_trust_signal": {
                            "source": "runtime_consumption_snapshot_index",
                            "citation": "runtime-consumption/final-valid.json",
                            "freshness": "final",
                            "acl": "protocol-binding-receipt-id"
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("valid final snapshot should be writable");

        thread::sleep(Duration::from_millis(5));

        let invalid_path = runtime_dir.join("final-incomplete.json");
        fs::write(
            &invalid_path,
            serde_json::json!({
                "surface": "vida taskflow consume continue",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": []
                },
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                }
            })
            .to_string(),
        )
        .expect("incomplete final snapshot should be writable");

        let selected = latest_final_runtime_consumption_snapshot_path(&root)
            .expect("latest valid final snapshot should resolve")
            .expect("one valid final snapshot should be available");
        assert_eq!(selected, valid_path.display().to_string());

        let _ = fs::remove_dir_all(root);
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
    fn canonical_lane_status_str_trims_whitespace_for_release1_lane_status() {
        assert_eq!(
            canonical_lane_status_str("  lane_running  "),
            Some("lane_running")
        );
        assert_eq!(canonical_lane_status_str("lane_block"), None);
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
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
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
        assert!(harness.path().join("README.md").is_file());
        assert!(harness.path().join(DEFAULT_PROJECT_ROOT_MAP).is_file());
        assert!(harness.path().join(DEFAULT_PROJECT_PRODUCT_INDEX).is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_PRODUCT_SPEC_README)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_PROCESS_README)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_RESEARCH_README)
            .is_file());
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
    fn task_help_lists_mutation_commands() {
        let mut command = Cli::command();
        let task = command
            .find_subcommand_mut("task")
            .expect("task subcommand should exist");
        let help = task.render_long_help().to_string();
        assert!(help.contains("create"), "task help should list create");
        assert!(help.contains("update"), "task help should list update");
        assert!(help.contains("close"), "task help should list close");
        assert!(
            help.contains("next-display-id"),
            "task help should list next-display-id"
        );
        assert!(
            help.contains("export-jsonl"),
            "task help should list export-jsonl"
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_bootstrap_aliases() {
        let (target, path) = crate::protocol_surface::resolve_protocol_view_target("AGENTS")
            .expect("AGENTS alias should resolve");
        assert_eq!(target.canonical_id, "bootstrap/router");
        assert!(
            path.ends_with("vida/config/instructions/system-maps/bootstrap.router-guide.md"),
            "bootstrap router guide path should resolve"
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_worker_entry_name() {
        let (target, path) = crate::protocol_surface::resolve_protocol_view_target(
            "agent-definitions/entry.worker-entry",
        )
        .expect("worker entry should resolve");
        assert_eq!(target.canonical_id, "agent-definitions/entry.worker-entry");
        assert!(
            path.ends_with("vida/config/instructions/agent-definitions/entry.worker-entry.md"),
            "worker entry path should resolve"
        );
    }

    #[test]
    fn resolve_protocol_view_target_supports_generic_canonical_ids_without_md() {
        let (target, path) = crate::protocol_surface::resolve_protocol_view_target(
            "instruction-contracts/core.orchestration-protocol",
        )
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
        let (target, path) = crate::protocol_surface::resolve_protocol_view_target(
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
        let section =
            crate::protocol_surface::extract_protocol_view_fragment(content, "section-web-search")
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
    fn init_replaces_agents_template_and_keeps_existing_sidecar_with_backup() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        fs::write(
            harness.path().join("AGENTS.md"),
            "project-specific bootstrap notes\n",
        )
        .expect("existing agents should be written");
        fs::write(
            harness.path().join("AGENTS.sidecar.md"),
            "current sidecar content\n",
        )
        .expect("existing sidecar should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );

        let sidecar = fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
            .expect("sidecar should still exist");
        assert_eq!(sidecar, "current sidecar content\n");

        let backup = fs::read_to_string(
            harness
                .path()
                .join(".vida/receipts/AGENTS.pre-init.backup.md"),
        )
        .expect("agents backup should be written");
        assert_eq!(backup, "project-specific bootstrap notes\n");
    }

    #[test]
    fn project_activator_reports_pending_activation_for_partial_project() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        fs::write(harness.path().join("README.md"), "# demo\n").expect("readme should exist");

        let view = project_activator_surface::build_project_activator_view(harness.path());

        assert_eq!(view["status"], "pending");
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
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
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
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let view = project_activator_surface::build_project_activator_view(root);

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

        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(view["status"], "pending");
        assert_eq!(view["activation_pending"], true);
        assert_eq!(view["triggers"]["sidecar_or_project_docs_too_thin"], false);
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
    fn project_activator_fails_closed_for_partial_activation_submission() {
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
            ExitCode::from(2)
        );

        assert!(!harness.path().join(".codex/config.toml").exists());
        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(view["activation_pending"], true);
        assert!(view["host_environment"]["selected_cli_system"].is_null());
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

        assert!(harness.path().join(".codex/config.toml").is_file());
        assert!(harness.path().join(".codex/agents").is_dir());
        assert!(harness.path().join(WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(WORKER_STRATEGY_STATE).is_file());
        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("cli_system: codex"));
        assert!(config.contains("host_environment:"));
        assert!(config.contains("protocol_activation:\n  agent_system: true"));
        assert!(config.contains("agent_only_development: true"));
        assert!(config.contains("agent_system:\n  init_on_boot: true"));
        assert!(config.contains("mode: native"));
        assert!(config.contains("state_owner: orchestrator_only"));
        assert!(config.contains("max_parallel_agents: 4"));

        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(view["host_environment"]["selected_cli_system"], "codex");
        assert_eq!(
            view["host_environment"]["selected_cli_execution_class"],
            "internal"
        );
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".codex");
        assert_eq!(
            view["normal_work_defaults"]["local_host_agent_guide"],
            DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC
        );
        assert!(view["normal_work_defaults"]
            .get("local_codex_guide")
            .is_none());
        assert!(view["normal_work_defaults"]
            .get("codex_tier_rates")
            .is_none());
    }

    #[test]
    fn project_activator_accepts_host_cli_selection_and_materializes_copy_tree_template() {
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
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        assert!(harness.path().join(".qwen").is_dir());
        let config = fs::read_to_string(harness.path().join("vida.config.yaml"))
            .expect("config should exist");
        assert!(config.contains("cli_system: qwen"));

        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(view["host_environment"]["selected_cli_system"], "qwen");
        assert_eq!(
            view["host_environment"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".qwen");
        assert_eq!(
            view["normal_work_defaults"]["default_agent_topology"],
            serde_json::json!(["qwen-primary"])
        );
        assert_eq!(
            view["normal_work_defaults"]["carrier_tier_rates"]["qwen"],
            4
        );
        assert!(view["normal_work_defaults"]
            .get("local_codex_guide")
            .is_none());
        assert!(view["normal_work_defaults"]
            .get("codex_tier_rates")
            .is_none());
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("codex")));
    }

    #[test]
    fn runtime_host_execution_contract_reflects_external_qwen_selection() {
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
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let contract = runtime_host_execution_contract_for_root(harness.path());
        assert_eq!(contract["selected_cli_system"], "qwen");
        assert_eq!(contract["selected_cli_execution_class"], "external");
        assert_eq!(contract["runtime_template_root"], ".qwen");
        assert_eq!(contract["template_materialized"], true);
    }

    #[test]
    fn project_activator_view_uses_builtin_host_registry_without_overlay_systems() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(
            view["host_environment"]["selected_cli_system"],
            serde_json::Value::Null
        );
        assert_eq!(view["host_environment"]["selection_required"], true);
        assert_eq!(view["host_environment"]["template_materialized"], false);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".codex");
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("codex")));
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
        assert!(view["host_environment"]["template_source_root"]
            .as_str()
            .expect("template source root should render")
            .ends_with("/.codex"));
    }

    #[test]
    fn project_activator_materializes_builtin_copy_tree_template_without_overlay_entry() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let source = project_activator_surface::resolve_host_cli_template_source("qwen", None)
            .expect("builtin qwen template source should resolve");
        assert!(source.ends_with(".qwen"));

        let runtime_root =
            project_activator_surface::materialize_host_cli_template(harness.path(), "qwen", None)
                .expect("builtin qwen template should materialize");
        assert!(runtime_root.ends_with(".qwen"));
        assert!(harness.path().join(".qwen").is_dir());
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
        assert!(harness.path().join(WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(WORKER_STRATEGY_STATE).is_file());
        assert!(
            harness
                .path()
                .join(".vida/receipts/project-activation.latest.json")
                .is_file(),
            "activation receipt should be written"
        );

        let view = project_activator_surface::build_project_activator_view(harness.path());
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

        let view = project_activator_surface::build_project_activator_view(harness.path());
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["agent_identity"],
            "execution_carrier"
        );
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["runtime_role_identity"],
            "activation_state"
        );
        assert_eq!(
            view["normal_work_defaults"]["execution_carrier_model"]["selection_rule"],
            "capability_first_then_score_guard_then_cheapest_tier"
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]["snapshot"]
                .as_str()
                .unwrap_or_default()
                .contains("vida taskflow consume agent-system --json")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["carrier_catalog"]
                .as_str()
                .unwrap_or_default()
                .contains(".snapshot.carriers")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["runtime_roles"]
                .as_str()
                .unwrap_or_default()
                .contains("roles")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]["scores"]
                .as_str()
                .unwrap_or_default()
                .contains(".snapshot.worker_strategy.agents")
        );
        assert!(
            view["normal_work_defaults"]["execution_carrier_model"]["inspect_commands"]
                ["selection_preview"]
                .as_str()
                .unwrap_or_default()
                .contains(".payload.taskflow_handoff_plan.runtime_assignment")
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
        assert!(
            junior.contains("vida_task_classes = \"implementation,delivery_task,execution_block\"")
        );

        let middle = fs::read_to_string(harness.path().join(".codex/agents/middle.toml"))
            .expect("middle agent should exist");
        assert!(middle.contains("vida_rate = \"4\""));
        assert!(middle.contains("vida_runtime_roles = \"business_analyst,pm,coach,worker\""));
        assert!(middle.contains(
            "vida_task_classes = \"specification,planning,coach,implementation_medium\""
        ));

        let senior = fs::read_to_string(harness.path().join(".codex/agents/senior.toml"))
            .expect("senior agent should exist");
        assert!(senior.contains("vida_rate = \"16\""));
        assert!(senior.contains("vida_runtime_roles = \"verifier,prover\""));
        assert!(senior.contains(
            "vida_task_classes = \"verification,review,quality_gate,release_readiness\""
        ));

        let architect = fs::read_to_string(harness.path().join(".codex/agents/architect.toml"))
            .expect("architect agent should exist");
        assert!(architect.contains("vida_rate = \"32\""));
        assert!(architect.contains("vida_reasoning_band = \"xhigh\""));
        assert!(architect.contains(
            "vida_task_classes = \"architecture,execution_preparation,hard_escalation,meta_analysis\""
        ));

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
    fn runtime_assignment_uses_overlay_ladder_for_all_four_tiers() {
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

        let config = project_activator_surface::read_yaml_file_checked(
            &harness.path().join("vida.config.yaml"),
        )
        .expect("config");
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
            let plan = build_runtime_execution_plan_from_snapshot(&bundle, &selection);
            let carrier_runtime_assignment = plan["carrier_runtime_assignment"].clone();
            let runtime_assignment = plan["runtime_assignment"].clone();
            assert_eq!(carrier_runtime_assignment, runtime_assignment);
            assert!(plan.get("codex_runtime_assignment").is_none());
            runtime_assignment
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
    fn selected_backend_prefers_carrier_tier_over_internal_subagents() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "preferred_agent_tier": "junior",
                    "preferred_agent_type": "junior",
                    "subagents": "internal_subagents",
                    "runtime_assignment": {
                        "selected_tier": "junior",
                        "activation_agent_type": "junior",
                    }
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("junior")
        );
    }

    #[test]
    fn selected_backend_accepts_legacy_codex_runtime_assignment_alias() {
        let execution_plan = serde_json::json!({
            "codex_runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "subagents": "internal_subagents"
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("middle")
        );
    }

    #[test]
    fn selected_backend_prefers_carrier_backend_hint_over_legacy_subagents() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "carrier_backend_hint": "neutral_hint",
                    "subagents": "internal_subagents"
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("neutral_hint")
        );
    }

    #[test]
    fn fallback_run_graph_status_uses_carrier_tier_for_conversation_routes() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "status": "design_first",
                "codex_runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                },
                "default_route": {
                    "subagents": "internal_subagents"
                },
                "development_flow": {
                    "implementation": {
                        "preferred_agent_tier": "junior",
                        "subagents": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let status = fallback_runtime_consumption_run_graph_status(&role_selection, "run-test");
        assert_eq!(status.selected_backend, "middle");
    }

    #[tokio::test]
    async fn runtime_consumption_bootstrap_fails_closed_with_blocked_fallback_when_seed_derivation_fails(
    ) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-consumption-seed-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));
        let cwd = std::env::temp_dir().join(format!(
            "vida-runtime-consumption-seed-fail-closed-cwd-{}-{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&cwd).expect("create isolated cwd");
        let _cwd = guard_current_dir(&cwd);
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let bootstrap =
            build_runtime_consumption_run_graph_bootstrap(&store, &role_selection).await;
        assert_eq!(bootstrap["status"], "blocked");
        assert_eq!(bootstrap["handoff_ready"], false);
        assert!(bootstrap["fallback_reason"]
            .as_str()
            .is_some_and(|value| value.contains("seed_failed")));

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.status, "blocked");
        assert!(!latest_status.recovery_ready);
        assert_eq!(latest_status.context_state, "open");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn downstream_receipt_backend_prefers_activation_agent_type() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let root_receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let downstream = build_downstream_dispatch_receipt(&role_selection, &root_receipt)
            .expect("downstream receipt should build");
        assert_eq!(downstream.activation_agent_type.as_deref(), Some("junior"));
        assert_eq!(downstream.selected_backend.as_deref(), Some("junior"));
    }

    #[test]
    fn spec_pack_downstream_routes_to_specification_lane_when_agent_only_enabled() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let (target, command, _note, ready, blockers) =
            derive_downstream_dispatch_preview(&role_selection, &receipt);
        assert_eq!(target.as_deref(), Some("specification"));
        assert_eq!(command.as_deref(), Some("vida agent-init"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn packet_ready_specification_lane_stays_active_while_work_pool_handoff_remains_blocked() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let (target, command, note, ready, blockers) =
            derive_downstream_dispatch_preview(&role_selection, &receipt);
        assert_eq!(target.as_deref(), Some("work-pool-pack"));
        assert_eq!(
            command.as_deref(),
            Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
            )
        );
        assert!(!ready);
        assert!(blockers.contains(&"pending_specification_evidence".to_string()));
        assert!(blockers.contains(&"pending_design_finalize".to_string()));
        assert!(blockers.contains(&"pending_spec_task_close".to_string()));
        assert_eq!(
            active_downstream_dispatch_target(&receipt).as_deref(),
            Some("specification")
        );
        assert!(note
            .as_deref()
            .unwrap_or_default()
            .contains("wait for bounded evidence return"));
    }

    #[test]
    fn specification_downstream_activation_uses_specification_contract() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let (_kind, surface, agent_type, runtime_role) =
            downstream_activation_fields(&role_selection, "specification");
        assert_eq!(surface.as_deref(), Some("vida agent-init"));
        assert_eq!(agent_type.as_deref(), Some("middle"));
        assert_eq!(runtime_role.as_deref(), Some("business_analyst"));
    }

    #[test]
    fn executed_worker_lane_sets_downstream_ready_without_evidence_blocker() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-dev".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("dispatch-result.json".to_string()),
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let (target, _command, _note, ready, blockers) =
            derive_downstream_dispatch_preview(&role_selection, &receipt);
        assert_eq!(target.as_deref(), Some("coach"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn refresh_downstream_dispatch_preview_unblocks_dev_handoff_after_work_pool_execution() {
        let root = std::env::temp_dir().join(format!(
            "vida-refresh-downstream-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("work-pool-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-work-pool",
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-work-pool".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("dev-pack".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_work_pool_shape".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        refresh_downstream_dispatch_preview(
            &root,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .expect("refresh should succeed");

        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(
            receipt.downstream_dispatch_command.as_deref(),
            Some("vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json")
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn downstream_preview_ready_blocker_parity_guard_detects_inconsistency() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            super::downstream_dispatch_ready_blocker_parity_error(true, &blockers),
            Some(
                "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                    .to_string()
            )
        );
        assert!(super::downstream_dispatch_ready_blocker_parity_error(false, &blockers).is_none());
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

        let config = project_activator_surface::read_yaml_file_checked(
            &harness.path().join("vida.config.yaml"),
        )
        .expect("config");
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

        let carrier_runtime_assignment = plan["carrier_runtime_assignment"].clone();
        let runtime_assignment = plan["runtime_assignment"].clone();
        assert_eq!(carrier_runtime_assignment, runtime_assignment);
        assert!(plan.get("codex_runtime_assignment").is_none());
        assert!(runtime_assignment.get("internal_named_lane_id").is_none());
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

        let config = project_activator_surface::read_yaml_file_checked(
            &harness.path().join("vida.config.yaml"),
        )
        .expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let carrier_runtime = bundle["carrier_runtime"].clone();
        assert!(bundle.get("codex_multi_agent").is_none());
        let dispatch_aliases = carrier_runtime["dispatch_aliases"]
            .as_array()
            .expect("dispatch aliases should still be an array");

        assert!(dispatch_aliases.is_empty());
    }

    #[test]
    fn project_activator_fails_closed_when_dispatch_alias_registry_is_configured_but_missing() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace(
            "dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml",
            "dispatch_aliases: .vida/project/agent-extensions/missing-dispatch-aliases.yaml",
        );
        fs::write(&config_path, updated).expect("config should be rewritten");

        assert_ne!(
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

        let scorecards = read_json_file_if_present(&harness.path().join(WORKER_SCORECARDS_STATE))
            .expect("scorecards should exist");
        let rows = scorecards["agents"]["junior"]["feedback"]
            .as_array()
            .expect("feedback rows should render");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["score"], 92);
        assert_eq!(rows[0]["outcome"], "success");
        assert_eq!(rows[0]["task_class"], "implementation");

        let strategy = read_json_file_if_present(&harness.path().join(WORKER_STRATEGY_STATE))
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
    fn agent_feedback_records_scorecard_for_non_codex_selected_system() {
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
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-feedback",
                "--agent-id",
                "qwen-primary",
                "--score",
                "81",
                "--outcome",
                "success",
                "--task-class",
                "implementation",
                "--notes",
                "external carrier feedback",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let scorecards = read_json_file_if_present(&harness.path().join(WORKER_SCORECARDS_STATE))
            .expect("scorecards should exist");
        let rows = scorecards["agents"]["qwen-primary"]["feedback"]
            .as_array()
            .expect("feedback rows should render");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["score"], 81);
        assert_eq!(rows[0]["outcome"], "success");
    }

    #[test]
    fn merge_project_activation_marks_init_pending_when_activation_is_incomplete() {
        let init_view = serde_json::json!({
            "status": "ready"
        });
        let project_activation_view = serde_json::json!({
            "status": "pending",
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

        let merged = project_activator_surface::merge_project_activation_into_init_view(
            init_view,
            &project_activation_view,
        );

        assert_eq!(merged["status"], "pending");
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
    fn runtime_dispatch_packet_contract_accepts_template_specific_minimums() {
        let delivery = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": runtime_delivery_task_packet(
                "run-1",
                "implementer",
                "worker",
                "implementation",
                "implementation",
                "request text",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&delivery, "test packet").is_ok());

        let coach = serde_json::json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": runtime_coach_review_packet(
                "run-1",
                "coach",
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&coach, "test packet").is_ok());

        let verifier = serde_json::json!({
            "packet_template_kind": "verifier_proof_packet",
            "verifier_proof_packet": runtime_verifier_proof_packet(
                "run-1",
                "verification",
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&verifier, "test packet").is_ok());
    }

    #[test]
    fn runtime_dispatch_packet_contract_fails_closed_for_missing_required_fields() {
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "scope_in": ["dispatch_target:implementer"],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?"
            }
        });
        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("packet without goal should fail closed");
        assert!(error.contains("missing required packet fields"));
        assert!(error.contains("goal"));
    }

    #[test]
    fn runtime_dispatch_packet_carries_external_host_runtime_contract() {
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
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let state_root = harness.path().join(".vida/data/state");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement backend execution".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "qwen-primary",
                            "activation_runtime_role": "worker",
                            "closure_class": "implementation",
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-qwen-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen-primary".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let packet = read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");
        assert_eq!(packet["host_runtime"]["selected_cli_system"], "qwen");
        assert_eq!(
            packet["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(packet["host_runtime"]["runtime_template_root"], ".qwen");
        assert_eq!(packet["selected_backend"], "qwen-primary");
    }

    #[test]
    fn runtime_tracked_flow_packet_marks_view_only_materialization_semantics() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("pbi_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("work-pool-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "task_id": "feature-x-work-pool",
                        "title": "Work-pool pack: Feature X",
                        "runtime": "vida taskflow",
                        "inspect_command": "vida task show feature-x-work-pool --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open --json",
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open --json",
                        "close_command": "vida task close feature-x-work-pool --reason 'closed' --json",
                        "required": true
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let packet = runtime_tracked_flow_packet(&role_selection, "run-1", "work-pool-pack");
        assert_eq!(
            packet["activation_semantics"],
            "tracked_flow_materialization_only"
        );
        assert_eq!(packet["view_only"], true);
        assert_eq!(packet["executes_packet"], false);
        assert_eq!(packet["transfers_root_session_write_authority"], false);
    }

    #[test]
    fn execute_runtime_dispatch_handoff_reuses_canonical_agent_init_packet_view() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let state_root = taskflow_task_bridge::proxy_state_dir();
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    "continue development"
                ),
                "dispatch_target": "implementer",
                "request_text": "continue development",
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &store,
                &role_selection,
                &receipt,
            ))
            .expect("agent-lane dispatch handoff should render");

        assert_eq!(result["surface"], "vida agent-init");
        assert_eq!(result["execution_state"], "packet_ready");
        assert_eq!(result["selection"]["mode"], "dispatch_packet");
        assert_eq!(result["selection"]["selected_role"], "worker");
        assert_eq!(
            result["activation_semantics"]["activation_kind"],
            "activation_view"
        );
        assert_eq!(result["activation_semantics"]["view_only"], true);
        assert_eq!(
            result["activation_command"],
            serde_json::json!(format!(
                "vida agent-init --dispatch-packet {} --json",
                shell_quote(&dispatch_packet_path.display().to_string())
            ))
        );
        assert_eq!(result["role_selection"]["selected_role"], "worker");
    }

    #[test]
    fn execute_runtime_dispatch_handoff_executes_configured_external_backend() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "command: qwen\n        static_args:\n          - -y\n          - -o\n          - text",
            "command: sh\n        static_args:\n          - -lc\n          - \"printf 'external-dispatch:%s' \\\"$1\\\"\"\n          - vida-dispatch",
        );
        fs::write(&config_path, updated).expect("config should update");

        let state_root = taskflow_task_bridge::proxy_state_dir();
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    "continue development"
                ),
                "dispatch_target": "implementer",
                "request_text": "continue development",
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &store,
                &role_selection,
                &receipt,
            ))
            .expect("external agent-lane dispatch handoff should execute");

        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(result["backend_dispatch"]["backend_id"], "qwen_cli");
        assert!(result["activation_command"]
            .as_str()
            .expect("activation command should render")
            .contains("sh"));
        assert!(result["provider_output"]
            .as_str()
            .expect("provider output should render")
            .contains("external-dispatch:Read and execute the VIDA dispatch packet"));
        assert_eq!(result["role_selection"]["selected_role"], "worker");
    }

    #[test]
    fn runtime_agent_lane_dispatch_prefers_receipt_selected_backend_for_external_hosts() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            "/tmp/runtime-dispatch-packet.json",
            Some("kilo_cli"),
        );

        assert_eq!(dispatch.surface, "external_cli:kilo_cli");
        assert!(
            dispatch.activation_command.contains("kilo"),
            "expected kilo command, got {}",
            dispatch.activation_command
        );
        assert!(
            dispatch.activation_command.contains("--dir"),
            "expected workdir flag, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_id"], "kilo_cli");
    }

    #[test]
    fn runtime_agent_lane_dispatch_keeps_internal_hosts_on_agent_init() {
        let _lock = current_dir_lock().lock().expect("lock should succeed");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = CurrentDirGuard::change_to(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            "/tmp/runtime-dispatch-packet.json",
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert!(
            dispatch.activation_command.contains("vida agent-init"),
            "expected canonical internal activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
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
                "    dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml\n",
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
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
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
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let overlay =
            project_activator_surface::read_yaml_file_checked(&root.join("vida.config.yaml"))
                .expect("overlay should parse");
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
                "    dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml\n",
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
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
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
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let overlay =
            project_activator_surface::read_yaml_file_checked(&root.join("vida.config.yaml"))
                .expect("overlay should parse");
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
            project_activator_surface::read_yaml_file_checked(&root.join("vida.config.yaml"))
                .expect("overlay should parse");
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
            !init_surfaces::looks_like_init_bootstrap_source_root(root),
            "taskflow binary alone should not qualify as an init bootstrap source"
        );

        fs::create_dir_all(root.join("install/assets")).expect("install assets dir should exist");
        fs::create_dir_all(root.join(".codex")).expect(".codex dir should exist");
        fs::write(
            root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("generated AGENTS scaffold should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("project sidecar should exist");
        fs::write(
            root.join("install/assets/vida.config.yaml.template"),
            "project:\n  id: demo\n",
        )
        .expect("config template should exist");
        assert!(
            init_surfaces::looks_like_init_bootstrap_source_root(root),
            "bootstrap source should require actual init assets rather than runtime-only markers"
        );
    }

    #[test]
    fn downstream_lane_superseded_requires_supersedes_receipt_evidence() {
        let blocker = missing_downstream_lane_evidence_blocker(
            Some(LaneStatus::LaneSuperseded),
            None,
            Some("exception-1"),
        );
        assert_eq!(blocker, Some(BlockerCode::MissingLaneReceipt));
    }

    #[test]
    fn downstream_lane_exception_takeover_guard_remains_unchanged() {
        let blocker = missing_downstream_lane_evidence_blocker(
            Some(LaneStatus::LaneExceptionTakeover),
            None,
            None,
        );
        assert_eq!(blocker, Some(BlockerCode::ExceptionPathMissing));
    }

    #[test]
    fn release1_operator_contracts_envelope_normalizes_status_to_canonical_vocabulary() {
        let envelope = build_release1_operator_contracts_envelope(
            " pass ",
            Vec::new(),
            Vec::new(),
            serde_json::json!({}),
        );

        assert_eq!(envelope["status"], "pass");
    }

    #[test]
    fn release1_operator_contracts_envelope_accepts_ok_compat_status() {
        let envelope = build_release1_operator_contracts_envelope(
            "ok",
            Vec::new(),
            Vec::new(),
            serde_json::json!({}),
        );

        assert_eq!(envelope["status"], "pass");
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
            verdict: Some("ready".to_string()),
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
            verdict: Some("blocked".to_string()),
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
    fn taskflow_consume_final_verdict_blocks_when_readiness_artifact_path_missing() {
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
            artifact_path: None,
            output: String::new(),
        };
        let proof = RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec!["missing_inventory_or_projection_evidence"]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_blocks_when_readiness_artifact_path_empty() {
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
            artifact_path: Some("   ".to_string()),
            output: String::new(),
        };
        let proof = RuntimeConsumptionEvidence {
            surface: "proof".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: None,
            output: "✅ OK: proofcheck".to_string(),
        };

        let verdict = build_docflow_runtime_verdict(&registry, &check, &readiness, &proof);

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(
            verdict.blockers,
            vec!["missing_inventory_or_projection_evidence"]
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
            activation_status: "ready_enough_for_normal_work".to_string(),
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
            activation_status: "pending".to_string(),
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
                "boot_incompatible",
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
            activation_status: "ready_enough_for_normal_work".to_string(),
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
        assert_eq!(
            admission.blockers,
            vec!["pending_design_packet", "pending_developer_handoff_packet"]
        );
    }

    #[test]
    fn taskflow_consume_final_verdict_blocks_when_proof_verdict_is_missing() {
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

        assert_eq!(verdict.status, "block");
        assert!(!verdict.ready);
        assert_eq!(verdict.blockers, vec!["missing_proof_verdict"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn taskflow_consume_final_fails_closed_when_latest_dispatch_receipt_summary_is_missing() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-consume-final-summary-missing-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");

        let latest_status = crate::state_store::RunGraphStatus {
            run_id: "run-final".to_string(),
            task_id: "task-final".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("worker".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "taskflow_state_store".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "runtime_consumption_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_worker".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.worker".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let mut payload = serde_json::json!({
            "dispatch_receipt": {
                "run_id": "run-final",
                "dispatch_status": "executed",
                "lane_status": "lane_running",
                "blocker_code": serde_json::Value::Null,
            },
            "direct_consumption_ready": true,
        });

        let blocker_code =
            runtime_consumption_final_dispatch_receipt_blocker_code(&store, &payload)
                .expect("blocker evaluation should succeed")
                .expect("missing receipt summary should fail closed");
        assert_eq!(
            blocker_code,
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER
        );

        apply_runtime_consumption_final_dispatch_receipt_blocker(&mut payload, &blocker_code);
        assert_eq!(payload["direct_consumption_ready"], false);
        assert_eq!(payload["dispatch_receipt"]["blocker_code"], blocker_code);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn taskflow_consume_final_propagates_checkpoint_leakage_blocker_code() {
        let payload = serde_json::json!({
            "dispatch_receipt": {
                "run_id": "run-final",
                "dispatch_status": "executed",
                "lane_status": "lane_open",
                "blocker_code": serde_json::Value::Null,
            },
            "direct_consumption_ready": true,
        });

        let blocker_code =
            runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
                "run-final",
                "run-final",
                Err(
                    "invalid task record: run-graph dispatch receipt summary is inconsistent for `run-final`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id=run-older)"
                        .to_string(),
                ),
            )
            .expect("blocker evaluation should succeed")
            .expect("checkpoint leakage should fail closed");
        assert_eq!(
            blocker_code,
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER
        );

        let mut payload = payload;
        apply_runtime_consumption_final_dispatch_receipt_blocker(&mut payload, &blocker_code);
        assert_eq!(payload["direct_consumption_ready"], false);
        assert_eq!(payload["dispatch_receipt"]["blocker_code"], blocker_code);
    }
}
