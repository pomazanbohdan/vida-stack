use super::*;
use taskflow_authority::task_attempts::{
    decide_task_attempt_binding, decide_task_attempt_rollup,
    normalize_task_attempt_status as normalize_authority_attempt_status,
    summarize_task_stage_attempts, TaskAttemptBindingDecision, TaskAttemptBindingInput,
    TaskAttemptRollupAttempt, TaskAttemptRollupInput, TaskAttemptSummaryInput,
};

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
    #[serde(default)]
    pub selected_model_profile_readiness_status: Option<String>,
    #[serde(default)]
    pub budget_posture: Option<String>,
    #[serde(default)]
    pub cap_posture: Option<String>,
    #[serde(default)]
    pub write_scope_classification: Option<String>,
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
    pub selected_model_profile_readiness_status: Option<String>,
    pub budget_posture: Option<String>,
    pub cap_posture: Option<String>,
    pub write_scope_classification: Option<String>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SurrealValue, PartialEq, Eq)]
pub struct TaskStageConsolidationReceipt {
    pub receipt_id: String,
    pub task_id: String,
    pub stage_id: String,
    pub consolidator_profile: String,
    pub merge_policy: String,
    pub result_status: String,
    pub attempt_count: usize,
    pub accepted_attempt_ids: Vec<String>,
    pub rejected_attempt_ids: Vec<String>,
    pub stale_attempt_ids: Vec<String>,
    pub partial_attempt_ids: Vec<String>,
    pub timeout_attempt_ids: Vec<String>,
    pub cap_limited_attempt_ids: Vec<String>,
    pub conflict_attempt_ids: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub facts: Vec<String>,
    pub hypotheses: Vec<String>,
    pub conflicts: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeAttemptReferenceRow {
    task_id: String,
    attempt_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeStageReferenceRow {
    task_id: String,
    stage_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeConsolidationReferenceRow {
    task_id: String,
    receipt_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeRunGraphReferenceRow {
    task_id: String,
    run_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeRunGraphReplayReferenceRow {
    resolved_task_id: String,
    receipt_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct TaskRuntimeDispatchIdentityReferenceRow {
    run_id: String,
    feature_epic_id: Option<String>,
    spec_task_id: Option<String>,
    work_pool_task_id: Option<String>,
    dev_task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConsolidateTaskStageAttemptsRequest {
    pub receipt_id: Option<String>,
    pub task_id: String,
    pub stage_id: String,
    pub consolidator_profile: String,
    pub merge_policy: String,
    pub artifact_refs: Vec<String>,
    pub facts: Vec<String>,
    pub hypotheses: Vec<String>,
    pub conflicts: Vec<String>,
    pub partial_attempt_ids: Vec<String>,
    pub timeout_attempt_ids: Vec<String>,
    pub cap_limited_attempt_ids: Vec<String>,
}

impl StateStore {
    pub(crate) async fn task_runtime_reference_labels_for_tasks(
        &self,
        task_ids: &std::collections::BTreeSet<String>,
    ) -> Result<std::collections::BTreeMap<String, Vec<String>>, StateStoreError> {
        let mut references = std::collections::BTreeMap::<String, Vec<String>>::new();
        if task_ids.is_empty() {
            return Ok(references);
        }

        let task_id_list = task_ids
            .iter()
            .map(|task_id| format!("'{}'", escape_surql_literal(task_id)))
            .collect::<Vec<_>>()
            .join(", ");

        let attempt_rows: Vec<TaskRuntimeAttemptReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, attempt_id FROM task_attempt WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, attempt_id ASC;"
            ))
            .await?;
        for row in attempt_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("task_attempt:{}", row.attempt_id),
            );
        }

        let stage_rows: Vec<TaskRuntimeStageReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, stage_id FROM task_stage WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, stage_id ASC;"
            ))
            .await?;
        for row in stage_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("task_stage:{}", row.stage_id),
            );
        }

