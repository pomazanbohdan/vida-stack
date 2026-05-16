# Spec Protocol Foundation Session Scoped Orchestr Design

Purpose:
Define the bounded product/runtime protocol foundation for multiple orchestrator
sessions that may operate in the same VIDA project root without stealing each
other's active continuation, lane ownership, or write authority.

Status: `approved`

Problem:
The current TaskFlow and lane surfaces can expose an active run, delegated lane,
or recovery continuation without enough session ownership evidence to prove which
orchestrator is allowed to resume it. In a multi-orchestrator project this makes
three states ambiguous: an open delegated cycle, a stale or competing root
session, and a scoped continuation that belongs to a different orchestrator. The
runtime must fail closed when ownership is unclear, but it also needs a compact
protocol for claiming, inspecting, superseding, and resuming work without
creating duplicate epics or bypassing delegated execution law.

Bounded Scope:
- TaskFlow run and continuation ownership metadata for orchestrator session ids,
  host thread identity, and worktree/environment identity.
- Lane claim and supersession rules for delegated lanes, exception takeovers, and
  recovery continuations.
- Status and diagnostic projections that distinguish stale ownership,
  competing ownership, admissible claim, and active claim.
- Protocol text and proof targets needed before implementation slices mutate
  TaskFlow, DocFlow, lane, or status surfaces.

Protocol Requirements:
- Every active run, continuation binding, delegated lane, and exception takeover
  must expose the owning orchestrator session when that evidence is available.
- A root orchestrator may resume a bounded unit only when it owns the active
  claim, has an admissible reclaim path, or records an explicit active exception
  takeover for the same bounded unit and owned write scope.
- Activation/view-only handoffs, stale lane handles, and carrier timeouts remain
  blockers until they are completed, superseded, or reclaimed with explicit
  receipt evidence.
- Parallel work is allowed only when each session has disjoint bounded units and
  disjoint write scopes; otherwise the runtime posture is sequential.
- User-facing status must report the compact reason a continuation is blocked:
  missing owner evidence, stale owner, competing active owner, open delegated
  cycle, or recovery not ready.

Expected Behavior:
- `vida orchestrator-init --json` and `vida status --json` surface the active
  bounded unit, owner evidence, and sequential-versus-parallel posture for the
  current root session.
- `vida task next --json` does not silently hand out a continuation owned by a
  different active orchestrator; it reports the owner conflict and the lawful
  reclaim or wait path.
- `vida lane show <run-id> --json` distinguishes receipt-recorded,
  admissible-not-active, and active exception takeover states.
- `vida agent-init --dispatch-packet --execute-dispatch --json` remains the
  normal write-producing route and refuses execution when the run-graph recovery
  gate is false.
- Runtime self-diagnostic classifies contradictory ownership projections as VIDA
  runtime issue candidates instead of recommending impossible operator commands.

Proof Targets:
- Design review verifies the metadata fields and state names are sufficient for
  TaskFlow, lane, status, and self-diagnostic surfaces.
- Follow-on implementation slices add focused tests for:
  session-owned continuation visibility, competing-owner fail-closed behavior,
  stale-owner reclaim diagnostics, and exception takeover state projection.
- Runtime command checks:
  `vida orchestrator-init --json`
  `vida task next --json`
  `vida lane show <run-id> --json`
  `vida taskflow consume continue --run-id <run-id> --json`

Non-goals:
- Implementing the full multi-orchestrator storage migration in this spec slice.
- Changing carrier scoring, model selection, or backend routing policy.
- Allowing root-session local implementation without delegated receipt evidence
  or active exception takeover.
- Creating a general distributed lock service outside the VIDA runtime state
  store.

Follow-on Slices:
- Add session claim metadata to TaskFlow continuation and run-graph records.
- Project ownership and stale-owner diagnostics through orchestrator, task, lane,
  and status surfaces.
- Add reclaim/supersession tests for delegated lane blockers and exception
  takeover states.

-----
artifact_path: product/spec/spec-protocol-foundation-session-scoped-orchestr-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-16
schema_version: 1
status: canonical
source_path: docs/product/spec/spec-protocol-foundation-session-scoped-orchestr-design.md
created_at: 2026-05-16T19:46:09.3307329Z
updated_at: 2026-05-16T19:46:09.3307329Z
changelog_ref: spec-protocol-foundation-session-scoped-orchestr-design.changelog.jsonl
