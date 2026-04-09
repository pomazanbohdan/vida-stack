use super::*;

pub(crate) struct TaskReconciliationSummaryInput {
    pub(crate) operation: String,
    pub(crate) source_kind: String,
    pub(crate) source_path: Option<String>,
    pub(crate) task_count: usize,
    pub(crate) dependency_count: usize,
    pub(crate) stale_removed_count: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct TaskReconciliationSummaryRow {
    pub(crate) receipt_id: String,
    pub(crate) operation: String,
    pub(crate) source_kind: String,
    pub(crate) source_path: Option<String>,
    pub(crate) task_count: usize,
    pub(crate) dependency_count: usize,
    pub(crate) stale_removed_count: usize,
    pub(crate) recorded_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct TaskReconciliationSummary {
    pub receipt_id: String,
    pub operation: String,
    pub source_kind: String,
    pub source_path: Option<String>,
    pub task_count: usize,
    pub dependency_count: usize,
    pub stale_removed_count: usize,
    pub recorded_at: String,
}

impl TaskReconciliationSummary {
    pub fn as_display(&self) -> String {
        let source_path = self.source_path.as_deref().unwrap_or("none");
        format!(
            "{} via {} (tasks={}, dependencies={}, stale_removed={}, source_path={})",
            self.operation,
            self.source_kind,
            self.task_count,
            self.dependency_count,
            self.stale_removed_count,
            source_path
        )
    }
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
pub(crate) struct TaskReconciliationRollupRow {
    pub(crate) operation: String,
    pub(crate) source_kind: String,
    pub(crate) source_path: Option<String>,
    pub(crate) task_count: usize,
    pub(crate) dependency_count: usize,
    pub(crate) stale_removed_count: usize,
    pub(crate) recorded_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TaskReconciliationRollup {
    pub total_receipts: usize,
    pub latest_recorded_at: Option<String>,
    pub latest_source_path: Option<String>,
    pub total_task_rows: usize,
    pub total_dependency_rows: usize,
    pub total_stale_removed: usize,
    pub by_operation: BTreeMap<String, usize>,
    pub by_source_kind: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) rows: Vec<TaskReconciliationRollupRow>,
}

impl TaskReconciliationRollup {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "0 receipts".to_string();
        }

        let operations = self
            .by_operation
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let source_kinds = self
            .by_source_kind
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let latest_recorded_at = self.latest_recorded_at.as_deref().unwrap_or("none");
        let latest_source_path = self.latest_source_path.as_deref().unwrap_or("none");

        format!(
            "{} receipts (tasks={}, dependencies={}, stale_removed={}, operations: {}; source_kinds: {}; latest_recorded_at={}; latest_source_path={})",
            self.total_receipts,
            self.total_task_rows,
            self.total_dependency_rows,
            self.total_stale_removed,
            operations,
            source_kinds,
            latest_recorded_at,
            latest_source_path
        )
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TaskflowSnapshotBridgeSummary {
    pub total_receipts: usize,
    pub export_receipts: usize,
    pub import_receipts: usize,
    pub replace_receipts: usize,
    pub object_export_receipts: usize,
    pub memory_export_receipts: usize,
    pub memory_import_receipts: usize,
    pub memory_replace_receipts: usize,
    pub file_export_receipts: usize,
    pub file_import_receipts: usize,
    pub file_replace_receipts: usize,
    pub total_task_rows: usize,
    pub total_dependency_rows: usize,
    pub total_stale_removed: usize,
    pub latest_operation: Option<String>,
    pub latest_source_kind: Option<String>,
    pub latest_source_path: Option<String>,
    pub latest_recorded_at: Option<String>,
}

impl TaskflowSnapshotBridgeSummary {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "idle (no snapshot bridge receipts)".to_string();
        }

        format!(
            "receipts={} export={} import={} replace={} object={} memory={} file={} tasks={} dependencies={} stale_removed={} latest={} via {} source_path={}",
            self.total_receipts,
            self.export_receipts,
            self.import_receipts,
            self.replace_receipts,
            self.object_export_receipts,
            self.memory_export_receipts
                + self.memory_import_receipts
                + self.memory_replace_receipts,
            self.file_export_receipts + self.file_import_receipts + self.file_replace_receipts,
            self.total_task_rows,
            self.total_dependency_rows,
            self.total_stale_removed,
            self.latest_operation.as_deref().unwrap_or("none"),
            self.latest_source_kind.as_deref().unwrap_or("none"),
            self.latest_source_path.as_deref().unwrap_or("none"),
        )
    }
}

pub(crate) fn count_snapshot_bridge_rows(
    rows: &[TaskReconciliationRollupRow],
    operation: Option<&str>,
    source_kind: Option<&str>,
) -> usize {
    rows.iter()
        .filter(|row| {
            operation
                .map(|expected| row.operation == expected)
                .unwrap_or(true)
        })
        .filter(|row| {
            source_kind
                .map(|expected| row.source_kind == expected)
                .unwrap_or(true)
        })
        .count()
}
