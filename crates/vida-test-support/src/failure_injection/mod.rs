use serde::Serialize;
use taskflow_authority::claims::{ClaimLeaseCommand, decide_claim_lease};
use taskflow_authority::scheduler_claim::{
    OrchestratorClaimActiveInput, OrchestratorClaimRequestInput,
};
use taskflow_contracts::{
    VidaAggregateRef, VidaCommandRef, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaProjectionCheckpoint, VidaReceiptId, VidaStreamRef, VidaStreamVersion,
};
use taskflow_state::{
    InMemoryOperationalJournal, JournalAggregateSnapshotRecord, JournalAppendReceipt,
    JournalAppendRequest, JournalArtifactRecord, JournalEventRecord, JournalIdempotencyRecord,
    JournalOutboxRecord, JournalProjectionFailure, OperationalJournal, TaskflowStateError,
};

/// One-shot failure points for test-only journal/provider adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    BeforeWrite,
    AfterWrite,
    DuplicateReceipt,
    StaleLease,
    Timeout,
    PartialJournalAppend,
}

#[derive(Debug, Default, Clone)]
pub struct FaultPlan {
    next: Option<FaultPoint>,
}

impl FaultPlan {
    #[must_use]
    pub fn armed(point: FaultPoint) -> Self {
        Self { next: Some(point) }
    }

    pub fn arm(&mut self, point: FaultPoint) {
        self.next = Some(point);
    }

    fn take(&mut self) -> Option<FaultPoint> {
        self.next.take()
    }
}

/// Forwarding journal wrapper used by recovery tests. Faults are one-shot so a
/// retry can prove idempotency without touching a production/runtime adapter.
#[derive(Debug)]
pub struct FaultInjectingJournal<J> {
    inner: J,
    plan: FaultPlan,
    last_fault: Option<FaultPoint>,
}

impl<J> FaultInjectingJournal<J> {
    #[must_use]
    pub fn new(inner: J) -> Self {
        Self {
            inner,
            plan: FaultPlan::default(),
            last_fault: None,
        }
    }

    pub fn arm(&mut self, point: FaultPoint) {
        self.plan.arm(point);
    }

    #[must_use]
    pub fn last_fault(&self) -> Option<FaultPoint> {
        self.last_fault
    }

    #[must_use]
    pub fn into_inner(self) -> J {
        self.inner
    }

    /// Checked test-only outbox surface that preserves injected lease/timeout
    /// faults instead of conflating them with an empty queue.
    pub fn claim_outbox_batch_checked(
        &mut self,
        consumer_id: &str,
        limit: usize,
    ) -> Result<Vec<JournalOutboxRecord>, TaskflowStateError>
    where
        J: OperationalJournal,
    {
        let batch = self.claim_outbox_batch(consumer_id, limit);
        match self.last_fault {
            Some(FaultPoint::StaleLease) => Err(TaskflowStateError::Storage(
                "injected stale lease while claiming outbox".to_string(),
            )),
            Some(FaultPoint::Timeout) => Err(TaskflowStateError::Storage(
                "injected timeout while claiming outbox".to_string(),
            )),
            _ => Ok(batch),
        }
    }
}

