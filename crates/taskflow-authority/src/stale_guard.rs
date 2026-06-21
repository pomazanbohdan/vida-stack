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
    terminal_retired_runtime_run: bool,
    exception_takeover_matches_active_taskflow_work: bool,
    latest_run_graph_task_orthogonal_to_taskflow_active_work: bool,
) -> bool {
    let terminal_retired_missing_task_run =
        latest_run_graph_task_missing && terminal_retired_runtime_run;
    latest_run_graph_task_closed
        || (!terminal_retired_missing_task_run && latest_run_graph_task_missing)
        || (!terminal_retired_missing_task_run
            && !exception_takeover_matches_active_taskflow_work
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
        StaleRunGraphReceipt, StaleRunGraphStatus, TaskflowActiveCandidate,
        latest_run_graph_task_orthogonal_to_taskflow_active_work,
        latest_run_graph_task_stale_for_write_guard, missing_task_stale_blocked_run_can_retire,
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
            true, false, false, true, false
        ));
        assert!(latest_run_graph_task_stale_for_write_guard(
            false, true, false, true, false
        ));
        assert!(latest_run_graph_task_stale_for_write_guard(
            false, false, false, false, true
        ));
        assert!(!latest_run_graph_task_stale_for_write_guard(
            false, false, false, true, true
        ));
        assert!(!latest_run_graph_task_stale_for_write_guard(
            false, false, false, false, false
        ));
        assert!(!latest_run_graph_task_stale_for_write_guard(
            true, false, true, false, true
        ));
        assert!(latest_run_graph_task_stale_for_write_guard(
            false, true, true, true, false
        ));
    }
}
