use clap::Parser;
use std::process::ExitCode;
use vida_pi_agent::{Cli, run_cli_with_stdin};

fn main() -> ExitCode {
    let output = run_cli_with_stdin(Cli::parse(), std::io::stdin());
    println!(
        "{}",
        serde_json::to_string(&output.payload).expect("VIDA Pi adapter result JSON should render")
    );
    if output.exit_code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(output.exit_code as u8)
    }
}
