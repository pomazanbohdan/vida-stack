use serde::{Deserialize, Serialize};
use taskflow_core::{IssueType, TaskId, TaskStatus, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub issue_type: IssueType,
    pub updated_at: Timestamp,
}

impl TaskRecord {
    #[must_use]
    pub fn new(id: TaskId, title: impl Into<String>, issue_type: IssueType) -> Self {
        Self {
            id,
            title: title.into(),
            status: TaskStatus::Open,
            issue_type,
            updated_at: Timestamp::now_utc(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub issue_id: TaskId,
    pub depends_on_id: TaskId,
    pub dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::TaskRecord;
    use taskflow_core::{IssueType, TaskId, TaskStatus};

    #[test]
    fn task_record_defaults_to_open() {
        let record = TaskRecord::new(TaskId::new("vida-rf1"), "program", IssueType::Epic);
        assert!(matches!(record.status, TaskStatus::Open));
    }
}
