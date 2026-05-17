#![allow(dead_code)]

use super::*;
use fs2::FileExt;
use std::fs::OpenOptions;

const CLAIM_ACQUIRE_GUARD_RETRY_DELAY_MS: u64 = 25;
const CLAIM_ACQUIRE_GUARD_MAX_WAIT_MS: u64 = 30_000;
const CLAIM_ACQUIRE_GUARD_RETRY_COUNT: usize =
    (CLAIM_ACQUIRE_GUARD_MAX_WAIT_MS / CLAIM_ACQUIRE_GUARD_RETRY_DELAY_MS) as usize;

struct ClaimAcquireGuard {
    file: std::fs::File,
}

impl ClaimAcquireGuard {
    async fn acquire(root: &std::path::Path) -> Result<Self, StateStoreError> {
        let guard_path = root.join(".vida-orchestrator-claim-acquire.guard");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&guard_path)?;
        for attempt in 0..CLAIM_ACQUIRE_GUARD_RETRY_COUNT {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(error) if Self::is_lock_contention_error(&error) => {
                    if attempt + 1 < CLAIM_ACQUIRE_GUARD_RETRY_COUNT {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            CLAIM_ACQUIRE_GUARD_RETRY_DELAY_MS,
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
            "timed out while waiting for orchestrator claim acquisition guard",
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

impl Drop for ClaimAcquireGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub(crate) struct OrchestratorClaim {
    pub claim_id: String,
    pub state_root_id: String,
    pub worktree_environment_id: String,
    pub orchestrator_session_id: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub lane_id: Option<String>,
    pub claim_kind: String,
    pub conflict_domain: Option<String>,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
    pub lease_mode: String,
    pub status: String,
    pub created_at: String,
    pub lease_expires_at: String,
    pub last_heartbeat_at: String,
    pub released_at: Option<String>,
    pub release_reason: Option<String>,
    pub resource_revision: u64,
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaseMode {
    Exclusive,
    SharedRead,
    Observe,
}

impl LeaseMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Exclusive => "exclusive",
            Self::SharedRead => "shared_read",
            Self::Observe => "observe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrchestratorClaimStatus {
    Active,
    Renewed,
    Blocked,
    Released,
    Expired,
    Superseded,
    Reclaimed,
}

impl OrchestratorClaimStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Renewed => "renewed",
            Self::Blocked => "blocked",
            Self::Released => "released",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
            Self::Reclaimed => "reclaimed",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AcquireOrchestratorClaimRequest {
    pub claim_id: String,
    pub state_root_id: String,
    pub worktree_environment_id: String,
    pub orchestrator_session_id: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub lane_id: Option<String>,
    pub claim_kind: String,
    pub conflict_domain: Option<String>,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
    pub lease_mode: LeaseMode,
    pub lease_seconds: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub(crate) struct OrchestratorClaimCompatibilityConflict {
    pub conflict_kind: String,
    pub claim_id: String,
    pub orchestrator_session_id: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub conflict_domain: Option<String>,
    pub path: Option<String>,
    pub blocker_code: String,
}

fn claim_time() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn claim_timestamp(time: OffsetDateTime) -> String {
    time.format(&Rfc3339)
        .unwrap_or_else(|_| time.unix_timestamp_nanos().to_string())
}

fn claim_expiry(now: OffsetDateTime, lease_seconds: i64) -> String {
    let bounded_seconds = if lease_seconds == 0 { 1 } else { lease_seconds };
    claim_timestamp(now + time::Duration::seconds(bounded_seconds))
}

fn claim_is_active(status: &str) -> bool {
    matches!(status, "active" | "renewed" | "blocked")
}

fn claim_is_expired(claim: &OrchestratorClaim, now: &str) -> bool {
    claim_is_active(&claim.status)
        && !claim.lease_expires_at.trim().is_empty()
        && claim.lease_expires_at.as_str() <= now
}

fn normalize_claim_path(path: &str) -> Option<String> {
    let mut value = path.trim().replace('\\', "/");
    while let Some(stripped) = value.strip_prefix("./") {
        value = stripped.to_string();
    }
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    value = parts.join("/");
    #[cfg(windows)]
    {
        value = value.to_ascii_lowercase();
    }
    value = value.trim_matches('/').to_string();
    (!value.is_empty()).then_some(value)
}

fn claim_paths_intersect(left: &str, right: &str) -> bool {
    let Some(left) = normalize_claim_path(left) else {
        return false;
    };
    let Some(right) = normalize_claim_path(right) else {
        return false;
    };
    left == right
        || left
            .strip_prefix(right.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn claim_modes_conflict(request_mode: LeaseMode, active_mode: &str) -> bool {
    if request_mode == LeaseMode::Observe || active_mode == LeaseMode::Observe.as_str() {
        return false;
    }
    request_mode == LeaseMode::Exclusive || active_mode == LeaseMode::Exclusive.as_str()
}

fn claim_conflict(
    request: &AcquireOrchestratorClaimRequest,
    claim: &OrchestratorClaim,
) -> Option<OrchestratorClaimCompatibilityConflict> {
    if !claim_modes_conflict(request.lease_mode, &claim.lease_mode) {
        return None;
    }
    if request.task_id.is_some() && request.task_id == claim.task_id {
        return Some(claim_conflict_payload("task", claim, None));
    }
    if request.run_id.is_some() && request.run_id == claim.run_id {
        return Some(claim_conflict_payload("run", claim, None));
    }
    let active_write_paths = claim
        .owned_paths
        .iter()
        .map(|path| (path, "owned_path"))
        .collect::<Vec<_>>();
    let active_paths = claim
        .owned_paths
        .iter()
        .map(|path| (path, "owned_path"))
        .chain(
            claim
                .read_only_paths
                .iter()
                .map(|path| (path, "read_only_path")),
        )
        .collect::<Vec<_>>();
    for requested_path in &request.owned_paths {
        for (active_path, conflict_kind) in &active_paths {
            if claim_paths_intersect(requested_path, active_path) {
                return Some(claim_conflict_payload(
                    conflict_kind,
                    claim,
                    normalize_claim_path(requested_path),
                ));
            }
        }
    }
    for requested_path in &request.read_only_paths {
        for (active_path, conflict_kind) in &active_write_paths {
            if claim_paths_intersect(requested_path, active_path) {
                return Some(claim_conflict_payload(
                    conflict_kind,
                    claim,
                    normalize_claim_path(requested_path),
                ));
            }
        }
    }
    if let (Some(left), Some(right)) = (
        request.conflict_domain.as_deref(),
        claim.conflict_domain.as_deref(),
    ) {
        if !left.trim().is_empty() && left == right {
            return Some(claim_conflict_payload("conflict_domain", claim, None));
        }
    }
    None
}

fn claim_conflict_payload(
    conflict_kind: &str,
    claim: &OrchestratorClaim,
    path: Option<String>,
) -> OrchestratorClaimCompatibilityConflict {
    OrchestratorClaimCompatibilityConflict {
        conflict_kind: conflict_kind.to_string(),
        claim_id: claim.claim_id.clone(),
        orchestrator_session_id: claim.orchestrator_session_id.clone(),
        task_id: claim.task_id.clone(),
        run_id: claim.run_id.clone(),
        conflict_domain: claim.conflict_domain.clone(),
        path,
        blocker_code: format!("orchestrator_claim_conflict_{conflict_kind}"),
    }
}

impl StateStore {
    pub(crate) async fn expire_stale_orchestrator_claims(&self) -> Result<usize, StateStoreError> {
        let now = claim_timestamp(claim_time());
        let active = self.active_orchestrator_claims().await?;
        let stale = active
            .into_iter()
            .filter(|claim| claim_is_expired(claim, &now))
            .collect::<Vec<_>>();
        for mut claim in stale.iter().cloned() {
            claim.status = OrchestratorClaimStatus::Expired.as_str().to_string();
            claim.released_at = Some(now.clone());
            claim.release_reason = Some("lease_expired_requires_reclaim".to_string());
            claim.resource_revision += 1;
            let _: Option<OrchestratorClaim> = self
                .db
                .upsert(("orchestrator_claim", claim.claim_id.as_str()))
                .content(claim)
                .await?;
        }
        Ok(stale.len())
    }

    pub(crate) async fn active_orchestrator_claims(
        &self,
    ) -> Result<Vec<OrchestratorClaim>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM orchestrator_claim \
                 WHERE status IN ['active', 'renewed', 'blocked'] \
                 ORDER BY created_at DESC, claim_id DESC;",
            )
            .await?;
        let rows: Vec<OrchestratorClaim> = query.take(0)?;
        Ok(rows)
    }

    pub(crate) async fn expired_orchestrator_claims(
        &self,
    ) -> Result<Vec<OrchestratorClaim>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM orchestrator_claim \
                 WHERE status = 'expired' \
                 ORDER BY created_at DESC, claim_id DESC;",
            )
            .await?;
        let rows: Vec<OrchestratorClaim> = query.take(0)?;
        Ok(rows)
    }

    async fn active_orchestrator_claims_for_scope(
        &self,
        state_root_id: &str,
        worktree_environment_id: &str,
    ) -> Result<Vec<OrchestratorClaim>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM orchestrator_claim \
                 WHERE status IN ['active', 'renewed', 'blocked'] \
                 AND state_root_id = $state_root_id \
                 AND worktree_environment_id = $worktree_environment_id \
                 ORDER BY created_at DESC, claim_id DESC;",
            )
            .bind(("state_root_id", state_root_id.to_string()))
            .bind((
                "worktree_environment_id",
                worktree_environment_id.to_string(),
            ))
            .await?;
        let rows: Vec<OrchestratorClaim> = query.take(0)?;
        Ok(rows)
    }

    async fn expired_orchestrator_claims_for_scope(
        &self,
        state_root_id: &str,
        worktree_environment_id: &str,
    ) -> Result<Vec<OrchestratorClaim>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM orchestrator_claim \
                 WHERE status = 'expired' \
                 AND state_root_id = $state_root_id \
                 AND worktree_environment_id = $worktree_environment_id \
                 ORDER BY created_at DESC, claim_id DESC;",
            )
            .bind(("state_root_id", state_root_id.to_string()))
            .bind((
                "worktree_environment_id",
                worktree_environment_id.to_string(),
            ))
            .await?;
        let rows: Vec<OrchestratorClaim> = query.take(0)?;
        Ok(rows)
    }

    pub(crate) async fn orchestrator_claim(
        &self,
        claim_id: &str,
    ) -> Result<Option<OrchestratorClaim>, StateStoreError> {
        let row: Option<OrchestratorClaim> =
            self.db.select(("orchestrator_claim", claim_id)).await?;
        Ok(row)
    }

    pub(crate) async fn acquire_orchestrator_claim(
        &self,
        request: AcquireOrchestratorClaimRequest,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        validate_claim_request(&request)?;
        let _guard = ClaimAcquireGuard::acquire(self.root()).await?;
        self.expire_stale_orchestrator_claims().await?;
        if let Some(conflict) = self
            .orchestrator_claim_reclaim_required_conflict(&request)
            .await?
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "orchestrator_claim_expired_requires_reclaim:{}:{}",
                    conflict.claim_id, conflict.conflict_kind
                ),
            });
        }
        let active = self
            .active_orchestrator_claims_for_scope(
                &request.state_root_id,
                &request.worktree_environment_id,
            )
            .await?;
        if let Some(conflict) = active
            .iter()
            .find_map(|claim| claim_conflict(&request, claim))
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "{}:{}:{}",
                    conflict.blocker_code, conflict.conflict_kind, conflict.claim_id
                ),
            });
        }

        let now = claim_time();
        let created_at = claim_timestamp(now);
        let claim = OrchestratorClaim {
            claim_id: request.claim_id.clone(),
            state_root_id: request.state_root_id,
            worktree_environment_id: request.worktree_environment_id,
            orchestrator_session_id: request.orchestrator_session_id,
            task_id: request.task_id,
            run_id: request.run_id,
            lane_id: request.lane_id,
            claim_kind: request.claim_kind,
            conflict_domain: request.conflict_domain,
            owned_paths: request
                .owned_paths
                .iter()
                .filter_map(|path| normalize_claim_path(path))
                .collect(),
            read_only_paths: request
                .read_only_paths
                .iter()
                .filter_map(|path| normalize_claim_path(path))
                .collect(),
            lease_mode: request.lease_mode.as_str().to_string(),
            status: OrchestratorClaimStatus::Active.as_str().to_string(),
            created_at: created_at.clone(),
            lease_expires_at: claim_expiry(now, request.lease_seconds),
            last_heartbeat_at: created_at,
            released_at: None,
            release_reason: None,
            resource_revision: 1,
            blocker_codes: Vec::new(),
        };
        let _: Option<OrchestratorClaim> = self
            .db
            .upsert(("orchestrator_claim", claim.claim_id.as_str()))
            .content(claim.clone())
            .await?;
        Ok(claim)
    }

    async fn orchestrator_claim_reclaim_required_conflict(
        &self,
        request: &AcquireOrchestratorClaimRequest,
    ) -> Result<Option<OrchestratorClaimCompatibilityConflict>, StateStoreError> {
        let expired = self
            .expired_orchestrator_claims_for_scope(
                &request.state_root_id,
                &request.worktree_environment_id,
            )
            .await?;
        Ok(expired
            .iter()
            .find_map(|claim| claim_conflict(request, claim)))
    }

    pub(crate) async fn heartbeat_orchestrator_claim(
        &self,
        claim_id: &str,
        expected_resource_revision: u64,
        lease_seconds: i64,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        let _guard = ClaimAcquireGuard::acquire(self.root()).await?;
        let mut claim = self.orchestrator_claim(claim_id).await?.ok_or_else(|| {
            StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator claim not found: {claim_id}"),
            }
        })?;
        let now = claim_time();
        let now_text = claim_timestamp(now);
        if !claim_is_active(&claim.status) || claim_is_expired(&claim, &now_text) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator_claim_not_active_for_heartbeat:{claim_id}"),
            });
        }
        if claim.resource_revision != expected_resource_revision {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "orchestrator_claim_resource_revision_mismatch:{}:{}:{}",
                    claim_id, expected_resource_revision, claim.resource_revision
                ),
            });
        }
        claim.status = OrchestratorClaimStatus::Renewed.as_str().to_string();
        claim.last_heartbeat_at = now_text;
        claim.lease_expires_at = claim_expiry(now, lease_seconds);
        claim.resource_revision += 1;
        let _: Option<OrchestratorClaim> = self
            .db
            .upsert(("orchestrator_claim", claim_id))
            .content(claim.clone())
            .await?;
        Ok(claim)
    }

    pub(crate) async fn release_orchestrator_claim(
        &self,
        claim_id: &str,
        expected_resource_revision: u64,
        reason: &str,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        let _guard = ClaimAcquireGuard::acquire(self.root()).await?;
        let mut claim = self.orchestrator_claim(claim_id).await?.ok_or_else(|| {
            StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator claim not found: {claim_id}"),
            }
        })?;
        if !claim_is_active(&claim.status) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator_claim_not_active_for_release:{claim_id}"),
            });
        }
        if claim.resource_revision != expected_resource_revision {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "orchestrator_claim_resource_revision_mismatch:{}:{}:{}",
                    claim_id, expected_resource_revision, claim.resource_revision
                ),
            });
        }
        claim.status = OrchestratorClaimStatus::Released.as_str().to_string();
        claim.released_at = Some(claim_timestamp(claim_time()));
        claim.release_reason = Some(reason.to_string());
        claim.resource_revision += 1;
        let _: Option<OrchestratorClaim> = self
            .db
            .upsert(("orchestrator_claim", claim_id))
            .content(claim.clone())
            .await?;
        Ok(claim)
    }

    #[cfg(test)]
    pub(crate) async fn mark_orchestrator_claim_blocked_for_test(
        &self,
        claim_id: &str,
        blocker_codes: Vec<String>,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        let mut claim = self.orchestrator_claim(claim_id).await?.ok_or_else(|| {
            StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator claim not found: {claim_id}"),
            }
        })?;
        claim.status = OrchestratorClaimStatus::Blocked.as_str().to_string();
        claim.blocker_codes = blocker_codes;
        claim.resource_revision += 1;
        let _: Option<OrchestratorClaim> = self
            .db
            .upsert(("orchestrator_claim", claim_id))
            .content(claim.clone())
            .await?;
        Ok(claim)
    }

    pub(crate) async fn supersede_expired_orchestrator_claim(
        &self,
        claim_id: &str,
        expected_resource_revision: u64,
        superseded_by_claim_id: &str,
        reason: &str,
    ) -> Result<OrchestratorClaim, StateStoreError> {
        let _guard = ClaimAcquireGuard::acquire(self.root()).await?;
        let mut claim = self.orchestrator_claim(claim_id).await?.ok_or_else(|| {
            StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator claim not found: {claim_id}"),
            }
        })?;
        if claim.status != OrchestratorClaimStatus::Expired.as_str() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("orchestrator_claim_not_expired_for_supersede:{claim_id}"),
            });
        }
        if claim.resource_revision != expected_resource_revision {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "orchestrator_claim_resource_revision_mismatch:{}:{}:{}",
                    claim_id, expected_resource_revision, claim.resource_revision
                ),
            });
        }
        claim.status = OrchestratorClaimStatus::Superseded.as_str().to_string();
        claim.released_at = Some(claim_timestamp(claim_time()));
        claim.release_reason = Some(format!(
            "superseded_by:{}:{}",
            superseded_by_claim_id,
            reason.trim()
        ));
        claim.resource_revision += 1;
        let _: Option<OrchestratorClaim> = self
            .db
            .upsert(("orchestrator_claim", claim_id))
            .content(claim.clone())
            .await?;
        Ok(claim)
    }
}

