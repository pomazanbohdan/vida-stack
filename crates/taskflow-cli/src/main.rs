use clap::Parser;
use std::process::ExitCode;
use taskflow_cli::{Cli, run};

fn main() -> ExitCode {
    run(Cli::parse())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskflow_entrypoint_signature_contract() {
        let _: fn() -> ExitCode = main;
    }
}
