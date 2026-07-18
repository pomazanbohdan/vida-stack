#[cfg(test)]
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taskflow_contracts::{DependencyEdge, TaskRecord};
use taskflow_contracts::{
    VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEventCursor, VidaEventRef,
    VidaIdempotencyKey, VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId,
};
use taskflow_state::{
    InMemoryOperationalJournal, InMemoryTaskStore, JournalAggregateSnapshotRecord,
    JournalAppendReceipt, JournalAppendRequest, JournalArtifactRecord, JournalEventRecord,
    JournalIdempotencyRecord, JournalOutboxRecord, JournalProjectionFailure, OperationalJournal,
    TaskStore, TaskflowStateError,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSnapshot {
    pub tasks: Vec<TaskRecord>,
    pub dependencies: Vec<DependencyEdge>,
}

/// Durable filesystem adapter for the storage-neutral operational journal contract.
///
/// This is a compatibility adapter and test surface; canonical TaskFlow authority remains
/// SurrealKV. The serialized in-memory journal is intentionally kept behind the same port so
/// ordering, replay, idempotency, checkpoint, and outbox semantics are shared with other adapters.
#[derive(Debug)]
pub struct FileOperationalJournal {
    path: PathBuf,
    journal: InMemoryOperationalJournal,
    persistence_error: Option<String>,
}

#[cfg(test)]
thread_local! {
    static INJECT_PARTIAL_WRITE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
fn arm_partial_write_injection() {
    INJECT_PARTIAL_WRITE.with(|flag| flag.set(true));
}

#[cfg(test)]
fn take_partial_write_injection() -> bool {
    INJECT_PARTIAL_WRITE.with(|flag| flag.replace(false))
}

impl FileOperationalJournal {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            return Self::open(path);
        }
        let journal = Self {
            path,
            journal: InMemoryOperationalJournal::default(),
            persistence_error: None,
        };
        journal.persist()?;
        Ok(journal)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let path = path.as_ref().to_path_buf();
        let payload = fs::read(&path).map_err(storage_error)?;
        let journal = match decode_journal_payload(&payload, "primary") {
            Ok(journal) => journal,
            Err(primary_error) => {
                let backup_path = path.with_extension("bak");
                let backup_payload = fs::read(&backup_path).map_err(|backup_error| {
                    TaskflowStateError::PayloadDecode(format!(
                        "{primary_error}; filesystem recovery backup unavailable: {backup_error}"
                    ))
                })?;
                let journal = decode_journal_payload(&backup_payload, "recovery backup")?;
                fs::copy(&backup_path, &path).map_err(storage_error)?;
                journal
            }
        };
        Ok(Self {
            path,
            journal,
            persistence_error: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Option<VidaProjectionCheckpoint> {
        self.journal.projection_checkpoint(projection_id).cloned()
    }

    #[must_use]
    pub fn persistence_error(&self) -> Option<&str> {
        self.persistence_error.as_deref()
    }

    pub fn ensure_persistence_healthy(&self) -> Result<(), TaskflowStateError> {
        if let Some(error) = &self.persistence_error {
            return Err(TaskflowStateError::Storage(error.clone()));
        }
        Ok(())
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("bak")
    }

    fn remember_persistence_error(&mut self, error: TaskflowStateError) {
        self.persistence_error = Some(error.to_string());
    }

    fn persist_or_mark_unhealthy(&mut self) -> bool {
        match self.persist() {
            Ok(()) => true,
            Err(error) => {
                self.remember_persistence_error(error);
                false
            }
        }
    }

    fn persist(&self) -> Result<(), TaskflowStateError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(storage_error)?;
        }
        let payload = serde_json::to_vec_pretty(&self.journal).map_err(|error| {
            TaskflowStateError::Storage(format!("filesystem journal serialization: {error}"))
        })?;
        if self.path.exists() {
            fs::copy(&self.path, self.backup_path()).map_err(storage_error)?;
        }
        #[cfg(test)]
        if take_partial_write_injection() {
            let partial_len = (payload.len() / 2).max(1);
            fs::write(&self.path, &payload[..partial_len]).map_err(storage_error)?;
            return Err(TaskflowStateError::Storage(
                "injected partial write interruption".to_string(),
            ));
        }
        fs::write(&self.path, payload).map_err(storage_error)
    }

    fn persist_after<T>(
        &mut self,
        result: Result<T, TaskflowStateError>,
    ) -> Result<T, TaskflowStateError> {
        let persistence = self.persist();
        match result {
            Ok(value) => match persistence {
                Ok(()) => Ok(value),
                Err(error) => {
                    self.remember_persistence_error(error);
                    Err(TaskflowStateError::Storage(
                        self.persistence_error
                            .clone()
                            .unwrap_or_else(|| "filesystem persistence failed".to_string()),
                    ))
                }
            },
            Err(operation_error) => match persistence {
                Ok(()) => Err(operation_error),
                Err(persistence_error) => {
                    let combined =
                        format!("{operation_error}; persistence failed: {persistence_error}");
                    self.remember_persistence_error(TaskflowStateError::Storage(combined.clone()));
                    Err(TaskflowStateError::Storage(combined))
                }
            },
        }
    }
}

fn decode_journal_payload(
    payload: &[u8],
    source: &str,
) -> Result<InMemoryOperationalJournal, TaskflowStateError> {
    serde_json::from_slice(payload).map_err(|error| {
        TaskflowStateError::PayloadDecode(format!("filesystem {source} journal payload: {error}"))
    })
}

fn storage_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(error.to_string())
}

