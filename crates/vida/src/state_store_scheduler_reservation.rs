#![allow(dead_code)]

use super::*;

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

fn scheduler_reservation_timestamp(time: OffsetDateTime) -> String {
    time.format(&Rfc3339)
        .unwrap_or_else(|_| time.unix_timestamp_nanos().to_string())
}

fn scheduler_reservation_expiry(now: OffsetDateTime, lease_seconds: i64) -> String {
    let bounded_seconds = if lease_seconds == 0 { 1 } else { lease_seconds };
    scheduler_reservation_timestamp(now + time::Duration::seconds(bounded_seconds))
}

fn scheduler_reservation_is_active(status: &str) -> bool {
    matches!(status, "reserved" | "executing")
}

fn scheduler_reservation_is_expired(reservation: &SchedulerDispatchReservation, now: &str) -> bool {
    scheduler_reservation_is_active(&reservation.lease_status)
        && !reservation.lease_expires_at.trim().is_empty()
        && reservation.lease_expires_at.as_str() <= now
}

fn scheduler_reservation_collision(
    request: &AcquireSchedulerDispatchReservationRequest,
    active: &[SchedulerDispatchReservation],
) -> Option<String> {
    for reservation in active {
        if reservation.task_id == request.task_id {
            return Some(format!(
                "scheduler_task_already_reserved:{}:{}",
                request.task_id, reservation.reservation_id
            ));
        }
        if let (Some(left), Some(right)) = (
            request.conflict_domain.as_deref(),
            reservation.conflict_domain.as_deref(),
        ) {
            if !left.trim().is_empty() && left == right {
                return Some(format!(
                    "scheduler_conflict_domain_reserved:{}:{}",
                    left, reservation.reservation_id
                ));
            }
        }
    }
    None
}

impl StateStore {
    pub(crate) async fn expire_stale_scheduler_dispatch_reservations(
        &self,
    ) -> Result<usize, StateStoreError> {
        let now = scheduler_reservation_timestamp(scheduler_reservation_time());
        let active = self.active_scheduler_dispatch_reservations().await?;
        let stale = active
            .into_iter()
            .filter(|reservation| scheduler_reservation_is_expired(reservation, &now))
            .collect::<Vec<_>>();
        for mut reservation in stale.iter().cloned() {
            reservation.lease_status = SchedulerDispatchReservationStatus::Expired
                .as_str()
                .to_string();
            reservation.released_at = Some(now.clone());
            reservation.release_reason = Some("lease_expired".to_string());
            let _: Option<SchedulerDispatchReservation> = self
                .db
                .upsert((
                    "scheduler_dispatch_reservation",
                    reservation.reservation_id.as_str(),
                ))
                .content(reservation)
                .await?;
        }
        Ok(stale.len())
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

    pub(crate) async fn acquire_scheduler_dispatch_reservations(
        &self,
        requests: &[AcquireSchedulerDispatchReservationRequest],
    ) -> Result<Vec<SchedulerDispatchReservation>, StateStoreError> {
        self.expire_stale_scheduler_dispatch_reservations().await?;
        let mut active = self.active_scheduler_dispatch_reservations().await?;
        let now = scheduler_reservation_time();
        let reserved_at = scheduler_reservation_timestamp(now);
        let mut reservations = Vec::new();

        for request in requests {
            if let Some(reason) = scheduler_reservation_collision(request, &active) {
                return Err(StateStoreError::InvalidTaskRecord { reason });
            }
            let reservation = SchedulerDispatchReservation {
                reservation_id: request.reservation_id.clone(),
                task_id: request.task_id.clone(),
                run_id: None,
                dispatch_receipt_id: request.dispatch_receipt_id.clone(),
                launch_role: request.launch_role.clone(),
                launch_index: request.launch_index,
                conflict_domain: request.conflict_domain.clone(),
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
            active.push(reservation.clone());
            reservations.push(reservation);
        }

        for reservation in &reservations {
            let _: Option<SchedulerDispatchReservation> = self
                .db
                .upsert((
                    "scheduler_dispatch_reservation",
                    reservation.reservation_id.as_str(),
                ))
                .content(reservation.clone())
                .await?;
        }
        Ok(reservations)
    }

    #[allow(dead_code)]
    pub(crate) async fn mark_scheduler_dispatch_reservation_executing(
        &self,
        reservation_id: &str,
        run_id: Option<&str>,
        execute_status: &str,
    ) -> Result<(), StateStoreError> {
        let mut reservation = self
            .scheduler_dispatch_reservation(reservation_id)
            .await?
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler reservation not found: {reservation_id}"),
            })?;
        reservation.lease_status = SchedulerDispatchReservationStatus::Executing
            .as_str()
            .to_string();
        reservation.run_id = run_id.map(str::to_string);
        reservation.execute_status = execute_status.to_string();
        reservation.heartbeat_at =
            Some(scheduler_reservation_timestamp(scheduler_reservation_time()));
        let _: Option<SchedulerDispatchReservation> = self
            .db
            .upsert(("scheduler_dispatch_reservation", reservation_id))
            .content(reservation)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn heartbeat_scheduler_dispatch_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<(), StateStoreError> {
        let mut reservation = self
            .scheduler_dispatch_reservation(reservation_id)
            .await?
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler reservation not found: {reservation_id}"),
            })?;
        reservation.heartbeat_at =
            Some(scheduler_reservation_timestamp(scheduler_reservation_time()));
        let _: Option<SchedulerDispatchReservation> = self
            .db
            .upsert(("scheduler_dispatch_reservation", reservation_id))
            .content(reservation)
            .await?;
        Ok(())
    }

    pub(crate) async fn release_scheduler_dispatch_reservation(
        &self,
        reservation_id: &str,
        reason: &str,
    ) -> Result<(), StateStoreError> {
        let mut reservation = self
            .scheduler_dispatch_reservation(reservation_id)
            .await?
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!("scheduler reservation not found: {reservation_id}"),
            })?;
        reservation.lease_status = SchedulerDispatchReservationStatus::Released
            .as_str()
            .to_string();
        reservation.released_at =
            Some(scheduler_reservation_timestamp(scheduler_reservation_time()));
        reservation.release_reason = Some(reason.to_string());
        let _: Option<SchedulerDispatchReservation> = self
            .db
            .upsert(("scheduler_dispatch_reservation", reservation_id))
            .content(reservation)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn scheduler_reservation_acquire_blocks_duplicate_task() {
        let root = temp_state_dir("duplicate-task");
        let store = StateStore::open(root.clone()).await.expect("open store");
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
                Some("domain-b"),
            )])
            .await
            .expect_err("duplicate task should block");

        assert!(error
            .to_string()
            .contains("scheduler_task_already_reserved:task-1:reservation-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_acquire_blocks_conflict_domain() {
        let root = temp_state_dir("conflict-domain");
        let store = StateStore::open(root.clone()).await.expect("open store");
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

        assert!(error
            .to_string()
            .contains("scheduler_conflict_domain_reserved:domain-a:reservation-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_release_removes_active_collision() {
        let root = temp_state_dir("release");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .acquire_scheduler_dispatch_reservations(&[reservation_request(
                "reservation-1",
                "task-1",
                Some("domain-a"),
            )])
            .await
            .expect("first reservation should acquire");
        store
            .release_scheduler_dispatch_reservation("reservation-1", "test_release")
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
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduler_reservation_expired_lease_can_be_reclaimed() {
        let root = temp_state_dir("expired");
        let store = StateStore::open(root.clone()).await.expect("open store");
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
        let _ = fs::remove_dir_all(root);
    }
}
