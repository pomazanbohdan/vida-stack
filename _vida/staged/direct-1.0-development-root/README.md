# VIDA Direct 1.0 Development Root

Purpose: provide one canonical staged environment root that can be copied into the final development environment and used as the compact-safe source for plan-driven agent development toward `VIDA 1.0`.

This folder is not the final runtime repository root.
It is a portable handoff root that contains:

1. the master development plan,
2. the next-agent start prompt,
3. the staged Binary Foundation scaffold,
4. transfer rules,
5. exact truth-source pointers back into the canonical `_vida/docs/*` specs and continuation artifacts.

Use this folder when:

1. the current repository must remain clean outside `_vida/*`,
2. the next environment will run the real implementation,
3. the next agent needs one place to start from without broad rereads.

Contents:

1. `MASTER-DEVELOPMENT-PLAN.md`
2. `NEXT-AGENT-START-PROMPT.md`
3. `TRANSFER-RULES.md`
4. `binary-foundation/`

Core rule:

1. this staged root organizes implementation work,
2. canonical product-law remains in `_vida/docs/plans/*` and `_vida/docs/research/*`,
3. if a staged artifact conflicts with canonical docs, the canonical docs win.

