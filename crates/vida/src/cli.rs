use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const ROOT_AFTER_HELP: &str = "Runtime-family help paths:\n  vida taskflow help\n  vida task --help\n  vida taskflow help parallelism\n  vida route explain --json\n  vida state reset --archive --reinit --json\n  vida docflow help\n  vida docs update --json";

const TASK_LONG_ABOUT: &str = "Task inspection, mutation, and graph routing over the authoritative state store.\n\nUse `vida task` for the canonical backlog contract. Parent-child edges preserve structure, `blocks` edges preserve ordering, and execution semantics add fail-closed sequencing/parallelism metadata on top of graph truth.";

const TASK_AFTER_HELP: &str = "Most-used task commands:\n  vida task ready\n  vida task next\n  vida task show <task-id>\n  vida task progress <task-id>\n  vida task deps <task-id>\n  vida task tree <task-id>\n  vida task import --file tasks.jsonl --parent-id <parent-id> --dry-run\n  vida task split <task-id> --child child-a:\"First slice\" --child child-b:\"Second slice\" --reason \"oversized task\"\n  vida task spawn-blocker <task-id> <blocker-task-id> \"Blocker title\" --reason \"new dependency\"\n  vida task reparent-children <from-parent-id> <to-parent-id>\n  vida task defect-batch-rehome <from-parent-id> <to-parent-id> --pause-task-id <task-id> --start-task-id <task-id>\n  vida task critical-path\n  vida taskflow help parallelism\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.\n\nLarge-batch transport:\n  Use `vida task import --file tasks.jsonl --dry-run` for many task creates instead of pasting oversized shell payloads.\n  Use JSONL/NDJSON files for large batches and `vida task dep add-bulk --edge-file edges.txt --dry-run` for many dependency edges.\n\nParallelism guidance:\n  Use `vida taskflow help parallelism` for the canonical execution_mode/order_bucket/parallel_group/conflict_domain contract.\n  `vida task help parallelism` remains a compatibility alias to the same TaskFlow-owned help.\n  Use `vida taskflow graph-summary` for the default operator summary; add `--json` only for machine-readable scheduling fields.\n  Missing execution semantics never imply safe parallel execution.";

const TASKFLOW_LONG_ABOUT: &str = "Delegate to the TaskFlow runtime family.\n\nTaskFlow is the execution/runtime authority. Use it for tracked execution, backlog pressure, run-graph state, packet inspection, continuation binding, and closure handoff.";

const TASKFLOW_AFTER_HELP: &str = "Family entrypoints:\n  vida taskflow help\n  vida taskflow help task\n  vida taskflow help parallelism\n  vida taskflow help dependencies\n  vida taskflow help queue\n  vida taskflow help dispatch\n  vida taskflow help scheduler\n  vida taskflow help scheduling\n  vida task tree <task-id> --json\n  vida task import --file tasks.yaml --parent-id <parent-id> --dry-run --json\n  vida taskflow graph explain <task-id> --json\n  vida taskflow graph-summary --json\n  vida taskflow closeout --json --compact\n  vida taskflow receipt-pack --since HEAD~1\n  vida taskflow plan generate --json\n  vida taskflow replan split <task-id> --child child-a:\"First slice\" --child child-b:\"Second slice\" --reason \"oversized task\" --json\n  vida taskflow replan spawn-blocker <task-id> <blocker-task-id> \"Blocker title\" --reason \"new dependency\" --json\n  vida taskflow scheduler dispatch --json\n  vida taskflow scheduling actualize --scope open-epics --dry-run --json\n  vida taskflow route explain --json\n  vida taskflow validate-routing --json\n  vida taskflow pricing status --json\n  vida taskflow pricing import --source-file <path> --dry-run --json\n  vida taskflow status --summary --json\n  vida taskflow run-graph status <run-id> --json\n  vida taskflow recovery status <run-id> --json\n  vida taskflow packet latest --json\n  vida taskflow packet repair --run-id <run-id> --from-task <task-id> --json\n  vida taskflow bootstrap-spec \"feature request\" --json\n  vida task next --json\n\nLarge-batch transport:\n  Put large task batches in JSONL/YAML files and run `vida task import --file <path> --dry-run` before applying.\n  Put large dependency batches in an edge file and run `vida task dep add-bulk --edge-file <path> --dry-run`.\n\nParallelism guidance:\n  `vida taskflow graph explain <task-id> --json` explains one task's ready/blocked/parallel-safe posture from canonical projection truth.\n  `vida taskflow graph-summary --json` exposes `current_task_id`, `scheduling.ready[*].ready_parallel_safe`, `parallel_blockers`, and `parallel_candidates_after_current`.\n  `vida taskflow scheduler dispatch --json` turns that projection into a preview-first launch plan capped by `max_parallel_agents`.\n  `vida taskflow scheduling actualize --dry-run --json` previews conservative scheduling metadata repairs before `--apply` mutates tasks.\n  `vida taskflow help parallelism` explains execution semantics fields and fail-closed scheduling rules.";

const DOCFLOW_LONG_ABOUT: &str = "Delegate to the DocFlow runtime family.\n\nDocFlow is the standalone documentation/readiness utility. Use it for documentation bootstrap, artifact init, validation, readiness checks, inventory, relations, and agent handoff instructions.";

const DOCFLOW_AFTER_HELP: &str = "Family entrypoints:\n  vida docflow help\n  vida docflow init\n  vida docflow init --json\n  vida docflow init --help\n  vida docflow repair-footer --help\n  vida docflow finalize-edit --help\n  vida docflow doctor --root .\n  vida docflow check-file --path <file> --json\n  vida docflow check --root . --json <file>\n  vida docflow readiness-check --profile active-canon\n  vida docflow registry --root .\n\nInit/repair contract:\n  `vida docflow init` without positional args prints agent bootstrap instructions.\n  `vida docflow init --json` prints the same contract as machine-readable JSON.\n  `vida docflow init <markdown_file> <artifact_path> <artifact_type> <change_note>` initializes a canonical markdown artifact.\n  `vida docflow repair-footer <markdown_file>` initializes missing footer metadata on legacy markdown files.";

const SERVICE_AFTER_HELP: &str = "Service operations:\n  vida service hello --json\n  vida service status --json\n  vida service capabilities --json\n  vida service endpoints --json\n  vida service endpoint-status --json\n  vida service lifecycle-plan --json\n  vida service lifecycle-status --json\n  vida service events --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const PROJECT_AFTER_HELP: &str = "Project operations:\n  vida project list --json\n  vida project resolve --project <project-id> --json\n  vida project status --project <project-id> --json\n\nOptions:\n  --project <project-id>    Project id or reference for project-scoped operations\n  --json                    Emit machine-readable JSON output";

const WIZARD_AFTER_HELP: &str = "Wizard operations:\n  vida wizard inspect --json\n  vida wizard draft --json\n  vida wizard validate --json\n  vida wizard diff --json\n\nOptions:\n  --json    Emit machine-readable JSON output";

const JOB_AFTER_HELP: &str = "Job operations:\n  vida job status\n  vida job status --json\n\nOutput:\n  Default output is compact TOON/plain with job status, authority, runner, and next action.\n  Use --json only when a machine-readable payload is required.";

const RECEIPT_AFTER_HELP: &str = "Receipt operations:\n  vida receipt get --json\n\nOptions:\n  --json    Emit machine-readable JSON output";
const PROOF_AFTER_HELP: &str = "Proof operations:\n  vida proof browser --route <route> --expect <text> --json\n\nBrowser proof options:\n  --route <route>    Browser route or URL to prove\n  --expect <text>    Text or route marker expected in the collected browser proof\n  --json             Emit machine-readable JSON output";
const SESSION_AFTER_HELP: &str = "Session operations:\n  vida session triage\n  vida session triage --task <task-id>\n  vida session triage --json\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.";
const QUALITY_AFTER_HELP: &str = "Quality operations:\n  vida quality gate --prepush\n  vida quality gate --prepush --advise\n  vida quality gate --prepush --json --advise\n\nOptions:\n  --prepush                        Evaluate the pre-push quality gate advisor\n  --advise                         Include remediation guidance\n  --coverage-file <path>           Read LCOV coverage evidence from this file\n  --coverage-threshold <percent>   Coverage threshold used for covered-line deficit math\n  --project-root <path>            Repository root used for git dirty/changed file evidence\n  --json                           Emit machine-readable JSON output\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.";
const PACK_AFTER_HELP: &str = "Pack operations:\n  vida pack list\n  vida pack list --json\n  vida pack show spec-four-pack\n  vida pack show spec-four-pack --json\n  vida pack validate\n  vida pack validate --json\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.";
const STATE_AFTER_HELP: &str = "State operations:\n  vida state reset --archive --reinit\n  vida state reset --archive --reinit --json\n  vida state reset --archive --reinit --state-dir <path> --json\n\nOptions:\n  --archive             Rename the current state root to a timestamped sibling archive before reset\n  --reinit              Recreate the authoritative state spine after archive\n  --state-dir <path>    Override the TaskFlow state directory\n  --json                Emit machine-readable JSON output\n\nOutput:\n  Default output is compact plain text for operators.\n  Use --json for machine-readable automation.";
const CODER_AFTER_HELP: &str = "Coder operations:\n  vida coder capabilities\n  vida coder provider-check --provider codex\n  vida coder run --request \"bounded implementation request\"\n\nOptions:\n  --provider <provider>   Provider id to inspect before execution\n  --request <request>     Bounded coder request text for future provider execution\n  --json                  Emit machine-readable JSON output\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.\n  `capabilities` is read-only and succeeds.\n  `provider-check` is a stub that reports provider execution is unavailable.\n  `run` fails closed before any provider execution until a provider adapter is implemented.";
const AGENT_INIT_AFTER_HELP: &str = "Agent init operations:\n  vida agent-init\n  vida agent-init --dispatch-packet <packet-path> --execute-dispatch\n  vida agent-init --auto-dispatch-packet --execute-dispatch\n\nOutput:\n  Default blocked output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload or full blocked evidence is required.";

