# LDRK Baseline Drift Map

Status: generated baseline artifact for TaskFlow task `ldr-001`.

## Summary

| Metric | Value |
| --- | --- |
| targeted_production_loc | 161388 |
| direct_surface_mutation_candidates | 1621 |
| duplicate_classifier_candidates | 260 |
| status_helper_false_positive_candidates | 409 |
| cfg_test_classifier_candidates | 276 |
| cfg_test_status_helper_candidates | 767 |
| canonical_cli_leaf_command_candidates | 160 |
| command_specific_option_candidates | 530 |

## Direct Mutation Candidates

| Path | Line | Entity | Operation | Replacement Operation |
| --- | --- | --- | --- | --- |
| crates/taskflow-authority/src/claims/mod.rs | 372 | claim | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-authority/src/operation_authorization.rs | 109 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-authority/src/scheduler_claim.rs | 476 | claim | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 111 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 112 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/aggregate.rs | 688 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/graph.rs | 56 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/lifecycle.rs | 62 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/split.rs | 34 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/update.rs | 1 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 66 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 707 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 708 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 711 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 714 | claim | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 982 | claim | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 33 | lane_packet | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 40 | claim,task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1961 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1967 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1971 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1983 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3285 | dispatch_receipt,lane_packet,run_graph_state | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3289 | lane_packet | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4204 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4859 | lane_packet | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4860 | lane_packet | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4868 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4877 | dispatch_receipt,lane_packet,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5316 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5334 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5342 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5348 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5367 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5409 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5419 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5583 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5715 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5734 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5800 | dispatch_receipt,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5833 | lane_packet | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5906 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5926 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6133 | host_bridge_artifact | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6393 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6581 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6630 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6763 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6818 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6913 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6962 | dispatch_receipt,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7101 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 7216 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 9946 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 302 | run_graph_state,task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 686 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 749 | dispatch_receipt,run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 804 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 815 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 5 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 37 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 40 | lane_packet,task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 44 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 45 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 49 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 54 | lane_packet,run_graph_state,task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 56 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 191 | claim | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 947 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 955 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 970 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 978 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 980 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 997 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1017 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1469 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1478 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1818 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1959 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 2141 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |

## Classifier Candidates

| Path | Line | Function | Replacement Operation |
| --- | --- | --- | --- |
| crates/taskflow-authority/src/projection_cache.rs | 54 | cached_status_projection_has_required_shape | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_evidence.rs | 42 | blocked_source_lane_from_packet_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 95 | normalize_scheduler_reservation_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 64 | run_graph_status_is_terminal_closure | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/terminal_closure.rs | 45 | status_is_terminal_closure | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 208 | canonical_blocker_code_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 213 | canonical_blocker_code_value_from_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 258 | canonical_parametric_blocker_code_value | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 284 | is_selected_lane_assignment_guard_blocked | fold into shared CompletionOutcome/verdict contract |
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
| crates/vida/src/agent_dispatch_surface.rs | 1991 | blocked_candidate | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_dispatch_surface.rs | 3108 | fail_closed_flow_projection_for_continuation_gate | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 481 | has_failure_state_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 508 | has_resolved_failure_artifact_action_context | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 569 | has_unresolved_failure_artifact_context | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 669 | has_failure_state_artifact_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 687 | has_current_failure_outcome_language | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/agent_feedback_surface.rs | 770 | has_contrastive_blocker_clause | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/command_pipeline.rs | 171 | blocked_response | fold into shared CompletionOutcome/verdict contract |
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
| crates/vida/src/doctor_surface.rs | 24 | governance_projection_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 80 | run_graph_status_has_unsupported_architecture_reserved_workflow_boundary | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 139 | trace_evidence_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/doctor_surface.rs | 275 | doctor_operator_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/init_surfaces.rs | 413 | agent_init_dispatch_timeout_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1399 | blocked_source_target_from_summary_packet | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1521 | lane_summary_dispatch_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1562 | lane_summary_is_terminal_completed | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1574 | lane_summary_raw_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1610 | canonical_lane_show_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1643 | lane_show_preserves_raw_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 1656 | blocked_lane_show_next_action | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 2892 | explicit_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 2956 | parse_host_bridge_completion_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 2986 | host_bridge_completion_result_value_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3036 | supplied_host_bridge_completion_result_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3095 | supplied_host_bridge_completion_result_is_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3232 | host_bridge_request_is_retryable_completion_state | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3810 | host_bridge_scope_validation_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3823 | host_bridge_completion_summary_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3904 | host_bridge_request_has_retryable_completion_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/lane_surface.rs | 3934 | host_bridge_request_has_completed_preview_refresh_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/operator_projection_cache.rs | 238 | pass_projection_requires_recompute_without_operator_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/operator_session_projection.rs | 800 | projection_operator_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/orchestrator_session_surface.rs | 381 | classify_sessions | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/orchestrator_session_surface.rs | 388 | classify_sessions_with_liveness | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/release1_contracts.rs | 255 | classify_compatibility_boundary | fold into shared CompletionOutcome/verdict contract |
| crates/vida/src/release1_contracts.rs | 805 | closure_admission_row_is_pass | fold into shared CompletionOutcome/verdict contract |

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
