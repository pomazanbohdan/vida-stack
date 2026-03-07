# Script Runtime Architecture

Purpose: define the canonical split between shell entrypoints and Python engine logic inside the VIDA framework runtime.

## Core Rule

VIDA runtime scripts use a hybrid model:

1. shell scripts (`_vida/scripts/*.sh`) keep the command surface stable,
2. Python engines (`_vida/scripts/*.py`) own stateful parsing, validation, routing, and scoring logic,
3. project delivery scripts stay in `scripts/` and are not part of this framework contract.

Rule:

1. do not duplicate the same business logic in both shell and Python,
2. when logic is migrated to Python, keep the shell file as a thin wrapper unless the command surface is intentionally removed,
3. shell wrappers may still own bootstrap-only responsibilities such as environment setup, lock handling, or command routing when that logic is shell-native.

## Ownership Split

Shell layer is responsible for:

1. preserving canonical command names and CLI ergonomics,
2. minimal environment bootstrap (`SCRIPT_DIR`, repo root, `exec python3 ...`),
3. shell-native orchestration such as lock acquisition, process composition, or command dispatch,
4. integration with other shell-first runtime adapters when no complex state derivation is needed.

Python engine layer is responsible for:

1. JSON/JSONL parsing,
2. task/runtime state derivation,
3. scoring, routing, and validation rules,
4. deterministic snapshot generation,
5. logic that would otherwise require deep `jq|sed|awk` pipelines.

## Current Framework Examples

Thin shell wrapper -> Python engine:

1. `_vida/scripts/todo-tool.sh` -> `_vida/scripts/todo-runtime.py`
2. `_vida/scripts/todo-sync-plan.sh` -> `_vida/scripts/todo-runtime.py`
3. `_vida/scripts/todo-plan-validate.sh` -> `_vida/scripts/todo-runtime.py`
4. `_vida/scripts/beads-verify-log.sh` -> `_vida/scripts/beads-verify-runtime.py`

Native Python runtime helpers:

1. `_vida/scripts/vida-config.py`
2. `_vida/scripts/subagent-system.py`

`_vida/scripts/vida-config.py` owns:

1. portable subset parsing for root overlay,
2. schema validation for canonical overlay sections,
3. fail-fast CLI/runtime helpers for overlay-dependent consumers.

Shell-first runtime adapters that remain shell-owned:

1. `_vida/scripts/beads-workflow.sh`
2. `_vida/scripts/quality-health-check.sh`
3. `_vida/scripts/beads-bg-sync.sh`
4. `_vida/scripts/br-safe.sh`

Rule:

1. shell-first adapters may call Python engines, but they remain the canonical place for runtime sequencing and lock-sensitive orchestration.

## Migration Rules

When migrating a framework script from shell logic to Python:

1. identify the canonical caller surface first,
2. preserve CLI arguments and exit-code semantics,
3. move parsing/derivation/validation logic into Python,
4. keep the shell wrapper minimal and single-purpose,
5. re-run consumer scripts that depend on the command,
6. update this document and all linked framework references in the same change.

## Verification Expectations

Minimum proof for script-runtime migrations:

1. `python3 -m py_compile` passes for every new Python engine,
2. `bash -n` passes for every touched shell wrapper,
3. direct command smoke tests pass for the migrated command,
4. at least one real framework consumer path is verified after migration.

## Boundary Rule

This document covers framework-owned runtime script architecture only.

Do not place here:

1. project build commands,
2. app-specific delivery scripts,
3. product runtime runbooks,
4. subagent/model choices for project overlay.
