use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const ROOT_AFTER_HELP: &str = "Runtime-family help paths:\n  vida taskflow help\n  vida task --help\n  vida taskflow help parallelism\n  vida route explain --json\n  vida docflow help\n  vida docs update --json";

const TASK_LONG_ABOUT: &str = "Task inspection, mutation, and graph routing over the authoritative state store.\n\nUse `vida task` for the canonical backlog contract. Parent-child edges preserve structure, `blocks` edges preserve ordering, and execution semantics add fail-closed sequencing/parallelism metadata on top of graph truth.";

const TASK_AFTER_HELP: &str = "Most-used task commands:\n  vida task ready --json\n  vida task next --json\n  vida task show <task-id> --json\n  vida task progress <task-id> --json\n  vida task deps <task-id> --json\n  vida task tree <task-id> --json\n  vida task split <task-id> --child child-a:\"First slice\" --reason \"oversized task\" --json\n  vida task spawn-blocker <task-id> <blocker-task-id> \"Blocker title\" --reason \"new dependency\" --json\n  vida task reparent-children <from-parent-id> <to-parent-id> --json\n  vida task defect-batch-rehome <from-parent-id> <to-parent-id> --pause-task-id <task-id> --start-task-id <task-id> --json\n  vida task critical-path --json\n  vida taskflow help parallelism\n\nParallelism guidance:\n  Use `vida taskflow help parallelism` for the canonical execution_mode/order_bucket/parallel_group/conflict_domain contract.\n  `vida task help parallelism` remains a compatibility alias to the same TaskFlow-owned help.\n  Use `vida taskflow graph-summary --json` to see `ready_parallel_safe`, `parallel_blockers`, and `parallel_candidates_after_current`.\n  Missing execution semantics never imply safe parallel execution.";

const TASKFLOW_LONG_ABOUT: &str = "Delegate to the TaskFlow runtime family.\n\nTaskFlow is the execution/runtime authority. Use it for tracked execution, backlog pressure, run-graph state, packet inspection, continuation binding, and closure handoff.";

const TASKFLOW_AFTER_HELP: &str = "Family entrypoints:\n  vida taskflow help\n  vida taskflow help task\n  vida taskflow help parallelism\n  vida taskflow help dependencies\n  vida taskflow help queue\n  vida taskflow help dispatch\n  vida taskflow help scheduler\n  vida task tree <task-id> --json\n  vida taskflow graph explain <task-id> --json\n  vida taskflow graph-summary --json\n  vida taskflow plan generate --json\n  vida taskflow replan split <task-id> --child child-a:\"First slice\" --reason \"oversized task\" --json\n  vida taskflow replan spawn-blocker <task-id> <blocker-task-id> \"Blocker title\" --reason \"new dependency\" --json\n  vida taskflow scheduler dispatch --json\n  vida taskflow route explain --json\n  vida taskflow validate-routing --json\n  vida taskflow pricing status --json\n  vida taskflow pricing import --source-file <path> --dry-run --json\n  vida taskflow status --summary --json\n  vida taskflow run-graph status <run-id> --json\n  vida taskflow recovery status <run-id> --json\n  vida taskflow packet latest --json\n  vida taskflow bootstrap-spec \"feature request\" --json\n  vida task next --json\n\nParallelism guidance:\n  `vida taskflow graph explain <task-id> --json` explains one task's ready/blocked/parallel-safe posture from canonical projection truth.\n  `vida taskflow graph-summary --json` exposes `current_task_id`, `scheduling.ready[*].ready_parallel_safe`, `parallel_blockers`, and `parallel_candidates_after_current`.\n  `vida taskflow scheduler dispatch --json` turns that projection into a preview-first launch plan capped by `max_parallel_agents`.\n  `vida taskflow help parallelism` explains execution semantics fields and fail-closed scheduling rules.";

const DOCFLOW_LONG_ABOUT: &str = "Delegate to the DocFlow runtime family.\n\nDocFlow is the standalone documentation/readiness utility. Use it for documentation bootstrap, artifact init, validation, readiness checks, inventory, relations, and agent handoff instructions.";

const DOCFLOW_AFTER_HELP: &str = "Family entrypoints:\n  vida docflow help\n  vida docflow init\n  vida docflow init --json\n  vida docflow init --help\n  vida docflow repair-footer --help\n  vida docflow finalize-edit --help\n  vida docflow doctor --root .\n  vida docflow check-file --path <file> --json\n  vida docflow check --root . --json <file>\n  vida docflow readiness-check --profile active-canon\n  vida docflow registry --root .\n\nInit/repair contract:\n  `vida docflow init` without positional args prints agent bootstrap instructions.\n  `vida docflow init --json` prints the same contract as machine-readable JSON.\n  `vida docflow init <markdown_file> <artifact_path> <artifact_type> <change_note>` initializes a canonical markdown artifact.\n  `vida docflow repair-footer <markdown_file>` initializes missing footer metadata on legacy markdown files.";

const SERVICE_AFTER_HELP: &str = "Service operations:\n  vida service hello --json\n  vida service status --json\n  vida service capabilities --json\n  vida service endpoints --json\n  vida service endpoint-status --json\n  vida service lifecycle-plan --json\n  vida service lifecycle-status --json\n  vida service events --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const PROJECT_AFTER_HELP: &str = "Project operations:\n  vida project list --json\n  vida project resolve --project <project-id> --json\n  vida project status --project <project-id> --json\n\nOptions:\n  --project <project-id>    Project id or reference for project-scoped operations\n  --json                    Emit machine-readable JSON output";

const WIZARD_AFTER_HELP: &str = "Wizard operations:\n  vida wizard inspect --json\n  vida wizard draft --json\n  vida wizard validate --json\n  vida wizard diff --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const JOB_AFTER_HELP: &str = "Job operations:\n  vida job status --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const RECEIPT_AFTER_HELP: &str = "Receipt operations:\n  vida receipt get --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const TASK_CREATE_ABOUT: &str = "Create one tracked task in the authoritative backlog store.";
const TASK_CREATE_LONG_ABOUT: &str = "Create one tracked task in the authoritative backlog store.\n\nExecution semantics are additive to graph truth:\n- `--execution-mode sequential` keeps the task single-lane by default\n- `--execution-mode parallel_safe` allows parallel admission only when other semantics also match\n- `--execution-mode exclusive` blocks parallel execution\n- `--order-bucket`, `--parallel-group`, and `--conflict-domain` refine safe co-scheduling";
const TASK_CREATE_AFTER_HELP: &str = "Examples:\n  vida task create <task-id> <title> --parent-id <parent-id> --json\n  vida task create <task-id> --title <title> --json\n  vida task create <task-id> <title> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n\nNotes:\n  Provide exactly one title source: positional <title> or --title <title>.\n  Missing execution semantics fail closed for parallel scheduling.\n  Use `vida taskflow graph-summary --json` to verify parallel-safe admission after mutation.";

