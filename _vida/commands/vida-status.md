# /vida-status — BR TODO Dashboard

Purpose: read-only terminal report for decision-making over current `br` workload.

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> dashboard scope selection and output mode
2. `CL2 Reality And Inputs` -> read-only queue collection from `br`
3. `CL3 Contract And Decisions` -> ready/non-ready grouping and summary classification
4. `CL4 Materialization` -> terminal report rendering
5. `CL5 Gates And Handoff` -> explicit read-only completion guarantee

Canonical source: `_vida/docs/command-layer-protocol.md`

## Contract

1. This command is informational only.
2. It MUST NOT change task statuses, dependencies, or content.
3. It reads data only from `br` JSON commands.
4. It shows both top-level tasks and subtasks with concise descriptions.
5. It must end without changing runtime mode, queue order, or task content.

## Runtime Command

```bash
bash _vida/scripts/vida-status.sh
```

## Output Sections

1. `Summary`:
   - top-level TODO (`open`)
   - top-level IN PROGRESS (`in_progress`)
   - subtasks TODO (`open` + `parent!=null`)
   - subtasks IN PROGRESS (`in_progress` + `parent!=null`)
2. `Top-level Tasks (open + in_progress)` with `ready=yes/no` flag.
3. `Subtasks TODO (open)`.
4. `Subtasks IN PROGRESS`.

## Data Sources

1. `br list --status open --json` via canonical wrapper path
2. `br list --status in_progress --json` via canonical wrapper path
3. `br ready --json` via canonical wrapper path

## Constraints

1. No writes to `br`.
2. No non-canonical state model usage.
3. No separate sync-command integration.
