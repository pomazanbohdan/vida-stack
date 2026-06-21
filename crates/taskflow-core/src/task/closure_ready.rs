//! Pure task closure-readiness projection helpers.

use super::progress::TaskProgressSummary;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskClosureReadiness {
    pub ready_for_close: bool,
    pub closure_candidate: bool,
    pub closure_candidate_state: String,
    pub closure_candidate_reason: Option<String>,
    pub next_required_command: Option<String>,
    pub recommended_next_action: String,
}

#[must_use]
pub fn task_closure_readiness_from_progress(summary: &TaskProgressSummary) -> TaskClosureReadiness {
    TaskClosureReadiness {
        ready_for_close: summary.ready_for_close,
        closure_candidate: summary.closure_candidate,
        closure_candidate_state: summary.closure_candidate_state.clone(),
        closure_candidate_reason: summary.closure_candidate_reason.clone(),
        next_required_command: summary.next_required_command.clone(),
        recommended_next_action: summary.recommended_next_action.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::task_closure_readiness_from_progress;
    use crate::task::progress::{TaskProgressRow, TaskProgressSummary};

    #[test]
    fn closure_readiness_is_projected_from_core_progress_policy() {
        let summary = TaskProgressSummary {
            root_task: TaskProgressRow {
                id: "VH-30".to_string(),
                title: "Extract task policy".to_string(),
                status: "open".to_string(),
                issue_type: "task".to_string(),
                priority: 2,
                labels: Vec::new(),
                proof_targets: Vec::new(),
                proof_satisfied: false,
                parent_id: None,
            },
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: 1,
            descendant_count: 1,
            open_count: 0,
            in_progress_count: 0,
            closed_count: 1,
            epic_count: 0,
            status_counts: BTreeMap::from([("closed".to_string(), 1)]),
            percent_closed: 100.0,
            closure_candidate: true,
            closure_candidate_state: "ready_to_close".to_string(),
            closure_candidate_reason: Some("descendants are closed".to_string()),
            ready_for_close: true,
            missing_proof: false,
            proof_blocked_by_runtime: false,
            blocked_by_runtime: false,
            next_required_command: Some(
                "vida task close VH-30 --reason \"descendants closed\"".to_string(),
            ),
            recommended_next_action: "vida task close VH-30 --reason \"descendants closed\""
                .to_string(),
            canonical_commands: Vec::new(),
        };

        let readiness = task_closure_readiness_from_progress(&summary);

        assert!(readiness.ready_for_close);
        assert_eq!(readiness.closure_candidate_state, "ready_to_close");
        assert_eq!(
            readiness.next_required_command.as_deref(),
            Some("vida task close VH-30 --reason \"descendants closed\"")
        );
    }
}
