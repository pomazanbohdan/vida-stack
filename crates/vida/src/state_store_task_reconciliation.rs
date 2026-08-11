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

#[cfg(test)]
mod tests {
    use super::{
        TaskReconciliationRollup, TaskReconciliationRollupRow, TaskReconciliationSummary,
        TaskflowSnapshotBridgeSummary, count_snapshot_bridge_rows,
    };
    use std::collections::BTreeMap;

    fn row(
        operation: &str,
        source_kind: &str,
        source_path: Option<&str>,
        recorded_at: &str,
    ) -> TaskReconciliationRollupRow {
        TaskReconciliationRollupRow {
            operation: operation.to_string(),
            source_kind: source_kind.to_string(),
            source_path: source_path.map(str::to_string),
            task_count: 2,
            dependency_count: 3,
            stale_removed_count: 1,
            recorded_at: recorded_at.to_string(),
        }
    }

    #[test]
    fn reconciliation_summary_display_exposes_optional_source_path() {
        let summary = TaskReconciliationSummary {
            receipt_id: "receipt-1".to_string(),
            operation: "import".to_string(),
            source_kind: "taskflow".to_string(),
            source_path: None,
            task_count: 2,
            dependency_count: 3,
            stale_removed_count: 1,
            recorded_at: "2026-08-11T00:00:00Z".to_string(),
        };

        assert_eq!(
            summary.as_display(),
            "import via taskflow (tasks=2, dependencies=3, stale_removed=1, source_path=none)"
        );
    }

    #[test]
    fn reconciliation_rollup_display_handles_zero_and_many_receipts() {
        let empty = TaskReconciliationRollup {
            total_receipts: 0,
            latest_recorded_at: None,
            latest_source_path: None,
            total_task_rows: 0,
            total_dependency_rows: 0,
            total_stale_removed: 0,
            by_operation: BTreeMap::new(),
            by_source_kind: BTreeMap::new(),
            rows: Vec::new(),
        };
        assert_eq!(empty.as_display(), "0 receipts");

        let populated = TaskReconciliationRollup {
            total_receipts: 2,
            latest_recorded_at: Some("2026-08-11T00:01:00Z".to_string()),
            latest_source_path: Some("snapshot.json".to_string()),
            total_task_rows: 4,
            total_dependency_rows: 6,
            total_stale_removed: 2,
            by_operation: BTreeMap::from([(String::from("import"), 2)]),
            by_source_kind: BTreeMap::from([(String::from("taskflow"), 2)]),
            rows: Vec::new(),
        };
        assert_eq!(
            populated.as_display(),
            "2 receipts (tasks=4, dependencies=6, stale_removed=2, operations: import=2; source_kinds: taskflow=2; latest_recorded_at=2026-08-11T00:01:00Z; latest_source_path=snapshot.json)"
        );
    }

    #[test]
    fn snapshot_bridge_row_count_filters_each_optional_dimension() {
        let rows = vec![
            row("import", "memory", Some("memory.json"), "one"),
            row("replace", "file", Some("file.json"), "two"),
            row("import", "file", None, "three"),
        ];

        assert_eq!(count_snapshot_bridge_rows(&rows, None, None), 3);
        assert_eq!(count_snapshot_bridge_rows(&rows, Some("import"), None), 2);
        assert_eq!(count_snapshot_bridge_rows(&rows, None, Some("file")), 2);
        assert_eq!(
            count_snapshot_bridge_rows(&rows, Some("import"), Some("memory")),
            1
        );
        assert_eq!(
            count_snapshot_bridge_rows(&rows, Some("export"), Some("memory")),
            0
        );
    }

    #[test]
    fn snapshot_bridge_display_sums_memory_and_file_operation_counts() {
        let idle = TaskflowSnapshotBridgeSummary {
            total_receipts: 0,
            export_receipts: 0,
            import_receipts: 0,
            replace_receipts: 0,
            object_export_receipts: 0,
            memory_export_receipts: 0,
            memory_import_receipts: 0,
            memory_replace_receipts: 0,
            file_export_receipts: 0,
            file_import_receipts: 0,
            file_replace_receipts: 0,
            total_task_rows: 0,
            total_dependency_rows: 0,
            total_stale_removed: 0,
            latest_operation: None,
            latest_source_kind: None,
            latest_source_path: None,
            latest_recorded_at: None,
        };
        assert_eq!(idle.as_display(), "idle (no snapshot bridge receipts)");

        let active = TaskflowSnapshotBridgeSummary {
            total_receipts: 3,
            export_receipts: 1,
            import_receipts: 1,
            replace_receipts: 1,
            object_export_receipts: 1,
            memory_export_receipts: 1,
            memory_import_receipts: 1,
            memory_replace_receipts: 1,
            file_export_receipts: 1,
            file_import_receipts: 0,
            file_replace_receipts: 1,
            total_task_rows: 4,
            total_dependency_rows: 5,
            total_stale_removed: 1,
            latest_operation: Some("replace".to_string()),
            latest_source_kind: Some("file".to_string()),
            latest_source_path: Some("snapshot.json".to_string()),
            latest_recorded_at: Some("now".to_string()),
        };
        assert_eq!(
            active.as_display(),
            "receipts=3 export=1 import=1 replace=1 object=1 memory=3 file=2 tasks=4 dependencies=5 stale_removed=1 latest=replace via file source_path=snapshot.json"
        );
    }
}
