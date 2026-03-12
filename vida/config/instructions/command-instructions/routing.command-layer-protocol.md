# Command Layer Protocol (CLP)

Purpose: define one canonical protocol-layer matrix for VIDA command surfaces.

Scope:

1. Applies to `vida/config/instructions/command-instructions.vida-*.md`.
2. Decomposes each command into stable protocol-scoped units.
3. Keeps command docs aligned without duplicating full runtime rules in every file.

## Core Contract

Each canonical `/vida-*` command must map its flow to the same five command layers.

Rule:

1. command docs describe which layers they execute,
2. protocol docs remain the detailed source for each layer,
3. command docs must not restate full protocol logic when the canonical source already exists.

## Canonical Command Layers

### `CL1 Intake`

Purpose:

1. resolve request scope,
2. identify entry conditions,
3. select the command-specific starting context.

Typical outputs:

1. scope target,
2. active task or artifact context,
3. command entry mode.

### `CL2 Reality And Inputs`

Purpose:

1. gather required inputs,
2. validate external facts or task-state reality when needed,
3. confirm that upstream prerequisites are present.

Typical outputs:

1. validated inputs,
2. blocker detection,
3. evidence needed for safe continuation.

### `CL3 Contract And Decisions`

Purpose:

1. form or refine the command-level contract,
2. resolve decisions or routing choices,
3. make the command boundary explicit before materialization.

Typical outputs:

1. decision set,
2. scope boundary,
3. ready-to-execute contract.

### `CL4 Materialization`

Purpose:

1. perform the core mutation or artifact production of the command,
2. create the main outputs the command exists to produce,
3. keep state mutation within the command's canonical ownership.

Typical outputs:

1. docs/spec updates,
2. `TaskFlow task updates,
3. implementation changes,
4. status rendering.

### `CL5 Gates And Handoff`

Purpose:

1. run verification or closure gates,
2. produce the next-command handoff,
3. make user-visible completion state explicit.

Typical outputs:

1. verdict,
2. next step,
3. readiness or close decision.

Report-stage rule:

1. A user-facing report may appear at any command layer only when it serves that layer's canonical purpose rather than replacing the next lawful step.
2. Reports are lawful at:
   - `CL1` as scope/intake framing,
   - `CL2` as evidence/reality status,
   - `CL3` as contract/decision summary,
   - `CL4` as artifact/materialization summary,
   - `CL5` as gate verdict and handoff.
3. A report is informational by default, not terminal by default.
4. If the next lawful step is already determined and no blocker exists, the command should continue automatically after the report when autonomous continuation is active.
5. Explicit user intent to discuss the current report converts that report boundary into a lawful stop point before the next step.
6. Validation reports that exist to approve or reject upcoming execution are gating reports and must not auto-continue into execution.
7. When a bounded task still contains unfinished lawful work after a report, the command should continue until that bounded task reaches actual closure rather than stopping at the report boundary.

Research-heavy closure rule:

1. For commands whose `CL2` or `CL3` materially depend on research, external validation, or comparative source analysis, `CL5` may not close while unresolved material research questions remain.
2. In these commands, the canonical default is autonomous continuation through the next required bounded research step until closure criteria are satisfied or a lawful blocker is reached.
3. In these commands, downstream practical validation or implementation-facing continuation is forbidden until the bounded sequence below is complete:
   - `CL2` research/evidence gathering,
   - research artifact update,
   - requirement formation,
   - `CL3` contract/spec/intake formation.
4. Evidence that exists only in chat, temporary notes, or ad hoc reasoning does not satisfy this sequence.
5. When a report is produced inside this sequence, default behavior is `report -> next lawful step`, not `report -> stop`, unless a lawful blocker or explicit user pause exists.

## Command Matrix

| Command | CL1 Intake | CL2 Reality And Inputs | CL3 Contract And Decisions | CL4 Materialization | CL5 Gates And Handoff |
|---|---|---|---|---|---|
| `/vida-research` | topic and research mode selection | source collection + WVP when needed | actionable candidate approval boundary | research/feature/decision artifact updates | handoff inputs for `/vida-spec` |
| `/vida-spec` | spec intake and scope brief | discovery + API/WVP reality checks | design contract + technical contract + conflict resolution | spec persistence + confidence scoring | reassessment + ready verdict |
| `/vida-form-task` | scope/task-pool intake and preflight | prerequisite and blocker validation | planning contract from question cards | `br` pool build + dependency graph | readiness verdict + launch gate |
| `/vida-implement` | launch intake + context hydration | queue intake + skills + preflight | change-impact decisions | implementation loop | verify/review + close/continue |
| `/vida-bug-fix` | issue intake + impact normalization | reproduction + root-cause evidence | fix/regression plan | fix implementation | verification + sync + close verdict |
| `/vida-status` | dashboard scope selection | read-only `br` data collection | queue classification and grouping | report rendering | read-only completion guarantee |

## Delegation Contract

These layers are the canonical protocol-scoped units for future planning and delegation work.

Rules:

1. read-heavy layers (`CL1`, `CL2`, parts of `CL3`, parts of `CL5`) are delegation-friendly,
2. mutation-heavy layers (`CL4`) remain single-writer unless explicit isolation exists,
3. command decomposition must reference layer ids before introducing worker/task granularity.

Research-to-practice sequence:

1. When a command contains a research-dependent path, the canonical sequence is:
   - gather bounded evidence in `CL2`,
   - update the command-owned research/reality artifact,
   - derive or refresh requirements,
   - form/refine the command contract/spec in `CL3`,
   - only then enter practical materialization or downstream execution.
2. Skipping the artifact, requirements, or contract refresh steps is a protocol violation even if the answer appears obvious.
3. If a report is emitted between these steps, it must explicitly preserve the next required step and must not silently become a stopping point.

Task-complete default:

1. For any command, once a bounded task is active, the operator should finish all still-lawful in-scope steps before stopping.
2. Stop early only when:
   - a blocker or gate is reached,
   - the next step would widen scope materially,
   - the user explicitly asked to pause, discuss, or reprioritize.
3. "Good enough to summarize" is not a lawful stop condition when bounded task-owned work still remains.

Protocol-unit format:

1. represent delegable/plannable units as `<command>#CL1..CL5`,
2. use the unit id in TaskFlow goals, audit inventory, and worker packets when work is scoped below full-command level,
3. keep final gate ownership in the orchestrator even when read-heavy evidence collection is delegated.

## Consistency Rules

When command topology changes in the same scope:

1. update this file,
2. update touched `vida/config/instructions/command-instructions.vida-*.md`,
3. update `vida/config/instructions/system-maps/protocol.index.md`,
4. update `vida/config/instructions/system-maps/framework.map.md` if framework topology language changes.

## Related

1. `vida/config/instructions/command-instructions/operator.command-catalog-index.md`
2. `vida/config/instructions/system-maps/framework.map.md`
3. `vida/config/instructions/system-maps/protocol.index.md`
4. `vida/config/instructions/command-instructions/operator.vida-research-guide.md`
5. `vida/config/instructions/command-instructions/operator.vida-spec-guide.md`
6. `vida/config/instructions/command-instructions/operator.vida-form-task-guide.md`
7. `vida/config/instructions/command-instructions/operator.vida-implement-guide.md`
8. `vida/config/instructions/command-instructions/operator.vida-bug-fix-guide.md`
9. `vida/config/instructions/command-instructions/operator.vida-status-guide.md`

-----
artifact_path: config/command-instructions/command-layer.protocol
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/routing.command-layer-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:24:49+02:00'
changelog_ref: routing.command-layer-protocol.changelog.jsonl
