pub const MODULE: &str = "stale_guard";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleRunGraphStatus<'a> {
    pub status: &'a str,
    pub lifecycle_stage: &'a str,
    pub next_node: Option<&'a str>,
    pub resume_target: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleRunGraphReceipt<'a> {
    pub dispatch_status: &'a str,
    pub lane_status: &'a str,
    pub downstream_dispatch_status: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskflowActiveCandidate<'a> {
    pub task_id: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleRunTaskState {
    Missing,
    Closed,
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleRunRetireAdmissibility {
    AllowedMissingTask,
    AllowedClosedTask,
    BlockedTerminalRun,
    BlockedTaskNotClosed,
    BlockedMissingTaskReceiptShape,
}

impl StaleRunRetireAdmissibility {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::AllowedMissingTask | Self::AllowedClosedTask)
    }

    pub fn blocker_code(self) -> Option<&'static str> {
        match self {
            Self::AllowedMissingTask | Self::AllowedClosedTask => None,
            Self::BlockedTerminalRun => Some("lane_retire_terminal_run"),
            Self::BlockedTaskNotClosed => Some("lane_retire_task_not_closed"),
            Self::BlockedMissingTaskReceiptShape => {
                Some("lane_retire_missing_task_receipt_not_retireable")
            }
        }
    }
}

pub fn missing_task_stale_blocked_run_can_retire(
    status: &StaleRunGraphStatus<'_>,
    receipt: &StaleRunGraphReceipt<'_>,
) -> bool {
    if run_graph_status_is_terminal_closure(status) {
        return false;
    }

    let blocked_or_running = matches!(receipt.lane_status, "lane_running" | "lane_blocked");
    let prelaunch_packet_ready = receipt.dispatch_status == "executed"
        && receipt.lane_status == "lane_completed"
        && receipt.downstream_dispatch_status == Some("packet_ready");
    (receipt.dispatch_status == "blocked" && blocked_or_running) || prelaunch_packet_ready
}

pub fn stale_run_retire_admissibility(
    status: &StaleRunGraphStatus<'_>,
    receipt: Option<&StaleRunGraphReceipt<'_>>,
    task_state: StaleRunTaskState,
) -> StaleRunRetireAdmissibility {
    if run_graph_status_is_terminal_closure(status) {
        return StaleRunRetireAdmissibility::BlockedTerminalRun;
    }

    match task_state {
        StaleRunTaskState::Closed => StaleRunRetireAdmissibility::AllowedClosedTask,
        StaleRunTaskState::Open => StaleRunRetireAdmissibility::BlockedTaskNotClosed,
        StaleRunTaskState::Missing => match receipt {
            Some(receipt) if missing_task_stale_blocked_run_can_retire(status, receipt) => {
                StaleRunRetireAdmissibility::AllowedMissingTask
            }
            None => StaleRunRetireAdmissibility::AllowedMissingTask,
            Some(_) => StaleRunRetireAdmissibility::BlockedMissingTaskReceiptShape,
        },
    }
}