        let consolidation_rows: Vec<TaskRuntimeConsolidationReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, receipt_id FROM task_stage_consolidation_receipt WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, receipt_id ASC;"
            ))
            .await?;
        for row in consolidation_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("task_stage_consolidation_receipt:{}", row.receipt_id),
            );
        }

        let execution_plan_rows: Vec<TaskRuntimeRunGraphReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, run_id FROM execution_plan_state WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, run_id ASC;"
            ))
            .await?;
        for row in execution_plan_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("execution_plan_state:{}", row.run_id),
            );
        }

        let continuation_rows: Vec<TaskRuntimeRunGraphReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, run_id FROM run_graph_continuation_binding WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, run_id ASC;"
            ))
            .await?;
        for row in continuation_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("run_graph_continuation_binding:{}", row.run_id),
            );
        }

        let dispatch_context_rows: Vec<TaskRuntimeRunGraphReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT task_id, run_id FROM run_graph_dispatch_context WHERE task_id IN [{task_id_list}] ORDER BY task_id ASC, run_id ASC;"
            ))
            .await?;
        for row in dispatch_context_rows {
            push_task_runtime_reference(
                &mut references,
                row.task_id,
                format!("run_graph_dispatch_context:{}", row.run_id),
            );
        }

        let replay_rows: Vec<TaskRuntimeRunGraphReplayReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT resolved_task_id, receipt_id FROM run_graph_replay_lineage_receipt WHERE resolved_task_id IN [{task_id_list}] ORDER BY resolved_task_id ASC, receipt_id ASC;"
            ))
            .await?;
        for row in replay_rows {
            push_task_runtime_reference(
                &mut references,
                row.resolved_task_id,
                format!("run_graph_replay_lineage_receipt:{}", row.receipt_id),
            );
        }

        let identity_rows: Vec<TaskRuntimeDispatchIdentityReferenceRow> = self
            .task_runtime_reference_query(format!(
                "SELECT run_id, feature_epic_id, spec_task_id, work_pool_task_id, dev_task_id FROM run_graph_dispatch_task_identity WHERE feature_epic_id IN [{task_id_list}] OR spec_task_id IN [{task_id_list}] OR work_pool_task_id IN [{task_id_list}] OR dev_task_id IN [{task_id_list}] ORDER BY run_id ASC;"
            ))
            .await?;
        for row in identity_rows {
            for (field, task_id) in [
                ("feature_epic_id", row.feature_epic_id),
                ("spec_task_id", row.spec_task_id),
                ("work_pool_task_id", row.work_pool_task_id),
                ("dev_task_id", row.dev_task_id),
            ] {
                if let Some(task_id) = task_id.filter(|task_id| task_ids.contains(task_id)) {
                    push_task_runtime_reference(
                        &mut references,
                        task_id,
                        format!("run_graph_dispatch_task_identity:{}:{field}", row.run_id),
                    );
                }
            }
        }

        for labels in references.values_mut() {
            labels.sort();
            labels.dedup();
        }

        Ok(references)
    }

    async fn task_runtime_reference_query<T>(
        &self,
        query: String,
    ) -> Result<Vec<T>, StateStoreError>
    where
        T: serde::de::DeserializeOwned + SurrealValue,
    {
        let mut response = match self.db.query(query).await {
            Ok(response) => response,
            Err(error) if task_runtime_reference_missing_optional_table(&error.to_string()) => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error.into()),
        };
        match response.take(0) {
            Ok(rows) => Ok(rows),
            Err(error) if task_runtime_reference_missing_optional_table(&error.to_string()) => {
                Ok(Vec::new())
            }
            Err(error) => Err(error.into()),
        }
    }

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
            selected_model_profile_readiness_status: normalize_optional(
                request.selected_model_profile_readiness_status,
            ),
            budget_posture: normalize_optional(request.budget_posture),
            cap_posture: normalize_optional(request.cap_posture),
            write_scope_classification: normalize_optional(request.write_scope_classification),
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
        let attempts = self.task_stage_attempts(&task_id, &stage_id).await?;
        let stage = self.task_stage_record(&task_id, &stage_id).await?;
        let mut summary =
            task_stage_summary_from_attempts(task_id, stage_id.clone(), stage.as_ref(), &attempts);
        if summary.latest_consolidation_receipt_id.is_none() {
            summary.latest_consolidation_receipt_id = self
                .latest_task_stage_consolidation_receipt_id(&summary.task_id, &stage_id)
                .await?;
        }
        Ok(summary)
    }

    pub async fn task_stage_summaries_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskStageSummary>, StateStoreError> {
        let task_id = normalize_non_empty("task_id", task_id)?;
        let attempts = self.task_attempts_for_task(&task_id).await?;
        let mut attempts_by_stage = BTreeMap::<String, Vec<TaskAttemptRecord>>::new();
        for attempt in attempts {
            attempts_by_stage
                .entry(attempt.stage_id.clone())
                .or_default()
                .push(attempt);
        }

        let mut summaries = Vec::with_capacity(attempts_by_stage.len());
        for (stage_id, attempts) in attempts_by_stage {
            let stage = self.task_stage_record(&task_id, &stage_id).await?;
            let mut summary = task_stage_summary_from_attempts(
                task_id.clone(),
                stage_id.clone(),
                stage.as_ref(),
                &attempts,
            );
            if summary.latest_consolidation_receipt_id.is_none() {
                summary.latest_consolidation_receipt_id = self
                    .latest_task_stage_consolidation_receipt_id(&task_id, &stage_id)
                    .await?;
            }
            summaries.push(summary);
        }
        Ok(summaries)
    }

    pub async fn task_stage_attempts(
        &self,
        task_id: &str,
        stage_id: &str,
    ) -> Result<Vec<TaskAttemptRecord>, StateStoreError> {
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
        Ok(attempts)
    }

    pub async fn task_attempts_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskAttemptRecord>, StateStoreError> {
        let task_id = normalize_non_empty("task_id", task_id)?;
        self.show_task(&task_id).await?;
        let mut response = self
            .db
            .query(format!(
                "SELECT * FROM task_attempt WHERE task_id = '{}' ORDER BY stage_id ASC, updated_at ASC;",
                escape_surql_literal(&task_id)
            ))
            .await?;
        let attempts: Vec<TaskAttemptRecord> = response.take(0)?;
        Ok(attempts)
    }

    pub async fn consolidate_task_stage_attempts(
        &self,
        request: ConsolidateTaskStageAttemptsRequest,
    ) -> Result<TaskStageConsolidationReceipt, StateStoreError> {
        let task = self
            .validate_task_attempt_binding(&request.task_id, &request.stage_id)
            .await?;
        let task_id = normalize_non_empty("task_id", &request.task_id)?;
        let stage_id = normalize_non_empty("stage_id", &request.stage_id)?;
        let attempts = self.task_stage_attempts(&task_id, &stage_id).await?;
        if attempts.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task attempt consolidate requires at least one attempt for task `{task_id}` stage `{stage_id}`"
                ),
            });
        }
        let conflicts = normalize_artifact_refs(request.conflicts);
        let rollup = decide_task_attempt_rollup(TaskAttemptRollupInput {
            task_updated_at: task.updated_at.clone(),
            attempts: attempts
                .iter()
                .map(|attempt| TaskAttemptRollupAttempt {
                    attempt_id: attempt.attempt_id.clone(),
                    status: attempt.status.clone(),
                    freshness: attempt.freshness.clone(),
                })
                .collect(),
            requested_partial_attempt_ids: normalize_artifact_refs(request.partial_attempt_ids),
            conflicts: conflicts.clone(),
        });
        if !rollup.stale_attempt_ids.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "stale_task_binding: cannot consolidate stale attempts for task `{}` stage `{}`: {}",
                    task.id,
                    stage_id,
                    rollup.stale_attempt_ids.join(", ")
                ),
            });
        }

        let receipt_id = request.receipt_id.unwrap_or_else(|| {
            format!(
                "{}--{}--consolidation--{}",
                sanitize_record_id(&task_id),
                sanitize_record_id(&stage_id),
                unix_timestamp_nanos()
            )
        });
        let receipt = TaskStageConsolidationReceipt {
            receipt_id: normalize_non_empty("receipt_id", &receipt_id)?,
            task_id: task_id.clone(),
            stage_id: stage_id.clone(),
            consolidator_profile: normalize_non_empty(
                "consolidator_profile",
                &request.consolidator_profile,
            )?,
            merge_policy: normalize_non_empty("merge_policy", &request.merge_policy)?,
            result_status: rollup.result_status,
            attempt_count: attempts.len(),
            accepted_attempt_ids: rollup.accepted_attempt_ids,
            rejected_attempt_ids: rollup.rejected_attempt_ids,
            stale_attempt_ids: Vec::new(),
            partial_attempt_ids: rollup.partial_attempt_ids,
            timeout_attempt_ids: normalize_artifact_refs(request.timeout_attempt_ids),
            cap_limited_attempt_ids: normalize_artifact_refs(request.cap_limited_attempt_ids),
            conflict_attempt_ids: rollup.conflict_attempt_ids,
            artifact_refs: normalize_artifact_refs(request.artifact_refs),
            facts: normalize_artifact_refs(request.facts),
            hypotheses: normalize_artifact_refs(request.hypotheses),
            conflicts,
            created_at: task_attempt_timestamp(),
        };

        let _: Option<TaskStageConsolidationReceipt> = self
            .db
            .upsert((
                "task_stage_consolidation_receipt",
                receipt.receipt_id.as_str(),
            ))
            .content(receipt.clone())
            .await?;

        let record_id = task_stage_record_id(&task_id, &stage_id)?;
        let existing = self.task_stage_record(&task_id, &stage_id).await?;
        let now = task_attempt_timestamp();
        let stage = TaskStageRecord {
            stage_record_id: record_id,
            task_id,
            stage_id,
            status: receipt.result_status.clone(),
            latest_consolidation_receipt_id: Some(receipt.receipt_id.clone()),
            created_at: existing
                .as_ref()
                .map(|stage| stage.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        };
        let _: Option<TaskStageRecord> = self
            .db
            .upsert(("task_stage", stage.stage_record_id.as_str()))
            .content(stage)
            .await?;

        Ok(receipt)
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

    async fn latest_task_stage_consolidation_receipt_id(
        &self,
        task_id: &str,
        stage_id: &str,
    ) -> Result<Option<String>, StateStoreError> {
        let mut response = match self
            .db
            .query(format!(
                "SELECT task_id, receipt_id, created_at FROM task_stage_consolidation_receipt WHERE task_id = '{}' AND stage_id = '{}' ORDER BY created_at DESC LIMIT 1;",
                escape_surql_literal(task_id),
                escape_surql_literal(stage_id)
            ))
            .await
        {
            Ok(response) => response,
            Err(error) if task_runtime_reference_missing_optional_table(&error.to_string()) => {
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let rows: Vec<TaskRuntimeConsolidationReferenceRow> = match response.take(0) {
            Ok(rows) => rows,
            Err(error) if task_runtime_reference_missing_optional_table(&error.to_string()) => {
                Vec::new()
            }
            Err(error) => return Err(error.into()),
        };
        Ok(rows.into_iter().next().map(|row| row.receipt_id))
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
            latest_consolidation_receipt_id: attempt.consolidation_receipt_id.clone().or_else(
                || {
                    existing
                        .as_ref()
                        .and_then(|stage| stage.latest_consolidation_receipt_id.clone())
                },
            ),
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
        match decide_task_attempt_binding(TaskAttemptBindingInput {
            task_id: task.id.clone(),
            task_status: task.status.clone(),
            issue_type: task.issue_type.clone(),
            program_container: work_item_is_program_container(&task.issue_type),
        }) {
            TaskAttemptBindingDecision::Admitted => {}
            TaskAttemptBindingDecision::ClosedTask { task_id } => {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "task attempt binding is stale because task `{task_id}` is closed"
                    ),
                });
            }
            TaskAttemptBindingDecision::ProgramContainer {
                task_id,
                issue_type,
            } => {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "task attempt binding requires a leaf task, got `{task_id}` of type `{issue_type}`"
                    ),
                });
            }
        }
        Ok(task)
    }
}

