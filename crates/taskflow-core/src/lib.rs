use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

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
    use super::{TaskId, TaskStatus, validate_task_id};

    #[test]
    fn task_status_terminal_rule_is_explicit() {
        assert!(TaskStatus::Closed.is_terminal());
        assert!(!TaskStatus::Open.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
    }

    #[test]
    fn empty_task_id_is_rejected() {
        let id = TaskId::new("   ");
        assert!(validate_task_id(&id).is_err());
    }
}
