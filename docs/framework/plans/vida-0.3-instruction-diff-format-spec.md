# VIDA 0.3 Instruction Diff Format Spec

Purpose: define the canonical structured diff format used by instruction sidecars to alter the effective projection of immutable base instruction artifacts.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

Instruction updates are represented as structured diffs, not raw free-text rewrites and not in-place edits of immutable base artifacts.

The canonical model is:

1. immutable base instruction artifact,
2. one or more sidecar diff records,
3. deterministic projection merge,
4. one consistent effective output.

Compact rule:

`instruction customization uses structured diff sidecars over immutable base artifacts`

---

## 2. Why Structured Diff And Not Raw Unified Diff

Raw unified diff is useful as a human-readable export, but not as the canonical runtime format.

Structured diff is preferred because:

1. it is easier to validate,
2. it is more stable across migrations,
3. it supports anchor-aware rebinding,
4. it is easier for LLMs and tools to author deterministically,
5. it fits DB storage and query patterns better.

---

## 3. Canonical Diff Record

Each instruction diff record must contain at minimum:

1. `patch_id`
2. `target_artifact_id`
3. `target_artifact_version`
4. `target_artifact_hash` or equivalent integrity token
5. `patch_precedence`
6. `operations[]`
7. `author_class`
8. `applies_if`
9. `created_at`
10. `active`

---

## 4. Canonical Operations

Minimum instruction diff operations:

1. `delete_range`
2. `replace_range`
3. `insert_before`
4. `insert_after`
5. `replace_with_many`
6. `append_block`
7. `insert_section`
8. `deactivate_patch`

Rules:

1. one operation may replace one source segment with multiple output lines,
2. delete and replace must target an explicit anchor,
3. insert must declare whether it applies before or after the anchor,
4. augmentation operations may add new lines or sections without deleting base content,
5. patch operations must be deterministic and replayable.

---

## 5. Targeting Modes

Allowed targeting modes:

1. `anchor_hash`
2. `exact_text`
3. `line_span`

Resolution order:

1. `anchor_hash`
2. `exact_text`
3. `line_span`

Rule:

1. `line_span` must not be the only integrity mechanism for long-lived patches,
2. anchor-aware targeting is required for migration safety.

---

## 6. Projection Merge Law

The projection engine must:

1. load immutable base artifact,
2. select active applicable sidecar diffs,
3. sort by precedence and deterministic tie-breaker,
4. validate anchors,
5. apply operations in deterministic order,
6. produce one consistent effective projection,
7. record a projection receipt.

The projection result is:

1. runtime-usable,
2. inspectable,
3. reproducible,
4. not a replacement for the immutable base records.

---

## 7. Conflict Rules

Conflict exists when:

1. two active diffs modify the same anchor incompatibly,
2. a patch targets an anchor that no longer exists,
3. precedence does not resolve the contradiction cleanly.

Conflict handling:

1. higher-precedence patch may suppress lower-precedence patch when explicitly allowed,
2. unresolved conflict must fail closed,
3. silent best-effort merging is forbidden.

---

## 8. Migration And Rebind

When the base artifact version changes:

1. existing diffs must be revalidated,
2. anchor-based rebinding may occur only when deterministic,
3. failed rebinding must deactivate or block the patch explicitly,
4. the runtime must not silently apply a patch to the wrong segment.

---

## 9. Export Rule

Human-readable unified diff may be exported for review/debug, but the canonical runtime form remains the structured diff record.
-----
artifact_path: framework/plans/vida-0.3-instruction-diff-format-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-instruction-diff-format-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-instruction-diff-format-spec.changelog.jsonl
P26-03-09T21: 44:13Z
