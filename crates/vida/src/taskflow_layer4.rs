use std::process::ExitCode;

pub(crate) fn print_taskflow_proxy_help(topic: Option<&str>) {
    match topic {
        Some("task") => {
            println!("VIDA TaskFlow help: task");
            println!();
            println!("Purpose:");
            println!(
                "  Inspect and mutate the primary backlog through the authoritative runtime store."
            );
            println!(
                "  `vida task` is the root parity surface; `vida taskflow task` remains the family-scoped entrypoint."
            );
            println!();
            println!("Source of truth:");
            println!(
                "  Runtime store: vida task and vida taskflow task over the authoritative state store."
            );
            println!("  Canonical snapshot artifact: .vida/exports/tasks.snapshot.jsonl");
            println!(
                "  `vida task replace-jsonl` authoritatively replaces the store from that snapshot artifact."
            );
            println!();
            println!("Dependency semantics:");
            println!("  Parent-child edges preserve epic/task structure.");
            println!("  Blocks edges preserve readiness and execution ordering.");
            println!(
                "  `task ready` returns the current unblocked ready set from the runtime store."
            );
            println!("  Execution semantics are additive scheduling truth on top of the graph.");
            println!(
                "  Missing execution semantics never imply safe parallel execution for write-producing work."
            );
            println!();
            println!("Execution semantics:");
            println!("  execution_mode: sequential | parallel_safe | exclusive");
            println!(
                "  order_bucket: bounded sequencing bucket used by scheduler summaries and wave grouping"
            );
            println!(
                "  parallel_group: opt-in co-scheduling group that must match before parallel-safe admission"
            );
            println!(
                "  conflict_domain: write-collision classifier; matching domains block concurrent execution"
            );
            println!();
            println!("Canonical commands:");
            println!("  vida task list --all --json");
            println!("  vida task ready --json");
            println!("  vida task next [--scope <task-id>] [--state-dir <path>] [--json]");
            println!("  vida task ready --scope <task-id> --json");
            println!("  vida task show <task-id> --json");
            println!("  vida task deps <task-id> --json");
            println!("  vida task reverse-deps <task-id> --json");
            println!("  vida task blocked --json");
            println!("  vida task tree <task-id> --json");
            println!("  vida task critical-path --json");
            println!("  vida task next-display-id <parent-display-id> --json");
            println!(
                "  vida task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description \"...\" --json"
            );
            println!(
                "  vida task ensure <task-id> <title> --parent-id <parent-id> --description \"...\" --labels <label> --json"
            );
            println!(
                "  vida task update <task-id> --status in_progress --notes-file <path> --json"
            );
            println!(
                "  vida task update <task-id> --execution-mode parallel_safe --order-bucket <bucket> --parallel-group <group> --conflict-domain <domain> --json"
            );
            println!("  vida task close <task-id> --reason \"...\" --json");
            println!("  vida task help parallelism");
            println!("  vida task import-jsonl .vida/exports/tasks.snapshot.jsonl --json");
            println!("  vida task replace-jsonl .vida/exports/tasks.snapshot.jsonl --json");
            println!("  vida task export-jsonl .vida/exports/tasks.snapshot.jsonl --json");
            println!();
            println!("Failure modes:");
            println!("  Missing or ambiguous runtime root fails closed.");
            println!(
                "  Invalid task ids, illegal status transitions, or unresolved parent/display ids fail closed from the delegated runtime."
            );
            println!(
                "  Parallel-safe admission fails closed when execution_mode/order_bucket/parallel_group/conflict_domain truth is missing or incompatible."
            );
            println!(
                "  Export artifacts can drift; verify live state with `task show` or `task list`."
            );
            println!();
            println!("Operator recipes:");
            println!("  Check the next lawful slice: vida task ready --json");
            println!(
                "  Read the aggregate next operator step: vida task next [--scope <task-id>] [--state-dir <path>] [--json]"
            );
            println!(
                "  Check the next lawful slice within one subtree: vida task ready --scope <task-id> --json"
            );
            println!("  Inspect one task before mutation: vida task show <task-id> --json");
            println!(
                "  Inspect direct dependencies before resequencing: vida task deps <task-id> --json"
            );
            println!(
                "  Inspect reverse dependencies before closure: vida task reverse-deps <task-id> --json"
            );
            println!("  Inspect the currently blocked set: vida task blocked --json");
            println!(
                "  Inspect one subtree when sequencing nested work: vida task tree <task-id> --json"
            );
            println!(
                "  Inspect the current critical path before parallelizing: vida task critical-path --json"
            );
            println!(
                "  Inspect scheduler truth before parallelizing: vida taskflow graph-summary --json"
            );
            println!("  Read the sequencing/parallelism contract: vida task help parallelism");
            println!(
                "  Reserve the next child display id: vida task next-display-id <parent-display-id> --json"
            );
            println!(
                "  Create one bounded child task: vida task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description \"...\" --json"
            );
            println!(
                "  Reuse-or-create one tracked handoff task idempotently: vida task ensure <task-id> <title> --parent-id <parent-id> --description \"...\" --labels <label> --json"
            );
            println!(
                "  Record real progress after a proven step: vida task update <task-id> --status <status> --notes-file <path> --json"
            );
            println!(
                "  Import one bounded backlog snapshot when explicitly needed: vida task import-jsonl .vida/exports/tasks.snapshot.jsonl --json"
            );
            println!(
                "  Authoritatively replace the current backlog snapshot when needed: vida task replace-jsonl .vida/exports/tasks.snapshot.jsonl --json"
            );
            println!(
                "  Export the current runtime snapshot when needed: vida task export-jsonl .vida/exports/tasks.snapshot.jsonl --json"
            );
            return;
        }
        Some("parallelism") | Some("scheduling") => {
            println!("VIDA TaskFlow help: parallelism");
            println!();
            println!("Purpose:");
            println!(
                "  Explain the first-class sequencing and parallel-safe scheduling contract used by the authoritative task graph."
            );
            println!(
                "  Graph edges remain canonical for hard ordering; execution semantics add bounded scheduling truth on top."
            );
            println!();
            println!("Canonical fields:");
            println!("  execution_mode");
            println!("    sequential    default single-lane posture unless later semantics prove otherwise");
            println!(
                "    parallel_safe opt-in parallel admission; still requires matching order bucket and parallel group plus non-colliding conflict domains"
            );
            println!(
                "    exclusive     explicitly blocks co-scheduling even when the graph itself is unblocked"
            );
            println!("  order_bucket");
            println!(
                "    bounded sequencing bucket used to group tasks that may be considered together by the scheduler"
            );
            println!("  parallel_group");
            println!(
                "    explicit co-scheduling group; mismatched or missing groups fail closed for parallel-safe admission"
            );
            println!("  conflict_domain");
            println!(
                "    write-collision classifier; matching non-empty domains block concurrent execution"
            );
            println!();
            println!("Admission rules:");
            println!("  Graph readiness is necessary but not sufficient for parallel execution.");
            println!("  Missing semantics fail closed: unblocked does not mean parallel-safe.");
            println!(
                "  `ready_parallel_safe` becomes true only when the candidate is ready now and its semantics are compatible with the current bounded unit."
            );
            println!(
                "  `parallel_blockers` explains why a ready task is still unsafe to co-schedule."
            );
            println!();
            println!("Canonical commands:");
            println!("  vida task help parallelism");
            println!("  vida taskflow help parallelism");
            println!("  vida taskflow graph-summary --json");
            println!(
                "  vida task update <task-id> --execution-mode <mode> --order-bucket <bucket> --parallel-group <group> --conflict-domain <domain> --json"
            );
            println!(
                "  vida task create <task-id> <title> --execution-mode <mode> --order-bucket <bucket> --parallel-group <group> --conflict-domain <domain> --json"
            );
            println!();
            println!("Graph-summary fields to inspect:");
            println!("  current_task_id");
            println!("  scheduling.ready[*].ready_now");
            println!("  scheduling.ready[*].ready_parallel_safe");
            println!("  scheduling.ready[*].parallel_blockers");
            println!("  scheduling.parallel_candidates_after_current");
            println!();
            println!("Common blocker codes:");
            println!("  current_task_reference");
            println!("  execution_mode_not_parallel_safe");
            println!("  current_execution_mode_not_parallel_safe");
            println!("  order_bucket_mismatch_or_missing");
            println!("  parallel_group_mismatch");
            println!("  conflict_domain_collision");
            println!("  missing_conflict_domain");
            println!("  graph_blocked");
            println!();
            println!("Failure modes:");
            println!(
                "  Never infer safe concurrency from notes alone; use graph edges plus execution semantics."
            );
            println!(
                "  If the current task itself lacks compatible semantics, other ready tasks remain visible but not parallel-safe."
            );
            return;
        }
        Some("next") => {
            println!("VIDA TaskFlow help: next");
            println!();
            println!("Purpose:");
            println!(
                "  Aggregate the next lawful operator step from backlog readiness, latest run-graph recovery, and bounded continuation state."
            );
            println!(
                "  This is a read-only launcher-owned planning surface over the authoritative TaskFlow state store."
            );
            println!();
            println!("Canonical command:");
            println!("  vida task next [--scope <task-id>] [--state-dir <path>] [--json]");
            println!();
            println!("Returned semantics:");
            println!(
                "  status, blocker_codes, next_actions, recommended_command, scope_task_id, ready_count, primary_ready_task, latest_run_graph, recovery, gate, dispatch"
            );
            println!();
            println!("Failure modes:");
            println!("  Missing or unreadable authoritative state fails closed.");
            println!("  Unknown scoped task ids fail closed from the authoritative task graph.");
            println!(
                "  `next` is an inspection/planning surface and must not be treated as a mutation or dispatch command by itself."
            );
            return;
        }
        Some("graph-summary") => {
            println!("VIDA TaskFlow help: graph-summary");
            println!();
            println!("Purpose:");
            println!(
                "  Summarize backlog graph pressure across the ready set, blocked set, and current critical path."
            );
            println!(
                "  This is a read-only launcher-owned operator surface over the authoritative TaskFlow state store."
            );
            println!();
            println!("Canonical command:");
            println!("  vida taskflow graph-summary [--json]");
            println!();
            println!("Returned semantics:");
            println!(
                "  status, blocker_codes, next_actions, ready_count, blocked_count, critical_path_length, current_task_id, primary_ready_task, primary_blocked_task, scheduling.ready[*].ready_parallel_safe, scheduling.ready[*].parallel_blockers, scheduling.parallel_candidates_after_current, waves, critical_path"
            );
            println!();
            println!("Failure modes:");
            println!("  Missing or unreadable authoritative state fails closed.");
            println!(
                "  Invalid dependency graphs fail closed through the critical-path contract; repair with `vida task validate-graph` first."
            );
            return;
        }
        Some("consume") => {
            println!("VIDA TaskFlow help: consume");
            println!();
            println!("Purpose:");
            println!(
                "  Inspect the bounded TaskFlow runtime-consumption bundle and drive the scheduler-owned closure handoff seam."
            );
            println!(
                "  Bundle inspection, final intake, continuation, and bounded advance are launcher-owned and in-process over authoritative Rust state plus the bounded DocFlow branch."
            );
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow consume bundle [--json]");
            println!("  vida taskflow consume bundle check [--json]");
            println!("  vida taskflow consume agent-system [--json]");
            println!("  vida taskflow consume final \"<request>\" --json");
            println!(
                "  vida taskflow consume continue [--run-id <run-id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]"
            );
            println!(
                "  vida taskflow consume advance [--run-id <run-id>] [--max-rounds <n>] [--json]"
            );
            println!("  vida taskflow bootstrap-spec \"<request>\" --json");
            println!();
            println!("Failure modes:");
            println!(
                "  `bundle` requires a booted authoritative state root and fails closed if runtime bundle surfaces are missing."
            );
            println!("  `agent-system` fails closed when the activation bundle is unavailable.");
            println!("  Unsupported consume modes fail closed.");
            println!(
                "  `final` fails closed when the runtime bundle is not ready or the bounded DocFlow evidence branch returns blocking results."
            );
            println!(
                "  `continue` and `advance` fail closed when no lawful persisted dispatch receipt or packet can be resolved for the requested run."
            );
            println!();
            println!("Operator recipes:");
            println!(
                "  Verify the active runtime bundle before closure packaging: vida taskflow consume bundle check --json"
            );
            println!(
                "  Read one canonical carrier/role/score snapshot: vida taskflow consume agent-system --json"
            );
            println!(
                "  Materialize one routed intake packet: vida taskflow consume final \"<request>\" --json"
            );
            println!(
                "  Resume one persisted chain from the latest or selected packet: vida taskflow consume continue [--run-id <run-id>] --json"
            );
            println!(
                "  Let the bounded scheduler progress ready steps automatically: vida taskflow consume advance [--run-id <run-id>] [--max-rounds <n>] --json"
            );
            return;
        }
        Some("continuation") => {
            println!("VIDA TaskFlow help: continuation");
            println!();
            println!("Purpose:");
            println!(
                "  Record explicit continuation binding for the currently lawful bounded unit."
            );
            println!();
            println!("Canonical commands:");
            println!(
                "  vida taskflow continuation bind <run-id> [--task-id <task-id>] [--why <text>] [--json]"
            );
            println!();
            println!("Returned semantics:");
            println!(
                "  active_bounded_unit, binding_source, why_this_unit, primary_path, sequential_vs_parallel_posture"
            );
            println!();
            println!("Failure modes:");
            println!(
                "  Binding fails closed when the run does not expose a bindable active bounded unit."
            );
            println!(
                "  Explicit backlog-task binding fails closed when the cited task is missing or already closed."
            );
            println!("  Explicit binding does not replace persisted dispatch receipt evidence.");
            return;
        }
        Some("packet") => {
            println!("VIDA TaskFlow help: packet");
            println!();
            println!("Purpose:");
            println!(
                "  Render persisted dispatch packet evidence and lawful resume inputs for one run."
            );
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow packet render <run-id> [--json]");
            println!();
            println!("Returned semantics:");
            println!(
                "  dispatch receipt, dispatch/downstream packet bodies, persisted packet paths, continue command"
            );
            println!();
            println!("Failure modes:");
            println!(
                "  Packet rendering fails closed when no persisted dispatch receipt or packet path exists."
            );
            return;
        }
        Some("run-graph") => {
            println!("VIDA TaskFlow help: run-graph");
            println!();
            println!("Purpose:");
            println!("  Create and inspect node-level execution state for one routed task run.");
            println!(
                "  Run-graph is not a second task queue; it complements task lifecycle state."
            );
            println!(
                "  The current run-graph surface is launcher-owned and in-process for both mutation and inspection."
            );
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow run-graph seed <task_id> <request_text> [--json]");
            println!("  vida taskflow run-graph advance <task_id> [--json]");
            println!("  vida taskflow run-graph dispatch-init <task_id> [--json]");
            println!("  vida taskflow run-graph init <task_id> <task_class> [route_task_class]");
            println!(
                "  vida taskflow run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]"
            );
            println!("  vida taskflow run-graph status <task_id>");
            println!("  vida taskflow run-graph latest [--json]");
            println!();
            println!("Failure modes:");
            println!(
                "  `seed` fails closed when overlay-driven lane selection or agent-system bundle validation fails."
            );
            println!(
                "  `advance` currently fails closed unless the run is a seeded implementation or seeded scope-discussion dispatch."
            );
            println!(
                "  `dispatch-init` fails closed when no persisted seeded dispatch context exists for the selected run."
            );
            println!(
                "  Clean implementation review enters an explicit approval wait; mark approval explicitly through `vida taskflow run-graph update <task-id> implementation review_ensemble approved implementation` before the final completion advance."
            );
            println!("  Invalid JSON in meta_json fails closed before mutation.");
            println!("  `latest` returns `none`/`null` when no routed run has been recorded yet.");
            println!("  Run-graph state must not be treated as backlog readiness authority.");
            return;
        }
        Some("recovery") => {
            println!("VIDA TaskFlow help: recovery");
            println!();
            println!("Purpose:");
            println!(
                "  Inspect donor-aligned resumability state derived from the authoritative Rust run-graph contract."
            );
            println!("  Recovery status is a read-only launcher-owned inspection surface.");
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow recovery status <run-id> [--json]");
            println!("  vida taskflow recovery latest [--json]");
            println!("  vida taskflow recovery checkpoint <run-id> [--json]");
            println!("  vida taskflow recovery checkpoint-latest [--json]");
            println!("  vida taskflow recovery gate <run-id> [--json]");
            println!("  vida taskflow recovery gate-latest [--json]");
            println!("  vida lane show <run-id> [--json]");
            println!("  vida lane exception-takeover <run-id> --receipt-id <id> [--json]");
            println!("  vida lane supersede <run-id> --receipt-id <id> [--json]");
            println!();
            println!("Returned semantics:");
            println!(
                "  resume_node, resume_status, checkpoint_kind, resume_target, policy_gate, handoff_state, recovery_ready"
            );
            println!();
            println!("Recovery-critical lane mutations:");
            println!(
                "  Record exception-path evidence with `vida lane exception-takeover` before any local takeover path."
            );
            println!(
                "  Record explicit supersession with `vida lane supersede` before treating admissible takeover as active authority."
            );
            println!(
                "  Inspect the current lane envelope with `vida lane show` when recovery and write-guard posture disagree."
            );
            println!();
            println!("Failure modes:");
            println!("  Missing run ids fail closed from the authoritative state store.");
            println!("  `latest` returns `none`/`null` when no routed run has been recorded yet.");
            println!("  Recovery state must not be treated as backlog readiness authority.");
            return;
        }
        Some("doctor") => {
            println!("VIDA TaskFlow help: doctor");
            println!();
            println!("Purpose:");
            println!(
                "  Diagnose launcher/runtime health for bootstrap, task-store visibility, and graph integrity."
            );
            println!();
            println!("Canonical command:");
            println!("  vida taskflow doctor [--json]");
            println!();
            println!("Checks currently surfaced:");
            println!("  storage metadata");
            println!("  authoritative state spine");
            println!("  task store summary");
            println!("  run graph summary");
            println!("  dependency graph integrity");
            println!("  protocol-binding summary and latest receipt posture");
            println!("  runtime-consumption evidence posture");
            println!("  latest recovery, checkpoint, gate, and dispatch receipt summaries");
            println!("  boot compatibility, migration preflight, and effective bundle integrity");
            println!("  retrieval-trust and release-admission evidence parity");
            println!();
            println!("Failure modes:");
            println!(
                "  Broken state roots, incompatible migration posture, or missing runtime artifacts fail closed."
            );
            return;
        }
        Some("protocol-binding") => {
            println!("VIDA TaskFlow help: protocol-binding");
            println!();
            println!("Purpose:");
            println!(
                "  Materialize and inspect the bounded Wave-1 protocol-binding bridge over the authoritative TaskFlow state store."
            );
            println!(
                "  Binding truth lives in the DB-backed runtime state, not in detached file logs."
            );
            println!();
            println!("Canonical commands:");
            println!("  vida taskflow protocol-binding sync [--json]");
            println!("  vida taskflow protocol-binding status [--json]");
            println!("  vida taskflow protocol-binding check [--json]");
            println!();
            println!("Returned semantics:");
            println!(
                "  scenario, binding status, active protocol count, blockers, primary state authority, latest receipt"
            );
            println!();
            println!("Failure modes:");
            println!("  `sync` fails closed when canonical protocol sources are missing.");
            println!(
                "  `check` fails closed when no sync receipt exists or the latest receipt still has unbound/blocking rows."
            );
            println!("  Detached JSON export alone is not treated as binding closure.");
            return;
        }
        Some(_) | None => {}
    }

    println!("VIDA TaskFlow runtime family");
    println!();
    println!("Usage:");
    println!("  vida taskflow <args...>");
    println!(
        "  vida taskflow help [task|parallelism|next|graph-summary|status|consume|continuation|packet|run-graph|recovery|doctor|protocol-binding|query]"
    );
    println!("  vida taskflow <command> --help");
    println!();
    println!("Purpose:");
    println!(
        "  Enter the TaskFlow runtime family for tracked execution, backlog state, run-graph state, and closure handoff."
    );
    println!();
    println!("Source of truth notes:");
    println!("  TaskFlow is the execution/runtime authority.");
    println!(
        "  `vida task` and `vida taskflow task` address the same authoritative backlog store."
    );
    println!("  `.vida/exports/tasks.snapshot.jsonl` is export-only, not the live runtime store.");
    println!();
    println!("Runtime routing:");
    println!(
        "  In a project tree, vida resolves the root from the current working directory without manual VIDA_ROOT export."
    );
    println!(
        "  In repo mode the delegated runtime resolves to the local TaskFlow runtime implementation."
    );
    println!(
        "  In installed mode it resolves the sibling taskflow binary from the active vida bin root."
    );
    println!("  Unknown roots or missing binaries fail closed.");
    println!();
    println!("Most-used command homes:");
    println!("  task        backlog inspection and mutation");
    println!("  next        aggregate next lawful step across backlog and recovery state");
    println!("  graph-summary  ready/blocked pressure plus critical-path summary");
    println!("  parallelism explicit sequencing and parallel-safe scheduling contract");
    println!("  status      family-scoped alias to the root operator status surface");
    println!("  continuation explicit bounded-unit binding");
    println!("  packet      persisted runtime packet inspection");
    println!("  run-graph   resumability and node-state inspection");
    println!("  consume     explicit TaskFlow -> final closure handoff");
    println!("  query       launcher-owned command-discovery helper");
    println!(
        "  bootstrap-spec  one-shot epic/spec/doc bootstrap for design-first feature requests"
    );
    println!("  protocol-binding  bounded protocol/runtime bridge receipts");
    println!();
    println!("Canonical examples:");
    println!("  vida task ready --json");
    println!("  vida task next --json");
    println!("  vida taskflow graph-summary --json");
    println!("  vida taskflow help parallelism");
    println!("  vida taskflow status --summary --json");
    println!("  vida task show <task-id> --json");
    println!("  vida taskflow run-graph status <task-id>");
    println!("  vida taskflow continuation bind <run-id> --task-id <task-id> --json");
    println!("  vida taskflow run-graph dispatch-init <task-id> --json");
    println!("  vida taskflow packet render <task-id> --json");
    println!("  vida taskflow consume final \"proof path\" --json");
    println!("  vida taskflow consume continue --json");
    println!("  vida taskflow consume advance --max-rounds 4 --json");
    println!("  vida taskflow bootstrap-spec \"feature request\" --json");
    println!();
    println!("Operator recipes:");
    println!("  Find the next lawful step: vida task next --json");
    println!("  Inspect ready vs blocked pressure: vida taskflow graph-summary --json");
    println!(
        "  Inspect sequencing and parallel-safe admission rules: vida taskflow help parallelism"
    );
    println!("  Inspect TaskFlow-wide operator posture: vida taskflow status --summary --json");
    println!("  Inspect the canonical backlog contract: vida task --help");
    println!("  Ask which surface to use: vida taskflow query \"what should I run next?\"");
    println!("  Bind the current bounded unit explicitly: vida taskflow help continuation");
    println!("  Inspect persisted packet evidence: vida taskflow help packet");
    println!("  Inspect resumability state: vida taskflow help run-graph");
    println!("  Review runtime diagnostics: vida taskflow help doctor");
    println!();
    println!("Failure modes:");
    println!(
        "  Missing runtime family binary, ambiguous root, and unsupported delegated arguments fail closed."
    );
    println!("  Use topic help to inspect command contracts before mutating runtime state.");
    println!(
        "  A green test, successful build, or commentary update is not a stop boundary when a next lawful continuation item is already known."
    );
    println!(
        "  User-ordered execution takes priority over self-directed cleanup or adjacent development unless the user explicitly authorizes a broader scope."
    );
}

