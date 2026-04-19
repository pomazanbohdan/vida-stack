use super::*;
use serde::Deserialize;

#[derive(Debug, Default, serde::Serialize, SurrealValue, Clone, PartialEq, Eq)]
pub struct TaskExecutionSemantics {
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub order_bucket: Option<String>,
    #[serde(default)]
    pub parallel_group: Option<String>,
    #[serde(default)]
    pub conflict_domain: Option<String>,
}

#[derive(Deserialize)]
struct TaskExecutionSemanticsWire {
    #[serde(default)]
    execution_mode: Option<String>,
    #[serde(default)]
    order_bucket: Option<String>,
    #[serde(default)]
    parallel_group: Option<String>,
    #[serde(default)]
    conflict_domain: Option<String>,
}

impl<'de> serde::Deserialize<'de> for TaskExecutionSemantics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <Option<TaskExecutionSemanticsWire> as Deserialize>::deserialize(deserializer)?;
        Ok(value
            .map(|wire| Self {
                execution_mode: wire.execution_mode,
                order_bucket: wire.order_bucket,
                parallel_group: wire.parallel_group,
                conflict_domain: wire.conflict_domain,
            })
            .unwrap_or_default())
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskJsonlRecord {
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) display_id: Option<String>,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) priority: u32,
    #[serde(default)]
    pub(crate) issue_type: String,
    #[serde(default)]
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) created_by: String,
    #[serde(default)]
    pub(crate) updated_at: String,
    #[serde(default)]
    pub(crate) closed_at: Option<String>,
    #[serde(default)]
    pub(crate) close_reason: Option<String>,
    #[serde(default)]
    pub(crate) source_repo: String,
    #[serde(default)]
    pub(crate) compaction_level: u32,
    #[serde(default)]
    pub(crate) original_size: u32,
    #[serde(default)]
    pub(crate) notes: Option<String>,
    #[serde(default)]
    pub(crate) labels: Vec<String>,
    #[serde(default)]
    pub(crate) execution_semantics: TaskExecutionSemantics,
    #[serde(default)]
    pub(crate) dependencies: Vec<TaskDependencyJsonlRecord>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskDependencyJsonlRecord {
    pub(crate) issue_id: String,
    pub(crate) depends_on_id: String,
    #[serde(rename = "type")]
    pub(crate) edge_type: String,
    #[serde(default)]
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) created_by: String,
    #[serde(default)]
    pub(crate) metadata: String,
    #[serde(default)]
    pub(crate) thread_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone)]
pub(crate) struct TaskContent {
    pub(crate) task_id: String,
    pub(crate) display_id: Option<String>,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) priority: u32,
    pub(crate) issue_type: String,
    pub(crate) created_at: String,
    pub(crate) created_by: String,
    pub(crate) updated_at: String,
    pub(crate) closed_at: Option<String>,
    pub(crate) close_reason: Option<String>,
    pub(crate) source_repo: String,
    pub(crate) compaction_level: u32,
    pub(crate) original_size: u32,
    pub(crate) notes: Option<String>,
    pub(crate) labels: Vec<String>,
    #[serde(default)]
    pub(crate) execution_semantics: TaskExecutionSemantics,
    pub(crate) dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub(crate) struct TaskStorageRow {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) display_id: Option<String>,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) priority: u32,
    pub(crate) issue_type: String,
    pub(crate) created_at: String,
    pub(crate) created_by: String,
    pub(crate) updated_at: String,
    pub(crate) closed_at: Option<String>,
    pub(crate) close_reason: Option<String>,
    pub(crate) source_repo: String,
    pub(crate) compaction_level: u32,
    pub(crate) original_size: u32,
    pub(crate) notes: Option<String>,
    pub(crate) labels: Vec<String>,
    #[serde(default)]
    pub(crate) execution_semantics: TaskExecutionSemantics,
    pub(crate) dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    #[serde(default)]
    pub display_id: Option<String>,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: u32,
    pub issue_type: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,
    pub source_repo: String,
    pub compaction_level: u32,
    pub original_size: u32,
    pub notes: Option<String>,
    pub labels: Vec<String>,
    #[serde(default)]
    pub execution_semantics: TaskExecutionSemantics,
    pub dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug)]
pub struct CreateTaskRequest<'a> {
    pub task_id: &'a str,
    pub title: &'a str,
    pub display_id: Option<&'a str>,
    pub description: &'a str,
    pub issue_type: &'a str,
    pub status: &'a str,
    pub priority: u32,
    pub parent_id: Option<&'a str>,
    pub labels: &'a [String],
    pub execution_semantics: TaskExecutionSemantics,
    pub created_by: &'a str,
    pub source_repo: &'a str,
}

