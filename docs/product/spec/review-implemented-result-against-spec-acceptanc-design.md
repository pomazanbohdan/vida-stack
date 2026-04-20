# Review Implemented Result Against Spec Acceptanc Design

Status: `approved`

## Summary
- Feature / change: formalize the bounded coach-lane review contract that judges an implemented result against the approved spec, acceptance criteria, and `definition_of_done`.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- VIDA already models `coach` as a distinct runtime role between implementer and verifier.
- The dispatch alias for `development_coach` defines the mission clearly: judge whether the implemented result matches the approved spec, acceptance criteria, and definition of done, then return bounded approval-forward or bounded rework evidence.
- `runtime_dispatch_packets.rs` already emits a `coach_review_packet` with:
  - `review_goal`
  - `read_only_paths`
  - `definition_of_done`
  - `proof_target`
  - `review_focus`
  - `blocking_question`
- `runtime_dispatch_state.rs` already validates key required fields for `coach_review_packet`, but this feature slice needs the design contract written down so the lane remains packet-local, formative, and distinct from independent verification.

## Goal
- Make the coach review lane explicit and durable as a bounded spec-conformance gate.
- Preserve the separation between implementer, coach, and verifier responsibilities.
- Ensure coach review remains packet-local, evidence-based, and fail-closed on missing review contract fields.

## Requirements

### Functional Requirements
- `coach_review_packet` must include a non-empty `review_goal`.
- `coach_review_packet` must carry at least one lawful scope field via `owned_paths` or `read_only_paths`.
- `coach_review_packet` must include non-empty `definition_of_done` and `proof_target`.
- Coach review must determine one of two bounded outcomes:
  - approval-forward
  - bounded rework with concrete drift evidence
- Coach review must compare the implemented result against:
  - the approved bounded packet
  - acceptance criteria
  - `definition_of_done`

### Non-Functional Requirements
- Keep this slice bounded to coach-lane packet semantics and validation, not a broader redesign of verification or closure law.
- Preserve coach as formative review only, not a hidden second implementer lane.
- Keep the lane contract observable through packet payloads and existing runtime evidence surfaces.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  `docs/product/spec/review-implemented-result-against-spec-acceptanc-design.md`
- Framework protocols affected:
  none
- Runtime families affected:
  coach-lane packet shaping
  coach packet validation
  handoff sequencing between implementer, coach, and verifier
- Config / receipts / runtime surfaces affected:
  `development_coach` dispatch alias
  `coach_review_packet`
  downstream handoff readiness after implementer evidence

## Design Decisions

### 1. Coach Is A Packet-Local Formative Gate
Will implement / choose:
- Keep `coach` focused on bounded conformance to the already-approved packet contract.
- Why:
  - coach must determine whether the implementation still matches the agreed scope without widening into architecture, milestone planning, or release approval.
- Trade-off:
  - some broader quality observations stay out of coach review and remain verifier or architect concerns.

### 2. Coach Must Stay Distinct From Verifier
Will implement / choose:
- Preserve coach as formative review and verifier as independent proof/closure-readiness review.
- Why:
  - collapsing the two roles would weaken the independent closure gate and blur packet-local rework versus closure evidence.
- Trade-off:
  - there are two downstream quality surfaces, but each keeps a cleaner responsibility boundary.

### 3. Coach Packet Contract Is Fail-Closed
Will implement / choose:
- Treat missing `review_goal`, missing lawful scope, missing `definition_of_done`, or missing `proof_target` as contract failures for `coach_review_packet`.
- Why:
  - coach review is only meaningful if the bounded review contract is explicit in the packet itself.
- Trade-off:
  - malformed packets stop earlier instead of relying on implicit review assumptions.

## Technical Design

### Core Components
- `docs/process/agent-extensions/dispatch-aliases.yaml`
  - `development_coach` alias
  - role, mission, and bounded-review instructions
- `crates/vida/src/runtime_dispatch_packets.rs`
  - `runtime_coach_review_packet(...)`
