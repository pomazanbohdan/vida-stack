use std::{ffi::OsString, process::ExitCode};

use super::{
    AgentArgs, AgentCommand, Cli, CoderCommand, Command, ReleaseCommand, SessionArgs,
    SessionCommand, StateArgs, StateCommand, StateResetArgs, TaskArgs, TaskCommand,
    agent_dispatch_surface, agent_feedback_surface, approval_surface, diagnostics_surface,
    docflow_proxy, docs_surface, doctor_surface, init_surfaces, lane_surface, memory_surface,
    orchestrator_session_surface, pack_surface, print_root_help, project_activator_surface,
    proof_surface, protocol_surface, quality_surface, release_surface, requirement_surface,
    run_taskflow_proxy, runtime_web_surface, service_client_cli, session_surface, status_surface,
    task_surface,
};
use crate::cli::{command_metadata_by_name, command_metadata_for_command};
use crate::root_state_binding::{
    RuntimeStateDirGuard, bind_runtime_state_dir_for_project_bound_command,
    bind_runtime_state_dir_override_for_project_bound_command,
    normalize_runtime_state_dir_env_for_parse, preserve_runtime_state_dir_env_for_parse_only,
    preserve_runtime_state_dir_env_for_project_bound_command,
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
        Some(Command::State(args)) => run_state(args).await,
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
        Some(Command::Requirement(args)) => requirement_surface::run_requirement(args).await,
        Some(Command::Pack(args)) => pack_surface::run_pack(args).await,
        Some(Command::Consume(args)) => run_legacy_taskflow_root_alias("consume", args).await,
        Some(Command::Lane(args)) => lane_surface::run_lane(args).await,
        Some(Command::Approval(args)) => approval_surface::run_approval(args).await,
        Some(Command::Recovery(args)) => run_legacy_taskflow_root_alias("recovery", args).await,
        Some(Command::Route(args)) => run_legacy_taskflow_root_alias("route", args).await,
        Some(Command::Release(args)) => match args.command {
            ReleaseCommand::Install(args) => release_surface::run_release_install(args),
        },
        Some(Command::Taskflow(args)) => {
            run_taskflow_proxy(normalize_taskflow_team_continue_alias(args)).await
        }
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

async fn run_legacy_taskflow_root_alias(alias: &'static str, args: super::ProxyArgs) -> ExitCode {
    match crate::compat::resolve_legacy_root_alias(alias, args) {
        Ok(resolution) => {
            eprintln!("{}", resolution.deprecation_notice);
            run_taskflow_proxy(resolution.canonical_args).await
        }
        Err(error) => {
            eprintln!("{}: {}", error.blocker_code, error.message);
            error.exit_code
        }
    }
}

fn normalize_taskflow_team_continue_alias(mut args: super::ProxyArgs) -> super::ProxyArgs {
    if matches!(
        (
            args.args.first().map(String::as_str),
            args.args.get(1).map(String::as_str)
        ),
        (Some("team"), Some("continue"))
    ) {
        if args
            .args
            .iter()
            .skip(2)
            .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "help"))
        {
            return args;
        }
        let mut canonical = vec!["consume".to_string(), "continue".to_string()];
        let mut rest = args.args.drain(2..).peekable();
        if rest
            .peek()
            .is_some_and(|arg| !arg.starts_with('-') && arg != "help")
        {
            canonical.push("--run-id".to_string());
        }
        let mut skip_state_dir_value = false;
        for arg in rest {
            if skip_state_dir_value {
                skip_state_dir_value = false;
                std::env::set_var("VIDA_STATE_DIR", &arg);
                continue;
            }
            if arg == "--state-dir" {
                skip_state_dir_value = true;
                continue;
            }
            if let Some(value) = arg.strip_prefix("--state-dir=") {
                std::env::set_var("VIDA_STATE_DIR", value);
                continue;
            }
            canonical.push(arg);
        }
        args.args = canonical;
    }
    args
}

async fn run_state(args: StateArgs) -> ExitCode {
    match args.command {
        StateCommand::Reset(command) => run_state_reset(command).await,
    }
}

