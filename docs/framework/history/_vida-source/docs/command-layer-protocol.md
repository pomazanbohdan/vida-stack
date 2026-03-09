# Command Layer Protocol (CLP)

Purpose: define one canonical protocol-layer matrix for VIDA command surfaces.

Scope:

1. Applies to `_vida/commands/vida-*.md`.
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
2. `br` task updates,
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
3. command decomposition must reference layer ids before introducing subagent/task granularity.

Protocol-unit format:

1. represent delegable/plannable units as `<command>#CL1..CL5`,
2. use the unit id in TODO goals, audit inventory, and subagent prompts when work is scoped below full-command level,
3. keep final gate ownership in the orchestrator even when read-heavy evidence collection is delegated.

## Consistency Rules

When command topology changes in the same scope:

1. update this file,
2. update touched `_vida/commands/vida-*.md`,
3. update `_vida/docs/protocol-index.md`,
4. update `_vida/docs/framework-map-protocol.md` if framework topology language changes.

## Related

1. `_vida/commands.md`
2. `_vida/docs/framework-map-protocol.md`
3. `_vida/docs/protocol-index.md`
4. `_vida/commands/vida-research.md`
5. `_vida/commands/vida-spec.md`
6. `_vida/commands/vida-form-task.md`
7. `_vida/commands/vida-implement.md`
8. `_vida/commands/vida-bug-fix.md`
9. `_vida/commands/vida-status.md`
