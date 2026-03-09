# Product Proving Packs Protocol

Purpose: provide reusable proving-pack templates for high-value product and framework regression surfaces.

Canonical helper:

```bash
python3 docs/framework/history/_vida-source/scripts/proving-pack.py <navigation_ownership|account_switch|locale_preservation|drawer_interaction|framework_self> [--task-id <task_id>]
```

Rules:

1. proving packs are reusable verification scaffolds, not task-state engines,
2. they help the orchestrator prepare bounded regression evidence without widening scope,
3. they should be used before bespoke ad hoc checklists when an existing proving surface already matches the issue,
4. framework proving should prefer the `framework_self` pack before broad rereads.
