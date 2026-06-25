use taskflow_core::run_graph::model::{
    DefaultRunGraphStatusFields, DispatchReceiptSnapshot, RunGraphStatusSnapshot,
    RunGraphTransitionDecision, RunGraphTransitionKind, TaskClosureSnapshot,
    decide_run_graph_transition,
};

pub const MODULE: &str = "run_graph_transition";

pub const RUN_GRAPH_NEXT_ACTION_INSPECT_LANE: &str = "inspect lane recovery evidence";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunGraphDispatchTargetFormat {
    Lane,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyRunGraphTransitionInput {
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub route_task_class: String,
    pub selected_backend: String,
    pub lane_id: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub checkpoint_kind: String,
    pub target_format: RunGraphDispatchTargetFormat,
    pub recovery_ready: bool,
}

#[must_use]
pub fn ready_run_graph_transition(
    input: ReadyRunGraphTransitionInput,
) -> DefaultRunGraphStatusFields {
    let (handoff_state, resume_target) =
        run_graph_handoff(input.next_node.as_deref(), input.target_format);

    DefaultRunGraphStatusFields {
        run_id: input.run_id,
        task_id: input.task_id,
        task_class: input.task_class,
        active_node: input.active_node,
        next_node: input.next_node,
        status: "ready".to_string(),
        route_task_class: input.route_task_class,
        selected_backend: input.selected_backend,
        lane_id: input.lane_id,
        lifecycle_stage: input.lifecycle_stage,
        policy_gate: input.policy_gate,
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: input.checkpoint_kind,
        resume_target,
        recovery_ready: input.recovery_ready,
    }
}

#[must_use]
pub fn run_graph_handoff(
    next_node: Option<&str>,
    target_format: RunGraphDispatchTargetFormat,
) -> (String, String) {
    let handoff_state = next_node
        .map(|next| format!("awaiting_{next}"))
        .unwrap_or_else(|| "none".to_string());
    let resume_target = next_node
        .map(|next| match target_format {
            RunGraphDispatchTargetFormat::Lane => format!("dispatch.{next}_lane"),
            RunGraphDispatchTargetFormat::Direct => format!("dispatch.{next}"),
        })
        .unwrap_or_else(|| "none".to_string());
    (handoff_state, resume_target)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphAuthorityInput {
    pub status: RunGraphStatusSnapshot,
    pub receipt: Option<DispatchReceiptSnapshot>,
    pub closure: Option<TaskClosureSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphAuthorityDecision {
    pub decision: RunGraphTransitionDecision,
    pub next_actions: Vec<String>,
}

#[must_use]
pub fn admit_run_graph_transition(input: RunGraphAuthorityInput) -> RunGraphAuthorityDecision {
    let decision = decide_run_graph_transition(
        &input.status,
        input.receipt.as_ref(),
        input.closure.as_ref(),
    );
    let next_actions = next_actions_for_transition(&decision);

    RunGraphAuthorityDecision {
        decision,
        next_actions,
    }
}

fn next_actions_for_transition(decision: &RunGraphTransitionDecision) -> Vec<String> {
    if decision.kind == RunGraphTransitionKind::BlockedLane {
        vec![RUN_GRAPH_NEXT_ACTION_INSPECT_LANE.to_string()]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod run_graph_tests {
    use super::{
        RUN_GRAPH_NEXT_ACTION_INSPECT_LANE, ReadyRunGraphTransitionInput, RunGraphAuthorityInput,
        RunGraphDispatchTargetFormat, admit_run_graph_transition, ready_run_graph_transition,
        run_graph_handoff,
    };
    use taskflow_core::run_graph::model::{
        DispatchReceiptSnapshot, RunGraphStatusSnapshot, RunGraphTransitionKind,
        TaskClosureSnapshot,
    };

    #[test]
    fn run_graph_authority_admits_terminal_closure() {
        let decision = admit_run_graph_transition(RunGraphAuthorityInput {
            status: status(),
            receipt: None,
            closure: Some(TaskClosureSnapshot {
                task_id: "task-1".to_string(),
                status: "closed".to_string(),
                terminally_closed: true,
            }),
        });

        assert_eq!(
            decision.decision.kind,
            RunGraphTransitionKind::TerminalClosure
        );
        assert!(decision.decision.admitted);
        assert!(decision.next_actions.is_empty());
    }

    #[test]
    fn run_graph_authority_rejects_blocked_downstream_ready_handoff() {
        let mut status = status();
        status.active_node = "developer".to_string();
        status.next_node = Some("tester".to_string());
        status.resume_target = "dispatch.tester".to_string();

        let decision = admit_run_graph_transition(RunGraphAuthorityInput {
            status,
            receipt: Some(DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_failed".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_target: Some("tester".to_string()),
                downstream_dispatch_blockers: Vec::new(),
            }),
            closure: None,
        });

        assert_eq!(decision.decision.kind, RunGraphTransitionKind::BlockedLane);
        assert!(!decision.decision.admitted);
        assert_eq!(
            decision.next_actions,
            vec![RUN_GRAPH_NEXT_ACTION_INSPECT_LANE]
        );
    }

    #[test]
    fn run_graph_authority_marks_stalled_lane_actionable() {
        let decision = admit_run_graph_transition(RunGraphAuthorityInput {
            status: status(),
            receipt: Some(DispatchReceiptSnapshot {
                dispatch_target: "developer".to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: Some("lane_blocked".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_target: None,
                downstream_dispatch_blockers: Vec::new(),
            }),
            closure: None,
        });

        assert_eq!(decision.decision.kind, RunGraphTransitionKind::BlockedLane);
        assert!(!decision.decision.admitted);
        assert_eq!(
            decision.next_actions,
            vec![RUN_GRAPH_NEXT_ACTION_INSPECT_LANE]
        );
    }

    #[test]
    fn ready_run_graph_transition_builds_lane_handoff() {
        let transition = ready_run_graph_transition(ReadyRunGraphTransitionInput {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("developer".to_string()),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "developer_lane".to_string(),
            lifecycle_stage: "developer_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            target_format: RunGraphDispatchTargetFormat::Lane,
            recovery_ready: true,
        });

        assert_eq!(transition.status, "ready");
        assert_eq!(transition.handoff_state, "awaiting_developer");
        assert_eq!(transition.resume_target, "dispatch.developer_lane");
        assert_eq!(transition.context_state, "sealed");
        assert!(transition.recovery_ready);
    }

    #[test]
    fn run_graph_handoff_supports_direct_conversation_targets() {
        assert_eq!(
            run_graph_handoff(Some("spec-pack"), RunGraphDispatchTargetFormat::Direct),
            (
                "awaiting_spec-pack".to_string(),
                "dispatch.spec-pack".to_string()
            )
        );
        assert_eq!(
            run_graph_handoff(None, RunGraphDispatchTargetFormat::Lane),
            ("none".to_string(), "none".to_string())
        );
    }

    fn status() -> RunGraphStatusSnapshot {
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
