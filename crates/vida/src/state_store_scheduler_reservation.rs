#![allow(dead_code)]

use super::state_store_open::{ExclusiveFileAcquireGuard, ExclusiveFileAcquireGuardSpec};
use super::*;
use taskflow_authority::scheduler_claim;
use taskflow_core::scheduling::scheduler_dispatch::{self, ParallelSafetyInput};

const RESERVATION_ACQUIRE_GUARD_RETRY_DELAY_MS: u64 = 25;
const RESERVATION_ACQUIRE_GUARD_MAX_WAIT_MS: u64 = 30_000;
const RESERVATION_ACQUIRE_GUARD_RETRY_COUNT: usize =
    (RESERVATION_ACQUIRE_GUARD_MAX_WAIT_MS / RESERVATION_ACQUIRE_GUARD_RETRY_DELAY_MS) as usize;
const DEFAULT_SCHEDULER_RESERVATION_LEASE_SECONDS: i64 = 60;
const HOST_AGENT_CAPACITY_UNAVAILABLE: &str = "host_agent_capacity_unavailable";
const WAIT_FOR_SCHEDULER_CAPACITY_ACTION: &str =
    "wait_for_active_scheduler_reservation_release_or_expiry";
const SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED: &str =
    "scheduler_reservation_atomic_batch_capacity_exceeded";
const RETRY_SMALLER_OR_CONSISTENT_RESERVATION_BATCH_ACTION: &str =
    "retry_scheduler_reservation_with_smaller_batch_or_consistent_capacity";
const RETRY_REDUCED_PROJECTED_RESERVATION_CAPACITY_ACTION: &str =
    "retry_scheduler_reservation_after_active_release_or_with_smaller_batch_or_consistent_capacity";
const RESERVATION_AUTHORITY_CANONICAL: &str = "canonical";
const RESERVATION_AUTHORITY_BACKFILL_BLOCKED: &str = "backfill_blocked";
const RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING: &str =
    "scheduler_reservation_authority_backfill_task_missing";
const RESTORE_RESERVATION_TASK_ACTION: &str =
    "restore_canonical_task_record_before_scheduler_reservation_retry";
const RESERVATION_LEASE_CREDENTIALS_REQUIRED: &str =
    "scheduler_reservation_lease_credentials_required";
const RESERVATION_LEASE_AUTHENTICATION_FAILED: &str =
    "scheduler_reservation_lease_authentication_failed";

struct ReservationAcquireGuard {
    _guard: ExclusiveFileAcquireGuard,
}

