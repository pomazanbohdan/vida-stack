mod activation_status;
mod agent_feedback_surface;
mod cli;
mod config_value_utils;
mod docflow_proxy;
mod doctor_surface;
mod init_surfaces;
mod memory_surface;
mod operator_contracts;
mod project_activator_surface;
mod protocol_surface;
mod release1_contracts;
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
use std::time::SystemTime;
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
use docflow_cli::{
    CheckArgs as DocflowCheckArgs, Cli as DocflowCli, Command as DocflowCommand,
    ProofcheckArgs as DocflowProofcheckArgs, RegistryScanArgs,
};
pub(crate) use init_surfaces::resolve_init_bootstrap_source_root;
pub(crate) use project_activator_surface::build_project_activator_view;
pub(crate) use project_activator_surface::merge_project_activation_into_init_view;
pub(crate) use project_activator_surface::ProjectActivationAnswers;
use release1_contracts::{
    blocker_code_str, blocker_code_value, canonical_release1_contract_status_str,
    derive_lane_status, missing_downstream_lane_evidence_blocker, BlockerCode, LaneStatus,
};
use state_store::{LauncherActivationSnapshot, StateStore, StateStoreError};
pub(crate) use surface_render::{
    print_root_help, print_surface_header, print_surface_line, print_surface_ok,
};
use task_cli_render::{
    print_blocked_tasks, print_task_critical_path, print_task_dependencies,
    print_task_dependency_mutation, print_task_dependency_tree, print_task_export_summary,
    print_task_graph_issues, print_task_list, print_task_mutation, print_task_next_display_id,
    print_task_show,
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
const DEFAULT_PROJECT_CODEX_GUIDE_DOC: &str = "docs/process/codex-agent-configuration-guide.md";
const DEFAULT_PROJECT_DOC_TOOLING_DOC: &str = "docs/process/documentation-tooling-map.md";
const DEFAULT_PROJECT_RESEARCH_README: &str = "docs/research/README.md";
const PROJECT_ACTIVATION_RECEIPT_LATEST: &str = ".vida/receipts/project-activation.latest.json";
const SPEC_BOOTSTRAP_RECEIPT_LATEST: &str = ".vida/receipts/spec-bootstrap.latest.json";
const CODEX_WORKER_SCORECARDS_STATE: &str = ".vida/state/worker-scorecards.json";
const CODEX_WORKER_STRATEGY_STATE: &str = ".vida/state/worker-strategy.json";
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
    run(Cli::parse()).await
}

async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        None => {
            print_root_help();
            ExitCode::SUCCESS
        }
        Some(Command::Init(args)) => init_surfaces::run_init(args).await,
        Some(Command::Boot(args)) => init_surfaces::run_boot(args).await,
        Some(Command::OrchestratorInit(args)) => init_surfaces::run_orchestrator_init(args).await,
        Some(Command::AgentInit(args)) => init_surfaces::run_agent_init(args).await,
        Some(Command::Protocol(args)) => protocol_surface::run_protocol(args).await,
        Some(Command::ProjectActivator(args)) => {
            project_activator_surface::run_project_activator(args).await
        }
        Some(Command::AgentFeedback(args)) => {
            agent_feedback_surface::run_agent_feedback(args).await
        }
        Some(Command::Task(args)) => task_surface::run_task(args).await,
        Some(Command::Memory(args)) => memory_surface::run_memory(args).await,
        Some(Command::Status(args)) => status_surface::run_status(args).await,
        Some(Command::Doctor(args)) => doctor_surface::run_doctor(args).await,
        Some(Command::Taskflow(args)) => run_taskflow_proxy(args).await,
        Some(Command::Docflow(args)) => docflow_proxy::run_docflow_proxy(args),
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
    let estimated_units = input.estimated_task_price_units.unwrap_or_default();
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
    if budget
        .get("by_agent_id")
        .and_then(serde_json::Value::as_object)
        .is_none()
    {
        budget.insert("by_agent_id".to_string(), serde_json::json!({}));
    }
    if budget
        .get("by_task_class")
        .and_then(serde_json::Value::as_object)
        .is_none()
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

fn carrier_runtime_section<'a>(compiled_bundle: &'a serde_json::Value) -> &'a serde_json::Value {
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
        "preferred_agent_type": runtime_assignment["selected_agent_id"],
        "preferred_agent_tier": runtime_assignment["selected_tier"],
        "preferred_runtime_role": runtime_assignment["runtime_role"],
        "runtime_assignment": runtime_assignment.clone(),
        "codex_runtime_assignment": runtime_assignment,
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
    runtime_roles.is_empty() || runtime_roles.contains(&runtime_role)
}

fn role_supports_codex_task_class(role: &serde_json::Value, task_class: &str) -> bool {
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    task_classes.is_empty() || task_classes.contains(&task_class)
}

fn codex_dispatch_alias_row<'a>(
    compiled_bundle: &'a serde_json::Value,
    alias_id: &str,
) -> Option<&'a serde_json::Value> {
    carrier_runtime_section(compiled_bundle)["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["role_id"].as_str() == Some(alias_id))
}

