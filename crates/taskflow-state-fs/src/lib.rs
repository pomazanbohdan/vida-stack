use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use taskflow_contracts::{DependencyEdge, TaskRecord};
use taskflow_state::{InMemoryTaskStore, TaskStore};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSnapshot {
    pub tasks: Vec<TaskRecord>,
    pub dependencies: Vec<DependencyEdge>,
}

#[must_use]
pub fn snapshot_from_store(store: &impl TaskStore) -> TaskSnapshot {
    let mut tasks: Vec<TaskRecord> = store.list_tasks().into_iter().cloned().collect();
    tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));

    let mut dependencies: Vec<DependencyEdge> = tasks
        .iter()
        .flat_map(|task| store.list_dependencies(&task.id).into_iter().cloned())
        .collect();
    dependencies.sort_by(|left, right| {
        left.issue_id
            .0
            .cmp(&right.issue_id.0)
            .then_with(|| left.depends_on_id.0.cmp(&right.depends_on_id.0))
            .then_with(|| left.dependency_type.cmp(&right.dependency_type))
    });

    TaskSnapshot {
        tasks,
        dependencies,
    }
}

#[must_use]
pub fn restore_in_memory_store(snapshot: &TaskSnapshot) -> InMemoryTaskStore {
    let mut store = InMemoryTaskStore::default();
    for task in &snapshot.tasks {
        store.upsert_task(task.clone());
    }
    for dependency in &snapshot.dependencies {
        store.add_dependency(dependency.clone());
    }
    store
}

pub fn write_snapshot(
    path: impl AsRef<Path>,
    snapshot: &TaskSnapshot,
) -> Result<(), std::io::Error> {
    let payload = serde_json::to_vec_pretty(snapshot)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(path, payload)
}

pub fn read_snapshot(path: impl AsRef<Path>) -> Result<TaskSnapshot, std::io::Error> {
    let payload = fs::read(path)?;
    serde_json::from_slice(&payload).map_err(|error| std::io::Error::other(error.to_string()))
}

pub fn write_store_snapshot(
    path: impl AsRef<Path>,
    store: &impl TaskStore,
) -> Result<(), std::io::Error> {
    write_snapshot(path, &snapshot_from_store(store))
}

pub fn read_snapshot_into_memory(
    path: impl AsRef<Path>,
) -> Result<InMemoryTaskStore, std::io::Error> {
    let snapshot = read_snapshot(path)?;
    Ok(restore_in_memory_store(&snapshot))
}

#[cfg(test)]
mod tests {
    use super::{
        TaskSnapshot, read_snapshot, read_snapshot_into_memory, restore_in_memory_store,
        snapshot_from_store, write_snapshot, write_store_snapshot,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use taskflow_contracts::{DependencyEdge, TaskRecord};
    use taskflow_core::{IssueType, TaskId};
    use taskflow_state::{InMemoryTaskStore, TaskStore};

    fn temp_snapshot_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough")
            .as_nanos();
        std::env::temp_dir().join(format!("taskflow-state-fs-{nanos}.json"))
    }

    #[test]
    fn snapshot_round_trips_to_disk() {
        let path = temp_snapshot_path();
        let snapshot = TaskSnapshot {
            tasks: vec![TaskRecord::new(
                TaskId::new("vida-rf1-taskflow-state"),
                "state",
                IssueType::Task,
            )],
            dependencies: vec![DependencyEdge {
                issue_id: TaskId::new("vida-rf1-taskflow-state"),
                depends_on_id: TaskId::new("vida-rf1-taskflow-core"),
                dependency_type: "blocks".into(),
            }],
        };

        write_snapshot(&path, &snapshot).expect("snapshot should write");
        let loaded = read_snapshot(&path).expect("snapshot should load");
        fs::remove_file(&path).expect("temp snapshot should be removed");

        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.dependencies.len(), 1);
        assert_eq!(
            loaded.dependencies[0].depends_on_id.0,
            "vida-rf1-taskflow-core"
        );
    }

    #[test]
    fn snapshot_materializes_from_task_store_with_deterministic_order() {
        let mut store = InMemoryTaskStore::default();
        store.upsert_task(TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-runtime"),
            "runtime",
            IssueType::Task,
        ));
        store.upsert_task(TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-core"),
            "core",
            IssueType::Task,
        ));
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-state"),
            dependency_type: "blocks".into(),
        });
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-core"),
            dependency_type: "parent-child".into(),
        });

        let snapshot = snapshot_from_store(&store);

        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.tasks[0].id.0, "vida-rf1-taskflow-core");
        assert_eq!(snapshot.tasks[1].id.0, "vida-rf1-taskflow-runtime");
        assert_eq!(snapshot.dependencies.len(), 2);
        assert_eq!(
            snapshot.dependencies[0].depends_on_id.0,
            "vida-rf1-taskflow-core"
        );
        assert_eq!(
            snapshot.dependencies[1].depends_on_id.0,
            "vida-rf1-taskflow-state"
        );
    }

    #[test]
    fn restore_in_memory_store_round_trips_snapshot_rows() {
        let snapshot = TaskSnapshot {
            tasks: vec![
                TaskRecord::new(
                    TaskId::new("vida-rf1-taskflow-state"),
                    "state",
                    IssueType::Task,
                ),
                TaskRecord::new(
                    TaskId::new("vida-rf1-taskflow-runtime"),
                    "runtime",
                    IssueType::Task,
                ),
            ],
            dependencies: vec![DependencyEdge {
                issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
                depends_on_id: TaskId::new("vida-rf1-taskflow-state"),
                dependency_type: "blocks".into(),
            }],
        };

        let store = restore_in_memory_store(&snapshot);
        let runtime = store
            .get_task(&TaskId::new("vida-rf1-taskflow-runtime"))
            .expect("runtime task should restore");
        assert_eq!(runtime.title, "runtime");

        let dependencies = store.list_dependencies(&TaskId::new("vida-rf1-taskflow-runtime"));
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].depends_on_id.0, "vida-rf1-taskflow-state");
    }

    #[test]
    fn file_backed_store_export_and_import_round_trips() {
        let path = temp_snapshot_path();
        let mut store = InMemoryTaskStore::default();
        store.upsert_task(TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-state"),
            "state",
            IssueType::Task,
        ));
        store.upsert_task(TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-runtime"),
            "runtime",
            IssueType::Task,
        ));
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-state"),
            dependency_type: "blocks".into(),
        });

        write_store_snapshot(&path, &store).expect("store snapshot should write");
        let restored = read_snapshot_into_memory(&path).expect("store snapshot should restore");
        fs::remove_file(&path).expect("temp snapshot should be removed");

        let tasks = restored.list_tasks();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id.0, "vida-rf1-taskflow-runtime");
        assert_eq!(tasks[1].id.0, "vida-rf1-taskflow-state");

        let dependencies = restored.list_dependencies(&TaskId::new("vida-rf1-taskflow-runtime"));
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].depends_on_id.0, "vida-rf1-taskflow-state");
    }
}
