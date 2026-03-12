mod state_store;
mod temp_state;

use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::process::ExitCode;
use std::{collections::HashSet, fs};
use std::time::SystemTime;

use clap::{Args, CommandFactory, Parser, Subcommand};
use docflow_cli::{
    CheckArgs as DocflowCheckArgs, Cli as DocflowCli, Command as DocflowCommand,
    ProofcheckArgs as DocflowProofcheckArgs,
};
use state_store::{
    BlockedTaskRecord, RunGraphStatus, StateStore, StateStoreError, TaskCriticalPath,
    TaskDependencyRecord, TaskDependencyStatus, TaskDependencyTreeEdge, TaskDependencyTreeNode,
    TaskGraphIssue, TaskRecord,
};
use time::format_description::well_known::Rfc3339;

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
        Some(Command::Boot(args)) => run_boot(args).await,
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

fn print_taskflow_proxy_help(topic: Option<&str>) {
    match topic {
        Some("task") => {
            println!("VIDA TaskFlow help: task");
            println!();
            println!("Purpose:");
            println!("  Inspect and mutate the primary backlog through the TaskFlow runtime store.");
            println!("  JSONL is import/export compatibility only; it is not the live source of truth.");
            println!();
            println!("Source of truth:");
            println!("  Runtime store: taskflow-v0 task over the authoritative state store.");
            println!("  Fallback/export only: .beads/issues.jsonl");
            println!();
            println!("Dependency semantics:");
            println!("  Parent-child edges preserve epic/task structure.");
            println!("  Blocks edges preserve readiness and execution ordering.");
            println!("  `task ready` returns the current unblocked ready set from the runtime store.");
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow task list --all --json");
            println!("  vida taskflow task ready --json");
            println!("  vida taskflow task show <task-id> --json");
            println!("  vida taskflow task update <task-id> --status in_progress --notes \"...\" --json");
            println!("  vida taskflow task close <task-id> --reason \"...\" --json");
            println!();
            println!("Failure modes:");
            println!("  Missing or ambiguous runtime root fails closed.");
            println!("  Invalid task ids or illegal status transitions fail closed from the delegated runtime.");
            println!("  Export artifacts can drift; verify live state with `task show` or `task list`.");
            println!();
            println!("Operator recipes:");
            println!("  Check the next lawful slice: vida taskflow task ready --json");
            println!("  Inspect one task before mutation: vida taskflow task show <task-id> --json");
            println!("  Record real progress after a proven step: vida taskflow task update <task-id> --status <status> --notes \"...\" --json");
            return;
        }
        Some("consume") => {
            println!("VIDA TaskFlow help: consume");
            println!();
            println!("Purpose:");
            println!("  Inspect the bounded TaskFlow runtime-consumption bundle and enter the final closure handoff seam.");
            println!("  Bundle inspection and final closure loop are launcher-owned and in-process over authoritative Rust state plus the bounded DocFlow branch.");
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow consume bundle [--json]");
            println!("  vida taskflow consume bundle check [--json]");
            println!("  vida taskflow consume final \"<request>\" --json");
            println!();
            println!("Failure modes:");
            println!("  `bundle` requires a booted authoritative state root and fails closed if runtime bundle surfaces are missing.");
            println!("  Unsupported consume modes fail closed.");
            println!("  `final` fails closed when the runtime bundle is not ready or the bounded DocFlow evidence branch returns blocking results.");
            println!();
            println!("Operator recipes:");
            println!("  Verify the active runtime bundle before closure packaging: vida taskflow consume bundle check --json");
            println!("  Use `consume final` only when the current implementation/proof slice is complete and ready for final closure packaging.");
            return;
        }
        Some("run-graph") => {
            println!("VIDA TaskFlow help: run-graph");
            println!();
            println!("Purpose:");
            println!("  Create and inspect node-level execution state for one routed task run.");
            println!("  Run-graph is not a second task queue; it complements task lifecycle state.");
            println!("  The current run-graph surface is launcher-owned and in-process for both mutation and inspection.");
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow run-graph init <task_id> <task_class> [route_task_class]");
            println!("  vida taskflow run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]");
            println!("  vida taskflow run-graph status <task_id>");
            println!("  vida taskflow run-graph latest [--json]");
            println!();
            println!("Failure modes:");
            println!("  Invalid JSON in meta_json fails closed before mutation.");
            println!("  `latest` returns `none`/`null` when no routed run has been recorded yet.");
            println!("  Run-graph state must not be treated as backlog readiness authority.");
            return;
        }
        Some("recovery") => {
            println!("VIDA TaskFlow help: recovery");
            println!();
            println!("Purpose:");
            println!("  Inspect donor-aligned resumability state derived from the authoritative Rust run-graph contract.");
            println!("  Recovery status is a read-only launcher-owned inspection surface.");
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow recovery status <run-id> [--json]");
            println!("  vida taskflow recovery latest [--json]");
            println!("  vida taskflow recovery checkpoint <run-id> [--json]");
            println!("  vida taskflow recovery checkpoint-latest [--json]");
            println!("  vida taskflow recovery gate <run-id> [--json]");
            println!("  vida taskflow recovery gate-latest [--json]");
            println!();
            println!("Returned semantics:");
            println!("  resume_node, resume_status, checkpoint_kind, resume_target, policy_gate, handoff_state, recovery_ready");
            println!();
            println!("Failure modes:");
            println!("  Missing run ids fail closed from the authoritative state store.");
            println!("  `latest` returns `none`/`null` when no routed run has been recorded yet.");
            println!("  Recovery state must not be treated as backlog readiness authority.");
            return;
        }
        Some("doctor") => {
            println!("VIDA TaskFlow help: doctor");
            println!();
            println!("Purpose:");
            println!("  Diagnose launcher/runtime health for bootstrap, task-store visibility, and graph integrity.");
            println!();
            println!("Canonical command:");
            println!("  vida doctor");
            println!();
            println!("Checks currently surfaced:");
            println!("  storage metadata");
            println!("  authoritative state spine");
            println!("  task store summary");
            println!("  run graph summary");
            println!("  boot compatibility, migration preflight, and effective bundle integrity");
            println!();
            println!("Failure modes:");
            println!("  Broken state roots, incompatible migration posture, or missing runtime artifacts fail closed.");
            return;
        }
        Some(_) => {}
        None => {}
    }

    println!("VIDA TaskFlow runtime family");
    println!();
    println!("Usage:");
    println!("  vida taskflow <args...>");
    println!("  vida taskflow help [task|consume|run-graph|recovery|doctor]");
    println!("  vida taskflow <command> --help");
    println!();
    println!("Purpose:");
    println!("  Enter the TaskFlow runtime family for tracked execution, backlog state, run-graph state, and closure handoff.");
    println!();
    println!("Source of truth notes:");
    println!("  TaskFlow is the execution/runtime authority.");
    println!("  `taskflow-v0 task` is the primary backlog store during the bridge.");
    println!("  `.beads/issues.jsonl` remains fallback/export only, not the live runtime store.");
    println!();
    println!("Runtime routing:");
    println!("  In a project tree, vida resolves the root from the current working directory without manual VIDA_ROOT export.");
    println!("  In repo mode the delegated runtime resolves to taskflow-v0/src/vida.");
    println!("  In installed mode it resolves the sibling taskflow binary from the active vida bin root.");
    println!("  Unknown roots or missing binaries fail closed.");
    println!();
    println!("Most-used command homes:");
    println!("  task        backlog inspection and mutation");
    println!("  run-graph   resumability and node-state inspection");
    println!("  consume     explicit TaskFlow -> final closure handoff");
    println!();
    println!("Canonical examples:");
    println!("  vida taskflow task ready --json");
    println!("  vida taskflow task show <task-id> --json");
    println!("  vida taskflow run-graph status <task-id>");
    println!("  vida taskflow consume final \"proof path\" --json");
    println!();
    println!("Operator recipes:");
    println!("  Find the next lawful slice: vida taskflow task ready --json");
    println!("  Inspect one tracked item: vida taskflow help task");
    println!("  Inspect resumability state: vida taskflow help run-graph");
    println!("  Review runtime diagnostics: vida taskflow help doctor");
    println!();
    println!("Failure modes:");
    println!("  Missing runtime family binary, ambiguous root, and unsupported delegated arguments fail closed.");
    println!("  Use topic help to inspect command contracts before mutating runtime state.");
}