const TASK_UPDATE_ABOUT: &str = "Update one tracked task in the authoritative backlog store.";
const TASK_UPDATE_LONG_ABOUT: &str = "Update one tracked task in the authoritative backlog store.\n\nUse execution-semantics flags to correct sequencing and parallelism truth without moving ordering back into notes:\n- `--execution-mode sequential|parallel_safe|exclusive`\n- `--order-bucket <id>`\n- `--parallel-group <id>`\n- `--conflict-domain <id>`\n- matching `--clear-*` flags remove one semantics field";
const TASK_UPDATE_AFTER_HELP: &str = "Examples:\n  vida task update <task-id> --status in_progress --json\n  vida task update <task-id> --title \"Retitled task\" --priority 1 --json\n  vida task update <task-id> --parent-id <parent-id> --json\n  vida task update <task-id> --clear-parent-id --json\n  vida task update <task-id> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n  vida task update <task-id> --clear-parallel-group --clear-conflict-domain --json\n\nNotes:\n  Use either a value flag or the matching clear flag, not both.\n  Re-check `vida taskflow graph-summary --json` after updates to confirm `ready_parallel_safe` and `parallel_blockers`.";

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub(crate) enum RenderMode {
    #[default]
    Plain,
    Color,
    #[value(name = "color_emoji")]
    ColorEmoji,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub(crate) enum TaskHandoffStatusArg {
    #[default]
    Pass,
    Blocked,
}

impl TaskHandoffStatusArg {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "vida",
    bin_name = "vida",
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (built ",
        env!("VIDA_BUILD_TIMESTAMP_UTC"),
        ")"
    ),
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
    #[command(about = "preview delegated agent lane selection without executing dispatch")]
    Agent(AgentArgs),
    #[command(about = "resolve and render framework protocol/guide surfaces")]
    Protocol(ProtocolArgs),
    #[command(
        about = "inspect or repair project activation posture and bounded onboarding next steps"
    )]
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
    #[command(about = "run canonical runtime diagnostics for completed slices")]
    Diagnostics(DiagnosticsArgs),
    #[command(
        about = "service-first CLI surface backed by VidaClient service operations",
        after_help = SERVICE_AFTER_HELP
    )]
    Service(ProxyArgs),
    #[command(
        about = "service-first CLI surface backed by VidaClient project operations",
        after_help = PROJECT_AFTER_HELP
    )]
    Project(ProxyArgs),
    #[command(
        about = "service-first CLI surface backed by VidaClient wizard operations",
        after_help = WIZARD_AFTER_HELP
    )]
    Wizard(ProxyArgs),
    #[command(
        about = "service-first CLI surface backed by VidaClient job operations",
        after_help = JOB_AFTER_HELP
    )]
    Job(ProxyArgs),
    #[command(
        about = "service-first CLI surface backed by VidaClient receipt operations",
        after_help = RECEIPT_AFTER_HELP
    )]
    Receipt(ProxyArgs),
    #[command(about = "update scoped VIDA project documentation carriers")]
    Docs(DocsArgs),
    #[command(about = "inspect or reclaim VIDA orchestrator session ownership evidence")]
    OrchestratorSession(OrchestratorSessionArgs),
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
    #[command(about = "thin root alias to the TaskFlow route diagnostics family")]
    Route(ProxyArgs),
    #[command(about = "build and install the VIDA release binary")]
    Release(ReleaseArgs),
    #[command(
        about = "delegate to the TaskFlow runtime family",
        long_about = TASKFLOW_LONG_ABOUT,
        after_help = TASKFLOW_AFTER_HELP
    )]
    Taskflow(ProxyArgs),
    #[command(
        about = "delegate to the DocFlow runtime family",
        long_about = DOCFLOW_LONG_ABOUT,
        after_help = DOCFLOW_AFTER_HELP
    )]
    Docflow(ProxyArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct ProxyArgs {
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Family command arguments and options forwarded to the selected runtime surface"
    )]
    pub(crate) args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct AgentArgs {
    #[command(subcommand)]
    pub(crate) command: AgentCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum AgentCommand {
    #[command(
        about = "preview next bounded agent dispatch lanes with carrier/model/cost selection truth from TaskFlow readiness"
    )]
    DispatchNext(AgentDispatchNextArgs),
    #[command(
        about = "select a configured carrier/model/reasoning profile for one runtime role and task class"
    )]
    Select(AgentSelectArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AgentDispatchNextArgs {
    #[arg(
        long = "lanes",
        default_value_t = 4,
        help = "Maximum preview lanes to inspect before any manual `vida agent-init` launch"
    )]
    pub(crate) lanes: usize,

    #[arg(long = "scope", help = "Optional TaskFlow scope task id")]
    pub(crate) scope: Option<String>,

    #[arg(
        long = "current-task-id",
        help = "Optional current task id for parallel-safety checks"
    )]
    pub(crate) current_task_id: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,

    #[arg(
        long = "dev-team",
        help = "Preview configured dev-team flow sequence from vida.config.yaml, including analyst, developer, duplication reviewer, final coach, tester/prover, and release closure"
    )]
    pub(crate) dev_team: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AgentSelectArgs {
    #[arg(long = "runtime-role", default_value = "worker")]
    pub(crate) runtime_role: String,

    #[arg(long = "task-class", default_value = "implementation")]
    pub(crate) task_class: String,

    #[arg(long = "conversation-role", default_value = "orchestrator")]
    pub(crate) conversation_role: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct ReleaseArgs {
    #[command(subcommand)]
    pub(crate) command: ReleaseCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ReleaseCommand {
    #[command(
        about = "build and install target/release/vida to the canonical current/bin target",
        long_about = "Build and install the VIDA release binary.\n\nBy default this runs `cargo build -p vida --release` and installs the platform release binary to the canonical current/bin target under the VIDA install root. Use `--target path` to update the first `vida` found on PATH. Use `--skip-build` with `--source-binary` and `--install-root` for deterministic smoke tests or controlled local installs."
    )]
    Install(ReleaseInstallArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct ReleaseInstallArgs {
    #[arg(
        long = "target",
        default_value = "current",
        help = "Install target: current or path. Legacy all/local/cargo aliases resolve to current."
    )]
    pub(crate) target: String,

    #[arg(long = "skip-build", help = "Skip `cargo build -p vida --release`")]
    pub(crate) skip_build: bool,

    #[arg(
        long = "source-binary",
        help = "Source vida binary path; defaults to the platform release binary under target/release"
    )]
    pub(crate) source_binary: Option<PathBuf>,

    #[arg(
        long = "install-root",
        help = "Root used for install paths; defaults to HOME/USERPROFILE"
    )]
    pub(crate) install_root: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct DocsArgs {
    #[command(subcommand)]
    pub(crate) command: DocsCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum DocsCommand {
    #[command(
        about = "update only AGENTS.md and VIDA instruction protocol docs",
        long_about = "Update the current project's scoped VIDA documentation carriers.\n\nThis command is intentionally narrow: it rewrites only AGENTS.md and protocol markdown files ending in `-protocol.md` under vida/config/instructions. It does not update AGENTS.sidecar.md, vida.config.yaml, non-protocol instruction files, README.md, product docs, runtime state, or receipts."
    )]
    Update(DocsUpdateArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DocsUpdateArgs {
    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct TaskArgs {
    #[command(subcommand)]
    pub(crate) command: TaskCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskCommand {
    #[command(about = "show TaskFlow-owned help topics and compatibility aliases")]
    Help(TaskHelpArgs),
    #[command(about = "import backlog tasks from a JSONL snapshot file")]
    ImportJsonl(TaskImportJsonlArgs),
    #[command(about = "authoritatively replace backlog state from a canonical snapshot artifact")]
    ReplaceJsonl(TaskReplaceJsonlArgs),
    #[command(about = "export backlog tasks to a JSONL snapshot file")]
    ExportJsonl(TaskExportJsonlArgs),
    #[command(about = "list tracked backlog tasks with optional status filtering")]
    List(TaskListArgs),
    #[command(about = "show one tracked task with dependency and planner metadata")]
    Show(TaskShowArgs),
    #[command(about = "show progress and dependency context for one tracked task")]
    Progress(TaskDepsArgs),
    #[command(about = "list tasks ready for execution from canonical graph truth")]
    Ready(TaskReadyArgs),
    #[command(about = "select the next TaskFlow work item from current graph state")]
    Next(TaskNextArgs),
    #[command(about = "resolve the next lawful task continuation item without heuristic guessing")]
    NextLawful(TaskNextLawfulArgs),
    #[command(about = "allocate the next child display id under a parent display id")]
    NextDisplayId(TaskNextDisplayIdArgs),
    #[command(
        about = TASK_CREATE_ABOUT,
        long_about = TASK_CREATE_LONG_ABOUT,
        after_help = TASK_CREATE_AFTER_HELP
    )]
    Create(TaskCreateArgs),
    #[command(about = "create the task if missing while preserving an existing task")]
    Ensure(TaskCreateArgs),
    #[command(
        about = TASK_UPDATE_ABOUT,
        long_about = TASK_UPDATE_LONG_ABOUT,
        after_help = TASK_UPDATE_AFTER_HELP
    )]
    Update(TaskUpdateArgs),
    #[command(about = "inspect dirty git files against one task's owned paths")]
    OwnedStatus(TaskOwnedStatusArgs),
    #[command(about = "record delegated agent handoff receipts for a task")]
    Handoff(TaskHandoffArgs),
    #[command(about = "close one tracked task with evidence and optional release automation")]
    Close(TaskCloseArgs),
    #[command(about = "retire historical run-graph rows for already-closed tasks")]
    ReconcileClosedRuns(TaskReconcileClosedRunsArgs),
    #[command(about = "split one oversized task into bounded child tasks")]
    Split(TaskSplitArgs),
    #[command(about = "create a blocker/dependency task linked to one blocked source task")]
    SpawnBlocker(TaskSpawnBlockerArgs),
    #[command(
        about = "preview adaptive replanner finding classification without mutating graph state"
    )]
    AdaptivePreview(TaskAdaptivePreviewArgs),
    #[command(about = "show direct dependency edges for one task")]
    Deps(TaskDepsArgs),
    #[command(about = "show tasks that depend on one task")]
    ReverseDeps(TaskDepsArgs),
    #[command(about = "list blocked tasks and graph blockers")]
    Blocked(TaskBlockedArgs),
    #[command(about = "inspect direct children for one task from the authoritative backlog store")]
    Children(TaskDepsArgs),
    #[command(
        about = "bulk-reparent direct children from one parent task to another",
        alias = "move-children"
    )]
    ReparentChildren(TaskBulkReparentArgs),
    #[command(
        about = "atomically rehome a defect batch and update pause/start task states",
        alias = "defect-batch"
    )]
    DefectBatchRehome(TaskDefectBatchRehomeArgs),
    #[command(
        about = "inspect one recursive task subtree from the authoritative backlog store",
        alias = "subtree"
    )]
    Tree(TaskDepsArgs),
    #[command(about = "validate dependency graph consistency and parent-child structure")]
    ValidateGraph(TaskBlockedArgs),
    #[command(about = "mutate direct dependency edges for tracked tasks")]
    Dep(TaskDepArgs),
    #[command(about = "show the current critical path through blocked and ready tasks")]
    CriticalPath(TaskBlockedArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskDepArgs {
    #[command(subcommand)]
    pub(crate) command: TaskDependencyCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskDependencyCommand {
    #[command(about = "add one dependency edge between two tracked tasks")]
    Add(TaskDependencyMutationCommandArgs),
    #[command(about = "add multiple dependency edges from flags or an edge file")]
    AddBulk(TaskDependencyBulkAddCommandArgs),
    #[command(about = "remove one dependency edge between two tracked tasks")]
    Remove(TaskDependencyTargetCommandArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskHelpArgs {
    pub(crate) topic: Option<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDependencyMutationCommandArgs {
    #[arg(help = "Task id that will receive the dependency edge")]
    pub(crate) task_id: String,
    #[arg(help = "Task id that the task depends on")]
    pub(crate) depends_on_id: String,
    #[arg(help = "Dependency edge type, such as parent-child or blocks")]
    pub(crate) edge_type: String,

    #[arg(long = "created-by", default_value = "vida")]
    pub(crate) created_by: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDependencyTargetCommandArgs {
    #[arg(help = "Task id that currently owns the dependency edge")]
    pub(crate) task_id: String,
    #[arg(help = "Task id currently referenced by the dependency edge")]
    pub(crate) depends_on_id: String,
    #[arg(help = "Dependency edge type to remove")]
    pub(crate) edge_type: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskDependencyBulkAddCommandArgs {
    #[arg(
        long = "edge",
        help = "Dependency edge in issue_id:depends_on_id:edge_type format; repeat for each edge"
    )]
    pub(crate) edges: Vec<String>,

    #[arg(
        long = "edge-file",
        help = "Read dependency edges from a newline-delimited issue_id:depends_on_id:edge_type file"
    )]
    pub(crate) edge_file: Option<PathBuf>,

    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,

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
pub(crate) struct TaskImportJsonlArgs {
    #[arg(help = "Path to the JSONL task snapshot to import")]
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
    #[arg(help = "Path to the canonical JSONL task snapshot to apply")]
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
    #[arg(help = "Path where the JSONL task snapshot should be written")]
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
    #[arg(help = "Task id to inspect")]
    pub(crate) task_id: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskOwnedStatusArgs {
    #[arg(help = "Task id whose owned paths should be checked against git status")]
    pub(crate) task_id: String,

    #[arg(
        long = "file",
        help = "Explicit owned path override. Repeat for multiple files or directories."
    )]
    pub(crate) files: Vec<PathBuf>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskHandoffArgs {
    #[command(subcommand)]
    pub(crate) command: TaskHandoffCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskHandoffCommand {
    #[command(about = "accept and persist one delegated agent handoff receipt")]
    Accept(TaskHandoffAcceptArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskHandoffAcceptArgs {
    #[arg(help = "Task id receiving the delegated handoff receipt")]
    pub(crate) task_id: String,

    #[arg(
        long = "agent",
        help = "Delegated agent or carrier id that produced the handoff"
    )]
    pub(crate) agent: Option<String>,

    #[arg(
        long = "file",
        help = "Changed file path reported by the handoff; repeat for multiple paths"
    )]
    pub(crate) files: Vec<PathBuf>,

    #[arg(
        long = "proof",
        help = "Proof command reported by the handoff; repeat for multiple commands"
    )]
    pub(crate) proofs: Vec<String>,

    #[arg(long = "status", value_enum, default_value_t = TaskHandoffStatusArg::Pass)]
    pub(crate) status: TaskHandoffStatusArg,

    #[arg(
        long = "blocker",
        visible_alias = "blocker-code",
        help = "Blocker code for blocked handoffs; repeat for multiple blockers"
    )]
    pub(crate) blockers: Vec<String>,

    #[arg(
        long = "next-action",
        help = "Operator next action for blocked or incomplete handoffs; repeat for multiple actions"
    )]
    pub(crate) next_actions: Vec<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskNextDisplayIdArgs {
    #[arg(help = "Parent display id whose next child display id should be allocated")]
    pub(crate) parent_display_id: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskCreateArgs {
    #[arg(help = "Stable task id to create in the authoritative backlog store")]
    pub(crate) task_id: String,

    #[arg(value_name = "TITLE", help = "Task title; alternatively pass --title")]
    pub(crate) positional_title: Option<String>,

    #[arg(
        long = "title",
        value_name = "TITLE",
        help = "Task title; alternative to positional <TITLE>"
    )]
    pub(crate) title: Option<String>,

    #[arg(
        long = "type",
        default_value = "task",
        help = "Task issue type to store, such as task, todo, defect, or epic"
    )]
    pub(crate) issue_type: String,

    #[arg(
        long = "status",
        default_value = "open",
        help = "Initial task status to store in the backlog"
    )]
    pub(crate) status: String,

    #[arg(
        long = "priority",
        default_value_t = 2,
        help = "Task priority where lower numbers are selected earlier"
    )]
    pub(crate) priority: u32,

    #[arg(long = "display-id", help = "Human-readable display id to assign")]
    pub(crate) display_id: Option<String>,

    #[arg(long = "parent-id", help = "Parent task id for a child task edge")]
    pub(crate) parent_id: Option<String>,

    #[arg(
        long = "parent-display-id",
        help = "Parent display id used when resolving or deriving display ids"
    )]
    pub(crate) parent_display_id: Option<String>,

    #[arg(
        long = "auto-display-from",
        help = "Parent display id whose next child display id should be generated"
    )]
    pub(crate) auto_display_from: Option<String>,

    #[arg(
        long = "description",
        default_value = "",
        help = "Task description stored in the backlog"
    )]
    pub(crate) description: String,

    #[arg(long = "notes", help = "Task notes stored in the backlog")]
    pub(crate) notes: Option<String>,

    #[arg(long = "notes-file", help = "Read task notes from this file path")]
    pub(crate) notes_file: Option<PathBuf>,

    #[arg(
        long = "labels",
        value_delimiter = ',',
        help = "Task labels. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) labels: Vec<String>,

    #[arg(
        long = "execution-mode",
        help = "Execution scheduling mode: sequential, parallel_safe, or exclusive"
    )]
    pub(crate) execution_mode: Option<String>,

    #[arg(
        long = "order-bucket",
        help = "Ordering bucket used to keep related work sequenced"
    )]
    pub(crate) order_bucket: Option<String>,

    #[arg(
        long = "parallel-group",
        help = "Parallel admission group for compatible parallel-safe tasks"
    )]
    pub(crate) parallel_group: Option<String>,

    #[arg(
        long = "conflict-domain",
        help = "Conflict domain that prevents unsafe co-scheduling"
    )]
    pub(crate) conflict_domain: Option<String>,

    #[arg(
        long = "owned-path",
        value_delimiter = ',',
        help = "Planner metadata owned paths to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) owned_paths: Vec<String>,

    #[arg(
        long = "acceptance-target",
        value_delimiter = ',',
        help = "Planner metadata acceptance targets to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) acceptance_targets: Vec<String>,

    #[arg(
        long = "proof-target",
        value_delimiter = ',',
        help = "Planner metadata proof targets to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) proof_targets: Vec<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskUpdateArgs {
    #[arg(help = "Task id to update in the authoritative backlog store")]
    pub(crate) task_id: String,

    #[arg(long = "title", help = "Replacement task title")]
    pub(crate) title: Option<String>,

    #[arg(long = "status", help = "Replacement task status")]
    pub(crate) status: Option<String>,

    #[arg(long = "priority", help = "Replacement task priority")]
    pub(crate) priority: Option<u32>,

    #[arg(long = "notes", help = "Replacement task notes")]
    pub(crate) notes: Option<String>,

    #[arg(
        long = "notes-file",
        help = "Read replacement task notes from this file path"
    )]
    pub(crate) notes_file: Option<PathBuf>,

    #[arg(long = "description", help = "Replacement task description")]
    pub(crate) description: Option<String>,

    #[arg(long = "parent-id", help = "Replacement parent task id")]
    pub(crate) parent_id: Option<String>,

    #[arg(long = "clear-parent-id", help = "Remove the current parent task edge")]
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

    #[arg(
        long = "execution-mode",
        help = "Replacement execution scheduling mode"
    )]
    pub(crate) execution_mode: Option<String>,

    #[arg(long = "order-bucket", help = "Replacement ordering bucket")]
    pub(crate) order_bucket: Option<String>,

    #[arg(long = "parallel-group", help = "Replacement parallel admission group")]
    pub(crate) parallel_group: Option<String>,

    #[arg(long = "conflict-domain", help = "Replacement conflict domain")]
    pub(crate) conflict_domain: Option<String>,

    #[arg(
        long = "owned-path",
        value_delimiter = ',',
        help = "Planner metadata owned paths to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) owned_paths: Vec<String>,

    #[arg(
        long = "acceptance-target",
        value_delimiter = ',',
        help = "Planner metadata acceptance targets to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) acceptance_targets: Vec<String>,

    #[arg(
        long = "proof-target",
        value_delimiter = ',',
        help = "Planner metadata proof targets to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) proof_targets: Vec<String>,

    #[arg(
        long = "clear-execution-mode",
        help = "Remove the task execution scheduling mode"
    )]
    pub(crate) clear_execution_mode: bool,

    #[arg(long = "clear-order-bucket", help = "Remove the task ordering bucket")]
    pub(crate) clear_order_bucket: bool,

    #[arg(
        long = "clear-parallel-group",
        help = "Remove the task parallel admission group"
    )]
    pub(crate) clear_parallel_group: bool,

    #[arg(
        long = "clear-conflict-domain",
        help = "Remove the task conflict domain"
    )]
    pub(crate) clear_conflict_domain: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "render",
        env = "VIDA_RENDER",
        value_enum,
        default_value_t = RenderMode::Plain,
        help = "Render output mode for human-readable command output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskCloseArgs {
    #[arg(help = "Task id to close")]
    pub(crate) task_id: String,

    #[arg(long = "reason", help = "Closure reason and evidence summary")]
    pub(crate) reason: String,

    #[arg(long = "source", hide = true)]
    pub(crate) source: Option<String>,

    #[arg(long = "release", help = "Run a release build after successful close")]
    pub(crate) release: bool,

    #[arg(
        long = "install",
        help = "Install the release binary after successful close"
    )]
    pub(crate) install: bool,

    #[arg(
        long = "install-target",
        default_value = "current",
        help = "Release install target when --install is set: current or path"
    )]
    pub(crate) install_target: String,

    #[arg(
        long = "skip-release-build",
        help = "Skip the release build during --install"
    )]
    pub(crate) skip_release_build: bool,

    #[arg(
        long = "source-binary",
        help = "Source vida binary path for --install; defaults to target/release/vida"
    )]
    pub(crate) source_binary: Option<PathBuf>,

    #[arg(
        long = "install-root",
        help = "Root used for release install paths; defaults to HOME"
    )]
    pub(crate) install_root: Option<PathBuf>,

    #[arg(
        long = "commit",
        help = "Commit explicit --commit-file paths after close"
    )]
    pub(crate) commit: bool,

    #[arg(long = "push", help = "Push after an explicit post-close commit")]
    pub(crate) push: bool,

    #[arg(
        long = "stage-owned",
        help = "For --commit, stage dirty files covered by task planner_metadata.owned_paths"
    )]
    pub(crate) stage_owned: bool,

    #[arg(
        long = "commit-file",
        help = "File path owned by this bounded task to stage and commit; repeat for multiple paths"
    )]
    pub(crate) commit_files: Vec<PathBuf>,

    #[arg(
        long = "commit-message",
        help = "Commit message for --commit; defaults to a task-close message"
    )]
    pub(crate) commit_message: Option<String>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskReconcileClosedRunsArgs {
    #[arg(long = "limit", default_value_t = 100)]
    pub(crate) limit: usize,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskSplitArgs {
    #[arg(help = "Oversized parent task id to split")]
    pub(crate) task_id: String,

    #[arg(
        long = "child",
        required = true,
        help = "Child spec in `<task-id>:<title>` form. Repeat for multiple bounded children."
    )]
    pub(crate) children: Vec<String>,

    #[arg(
        long = "reason",
        help = "Reason the task must be split into bounded children"
    )]
    pub(crate) reason: String,

    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskSpawnBlockerArgs {
    #[arg(help = "Blocked source task id")]
    pub(crate) task_id: String,
    #[arg(help = "New blocker task id to create")]
    pub(crate) blocker_task_id: String,
    #[arg(help = "Title for the new blocker task")]
    pub(crate) title: String,

    #[arg(
        long = "reason",
        help = "Reason this blocker prevents the source task from progressing"
    )]
    pub(crate) reason: String,

    #[arg(long = "description")]
    pub(crate) description: Option<String>,

    #[arg(long = "type", default_value = "task")]
    pub(crate) issue_type: String,

    #[arg(long = "status", default_value = "open")]
    pub(crate) status: String,

    #[arg(long = "priority")]
    pub(crate) priority: Option<u32>,

    #[arg(
        long = "labels",
        value_delimiter = ',',
        help = "Blocker task labels. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) labels: Vec<String>,

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
pub(crate) struct TaskAdaptivePreviewArgs {
    #[arg(long = "finding-json")]
    pub(crate) finding_json: Option<String>,