pub(crate) fn taskflow_help_topic(args: &[String]) -> Option<Option<&str>> {
    match args {
        [] => Some(None),
        [head] if matches!(head.as_str(), "help" | "--help" | "-h") => Some(None),
        [head, topic, ..] if head == "help" => Some(Some(topic.as_str())),
        [command, flag, ..] if matches!(flag.as_str(), "--help" | "-h") => {
            Some(Some(command.as_str()))
        }
        _ => None,
    }
}

struct TaskflowQueryAnswer<'a> {
    intent: &'a str,
    why: &'a str,
    command: &'a str,
    failure_modes: &'a str,
}

fn taskflow_query_answer(query: &str) -> TaskflowQueryAnswer<'static> {
    let normalized = query.to_ascii_lowercase();
    if normalized.contains("parallel")
        || normalized.contains("parallel safe")
        || normalized.contains("parallel-safe")
        || normalized.contains("parallelism")
        || normalized.contains("sequencing")
        || normalized.contains("execution mode")
        || normalized.contains("order bucket")
        || normalized.contains("parallel group")
        || normalized.contains("conflict domain")
        || normalized.contains("co-schedul")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-parallelism",
            why: "Sequencing and parallel-safe admission are now first-class task semantics, so the safest operator path is to inspect the scheduler projection and the explicit parallelism contract together.",
            command: "vida taskflow graph-summary --json",
            failure_modes: "Graph readiness alone is not parallel authority. If the scheduler output is unclear, read `vida task help parallelism` and treat missing execution semantics as fail-closed for co-scheduling.",
        };
    }

    if normalized.contains("next display")
        || normalized.contains("display id")
        || normalized.contains("child slot")
    {
        return TaskflowQueryAnswer {
            intent: "next-display-id",
            why: "Display-id reservation should come from the live backlog runtime before creating a new child task under an epic.",
            command: "vida task next-display-id <parent-display-id> --json",
            failure_modes: "Unknown parent display ids fail closed in the delegated runtime, and the returned slot should be treated as runtime-state dependent until the child task is actually created.",
        };
    }

    if normalized.contains("next")
        || normalized.contains("ready")
        || normalized.contains("what should i run")
        || normalized.contains("what do i run")
    {
        return TaskflowQueryAnswer {
            intent: "next-ready-slice",
            why: "TaskFlow readiness is the canonical way to pick the next unblocked execution slice.",
            command: "vida task next --json",
            failure_modes: "Next-step output depends on current runtime state; inspect the embedded blockers, ready task, and recovery summary before mutating runtime state.",
        };
    }

    if normalized.contains("latest")
        && (normalized.contains("run-graph")
            || normalized.contains("run graph")
            || normalized.contains("recovery"))
    {
        return TaskflowQueryAnswer {
            intent: "inspect-latest-resumability",
            why: "Latest run-graph and recovery inspection surfaces are the canonical launcher-owned summaries for the most recent routed run.",
            command: "vida taskflow recovery latest --json",
            failure_modes: "Latest recovery inspection returns null when no routed run exists yet and must not be treated as backlog readiness authority.",
        };
    }

    if normalized.contains("gate") {
        return TaskflowQueryAnswer {
            intent: "inspect-gate",
            why: "Gate inspection is the bounded recovery projection for policy gate, handoff state, and context state on one routed run.",
            command: "vida taskflow recovery gate <run-id> --json",
            failure_modes: "Gate inspection must not be treated as backlog readiness authority, and missing run ids fail closed.",
        };
    }

    if normalized.contains("approval")
        || normalized.contains("approve")
        || normalized.contains("approval wait")
    {
        return TaskflowQueryAnswer {
            intent: "record-approval",
            why: "Implementation runs now stop at an explicit approval wait after clean review and require an explicit approval status before final completion.",
            command: "vida taskflow run-graph update <task-id> implementation review_ensemble approved implementation",
            failure_modes: "Approval should be recorded only for the active review node on the intended run; incorrect task ids or route context will fail closed or mutate the wrong run state.",
        };
    }

    if normalized.contains("protocol binding")
        || normalized.contains("protocol-binding")
        || normalized.contains("binding status")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-protocol-binding",
            why: "The Wave-1 protocol bridge should be inspected through the bounded TaskFlow protocol-binding surface backed by the authoritative state store.",
            command: "vida taskflow protocol-binding status --json",
            failure_modes: "If no protocol-binding receipt exists yet, run `vida taskflow protocol-binding sync --json` first and treat detached file logs as non-authoritative.",
        };
    }

    if normalized.contains("show")
        || normalized.contains("inspect")
        || normalized.contains("task id")
        || normalized.contains("one task")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-task",
            why: "Task inspection should read one canonical record from the runtime store before mutation.",
            command: "vida task show <task-id> --json",
            failure_modes: "Unknown task ids fail closed in the delegated runtime.",
        };
    }

    if normalized.contains("create")
        || normalized.contains("new task")
        || normalized.contains("add task")
        || normalized.contains("new slice")
        || normalized.contains("backlog item")
    {
        return TaskflowQueryAnswer {
            intent: "create-task",
            why: "New tracked work should be created directly in the primary backlog runtime with an explicit parent and display-id allocation path.",
            command: "vida task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description \"...\" --json",
            failure_modes: "Task ids must remain stable, parent/display references must resolve in the delegated runtime, and creation should be recorded only after the target epic or parent task has been confirmed.",
        };
    }

    if normalized.contains("update")
        || normalized.contains("progress")
        || normalized.contains("status")
    {
        return TaskflowQueryAnswer {
            intent: "record-progress",
            why: "Progress should be recorded against the primary backlog store after a proven runtime or documentation step.",
            command: "vida task update <task-id> --status in_progress --notes-file <path> --json",
            failure_modes: "Illegal status transitions or missing task ids fail closed in the delegated runtime. When shell metacharacters or multiline notes are involved, prefer `--notes-file` over inline shell quoting.",
        };
    }

    if normalized.contains("close")
        || normalized.contains("done")
        || normalized.contains("completed")
    {
        return TaskflowQueryAnswer {
            intent: "close-task",
            why: "Closure should happen only after proof/doc sync confirms the slice is complete.",
            command: "vida task close <task-id> --reason \"...\" --json",
            failure_modes: "Closing the wrong task mutates the primary backlog; inspect the task first if the identifier is uncertain.",
        };
    }

    if (normalized.contains("replace") && normalized.contains("snapshot"))
        || normalized.contains("apply snapshot")
        || normalized.contains("authoritative replace")
        || normalized.contains("snapshot replace")
        || normalized.contains("restore snapshot")
    {
        return TaskflowQueryAnswer {
            intent: "replace-backlog-snapshot",
            why: "Authoritative backlog replacement should use the canonical snapshot artifact and the store's replace path instead of additive import-only wiring.",
            command: "vida task replace-jsonl <path> --json",
            failure_modes: "Replacement mutates the live backlog by removing stale tasks absent from the snapshot; inspect the artifact first if identity or completeness is uncertain.",
        };
    }

    if normalized.contains("export") || normalized.contains("jsonl") {
        return TaskflowQueryAnswer {
            intent: "export-runtime-store",
            why: "JSONL export is the bounded compatibility snapshot for the current backlog/runtime state, not the live source of truth.",
            command: "vida task export-jsonl .vida/exports/tasks.snapshot.jsonl --json",
            failure_modes: "Export artifacts can drift immediately after they are written, so verify live state through the runtime store when operator decisions depend on freshness.",
        };
    }

    if normalized.contains("resume")
        || normalized.contains("resum")
        || normalized.contains("run-graph")
        || normalized.contains("run graph")
        || normalized.contains("recovery")
    {
        return TaskflowQueryAnswer {
            intent: "inspect-resumability",
            why: "Run-graph and recovery state are the canonical node-level resumability surfaces for one routed execution run.",
            command: "vida taskflow recovery latest --json",
            failure_modes: "Recovery inspection must not be treated as backlog readiness authority; when no latest run exists, continue via `vida taskflow consume continue --json` or inspect a specific run id explicitly.",
        };
    }

    if normalized.contains("checkpoint") {
        return TaskflowQueryAnswer {
            intent: "inspect-checkpoint",
            why: "Checkpoint state is the bounded recovery projection for resume target and checkpoint kind on one routed run.",
            command: "vida taskflow recovery checkpoint <run-id> --json",
            failure_modes: "Checkpoint inspection must not be treated as backlog readiness authority, and missing run ids fail closed.",
        };
    }

    if normalized.contains("doctor")
        || normalized.contains("diagnose")
        || normalized.contains("health")
        || normalized.contains("broken")
    {
        return TaskflowQueryAnswer {
            intent: "diagnose-runtime",
            why: "Launcher/runtime health should be checked through the fail-closed doctor surface before further mutation.",
            command: "vida taskflow doctor --json",
            failure_modes: "Doctor reports the current local runtime state only; incompatible boot/migration posture must be resolved before continuing.",
        };
    }

    if normalized.contains("final")
        || normalized.contains("consume")
        || normalized.contains("closure")
        || normalized.contains("handoff")
    {
        return TaskflowQueryAnswer {
            intent: "closure-handoff",
            why: "Direct consumption is the explicit TaskFlow-to-closure bridge when implementation and proof are already complete.",
            command: "vida taskflow consume final \"<request>\" --json",
            failure_modes: "Use only at closure time; final consumption now fails closed when the runtime bundle is not ready or the bounded DocFlow evidence branch returns blocking results.",
        };
    }

    TaskflowQueryAnswer {
        intent: "help-fallback",
        why: "No confident workflow match was found, so the safest bounded answer is the canonical help surface.",
        command: "vida taskflow help",
        failure_modes: "If the query is too vague, inspect topic help first and then rerun a more specific query.",
    }
}

