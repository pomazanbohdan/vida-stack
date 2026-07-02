use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use effectum::{Job, JobRecoveryBehavior};
use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    VidaCommandRef, VidaEffectRef, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaOperation, VidaStreamRef,
};
use taskflow_state::{JournalOutboxState, OperationalJournal};
use taskflow_state_redb::{RedbOperationalJournal, RedbOutboxEffectRecord};

pub type EffectumQueue = effectum::Queue;

pub const EFFECTUM_OUTBOX_WORKER: &str = "vida.redb.outbox.effect";
pub const HOST_BRIDGE_ADAPTER_REQUEST_WORKER: &str = "vida.host_bridge.adapter_request";
pub const DEAD_LETTER_BLOCKER_CODE: &str = "vida_job_dead_letter";
pub const HOST_BRIDGE_DEAD_LETTER_BLOCKER_CODE: &str = "host_bridge_adapter_request_dead_letter";

#[derive(Debug, thiserror::Error)]
pub enum RuntimeLocalJobError {
    #[error("decode Effectum outbox job payload: {detail}")]
    DecodeEffectumOutboxPayload { detail: String },
    #[error(
        "refuse to acknowledge redb outbox `{outbox_id}` effect `{effect_id}` from Effectum metadata without executing the persisted VidaEffectIntent"
    )]
    MetadataOnlyEffectumAck {
        outbox_id: String,
        effect_id: String,
    },
    #[error("worker command payload missing `{field}`")]
    MissingWorkerPayloadField { field: String },
    #[error("open redb outbox journal `{path}` for worker ack: {detail}")]
    OpenRedbOutboxForWorkerAck { path: String, detail: String },
    #[error("read redb outbox `{outbox_id}` for worker ack: {detail}")]
    ReadRedbOutboxForWorkerAck { outbox_id: String, detail: String },
    #[error("redb outbox `{outbox_id}` is missing for worker ack")]
    MissingRedbOutboxForWorkerAck { outbox_id: String },
    #[error("worker ack authority `{authority}` is not redb_outbox")]
    WorkerAckAuthorityMismatch { authority: String },
    #[error("worker ack runner `{runner}` is not effectum")]
    WorkerAckRunnerMismatch { runner: String },
    #[error(
        "worker ack payload operation `{payload_operation}` does not match command operation `{command_operation}`"
    )]
    WorkerAckOperationMismatch {
        payload_operation: String,
        command_operation: String,
    },
    #[error(
        "worker ack effect_id `{effect_id}` does not match persisted effect `{persisted_effect_id}`"
    )]
    WorkerAckEffectMismatch {
        effect_id: String,
        persisted_effect_id: String,
    },
    #[error("worker ack command_id must match persisted outbox `{outbox_id}`")]
    WorkerAckCommandIdMismatch { outbox_id: String },
    #[error("worker ack idempotency_key must match persisted outbox `{outbox_id}`")]
    WorkerAckIdempotencyKeyMismatch { outbox_id: String },
    #[error(
        "worker ack claimed_by `{claimed_by}` does not match persisted claim `{persisted_claim}`"
    )]
    WorkerAckClaimMismatch {
        claimed_by: String,
        persisted_claim: String,
    },
    #[error("worker ack requires claimed outbox state, found `{state}`")]
    WorkerAckStateInvalid { state: String },
    #[error("mark outbox `{outbox_id}` succeeded: {detail}")]
    MarkOutboxSucceeded { outbox_id: String, detail: String },
    #[error("mark outbox `{outbox_id}` failed: {detail}")]
    MarkOutboxFailed { outbox_id: String, detail: String },
    #[error("unsupported worker command outcome `{outcome}`")]
    UnsupportedWorkerCommandOutcome { outcome: String },
    #[error("create Effectum queue directory `{path}`: {source}")]
    CreateEffectumQueueDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("open Effectum queue `{path}` with recovery `{recovery_behavior}`: {detail}")]
    OpenEffectumQueue {
        path: String,
        recovery_behavior: String,
        detail: String,
    },
    #[error("refuse to enqueue terminal durable job `{job_id}` with lifecycle `{lifecycle:?}`")]
    EnqueueTerminalDurableJob {
        job_id: String,
        lifecycle: DurableJobLifecycle,
    },
    #[error("open redb outbox journal `{path}`: {detail}")]
    OpenRedbOutboxJournal { path: String, detail: String },
    #[error("read redb outbox records `{path}`: {detail}")]
    ReadRedbOutboxRecords { path: String, detail: String },
    #[error("unsupported Effectum recovery behavior `{behavior}`")]
    UnsupportedEffectumRecoveryBehavior { behavior: String },
    #[error("serialize Effectum job payload `{job_id}`: {detail}")]
    SerializeEffectumJobPayload { job_id: String, detail: String },
    #[error("enqueue Effectum job `{job_id}` for outbox `{outbox_id}`: {detail}")]
    EnqueueEffectumJob {
        job_id: String,
        outbox_id: String,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectumQueueConfig {
    pub sqlite_path: PathBuf,
    pub recovery_behavior: String,
}

impl EffectumQueueConfig {
    pub fn new(sqlite_path: impl Into<PathBuf>) -> Self {
        Self {
            sqlite_path: sqlite_path.into(),
            recovery_behavior: "fail_and_retry_immediately".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectumWorkerSpec {
    pub job_kind: String,
    pub command_operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectumWorkerRegistry {
    pub workers: Vec<EffectumWorkerSpec>,
}

impl EffectumWorkerRegistry {
    pub fn outbox_commands(operation: impl Into<String>) -> Self {
        Self {
            workers: vec![EffectumWorkerSpec {
                job_kind: EFFECTUM_OUTBOX_WORKER.to_string(),
                command_operation: operation.into(),
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub base_backoff_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryBackoffKind {
    Exponential,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryBackoffPolicy {
    max_attempts: u64,
    base_delay_millis: u64,
    kind: RetryBackoffKind,
}

impl RetryBackoffPolicy {
    pub const fn exponential_seconds(max_attempts: u64, base_delay_seconds: u64) -> Self {
        Self {
            max_attempts,
            base_delay_millis: base_delay_seconds.saturating_mul(1_000),
            kind: RetryBackoffKind::Exponential,
        }
    }

    pub const fn linear_seconds(max_attempts: u64, base_delay_seconds: u64) -> Self {
        Self {
            max_attempts,
            base_delay_millis: base_delay_seconds.saturating_mul(1_000),
            kind: RetryBackoffKind::Linear,
        }
    }

    pub const fn linear_attempts(max_attempts: u64, base_delay_millis: u64) -> Self {
        Self {
            max_attempts,
            base_delay_millis,
            kind: RetryBackoffKind::Linear,
        }
    }

    pub const fn linear_millis(max_wait_millis: u64, base_delay_millis: u64) -> Self {
        let max_attempts = if base_delay_millis == 0 {
            1
        } else {
            max_wait_millis / base_delay_millis
        };
        Self {
            max_attempts,
            base_delay_millis,
            kind: RetryBackoffKind::Linear,
        }
    }

    pub const fn max_attempts(&self) -> u64 {
        self.max_attempts
    }

    pub const fn max_attempts_usize(&self) -> usize {
        if self.max_attempts > usize::MAX as u64 {
            usize::MAX
        } else {
            self.max_attempts as usize
        }
    }

    pub const fn base_delay_millis(&self) -> u64 {
        self.base_delay_millis
    }

    pub fn retry_delay_millis(&self, attempt_count: u64) -> u64 {
        match self.kind {
            RetryBackoffKind::Exponential => {
                let exponent = attempt_count.saturating_sub(1).min(8);
                self.base_delay_millis.saturating_mul(1 << exponent)
            }
            RetryBackoffKind::Linear => self.base_delay_millis.saturating_mul(attempt_count.max(1)),
        }
    }

    pub fn retry_delay_seconds(&self, attempt_count: u64) -> u64 {
        self.retry_delay_millis(attempt_count).saturating_add(999) / 1_000
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_seconds: 30,
        }
    }
}

impl RetryPolicy {
    pub fn backoff_policy(&self) -> RetryBackoffPolicy {
        RetryBackoffPolicy::exponential_seconds(self.max_attempts, self.base_backoff_seconds)
    }

    pub fn retry_after_seconds(&self, attempt_count: u64) -> u64 {
        self.backoff_policy().retry_delay_seconds(attempt_count)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxJobSnapshot {
    pub outbox_id: VidaEventRef,
    pub effect_id: VidaEffectRef,
    pub state: JournalOutboxState,
    pub attempt_count: u64,
    pub source_event_cursor: Option<VidaEventCursor>,
    pub failure_reason: Option<String>,
}

impl From<&RedbOutboxEffectRecord> for OutboxJobSnapshot {
    fn from(record: &RedbOutboxEffectRecord) -> Self {
        Self {
            outbox_id: record.outbox_id.clone(),
            effect_id: record.effect.effect_id.clone(),
            state: record.state.clone(),
            attempt_count: record.attempt_count,
            source_event_cursor: record.source_event_cursor.clone(),
            failure_reason: record.failure_reason.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableJobId(pub String);

impl DurableJobId {
    pub fn from_effect_id(effect_id: &VidaEffectRef) -> Self {
        let stable = effect_id
            .0
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '-'
                }
            })
            .collect::<String>();
        Self(format!("vida-effect-{stable}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableJobLifecycle {
    Pending,
    Retryable,
    Running,
    Succeeded,
    DeadLettered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableJobBlocker {
    pub code: String,
    pub repair_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableJobTraceEntry {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectAckOutcome {
    Succeeded,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerCommandSubmission {
    pub operation: VidaOperation,
    pub command_id: VidaCommandRef,
    pub idempotency_key: VidaIdempotencyKey,
    pub stream_id: VidaStreamRef,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectumEnqueueReceipt {
    pub job_id: DurableJobId,
    pub effectum_job_id: String,
    pub outbox_id: VidaEventRef,
    pub effect_id: VidaEffectRef,
    pub duplicate: bool,
    pub queue_path: PathBuf,
    pub trace: Vec<DurableJobTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeRequestEnqueueReceipt {
    pub job_id: DurableJobId,
    pub effectum_job_id: String,
    pub request_id: String,
    pub duplicate: bool,
    pub queue_path: PathBuf,
    pub trace: Vec<DurableJobTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectumOutboxWorker {
    pub command_operation: VidaOperation,
}

#[derive(Debug, Clone)]
pub struct EffectumWorkerPipeline {
    pub worker: EffectumOutboxWorker,
    pub sender: tokio::sync::mpsc::UnboundedSender<WorkerCommandSubmission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EffectumOutboxJobPayload {
    job_id: String,
    outbox_id: String,
    effect_id: String,
    authority: String,
    runner: String,
    next_action: String,
    claimed_by: Option<String>,
    trace: Vec<DurableJobTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HostBridgeRequestJobPayload {
    job_id: String,
    request_id: String,
    run_id: Option<String>,
    authority: String,
    runner: String,
    next_action: String,
    trace: Vec<DurableJobTraceEntry>,
}

impl EffectumOutboxWorker {
    pub fn new(command_operation: impl Into<String>) -> Self {
        Self {
            command_operation: VidaOperation(command_operation.into()),
        }
    }

    pub fn acknowledgement_command(
        &self,
        snapshot: &OutboxJobSnapshot,
        outcome: EffectAckOutcome,
    ) -> WorkerCommandSubmission {
        let outcome_payload = match outcome {
            EffectAckOutcome::Succeeded => serde_json::json!({ "status": "succeeded" }),
            EffectAckOutcome::Failed { reason } => {
                serde_json::json!({ "status": "failed", "reason": reason })
            }
        };
        let command_id = self.command_id_for(snapshot);
        let idempotency_key = self.idempotency_key_for(snapshot);
        WorkerCommandSubmission {
            operation: self.command_operation.clone(),
            command_id,
            idempotency_key,
            stream_id: snapshot
                .source_event_cursor
                .as_ref()
                .map(|cursor| VidaStreamRef(format!("job-trace:{}", cursor.0)))
                .unwrap_or_else(|| VidaStreamRef(format!("job-trace:{}", snapshot.outbox_id.0))),
            payload: serde_json::json!({
                "outbox_id": snapshot.outbox_id.0,
                "effect_id": snapshot.effect_id.0,
                "runner": "effectum",
                "authority": "redb_outbox",
                "operation": self.command_operation.0,
                "command_id": self.command_id_for(snapshot).0,
                "idempotency_key": self.idempotency_key_for(snapshot).0,
                "claimed_by": claimed_consumer_id(snapshot),
                "outcome": outcome_payload,
            }),
        }
    }

    fn command_id_for(&self, snapshot: &OutboxJobSnapshot) -> VidaCommandRef {
        VidaCommandRef(format!("job-ack:{}", snapshot.outbox_id.0))
    }

    fn idempotency_key_for(&self, snapshot: &OutboxJobSnapshot) -> VidaIdempotencyKey {
        VidaIdempotencyKey(format!("job-ack:{}", snapshot.outbox_id.0))
    }
}

fn claimed_consumer_id(snapshot: &OutboxJobSnapshot) -> Option<&str> {
    match &snapshot.state {
        JournalOutboxState::Claimed { consumer_id } => Some(consumer_id.as_str()),
        _ => None,
    }
}

impl EffectumWorkerPipeline {
    pub fn new(
        command_operation: impl Into<String>,
        sender: tokio::sync::mpsc::UnboundedSender<WorkerCommandSubmission>,
    ) -> Self {
        Self {
            worker: EffectumOutboxWorker::new(command_operation),
            sender,
        }
    }
}

pub fn effectum_outbox_job_runner() -> effectum::JobRunner<EffectumWorkerPipeline> {
    effectum::JobRunner::builder(
        EFFECTUM_OUTBOX_WORKER,
        |job: effectum::RunningJob, _pipeline: EffectumWorkerPipeline| async move {
            let payload: EffectumOutboxJobPayload = job.json_payload().map_err(|error| {
                RuntimeLocalJobError::DecodeEffectumOutboxPayload {
                    detail: error.to_string(),
                }
            })?;
            Err::<WorkerCommandSubmission, RuntimeLocalJobError>(
                RuntimeLocalJobError::MetadataOnlyEffectumAck {
                    outbox_id: payload.outbox_id,
                    effect_id: payload.effect_id,
                },
            )
        },
    )
    .build()
}

pub fn apply_worker_command_to_redb(
    journal_path: &Path,
    command: &WorkerCommandSubmission,
) -> Result<(), RuntimeLocalJobError> {
    let outbox_id = required_payload_str(command, "outbox_id")?;
    let effect_id = required_payload_str(command, "effect_id")?;
    let authority = required_payload_str(command, "authority")?;
    let runner = required_payload_str(command, "runner")?;
    let operation = required_payload_str(command, "operation")?;
    let command_id = required_payload_str(command, "command_id")?;
    let idempotency_key = required_payload_str(command, "idempotency_key")?;
    let claimed_by = required_payload_str(command, "claimed_by")?;
    let outcome = command
        .payload
        .get("outcome")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RuntimeLocalJobError::MissingWorkerPayloadField {
            field: "outcome.status".to_string(),
        })?;
    let mut journal = RedbOperationalJournal::open(journal_path).map_err(|error| {
        RuntimeLocalJobError::OpenRedbOutboxForWorkerAck {
            path: journal_path.display().to_string(),
            detail: error.to_string(),
        }
    })?;
    let outbox_ref = VidaEventRef(outbox_id.to_string());
    let persisted = journal
        .outbox_effect_record(&outbox_ref)
        .map_err(|error| RuntimeLocalJobError::ReadRedbOutboxForWorkerAck {
            outbox_id: outbox_id.to_string(),
            detail: error.to_string(),
        })?
        .ok_or_else(|| RuntimeLocalJobError::MissingRedbOutboxForWorkerAck {
            outbox_id: outbox_id.to_string(),
        })?;

    validate_worker_ack_command(
        command,
        &persisted,
        effect_id,
        authority,
        runner,
        operation,
        command_id,
        idempotency_key,
        claimed_by,
    )?;

    match outcome {
        "succeeded" => journal.mark_outbox_succeeded(&outbox_ref).map_err(|error| {
            RuntimeLocalJobError::MarkOutboxSucceeded {
                outbox_id: outbox_id.to_string(),
                detail: error.to_string(),
            }
        }),
        "failed" => {
            let reason = command
                .payload
                .get("outcome")
                .and_then(|value| value.get("reason"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Effectum worker reported failure")
                .to_string();
            journal
                .mark_outbox_failed(&outbox_ref, reason)
                .map_err(|error| RuntimeLocalJobError::MarkOutboxFailed {
                    outbox_id: outbox_id.to_string(),
                    detail: error.to_string(),
                })
        }
        other => Err(RuntimeLocalJobError::UnsupportedWorkerCommandOutcome {
            outcome: other.to_string(),
        }),
    }
}

fn required_payload_str<'a>(
    command: &'a WorkerCommandSubmission,
    field: &str,
) -> Result<&'a str, RuntimeLocalJobError> {
    command
        .payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RuntimeLocalJobError::MissingWorkerPayloadField {
            field: field.to_string(),
        })
}

fn validate_worker_ack_command(
    command: &WorkerCommandSubmission,
    persisted: &RedbOutboxEffectRecord,
    effect_id: &str,
    authority: &str,
    runner: &str,
    operation: &str,
    command_id: &str,
    idempotency_key: &str,
    claimed_by: &str,
) -> Result<(), RuntimeLocalJobError> {
    if authority != "redb_outbox" {
        return Err(RuntimeLocalJobError::WorkerAckAuthorityMismatch {
            authority: authority.to_string(),
        });
    }
    if runner != "effectum" {
        return Err(RuntimeLocalJobError::WorkerAckRunnerMismatch {
            runner: runner.to_string(),
        });
    }
    if operation != command.operation.0 {
        return Err(RuntimeLocalJobError::WorkerAckOperationMismatch {
            payload_operation: operation.to_string(),
            command_operation: command.operation.0.clone(),
        });
    }
    if effect_id != persisted.effect.effect_id.0 {
        return Err(RuntimeLocalJobError::WorkerAckEffectMismatch {
            effect_id: effect_id.to_string(),
            persisted_effect_id: persisted.effect.effect_id.0.clone(),
        });
    }
    let expected_ack = format!("job-ack:{}", persisted.outbox_id.0);
    if command.command_id.0 != expected_ack || command_id != expected_ack {
        return Err(RuntimeLocalJobError::WorkerAckCommandIdMismatch {
            outbox_id: persisted.outbox_id.0.clone(),
        });
    }
    if command.idempotency_key.0 != expected_ack || idempotency_key != expected_ack {
        return Err(RuntimeLocalJobError::WorkerAckIdempotencyKeyMismatch {
            outbox_id: persisted.outbox_id.0.clone(),
        });
    }
    match &persisted.state {
        JournalOutboxState::Claimed { consumer_id } if consumer_id == claimed_by => Ok(()),
        JournalOutboxState::Claimed { consumer_id } => {
            Err(RuntimeLocalJobError::WorkerAckClaimMismatch {
                claimed_by: claimed_by.to_string(),
                persisted_claim: consumer_id.clone(),
            })
        }
        other => Err(RuntimeLocalJobError::WorkerAckStateInvalid {
            state: format!("{other:?}"),
        }),
    }
}

pub async fn open_effectum_queue(
    config: &EffectumQueueConfig,
) -> Result<EffectumQueue, RuntimeLocalJobError> {
    if let Some(parent) = config.sqlite_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            RuntimeLocalJobError::CreateEffectumQueueDirectory {
                path: parent.display().to_string(),
                source: error,
            }
        })?;
    }
    EffectumQueue::builder(&config.sqlite_path)
        .job_recovery_behavior(recovery_behavior(&config.recovery_behavior)?)
        .build()
        .await
        .map_err(|error| RuntimeLocalJobError::OpenEffectumQueue {
            path: config.sqlite_path.display().to_string(),
            recovery_behavior: config.recovery_behavior.clone(),
            detail: error.to_string(),
        })
}

pub async fn enqueue_outbox_job_idempotently(
    queue: &EffectumQueue,
    config: &EffectumQueueConfig,
    plan: &DurableJobPlan,
    policy: &RetryPolicy,
) -> Result<EffectumEnqueueReceipt, RuntimeLocalJobError> {
    if matches!(
        plan.lifecycle,
        DurableJobLifecycle::Succeeded | DurableJobLifecycle::DeadLettered
    ) {
        return Err(RuntimeLocalJobError::EnqueueTerminalDurableJob {
            job_id: plan.job_id.0.clone(),
            lifecycle: plan.lifecycle.clone(),
        });
    }

    let effectum_job_id = deterministic_effectum_job_id(&plan.job_id);
    if queue.get_job_status(effectum_job_id).await.is_ok() {
        return Ok(enqueue_receipt(config, plan, effectum_job_id, true));
    }

    let job = effectum_job_for_plan(plan, policy, effectum_job_id)?;
    match queue.add_job(job).await {
        Ok(job_id) => Ok(enqueue_receipt(config, plan, job_id, false)),
        Err(error) => {
            if queue.get_job_status(effectum_job_id).await.is_ok() {
                Ok(enqueue_receipt(config, plan, effectum_job_id, true))
            } else {
                Err(RuntimeLocalJobError::EnqueueEffectumJob {
                    job_id: plan.job_id.0.clone(),
                    outbox_id: plan.outbox_id.0.clone(),
                    detail: error.to_string(),
                })
            }
        }
    }
}

pub async fn enqueue_host_bridge_request_job_idempotently(
    queue: &EffectumQueue,
    config: &EffectumQueueConfig,
    snapshot: &HostBridgeRequestJobSnapshot,
    policy: &RetryPolicy,
) -> Result<HostBridgeRequestEnqueueReceipt, RuntimeLocalJobError> {
    let plan = plan_host_bridge_request_job(snapshot, policy);
    if matches!(
        plan.lifecycle,
        DurableJobLifecycle::Succeeded | DurableJobLifecycle::DeadLettered
    ) {
        return Err(RuntimeLocalJobError::EnqueueTerminalDurableJob {
            job_id: plan.job_id.0.clone(),
            lifecycle: plan.lifecycle.clone(),
        });
    }

    let effectum_job_id = deterministic_effectum_job_id(&plan.job_id);
    if queue.get_job_status(effectum_job_id).await.is_ok() {
        return Ok(host_bridge_enqueue_receipt(
            config,
            snapshot,
            &plan,
            effectum_job_id,
            true,
        ));
    }

    let job = effectum_host_bridge_job_for_plan(snapshot, &plan, policy, effectum_job_id)?;
    match queue.add_job(job).await {
        Ok(job_id) => Ok(host_bridge_enqueue_receipt(
            config, snapshot, &plan, job_id, false,
        )),
        Err(error) => {
            if queue.get_job_status(effectum_job_id).await.is_ok() {
                Ok(host_bridge_enqueue_receipt(
                    config,
                    snapshot,
                    &plan,
                    effectum_job_id,
                    true,
                ))
            } else {
                Err(RuntimeLocalJobError::EnqueueEffectumJob {
                    job_id: plan.job_id.0.clone(),
                    outbox_id: snapshot.request_id.clone(),
                    detail: error.to_string(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableJobPlan {
    pub job_id: DurableJobId,
    pub outbox_id: VidaEventRef,
    pub effect_id: VidaEffectRef,
    pub lifecycle: DurableJobLifecycle,
    pub next_action: String,
    pub claimed_by: Option<String>,
    pub retry_after_seconds: Option<u64>,
    pub blocker: Option<DurableJobBlocker>,
    pub trace: Vec<DurableJobTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeRequestJobSnapshot {
    pub request_id: String,
    pub run_id: Option<String>,
    pub status: String,
    pub attempt_count: u64,
    pub failure_reason: Option<String>,
    pub result_path: Option<String>,
}

impl HostBridgeRequestJobSnapshot {
    pub fn from_request(request: &serde_json::Value) -> Option<Self> {
        let request_id = request
            .get("request_id")
            .and_then(serde_json::Value::as_str)?
            .to_string();
        Some(Self {
            request_id,
            run_id: request
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            status: request
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("pending")
                .to_string(),
            attempt_count: request
                .get("attempt_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            failure_reason: request
                .get("failure_reason")
                .or_else(|| request.get("blocker_reason"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            result_path: request
                .get("result_path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        })
    }
}

fn host_bridge_request_job_id(request_id: &str) -> DurableJobId {
    let stable = request_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    DurableJobId(format!("host-bridge-request-{stable}"))
}

pub fn plan_host_bridge_request_job(
    snapshot: &HostBridgeRequestJobSnapshot,
    policy: &RetryPolicy,
) -> DurableJobPlan {
    let job_id = host_bridge_request_job_id(&snapshot.request_id);
    let outbox_id = VidaEventRef(snapshot.request_id.clone());
    let effect_id = VidaEffectRef(job_id.0.clone());
    let trace = vec![
        trace_entry("authority", "host_bridge_request"),
        trace_entry("effectum_job_kind", HOST_BRIDGE_ADAPTER_REQUEST_WORKER),
    ];
    match snapshot.status.as_str() {
        "completed" | "pass" | "done" => DurableJobPlan {
            job_id,
            outbox_id,
            effect_id,
            lifecycle: DurableJobLifecycle::Succeeded,
            next_action: "none".to_string(),
            claimed_by: None,
            retry_after_seconds: None,
            blocker: None,
            trace,
        },
        "running" | "executing" => DurableJobPlan {
            job_id,
            outbox_id,
            effect_id,
            lifecycle: DurableJobLifecycle::Running,
            next_action: "wait_for_host_bridge_adapter_result".to_string(),
            claimed_by: snapshot.run_id.clone(),
            retry_after_seconds: None,
            blocker: None,
            trace,
        },
        "blocked" | "failed" if snapshot.attempt_count >= policy.max_attempts => DurableJobPlan {
            job_id,
            outbox_id,
            effect_id,
            lifecycle: DurableJobLifecycle::DeadLettered,
            next_action: "emit_blocked_host_bridge_result".to_string(),
            claimed_by: None,
            retry_after_seconds: None,
            blocker: Some(DurableJobBlocker {
                code: HOST_BRIDGE_DEAD_LETTER_BLOCKER_CODE.to_string(),
                repair_action: format!(
                    "Inspect host bridge request `{}` failure `{}` and retry with corrected adapter evidence.",
                    snapshot.request_id,
                    snapshot
                        .failure_reason
                        .as_deref()
                        .unwrap_or("host bridge adapter exhausted retries")
                ),
            }),
            trace,
        },
        "blocked" | "failed" | "retryable_blocked" => DurableJobPlan {
            job_id,
            outbox_id,
            effect_id,
            lifecycle: DurableJobLifecycle::Retryable,
            next_action: "retry_host_bridge_adapter_request".to_string(),
            claimed_by: None,
            retry_after_seconds: Some(backoff_seconds(snapshot.attempt_count, policy)),
            blocker: None,
            trace,
        },
        _ => DurableJobPlan {
            job_id,
            outbox_id,
            effect_id,
            lifecycle: DurableJobLifecycle::Pending,
            next_action: "enqueue_host_bridge_adapter_request_idempotently".to_string(),
            claimed_by: None,
            retry_after_seconds: None,
            blocker: None,
            trace,
        },
    }
}

pub fn unavailable_job_status(job_id: &str, reason: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "job_id": job_id,
        "status": "unavailable",
        "authority": "redb_outbox",
        "runner": "effectum",
        "next_action": "provide a readable persisted redb outbox journal before claiming job lifecycle state",
        "blocker": DurableJobBlocker {
            code: "vida_job_journal_unavailable".to_string(),
            repair_action: reason.into(),
        },
        "trace": [
            {"kind": "authority", "detail": "redb_outbox"},
            {"kind": "effectum_job_kind", "detail": EFFECTUM_OUTBOX_WORKER},
        ]
    })
}

pub fn plan_outbox_job_from_redb(
    journal_path: &Path,
    job_id: &str,
    policy: &RetryPolicy,
) -> Result<Option<DurableJobPlan>, RuntimeLocalJobError> {
    let journal = RedbOperationalJournal::open(journal_path).map_err(|error| {
        RuntimeLocalJobError::OpenRedbOutboxJournal {
            path: journal_path.display().to_string(),
            detail: error.to_string(),
        }
    })?;
    let records = journal.outbox_effect_records().map_err(|error| {
        RuntimeLocalJobError::ReadRedbOutboxRecords {
            path: journal_path.display().to_string(),
            detail: error.to_string(),
        }
    })?;
    Ok(plan_outbox_job_from_records(&records, job_id, policy))
}

pub fn plan_outbox_job_from_records(
    records: &[RedbOutboxEffectRecord],
    job_id: &str,
    policy: &RetryPolicy,
) -> Option<DurableJobPlan> {
    let selected = if job_id == "latest" {
        records.iter().max_by_key(|record| {
            record
                .source_event_cursor
                .as_ref()
                .map(|cursor| cursor.0.clone())
        })
    } else {
        records.iter().find(|record| {
            record.outbox_id.0 == job_id
                || record.effect.effect_id.0 == job_id
                || DurableJobId::from_effect_id(&record.effect.effect_id).0 == job_id
        })
    }?;
    Some(plan_outbox_job(&OutboxJobSnapshot::from(selected), policy))
}

pub fn plan_outbox_job(snapshot: &OutboxJobSnapshot, policy: &RetryPolicy) -> DurableJobPlan {
    let job_id = DurableJobId::from_effect_id(&snapshot.effect_id);
    let mut trace = vec![
        trace_entry("authority", "redb_outbox"),
        trace_entry("effectum_job_kind", EFFECTUM_OUTBOX_WORKER),
    ];
    if let Some(cursor) = &snapshot.source_event_cursor {
        trace.push(trace_entry("source_event_cursor", &cursor.0));
    }

    match &snapshot.state {
        JournalOutboxState::Pending => DurableJobPlan {
            job_id,
            outbox_id: snapshot.outbox_id.clone(),
            effect_id: snapshot.effect_id.clone(),
            lifecycle: DurableJobLifecycle::Pending,
            next_action: "enqueue_effectum_job_idempotently".to_string(),
            claimed_by: None,
            retry_after_seconds: None,
            blocker: None,
            trace,
        },
        JournalOutboxState::Claimed { .. } => DurableJobPlan {
            job_id,
            outbox_id: snapshot.outbox_id.clone(),
            effect_id: snapshot.effect_id.clone(),
            lifecycle: DurableJobLifecycle::Retryable,
            next_action: "recover_claimed_job_after_restart".to_string(),
            claimed_by: claimed_consumer_id(snapshot).map(str::to_string),
            retry_after_seconds: Some(backoff_seconds(snapshot.attempt_count, policy)),
            blocker: None,
            trace,
        },
        JournalOutboxState::Failed { reason } if snapshot.attempt_count >= policy.max_attempts => {
            DurableJobPlan {
                job_id,
                outbox_id: snapshot.outbox_id.clone(),
                effect_id: snapshot.effect_id.clone(),
                lifecycle: DurableJobLifecycle::DeadLettered,
                next_action: "expose_dead_letter_blocker".to_string(),
                claimed_by: None,
                retry_after_seconds: None,
                blocker: Some(DurableJobBlocker {
                    code: DEAD_LETTER_BLOCKER_CODE.to_string(),
                    repair_action: format!(
                        "Inspect outbox `{}` failure `{reason}`, repair the external effect target, then requeue from redb outbox evidence.",
                        snapshot.outbox_id.0
                    ),
                }),
                trace,
            }
        }
        JournalOutboxState::Failed { .. } => DurableJobPlan {
            job_id,
            outbox_id: snapshot.outbox_id.clone(),
            effect_id: snapshot.effect_id.clone(),
            lifecycle: DurableJobLifecycle::Retryable,
            next_action: "schedule_retry_from_redb_outbox".to_string(),
            claimed_by: None,
            retry_after_seconds: Some(backoff_seconds(snapshot.attempt_count, policy)),
            blocker: None,
            trace,
        },
        JournalOutboxState::Succeeded => DurableJobPlan {
            job_id,
            outbox_id: snapshot.outbox_id.clone(),
            effect_id: snapshot.effect_id.clone(),
            lifecycle: DurableJobLifecycle::Succeeded,
            next_action: "none".to_string(),
            claimed_by: None,
            retry_after_seconds: None,
            blocker: None,
            trace,
        },
    }
}

pub fn job_status_payload(plan: &DurableJobPlan) -> serde_json::Value {
    serde_json::json!({
        "job_id": plan.job_id.0,
        "status": format!("{:?}", plan.lifecycle).to_ascii_lowercase(),
        "outbox_id": plan.outbox_id.0,
        "effect_id": plan.effect_id.0,
        "next_action": plan.next_action,
        "retry_after_seconds": plan.retry_after_seconds,
        "blocker": plan.blocker,
        "trace": plan.trace,
        "authority": "redb_outbox",
        "runner": "effectum",
    })
}

pub fn host_bridge_request_job_status_payload(plan: &DurableJobPlan) -> serde_json::Value {
    let mut payload = job_status_payload(plan);
    payload["authority"] = serde_json::json!("host_bridge_request");
    payload["runner"] = serde_json::json!("parent_host_adapter");
    payload["job_type"] = serde_json::json!(HOST_BRIDGE_ADAPTER_REQUEST_WORKER);
    payload
}

fn recovery_behavior(value: &str) -> Result<JobRecoveryBehavior, RuntimeLocalJobError> {
    match value {
        "fail_and_retry_immediately" => Ok(JobRecoveryBehavior::FailAndRetryImmediately),
        "fail_and_retry_with_backoff" => Ok(JobRecoveryBehavior::FailAndRetryWithBackoff),
        other => Err(RuntimeLocalJobError::UnsupportedEffectumRecoveryBehavior {
            behavior: other.to_string(),
        }),
    }
}

fn deterministic_effectum_job_id(job_id: &DurableJobId) -> uuid::Uuid {
    let mut bytes = stable_128bit_hash(job_id.0.as_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn effectum_job_for_plan(
    plan: &DurableJobPlan,
    policy: &RetryPolicy,
    effectum_job_id: uuid::Uuid,
) -> Result<Job, RuntimeLocalJobError> {
    let payload = EffectumOutboxJobPayload {
        job_id: plan.job_id.0.clone(),
        outbox_id: plan.outbox_id.0.clone(),
        effect_id: plan.effect_id.0.clone(),
        authority: "redb_outbox".to_string(),
        runner: "effectum".to_string(),
        next_action: plan.next_action.clone(),
        claimed_by: plan.claimed_by.clone(),
        trace: plan.trace.clone(),
    };
    let mut job = Job::builder(EFFECTUM_OUTBOX_WORKER)
        .name(&plan.job_id.0)
        .json_payload(&payload)
        .map_err(|error| RuntimeLocalJobError::SerializeEffectumJobPayload {
            job_id: plan.job_id.0.clone(),
            detail: error.to_string(),
        })?
        .max_retries(policy.max_attempts.try_into().unwrap_or(u32::MAX))
        .backoff_initial_interval(Duration::from_secs(policy.base_backoff_seconds))
        .build();
    job.id = effectum_job_id;
    Ok(job)
}

fn effectum_host_bridge_job_for_plan(
    snapshot: &HostBridgeRequestJobSnapshot,
    plan: &DurableJobPlan,
    policy: &RetryPolicy,
    effectum_job_id: uuid::Uuid,
) -> Result<Job, RuntimeLocalJobError> {
    let payload = HostBridgeRequestJobPayload {
        job_id: plan.job_id.0.clone(),
        request_id: snapshot.request_id.clone(),
        run_id: snapshot.run_id.clone(),
        authority: "host_bridge_request".to_string(),
        runner: "parent_host_adapter".to_string(),
        next_action: plan.next_action.clone(),
        trace: plan.trace.clone(),
    };
    let mut job = Job::builder(HOST_BRIDGE_ADAPTER_REQUEST_WORKER)
        .name(&plan.job_id.0)
        .json_payload(&payload)
        .map_err(|error| RuntimeLocalJobError::SerializeEffectumJobPayload {
            job_id: plan.job_id.0.clone(),
            detail: error.to_string(),
        })?
        .max_retries(policy.max_attempts.try_into().unwrap_or(u32::MAX))
        .backoff_initial_interval(Duration::from_secs(policy.base_backoff_seconds))
        .build();
    job.id = effectum_job_id;
    Ok(job)
}

fn enqueue_receipt(
    config: &EffectumQueueConfig,
    plan: &DurableJobPlan,
    effectum_job_id: uuid::Uuid,
    duplicate: bool,
) -> EffectumEnqueueReceipt {
    let mut trace = plan.trace.clone();
    trace.push(trace_entry("effectum_job_id", &effectum_job_id.to_string()));
    trace.push(trace_entry(
        "effectum_enqueue",
        if duplicate { "duplicate" } else { "created" },
    ));
    EffectumEnqueueReceipt {
        job_id: plan.job_id.clone(),
        effectum_job_id: effectum_job_id.to_string(),
        outbox_id: plan.outbox_id.clone(),
        effect_id: plan.effect_id.clone(),
        duplicate,
        queue_path: config.sqlite_path.clone(),
        trace,
    }
}

fn host_bridge_enqueue_receipt(
    config: &EffectumQueueConfig,
    snapshot: &HostBridgeRequestJobSnapshot,
    plan: &DurableJobPlan,
    effectum_job_id: uuid::Uuid,
    duplicate: bool,
) -> HostBridgeRequestEnqueueReceipt {
    let mut trace = plan.trace.clone();
    trace.push(trace_entry("effectum_job_id", &effectum_job_id.to_string()));
    trace.push(trace_entry(
        "effectum_enqueue",
        if duplicate { "duplicate" } else { "created" },
    ));
    HostBridgeRequestEnqueueReceipt {
        job_id: plan.job_id.clone(),
        effectum_job_id: effectum_job_id.to_string(),
        request_id: snapshot.request_id.clone(),
        duplicate,
        queue_path: config.sqlite_path.clone(),
        trace,
    }
}

fn stable_128bit_hash(input: &[u8]) -> [u8; 16] {
    fn fnv(seed: u64, input: &[u8]) -> u64 {
        input.iter().fold(seed, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
    }
    let first = fnv(0xcbf29ce484222325, input);
    let second = fnv(0x84222325cbf29ce4, input);
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&first.to_be_bytes());
    bytes[8..].copy_from_slice(&second.to_be_bytes());
    bytes
}

fn backoff_seconds(attempt_count: u64, policy: &RetryPolicy) -> u64 {
    policy.retry_after_seconds(attempt_count)
}

fn trace_entry(kind: &str, detail: &str) -> DurableJobTraceEntry {
    DurableJobTraceEntry {
        kind: kind.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taskflow_contracts::{
        VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent,
        VidaIdempotencyKey, VidaOperation, VidaSchemaId, VidaSchemaVersion, VidaStreamRef,
        VidaStreamVersion, VidaTimestamp,
    };
    use taskflow_state::{JournalAppendRequest, OperationalJournal};
    use taskflow_state_redb::RedbOperationalJournal;
    use tempfile::tempdir;

    #[test]
    fn effectum_queue_config_records_project_local_recovery_behavior() {
        let config = EffectumQueueConfig::new(".vida/data/effectum/jobs.sqlite");
        let registry = EffectumWorkerRegistry::outbox_commands("vida.effect.ack");

        assert_eq!(
            config.sqlite_path,
            PathBuf::from(".vida/data/effectum/jobs.sqlite")
        );
        assert_eq!(config.recovery_behavior, "fail_and_retry_immediately");
        assert_eq!(registry.workers[0].job_kind, EFFECTUM_OUTBOX_WORKER);
        assert_eq!(registry.workers[0].command_operation, "vida.effect.ack");
    }

    #[test]
    fn retry_backoff_policy_covers_exponential_linear_and_bounded_windows() {
        let effectum = RetryPolicy::default();
        assert_eq!(effectum.retry_after_seconds(1), 30);
        assert_eq!(effectum.retry_after_seconds(3), 120);

        let worker = RetryBackoffPolicy::linear_seconds(3, 15);
        assert_eq!(worker.retry_delay_seconds(1), 15);
        assert_eq!(worker.retry_delay_seconds(3), 45);

        let read_only = RetryBackoffPolicy::linear_attempts(800, 25);
        assert_eq!(read_only.max_attempts_usize(), 800);
        assert_eq!(read_only.base_delay_millis(), 25);

        let state_open = RetryBackoffPolicy::linear_millis(30_000, 25);
        assert_eq!(state_open.max_attempts_usize(), 1_200);
        assert_eq!(state_open.base_delay_millis(), 25);
    }

    #[test]
    fn deterministic_effect_job_ids_deduplicate_enqueue_attempts() {
        let effect_id = VidaEffectRef("effect:send/email".to_string());

        assert_eq!(
            DurableJobId::from_effect_id(&effect_id),
            DurableJobId::from_effect_id(&effect_id)
        );
        assert_eq!(
            DurableJobId::from_effect_id(&effect_id).0,
            "vida-effect-effect-send-email"
        );
    }

    #[tokio::test]
    async fn effectum_queue_opens_and_enqueue_is_idempotent_for_outbox_effect() {
        let dir = tempdir().unwrap();
        let config = EffectumQueueConfig::new(dir.path().join("jobs.sqlite"));
        let queue = open_effectum_queue(&config).await.unwrap();
        let policy = RetryPolicy::default();
        let snapshot = OutboxJobSnapshot {
            outbox_id: VidaEventRef("outbox-effect-1".to_string()),
            effect_id: VidaEffectRef("effect-email-1".to_string()),
            state: JournalOutboxState::Pending,
            attempt_count: 0,
            source_event_cursor: Some(VidaEventCursor("cursor-1".to_string())),
            failure_reason: None,
        };
        let plan = plan_outbox_job(&snapshot, &policy);

        let first = enqueue_outbox_job_idempotently(&queue, &config, &plan, &policy)
            .await
            .unwrap();
        let second = enqueue_outbox_job_idempotently(&queue, &config, &plan, &policy)
            .await
            .unwrap();
        let effectum_job_id = uuid::Uuid::parse_str(&first.effectum_job_id).unwrap();
        let status = queue.get_job_status(effectum_job_id).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&status.payload).unwrap();

        assert!(!first.duplicate);
        assert!(second.duplicate);
        assert_eq!(first.effectum_job_id, second.effectum_job_id);
        assert_eq!(first.queue_path, config.sqlite_path);
        assert_eq!(status.state, effectum::JobState::Pending);
        assert_eq!(status.name.as_deref(), Some(plan.job_id.0.as_str()));
        assert_eq!(status.job_type, EFFECTUM_OUTBOX_WORKER);
        assert_eq!(payload["authority"], "redb_outbox");
        assert_eq!(payload["runner"], "effectum");
        assert_eq!(payload["outbox_id"], snapshot.outbox_id.0);
        assert_eq!(payload["effect_id"], snapshot.effect_id.0);
        assert!(
            first
                .trace
                .iter()
                .any(|entry| entry.kind == "effectum_enqueue" && entry.detail == "created")
        );
        assert!(
            second
                .trace
                .iter()
                .any(|entry| entry.kind == "effectum_enqueue" && entry.detail == "duplicate")
        );
    }

    #[tokio::test]
    async fn effectum_worker_refuses_to_acknowledge_metadata_only_job() {
        let dir = tempdir().unwrap();
        let journal_path = dir.path().join("journal.redb");
        let config = EffectumQueueConfig::new(dir.path().join("jobs.sqlite"));
        let mut journal = RedbOperationalJournal::create(&journal_path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append effect");
        let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
        let outbox_id = claimed[0].outbox_id.clone();
        let record = journal
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");
        let plan = plan_outbox_job(&OutboxJobSnapshot::from(&record), &RetryPolicy::default());
        drop(journal);

        let queue = open_effectum_queue(&config).await.unwrap();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let pipeline = EffectumWorkerPipeline::new("vida.completion.record", sender);
        let worker = effectum::Worker::builder(&queue, pipeline)
            .jobs([effectum_outbox_job_runner()])
            .build()
            .await
            .unwrap();
        enqueue_outbox_job_idempotently(&queue, &config, &plan, &RetryPolicy::default())
            .await
            .unwrap();

        let command = tokio::time::timeout(Duration::from_millis(250), receiver.recv()).await;
        worker
            .unregister(Some(Duration::from_secs(5)))
            .await
            .unwrap();

        let reopened = RedbOperationalJournal::open(&journal_path).expect("reopen journal");
        let record = reopened
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");

        assert!(command.is_err(), "metadata-only job must not submit ack");
        assert_eq!(
            record.state,
            JournalOutboxState::Claimed {
                consumer_id: "effectum-worker-1".to_string()
            }
        );
    }

    #[test]
    fn redb_writeback_rejects_forged_ack_that_does_not_match_persisted_outbox() {
        let dir = tempdir().expect("tempdir");
        let journal_path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&journal_path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append effect");
        let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
        let outbox_id = claimed[0].outbox_id.clone();
        drop(journal);

        let forged = WorkerCommandSubmission {
            operation: VidaOperation("attacker.operation.not.vida.completion.record".to_string()),
            command_id: VidaCommandRef(format!("job-ack:{}", outbox_id.0)),
            idempotency_key: VidaIdempotencyKey(format!("job-ack:{}", outbox_id.0)),
            stream_id: VidaStreamRef(format!("job-trace:{}", outbox_id.0)),
            payload: serde_json::json!({
                "outbox_id": outbox_id.0,
                "effect_id": "wrong-effect-id",
                "runner": "effectum",
                "authority": "redb_outbox",
                "operation": "attacker.operation.not.vida.completion.record",
                "command_id": format!("job-ack:{}", outbox_id.0),
                "idempotency_key": format!("job-ack:{}", outbox_id.0),
                "claimed_by": "effectum-worker-1",
                "outcome": { "status": "succeeded" },
            }),
        };

        let error = apply_worker_command_to_redb(&journal_path, &forged)
            .expect_err("forged ack must fail closed");

        assert!(
            matches!(
                error,
                RuntimeLocalJobError::WorkerAckEffectMismatch {
                    ref effect_id,
                    ref persisted_effect_id
                } if effect_id == "wrong-effect-id" && persisted_effect_id == "effect-1"
            ),
            "unexpected error variant: {error:?}"
        );
        let reopened = RedbOperationalJournal::open(&journal_path).expect("reopen journal");
        let record = reopened
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");
        assert_eq!(
            record.state,
            JournalOutboxState::Claimed {
                consumer_id: "effectum-worker-1".to_string()
            }
        );
    }

    #[test]
    fn worker_ack_missing_outcome_reports_typed_payload_variant() {
        let command = WorkerCommandSubmission {
            operation: VidaOperation("vida.completion.record".to_string()),
            command_id: VidaCommandRef("job-ack:outbox-1".to_string()),
            idempotency_key: VidaIdempotencyKey("job-ack:outbox-1".to_string()),
            stream_id: VidaStreamRef("job-trace:outbox-1".to_string()),
            payload: serde_json::json!({
                "outbox_id": "outbox-1",
                "effect_id": "effect-1",
                "runner": "effectum",
                "authority": "redb_outbox",
                "operation": "vida.completion.record",
                "command_id": "job-ack:outbox-1",
                "idempotency_key": "job-ack:outbox-1",
                "claimed_by": "effectum-worker-1",
            }),
        };

        let error = apply_worker_command_to_redb(Path::new("unused.redb"), &command)
            .expect_err("missing outcome must fail before journal open");

        assert!(
            matches!(
                error,
                RuntimeLocalJobError::MissingWorkerPayloadField { ref field } if field == "outcome.status"
            ),
            "unexpected error variant: {error:?}"
        );
    }

    #[tokio::test]
    async fn enqueue_terminal_job_reports_typed_variant() {
        let dir = tempdir().unwrap();
        let config = EffectumQueueConfig::new(dir.path().join("jobs.sqlite"));
        let queue = open_effectum_queue(&config).await.unwrap();
        let plan = plan_outbox_job(
            &OutboxJobSnapshot {
                outbox_id: VidaEventRef("outbox-succeeded-1".to_string()),
                effect_id: VidaEffectRef("effect-succeeded-1".to_string()),
                state: JournalOutboxState::Succeeded,
                attempt_count: 1,
                source_event_cursor: None,
                failure_reason: None,
            },
            &RetryPolicy::default(),
        );

        let error =
            enqueue_outbox_job_idempotently(&queue, &config, &plan, &RetryPolicy::default())
                .await
                .expect_err("terminal job must not enqueue");

        assert!(
            matches!(
                error,
                RuntimeLocalJobError::EnqueueTerminalDurableJob { ref job_id, ref lifecycle }
                    if job_id == "vida-effect-effect-succeeded-1"
                        && *lifecycle == DurableJobLifecycle::Succeeded
            ),
            "unexpected error variant: {error:?}"
        );
    }

    #[test]
    fn redb_writeback_accepts_ack_matching_persisted_claim_and_effect() {
        let dir = tempdir().expect("tempdir");
        let journal_path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&journal_path).expect("create journal");
        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append effect");
        let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
        let outbox_id = claimed[0].outbox_id.clone();
        let record = journal
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");
        let command = EffectumOutboxWorker::new("vida.completion.record").acknowledgement_command(
            &OutboxJobSnapshot::from(&record),
            EffectAckOutcome::Succeeded,
        );
        drop(journal);

        apply_worker_command_to_redb(&journal_path, &command).expect("apply matching ack");

        let reopened = RedbOperationalJournal::open(&journal_path).expect("reopen journal");
        let record = reopened
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");
        assert_eq!(record.state, JournalOutboxState::Succeeded);
    }

    #[tokio::test]
    async fn reopened_effectum_queue_preserves_pending_outbox_job_for_restart_resume() {
        let dir = tempdir().unwrap();
        let config = EffectumQueueConfig::new(dir.path().join("jobs.sqlite"));
        let policy = RetryPolicy::default();
        let snapshot = OutboxJobSnapshot {
            outbox_id: VidaEventRef("outbox-restart-1".to_string()),
            effect_id: VidaEffectRef("effect-restart-1".to_string()),
            state: JournalOutboxState::Pending,
            attempt_count: 0,
            source_event_cursor: None,
            failure_reason: None,
        };
        let plan = plan_outbox_job(&snapshot, &policy);
        {
            let queue = open_effectum_queue(&config).await.unwrap();
            enqueue_outbox_job_idempotently(&queue, &config, &plan, &policy)
                .await
                .unwrap();
        }

        let reopened = open_effectum_queue(&config).await.unwrap();
        let status = reopened
            .get_job_status(deterministic_effectum_job_id(&plan.job_id))
            .await
            .unwrap();
        let duplicate = enqueue_outbox_job_idempotently(&reopened, &config, &plan, &policy)
            .await
            .unwrap();

        assert_eq!(status.state, effectum::JobState::Pending);
        assert_eq!(status.name.as_deref(), Some(plan.job_id.0.as_str()));
        assert!(duplicate.duplicate);
    }

    #[test]
    fn dead_letter_plan_exposes_blocker_and_repair_action() {
        let snapshot = OutboxJobSnapshot {
            outbox_id: VidaEventRef("outbox-1".to_string()),
            effect_id: VidaEffectRef("effect-1".to_string()),
            state: JournalOutboxState::Failed {
                reason: "transport failure".to_string(),
            },
            attempt_count: 3,
            source_event_cursor: Some(VidaEventCursor("global-1".to_string())),
            failure_reason: Some("transport failure".to_string()),
        };

        let plan = plan_outbox_job(&snapshot, &RetryPolicy::default());

        assert_eq!(plan.lifecycle, DurableJobLifecycle::DeadLettered);
        assert_eq!(
            plan.blocker.as_ref().map(|blocker| blocker.code.as_str()),
            Some(DEAD_LETTER_BLOCKER_CODE)
        );
        assert!(
            plan.blocker
                .as_ref()
                .expect("blocker")
                .repair_action
                .contains("requeue from redb outbox evidence")
        );
    }

    #[test]
    fn host_bridge_request_job_is_keyed_by_request_id() {
        let request = serde_json::json!({
            "request_id": "req/1",
            "run_id": "run-1",
            "status": "pending",
            "attempt_count": 0
        });
        let snapshot =
            HostBridgeRequestJobSnapshot::from_request(&request).expect("request snapshot");
        let first = plan_host_bridge_request_job(&snapshot, &RetryPolicy::default());
        let second = plan_host_bridge_request_job(&snapshot, &RetryPolicy::default());

        assert_eq!(first.job_id, second.job_id);
        assert_eq!(first.job_id.0, "host-bridge-request-req-1");
        assert_eq!(first.lifecycle, DurableJobLifecycle::Pending);
        assert_eq!(
            first.next_action,
            "enqueue_host_bridge_adapter_request_idempotently"
        );
    }

    #[tokio::test]
    async fn host_bridge_request_enqueue_is_idempotent_by_request_id() {
        let dir = tempdir().unwrap();
        let config = EffectumQueueConfig::new(dir.path().join("host-bridge-jobs.sqlite"));
        let queue = open_effectum_queue(&config).await.unwrap();
        let request = serde_json::json!({
            "request_id": "req-idempotent",
            "run_id": "run-1",
            "status": "pending",
            "attempt_count": 0
        });
        let snapshot =
            HostBridgeRequestJobSnapshot::from_request(&request).expect("request snapshot");

        let first = enqueue_host_bridge_request_job_idempotently(
            &queue,
            &config,
            &snapshot,
            &RetryPolicy::default(),
        )
        .await
        .unwrap();
        let second = enqueue_host_bridge_request_job_idempotently(
            &queue,
            &config,
            &snapshot,
            &RetryPolicy::default(),
        )
        .await
        .unwrap();

        assert!(!first.duplicate);
        assert!(second.duplicate);
        assert_eq!(first.effectum_job_id, second.effectum_job_id);
        assert_eq!(first.request_id, "req-idempotent");
        assert!(
            second
                .trace
                .iter()
                .any(|entry| entry.kind == "effectum_enqueue" && entry.detail == "duplicate")
        );
    }

    #[tokio::test]
    async fn reopened_effectum_queue_preserves_host_bridge_request_job_for_restart_replay() {
        let dir = tempdir().unwrap();
        let config = EffectumQueueConfig::new(dir.path().join("host-bridge-restart.sqlite"));
        let request = serde_json::json!({
            "request_id": "req-restart",
            "run_id": "run-1",
            "status": "pending",
            "attempt_count": 0
        });
        let snapshot =
            HostBridgeRequestJobSnapshot::from_request(&request).expect("request snapshot");
        let first_job_id = {
            let queue = open_effectum_queue(&config).await.unwrap();
            enqueue_host_bridge_request_job_idempotently(
                &queue,
                &config,
                &snapshot,
                &RetryPolicy::default(),
            )
            .await
            .unwrap()
            .effectum_job_id
        };

        let reopened = open_effectum_queue(&config).await.unwrap();
        let duplicate = enqueue_host_bridge_request_job_idempotently(
            &reopened,
            &config,
            &snapshot,
            &RetryPolicy::default(),
        )
        .await
        .unwrap();
        let status = reopened
            .get_job_status(uuid::Uuid::parse_str(&first_job_id).unwrap())
            .await
            .unwrap();

        assert!(duplicate.duplicate);
        assert_eq!(duplicate.effectum_job_id, first_job_id);
        assert_eq!(status.state, effectum::JobState::Pending);
        assert_eq!(status.job_type, HOST_BRIDGE_ADAPTER_REQUEST_WORKER);
    }

    #[test]
    fn host_bridge_request_job_recovers_blocked_request_as_retryable() {
        let request = serde_json::json!({
            "request_id": "req-retry",
            "run_id": "run-1",
            "status": "blocked",
            "attempt_count": 1,
            "failure_reason": "parent host capacity unavailable"
        });
        let snapshot =
            HostBridgeRequestJobSnapshot::from_request(&request).expect("request snapshot");
        let plan = plan_host_bridge_request_job(
            &snapshot,
            &RetryPolicy {
                max_attempts: 3,
                base_backoff_seconds: 10,
            },
        );

        assert_eq!(plan.lifecycle, DurableJobLifecycle::Retryable);
        assert_eq!(plan.retry_after_seconds, Some(10));
        assert_eq!(plan.next_action, "retry_host_bridge_adapter_request");
        assert!(plan.blocker.is_none());
    }

    #[test]
    fn host_bridge_request_job_dead_letters_after_retry_exhaustion() {
        let request = serde_json::json!({
            "request_id": "req-dead",
            "status": "failed",
            "attempt_count": 3,
            "failure_reason": "adapter result missing"
        });
        let snapshot =
            HostBridgeRequestJobSnapshot::from_request(&request).expect("request snapshot");
        let plan = plan_host_bridge_request_job(&snapshot, &RetryPolicy::default());
        let payload = host_bridge_request_job_status_payload(&plan);

        assert_eq!(plan.lifecycle, DurableJobLifecycle::DeadLettered);
        assert_eq!(
            plan.blocker.as_ref().map(|blocker| blocker.code.as_str()),
            Some(HOST_BRIDGE_DEAD_LETTER_BLOCKER_CODE)
        );
        assert_eq!(payload["authority"], "host_bridge_request");
        assert_eq!(payload["runner"], "parent_host_adapter");
        assert_eq!(payload["job_type"], HOST_BRIDGE_ADAPTER_REQUEST_WORKER);
    }

    #[test]
    fn claimed_outbox_recovers_after_restart_as_retryable_effectum_job() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append effect intent");
        let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
        let outbox_id = claimed[0].outbox_id.clone();
        drop(journal);

        let reopened = RedbOperationalJournal::open(&path).expect("reopen journal");
        let record = reopened
            .outbox_effect_record(&outbox_id)
            .expect("read outbox")
            .expect("outbox record");
        let plan = plan_outbox_job(&OutboxJobSnapshot::from(&record), &RetryPolicy::default());

        assert_eq!(plan.job_id.0, "vida-effect-effect-1");
        assert_eq!(plan.lifecycle, DurableJobLifecycle::Retryable);
        assert_eq!(plan.next_action, "recover_claimed_job_after_restart");
        assert_eq!(plan.retry_after_seconds, Some(30));
        assert!(
            plan.trace
                .iter()
                .any(|entry| { entry.kind == "authority" && entry.detail == "redb_outbox" })
        );
    }

    #[test]
    fn worker_registry_exposes_commands_without_store_authority() {
        let registry = EffectumWorkerRegistry::outbox_commands("vida.completion.record");

        assert_eq!(registry.workers.len(), 1);
        assert_eq!(registry.workers[0].job_kind, EFFECTUM_OUTBOX_WORKER);
        assert_eq!(
            registry.workers[0].command_operation,
            "vida.completion.record"
        );
    }

    #[test]
    fn effectum_worker_boundary_submits_pipeline_command_not_store_mutation() {
        let worker = EffectumOutboxWorker::new("vida.completion.record");
        let snapshot = OutboxJobSnapshot {
            outbox_id: VidaEventRef("outbox-1".to_string()),
            effect_id: VidaEffectRef("effect-1".to_string()),
            state: JournalOutboxState::Claimed {
                consumer_id: "effectum-worker-1".to_string(),
            },
            attempt_count: 1,
            source_event_cursor: Some(VidaEventCursor("global-1".to_string())),
            failure_reason: None,
        };

        let command = worker.acknowledgement_command(&snapshot, EffectAckOutcome::Succeeded);

        assert_eq!(command.operation.0, "vida.completion.record");
        assert_eq!(command.idempotency_key.0, "job-ack:outbox-1");
        assert_eq!(command.payload["authority"], "redb_outbox");
        assert_eq!(command.payload["runner"], "effectum");
        assert_eq!(command.payload["outcome"]["status"], "succeeded");
    }

    #[test]
    fn persisted_redb_outbox_records_drive_job_status_variants() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("journal.redb");
        let mut journal = RedbOperationalJournal::create(&path).expect("create journal");

        journal
            .append(append_request(0, vec![event(1)], vec![effect("effect-1")]))
            .expect("append first effect");
        let first = journal.claim_outbox_batch("effectum-worker-1", 1);
        journal
            .mark_outbox_failed(&first[0].outbox_id, "first failure".to_string())
            .expect("mark first failed");
        journal
            .schedule_outbox_retry(
                &first[0].outbox_id,
                Some("2026-06-23T01:00:00Z".to_string()),
            )
            .expect("schedule first retry");
        let first_retry = journal.claim_outbox_batch("effectum-worker-2", 1);
        journal
            .mark_outbox_failed(&first_retry[0].outbox_id, "second failure".to_string())
            .expect("mark second failed");

        let mut second = append_request(1, vec![event(2)], vec![effect("effect-2")]);
        second.idempotency_key = VidaIdempotencyKey("idem-2".to_string());
        journal.append(second).expect("append second effect");
        let second_claim = journal.claim_outbox_batch("effectum-worker-1", 1);
        journal
            .mark_outbox_succeeded(&second_claim[0].outbox_id)
            .expect("mark second succeeded");
        drop(journal);

        let retryable = plan_outbox_job_from_redb(
            &path,
            "vida-effect-effect-1",
            &RetryPolicy {
                max_attempts: 3,
                base_backoff_seconds: 15,
            },
        )
        .expect("read persisted outbox")
        .expect("retryable job");
        assert_eq!(retryable.lifecycle, DurableJobLifecycle::Retryable);
        assert_eq!(retryable.retry_after_seconds, Some(30));

        let dead_letter = plan_outbox_job_from_redb(
            &path,
            "vida-effect-effect-1",
            &RetryPolicy {
                max_attempts: 2,
                base_backoff_seconds: 15,
            },
        )
        .expect("read persisted outbox")
        .expect("dead-letter job");
        assert_eq!(dead_letter.lifecycle, DurableJobLifecycle::DeadLettered);
        assert_eq!(
            dead_letter
                .blocker
                .as_ref()
                .map(|blocker| blocker.code.as_str()),
            Some(DEAD_LETTER_BLOCKER_CODE)
        );

        let succeeded = plan_outbox_job_from_redb(&path, "effect-2", &RetryPolicy::default())
            .expect("read persisted outbox")
            .expect("succeeded job");
        assert_eq!(succeeded.lifecycle, DurableJobLifecycle::Succeeded);

        assert!(
            plan_outbox_job_from_redb(&path, "missing", &RetryPolicy::default())
                .expect("read persisted outbox")
                .is_none()
        );
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
            schema_id: VidaSchemaId("schema.task.updated".to_string()),
            event_version: VidaSchemaVersion(1),
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
}
