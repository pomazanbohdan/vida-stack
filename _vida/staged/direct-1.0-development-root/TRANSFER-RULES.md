# Transfer Rules

Purpose: define how this staged development root should be used in the final development environment.

## Copy Boundary

Copy this whole folder into the final development environment as a working handoff bundle.

Do not treat this folder itself as product law.

## Canonical Truth Sources

In the final environment, treat these as the canonical sources of truth:

1. `_vida/docs/plans/2026-03-08-vida-direct-1.0-compact-continuation-plan.md`
2. `_vida/docs/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
3. `_vida/docs/research/2026-03-08-vida-binary-foundation-next-step-after-compact-instruction.md`
4. `_vida/docs/plans/2026-03-08-vida-0.3-command-tree-spec.md`
5. `_vida/docs/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
6. `_vida/docs/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
7. `_vida/docs/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`
8. `_vida/docs/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
9. `_vida/docs/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md`
10. `_vida/docs/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`

## Development Start Rule

The next agent should:

1. read `NEXT-AGENT-START-PROMPT.md`,
2. verify the current blocker state from the continuation plan,
3. slice bounded child tasks from `MASTER-DEVELOPMENT-PLAN.md`,
4. start only the earliest lawful wave.

## Binary Foundation Transfer Rule

`binary-foundation/` is a staged scaffold only.

Apply it into the final environment root only when the next agent confirms:

1. `Binary Foundation` is still the active lawful wave,
2. the final environment allows root/runtime mutation,
3. no newer continuation artifact has superseded the staged scaffold.

## Return Path Rule

Any real progress made in the final environment must come back here as:

1. canonical doc updates,
2. next-step instruction updates,
3. receipts or proof summaries,
4. updated staged assets if the transfer bundle changes materially.

