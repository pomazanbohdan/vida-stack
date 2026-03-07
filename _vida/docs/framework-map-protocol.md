# Framework Map Protocol (FMP)

Purpose: single canonical map of the VIDA runtime framework boundaries.

This protocol answers three questions:

1. What is runtime core (must stay minimal and clean).
2. What is project artifact space (business/spec/research outputs).
3. How requests move through the framework end-to-end.

## 1) Runtime Core (Canonical)

Runtime core is the minimal set required to run agent workflows:

1. `AGENTS.md` — bootstrap router, lane dispatch, and cross-lane invariants.
2. `_vida/docs/ORCHESTRATOR-ENTRY.MD` — orchestrator entry contract, request intent gate, and TODO engagement gate.
3. `_vida/docs/SUBAGENT-ENTRY.MD` + `_vida/docs/SUBAGENT-THINKING.MD` — worker entry contract and bounded worker reasoning subset.
4. `_vida/commands/*.md` — public command contracts.
5. `_vida/docs/*.md` — operational protocols (routing, command-layer matrix, SCP, IEP, WVP, etc.).
6. `_vida/docs/beads-protocol.md` — cross-flow task-state/execution contract.
7. `_vida/scripts/*.sh` + selected `_vida/scripts/*.py` — executable protocol adapters.
8. `_vida/templates/*` — framework-owned templates for project-owned external artifacts.
9. `_vida/docs/script-runtime-architecture.md` — canonical shell-wrapper vs Python-engine ownership split.
10. `.beads/` + `br` state — SSOT for task lifecycle.

Rule:

1. Runtime core must be clear and single-path.
2. Runtime core uses only canonical command and protocol paths.
3. Do not reintroduce non-canonical topology artifacts.

## 2) Project Artifact Space (Non-Core)

Project artifacts are not runtime orchestration code. They contain delivery content:

1. `docs/` — research, specs, planning, ADR-like decisions, reports.
2. `docs/process/` — project operational runbooks and canonical human-readable command contracts.
3. `scripts/` — executable project build/run/validation/audit entrypoints.
4. `vida.config.yaml` — optional project overlay manifest consumed by VIDA at boot.
5. `docs/research/vida-framework/` — historical project research and migration artifacts.

Rule:

1. Artifact docs may evolve independently.
2. Runtime protocols must only reference artifact locations that are currently canonical.
3. Project build/run/observability guidance must not live in `_vida/docs/`.
4. `_vida/scripts/` is reserved for framework/runtime protocol adapters, not product delivery scripts.
5. Requests that touch both layers must be split by ownership: framework policy changes stay in `AGENTS.md` / `_vida/*`, while project delivery behavior stays in `docs/*` / `scripts/*`.
6. `vida.config.yaml` is project-owned overlay data; framework owns only the schema, validation, and activation semantics.
7. framework-owned starter templates for project artifacts live in `_vida/templates/*`; the instantiated artifacts remain project-owned.

## 3) Layer Map

```text
L0 Policy      : AGENTS.md
L0a Router      : AGENTS.md
L0b Orchestrator: _vida/docs/ORCHESTRATOR-ENTRY.MD
L0c Worker      : _vida/docs/SUBAGENT-ENTRY.MD + _vida/docs/SUBAGENT-THINKING.MD
L1 Routing      : _vida/docs/orchestration-protocol.md + _vida/docs/use-case-packs.md
L2 Command Map  : _vida/commands/*.md + _vida/docs/command-layer-protocol.md
L3 Reasoning    : _vida/docs/thinking-protocol.md + _vida/docs/web-validation-protocol.md
L4 Contracts    : _vida/docs/spec-contract-protocol.md + _vida/docs/form-task-protocol.md
L5 Execution    : _vida/docs/implement-execution-protocol.md + _vida/docs/bug-fix-protocol.md
L6 State/Logs   : _vida/docs/beads-protocol.md + _vida/docs/todo-protocol.md + _vida/docs/log-policy.md
L7 Health       : _vida/docs/pipelines.md + _vida/scripts/quality-health-check.sh
L8 Script Core  : _vida/docs/script-runtime-architecture.md + _vida/scripts/*.sh + selected _vida/scripts/*.py
L9 Bootstrap    : _vida/docs/project-bootstrap-protocol.md + _vida/scripts/project-bootstrap.py
```

State/log-read invariant:

1. State and log inspection is budgeted.
2. Default path is exact-key search against one specific file, then short-window reads.
3. Broad recursive scans of `.vida/logs`, `.vida/state`, and `.beads` are forbidden unless the active lane contract explicitly escalates.

## 4) Request Flow Map

```text
User Request
  -> Bootstrap Router (AGENTS.md)
  -> Lane Selection (orchestrator or worker)
  -> Problem Framing + Lens Selection
  -> Request Intent Classification (answer_only | artifact_flow | execution_flow | mixed)
  -> If answer_only: bounded analysis/synthesis -> User Report
  -> If task/artifact flow: Pack Detection (use-case-packs)
  -> Command Contract (/vida-*)
  -> Command Layer Selection (CL1..CL5)
  -> Protocol Execution (SCP/IEP/BFP/WVP)
  -> Subagent-First Analysis (mode-aware when enabled)
  -> Change-Impact Reconciliation (if drift)
  -> TODO Blocks (block-plan/start/end/reflect/verify)
  -> br State Update (open/in_progress/closed)
  -> Health Check
  -> User Report
```

## 5) Consistency Rules

When changing framework structure, in the same change set:

1. Update this file (`framework-map-protocol.md`).
2. Update `_vida/docs/protocol-index.md` links/domain rows.
3. Update `AGENTS.md` operational references if read-set changed.
4. Update `docs/README.md` and `docs/process/*` if project runbooks moved.
5. Remove outdated references immediately (LEGACY-ZERO).
6. If the same request changes both framework and project scope, verify that each edit landed in its ownership layer before closing the block.
7. Keep the request-intent gate and log-read budget synchronized across bootstrap router, orchestrator entry, worker entry, and orchestration docs.

## 6) Fast Integrity Checks

Use these checks after structural edits:

```bash
! rg -n "_vida/shared/|_vida/reports/|_vida/scratchpad/|/vida-lead|/vida-cascade|/vida-epic|vida-spec-categories|bash scripts/beads-workflow\.sh|bash scripts/quality-health-check\.sh" AGENTS.md _vida/docs _vida/commands -g '!_vida/docs/framework-map-protocol.md'
bash _vida/scripts/vida-command-audit.sh report <task_id>
bash _vida/scripts/quality-health-check.sh --mode quick <task_id>
```

## 7) Decision Boundary

Use this protocol when:

1. deciding whether a file belongs to runtime core or artifact space,
2. refactoring command/protocol topology,
3. resolving duplicate sources of truth.

If conflict appears, precedence order:

1. `AGENTS.md` (L0)
2. `_vida/docs/protocol-index.md`
3. this file (`framework-map-protocol.md`)
4. command-level docs
