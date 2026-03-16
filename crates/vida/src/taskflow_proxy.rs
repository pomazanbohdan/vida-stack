use std::process::ExitCode;

use crate::taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use crate::taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use crate::taskflow_spec_bootstrap::run_taskflow_bootstrap_spec;
use crate::taskflow_task_bridge::run_taskflow_task_bridge;
use crate::{
    resolve_repo_root, taskflow_consume, taskflow_protocol_binding, Command, ProxyArgs,
};
use clap::Parser;

async fn route_taskflow_doctor(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Doctor(doctor_args)) => crate::doctor_surface::run_doctor(doctor_args).await,
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
        return taskflow_protocol_binding::run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        if matches!(
            args.args.get(1).map(String::as_str),
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
