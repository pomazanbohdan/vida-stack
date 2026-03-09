# VIDA 0.3 Command Tree Spec

Purpose: define the future `vida` operator surface for direct `1.0`, freeze command-family law, and separate preserved command semantics from discardable `0.1` shell topology.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

The future `vida` command tree is frozen at the root-family level as:

1. `vida boot`
2. `vida task ...`
3. `vida memory ...`
4. `vida status`
5. `vida doctor`

This is command-law, not shell-topology law.

`1.0` must preserve:

1. operator intent,
2. command boundaries,
3. gate semantics,
4. compact output expectations,
5. single-writer and fail-closed runtime law.

`1.0` must not preserve mechanically:

1. `/vida-*` slash-command names as product law,
2. `*.sh|*.py` entrypoints,
3. `br`/`.beads`/`.vida` storage topology,
4. helper/path/flag naming,
5. markdown-first startup as the primary operator surface.

Compact rule:

`freeze the five future operator families; map current semantics into them; discard shell-era topology`

---

## 2. Why This Spec Comes Next

This artifact must come before state, instruction, route, and migration internals because the operator surface is the public product contract.

It freezes:

1. where boot begins,
2. which root commands exist,
3. which family owns task mutation,
4. which family is read-only,
5. where diagnosis and migration visibility live.

Without this boundary, later kernel specs would risk freezing internal schemas before the command surface they are supposed to serve is stable.

---

## 3. Source Basis

Local source basis:

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/command-instructions.command-layer-protocol.md`
4. `vida/config/instructions/instruction-contracts.orchestration-protocol.md`
5. `vida/config/instructions/system-maps.protocol-index.md`
6. `vida/config/instructions/agent-definitions.protocol.md`
7. `vida/config/instructions/runtime-instructions.framework-memory-protocol.md`
8. `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`
9. `vida/config/instructions/command-instructions.commands.md`
10. `vida/config/instructions/command-instructions.vida-research.md`
11. `vida/config/instructions/command-instructions.vida-spec.md`
12. `vida/config/instructions/command-instructions.vida-form-task.md`
13. `vida/config/instructions/command-instructions.vida-implement.md`
14. `vida/config/instructions/command-instructions.vida-bug-fix.md`
15. `vida/config/instructions/command-instructions.vida-status.md`
16. `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
17. `docs/framework/plans/vida-semantic-extraction-layer-map.md`
18. `docs/framework/plans/vida-0.2-semantic-freeze-spec.md`
19. `docs/framework/plans/vida-0.2-bridge-policy.md`
20. `docs/framework/plans/vida-direct-1.0-local-spec-program.md`

Target-direction source basis:

1. `https://raw.githubusercontent.com/pomazanbohdan/vida-stack/refs/heads/main/README.md`
2. `https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/VERSION-PLAN.md`

Bounded external memory reference reviewed but not adopted as product law here:

1. `https://raw.githubusercontent.com/pomazanbohdan/memory-mcp-1file/refs/heads/master/README.md`
2. `https://raw.githubusercontent.com/pomazanbohdan/memory-mcp-1file/refs/heads/master/ARCHITECTURE.md`
3. `https://raw.githubusercontent.com/pomazanbohdan/memory-mcp-1file/refs/heads/master/src/storage/surrealdb/memory_ops.rs`
4. `https://raw.githubusercontent.com/pomazanbohdan/memory-mcp-1file/refs/heads/master/src/storage/schema.surql`

Source synthesis rule:

1. current `0.1` commands provide semantic source material,
2. the future `vida-stack` direction provides the root operator surface,
3. when they differ, preserve the behavior and boundary, not the old shell-era naming.

---

## 4. Purpose Of The Command Tree Spec

This spec exists to answer one question:

1. what future operator-visible command families must `vida` expose so that later kernels can be designed under a stable command boundary?

This spec does not define:

1. exact `SurrealDB` record schema,
2. exact instruction serialization,
3. exact receipt payload shapes,
4. exact migration procedures,
5. exact Rust module layout.

It defines:

1. the future root command surface,
2. which current command semantics survive,
3. which current command/doc/script surfaces are discardable topology,
4. which downstream kernel contracts each command family depends on.

---

## 5. Semantic Transfer From `0.1` To `1.0`

Current `0.1` semantic command families are:

1. `research`
2. `spec`
3. `form-task`
4. `implement`
5. `bug-fix`
6. `status`

Current semantic support surfaces also exist for:

1. boot and compact resume,
2. framework memory,
3. silent diagnosis and operator health visibility.

Direct `1.0` does not keep those as separate top-level shell families.

Instead:

1. `status` survives directly as `vida status`,
2. boot/resume semantics become `vida boot`,
3. memory semantics become `vida memory ...`,
4. diagnosis/health semantics become `vida doctor`,
5. `research|spec|form-task|implement|bug-fix` collapse into the semantic sub-family space under `vida task ...`.

Transfer rule:

1. preserve the behavior,
2. allow the topology to change,
3. do not require the old names to survive above the root-family boundary.

---

## 6. Future Root Operator Surface

### 6.1 `vida boot`

Purpose:

1. primary startup entry for the local binary,
2. compact-safe runtime hydration,
3. startup compatibility and migration preflight,
4. packed startup guidance for the next operator action.

Semantic responsibilities:

1. resolve bootstrap boundary,
2. load effective instructions from the runtime-owned instruction layer,
3. restore authoritative runtime state and resumable position,
4. expose boot-time blockers before wider execution,
5. emit compact startup outputs suitable for human or agent continuation.

Must not own:

1. general task mutation,
2. diagnostic deep-dive flows better served by `doctor`,
3. full dashboard rendering better served by `status`.

### 6.2 `vida task ...`

Purpose:

1. singular workflow/task execution family,
2. the only root family that owns workflow/task mutation,
3. the command family that absorbs the current `research|spec|form-task|implement|bug-fix` semantic chain.

Required semantic lanes inside `task`:

1. evidence/contract formation,
2. planning and dependency formation,
3. launch/readiness gating,
4. execution/materialization,
5. bug-fix and regression-proven repair,
6. verification/review/close/handoff,
7. read-only task inspection.

Naming rule:

1. the exact subcommand names below `vida task` remain intentionally open at this stage,
2. the semantic lanes above are frozen,
3. later instruction and route specs may refine subcommand naming without invalidating this root family.

Hard boundary:

1. `vida task ...` is the only family that may own authoritative workflow-state mutation in normal operation,
2. other root families may observe, diagnose, or summarize task state, but must not silently mutate it.

### 6.3 `vida memory ...`

Purpose:

1. durable memory access for framework/project operation,
2. explicit capture and retrieval of lessons, corrections, and anomalies,
3. operator-visible memory summaries.

Frozen semantic scope:

1. record durable memory,
2. query durable memory,
3. inspect memory summaries and recent entries,
4. distinguish durable memory from chat recap, logs, and scorecards.

Bounded future direction acknowledged by external references:

1. richer local search, update, delete, and invalidation semantics may appear later,
2. embedded-store backing is compatible with current local-first `1.0` direction,
3. none of that widens this artifact into MCP protocol, graph/vector internals, or storage-schema law.

Boundary:

1. `memory` is an operator family now,
2. its detailed backing kernel/storage contract is not fully defined by this artifact,
3. no MCP/server/plugin/vector surface is implied by defining this root family.

### 6.4 `vida status`

Purpose:

1. low-cost read-only operator visibility,
2. concise runtime and workflow status,
3. operator-facing summary of readiness, progress, approvals, and resumable position.

Frozen semantic law:

1. `status` is informational only,
2. it must not mutate task state, queue order, or runtime mode,
3. it summarizes authoritative runtime artifacts rather than creating new state.

Status may summarize:

1. workflow/task position,
2. readiness/blocker posture,
3. approval and verification surface counts,
4. memory/diagnosis/run-graph visibility when useful.

Status must not replace:

1. `doctor` for diagnosis,
2. `memory` for durable memory operations,
3. `task` for state mutation.

### 6.5 `vida doctor`

Purpose:

1. diagnose runtime, compatibility, migration, instruction, and governance problems,
2. expose fail-closed blockers before unsafe execution,
3. provide structured remediation guidance.

Frozen semantic scope:

1. startup and compatibility checks,
2. migration readiness and blocker reporting,
3. instruction/runtime drift detection,
4. diagnosis of framework/runtime friction,
5. explicit evidence and remediation outputs.

Doctor boundary:

1. `doctor` explains and proves blocker state,
2. it does not become a silent bypass around route, approval, or migration law,
3. it may recommend or trigger bounded repair flows later, but that repair law is outside this spec.

