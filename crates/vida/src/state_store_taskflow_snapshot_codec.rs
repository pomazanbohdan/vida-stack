use std::collections::BTreeMap;

use super::{
    CanonicalDependencyEdge, CanonicalIssueType, CanonicalTaskId, CanonicalTaskRecord,
    CanonicalTaskStatus, CanonicalTimestamp, OffsetDateTime, Rfc3339, StateStore, StateStoreError,
    TaskDependencyRecord, TaskPlannerMetadata, TaskRecord, TaskSnapshot,
};

#[allow(dead_code)]
pub(super) fn parse_canonical_timestamp(
    value: &str,
) -> Result<CanonicalTimestamp, StateStoreError> {
    if let Ok(parsed) = OffsetDateTime::parse(value, &Rfc3339) {
        return Ok(CanonicalTimestamp(parsed));
    }

    let nanos =
        value
            .parse::<i128>()
            .map_err(|_| StateStoreError::InvalidCanonicalTaskflowExport {
                reason: format!("updated_at is not RFC3339 or unix nanos: {value}"),
            })?;
    let parsed = OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|error| {
        StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("updated_at unix nanos is invalid ({value}): {error}"),
        }
    })?;
    Ok(CanonicalTimestamp(parsed))
}

#[allow(dead_code)]
pub(super) fn parse_canonical_task_status(
    value: &str,
) -> Result<CanonicalTaskStatus, StateStoreError> {
    match value {
        "open" => Ok(CanonicalTaskStatus::Open),
        "in_progress" => Ok(CanonicalTaskStatus::InProgress),
        "closed" => Ok(CanonicalTaskStatus::Closed),
        "blocked" => Ok(CanonicalTaskStatus::Blocked),
        other => Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("unsupported taskflow-core status mapping: {other}"),
        }),
    }
}

#[allow(dead_code)]
pub(super) fn parse_canonical_issue_type(
    value: &str,
) -> Result<CanonicalIssueType, StateStoreError> {
    match value {
        "epic" => Ok(CanonicalIssueType::Epic),
        "task" => Ok(CanonicalIssueType::Task),
        "bug" => Ok(CanonicalIssueType::Bug),
        "spike" => Ok(CanonicalIssueType::Spike),
        other => Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("unsupported taskflow-core issue_type mapping: {other}"),
        }),
    }
}

#[allow(dead_code)]
pub(super) fn task_dependency_to_canonical_edge(
    dependency: &TaskDependencyRecord,
) -> Result<CanonicalDependencyEdge, StateStoreError> {
    Ok(CanonicalDependencyEdge {
        issue_id: canonical_task_id_from_state("dependency.issue_id", &dependency.issue_id)?,
        depends_on_id: canonical_task_id_from_state(
            "dependency.depends_on_id",
            &dependency.depends_on_id,
        )?,
        dependency_type: dependency.edge_type.clone(),
    })
}

#[allow(dead_code)]
pub(super) fn task_record_to_canonical_snapshot_row(
    task: &TaskRecord,
) -> Result<CanonicalTaskRecord, StateStoreError> {
    Ok(CanonicalTaskRecord {
        id: canonical_task_id_from_state("task.id", &task.id)?,
        title: task.title.clone(),
        status: parse_canonical_task_status(&task.status)?,
        issue_type: parse_canonical_issue_type(&task.issue_type)?,
        updated_at: parse_canonical_timestamp(&task.updated_at)?,
    })
}

fn canonical_task_id_from_state(
    field_name: &str,
    value: &str,
) -> Result<CanonicalTaskId, StateStoreError> {
    CanonicalTaskId::try_new(value).map_err(|error| {
        StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("{field_name} is not a valid task id: {error}"),
        }
    })
}

pub(super) fn canonical_task_status_label(status: CanonicalTaskStatus) -> &'static str {
    match status {
        CanonicalTaskStatus::Open => "open",
        CanonicalTaskStatus::InProgress => "in_progress",
        CanonicalTaskStatus::Closed => "closed",
        CanonicalTaskStatus::Blocked => "blocked",
    }
}

pub(super) fn canonical_issue_type_label(issue_type: CanonicalIssueType) -> &'static str {
    match issue_type {
        CanonicalIssueType::Epic => "epic",
        CanonicalIssueType::Task => "task",
        CanonicalIssueType::Bug => "bug",
        CanonicalIssueType::Spike => "spike",
    }
}

pub(super) fn canonical_timestamp_label(timestamp: &CanonicalTimestamp) -> String {
    timestamp
        .0
        .format(&Rfc3339)
        .unwrap_or_else(|_| timestamp.0.unix_timestamp_nanos().to_string())
}

