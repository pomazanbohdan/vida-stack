use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SurrealValue, PartialEq, Eq)]
pub struct TaskAttemptRecord {
    pub attempt_id: String,
    pub task_id: String,
    pub stage_id: String,
    pub backend: String,
    pub model_profile: String,
    pub isolation: String,
    pub freshness: String,
    pub status: String,
    pub artifact_refs: Vec<String>,
    pub consolidation_receipt_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SurrealValue, PartialEq, Eq)]
pub struct TaskStageRecord {
    pub stage_record_id: String,
    pub task_id: String,
    pub stage_id: String,
    pub status: String,
    pub latest_consolidation_receipt_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RecordTaskAttemptRequest {
    pub attempt_id: Option<String>,
    pub task_id: String,
    pub stage_id: String,
    pub backend: String,
    pub model_profile: String,
    pub isolation: String,
    pub freshness: Option<String>,
    pub status: String,
    pub artifact_refs: Vec<String>,
    pub consolidation_receipt_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransitionTaskAttemptRequest {
    pub attempt_id: String,
    pub task_id: String,
    pub stage_id: String,
    pub status: String,
    pub artifact_refs: Vec<String>,
    pub consolidation_receipt_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TaskStageSummary {
    pub task_id: String,
    pub stage_id: String,
    pub stage_status: Option<String>,
    pub attempt_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub latest_attempt_id: Option<String>,
    pub latest_attempt_status: Option<String>,
    pub latest_consolidation_receipt_id: Option<String>,
    pub artifact_refs: Vec<String>,
}

const TASK_ATTEMPT_STATUSES: &[&str] = &[
    "submitted",
    "running",
    "produced",
    "validating",
    "accepted",
    "partially_accepted",
    "rejected",
    "stale",
    "failed",
    "consumed",
];

impl StateStore {
    pub async fn record_task_attempt(
        &self,
        request: RecordTaskAttemptRequest,
    ) -> Result<TaskAttemptRecord, StateStoreError> {
        let task = self
            .validate_task_attempt_binding(&request.task_id, &request.stage_id)
            .await?;
        let status = normalize_task_attempt_status(&request.status)?;
        let attempt_id = request.attempt_id.unwrap_or_else(|| {
            format!(
                "{}--{}--{}",
                sanitize_record_id(&request.task_id),
                sanitize_record_id(&request.stage_id),
                unix_timestamp_nanos()
            )
        });
        let now = task_attempt_timestamp();
        let record = TaskAttemptRecord {
            attempt_id: normalize_non_empty("attempt_id", &attempt_id)?,
            task_id: normalize_non_empty("task_id", &request.task_id)?,
            stage_id: normalize_non_empty("stage_id", &request.stage_id)?,
            backend: normalize_non_empty("backend", &request.backend)?,
            model_profile: normalize_non_empty("model_profile", &request.model_profile)?,
            isolation: normalize_non_empty("isolation", &request.isolation)?,
            freshness: request
                .freshness
                .map(|value| normalize_non_empty("freshness", &value))
                .transpose()?
                .unwrap_or(task.updated_at),
            status,
            artifact_refs: normalize_artifact_refs(request.artifact_refs),
            consolidation_receipt_id: normalize_optional(request.consolidation_receipt_id),
            created_at: now.clone(),
            updated_at: now,
        };

        let _: Option<TaskAttemptRecord> = self
            .db
            .upsert(("task_attempt", record.attempt_id.as_str()))
            .content(record.clone())
            .await?;
        self.upsert_task_stage_from_attempt(&record).await?;
        Ok(record)
    }

    pub async fn transition_task_attempt(
        &self,
        request: TransitionTaskAttemptRequest,
    ) -> Result<TaskAttemptRecord, StateStoreError> {
        let task = self
            .validate_task_attempt_binding(&request.task_id, &request.stage_id)
            .await?;
        let mut attempt = self.task_attempt(&request.attempt_id).await?;
        if attempt.task_id != request.task_id.trim() || attempt.stage_id != request.stage_id.trim()
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "attempt `{}` is bound to task `{}` stage `{}`, not task `{}` stage `{}`",
                    attempt.attempt_id,
                    attempt.task_id,
                    attempt.stage_id,
                    request.task_id.trim(),
                    request.stage_id.trim()
                ),
            });
        }
        if attempt.freshness != task.updated_at {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "stale_task_binding: attempt `{}` freshness `{}` does not match task `{}` updated_at `{}`",
                    attempt.attempt_id, attempt.freshness, task.id, task.updated_at
                ),
            });
        }
        attempt.status = normalize_task_attempt_status(&request.status)?;
        attempt.updated_at = task_attempt_timestamp();
        for artifact_ref in normalize_artifact_refs(request.artifact_refs) {
            if !attempt.artifact_refs.contains(&artifact_ref) {
                attempt.artifact_refs.push(artifact_ref);
            }
        }
        if let Some(receipt_id) = normalize_optional(request.consolidation_receipt_id) {
            attempt.consolidation_receipt_id = Some(receipt_id);
        }
        let _: Option<TaskAttemptRecord> = self
            .db
            .upsert(("task_attempt", attempt.attempt_id.as_str()))
            .content(attempt.clone())
            .await?;
        self.upsert_task_stage_from_attempt(&attempt).await?;
        Ok(attempt)
    }

    pub async fn task_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<TaskAttemptRecord, StateStoreError> {
        let attempt_id = normalize_non_empty("attempt_id", attempt_id)?;
        let attempt: Option<TaskAttemptRecord> = self
            .db
            .select(("task_attempt", attempt_id.as_str()))
            .await?;
        attempt.ok_or_else(|| StateStoreError::InvalidTaskRecord {
            reason: format!("task attempt is missing: {attempt_id}"),
        })
    }

    pub async fn task_stage_summary(
        &self,
        task_id: &str,
        stage_id: &str,
    ) -> Result<TaskStageSummary, StateStoreError> {
        self.validate_task_attempt_binding(task_id, stage_id)
            .await?;
        let task_id = normalize_non_empty("task_id", task_id)?;
        let stage_id = normalize_non_empty("stage_id", stage_id)?;
        let mut response = self
            .db
            .query(format!(
                "SELECT * FROM task_attempt WHERE task_id = '{}' AND stage_id = '{}' ORDER BY updated_at ASC;",
                escape_surql_literal(&task_id),
                escape_surql_literal(&stage_id)
            ))
            .await?;
        let attempts: Vec<TaskAttemptRecord> = response.take(0)?;
        let stage = self.task_stage_record(&task_id, &stage_id).await?;
        Ok(task_stage_summary_from_attempts(
            task_id,
            stage_id,
            stage.as_ref(),
            &attempts,
        ))
    }

    async fn task_stage_record(
        &self,
        task_id: &str,
        stage_id: &str,
    ) -> Result<Option<TaskStageRecord>, StateStoreError> {
        let record_id = task_stage_record_id(task_id, stage_id)?;
        let stage: Option<TaskStageRecord> =
            self.db.select(("task_stage", record_id.as_str())).await?;
        Ok(stage)
    }

    async fn upsert_task_stage_from_attempt(
        &self,
        attempt: &TaskAttemptRecord,
    ) -> Result<TaskStageRecord, StateStoreError> {
        let record_id = task_stage_record_id(&attempt.task_id, &attempt.stage_id)?;
        let existing = self
            .task_stage_record(&attempt.task_id, &attempt.stage_id)
            .await?;
        let now = task_attempt_timestamp();
        let stage = TaskStageRecord {
            stage_record_id: record_id,
            task_id: attempt.task_id.clone(),
            stage_id: attempt.stage_id.clone(),
            status: attempt.status.clone(),
            latest_consolidation_receipt_id: attempt.consolidation_receipt_id.clone(),
            created_at: existing
                .as_ref()
                .map(|stage| stage.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        };
        let _: Option<TaskStageRecord> = self
            .db
            .upsert(("task_stage", stage.stage_record_id.as_str()))
            .content(stage.clone())
            .await?;
        Ok(stage)
    }

    async fn validate_task_attempt_binding(
        &self,
        task_id: &str,
        stage_id: &str,
    ) -> Result<TaskRecord, StateStoreError> {
        let task_id = normalize_non_empty("task_id", task_id)?;
        normalize_non_empty("stage_id", stage_id)?;
        let task = self.show_task(&task_id).await?;
        if Self::task_status_is_closed_like(&task.status) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task attempt binding is stale because task `{}` is closed",
                    task.id
                ),
            });
        }
        if work_item_is_program_container(&task.issue_type) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task attempt binding requires a leaf task, got `{}` of type `{}`",
                    task.id, task.issue_type
                ),
            });
        }
        Ok(task)
    }
}

