use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    VidaArtifactRef, VidaDomainEventEnvelope, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_state::{
    append_request_fingerprint, JournalAppendReceipt, JournalAppendRequest, JournalArtifactRecord,
    JournalEventRecord, JournalIdempotencyRecord, JournalIdempotencyState, JournalOutboxRecord,
    JournalOutboxState, JournalProjectionFailure, OperationalJournal, TaskflowStateError,
};

const SCHEMA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("journal_schema");
const EVENTS_BY_STREAM_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("events_by_stream");
const EVENTS_BY_GLOBAL_CURSOR_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("events_by_global_cursor");
const APPEND_IDEMPOTENCY_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("append_idempotency_by_key");
const IDEMPOTENCY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("idempotency_by_key");
const OUTBOX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("outbox_by_id");
const PROJECTION_CHECKPOINT_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("projection_checkpoint_by_id");
const PROJECTION_FAILURE_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("projection_failure_by_cursor");
const ARTIFACT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_by_ref");
const SCHEMA_VERSION_KEY: &str = "schema_version";
const SCHEMA_VERSION: &str = "1";

#[derive(Debug)]
pub struct RedbOperationalJournal {
    db: Database,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedbAppendIdempotencyRecord {
    request_fingerprint: String,
    receipt: JournalAppendReceipt,
}

impl RedbOperationalJournal {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let db = Database::create(path).map_err(storage_error)?;
        let journal = Self { db };
        journal.initialize_schema()?;
        Ok(journal)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, TaskflowStateError> {
        let db = Database::open(path).map_err(storage_error)?;
        Ok(Self { db })
    }

