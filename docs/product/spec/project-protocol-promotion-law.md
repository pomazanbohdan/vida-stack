# Project Protocol Promotion Law

Status: active product law

Purpose: define how project-owned protocols are recognized by the runtime, how they remain distinct from sealed framework law, and under what bounded conditions they may be promoted from known project surfaces into compiled executable runtime bundles.

## 1. Core Distinction

Project-owned protocols exist in two different states:

1. `known project protocols`
   - the system knows they exist and may discover or reference them
2. `compiled executable project protocols`
   - the runtime has admitted them into executable bundle composition

Default rule:

1. a project protocol is known first,
2. it is not executable merely because it exists.

## 2. Why Promotion Exists

The runtime needs this rule because:

1. projects may create many protocols,
2. not all of them should control execution,
3. executable runtime posture must remain bounded, validated, and fail-closed.

## 3. Promotion Stages

One project protocol may move through these stages:

1. `registered`
2. `mapped`
3. `validated`
4. `bound`
5. `compiled`
6. `executable`

## 4. Stage Meanings

### 4.1 Registered

The system has a recognized project protocol artifact and can discover it.

### 4.2 Mapped

The protocol has:

1. an owner/project location,
2. discovery/map presence,
3. a known project relation to runtime behavior.

### 4.3 Validated

The runtime can confirm:

1. the artifact resolves,
2. the project configuration lawfully references it,
3. it does not violate sealed framework boundaries.

### 4.4 Bound

The protocol is now attached to at least one runtime use point such as:

1. a role or lane posture,
2. a flow,
3. a gate,
4. an output/render class,
5. a command/init path.

### 4.5 Compiled

The protocol has been admitted into the machine-readable runtime bundle.

### 4.6 Executable

The runtime may now use the promoted protocol directly during execution.

## 5. Promotion Requirements

A project protocol may enter compiled execution only when all required inputs exist:

1. discovery/map evidence,
2. explicit trigger or use point,
3. lawful runtime binding,
4. gate and evidence posture where needed,
5. fail-closed validation,
6. no conflict with sealed framework/system law.

## 6. Framework Boundary

Promotion must not allow project protocols to replace:

1. core protocols,
2. system orchestration law,
3. sealed framework safety protocols,
4. framework-owned routing/gate invariants.

Project protocols may be promoted only where the framework has left lawful project-owned extension room.

## 7. Release-1 Rule

For Release 1:

1. framework protocols compile always,
2. project protocol promotion remains bounded and selective,
3. many project protocols may remain known but non-executable,
4. the runtime must distinguish these two cases explicitly.

## 8. Non-Promotion Case

Not every project instruction-like surface should become an executable protocol.

Example rule:

1. project team management or descriptive project guidance may stay configuration/discovery data rather than entering executable bundle law,
2. descriptive presence alone is not enough for promotion.

## 9. Failure Rule

If a project protocol cannot pass promotion requirements:

1. it remains known,
2. it does not compile,
3. execution must not assume it is active,
4. the system should surface the reason rather than silently dropping it.

## 10. Completion Proof

This model is closed enough for Release 1 when:

1. the runtime clearly distinguishes known versus compiled project protocols,
2. promotion requires explicit validation and binding,
3. framework/system law remains protected,
4. promoted project protocols can enter executable bundles only through bounded lawful admission.

-----
artifact_path: product/spec/project-protocol-promotion-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/project-protocol-promotion-law.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-12T07:48:27+02:00'
changelog_ref: project-protocol-promotion-law.changelog.jsonl
