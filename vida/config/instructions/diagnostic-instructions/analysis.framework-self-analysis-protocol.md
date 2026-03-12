# VIDA Framework Self-Analysis Protocol (FSAP)

Purpose: run a bounded meta-diagnostic of the VIDA framework itself when the user explicitly asks to inspect protocol friction, instruction conflicts, token overhead, runtime ergonomics, or framework/process efficiency.

Quality and token efficiency are equal-weight goals in FSAP/debug mode.

Silent mode note:

1. Explicit FSAP is the foreground diagnosis protocol.
2. Silent diagnosis is the background capture protocol activated from root `vida.config.yaml`.
3. Silent diagnosis must hand off framework fixes into normal tracked framework work after the active task boundary; it is not a license for silent in-place framework rewrites during unrelated product work.

## Hard-Law Doctrine

FSAP must treat mandatory framework behavior as executable law, not advisory prose.

1. If a framework rule is expressed as `must`, `required`, `forbidden`, `invalid`, or `blocked`, FSAP must verify that a runtime gate, verifier, blocker code, or fail-fast path exists for it.
2. If a mandatory rule exists only as guidance text, classify that as a `framework-owned` protocol gap even when operators usually follow it.
3. Recommendations are valid only for non-mandatory heuristics, ranking preferences, and explicitly labeled option matrices.
4. When options exist, FSAP must require explicit option-selection conditions instead of vague language such as `maybe`, `probably`, `consider`, or `prefer` without a decision boundary.
5. Canonical update rule: every mandatory behavior discovered during FSAP should either:
   - already have a runtime enforcement path,
   - be implemented with one during the same improvement cycle,
   - or be reported as an unresolved framework defect.

## Trigger

Run FSAP only on explicit user request, for example:

1. "diagnose VIDA/framework"
2. "analyze what should be improved in the framework"
3. "run VIDA self-analysis"
4. "check instruction or script conflicts"
5. "find what reduces iterations, token cost, or context rereads"

Do not use FSAP for product/codebase diagnosis unless the user explicitly asks about the framework/runtime itself.

## Routing

1. Default execution lane: main orchestrator only for direct chat diagnosis and tracked FSAP trigger framing.
2. Default task-flow policy: bypass TaskFlow/pack flow and execute in chat mode unless the user explicitly requests tracked execution or a formal artifact.
3. Delegation policy:
   - do not delegate the primary FSAP trigger framing or ownership split to workers by default;
   - in untracked chat mode, use delegated lanes only for narrow secondary verification when the orchestrator is blocked or the user explicitly requests delegation;
   - in tracked FSAP/remediation mode, delegated verification/proving is the default before closure or report-finalization, and a local-only close path requires a structured override receipt.
4. Thinking mode:
   - default `META` for explicit self-analysis requests;
   - downgrade to `MAR` only for narrow single-script questions with low blast radius.
5. Scope: `AGENTS.md`, `vida/config/instructions/*`, `*`, runtime logs, and only the project evidence that proves a framework-level friction point.
6. Instruction-layer efficiency is in scope: FSAP must inspect `AGENTS.md`, lane entry contracts, and canonical protocols when instruction ambiguity or drift increases rereads, optionality, routing confusion, or token cost.

When the user explicitly requests tracked execution, use `reflection-pack` and the dedicated FSAP chain:

1. `FSAP01`: `FSAP-0_2_Trigger_Runtime_Snapshot_and_Evidence_Scope`
2. `FSAP02`: `FSAP-3_5_Friction_Classification_Ownership_Split_and_Improvement_Decision`
3. `FSAP03`: `FSAP-6_8_Canonical_Update_Delegated_Verification_and_Report`

Reflection-pack bridge admissibility:

1. `reflection-pack` may route into FSAP only when tracked framework self-analysis/remediation is the actual target.
2. Ordinary documentation drift, spec/task-pool synchronization, or generic change-impact handling inside `reflection-pack` must stay with their existing canonical owners.
3. Entering `reflection-pack` alone is not proof that FSAP is active.

## Core Boundary

FSAP must separate findings into two ownership buckets:

1. `framework-owned`
   - VIDA runtime protocols
   - AGENTS rules
   - `vida/config/instructions/*`
   - `*`
2. `project-owned`
   - app-specific runbooks
   - `docs/*`
   - `scripts/*`
   - codebase/tooling issues that only expose a framework gap

Rule:

1. Do not "fix project pain" inside `legacy helper surfaces`.
2. Do not store framework policy in `docs/*`.
3. If one symptom spans both layers, produce split actions per ownership layer.

## FSAP Workflow

1. `FSAP-0 Trigger Confirmation`
   - confirm the request is about VIDA/framework behavior, not product behavior.
2. `FSAP-1 Runtime State Snapshot`
   - capture the current orchestrator/runtime state and relevant health/status views.
   - if tracked execution is active, also capture current task id, active TaskFlow block, and pack state.
   - preferred shortcuts:
     - dev/task-state visibility: `taskflow-v0 boot snapshot --json`
     - untracked mode: `taskflow-v0 system snapshot` plus bounded queue/status reads as needed,
     - tracked mode: `bash framework-self-check.sh <task_id>`.
