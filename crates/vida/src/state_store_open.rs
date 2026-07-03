use super::*;
use fs2::FileExt;
use std::fs::OpenOptions;
use std::sync::Arc;
use surrealdb_core::cnf::ConfigMap;
use surrealdb_core::kvs::Datastore;
use surrealdb_core::options::EngineOptions;
use tokio_util::sync::CancellationToken;
use vida_runtime_local::jobs::RetryBackoffPolicy;

const AUTHORITATIVE_DATASTORE_LOCK_MAX_WAIT_MS: u64 = 30_000;
const AUTHORITATIVE_DATASTORE_LOCK_RETRY_POLICY: RetryBackoffPolicy =
    RetryBackoffPolicy::linear_millis(AUTHORITATIVE_DATASTORE_LOCK_MAX_WAIT_MS, 25);
const AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS: u64 =
    AUTHORITATIVE_DATASTORE_LOCK_RETRY_POLICY.base_delay_millis();
const AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT: usize =
    AUTHORITATIVE_DATASTORE_LOCK_RETRY_POLICY.max_attempts_usize();
const AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT: usize = AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT;
const AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS: u64 = AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS;
const READ_ONLY_OPEN_RETRY_POLICY: RetryBackoffPolicy =
    RetryBackoffPolicy::linear_attempts(800, 25);
const READ_ONLY_OPEN_RETRY_COUNT: usize = READ_ONLY_OPEN_RETRY_POLICY.max_attempts_usize();
const READ_ONLY_OPEN_RETRY_DELAY_MS: u64 = READ_ONLY_OPEN_RETRY_POLICY.base_delay_millis();
const READ_ONLY_OPEN_MIN_TIMEOUT_MS: u64 = 10_000;
const DATASTORE_CLOSE_SETTLE_MS: u64 = 250;
const STALE_LOCK_MARKER_REMOVE_RETRY_POLICY: RetryBackoffPolicy =
    RetryBackoffPolicy::linear_attempts(20, 25);
const STALE_LOCK_MARKER_REMOVE_RETRY_COUNT: usize =
    STALE_LOCK_MARKER_REMOVE_RETRY_POLICY.max_attempts_usize();
const STALE_LOCK_MARKER_REMOVE_RETRY_DELAY_MS: u64 =
    STALE_LOCK_MARKER_REMOVE_RETRY_POLICY.base_delay_millis();
const FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_POLICY: RetryBackoffPolicy =
    RetryBackoffPolicy::linear_attempts(8, DATASTORE_CLOSE_SETTLE_MS);
const FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_COUNT: usize =
    FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_POLICY.max_attempts_usize();
const FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_DELAY_MS: u64 =
    FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_POLICY.base_delay_millis();
const VIDA_SURREALKV_MAX_MEMTABLE_SIZE_BYTES: usize = 16 * 1024 * 1024;
const VIDA_SURREALKV_BLOCK_CACHE_CAPACITY_BYTES: u64 = 16 * 1024 * 1024;
const VIDA_SURREALKV_VLOG_MAX_FILE_SIZE_BYTES: u64 = 64 * 1024 * 1024;

pub(super) struct ExclusiveFileAcquireGuard {
    file: std::fs::File,
}

pub(super) struct ExclusiveFileAcquireGuardSpec {
    guard_file_name: &'static str,
    retry_count: usize,
    retry_delay_ms: u64,
    timeout_message: &'static str,
    include_windows_raw_lock_codes: bool,
}

impl ExclusiveFileAcquireGuardSpec {
    pub(super) const fn new(
        guard_file_name: &'static str,
        retry_count: usize,
        retry_delay_ms: u64,
        timeout_message: &'static str,
        include_windows_raw_lock_codes: bool,
    ) -> Self {
        Self {
            guard_file_name,
            retry_count,
            retry_delay_ms,
            timeout_message,
            include_windows_raw_lock_codes,
        }
    }
}

impl ExclusiveFileAcquireGuard {
    pub(super) async fn acquire(
        root: &Path,
        spec: ExclusiveFileAcquireGuardSpec,
    ) -> Result<Self, StateStoreError> {
        let guard_path = root.join(spec.guard_file_name);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&guard_path)?;
        for attempt in 0..spec.retry_count {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(error)
                    if exclusive_file_lock_contention_error(
                        &error,
                        spec.include_windows_raw_lock_codes,
                    ) =>
                {
                    if attempt + 1 < spec.retry_count {
                        tokio::time::sleep(std::time::Duration::from_millis(spec.retry_delay_ms))
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
            spec.timeout_message,
        )))
    }
}

impl Drop for ExclusiveFileAcquireGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn exclusive_file_lock_contention_error(
    error: &std::io::Error,
    include_windows_raw_lock_codes: bool,
) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    ) || error.raw_os_error().is_some_and(|code| {
        code == libc::EWOULDBLOCK
            || code == libc::EAGAIN
            || (include_windows_raw_lock_codes && cfg!(windows) && matches!(code, 5 | 32 | 33))
    })
}

pub(super) struct AuthoritativeOpenGuard {
    _guard: ExclusiveFileAcquireGuard,
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
    let tasklist_path = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
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
    pub(super) async fn acquire(root: &Path) -> Result<Self, StateStoreError> {
        Ok(Self {
            _guard: ExclusiveFileAcquireGuard::acquire(
                root,
                ExclusiveFileAcquireGuardSpec::new(
                    ".vida-authoritative-open.guard",
                    AUTHORITATIVE_OPEN_GUARD_RETRY_COUNT,
                    AUTHORITATIVE_OPEN_GUARD_RETRY_DELAY_MS,
                    "timed out while waiting for authoritative datastore access serialization guard",
                    true,
                ),
            )
            .await?,
        })
    }

    fn is_lock_contention_error(error: &std::io::Error) -> bool {
        exclusive_file_lock_contention_error(error, true)
    }
}

pub(super) fn state_schema_document() -> &'static str {
    static STATE_SCHEMA_DOCUMENT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    STATE_SCHEMA_DOCUMENT
        .get_or_init(|| {
            let storage_schema =
                SurrealStoreTarget::new(DEFAULT_STATE_DIR).bootstrap_schema_document();
            format!("{storage_schema}\n\n{INSTRUCTION_STATE_SCHEMA}")
        })
        .as_str()
}

impl StateStore {
    fn bounded_surrealkv_config() -> ConfigMap {
        ConfigMap::empty()
            .with_key_value(
                "surrealkv_max_memtable_size",
                VIDA_SURREALKV_MAX_MEMTABLE_SIZE_BYTES.to_string(),
            )
            .with_key_value(
                "surrealkv_block_cache_capacity",
                VIDA_SURREALKV_BLOCK_CACHE_CAPACITY_BYTES.to_string(),
            )
            .with_key_value(
                "surrealkv_vlog_max_file_size",
                VIDA_SURREALKV_VLOG_MAX_FILE_SIZE_BYTES.to_string(),
            )
    }

    async fn open_bounded_surrealkv(
        root: &Path,
        bootstrap: bool,
    ) -> Result<Surreal<Db>, StateStoreError> {
        let endpoint = format!("surrealkv://{}", root.display());
        let datastore = Datastore::builder()
            .with_config(Self::bounded_surrealkv_config())
            .build_with_path(&endpoint)
            .await
            .map_err(|error| Self::bounded_surrealkv_open_error("open", error.to_string()))?;
        datastore.check_version().await.map_err(|error| {
            Self::bounded_surrealkv_open_error("check version for", error.to_string())
        })?;
        if bootstrap {
            datastore.bootstrap().await.map_err(|error| {
                Self::bounded_surrealkv_open_error("bootstrap", error.to_string())
            })?;
        }
        Surreal::<Db>::unstable_from_datastore(
            CancellationToken::new(),
            Arc::new(datastore),
            None,
            EngineOptions::default(),
        )
        .await
        .map_err(StateStoreError::from)
    }

    fn bounded_surrealkv_open_error(action: &str, error: String) -> StateStoreError {
        let reason = format!("failed to {action} bounded SurrealKV datastore: {error}");
        if Self::message_is_lock_contention(&reason) {
            StateStoreError::Io(std::io::Error::new(std::io::ErrorKind::WouldBlock, reason))
        } else {
            StateStoreError::InvalidStorageMetadata { reason }
        }
    }

    fn effective_read_only_open_timeout(timeout: std::time::Duration) -> std::time::Duration {
        timeout.max(std::time::Duration::from_millis(
            READ_ONLY_OPEN_MIN_TIMEOUT_MS,
        ))
    }

