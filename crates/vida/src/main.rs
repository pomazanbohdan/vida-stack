mod state_store;
mod taskflow_layer4;
mod taskflow_run_graph;
mod taskflow_runtime_bundle;
mod temp_state;

use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
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
    println!("  installed mode: compatibility wrapper with `help|overview only`.");
    println!("  unsupported or out-of-mode commands fail closed instead of silently falling through to donor wrappers.");
    println!();
    println!("Implemented in-process command surface:");
    let mut command = DocflowCli::command();
    let help = command.render_long_help().to_string();
    print!("{help}");
    if !help.ends_with('\n') {
        println!();
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum DocflowLauncherMode {
    RepoDev,
    InstalledCompatibility,
}

fn current_docflow_launcher_mode() -> DocflowLauncherMode {
    if resolve_installed_runtime_root().is_some() {
        DocflowLauncherMode::InstalledCompatibility
    } else {
        DocflowLauncherMode::RepoDev
    }
}

fn installed_docflow_command_allowed(args: &[String]) -> bool {
    args.is_empty()
        || matches!(args, [command] if command == "help")
        || matches!(args, [command, ..] if command == "overview")
}

fn looks_like_project_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file() && path.join("vida/root-map.md").is_file()
}

fn resolve_repo_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("VIDA_ROOT") {
        let root = PathBuf::from(root);
        if root.exists() {
            return Ok(root);
        }
        return Err(format!(
            "VIDA_ROOT points to a missing path: {}",
            root.display()
        ));
    }

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
        )),
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
        0 => resolve_repo_root(),
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

fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    let candidate = root.join("bin/taskflow-v0");
    candidate.exists().then(|| root.to_path_buf())
}

fn resolve_taskflow_binary() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("VIDA_TASKFLOW_BIN") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        return Err(format!(
            "VIDA_TASKFLOW_BIN points to a missing path: {}",
            path.display()
        ));
    }

    let root = resolve_repo_root()?;
    let mut candidates = Vec::new();
    if let Some(installed_root) = resolve_installed_runtime_root() {
        candidates.push(installed_root.join("bin/taskflow-v0"));
    }
    candidates.push(root.join("bin/taskflow-v0"));
    candidates.push(root.join("taskflow-v0/src/vida"));

    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve taskflow runtime for project root {}",
            root.display()
        )
    })
}

fn taskflow_runtime_roots(project_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(installed_root) = resolve_installed_runtime_root() {
        roots.push(installed_root);
        if let Some(project_root) = project_root {
            let project_root = project_root.to_path_buf();
            if !roots.iter().any(|root| root == &project_root) {
                roots.push(project_root);
            }
        }
        return roots;
    }

    let repo_root = repo_runtime_root();
    roots.push(repo_root);

    if let Some(project_root) = project_root {
        let project_root = project_root.to_path_buf();
        if !roots.iter().any(|root| root == &project_root) {
            roots.push(project_root);
        }
    }

    roots
}

