# Use-Case Packs (Slim-VIDA)

Purpose: lightweight runtime routing. Use one focused playbook per request instead of loading all VIDA layers.

## Core Packs

| Pack | Trigger | Minimal Inputs | Mandatory Outputs |
|---|---|---|---|
| `research-pack` | Unknown domain, external validation needed | user goal, scope limits | source-backed findings, risks, next options |
| `spec-pack` | Requirement/spec creation or update | target feature, constraints | updated spec scope, AC, edge cases |
| `work-pool-pack` | build/update task pool between spec and dev | approved scope/spec, priority, dependencies | decomposed task pool in `br` + launch decision |
| `dev-pack` | start/continue implementation | active `TaskFlow` task, target files | code/test changes + verification |
| `bug-pool-pack` | bug triage/fix loop | bug evidence, reproduction | root-cause fix + regression checks |
| `reflection-pack` | decisions/docs drift, scope/AC/dependency drift, or explicit VIDA self-analysis request | accepted decisions, touched docs, drift trigger | contracts/docs/task-pool synchronized |

## Runtime Contract

Generic orchestration lifecycle is owned by `vida/config/instructions/instruction-contracts.orchestration-protocol.md`.
This file owns pack taxonomy, pack-selection intent, and pack-specific step maps.

Transition note:

1. pack helper commands below are legacy wrapper examples,
2. they remain migration-only until pack orchestration is either moved into `vida-v0` or intentionally retired,
3. they must not be read as the final post-cutover canonical runtime home.

1. Detect pack via `bash docs/framework/history/_vida-source/scripts/vida-pack-helper.sh detect "<request>"`.
2. Start the selected pack session:
   `bash docs/framework/history/_vida-source/scripts/vida-pack-helper.sh start <task_id> <pack_id> "<goal>" [constraints]`.
3. Scaffold the pack-specific TaskFlow plan (optional but recommended):
   `bash docs/framework/history/_vida-source/scripts/vida-pack-helper.sh scaffold <task_id> <pack_id>`.
   - Scaffold pre-registers only near-term 2-3 blocks; extend next blocks just-in-time.
4. Execute pack blocks via the standard workflow:
   `block-plan -> block-start -> block-end -> reflect -> verify`.
5. End the selected pack session:
   `bash docs/framework/history/_vida-source/scripts/vida-pack-helper.sh end <task_id> <pack_id> <done|partial|failed> "<summary>" [next]`.

Rule:

1. these commands are migration-only wrapper examples,
2. they are allowed only until pack orchestration is moved into `vida-v0` or intentionally retired.

Shortcut for standard non-dev pack initialization:

```bash
bash docs/framework/history/_vida-source/scripts/nondev-pack-init.sh <task_id> <research-pack|spec-pack|work-pool-pack|bug-pool-pack|reflection-pack> "<goal>" [constraints]
```

## Change-Impact Absorption (Cascade Behavior)

Standalone cascade handling is absorbed into pack orchestration.

Trigger conditions:

1. scope or AC changed after spec approval,
2. new dependency discovered during implementation,
3. decision update changes contract expectations,
4. task pool in `br` no longer matches current spec.

Mandatory route:

1. stop execution with `BLK_CHANGE_IMPACT_PENDING`,
2. start `reflection-pack` and update affected artifacts,
3. run `/vida-spec review` to re-validate contract,
4. run `/vida-form-task` to reconcile/rebuild task pool,
5. continue `/vida-implement` only after explicit launch confirmation.

## SCP Gate (Non-Dev)

`vida/config/instructions/runtime-instructions.spec-contract-protocol.md` is mandatory for all non-dev packs:

1. `research-pack`
2. `spec-pack`
3. `work-pool-pack`
4. `bug-pool-pack`
5. `reflection-pack`

Web validation gate:

1. For all non-dev packs, run `vida/config/instructions/runtime-instructions.web-validation-protocol.md` when triggers fire.
2. Record WVP evidence before pack completion.

Exemption:

1. `dev-pack` (`/vida-implement*`) is excluded and uses implementation protocol.

## Completion Gates

1. At least one `block_end` exists for execution packs.
2. `pack_start` and `pack_end` counts are balanced.
3. Strict verify passes before close/handoff.
4. Reflection updates canonical docs before reporting done.
5. For non-dev packs, SCP confidence score is reported before completion.
6. If WVP trigger fired, pack cannot be marked done without WVP evidence.

