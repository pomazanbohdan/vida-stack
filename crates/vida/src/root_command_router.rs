use std::process::ExitCode;

use super::{
    agent_dispatch_surface, agent_feedback_surface, approval_surface, diagnostics_surface,
    docflow_proxy, docs_surface, doctor_surface, init_surfaces, lane_surface, memory_surface,
    orchestrator_session_surface, print_root_help, project_activator_surface, protocol_surface,
    release_surface, resolve_runtime_project_root, run_taskflow_proxy, service_client_cli,
    state_store, status_surface, task_surface, AgentArgs, AgentCommand, Cli, Command,
    ReleaseCommand, TaskArgs, TaskCommand,
};

pub(crate) async fn run_root_command(cli: Cli) -> ExitCode {
    let mut timing =
        crate::command_lifecycle_hooks::CommandTimingContext::from_env(command_label(&cli.command));
    let pre_execution_started = std::time::Instant::now();
    let _runtime_state_dir_guard = match prepare_runtime_state_dir(&cli.command) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    timing.record_phase(
        crate::command_lifecycle_hooks::CommandPhase::PreExecution,
        pre_execution_started.elapsed(),
    );

    let execution_started = std::time::Instant::now();
    let exit_code = match cli.command {
        None => {
            print_root_help();
            ExitCode::SUCCESS
        }
        Some(Command::Init(args)) => init_surfaces::run_init(args).await,
        Some(Command::Boot(args)) => init_surfaces::run_boot(args).await,
        Some(Command::OrchestratorInit(args)) => init_surfaces::run_orchestrator_init(args).await,
        Some(Command::AgentInit(args)) => init_surfaces::run_agent_init(args).await,
        Some(Command::Agent(args)) => agent_dispatch_surface::run_agent(args).await,
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
        Some(Command::Diagnostics(args)) => diagnostics_surface::run_diagnostics(args).await,
        Some(Command::Service(args)) => service_client_cli::run_service(args),
        Some(Command::Project(args)) => service_client_cli::run_project(args),
        Some(Command::Wizard(args)) => service_client_cli::run_wizard(args),
        Some(Command::Job(args)) => service_client_cli::run_job(args),
        Some(Command::Receipt(args)) => service_client_cli::run_receipt(args),
        Some(Command::Docs(args)) => docs_surface::run_docs(args).await,
        Some(Command::OrchestratorSession(args)) => {
            orchestrator_session_surface::run_orchestrator_session(args).await
        }
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
        Some(Command::Route(args)) => {
            let mut prefixed = vec!["route".to_string()];
            prefixed.extend(args.args);
            run_taskflow_proxy(super::ProxyArgs { args: prefixed }).await
        }
        Some(Command::Release(args)) => match args.command {
            ReleaseCommand::Install(args) => release_surface::run_release_install(args),
        },
        Some(Command::Taskflow(args)) => run_taskflow_proxy(args).await,
        Some(Command::Docflow(args)) => docflow_proxy::run_docflow_proxy(args),
        Some(Command::External(args)) => run_unknown(&args),
    };
    timing.record_phase(
        crate::command_lifecycle_hooks::CommandPhase::Execution,
        execution_started.elapsed(),
    );
    timing.finalize_and_emit(exit_code);
    exit_code
}

fn command_label(command: &Option<Command>) -> String {
    match command {
        None => "vida".to_string(),
        Some(Command::Init(_)) => "vida init".to_string(),
        Some(Command::Boot(_)) => "vida boot".to_string(),
        Some(Command::OrchestratorInit(_)) => "vida orchestrator-init".to_string(),
        Some(Command::AgentInit(_)) => "vida agent-init".to_string(),
        Some(Command::Agent(_)) => "vida agent".to_string(),
        Some(Command::Protocol(_)) => "vida protocol".to_string(),
        Some(Command::ProjectActivator(_)) => "vida project-activator".to_string(),
        Some(Command::AgentFeedback(_)) => "vida agent-feedback".to_string(),
        Some(Command::Task(_)) => "vida task".to_string(),
        Some(Command::Memory(_)) => "vida memory".to_string(),
        Some(Command::Status(_)) => "vida status".to_string(),
        Some(Command::Doctor(_)) => "vida doctor".to_string(),
        Some(Command::Diagnostics(_)) => "vida diagnostics".to_string(),
        Some(Command::Service(_)) => "vida service".to_string(),
        Some(Command::Project(_)) => "vida project".to_string(),
        Some(Command::Wizard(_)) => "vida wizard".to_string(),
        Some(Command::Job(_)) => "vida job".to_string(),
        Some(Command::Receipt(_)) => "vida receipt".to_string(),
        Some(Command::Docs(_)) => "vida docs".to_string(),
        Some(Command::OrchestratorSession(_)) => "vida orchestrator-session".to_string(),
        Some(Command::Consume(_)) => "vida consume".to_string(),
        Some(Command::Lane(_)) => "vida lane".to_string(),
        Some(Command::Approval(_)) => "vida approval".to_string(),
        Some(Command::Recovery(_)) => "vida recovery".to_string(),
        Some(Command::Route(_)) => "vida route".to_string(),
        Some(Command::Release(_)) => "vida release".to_string(),
        Some(Command::Taskflow(_)) => "vida taskflow".to_string(),
        Some(Command::Docflow(_)) => "vida docflow".to_string(),
        Some(Command::External(args)) => args
            .first()
            .map(|name| format!("vida {name}"))
            .unwrap_or_else(|| "vida external".to_string()),
    }
}