    fn strict_read_only_open_timeout(timeout: std::time::Duration) -> std::time::Duration {
        timeout.max(std::time::Duration::from_millis(1))
    }

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
        for attempt in 0..STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
            match fs::remove_file(&lock_path) {
                Ok(()) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(error) if AuthoritativeOpenGuard::is_lock_contention_error(&error) => {
                    if attempt + 1 < STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
                        std::thread::sleep(std::time::Duration::from_millis(
                            STALE_LOCK_MARKER_REMOVE_RETRY_DELAY_MS,
                        ));
                        continue;
                    }
                    return Ok(false);
                }
                Err(error) => return Err(StateStoreError::Io(error)),
            }
        }
        Ok(false)
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
        let lock_path = root.join("LOCK");
        let mut lock_text = None;
        for attempt in 0..STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
            match fs::read_to_string(&lock_path) {
                Ok(text) => {
                    lock_text = Some(text);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(error) if AuthoritativeOpenGuard::is_lock_contention_error(&error) => {
                    if attempt + 1 < STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
                        std::thread::sleep(std::time::Duration::from_millis(
                            STALE_LOCK_MARKER_REMOVE_RETRY_DELAY_MS,
                        ));
                        continue;
                    }
                    return Ok(false);
                }
                Err(error) => return Err(StateStoreError::Io(error)),
            }
        }
        let Some(lock_text) = lock_text else {
            return Ok(false);
        };
        let Ok(pid) = lock_text.trim().parse::<u32>() else {
            return Ok(false);
        };
        if pid != std::process::id() {
            return Ok(false);
        }
        for attempt in 0..STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
            match fs::remove_file(&lock_path) {
                Ok(()) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(error) if AuthoritativeOpenGuard::is_lock_contention_error(&error) => {
                    if attempt + 1 < STALE_LOCK_MARKER_REMOVE_RETRY_COUNT {
                        std::thread::sleep(std::time::Duration::from_millis(
                            STALE_LOCK_MARKER_REMOVE_RETRY_DELAY_MS,
                        ));
                        continue;
                    }
                    return Ok(false);
                }
                Err(error) => return Err(StateStoreError::Io(error)),
            }
        }
        Ok(false)
    }

    fn reclaim_self_owned_failed_authoritative_datastore_lock_marker_after_timeout(
        root: &Path,
    ) -> Result<bool, StateStoreError> {
        for attempt in 0..FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_COUNT {
            if Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker(root)? {
                return Ok(true);
            }
            if attempt + 1 < FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_COUNT {
                std::thread::sleep(std::time::Duration::from_millis(
                    FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_DELAY_MS,
                ));
            }
        }
        Ok(false)
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
            || normalized.contains("os error 5")
            || normalized.contains("access is denied")
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
                    let _ =
                        Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker(&root)?;
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
        Box::pin(Self::open_impl(root)).await
    }

    async fn open_impl(root: PathBuf) -> Result<Self, StateStoreError> {
        fs::create_dir_all(&root)?;
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;
        Self::open_with_authoritative_lock_retry(root, Self::open_once).await
    }

    pub async fn open_existing(root: PathBuf) -> Result<Self, StateStoreError> {
        Box::pin(Self::open_existing_impl(root)).await
    }

    async fn open_existing_impl(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }
        if !Self::state_dir_has_existing_datastore_payload(&root)? {
            return Self::open(root).await;
        }
        let _guard = AuthoritativeOpenGuard::acquire(&root).await?;
        Self::open_with_authoritative_lock_retry(root, Self::open_existing_once).await
    }

    pub async fn open_existing_with_timeout(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        Box::pin(Self::open_existing_with_timeout_impl(root, timeout)).await
    }

    async fn open_existing_with_timeout_impl(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        match tokio::time::timeout(timeout, Self::open_existing(root.clone())).await {
            Ok(result) => result,
            Err(_) => {
                let _ = Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker_after_timeout(&root)?;
                Err(StateStoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "timed out while waiting for authoritative datastore lock; another VIDA process still holds the authoritative datastore lock, so stop or wait for that process and retry the command",
                )))
            }
        }
    }

    pub async fn open_existing_read_only(root: PathBuf) -> Result<Self, StateStoreError> {
        Box::pin(Self::open_existing_read_only_impl(root)).await
    }

    async fn open_existing_read_only_impl(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }

        for attempt in 0..READ_ONLY_OPEN_RETRY_COUNT {
            let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(&root)?;
            match Self::open_existing_read_only_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(error) if Self::error_is_lock_contention(&error) => {
                    let _ =
                        Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker(&root)?;
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
        Box::pin(Self::open_existing_read_only_with_timeout_impl(
            root, timeout,
        ))
        .await
    }

    async fn open_existing_read_only_with_timeout_impl(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        let timeout = Self::effective_read_only_open_timeout(timeout);
        Self::open_existing_read_only_with_resolved_timeout(root, timeout).await
    }

    pub async fn open_existing_read_only_with_strict_timeout(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        Box::pin(Self::open_existing_read_only_with_strict_timeout_impl(
            root, timeout,
        ))
        .await
    }

    async fn open_existing_read_only_with_strict_timeout_impl(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        let timeout = Self::strict_read_only_open_timeout(timeout);
        Self::open_existing_read_only_with_resolved_timeout(root, timeout).await
    }

    pub async fn open_existing_structural_read_only_with_timeout(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        Box::pin(Self::open_existing_structural_read_only_with_timeout_impl(
            root, timeout,
        ))
        .await
    }

    async fn open_existing_structural_read_only_with_timeout_impl(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }
        let timeout = Self::strict_read_only_open_timeout(timeout);
        match tokio::time::timeout(timeout, async {
            let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(&root)?;
            Self::open_existing_structural_read_only_once(root.clone()).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let _ = Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker_after_timeout(&root)?;
                Err(StateStoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "timed out while opening structural read-only state; retry later or run a full diagnostic surface for recovery",
                )))
            }
        }
    }

    async fn open_existing_read_only_with_resolved_timeout(
        root: PathBuf,
        timeout: std::time::Duration,
    ) -> Result<Self, StateStoreError> {
        match tokio::time::timeout(timeout, Self::open_existing_read_only(root.clone())).await {
            Ok(result) => result,
            Err(_) => {
                let _ = Self::reclaim_self_owned_failed_authoritative_datastore_lock_marker_after_timeout(&root)?;
                Err(StateStoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "timed out while waiting for authoritative datastore lock; another VIDA process still holds the authoritative datastore lock, so stop or wait for that process and retry the command",
                )))
            }
        }
    }

    pub(crate) async fn close(self) {
        drop(self);
        tokio::time::sleep(std::time::Duration::from_millis(DATASTORE_CLOSE_SETTLE_MS)).await;
    }

    fn state_dir_has_existing_datastore_payload(root: &Path) -> Result<bool, StateStoreError> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if matches!(
                file_name.as_ref(),
                ".vida-authoritative-open.guard"
                    | "LOCK"
                    | ".operator-projection-cache-state-marker"
            ) {
                continue;
            }
            return Ok(true);
        }
        Ok(false)
    }

    async fn open_existing_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Box::pin(Self::open_bounded_surrealkv(&root, true)).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;
        Ok(Self { db, root })
    }

    async fn open_existing_read_only_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Box::pin(Self::open_bounded_surrealkv(&root, true)).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;
        Ok(Self { db, root })
    }

    async fn open_existing_structural_read_only_once(
        root: PathBuf,
    ) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Box::pin(Self::open_bounded_surrealkv(&root, false)).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        Ok(Self { db, root })
    }

    async fn open_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Box::pin(Self::open_bounded_surrealkv(&root, true)).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;

        let store = Self { db, root };
        store.sanitize_legacy_task_execution_semantics().await?;
        store.sanitize_legacy_task_planner_metadata().await?;
        store.expire_stale_scheduler_dispatch_reservations().await?;
        store.expire_stale_orchestrator_claims().await?;
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
        for code in [5, 32, 33] {
            let error = StateStoreError::Io(std::io::Error::from_raw_os_error(code));
            assert!(
                StateStore::error_is_lock_contention(&error),
                "Windows raw OS error {code} should be retried as lock contention"
            );
            assert!(
                !exclusive_file_lock_contention_error(
                    &std::io::Error::from_raw_os_error(code),
                    false
                ),
                "non-authoritative guards should preserve their narrower raw OS code contract"
            );
        }
    }

    #[tokio::test]
    async fn exclusive_file_acquire_guard_preserves_path_and_timeout_message() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-exclusive-file-acquire-guard-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create acquire guard test root");

        let message = "timed out while waiting for test acquisition guard";
        let error = match ExclusiveFileAcquireGuard::acquire(
            &root,
            ExclusiveFileAcquireGuardSpec::new(".vida-test-acquire.guard", 0, 25, message, false),
        )
        .await
        {
            Ok(_) => panic!("zero-attempt acquire should return timeout error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains(message));
        assert!(root.join(".vida-test-acquire.guard").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn state_open_retry_windows_use_shared_backoff_policy() {
        assert_eq!(AUTHORITATIVE_DATASTORE_LOCK_RETRY_COUNT, 1_200);
        assert_eq!(AUTHORITATIVE_DATASTORE_LOCK_RETRY_DELAY_MS, 25);
        assert_eq!(READ_ONLY_OPEN_RETRY_COUNT, 800);
        assert_eq!(READ_ONLY_OPEN_RETRY_DELAY_MS, 25);
        assert_eq!(STALE_LOCK_MARKER_REMOVE_RETRY_COUNT, 20);
        assert_eq!(STALE_LOCK_MARKER_REMOVE_RETRY_DELAY_MS, 25);
        assert_eq!(FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_COUNT, 8);
        assert_eq!(
            FAILED_OPEN_SELF_LOCK_CLEANUP_RETRY_DELAY_MS,
            DATASTORE_CLOSE_SETTLE_MS
        );
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
    async fn runtime_storage_auto_repair_read_only_open_restores_missing_schema_tables() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-storage-auto-repair-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create legacy state root without schema");

        let store = StateStore::open_existing_read_only(root.clone())
            .await
            .expect("read-only open should apply canonical schema repair");
        let metadata = store
            .storage_metadata_summary()
            .await
            .expect("storage metadata should be repaired during read-only open");
        assert_eq!(metadata.engine, "surrealdb");
        assert_eq!(metadata.backend, "kv-surrealkv");
        assert_eq!(metadata.namespace, STATE_NAMESPACE);
        assert_eq!(metadata.database, STATE_DATABASE);

        let activation_snapshot = store.read_launcher_activation_snapshot().await;
        assert!(
            matches!(
                activation_snapshot,
                Err(StateStoreError::MissingLauncherActivationSnapshot)
            ),
            "launcher activation table should exist after repair and report only the missing row"
        );
        store.close().await;
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
            "IO error: Access is denied. (os error 5)",
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
    fn read_only_timeout_has_recovery_floor() {
        assert_eq!(
            StateStore::effective_read_only_open_timeout(std::time::Duration::from_secs(2)),
            std::time::Duration::from_millis(READ_ONLY_OPEN_MIN_TIMEOUT_MS)
        );
        assert_eq!(
            StateStore::effective_read_only_open_timeout(std::time::Duration::from_secs(12)),
            std::time::Duration::from_secs(12)
        );
    }

    #[test]
    fn strict_read_only_timeout_preserves_operator_latency_budget() {
        assert_eq!(
            StateStore::strict_read_only_open_timeout(std::time::Duration::from_secs(2)),
            std::time::Duration::from_secs(2)
        );
        assert_eq!(
            StateStore::strict_read_only_open_timeout(std::time::Duration::ZERO),
            std::time::Duration::from_millis(1)
        );
    }

    #[test]
    fn bounded_surrealkv_config_sets_memory_caps() {
        let rendered = format!("{:?}", StateStore::bounded_surrealkv_config());

        assert!(rendered.contains("surrealkv_max_memtable_size"));
        assert!(rendered.contains("16777216"));
        assert!(rendered.contains("surrealkv_block_cache_capacity"));
        assert!(rendered.contains("surrealkv_vlog_max_file_size"));
        assert!(rendered.contains("67108864"));
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
    fn self_owned_failed_authoritative_lock_cleanup_reclaims_current_pid() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-current-pid-failed-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), std::process::id().to_string())
            .expect("write current-pid lock");

        let reclaimed =
            StateStore::reclaim_self_owned_failed_authoritative_datastore_lock_marker(&root)
                .expect("current-pid failed lock marker should not error");

        assert!(reclaimed);
        assert!(!root.join("LOCK").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn self_owned_failed_authoritative_lock_cleanup_after_timeout_reclaims_current_pid() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-current-pid-timeout-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), std::process::id().to_string())
            .expect("write current-pid lock");

        let reclaimed = StateStore::reclaim_self_owned_failed_authoritative_datastore_lock_marker_after_timeout(&root)
            .expect("current-pid timeout lock cleanup should not error");

        assert!(reclaimed);
        assert!(!root.join("LOCK").exists());
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

    #[cfg(any(windows, target_os = "linux"))]
    #[tokio::test]
    async fn open_reclaims_dead_authoritative_lock_marker_before_database_open() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-open-reclaims-dead-authoritative-lock-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), u32::MAX.to_string()).expect("write dead foreign lock");

        let store = StateStore::open(root.clone())
            .await
            .expect("open should reclaim dead lock marker before database open");
        store.close().await;

        let dead_pid = u32::MAX.to_string();
        assert_ne!(
            fs::read_to_string(root.join("LOCK")).ok().as_deref(),
            Some(dead_pid.as_str()),
            "dead authoritative lock marker should be reclaimed before SurrealKV open"
        );
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

        let reclaimed =
            StateStore::reclaim_self_owned_failed_authoritative_datastore_lock_marker(&root)
                .expect("invalid lock should not error");

        assert!(!reclaimed);
        assert_eq!(
            fs::read_to_string(root.join("LOCK")).expect("read preserved lock"),
            "unknown"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
