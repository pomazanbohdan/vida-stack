use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MODULE: &str = "role_step";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleStepFlowDefinition {
    pub flow_id: String,
    pub schema_hash: String,
    pub steps: Vec<RoleStepDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleStepDefinition {
    pub role_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub lifecycle_stage: String,
    pub proof_gate: Option<String>,
    pub closes_workflow: bool,
}

impl RoleStepDefinition {
    #[must_use]
    pub fn task_role_step(&self) -> TaskRoleStep {
        TaskRoleStep {
            role_id: self.role_id.clone(),
            runtime_role: self.runtime_role.clone(),
            task_class: self.task_class.clone(),
            lifecycle_stage: self.lifecycle_stage.clone(),
            closes_workflow: self.closes_workflow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRoleStep {
    pub role_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub lifecycle_stage: String,
    pub closes_workflow: bool,
}

impl TaskRoleStep {
    #[must_use]
    pub fn new(
        role_id: impl Into<String>,
        runtime_role: impl Into<String>,
        task_class: impl Into<String>,
        lifecycle_stage: impl Into<String>,
    ) -> Self {
        Self {
            role_id: normalize_step_token(role_id.into()),
            runtime_role: normalize_step_token(runtime_role.into()),
            task_class: normalize_step_token(task_class.into()),
            lifecycle_stage: normalize_step_token(lifecycle_stage.into()),
            closes_workflow: false,
        }
    }

    #[must_use]
    pub fn closing(mut self) -> Self {
        self.closes_workflow = true;
        self
    }

    #[must_use]
    pub fn state_name(&self) -> String {
        format!("role_{}", self.role_id)
    }

    #[must_use]
    pub fn planning() -> Self {
        Self::new("planning", "business_analyst", "specification", "planning")
    }

    #[must_use]
    pub fn developer() -> Self {
        Self::new("developer", "worker", "implementation", "developer")
    }

    #[must_use]
    pub fn tester() -> Self {
        Self::new("tester", "verifier", "verification", "tester")
    }

    #[must_use]
    pub fn closure() -> Self {
        Self::new("closure", "prover", "release_readiness", "closure").closing()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRoleStepStatus {
    Pending,
    Ready,
    PacketPlanned,
    PacketReady,
    Dispatched,
    Running,
    ResultReceived,
    Validating,
    Accepted,
    Rework,
    Blocked,
    Completed,
    FlowVersionDrift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRoleStepState {
    pub flow_id: String,
    pub flow_schema_hash: String,
    pub step_index: usize,
    pub role_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub lifecycle_stage: String,
    pub status: TaskRoleStepStatus,
    pub outcome: Option<String>,
    pub proof_gate: Option<String>,
    pub packet_ref: Option<String>,
    pub receipt_ref: Option<String>,
    pub attempt_ref: Option<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoleStepStateError {
    #[error("flow has no steps")]
    EmptyFlow,
    #[error("allowed_next_node `{0}` is not present in flow `{1}`")]
    UnknownAllowedNextNode(String, String),
    #[error("flow version drift: expected `{expected}`, got `{actual}`")]
    FlowVersionDrift { expected: String, actual: String },
    #[error("cannot transition from terminal role-step state `{0:?}`")]
    TerminalState(TaskRoleStepStatus),
}

impl TaskRoleStepState {
    pub fn from_first_step(flow: &RoleStepFlowDefinition) -> Result<Self, RoleStepStateError> {
        Self::from_step(flow, 0)
    }

    pub fn from_step(
        flow: &RoleStepFlowDefinition,
        step_index: usize,
    ) -> Result<Self, RoleStepStateError> {
        let step = flow
            .steps
            .get(step_index)
            .ok_or(RoleStepStateError::EmptyFlow)?;
        Ok(Self {
            flow_id: flow.flow_id.clone(),
            flow_schema_hash: flow.schema_hash.clone(),
            step_index,
            role_id: step.role_id.clone(),
            runtime_role: step.runtime_role.clone(),
            task_class: step.task_class.clone(),
            lifecycle_stage: step.lifecycle_stage.clone(),
            status: TaskRoleStepStatus::Pending,
            outcome: None,
            proof_gate: step.proof_gate.clone(),
            packet_ref: None,
            receipt_ref: None,
            attempt_ref: None,
            blockers: Vec::new(),
        })
    }

    pub fn mark_ready(&mut self) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.status = TaskRoleStepStatus::Ready;
        Ok(())
    }

    pub fn plan_packet(&mut self, packet_ref: impl Into<String>) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.packet_ref = Some(packet_ref.into());
        self.status = TaskRoleStepStatus::PacketPlanned;
        Ok(())
    }

    pub fn mark_packet_ready(&mut self) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.status = TaskRoleStepStatus::PacketReady;
        Ok(())
    }

    pub fn dispatch(&mut self, attempt_ref: impl Into<String>) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.attempt_ref = Some(attempt_ref.into());
        self.status = TaskRoleStepStatus::Dispatched;
        Ok(())
    }

    pub fn receive_result(
        &mut self,
        receipt_ref: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.receipt_ref = Some(receipt_ref.into());
        self.outcome = Some(outcome.into());
        self.status = TaskRoleStepStatus::ResultReceived;
        Ok(())
    }

    pub fn validate(&mut self) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.status = TaskRoleStepStatus::Validating;
        Ok(())
    }

    pub fn block(&mut self, blocker: impl Into<String>) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.blockers.push(blocker.into());
        self.status = TaskRoleStepStatus::Blocked;
        Ok(())
    }

    pub fn accept_next(
        &self,
        flow: &RoleStepFlowDefinition,
        allowed_next_node: &str,
        current_flow_hash: &str,
    ) -> Result<Self, RoleStepStateError> {
        if self.flow_schema_hash != current_flow_hash {
            let mut drifted = self.clone();
            drifted.status = TaskRoleStepStatus::FlowVersionDrift;
            return Err(RoleStepStateError::FlowVersionDrift {
                expected: self.flow_schema_hash.clone(),
                actual: current_flow_hash.to_string(),
            });
        }
        let next_index = flow
            .steps
            .iter()
            .position(|step| step.role_id == allowed_next_node)
            .ok_or_else(|| {
                RoleStepStateError::UnknownAllowedNextNode(
                    allowed_next_node.to_string(),
                    flow.flow_id.clone(),
                )
            })?;
        let mut next = Self::from_step(flow, next_index)?;
        next.status = TaskRoleStepStatus::Accepted;
        Ok(next)
    }

    pub fn complete(&mut self) -> Result<(), RoleStepStateError> {
        self.ensure_mutable()?;
        self.status = TaskRoleStepStatus::Completed;
        Ok(())
    }

    fn ensure_mutable(&self) -> Result<(), RoleStepStateError> {
        if matches!(
            self.status,
            TaskRoleStepStatus::Completed | TaskRoleStepStatus::FlowVersionDrift
        ) {
            return Err(RoleStepStateError::TerminalState(self.status));
        }
        Ok(())
    }

    #[must_use]
    pub fn task_role_step(&self) -> TaskRoleStep {
        TaskRoleStep {
            role_id: self.role_id.clone(),
            runtime_role: self.runtime_role.clone(),
            task_class: self.task_class.clone(),
            lifecycle_stage: self.lifecycle_stage.clone(),
            closes_workflow: self
                .proof_gate
                .as_deref()
                .is_some_and(|gate| gate == "closure" || gate == "release_readiness"),
        }
    }
}

fn normalize_step_token(value: String) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(flow_id: &str) -> RoleStepFlowDefinition {
        RoleStepFlowDefinition {
            flow_id: flow_id.to_string(),
            schema_hash: format!("{flow_id}-hash-1"),
            steps: vec![
                RoleStepDefinition {
                    role_id: "analyst".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "analysis".to_string(),
                    lifecycle_stage: "analysis".to_string(),
                    proof_gate: None,
                    closes_workflow: false,
                },
                RoleStepDefinition {
                    role_id: "developer".to_string(),
                    runtime_role: "implementation".to_string(),
                    task_class: "implementation".to_string(),
                    lifecycle_stage: "implementation".to_string(),
                    proof_gate: Some("implementation_receipt".to_string()),
                    closes_workflow: false,
                },
                RoleStepDefinition {
                    role_id: "tester".to_string(),
                    runtime_role: "verification".to_string(),
                    task_class: "verification".to_string(),
                    lifecycle_stage: "verification".to_string(),
                    proof_gate: Some("test_report".to_string()),
                    closes_workflow: false,
                },
            ],
        }
    }

    #[test]
    fn golden_state_progression_covers_task_defect_runtime_defect_and_architecture_flows() {
        for flow_id in ["task", "defect", "runtime_defect", "architecture"] {
            let flow = flow(flow_id);
            let mut state = TaskRoleStepState::from_first_step(&flow).unwrap();
            state.mark_ready().unwrap();
            state.plan_packet(format!("{flow_id}-packet")).unwrap();
            state.mark_packet_ready().unwrap();
            state.dispatch(format!("{flow_id}-attempt")).unwrap();
            state
                .receive_result(format!("{flow_id}-receipt"), "passed")
                .unwrap();
            state.validate().unwrap();
            let next = state
                .accept_next(&flow, "developer", &flow.schema_hash)
                .unwrap();

            assert_eq!(next.flow_id, flow_id);
            assert_eq!(next.role_id, "developer");
            assert_eq!(next.status, TaskRoleStepStatus::Accepted);
            assert_eq!(next.proof_gate.as_deref(), Some("implementation_receipt"));
        }
    }

    #[test]
    fn arbitrary_allowed_next_node_is_rejected() {
        let flow = flow("task");
        let state = TaskRoleStepState::from_first_step(&flow).unwrap();

        let error = state
            .accept_next(&flow, "unconfigured_role", &flow.schema_hash)
            .expect_err("unconfigured role must fail");

        assert_eq!(
            error,
            RoleStepStateError::UnknownAllowedNextNode(
                "unconfigured_role".to_string(),
                "task".to_string()
            )
        );
    }

    #[test]
    fn config_drift_migration_test_reports_explicit_state() {
        let flow = flow("task");
        let state = TaskRoleStepState::from_first_step(&flow).unwrap();

        let error = state
            .accept_next(&flow, "developer", "task-hash-2")
            .expect_err("hash drift must fail");

        assert_eq!(
            error,
            RoleStepStateError::FlowVersionDrift {
                expected: "task-hash-1".to_string(),
                actual: "task-hash-2".to_string()
            }
        );
    }
}
