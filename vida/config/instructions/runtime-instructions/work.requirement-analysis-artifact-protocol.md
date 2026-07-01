# Requirement Analysis Artifact Protocol

Purpose: define the runtime artifact schema and public CLI contract for `vida requirement analyze`.

## CLI Contract

1. `vida requirement analyze --task-id <task-id>` renders compact operator output by default.
2. `vida requirement analyze --request-id <request-id>` is allowed when no TaskFlow task exists yet.
3. `vida requirement analyze --json` renders the same contract as machine-readable JSON.
4. `--input <text>` is repeatable for direct requirement text; `--request <text>` is an operator alias.
5. `--source-file <path>` adds source text from a readable file.
6. `--depth-mode <mode>` accepts `quick`, `standard`, and `critical`; default is `standard`.
7. `--codebase-inspected` marks code impact as inspected; `--inspect-codebase` is an operator alias.

## Artifact Fields

The artifact is self-describing and must include:

1. `task_id` or `request_id`
2. `source_inputs`
3. `requirement_classification`
4. `depth_mode`
5. `requirement_atoms`
6. `selected_methods`
7. `selected_roles`
8. `role_findings_summary`
9. `detected_conflicts`
10. `open_questions.critical`
11. `open_questions.important`
12. `open_questions.optional`
13. `working_assumptions`
14. `solution_options`
15. `recommended_option`
16. `readiness_verdict`
17. `downstream_routes`
18. `acceptance_criteria`
19. `test_matrix`
20. `codebase_impact` when code was inspected
21. `developer_handoff`
22. `output_contract`

## Output Policy

1. Default output is compact TOON/plain and must be understandable without external documents.
2. JSON output is explicit and intended for runtime consumers.
3. Readiness statuses are `ready_for_developer_handoff`, `blocked`, `needs_questions`, and `draft`; `ready` remains a compact label for readiness tables.
4. Downstream route entries must name the lawful next node or blocker that prevents routing.

-----
artifact_path: config/runtime-instructions/requirement-analysis-artifact.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.requirement-analysis-artifact-protocol.md
created_at: '2026-07-01T00:49:00+03:00'
updated_at: '2026-07-01T00:49:00+03:00'
changelog_ref: work.requirement-analysis-artifact-protocol.changelog.jsonl
