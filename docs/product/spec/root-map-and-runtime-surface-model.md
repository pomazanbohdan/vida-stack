# VIDA Root Map And Runtime Surface Model

Status: active product law

Purpose: define the root-map architecture for VIDA and the canonical treatment of multiple bounded runtime surfaces so that `DocFlow`, `taskflow`, and future runtime families remain independently usable while also being unified under one discoverable VIDA framework map.

## 1. Core Requirement

VIDA must expose one-pass discoverability at the root-map level.

An agent must be able to initialize framework and project understanding without broad repo scanning by reading:

1. bootstrap carriers,
2. framework root map,
3. project root map.

These maps must expose the critical knowledge needed to route into deeper protocol, template, runtime, and project-document surfaces.

## 2. Runtime Surface Family Rule

VIDA may contain multiple bounded runtime surfaces.

Current known runtime surfaces include:

1. `DocFlow`
2. `taskflow`

Future runtime surfaces are expected.

Rules:

1. each runtime surface must be independently understandable as its own bounded subsystem,
2. each runtime surface must also be discoverable as part of the unified VIDA framework,
3. no runtime surface may become the hidden owner of framework-wide truth unless canon explicitly promotes it,
4. the framework root map must expose all active runtime families and where their canonical docs/maps live,
5. adding a new runtime family requires updating the framework root map rather than relying on ad hoc file discovery.

## 3. Independent And Unified Posture

For every runtime family such as `DocFlow`, `taskflow`, or a future runtime:

1. it must have a bounded identity,
2. it must have a canonical documentation/map surface of its own,
3. it must have a clear relationship to framework canon,
4. it must have a clear relationship to project-facing docs when applicable,
5. it must be readable both:
   - as a standalone runtime surface,
   - and as one member of the broader VIDA runtime family.

Forbidden pattern:

1. a runtime family is discoverable only by scattered file paths or historical habit rather than through a canonical root-map route.

## 4. Root-Map Stack

The required root-map stack is:

1. bootstrap carriers,
2. framework root map,
3. project root map,
4. runtime-family submaps,
5. template map.

### 4.1 Bootstrap Carriers

