# Protocol Consistency Audit Protocol

Purpose: define the canonical framework audit protocol for checking whether framework protocols remain internally consistent across ownership, activation, terminology, routing, and canonical discovery surfaces.

## Scope

This protocol applies when the task is to:

1. audit one protocol or one protocol family for internal consistency,
2. verify that protocol wording matches current owner boundaries and activation law,
3. check that maps, indexes, and canonical owner artifacts remain aligned,
4. detect duplicated law, stale routing, stale terminology, or missing protocol inventory,
5. correct bounded protocol drift inside active framework canon.

This protocol audits framework protocol artifacts themselves.

It does not replace:

1. `vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md` for runtime execution drift,
2. `vida/config/instructions/diagnostic-instructions/analysis.framework-self-analysis-protocol.md` for broader framework-wide friction diagnosis,
3. `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md` for authoring new protocols or major protocol restructuring.

## Trigger

Activate this protocol when at least one is true:

1. the user asks to review, verify, audit, or normalize framework protocols,
2. one protocol family must be checked for consistency against `core`, shell, activation, or domain maps,
3. protocol terminology, ownership, or canonical routing appears contradictory,
4. a protocol-category audit is being executed across several adjacent owner artifacts.

## Core Contract

Every protocol-consistency audit must verify all of:

1. `owner consistency`
   - the protocol owns only its bounded concern and does not absorb adjacent owner law,
2. `activation consistency`
   - the protocol's active/use wording matches `bridge.instruction-activation-protocol.md`,
3. `domain consistency`
   - the protocol is routed in the correct domain family and not misclassified as `core` or non-`core`,
4. `layer consistency`
   - the protocol's content matches the correct owner layer and does not drift into bootstrap, governance, runtime, or project-doc ownership,
5. `terminology consistency`
   - lane, worker, runtime, documentation, naming, and audit vocabulary matches current framework canon,
6. `inventory consistency`
   - protocol-bearing artifacts, maps, and indexes remain synchronized,
7. `reference consistency`
   - canonical references point to current owners rather than stale or migration-only surfaces,
8. `closure consistency`
   - validation and proof requirements match the touched owner surfaces.

## Audit Sequence

Run the audit in this order:

1. identify the protocol family and bounded audit scope,
2. open the current canonical owner artifacts,
3. compare each artifact against:
   - owner-layer routing,
   - activation law,
   - domain-family routing,
   - canonical protocol index wiring,
4. classify each issue as one of:
   - `owner_drift`
   - `activation_drift`
   - `domain_inventory_gap`
   - `terminology_drift`
   - `reference_drift`
   - `duplicate_law`
   - `validation_drift`
5. correct bounded drift in the same work cycle when safe,
6. validate the touched scope before closure.

## Required Comparison Surfaces

Minimum comparison set:

1. `vida/config/instructions/system-maps/framework.protocol-layers-map.md`
2. `vida/config/instructions/system-maps/framework.protocol-domains-map.md`
3. `vida/config/instructions/system-maps/protocol.index.md`
4. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
5. the active protocol-bearing artifacts under audit

Use adjacent owner artifacts when the audit depends on a live boundary with:

1. `core`
2. orchestration shell
3. runtime execution
4. documentation canon
5. diagnostics
6. artifact and naming governance

## Boundary Rules

1. Do not create a new protocol merely because wording feels weak; first verify whether an existing canonical owner already covers the concern.
2. Do not leave contradictory owner wording active across sibling protocols when the drift can be corrected within the current bounded scope.
3. Do not treat migration-only helpers, transition maps, changelog entries, or generated status artifacts as primary owner law.
4. If the audit reveals a missing bounded owner domain, create a new protocol only when no current canonical owner can lawfully absorb the concern.

## Validation Rule

Protocol-consistency audit work is closed only when:

1. `check` passes on the changed scope,
2. `activation-check` passes when activation wording or protocol-bearing routing changed,
3. `protocol-coverage-check --profile active-canon` passes when protocol-bearing inventory or index rows changed,
4. `doctor --profile active-canon-strict` passes when canonical protocol artifacts or maps changed,
5. `proofcheck --profile active-canon-strict` passes for cross-layer or multi-artifact protocol-audit work,
6. `readiness-check --profile active-canon` passes when readiness-facing or boot-gate surfaces changed.

## Output Contract

The audit result must make visible:

1. audited protocol family or scope,
2. checked owner surfaces,
3. findings grouped by drift class,
4. bounded fixes applied,
5. residual watchpoints if any remain.

## Fail-Closed Rule

1. Do not report a protocol family as consistent when owner, activation, or index wiring still conflict.
2. Do not leave a touched protocol-bearing artifact out of `protocol.index.md`.
3. Do not treat partially corrected category drift as green closure.

## References

1. `vida/config/instructions/diagnostic-instructions/analysis.framework-self-analysis-protocol.md`
2. `vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md`
3. `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md`
4. `docs/process/framework-three-layer-refactoring-audit.md`

-----
artifact_path: config/diagnostic-instructions/protocol-consistency-audit.protocol
artifact_type: diagnostic_instruction
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md
created_at: '2026-03-12T11:55:00+02:00'
updated_at: '2026-03-12T11:55:06+02:00'
changelog_ref: analysis.protocol-consistency-audit-protocol.changelog.jsonl