    fn initialize_schema(&self) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut schema = write.open_table(SCHEMA_TABLE).map_err(storage_error)?;
            schema
                .insert(SCHEMA_VERSION_KEY, SCHEMA_VERSION.as_bytes())
                .map_err(storage_error)?;
            let _ = write
                .open_table(EVENTS_BY_STREAM_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(EVENTS_BY_GLOBAL_CURSOR_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(APPEND_IDEMPOTENCY_TABLE)
                .map_err(storage_error)?;
            let _ = write.open_table(IDEMPOTENCY_TABLE).map_err(storage_error)?;
            let _ = write.open_table(OUTBOX_TABLE).map_err(storage_error)?;
            let _ = write
                .open_table(PROJECTION_CHECKPOINT_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(PROJECTION_FAILURE_TABLE)
                .map_err(storage_error)?;
            let _ = write.open_table(ARTIFACT_TABLE).map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn read_all<T: DeserializeOwned>(
        &self,
        table_definition: TableDefinition<&str, &[u8]>,
    ) -> Result<Vec<T>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(table_definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let mut records = Vec::new();
        for row in table.iter().map_err(storage_error)? {
            let (_, value) = row.map_err(storage_error)?;
            records.push(serde_json::from_slice(value.value()).map_err(storage_error)?);
        }
        Ok(records)
    }

    fn read_one<T: DeserializeOwned>(
        &self,
        table_definition: TableDefinition<&str, &[u8]>,
        key: &str,
    ) -> Result<Option<T>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(table_definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        table
            .get(key)
            .map_err(storage_error)?
            .map(|row| serde_json::from_slice(row.value()).map_err(storage_error))
            .transpose()
    }

    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Result<Option<VidaProjectionCheckpoint>, TaskflowStateError> {
        self.read_one(PROJECTION_CHECKPOINT_TABLE, &projection_id.0)
    }

    pub fn projection_failures(&self) -> Result<Vec<JournalProjectionFailure>, TaskflowStateError> {
        self.read_all(PROJECTION_FAILURE_TABLE)
    }

    pub fn artifact(
        &self,
        artifact_ref: &VidaArtifactRef,
    ) -> Result<Option<JournalArtifactRecord>, TaskflowStateError> {
        self.read_one(ARTIFACT_TABLE, &artifact_ref.0)
    }
}

impl OperationalJournal for RedbOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        let request_fingerprint = append_request_fingerprint(&request);
        let write = self.db.begin_write().map_err(storage_error)?;
        let result = {
            let mut events_by_stream = write
                .open_table(EVENTS_BY_STREAM_TABLE)
                .map_err(storage_error)?;
            let mut events_by_global = write
                .open_table(EVENTS_BY_GLOBAL_CURSOR_TABLE)
                .map_err(storage_error)?;
            let mut append_idempotency = write
                .open_table(APPEND_IDEMPOTENCY_TABLE)
                .map_err(storage_error)?;

            if let Some(row) = append_idempotency
                .get(request.idempotency_key.0.as_str())
                .map_err(storage_error)?
            {
                let record: RedbAppendIdempotencyRecord =
                    serde_json::from_slice(row.value()).map_err(storage_error)?;
                if record.request_fingerprint == request_fingerprint {
                    return Ok(record.receipt);
                }
                return Err(TaskflowStateError::IdempotencyConflict(
                    request.idempotency_key.0,
                ));
            }

            let stream_key = request.stream_id.0.clone();
            let mut actual_version = 0;
            for row in events_by_stream.iter().map_err(storage_error)? {
                let (_, value) = row.map_err(storage_error)?;
                let event: VidaDomainEventEnvelope =
                    serde_json::from_slice(value.value()).map_err(storage_error)?;
                if event.stream_id == request.stream_id {
                    actual_version += 1;
                }
            }
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

            let mut first_index: usize = 0;
            for row in events_by_global.iter().map_err(storage_error)? {
                row.map_err(storage_error)?;
                first_index += 1;
            }
            let event_count = request.events.len();
            let effect_intent_count = request.effect_intents.len();
            let mut next_global_index: usize = first_index;
            for event in request.events.clone() {
                next_global_index += 1;
                let cursor = VidaEventCursor(format!("global-{next_global_index}"));
                let stream_event_key =
                    format!("{}:{:020}", request.stream_id.0, event.stream_version.0);
                let global_event_key = format!("{next_global_index:020}");
                let event_payload = serde_json::to_vec(&event).map_err(storage_error)?;
                events_by_stream
                    .insert(stream_event_key.as_str(), event_payload.as_slice())
                    .map_err(storage_error)?;
                let global_record = JournalEventRecord {
                    global_cursor: cursor,
                    event,
                };
                let global_payload = serde_json::to_vec(&global_record).map_err(storage_error)?;
                events_by_global
                    .insert(global_event_key.as_str(), global_payload.as_slice())
                    .map_err(storage_error)?;
            }
            drop(events_by_stream);
            drop(events_by_global);

            let mut outbox = write.open_table(OUTBOX_TABLE).map_err(storage_error)?;
            let mut outbox_len = 0;
            for row in outbox.iter().map_err(storage_error)? {
                row.map_err(storage_error)?;
                outbox_len += 1;
            }
            for effect in request.effect_intents {
                outbox_len += 1;
                let record = JournalOutboxRecord {
                    outbox_id: VidaEventRef(format!("outbox-{outbox_len}")),
                    effect,
                    state: JournalOutboxState::Pending,
                };
                let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                outbox
                    .insert(record.outbox_id.0.as_str(), payload.as_slice())
                    .map_err(storage_error)?;
            }
            drop(outbox);

            let last_index = next_global_index.saturating_sub(1);
            let receipt = JournalAppendReceipt {
                stream_id: request.stream_id,
                first_global_cursor: if event_count > 0 {
                    Some(VidaEventCursor(format!("global-{}", first_index + 1)))
                } else {
                    None
                },
                last_global_cursor: if event_count > 0 {
                    Some(VidaEventCursor(format!("global-{last_index}")))
                } else {
                    None
                },
                stream_version: VidaStreamVersion(actual_version + event_count as u64),
                event_count,
                effect_intent_count,
            };
            let idempotency_record = RedbAppendIdempotencyRecord {
                request_fingerprint,
                receipt: receipt.clone(),
            };
            let payload = serde_json::to_vec(&idempotency_record).map_err(storage_error)?;
            append_idempotency
                .insert(request.idempotency_key.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
            Ok(receipt)
        }?;
        write.commit().map_err(storage_error)?;
        Ok(result)
    }

    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope> {
        let mut events: Vec<VidaDomainEventEnvelope> = self
            .read_all(EVENTS_BY_STREAM_TABLE)
            .unwrap_or_default()
            .into_iter()
            .filter(|event: &VidaDomainEventEnvelope| event.stream_id == *stream_id)
            .collect();
        events.sort_by_key(|event| event.stream_version.0);
        events
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        let Ok(records): Result<Vec<JournalEventRecord>, TaskflowStateError> =
            self.read_all(EVENTS_BY_GLOBAL_CURSOR_TABLE)
        else {
            return Vec::new();
        };
        let start = cursor
            .and_then(|cursor| {
                records
                    .iter()
                    .position(|record| record.global_cursor == *cursor)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        records.into_iter().skip(start).take(limit).collect()
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: taskflow_contracts::VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(IDEMPOTENCY_TABLE).map_err(storage_error)?;
            if table.get(key.0.as_str()).map_err(storage_error)?.is_some() {
                return Err(TaskflowStateError::IdempotencyConflict(key.0));
            }
            let record = JournalIdempotencyRecord {
                key: key.clone(),
                command_id,
                state: JournalIdempotencyState::Started,
                receipt_id: None,
                conflict_reason: None,
            };
            let payload = serde_json::to_vec(&record).map_err(storage_error)?;
            table
                .insert(key.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(IDEMPOTENCY_TABLE).map_err(storage_error)?;
            let mut record: JournalIdempotencyRecord = {
                let Some(row) = table.get(key.0.as_str()).map_err(storage_error)? else {
                    return Err(TaskflowStateError::IdempotencyConflict(key.0.clone()));
                };
                serde_json::from_slice(row.value()).map_err(storage_error)?
            };
            record.state = JournalIdempotencyState::Completed;
            record.receipt_id = Some(receipt_id);
            let payload = serde_json::to_vec(&record).map_err(storage_error)?;
            table
                .insert(key.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(IDEMPOTENCY_TABLE).map_err(storage_error)?;
            let mut record: JournalIdempotencyRecord = {
                let Some(row) = table.get(key.0.as_str()).map_err(storage_error)? else {
                    return Err(TaskflowStateError::IdempotencyConflict(key.0.clone()));
                };
                serde_json::from_slice(row.value()).map_err(storage_error)?
            };
            record.state = JournalIdempotencyState::Conflicted;
            record.conflict_reason = Some(reason);
            let payload = serde_json::to_vec(&record).map_err(storage_error)?;
            table
                .insert(key.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        let record = self.read_one(IDEMPOTENCY_TABLE, &key.0).ok().flatten()?;
        Some(Box::leak(Box::new(record)))
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        let result = (|| -> Result<Vec<JournalOutboxRecord>, TaskflowStateError> {
            let write = self.db.begin_write().map_err(storage_error)?;
            let mut claimed = Vec::new();
            {
                let mut table = write.open_table(OUTBOX_TABLE).map_err(storage_error)?;
                let mut records = Vec::new();
                for row in table.iter().map_err(storage_error)? {
                    let (key, value) = row.map_err(storage_error)?;
                    let record: JournalOutboxRecord =
                        serde_json::from_slice(value.value()).map_err(storage_error)?;
                    records.push((key.value().to_string(), record));
                }
                for (key, mut record) in records {
                    if !matches!(record.state, JournalOutboxState::Pending) {
                        continue;
                    }
                    record.state = JournalOutboxState::Claimed {
                        consumer_id: consumer_id.to_string(),
                    };
                    claimed.push(record.clone());
                    let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                    table
                        .insert(key.as_str(), payload.as_slice())
                        .map_err(storage_error)?;
                    if claimed.len() == limit {
                        break;
                    }
                }
            }
            write.commit().map_err(storage_error)?;
            Ok(claimed)
        })();
        result.unwrap_or_default()
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        self.update_outbox(outbox_id, |record| {
            record.state = JournalOutboxState::Succeeded;
        })
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.update_outbox(outbox_id, |record| {
            record.state = JournalOutboxState::Failed { reason };
        })
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        let _ = self.write_record(
            PROJECTION_CHECKPOINT_TABLE,
            &checkpoint.projection_id.0,
            &checkpoint,
        );
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        let key = format!(
            "{}:{}:{}",
            failure.projection_id.0, failure.stream_id.0, failure.error
        );
        let _ = self.write_record(PROJECTION_FAILURE_TABLE, &key, &failure);
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        let _ = self.write_record(ARTIFACT_TABLE, &artifact.artifact_ref.0, &artifact);
    }
}

impl RedbOperationalJournal {
    fn write_record<T: Serialize>(
        &self,
        table_definition: TableDefinition<&str, &[u8]>,
        key: &str,
        record: &T,
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(table_definition).map_err(storage_error)?;
            let payload = serde_json::to_vec(record).map_err(storage_error)?;
            table
                .insert(key, payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn update_outbox(
        &self,
        outbox_id: &VidaEventRef,
        update: impl FnOnce(&mut JournalOutboxRecord),
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(OUTBOX_TABLE).map_err(storage_error)?;
            let mut record: JournalOutboxRecord = {
                let Some(row) = table.get(outbox_id.0.as_str()).map_err(storage_error)? else {
                    return Err(TaskflowStateError::OutboxRecordNotFound(
                        outbox_id.0.clone(),
                    ));
                };
                serde_json::from_slice(row.value()).map_err(storage_error)?
            };
            update(&mut record);
            let payload = serde_json::to_vec(&record).map_err(storage_error)?;
            table
                .insert(outbox_id.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }
}

fn storage_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        RedbOperationalJournal, APPEND_IDEMPOTENCY_TABLE, EVENTS_BY_GLOBAL_CURSOR_TABLE,
        EVENTS_BY_STREAM_TABLE, OUTBOX_TABLE,
    };
    use redb::{ReadableDatabase, ReadableTable, TableDefinition};
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

        let read = reopened.db.begin_read().expect("read redb");
        assert_eq!(
            table_len(&read, EVENTS_BY_STREAM_TABLE),
            1,
            "stream events must be stored as normalized rows"
        );
        assert_eq!(
            table_len(&read, EVENTS_BY_GLOBAL_CURSOR_TABLE),
            1,
            "global events must be stored as normalized rows"
        );
        assert_eq!(
            table_len(&read, OUTBOX_TABLE),
            1,
            "outbox effects must be stored as normalized rows"
        );
        assert_eq!(
            table_len(&read, APPEND_IDEMPOTENCY_TABLE),
            1,
            "append idempotency must be stored as a keyed row"
        );
        let snapshot_table: TableDefinition<&str, &[u8]> =
            TableDefinition::new("operational_journal");
        assert!(
            matches!(
                read.open_table(snapshot_table),
                Err(redb::TableError::TableDoesNotExist(_))
            ),
            "normalized adapter must not keep the scaffold snapshot table"
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
        let mut stale_request = append_request(0, vec![event(2)], Vec::new());
        stale_request.idempotency_key = VidaIdempotencyKey("idem-2".to_string());
        let error = journal
            .append(stale_request)
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
    fn append_returns_cached_receipt_for_same_payload_retry() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        let first = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let mut retry_request = append_request(1, vec![event(1)], vec![effect("effect-1")]);
        retry_request.expected_stream_version = Some(VidaStreamVersion(1));
        let retry = journal
            .append(retry_request)
            .expect("same payload retry should return cached receipt");

        assert_eq!(retry, first);
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 1);
    }

    #[test]
    fn append_rejects_changed_payload_for_same_idempotency_key() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let error = journal
            .append(append_request(1, vec![event(2)], vec![effect("effect-2")]))
            .expect_err("changed payload with same idempotency key must fail");

        assert_eq!(
            error,
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

    fn table_len(
        read: &redb::ReadTransaction,
        table_definition: TableDefinition<&str, &[u8]>,
    ) -> usize {
        let table = read.open_table(table_definition).expect("open table");
        let mut len = 0;
        for row in table.iter().expect("iterate table") {
            row.expect("table row");
            len += 1;
        }
        len
    }
}
