# TaskFlow Actualization Protocol

Status: active runtime protocol

Protocol id: `taskflow.actualization`

Canonical surface: `vida taskflow actualize`

Purpose: define the project-neutral VIDA runtime protocol for refreshing a TaskFlow graph from live evidence. The protocol treats TaskFlow as the authoritative work graph, not as a flat task list.

## Scope

This protocol defines how any VIDA project refreshes:

1. task status,
2. owner task selection,
3. dependencies and blockers,
4. parent and child placement,
5. priority,
6. duplicate handling,
7. proof-task placement,
8. execution order,
9. parallel admission metadata,
10. validation and reporting.

This protocol does not define:

1. project-specific task ids,
2. project-specific source paths,
3. framework implementation details,
4. provider-specific execution adapters,
5. product requirements outside the TaskFlow graph.

## Core Rule

Live runtime evidence is authoritative.

Operators must prefer current TaskFlow and runtime surfaces over:

1. stale notes,
2. old reports,
3. cached summaries,
4. branch assumptions,
5. external issue text,
6. non-authoritative exports.

TaskFlow actualization is valid only when every material decision can cite live evidence from runtime surfaces or a bounded operator observation recorded during the session.

## Command Contract

Default command:

```text
vida taskflow actualize --preview --json
```

Apply command:

```text
vida taskflow actualize --apply --json
```

The default mode is preview.

`--apply` is required before any mutation.

`--include-closed` may include closed tasks for audit evidence, but closed-task mutations remain forbidden unless a later protocol version defines and explicitly gates a notes-only closed-audit mutation path.

## Required Read Phase

Every actualization run reads these surfaces or their runtime-equivalent state-store projections:

1. runtime init and current bounded unit,
2. open tasks,
3. in-progress tasks,
4. blocked tasks,
5. ready tasks,
6. critical path,
7. graph validation,
8. dependencies for declared anchor tasks,
9. reverse dependencies for declared anchor tasks,
10. key epic trees,
11. scheduler and parallelism projection.

The read phase must record:

1. which source produced the evidence,
2. whether the source is authoritative,
3. whether the source was fresh enough for mutation decisions,
4. any blocker that prevents safe mutation.

## Required Write Phase

The write phase exists only when `--apply` is set.

A mutating apply run must:

1. create or update one bounded actualization step,
2. mutate only open, in-progress, or blocked tasks by default,
3. keep closed tasks immutable by default,
4. update only fields supported by authoritative TaskFlow mutation surfaces,
5. validate the graph after every coherent mutation batch,
6. stop immediately if graph validation fails,
7. report changed task ids and mutation reasons.

Allowed mutation targets:

1. status,
2. priority,
3. dependency edges,
4. parent edge,
5. labels,
6. notes,
7. proof targets,
8. acceptance targets,
9. execution mode,
10. order bucket,
11. parallel group,
12. conflict domain,
13. planner metadata.

No write phase may mutate a closed task by default.

## Graph Semantics

TaskFlow graph edges have separate meanings:

1. parent-child edges represent ownership and containment,
2. blocks edges represent execution order and readiness,
3. proof dependencies represent verification order,
4. conflict domains represent unsafe co-scheduling surfaces.

Parent-child edges must not be used as hidden execution ordering.

Hard ordering must use dependency or blocker edges.

Parallelism must be explicit. Missing execution semantics fail closed.

## Owner Selection

Every runtime invariant must have exactly one owner task.

Owner selection order:

1. prefer current live runtime evidence,
2. prefer non-closed tasks,
3. prefer in-progress, then blocked, then open tasks,
4. prefer lower numeric priority,
5. prefer higher reverse-dependency count when it reflects real downstream ownership,
6. prefer the task whose parent tree owns the runtime transition,
7. prefer the task with proof targets that match the invariant.

Closed tasks may be cited as historical evidence but must not become mutable owners.

## Duplicate Classification

Every duplicate-looking task must be classified before mutation.

Allowed duplicate classes:

1. `owner`: the one task that owns the invariant.
2. `proof_task`: proves the owner invariant and depends on the owner.
3. `dependent_follow_up`: extends the owner after the owner lands.
4. `stale_duplicate`: repeats superseded intent and should not lead execution.
5. `wrong_parent_item`: belongs under another owner or epic.

Duplicate handling must be non-destructive by default.

Do not close or delete a duplicate during actualization unless a separate closure protocol proves that closure is lawful.

