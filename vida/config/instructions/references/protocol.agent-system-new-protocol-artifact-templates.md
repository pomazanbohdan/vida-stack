# Agent-System New-Protocol Artifact Templates

Purpose: provide bounded template shapes for the artifacts emitted by `work.agent-system-new-protocol-development-and-update-protocol.md`.

Status:

1. this file is non-canonical reference material,
2. it does not own protocol law,
3. the canonical law remains in `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md`.

## Requirement Inventory Row Template

```yaml
requirement_id: REQ-001
source: user | spec | protocol_update | command_determinization | external_requirement
normalized_requirement: "<bounded requirement statement>"
coverage_class: identity | scope | trigger | input | output | flow | gate | blocker | evidence | ownership | interaction | verification | approval | recovery | activation | validation
earliest_owner_layer: Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6
current_status: covered | provisional | blocked | rejected
conflict_note: "<empty or short conflict summary>"
```

## Layer Output Packet Template

```yaml
layer: Layer N
purpose: "<why this layer exists>"
outputs:
  - "<layer-specific output>"
green_criteria:
  - "<criterion>"
active_blockers:
  - "<blocker code or empty>"
current_state: LAYER_IN_PROGRESS | WAITING_USER_INPUT | LAYER_COVERED | BLOCKED
current_verdict: LAYER_COVERED | WAITING_USER_INPUT | BLOCKED | GREEN_AT_LAYER
```

## Closure Receipt Template

```yaml
mode: new-protocol | update | determinization | absorption
update_class: editorial | clarification | bounded_extension | boundary_narrowing | boundary_expansion | split | merge | breaking_change | full_absorption
target_closure_layer: Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6
highest_green_layer: Layer 0 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6
current_state: NOT_STARTED | LAYER_IN_PROGRESS | WAITING_USER_INPUT | LAYER_COVERED | BLOCKED | PROTOCOL_GREEN_PARTIAL | PROTOCOL_GREEN_FULL
current_verdict: LAYER_COVERED | WAITING_USER_INPUT | BLOCKED | GREEN_AT_LAYER | FRAMEWORK_GREEN
active_blockers:
  - "<blocker code>"
owner_resolution: "<one canonical owner / split owner / unresolved>"
index_wired: true | false
activation_wired: true | false
validation_passed: true | false
requirements_mapped: true | false
requirements_unmapped:
  - "<requirement id>"
```

## Determinization Receipt Template

```yaml
command_surface: "<command or surface>"
responsibility_inventory:
  - "<bounded responsibility>"
existing_protocol_owners:
  - "<protocol path>"
new_protocol_candidates:
  - "<candidate name>"
shared_entrypoint_retained: true | false
hidden_law_removed: true | false
requirements_reproduced: true | false
```

## Absorption Receipt Template

```yaml
stronger_owner: "<canonical protocol>"
weaker_owner: "<absorbed protocol>"
coverage_proven: true | false
material_requirements_preserved: true | false
duplicate_law_removed: true | false
pointer_only_retained: true | false
artifact_deleted: true | false
index_rewired: true | false
activation_rewired: true | false
unmapped_requirements_after_absorption:
  - "<requirement id>"
```

## Self-Assessment Receipt Template

```yaml
assessed_protocol: agent-system-new-protocol-development-and-update
update_class: "<class>"
earliest_reopened_layer: Layer 0 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6
highest_green_layer: Layer 0 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6
current_state: NOT_STARTED | LAYER_IN_PROGRESS | WAITING_USER_INPUT | LAYER_COVERED | BLOCKED | PROTOCOL_GREEN_PARTIAL | PROTOCOL_GREEN_FULL
current_verdict: LAYER_COVERED | WAITING_USER_INPUT | BLOCKED | GREEN_AT_LAYER | FRAMEWORK_GREEN
ideal_state_claim: true | false
active_blockers:
  - "<blocker code>"
residual_gaps:
  - "<gap summary>"
autonomy_proven: true | false
acceptance_law_satisfied: true | false
```

## Residual Gap Row Template

```yaml
gap_id: GAP-001
layer: Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6 | cross-layer
gap_summary: "<short bounded summary>"
current_effect: "<what is weakened>"
blocking_status: blocking | non_blocking
next_action: "<next bounded action>"
```

-----
artifact_path: config/instructions/references/agent-system-new-protocol-artifact-templates
artifact_type: reference
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/references/protocol.agent-system-new-protocol-artifact-templates.md
created_at: '2026-03-11T08:17:28+02:00'
updated_at: '2026-03-11T13:45:47+02:00'
changelog_ref: protocol.agent-system-new-protocol-artifact-templates.changelog.jsonl
