# Status Families And Query Surface Model

Status: active product law

Purpose: define status as a family of bounded query/render surfaces rather than as one ambiguous global output, and establish the minimum Release-1 operator status families for the `VIDA` shell.

## 1. Core Rule

`status` is not one command that means everything.

Instead:

1. each major subsystem exposes its own bounded status family,
2. each status family has its own source of truth,
3. rendered operator/model summaries must be built from those bounded families.

## 2. Query-First Rule

Status rendering must follow this order:

1. query the bounded truth surface,
2. assemble the required state,
3. render the result for CLI or model output,
4. preserve the ability to inspect the underlying queried state directly.

That means:

1. model narration does not replace query surfaces,
2. rendered summaries must remain grounded in explicit state retrieval.

Operator query routing rule:

1. when a user asks for execution-preparation artifact readiness, route to `vida taskflow artifacts list --json` or `vida taskflow artifacts show <artifact-id> --json`,
2. when a user asks for routing/model-selection config actuation, route to `vida taskflow config-actuation census --json`,
3. generic task inspection must not swallow these more specific artifact and config-actuation intents.

## 3. Release-1 Must-Have Families

Release 1 must support at least:

1. `orchestrator status`
   - active session/runtime posture, loaded bundle family, active mode
2. `doctor`
   - health, integrity, and bounded diagnostics
3. `config status`
   - active project activation/configuration posture
4. `flow status`
   - active runtime flows and execution posture
5. `tasks status`
   - active tasks, counts, distribution, and bounded task-state view
6. `sync status`
   - DB/filesystem synchronization and reconcile posture
7. `memory status`
   - command-time memory snapshot only for Release 1
8. `execution-preparation artifact status`
   - required/missing/materialized artifact truth from the runtime-consumption snapshot
9. `config actuation status`
   - routing/model-selection config keys mapped to validators, runtime consumers, and proof posture

## 4. Family-Specific Meaning

### 4.1 Orchestrator Status

Shows:

1. which bundle/init posture is active,
2. which runtime mode the shell is in,
3. which high-level execution context is currently loaded.

### 4.2 Doctor

Shows:

1. integrity and consistency diagnostics,
2. validation failures,
3. bounded system/runtime health,
4. what needs repair before lawful continuation.

### 4.3 Config Status

Shows:

1. active roles,
2. active skills,
3. active profiles,
4. active flows,
5. active teams,
6. model/backend policy,
7. activation mode,
8. protocol registration/promotion posture.

### 4.4 Flow Status

Shows:

1. active flows,
2. blocked or escalated counts,
3. bounded execution progression,
4. current flow-level posture.

### 4.5 Tasks Status

Shows:

1. active tasks,
2. counts by state,
3. current task work distribution,
4. bounded execution/task overview.

### 4.6 Sync Status

Shows:

1. DB/filesystem sync posture,
2. pending or blocked reconcile work,
3. restore/import/export state when applicable.

### 4.7 Memory Status

Release-1 boundary:

1. this is a command-time snapshot,
2. it may show the currently available memory/index state,
3. it must not assume an always-on daemon or background freshness engine.

## 5. Rendering Rule

If a user asks for status conversationally:

1. the runtime should resolve the right status family first,
2. query it,
3. then render the answer.

Default rule:

1. no monolithic catch-all `status` should hide which family was actually queried.

## 6. Filesystem Projection

Where the current implementation can materialize status safely:

1. filesystem projection may mirror bounded status state,
2. but the status family still belongs to the operational runtime truth model,
3. mirrored files must not become a second truth source.

## 7. Boundary Rule

1. status families are operator/query surfaces,
2. they are not the owner of the runtime entities they report,
3. doctor is one status family rather than a separate non-status universe,
4. deeper observability surfaces such as full run history, routing decisions, and compliance trail remain later discussion unless explicitly promoted.

## 8. Completion Proof

This model is closed enough for Release 1 when:

1. each required family has a bounded query path,
2. CLI can render them,
3. model-facing output is grounded in those same query paths,
4. no family requires a later always-on daemon to exist,
5. operators can distinguish between status families rather than receiving one ambiguous aggregate answer.

-----
artifact_path: product/spec/status-families-and-query-surface-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/status-families-and-query-surface-model.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: 2026-04-26T14:58:34.964609994Z
changelog_ref: status-families-and-query-surface-model.changelog.jsonl
