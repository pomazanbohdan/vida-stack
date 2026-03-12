use std::collections::HashMap;

use taskflow_contracts::{DependencyEdge, TaskRecord};
use taskflow_core::TaskId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskflowStateError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
}

pub trait TaskStore {
    fn upsert_task(&mut self, task: TaskRecord);
    fn get_task(&self, id: &TaskId) -> Result<&TaskRecord, TaskflowStateError>;
    fn list_tasks(&self) -> Vec<&TaskRecord>;
    fn add_dependency(&mut self, edge: DependencyEdge);
    fn list_dependencies(&self, id: &TaskId) -> Vec<&DependencyEdge>;
}

#[derive(Debug, Default)]
pub struct InMemoryTaskStore {
    tasks: HashMap<String, TaskRecord>,
    dependencies: Vec<DependencyEdge>,
}

impl TaskStore for InMemoryTaskStore {
    fn upsert_task(&mut self, task: TaskRecord) {
        self.tasks.insert(task.id.0.clone(), task);
    }

    fn get_task(&self, id: &TaskId) -> Result<&TaskRecord, TaskflowStateError> {
        self.tasks
            .get(&id.0)
            .ok_or_else(|| TaskflowStateError::TaskNotFound(id.0.clone()))
    }

    fn list_tasks(&self) -> Vec<&TaskRecord> {
        let mut rows: Vec<_> = self.tasks.values().collect();
        rows.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        rows
    }

    fn add_dependency(&mut self, edge: DependencyEdge) {
        self.dependencies.push(edge);
    }

    fn list_dependencies(&self, id: &TaskId) -> Vec<&DependencyEdge> {
        self.dependencies
            .iter()
            .filter(|edge| edge.issue_id.0 == id.0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemoryTaskStore, TaskStore, TaskflowStateError};
    use taskflow_contracts::{DependencyEdge, TaskRecord};
    use taskflow_core::{IssueType, TaskId};

    #[test]
    fn in_memory_store_round_trips_task_records() {
        let mut store = InMemoryTaskStore::default();
        let task = TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-state"),
            "state",
            IssueType::Task,
        );

        store.upsert_task(task.clone());

        let loaded = store
            .get_task(&TaskId::new("vida-rf1-taskflow-state"))
            .expect("task should exist");
        assert_eq!(loaded.title, task.title);
    }

    #[test]
    fn missing_task_is_reported() {
        let store = InMemoryTaskStore::default();
        let error = store
            .get_task(&TaskId::new("missing"))
            .expect_err("task should not exist");
        assert_eq!(error, TaskflowStateError::TaskNotFound("missing".into()));
    }

    #[test]
    fn dependency_listing_is_scoped_to_issue() {
        let mut store = InMemoryTaskStore::default();
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-state"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-core"),
            dependency_type: "blocks".into(),
        });
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-state"),
            dependency_type: "blocks".into(),
        });

        let rows = store.list_dependencies(&TaskId::new("vida-rf1-taskflow-state"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].depends_on_id.0, "vida-rf1-taskflow-core");
    }
}
