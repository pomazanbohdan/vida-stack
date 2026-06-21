use taskflow_core::task::lifecycle::{
    TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN, TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID,
    TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID, TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS,
    TASK_LIFECYCLE_TABLE_SCHEMA_VERSION, TaskLifecycleDecision, TaskLifecycleEvent,
    TaskLifecycleInput, TaskLifecycleStatus, decide_task_lifecycle,
};

pub const MODULE: &str = "task_transition";

pub const TASK_LIFECYCLE_NEXT_ACTION_FIX_TASK_ID: &str =
    "provide a non-empty task id before mutating TaskFlow lifecycle state";
pub const TASK_LIFECYCLE_NEXT_ACTION_FIX_STATUS: &str =
    "provide a known task status before mutating TaskFlow lifecycle state";
pub const TASK_LIFECYCLE_NEXT_ACTION_FIX_GRAPH: &str =
    "repair the TaskFlow graph before mutating lifecycle state";
pub const TASK_LIFECYCLE_NEXT_ACTION_CLOSE_CHILDREN: &str =
    "close or reparent active children before closing the parent task";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskLifecycleAdmissionStatus {
    Admitted,
    Blocked,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleAdmission {
    pub schema_version: u32,
    pub event: TaskLifecycleEvent,
    pub status: TaskLifecycleAdmissionStatus,
    pub decision: TaskLifecycleDecision,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
}

impl TaskLifecycleAdmission {
    #[must_use]
    pub fn admitted(&self) -> bool {
        self.status == TaskLifecycleAdmissionStatus::Admitted
    }

    #[must_use]
    pub fn blocked(&self) -> bool {
        self.status == TaskLifecycleAdmissionStatus::Blocked
    }

    #[must_use]
    pub fn deferred(&self) -> bool {
        self.status == TaskLifecycleAdmissionStatus::Deferred
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleRuntimeEvidence {
    pub active_child_count: usize,
    pub graph_issues: Vec<taskflow_core::task::graph::TaskGraphIssue>,
    pub defer_lifecycle_mutation: bool,
}

impl TaskLifecycleRuntimeEvidence {
    #[must_use]
    pub fn ready() -> Self {
        Self {
            active_child_count: 0,
            graph_issues: Vec::new(),
            defer_lifecycle_mutation: false,
        }
    }
}

#[must_use]
pub fn admit_task_lifecycle(
    mut input: TaskLifecycleInput,
    evidence: TaskLifecycleRuntimeEvidence,
) -> TaskLifecycleAdmission {
    input.active_child_count = evidence.active_child_count;
    input.graph_issues = evidence.graph_issues;

    let event = input.event;
    let decision = decide_task_lifecycle(input);
    let blocker_codes = decision.blocker_codes.clone();
    let next_actions = next_actions_for_blockers(&blocker_codes);
    let status = if decision.admitted {
        if evidence.defer_lifecycle_mutation {
            TaskLifecycleAdmissionStatus::Deferred
        } else {
            TaskLifecycleAdmissionStatus::Admitted
        }
    } else {
        TaskLifecycleAdmissionStatus::Blocked
    };

    TaskLifecycleAdmission {
        schema_version: TASK_LIFECYCLE_TABLE_SCHEMA_VERSION,
        event,
        status,
        decision,
        blocker_codes,
        next_actions,
    }
}

#[must_use]
pub fn lifecycle_status_from_str(status: &str) -> Result<TaskLifecycleStatus, String> {
    TaskLifecycleStatus::try_from(status).map_err(|error| {
        let _ = error;
        TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS.to_string()
    })
}

fn next_actions_for_blockers(blocker_codes: &[String]) -> Vec<String> {
    blocker_codes
        .iter()
        .filter_map(|code| match code.as_str() {
            TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID => {
                Some(TASK_LIFECYCLE_NEXT_ACTION_FIX_TASK_ID.to_string())
            }
            TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS => {
                Some(TASK_LIFECYCLE_NEXT_ACTION_FIX_STATUS.to_string())
            }
            TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID => {
                Some(TASK_LIFECYCLE_NEXT_ACTION_FIX_GRAPH.to_string())
            }
            TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN => {
                Some(TASK_LIFECYCLE_NEXT_ACTION_CLOSE_CHILDREN.to_string())
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod lifecycle_tests {
    use super::{
        TaskLifecycleAdmissionStatus, TaskLifecycleRuntimeEvidence, admit_task_lifecycle,
        lifecycle_status_from_str,
    };
    use taskflow_core::task::graph::TaskGraphIssue;
    use taskflow_core::task::lifecycle::{
        TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN, TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID,
        TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS, TaskLifecycleEvent, TaskLifecycleInput,
        TaskLifecycleStatus,
    };

    #[test]
    fn lifecycle_authority_admits_core_decision_with_runtime_evidence() {
        let mut input = TaskLifecycleInput::new("task-1", TaskLifecycleEvent::UpdateStatus);
        input.requested_status = Some(TaskLifecycleStatus::InProgress);

        let admission = admit_task_lifecycle(input, TaskLifecycleRuntimeEvidence::ready());

        assert!(admission.admitted());
        assert_eq!(admission.status, TaskLifecycleAdmissionStatus::Admitted);
        assert_eq!(
            admission.decision.next_status,
            Some(TaskLifecycleStatus::InProgress)
        );
        assert_eq!(admission.decision.touched_task_ids, vec!["task-1"]);
        assert!(admission.blocker_codes.is_empty());
        assert!(admission.next_actions.is_empty());
    }

    #[test]
    fn lifecycle_authority_blocks_parent_close_when_children_remain() {
        let admission = admit_task_lifecycle(
            TaskLifecycleInput::new("parent", TaskLifecycleEvent::Close),
            TaskLifecycleRuntimeEvidence {
                active_child_count: 2,
                graph_issues: Vec::new(),
                defer_lifecycle_mutation: false,
            },
        );

        assert!(admission.blocked());
        assert_eq!(
            admission.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN]
        );
        assert_eq!(admission.next_actions.len(), 1);
    }

    #[test]
    fn lifecycle_authority_defers_admitted_decision_without_recomputing_status() {
        let admission = admit_task_lifecycle(
            TaskLifecycleInput::new("task-1", TaskLifecycleEvent::Close),
            TaskLifecycleRuntimeEvidence {
                active_child_count: 0,
                graph_issues: Vec::new(),
                defer_lifecycle_mutation: true,
            },
        );

        assert!(admission.deferred());
        assert_eq!(admission.status, TaskLifecycleAdmissionStatus::Deferred);
        assert_eq!(
            admission.decision.next_status,
            Some(TaskLifecycleStatus::Closed)
        );
        assert!(admission.blocker_codes.is_empty());
    }

    #[test]
    fn lifecycle_authority_preserves_graph_blocker_codes_and_actions() {
        let admission = admit_task_lifecycle(
            TaskLifecycleInput::new("task-1", TaskLifecycleEvent::Close),
            TaskLifecycleRuntimeEvidence {
                active_child_count: 0,
                graph_issues: vec![TaskGraphIssue {
                    issue_type: "self_dependency".to_string(),
                    issue_id: "task-1".to_string(),
                    depends_on_id: Some("task-1".to_string()),
                    edge_type: Some("blocks".to_string()),
                    detail: "task must not depend on itself".to_string(),
                }],
                defer_lifecycle_mutation: false,
            },
        );

        assert!(admission.blocked());
        assert_eq!(
            admission.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID]
        );
        assert_eq!(admission.decision.graph_issues.len(), 1);
        assert_eq!(admission.next_actions.len(), 1);
    }

    #[test]
    fn lifecycle_authority_normalizes_status_parse_errors_to_blocker_code() {
        assert_eq!(
            lifecycle_status_from_str("unknown"),
            Err(TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS.to_string())
        );
        assert_eq!(
            lifecycle_status_from_str("completed"),
            Ok(TaskLifecycleStatus::Closed)
        );
    }
}
