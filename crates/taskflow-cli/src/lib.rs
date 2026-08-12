use clap::{CommandFactory, Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const AFTER_HELP: &str = "Standalone TaskFlow wrapper.\n\nExamples:\n  taskflow help\n  taskflow help parallelism\n  taskflow validate-routing --json\n  taskflow consume agent-system --json\n  taskflow scheduler dispatch --json\n\nAgent startup:\n  1. Read AGENTS.md and AGENTS.sidecar.md.\n  2. Run `vida orchestrator-init --json` for root lanes or `vida agent-init --json` for worker lanes.\n  3. Use JSON output for blockers, receipts, and next_actions.\n\nBehavior:\n  This binary safely delegates to `vida taskflow ...` without a shell.\n  If `VIDA_STATE_DIR` is unset, it resolves the current project root and binds\n  `<project-root>/.vida/data/state` before delegation.\n  Unknown or unsupported subcommands are not reinterpreted locally; they are\n  passed through to the launcher-owned TaskFlow surface as-is.\n\nEnvironment:\n  VIDA_TASKFLOW_VIDA_BIN  Override the VIDA executable used for delegation.";

#[derive(Debug, Parser)]
#[command(
    name = "taskflow",
    version = env!("CARGO_PKG_VERSION"),
    disable_help_subcommand = true,
    about = "Standalone TaskFlow CLI wrapper",
    long_about = "Standalone TaskFlow CLI wrapper\n\nThis binary provides a stable `taskflow` entrypoint while the canonical TaskFlow command family remains implemented under `vida taskflow`.",
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<CommandKind>,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    #[command(about = "Show launcher-owned TaskFlow help or one bounded help topic")]
    Help(HelpArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Debug, clap::Args)]
struct HelpArgs {
    #[arg()]
    topic: Option<String>,
}

pub fn run(cli: Cli) -> ExitCode {
    match cli.command {
        None => print_root_help(),
        Some(CommandKind::Help(args)) => delegate_help(args.topic.as_deref()),
        Some(CommandKind::External(args)) => delegate_taskflow(&args),
    }
}