## Universal Pool Model

Actualization orders work into generic runtime pools.

Pool 00: graph hygiene and mutation validity.

Pool 01: foundational registry, config, and runtime-contract normalization.

Pool 02: dispatch and lane materialization.

Pool 03: continuation, packet lineage, and next-lawful action generation.

Pool 04: delegated-cycle and activation-view convergence.

Pool 05: host-bridge provenance, artifacts, and completion output.

Pool 06: session-aware scheduling and parallel admission.

Pool 07: provider and mode readiness.

Pool 08: proof suites and persisted fixtures.

Pool 09: cleanup, legacy reduction, and optional hardening.

Later pools depend on earlier pools when the earlier pool owns a runtime invariant that the later pool consumes.

Cleanup and optional hardening must not block critical runtime flow unless live evidence shows they are required for graph validity or execution safety.

## Priority Law

Priority must reflect runtime order, not operator preference alone.

Default priority order:

1. graph invalidity,
2. mutation validity,
3. foundational runtime contract,
4. dispatch and lane materialization,
5. continuation and lineage,
6. activation convergence,
7. provenance and artifact completion,
8. session-aware parallel admission,
9. provider readiness,
10. proof suites,
11. cleanup and hardening.

If a downstream defect blocks an upstream proof, it moves into the earliest pool that explains the blocker.

## Parallelism Law

Parallel execution is opt-in.

A task is not parallel-safe unless all required execution semantics are explicit:

1. `execution_mode`,
2. `order_bucket`,
3. `parallel_group`,
4. `conflict_domain`.

Missing or incompatible semantics block parallel admission.

Tasks may share a parallel group only when their conflict domains and owned runtime transitions are disjoint.

## Mutation Batch Law

A mutation batch is coherent when all mutations share one reason and one validation boundary.

Examples:

1. normalize missing execution semantics for one pool,
2. add blocker edges from a foundation owner to downstream proof tasks,
3. rehome wrong-parent items under the owner proven by a tree inspection,
4. add proof labels and proof targets to proof tasks for one invariant.

After each coherent batch:

```text
vida task validate-graph --json
```

The batch is rejected if validation fails.

## Fail-Closed Conditions

Actualization must fail closed when:

1. the authoritative state store is unavailable,
2. graph validation fails,
3. the active bounded unit is ambiguous and mutation depends on it,
4. anchor task identity is missing,
5. key epic tree inspection fails for a required rehome decision,
6. duplicate owner selection is ambiguous,
7. closed-task mutation would be required,
8. parallelism metadata is missing for a parallel admission decision,
9. write scope is not bounded,
10. a mutation surface cannot prove the changed graph.

Fail-closed output must include blocker codes and next actions.

## Report Contract

Every actualization run reports:

1. ordered pools,
2. changed tasks,
3. skipped closed tasks,
4. duplicate decisions,
5. blocked mutations,
6. validation proof,
7. remaining caveats,
8. next lawful TaskFlow command.

Preview runs report proposed semantics and blockers.

Apply runs report exact changed task ids and graph validation after mutation.

## Acceptance

An actualization run is accepted when:

1. `vida task validate-graph --json` passes,
2. no closed task mutates by default,
3. every changed task has a reason and evidence source,
4. every runtime transition has one owner task,
5. duplicate handling is explicit and non-destructive,
6. critical path begins with current foundation or blocker work,
7. ready set matches intended next lawful work,
8. blocked set explains downstream pools,
9. parallelism decisions fail closed on missing semantics.

## Test Matrix

The runtime implementation must cover:

1. duplicate classification,
2. owner selection,
3. pool ordering,
4. semantics validation,
5. preview JSON,
6. apply JSON,
7. closed-task immutability,
8. invalid graph fail-closed output,
9. conflicting mode rejection,
10. parallelism fail-closed behavior.

## Operator Checklist

1. Run runtime init.
2. Capture active bounded unit and posture.
3. Run actualize preview.
4. Inspect graph validation, critical path, ready set, blocked set, anchors, and epic trees.
5. Pick owner tasks for runtime invariants.
6. Classify duplicates.
7. Add blocker edges for hard ordering.
8. Normalize priority and execution semantics.
9. Rehome only with live tree evidence.
10. Apply one coherent mutation batch.
11. Validate graph.
12. Repeat until the graph reports a coherent next lawful path.
13. Report changed tasks, skipped closed tasks, duplicate decisions, blockers, and validation proof.