---

## 7. Command-Family Mapping From Current `0.1`

| Current semantic source | Preserved future family | What survives | What does not survive |
|---|---|---|---|
| boot snapshot + bootstrap router | `vida boot` | boot boundary, compact resume, startup guidance | `python3 vida-boot-snapshot.py`, `boot-profile.sh`, receipt file names |
| `/vida-status` + operator status surfaces | `vida status` | read-only operator visibility | `bash vida-status.sh`, `br ... --json`, tmp-file choreography |
| `/vida-research` | `vida task ...` | evidence gathering and handoff semantics | top-level slash name and markdown doc path |
| `/vida-spec` | `vida task ...` | contract formation, reality validation, readiness semantics | top-level slash name and SCP shell-era surface naming |
| `/vida-form-task` | `vida task ...` | planning, dependency graph, readiness verdict, launch gate | helper-driven pack/task-pool shell plumbing |
| `/vida-implement` | `vida task ...` | single-writer execution loop, drift stop, verify/review then continue | old command name and current script entrypoints |
| `/vida-bug-fix` | `vida task ...` | root-cause-first repair and regression closure law | separate shell-era top-level product surface |
| framework memory surfaces | `vida memory ...` | durable lesson/correction/anomaly semantics | `framework-memory.py` CLI spelling and file path |
| silent diagnosis and self-check surfaces | `vida doctor` | diagnosis and blocker-report semantics | `vida-silent-diagnosis.py` CLI spelling and current path layout |

Mapping rule:

1. current semantic families are source material,
2. future root families are the product contract,
3. old family names survive only when they coincide with the future root family law.

---

## 8. Semantic Command Law Vs Discardable `0.1` Topology

### 8.1 Preserve Semantically

Preserve:

1. the five future root families,
2. the shared command-layer model `CL1 Intake -> CL2 Reality And Inputs -> CL3 Contract And Decisions -> CL4 Materialization -> CL5 Gates And Handoff`,
3. request-intent and tracked-flow boundary semantics,
4. read-only `status`,
5. single-writer task mutation ownership,
6. launch-gated execution,
7. root-cause-first bug-fix closure law,
8. compact outputs and explicit next-step handoff semantics,
9. durable memory as a first-class runtime surface,
10. diagnosis as a first-class runtime surface,
11. fail-closed startup and compatibility posture.

### 8.2 Discard Mechanically

Do not freeze:

1. `vida/config/instructions/command-instructions.commands.md` or `docs/framework/commands/vida-*.md` as product command topology,
2. `bash ...` or `python3 ...` entrypoints,
3. pack/helper verbs such as `detect|start|scaffold|end`,
4. shell profiles such as `lean|standard|full`,
5. `br`/`beads_br` flags, JSON/JSONL command shapes, or `--no-db`,
6. `.vida/*` and `.beads/*` paths,
7. tmp-file choreography or `.latest.*` receipt naming,
8. current CLI-only worker transport strings,
9. markdown-first startup and repo reread as the primary runtime UX,
10. current shell/Python helper boundaries or file layout.

Negative-filter rule:

1. if a proposed command-tree node names a current helper, path, storage backend, or shell-era flag as product law, reject it as topology leakage.

---

## 9. Command-Level Kernel Dependencies

### 9.1 Dependency On State Kernel Schema Spec

The command tree depends on the state kernel for:

1. authoritative workflow entities,
2. statuses and transitions,
3. blocker and dependency representation,
4. resumable runtime position,
5. command-visible state mutation boundaries.

State-kernel consequence:

1. `vida task ...` cannot be finalized below the root-family boundary until task/state vocabulary and mutation law are specified.

### 9.2 Dependency On Instruction Kernel Spec

The command tree depends on the instruction kernel for:

1. runtime-owned instruction loading,
2. effective command behavior composition,
3. overlay validation and precedence,
4. the rule that `Instruction Contract` owns behavior and `Prompt Template Configuration` does not.

Instruction-kernel consequence:

1. command families are frozen here,
2. command capsules and final subcommand composition rules are refined later in the instruction kernel.

### 9.3 Dependency On Route And Receipt Spec

The command tree depends on route/receipt law for:

