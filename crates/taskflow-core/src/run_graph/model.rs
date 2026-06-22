//! Run-graph model defaults for TaskFlow runtime decomposition.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultRunGraphStatusFields {
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub route_task_class: String,
    pub selected_backend: String,
    pub lane_id: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

#[must_use]
pub fn default_run_graph_status_fields(
    task_id: impl Into<String>,
    task_class: impl Into<String>,
    route_task_class: impl Into<String>,
) -> DefaultRunGraphStatusFields {
    let task_id = task_id.into();
    let task_class = task_class.into();
    DefaultRunGraphStatusFields {
        run_id: task_id.clone(),
        task_id,
        task_class: task_class.clone(),
        active_node: task_class,
        next_node: None,
        status: "pending".to_string(),
        route_task_class: route_task_class.into(),
        selected_backend: "unknown".to_string(),
        lane_id: "unassigned".to_string(),
        lifecycle_stage: "initialized".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "none".to_string(),
        context_state: "open".to_string(),
        checkpoint_kind: "none".to_string(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunGraphStatusSnapshot {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub lifecycle_stage: String,
    pub handoff_state: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchReceiptSnapshot {
    pub dispatch_target: String,
    pub dispatch_status: String,
    pub lane_status: Option<String>,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskClosureSnapshot {
    pub task_id: String,
    pub status: String,
    pub terminally_closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunGraphTransitionKind {
    TerminalClosure,
    ExceptionTakeover,
    CompletedLane,
    BlockedLane,
    DownstreamReadyHandoff,
    NoTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunGraphTransitionDecision {
    pub kind: RunGraphTransitionKind,
    pub admitted: bool,
    pub active_node: String,
    pub next_node: Option<String>,
    pub resume_target: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
}

#[must_use]
pub fn decide_run_graph_transition(
    status: &RunGraphStatusSnapshot,
    receipt: Option<&DispatchReceiptSnapshot>,
    closure: Option<&TaskClosureSnapshot>,
) -> RunGraphTransitionDecision {
    if closure
        .filter(|closure| closure.terminally_closed && closure.task_id == status.task_id)
        .is_some()
    {
        return RunGraphTransitionDecision {
            kind: RunGraphTransitionKind::TerminalClosure,
            admitted: true,
            active_node: status.active_node.clone(),
            next_node: None,
            resume_target: "none".to_string(),
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }

    if let Some(receipt) = receipt {
        if receipt_has_active_exception_takeover(receipt) {
            if !receipt_target_matches_current_graph(status, receipt) {
                return untrusted_receipt_decision(status);
            }

            return RunGraphTransitionDecision {
                kind: RunGraphTransitionKind::ExceptionTakeover,
                admitted: true,
                active_node: receipt.dispatch_target.clone(),
                next_node: Some(receipt.dispatch_target.clone()),
                resume_target: format!("dispatch.{}", receipt.dispatch_target),
                blocker_codes: Vec::new(),
                next_actions: Vec::new(),
            };
        }

        if receipt.downstream_dispatch_ready
            && receipt
                .downstream_dispatch_target
                .as_deref()
                .is_some_and(|target| !target.trim().is_empty())
            && receipt.downstream_dispatch_blockers.is_empty()
        {
            if !is_executed_receipt(receipt)
                || !receipt_target_matches_current_graph(status, receipt)
                || !downstream_target_matches_expected_next(status, receipt)
            {
                return untrusted_receipt_decision(status);
            }

            let target = receipt.downstream_dispatch_target.clone();
            return RunGraphTransitionDecision {
                kind: RunGraphTransitionKind::DownstreamReadyHandoff,
                admitted: true,
                active_node: status.active_node.clone(),
                next_node: target.clone(),
                resume_target: target
                    .as_deref()
                    .map(|target| format!("dispatch.{target}"))
                    .unwrap_or_else(|| status.resume_target.clone()),
                blocker_codes: Vec::new(),
                next_actions: Vec::new(),
            };
        }

        if receipt.lane_status.as_deref() == Some("lane_completed") {
            if !is_executed_receipt(receipt)
                || !receipt_target_matches_current_graph(status, receipt)
            {
                return untrusted_receipt_decision(status);
            }

            return RunGraphTransitionDecision {
                kind: RunGraphTransitionKind::CompletedLane,
                admitted: true,
                active_node: receipt.dispatch_target.clone(),
                next_node: status.next_node.clone(),
                resume_target: status.resume_target.clone(),
                blocker_codes: Vec::new(),
                next_actions: Vec::new(),
            };
        }

        if is_blocked_lane(receipt.lane_status.as_deref()) {
            return RunGraphTransitionDecision {
                kind: RunGraphTransitionKind::BlockedLane,
                admitted: false,
                active_node: status.active_node.clone(),
                next_node: status.next_node.clone(),
                resume_target: status.resume_target.clone(),
                blocker_codes: vec!["lane_blocked".to_string()],
                next_actions: vec!["inspect lane recovery evidence".to_string()],
            };
        }
    }

    RunGraphTransitionDecision {
        kind: RunGraphTransitionKind::NoTransition,
        admitted: true,
        active_node: status.active_node.clone(),
        next_node: status.next_node.clone(),
        resume_target: status.resume_target.clone(),
        blocker_codes: Vec::new(),
        next_actions: Vec::new(),
    }
}

fn is_executed_receipt(receipt: &DispatchReceiptSnapshot) -> bool {
    receipt.dispatch_status.trim() == "executed"
}

fn receipt_target_matches_current_graph(
    status: &RunGraphStatusSnapshot,
    receipt: &DispatchReceiptSnapshot,
) -> bool {
    let dispatch_target = receipt.dispatch_target.trim();
    !dispatch_target.is_empty()
        && (dispatch_target == status.active_node
            || status
                .next_node
                .as_deref()
                .is_some_and(|next_node| dispatch_target == next_node))
}

fn downstream_target_matches_expected_next(
    status: &RunGraphStatusSnapshot,
    receipt: &DispatchReceiptSnapshot,
) -> bool {
    let Some(target) = receipt.downstream_dispatch_target.as_deref() else {
        return false;
    };
    let target = target.trim();
    !target.is_empty()
        && status
            .next_node
            .as_deref()
            .is_some_and(|next_node| target == next_node)
}

fn untrusted_receipt_decision(status: &RunGraphStatusSnapshot) -> RunGraphTransitionDecision {
    RunGraphTransitionDecision {
        kind: RunGraphTransitionKind::NoTransition,
        admitted: false,
        active_node: status.active_node.clone(),
        next_node: status.next_node.clone(),
        resume_target: status.resume_target.clone(),
        blocker_codes: vec!["untrusted_dispatch_receipt".to_string()],
        next_actions: vec![
            "verify dispatch receipt binding before advancing run graph".to_string(),
        ],
    }
}

fn receipt_has_active_exception_takeover(receipt: &DispatchReceiptSnapshot) -> bool {
    receipt.lane_status.as_deref() == Some("lane_exception_takeover")
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
}

fn is_blocked_lane(lane_status: Option<&str>) -> bool {
    matches!(
        lane_status,
        Some("lane_blocked" | "lane_failed" | "lane_exception_recorded")
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DispatchReceiptSnapshot, RunGraphStatusSnapshot, RunGraphTransitionKind,
        TaskClosureSnapshot, decide_run_graph_transition, default_run_graph_status_fields,
    };

    #[test]
    fn default_run_graph_status_fields_keep_legacy_defaults() {
        let fields = default_run_graph_status_fields("task-1", "developer", "implementation");

        assert_eq!(fields.run_id, "task-1");
        assert_eq!(fields.task_id, "task-1");
        assert_eq!(fields.task_class, "developer");
        assert_eq!(fields.active_node, "developer");
        assert_eq!(fields.route_task_class, "implementation");
        assert_eq!(fields.status, "pending");
        assert_eq!(fields.selected_backend, "unknown");
        assert_eq!(fields.lane_id, "unassigned");
        assert_eq!(fields.lifecycle_stage, "initialized");
        assert_eq!(fields.policy_gate, "not_required");
        assert_eq!(fields.handoff_state, "none");
        assert_eq!(fields.context_state, "open");
        assert_eq!(fields.checkpoint_kind, "none");
        assert_eq!(fields.resume_target, "none");
        assert!(!fields.recovery_ready);
        assert!(fields.next_node.is_none());
    }

    #[test]
    fn run_graph_transition_prefers_terminal_task_closure() {
        let decision = decide_run_graph_transition(
            &status_snapshot(),
            None,
            Some(&TaskClosureSnapshot {
                task_id: "task-1".to_string(),
                status: "closed".to_string(),
                terminally_closed: true,
            }),
        );

        assert_eq!(decision.kind, RunGraphTransitionKind::TerminalClosure);
        assert!(decision.admitted);
        assert_eq!(decision.resume_target, "none");
        assert!(decision.next_node.is_none());
    }

    #[test]
    fn run_graph_transition_activates_exception_takeover_receipt() {
        let decision = decide_run_graph_transition(
            &status_snapshot(),
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: Some("lane_exception_takeover".to_string()),
                supersedes_receipt_id: Some("receipt-1".to_string()),
                exception_path_receipt_id: Some("receipt-1".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_target: None,
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );

        assert_eq!(decision.kind, RunGraphTransitionKind::ExceptionTakeover);
        assert!(decision.admitted);
        assert_eq!(decision.active_node, "developer");
        assert_eq!(decision.resume_target, "dispatch.developer");
    }

    #[test]
    fn run_graph_transition_classifies_completed_and_blocked_lanes() {
        let completed = decide_run_graph_transition(
            &status_snapshot(),
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_completed".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_target: None,
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );
        assert_eq!(completed.kind, RunGraphTransitionKind::CompletedLane);
        assert!(completed.admitted);

        let blocked = decide_run_graph_transition(
            &status_snapshot(),
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: Some("lane_blocked".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_target: None,
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );
        assert_eq!(blocked.kind, RunGraphTransitionKind::BlockedLane);
        assert!(!blocked.admitted);
        assert_eq!(blocked.blocker_codes, vec!["lane_blocked"]);
    }

    #[test]
    fn run_graph_transition_admits_downstream_ready_handoff() {
        let status = active_developer_status_snapshot();
        let decision = decide_run_graph_transition(
            &status,
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_open".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_target: Some("tester".to_string()),
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );

        assert_eq!(
            decision.kind,
            RunGraphTransitionKind::DownstreamReadyHandoff
        );
        assert!(decision.admitted);
        assert_eq!(decision.next_node, Some("tester".to_string()));
        assert_eq!(decision.resume_target, "dispatch.tester");
    }

    #[test]
    fn run_graph_transition_prioritizes_completed_lane_downstream_ready_handoff() {
        let status = active_developer_status_snapshot();
        let decision = decide_run_graph_transition(
            &status,
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_completed".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_target: Some("tester".to_string()),
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );

        assert_eq!(
            decision.kind,
            RunGraphTransitionKind::DownstreamReadyHandoff
        );
        assert!(decision.admitted);
        assert_eq!(decision.next_node, Some("tester".to_string()));
        assert_eq!(decision.resume_target, "dispatch.tester");
    }

    #[test]
    fn run_graph_transition_matrix_preserves_priority_and_blockers() {
        struct Case {
            name: &'static str,
            receipt: Option<DispatchReceiptSnapshot>,
            closure: Option<TaskClosureSnapshot>,
            kind: RunGraphTransitionKind,
            admitted: bool,
            next_node: Option<&'static str>,
            resume_target: &'static str,
            blocker_codes: &'static [&'static str],
        }

        let downstream_status = active_developer_status_snapshot();
        let cases = [
            Case {
                name: "terminal closure wins over receipt evidence",
                receipt: Some(exception_takeover_receipt()),
                closure: Some(TaskClosureSnapshot {
                    task_id: "task-1".to_string(),
                    status: "closed".to_string(),
                    terminally_closed: true,
                }),
                kind: RunGraphTransitionKind::TerminalClosure,
                admitted: true,
                next_node: None,
                resume_target: "none",
                blocker_codes: &[],
            },
            Case {
                name: "exception takeover supersession resumes owner",
                receipt: Some(exception_takeover_receipt()),
                closure: None,
                kind: RunGraphTransitionKind::ExceptionTakeover,
                admitted: true,
                next_node: Some("developer"),
                resume_target: "dispatch.developer",
                blocker_codes: &[],
            },
            Case {
                name: "completed lane keeps current resume target",
                receipt: Some(receipt_with_lane_status("lane_completed")),
                closure: None,
                kind: RunGraphTransitionKind::CompletedLane,
                admitted: true,
                next_node: Some("developer"),
                resume_target: "dispatch.developer",
                blocker_codes: &[],
            },
            Case {
                name: "failed lane fails closed",
                receipt: Some(receipt_with_lane_status("lane_failed")),
                closure: None,
                kind: RunGraphTransitionKind::BlockedLane,
                admitted: false,
                next_node: Some("developer"),
                resume_target: "dispatch.developer",
                blocker_codes: &["lane_blocked"],
            },
            Case {
                name: "downstream ready handoff advances resume target",
                receipt: Some(downstream_receipt(true, Some("tester"), vec![])),
                closure: None,
                kind: RunGraphTransitionKind::DownstreamReadyHandoff,
                admitted: true,
                next_node: Some("tester"),
                resume_target: "dispatch.tester",
                blocker_codes: &[],
            },
            Case {
                name: "downstream blockers suppress handoff",
                receipt: Some(downstream_receipt(
                    true,
                    Some("tester"),
                    vec!["missing_receipt".to_string()],
                )),
                closure: None,
                kind: RunGraphTransitionKind::NoTransition,
                admitted: true,
                next_node: Some("tester"),
                resume_target: "dispatch.tester",
                blocker_codes: &[],
            },
        ];

        for case in cases {
            let status = if case.name.contains("downstream") {
                &downstream_status
            } else {
                &status_snapshot()
            };
            let decision =
                decide_run_graph_transition(status, case.receipt.as_ref(), case.closure.as_ref());

            assert_eq!(decision.kind, case.kind, "{}", case.name);
            assert_eq!(decision.admitted, case.admitted, "{}", case.name);
            assert_eq!(
                decision.next_node.as_deref(),
                case.next_node,
                "{}",
                case.name
            );
            assert_eq!(decision.resume_target, case.resume_target, "{}", case.name);
            assert_eq!(decision.blocker_codes, case.blocker_codes, "{}", case.name);
        }
    }

    #[test]
    fn run_graph_transition_rejects_forged_completed_lane_receipt() {
        let decision = decide_run_graph_transition(
            &status_snapshot(),
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "attacker_controlled_lane".to_string(),
                dispatch_status: "routed_not_executed".to_string(),
                lane_status: Some("lane_completed".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_target: None,
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );

        assert_eq!(decision.kind, RunGraphTransitionKind::NoTransition);
        assert!(!decision.admitted);
        assert_eq!(decision.active_node, "planning");
        assert_eq!(decision.blocker_codes, vec!["untrusted_dispatch_receipt"]);
    }

    #[test]
    fn run_graph_transition_rejects_forged_downstream_handoff_receipt() {
        let decision = decide_run_graph_transition(
            &status_snapshot(),
            Some(&DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_open".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_target: Some("attacker.backend.root".to_string()),
                downstream_dispatch_blockers: Vec::new(),
            }),
            None,
        );

        assert_eq!(decision.kind, RunGraphTransitionKind::NoTransition);
        assert!(!decision.admitted);
        assert_eq!(decision.next_node, Some("developer".to_string()));
        assert_eq!(decision.resume_target, "dispatch.developer");
        assert_eq!(decision.blocker_codes, vec!["untrusted_dispatch_receipt"]);
    }

    fn receipt_with_lane_status(lane_status: &str) -> DispatchReceiptSnapshot {
        DispatchReceiptSnapshot {
            dispatch_target: "developer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: Some(lane_status.to_string()),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: None,
            downstream_dispatch_blockers: Vec::new(),
        }
    }

    fn exception_takeover_receipt() -> DispatchReceiptSnapshot {
        DispatchReceiptSnapshot {
            lane_status: Some("lane_exception_takeover".to_string()),
            supersedes_receipt_id: Some("supersede-1".to_string()),
            exception_path_receipt_id: Some("receipt-1".to_string()),
            ..receipt_with_lane_status("lane_exception_takeover")
        }
    }

    fn downstream_receipt(
        ready: bool,
        target: Option<&str>,
        blockers: Vec<String>,
    ) -> DispatchReceiptSnapshot {
        DispatchReceiptSnapshot {
            lane_status: Some("lane_open".to_string()),
            downstream_dispatch_ready: ready,
            downstream_dispatch_target: target.map(str::to_string),
            downstream_dispatch_blockers: blockers,
            ..receipt_with_lane_status("lane_open")
        }
    }

    fn active_developer_status_snapshot() -> RunGraphStatusSnapshot {
        RunGraphStatusSnapshot {
            active_node: "developer".to_string(),
            next_node: Some("tester".to_string()),
            resume_target: "dispatch.tester".to_string(),
            ..status_snapshot()
        }
    }

    fn status_snapshot() -> RunGraphStatusSnapshot {
        RunGraphStatusSnapshot {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("developer".to_string()),
            status: "ready".to_string(),
            lifecycle_stage: "developer_dispatch_ready".to_string(),
            handoff_state: "awaiting_developer".to_string(),
            resume_target: "dispatch.developer".to_string(),
            recovery_ready: true,
        }
    }
}