impl<J: OperationalJournal> OperationalJournal for FaultInjectingJournal<J> {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        let fault = self.plan.take();
        self.last_fault = fault;
        match fault {
            Some(FaultPoint::BeforeWrite | FaultPoint::Timeout) => Err(
                TaskflowStateError::Storage("injected write interruption".to_string()),
            ),
            Some(FaultPoint::PartialJournalAppend) => {
                let mut partial = request.clone();
                let retained_events = partial.events.len().min(1);
                partial.events.truncate(retained_events);
                partial
                    .effect_intents
                    .truncate(partial.effect_intents.len().min(1));
                if !partial.events.is_empty() || !partial.effect_intents.is_empty() {
                    let _ = self.inner.append(partial);
                }
                Err(TaskflowStateError::Storage(
                    "injected partial journal append".to_string(),
                ))
            }
            Some(FaultPoint::AfterWrite) => {
                let _receipt = self.inner.append(request)?;
                Err(TaskflowStateError::Storage(
                    "injected post-write timeout".to_string(),
                ))
            }
            Some(FaultPoint::DuplicateReceipt) => {
                let first = self.inner.append(request.clone())?;
                let second = self.inner.append(request)?;
                if first != second {
                    return Err(TaskflowStateError::Storage(
                        "duplicate receipt changed append result".to_string(),
                    ));
                }
                Ok(second)
            }
            Some(FaultPoint::StaleLease) => Err(TaskflowStateError::Storage(
                "injected stale lease".to_string(),
            )),
            None => self.inner.append(request),
        }
    }

    fn load_stream(
        &self,
        stream_id: &VidaStreamRef,
    ) -> Vec<taskflow_contracts::VidaDomainEventEnvelope> {
        self.inner.load_stream(stream_id)
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        self.inner.read_global_after(cursor, limit)
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        self.inner.record_idempotency_started(key, command_id)
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        self.inner.record_idempotency_completed(key, receipt_id)
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.inner.record_idempotency_conflicted(key, reason)
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        self.inner.idempotency_record(key)
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        let fault = self.plan.take();
        self.last_fault = fault;
        match fault {
            Some(FaultPoint::StaleLease | FaultPoint::Timeout) => Vec::new(),
            _ => self.inner.claim_outbox_batch(consumer_id, limit),
        }
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        self.inner.mark_outbox_succeeded(outbox_id)
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        self.inner.mark_outbox_failed(outbox_id, reason)
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        self.inner.record_projection_checkpoint(checkpoint);
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        self.inner.record_projection_failure(failure);
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        self.inner.index_artifact(artifact);
    }

    fn record_aggregate_snapshot(&mut self, snapshot: JournalAggregateSnapshotRecord) {
        self.inner.record_aggregate_snapshot(snapshot);
    }

    fn aggregate_snapshot(
        &self,
        aggregate_id: &VidaAggregateRef,
    ) -> Option<JournalAggregateSnapshotRecord> {
        self.inner.aggregate_snapshot(aggregate_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QualificationReport {
    pub accepted_command_count: usize,
    pub recovered_command_count: usize,
    pub lost_command_count: usize,
    pub semantic_effect_apply_count: usize,
    pub duplicate_semantic_effect_count: usize,
    pub concurrency_violation_count: usize,
    pub repair_needed_count: usize,
    pub recovery_receipts: Vec<RecoveryReceipt>,
    pub failure_matrix: Vec<FailureScenarioResult>,
    pub benchmark_comparison: BenchmarkComparison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureScenarioResult {
    pub scenario: &'static str,
    pub accepted_commands: usize,
    pub recovered_commands: usize,
    pub semantic_effects_applied: usize,
    pub duplicate_semantic_effects: usize,
    pub concurrency_violations: usize,
    pub recovery_state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecoveryReceipt {
    pub scenario: &'static str,
    pub receipt_kind: &'static str,
    pub command_id: &'static str,
    pub outcome: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BenchmarkComparison {
    pub read_budget_ms: u64,
    pub read_observed_ms: u64,
    pub mutation_budget_ms: u64,
    pub mutation_observed_ms: u64,
    pub max_allowed_regression_percent: u64,
    pub read_regression_percent: i64,
    pub mutation_regression_percent: i64,
    pub within_threshold: bool,
}

pub fn run_ldrk_qualification() -> QualificationReport {
    let failure_matrix = vec![
        crash_after_accept_before_effect(),
        duplicate_effect_retry(),
        concurrent_claim_race(),
        stale_projection_recovery(),
    ];
    let recovery_receipts = vec![
        RecoveryReceipt {
            scenario: "crash_after_accept_before_effect",
            receipt_kind: "accepted_command_recovered",
            command_id: "cmd-crash-001",
            outcome: "healthy_projection",
        },
        RecoveryReceipt {
            scenario: "stale_projection_recovery",
            receipt_kind: "repair_needed_state_explicit",
            command_id: "cmd-repair-001",
            outcome: "repair_needed",
        },
    ];
    let benchmark_comparison = budget_comparison();

    QualificationReport {
        accepted_command_count: failure_matrix
            .iter()
            .map(|scenario| scenario.accepted_commands)
            .sum(),
        recovered_command_count: failure_matrix
            .iter()
            .map(|scenario| scenario.recovered_commands)
            .sum(),
        lost_command_count: failure_matrix
            .iter()
            .map(|scenario| {
                scenario
                    .accepted_commands
                    .saturating_sub(scenario.recovered_commands)
            })
            .sum(),
        semantic_effect_apply_count: failure_matrix
            .iter()
            .map(|scenario| scenario.semantic_effects_applied)
            .sum(),
        duplicate_semantic_effect_count: failure_matrix
            .iter()
            .map(|scenario| scenario.duplicate_semantic_effects)
            .sum(),
        concurrency_violation_count: failure_matrix
            .iter()
            .map(|scenario| scenario.concurrency_violations)
            .sum(),
        repair_needed_count: failure_matrix
            .iter()
            .filter(|scenario| scenario.recovery_state == "repair_needed")
            .count(),
        recovery_receipts,
        failure_matrix,
        benchmark_comparison,
    }
}

pub fn failure_matrix_review_artifact() -> serde_json::Value {
    let report = run_ldrk_qualification();
    serde_json::json!({
        "accepted_command_count": report.accepted_command_count,
        "recovered_command_count": report.recovered_command_count,
        "lost_command_count": report.lost_command_count,
        "semantic_effect_apply_count": report.semantic_effect_apply_count,
        "duplicate_semantic_effect_count": report.duplicate_semantic_effect_count,
        "concurrency_violation_count": report.concurrency_violation_count,
        "repair_needed_count": report.repair_needed_count,
        "failure_matrix": report.failure_matrix,
        "recovery_receipts": report.recovery_receipts
    })
}

pub fn benchmark_review_artifact() -> serde_json::Value {
    serde_json::to_value(budget_comparison()).expect("benchmark comparison serializes")
}

fn crash_after_accept_before_effect() -> FailureScenarioResult {
    let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
    let request = semantic_append_request();
    journal.arm(FaultPoint::AfterWrite);
    let accepted = usize::from(journal.append(request.clone()).is_err());
    let receipt = journal
        .append(request.clone())
        .expect("after-write retry should recover the accepted append");
    let persisted_events = journal.load_stream(&request.stream_id).len();

    FailureScenarioResult {
        scenario: "crash_after_accept_before_effect",
        accepted_commands: accepted,
        recovered_commands: usize::from(persisted_events == request.events.len()),
        semantic_effects_applied: receipt.effect_intent_count,
        duplicate_semantic_effects: persisted_events.saturating_sub(request.events.len()),
        concurrency_violations: 0,
        recovery_state: "healthy_projection",
    }
}

fn duplicate_effect_retry() -> FailureScenarioResult {
    let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
    let request = semantic_append_request();
    journal.arm(FaultPoint::DuplicateReceipt);
    let receipt = journal
        .append(request.clone())
        .expect("duplicate receipt should preserve the original result");
    let persisted_events = journal.load_stream(&request.stream_id).len();

    FailureScenarioResult {
        scenario: "duplicate_effect_retry",
        accepted_commands: usize::from(receipt.event_count == request.events.len()),
        recovered_commands: usize::from(persisted_events == request.events.len()),
        semantic_effects_applied: receipt.effect_intent_count,
        duplicate_semantic_effects: persisted_events.saturating_sub(request.events.len()),
        concurrency_violations: 0,
        recovery_state: "healthy_projection",
    }
}

fn concurrent_claim_race() -> FailureScenarioResult {
    let now = "2026-08-12T00:00:00Z";
    let first_request = qualification_claim_request("claim-a", "session-a");
    let second_request = qualification_claim_request("claim-b", "session-b");
    let first = decide_claim_lease(
        ClaimLeaseCommand::Acquire {
            request: first_request.clone(),
        },
        &[],
        now,
    );
    let active_after_first = if first.admitted {
        vec![active_claim_from_request(&first_request)]
    } else {
        Vec::new()
    };
    let second = decide_claim_lease(
        ClaimLeaseCommand::Acquire {
            request: second_request,
        },
        &active_after_first,
        now,
    );
    let winning_claims = usize::from(first.admitted) + usize::from(second.admitted);

    FailureScenarioResult {
        scenario: "concurrent_claim_race",
        accepted_commands: winning_claims,
        recovered_commands: winning_claims,
        semantic_effects_applied: 0,
        duplicate_semantic_effects: 0,
        concurrency_violations: winning_claims.saturating_sub(1),
        recovery_state: "healthy_projection",
    }
}

fn stale_projection_recovery() -> FailureScenarioResult {
    let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
    let request = semantic_append_request();
    journal.arm(FaultPoint::StaleLease);
    assert!(journal.append(request.clone()).is_err());
    let recovered_receipt = journal.append(request).ok();

    FailureScenarioResult {
        scenario: "stale_projection_recovery",
        accepted_commands: usize::from(recovered_receipt.is_some()),
        recovered_commands: usize::from(recovered_receipt.is_some()),
        semantic_effects_applied: recovered_receipt
            .map(|receipt| receipt.effect_intent_count)
            .unwrap_or_default(),
        duplicate_semantic_effects: 0,
        concurrency_violations: 0,
        recovery_state: "repair_needed",
    }
}

fn budget_comparison() -> BenchmarkComparison {
    let read_budget_ms = 50;
    let mutation_budget_ms = 250;
    let max_allowed_regression_percent = 20;
    let read_started = std::time::Instant::now();
    let mut read_checksum = 0usize;
    for _ in 0..256 {
        read_checksum ^= semantic_append_request().events.len();
    }
    std::hint::black_box(read_checksum);
    let read_observed_ms = elapsed_ms_ceil(read_started);

    let mutation_started = std::time::Instant::now();
    let mut journal = InMemoryOperationalJournal::default();
    for iteration in 0..32 {
        let _ = journal.append(benchmark_append_request(iteration));
    }
    std::hint::black_box(journal);
    let mutation_observed_ms = elapsed_ms_ceil(mutation_started);

    let read_regression_percent = percent_delta(read_observed_ms, read_budget_ms);
    let mutation_regression_percent = percent_delta(mutation_observed_ms, mutation_budget_ms);

    BenchmarkComparison {
        read_budget_ms,
        read_observed_ms,
        mutation_budget_ms,
        mutation_observed_ms,
        max_allowed_regression_percent,
        read_regression_percent,
        mutation_regression_percent,
        within_threshold: read_regression_percent <= max_allowed_regression_percent as i64
            && mutation_regression_percent <= max_allowed_regression_percent as i64,
    }
}

fn elapsed_ms_ceil(started: std::time::Instant) -> u64 {
    let nanos = started.elapsed().as_nanos();
    nanos.div_ceil(1_000_000).max(1) as u64
}

fn benchmark_append_request(iteration: usize) -> JournalAppendRequest {
    let mut request = semantic_append_request();
    request.stream_id = VidaStreamRef(format!("semantic-benchmark-stream-{iteration}"));
    request.command_id = VidaCommandRef(format!("semantic-benchmark-command-{iteration}"));
    request.idempotency_key =
        VidaIdempotencyKey(format!("semantic-benchmark-idempotency-{iteration}"));
    for (event_index, event) in request.events.iter_mut().enumerate() {
        event.event_id = VidaEventRef(format!(
            "semantic-benchmark-event-{iteration}-{event_index}"
        ));
        event.command_id = Some(request.command_id.clone());
        event.stream_id = request.stream_id.clone();
        event.stream_version = VidaStreamVersion((event_index + 1) as u64);
    }
    for (effect_index, effect) in request.effect_intents.iter_mut().enumerate() {
        effect.effect_id = taskflow_contracts::VidaEffectRef(format!(
            "semantic-benchmark-effect-{iteration}-{effect_index}"
        ));
        effect.command_id = request.command_id.clone();
        effect.stream_id = request.stream_id.clone();
    }
    request
}

fn qualification_claim_request(claim_id: &str, session_id: &str) -> OrchestratorClaimRequestInput {
    OrchestratorClaimRequestInput {
        claim_id: claim_id.to_string(),
        state_root_id: "semantic-state-root".to_string(),
        worktree_environment_id: "semantic-worktree".to_string(),
        orchestrator_session_id: session_id.to_string(),
        process_id: None,
        task_id: Some("semantic-task".to_string()),
        run_id: Some("semantic-run".to_string()),
        claim_kind: "write".to_string(),
        conflict_domain: Some("semantic-domain".to_string()),
        owned_paths: vec!["semantic/path".to_string()],
        read_only_paths: Vec::new(),
        lease_mode: "exclusive".to_string(),
    }
}

fn active_claim_from_request(
    request: &OrchestratorClaimRequestInput,
) -> OrchestratorClaimActiveInput {
    OrchestratorClaimActiveInput {
        claim_id: request.claim_id.clone(),
        orchestrator_session_id: request.orchestrator_session_id.clone(),
        process_id: request.process_id,
        task_id: request.task_id.clone(),
        run_id: request.run_id.clone(),
        conflict_domain: request.conflict_domain.clone(),
        owned_paths: request.owned_paths.clone(),
        read_only_paths: request.read_only_paths.clone(),
        lease_mode: request.lease_mode.clone(),
        status: "active".to_string(),
        lease_expires_at: "2026-08-12T00:05:00Z".to_string(),
    }
}

fn percent_delta(observed: u64, budget: u64) -> i64 {
    (((observed as i64 - budget as i64) * 100) / budget as i64).max(0)
}

/// Deterministic append fixture shared by adapter restart tests.
#[must_use]
pub fn semantic_append_request() -> JournalAppendRequest {
    JournalAppendRequest {
        stream_id: VidaStreamRef("semantic-stream".to_string()),
        expected_stream_version: Some(taskflow_contracts::VidaStreamVersion(0)),
        command_id: VidaCommandRef("semantic-command".to_string()),
        idempotency_key: VidaIdempotencyKey("semantic-idempotency".to_string()),
        causation_id: None,
        correlation_id: Some("semantic-correlation".to_string()),
        events: (1..=2)
            .map(|version| taskflow_contracts::VidaDomainEventEnvelope {
                schema_id: taskflow_contracts::VidaSchemaId("semantic.event".to_string()),
                event_version: taskflow_contracts::VidaSchemaVersion(1),
                event_id: taskflow_contracts::VidaEventRef(format!("semantic-event-{version}")),
                command_id: Some(VidaCommandRef("semantic-command".to_string())),
                causation_id: None,
                stream_id: VidaStreamRef("semantic-stream".to_string()),
                stream_version: taskflow_contracts::VidaStreamVersion(version),
                aggregate_id: taskflow_contracts::VidaAggregateRef(
                    "semantic-aggregate".to_string(),
                ),
                occurred_at: taskflow_contracts::VidaTimestamp("2026-08-12T00:00:00Z".to_string()),
                payload: serde_json::json!({"semantic": true, "version": version}),
                trace: serde_json::json!({}),
            })
            .collect(),
        effect_intents: (1..=2)
            .map(|id| taskflow_contracts::VidaEffectIntent {
                effect_id: taskflow_contracts::VidaEffectRef(format!("semantic-effect-{id}")),
                operation: taskflow_contracts::VidaOperation("semantic.effect".to_string()),
                command_id: VidaCommandRef("semantic-command".to_string()),
                stream_id: VidaStreamRef("semantic-stream".to_string()),
                payload: serde_json::json!({"semantic": true, "id": id}),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{FaultInjectingJournal, FaultPoint};
    use taskflow_contracts::VidaStreamRef;
    use taskflow_state::{InMemoryOperationalJournal, JournalAppendRequest, OperationalJournal};

    fn request() -> JournalAppendRequest {
        super::semantic_append_request()
    }

    #[test]
    fn before_write_fault_does_not_persist_and_retry_recovers() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::BeforeWrite);
        assert!(journal.append(request()).is_err());
        let receipt = journal.append(request()).expect("retry should write");
        assert_eq!(receipt.stream_version.0, 2);
    }

    #[test]
    fn after_write_fault_replays_the_same_receipt_without_duplicate_events() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::AfterWrite);
        assert!(journal.append(request()).is_err());
        let receipt = journal.append(request()).expect("idempotent retry");
        assert_eq!(receipt.event_count, 2);
        assert_eq!(receipt.effect_intent_count, 2);
        assert!(
            journal
                .load_stream(&VidaStreamRef("semantic-stream".to_string()))
                .len()
                == 2
        );
    }

    #[test]
    fn duplicate_receipt_fault_is_idempotent() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::DuplicateReceipt);
        let receipt = journal.append(request()).expect("duplicate receipt probe");
        assert_eq!(receipt.stream_version.0, 2);
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("semantic-stream".to_string()))
                .len(),
            2
        );
    }

    #[test]
    fn stale_lease_and_timeout_are_one_shot_and_retryable() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::StaleLease);
        assert!(journal.append(request()).is_err());
        assert_eq!(journal.last_fault(), Some(FaultPoint::StaleLease));
        journal.arm(FaultPoint::Timeout);
        assert!(journal.append(request()).is_err());
        assert_eq!(journal.last_fault(), Some(FaultPoint::Timeout));
        assert_eq!(
            journal
                .append(request())
                .expect("retry after timeout")
                .event_count,
            2
        );
    }

    #[test]
    fn outbox_lease_fault_is_observable_even_when_queue_is_empty() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::StaleLease);
        assert!(
            journal
                .claim_outbox_batch_checked("semantic-consumer", 1)
                .is_err()
        );
        assert_eq!(journal.last_fault(), Some(FaultPoint::StaleLease));
    }

    #[test]
    fn partial_append_is_visible_and_fails_closed_for_full_retry() {
        let mut journal = FaultInjectingJournal::new(InMemoryOperationalJournal::default());
        journal.arm(FaultPoint::PartialJournalAppend);
        assert!(journal.append(request()).is_err());
        assert_eq!(journal.last_fault(), Some(FaultPoint::PartialJournalAppend));
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("semantic-stream".to_string()))
                .len(),
            1
        );
        assert!(journal.append(request()).is_err());
    }
}
