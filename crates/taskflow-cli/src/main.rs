use clap::Parser;
use std::process::ExitCode;
use taskflow_cli::{Cli, run};

fn main() -> ExitCode {
    run(Cli::parse())
}