fn build_codex_runtime_assignment_from_dispatch_alias(
    compiled_bundle: &serde_json::Value,
    alias_id: &str,
    fallback_task_class: &str,
) -> serde_json::Value {
    let Some(alias) = codex_dispatch_alias_row(compiled_bundle, alias_id) else {
        return serde_json::json!({
            "enabled": false,
            "reason": "dispatch_alias_missing",
            "dispatch_alias_id": alias_id,
            "task_class": fallback_task_class,
        });
    };
    let runtime_role = json_string(alias.get("default_runtime_role"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| codex_runtime_role_for_task_class(fallback_task_class).to_string());
    let task_class = alias["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .find(|value| !value.is_empty())
        .unwrap_or(fallback_task_class)
        .to_string();
    let mut assignment = build_codex_runtime_assignment_from_resolved_constraints(
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
        && codex_dispatch_alias_row(compiled_bundle, preferred_alias_id).is_some()
    {
        return Some(preferred_alias_id.to_string());
    }
    let runtime_role = codex_runtime_role_for_task_class(task_class);
    compiled_bundle["codex_multi_agent"]["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|alias| {
            role_supports_runtime_role(alias, runtime_role)
                && role_supports_codex_task_class(alias, task_class)
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
            "completion_blocker": "pending_specification_evidence",
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
            "completion_blocker": "pending_execution_preparation_evidence",
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
            "completion_blocker": "pending_implementation_evidence",
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
            "completion_blocker": "pending_review_clean_evidence",
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
            "completion_blocker": "pending_verification_evidence",
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
                build_codex_runtime_assignment_from_dispatch_alias(
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

fn build_codex_runtime_assignment_from_resolved_constraints(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
) -> serde_json::Value {
    let carrier_runtime = carrier_runtime_section(compiled_bundle);
    let Some(roles) = carrier_runtime["roles"].as_array() else {
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
            let supports_task_class = role_supports_codex_task_class(role, task_class);
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
    let complexity_multiplier = codex_task_complexity_multiplier(task_class);
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
        build_codex_runtime_assignment(compiled_bundle, selection, requires_design_gate);
    let lane_sequence = dispatch_contract["lane_sequence"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    serde_json::json!({
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
        "runtime_assignment": runtime_assignment.clone(),
        "codex_runtime_assignment": runtime_assignment,
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
        "carrier_runtime": carrier_runtime.clone(),
        "codex_multi_agent": carrier_runtime,
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
    if readiness
        .artifact_path
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        blockers.push("missing_inventory_or_projection_evidence".to_string());
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
        blockers.extend(bundle_check.blockers.iter().cloned());
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
        blockers.push("pending_developer_handoff_packet".to_string());
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
    let lane_catalog = dispatch_contract["lane_catalog"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let activation_chain = lane_catalog
        .iter()
        .map(|(target, lane)| {
            (
                target.clone(),
                dispatch_contract_lane_activation(lane).clone(),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    if execution_plan["status"] == "design_first" {
        return serde_json::json!({
            "status": "spec_first_handoff_required",
            "orchestration_contract": execution_plan["orchestration_contract"],
            "tracked_flow_bootstrap": execution_plan["tracked_flow_bootstrap"],
            "design_packet_activation": runtime_assignment_from_execution_plan(execution_plan),
            "post_design_activation_chain": activation_chain,
            "post_design_lane_contract": lane_catalog,
            "handoff_ready": true,
        });
    }

    serde_json::json!({
        "status": "execution_handoff_ready",
        "orchestration_contract": execution_plan["orchestration_contract"],
        "activation_chain": activation_chain,
        "lane_contract": lane_catalog,
        "runtime_assignment": runtime_assignment_from_execution_plan(execution_plan),
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

fn missing_agent_lane_dispatch_packet_error(dispatch_target: &str) -> String {
    let _ = blocker_code_str(BlockerCode::MissingPacket);
    format!("Agent lane dispatch for `{dispatch_target}` is missing dispatch_packet_path")
}

fn downstream_activation_fields(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" => (
            "taskflow_pack".to_string(),
            match dispatch_target {
                "spec-pack" => Some("vida taskflow bootstrap-spec".to_string()),
                "work-pool-pack" => Some("vida task create".to_string()),
                "dev-pack" => Some("vida task create".to_string()),
                _ => None,
            },
            None,
            None,
        ),
        "closure" => ("closure".to_string(), None, None, None),
        _ => {
            let lane = dispatch_contract_lane(&role_selection.execution_plan, dispatch_target);
            (
                "agent_lane".to_string(),
                Some("vida agent-init".to_string()),
                lane.and_then(|row| {
                    json_string(dispatch_contract_lane_activation(row).get("activation_agent_type"))
                }),
                lane.and_then(|row| {
                    json_string(
                        dispatch_contract_lane_activation(row).get("activation_runtime_role"),
                    )
                }),
            )
        }
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
    let selected_backend = activation_agent_type
        .clone()
        .or_else(|| receipt.selected_backend.clone())
        .filter(|value| !value.is_empty());
    let dispatch_status = if receipt.downstream_dispatch_ready {
        "routed".to_string()
    } else {
        "blocked".to_string()
    };
    Some(crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: dispatch_status.clone(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        lane_status: derive_lane_status(
            &dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: receipt.downstream_dispatch_command.clone(),
        dispatch_packet_path: receipt.downstream_dispatch_packet_path.clone(),
        dispatch_result_path: None,
        blocker_code: if dispatch_status == "blocked" && receipt.dispatch_status != "executed" {
            blocker_code_value(BlockerCode::MissingLaneReceipt)
        } else if dispatch_status == "blocked" && receipt.downstream_dispatch_packet_path.is_none()
        {
            blocker_code_value(BlockerCode::MissingPacket)
        } else {
            None
        },
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
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
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
    root_receipt.downstream_dispatch_active_target =
        step_receipt.downstream_dispatch_active_target.clone();
    root_receipt.supersedes_receipt_id = step_receipt.supersedes_receipt_id.clone();
    root_receipt.exception_path_receipt_id = step_receipt.exception_path_receipt_id.clone();
    root_receipt.blocker_code = step_receipt.blocker_code.clone();
}

fn active_downstream_dispatch_target(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if receipt.dispatch_kind == "agent_lane" && receipt.dispatch_status != "executed" {
        Some(receipt.dispatch_target.clone())
    } else {
        None
    }
}

fn agent_init_packet_flag_for_path(packet_path: &str) -> &'static str {
    if packet_path.contains("/downstream-dispatch-packets/")
        || packet_path.contains("downstream-dispatch-packets")
    {
        "--downstream-packet"
    } else {
        "--dispatch-packet"
    }
}

fn agent_init_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --json",
        agent_init_packet_flag_for_path(packet_path),
        shell_quote(packet_path)
    )
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
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    dispatch_kind: &str,
) -> String {
    if dispatch_kind == "taskflow_pack" {
        return "tracked_flow_packet".to_string();
    }
    dispatch_contract_lane(execution_plan, dispatch_target)
        .and_then(|lane| json_string(lane.get("packet_template_kind")))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "delivery_task_packet".to_string())
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
    let agent_only_development =
        execution_plan_agent_only_development_required(&role_selection.execution_plan);
    let dispatch_contract = &role_selection.execution_plan["development_flow"]["dispatch_contract"];
    let lane_sequence = dispatch_contract_lane_sequence(dispatch_contract);
    let execution_lane_sequence = dispatch_contract_execution_lane_sequence(dispatch_contract);
    match receipt.dispatch_target.as_str() {
        "spec-pack" if agent_only_development => (
            Some(
                lane_sequence
                    .first()
                    .map(|value| value.as_str())
                    .unwrap_or("specification")
                    .to_string(),
            ),
            Some("vida agent-init".to_string()),
            Some(
                "after spec-pack task materialization, dispatch the business-analyst lane for bounded research/specification/planning before work-pool shaping"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
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
            Some(
                execution_lane_sequence
                    .first()
                    .map(|value| value.as_str())
                    .unwrap_or("implementer")
                    .to_string(),
            ),
            Some("vida agent-init".to_string()),
            Some(
                "after the dev packet is created, activate the selected implementer lane for bounded execution"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
        _ if receipt.dispatch_kind == "agent_lane" => {
            let current_lane =
                dispatch_contract_lane(&role_selection.execution_plan, &receipt.dispatch_target);
            if current_lane.and_then(|lane| lane["stage"].as_str()) == Some("design_gate")
                || (receipt.dispatch_target == "specification"
                    && current_lane.and_then(|lane| lane["stage"].as_str()).is_none()
                    && dispatch_contract.get("specification_activation").is_some())
            {
                let evidence_blocker = current_lane
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or("pending_specification_evidence");
                return (
                    Some("work-pool-pack".to_string()),
                    json_string(
                        role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                            .get("create_command"),
                    ),
                    Some(
                        if receipt.dispatch_status == "executed" {
                            "after specification/planning evidence is recorded, finalize the design doc and close spec-pack before work-pool shaping"
                        } else {
                            "specification/planning lane is active; wait for bounded evidence return before design finalization, spec-pack closure, and work-pool shaping"
                        }
                            .to_string(),
                    ),
                    false,
                    vec![
                        evidence_blocker.to_string(),
                        "pending_design_finalize".to_string(),
                        "pending_spec_task_close".to_string(),
                    ],
                );
            }
            let current_index = execution_lane_sequence
                .iter()
                .position(|target| target == &receipt.dispatch_target);
            let effective_current_target = current_index
                .map(|_| receipt.dispatch_target.clone())
                .or_else(|| {
                    receipt
                        .activation_runtime_role
                        .as_deref()
                        .and_then(canonical_lane_target_for_runtime_role)
                        .map(str::to_string)
                });
            let current_index = current_index.or_else(|| {
                receipt
                    .activation_runtime_role
                    .as_deref()
                    .and_then(canonical_lane_target_for_runtime_role)
                    .and_then(|target| {
                        execution_lane_sequence
                            .iter()
                            .position(|candidate| candidate == target)
                    })
            });
            let Some(current_index) = current_index else {
                return (None, None, None, false, Vec::new());
            };
            let next_target = execution_lane_sequence.get(current_index + 1);
            if let Some(next_target) = next_target {
                let blocker = effective_current_target
                    .as_deref()
                    .and_then(|target| dispatch_contract_lane(&role_selection.execution_plan, target))
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or("pending_lane_evidence")
                    .to_string();
                let has_lane_evidence = receipt.dispatch_status == "executed"
                    || receipt
                        .dispatch_result_path
                        .as_deref()
                        .is_some_and(|path| !path.trim().is_empty());
                (
                    Some(next_target.clone()),
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "after `{}` evidence is recorded, activate `{}` for the next bounded lane",
                        receipt.dispatch_target, next_target
                    )),
                    has_lane_evidence,
                    if has_lane_evidence {
                        Vec::new()
                    } else {
                        vec![blocker]
                    },
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

fn downstream_dispatch_ready_blocker_parity_error(
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: &[String],
) -> Option<String> {
    if downstream_dispatch_ready && !downstream_dispatch_blockers.is_empty() {
        return Some(
            "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                .to_string(),
        );
    }
    None
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
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    receipt.downstream_dispatch_target = downstream_dispatch_target;
    receipt.downstream_dispatch_command = downstream_dispatch_command;
    receipt.downstream_dispatch_note = downstream_dispatch_note;
    receipt.downstream_dispatch_ready = downstream_dispatch_ready;
    receipt.downstream_dispatch_blockers = downstream_dispatch_blockers;
    receipt.downstream_dispatch_status = None;
    receipt.downstream_dispatch_result_path = None;
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_active_target = active_downstream_dispatch_target(receipt);
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

fn runtime_packet_handoff_task_class(
    dispatch_target: &str,
    handoff_runtime_role: &str,
) -> &'static str {
    match dispatch_target {
        "specification" => "specification",
        "planning" => "planning",
        "coach" => "coach",
        "verification" => "verification",
        "escalation" => "architecture",
        "implementer" => "implementation",
        _ => match handoff_runtime_role {
            "business_analyst" => "specification",
            "pm" => "planning",
            "coach" => "coach",
            "verifier" => "verification",
            "solution_architect" => "architecture",
            _ => "implementation",
        },
    }
}

fn runtime_delivery_task_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
    request_text: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "backlog_id": run_id,
        "release_slice": "none",
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Execute bounded `{dispatch_target}` handoff for the active runtime request"),
        "non_goals": [
            "unbounded repository-wide rewrites",
            "out-of-scope taskflow state mutation"
        ],
        "scope_in": [
            format!("dispatch_target:{dispatch_target}"),
            format!("runtime_role:{handoff_runtime_role}")
        ],
        "scope_out": [
            "mutation outside bounded packet scope",
            "closure without recorded handoff evidence"
        ],
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "inputs": [
            "role_selection_full",
            "run_graph_bootstrap",
            "taskflow_handoff_plan"
        ],
        "outputs": [
            "dispatch_result_artifact",
            "updated_run_graph_dispatch_receipt"
        ],
        "definition_of_done": [
            format!("`{dispatch_target}` handoff produces a bounded runtime result artifact"),
            "dispatch receipt and downstream preview are refreshed consistently"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop after writing bounded dispatch result or explicit blocker",
            "do not widen scope beyond the active packet target"
        ],
        "blocking_question": format!("What is the next bounded action required for `{dispatch_target}`?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier",
        "request_excerpt": request_text.chars().take(240).collect::<String>(),
    })
}

fn runtime_execution_block_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::execution-block"),
        "parent_packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "backlog_id": run_id,
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Resolve bounded execution blocker for `{dispatch_target}`"),
        "scope_in": [
            format!("dispatch_target:{dispatch_target}")
        ],
        "scope_out": [
            "new feature scope without bounded packet update"
        ],
        "owned_paths": [],
        "definition_of_done": [
            "bounded blocker is resolved with receipt-backed evidence"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime receipt evidence that blocker is resolved or escalated",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop once blocker resolution evidence is recorded"
        ],
        "blocking_question": format!("Which explicit blocker prevents closing `{dispatch_target}` now?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier"
    })
}

fn runtime_coach_review_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::coach-review"),
        "source_packet_id": format!("{run_id}::implementer::delivery"),
        "review_goal": format!("Judge whether `{dispatch_target}` remains aligned with the approved bounded packet, acceptance criteria, and definition of done"),
        "owned_paths": [],
        "definition_of_done": [
            "coach review returns bounded approval-forward or bounded rework evidence"
        ],
        "proof_target": proof_target,
        "active_skills": "no_applicable_skill",
        "review_focus": [
            "spec_conformance",
            "acceptance_criteria_alignment",
            "bounded_scope_drift"
        ],
        "blocking_question": format!("Does `{dispatch_target}` match the approved bounded contract cleanly enough to proceed?"),
        "handoff_runtime_role": "coach",
        "handoff_task_class": "coach",
        "handoff_selection": "runtime_selected_tier",
    })
}

fn runtime_verifier_proof_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::verifier-proof"),
        "source_packet_id": format!("{run_id}::implementer::delivery"),
        "proof_goal": format!("Independently verify bounded closure readiness for `{dispatch_target}`"),
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": proof_target,
        "owned_paths": [],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("What proof is still missing before `{dispatch_target}` can close?"),
        "handoff_runtime_role": "verifier",
        "handoff_task_class": "verification",
        "handoff_selection": "runtime_selected_tier",
    })
}

fn runtime_escalation_packet(run_id: &str, dispatch_target: &str) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::escalation"),
        "source_packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "conflict_type": "architecture",
        "decision_needed": format!("Resolve the bounded architecture-preparation or escalation decision for `{dispatch_target}`"),
        "options": [
            "approve current bounded route",
            "reshape bounded handoff",
            "block execution pending architectural clarification"
        ],
        "constraints": [
            "preserve one bounded packet owner",
            "do not widen scope without a new bounded packet"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("Which architectural decision is required before `{dispatch_target}` can proceed coherently?"),
        "handoff_runtime_role": "solution_architect",
        "handoff_task_class": "architecture",
        "handoff_selection": "runtime_selected_tier",
    })
}

fn runtime_packet_prompt(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    request_text: &str,
    orchestration_contract: &serde_json::Value,
) -> String {
    let replan_points = orchestration_contract["replanning"]["checkpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Packet run_id={run_id}\nTarget={dispatch_target}\nRuntime role={handoff_runtime_role}\nRoot session role=orchestrator\nExecution mode=delegated_orchestration_cycle\nFirst substantive response: publish a concise plan before edits or implementation.\nLocal orchestrator coding is forbidden without an explicit exception path.\nReplan checkpoints: {replan_points}\nGoal: execute only this bounded handoff and produce receipt-backed evidence.\nRequest: {request_text}"
    )
}

fn runtime_dispatch_command_for_packet_path(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: &str,
) -> Option<String> {
    match receipt.dispatch_kind.as_str() {
        "taskflow_pack" => {
            runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target)
        }
        "agent_lane" => Some(agent_init_command_for_packet_path(packet_path)),
        _ => runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target),
    }
}

pub(crate) struct RuntimeDispatchPacketContext<'a> {
    pub(crate) state_root: &'a Path,
    pub(crate) role_selection: &'a RuntimeConsumptionLaneSelection,
    pub(crate) receipt: &'a crate::state_store::RunGraphDispatchReceipt,
    pub(crate) taskflow_handoff_plan: &'a serde_json::Value,
    pub(crate) run_graph_bootstrap: &'a serde_json::Value,
}

impl<'a> RuntimeDispatchPacketContext<'a> {
    pub(crate) fn new(
        state_root: &'a Path,
        role_selection: &'a RuntimeConsumptionLaneSelection,
        receipt: &'a crate::state_store::RunGraphDispatchReceipt,
        taskflow_handoff_plan: &'a serde_json::Value,
        run_graph_bootstrap: &'a serde_json::Value,
    ) -> Self {
        Self {
            state_root,
            role_selection,
            receipt,
            taskflow_handoff_plan,
            run_graph_bootstrap,
        }
    }
}

#[cfg(test)]
mod runtime_dispatch_packet_context_tests {
    use super::*;
    use crate::state_store::RunGraphDispatchReceipt;
    use serde_json::json;

    #[test]
    fn context_preserves_inputs() {
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test-mode".to_string(),
            fallback_role: "junior".to_string(),
            request: "req".to_string(),
            selected_role: "junior".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: json!({}),
            execution_plan: json!({ "orchestration_contract": {}, "tracked_flow_bootstrap": {} }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
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
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let execution_plan_value = json!({"plan": "value"});
        let bootstrap_value = json!({"bootstrap": "value"});
        let ctx = RuntimeDispatchPacketContext::new(
            Path::new("/tmp"),
            &selection,
            &receipt,
            &execution_plan_value,
            &bootstrap_value,
        );
        assert_eq!(ctx.receipt.run_id, "run-test");
        assert_eq!(ctx.role_selection.request, "req");
    }
}

fn write_runtime_dispatch_packet(ctx: &RuntimeDispatchPacketContext<'_>) -> Result<String, String> {
    let packet_dir = ctx
        .state_root
        .join("runtime-consumption")
        .join("dispatch-packets");
    std::fs::create_dir_all(&packet_dir)
        .map_err(|error| format!("Failed to create dispatch-packets directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", ctx.receipt.run_id));
    let packet_path_display = packet_path.display().to_string();
    let packet_template_kind = runtime_dispatch_packet_kind(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
        &ctx.receipt.dispatch_kind,
    );
    let handoff_runtime_role = ctx
        .receipt
        .activation_runtime_role
        .as_deref()
        .unwrap_or(ctx.role_selection.selected_role.as_str());
    let handoff_task_class =
        runtime_packet_handoff_task_class(&ctx.receipt.dispatch_target, handoff_runtime_role);
    let closure_class = dispatch_contract_lane(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
    )
    .and_then(|lane| lane["closure_class"].as_str())
    .unwrap_or("implementation");
    let activation_command = runtime_dispatch_command_for_packet_path(
        ctx.role_selection,
        ctx.receipt,
        &packet_path_display,
    );
    let delivery_task_packet = runtime_delivery_task_packet(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &ctx.role_selection.request,
    );
    let execution_block_packet = runtime_execution_block_packet(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    let body = serde_json::json!({
        "packet_kind": "runtime_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet.clone()
        } else {
            serde_json::Value::Null
        },
        "execution_block_packet": if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
        "coach_review_packet": if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&ctx.receipt.run_id, &ctx.receipt.dispatch_target)
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &ctx.receipt.run_id,
            &ctx.receipt.dispatch_target,
            handoff_runtime_role,
            &ctx.role_selection.request,
            &ctx.role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": ctx.receipt.recorded_at,
        "run_id": ctx.receipt.run_id,
        "dispatch_target": ctx.receipt.dispatch_target,
        "dispatch_status": ctx.receipt.dispatch_status,
        "lane_status": ctx.receipt.lane_status,
        "blocker_code": ctx.receipt.blocker_code,
        "supersedes_receipt_id": ctx.receipt.supersedes_receipt_id,
        "exception_path_receipt_id": ctx.receipt.exception_path_receipt_id,
        "dispatch_kind": ctx.receipt.dispatch_kind,
        "dispatch_surface": ctx.receipt.dispatch_surface,
        "dispatch_command": activation_command,
        "activation_agent_type": ctx.receipt.activation_agent_type,
        "activation_runtime_role": ctx.receipt.activation_runtime_role,
        "selected_backend": ctx.receipt.selected_backend,
        "request_text": ctx.role_selection.request,
        "role_selection": {
            "selected_role": ctx.role_selection.selected_role,
            "conversational_mode": ctx.role_selection.conversational_mode,
            "tracked_flow_entry": ctx.role_selection.tracked_flow_entry,
            "confidence": ctx.role_selection.confidence,
        },
        "role_selection_full": ctx.role_selection,
        "taskflow_handoff_plan": ctx.taskflow_handoff_plan,
        "run_graph_bootstrap": ctx.run_graph_bootstrap,
        "orchestration_contract": ctx.role_selection.execution_plan["orchestration_contract"],
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
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve current directory for dispatch execution: {error}")
        })?);
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
            "status": "pass",
            "closure_ready": true,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "note": "runtime downstream scheduler reached closure without additional lane activation",
        })),
        _ => {
            let dispatch_packet_path =
                receipt.dispatch_packet_path.as_deref().ok_or_else(|| {
                    missing_agent_lane_dispatch_packet_error(&receipt.dispatch_target)
                })?;
            let bundle =
                crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload(store)
                    .await?;
            let project_activation_view =
                project_activator_surface::build_project_activator_view(&project_root);
            let init_view = project_activator_surface::merge_project_activation_into_init_view(
                bundle.agent_init_view,
                &project_activation_view,
            );
            Ok(serde_json::json!({
                "surface": "vida agent-init",
                "status": "pass",
                "execution_state": "packet_ready",
                "activation_command": agent_init_command_for_packet_path(dispatch_packet_path),
                "dispatch_packet_path": dispatch_packet_path,
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
    let execution_state = json_string(execution_result.get("execution_state"))
        .unwrap_or_else(|| "executed".to_string());
    receipt.dispatch_status = execution_state;
    let closure_completed = receipt.dispatch_target == "closure"
        && receipt.dispatch_status == "executed"
        && json_bool(execution_result.get("closure_ready"), false);
    let mut lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    );
    if closure_completed {
        lane_status = LaneStatus::LaneCompleted;
    }
    receipt.lane_status = lane_status.as_str().to_string();
    receipt.blocker_code =
        if receipt.dispatch_status == "blocked" && receipt.dispatch_packet_path.is_none() {
            blocker_code_value(BlockerCode::MissingPacket)
        } else {
            None
        };
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    if receipt.dispatch_status == "executed" {
        if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
            if let Ok(status) = store.run_graph_status(&run_id).await {
                let executed_status =
                    apply_first_handoff_execution_to_run_graph_status(&status, receipt);
                store
                    .record_run_graph_status(&executed_status)
                    .await
                    .map_err(|error| {
                        format!("Failed to record executed run-graph status: {error}")
                    })?;
            }
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
    let root_lane_has_execution_evidence = root_receipt.dispatch_status == "executed"
        || (root_receipt.dispatch_status == "packet_ready"
            && root_receipt
                .dispatch_result_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()));
    if !root_lane_has_execution_evidence || !root_receipt.downstream_dispatch_ready {
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
        if downstream_receipt.dispatch_status != "routed"
            || (downstream_receipt.dispatch_kind == "taskflow_pack"
                && taskflow_task_bridge::infer_project_root_from_state_root(state_root).is_none())
        {
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
        if let Some(error) =
            downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
        {
            return Err(error);
        }
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
        downstream_receipt.downstream_dispatch_status =
            Some(downstream_receipt.dispatch_status.clone());
        downstream_receipt.downstream_dispatch_result_path =
            downstream_receipt.dispatch_result_path.clone();
        downstream_receipt.downstream_dispatch_active_target =
            active_downstream_dispatch_target(&downstream_receipt);
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
        if downstream_receipt.dispatch_status == "executed" {
            root_receipt.downstream_dispatch_executed_count += 1;
        }
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
    packet_path: Option<&Path>,
) -> serde_json::Value {
    let downstream_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .unwrap_or_default();
    let handoff_runtime_role = receipt
        .activation_runtime_role
        .as_deref()
        .unwrap_or(role_selection.selected_role.as_str());
    let packet_template_kind = if downstream_target.is_empty() {
        "delivery_task_packet".to_string()
    } else {
        runtime_dispatch_packet_kind(
            &role_selection.execution_plan,
            downstream_target,
            if matches!(
                downstream_target,
                "spec-pack" | "work-pool-pack" | "dev-pack"
            ) {
                "taskflow_pack"
            } else {
                "agent_lane"
            },
        )
    };
    let activation_command = packet_path
        .and_then(|path| path.to_str())
        .map(agent_init_command_for_packet_path);
    let handoff_task_class =
        runtime_packet_handoff_task_class(downstream_target, handoff_runtime_role);
    let closure_class = dispatch_contract_lane(&role_selection.execution_plan, downstream_target)
        .and_then(|lane| lane["closure_class"].as_str())
        .unwrap_or("implementation");
    let delivery_task_packet = runtime_delivery_task_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &role_selection.request,
    );
    let execution_block_packet = runtime_execution_block_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    let body = serde_json::json!({
        "packet_kind": "runtime_downstream_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet
        } else {
            serde_json::Value::Null
        },
        "execution_block_packet": if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
        "coach_review_packet": if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &receipt.run_id,
                downstream_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &receipt.run_id,
                downstream_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &receipt.run_id,
            downstream_target,
            handoff_runtime_role,
            &role_selection.request,
            &role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": receipt.recorded_at,
        "run_id": receipt.run_id,
        "source_dispatch_target": receipt.dispatch_target,
        "source_dispatch_status": receipt.dispatch_status,
        "source_lane_status": receipt.lane_status,
        "source_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "source_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "source_blocker_code": receipt.blocker_code,
        "downstream_dispatch_target": receipt.downstream_dispatch_target,
        "downstream_dispatch_command": activation_command.or_else(|| receipt.downstream_dispatch_command.clone()),
        "downstream_dispatch_note": receipt.downstream_dispatch_note,
        "downstream_dispatch_ready": receipt.downstream_dispatch_ready,
        "downstream_dispatch_blockers": receipt.downstream_dispatch_blockers,
        "downstream_dispatch_status": receipt.downstream_dispatch_status,
        "downstream_lane_status": receipt
            .downstream_dispatch_status
            .as_deref()
            .map(|status| {
                derive_lane_status(
                    status,
                    receipt.supersedes_receipt_id.as_deref(),
                    receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string()
            }),
        "downstream_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "downstream_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "downstream_dispatch_result_path": receipt.downstream_dispatch_result_path,
        "downstream_dispatch_active_target": receipt.downstream_dispatch_active_target,
        "activation_agent_type": receipt.activation_agent_type,
        "activation_runtime_role": receipt.activation_runtime_role,
        "role_selection_full": role_selection,
        "run_graph_bootstrap": run_graph_bootstrap,
        "orchestration_contract": role_selection.execution_plan["orchestration_contract"],
    });
    body
}

fn write_runtime_downstream_dispatch_packet_at(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let body = downstream_dispatch_packet_body(
        role_selection,
        run_graph_bootstrap,
        receipt,
        Some(packet_path),
    );
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
        _ => dispatch_contract_execution_lane_sequence(
            &role_selection.execution_plan["development_flow"]["dispatch_contract"],
        )
        .first()
        .map(|value| value.as_str())
        .unwrap_or(role_selection.selected_role.as_str())
        .to_string(),
    };
    let selected_route = if conversational_mode.is_some() {
        &role_selection.execution_plan["default_route"]
    } else {
        dispatch_contract_lane(&role_selection.execution_plan, &route_target).unwrap_or(
            runtime_assignment_from_execution_plan(&role_selection.execution_plan),
        )
    };
    let route_backend =
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, selected_route)
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

fn blocking_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let mut status = fallback_runtime_consumption_run_graph_status(role_selection, run_id);
    status.status = "blocked".to_string();
    status.next_node = None;
    status.lifecycle_stage = "runtime_consumption_blocked".to_string();
    status.handoff_state = "none".to_string();
    status.context_state = "open".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
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
                            let blocked_status = blocking_runtime_consumption_run_graph_status(
                                role_selection,
                                &run_id,
                            );
                            let blocked_status_json = serde_json::to_value(&blocked_status)
                                .unwrap_or(serde_json::Value::Null);
                            let blocked_write_error =
                                store.record_run_graph_status(&blocked_status).await.err();
                            return serde_json::json!({
                                "status": "blocked",
                                "handoff_ready": false,
                                "run_id": run_id,
                                "seed": seed_payload_json,
                                "latest_status": blocked_status_json,
                                "reason": if let Some(blocked_write_error) = blocked_write_error {
                                    format!(
                                        "record_advance_failed: {error}; compensating_blocked_record_failed: {blocked_write_error}"
                                    )
                                } else {
                                    format!("record_advance_failed: {error}")
                                },
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
            let status = blocking_runtime_consumption_run_graph_status(role_selection, &run_id);
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
                "status": "blocked",
                "handoff_ready": false,
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
            "consume_final_surface": "vida taskflow consume final",
        }),
    );
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": "vida taskflow consume final",
        "status": consume_final_status,
        "blocker_codes": consume_final_blocker_codes,
        "next_actions": consume_final_next_actions,
        "artifact_refs": operator_contracts["artifact_refs"].clone(),
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
            "status": consume_final_status,
            "blocker_codes": consume_final_blocker_codes,
            "next_actions": consume_final_next_actions,
            "artifact_refs": operator_contracts["artifact_refs"].clone(),
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
        blocker_codes.push("bundle_activation_not_ready".to_string());
    }
    if release1_status_is_blocked(&payload["docflow_verdict"]["status"]) {
        blocker_codes.push("docflow_verdict_block".to_string());
    }
    if release1_status_is_blocked(&payload["closure_admission"]["status"]) {
        blocker_codes.push("closure_admission_block".to_string());
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

fn runtime_consumption_final_dispatch_receipt_blocker_code(
    store: &StateStore,
    payload_json: &serde_json::Value,
) -> Result<Option<String>, String> {
    let Some(latest_status) = block_on_state_store(store.latest_run_graph_status())? else {
        return Ok(None);
    };
    let Some(payload_run_id) = payload_json["dispatch_receipt"]["run_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        ));
    };
    runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
        latest_status.run_id.as_str(),
        payload_run_id,
        block_on_state_store(store.latest_run_graph_dispatch_receipt_summary()),
    )
}

fn runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
    latest_status_run_id: &str,
    payload_run_id: &str,
    dispatch_receipt_summary: Result<
        Option<crate::state_store::RunGraphDispatchReceiptSummary>,
        String,
    >,
) -> Result<Option<String>, String> {
    if payload_run_id != latest_status_run_id {
        return Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        ));
    }

    match dispatch_receipt_summary {
        Ok(Some(summary)) if summary.run_id == latest_status_run_id => Ok(None),
        Ok(_) => Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        )),
        Err(error) if error.contains("latest checkpoint evidence must share the same run_id") => {
            Ok(Some(
                RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER.to_string(),
            ))
        }
        Err(error) => Err(error),
    }
}

