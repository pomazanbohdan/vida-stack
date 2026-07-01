# Requirement Analysis Protocol (RAP)

Purpose: one canonical facade-owner protocol for turning raw requirement requests into a structured, evidence-backed runtime handoff.

Identity:

1. canonical id: `runtime-instructions/work.requirement-analysis-protocol`,
2. short id: `requirement-analysis`,
3. command surface: `vida requirement analyze`,
4. owner layer: runtime instruction,
5. active output artifact: `requirement_analysis`.

## Scope

RAP applies when the operator or runtime must decide what a request means before TaskFlow writer work starts.

It owns:

1. requirement identity and source binding,
2. requirement classification,
3. requirement atoms,
4. selected analysis methods,
5. selected analysis roles,
6. conflicts and open questions,
7. working assumptions,
8. solution options and recommendation,
9. readiness verdict,
10. downstream route recommendation,
11. acceptance criteria,
12. test matrix,
13. codebase impact summary,
14. developer handoff contract,
15. optional Party Chat challenge-round recommendation.

## Non-Goals

RAP does not:

1. authorize implementation by itself,
2. replace TaskFlow task ownership,
3. bypass approval, coach, verifier, proof, or closure law,
4. make URDSP, BMAD, Party Chat, chat history, or research notes active owner law,
5. silently widen scope after a writer lane starts.

External methods and prior frameworks may be provenance only. The executable rule is this protocol plus the current runtime/config state.

## Mandatory Inputs

At least one identity input is required:

1. `task_id`, or
2. `request_id`.

At least one source input should be present when the analysis is not a placeholder:

1. operator text,
2. source file,
3. issue text,
4. product/spec artifact,
5. runtime defect report,
6. upstream PR or review evidence.

When no source text is supplied, the artifact must mark the placeholder input explicitly.

## Triggers

Use RAP when any trigger is true:

1. a user asks to analyze, clarify, or convert requirements,
2. a request includes mixed feature, defect, release, documentation, or cleanup language,
3. implementation scope is unclear,
4. acceptance criteria or proof targets are missing,
5. TaskFlow needs a writer-ready handoff,
6. conflict, ambiguity, or low confidence may require an expert challenge route.

## Expected Output

The artifact must be self-contained and machine-readable.

Required fields:

1. `task_id` or `request_id`,
2. `source_inputs`,
3. `requirement_classification`,
4. `depth_mode`,
5. `requirement_atoms`,
6. `selected_methods`,
7. `selected_roles`,
8. `role_findings_summary`,
9. `detected_conflicts`,
10. `challenge_route`,
11. `open_questions`,
12. `working_assumptions`,
13. `solution_options`,
14. `recommended_option`,
15. `readiness_verdict`,
16. `downstream_routes`,
17. `acceptance_criteria`,
18. `test_matrix`,
19. `output_contract`,
20. `codebase_impact`,
21. `developer_handoff`.

## Minimal Flow

1. Bind identity.
2. Normalize source inputs.
3. Classify request class.
4. Derive atomic requirements with source references.
5. Select analysis methods and roles from active config or registry.
6. Detect conflicts and open questions.
7. Build working assumptions only when they are explicit and reviewable.
8. Create solution options.
9. Pick a recommended option or mark the artifact blocked.
10. Decide readiness.
11. Emit downstream routes.
12. Emit acceptance criteria and test matrix.
13. Emit developer handoff.
14. Emit optional `challenge_route` when configured triggers fire.

## Gates

Writer work is not authorized unless:

1. identity is present,
2. source inputs are sufficient for the requested depth,
3. critical open questions are empty,
4. detected conflicts are either resolved or explicitly routed,
5. readiness is `ready_for_developer_handoff`,
6. acceptance criteria are present,
7. test matrix is present,
8. downstream route is lawful under TaskFlow,
9. proof expectations are present.

## Blocker Classes

1. `missing_requirement_identity`
2. `requirement_source_unreadable`
3. `requirement_source_insufficient`
4. `requirement_conflict_unresolved`
5. `requirement_open_questions_blocking`
6. `requirement_route_unavailable`
7. `requirement_proof_missing`

Each blocker must include a concrete next action.

## Owner Boundaries

RAP routes to peer protocols instead of copying their law:

1. `runtime-instructions/work.spec-intake-protocol` for broad or mixed intake,
2. `runtime-instructions/work.spec-contract-protocol` for non-development spec contracts,
3. `runtime-instructions/bridge.issue-contract-protocol` for bug or defect reports,
4. `runtime-instructions/work.taskflow-protocol` for TaskFlow task creation and execution,
5. `runtime-instructions/work.problem-party-protocol` for Party Chat challenge rounds.

## Party Chat Challenge Route

RAP may recommend Party Chat only when active config or registry policy enables it and one of the configured triggers fires.

Valid trigger families:

1. critical depth,
2. explicit multi-perspective user request,
3. unresolved role conflict,
4. cross-boundary architecture, security, data, or API ambiguity,
5. low-confidence readiness.

Routine clear requirements must not recommend Party Chat by default.

Party Chat output must be structured as:

1. findings,
2. conflicts,
3. questions,
4. options,
5. synthesis.

It must not bypass TaskFlow writer, coach, verifier, approval, or closure law.

## User Interaction Points

Ask the user only when:

1. a critical open question blocks readiness,
2. source evidence is missing and cannot be inferred,
3. multiple options have materially different product outcomes,
4. approval is required before widening scope,
5. credentialed or private data is needed.

Otherwise continue autonomously with recorded assumptions.

## Verification

Closure proof must include:

1. `vida protocol view requirement-analysis --json`,
2. `vida protocol view runtime-instructions/work.requirement-analysis-protocol --json`,
3. a representative `vida requirement analyze --json` artifact when command behavior changes,
4. focused tests for any new routing behavior.

## Recovery

If analysis cannot complete:

1. emit a blocked artifact,
2. preserve identity and source refs,
3. name the blocker class,
4. name the next legal command or user input,
5. avoid partial writer authorization.

## Runtime Adoption

Runtime surfaces that expose requirement analysis must:

1. include `challenge_route` in JSON output,
2. keep default output compact,
3. keep JSON complete,
4. resolve the protocol through `vida protocol view requirement-analysis`,
5. keep short-id discovery index-backed rather than one-off hardcoded aliases.

-----
artifact_path: config/runtime-instructions/work.requirement-analysis-protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.requirement-analysis-protocol.md
created_at: '2026-07-01T00:00:00Z'
updated_at: '2026-07-01T00:00:00Z'
changelog_ref: work.requirement-analysis-protocol.changelog.jsonl
