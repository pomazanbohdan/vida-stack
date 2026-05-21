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
        return binding.binding_source != "explicit_continuation_bind_task"
            && binding_kind != Some("task_graph_task");
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

fn dispatch_prelaunch_binding_matches_blocked_status(
    binding: &crate::state_store::RunGraphContinuationBinding,
    status: &crate::state_store::RunGraphStatus,
) -> bool {
    let binding_kind = binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str);

    binding.status == "bound"
        && binding.run_id == status.run_id
        && binding.task_id == status.task_id
        && binding.binding_source == "dispatch_prelaunch_blocked"
        && binding_kind == Some("task_graph_task")
}

fn explicit_task_binding_is_admissible_without_status(
    binding: &crate::state_store::RunGraphContinuationBinding,
) -> bool {
    let binding_kind = binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str);
    let task_id = binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    binding.status == "bound"
        && binding.binding_source == "explicit_continuation_bind_task"
        && binding_kind == Some("task_graph_task")
        && task_id.is_some()
}

fn run_graph_status_is_blocked(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "blocked" || normalized == "lane_blocked" || normalized.ends_with("_blocked")
}

fn active_exception_takeover_evidence_matches_status(
    status: &crate::state_store::RunGraphStatus,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    terminal_continue_run_id: Option<&str>,
) -> bool {
    let Some(dispatch) = dispatch else {
        return false;
    };
    if terminal_continue_run_id == Some(status.run_id.as_str()) {
        return false;
    }
    let exception_takeover_state = crate::release1_contracts::exception_takeover_state(
        dispatch.exception_path_receipt_id.as_deref(),
        dispatch.supersedes_receipt_id.as_deref(),
        None,
    );
    dispatch.run_id == status.run_id
        && (dispatch.lane_status == "lane_exception_takeover"
            || exception_takeover_state.is_active())
        && dispatch
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && dispatch
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn active_exception_takeover_binding_matches_status(
    binding: &crate::state_store::RunGraphContinuationBinding,
    status: &crate::state_store::RunGraphStatus,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    let binding_kind = binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str);

    binding.status == "bound"
        && binding.run_id == status.run_id
        && binding.task_id == status.task_id
        && crate::taskflow_continuation::is_downstream_chain_continuation_binding_source(
            &binding.binding_source,
        )
        && binding_kind == Some("run_graph_task")
        && active_exception_takeover_evidence_matches_status(status, dispatch, None)
}

fn active_exception_takeover_binding_summary_json(
    binding: &crate::state_store::RunGraphContinuationBinding,
    status: &crate::state_store::RunGraphStatus,
    continuation_required_now: bool,
    pause_boundary_gate: &str,
) -> serde_json::Value {
    let continuation_resumable = exception_takeover_continuation_resumable(status);
    serde_json::json!({
        "status": binding.status,
        "continuation_allowed": binding.status == "bound",
        "continuation_resumable": continuation_resumable,
        "resume_blocker": if continuation_resumable {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(exception_takeover_resume_blocker(status).to_string())
        },
        "continuation_required_now": continuation_required_now,
        "active_bounded_unit": binding.active_bounded_unit,
        "binding_source": binding.binding_source,
        "why_this_unit": binding.why_this_unit,
        "primary_path": binding.primary_path,
        "sequential_vs_parallel_posture": binding.sequential_vs_parallel_posture,
        "pause_boundary_gate": pause_boundary_gate,
        "ambiguity_reason": serde_json::Value::Null,
        "active_exception_takeover": true,
        "next_actions": active_exception_takeover_next_actions(status)
    })
}