1. analysis/write/review/verification gating,
2. approval and escalation receipts,
3. machine-readable completion and blocker artifacts,
4. command-visible handoff and closure proof.

Route consequence:

1. this spec freezes where those receipts surface in operator UX,
2. later route/receipt work defines exact payloads and authorization law.

### 9.4 Dependency On Migration Kernel Spec

The command tree depends on migration law for:

1. startup compatibility checks,
2. fail-closed schema/instruction upgrades,
3. bridge import from `0.1` artifacts,
4. doctor-visible migration status and blockers.

Migration consequence:

1. `boot` and `doctor` are frozen now as the operator homes for migration visibility,
2. migration kernel later defines exact migration states, receipts, and rollback behavior.

### 9.5 Explicit Boundary For Memory

This spec intentionally freezes `vida memory ...` as a root family before a dedicated memory-kernel artifact exists.

Rule:

1. the command boundary is frozen now,
2. detailed memory backing behavior remains an open downstream contract,
3. this does not authorize widening the current program into MCP, remote memory, or plugin memory surfaces.

---

## 10. Command-Level Invariants

`1.0` command-tree invariants:

1. exactly five root command families are canonical in the local-binary `1.0` line,
2. `vida task ...` is the sole workflow/task mutation-owning root family,
3. `vida status` is read-only,
4. `vida boot` is the primary startup entry,
5. `vida doctor` owns diagnosis and fail-closed blocker visibility, not hidden bypass behavior,
6. `vida memory ...` owns durable memory access and is distinct from chat recap, logs, and scorecards,
7. command outputs must remain compact and structured enough for human or agent continuation,
8. command behavior must remain allowlisted, explicit, and fail-closed,
9. semantic law outranks shell-era topology,
10. no root family may silently expand into daemon, remote-control-plane, or plugin responsibilities in `1.0`.

---

## 11. Command-Level Non-Goals

This spec does not:

1. freeze exact flag syntax, option spelling, or CLI help text,
2. preserve current slash-command names as product law,
3. preserve shell/Python helper/module boundaries,
4. define exact `SurrealDB` tables, records, or edge layouts,
5. define instruction-store serialization details,
6. define receipt schemas in full,
7. define migration procedures in full,
8. introduce MCP, A2A, A2UI, remote identity, gateways, or remote memory,
9. introduce daemon/watcher/dashboard/plugin responsibilities,
10. start Rust implementation work.

Storage decision note:

1. direct `1.0` storage is canonical embedded `SurrealDB` on `kv-surrealkv`,
2. this command-tree artifact stays above storage layout, but it no longer leaves the product backend ambiguous.

---

## 12. Open Ambiguities

The following remain intentionally open:

1. the exact subcommand names below `vida task ...`,
2. the exact compact output envelope for each root family,
3. the exact operator-visible split between `status` summary and `doctor` deep diagnosis,
4. the exact scope of `memory` query/update/invalidate verbs beyond durable-memory minimums,
5. the precise line between command capsules defined by the instruction kernel and command syntax defined by the command tree,
6. whether a dedicated memory-kernel spec is cut explicitly after the current `0.3` sequence or absorbed elsewhere later.

Ambiguity rule:

1. later specs may refine these details,
2. they must not invalidate the five-root-family decision made here.

---

## 13. Downstream Contracts Unlocked By This Spec

This artifact unlocks:

1. `State Kernel Schema Spec`
   - because state entities, statuses, and mutation law now have a frozen operator home.
2. `Instruction Kernel Spec`
   - because command behavior now has a fixed root-family boundary to bind instruction capsules to.
3. `Migration Kernel Spec`
   - because `boot` and `doctor` are now frozen as the command homes for startup checks and migration visibility.
4. `Route And Receipt Spec`
   - because command families now define where route, approval, verification, and escalation artifacts appear.
5. `Parity And Conformance Spec`
   - because the operator surface now has a stable command-family matrix for fixture generation and conformance tests.

---

## 14. Immediate Next Artifact

The next artifact is:

1. `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`

Reason:

1. command semantics are now frozen at the operator boundary,
2. the next blocker is the authoritative state vocabulary and mutation model that `vida task ...`, `vida status`, `vida boot`, and `vida doctor` must read or enforce.
-----
artifact_path: framework/plans/vida-0.3-command-tree-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-command-tree-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-command-tree-spec.changelog.jsonl
P26-03-09T21: 44:13Z
