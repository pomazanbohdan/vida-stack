use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub(crate) enum RenderMode {
    #[default]
    Plain,
    Color,
    #[value(name = "color_emoji")]
    ColorEmoji,
}

#[derive(Parser, Debug)]
#[command(
    name = "vida",
    disable_help_subcommand = true,
    about = "VIDA Binary Foundation",
    long_about = "VIDA Binary Foundation\n\nRoot commands stay fail-closed. TaskFlow remains execution authority; DocFlow remains the documentation/readiness surface.",
    after_help = "Runtime-family help paths:\n  vida taskflow help\n  vida docflow help"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    #[command(about = "bootstrap framework carriers into the current project")]
    Init(BootArgs),
    #[command(about = "initialize authoritative state and instruction/framework-memory surfaces")]
    Boot(BootArgs),
    #[command(about = "render the compiled startup view for the orchestrator lane")]
    OrchestratorInit(InitArgs),
    #[command(
        about = "render the bounded startup view or packet activation view for a worker/agent lane"
    )]
    AgentInit(AgentInitArgs),
    #[command(about = "resolve and render framework protocol/guide surfaces")]
    Protocol(ProtocolArgs),
    #[command(about = "inspect project activation posture and bounded onboarding next steps")]
    ProjectActivator(ProjectActivatorArgs),
    #[command(about = "record host-agent feedback and refresh local strategy state")]
    AgentFeedback(AgentFeedbackArgs),
    #[command(
        about = "task inspection, mutation, and graph routing over the authoritative state store"
    )]
    Task(TaskArgs),
    #[command(about = "inspect the effective instruction bundle")]
    Memory(MemoryArgs),
    #[command(about = "inspect backend, state spine, and latest receipts")]
    Status(StatusArgs),
    #[command(about = "run bounded runtime integrity checks")]
    Doctor(DoctorArgs),
    #[command(about = "delegate to the TaskFlow runtime family")]
    Taskflow(ProxyArgs),
    #[command(about = "delegate to the DocFlow runtime family")]
    Docflow(ProxyArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct ProxyArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct TaskArgs {
    #[command(subcommand)]
    pub(crate) command: TaskCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskCommand {
    Help(TaskHelpArgs),
    ImportJsonl(TaskImportJsonlArgs),
    ExportJsonl(TaskExportJsonlArgs),
    List(TaskListArgs),
    Show(TaskShowArgs),
    Ready(TaskReadyArgs),
    Next(TaskNextArgs),
    NextDisplayId(TaskNextDisplayIdArgs),
    Create(TaskCreateArgs),
    Ensure(TaskCreateArgs),
    Update(TaskUpdateArgs),
    Close(TaskCloseArgs),
    Deps(TaskDepsArgs),
    ReverseDeps(TaskDepsArgs),
    Blocked(TaskBlockedArgs),
    Tree(TaskDepsArgs),
    ValidateGraph(TaskBlockedArgs),
    Dep(TaskDepArgs),
    CriticalPath(TaskBlockedArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskDepArgs {
    #[command(subcommand)]
    pub(crate) command: TaskDependencyCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskDependencyCommand {
    Add(TaskDependencyMutationCommandArgs),
    Remove(TaskDependencyTargetCommandArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskHelpArgs {
    pub(crate) topic: Option<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDependencyMutationCommandArgs {
    pub(crate) task_id: String,
    pub(crate) depends_on_id: String,
    pub(crate) edge_type: String,

    #[arg(long = "created-by", default_value = "vida")]
    pub(crate) created_by: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDependencyTargetCommandArgs {
    pub(crate) task_id: String,
    pub(crate) depends_on_id: String,
    pub(crate) edge_type: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskImportJsonlArgs {
    pub(crate) path: PathBuf,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskExportJsonlArgs {
    pub(crate) path: PathBuf,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskListArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "status")]
    pub(crate) status: Option<String>,

    #[arg(long = "all")]
    pub(crate) all: bool,

    #[arg(long = "summary")]
    pub(crate) summary: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskShowArgs {
    pub(crate) task_id: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskNextDisplayIdArgs {
    pub(crate) parent_display_id: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskCreateArgs {
    pub(crate) task_id: String,
    pub(crate) title: String,

    #[arg(long = "type", default_value = "task")]
    pub(crate) issue_type: String,

    #[arg(long = "status", default_value = "open")]
    pub(crate) status: String,

    #[arg(long = "priority", default_value_t = 2)]
    pub(crate) priority: u32,

    #[arg(long = "display-id")]
    pub(crate) display_id: Option<String>,

    #[arg(long = "parent-id")]
    pub(crate) parent_id: Option<String>,

    #[arg(long = "parent-display-id")]
    pub(crate) parent_display_id: Option<String>,

    #[arg(long = "auto-display-from")]
    pub(crate) auto_display_from: Option<String>,

    #[arg(long = "description", default_value = "")]
    pub(crate) description: String,

    #[arg(long = "labels")]
    pub(crate) labels: Vec<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskUpdateArgs {
    pub(crate) task_id: String,

    #[arg(long = "status")]
    pub(crate) status: Option<String>,

    #[arg(long = "notes")]
    pub(crate) notes: Option<String>,

    #[arg(long = "description")]
    pub(crate) description: Option<String>,

    #[arg(long = "add-label")]
    pub(crate) add_labels: Vec<String>,

    #[arg(long = "remove-label")]
    pub(crate) remove_labels: Vec<String>,

    #[arg(long = "set-labels")]
    pub(crate) set_labels: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskCloseArgs {
    pub(crate) task_id: String,

    #[arg(long = "reason")]
    pub(crate) reason: String,

    #[arg(long = "source", hide = true)]
    pub(crate) source: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskReadyArgs {
    #[arg(long = "scope")]
    pub(crate) scope: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskNextArgs {
    #[arg(long = "scope")]
    pub(crate) scope: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDepsArgs {
    pub(crate) task_id: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskBlockedArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "summary")]
    pub(crate) summary: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct BootArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "instruction-source-root", env = "VIDA_INSTRUCTION_SOURCE_ROOT")]
    pub(crate) instruction_source_root: Option<PathBuf>,

    #[arg(
        long = "framework-memory-source-root",
        env = "VIDA_FRAMEWORK_MEMORY_SOURCE_ROOT"
    )]
    pub(crate) framework_memory_source_root: Option<PathBuf>,

    #[arg(hide = true, trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct InitArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct ProjectActivatorArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "project-id")]
    pub(crate) project_id: Option<String>,

    #[arg(long = "project-name")]
    pub(crate) project_name: Option<String>,

    #[arg(long = "language")]
    pub(crate) language: Option<String>,

    #[arg(long = "user-communication-language")]
    pub(crate) user_communication_language: Option<String>,

    #[arg(long = "reasoning-language")]
    pub(crate) reasoning_language: Option<String>,

    #[arg(long = "documentation-language")]
    pub(crate) documentation_language: Option<String>,

    #[arg(long = "todo-protocol-language")]
    pub(crate) todo_protocol_language: Option<String>,

    #[arg(long = "host-cli-system")]
    pub(crate) host_cli_system: Option<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct AgentFeedbackArgs {
    #[arg(long = "agent-id")]
    pub(crate) agent_id: String,

    #[arg(long = "score")]
    pub(crate) score: u64,

    #[arg(long = "outcome")]
    pub(crate) outcome: Option<String>,

    #[arg(long = "task-class")]
    pub(crate) task_class: Option<String>,

    #[arg(long = "notes")]
    pub(crate) notes: Option<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct AgentInitArgs {
    pub(crate) request_text: Option<String>,

    #[arg(long = "role")]
    pub(crate) role: Option<String>,

    #[arg(long = "dispatch-packet")]
    pub(crate) dispatch_packet: Option<String>,

    #[arg(long = "downstream-packet")]
    pub(crate) downstream_packet: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ProtocolArgs {
    #[command(subcommand)]
    pub(crate) command: ProtocolCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ProtocolCommand {
    View(ProtocolViewArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ProtocolViewArgs {
    #[arg(required = true, num_args = 1..)]
    pub(crate) names: Vec<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct MemoryArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct StatusArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "summary")]
    pub(crate) summary: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DoctorArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "summary")]
    pub(crate) summary: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}