impl OperationalJournal for FileOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.append(request);
        self.persist_after(result)
    }

    fn load_stream(
        &self,
        stream_id: &taskflow_contracts::VidaStreamRef,
    ) -> Vec<VidaDomainEventEnvelope> {
        self.journal.load_stream(stream_id)
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        self.journal.read_global_after(cursor, limit)
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.record_idempotency_started(key, command_id);
        self.persist_after(result)
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.record_idempotency_completed(key, receipt_id);
        self.persist_after(result)
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.record_idempotency_conflicted(key, reason);
        self.persist_after(result)
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        self.journal.idempotency_record(key)
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        if self.persistence_error.is_some() {
            return Vec::new();
        }
        let claimed = self.journal.claim_outbox_batch(consumer_id, limit);
        if self.persist_or_mark_unhealthy() {
            claimed
        } else {
            Vec::new()
        }
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.mark_outbox_succeeded(outbox_id);
        self.persist_after(result)
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.ensure_persistence_healthy()?;
        let result = self.journal.mark_outbox_failed(outbox_id, reason);
        self.persist_after(result)
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        if self.persistence_error.is_some() {
            return;
        }
        self.journal.record_projection_checkpoint(checkpoint);
        self.persist_or_mark_unhealthy();
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        if self.persistence_error.is_some() {
            return;
        }
        self.journal.record_projection_failure(failure);
        self.persist_or_mark_unhealthy();
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        if self.persistence_error.is_some() {
            return;
        }
        self.journal.index_artifact(artifact);
        self.persist_or_mark_unhealthy();
    }

    fn record_aggregate_snapshot(&mut self, snapshot: JournalAggregateSnapshotRecord) {
        if self.persistence_error.is_some() {
            return;
        }
        self.journal.record_aggregate_snapshot(snapshot);
        self.persist_or_mark_unhealthy();
    }

    fn aggregate_snapshot(
        &self,
        aggregate_id: &VidaAggregateRef,
    ) -> Option<JournalAggregateSnapshotRecord> {
        self.journal.aggregate_snapshot(aggregate_id)
    }
}

