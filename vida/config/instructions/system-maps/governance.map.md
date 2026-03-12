# Governance Map

Purpose: expose the human-governance surfaces of VIDA so approval, contribution, policy-gate, and lifecycle rules are discoverable without mixing them into topology maps or runtime-family maps.

## Governance Surfaces

1. Bootstrap governance and hard invariants
   - `AGENTS.md`
2. Human approval lifecycle
   - `vida/config/instructions/runtime-instructions/work.human-approval-protocol.md`
3. User approval loop between tasks
   - `vida/config/instructions/runtime-instructions/bridge.task-approval-loop-protocol.md`
4. Verification, approval, and closure machine-readable policies
   - `vida/config/policies/verification_policy.yaml`
   - `vida/config/policies/approval_policy.yaml`
   - `vida/config/policies/closure_policy.yaml`
5. Spec freshness / newer-decision precedence
   - `vida/config/instructions/runtime-instructions/work.spec-freshness-protocol.md`
6. Document lifecycle / metadata freshness
   - `vida/config/instructions/runtime-instructions/work.document-lifecycle-protocol.md`
7. External-evidence / live-validation governance
   - `vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`
8. Repository contribution and publication expectations
   - `CONTRIBUTING.md`

## Activation Triggers

Read this map when:

1. approval or policy-gate behavior is active,
2. closure readiness depends on verification or approval semantics,
3. contribution, publication, or repo-governance questions are active,
4. document lifecycle or freshness policy is being changed,
5. a map needs the current human-governance owner surface.

## Routing

1. Approval receipts and approval blocking states:
   - continue to `vida/config/instructions/runtime-instructions/work.human-approval-protocol.md`
2. User gating between tasks:
   - continue to `vida/config/instructions/runtime-instructions/bridge.task-approval-loop-protocol.md`
3. Verification / approval / closure policy values:
   - continue to `vida/config/policies/verification_policy.yaml`
   - `vida/config/policies/approval_policy.yaml`
   - `vida/config/policies/closure_policy.yaml`
4. Versioning / newer-decision precedence:
   - continue to `vida/config/instructions/runtime-instructions/work.spec-freshness-protocol.md`
5. Contribution / publication rules:
   - continue to `CONTRIBUTING.md`

## Boundary Rule

1. Governance discovery is owned here, not by topology maps.
2. Runtime families may consume governance rules but do not own them.
3. Project process docs may add project-specific operating rules, but they must not replace framework governance canon.

-----
artifact_path: config/system-maps/governance.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/governance.map.md
created_at: '2026-03-10T09:30:00+02:00'
updated_at: '2026-03-11T13:40:40+02:00'
changelog_ref: governance.map.changelog.jsonl
