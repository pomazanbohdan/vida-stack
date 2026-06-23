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