    #[arg(long = "finding-file")]
    pub(crate) finding_file: Option<PathBuf>,

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

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskNextLawfulArgs {
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
pub(crate) struct TaskDepsArgs {
    #[arg(help = "Task id whose dependency graph should be inspected")]
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
    #[arg(help = "Current parent task id for children being moved")]
    pub(crate) from_parent_id: String,
    #[arg(help = "New parent task id for moved children")]
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
pub(crate) struct TaskDefectBatchRehomeArgs {
    #[arg(help = "Current parent task id for defect tasks being moved")]
    pub(crate) from_parent_id: String,
    #[arg(help = "New parent task id for defect tasks being moved")]
    pub(crate) to_parent_id: String,

    #[arg(
        long = "child-id",
        help = "Only move the listed direct child ids. Repeat to move a subset."
    )]
    pub(crate) child_ids: Vec<String>,

    #[arg(
        long = "pause-task-id",
        help = "Task id to mark paused in the same validated mutation. Repeat to pause multiple tasks."
    )]
    pub(crate) pause_task_ids: Vec<String>,

    #[arg(
        long = "start-task-id",
        help = "Task id to mark in_progress in the same validated mutation. Repeat to start multiple tasks."
    )]
    pub(crate) start_task_ids: Vec<String>,

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

    #[arg(
        long = "full",
        help = "Render the full init envelope; routine --json output defaults to compact summary"
    )]
    pub(crate) full: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct ProjectActivatorArgs {
    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Explicit project activation state dir; defaults to the current project's authoritative .vida/data/state"
    )]
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

    #[arg(
        long = "repair",
        help = "Materialize missing safe-default project activation/config/docs projections and selected host template when possible"
    )]
    pub(crate) repair: bool,

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
    #[arg(help = "Optional request text to classify into a bounded agent lane")]
    pub(crate) request_text: Option<String>,

    #[arg(
        long = "role",
        help = "Requested runtime role or conversation role for lane activation"
    )]
    pub(crate) role: Option<String>,

    #[arg(
        long = "dispatch-packet",
        help = "Runtime dispatch packet path to activate"
    )]
    pub(crate) dispatch_packet: Option<String>,

    #[arg(
        long = "downstream-packet",
        help = "Runtime downstream dispatch packet path to activate"
    )]
    pub(crate) downstream_packet: Option<String>,

    #[arg(
        long = "execute-dispatch",
        help = "Execute the packet handoff instead of rendering only an activation view"
    )]
    pub(crate) execute_dispatch: bool,

    #[arg(
        long = "auto-dispatch-packet",
        help = "Build a dispatch packet for the active runtime unit before execution"
    )]
    pub(crate) auto_dispatch_packet: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
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

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct DiagnosticsArgs {
    #[command(subcommand)]
    pub(crate) command: DiagnosticsCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum DiagnosticsCommand {
    #[command(
        about = "reconcile git, TaskFlow, DocFlow, run-graph, dispatch, owner, and issue workflow evidence after commit"
    )]
    PostCommit(DiagnosticsPostCommitArgs),
    #[command(about = "check whether a bounded gate has concrete evidence refs")]
    EvidenceCheck(DiagnosticsEvidenceCheckArgs),
    #[command(about = "check changed paths and protocol ids against read-only runtime rules")]
    RulesCheck(DiagnosticsRulesCheckArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DiagnosticsPostCommitArgs {
    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DiagnosticsEvidenceCheckArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "task-id")]
    pub(crate) task_id: Option<String>,

    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DiagnosticsRulesCheckArgs {
    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "task-id")]
    pub(crate) task_id: Option<String>,

    #[arg(long = "changed-path")]
    pub(crate) changed_paths: Vec<PathBuf>,

    #[arg(long = "protocol-id")]
    pub(crate) protocol_ids: Vec<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct OrchestratorSessionArgs {
    #[command(subcommand)]
    pub(crate) command: OrchestratorSessionCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum OrchestratorSessionCommand {
    #[command(about = "show current, live, stale, and legacy owner evidence")]
    Show(OrchestratorSessionShowArgs),
    #[command(about = "mark a stale orchestrator session as reclaimed by the current session")]
    Reclaim(OrchestratorSessionReclaimArgs),
    #[command(about = "transfer a stale orchestrator session to the current session")]
    Transfer(OrchestratorSessionTransferArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct OrchestratorSessionShowArgs {
    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct OrchestratorSessionReclaimArgs {
    #[arg(help = "Stale orchestrator session id to reclaim")]
    pub(crate) session_id: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct OrchestratorSessionTransferArgs {
    #[arg(help = "Stale orchestrator session id to transfer")]
    pub(crate) session_id: String,

    #[arg(
        long = "to-current",
        help = "Transfer the stale session to the current orchestrator session"
    )]
    pub(crate) to_current: bool,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[cfg(test)]
mod tests {
    use super::{Cli, TaskCommand};
    use clap::{CommandFactory, Parser};

    fn assert_help_has_no_blank_description_rows(label: &str, help: &str) {
        let lines: Vec<&str> = help.lines().collect();
        let blank_rows: Vec<usize> = help
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let previous = if index == 0 {
                    ""
                } else {
                    lines[index - 1].trim()
                };
                let follows_option_or_argument = previous.starts_with('-')
                    || previous.starts_with('<')
                    || previous.starts_with('[');
                if follows_option_or_argument && !line.is_empty() && line.trim().is_empty() {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            blank_rows.is_empty(),
            "{label} help should not contain blank description rows at lines {blank_rows:?}:\n{help}"
        );
    }

    #[test]
    fn root_version_includes_build_timestamp_to_seconds() {
        let build_timestamp = env!("VIDA_BUILD_TIMESTAMP_UTC");
        assert_eq!(
            build_timestamp.len(),
            20,
            "build timestamp should use UTC RFC3339 seconds"
        );
        assert!(build_timestamp.contains('T'));
        assert!(build_timestamp.ends_with('Z'));
        assert!(
            !build_timestamp.contains('.'),
            "build timestamp should not include subsecond precision"
        );

        let version = Cli::command()
            .get_version()
            .expect("root CLI version should be configured")
            .to_string();
        assert_eq!(
            version,
            format!("{} (built {build_timestamp})", env!("CARGO_PKG_VERSION"))
        );
    }

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

    #[test]
    fn task_close_help_lists_release_automation_options() {
        let error = Cli::try_parse_from(["vida", "task", "close", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("--release"));
        assert!(help.contains("--install"));
        assert!(help.contains("--install-target"));
        assert!(help.contains("--skip-release-build"));
        assert!(help.contains("--commit"));
        assert!(help.contains("--push"));
        assert!(help.contains("--commit-file"));
        assert!(help.contains("--commit-message"));
    }

    #[test]
    fn task_create_help_lists_positional_and_title_option() {
        let error = Cli::try_parse_from(["vida", "task", "create", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("<TASK_ID>"));
        assert!(help.contains("[TITLE]"));
        assert!(help.contains("--title <TITLE>"));
        assert!(help.contains("Provide exactly one title source"));
    }

    #[test]
    fn task_defect_batch_rehome_help_and_options_are_discoverable() {
        let error = Cli::try_parse_from(["vida", "task", "defect-batch-rehome", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();
        assert!(help.contains("<FROM_PARENT_ID>"));
        assert!(help.contains("<TO_PARENT_ID>"));
        assert!(help.contains("--child-id"));
        assert!(help.contains("--pause-task-id"));
        assert!(help.contains("--start-task-id"));
        assert!(help.contains("--dry-run"));
        assert!(help.contains("--json"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "defect-batch-rehome",
            "old-epic",
            "new-epic",
            "--child-id",
            "defect-a",
            "--pause-task-id",
            "old-active",
            "--start-task-id",
            "new-active",
            "--dry-run",
            "--json",
        ])
        .expect("defect-batch-rehome should parse");
        let Some(super::Command::Task(task_args)) = parsed.command else {
            panic!("task command should parse");
        };
        let TaskCommand::DefectBatchRehome(command) = task_args.command else {
            panic!("defect-batch-rehome command should parse");
        };
        assert_eq!(command.from_parent_id, "old-epic");
        assert_eq!(command.to_parent_id, "new-epic");
        assert_eq!(command.child_ids, vec!["defect-a".to_string()]);
        assert_eq!(command.pause_task_ids, vec!["old-active".to_string()]);
        assert_eq!(command.start_task_ids, vec!["new-active".to_string()]);
        assert!(command.dry_run);
        assert!(command.json);
    }

    #[test]
    fn task_owned_status_help_and_close_stage_owned_are_discoverable() {
        let owned_error = Cli::try_parse_from(["vida", "task", "owned-status", "--help"])
            .expect_err("help should render clap display error");
        let owned_help = owned_error.to_string();
        assert!(owned_help.contains("<TASK_ID>"));
        assert!(owned_help.contains("--file"));
        assert!(owned_help.contains("--json"));

        let close_error = Cli::try_parse_from(["vida", "task", "close", "--help"])
            .expect_err("help should render clap display error");
        let close_help = close_error.to_string();
        assert!(close_help.contains("--stage-owned"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "close",
            "task-owned",
            "--reason",
            "done",
            "--stage-owned",
        ])
        .expect("--stage-owned should parse");
        let Some(super::Command::Task(task_args)) = parsed.command else {
            panic!("task command should parse");
        };
        let TaskCommand::Close(close) = task_args.command else {
            panic!("close command should parse");
        };
        assert!(close.stage_owned);
        assert!(!close.commit);
    }

    #[test]
    fn task_handoff_accept_help_is_discoverable() {
        let handoff_error = Cli::try_parse_from(["vida", "task", "handoff", "--help"])
            .expect_err("help should render clap display error");
        let handoff_help = handoff_error.to_string();
        assert!(handoff_help.contains("accept"));

        let accept_error = Cli::try_parse_from(["vida", "task", "handoff", "accept", "--help"])
            .expect_err("help should render clap display error");
        let accept_help = accept_error.to_string();
        assert!(accept_help.contains("<TASK_ID>"));
        assert!(accept_help.contains("--agent"));
        assert!(accept_help.contains("--file"));
        assert!(accept_help.contains("--proof"));
        assert!(accept_help.contains("--status"));
        assert!(accept_help.contains("--json"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "handoff",
            "accept",
            "task-a",
            "--agent",
            "worker-1",
            "--file",
            "crates/vida/src/task_surface.rs",
            "--proof",
            "cargo test -p vida --bin vida task_handoff",
            "--json",
        ])
        .expect("handoff accept should parse");
        let Some(super::Command::Task(task_args)) = parsed.command else {
            panic!("task command should parse");
        };
        let TaskCommand::Handoff(handoff) = task_args.command else {
            panic!("handoff command should parse");
        };
        let crate::TaskHandoffCommand::Accept(accept) = handoff.command;
        assert_eq!(accept.task_id, "task-a");
        assert_eq!(accept.agent.as_deref(), Some("worker-1"));
        assert_eq!(accept.files.len(), 1);
        assert_eq!(accept.proofs.len(), 1);
        assert_eq!(accept.status.as_str(), "pass");
    }

    #[test]
    fn agent_dispatch_next_help_is_discoverable() {
        let root_error = Cli::try_parse_from(["vida", "--help"])
            .expect_err("help should render clap display error");
        let root_help = root_error.to_string();
        assert!(root_help.contains("agent"));

        let agent_error = Cli::try_parse_from(["vida", "agent", "--help"])
            .expect_err("help should render clap display error");
        let agent_help = agent_error.to_string();
        assert!(agent_help.contains("dispatch-next"));

        let dispatch_error = Cli::try_parse_from(["vida", "agent", "dispatch-next", "--help"])
            .expect_err("help should render clap display error");
        let dispatch_help = dispatch_error.to_string();
        assert!(dispatch_help.contains("--lanes"));
        assert!(dispatch_help.contains("--scope"));
        assert!(dispatch_help.contains("--current-task-id"));
        assert!(dispatch_help.contains("--state-dir"));
        assert!(dispatch_help.contains("--json"));
        assert!(dispatch_help.contains("--dev-team"));

        let parsed = Cli::try_parse_from([
            "vida",
            "agent",
            "dispatch-next",
            "--lanes",
            "4",
            "--scope",
            "audit-epic",
            "--state-dir",
            "/tmp/vida-state",
            "--json",
        ])
        .expect("agent dispatch-next should parse");
        let Some(super::Command::Agent(agent_args)) = parsed.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::DispatchNext(dispatch) = agent_args.command else {
            panic!("agent dispatch-next command should parse");
        };
        assert_eq!(dispatch.lanes, 4);
        assert_eq!(dispatch.scope.as_deref(), Some("audit-epic"));
        assert_eq!(
            dispatch
                .state_dir
                .as_ref()
                .map(|path| path.display().to_string()),
            Some("/tmp/vida-state".to_string())
        );
        assert!(!dispatch.dev_team);
        assert!(dispatch.json);

        let dispatch_dev_team = Cli::try_parse_from([
            "vida",
            "agent",
            "dispatch-next",
            "--lanes",
            "5",
            "--dev-team",
        ])
        .expect("agent dispatch-next should parse");
        let Some(super::Command::Agent(agent_args)) = dispatch_dev_team.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::DispatchNext(dispatch_dev_team) = agent_args.command else {
            panic!("agent dispatch-next command should parse");
        };
        assert!(dispatch_dev_team.dev_team);
        assert_eq!(dispatch_dev_team.lanes, 5);

        let parsed_select = Cli::try_parse_from([
            "vida",
            "agent",
            "select",
            "--runtime-role",
            "verifier",
            "--task-class",
            "verification",
            "--json",
        ])
        .expect("agent select should parse");
        let Some(super::Command::Agent(agent_args)) = parsed_select.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::Select(select) = agent_args.command else {
            panic!("agent select command should parse");
        };
        assert_eq!(select.runtime_role, "verifier");
        assert_eq!(select.task_class, "verification");
        assert!(select.json);
    }

    #[test]
    fn task_next_lawful_help_is_discoverable() {
        let task_help_error = Cli::try_parse_from(["vida", "task", "--help"])
            .expect_err("help should render clap display error");
        let task_help = task_help_error.to_string();
        assert!(task_help.contains("next-lawful"));

        let next_lawful_error = Cli::try_parse_from(["vida", "task", "next-lawful", "--help"])
            .expect_err("help should render clap display error");
        let next_lawful_help = next_lawful_error.to_string();
        assert!(next_lawful_help.contains("--scope"));
        assert!(next_lawful_help.contains("--state-dir"));
        assert!(next_lawful_help.contains("--json"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "next-lawful",
            "--scope",
            "audit-epic",
            "--json",
        ])
        .expect("next-lawful should parse");
        let Some(super::Command::Task(task_args)) = parsed.command else {
            panic!("task command should parse");
        };
        let TaskCommand::NextLawful(next_lawful) = task_args.command else {
            panic!("next-lawful command should parse");
        };
        assert_eq!(next_lawful.scope.as_deref(), Some("audit-epic"));
        assert!(next_lawful.json);
    }

    #[test]
    fn cli_help_description_inventory_covers_task_family_defect() {
        let task_error = Cli::try_parse_from(["vida", "task", "--help"])
            .expect_err("help should render clap display error");
        let task_help = task_error.to_string();
        assert_help_has_no_blank_description_rows("task", &task_help);
        for expected in [
            "show TaskFlow-owned help topics",
            "import backlog tasks from a JSONL snapshot file",
            "list tracked backlog tasks with optional status filtering",
            "show one tracked task with dependency and planner metadata",
            "allocate the next child display id",
            "close one tracked task with evidence",
            "validate dependency graph consistency",
            "mutate direct dependency edges for tracked tasks",
            "show the current critical path",
        ] {
            assert!(
                task_help.contains(expected),
                "task help should contain description fragment `{expected}`:\n{task_help}"
            );
        }

        let dep_error = Cli::try_parse_from(["vida", "task", "dep", "--help"])
            .expect_err("help should render clap display error");
        let dep_help = dep_error.to_string();
        assert_help_has_no_blank_description_rows("task dep", &dep_help);
        for expected in [
            "add one dependency edge",
            "add multiple dependency edges",
            "remove one dependency edge",
        ] {
            assert!(
                dep_help.contains(expected),
                "task dep help should contain `{expected}`:\n{dep_help}"
            );
        }

        let create_error = Cli::try_parse_from(["vida", "task", "create", "--help"])
            .expect_err("help should render clap display error");
        let create_help = create_error.to_string();
        assert_help_has_no_blank_description_rows("task create", &create_help);
        assert!(create_help.contains("Stable task id to create"));
        assert!(create_help.contains("Emit machine-readable JSON output"));

        let update_error = Cli::try_parse_from(["vida", "task", "update", "--help"])
            .expect_err("help should render clap display error");
        let update_help = update_error.to_string();
        assert_help_has_no_blank_description_rows("task update", &update_help);
        assert!(update_help.contains("Replacement execution scheduling mode"));

        let show_error = Cli::try_parse_from(["vida", "task", "show", "--help"])
            .expect_err("help should render clap display error");
        let show_help = show_error.to_string();
        assert_help_has_no_blank_description_rows("task show", &show_help);
        assert!(show_help.contains("Task id to inspect"));

        let agent_init_error = Cli::try_parse_from(["vida", "agent-init", "--help"])
            .expect_err("help should render clap display error");
        let agent_init_help = agent_init_error.to_string();
        assert_help_has_no_blank_description_rows("agent-init", &agent_init_help);
        assert!(agent_init_help.contains("Optional request text"));
        assert!(agent_init_help.contains("Execute the packet handoff"));

        let reclaim_error =
            Cli::try_parse_from(["vida", "orchestrator-session", "reclaim", "--help"])
                .expect_err("help should render clap display error");
        let reclaim_help = reclaim_error.to_string();
        assert_help_has_no_blank_description_rows("orchestrator-session reclaim", &reclaim_help);
        assert!(reclaim_help.contains("Stale orchestrator session id"));
    }

    #[test]
    fn task_create_accepts_positional_title_and_title_option() {
        let positional = Cli::try_parse_from(["vida", "task", "create", "task-a", "Task A"])
            .expect("positional title should parse");
        let Some(super::Command::Task(task_args)) = positional.command else {
            panic!("task command should parse");
        };
        let TaskCommand::Create(create) = task_args.command else {
            panic!("create command should parse");
        };
        assert_eq!(create.task_id, "task-a");
        assert_eq!(create.positional_title.as_deref(), Some("Task A"));
        assert_eq!(create.title, None);

        let option = Cli::try_parse_from(["vida", "task", "create", "task-b", "--title", "Task B"])
            .expect("--title should parse");
        let Some(super::Command::Task(task_args)) = option.command else {
            panic!("task command should parse");
        };
        let TaskCommand::Create(create) = task_args.command else {
            panic!("create command should parse");
        };
        assert_eq!(create.task_id, "task-b");
        assert_eq!(create.positional_title, None);
        assert_eq!(create.title.as_deref(), Some("Task B"));
    }
}