## `spec-pack` Step Map (SCP)

When scaffolding `spec-pack`, use SCP block chain:

1. `SCP01`: Intake + Interactive Discovery
2. `SCP02`: Conflict Check + API Reality Validation
3. `SCP03`: Design Contract + Technical Contract
4. `SCP04`: Skills Routing + Confidence Scoring
5. `SCP05`: Reassessment + Ready Verdict

## `work-pool-pack` Step Map (`/vida-form-task`)

Use `vida/config/instructions/command-instructions.form-task-protocol.md` as canonical flow:

1. `FT01`: Intake + Preflight (spec/SCP/blockers)
2. `FT02`: Change-impact reconciliation (if drift detected)
3. `FT03`: Task-scope options + user question cards
4. `FT04`: Build/Update `TaskFlow` task pool
5. `FT05`: Dependency graph + cycle validation
6. `FT06`: Readiness verdict (`ready|blocked|deferred`)
7. `FT07`: Launch gate (explicit user confirm before `/vida-implement`)

## `dev-pack` Step Map (`/vida-implement`)

Use `vida/config/instructions/command-instructions.implement-execution-protocol.md` as canonical flow:

Command-layer alignment:

1. `CL1` -> `IEP01`
2. `CL2` -> `IEP02` + `IEP03`
3. `CL3` -> change-impact gate inside `IEP03`
4. `CL4` -> `IEP04`
5. `CL5` -> `IEP05` + `IEP06` + `IEP07`

1. `IEP01`: Launch intake + context hydration
2. `IEP02`: Ready queue intake from `br` + dynamic skills routing
3. `IEP03`: Preflight + change-impact gate
4. `IEP04`: Implementation loop for current task
5. `IEP05`: Verify/review/regression/API-live validation gates
6. `IEP06`: Close task + auto-continue to next ready task
7. `IEP07`: Pool completion + documentation/spec synchronization

## `bug-pool-pack` Step Map (BFP)

Use unified bug-fix protocol from `vida/config/instructions/command-instructions.bug-fix-protocol.md`:

Command-layer alignment:

1. `CL1` -> `BFP01`
2. `CL2` -> `BFP02`
3. `CL3` -> `BFP03`
4. `CL4` -> implementation inside `BFP03`
5. `CL5` -> `BFP04` + `BFP05`

1. `BFP01`: Intake/normalization + impact priority
2. `BFP02`: Reproduce + root-cause evidence
3. `BFP03`: Fix planning + implementation
4. `BFP04`: Verification + regression
5. `BFP05`: Documentation/spec sync + closure verdict

## `reflection-pack` Step Map

Default reflection/documentation reconciliation scaffold:

1. `P01`: Reconcile decisions with canonical docs
2. `P02`: Update SSOT and protocol index
3. `P03`: Capture changes and verify conflicts

Explicit VIDA/framework self-analysis scaffold (`reflection-pack` + FSAP):

1. `FSAP01`: `FSAP-0_2_Trigger_Runtime_Snapshot_and_Evidence_Scope`
2. `FSAP02`: `FSAP-3_5_Friction_Classification_Ownership_Split_and_Improvement_Decision`
3. `FSAP03`: `FSAP-6_8_Canonical_Update_Delegated_Verification_and_Report`

## Notes

1. `br` remains the only task-state source of truth.
2. TaskFlow board is execution visibility, not task-state authority.
3. Multi-pack requests use sequence: `research -> spec -> work-pool -> dev/bug-pool -> reflection`.
4. Explicit VIDA/framework diagnosis requests route through `reflection-pack` and `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md`.
5. Tracked FSAP/remediation closure requires delegated verification/proving evidence or a structured override receipt recorded through `docs/framework/history/_vida-source/scripts/fsap-verification-gate.py`.

Legacy note:

1. `vida-pack-helper` and `nondev-pack-init` are explicitly migration-only wrapper surfaces per `vida/config/instructions/system-maps.runtime-transition-map.md`.

Non-dev handoff boundary:

1. `research-pack` hands off evidence and approved business-level deltas to `spec-pack`.
2. `spec-pack` hands off approved contract and readiness context to `work-pool-pack`.
3. `work-pool-pack` hands off ready queue plus explicit launch decision to `dev-pack`.

-----
artifact_path: config/command-instructions/use-case-packs
artifact_type: command_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/command-instructions.use-case-packs.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: command-instructions.use-case-packs.changelog.jsonl
