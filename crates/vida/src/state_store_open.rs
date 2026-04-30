use super::*;
use fs2::FileExt;
use std::fs::OpenOptions;

const AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS: u64 = 25;
const AUTHORITATIVE_DATASTORE_LOCK_MAX_WAIT_MS: u64 = 30_000;
const AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT: usize = (AUTHORITATIVE_DATASTORE_LOCK_MAX_WAIT_MS
    / AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS)
    as usize;
const AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT: usize = AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT;
const AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS: u64 = AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS;
const READ_ONLY_OPEN_RETRY_COUNT: usize = 800;
const READ_ONLY_OPEN_RETRY_DELAY_MS: u64 = 25;

struct AuthoritativeOpenGuard {
    file: std::fs::File,
}

impl AuthoritativeOpenGuard {
    async fn acquire(root: &Path) -> Result<Self, StateStoreError> {
        let guard_path = root.join(".vida-authoritative-open.guard");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&guard_path)?;
        for attempt in 0..AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT {
            match file.try_lock_exclusive() {
                Ok(()) => {
                    return Ok(Self { file });
                }
                Err(error) if Self::is_lock_contention_error(&error) => {
                    if attempt + 1 < AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS,
                        ))
                        .await;
                        continue;
                    }
                    return Err(StateStoreError::Io(error));
                }
                Err(error) => return Err(StateStoreError::Io(error)),
            }
        }

        Err(StateStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out while waiting for authoritative datastore access serialization guard",
        )))
    }

    fn is_lock_contention_error(error: &std::io::Error) -> bool {
        matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::Interrupted
        ) || error
            .raw_os_error()
            .is_some_and(|code| code == libc::EWOULDBLOCK || code == libc::EAGAIN)
    }
}

impl Drop for AuthoritativeOpenGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub(super) fn state_schema_document() -> String {
    let storage_schema = SurrealStoreTarget::new(DEFAULT_STATE_DIR).bootstrap_schema_document();
    format!("{storage_schema}\n\n{INSTRUCTION_STATE_SCHEMA}")
}

impl StateStore {
    pub(crate) fn error_is_lock_contention(error: &StateStoreError) -> bool {
        Self::message_is_lock_contention(&error.to_string())
    }

    pub(crate) fn message_is_lock_contention(message: &str) -> bool {
        message.contains("LOCK")
            || message.contains("lock")
            || message.contains("Resource temporarily unavailable")
    }

    async fn sanitize_legacy_task_execution_semantics(&self) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query("UPDATE task SET execution_semantics = {} WHERE execution_semantics = NONE;")
            .await?;
        Ok(())
    }

    async fn sanitize_legacy_task_planner_metadata(&self) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query("UPDATE task SET planner_metadata = {} WHERE planner_metadata = NONE;")
            .await?;
        let _ = self
            .db
            .query(
                "UPDATE task SET planner_metadata.owned_paths = [] WHERE planner_metadata != NONE AND planner_metadata.owned_paths = NONE;",
            )
            .await?;
        let _ = self
            .db
            .query(
                "UPDATE task SET planner_metadata.acceptance_targets = [] WHERE planner_metadata != NONE AND planner_metadata.acceptance_targets = NONE;",
            )
            .await?;
        let _ = self
            .db
            .query(
                "UPDATE task SET planner_metadata.proof_targets = [] WHERE planner_metadata != NONE AND planner_metadata.proof_targets = NONE;",
            )
            .await?;
        Ok(())
    }

    async fn open_with_authoritative_lock_retry<F, Fut>(
        root: PathBuf,
        mut open_once: F,
    ) -> Result<Self, StateStoreError>
    where
        F: FnMut(PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<Self, StateStoreError>>,
    {
        for attempt in 0..AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT {
            match open_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error))
                    if attempt + 1 < AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT =>
                {
                    if Self::message_is_lock_contention(&error.to_string()) {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS,
                        ))
                        .await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        open_once(root).await
    }

    pub async fn open(root: PathBuf) -> Result<Self, StateStoreError> {
        fs::create_dir_all(&root)?;
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;
        Self::open_with_authoritative_lock_retry(root, Self::open_once).await
    }

    pub async fn open_existing(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;
        Self::open_with_authoritative_lock_retry(root, Self::open_existing_once).await
    }

    pub async fn open_existing_read_only(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }

        for attempt in 0..READ_ONLY_OPEN_RETRY_COUNT {
            match Self::open_existing_read_only_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt + 1 < READ_ONLY_OPEN_RETRY_COUNT => {
                    if Self::message_is_lock_contention(&error.to_string()) {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            READ_ONLY_OPEN_RETRY_DELAY_MS,
                        ))
                        .await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_existing_read_only_once(root).await
    }

    async fn open_existing_once(root: PathBuf) -> Result<Self, StateStoreError> {
        Self::open_once(root).await
    }

    async fn open_existing_read_only_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(root.clone()).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        Ok(Self { db, root })
    }

    async fn open_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(root.clone()).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;

        let store = Self { db, root };
        store.sanitize_legacy_task_execution_semantics().await?;
        store.sanitize_legacy_task_planner_metadata().await?;
        store.expire_stale_scheduler_dispatch_reservations().await?;
        store.ensure_minimal_authoritative_state_spine().await?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn read_only_open_bypasses_authoritative_open_guard() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-read-only-open-guard-{}-{nanos}",
            std::process::id()
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        drop(store);

        let _guard = AuthoritativeOpenGuard::acquire(&root)
            .await
            .expect("hold authoritative guard");
        let read_only_open = tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            StateStore::open_existing_read_only(root.clone()),
        )
        .await
        .expect("read-only open should not wait for authoritative guard");

        assert!(read_only_open.is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn read_only_open_waits_for_concurrent_read_lock() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-read-only-open-contention-{}-{nanos}",
            std::process::id()
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        drop(store);

        let first_reader = StateStore::open_existing_read_only(root.clone())
            .await
            .expect("first read-only store should open");
        let second_root = root.clone();
        let second_reader =
            tokio::spawn(async move { StateStore::open_existing_read_only(second_root).await });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(first_reader);

        let second_result = tokio::time::timeout(std::time::Duration::from_secs(5), second_reader)
            .await
            .expect("second read-only open should wait for the first read lock")
            .expect("second read-only task should not panic");
        assert!(second_result.is_ok());
        let _ = fs::remove_dir_all(&root);
    }
}
