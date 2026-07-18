use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    DependencyEdge, TaskRecord, VidaAggregateRef, VidaArtifactRef, VidaCommandRef,
    VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef, VidaEventCursor, VidaEventRef,
    VidaIdempotencyKey, VidaOperation, VidaProjectionCheckpoint, VidaProjectionRef, VidaReceiptId,
    VidaSchemaId, VidaSchemaVersion, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
};
use taskflow_core::{
    TaskId,
    role_step::TaskRoleStep,
    run_workflow::{
        RunWorkflowAggregate, RunWorkflowCommand, RunWorkflowEffectIntent, RunWorkflowEvent,
        RunWorkflowState,
    },
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskflowStateError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("stream version conflict for {stream_id}: expected {expected:?}, actual {actual}")]
    StreamVersionConflict {
        stream_id: String,
        expected: Option<u64>,
        actual: u64,
    },
    #[error("idempotency conflict: {0}")]
    IdempotencyConflict(String),
    #[error(
        "event {event_id} belongs to stream {event_stream_id}, not append stream {request_stream_id}"
    )]
    EventStreamMismatch {
        request_stream_id: String,
        event_stream_id: String,
        event_id: String,
    },
    #[error("outbox record not found: {0}")]
    OutboxRecordNotFound(String),
    #[error("state storage error: {0}")]
    Storage(String),
    #[error("journal payload decode error: {0}")]
    PayloadDecode(String),
    #[error("run workflow journal replay validation error: {0}")]
    JournalReplayValidation(String),
}

pub trait TaskStore {
    fn upsert_task(&mut self, task: TaskRecord);
    fn get_task(&self, id: &TaskId) -> Result<&TaskRecord, TaskflowStateError>;
    fn list_tasks(&self) -> Vec<&TaskRecord>;
    fn add_dependency(&mut self, edge: DependencyEdge);
    fn list_dependencies(&self, id: &TaskId) -> Vec<&DependencyEdge>;
}

#[derive(Debug, Default)]
pub struct InMemoryTaskStore {
    tasks: HashMap<String, TaskRecord>,
    dependencies: Vec<DependencyEdge>,
}

impl TaskStore for InMemoryTaskStore {
    fn upsert_task(&mut self, task: TaskRecord) {
        self.tasks.insert(task.id.as_str().to_string(), task);
    }

    fn get_task(&self, id: &TaskId) -> Result<&TaskRecord, TaskflowStateError> {
        self.tasks
            .get(id.as_str())
            .ok_or_else(|| TaskflowStateError::TaskNotFound(id.as_str().to_string()))
    }

    fn list_tasks(&self) -> Vec<&TaskRecord> {
        let mut rows: Vec<_> = self.tasks.values().collect();
        rows.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        rows
    }

    fn add_dependency(&mut self, edge: DependencyEdge) {
        self.dependencies.push(edge);
    }

