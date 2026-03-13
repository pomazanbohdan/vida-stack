# Project Skill Initialization And Activation Protocol

Status: active project process doc

Purpose: define the mandatory project-side initialization and activation rule for available skills so orchestrator and delegated agents always inspect the current skill catalog and attach the relevant skills before starting bounded work.

## Scope

This protocol defines:

1. the mandatory skill initialization step after bootstrap,
2. how available skills are discovered,
3. when a skill must be activated,
4. how skill activation relates to packet shaping and agent dispatch.

This protocol does not define:

1. framework lane law,
2. one specific skill body,
3. one specific task packet,
4. permanent activation of all skills at once.

## Core Rule

Available skills must be inspected before normal project work begins.

Project rule:

1. skill initialization is mandatory after bootstrap for development orchestration and delegated worker lanes,
2. only the relevant skills should be activated for the current bounded work,
3. the agent must not skip an applicable skill merely because the task looks familiar.

## Skill Discovery Source

The active skill catalog is the current session-visible skill list exposed by bootstrap/runtime context.

Discovery rule:

1. treat the visible skill list as the current source of available skills,
2. do not assume a skill exists unless it is present in the active catalog,
3. if a listed skill is relevant, read its `SKILL.md` before proceeding with bounded work.

## Mandatory Initialization Step

After bootstrap and before packet shaping or packet execution:

1. inspect the active skill catalog,
2. identify whether any available skill matches the current bounded work,
3. read the relevant `SKILL.md` bodies,
4. record or state which relevant skills are active for the current bounded step,
5. continue only after the relevant skill set is known.

If no available skill applies:

1. explicitly treat the active step as `no_applicable_skill`,
2. continue under the normal protocol stack.

## Activation Rule

A skill must be activated when either is true:

1. the user explicitly names the skill,
2. the bounded work clearly matches the skill description in the active catalog.

Activation rule:

1. do not activate every available skill by default,
2. activate the minimal relevant set,
3. keep activation bounded to the current step or packet,
4. re-evaluate the relevant skill set when the bounded work changes materially.

## Orchestrator Rule

The orchestrator must:

1. check the available skill catalog during session startup,
2. note which skills are relevant to current shaping or execution,
3. include relevant skill expectations in dispatched packets when the worker will need them,
4. avoid broad skill loading when no current bounded need exists.

The orchestrator must not:

1. assume workers will infer relevant skills on their own without packet or bootstrap guidance,
2. dispatch work before relevant skill activation is checked,
3. activate unrelated skills as startup noise.

## Worker Rule

Delegated agents must:

1. inspect the available skill catalog as part of lane bootstrap,
2. read the relevant `SKILL.md` files before doing packet work,
3. stay within the minimal relevant skill set for the current packet,
4. fail closed when a named or clearly required skill is missing and the packet depends on it materially.

## Packet Rule

When a packet depends materially on one or more skills, the packet should identify:

1. the relevant skill name or names,
2. whether activation is `required` or `optional`,
3. the bounded reason the skill matters to the packet.

If the packet is silent but the skill match is obvious from the active catalog, the receiving lane must still activate the relevant skill.

## Session-Start Rule

No session is launch-ready until both are true:

1. runtime/bootstrap state is valid,
2. relevant skills have been checked and activated or `no_applicable_skill` has been made explicit.

## Routing

1. for routine startup skill/readiness gating, read `docs/process/project-start-readiness-runtime-capsule.md`,
2. for orchestrator startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for top-level routing, read `docs/process/project-orchestrator-operating-protocol.md`,
4. for delegated packet/team law, read `docs/process/team-development-and-orchestration-protocol.md`,
5. for Codex role/runtime settings, read `docs/process/codex-agent-configuration-guide.md`.

-----
artifact_path: process/project-skill-initialization-and-activation-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-skill-initialization-and-activation-protocol.md
created_at: '2026-03-13T19:10:00+02:00'
updated_at: '2026-03-13T19:10:00+02:00'
changelog_ref: project-skill-initialization-and-activation-protocol.changelog.jsonl