fn print_taskflow_query_help() {
    println!("VIDA TaskFlow query");
    println!();
    println!("Purpose:");
    println!(
        "  Answer common operator workflow questions with one bounded recommended TaskFlow command."
    );
    println!(
        "  The query surface is deterministic and launcher-owned; it does not call models or external tools."
    );
    println!();
    println!("Usage:");
    println!("  vida taskflow query \"what should I run next?\"");
    println!("  vida taskflow query \"how do I inspect one task?\"");
    println!("  vida taskflow query \"what can run in parallel with the current task?\"");
    println!("  vida taskflow query \"how do I create a new task under this epic?\"");
    println!("  vida taskflow query \"how do I replace the current backlog snapshot?\"");
    println!("  vida taskflow query \"how do I check resumability?\"");
    println!();
    println!("Current intents:");
    println!(
        "  parallelism/scheduling, next/ready, inspect/show, create/new, update/progress, close/done, display-id, export/jsonl, replace/snapshot, resume/run-graph, doctor/health, final/consume, protocol-binding"
    );
    println!();
    println!("Failure modes:");
    println!("  Vague queries fall back to `vida taskflow help`.");
    println!(
        "  Query/help output is advisory only and does not authorize stopping when a next lawful bounded step is already known."
    );
}

pub(crate) fn run_taskflow_query(args: &[String]) -> ExitCode {
    match args {
        [head] if matches!(head.as_str(), "query") => {
            print_taskflow_query_help();
            ExitCode::SUCCESS
        }
        [head, flag] if head == "query" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_query_help();
            ExitCode::SUCCESS
        }
        [head, query @ ..] if head == "query" => {
            let joined = query.join(" ");
            let answer = taskflow_query_answer(&joined);
            println!("VIDA TaskFlow query answer");
            println!();
            println!("Query:");
            println!("  {joined}");
            println!("Intent:");
            println!("  {}", answer.intent);
            println!("Why:");
            println!("  {}", answer.why);
            println!("Recommended command:");
            println!("  {}", answer.command);
            println!("Failure modes:");
            println!("  {}", answer.failure_modes);
            ExitCode::SUCCESS
        }
        _ => ExitCode::from(2),
    }
}

#[cfg(test)]
mod tests {
    use super::taskflow_query_answer;

    #[test]
    fn taskflow_query_answer_routes_parallelism_requests() {
        let answer = taskflow_query_answer("what can run in parallel with the current task");
        assert_eq!(answer.intent, "inspect-parallelism");
        assert_eq!(answer.command, "vida taskflow graph-summary --json");
        let why = answer.why.to_lowercase();
        assert!(why.contains("parallel-safe"));
        assert!(why.contains("scheduler projection"));
    }

    #[test]
    fn taskflow_query_answer_routes_replace_snapshot_requests() {
        let answer = taskflow_query_answer("replace snapshot artifact");
        assert_eq!(answer.intent, "replace-backlog-snapshot");
        assert_eq!(answer.command, "vida task replace-jsonl <path> --json");
        let why = answer.why.to_lowercase();
        assert!(why.contains("backlog replacement"));
        assert!(why.contains("canonical snapshot artifact"));
    }
}
