use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

/// TaskFlow consume/resume skeletons for future core extraction.
pub mod consume;
/// Shared TaskFlow path normalization and ownership policy.
pub mod path_policy;
/// TaskFlow run-graph skeletons for future core extraction.
pub mod run_graph;
/// TaskFlow scheduling skeletons for future core extraction.
pub mod scheduling;
/// TaskFlow task command skeletons for future core extraction.
pub mod task;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Closed,
    Blocked,
}

impl TaskStatus {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed)
    }

    #[must_use]
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
            Self::Blocked => "paused",
        }
    }
}

fn normalize_task_status_token(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

#[must_use]
pub fn parse_task_status(value: &str) -> Option<TaskStatus> {
    match normalize_task_status_token(value).as_str() {
        "" => None,
        "open" | "new" | "todo" | "to_do" | "backlog" => Some(TaskStatus::Open),
        "in_progress" | "progress" | "started" | "doing" | "active" => Some(TaskStatus::InProgress),
        "done" | "closed" | "complete" | "completed" | "resolved" | "merged" => {
            Some(TaskStatus::Closed)
        }
        "paused" | "blocked" => Some(TaskStatus::Blocked),
        _ => None,
    }
}

#[must_use]
pub fn canonical_task_status(value: &str) -> Option<&'static str> {
    parse_task_status(value).map(TaskStatus::canonical_name)
}

#[must_use]
pub fn task_status_is_closed_like(value: &str) -> bool {
    parse_task_status(value).is_some_and(TaskStatus::is_terminal)
}

#[must_use]
pub fn task_status_is_open_like(value: &str) -> bool {
    matches!(
        parse_task_status(value),
        Some(TaskStatus::Open | TaskStatus::InProgress)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Epic,
    Task,
    Bug,
    Spike,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptId(pub Uuid);

impl ReceiptId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ReceiptId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamp(pub OffsetDateTime);

impl Timestamp {
    #[must_use]
    pub fn now_utc() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

#[derive(Debug, Error)]
pub enum TaskflowCoreError {
    #[error("empty task identifier is not allowed")]
    EmptyTaskId,
}

#[must_use]
pub fn validate_task_id(id: &TaskId) -> Result<(), TaskflowCoreError> {
    if id.0.trim().is_empty() {
        Err(TaskflowCoreError::EmptyTaskId)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TaskId, TaskStatus, canonical_task_status, parse_task_status, task_status_is_closed_like,
        task_status_is_open_like, validate_task_id,
    };

    #[test]
    fn task_status_terminal_rule_is_explicit() {
        assert!(TaskStatus::Closed.is_terminal());
        assert!(!TaskStatus::Open.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
    }

    #[test]
    fn task_status_aliases_have_one_canonical_policy() {
        for alias in [
            "closed",
            "completed",
            "complete",
            "done",
            "resolved",
            "merged",
        ] {
            assert_eq!(parse_task_status(alias), Some(TaskStatus::Closed));
            assert_eq!(canonical_task_status(alias), Some("closed"));
            assert!(task_status_is_closed_like(alias));
            assert!(!task_status_is_open_like(alias));
        }
        for alias in ["open", "new", "todo", "backlog"] {
            assert_eq!(canonical_task_status(alias), Some("open"));
            assert!(task_status_is_open_like(alias));
        }
        for alias in ["in-progress", "progress", "started", "doing", "active"] {
            assert_eq!(canonical_task_status(alias), Some("in_progress"));
            assert!(task_status_is_open_like(alias));
        }
        assert_eq!(canonical_task_status("cancelled"), None);
        assert!(!task_status_is_closed_like("cancelled"));
    }

    #[test]
    fn empty_task_id_is_rejected() {
        let id = TaskId::new("   ");
        assert!(validate_task_id(&id).is_err());
    }
}
