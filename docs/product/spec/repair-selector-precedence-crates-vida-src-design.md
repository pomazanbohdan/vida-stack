# Repair Selector Precedence Crates Vida Src Design

Status: `approved`

## Summary
- Feature / change: repair selector precedence in `crates/vida/src/runtime_lane_summary.rs` so bounded Rust-file repair requests route to implementation/worker instead of planning/specification.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `build_runtime_lane_selection_from_bundle(...)` scores conversation-mode keyword candidates before selecting a runtime role.
- Requests that include planning/specification words can currently drift toward `scope_discussion` or `pbi_discussion` even when the same request is a write-producing bounded repair against a concrete Rust file.
- The live bounded examples in `runtime_lane_summary.rs` and `taskflow_run_graph.rs` show the intended contract: a request that says to repair/fix a specific `.rs` file and add regression tests must route to `worker` / implementation rather than `business_analyst` / specification planning.
- The current routing logic already contains an override branch for bounded repair terms, but this feature slice must make that precedence rule explicit, durable, and regression-proven so future selector edits do not collapse the distinction again.

## Goal
- Ensure bounded Rust-file repair requests override planning-style conversation routing and select `worker`.
- Preserve lawful planning/spec-first routing for genuine design/specification requests.
- Freeze the observable selector outputs for this boundary: `selected_role`, `conversational_mode`, `reason`, and the matched-term evidence.

## Requirements

### Functional Requirements
- Requests that combine repair/fix intent with concrete Rust file or code-test scope must route to `worker`.
- Requests that are genuinely spec-first, discovery-first, or acceptance-criteria-first must remain on `scope_discussion`.
- Verification/review intent must continue to win over bounded repair terms when both are present.
- The selector must record `reason = auto_explicit_implementation_request_override` when the bounded repair override fires.
- When the override fires, `conversational_mode` must be `None`.

### Non-Functional Requirements
- Keep the change bounded to the selector precedence logic in `runtime_lane_summary.rs` and focused regressions.
- Do not redesign the broader execution-plan builder or bundle compilation path.
- Keep the proof readable through unit tests and seeded run-graph tests that use minimal prompt differences.
- Keep the routing contract provider-compatible with OpenAI Responses/Tools semantics and Azure OpenAI reasoning/tool semantics.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  `docs/product/spec/repair-selector-precedence-crates-vida-src-design.md`
- Framework protocols affected:
  none
- Runtime families affected:
  lane selection
  seeded run-graph routing truth
- Config / receipts / runtime surfaces affected:
  auto role-selection precedence
  `selected_role`
  `conversational_mode`
  `reason`
  `matched_terms`

## Provider Compatibility

### OpenAI API Compatibility
- OpenAI treats `developer` instructions as higher priority than `user` messages, which is compatible with maintaining an application-level routing policy above raw user wording.
- OpenAI tool invocation is model-directed by default when tools are configured, and the application can explicitly steer or constrain this behavior with `tool_choice`.
- This feature therefore remains application-level policy, not a replacement for provider tool semantics:
  - internal `worker` means the request is routed onto an implementation-capable path
  - internal `scope_discussion` means the request stays on planning/specification path
  - provider-native tool calling remains underneath that routing decision

### Azure OpenAI Compatibility
- Azure OpenAI reasoning documentation states that `developer` messages are functionally the same as system messages for the supported reasoning models, and explicitly warns not to send both `developer` and `system` in the same request.
- Azure OpenAI also exposes `tool_choice`, `max_output_tokens`, `max_tool_calls`, and `parallel_tool_calls` on the Responses surface, which means deterministic application policy can be layered above provider tool execution without violating the provider contract.
- This feature must therefore preserve a clean provider mapping:
  - when the application selects a planning/spec path, the downstream provider request should stay planning-oriented rather than depending on tool auto-selection to recover intent
  - when the application selects a bounded implementation path, the downstream provider request may still use provider-native tools, but the route selection itself should already be deterministic

### Provider Guardrails
- Do not depend on provider auto tool selection alone for high-risk routing boundaries where planning/spec and write-producing implementation prompts can be confused.
- Do not mix `developer` and `system` messages in a single Azure OpenAI reasoning request.
- Prefer one explicit instruction layer plus application policy over duplicated provider-level instruction stacks.

## Design Decisions

### 1. Bounded Code Repair Beats Planning Keywords Only When Scope Is Concrete
Will implement / choose:
- Treat bounded repair intent as the conjunction of repair/fix wording plus concrete Rust-file/code-test scope.
- This keeps the override narrow and prevents broad "fix this feature" wording from silently forcing implementation routing.
- Trade-off: the selector remains keyword-driven, but the precedence fence becomes specific and testable.

### 2. Planning Requests Keep Their Route Even If They Mention A Future Fix
Will implement / choose:
- Preserve `scope_discussion` for prompts that primarily ask for research, specification, acceptance criteria, or planning, even if they mention a later code fix.
- This keeps spec-first workflow lawful and avoids collapsing planning into implementation every time the user mentions "fix".
- Trade-off: wording-only heuristics remain sensitive, so adjacent regression coverage is required.

### 3. Observable Selector Outputs Are Part Of The Contract
Will implement / choose:
- Lock down `selected_role`, `conversational_mode`, `reason`, and representative `matched_terms` in focused regressions.
- This ensures future refactors do not preserve only the role while drifting the reason or conversational-mode truth.
- Trade-off: tests become slightly more specific, but operator-facing routing evidence stays trustworthy.

