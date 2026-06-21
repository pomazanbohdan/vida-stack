//! Task import/export command helpers.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotGraphValidationContext {
    DirectSnapshot,
    AdditiveMerge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSnapshotGraphIssue {
    pub issue_type: String,
    pub issue_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskImportJsonlSummary {
    pub source_path: String,
    pub imported_count: usize,
    pub unchanged_count: usize,
    pub updated_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskReplaceJsonlSummary {
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskExportJsonlSummary {
    pub exported_count: u64,
    pub target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskReconciliationRollupRowInput {
    pub operation: String,
    pub source_kind: String,
    pub source_path: Option<String>,
    pub task_count: usize,
    pub dependency_count: usize,
    pub stale_removed_count: usize,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskReconciliationRollupDecision {
    pub total_receipts: usize,
    pub latest_recorded_at: Option<String>,
    pub latest_source_path: Option<String>,
    pub total_task_rows: usize,
    pub total_dependency_rows: usize,
    pub total_stale_removed: usize,
    pub by_operation: BTreeMap<String, usize>,
    pub by_source_kind: BTreeMap<String, usize>,
}

#[must_use]
pub fn snapshot_graph_validation_error(
    context: SnapshotGraphValidationContext,
    issues: &[TaskSnapshotGraphIssue],
) -> Option<String> {
    let first = issues.first()?;
    let prefix = match context {
        SnapshotGraphValidationContext::DirectSnapshot => "snapshot graph is invalid",
        SnapshotGraphValidationContext::AdditiveMerge => {
            "snapshot graph is invalid after additive merge"
        }
    };
    Some(format!(
        "{prefix}: {} on {}",
        first.issue_type, first.issue_id
    ))
}

#[must_use]
pub fn task_reconciliation_rollup(
    rows: &[TaskReconciliationRollupRowInput],
) -> TaskReconciliationRollupDecision {
    let mut by_operation = BTreeMap::<String, usize>::new();
    let mut by_source_kind = BTreeMap::<String, usize>::new();
    let latest_recorded_at = rows.first().map(|row| row.recorded_at.clone());
    let latest_source_path = rows.first().and_then(|row| row.source_path.clone());
    let mut total_task_rows = 0usize;
    let mut total_dependency_rows = 0usize;
    let mut total_stale_removed = 0usize;

    for row in rows {
        *by_operation.entry(row.operation.clone()).or_insert(0) += 1;
        *by_source_kind.entry(row.source_kind.clone()).or_insert(0) += 1;
        total_task_rows += row.task_count;
        total_dependency_rows += row.dependency_count;
        total_stale_removed += row.stale_removed_count;
    }

    TaskReconciliationRollupDecision {
        total_receipts: by_operation.values().sum(),
        latest_recorded_at,
        latest_source_path,
        total_task_rows,
        total_dependency_rows,
        total_stale_removed,
        by_operation,
        by_source_kind,
    }
}

#[must_use]
pub fn task_import_jsonl_success_fields(
    status: &str,
    summary: &TaskImportJsonlSummary,
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "source_path": summary.source_path,
        "imported_count": summary.imported_count,
        "unchanged_count": summary.unchanged_count,
        "updated_count": summary.updated_count,
    })
}

#[must_use]
pub fn task_replace_jsonl_success_fields(
    status: &str,
    summary: &TaskReplaceJsonlSummary,
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "operation": "replace_snapshot",
        "source_path": summary.source_path,
    })
}

#[must_use]
pub fn task_export_jsonl_success_fields(summary: &TaskExportJsonlSummary) -> serde_json::Value {
    serde_json::json!({
        "exported_count": summary.exported_count,
        "target_path": summary.target_path,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SnapshotGraphValidationContext, TaskExportJsonlSummary, TaskImportJsonlSummary,
        TaskReconciliationRollupRowInput, TaskReplaceJsonlSummary, TaskSnapshotGraphIssue,
        snapshot_graph_validation_error, task_export_jsonl_success_fields,
        task_import_jsonl_success_fields, task_reconciliation_rollup,
        task_replace_jsonl_success_fields,
    };

    #[test]
    fn task_import_jsonl_success_fields_preserve_public_payload_shape() {
        let payload = task_import_jsonl_success_fields(
            "pass",
            &TaskImportJsonlSummary {
                source_path: "tasks.jsonl".to_string(),
                imported_count: 3,
                unchanged_count: 2,
                updated_count: 1,
            },
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["source_path"], "tasks.jsonl");
        assert_eq!(payload["imported_count"], 3);
        assert_eq!(payload["unchanged_count"], 2);
        assert_eq!(payload["updated_count"], 1);
    }

    #[test]
    fn task_replace_jsonl_success_fields_preserve_public_payload_shape() {
        let payload = task_replace_jsonl_success_fields(
            "pass",
            &TaskReplaceJsonlSummary {
                source_path: "snapshot.jsonl".to_string(),
            },
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["operation"], "replace_snapshot");
        assert_eq!(payload["source_path"], "snapshot.jsonl");
    }

    #[test]
    fn task_export_jsonl_success_fields_preserve_public_payload_shape() {
        let payload = task_export_jsonl_success_fields(&TaskExportJsonlSummary {
            exported_count: 42,
            target_path: "out.jsonl".to_string(),
        });

        assert_eq!(payload["exported_count"], 42);
        assert_eq!(payload["target_path"], "out.jsonl");
    }

    #[test]
    fn snapshot_graph_validation_error_names_context_and_first_issue() {
        let issues = vec![TaskSnapshotGraphIssue {
            issue_type: "multiple_parent_edges".to_string(),
            issue_id: "task-a".to_string(),
        }];

        assert_eq!(
            snapshot_graph_validation_error(
                SnapshotGraphValidationContext::DirectSnapshot,
                &issues
            )
            .as_deref(),
            Some("snapshot graph is invalid: multiple_parent_edges on task-a")
        );
        assert_eq!(
            snapshot_graph_validation_error(SnapshotGraphValidationContext::AdditiveMerge, &issues)
                .as_deref(),
            Some("snapshot graph is invalid after additive merge: multiple_parent_edges on task-a")
        );
        assert!(
            snapshot_graph_validation_error(SnapshotGraphValidationContext::DirectSnapshot, &[])
                .is_none()
        );
    }

    #[test]
    fn task_reconciliation_rollup_aggregates_counts_and_latest_row() {
        let decision = task_reconciliation_rollup(&[
            TaskReconciliationRollupRowInput {
                operation: "import_snapshot".to_string(),
                source_kind: "canonical_snapshot_file".to_string(),
                source_path: Some("new.json".to_string()),
                task_count: 3,
                dependency_count: 2,
                stale_removed_count: 0,
                recorded_at: "2026-06-22T00:02:00Z".to_string(),
            },
            TaskReconciliationRollupRowInput {
                operation: "replace_snapshot".to_string(),
                source_kind: "canonical_snapshot_memory".to_string(),
                source_path: Some("old.json".to_string()),
                task_count: 5,
                dependency_count: 4,
                stale_removed_count: 1,
                recorded_at: "2026-06-22T00:01:00Z".to_string(),
            },
        ]);

        assert_eq!(decision.total_receipts, 2);
        assert_eq!(
            decision.latest_recorded_at.as_deref(),
            Some("2026-06-22T00:02:00Z")
        );
        assert_eq!(decision.latest_source_path.as_deref(), Some("new.json"));
        assert_eq!(decision.total_task_rows, 8);
        assert_eq!(decision.total_dependency_rows, 6);
        assert_eq!(decision.total_stale_removed, 1);
        assert_eq!(decision.by_operation["import_snapshot"], 1);
        assert_eq!(decision.by_source_kind["canonical_snapshot_file"], 1);
    }
}
