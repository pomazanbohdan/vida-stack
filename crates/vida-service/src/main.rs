use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vida_service::{ipc_conformance_matrix, lifecycle_plan, run_foreground_until_shutdown};

#[derive(Debug, Parser)]
#[command(name = "vida-service")]
#[command(about = "Headless VIDA local service daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Foreground,
    LifecyclePlan {
        #[arg(long, default_value = "dry_run")]
        mode: String,
        #[arg(long)]
        json: bool,
    },
    IpcMatrix {
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = vida_service::ServiceDaemonConfig::local_default();
    match cli.command.unwrap_or(Command::Foreground) {
        Command::Foreground => match run_foreground_until_shutdown(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("vida-service foreground failed: {error}");
                ExitCode::from(1)
            }
        },
        Command::LifecyclePlan { mode, json } => {
            let plan = lifecycle_plan(&mode, &config);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&plan).expect("serialize lifecycle plan")
                );
            } else {
                println!("vida-service lifecycle-plan");
                println!("  mode: {}", plan.mode);
                println!("  service_name: {}", plan.service_name);
                println!("  apply_requires_token: {}", plan.apply_requires_token);
                println!("  restart_policy: {}", plan.restart_policy);
            }
            ExitCode::SUCCESS
        }
        Command::IpcMatrix { json } => {
            let matrix = ipc_conformance_matrix();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&matrix).expect("serialize IPC matrix")
                );
            } else {
                println!("vida-service ipc-matrix");
                println!("  rows: {}", matrix.len());
                for row in matrix {
                    println!("  {},{},{}", row.platform, row.transport, row.framing);
                }
            }
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn service_cli_defaults_to_foreground_command() {
        let cli = Cli::try_parse_from(["vida-service"]).expect("default CLI parses");
        assert!(cli.command.is_none());
    }

    #[test]
    fn service_cli_parses_json_subcommands_and_lifecycle_mode() {
        let lifecycle = Cli::try_parse_from([
            "vida-service",
            "lifecycle-plan",
            "--mode",
            "apply",
            "--json",
        ])
        .expect("lifecycle plan parses");
        assert!(matches!(
            lifecycle.command,
            Some(Command::LifecyclePlan { mode, json }) if mode == "apply" && json
        ));

        let matrix = Cli::try_parse_from(["vida-service", "ipc-matrix", "--json"])
            .expect("IPC matrix parses");
        assert!(matches!(matrix.command, Some(Command::IpcMatrix { json }) if json));
    }
}