fn active_exception_takeover_status_summary_json(
    status: &crate::state_store::RunGraphStatus,
    continuation_required_now: bool,
    pause_boundary_gate: &str,
) -> serde_json::Value {
    let continuation_resumable = exception_takeover_continuation_resumable(status);
    serde_json::json!({
        "status": "bound",
        "continuation_allowed": true,
        "continuation_resumable": continuation_resumable,
        "resume_blocker": if continuation_resumable {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(exception_takeover_resume_blocker(status).to_string())
        },
        "continuation_required_now": continuation_required_now,
        "active_bounded_unit": {
            "kind": "run_graph_task",
            "task_id": status.task_id,
            "run_id": status.run_id,
            "active_node": status.active_node,
        },
        "binding_source": "latest_run_graph_exception_takeover_dispatch",
        "why_this_unit": format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}` at node `{}`.",
            status.task_id, status.active_node
        ),
        "primary_path": "normal_delivery_path",
        "sequential_vs_parallel_posture": "sequential_only_exception_takeover",
        "pause_boundary_gate": pause_boundary_gate,
        "ambiguity_reason": serde_json::Value::Null,
        "active_exception_takeover": true,
        "next_actions": active_exception_takeover_next_actions(status)
    })
}

fn active_exception_takeover_next_actions(
    status: &crate::state_store::RunGraphStatus,
) -> Vec<String> {
    if exception_takeover_continuation_resumable(status) {
        return vec![format!(
            "Continue the active exception-backed bounded unit with `vida taskflow consume continue --run-id {} --json`.",
            status.run_id
        )];
    }
    if status.resume_target == "none" || !status.resume_target.starts_with("dispatch.") {
        return vec![
            format!(
                "Inspect the active recovery state with `vida taskflow recovery status {} --json` before attempting resume.",
                status.run_id
            ),
            crate::status_surface_signals::recovery_resume_target_missing_next_action(
                Some(status.run_id.as_str()),
                Some(status.task_id.as_str()),
            ),
        ];
    }
    vec![
        format!(
            "Inspect the active exception-takeover scope with `vida lane show {} --json` before attempting resume.",
            status.run_id
        ),
        format!(
            "Do not run `vida taskflow consume continue --run-id {} --json` until recovery_ready is true and resume_target is a dispatch target.",
            status.run_id
        ),
    ]
}

fn exception_takeover_continuation_resumable(status: &crate::state_store::RunGraphStatus) -> bool {
    status.recovery_ready && status.resume_target.starts_with("dispatch.")
}

fn exception_takeover_resume_blocker(status: &crate::state_store::RunGraphStatus) -> &'static str {
    if status.resume_target == "none" || !status.resume_target.starts_with("dispatch.") {
        "next_action_target_missing"
    } else {
        "recovery_ready_false"
    }
}

fn binding_summary_json(
    binding: &crate::state_store::RunGraphContinuationBinding,
    continuation_required_now: bool,
    pause_boundary_gate: &str,
    continuation_next_actions: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
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
            continuation_next_actions
        } else {
            Vec::<String>::new()
        }
    })
}

pub(crate) fn build_continuation_binding_summary(
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    terminal_consume_continue_run_id: Option<&str>,
    evidence_ambiguous: bool,
) -> serde_json::Value {
    build_continuation_binding_summary_with_idle_policy(
        explicit_binding,
        latest_run_graph_status,
        latest_run_graph_recovery,
        latest_run_graph_dispatch_receipt,
        terminal_consume_continue_run_id,
        evidence_ambiguous,
        false,
        false,
    )
}

pub(crate) fn build_continuation_binding_summary_with_idle_policy(
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    terminal_consume_continue_run_id: Option<&str>,
    evidence_ambiguous: bool,
    terminal_completed_without_next_unit_is_idle: bool,
    latest_run_graph_task_closed: bool,
) -> serde_json::Value {
    build_continuation_binding_summary_with_task_authority(
        explicit_binding,
        latest_run_graph_status,
        latest_run_graph_recovery,
        latest_run_graph_dispatch_receipt,
        terminal_consume_continue_run_id,
        evidence_ambiguous,
        terminal_completed_without_next_unit_is_idle,
        latest_run_graph_task_closed,
        false,
    )
}

pub(crate) fn build_continuation_binding_summary_with_task_authority(
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    terminal_consume_continue_run_id: Option<&str>,
    evidence_ambiguous: bool,
    terminal_completed_without_next_unit_is_idle: bool,
    latest_run_graph_task_closed: bool,
    latest_run_graph_task_missing: bool,
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
    if let Some(status) = latest_run_graph_status {
        if latest_run_graph_task_missing && run_graph_status_is_blocked(&status.status) {
            return serde_json::json!({
                "status": "idle",
                "continuation_allowed": false,
                "continuation_required_now": false,
                "active_bounded_unit": serde_json::Value::Null,
                "binding_source": serde_json::Value::Null,
                "why_this_unit": format!(
                    "Latest run `{}` is blocked but its task `{}` is missing from authoritative TaskFlow state.",
                    status.run_id, status.task_id
                ),
                "primary_path": "taskflow_selection_path",
                "sequential_vs_parallel_posture": "sequential_only_taskflow_authority",
                "pause_boundary_gate": "allowed_if_authoritative_taskflow_selection_exists",
                "ambiguity_reason": serde_json::Value::Null,
                "stale_missing_task_run_graph_status": {
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": status.active_node,
                    "status": status.status,
                    "lifecycle_stage": status.lifecycle_stage,
                },
                "next_actions": []
            });
        }
    }
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
            if terminal_completed_without_next_unit_is_idle && !delegated_cycle_open {
                return serde_json::json!({
                    "status": "idle",
                    "continuation_allowed": false,
                    "continuation_required_now": false,
                    "active_bounded_unit": serde_json::Value::Null,
                    "binding_source": serde_json::Value::Null,
                    "why_this_unit": format!(
                        "Latest run `{}` is blocked, but no active TaskFlow work is present.",
                        status.run_id
                    ),
                    "primary_path": "idle_project_ready",
                    "sequential_vs_parallel_posture": "not_applicable_no_active_work",
                    "pause_boundary_gate": "allowed_no_active_work",
                    "ambiguity_reason": serde_json::Value::Null,
                    "stale_blocked_run_graph_status": {
                        "task_id": status.task_id,
                        "run_id": status.run_id,
                        "active_node": status.active_node,
                        "status": status.status,
                        "lifecycle_stage": status.lifecycle_stage,
                    },
                    "next_actions": []
                });
            }

            if let Some(binding) = explicit_binding {
                if active_exception_takeover_binding_matches_status(
                    binding,
                    status,
                    latest_run_graph_dispatch_receipt,
                ) && terminal_consume_continue_run_id != Some(status.run_id.as_str())
                {
                    return active_exception_takeover_binding_summary_json(
                        binding,
                        status,
                        continuation_required_now,
                        pause_boundary_gate,
                    );
                }
                if explicit_binding_is_admissible_for_status(binding, status)
                    && active_exception_takeover_evidence_matches_status(
                        status,
                        latest_run_graph_dispatch_receipt,
                        None,
                    )
                {
                    return binding_summary_json(
                        binding,
                        continuation_required_now,
                        pause_boundary_gate,
                        continuation_next_actions.clone(),
                    );
                }
                if dispatch_prelaunch_binding_matches_blocked_status(binding, status) {
                    return binding_summary_json(
                        binding,
                        continuation_required_now,
                        pause_boundary_gate,
                        continuation_next_actions.clone(),
                    );
                }
            }
            if active_exception_takeover_evidence_matches_status(
                status,
                latest_run_graph_dispatch_receipt,
                terminal_consume_continue_run_id,
            ) {
                return active_exception_takeover_status_summary_json(
                    status,
                    continuation_required_now,
                    pause_boundary_gate,
                );
            }

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
                "next_actions": ({
                    let mut next_actions = vec![
                        "Do not continue normal delivery while the latest run-graph status is blocked."
                            .to_string(),
                    ];
                    next_actions.extend(crate::status_surface_signals::blocked_run_graph_status_next_actions(
                        Some(status.run_id.as_str()),
                        Some(status.task_id.as_str()),
                        latest_run_graph_task_closed,
                    ));
                    next_actions
                })
            });
        }

        if let Some(binding) = explicit_binding {
            if explicit_binding_is_admissible_for_status(binding, status) {
                return binding_summary_json(
                    binding,
                    continuation_required_now,
                    pause_boundary_gate,
                    continuation_next_actions.clone(),
                );
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

        if let Some(receipt) = latest_run_graph_dispatch_receipt {
            let downstream_target = receipt
                .downstream_dispatch_target
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let downstream_status_ready = matches!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready") | Some("executed")
            );
            if receipt.run_id == status.run_id
                && receipt.downstream_dispatch_ready
                && downstream_status_ready
            {
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

        if terminal_completed_without_next_unit_is_idle && terminal_completed_without_next_unit {
            return serde_json::json!({
                "status": "idle",
                "continuation_allowed": false,
                "continuation_required_now": false,
                "active_bounded_unit": serde_json::Value::Null,
                "binding_source": serde_json::Value::Null,
                "why_this_unit": format!(
                    "Latest run `{}` is closure_complete and no active TaskFlow work is present.",
                    status.run_id
                ),
                "primary_path": "idle_project_ready",
                "sequential_vs_parallel_posture": "not_applicable_no_active_work",
                "pause_boundary_gate": "allowed_no_active_work",
                "ambiguity_reason": serde_json::Value::Null,
                "next_actions": []
            });
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
                    crate::status_surface_signals::terminal_next_action_requires_authoritative_run_state(
                        Some(status.run_id.as_str()),
                    )
                ]
        });
    }

    if let Some(binding) = explicit_binding {
        if explicit_task_binding_is_admissible_without_status(binding) {
            return binding_summary_json(
                binding,
                continuation_required_now,
                pause_boundary_gate,
                continuation_next_actions,
            );
        }
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
            crate::status_surface_signals::continuation_binding_ambiguous_next_action()
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
    let orthogonal = binding_source.starts_with("latest_run_graph")
        && bound_to_run_graph_task
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
        object.insert(
            "continuation_required_now".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "pause_boundary_gate".to_string(),
            serde_json::Value::String("forbidden_while_ambiguous".to_string()),
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
        object.insert(
            "next_actions".to_string(),
            serde_json::json!([
                "Do not assume the latest run-graph binding is the active bounded unit while TaskFlow has different in-progress task candidates.",
                crate::status_surface_signals::continuation_binding_ambiguous_next_action()
            ]),
        );
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::{
        add_taskflow_active_work_truth, build_continuation_binding_summary,
        build_continuation_binding_summary_with_idle_policy, taskflow_active_candidates_from_tasks,
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

    fn exception_takeover_dispatch(
        run_id: &str,
    ) -> crate::state_store::RunGraphDispatchReceiptSummary {
        crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some(
                "stream4-recovery-status-actionable-command-fix".to_string(),
            ),
            exception_path_receipt_id: Some(
                "stream4-recovery-status-actionable-command-fix".to_string(),
            ),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("exception takeover active".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_terminal_write_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-24T18:50:54Z".to_string(),
        }
    }

    fn exception_takeover_binding(
        run_id: &str,
        binding_source: &str,
    ) -> crate::state_store::RunGraphContinuationBinding {
        crate::state_store::RunGraphContinuationBinding {
            run_id: run_id.to_string(),
            task_id: run_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": run_id,
                "run_id": run_id,
                "active_node": "analysis"
            }),
            binding_source: binding_source.to_string(),
            why_this_unit:
                "Explicit continuation binding records the active exception-takeover unit."
                    .to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("audit-p1-state-store-init-lock-timeout-proof-blocker".to_string()),
            recorded_at: "2026-04-26T07:52:53Z".to_string(),
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

        let summary = build_continuation_binding_summary(
            None,
            Some(&status),
            Some(&recovery),
            None,
            None,
            false,
        );

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

        let summary =
            build_continuation_binding_summary(None, Some(&status), None, None, None, false);

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
    fn blocked_dispatch_prelaunch_binding_still_surfaces_active_unit() {
        let task_id = "universal-surfaces-epic-2-wizard-settings-container";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(task_id, "analysis", "analysis");
        status.task_id = task_id.to_string();
        status.run_id = task_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: task_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "task_id": task_id,
                "run_id": task_id,
                "task_status": "open",
                "issue_type": "task",
            }),
            binding_source: "dispatch_prelaunch_blocked".to_string(),
            why_this_unit: "Explicit continuation binding records task `universal-surfaces-epic-2-wizard-settings-container`.".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
            request_text: Some(task_id.to_string()),
            recorded_at: "2026-05-21T09:08:04Z".to_string(),
        };

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            None,
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(summary["active_bounded_unit"]["task_id"], task_id);
        assert_eq!(summary["binding_source"], "dispatch_prelaunch_blocked");
        assert_eq!(
            summary["why_this_unit"],
            "Explicit continuation binding records task `universal-surfaces-epic-2-wizard-settings-container`."
        );
        assert_eq!(
            summary["sequential_vs_parallel_posture"],
            "sequential_only_open_cycle"
        );
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
    }

    #[test]
    fn blocked_latest_run_graph_status_is_idle_when_taskflow_has_no_active_work() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-bounded-slice-material-owned-app-chrome",
            "runtime-bounded-slice-material-owned-app-chrome",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let summary = build_continuation_binding_summary_with_idle_policy(
            None,
            Some(&status),
            None,
            None,
            None,
            false,
            true,
            false,
        );

        assert_eq!(summary["status"], "idle");
        assert_eq!(summary["continuation_allowed"], false);
        assert_eq!(summary["active_bounded_unit"], serde_json::Value::Null);
        assert_eq!(summary["primary_path"], "idle_project_ready");
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
        assert_eq!(
            summary["stale_blocked_run_graph_status"]["run_id"],
            "runtime-bounded-slice-material-owned-app-chrome"
        );
    }

    #[test]
    fn blocked_latest_run_graph_status_accepts_active_exception_takeover_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let binding = exception_takeover_binding(
            "runtime-audit-state-store-init-lock-timeout",
            crate::taskflow_continuation::CONSUME_CONTINUE_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE,
        );
        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            Some(&dispatch),
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(
            summary["binding_source"],
            "consume_continue_after_downstream_chain"
        );
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "runtime-audit-state-store-init-lock-timeout"
        );
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
        assert_eq!(summary["continuation_resumable"], false);
        assert_eq!(summary["resume_blocker"], "next_action_target_missing");
        assert!(summary["next_actions"].as_array().is_some_and(|actions| {
            actions.iter().any(|action| action
                .as_str()
                .is_some_and(|value| value.contains("vida taskflow recovery status")))
                && actions.iter().any(|action| action
                    .as_str()
                    .is_some_and(|value| value.contains("vida taskflow continuation bind")))
                && actions.iter().all(|action| action
                    .as_str()
                    .is_some_and(|value| !value.starts_with("Continue the active exception-backed bounded unit with `vida taskflow consume continue")))
        }));
    }

    #[test]
    fn blocked_latest_run_graph_status_accepts_explicit_same_run_binding_with_exception_takeover_receipt(
    ) {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "coach",
        );
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            task_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "taskflow-case-18-rollout-regression-gate",
                "run_id": "taskflow-case-18-rollout-regression-gate",
                "active_node": "coach"
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "Explicit continuation binding records task `taskflow-case-18-rollout-regression-gate` at node `coach` as the active bounded unit.".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
            request_text: Some("CASE-18 rollout regression gate".to_string()),
            recorded_at: "2026-05-21T11:19:38Z".to_string(),
        };
        let dispatch = exception_takeover_dispatch("taskflow-case-18-rollout-regression-gate");

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            Some(&dispatch),
            Some("taskflow-case-18-rollout-regression-gate"),
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(summary["binding_source"], "explicit_continuation_bind");
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "taskflow-case-18-rollout-regression-gate"
        );
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
    }

    #[test]
    fn blocked_latest_run_graph_status_for_closed_task_is_retire_actionable() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "closed-feature-task",
            "closed-feature-task",
            "analysis",
        );
        status.run_id = "run-blocked".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let summary = build_continuation_binding_summary_with_idle_policy(
            None,
            Some(&status),
            None,
            None,
            None,
            false,
            false,
            true,
        );

        assert_eq!(summary["status"], "ambiguous");
        assert!(summary["next_actions"]
            .as_array()
            .is_some_and(
                |rows| rows.iter().any(|row| row.as_str().is_some_and(|value| {
                    value.contains("closed-feature-task")
                        && value.contains(
                            "vida lane retire run-blocked --receipt-id <concrete-receipt-id> --reason <reason> --json",
                        )
                        && value.contains("run-blocked")
                }))
            ));
        assert!(summary["next_actions"]
            .as_array()
            .is_some_and(
                |rows| rows.iter().any(|row| row.as_str().is_some_and(|value| {
                    value.contains("vida taskflow recovery status run-blocked --json")
                }))
            ));
    }

    #[test]
    fn blocked_latest_run_graph_status_accepts_direct_consume_exception_takeover_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-cross-platform-vida-install-init-runtime",
            "runtime-cross-platform-vida-install-init-runtime",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let binding = exception_takeover_binding(
            "runtime-cross-platform-vida-install-init-runtime",
            crate::taskflow_continuation::CONSUME_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE,
        );
        let dispatch =
            exception_takeover_dispatch("runtime-cross-platform-vida-install-init-runtime");

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            Some(&dispatch),
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(
            summary["binding_source"],
            crate::taskflow_continuation::CONSUME_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE
        );
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "runtime-cross-platform-vida-install-init-runtime"
        );
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
    }

    #[test]
    fn blocked_latest_run_graph_status_accepts_exception_takeover_despite_stale_explicit_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();

        let stale_binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "feature-reconcile-qwen-cli-carrier-drift-across-config-code".to_string(),
            task_id: "feature-reconcile-qwen-cli-carrier-drift-across-config-code".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "downstream_dispatch_target",
                "task_id": "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
                "run_id": "feature-reconcile-qwen-cli-carrier-drift-across-config-code",
                "dispatch_target": "closure"
            }),
            binding_source: "task_close_reconcile".to_string(),
            why_this_unit: "stale close reconcile binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: "2026-04-22T08:44:36Z".to_string(),
        };
        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");

        let summary = build_continuation_binding_summary(
            Some(&stale_binding),
            Some(&status),
            None,
            Some(&dispatch),
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(
            summary["binding_source"],
            "latest_run_graph_exception_takeover_dispatch"
        );
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "runtime-audit-state-store-init-lock-timeout"
        );
        assert_eq!(summary["active_exception_takeover"], true);
        assert_eq!(summary["continuation_resumable"], false);
        assert_eq!(summary["resume_blocker"], "next_action_target_missing");
        assert!(summary["next_actions"].as_array().is_some_and(|actions| {
            actions.iter().any(|action| {
                action
                    .as_str()
                    .is_some_and(|value| value.contains("vida taskflow recovery status"))
            })
        }));
    }

    #[test]
    fn exception_takeover_summary_keeps_recovery_ready_false_when_dispatch_target_exists() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = false;

        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");
        let summary = build_continuation_binding_summary(
            None,
            Some(&status),
            None,
            Some(&dispatch),
            None,
            false,
        );

        assert_eq!(summary["continuation_resumable"], false);
        assert_eq!(summary["resume_blocker"], "recovery_ready_false");
        assert!(summary["next_actions"].as_array().is_some_and(|actions| {
            actions.iter().any(|action| {
                action
                    .as_str()
                    .is_some_and(|value| value.contains("vida lane show"))
            })
        }));
    }

    #[test]
    fn blocked_latest_run_graph_status_accepts_superseded_exception_even_when_lane_status_is_stale_recorded(
    ) {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-stale-lane-status",
            "run-stale-lane-status",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        let mut dispatch = exception_takeover_dispatch("run-stale-lane-status");
        dispatch.lane_status = "lane_exception_recorded".to_string();
        dispatch.supersedes_receipt_id = Some("takeover-receipt".to_string());
        dispatch.exception_path_receipt_id = Some("takeover-receipt".to_string());

        let summary = build_continuation_binding_summary(
            None,
            Some(&status),
            None,
            Some(&dispatch),
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["active_exception_takeover"], true);
        assert_eq!(
            summary["binding_source"],
            "latest_run_graph_exception_takeover_dispatch"
        );
        assert_eq!(
            summary["active_bounded_unit"]["run_id"],
            "run-stale-lane-status"
        );
    }

    #[test]
    fn blocked_latest_run_graph_status_retires_exception_takeover_after_terminal_continue() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-audit-state-store-init-lock-timeout",
            "runtime-audit-state-store-init-lock-timeout",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        let dispatch = exception_takeover_dispatch("runtime-audit-state-store-init-lock-timeout");

        let summary = build_continuation_binding_summary(
            None,
            Some(&status),
            None,
            Some(&dispatch),
            Some("runtime-audit-state-store-init-lock-timeout"),
            false,
        );

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(
            summary["ambiguity_reason"],
            "latest_run_graph_status_blocked"
        );
        assert_ne!(
            summary["binding_source"],
            "latest_run_graph_exception_takeover_dispatch"
        );
        assert!(!summary["next_actions"]
            .as_array()
            .expect("next actions should be present")
            .iter()
            .any(|action| action.as_str().is_some_and(|value| value.contains(
                "consume continue --run-id runtime-audit-state-store-init-lock-timeout"
            ))));
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

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            None,
            None,
            false,
        );

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

        let summary =
            build_continuation_binding_summary(None, Some(&status), None, None, None, false);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(
            summary["ambiguity_reason"],
            "completed_without_explicit_next_bounded_unit"
        );
    }

    #[test]
    fn completed_closure_binds_receipt_backed_downstream_target() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.next_node = None;

        let mut receipt = exception_takeover_dispatch("task-1");
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());

        let summary = build_continuation_binding_summary_with_idle_policy(
            None,
            Some(&status),
            None,
            Some(&receipt),
            None,
            false,
            false,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(
            summary["binding_source"],
            "latest_run_graph_dispatch_receipt"
        );
        assert_eq!(
            summary["active_bounded_unit"]["kind"],
            "downstream_dispatch_target"
        );
        assert_eq!(summary["active_bounded_unit"]["dispatch_target"], "coach");
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

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            None,
            None,
            false,
        );

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

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            None,
            None,
            false,
        );

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

        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            None,
            None,
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["binding_source"], "explicit_continuation_bind_task");
        assert_eq!(summary["active_bounded_unit"]["task_id"], "task-upstream");
        assert_eq!(
            summary["sequential_vs_parallel_posture"],
            "sequential_only_explicit_task_bound"
        );
    }

    #[test]
    fn explicit_task_graph_binding_without_latest_run_graph_status_is_bound() {
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "previous-run".to_string(),
            task_id: "multi-orch-session-20-scoped-queries".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "task_id": "multi-orch-session-20-scoped-queries",
                "run_id": "previous-run",
                "task_status": "in_progress",
                "issue_type": "task"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "operator explicitly bound the next ready TaskFlow task".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("continue scoped queries".to_string()),
            recorded_at: "2026-05-17T10:00:00Z".to_string(),
        };

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "coach",
        );
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        let dispatch = exception_takeover_dispatch("taskflow-case-18-rollout-regression-gate");
        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            Some(&dispatch),
            Some("taskflow-case-18-rollout-regression-gate"),
            false,
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
        assert_eq!(summary["binding_source"], "explicit_continuation_bind_task");
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "multi-orch-session-20-scoped-queries"
        );

        let summary = add_taskflow_active_work_truth(
            summary,
            taskflow_active_candidates_from_tasks(&[task_record(
                "multi-orch-session-20-scoped-queries",
                "in_progress",
            )]),
        );

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["binding_scope"], "taskflow_explicit");
        assert_eq!(summary["orthogonal_to_taskflow_active_work"], false);
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

        let mut recovery = crate::taskflow_run_graph::default_run_graph_recovery_summary(
            &status.task_id,
            &status.run_id,
        );
        recovery.delegation_gate.delegated_cycle_open = true;

        let summary = build_continuation_binding_summary(
            None,
            Some(&status),
            Some(&recovery),
            None,
            None,
            false,
        );
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
        assert_eq!(summary["continuation_required_now"], false);
        assert_eq!(summary["pause_boundary_gate"], "forbidden_while_ambiguous");
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
        assert!(summary["next_actions"].as_array().is_some_and(|rows| {
            rows.iter().all(|row| {
                row.as_str()
                    .is_some_and(|value| !value.contains("vida taskflow consume continue --run-id"))
            })
        }));
    }

    #[test]
    fn taskflow_active_work_truth_preserves_explicit_run_graph_binding() {
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            task_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "taskflow-case-18-rollout-regression-gate",
                "run_id": "taskflow-case-18-rollout-regression-gate",
                "active_node": "coach"
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "Explicit continuation binding records case 18.".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
            request_text: Some("CASE-18 rollout regression gate".to_string()),
            recorded_at: "2026-05-21T11:25:10Z".to_string(),
        };

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "coach",
        );
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        let dispatch = exception_takeover_dispatch("taskflow-case-18-rollout-regression-gate");
        let summary = build_continuation_binding_summary(
            Some(&binding),
            Some(&status),
            None,
            Some(&dispatch),
            Some("taskflow-case-18-rollout-regression-gate"),
            false,
        );
        let taskflow_candidates = taskflow_active_candidates_from_tasks(&[
            task_record("runtime-normal-operation-recovery-epic", "in_progress"),
            task_record("taskflow-testing-defects-epic", "in_progress"),
        ]);
        let summary = add_taskflow_active_work_truth(summary, taskflow_candidates);

        assert_eq!(summary["status"], "bound");
        assert_eq!(summary["continuation_allowed"], true);
        assert_eq!(summary["binding_scope"], "run_graph_explicit");
        assert_eq!(summary["orthogonal_to_taskflow_active_work"], false);
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            "taskflow-case-18-rollout-regression-gate"
        );
        assert_eq!(summary["ambiguity_reason"], serde_json::Value::Null);
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

        let summary =
            build_continuation_binding_summary(None, Some(&status), None, None, None, false);
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
