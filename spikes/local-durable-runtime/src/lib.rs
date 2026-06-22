use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;

const EVENTS: TableDefinition<u64, &str> = TableDefinition::new("events");
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");
const IDEMPOTENCY: TableDefinition<&str, &str> = TableDefinition::new("idempotency");
const OUTBOX: TableDefinition<u64, &str> = TableDefinition::new("outbox");

#[derive(Debug, PartialEq, Eq)]
pub enum JournalError {
    VersionMismatch { expected: u64, actual: u64 },
    IdempotencyConflict { key: String },
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::VersionMismatch { expected, actual } => {
                write!(f, "expected journal version {expected}, got {actual}")
            }
            JournalError::IdempotencyConflict { key } => {
                write!(
                    f,
                    "idempotency key {key} was reused with a different payload"
                )
            }
        }
    }
}

impl Error for JournalError {}

pub fn open(path: impl AsRef<Path>) -> Result<Database, Box<dyn Error>> {
    Ok(Database::create(path)?)
}

pub fn append_event(
    db: &Database,
    expected_version: u64,
    payload: &str,
) -> Result<u64, Box<dyn Error>> {
    let tx = db.begin_write()?;
    let actual = current_version_in(&tx)?;
    if actual != expected_version {
        return Err(Box::new(JournalError::VersionMismatch {
            expected: expected_version,
            actual,
        }));
    }
    let next = actual + 1;
    {
        let mut events = tx.open_table(EVENTS)?;
        events.insert(next, payload)?;
    }
    {
        let mut meta = tx.open_table(META)?;
        meta.insert("version", next)?;
    }
    tx.commit()?;
    Ok(next)
}

pub fn append_event_and_queue_effect(
    db: &Database,
    expected_version: u64,
    payload: &str,
    effect_id: u64,
    effect_payload: &str,
) -> Result<u64, Box<dyn Error>> {
    let tx = db.begin_write()?;
    let actual = current_version_in(&tx)?;
    if actual != expected_version {
        return Err(Box::new(JournalError::VersionMismatch {
            expected: expected_version,
            actual,
        }));
    }
    let next = actual + 1;
    {
        let mut events = tx.open_table(EVENTS)?;
        events.insert(next, payload)?;
    }
    {
        let mut outbox = tx.open_table(OUTBOX)?;
        outbox.insert(effect_id, effect_payload)?;
    }
    {
        let mut meta = tx.open_table(META)?;
        meta.insert("version", next)?;
    }
    tx.commit()?;
    Ok(next)
}

pub fn replay_events(db: &Database) -> Result<Vec<(u64, String)>, Box<dyn Error>> {
    let tx = db.begin_read()?;
    let table = tx.open_table(EVENTS)?;
    let mut rows = Vec::new();
    for item in table.iter()? {
        let (version, payload) = item?;
        rows.push((version.value(), payload.value().to_owned()));
    }
    Ok(rows)
}

pub fn record_idempotent_response(
    db: &Database,
    key: &str,
    payload: &str,
    response: &str,
) -> Result<String, Box<dyn Error>> {
    let payload_hash = stable_hash(payload);
    let record = format!("{payload_hash}:{response}");
    let tx = db.begin_write()?;
    {
        let mut table = tx.open_table(IDEMPOTENCY)?;
        if let Some(existing) = table.get(key)? {
            let existing = existing.value().to_owned();
            if let Some((stored_hash, stored_response)) = existing.split_once(':') {
                if stored_hash == payload_hash {
                    return Ok(stored_response.to_owned());
                }
            }
            return Err(Box::new(JournalError::IdempotencyConflict {
                key: key.to_owned(),
            }));
        }
        table.insert(key, record.as_str())?;
    }
    tx.commit()?;
    Ok(response.to_owned())
}

pub fn pending_effects(db: &Database) -> Result<Vec<(u64, String)>, Box<dyn Error>> {
    let tx = db.begin_read()?;
    let table = tx.open_table(OUTBOX)?;
    let mut rows = Vec::new();
    for item in table.iter()? {
        let (id, payload) = item?;
        rows.push((id.value(), payload.value().to_owned()));
    }
    Ok(rows)
}

pub fn complete_effect(db: &Database, effect_id: u64) -> Result<(), Box<dyn Error>> {
    let tx = db.begin_write()?;
    {
        let mut table = tx.open_table(OUTBOX)?;
        table.remove(effect_id)?;
    }
    tx.commit()?;
    Ok(())
}

fn current_version_in(tx: &redb::WriteTransaction) -> Result<u64, Box<dyn Error>> {
    let table = tx.open_table(META)?;
    Ok(table.get("version")?.map(|v| v.value()).unwrap_or(0))
}

fn stable_hash(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn append_with_expected_version_and_replay_ordered_events() {
        let path = db_path("append_replay");
        let db = open(&path).unwrap();

        assert_eq!(append_event(&db, 0, "workflow.started").unwrap(), 1);
        assert_eq!(append_event(&db, 1, "workflow.dispatched").unwrap(), 2);

        assert_eq!(
            replay_events(&db).unwrap(),
            vec![
                (1, "workflow.started".to_owned()),
                (2, "workflow.dispatched".to_owned())
            ]
        );
        cleanup(path);
    }

    #[test]
    fn expected_version_mismatch_fails_closed() {
        let path = db_path("version_mismatch");
        let db = open(&path).unwrap();

        append_event(&db, 0, "workflow.started").unwrap();
        let err = append_event(&db, 0, "stale.write").unwrap_err();

        assert_eq!(
            err.downcast_ref::<JournalError>(),
            Some(&JournalError::VersionMismatch {
                expected: 0,
                actual: 1
            })
        );
        cleanup(path);
    }

    #[test]
    fn reopen_after_append_before_effect_execution_preserves_pending_outbox() {
        let path = db_path("reopen_outbox");
        {
            let db = open(&path).unwrap();
            append_event_and_queue_effect(&db, 0, "effect.queued", 42, "retry:notify").unwrap();
        }

        let reopened = open(&path).unwrap();

        assert_eq!(
            replay_events(&reopened).unwrap(),
            vec![(1, "effect.queued".to_owned())]
        );
        assert_eq!(
            pending_effects(&reopened).unwrap(),
            vec![(42, "retry:notify".to_owned())]
        );
        complete_effect(&reopened, 42).unwrap();
        assert!(pending_effects(&reopened).unwrap().is_empty());
        cleanup(path);
    }

    #[test]
    fn duplicate_idempotency_key_returns_previous_response() {
        let path = db_path("idempotency_same");
        let db = open(&path).unwrap();

        assert_eq!(
            record_idempotent_response(&db, "op-1", "payload-a", "response-a").unwrap(),
            "response-a"
        );
        assert_eq!(
            record_idempotent_response(&db, "op-1", "payload-a", "ignored-new-response").unwrap(),
            "response-a"
        );
        cleanup(path);
    }

    #[test]
    fn same_idempotency_key_with_different_payload_fails_closed() {
        let path = db_path("idempotency_conflict");
        let db = open(&path).unwrap();

        record_idempotent_response(&db, "op-1", "payload-a", "response-a").unwrap();
        let err = record_idempotent_response(&db, "op-1", "payload-b", "response-b").unwrap_err();

        assert_eq!(
            err.downcast_ref::<JournalError>(),
            Some(&JournalError::IdempotencyConflict {
                key: "op-1".to_owned()
            })
        );
        cleanup(path);
    }

    fn db_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("vida-redb-{name}-{nonce}.redb"))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