### 4. Deterministic Routing Happens Before Provider Tool Selection
Will implement / choose:
- Keep the planning-vs-implementation decision in the application router before any provider-native tool execution.
- Use provider `tool_choice` or equivalent downstream controls only as an execution-shaping mechanism after route selection, not as the primary repair for selector ambiguity.
- Trade-off: the system keeps one more layer of explicit policy, but avoids relying on model-side tool choice to correct upstream routing mistakes.

## Technical Design

### Core Components
- `crates/vida/src/runtime_lane_summary.rs`
  - candidate ranking for `scope_discussion` / `pbi_discussion`
  - explicit verification and implementation overrides
  - bounded repair override for file-scoped code fixes
- `crates/vida/src/taskflow_run_graph.rs`
  - seeded run-graph regression proving the routed task class no longer drifts to `spec-pack`

### Data / State Model
- The selector operates on four evidence families:
  - conversation-mode keyword matches
  - explicit implementation request terms
  - explicit verification request terms
  - explicit bounded code repair terms
- Bounded repair intent remains a conjunction:
  - repair/fix language such as `repair`, `fix`, `bug`, `regression`, `regression test`
  - concrete file/code/test scope such as `.rs`, `crates/`, `src/`, `rust file`, `unit test`, `proof`

### Integration Points
- lane-selection routing in `build_runtime_lane_selection_from_bundle(...)`
- helper `explicit_bounded_code_repair_terms(...)`
- seeded run-graph route derivation that must keep implementation requests off the `spec-pack` path

### Bounded File Set
- `docs/product/spec/repair-selector-precedence-crates-vida-src-design.md`
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/taskflow_run_graph.rs`

## Fail-Closed Constraints
- Do not let general planning/specification wording suppress a concrete bounded Rust-file repair request.
- Do not let the bounded repair override fire for broad or ambiguous feature-fix prompts that lack file/code/test scope.
- Do not let verification/review requests drift into implementation routing.
- Do not widen this slice into a full rewrite of the conversation-mode router.

## Implementation Plan

### Phase 1
- Inspect the current precedence path around candidate sorting and override branches.
- Confirm the exact prompt shapes that should route to `worker` versus `scope_discussion`.
- First proof target: the boundary is reproduced and stated in the design doc with exact observable outputs.

### Phase 2
- Repair the selector precedence or adjacent helper logic as needed so bounded Rust-file repair requests override planning-mode matches.
- Add or update focused regressions in `runtime_lane_summary.rs`.
- Second proof target: the runtime-lane tests show bounded repair prompts route to `worker` with `conversational_mode = None`.

### Phase 3
- Re-run nearby seeded run-graph tests that consume the same request family.
- Confirm implementation routing no longer drifts to `spec-pack` for bounded repair prompts.
- Final proof target: green focused tests in both selector and seeded run-graph surfaces.

## Validation / Proof
- Unit tests:
  - `bounded_rust_file_repair_request_overrides_scope_discussion_route`
  - `feature_design_request_keeps_scope_discussion_route_even_with_fix_wording`
  - `weak_pbi_discussion_match_does_not_override_explicit_fix_patch_intent`
- Integration tests:
  - bounded `cargo test -p vida` filters for `runtime_lane_summary` and `taskflow_run_graph`
- Runtime checks:
  - seeded route derivation for bounded repair wording must not yield `business_analyst` / `spec-pack`
- Canonical checks:
  - `vida docflow finalize-edit docs/product/spec/repair-selector-precedence-crates-vida-src-design.md "record bounded selector-precedence repair design"`
  - `vida docflow check --root . docs/product/spec/repair-selector-precedence-crates-vida-src-design.md`

## Observability
- Operator-visible outcome:
  bounded Rust-file repair requests select `worker` instead of planning/specification
- Runtime evidence:
  selector `reason` shows `auto_explicit_implementation_request_override`
  `conversational_mode` is absent when the override fires
- Seeded route truth:
  implementation/worker routing no longer projects `spec-pack` for the bounded repair prompt family

## External References
- OpenAI text generation and role priority:
  https://developers.openai.com/api/docs/guides/text
- OpenAI tools and tool-selection semantics:
  https://developers.openai.com/api/docs/guides/tools
- Azure OpenAI Responses API reference:
  https://learn.microsoft.com/en-us/azure/foundry/openai/latest
- Azure OpenAI reasoning models:
  https://learn.microsoft.com/en-us/azure/foundry/openai/how-to/reasoning
- Azure OpenAI function calling:
  https://learn.microsoft.com/en-us/azure/foundry/openai/how-to/function-calling

## Rollout Strategy
- Deliver as one bounded selector-precedence slice.
- Validate with focused routing regressions before implementation handoff.
- No migration or state rewrite is required.

## Future Considerations
- If more prompt families need precedence fences, the selector may benefit from an explicit ranked-rule table instead of open-coded branch ordering.
- If prompt evidence becomes richer than keywords, this bounded repair can later be generalized into a more structured routing classifier.

## References
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`

-----
artifact_path: product/spec/repair-selector-precedence-crates-vida-src-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-16
schema_version: 1
status: canonical
source_path: docs/product/spec/repair-selector-precedence-crates-vida-src-design.md
created_at: 2026-04-16T06:45:45.126690531Z
updated_at: 2026-04-20T08:45:55.304729458Z
changelog_ref: repair-selector-precedence-crates-vida-src-design.changelog.jsonl
