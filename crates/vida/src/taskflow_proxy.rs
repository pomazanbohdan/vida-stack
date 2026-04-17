use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use crate::taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use crate::taskflow_spec_bootstrap::run_taskflow_bootstrap_spec;
use crate::taskflow_task_bridge::{enforce_execution_preparation_contract_gate, proxy_state_dir};
use crate::{
    print_surface_header, print_surface_line, surface_render, taskflow_consume,
    taskflow_protocol_binding, Command, ProxyArgs, RenderMode, TaskCommand, TaskReadyArgs,
};
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GraphSummaryTaskRef {
    id: String,
    display_id: Option<String>,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GraphSummaryWaveBucket {
    wave_id: String,
    task_count: usize,
    ready_count: usize,
    blocked_count: usize,
    primary_ready_task: Option<GraphSummaryTaskRef>,
    primary_blocked_task: Option<GraphSummaryTaskRef>,
}

fn graph_summary_task_ref(task: &crate::state_store::TaskRecord) -> GraphSummaryTaskRef {
    GraphSummaryTaskRef {
        id: task.id.clone(),
        display_id: task.display_id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
    }
}

fn recovery_holds_active_bound_run(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|summary| summary.delegation_gate.delegated_cycle_open)
}

fn authoritative_dispatch_blocker_codes(
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let Some(dispatch) = dispatch else {
        return Vec::new();
    };
    let mut blocker_codes = Vec::new();
    if let Some(blocker_code) = dispatch
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if matches!(dispatch.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(
            dispatch.lane_status.as_str(),
            "lane_blocked" | "lane_failed"
        )
    {
        blocker_codes.extend(
            dispatch
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes)
}

fn task_wave_label(
    task_id: &str,
    by_id: &BTreeMap<String, crate::state_store::TaskRecord>,
    memo: &mut BTreeMap<String, Option<String>>,
    active: &mut BTreeSet<String>,
) -> Option<String> {
    if let Some(cached) = memo.get(task_id) {
        return cached.clone();
    }
    if !active.insert(task_id.to_string()) {
        return None;
    }

    let resolved = by_id.get(task_id).and_then(|task| {
        task.execution_semantics
            .order_bucket
            .clone()
            .or_else(|| {
                task.labels
                    .iter()
                    .find(|label| label.as_str() == "wave" || label.starts_with("wave-"))
                    .cloned()
            })
            .or_else(|| {
                task.dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type == "parent-child")
                    .find_map(|dependency| {
                        task_wave_label(&dependency.depends_on_id, by_id, memo, active)
                    })
            })
    });

    active.remove(task_id);
    memo.insert(task_id.to_string(), resolved.clone());
    resolved
}

fn wave_sort_key(wave_id: &str) -> (usize, usize, &str) {
    if wave_id == "unassigned" {
        return (2, usize::MAX, wave_id);
    }
    if wave_id == "wave" {
        return (0, 0, wave_id);
    }
    if let Some(index) = wave_id.strip_prefix("wave-") {
        if let Ok(parsed) = index.parse::<usize>() {
            return (0, parsed, wave_id);
        }
    }
    (1, usize::MAX, wave_id)
}

fn build_graph_summary_waves(
    all_tasks: &[crate::state_store::TaskRecord],
    ready_tasks: &[crate::state_store::TaskRecord],
    blocked_tasks: &[crate::state_store::BlockedTaskRecord],
) -> Vec<GraphSummaryWaveBucket> {
    #[derive(Default)]
    struct WaveAccumulator {
        task_ids: BTreeSet<String>,
        ready_count: usize,
        blocked_count: usize,
        primary_ready_task: Option<GraphSummaryTaskRef>,
        primary_blocked_task: Option<GraphSummaryTaskRef>,
    }

    let by_id = all_tasks
        .iter()
        .cloned()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    let mut memo = BTreeMap::<String, Option<String>>::new();
    let mut active = BTreeSet::<String>::new();
    let mut buckets = BTreeMap::<String, WaveAccumulator>::new();

    for task in all_tasks {
        if task.issue_type == "epic" || task.status == "closed" {
            continue;
        }
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        buckets
            .entry(wave_id)
            .or_default()
            .task_ids
            .insert(task.id.clone());
    }

    for task in ready_tasks {
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.ready_count += 1;
        if bucket.primary_ready_task.is_none() {
            bucket.primary_ready_task = Some(graph_summary_task_ref(task));
        }
    }

    for record in blocked_tasks {
        let wave_id = task_wave_label(&record.task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.blocked_count += 1;
        if bucket.primary_blocked_task.is_none() {
            bucket.primary_blocked_task = Some(graph_summary_task_ref(&record.task));
        }
    }

    let mut waves = buckets
        .into_iter()
        .map(|(wave_id, bucket)| GraphSummaryWaveBucket {
            wave_id,
            task_count: bucket.task_ids.len(),
            ready_count: bucket.ready_count,
            blocked_count: bucket.blocked_count,
            primary_ready_task: bucket.primary_ready_task,
            primary_blocked_task: bucket.primary_blocked_task,
        })
        .collect::<Vec<_>>();
    waves.sort_by(|left, right| {
        wave_sort_key(&left.wave_id)
            .cmp(&wave_sort_key(&right.wave_id))
            .then_with(|| right.ready_count.cmp(&left.ready_count))
            .then_with(|| right.blocked_count.cmp(&left.blocked_count))
            .then_with(|| left.wave_id.cmp(&right.wave_id))
    });
    waves
}

fn parse_taskflow_next_args(
    args: &[String],
) -> Result<(bool, Option<&str>, Option<PathBuf>), &'static str> {
    if !matches!(args.first().map(String::as_str), Some("next")) {
        return Err("Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]");
    }

    let mut as_json = false;
    let mut scope_task_id = None;
    let mut state_dir = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--scope" => {
                let Some(task_id) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                scope_task_id = Some(task_id.as_str());
                index += 2;
            }
            "--state-dir" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                state_dir = Some(PathBuf::from(path));
                index += 2;
            }
            "--help" | "-h" if index == 1 && args.len() == 2 => {
                return Ok((false, Some("__help__"), None));
            }
            _ => {
                return Err(
                    "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                );
            }
        }
    }

    Ok((as_json, scope_task_id, state_dir))
}