fn print_root_help() -> ExitCode {
    let mut command = Cli::command();
    match command.print_help() {
        Ok(()) => {
            println!();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn delegate_help(topic: Option<&str>) -> ExitCode {
    let mut args = vec!["help".to_string()];
    if let Some(topic) = topic.filter(|value| !value.trim().is_empty()) {
        args.push(topic.to_string());
    }
    delegate_taskflow(&args)
}

fn delegate_taskflow(args: &[String]) -> ExitCode {
    let vida = resolve_vida_binary();
    let mut command = Command::new(&vida);
    command.arg("taskflow");
    command.args(args);
    command.stdin(std::process::Stdio::inherit());
    command.stdout(std::process::Stdio::inherit());
    command.stderr(std::process::Stdio::inherit());

    if std::env::var_os("VIDA_STATE_DIR").is_none() && command_needs_project_root(args) {
        match resolve_runtime_project_root() {
            Ok(project_root) => {
                command.env("VIDA_STATE_DIR", project_root.join(".vida/data/state"));
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }

    match command.status() {
        Ok(status) => exit_code_from_status(status.code()),
        Err(error) => {
            eprintln!(
                "taskflow\n  status: blocked\n  blocker_codes: [vida_runtime_unavailable]\n  attempted_runtime: {}\n  error: {}\n  next_actions:\n    - install the bundled VIDA runtime\n    - set VIDA_TASKFLOW_VIDA_BIN to the vida executable path\n    - run `vida taskflow help` to verify the canonical TaskFlow surface",
                vida.display(),
                error
            );
            ExitCode::from(1)
        }
    }
}

fn resolve_vida_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("VIDA_TASKFLOW_VIDA_BIN")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(parent) = current_exe.parent()
    {
        let candidate = parent.join(exe_name("vida"));
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(exe_name("vida"))
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn exit_code_from_status(code: Option<i32>) -> ExitCode {
    match code {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::from(1),
    }
}

fn command_needs_project_root(args: &[String]) -> bool {
    !matches!(
        args.first().map(String::as_str),
        None | Some("help" | "--help" | "-h")
    )
}

fn resolve_runtime_project_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    find_project_root(&cwd).ok_or_else(|| {
        format!(
            "Unable to resolve VIDA project root from `{}`. Run from an activated VIDA project (requires AGENTS.md, vida.config.yaml, and .vida/{{config,db,project}}) or set VIDA_STATE_DIR explicitly.",
            cwd.display()
        )
    })
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    for candidate in start.ancestors() {
        if looks_like_project_root(candidate) {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

fn looks_like_project_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file()
        && path.join("vida.config.yaml").is_file()
        && path.join(".vida/config").is_dir()
        && path.join(".vida/db").is_dir()
        && path.join(".vida/project").is_dir()
}

#[cfg(test)]
mod tests {
    use super::{
        command_needs_project_root, exit_code_from_status, find_project_root,
        looks_like_project_root,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("taskflow-cli-contract-{nonce}"));
        fs::create_dir_all(root.join(".vida/config")).expect("config directory should exist");
        fs::create_dir_all(root.join(".vida/db")).expect("db directory should exist");
        fs::create_dir_all(root.join(".vida/project")).expect("project directory should exist");
        fs::write(root.join("AGENTS.md"), "agents").expect("agents marker should exist");
        fs::write(root.join("vida.config.yaml"), "{}").expect("config marker should exist");
        root
    }

    #[test]
    fn project_root_requirement_distinguishes_help_from_runtime_commands() {
        assert!(!command_needs_project_root(&[]));
        assert!(!command_needs_project_root(&[
            "help".into(),
            "topic".into()
        ]));
        assert!(!command_needs_project_root(&["--help".into()]));
        assert!(!command_needs_project_root(&["-h".into()]));
        assert!(command_needs_project_root(&[
            "consume".into(),
            "agent-system".into()
        ]));
    }

    #[test]
    fn exit_code_mapping_preserves_valid_codes_and_blocks_invalid_statuses() {
        assert_eq!(
            exit_code_from_status(Some(0)),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(
            exit_code_from_status(Some(255)),
            std::process::ExitCode::from(255)
        );
        assert_eq!(
            exit_code_from_status(Some(-1)),
            std::process::ExitCode::from(1)
        );
        assert_eq!(
            exit_code_from_status(Some(256)),
            std::process::ExitCode::from(1)
        );
        assert_eq!(exit_code_from_status(None), std::process::ExitCode::from(1));
    }

    #[test]
    fn project_root_detection_requires_all_activation_markers_and_walks_ancestors() {
        let root = test_root();
        let nested = root.join("nested/work");
        fs::create_dir_all(&nested).expect("nested project path should exist");

        assert!(looks_like_project_root(&root));
        assert_eq!(find_project_root(&nested), Some(root.clone()));

        fs::remove_dir_all(root.join(".vida/project")).expect("project marker should be removable");
        assert!(!looks_like_project_root(&root));
        assert_eq!(find_project_root(&nested), None);
        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn project_root_detection_rejects_each_missing_activation_marker() {
        let root = test_root();
        let markers = [
            ("AGENTS.md", false),
            ("vida.config.yaml", false),
            (".vida/config", true),
            (".vida/db", true),
            (".vida/project", true),
        ];

        for (relative, is_directory) in markers {
            let marker = root.join(relative);
            if is_directory {
                fs::remove_dir_all(&marker).expect("directory marker should be removable");
            } else {
                fs::remove_file(&marker).expect("file marker should be removable");
            }
            assert!(
                !looks_like_project_root(&root),
                "missing marker should block project-root detection: {}",
                marker.display()
            );
            if is_directory {
                fs::create_dir_all(&marker).expect("directory marker should be restorable");
            } else {
                fs::write(&marker, "marker").expect("file marker should be restorable");
            }
        }

        assert!(looks_like_project_root(&root));
        fs::remove_dir_all(root).expect("test root should be removed");
    }
}
