//! Pure task lifecycle transition table.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{TaskStatus, parse_task_status};

use super::graph::TaskGraphIssue;

pub const TASK_LIFECYCLE_TABLE_SCHEMA_VERSION: u32 = 1;

pub const TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID: &str = "empty_task_id";
pub const TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS: &str = "unknown_task_status";
pub const TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID: &str = "task_graph_invalid";
pub const TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN: &str = "active_children_remain";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecycleStatus {
    Open,
    InProgress,
    Closed,
    Paused,
}

impl TaskLifecycleStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
            Self::Paused => "paused",
        }
    }
}

impl TryFrom<&str> for TaskLifecycleStatus {
    type Error = TaskLifecycleStatusParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match parse_task_status(value) {
            Some(TaskStatus::Open) => Ok(Self::Open),
            Some(TaskStatus::InProgress) => Ok(Self::InProgress),
            Some(TaskStatus::Closed) => Ok(Self::Closed),
            Some(TaskStatus::Blocked) => Ok(Self::Paused),
            None => Err(TaskLifecycleStatusParseError {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleStatusParseError {
    pub value: String,
}

impl std::fmt::Display for TaskLifecycleStatusParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown task lifecycle status `{}`", self.value)
    }
}

impl std::error::Error for TaskLifecycleStatusParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecycleEvent {
    Create,
    UpdateStatus,
    Close,
    Reparent,
    ExtendParent,
    EmptyParent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TaskLifecycleEffect {
    SetStatus {
        task_id: String,
        status: TaskLifecycleStatus,
    },
    TouchTask {
        task_id: String,
    },
    ReopenParent {
        task_id: String,
    },
    CloseParent {
        task_id: String,
    },
}

impl TaskLifecycleEffect {
    fn touched_task_id(&self) -> &str {
        match self {
            Self::SetStatus { task_id, .. }
            | Self::TouchTask { task_id }
            | Self::ReopenParent { task_id }
            | Self::CloseParent { task_id } => task_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleInput {
    pub schema_version: u32,
    pub task_id: String,
    pub event: TaskLifecycleEvent,
    pub current_status: Option<TaskLifecycleStatus>,
    pub requested_status: Option<TaskLifecycleStatus>,
    pub parent_id: Option<String>,
    pub active_child_count: usize,
    pub graph_issues: Vec<TaskGraphIssue>,
}

impl TaskLifecycleInput {
    #[must_use]
    pub fn new(task_id: impl Into<String>, event: TaskLifecycleEvent) -> Self {
        Self {
            schema_version: TASK_LIFECYCLE_TABLE_SCHEMA_VERSION,
            task_id: task_id.into(),
            event,
            current_status: None,
            requested_status: None,
            parent_id: None,
            active_child_count: 0,
            graph_issues: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleDecision {
    pub schema_version: u32,
    pub event: TaskLifecycleEvent,
    pub admitted: bool,
    pub next_status: Option<TaskLifecycleStatus>,
    pub effects: Vec<TaskLifecycleEffect>,
    pub touched_task_ids: Vec<String>,
    pub blocker_codes: Vec<String>,
    pub graph_issues: Vec<TaskGraphIssue>,
}

impl TaskLifecycleDecision {
    fn admitted(
        event: TaskLifecycleEvent,
        next_status: Option<TaskLifecycleStatus>,
        effects: Vec<TaskLifecycleEffect>,
    ) -> Self {
        Self {
            schema_version: TASK_LIFECYCLE_TABLE_SCHEMA_VERSION,
            event,
            admitted: true,
            next_status,
            touched_task_ids: touched_task_ids(&effects),
            effects,
            blocker_codes: Vec::new(),
            graph_issues: Vec::new(),
        }
    }

    fn blocked(
        event: TaskLifecycleEvent,
        blocker_codes: Vec<String>,
        graph_issues: Vec<TaskGraphIssue>,
    ) -> Self {
        Self {
            schema_version: TASK_LIFECYCLE_TABLE_SCHEMA_VERSION,
            event,
            admitted: false,
            next_status: None,
            effects: Vec::new(),
            touched_task_ids: Vec::new(),
            blocker_codes,
            graph_issues,
        }
    }
}

#[must_use]
pub fn decide_task_lifecycle(input: TaskLifecycleInput) -> TaskLifecycleDecision {
    let task_id = input.task_id.trim();
    if task_id.is_empty() {
        return TaskLifecycleDecision::blocked(
            input.event,
            vec![TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID.to_string()],
            Vec::new(),
        );
    }
    if !input.graph_issues.is_empty() {
        return TaskLifecycleDecision::blocked(
            input.event,
            vec![TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID.to_string()],
            input.graph_issues,
        );
    }

    match input.event {
        TaskLifecycleEvent::Create => admit_status(
            input.event,
            task_id,
            input.requested_status.unwrap_or(TaskLifecycleStatus::Open),
        ),
        TaskLifecycleEvent::UpdateStatus => match input.requested_status {
            Some(status) => admit_status(input.event, task_id, status),
            None => TaskLifecycleDecision::blocked(
                input.event,
                vec![TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS.to_string()],
                Vec::new(),
            ),
        },
        TaskLifecycleEvent::Close if input.active_child_count > 0 => {
            TaskLifecycleDecision::blocked(
                input.event,
                vec![TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN.to_string()],
                Vec::new(),
            )
        }
        TaskLifecycleEvent::Close => {
            admit_status(input.event, task_id, TaskLifecycleStatus::Closed)
        }
        TaskLifecycleEvent::Reparent => {
            let mut effects = vec![TaskLifecycleEffect::TouchTask {
                task_id: task_id.to_string(),
            }];
            if let Some(parent_id) = input
                .parent_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
            {
                effects.push(TaskLifecycleEffect::ReopenParent {
                    task_id: parent_id.to_string(),
                });
            }
            TaskLifecycleDecision::admitted(input.event, input.current_status, effects)
        }
        TaskLifecycleEvent::ExtendParent => TaskLifecycleDecision::admitted(
            input.event,
            Some(TaskLifecycleStatus::Open),
            vec![TaskLifecycleEffect::ReopenParent {
                task_id: task_id.to_string(),
            }],
        ),
        TaskLifecycleEvent::EmptyParent => TaskLifecycleDecision::admitted(
            input.event,
            Some(TaskLifecycleStatus::Closed),
            vec![TaskLifecycleEffect::CloseParent {
                task_id: task_id.to_string(),
            }],
        ),
    }
}

fn admit_status(
    event: TaskLifecycleEvent,
    task_id: &str,
    status: TaskLifecycleStatus,
) -> TaskLifecycleDecision {
    TaskLifecycleDecision::admitted(
        event,
        Some(status),
        vec![TaskLifecycleEffect::SetStatus {
            task_id: task_id.to_string(),
            status,
        }],
    )
}

fn touched_task_ids(effects: &[TaskLifecycleEffect]) -> Vec<String> {
    effects
        .iter()
        .map(TaskLifecycleEffect::touched_task_id)
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN, TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID,
        TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID, TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS,
        TASK_LIFECYCLE_TABLE_SCHEMA_VERSION, TaskLifecycleEffect, TaskLifecycleEvent,
        TaskLifecycleInput, TaskLifecycleStatus, decide_task_lifecycle,
    };
    use crate::task::graph::TaskGraphIssue;

    #[test]
    fn task_lifecycle_create_defaults_to_open_with_stable_schema() {
        let decision = decide_task_lifecycle(TaskLifecycleInput::new(
            "task-1",
            TaskLifecycleEvent::Create,
        ));

        assert!(decision.admitted);
        assert_eq!(decision.schema_version, TASK_LIFECYCLE_TABLE_SCHEMA_VERSION);
        assert_eq!(decision.next_status, Some(TaskLifecycleStatus::Open));
        assert_eq!(
            decision.effects,
            vec![TaskLifecycleEffect::SetStatus {
                task_id: "task-1".to_string(),
                status: TaskLifecycleStatus::Open
            }]
        );
        assert_eq!(decision.touched_task_ids, vec!["task-1"]);
    }

    #[test]
    fn task_lifecycle_update_requires_requested_status() {
        let decision = decide_task_lifecycle(TaskLifecycleInput::new(
            "task-1",
            TaskLifecycleEvent::UpdateStatus,
        ));

        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS]
        );
    }

    #[test]
    fn task_lifecycle_close_rejects_active_children() {
        let mut input = TaskLifecycleInput::new("parent", TaskLifecycleEvent::Close);
        input.active_child_count = 1;

        let decision = decide_task_lifecycle(input);

        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN]
        );
        assert!(decision.effects.is_empty());
    }

    #[test]
    fn task_lifecycle_reparent_returns_touch_and_reopen_effects() {
        let mut input = TaskLifecycleInput::new("child", TaskLifecycleEvent::Reparent);
        input.current_status = Some(TaskLifecycleStatus::Open);
        input.parent_id = Some("new-parent".to_string());

        let decision = decide_task_lifecycle(input);

        assert!(decision.admitted);
        assert_eq!(decision.touched_task_ids, vec!["child", "new-parent"]);
        assert_eq!(
            decision.effects,
            vec![
                TaskLifecycleEffect::TouchTask {
                    task_id: "child".to_string()
                },
                TaskLifecycleEffect::ReopenParent {
                    task_id: "new-parent".to_string()
                }
            ]
        );
    }

    #[test]
    fn task_lifecycle_reparent_without_parent_only_touches_child() {
        let input = TaskLifecycleInput::new("child", TaskLifecycleEvent::Reparent);

        let decision = decide_task_lifecycle(input);

        assert!(decision.admitted);
        assert_eq!(decision.touched_task_ids, vec!["child"]);
        assert_eq!(
            decision.effects,
            vec![TaskLifecycleEffect::TouchTask {
                task_id: "child".to_string()
            }]
        );
    }

    #[test]
    fn task_lifecycle_returns_graph_issues_without_db_or_filesystem() {
        let mut input = TaskLifecycleInput::new("task-1", TaskLifecycleEvent::Close);
        input.graph_issues = vec![TaskGraphIssue {
            issue_type: "self_dependency".to_string(),
            issue_id: "task-1".to_string(),
            depends_on_id: Some("task-1".to_string()),
            edge_type: Some("blocks".to_string()),
            detail: "task must not depend on itself".to_string(),
        }];

        let decision = decide_task_lifecycle(input);

        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_GRAPH_INVALID]
        );
        assert_eq!(decision.graph_issues.len(), 1);
    }

    #[test]
    fn task_lifecycle_rejects_empty_task_id_and_parses_status_aliases() {
        let decision =
            decide_task_lifecycle(TaskLifecycleInput::new(" ", TaskLifecycleEvent::Close));

        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec![TASK_LIFECYCLE_BLOCKER_EMPTY_TASK_ID]
        );
        assert_eq!(
            TaskLifecycleStatus::try_from("completed"),
            Ok(TaskLifecycleStatus::Closed)
        );
        assert_eq!(TaskLifecycleStatus::Paused.as_str(), "paused");
    }

    #[test]
    fn task_lifecycle_matrix_preserves_status_and_blocker_contracts() {
        struct Case {
            event: TaskLifecycleEvent,
            requested_status: Option<TaskLifecycleStatus>,
            active_child_count: usize,
            admitted: bool,
            next_status: Option<TaskLifecycleStatus>,
            blocker_codes: &'static [&'static str],
        }

        let cases = [
            Case {
                event: TaskLifecycleEvent::Create,
                requested_status: None,
                active_child_count: 0,
                admitted: true,
                next_status: Some(TaskLifecycleStatus::Open),
                blocker_codes: &[],
            },
            Case {
                event: TaskLifecycleEvent::UpdateStatus,
                requested_status: Some(TaskLifecycleStatus::InProgress),
                active_child_count: 0,
                admitted: true,
                next_status: Some(TaskLifecycleStatus::InProgress),
                blocker_codes: &[],
            },
            Case {
                event: TaskLifecycleEvent::UpdateStatus,
                requested_status: None,
                active_child_count: 0,
                admitted: false,
                next_status: None,
                blocker_codes: &[TASK_LIFECYCLE_BLOCKER_UNKNOWN_STATUS],
            },
            Case {
                event: TaskLifecycleEvent::Close,
                requested_status: None,
                active_child_count: 0,
                admitted: true,
                next_status: Some(TaskLifecycleStatus::Closed),
                blocker_codes: &[],
            },
            Case {
                event: TaskLifecycleEvent::Close,
                requested_status: None,
                active_child_count: 2,
                admitted: false,
                next_status: None,
                blocker_codes: &[TASK_LIFECYCLE_BLOCKER_ACTIVE_CHILDREN_REMAIN],
            },
        ];

        for case in cases {
            let mut input = TaskLifecycleInput::new("task-1", case.event);
            input.requested_status = case.requested_status;
            input.active_child_count = case.active_child_count;

            let decision = decide_task_lifecycle(input);

            assert_eq!(decision.admitted, case.admitted, "{:?}", case.event);
            assert_eq!(decision.next_status, case.next_status, "{:?}", case.event);
            assert_eq!(
                decision.blocker_codes, case.blocker_codes,
                "{:?}",
                case.event
            );
            assert_eq!(
                decision.effects.is_empty(),
                !case.admitted,
                "{:?}",
                case.event
            );
        }
    }
}