fn taskflow_help_topic(args: &[String]) -> Option<Option<&str>> {
    match args {
        [] => Some(None),
        [head] if matches!(head.as_str(), "help" | "--help" | "-h") => Some(None),
        [head, topic, ..] if head == "help" => Some(Some(topic.as_str())),
        [command, flag, ..] if matches!(flag.as_str(), "--help" | "-h") => Some(Some(command.as_str())),
        _ => None,
    }
}

struct TaskflowQueryAnswer<'a> {
    intent: &'a str,
    why: &'a str,
    command: &'a str,
    failure_modes: &'a str,
}

fn taskflow_query_answer(query: &str) -> TaskflowQueryAnswer<'static> {
    let normalized = query.to_ascii_lowercase();
    if normalized.contains("next")
        || normalized.contains("ready")
        || normalized.contains("what should i run")
        || normalized.contains("what do i run")
    {
        return TaskflowQueryAnswer {
            intent: "next-ready-slice",
            why: "TaskFlow readiness is the canonical way to pick the next unblocked execution slice.",
            command: "vida taskflow task ready --json",
            failure_modes: "Ready output depends on current runtime state; blocked or stale exported artifacts must be checked through the runtime store.",
        };
    }

    if normalized.contains("latest")
        && (normalized.contains("run-graph")
            || normalized.contains("run graph")
            || normalized.contains("recovery"))
    {
        return TaskflowQueryAnswer {
            intent: "inspect-latest-resumability",
            why: "Latest run-graph and recovery inspection surfaces are the canonical launcher-owned summaries for the most recent routed run.",
            command: "vida taskflow recovery latest --json",
            failure_modes: "Latest recovery inspection returns null when no routed run exists yet and must not be treated as backlog readiness authority.",
        };
    }

    if normalized.contains("gate") {
        return TaskflowQueryAnswer {
            intent: "inspect-gate",
            why: "Gate inspection is the bounded recovery projection for policy gate, handoff state, and context state on one routed run.",
            command: "vida taskflow recovery gate <run-id> --json",
            failure_modes: "Gate inspection must not be treated as backlog readiness authority, and missing run ids fail closed.",
        };
    }

    if normalized.contains("show")
        || normalized.contains("inspect")
        || normalized.contains("task id")
        || normalized.contains("one task")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-task",
            why: "Task inspection should read one canonical record from the runtime store before mutation.",
            command: "vida taskflow task show <task-id> --json",
            failure_modes: "Unknown task ids fail closed in the delegated runtime.",
        };
    }

    if normalized.contains("update") || normalized.contains("progress") || normalized.contains("status") {
        return TaskflowQueryAnswer {
            intent: "record-progress",
            why: "Progress should be recorded against the primary backlog store after a proven runtime or documentation step.",
            command: "vida taskflow task update <task-id> --status in_progress --notes \"...\" --json",
            failure_modes: "Illegal status transitions or missing task ids fail closed in the delegated runtime.",
        };
    }

    if normalized.contains("close") || normalized.contains("done") || normalized.contains("completed") {
        return TaskflowQueryAnswer {
            intent: "close-task",
            why: "Closure should happen only after proof/doc sync confirms the slice is complete.",
            command: "vida taskflow task close <task-id> --reason \"...\" --json",
            failure_modes: "Closing the wrong task mutates the primary backlog; inspect the task first if the identifier is uncertain.",
        };
    }

    if normalized.contains("resume")
        || normalized.contains("resum")
        || normalized.contains("run-graph")
        || normalized.contains("run graph")
        || normalized.contains("recovery")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-resumability",
            why: "Run-graph and recovery state are the canonical node-level resumability surfaces for one routed execution run.",
            command: "vida taskflow recovery status <run-id> --json",
            failure_modes: "Recovery inspection must not be treated as backlog readiness authority, and missing run ids fail closed.",
        };
    }

    if normalized.contains("checkpoint") {
        return TaskflowQueryAnswer {
            intent: "inspect-checkpoint",
            why: "Checkpoint state is the bounded recovery projection for resume target and checkpoint kind on one routed run.",
            command: "vida taskflow recovery checkpoint <run-id> --json",
            failure_modes: "Checkpoint inspection must not be treated as backlog readiness authority, and missing run ids fail closed.",
        };
    }

    if normalized.contains("doctor")
        || normalized.contains("diagnose")
        || normalized.contains("health")
        || normalized.contains("broken")
    {
        return TaskflowQueryAnswer {
            intent: "diagnose-runtime",
            why: "Launcher/runtime health should be checked through the fail-closed doctor surface before further mutation.",
            command: "vida doctor",
            failure_modes: "Doctor reports the current local runtime state only; incompatible boot/migration posture must be resolved before continuing.",
        };
    }

    if normalized.contains("final")
        || normalized.contains("consume")
        || normalized.contains("closure")
        || normalized.contains("handoff")
    {
        return TaskflowQueryAnswer {
            intent: "closure-handoff",
            why: "Direct consumption is the explicit TaskFlow-to-closure bridge when implementation and proof are already complete.",
            command: "vida taskflow consume final \"<request>\" --json",
            failure_modes: "Use only at closure time; final consumption now fails closed when the runtime bundle is not ready or the bounded DocFlow evidence branch returns blocking results.",
        };
    }

    TaskflowQueryAnswer {
        intent: "help-fallback",
        why: "No confident workflow match was found, so the safest bounded answer is the canonical help surface.",
        command: "vida taskflow help",
        failure_modes: "If the query is too vague, inspect topic help first and then rerun a more specific query.",
    }
}

