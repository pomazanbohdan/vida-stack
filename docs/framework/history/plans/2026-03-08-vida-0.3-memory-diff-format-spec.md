# VIDA 0.3 Memory Diff Format Spec

Purpose: define the canonical diff/update format for project-memory and framework-memory records.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

Memory updates are record-level diffs, not raw text patching.

The canonical model is:

1. existing memory records,
2. structured update records,
3. deterministic resolution into current memory state.

---

## 2. Applicable Slices

This format applies to:

1. `project_memory`
2. `framework_memory`

It does not define text-level instruction sidecar diffs.

---

## 3. Canonical Operations

Minimum operations:

1. `append_entry`
2. `supersede_entry`
3. `mark_stale`
4. `reclassify_entry`
5. `merge_entries`
6. `attach_note`
7. `deactivate_entry`

---

## 4. Required Fields

Each memory diff record must include:

1. `update_id`
2. `target_slice`
3. `target_record_ids`
4. `operation`
5. `payload`
6. `author_class`
7. `created_at`
8. `active`

---

## 5. Resolution Rules

1. Memory resolution must be deterministic.
2. Superseded records remain inspectable as history.
3. Deactivated or stale entries must not masquerade as current truth.
4. Silent destructive overwrite is forbidden.
