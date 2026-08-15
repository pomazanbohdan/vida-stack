# Solution Minimality Protocol

Purpose: select the smallest complete, correct, and safe solution for a change-producing decision without weakening explicit requirements or safety invariants.

## Purpose

1. Replace vague simplicity preferences with one ordered first-fit decision contract.
2. Keep minimality inside an admissible safety floor.
3. Put fixes in the semantic owner of the affected invariant.
4. Leave the smallest proof appropriate to the changed behavior.

## Trigger

Apply this protocol after context and trace gates and before implementation selection when a step may produce a code, configuration, documentation, or process change.

Do not activate it for answer-only, translation, formatting, status reporting, or read-only discovery unless that work selects or recommends a concrete change.

Activation class: `triggered_domain`. Orchestrator bootstrap keeps the trigger available; worker lanes use it when their packet authorizes change-producing work.

## Scope

This protocol owns solution-candidate minimality, semantic-owner selection, and the minimum proof budget.

It does not own:

1. cross-step state, owned by `instruction-contracts/overlay.session-context-continuity-protocol`,
2. reasoning-mode selection, owned by `instruction-contracts/overlay.step-thinking-protocol`,
3. mutation authority, task state, approvals, or release gates,
4. correctness, security, accessibility, or validation policies that impose stronger proof.

## Authority

1. This file is the canonical owner for solution minimality in VIDA.
2. Bootstrap carriers, runtime capsules, activation maps, and indexes may point here but must not duplicate this algorithm.
3. Higher-precedence safety, user, platform, runtime-authority, and proof requirements override a smaller candidate.
4. The Ponytail project is provenance for the ordered-minimality pattern only; its names, modes, adapters, hooks, and benchmark claims have no VIDA runtime authority.

## Inputs

Required inputs:

1. requested outcome and explicit acceptance requirements,
2. active `must_do`, `must_not`, allowed scope, and protected scope,
3. traced entry points, callers, state, persistence, side effects, and trust boundaries,
4. existing codebase capabilities, standard library, platform primitives, and installed dependencies,
5. proof requirements imposed by the changed behavior and higher-precedence protocols.

## Outputs

The result is one first-fit admissible candidate plus its smallest required proof.

For a non-trivial, risky, disputed, or audited decision, retain a compact `solution_minimality_receipt`:

```yaml
solution_minimality_receipt:
  trace_scope: <entry-points/callers/state/trust-boundaries>
  safety_floor: <preserved-invariants>
  selected_rung: <skip|reuse_existing|stdlib|platform_native|installed_dependency|compact|minimum_new_code>
  rejected_prior_rungs: <reason-each-was-incomplete-incorrect-or-unsafe>
  semantic_owner: <shared-invariant-owner>
  proof: <smallest-runnable-check-or-explicit-triviality-reason>
```

Routine user-facing output must not print this receipt, rejected-rung analysis, or ladder narration unless it changes the decision, explains a blocker, or the user requests it.

## Rules

### 1. Trace First

Before evaluating candidates:

1. trace the real flow end to end,
2. identify all affected callers and shared state,
3. identify persistence, side effects, trust boundaries, and failure paths,
4. reject a small diff in the wrong ownership layer as non-admissible.

### Reflex Budget

1. Reuse already-read context, current indexes, manifests, and proof before making another lookup.
2. Evaluate each rung from available evidence first; when an earlier rung remains unresolved, use one bounded batched lookup round across only the relevant code, manifest, or platform sources.
3. Stop immediately at the first admissible rung. Do not research, compare, or narrate later rungs after selection.
4. Expand beyond the first lookup round only when semantic ownership, correctness, safety, compatibility, or required proof remains materially uncertain.
5. Do not repeat a lookup merely to reconfirm current evidence already sufficient for the decision.
6. Apply the Safe Default Gate before starting a clarification turn.

