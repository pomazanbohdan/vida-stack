# Multi-Orchestrator Session Ownership Claims Contract

Purpose: Define session-scoped ownership, claims, leases, status semantics, and implementation work graph for multiple orchestrators in one project.

## 1. Problem

VIDA currently lets a blocked run in one repository/project root become the effective global runtime blocker for every orchestrator session that opens the same root. That is wrong for multi-session operation. A blocked task owned by session A must not stop session B when session B is working on a different task, a disjoint path set, or an observe-only diagnosis.

The current defect has two root causes:

1. runtime status and continuation selection can consult the latest project run globally rather than the latest run owned by the current orchestrator session or by the same bounded unit,
2. fallback session identity can collapse multiple host sessions in the same worktree into one stable local id when no host-provided session/thread id is available.

The target behavior is: one project root has one shared DB-first truth store, but many session-scoped orchestrator controllers may work in that store at the same time. Shared truth remains global; blocking authority is scoped by session, task, conflict domain, owned path, and explicit global blocker class.

## 2. Identity Model

Runtime ownership must distinguish four identities:

1. `project_id` / `state_root_id` - the shared VIDA project truth store.
2. `worktree_environment_id` - the concrete checkout/environment where filesystem writes can occur.
3. `orchestrator_session_id` - one live controller session in a host tool/thread/terminal.
4. `task_id`, `run_id`, and `lane_id` - the bounded work unit and its execution lane.

`orchestrator_session_id` must be durable for the current host session but must not be derived only from `(project_root, state_dir)`. If `VIDA_ORCHESTRATOR_SESSION_ID`, `CODEX_SESSION_ID`, or `CODEX_THREAD_ID` is unavailable, the runtime must synthesize a per-session token under `.vida/state/sessions` or an equivalent DB-first session registry and record host/process/thread evidence when available.

## 3. Claim And Lease Model

Add an `orchestrator_claim` state record for each active controller claim:

| Field | Meaning |
|---|---|
| `claim_id` | Stable claim id. |
| `project_id` / `state_root_id` | Shared project truth owner. |
| `worktree_environment_id` | Environment in which writes or proofs may occur. |
| `orchestrator_session_id` | Live controller that owns or observes the claim. |
| `task_id`, `run_id`, `lane_id` | Bounded runtime unit, nullable only for session bootstrap or project-level observation. |
| `claim_kind` | `planning`, `dispatch`, `write`, `proof`, `recovery`, or `observe`. |
| `conflict_domain` | Logical resource domain used for scheduling and exclusion. |
| `owned_paths` / `read_only_paths` | Filesystem path scope, normalized to project-relative prefixes. |
| `lease_mode` | `exclusive`, `shared_read`, or `observe`. |
| `status` | `active`, `renewed`, `blocked`, `released`, `expired`, `superseded`, or `reclaimed`. |
| `lease_expires_at` / `last_heartbeat_at` | Liveness evidence. |
| `resource_revision` | Optimistic concurrency version for claim mutation. |

Claim law:

1. A session may hold multiple observe claims, but at most one active write-producing claim per bounded unit unless a runtime packet explicitly permits fanout.
2. A blocked claim blocks only claims that share the same task/run, intersecting `owned_paths`, or the same exclusive `conflict_domain`.
3. A foreign blocked claim is visibility evidence, not a blocker, when the current session owns a disjoint bounded unit and no global blocker class is present.
4. Expired claims are not ignored silently. They must be shown as stale and can be reclaimed only through a recorded reclaim/supersede transition.
5. Global blockers remain global only when they protect shared state integrity: schema migration, state-store lock corruption, DB unreadability, root configuration mutation, or release-wide invariants.

## 4. Runtime Status Semantics

`vida status --json` must split project truth into scoped projections:

1. `current_session` - active session id, worktree id, owned claims, active bounded unit, current blockers, and next lawful command.
2. `project_foreign_runs` - other sessions' active/blocked runs grouped by session and task.
3. `project_foreign_blockers` - blockers that are real but not currently blocking this session.
4. `global_blockers` - blockers that stop every session.
5. `claim_conflicts` - concrete path/conflict-domain intersections that explain why a candidate packet cannot start.

`vida taskflow next`, `consume continue`, `advance`, and continuation binding must admit work by this order:

1. reject if a global blocker exists,
2. reject if the current session has an unresolved open delegated cycle for the same bounded unit,
3. reject if another live session holds an active exclusive claim on the same task/run, intersecting owned paths, or same exclusive conflict domain,
4. reject if the task has explicit `blocks` dependencies not closed,
5. otherwise admit the candidate even when an unrelated foreign session is blocked.

## 5. Protocol Updates

The operating protocols must treat the orchestrator as a session-scoped controller, not as a project-global singleton:

1. Bootstrap creates or resumes one `orchestrator_session_id`.
2. Packet shaping records intended claim kind, conflict domain, path scope, and whether the work is exclusive or parallel-safe.
3. Delegation creates child lane claims tied to the parent session and task.
4. Completion releases or supersedes the claim before another bounded unit is admitted.
5. Re-entry restores the same session when possible; if the host context cannot prove continuity, the runtime must show ambiguity rather than inheriting another session's active task.

## 6. Implementation Work Graph

The epic for this behavior is `feature-multi-orchestrator-session-scoped-ownership-clai`.

Implementation must proceed as:

1. Foundation, sequential: update specs/protocols and canonical task graph.
2. Parallel wave A:
   - session identity and heartbeat model,
   - claim/lease state model and store schema,
   - migration compatibility for ownerless legacy runs.
3. Parallel wave B, after wave A:
   - scoped run/status/continuation queries,
   - operator surface JSON/plain output split,
   - scheduler admission rules using task dependencies plus claim conflicts.
4. Verification, sequential after implementation:
   - two sessions in one repo with disjoint tasks both advance,
   - same task/write scope conflicts fail closed,
   - stale lease reclaim requires explicit receipt,
   - foreign blocked run is visible but nonblocking,
   - global state-store blocker blocks all sessions.

## 7. Proof Targets

Required proof before closure:

1. unit tests for claim compatibility and path/conflict-domain intersection,
2. integration fixture with two synthetic orchestrator sessions in the same project root,
3. status JSON golden output containing `current_session`, `project_foreign_runs`, `project_foreign_blockers`, `global_blockers`, and `claim_conflicts`,
4. regression for legacy global `latest_run_graph_status` no longer blocking unrelated current-session admission,
5. DocFlow readiness for this spec and touched operating protocols.

-----
artifact_path: product/spec/multi-orchestrator-session-ownership-claims-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-15
schema_version: 1
status: canonical
source_path: docs/product/spec/multi-orchestrator-session-ownership-claims-contract.md
created_at: 2026-05-15T09:02:59.3833285Z
updated_at: 2026-05-15T09:13:16.3690981Z
changelog_ref: multi-orchestrator-session-ownership-claims-contract.changelog.jsonl