fn print_taskflow_query_help() {
    println!("VIDA TaskFlow query");
    println!();
    println!("Purpose:");
    println!("  Answer common operator workflow questions with one bounded recommended TaskFlow command.");
    println!("  The query surface is deterministic and launcher-owned; it does not call models or external tools.");
    println!();
    println!("Usage:");
    println!("  vida taskflow query \"what should I run next?\"");
    println!("  vida taskflow query \"how do I inspect one task?\"");
    println!("  vida taskflow query \"how do I check resumability?\"");
    println!();
    println!("Current intents:");
    println!("  next/ready, inspect/show, update/progress, close/done, resume/run-graph, doctor/health, final/consume");
    println!();
    println!("Failure modes:");
    println!("  Vague queries fall back to `vida taskflow help`.");
}

fn run_taskflow_query(args: &[String]) -> ExitCode {
    match args {
        [head] if matches!(head.as_str(), "query") => {
            print_taskflow_query_help();
            ExitCode::SUCCESS
        }
        [head, flag] if head == "query" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_query_help();
            ExitCode::SUCCESS
        }
        [head, query @ ..] if head == "query" => {
            let joined = query.join(" ");
            let answer = taskflow_query_answer(&joined);
            println!("VIDA TaskFlow query answer");
            println!();
            println!("Query:");
            println!("  {joined}");
            println!("Intent:");
            println!("  {}", answer.intent);
            println!("Why:");
            println!("  {}", answer.why);
            println!("Recommended command:");
            println!("  {}", answer.command);
            println!("Failure modes:");
            println!("  {}", answer.failure_modes);
            ExitCode::SUCCESS
        }
        _ => ExitCode::from(2),
    }
}

fn print_docflow_proxy_help() {
    println!("VIDA DocFlow runtime family");
    println!();
    println!("Behavior:");
    println!("  vida routes the active DocFlow command map in-process through the Rust CLI.");
    println!("  Unsupported commands fail closed instead of silently falling through to donor wrappers.");
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
    path.join("AGENTS.md").is_file() && path.join("vida/root-map.md").is_file()
}

fn resolve_repo_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("VIDA_ROOT") {
        let root = PathBuf::from(root);
        if root.exists() {
            return Ok(root);
        }
        return Err(format!("VIDA_ROOT points to a missing path: {}", root.display()));
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

    first_existing_path(&candidates)
        .ok_or_else(|| format!("Unable to resolve taskflow runtime for project root {}", root.display()))
}

fn run_proxy(program: &Path, args: &[String], envs: &[(&str, &str)]) -> ExitCode {
    let mut command = ProcessCommand::new(program);
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }

    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("Failed to execute {}: {error}", program.display());
            ExitCode::from(1)
        }
    }
}