    fn list_dependencies(&self, id: &TaskId) -> Vec<&DependencyEdge> {
        self.dependencies
            .iter()
            .filter(|edge| edge.issue_id.as_str() == id.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalAppendRequest {
    pub stream_id: VidaStreamRef,
    pub expected_stream_version: Option<VidaStreamVersion>,
    pub command_id: VidaCommandRef,
    pub idempotency_key: VidaIdempotencyKey,
    pub causation_id: Option<VidaCommandRef>,
    pub correlation_id: Option<String>,
    pub events: Vec<VidaDomainEventEnvelope>,
    pub effect_intents: Vec<VidaEffectIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalAppendReceipt {
    pub stream_id: VidaStreamRef,
    pub first_global_cursor: Option<VidaEventCursor>,
    pub last_global_cursor: Option<VidaEventCursor>,
    pub stream_version: VidaStreamVersion,
    pub event_count: usize,
    pub effect_intent_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEventRecord {
    pub global_cursor: VidaEventCursor,
    pub event: VidaDomainEventEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalIdempotencyState {
    Started,
    Completed,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalIdempotencyRecord {
    pub key: VidaIdempotencyKey,
    pub command_id: VidaCommandRef,
    pub state: JournalIdempotencyState,
    pub receipt_id: Option<VidaReceiptId>,
    pub conflict_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalOutboxState {
    Pending,
    Claimed { consumer_id: String },
    Succeeded,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalOutboxRecord {
    pub outbox_id: VidaEventRef,
    pub effect: VidaEffectIntent,
    pub state: JournalOutboxState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalProjectionFailure {
    pub projection_id: VidaProjectionRef,
    pub stream_id: VidaStreamRef,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalArtifactRecord {
    pub artifact_ref: VidaArtifactRef,
    pub content_hash: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalAggregateSnapshotRecord {
    pub aggregate_id: VidaAggregateRef,
    pub schema_id: VidaSchemaId,
    pub schema_version: VidaSchemaVersion,
    pub stream_id: VidaStreamRef,
    pub stream_version: VidaStreamVersion,
    pub payload: serde_json::Value,
    pub replay_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JournalAppendIdempotencyRecord {
    request_fingerprint: String,
    receipt: JournalAppendReceipt,
    #[serde(default)]
    conflicted: bool,
}

pub fn append_request_fingerprint(request: &JournalAppendRequest) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        request.stream_id,
        request.command_id,
        request.idempotency_key,
        request.causation_id,
        request.correlation_id,
        request.events,
        request.effect_intents
    )
}

pub fn validate_append_event_streams(
    request: &JournalAppendRequest,
) -> Result<(), TaskflowStateError> {
    for event in &request.events {
        if event.stream_id != request.stream_id {
            return Err(TaskflowStateError::EventStreamMismatch {
                request_stream_id: request.stream_id.0.clone(),
                event_stream_id: event.stream_id.0.clone(),
                event_id: event.event_id.0.clone(),
            });
        }
    }
    Ok(())
}

pub trait OperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError>;
    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope>;
    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord>;
    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError>;
    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError>;
    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError>;
    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord>;
    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord>;
    fn mark_outbox_succeeded(&mut self, outbox_id: &VidaEventRef)
    -> Result<(), TaskflowStateError>;
    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError>;
    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint);
    fn record_projection_failure(&mut self, failure: JournalProjectionFailure);
    fn index_artifact(&mut self, artifact: JournalArtifactRecord);
    fn record_aggregate_snapshot(&mut self, snapshot: JournalAggregateSnapshotRecord);
    fn aggregate_snapshot(
        &self,
        aggregate_id: &VidaAggregateRef,
    ) -> Option<JournalAggregateSnapshotRecord>;
}

pub struct RunWorkflowJournalRepository<'a, J: OperationalJournal + ?Sized> {
    journal: &'a mut J,
}

impl<'a, J: OperationalJournal + ?Sized> RunWorkflowJournalRepository<'a, J> {
    pub fn new(journal: &'a mut J) -> Self {
        Self { journal }
    }

    pub fn load(
        &self,
        run_id: &str,
        task_id: &str,
    ) -> Result<RunWorkflowAggregate, TaskflowStateError> {
        let stream_id = run_workflow_stream_id(run_id);
        let mut aggregate = RunWorkflowAggregate::new(run_id, task_id);
        for envelope in self.journal.load_stream(&stream_id) {
            validate_run_workflow_envelope_metadata(
                &envelope,
                &stream_id,
                run_id,
                aggregate.version + 1,
            )?;
            let event: RunWorkflowEvent = serde_json::from_value(envelope.payload)
                .map_err(|error| TaskflowStateError::PayloadDecode(error.to_string()))?;
            let replayed = replay_run_workflow_event(&aggregate, &event)?;
            aggregate = RunWorkflowAggregate::from_snapshot(
                run_id,
                task_id,
                replayed.state_after,
                envelope.stream_version.0,
            );
        }
        Ok(aggregate)
    }

    pub fn append(
        &mut self,
        aggregate_before: &RunWorkflowAggregate,
        event: RunWorkflowEvent,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        let event = replay_run_workflow_event(aggregate_before, &event)?;
        let next_version = aggregate_before.version + 1;
        let stream_id = run_workflow_stream_id(&aggregate_before.run_id);
        let command_id = VidaCommandRef(format!(
            "run-workflow:{}:{}",
            aggregate_before.run_id, next_version
        ));
        self.journal.append(JournalAppendRequest {
            stream_id: stream_id.clone(),
            expected_stream_version: Some(VidaStreamVersion(aggregate_before.version)),
            command_id: command_id.clone(),
            idempotency_key: VidaIdempotencyKey(format!("{}:{}", command_id.0, next_version)),
            causation_id: None,
            correlation_id: Some(aggregate_before.task_id.clone()),
            events: vec![VidaDomainEventEnvelope {
                schema_id: VidaSchemaId("taskflow.run_workflow.event".to_string()),
                event_version: VidaSchemaVersion(1),
                event_id: VidaEventRef(format!("{}:event", command_id.0)),
                command_id: Some(command_id.clone()),
                causation_id: None,
                stream_id,
                stream_version: VidaStreamVersion(next_version),
                aggregate_id: VidaAggregateRef(aggregate_before.run_id.clone()),
                occurred_at: VidaTimestamp(format!("version-{next_version}")),
                payload: serde_json::to_value(event.clone())
                    .expect("run workflow event should serialize"),
                trace: serde_json::json!({
                    "task_id": aggregate_before.task_id,
                    "snapshot_hash": aggregate_before.snapshot_replay_hash()
                }),
            }],
            effect_intents: event_effects_to_journal_effects(
                &command_id,
                &aggregate_before.run_id,
                next_version,
                &event.effect_intents,
            ),
        })
    }

    pub fn save_snapshot(&mut self, aggregate: &RunWorkflowAggregate) {
        self.journal
            .record_aggregate_snapshot(run_workflow_snapshot_record(aggregate));
    }

    pub fn load_snapshot(
        &self,
        run_id: &str,
        task_id: &str,
    ) -> Result<Option<RunWorkflowAggregate>, TaskflowStateError> {
        let aggregate_id = VidaAggregateRef(run_id.to_string());
        let Some(snapshot) = self.journal.aggregate_snapshot(&aggregate_id) else {
            return Ok(None);
        };
        validate_run_workflow_snapshot_record(&snapshot, run_id, task_id).map(Some)
    }
}

fn run_workflow_snapshot_record(
    aggregate: &RunWorkflowAggregate,
) -> JournalAggregateSnapshotRecord {
    JournalAggregateSnapshotRecord {
        aggregate_id: VidaAggregateRef(aggregate.run_id.clone()),
        schema_id: VidaSchemaId("taskflow.run_workflow.snapshot".to_string()),
        schema_version: VidaSchemaVersion(1),
        stream_id: run_workflow_stream_id(&aggregate.run_id),
        stream_version: VidaStreamVersion(aggregate.version),
        payload: serde_json::json!({
            "run_id": aggregate.run_id.clone(),
            "task_id": aggregate.task_id.clone(),
            "state": aggregate.state.clone(),
            "version": aggregate.version,
        }),
        replay_hash: aggregate.snapshot_replay_hash(),
    }
}

fn validate_run_workflow_snapshot_record(
    snapshot: &JournalAggregateSnapshotRecord,
    run_id: &str,
    task_id: &str,
) -> Result<RunWorkflowAggregate, TaskflowStateError> {
    if snapshot.schema_id.0 != "taskflow.run_workflow.snapshot" || snapshot.schema_version.0 != 1 {
        return Err(TaskflowStateError::PayloadDecode(format!(
            "unsupported run workflow snapshot schema {}@{}",
            snapshot.schema_id.0, snapshot.schema_version.0
        )));
    }
    if snapshot.aggregate_id.0 != run_id || snapshot.stream_id != run_workflow_stream_id(run_id) {
        return Err(TaskflowStateError::PayloadDecode(
            "run workflow snapshot aggregate identity mismatch".to_string(),
        ));
    }
    #[derive(Deserialize)]
    struct SnapshotPayload {
        run_id: String,
        task_id: String,
        state: RunWorkflowState,
        version: u64,
    }
    let payload: SnapshotPayload = serde_json::from_value(snapshot.payload.clone())
        .map_err(|error| TaskflowStateError::PayloadDecode(error.to_string()))?;
    if payload.run_id != run_id || payload.task_id != task_id {
        return Err(TaskflowStateError::PayloadDecode(
            "run workflow snapshot payload identity mismatch".to_string(),
        ));
    }
    if payload.version != snapshot.stream_version.0 {
        return Err(TaskflowStateError::PayloadDecode(
            "run workflow snapshot version mismatch".to_string(),
        ));
    }
    let aggregate = RunWorkflowAggregate::from_snapshot(
        payload.run_id,
        payload.task_id,
        payload.state,
        payload.version,
    );
    if aggregate.snapshot_replay_hash() != snapshot.replay_hash {
        return Err(TaskflowStateError::PayloadDecode(
            "run workflow snapshot replay hash mismatch".to_string(),
        ));
    }
    Ok(aggregate)
}

fn validate_run_workflow_envelope_metadata(
    envelope: &VidaDomainEventEnvelope,
    stream_id: &VidaStreamRef,
    run_id: &str,
    expected_stream_version: u64,
) -> Result<(), TaskflowStateError> {
    if envelope.schema_id != VidaSchemaId("taskflow.run_workflow.event".to_string()) {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "unexpected schema_id {} for run workflow event",
            envelope.schema_id.0
        )));
    }
    if envelope.event_version != VidaSchemaVersion(1) {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "unexpected event_version {} for run workflow event",
            envelope.event_version.0
        )));
    }
    if envelope.stream_id != *stream_id {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "event stream {} does not match expected stream {}",
            envelope.stream_id.0, stream_id.0
        )));
    }
    if envelope.aggregate_id != VidaAggregateRef(run_id.to_string()) {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "event aggregate {} does not match run {}",
            envelope.aggregate_id.0, run_id
        )));
    }
    if envelope.stream_version != VidaStreamVersion(expected_stream_version) {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "event stream_version {} does not match expected {}",
            envelope.stream_version.0, expected_stream_version
        )));
    }
    Ok(())
}

