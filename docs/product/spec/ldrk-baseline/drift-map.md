# LDRK Baseline Drift Map

Status: generated baseline artifact for TaskFlow task `ldr-001`.

## Summary

| Metric | Value |
| --- | --- |
| targeted_production_loc | 280664 |
| direct_surface_mutation_candidates | 1566 |
| duplicate_classifier_candidates | 1596 |
| canonical_cli_leaf_command_candidates | 160 |
| command_specific_option_candidates | 527 |

## Direct Mutation Candidates

| Path | Line | Entity | Operation | Replacement Operation |
| --- | --- | --- | --- | --- |
| crates/taskflow-authority/src/scheduler_claim.rs | 476 | claim | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 91 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-contracts/src/lib.rs | 92 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/graph.rs | 56 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/lifecycle.rs | 62 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/split.rs | 34 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-core/src/task/update.rs | 1 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/taskflow-state/src/lib.rs | 29 | task_record | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida-contracts/src/lib.rs | 30 | lane_packet | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1810 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1816 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1820 | lane_packet | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 1832 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3162 | dispatch_receipt,lane_packet,run_graph_state | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 3166 | lane_packet | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4054 | host_bridge_artifact | insert | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4673 | lane_packet | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4674 | lane_packet | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4682 | lane_packet | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 4691 | dispatch_receipt,lane_packet,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5130 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5148 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5156 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5162 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5181 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5223 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5233 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5397 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5511 | host_bridge_artifact | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5531 | host_bridge_artifact | create_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5787 | host_bridge_artifact | remove_dir_all | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 5975 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6024 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6157 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6212 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6307 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6356 | dispatch_receipt,run_graph_state | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6495 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 6610 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/agent_dispatch_surface.rs | 9264 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 302 | run_graph_state,task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 686 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 749 | dispatch_receipt,run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 804 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/approval_surface.rs | 815 | run_graph_state | persist | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 5 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 36 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 39 | lane_packet,task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 43 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 44 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 48 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 53 | lane_packet,run_graph_state,task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 55 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 189 | claim | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 944 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 952 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 967 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 975 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 977 | task_record | write | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 994 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1014 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1459 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1468 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1805 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 1946 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 2122 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 2178 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 2186 | task_record | record | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 2804 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 3569 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 3570 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 3613 | task_record | append | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 3643 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 3690 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4164 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4167 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4168 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4171 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4174 | task_record | update | route through VidaCommandEnvelope and OperationalJournal port before cutover |
| crates/vida/src/cli.rs | 4200 | task_record | create | route through VidaCommandEnvelope and OperationalJournal port before cutover |

## Classifier Candidates