async fn route_taskflow_doctor(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Doctor(doctor_args)) => {
                crate::doctor_surface::run_doctor(doctor_args).await
            }
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

async fn route_taskflow_status(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Status(mut status_args)) => {
                if status_args.state_dir.is_none() {
                    let state_dir = match resolve_taskflow_proxy_state_dir(None) {
                        Ok(state_dir) => state_dir,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    status_args.state_dir = Some(state_dir);
                }
                crate::status_surface::run_status(status_args).await
            }
            _ => {
                eprintln!("Unsupported `vida taskflow status` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_ready(command: TaskReadyArgs) -> ExitCode {
    let state_dir = match resolve_taskflow_proxy_state_dir(command.state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let store = if state_dir.exists() {
        crate::state_store::StateStore::open_existing(state_dir).await
    } else {
        crate::state_store::StateStore::open(state_dir).await
    };
    match store {
        Ok(store) => match store.ready_tasks_scoped(command.scope.as_deref()).await {
            Ok(tasks) => {
                let payload =
                    serde_json::to_value(&tasks).expect("taskflow ready payload should serialize");
                if surface_render::print_surface_json(
                    &payload,
                    command.json,
                    "taskflow ready payload should render as json",
                ) {
                    return ExitCode::SUCCESS;
                }

                print_surface_header(command.render, "vida taskflow task ready");
                if tasks.is_empty() {
                    print_surface_line(command.render, "ready tasks", "none");
                    return ExitCode::SUCCESS;
                }

                for task in tasks {
                    println!("{}\t{}\t{}", task.id, task.status, task.title);
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Failed to compute taskflow ready tasks: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn taskflow_task_subcommand_supported(subcommand: &str) -> bool {
    matches!(
        subcommand,
        "help"
            | "import-jsonl"
            | "replace-jsonl"
            | "export-jsonl"
            | "list"
            | "show"
            | "ready"
            | "next"
            | "next-display-id"
            | "create"
            | "update"
            | "close"
            | "deps"
            | "reverse-deps"
            | "blocked"
            | "tree"
            | "validate-graph"
            | "dep"
            | "critical-path"
    )
}

async fn route_taskflow_task(args: &[String]) -> ExitCode {
    if let Some(subcommand) = args.get(1).map(String::as_str) {
        if !subcommand.starts_with('-') && !taskflow_task_subcommand_supported(subcommand) {
            eprintln!(
                "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to the external TaskFlow runtime."
            );
            return ExitCode::from(2);
        }
    }

    let mut argv = vec!["vida".to_string(), "task".to_string()];
    if matches!(args.get(1).map(String::as_str), Some("close")) {
        argv.push("close".to_string());
        argv.push("--source".to_string());
        argv.push("vida taskflow task close".to_string());
        argv.extend(args.iter().skip(2).cloned());
    } else {
        argv.extend(args.iter().skip(1).cloned());
    }
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => match task_args.command {
                TaskCommand::Ready(command) => route_taskflow_ready(command).await,
                _ => crate::task_surface::run_task(task_args).await,
            },
            _ => {
                eprintln!(
                    "Unsupported `vida taskflow task` subcommand. This compatibility alias must match the canonical `vida task` contract."
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn resolve_taskflow_proxy_state_dir(state_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    match state_dir {
        Some(state_dir) => Ok(state_dir),
        None => crate::resolve_runtime_project_root()
            .map(|project_root| project_root.join(crate::state_store::default_state_dir())),
    }
}

pub(crate) async fn run_taskflow_next_surface(args: &[String]) -> ExitCode {
    let (as_json, scope_task_id, state_dir) = match parse_taskflow_next_args(args) {
        Ok((_, Some("__help__"), _)) => {
            print_taskflow_proxy_help(Some("next"));
            return ExitCode::SUCCESS;
        }
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };

    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let runtime_consumption = match crate::runtime_consumption_summary(&state_dir) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to summarize runtime-consumption state: {error}");
            return ExitCode::from(1);
        }
    };

    let store = match crate::state_store::StateStore::open_existing(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks = match store.ready_tasks_scoped(scope_task_id).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute ready tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let latest_run_graph = match store.latest_run_graph_status().await {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to read latest run-graph status: {error}");
            return ExitCode::from(1);
        }
    };
    let recovery = match store.latest_run_graph_recovery_summary().await {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to read latest recovery summary: {error}");
            return ExitCode::from(1);
        }
    };
    let gate = match store.latest_run_graph_gate_summary().await {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to read latest gate summary: {error}");
            return ExitCode::from(1);
        }
    };
    let dispatch = match store.latest_run_graph_dispatch_receipt_summary().await {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to read latest dispatch receipt summary: {error}");
            return ExitCode::from(1);
        }
    };

    let recovery_holds_active_bound_run = recovery_holds_active_bound_run(recovery.as_ref());
    let primary_ready_task = (!recovery_holds_active_bound_run)
        .then(|| ready_tasks.first())
        .flatten()
        .map(|task| {
            serde_json::json!({
                "id": task.id,
                "display_id": task.display_id,
                "title": task.title,
                "status": task.status,
                "priority": task.priority,
                "issue_type": task.issue_type,
            })
        });

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();
    let recommended_command = if recovery_holds_active_bound_run {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::OpenDelegatedCycle,
        ) {
            blocker_codes.push(code);
        }
        blocker_codes.extend(authoritative_dispatch_blocker_codes(dispatch.as_ref()));
        if runtime_consumption.latest_kind.as_deref() == Some("final") {
            next_actions.push(
                "Continue the active bound run with `vida taskflow consume continue --json` before considering backlog ready-head work."
                    .to_string(),
            );
            Some("vida taskflow consume continue --json".to_string())
        } else {
            next_actions.push(
                "Inspect the active bound recovery state with `vida taskflow recovery latest --json` before considering backlog ready-head work."
                    .to_string(),
            );
            Some("vida taskflow recovery latest --json".to_string())
        }
    } else if let Some(task) = ready_tasks.first() {
        next_actions.push(format!(
            "Inspect the primary ready task with `vida task show {} --json` before dispatch.",
            task.id
        ));
        Some(format!("vida task show {} --json", task.id))
    } else if recovery
        .as_ref()
        .is_some_and(|summary| summary.recovery_ready)
        && runtime_consumption.latest_kind.as_deref() == Some("final")
    {
        next_actions.push(
            "Continue the latest lawful delegated chain with `vida taskflow consume continue --json`."
                .to_string(),
        );
        Some("vida taskflow consume continue --json".to_string())
    } else {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
        let ready_command = if let Some(task_id) = scope_task_id {
            format!("vida task ready --scope {task_id} --json")
        } else {
            "vida task ready --json".to_string()
        };
        next_actions.push(format!(
            "No ready backlog slice is available right now; inspect `{ready_command}` and `vida taskflow recovery latest --json`."
        ));
        Some(ready_command)
    };

    if recovery.is_some() {
        next_actions.push(
            "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
                .to_string(),
        );
    }

    if recovery.is_some() && runtime_consumption.latest_kind.as_deref() != Some("final") {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::ExecutionPreparationGateBlocked,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "Materialize final execution-preparation evidence with `vida taskflow consume final \"<request>\" --json` before attempting continuation."
                .to_string(),
        );
    }

    blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let payload = serde_json::json!({
        "surface": "vida taskflow next",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "recommended_command": recommended_command,
        "scope_task_id": scope_task_id,
        "ready_count": ready_tasks.len(),
        "primary_ready_task": primary_ready_task,
        "latest_run_graph": latest_run_graph,
        "recovery": recovery,
        "gate": gate,
        "dispatch": dispatch,
        "runtime_consumption": runtime_consumption,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow next");
        crate::print_surface_line(RenderMode::Plain, "status", status);
        if !blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        if let Some(task_id) = scope_task_id {
            crate::print_surface_line(RenderMode::Plain, "scope_task_id", task_id);
        }
        if let Some(task) = primary_ready_task
            .as_ref()
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", task);
            if let Some(title) = primary_ready_task
                .as_ref()
                .and_then(|value| value.get("title"))
                .and_then(serde_json::Value::as_str)
            {
                crate::print_surface_line(RenderMode::Plain, "title", title);
            }
        } else {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", "none");
        }
        if let Some(command) = payload["recommended_command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_command", command);
        }
        if let Some(next_action) = next_actions.first() {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_graph_summary(args: &[String]) -> ExitCode {
    let as_json = match args {
        [head] if head == "graph-summary" => false,
        [head, flag] if head == "graph-summary" && flag == "--json" => true,
        [head, flag] if head == "graph-summary" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("graph-summary"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: vida taskflow graph-summary [--json]");
            return ExitCode::from(2);
        }
    };

    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks = match store.ready_tasks_scoped(None).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute ready tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let blocked_tasks = match store.blocked_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute blocked tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let all_tasks = match store.list_tasks(None, true).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to list tasks for wave summary: {error}");
            return ExitCode::from(1);
        }
    };
    let critical_path = match store.critical_path().await {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to compute critical path: {error}");
            return ExitCode::from(1);
        }
    };
    let scheduling = match store.scheduling_projection_scoped(None, None).await {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute scheduling projection: {error}");
            return ExitCode::from(1);
        }
    };
    let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);

    let primary_ready_task = scheduling.ready.first().map(|candidate| {
        serde_json::json!({
            "task": graph_summary_task_ref(&candidate.task),
            "ready_now": candidate.ready_now,
            "ready_parallel_safe": candidate.ready_parallel_safe,
            "active_critical_path": candidate.active_critical_path,
            "parallel_blockers": candidate.parallel_blockers,
            "execution_semantics": candidate.task.execution_semantics,
        })
    });
    let primary_blocked_task = blocked_tasks.first().map(|record| {
        serde_json::json!({
            "id": record.task.id,
            "display_id": record.task.display_id,
            "title": record.task.title,
            "status": record.task.status,
            "priority": record.task.priority,
            "issue_type": record.task.issue_type,
            "blocker_count": record.blockers.len(),
            "blockers": record.blockers,
        })
    });

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();

    if let Some(task) = ready_tasks.first() {
        next_actions.push(format!(
            "Inspect the primary ready task with `vida task show {} --json` before dispatch.",
            task.id
        ));
    }
    if let Some(record) = blocked_tasks.first() {
        next_actions.push(format!(
            "Inspect the highest-priority blocked task with `vida task deps {} --json` before resequencing.",
            record.task.id
        ));
    }
    if critical_path.length > 0 {
        next_actions.push(
            "Inspect the current graph bottleneck with `vida task critical-path --json` before parallelizing additional work."
                .to_string(),
        );
    }
    if ready_tasks.is_empty() {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
    }
    if blocked_tasks.is_empty() && critical_path.length == 0 {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::TaskGraphEmpty,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "No active execution graph is present; inspect `vida task list --all --json` before sequencing new work."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let payload = serde_json::json!({
        "surface": "vida taskflow graph-summary",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "ready_count": ready_tasks.len(),
        "blocked_count": blocked_tasks.len(),
        "critical_path_length": critical_path.length,
        "current_task_id": scheduling.current_task_id,
        "primary_ready_task": primary_ready_task,
        "primary_blocked_task": primary_blocked_task,
        "scheduling": scheduling,
        "waves": waves,
        "critical_path": critical_path,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph-summary");
        crate::print_surface_line(RenderMode::Plain, "status", status);
        if !blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "blocked_count",
            &blocked_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "critical_path_length",
            &critical_path.length.to_string(),
        );
        crate::print_surface_line(RenderMode::Plain, "wave_count", &waves.len().to_string());
        if let Some(task) = ready_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", &task.id);
        }
        if let Some(task_id) = payload["current_task_id"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "current_task_id", task_id);
        }
        if let Some(record) = blocked_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_blocked_task", &record.task.id);
        }
        if let Some(wave) = waves.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_wave", &wave.wave_id);
        }
        if let Some(next_action) = next_actions.first() {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_graph_summary_waves, taskflow_task_subcommand_supported, GraphSummaryWaveBucket,
    };
    use crate::state_store::{BlockedTaskRecord, TaskDependencyRecord, TaskRecord};

    fn task(
        id: &str,
        issue_type: &str,
        status: &str,
        priority: u32,
        labels: &[&str],
        dependencies: Vec<TaskDependencyRecord>,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: format!("task {id}"),
            description: String::new(),
            issue_type: issue_type.to_string(),
            status: status.to_string(),
            priority,
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            updated_at: "0".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            dependencies,
        }
    }

    fn parent_dependency(issue_id: &str, depends_on_id: &str) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    #[test]
    fn graph_summary_waves_follow_parent_chain_wave_labels() {
        let wave_epic = task("wave-0-epic", "epic", "open", 1, &["wave-0"], Vec::new());
        let child_ready = task(
            "r1-ready",
            "task",
            "open",
            1,
            &[],
            vec![parent_dependency("r1-ready", "wave-0-epic")],
        );
        let child_blocked = task(
            "r1-blocked",
            "task",
            "in_progress",
            2,
            &[],
            vec![parent_dependency("r1-blocked", "wave-0-epic")],
        );
        let unassigned = task("r1-unassigned", "task", "open", 3, &[], Vec::new());

        let all_tasks = vec![
            wave_epic.clone(),
            child_ready.clone(),
            child_blocked.clone(),
            unassigned.clone(),
        ];
        let ready_tasks = vec![child_ready.clone(), unassigned.clone()];
        let blocked_tasks = vec![BlockedTaskRecord {
            task: child_blocked.clone(),
            blockers: Vec::new(),
        }];

        let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);
        assert_eq!(
            waves,
            vec![
                GraphSummaryWaveBucket {
                    wave_id: "wave-0".to_string(),
                    task_count: 2,
                    ready_count: 1,
                    blocked_count: 1,
                    primary_ready_task: Some(super::graph_summary_task_ref(&child_ready)),
                    primary_blocked_task: Some(super::graph_summary_task_ref(&child_blocked)),
                },
                GraphSummaryWaveBucket {
                    wave_id: "unassigned".to_string(),
                    task_count: 1,
                    ready_count: 1,
                    blocked_count: 0,
                    primary_ready_task: Some(super::graph_summary_task_ref(&unassigned)),
                    primary_blocked_task: None,
                },
            ]
        );
    }

    #[test]
    fn graph_summary_waves_prefer_order_bucket_over_labels() {
        let mut explicit_bucket = task("bucket-task", "task", "open", 1, &["wave-99"], Vec::new());
        explicit_bucket.execution_semantics.order_bucket = Some("wave-2".to_string());

        let waves = build_graph_summary_waves(
            std::slice::from_ref(&explicit_bucket),
            std::slice::from_ref(&explicit_bucket),
            &[],
        );
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].wave_id, "wave-2");
        assert_eq!(waves[0].ready_count, 1);
    }

    #[test]
    fn taskflow_task_subcommand_supports_replace_jsonl() {
        assert!(taskflow_task_subcommand_supported("replace-jsonl"));
    }

    #[test]
    fn recovery_holds_active_bound_run_when_delegated_cycle_is_open() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_active".to_string(),
            },
        };

        assert!(super::recovery_holds_active_bound_run(Some(&recovery)));
    }

    #[test]
    fn recovery_does_not_hold_ready_head_when_delegated_cycle_is_clear() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            resume_node: None,
            resume_status: "completed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "none".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                reporting_pause_gate: "clear".to_string(),
                continuation_signal: "none".to_string(),
                blocker_code: None,
                lifecycle_stage: "closure_complete".to_string(),
            },
        };

        assert!(!super::recovery_holds_active_bound_run(Some(&recovery)));
        assert!(!super::recovery_holds_active_bound_run(None));
    }

    #[test]
    fn authoritative_dispatch_blocker_codes_include_primary_and_downstream_blockers() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
            activation_runtime_role: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            downstream_dispatch_trace_path: Some("/tmp/downstream-trace.json".to_string()),
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: Some("coach".to_string()),
            downstream_dispatch_note: Some("after coach, verify".to_string()),
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
        };

        let blocker_codes = super::authoritative_dispatch_blocker_codes(Some(&dispatch));
        assert_eq!(blocker_codes.len(), 2);
        assert!(blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence"));
    }
}

