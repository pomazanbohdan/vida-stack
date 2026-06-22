use std::collections::HashMap;
use std::path::Path;

use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    VidaArtifactRef, VidaDomainEventEnvelope, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_state::{
    JournalAppendReceipt, JournalAppendRequest, JournalArtifactRecord, JournalEventRecord,
    JournalIdempotencyRecord, JournalIdempotencyState, JournalOutboxRecord, JournalOutboxState,
    JournalProjectionFailure, OperationalJournal, TaskflowStateError,
};

const JOURNAL_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("operational_journal");
const SNAPSHOT_KEY: &str = "snapshot";

#[derive(Debug)]
pub struct RedbOperationalJournal {
    db: Database,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct JournalSnapshot {
    streams: HashMap<String, Vec<VidaDomainEventEnvelope>>,
    global_events: Vec<JournalEventRecord>,
    idempotency: HashMap<String, JournalIdempotencyRecord>,
    outbox: Vec<JournalOutboxRecord>,
    projection_checkpoints: HashMap<String, VidaProjectionCheckpoint>,
    projection_failures: Vec<JournalProjectionFailure>,
    artifacts: HashMap<String, JournalArtifactRecord>,
}

impl RedbOperationalJournal {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let db = Database::create(path).map_err(storage_error)?;
        let journal = Self { db };
        journal.with_snapshot(|_| Ok(()))?;
        Ok(journal)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let db = Database::open(path).map_err(storage_error)?;
        Ok(Self { db })
    }

    fn read_snapshot(&self) -> Result<JournalSnapshot, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(JOURNAL_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(JournalSnapshot::default()),
            Err(error) => return Err(storage_error(error)),
        };
        let Some(row) = table.get(SNAPSHOT_KEY).map_err(storage_error)? else {
            return Ok(JournalSnapshot::default());
        };
        serde_json::from_slice(row.value()).map_err(storage_error)
    }

    fn write_snapshot(&self, snapshot: &JournalSnapshot) -> Result<(), TaskflowStateError> {
        let payload = serde_json::to_vec(snapshot).map_err(storage_error)?;
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(JOURNAL_TABLE).map_err(storage_error)?;
            table
                .insert(SNAPSHOT_KEY, payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn with_snapshot<T>(
        &self,
        mutate: impl FnOnce(&mut JournalSnapshot) -> Result<T, TaskflowStateError>,
    ) -> Result<T, TaskflowStateError> {
        let mut snapshot = self.read_snapshot()?;
        let result = mutate(&mut snapshot)?;
        self.write_snapshot(&snapshot)?;
        Ok(result)
    }

    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Result<Option<VidaProjectionCheckpoint>, TaskflowStateError> {
        Ok(self
            .read_snapshot()?
            .projection_checkpoints
            .get(&projection_id.0)
            .cloned())
    }

    pub fn projection_failures(&self) -> Result<Vec<JournalProjectionFailure>, TaskflowStateError> {
        Ok(self.read_snapshot()?.projection_failures)
    }

    pub fn artifact(
        &self,
        artifact_ref: &VidaArtifactRef,
    ) -> Result<Option<JournalArtifactRecord>, TaskflowStateError> {
        Ok(self
            .read_snapshot()?
            .artifacts
            .get(&artifact_ref.0)
            .cloned())
    }
}

impl OperationalJournal for RedbOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            let stream_key = request.stream_id.0.clone();
            let actual_version = snapshot
                .streams
                .get(&stream_key)
                .map_or(0, |events| events.len() as u64);
            if request
                .expected_stream_version
                .as_ref()
                .map(|version| version.0)
                != Some(actual_version)
            {
                return Err(TaskflowStateError::StreamVersionConflict {
                    stream_id: stream_key,
                    expected: request
                        .expected_stream_version
                        .as_ref()
                        .map(|version| version.0),
                    actual: actual_version,
                });
            }

            let first_index = snapshot.global_events.len();
            let event_count = request.events.len();
            let effect_intent_count = request.effect_intents.len();
            let stream = snapshot
                .streams
                .entry(request.stream_id.0.clone())
                .or_default();
            for event in request.events {
                let cursor =
                    VidaEventCursor(format!("global-{}", snapshot.global_events.len() + 1));
                stream.push(event.clone());
                snapshot.global_events.push(JournalEventRecord {
                    global_cursor: cursor,
                    event,
                });
            }
            for effect in request.effect_intents {
                snapshot.outbox.push(JournalOutboxRecord {
                    outbox_id: VidaEventRef(format!("outbox-{}", snapshot.outbox.len() + 1)),
                    effect,
                    state: JournalOutboxState::Pending,
                });
            }
            let last_index = snapshot.global_events.len().saturating_sub(1);
            Ok(JournalAppendReceipt {
                stream_id: request.stream_id,
                first_global_cursor: snapshot
                    .global_events
                    .get(first_index)
                    .map(|record| record.global_cursor.clone()),
                last_global_cursor: snapshot
                    .global_events
                    .get(last_index)
                    .map(|record| record.global_cursor.clone()),
                stream_version: VidaStreamVersion(stream.len() as u64),
                event_count,
                effect_intent_count,
            })
        })
    }

    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope> {
        self.read_snapshot()
            .ok()
            .and_then(|snapshot| snapshot.streams.get(&stream_id.0).cloned())
            .unwrap_or_default()
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        let Ok(snapshot) = self.read_snapshot() else {
            return Vec::new();
        };
        let start = cursor
            .and_then(|cursor| {
                snapshot
                    .global_events
                    .iter()
                    .position(|record| record.global_cursor == *cursor)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        snapshot
            .global_events
            .into_iter()
            .skip(start)
            .take(limit)
            .collect()
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: taskflow_contracts::VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            if snapshot.idempotency.contains_key(&key.0) {
                return Err(TaskflowStateError::IdempotencyConflict(key.0));
            }
            snapshot.idempotency.insert(
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
        })
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            let record = snapshot
                .idempotency
                .get_mut(&key.0)
                .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
            record.state = JournalIdempotencyState::Completed;
            record.receipt_id = Some(receipt_id);
            Ok(())
        })
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            let record = snapshot
                .idempotency
                .get_mut(&key.0)
                .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
            record.state = JournalIdempotencyState::Conflicted;
            record.conflict_reason = Some(reason);
            Ok(())
        })
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        let record = self
            .read_snapshot()
            .ok()
            .and_then(|snapshot| snapshot.idempotency.get(&key.0).cloned())?;
        Some(Box::leak(Box::new(record)))
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        self.with_snapshot(|snapshot| {
            let mut claimed = Vec::new();
            for record in &mut snapshot.outbox {
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
            Ok(claimed)
        })
        .unwrap_or_default()
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            let record = snapshot
                .outbox
                .iter_mut()
                .find(|record| record.outbox_id == *outbox_id)
                .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
            record.state = JournalOutboxState::Succeeded;
            Ok(())
        })
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.with_snapshot(|snapshot| {
            let record = snapshot
                .outbox
                .iter_mut()
                .find(|record| record.outbox_id == *outbox_id)
                .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
            record.state = JournalOutboxState::Failed { reason };
            Ok(())
        })
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        let _ = self.with_snapshot(|snapshot| {
            snapshot
                .projection_checkpoints
                .insert(checkpoint.projection_id.0.clone(), checkpoint);
            Ok(())
        });
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        let _ = self.with_snapshot(|snapshot| {
            snapshot.projection_failures.push(failure);
            Ok(())
        });
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        let _ = self.with_snapshot(|snapshot| {
            snapshot
                .artifacts
                .insert(artifact.artifact_ref.0.clone(), artifact);
            Ok(())
        });
    }
}

