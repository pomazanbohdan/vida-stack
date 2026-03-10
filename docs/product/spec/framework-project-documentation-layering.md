# VIDA Framework / Project Documentation Layering

Status: active product law

Purpose: define the canonical layering model for framework documentation, agent-role documentation, bootstrap/environment documentation, human-governance documentation, and project documentation so that VIDA remains framework-first while project-specific documentation remains clearly separated.

## Core Model

The repository must be understood through distinct documentation/control layers.

The canonical split is:

1. `Framework Canon`
2. `Agent Role Layer`
3. `Bootstrap / Environment Layer`
4. `Human Governance Layer`
5. `Project Documentation Layer`

These layers must not be collapsed into one mixed instruction surface.

## Layer Definitions

### 1. Framework Canon

Framework canon is the source of truth for VIDA behavior independent of one local agent shell or one project.

It owns:

1. laws,
2. states,
3. transitions,
4. receipts,
5. schemas,
6. command semantics,
7. routing and escalation law,
8. approval and verification behavior,
9. compact/resume behavior.

Canonical active home:

1. `vida/config/**`
2. active framework maps and instruction canon under `vida/config/instructions/**`

Framework canon must not be treated as a local prompt bundle or editor-specific helper layer.

### 2. Agent Role Layer

The agent role layer defines how one class of agent participates in the framework.

It owns:

1. role contracts,
2. allowed actions,
3. responsibility boundaries,
4. handoff rules,
5. escalation behavior,
6. role-specific interaction contracts.

This layer is derived from framework canon and must not become a competing source of system truth.

### 3. Bootstrap / Environment Layer

The bootstrap/environment layer defines how the current working environment starts and hosts the agent/runtime.

It owns:

1. bootstrap carriers,
2. environment-specific entrypoints,
3. local CLI/runtime integration,
4. dev-environment notes,
5. temporary implementation adapters,
6. execution-shell ergonomics.

This layer must not become a second owner of framework law.

### 4. Human Governance Layer

The human governance layer defines contribution and approval rules for human-directed work.

It owns:

1. contribution rules,
2. edit rules,
3. approval rules,
4. migration policy,
5. versioning policy,
6. governance exceptions.

### 5. Project Documentation Layer

The project documentation layer defines the product/project that is being developed on top of VIDA.

It owns:

1. project/product maps,
2. product specs,
3. project process docs,
4. project memory docs,
5. project-facing documentation alignment and readiness views.

Canonical active homes:

1. `docs/product/**`
2. `docs/process/**`
3. `docs/project-memory/**`

## Framework-First Rule

VIDA must be framework-first.

That means:

1. framework truth is defined before role-specific agent instructions,
2. agent instructions are derived from framework semantics,
3. bootstrap/environment notes must not redefine framework law,
4. project documentation must not be treated as the owner of framework behavior.

Forbidden inversion:

1. an agent must not learn core VIDA behavior primarily from local operational files, ad hoc prompts, or project-only notes when the behavior belongs to the framework canon.

## Canonical Derivation Chain

The required derivation chain is:

1. spec,
2. law,
3. state/command semantics,
4. agent contract,
5. runtime behavior.

Operational summaries, bootstrap notes, and helper surfaces may point into this chain, but they must not replace it.

## Placement Rule

Documentation must live in the ownership layer that matches its actual function.

Rules:

1. framework truth belongs in framework-owned surfaces under `vida/config/**` and related canonical framework maps,
2. agent-role rules belong in the agent-role layer and must remain derived from framework truth,
3. bootstrap/environment notes may live in bootstrap surfaces, but only for environment-specific execution concerns,
4. project/product documentation belongs in `docs/**`,
5. one concept must have one canonical owner layer.

## Bootstrap Two-Map Rule

Initialization uses one bootstrap carrier step followed by one two-map initialization step.

Step 1:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`

Step 2:

1. framework map initialization,
2. project docs map initialization.

Rules:

1. `AGENTS.md` is the framework bootstrap carrier,
2. `AGENTS.sidecar.md` is the project docs bootstrap carrier,
3. framework map discovery must come from framework-owned map surfaces,
4. project document discovery must come from the project docs map,
5. neither map is optional during initialization.

## Root-Map Requirement

Each major root must expose one clear root map so that a single pass can orient an agent without broad repo scanning.

Required root-map outcomes:

1. the framework root map must expose canonical framework maps, protocols, templates, and route entrypoints,
2. the project root map must expose project maps, product specs, process docs, project-memory docs, and project document entrypoints,
3. each root map must distinguish owner surfaces from pointer surfaces,
4. each root map must route by task type rather than only list files.

## Duplication and Mixing Rule

The following are documentation defects:

1. one file mixing framework canon, role contract, bootstrap notes, and project guidance without a single clear owner layer,
2. the same rule appearing as active law in more than one layer,
3. local/bootstrap notes being treated as framework truth,
4. project documentation pointers being embedded as canonical framework-map content,
5. framework behavior being defined in project-only docs.

When such mixing is discovered, the bounded correction should:

1. identify the correct owner layer,
2. move law-bearing content to that owner,
3. leave only derived summaries or pointers in the other layers when still useful.

## Transitional Migration Rule

The current repository may still contain documentation shaped by the transitional `vida/config/instructions/**` architecture and the current `docs/**` layout.

During transition:

1. do not create a second active canon while restructuring,
2. normalize ownership and derivation before large directory migration,
3. prefer classification and map cleanup before large physical moves,
4. promote a new root or subtree only when it replaces ambiguity rather than adds another parallel source.

## Minimum Audit Questions

Documentation structure is considered healthy only when one route can answer all of the following:

1. what is the canonical state model,
2. what is the canonical transition/receipt law,
3. what are the agent roles and their allowed actions,
4. what runtime/bootstrap protocols connect those roles,
5. which files are bootstrap-only and which files are framework truth,
6. which files are project truth rather than framework truth.

If one-pass routing cannot answer these questions, the documentation/control plane remains under-structured.

-----
artifact_path: product/spec/framework-project-documentation-layering
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/framework-project-documentation-layering.md
created_at: '2026-03-10T05:25:00+02:00'
updated_at: '2026-03-10T05:25:00+02:00'
changelog_ref: framework-project-documentation-layering.changelog.jsonl