pub(super) fn canonical_edge_to_task_dependency_record(
    dependency: &CanonicalDependencyEdge,
) -> TaskDependencyRecord {
    TaskDependencyRecord {
        issue_id: dependency.issue_id.as_str().to_string(),
        depends_on_id: dependency.depends_on_id.as_str().to_string(),
        edge_type: dependency.dependency_type.clone(),
        created_at: "canonical-taskflow-snapshot".to_string(),
        created_by: "taskflow-state-fs".to_string(),
        metadata: "{}".to_string(),
        thread_id: String::new(),
    }
}

pub(super) fn canonical_snapshot_row_to_task_record(
    task: &CanonicalTaskRecord,
) -> Result<TaskRecord, StateStoreError> {
    let task_id = task.id.as_str().trim().to_string();
    if task_id.is_empty() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: "canonical taskflow snapshot task id is empty".to_string(),
        });
    }

    let updated_at = canonical_timestamp_label(&task.updated_at);
    let status = canonical_task_status_label(task.status).to_string();
    let (closed_at, close_reason) = if matches!(task.status, CanonicalTaskStatus::Closed) {
        (
            Some(updated_at.clone()),
            Some("imported_from_canonical_taskflow_snapshot".to_string()),
        )
    } else {
        (None, None)
    };
    Ok(TaskRecord {
        id: task_id,
        display_id: None,
        title: task.title.clone(),
        description: String::new(),
        status,
        priority: 0,
        issue_type: canonical_issue_type_label(task.issue_type).to_string(),
        created_at: updated_at.clone(),
        created_by: "taskflow-state-fs".to_string(),
        updated_at,
        closed_at,
        close_reason,
        source_repo: "taskflow-state-fs".to_string(),
        compaction_level: 0,
        original_size: 0,
        notes: None,
        labels: Vec::new(),
        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
        planner_metadata: TaskPlannerMetadata::default(),
        provider_mapping: None,
        dependencies: Vec::new(),
    })
}

pub(super) fn task_records_from_canonical_snapshot(
    snapshot: &TaskSnapshot,
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let task_records = task_records_from_canonical_snapshot_rows(snapshot)?;
    let issues = StateStore::validate_task_graph_rows(&task_records);
    if let Some(first) = issues.first() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!(
                "snapshot graph is invalid: {} on {}",
                first.issue_type, first.issue_id
            ),
        });
    }

    Ok(task_records)
}

pub(super) fn task_records_from_canonical_snapshot_for_additive_import(
    snapshot: &TaskSnapshot,
    existing_tasks: &[TaskRecord],
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let imported_tasks = task_records_from_canonical_snapshot_rows(snapshot)?;
    let mut merged_tasks = existing_tasks
        .iter()
        .cloned()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    for task in &imported_tasks {
        merged_tasks.insert(task.id.clone(), task.clone());
    }

    let merged_rows = merged_tasks.into_values().collect::<Vec<_>>();
    let issues = StateStore::validate_task_graph_rows(&merged_rows);
    if let Some(first) = issues.first() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!(
                "snapshot graph is invalid after additive merge: {} on {}",
                first.issue_type, first.issue_id
            ),
        });
    }

    Ok(imported_tasks)
}

pub(super) fn task_records_from_canonical_snapshot_rows(
    snapshot: &TaskSnapshot,
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let mut dependencies_by_issue = BTreeMap::<String, Vec<TaskDependencyRecord>>::new();
    for dependency in &snapshot.dependencies {
        dependencies_by_issue
            .entry(dependency.issue_id.as_str().to_string())
            .or_default()
            .push(canonical_edge_to_task_dependency_record(dependency));
    }

    let mut task_records = Vec::with_capacity(snapshot.tasks.len());
    for task in &snapshot.tasks {
        let mut task_record = canonical_snapshot_row_to_task_record(task)?;
        if let Some(dependencies) = dependencies_by_issue.remove(task.id.as_str()) {
            task_record.dependencies = dependencies;
        }
        task_records.push(task_record);
    }

    Ok(task_records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_record_with_id(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: "Task".to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 0,
            issue_type: "task".to_string(),
            created_at: "2026-03-08T00:00:00Z".to_string(),
            created_by: "tester".to_string(),
            updated_at: "2026-03-08T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn dependency_record(issue_id: &str, depends_on_id: &str) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "blocks".to_string(),
            created_at: "2026-03-08T00:00:00Z".to_string(),
            created_by: "tester".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    #[test]
    fn task_record_to_canonical_snapshot_row_rejects_empty_state_id_without_panic() {
        let error = task_record_to_canonical_snapshot_row(&task_record_with_id(" \t "))
            .expect_err("invalid persisted task id should be returned as an export error");

        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("task.id"));
                assert!(reason.contains("EmptyTaskId"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn task_dependency_to_canonical_edge_rejects_empty_state_ids_without_panic() {
        let error = task_dependency_to_canonical_edge(&dependency_record("vida-task", " "))
            .expect_err("invalid persisted dependency id should be returned as an export error");

        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("dependency.depends_on_id"));
                assert!(reason.contains("EmptyTaskId"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