fn validate_claim_request(
    request: &AcquireOrchestratorClaimRequest,
) -> Result<(), StateStoreError> {
    let missing = [
        ("claim_id", request.claim_id.as_str()),
        ("state_root_id", request.state_root_id.as_str()),
        (
            "worktree_environment_id",
            request.worktree_environment_id.as_str(),
        ),
        (
            "orchestrator_session_id",
            request.orchestrator_session_id.as_str(),
        ),
        ("claim_kind", request.claim_kind.as_str()),
    ]
    .into_iter()
    .find_map(|(name, value)| value.trim().is_empty().then_some(name));
    if let Some(name) = missing {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!("orchestrator_claim_missing_required_field:{name}"),
        });
    }
    if request.lease_mode != LeaseMode::Observe
        && request.task_id.is_none()
        && request.run_id.is_none()
    {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: "orchestrator_claim_missing_bounded_unit".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vida-orchestrator-claim-{name}-{nanos}"))
    }

    fn claim_request(
        claim_id: &str,
        session_id: &str,
        task_id: &str,
        run_id: &str,
        conflict_domain: &str,
        owned_paths: &[&str],
    ) -> AcquireOrchestratorClaimRequest {
        AcquireOrchestratorClaimRequest {
            claim_id: claim_id.to_string(),
            state_root_id: "state-root".to_string(),
            worktree_environment_id: "worktree".to_string(),
            orchestrator_session_id: session_id.to_string(),
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            lane_id: Some("lane".to_string()),
            claim_kind: "write".to_string(),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: owned_paths.iter().map(|path| path.to_string()).collect(),
            read_only_paths: Vec::new(),
            lease_mode: LeaseMode::Exclusive,
            lease_seconds: 60,
        }
    }

    #[tokio::test]
    async fn orchestrator_claim_acquire_allows_disjoint_units() {
        let root = temp_state_dir("disjoint");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let first = store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src/a.rs"],
            ))
            .await
            .expect("first claim");
        let second = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-2",
                "run-2",
                "domain-b",
                &["crates/vida/src/b.rs"],
            ))
            .await
            .expect("disjoint claim");

        assert_eq!(first.status, "active");
        assert_eq!(second.status, "active");
        assert_eq!(store.active_orchestrator_claims().await.unwrap().len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn orchestrator_claim_blocks_same_task_and_conflict_domain() {
        let root = temp_state_dir("same-task");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src/a.rs"],
            ))
            .await
            .expect("first claim");

        let same_task_error = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-1",
                "run-2",
                "domain-b",
                &["crates/vida/src/b.rs"],
            ))
            .await
            .expect_err("same task should block");
        assert!(same_task_error
            .to_string()
            .contains("orchestrator_claim_conflict_task:task:claim-1"));

        let same_domain_error = store
            .acquire_orchestrator_claim(claim_request(
                "claim-3",
                "session-3",
                "task-3",
                "run-3",
                "domain-a",
                &["crates/vida/src/c.rs"],
            ))
            .await
            .expect_err("same conflict domain should block");
        assert!(same_domain_error
            .to_string()
            .contains("orchestrator_claim_conflict_conflict_domain:conflict_domain:claim-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn orchestrator_claim_blocks_owned_path_intersection() {
        let root = temp_state_dir("path");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src"],
            ))
            .await
            .expect("first claim");

        let error = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-2",
                "run-2",
                "domain-b",
                &[".\\crates\\vida\\src\\taskflow_proxy.rs"],
            ))
            .await
            .expect_err("path intersection should block");

        assert!(error
            .to_string()
            .contains("orchestrator_claim_conflict_owned_path:owned_path:claim-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn orchestrator_claim_blocks_exclusive_shared_read_path_intersection() {
        let root = temp_state_dir("read-path");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                lease_mode: LeaseMode::SharedRead,
                owned_paths: Vec::new(),
                read_only_paths: vec!["crates/vida/src".to_string()],
                ..claim_request("claim-1", "session-1", "task-1", "run-1", "domain-a", &[])
            })
            .await
            .expect("shared read claim");

        let error = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-2",
                "run-2",
                "domain-b",
                &["crates/vida/src/taskflow_proxy.rs"],
            ))
            .await
            .expect_err("exclusive write should block shared-read path");

        assert!(error
            .to_string()
            .contains("orchestrator_claim_conflict_read_only_path:read_only_path:claim-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn orchestrator_claim_scope_prevents_foreign_environment_conflict() {
        let root = temp_state_dir("scope");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src/a.rs"],
            ))
            .await
            .expect("first claim");

        let claim = store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                state_root_id: "other-state-root".to_string(),
                worktree_environment_id: "other-worktree".to_string(),
                ..claim_request(
                    "claim-2",
                    "session-2",
                    "task-1",
                    "run-2",
                    "domain-a",
                    &["crates/vida/src/a.rs"],
                )
            })
            .await
            .expect("foreign environment should not block");

        assert_eq!(claim.status, "active");
        assert_eq!(store.active_orchestrator_claims().await.unwrap().len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn blocked_foreign_claim_is_nonblocking_when_disjoint() {
        let root = temp_state_dir("foreign-blocked-disjoint");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let blocked = store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src/a.rs"],
            ))
            .await
            .expect("first claim");
        store
            .mark_orchestrator_claim_blocked_for_test(
                &blocked.claim_id,
                vec!["foreign_lane_waiting_for_operator".to_string()],
            )
            .await
            .expect("persist blocked claim");

        let disjoint = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-2",
                "run-2",
                "domain-b",
                &["crates/vida/src/b.rs"],
            ))
            .await
            .expect("disjoint blocked foreign claim should not block");

        assert_eq!(disjoint.status, "active");
        assert_eq!(store.active_orchestrator_claims().await.unwrap().len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn orchestrator_claim_heartbeat_release_and_expiry_are_revisioned() {
        let root = temp_state_dir("revision");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let claim = store
            .acquire_orchestrator_claim(claim_request(
                "claim-1",
                "session-1",
                "task-1",
                "run-1",
                "domain-a",
                &["crates/vida/src/a.rs"],
            ))
            .await
            .expect("first claim");
        let renewed = store
            .heartbeat_orchestrator_claim("claim-1", claim.resource_revision, 60)
            .await
            .expect("heartbeat should renew");
        assert_eq!(renewed.status, "renewed");
        assert_eq!(renewed.resource_revision, 2);

        let mismatch = store
            .release_orchestrator_claim("claim-1", 1, "done")
            .await
            .expect_err("stale revision should fail");
        assert!(mismatch
            .to_string()
            .contains("orchestrator_claim_resource_revision_mismatch:claim-1:1:2"));

        let released = store
            .release_orchestrator_claim("claim-1", 2, "done")
            .await
            .expect("release should persist");
        assert_eq!(released.status, "released");
        assert_eq!(released.resource_revision, 3);
        assert!(store.active_orchestrator_claims().await.unwrap().is_empty());
        let released_heartbeat = store
            .heartbeat_orchestrator_claim("claim-1", 3, 60)
            .await
            .expect_err("released claim heartbeat should fail");
        assert!(released_heartbeat
            .to_string()
            .contains("orchestrator_claim_not_active_for_heartbeat:claim-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn expired_orchestrator_claim_requires_reclaim_before_reuse() {
        let root = temp_state_dir("expired");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                lease_seconds: -60,
                ..claim_request(
                    "claim-1",
                    "session-1",
                    "task-1",
                    "run-1",
                    "domain-a",
                    &["crates/vida/src/a.rs"],
                )
            })
            .await
            .expect("first claim");
        assert_eq!(store.expire_stale_orchestrator_claims().await.unwrap(), 1);

        let error = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-1",
                "run-2",
                "domain-b",
                &["crates/vida/src/b.rs"],
            ))
            .await
            .expect_err("expired matching claim should require reclaim");
        assert!(error
            .to_string()
            .contains("orchestrator_claim_expired_requires_reclaim:claim-1:task"));

        let expired = store
            .orchestrator_claim("claim-1")
            .await
            .unwrap()
            .expect("expired claim");
        let superseded = store
            .supersede_expired_orchestrator_claim(
                "claim-1",
                expired.resource_revision,
                "claim-2",
                "recorded_reclaim_receipt",
            )
            .await
            .expect("supersede expired claim");
        assert_eq!(superseded.status, "superseded");

        let replacement = store
            .acquire_orchestrator_claim(claim_request(
                "claim-2",
                "session-2",
                "task-1",
                "run-2",
                "domain-b",
                &["crates/vida/src/b.rs"],
            ))
            .await
            .expect("replacement claim after supersede");
        assert_eq!(replacement.status, "active");
        let _ = fs::remove_dir_all(root);
    }
}