3. `FSAP-2 Evidence Collection`
   - inspect only the protocols/scripts actually involved in the observed friction.
   - prefer direct script/doc reads over broad repo sweeps.
4. `FSAP-3 Friction Classification`
   - classify each issue as:
     - protocol gap
     - script/runtime bug
     - instruction conflict
     - ergonomics/observability gap
     - project issue mislocated in framework
   - additionally classify whether the issue is:
     - `hard-law missing`
     - `hard-law present but unenforced`
     - `heuristic and correctly non-mandatory`
5. `FSAP-4 Ownership Split`
   - mark every finding `framework-owned` or `project-owned`.
6. `FSAP-5 Improvement Decision`
   - choose fixes that reduce:
     - quality regressions,
     - iteration count
     - repeated rereads
     - stale state/conflicting status
     - unnecessary token spend
     - ambiguous ownership
   - do not accept a "token-efficient" change that weakens enforcement quality, and do not accept a "quality" change that needlessly multiplies tokens when an equally safe cheaper path exists.
   - for every mandatory rule, decide the concrete enforcement surface:
     - runtime gate,
     - verifier gate,
     - blocker code,
     - schema validation,
     - or structured option matrix
7. `FSAP-6 Canonical Update`
   - update framework files in `legacy helper surfaces`.
   - if project fixes are in scope, update `docs/*` / `scripts/*` separately in the same request.
   - do not leave a mandatory finding in advisory wording when the framework can enforce it mechanically.
8. `FSAP-7 Verification`
   - run the lightest proof that the framework fix changed behavior.
   - in tracked mode, prefer delegated verification/proving lanes over a second local orchestrator-only audit.
   - before closure-ready state, require either:
     - a delegated verification artifact with real worker activity, or
     - a structured override receipt recorded by `fsap-verification-gate.py`.
9. `FSAP-8 Report`
   - report findings in chat, grouped by ownership.

## Required Evidence

Every FSAP report must include:

1. active execution context:
   - untracked mode: direct orchestrator FSAP run,
   - tracked mode: active `TaskFlow` task id + short description
2. active TaskFlow block(s) when tracked mode is active
3. concrete file/script references for each finding
4. why the issue increases iterations/context/tokens
5. whether the fix belongs to framework or project layer
6. whether each mandatory finding is already enforced, newly enforced, or still unenforced

## Preferred Verification

Use the smallest proof that demonstrates the framework change:

1. `bash -n` for shell scripts
2. `taskflow-tool current|compact` for TaskFlow/runtime state fixes in tracked mode
3. `python3 fsap-verification-gate.py check <task_id>` for tracked FSAP verification readiness
4. `quality-health-check.sh --mode quick <task_id>` for protocol sanity in tracked mode
5. a focused smoke command that reproduces the improved behavior

Avoid full project build/test loops unless the framework change directly affects them.

## Optional Tracked Escalation

If the user explicitly requests task tracking, formal artifact production, or deferred multi-step follow-through, FSAP may escalate into tracked mode:

```bash
bash framework-wave-start.sh <task_id> <reflection-pack|dev-pack|work-pool-pack> "<goal>" [constraints]
```

Rule:

1. `framework-wave-start.sh` is a migration-only wrapper surface.
2. It must not be treated as the long-term canonical runtime entrypoint after the `taskflow-v0` cutover.

Use this only when at least one is true:

1. the user explicitly asks for tracked execution,
2. the diagnosis must create or update formal artifacts,
3. the follow-up work is too large for a direct orchestrator chat run.

## Output Contract

Report in this structure:

1. `Framework-owned findings`
2. `Project-owned findings`
3. `Implemented framework improvements`
4. `Implemented project improvements` (only if in scope)
5. `Residual risks / next best improvements`

## Anti-Patterns

1. Mixing project bugs into framework conclusions without ownership split.
2. Broad rereads of unrelated protocols that do not change the decision.
3. Reporting "framework is better now" without a concrete behavioral proof.
4. Leaving framework/project ownership ambiguous after the analysis.
5. Auto-routing explicit VIDA self-diagnosis into TaskFlow when the user asked for direct diagnosis only.
6. Delegating the primary FSAP analysis away from the main orchestrator without an explicit reason.
7. Using the self-diagnosis exception to close tracked FSAP/remediation work without delegated verification or a structured override receipt.
7. Starting token-cost diagnosis with broad queue discovery before trying the compact boot snapshot or another exact-key status source.
8. Leaving mandatory framework behavior as a recommendation when the runtime could block or verify it directly.

-----
artifact_path: config/diagnostic-instructions/framework-self-analysis.protocol
artifact_type: diagnostic_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/diagnostic-instructions/analysis.framework-self-analysis-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:34:06+02:00'
changelog_ref: analysis.framework-self-analysis-protocol.changelog.jsonl