fn replay_run_workflow_event(
    aggregate_before: &RunWorkflowAggregate,
    persisted: &RunWorkflowEvent,
) -> Result<RunWorkflowEvent, TaskflowStateError> {
    if persisted.state_before != aggregate_before.state {
        return Err(TaskflowStateError::JournalReplayValidation(format!(
            "event state_before {:?} does not match aggregate state {:?}",
            persisted.state_before, aggregate_before.state
        )));
    }

    let mut replay = aggregate_before.clone();
    let expected = replay.handle(persisted.command.clone());
    if &expected != persisted {
        return Err(TaskflowStateError::JournalReplayValidation(
            "persisted event does not match run workflow state machine replay".to_string(),
        ));
    }
    Ok(expected)
}

fn run_workflow_stream_id(run_id: &str) -> VidaStreamRef {
    VidaStreamRef(format!("run-workflow:{run_id}"))
}

fn event_effects_to_journal_effects(
    command_id: &VidaCommandRef,
    run_id: &str,
    version: u64,
    intents: &[RunWorkflowEffectIntent],
) -> Vec<VidaEffectIntent> {
    intents
        .iter()
        .enumerate()
        .map(|(index, intent)| VidaEffectIntent {
            effect_id: VidaEffectRef(format!("{}:effect:{index}", command_id.0)),
            operation: VidaOperation(format!("taskflow.run_workflow.effect.{intent:?}")),
            command_id: command_id.clone(),
            stream_id: run_workflow_stream_id(run_id),
            payload: serde_json::json!({
                "intent": format!("{intent:?}"),
                "stream_version": version
            }),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunWorkflowRepositoryConformanceReport {
    pub run_id: String,
    pub final_snapshot_hash: String,
    pub event_count: usize,
}

pub fn verify_run_workflow_repository_conformance<J: OperationalJournal + ?Sized>(
    journal: &mut J,
    run_id: &str,
) -> Result<RunWorkflowRepositoryConformanceReport, TaskflowStateError> {
    let task_id = "ldr-031";
    let mut aggregate = RunWorkflowAggregate::new(run_id, task_id);
    let initial_hash = aggregate.snapshot_replay_hash();
    for command in [
        RunWorkflowCommand::Start {
            first_step: TaskRoleStep::planning(),
        },
        RunWorkflowCommand::Dispatch {
            target: TaskRoleStep::developer(),
        },
    ] {
        let before = aggregate.clone();
        let event = aggregate.handle(command);
        RunWorkflowJournalRepository::new(journal).append(&before, event)?;
    }

    let loaded = RunWorkflowJournalRepository::new(journal).load(run_id, task_id)?;
    if loaded.state
        != (RunWorkflowState::Active {
            step: TaskRoleStep::developer(),
        })
    {
        return Err(TaskflowStateError::Storage(
            "run workflow repository replay state mismatch".to_string(),
        ));
    }
    if loaded.version != aggregate.version {
        return Err(TaskflowStateError::Storage(
            "run workflow repository replay version mismatch".to_string(),
        ));
    }
    if loaded.snapshot_replay_hash() != aggregate.snapshot_replay_hash() {
        return Err(TaskflowStateError::Storage(
            "run workflow repository replay hash mismatch".to_string(),
        ));
    }
    if initial_hash == loaded.snapshot_replay_hash() {
        return Err(TaskflowStateError::Storage(
            "run workflow repository replay hash did not change after events".to_string(),
        ));
    }

    Ok(RunWorkflowRepositoryConformanceReport {
        run_id: run_id.to_string(),
        final_snapshot_hash: loaded.snapshot_replay_hash(),
        event_count: loaded.version as usize,
    })
}

pub fn verify_run_workflow_repository_corrupt_payload_fails_closed<
    J: OperationalJournal + ?Sized,
>(
    journal: &mut J,
    run_id: &str,
) -> Result<(), TaskflowStateError> {
    let command_id = VidaCommandRef(format!("run-workflow:{run_id}:1"));
    journal.append(JournalAppendRequest {
        stream_id: run_workflow_stream_id(run_id),
        expected_stream_version: Some(VidaStreamVersion(0)),
        command_id: command_id.clone(),
        idempotency_key: VidaIdempotencyKey(format!("run-workflow:{run_id}:1")),
        causation_id: None,
        correlation_id: Some("ldr-031".to_string()),
        events: vec![VidaDomainEventEnvelope {
            schema_id: VidaSchemaId("taskflow.run_workflow.event".to_string()),
            event_version: VidaSchemaVersion(1),
            event_id: VidaEventRef(format!("run-workflow:{run_id}:1:event")),
            command_id: Some(command_id),
            causation_id: None,
            stream_id: run_workflow_stream_id(run_id),
            stream_version: VidaStreamVersion(1),
            aggregate_id: VidaAggregateRef(run_id.to_string()),
            occurred_at: VidaTimestamp("version-1".to_string()),
            payload: serde_json::json!({"unexpected": "shape"}),
            trace: serde_json::json!({}),
        }],
        effect_intents: Vec::new(),
    })?;

    match RunWorkflowJournalRepository::new(journal).load(run_id, "ldr-031") {
        Err(TaskflowStateError::PayloadDecode(_)) => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(TaskflowStateError::Storage(
            "corrupt run workflow repository payload loaded successfully".to_string(),
        )),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InMemoryOperationalJournal {
    streams: HashMap<String, Vec<VidaDomainEventEnvelope>>,
    global_events: Vec<JournalEventRecord>,
    idempotency: HashMap<String, JournalIdempotencyRecord>,
    append_idempotency: HashMap<String, JournalAppendIdempotencyRecord>,
    outbox: Vec<JournalOutboxRecord>,
    projection_checkpoints: HashMap<String, VidaProjectionCheckpoint>,
    projection_failures: Vec<JournalProjectionFailure>,
    artifacts: HashMap<String, JournalArtifactRecord>,
    aggregate_snapshots: HashMap<String, JournalAggregateSnapshotRecord>,
}

impl InMemoryOperationalJournal {
    #[must_use]
    pub fn projection_checkpoint(
        &self,
        projection_id: &VidaProjectionRef,
    ) -> Option<&VidaProjectionCheckpoint> {
        self.projection_checkpoints.get(&projection_id.0)
    }

    #[must_use]
    pub fn projection_failures(&self) -> &[JournalProjectionFailure] {
        &self.projection_failures
    }

    #[must_use]
    pub fn artifact(&self, artifact_ref: &VidaArtifactRef) -> Option<&JournalArtifactRecord> {
        self.artifacts.get(&artifact_ref.0)
    }
}

impl OperationalJournal for InMemoryOperationalJournal {
    fn append(
        &mut self,
        request: JournalAppendRequest,
    ) -> Result<JournalAppendReceipt, TaskflowStateError> {
        validate_append_event_streams(&request)?;

        let idempotency_key = request.idempotency_key.clone();
        let command_id = request.command_id.clone();
        let request_fingerprint = append_request_fingerprint(&request);
        if let Some(record) = self.append_idempotency.get_mut(&idempotency_key.0) {
            if record.request_fingerprint == request_fingerprint {
                if record.conflicted {
                    return Err(TaskflowStateError::IdempotencyConflict(idempotency_key.0));
                }
                return Ok(record.receipt.clone());
            }
            record.conflicted = true;
            self.idempotency.insert(
                idempotency_key.0.clone(),
                JournalIdempotencyRecord {
                    key: idempotency_key.clone(),
                    command_id,
                    state: JournalIdempotencyState::Conflicted,
                    receipt_id: Some(VidaReceiptId(format!("append:{}", idempotency_key.0))),
                    conflict_reason: Some(
                        "idempotency_payload_conflict: same idempotency key used with different append payload"
                            .to_string(),
                    ),
                },
            );
            return Err(TaskflowStateError::IdempotencyConflict(idempotency_key.0));
        }

        let key = request.stream_id.0.clone();
        let stream = self.streams.entry(key.clone()).or_default();
        let actual_version = stream.len() as u64;
        if request
            .expected_stream_version
            .as_ref()
            .map(|version| version.0)
            != Some(actual_version)
        {
            return Err(TaskflowStateError::StreamVersionConflict {
                stream_id: key,
                expected: request
                    .expected_stream_version
                    .as_ref()
                    .map(|version| version.0),
                actual: actual_version,
            });
        }

        let first_index = self.global_events.len();
        let event_count = request.events.len();
        let effect_intent_count = request.effect_intents.len();
        for event in request.events {
            let cursor = VidaEventCursor(format!("global-{}", self.global_events.len() + 1));
            stream.push(event.clone());
            self.global_events.push(JournalEventRecord {
                global_cursor: cursor,
                event,
            });
        }
        for effect in request.effect_intents {
            self.outbox.push(JournalOutboxRecord {
                outbox_id: VidaEventRef(format!("outbox-{}", self.outbox.len() + 1)),
                effect,
                state: JournalOutboxState::Pending,
            });
        }

        let last_index = self.global_events.len().saturating_sub(1);
        let receipt = JournalAppendReceipt {
            stream_id: request.stream_id,
            first_global_cursor: self
                .global_events
                .get(first_index)
                .map(|record| record.global_cursor.clone()),
            last_global_cursor: self
                .global_events
                .get(last_index)
                .map(|record| record.global_cursor.clone()),
            stream_version: VidaStreamVersion(stream.len() as u64),
            event_count,
            effect_intent_count,
        };
        self.idempotency.insert(
            idempotency_key.0.clone(),
            JournalIdempotencyRecord {
                key: idempotency_key.clone(),
                command_id: command_id.clone(),
                state: JournalIdempotencyState::Completed,
                receipt_id: Some(VidaReceiptId(format!("append:{}", command_id.0))),
                conflict_reason: None,
            },
        );
        self.append_idempotency.insert(
            idempotency_key.0,
            JournalAppendIdempotencyRecord {
                request_fingerprint,
                receipt: receipt.clone(),
                conflicted: false,
            },
        );
        Ok(receipt)
    }

    fn load_stream(&self, stream_id: &VidaStreamRef) -> Vec<VidaDomainEventEnvelope> {
        self.streams.get(&stream_id.0).cloned().unwrap_or_default()
    }

    fn read_global_after(
        &self,
        cursor: Option<&VidaEventCursor>,
        limit: usize,
    ) -> Vec<JournalEventRecord> {
        let start = cursor
            .and_then(|cursor| {
                self.global_events
                    .iter()
                    .position(|record| record.global_cursor == *cursor)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        self.global_events
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect()
    }

    fn record_idempotency_started(
        &mut self,
        key: VidaIdempotencyKey,
        command_id: VidaCommandRef,
    ) -> Result<(), TaskflowStateError> {
        if self.idempotency.contains_key(&key.0) {
            return Err(TaskflowStateError::IdempotencyConflict(key.0));
        }
        self.idempotency.insert(
            key.0.clone(),
            JournalIdempotencyRecord {
                key,
                command_id,
                state: JournalIdempotencyState::Started,
                receipt_id: None,
                conflict_reason: None,
            },
        );
        Ok(())
    }

    fn record_idempotency_completed(
        &mut self,
        key: &VidaIdempotencyKey,
        receipt_id: VidaReceiptId,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .idempotency
            .get_mut(&key.0)
            .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
        record.state = JournalIdempotencyState::Completed;
        record.receipt_id = Some(receipt_id);
        Ok(())
    }

    fn record_idempotency_conflicted(
        &mut self,
        key: &VidaIdempotencyKey,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .idempotency
            .get_mut(&key.0)
            .ok_or_else(|| TaskflowStateError::IdempotencyConflict(key.0.clone()))?;
        record.state = JournalIdempotencyState::Conflicted;
        record.conflict_reason = Some(reason);
        Ok(())
    }

    fn idempotency_record(&self, key: &VidaIdempotencyKey) -> Option<&JournalIdempotencyRecord> {
        self.idempotency.get(&key.0)
    }

    fn claim_outbox_batch(&mut self, consumer_id: &str, limit: usize) -> Vec<JournalOutboxRecord> {
        let mut claimed = Vec::new();
        for record in self.outbox.iter_mut() {
            if !matches!(record.state, JournalOutboxState::Pending) {
                continue;
            }
            record.state = JournalOutboxState::Claimed {
                consumer_id: consumer_id.to_string(),
            };
            claimed.push(record.clone());
            if claimed.len() == limit {
                break;
            }
        }
        claimed
    }

    fn mark_outbox_succeeded(
        &mut self,
        outbox_id: &VidaEventRef,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .outbox
            .iter_mut()
            .find(|record| record.outbox_id == *outbox_id)
            .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
        record.state = JournalOutboxState::Succeeded;
        Ok(())
    }

    fn mark_outbox_failed(
        &mut self,
        outbox_id: &VidaEventRef,
        reason: String,
    ) -> Result<(), TaskflowStateError> {
        let record = self
            .outbox
            .iter_mut()
            .find(|record| record.outbox_id == *outbox_id)
            .ok_or_else(|| TaskflowStateError::OutboxRecordNotFound(outbox_id.0.clone()))?;
        record.state = JournalOutboxState::Failed { reason };
        Ok(())
    }

    fn record_projection_checkpoint(&mut self, checkpoint: VidaProjectionCheckpoint) {
        if self
            .projection_checkpoints
            .get(&checkpoint.projection_id.0)
            .is_some_and(|existing| projection_checkpoint_is_stale(existing, &checkpoint))
        {
            return;
        }
        self.projection_checkpoints
            .insert(checkpoint.projection_id.0.clone(), checkpoint);
    }

    fn record_projection_failure(&mut self, failure: JournalProjectionFailure) {
        self.projection_failures.push(failure);
    }

    fn index_artifact(&mut self, artifact: JournalArtifactRecord) {
        self.artifacts
            .insert(artifact.artifact_ref.0.clone(), artifact);
    }

    fn record_aggregate_snapshot(&mut self, snapshot: JournalAggregateSnapshotRecord) {
        self.aggregate_snapshots
            .insert(snapshot.aggregate_id.0.clone(), snapshot);
    }

    fn aggregate_snapshot(
        &self,
        aggregate_id: &VidaAggregateRef,
    ) -> Option<JournalAggregateSnapshotRecord> {
        self.aggregate_snapshots.get(&aggregate_id.0).cloned()
    }
}

fn projection_checkpoint_is_stale(
    existing: &VidaProjectionCheckpoint,
    candidate: &VidaProjectionCheckpoint,
) -> bool {
    let existing_cursor = projection_checkpoint_cursor_number(&existing.event_cursor);
    let candidate_cursor = projection_checkpoint_cursor_number(&candidate.event_cursor);
    candidate_cursor < existing_cursor
        || (candidate_cursor == existing_cursor
            && candidate.stream_version.0 < existing.stream_version.0)
}

fn projection_checkpoint_cursor_number(cursor: &VidaEventCursor) -> u64 {
    cursor
        .0
        .rsplit_once('-')
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryOperationalJournal, InMemoryTaskStore, JournalAppendRequest, JournalArtifactRecord,
        JournalIdempotencyState, JournalOutboxState, JournalProjectionFailure, OperationalJournal,
        RunWorkflowJournalRepository, TaskStore, TaskflowStateError, run_workflow_snapshot_record,
        verify_run_workflow_repository_conformance,
        verify_run_workflow_repository_corrupt_payload_fails_closed,
    };
    use taskflow_contracts::{
        DependencyEdge, TaskRecord, VidaAggregateRef, VidaArtifactRef, VidaCommandRef,
        VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef, VidaEventCursor, VidaEventRef,
        VidaIdempotencyKey, VidaOperation, VidaProjectionCheckpoint, VidaProjectionRef,
        VidaReceiptId, VidaSchemaId, VidaSchemaVersion, VidaStreamRef, VidaStreamVersion,
        VidaTimestamp,
    };
    use taskflow_core::{
        IssueType, TaskId,
        role_step::TaskRoleStep,
        run_workflow::{
            RunWorkflowAggregate, RunWorkflowCommand, RunWorkflowEffectIntent, RunWorkflowEvent,
            RunWorkflowState,
        },
    };

    #[test]
    fn in_memory_store_round_trips_task_records() {
        let mut store = InMemoryTaskStore::default();
        let task = TaskRecord::new(
            TaskId::new("vida-rf1-taskflow-state"),
            "state",
            IssueType::Task,
        );

        store.upsert_task(task.clone());

        let loaded = store
            .get_task(&TaskId::new("vida-rf1-taskflow-state"))
            .expect("task should exist");
        assert_eq!(loaded.title, task.title);
    }

    #[test]
    fn missing_task_is_reported() {
        let store = InMemoryTaskStore::default();
        let error = store
            .get_task(&TaskId::new("missing"))
            .expect_err("task should not exist");
        assert_eq!(error, TaskflowStateError::TaskNotFound("missing".into()));
    }

    #[test]
    fn dependency_listing_is_scoped_to_issue() {
        let mut store = InMemoryTaskStore::default();
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-state"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-core"),
            dependency_type: "blocks".into(),
        });
        store.add_dependency(DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-runtime"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-state"),
            dependency_type: "blocks".into(),
        });

        let rows = store.list_dependencies(&TaskId::new("vida-rf1-taskflow-state"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].depends_on_id.as_str(), "vida-rf1-taskflow-core");
    }

    #[test]
    fn operational_journal_appends_and_reads_with_expected_version() {
        let mut journal = InMemoryOperationalJournal::default();
        let request = append_request(0, vec![event(1)], vec![effect("effect-1")]);

        let receipt = journal.append(request).expect("append should pass");

        assert_eq!(receipt.stream_id, VidaStreamRef("stream-1".to_string()));
        assert_eq!(receipt.stream_version, VidaStreamVersion(1));
        assert_eq!(receipt.event_count, 1);
        assert_eq!(receipt.effect_intent_count, 1);
        assert_eq!(
            receipt.first_global_cursor,
            Some(VidaEventCursor("global-1".to_string()))
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.read_global_after(None, 10).len(), 1);

        let conflict = journal
            .append(append_request(0, vec![event(2)], Vec::new()))
            .expect_err("changed payload with duplicate idempotency key must fail");
        assert_eq!(
            conflict,
            TaskflowStateError::IdempotencyConflict("idem-1".to_string())
        );
    }

    #[test]
    fn operational_journal_rejects_event_stream_mismatch() {
        let mut journal = InMemoryOperationalJournal::default();

        journal
            .append(append_request(0, vec![event(1)], Vec::new()))
            .expect("victim append should pass");
        let mut malformed = append_request(0, vec![event(2)], Vec::new());
        malformed.stream_id = VidaStreamRef("attacker-stream".to_string());
        malformed.idempotency_key = VidaIdempotencyKey("idem-2".to_string());

        let error = journal
            .append(malformed)
            .expect_err("mismatched event stream must fail");

        assert_eq!(
            error,
            TaskflowStateError::EventStreamMismatch {
                request_stream_id: "attacker-stream".to_string(),
                event_stream_id: "stream-1".to_string(),
                event_id: "event-2".to_string(),
            }
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert!(
            journal
                .load_stream(&VidaStreamRef("attacker-stream".to_string()))
                .is_empty()
        );
    }

    #[test]
    fn operational_journal_returns_cached_receipt_for_same_payload_retry() {
        let mut journal = InMemoryOperationalJournal::default();

        let receipt = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let exact_retry = journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("same payload retry should return cached receipt");
        let updated_version_retry = journal
            .append(append_request(1, vec![event(1)], vec![effect("effect-1")]))
            .expect("same payload retry with updated precondition should return cached receipt");

        assert_eq!(exact_retry, receipt);
        assert_eq!(updated_version_retry, receipt);
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 1);
    }

    #[test]
    fn operational_journal_rejects_changed_payload_for_same_idempotency_key() {
        let mut journal = InMemoryOperationalJournal::default();

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("first append should pass");
        let replay = journal
            .append(append_request(1, vec![event(2)], vec![effect("effect-2")]))
            .expect_err("changed payload with same idempotency key must fail");

        assert_eq!(
            replay,
            TaskflowStateError::IdempotencyConflict("idem-1".to_string())
        );
        assert_eq!(
            journal
                .load_stream(&VidaStreamRef("stream-1".to_string()))
                .len(),
            1
        );
        assert_eq!(journal.claim_outbox_batch("worker-1", 10).len(), 1);
    }

    #[test]
    fn operational_journal_tracks_idempotency_lifecycle() {
        let mut journal = InMemoryOperationalJournal::default();
        let key = VidaIdempotencyKey("idem-1".to_string());
        journal
            .record_idempotency_started(key.clone(), VidaCommandRef("command-1".to_string()))
            .expect("start should pass");
        journal
            .record_idempotency_completed(&key, VidaReceiptId("receipt-1".to_string()))
            .expect("complete should pass");

        let record = journal
            .idempotency_record(&key)
            .expect("idempotency record should exist");
        assert_eq!(record.state, JournalIdempotencyState::Completed);
        assert_eq!(
            record.receipt_id,
            Some(VidaReceiptId("receipt-1".to_string()))
        );
        assert!(
            journal
                .record_idempotency_started(key, VidaCommandRef("command-1".to_string()))
                .is_err()
        );
    }

    #[test]
    fn operational_journal_claims_outbox_and_records_projection_artifacts() {
        let mut journal = InMemoryOperationalJournal::default();
        journal
            .append(append_request(0, Vec::new(), vec![effect("effect-1")]))
            .expect("append should pass");

        let claimed = journal.claim_outbox_batch("worker-1", 1);
        assert_eq!(claimed.len(), 1);
        assert!(matches!(
            claimed[0].state,
            JournalOutboxState::Claimed { ref consumer_id } if consumer_id == "worker-1"
        ));
        journal
            .mark_outbox_succeeded(&claimed[0].outbox_id)
            .expect("mark succeeded should pass");

        let checkpoint = projection_checkpoint(1);
        journal.record_projection_checkpoint(checkpoint.clone());
        assert_eq!(
            journal.projection_checkpoint(&VidaProjectionRef("projection-1".to_string())),
            Some(&checkpoint)
        );
        journal.record_projection_checkpoint(projection_checkpoint(0));
        assert_eq!(
            journal.projection_checkpoint(&VidaProjectionRef("projection-1".to_string())),
            Some(&checkpoint),
            "out-of-order older checkpoints must not overwrite newer projection state"
        );

        let failure = JournalProjectionFailure {
            projection_id: VidaProjectionRef("projection-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            error: "projection failed".to_string(),
        };
        journal.record_projection_failure(failure.clone());
        assert_eq!(journal.projection_failures(), &[failure]);

        let artifact = JournalArtifactRecord {
            artifact_ref: VidaArtifactRef("artifact-1".to_string()),
            content_hash: "sha256:abc".to_string(),
            path: "artifacts/artifact-1.json".to_string(),
        };
        journal.index_artifact(artifact.clone());
        assert_eq!(
            journal.artifact(&VidaArtifactRef("artifact-1".to_string())),
            Some(&artifact)
        );
    }

    #[test]
    fn run_workflow_repository_appends_and_replays_from_journal() {
        let mut journal = InMemoryOperationalJournal::default();

        let report = verify_run_workflow_repository_conformance(&mut journal, "run-031")
            .expect("repository conformance should pass");

        assert_eq!(report.run_id, "run-031");
        assert_eq!(report.event_count, 2);
        assert!(!report.final_snapshot_hash.is_empty());
    }

    #[test]
    fn run_workflow_repository_snapshot_replay_hash_matches_journal_replay() {
        let mut journal = InMemoryOperationalJournal::default();

        let first = verify_run_workflow_repository_conformance(&mut journal, "run-031-hash")
            .expect("repository conformance should pass");
        let replay = RunWorkflowJournalRepository::new(&mut journal)
            .load("run-031-hash", "ldr-031")
            .expect("repository replay should pass");
        RunWorkflowJournalRepository::new(&mut journal).save_snapshot(&replay);
        let snapshot = RunWorkflowJournalRepository::new(&mut journal)
            .load_snapshot("run-031-hash", "ldr-031")
            .expect("snapshot load should pass")
            .expect("snapshot should exist");

        assert_eq!(replay.snapshot_replay_hash(), first.final_snapshot_hash);
        assert_eq!(snapshot.snapshot_replay_hash(), first.final_snapshot_hash);
        assert_eq!(replay.version, 2);
    }

    #[test]
    fn run_workflow_repository_load_fails_closed_on_corrupt_payload() {
        let mut journal = InMemoryOperationalJournal::default();

        verify_run_workflow_repository_corrupt_payload_fails_closed(
            &mut journal,
            "run-031-corrupt",
        )
        .expect("corrupt payload must fail closed");
    }

    #[test]
    fn run_workflow_repository_snapshot_load_fails_closed_on_future_schema() {
        let mut journal = InMemoryOperationalJournal::default();
        let aggregate = RunWorkflowAggregate::from_snapshot(
            "run-031-future",
            "ldr-031",
            RunWorkflowState::Active {
                step: TaskRoleStep::developer(),
            },
            3,
        );
        let mut snapshot = run_workflow_snapshot_record(&aggregate);
        snapshot.schema_version = VidaSchemaVersion(99);
        journal.record_aggregate_snapshot(snapshot);

        let error = RunWorkflowJournalRepository::new(&mut journal)
            .load_snapshot("run-031-future", "ldr-031")
            .expect_err("future snapshot schema must fail closed");

        assert!(matches!(error, TaskflowStateError::PayloadDecode(_)));
    }

    #[test]
    fn run_workflow_repository_load_rejects_forged_terminal_state() {
        let mut journal = InMemoryOperationalJournal::default();
        let run_id = "run-031-forged";
        let command_id = VidaCommandRef(format!("run-workflow:{run_id}:1"));
        let forged = RunWorkflowEvent {
            command: RunWorkflowCommand::Close,
            state_before: RunWorkflowState::Idle,
            state_after: RunWorkflowState::Completed,
            effect_intents: vec![RunWorkflowEffectIntent::RecordTerminal],
            blocker_code: None,
        };

        journal
            .append(JournalAppendRequest {
                stream_id: VidaStreamRef(format!("run-workflow:{run_id}")),
                expected_stream_version: Some(VidaStreamVersion(0)),
                command_id: command_id.clone(),
                idempotency_key: VidaIdempotencyKey(format!(
                    "{command_id}:1",
                    command_id = command_id.0
                )),
                causation_id: None,
                correlation_id: Some("ldr-031".to_string()),
                events: vec![VidaDomainEventEnvelope {
                    schema_id: VidaSchemaId("taskflow.run_workflow.event".to_string()),
                    event_version: VidaSchemaVersion(1),
                    event_id: VidaEventRef(format!("{}:event", command_id.0)),
                    command_id: Some(command_id),
                    causation_id: None,
                    stream_id: VidaStreamRef(format!("run-workflow:{run_id}")),
                    stream_version: VidaStreamVersion(1),
                    aggregate_id: VidaAggregateRef(run_id.to_string()),
                    occurred_at: VidaTimestamp("version-1".to_string()),
                    payload: serde_json::to_value(forged).expect("forged event should serialize"),
                    trace: serde_json::json!({}),
                }],
                effect_intents: Vec::new(),
            })
            .expect("raw append should set up forged journal event");

        let error = RunWorkflowJournalRepository::new(&mut journal)
            .load(run_id, "ldr-031")
            .expect_err("forged terminal state must fail closed");
        assert!(matches!(
            error,
            TaskflowStateError::JournalReplayValidation(_)
        ));
    }

    #[test]
    fn run_workflow_repository_load_rejects_non_sequential_metadata() {
        let mut journal = InMemoryOperationalJournal::default();
        let run_id = "run-031-gap";
        let stream_id = VidaStreamRef(format!("run-workflow:{run_id}"));
        let command_id = VidaCommandRef(format!("run-workflow:{run_id}:2"));
        let mut aggregate = RunWorkflowAggregate::new(run_id, "ldr-031");
        let valid_event = aggregate.handle(RunWorkflowCommand::Start {
            first_step: TaskRoleStep::planning(),
        });

        journal
            .append(JournalAppendRequest {
                stream_id: stream_id.clone(),
                expected_stream_version: Some(VidaStreamVersion(0)),
                command_id: command_id.clone(),
                idempotency_key: VidaIdempotencyKey(format!("{}:2", command_id.0)),
                causation_id: None,
                correlation_id: Some("ldr-031".to_string()),
                events: vec![VidaDomainEventEnvelope {
                    schema_id: VidaSchemaId("taskflow.run_workflow.event".to_string()),
                    event_version: VidaSchemaVersion(1),
                    event_id: VidaEventRef(format!("{}:event", command_id.0)),
                    command_id: Some(command_id),
                    causation_id: None,
                    stream_id,
                    stream_version: VidaStreamVersion(2),
                    aggregate_id: VidaAggregateRef(run_id.to_string()),
                    occurred_at: VidaTimestamp("version-2".to_string()),
                    payload: serde_json::to_value(valid_event).expect("event should serialize"),
                    trace: serde_json::json!({}),
                }],
                effect_intents: Vec::new(),
            })
            .expect("raw append should set up metadata gap");

        let error = RunWorkflowJournalRepository::new(&mut journal)
            .load(run_id, "ldr-031")
            .expect_err("non-sequential stream_version must fail closed");
        assert!(matches!(
            error,
            TaskflowStateError::JournalReplayValidation(_)
        ));
    }

    #[test]
    fn run_workflow_repository_append_rejects_caller_forged_event() {
        let mut journal = InMemoryOperationalJournal::default();
        let aggregate = RunWorkflowAggregate::new("run-031-append-forged", "ldr-031");
        let forged = RunWorkflowEvent {
            command: RunWorkflowCommand::Close,
            state_before: RunWorkflowState::Idle,
            state_after: RunWorkflowState::Completed,
            effect_intents: vec![RunWorkflowEffectIntent::RecordTerminal],
            blocker_code: None,
        };

        let error = RunWorkflowJournalRepository::new(&mut journal)
            .append(&aggregate, forged)
            .expect_err("append must validate events against the state machine");
        assert!(matches!(
            error,
            TaskflowStateError::JournalReplayValidation(_)
        ));
    }

    fn append_request(
        expected_stream_version: u64,
        events: Vec<VidaDomainEventEnvelope>,
        effect_intents: Vec<VidaEffectIntent>,
    ) -> JournalAppendRequest {
        JournalAppendRequest {
            stream_id: VidaStreamRef("stream-1".to_string()),
            expected_stream_version: Some(VidaStreamVersion(expected_stream_version)),
            command_id: VidaCommandRef("command-1".to_string()),
            idempotency_key: VidaIdempotencyKey("idem-1".to_string()),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            correlation_id: Some("correlation-1".to_string()),
            events,
            effect_intents,
        }
    }

    fn event(stream_version: u64) -> VidaDomainEventEnvelope {
        VidaDomainEventEnvelope {
            schema_id: taskflow_contracts::VidaSchemaId("schema.task.updated".to_string()),
            event_version: taskflow_contracts::VidaSchemaVersion(1),
            event_id: VidaEventRef(format!("event-{stream_version}")),
            command_id: Some(VidaCommandRef("command-1".to_string())),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            stream_id: VidaStreamRef("stream-1".to_string()),
            stream_version: VidaStreamVersion(stream_version),
            aggregate_id: VidaAggregateRef("task-1".to_string()),
            occurred_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
            payload: serde_json::json!({ "stream_version": stream_version }),
            trace: serde_json::json!({ "correlation_id": "correlation-1" }),
        }
    }

    fn effect(effect_id: &str) -> VidaEffectIntent {
        VidaEffectIntent {
            effect_id: VidaEffectRef(effect_id.to_string()),
            operation: VidaOperation("vida.effect.dispatch".to_string()),
            command_id: VidaCommandRef("command-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            payload: serde_json::json!({ "effect_id": effect_id }),
        }
    }

    fn projection_checkpoint(stream_version: u64) -> VidaProjectionCheckpoint {
        VidaProjectionCheckpoint {
            projection_id: VidaProjectionRef("projection-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            event_cursor: VidaEventCursor(format!("global-{stream_version}")),
            stream_version: VidaStreamVersion(stream_version),
            updated_at: VidaTimestamp("2026-06-22T00:00:00Z".to_string()),
        }
    }
}