const TASK_CREATE_ABOUT: &str = "Create one tracked task in the authoritative backlog store.";
const TASK_CREATE_LONG_ABOUT: &str = "Create one tracked task in the authoritative backlog store.\n\nExecution semantics are additive to graph truth:\n- `--execution-mode sequential` keeps the task single-lane by default\n- `--execution-mode parallel_safe` allows parallel admission only when other semantics also match\n- `--execution-mode exclusive` blocks parallel execution\n- `--execution-mode container_only` marks a work-pool/container task as non-executable by the scheduler\n- `--order-bucket`, `--parallel-group`, and `--conflict-domain` refine safe co-scheduling";
const TASK_CREATE_AFTER_HELP: &str = "Examples:\n  vida task create <task-id> <title> --parent-id <parent-id>\n  vida task create <subtask-id> <title> --type subtask --parent-id <task-id>\n  vida task create <step-id> <title> --type step --parent-id <task-or-subtask-id>\n  vida task create <task-id> --title <title> --parent-id <parent-id> --description \"...\" --notes \"...\"\n  vida task create <task-id> <title> --parent-id <parent-id> --owned-path crates/vida/src/lib.rs --acceptance-target \"Default output shows the needed field\" --proof-target \"cargo test -p vida focused_test\"\n  vida task create <task-id> <title> --acceptance-target-literal \"One prose target, with commas preserved\" --proof-target-literal \"Manual proof, with punctuation preserved\"\n  vida task create <task-id> <title> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs\n\nOne-shot metadata:\n  When owned paths, acceptance targets, proof targets, labels, notes, or execution semantics are known, pass them on `vida task create` instead of creating the task and immediately updating it.\n  Comma-delimited list flags remain available for compact lists; use the `*-literal` variants or `vida task import --file` JSON/YAML arrays for long prose values that contain commas.\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json only when a machine-readable payload is required.\n\nNotes:\n  Provide exactly one title source: positional <title> or --title <title>.\n  `step` is the canonical execution-step type; `todo` remains accepted as a deprecated alias without rewriting existing records.\n  Missing execution semantics fail closed for parallel scheduling.\n  Use `vida taskflow graph-summary` to verify parallel-safe admission after mutation; use `--json` only for machine-readable automation.\n  For many task creates or long per-task metadata, write a JSONL/YAML file and use `vida task import --file tasks.jsonl --dry-run` instead of an oversized shell command.";
const TASK_IMPORT_ABOUT: &str = "Create many tracked tasks from a structured file.";
const TASK_IMPORT_LONG_ABOUT: &str = "Create many tracked tasks from a structured file without oversized shell payloads.\n\nUse this surface when a task batch is too large for a reliable shell command, when per-task descriptions or notes are long, or when operators need a reviewable file before mutating TaskFlow state.\n\nSupported input:\n- JSON or YAML array of task objects\n- JSON or YAML object with a `tasks` array\n- JSONL/NDJSON with one task object per line\n\nEach task object requires `id` (or `task_id`) and `title`. Optional fields include `display_id`, `description`, `type`/`issue_type`, `status`, `priority`, `parent_id`, `notes`, `labels`, execution semantics, and planner metadata. Command flags provide defaults for parent assignment, execution semantics, labels, owned paths, acceptance targets, and proof targets.";
const TASK_IMPORT_AFTER_HELP: &str = "Examples:\n  vida task import --file tasks.jsonl --parent-id <parent-id> --dry-run --json\n  vida task import --file tasks.yaml --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n  vida task create-bulk --file tasks.json --labels operator-dx,taskflow --acceptance-target \"Tasks imported\" --proof-target \"cargo test -p vida task_bulk_import\" --json\n\nInput task object fields:\n  id | task_id, title, display_id, description, type | issue_type, status, priority, parent_id, notes\n  labels: [\"operator-dx\"] or \"operator-dx,taskflow\"\n  execution_semantics: { execution_mode, order_bucket, parallel_group, conflict_domain }\n  planner_metadata: { owned_paths, acceptance_targets, proof_targets, risk, estimate, lane_hint }\n\nLarge-batch transport:\n  Prefer JSONL/NDJSON for large batches because each task is one bounded line in a file.\n  If the shell reports a command line or payload is too large, move the task objects into a file and rerun `vida task import --file <path> --dry-run`.\n  Use JSON/YAML array entries for literal metadata values that contain commas; string fields and command defaults keep comma-delimited compatibility.\n  Use `vida task dep add-bulk --edge-file edges.txt --dry-run` for large dependency-edge batches.\n\nNotes:\n  `--dry-run` validates against the current graph and does not mutate TaskFlow state.\n  Per-task fields override command defaults; list defaults are appended and de-duplicated.\n  JSONL lets operators import large batches from a file instead of passing oversized command payloads.";

const TASK_UPDATE_ABOUT: &str = "Update one tracked task in the authoritative backlog store.";
const TASK_UPDATE_LONG_ABOUT: &str = "Update one tracked task in the authoritative backlog store.\n\nUse execution-semantics flags to correct sequencing and parallelism truth without moving ordering back into notes:\n- `--execution-mode sequential|parallel_safe|exclusive|container_only`\n- `--order-bucket <id>`\n- `--parallel-group <id>`\n- `--conflict-domain <id>`\n- matching `--clear-*` flags remove one semantics field\n\nPlanner proof target updates are replacements, not appends. Use `--clear-proof-targets` to remove obsolete proof targets.";
const TASK_UPDATE_AFTER_HELP: &str = "Examples:\n  vida task update <task-id> --status in_progress --json\n  vida task update <task-id> --title \"Retitled task\" --priority 1 --json\n  vida task update <task-id> --parent-id <parent-id> --json\n  vida task update <task-id> --clear-parent-id --json\n  vida task update <task-id> --proof-target \"cargo test -p vida focused_test\" --json\n  vida task update <task-id> --acceptance-target-literal \"One prose target, with commas preserved\" --json\n  vida task update <task-id> --clear-proof-targets --json\n  vida task update <task-id> --execution-mode parallel_safe --order-bucket wave-a --parallel-group docs --conflict-domain docs --json\n  vida task update <task-id> --clear-parallel-group --clear-conflict-domain --json\n\nNotes:\n  Use either a value flag or the matching clear flag, not both.\n  `--proof-target` replaces the configured planner proof_targets; it does not append to stale targets.\n  Comma-delimited list flags remain available for compact lists; use `*-literal` variants for long prose values that contain commas.\n  Re-check `vida taskflow graph-summary --json` after updates to confirm `ready_parallel_safe` and `parallel_blockers`.\n  For long notes, use `--notes-file <path>`; for many task updates or creates, use `vida task import --file tasks.jsonl --dry-run`.";
const TASK_BLOCK_ABOUT: &str = "record a runtime blocker on one task without closing it";
const TASK_BLOCK_LONG_ABOUT: &str = "Record a runtime blocker on one task without closing it.\n\nThe command marks the task status as `blocked`, appends a structured blocker note to existing task notes, refreshes the canonical TaskFlow snapshot, and emits a machine-readable receipt when `--json` is set.";
const TASK_BLOCK_AFTER_HELP: &str = "Examples:\n  vida task block <task-id> --reason \"runtime bridge unavailable\" --evidence \"agent-init returned host_tool_capability_missing\" --json\n  vida task block <task-id> --reason \"browser proof unavailable\" --blocker web_runtime_unhealthy --next-action \"run vida runtime web status --json\" --json\n\nOptions:\n  --reason <text>       Human-readable blocker reason; required\n  --evidence <text>     Evidence command, file, receipt, or observation; accepts repeated flags\n  --blocker <code>      Canonical blocker code; accepts comma-separated values and repeated flags\n  --next-action <text>  Suggested recovery or continuation action; accepts repeated flags\n  --state-dir <path>    Override the TaskFlow state directory\n  --json                Emit machine-readable JSON output";
const TASK_VERIFY_ABOUT: &str =
    "record partial verification evidence on one task without closing it";
const TASK_VERIFY_LONG_ABOUT: &str = "Record partial verification evidence on one task without closing it.\n\nUse this when source changes and tests are verified but browser, API, or external proof remains unavailable due to a runtime condition. The command leaves the task open, appends structured verification notes, updates proof-blocking labels, and emits source_fixed/tests_green/proof_blocked fields in JSON.";
const TASK_VERIFY_AFTER_HELP: &str = "Examples:\n  vida task verify <task-id> --source-fixed --tests-green --proof-blocked --proof-blocker \"browser proof unavailable\" --evidence \"cargo test -p vida task_verify\" --json\n\nOptions:\n  --source-fixed          Record that the source fix is complete\n  --tests-green           Record that focused tests passed\n  --proof-blocked         Record that final proof is pending on runtime/external conditions\n  --proof-blocker <text>  Human-readable proof blocker reason\n  --evidence <text>       Evidence command, file, receipt, or observation; accepts repeated flags\n  --state-dir <path>      Override the TaskFlow state directory\n  --json                  Emit machine-readable JSON output";
const TASK_VALIDATOR_PACKET_ABOUT: &str =
    "render a compact pre-commit validator packet for one task";
const TASK_VALIDATOR_PACKET_AFTER_HELP: &str = "Examples:\n  vida task validator-packet <task-id>\n  vida task validator-packet <task-id> --proof \"cargo check -p vida --tests\" --json\n\nOutput:\n  Default output is a copyable validator packet with active task context, owned files, diffstat, bounded hunks, proof commands, prior validator blockers, and the expected PASS/BLOCKED schema.\n  Use --json for machine-readable packet fields.";
const TASK_PRUNE_CLOSED_EPICS_ABOUT: &str =
    "archive and prune closed epic task rows without touching runtime receipts";
const TASK_PRUNE_CLOSED_EPICS_LONG_ABOUT: &str = "Archive and prune only TaskFlow task rows for closed epic/container subtrees.\n\nThe command previews by default. Use --apply to write a JSONL archive of pruned task rows and then delete only those task rows plus their owned task_dependency rows. Runtime receipts, run-graph state, lane state, and non-task runtime state are never removed by this surface.";
const TASK_PRUNE_CLOSED_EPICS_AFTER_HELP: &str = "Examples:\n  vida task prune-closed-epics\n  vida task prune-closed-epics --apply\n  vida task prune-closed-epics --apply --archive-dir .vida/data/state/task-archives --json\n\nOptions:\n  --apply                Archive and prune eligible closed epic/container task rows; default previews only\n  --archive-dir <path>   Directory for JSONL task-row archives when --apply is set\n  --state-dir <path>     Override the TaskFlow state directory\n  --json                 Emit machine-readable JSON output\n\nOutput:\n  Default output is compact plain text for operators. Use --json for machine-readable automation.";
const TASK_ATTEMPT_AFTER_HELP: &str = "Examples:\n  vida task attempt dispatch <task-id> --stage-id analysis\n  vida task attempt status <task-id> --stage-id analysis\n  vida task attempt collect <task-id> --stage-id analysis --attempt-id <attempt-id> --artifact-ref attempt-artifacts/<attempt-id>.json --status produced\n  vida task attempt record <task-id> --stage-id analysis --backend vibe --model-profile medium --isolation readonly --freshness snapshot-2026-06-05 --status submitted --artifact-ref attempt-artifacts/<attempt-id>.json\n  vida task attempt transition <attempt-id> --task-id <task-id> --stage-id analysis --status accepted --consolidation-receipt receipt-1\n  vida task attempt summary <task-id> --stage-id analysis\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json for machine-readable automation.";
const TASK_ATTEMPT_ARTIFACT_REF_HELP: &str =
    "State-root-relative or absolute attempt artifact JSON file; accepts repeated flags";
