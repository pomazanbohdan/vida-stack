use clap::Parser;
use docflow_cli::{Cli, run};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    println!("{}", run(cli));
    ExitCode::SUCCESS
}
