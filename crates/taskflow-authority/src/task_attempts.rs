use std::collections::BTreeMap;

pub const MODULE: &str = "task_attempts";

pub const TASK_ATTEMPT_STATUSES: &[&str] = &[
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptSummaryInput {
    pub attempt_id: String,
    pub status: String,
    pub artifact_refs: Vec<String>,
    pub consolidation_receipt_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStageSummaryDecision {
    pub attempt_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub latest_attempt_id: Option<String>,
    pub latest_attempt_status: Option<String>,
    pub latest_consolidation_receipt_id: Option<String>,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptRollupAttempt {
    pub attempt_id: String,
    pub status: String,
    pub freshness: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptRollupInput {
    pub task_updated_at: String,
    pub attempts: Vec<TaskAttemptRollupAttempt>,
    pub requested_partial_attempt_ids: Vec<String>,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptRollupDecision {
    pub result_status: String,
    pub accepted_attempt_ids: Vec<String>,
    pub rejected_attempt_ids: Vec<String>,
    pub stale_attempt_ids: Vec<String>,
    pub partial_attempt_ids: Vec<String>,
    pub conflict_attempt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttemptBindingInput {
    pub task_id: String,
    pub task_status: String,
    pub issue_type: String,
    pub program_container: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAttemptBindingDecision {
    Admitted,
    ClosedTask { task_id: String },
    ProgramContainer { task_id: String, issue_type: String },
}

pub fn normalize_task_attempt_status(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("task attempt field `status` must be non-empty".to_string());
    }
    if TASK_ATTEMPT_STATUSES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!(
            "task attempt status `{}` is invalid; expected one of {}",
            value.trim(),
            TASK_ATTEMPT_STATUSES.join(", ")
        ))
    }
}

#[must_use]
pub fn decide_task_attempt_binding(input: TaskAttemptBindingInput) -> TaskAttemptBindingDecision {
    if taskflow_core::task_status_is_closed_like(&input.task_status) {
        return TaskAttemptBindingDecision::ClosedTask {
            task_id: input.task_id,
        };
    }
    if input.program_container {
        return TaskAttemptBindingDecision::ProgramContainer {
            task_id: input.task_id,
            issue_type: input.issue_type,
        };
    }
    TaskAttemptBindingDecision::Admitted
}

#[must_use]
pub fn decide_task_attempt_rollup(input: TaskAttemptRollupInput) -> TaskAttemptRollupDecision {
    let accepted_attempt_ids = input
        .attempts
        .iter()
        .filter(|attempt| attempt.status == "accepted")
        .map(|attempt| attempt.attempt_id.clone())
        .collect::<Vec<_>>();
    let rejected_attempt_ids = input
        .attempts
        .iter()
        .filter(|attempt| attempt.status == "rejected")
        .map(|attempt| attempt.attempt_id.clone())
        .collect::<Vec<_>>();
    let stale_attempt_ids = input
        .attempts
        .iter()
        .filter(|attempt| attempt.freshness != input.task_updated_at)
        .map(|attempt| attempt.attempt_id.clone())
        .collect::<Vec<_>>();
    let mut partial_attempt_ids = input.requested_partial_attempt_ids;
    for attempt in input
        .attempts
        .iter()
        .filter(|attempt| attempt.status == "partially_accepted")
    {
        if !partial_attempt_ids.contains(&attempt.attempt_id) {
            partial_attempt_ids.push(attempt.attempt_id.clone());
        }
    }
    let conflict_attempt_ids = if input.conflicts.is_empty() {
        Vec::new()
    } else {
        input
            .attempts
            .iter()
            .map(|attempt| attempt.attempt_id.clone())
            .collect()
    };
    TaskAttemptRollupDecision {
        result_status: if input.conflicts.is_empty() {
            "accepted".to_string()
        } else {
            "conflict".to_string()
        },
        accepted_attempt_ids,
        rejected_attempt_ids,
        stale_attempt_ids,
        partial_attempt_ids,
        conflict_attempt_ids,
    }
}

#[must_use]
pub fn summarize_task_stage_attempts(
    attempts: &[TaskAttemptSummaryInput],
    stage_latest_consolidation_receipt_id: Option<String>,
) -> TaskStageSummaryDecision {
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
    let latest_consolidation_receipt_id = latest
        .and_then(|attempt| attempt.consolidation_receipt_id.clone())
        .or_else(|| {
            if attempts.is_empty() {
                stage_latest_consolidation_receipt_id
            } else {
                None
            }
        });
    TaskStageSummaryDecision {
        attempt_count: attempts.len(),
        status_counts,
        latest_attempt_id: latest.map(|attempt| attempt.attempt_id.clone()),
        latest_attempt_status: latest.map(|attempt| attempt.status.clone()),
        latest_consolidation_receipt_id,
        artifact_refs,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TaskAttemptBindingDecision, TaskAttemptBindingInput, TaskAttemptRollupAttempt,
        TaskAttemptRollupInput, TaskAttemptSummaryInput, decide_task_attempt_binding,
        decide_task_attempt_rollup, normalize_task_attempt_status, summarize_task_stage_attempts,
    };

    #[test]
    fn task_attempt_binding_admits_open_leaf_tasks() {
        let decision = decide_task_attempt_binding(TaskAttemptBindingInput {
            task_id: "task-a".to_string(),
            task_status: "in_progress".to_string(),
            issue_type: "task".to_string(),
            program_container: false,
        });

        assert_eq!(decision, TaskAttemptBindingDecision::Admitted);
    }

    #[test]
    fn task_attempt_binding_rejects_closed_tasks() {
        let decision = decide_task_attempt_binding(TaskAttemptBindingInput {
            task_id: "task-a".to_string(),
            task_status: "closed".to_string(),
            issue_type: "task".to_string(),
            program_container: false,
        });

        assert_eq!(
            decision,
            TaskAttemptBindingDecision::ClosedTask {
                task_id: "task-a".to_string()
            }
        );
    }

    #[test]
    fn task_attempt_binding_rejects_program_containers() {
        let decision = decide_task_attempt_binding(TaskAttemptBindingInput {
            task_id: "epic-a".to_string(),
            task_status: "open".to_string(),
            issue_type: "epic".to_string(),
            program_container: true,
        });

        assert_eq!(
            decision,
            TaskAttemptBindingDecision::ProgramContainer {
                task_id: "epic-a".to_string(),
                issue_type: "epic".to_string()
            }
        );
    }

    #[test]
    fn task_attempt_rollup_classifies_accepted_rejected_partial_and_stale_attempts() {
        let decision = decide_task_attempt_rollup(TaskAttemptRollupInput {
            task_updated_at: "task-v2".to_string(),
            attempts: vec![
                TaskAttemptRollupAttempt {
                    attempt_id: "accepted-a".to_string(),
                    status: "accepted".to_string(),
                    freshness: "task-v2".to_string(),
                },
                TaskAttemptRollupAttempt {
                    attempt_id: "rejected-a".to_string(),
                    status: "rejected".to_string(),
                    freshness: "task-v2".to_string(),
                },
                TaskAttemptRollupAttempt {
                    attempt_id: "partial-a".to_string(),
                    status: "partially_accepted".to_string(),
                    freshness: "task-v1".to_string(),
                },
            ],
            requested_partial_attempt_ids: vec!["manual-partial".to_string()],
            conflicts: Vec::new(),
        });

        assert_eq!(decision.result_status, "accepted");
        assert_eq!(decision.accepted_attempt_ids, vec!["accepted-a"]);
        assert_eq!(decision.rejected_attempt_ids, vec!["rejected-a"]);
        assert_eq!(
            decision.partial_attempt_ids,
            vec!["manual-partial", "partial-a"]
        );
        assert_eq!(decision.stale_attempt_ids, vec!["partial-a"]);
        assert!(decision.conflict_attempt_ids.is_empty());
    }

    #[test]
    fn task_attempt_rollup_marks_all_attempts_conflicting_when_conflicts_exist() {
        let decision = decide_task_attempt_rollup(TaskAttemptRollupInput {
            task_updated_at: "task-v1".to_string(),
            attempts: vec![
                TaskAttemptRollupAttempt {
                    attempt_id: "attempt-a".to_string(),
                    status: "accepted".to_string(),
                    freshness: "task-v1".to_string(),
                },
                TaskAttemptRollupAttempt {
                    attempt_id: "attempt-b".to_string(),
                    status: "rejected".to_string(),
                    freshness: "task-v1".to_string(),
                },
            ],
            requested_partial_attempt_ids: Vec::new(),
            conflicts: vec!["same-file".to_string()],
        });

        assert_eq!(decision.result_status, "conflict");
        assert_eq!(
            decision.conflict_attempt_ids,
            vec!["attempt-a", "attempt-b"]
        );
    }

    #[test]
    fn task_stage_summary_selects_latest_attempt_and_deduplicates_artifacts() {
        let decision = summarize_task_stage_attempts(
            &[
                TaskAttemptSummaryInput {
                    attempt_id: "attempt-a".to_string(),
                    status: "rejected".to_string(),
                    artifact_refs: vec!["artifact-a".to_string()],
                    consolidation_receipt_id: None,
                    updated_at: "2026-06-05T00:00:00Z".to_string(),
                },
                TaskAttemptSummaryInput {
                    attempt_id: "attempt-b".to_string(),
                    status: "accepted".to_string(),
                    artifact_refs: vec!["artifact-a".to_string(), "artifact-b".to_string()],
                    consolidation_receipt_id: Some("receipt-b".to_string()),
                    updated_at: "2026-06-05T00:01:00Z".to_string(),
                },
            ],
            None,
        );

        assert_eq!(decision.attempt_count, 2);
        assert_eq!(decision.status_counts["accepted"], 1);
        assert_eq!(decision.status_counts["rejected"], 1);
        assert_eq!(decision.latest_attempt_id.as_deref(), Some("attempt-b"));
        assert_eq!(
            decision.latest_consolidation_receipt_id.as_deref(),
            Some("receipt-b")
        );
        assert_eq!(
            decision.artifact_refs,
            vec!["artifact-a".to_string(), "artifact-b".to_string()]
        );
    }

    #[test]
    fn task_stage_summary_ignores_stage_receipt_when_latest_attempt_has_none() {
        let decision = summarize_task_stage_attempts(
            &[TaskAttemptSummaryInput {
                attempt_id: "attempt-b".to_string(),
                status: "accepted".to_string(),
                artifact_refs: vec!["artifact-b".to_string()],
                consolidation_receipt_id: None,
                updated_at: "2026-06-05T00:01:00Z".to_string(),
            }],
            Some("stage-receipt-b".to_string()),
        );

        assert_eq!(decision.latest_attempt_id.as_deref(), Some("attempt-b"));
        assert_eq!(decision.latest_consolidation_receipt_id.as_deref(), None);
    }

    #[test]
    fn task_stage_summary_reports_stage_receipt_when_no_attempts_exist() {
        let decision = summarize_task_stage_attempts(&[], Some("stage-receipt-b".to_string()));

        assert_eq!(decision.latest_attempt_id.as_deref(), None);
        assert_eq!(
            decision.latest_consolidation_receipt_id.as_deref(),
            Some("stage-receipt-b")
        );
    }

    #[test]
    fn task_attempt_statuses_fail_closed() {
        let error = normalize_task_attempt_status("completed").expect_err("completed is legacy");
        assert!(error.contains("expected one of submitted, running, produced"));
    }
}
