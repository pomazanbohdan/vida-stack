# Self-Reflection Protocol

Purpose: keep decisions consistent, evidence-based, and easy to audit across block-by-block execution.

## When To Run

Run a self-reflection entry:

1. Before closing a task.
2. After context compression recovery (`compact post`).
3. Before high-impact architecture/security decisions.
4. When confidence is below 80%.

## Minimal Format (6 fields)

1. `goal` — what is being solved now.
2. `constraints` — critical rules/limits.
3. `evidence` — verified facts only.
4. `decision` — selected path and why.
5. `risks` — key failure/regression risks.
6. `next_step` — one concrete next action.

## Logging Command

Use the shared execution log channel:

```bash
bash beads-workflow.sh reflect <task_id> <goal> <constraints> <evidence> <decision> <risks> <next_step> [confidence]
```

Example:

```bash
bash beads-workflow.sh reflect bd-34r5 \
  "Normalize AGENTS sections" \
  "Keep boot rules + no duplicate policy blocks" \
  "Diff + protocol checks completed" \
  "Move operational detail to vida/config/instructions/*" \
  "Broken links after move" \
  "Run links/refs validation" \
  "85"
```

## Quality Rules

1. Keep entries short (30-90 seconds).
2. Distinguish facts from assumptions.
3. Every reflection must include one concrete `next_step`.
4. Avoid generic text like "all good" without evidence.

-----
artifact_path: config/diagnostic-instructions/self-reflection.protocol
artifact_type: diagnostic_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/diagnostic-instructions.self-reflection-protocol.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: diagnostic-instructions.self-reflection-protocol.changelog.jsonl