pub(crate) async fn run_taskflow_proxy(args: ProxyArgs) -> ExitCode {
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
        return route_taskflow_task(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("next")) {
        return run_taskflow_next_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph-summary")) {
        return run_taskflow_graph_summary(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("doctor")) {
        return route_taskflow_doctor(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("status")) {
        return route_taskflow_status(&args.args).await;
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
        return taskflow_protocol_binding::run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("continuation")) {
        return crate::taskflow_continuation::run_taskflow_continuation(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("packet")) {
        return crate::taskflow_packet::run_taskflow_packet(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        let consume_subcommand = args.args.get(1).map(String::as_str);
        if matches!(consume_subcommand, Some("continue" | "advance")) {
            let state_root = proxy_state_dir();
            if let Err(error) = enforce_execution_preparation_contract_gate(&state_root) {
                eprintln!(
                    "{error}\nFail-closed: `vida taskflow consume {}` requires release-1 execution-preparation evidence/contract.",
                    consume_subcommand.unwrap_or("unknown")
                );
                return ExitCode::from(1);
            }
        }
        if matches!(
            consume_subcommand,
            None | Some(
                "bundle" | "agent-system" | "final" | "continue" | "advance" | "--help" | "-h"
            )
        ) {
            return taskflow_consume::run_taskflow_consume(&args.args).await;
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
            Some("seed" | "advance" | "dispatch-init" | "init" | "update")
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
