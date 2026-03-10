# AGENTS Sidecar

Purpose: provide current project/runtime context and canonical map pointers for bootstrap without overloading `AGENTS.md`.

## Project Context

1. Repository: `vida-stack`
2. Current direction:
   - VIDA `0.2.0` is the proving and continuation runtime line
   - VIDA `1.0.0` is the target durable runtime line
   - both lines are expected to converge on one shared semantic runtime spine
3. Active instruction canon:
   - `vida/config/instructions/*`
4. Current transitional implementation:
   - `taskflow-v0/*`
5. Active product/process/project documentation:
   - `docs/product/*`
   - `docs/process/*`
   - `docs/project-memory/*`

## Transitional Runtime Note

1. VIDA `0.2.0` is currently under active development.
2. The target task-work mechanism is the canonical TaskFlow.
3. This target task mechanism is not yet working as the intended default operational path in the current transitional state.
4. Until it is explicitly brought online and re-authorized, ignore instructions that assume the canonical TaskFlow task mechanism is already functioning as the default task path.

## Transitional Information-System Note

1. The canonical VIDA 1 documentation, instruction, and inventory architecture is defined by product specs and `vida/config/**`.
2. The current repository tooling `codex-v0/codex.py` is the transitional `0.2.0` implementation substrate for that information-system layer.
3. Treat `codex` in the same architectural posture that `taskflow-v0/**` occupies for the runtime layer:
   - useful and active now,
   - implementation-facing,
   - not the authority that defines the final VIDA 1 architecture.
4. When product specs and current `codex` behavior diverge, the product spec wins and `codex` must be corrected.
5. The development goal for `codex` is dual:
   - use the `codex` system to operate the project itself,
   - evolve `codex` layer by layer according to the canonical VIDA specification.
6. `codex` development must follow the layered closure rule from the product spec:
   - each completed layer must be independently useful,
   - each next layer may deepen only what lower layers already close,
   - do not depend on future-layer capability to justify current-layer behavior.
7. For every new layer, development is documentation-first:
   - first bring the canonical layer documentation into shape,
   - then implement against that documented law,
   - do not justify new implementation behavior with undocumented layer rules.
8. Gaps discovered inside already-green layers must be corrected immediately when the fix is bounded and safe:
   - do not defer a green-layer usability, validation, or mutation-path defect merely because the main task can continue,
   - fix the gap in the same work cycle unless doing so would widen scope materially.
9. `Layer 8: Canonical Runtime Consumption` is directly blocked on `taskflow`:
   - `codex` can prepare inventory, readiness, and bundle authority,
   - but only `taskflow` becoming the primary runtime consumer can close that layer.
10. Documentation formatting authority resolves top-down:
   - active skill-specific artifact format,
   - explicit project documentation standard,
   - promoted product-law documentation contract,
   - `codex` fallback behavior.
11. `codex` defaults must not override a higher-precedence artifact standard.
12. Deduplicate active law whenever a bounded documentation task safely can:
   - one canonical rule should have one canonical home,
   - maps and pointers may summarize it, but must not become a second law-bearing source.

## Canonical Maps

1. Framework/documentation architecture map:
   - `vida/config/instructions/system-maps.framework-map-protocol.md`
2. Domain protocol registry:
   - `vida/config/instructions/system-maps.protocol-index.md`
3. Product spec map:
   - `docs/product/spec/current-spec-map.md`
4. Project documentation system:
   - `docs/product/spec/project-documentation-system.md`
5. Canonical runtime readiness law:
   - `docs/product/spec/canonical-runtime-readiness-law.md`

## Runtime Orientation

1. Bootstrap router:
   - `AGENTS.md`