The compact runtime formula is: `reuse current evidence -> one batched lookup round -> first admissible rung -> stop`. This is a bounded reflex with a safety escape, not a hard one-tool-call limit.

### 2. Safety Floor

A candidate is admissible only when it is complete, correct, safe, and satisfies the explicit request.

The irreducible floor includes, when applicable:

1. trust-boundary validation,
2. error handling that prevents data loss,
3. security and privacy controls,
4. accessibility behavior,
5. explicitly requested behavior and compatibility constraints,
6. calibration controls for physical hardware,
7. stronger repository, runtime, CI, or release gates.

Safety is a predicate on candidates, not a later optimization pass.

### 3. Ordered First-Fit Ladder

Evaluate in this exact order and stop at the first admissible candidate:

| Rung | Question | Candidate |
|---:|---|---|
| 1 | Is the requirement speculative, already satisfied, or unnecessary for the requested outcome? | `skip` |
| 2 | Does the repository already own the helper, type, primitive, or pattern? | `reuse_existing` |
| 3 | Does the language standard library cover it? | `stdlib` |
| 4 | Does the target platform, framework, browser, database, or operating system provide it natively? | `platform_native` |
| 5 | Does an already installed dependency cover it correctly? | `installed_dependency` |
| 6 | Can it be expressed compactly without losing edge-case correctness, readability, or safety? | `compact` |
| 7 | Otherwise, what is the minimum new behavior needed? | `minimum_new_code` |

`skip` is invalid for an explicit unsatisfied requirement. A later rung is invalid while an earlier rung remains admissible.

### Safe Default Gate

Proceed without clarification only when exactly one admissible option exists, it is reversible, it preserves scope, safety, and authority, and its rollback path is known.

Ask the user only when no single safe admissible default exists or the missing choice materially changes scope, safety, authority, or irreversible behavior. Missing preference alone is not a blocker.

### Deterministic Tie-Breaker

When multiple candidates are equally complete, correct, safe, and comparable in proof cost, apply the first distinguishing criterion in this order:

`deletion -> reuse -> fewer files -> fewer dependencies -> fewer calls -> lower cognitive load`

Stop at the first distinguishing criterion. This tie-breaker never overrides the ordered ladder, explicit requirements, the safety floor, semantic ownership, or stronger proof gates.

### 4. Semantic Owner And Root Cause

1. Trace from the symptom to the invariant owner.
2. Search all callers of the candidate owner.
3. Apply one fix at the shared location that owns the invariant.
4. Do not repeat symptom patches in callers when one owner fix can preserve the invariant.
5. Keep the write set bounded to the owner and directly required callers, projections, and proof.

### 5. Dependency Gate

A new dependency is admissible only after repository, standard-library, platform-native, and installed-dependency options are proven insufficient because of a compatibility gap, missing required edge cases, measured scale/performance need, or material maintenance benefit.

### Inline Over-Engineering Smell Scan

After candidate selection and before proof, scan only the selected candidate and current diff for:

1. a single-implementation interface,
2. a one-product factory,
3. configuration with no active setter or consumer,
4. a wrapper that only delegates,
5. speculative scaffold or flexibility with no explicit requirement.

This scan reuses the current diff and context. It must not launch a new search, agent, review lane, or audit pass. Remove a smell immediately only when the cut is safe and remains inside the bounded scope; otherwise retain it with one concrete reason. Emit nothing when no smell is found.

### 6. Minimum Proof

1. Non-trivial logic must leave at least one runnable regression check.
2. Prefer one focused existing test or assert-based executable check.
3. Do not introduce a new test framework, broad fixtures, or unrelated harness code solely for a bounded check.
4. A trivial one-line change without branch, loop, parser, state, trust-boundary, or material regression risk may use an explicit triviality reason instead of a new test.
5. Higher-precedence repository or risk gates remain mandatory.

### 7. Deliberate Limit

When the selected solution knowingly has a ceiling, record both the ceiling and a measurable upgrade trigger in the existing task, code comment, or decision artifact. Do not create speculative upgrade machinery.