const TASK_ATTEMPT_ARTIFACT_REF_LONG_HELP: &str = "State-root-relative or absolute attempt artifact JSON file; accepts repeated flags.\n\nContract:\n  Root: .vida/data/state or --state-dir; prefer attempt-artifacts/<attempt-id>.json.\n  Size: max 64 KiB (65536 bytes).\n  JSON: schema_version=stage-attempt-v1, matching attempt_id, task_id, and stage_id.\n  Facts: include observed_facts or facts array.\n  Useful arrays: related_files, changed_files, proof_commands, hypotheses, proof_results, risks, limitations, conflicts.";

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub(crate) enum RenderMode {
    #[default]
    Plain,
    Color,
    #[value(name = "color_emoji")]
    ColorEmoji,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum TaskImportFormatArg {
    #[default]
    Auto,
    Json,
    Yaml,
    Jsonl,
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
        about = "render the bounded startup view or packet activation view for a worker/agent lane",
        after_help = AGENT_INIT_AFTER_HELP
    )]
    AgentInit(AgentInitArgs),
    #[command(about = "preview delegated agent lane selection without executing dispatch")]
    Agent(AgentArgs),
    #[command(
        about = "inspect and invoke the feature-gated VIDA coder provider surface",
        after_help = CODER_AFTER_HELP
    )]
    Coder(CoderArgs),
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
    #[command(
        about = "archive and reinitialize authoritative runtime state",
        after_help = STATE_AFTER_HELP
    )]
    State(StateArgs),
    #[command(about = "operate runtime-owned local development services")]
    Runtime(RuntimeArgs),
    #[command(about = "run bounded runtime integrity checks")]
    Doctor(DoctorArgs),
    #[command(about = "run canonical runtime diagnostics for completed slices")]
    Diagnostics(DiagnosticsArgs),
    #[command(
        about = "collect or diagnose runtime proof evidence",
        after_help = PROOF_AFTER_HELP
    )]
    Proof(ProofArgs),
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
    #[command(
        about = "summarize session triage evidence for the active bounded unit",
        after_help = SESSION_AFTER_HELP
    )]
    Session(SessionArgs),
    #[command(
        about = "inspect quality gates and remediation advice",
        after_help = QUALITY_AFTER_HELP
    )]
    Quality(QualityArgs),
    #[command(
        about = "inspect and validate active project role packs",
        after_help = PACK_AFTER_HELP
    )]
    Pack(PackArgs),
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
pub(crate) struct SessionArgs {
    #[command(subcommand)]
    pub(crate) command: SessionCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum SessionCommand {
    #[command(
        about = "bundle active unit, graph validity, task summary, and latest run parity evidence",
        after_help = "Examples:\n  vida session triage\n  vida session triage --task runtime-session-triage-proof-bundle-command\n  vida session triage --json\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json for machine-readable automation."
    )]
    Triage(SessionTriageArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct SessionTriageArgs {
    #[arg(
        long = "task",
        help = "Optional TaskFlow task id to include in the triage summary"
    )]
    pub(crate) task_id: Option<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for session triage evidence"
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

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct QualityArgs {
    #[command(subcommand)]
    pub(crate) command: QualityCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum QualityCommand {
    #[command(
        about = "evaluate pre-push quality gate evidence and remediation advice",
        after_help = QUALITY_AFTER_HELP
    )]
    Gate(QualityGateArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct QualityGateArgs {
    #[arg(long = "prepush", help = "Evaluate the pre-push quality gate advisor")]
    pub(crate) prepush: bool,

    #[arg(long = "advise", help = "Include remediation guidance in the output")]
    pub(crate) advise: bool,

    #[arg(
        long = "project-root",
        help = "Repository root used for git dirty/changed file evidence"
    )]
    pub(crate) project_root: Option<PathBuf>,

    #[arg(
        long = "coverage-file",
        help = "Read LCOV coverage evidence from this file"
    )]
    pub(crate) coverage_file: Option<PathBuf>,

    #[arg(
        long = "coverage-threshold",
        default_value_t = 90.0,
        help = "Coverage threshold used for covered-line deficit math"
    )]
    pub(crate) coverage_threshold: f64,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct PackArgs {
    #[command(subcommand)]
    pub(crate) command: PackCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum PackCommand {
    #[command(about = "list active project role packs")]
    List(PackListArgs),
    #[command(about = "show one active project role pack")]
    Show(PackShowArgs),
    #[command(about = "validate active project role packs")]
    Validate(PackValidateArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PackListArgs {
    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PackShowArgs {
    pub(crate) pack_id: String,
    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PackValidateArgs {
    pub(crate) pack_id: Option<String>,
    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct StateArgs {
    #[command(subcommand)]
    pub(crate) command: StateCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum StateCommand {
    #[command(
        about = "archive the current state root and optionally recreate an empty authoritative spine",
        after_help = STATE_AFTER_HELP
    )]
    Reset(StateResetArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct StateResetArgs {
    #[arg(
        long = "archive",
        help = "Rename the current state root to a timestamped sibling archive before reset"
    )]
    pub(crate) archive: bool,

    #[arg(
        long = "reinit",
        help = "Recreate the authoritative state spine after archive"
    )]
    pub(crate) reinit: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render mode for human output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact text"
    )]
    pub(crate) json: bool,
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
    #[command(
        name = "host-bridge",
        about = "render a pending host-tool bridge request as an executable parent-host adapter contract"
    )]
    HostBridge(AgentHostBridgeArgs),
    #[command(
        about = "summarize active agent and lane state as a compact closeout signal",
        after_help = "Examples:\n  vida agent status\n  vida agent status --compact\n  vida agent status --json --compact\n\nOutput:\n  Default output is compact TOON/plain for operators.\n  Use --json for machine-readable automation."
    )]
    Status(AgentStatusArgs),
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct CoderArgs {
    #[command(subcommand)]
    pub(crate) command: CoderCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum CoderCommand {
    #[command(
        about = "emit machine-readable VIDA coder capability metadata",
        after_help = CODER_AFTER_HELP
    )]
    Capabilities(CoderCapabilitiesArgs),
    #[command(
        about = "check provider readiness without executing provider code",
        after_help = CODER_AFTER_HELP
    )]
    ProviderCheck(CoderProviderCheckArgs),
    #[command(
        about = "fail closed before provider execution until coder providers are implemented",
        after_help = CODER_AFTER_HELP
    )]
    Run(CoderRunArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct CoderCapabilitiesArgs {
    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct CoderProviderCheckArgs {
    #[arg(
        long = "provider",
        default_value = "codex",
        help = "Provider id to inspect before execution"
    )]
    pub(crate) provider: String,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct CoderRunArgs {
    #[arg(
        long = "provider",
        default_value = "codex",
        help = "Provider id reserved for future execution"
    )]
    pub(crate) provider: String,

    #[arg(
        long = "request",
        help = "Bounded coder request text for future provider execution"
    )]
    pub(crate) request: Option<String>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AgentStatusArgs {
    #[arg(
        long = "compact",
        help = "Emit the compact closeout-oriented field set; currently the default view"
    )]
    pub(crate) compact: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for agent and lane status"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
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

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for readiness and continuation projections"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,

    #[arg(
        long = "full",
        help = "Include full dispatch diagnostics in JSON output; default JSON stays compact for operator latency"
    )]
    pub(crate) full: bool,

    #[arg(
        long = "dev-team",
        help = "Preview configured dev-team flow sequence from vida.config.yaml, for example specifier, coder, refactorer, and architect for the clean four-pack runtime-defect flow"
    )]
    pub(crate) dev_team: bool,

    #[arg(
        long = "materialize-packets",
        help = "Write receipt-backed dispatch packets for the selected lanes instead of returning only a preview"
    )]
    pub(crate) materialize_packets: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AgentSelectArgs {
    #[arg(
        long = "runtime-role",
        default_value = "worker",
        help = "Runtime role to select a carrier for, for example worker, coach, tester, or reviewer"
    )]
    pub(crate) runtime_role: String,

    #[arg(
        long = "task-class",
        default_value = "implementation",
        help = "Task class used for carrier/model eligibility, for example analysis, implementation, or verification"
    )]
    pub(crate) task_class: String,

    #[arg(
        long = "conversation-role",
        default_value = "orchestrator",
        help = "Host conversation role requesting the selection"
    )]
    pub(crate) conversation_role: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for carrier selection"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AgentHostBridgeArgs {
    #[arg(
        long = "request",
        help = "Path to a pending host_tool_bridge_request JSON artifact"
    )]
    pub(crate) request: PathBuf,

    #[arg(
        long = "attach-artifact",
        help = "Attach a receipt-backed patch_proposal or isolated_worktree_manifest artifact to the host bridge request before lane completion; accepts repeated flags"
    )]
    pub(crate) attach_artifacts: Vec<PathBuf>,

    #[arg(
        long = "artifact-kind",
        default_value = "patch_proposal",
        value_parser = ["patch_proposal", "isolated_worktree_manifest"],
        help = "Implementation artifact kind to attach when --attach-artifact is used"
    )]
    pub(crate) artifact_kind: String,

    #[arg(
        long = "changed-file",
        help = "Changed file covered by the attached artifact when the artifact file does not contain changed_files; accepts repeated flags"
    )]
    pub(crate) changed_files: Vec<String>,

    #[arg(
        long = "attempt-id",
        help = "Optional TaskFlow implementation attempt id to authorize the attached artifact"
    )]
    pub(crate) attempt_id: Option<String>,

    #[arg(
        long = "consolidation-receipt",
        help = "Optional TaskFlow consolidation receipt id backing the attached implementation artifact"
    )]
    pub(crate) consolidation_receipt_id: Option<String>,

    #[arg(
        long = "complete",
        hide = true,
        help = "After parent-host execution, complete the lane through the validated lane completion surface"
    )]
    pub(crate) complete: bool,

    #[arg(
        long = "host-agent-id",
        help = "Host agent id returned by the parent host adapter"
    )]
    pub(crate) host_agent_id: Option<String>,

    #[arg(
        long = "summary",
        alias = "host-bridge-summary",
        help = "Receipt summary from the parent host adapter; --host-bridge-summary is accepted as a lane-completion alias"
    )]
    pub(crate) summary: Option<String>,

    #[arg(
        long = "decision",
        hide = true,
        help = "Host bridge completion decision passed through to lane completion"
    )]
    pub(crate) decision: Option<String>,

    #[arg(
        long = "verdict",
        hide = true,
        help = "Host bridge completion verdict passed through to lane completion"
    )]
    pub(crate) verdict: Option<String>,

    #[arg(
        long = "allowed-next-node",
        hide = true,
        help = "Next workflow node allowed by the host bridge completion result"
    )]
    pub(crate) allowed_next_node: Option<String>,

    #[arg(
        long = "blocker-codes",
        hide = true,
        help = "Completion blocker codes as a JSON array or compact list"
    )]
    pub(crate) blocker_codes: Option<String>,

    #[arg(
        long = "blocker-code",
        hide = true,
        help = "Completion blocker code; accepts repeated flags"
    )]
    pub(crate) blocker_code: Vec<String>,

    #[arg(
        long = "rework-target",
        hide = true,
        help = "Workflow target that should receive rework when completion is blocked"
    )]
    pub(crate) rework_target: Option<String>,

    #[arg(
        long = "submit-result",
        help = "Submit one parent-host bridge result JSON file and apply the validated lane completion flow"
    )]
    pub(crate) submit_result: Option<PathBuf>,

    #[arg(
        long = "host-bridge-result-file",
        hide = true,
        help = "Path to the parent host bridge result file used for completion"
    )]
    pub(crate) result_file: Option<PathBuf>,

    #[arg(
        long = "receipt-id",
        help = "Optional completion receipt id; defaults from request run and dispatch target"
    )]
    pub(crate) receipt_id: Option<String>,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for host bridge provenance checks"
    )]
    pub(crate) state_dir: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct ProofArgs {
    #[command(subcommand)]
    pub(crate) command: ProofCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ProofCommand {
    #[command(
        about = "collect browser proof artifacts for one route",
        after_help = "Examples:\n  vida proof browser --route http://127.0.0.1:51235/#/module/project --expect \"My Tasks\" --json\n\nOptions:\n  --route <route>    Browser route or URL to prove\n  --expect <text>    Text or route marker expected in the collected browser proof\n  --json             Emit machine-readable JSON output"
    )]
    Browser(ProofBrowserArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ProofBrowserArgs {
    #[arg(long = "route", help = "Browser route or URL to prove")]
    pub(crate) route: String,

    #[arg(
        long = "expect",
        help = "Text or route marker expected in the collected browser proof"
    )]
    pub(crate) expect: Option<String>,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct RuntimeArgs {
    #[command(subcommand)]
    pub(crate) command: RuntimeCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RuntimeCommand {
    #[command(about = "inspect or restart local web proof services")]
    Web(RuntimeWebArgs),
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct RuntimeWebArgs {
    #[command(subcommand)]
    pub(crate) command: RuntimeWebCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RuntimeWebCommand {
    #[command(
        about = "inspect current-repo web proof listeners and proxy health",
        after_help = "Examples:\n  vida runtime web status --scope current-repo --include-edge-proxy --json\n\nOptions:\n  --scope current-repo       Limit diagnostics to the current repository\n  --include-edge-proxy       Include edge proxy listeners in diagnostics\n  --json                     Emit machine-readable JSON output"
    )]
    Status(RuntimeWebStatusArgs),

    #[command(
        about = "restart current-repo web proof listeners with fail-closed ownership checks",
        after_help = "Examples:\n  vida runtime web restart --scope current-repo --include-edge-proxy --dry-run --json\n  vida runtime web restart --scope current-repo --include-edge-proxy --json\n\nOptions:\n  --scope current-repo       Limit restart planning to the current repository\n  --include-edge-proxy       Include edge proxy listeners in the restart plan\n  --dry-run                  Preview actions without stopping or starting processes\n  --json                     Emit machine-readable JSON output"
    )]
    Restart(RuntimeWebRestartArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct RuntimeWebStatusArgs {
    #[arg(
        long = "scope",
        default_value = "current-repo",
        value_parser = ["current-repo"],
        help = "Status scope; only current-repo is supported"
    )]
    pub(crate) scope: String,

    #[arg(
        long = "include-edge-proxy",
        help = "Include configured edge proxy listeners in diagnostics"
    )]
    pub(crate) include_edge_proxy: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct RuntimeWebRestartArgs {
    #[arg(
        long = "scope",
        default_value = "current-repo",
        value_parser = ["current-repo"],
        help = "Restart scope; only current-repo is supported"
    )]
    pub(crate) scope: String,

    #[arg(
        long = "include-edge-proxy",
        help = "Include configured edge proxy listeners in the restart plan"
    )]
    pub(crate) include_edge_proxy: bool,

    #[arg(
        long = "dry-run",
        help = "Preview restart actions without stopping or starting processes"
    )]
    pub(crate) dry_run: bool,

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
        help = "Install target: current, cur, or path. Legacy all/local/cargo aliases resolve to current."
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
    #[command(
        about = TASK_IMPORT_ABOUT,
        long_about = TASK_IMPORT_LONG_ABOUT,
        after_help = TASK_IMPORT_AFTER_HELP,
        visible_aliases = ["create-bulk", "bulk-create"]
    )]
    Import(TaskBulkImportArgs),
    #[command(about = "import backlog tasks from a JSONL snapshot file")]
    ImportJsonl(TaskImportJsonlArgs),
    #[command(about = "authoritatively replace backlog state from a canonical snapshot artifact")]
    ReplaceJsonl(TaskReplaceJsonlArgs),
    #[command(about = "export backlog tasks to a JSONL snapshot file")]
    ExportJsonl(TaskExportJsonlArgs),
    #[command(about = "list tracked backlog tasks with optional status filtering")]
    List(TaskListArgs),
    #[command(about = "search tracked backlog tasks with compact filters")]
    Search(TaskSearchArgs),
    #[command(about = "show one tracked task with dependency and planner metadata")]
    Show(TaskShowArgs),
    #[command(
        name = "validator-packet",
        about = TASK_VALIDATOR_PACKET_ABOUT,
        after_help = TASK_VALIDATOR_PACKET_AFTER_HELP
    )]
    ValidatorPacket(TaskValidatorPacketArgs),
    #[command(about = "show progress and dependency context for one task or open epics")]
    Progress(TaskProgressArgs),
    #[command(about = "inspect whether one task or epic is ready to close")]
    ClosureReady(TaskClosureReadyArgs),
    #[command(
        about = "bundle task closeout proof, closure, graph, progress, and temp hygiene checks",
        long_about = "Bundle the small pre-close checks operators otherwise run one by one: proof status, closure readiness, graph validation, parent progress, and tracked temporary artifact hygiene. Default output is compact human-readable text; use --json for the machine-readable contract."
    )]
    Closeout(TaskCloseoutArgs),
    #[command(about = "inspect proof targets and evidence status for one tracked task")]
    Proof(TaskProofArgs),
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
    #[command(
        about = "reset a task/subtask subtree to its initial open state",
        long_about = "Reset the selected task or subtask subtree back to open state while preserving task records, dependencies, planner metadata, labels, and notes. Closed timestamps and close reasons are cleared. Execution steps are excluded by default; pass --include-steps only when you intentionally want to replay step state too.",
        after_help = "Examples:\n  vida task reset <task-id>\n  vida task reset <task-id> --dry-run\n  vida task reset <task-id> --include-steps --json"
    )]
    Reset(TaskResetArgs),
    #[command(about = "append-only task notes without replacing existing notes")]
    Note(TaskNoteArgs),
    #[command(
        about = TASK_BLOCK_ABOUT,
        long_about = TASK_BLOCK_LONG_ABOUT,
        after_help = TASK_BLOCK_AFTER_HELP
    )]
    Block(TaskBlockArgs),
    #[command(
        about = TASK_VERIFY_ABOUT,
        long_about = TASK_VERIFY_LONG_ABOUT,
        after_help = TASK_VERIFY_AFTER_HELP
    )]
    Verify(TaskVerifyArgs),
    #[command(
        about = "record and inspect per-stage task attempt ledger state",
        after_help = TASK_ATTEMPT_AFTER_HELP
    )]
    Attempt(TaskAttemptArgs),
    #[command(about = "inspect per-stage task execution status over the task attempt ledger")]
    Stage(TaskStageArgs),
    #[command(about = "inspect dirty git files against one task's owned paths")]
    OwnedStatus(TaskOwnedStatusArgs),
    #[command(about = "record delegated agent handoff receipts for a task")]
    Handoff(TaskHandoffArgs),
    #[command(about = "inspect task-scoped exception takeover and root-write status")]
    Takeover(TaskTakeoverArgs),
    #[command(about = "close one tracked task with evidence and optional release automation")]
    Close(TaskCloseArgs),
    #[command(about = "reconcile open epics whose descendants are complete")]
    Reconcile(TaskReconcileArgs),
    #[command(about = "retire historical run-graph rows for already-closed tasks")]
    ReconcileClosedRuns(TaskReconcileClosedRunsArgs),
    #[command(
        name = "prune-closed-epics",
        about = TASK_PRUNE_CLOSED_EPICS_ABOUT,
        long_about = TASK_PRUNE_CLOSED_EPICS_LONG_ABOUT,
        after_help = TASK_PRUNE_CLOSED_EPICS_AFTER_HELP
    )]
    PruneClosedEpics(TaskPruneClosedEpicsArgs),
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
    #[command(about = "ensure one dependency edge exists without duplicating it")]
    Ensure(TaskDependencyMutationCommandArgs),
    #[command(
        about = "add multiple dependency edges from flags or an edge file; use --edge-file for large batches"
    )]
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
        help = "Dependency edge in issue_id:depends_on_id:edge_type format; repeat for small batches"
    )]
    pub(crate) edges: Vec<String>,

    #[arg(
        long = "edge-file",
        help = "Read dependency edges from a newline-delimited issue_id:depends_on_id:edge_type file; preferred for large batches to avoid oversized shell payloads"
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
pub(crate) struct TaskBulkImportArgs {
    #[arg(
        long = "file",
        short = 'f',
        value_name = "PATH",
        help = "Structured task batch file to import (JSON, YAML, or JSONL)"
    )]
    pub(crate) file: PathBuf,

    #[arg(
        long = "format",
        value_enum,
        default_value_t = TaskImportFormatArg::Auto,
        help = "Input format: auto, json, yaml, or jsonl"
    )]
    pub(crate) format: TaskImportFormatArg,

    #[arg(
        long = "parent-id",
        help = "Default parent task id for imported tasks that omit parent_id"
    )]
    pub(crate) parent_id: Option<String>,

    #[arg(
        long = "type",
        default_value = "task",
        help = "Default task issue type for imported tasks that omit type or issue_type"
    )]
    pub(crate) issue_type: String,

    #[arg(
        long = "status",
        default_value = "open",
        help = "Default task status for imported tasks that omit status"
    )]
    pub(crate) status: String,

    #[arg(
        long = "priority",
        default_value_t = 2,
        help = "Default task priority for imported tasks that omit priority"
    )]
    pub(crate) priority: u32,

    #[arg(
        long = "labels",
        value_delimiter = ',',
        help = "Default labels appended to each imported task. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) labels: Vec<String>,

    #[arg(
        long = "execution-mode",
        help = "Default execution scheduling mode: sequential, parallel_safe, exclusive, or container_only"
    )]
    pub(crate) execution_mode: Option<String>,

    #[arg(
        long = "order-bucket",
        help = "Default ordering bucket for imported tasks"
    )]
    pub(crate) order_bucket: Option<String>,

    #[arg(
        long = "parallel-group",
        help = "Default parallel admission group for imported tasks"
    )]
    pub(crate) parallel_group: Option<String>,

    #[arg(
        long = "conflict-domain",
        help = "Default conflict domain for imported tasks"
    )]
    pub(crate) conflict_domain: Option<String>,

    #[arg(
        long = "owned-path",
        value_delimiter = ',',
        help = "Default planner metadata owned paths appended to each imported task"
    )]
    pub(crate) owned_paths: Vec<String>,

    #[arg(
        long = "acceptance-target",
        visible_alias = "acceptance",
        value_delimiter = ',',
        help = "Default planner metadata acceptance targets appended to each imported task"
    )]
    pub(crate) acceptance_targets: Vec<String>,

    #[arg(
        long = "proof-target",
        visible_alias = "proof",
        value_delimiter = ',',
        help = "Default planner metadata proof targets appended to each imported task"
    )]
    pub(crate) proof_targets: Vec<String>,

    #[arg(
        long = "dry-run",
        help = "Validate the import without mutating TaskFlow state"
    )]
    pub(crate) dry_run: bool,

    #[arg(
        long = "created-by",
        default_value = "vida task import",
        help = "Created-by value stored on imported tasks"
    )]
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

    #[arg(
        long = "query",
        help = "Filter tasks whose id, title, description, or notes contain this text"
    )]
    pub(crate) query: Option<String>,

    #[arg(long = "issue-type", help = "Filter tasks by issue type")]
    pub(crate) issue_type: Option<String>,

    #[arg(
        long = "parent-id",
        help = "Filter tasks that are direct children of this parent task"
    )]
    pub(crate) parent_id: Option<String>,

    #[arg(long = "limit", help = "Maximum number of task rows to return")]
    pub(crate) limit: Option<usize>,

    #[arg(
        long = "fields",
        help = "Comma-separated JSON task row fields to include, for example id,status,title"
    )]
    pub(crate) fields: Option<String>,

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for task rows: compact, summary, or full"
    )]
    pub(crate) view: String,

    #[arg(long = "all")]
    pub(crate) all: bool,

    #[arg(long = "summary")]
    pub(crate) summary: bool,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskSearchArgs {
    #[arg(help = "Text to search across task id, title, description, and notes")]
    pub(crate) query: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "status")]
    pub(crate) status: Option<String>,

    #[arg(long = "issue-type", help = "Filter tasks by issue type")]
    pub(crate) issue_type: Option<String>,

    #[arg(
        long = "parent-id",
        help = "Filter tasks that are direct children of this parent task"
    )]
    pub(crate) parent_id: Option<String>,

    #[arg(long = "all", help = "Include closed tasks in search results")]
    pub(crate) all: bool,

    #[arg(
        long = "limit",
        default_value_t = 50,
        help = "Maximum number of task rows to return"
    )]
    pub(crate) limit: usize,

    #[arg(
        long = "fields",
        help = "Comma-separated JSON task row fields to include, for example id,status,title"
    )]
    pub(crate) fields: Option<String>,

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for task rows: compact, summary, or full"
    )]
    pub(crate) view: String,

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

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for task detail: compact, summary, or full"
    )]
    pub(crate) view: String,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskValidatorPacketArgs {
    #[arg(help = "Task id for the validator packet")]
    pub(crate) task_id: String,

    #[arg(
        long = "proof",
        help = "Proof command to include in the packet. Repeat for multiple commands."
    )]
    pub(crate) proofs: Vec<String>,

    #[arg(
        long = "max-hunks",
        default_value_t = 8,
        help = "Maximum diff hunks to include in the packet"
    )]
    pub(crate) max_hunks: usize,

    #[arg(
        long = "max-lines",
        default_value_t = 180,
        help = "Maximum diff lines to include in the packet"
    )]
    pub(crate) max_lines: usize,

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
pub(crate) struct TaskNoteArgs {
    #[command(subcommand)]
    pub(crate) command: TaskNoteCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskNoteCommand {
    #[command(about = "append one message to a task's existing notes")]
    Append(TaskNoteAppendArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskNoteAppendArgs {
    #[arg(help = "Task id whose notes should receive the appended message")]
    pub(crate) task_id: String,

    #[arg(long = "message", help = "Message to append to the task notes")]
    pub(crate) message: Option<String>,

    #[arg(
        long = "message-file",
        help = "Read the appended note message from this file path"
    )]
    pub(crate) message_file: Option<PathBuf>,

    #[arg(
        long = "separator",
        default_value = "\n\n",
        help = "Text inserted between existing notes and the appended message"
    )]
    pub(crate) separator: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
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

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskProofArgs {
    #[command(subcommand)]
    pub(crate) command: TaskProofCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskProofCommand {
    #[command(
        about = "show configured proof targets and close-evidence coverage for one task",
        after_help = "Output:\n  Default output is compact TOON/plain and includes proof_targets[n]{target,status,evidence_source,artifact_status,next_action} rows.\n  Use --json only when the machine-readable proof_targets array is required.\n\nExamples:\n  vida task proof status task-1\n  vida task proof status task-1 --json"
    )]
    Status(TaskProofStatusArgs),
    #[command(
        about = "attach browser proof artifact evidence to one task",
        after_help = "Examples:\n  vida task proof attach-browser task-1 --route /odoo --result pass --screenshot artifacts/task-1.png --json\n\nOptions:\n  --route <route>       Browser route or URL that was checked\n  --result <result>     Proof result: pass, fail, or blocked\n  --screenshot <path>   Screenshot artifact path; optional but recommended\n  --expect <text>       Expected text or route marker\n  --evidence <text>     Additional evidence detail; accepts repeated flags\n  --state-dir <path>    Override the TaskFlow state directory\n  --json                Emit machine-readable JSON output"
    )]
    AttachBrowser(TaskProofAttachBrowserArgs),
    #[command(
        name = "attach-evidence",
        about = "attach structured proof evidence to one task",
        after_help = "Examples:\n  vida task proof attach-evidence task-1 --proof-target \"cargo test -p vida proof\" --result pass --evidence \"test log\"\n  vida task proof attach-evidence task-1 --proof-target \"proof a\" --proof-target \"proof b\" --result pass\n  vida task proof attach-evidence task-1 --proof-target \"cargo test -p vida proof\" --result pass --artifact-ref logs/a.txt --artifact-ref logs/b.txt --json\n\nOptions:\n  --proof-target <text> Proof target this evidence satisfies; repeat to attach the same evidence to multiple targets\n  --result <result>     Proof result: pass, fail, or blocked\n  --command <command>   Command or artifact command equivalent; defaults to each --proof-target\n  --artifact-ref <path> Receipt, log, screenshot, or artifact path; repeat to attach multiple artifacts\n  --evidence <text>     Additional evidence detail; accepts repeated flags\n  --state-dir <path>    Override the TaskFlow state directory\n  --json                Emit machine-readable JSON output"
    )]
    AttachEvidence(TaskProofAttachEvidenceArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskProofStatusArgs {
    #[arg(help = "Task id whose proof status should be inspected")]
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

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskProofAttachBrowserArgs {
    #[arg(help = "Task id whose browser proof evidence should be updated")]
    pub(crate) task_id: String,

    #[arg(long = "route", help = "Browser route or URL that was checked")]
    pub(crate) route: String,

    #[arg(long = "result", help = "Proof result: pass, fail, or blocked")]
    pub(crate) result: String,

    #[arg(long = "screenshot", help = "Screenshot artifact path")]
    pub(crate) screenshot: Option<String>,

    #[arg(long = "expect", help = "Expected text or route marker")]
    pub(crate) expect: Option<String>,

    #[arg(
        long = "evidence",
        help = "Additional evidence detail; accepts repeated flags"
    )]
    pub(crate) evidence: Vec<String>,

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
pub(crate) struct TaskProofAttachEvidenceArgs {
    #[arg(help = "Task id whose structured proof evidence should be updated")]
    pub(crate) task_id: String,

    #[arg(
        long = "proof-target",
        help = "Configured proof target this evidence satisfies; repeat for bulk attach"
    )]
    pub(crate) proof_target: Vec<String>,

    #[arg(long = "result", help = "Proof result: pass, fail, or blocked")]
    pub(crate) result: String,

    #[arg(
        long = "command",
        help = "Command or artifact command equivalent; defaults to --proof-target"
    )]
    pub(crate) command: Option<String>,

    #[arg(
        long = "artifact-ref",
        help = "Receipt, log, screenshot, or artifact path; repeat to attach multiple artifacts"
    )]
    pub(crate) artifact_ref: Vec<String>,

    #[arg(
        long = "evidence",
        help = "Additional evidence detail; accepts repeated flags"
    )]
    pub(crate) evidence: Vec<String>,

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
pub(crate) struct TaskTakeoverArgs {
    #[command(subcommand)]
    pub(crate) command: TaskTakeoverCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskTakeoverCommand {
    #[command(about = "show whether local exception takeover is active for one task")]
    Status(TaskTakeoverStatusArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskTakeoverStatusArgs {
    #[arg(help = "Optional task id whose takeover state should be inspected")]
    pub(crate) task_id: Option<String>,

    #[arg(
        long = "task-id",
        help = "Task id filter; equivalent to positional task id"
    )]
    pub(crate) task_id_filter: Option<String>,

    #[arg(
        long = "run-id",
        help = "Run id filter for the takeover lane to inspect"
    )]
    pub(crate) run_id: Option<String>,

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
        help = "Task issue type to store, such as task, subtask, step, defect, or epic; todo is a deprecated alias for step"
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
        help = "Execution scheduling mode: sequential, parallel_safe, exclusive, or container_only"
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
        long = "owned-path-literal",
        help = "Planner metadata owned path to set without comma splitting. Repeat for multiple literal values."
    )]
    pub(crate) owned_path_literals: Vec<String>,

    #[arg(
        long = "acceptance-target",
        visible_alias = "acceptance",
        value_delimiter = ',',
        help = "Planner metadata acceptance targets to set. Accepts comma-separated values, repeated flags, and alias --acceptance."
    )]
    pub(crate) acceptance_targets: Vec<String>,

    #[arg(
        long = "acceptance-target-literal",
        visible_alias = "acceptance-literal",
        help = "Planner metadata acceptance target to set without comma splitting. Repeat for long prose values."
    )]
    pub(crate) acceptance_target_literals: Vec<String>,

    #[arg(
        long = "proof-target",
        visible_alias = "proof",
        value_delimiter = ',',
        help = "Planner metadata proof targets to set. Accepts comma-separated values, repeated flags, and alias --proof."
    )]
    pub(crate) proof_targets: Vec<String>,

    #[arg(
        long = "proof-target-literal",
        visible_alias = "proof-literal",
        help = "Planner metadata proof target to set without comma splitting. Repeat for long prose values."
    )]
    pub(crate) proof_target_literals: Vec<String>,

    #[arg(
        long = "release-proof-template",
        help = "Append standard release/install proof targets for runtime-defect closeout"
    )]
    pub(crate) release_proof_template: bool,

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
        long = "owned-path-literal",
        help = "Planner metadata owned path to set without comma splitting. Repeat for multiple literal values."
    )]
    pub(crate) owned_path_literals: Vec<String>,

    #[arg(
        long = "acceptance-target",
        value_delimiter = ',',
        help = "Planner metadata acceptance targets to set. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) acceptance_targets: Vec<String>,

    #[arg(
        long = "acceptance-target-literal",
        help = "Planner metadata acceptance target to set without comma splitting. Repeat for long prose values."
    )]
    pub(crate) acceptance_target_literals: Vec<String>,

    #[arg(
        long = "proof-target",
        value_delimiter = ',',
        help = "Planner metadata proof targets to replace. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) proof_targets: Vec<String>,

    #[arg(
        long = "proof-target-literal",
        help = "Planner metadata proof target to replace without comma splitting. Repeat for long prose values."
    )]
    pub(crate) proof_target_literals: Vec<String>,

    #[arg(
        long = "release-proof-template",
        help = "Append standard release/install proof targets while preserving existing proof targets"
    )]
    pub(crate) release_proof_template: bool,

    #[arg(
        long = "clear-proof-targets",
        help = "Remove all planner metadata proof targets; cannot be combined with --proof-target."
    )]
    pub(crate) clear_proof_targets: bool,

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

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskResetArgs {
    #[arg(help = "Task or subtask id whose subtree should reset to open")]
    pub(crate) task_id: String,

    #[arg(
        long = "include-steps",
        help = "Also reset execution-step rows under the selected subtree"
    )]
    pub(crate) include_steps: bool,

    #[arg(
        long = "dry-run",
        help = "Preview affected rows without mutating state"
    )]
    pub(crate) dry_run: bool,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory"
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
pub(crate) struct TaskBlockArgs {
    #[arg(help = "Task id to mark blocked in the authoritative backlog store")]
    pub(crate) task_id: String,