impl ReservationAcquireGuard {
    async fn acquire(root: &std::path::Path) -> Result<Self, StateStoreError> {
        Ok(Self {
            _guard: ExclusiveFileAcquireGuard::acquire(
                root,
                ExclusiveFileAcquireGuardSpec::new(
                    ".vida-scheduler-dispatch-reservation-acquire.guard",
                    RESERVATION_ACQUIRE_GUARD_RETRY_COUNT,
                    RESERVATION_ACQUIRE_GUARD_RETRY_DELAY_MS,
                    "timed out while waiting for scheduler reservation acquisition guard",
                    false,
                ),
            )
            .await?,
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub(crate) struct SchedulerDispatchReservation {
    pub reservation_id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub dispatch_receipt_id: Option<String>,
    pub launch_role: String,
    pub launch_index: u64,
    pub conflict_domain: Option<String>,
    pub scope_task_id: Option<String>,
    pub requested_current_task_id: Option<String>,
    pub selection_source: String,
    pub max_parallel_agents: u64,
    pub command: String,
    pub state_dir: String,
    pub lease_owner: String,
    pub lease_token: String,
    pub lease_status: String,
    pub reserved_at: String,
    pub lease_expires_at: String,
    pub heartbeat_at: Option<String>,
    pub released_at: Option<String>,
    pub release_reason: Option<String>,
    pub execute_status: String,
    pub blocker_codes: Vec<String>,
    pub receipt_path: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
struct SchedulerDispatchReservationAuthority {
    reservation_id: String,
    task_id: String,
    conflict_domain: Option<String>,
    execution_mode: Option<String>,
    order_bucket: Option<String>,
    parallel_group: Option<String>,
    owned_paths: Vec<String>,
    lease_seconds: i64,
    #[serde(default = "canonical_scheduler_reservation_authority_status")]
    authority_status: String,
    #[serde(default)]
    next_actions: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
struct SchedulerDispatchReservationPersistencePair {
    reservation_id: String,
    reservation: SchedulerDispatchReservation,
    authority: SchedulerDispatchReservationAuthority,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub(crate) struct SchedulerDispatchReservationEvidence {
    pub reservation_id: String,
    pub task_id: String,
    pub conflict_domain: Option<String>,
    pub execution_mode: Option<String>,
    pub order_bucket: Option<String>,
    pub parallel_group: Option<String>,
    pub owned_paths: Vec<String>,
    pub lease_seconds: i64,
    pub authority_status: String,
    pub lease_status: String,
    pub execute_status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchedulerDispatchReservationStatus {
    Reserved,
    Executing,
    Released,
    Expired,
    Blocked,
}

impl SchedulerDispatchReservationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Executing => "executing",
            Self::Released => "released",
            Self::Expired => "expired",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AcquireSchedulerDispatchReservationRequest {
    pub reservation_id: String,
    pub task_id: String,
    pub launch_role: String,
    pub launch_index: u64,
    pub conflict_domain: Option<String>,
    pub scope_task_id: Option<String>,
    pub requested_current_task_id: Option<String>,
    pub selection_source: String,
    pub max_parallel_agents: u64,
    pub command: String,
    pub state_dir: String,
    pub lease_owner: String,
    pub lease_token: String,
    pub lease_seconds: i64,
    pub dispatch_receipt_id: Option<String>,
    pub receipt_path: Option<String>,
}

fn scheduler_reservation_time() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn default_scheduler_reservation_lease_seconds() -> i64 {
    DEFAULT_SCHEDULER_RESERVATION_LEASE_SECONDS
}

fn canonical_scheduler_reservation_authority_status() -> String {
    RESERVATION_AUTHORITY_CANONICAL.to_string()
}

fn scheduler_reservation_timestamp(time: OffsetDateTime) -> String {
    time.format(&Rfc3339)
        .unwrap_or_else(|_| time.unix_timestamp_nanos().to_string())
}

fn scheduler_reservation_expiry(now: OffsetDateTime, lease_seconds: i64) -> String {
    let bounded_seconds = if lease_seconds == 0 { 1 } else { lease_seconds };
    scheduler_reservation_timestamp(now + time::Duration::seconds(bounded_seconds))
}

fn scheduler_reservation_is_expired(reservation: &SchedulerDispatchReservation, now: &str) -> bool {
    scheduler_claim::scheduler_reservation_is_expired(
        &scheduler_reservation_authority_input(reservation),
        now,
    )
}

fn scheduler_reservation_collision(
    candidate: &SchedulerDispatchReservation,
    candidate_authority: &SchedulerDispatchReservationAuthority,
    active: &[(
        SchedulerDispatchReservation,
        SchedulerDispatchReservationAuthority,
    )],
) -> Option<String> {
    if let Some((reservation, _)) = active
        .iter()
        .find(|(reservation, _)| reservation.reservation_id == candidate.reservation_id)
    {
        return Some(format!(
            "scheduler_reservation_id_already_active:{}:{}",
            candidate.reservation_id, reservation.task_id
        ));
    }
    let authority_active = active
        .iter()
        .map(|(reservation, _)| scheduler_reservation_authority_input(reservation))
        .collect::<Vec<_>>();
    scheduler_claim::decide_scheduler_reservation_collision(
        &scheduler_reservation_request_authority_input(candidate),
        &authority_active,
    )
    .or_else(|| {
        active.iter().find_map(|(reservation, authority)| {
            let blockers = scheduler_dispatch::parallel_blockers_against_current(
                scheduler_reservation_parallel_safety_input(candidate_authority),
                Some(scheduler_reservation_parallel_safety_input(authority)),
            );
            (!blockers.is_empty()).then(|| {
                format!(
                    "scheduler_active_reservation_collision:{}:{}",
                    reservation.reservation_id,
                    blockers.join(",")
                )
            })
        })
    })
}

fn scheduler_reservation_blocker_codes(blocker_codes: &[String]) -> Vec<String> {
    scheduler_claim::normalize_scheduler_reservation_blocker_codes(blocker_codes)
}

fn scheduler_reservation_request_authority_input(
    reservation: &SchedulerDispatchReservation,
) -> scheduler_claim::SchedulerReservationRequestInput {
    scheduler_claim::SchedulerReservationRequestInput {
        reservation_id: reservation.reservation_id.clone(),
        task_id: reservation.task_id.clone(),
        conflict_domain: reservation.conflict_domain.clone(),
    }
}

fn scheduler_reservation_parallel_safety_input(
    authority: &SchedulerDispatchReservationAuthority,
) -> ParallelSafetyInput<'_> {
    ParallelSafetyInput {
        task_id: authority.task_id.as_str(),
        execution_mode: authority.execution_mode.as_deref(),
        order_bucket: authority.order_bucket.as_deref(),
        parallel_group: authority.parallel_group.as_deref(),
        conflict_domain: authority.conflict_domain.as_deref(),
        owned_paths: authority.owned_paths.iter().map(String::as_str).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchedulerReservationCapacityRejection {
    blocker_code: &'static str,
    next_action: &'static str,
    active_count: usize,
    batch_count: usize,
    projected_count: usize,
    capacity: usize,
}

impl SchedulerReservationCapacityRejection {
    fn reason(&self) -> String {
        format!(
            "{}:active={}:capacity={}:batch={}:projected={}:next_action={}",
            self.blocker_code,
            self.active_count,
            self.capacity,
            self.batch_count,
            self.projected_count,
            self.next_action
        )
    }

    fn persisted_status(&self) -> String {
        if self.blocker_code == HOST_AGENT_CAPACITY_UNAVAILABLE {
            self.blocker_code.to_string()
        } else {
            self.reason()
        }
    }

    fn evidence(&self) -> (Vec<String>, Vec<String>) {
        (
            vec![self.blocker_code.to_string()],
            vec![self.next_action.to_string()],
        )
    }
}

fn scheduler_reservation_capacity_rejection(
    active_count: usize,
    batch_count: usize,
    capacity: u64,
) -> Option<SchedulerReservationCapacityRejection> {
    let capacity = capacity.max(1) as usize;
    let projected_count = active_count.saturating_add(batch_count);
    if active_count >= capacity {
        return Some(SchedulerReservationCapacityRejection {
            blocker_code: HOST_AGENT_CAPACITY_UNAVAILABLE,
            next_action: WAIT_FOR_SCHEDULER_CAPACITY_ACTION,
            active_count,
            batch_count,
            projected_count,
            capacity,
        });
    }
    (projected_count > capacity).then_some(SchedulerReservationCapacityRejection {
        blocker_code: SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED,
        next_action: if active_count == 0 {
            RETRY_SMALLER_OR_CONSISTENT_RESERVATION_BATCH_ACTION
        } else {
            RETRY_REDUCED_PROJECTED_RESERVATION_CAPACITY_ACTION
        },
        active_count,
        batch_count,
        projected_count,
        capacity,
    })
}

fn scheduler_reservation_next_actions(next_actions: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    next_actions
        .iter()
        .map(|action| action.trim())
        .filter(|action| !action.is_empty())
        .filter_map(|action| seen.insert(action.to_string()).then(|| action.to_string()))
        .collect()
}

fn scheduler_reservation_authority_input(
    reservation: &SchedulerDispatchReservation,
) -> scheduler_claim::SchedulerReservationActiveInput {
    scheduler_claim::SchedulerReservationActiveInput {
        reservation_id: reservation.reservation_id.clone(),
        task_id: reservation.task_id.clone(),
        conflict_domain: reservation.conflict_domain.clone(),
        lease_status: reservation.lease_status.clone(),
        lease_expires_at: reservation.lease_expires_at.clone(),
    }
}

fn scheduler_reservation_authority_from_task(
    reservation: &SchedulerDispatchReservation,
    task: &TaskRecord,
    lease_seconds: i64,
    next_actions: Vec<String>,
) -> SchedulerDispatchReservationAuthority {
    SchedulerDispatchReservationAuthority {
        reservation_id: reservation.reservation_id.clone(),
        task_id: reservation.task_id.clone(),
        conflict_domain: task.execution_semantics.conflict_domain.clone(),
        execution_mode: task.execution_semantics.execution_mode.clone(),
        order_bucket: task.execution_semantics.order_bucket.clone(),
        parallel_group: task.execution_semantics.parallel_group.clone(),
        owned_paths: task.planner_metadata.owned_paths.clone(),
        lease_seconds,
        authority_status: canonical_scheduler_reservation_authority_status(),
        next_actions,
    }
}

fn scheduler_reservation_unresolved_authority(
    reservation: &SchedulerDispatchReservation,
    existing: Option<SchedulerDispatchReservationAuthority>,
) -> SchedulerDispatchReservationAuthority {
    let mut authority = existing.unwrap_or_else(|| SchedulerDispatchReservationAuthority {
        reservation_id: reservation.reservation_id.clone(),
        task_id: reservation.task_id.clone(),
        conflict_domain: reservation.conflict_domain.clone(),
        execution_mode: None,
        order_bucket: None,
        parallel_group: None,
        owned_paths: Vec::new(),
        lease_seconds: default_scheduler_reservation_lease_seconds(),
        authority_status: RESERVATION_AUTHORITY_BACKFILL_BLOCKED.to_string(),
        next_actions: Vec::new(),
    });
    authority.reservation_id = reservation.reservation_id.clone();
    authority.task_id = reservation.task_id.clone();
    authority.authority_status = RESERVATION_AUTHORITY_BACKFILL_BLOCKED.to_string();
    authority.next_actions = scheduler_reservation_next_actions(
        &[
            authority.next_actions,
            vec![RESTORE_RESERVATION_TASK_ACTION.to_string()],
        ]
        .concat(),
    );
    authority
}

fn scheduler_reservation_authenticate(
    reservation: &SchedulerDispatchReservation,
    lease_owner: &str,
    lease_token: &str,
) -> Result<(), StateStoreError> {
    if lease_owner.trim().is_empty() || lease_token.trim().is_empty() {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{}",
                reservation.reservation_id
            ),
        });
    }
    if reservation.lease_owner != lease_owner || reservation.lease_token != lease_token {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "{RESERVATION_LEASE_AUTHENTICATION_FAILED}:{}",
                reservation.reservation_id
            ),
        });
    }
    Ok(())
}

impl StateStore {
    async fn persist_scheduler_dispatch_reservation_pairs(
        &self,
        pairs: &[(
            SchedulerDispatchReservation,
            SchedulerDispatchReservationAuthority,
        )],
        force_failure_after_reservation_write: bool,
    ) -> Result<(), StateStoreError> {
        if pairs.is_empty() {
            return Ok(());
        }
        let writes = pairs
            .iter()
            .map(
                |(reservation, authority)| SchedulerDispatchReservationPersistencePair {
                    reservation_id: reservation.reservation_id.clone(),
                    reservation: reservation.clone(),
                    authority: authority.clone(),
                },
            )
            .collect::<Vec<_>>();
        let response = self
            .db
            .query(
                "BEGIN TRANSACTION; \
                 FOR $write IN $writes { \
                   UPSERT type::record('scheduler_dispatch_reservation', $write.reservation_id) CONTENT $write.reservation; \
                   IF $force_failure_after_reservation_write { \
                     THROW 'scheduler_reservation_test_atomic_rollback'; \
                   }; \
                   UPSERT type::record('scheduler_dispatch_reservation_authority', $write.reservation_id) CONTENT $write.authority; \
                 }; \
                 COMMIT TRANSACTION;",
            )
            .bind(("writes", writes))
            .bind((
                "force_failure_after_reservation_write",
                force_failure_after_reservation_write,
            ))
            .await?;
        response.check()?;
        Ok(())
    }

    async fn reconcile_active_scheduler_reservation_authority_under_guard(
        &self,
        tasks: &[TaskRecord],
    ) -> Result<
        Vec<(
            SchedulerDispatchReservation,
            SchedulerDispatchReservationAuthority,
        )>,
        StateStoreError,
    > {
        let active_reservations = self.active_scheduler_dispatch_reservations().await?;
        let mut active = Vec::with_capacity(active_reservations.len());
        let mut repairs = Vec::new();
        let mut blocked_ids = Vec::new();

        for original_reservation in active_reservations {
            let existing_authority = self
                .scheduler_dispatch_reservation_authority(&original_reservation.reservation_id)
                .await?;
            let Some(task) = tasks
                .iter()
                .find(|task| task.id == original_reservation.task_id)
            else {
                let mut blocked_reservation = original_reservation.clone();
                blocked_reservation.blocker_codes = scheduler_reservation_blocker_codes(
                    &[
                        blocked_reservation.blocker_codes,
                        vec![RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING.to_string()],
                    ]
                    .concat(),
                );
                let blocked_authority = scheduler_reservation_unresolved_authority(
                    &blocked_reservation,
                    existing_authority,
                );
                blocked_ids.push(blocked_reservation.reservation_id.clone());
                repairs.push((blocked_reservation.clone(), blocked_authority.clone()));
                active.push((blocked_reservation, blocked_authority));
                continue;
            };

            let mut reservation = original_reservation.clone();
            reservation.conflict_domain = task.execution_semantics.conflict_domain.clone();
            reservation
                .blocker_codes
                .retain(|code| code != RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING);
            reservation.blocker_codes =
                scheduler_reservation_blocker_codes(&reservation.blocker_codes);
            let lease_seconds = existing_authority
                .as_ref()
                .map(|authority| authority.lease_seconds)
                .unwrap_or_else(default_scheduler_reservation_lease_seconds);
            let next_actions = existing_authority
                .as_ref()
                .map(|authority| {
                    authority
                        .next_actions
                        .iter()
                        .filter(|action| action.as_str() != RESTORE_RESERVATION_TASK_ACTION)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let authority = scheduler_reservation_authority_from_task(
                &reservation,
                task,
                lease_seconds,
                scheduler_reservation_next_actions(&next_actions),
            );
            if reservation != original_reservation
                || existing_authority.as_ref() != Some(&authority)
            {
                repairs.push((reservation.clone(), authority.clone()));
            }
            active.push((reservation, authority));
        }

        self.persist_scheduler_dispatch_reservation_pairs(&repairs, false)
            .await?;
        if !blocked_ids.is_empty() {
            blocked_ids.sort();
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "scheduler_active_reservation_authority_backfill_blocked:{}:blocker_code={RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING}:next_action={RESTORE_RESERVATION_TASK_ACTION}",
                    blocked_ids.join(",")
                ),
            });
        }
        Ok(active)
    }

    async fn expire_active_scheduler_reservations_under_guard(
        &self,
        active: &mut Vec<(
            SchedulerDispatchReservation,
            SchedulerDispatchReservationAuthority,
        )>,
    ) -> Result<usize, StateStoreError> {
        let now = scheduler_reservation_timestamp(scheduler_reservation_time());
        let mut retained = Vec::with_capacity(active.len());
        let mut stale = Vec::new();
        for (mut reservation, authority) in active.drain(..) {
            if scheduler_reservation_is_expired(&reservation, &now) {
                reservation.lease_status = SchedulerDispatchReservationStatus::Expired
                    .as_str()
                    .to_string();
                reservation.execute_status = SchedulerDispatchReservationStatus::Expired
                    .as_str()
                    .to_string();
                reservation.released_at = Some(now.clone());
                reservation.release_reason = Some("lease_expired".to_string());
                stale.push((reservation, authority));
            } else {
                retained.push((reservation, authority));
            }
        }
        let stale_count = stale.len();
        self.persist_scheduler_dispatch_reservation_pairs(&stale, false)
            .await?;
        *active = retained;
        Ok(stale_count)
    }

    pub(crate) async fn expire_stale_scheduler_dispatch_reservations(
        &self,
    ) -> Result<usize, StateStoreError> {
        let _guard = ReservationAcquireGuard::acquire(self.root()).await?;
        let tasks = self.all_tasks().await?;
        let mut active = self
            .reconcile_active_scheduler_reservation_authority_under_guard(&tasks)
            .await?;
        self.expire_active_scheduler_reservations_under_guard(&mut active)
            .await
    }

    pub(crate) async fn active_scheduler_dispatch_reservations(
        &self,
    ) -> Result<Vec<SchedulerDispatchReservation>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM scheduler_dispatch_reservation \
                 WHERE lease_status IN ['reserved', 'executing'] \
                 ORDER BY reserved_at DESC, reservation_id DESC;",
            )
            .await?;
        let rows: Vec<SchedulerDispatchReservation> = query.take(0)?;
        Ok(rows)
    }