fn resolve_taskflow_store_helper(project_root: Option<&Path>) -> Result<PathBuf, String> {
    let candidates = taskflow_runtime_roots(project_root)
        .into_iter()
        .map(|root| root.join("taskflow-v0/helpers/turso_task_store.py"))
        .collect::<Vec<_>>();
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve taskflow task-store helper. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
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

fn resolve_task_store_python(project_root: Option<&Path>) -> PathBuf {
    if let Some(path) = std::env::var_os("VIDA_V0_TURSO_PYTHON") {
        return PathBuf::from(path);
    }

    for root in taskflow_runtime_roots(project_root) {
        let candidate = root.join(".venv/bin/python3");
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from("python3")
}

fn legacy_task_store_db_path(project_root: &Path) -> PathBuf {
    project_root.join(".vida/state/vida-legacy.db")
}

fn helper_invalid_output(message: &str) -> String {
    serde_json::json!({
        "status": "error",
        "reason": "invalid_helper_output",
        "output": message,
    })
    .to_string()
}

fn run_task_store_helper(
    project_root: &Path,
    args: &[String],
) -> Result<serde_json::Value, String> {
    let helper = resolve_taskflow_store_helper(Some(project_root))?;
    let python = resolve_task_store_python(Some(project_root));
    let db_path = legacy_task_store_db_path(project_root);
    let output = ProcessCommand::new(&python)
        .arg(&helper)
        .arg("--db")
        .arg(&db_path)
        .args(args)
        .env("VIDA_ROOT", project_root)
        .current_dir(project_root)
        .output()
        .map_err(|error| format!("Failed to execute {}: {error}", python.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = if stdout.is_empty() {
        stderr
    } else if stderr.is_empty() {
        stdout
    } else {
        format!("{stdout}\n{stderr}")
    };

    serde_json::from_str(&combined).map_err(|_| helper_invalid_output(&combined))
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
            let rows =
                run_task_store_helper(project_root, &["list".to_string(), "--all".to_string()])?;
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
            Ok(if helper_value_is_ok(&payload) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
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
            if as_json {
                print_json_pretty(&payload);
            } else {
                print_json_pretty(&payload);
            }
            Ok(ExitCode::SUCCESS)
        }
        _ => Err("unsupported_taskflow_task_bridge".to_string()),
    }
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
            protocol_id: "bridge.instruction-activation-protocol",
            source_path:
                "vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md",
            activation_class: "always_on",
            runtime_owner: "vida::taskflow::protocol_binding::activation_bridge",
            enforcement_type: "activation-resolution",
            proof_surface: "vida docflow activation-check --profile active-canon",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "work.taskflow-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.taskflow-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::taskflow::protocol_binding::taskflow_surface",
            enforcement_type: "execution-discipline",
            proof_surface: "vida taskflow consume bundle check --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime.task-state-telemetry-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::state_store::task_state_telemetry",
            enforcement_type: "state-telemetry",
            proof_surface: "vida status --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "work.execution-health-check-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::doctor::execution_health",
            enforcement_type: "health-gate",
            proof_surface: "vida taskflow doctor --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "work.task-state-reconciliation-protocol",
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
        if !protocol_index.contains(seed.source_path) {
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
                        "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to taskflow-v0."
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
        Some("protocol-binding")
    ) {
        return run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        if matches!(
            args.args.get(1).map(String::as_str),
            None | Some("bundle" | "final" | "--help" | "-h")
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
        "Unsupported `vida taskflow {subcommand}` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to taskflow-v0."
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

    if current_docflow_launcher_mode() == DocflowLauncherMode::InstalledCompatibility
        && !installed_docflow_command_allowed(&args.args)
    {
        eprintln!(
            "Installed `vida docflow` is a compatibility wrapper with `help|overview only`. \
Repo/dev binaries keep the full in-process Rust DocFlow shell."
        );
        return ExitCode::from(2);
    }

    let argv = std::iter::once("docflow".to_string())
        .chain(args.args.clone())
        .collect::<Vec<_>>();

    match DocflowCli::try_parse_from(argv.clone()) {
        Ok(cli) => {
            println!("{}", docflow_cli::run(cli));
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
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida orchestrator-init",
                                "init": bundle.orchestrator_init_view,
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
                            bundle.orchestrator_init_view["status"]
                                .as_str()
                                .unwrap_or("unknown"),
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

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": "vida agent-init",
                        "init": bundle.agent_init_view,
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
                    bundle.agent_init_view["status"]
                        .as_str()
                        .unwrap_or("unknown"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "selected role",
                    selection["selected_role"].as_str().unwrap_or("unknown"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "fallback surface",
                    bundle.agent_init_view["source_mode_fallback_surface"]
                        .as_str()
                        .unwrap_or(""),
                );
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
    run_boot(args).await
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
                            "latest_run_graph_status": latest_run_graph_status,
                            "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                            "latest_run_graph_recovery": latest_run_graph_recovery,
                            "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                            "latest_run_graph_gate": latest_run_graph_gate,
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
                    "vida={}, project_root={}, taskflow_runtime={}",
                    launcher_runtime_paths.vida,
                    launcher_runtime_paths.project_root,
                    launcher_runtime_paths.taskflow_runtime
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
    taskflow_runtime: String,
}

fn doctor_launcher_summary_json() -> Result<DoctorLauncherSummary, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let project_root = resolve_repo_root()?;
    let taskflow_binary = resolve_taskflow_binary()?;
    Ok(DoctorLauncherSummary {
        vida: current_exe.display().to_string(),
        project_root: project_root.display().to_string(),
        taskflow_runtime: taskflow_binary.display().to_string(),
    })
}

fn doctor_launcher_summary_for_root(project_root: &Path) -> Result<DoctorLauncherSummary, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let taskflow_binary = resolve_taskflow_binary()?;
    Ok(DoctorLauncherSummary {
        vida: current_exe.display().to_string(),
        project_root: project_root.display().to_string(),
        taskflow_runtime: taskflow_binary.display().to_string(),
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

#[derive(Debug, serde::Serialize)]
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
    agent_system: &serde_json::Value,
    route_id: &str,
) -> serde_json::Value {
    let Some(route) = json_lookup(agent_system, &["routing", route_id]) else {
        return serde_json::Value::Null;
    };
    serde_json::json!({
        "route_id": route_id,
        "subagents": json_string(json_lookup(route, &["subagents"])).unwrap_or_default(),
        "fanout_subagents": json_string(json_lookup(route, &["fanout_subagents"])).unwrap_or_default(),
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

pub(crate) fn build_runtime_execution_plan_from_snapshot(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let agent_system = &compiled_bundle["agent_system"];
    let implementation = summarize_agent_route_from_snapshot(agent_system, "implementation");
    let coach_route_id = implementation["coach_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("coach");
    let verification_route_id = implementation["verification_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("verification");
    serde_json::json!({
        "system_mode": json_string(json_lookup(agent_system, &["mode"])).unwrap_or_default(),
        "state_owner": json_string(json_lookup(agent_system, &["state_owner"])).unwrap_or_default(),
        "max_parallel_agents": json_lookup(agent_system, &["max_parallel_agents"]).cloned().unwrap_or(serde_json::Value::Null),
        "default_route": summarize_agent_route_from_snapshot(agent_system, "default"),
        "conversation_stage": {
            "selected_role": selection.selected_role,
            "conversational_mode": selection.conversational_mode,
            "tracked_flow_entry": selection.tracked_flow_entry,
            "allow_freeform_chat": selection.allow_freeform_chat,
            "single_task_only": selection.single_task_only,
        },
        "development_flow": {
            "implementation": implementation,
            "coach": summarize_agent_route_from_snapshot(agent_system, coach_route_id),
            "verification": summarize_agent_route_from_snapshot(agent_system, verification_route_id),
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

fn build_compiled_agent_extension_bundle_for_root(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<serde_json::Value, String> {
    let enabled_project_roles = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_roles"],
    ));
    let enabled_project_skills = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_skills"],
    ));
    let enabled_project_profiles = yaml_string_list(yaml_lookup(
        config,
        &["agent_extensions", "enabled_project_profiles"],
    ));
    let enabled_project_flows = yaml_string_list(yaml_lookup(
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
    let mut validation_errors = Vec::new();
    let roles_registry = match roles_path.as_deref() {
        Some(path) => match read_yaml_file_checked(&resolve_overlay_path(root, path)) {
            Ok(value) => value,
            Err(error) => {
                if require_registry_files {
                    validation_errors.push(error);
                }
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !enabled_project_roles.is_empty() {
                validation_errors.push(
                    "agent extension roles registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let skills_registry = match skills_path.as_deref() {
        Some(path) => match read_yaml_file_checked(&resolve_overlay_path(root, path)) {
            Ok(value) => value,
            Err(error) => {
                if require_registry_files {
                    validation_errors.push(error);
                }
                serde_yaml::Value::Null
            }
        },
        None => serde_yaml::Value::Null,
    };
    let profiles_registry = match profiles_path.as_deref() {
        Some(path) => match read_yaml_file_checked(&resolve_overlay_path(root, path)) {
            Ok(value) => value,
            Err(error) => {
                if require_registry_files {
                    validation_errors.push(error);
                }
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !enabled_project_profiles.is_empty() {
                validation_errors.push(
                    "agent extension profiles registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let flows_registry = match flows_path.as_deref() {
        Some(path) => match read_yaml_file_checked(&resolve_overlay_path(root, path)) {
            Ok(value) => value,
            Err(error) => {
                if require_registry_files {
                    validation_errors.push(error);
                }
                serde_yaml::Value::Null
            }
        },
        None => {
            if require_registry_files && !enabled_project_flows.is_empty() {
                validation_errors.push(
                    "agent extension flows registry path is required but missing".to_string(),
                );
            }
            serde_yaml::Value::Null
        }
    };
    let codex_root = root.join(".codex");
    let codex_config = read_simple_toml_sections(&codex_root.join("config.toml"));
    let codex_roles = codex_config
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
            Some(serde_json::json!({
                "role_id": role_id,
                "description": values.get("description").cloned().unwrap_or_default(),
                "config_file": config_file,
                "model": role_config.get("model").cloned().unwrap_or_default(),
                "model_reasoning_effort": role_config.get("model_reasoning_effort").cloned().unwrap_or_default(),
                "sandbox_mode": role_config.get("sandbox_mode").cloned().unwrap_or_default(),
            }))
        })
        .collect::<Vec<_>>();

    let bundle = serde_json::json!({
        "ok": true,
        "enabled": yaml_bool(yaml_lookup(config, &["agent_extensions", "enabled"]), false),
        "map_doc": yaml_string(yaml_lookup(config, &["agent_extensions", "map_doc"])).unwrap_or_default(),
        "enabled_framework_roles": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_framework_roles"])),
        "enabled_standard_flow_sets": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_standard_flow_sets"])),
        "enabled_shared_skills": yaml_string_list(yaml_lookup(config, &["agent_extensions", "enabled_shared_skills"])),
        "default_flow_set": yaml_string(yaml_lookup(config, &["agent_extensions", "default_flow_set"])).unwrap_or_default(),
        "project_roles": registry_rows_by_key(&roles_registry, "roles", "role_id", &enabled_project_roles),
        "project_skills": registry_rows_by_key(&skills_registry, "skills", "skill_id", &enabled_project_skills),
        "project_profiles": registry_rows_by_key(&profiles_registry, "profiles", "profile_id", &enabled_project_profiles),
        "project_flows": registry_rows_by_key(&flows_registry, "flow_sets", "flow_id", &enabled_project_flows),
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
        }),
    });
    let readiness_output = docflow_cli::run(DocflowCli {
        command: DocflowCommand::ReadinessCheck(DocflowCheckArgs {
            root: None,
            profile: "active-canon".to_string(),
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
        artifact_path: Some("vida/config/codex-readiness.current.jsonl".to_string()),
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
                "artifact_path": "vida/config/codex-readiness.current.jsonl",
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
                        let closure_admission =
                            build_runtime_closure_admission(&bundle_check, &docflow_verdict);
                        let payload = TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: time::OffsetDateTime::now_utc()
                                .format(&Rfc3339)
                                .expect("rfc3339 timestamp should render"),
                            closure_authority: "taskflow".to_string(),
                            role_selection: match build_runtime_lane_selection_with_store(
                                &store,
                                &request_text,
                            )
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
                                            closure_admission,
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
                            },
                            request_text,
                            direct_consumption_ready: bundle_check.ok && docflow_verdict.ready,
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
                            let closure_admission =
                                build_runtime_closure_admission(&bundle_check, &docflow_verdict);
                            let role_selection = blocking_lane_selection(&request_text, &error);
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
#[command(name = "vida", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init(BootArgs),
    Boot(BootArgs),
    OrchestratorInit(InitArgs),
    AgentInit(AgentInitArgs),
    Task(TaskArgs),
    Memory(MemoryArgs),
    Status(StatusArgs),
    Doctor(DoctorArgs),
    Taskflow(ProxyArgs),
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
struct AgentInitArgs {
    request_text: Option<String>,

    #[arg(long = "role")]
    role: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    state_dir: Option<PathBuf>,

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
    println!("  init      compatibility root bootstrap surface");
    println!(
        "  boot      initialize authoritative state and instruction/framework-memory surfaces"
    );
    println!("  orchestrator-init  render the compiled startup view for the orchestrator lane");
    println!("  agent-init         render the bounded startup view for a worker/agent lane");
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
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
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
        assert!(init_index < boot_index, "init should appear before boot in help");
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
            artifact_path: Some("vida/config/codex-readiness.current.jsonl".to_string()),
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
            artifact_path: Some("vida/config/codex-readiness.current.jsonl".to_string()),
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

        let admission = build_runtime_closure_admission(&bundle_check, &docflow_verdict);

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

        let admission = build_runtime_closure_admission(&bundle_check, &docflow_verdict);

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
}
