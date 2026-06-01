use super::*;
use serde::Deserialize;

pub const WORK_ITEM_TAXONOMY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkItemCategory {
    ProgramContainer,
    Delivery,
    Defect,
    Review,
    Architecture,
    Release,
    Operations,
    Process,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkItemTaxonomyEntry {
    pub canonical_issue_type: &'static str,
    pub aliases: &'static [&'static str],
    pub category: WorkItemCategory,
    pub parent_required: bool,
    pub flow_bindable: bool,
    pub default_flow_binding: &'static str,
    pub source_tiers: &'static [&'static str],
}

pub const WORK_ITEM_TAXONOMY: &[WorkItemTaxonomyEntry] = &[
    WorkItemTaxonomyEntry {
        canonical_issue_type: "epic",
        aliases: &[],
        category: WorkItemCategory::ProgramContainer,
        parent_required: false,
        flow_bindable: true,
        default_flow_binding: "default_delivery",
        source_tiers: &["operator_request", "planning"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "task",
        aliases: &[],
        category: WorkItemCategory::Delivery,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "default_delivery",
        source_tiers: &["operator_request", "planned_delivery"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "defect",
        aliases: &["bug"],
        category: WorkItemCategory::Defect,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "defect_repair_verified",
        source_tiers: &["runtime_status", "test_failure", "operator_report"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "runtime_defect",
        aliases: &[],
        category: WorkItemCategory::Defect,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "runtime_defect_remediation",
        source_tiers: &["runtime_status", "downstream_runtime_report"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "pull_request",
        aliases: &["pr"],
        category: WorkItemCategory::Review,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "pr_repair_verified",
        source_tiers: &["pull_request"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "pr_repair",
        aliases: &[],
        category: WorkItemCategory::Review,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "pr_repair_verified",
        source_tiers: &["pull_request", "ci_failure"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "architecture",
        aliases: &["spike"],
        category: WorkItemCategory::Architecture,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "architecture_design",
        source_tiers: &["operator_request", "architecture_review"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "release_readiness",
        aliases: &[],
        category: WorkItemCategory::Release,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "release_readiness_gate",
        source_tiers: &["release_check", "ci_failure"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "service_tui",
        aliases: &[],
        category: WorkItemCategory::Delivery,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "service_tui_orchestration",
        source_tiers: &["operator_request", "planned_delivery"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "internal_agent_development",
        aliases: &[],
        category: WorkItemCategory::Operations,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "hook_enabled_internal_agent_development",
        source_tiers: &["runtime_status", "operator_request"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "ci_failure",
        aliases: &[],
        category: WorkItemCategory::Defect,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "defect_repair_verified",
        source_tiers: &["ci_failure"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "optimization",
        aliases: &[],
        category: WorkItemCategory::Process,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "default_delivery",
        source_tiers: &["operator_friction", "self_diagnostic"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "documentation_process",
        aliases: &["documentation"],
        category: WorkItemCategory::Process,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "default_delivery",
        source_tiers: &["documentation_review", "operator_request"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "operator_surface_gap",
        aliases: &[],
        category: WorkItemCategory::Operations,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "runtime_defect_remediation",
        source_tiers: &["operator_friction", "runtime_status"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "external_downstream_report",
        aliases: &["downstream_report"],
        category: WorkItemCategory::Defect,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "runtime_defect_remediation",
        source_tiers: &["downstream_runtime_report", "operator_report"],
    },
    WorkItemTaxonomyEntry {
        canonical_issue_type: "debug",
        aliases: &[],
        category: WorkItemCategory::Operations,
        parent_required: true,
        flow_bindable: true,
        default_flow_binding: "debug_fast",
        source_tiers: &["operator_request", "runtime_status"],
    },
];

pub fn normalize_work_item_issue_type(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| match ch {
            ' ' | '-' => '_',
            _ => ch,
        })
        .collect()
}

pub fn work_item_taxonomy_entry(issue_type: &str) -> Option<&'static WorkItemTaxonomyEntry> {
    let normalized = normalize_work_item_issue_type(issue_type);
    WORK_ITEM_TAXONOMY.iter().find(|entry| {
        entry.canonical_issue_type == normalized
            || entry
                .aliases
                .iter()
                .any(|alias| normalize_work_item_issue_type(alias) == normalized)
    })
}

pub fn work_item_requires_parent(issue_type: &str) -> bool {
    work_item_taxonomy_entry(issue_type)
        .map(|entry| entry.parent_required)
        .unwrap_or(true)
}

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

#[derive(Debug, Default, serde::Serialize, SurrealValue, Clone, PartialEq, Eq)]
pub struct TaskPlannerMetadata {
    #[serde(default)]
    pub owned_paths: Vec<String>,
    #[serde(default)]
    pub acceptance_targets: Vec<String>,
    #[serde(default)]
    pub proof_targets: Vec<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub estimate: Option<String>,
    #[serde(default)]
    pub lane_hint: Option<String>,
}

#[derive(Deserialize)]
struct TaskPlannerMetadataWire {
    #[serde(default)]
    owned_paths: Option<Vec<String>>,
    #[serde(default)]
    acceptance_targets: Option<Vec<String>>,
    #[serde(default)]
    proof_targets: Option<Vec<String>>,
    #[serde(default)]
    risk: Option<String>,
    #[serde(default)]
    estimate: Option<String>,
    #[serde(default)]
    lane_hint: Option<String>,
}

impl<'de> serde::Deserialize<'de> for TaskPlannerMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <Option<TaskPlannerMetadataWire> as Deserialize>::deserialize(deserializer)?;
        Ok(value
            .map(|wire| Self {
                owned_paths: wire.owned_paths.unwrap_or_default(),
                acceptance_targets: wire.acceptance_targets.unwrap_or_default(),
                proof_targets: wire.proof_targets.unwrap_or_default(),
                risk: wire.risk,
                estimate: wire.estimate,
                lane_hint: wire.lane_hint,
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
    pub(crate) planner_metadata: TaskPlannerMetadata,
    #[serde(default)]
    pub(crate) dependencies: Vec<TaskDependencyJsonlRecord>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskDependencyJsonlRecord {
    pub(crate) issue_id: String,
    pub(crate) depends_on_id: String,
    #[serde(rename = "type", alias = "edge_type")]
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
    #[serde(default)]
    pub(crate) planner_metadata: TaskPlannerMetadata,
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
    #[serde(default)]
    pub(crate) planner_metadata: TaskPlannerMetadata,
    pub(crate) dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub(crate) struct TaskStorageRowStored {
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
    pub(crate) execution_semantics: Option<TaskExecutionSemantics>,
    #[serde(default)]
    pub(crate) planner_metadata: Option<TaskPlannerMetadata>,
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
    #[serde(default)]
    pub planner_metadata: TaskPlannerMetadata,
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
    pub planner_metadata: TaskPlannerMetadata,
    pub created_by: &'a str,
    pub source_repo: &'a str,
}

#[derive(Debug)]
pub struct UpdateTaskRequest<'a> {
    pub task_id: &'a str,
    pub title: Option<&'a str>,
    pub status: Option<&'a str>,
    pub priority: Option<u32>,
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
    pub planner_metadata: Option<TaskPlannerMetadata>,
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
pub struct TaskDefectBatchRehomeResult {
    pub from_parent_id: String,
    pub to_parent_id: String,
    pub requested_child_ids: Vec<String>,
    pub moved_child_ids: Vec<String>,
    pub paused_task_ids: Vec<String>,
    pub started_task_ids: Vec<String>,
    pub moved_count: usize,
    pub paused_count: usize,
    pub started_count: usize,
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
    #[serde(default)]
    pub child_display_id: Option<String>,
    #[serde(default)]
    pub child_title: Option<String>,
    pub child_status: String,
    #[serde(default)]
    pub child_priority: Option<u32>,
    pub child_issue_type: Option<String>,
    #[serde(default)]
    pub child_labels: Vec<String>,
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
            planner_metadata: value.planner_metadata,
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
            planner_metadata: value.planner_metadata,
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
            planner_metadata: value.planner_metadata,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskStorageRowStored> for TaskStorageRow {
    fn from(value: TaskStorageRowStored) -> Self {
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
            execution_semantics: value.execution_semantics.unwrap_or_default(),
            planner_metadata: value.planner_metadata.unwrap_or_default(),
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
            planner_metadata: value.planner_metadata,
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

#[cfg(test)]
mod tests {
    use super::{
        normalize_work_item_issue_type, work_item_requires_parent, work_item_taxonomy_entry,
        TaskPlannerMetadata, TaskStorageRow, WORK_ITEM_TAXONOMY,
    };

    #[test]
    fn work_item_taxonomy_has_unique_canonical_issue_types() {
        let mut seen = std::collections::BTreeSet::new();

        for entry in WORK_ITEM_TAXONOMY {
            assert!(
                seen.insert(entry.canonical_issue_type),
                "duplicate taxonomy entry for {}",
                entry.canonical_issue_type
            );
            assert!(
                !entry.default_flow_binding.trim().is_empty(),
                "taxonomy entry {} must bind a default flow",
                entry.canonical_issue_type
            );
            assert!(
                !entry.source_tiers.is_empty(),
                "taxonomy entry {} must declare source tiers",
                entry.canonical_issue_type
            );
            assert!(
                !entry.flow_bindable || !entry.default_flow_binding.trim().is_empty(),
                "flow-bindable taxonomy entry {} must bind a default flow",
                entry.canonical_issue_type
            );
        }
    }

    #[test]
    fn work_item_taxonomy_normalizes_provider_neutral_issue_types() {
        assert_eq!(
            normalize_work_item_issue_type("Pull Request"),
            "pull_request"
        );
        assert_eq!(
            normalize_work_item_issue_type("runtime-defect"),
            "runtime_defect"
        );
        assert_eq!(
            work_item_taxonomy_entry("PR Repair")
                .expect("pr repair taxonomy")
                .default_flow_binding,
            "pr_repair_verified"
        );
        assert_eq!(
            work_item_taxonomy_entry("bug")
                .expect("bug alias")
                .canonical_issue_type,
            "defect"
        );
        assert_eq!(
            work_item_taxonomy_entry("spike")
                .expect("spike alias")
                .canonical_issue_type,
            "architecture"
        );
    }

    #[test]
    fn work_item_taxonomy_parent_rule_is_fail_closed() {
        assert!(!work_item_requires_parent("epic"));
        assert!(work_item_requires_parent("task"));
        assert!(work_item_requires_parent("unknown_future_type"));
    }

    #[test]
    fn task_planner_metadata_deserializes_from_null_as_default() {
        let metadata: TaskPlannerMetadata =
            serde_json::from_value(serde_json::Value::Null).expect("null planner metadata");

        assert_eq!(metadata, TaskPlannerMetadata::default());
    }

    #[test]
    fn task_storage_row_deserializes_missing_planner_metadata_as_default() {
        let row: TaskStorageRow = serde_json::from_value(serde_json::json!({
            "task_id": "task-1",
            "display_id": null,
            "title": "Task",
            "description": "desc",
            "status": "open",
            "priority": 1,
            "issue_type": "task",
            "created_at": "0",
            "created_by": "test",
            "updated_at": "0",
            "closed_at": null,
            "close_reason": null,
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "notes": null,
            "labels": [],
            "execution_semantics": null,
            "dependencies": []
        }))
        .expect("task storage row should deserialize without planner_metadata");

        assert_eq!(row.planner_metadata, TaskPlannerMetadata::default());
    }

    #[test]
    fn task_storage_row_deserializes_null_planner_metadata_as_default() {
        let row: TaskStorageRow = serde_json::from_value(serde_json::json!({
            "task_id": "task-1",
            "display_id": null,
            "title": "Task",
            "description": "desc",
            "status": "open",
            "priority": 1,
            "issue_type": "task",
            "created_at": "0",
            "created_by": "test",
            "updated_at": "0",
            "closed_at": null,
            "close_reason": null,
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "notes": null,
            "labels": [],
            "execution_semantics": null,
            "planner_metadata": null,
            "dependencies": []
        }))
        .expect("task storage row should deserialize null planner_metadata");

        assert_eq!(row.planner_metadata, TaskPlannerMetadata::default());
    }

    #[test]
    fn task_storage_row_deserializes_planner_metadata_with_null_list_fields_as_default() {
        let row: TaskStorageRow = serde_json::from_value(serde_json::json!({
            "task_id": "task-1",
            "display_id": null,
            "title": "Task",
            "description": "desc",
            "status": "open",
            "priority": 1,
            "issue_type": "task",
            "created_at": "0",
            "created_by": "test",
            "updated_at": "0",
            "closed_at": null,
            "close_reason": null,
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "notes": null,
            "labels": [],
            "execution_semantics": null,
            "planner_metadata": {
                "owned_paths": null,
                "acceptance_targets": null,
                "proof_targets": null,
                "risk": null,
                "estimate": null,
                "lane_hint": null
            },
            "dependencies": []
        }))
        .expect("task storage row should deserialize planner_metadata with null list fields");

        assert_eq!(row.planner_metadata, TaskPlannerMetadata::default());
    }
}
