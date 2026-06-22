use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    VidaArtifactRef, VidaDomainEventEnvelope, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_state::{
    JournalAppendReceipt, JournalAppendRequest, JournalArtifactRecord, JournalEventRecord,
    JournalIdempotencyRecord, JournalIdempotencyState, JournalOutboxRecord, JournalOutboxState,
    JournalProjectionFailure, OperationalJournal, TaskflowStateError, append_request_fingerprint,
    validate_append_event_streams,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedbJournalHealth {
    pub schema_version: String,
    pub stream_event_count: usize,
    pub global_event_count: usize,
    pub append_idempotency_count: usize,
    pub idempotency_count: usize,
    pub outbox_pending_count: usize,
    pub outbox_claimed_count: usize,
    pub outbox_succeeded_count: usize,
    pub outbox_failed_count: usize,
    pub projection_checkpoint_count: usize,
    pub projection_failure_count: usize,
    pub artifact_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedbJournalBlocker {
    pub code: &'static str,
    pub next_action: String,
}

pub const REDB_SINGLE_WRITER_BLOCKER_CODE: &str = "redb_single_writer_lock_held";
pub const REDB_CORRUPT_PAYLOAD_BLOCKER_CODE: &str = "redb_journal_payload_corrupt";

pub fn classify_redb_journal_error(
    error: &TaskflowStateError,
    journal_path: impl AsRef<Path>,
) -> Option<RedbJournalBlocker> {
    let TaskflowStateError::Storage(reason) = error else {
        return None;
    };
    let journal_path = journal_path.as_ref().display();
    let reason_lower = reason.to_ascii_lowercase();
    if reason_lower.contains("lock")
        || reason_lower.contains("being used by another process")
        || reason_lower.contains("database is already open")
    {
        return Some(RedbJournalBlocker {
            code: REDB_SINGLE_WRITER_BLOCKER_CODE,
            next_action: format!(
                "Wait for or stop the process holding the redb journal writer lock, then retry the journal operation for `{journal_path}`."
            ),
        });
    }
    if reason_lower.contains("corrupt")
        || reason_lower.contains("json")
        || reason_lower.contains("expected")
        || reason_lower.contains("eof")
    {
        return Some(RedbJournalBlocker {
            code: REDB_CORRUPT_PAYLOAD_BLOCKER_CODE,
            next_action: format!(
                "Quarantine the redb journal at `{journal_path}`, restore from the last trusted snapshot, or rebuild projections from verified event cursor evidence."
            ),
        });
    }
    None
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
            records.push(serde_json::from_slice(value.value()).map_err(payload_decode_error)?);
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
            .map(|row| serde_json::from_slice(row.value()).map_err(payload_decode_error))
            .transpose()
    }

    fn schema_version(&self) -> Result<String, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = read.open_table(SCHEMA_TABLE).map_err(storage_error)?;
        let Some(row) = table.get(SCHEMA_VERSION_KEY).map_err(storage_error)? else {
            return Err(TaskflowStateError::Storage(
                "redb journal schema version is missing".to_string(),
            ));
        };
        std::str::from_utf8(row.value())
            .map(str::to_string)
            .map_err(storage_error)
    }

    pub fn health_status(&self) -> Result<RedbJournalHealth, TaskflowStateError> {
        let schema_version = self.schema_version()?;
        if schema_version != SCHEMA_VERSION {
            return Err(TaskflowStateError::Storage(format!(
                "redb journal schema version mismatch: expected={SCHEMA_VERSION} actual={schema_version}"
            )));
        }
        let stream_events: Vec<VidaDomainEventEnvelope> = self.read_all(EVENTS_BY_STREAM_TABLE)?;
        let global_events: Vec<JournalEventRecord> =
            self.read_all(EVENTS_BY_GLOBAL_CURSOR_TABLE)?;
        if stream_events.len() != global_events.len() {
            return Err(TaskflowStateError::Storage(format!(
                "redb journal stream/global event count mismatch: stream={} global={}",
                stream_events.len(),
                global_events.len()
            )));
        }
        for record in &global_events {
            let event_id = &record.event.event_id;
            if !stream_events
                .iter()
                .any(|event| event.event_id == *event_id)
            {
                return Err(TaskflowStateError::Storage(format!(
                    "redb journal global event `{}` is missing from stream index",
                    event_id.0
                )));
            }
        }
        let mut seen_event_ids = std::collections::BTreeSet::new();
        let mut expected_cursor = 1usize;
        for record in &global_events {
            let expected = VidaEventCursor(format!("global-{expected_cursor}"));
            if record.global_cursor != expected {
                return Err(TaskflowStateError::Storage(format!(
                    "redb journal global cursor gap: expected={} actual={}",
                    expected.0, record.global_cursor.0
                )));
            }
            if !seen_event_ids.insert(record.event.event_id.0.clone()) {
                return Err(TaskflowStateError::Storage(format!(
                    "redb journal duplicate event id `{}`",
                    record.event.event_id.0
                )));
            }
            expected_cursor += 1;
        }

        let append_idempotency: Vec<RedbAppendIdempotencyRecord> =
            self.read_all(APPEND_IDEMPOTENCY_TABLE)?;
        let idempotency: Vec<JournalIdempotencyRecord> = self.read_all(IDEMPOTENCY_TABLE)?;
        let outbox: Vec<JournalOutboxRecord> = self.read_all(OUTBOX_TABLE)?;
        let mut outbox_pending_count = 0;
        let mut outbox_claimed_count = 0;
        let mut outbox_succeeded_count = 0;
        let mut outbox_failed_count = 0;
        for record in &outbox {
            match record.state {
                JournalOutboxState::Pending => outbox_pending_count += 1,
                JournalOutboxState::Claimed { .. } => outbox_claimed_count += 1,
                JournalOutboxState::Succeeded => outbox_succeeded_count += 1,
                JournalOutboxState::Failed { .. } => outbox_failed_count += 1,
            }
        }

        Ok(RedbJournalHealth {
            schema_version,
            stream_event_count: stream_events.len(),
            global_event_count: global_events.len(),
            append_idempotency_count: append_idempotency.len(),
            idempotency_count: idempotency.len(),
            outbox_pending_count,
            outbox_claimed_count,
            outbox_succeeded_count,
            outbox_failed_count,
            projection_checkpoint_count: self
                .read_all::<VidaProjectionCheckpoint>(PROJECTION_CHECKPOINT_TABLE)?
                .len(),
            projection_failure_count: self
                .read_all::<JournalProjectionFailure>(PROJECTION_FAILURE_TABLE)?
                .len(),
            artifact_count: self
                .read_all::<JournalArtifactRecord>(ARTIFACT_TABLE)?
                .len(),
        })
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
        validate_append_event_streams(&request)?;
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

fn payload_decode_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(format!("redb journal payload corrupt: {error}"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Instant;

    use super::{
        APPEND_IDEMPOTENCY_TABLE, EVENTS_BY_GLOBAL_CURSOR_TABLE, EVENTS_BY_STREAM_TABLE,
        OUTBOX_TABLE, REDB_CORRUPT_PAYLOAD_BLOCKER_CODE, REDB_SINGLE_WRITER_BLOCKER_CODE,
        RedbOperationalJournal, classify_redb_journal_error,
    };
    use redb::{ReadableDatabase, ReadableTable, TableDefinition};
    use taskflow_contracts::{
        VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
        VidaEventCursor, VidaEventRef, VidaIdempotencyKey, VidaOperation, VidaProjectionCheckpoint,
        VidaProjectionRef, VidaReceiptId, VidaSchemaId, VidaSchemaVersion, VidaStreamRef,
        VidaStreamVersion, VidaTimestamp,
    };
    use taskflow_state::{
        JournalAppendRequest, JournalArtifactRecord, JournalIdempotencyState, JournalOutboxState,
        OperationalJournal, RunWorkflowJournalRepository, TaskflowStateError,
        verify_run_workflow_repository_conformance,
        verify_run_workflow_repository_corrupt_payload_fails_closed,
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
    fn run_workflow_repository_replays_snapshot_hash_after_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        let report = verify_run_workflow_repository_conformance(&mut journal, "run-031-redb")
            .expect("repository conformance should pass");
        drop(journal);

        let mut reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let loaded = RunWorkflowJournalRepository::new(&mut reopened)
            .load("run-031-redb", "ldr-031")
            .expect("repository load should pass");

        assert_eq!(loaded.snapshot_replay_hash(), report.final_snapshot_hash);
        assert_eq!(report.event_count, 2);
        assert_eq!(
            reopened
                .health_status()
                .expect("journal health")
                .stream_event_count,
            2
        );
    }

    #[test]
    fn run_workflow_repository_load_fails_closed_on_corrupt_payload() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        verify_run_workflow_repository_corrupt_payload_fails_closed(
            &mut journal,
            "run-031-redb-corrupt",
        )
        .expect("corrupt payload must fail closed");
    }

    #[test]
    fn append_rejects_event_stream_mismatch() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("victim append should pass");
        let mut malformed = append_request(0, vec![event(2)], Vec::new());
        malformed.stream_id = VidaStreamRef("attacker-stream".to_string());
        malformed.idempotency_key = VidaIdempotencyKey("idem-2".to_string());

        let error = journal
            .append(malformed)
            .expect_err("mismatched event stream must fail");

        assert_eq!(
            error,
            TaskflowStateError::EventStreamMismatch {
                request_stream_id: "attacker-stream".to_string(),
                event_stream_id: "stream-1".to_string(),
                event_id: "event-2".to_string(),
            }
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1,
            "mismatched append must not inject an event into the victim stream"
        );
        assert!(
            journal
                .load_stream(&VidaStreamRef("attacker-stream".to_string()))
                .is_empty(),
            "mismatched append must not write under the request stream either"
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
            let error = second.expect_err("redb should reject a second open while writer is held");
            let blocker =
                classify_redb_journal_error(&error, &path).expect("lock error should classify");
            assert_eq!(blocker.code, REDB_SINGLE_WRITER_BLOCKER_CODE);
            assert!(
                blocker
                    .next_action
                    .contains("holding the redb journal writer lock")
            );
        }
    }

    #[test]
    fn health_status_reports_integrity_counts_after_claim_and_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(
                0,
                vec![event(1), event(2), event(3)],
                vec![effect("effect-1"), effect("effect-2")],
            ))
            .expect("append should pass");
        journal.claim_outbox_batch("worker-1", 1);
        journal.record_projection_checkpoint(projection_checkpoint(3));
        journal.index_artifact(JournalArtifactRecord {
            artifact_ref: taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
            content_hash: "hash-1".to_string(),
            path: "tests/journal/artifact-1.json".to_string(),
        });
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let health = reopened.health_status().expect("health status should pass");

        assert_eq!(health.schema_version, "1");
        assert_eq!(health.stream_event_count, 3);
        assert_eq!(health.global_event_count, 3);
        assert_eq!(health.append_idempotency_count, 1);
        assert_eq!(health.outbox_pending_count, 1);
        assert_eq!(health.outbox_claimed_count, 1);
        assert_eq!(health.projection_checkpoint_count, 1);
        assert_eq!(health.artifact_count, 1);
    }

    #[test]
    fn replay_after_projection_checkpoint_resumes_from_event_cursor() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, (1..=5).map(event).collect(), Vec::new()))
            .expect("append should pass");
        journal.record_projection_checkpoint(projection_checkpoint(3));
        let checkpoint = journal
            .projection_checkpoint(&VidaProjectionRef("projection-1".to_string()))
            .expect("checkpoint read should pass")
            .expect("checkpoint should exist");

        let replay = journal.read_global_after(Some(&checkpoint.event_cursor), 10);

        assert_eq!(replay.len(), 2);
        assert_eq!(
            replay[0].event.event_id,
            VidaEventRef("event-4".to_string())
        );
        assert_eq!(
            replay[1].event.event_id,
            VidaEventRef("event-5".to_string())
        );
    }

    #[test]
    fn claimed_outbox_state_survives_restart_before_execution() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append should pass");
        let claimed = journal.claim_outbox_batch("worker-1", 1);
        assert!(matches!(
            claimed[0].state,
            JournalOutboxState::Claimed { ref consumer_id } if consumer_id == "worker-1"
        ));
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let health = reopened.health_status().expect("health should pass");

        assert_eq!(health.outbox_claimed_count, 1);
        assert_eq!(health.outbox_pending_count, 0);
        assert_eq!(
            reopened
                .read_global_after(None, 10)
                .into_iter()
                .map(|record| record.event.event_id)
                .collect::<Vec<_>>(),
            vec![VidaEventRef("event-1".to_string())]
        );
    }

    #[test]
    fn projection_cache_wipe_rebuilds_from_event_cursor() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, (1..=4).map(event).collect(), Vec::new()))
            .expect("append should pass");
        journal.record_projection_checkpoint(projection_checkpoint(2));
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let checkpoint = reopened
            .projection_checkpoint(&VidaProjectionRef("projection-1".to_string()))
            .expect("checkpoint read should pass")
            .expect("checkpoint should exist");
        let rebuilt = reopened.read_global_after(Some(&checkpoint.event_cursor), 10);

        assert_eq!(
            rebuilt
                .iter()
                .map(|record| record.event.event_id.clone())
                .collect::<Vec<_>>(),
            vec![
                VidaEventRef("event-3".to_string()),
                VidaEventRef("event-4".to_string())
            ]
        );
        assert_eq!(
            rebuilt.last().expect("rebuilt events").global_cursor,
            VidaEventCursor("global-4".to_string())
        );
    }

    #[test]
    fn project_roots_with_distinct_redb_files_are_isolated() {
        let dir = tempdir().expect("tempdir");
        let path_a = dir.path().join("project-a.redb");
        let path_b = dir.path().join("project-b.redb");
        let mut journal_a = RedbOperationalJournal::create(&path_a).expect("create journal a");
        let mut journal_b = RedbOperationalJournal::create(&path_b).expect("create journal b");

        journal_a
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append project a");
        let mut request_b = append_request(0, vec![event_for_stream("stream-b", 1)], Vec::new());
        request_b.stream_id = VidaStreamRef("stream-b".to_string());
        request_b.idempotency_key = VidaIdempotencyKey("idem-b".to_string());
        journal_b.append(request_b).expect("append project b");

        assert_eq!(
            journal_a
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert!(
            journal_a
                .load_stream(&VidaStreamRef("stream-b".to_string()))
                .is_empty()
        );
        assert_eq!(
            journal_b
                .load_stream(&VidaStreamRef("stream-b".to_string()))
                .len(),
            1
        );
    }

    #[test]
    fn health_status_fails_closed_on_corrupted_payload() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append should pass");
        drop(journal);
        write_corrupt_record(&path, EVENTS_BY_GLOBAL_CURSOR_TABLE, "cursor-corrupt");

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let error = reopened
            .health_status()
            .expect_err("corrupted event payload must fail health status");

        assert!(
            matches!(error, TaskflowStateError::Storage(_)),
            "corrupt persisted JSON must surface as storage failure"
        );
        let blocker =
            classify_redb_journal_error(&error, &path).expect("corrupt payload should classify");
        assert_eq!(blocker.code, REDB_CORRUPT_PAYLOAD_BLOCKER_CODE);
        assert!(blocker.next_action.contains("rebuild projections"));
    }

    #[test]
    fn health_status_fails_closed_on_cursor_gap() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append should pass");
        drop(journal);
        write_global_record(&path, "00000000000000000002", "global-99", event(2));

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let error = reopened
            .health_status()
            .expect_err("cursor gap must fail health status");

        let message = error.to_string();
        assert!(
            message.contains("redb journal global cursor gap")
                || message.contains("is missing from stream index")
                || message.contains("stream/global event count mismatch")
        );
    }

    #[test]
    fn health_status_fails_closed_on_duplicate_event_id() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append should pass");
        drop(journal);
        let mut duplicate = event(2);
        duplicate.event_id = VidaEventRef("event-1".to_string());
        write_stream_event(&path, "stream-1:00000000000000000002", duplicate.clone());
        write_global_record(&path, "00000000000000000002", "global-2", duplicate);

        let reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let error = reopened
            .health_status()
            .expect_err("duplicate event id must fail health status");

        assert!(
            error
                .to_string()
                .contains("redb journal duplicate event id")
        );
    }

    #[test]
    #[ignore = "proof benchmark: run explicitly for ldr-030-redb-integrity benchmark output"]
    fn replay_10k_events_reports_timing() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        let events = (1..=10_000).map(event).collect::<Vec<_>>();
        let append_started = Instant::now();
        journal
            .append(append_request(0, events, Vec::new()))
            .expect("10k append should pass");
        let append_elapsed = append_started.elapsed();

        let replay_started = Instant::now();
        let replayed = journal.read_global_after(None, 10_000);
        let replay_elapsed = replay_started.elapsed();
        let health = journal.health_status().expect("health should pass");

        println!(
            "redb_journal_10k_replay events={} append_ms={} replay_ms={} stream_events={} global_events={}",
            replayed.len(),
            append_elapsed.as_millis(),
            replay_elapsed.as_millis(),
            health.stream_event_count,
            health.global_event_count
        );
        assert_eq!(replayed.len(), 10_000);
        assert_eq!(health.stream_event_count, 10_000);
        assert_eq!(health.global_event_count, 10_000);
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
        event_for_stream("stream-1", stream_version)
    }

    fn event_for_stream(stream_id: &str, stream_version: u64) -> VidaDomainEventEnvelope {
        VidaDomainEventEnvelope {
            schema_id: VidaSchemaId("schema.task.updated".to_string()),
            event_version: VidaSchemaVersion(1),
            event_id: VidaEventRef(format!("event-{stream_version}")),
            command_id: Some(VidaCommandRef("command-1".to_string())),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            stream_id: VidaStreamRef(stream_id.to_string()),
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

    fn write_corrupt_record(
        path: &Path,
        table_definition: TableDefinition<&str, &[u8]>,
        key: &str,
    ) {
        let db = redb::Database::open(path).expect("open redb for corruption fixture");
        let write = db.begin_write().expect("begin corrupt fixture write");
        {
            let mut table = write
                .open_table(table_definition)
                .expect("open corruption fixture table");
            table
                .insert(key, b"{not-json".as_slice())
                .expect("insert corruption fixture");
        }
        write.commit().expect("commit corruption fixture");
    }

    fn write_stream_event(path: &Path, key: &str, event: VidaDomainEventEnvelope) {
        let db = redb::Database::open(path).expect("open redb for stream fixture");
        let write = db.begin_write().expect("begin stream fixture write");
        {
            let mut table = write
                .open_table(EVENTS_BY_STREAM_TABLE)
                .expect("open stream fixture table");
            let payload = serde_json::to_vec(&event).expect("serialize stream fixture");
            table
                .insert(key, payload.as_slice())
                .expect("insert stream fixture");
        }
        write.commit().expect("commit stream fixture");
    }

    fn write_global_record(path: &Path, key: &str, cursor: &str, event: VidaDomainEventEnvelope) {
        let db = redb::Database::open(path).expect("open redb for global fixture");
        let write = db.begin_write().expect("begin global fixture write");
        {
            let mut table = write
                .open_table(EVENTS_BY_GLOBAL_CURSOR_TABLE)
                .expect("open global fixture table");
            let record = taskflow_state::JournalEventRecord {
                global_cursor: VidaEventCursor(cursor.to_string()),
                event,
            };
            let payload = serde_json::to_vec(&record).expect("serialize global fixture");
            table
                .insert(key, payload.as_slice())
                .expect("insert global fixture");
        }
        write.commit().expect("commit global fixture");
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
