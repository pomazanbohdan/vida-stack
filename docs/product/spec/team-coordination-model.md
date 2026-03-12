# Team Coordination Model

Status: active product law

Purpose: define `team` as a compiled coordination object in the VIDA runtime rather than a flat list of roles, and establish how teams are configured by the project while coordinated by framework/runtime law.

## 1. Why Team Exists

`team` is needed because one delivery objective may require more than one role/profile/agent posture, but the runtime still needs one bounded coordination object rather than an ad hoc set of participants.

`team` answers:

1. which coordinated participants are active for one delivery objective,
2. how they are coordinated,
3. what shared policy and closure rules apply,
4. how handoff and bounded collaboration occur.

## 2. Relation To Other Runtime Objects

The runtime objects are not interchangeable:

1. `role class`
   - the framework-recognized type of role
2. `profile`
   - one configured posture of a role class
3. `agent`
   - one runtime executor operating under a selected bundle
4. `team`
   - one coordinated runtime composition of roles, profiles, agents, and flow posture for a shared objective

## 3. External Coordination Baseline

Current official orchestration baselines support treating `team` as a coordination object, not a passive list:

1. OpenAI official agent patterns separate manager/tool usage and handoffs as explicit coordination forms,
2. Anthropic official subagent and hook surfaces imply bounded participants, lifecycle, and event-aware coordination,
3. Microsoft official orchestration patterns explicitly model sequential, concurrent, handoff, and group collaboration as coordination patterns.

VIDA rule:

1. a team must therefore carry coordination law, not only membership data.

## 4. Minimum Team Fields

Every team must expose at least:

1. `team_id`
2. `name`
3. `purpose`
4. `coordination_pattern`
5. `participants`
6. `activation_policy`
7. `shared_policy`
8. `context_policy`
9. `handoff_policy`
10. `closure_policy`
11. `state`

## 5. Coordination Patterns

Supported coordination-pattern classes include:

1. `manager-led`
2. `handoff-led`
3. `sequential`
4. `concurrent`
5. `group-collaboration`
6. `planner-led`

Pattern rule:

1. projects may choose which patterns are enabled,
2. the framework/runtime still owns what each pattern means operationally.

## 6. Participants

Participants may include:

1. role classes,
2. project roles,
3. profiles,
4. concrete agents,
5. optional specialized execution lanes admitted by the active project/runtime posture.

Participant rule:

1. a team may be project-configured,
2. but it must still resolve to lawful framework/runtime participant types.

## 7. Activation

Teams may be activated in two ways:

1. explicitly by project configuration,
2. dynamically by automatic runtime selection when the project has enabled that mode.

Activation rule:

1. automatic activation may choose only from enabled team-capable inputs,
2. it must remain within active project policy,
3. it must fail closed when no lawful team posture resolves.

## 8. Shared Policy

Teams may carry shared policy for:

1. cost,
2. quality,
3. approval,
4. escalation,
5. evidence/closure expectations.

Shared-policy rule:

1. team policy may specialize project/runtime behavior,
2. it must not weaken framework safety law.

## 9. Context And Handoff

Teams must define:

1. whether context is shared or packet-bounded,
2. which handoffs are allowed,
3. whether approval or verification must occur between participant transitions,
4. how bounded collaboration closes.

## 10. Team State

Minimum team-state classes:

1. `idle`
2. `active`
3. `blocked`
4. `escalated`
5. `closed`

State rule:

1. team state is runtime state, not narrative commentary,
2. it must be queryable through status families and runtime evidence.

## 11. Boundary Rule

1. teams are project-configured runtime compositions,
2. teams are not sealed framework protocols,
3. teams are not just freeform project notes,
4. framework/runtime owns coordination semantics,
5. project configuration owns which team compositions are enabled.

## 12. Completion Proof

This model is operationally closed enough for Release 1 when:

1. teams are recognized as first-class project activation objects,
2. teams resolve to lawful role/profile/agent compositions,
3. coordination pattern and closure policy are explicit,
4. team activation can be explicit or auto-selected under project policy,
5. team state is queryable through runtime status surfaces.

-----
artifact_path: product/spec/team-coordination-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: docs/product/spec/team-coordination-model.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-11T23:01:49+02:00'
changelog_ref: team-coordination-model.changelog.jsonl
