# /vida-spec — Final Specification Engine (SCP-Based)

Purpose: produce implementation-ready specs that are aligned with user intent, real external contracts, and project architecture.

Primary protocol: `spec-contract-protocol.md` (SCP).

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> spec brief, scope target, and required artifact intake.
2. `CL2 Reality And Inputs` -> discovery plus external/API reality validation through SCP.
3. `CL3 Contract And Decisions` -> conflict resolution plus design/technical contract decisions.
4. `CL4 Materialization` -> spec artifact production, skills routing, and confidence scoring.
5. `CL5 Gates And Handoff` -> reassessment, ready verdict, and approved-contract handoff to `/vida-form-task`.

Canonical source: `command-layer-protocol.md`

Handoff boundary:

1. `/vida-spec` owns the approved product/technical contract.
2. Task-pool creation starts in `/vida-form-task`.
3. Spec-ready state may auto-enter downstream implementation flow only through the execution-entry gates defined by autonomous-execution and validation-report law; there is no separate live project overlay toggle for that behavior.

## Position in Current Engine

1. `/vida-research` = BA external discovery.
2. `/vida-spec` = SA contract formation (including feature->spec coverage checks).
3. `/vida-form-task` = task/pool formation from approved contract.
4. `/vida-implement*` = development execution (outside SCP scope).

## Runtime Invariants

1. Task state SSOT: `vida taskflow task` + TaskFlow execution logs only.
2. No legacy `state/**` and no separate sync-command behavior.
3. No hidden side-effects outside TaskFlow workflow.
4. Do not finalize spec without explicit reassessment confirmation.

## Mandatory Inputs (Read Before Any Spec Output)

1. `docs/feature-checklist.md`
2. `docs/research/*` relevant files
3. `docs/specs/README.md` + related `docs/specs/**`
4. `docs/decisions.md`
5. `spec-contract-protocol.md`
6. `web-validation-protocol.md`
7. `runtime.task-state-telemetry-protocol.md` + `runtime-instructions/work.taskflow-protocol`

## Commands

### `/vida-spec status`

Read-only overview of specs:

1. DRAFT
2. NEEDS_REVIEW
3. STABLE

### `/vida-spec create <scope>`

Run full SCP contract flow:

1. `SCP-0 Intake`: spec brief (goal, in/out scope, constraints).
2. `SCP-1 Discovery`: structured user questions with options.
3. `SCP-2 Conflict`: resolve incompatible decisions.
4. `SCP-3 Reality`: live API/docs validation where applicable.
5. `SCP-4 Design Contract`: UX flow, states, components.
6. `SCP-5 Technical Contract`: interfaces, data/error contracts, observability.
7. `SCP-6 Skill Routing`: dynamic skill discovery + optional project skill scaffold.
8. `SCP-6.5 Draft Execution-Spec`: compact bounded execution contract for downstream review.
9. `Confidence`: calculate weighted SCP confidence.
10. `SCP-7 Reassessment`: consolidated confirmation with user.
11. `SCP-8 Ready Verdict`: ready/conditional/not-ready.
12. Persist spec and update the DB-backed task/docs anchor when needed.

### Internal Category Decomposition (Built-in)

Category decomposition is an internal step of `/vida-spec`.

1. Do not use a separate command for category splitting.
2. When scope is broad, produce category sections directly in this flow.
3. Default buckets:
   - architecture,
   - ui,
   - integration,
   - security,
   - testing.
4. Merge category outputs directly into canonical `docs/specs/**` artifacts.
5. Keep one contract surface: `/vida-spec` owns category decomposition and final spec output.

### `/vida-spec review <scope>`

Consistency and drift check against:

1. feature checklist,
2. research evidence,
3. decisions,
4. external API reality evidence,
5. design/technical contracts.

Output: conflicts, missing constraints, risks, verdict.

Absorbed cascade behavior:

1. if drift is confirmed, `/vida-spec review` becomes the contract re-baseline step,
2. next required step is `/vida-form-task` to reconcile task pool and dependencies,
3. implementation remains blocked until launch is reconfirmed.

## User Interaction Protocol (Mandatory)

When collecting decisions:

1. Use structured options with one recommended choice.
2. Ask one category at a time (or max 3 tightly coupled categories).
3. After each answer: run conflict check against prior decisions.
4. For multi-select needs: use sequential questions or explicit combination options.
5. Capture free-form user choice via `Other` path when needed.

Decision categories (default):

1. business outcome,
2. UX/design behavior,
3. data/state model,
4. external API/integration,
5. security/compliance,
6. NFR (performance/offline/reliability).

## External Contract Validation (Mandatory When API Exists)

Use `web-validation-protocol.md` as canonical validation contract.

Evidence must include:

1. live request/response samples (`curl` or equivalent),
2. status codes + error bodies,
3. auth/session behavior,
4. payload/field reality vs expected,
5. documented constraints (rate/pagination/version).

If reality conflicts with prior assumption, assumption must be replaced.

## Design Contract (Mandatory in Spec)

Include:

1. user flow states (happy/error/empty/loading/retry),
2. navigation and entry/exit points,
3. component behavior and interaction rules,
4. accessibility/responsive constraints,
5. explicit mapping UI-state -> API/error state.

## Dynamic Skills Routing

Mandatory skill scan before finalizing non-trivial spec:

```bash
python3 skill-discovery.py suggest "<scope/request>" --top 8
```

Rules:

1. choose minimal sufficient skill set;
2. record applied skills + rationale in spec;
3. if skill gap exists, scaffold project candidate:

```bash
python3 skill-discovery.py scaffold <skill-name> "<description>"
```

## Confidence Gate (Weighted)

Compute before ready verdict:

```bash
python3 scp-confidence.py \
  --user-alignment <0..100> \
  --api-reality <0..100> \
  --evidence-quality <0..100> \
  --architecture-fit <0..100> \
  --delivery-readiness <0..100>
```

Bands:

1. `>=85` ready,
2. `70..84` conditional,
3. `<70` not ready.

## Required Spec Output Sections

1. Scope + goals
2. Decision Log summary
3. API Reality Matrix summary (if applicable)
4. Design Contract
5. Technical Contract
6. Acceptance Criteria
7. Risks/compatibility/rollback
8. Skills Applied + missing-skill plan
9. Confidence Scorecard
10. Draft Execution-Spec
11. References (research/docs/API evidence links)

## Pack Integration

Use `spec-pack` with SCP step mapping. Log execution as TaskFlow blocks and keep `next_step` chain explicit.

-----
artifact_path: config/command-instructions/vida.spec
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.vida-spec-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:27:08+02:00'
changelog_ref: operator.vida-spec-guide.changelog.jsonl
