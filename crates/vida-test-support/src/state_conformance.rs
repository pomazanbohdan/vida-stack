use taskflow_contracts::{
    VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
    VidaEventCursor, VidaEventRef, VidaIdempotencyKey, VidaOperation, VidaProjectionCheckpoint,
    VidaProjectionRef, VidaSchemaId, VidaSchemaVersion, VidaStreamRef, VidaStreamVersion,
    VidaTimestamp,
};
use taskflow_state::{
    InMemoryOperationalJournal, JournalAppendRequest, JournalIdempotencyState, JournalOutboxState,
    OperationalJournal, TaskflowStateError, verify_run_workflow_repository_conformance,
    verify_run_workflow_repository_corrupt_payload_fails_closed,
};

pub trait StateAdapterFactory {
    fn backend_name(&self) -> &str;
    fn fresh(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError>;
    fn reopen(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError>;
    fn supports_restart_recovery(&self) -> bool;
    fn supports_checkpoint_recovery(&self) -> bool {
        false
    }
    fn reopened_checkpoint(
        &mut self,
        _projection_id: &VidaProjectionRef,
    ) -> Result<Option<VidaProjectionCheckpoint>, TaskflowStateError> {
        Ok(None)
    }
    fn inject_partial_write_once(&mut self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateAdapterConformanceReport {
    pub backend: String,
    pub checks: Vec<String>,
    pub replay_hash: String,
    pub restart_recovered: bool,
    pub checkpoint_recovered: bool,
    pub partial_write_recovered: bool,
}

pub fn run_state_adapter_conformance<F: StateAdapterFactory>(
    factory: &mut F,
) -> Result<StateAdapterConformanceReport, TaskflowStateError> {
    let mut journal = factory.fresh()?;
    let request = append_request();
    let receipt = journal.append(request.clone())?;
    let stream_id = request.stream_id.clone();
    let stream = journal.load_stream(&stream_id);
    if stream
        .iter()
        .map(|event| event.event_id.0.as_str())
        .collect::<Vec<_>>()
        != ["event-1", "event-2", "event-3"]
    {
        return Err(storage("append/read ordering drifted"));
    }

    let global = journal.read_global_after(None, 10);
    if global.len() != 3
        || global[0].global_cursor != VidaEventCursor("global-1".to_string())
        || global[2].global_cursor != VidaEventCursor("global-3".to_string())
    {
        return Err(storage("global replay ordering drifted"));
    }

    let duplicate = journal.append(request.clone())?;
    if duplicate != receipt {
        return Err(storage(
            "same idempotency request did not return its original receipt",
        ));
    }
    if !matches!(
        journal
            .idempotency_record(&request.idempotency_key)
            .map(|record| &record.state),
        Some(JournalIdempotencyState::Completed)
    ) {
        return Err(storage("append idempotency record is not completed"));
    }
    let mut conflicting_request = request.clone();
    conflicting_request.correlation_id = Some("different-correlation".to_string());
    if !matches!(
        journal.append(conflicting_request),
        Err(TaskflowStateError::IdempotencyConflict(_))
    ) {
        return Err(storage("idempotency conflict did not fail closed"));
    }
    if !matches!(
        journal
            .idempotency_record(&request.idempotency_key)
            .map(|record| &record.state),
        Some(JournalIdempotencyState::Conflicted)
    ) {
        return Err(storage("append idempotency conflict was not recorded"));
    }
    if !matches!(
        journal.append(request.clone()),
        Err(TaskflowStateError::IdempotencyConflict(_))
    ) {
        return Err(storage("conflicted idempotency key was allowed to retry"));
    }

    let expected_checkpoint = checkpoint(2);
    journal.record_projection_checkpoint(expected_checkpoint.clone());
    journal.record_projection_checkpoint(checkpoint(1));
    let recovered = journal.read_global_after(Some(&VidaEventCursor("global-2".to_string())), 10);
    if recovered
        .iter()
        .map(|record| record.event.event_id.0.as_str())
        .collect::<Vec<_>>()
        != ["event-3"]
    {
        return Err(storage(
            "checkpoint recovery did not resume after the checkpoint cursor",
        ));
    }

    let first_claim = journal.claim_outbox_batch("conformance-consumer", 1);
    if first_claim.len() != 1 || !matches!(first_claim[0].state, JournalOutboxState::Claimed { .. })
    {
        return Err(storage("outbox claim semantics drifted"));
    }
    journal.mark_outbox_succeeded(&first_claim[0].outbox_id)?;
    let second_claim = journal.claim_outbox_batch("conformance-consumer", 10);
    if second_claim.len() != 1 {
        return Err(storage("outbox did not retain the second pending effect"));
    }
    journal.mark_outbox_failed(
        &second_claim[0].outbox_id,
        "test outbox effect failure".to_string(),
    )?;

    let mut replay_journal = factory.fresh()?;
    let first_replay =
        verify_run_workflow_repository_conformance(&mut *replay_journal, "conformance-replay")?;
    let mut replay_again_journal = factory.fresh()?;
    let second_replay = verify_run_workflow_repository_conformance(
        &mut *replay_again_journal,
        "conformance-replay",
    )?;
    if first_replay.final_snapshot_hash != second_replay.final_snapshot_hash {
        return Err(storage("replay hash is not deterministic"));
    }

    let mut malformed_journal = factory.fresh()?;
    verify_run_workflow_repository_corrupt_payload_fails_closed(
        &mut *malformed_journal,
        "conformance-malformed",
    )?;

    let partial_write_injected = factory.inject_partial_write_once();
    let mut partial_write_recovered = false;
    let fresh_request = fresh_append_request();
    if partial_write_injected {
        let interrupted = journal.append(fresh_request.clone());
        if !matches!(
            interrupted,
            Err(TaskflowStateError::Storage(reason)) if reason.contains("injected partial write interruption")
        ) {
            return Err(storage(
                "injected partial write did not fail at the persistence boundary",
            ));
        }
    }

    let mut restart_recovered = false;
    let mut checkpoint_recovered = false;
    if factory.supports_restart_recovery() {
        drop(journal);
        let mut reopened = factory.reopen()?;
        if reopened.load_stream(&stream_id).len() != 3
            || reopened.read_global_after(None, 10).len() != 3
        {
            return Err(storage(
                "restart did not recover the committed event stream",
            ));
        }
        if !matches!(
            reopened
                .idempotency_record(&request.idempotency_key)
                .map(|record| &record.state),
            Some(JournalIdempotencyState::Conflicted)
        ) {
            return Err(storage(
                "restart did not recover the durable idempotency conflict",
            ));
        }
        if factory.supports_checkpoint_recovery()
            && factory.reopened_checkpoint(&expected_checkpoint.projection_id)?
                != Some(expected_checkpoint.clone())
        {
            return Err(storage(
                "restart did not recover the durable projection checkpoint",
            ));
        }
        checkpoint_recovered = factory.supports_checkpoint_recovery();
        if !matches!(
            reopened.append(request),
            Err(TaskflowStateError::IdempotencyConflict(_))
        ) {
            return Err(storage(
                "reopened conflicted idempotency key was allowed to retry",
            ));
        }
        if partial_write_injected {
            if !reopened.load_stream(&fresh_request.stream_id).is_empty()
                || reopened.read_global_after(None, 10).len() != 3
            {
                return Err(storage(
                    "fresh append interruption left a phantom event or lost committed events",
                ));
            }
            let recovered_receipt = reopened.append(fresh_request)?;
            if recovered_receipt.event_count != 1
                || reopened.load_stream(&recovered_receipt.stream_id).len() != 1
                || reopened.read_global_after(None, 10).len() != 4
            {
                return Err(storage(
                    "fresh append retry did not recover exactly one event without loss",
                ));
            }
        }
        restart_recovered = true;
        partial_write_recovered = partial_write_injected;
    }

    let mut checks = vec![
        "append_read_ordering".to_string(),
        "replay_determinism".to_string(),
        "idempotency".to_string(),
        "checkpoint_recovery".to_string(),
        "outbox_effects".to_string(),
        "malformed_payloads".to_string(),
        "restart_recovery".to_string(),
    ];
    if partial_write_recovered {
        checks.push("partial_write_recovery".to_string());
        checks.push("fresh_append_recovery".to_string());
    }

    Ok(StateAdapterConformanceReport {
        backend: factory.backend_name().to_string(),
        checks,
        replay_hash: first_replay.final_snapshot_hash,
        restart_recovered,
        checkpoint_recovered,
        partial_write_recovered,
    })
}

pub struct InMemoryStateAdapterFactory;

impl StateAdapterFactory for InMemoryStateAdapterFactory {
    fn backend_name(&self) -> &str {
        "in-memory"
    }

    fn fresh(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError> {
        Ok(Box::new(InMemoryOperationalJournal::default()))
    }

    fn reopen(&mut self) -> Result<Box<dyn OperationalJournal>, TaskflowStateError> {
        self.fresh()
    }

    fn supports_restart_recovery(&self) -> bool {
        false
    }
}

fn append_request() -> JournalAppendRequest {
    JournalAppendRequest {
        stream_id: VidaStreamRef("conformance-stream".to_string()),
        expected_stream_version: Some(VidaStreamVersion(0)),
        command_id: VidaCommandRef("conformance-command".to_string()),
        idempotency_key: VidaIdempotencyKey("conformance-idempotency".to_string()),
        causation_id: None,
        correlation_id: Some("conformance-correlation".to_string()),
        events: (1..=3).map(event).collect(),
        effect_intents: vec![effect("effect-1"), effect("effect-2")],
    }
}

fn fresh_append_request() -> JournalAppendRequest {
    JournalAppendRequest {
        stream_id: VidaStreamRef("conformance-fresh-append-stream".to_string()),
        expected_stream_version: Some(VidaStreamVersion(0)),
        command_id: VidaCommandRef("conformance-fresh-append-command".to_string()),
        idempotency_key: VidaIdempotencyKey("conformance-fresh-append-idempotency".to_string()),
        causation_id: None,
        correlation_id: Some("conformance-fresh-append-correlation".to_string()),
        events: vec![VidaDomainEventEnvelope {
            schema_id: VidaSchemaId("taskflow.state.conformance".to_string()),
            event_version: VidaSchemaVersion(1),
            event_id: VidaEventRef("fresh-append-event-1".to_string()),
            command_id: Some(VidaCommandRef(
                "conformance-fresh-append-command".to_string(),
            )),
            causation_id: None,
            stream_id: VidaStreamRef("conformance-fresh-append-stream".to_string()),
            stream_version: VidaStreamVersion(1),
            aggregate_id: VidaAggregateRef("conformance-fresh-append-aggregate".to_string()),
            occurred_at: VidaTimestamp("fresh-append-version-1".to_string()),
            payload: serde_json::json!({ "version": 1 }),
            trace: serde_json::json!({ "suite": "state-adapter-conformance" }),
        }],
        effect_intents: Vec::new(),
    }
}

fn event(version: u64) -> VidaDomainEventEnvelope {
    VidaDomainEventEnvelope {
        schema_id: VidaSchemaId("taskflow.state.conformance".to_string()),
        event_version: VidaSchemaVersion(1),
        event_id: VidaEventRef(format!("event-{version}")),
        command_id: Some(VidaCommandRef("conformance-command".to_string())),
        causation_id: None,
        stream_id: VidaStreamRef("conformance-stream".to_string()),
        stream_version: VidaStreamVersion(version),
        aggregate_id: VidaAggregateRef("conformance-aggregate".to_string()),
        occurred_at: VidaTimestamp(format!("version-{version}")),
        payload: serde_json::json!({ "version": version }),
        trace: serde_json::json!({ "suite": "state-adapter-conformance" }),
    }
}

fn effect(id: &str) -> VidaEffectIntent {
    VidaEffectIntent {
        effect_id: VidaEffectRef(id.to_string()),
        operation: VidaOperation("taskflow.state.conformance.effect".to_string()),
        command_id: VidaCommandRef("conformance-command".to_string()),
        stream_id: VidaStreamRef("conformance-stream".to_string()),
        payload: serde_json::json!({ "effect_id": id }),
    }
}

fn checkpoint(version: u64) -> VidaProjectionCheckpoint {
    VidaProjectionCheckpoint {
        projection_id: VidaProjectionRef("conformance-projection".to_string()),
        stream_id: VidaStreamRef("conformance-stream".to_string()),
        event_cursor: VidaEventCursor(format!("global-{version}")),
        stream_version: VidaStreamVersion(version),
        updated_at: VidaTimestamp(format!("version-{version}")),
    }
}

fn storage(message: &str) -> TaskflowStateError {
    TaskflowStateError::Storage(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::{InMemoryStateAdapterFactory, run_state_adapter_conformance};

    #[test]
    fn in_memory_adapter_passes_shared_state_corpus() {
        let mut factory = InMemoryStateAdapterFactory;
        let report = run_state_adapter_conformance(&mut factory)
            .expect("in-memory adapter should pass the shared corpus");

        assert_eq!(report.backend, "in-memory");
        assert_eq!(report.checks.len(), 7);
        assert!(!report.replay_hash.is_empty());
        assert!(!report.restart_recovered);
        assert!(!report.checkpoint_recovered);
        assert!(!report.partial_write_recovered);
    }
}
