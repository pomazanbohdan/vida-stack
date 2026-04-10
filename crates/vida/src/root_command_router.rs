use std::process::ExitCode;

use serde::Serialize;

use super::{
    agent_feedback_surface, docflow_proxy, doctor_surface, init_surfaces, lane_surface,
    memory_surface, print_root_help, project_activator_surface, protocol_surface,
    resolve_runtime_project_root, run_taskflow_proxy, state_store, status_surface, task_surface,
    Cli, Command, TaskArgs, TaskCommand,
};

#[derive(Serialize)]
struct UnsupportedRootSurfaceEnvelope<'a> {
    surface: &'a str,
    status: &'a str,
    trace_id: Option<&'a str>,
    workflow_class: Option<&'a str>,
    risk_tier: Option<&'a str>,
    artifact_refs: Vec<String>,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    reason: String,
}

fn run_reserved_root_surface(surface: &'static str, args: &[String]) -> ExitCode {
    let as_json = args.iter().any(|arg| arg == "--json");
    let envelope = UnsupportedRootSurfaceEnvelope {
        surface,
        status: "blocked",
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: Vec::new(),
        next_actions: match surface {
            "vida approval" => vec![
                "Use `vida taskflow query \"approval\"` to discover the bounded approval command, then apply the run-graph update explicitly.".to_string(),
            ],
            _ => vec!["Use the bounded TaskFlow family surface until the root surface is implemented.".to_string()],
        },
        blocker_codes: vec!["unsupported_blocker_code".to_string()],
        reason: format!(
            "{surface} is a reserved Release-1 root surface but does not yet have a family-owned implementation; the runtime fails closed instead of silently falling back."
        ),
    };

    if crate::surface_render::print_surface_json(
        &envelope,
        as_json,
        "reserved root surface envelope should serialize",
    ) {
        return ExitCode::from(2);
    }

    crate::print_surface_header(crate::RenderMode::Plain, surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "blocker_codes",
        &envelope.blocker_codes.join(", "),
    );
    crate::print_surface_line(crate::RenderMode::Plain, "reason", &envelope.reason);
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    ExitCode::from(2)
}

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
        Some(Command::Approval(args)) => run_reserved_root_surface("vida approval", &args.args),
        Some(Command::Recovery(args)) => {
            let mut prefixed = vec!["recovery".to_string()];
            prefixed.extend(args.args);
            run_taskflow_proxy(super::ProxyArgs { args: prefixed }).await
        }
        Some(Command::Taskflow(args)) => run_taskflow_proxy(args).await,
        Some(Command::Docflow(args)) => docflow_proxy::run_docflow_proxy(args),
        Some(Command::External(args)) => run_unknown(&args),
    }
}

fn task_command_needs_project_root(args: &TaskArgs) -> bool {
    !matches!(args.command, TaskCommand::Help(_))
}

fn prepare_runtime_state_dir(command: &Option<Command>) -> Option<String> {
    if std::env::var_os("VIDA_STATE_DIR").is_some() {
        return None;
    }

    let needs_project_root = match command {
        Some(Command::Task(args)) => task_command_needs_project_root(args),
        _ => false,
    };

    if !needs_project_root {
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

fn run_unknown(args: &[String]) -> ExitCode {
    let command = args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unknown command family `{command}`. Use `vida --help` to inspect the frozen root surface."
    );
    ExitCode::from(2)
}
