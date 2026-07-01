# Requirement Analysis Method Skills

Status: active project process note

Purpose: record the supporting evidence and activation rationale for requirement-analysis method skills exported through `docs/process/agent-extensions/skills.yaml` and enabled by `vida.config.yaml`.

## Authority Boundary

1. Runtime authority is the local registry row plus `vida.config.yaml` activation.
2. External references are supporting research evidence only.
3. A downstream agent must be able to use each skill from its autonomous registry description without opening an external source.
4. Method skill ids are stable registry keys; role routing remains controlled by `compatible_base_roles` and the active runtime flow.

## Supporting Evidence

1. IIBA BABOK glossary frames business analysis as defining needs and recommending value-delivering solutions; it also defines business rules, acceptance criteria, actors, assumptions, and business analysis artifacts.
2. IREB CPRE glossary treats requirements engineering as elicitation, negotiation, and validation; it classifies requirements into functional requirements, quality requirements, and constraints, with quality requirements and constraints also called non-functional requirements.
3. IREB defines a method as systematic application of techniques to achieve an objective or create a work product, which matches this registry's use of bounded method skills.
4. OWASP ASVS provides a security-requirements verification basis and a stable identifier model for application security control requirements.
5. OpenAPI defines a language-agnostic API description that lets humans and computers understand HTTP API capabilities without source-code access, which supports contract-analysis skills.
6. OMG PSSM defines execution semantics for UML state machines, which supports state-machine and workflow-analysis skills for lifecycle-heavy requirements.

## Activation Rationale

1. `ra_requirement_clarification` covers elicitation, consolidation, assumptions, constraints, open questions, and acceptance criteria.
2. `ra_user_flow_analysis` covers actor goals, scenarios, interaction paths, usability risks, and UX-facing requirement shape.
3. `ra_business_rule_analysis` covers obligations, prohibitions, decision logic, exceptions, and policy-to-requirement traceability.
4. `ra_state_machine_workflow_analysis` covers states, events, guards, transitions, invalid transitions, recovery, concurrency, and run-to-completion assumptions.
5. `ra_data_lifecycle_analysis` covers data creation, validation, persistence, synchronization, retention, deletion, migration, privacy, and downstream contracts.
6. `ra_api_integration_contract_analysis` covers API boundaries, schemas, operations, versioning, errors, authentication, compatibility, and consumer-provider obligations.
7. `ra_security_privacy_review` covers abuse cases, protected assets, access control, exposure limits, verification requirements, and fail-closed handling.
8. `ra_nonfunctional_requirement_review` covers measurable quality requirements and constraints including performance, reliability, maintainability, safety, security, usability, and operations.
9. `ra_test_matrix_design` covers equivalence classes, boundary values, decision tables, state transitions, negative cases, fixtures, and proof targets.
10. `ra_codebase_impact_analysis` covers owned paths, modules, APIs, migrations, runtime state, tests, docs, release gates, dependency risks, and implementation order.

## Trace

- source_refs:
  - https://www.iiba.org/career-resources/a-business-analysis-professionals-foundation-for-success/babok/glossary/
  - https://cpre.ireb.org/en/downloads-and-resources/glossary
  - https://owasp.org/www-project-application-security-verification-standard/
  - https://swagger.io/specification/
  - https://www.omg.org/spec/PSSM/

-----
artifact_path: process/agent-extensions/requirement-analysis-method-skills
artifact_type: process_note
artifact_version: '1'
schema_version: '1'
status: active
source_path: docs/process/agent-extensions/requirement-analysis-method-skills.md
created_at: 2026-07-01T00:00:00Z
