# Clarify Enforce Immediate Project Agent First Design

Status: implemented

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: Clarify and enforce that project agent-first development means TaskFlow/VIDA delegated lanes through `vida agent-init`, not ad hoc root-session coding or host-tool-specific subagent APIs.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | project activation`
- Status: implemented

## Current Context
- Existing system overview
  - Project canon already says normal write-producing work is delegation-first and root-session local writing requires an explicit exception path.
  - Runtime dispatch already materializes worker lanes through `vida agent-init --dispatch-packet ... --json`.
  - Host CLI runtimes such as Codex/Qwen/Kilo/OpenCode are modeled as carrier systems and execution backends under `vida.config.yaml`.
- Key components and relationships
  - `AGENTS.md` and `AGENTS.sidecar.md` bootstrap the session and point into project process law.
  - `docs/process/*` owner docs define delegation-first, packet law, and local-write exception rules.
  - `crates/vida/src/init_surfaces.rs` renders scaffolded guidance docs and init guidance shown to operators.
  - `crates/vida/src/main.rs` renders runtime orchestration contracts and packet prompts used for delegated lanes.
- Current pain point or gap
  - The project law is explicit about delegation-first, but some guidance still describes “agentic” or “delegated Codex team” behavior too generically.
  - That leaves room to misread project agent mode as host-level `spawn_agent` semantics instead of the canonical `vida agent-init` lane flow.
  - The startup/runtime guidance does not state strongly enough that host subagent APIs are optional carrier/executor details, not the canonical meaning of project-agent-first development.
  - The repository overlay still points the active host CLI selection at `qwen` external mode even though the active project canon and rendered runtime guidance treat the Codex/internal path as the primary agent-first execution posture.
  - `vida status --json` still treats enabled external CLI subagents as if they make the whole session externally dependent, even when the selected host execution class is internal and the external subagents are only optional carrier details.
  - Runtime-consumption status and continuation also allowed one live ambiguity: a newer `bundle-check` snapshot could overshadow the latest valid `final` snapshot and re-open release-admission blockers even after lawful `consume final` evidence already existed.
  - Runtime dispatch packets still rely on prose/template alignment rather than one compiled template-specific packet-minimum validator, so `vida taskflow consume final`, persisted dispatch packets, resume, and `vida agent-init` can drift if packet-family requirements are not enforced from shared code.

## Goal
- What this change should achieve
  - Make the first correct interpretation obvious: for normal write-producing work, root sessions shape and dispatch through `vida agent-init` lanes.
  - State that host executor subagent APIs are optional backend details and do not replace the VIDA/TaskFlow delegated lane contract.
  - Surface the distinction both in canonical docs and in runtime-generated prompts/instructions.
- What success looks like
  - Bootstrap and process docs explicitly distinguish project delegated lanes from host-local subagent APIs.
  - Runtime packet prompts remind worker lanes that project agent-first execution is `vida agent-init`-backed and root-session local coding is forbidden without an exception path.
  - Runtime dispatch and downstream packets fail closed when the active `packet_template_kind` is missing its canonical mandatory fields.
  - Targeted tests stay green after the wording and contract updates.
- What is explicitly out of scope
  - Changing the carrier-selection model.
  - Replacing `vida agent-init` with a new runtime surface.
  - Reworking the entire project team topology or packet schema.

## Requirements

### Functional Requirements
- Must explicitly define project agent-first execution as TaskFlow/VIDA delegated lane execution through `vida agent-init`.
- Must explicitly state that host runtime subagent APIs are backend/carrier implementation details, not the canonical development control surface.
- Must keep root-session write prohibition and exception-path law intact.
- Must update both live bootstrap docs and scaffold sources when bootstrap carrier wording changes.
- Must update runtime-generated operator/prompt surfaces so the distinction is visible during actual delegated execution.
- Must ensure release-admission and continuation gates prefer the newest valid `final` runtime-consumption snapshot over newer non-final helper snapshots such as `bundle-check`.
- Must enforce one shared template-specific packet validator for persisted/runtime dispatch packets so packet-ready handoff cannot bypass the canonical project packet minimum.

### Non-Functional Requirements
- Performance
  - No meaningful runtime overhead.
- Scalability
  - Guidance must remain backend-neutral across Codex/Qwen/Kilo/OpenCode carrier systems.
- Observability
  - Runtime prompts and orchestration contracts should expose the clarified execution surface without requiring external interpretation.
- Security
  - Clarification must reinforce fail-closed local-write behavior rather than weakening it.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `vida.config.yaml`
  - `AGENTS.md`
  - `AGENTS.sidecar.md`
  - `install/assets/AGENTS.scaffold.md`
  - `docs/process/decisions.md`
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/project-packet-and-lane-runtime-capsule.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
  - `docs/process/agent-system.md`
  - `docs/process/codex-agent-configuration-guide.md`
- Framework protocols affected:
  - none beyond bootstrap-carried wording sync requirements
- Runtime families affected:
  - `taskflow`
  - `project activation`
- Config / receipts / runtime surfaces affected:
  - delegated orchestration contract JSON emitted from `crates/vida/src/main.rs`
  - packet prompt text emitted for `vida agent-init` lanes
  - persisted dispatch packet validation for `vida agent-init` and `vida taskflow consume continue`
  - scaffolded process docs emitted from `crates/vida/src/init_surfaces.rs`

## Design Decisions

### 1. Project agent-first will be defined in terms of `vida agent-init` lane dispatch
Will implement / choose:
- Add explicit wording to bootstrap and owner process docs that project-normal write-producing work routes through `vida agent-init` delegated lanes once a lawful packet exists.
- Why
  - This is already the canonical runtime surface and the main ambiguity is interpretive, not architectural.
- Trade-offs
  - Repeats the same clarification across several docs, but that is preferable to leaving bootstrap and owner docs misaligned.
- Alternatives considered
  - Clarify only in one doc.
  - Rejected because startup/bootstrap readers may stop earlier than the deepest owner protocol.
- ADR link if this must become a durable decision record
  - none

### 2. Host subagent APIs will be described as optional carrier implementation details
Will implement / choose:
- Add wording to process docs and runtime prompt/orchestration-contract surfaces that host `spawn_agent`-style APIs are not themselves the canonical project execution contract.
- Why
  - Prevents root sessions from treating external host-tool affordances as the primary gate for whether delegated project development is lawful.
- Trade-offs
  - Slightly more explicit runtime prose in prompts and generated docs.
- Alternatives considered
  - Leave host/executor semantics implicit.
  - Rejected because the observed failure mode came from that ambiguity.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - Bootstrap carrier docs and sidecar map.
  - Project process docs for orchestrator, packet/lane, team topology, and host agent system.
  - Runtime-generated init/scaffold docs and delegated packet prompt text.
  - Shared dispatch-packet template validator used by packet writing and packet reading surfaces.
- Key interfaces
  - `vida orchestrator-init`
  - `vida agent-init`
  - runtime orchestration contract JSON
- Bounded responsibilities
  - Docs define the canonical interpretation.
  - Launcher/runtime prompts restate it at execution time.

### Data / State Model
- Important entities
  - delegated lane
  - root-session write guard
  - host carrier/runtime backend
- Receipts / runtime state / config fields
  - `dispatch_surface`
  - `dispatch_command`
  - `carrier_runtime`
  - `runtime_assignment`
  - root-session exception-path requirements
- Migration or compatibility notes
  - No store/schema migration is required.
  - Resume/read paths must stay backward-compatible with older persisted runtime packets that predate the current packet-minimum contract.
  - `vida taskflow consume continue` may normalize legacy persisted dispatch/downstream packets by backfilling the canonical runtime `read_only_paths` set before fail-closed validation runs.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - orchestrator -> worker/coach/verifier lane dispatch through `vida agent-init`
- Cross-document / cross-protocol dependencies
  - bootstrap carrier sync rule between `AGENTS.md` and `install/assets/AGENTS.scaffold.md`

### Bounded File Set
- `AGENTS.md`
- `AGENTS.sidecar.md`
- `install/assets/AGENTS.scaffold.md`
- `vida.config.yaml`
- `docs/process/decisions.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/project-packet-and-lane-runtime-capsule.md`
- `docs/process/project-development-packet-template-protocol.md`
- `docs/process/team-development-and-orchestration-protocol.md`
- `docs/process/agent-system.md`
- `docs/process/codex-agent-configuration-guide.md`
- `docs/process/project-operations.md`
- `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/main.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/src/taskflow_task_bridge.rs`
- `crates/vida/tests/boot_smoke.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No local root-session implementation as the default response to a normal write-producing task.
  - No reinterpretation of host subagent availability as the canonical legality gate for project delegated work.
- Required receipts / proofs / gates
  - root-session local write still requires explicit exception-path evidence.
  - delegated packet execution remains tied to `vida agent-init`/TaskFlow lane receipts.
- Safety boundaries that must remain true during rollout
  - carrier/runtime neutrality remains intact.
  - delegation-first and coach/verifier separation remain intact.

## Implementation Plan

### Phase 1
- Update the design doc with the bounded file set and proof targets.
- Update bootstrap and owner process docs to define project agent-first execution unambiguously.
- First proof target
  - `vida docflow check --root . docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`

### Phase 2
- Update generated init/scaffold guidance and runtime packet/orchestration prompts.
- Add shared packet-template validation for dispatch packet persistence, resume, and `vida agent-init`.
- Add or update tests that pin the clarified wording where runtime emits it.
- Second proof target
  - targeted `cargo test -p vida ...` coverage for boot/runtime guidance surfaces

### Phase 3
- Run release build.
- Refresh the installed/system binary from the fresh release artifact.
- Commit and push the bounded change set.
- Final proof target
  - release build plus targeted runtime smoke/contract checks

## Validation / Proof
- Unit tests:
  - `cargo test -p vida boot_smoke -- --nocapture`
- Integration tests:
  - targeted `vida` runtime/boot smoke assertions already hosted in `crates/vida/tests/boot_smoke.rs`
- Runtime checks:
  - `vida orchestrator-init`
  - `vida status --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
  - `cargo test -p vida release1_contracts -- --nocapture`
  - `cargo test -p vida boot_smoke -- --nocapture`
  - `cargo build --release -p vida`

## Observability
- Logging points
  - none added
- Metrics / counters
  - none added
- Receipts / runtime state written
  - TaskFlow spec-bootstrap receipts
  - normal build/test receipts through existing tooling

## Rollout Strategy
- Development rollout
  - update docs and runtime prompt surfaces in one bounded change
- Migration / compatibility notes
  - no data migration
  - legacy persisted runtime packets remain admissible through bounded resume-path normalization rather than operator-side manual packet repair
- Operator or user restart / restart-notice requirements
  - reinstall/update the system `vida` binary after the release build so future sessions read the corrected runtime surfaces immediately

## Future Considerations
- Follow-up ideas
  - add a dedicated runtime field that names the canonical delegated execution surface explicitly in JSON, not only prompt prose
- Known limitations
  - host-tool system prompts outside this repository may still impose their own subagent policies; the project fix can only make the repository/runtime contract explicit
- Technical debt left intentionally
  - no broader redesign of carrier/runtime-neutral terminology in this patch

## References
- Related specs
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/release-1-capability-matrix.md`
- Related protocols
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/project-packet-and-lane-runtime-capsule.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/clarify-enforce-immediate-project-agent-first-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-04
schema_version: 1
status: canonical
source_path: docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md
created_at: 2026-04-04T17:55:40.339798941Z
updated_at: 2026-04-04T18:01:04.58943557Z
changelog_ref: clarify-enforce-immediate-project-agent-first-design.changelog.jsonl
