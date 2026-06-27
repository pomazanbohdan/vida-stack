# LDRK Baseline Drift Map

Status: generated baseline artifact for TaskFlow task `ldr-001`.

## Summary

| Metric | Value |
| --- | --- |
| targeted_production_loc | 165072 |
| all_runtime_lexical_mutation_candidates | 1670 |
| surface_direct_mutation_candidates | 530 |
| duplicate_classifier_candidates | 247 |
| status_helper_false_positive_candidates | 370 |
| cfg_test_classifier_candidates | 280 |
| cfg_test_status_helper_candidates | 803 |
| canonical_cli_leaf_command_candidates | 36 |
| command_specific_option_candidates | 191 |
| subprocess_command_name_count | 5 |
| legacy_derive_attribute_leaf_candidate_count | 164 |
| legacy_derive_attribute_option_candidate_count | 556 |

## LDR-074 Final Gate Status

Status: `fail`; classification: `partially_fixed`.

| Metric | Value | Threshold | Status |
| --- | --- | --- | --- |
| targeted_production_loc | 165072 | 182431 | pass |
| duplicate_classifier_candidates | 247 | 479 | pass |
| canonical_cli_leaf_command_candidates | 36 | 96 | pass |
| command_specific_option_candidates | 191 | 263 | pass |
| surface_direct_mutation_candidates | 530 | 0 | fail |

All-runtime lexical mutation candidates remain reported separately because the LDR-074 acceptance gate is scoped to CLI/TUI/transport mutation paths.
Legacy derive command attributes remain reported separately because they count Rust metadata annotations rather than canonical operator command leaves.
Legacy derive arg attributes remain reported separately because they count Rust metadata annotations rather than unique operator option names.
Subprocess command names remain reported separately because `Command::new` calls in runtime helpers are not canonical VIDA CLI leaves.

Next slices:

- ldr-074b: reduce canonical CLI leaf and option counts
- ldr-074c: eliminate or classify CLI/TUI/transport direct mutation candidates

## Direct Mutation Candidates

| Path | Line | Entity | Operation | Replacement Operation |
| --- | --- | --- | --- | --- |
| crates/taskflow-authority/src/claims/mod.rs | 372 | claim | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-authority/src/operation_authorization.rs | 109 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-authority/src/scheduler_claim.rs | 476 | claim | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 111 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 112 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/aggregate.rs | 770 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/graph.rs | 56 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/lifecycle.rs | 62 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/split.rs | 34 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/update.rs | 1 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 66 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 809 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 810 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 813 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 816 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 1122 | claim | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 33 | lane_packet | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 40 | claim,task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 94 | claim,task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 2031 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 2037 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 2041 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 2053 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3523 | dispatch_receipt,lane_packet,run_graph_state | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3527 | lane_packet | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4822 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5835 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5908 | lane_packet | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5909 | lane_packet | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5917 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5926 | dispatch_receipt,lane_packet,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6387 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6405 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6413 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6419 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6438 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6480 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6490 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6660 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6799 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6818 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6907 | dispatch_receipt,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6940 | lane_packet | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7008 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7028 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7235 | host_bridge_artifact | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7513 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7703 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7752 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7886 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7941 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 8037 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 8086 | dispatch_receipt,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 8226 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 8342 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 11542 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 302 | run_graph_state,task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 686 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 749 | dispatch_receipt,run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 804 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 815 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 5 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 38 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 41 | lane_packet,task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 45 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 46 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 50 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 58 | lane_packet,run_graph_state,task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 60 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 198 | claim | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1017 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1025 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1040 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1048 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1050 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1067 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1087 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1584 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1593 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1933 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |

## Classifier Candidates