Bootstrap carriers remain:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`

They start initialization but must not absorb the full map layer.

### 4.2 Framework Root Map

The framework root map must expose:

1. canonical framework maps,
2. protocol registry,
3. role-layer entrypoints,
4. bootstrap/environment surfaces,
5. governance surfaces,
6. runtime-family map entrypoints,
7. template-map entrypoint,
8. activation-trigger guidance,
9. owner-vs-pointer boundaries.

### 4.3 Project Root Map

The project root map must expose:

1. project product maps,
2. project specs,
3. project process docs,
4. project-memory docs,
5. project-facing template surfaces when they exist,
6. activation-trigger guidance for project-document tasks,
7. owner-vs-pointer boundaries.

### 4.4 Runtime-Family Submaps

Each runtime family must have a bounded map surface that exposes:

1. runtime purpose,
2. ownership boundary,
3. canonical docs and protocols,
4. related executable/config surfaces,
5. related templates,
6. related project-facing dependencies when applicable,
7. activation triggers.

For `VIDA 1.0`, each runtime-family map should also be able to expose:

1. checkpoint/recovery ownership,
2. replay-safe transition boundaries,
3. history/context shaping policy for cross-lane handoff,
4. health/observability entrypoints,
5. idempotency-sensitive mutation or proof surfaces when they exist.

### 4.5 Template Map

There must be a template map surface.

It must expose:

1. template families,
2. template owners,
3. where templates live,
4. which surfaces consume them,
5. which triggers require reading the template map,
6. which templates are framework-owned versus project-owned.

### 4.6 Current Canonical Implementation Paths

The current canonical root-map stack is implemented as:

1. framework root map:
   - `vida/root-map.md`
2. project root map:
   - `docs/project-root-map.md`
3. governance map:
   - `vida/config/instructions/system-maps/governance.map.md`
4. runtime-family index:
   - `vida/config/instructions/system-maps/runtime-family.index.md`
5. runtime-family maps:
   - `vida/config/instructions/system-maps/runtime-family.docflow-map.md`
   - `vida/config/instructions/system-maps/runtime-family.taskflow-map.md`
6. template map:
   - `vida/config/instructions/system-maps/template.map.md`
7. project-owned documentation tooling map:
   - `docs/process/documentation-tooling-map.md`
8. observability map:
   - `vida/config/instructions/system-maps/observability.map.md`

Rule:

1. future runtime families must follow this same structure rather than creating ad hoc discovery paths.

## 5. Critical Knowledge Requirement

Top-level maps must carry critical routing knowledge, not only file lists.

At minimum a root map must answer:

1. what lives here canonically,
2. what is only a pointer,
3. what protocols exist here,
4. where templates are,
5. what runtime families exist,
6. what triggers activate the need to read deeper surfaces,
7. where to go next for related surfaces.

## 6. Activation Trigger Requirement

Every root map or submap must expose explicit activation triggers.

The trigger surface must answer:

1. when this map must be read,
2. what task shapes activate it,
3. what related maps or protocols should be read next,
4. what not to read unless a trigger requires it.

Forbidden pattern:

1. a map exists but gives no activation guidance, forcing broad rereads or guesswork.

## 7. Placement Rule

Map placement must match ownership.

Rules:

1. the framework root map belongs to the framework-owned layer,
2. the project root map belongs to the project documentation layer,
3. runtime-family submaps belong to the ownership layer of the runtime family they describe,
4. the template map belongs to the layer that canonically owns template discovery,
5. no map should simultaneously act as framework root map, project root map, and runtime-family map.

## 8. Optimization Rule

Structure optimization must improve discoverability without creating a second canon.

During optimization:

1. keep one canonical root-map route,
2. prefer extracting missing map surfaces before moving large trees,
3. normalize owner layers before directory renames,
4. ensure `DocFlow`, `taskflow`, and future runtimes remain separately intelligible,
5. ensure the unified VIDA framework root map remains the top routing surface for framework discovery.

## 9. Minimum Outcomes

The structure is considered healthy only when one bounded read path can tell an agent:

1. what the framework root is,
2. what the project root is,
3. what runtime families exist,
4. where templates live,
5. which protocols exist,
6. which triggers activate which maps,
7. where the next related surfaces live.

If these questions cannot be answered through the root-map stack, the structure remains under-optimized.

## 10. VIDA 1.0 External-Alignment Targets

The `VIDA 1.0` runtime-family and root-map model should remain compatible with the following external architectural patterns:

1. supervisor-driven bounded handoffs rather than broad shared chat context,
2. explicit context/history filtering per receiving lane,
3. hierarchical multi-agent supervision when one top-level orchestrator is not enough,
4. durable execution with restart/resume safety,
5. checkpoint-based recovery and replay-safe transitions,
6. observability/health surfaces that remain discoverable through canonical maps.

Target implications:

1. future runtime families must remain independently understandable while still being attachable to one supervisor/orchestrator model,
2. runtime-family discovery must not hide which family owns:
   - durable state,
   - checkpointing,
   - replay/recovery,
   - verification/proving,
   - observability/health,
3. root maps must remain able to route an agent toward those surfaces without broad repo scanning.

External alignment references:

1. OpenAI Agents SDK overview:
   - https://developers.openai.com/api/docs/guides/agents-sdk
2. OpenAI Agents SDK handoffs:
   - https://openai.github.io/openai-agents-js/guides/handoffs/
3. LangGraph supervisor:
   - https://langchain-ai.github.io/langgraphjs/reference/modules/langgraph-supervisor.html
4. Temporal documentation:
   - https://docs.temporal.io/
5. Eventuous checkpoints:
   - https://eventuous.dev/docs/subscriptions/checkpoint/
6. Eventuous subscription diagnostics:
   - https://eventuous.dev/docs/subscriptions/subs-diagnostics/

-----
artifact_path: product/spec/root-map-and-runtime-surface-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/root-map-and-runtime-surface-model.md
created_at: '2026-03-10T05:42:00+02:00'
updated_at: '2026-03-10T23:52:08+02:00'
changelog_ref: root-map-and-runtime-surface-model.changelog.jsonl