fn storage_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::RedbOperationalJournal;
    use taskflow_contracts::{
        VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
        VidaEventRef, VidaIdempotencyKey, VidaOperation, VidaReceiptId, VidaSchemaId,
        VidaSchemaVersion, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
    };
    use taskflow_state::{
        JournalAppendRequest, JournalIdempotencyState, JournalOutboxState, OperationalJournal,
        TaskflowStateError,
    };
    use tempfile::tempdir;

    #[test]
    fn append_load_and_reopen_round_trip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        let receipt = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append should pass");

        assert_eq!(receipt.stream_version, VidaStreamVersion(1));
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        assert_eq!(
            reopened.load_stream(&VidaStreamRef("stream-1".to_string()))[0].event_id,
            VidaEventRef("event-1".to_string())
        );
    }

    #[test]
    fn append_rejects_stale_expected_version() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("first append should pass");
        let error = journal
            .append(append_request(0, vec![event(2)], Vec::new()))
            .expect_err("stale expected version should fail");

        assert_eq!(
            error,
            TaskflowStateError::StreamVersionConflict {
                stream_id: "stream-1".to_string(),
                expected: Some(0),
                actual: 1,
            }
        );
    }

    #[test]
    fn idempotency_and_outbox_survive_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .record_idempotency_started(
                VidaIdempotencyKey("idem-1".to_string()),
                VidaCommandRef("command-1".to_string()),
            )
            .expect("idempotency start should pass");
        journal
            .record_idempotency_completed(
                &VidaIdempotencyKey("idem-1".to_string()),
                VidaReceiptId("receipt-1".to_string()),
            )
            .expect("idempotency complete should pass");
        journal
            .append(append_request(0, Vec::new(), vec![effect("effect-1")]))
            .expect("effect append should pass");
        let claimed = journal.claim_outbox_batch("worker-1", 1);
        assert!(matches!(
            claimed[0].state,
            JournalOutboxState::Claimed { ref consumer_id } if consumer_id == "worker-1"
        ));
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        assert_eq!(
            reopened
                .idempotency_record(&VidaIdempotencyKey("idem-1".to_string()))
                .expect("idempotency record")
                .state,
            JournalIdempotencyState::Completed
        );
    }

    #[test]
    fn second_writer_is_rejected_on_windows_lock() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        let _held_write = journal.db.begin_write().expect("first writer");
        let second = RedbOperationalJournal::open(&path);

        if cfg!(windows) {
            assert!(
                second.is_err(),
                "redb should reject a second open while the first writer is held"
            );
        }
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
            schema_id: VidaSchemaId("schema.task.updated".to_string()),
            event_version: VidaSchemaVersion(1),
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
}
