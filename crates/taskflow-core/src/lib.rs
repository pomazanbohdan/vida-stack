use nutype::nutype;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

/// TaskFlow consume/resume skeletons for future core extraction.
pub mod consume;
/// Storage-neutral deterministic side-effect intent model.
pub mod effects;
/// Shared TaskFlow path normalization and ownership policy.
pub mod path_policy;
/// Compiled role-step state for configured development-team flows.
pub mod role_step;
/// TaskFlow run-graph skeletons for future core extraction.
pub mod run_graph;
/// Deterministic run workflow aggregate and state machine.
pub mod run_workflow;
/// Shared runtime packet identity and receipt pairing policy.
pub mod runtime_packet_identity;
/// TaskFlow scheduling skeletons for future core extraction.
pub mod scheduling;
/// TaskFlow task command skeletons for future core extraction.
pub mod task;

#[derive(Debug, Error)]
pub enum TaskflowCoreError {
    #[error("empty task identifier is not allowed")]
    EmptyTaskId,
}

#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
struct ValidatedTaskId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(ValidatedTaskId);

impl TaskId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("task id should be non-empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, TaskflowCoreError> {
        ValidatedTaskId::try_new(value.into())
            .map(Self)
            .map_err(|_| TaskflowCoreError::EmptyTaskId)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_inner()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
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
    Subtask,
    Step,
    Bug,
    Spike,
}

#[must_use]
pub fn normalize_issue_type(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

#[must_use]
pub fn canonical_issue_type(value: &str) -> String {
    match normalize_issue_type(value).as_str() {
        "sub_task" => "subtask".to_string(),
        "todo" => "step".to_string(),
        canonical => canonical.to_string(),
    }
}

#[must_use]
pub fn issue_type_is_execution_step(value: &str) -> bool {
    canonical_issue_type(value) == "step"
}

#[must_use]
pub fn issue_type_contributes_to_task_stats(value: &str) -> bool {
    !issue_type_is_execution_step(value)
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

pub fn validate_task_id(id: &TaskId) -> Result<(), TaskflowCoreError> {
    TaskId::try_new(id.as_str()).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{
        TaskId, TaskStatus, canonical_issue_type, canonical_task_status,
        issue_type_contributes_to_task_stats, issue_type_is_execution_step, parse_task_status,
        task_status_is_closed_like, task_status_is_open_like, validate_task_id,
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
    fn issue_type_taxonomy_canonicalizes_step_and_subtask() {
        assert_eq!(canonical_issue_type("sub-task"), "subtask");
        assert_eq!(canonical_issue_type("Sub Task"), "subtask");
        assert_eq!(canonical_issue_type("todo"), "step");
        assert_eq!(canonical_issue_type("TODO"), "step");
        assert!(issue_type_is_execution_step("step"));
        assert!(issue_type_is_execution_step("todo"));
        assert!(!issue_type_contributes_to_task_stats("todo"));
        assert!(issue_type_contributes_to_task_stats("subtask"));
    }

    #[test]
    fn empty_task_id_is_rejected() {
        assert!(TaskId::try_new("   ").is_err());
    }

    #[test]
    fn task_id_display_uses_validated_string_without_recursion() {
        let id = TaskId::new(" vida-rf1 ");

        assert_eq!(id.to_string(), "vida-rf1");
    }

    #[test]
    fn task_id_serializes_as_public_json_string() {
        let id = TaskId::new(" vida-rf1 ");
        assert_eq!(id.as_str(), "vida-rf1");
        let json = serde_json::to_string(&id).expect("task id should serialize");
        assert_eq!(json, "\"vida-rf1\"");
        let restored: TaskId = serde_json::from_str(&json).expect("task id should deserialize");
        assert_eq!(restored.as_str(), "vida-rf1");
        assert!(validate_task_id(&restored).is_ok());
    }
}