    #[allow(dead_code)]
    pub(crate) async fn scheduler_dispatch_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<Option<SchedulerDispatchReservation>, StateStoreError> {
        let row: Option<SchedulerDispatchReservation> = self
            .db
            .select(("scheduler_dispatch_reservation", reservation_id))
            .await?;
        Ok(row)
    }

    async fn scheduler_dispatch_reservation_authority(
        &self,
        reservation_id: &str,
    ) -> Result<Option<SchedulerDispatchReservationAuthority>, StateStoreError> {
        let row: Option<SchedulerDispatchReservationAuthority> = self
            .db
            .select(("scheduler_dispatch_reservation_authority", reservation_id))
            .await?;
        Ok(row)
    }

    pub(crate) async fn scheduler_dispatch_reservation_evidence(
        &self,
        reservation_id: &str,
    ) -> Result<Option<SchedulerDispatchReservationEvidence>, StateStoreError> {
        let Some(reservation) = self.scheduler_dispatch_reservation(reservation_id).await? else {
            return Ok(None);
        };
        let authority = self
            .scheduler_dispatch_reservation_authority(reservation_id)
            .await?
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "scheduler_reservation_evidence_authority_missing:{reservation_id}"
                ),
            })?;
        Ok(Some(SchedulerDispatchReservationEvidence {
            reservation_id: reservation.reservation_id,
            task_id: reservation.task_id,
            conflict_domain: authority.conflict_domain,
            execution_mode: authority.execution_mode,
            order_bucket: authority.order_bucket,
            parallel_group: authority.parallel_group,
            owned_paths: authority.owned_paths,
            lease_seconds: authority.lease_seconds,
            authority_status: authority.authority_status,
            lease_status: reservation.lease_status,
            execute_status: reservation.execute_status,
            blocker_codes: reservation.blocker_codes,
            next_actions: authority.next_actions,
        }))
    }

    pub(crate) async fn acquire_scheduler_dispatch_reservations(
        &self,
        requests: &[AcquireSchedulerDispatchReservationRequest],
    ) -> Result<Vec<SchedulerDispatchReservation>, StateStoreError> {
        self.acquire_scheduler_dispatch_reservations_with_persistence_fault(requests, false)
            .await
    }

    async fn acquire_scheduler_dispatch_reservations_with_persistence_fault(
        &self,
        requests: &[AcquireSchedulerDispatchReservationRequest],
        force_failure_after_reservation_write: bool,
    ) -> Result<Vec<SchedulerDispatchReservation>, StateStoreError> {
        let _guard = ReservationAcquireGuard::acquire(self.root()).await?;
        let tasks = self.all_tasks().await?;
        let mut active = self
            .reconcile_active_scheduler_reservation_authority_under_guard(&tasks)
            .await?;
        self.expire_active_scheduler_reservations_under_guard(&mut active)
            .await?;
        let persisted_active_count = active.len();
        let batch_count = requests.len();
        active.reserve(requests.len());
        let now = scheduler_reservation_time();
        let reserved_at = scheduler_reservation_timestamp(now);
        let mut reservations = Vec::new();

        for request in requests {
            let task = tasks
                .iter()
                .find(|task| task.id == request.task_id)
                .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "scheduler_reservation_task_identity_missing:{}",
                        request.task_id
                    ),
                })?;
            let reservation = SchedulerDispatchReservation {
                reservation_id: request.reservation_id.clone(),
                task_id: request.task_id.clone(),
                run_id: None,
                dispatch_receipt_id: request.dispatch_receipt_id.clone(),
                launch_role: request.launch_role.clone(),
                launch_index: request.launch_index,
                conflict_domain: task.execution_semantics.conflict_domain.clone(),
                scope_task_id: request.scope_task_id.clone(),
                requested_current_task_id: request.requested_current_task_id.clone(),
                selection_source: request.selection_source.clone(),
                max_parallel_agents: request.max_parallel_agents,
                command: request.command.clone(),
                state_dir: request.state_dir.clone(),
                lease_owner: request.lease_owner.clone(),
                lease_token: request.lease_token.clone(),
                lease_status: SchedulerDispatchReservationStatus::Reserved
                    .as_str()
                    .to_string(),
                reserved_at: reserved_at.clone(),
                lease_expires_at: scheduler_reservation_expiry(now, request.lease_seconds),
                heartbeat_at: None,
                released_at: None,
                release_reason: None,
                execute_status: "reserved".to_string(),
                blocker_codes: Vec::new(),
                receipt_path: request.receipt_path.clone(),
            };
            let mut authority = scheduler_reservation_authority_from_task(
                &reservation,
                task,
                request.lease_seconds,
                Vec::new(),
            );
            if request.lease_owner.trim().is_empty() || request.lease_token.trim().is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{}",
                        request.reservation_id
                    ),
                });
            }
            if request.conflict_domain != reservation.conflict_domain {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "scheduler_reservation_task_identity_mismatch:{}:conflict_domain",
                        request.task_id
                    ),
                });
            }
            if let Some(reason) = scheduler_reservation_collision(&reservation, &authority, &active)
            {
                return Err(StateStoreError::InvalidTaskRecord { reason });
            }
            if let Some(rejection) = scheduler_reservation_capacity_rejection(
                persisted_active_count,
                batch_count,
                request.max_parallel_agents,
            ) {
                let reason = rejection.reason();
                let persisted_status = rejection.persisted_status();
                let (blocker_codes, next_actions) = rejection.evidence();
                let mut blocked_reservation = reservation;
                blocked_reservation.lease_status = SchedulerDispatchReservationStatus::Blocked
                    .as_str()
                    .to_string();
                blocked_reservation.execute_status = persisted_status.clone();
                blocked_reservation.blocker_codes =
                    scheduler_reservation_blocker_codes(&blocker_codes);
                blocked_reservation.released_at = Some(reserved_at.clone());
                blocked_reservation.release_reason = Some(persisted_status);
                authority.next_actions = scheduler_reservation_next_actions(&next_actions);
                self.persist_scheduler_dispatch_reservation_pairs(
                    &[(blocked_reservation, authority)],
                    force_failure_after_reservation_write,
                )
                .await?;
                return Err(StateStoreError::InvalidTaskRecord { reason });
            }
            active.push((reservation.clone(), authority.clone()));
            reservations.push((reservation, authority));
        }

        self.persist_scheduler_dispatch_reservation_pairs(
            &reservations,
            force_failure_after_reservation_write,
        )
        .await?;
        Ok(reservations
            .into_iter()
            .map(|(reservation, _)| reservation)
            .collect())
    }

    #[allow(dead_code)]
    pub(crate) async fn mark_scheduler_dispatch_reservation_executing(
        &self,
        reservation_id: &str,
        _run_id: Option<&str>,
        _execute_status: &str,
    ) -> Result<(), StateStoreError> {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!("{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{reservation_id}"),
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn mark_scheduler_dispatch_reservation_executing_checked(
        &self,
        reservation_id: &str,
        lease_owner: &str,
        lease_token: &str,
        run_id: Option<&str>,
        execute_status: &str,
    ) -> Result<(), StateStoreError> {
        let _guard = ReservationAcquireGuard::acquire(self.root()).await?;
        let tasks = self.all_tasks().await?;
        let mut active = self
            .reconcile_active_scheduler_reservation_authority_under_guard(&tasks)
            .await?;
        self.expire_active_scheduler_reservations_under_guard(&mut active)
            .await?;
        let Some((mut reservation, authority)) = active
            .into_iter()
            .find(|(reservation, _)| reservation.reservation_id == reservation_id)
        else {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler_reservation_not_active_for_execution:{reservation_id}"),
            });
        };
        scheduler_reservation_authenticate(&reservation, lease_owner, lease_token)?;
        let now = scheduler_reservation_time();
        reservation.lease_status = SchedulerDispatchReservationStatus::Executing
            .as_str()
            .to_string();
        reservation.run_id = run_id.map(str::to_string);
        reservation.execute_status = execute_status.to_string();
        reservation.heartbeat_at = Some(scheduler_reservation_timestamp(now));
        reservation.lease_expires_at = scheduler_reservation_expiry(now, authority.lease_seconds);
        self.persist_scheduler_dispatch_reservation_pairs(&[(reservation, authority)], false)
            .await
    }

    #[allow(dead_code)]
    pub(crate) async fn heartbeat_scheduler_dispatch_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<(), StateStoreError> {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!("{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{reservation_id}"),
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn heartbeat_scheduler_dispatch_reservation_checked(
        &self,
        reservation_id: &str,
        lease_owner: &str,
        lease_token: &str,
    ) -> Result<(), StateStoreError> {
        let _guard = ReservationAcquireGuard::acquire(self.root()).await?;
        let tasks = self.all_tasks().await?;
        let mut active = self
            .reconcile_active_scheduler_reservation_authority_under_guard(&tasks)
            .await?;
        self.expire_active_scheduler_reservations_under_guard(&mut active)
            .await?;
        let Some((mut reservation, authority)) = active
            .into_iter()
            .find(|(reservation, _)| reservation.reservation_id == reservation_id)
        else {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler_reservation_not_active_for_heartbeat:{reservation_id}"),
            });
        };
        scheduler_reservation_authenticate(&reservation, lease_owner, lease_token)?;
        let now = scheduler_reservation_time();
        reservation.heartbeat_at = Some(scheduler_reservation_timestamp(now));
        reservation.lease_expires_at = scheduler_reservation_expiry(now, authority.lease_seconds);
        self.persist_scheduler_dispatch_reservation_pairs(&[(reservation, authority)], false)
            .await
    }

    pub(crate) async fn release_scheduler_dispatch_reservation(
        &self,
        reservation_id: &str,
        _reason: &str,
    ) -> Result<(), StateStoreError> {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!("{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{reservation_id}"),
        })
    }

    pub(crate) async fn release_scheduler_dispatch_reservation_with_blockers(
        &self,
        reservation_id: &str,
        _reason: &str,
        _blocker_codes: &[String],
    ) -> Result<(), StateStoreError> {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!("{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{reservation_id}"),
        })
    }

    pub(crate) async fn release_scheduler_dispatch_reservation_with_evidence(
        &self,
        reservation_id: &str,
        _reason: &str,
        _blocker_codes: &[String],
        _next_actions: &[String],
    ) -> Result<(), StateStoreError> {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!("{RESERVATION_LEASE_CREDENTIALS_REQUIRED}:{reservation_id}"),
        })
    }

    pub(crate) async fn release_scheduler_dispatch_reservation_checked(
        &self,
        reservation_id: &str,
        lease_owner: &str,
        lease_token: &str,
        reason: &str,
    ) -> Result<(), StateStoreError> {
        self.release_scheduler_dispatch_reservation_with_evidence_checked(
            reservation_id,
            lease_owner,
            lease_token,
            reason,
            &[],
            &[],
        )
        .await
    }

    pub(crate) async fn release_scheduler_dispatch_reservation_with_blockers_checked(
        &self,
        reservation_id: &str,
        lease_owner: &str,
        lease_token: &str,
        reason: &str,
        blocker_codes: &[String],
    ) -> Result<(), StateStoreError> {
        self.release_scheduler_dispatch_reservation_with_evidence_checked(
            reservation_id,
            lease_owner,
            lease_token,
            reason,
            blocker_codes,
            &[],
        )
        .await
    }

    pub(crate) async fn release_scheduler_dispatch_reservation_with_evidence_checked(
        &self,
        reservation_id: &str,
        lease_owner: &str,
        lease_token: &str,
        reason: &str,
        blocker_codes: &[String],
        next_actions: &[String],
    ) -> Result<(), StateStoreError> {
        let _guard = ReservationAcquireGuard::acquire(self.root()).await?;
        let tasks = self.all_tasks().await?;
        let mut active = self
            .reconcile_active_scheduler_reservation_authority_under_guard(&tasks)
            .await?;
        self.expire_active_scheduler_reservations_under_guard(&mut active)
            .await?;
        let Some((mut reservation, mut authority)) = active
            .into_iter()
            .find(|(reservation, _)| reservation.reservation_id == reservation_id)
        else {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler_reservation_not_active_for_release:{reservation_id}"),
            });
        };
        scheduler_reservation_authenticate(&reservation, lease_owner, lease_token)?;
        reservation.lease_status = SchedulerDispatchReservationStatus::Released
            .as_str()
            .to_string();
        if !reason.trim().is_empty() {
            reservation.execute_status = reason.to_string();
        }
        reservation.blocker_codes = scheduler_reservation_blocker_codes(blocker_codes);
        authority.next_actions = scheduler_reservation_next_actions(next_actions);
        reservation.released_at =
            Some(scheduler_reservation_timestamp(scheduler_reservation_time()));
        reservation.release_reason = Some(reason.to_string());
        self.persist_scheduler_dispatch_reservation_pairs(&[(reservation, authority)], false)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_record_with_semantics(
        task_id: &str,
        execution_mode: Option<&str>,
        order_bucket: Option<&str>,
        parallel_group: Option<&str>,
        conflict_domain: Option<&str>,
        owned_paths: &[&str],
    ) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            updated_at: "1".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics {
                execution_mode: execution_mode.map(str::to_string),
                order_bucket: order_bucket.map(str::to_string),
                parallel_group: parallel_group.map(str::to_string),
                conflict_domain: conflict_domain.map(str::to_string),
            },
            planner_metadata: TaskPlannerMetadata {
                owned_paths: owned_paths.iter().map(|path| (*path).to_string()).collect(),
                ..TaskPlannerMetadata::default()
            },
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn task_record(task_id: &str, conflict_domain: &str, owned_paths: &[&str]) -> TaskRecord {
        task_record_with_semantics(
            task_id,
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some(conflict_domain),
            owned_paths,
        )
    }

    async fn persist_task(
        store: &StateStore,
        task_id: &str,
        conflict_domain: &str,
        owned_paths: &[&str],
    ) {
        store
            .persist_task_record(task_record(task_id, conflict_domain, owned_paths))
            .await
            .expect("persist task");
    }

    async fn persist_task_with_semantics(
        store: &StateStore,
        task_id: &str,
        execution_mode: Option<&str>,
        order_bucket: Option<&str>,
        parallel_group: Option<&str>,
        conflict_domain: Option<&str>,
        owned_paths: &[&str],
    ) {
        store
            .persist_task_record(task_record_with_semantics(
                task_id,
                execution_mode,
                order_bucket,
                parallel_group,
                conflict_domain,
                owned_paths,
            ))
            .await
            .expect("persist task with semantics");
    }

    fn reservation_request(
        reservation_id: &str,
        task_id: &str,
        conflict_domain: Option<&str>,
    ) -> AcquireSchedulerDispatchReservationRequest {
        AcquireSchedulerDispatchReservationRequest {
            reservation_id: reservation_id.to_string(),
            task_id: task_id.to_string(),
            launch_role: "primary".to_string(),
            launch_index: 0,
            conflict_domain: conflict_domain.map(str::to_string),
            scope_task_id: None,
            requested_current_task_id: None,
            selection_source: "test".to_string(),
            max_parallel_agents: 2,
            command: "vida agent-init --json".to_string(),
            state_dir: "/tmp/vida-state".to_string(),
            lease_owner: "test-owner".to_string(),
            lease_token: format!("token-{reservation_id}"),
            lease_seconds: 60,
            dispatch_receipt_id: Some("receipt-1".to_string()),
            receipt_path: Some("/tmp/receipt.json".to_string()),
        }
    }

    fn temp_state_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vida-scheduler-reservation-{name}-{nanos}"))
    }

    async fn delete_test_record(store: &StateStore, table: &str, record_id: &str) {
        let response = store
            .db
            .query(format!("DELETE type::record('{table}', $record_id);"))
            .bind(("record_id", record_id.to_string()))
            .await
            .expect("delete test record");
        response.check().expect("delete statement should pass");
    }

    async fn acquire_one(
        store: &StateStore,
        reservation_id: &str,
        task_id: &str,
        conflict_domain: Option<&str>,
    ) -> SchedulerDispatchReservation {
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                reservation_id,
                task_id,
                conflict_domain,
            )])
            .await
            .expect("reservation should acquire")
            .into_iter()
            .next()
            .expect("one reservation should return")
    }

    #[tokio::test]
    async fn scheduler_reservation_z_zero_empty_acquire_is_a_persisted_noop() {
        let root = temp_state_dir("empty");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let reservations = store
            .acquire_scheduler_dispatch_reservations(&[])
            .await
            .expect("empty acquire should pass");

        assert!(reservations.is_empty());
        assert!(
            store
                .active_scheduler_dispatch_reservations()
                .await
                .expect("read active reservations")
                .is_empty()
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_o_one_persists_canonical_identity_and_authority() {
        let root = temp_state_dir("identity-reopen");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(
            &store,
            "task-1",
            "domain-a",
            &["crates/vida/src/reservation.rs"],
        )
        .await;

        let acquired = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("reservation should acquire");
        assert_eq!(acquired[0].task_id, "task-1");
        let authority = store
            .scheduler_dispatch_reservation_authority("reservation-1")
            .await
            .expect("read authority")
            .expect("authority should persist");
        assert_eq!(authority.execution_mode.as_deref(), Some("parallel_safe"));
        assert_eq!(authority.order_bucket.as_deref(), Some("wave-1"));
        assert_eq!(authority.parallel_group.as_deref(), Some("writers"));
        assert_eq!(authority.authority_status, RESERVATION_AUTHORITY_CANONICAL);
        assert_eq!(
            authority.owned_paths,
            vec!["crates/vida/src/reservation.rs"]
        );

        drop(store);
        let reopened = StateStore::open(root.clone()).await.expect("reopen store");
        let persisted = reopened
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read reservation")
            .expect("reservation should persist across reopen");
        assert_eq!(persisted, acquired[0]);
        let persisted_authority = reopened
            .scheduler_dispatch_reservation_authority("reservation-1")
            .await
            .expect("read reopened authority")
            .expect("authority should persist across reopen");
        assert_eq!(persisted_authority, authority);
        let evidence = reopened
            .scheduler_dispatch_reservation_evidence("reservation-1")
            .await
            .expect("read reopened evidence")
            .expect("evidence should project persisted authority");
        assert_eq!(evidence.conflict_domain.as_deref(), Some("domain-a"));
        assert_eq!(evidence.execution_mode.as_deref(), Some("parallel_safe"));
        assert_eq!(evidence.order_bucket.as_deref(), Some("wave-1"));
        assert_eq!(evidence.parallel_group.as_deref(), Some("writers"));
        assert_eq!(evidence.owned_paths, vec!["crates/vida/src/reservation.rs"]);
        assert_eq!(evidence.lease_seconds, 60);
        assert_eq!(evidence.authority_status, RESERVATION_AUTHORITY_CANONICAL);
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_m_many_persists_all_pairs_atomically() {
        let root = temp_state_dir("many");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-b", &["crates/b"]).await;
        let mut first = reservation_request("reservation-1", "task-1", Some("domain-a"));
        let mut second = reservation_request("reservation-2", "task-2", Some("domain-b"));
        first.max_parallel_agents = 2;
        second.max_parallel_agents = 2;

        let acquired = store
            .acquire_scheduler_dispatch_reservations(&[first, second])
            .await
            .expect("multi-request acquire should commit together");

        assert_eq!(acquired.len(), 2);
        for reservation_id in ["reservation-1", "reservation-2"] {
            assert!(
                store
                    .scheduler_dispatch_reservation(reservation_id)
                    .await
                    .expect("read reservation")
                    .is_some()
            );
            assert!(
                store
                    .scheduler_dispatch_reservation_authority(reservation_id)
                    .await
                    .expect("read authority")
                    .is_some()
            );
        }
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_atomic_rollback_leaves_neither_table_partially_written() {
        let root = temp_state_dir("atomic-rollback");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-b", &["crates/b"]).await;
        let mut first = reservation_request("reservation-1", "task-1", Some("domain-a"));
        let mut second = reservation_request("reservation-2", "task-2", Some("domain-b"));
        first.max_parallel_agents = 2;
        second.max_parallel_agents = 2;

        store
            .acquire_scheduler_dispatch_reservations_with_persistence_fault(&[first, second], true)
            .await
            .expect_err("in-transaction fault should roll back every write");

        for reservation_id in ["reservation-1", "reservation-2"] {
            assert!(
                store
                    .scheduler_dispatch_reservation(reservation_id)
                    .await
                    .expect("read reservation")
                    .is_none()
            );
            match store
                .scheduler_dispatch_reservation_authority(reservation_id)
                .await
            {
                Ok(None) => {}
                Err(error) if error.to_string().contains("does not exist") => {}
                other => panic!("rollback must leave no authority row: {other:?}"),
            }
        }
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_migration_backfills_missing_authority_from_task_truth() {
        let root = temp_state_dir("migration-backfill");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(
            &store,
            "task-1",
            "domain-a",
            &["crates/vida/src/reservation.rs"],
        )
        .await;
        acquire_one(&store, "reservation-1", "task-1", Some("domain-a")).await;
        delete_test_record(
            &store,
            "scheduler_dispatch_reservation_authority",
            "reservation-1",
        )
        .await;

        store
            .acquire_scheduler_dispatch_reservations(&[])
            .await
            .expect("legacy authority should backfill under acquire guard");

        let authority = store
            .scheduler_dispatch_reservation_authority("reservation-1")
            .await
            .expect("read backfill")
            .expect("authority should be recreated");
        assert_eq!(authority.task_id, "task-1");
        assert_eq!(authority.conflict_domain.as_deref(), Some("domain-a"));
        assert_eq!(authority.execution_mode.as_deref(), Some("parallel_safe"));
        assert_eq!(authority.order_bucket.as_deref(), Some("wave-1"));
        assert_eq!(authority.parallel_group.as_deref(), Some("writers"));
        assert_eq!(
            authority.owned_paths,
            vec!["crates/vida/src/reservation.rs"]
        );
        assert_eq!(
            authority.lease_seconds,
            DEFAULT_SCHEDULER_RESERVATION_LEASE_SECONDS
        );
        assert_eq!(authority.authority_status, RESERVATION_AUTHORITY_CANONICAL);
        let evidence = store
            .scheduler_dispatch_reservation_evidence("reservation-1")
            .await
            .expect("read backfilled evidence")
            .expect("backfilled authority should be publicly queryable");
        assert_eq!(evidence.conflict_domain.as_deref(), Some("domain-a"));
        assert_eq!(evidence.execution_mode.as_deref(), Some("parallel_safe"));
        assert_eq!(evidence.order_bucket.as_deref(), Some("wave-1"));
        assert_eq!(evidence.parallel_group.as_deref(), Some("writers"));
        assert_eq!(evidence.owned_paths, vec!["crates/vida/src/reservation.rs"]);
        assert_eq!(
            evidence.lease_seconds,
            DEFAULT_SCHEDULER_RESERVATION_LEASE_SECONDS
        );
        assert_eq!(evidence.authority_status, RESERVATION_AUTHORITY_CANONICAL);
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_e_exception_backfill_failure_persists_actionable_evidence() {
        let root = temp_state_dir("migration-blocked");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        acquire_one(&store, "reservation-1", "task-1", Some("domain-a")).await;
        delete_test_record(
            &store,
            "scheduler_dispatch_reservation_authority",
            "reservation-1",
        )
        .await;
        delete_test_record(&store, "task", "task-1").await;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[])
            .await
            .expect_err("missing canonical task must fail closed");
        assert!(
            error
                .to_string()
                .contains(RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING)
        );
        assert!(error.to_string().contains(RESTORE_RESERVATION_TASK_ACTION));
        let evidence = store
            .scheduler_dispatch_reservation_evidence("reservation-1")
            .await
            .expect("query persisted evidence")
            .expect("blocked evidence should exist");
        assert_eq!(
            evidence.blocker_codes,
            vec![RESERVATION_AUTHORITY_BACKFILL_TASK_MISSING]
        );
        assert_eq!(evidence.next_actions, vec![RESTORE_RESERVATION_TASK_ACTION]);
        assert_eq!(evidence.lease_status, "reserved");
        assert_eq!(evidence.conflict_domain.as_deref(), Some("domain-a"));
        assert_eq!(evidence.execution_mode, None);
        assert_eq!(evidence.order_bucket, None);
        assert_eq!(evidence.parallel_group, None);
        assert!(evidence.owned_paths.is_empty());
        assert_eq!(
            evidence.lease_seconds,
            DEFAULT_SCHEDULER_RESERVATION_LEASE_SECONDS
        );
        assert_eq!(
            evidence.authority_status,
            RESERVATION_AUTHORITY_BACKFILL_BLOCKED
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_collision_blocks_duplicate_reservation_id() {
        let root = temp_state_dir("duplicate-reservation-id");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-b", &["crates/b"]).await;
        acquire_one(&store, "reservation-1", "task-1", Some("domain-a")).await;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-2",
                Some("domain-b"),
            )])
            .await
            .expect_err("duplicate reservation id should block overwrite");

        assert!(
            error
                .to_string()
                .contains("scheduler_reservation_id_already_active:reservation-1:task-1")
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_collision_blocks_duplicate_task() {
        let root = temp_state_dir("duplicate-task");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("first reservation should acquire");

        let error = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-2",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect_err("duplicate task should block");

        assert!(
            error
                .to_string()
                .contains("scheduler_task_already_reserved:task-1:reservation-1")
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_collision_blocks_conflict_domain() {
        let root = temp_state_dir("conflict-domain");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-a", &["crates/b"]).await;
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("first reservation should acquire");

        let error = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-2",
                "task-2",
                Some("domain-a"),
            )])
            .await
            .expect_err("conflict domain should block");

        assert!(
            error
                .to_string()
                .contains("scheduler_conflict_domain_reserved:domain-a:reservation-1")
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_request_domain_must_match_canonical_task_metadata() {
        let root = temp_state_dir("request-domain-mismatch");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-b"),
            )])
            .await
            .expect_err("request domain cannot override task authority");

        assert!(
            error
                .to_string()
                .contains("scheduler_reservation_task_identity_mismatch:task-1:conflict_domain")
        );
        assert!(
            store
                .scheduler_dispatch_reservation("reservation-1")
                .await
                .expect("read reservation")
                .is_none()
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_collision_matrix_covers_every_pairwise_metadata_class() {
        struct CollisionCase {
            name: &'static str,
            current_execution_mode: Option<&'static str>,
            current_order_bucket: Option<&'static str>,
            current_parallel_group: Option<&'static str>,
            current_conflict_domain: Option<&'static str>,
            current_paths: &'static [&'static str],
            candidate_execution_mode: Option<&'static str>,
            candidate_order_bucket: Option<&'static str>,
            candidate_parallel_group: Option<&'static str>,
            candidate_conflict_domain: Option<&'static str>,
            candidate_paths: &'static [&'static str],
            expected_blocker: &'static str,
        }
        let cases = [
            CollisionCase {
                name: "candidate-execution-mode",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("sequential"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "execution_mode_not_parallel_safe",
            },
            CollisionCase {
                name: "current-execution-mode",
                current_execution_mode: Some("sequential"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "current_execution_mode_not_parallel_safe",
            },
            CollisionCase {
                name: "candidate-missing-owned-paths",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &[],
                expected_blocker: "missing_owned_paths_for_parallel_execution",
            },
            CollisionCase {
                name: "current-missing-owned-paths",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &[],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "current_missing_owned_paths_for_parallel_execution",
            },
            CollisionCase {
                name: "owned-path-overlap",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/shared/src"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/shared/src/lib.rs"],
                expected_blocker: "owned_path_collision",
            },
            CollisionCase {
                name: "order-bucket",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-2"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "order_bucket_mismatch_or_missing",
            },
            CollisionCase {
                name: "missing-conflict-domain",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: None,
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("writers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "missing_conflict_domain",
            },
            CollisionCase {
                name: "parallel-group",
                current_execution_mode: Some("parallel_safe"),
                current_order_bucket: Some("wave-1"),
                current_parallel_group: Some("writers"),
                current_conflict_domain: Some("domain-a"),
                current_paths: &["crates/a"],
                candidate_execution_mode: Some("parallel_safe"),
                candidate_order_bucket: Some("wave-1"),
                candidate_parallel_group: Some("reviewers"),
                candidate_conflict_domain: Some("domain-b"),
                candidate_paths: &["crates/b"],
                expected_blocker: "parallel_group_mismatch",
            },
        ];

        for case in cases {
            let root = temp_state_dir(case.name);
            let store = StateStore::open(root.clone()).await.expect("open store");
            persist_task_with_semantics(
                &store,
                "task-1",
                case.current_execution_mode,
                case.current_order_bucket,
                case.current_parallel_group,
                case.current_conflict_domain,
                case.current_paths,
            )
            .await;
            persist_task_with_semantics(
                &store,
                "task-2",
                case.candidate_execution_mode,
                case.candidate_order_bucket,
                case.candidate_parallel_group,
                case.candidate_conflict_domain,
                case.candidate_paths,
            )
            .await;
            acquire_one(
                &store,
                "reservation-1",
                "task-1",
                case.current_conflict_domain,
            )
            .await;
            let error = store
                .acquire_scheduler_dispatch_reservations(&[reservation_request(
                    "reservation-2",
                    "task-2",
                    case.candidate_conflict_domain,
                )])
                .await
                .expect_err("pairwise collision should block");
            assert!(
                error.to_string().contains(case.expected_blocker),
                "{} should include {}: {}",
                case.name,
                case.expected_blocker,
                error
            );
            drop(store);
            let _ = fs::remove_dir_all(root);
        }
    }

    #[tokio::test]
    async fn scheduler_reservation_pairwise_collision_checks_every_active_reservation() {
        let root = temp_state_dir("collision-every-active");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-b", &["crates/b"]).await;
        persist_task(&store, "task-3", "domain-c", &["crates/b/child"]).await;
        let mut first = reservation_request("reservation-1", "task-1", Some("domain-a"));
        let mut second = reservation_request("reservation-2", "task-2", Some("domain-b"));
        first.max_parallel_agents = 3;
        second.max_parallel_agents = 3;
        store
            .acquire_scheduler_dispatch_reservations(&[first, second])
            .await
            .expect("two disjoint reservations should acquire");
        let mut candidate = reservation_request("reservation-3", "task-3", Some("domain-c"));
        candidate.max_parallel_agents = 3;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[candidate])
            .await
            .expect_err("candidate must collide with the second active reservation");

        assert!(
            error.to_string().contains(
                "scheduler_active_reservation_collision:reservation-2:owned_path_collision"
            )
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_doubt_atomic_batch_capacity_never_reports_tentative_as_active() {
        // Assumption: capacity evidence reports only persisted active reservations.
        // Doubt: an earlier tentative request could be mislabeled active when a later request rejects.
        // Test: empty store + A(max=2) + B(max=1) rejects atomically with active=0 and no A row.
        let root = temp_state_dir("batch-capacity-tentative-active");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-a", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-b", "domain-b", &["crates/b"]).await;
        let mut request_a = reservation_request("reservation-a", "task-a", Some("domain-a"));
        let mut request_b = reservation_request("reservation-b", "task-b", Some("domain-b"));
        request_a.max_parallel_agents = 2;
        request_b.max_parallel_agents = 1;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[request_a, request_b])
            .await
            .expect_err("incompatible batch capacity should reject atomically");
        let reason = error.to_string();
        assert!(reason.contains(SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED));
        assert!(reason.contains("active=0:capacity=1:batch=2:projected=2"));
        assert!(reason.contains(RETRY_SMALLER_OR_CONSISTENT_RESERVATION_BATCH_ACTION));
        assert!(!reason.contains(HOST_AGENT_CAPACITY_UNAVAILABLE));
        assert!(
            store
                .scheduler_dispatch_reservation("reservation-a")
                .await
                .expect("read tentative A")
                .is_none()
        );
        assert!(
            store
                .scheduler_dispatch_reservation_authority("reservation-a")
                .await
                .expect("read tentative A authority")
                .is_none()
        );
        let evidence = store
            .scheduler_dispatch_reservation_evidence("reservation-b")
            .await
            .expect("read rejected B evidence")
            .expect("rejected B evidence should persist");
        assert_eq!(
            evidence.blocker_codes,
            vec![SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED]
        );
        assert_eq!(
            evidence.next_actions,
            vec![RETRY_SMALLER_OR_CONSISTENT_RESERVATION_BATCH_ACTION]
        );
        assert!(
            evidence
                .execute_status
                .contains("active=0:capacity=1:batch=2:projected=2")
        );
        assert!(
            store
                .active_scheduler_dispatch_reservations()
                .await
                .expect("read active reservations")
                .is_empty()
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_batch_capacity_checks_earlier_request_against_final_projection()
    {
        let root = temp_state_dir("batch-capacity-earlier-boundary");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-a", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-b", "domain-b", &["crates/b"]).await;
        let mut request_a = reservation_request("reservation-a", "task-a", Some("domain-a"));
        let mut request_b = reservation_request("reservation-b", "task-b", Some("domain-b"));
        request_a.max_parallel_agents = 1;
        request_b.max_parallel_agents = 2;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[request_a, request_b])
            .await
            .expect_err("final batch size must satisfy every request capacity");

        assert!(
            error
                .to_string()
                .contains("active=0:capacity=1:batch=2:projected=2")
        );
        let evidence = store
            .scheduler_dispatch_reservation_evidence("reservation-a")
            .await
            .expect("read rejected A evidence")
            .expect("earliest incompatible request should persist evidence");
        assert_eq!(
            evidence.blocker_codes,
            vec![SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED]
        );
        assert!(
            store
                .scheduler_dispatch_reservation("reservation-b")
                .await
                .expect("read unprocessed B")
                .is_none()
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_i_interface_capacity_rejection_is_persisted_and_queryable() {
        let root = temp_state_dir("capacity");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-b", &["crates/b"]).await;
        let mut first = reservation_request("reservation-1", "task-1", Some("domain-a"));
        first.max_parallel_agents = 1;
        store
            .acquire_scheduler_dispatch_reservations(&[first])
            .await
            .expect("first reservation should acquire");
        let mut second = reservation_request("reservation-2", "task-2", Some("domain-b"));
        second.max_parallel_agents = 1;

        let error = store
            .acquire_scheduler_dispatch_reservations(&[second])
            .await
            .expect_err("capacity should block second reservation");
        let evidence = error.to_string();
        assert!(evidence.contains(HOST_AGENT_CAPACITY_UNAVAILABLE));
        assert!(evidence.contains(WAIT_FOR_SCHEDULER_CAPACITY_ACTION));
        assert!(evidence.contains("active=1:capacity=1"));
        let persisted = store
            .scheduler_dispatch_reservation_evidence("reservation-2")
            .await
            .expect("capacity evidence query should pass")
            .expect("capacity rejection should persist");
        assert_eq!(persisted.lease_status, "blocked");
        assert_eq!(persisted.execute_status, HOST_AGENT_CAPACITY_UNAVAILABLE);
        assert_eq!(
            persisted.blocker_codes,
            vec![HOST_AGENT_CAPACITY_UNAVAILABLE]
        );
        assert_eq!(
            persisted.next_actions,
            vec![WAIT_FOR_SCHEDULER_CAPACITY_ACTION]
        );
        assert_eq!(
            store
                .active_scheduler_dispatch_reservations()
                .await
                .expect("active reservations")
                .len(),
            1
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_e_exception_unauthenticated_legacy_mutators_fail_closed() {
        let root = temp_state_dir("legacy-auth-block");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        acquire_one(&store, "reservation-1", "task-1", Some("domain-a")).await;

        for error in [
            store
                .mark_scheduler_dispatch_reservation_executing(
                    "reservation-1",
                    Some("run-1"),
                    "executing",
                )
                .await
                .expect_err("legacy executing mutation should block"),
            store
                .heartbeat_scheduler_dispatch_reservation("reservation-1")
                .await
                .expect_err("legacy heartbeat mutation should block"),
            store
                .release_scheduler_dispatch_reservation("reservation-1", "completed")
                .await
                .expect_err("legacy release mutation should block"),
        ] {
            assert!(
                error
                    .to_string()
                    .contains(RESERVATION_LEASE_CREDENTIALS_REQUIRED)
            );
        }
        let reservation = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read reservation")
            .expect("reservation remains active");
        assert_eq!(reservation.lease_status, "reserved");
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_wrong_lease_owner_or_token_never_mutates_lifecycle() {
        let root = temp_state_dir("wrong-lease-credentials");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        acquire_one(&store, "reservation-1", "task-1", Some("domain-a")).await;
        let before = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read reservation")
            .expect("reservation exists");

        let errors = [
            store
                .mark_scheduler_dispatch_reservation_executing_checked(
                    "reservation-1",
                    "wrong-owner",
                    "token-reservation-1",
                    Some("run-1"),
                    "executing",
                )
                .await
                .expect_err("wrong owner should block executing"),
            store
                .heartbeat_scheduler_dispatch_reservation_checked(
                    "reservation-1",
                    "test-owner",
                    "wrong-token",
                )
                .await
                .expect_err("wrong token should block heartbeat"),
            store
                .release_scheduler_dispatch_reservation_checked(
                    "reservation-1",
                    "wrong-owner",
                    "wrong-token",
                    "completed",
                )
                .await
                .expect_err("wrong credentials should block release"),
        ];
        for error in errors {
            assert!(
                error
                    .to_string()
                    .contains(RESERVATION_LEASE_AUTHENTICATION_FAILED)
            );
            assert!(!error.to_string().contains("token-reservation-1"));
        }
        let after = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read reservation")
            .expect("reservation exists");
        assert_eq!(after, before);
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_release_removes_active_collision() {
        let root = temp_state_dir("release");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-a", &["crates/b"]).await;
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("first reservation should acquire");
        store
            .release_scheduler_dispatch_reservation_checked(
                "reservation-1",
                "test-owner",
                "token-reservation-1",
                "test_release",
            )
            .await
            .expect("release should persist");

        let reservations = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-2",
                "task-2",
                Some("domain-a"),
            )])
            .await
            .expect("released reservation should not collide");

        assert_eq!(reservations.len(), 1);
        assert_eq!(reservations[0].reservation_id, "reservation-2");
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_release_persists_canonical_blocker_codes_and_actions() {
        let root = temp_state_dir("release-blockers");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("reservation should acquire");

        store
            .release_scheduler_dispatch_reservation_with_evidence_checked(
                "reservation-1",
                "test-owner",
                "token-reservation-1",
                "activation_view_only",
                &[
                    "scheduler_agent_init_activation_view_only".to_string(),
                    "scheduler_agent_init_activation_view_only".to_string(),
                ],
                &[
                    " retry_agent_init_with_execution ".to_string(),
                    "retry_agent_init_with_execution".to_string(),
                ],
            )
            .await
            .expect("release should persist blocker truth");

        let evidence = store
            .scheduler_dispatch_reservation_evidence("reservation-1")
            .await
            .expect("evidence should read")
            .expect("released evidence should remain queryable");
        assert_eq!(evidence.execute_status, "activation_view_only");
        assert_eq!(
            evidence.blocker_codes,
            vec!["scheduler_agent_init_activation_view_only"]
        );
        assert_eq!(
            evidence.next_actions,
            vec!["retry_agent_init_with_execution"]
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_b_boundary_authenticated_heartbeat_expiry_and_release() {
        let root = temp_state_dir("heartbeat");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("reservation should acquire");
        store
            .mark_scheduler_dispatch_reservation_executing_checked(
                "reservation-1",
                "test-owner",
                "token-reservation-1",
                Some("run-1"),
                "executing",
            )
            .await
            .expect("execution identity should persist");
        let before = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read reservation")
            .expect("reservation exists");
        std::thread::sleep(std::time::Duration::from_millis(2));

        store
            .heartbeat_scheduler_dispatch_reservation_checked(
                "reservation-1",
                "test-owner",
                "token-reservation-1",
            )
            .await
            .expect("heartbeat should renew active lease");
        let renewed = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read renewed reservation")
            .expect("renewed reservation exists");
        assert_eq!(renewed.task_id, "task-1");
        assert_eq!(renewed.run_id.as_deref(), Some("run-1"));
        assert_eq!(
            renewed.lease_status,
            SchedulerDispatchReservationStatus::Executing.as_str()
        );
        assert!(renewed.heartbeat_at > before.heartbeat_at);
        assert!(renewed.lease_expires_at > before.lease_expires_at);

        store
            .release_scheduler_dispatch_reservation_checked(
                "reservation-1",
                "test-owner",
                "token-reservation-1",
                "completed",
            )
            .await
            .expect("release should persist");
        let released = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("read released reservation")
            .expect("released reservation remains queryable");
        assert_eq!(released.task_id, "task-1");
        assert_eq!(released.run_id.as_deref(), Some("run-1"));
        assert_eq!(
            released.lease_status,
            SchedulerDispatchReservationStatus::Released.as_str()
        );
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_expired_lease_can_be_reclaimed() {
        let root = temp_state_dir("expired");
        let store = StateStore::open(root.clone()).await.expect("open store");
        persist_task(&store, "task-1", "domain-a", &["crates/a"]).await;
        persist_task(&store, "task-2", "domain-a", &["crates/b"]).await;
        let mut expired = reservation_request("reservation-1", "task-1", Some("domain-a"));
        expired.lease_seconds = -1;
        store
            .acquire_scheduler_dispatch_reservations(&[expired])
            .await
            .expect("expired reservation should initially persist");

        let reservations = store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-2",
                "task-2",
                Some("domain-a"),
            )])
            .await
            .expect("expired reservation should not collide");

        assert_eq!(reservations.len(), 1);
        assert_eq!(reservations[0].reservation_id, "reservation-2");
        let expired = store
            .scheduler_dispatch_reservation("reservation-1")
            .await
            .expect("reservation should read")
            .expect("expired reservation should remain queryable");
        assert_eq!(
            expired.lease_status,
            SchedulerDispatchReservationStatus::Expired.as_str()
        );
        assert_eq!(expired.execute_status, "expired");
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scheduler_reservation_s_simple_normalizes_capacity_evidence() {
        let persisted_saturation = scheduler_reservation_capacity_rejection(2, 1, 2)
            .expect("persisted saturation should reject");
        assert_eq!(
            persisted_saturation.blocker_code,
            HOST_AGENT_CAPACITY_UNAVAILABLE
        );
        assert_eq!(
            persisted_saturation.next_action,
            WAIT_FOR_SCHEDULER_CAPACITY_ACTION
        );
        assert_eq!(persisted_saturation.active_count, 2);
        assert_eq!(persisted_saturation.projected_count, 3);
        let atomic_batch = scheduler_reservation_capacity_rejection(0, 2, 1)
            .expect("internally incompatible batch should reject");
        assert_eq!(
            atomic_batch.blocker_code,
            SCHEDULER_RESERVATION_ATOMIC_BATCH_CAPACITY_EXCEEDED
        );
        assert_eq!(atomic_batch.active_count, 0);
        assert_eq!(atomic_batch.batch_count, 2);
        assert_eq!(atomic_batch.projected_count, 2);
        let mixed_projection = scheduler_reservation_capacity_rejection(1, 2, 2)
            .expect("active plus batch projection should reject");
        assert_eq!(
            mixed_projection.next_action,
            RETRY_REDUCED_PROJECTED_RESERVATION_CAPACITY_ACTION
        );
        assert_eq!(scheduler_reservation_capacity_rejection(1, 1, 2), None);
        assert_eq!(
            scheduler_reservation_next_actions(&[
                " retry ".to_string(),
                "retry".to_string(),
                String::new(),
            ]),
            vec!["retry"]
        );
    }
}
