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
const DATASTORE_CLOSE_SETTLE_MS: u64 = 250;

struct AuthoritativeOpenGuard {
    file: std::fs::File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessLiveness {
    Alive,
    Dead,
    Unknown,
}

#[cfg(target_os = "windows")]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        return ProcessLiveness::Alive;
    }
    let Ok(system_root) = std::env::var("SystemRoot") else {
        return ProcessLiveness::Unknown;
    };
    let tasklist_path = std::path::Path::new(&system_root)
        .join("System32")
        .join("tasklist.exe");
    let Ok(output) = std::process::Command::new(tasklist_path)
        .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
        .output()
    else {
        return ProcessLiveness::Unknown;
    };
    if !output.status.success() {
        return ProcessLiveness::Unknown;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let needle = format!(",\"{process_id}\",");
    if stdout.contains(&needle) {
        ProcessLiveness::Alive
    } else if !stdout
        .lines()
        .any(|line| line.trim_start().starts_with('"'))
    {
        ProcessLiveness::Dead
    } else {
        ProcessLiveness::Unknown
    }
}

#[cfg(target_os = "linux")]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        return ProcessLiveness::Alive;
    }
    if std::path::PathBuf::from(format!("/proc/{process_id}")).exists() {
        ProcessLiveness::Alive
    } else {
        ProcessLiveness::Dead
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        ProcessLiveness::Alive
    } else {
        ProcessLiveness::Unknown
    }
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
        ) || error.raw_os_error().is_some_and(|code| {
            code == libc::EWOULDBLOCK
                || code == libc::EAGAIN
                || (cfg!(windows) && matches!(code, 32 | 33))
        })
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
    fn reclaim_recoverable_authoritative_datastore_lock_marker_with_liveness(
        root: &Path,
        process_liveness: impl Fn(u32) -> ProcessLiveness,
    ) -> Result<bool, StateStoreError> {
        let lock_path = root.join("LOCK");
        let lock_text = match fs::read_to_string(&lock_path) {
            Ok(lock_text) => lock_text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) if AuthoritativeOpenGuard::is_lock_contention_error(&error) => {
                return Ok(false);
            }
            Err(error) => return Err(StateStoreError::Io(error)),
        };
        let Ok(pid) = lock_text.trim().parse::<u32>() else {
            return Ok(false);
        };
        let recoverable = process_liveness(pid) == ProcessLiveness::Dead;
        if !recoverable {
            return Ok(false);
        };
        match fs::remove_file(&lock_path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) if AuthoritativeOpenGuard::is_lock_contention_error(&error) => Ok(false),
            Err(error) => Err(StateStoreError::Io(error)),
        }
    }

    pub(crate) fn reclaim_recoverable_authoritative_datastore_lock_marker(
        root: &Path,
    ) -> Result<bool, StateStoreError> {
        Self::reclaim_recoverable_authoritative_datastore_lock_marker_with_liveness(
            root,
            local_process_liveness,
        )
    }

    pub(crate) fn reclaim_self_owned_failed_authoritative_datastore_lock_marker(
        root: &Path,
    ) -> Result<bool, StateStoreError> {
        Self::reclaim_recoverable_authoritative_datastore_lock_marker(root)
    }

    pub(crate) fn error_is_lock_contention(error: &StateStoreError) -> bool {
        match error {
            StateStoreError::Io(io_error) => {
                AuthoritativeOpenGuard::is_lock_contention_error(io_error)
            }
            StateStoreError::Db(db_error) => {
                Self::message_is_lock_contention(&db_error.to_string())
                    || Self::message_is_lock_contention(&format!("{db_error:?}"))
            }
            _ => false,
        }
    }

    pub(crate) fn message_is_lock_contention(message: &str) -> bool {
        let normalized = message.to_ascii_lowercase();
        normalized.contains("lock")
            || normalized.contains("resource temporarily unavailable")
            || normalized.contains("os error 32")
            || normalized.contains("os error 33")
            || normalized.contains("being used by another process")
            || normalized.contains("process cannot access the file")
            || normalized.contains("portion of the file")
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
                Err(error) if Self::error_is_lock_contention(&error) => {
                    let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(&root)?;
                    if attempt + 1 < AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS,
                        ))
                        .await;
                        continue;
                    }
                    return Err(error);
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
                Err(error) if Self::error_is_lock_contention(&error) => {
                    let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(&root)?;
                    if attempt + 1 < READ_ONLY_OPEN_RETRY_COUNT {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            READ_ONLY_OPEN_RETRY_DELAY_MS,
                        ))
                        .await;
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_existing_read_only_once(root).await
    }

    pub async fn open_existing_read_only_with_timeout(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        match tokio::time::timeout(timeout, Self::open_existing_read_only(root)).await {
            Ok(result) => result,
            Err(_) => Err(StateStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timed out while waiting for authoritative datastore lock; another VIDA process still holds the authoritative datastore lock, so stop or wait for that process and retry the command",
            ))),
        }
    }

    pub(crate) async fn close(self) {
        drop(self);
        tokio::time::sleep(std::time::Duration::from_millis(DATASTORE_CLOSE_SETTLE_MS)).await;
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
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(windows)]
    #[test]
    fn windows_lock_violation_errors_are_lock_contention() {
        for code in [32, 33] {
            let error = StateStoreError::Io(std::io::Error::from_raw_os_error(code));
            assert!(
                StateStore::error_is_lock_contention(&error),
                "Windows raw OS error {code} should be retried as lock contention"
            );
        }
    }

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
        store.close().await;

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
        store.close().await;

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
        assert!(
            second_result.is_ok(),
            "second read-only open should succeed after first reader closes: {second_result:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn error_is_lock_contention_ignores_non_lock_errors() {
        let error = StateStoreError::MissingStateDir(PathBuf::from("/tmp/vida-lock-missing-state"));
        assert!(!StateStore::error_is_lock_contention(&error));
    }

    #[test]
    fn db_wrapped_windows_lock_messages_are_lock_contention() {
        for message in [
            "IO error: The process cannot access the file because another process has locked a portion of the file. (os error 33)",
            "IO error: The process cannot access the file because it is being used by another process. (os error 32)",
            "surrealkv: failed to open database: resource temporarily unavailable",
        ] {
            assert!(
                StateStore::message_is_lock_contention(message),
                "DB-wrapped lock message should be retried as lock contention: {message}"
            );
        }
    }

    #[test]
    fn self_pid_authoritative_lock_marker_is_preserved() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-self-owned-failed-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), std::process::id().to_string()).expect("write self lock");

        let reclaimed = StateStore::reclaim_recoverable_authoritative_datastore_lock_marker(&root)
            .expect("self-pid lock marker should not error");

        assert!(!reclaimed);
        assert_eq!(
            fs::read_to_string(root.join("LOCK")).expect("read preserved self-pid lock"),
            std::process::id().to_string()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn recoverable_authoritative_lock_cleanup_reclaims_dead_foreign_pid() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-foreign-failed-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        let foreign_pid = std::process::id().saturating_add(1);
        fs::write(root.join("LOCK"), foreign_pid.to_string()).expect("write foreign lock");

        let reclaimed =
            StateStore::reclaim_recoverable_authoritative_datastore_lock_marker_with_liveness(
                &root,
                |pid| {
                    assert_eq!(pid, foreign_pid);
                    ProcessLiveness::Dead
                },
            )
            .expect("dead foreign lock should not error");

        assert!(reclaimed);
        assert!(!root.join("LOCK").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn recoverable_authoritative_lock_cleanup_preserves_live_foreign_pid() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-live-foreign-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        let foreign_pid = std::process::id().saturating_add(1);
        fs::write(root.join("LOCK"), foreign_pid.to_string()).expect("write foreign lock");

        let reclaimed =
            StateStore::reclaim_recoverable_authoritative_datastore_lock_marker_with_liveness(
                &root,
                |pid| {
                    assert_eq!(pid, foreign_pid);
                    ProcessLiveness::Alive
                },
            )
            .expect("live foreign lock should not error");

        assert!(!reclaimed);
        assert_eq!(
            fs::read_to_string(root.join("LOCK")).expect("read preserved lock"),
            foreign_pid.to_string()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn self_owned_failed_authoritative_lock_cleanup_preserves_invalid_marker() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-invalid-failed-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), "unknown").expect("write invalid lock");

        let reclaimed = StateStore::reclaim_recoverable_authoritative_datastore_lock_marker(&root)
            .expect("invalid lock should not error");

        assert!(!reclaimed);
        assert_eq!(
            fs::read_to_string(root.join("LOCK")).expect("read preserved lock"),
            "unknown"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
