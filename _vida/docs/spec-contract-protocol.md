# Spec Contract Protocol (SCP)

Purpose: guarantee that specifications reflect real user intent, real external contracts, and implementation reality.

Scope:

1. Mandatory for non-development flows:
   - `research-pack`
   - `spec-pack`
   - `work-pool-pack`
   - `bug-pool-pack`
   - `reflection-pack`
2. Excluded from direct development execution flow (`/vida-implement*`, `dev-pack`).
3. Raw issue text for bugfix execution is normalized by `_vida/docs/issue-contract-protocol.md` instead of running full SCP by default.

## Core Principle

Spec is a contract between:

1. user decisions,
2. research evidence,
3. real external API/docs,
4. architecture constraints,
5. delivery/testability constraints.

## SCP / Issue-Contract Boundary

1. SCP remains the canonical contract path for non-development spec work.
2. `issue_contract` is the narrow execution-side contract for bug/spec convergence when the incoming issue text is effectively the working spec.
3. If issue analysis shows a non-equivalent change (`spec_delta_required`), route back into SCP/FTP reconciliation before implementation continues.

## Command Layer Mapping

For `/vida-spec`, SCP layers map to CLP as follows:

1. `CL1 Intake` -> `SCP-0 Intake`
2. `CL2 Reality And Inputs` -> `SCP-1 Interactive Discovery` + `SCP-2 Conflict Check` + `SCP-3 External Reality Validation`
3. `CL3 Contract And Decisions` -> `SCP-4 Design Contract` + `SCP-5 Technical Contract`
4. `CL4 Materialization` -> `SCP-6 Skills Routing` + confidence/artifact assembly
5. `CL5 Gates And Handoff` -> `SCP-7 Reassessment Gate` + `SCP-8 Ready Verdict` + `/vida-form-task` handoff

Canonical layer source: `_vida/docs/command-layer-protocol.md`

## Protocol Flow

1. `SCP-0 Intake`
   - Build `Spec Brief`: scope, business goal, constraints, out-of-scope.
2. `SCP-1 Interactive Discovery`
   - Run category-based questioning with options + recommendation + trade-offs.
   - Categories: business, UX/design, data, API/integration, security, NFR.
   - Record decisions into a decision log.
3. `SCP-2 Conflict Check`
   - Detect incompatible user choices; resolve before continuing.
4. `SCP-3 External Reality Validation`
   - Execute `_vida/docs/web-validation-protocol.md` (WVP) as canonical validation flow.
   - If external API exists: verify with live requests (`curl`), docs, auth, payload, errors.
   - Build `API Reality Matrix` (`expected/actual/conflict/unknown`).
5. `SCP-4 Design Contract`
   - Define user flows, screen states, component/state behavior, error/loading/empty states.
6. `SCP-5 Technical Contract`
   - Define interfaces, DTO/contracts, module boundaries, observability/error policy, AC.
7. `SCP-6 Skills Routing`
   - Discover relevant skills dynamically for the current scope.
   - If missing capability exists, scaffold project skill candidate.
8. `SCP-7 Reassessment Gate`
   - Show consolidated decisions/contracts to user and require explicit confirmation.
9. `SCP-8 Ready Verdict`
   - Mark spec ready only if all gates are satisfied.

## Mandatory Web Search and External Verification

Canonical rules are defined in `_vida/docs/web-validation-protocol.md`.

SCP-specific enforcement:

1. During `SCP-3`, run WVP for all fired triggers.
2. If external API exists, live validation is mandatory before finalizing assumptions.
3. Record WVP evidence and URLs in spec notes/artifacts.
4. If WVP ends with unresolved conflict, do not pass `SCP-8`.

## Confidence Model (Weighted)

Compute readiness confidence before `SCP-8`:

```
confidence =
  0.25 * user_alignment +
  0.25 * api_reality +
  0.20 * evidence_quality +
  0.15 * architecture_fit +
  0.15 * delivery_readiness
```

Each component is scored 0..100.

Interpretation:

1. `>= 85` Ready.
2. `70..84` Conditional ready (explicit risk list).
3. `< 70` Not ready.

Downgrade factors (inspired by certainty frameworks):

1. inconsistency between sources,
2. indirect or stale evidence,
3. unresolved conflict decisions,
4. missing error-contract validation,
5. no user reassessment confirmation.

Weight rationale:

1. `user_alignment` + `api_reality` are highest (0.25 + 0.25) because mismatch here causes direct product regressions.
2. `evidence_quality` is next (0.20) to penalize weak/indirect sources.
3. `architecture_fit` and `delivery_readiness` (0.15 + 0.15) ensure implementability.

## External Method Basis (References)

1. NIST AI RMF (measure/manage emphasis on uncertainty, documentation, continuous monitoring):
   - https://www.nist.gov/itl/ai-risk-management-framework/nist-ai-rmf-playbook
   - https://airc.nist.gov/airmf-resources/airmf/5-sec-core/
2. GRADE certainty model (transparent downgrades: inconsistency, indirectness, imprecision, bias):
   - https://training.cochrane.org/grade-approach
   - https://training.cochrane.org/handbook/current/chapter-14
3. Contract-testing doctrine for consumer/provider assumptions:
   - https://docs.pact.io/
   - https://docs.pact.io/consumer

## Questioning Contract

1. Use structured user questions with clear options and one recommended option.
2. One decision card at a time (or 2-3 tightly related cards per batch).
3. Multi-select is emulated via:
   - sequential focused questions, or
   - explicit combination options, or
   - free-text `Other` response.

## SCP -> FTP/TODO Handoff Contract

SCP output must be planning-ready for `/vida-form-task` and TODO decomposition.

Required handoff fields:

1. `scope_boundary` (`IN/OUT` + exclusions).
2. `delivery_cut` (MVP vs full slice intent).
3. `dependency_strategy` hint (sequential vs parallel-safe).
4. `risk_policy` baseline (conservative/balanced/aggressive).
5. `decision_conflicts` (must be empty for ready state).
6. `external_contract_status` (`validated|conflicting|unknown`).

Handoff rule:

1. If `decision_conflicts` is non-empty -> fail readiness.
2. If `external_contract_status` is not `validated` where required -> fail readiness.
3. FTP must consume these fields before creating task pool.

## Dynamic Skills Contract

Use dynamic skill discovery for each new scope:

```bash
python3 _vida/scripts/skill-discovery.py suggest "<request or scope>" --top 8
```

Interpretation:

1. Select minimal sufficient skill set.
2. Record selected skills and rationale in spec notes.
3. If no suitable project skill exists, scaffold one:

```bash
python3 _vida/scripts/skill-discovery.py scaffold <skill-name> "<description>"
```

4. New skill is candidate until reviewed and wired into runtime docs.

## Artifacts (Mandatory for spec-pack)

1. `Decision Log`
2. `API Reality Matrix` (when API exists)
3. `Design Contract` section
4. `Technical Contract` section
5. `Skills Applied` section
6. `Confidence Scorecard` (five weighted components + final score)

Artifact templates: `_vida/docs/spec-contract-artifacts.md`

## Minimal Transparency Rules

When reporting status to user:

1. show active task ID + title + short description,
2. show current SCP step,
3. show open decisions/blockers,
4. show next step.
5. show current confidence (score + band).

## Fail Conditions

Spec is NOT ready if any is true:

1. unresolved decision conflicts,
2. unvalidated external API assumptions,
3. missing design states for critical flows,
4. no explicit acceptance criteria,
5. no user confirmation at reassessment gate.