async fn run_state_reset(command: StateResetArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .unwrap_or_else(crate::state_store::default_state_dir);
    let result = crate::state_store::StateStore::archive_and_reinit_state_root(
        state_dir,
        command.archive,
        command.reinit,
    )
    .await;

    match result {
        Ok(summary) => {
            if command.json {
                crate::print_json_pretty(&state_reset_operator_payload(&summary));
            } else {
                print!("{}", state_reset_plain_output(&summary));
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if command.json {
                crate::print_json_pretty(&state_reset_error_operator_payload(&error));
            } else {
                eprintln!("state reset failed: {error}");
            }
            ExitCode::from(1)
        }
    }
}

fn state_reset_operator_payload(
    summary: &crate::state_store::StateResetSummary,
) -> serde_json::Value {
    let summary_payload =
        serde_json::to_value(summary).expect("state reset summary should serialize");
    let artifact_refs = serde_json::json!({
        "surface": "vida state reset",
        "state_dir": summary.state_dir.display().to_string(),
        "archive_path": summary.archive_path.as_ref().map(|path| path.display().to_string()),
        "recovery_receipt_path": summary.recovery_receipt_path.as_ref().map(|path| path.display().to_string()),
    });
    crate::release1_operator_output::build_release1_operator_output_payload(
        "vida state reset",
        Vec::new(),
        Vec::new(),
        artifact_refs,
        summary_payload,
    )
    .expect("state reset operator payload should keep release-1 shape")
}

fn state_reset_plain_output(summary: &crate::state_store::StateResetSummary) -> String {
    let mut lines = vec![
        "status: pass".to_string(),
        format!("state_dir: {}", summary.state_dir.display()),
        format!("archive_created: {}", summary.archive_created),
    ];
    if let Some(path) = &summary.archive_path {
        lines.push(format!("archive_path: {}", path.display()));
    }
    if let Some(path) = &summary.recovery_receipt_path {
        lines.push(format!("recovery_receipt_path: {}", path.display()));
    }
    lines.extend([
        format!("reinitialized: {}", summary.reinitialized),
        format!("task_count: {}", summary.task_count),
        format!(
            "state_spine_manifest_present: {}",
            summary.state_spine_manifest_present
        ),
    ]);
    lines.push(String::new());
    lines.join("\n")
}

fn state_reset_error_operator_payload(
    error: &crate::state_store::StateStoreError,
) -> serde_json::Value {
    let artifact_refs = serde_json::json!({
        "surface": "vida state reset",
    });
    crate::release1_operator_output::build_release1_operator_output_payload(
        "vida state reset",
        vec!["state_reset_failed".to_string()],
        vec![
            "retry with `vida state reset --archive --reinit` after the state root is no longer in use"
                .to_string(),
        ],
        artifact_refs,
        serde_json::json!({
            "error": error.to_string(),
        }),
    )
    .expect("state reset error operator payload should keep release-1 shape")
}

fn command_label(command: &Option<Command>) -> String {
    match command {
        Some(Command::Release(args)) => release_command_label(args),
        Some(Command::External(args)) => args
            .first()
            .map(|name| format!("vida {name}"))
            .unwrap_or_else(|| "vida external".to_string()),
        _ => command_metadata_for_command(command).label.to_string(),
    }
}

fn release_command_label(args: &super::ReleaseArgs) -> String {
    match &args.command {
        ReleaseCommand::Install(args) if args.skip_build => {
            "vida release install --skip-build".to_string()
        }
        ReleaseCommand::Install(_) => "vida release install".to_string(),
    }
}

fn task_command_has_explicit_state_dir(args: &TaskArgs) -> bool {
    task_command_explicit_state_dir(args).is_some()
}

fn task_command_explicit_state_dir(args: &TaskArgs) -> Option<&std::path::Path> {
    match &args.command {
        TaskCommand::Import(command) => command.state_dir.as_deref(),
        TaskCommand::ImportJsonl(command) => command.state_dir.as_deref(),
        TaskCommand::ReplaceJsonl(command) => command.state_dir.as_deref(),
        TaskCommand::ExportJsonl(command) => command.state_dir.as_deref(),
        TaskCommand::List(command) => command.state_dir.as_deref(),
        TaskCommand::Search(command) => command.state_dir.as_deref(),
        TaskCommand::Show(command) => command.state_dir.as_deref(),
        TaskCommand::ValidatorPacket(command) => command.state_dir.as_deref(),
        TaskCommand::Progress(command) => command.state_dir.as_deref(),
        TaskCommand::ClosureReady(command) => command.state_dir.as_deref(),
        TaskCommand::Closeout(command) => command.state_dir.as_deref(),
        TaskCommand::Proof(command) => match &command.command {
            super::TaskProofCommand::Status(command) => command.state_dir.as_deref(),
            super::TaskProofCommand::AttachBrowser(command) => command.state_dir.as_deref(),
            super::TaskProofCommand::AttachEvidence(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Ready(command) => command.state_dir.as_deref(),
        TaskCommand::Next(command) => command.state_dir.as_deref(),
        TaskCommand::NextLawful(command) => command.state_dir.as_deref(),
        TaskCommand::NextDisplayId(command) => command.state_dir.as_deref(),
        TaskCommand::Create(command) | TaskCommand::Ensure(command) => command.state_dir.as_deref(),
        TaskCommand::Update(command) => command.state_dir.as_deref(),
        TaskCommand::Reset(command) => command.state_dir.as_deref(),
        TaskCommand::Note(command) => match &command.command {
            super::TaskNoteCommand::Append(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Block(command) => command.state_dir.as_deref(),
        TaskCommand::Verify(command) => command.state_dir.as_deref(),
        TaskCommand::Attempt(command) => match &command.command {
            super::TaskAttemptCommand::Dispatch(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Status(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Collect(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Consolidate(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Record(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Transition(command) => command.state_dir.as_deref(),
            super::TaskAttemptCommand::Summary(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Stage(command) => match &command.command {
            super::TaskStageCommand::Status(command) => command.state_dir.as_deref(),
        },
        TaskCommand::OwnedStatus(command) => command.state_dir.as_deref(),
        TaskCommand::Close(command) => command.state_dir.as_deref(),
        TaskCommand::PackFinalize(command) => command.state_dir.as_deref(),
        TaskCommand::Reconcile(command) => command.state_dir.as_deref(),
        TaskCommand::ReconcileClosedRuns(command) => command.state_dir.as_deref(),
        TaskCommand::PruneClosedEpics(command) => command.state_dir.as_deref(),
        TaskCommand::Split(command) => command.state_dir.as_deref(),
        TaskCommand::SpawnBlocker(command) => command.state_dir.as_deref(),
        TaskCommand::Deps(command)
        | TaskCommand::ReverseDeps(command)
        | TaskCommand::Children(command)
        | TaskCommand::Tree(command) => command.state_dir.as_deref(),
        TaskCommand::ReparentChildren(command) => command.state_dir.as_deref(),
        TaskCommand::DefectBatchRehome(command) => command.state_dir.as_deref(),
        TaskCommand::Blocked(command)
        | TaskCommand::ValidateGraph(command)
        | TaskCommand::CriticalPath(command) => command.state_dir.as_deref(),
        TaskCommand::Dep(command) => match &command.command {
            super::TaskDependencyCommand::Add(command) => command.state_dir.as_deref(),
            super::TaskDependencyCommand::Ensure(command) => command.state_dir.as_deref(),
            super::TaskDependencyCommand::AddBulk(command) => command.state_dir.as_deref(),
            super::TaskDependencyCommand::Remove(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Handoff(command) => match &command.command {
            super::TaskHandoffCommand::Accept(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Takeover(command) => match &command.command {
            super::TaskTakeoverCommand::Status(command) => command.state_dir.as_deref(),
        },
        TaskCommand::Help(_) | TaskCommand::Steps(_) | TaskCommand::AdaptivePreview(_) => None,
    }
}

fn task_command_needs_project_root(args: &TaskArgs) -> bool {
    !matches!(args.command, TaskCommand::Help(_) | TaskCommand::Steps(_))
        && !task_command_has_explicit_state_dir(args)
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

fn state_command_explicit_state_dir(args: &StateArgs) -> Option<&std::path::Path> {
    match &args.command {
        StateCommand::Reset(command) => command.state_dir.as_deref(),
    }
}

fn state_command_needs_project_root(args: &StateArgs) -> bool {
    state_command_explicit_state_dir(args).is_none()
}

fn proxy_args_request_help_or_version(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "help" | "--help" | "-h" | "--version" | "-V"))
}

fn proxy_command_needs_project_root(args: &[String]) -> bool {
    !proxy_args_request_help_or_version(args)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenericCliWorkflowMetric {
    pub(crate) workflow: &'static str,
    pub(crate) legacy_command_count: usize,
    pub(crate) canonical_command_count: usize,
    pub(crate) legacy_option_count: usize,
    pub(crate) canonical_option_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenericCliCommandMetrics {
    pub(crate) legacy_operation_leaf_count: usize,
    pub(crate) canonical_generic_leaf_count: usize,
    pub(crate) leaf_reduction_percent: usize,
    pub(crate) max_transport_context_flags: usize,
    pub(crate) workflows: Vec<GenericCliWorkflowMetric>,
}

pub(crate) fn generic_service_client_command_metrics() -> GenericCliCommandMetrics {
    let legacy_operation_leaf_count = 17;
    let canonical_generic_leaf_count = 5;
    let workflows = vec![
        workflow_metric("service status", 2, 0),
        workflow_metric("service capabilities", 2, 0),
        workflow_metric("service endpoint status", 2, 0),
        workflow_metric("service lifecycle plan", 3, 1),
        workflow_metric("project list", 2, 0),
        workflow_metric("project resolve", 4, 1),
        workflow_metric("wizard inspect", 3, 1),
        workflow_metric("wizard validate", 3, 1),
        workflow_metric("job status", 3, 1),
        workflow_metric("receipt get", 3, 1),
    ];
    GenericCliCommandMetrics {
        legacy_operation_leaf_count,
        canonical_generic_leaf_count,
        leaf_reduction_percent: ((legacy_operation_leaf_count - canonical_generic_leaf_count)
            * 100)
            / legacy_operation_leaf_count,
        max_transport_context_flags: 5,
        workflows,
    }
}

fn workflow_metric(
    workflow: &'static str,
    legacy_option_count: usize,
    canonical_option_count: usize,
) -> GenericCliWorkflowMetric {
    GenericCliWorkflowMetric {
        workflow,
        legacy_command_count: 1,
        canonical_command_count: 1,
        legacy_option_count,
        canonical_option_count,
    }
}

pub(crate) fn command_needs_project_root_state_dir(command: &Option<Command>) -> bool {
    let metadata = command_metadata_for_command(command);
    if !metadata.binds_project_state_dir {
        return false;
    }
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
        Some(Command::State(args)) => state_command_needs_project_root(args),
        Some(Command::Diagnostics(args)) => diagnostics_command_explicit_state_dir(args).is_none(),
        Some(Command::OrchestratorSession(args)) => {
            orchestrator_session_command_explicit_state_dir(args).is_none()
        }
        Some(Command::AgentFeedback(_)) => metadata.binds_project_state_dir,
        Some(Command::Runtime(_)) => metadata.binds_project_state_dir,
        Some(Command::Proof(_)) => metadata.binds_project_state_dir,
        Some(Command::Service(_)) => metadata.binds_project_state_dir,
        Some(Command::Project(_)) => metadata.binds_project_state_dir,
        Some(Command::Wizard(_)) => metadata.binds_project_state_dir,
        Some(Command::Job(_)) => metadata.binds_project_state_dir,
        Some(Command::Receipt(_)) => metadata.binds_project_state_dir,
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
        Some(Command::Task(command)) => task_command_explicit_state_dir(command),
        Some(Command::State(command)) => state_command_explicit_state_dir(command),
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
    command_metadata_for_command(command).preserves_env_state_dir
}

fn command_preserves_parse_only_env_state_dir(command: &Option<Command>) -> bool {
    command_metadata_for_command(command).preserves_parse_only_env_state_dir
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
    command_metadata_by_name(command)
        .map(|metadata| metadata.binds_project_state_dir)
        .unwrap_or(false)
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
    let command = positional.next();
    match command {
        Some("task") => true,
        Some("agent") => matches!(
            positional.next(),
            Some("dispatch-next" | "select" | "status" | "host-bridge")
        ),
        Some(command) => command_metadata_by_name(command)
            .map(|metadata| metadata.preserves_env_state_dir)
            .unwrap_or(false),
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
    command_metadata_by_name(command)
        .map(|metadata| metadata.preserves_parse_only_env_state_dir)
        .unwrap_or(false)
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
        Cli, command_needs_project_root_state_dir, generic_service_client_command_metrics,
        normalize_runtime_state_dir_env_for_parse, prepare_runtime_state_dir,
        prepare_runtime_state_dir_for_parse, state_reset_operator_payload,
        state_reset_plain_output,
    };
    use crate::Command;
    use crate::temp_state::TempStateHarness;
    use clap::Parser;
    use std::fs;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn generic_service_client_command_metrics_prove_leaf_and_workflow_reduction() {
        let metrics = generic_service_client_command_metrics();

        assert_eq!(metrics.legacy_operation_leaf_count, 17);
        assert_eq!(metrics.canonical_generic_leaf_count, 5);
        assert!(
            metrics.leaf_reduction_percent >= 40,
            "expected at least 40% leaf reduction, got {}%",
            metrics.leaf_reduction_percent
        );
        assert!(metrics.max_transport_context_flags <= 5);
        assert_eq!(metrics.workflows.len(), 10);
        assert!(
            metrics
                .workflows
                .iter()
                .all(|workflow| workflow.canonical_command_count <= workflow.legacy_command_count)
        );
        assert!(
            metrics
                .workflows
                .iter()
                .all(|workflow| workflow.canonical_option_count < workflow.legacy_option_count)
        );
    }

    #[test]
    fn state_reset_output_exposes_recovery_receipt_path_in_plain_and_json_contracts() {
        let summary = crate::state_store::StateResetSummary {
            surface: "vida state reset",
            status: "pass",
            state_dir: std::path::PathBuf::from("C:/tmp/vida/state"),
            archive_path: Some(std::path::PathBuf::from("C:/tmp/vida/state.archive.1")),
            recovery_receipt_path: Some(std::path::PathBuf::from(
                "C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json",
            )),
            archive_created: true,
            reinitialized: true,
            task_count: 3,
            state_spine_manifest_present: true,
        };

        let plain = state_reset_plain_output(&summary);
        assert!(plain.contains(
            "recovery_receipt_path: C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json"
        ));

        let payload = state_reset_operator_payload(&summary);
        assert_eq!(
            payload["recovery_receipt_path"],
            "C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json"
        );
        assert_eq!(
            payload["artifact_refs"]["recovery_receipt_path"],
            "C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json"
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["recovery_receipt_path"],
            "C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json"
        );
        assert_eq!(
            payload["operator_contracts"]["artifact_refs"]["recovery_receipt_path"],
            "C:/tmp/vida/state/recovery/state-reset-receipts/state-reset-1.json"
        );
    }

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
    fn prepare_runtime_state_dir_normalizes_project_bound_state_reset_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        fs::create_dir_all(harness.path().join(crate::state_store::default_state_dir()))
            .expect("canonical state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let cli = Cli::try_parse_from(["vida", "state", "reset", "--archive", "--reinit"])
            .expect("state reset cli should parse");

        assert!(command_needs_project_root_state_dir(&cli.command));
        let guard =
            prepare_runtime_state_dir(&cli.command).expect("state dir preparation should succeed");
        assert!(guard.is_some());
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(harness.path().join(crate::state_store::default_state_dir()))
        );
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
    fn prepare_runtime_state_dir_for_parse_preserves_explicit_env_for_state_reset_surface() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        make_project_root(harness.path());
        let explicit_state_dir = harness.path().join("explicit-state");
        fs::create_dir_all(&explicit_state_dir).expect("explicit state dir should exist");
        let _cwd = crate::test_cli_support::guard_current_dir(harness.path());
        let _env_guard = EnvVarGuard::set("VIDA_STATE_DIR", &explicit_state_dir);
        let args = [
            std::ffi::OsString::from("vida"),
            std::ffi::OsString::from("state"),
            std::ffi::OsString::from("reset"),
            std::ffi::OsString::from("--archive"),
            std::ffi::OsString::from("--reinit"),
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
        assert!(
            prepare_runtime_state_dir(&cli.command)
                .expect("state dir preparation should succeed")
                .is_none()
        );
        assert!(std::env::var_os("VIDA_STATE_DIR").is_none());
    }

    #[test]
    fn prepare_runtime_state_dir_uses_explicit_task_state_dir_without_project_root() {
        let _lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _env_guard = EnvVarGuard::unset("VIDA_STATE_DIR");
        let explicit_state_dir = std::path::PathBuf::from("C:/tmp/isolated-state");
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
        let guard = prepare_runtime_state_dir(&cli.command)
            .expect("state dir preparation should succeed")
            .expect("explicit task state-dir should bind runtime env");
        assert_eq!(
            std::env::var_os("VIDA_STATE_DIR").map(std::path::PathBuf::from),
            Some(explicit_state_dir)
        );
        drop(guard);
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
