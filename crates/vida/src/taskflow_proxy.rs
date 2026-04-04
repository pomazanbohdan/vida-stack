use std::process::ExitCode;

use crate::taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use crate::taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use crate::taskflow_spec_bootstrap::run_taskflow_bootstrap_spec;
use crate::taskflow_task_bridge::{enforce_execution_preparation_contract_gate, proxy_state_dir};
use crate::{taskflow_consume, taskflow_protocol_binding, Command, ProxyArgs, RenderMode};
use clap::Parser;

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

async fn route_taskflow_task(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(std::iter::once("task".to_string()))
        .chain(args.iter().skip(1).cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => crate::task_surface::run_task(task_args).await,
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

async fn run_taskflow_next(args: &[String]) -> ExitCode {
    let as_json = match args {
        [head] if head == "next" => false,
        [head, flag] if head == "next" && flag == "--json" => true,
        [head, flag] if head == "next" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("next"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: vida taskflow next [--json]");
            return ExitCode::from(2);
        }
    };

    let state_dir = proxy_state_dir();
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

    let ready_tasks = match store.ready_tasks_scoped(None).await {
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

    let primary_ready_task = ready_tasks.first().map(|task| {
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
    let recommended_command = if let Some(task) = ready_tasks.first() {
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
        blocker_codes.push("no_ready_tasks".to_string());
        next_actions.push(
            "No ready backlog slice is available right now; inspect `vida task ready --json` and `vida taskflow recovery latest --json`."
                .to_string(),
        );
        Some("vida task ready --json".to_string())
    };

    if recovery.is_some() {
        next_actions.push(
            "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
                .to_string(),
        );
    }

    if recovery.is_some() && runtime_consumption.latest_kind.as_deref() != Some("final") {
        blocker_codes.push("execution_preparation_gate_blocked".to_string());
        next_actions.push(
            "Materialize final execution-preparation evidence with `vida taskflow consume final \"<request>\" --json` before attempting continuation."
                .to_string(),
        );
    }

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
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        if let Some(task) = ready_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", &task.id);
            crate::print_surface_line(RenderMode::Plain, "title", &task.title);
        } else {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", "none");
        }
        if let Some(command) = payload["recommended_command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_command", command);
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
    let critical_path = match store.critical_path().await {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to compute critical path: {error}");
            return ExitCode::from(1);
        }
    };

    let primary_ready_task = ready_tasks.first().map(|task| {
        serde_json::json!({
            "id": task.id,
            "display_id": task.display_id,
            "title": task.title,
            "status": task.status,
            "priority": task.priority,
            "issue_type": task.issue_type,
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
        blocker_codes.push("no_ready_tasks".to_string());
    }
    if blocked_tasks.is_empty() && critical_path.length == 0 {
        blocker_codes.push("task_graph_empty".to_string());
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
        "primary_ready_task": primary_ready_task,
        "primary_blocked_task": primary_blocked_task,
        "critical_path": critical_path,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph-summary");
        crate::print_surface_line(RenderMode::Plain, "status", status);
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
        if let Some(task) = ready_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", &task.id);
        }
        if let Some(record) = blocked_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_blocked_task", &record.task.id);
        }
    }

    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
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
        return run_taskflow_next(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph-summary")) {
        return run_taskflow_graph_summary(&args.args).await;
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
        return taskflow_protocol_binding::run_taskflow_protocol_binding(&args.args).await;
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