fn task_command_has_explicit_state_dir(args: &TaskArgs) -> bool {
    match &args.command {
        TaskCommand::ImportJsonl(command) => command.state_dir.is_some(),
        TaskCommand::ReplaceJsonl(command) => command.state_dir.is_some(),
        TaskCommand::ExportJsonl(command) => command.state_dir.is_some(),
        TaskCommand::List(command) => command.state_dir.is_some(),
        TaskCommand::Show(command) => command.state_dir.is_some(),
        TaskCommand::Progress(command) => command.state_dir.is_some(),
        TaskCommand::Ready(command) => command.state_dir.is_some(),
        TaskCommand::Next(command) => command.state_dir.is_some(),
        TaskCommand::NextLawful(command) => command.state_dir.is_some(),
        TaskCommand::NextDisplayId(command) => command.state_dir.is_some(),
        TaskCommand::Create(command) | TaskCommand::Ensure(command) => command.state_dir.is_some(),
        TaskCommand::Update(command) => command.state_dir.is_some(),
        TaskCommand::OwnedStatus(command) => command.state_dir.is_some(),
        TaskCommand::Close(command) => command.state_dir.is_some(),
        TaskCommand::ReconcileClosedRuns(command) => command.state_dir.is_some(),
        TaskCommand::Split(command) => command.state_dir.is_some(),
        TaskCommand::SpawnBlocker(command) => command.state_dir.is_some(),
        TaskCommand::Deps(command)
        | TaskCommand::ReverseDeps(command)
        | TaskCommand::Children(command)
        | TaskCommand::Tree(command) => command.state_dir.is_some(),
        TaskCommand::ReparentChildren(command) => command.state_dir.is_some(),
        TaskCommand::DefectBatchRehome(command) => command.state_dir.is_some(),
        TaskCommand::Blocked(command)
        | TaskCommand::ValidateGraph(command)
        | TaskCommand::CriticalPath(command) => command.state_dir.is_some(),
        TaskCommand::Dep(command) => match &command.command {
            super::TaskDependencyCommand::Add(command) => command.state_dir.is_some(),
            super::TaskDependencyCommand::AddBulk(command) => command.state_dir.is_some(),
            super::TaskDependencyCommand::Remove(command) => command.state_dir.is_some(),
        },
        TaskCommand::Handoff(command) => match &command.command {
            super::TaskHandoffCommand::Accept(command) => command.state_dir.is_some(),
        },
        TaskCommand::Help(_) | TaskCommand::AdaptivePreview(_) => false,
    }
}

fn task_command_needs_project_root(args: &TaskArgs) -> bool {
    !matches!(args.command, TaskCommand::Help(_)) && !task_command_has_explicit_state_dir(args)
}

fn agent_command_needs_project_root(args: &AgentArgs) -> bool {
    match &args.command {
        AgentCommand::DispatchNext(command) => command.state_dir.is_none(),
        AgentCommand::Select(command) => command.state_dir.is_none(),
    }
}

fn proxy_args_request_help_or_version(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "help" | "--help" | "-h" | "--version" | "-V"))
}

fn proxy_command_needs_project_root(args: &[String]) -> bool {
    !proxy_args_request_help_or_version(args)
}

pub(crate) fn command_needs_project_root_state_dir(command: &Option<Command>) -> bool {
    match command {
        Some(Command::Task(args)) => task_command_needs_project_root(args),
        Some(Command::Agent(args)) => agent_command_needs_project_root(args),
        Some(
            Command::Taskflow(args)
            | Command::Consume(args)
            | Command::Recovery(args)
            | Command::Route(args),
        ) => proxy_command_needs_project_root(&args.args),
        Some(Command::Lane(args) | Command::Approval(args)) => {
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
            | Command::Diagnostics(_)
            | Command::Service(_)
            | Command::Project(_)
            | Command::Wizard(_)
            | Command::Job(_)
            | Command::Receipt(_)
            | Command::OrchestratorSession(_),
        ) => true,
        _ => false,
    }
}

pub(crate) struct RuntimeStateDirGuard {
    previous: Option<std::ffi::OsString>,
    active: bool,
}

