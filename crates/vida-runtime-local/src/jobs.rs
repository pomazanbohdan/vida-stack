use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taskflow_contracts::{
    VidaCommandRef, VidaEffectRef, VidaEventCursor, VidaEventRef, VidaIdempotencyKey,
    VidaOperation, VidaStreamRef,
};
use taskflow_state::JournalOutboxState;
use taskflow_state_redb::{RedbOperationalJournal, RedbOutboxEffectRecord};

pub type EffectumQueue = effectum::Queue;

pub const EFFECTUM_OUTBOX_WORKER: &str = "vida.redb.outbox.effect";
pub const DEAD_LETTER_BLOCKER_CODE: &str = "vida_job_dead_letter";

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

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_seconds: 30,
        }
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
pub struct EffectumOutboxWorker {
    pub command_operation: VidaOperation,
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
        WorkerCommandSubmission {
            operation: self.command_operation.clone(),
            command_id: VidaCommandRef(format!("job-ack:{}", snapshot.outbox_id.0)),
            idempotency_key: VidaIdempotencyKey(format!("job-ack:{}", snapshot.outbox_id.0)),
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
                "outcome": outcome_payload,
            }),
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
    pub retry_after_seconds: Option<u64>,
    pub blocker: Option<DurableJobBlocker>,
    pub trace: Vec<DurableJobTraceEntry>,
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
) -> Result<Option<DurableJobPlan>, String> {
    let journal = RedbOperationalJournal::open(journal_path).map_err(|error| {
        format!(
            "open redb outbox journal `{}`: {error}",
            journal_path.display()
        )
    })?;
    let records = journal.outbox_effect_records().map_err(|error| {
        format!(
            "read redb outbox records `{}`: {error}",
            journal_path.display()
        )
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

fn backoff_seconds(attempt_count: u64, policy: &RetryPolicy) -> u64 {
    let exponent = attempt_count.saturating_sub(1).min(8);
    policy.base_backoff_seconds.saturating_mul(1 << exponent)
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
