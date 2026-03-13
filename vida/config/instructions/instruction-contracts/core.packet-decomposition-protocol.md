# Core Packet Decomposition Protocol

Purpose: define the framework-owned rule for shaping bounded packets, selecting the shallowest lawful leaf, and refining to deeper packet leaves only just in time.

## Scope

This protocol defines:

1. the bounded packet rule for orchestration,
2. the default leaf-depth rule,
3. when deeper refinement is required,
4. the prohibition on backlog-wide premature micro-splitting.

This protocol does not define:

1. one project's packet field names,
2. one specific backlog item,
3. one specific runtime-family task schema,
4. one role prompt body.

## Core Rule

Normal orchestration must route through the shallowest lawful bounded leaf.

Framework rule:

1. do not delegate feature-, milestone-, or paragraph-shaped work,
2. begin with one bounded packet that has one dominant goal, one owner, one proof target, and one stop boundary,
3. refine deeper only when one-owner closure still fails.

## Default Leaf Rule

The default orchestration leaf is the first bounded packet level that satisfies all of the following:

1. one dominant goal,
2. one bounded owner,
3. one bounded write scope or read-only scope,
4. one proof target or verification boundary,
5. one lane cycle can judge closure.

## Just-In-Time Refinement Rule

Refine to a deeper bounded packet only when at least one is true:

1. the current packet still crosses multiple mutable contracts,
2. the current packet mixes implementation and unrelated proof/seam closure,
3. the current packet still crosses more than one owner boundary,
4. `definition_of_done` remains too broad for one bounded cycle.

JIT rule:

1. deeper refinement is for the next active item or smallest near-critical-path set only,
2. do not pre-split the whole backlog into speculative deep leaves.

## Premature Micro-Splitting Rule

The following are forbidden by default:

1. converting the full backlog into deep packet trees before dispatch is near,
2. splitting future items only because deep leaves look tidier in prose,
3. treating backlog-wide micro-splitting as a substitute for lawful shaping.

## Packet Boundary Rule

A lawful packet must expose:

1. one bounded unit,
2. one owner,
3. one bounded scope,
4. one proof target,
5. one blocking question,
6. one explicit stop condition.

If any of these are missing, the packet is not ready and must be reshaped.

## Post-Leaf Rebuild Rule

1. When one bounded leaf closes and the parent chain remains open, the next action is to rebuild the parent bounded unit before any closure-style summary or route suspension.
2. Rebuild must determine exactly one of:
   - the next lawful bounded leaf,
   - an explicit blocker/escalation state,
   - fully closed parent chain
3. If the rebuild yields a coherent next lawful bounded leaf, that leaf becomes the required next route target rather than an optional future plan.
4. If the rebuild cannot yield a coherent next lawful bounded leaf, emit an explicit blocker or escalation receipt instead of a terminal-sounding summary.
5. Treating a closed leaf as implicit closure of the parent chain is protocol-invalid decomposition.

## Related

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
3. `vida/config/instructions/system-maps/protocol.index.md`

-----
artifact_path: config/instructions/instruction-contracts/core.packet-decomposition-protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md
created_at: '2026-03-13T22:00:00+02:00'
updated_at: '2026-03-13T22:00:00+02:00'
changelog_ref: core.packet-decomposition-protocol.changelog.jsonl