impl Drop for RuntimeStateDirGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Some(previous) = &self.previous {
            std::env::set_var("VIDA_STATE_DIR", previous);
        } else {
            std::env::remove_var("VIDA_STATE_DIR");
        }
    }
}

fn prepare_runtime_state_dir(
    command: &Option<Command>,
) -> Result<Option<RuntimeStateDirGuard>, String> {
    if std::env::var_os("VIDA_STATE_DIR").is_some() {
        return Ok(normalize_runtime_state_dir_env_for_parse());
    }

    if !command_needs_project_root_state_dir(command) {
        return Ok(None);
    }

    match resolve_runtime_project_root() {
        Ok(project_root) => {
            let previous = std::env::var_os("VIDA_STATE_DIR");
            std::env::set_var(
                "VIDA_STATE_DIR",
                project_root.join(state_store::default_state_dir()),
            );
            Ok(Some(RuntimeStateDirGuard {
                previous,
                active: true,
            }))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn normalize_runtime_state_dir_env_for_parse() -> Option<RuntimeStateDirGuard> {
    let existing = std::env::var_os("VIDA_STATE_DIR")?;
    let existing_path = std::path::PathBuf::from(&existing);
    let normalized = normalize_runtime_state_dir_override(&existing_path)?;
    if normalized == existing_path {
        return None;
    }
    std::env::set_var("VIDA_STATE_DIR", normalized);
    Some(RuntimeStateDirGuard {
        previous: Some(existing),
        active: true,
    })
}

fn normalize_runtime_state_dir_override(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let file_name = path.file_name().and_then(|value| value.to_str())?;
    if file_name != ".vida" {
        return None;
    }
    let canonical_state = path.join("data").join("state");
    canonical_state.is_dir().then_some(canonical_state)
}

#[cfg(test)]
mod tests {
    use super::{
        command_needs_project_root_state_dir, normalize_runtime_state_dir_env_for_parse,
        prepare_runtime_state_dir, Cli,
    };
    use crate::temp_state::TempStateHarness;
    use crate::Command;
    use clap::Parser;
    use std::fs;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }

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
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        fs::create_dir_all(harness.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from(["vida", "status"]).expect("status cli should parse");

        assert!(command_needs_project_root_state_dir(&cli.command));
        let guard =
            prepare_runtime_state_dir(&cli.command).expect("state dir preparation should succeed");
        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(harness.path().join(crate::state_store::default_state_dir()))
        );
        drop(guard);
        assert!(std::env::var_os("VIDA_STATE_DIR").is_none());
    }

    #[test]
    fn prepare_runtime_state_dir_normalizes_legacy_project_vida_env_override() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        fs::create_dir_all(harness.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let legacy_state_dir = harness.path().join(".vida");
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &legacy_state_dir);
        let cli = Cli::try_parse_from(["vida", "status"]).expect("status cli should parse");

        let guard =
            prepare_runtime_state_dir(&cli.command).expect("state dir preparation should succeed");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(harness.path().join(crate::state_store::default_state_dir()))
        );
        drop(guard);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(legacy_state_dir)
        );
    }

    #[test]
    fn normalize_runtime_state_dir_env_for_parse_updates_clap_env_args() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        fs::create_dir_all(harness.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let legacy_state_dir = harness.path().join(".vida");
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &legacy_state_dir);

        let guard = normalize_runtime_state_dir_env_for_parse()
            .expect("legacy project .vida env override should normalize before parse");
        let cli = Cli::try_parse_from(["vida", "status"]).expect("status cli should parse");

        match cli.command {
            Some(Command::Status(args)) => assert_eq!(
                args.state_dir,
                Some(harness.path().join(crate::state_store::default_state_dir()))
            ),
            other => panic!("expected status command, got {other:?}"),
        }
        drop(guard);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(legacy_state_dir)
        );
    }

    #[test]
    fn prepare_runtime_state_dir_keeps_boot_permissive_for_temp_roots() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from(["vida", "boot"]).expect("boot cli should parse");

        assert!(!command_needs_project_root_state_dir(&cli.command));
        assert!(prepare_runtime_state_dir(&cli.command)
            .expect("state dir preparation should succeed")
            .is_none());
        assert!(std::env::var_os("VIDA_STATE_DIR").is_none());
    }

    #[test]
    fn prepare_runtime_state_dir_skips_project_root_for_explicit_task_state_dir() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from([
            "vida",
            "task",
            "next-lawful",
            "--state-dir",
            "C:/tmp/isolated-state",
            "--json",
        ])
        .expect("task next-lawful cli should parse");

        assert!(!command_needs_project_root_state_dir(&cli.command));
        assert!(prepare_runtime_state_dir(&cli.command)
            .expect("state dir preparation should succeed")
            .is_none());
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