fn proxy_state_dir() -> PathBuf {
    std::env::var_os("VIDA_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(state_store::default_state_dir)
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

fn default_run_graph_status(task_id: &str, task_class: &str, route_task_class: &str) -> RunGraphStatus {
    RunGraphStatus {
        run_id: task_id.to_string(),
        task_id: task_id.to_string(),
        task_class: task_class.to_string(),
        active_node: task_class.to_string(),
        next_node: None,
        status: "pending".to_string(),
        route_task_class: route_task_class.to_string(),
        selected_backend: "unknown".to_string(),
        lane_id: "unassigned".to_string(),
        lifecycle_stage: "initialized".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "none".to_string(),
        context_state: "open".to_string(),
        checkpoint_kind: "none".to_string(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    }
}

fn merge_run_graph_meta(mut status: RunGraphStatus, meta: &serde_json::Value) -> RunGraphStatus {
    status.next_node = json_string_field(meta, "next_node").or(status.next_node);
    status.selected_backend =
        json_string_field(meta, "selected_backend").unwrap_or(status.selected_backend);
    status.lane_id = json_string_field(meta, "lane_id").unwrap_or(status.lane_id);
    status.lifecycle_stage =
        json_string_field(meta, "lifecycle_stage").unwrap_or(status.lifecycle_stage);
    status.policy_gate = json_string_field(meta, "policy_gate").unwrap_or(status.policy_gate);
    status.handoff_state = json_string_field(meta, "handoff_state").unwrap_or(status.handoff_state);
    status.context_state = json_string_field(meta, "context_state").unwrap_or(status.context_state);
    status.checkpoint_kind =
        json_string_field(meta, "checkpoint_kind").unwrap_or(status.checkpoint_kind);
    status.resume_target = json_string_field(meta, "resume_target").unwrap_or(status.resume_target);
    status.recovery_ready = json_bool_field(meta, "recovery_ready").unwrap_or(status.recovery_ready);
    status
}

async fn run_taskflow_run_graph_mutation(args: &[String]) -> ExitCode {
    let state_dir = proxy_state_dir();
    let store = match StateStore::open(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match args {
        [head, subcommand, task_id, task_class] if head == "run-graph" && subcommand == "init" => {
            let status = default_run_graph_status(task_id, task_class, task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store.root().join("run-graph").join(format!("{task_id}.json")).display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, route_task_class]
            if head == "run-graph" && subcommand == "init" =>
        {
            let status = default_run_graph_status(task_id, task_class, route_task_class);
            match store.record_run_graph_status(&status).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store.root().join("run-graph").join(format!("{task_id}.json")).display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to initialize run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: existing.route_task_class,
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store.root().join("run-graph").join(format!("{task_id}.json")).display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class]
            if head == "run-graph" && subcommand == "update" =>
        {
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = RunGraphStatus {
                run_id: task_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: node.to_string(),
                next_node: existing.next_node,
                status: status.to_string(),
                route_task_class: route_task_class.to_string(),
                selected_backend: existing.selected_backend,
                lane_id: existing.lane_id,
                lifecycle_stage: existing.lifecycle_stage,
                policy_gate: existing.policy_gate,
                handoff_state: existing.handoff_state,
                context_state: existing.context_state,
                checkpoint_kind: existing.checkpoint_kind,
                resume_target: existing.resume_target,
                recovery_ready: existing.recovery_ready,
            };
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store.root().join("run-graph").join(format!("{task_id}.json")).display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, task_id, task_class, node, status, route_task_class, meta_json]
            if head == "run-graph" && subcommand == "update" =>
        {
            let meta: serde_json::Value = match serde_json::from_str(meta_json) {
                Ok(meta) => meta,
                Err(error) => {
                    eprintln!("[run-graph] meta_json must be valid JSON: {error}");
                    return ExitCode::from(2);
                }
            };
            let existing = match store.run_graph_status(task_id).await {
                Ok(existing) => existing,
                Err(StateStoreError::MissingTask { .. }) => {
                    default_run_graph_status(task_id, task_class, route_task_class)
                }
                Err(error) => {
                    eprintln!("Failed to read existing run-graph state: {error}");
                    return ExitCode::from(1);
                }
            };
            let merged = merge_run_graph_meta(
                RunGraphStatus {
                    run_id: task_id.to_string(),
                    task_id: task_id.to_string(),
                    task_class: task_class.to_string(),
                    active_node: node.to_string(),
                    next_node: existing.next_node,
                    status: status.to_string(),
                    route_task_class: route_task_class.to_string(),
                    selected_backend: existing.selected_backend,
                    lane_id: existing.lane_id,
                    lifecycle_stage: existing.lifecycle_stage,
                    policy_gate: existing.policy_gate,
                    handoff_state: existing.handoff_state,
                    context_state: existing.context_state,
                    checkpoint_kind: existing.checkpoint_kind,
                    resume_target: existing.resume_target,
                    recovery_ready: existing.recovery_ready,
                },
                &meta,
            );
            match store.record_run_graph_status(&merged).await {
                Ok(()) => {
                    println!(
                        "{}",
                        store.root().join("run-graph").join(format!("{task_id}.json")).display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to update run-graph state: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "init" => {
            eprintln!("Usage: vida taskflow run-graph init <task_id> <task_class> [route_task_class]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "update" => {
            eprintln!("Usage: vida taskflow run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

async fn open_existing_state_store_with_retry(state_dir: PathBuf) -> Result<StateStore, StateStoreError> {
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

async fn run_taskflow_recovery(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "recovery" => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "recovery" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("recovery"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "recovery" && subcommand == "gate-latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery gate-latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery gate-latest");
                        print_surface_line(RenderMode::Plain, "gate", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
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
            if head == "recovery" && subcommand == "gate-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_gate_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate-latest",
                                "gate": summary,
                            }))
                            .expect("latest gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "gate" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery gate");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "gate", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "gate" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_gate_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery gate",
                                "run_id": summary.run_id,
                                "gate": summary,
                            }))
                            .expect("gate summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read gate summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "checkpoint-latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery checkpoint-latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery checkpoint-latest");
                        print_surface_line(RenderMode::Plain, "checkpoint", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
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
            if head == "recovery" && subcommand == "checkpoint-latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint-latest",
                                "checkpoint": summary,
                            }))
                            .expect("latest checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "checkpoint" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery checkpoint");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "checkpoint", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "checkpoint" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_checkpoint_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery checkpoint",
                                "run_id": summary.run_id,
                                "checkpoint": summary,
                            }))
                            .expect("checkpoint summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read checkpoint summary: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "recovery" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
                    Ok(Some(summary)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery latest");
                        print_surface_line(RenderMode::Plain, "recovery", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest recovery status: {error}");
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
            if head == "recovery" && subcommand == "latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_recovery_summary().await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery latest",
                                "recovery": summary,
                            }))
                            .expect("latest recovery summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "recovery" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow recovery status");
                        print_surface_line(RenderMode::Plain, "run", &summary.run_id);
                        print_surface_line(RenderMode::Plain, "recovery", &summary.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "recovery" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_recovery_summary(run_id).await {
                    Ok(summary) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow recovery status",
                                "run_id": summary.run_id,
                                "recovery": summary,
                            }))
                            .expect("recovery summary should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read recovery status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate-latest" => {
            eprintln!("Usage: vida taskflow recovery gate-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "gate" => {
            eprintln!("Usage: vida taskflow recovery gate <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint-latest" => {
            eprintln!("Usage: vida taskflow recovery checkpoint-latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "checkpoint" => {
            eprintln!("Usage: vida taskflow recovery checkpoint <run-id> [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow recovery latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "recovery" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow recovery status <run-id> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

async fn run_taskflow_run_graph(args: &[String]) -> ExitCode {
    match args {
        [head, flag] if head == "run-graph" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("run-graph"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "run-graph" && subcommand == "latest" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(Some(status)) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph latest");
                        print_surface_line(RenderMode::Plain, "status", "none");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
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
            if head == "run-graph" && subcommand == "latest" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.latest_run_graph_status().await {
                    Ok(status) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph latest",
                                "status": status,
                            }))
                            .expect("latest run-graph status should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id] if head == "run-graph" && subcommand == "status" => {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        print_surface_header(RenderMode::Plain, "vida taskflow run-graph status");
                        print_surface_line(RenderMode::Plain, "run", &status.run_id);
                        print_surface_line(RenderMode::Plain, "status", &status.as_display());
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, run_id, flag]
            if head == "run-graph" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = proxy_state_dir();
            match open_existing_state_store_with_retry(state_dir).await {
                Ok(store) => match store.run_graph_status(run_id).await {
                    Ok(status) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida taskflow run-graph status",
                                "run_id": status.run_id,
                                "status": status,
                            }))
                            .expect("run-graph status should render as json")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read run-graph status: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "latest" => {
            eprintln!("Usage: vida taskflow run-graph latest [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "run-graph" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow run-graph status <run-id> [--json]");
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
        if matches!(args.args.get(1).map(String::as_str), Some("init" | "update")) {
            return run_taskflow_run_graph_mutation(&args.args).await;
        }
    }

    match resolve_taskflow_binary() {
        Ok(binary) => match resolve_repo_root() {
            Ok(root) => {
                let root_display = root.display().to_string();
                run_proxy(&binary, &args.args, &[("VIDA_ROOT", &root_display)])
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
        TaskCommand::Dep(command) => {
            match command.command {
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
            }
        }
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
                let effective_bundle_receipt = match store.latest_effective_bundle_receipt_summary().await {
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
                let task_reconciliation_rollup =
                    match store.task_reconciliation_rollup().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read task reconciliation rollup: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let snapshot_bridge =
                    match store.taskflow_snapshot_bridge_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read taskflow snapshot bridge summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let runtime_consumption =
                    match runtime_consumption_summary(store.root()) {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read runtime-consumption summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_status =
                    match store.latest_run_graph_status().await {
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
                let latest_run_graph_checkpoint =
                    match store.latest_run_graph_checkpoint_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph checkpoint summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                let latest_run_graph_gate =
                    match store.latest_run_graph_gate_summary().await {
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
                            "latest_run_graph_status": latest_run_graph_status,
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
                print_surface_line(render, "migration receipts", &migration_receipts.as_display());
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
                match latest_run_graph_status {
                    Some(status) => {
                        print_surface_line(
                            render,
                            "latest run graph status",
                            &status.as_display(),
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
                        print_surface_line(
                            render,
                            "latest run graph gate",
                            &summary.as_display(),
                        );
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
            let latest_task_reconciliation = match store.latest_task_reconciliation_summary().await {
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
                Ok(root_artifact_id) => match store.inspect_effective_instruction_bundle(&root_artifact_id).await {
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
                        "latest_run_graph_status": latest_run_graph_status,
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
            print_surface_ok(render, "migration receipts", &migration_receipts.as_display());
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
            match latest_run_graph_status {
                Some(status) => {
                    print_surface_ok(render, "latest run graph status", &status.as_display());
                }
                None => {
                    print_surface_ok(render, "latest run graph status", "none");
                }
            }
            match latest_run_graph_recovery {
                Some(summary) => {
                    print_surface_ok(
                        render,
                        "latest run graph recovery",
                        &summary.as_display(),
                    );
                }
                None => {
                    print_surface_ok(render, "latest run graph recovery", "none");
                }
            }
            match latest_run_graph_checkpoint {
                Some(summary) => {
                    print_surface_ok(
                        render,
                        "latest run graph checkpoint",
                        &summary.as_display(),
                    );
                }
                None => {
                    print_surface_ok(render, "latest run graph checkpoint", "none");
                }
            }
            match latest_run_graph_gate {
                Some(summary) => {
                    print_surface_ok(
                        render,
                        "latest run graph gate",
                        &summary.as_display(),
                    );
                }
                None => {
                    print_surface_ok(render, "latest run graph gate", "none");
                }
            }
            print_surface_ok(
                render,
                "effective instruction bundle",
                &effective_instruction_bundle.mandatory_chain_order.join(" -> "),
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

#[derive(Debug, serde::Serialize)]
struct TaskflowConsumeBundlePayload {
    artifact_name: String,
    artifact_type: String,
    generated_at: String,
    vida_root: String,
    config_path: String,
    launcher_runtime_paths: DoctorLauncherSummary,
    effective_instruction_bundle: serde_json::Value,
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
struct RuntimeConsumptionLaneSelection {
    ok: bool,
    selection_mode: String,
    fallback_role: String,
    request: String,
    selected_role: String,
    conversational_mode: Option<String>,
    single_task_only: bool,
    tracked_flow_entry: Option<String>,
    allow_freeform_chat: bool,
    confidence: String,
    matched_terms: Vec<String>,
    compiled_bundle: serde_json::Value,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionEvidence {
    surface: String,
    ok: bool,
    row_count: usize,
    output: String,
}

#[derive(Debug, serde::Serialize)]
struct RuntimeConsumptionOverview {
    surface: String,
    ok: bool,
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
    Ok(resolve_repo_root()?.join("vida.config.yaml"))
}

fn load_project_overlay_yaml() -> Result<serde_yaml::Value, String> {
    let path = config_file_path()?;
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn read_yaml_file(path: &Path) -> serde_yaml::Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return serde_yaml::Value::Null;
    };
    serde_yaml::from_str(&raw).unwrap_or(serde_yaml::Value::Null)
}

fn yaml_lookup<'a>(value: &'a serde_yaml::Value, path: &[&str]) -> Option<&'a serde_yaml::Value> {
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

fn yaml_bool(value: Option<&serde_yaml::Value>, default: bool) -> bool {
    value.and_then(|node| node.as_bool()).unwrap_or(default)
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

fn build_compiled_agent_extension_bundle(config: &serde_yaml::Value) -> serde_json::Value {
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
    let root = resolve_repo_root().unwrap_or_else(|_| PathBuf::from("."));
    let roles_registry = roles_path
        .as_deref()
        .map(|path| read_yaml_file(&root.join(path)))
        .unwrap_or(serde_yaml::Value::Null);
    let skills_registry = skills_path
        .as_deref()
        .map(|path| read_yaml_file(&root.join(path)))
        .unwrap_or(serde_yaml::Value::Null);
    let profiles_registry = profiles_path
        .as_deref()
        .map(|path| read_yaml_file(&root.join(path)))
        .unwrap_or(serde_yaml::Value::Null);
    let flows_registry = flows_path
        .as_deref()
        .map(|path| read_yaml_file(&root.join(path)))
        .unwrap_or(serde_yaml::Value::Null);

    serde_json::json!({
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
        "role_selection": serde_json::to_value(yaml_lookup(config, &["agent_extensions", "role_selection"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
    })
}

fn pack_keyword_terms(config: &serde_yaml::Value, keys: &[&str]) -> Vec<String> {
    let mut terms = Vec::new();
    for key in keys {
        for value in split_csv_like(
            &yaml_string(yaml_lookup(config, &["pack_router_keywords", key])).unwrap_or_default(),
        ) {
            if !terms.contains(&value) {
                terms.push(value);
            }
        }
    }
    terms
}

fn standard_mode_keywords(mode_id: &str, config: &serde_yaml::Value) -> Vec<String> {
    let mut keywords = match mode_id {
        "scope_discussion" => vec![
            "scope", "scoping", "requirement", "requirements", "acceptance", "constraint",
            "constraints", "clarify", "clarification", "discover", "discovery", "spec",
            "specification", "user story", "ac",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>(),
        "pbi_discussion" => vec![
            "pbi", "backlog", "priority", "prioritize", "prioritization", "task", "ticket",
            "delivery cut", "estimate", "estimation", "roadmap", "decompose",
            "decomposition", "work pool",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    let extra = match mode_id {
        "scope_discussion" => pack_keyword_terms(config, &["spec"]),
        "pbi_discussion" => pack_keyword_terms(config, &["pool", "pool_strong", "pool_dependency"]),
        _ => Vec::new(),
    };
    for keyword in extra {
        if !keywords.contains(&keyword) {
            keywords.push(keyword);
        }
    }
    keywords
}

fn contains_keywords(request: &str, keywords: &[String]) -> Vec<String> {
    keywords
        .iter()
        .filter(|keyword| request.contains(keyword.as_str()))
        .cloned()
        .collect()
}

fn build_runtime_lane_selection(request: &str) -> Result<RuntimeConsumptionLaneSelection, String> {
    let config = load_project_overlay_yaml()?;
    let bundle = build_compiled_agent_extension_bundle(&config);
    let selection_mode = yaml_string(yaml_lookup(
        &config,
        &["agent_extensions", "role_selection", "mode"],
    ))
    .unwrap_or_else(|| "fixed".to_string());
    let configured_fallback = yaml_string(yaml_lookup(
        &config,
        &["agent_extensions", "role_selection", "fallback_role"],
    ))
    .unwrap_or_else(|| "orchestrator".to_string());
    let fallback_role = if role_exists_in_lane_bundle(&bundle, &configured_fallback) {
        configured_fallback
    } else {
        "orchestrator".to_string()
    };
    let normalized_request = request.to_lowercase();
    let mut result = RuntimeConsumptionLaneSelection {
        ok: true,
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
        reason: String::new(),
    };

    if selection_mode != "auto" {
        result.reason = "fixed_mode".to_string();
        return Ok(result);
    }

    let Some(serde_yaml::Value::Mapping(conversation_modes)) = yaml_lookup(
        &config,
        &["agent_extensions", "role_selection", "conversation_modes"],
    ) else {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        return Ok(result);
    };
    if normalized_request.trim().is_empty() {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        return Ok(result);
    }

    let mut candidates = Vec::new();
    for (mode_key, mode_value) in conversation_modes {
        let Some(mode_id) = mode_key.as_str() else {
            continue;
        };
        let serde_yaml::Value::Mapping(_) = mode_value else {
            continue;
        };
        if !yaml_bool(yaml_lookup(mode_value, &["enabled"]), true) {
            continue;
        }
        let matched_terms =
            contains_keywords(&normalized_request, &standard_mode_keywords(mode_id, &config));
        candidates.push((
            mode_id.to_string(),
            yaml_string(yaml_lookup(mode_value, &["role"])).unwrap_or_else(|| fallback_role.clone()),
            yaml_bool(yaml_lookup(mode_value, &["single_task_only"]), false),
            yaml_string(yaml_lookup(mode_value, &["tracked_flow_entry"])),
            yaml_bool(yaml_lookup(mode_value, &["allow_freeform_chat"]), false),
            matched_terms,
        ));
    }

    if candidates.is_empty() {
        result.reason = "auto_no_enabled_modes".to_string();
        return Ok(result);
    }

    candidates.sort_by(|a, b| {
        b.5.len()
            .cmp(&a.5.len())
            .then_with(|| a.0.cmp(&b.0))
    });
    let selected = &candidates[0];
    if selected.5.is_empty() {
        result.reason = "auto_no_keyword_match".to_string();
        return Ok(result);
    }
    if !role_exists_in_lane_bundle(&bundle, &selected.1) {
        result.reason = "auto_selected_unknown_role".to_string();
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
    Ok(result)
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

async fn build_taskflow_consume_bundle_payload(
    store: &StateStore,
) -> Result<TaskflowConsumeBundlePayload, String> {
    let vida_root =
        resolve_repo_root().map_err(|error| format!("Failed to resolve project root: {error}"))?;
    let launcher_runtime_paths = doctor_launcher_summary_json()
        .map_err(|error| format!("Failed to resolve launcher/runtime paths: {error}"))?;
    let root_artifact_id = store
        .active_instruction_root()
        .await
        .map_err(|error| format!("Failed to read active instruction root: {error}"))?;
    let effective_instruction_bundle = store
        .inspect_effective_instruction_bundle(&root_artifact_id)
        .await
        .map_err(|error| format!("Failed to inspect effective instruction bundle: {error}"))?;
    let boot_compatibility = store
        .latest_boot_compatibility_summary()
        .await
        .map_err(|error| format!("Failed to read boot compatibility summary: {error}"))?
        .ok_or_else(|| {
            "Boot compatibility summary is missing from the authoritative state store.".to_string()
        })?;
    let migration_preflight = store
        .latest_migration_preflight_summary()
        .await
        .map_err(|error| format!("Failed to read migration preflight summary: {error}"))?
        .ok_or_else(|| {
            "Migration preflight summary is missing from the authoritative state store.".to_string()
        })?;
    let task_store = store
        .task_store_summary()
        .await
        .map_err(|error| format!("Failed to read task store summary: {error}"))?;
    let run_graph = store
        .run_graph_summary()
        .await
        .map_err(|error| format!("Failed to read run graph summary: {error}"))?;

    Ok(TaskflowConsumeBundlePayload {
        artifact_name: "taskflow_runtime_bundle".to_string(),
        artifact_type: "runtime_bundle".to_string(),
        generated_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        vida_root: vida_root.display().to_string(),
        config_path: vida_root.join("vida.config.yaml").display().to_string(),
        launcher_runtime_paths,
        effective_instruction_bundle: serde_json::json!({
            "root_artifact_id": effective_instruction_bundle.root_artifact_id,
            "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
            "source_version_tuple": effective_instruction_bundle.source_version_tuple,
            "receipt_id": effective_instruction_bundle.receipt_id,
            "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
        }),
        boot_compatibility: serde_json::json!({
            "classification": boot_compatibility.classification,
            "reasons": boot_compatibility.reasons,
            "next_step": boot_compatibility.next_step,
        }),
        migration_preflight: serde_json::json!({
            "compatibility_classification": migration_preflight.compatibility_classification,
            "migration_state": migration_preflight.migration_state,
            "blockers": migration_preflight.blockers,
            "source_version_tuple": migration_preflight.source_version_tuple,
            "next_step": migration_preflight.next_step,
        }),
        task_store: serde_json::json!({
            "total_count": task_store.total_count,
            "open_count": task_store.open_count,
            "in_progress_count": task_store.in_progress_count,
            "closed_count": task_store.closed_count,
            "epic_count": task_store.epic_count,
            "ready_count": task_store.ready_count,
        }),
        run_graph: serde_json::json!({
            "execution_plan_count": run_graph.execution_plan_count,
            "routed_run_count": run_graph.routed_run_count,
            "governance_count": run_graph.governance_count,
            "resumability_count": run_graph.resumability_count,
            "reconciliation_count": run_graph.reconciliation_count,
        }),
    })
}

fn taskflow_consume_bundle_check(
    payload: &TaskflowConsumeBundlePayload,
) -> TaskflowConsumeBundleCheck {
    let mut blockers = Vec::new();
    let root_artifact_id = payload
        .effective_instruction_bundle
        .get("root_artifact_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let artifact_count = payload
        .effective_instruction_bundle
        .get("artifact_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    let boot_classification = payload
        .boot_compatibility
        .get("classification")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let migration_state = payload
        .migration_preflight
        .get("migration_state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let next_step = payload
        .migration_preflight
        .get("next_step")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let bundle_order = payload
        .effective_instruction_bundle
        .get("mandatory_chain_order")
        .and_then(serde_json::Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);

    if root_artifact_id.is_empty() {
        blockers.push("missing_root_artifact_id".to_string());
    }
    if bundle_order == 0 {
        blockers.push("missing_mandatory_chain_order".to_string());
    }
    if artifact_count == 0 {
        blockers.push("missing_effective_bundle_artifacts".to_string());
    }
    if boot_classification != "compatible" {
        blockers.push("boot_incompatible".to_string());
    }
    if migration_state != "no_migration_required" || next_step != "normal_boot_allowed" {
        blockers.push("migration_not_ready".to_string());
    }

    TaskflowConsumeBundleCheck {
        ok: blockers.is_empty(),
        blockers,
        root_artifact_id,
        artifact_count,
        boot_classification,
        migration_state,
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
    RuntimeConsumptionOverview,
) {
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

    let check_rows = count_nonempty_lines(&check_output);
    let readiness_rows = count_nonempty_lines(&readiness_output);
    let proof_ok = proof_output.contains("✅ OK: proofcheck");
    let proof_blocking = !proof_ok;

    let check = RuntimeConsumptionEvidence {
        surface: "vida docflow check --profile active-canon".to_string(),
        ok: check_output.trim().is_empty(),
        row_count: check_rows,
        output: check_output,
    };
    let readiness = RuntimeConsumptionEvidence {
        surface: "vida docflow readiness-check --profile active-canon".to_string(),
        ok: readiness_output.trim().is_empty(),
        row_count: readiness_rows,
        output: readiness_output,
    };
    let proof = RuntimeConsumptionEvidence {
        surface: "vida docflow proofcheck --profile active-canon".to_string(),
        ok: proof_ok,
        row_count: count_nonempty_lines(&proof_output),
        output: proof_output,
    };
    let overview = RuntimeConsumptionOverview {
        surface: "vida taskflow direct runtime-consumption overview".to_string(),
        ok: check.ok && readiness.ok && proof.ok,
        check_rows,
        readiness_rows,
        proof_blocking,
    };

    (check, readiness, proof, overview)
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
        let entry = entry.map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
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
                            payload.effective_instruction_bundle["root_artifact_id"]
                                .as_str()
                                .unwrap_or("unknown"),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "bundle order",
                            &payload.effective_instruction_bundle["mandatory_chain_order"]
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
                        print_surface_header(RenderMode::Plain, "vida taskflow consume bundle check");
                        print_surface_line(RenderMode::Plain, "ok", if check.ok { "true" } else { "false" });
                        print_surface_line(RenderMode::Plain, "root artifact", &check.root_artifact_id);
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
                        let (check, readiness, proof, overview) = build_docflow_runtime_evidence();
                        let payload = TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: time::OffsetDateTime::now_utc()
                                .format(&Rfc3339)
                                .expect("rfc3339 timestamp should render"),
                            closure_authority: "taskflow".to_string(),
                            role_selection: match build_runtime_lane_selection(&request_text) {
                                Ok(selection) => selection,
                                Err(error) => {
                                    eprintln!("{error}");
                                    return ExitCode::from(1);
                                }
                            },
                            request_text,
                            direct_consumption_ready: bundle_check.ok
                                && check.ok
                                && readiness.ok
                                && proof.ok,
                            runtime_bundle,
                            bundle_check,
                            docflow_activation: RuntimeConsumptionDocflowActivation {
                                activated: true,
                                runtime_family: "docflow".to_string(),
                                owner_runtime: "taskflow".to_string(),
                                evidence: serde_json::json!({
                                    "overview": overview,
                                    "check": check,
                                    "readiness": readiness,
                                    "proof": proof,
                                }),
                            },
                        };
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

                        if as_json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "surface": "vida taskflow consume final",
                                    "payload": payload,
                                    "snapshot_path": snapshot_path,
                                }))
                                .expect("consume final should render as json")
                            );
                        } else {
                            print_surface_header(RenderMode::Plain, "vida taskflow consume final");
                            print_surface_line(
                                RenderMode::Plain,
                                "request",
                                &payload.request_text,
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "bundle ready",
                                if payload.bundle_check.ok { "true" } else { "false" },
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "docflow ready",
                                if payload.direct_consumption_ready {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "snapshot path",
                                &snapshot_path,
                            );
                        }

                        if payload.direct_consumption_ready {
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
        let issue_type = dependency.dependency_issue_type.as_deref().unwrap_or("unknown");
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
        &format!("{}\t{}\t{}", tree.task.id, tree.task.status, tree.task.title),
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

fn normalize_root_arg(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
enum RenderMode {
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
    Boot(BootArgs),
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
    println!(
        "  boot      initialize authoritative state and instruction/framework-memory surfaces"
    );
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
}