fn task_stage_summary_from_attempts(
    task_id: String,
    stage_id: String,
    stage: Option<&TaskStageRecord>,
    attempts: &[TaskAttemptRecord],
) -> TaskStageSummary {
    let mut status_counts = BTreeMap::new();
    let mut artifact_refs = Vec::new();
    for attempt in attempts {
        *status_counts.entry(attempt.status.clone()).or_insert(0) += 1;
        for artifact_ref in &attempt.artifact_refs {
            if !artifact_refs.contains(artifact_ref) {
                artifact_refs.push(artifact_ref.clone());
            }
        }
    }
    let latest = attempts.iter().max_by(|left, right| {
        left.updated_at
            .cmp(&right.updated_at)
            .then_with(|| left.attempt_id.cmp(&right.attempt_id))
    });
    TaskStageSummary {
        task_id,
        stage_id,
        stage_status: stage.map(|stage| stage.status.clone()),
        attempt_count: attempts.len(),
        status_counts,
        latest_attempt_id: latest.map(|attempt| attempt.attempt_id.clone()),
        latest_attempt_status: latest.map(|attempt| attempt.status.clone()),
        latest_consolidation_receipt_id: stage
            .and_then(|stage| stage.latest_consolidation_receipt_id.clone())
            .or_else(|| latest.and_then(|attempt| attempt.consolidation_receipt_id.clone())),
        artifact_refs,
    }
}

