use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const ROOT_AFTER_HELP: &str = "Runtime-family help paths:\n  vida taskflow help\n  vida task --help\n  vida taskflow help parallelism\n  vida docflow help";

const TASK_LONG_ABOUT: &str = "Task inspection, mutation, and graph routing over the authoritative state store.\n\nUse `vida task` for the canonical backlog contract. Parent-child edges preserve structure, `blocks` edges preserve ordering, and execution semantics add fail-closed sequencing/parallelism metadata on top of graph truth.";

const TASK_AFTER_HELP: &str = "Most-used task commands:\n  vida task ready --json\n  vida task next --json\n  vida task show <task-id> --json\n  vida task progress <task-id> --json\n  vida task deps <task-id> --json\n  vida task tree <task-id> --json\n  vida task reparent-children <from-parent-id> <to-parent-id> --json\n  vida task critical-path --json\n  vida taskflow help parallelism\n\nParallelism guidance:\n  Use `vida taskflow help parallelism` for the canonical execution_mode/order_bucket/parallel_group/conflict_domain contract.\n  `vida task help parallelism` remains a compatibility alias to the same TaskFlow-owned help.\n  Use `vida taskflow graph-summary --json` to see `ready_parallel_safe`, `parallel_blockers`, and `parallel_candidates_after_current`.\n  Missing execution semantics never imply safe parallel execution.";

const TASKFLOW_LONG_ABOUT: &str = "Delegate to the TaskFlow runtime family.\n\nTaskFlow is the execution/runtime authority. Use it for tracked execution, backlog pressure, run-graph state, packet inspection, continuation binding, and closure handoff.";

const TASKFLOW_AFTER_HELP: &str = "Family entrypoints:\n  vida taskflow help\n  vida taskflow help task\n  vida taskflow help parallelism\n  vida taskflow help dependencies\n  vida taskflow help queue\n  vida taskflow help dispatch\n  vida task tree <task-id> --json\n  vida taskflow graph-summary --json\n  vida taskflow route explain --json\n  vida taskflow validate-routing --json\n  vida taskflow status --summary --json\n  vida taskflow run-graph status <run-id> --json\n  vida taskflow recovery status <run-id> --json\n  vida taskflow packet latest --json\n  vida taskflow bootstrap-spec \"feature request\" --json\n  vida task next --json\n\nParallelism guidance:\n  `vida taskflow graph-summary --json` exposes `current_task_id`, `scheduling.ready[*].ready_parallel_safe`, `parallel_blockers`, and `parallel_candidates_after_current`.\n  `vida taskflow help parallelism` explains execution semantics fields and fail-closed scheduling rules.";

const TASK_CREATE_ABOUT: &str = "Create one tracked task in the authoritative backlog store.";
const TASK_CREATE_LONG_ABOUT: &str = "Create one tracked task in the authoritative backlog store.\n\nExecution semantics are additive to graph truth:\n- `--execution-mode sequential` keeps the task single-lane by default\n- `--execution-mode parallel_safe` allows parallel admission only when other semantics also match\n- `--execution-mode exclusive` blocks parallel execution\n- `--order-bucket`, `--parallel-group`, and `--conflict-domain` refine safe co-scheduling";
const TASK_CREATE_AFTER_HELP: &str = "Examples:\n  vida task create <task-id> <title> --parent-id <parent-id> --json\n  vida task create <task-id> <title> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n\nNotes:\n  Missing execution semantics fail closed for parallel scheduling.\n  Use `vida taskflow graph-summary --json` to verify parallel-safe admission after mutation.";

