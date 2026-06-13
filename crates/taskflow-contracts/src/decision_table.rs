use serde::{Deserialize, Serialize};
use taskflow_core::TaskId;

pub const DECISION_TABLE_SCHEMA_VERSION: u32 = 1;

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
        DecisionTableOutput, DecisionTableRule, DecisionTableValue,
    };
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
        assert_eq!(request.rules[0].rule_id, "rule.ready");
        assert!(request.rules[0].stop_on_match);
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
        assert_eq!(response.status, DecisionTableEvaluationStatus::Matched);
        assert_eq!(response.matched_rule_ids, vec!["rule.ready"]);
        assert!(response.blocker_codes.is_empty());
    }

    #[test]
    fn blocked_response_is_fail_closed_without_outputs() {
        let response = DecisionTableEvaluationResponse::blocked(
            "taskflow.route",
            Some(TaskId::new("task-1")),
            vec!["missing_required_input".to_string()],
        );

        assert_eq!(response.status, DecisionTableEvaluationStatus::Blocked);
        assert!(response.matched_rule_ids.is_empty());
        assert!(response.outputs.is_empty());
        assert_eq!(response.blocker_codes, vec!["missing_required_input"]);
    }
}
