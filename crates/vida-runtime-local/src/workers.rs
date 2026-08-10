use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::jobs::RetryBackoffPolicy;

pub const WORKER_CLAIM_CONFLICT_BLOCKER: &str = "vida_worker_claim_conflict";
pub const WORKER_RETRY_EXHAUSTED_BLOCKER: &str = "vida_worker_retry_exhausted";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerAutomationConfig {
    pub max_attempts: u64,
    pub base_retry_seconds: u64,
}

impl Default for WorkerAutomationConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_retry_seconds: 15,
        }
    }
}

impl WorkerAutomationConfig {
    pub fn retry_policy(&self) -> RetryBackoffPolicy {
        RetryBackoffPolicy::linear_seconds(self.max_attempts, self.base_retry_seconds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalystCompletionRequest {
    pub run_id: String,
    pub from_role: String,
    pub next_role: String,
    pub idempotency_key: String,
    pub approval_required: bool,
    pub cedar_action: String,
}

impl AnalystCompletionRequest {
    pub fn next_developer_packet(
        run_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            from_role: "analyst".to_string(),
            next_role: "developer".to_string(),
            idempotency_key: idempotency_key.into(),
            approval_required: false,
            cedar_action: "vida.taskflow.materialize_next_packet".to_string(),
        }
    }

    pub fn requiring_approval(mut self) -> Self {
        self.approval_required = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CedarPolicyVerdict {
    pub policy_engine: String,
    pub policy_ref: String,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextRolePacket {
    pub run_id: String,
    pub packet_id: String,
    pub from_role: String,
    pub next_role: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedApprovalState {
    pub run_id: String,
    pub required_role: String,
    pub approval_kind: String,
    pub resume_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationTraceEntry {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationCommandEnvelope {
    pub operation: String,
    pub run_id: String,
    pub idempotency_key: String,
    pub policy_ref: String,
    pub payload: serde_json::Value,
    pub trace: Vec<AutomationTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerClaimConflict {
    pub run_id: String,
    pub active_idempotency_key: String,
    pub attempted_idempotency_key: String,
    pub blocker_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryObservation {
    pub run_id: String,
    pub attempt: u64,
    pub max_attempts: u64,
    pub retry_after_seconds: u64,
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomationWorkerStatus {
    MaterializedNextPacket,
    ApprovalRequired,
    Retrying,
    Paused,
    Conflict,
    PolicyDenied,
    RetryExhausted,
    IdempotentReplay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationWorkerOutcome {
    pub status: AutomationWorkerStatus,
    pub packet: Option<NextRolePacket>,
    pub approval: Option<TypedApprovalState>,
    pub command: Option<AutomationCommandEnvelope>,
    pub conflict: Option<WorkerClaimConflict>,
    pub retry: Option<RetryObservation>,
    pub policy_verdict: CedarPolicyVerdict,
    pub trace: Vec<AutomationTraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AutomationWorkerState {
    pub paused: bool,
    pub active_claims: BTreeMap<String, String>,
    pub completed_packets: BTreeMap<String, NextRolePacket>,
    pub attempts_by_run: BTreeMap<String, u64>,
    pub transient_failures_remaining: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationWorkerRuntime {
    pub config: WorkerAutomationConfig,
    pub state: AutomationWorkerState,
}

impl AutomationWorkerRuntime {
    pub fn new(config: WorkerAutomationConfig) -> Self {
        Self {
            config,
            state: AutomationWorkerState::default(),
        }
    }

    pub fn from_state(config: WorkerAutomationConfig, state: AutomationWorkerState) -> Self {
        Self { config, state }
    }

    pub fn state_snapshot(&self) -> AutomationWorkerState {
        self.state.clone()
    }

    pub fn pause(&mut self) {
        self.state.paused = true;
    }

    pub fn resume(&mut self) {
        self.state.paused = false;
    }

    pub fn inject_transient_failures(&mut self, run_id: impl Into<String>, count: u64) {
        self.state
            .transient_failures_remaining
            .insert(run_id.into(), count);
    }

    pub fn process_analyst_completion(
        &mut self,
        request: AnalystCompletionRequest,
    ) -> AutomationWorkerOutcome {
        let policy_verdict = authorize_next_packet_materialization(&request);

        if self.state.paused {
            return outcome(AutomationWorkerStatus::Paused, policy_verdict);
        }

        if !policy_verdict.allowed {
            return outcome(AutomationWorkerStatus::PolicyDenied, policy_verdict);
        }

        if let Some(packet) = self.state.completed_packets.get(&request.idempotency_key) {
            return AutomationWorkerOutcome {
                status: AutomationWorkerStatus::IdempotentReplay,
                packet: Some(packet.clone()),
                approval: None,
                command: Some(command_envelope(
                    "taskflow.replay_next_packet",
                    &request,
                    &policy_verdict,
                    serde_json::json!({ "packet": packet }),
                )),
                conflict: None,
                retry: None,
                policy_verdict,
                trace: trace_for(
                    "idempotent_replay",
                    "completed packet returned by idempotency key",
                ),
            };
        }

        if request.approval_required {
            return AutomationWorkerOutcome {
                status: AutomationWorkerStatus::ApprovalRequired,
                packet: None,
                approval: Some(TypedApprovalState {
                    run_id: request.run_id.clone(),
                    required_role: "operator".to_string(),
                    approval_kind: "taskflow_next_role_materialization".to_string(),
                    resume_action: "approve_then_materialize_next_packet".to_string(),
                }),
                command: Some(command_envelope(
                    "taskflow.await_operator_approval",
                    &request,
                    &policy_verdict,
                    serde_json::json!({
                        "required_role": "operator",
                        "approval_kind": "taskflow_next_role_materialization",
                        "resume_action": "approve_then_materialize_next_packet"
                    }),
                )),
                conflict: None,
                retry: None,
                policy_verdict,
                trace: trace_for(
                    "approval_required",
                    "typed approval state blocks automatic materialization",
                ),
            };
        }

        if let Some(active_key) = self.state.active_claims.get(&request.run_id) {
            if active_key != &request.idempotency_key {
                return AutomationWorkerOutcome {
                    status: AutomationWorkerStatus::Conflict,
                    packet: None,
                    approval: None,
                    command: None,
                    conflict: Some(WorkerClaimConflict {
                        run_id: request.run_id,
                        active_idempotency_key: active_key.clone(),
                        attempted_idempotency_key: request.idempotency_key,
                        blocker_code: WORKER_CLAIM_CONFLICT_BLOCKER.to_string(),
                    }),
                    retry: None,
                    policy_verdict,
                    trace: trace_for("claim_conflict", WORKER_CLAIM_CONFLICT_BLOCKER),
                };
            }
        } else {
            self.state
                .active_claims
                .insert(request.run_id.clone(), request.idempotency_key.clone());
        }

        if self
            .state
            .transient_failures_remaining
            .get(&request.run_id)
            .copied()
            .unwrap_or_default()
            > 0
        {
            return self.observe_retry(request, policy_verdict);
        }

        let packet = materialize_next_packet(&request);
        self.state
            .completed_packets
            .insert(request.idempotency_key.clone(), packet.clone());
        self.state.active_claims.remove(&request.run_id);

        AutomationWorkerOutcome {
            status: AutomationWorkerStatus::MaterializedNextPacket,
            packet: Some(packet),
            approval: None,
            command: Some(command_envelope(
                "taskflow.materialize_next_packet",
                &request,
                &policy_verdict,
                serde_json::json!({
                    "packet_id": self
                        .state
                        .completed_packets
                        .get(&request.idempotency_key)
                        .map(|packet| packet.packet_id.clone()),
                    "from_role": request.from_role,
                    "next_role": request.next_role
                }),
            )),
            conflict: None,
            retry: None,
            policy_verdict,
            trace: trace_for(
                "materialized_next_packet",
                "durable packet materialization command accepted",
            ),
        }
    }

    fn observe_retry(
        &mut self,
        request: AnalystCompletionRequest,
        policy_verdict: CedarPolicyVerdict,
    ) -> AutomationWorkerOutcome {
        let attempt = self
            .state
            .attempts_by_run
            .entry(request.run_id.clone())
            .and_modify(|attempt| *attempt += 1)
            .or_insert(1);

        let remaining = self
            .state
            .transient_failures_remaining
            .entry(request.run_id.clone())
            .or_default();
        *remaining = remaining.saturating_sub(1);

        let retry_policy = self.config.retry_policy();
        let exhausted = *attempt >= retry_policy.max_attempts() && *remaining > 0;
        AutomationWorkerOutcome {
            status: if exhausted {
                AutomationWorkerStatus::RetryExhausted
            } else {
                AutomationWorkerStatus::Retrying
            },
            packet: None,
            approval: None,
            command: None,
            conflict: None,
            retry: Some(RetryObservation {
                run_id: request.run_id.clone(),
                attempt: *attempt,
                max_attempts: retry_policy.max_attempts(),
                retry_after_seconds: retry_policy.retry_delay_seconds(*attempt),
                blocker_code: exhausted.then(|| WORKER_RETRY_EXHAUSTED_BLOCKER.to_string()),
            }),
            policy_verdict,
            trace: trace_for(
                "retry_observed",
                if exhausted {
                    WORKER_RETRY_EXHAUSTED_BLOCKER
                } else {
                    "transient failure scheduled for bounded retry"
                },
            ),
        }
    }
}

pub fn authorize_next_packet_materialization(
    request: &AnalystCompletionRequest,
) -> CedarPolicyVerdict {
    let allowed = request.cedar_action == "vida.taskflow.materialize_next_packet"
        && request.from_role == "analyst"
        && request.next_role == "developer";
    CedarPolicyVerdict {
        policy_engine: "cedar".to_string(),
        policy_ref: "cedar://vida/taskflow/automation-workers/materialize-next-packet".to_string(),
        allowed,
        reason: if allowed {
            "analyst completion may materialize the developer packet".to_string()
        } else {
            "request does not satisfy the Cedar transition policy projection".to_string()
        },
    }
}

fn materialize_next_packet(request: &AnalystCompletionRequest) -> NextRolePacket {
    NextRolePacket {
        run_id: request.run_id.clone(),
        packet_id: format!(
            "packet:{}:{}:{}",
            request.run_id, request.next_role, request.idempotency_key
        ),
        from_role: request.from_role.clone(),
        next_role: request.next_role.clone(),
        idempotency_key: request.idempotency_key.clone(),
    }
}

fn outcome(
    status: AutomationWorkerStatus,
    policy_verdict: CedarPolicyVerdict,
) -> AutomationWorkerOutcome {
    AutomationWorkerOutcome {
        status,
        packet: None,
        approval: None,
        command: None,
        conflict: None,
        retry: None,
        policy_verdict,
        trace: Vec::new(),
    }
}

fn command_envelope(
    operation: &str,
    request: &AnalystCompletionRequest,
    policy_verdict: &CedarPolicyVerdict,
    payload: serde_json::Value,
) -> AutomationCommandEnvelope {
    AutomationCommandEnvelope {
        operation: operation.to_string(),
        run_id: request.run_id.clone(),
        idempotency_key: request.idempotency_key.clone(),
        policy_ref: policy_verdict.policy_ref.clone(),
        payload,
        trace: vec![
            trace_entry("policy_engine", &policy_verdict.policy_engine),
            trace_entry("policy_ref", &policy_verdict.policy_ref),
            trace_entry("from_role", &request.from_role),
            trace_entry("next_role", &request.next_role),
        ],
    }
}

fn trace_for(kind: &str, detail: &str) -> Vec<AutomationTraceEntry> {
    vec![trace_entry(kind, detail)]
}

fn trace_entry(kind: &str, detail: &str) -> AutomationTraceEntry {
    AutomationTraceEntry {
        kind: kind.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_runs_conflict_test() {
        let mut runtime = AutomationWorkerRuntime::new(WorkerAutomationConfig::default());
        runtime.inject_transient_failures("run-1", 1);

        let first = runtime.process_analyst_completion(
            AnalystCompletionRequest::next_developer_packet("run-1", "idem-1"),
        );
        assert_eq!(first.status, AutomationWorkerStatus::Retrying);
        assert_eq!(first.retry.as_ref().map(|retry| retry.attempt), Some(1));

        let conflict = runtime.process_analyst_completion(
            AnalystCompletionRequest::next_developer_packet("run-1", "idem-2"),
        );
        assert_eq!(conflict.status, AutomationWorkerStatus::Conflict);
        assert_eq!(
            conflict
                .conflict
                .as_ref()
                .map(|conflict| conflict.blocker_code.as_str()),
            Some(WORKER_CLAIM_CONFLICT_BLOCKER)
        );
        assert!(runtime.state.completed_packets.is_empty());
    }

    #[test]
    fn end_to_end_analyst_to_next_role_scenario() {
        let mut runtime = AutomationWorkerRuntime::new(WorkerAutomationConfig::default());
        let request = AnalystCompletionRequest::next_developer_packet("run-42", "idem-42");

        let completed = runtime.process_analyst_completion(request.clone());
        assert_eq!(
            completed.status,
            AutomationWorkerStatus::MaterializedNextPacket
        );
        assert!(completed.policy_verdict.allowed);
        assert_eq!(completed.policy_verdict.policy_engine, "cedar");
        assert_eq!(
            completed
                .packet
                .as_ref()
                .map(|packet| packet.next_role.as_str()),
            Some("developer")
        );
        assert_eq!(
            completed.packet,
            Some(NextRolePacket {
                run_id: "run-42".to_string(),
                packet_id: "packet:run-42:developer:idem-42".to_string(),
                from_role: "analyst".to_string(),
                next_role: "developer".to_string(),
                idempotency_key: "idem-42".to_string(),
            })
        );
        let command = completed.command.as_ref().expect("command envelope");
        assert_eq!(command.operation, "taskflow.materialize_next_packet");
        assert_eq!(command.run_id, "run-42");
        assert_eq!(command.idempotency_key, "idem-42");
        assert_eq!(command.payload["from_role"], "analyst");
        assert_eq!(command.payload["next_role"], "developer");
        assert_eq!(
            command.payload["packet_id"],
            "packet:run-42:developer:idem-42"
        );
        assert!(
            command
                .trace
                .iter()
                .any(|entry| entry.kind == "policy_engine" && entry.detail == "cedar")
        );

        let replay = runtime.process_analyst_completion(request);
        assert_eq!(replay.status, AutomationWorkerStatus::IdempotentReplay);
        assert_eq!(replay.packet, completed.packet);
        assert_eq!(
            replay
                .command
                .as_ref()
                .map(|command| command.operation.as_str()),
            Some("taskflow.replay_next_packet")
        );
        assert_eq!(
            replay.trace,
            vec![AutomationTraceEntry {
                kind: "idempotent_replay".to_string(),
                detail: "completed packet returned by idempotency key".to_string(),
            }]
        );
        assert_eq!(runtime.state.completed_packets.len(), 1);

        let denied_replay = runtime.process_analyst_completion(AnalystCompletionRequest {
            run_id: "run-42".to_string(),
            from_role: "analyst".to_string(),
            next_role: "developer".to_string(),
            idempotency_key: "idem-42".to_string(),
            approval_required: false,
            cedar_action: "vida.taskflow.bypass_policy".to_string(),
        });
        assert_eq!(denied_replay.status, AutomationWorkerStatus::PolicyDenied);
        assert!(denied_replay.packet.is_none());
        assert!(denied_replay.command.is_none());

        let approval = runtime.process_analyst_completion(
            AnalystCompletionRequest::next_developer_packet("run-approval", "idem-approval")
                .requiring_approval(),
        );
        assert_eq!(approval.status, AutomationWorkerStatus::ApprovalRequired);
        assert_eq!(
            approval
                .command
                .as_ref()
                .map(|command| command.operation.as_str()),
            Some("taskflow.await_operator_approval")
        );
        assert_eq!(
            approval
                .approval
                .as_ref()
                .map(|approval| approval.resume_action.as_str()),
            Some("approve_then_materialize_next_packet")
        );
        assert_eq!(
            approval.approval,
            Some(TypedApprovalState {
                run_id: "run-approval".to_string(),
                required_role: "operator".to_string(),
                approval_kind: "taskflow_next_role_materialization".to_string(),
                resume_action: "approve_then_materialize_next_packet".to_string(),
            })
        );
        assert_eq!(
            approval
                .command
                .as_ref()
                .map(|command| command.payload.clone()),
            Some(serde_json::json!({
                "required_role": "operator",
                "approval_kind": "taskflow_next_role_materialization",
                "resume_action": "approve_then_materialize_next_packet"
            }))
        );
        assert!(!runtime.state.active_claims.contains_key("run-approval"));
    }

    #[test]
    fn worker_pause_restart_test() {
        let config = WorkerAutomationConfig {
            max_attempts: 2,
            base_retry_seconds: 5,
        };
        let mut runtime = AutomationWorkerRuntime::new(config.clone());
        runtime.pause();

        let request =
            AnalystCompletionRequest::next_developer_packet("run-restart", "idem-restart");
        let paused = runtime.process_analyst_completion(request.clone());
        assert_eq!(paused.status, AutomationWorkerStatus::Paused);

        let snapshot = runtime.state_snapshot();
        let mut restarted = AutomationWorkerRuntime::from_state(config, snapshot);
        assert!(restarted.state.paused);
        restarted.resume();
        restarted.inject_transient_failures("run-restart", 1);

        let retry = restarted.process_analyst_completion(request.clone());
        assert_eq!(retry.status, AutomationWorkerStatus::Retrying);
        assert_eq!(
            retry
                .trace
                .iter()
                .find(|entry| entry.kind == "retry_observed")
                .map(|entry| entry.detail.as_str()),
            Some("transient failure scheduled for bounded retry")
        );
        assert_eq!(
            retry.retry,
            Some(RetryObservation {
                run_id: "run-restart".to_string(),
                attempt: 1,
                max_attempts: 2,
                retry_after_seconds: 5,
                blocker_code: None,
            })
        );

        let completed = restarted.process_analyst_completion(request);
        assert_eq!(
            completed.status,
            AutomationWorkerStatus::MaterializedNextPacket
        );
        assert_eq!(restarted.state.completed_packets.len(), 1);
    }

    #[test]
    fn worker_state_snapshot_preserves_injected_failure_budget() {
        let config = WorkerAutomationConfig {
            max_attempts: 3,
            base_retry_seconds: 7,
        };
        let mut runtime = AutomationWorkerRuntime::new(config.clone());
        runtime.inject_transient_failures("run-snapshot", 2);
        let snapshot = runtime.state_snapshot();
        assert_eq!(
            snapshot.transient_failures_remaining.get("run-snapshot"),
            Some(&2)
        );

        let request =
            AnalystCompletionRequest::next_developer_packet("run-snapshot", "idem-snapshot");
        let mut resumed = AutomationWorkerRuntime::from_state(config, snapshot);
        assert_eq!(
            resumed.process_analyst_completion(request.clone()).status,
            AutomationWorkerStatus::Retrying
        );
        assert_eq!(
            resumed.process_analyst_completion(request.clone()).status,
            AutomationWorkerStatus::Retrying
        );
        let completed = resumed.process_analyst_completion(request);
        assert_eq!(
            completed.status,
            AutomationWorkerStatus::MaterializedNextPacket
        );
        assert_eq!(resumed.state.attempts_by_run.get("run-snapshot"), Some(&2));
    }

    #[test]
    fn retry_exhaustion_fails_closed_without_packet_or_command() {
        let config = WorkerAutomationConfig {
            max_attempts: 2,
            base_retry_seconds: 5,
        };
        let mut runtime = AutomationWorkerRuntime::new(config);
        let request = AnalystCompletionRequest::next_developer_packet("run-exhausted", "idem-1");
        runtime.inject_transient_failures("run-exhausted", 3);

        let first = runtime.process_analyst_completion(request.clone());
        assert_eq!(first.status, AutomationWorkerStatus::Retrying);

        let exhausted = runtime.process_analyst_completion(request);
        assert_eq!(exhausted.status, AutomationWorkerStatus::RetryExhausted);
        assert!(exhausted.packet.is_none());
        assert!(exhausted.command.is_none());
        assert_eq!(
            exhausted.retry,
            Some(RetryObservation {
                run_id: "run-exhausted".to_string(),
                attempt: 2,
                max_attempts: 2,
                retry_after_seconds: 10,
                blocker_code: Some(WORKER_RETRY_EXHAUSTED_BLOCKER.to_string()),
            })
        );
        assert_eq!(
            exhausted
                .trace
                .iter()
                .find(|entry| entry.kind == "retry_observed")
                .map(|entry| entry.detail.as_str()),
            Some(WORKER_RETRY_EXHAUSTED_BLOCKER)
        );
        assert!(runtime.state.completed_packets.is_empty());
    }

    #[test]
    fn cedar_transition_policy_matrix_rejects_wrong_roles_and_actions() {
        let valid = AnalystCompletionRequest::next_developer_packet("run-policy", "idem-policy");
        let verdict = authorize_next_packet_materialization(&valid);
        assert!(verdict.allowed);
        assert_eq!(verdict.policy_engine, "cedar");

        for (from_role, next_role, cedar_action) in [
            (
                "developer",
                "developer",
                "vida.taskflow.materialize_next_packet",
            ),
            ("analyst", "tester", "vida.taskflow.materialize_next_packet"),
            ("analyst", "developer", "vida.taskflow.bypass_policy"),
        ] {
            let request = AnalystCompletionRequest {
                run_id: "run-policy".to_string(),
                from_role: from_role.to_string(),
                next_role: next_role.to_string(),
                idempotency_key: "idem-policy".to_string(),
                approval_required: false,
                cedar_action: cedar_action.to_string(),
            };
            let verdict = authorize_next_packet_materialization(&request);
            assert!(!verdict.allowed, "unexpectedly allowed {request:?}");
            assert!(verdict.reason.contains("does not satisfy"));
        }
    }

    #[test]
    fn retry_budget_exactly_exhausted_completes_after_last_transient_failure() {
        let config = WorkerAutomationConfig {
            max_attempts: 2,
            base_retry_seconds: 5,
        };
        let mut runtime = AutomationWorkerRuntime::new(config);
        let request = AnalystCompletionRequest::next_developer_packet("run-boundary", "idem-1");
        runtime.inject_transient_failures("run-boundary", 2);

        let first = runtime.process_analyst_completion(request.clone());
        assert_eq!(first.status, AutomationWorkerStatus::Retrying);
        assert_eq!(first.retry.as_ref().map(|retry| retry.attempt), Some(1));

        let second = runtime.process_analyst_completion(request.clone());
        assert_eq!(second.status, AutomationWorkerStatus::Retrying);
        assert_eq!(second.retry.as_ref().map(|retry| retry.attempt), Some(2));
        assert_eq!(
            second
                .retry
                .as_ref()
                .and_then(|retry| retry.blocker_code.as_deref()),
            None
        );

        let completed = runtime.process_analyst_completion(request);
        assert_eq!(
            completed.status,
            AutomationWorkerStatus::MaterializedNextPacket
        );
        assert!(completed.packet.is_some());
        assert!(runtime.state.active_claims.is_empty());
    }
}