| Path | Line | Function | Replacement Operation |
| --- | --- | --- | --- |
| crates/taskflow-authority/src/exception_takeover.rs | 49 | lane_status_strategy | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/exception_takeover.rs | 146 | exception_takeover_state_label_keeps_recorded_receipts_blocked_when_gate_blocks | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/exception_takeover.rs | 189 | exception_takeover_state_label_fails_closed_without_recovery_or_supersession | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/exception_takeover.rs | 203 | exception_takeover_state_label_keeps_blocked_takeover_admissible_not_active | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/final_snapshot.rs | 133 | final_snapshot_release_admission_accepts_clean_pass | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/final_snapshot.rs | 140 | final_snapshot_release_admission_rejects_missing_blocked_or_incomplete | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/final_snapshot.rs | 217 | terminal_consume_continue_snapshot_rejects_blocked_or_actionable_outputs | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 41 | cached_status_projection_admissible | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 54 | cached_status_projection_has_required_shape | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 65 | cached_status_projection_matches_current_session | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 148 | cached_status_projection_shape_accepts_summary_or_full_contract_shape | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 178 | cached_status_projection_matches_session_or_worktree | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 204 | cached_status_projection_rejects_sessionless_cache_even_with_state_marker | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/projection_cache.rs | 231 | cached_status_projection_admissible_requires_status_surface_and_session_match | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_evidence.rs | 42 | blocked_source_lane_from_packet_evidence | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_evidence.rs | 104 | blocked_source_lane_uses_packet_facts_without_runtime_paths | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_evidence.rs | 126 | terminal_source_lane_with_ready_downstream_is_not_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_transition.rs | 77 | run_graph_authority_rejects_blocked_downstream_ready_handoff | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/run_graph_transition.rs | 131 | status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 95 | normalize_scheduler_reservation_blocker_codes | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 304 | scheduler_reservation_collision_classifies_duplicate_task_and_domain | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 340 | scheduler_reservation_blocker_codes_are_canonical | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 373 | orchestrator_claim_request_validation_fails_closed | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/scheduler_claim.rs | 417 | orchestrator_claim_classifies_task_domain_and_path_conflicts | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 23 | missing_task_stale_blocked_run_can_retire | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 64 | run_graph_status_is_terminal_closure | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 83 | active_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 92 | terminal_closure_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 102 | missing_task_stale_blocked_run_accepts_blocked_or_running_lane_receipt | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 118 | missing_task_stale_blocked_run_accepts_prelaunch_packet_ready_receipt | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 132 | missing_task_stale_blocked_run_rejects_terminal_closure_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 146 | missing_task_stale_blocked_run_rejects_unrelated_receipt_shape | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/stale_guard.rs | 198 | latest_run_graph_task_stale_for_write_guard_matches_status_formula | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_attempts.rs | 77 | normalize_task_attempt_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_attempts.rs | 249 | task_attempt_rollup_classifies_accepted_rejected_partial_and_stale_attempts | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_attempts.rs | 348 | task_attempt_statuses_fail_closed | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 43 | blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 104 | lifecycle_status_from_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 111 | next_actions_for_blockers | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 164 | lifecycle_authority_blocks_parent_close_when_children_remain | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 183 | lifecycle_authority_defers_admitted_decision_without_recomputing_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 203 | lifecycle_authority_preserves_graph_blocker_codes_and_actions | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/task_transition.rs | 229 | lifecycle_authority_normalizes_status_parse_errors_to_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/terminal_closure.rs | 45 | status_is_terminal_closure | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/terminal_closure.rs | 58 | terminal_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-authority/src/terminal_closure.rs | 117 | terminal_missing_task_closure_rejects_non_terminal_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 208 | canonical_blocker_code_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 213 | canonical_blocker_code_value_from_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 258 | canonical_parametric_blocker_code_value | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 274 | selected_lane_assignment_guard_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 284 | is_selected_lane_assignment_guard_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 299 | blocker_code_round_trips_canonical_strings | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 313 | blocker_code_list_dedupes_and_sorts | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 329 | blocker_code_dynamic_lane_assignment_codes_stay_canonical | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 356 | blocker_code_rejects_unknown_strings | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/blocker_code.rs | 361 | blocker_code_legacy_preserving_list_keeps_unknown_values | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 92 | blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 109 | is_fail_closed_blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 136 | blocked | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 206 | legacy_passthrough | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 257 | legacy_passthrough | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 436 | blocked_response_is_fail_closed_without_outputs | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 477 | transition_contract_blocked_decision_is_fail_closed | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/decision_table.rs | 510 | transition_contract_matrix_preserves_outcome_status_and_legacy_edges | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 163 | canonical_approval_status_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 170 | canonical_lane_status_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 175 | canonical_release1_contract_status_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 182 | release1_contract_status_str | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 199 | approval_status_round_trips | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 210 | lane_status_round_trips | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-contracts/src/status_code.rs | 225 | release1_contract_status_round_trips | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 17 | blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 25 | classify_state_access_error | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 33 | state_access_blocker_code | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/continue_use_case.rs | 61 | state_access_error_classification_distinguishes_locks_from_open_failures | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/consume/resume_state_machine.rs | 80 | resume_lifecycle_allows_blocked_retry_path | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 102 | normalize_task_status_token | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 107 | parse_task_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 121 | canonical_task_status | fold into shared CompletionOutcome/verdict contract |
| crates/taskflow-core/src/lib.rs | 126 | task_status_is_closed_like | fold into shared CompletionOutcome/verdict contract |

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