#[derive(Debug)]
pub struct UpdateTaskRequest<'a> {
    pub task_id: &'a str,
    pub status: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub description: Option<&'a str>,
    pub parent_id: Option<Option<&'a str>>,
    pub add_labels: &'a [String],
    pub remove_labels: &'a [String],
    pub set_labels: Option<&'a [String]>,
    pub execution_mode: Option<Option<&'a str>>,
    pub order_bucket: Option<Option<&'a str>>,
    pub parallel_group: Option<Option<&'a str>>,
    pub conflict_domain: Option<Option<&'a str>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskBulkReparentResult {
    pub from_parent_id: String,
    pub to_parent_id: String,
    pub requested_child_ids: Vec<String>,
    pub moved_child_ids: Vec<String>,
    pub moved_count: usize,
    pub dry_run: bool,
    pub tasks: Vec<TaskRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskSchedulingCandidate {
    pub task: TaskRecord,
    pub ready_now: bool,
    pub ready_parallel_safe: bool,
    pub blocked_by: Vec<TaskDependencyStatus>,
    pub active_critical_path: bool,
    pub parallel_blockers: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskSchedulingProjection {
    pub current_task_id: Option<String>,
    pub ready: Vec<TaskSchedulingCandidate>,
    pub blocked: Vec<TaskSchedulingCandidate>,
    pub parallel_candidates_after_current: Vec<TaskRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyStatus {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub dependency_status: String,
    pub dependency_issue_type: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BlockedTaskRecord {
    pub task: TaskRecord,
    pub blockers: Vec<TaskDependencyStatus>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyTreeNode {
    pub task: TaskRecord,
    pub dependencies: Vec<TaskDependencyTreeEdge>,
    pub children: Vec<TaskDependencyTreeChild>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyTreeEdge {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub dependency_status: String,
    pub dependency_issue_type: Option<String>,
    pub node: Option<Box<TaskDependencyTreeNode>>,
    pub cycle: bool,
    pub missing: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyTreeChild {
    pub child_id: String,
    pub child_status: String,
    pub child_issue_type: Option<String>,
    pub node: Option<Box<TaskDependencyTreeNode>>,
    pub cycle: bool,
    pub missing: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct TaskProgressSummary {
    pub root_task: TaskRecord,
    pub progress_basis: String,
    pub direct_child_count: usize,
    pub descendant_count: usize,
    pub open_count: usize,
    pub in_progress_count: usize,
    pub closed_count: usize,
    pub epic_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub percent_closed: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskGraphIssue {
    pub issue_type: String,
    pub issue_id: String,
    pub depends_on_id: Option<String>,
    pub edge_type: Option<String>,
    pub detail: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskCriticalPath {
    pub length: usize,
    pub root_task_id: Option<String>,
    pub terminal_task_id: Option<String>,
    pub release_1_contract_steps: Vec<TaskRelease1ContractStep>,
    pub nodes: Vec<TaskCriticalPathNode>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskRelease1ContractStep {
    pub id: String,
    pub mode: String,
    pub blocker_code: String,
    pub next_action: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskCriticalPathNode {
    pub id: String,
    pub title: String,
    pub status: String,
    pub issue_type: String,
    pub priority: u32,
}

#[derive(Debug)]
pub struct TaskImportSummary {
    pub source_path: String,
    pub imported_count: usize,
    pub unchanged_count: usize,
    pub updated_count: usize,
}

impl TaskImportSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{} imported, {} unchanged, {} updated from {}",
            self.imported_count, self.unchanged_count, self.updated_count, self.source_path
        )
    }
}

#[derive(Debug)]
pub struct TaskStoreSummary {
    pub total_count: usize,
    pub open_count: usize,
    pub in_progress_count: usize,
    pub closed_count: usize,
    pub epic_count: usize,
    pub ready_count: usize,
}

impl TaskStoreSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{} total, {} open, {} in_progress, {} closed, {} epics, {} ready",
            self.total_count,
            self.open_count,
            self.in_progress_count,
            self.closed_count,
            self.epic_count,
            self.ready_count
        )
    }
}

impl From<TaskJsonlRecord> for TaskContent {
    fn from(value: TaskJsonlRecord) -> Self {
        Self {
            task_id: value.id,
            display_id: value.display_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            execution_semantics: value.execution_semantics,
            dependencies: value
                .dependencies
                .into_iter()
                .map(TaskDependencyRecord::from)
                .collect(),
        }
    }
}

impl From<TaskContent> for TaskStorageRow {
    fn from(value: TaskContent) -> Self {
        Self {
            task_id: value.task_id,
            display_id: value.display_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            execution_semantics: value.execution_semantics,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskStorageRow> for TaskRecord {
    fn from(value: TaskStorageRow) -> Self {
        Self {
            id: value.task_id,
            display_id: value.display_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            execution_semantics: value.execution_semantics,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskRecord> for TaskStorageRow {
    fn from(value: TaskRecord) -> Self {
        Self {
            task_id: value.id,
            display_id: value.display_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            execution_semantics: value.execution_semantics,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskDependencyJsonlRecord> for TaskDependencyRecord {
    fn from(value: TaskDependencyJsonlRecord) -> Self {
        Self {
            issue_id: value.issue_id,
            depends_on_id: value.depends_on_id,
            edge_type: value.edge_type,
            created_at: value.created_at,
            created_by: value.created_by,
            metadata: value.metadata,
            thread_id: value.thread_id,
        }
    }
}