## Forbidden

1. Do not add speculative requirements to make a design look complete.
2. Do not duplicate an existing helper, type, policy, or platform primitive.
3. Do not add a dependency without passing the dependency gate.
4. Do not prefer a one-liner that is incomplete, obscure, unsafe, or edge-case incorrect.
5. Do not trade away validation, data-loss prevention, security, accessibility, explicit behavior, or hardware calibration for fewer lines.
6. Do not patch each symptom when a shared invariant owner is known.
7. Do not expand a bounded change into review, audit, debt-ledger, benchmark, mode, adapter, or telemetry infrastructure without an independent trigger and owner.

## Escalation

1. If trace evidence is incomplete, return to trace rather than guessing a small patch.
2. If no earlier rung is admissible, select `minimum_new_code` and state the blocking evidence for prior rungs.
3. If minimality conflicts with safety or an explicit requirement, safety and the explicit requirement win.
4. If ownership is disputed or multiple callers compute the same invariant, escalate through the active architecture/reasoning protocol before editing.
5. If the smallest runnable proof cannot execute, report the exact proof blocker; do not convert missing proof into a passing result.

## Validation

Protocol validation requires:

1. source DocFlow `check-file` for this owner and each changed pointer surface,
2. `protocol-coverage-check --profile active-canon`,
3. exact canaries for `Trace First`, `Reflex Budget`, `Safety Floor`, `Ordered First-Fit Ladder`, `Safe Default Gate`, `Deterministic Tie-Breaker`, `Semantic Owner And Root Cause`, `Inline Over-Engineering Smell Scan`, and `Minimum Proof`,
4. synchronized `AGENTS.md` and packaged scaffold pointer text,
5. token measurement with `tiktoken-cli --model gpt-4o`,
6. `cargo test -p docflow-cli --test solution_minimality_behavior` for the offline behavior grader.

Behavioral validation for consumers should probe:

1. explicit requirements cannot be skipped,
2. an existing helper wins over new code,
3. safety constraints reject a shorter unsafe candidate,
4. a shared invariant owner wins over repeated caller patches,
5. non-trivial logic leaves one runnable check,
6. selecting an existing helper performs no dependency research,
7. selecting the standard library performs no platform or package research,
8. one safe reversible admissible default proceeds without a clarification turn,
9. material uncertainty about ownership, correctness, safety, compatibility, or proof permits one bounded evidence-budget expansion,
10. an irreversible or materially scope-changing choice requires clarification,
11. equal candidates follow the deterministic tie-break order,
12. each declared over-engineering smell is detectable,
13. a clean diff yields no smell finding,
14. a shorter candidate that weakens safety is rejected.

## Token Budget

1. This owner carries the full decision contract.
2. Bootstrap carriers carry only trigger and owner pointers; runtime capsules may carry the compact reflex formula but not the full algorithm.
3. No fixed token target applies; semantic atoms, ordered rungs, safety floor, and validation gates must remain explicit.
4. Record the measured token count in task or changelog evidence.

## Metadata

1. Canonical id: `instruction-contracts/overlay.solution-minimality-protocol`.
2. Activation: `triggered_domain` for change-producing decisions.
3. Loading posture: compact pointer in bootstrap/capsules, owner on demand.
4. Provenance: user-supplied analysis of `DietrichGebert/ponytail` at `main@2ed6c52c`; provenance only, not runtime authority.
5. Initial task: `protocol-solution-minimality-20260815`.

-----
artifact_path: config/instructions/instruction-contracts/overlay.solution-minimality.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-08-15'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.solution-minimality-protocol.md
created_at: 2026-08-15T18:51:05+03:00
updated_at: 2026-08-15T16:59:19.2657398Z
changelog_ref: overlay.solution-minimality-protocol.changelog.jsonl
protocol_authoring_gate: enforced