2. Orchestrator entry:
   - `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. Worker entry:
   - `vida/config/instructions/agent-definitions.worker-entry.md`
4. Worker thinking subset:
   - `vida/config/instructions/instruction-contracts.worker-thinking.md`

## Working Rule

1. Use `AGENTS.md` for lane routing and hard invariants.
2. Use this sidecar for project context, map discovery, and current repository orientation.
3. Prefer canonical maps and instruction artifacts over broad manual repo scanning when bootstrapping context.
4. When the active task is documentation-shaped, treat `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md` as one-touch active context, not as an optional later read.

## Repository Documentation Commands

Read/status commands:

1. `overview [--profile <name>]`
   - one-command documentation state overview with totals and current issues/warnings
2. `layer-status --layer <N> [--format toon|jsonl]`
   - show the current layer plus adjacent layers from the canonical layer matrix
3. `doctor --layer <N> [--format toon|jsonl]`
   - run bounded doctor checks only for the canonical documentation scope of one layer
4. `proofcheck --layer <N> [--format toon|jsonl]`
   - run the bounded closure proof path only for one canonical layer scope
5. `summary --root <dir> [--format toon|jsonl]`
   - compact totals by layer, owner, and status
6. `registry --root <dir>`
   - one machine-readable row per markdown artifact
7. `registry-write --root <dir> [--output <file>] [--canonical]`
   - materialize one registry snapshot for later automation or write the canonical registry path
8. `scan --root <dir> [--missing-only]`
   - per-file latest-state rows
9. `changelog <file> [--limit N] [--newest-first] [--format toon|jsonl]`
   - one artifact history
10. `changelog-task --root <dir> <task_id> [--limit N] [--newest-first] [--format toon|jsonl]`
   - all matching history rows for one task id
11. `task-summary --root <dir> --task-id <id> [--format toon|jsonl]`
   - aggregate task-level history summary
12. `deps <file> [--format toon|jsonl]`
   - direct footer refs, markdown links, and reverse mentions
13. `deps-map <file-or-dir> [--format toon|jsonl]`
   - graph-style dependency edge inventory for one file or a whole scope
14. `artifact-impact [--file <file> | --artifact <artifact_path>] [--format toon|jsonl]`
   - show all direct document impacts for one artifact identity
15. `task-impact --root <dir> --task-id <id> [--format toon|jsonl]`
   - show indirect documentation impacts around artifacts touched by one task
16. `links <file-or-dir> [--format toon|jsonl]`
   - markdown-link inventory for one file or a whole scope

Mutation/finalization commands:

1. `touch <file> <note> [--event ...] [--task-id ...] [--actor ...] [--scope ...] [--tags ...]`
   - update `updated_at` and append changelog in one step
2. `finalize-edit <file> [<file> ...] <note> [--status ...] [--artifact-version ...] [--artifact-revision ...] [--set key=value ...]`
   - finalize one or more prior manual diff edits across one or more files with one metadata/changelog update per file
3. `init <file> <artifact_path> <artifact_type> <note> [...]`
   - initialize a canonical markdown artifact with footer and sidecar changelog
4. `move <file> <destination> <note> [...]`
   - relocate one artifact plus its sidecar changelog
5. `rename-artifact <file> <artifact_path> <note> [...]`
   - change canonical artifact identity without moving the file
6. `migrate-links <file-or-dir> <old_target> <new_target> <note> [--dry-run] [--format toon|jsonl] [...]`
   - exact markdown-link rewrite with preview, summary, changelog, and validation

Validation commands:

1. `check --root <dir> [files...]`
   - footer and sidecar health checks
2. `fastcheck --root <dir> [files...]`
   - quick bounded validation for footer, relation refs, and broken links
3. `activation-check --root <dir> [files...]`
   - verify that changed protocol surfaces still have lawful activation bindings
4. `protocol-coverage-check --profile active-canon [--format toon|jsonl]`
   - verify that canonical protocol-bearing artifacts still appear in the protocol index and still have valid activation coverage
5. `readiness-check --profile active-canon [--format toon|jsonl]`
   - verify version tuples, compatibility classes, projection parity, bundle completeness, and boot-gate artifact presence
6. `readiness-write --profile active-canon [--canonical]`
   - materialize the current bounded readiness report to one shared artifact path
7. `proofcheck --profile active-canon-strict [files...] [--layer <N>] [--format toon|jsonl]`
   - run the bounded closure proof path for fastcheck, protocol coverage, readiness, and strict doctor in one command, either by explicit files or one canonical layer scope
8. `doctor --root <dir> [--show-warnings] [--format toon|jsonl] [--fail-on-warnings] [--layer <N>]`
   - stronger consistency checks for metadata, changelogs, links, relation refs, and policy exceptions

Operational rule:

1. if one documentation change needs multiple diff operations, perform the diff edits first and run exactly one `finalize-edit` afterward,
2. prefer one changelog entry per logical document edit batch, not one entry per low-level diff step,
3. history and human-facing map commands default to `toon`; use `--format jsonl` when machine-readable output is needed,
4. machine-oriented plumbing commands such as `scan`, `registry`, `registry-write`, and `check` stay `jsonl`-first,
5. command result status is printed at command end with leading emoji:
   - `✅ OK`
   - `❌ ERROR`
6. use `fastcheck` during active editing for quick bounded safety and `doctor` before closure of canonical changes,
7. use `activation-check` whenever protocol activation wiring changed,
8. use `protocol-coverage-check` whenever canonical protocol rows, protocol-bearing artifacts, or activation-index wiring changed,
9. use `readiness-check` whenever readiness law, projection parity, canonical bundle shape, compatibility classes, or boot-gate artifacts changed,
10. use `readiness-write --canonical` when downstream automation or runtime proving needs the current shared readiness artifact,
11. prefer `proofcheck --layer <N>` when the active documentation work is tightly bounded to one canonical layer,
12. prefer `proofcheck --profile active-canon-strict` when the change spans multiple active-canon layers,
13. use `doctor --layer <N>` for bounded layer audits and `doctor --profile ...` for cross-layer audits,
14. use `layer-status` when the active work is organized around one canonical layer and its adjacent layers.

-----
artifact_path: project/repository/agents.sidecar
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: AGENTS.sidecar.md
created_at: '2026-03-10T02:13:40+02:00'
updated_at: '2026-03-10T04:25:26+02:00'
changelog_ref: AGENTS.sidecar.changelog.jsonl
