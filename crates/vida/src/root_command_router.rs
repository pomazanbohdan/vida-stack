use std::{ffi::OsString, process::ExitCode};

use super::{
    agent_dispatch_surface, agent_feedback_surface, approval_surface, diagnostics_surface,
    docflow_proxy, docs_surface, doctor_surface, init_surfaces, lane_surface, memory_surface,
    orchestrator_session_surface, print_root_help, project_activator_surface, proof_surface,
    protocol_surface, quality_surface, release_surface, run_taskflow_proxy, runtime_web_surface,
    service_client_cli, session_surface, status_surface, task_surface, AgentArgs, AgentCommand,
    Cli, CoderCommand, Command, ReleaseCommand, SessionArgs, SessionCommand, TaskArgs, TaskCommand,
};
use crate::root_state_binding::{
    bind_runtime_state_dir_for_project_bound_command,
    bind_runtime_state_dir_override_for_project_bound_command,
    normalize_runtime_state_dir_env_for_parse, preserve_runtime_state_dir_env_for_parse_only,
    preserve_runtime_state_dir_env_for_project_bound_command, RuntimeStateDirGuard,
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
        Some(Command::Coder(args)) => match args.command {
            CoderCommand::Capabilities(args) => vida::run_coder_capabilities(args.json),
            CoderCommand::ProviderCheck(args) => {
                vida::run_coder_provider_check(&args.provider, args.json)
            }
            CoderCommand::Run(args) => {
                vida::run_coder(&args.provider, args.request.as_deref(), args.json)
            }
        },
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
        Some(Command::Runtime(args)) => runtime_web_surface::run_runtime(args).await,
        Some(Command::Doctor(args)) => doctor_surface::run_doctor(args).await,
        Some(Command::Diagnostics(args)) => diagnostics_surface::run_diagnostics(args).await,
        Some(Command::Proof(args)) => proof_surface::run_proof(args).await,
        Some(Command::Service(args)) => service_client_cli::run_service(args),
        Some(Command::Project(args)) => service_client_cli::run_project(args),
        Some(Command::Wizard(args)) => service_client_cli::run_wizard(args),
        Some(Command::Job(args)) => service_client_cli::run_job(args),
        Some(Command::Receipt(args)) => service_client_cli::run_receipt(args),
        Some(Command::Docs(args)) => docs_surface::run_docs(args).await,
        Some(Command::OrchestratorSession(args)) => {
            orchestrator_session_surface::run_orchestrator_session(args).await
        }
        Some(Command::Session(args)) => session_surface::run_session(args).await,
        Some(Command::Quality(args)) => quality_surface::run_quality(args).await,
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
        Some(Command::Coder(_)) => "vida coder".to_string(),
        Some(Command::Protocol(_)) => "vida protocol".to_string(),
        Some(Command::ProjectActivator(_)) => "vida project-activator".to_string(),
        Some(Command::AgentFeedback(_)) => "vida agent-feedback".to_string(),
        Some(Command::Task(_)) => "vida task".to_string(),
        Some(Command::Memory(_)) => "vida memory".to_string(),
        Some(Command::Status(_)) => "vida status".to_string(),
        Some(Command::Runtime(_)) => "vida runtime".to_string(),
        Some(Command::Doctor(_)) => "vida doctor".to_string(),
        Some(Command::Diagnostics(_)) => "vida diagnostics".to_string(),
        Some(Command::Proof(_)) => "vida proof".to_string(),
        Some(Command::Service(_)) => "vida service".to_string(),
        Some(Command::Project(_)) => "vida project".to_string(),
        Some(Command::Wizard(_)) => "vida wizard".to_string(),
        Some(Command::Job(_)) => "vida job".to_string(),
        Some(Command::Receipt(_)) => "vida receipt".to_string(),
        Some(Command::Docs(_)) => "vida docs".to_string(),
        Some(Command::OrchestratorSession(_)) => "vida orchestrator-session".to_string(),
        Some(Command::Session(_)) => "vida session".to_string(),
        Some(Command::Quality(_)) => "vida quality".to_string(),
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
        TaskCommand::Search(command) => command.state_dir.is_some(),
        TaskCommand::Show(command) => command.state_dir.is_some(),
        TaskCommand::Progress(command) => command.state_dir.is_some(),
        TaskCommand::ClosureReady(command) => command.state_dir.is_some(),
        TaskCommand::Proof(command) => match &command.command {
            super::TaskProofCommand::Status(command) => command.state_dir.is_some(),
            super::TaskProofCommand::AttachBrowser(command) => command.state_dir.is_some(),
        },
        TaskCommand::Ready(command) => command.state_dir.is_some(),
        TaskCommand::Next(command) => command.state_dir.is_some(),
        TaskCommand::NextLawful(command) => command.state_dir.is_some(),
        TaskCommand::NextDisplayId(command) => command.state_dir.is_some(),
        TaskCommand::Create(command) | TaskCommand::Ensure(command) => command.state_dir.is_some(),
        TaskCommand::Update(command) => command.state_dir.is_some(),
        TaskCommand::Note(command) => match &command.command {
            super::TaskNoteCommand::Append(command) => command.state_dir.is_some(),
        },
        TaskCommand::Block(command) => command.state_dir.is_some(),
        TaskCommand::Verify(command) => command.state_dir.is_some(),
        TaskCommand::Attempt(command) => match &command.command {
            super::TaskAttemptCommand::Dispatch(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Status(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Collect(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Consolidate(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Record(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Transition(command) => command.state_dir.is_some(),
            super::TaskAttemptCommand::Summary(command) => command.state_dir.is_some(),
        },
        TaskCommand::Stage(command) => match &command.command {
            super::TaskStageCommand::Status(command) => command.state_dir.is_some(),
        },
        TaskCommand::OwnedStatus(command) => command.state_dir.is_some(),
        TaskCommand::Close(command) => command.state_dir.is_some(),
        TaskCommand::Reconcile(command) => command.state_dir.is_some(),
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
            super::TaskDependencyCommand::Ensure(command) => command.state_dir.is_some(),
            super::TaskDependencyCommand::AddBulk(command) => command.state_dir.is_some(),
            super::TaskDependencyCommand::Remove(command) => command.state_dir.is_some(),
        },
        TaskCommand::Handoff(command) => match &command.command {
            super::TaskHandoffCommand::Accept(command) => command.state_dir.is_some(),
        },
        TaskCommand::Takeover(command) => match &command.command {
            super::TaskTakeoverCommand::Status(command) => command.state_dir.is_some(),
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
        AgentCommand::HostBridge(command) => command.state_dir.is_none(),
        AgentCommand::Status(command) => command.state_dir.is_none(),
    }
}

fn diagnostics_command_explicit_state_dir(
    args: &super::DiagnosticsArgs,
) -> Option<&std::path::Path> {
    match &args.command {
        super::DiagnosticsCommand::PostCommit(command) => command.state_dir.as_deref(),
        super::DiagnosticsCommand::EvidenceCheck(command) => command.state_dir.as_deref(),
        super::DiagnosticsCommand::RulesCheck(command) => command.state_dir.as_deref(),
    }
}

fn orchestrator_session_command_explicit_state_dir(
    args: &super::OrchestratorSessionArgs,
) -> Option<&std::path::Path> {
    match &args.command {
        super::OrchestratorSessionCommand::Show(command) => command.state_dir.as_deref(),
        super::OrchestratorSessionCommand::Reclaim(command) => command.state_dir.as_deref(),
        super::OrchestratorSessionCommand::Transfer(command) => command.state_dir.as_deref(),
    }
}

fn session_command_has_explicit_state_dir(args: &SessionArgs) -> bool {
    match &args.command {
        SessionCommand::Triage(command) => command.state_dir.is_some(),
    }
}

fn session_command_needs_project_root(args: &SessionArgs) -> bool {
    !session_command_has_explicit_state_dir(args)
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
        Some(Command::OrchestratorInit(args)) => args.state_dir.is_none(),
        Some(Command::AgentInit(args)) => args.state_dir.is_none(),
        Some(Command::ProjectActivator(args)) => args.state_dir.is_none(),
        Some(Command::Memory(args)) => args.state_dir.is_none(),
        Some(Command::Status(args)) => args.state_dir.is_none(),
        Some(Command::Diagnostics(args)) => diagnostics_command_explicit_state_dir(args).is_none(),
        Some(Command::OrchestratorSession(args)) => {
            orchestrator_session_command_explicit_state_dir(args).is_none()
        }
        Some(
            Command::AgentFeedback(_)
            | Command::Runtime(_)
            | Command::Proof(_)
            | Command::Service(_)
            | Command::Project(_)
            | Command::Wizard(_)
            | Command::Job(_)
            | Command::Receipt(_),
        ) => true,
        Some(Command::Session(args)) => session_command_needs_project_root(args),
        Some(Command::Doctor(args)) => args.state_dir.is_none(),
        _ => false,
    }
}

pub(crate) fn prepare_runtime_state_dir_for_parse(
    args: &[OsString],
) -> Result<Option<RuntimeStateDirGuard>, String> {
    if raw_args_are_agent_state_dir_bound_surface(args) {
        if let Some(state_dir) = raw_args_explicit_state_dir(args) {
            return bind_runtime_state_dir_override_for_project_bound_command(&state_dir);
        }
    }
    if raw_args_are_env_authoritative_state_surface(args)
        && std::env::var_os("VIDA_STATE_DIR").is_some()
    {
        return Ok(preserve_runtime_state_dir_env_for_project_bound_command());
    }
    if raw_args_are_env_parse_only_state_surface(args)
        && std::env::var_os("VIDA_STATE_DIR").is_some()
    {
        return Ok(preserve_runtime_state_dir_env_for_parse_only());
    }
    if raw_args_need_project_root_state_dir(args) {
        return bind_runtime_state_dir_for_project_bound_command();
    }
    if std::env::var_os("VIDA_STATE_DIR").is_some() {
        return Ok(normalize_runtime_state_dir_env_for_parse());
    }
    Ok(None)
}

fn prepare_runtime_state_dir(
    command: &Option<Command>,
) -> Result<Option<RuntimeStateDirGuard>, String> {
    if command_preserves_parse_only_env_state_dir(command)
        && std::env::var_os("VIDA_STATE_DIR").is_some()
    {
        return Ok(preserve_runtime_state_dir_env_for_parse_only());
    }
    if let Some(state_dir) = command_explicit_state_dir(command) {
        return bind_runtime_state_dir_override_for_project_bound_command(state_dir);
    }
    if command_preserves_explicit_env_state_dir(command)
        && std::env::var_os("VIDA_STATE_DIR").is_some()
    {
        return Ok(preserve_runtime_state_dir_env_for_project_bound_command());
    }
    if !command_needs_project_root_state_dir(command) {
        if std::env::var_os("VIDA_STATE_DIR").is_some() {
            return Ok(normalize_runtime_state_dir_env_for_parse());
        }
        return Ok(None);
    }

    bind_runtime_state_dir_for_project_bound_command()
}

fn command_explicit_state_dir(command: &Option<Command>) -> Option<&std::path::Path> {
    match command {
        Some(Command::Agent(AgentArgs {
            command: AgentCommand::DispatchNext(command),
        })) => command.state_dir.as_deref(),
        Some(Command::Agent(AgentArgs {
            command: AgentCommand::Select(command),
        })) => command.state_dir.as_deref(),
        Some(Command::Agent(AgentArgs {
            command: AgentCommand::HostBridge(command),
        })) => command.state_dir.as_deref(),
        Some(Command::Agent(AgentArgs {
            command: AgentCommand::Status(command),
        })) => command.state_dir.as_deref(),
        Some(Command::OrchestratorInit(command)) => command.state_dir.as_deref(),
        Some(Command::ProjectActivator(command)) => command.state_dir.as_deref(),
        Some(Command::Memory(command)) => command.state_dir.as_deref(),
        Some(Command::Status(command)) => command.state_dir.as_deref(),
        Some(Command::Doctor(command)) => command.state_dir.as_deref(),
        Some(Command::Diagnostics(command)) => diagnostics_command_explicit_state_dir(command),
        Some(Command::OrchestratorSession(command)) => {
            orchestrator_session_command_explicit_state_dir(command)
        }
        Some(Command::Session(SessionArgs {
            command: SessionCommand::Triage(command),
        })) => command.state_dir.as_deref(),
        _ => None,
    }
}

fn command_preserves_explicit_env_state_dir(command: &Option<Command>) -> bool {
    matches!(
        command,
        Some(Command::Status(_) | Command::Taskflow(_))
            | Some(Command::Consume(_) | Command::Recovery(_) | Command::Route(_))
            | Some(Command::Lane(_) | Command::Approval(_))
            | Some(Command::OrchestratorInit(_))
            | Some(Command::ProjectActivator(_) | Command::Memory(_))
            | Some(Command::Doctor(_) | Command::Diagnostics(_))
            | Some(Command::OrchestratorSession(_))
            | Some(Command::Session(_))
            | Some(Command::Agent(AgentArgs {
                command: AgentCommand::DispatchNext(_)
                    | AgentCommand::Select(_)
                    | AgentCommand::HostBridge(_)
                    | AgentCommand::Status(_)
            }))
    )
}

fn command_preserves_parse_only_env_state_dir(command: &Option<Command>) -> bool {
    matches!(command, Some(Command::AgentInit(_)))
}

fn raw_args_need_project_root_state_dir(args: &[OsString]) -> bool {
    let Some(command) = args
        .iter()
        .skip(1)
        .find_map(|arg| arg.to_str())
        .filter(|arg| !arg.starts_with('-'))
    else {
        return false;
    };
    if raw_args_request_help_or_version(args) || raw_args_have_explicit_state_dir(args) {
        return false;
    }
    matches!(
        command,
        "orchestrator-init"
            | "agent-init"
            | "agent"
            | "project-activator"
            | "agent-feedback"
            | "task"
            | "memory"
            | "status"
            | "runtime"
            | "doctor"
            | "diagnostics"
            | "proof"
            | "service"
            | "project"
            | "wizard"
            | "job"
            | "receipt"
            | "orchestrator-session"
            | "consume"
            | "lane"
            | "approval"
            | "recovery"
            | "route"
            | "taskflow"
    )
}

fn raw_args_are_env_authoritative_state_surface(args: &[OsString]) -> bool {
    if raw_args_request_help_or_version(args) || raw_args_have_explicit_state_dir(args) {
        return false;
    }
    let mut positional = args
        .iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .filter(|arg| !arg.starts_with('-'));
    match positional.next() {
        Some("agent") => matches!(
            positional.next(),
            Some("dispatch-next" | "select" | "status" | "host-bridge")
        ),
        Some(
            "orchestrator-init"
            | "task"
            | "taskflow"
            | "project-activator"
            | "memory"
            | "status"
            | "doctor"
            | "diagnostics"
            | "orchestrator-session"
            | "lane"
            | "approval",
        ) => true,
        Some("consume" | "recovery" | "route") => true,
        _ => false,
    }
}

fn raw_args_are_env_parse_only_state_surface(args: &[OsString]) -> bool {
    if raw_args_request_help_or_version(args) || raw_args_have_explicit_state_dir(args) {
        return false;
    }
    let Some(command) = args
        .iter()
        .skip(1)
        .find_map(|arg| arg.to_str())
        .filter(|arg| !arg.starts_with('-'))
    else {
        return false;
    };
    matches!(command, "agent-init")
}

fn raw_args_are_agent_state_dir_bound_surface(args: &[OsString]) -> bool {
    if raw_args_request_help_or_version(args) {
        return false;
    }
    let mut positional = args
        .iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .filter(|arg| !arg.starts_with('-'));
    matches!(
        (positional.next(), positional.next()),
        (
            Some("agent"),
            Some("dispatch-next" | "select" | "status" | "host-bridge")
        )
    )
}

fn raw_args_request_help_or_version(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .any(|arg| {
            matches!(
                arg,
                "help" | "--help" | "-h" | "--version" | "-V" | "--HELP" | "-H"
            )
        })
}

fn raw_args_have_explicit_state_dir(args: &[OsString]) -> bool {
    raw_args_explicit_state_dir(args).is_some()
}

fn raw_args_explicit_state_dir(args: &[OsString]) -> Option<std::path::PathBuf> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(arg) = arg.to_str() else {
            continue;
        };
        if arg == "--state-dir" {
            return iter.next().map(std::path::PathBuf::from);
        }
        if let Some(value) = arg.strip_prefix("--state-dir=") {
            return Some(std::path::PathBuf::from(value));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        command_needs_project_root_state_dir, normalize_runtime_state_dir_env_for_parse,
        prepare_runtime_state_dir, prepare_runtime_state_dir_for_parse, Cli,
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
    fn prepare_runtime_state_dir_preserves_explicit_env_for_status_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let explicit_state_dir = harness.path().join("explicit-state");
        fs::create_dir_all(&explicit_state_dir).expect("explicit state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &explicit_state_dir);
        let cli = Cli::try_parse_from(["vida", "status"]).expect("status cli should parse");

        let guard =
            prepare_runtime_state_dir(&cli.command).expect("state dir preparation should succeed");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(explicit_state_dir)
        );
    }

    #[test]
    fn prepare_runtime_state_dir_for_parse_preserves_explicit_env_for_status_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let explicit_state_dir = harness.path().join("explicit-state");
        fs::create_dir_all(&explicit_state_dir).expect("explicit state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &explicit_state_dir);
        let args = [
            std::ffi::OsString::from("vida"),
            std::ffi::OsString::from("status"),
        ];

        let guard = prepare_runtime_state_dir_for_parse(&args)
            .expect("pre-parse state dir preparation should succeed");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(explicit_state_dir)
        );
    }

    #[test]
    fn prepare_runtime_state_dir_for_parse_preserves_explicit_env_for_lane_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let explicit_state_dir = harness.path().join("explicit-state");
        fs::create_dir_all(&explicit_state_dir).expect("explicit state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &explicit_state_dir);
        let args = [
            std::ffi::OsString::from("vida"),
            std::ffi::OsString::from("lane"),
            std::ffi::OsString::from("show"),
            std::ffi::OsString::from("run-1"),
            std::ffi::OsString::from("--json"),
        ];

        let guard = prepare_runtime_state_dir_for_parse(&args)
            .expect("pre-parse state dir preparation should succeed");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(explicit_state_dir)
        );
    }

    #[test]
    fn prepare_runtime_state_dir_preserves_explicit_env_for_lane_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let explicit_state_dir = harness.path().join("explicit-state");
        fs::create_dir_all(&explicit_state_dir).expect("explicit state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &explicit_state_dir);
        let cli =
            Cli::try_parse_from(["vida", "lane", "show", "run-1", "--json"]).expect("lane cli");

        let guard =
            prepare_runtime_state_dir(&cli.command).expect("state dir preparation should succeed");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(explicit_state_dir)
        );
    }

    #[test]
    fn proof_browser_surface_is_project_bound_for_runtime_state() {
        let cli = Cli::try_parse_from([
            "vida",
            "proof",
            "browser",
            "--route",
            "http://127.0.0.1:51235",
            "--expect",
            "READY",
            "--json",
        ])
        .expect("proof browser cli should parse");

        assert!(command_needs_project_root_state_dir(&cli.command));
    }

    #[test]
    fn prepare_runtime_state_dir_normalizes_legacy_project_vida_env_override() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        fs::create_dir_all(harness.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let legacy_state_dir = harness.path().join(".vida");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
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
    fn prepare_runtime_state_dir_preserves_explicit_env_for_agent_dispatch_preview_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let packet_project =
            TempStateHarness::new().expect("packet temp harness should initialize");
        make_project_root(active_project.path());
        make_project_root(packet_project.path());
        let active_state_dir = active_project
            .path()
            .join(crate::state_store::default_state_dir());
        let packet_state_dir = packet_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&active_state_dir).expect("active state dir should exist");
        fs::create_dir_all(&packet_state_dir).expect("packet state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &packet_state_dir);
        let raw_args = ["vida", "agent", "dispatch-next", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("pre-parse state dir preparation should succeed");
        let cli = Cli::try_parse_from(raw_args).expect("agent dispatch-next cli should parse");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(packet_state_dir.clone())
        );
        assert_eq!(
            std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
            Some(packet_project.path().to_path_buf())
        );
        assert_eq!(
            std::env::current_dir().expect("cwd should read"),
            packet_project.path()
        );
        match cli.command {
            Some(Command::Agent(args)) => match args.command {
                crate::AgentCommand::DispatchNext(command) => {
                    assert_eq!(command.state_dir, Some(packet_state_dir.clone()));
                }
                other => panic!("expected dispatch-next command, got {other:?}"),
            },
            other => panic!("expected agent command, got {other:?}"),
        }
        drop(guard);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(packet_state_dir)
        );
    }

    #[test]
    fn prepare_runtime_state_dir_binds_explicit_agent_dispatch_state_dir_over_env() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let env_project = TempStateHarness::new().expect("env temp harness should initialize");
        let packet_project =
            TempStateHarness::new().expect("packet temp harness should initialize");
        make_project_root(active_project.path());
        make_project_root(env_project.path());
        make_project_root(packet_project.path());
        let env_state_dir = env_project
            .path()
            .join(crate::state_store::default_state_dir());
        let packet_state_dir = packet_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&env_state_dir).expect("env state dir should exist");
        fs::create_dir_all(&packet_state_dir).expect("packet state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &env_state_dir);
        let raw_args = [
            std::ffi::OsString::from("vida"),
            std::ffi::OsString::from("agent"),
            std::ffi::OsString::from("dispatch-next"),
            std::ffi::OsString::from("--state-dir"),
            packet_state_dir.clone().into_os_string(),
            std::ffi::OsString::from("--json"),
        ];

        let parse_guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("pre-parse state dir preparation should succeed");
        let cli = Cli::try_parse_from(raw_args).expect("agent dispatch-next cli should parse");
        let command_guard =
            prepare_runtime_state_dir(&cli.command).expect("command guard should bind state dir");

        assert!(parse_guard.is_some());
        assert!(command_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(packet_state_dir.clone())
        );
        assert_eq!(
            std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
            Some(packet_project.path().to_path_buf())
        );
        assert_eq!(
            std::env::current_dir().expect("cwd should read"),
            packet_project.path()
        );
        match cli.command {
            Some(Command::Agent(args)) => match args.command {
                crate::AgentCommand::DispatchNext(command) => {
                    assert_eq!(command.state_dir, Some(packet_state_dir.clone()));
                }
                other => panic!("expected dispatch-next command, got {other:?}"),
            },
            other => panic!("expected agent command, got {other:?}"),
        }
    }

    #[test]
    fn prepare_runtime_state_dir_preserves_env_for_runtime_state_dir_surface_matrix() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let env_project = TempStateHarness::new().expect("env temp harness should initialize");
        make_project_root(active_project.path());
        make_project_root(env_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        let env_state_dir = env_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&env_state_dir).expect("env state dir should exist");
        let scenarios = [
            vec![
                "vida",
                "agent",
                "host-bridge",
                "--request",
                "request.json",
                "--json",
            ],
            vec!["vida", "agent", "status", "--json"],
            vec!["vida", "orchestrator-init", "--json"],
            vec!["vida", "memory"],
            vec!["vida", "diagnostics", "post-commit", "--json"],
            vec!["vida", "orchestrator-session", "show", "--json"],
            vec!["vida", "status", "--json"],
            vec!["vida", "doctor", "--json"],
        ];

        for scenario in scenarios {
            let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
            let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &env_state_dir);
            let raw_args = scenario
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>();

            let parse_guard = prepare_runtime_state_dir_for_parse(&raw_args)
                .unwrap_or_else(|error| panic!("{scenario:?} pre-parse failed: {error}"));
            let cli = Cli::try_parse_from(raw_args)
                .unwrap_or_else(|error| panic!("{scenario:?} cli parse failed: {error}"));
            let command_guard = prepare_runtime_state_dir(&cli.command)
                .unwrap_or_else(|error| panic!("{scenario:?} command guard failed: {error}"));

            assert!(
                parse_guard.is_some(),
                "{scenario:?} should install a pre-parse state-dir guard"
            );
            assert!(
                command_guard.is_some(),
                "{scenario:?} should install a command state-dir guard"
            );
            assert_eq!(
                std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
                Some(env_state_dir.clone()),
                "{scenario:?} must preserve VIDA_STATE_DIR as authoritative"
            );
            assert_eq!(
                std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
                Some(env_project.path().to_path_buf()),
                "{scenario:?} must bind VIDA_ROOT to the env state project"
            );
            assert_eq!(
                std::env::current_dir().expect("cwd should read"),
                env_project.path(),
                "{scenario:?} must bind cwd to the env state project"
            );
        }
    }

    #[test]
    fn prepare_runtime_state_dir_for_parse_preserves_bare_env_for_agent_init() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let bare_state = TempStateHarness::new().expect("bare state harness should initialize");
        make_project_root(active_project.path());
        fs::create_dir_all(bare_state.path()).expect("bare state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _root_guard = EnvVarGuard::unset("VIDA_ROOT");
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", bare_state.path());
        let raw_args = ["vida", "agent-init", "--role", "tester", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("agent-init should preserve bare env state dir for parse");
        let cli = Cli::try_parse_from(raw_args).expect("agent-init cli should parse");
        let command_guard = prepare_runtime_state_dir(&cli.command)
            .expect("agent-init command guard should preserve bare state dir");

        assert!(guard.is_some());
        assert!(command_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(bare_state.path().to_path_buf())
        );
        assert!(std::env::var_os("VIDA_ROOT").is_none());
        assert_eq!(
            std::env::current_dir().expect("cwd should read"),
            active_project.path()
        );
        match cli.command {
            Some(Command::AgentInit(command)) => {
                assert_eq!(command.state_dir, Some(bare_state.path().to_path_buf()));
            }
            other => panic!("expected agent-init command, got {other:?}"),
        }
    }

    #[test]
    fn prepare_runtime_state_dir_preserves_env_for_taskflow_packet_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let packet_project =
            TempStateHarness::new().expect("packet temp harness should initialize");
        make_project_root(active_project.path());
        make_project_root(packet_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        let packet_state_dir = packet_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&packet_state_dir).expect("packet state dir should exist");
        {
            let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
            let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &packet_state_dir);
            let raw_args = ["vida", "taskflow", "packet", "latest", "--json"]
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>();

            let guard = prepare_runtime_state_dir_for_parse(&raw_args)
                .expect("packet surface should preserve env state-dir");

            assert!(guard.is_some());
            assert_eq!(
                std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
                Some(packet_state_dir.clone())
            );
            assert_eq!(
                std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
                Some(packet_project.path().to_path_buf())
            );
            assert_eq!(
                std::env::current_dir().expect("cwd should read"),
                packet_project.path()
            );
            drop(guard);
        }

        {
            let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
            let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &packet_state_dir);
            let run_graph_args = ["vida", "taskflow", "run-graph", "latest", "--json"]
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>();
            let run_graph_guard = prepare_runtime_state_dir_for_parse(&run_graph_args)
                .expect("run-graph surface should preserve env state-dir");

            assert!(run_graph_guard.is_some());
            assert_eq!(
                std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
                Some(packet_state_dir.clone())
            );
            assert_eq!(
                std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
                Some(packet_project.path().to_path_buf())
            );
            drop(run_graph_guard);
        }
    }

    #[test]
    fn prepare_runtime_state_dir_preserves_env_for_task_surface_before_parse() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let isolated_state =
            TempStateHarness::new().expect("isolated temp harness should initialize");
        make_project_root(active_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        fs::create_dir_all(isolated_state.path()).expect("isolated state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", isolated_state.path());
        let raw_args = [
            "vida",
            "task",
            "create",
            "fixture-task",
            "Fixture",
            "--json",
        ]
        .into_iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();

        let guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("task surface should preserve env state-dir");
        let cli = Cli::try_parse_from(raw_args).expect("task create cli should parse");

        assert!(guard.is_none());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(isolated_state.path().to_path_buf())
        );
        match cli.command {
            Some(Command::Task(args)) => match args.command {
                crate::TaskCommand::Create(command) => {
                    assert_eq!(command.state_dir, Some(isolated_state.path().to_path_buf()));
                }
                other => panic!("expected task create command, got {other:?}"),
            },
            other => panic!("expected task command, got {other:?}"),
        }
    }

    #[test]
    fn project_activator_state_dir_preserves_env_project_root_before_parse() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project =
            TempStateHarness::new().expect("active temp harness should initialize");
        let activator_project =
            TempStateHarness::new().expect("activator temp harness should initialize");
        make_project_root(active_project.path());
        make_project_root(activator_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        let activator_state_dir = activator_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&activator_state_dir).expect("activator state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &activator_state_dir);
        let raw_args = ["vida", "project-activator", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("project activator should preserve env state-dir");

        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(activator_state_dir)
        );
        assert_eq!(
            std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
            Some(activator_project.path().to_path_buf())
        );
        assert_eq!(
            std::env::current_dir().expect("cwd should read"),
            activator_project.path()
        );
        drop(guard);
    }

    #[test]
    fn prepare_runtime_state_dir_allows_valid_env_state_when_cwd_is_outside_project() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let project = TempStateHarness::new().expect("project temp harness should initialize");
        let outside = TempStateHarness::new().expect("outside temp harness should initialize");
        make_project_root(project.path());
        fs::create_dir_all(project.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let legacy_state_dir = project.path().join(".vida");
        let expected_state_dir = project.path().join(crate::state_store::default_state_dir());
        let _cwd = crate::test_cli_support::guard_current_dir(outside.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &legacy_state_dir);
        let _root_env_guard = EnvVarGuard::unset("VIDA_ROOT");
        let raw_args = ["vida", "status", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let parse_guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("pre-parse env fallback should be accepted outside a project");
        let cli = Cli::try_parse_from(raw_args).expect("status cli should parse");

        assert!(parse_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(expected_state_dir.clone())
        );
        assert_eq!(
            std::env::var_os("VIDA_ROOT").map(std::path::PathBuf::from),
            Some(project.path().to_path_buf())
        );
        match &cli.command {
            Some(Command::Status(args)) => {
                assert_eq!(args.state_dir, Some(expected_state_dir.clone()));
            }
            other => panic!("expected status command, got {other:?}"),
        }
        let runtime_guard = prepare_runtime_state_dir(&cli.command)
            .expect("env fallback should survive post-parse");
        assert!(runtime_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(expected_state_dir)
        );
        drop(runtime_guard);
        drop(parse_guard);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(legacy_state_dir)
        );
        assert!(std::env::var_os("VIDA_ROOT").is_none());
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
    fn doctor_honors_env_state_dir_inside_project_root() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project = TempStateHarness::new().expect("active harness should initialize");
        let isolated_project = TempStateHarness::new().expect("isolated harness should initialize");
        make_project_root(active_project.path());
        make_project_root(isolated_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        let isolated_state_dir = isolated_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&isolated_state_dir).expect("isolated state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &isolated_state_dir);
        let raw_args = ["vida", "doctor", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let parse_guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("doctor pre-parse should preserve explicit env state dir");
        let cli = Cli::try_parse_from(raw_args).expect("doctor cli should parse");

        assert!(parse_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(isolated_state_dir.clone())
        );
        match &cli.command {
            Some(Command::Doctor(args)) => {
                assert_eq!(args.state_dir, Some(isolated_state_dir.clone()));
            }
            other => panic!("expected doctor command, got {other:?}"),
        }
        assert!(!command_needs_project_root_state_dir(&cli.command));
        let runtime_guard = prepare_runtime_state_dir(&cli.command)
            .expect("doctor runtime preparation should keep explicit env state dir");
        assert!(runtime_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(isolated_state_dir)
        );
        drop(parse_guard);
    }

    #[test]
    fn taskflow_honors_env_state_dir_inside_project_root() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let active_project = TempStateHarness::new().expect("active harness should initialize");
        let isolated_project = TempStateHarness::new().expect("isolated harness should initialize");
        make_project_root(active_project.path());
        make_project_root(isolated_project.path());
        fs::create_dir_all(
            active_project
                .path()
                .join(crate::state_store::default_state_dir()),
        )
        .expect("active state dir should exist");
        let isolated_state_dir = isolated_project
            .path()
            .join(crate::state_store::default_state_dir());
        fs::create_dir_all(&isolated_state_dir).expect("isolated state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(active_project.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &isolated_state_dir);
        let raw_args = ["vida", "taskflow", "run-graph", "status", "run-1", "--json"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let parse_guard = prepare_runtime_state_dir_for_parse(&raw_args)
            .expect("taskflow pre-parse should preserve explicit env state dir");
        let cli = Cli::try_parse_from(raw_args).expect("taskflow cli should parse");

        assert!(parse_guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(isolated_state_dir.clone())
        );
        assert!(command_needs_project_root_state_dir(&cli.command));
        let runtime_guard = prepare_runtime_state_dir(&cli.command)
            .expect("taskflow runtime preparation should preserve explicit env state dir");
        drop(runtime_guard);
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(isolated_state_dir)
        );
        drop(parse_guard);
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