| Path | Line | Function | Replacement Operation |
| --- | --- | --- | --- |
| crates/taskflow-authority/src/projection_cache.rs | 54 | cached_status_projection_has_required_shape | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_evidence.rs | 42 | blocked_source_lane_from_packet_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 95 | normalize_scheduler_reservation_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 64 | run_graph_status_is_terminal_closure | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/terminal_closure.rs | 46 | status_is_terminal_closure_without_next_unit | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 213 | canonical_blocker_code_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 218 | canonical_blocker_code_value_from_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 263 | canonical_parametric_blocker_code_value | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 289 | is_selected_lane_assignment_guard_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 109 | is_fail_closed_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 17 | blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 25 | classify_state_access_error | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 33 | state_access_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 132 | task_status_is_closed_like | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 137 | task_status_is_open_like | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/run_graph/model.rs | 274 | is_blocked_lane | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/run_workflow/mod.rs | 418 | blocked_state_for_lifecycle | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/run_workflow/mod.rs | 587 | is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/run_workflow/mod.rs | 606 | blocked_state | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/task/block.rs | 18 | canonical_task_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/task/block.rs | 30 | canonical_task_blocker_code_segment | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/task/takeover.rs | 53 | classify_exception_takeover_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/task/verify.rs | 400 | task_verify_label_is_runtime_proof_blocker | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/activation_status.rs | 18 | activation_status_is_pending | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 45 | blocker_code_value | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 673 | retryable_host_bridge_completion_request_for_state_root | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 713 | completed_host_bridge_completion_request_for_state_root | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 739 | retryable_host_bridge_completion_request | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 2061 | blocked_candidate | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 3311 | fail_closed_flow_projection_for_continuation_gate | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 491 | has_failure_state_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 518 | has_resolved_failure_artifact_action_context | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 579 | has_unresolved_failure_artifact_context | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 679 | has_failure_state_artifact_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 697 | has_current_failure_outcome_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 780 | has_contrastive_blocker_clause | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/command_pipeline.rs | 260 | blocked_response | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/consume_final_operator_surface.rs | 237 | docflow_verdict_vida_gate_result | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/consume_final_operator_surface.rs | 276 | consume_final_operator_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/continuation_binding_summary.rs | 5 | explicit_binding_is_admissible_for_status | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/continuation_binding_summary.rs | 58 | explicit_task_binding_is_admissible_without_status | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/continuation_binding_summary.rs | 106 | run_graph_status_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/continuation_binding_summary.rs | 355 | dispatch_summary_has_clean_completed_lane | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/contract_profile_adapter.rs | 10 | blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/contract_profile_adapter.rs | 16 | blocker_code_str | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/contract_profile_adapter.rs | 22 | canonical_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/contract_profile_adapter.rs | 105 | operator_contract_status_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/contract_profile_adapter.rs | 160 | classify_compatibility_boundary | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/diagnostics_surface.rs | 69 | recovery_summary_is_completed_terminal_closure_for_task | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/docflow_runtime_verdict.rs | 6 | runtime_blocker_codes_for_docflow_closeout | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/docflow_runtime_verdict.rs | 17 | docflow_runtime_verdict_next_actions | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/docflow_runtime_verdict.rs | 25 | build_docflow_runtime_verdict | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 27 | governance_projection_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 83 | run_graph_status_has_unsupported_architecture_reserved_workflow_boundary | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 147 | trace_evidence_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 283 | doctor_operator_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/external_provider_health.rs | 132 | classify_external_provider_error | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/init_surfaces.rs | 413 | agent_init_dispatch_timeout_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1164 | lane_status_is_terminal_closure_without_next_unit | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1526 | blocked_source_target_from_summary_packet | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1648 | lane_summary_dispatch_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1688 | lane_summary_is_terminal_completed | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1700 | lane_summary_raw_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1736 | canonical_lane_show_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1769 | lane_show_preserves_raw_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1782 | blocked_lane_show_next_action | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3018 | explicit_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3082 | parse_host_bridge_completion_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3112 | host_bridge_completion_result_value_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3186 | supplied_host_bridge_completion_result_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3245 | supplied_host_bridge_completion_result_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3404 | host_bridge_request_is_retryable_completion_state | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3982 | host_bridge_scope_validation_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 4058 | host_bridge_request_has_retryable_completion_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 4088 | host_bridge_request_has_completed_preview_refresh_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/operator_projection_cache.rs | 238 | pass_projection_requires_recompute_without_operator_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/operator_session_projection.rs | 841 | projection_operator_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/orchestrator_session_surface.rs | 381 | classify_sessions | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/orchestrator_session_surface.rs | 388 | classify_sessions_with_liveness | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/release1_contracts.rs | 255 | classify_compatibility_boundary | fold into shared CompletionOutcome/verdict contract |

## Host-Bridge Defect Path Review

The current host-bridge defect path maps to the `host_bridge_artifact`, `dispatch_receipt`, `lane_packet`, and `continuation_binding` entity groups. LDRK implementation should replace direct artifact/receipt mutation with a single command envelope and journaled completion outcome before cutover.

-----
artifact_path: product/spec/ldrk-baseline/drift-map
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-22
schema_version: 1
status: generated
source_path: docs/product/spec/ldrk-baseline/drift-map.md
created_at: 2026-06-22T00:00:00Z
updated_at: 2026-06-22T00:00:00Z
changelog_ref: ldrk-baseline/drift-map.changelog.jsonl
