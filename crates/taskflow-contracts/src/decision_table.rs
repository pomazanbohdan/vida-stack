use serde::{Deserialize, Serialize};
use taskflow_core::TaskId;

pub const DECISION_TABLE_SCHEMA_VERSION: u32 = 1;
pub const TRANSITION_CONTRACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableEvaluationRequest {
    pub schema_version: u32,
    pub table_id: String,
    pub task_id: Option<TaskId>,
    pub inputs: Vec<DecisionTableInput>,
    pub rules: Vec<DecisionTableRule>,
}

impl DecisionTableEvaluationRequest {
    #[must_use]
    pub fn new(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        inputs: Vec<DecisionTableInput>,
        rules: Vec<DecisionTableRule>,
    ) -> Self {
        Self {
            schema_version: DECISION_TABLE_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            inputs,
            rules,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableEvaluationResponse {
    pub schema_version: u32,
    pub table_id: String,
    pub task_id: Option<TaskId>,
    pub status: DecisionTableEvaluationStatus,
    pub matched_rule_ids: Vec<String>,
    pub outputs: Vec<DecisionTableOutput>,
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionContractDecision {
    pub schema_version: u32,
    pub table_id: String,
    pub task_id: Option<TaskId>,
    pub outcome: TransitionContractOutcome,
    pub status: TransitionContractStatus,
    pub blocker_codes: Vec<TransitionContractBlocker>,
    pub notes: Vec<String>,
}

impl TransitionContractDecision {
    #[must_use]
    pub fn admitted(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        notes: Vec<String>,
    ) -> Self {
        Self {
            schema_version: TRANSITION_CONTRACT_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            outcome: TransitionContractOutcome::Admitted,
            status: TransitionContractStatus::known(TransitionContractStatusCode::Admitted),
            blocker_codes: Vec::new(),
            notes,
        }
    }

    #[must_use]
    pub fn rejected(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        blocker_codes: Vec<TransitionContractBlocker>,
    ) -> Self {
        Self {
            schema_version: TRANSITION_CONTRACT_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            outcome: TransitionContractOutcome::Rejected,
            status: TransitionContractStatus::known(TransitionContractStatusCode::Rejected),
            blocker_codes,
            notes: Vec::new(),
        }
    }

    #[must_use]
    pub fn blocked(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        blocker_codes: Vec<TransitionContractBlocker>,
    ) -> Self {
        Self {
            schema_version: TRANSITION_CONTRACT_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            outcome: TransitionContractOutcome::Blocked,
            status: TransitionContractStatus::known(TransitionContractStatusCode::Blocked),
            blocker_codes,
            notes: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_fail_closed_blocked(&self) -> bool {
        self.outcome == TransitionContractOutcome::Blocked
            && self.status.as_str() == TransitionContractStatusCode::Blocked.as_str()
            && !self.blocker_codes.is_empty()
    }
}

impl DecisionTableEvaluationResponse {
    #[must_use]
    pub fn matched(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        matched_rule_ids: Vec<String>,
        outputs: Vec<DecisionTableOutput>,
    ) -> Self {
        Self {
            schema_version: DECISION_TABLE_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            status: DecisionTableEvaluationStatus::Matched,
            matched_rule_ids,
            outputs,
            blocker_codes: Vec::new(),
        }
    }

    #[must_use]
    pub fn blocked(
        table_id: impl Into<String>,
        task_id: Option<TaskId>,
        blocker_codes: Vec<String>,
    ) -> Self {
        Self {
            schema_version: DECISION_TABLE_SCHEMA_VERSION,
            table_id: table_id.into(),
            task_id,
            status: DecisionTableEvaluationStatus::Blocked,
            matched_rule_ids: Vec::new(),
            outputs: Vec::new(),
            blocker_codes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionContractOutcome {
    Admitted,
    Rejected,
    Blocked,
}

impl TransitionContractOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Rejected => "rejected",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionContractStatusCode {
    Admitted,
    Rejected,
    Blocked,
}

impl TransitionContractStatusCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Rejected => "rejected",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransitionContractStatus {
    value: String,
}

impl TransitionContractStatus {
    #[must_use]
    pub fn known(status: TransitionContractStatusCode) -> Self {
        Self {
            value: status.as_str().to_string(),
        }
    }

    #[must_use]
    pub fn legacy_passthrough(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed != value {
            return None;
        }
        Some(Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionContractBlockerCode {
    MissingRequiredInput,
    InvalidTransition,
    StaleEvidence,
    DuplicateEdge,
}

impl TransitionContractBlockerCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRequiredInput => "missing_required_input",
            Self::InvalidTransition => "invalid_transition",
            Self::StaleEvidence => "stale_evidence",
            Self::DuplicateEdge => "duplicate_edge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransitionContractBlocker {
    value: String,
}

impl TransitionContractBlocker {
    #[must_use]
    pub fn known(blocker: TransitionContractBlockerCode) -> Self {
        Self {
            value: blocker.as_str().to_string(),
        }
    }

    #[must_use]
    pub fn legacy_passthrough(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed != value {
            return None;
        }
        Some(Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableInput {
    pub field: String,
    pub value: DecisionTableValue,
}

impl DecisionTableInput {
    #[must_use]
    pub fn new(field: impl Into<String>, value: DecisionTableValue) -> Self {
        Self {
            field: field.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableRule {
    pub rule_id: String,
    pub priority: u32,
    pub conditions: Vec<DecisionTableCondition>,
    pub outputs: Vec<DecisionTableOutput>,
    pub stop_on_match: bool,
}

impl DecisionTableRule {
    #[must_use]
    pub fn new(
        rule_id: impl Into<String>,
        priority: u32,
        conditions: Vec<DecisionTableCondition>,
        outputs: Vec<DecisionTableOutput>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            priority,
            conditions,
            outputs,
            stop_on_match: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableCondition {
    pub field: String,
    pub operator: DecisionTableOperator,
    pub expected: DecisionTableValue,
}

impl DecisionTableCondition {
    #[must_use]
    pub fn equals(field: impl Into<String>, expected: DecisionTableValue) -> Self {
        Self {
            field: field.into(),
            operator: DecisionTableOperator::Equals,
            expected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTableOutput {
    pub field: String,
    pub value: DecisionTableValue,
}

impl DecisionTableOutput {
    #[must_use]
    pub fn new(field: impl Into<String>, value: DecisionTableValue) -> Self {
        Self {
            field: field.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTableOperator {
    Equals,
    NotEquals,
    Exists,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DecisionTableValue {
    String(String),
    Boolean(bool),
    Integer(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTableEvaluationStatus {
    Matched,
    NoMatch,
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::{
        DECISION_TABLE_SCHEMA_VERSION, DecisionTableCondition, DecisionTableEvaluationRequest,
        DecisionTableEvaluationResponse, DecisionTableEvaluationStatus, DecisionTableInput,
        DecisionTableOperator, DecisionTableOutput, DecisionTableRule, DecisionTableValue,
        TRANSITION_CONTRACT_SCHEMA_VERSION, TransitionContractBlocker,
        TransitionContractBlockerCode, TransitionContractDecision, TransitionContractOutcome,
        TransitionContractStatus, TransitionContractStatusCode,
    };
    use serde_json::json;
    use taskflow_core::TaskId;

    #[test]
    fn request_schema_version_is_pinned() {
        let rule = DecisionTableRule::new(
            "rule.ready",
            10,
            vec![DecisionTableCondition::equals(
                "task.status",
                DecisionTableValue::String("open".to_string()),
            )],
            vec![DecisionTableOutput::new(
                "route",
                DecisionTableValue::String("dispatch".to_string()),
            )],
        );
        let request = DecisionTableEvaluationRequest::new(
            "taskflow.route",
            Some(TaskId::new("task-1")),
            vec![DecisionTableInput::new(
                "task.status",
                DecisionTableValue::String("open".to_string()),
            )],
            vec![rule],
        );

        assert_eq!(request.schema_version, DECISION_TABLE_SCHEMA_VERSION);
        assert_eq!(request.table_id, "taskflow.route");
        assert_eq!(request.task_id, Some(TaskId::new("task-1")));
        assert_eq!(request.inputs.len(), 1);
        assert_eq!(request.inputs[0].field, "task.status");
        assert_eq!(
            request.inputs[0].value,
            DecisionTableValue::String("open".to_string())
        );
        assert_eq!(request.rules[0].rule_id, "rule.ready");
        assert_eq!(request.rules[0].priority, 10);
        assert_eq!(request.rules[0].conditions.len(), 1);
        assert_eq!(request.rules[0].conditions[0].field, "task.status");
        assert_eq!(
            request.rules[0].conditions[0].operator,
            DecisionTableOperator::Equals
        );
        assert_eq!(
            request.rules[0].conditions[0].expected,
            DecisionTableValue::String("open".to_string())
        );
        assert_eq!(request.rules[0].outputs.len(), 1);
        assert_eq!(request.rules[0].outputs[0].field, "route");
        assert_eq!(
            request.rules[0].outputs[0].value,
            DecisionTableValue::String("dispatch".to_string())
        );
        assert!(request.rules[0].stop_on_match);
    }

    #[test]
    fn request_json_round_trip_preserves_optional_task_and_values() {
        let request = DecisionTableEvaluationRequest::new(
            "taskflow.route",
            None,
            vec![
                DecisionTableInput::new("enabled", DecisionTableValue::Boolean(true)),
                DecisionTableInput::new("attempt", DecisionTableValue::Integer(2)),
            ],
            Vec::new(),
        );

        let encoded = serde_json::to_value(&request).expect("request serializes");
        let decoded: DecisionTableEvaluationRequest =
            serde_json::from_value(encoded).expect("request deserializes");

        assert_eq!(decoded, request);
        assert!(decoded.task_id.is_none());
    }

    #[test]
    fn matched_response_carries_rule_and_output_boundary() {
        let response = DecisionTableEvaluationResponse::matched(
            "taskflow.route",
            Some(TaskId::new("task-1")),
            vec!["rule.ready".to_string()],
            vec![DecisionTableOutput::new(
                "route",
                DecisionTableValue::String("dispatch".to_string()),
            )],
        );

        assert_eq!(response.schema_version, DECISION_TABLE_SCHEMA_VERSION);
        assert_eq!(response.table_id, "taskflow.route");
        assert_eq!(response.task_id, Some(TaskId::new("task-1")));
        assert_eq!(response.status, DecisionTableEvaluationStatus::Matched);
        assert_eq!(response.matched_rule_ids, vec!["rule.ready"]);
        assert_eq!(response.outputs.len(), 1);
        assert_eq!(response.outputs[0].field, "route");
        assert_eq!(
            response.outputs[0].value,
            DecisionTableValue::String("dispatch".to_string())
        );
        assert!(response.blocker_codes.is_empty());
    }

    #[test]
    fn blocked_response_is_fail_closed_without_outputs() {
        let response = DecisionTableEvaluationResponse::blocked(
            "taskflow.route",
            Some(TaskId::new("task-1")),
            vec!["missing_required_input".to_string()],
        );

        assert_eq!(response.table_id, "taskflow.route");
        assert_eq!(response.task_id, Some(TaskId::new("task-1")));
        assert_eq!(response.status, DecisionTableEvaluationStatus::Blocked);
        assert!(response.matched_rule_ids.is_empty());
        assert!(response.outputs.is_empty());
        assert_eq!(response.blocker_codes, vec!["missing_required_input"]);
    }

    #[test]
    fn transition_contract_admitted_decision_has_stable_schema() {
        let decision = TransitionContractDecision::admitted(
            "task.lifecycle",
            Some(TaskId::new("task-1")),
            vec!["golden behavior preserved".to_string()],
        );
        let encoded = serde_json::to_value(&decision).expect("decision serializes");

        assert_eq!(decision.schema_version, TRANSITION_CONTRACT_SCHEMA_VERSION);
        assert_eq!(decision.outcome, TransitionContractOutcome::Admitted);
        assert_eq!(decision.status.as_str(), "admitted");
        assert!(decision.blocker_codes.is_empty());
        assert_eq!(
            encoded,
            json!({
                "schema_version": 1,
                "table_id": "task.lifecycle",
                "task_id": "task-1",
                "outcome": "admitted",
                "status": "admitted",
                "blocker_codes": [],
                "notes": ["golden behavior preserved"]
            })
        );
    }

    #[test]
    fn transition_contract_rejected_decision_preserves_blockers_without_blocked_status() {
        let decision = TransitionContractDecision::rejected(
            "task.lifecycle",
            Some(TaskId::new("task-1")),
            vec![TransitionContractBlocker::known(
                TransitionContractBlockerCode::InvalidTransition,
            )],
        );

        assert_eq!(decision.table_id, "task.lifecycle");
        assert_eq!(decision.task_id, Some(TaskId::new("task-1")));
        assert_eq!(decision.outcome, TransitionContractOutcome::Rejected);
        assert_eq!(decision.status.as_str(), "rejected");
        assert_eq!(decision.blocker_codes[0].as_str(), "invalid_transition");
        assert!(decision.notes.is_empty());
        assert!(!decision.is_fail_closed_blocked());
    }

    #[test]
    fn transition_contract_blocked_decision_is_fail_closed() {
        let decision = TransitionContractDecision::blocked(
            "task.lifecycle",
            Some(TaskId::new("task-1")),
            vec![TransitionContractBlocker::known(
                TransitionContractBlockerCode::MissingRequiredInput,
            )],
        );

        assert_eq!(decision.table_id, "task.lifecycle");
        assert_eq!(decision.task_id, Some(TaskId::new("task-1")));
        assert!(decision.is_fail_closed_blocked());
        assert_eq!(decision.outcome.as_str(), "blocked");
        assert_eq!(decision.status.as_str(), "blocked");
        assert_eq!(decision.blocker_codes[0].as_str(), "missing_required_input");
    }

    #[test]
    fn transition_contract_fail_closed_check_requires_outcome_status_and_blocker() {
        let mut decision = TransitionContractDecision::blocked(
            "task.lifecycle",
            Some(TaskId::new("task-1")),
            vec![TransitionContractBlocker::known(
                TransitionContractBlockerCode::MissingRequiredInput,
            )],
        );

        assert!(decision.is_fail_closed_blocked());

        decision.blocker_codes.clear();
        assert!(!decision.is_fail_closed_blocked());

        decision
            .blocker_codes
            .push(TransitionContractBlocker::known(
                TransitionContractBlockerCode::MissingRequiredInput,
            ));
        decision.status = TransitionContractStatus::known(TransitionContractStatusCode::Rejected);
        assert!(!decision.is_fail_closed_blocked());

        decision.status = TransitionContractStatus::known(TransitionContractStatusCode::Blocked);
        decision.outcome = TransitionContractOutcome::Rejected;
        assert!(!decision.is_fail_closed_blocked());
    }

    #[test]
    fn transition_contract_preserves_unknown_legacy_strings() {
        let status = TransitionContractStatus::legacy_passthrough("legacy_waiting")
            .expect("legacy status passes through");
        let blocker = TransitionContractBlocker::legacy_passthrough("legacy_blocker")
            .expect("legacy blocker passes through");

        assert_eq!(status.as_str(), "legacy_waiting");
        assert_eq!(blocker.as_str(), "legacy_blocker");
        assert!(TransitionContractStatus::legacy_passthrough(" legacy_waiting ").is_none());
        assert!(TransitionContractBlocker::legacy_passthrough("").is_none());
        assert_eq!(
            TransitionContractStatus::known(TransitionContractStatusCode::Rejected).as_str(),
            "rejected"
        );
    }

    #[test]
    fn transition_contract_matrix_preserves_outcome_status_and_legacy_edges() {
        let cases = [
            (
                TransitionContractDecision::admitted(
                    "task.lifecycle",
                    Some(TaskId::new("task-1")),
                    vec!["accepted".to_string()],
                ),
                TransitionContractOutcome::Admitted,
                "admitted",
                false,
            ),
            (
                TransitionContractDecision::rejected(
                    "task.lifecycle",
                    Some(TaskId::new("task-1")),
                    vec![TransitionContractBlocker::known(
                        TransitionContractBlockerCode::InvalidTransition,
                    )],
                ),
                TransitionContractOutcome::Rejected,
                "rejected",
                false,
            ),
            (
                TransitionContractDecision::blocked(
                    "task.lifecycle",
                    Some(TaskId::new("task-1")),
                    vec![TransitionContractBlocker::known(
                        TransitionContractBlockerCode::MissingRequiredInput,
                    )],
                ),
                TransitionContractOutcome::Blocked,
                "blocked",
                true,
            ),
        ];

        for (decision, outcome, status, fail_closed) in cases {
            assert_eq!(decision.schema_version, TRANSITION_CONTRACT_SCHEMA_VERSION);
            assert_eq!(decision.outcome, outcome);
            assert_eq!(decision.status.as_str(), status);
            assert_eq!(decision.is_fail_closed_blocked(), fail_closed);
        }

        for value in ["legacy_waiting", "legacy:custom", "UPSTREAM_STATUS"] {
            assert_eq!(
                TransitionContractStatus::legacy_passthrough(value)
                    .expect("non-empty exact legacy status is accepted")
                    .as_str(),
                value
            );
            assert_eq!(
                TransitionContractBlocker::legacy_passthrough(value)
                    .expect("non-empty exact legacy blocker is accepted")
                    .as_str(),
                value
            );
        }

        for value in ["", " trimmed ", "\tindent"] {
            assert!(TransitionContractStatus::legacy_passthrough(value).is_none());
            assert!(TransitionContractBlocker::legacy_passthrough(value).is_none());
        }
    }
}
