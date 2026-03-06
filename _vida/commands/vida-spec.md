# /vida-spec — Final Specification Engine (SCP-Based)

Purpose: produce implementation-ready specs that are aligned with user intent, real external contracts, and project architecture.

Primary protocol: `_vida/docs/spec-contract-protocol.md` (SCP).

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> spec brief, scope target, and required artifact intake.
2. `CL2 Reality And Inputs` -> discovery plus external/API reality validation through SCP.
3. `CL3 Contract And Decisions` -> conflict resolution plus design/technical contract decisions.
4. `CL4 Materialization` -> spec artifact production, skills routing, and confidence scoring.
5. `CL5 Gates And Handoff` -> reassessment, ready verdict, and approved-contract handoff to `/vida-form-task`.

Canonical source: `_vida/docs/command-layer-protocol.md`

Handoff boundary:

1. `/vida-spec` owns the approved product/technical contract.
2. Task-pool creation starts in `/vida-form-task`.

## Position in Current Engine

1. `/vida-research` = BA external discovery.
2. `/vida-spec` = SA contract formation (including feature->spec coverage checks).
3. `/vida-form-task` = task/pool formation from approved contract.
4. `/vida-implement*` = development execution (outside SCP scope).

## Runtime Invariants

1. Task state SSOT: `br` + beads logs only.
2. No legacy `state/**` and no separate sync-command behavior.
3. No hidden side-effects outside TODO/beads workflow.
4. Do not finalize spec without explicit reassessment confirmation.

## Mandatory Inputs (Read Before Any Spec Output)

1. `docs/feature-checklist.md`
2. `docs/research/*` relevant files
3. `docs/specs/README.md` + related `docs/specs/**`
4. `docs/decisions.md`
5. `_vida/docs/spec-contract-protocol.md`
6. `_vida/docs/web-validation-protocol.md`
7. `_vida/docs/beads-protocol.md` + `_vida/docs/todo-protocol.md`

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
8. `Confidence`: calculate weighted SCP confidence.
9. `SCP-7 Reassessment`: consolidated confirmation with user.
10. `SCP-8 Ready Verdict`: ready/conditional/not-ready.
11. Persist spec and update `br` docs anchor when needed.

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

Use `_vida/docs/web-validation-protocol.md` as canonical validation contract.

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
python3 _vida/scripts/skill-discovery.py suggest "<scope/request>" --top 8
```

Rules:

1. choose minimal sufficient skill set;
2. record applied skills + rationale in spec;
3. if skill gap exists, scaffold project candidate:

```bash
python3 _vida/scripts/skill-discovery.py scaffold <skill-name> "<description>"
```

## Confidence Gate (Weighted)

Compute before ready verdict:

```bash
python3 _vida/scripts/scp-confidence.py \
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
10. References (research/docs/API evidence links)

## Pack Integration

Use `spec-pack` with SCP step mapping. Log execution as TODO blocks and keep `next_step` chain explicit.
