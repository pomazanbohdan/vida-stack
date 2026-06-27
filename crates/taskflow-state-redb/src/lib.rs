use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    DependencyEdge, TaskRecord, VidaAggregateRef, VidaArtifactRef, VidaCommandRef,
    VidaDomainEventEnvelope, VidaEffectIntent, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_state::{
    JournalAggregateSnapshotRecord, JournalAppendReceipt, JournalAppendRequest,
    JournalArtifactRecord, JournalEventRecord, JournalIdempotencyRecord, JournalIdempotencyState,
    JournalOutboxRecord, JournalOutboxState, JournalProjectionFailure, OperationalJournal,
    TaskflowStateError, append_request_fingerprint, validate_append_event_streams,
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
const AGGREGATE_SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("aggregate_snapshot_by_id");
const TASKFLOW_SNAPSHOT_TASK_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("taskflow_snapshot_task_by_id");
const TASKFLOW_SNAPSHOT_DEPENDENCY_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("taskflow_snapshot_dependency_by_key");
const TASKFLOW_SNAPSHOT_SOURCE_METADATA_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("taskflow_snapshot_source_metadata_by_entity");
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
    #[serde(default)]
    command_id: Option<VidaCommandRef>,
    #[serde(default)]
    first_seen_at: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    result_hash: Option<String>,
    #[serde(default)]
    result_ref: Option<VidaReceiptId>,
    #[serde(default)]
    conflict_reason: Option<String>,
    #[serde(default)]
    retry_count: u64,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    committed_event_cursor: Option<VidaEventCursor>,
    #[serde(default)]
    conflict_code: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbProjectionCheckpointRecord {
    pub projection_id: VidaProjectionRef,
    pub stream_id: VidaStreamRef,
    pub last_global_cursor: VidaEventCursor,
    pub last_stream_version: VidaStreamVersion,
    pub input_hash: String,
    pub output_hash: String,
    pub schema_version: String,
    pub projector_version: String,
    pub updated_at: taskflow_contracts::VidaTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbProjectionReadBarrier {
    pub projection_id: VidaProjectionRef,
    pub required_event_cursor: VidaEventCursor,
    pub as_of_event_cursor: Option<VidaEventCursor>,
    pub status: String,
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbProjectionFailureRecord {
    pub projection_id: VidaProjectionRef,
    pub stream_id: VidaStreamRef,
    pub source_event_cursor: Option<VidaEventCursor>,
    pub failure_kind: String,
    pub failure_message: String,
    pub retry_after: Option<String>,
    pub repair_plan_ref: Option<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedbOutboxEffectRecord {
    pub outbox_id: VidaEventRef,
    pub effect: VidaEffectIntent,
    pub state: JournalOutboxState,
    pub source_stream_id: VidaStreamRef,
    pub source_event_cursor: Option<VidaEventCursor>,
    pub command_id: VidaCommandRef,
    pub effect_hash: String,
    pub attempt_count: u64,
    pub claimed_by: Option<String>,
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub retry_after: Option<String>,
    pub schema_version: String,
    pub lifecycle_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbArtifactIndexRecord {
    pub artifact_ref: VidaArtifactRef,
    pub content_hash: String,
    pub path: String,
    pub producer_event_cursor: Option<VidaEventCursor>,
    pub lifecycle_state: String,
    pub reconciliation_status: String,
    pub path_hash: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbArtifactMaterializationReceipt {
    pub artifact_ref: VidaArtifactRef,
    pub path: String,
    pub content_hash: String,
    pub source_event_cursor: VidaEventCursor,
    pub schema_version: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedbArtifactReconciliationRecord {
    pub artifact_ref: VidaArtifactRef,
    pub path: String,
    pub expected_content_hash: String,
    pub computed_content_hash: Option<String>,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbTaskflowSnapshotParity {
    pub status: String,
    pub task_count: usize,
    pub dependency_count: usize,
    pub task_hash: String,
    pub dependency_hash: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub normalization_finding: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbLegacySnapshotNormalizationFinding {
    pub entity_kind: String,
    pub entity_ref: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub status: String,
    pub finding: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedbTaskflowSnapshotImportPreview {
    pub status: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub normalization_findings: Vec<RedbLegacySnapshotNormalizationFinding>,
    pub quarantine_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RedbJournalBlocker {
    pub code: &'static str,
    pub next_action: String,
}

pub const REDB_SINGLE_WRITER_BLOCKER_CODE: &str = "redb_single_writer_lock_held";
pub const REDB_CORRUPT_PAYLOAD_BLOCKER_CODE: &str = "redb_journal_payload_corrupt";
pub const REDB_PROJECTION_FAILURE_BLOCKER_CODE: &str = "redb_projection_failure_recorded";
pub const REDB_STREAM_VERSION_CONFLICT_BLOCKER_CODE: &str = "redb_stream_version_conflict";

pub fn classify_redb_journal_error(
    error: &TaskflowStateError,
    journal_path: impl AsRef<Path>,
) -> Option<RedbJournalBlocker> {
    if let TaskflowStateError::StreamVersionConflict {
        stream_id,
        expected,
        actual,
    } = error
    {
        return Some(RedbJournalBlocker {
            code: REDB_STREAM_VERSION_CONFLICT_BLOCKER_CODE,
            next_action: format!(
                "Reload stream `{stream_id}` from the redb journal, rebase the command on actual version `{actual}`, then retry with expected version `{}`.",
                expected
                    .map(|version| version.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        });
    }
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

pub fn redb_journal_blocker_operator_payload(
    blocker: &RedbJournalBlocker,
    journal_path: impl AsRef<Path>,
) -> serde_json::Value {
    serde_json::json!({
        "status": "blocked",
        "blocker_codes": [blocker.code],
        "next_actions": [blocker.next_action],
        "artifact_refs": {
            "journal_path": journal_path.as_ref().display().to_string(),
        }
    })
}

pub fn redb_projection_health_operator_payload(
    health: &RedbJournalHealth,
    failures: &[RedbProjectionFailureRecord],
) -> serde_json::Value {
    let blocked = !failures.is_empty();
    serde_json::json!({
        "status": if blocked { "blocked" } else { "pass" },
        "blocker_codes": if blocked {
            vec![REDB_PROJECTION_FAILURE_BLOCKER_CODE]
        } else {
            Vec::<&str>::new()
        },
        "next_actions": if blocked {
            vec!["Inspect redb projection failure records, repair the projector or source event payload, then rebuild projections from the recorded source_event_cursor."]
        } else {
            Vec::<&str>::new()
        },
        "artifact_refs": {
            "projection_checkpoint_count": health.projection_checkpoint_count,
            "projection_failure_count": health.projection_failure_count,
            "latest_projection_failure_hash": failures.last().map(|failure| failure.content_hash.clone()),
        }
    })
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
            let _ = write
                .open_table(AGGREGATE_SNAPSHOT_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(TASKFLOW_SNAPSHOT_TASK_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(TASKFLOW_SNAPSHOT_DEPENDENCY_TABLE)
                .map_err(storage_error)?;
            let _ = write
                .open_table(TASKFLOW_SNAPSHOT_SOURCE_METADATA_TABLE)
                .map_err(storage_error)?;
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
        let outbox = self.outbox_effect_records()?;
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
            projection_checkpoint_count: self.projection_checkpoint_records()?.len(),
            projection_failure_count: self.projection_failure_records()?.len(),
            artifact_count: self.artifact_index_records()?.len(),
        })
    }

    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Result<Option<VidaProjectionCheckpoint>, TaskflowStateError> {
        self.projection_checkpoint_record(projection_id)
            .map(|record| record.map(RedbProjectionCheckpointRecord::into_checkpoint))
    }

    pub fn projection_failures(&self) -> Result<Vec<JournalProjectionFailure>, TaskflowStateError> {
        self.projection_failure_records().map(|records| {
            records
                .into_iter()
                .map(RedbProjectionFailureRecord::into_failure)
                .collect()
        })
    }

    pub fn projection_checkpoint_record(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Result<Option<RedbProjectionCheckpointRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(PROJECTION_CHECKPOINT_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        table
            .get(projection_id.0.as_str())
            .map_err(storage_error)?
            .map(|row| decode_projection_checkpoint_record(row.value()))
            .transpose()
    }

    pub fn projection_checkpoint_records(
        &self,
    ) -> Result<Vec<RedbProjectionCheckpointRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(PROJECTION_CHECKPOINT_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let mut records = Vec::new();
        for row in table.iter().map_err(storage_error)? {
            let (_, value) = row.map_err(storage_error)?;
            records.push(decode_projection_checkpoint_record(value.value())?);
        }
        Ok(records)
    }

    pub fn projection_read_barrier(
        &self,
        projection_id: &VidaProjectionRef,
        required_event_cursor: &VidaEventCursor,
    ) -> Result<RedbProjectionReadBarrier, TaskflowStateError> {
        let checkpoint = self.projection_checkpoint_record(projection_id)?;
        let as_of_event_cursor = checkpoint.map(|record| record.last_global_cursor);
        let required_cursor = strict_global_cursor_number(required_event_cursor)?;
        let status = if as_of_event_cursor
            .as_ref()
            .map(strict_global_cursor_number)
            .transpose()?
            .is_some_and(|cursor| cursor >= required_cursor)
        {
            "pass"
        } else {
            "blocked"
        };
        Ok(RedbProjectionReadBarrier {
            projection_id: projection_id.clone(),
            required_event_cursor: required_event_cursor.clone(),
            as_of_event_cursor,
            status: status.to_string(),
            blocker_code: (status == "blocked").then(|| "projection_not_caught_up".to_string()),
        })
    }

    pub fn projection_failure_records(
        &self,
    ) -> Result<Vec<RedbProjectionFailureRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(PROJECTION_FAILURE_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let mut records = Vec::new();
        for row in table.iter().map_err(storage_error)? {
            let (_, value) = row.map_err(storage_error)?;
            records.push(decode_projection_failure_record(value.value())?);
        }
        Ok(records)
    }

    pub fn artifact(
        &self,
        artifact_ref: &VidaArtifactRef,
    ) -> Result<Option<JournalArtifactRecord>, TaskflowStateError> {
        self.artifact_index_record(artifact_ref)
            .map(|record| record.map(RedbArtifactIndexRecord::into_artifact))
    }

    pub fn outbox_effect_record(
        &self,
        outbox_id: &VidaEventRef,
    ) -> Result<Option<RedbOutboxEffectRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(OUTBOX_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        table
            .get(outbox_id.0.as_str())
            .map_err(storage_error)?
            .map(|row| decode_outbox_effect_record(row.value()))
            .transpose()
    }

    pub fn outbox_effect_records(&self) -> Result<Vec<RedbOutboxEffectRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(OUTBOX_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let mut records = Vec::new();
        for row in table.iter().map_err(storage_error)? {
            let (_, value) = row.map_err(storage_error)?;
            records.push(decode_outbox_effect_record(value.value())?);
        }
        Ok(records)
    }

    pub fn schedule_outbox_retry(
        &self,
        outbox_id: &VidaEventRef,
        retry_after: Option<String>,
    ) -> Result<(), TaskflowStateError> {
        self.update_outbox(outbox_id, |record| {
            record.state = JournalOutboxState::Pending;
            record.claimed_by = None;
            record.failure_reason = None;
            record.retry_after = retry_after;
            record.lifecycle_state = outbox_lifecycle_state(&record.state);
        })
    }

    pub fn artifact_index_record(
        &self,
        artifact_ref: &VidaArtifactRef,
    ) -> Result<Option<RedbArtifactIndexRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(ARTIFACT_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        table
            .get(artifact_ref.0.as_str())
            .map_err(storage_error)?
            .map(|row| decode_artifact_index_record(row.value()))
            .transpose()
    }

    pub fn artifact_index_records(
        &self,
    ) -> Result<Vec<RedbArtifactIndexRecord>, TaskflowStateError> {
        let read = self.db.begin_read().map_err(storage_error)?;
        let table = match read.open_table(ARTIFACT_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let mut records = Vec::new();
        for row in table.iter().map_err(storage_error)? {
            let (_, value) = row.map_err(storage_error)?;
            records.push(decode_artifact_index_record(value.value())?);
        }
        Ok(records)
    }

    pub fn materialized_artifact_receipt(
        &self,
        artifact_ref: &VidaArtifactRef,
    ) -> Result<RedbArtifactMaterializationReceipt, TaskflowStateError> {
        let Some(record) = self.artifact_index_record(artifact_ref)? else {
            return Err(TaskflowStateError::Storage(format!(
                "artifact index record not found: {}",
                artifact_ref.0
            )));
        };
        let Some(source_event_cursor) = record.producer_event_cursor.clone() else {
            return Err(TaskflowStateError::Storage(format!(
                "artifact `{}` is missing source event cursor",
                artifact_ref.0
            )));
        };
        if record.schema_version != SCHEMA_VERSION {
            return Err(TaskflowStateError::Storage(format!(
                "artifact `{}` schema version mismatch: expected={} actual={}",
                artifact_ref.0, SCHEMA_VERSION, record.schema_version
            )));
        }
        if record.reconciliation_status != "sha256_pass" {
            return Err(TaskflowStateError::Storage(format!(
                "artifact `{}` is not hash-reconciled: {}",
                artifact_ref.0, record.reconciliation_status
            )));
        }
        Ok(RedbArtifactMaterializationReceipt {
            artifact_ref: record.artifact_ref,
            path: record.path,
            content_hash: record.content_hash,
            source_event_cursor,
            schema_version: record.schema_version,
            status: "materialized".to_string(),
        })
    }

    pub fn reconcile_and_materialize_artifact(
        &self,
        artifact_ref: &VidaArtifactRef,
        project_root: impl AsRef<Path>,
    ) -> Result<RedbArtifactMaterializationReceipt, TaskflowStateError> {
        let Some(mut record) = self.artifact_index_record(artifact_ref)? else {
            return Err(TaskflowStateError::Storage(format!(
                "artifact index record not found: {}",
                artifact_ref.0
            )));
        };
        let reconciliation = reconcile_artifact_record(record.clone(), project_root.as_ref())?;
        record.reconciliation_status = format!("sha256_{}", reconciliation.status);
        self.write_record(ARTIFACT_TABLE, &record.artifact_ref.0, &record)?;
        self.materialized_artifact_receipt(artifact_ref)
    }

    pub fn reconcile_artifact_hashes(
        &self,
        project_root: impl AsRef<Path>,
    ) -> Result<Vec<RedbArtifactReconciliationRecord>, TaskflowStateError> {
        let project_root = project_root.as_ref();
        self.artifact_index_records()?
            .into_iter()
            .map(|record| reconcile_artifact_record(record, project_root))
            .collect()
    }

    pub fn replace_taskflow_snapshot(
        &self,
        snapshot: &taskflow_state_fs::TaskSnapshot,
    ) -> Result<RedbTaskflowSnapshotParity, TaskflowStateError> {
        let preview = taskflow_snapshot_import_preview(
            snapshot,
            "memory_snapshot",
            "taskflow_state_fs::TaskSnapshot",
            taskflow_snapshot_source_hash(snapshot),
        );
        self.replace_taskflow_snapshot_with_preview(snapshot, preview)
    }

    fn replace_taskflow_snapshot_with_preview(
        &self,
        snapshot: &taskflow_state_fs::TaskSnapshot,
        preview: RedbTaskflowSnapshotImportPreview,
    ) -> Result<RedbTaskflowSnapshotParity, TaskflowStateError> {
        if preview.quarantine_count > 0 {
            return Err(TaskflowStateError::Storage(format!(
                "taskflow snapshot import quarantined {} finding(s): {}",
                preview.quarantine_count,
                preview
                    .normalization_findings
                    .iter()
                    .filter(|finding| finding.status == "quarantined")
                    .map(|finding| finding.finding.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }
        let normalized = normalize_taskflow_snapshot(snapshot);
        validate_taskflow_snapshot_storage_keys(&normalized)?;
        let metadata = preview.normalization_findings;
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut tasks = write
                .open_table(TASKFLOW_SNAPSHOT_TASK_TABLE)
                .map_err(storage_error)?;
            let task_keys = table_keys(&tasks)?;
            for key in task_keys {
                tasks.remove(key.as_str()).map_err(storage_error)?;
            }
            for task in &normalized.tasks {
                let payload = serde_json::to_vec(task).map_err(storage_error)?;
                tasks
                    .insert(task.id.as_str(), payload.as_slice())
                    .map_err(storage_error)?;
            }
            drop(tasks);

            let mut dependencies = write
                .open_table(TASKFLOW_SNAPSHOT_DEPENDENCY_TABLE)
                .map_err(storage_error)?;
            let dependency_keys = table_keys(&dependencies)?;
            for key in dependency_keys {
                dependencies.remove(key.as_str()).map_err(storage_error)?;
            }
            for dependency in &normalized.dependencies {
                let key = task_dependency_key(dependency);
                let payload = serde_json::to_vec(dependency).map_err(storage_error)?;
                dependencies
                    .insert(key.as_str(), payload.as_slice())
                    .map_err(storage_error)?;
            }
            drop(dependencies);

            let mut source_metadata = write
                .open_table(TASKFLOW_SNAPSHOT_SOURCE_METADATA_TABLE)
                .map_err(storage_error)?;
            let metadata_keys = table_keys(&source_metadata)?;
            for key in metadata_keys {
                source_metadata
                    .remove(key.as_str())
                    .map_err(storage_error)?;
            }
            for finding in &metadata {
                let key = taskflow_snapshot_source_metadata_key(finding);
                let payload = serde_json::to_vec(finding).map_err(storage_error)?;
                source_metadata
                    .insert(key.as_str(), payload.as_slice())
                    .map_err(storage_error)?;
            }
        }
        write.commit().map_err(storage_error)?;
        self.taskflow_snapshot_parity(&normalized)
    }

    pub fn replace_taskflow_snapshot_from_file(
        &self,
        snapshot_path: impl AsRef<Path>,
    ) -> Result<RedbTaskflowSnapshotParity, TaskflowStateError> {
        let snapshot_path = snapshot_path.as_ref();
        let source_bytes = fs::read(snapshot_path).map_err(storage_error)?;
        let snapshot = taskflow_state_fs::read_snapshot(snapshot_path).map_err(storage_error)?;
        let preview = taskflow_snapshot_import_preview(
            &snapshot,
            "file_snapshot",
            snapshot_path.display().to_string(),
            format!("sha256:{}", sha256_hex(&source_bytes)),
        );
        if preview.quarantine_count > 0 {
            return Err(TaskflowStateError::Storage(format!(
                "taskflow snapshot import quarantined {} finding(s): {}",
                preview.quarantine_count,
                preview
                    .normalization_findings
                    .iter()
                    .filter(|finding| finding.status == "quarantined")
                    .map(|finding| finding.finding.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }
        let mut report = self.replace_taskflow_snapshot_with_preview(&snapshot, preview)?;
        report.source_kind = "file_snapshot".to_string();
        report.source_ref = snapshot_path.display().to_string();
        report.source_hash = format!("sha256:{}", sha256_hex(&source_bytes));
        Ok(report)
    }

    pub fn preview_taskflow_snapshot_import(
        &self,
        snapshot: &taskflow_state_fs::TaskSnapshot,
    ) -> RedbTaskflowSnapshotImportPreview {
        taskflow_snapshot_import_preview(
            snapshot,
            "memory_snapshot",
            "taskflow_state_fs::TaskSnapshot",
            taskflow_snapshot_source_hash(snapshot),
        )
    }

    pub fn taskflow_snapshot_source_metadata(
        &self,
    ) -> Result<Vec<RedbLegacySnapshotNormalizationFinding>, TaskflowStateError> {
        self.read_all(TASKFLOW_SNAPSHOT_SOURCE_METADATA_TABLE)
    }

    pub fn export_taskflow_snapshot(
        &self,
    ) -> Result<taskflow_state_fs::TaskSnapshot, TaskflowStateError> {
        let tasks = self.read_all::<TaskRecord>(TASKFLOW_SNAPSHOT_TASK_TABLE)?;
        let dependencies = self.read_all::<DependencyEdge>(TASKFLOW_SNAPSHOT_DEPENDENCY_TABLE)?;
        Ok(normalize_taskflow_snapshot(
            &taskflow_state_fs::TaskSnapshot {
                tasks,
                dependencies,
            },
        ))
    }

    pub fn taskflow_snapshot_parity(
        &self,
        expected: &taskflow_state_fs::TaskSnapshot,
    ) -> Result<RedbTaskflowSnapshotParity, TaskflowStateError> {
        let expected = normalize_taskflow_snapshot(expected);
        let actual = self.export_taskflow_snapshot()?;
        let expected_fingerprint = taskflow_snapshot_fingerprint(&expected);
        let actual_fingerprint = taskflow_snapshot_fingerprint(&actual);
        Ok(RedbTaskflowSnapshotParity {
            status: if expected_fingerprint == actual_fingerprint {
                "pass"
            } else {
                "mismatch"
            }
            .to_string(),
            task_count: actual_fingerprint.task_count,
            dependency_count: actual_fingerprint.dependency_count,
            task_hash: actual_fingerprint.task_hash,
            dependency_hash: actual_fingerprint.dependency_hash,
            source_kind: expected_fingerprint.source_kind,
            source_ref: expected_fingerprint.source_ref,
            source_hash: expected_fingerprint.source_hash,
            normalization_finding: expected_fingerprint.normalization_finding,
        })
    }

    pub fn record_projection_failure_at_cursor(
        &mut self,
        failure: JournalProjectionFailure,
        source_event_cursor: Option<VidaEventCursor>,
    ) -> Result<(), TaskflowStateError> {
        let record = RedbProjectionFailureRecord::from_failure(failure, source_event_cursor);
        let key = format!(
            "{}:{}:{}",
            record.projection_id.0, record.stream_id.0, record.content_hash
        );
        self.write_record(PROJECTION_FAILURE_TABLE, &key, &record)
    }
}

impl OperationalJournal for RedbOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        validate_append_event_streams(&request)?;
        let ledger_key = request.idempotency_key.clone();
        let ledger_command_id = request.command_id.clone();
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
            let mut idempotency = write.open_table(IDEMPOTENCY_TABLE).map_err(storage_error)?;

            let existing_append_record = {
                let row = append_idempotency
                    .get(request.idempotency_key.0.as_str())
                    .map_err(storage_error)?;
                match row {
                    Some(row) => Some(
                        serde_json::from_slice::<RedbAppendIdempotencyRecord>(row.value())
                            .map_err(storage_error)?,
                    ),
                    None => None,
                }
            };

            if let Some(mut record) = existing_append_record {
                if record.request_fingerprint == request_fingerprint {
                    if record.status == "conflicted" {
                        record.retry_count += 1;
                        let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                        append_idempotency
                            .insert(request.idempotency_key.0.as_str(), payload.as_slice())
                            .map_err(storage_error)?;
                        Err(TaskflowStateError::IdempotencyConflict(
                            ledger_key.0.clone(),
                        ))
                    } else {
                        record.retry_count += 1;
                        record.command_id = record
                            .command_id
                            .or_else(|| Some(ledger_command_id.clone()));
                        record.status = "completed".to_string();
                        record.result_hash = Some(append_receipt_fingerprint(&record.receipt));
                        record.result_ref = Some(append_receipt_ref(&ledger_key, &record.receipt));
                        record.last_error = None;
                        record.committed_event_cursor = record.receipt.last_global_cursor.clone();
                        let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                        append_idempotency
                            .insert(request.idempotency_key.0.as_str(), payload.as_slice())
                            .map_err(storage_error)?;
                        let lifecycle = JournalIdempotencyRecord {
                            key: ledger_key.clone(),
                            command_id: ledger_command_id.clone(),
                            state: JournalIdempotencyState::Completed,
                            receipt_id: record.result_ref.clone(),
                            conflict_reason: None,
                        };
                        write_idempotency_lifecycle(&mut idempotency, lifecycle)?;
                        Ok(record.receipt)
                    }
                } else {
                    let conflict_code = "idempotency_payload_conflict".to_string();
                    let reason = format!(
                        "{conflict_code}: same idempotency key used with different append payload"
                    );
                    record.retry_count += 1;
                    record.command_id = record
                        .command_id
                        .or_else(|| Some(ledger_command_id.clone()));
                    record.status = "conflicted".to_string();
                    record.conflict_code = Some(conflict_code);
                    record.conflict_reason = Some(reason.clone());
                    record.last_error = Some(reason.clone());
                    let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                    append_idempotency
                        .insert(request.idempotency_key.0.as_str(), payload.as_slice())
                        .map_err(storage_error)?;
                    let lifecycle = JournalIdempotencyRecord {
                        key: ledger_key.clone(),
                        command_id: ledger_command_id.clone(),
                        state: JournalIdempotencyState::Conflicted,
                        receipt_id: record.result_ref.clone(),
                        conflict_reason: Some(reason),
                    };
                    write_idempotency_lifecycle(&mut idempotency, lifecycle)?;
                    Err(TaskflowStateError::IdempotencyConflict(
                        ledger_key.0.clone(),
                    ))
                }
            } else {
                let existing_lifecycle = idempotency
                    .get(ledger_key.0.as_str())
                    .map_err(storage_error)?
                    .map(|existing| {
                        serde_json::from_slice::<JournalIdempotencyRecord>(existing.value())
                    })
                    .transpose()
                    .map_err(storage_error)?;
                if let Some(existing_lifecycle) =
                    existing_lifecycle.filter(|existing| existing.command_id != ledger_command_id)
                {
                    let reason = "idempotency_payload_conflict: same idempotency key used by a different command".to_string();
                    let lifecycle = JournalIdempotencyRecord {
                        key: ledger_key.clone(),
                        command_id: existing_lifecycle.command_id,
                        state: JournalIdempotencyState::Conflicted,
                        receipt_id: existing_lifecycle.receipt_id,
                        conflict_reason: Some(reason),
                    };
                    write_idempotency_lifecycle(&mut idempotency, lifecycle)?;
                    Err(TaskflowStateError::IdempotencyConflict(
                        ledger_key.0.clone(),
                    ))
                } else {
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
                        let global_payload =
                            serde_json::to_vec(&global_record).map_err(storage_error)?;
                        events_by_global
                            .insert(global_event_key.as_str(), global_payload.as_slice())
                            .map_err(storage_error)?;
                    }
                    drop(events_by_stream);
                    drop(events_by_global);
                    let source_event_cursor = if event_count > 0 {
                        Some(VidaEventCursor(format!("global-{next_global_index}")))
                    } else {
                        None
                    };

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
                        let durable = RedbOutboxEffectRecord::from_outbox_record(
                            record,
                            source_event_cursor.clone(),
                        );
                        let payload = serde_json::to_vec(&durable).map_err(storage_error)?;
                        outbox
                            .insert(durable.outbox_id.0.as_str(), payload.as_slice())
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
                        command_id: Some(ledger_command_id.clone()),
                        first_seen_at: receipt
                            .first_global_cursor
                            .as_ref()
                            .map(|cursor| cursor.0.clone())
                            .unwrap_or_else(|| format!("stream-version-{actual_version}")),
                        status: "completed".to_string(),
                        result_hash: Some(append_receipt_fingerprint(&receipt)),
                        result_ref: Some(append_receipt_ref(&ledger_key, &receipt)),
                        conflict_reason: None,
                        retry_count: 0,
                        last_error: None,
                        committed_event_cursor: receipt.last_global_cursor.clone(),
                        conflict_code: None,
                    };
                    let payload = serde_json::to_vec(&idempotency_record).map_err(storage_error)?;
                    append_idempotency
                        .insert(request.idempotency_key.0.as_str(), payload.as_slice())
                        .map_err(storage_error)?;
                    let lifecycle = JournalIdempotencyRecord {
                        key: ledger_key.clone(),
                        command_id: ledger_command_id,
                        state: JournalIdempotencyState::Completed,
                        receipt_id: idempotency_record.result_ref,
                        conflict_reason: None,
                    };
                    write_idempotency_lifecycle(&mut idempotency, lifecycle)?;
                    Ok(receipt)
                }
            }
        };
        write.commit().map_err(storage_error)?;
        result
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
                    let record = decode_outbox_effect_record(value.value())?;
                    records.push((key.value().to_string(), record));
                }
                for (key, mut record) in records {
                    if !matches!(record.state, JournalOutboxState::Pending) {
                        continue;
                    }
                    record.state = JournalOutboxState::Claimed {
                        consumer_id: consumer_id.to_string(),
                    };
                    record.attempt_count += 1;
                    record.claimed_by = Some(consumer_id.to_string());
                    record.failure_reason = None;
                    record.retry_after = None;
                    record.lifecycle_state = outbox_lifecycle_state(&record.state);
                    claimed.push(record.clone().into_outbox_record());
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
            record.claimed_by = None;
            record.failure_reason = None;
            record.retry_after = None;
            record.lifecycle_state = outbox_lifecycle_state(&record.state);
        })
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.update_outbox(outbox_id, |record| {
            record.state = JournalOutboxState::Failed {
                reason: reason.clone(),
            };
            record.claimed_by = None;
            record.failure_reason = Some(reason);
            record.retry_after = None;
            record.lifecycle_state = outbox_lifecycle_state(&record.state);
        })
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        let record = RedbProjectionCheckpointRecord::from_checkpoint(checkpoint);
        let _ = self.write_projection_checkpoint_record(record);
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        let _ = self.record_projection_failure_at_cursor(failure, self.latest_global_cursor());
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        let record = RedbArtifactIndexRecord::from_artifact(artifact, self.latest_global_cursor());
        let _ = self.write_record(ARTIFACT_TABLE, &record.artifact_ref.0, &record);
    }

    fn record_aggregate_snapshot(&mut self, snapshot: JournalAggregateSnapshotRecord) {
        let _ = self.write_record(
            AGGREGATE_SNAPSHOT_TABLE,
            &snapshot.aggregate_id.0,
            &snapshot,
        );
    }

    fn aggregate_snapshot(
        &self,
        aggregate_id: &VidaAggregateRef,
    ) -> Option<JournalAggregateSnapshotRecord> {
        self.read_one(AGGREGATE_SNAPSHOT_TABLE, &aggregate_id.0)
            .ok()
            .flatten()
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

    fn write_projection_checkpoint_record(
        &self,
        record: RedbProjectionCheckpointRecord,
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write
                .open_table(PROJECTION_CHECKPOINT_TABLE)
                .map_err(storage_error)?;
            let mut should_write = true;
            if let Some(existing) = table
                .get(record.projection_id.0.as_str())
                .map_err(storage_error)?
            {
                let existing = decode_projection_checkpoint_record(existing.value())?;
                if projection_checkpoint_record_is_stale(&existing, &record) {
                    should_write = false;
                }
            }
            if should_write {
                let payload = serde_json::to_vec(&record).map_err(storage_error)?;
                table
                    .insert(record.projection_id.0.as_str(), payload.as_slice())
                    .map_err(storage_error)?;
            }
        }
        write.commit().map_err(storage_error)
    }

    fn update_outbox(
        &self,
        outbox_id: &VidaEventRef,
        update: impl FnOnce(&mut RedbOutboxEffectRecord),
    ) -> Result<(), TaskflowStateError> {
        let write = self.db.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(OUTBOX_TABLE).map_err(storage_error)?;
            let mut record = {
                let Some(row) = table.get(outbox_id.0.as_str()).map_err(storage_error)? else {
                    return Err(TaskflowStateError::OutboxRecordNotFound(
                        outbox_id.0.clone(),
                    ));
                };
                decode_outbox_effect_record(row.value())?
            };
            update(&mut record);
            let payload = serde_json::to_vec(&record).map_err(storage_error)?;
            table
                .insert(outbox_id.0.as_str(), payload.as_slice())
                .map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }

    fn latest_global_cursor(&self) -> Option<VidaEventCursor> {
        let mut records: Vec<JournalEventRecord> =
            self.read_all(EVENTS_BY_GLOBAL_CURSOR_TABLE).ok()?;
        records.sort_by_key(|record| global_cursor_number(&record.global_cursor));
        records.last().map(|record| record.global_cursor.clone())
    }
}

impl RedbProjectionCheckpointRecord {
    fn from_checkpoint(checkpoint: VidaProjectionCheckpoint) -> Self {
        let input_hash = stable_hash_hex(&format!(
            "{}:{}:{}",
            checkpoint.projection_id.0, checkpoint.stream_id.0, checkpoint.event_cursor.0
        ));
        let output_hash = stable_hash_hex(&serde_json::to_string(&checkpoint).unwrap_or_default());
        Self {
            projection_id: checkpoint.projection_id,
            stream_id: checkpoint.stream_id,
            last_global_cursor: checkpoint.event_cursor,
            last_stream_version: checkpoint.stream_version,
            input_hash,
            output_hash,
            schema_version: SCHEMA_VERSION.to_string(),
            projector_version: "redb-operational-journal-projection".to_string(),
            updated_at: checkpoint.updated_at,
        }
    }

    fn into_checkpoint(self) -> VidaProjectionCheckpoint {
        VidaProjectionCheckpoint {
            projection_id: self.projection_id,
            stream_id: self.stream_id,
            event_cursor: self.last_global_cursor,
            stream_version: self.last_stream_version,
            updated_at: self.updated_at,
        }
    }
}

impl RedbOutboxEffectRecord {
    fn from_outbox_record(
        record: JournalOutboxRecord,
        source_event_cursor: Option<VidaEventCursor>,
    ) -> Self {
        let effect_hash =
            stable_hash_hex(&serde_json::to_string(&record.effect).unwrap_or_default());
        let claimed_by = claimed_by_from_outbox_state(&record.state);
        let failure_reason = failure_reason_from_outbox_state(&record.state);
        let lifecycle_state = outbox_lifecycle_state(&record.state);
        Self {
            outbox_id: record.outbox_id,
            source_stream_id: record.effect.stream_id.clone(),
            command_id: record.effect.command_id.clone(),
            effect: record.effect,
            state: record.state,
            source_event_cursor,
            effect_hash,
            attempt_count: if claimed_by.is_some() { 1 } else { 0 },
            claimed_by,
            failure_reason,
            retry_after: None,
            schema_version: SCHEMA_VERSION.to_string(),
            lifecycle_state,
        }
    }

    fn into_outbox_record(self) -> JournalOutboxRecord {
        JournalOutboxRecord {
            outbox_id: self.outbox_id,
            effect: self.effect,
            state: self.state,
        }
    }
}

fn decode_outbox_effect_record(value: &[u8]) -> Result<RedbOutboxEffectRecord, TaskflowStateError> {
    if let Ok(record) = serde_json::from_slice::<RedbOutboxEffectRecord>(value) {
        return Ok(record);
    }
    let record: JournalOutboxRecord =
        serde_json::from_slice(value).map_err(payload_decode_error)?;
    Ok(RedbOutboxEffectRecord::from_outbox_record(record, None))
}

fn outbox_lifecycle_state(state: &JournalOutboxState) -> String {
    match state {
        JournalOutboxState::Pending => "pending",
        JournalOutboxState::Claimed { .. } => "claimed",
        JournalOutboxState::Succeeded => "succeeded",
        JournalOutboxState::Failed { .. } => "failed",
    }
    .to_string()
}

fn claimed_by_from_outbox_state(state: &JournalOutboxState) -> Option<String> {
    match state {
        JournalOutboxState::Claimed { consumer_id } => Some(consumer_id.clone()),
        _ => None,
    }
}

fn failure_reason_from_outbox_state(state: &JournalOutboxState) -> Option<String> {
    match state {
        JournalOutboxState::Failed { reason } => Some(reason.clone()),
        _ => None,
    }
}

impl RedbArtifactIndexRecord {
    fn from_artifact(
        artifact: JournalArtifactRecord,
        producer_event_cursor: Option<VidaEventCursor>,
    ) -> Self {
        let path_hash = stable_hash_hex(&artifact.path);
        Self {
            artifact_ref: artifact.artifact_ref,
            content_hash: artifact.content_hash,
            path: artifact.path,
            producer_event_cursor,
            lifecycle_state: "indexed".to_string(),
            reconciliation_status: "pending_reconciliation".to_string(),
            path_hash,
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    fn into_artifact(self) -> JournalArtifactRecord {
        JournalArtifactRecord {
            artifact_ref: self.artifact_ref,
            content_hash: self.content_hash,
            path: self.path,
        }
    }
}

fn decode_artifact_index_record(
    value: &[u8],
) -> Result<RedbArtifactIndexRecord, TaskflowStateError> {
    if let Ok(record) = serde_json::from_slice::<RedbArtifactIndexRecord>(value) {
        return Ok(record);
    }
    let artifact: JournalArtifactRecord =
        serde_json::from_slice(value).map_err(payload_decode_error)?;
    Ok(RedbArtifactIndexRecord::from_artifact(artifact, None))
}

fn normalize_taskflow_snapshot(
    snapshot: &taskflow_state_fs::TaskSnapshot,
) -> taskflow_state_fs::TaskSnapshot {
    let mut tasks = snapshot.tasks.clone();
    tasks.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    let mut dependencies = snapshot.dependencies.clone();
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
    taskflow_state_fs::TaskSnapshot {
        tasks,
        dependencies,
    }
}

fn taskflow_snapshot_import_preview(
    snapshot: &taskflow_state_fs::TaskSnapshot,
    source_kind: impl Into<String>,
    source_ref: impl Into<String>,
    source_hash: impl Into<String>,
) -> RedbTaskflowSnapshotImportPreview {
    let source_kind = source_kind.into();
    let source_ref = source_ref.into();
    let source_hash = source_hash.into();
    let mut findings = Vec::new();
    let mut task_ids = BTreeSet::new();
    for task in &snapshot.tasks {
        let inserted = task_ids.insert(task.id.as_str().to_string());
        findings.push(RedbLegacySnapshotNormalizationFinding {
            entity_kind: "task".to_string(),
            entity_ref: task.id.as_str().to_string(),
            source_kind: source_kind.clone(),
            source_ref: source_ref.clone(),
            source_hash: source_hash.clone(),
            status: if inserted {
                "normalized"
            } else {
                "quarantined"
            }
            .to_string(),
            finding: if inserted {
                "task record accepted for deterministic normalized import".to_string()
            } else {
                format!(
                    "taskflow snapshot import rejected duplicate task id: {}",
                    task.id.as_str()
                )
            },
        });
    }

    let mut dependency_keys = BTreeSet::new();
    for dependency in &snapshot.dependencies {
        let key = task_dependency_key(dependency);
        let inserted = dependency_keys.insert(key.clone());
        findings.push(RedbLegacySnapshotNormalizationFinding {
            entity_kind: "dependency".to_string(),
            entity_ref: key.clone(),
            source_kind: source_kind.clone(),
            source_ref: source_ref.clone(),
            source_hash: source_hash.clone(),
            status: if inserted {
                "normalized"
            } else {
                "quarantined"
            }
            .to_string(),
            finding: if inserted {
                "dependency record accepted for deterministic normalized import".to_string()
            } else {
                format!("taskflow snapshot import rejected duplicate dependency key: {key}")
            },
        });
    }

    let quarantine_count = findings
        .iter()
        .filter(|finding| finding.status == "quarantined")
        .count();
    RedbTaskflowSnapshotImportPreview {
        status: if quarantine_count == 0 {
            "pass"
        } else {
            "quarantined"
        }
        .to_string(),
        source_kind,
        source_ref,
        source_hash,
        normalization_findings: findings,
        quarantine_count,
    }
}

fn validate_taskflow_snapshot_storage_keys(
    snapshot: &taskflow_state_fs::TaskSnapshot,
) -> Result<(), TaskflowStateError> {
    let mut task_ids = BTreeSet::new();
    for task in &snapshot.tasks {
        if !task_ids.insert(task.id.as_str().to_string()) {
            return Err(TaskflowStateError::Storage(format!(
                "taskflow snapshot import rejected duplicate task id: {}",
                task.id.as_str()
            )));
        }
    }

    let mut dependency_keys = BTreeSet::new();
    for dependency in &snapshot.dependencies {
        let key = task_dependency_key(dependency);
        if !dependency_keys.insert(key.clone()) {
            return Err(TaskflowStateError::Storage(format!(
                "taskflow snapshot import rejected duplicate dependency key: {key}"
            )));
        }
    }

    Ok(())
}

fn taskflow_snapshot_source_hash(snapshot: &taskflow_state_fs::TaskSnapshot) -> String {
    stable_hash_hex(&serde_json::to_string(snapshot).unwrap_or_default())
}

fn taskflow_snapshot_fingerprint(
    snapshot: &taskflow_state_fs::TaskSnapshot,
) -> RedbTaskflowSnapshotParity {
    let normalized = normalize_taskflow_snapshot(snapshot);
    RedbTaskflowSnapshotParity {
        status: "pass".to_string(),
        task_count: normalized.tasks.len(),
        dependency_count: normalized.dependencies.len(),
        task_hash: stable_hash_hex(&serde_json::to_string(&normalized.tasks).unwrap_or_default()),
        dependency_hash: stable_hash_hex(
            &serde_json::to_string(&normalized.dependencies).unwrap_or_default(),
        ),
        source_kind: "memory_snapshot".to_string(),
        source_ref: "taskflow_state_fs::TaskSnapshot".to_string(),
        source_hash: taskflow_snapshot_source_hash(&normalized),
        normalization_finding: "normalized_sorted_unique_storage_keys".to_string(),
    }
}

fn task_dependency_key(dependency: &DependencyEdge) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        dependency.issue_id.as_str(),
        dependency.depends_on_id.as_str(),
        dependency.dependency_type
    )
}

fn taskflow_snapshot_source_metadata_key(
    finding: &RedbLegacySnapshotNormalizationFinding,
) -> String {
    format!("{}\u{1f}{}", finding.entity_kind, finding.entity_ref)
}

fn table_keys(table: &redb::Table<&str, &[u8]>) -> Result<Vec<String>, TaskflowStateError> {
    let mut keys = Vec::new();
    for row in table.iter().map_err(storage_error)? {
        let (key, _) = row.map_err(storage_error)?;
        keys.push(key.value().to_string());
    }
    Ok(keys)
}

fn reconcile_artifact_record(
    record: RedbArtifactIndexRecord,
    project_root: &Path,
) -> Result<RedbArtifactReconciliationRecord, TaskflowStateError> {
    let artifact_path = Path::new(&record.path);
    let invalid_path = artifact_path.is_absolute()
        || artifact_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
    if invalid_path {
        return Ok(RedbArtifactReconciliationRecord {
            artifact_ref: record.artifact_ref,
            path: record.path,
            expected_content_hash: record.content_hash,
            computed_content_hash: None,
            status: "out_of_root".to_string(),
            detail: Some("artifact path must be relative to the project root".to_string()),
        });
    }

    let materialized_path = project_root.join(&record.path);
    let root = project_root.canonicalize().map_err(storage_error)?;
    let metadata = match std::fs::symlink_metadata(&materialized_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RedbArtifactReconciliationRecord {
                artifact_ref: record.artifact_ref,
                path: record.path,
                expected_content_hash: record.content_hash,
                computed_content_hash: None,
                status: "missing".to_string(),
                detail: Some("artifact path is not materialized".to_string()),
            });
        }
        Err(error) => return Err(storage_error(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Ok(RedbArtifactReconciliationRecord {
            artifact_ref: record.artifact_ref,
            path: record.path,
            expected_content_hash: record.content_hash,
            computed_content_hash: None,
            status: "out_of_root".to_string(),
            detail: Some("artifact path must be a regular file under the project root".to_string()),
        });
    }
    let materialized_path = materialized_path.canonicalize().map_err(storage_error)?;
    if !materialized_path.starts_with(&root) {
        return Ok(RedbArtifactReconciliationRecord {
            artifact_ref: record.artifact_ref,
            path: record.path,
            expected_content_hash: record.content_hash,
            computed_content_hash: None,
            status: "out_of_root".to_string(),
            detail: Some("artifact path must remain under the project root".to_string()),
        });
    }

    let content = match std::fs::read(&materialized_path) {
        Ok(content) => content,
        Err(error) => return Err(storage_error(error)),
    };
    let computed = content_hash_for_expected(&record.content_hash, &content);
    let status = if computed == record.content_hash {
        "pass"
    } else {
        "mismatch"
    };
    Ok(RedbArtifactReconciliationRecord {
        artifact_ref: record.artifact_ref,
        path: record.path,
        expected_content_hash: record.content_hash,
        computed_content_hash: Some(computed),
        status: status.to_string(),
        detail: None,
    })
}

fn decode_projection_checkpoint_record(
    value: &[u8],
) -> Result<RedbProjectionCheckpointRecord, TaskflowStateError> {
    if let Ok(record) = serde_json::from_slice::<RedbProjectionCheckpointRecord>(value) {
        return Ok(record);
    }
    let checkpoint: VidaProjectionCheckpoint =
        serde_json::from_slice(value).map_err(payload_decode_error)?;
    Ok(RedbProjectionCheckpointRecord::from_checkpoint(checkpoint))
}

impl RedbProjectionFailureRecord {
    fn from_failure(
        failure: JournalProjectionFailure,
        source_event_cursor: Option<VidaEventCursor>,
    ) -> Self {
        let content_hash = stable_hash_hex(&format!(
            "{}:{}:{:?}:{}",
            failure.projection_id.0, failure.stream_id.0, source_event_cursor, failure.error
        ));
        Self {
            projection_id: failure.projection_id,
            stream_id: failure.stream_id,
            source_event_cursor,
            failure_kind: "projection_rebuild_failed".to_string(),
            failure_message: failure.error,
            retry_after: None,
            repair_plan_ref: Some("redb-projection-repair-plan".to_string()),
            content_hash,
        }
    }

    fn into_failure(self) -> JournalProjectionFailure {
        JournalProjectionFailure {
            projection_id: self.projection_id,
            stream_id: self.stream_id,
            error: self.failure_message,
        }
    }
}

fn decode_projection_failure_record(
    value: &[u8],
) -> Result<RedbProjectionFailureRecord, TaskflowStateError> {
    if let Ok(record) = serde_json::from_slice::<RedbProjectionFailureRecord>(value) {
        return Ok(record);
    }
    let failure: JournalProjectionFailure =
        serde_json::from_slice(value).map_err(payload_decode_error)?;
    Ok(RedbProjectionFailureRecord::from_failure(failure, None))
}

fn projection_checkpoint_record_is_stale(
    existing: &RedbProjectionCheckpointRecord,
    candidate: &RedbProjectionCheckpointRecord,
) -> bool {
    let existing_cursor = global_cursor_number(&existing.last_global_cursor);
    let candidate_cursor = global_cursor_number(&candidate.last_global_cursor);
    candidate_cursor < existing_cursor
        || (candidate_cursor == existing_cursor
            && candidate.last_stream_version.0 < existing.last_stream_version.0)
}

fn global_cursor_number(cursor: &VidaEventCursor) -> usize {
    cursor
        .0
        .strip_prefix("global-")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

fn strict_global_cursor_number(cursor: &VidaEventCursor) -> Result<usize, TaskflowStateError> {
    cursor
        .0
        .strip_prefix("global-")
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| {
            TaskflowStateError::Storage(format!(
                "redb projection barrier cursor is malformed: {}",
                cursor.0
            ))
        })
}

fn storage_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(error.to_string())
}

fn payload_decode_error(error: impl std::fmt::Display) -> TaskflowStateError {
    TaskflowStateError::Storage(format!("redb journal payload corrupt: {error}"))
}

fn append_receipt_fingerprint(receipt: &JournalAppendReceipt) -> String {
    format!("{receipt:?}")
}

fn append_receipt_ref(key: &VidaIdempotencyKey, receipt: &JournalAppendReceipt) -> VidaReceiptId {
    VidaReceiptId(format!(
        "redb-append:{}:{}",
        key.0, receipt.stream_version.0
    ))
}

fn write_idempotency_lifecycle(
    table: &mut redb::Table<'_, &str, &[u8]>,
    record: JournalIdempotencyRecord,
) -> Result<(), TaskflowStateError> {
    let existing = table
        .get(record.key.0.as_str())
        .map_err(storage_error)?
        .map(|existing| serde_json::from_slice::<JournalIdempotencyRecord>(existing.value()))
        .transpose()
        .map_err(storage_error)?;
    if let Some(existing) = existing {
        if existing.command_id != record.command_id {
            let key = record.key.0.clone();
            let lifecycle_payload = serde_json::to_vec(&JournalIdempotencyRecord {
                key: record.key,
                command_id: existing.command_id,
                state: JournalIdempotencyState::Conflicted,
                receipt_id: existing.receipt_id,
                conflict_reason: Some("idempotency_payload_conflict: same idempotency key used by a different command".to_string()),
            })
            .map_err(storage_error)?;
            table
                .insert(key.as_str(), lifecycle_payload.as_slice())
                .map_err(storage_error)?;
            return Err(TaskflowStateError::IdempotencyConflict(key));
        }
    }

    let key = record.key.0.clone();
    let lifecycle_payload = serde_json::to_vec(&record).map_err(storage_error)?;
    table
        .insert(key.as_str(), lifecycle_payload.as_slice())
        .map_err(storage_error)?;
    Ok(())
}

fn stable_hash_hex(input: &str) -> String {
    stable_hash_bytes_hex(input.as_bytes())
}

fn content_hash_for_expected(expected: &str, content: &[u8]) -> String {
    if expected.starts_with("sha256:") {
        format!("sha256:{}", sha256_hex(content))
    } else if expected.len() == 64 && expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        sha256_hex(content)
    } else {
        stable_hash_bytes_hex(content)
    }
}

fn stable_hash_bytes_hex(input: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn sha256_hex(input: &[u8]) -> String {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut message = input.to_vec();
    let bit_len = (message.len() as u64) * 8;
    message.push(0x80);
    while (message.len() % 64) != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = H0;
    let mut words = [0u32; 64];
    for chunk in message.chunks(64) {
        for index in 0..16 {
            let offset = index * 4;
            words[index] = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_work = h[7];
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_work
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h_work = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_work);
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    use super::{
        APPEND_IDEMPOTENCY_TABLE, ARTIFACT_TABLE, EVENTS_BY_GLOBAL_CURSOR_TABLE,
        EVENTS_BY_STREAM_TABLE, OUTBOX_TABLE, REDB_CORRUPT_PAYLOAD_BLOCKER_CODE,
        REDB_PROJECTION_FAILURE_BLOCKER_CODE, REDB_SINGLE_WRITER_BLOCKER_CODE,
        REDB_STREAM_VERSION_CONFLICT_BLOCKER_CODE, RedbAppendIdempotencyRecord,
        RedbOperationalJournal, classify_redb_journal_error, redb_journal_blocker_operator_payload,
        redb_projection_health_operator_payload,
    };
    use redb::{ReadableDatabase, ReadableTable, TableDefinition};
    use taskflow_contracts::{
        DependencyEdge, TaskRecord, VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope,
        VidaEffectIntent, VidaEffectRef, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
        VidaOperation, VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId, VidaSchemaId,
        VidaSchemaVersion, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
    };
    use taskflow_state::{
        JournalAppendRequest, JournalArtifactRecord, JournalIdempotencyState, JournalOutboxState,
        JournalProjectionFailure, OperationalJournal, RunWorkflowJournalRepository,
        TaskflowStateError, verify_run_workflow_repository_conformance,
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
        let loaded_before_reopen = RunWorkflowJournalRepository::new(&mut journal)
            .load("run-031-redb", "ldr-031")
            .expect("repository load should pass");
        RunWorkflowJournalRepository::new(&mut journal).save_snapshot(&loaded_before_reopen);
        drop(journal);

        let mut reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let loaded = RunWorkflowJournalRepository::new(&mut reopened)
            .load("run-031-redb", "ldr-031")
            .expect("repository load should pass");
        let snapshot = RunWorkflowJournalRepository::new(&mut reopened)
            .load_snapshot("run-031-redb", "ldr-031")
            .expect("snapshot load should pass")
            .expect("snapshot should exist after reopen");

        assert_eq!(loaded.snapshot_replay_hash(), report.final_snapshot_hash);
        assert_eq!(snapshot.snapshot_replay_hash(), report.final_snapshot_hash);
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
        let blocker = classify_redb_journal_error(&error, &path).expect("conflict should classify");
        assert_eq!(blocker.code, REDB_STREAM_VERSION_CONFLICT_BLOCKER_CODE);
        assert!(blocker.next_action.contains("rebase the command"));
        let operator_payload = redb_journal_blocker_operator_payload(&blocker, &path);
        assert_eq!(operator_payload["status"], "blocked");
        assert_eq!(
            operator_payload["blocker_codes"][0],
            REDB_STREAM_VERSION_CONFLICT_BLOCKER_CODE
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.read_global_after(None, 10).len(), 1);
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 0);
        let append_ledger: Option<RedbAppendIdempotencyRecord> = journal
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-2")
            .expect("append ledger read");
        assert!(
            append_ledger.is_none(),
            "stale append must not write idempotency ledger rows"
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
    fn append_retry_cache_survives_restart_and_repeated_retries() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        let first = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        drop(journal);

        let mut reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        for _ in 0..64 {
            let mut retry_request = append_request(1, vec![event(1)], vec![effect("effect-1")]);
            retry_request.expected_stream_version = Some(VidaStreamVersion(1));
            let retry = reopened
                .append(retry_request)
                .expect("same payload retry after restart should return cached receipt");
            assert_eq!(retry, first);
        }

        let health = reopened.health_status().expect("journal health");
        assert_eq!(health.append_idempotency_count, 1);
        assert_eq!(health.idempotency_count, 1);
        assert_eq!(health.stream_event_count, 1);
        assert_eq!(health.global_event_count, 1);
        assert_eq!(health.outbox_pending_count, 1);
        let append_ledger: RedbAppendIdempotencyRecord = reopened
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-1")
            .expect("append ledger read")
            .expect("append ledger row");
        assert_eq!(append_ledger.status, "completed");
        assert_eq!(append_ledger.retry_count, 64);
        assert_eq!(
            append_ledger.committed_event_cursor,
            first.last_global_cursor
        );
        assert!(append_ledger.result_hash.is_some());
        assert!(append_ledger.result_ref.is_some());
        let command_ledger = reopened
            .idempotency_record(&VidaIdempotencyKey("idem-1".to_string()))
            .expect("command idempotency record");
        assert_eq!(command_ledger.state, JournalIdempotencyState::Completed);
        assert_eq!(command_ledger.receipt_id, append_ledger.result_ref);
    }

    #[test]
    fn append_retry_registry_dedupes_100_concurrent_callers_and_records_audit_trace() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = Arc::new(Mutex::new(
            RedbOperationalJournal::create(&path).expect("create journal"),
        ));

        let first = journal
            .lock()
            .expect("journal mutex")
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let mut handles = Vec::new();
        for _ in 0..100 {
            let journal = Arc::clone(&journal);
            handles.push(thread::spawn(move || {
                let mut retry_request = append_request(1, vec![event(1)], vec![effect("effect-1")]);
                retry_request.expected_stream_version = Some(VidaStreamVersion(1));
                journal
                    .lock()
                    .expect("journal mutex")
                    .append(retry_request)
                    .expect("same payload retry should return cached receipt")
            }));
        }

        for handle in handles {
            assert_eq!(handle.join().expect("retry worker should finish"), first);
        }

        let journal = journal.lock().expect("journal mutex");
        let stream = journal.load_stream(&VidaStreamRef("stream-1".to_string()));
        assert_eq!(stream.len(), 1);
        assert_eq!(stream[0].trace["correlation_id"], "correlation-1");
        let health = journal.health_status().expect("journal health");
        assert_eq!(health.stream_event_count, 1);
        assert_eq!(health.global_event_count, 1);
        assert_eq!(health.outbox_pending_count, 1);
        let append_ledger: RedbAppendIdempotencyRecord = journal
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-1")
            .expect("append ledger read")
            .expect("append ledger row");
        assert_eq!(append_ledger.status, "completed");
        assert_eq!(append_ledger.retry_count, 100);
        assert!(append_ledger.command_id.is_some());
        assert_eq!(append_ledger.first_seen_at, "global-1");
        assert!(append_ledger.conflict_code.is_none());
        assert!(append_ledger.conflict_reason.is_none());
        assert!(append_ledger.last_error.is_none());
        assert_eq!(
            append_ledger.result_ref,
            Some(super::append_receipt_ref(
                &VidaIdempotencyKey("idem-1".to_string()),
                &first
            ))
        );
        assert_eq!(
            append_ledger.committed_event_cursor,
            first.last_global_cursor
        );
        assert!(append_ledger.result_hash.is_some());
        let command_ledger = journal
            .idempotency_record(&VidaIdempotencyKey("idem-1".to_string()))
            .expect("command idempotency record");
        assert_eq!(command_ledger.state, JournalIdempotencyState::Completed);
        assert_eq!(command_ledger.receipt_id, append_ledger.result_ref);
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
        let append_ledger: RedbAppendIdempotencyRecord = journal
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-1")
            .expect("append ledger read")
            .expect("append ledger row");
        assert_eq!(append_ledger.status, "conflicted");
        assert_eq!(append_ledger.retry_count, 1);
        assert_eq!(
            append_ledger.conflict_code.as_deref(),
            Some("idempotency_payload_conflict")
        );
        assert!(
            append_ledger
                .conflict_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("idempotency_payload_conflict"))
        );
        assert!(
            append_ledger
                .last_error
                .as_deref()
                .is_some_and(|reason| reason.contains("idempotency_payload_conflict"))
        );
        let command_ledger = journal
            .idempotency_record(&VidaIdempotencyKey("idem-1".to_string()))
            .expect("command idempotency record");
        assert_eq!(command_ledger.state, JournalIdempotencyState::Conflicted);
        assert!(
            command_ledger
                .conflict_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("idempotency_payload_conflict"))
        );
        let original_retry = journal
            .append(append_request(1, vec![event(1)], vec![effect("effect-1")]))
            .expect_err("original payload retry after conflict must stay conflicted");
        assert_eq!(
            original_retry,
            TaskflowStateError::IdempotencyConflict("idem-1".to_string())
        );
        let append_ledger_after_original_retry: RedbAppendIdempotencyRecord = journal
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-1")
            .expect("append ledger read")
            .expect("append ledger row");
        assert_eq!(append_ledger_after_original_retry.status, "conflicted");
        assert_eq!(append_ledger_after_original_retry.retry_count, 2);
    }

    #[test]
    fn append_conflict_audit_survives_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        journal
            .append(append_request(1, vec![event(2)], vec![effect("effect-2")]))
            .expect_err("changed payload with same idempotency key must fail");
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let append_ledger: RedbAppendIdempotencyRecord = reopened
            .read_one(APPEND_IDEMPOTENCY_TABLE, "idem-1")
            .expect("append ledger read")
            .expect("append ledger row");
        assert_eq!(append_ledger.status, "conflicted");
        assert_eq!(
            append_ledger.conflict_code.as_deref(),
            Some("idempotency_payload_conflict")
        );
        let command_ledger = reopened
            .idempotency_record(&VidaIdempotencyKey("idem-1".to_string()))
            .expect("command idempotency record");
        assert_eq!(command_ledger.state, JournalIdempotencyState::Conflicted);
    }

    #[test]
    fn append_rejects_idempotency_key_reserved_by_another_command() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        let key = VidaIdempotencyKey("shared-key".to_string());
        let victim_command = VidaCommandRef("command-victim".to_string());

        journal
            .record_idempotency_started(key.clone(), victim_command.clone())
            .expect("victim reservation should pass");

        let mut attacker_request = append_request(0, vec![event(1)], Vec::new());
        attacker_request.idempotency_key = key.clone();
        attacker_request.command_id = VidaCommandRef("command-attacker".to_string());
        attacker_request.causation_id = Some(attacker_request.command_id.clone());
        let error = journal
            .append(attacker_request)
            .expect_err("attacker append must not overwrite victim idempotency row");

        assert_eq!(
            error,
            TaskflowStateError::IdempotencyConflict("shared-key".to_string())
        );
        let command_ledger = journal
            .idempotency_record(&key)
            .expect("victim idempotency record should remain");
        assert_eq!(command_ledger.command_id, victim_command);
        assert_eq!(command_ledger.state, JournalIdempotencyState::Conflicted);
        assert_eq!(command_ledger.receipt_id, None);
        assert!(
            command_ledger
                .conflict_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("idempotency_payload_conflict"))
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            0
        );
    }

    #[test]
    fn corrupt_payload_blocker_serializes_operator_json() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let blocker = classify_redb_journal_error(
            &TaskflowStateError::Storage("json expected eof while decoding record".to_string()),
            &path,
        )
        .expect("corrupt payload blocker should classify");

        let payload = serde_json::to_value(&blocker).expect("blocker should serialize");

        assert_eq!(payload["code"], REDB_CORRUPT_PAYLOAD_BLOCKER_CODE);
        assert!(
            payload["next_action"]
                .as_str()
                .expect("next action")
                .contains(path.to_string_lossy().as_ref())
        );
        let operator_payload = redb_journal_blocker_operator_payload(&blocker, &path);
        assert_eq!(operator_payload["status"], "blocked");
        assert_eq!(
            operator_payload["blocker_codes"][0],
            REDB_CORRUPT_PAYLOAD_BLOCKER_CODE
        );
        assert_eq!(
            operator_payload["next_actions"][0],
            blocker.next_action.as_str()
        );
        assert_eq!(
            operator_payload["artifact_refs"]["journal_path"],
            path.to_string_lossy().as_ref()
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
        journal
            .record_projection_failure_at_cursor(
                JournalProjectionFailure {
                    projection_id: VidaProjectionRef("projection-1".to_string()),
                    stream_id: VidaStreamRef("stream-1".to_string()),
                    error: "projector failed".to_string(),
                },
                Some(VidaEventCursor("global-2".to_string())),
            )
            .expect("projection failure should persist");
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
        assert_eq!(health.projection_failure_count, 1);
        assert_eq!(health.artifact_count, 1);
        let failures = reopened
            .projection_failure_records()
            .expect("projection failure records");
        assert_eq!(failures.len(), 1);
        assert_eq!(
            failures[0].source_event_cursor,
            Some(VidaEventCursor("global-2".to_string()))
        );
        assert_eq!(failures[0].failure_kind, "projection_rebuild_failed");
        assert!(failures[0].repair_plan_ref.is_some());
        assert!(!failures[0].content_hash.is_empty());
        let operator_payload = redb_projection_health_operator_payload(&health, &failures);
        assert_eq!(operator_payload["status"], "blocked");
        assert_eq!(
            operator_payload["blocker_codes"][0],
            REDB_PROJECTION_FAILURE_BLOCKER_CODE
        );
        assert_eq!(
            operator_payload["artifact_refs"]["projection_failure_count"],
            1
        );
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
    fn redb_outbox_record_tracks_attempt_failure_and_restart() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append should pass");
        let pending = journal
            .outbox_effect_records()
            .expect("outbox records should read");
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0].source_event_cursor,
            Some(VidaEventCursor("global-1".to_string()))
        );
        assert_eq!(
            pending[0].source_stream_id,
            VidaStreamRef("stream-1".to_string())
        );
        assert_eq!(pending[0].attempt_count, 0);
        assert_eq!(pending[0].lifecycle_state, "pending");
        assert!(!pending[0].effect_hash.is_empty());

        let claimed = journal.claim_outbox_batch("worker-1", 1);
        assert_eq!(claimed.len(), 1);
        let claimed_id = claimed[0].outbox_id.clone();
        drop(journal);

        let mut reopened = RedbOperationalJournal::open(&path).expect("open journal");
        let durable_claim = reopened
            .outbox_effect_record(&claimed_id)
            .expect("claimed outbox read")
            .expect("claimed outbox row");
        assert_eq!(durable_claim.attempt_count, 1);
        assert_eq!(durable_claim.claimed_by.as_deref(), Some("worker-1"));
        assert_eq!(durable_claim.lifecycle_state, "claimed");

        reopened
            .mark_outbox_failed(&claimed_id, "transport failure".to_string())
            .expect("mark failed should pass");
        let failed = reopened
            .outbox_effect_record(&claimed_id)
            .expect("failed outbox read")
            .expect("failed outbox row");
        assert_eq!(failed.attempt_count, 1);
        assert_eq!(failed.lifecycle_state, "failed");
        assert_eq!(failed.failure_reason.as_deref(), Some("transport failure"));
        assert!(matches!(
            failed.state,
            JournalOutboxState::Failed { ref reason } if reason == "transport failure"
        ));
        let health = reopened.health_status().expect("health should pass");
        assert_eq!(health.outbox_failed_count, 1);
        assert_eq!(health.outbox_claimed_count, 0);
        drop(reopened);

        let reopened_after_failure = RedbOperationalJournal::open(&path).expect("reopen journal");
        let failed_after_restart = reopened_after_failure
            .outbox_effect_record(&claimed_id)
            .expect("failed outbox read after restart")
            .expect("failed outbox row after restart");
        assert_eq!(failed_after_restart.attempt_count, 1);
        assert_eq!(failed_after_restart.lifecycle_state, "failed");
        assert_eq!(
            failed_after_restart.failure_reason.as_deref(),
            Some("transport failure")
        );
    }

    #[test]
    fn redb_outbox_retry_schedule_requeues_failed_effect_and_tracks_attempts() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append should pass");
        let first = journal.claim_outbox_batch("worker-1", 1);
        let outbox_id = first[0].outbox_id.clone();
        journal
            .mark_outbox_failed(&outbox_id, "transport failure".to_string())
            .expect("first failure");
        journal
            .schedule_outbox_retry(&outbox_id, Some("2026-06-23T01:00:00Z".to_string()))
            .expect("schedule retry");
        drop(journal);

        let mut reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let scheduled = reopened
            .outbox_effect_record(&outbox_id)
            .expect("scheduled outbox read")
            .expect("scheduled outbox row");
        assert_eq!(scheduled.lifecycle_state, "pending");
        assert_eq!(
            scheduled.retry_after.as_deref(),
            Some("2026-06-23T01:00:00Z")
        );
        assert_eq!(scheduled.attempt_count, 1);

        let second = reopened.claim_outbox_batch("worker-2", 1);
        assert_eq!(second.len(), 1);
        let claimed_again = reopened
            .outbox_effect_record(&outbox_id)
            .expect("claimed outbox read")
            .expect("claimed outbox row");
        assert_eq!(claimed_again.attempt_count, 2);
        assert_eq!(claimed_again.retry_after, None);
        assert!(matches!(
            claimed_again.state,
            JournalOutboxState::Claimed { ref consumer_id } if consumer_id == "worker-2"
        ));
    }

    #[test]
    fn artifact_hash_reconciliation_reports_sha256_pass_and_mismatch() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let artifact_dir = dir.path().join("artifacts");
        fs::create_dir_all(&artifact_dir).expect("create artifact dir");
        let artifact_path = artifact_dir.join("snapshot.json");
        let initial_content = b"{\"status\":\"ready\"}\n";
        fs::write(&artifact_path, initial_content).expect("write artifact");

        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append should pass");
        journal.index_artifact(JournalArtifactRecord {
            artifact_ref: taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
            content_hash: "sha256:682c055ddf7d0afe32b7b2646e1635ab3c83f65884a37aecdc8549e7031a3417"
                .to_string(),
            path: "artifacts/snapshot.json".to_string(),
        });

        let indexed = journal
            .artifact_index_record(&taskflow_contracts::VidaArtifactRef(
                "artifact-1".to_string(),
            ))
            .expect("artifact index read")
            .expect("artifact index row");
        assert_eq!(
            indexed.producer_event_cursor,
            Some(VidaEventCursor("global-1".to_string()))
        );
        assert_eq!(indexed.lifecycle_state, "indexed");
        assert_eq!(indexed.reconciliation_status, "pending_reconciliation");
        assert!(!indexed.path_hash.is_empty());

        let pass = journal
            .reconcile_artifact_hashes(dir.path())
            .expect("artifact reconcile pass");
        assert_eq!(pass.len(), 1);
        assert_eq!(pass[0].status, "pass");
        assert_eq!(
            pass[0].computed_content_hash.as_deref(),
            Some(indexed.content_hash.as_str())
        );
        let receipt = journal
            .reconcile_and_materialize_artifact(
                &taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
                dir.path(),
            )
            .expect("materialization receipt should pass");
        assert_eq!(receipt.status, "materialized");
        assert_eq!(receipt.path, "artifacts/snapshot.json");
        assert_eq!(
            receipt.source_event_cursor,
            VidaEventCursor("global-1".to_string())
        );
        assert_eq!(receipt.schema_version, "1");

        fs::write(&artifact_path, b"{\"status\":\"changed\"}\n").expect("modify artifact");
        let mismatch = journal
            .reconcile_artifact_hashes(dir.path())
            .expect("artifact reconcile mismatch");
        assert_eq!(mismatch[0].status, "mismatch");
        assert_ne!(
            mismatch[0].computed_content_hash.as_deref(),
            Some(indexed.content_hash.as_str())
        );
        let receipt_error = journal
            .reconcile_and_materialize_artifact(
                &taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
                dir.path(),
            )
            .expect_err("mismatched artifact must not materialize");
        assert!(receipt_error.to_string().contains("is not hash-reconciled"));
    }

    #[test]
    fn artifact_materialization_requires_source_cursor() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let artifact_dir = dir.path().join("artifacts");
        fs::create_dir_all(&artifact_dir).expect("create artifact dir");
        fs::write(artifact_dir.join("snapshot.json"), b"{}\n").expect("write artifact");

        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal.index_artifact(JournalArtifactRecord {
            artifact_ref: taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
            content_hash: "sha256:ca3d163bab055381827226140568f3bef7eaac187cebd76878e0b63e9e442356"
                .to_string(),
            path: "artifacts/snapshot.json".to_string(),
        });

        let error = journal
            .reconcile_and_materialize_artifact(
                &taskflow_contracts::VidaArtifactRef("artifact-1".to_string()),
                dir.path(),
            )
            .expect_err("artifact without source cursor must not materialize");

        assert!(error.to_string().contains("missing source event cursor"));
    }

    #[test]
    #[cfg(unix)]
    fn artifact_hash_reconciliation_rejects_symlink_escape() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let artifact_dir = dir.path().join("artifacts");
        fs::create_dir_all(&artifact_dir).expect("create artifact dir");
        let outside = dir
            .path()
            .parent()
            .expect("temp parent")
            .join(format!("outside-secret-{}.txt", std::process::id()));
        fs::write(
            &outside,
            b"outside secret
",
        )
        .expect("write outside secret");
        let symlink_path = artifact_dir.join("secret_link.txt");
        std::os::unix::fs::symlink(&outside, &symlink_path).expect("create symlink");

        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("append should pass");
        journal.index_artifact(JournalArtifactRecord {
            artifact_ref: taskflow_contracts::VidaArtifactRef("artifact-symlink".to_string()),
            content_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            path: "artifacts/secret_link.txt".to_string(),
        });

        let reconciled = journal
            .reconcile_artifact_hashes(dir.path())
            .expect("artifact reconcile symlink");
        assert_eq!(reconciled.len(), 1);
        assert_eq!(reconciled[0].status, "out_of_root");
        assert_eq!(reconciled[0].computed_content_hash, None);
        assert_eq!(
            reconciled[0].detail.as_deref(),
            Some("artifact path must be a regular file under the project root")
        );

        fs::remove_file(&outside).expect("remove outside secret");
    }

    #[test]
    fn sha256_hash_helper_matches_known_digest() {
        assert_eq!(
            super::sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn old_outbox_and_artifact_rows_remain_readable_after_record_upgrade() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .write_record(
                OUTBOX_TABLE,
                "outbox-old",
                &taskflow_state::JournalOutboxRecord {
                    outbox_id: VidaEventRef("outbox-old".to_string()),
                    effect: effect("effect-old"),
                    state: JournalOutboxState::Pending,
                },
            )
            .expect("old outbox row should write");
        journal
            .write_record(
                ARTIFACT_TABLE,
                "artifact-old",
                &JournalArtifactRecord {
                    artifact_ref: taskflow_contracts::VidaArtifactRef("artifact-old".to_string()),
                    content_hash: "hash-old".to_string(),
                    path: "artifacts/old.json".to_string(),
                },
            )
            .expect("old artifact row should write");

        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let outbox = reopened
            .outbox_effect_record(&VidaEventRef("outbox-old".to_string()))
            .expect("old outbox row should decode")
            .expect("old outbox row");
        assert_eq!(outbox.lifecycle_state, "pending");
        assert_eq!(outbox.source_event_cursor, None);
        assert!(!outbox.effect_hash.is_empty());

        let artifact = reopened
            .artifact_index_record(&taskflow_contracts::VidaArtifactRef(
                "artifact-old".to_string(),
            ))
            .expect("old artifact row should decode")
            .expect("old artifact row");
        assert_eq!(artifact.content_hash, "hash-old");
        assert_eq!(artifact.producer_event_cursor, None);
        assert_eq!(artifact.reconciliation_status, "pending_reconciliation");
        assert!(!artifact.path_hash.is_empty());
    }

    #[test]
    fn health_status_counts_old_projection_rows_with_compat_decode() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .write_record(
                super::PROJECTION_CHECKPOINT_TABLE,
                "projection-old",
                &VidaProjectionCheckpoint {
                    projection_id: VidaProjectionRef("projection-old".to_string()),
                    stream_id: VidaStreamRef("stream-1".to_string()),
                    event_cursor: VidaEventCursor("global-7".to_string()),
                    stream_version: VidaStreamVersion(7),
                    updated_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
                },
            )
            .expect("old checkpoint row should write");
        journal
            .write_record(
                super::PROJECTION_FAILURE_TABLE,
                "projection-old:stream-1:error",
                &JournalProjectionFailure {
                    projection_id: VidaProjectionRef("projection-old".to_string()),
                    stream_id: VidaStreamRef("stream-1".to_string()),
                    error: "old projector error".to_string(),
                },
            )
            .expect("old failure row should write");
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let health = reopened
            .health_status()
            .expect("health should compat decode");
        assert_eq!(health.projection_checkpoint_count, 1);
        assert_eq!(health.projection_failure_count, 1);
    }

    #[test]
    fn redb_shadow_imports_file_snapshot_and_proves_parity_after_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let snapshot_path = dir.path().join("taskflow-snapshot.json");
        let snapshot = taskflow_snapshot_fixture();
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot).expect("write snapshot");
        let file_snapshot =
            taskflow_state_fs::read_snapshot(&snapshot_path).expect("read snapshot");

        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        let imported = journal
            .replace_taskflow_snapshot_from_file(&snapshot_path)
            .expect("redb shadow import should pass");
        assert_eq!(imported.task_count, 2);
        assert_eq!(imported.dependency_count, 1);
        assert_eq!(imported.source_kind, "file_snapshot");
        assert_eq!(imported.source_ref, snapshot_path.display().to_string());
        assert!(imported.source_hash.starts_with("sha256:"));
        assert_eq!(
            imported.normalization_finding,
            "normalized_sorted_unique_storage_keys"
        );
        let preview = journal.preview_taskflow_snapshot_import(&file_snapshot);
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.quarantine_count, 0);
        assert_eq!(
            preview.normalization_findings.len(),
            file_snapshot.tasks.len() + file_snapshot.dependencies.len()
        );
        assert!(preview.normalization_findings.iter().all(|finding| {
            finding.status == "normalized"
                && !finding.source_hash.is_empty()
                && finding.source_ref == "taskflow_state_fs::TaskSnapshot"
                && finding.source_kind == "memory_snapshot"
        }));
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let exported = reopened
            .export_taskflow_snapshot()
            .expect("redb shadow export should pass");
        assert_eq!(
            serde_json::to_value(&exported.tasks).expect("exported tasks should serialize"),
            serde_json::to_value(&file_snapshot.tasks).expect("file tasks should serialize")
        );
        assert_eq!(
            serde_json::to_value(&exported.dependencies)
                .expect("exported dependencies should serialize"),
            serde_json::to_value(&file_snapshot.dependencies)
                .expect("file dependencies should serialize")
        );
        let parity = reopened
            .taskflow_snapshot_parity(&file_snapshot)
            .expect("redb parity should pass");
        assert_eq!(parity.status, "pass");
        assert_eq!(parity.task_hash, imported.task_hash);
        assert_eq!(parity.dependency_hash, imported.dependency_hash);
        assert_eq!(
            parity.normalization_finding,
            "normalized_sorted_unique_storage_keys"
        );
        let metadata = reopened
            .taskflow_snapshot_source_metadata()
            .expect("source metadata should read");
        assert_eq!(
            metadata.len(),
            file_snapshot.tasks.len() + file_snapshot.dependencies.len()
        );
        assert!(metadata.iter().all(|finding| {
            finding.status == "normalized"
                && finding.source_kind == "file_snapshot"
                && finding.source_ref == snapshot_path.display().to_string()
                && !finding.source_hash.is_empty()
                && !finding.finding.is_empty()
        }));
    }

    #[test]
    fn redb_shadow_import_is_idempotent_for_repeated_snapshot() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let snapshot_path = dir.path().join("taskflow-snapshot.json");
        let snapshot = taskflow_snapshot_fixture();
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot).expect("write snapshot");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");

        let first = journal
            .replace_taskflow_snapshot_from_file(&snapshot_path)
            .expect("first import should pass");
        let second = journal
            .replace_taskflow_snapshot_from_file(&snapshot_path)
            .expect("second import should pass");
        let exported = journal
            .export_taskflow_snapshot()
            .expect("export should pass");

        assert_eq!(second.task_count, first.task_count);
        assert_eq!(second.dependency_count, first.dependency_count);
        assert_eq!(second.task_hash, first.task_hash);
        assert_eq!(second.dependency_hash, first.dependency_hash);
        assert_eq!(second.source_hash, first.source_hash);
        assert_eq!(second.normalization_finding, first.normalization_finding);
        let metadata = journal
            .taskflow_snapshot_source_metadata()
            .expect("source metadata should read");
        assert_eq!(
            metadata.len(),
            snapshot.tasks.len() + snapshot.dependencies.len()
        );
        assert_eq!(
            serde_json::to_value(&exported.tasks).expect("exported tasks should serialize"),
            serde_json::to_value(&snapshot.tasks).expect("snapshot tasks should serialize")
        );
        assert_eq!(
            serde_json::to_value(&exported.dependencies)
                .expect("exported dependencies should serialize"),
            serde_json::to_value(&snapshot.dependencies)
                .expect("snapshot dependencies should serialize")
        );
    }

    #[test]
    fn redb_shadow_parity_reports_mismatch_for_different_snapshot() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .replace_taskflow_snapshot(&taskflow_snapshot_fixture())
            .expect("import should pass");
        let expected = taskflow_state_fs::TaskSnapshot {
            tasks: vec![TaskRecord::new(
                taskflow_core::TaskId::new("vida-root"),
                "Root",
                taskflow_core::IssueType::Epic,
            )],
            dependencies: Vec::new(),
        };

        let parity = journal
            .taskflow_snapshot_parity(&expected)
            .expect("parity check should return mismatch");

        assert_eq!(parity.status, "mismatch");
        assert_eq!(parity.task_count, 2);
        assert_eq!(parity.dependency_count, 1);
    }

    #[test]
    fn redb_shadow_replace_removes_stale_snapshot_rows() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        let first = taskflow_snapshot_fixture();
        journal
            .replace_taskflow_snapshot(&first)
            .expect("first import should pass");
        let replacement = taskflow_state_fs::TaskSnapshot {
            tasks: vec![TaskRecord::new(
                taskflow_core::TaskId::new("vida-root"),
                "Root",
                taskflow_core::IssueType::Epic,
            )],
            dependencies: Vec::new(),
        };

        let parity = journal
            .replace_taskflow_snapshot(&replacement)
            .expect("replacement import should pass");
        let exported = journal
            .export_taskflow_snapshot()
            .expect("replacement export should pass");

        assert_eq!(parity.task_count, 1);
        assert_eq!(parity.dependency_count, 0);
        assert_eq!(exported.tasks.len(), 1);
        assert_eq!(exported.tasks[0].id.as_str(), "vida-root");
        assert!(exported.dependencies.is_empty());
    }

    #[test]
    fn redb_shadow_replace_rejects_duplicate_task_ids() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        let duplicate = taskflow_state_fs::TaskSnapshot {
            tasks: vec![
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-duplicate"),
                    "First",
                    taskflow_core::IssueType::Task,
                ),
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-duplicate"),
                    "Second",
                    taskflow_core::IssueType::Task,
                ),
            ],
            dependencies: Vec::new(),
        };
        let preview = journal.preview_taskflow_snapshot_import(&duplicate);
        assert_eq!(preview.status, "quarantined");
        assert_eq!(preview.quarantine_count, 1);
        assert!(preview.normalization_findings.iter().any(|finding| {
            finding.entity_kind == "task"
                && finding.entity_ref == "vida-duplicate"
                && finding.status == "quarantined"
                && finding
                    .finding
                    .contains("taskflow snapshot import rejected duplicate task id")
        }));

        let error = journal
            .replace_taskflow_snapshot(&duplicate)
            .expect_err("duplicate task ids must be rejected");

        assert!(
            error
                .to_string()
                .contains("taskflow snapshot import rejected duplicate task id: vida-duplicate")
        );
        assert!(
            journal
                .export_taskflow_snapshot()
                .expect("export should pass")
                .tasks
                .is_empty()
        );
    }

    #[test]
    fn redb_shadow_replace_rejects_colliding_dependency_keys() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        let colliding = taskflow_state_fs::TaskSnapshot {
            tasks: vec![
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-a\u{1f}b"),
                    "A",
                    taskflow_core::IssueType::Task,
                ),
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-c"),
                    "C",
                    taskflow_core::IssueType::Task,
                ),
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-a"),
                    "A delimiter",
                    taskflow_core::IssueType::Task,
                ),
                TaskRecord::new(
                    taskflow_core::TaskId::new("b\u{1f}vida-c"),
                    "B delimiter",
                    taskflow_core::IssueType::Task,
                ),
            ],
            dependencies: vec![
                DependencyEdge {
                    issue_id: taskflow_core::TaskId::new("vida-a\u{1f}b"),
                    depends_on_id: taskflow_core::TaskId::new("vida-c"),
                    dependency_type: "blocks".to_string(),
                },
                DependencyEdge {
                    issue_id: taskflow_core::TaskId::new("vida-a"),
                    depends_on_id: taskflow_core::TaskId::new("b\u{1f}vida-c"),
                    dependency_type: "blocks".to_string(),
                },
            ],
        };
        let preview = journal.preview_taskflow_snapshot_import(&colliding);
        assert_eq!(preview.status, "quarantined");
        assert_eq!(preview.quarantine_count, 1);
        assert!(preview.normalization_findings.iter().any(|finding| {
            finding.entity_kind == "dependency"
                && finding.status == "quarantined"
                && finding
                    .finding
                    .contains("taskflow snapshot import rejected duplicate dependency key")
        }));

        let error = journal
            .replace_taskflow_snapshot(&colliding)
            .expect_err("colliding dependency storage keys must be rejected");

        assert!(
            error
                .to_string()
                .contains("taskflow snapshot import rejected duplicate dependency key")
        );
        assert!(
            journal
                .export_taskflow_snapshot()
                .expect("export should pass")
                .dependencies
                .is_empty()
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
        let checkpoint_record = reopened
            .projection_checkpoint_record(&VidaProjectionRef("projection-1".to_string()))
            .expect("checkpoint record read")
            .expect("checkpoint record");
        assert_eq!(
            checkpoint_record.last_global_cursor,
            VidaEventCursor("global-2".to_string())
        );
        assert_eq!(checkpoint_record.last_stream_version, VidaStreamVersion(2));
        assert_eq!(checkpoint_record.schema_version, "1");
        assert!(!checkpoint_record.input_hash.is_empty());
        assert!(!checkpoint_record.output_hash.is_empty());
    }

    #[test]
    fn projection_checkpoint_rejects_out_of_order_older_event_sequence() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal.record_projection_checkpoint(projection_checkpoint(4));
        journal.record_projection_checkpoint(projection_checkpoint(2));

        let checkpoint = journal
            .projection_checkpoint_record(&VidaProjectionRef("projection-1".to_string()))
            .expect("checkpoint record read")
            .expect("checkpoint record");
        assert_eq!(
            checkpoint.last_global_cursor,
            VidaEventCursor("global-4".to_string())
        );
        assert_eq!(checkpoint.last_stream_version, VidaStreamVersion(4));
    }

    #[test]
    fn projection_read_barrier_exposes_as_of_cursor_and_blocks_lag() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal.record_projection_checkpoint(projection_checkpoint(3));

        let pass = journal
            .projection_read_barrier(
                &VidaProjectionRef("projection-1".to_string()),
                &VidaEventCursor("global-3".to_string()),
            )
            .expect("barrier should read");
        assert_eq!(pass.status, "pass");
        assert_eq!(
            pass.as_of_event_cursor,
            Some(VidaEventCursor("global-3".to_string()))
        );
        assert_eq!(pass.blocker_code, None);

        let blocked = journal
            .projection_read_barrier(
                &VidaProjectionRef("projection-1".to_string()),
                &VidaEventCursor("global-4".to_string()),
            )
            .expect("barrier should read");
        assert_eq!(blocked.status, "blocked");
        assert_eq!(
            blocked.as_of_event_cursor,
            Some(VidaEventCursor("global-3".to_string()))
        );
        assert_eq!(
            blocked.blocker_code,
            Some("projection_not_caught_up".to_string())
        );

        let malformed = journal
            .projection_read_barrier(
                &VidaProjectionRef("projection-1".to_string()),
                &VidaEventCursor("cursor-three".to_string()),
            )
            .expect_err("malformed required cursor must fail closed");
        assert!(
            malformed
                .to_string()
                .contains("projection barrier cursor is malformed")
        );
    }

    #[test]
    fn old_projection_rows_remain_readable_after_record_upgrade() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let journal = RedbOperationalJournal::create(&path).expect("create journal");
        journal
            .write_record(
                super::PROJECTION_CHECKPOINT_TABLE,
                "projection-old",
                &VidaProjectionCheckpoint {
                    projection_id: VidaProjectionRef("projection-old".to_string()),
                    stream_id: VidaStreamRef("stream-1".to_string()),
                    event_cursor: VidaEventCursor("global-7".to_string()),
                    stream_version: VidaStreamVersion(7),
                    updated_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
                },
            )
            .expect("old checkpoint row should write");
        journal
            .write_record(
                super::PROJECTION_FAILURE_TABLE,
                "projection-old:stream-1:error",
                &JournalProjectionFailure {
                    projection_id: VidaProjectionRef("projection-old".to_string()),
                    stream_id: VidaStreamRef("stream-1".to_string()),
                    error: "old projector error".to_string(),
                },
            )
            .expect("old failure row should write");

        let checkpoint = journal
            .projection_checkpoint_record(&VidaProjectionRef("projection-old".to_string()))
            .expect("checkpoint row should decode")
            .expect("checkpoint row");
        assert_eq!(
            checkpoint.last_global_cursor,
            VidaEventCursor("global-7".to_string())
        );
        assert_eq!(checkpoint.last_stream_version, VidaStreamVersion(7));
        assert!(!checkpoint.input_hash.is_empty());
        assert!(!checkpoint.output_hash.is_empty());

        let failures = journal
            .projection_failure_records()
            .expect("failure rows should decode");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].failure_message, "old projector error");
        assert_eq!(failures[0].source_event_cursor, None);
        assert!(!failures[0].content_hash.is_empty());
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

    fn taskflow_snapshot_fixture() -> taskflow_state_fs::TaskSnapshot {
        taskflow_state_fs::TaskSnapshot {
            tasks: vec![
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-child"),
                    "Child",
                    taskflow_core::IssueType::Task,
                ),
                TaskRecord::new(
                    taskflow_core::TaskId::new("vida-root"),
                    "Root",
                    taskflow_core::IssueType::Epic,
                ),
            ],
            dependencies: vec![DependencyEdge {
                issue_id: taskflow_core::TaskId::new("vida-child"),
                depends_on_id: taskflow_core::TaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
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
