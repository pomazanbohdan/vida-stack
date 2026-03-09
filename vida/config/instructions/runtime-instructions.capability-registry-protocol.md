# Capability Registry Protocol (CRP)

Purpose: define a framework-owned typed capability registry for agent lanes and a deterministic compatibility gate between route task classes and candidate workers.

## Core Contract

1. Route selection may use scoring and cost heuristics.
2. Compatibility must be checked before scoring can authorize a lane.
3. A candidate that fails compatibility is ineligible, not merely low-ranked.

## Canonical Artifact

1. `.vida/state/capability-registry.json`

## Typed Task-Class Requirements

Registry must define requirement groups for at least:

1. `analysis`
2. `coach`
3. `verification`
4. `verification_ensemble`
5. `review_ensemble`
6. `problem_party`
7. `read_only_prep`
8. `implementation`

Each group must declare:

1. `allowed_write_scopes`
2. `required_capability_any`
3. `required_artifacts`
4. `forbidden_capabilities`

## Commands

```bash
python3 capability-registry.py build
python3 capability-registry.py check <task_class> <worker>
```

-----
artifact_path: config/runtime-instructions/capability-registry.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.capability-registry-protocol.md
created_at: 2026-03-08T02:15:22+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.capability-registry-protocol.changelog.jsonl
