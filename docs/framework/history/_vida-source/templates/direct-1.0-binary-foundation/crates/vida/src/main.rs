mod temp_state;

use std::env;
use std::process::ExitCode;

const ROOT_COMMANDS: &[&str] = &["boot", "task", "memory", "status", "doctor"];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    run(args)
}

fn run(args: Vec<String>) -> ExitCode {
    match args.as_slice() {
        [] => {
            print_root_help();
            ExitCode::SUCCESS
        }
        [flag] if is_help_flag(flag) => {
            print_root_help();
            ExitCode::SUCCESS
        }
        [command] if command == "boot" => run_boot(&[]),
        [command, rest @ ..] if command == "boot" => run_boot(rest),
        [command] if is_stub_command(command) => run_stub(command, &[]),
        [command, rest @ ..] if is_stub_command(command) => run_stub(command, rest),
        [command, ..] => {
            eprintln!(
                "Unknown command family `{command}`. Use `vida --help` to inspect the frozen root surface."
            );
            ExitCode::from(2)
        }
    }
}

fn run_boot(args: &[String]) -> ExitCode {
    if is_help_request(args) {
        print_boot_help();
        return ExitCode::SUCCESS;
    }

    if let Some(arg) = args.first() {
        eprintln!("Unsupported `vida boot` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    println!("vida boot scaffold ready");
    ExitCode::SUCCESS
}

fn run_stub(command: &str, args: &[String]) -> ExitCode {
    if is_help_request(args) {
        print_stub_help(command);
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "`vida {command}` is not implemented in Binary Foundation. This root command family is reserved and fail-closed until later waves."
    );
    ExitCode::from(2)
}

fn is_help_request(args: &[String]) -> bool {
    matches!(args, [flag] if is_help_flag(flag))
}

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help" | "help")
}

fn is_stub_command(command: &str) -> bool {
    matches!(command, "task" | "memory" | "status" | "doctor")
}

fn print_root_help() {
    println!("VIDA Binary Foundation");
    println!();
    println!("Usage:");
    println!("  vida <command>");
    println!("  vida --help");
    println!();
    println!("Root commands:");
    for command in ROOT_COMMANDS {
        println!("  {command}");
    }
    println!();
    println!("Binary Foundation only exposes the frozen root command surface.");
}

fn print_boot_help() {
    println!("vida boot");
    println!();
    println!("Usage:");
    println!("  vida boot");
    println!("  vida boot --help");
    println!();
    println!("Binary Foundation behavior:");
    println!("  Emits a minimal boot scaffold confirmation.");
}

fn print_stub_help(command: &str) {
    println!("vida {command}");
    println!();
    println!("Usage:");
    println!("  vida {command} --help");
    println!();
    println!("Binary Foundation behavior:");
    println!("  Reserved root command family.");
    println!("  Full semantics are deferred to later implementation waves.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;

    #[test]
    fn temp_state_harness_creates_and_cleans_directory() {
        let path = {
            let harness = TempStateHarness::new().expect("temp state harness should initialize");
            let path = harness.path().to_path_buf();
            assert!(path.exists());
            path
        };

        assert!(!path.exists());
    }

    #[test]
    fn boot_command_succeeds() {
        assert_eq!(run(vec!["boot".into()]), ExitCode::SUCCESS);
    }

    #[test]
    fn stub_commands_fail_closed_without_help() {
        for command in ["task", "memory", "status", "doctor"] {
            assert_eq!(run(vec![command.into()]), ExitCode::from(2));
        }
    }
}

