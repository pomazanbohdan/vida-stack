use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use docflow_cli::Cli as DocflowCli;

use crate::taskflow_spec_bootstrap::run_docflow_cli_command_with_exit;

use super::{resolve_repo_root, ProxyArgs};

fn proxy_requested_help(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("help") | Some("--help") | Some("-h")
    )
}

fn print_docflow_proxy_help() {
    println!("VIDA DocFlow runtime family");
    println!();
    println!("Mode-scoped launcher contract:");
    println!(
        "  repo/dev binary mode: vida routes the active DocFlow command map in-process through the Rust CLI."
    );
    println!("  installed mode: vida keeps the same in-process Rust DocFlow shell.");
    println!(
        "  unsupported commands fail closed instead of silently falling through to donor wrappers."
    );
    println!();
    println!("Implemented in-process command surface:");
    let mut command = DocflowCli::command();
    let help = command.render_long_help().to_string();
    print!("{help}");
    if !help.ends_with('\n') {
        println!();
    }
}

pub(crate) fn run_docflow_proxy(args: ProxyArgs) -> ExitCode {
    if proxy_requested_help(&args.args) {
        print_docflow_proxy_help();
        return ExitCode::SUCCESS;
    }

    let argv = std::iter::once("docflow".to_string())
        .chain(args.args.clone())
        .collect::<Vec<_>>();

    match DocflowCli::try_parse_from(argv.clone()) {
        Ok(_cli) => {
            let project_root = match resolve_repo_root() {
                Ok(project_root) => project_root,
                Err(_) => match std::env::current_dir() {
                    Ok(current_dir) => current_dir,
                    Err(error) => {
                        eprintln!("Failed to resolve current directory: {error}");
                        return ExitCode::from(1);
                    }
                },
            };
            match run_docflow_cli_command_with_exit(&project_root, &args.args) {
                Ok(result) => {
                    println!("{}", result.output);
                    return result.exit_code;
                }
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            }
        }
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                if let Err(print_error) = error.print() {
                    eprintln!("{print_error}");
                    return ExitCode::from(1);
                }
                return ExitCode::SUCCESS;
            }
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::{proxy_requested_help, run_docflow_proxy};
    use crate::ProxyArgs;

    fn proxy_args(args: &[&str]) -> ProxyArgs {
        ProxyArgs {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    #[test]
    fn proxy_help_detection_accepts_empty_help_and_flags() {
        assert!(proxy_requested_help(&[]));
        assert!(proxy_requested_help(&["help".to_string()]));
        assert!(proxy_requested_help(&["--help".to_string()]));
        assert!(proxy_requested_help(&["-h".to_string()]));
        assert!(!proxy_requested_help(&["overview".to_string()]));
    }

    #[test]
    fn docflow_proxy_returns_success_for_help_version_and_overview() {
        assert_eq!(run_docflow_proxy(proxy_args(&["help"])), ExitCode::SUCCESS);
        assert_eq!(
            run_docflow_proxy(proxy_args(&["--version"])),
            ExitCode::SUCCESS
        );
        assert_eq!(
            run_docflow_proxy(proxy_args(&[
                "overview",
                "--registry-count",
                "2",
                "--relation-count",
                "1",
            ])),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn docflow_proxy_rejects_unknown_docflow_commands() {
        assert_eq!(
            run_docflow_proxy(proxy_args(&["unknown-docflow-command"])),
            ExitCode::from(2)
        );
    }
}