    #[arg(long = "reason", help = "Human-readable blocker reason")]
    pub(crate) reason: String,

    #[arg(
        long = "evidence",
        help = "Evidence command, file path, receipt path, or observation for the blocker. Accepts repeated flags."
    )]
    pub(crate) evidence: Vec<String>,

    #[arg(
        long = "blocker",
        value_delimiter = ',',
        help = "Canonical blocker code. Accepts comma-separated values and repeated flags."
    )]
    pub(crate) blockers: Vec<String>,

    #[arg(
        long = "next-action",
        help = "Suggested recovery or continuation action. Accepts repeated flags."
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

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskVerifyArgs {
    #[arg(help = "Task id to record partial verification for")]
    pub(crate) task_id: String,

    #[arg(long = "source-fixed", help = "Record that the source fix is complete")]
    pub(crate) source_fixed: bool,

    #[arg(long = "tests-green", help = "Record that focused tests passed")]
    pub(crate) tests_green: bool,

    #[arg(
        long = "proof-blocked",
        help = "Record that final proof is pending on a runtime or external condition"
    )]
    pub(crate) proof_blocked: bool,

    #[arg(
        long = "proof-blocker",
        help = "Human-readable proof blocker reason when --proof-blocked is set"
    )]
    pub(crate) proof_blocker: Option<String>,

    #[arg(
        long = "evidence",
        help = "Evidence command, file path, receipt path, or observation. Accepts repeated flags."
    )]
    pub(crate) evidence: Vec<String>,

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
#[command(disable_help_subcommand = true)]
pub(crate) struct TaskAttemptArgs {
    #[command(subcommand)]
    pub(crate) command: TaskAttemptCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskAttemptCommand {
    #[command(about = "create stage attempt ledger rows from configured stage policy")]
    Dispatch(TaskAttemptDispatchArgs),
    #[command(about = "report stage attempt ledger status for one task and stage")]
    Status(TaskAttemptStatusArgs),
    #[command(about = "collect attempt artifacts into the ledger without mutating task notes")]
    Collect(TaskAttemptCollectArgs),
    #[command(about = "consolidate validated stage attempts into one canonical stage receipt")]
    Consolidate(TaskAttemptConsolidateArgs),
    #[command(about = "record one stage attempt for a task")]
    Record(TaskAttemptRecordArgs),
    #[command(about = "transition an existing stage attempt after validating task binding")]
    Transition(TaskAttemptTransitionArgs),
    #[command(about = "summarize stage attempt counts and latest consolidation evidence")]
    Summary(TaskAttemptSummaryArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptDispatchArgs {
    #[arg(help = "Task id that owns the stage attempts")]
    pub(crate) task_id: String,

    #[arg(
        long = "stage-id",
        visible_alias = "stage",
        value_name = "STAGE",
        help = "Configured stage id to dispatch"
    )]
    pub(crate) stage_id: String,

    #[arg(
        long = "backend",
        help = "Override backend or agent carrier for a single attempt"
    )]
    pub(crate) backend: Option<String>,

    #[arg(
        long = "model-profile",
        help = "Override model profile for a single attempt"
    )]
    pub(crate) model_profile: Option<String>,

    #[arg(
        long = "isolation",
        help = "Override isolation mode for a single attempt such as readonly or patch_proposal"
    )]
    pub(crate) isolation: Option<String>,

    #[arg(long = "attempt-id", help = "Optional caller-supplied attempt id")]
    pub(crate) attempt_id: Option<String>,

    #[arg(
        long = "policy",
        default_value = "configured",
        help = "Attempt dispatch policy source; configured uses agent_system.stage_attempt_policies"
    )]
    pub(crate) policy: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptStatusArgs {
    #[arg(help = "Task id that owns the stage attempts")]
    pub(crate) task_id: String,

    #[arg(
        long = "stage-id",
        visible_alias = "stage",
        value_name = "STAGE",
        help = "Stage id to summarize"
    )]
    pub(crate) stage_id: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptCollectArgs {
    #[arg(help = "Task id that owns the attempt")]
    pub(crate) task_id: String,

    #[arg(
        long = "stage-id",
        visible_alias = "stage",
        value_name = "STAGE",
        help = "Expected stage binding for the attempt"
    )]
    pub(crate) stage_id: String,

    #[arg(long = "attempt-id", help = "Attempt id to collect artifacts for")]
    pub(crate) attempt_id: Option<String>,

    #[arg(
        long = "artifact-ref",
        help = TASK_ATTEMPT_ARTIFACT_REF_HELP,
        long_help = TASK_ATTEMPT_ARTIFACT_REF_LONG_HELP
    )]
    pub(crate) artifact_refs: Vec<String>,

    #[arg(
        long = "status",
        default_value = "produced",
        help = "Collected attempt status: produced, validating, accepted, partially_accepted, rejected, stale, or failed"
    )]
    pub(crate) status: String,

    #[arg(
        long = "consolidation-receipt",
        help = "Optional consolidation receipt id produced from this attempt"
    )]
    pub(crate) consolidation_receipt_id: Option<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptConsolidateArgs {
    #[arg(help = "Task id that owns the stage attempts")]
    pub(crate) task_id: String,

    #[arg(
        long = "stage-id",
        visible_alias = "stage",
        value_name = "STAGE",
        help = "Stage id to consolidate"
    )]
    pub(crate) stage_id: String,

    #[arg(
        long = "consolidation-receipt",
        help = "Optional caller-supplied canonical stage consolidation receipt id"
    )]
    pub(crate) consolidation_receipt_id: Option<String>,

    #[arg(
        long = "consolidator-profile",
        default_value = "primary_orchestrator",
        help = "Consolidator model/profile or primary orchestrator policy used for the receipt"
    )]
    pub(crate) consolidator_profile: String,

    #[arg(
        long = "merge-policy",
        default_value = "facts_require_artifact_evidence_conflicts_fail_closed",
        help = "Merge policy used to separate facts, hypotheses, conflicts, and partial results"
    )]
    pub(crate) merge_policy: String,

    #[arg(
        long = "fact",
        help = "Operator-supplied canonical fact to merge with validated artifact facts; accepts repeated flags"
    )]
    pub(crate) facts: Vec<String>,

    #[arg(
        long = "hypothesis",
        help = "Operator-supplied hypothesis to keep separate from canonical facts; accepts repeated flags"
    )]
    pub(crate) hypotheses: Vec<String>,

    #[arg(
        long = "conflict",
        help = "Operator-supplied unresolved conflict to surface in the consolidation receipt; accepts repeated flags"
    )]
    pub(crate) conflicts: Vec<String>,

    #[arg(
        long = "partial-attempt-id",
        help = "Attempt id classified as partial coverage; accepts repeated flags"
    )]
    pub(crate) partial_attempt_ids: Vec<String>,

    #[arg(
        long = "timeout-attempt-id",
        help = "Attempt id classified as timed out; accepts repeated flags"
    )]
    pub(crate) timeout_attempt_ids: Vec<String>,

    #[arg(
        long = "cap-limited-attempt-id",
        help = "Attempt id classified as cap-limited; accepts repeated flags"
    )]
    pub(crate) cap_limited_attempt_ids: Vec<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptRecordArgs {
    #[arg(help = "Task id that owns the attempt")]
    pub(crate) task_id: String,

    #[arg(long = "attempt-id", help = "Optional caller-supplied attempt id")]
    pub(crate) attempt_id: Option<String>,

    #[arg(
        long = "stage-id",
        help = "Stage id such as analysis, design, implementation, coach, tester"
    )]
    pub(crate) stage_id: String,

    #[arg(
        long = "backend",
        help = "Backend or agent carrier used for this attempt"
    )]
    pub(crate) backend: String,

    #[arg(
        long = "model-profile",
        help = "Model or model profile used for this attempt"
    )]
    pub(crate) model_profile: String,

    #[arg(
        long = "isolation",
        help = "Isolation mode such as readonly, patch_proposal, or worktree"
    )]
    pub(crate) isolation: String,

    #[arg(
        long = "freshness",
        help = "Optional freshness boundary or snapshot id; defaults to the task updated_at value"
    )]
    pub(crate) freshness: Option<String>,

    #[arg(
        long = "status",
        default_value = "running",
        help = "Attempt status: submitted, running, produced, validating, accepted, partially_accepted, rejected, stale, failed, or consumed"
    )]
    pub(crate) status: String,

    #[arg(
        long = "artifact-ref",
        help = TASK_ATTEMPT_ARTIFACT_REF_HELP,
        long_help = TASK_ATTEMPT_ARTIFACT_REF_LONG_HELP
    )]
    pub(crate) artifact_refs: Vec<String>,

    #[arg(
        long = "consolidation-receipt",
        help = "Optional consolidation receipt id produced from this attempt"
    )]
    pub(crate) consolidation_receipt_id: Option<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
