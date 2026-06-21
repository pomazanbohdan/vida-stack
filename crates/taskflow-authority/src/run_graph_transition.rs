use taskflow_core::run_graph::model::{
    DispatchReceiptSnapshot, RunGraphStatusSnapshot, RunGraphTransitionDecision,
    RunGraphTransitionKind, TaskClosureSnapshot, decide_run_graph_transition,
};

pub const MODULE: &str = "run_graph_transition";

pub const RUN_GRAPH_NEXT_ACTION_INSPECT_LANE: &str = "inspect lane recovery evidence";

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
        RUN_GRAPH_NEXT_ACTION_INSPECT_LANE, RunGraphAuthorityInput, admit_run_graph_transition,
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
