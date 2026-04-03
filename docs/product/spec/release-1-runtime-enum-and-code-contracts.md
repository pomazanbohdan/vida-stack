# Release 1 Runtime Enum And Code Contracts

Status: active Release-1 implementation law

Purpose: define the canonical enum/value vocabulary and field-binding rules that runtime code, receipts, checkpoints, metrics, tests, and operator surfaces must share during `Release 1`.

## 1. Scope

This contract defines canonical values for:

1. `workflow_class`
2. `risk_tier`
3. `lane_status`
4. `approval_status`
5. `gate_level`
6. `blocker_code`
7. `compatibility_class`

It also fixes:

1. the canonical machine field names used to emit those values,
2. the rule that one shared implementation layer must own them,
3. the prohibition on shell-local synonyms or ad hoc machine strings.

## 1.1 Shared Implementation Rule

Release-1 runtime code must expose one shared implementation layer for:

1. parsing,
2. canonical serialization,
3. persistence encoding,
4. operator rendering,
5. fixture/golden validation.

Rule:

1. launcher surfaces must import the shared layer rather than invent local machine values,
2. `TaskFlow` and `DocFlow` family-owned modules must share the same canonical machine strings,
3. tests must validate against the same canonical implementation layer rather than against duplicated inline literals.

## 1.2 Canonical Machine Fields

When these values are emitted, the canonical field names are:

1. `workflow_class`
2. `risk_tier`
3. `lane_status`
4. `approval_status`
5. `gate_level`
6. `blocker_codes`
7. `compatibility_class`

Rule:

1. singular fields may use one canonical value,
2. blocker output must use `blocker_codes` as the machine field and not ad hoc aliases such as `blocker_code` for the primary shared surface,
3. compatibility output must use `compatibility_class` and not renamed machine variants such as `compatibility_classification`.

## 2. Workflow Class Values

Allowed values:

1. `informational_answer`
2. `retrieval_grounded_answer`
3. `documentation_mutation`
4. `internal_state_mutation`
5. `delegated_development_packet`
6. `tool_assisted_read`
7. `tool_assisted_write`
8. `memory_write`
9. `identity_or_policy_change`
10. `incident_response_or_recovery`

## 3. Risk Tier Values

Allowed values:

1. `R0`
2. `R1`
3. `R2`
4. `R3`
5. `R4`

## 4. Lane Status Values

Allowed values:

1. `packet_ready`
2. `lane_open`
3. `lane_running`
4. `lane_blocked`
5. `lane_completed`
6. `lane_superseded`
7. `lane_exception_takeover`

## 5. Approval Status Values

Allowed values:

1. `approval_not_required`
2. `approval_required`
3. `waiting_for_approval`
4. `approved`
5. `denied`
6. `expired`

## 6. Gate Level Values

Allowed values:

1. `block`
2. `warn`
3. `observe`

## 7. Compatibility Class Values

Allowed values:

1. `backward_compatible`
2. `reader_upgrade_required`
3. `migration_required`

Forbidden machine-value substitutes:

1. `compatible`
2. `incompatible`
3. `degraded`
4. `blocking`

## 8. Blocker Code Values

Allowed values include at minimum:

1. `missing_packet`
2. `missing_lane_receipt`
3. `open_delegated_cycle`
4. `exception_path_missing`
5. `closure_evidence_incomplete`
6. `owner_surface_contradiction`
7. `policy_denied`
8. `approval_required`
9. `approval_denied`
10. `approval_expired`
11. `delegation_chain_broken`
12. `tool_contract_missing`
13. `tool_contract_incomplete`
14. `tool_execution_failed`
15. `citation_missing`
16. `source_unregistered`
17. `freshness_policy_missing`
18. `freshness_violation`
19. `trace_incomplete`
20. `incident_evidence_missing`
21. `rollback_unavailable`
22. `proof_verdict_missing`
23. `metric_gate_failed`
24. `schema_contract_missing`
25. `timeout_without_takeover_authority`
26. `supersession_without_receipt`
27. `local_takeover_forbidden`
28. `recovery_not_trustworthy`

Registry rule:

1. `release-1-error-and-exception-taxonomy.md` is the owner document for the full blocker registry,
2. runtime code may not emit blocker machine strings that are absent from the canonical registry,
3. adding a new blocker requires updating the owner registry before or in the same change as code emission,
4. lower-snake-case shape alone is not sufficient for conformance.

## 9. Code Contract Rule

1. Release-1 Rust code must not invent alternative enum names for these values.
2. User-facing labels may differ, but canonical machine values must not.
3. Any expansion requires updating this contract first.
4. Absence of a shared implementation for one required value family is a Release-1 conformance gap even when prose docs already define the vocabulary.
5. launcher-only value shaping does not satisfy this contract.

## 10. Closure Rule

Release 1 is not closed until all are true:

1. one shared code layer exposes `workflow_class`, `risk_tier`, `lane_status`, `approval_status`, `gate_level`, `compatibility_class`, and the canonical blocker registry,
2. operator surfaces emit the canonical machine field names from this document,
3. persisted runtime artifacts and fixtures/goldens validate against the same value set,
4. legacy machine-value substitutes are removed or reduced to explicit compatibility readers rather than active emitters.

## 11. References

1. `docs/product/spec/release-1-error-and-exception-taxonomy.md`
2. `docs/product/spec/release-1-state-machine-specs.md`
3. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
4. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`
5. `docs/product/spec/release-1-operator-surface-contract.md`

-----
artifact_path: product/spec/release-1-runtime-enum-and-code-contracts
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-03
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-runtime-enum-and-code-contracts.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-04-03T18:55:00+03:00
changelog_ref: release-1-runtime-enum-and-code-contracts.changelog.jsonl
