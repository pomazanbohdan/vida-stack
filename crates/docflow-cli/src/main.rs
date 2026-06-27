use clap::Parser;
use docflow_cli::{Cli, run_with_exit};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let result = run_with_exit(cli);
    println!("{}", result.output);
    result.exit_code
}
