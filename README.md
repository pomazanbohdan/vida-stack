# Vida Stack

Purpose: explain the repository-level product narrative, current direction, and transitional architecture for Vida Stack.

Vida Stack is the active development repository for `VIDA 0.2.0` and the reference architecture for `VIDA 1.0`.

The repository currently has two explicit transitional implementation lines:

1. `taskflow-v0/`
   - the current `0.2.0` implementation substrate for runtime, task execution, routing, orchestration, verification, and operational state,
2. `codex-v0/`
   - the current `0.2.0` implementation substrate for documentation, instruction, inventory, validation, lineage, and documentation-side operations.

These are active and useful now, but neither defines the final authority of `VIDA 1.0` on its own. Product authority remains in:

1. `docs/product/spec/**`
2. `vida/config/**`
3. active instruction canon in `vida/config/instructions/**`

## Why This Repository Exists

Vida Stack exists to build a real control plane for agent-driven product engineering rather than a loose collection of prompts, scripts, or ad hoc helpers.

The target system must unify:

1. task execution,
2. routing and orchestration,
3. verification and approval,
4. memory and durable state,
5. instruction and command law,
6. canonical documentation and inventory,
7. migration and compatibility control.

## Current Transitional Architecture

The current repository is organized around four active surfaces:

1. `AGENTS.md`
   - bootstrap router and `L0` lane/invariant surface,
2. `AGENTS.sidecar.md`
   - current project context, map pointers, and operational notes,
3. `taskflow-v0/**`
   - transitional runtime substrate,
4. `codex-v0/**`
   - transitional documentation/inventory substrate.

Supporting canonical surfaces:

1. `vida/config/instructions/**`
   - active framework instruction canon,
2. `docs/product/spec/**`
   - promoted product law,
3. `docs/framework/plans/**`
   - active strategic and execution-spec program layer,
4. `docs/framework/research/**`
   - active research layer.

## TaskFlow-V0

`taskflow-v0` is the current proving runtime for `VIDA 0.2.0`.

Its job is to validate and harden:

1. task state and execution flow,
2. routing and route-law enforcement,
3. worker/orchestrator cooperation,
4. verification and approval gates,
5. runtime memory and state artifacts,
6. the behavioral spine that `VIDA 1.0` must preserve.

It is not the final `VIDA 1.0` runtime. It is the current implementation substrate that proves the runtime law in practice.

## Codex-V0

`codex-v0` is the current proving information-system for `VIDA 0.2.0`.

Its job is to validate and harden:

1. canonical document metadata,
2. sidecar lineage and changelogs,
3. inventory and registry generation,
4. validation and consistency gates,
5. lawful documentation mutation,
6. dependency and impact analysis,
7. documentation-first layer development.

Like `taskflow-v0`, it is not the final `VIDA 1.0` implementation. It is the current implementation substrate for the documentation/instruction/inventory side of the system.

## VIDA 1.0 Direction

`VIDA 1.0` is the target durable local binary line.

The current canonical direction is:

1. `taskflow` and `codex` must become separate bounded crates,
2. each must work independently as:
   - a reusable library,
   - its own CLI tool,
3. the top-level `vida` binary must compose them rather than collapse them back into one monolith.

That means:

1. `taskflow`
   - owns runtime/task execution behavior,
2. `codex`
   - owns canonical documentation/instruction/inventory behavior,
3. `vida`
   - composes those capabilities into one product operator surface.

## Current Working Rule

Development in this repository follows a documentation-first rule:

1. when a new layer or rule is introduced, canonical documentation is brought into shape first,
2. only after that may implementation be changed,
3. if implementation and spec diverge, the spec wins and the implementation must be corrected.

Layer closure is also incremental:

1. each completed layer must already provide standalone value,
2. each next layer may deepen only what lower layers already close,
3. future-layer assumptions must not be used to justify current-layer behavior.

## Current Maps

Primary orientation surfaces:

1. [AGENTS.md](/home/unnamed/project/vida-stack/AGENTS.md)
2. [AGENTS.sidecar.md](/home/unnamed/project/vida-stack/AGENTS.sidecar.md)
3. [framework-map-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.framework-map-protocol.md)
4. [protocol-index.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.protocol-index.md)
5. [current-spec-map.md](/home/unnamed/project/vida-stack/docs/product/spec/current-spec-map.md)
6. [canonical-documentation-and-inventory-layers.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-documentation-and-inventory-layers.md)

## Version Path

The versioned product path is defined in [VERSION-PLAN.md](/home/unnamed/project/vida-stack/VERSION-PLAN.md).

Core licensing is provided under [LICENSE](/home/unnamed/project/vida-stack/LICENSE).

-----
artifact_path: project/repository/readme
artifact_type: repository_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: README.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-10T03:39:28+02:00'
changelog_ref: README.changelog.jsonl