- `crates/vida/src/runtime_dispatch_state.rs`
  - `coach_review_packet` contract validation
  - downstream sequencing between implementer, coach, and verifier

### Data / State Model
- `coach_review_packet` required semantics:
  - `review_goal`: what alignment question the coach must answer
  - `owned_paths` / `read_only_paths`: the review boundary
  - `definition_of_done`: the bounded acceptance frame
  - `proof_target`: the concrete evidence target
  - `review_focus`: packet-local review categories
  - `blocking_question`: explicit decision question for the lane

### Integration Points
- implementer completion can activate coach as the next bounded downstream lane
- coach review can either:
  - return bounded approval-forward toward verification or next lawful step
  - return bounded rework guidance back into the implementation flow
- verifier remains downstream and independent from coach conclusions

### Bounded File Set
- `docs/product/spec/review-implemented-result-against-spec-acceptanc-design.md`
- `docs/process/agent-extensions/dispatch-aliases.yaml`
- `crates/vida/src/runtime_dispatch_packets.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

## Fail-Closed Constraints
- Coach must not silently replace implementer, verifier, or escalation lanes.
- Coach must not rewrite implementation unless the bounded packet explicitly authorizes mutation.
- Coach must not approve forward when the packet lacks explicit review contract fields.
- Coach must not widen review scope beyond the bounded packet and approved acceptance frame.

## Implementation Plan

### Phase 1
- Record the coach-lane contract from the existing alias and packet shape.
- Anchor the distinction between coach and verifier in the design doc.
- First proof target: the bounded review contract is explicit and canonical.

### Phase 2
- Verify that `coach_review_packet` validation covers the essential required fields.
- Tighten or extend focused tests only if packet contract gaps are found.
- Second proof target: malformed coach packets fail closed on missing review contract data.

### Phase 3
- Reconfirm downstream sequencing between implementer, coach, and verifier.
- Ensure coach-ready handoff remains packet-local and evidence-oriented.
- Final proof target: focused runtime tests preserve coach handoff and packet validation truth.

## Validation / Proof
- Unit tests:
  - focused `coach_review_packet` validation checks in `runtime_dispatch_state.rs`
- Integration tests:
  - bounded tests around implementer-to-coach downstream readiness and coach packet shaping
- Runtime checks:
  - inspect coach downstream packet payloads and validation behavior where needed
- Canonical checks:
  - `vida docflow finalize-edit docs/product/spec/review-implemented-result-against-spec-acceptanc-design.md "record bounded coach review packet design"`
  - `vida docflow check --root . docs/product/spec/review-implemented-result-against-spec-acceptanc-design.md`

## Observability
- Operator-visible outcome:
  coach review is visibly a packet-local spec-conformance gate, not a generic or hidden reviewer
- Runtime evidence:
  coach packets expose `review_goal`, `definition_of_done`, `proof_target`, and bounded review focus
- Sequencing truth:
  implementer completion can lawfully bridge into coach without collapsing directly into verification

## Rollout Strategy
- Ship as a bounded coach-lane contract clarification first.
- Use the design doc to govern any later runtime implementation tightening.
- No migration is required.

## Future Considerations
- A follow-up can standardize coach result artifact shape if review outcomes need stronger structured evidence.
- Another follow-up can align coach and verifier summaries on shared operator surfaces without collapsing their roles.

## References
- `docs/process/agent-extensions/dispatch-aliases.yaml`
- `docs/product/spec/agent-role-skill-profile-flow-model.md`
- `crates/vida/src/runtime_dispatch_packets.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

-----
artifact_path: product/spec/review-implemented-result-against-spec-acceptanc-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-13
schema_version: 1
status: canonical
source_path: docs/product/spec/review-implemented-result-against-spec-acceptanc-design.md
created_at: 2026-04-13T20:38:57.417580909Z
updated_at: 2026-04-20T08:47:39.805226841Z
changelog_ref: review-implemented-result-against-spec-acceptanc-design.changelog.jsonl