#[must_use]
pub fn snapshot_from_store(store: &impl TaskStore) -> TaskSnapshot {
    let mut tasks: Vec<TaskRecord> = store.list_tasks().into_iter().cloned().collect();
    tasks.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));

    let mut dependencies: Vec<DependencyEdge> = tasks
        .iter()
        .flat_map(|task| store.list_dependencies(&task.id).into_iter().cloned())
        .collect();
    dependencies.sort_by(|left, right| {
        left.issue_id
            .as_str()
            .cmp(right.issue_id.as_str())
            .then_with(|| {
                left.depends_on_id
                    .as_str()
                    .cmp(right.depends_on_id.as_str())
            })
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
        FileOperationalJournal, TaskSnapshot, read_snapshot, read_snapshot_into_memory,
        restore_in_memory_store, snapshot_from_store, write_snapshot, write_store_snapshot,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use taskflow_contracts::{
        DependencyEdge, TaskRecord, VidaAggregateRef, VidaArtifactRef, VidaEventCursor,
        VidaProjectionCheckpoint, VidaProjectionRef, VidaSchemaId, VidaSchemaVersion,
        VidaStreamRef, VidaStreamVersion, VidaTimestamp,
    };
    use taskflow_core::{IssueType, TaskId};
    use taskflow_state::{
        InMemoryTaskStore, JournalAggregateSnapshotRecord, JournalArtifactRecord,
        JournalProjectionFailure, OperationalJournal, TaskStore, TaskflowStateError,
    };
    use vida_test_support::state_conformance::{
        StateAdapterFactory, run_state_adapter_conformance,
    };

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
            loaded.dependencies[0].depends_on_id.as_str(),
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
        assert_eq!(snapshot.tasks[0].id.as_str(), "vida-rf1-taskflow-core");
        assert_eq!(snapshot.tasks[1].id.as_str(), "vida-rf1-taskflow-runtime");
        assert_eq!(snapshot.dependencies.len(), 2);
        assert_eq!(
            snapshot.dependencies[0].depends_on_id.as_str(),
            "vida-rf1-taskflow-core"
        );
        assert_eq!(
            snapshot.dependencies[1].depends_on_id.as_str(),
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
        assert_eq!(
            dependencies[0].depends_on_id.as_str(),
            "vida-rf1-taskflow-state"
        );
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
        assert_eq!(tasks[0].id.as_str(), "vida-rf1-taskflow-runtime");
        assert_eq!(tasks[1].id.as_str(), "vida-rf1-taskflow-state");

        let dependencies = restored.list_dependencies(&TaskId::new("vida-rf1-taskflow-runtime"));
        assert_eq!(dependencies.len(), 1);
        assert_eq!(
            dependencies[0].depends_on_id.as_str(),
            "vida-rf1-taskflow-state"
        );
    }

    struct FileJournalFactory {
        path: std::path::PathBuf,
        recovery_path: Option<std::path::PathBuf>,
        generation: u64,
    }

    impl FileJournalFactory {
        fn new() -> Self {
            Self {
                path: temp_snapshot_path().with_extension("journal.json"),
                recovery_path: None,
                generation: 0,
            }
        }
    }

    impl StateAdapterFactory for FileJournalFactory {
        fn backend_name(&self) -> &str {
            "filesystem"
        }

        fn fresh(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError> {
            self.generation += 1;
            self.path = self
                .path
                .with_file_name(format!("state-adapter-{}.journal.json", self.generation));
            let _ = fs::remove_file(&self.path);
            let journal = FileOperationalJournal::create(&self.path)?;
            if self.recovery_path.is_none() {
                self.recovery_path = Some(self.path.clone());
            }
            Ok(Box::new(journal))
        }

        fn reopen(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError> {
            let path = self.recovery_path.as_ref().ok_or_else(|| {
                TaskflowStateError::Storage(
                    "reopen requires the first committed journal".to_string(),
                )
            })?;
            Ok(Box::new(FileOperationalJournal::open(path)?))
        }

        fn supports_restart_recovery(&self) -> bool {
            true
        }

        fn supports_checkpoint_recovery(&self) -> bool {
            true
        }

        fn reopened_checkpoint(
            &mut self,
            projection_id: &VidaProjectionRef,
        ) -> Result<Option<VidaProjectionCheckpoint>, TaskflowStateError> {
            let path = self.recovery_path.as_ref().ok_or_else(|| {
                TaskflowStateError::Storage(
                    "checkpoint recovery requires the first committed journal".to_string(),
                )
            })?;
            Ok(FileOperationalJournal::open(path)?.projection_checkpoint(projection_id))
        }

        fn inject_partial_write_once(&mut self) -> bool {
            super::arm_partial_write_injection();
            true
        }
    }

    #[test]
    fn filesystem_adapter_passes_shared_state_corpus_and_reopen() {
        let mut factory = FileJournalFactory::new();
        let report = run_state_adapter_conformance(&mut factory)
            .expect("filesystem adapter should pass the shared corpus");

        assert_eq!(report.backend, "filesystem");
        assert_eq!(report.checks.len(), 9);
        assert!(report.restart_recovered);
        assert!(report.checkpoint_recovered);
        assert!(report.partial_write_recovered);
        let _ = fs::remove_file(&factory.path);
        let _ = fs::remove_file(factory.path.with_extension("bak"));
    }

    fn assert_infallible_persistence_failure(operation: impl FnOnce(&mut FileOperationalJournal)) {
        let path = temp_snapshot_path().with_extension("journal.json");
        let mut journal = FileOperationalJournal::create(&path).expect("journal should create");
        super::arm_partial_write_injection();
        operation(&mut journal);

        assert!(
            journal
                .persistence_error()
                .is_some_and(|error| error.contains("injected partial write interruption"))
        );
        assert!(journal.ensure_persistence_healthy().is_err());

        drop(journal);
        let reopened = FileOperationalJournal::open(&path).expect("backup should recover");
        assert!(reopened.persistence_error().is_none());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("bak"));
    }

    #[test]
    fn filesystem_infallible_persistence_methods_fail_closed_without_panicking() {
        assert_infallible_persistence_failure(|journal| {
            journal.claim_outbox_batch("fail-closed-consumer", 1);
        });
        assert_infallible_persistence_failure(|journal| {
            journal.record_projection_checkpoint(VidaProjectionCheckpoint {
                projection_id: VidaProjectionRef("fail-closed-projection".to_string()),
                stream_id: VidaStreamRef("fail-closed-stream".to_string()),
                event_cursor: VidaEventCursor("global-1".to_string()),
                stream_version: VidaStreamVersion(1),
                updated_at: VidaTimestamp("fail-closed-time".to_string()),
            });
        });
        assert_infallible_persistence_failure(|journal| {
            journal.record_projection_failure(JournalProjectionFailure {
                projection_id: VidaProjectionRef("fail-closed-projection".to_string()),
                stream_id: VidaStreamRef("fail-closed-stream".to_string()),
                error: "injected failure".to_string(),
            });
        });
        assert_infallible_persistence_failure(|journal| {
            journal.index_artifact(JournalArtifactRecord {
                artifact_ref: VidaArtifactRef("fail-closed-artifact".to_string()),
                content_hash: "hash".to_string(),
                path: "artifact.json".to_string(),
            });
        });
        assert_infallible_persistence_failure(|journal| {
            journal.record_aggregate_snapshot(JournalAggregateSnapshotRecord {
                aggregate_id: VidaAggregateRef("fail-closed-aggregate".to_string()),
                schema_id: VidaSchemaId("fail-closed-schema".to_string()),
                schema_version: VidaSchemaVersion(1),
                stream_id: VidaStreamRef("fail-closed-stream".to_string()),
                stream_version: VidaStreamVersion(1),
                payload: serde_json::json!({"state": "fail-closed"}),
                replay_hash: "hash".to_string(),
            });
        });
    }
}
