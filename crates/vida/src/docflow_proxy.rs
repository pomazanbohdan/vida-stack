use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use docflow_cli::Cli as DocflowCli;

use crate::taskflow_spec_bootstrap::run_docflow_cli_command;

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
            match run_docflow_cli_command(&project_root, &args.args) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
