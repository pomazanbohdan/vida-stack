use serde::{Deserialize, Serialize};
use taskflow_core::{IssueType, TaskId, TaskStatus, Timestamp};

pub mod artifact_kind;
pub mod blocker_code;
pub mod decision_table;
pub mod status_code;

pub use artifact_kind::{ArtifactKind, UnknownArtifactKind};
pub use blocker_code::{
    BlockerCode, UnknownBlockerCode, blocker_code_list_preserving_legacy,
    canonical_blocker_code_list, canonical_blocker_code_str, canonical_blocker_code_value_from_str,
    is_selected_lane_assignment_guard_blocked, is_selected_lane_runtime_assignment_truth_missing,
    selected_lane_assignment_guard_blocked, selected_lane_runtime_assignment_truth_missing,
};
pub use decision_table::{
    DECISION_TABLE_SCHEMA_VERSION, DecisionTableCondition, DecisionTableEvaluationRequest,
    DecisionTableEvaluationResponse, DecisionTableEvaluationStatus, DecisionTableInput,
    DecisionTableOperator, DecisionTableOutput, DecisionTableRule, DecisionTableValue,
    TRANSITION_CONTRACT_SCHEMA_VERSION, TransitionContractBlocker, TransitionContractBlockerCode,
    TransitionContractDecision, TransitionContractOutcome, TransitionContractStatus,
    TransitionContractStatusCode,
};
pub use status_code::{
    ApprovalStatus, LaneStatus, Release1ContractStatus, UnknownStatusCode,
    canonical_approval_status_str, canonical_lane_status_str,
    canonical_release1_contract_status_str, release1_contract_status_str,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub issue_type: IssueType,
    pub updated_at: Timestamp,
}

impl TaskRecord {
    #[must_use]
    pub fn new(id: TaskId, title: impl Into<String>, issue_type: IssueType) -> Self {
        Self {
            id,
            title: title.into(),
            status: TaskStatus::Open,
            issue_type,
            updated_at: Timestamp::now_utc(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemKind {
    pub schema_version: u32,
    pub canonical_issue_type: String,
    pub original_issue_type: String,
    pub provider_issue_type: Option<String>,
    pub category: String,
    pub parent_required: bool,
    pub flow_bindable: bool,
    pub default_flow_binding: String,
    pub source_tiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemProviderMapping {
    pub schema_version: u32,
    pub provider: String,
    pub external_id: String,
    pub external_url: Option<String>,
    pub external_parent_id: Option<String>,
    pub provider_issue_type: Option<String>,
    pub provider_status: Option<String>,
    pub provider_priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub issue_id: TaskId,
    pub depends_on_id: TaskId,
    pub dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::TaskRecord;
    use taskflow_core::{IssueType, TaskId, TaskStatus};

    #[test]
    fn task_record_defaults_to_open() {
        let record = TaskRecord::new(TaskId::new("vida-rf1"), "program", IssueType::Epic);
        assert!(matches!(record.status, TaskStatus::Open));
    }
}
