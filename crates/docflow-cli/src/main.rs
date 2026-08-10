use clap::Parser;
use docflow_cli::{Cli, run_with_exit};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let result = run_with_exit(cli);
    println!("{}", result.output);
    result.exit_code
}

#[cfg(test)]
mod tests {
    #[test]
    fn docflow_entrypoint_signature_contract() {
        let _: fn() -> std::process::ExitCode = super::main;
    }
}
