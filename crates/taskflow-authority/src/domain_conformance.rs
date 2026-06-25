use std::collections::BTreeSet;

use taskflow_core::run_graph::model::{
    DispatchReceiptSnapshot, RunGraphStatusSnapshot, RunGraphTransitionKind, TaskClosureSnapshot,
};
use taskflow_core::task::lifecycle::{
    TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN, TaskLifecycleEvent, TaskLifecycleInput,
    TaskLifecycleStatus,
};

use crate::continuation_transition::{ContinuationGateInput, decide_continuation_gate};
use crate::run_graph_transition::{RunGraphAuthorityInput, admit_run_graph_transition};
use crate::task_transition::{TaskLifecycleRuntimeEvidence, admit_task_lifecycle};

pub const MODULE: &str = "domain_conformance";
pub const DOMAIN_CONFORMANCE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainConformanceReport {
    pub schema_version: u32,
    pub scenario_results: Vec<DomainConformanceScenarioResult>,
}

impl DomainConformanceReport {
    #[must_use]
    pub fn scenario_count(&self) -> usize {
        self.scenario_results.len()
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.scenario_results
            .iter()
            .filter(|scenario| !scenario.passed)
            .count()
    }

    #[must_use]
    pub fn clean(&self) -> bool {
        self.error_count() == 0
    }

