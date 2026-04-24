use std::process::ExitCode;

use super::{
    Cli, Command, ReleaseCommand, TaskArgs, TaskCommand, agent_feedback_surface, approval_surface,
    docflow_proxy, doctor_surface, init_surfaces, lane_surface, memory_surface, print_root_help,
    project_activator_surface, protocol_surface, release_surface, resolve_runtime_project_root,
    run_taskflow_proxy, state_store, status_surface, task_surface,
};

pub(crate) async fn run_root_command(cli: Cli) -> ExitCode {
    if let Some(error) = prepare_runtime_state_dir(&cli.command) {
        eprintln!("{error}");
        return ExitCode::from(1);
    }

    match cli.command {
        None => {
            print_root_help();
            ExitCode::SUCCESS
        }
        Some(Command::Init(args)) => init_surfaces::run_init(args).await,
        Some(Command::Boot(args)) => init_surfaces::run_boot(args).await,
        Some(Command::OrchestratorInit(args)) => init_surfaces::run_orchestrator_init(args).await,
        Some(Command::AgentInit(args)) => init_surfaces::run_agent_init(args).await,
        Some(Command::Protocol(args)) => protocol_surface::run_protocol(args).await,
        Some(Command::ProjectActivator(args)) => {
            project_activator_surface::run_project_activator(args).await
        }
        Some(Command::AgentFeedback(args)) => {
            agent_feedback_surface::run_agent_feedback(args).await
        }
        Some(Command::Task(args)) => task_surface::run_task(args).await,
        Some(Command::Memory(args)) => memory_surface::run_memory(args).await,
        Some(Command::Status(args)) => status_surface::run_status(args).await,
        Some(Command::Doctor(args)) => doctor_surface::run_doctor(args).await,
        Some(Command::Consume(args)) => {
            let mut prefixed = vec!["consume".to_string()];
            prefixed.extend(args.args);
            run_taskflow_proxy(super::ProxyArgs { args: prefixed }).await
        }
        Some(Command::Lane(args)) => lane_surface::run_lane(args).await,
        Some(Command::Approval(args)) => approval_surface::run_approval(args).await,
        Some(Command::Recovery(args)) => {
            let mut prefixed = vec!["recovery".to_string()];
            prefixed.extend(args.args);
            run_taskflow_proxy(super::ProxyArgs { args: prefixed }).await
        }
        Some(Command::Release(args)) => match args.command {
            ReleaseCommand::Install(args) => release_surface::run_release_install(args),
        },
        Some(Command::Taskflow(args)) => run_taskflow_proxy(args).await,
        Some(Command::Docflow(args)) => docflow_proxy::run_docflow_proxy(args),
        Some(Command::External(args)) => run_unknown(&args),
    }
}

fn task_command_needs_project_root(args: &TaskArgs) -> bool {
    !matches!(args.command, TaskCommand::Help(_))
}

fn proxy_command_needs_project_root(args: &[String]) -> bool {
    !matches!(
        args.first().map(String::as_str),
        None | Some("help" | "--help" | "-h")
    )
}

fn command_needs_project_root_state_dir(command: &Option<Command>) -> bool {
    match command {
        Some(Command::Task(args)) => task_command_needs_project_root(args),
        Some(Command::Taskflow(args) | Command::Consume(args) | Command::Recovery(args)) => {
            proxy_command_needs_project_root(&args.args)
        }
        Some(
            Command::OrchestratorInit(_)
            | Command::AgentInit(_)
            | Command::ProjectActivator(_)
            | Command::AgentFeedback(_)
            | Command::Memory(_)
            | Command::Status(_)
            | Command::Doctor(_)
            | Command::Lane(_)
            | Command::Approval(_),
        ) => true,
        _ => false,
    }
}

fn prepare_runtime_state_dir(command: &Option<Command>) -> Option<String> {
    if std::env::var_os("VIDA_STATE_DIR").is_some() {
        return None;
    }

    if !command_needs_project_root_state_dir(command) {
        return None;
    }

    match resolve_runtime_project_root() {
        Ok(project_root) => {
            std::env::set_var(
                "VIDA_STATE_DIR",
                project_root.join(state_store::default_state_dir()),
            );
            None
        }
        Err(error) => Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, command_needs_project_root_state_dir, prepare_runtime_state_dir};
    use crate::temp_state::TempStateHarness;
    use clap::Parser;
    use std::fs;

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn make_project_root(root: &std::path::Path) {
        fs::create_dir_all(root.join(".vida/config")).expect("config dir should exist");
        fs::create_dir_all(root.join(".vida/db")).expect("db dir should exist");
        fs::create_dir_all(root.join(".vida/project")).expect("project dir should exist");
        fs::write(root.join("AGENTS.md"), "# bootstrap\n").expect("AGENTS.md should exist");
        fs::write(root.join("vida.config.yaml"), "project:\n  id: demo\n")
            .expect("config should exist");
    }

    #[test]
    fn prepare_runtime_state_dir_normalizes_project_bound_status_surface() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from(["vida", "status"]).expect("status cli should parse");

        assert!(command_needs_project_root_state_dir(&cli.command));
        assert_eq!(prepare_runtime_state_dir(&cli.command), None);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(harness.path().join(crate::state_store::default_state_dir()))
        );
    }

    #[test]
    fn prepare_runtime_state_dir_keeps_boot_permissive_for_temp_roots() {
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from(["vida", "boot"]).expect("boot cli should parse");

        assert!(!command_needs_project_root_state_dir(&cli.command));
        assert_eq!(prepare_runtime_state_dir(&cli.command), None);
        assert!(std::env::var_os("VIDA_STATE_DIR").is_none());
    }
}

fn run_unknown(args: &[String]) -> ExitCode {
    let command = args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unknown command family `{command}`. Use `vida --help` to inspect the frozen root surface."
    );
    ExitCode::from(2)
}
