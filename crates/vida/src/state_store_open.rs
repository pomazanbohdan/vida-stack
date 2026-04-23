use super::*;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;

const AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT: usize = 80;
const AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS: u64 = 25;

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
            let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if result == 0 {
                return Ok(Self { file });
            }
            let error = std::io::Error::last_os_error();
            let would_block = matches!(
                error.raw_os_error(),
                Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN
            );
            if would_block && attempt + 1 < AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT {
                tokio::time::sleep(std::time::Duration::from_millis(
                    AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS,
                ))
                .await;
                continue;
            }
            return Err(StateStoreError::Io(error));
        }

        Err(StateStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out while waiting for authoritative datastore access serialization guard",
        )))
    }
}

impl Drop for AuthoritativeOpenGuard {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

pub(super) fn state_schema_document() -> String {
    let storage_schema = SurrealStoreTarget::new(DEFAULT_STATE_DIR).bootstrap_schema_document();
    format!("{storage_schema}\n\n{INSTRUCTION_STATE_SCHEMA}")
}

impl StateStore {
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

    pub async fn open(root: PathBuf) -> Result<Self, StateStoreError> {
        fs::create_dir_all(&root)?;
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;

        for attempt in 0..80 {
            match Self::open_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_once(root).await
    }

    pub async fn open_existing(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;

        for attempt in 0..80 {
            match Self::open_existing_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_existing_once(root).await
    }

    pub async fn open_existing_read_only(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }

        for attempt in 0..80 {
            match Self::open_existing_read_only_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
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
        store.ensure_minimal_authoritative_state_spine().await?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
