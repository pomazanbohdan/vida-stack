# Framework Map Protocol (FMP)

Purpose: single canonical map of the VIDA runtime framework boundaries.

This protocol answers three questions:

1. What is runtime core (must stay minimal and clean).
2. What is project artifact space (business/spec/research outputs).
3. How requests move through the framework end-to-end.

## 1) Runtime Core (Canonical)

Runtime core is the minimal set required to run agent workflows:

1. `AGENTS.md` — bootstrap router, lane dispatch, and cross-lane invariants.
2. `docs/framework/ORCHESTRATOR-ENTRY.MD` — orchestrator entry contract, request intent gate, and TODO engagement gate.
3. `docs/framework/SUBAGENT-ENTRY.MD` + `docs/framework/SUBAGENT-THINKING.MD` — worker entry contract and bounded worker reasoning subset.
4. `docs/framework/*.md` — operational protocols (routing, command-layer matrix, SCP, IEP, WVP, etc.).
5. `docs/framework/beads-protocol.md` — cross-flow task-state/execution contract.
6. `docs/framework/runtime-transition-map.md` — canonical map from retired `docs/framework/history/_vida-source/scripts/*` surfaces into `vida-v0` or historical-only status.
7. `vida-v0/*` — transitional executable runtime package for the `0.2.0` rollback architecture line.
8. `docs/framework/templates/*` — current framework-owned reference templates.
9. `docs/framework/script-runtime-architecture.md` — canonical runtime transition and ownership split.
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
3. Project build/run/observability guidance must not live in `docs/framework/`.
4. `docs/framework/history/_vida-source/*` is historical migration/source material during cutover, not the target active home.
5. Requests that touch both layers must be split by ownership: framework policy changes stay in `AGENTS.md` / `docs/framework/*` / `vida-v0/*`, while project delivery behavior stays in `docs/*` / `scripts/*`.
6. `vida.config.yaml` is project-owned overlay data; framework owns only the schema, validation, and activation semantics.
7. framework-owned starter templates live in `docs/framework/templates/*`; the instantiated artifacts remain project-owned.

## 3) Layer Map

```text
L0 Policy      : AGENTS.md
L0a Router      : AGENTS.md
L0b Orchestrator: docs/framework/ORCHESTRATOR-ENTRY.MD
L0c Worker      : docs/framework/SUBAGENT-ENTRY.MD + docs/framework/SUBAGENT-THINKING.MD
L1 Routing      : docs/framework/orchestration-protocol.md + docs/framework/use-case-packs.md
L2 Command Map  : docs/framework/command-layer-protocol.md + docs/framework/runtime-transition-map.md
L3 Reasoning    : docs/framework/thinking-protocol.md + docs/framework/web-validation-protocol.md
                  support refs: docs/framework/algorithms-one-screen.md, docs/framework/algorithms-quick-reference.md
L4 Contracts    : docs/framework/spec-contract-protocol.md + docs/framework/form-task-protocol.md
L5 Execution    : docs/framework/implement-execution-protocol.md + docs/framework/bug-fix-protocol.md
L6 State/Logs   : docs/framework/beads-protocol.md + docs/framework/todo-protocol.md + docs/framework/log-policy.md
L7 Health       : docs/framework/pipelines.md + docs/framework/runtime-transition-map.md
L8 Runtime Core : docs/framework/script-runtime-architecture.md + vida-v0/*
L9 Bootstrap    : docs/framework/project-bootstrap-protocol.md + vida-v0/*
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
2. Update `docs/framework/protocol-index.md` links/domain rows.
3. Update `AGENTS.md` operational references if read-set changed.
4. Update `docs/README.md` and `docs/process/*` if project runbooks moved.
5. Remove outdated references immediately (LEGACY-ZERO).
6. If the same request changes both framework and project scope, verify that each edit landed in its ownership layer before closing the block.
7. Keep the request-intent gate and log-read budget synchronized across bootstrap router, orchestrator entry, worker entry, and orchestration docs.

## 6) Fast Integrity Checks

Use these checks after structural edits:

```bash
! rg -n "docs/framework/history/_vida-source/shared/|docs/framework/history/_vida-source/reports/|docs/framework/history/_vida-source/scratchpad/|/vida-lead|/vida-cascade|/vida-epic|vida-spec-categories|bash scripts/beads-workflow\.sh|bash scripts/quality-health-check\.sh" AGENTS.md docs/framework docs/product -g '!docs/framework/framework-map-protocol.md'
nim c vida-v0/src/vida.nim
nim c -r vida-v0/tests/test_kernel_runtime.nim
```

## 7) Decision Boundary

Use this protocol when:

1. deciding whether a file belongs to runtime core or artifact space,
2. refactoring command/protocol topology,
3. resolving duplicate sources of truth.

If conflict appears, precedence order:

1. `AGENTS.md` (L0)
2. `docs/framework/protocol-index.md`
3. this file (`framework-map-protocol.md`)
4. command-level docs