pub fn latest_run_graph_task_orthogonal_to_taskflow_active_work(
    latest_run_graph_status_task_id: Option<&str>,
    latest_run_graph_receipt_run_id: Option<&str>,
    taskflow_active_candidates: &[TaskflowActiveCandidate<'_>],
) -> bool {
    let [candidate] = taskflow_active_candidates else {
        return false;
    };

    latest_run_graph_status_task_id.is_some_and(|task_id| task_id != candidate.task_id)
        || latest_run_graph_receipt_run_id.is_some_and(|run_id| run_id != candidate.task_id)
}

pub fn latest_run_graph_task_stale_for_write_guard(
    latest_run_graph_task_missing: bool,
    latest_run_graph_task_closed: bool,
    exception_takeover_matches_active_taskflow_work: bool,
    latest_run_graph_task_orthogonal_to_taskflow_active_work: bool,
) -> bool {
    latest_run_graph_task_missing
        || latest_run_graph_task_closed
        || (!exception_takeover_matches_active_taskflow_work
            && latest_run_graph_task_orthogonal_to_taskflow_active_work)
}

fn run_graph_status_is_terminal_closure(status: &StaleRunGraphStatus<'_>) -> bool {
    status.status == "completed"
        && status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && status.resume_target == "none"
}

#[cfg(test)]
mod tests {
    use super::{
        StaleRunGraphReceipt, StaleRunGraphStatus, StaleRunRetireAdmissibility, StaleRunTaskState,
        TaskflowActiveCandidate, latest_run_graph_task_orthogonal_to_taskflow_active_work,
        latest_run_graph_task_stale_for_write_guard, missing_task_stale_blocked_run_can_retire,
        stale_run_retire_admissibility,
    };

    fn active_status() -> StaleRunGraphStatus<'static> {
        StaleRunGraphStatus {
            status: "running",
            lifecycle_stage: "active",
            next_node: Some("implement"),
            resume_target: "implement",
        }
    }

    fn terminal_closure_status() -> StaleRunGraphStatus<'static> {
        StaleRunGraphStatus {
            status: "completed",
            lifecycle_stage: "closure_complete",
            next_node: None,
            resume_target: "none",
        }
    }

    #[test]
    fn missing_task_stale_blocked_run_accepts_blocked_or_running_lane_receipt() {
        for lane_status in ["lane_running", "lane_blocked"] {
            let receipt = StaleRunGraphReceipt {
                dispatch_status: "blocked",
                lane_status,
                downstream_dispatch_status: None,
            };

            assert!(missing_task_stale_blocked_run_can_retire(
                &active_status(),
                &receipt
            ));
        }
    }

    #[test]
    fn missing_task_stale_blocked_run_accepts_prelaunch_packet_ready_receipt() {
        let receipt = StaleRunGraphReceipt {
            dispatch_status: "executed",
            lane_status: "lane_completed",
            downstream_dispatch_status: Some("packet_ready"),
        };

        assert!(missing_task_stale_blocked_run_can_retire(
            &active_status(),
            &receipt
        ));
    }

    #[test]
    fn missing_task_stale_blocked_run_rejects_terminal_closure_status() {
        let receipt = StaleRunGraphReceipt {
            dispatch_status: "blocked",
            lane_status: "lane_blocked",
            downstream_dispatch_status: None,
        };

        assert!(!missing_task_stale_blocked_run_can_retire(
            &terminal_closure_status(),
            &receipt
        ));
    }

    #[test]
    fn missing_task_stale_blocked_run_rejects_unrelated_receipt_shape() {
        let receipt = StaleRunGraphReceipt {
            dispatch_status: "executed",
            lane_status: "lane_completed",
            downstream_dispatch_status: Some("dispatched"),
        };

        assert!(!missing_task_stale_blocked_run_can_retire(
            &active_status(),
            &receipt
        ));
    }

    #[test]
    fn stale_run_retire_admissibility_matches_task_state_and_receipt_shape() {
        let retireable_receipt = StaleRunGraphReceipt {
            dispatch_status: "blocked",
            lane_status: "lane_blocked",
            downstream_dispatch_status: None,
        };
        let bridge_pending_receipt = StaleRunGraphReceipt {
            dispatch_status: "bridge_request_pending",
            lane_status: "lane_open",
            downstream_dispatch_status: None,
        };

        assert_eq!(
            stale_run_retire_admissibility(
                &active_status(),
                Some(&retireable_receipt),
                StaleRunTaskState::Missing,
            ),
            StaleRunRetireAdmissibility::AllowedMissingTask
        );
        assert_eq!(
            stale_run_retire_admissibility(&active_status(), None, StaleRunTaskState::Missing),
            StaleRunRetireAdmissibility::AllowedMissingTask
        );
        assert_eq!(
            stale_run_retire_admissibility(
                &active_status(),
                Some(&bridge_pending_receipt),
                StaleRunTaskState::Missing,
            ),
            StaleRunRetireAdmissibility::BlockedMissingTaskReceiptShape
        );
        assert_eq!(
            stale_run_retire_admissibility(
                &active_status(),
                Some(&retireable_receipt),
                StaleRunTaskState::Open,
            ),
            StaleRunRetireAdmissibility::BlockedTaskNotClosed
        );
        assert_eq!(
            stale_run_retire_admissibility(
                &terminal_closure_status(),
                Some(&retireable_receipt),
                StaleRunTaskState::Closed,
            ),
            StaleRunRetireAdmissibility::BlockedTerminalRun
        );
        assert_eq!(
            StaleRunRetireAdmissibility::BlockedTaskNotClosed.blocker_code(),
            Some("lane_retire_task_not_closed")
        );
        assert!(StaleRunRetireAdmissibility::AllowedClosedTask.is_allowed());
    }

    #[test]
    fn latest_run_graph_task_orthogonal_requires_single_active_candidate() {
        let active = TaskflowActiveCandidate {
            task_id: "active-task",
        };

        assert!(!latest_run_graph_task_orthogonal_to_taskflow_active_work(
            Some("active-task"),
            Some("active-task"),
            &[active]
        ));
        assert!(latest_run_graph_task_orthogonal_to_taskflow_active_work(
            Some("stale-task"),
            Some("active-task"),
            &[active]
        ));
        assert!(latest_run_graph_task_orthogonal_to_taskflow_active_work(
            Some("active-task"),
            Some("stale-run"),
            &[active]
        ));
        assert!(!latest_run_graph_task_orthogonal_to_taskflow_active_work(
            Some("stale-task"),
            Some("stale-run"),
            &[]
        ));
        assert!(!latest_run_graph_task_orthogonal_to_taskflow_active_work(
            Some("stale-task"),
            Some("stale-run"),
            &[
                active,
                TaskflowActiveCandidate {
                    task_id: "other-active-task",
                },
            ]
        ));
    }

    #[test]
    fn latest_run_graph_task_stale_for_write_guard_matches_status_formula() {
        assert!(latest_run_graph_task_stale_for_write_guard(
            true, false, true, false
        ));
        assert!(latest_run_graph_task_stale_for_write_guard(
            false, true, true, false
        ));
        assert!(latest_run_graph_task_stale_for_write_guard(
            false, false, false, true
        ));
        assert!(!latest_run_graph_task_stale_for_write_guard(
            false, false, true, true
        ));
        assert!(!latest_run_graph_task_stale_for_write_guard(
            false, false, false, false
        ));
    }
}