fn push_task_runtime_reference(
    references: &mut std::collections::BTreeMap<String, Vec<String>>,
    task_id: String,
    reference: String,
) {
    references.entry(task_id).or_default().push(reference);
}

fn task_runtime_reference_missing_optional_table(message: &str) -> bool {
    message.contains("does not exist") && message.contains("The table")
}

fn task_stage_summary_from_attempts(
    task_id: String,
    stage_id: String,
    stage: Option<&TaskStageRecord>,
    attempts: &[TaskAttemptRecord],
) -> TaskStageSummary {
    let summary = summarize_task_stage_attempts(
        &attempts
            .iter()
            .map(|attempt| TaskAttemptSummaryInput {
                attempt_id: attempt.attempt_id.clone(),
                status: attempt.status.clone(),
                artifact_refs: attempt.artifact_refs.clone(),
                consolidation_receipt_id: attempt.consolidation_receipt_id.clone(),
                updated_at: attempt.updated_at.clone(),
            })
            .collect::<Vec<_>>(),
        stage.and_then(|stage| stage.latest_consolidation_receipt_id.clone()),
    );
    TaskStageSummary {
        task_id,
        stage_id,
        stage_status: stage.map(|stage| stage.status.clone()),
        attempt_count: summary.attempt_count,
        status_counts: summary.status_counts,
        latest_attempt_id: summary.latest_attempt_id,
        latest_attempt_status: summary.latest_attempt_status,
        latest_consolidation_receipt_id: summary.latest_consolidation_receipt_id,
        artifact_refs: summary.artifact_refs,
    }
}

fn normalize_task_attempt_status(value: &str) -> Result<String, StateStoreError> {
    normalize_authority_attempt_status(value)
        .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })
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
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
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
                consolidation_receipt_id: None,
                selected_model_profile_readiness_status: Some("ready".to_string()),
                budget_posture: Some("in_budget".to_string()),
                cap_posture: Some("cap_ready".to_string()),
                write_scope_classification: Some("read-only".to_string()),
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
        assert_eq!(summary.latest_consolidation_receipt_id.as_deref(), None);
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
