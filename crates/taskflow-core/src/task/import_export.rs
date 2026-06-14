//! Task import/export command helpers.

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
        TaskExportJsonlSummary, TaskImportJsonlSummary, TaskReplaceJsonlSummary,
        task_export_jsonl_success_fields, task_import_jsonl_success_fields,
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
}
