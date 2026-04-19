fn explicit_binding_is_admissible_for_status(
    binding: &crate::state_store::RunGraphContinuationBinding,
    status: &crate::state_store::RunGraphStatus,
) -> bool {
    let binding_kind = binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str);

    if binding.run_id != status.run_id {
        return binding.binding_source == "explicit_continuation_bind_task"
            && binding_kind == Some("task_graph_task");
    }
    if status.status != "completed" {
        return true;
    }

    let terminal_completed_without_next_unit = status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none();

    if terminal_completed_without_next_unit {
        return matches!(binding_kind, Some("task_graph_task"));
    }

    matches!(
        binding_kind,
        Some("downstream_dispatch_target") | Some("task_graph_task")
    )
}

pub(crate) fn build_continuation_binding_summary(
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    evidence_ambiguous: bool,
) -> serde_json::Value {
    let active_run_id = latest_run_graph_status.map(|status| status.run_id.as_str());
    let delegated_cycle_open = latest_run_graph_recovery
        .is_some_and(|recovery| recovery.delegation_gate.delegated_cycle_open);
    let continuation_required_now = delegated_cycle_open;
    let pause_boundary_gate = if delegated_cycle_open {
        "non_blocking_only"
    } else {
        "allowed_if_no_further_bound_work_is_evidenced"
    };
    let continuation_next_actions = active_run_id
        .filter(|run_id| !run_id.trim().is_empty())
        .map(|run_id| {
            vec![
                "Do not stop on commentary, status output, or intermediate reporting while the delegated cycle is still open."
                    .to_string(),
                format!(
                    "Continue the active bounded unit with `vida taskflow consume continue --run-id {run_id} --json`."
                ),
                format!(
                    "Inspect the live delegated-cycle recovery state with `vida taskflow recovery status {run_id} --json` if routing context is needed before the next step."
                ),
            ]
        })
        .unwrap_or_default();
    if evidence_ambiguous {
        return serde_json::json!({
            "status": "ambiguous",
            "continuation_allowed": false,
            "continuation_required_now": false,
            "active_bounded_unit": serde_json::Value::Null,
            "binding_source": serde_json::Value::Null,
            "why_this_unit": serde_json::Value::Null,
            "primary_path": "diagnosis_path",
            "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
            "pause_boundary_gate": "forbidden_while_ambiguous",
            "ambiguity_reason": "runtime_evidence_ambiguous",
            "next_actions": [
                "Do not continue by heuristic while run-graph continuation evidence is ambiguous.",
                "Refresh continuation evidence with `vida taskflow consume continue --json` and recheck `vida status --json` before selecting the next bounded step."
            ]
        });
    }

    let sequential_vs_parallel_posture = if latest_run_graph_recovery
        .is_some_and(|recovery| recovery.delegation_gate.delegated_cycle_open)
    {
        "sequential_only_open_cycle"
    } else {
        "sequential_only"
    };

    if let Some(status) = latest_run_graph_status {
        let terminal_completed_without_next_unit = status.status == "completed"
            && status.lifecycle_stage == "closure_complete"
            && status
                .next_node
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none();

        if let Some(binding) = explicit_binding {
            if explicit_binding_is_admissible_for_status(binding, status) {
                return serde_json::json!({
                    "status": binding.status,
                    "continuation_allowed": binding.status == "bound",
                    "continuation_required_now": continuation_required_now,
                    "active_bounded_unit": binding.active_bounded_unit,
                    "binding_source": binding.binding_source,
                    "why_this_unit": binding.why_this_unit,
                    "primary_path": binding.primary_path,
                    "sequential_vs_parallel_posture": binding.sequential_vs_parallel_posture,
                    "pause_boundary_gate": pause_boundary_gate,
                    "ambiguity_reason": serde_json::Value::Null,
                    "next_actions": if continuation_required_now {
                        continuation_next_actions.clone()
                    } else {
                        Vec::<String>::new()
                    }
                });
            }
        }

        if status.status != "completed" {
            return serde_json::json!({
                "status": "bound",
                "continuation_allowed": true,
                "continuation_required_now": continuation_required_now,
                "active_bounded_unit": {
                    "kind": "run_graph_task",
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": status.active_node,
                },
                "binding_source": "latest_run_graph_status",
                "why_this_unit": format!(
                    "Latest runtime state is still active for task `{}` at node `{}`.",
                    status.task_id, status.active_node
                ),
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": sequential_vs_parallel_posture,
                "pause_boundary_gate": pause_boundary_gate,
                "ambiguity_reason": serde_json::Value::Null,
                "next_actions": if continuation_required_now {
                    continuation_next_actions.clone()
                } else {
                    Vec::<String>::new()
                }
            });
        }

        if !terminal_completed_without_next_unit {
            if let Some(receipt) = latest_run_graph_dispatch_receipt {
                let downstream_target = receipt
                    .downstream_dispatch_target
                    .as_deref()
                    .filter(|value| !value.trim().is_empty());
                let downstream_status_ready = matches!(
                    receipt.downstream_dispatch_status.as_deref(),
                    Some("packet_ready") | Some("executed")
                );
                if receipt.downstream_dispatch_ready && downstream_status_ready {
                    if let Some(dispatch_target) = downstream_target {
                        return serde_json::json!({
                            "status": "bound",
                            "continuation_allowed": true,
                            "continuation_required_now": false,
                            "active_bounded_unit": {
                                "kind": "downstream_dispatch_target",
                                "task_id": status.task_id,
                                "run_id": status.run_id,
                                "dispatch_target": dispatch_target,
                            },
                            "binding_source": "latest_run_graph_dispatch_receipt",
                            "why_this_unit": format!(
                                "Latest dispatch receipt explicitly names downstream target `{}` as the next lawful bounded unit.",
                                dispatch_target
                            ),
                            "primary_path": "normal_delivery_path",
                            "sequential_vs_parallel_posture": "sequential_only_downstream_bound",
                            "pause_boundary_gate": "allowed_if_no_further_bound_work_is_evidenced",
                            "ambiguity_reason": serde_json::Value::Null,
                            "next_actions": []
                        });
                    }
                }
            }
        }

        return serde_json::json!({
            "status": "ambiguous",
            "continuation_allowed": false,
            "continuation_required_now": false,
            "active_bounded_unit": serde_json::Value::Null,
            "binding_source": serde_json::Value::Null,
            "why_this_unit": serde_json::Value::Null,
            "primary_path": "diagnosis_path",
            "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
            "pause_boundary_gate": "forbidden_without_explicit_next_unit",
            "ambiguity_reason": "completed_without_explicit_next_bounded_unit",
            "next_actions": [
                "Do not continue by selecting the next ready task heuristically after a completed bounded slice.",
                "Either cite the explicit next bounded unit from the user, bind it with `vida taskflow continuation bind <run-id> --task-id <task-id> --json`, or refresh runtime evidence with `vida taskflow consume continue --json` before further implementation."
            ]
        });
    }

    serde_json::json!({
        "status": "ambiguous",
        "continuation_allowed": false,
        "continuation_required_now": false,
        "active_bounded_unit": serde_json::Value::Null,
        "binding_source": serde_json::Value::Null,
        "why_this_unit": serde_json::Value::Null,
        "primary_path": "diagnosis_path",
        "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
        "pause_boundary_gate": "forbidden_without_runtime_evidence",
        "ambiguity_reason": "missing_active_bounded_unit_runtime_evidence",
        "next_actions": [
            "Do not continue by plausibility when runtime state does not expose an explicit active bounded unit.",
            "Bind the bounded unit explicitly from user intent with `vida taskflow continuation bind <run-id> --task-id <task-id> --json` or refresh runtime evidence before implementation continues."
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::build_continuation_binding_summary;

    #[test]
    fn active_run_graph_status_binds_current_bounded_unit() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "implementation".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "implementation_active".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "implementation".to_string(),
            lifecycle_stage: "implementation_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementation_lane".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementation".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "implementation".to_string(),
                lifecycle_stage: "implementation_active".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };

        let summary =
            build_continuation_binding_summary(None, Some(&status), Some(&recovery), None, false);

        assert_eq!(summary["status"], "bound");
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            serde_json::Value::String("task-1".to_string())
        );
        assert_eq!(summary["binding_source"], "latest_run_graph_status");
        assert_eq!(summary["continuation_required_now"], true);
        assert_eq!(summary["pause_boundary_gate"], "non_blocking_only");
        assert!(summary["next_actions"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row
                .as_str()
                .is_some_and(|value| value.contains("consume continue --run-id task-1 --json")))));
    }

    #[test]
    fn completed_status_without_explicit_next_unit_is_ambiguous() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(
            summary["ambiguity_reason"],
            "completed_without_explicit_next_bounded_unit"
        );
    }

    #[test]
    fn explicit_binding_is_preferred_when_present() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "pm".to_string();
        status.status = "running".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "task-1",
                "active_node": "pm"
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "explicit".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("req".to_string()),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };

        let summary =
            build_continuation_binding_summary(Some(&binding), Some(&status), None, None, false);

        assert_eq!(summary["binding_source"], "explicit_continuation_bind");
        assert_eq!(summary["why_this_unit"], "explicit");
    }

    #[test]
    fn completed_status_accepts_explicit_task_graph_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "task-1".to_string(),
            task_id: "tf-post-r1-main-carveout".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "task_id": "tf-post-r1-main-carveout",
                "run_id": "task-1",
                "task_status": "in_progress",
                "issue_type": "task"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "user explicitly selected the active epic".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-13T10:00:00Z".to_string(),
        };

        let summary =
            build_continuation_binding_summary(Some(&binding), Some(&status), None, None, false);

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["binding_source"], "explicit_continuation_bind_task");
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "tf-post-r1-main-carveout"
        );
    }

    #[test]
    fn completed_status_rejects_stale_run_graph_task_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();

        let stale_binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "task-1",
                "active_node": "implementation"
            }),
            binding_source: "run_graph_advance".to_string(),
            why_this_unit: "stale active binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-13T10:00:00Z".to_string(),
        };

        let summary = build_continuation_binding_summary(
            Some(&stale_binding),
            Some(&status),
            None,
            None,
            false,
        );

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(
            summary["ambiguity_reason"],
            "completed_without_explicit_next_bounded_unit"
        );
    }

    #[test]
    fn explicit_task_graph_binding_from_different_run_is_preferred_over_latest_status() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-child",
            "implementation",
            "implementation",
        );
        status.active_node = "implementer".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-upstream".to_string(),
            task_id: "task-upstream".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "task_id": "task-upstream",
                "run_id": "run-upstream",
                "task_status": "in_progress",
                "issue_type": "task"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "operator explicitly rebound work to the upstream task".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-16T09:00:00Z".to_string(),
        };

        let summary =
            build_continuation_binding_summary(Some(&binding), Some(&status), None, None, false);

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["binding_source"], "explicit_continuation_bind_task");
        assert_eq!(summary["active_bounded_unit"]["task_id"], "task-upstream");
        assert_eq!(
            summary["sequential_vs_parallel_posture"],
            "sequential_only_explicit_task_bound"
        );
    }
}
