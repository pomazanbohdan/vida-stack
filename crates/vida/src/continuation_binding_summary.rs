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

fn run_graph_status_is_blocked(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "blocked" || normalized == "lane_blocked" || normalized.ends_with("_blocked")
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

        if run_graph_status_is_blocked(&status.status) {
            return serde_json::json!({
                "status": "ambiguous",
                "continuation_allowed": false,
                "continuation_required_now": false,
                "active_bounded_unit": serde_json::Value::Null,
                "binding_source": serde_json::Value::Null,
                "why_this_unit": serde_json::Value::Null,
                "primary_path": "diagnosis_path",
                "sequential_vs_parallel_posture": "unknown_until_run_graph_blocker_resolved",
                "pause_boundary_gate": "forbidden_while_run_graph_status_blocked",
                "ambiguity_reason": "latest_run_graph_status_blocked",
                "blocked_run_graph_status": {
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": status.active_node,
                    "status": status.status,
                    "lifecycle_stage": status.lifecycle_stage,
                },
                "next_actions": [
                    "Do not continue normal delivery while the latest run-graph status is blocked.",
                    "Inspect the blocked run-graph status with `vida taskflow recovery status <run-id> --json` and resolve the blocker before writing.",
                    "After the blocker is resolved, refresh continuation evidence with `vida taskflow consume continue --json` or bind the next bounded unit explicitly."
                ]
            });
        }

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

pub(crate) fn taskflow_active_candidates_from_tasks(
    tasks: &[crate::state_store::TaskRecord],
) -> Vec<serde_json::Value> {
    tasks
        .iter()
        .filter(|task| task.status == "in_progress")
        .map(|task| {
            serde_json::json!({
                "task_id": task.id,
                "display_id": task.display_id,
                "status": task.status,
                "issue_type": task.issue_type,
                "title": task.title,
            })
        })
        .collect()
}

pub(crate) fn add_taskflow_active_work_truth(
    mut summary: serde_json::Value,
    taskflow_active_candidates: Vec<serde_json::Value>,
) -> serde_json::Value {
    let binding_source = summary
        .get("binding_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let active_bounded_unit = summary.get("active_bounded_unit").cloned();
    let run_graph_task_id = active_bounded_unit
        .as_ref()
        .and_then(|unit| unit.get("task_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let bound_to_run_graph_task = active_bounded_unit
        .as_ref()
        .and_then(|unit| unit.get("kind"))
        .and_then(serde_json::Value::as_str)
        == Some("run_graph_task");
    let active_candidate_matches = run_graph_task_id.as_deref().is_some_and(|task_id| {
        taskflow_active_candidates.iter().any(|candidate| {
            candidate.get("task_id").and_then(serde_json::Value::as_str) == Some(task_id)
        })
    });
    let orthogonal = bound_to_run_graph_task
        && !taskflow_active_candidates.is_empty()
        && !active_candidate_matches;

    if let serde_json::Value::Object(object) = &mut summary {
        object.insert(
            "binding_scope".to_string(),
            serde_json::Value::String(
                if binding_source.starts_with("latest_run_graph") {
                    "run_graph_latest"
                } else if binding_source.contains("task") {
                    "taskflow_explicit"
                } else if binding_source.is_empty() {
                    "unbound"
                } else {
                    "run_graph_explicit"
                }
                .to_string(),
            ),
        );
        object.insert(
            "taskflow_active_candidates".to_string(),
            serde_json::Value::Array(taskflow_active_candidates.clone()),
        );
        object.insert(
            "orthogonal_to_taskflow_active_work".to_string(),
            serde_json::Value::Bool(orthogonal),
        );
    }

    if !orthogonal {
        return summary;
    }

    if let serde_json::Value::Object(object) = &mut summary {
        object.insert(
            "status".to_string(),
            serde_json::Value::String("ambiguous".to_string()),
        );
        object.insert(
            "continuation_allowed".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert("active_bounded_unit".to_string(), serde_json::Value::Null);
        object.insert(
            "run_graph_latest_binding".to_string(),
            active_bounded_unit.unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "binding_scope".to_string(),
            serde_json::Value::String(
                "run_graph_latest_orthogonal_to_taskflow_active_work".to_string(),
            ),
        );
        object.insert(
            "ambiguity_reason".to_string(),
            serde_json::Value::String(
                "latest_run_graph_binding_orthogonal_to_taskflow_active_work".to_string(),
            ),
        );
        object.insert("why_this_unit".to_string(), serde_json::Value::Null);
        object.insert(
            "primary_path".to_string(),
            serde_json::Value::String("diagnosis_path".to_string()),
        );
        object.insert(
            "sequential_vs_parallel_posture".to_string(),
            serde_json::Value::String("unknown_until_explicit_taskflow_binding".to_string()),
        );
        let mut next_actions = object
            .get("next_actions")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        next_actions.insert(
            0,
            serde_json::Value::String(
                "Do not assume the latest run-graph binding is the active bounded unit while TaskFlow has different in-progress task candidates.".to_string(),
            ),
        );
        next_actions.insert(
            1,
            serde_json::Value::String(
                "Bind the intended TaskFlow active task explicitly with `vida taskflow continuation bind <run-id> --task-id <task-id> --json` or close/reconcile stale in-progress tasks before writing.".to_string(),
            ),
        );
        object.insert(
            "next_actions".to_string(),
            serde_json::Value::Array(next_actions),
        );
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::{
        add_taskflow_active_work_truth, build_continuation_binding_summary,
        taskflow_active_candidates_from_tasks,
    };

    fn task_record(task_id: &str, status: &str) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: format!("Task {task_id}"),
            description: String::new(),
            status: status.to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies: Vec::new(),
        }
    }

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
    fn blocked_latest_run_graph_status_fails_closed() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "implementation",
            "implementation",
        );
        status.task_id = "runtime-audit-state-store-init-lock-timeout".to_string();
        status.run_id = "run-blocked".to_string();
        status.active_node = "implementer".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementation_blocked".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(summary["continuation_allowed"], false);
        assert_eq!(summary["continuation_required_now"], false);
        assert_eq!(summary["active_bounded_unit"], serde_json::Value::Null);
        assert_eq!(summary["binding_source"], serde_json::Value::Null);
        assert_eq!(summary["primary_path"], "diagnosis_path");
        assert_eq!(
            summary["ambiguity_reason"],
            "latest_run_graph_status_blocked"
        );
        assert_eq!(
            summary["blocked_run_graph_status"]["task_id"],
            "runtime-audit-state-store-init-lock-timeout"
        );
        assert_eq!(summary["blocked_run_graph_status"]["run_id"], "run-blocked");
        assert!(summary["next_actions"].as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row.as_str()
                    .is_some_and(|value| value.contains("resolve the blocker"))
            })
        }));
    }

    #[test]
    fn blocked_latest_run_graph_status_rejects_explicit_normal_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "implementer".to_string();
        status.status = "lane_blocked".to_string();
        status.lifecycle_stage = "implementation_blocked".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "task-1",
                "active_node": "implementer"
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "explicit".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-26T10:00:00Z".to_string(),
        };

        let summary =
            build_continuation_binding_summary(Some(&binding), Some(&status), None, None, false);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(summary["continuation_allowed"], false);
        assert_eq!(summary["binding_source"], serde_json::Value::Null);
        assert_eq!(summary["primary_path"], "diagnosis_path");
        assert_eq!(
            summary["ambiguity_reason"],
            "latest_run_graph_status_blocked"
        );
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

    #[test]
    fn taskflow_active_work_truth_marks_latest_run_graph_binding_orthogonal() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-run-closure-validation-proof-feature-task",
            "implementation",
            "implementation",
        );
        status.task_id = "runtime-run-closure-validation-proof-feature-task".to_string();
        status.active_node = "implementer".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "implementer_active".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);
        let taskflow_candidates = taskflow_active_candidates_from_tasks(&[task_record(
            "audit-p1-current-task",
            "in_progress",
        )]);
        let summary = add_taskflow_active_work_truth(summary, taskflow_candidates);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(summary["continuation_allowed"], false);
        assert_eq!(
            summary["ambiguity_reason"],
            "latest_run_graph_binding_orthogonal_to_taskflow_active_work"
        );
        assert_eq!(
            summary["binding_scope"],
            "run_graph_latest_orthogonal_to_taskflow_active_work"
        );
        assert_eq!(summary["orthogonal_to_taskflow_active_work"], true);
        assert_eq!(
            summary["run_graph_latest_binding"]["task_id"],
            "runtime-run-closure-validation-proof-feature-task"
        );
        assert_eq!(
            summary["taskflow_active_candidates"][0]["task_id"],
            "audit-p1-current-task"
        );
        assert!(summary["next_actions"].as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row.as_str().is_some_and(|value| {
                    value.contains("Do not assume the latest run-graph binding")
                })
            })
        }));
    }

    #[test]
    fn taskflow_active_work_truth_preserves_matching_latest_run_graph_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.task_id = "task-1".to_string();
        status.active_node = "implementer".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "implementer_active".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);
        let taskflow_candidates =
            taskflow_active_candidates_from_tasks(&[task_record("task-1", "in_progress")]);
        let summary = add_taskflow_active_work_truth(summary, taskflow_candidates);

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(summary["binding_scope"], "run_graph_latest");
        assert_eq!(summary["orthogonal_to_taskflow_active_work"], false);
        assert_eq!(summary["active_bounded_unit"]["task_id"], "task-1");
    }
}
