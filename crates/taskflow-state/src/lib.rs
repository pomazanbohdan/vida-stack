use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    DependencyEdge, TaskRecord, VidaArtifactRef, VidaCommandRef, VidaDomainEventEnvelope,
    VidaEffectIntent, VidaEventCursor, VidaEventRef, VidaIdempotencyKey, VidaProjectionCheckpoint,
    VidaProjectionRef, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_core::TaskId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskflowStateError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("stream version conflict for {stream_id}: expected {expected:?}, actual {actual}")]
    StreamVersionConflict {
        stream_id: String,
        expected: Option<u64>,
        actual: u64,
    },
    #[error("idempotency conflict: {0}")]
    IdempotencyConflict(String),
    #[error("outbox record not found: {0}")]
    OutboxRecordNotFound(String),
    #[error("state storage error: {0}")]
    Storage(String),
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
        self.tasks.insert(task.id.as_str().to_string(), task);
    }

    fn get_task(&self, id: &TaskId) -> Result<&TaskRecord, TaskflowStateError> {
        self.tasks
            .get(id.as_str())
            .ok_or_else(|| TaskflowStateError::TaskNotFound(id.as_str().to_string()))
    }

    fn list_tasks(&self) -> Vec<&TaskRecord> {
        let mut rows: Vec<_> = self.tasks.values().collect();
        rows.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        rows
    }

    fn add_dependency(&mut self, edge: DependencyEdge) {
        self.dependencies.push(edge);
    }

    fn list_dependencies(&self, id: &TaskId) -> Vec<&DependencyEdge> {
        self.dependencies
            .iter()
            .filter(|edge| edge.issue_id.as_str() == id.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalAppendRequest {
    pub stream_id: VidaStreamRef,
    pub expected_stream_version: Option<VidaStreamVersion>,
    pub command_id: VidaCommandRef,
    pub idempotency_key: VidaIdempotencyKey,
    pub causation_id: Option<VidaCommandRef>,
    pub correlation_id: Option<String>,
    pub events: Vec<VidaDomainEventEnvelope>,
    pub effect_intents: Vec<VidaEffectIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalAppendReceipt {
    pub stream_id: VidaStreamRef,
    pub first_global_cursor: Option<VidaEventCursor>,
    pub last_global_cursor: Option<VidaEventCursor>,
    pub stream_version: VidaStreamVersion,
    pub event_count: usize,
    pub effect_intent_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEventRecord {
    pub global_cursor: VidaEventCursor,
    pub event: VidaDomainEventEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalIdempotencyState {
    Started,
    Completed,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalIdempotencyRecord {
    pub key: VidaIdempotencyKey,
    pub command_id: VidaCommandRef,
    pub state: JournalIdempotencyState,
    pub receipt_id: Option<VidaReceiptId>,
    pub conflict_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalOutboxState {
    Pending,
    Claimed { consumer_id: String },
    Succeeded,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalOutboxRecord {
    pub outbox_id: VidaEventRef,
    pub effect: VidaEffectIntent,
    pub state: JournalOutboxState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalProjectionFailure {
    pub projection_id: VidaProjectionRef,
    pub stream_id: VidaStreamRef,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalArtifactRecord {
    pub artifact_ref: VidaArtifactRef,
    pub content_hash: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JournalAppendIdempotencyRecord {
    request_fingerprint: String,
    receipt: JournalAppendReceipt,
}

fn append_request_fingerprint(request: &JournalAppendRequest) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        request.stream_id,
        request.command_id,
        request.idempotency_key,
        request.causation_id,
        request.correlation_id,
        request.events,
        request.effect_intents
    )
}

pub trait OperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError>;
    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope>;
    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord>;
    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError>;
    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError>;
    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError>;
    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord>;
    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord>;
    fn mark_outbox_succeeded(&mut self, outbox_id: &VidaEventRef)
    -> Result<(), TaskflowStateError>;
    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError>;
    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint);
    fn record_projection_failure(&mut self, failure: JournalProjectionFailure);
    fn index_artifact(&mut self, artifact: JournalArtifactRecord);
}

#[derive(Debug, Default)]
pub struct InMemoryOperationalJournal {
    streams: HashMap<String, Vec<VidaDomainEventEnvelope>>,
    global_events: Vec<JournalEventRecord>,
    idempotency: HashMap<String, JournalIdempotencyRecord>,
    append_idempotency: HashMap<String, JournalAppendIdempotencyRecord>,
    outbox: Vec<JournalOutboxRecord>,
    projection_checkpoints: HashMap<String, VidaProjectionCheckpoint>,
    projection_failures: Vec<JournalProjectionFailure>,
    artifacts: HashMap<String, JournalArtifactRecord>,
}

impl InMemoryOperationalJournal {
    #[must_use]
    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Option<&VidaProjectionCheckpoint> {
        self.projection_checkpoints.get(&projection_id.0)
    }

    #[must_use]
    pub fn projection_failures(&self) -> &[JournalProjectionFailure] {
        &self.projection_failures
    }

    #[must_use]
    pub fn artifact(&self, artifact_ref: &VidaArtifactRef) -> Option<&JournalArtifactRecord> {
        self.artifacts.get(&artifact_ref.0)
    }
}

impl OperationalJournal for InMemoryOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        let idempotency_key = request.idempotency_key.clone();
        let command_id = request.command_id.clone();
        let request_fingerprint = append_request_fingerprint(&request);
        if let Some(record) = self.append_idempotency.get(&idempotency_key.0) {
            if record.request_fingerprint == request_fingerprint {
                return Ok(record.receipt.clone());
            }
            return Err(TaskflowStateError::IdempotencyConflict(idempotency_key.0));
        }

        let key = request.stream_id.0.clone();
        let stream = self.streams.entry(key.clone()).or_default();
        let actual_version = stream.len() as u64;
        if request
            .expected_stream_version
            .as_ref()
            .map(|version| version.0)
            != Some(actual_version)
        {
            return Err(TaskflowStateError::StreamVersionConflict {
                stream_id: key,
                expected: request
                    .expected_stream_version
                    .as_ref()
                    .map(|version| version.0),
                actual: actual_version,
            });
        }

        let first_index = self.global_events.len();
        let event_count = request.events.len();
        let effect_intent_count = request.effect_intents.len();
        for event in request.events {
            let cursor = VidaEventCursor(format!("global-{}", self.global_events.len() + 1));
            stream.push(event.clone());
            self.global_events.push(JournalEventRecord {
                global_cursor: cursor,
                event,
            });
        }
        for effect in request.effect_intents {
            self.outbox.push(JournalOutboxRecord {
                outbox_id: VidaEventRef(format!("outbox-{}", self.outbox.len() + 1)),
                effect,
                state: JournalOutboxState::Pending,
            });
        }

        let last_index = self.global_events.len().saturating_sub(1);
        let receipt = JournalAppendReceipt {
            stream_id: request.stream_id,
            first_global_cursor: self
                .global_events
                .get(first_index)
                .map(|record| record.global_cursor.clone()),
            last_global_cursor: self
                .global_events
                .get(last_index)
                .map(|record| record.global_cursor.clone()),
            stream_version: VidaStreamVersion(stream.len() as u64),
            event_count,
            effect_intent_count,
        };
        self.idempotency.insert(
            idempotency_key.0.clone(),
            JournalIdempotencyRecord {
                key: idempotency_key.clone(),
                command_id: command_id.clone(),
                state: JournalIdempotencyState::Completed,
                receipt_id: Some(VidaReceiptId(format!("append:{}", command_id.0))),
                conflict_reason: None,
            },
        );
        self.append_idempotency.insert(
            idempotency_key.0,
            JournalAppendIdempotencyRecord {
                request_fingerprint,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope> {
        self.streams.get(&stream_id.0).cloned().unwrap_or_default()
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        let start = cursor
            .and_then(|cursor| {
                self.global_events
                    .iter()
                    .position(|record| record.global_cursor == *cursor)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        self.global_events
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect()
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        if self.idempotency.contains_key(&key.0) {
            return Err(TaskflowStateError::IdempotencyConflict(key.0));
        }
        self.idempotency.insert(
            key.0.clone(),
            JournalIdempotencyRecord {
                key,
                command_id,
                state: JournalIdempotencyState::Started,
                receipt_id: None,
                conflict_reason: None,
            },
        );
        Ok(())
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .idempotency
            .get_mut(&key.0)
            .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
        record.state = JournalIdempotencyState::Completed;
        record.receipt_id = Some(receipt_id);
        Ok(())
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .idempotency
            .get_mut(&key.0)
            .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
        record.state = JournalIdempotencyState::Conflicted;
        record.conflict_reason = Some(reason);
        Ok(())
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        self.idempotency.get(&key.0)
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        let mut claimed = Vec::new();
        for record in self.outbox.iter_mut() {
            if !matches!(record.state, JournalOutboxState::Pending) {
                continue;
            }
            record.state = JournalOutboxState::Claimed {
                consumer_id: consumer_id.to_string(),
            };
            claimed.push(record.clone());
            if claimed.len() == limit {
                break;
            }
        }
        claimed
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .outbox
            .iter_mut()
            .find(|record| record.outbox_id == *outbox_id)
            .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
        record.state = JournalOutboxState::Succeeded;
        Ok(())
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .outbox
            .iter_mut()
            .find(|record| record.outbox_id == *outbox_id)
            .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
        record.state = JournalOutboxState::Failed { reason };
        Ok(())
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        self.projection_checkpoints
            .insert(checkpoint.projection_id.0.clone(), checkpoint);
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        self.projection_failures.push(failure);
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        self.artifacts
            .insert(artifact.artifact_ref.0.clone(), artifact);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryOperationalJournal, InMemoryTaskStore, JournalAppendRequest, JournalArtifactRecord,
        JournalIdempotencyState, JournalOutboxState, JournalProjectionFailure, OperationalJournal,
        TaskStore, TaskflowStateError,
    };
    use taskflow_contracts::{
        DependencyEdge, TaskRecord, VidaAggregateRef, VidaArtifactRef, VidaCommandRef,
        VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef, VidaEventCursor, VidaEventRef,
        VidaIdempotencyKey, VidaOperation, VidaProjectionCheckpoint, VidaProjectionRef,
        VidaReceiptId, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
    };
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
        assert_eq!(rows[0].depends_on_id.as_str(), "vida-rf1-taskflow-core");
    }

    #[test]
    fn operational_journal_appends_and_reads_with_expected_version() {
        let mut journal = InMemoryOperationalJournal::default();
        let request = append_request(0, vec![event(1)], vec![effect("effect-1")]);

        let receipt = journal.append(request).expect("append should pass");

        assert_eq!(receipt.stream_id, VidaStreamRef("stream-1".to_string()));
        assert_eq!(receipt.stream_version, VidaStreamVersion(1));
        assert_eq!(receipt.event_count, 1);
        assert_eq!(receipt.effect_intent_count, 1);
        assert_eq!(
            receipt.first_global_cursor,
            Some(VidaEventCursor("global-1".to_string()))
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.read_global_after(None, 10).len(), 1);

        let conflict = journal
            .append(append_request(0, vec![event(2)], Vec::new()))
            .expect_err("changed payload with duplicate idempotency key must fail");
        assert_eq!(
            conflict,
            TaskflowStateError::IdempotencyConflict("idem-1".to_string())
        );
    }

    #[test]
    fn operational_journal_returns_cached_receipt_for_same_payload_retry() {
        let mut journal = InMemoryOperationalJournal::default();

        let receipt = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let exact_retry = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("same payload retry should return cached receipt");
        let updated_version_retry = journal
            .append(append_request(1, vec![event(1)], vec![effect("effect-1")]))
            .expect("same payload retry with updated precondition should return cached receipt");

        assert_eq!(exact_retry, receipt);
        assert_eq!(updated_version_retry, receipt);
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 1);
    }

    #[test]
    fn operational_journal_rejects_changed_payload_for_same_idempotency_key() {
        let mut journal = InMemoryOperationalJournal::default();

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let replay = journal
            .append(append_request(1, vec![event(2)], vec![effect("effect-2")]))
            .expect_err("changed payload with same idempotency key must fail");

        assert_eq!(
            replay,
            TaskflowStateError::IdempotencyConflict("idem-1".to_string())
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 1);
    }

    #[test]
    fn operational_journal_tracks_idempotency_lifecycle() {
        let mut journal = InMemoryOperationalJournal::default();
        let key = VidaIdempotencyKey("idem-1".to_string());
        journal
            .record_idempotency_started(key.clone(), VidaCommandRef("command-1".to_string()))
            .expect("start should pass");
        journal
            .record_idempotency_completed(&key, VidaReceiptId("receipt-1".to_string()))
            .expect("complete should pass");

        let record = journal
            .idempotency_record(&key)
            .expect("idempotency record should exist");
        assert_eq!(record.state, JournalIdempotencyState::Completed);
        assert_eq!(
            record.receipt_id,
            Some(VidaReceiptId("receipt-1".to_string()))
        );
        assert!(
            journal
                .record_idempotency_started(key, VidaCommandRef("command-1".to_string()))
                .is_err()
        );
    }

    #[test]
    fn operational_journal_claims_outbox_and_records_projection_artifacts() {
        let mut journal = InMemoryOperationalJournal::default();
        journal
            .append(append_request(0, Vec::new(), vec![effect("effect-1")]))
            .expect("append should pass");

        let claimed = journal.claim_outbox_batch("worker-1", 1);
        assert_eq!(claimed.len(), 1);
        assert!(matches!(
            claimed[0].state,
            JournalOutboxState::Claimed { ref consumer_id } if consumer_id == "worker-1"
        ));
        journal
            .mark_outbox_succeeded(&claimed[0].outbox_id)
            .expect("mark succeeded should pass");

        let checkpoint = projection_checkpoint(1);
        journal.record_projection_checkpoint(checkpoint.clone());
        assert_eq!(
            journal.projection_checkpoint(&VidaProjectionRef("projection-1".to_string())),
            Some(&checkpoint)
        );

        let failure = JournalProjectionFailure {
            projection_id: VidaProjectionRef("projection-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            error: "projection failed".to_string(),
        };
        journal.record_projection_failure(failure.clone());
        assert_eq!(journal.projection_failures(), &[failure]);

        let artifact = JournalArtifactRecord {
            artifact_ref: VidaArtifactRef("artifact-1".to_string()),
            content_hash: "sha256:abc".to_string(),
            path: "artifacts/artifact-1.json".to_string(),
        };
        journal.index_artifact(artifact.clone());
        assert_eq!(
            journal.artifact(&VidaArtifactRef("artifact-1".to_string())),
            Some(&artifact)
        );
    }

    fn append_request(
        expected_stream_version: u64,
        events: Vec<VidaDomainEventEnvelope>,
        effect_intents: Vec<VidaEffectIntent>,
    ) -> JournalAppendRequest {
        JournalAppendRequest {
            stream_id: VidaStreamRef("stream-1".to_string()),
            expected_stream_version: Some(VidaStreamVersion(expected_stream_version)),
            command_id: VidaCommandRef("command-1".to_string()),
            idempotency_key: VidaIdempotencyKey("idem-1".to_string()),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            correlation_id: Some("correlation-1".to_string()),
            events,
            effect_intents,
        }
    }

    fn event(stream_version: u64) -> VidaDomainEventEnvelope {
        VidaDomainEventEnvelope {
            schema_id: taskflow_contracts::VidaSchemaId("schema.task.updated".to_string()),
            event_version: taskflow_contracts::VidaSchemaVersion(1),
            event_id: VidaEventRef(format!("event-{stream_version}")),
            command_id: Some(VidaCommandRef("command-1".to_string())),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            stream_id: VidaStreamRef("stream-1".to_string()),
            stream_version: VidaStreamVersion(stream_version),
            aggregate_id: VidaAggregateRef("task-1".to_string()),
            occurred_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
            payload: serde_json::json!({ "stream_version": stream_version }),
            trace: serde_json::json!({ "correlation_id": "correlation-1" }),
        }
    }

    fn effect(effect_id: &str) -> VidaEffectIntent {
        VidaEffectIntent {
            effect_id: VidaEffectRef(effect_id.to_string()),
            operation: VidaOperation("vida.effect.dispatch".to_string()),
            command_id: VidaCommandRef("command-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            payload: serde_json::json!({ "effect_id": effect_id }),
        }
    }

    fn projection_checkpoint(stream_version: u64) -> VidaProjectionCheckpoint {
        VidaProjectionCheckpoint {
            projection_id: VidaProjectionRef("projection-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            event_cursor: VidaEventCursor(format!("global-{stream_version}")),
            stream_version: VidaStreamVersion(stream_version),
            updated_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
        }
    }
}