#[command(disable_help_subcommand = true)]
pub(crate) struct TaskStageArgs {
    #[command(subcommand)]
    pub(crate) command: TaskStageCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum TaskStageCommand {
    #[command(about = "report stage status for one task from the task attempt ledger")]
    Status(TaskStageStatusArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskStageStatusArgs {
    #[arg(help = "Task id that owns the stage")]
    pub(crate) task_id: String,

    #[arg(
        long = "stage-id",
        visible_alias = "stage",
        value_name = "STAGE",
        help = "Stage id to summarize; omitted reports all stages for the task"
    )]
    pub(crate) stage_id: Option<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptTransitionArgs {
    #[arg(help = "Attempt id to transition")]
    pub(crate) attempt_id: String,

    #[arg(long = "task-id", help = "Expected task binding for the attempt")]
    pub(crate) task_id: String,

    #[arg(long = "stage-id", help = "Expected stage binding for the attempt")]
    pub(crate) stage_id: String,

    #[arg(
        long = "status",
        help = "New attempt status: submitted, running, produced, validating, accepted, partially_accepted, rejected, stale, failed, or consumed"
    )]
    pub(crate) status: String,

    #[arg(
        long = "artifact-ref",
        help = TASK_ATTEMPT_ARTIFACT_REF_HELP,
        long_help = TASK_ATTEMPT_ARTIFACT_REF_LONG_HELP
    )]
    pub(crate) artifact_refs: Vec<String>,

    #[arg(
        long = "consolidation-receipt",
        help = "Optional consolidation receipt id produced from this attempt"
    )]
    pub(crate) consolidation_receipt_id: Option<String>,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskAttemptSummaryArgs {
    #[arg(help = "Task id that owns the stage attempts")]
    pub(crate) task_id: String,

    #[arg(long = "stage-id", help = "Stage id to summarize")]
    pub(crate) stage_id: String,

    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory for this command"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render output mode for human-readable command output")]
    pub(crate) render: RenderMode,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TaskCloseArgs {
    #[arg(help = "Task id to close")]
    pub(crate) task_id: String,

    #[arg(long = "reason", help = "Closure reason and evidence summary")]
    pub(crate) reason: Option<String>,

    #[arg(
        long = "reason-file",
        value_name = "PATH",
        help = "Read the closure reason exactly from a UTF-8 file"
    )]
    pub(crate) reason_file: Option<PathBuf>,

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
        long = "include-global-progress",
        help = "Include all epic progress rows in close output; default close output is scoped and compact"
    )]
    pub(crate) include_global_progress: bool,

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

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskPruneClosedEpicsArgs {
    #[arg(
        long = "apply",
        help = "Archive and prune eligible closed epic/container task rows; default previews only"
    )]
    pub(crate) apply: bool,

    #[arg(
        long = "archive-dir",
        help = "Directory for JSONL task-row archives when --apply is set"
    )]
    pub(crate) archive_dir: Option<PathBuf>,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskReconcileArgs {
    #[arg(long = "epics", help = "Inspect open epic/container tasks")]
    pub(crate) epics: bool,

    #[arg(
        long = "close-if-complete",
        help = "Close eligible epics whose descendants are all closed-like"
    )]
    pub(crate) close_if_complete: bool,

    #[arg(
        long = "dry-run",
        help = "Report eligible epics without mutating TaskFlow state"
    )]
    pub(crate) dry_run: bool,

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

    #[arg(
        long = "fields",
        help = "Comma-separated JSON task row fields to include, for example id,status,title"
    )]
    pub(crate) fields: Option<String>,

    #[arg(long = "limit", help = "Maximum ready task rows to print")]
    pub(crate) limit: Option<usize>,

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for task rows: compact, summary, or full"
    )]
    pub(crate) view: String,

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
    #[arg(
        long = "scope",
        help = "Limit lawful continuation candidates to this task scope"
    )]
    pub(crate) scope: Option<String>,

    #[arg(
        long = "strategy",
        value_parser = ["default", "epic-sequential"],
        help = "Continuation selection strategy: default or epic-sequential"
    )]
    pub(crate) strategy: Option<String>,

    #[arg(
        long = "select",
        help = "Select a ready task id and return its canonical bind command"
    )]
    pub(crate) select: Option<String>,

    #[arg(
        long = "explain",
        help = "Include operator rationale for the returned next-lawful decision"
    )]
    pub(crate) explain: bool,

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
        help = "Render mode for human-readable output"
    )]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskProgressArgs {
    #[arg(
        help = "Task id whose progress should be inspected; omit when using --epics, or omit all selectors to see actionable progress commands"
    )]
    pub(crate) task_id: Option<String>,

    #[arg(
        long = "epics",
        help = "List open or in-progress epics with descendant progress counts"
    )]
    pub(crate) epics: bool,

    #[arg(long = "epic", help = "Limit --epics output to one epic id")]
    pub(crate) epic: Option<String>,

    #[arg(
        long = "basis",
        default_value = "descendants",
        help = "Progress basis: descendants or direct-children"
    )]
    pub(crate) basis: String,

    #[arg(long = "all", help = "Include closed epics when used with --epics")]
    pub(crate) all: bool,

    #[arg(
        long = "counts-only",
        help = "Emit compact counts-only progress totals without task or epic rows"
    )]
    pub(crate) counts_only: bool,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskClosureReadyArgs {
    #[arg(help = "Task or epic id whose closure readiness should be inspected")]
    pub(crate) task_id: String,

    #[arg(
        long = "basis",
        default_value = "descendants",
        help = "Closure readiness basis: descendants or direct-children"
    )]
    pub(crate) basis: String,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct TaskCloseoutArgs {
    #[arg(help = "Task or epic id whose closeout proof bundle should be inspected")]
    pub(crate) task_id: String,

    #[arg(
        long = "basis",
        default_value = "descendants",
        help = "Progress and closure basis: descendants or direct-children"
    )]
    pub(crate) basis: String,

    #[arg(
        long = "include-temp-scan",
        default_value_t = false,
        help = "Opt in to a tracked temporary artifact hygiene scan in the closeout bundle"
    )]
    pub(crate) include_temp_scan: bool,

    #[arg(long = "state-dir", env = "VIDA_STATE_DIR")]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain)]
    pub(crate) render: RenderMode,

    #[arg(long = "json", help = "Emit machine-readable JSON output")]
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

    #[arg(
        long = "full",
        help = "Include recursive tree nodes and descendant progress totals"
    )]
    pub(crate) full: bool,

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for init fields: compact, summary, or full"
    )]
    pub(crate) view: String,

    #[arg(
        long = "fields",
        help = "Comma-separated top-level init fields to include, for example status,active_bounded_unit,next_actions"
    )]
    pub(crate) fields: Option<String>,

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

    #[arg(
        long = "view",
        default_value = "summary",
        help = "Output view for init fields: compact, summary, or full"
    )]
    pub(crate) view: String,

    #[arg(
        long = "fields",
        help = "Comma-separated top-level init fields to include, for example status,active_bounded_unit,next_actions"
    )]
    pub(crate) fields: Option<String>,

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
        help = "Resume the packet handoff path and return receipt-backed execution or host-bridge handoff state instead of rendering only an activation view"
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
    #[arg(num_args = 0..)]
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
    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for status projections"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render mode for human output; plain is compact TOON by default")]
    pub(crate) render: RenderMode,

    #[arg(long = "summary", help = "Emit the compact status summary shape")]
    pub(crate) summary: bool,

    #[arg(
        long = "view",
        default_value = "compact",
        help = "Output view for status fields: compact, summary, or full"
    )]
    pub(crate) view: String,

    #[arg(
        long = "fields",
        help = "Comma-separated top-level status fields to include, for example status,blocker_codes,next_actions,taskflow_counts"
    )]
    pub(crate) fields: Option<String>,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DoctorArgs {
    #[arg(
        long = "state-dir",
        env = "VIDA_STATE_DIR",
        help = "Override the TaskFlow state directory used for doctor checks"
    )]
    pub(crate) state_dir: Option<PathBuf>,

    #[arg(long = "render", env = "VIDA_RENDER", value_enum, default_value_t = RenderMode::Plain, help = "Render mode for human output; plain is compact TOON by default")]
    pub(crate) render: RenderMode,

    #[arg(long = "summary", help = "Emit the compact doctor summary shape")]
    pub(crate) summary: bool,

    #[arg(
        long = "json",
        help = "Emit machine-readable JSON output instead of default compact TOON"
    )]
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
    use super::{Cli, CoderCommand, TaskCommand};
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
    fn task_progress_help_lists_epic_summary_options() {
        let error = Cli::try_parse_from(["vida", "task", "progress", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("[TASK_ID]"));
        assert!(help.contains("--epics"));
        assert!(help.contains("--all"));
        assert!(help.contains("omit when using --epics"));
        assert!(help.contains("open or in-progress epics"));
    }

    #[test]
    fn task_note_append_help_lists_message_options() {
        let error = Cli::try_parse_from(["vida", "task", "note", "append", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("<TASK_ID>"));
        assert!(help.contains("--message"));
        assert!(help.contains("--message-file"));
        assert!(help.contains("--separator"));
        assert!(help.contains("append"));
    }

    #[test]
    fn task_dep_ensure_help_lists_idempotent_dependency_options() {
        let dep_error = Cli::try_parse_from(["vida", "task", "dep", "--help"])
            .expect_err("help should render clap display error");
        assert!(dep_error.to_string().contains("ensure"));

        let error = Cli::try_parse_from(["vida", "task", "dep", "ensure", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("<TASK_ID>"));
        assert!(help.contains("<DEPENDS_ON_ID>"));
        assert!(help.contains("<EDGE_TYPE>"));
        assert!(help.contains("--created-by"));
        assert!(help.contains("dependency edge"));
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
        assert!(help.contains("--type subtask"));
        assert!(help.contains("--type step"));
        assert!(help.contains("todo is a deprecated alias"));
        assert!(help.contains("--owned-path crates/vida/src/lib.rs"));
        assert!(help.contains("--acceptance-target"));
        assert!(help.contains("--proof-target"));
        assert!(help.contains("One-shot metadata"));
        assert!(help.contains("instead of creating the task and immediately updating it"));
        assert!(help.contains("Default output is compact TOON/plain"));
        assert!(help.contains("vida task import --file tasks.jsonl --dry-run"));
        assert!(help.contains("oversized shell command"));
    }

    #[test]
    fn task_update_help_lists_proof_target_replacement_contract() {
        let error = Cli::try_parse_from(["vida", "task", "update", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("--proof-target"));
        assert!(help.contains("--clear-proof-targets"));
        assert!(help.contains("Planner proof target updates are replacements"));
        assert!(help.contains("--proof-target` replaces the configured planner proof_targets"));
    }

    #[test]
    fn task_import_help_lists_bulk_file_dry_run_and_metadata_options() {
        let error = Cli::try_parse_from(["vida", "task", "import", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("--file <PATH>"));
        assert!(help.contains("--format <FORMAT>"));
        assert!(help.contains("--dry-run"));
        assert!(help.contains("--parent-id"));
        assert!(help.contains("--execution-mode"));
        assert!(help.contains("--acceptance-target"));
        assert!(help.contains("--proof-target"));
        assert!(help.contains("Large-batch transport"));
        assert!(help.contains("command line or payload is too large"));
        assert!(help.contains("vida task dep add-bulk --edge-file edges.txt --dry-run"));
        assert!(help.contains("JSONL lets operators import large batches"));
    }

    #[test]
    fn task_help_surfaces_large_batch_file_transport_guidance() {
        let task_error = Cli::try_parse_from(["vida", "task", "--help"])
            .expect_err("help should render clap display error");
        let task_help = task_error.to_string();
        assert!(task_help.contains("Large-batch transport"));
        assert!(task_help.contains("vida task import --file tasks.jsonl --dry-run"));
        assert!(task_help.contains("vida task dep add-bulk --edge-file edges.txt --dry-run"));

        let taskflow_error = Cli::try_parse_from(["vida", "taskflow", "--help"])
            .expect_err("help should render clap display error");
        let taskflow_help = taskflow_error.to_string();
        assert!(taskflow_help.contains("Large-batch transport"));
        assert!(taskflow_help.contains("vida task import --file <path> --dry-run"));
        assert!(taskflow_help.contains("vida task dep add-bulk --edge-file <path> --dry-run"));

        let update_error = Cli::try_parse_from(["vida", "task", "update", "--help"])
            .expect_err("help should render clap display error");
        let update_help = update_error.to_string();
        assert!(update_help.contains("--notes-file <NOTES_FILE>"));
        assert!(update_help.contains("vida task import --file tasks.jsonl --dry-run"));

        let dep_error = Cli::try_parse_from(["vida", "task", "dep", "add-bulk", "--help"])
            .expect_err("help should render clap display error");
        let dep_help = dep_error.to_string();
        assert!(dep_help.contains("--edge-file <EDGE_FILE>"));
        assert!(dep_help.contains("large batches"));
        assert!(dep_help.contains("oversized shell payloads"));
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
    fn task_validator_packet_help_and_args_are_discoverable() {
        let packet_error = Cli::try_parse_from(["vida", "task", "validator-packet", "--help"])
            .expect_err("help should render clap display error");
        let packet_help = packet_error.to_string();
        assert!(packet_help.contains("<TASK_ID>"));
        assert!(packet_help.contains("--proof"));
        assert!(packet_help.contains("--max-hunks"));
        assert!(packet_help.contains("--max-lines"));
        assert!(packet_help.contains("--state-dir"));
        assert!(packet_help.contains("--json"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "validator-packet",
            "task-a",
            "--proof",
            "cargo check -p vida --tests",
            "--max-hunks",
            "3",
            "--max-lines",
            "80",
            "--json",
        ])
        .expect("validator-packet should parse");
        let Some(super::Command::Task(task_args)) = parsed.command else {
            panic!("task command should parse");
        };
        let TaskCommand::ValidatorPacket(command) = task_args.command else {
            panic!("validator-packet command should parse");
        };
        assert_eq!(command.task_id, "task-a");
        assert_eq!(
            command.proofs,
            vec!["cargo check -p vida --tests".to_string()]
        );
        assert_eq!(command.max_hunks, 3);
        assert_eq!(command.max_lines, 80);
        assert!(command.json);
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
        assert!(agent_help.contains("host-bridge"));

        let dispatch_error = Cli::try_parse_from(["vida", "agent", "dispatch-next", "--help"])
            .expect_err("help should render clap display error");
        let dispatch_help = dispatch_error.to_string();
        assert!(dispatch_help.contains("--lanes"));
        assert!(dispatch_help.contains("--scope"));
        assert!(dispatch_help.contains("--current-task-id"));
        assert!(dispatch_help.contains("--state-dir"));
        assert!(dispatch_help.contains("--json"));
        assert!(dispatch_help.contains("--full"));
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
            "--full",
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
        assert!(dispatch.full);

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

        let host_bridge_error = Cli::try_parse_from(["vida", "agent", "host-bridge", "--help"])
            .expect_err("help should render clap display error");
        let host_bridge_help = host_bridge_error.to_string();
        assert!(host_bridge_help.contains("--request"));
        assert!(host_bridge_help.contains("--attach-artifact"));
        assert!(host_bridge_help.contains("--artifact-kind"));
        assert!(host_bridge_help.contains("--changed-file"));
        assert!(host_bridge_help.contains("--attempt-id"));
        assert!(host_bridge_help.contains("--consolidation-receipt"));
        assert!(host_bridge_help.contains("--host-agent-id"));
        assert!(host_bridge_help.contains("--submit-result"));
        assert!(host_bridge_help.contains("--summary"));
        assert!(host_bridge_help.contains("--receipt-id"));
        assert!(host_bridge_help.contains("--state-dir"));
        assert!(host_bridge_help.contains("--json"));
        for flag in [
            "--complete",
            "--decision",
            "--verdict",
            "--allowed-next-node",
            "--blocker-codes",
            "--blocker-code",
            "--rework-target",
            "--host-bridge-result-file",
        ] {
            assert!(
                !host_bridge_help.contains(flag),
                "canonical host bridge help must not advertise legacy completion verdict flag {flag}"
            );
        }

        let parsed_host_bridge = Cli::try_parse_from([
            "vida",
            "agent",
            "host-bridge",
            "--request",
            "/tmp/host-bridge-request.json",
            "--host-agent-id",
            "agent-1",
            "--submit-result",
            "/tmp/host-bridge-result.json",
            "--summary",
            "done",
            "--receipt-id",
            "receipt-1",
            "--state-dir",
            "/tmp/vida-state",
            "--json",
        ])
        .expect("agent host-bridge should parse");
        let Some(super::Command::Agent(agent_args)) = parsed_host_bridge.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::HostBridge(host_bridge) = agent_args.command else {
            panic!("agent host-bridge command should parse");
        };
        assert_eq!(
            host_bridge.request.display().to_string(),
            "/tmp/host-bridge-request.json"
        );
        assert!(!host_bridge.complete);
        assert_eq!(host_bridge.host_agent_id.as_deref(), Some("agent-1"));
        assert_eq!(
            host_bridge
                .submit_result
                .as_deref()
                .map(|path| path.display().to_string()),
            Some("/tmp/host-bridge-result.json".to_string())
        );
        assert_eq!(host_bridge.summary.as_deref(), Some("done"));
        assert_eq!(host_bridge.receipt_id.as_deref(), Some("receipt-1"));
        assert_eq!(
            host_bridge
                .state_dir
                .as_deref()
                .map(|path| path.display().to_string()),
            Some("/tmp/vida-state".to_string())
        );
        assert!(host_bridge.json);

        let parsed_host_bridge_alias = Cli::try_parse_from([
            "vida",
            "agent",
            "host-bridge",
            "--request",
            "/tmp/host-bridge-request.json",
            "--complete",
            "--host-agent-id",
            "agent-1",
            "--host-bridge-summary",
            "done through alias",
        ])
        .expect("agent host-bridge summary alias should parse");
        let Some(super::Command::Agent(agent_args)) = parsed_host_bridge_alias.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::HostBridge(host_bridge_alias) = agent_args.command else {
            panic!("agent host-bridge command should parse");
        };
        assert!(host_bridge_alias.complete);
        assert_eq!(
            host_bridge_alias.summary.as_deref(),
            Some("done through alias")
        );

        let parsed_host_bridge_legacy_completion = Cli::try_parse_from([
            "vida",
            "agent",
            "host-bridge",
            "--request",
            "/tmp/host-bridge-request.json",
            "--complete",
            "--host-agent-id",
            "agent-1",
            "--decision",
            "blocked",
            "--verdict",
            "rework_required",
            "--allowed-next-node",
            "developer",
            "--blocker-codes",
            r#"["legacy_blocker"]"#,
            "--blocker-code",
            "legacy_blocker_2",
            "--rework-target",
            "developer",
        ])
        .expect("agent host-bridge legacy completion flags should remain parse-compatible");
        let Some(super::Command::Agent(agent_args)) = parsed_host_bridge_legacy_completion.command
        else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::HostBridge(host_bridge_legacy_completion) = agent_args.command
        else {
            panic!("agent host-bridge command should parse");
        };
        assert!(host_bridge_legacy_completion.complete);
        assert_eq!(
            host_bridge_legacy_completion.decision.as_deref(),
            Some("blocked")
        );
        assert_eq!(
            host_bridge_legacy_completion.verdict.as_deref(),
            Some("rework_required")
        );
        assert_eq!(
            host_bridge_legacy_completion.allowed_next_node.as_deref(),
            Some("developer")
        );
        assert_eq!(
            host_bridge_legacy_completion.blocker_codes.as_deref(),
            Some(r#"["legacy_blocker"]"#)
        );
        assert_eq!(
            host_bridge_legacy_completion.blocker_code,
            vec!["legacy_blocker_2".to_string()]
        );
        assert_eq!(
            host_bridge_legacy_completion.rework_target.as_deref(),
            Some("developer")
        );

        let parsed_host_bridge_attach = Cli::try_parse_from([
            "vida",
            "agent",
            "host-bridge",
            "--request",
            "/tmp/host-bridge-request.json",
            "--attach-artifact",
            "/tmp/patch-proposal.json",
            "--artifact-kind",
            "isolated_worktree_manifest",
            "--changed-file",
            "crates/vida/src/agent_dispatch_surface.rs",
            "--attempt-id",
            "attempt-1",
            "--consolidation-receipt",
            "receipt-attach-1",
        ])
        .expect("agent host-bridge attach should parse");
        let Some(super::Command::Agent(agent_args)) = parsed_host_bridge_attach.command else {
            panic!("agent command should parse");
        };
        let crate::AgentCommand::HostBridge(host_bridge_attach) = agent_args.command else {
            panic!("agent host-bridge command should parse");
        };
        assert_eq!(
            host_bridge_attach.attach_artifacts[0].display().to_string(),
            "/tmp/patch-proposal.json"
        );
        assert_eq!(
            host_bridge_attach.changed_files,
            vec!["crates/vida/src/agent_dispatch_surface.rs".to_string()]
        );
        assert_eq!(host_bridge_attach.attempt_id.as_deref(), Some("attempt-1"));
        assert_eq!(
            host_bridge_attach.consolidation_receipt_id.as_deref(),
            Some("receipt-attach-1")
        );
        assert_eq!(
            host_bridge_attach.artifact_kind,
            "isolated_worktree_manifest"
        );
        assert!(!host_bridge_attach.complete);
    }

    #[test]
    fn coder_surface_help_and_commands_are_discoverable() {
        let root_error = Cli::try_parse_from(["vida", "--help"])
            .expect_err("help should render clap display error");
        assert!(root_error.to_string().contains("coder"));

        let coder_error = Cli::try_parse_from(["vida", "coder", "--help"])
            .expect_err("help should render clap display error");
        let coder_help = coder_error.to_string();
        assert!(coder_help.contains("capabilities"));
        assert!(coder_help.contains("provider-check"));
        assert!(coder_help.contains("run"));
        assert!(coder_help.contains("Default output is compact TOON/plain"));
        assert!(coder_help.contains("Use --json only when a machine-readable payload is required."));
        assert!(coder_help.contains("vida coder capabilities\n"));
        assert!(!coder_help.contains("vida coder capabilities --json"));

        let capabilities_error = Cli::try_parse_from(["vida", "coder", "capabilities", "--help"])
            .expect_err("help should render clap display error");
        assert!(capabilities_error.to_string().contains("--json"));

        let parsed_provider = Cli::try_parse_from([
            "vida",
            "coder",
            "provider-check",
            "--provider",
            "codex",
            "--json",
        ])
        .expect("coder provider-check should parse");
        let Some(super::Command::Coder(coder_args)) = parsed_provider.command else {
            panic!("coder command should parse");
        };
        let CoderCommand::ProviderCheck(provider_check) = coder_args.command else {
            panic!("provider-check command should parse");
        };
        assert_eq!(provider_check.provider, "codex");
        assert!(provider_check.json);

        let parsed_run = Cli::try_parse_from([
            "vida",
            "coder",
            "run",
            "--request",
            "bounded request",
            "--json",
        ])
        .expect("coder run should parse");
        let Some(super::Command::Coder(coder_args)) = parsed_run.command else {
            panic!("coder command should parse");
        };
        let CoderCommand::Run(run) = coder_args.command else {
            panic!("run command should parse");
        };
        assert_eq!(run.provider, "codex");
        assert_eq!(run.request.as_deref(), Some("bounded request"));
        assert!(run.json);
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
        assert!(next_lawful_help.contains("--strategy"));
        assert!(next_lawful_help.contains("epic-sequential"));
        assert!(next_lawful_help.contains("--select"));
        assert!(next_lawful_help.contains("--explain"));
        assert!(next_lawful_help.contains("--state-dir"));
        assert!(next_lawful_help.contains("--json"));

        let parsed = Cli::try_parse_from([
            "vida",
            "task",
            "next-lawful",
            "--scope",
            "audit-epic",
            "--strategy",
            "epic-sequential",
            "--select",
            "task-1",
            "--explain",
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
        assert_eq!(next_lawful.strategy.as_deref(), Some("epic-sequential"));
        assert_eq!(next_lawful.select.as_deref(), Some("task-1"));
        assert!(next_lawful.explain);
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
        assert!(agent_init_help
            .contains("return receipt-backed execution or host-bridge handoff state"));
        assert!(agent_init_help.contains("Default blocked output is compact TOON/plain"));
        assert!(agent_init_help.contains(
            "Use --json only when a machine-readable payload or full blocked evidence is required"
        ));

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
