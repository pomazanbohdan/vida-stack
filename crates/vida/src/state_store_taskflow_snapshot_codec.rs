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
) -> CanonicalDependencyEdge {
    CanonicalDependencyEdge {
        issue_id: CanonicalTaskId::new(&dependency.issue_id),
        depends_on_id: CanonicalTaskId::new(&dependency.depends_on_id),
        dependency_type: dependency.edge_type.clone(),
    }
}

#[allow(dead_code)]
pub(super) fn task_record_to_canonical_snapshot_row(
    task: &TaskRecord,
) -> Result<CanonicalTaskRecord, StateStoreError> {
    Ok(CanonicalTaskRecord {
        id: CanonicalTaskId::new(&task.id),
        title: task.title.clone(),
        status: parse_canonical_task_status(&task.status)?,
        issue_type: parse_canonical_issue_type(&task.issue_type)?,
        updated_at: parse_canonical_timestamp(&task.updated_at)?,
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
        issue_id: dependency.issue_id.0.clone(),
        depends_on_id: dependency.depends_on_id.0.clone(),
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
    let task_id = task.id.0.trim().to_string();
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
            .entry(dependency.issue_id.0.clone())
            .or_default()
            .push(canonical_edge_to_task_dependency_record(dependency));
    }

    let mut task_records = Vec::with_capacity(snapshot.tasks.len());
    for task in &snapshot.tasks {
        let mut task_record = canonical_snapshot_row_to_task_record(task)?;
        if let Some(dependencies) = dependencies_by_issue.remove(&task.id.0) {
            task_record.dependencies = dependencies;
        }
        task_records.push(task_record);
    }

    Ok(task_records)
}