const TASK_UPDATE_ABOUT: &str = "Update one tracked task in the authoritative backlog store.";
const TASK_UPDATE_LONG_ABOUT: &str = "Update one tracked task in the authoritative backlog store.\n\nUse execution-semantics flags to correct sequencing and parallelism truth without moving ordering back into notes:\n- `--execution-mode sequential|parallel_safe|exclusive`\n- `--order-bucket <id>`\n- `--parallel-group <id>`\n- `--conflict-domain <id>`\n- matching `--clear-*` flags remove one semantics field";
const TASK_UPDATE_AFTER_HELP: &str = "Examples:\n  vida task update <task-id> --status in_progress --json\n  vida task update <task-id> --parent-id <parent-id> --json\n  vida task update <task-id> --clear-parent-id --json\n  vida task update <task-id> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n  vida task update <task-id> --clear-parallel-group --clear-conflict-domain --json\n\nNotes:\n  Use either a value flag or the matching clear flag, not both.\n  Re-check `vida taskflow graph-summary --json` after updates to confirm `ready_parallel_safe` and `parallel_blockers`.";

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
    long_about = "VIDA Binary Foundation\n\nTaskFlow remains execution authority; DocFlow remains the documentation/readiness surface. Root `lane` and `approval` are family-owned operator surfaces over the delegated runtime law.",
    after_help = ROOT_AFTER_HELP
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
        about = "task inspection, mutation, and graph routing over the authoritative state store",
        long_about = TASK_LONG_ABOUT,
        after_help = TASK_AFTER_HELP
    )]
    Task(TaskArgs),
    #[command(about = "inspect the effective instruction bundle")]
    Memory(MemoryArgs),
    #[command(about = "inspect backend, state spine, and latest receipts")]
    Status(StatusArgs),
    #[command(about = "run bounded runtime integrity checks")]
    Doctor(DoctorArgs),
    #[command(about = "thin root alias to the TaskFlow consume family")]
    Consume(ProxyArgs),
    #[command(about = "inspect or mutate canonical lane/takeover operator state")]
    Lane(ProxyArgs),
    #[command(
        about = "family-owned root operator surface for approval inspection over the run-graph approval law"
    )]
    Approval(ProxyArgs),
    #[command(about = "thin root alias to the TaskFlow recovery family")]
    Recovery(ProxyArgs),
    #[command(
        about = "delegate to the TaskFlow runtime family",
        long_about = TASKFLOW_LONG_ABOUT,
        after_help = TASKFLOW_AFTER_HELP
    )]
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
    #[command(about = "authoritatively replace backlog state from a canonical snapshot artifact")]
    ReplaceJsonl(TaskReplaceJsonlArgs),
    ExportJsonl(TaskExportJsonlArgs),
    List(TaskListArgs),
    Show(TaskShowArgs),
    Progress(TaskDepsArgs),
    Ready(TaskReadyArgs),
    Next(TaskNextArgs),
    NextDisplayId(TaskNextDisplayIdArgs),
    #[command(
        about = TASK_CREATE_ABOUT,
        long_about = TASK_CREATE_LONG_ABOUT,
        after_help = TASK_CREATE_AFTER_HELP
    )]
    Create(TaskCreateArgs),
    Ensure(TaskCreateArgs),
    #[command(
        about = TASK_UPDATE_ABOUT,
        long_about = TASK_UPDATE_LONG_ABOUT,
        after_help = TASK_UPDATE_AFTER_HELP
    )]
    Update(TaskUpdateArgs),
    Close(TaskCloseArgs),
    Deps(TaskDepsArgs),
    ReverseDeps(TaskDepsArgs),
    Blocked(TaskBlockedArgs),
    #[command(about = "inspect direct children for one task from the authoritative backlog store")]
    Children(TaskDepsArgs),
    #[command(
        about = "bulk-reparent direct children from one parent task to another",
        alias = "move-children"
    )]
    ReparentChildren(TaskBulkReparentArgs),
    #[command(
        about = "inspect one recursive task subtree from the authoritative backlog store",
        alias = "subtree"
    )]
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
pub(crate) struct TaskReplaceJsonlArgs {
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

    #[arg(
        long = "labels",
        value_delimiter = ',',
        help = "Task labels. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) labels: Vec<String>,

    #[arg(long = "execution-mode")]
    pub(crate) execution_mode: Option<String>,

    #[arg(long = "order-bucket")]
    pub(crate) order_bucket: Option<String>,

    #[arg(long = "parallel-group")]
    pub(crate) parallel_group: Option<String>,

    #[arg(long = "conflict-domain")]
    pub(crate) conflict_domain: Option<String>,

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

    #[arg(long = "notes-file")]
    pub(crate) notes_file: Option<PathBuf>,

    #[arg(long = "description")]
    pub(crate) description: Option<String>,

    #[arg(long = "parent-id")]
    pub(crate) parent_id: Option<String>,

    #[arg(long = "clear-parent-id")]
    pub(crate) clear_parent_id: bool,

    #[arg(
        long = "add-label",
        value_delimiter = ',',
        help = "Labels to add. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) add_labels: Vec<String>,

    #[arg(
        long = "remove-label",
        value_delimiter = ',',
        help = "Labels to remove. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) remove_labels: Vec<String>,

    #[arg(
        long = "set-labels",
        help = "Replace labels with a comma-separated list."
    )]
    pub(crate) set_labels: Option<String>,

    #[arg(long = "execution-mode")]
    pub(crate) execution_mode: Option<String>,

    #[arg(long = "order-bucket")]
    pub(crate) order_bucket: Option<String>,

    #[arg(long = "parallel-group")]
    pub(crate) parallel_group: Option<String>,

    #[arg(long = "conflict-domain")]
    pub(crate) conflict_domain: Option<String>,

    #[arg(long = "clear-execution-mode")]
    pub(crate) clear_execution_mode: bool,

    #[arg(long = "clear-order-bucket")]
    pub(crate) clear_order_bucket: bool,

    #[arg(long = "clear-parallel-group")]
    pub(crate) clear_parallel_group: bool,

    #[arg(long = "clear-conflict-domain")]
    pub(crate) clear_conflict_domain: bool,

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
pub(crate) struct TaskBulkReparentArgs {
    pub(crate) from_parent_id: String,
    pub(crate) to_parent_id: String,

    #[arg(
        long = "child-id",
        help = "Only move the listed direct child ids. Repeat to move a subset."
    )]
    pub(crate) child_ids: Vec<String>,

    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,

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

    #[arg(long = "execute-dispatch")]
    pub(crate) execute_dispatch: bool,

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

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn task_help_lists_mutation_commands() {
        let mut command = Cli::command();
        let task = command
            .find_subcommand_mut("task")
            .expect("task subcommand should exist");
        let help = task.render_long_help().to_string();
        assert!(help.contains("create"), "task help should list create");
        assert!(help.contains("update"), "task help should list update");
        assert!(help.contains("close"), "task help should list close");
        assert!(
            help.contains("next-display-id"),
            "task help should list next-display-id"
        );
        assert!(
            help.contains("export-jsonl"),
            "task help should list export-jsonl"
        );
    }
}