fn normalize_task_attempt_status(value: &str) -> Result<String, StateStoreError> {
    let normalized = normalize_non_empty("status", value)?.to_ascii_lowercase();
    if TASK_ATTEMPT_STATUSES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "task attempt status `{}` is invalid; expected one of {}",
                value.trim(),
                TASK_ATTEMPT_STATUSES.join(", ")
            ),
        })
    }
}

fn normalize_non_empty(field: &str, value: &str) -> Result<String, StateStoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StateStoreError::InvalidTaskRecord {
            reason: format!("task attempt field `{field}` must be non-empty"),
        });
    }
    Ok(value.to_string())
}

fn normalize_artifact_refs(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() && !normalized.iter().any(|existing| existing == value) {
            normalized.push(value.to_string());
        }
    }
    normalized
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn task_stage_record_id(task_id: &str, stage_id: &str) -> Result<String, StateStoreError> {
    Ok(format!(
        "{}--{}",
        sanitize_record_id(&normalize_non_empty("task_id", task_id)?),
        sanitize_record_id(&normalize_non_empty("stage_id", stage_id)?)
    ))
}

fn task_attempt_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| unix_timestamp().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_stage_summary_reports_counts_and_latest_receipt() {
        let stage = TaskStageRecord {
            stage_record_id: "task-a--analysis".to_string(),
            task_id: "task-a".to_string(),
            stage_id: "analysis".to_string(),
            status: "accepted".to_string(),
            latest_consolidation_receipt_id: Some("receipt-b".to_string()),
            created_at: "2026-06-05T00:00:00Z".to_string(),
            updated_at: "2026-06-05T00:02:00Z".to_string(),
        };
        let attempts = vec![
            TaskAttemptRecord {
                attempt_id: "attempt-a".to_string(),
                task_id: "task-a".to_string(),
                stage_id: "analysis".to_string(),
                backend: "internal".to_string(),
                model_profile: "low".to_string(),
                isolation: "readonly".to_string(),
                freshness: "snapshot-a".to_string(),
                status: "rejected".to_string(),
                artifact_refs: vec!["artifact-a".to_string()],
                consolidation_receipt_id: None,
                created_at: "2026-06-05T00:00:00Z".to_string(),
                updated_at: "2026-06-05T00:00:00Z".to_string(),
            },
            TaskAttemptRecord {
                attempt_id: "attempt-b".to_string(),
                task_id: "task-a".to_string(),
                stage_id: "analysis".to_string(),
                backend: "vibe".to_string(),
                model_profile: "medium".to_string(),
                isolation: "readonly".to_string(),
                freshness: "snapshot-b".to_string(),
                status: "accepted".to_string(),
                artifact_refs: vec!["artifact-a".to_string(), "artifact-b".to_string()],
                consolidation_receipt_id: Some("receipt-b".to_string()),
                created_at: "2026-06-05T00:01:00Z".to_string(),
                updated_at: "2026-06-05T00:01:00Z".to_string(),
            },
        ];

        let summary = task_stage_summary_from_attempts(
            "task-a".to_string(),
            "analysis".to_string(),
            Some(&stage),
            &attempts,
        );

        assert_eq!(summary.stage_status.as_deref(), Some("accepted"));
        assert_eq!(summary.attempt_count, 2);
        assert_eq!(summary.status_counts["rejected"], 1);
        assert_eq!(summary.status_counts["accepted"], 1);
        assert_eq!(summary.latest_attempt_id.as_deref(), Some("attempt-b"));
        assert_eq!(
            summary.latest_consolidation_receipt_id.as_deref(),
            Some("receipt-b")
        );
        assert_eq!(
            summary.artifact_refs,
            vec!["artifact-a".to_string(), "artifact-b".to_string()]
        );
    }

    #[test]
    fn task_attempt_statuses_fail_closed() {
        let error = normalize_task_attempt_status("completed").expect_err("completed is legacy");
        assert!(error
            .to_string()
            .contains("expected one of submitted, running, produced"));
    }
}
