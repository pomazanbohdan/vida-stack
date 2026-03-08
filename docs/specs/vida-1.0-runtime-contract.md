# Vida 1.0 Runtime Contract

Purpose: define the runtime model for the first binary release, including command surface, persistence model, instruction runtime, and upgrade behavior.

## Command Surface

The required command families are:

1. `vida boot`
2. `vida task ...`
3. `vida memory ...`
4. `vida status`
5. `vida doctor`

## Command Expectations

### `vida boot`

`vida boot` must:

1. open the embedded backend,
2. validate runtime and migration state,
3. restore current operating context,
4. present active task, next ready path, blockers, and startup guidance,
5. return compact structured output suitable for operator and agent use.

### `vida task ...`

The required task surface is:

1. `vida task next`
2. `vida task list`
3. `vida task show`
4. `vida task start`
5. `vida task close`
6. `vida task block`
7. `vida task ready`

`vida task next` is the most important command in the surface.

It should:

1. validate whether the current step can close,
2. inspect blockers and dependencies,
3. select the next eligible task,
4. transition it into active work when allowed,
5. return a compact report explaining what advanced or what prevented advancement.

### `vida memory ...`

The required memory surface is:

1. `vida memory add`
2. `vida memory list`
3. `vida memory search`
4. `vida memory status`

This surface should cover:

1. framework memory,
2. project memory,
3. lessons,
4. anomalies,
5. corrections,
6. operational context records.

### `vida status`

`vida status` should summarize:

1. active task,
2. ready queue head,
3. blockers,
4. approval state,
5. memory and lifecycle summaries,
6. operator-relevant runtime health.

### `vida doctor`

`vida doctor` should validate:

1. backend accessibility,
2. schema and migration integrity,
3. instruction compatibility,
4. invalid task state,
5. broken approval or lifecycle state,
6. inconsistent ownership or runtime law surfaces.

## Output Contract

Core operator flows should default to compact structured output.

TOON-native output is required for:

1. `vida boot`,
2. `vida task next`,
3. `vida status`,
4. `vida doctor`.

## Runtime Kernels

### State Kernel

The state kernel owns:

1. tasks,
2. blockers,
3. dependencies,
4. execution receipts,
5. approvals,
6. operator status.

### Memory Kernel

The memory kernel owns:

1. framework memory,
2. project memory,
3. lessons,
4. anomalies,
5. corrections,
6. operational carry-over records.

### Instruction Kernel

The instruction kernel owns:

1. core instructions,
2. project overlays,
3. user overlays,
4. instruction parts,
5. instruction capsules,
6. effective command composition.

Instruction rules:

1. core instructions are framework-owned,
2. project and user layers may extend the runtime,
3. overlays may not weaken framework invariants, route law, review law, or approval law,
4. command behavior is assembled from ordered instruction parts or capsules rather than one monolithic boot document.

### Migration Kernel

The migration kernel owns:

1. schema versioning,
2. instruction versioning,
3. startup migration execution,
4. compatibility validation,
5. migration receipts.

## Persistence Model

The embedded backend should hold the primary runtime truth for:

1. state,
2. memory,
3. instruction runtime,
4. lifecycle and approval records,
5. migration history.

The repository should stop being the primary startup container of framework behavior.

## Retrieval Model For 1.0

The required retrieval model for `1.0` is:

1. exact lookup,
2. metadata filtering,
3. explicit links between related runtime records.

Vector-search daemon integration is intentionally deferred.

## Upgrade Contract

A new binary release must:

1. detect runtime version mismatch,
2. validate whether migration is required,
3. run schema migration,
4. run instruction migration,
5. validate overlay compatibility,
6. record migration receipts before normal operation continues.

If the runtime cannot prove compatibility, startup should fail closed instead of silently drifting.