fn apply_runtime_consumption_final_dispatch_receipt_blocker(
    payload_json: &mut serde_json::Value,
    blocker_code: &str,
) {
    if let Some(payload_object) = payload_json.as_object_mut() {
        payload_object.insert(
            "direct_consumption_ready".to_string(),
            serde_json::Value::Bool(false),
        );
    }
    if let Some(dispatch_receipt) = payload_json
        .get_mut("dispatch_receipt")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch_receipt.insert(
            "blocker_code".to_string(),
            serde_json::Value::String(blocker_code.to_string()),
        );
    }
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
        assert!(harness.path().join(CODEX_WORKER_SCORECARDS_STATE).is_file());
        assert!(harness.path().join(CODEX_WORKER_STRATEGY_STATE).is_file());
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
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".codex");
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
        assert_eq!(view["host_environment"]["template_materialized"], true);
        assert_eq!(view["host_environment"]["runtime_template_root"], ".qwen");
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
        assert!(view["host_environment"]["supported_cli_systems"]
            .as_array()
            .expect("supported cli systems should render")
            .iter()
            .any(|value| value.as_str() == Some("qwen")));
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
                .contains("runtime_assignment")
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
            assert_eq!(plan["runtime_assignment"], plan["codex_runtime_assignment"]);
            plan["runtime_assignment"].clone()
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
            dispatch_surface: Some("vida task create".to_string()),
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
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
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
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
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
                "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
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

        assert_eq!(plan["runtime_assignment"], plan["codex_runtime_assignment"]);
        assert!(plan["runtime_assignment"]
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

        let config = project_activator_surface::read_yaml_file_checked(
            &harness.path().join("vida.config.yaml"),
        )
        .expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        assert_eq!(bundle["carrier_runtime"], bundle["codex_multi_agent"]);
        let dispatch_aliases = bundle["carrier_runtime"]["dispatch_aliases"]
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
            verdict: None,
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
            verdict: None,
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