    #[must_use]
    pub fn covered_semantic_areas(&self) -> Vec<&'static str> {
        self.scenario_results
            .iter()
            .map(|scenario| scenario.semantic_area)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainConformanceScenarioResult {
    pub name: &'static str,
    pub semantic_area: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[must_use]
pub fn verify_domain_conformance() -> DomainConformanceReport {
    let mut scenario_results = Vec::new();

    scenario_results.push(case_task_lifecycle_close_ok());
    scenario_results.push(case_task_lifecycle_child_guard());
    scenario_results.push(case_task_lifecycle_change());
    scenario_results.push(case_run_graph_terminal());
    scenario_results.push(case_run_graph_takeover());
    scenario_results.push(case_run_graph_lane_guard());
    scenario_results.push(case_run_graph_handoff());
    scenario_results.push(case_continuation_cycle());
    scenario_results.push(case_continuation_idle());

    DomainConformanceReport {
        schema_version: DOMAIN_CONFORMANCE_SCHEMA_VERSION,
        scenario_results,
    }
}

fn case_task_lifecycle_close_ok() -> DomainConformanceScenarioResult {
    let admission = admit_task_lifecycle(
        TaskLifecycleInput::new("task-1", TaskLifecycleEvent::Close),
        TaskLifecycleRuntimeEvidence::ready(),
    );

    scenario(
        "task.lifecycle.close_open_task",
        "task_lifecycle",
        admission.admitted()
            && admission.decision.next_status == Some(TaskLifecycleStatus::Closed)
            && admission.decision.touched_task_ids == vec!["task-1"]
            && admission.blocker_codes.is_empty(),
        format!(
            "status={:?}; next_status={:?}; blockers={:?}",
            admission.status, admission.decision.next_status, admission.blocker_codes
        ),
    )
}

fn case_task_lifecycle_child_guard() -> DomainConformanceScenarioResult {
    let admission = admit_task_lifecycle(
        TaskLifecycleInput::new("parent", TaskLifecycleEvent::Close),
        TaskLifecycleRuntimeEvidence {
            active_child_count: 1,
            graph_issues: Vec::new(),
            defer_lifecycle_mutation: false,
        },
    );

    scenario(
        "task.lifecycle.close_blocks_active_children",
        "task_lifecycle",
        admission.blocked()
            && admission.blocker_codes == vec![TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN],
        format!(
            "status={:?}; blockers={:?}; next_actions={:?}",
            admission.status, admission.blocker_codes, admission.next_actions
        ),
    )
}

fn case_task_lifecycle_change() -> DomainConformanceScenarioResult {
    let mut input = TaskLifecycleInput::new("task-1", TaskLifecycleEvent::UpdateStatus);
    input.requested_status = Some(TaskLifecycleStatus::InProgress);

    let admission = admit_task_lifecycle(input, TaskLifecycleRuntimeEvidence::ready());

    scenario(
        "task.lifecycle.update_status",
        "task_lifecycle",
        admission.admitted()
            && admission.decision.next_status == Some(TaskLifecycleStatus::InProgress)
            && admission.blocker_codes.is_empty(),
        format!(
            "status={:?}; next_status={:?}; blockers={:?}",
            admission.status, admission.decision.next_status, admission.blocker_codes
        ),
    )
}

fn case_run_graph_terminal() -> DomainConformanceScenarioResult {
    let decision = admit_run_graph_transition(RunGraphAuthorityInput {
        status: run_snapshot(),
        receipt: Some(exception_takeover_receipt()),
        closure: Some(TaskClosureSnapshot {
            task_id: "task-1".to_string(),
            status: "closed".to_string(),
            terminally_closed: true,
        }),
    });

    scenario(
        "run_graph.terminal_closure_wins",
        "run_graph",
        decision.decision.admitted
            && decision.decision.kind == RunGraphTransitionKind::TerminalClosure
            && decision.decision.next_node.is_none()
            && decision.decision.resume_target == "none",
        format!(
            "kind={:?}; admitted={}; resume_target={}",
            decision.decision.kind, decision.decision.admitted, decision.decision.resume_target
        ),
    )
}

fn case_run_graph_takeover() -> DomainConformanceScenarioResult {
    let decision = admit_run_graph_transition(RunGraphAuthorityInput {
        status: run_snapshot(),
        receipt: Some(exception_takeover_receipt()),
        closure: None,
    });

    scenario(
        "run_graph.exception_takeover_resumes_owner",
        "run_graph",
        decision.decision.admitted
            && decision.decision.kind == RunGraphTransitionKind::ExceptionTakeover
            && decision.decision.next_node.as_deref() == Some("developer")
            && decision.decision.resume_target == "dispatch.developer",
        format!(
            "kind={:?}; next_node={:?}; resume_target={}",
            decision.decision.kind, decision.decision.next_node, decision.decision.resume_target
        ),
    )
}

fn case_run_graph_lane_guard() -> DomainConformanceScenarioResult {
    let decision = admit_run_graph_transition(RunGraphAuthorityInput {
        status: run_snapshot(),
        receipt: Some(receipt_for_lane_state("lane_failed")),
        closure: None,
    });

    scenario(
        "run_graph.blocked_lane_fails_closed",
        "run_graph",
        !decision.decision.admitted
            && decision.decision.kind == RunGraphTransitionKind::BlockedLane
            && decision.decision.blocker_codes == vec!["lane_blocked"],
        format!(
            "kind={:?}; admitted={}; blockers={:?}",
            decision.decision.kind, decision.decision.admitted, decision.decision.blocker_codes
        ),
    )
}

fn case_run_graph_handoff() -> DomainConformanceScenarioResult {
    let mut status = run_snapshot();
    status.active_node = "developer".to_string();
    status.next_node = Some("tester".to_string());
    status.resume_target = "dispatch.tester".to_string();

    let decision = admit_run_graph_transition(RunGraphAuthorityInput {
        status,
        receipt: Some(DispatchReceiptSnapshot {
            downstream_dispatch_ready: true,
            downstream_dispatch_target: Some("tester".to_string()),
            downstream_dispatch_blockers: Vec::new(),
            ..receipt_for_lane_state("lane_open")
        }),
        closure: None,
    });

    scenario(
        "run_graph.downstream_handoff_advances_resume_target",
        "run_graph",
        decision.decision.admitted
            && decision.decision.kind == RunGraphTransitionKind::DownstreamReadyHandoff
            && decision.decision.next_node.as_deref() == Some("tester")
            && decision.decision.resume_target == "dispatch.tester",
        format!(
            "kind={:?}; next_node={:?}; resume_target={}",
            decision.decision.kind, decision.decision.next_node, decision.decision.resume_target
        ),
    )
}

fn case_continuation_cycle() -> DomainConformanceScenarioResult {
    let decision = decide_continuation_gate(ContinuationGateInput {
        delegated_cycle_open: true,
        active_exception_takeover_not_resumable: false,
    });

    scenario(
        "continuation.open_delegated_cycle_requires_progress",
        "continuation",
        decision.continuation_required_now
            && decision.pause_boundary_gate == "non_blocking_only"
            && decision.sequential_vs_parallel_posture == "sequential_only_open_cycle",
        format!(
            "required={}; pause_gate={}; posture={}",
            decision.continuation_required_now,
            decision.pause_boundary_gate,
            decision.sequential_vs_parallel_posture
        ),
    )
}

fn case_continuation_idle() -> DomainConformanceScenarioResult {
    let decision = decide_continuation_gate(ContinuationGateInput {
        delegated_cycle_open: false,
        active_exception_takeover_not_resumable: false,
    });

    scenario(
        "continuation.idle_pause_boundary",
        "continuation",
        !decision.continuation_required_now
            && decision.pause_boundary_gate == "allowed_if_no_further_bound_work_is_evidenced"
            && decision.sequential_vs_parallel_posture == "sequential_only",
        format!(
            "required={}; pause_gate={}; posture={}",
            decision.continuation_required_now,
            decision.pause_boundary_gate,
            decision.sequential_vs_parallel_posture
        ),
    )
}

fn scenario(
    name: &'static str,
    semantic_area: &'static str,
    passed: bool,
    detail: String,
) -> DomainConformanceScenarioResult {
    DomainConformanceScenarioResult {
        name,
        semantic_area,
        passed,
        detail,
    }
}

fn run_snapshot() -> RunGraphStatusSnapshot {
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

fn receipt_for_lane_state(lane_status: &str) -> DispatchReceiptSnapshot {
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
        ..receipt_for_lane_state("lane_exception_takeover")
    }
}

#[cfg(test)]
mod tests {
    use super::verify_domain_conformance;

    #[test]
    fn domain_conformance_corpus_is_clean_without_state_store() {
        let report = verify_domain_conformance();

        assert!(report.clean(), "{report:#?}");
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.scenario_count(), 9);
        assert_eq!(
            report.covered_semantic_areas(),
            vec!["continuation", "run_graph", "task_lifecycle"]
        );
    }
}
