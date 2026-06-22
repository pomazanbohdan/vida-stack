use serde::{Deserialize, Serialize};
use statig::{
    Outcome::{Handled, Super, Transition},
    blocking::{IntoStateMachine, IntoStateMachineExt, State, Superstate},
};

use crate::role_step::TaskRoleStep;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunWorkflowAggregate {
    pub run_id: String,
    pub task_id: String,
    pub state: RunWorkflowState,
    pub version: u64,
}

impl RunWorkflowAggregate {
    #[must_use]
    pub fn new(run_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            state: RunWorkflowState::Idle,
            version: 0,
        }
    }

    #[must_use]
    pub fn from_snapshot(
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        state: RunWorkflowState,
        version: u64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            state,
            version,
        }
    }

    #[must_use]
    pub fn handle(&mut self, command: RunWorkflowCommand) -> RunWorkflowEvent {
        let before = self.state.clone();
        if before.is_terminal() && !matches!(command, RunWorkflowCommand::RepairReopen { .. }) {
            return RunWorkflowEvent {
                command,
                state_before: before.clone(),
                state_after: before,
                effect_intents: vec![RunWorkflowEffectIntent::RejectMutation],
                blocker_code: Some("terminal_state_mutation_rejected".to_string()),
            };
        }

        let after = transition_with_statig(before.clone(), &command);
        if after != before {
            self.state = after.clone();
            self.version += 1;
        }

        RunWorkflowEvent {
            effect_intents: effect_intents_for(&command, &before, &after),
            command,
            state_before: before,
            state_after: after,
            blocker_code: None,
        }
    }

    #[must_use]
    pub fn snapshot_replay_hash(&self) -> String {
        serde_json::to_string(&(&self.run_id, &self.task_id, &self.state, self.version))
            .expect("run workflow aggregate snapshot should serialize")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunWorkflowState {
    Idle,
    Active { step: TaskRoleStep },
    ApprovalBlocked,
    LaneBlocked,
    RecoveryBlocked,
    Completed,
    Failed,
}

impl RunWorkflowState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    #[must_use]
    pub fn canonical_name(&self) -> String {
        match self {
            Self::Idle => "idle".to_string(),
            Self::Active { step } => step.state_name(),
            Self::ApprovalBlocked => "approval_blocked".to_string(),
            Self::LaneBlocked => "lane_blocked".to_string(),
            Self::RecoveryBlocked => "recovery_blocked".to_string(),
            Self::Completed => "completed".to_string(),
            Self::Failed => "failed".to_string(),
        }
    }

    fn from_role_step(step: TaskRoleStep) -> Self {
        Self::Active { step }
    }
}

pub type RoleStep = TaskRoleStep;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    Approval,
    Lane,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunWorkflowCommand {
    Start { first_step: RoleStep },
    Dispatch { target: RoleStep },
    CompleteLane { next: Option<RoleStep> },
    Block { reason: BlockReason },
    Recover { target: RoleStep },
    Close,
    Fail { code: String, retryable: bool },
    RepairReopen { target: RoleStep },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunWorkflowEvent {
    pub command: RunWorkflowCommand,
    pub state_before: RunWorkflowState,
    pub state_after: RunWorkflowState,
    pub effect_intents: Vec<RunWorkflowEffectIntent>,
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunWorkflowEffectIntent {
    PersistSnapshot,
    DispatchRole,
    RecordBlocker,
    RecordTerminal,
    RejectMutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionMatrixRow {
    pub from: RunWorkflowState,
    pub command: &'static str,
    pub to: RunWorkflowState,
    pub admitted: bool,
    pub effect_intents: Vec<RunWorkflowEffectIntent>,
}

#[must_use]
pub fn transition_matrix() -> Vec<TransitionMatrixRow> {
    let cases = [
        (
            RunWorkflowState::Idle,
            RunWorkflowCommand::Start {
                first_step: RoleStep::planning(),
            },
            "start",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::planning()),
            RunWorkflowCommand::Dispatch {
                target: RoleStep::developer(),
            },
            "dispatch_developer",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::developer()),
            RunWorkflowCommand::CompleteLane {
                next: Some(RoleStep::tester()),
            },
            "complete_developer",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::tester()),
            RunWorkflowCommand::CompleteLane {
                next: Some(RoleStep::closure()),
            },
            "complete_tester",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::closure()),
            RunWorkflowCommand::Close,
            "close",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::developer()),
            RunWorkflowCommand::Block {
                reason: BlockReason::Lane,
            },
            "block_lane",
        ),
        (
            RunWorkflowState::LaneBlocked,
            RunWorkflowCommand::Recover {
                target: RoleStep::developer(),
            },
            "recover",
        ),
        (
            RunWorkflowState::from_role_step(RoleStep::planning()),
            RunWorkflowCommand::Fail {
                code: "failed".to_string(),
                retryable: false,
            },
            "fail",
        ),
        (
            RunWorkflowState::Completed,
            RunWorkflowCommand::Dispatch {
                target: RoleStep::developer(),
            },
            "terminal_reject",
        ),
        (
            RunWorkflowState::Completed,
            RunWorkflowCommand::RepairReopen {
                target: RoleStep::developer(),
            },
            "repair_reopen",
        ),
    ];

    cases
        .into_iter()
        .map(|(from, command, name)| {
            let mut aggregate = RunWorkflowAggregate::from_snapshot("run", "task", from.clone(), 0);
            let event = aggregate.handle(command);
            TransitionMatrixRow {
                from,
                command: name,
                to: event.state_after,
                admitted: event.blocker_code.is_none(),
                effect_intents: event.effect_intents,
            }
        })
        .collect()
}

#[must_use]
pub fn transition_matrix_mermaid() -> String {
    let mut lines = vec!["stateDiagram-v2".to_string()];
    for row in transition_matrix() {
        if row.admitted {
            lines.push(format!(
                "  {} --> {}: {}",
                row.from.canonical_name(),
                row.to.canonical_name(),
                row.command
            ));
        } else {
            lines.push(format!(
                "  {} --> {}: {} rejected",
                row.from.canonical_name(),
                row.to.canonical_name(),
                row.command
            ));
        }
    }
    lines.join("\n")
}

#[must_use]
pub fn replay_events(
    initial: RunWorkflowAggregate,
    commands: &[RunWorkflowCommand],
) -> (RunWorkflowAggregate, Vec<RunWorkflowEvent>) {
    let mut aggregate = initial;
    let events = commands
        .iter()
        .cloned()
        .map(|command| aggregate.handle(command))
        .collect();
    (aggregate, events)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMappingCase {
    pub lifecycle_stage: &'static str,
    pub status: &'static str,
    pub decision: StatusMappingDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusMappingDecision {
    State(RunWorkflowState),
    Blocked { blocker_code: &'static str },
}

#[must_use]
pub fn status_mapping_corpus() -> Vec<StatusMappingCase> {
    vec![
        status_case("initialized", "pending", RunWorkflowState::Idle),
        status_case(
            "developer_dispatch_ready",
            "ready",
            RunWorkflowState::from_role_step(RoleStep::planning()),
        ),
        status_case(
            "developer_running",
            "running",
            RunWorkflowState::from_role_step(RoleStep::developer()),
        ),
        status_case(
            "tester_dispatch_ready",
            "ready",
            RunWorkflowState::from_role_step(RoleStep::developer()),
        ),
        status_case(
            "tester_running",
            "running",
            RunWorkflowState::from_role_step(RoleStep::tester()),
        ),
        status_case("tester_blocked", "blocked", RunWorkflowState::LaneBlocked),
        status_case(
            "closure_active",
            "running",
            RunWorkflowState::from_role_step(RoleStep::closure()),
        ),
        status_case("closure_complete", "completed", RunWorkflowState::Completed),
        status_case("failed_terminal", "failed", RunWorkflowState::Failed),
        StatusMappingCase {
            lifecycle_stage: "unknown_external_status",
            status: "unknown",
            decision: StatusMappingDecision::Blocked {
                blocker_code: "status_mapping_unknown",
            },
        },
    ]
}

fn status_case(
    lifecycle_stage: &'static str,
    status: &'static str,
    state: RunWorkflowState,
) -> StatusMappingCase {
    StatusMappingCase {
        lifecycle_stage,
        status,
        decision: StatusMappingDecision::State(state),
    }
}

fn transition_with_statig(
    current: RunWorkflowState,
    command: &RunWorkflowCommand,
) -> RunWorkflowState {
    let mut uninitialized = RunWorkflowMachine.uninitialized_state_machine();
    *uninitialized.state_mut() = current.clone();
    let mut machine = uninitialized.init();
    machine.handle(command);
    machine.state().clone()
}

fn effect_intents_for(
    command: &RunWorkflowCommand,
    before: &RunWorkflowState,
    after: &RunWorkflowState,
) -> Vec<RunWorkflowEffectIntent> {
    if before == after {
        return vec![RunWorkflowEffectIntent::PersistSnapshot];
    }
    match command {
        RunWorkflowCommand::Dispatch { .. }
        | RunWorkflowCommand::Start { .. }
        | RunWorkflowCommand::Recover { .. }
        | RunWorkflowCommand::RepairReopen { .. }
        | RunWorkflowCommand::CompleteLane { next: Some(_) } => {
            vec![
                RunWorkflowEffectIntent::PersistSnapshot,
                RunWorkflowEffectIntent::DispatchRole,
            ]
        }
        RunWorkflowCommand::Block { .. } => vec![
            RunWorkflowEffectIntent::PersistSnapshot,
            RunWorkflowEffectIntent::RecordBlocker,
        ],
        RunWorkflowCommand::Close
        | RunWorkflowCommand::Fail { .. }
        | RunWorkflowCommand::CompleteLane { next: None } => vec![
            RunWorkflowEffectIntent::PersistSnapshot,
            RunWorkflowEffectIntent::RecordTerminal,
        ],
    }
}

#[derive(Debug, Default)]
struct RunWorkflowMachine;

impl IntoStateMachine for RunWorkflowMachine {
    type State = RunWorkflowState;
    type Superstate<'sub> = RunWorkflowSuperstate;
    type Event<'evt> = RunWorkflowCommand;
    type Context<'ctx> = ();

    fn initial() -> Self::State {
        RunWorkflowState::Idle
    }
}

impl State<RunWorkflowMachine> for RunWorkflowState {
    fn call_handler(
        &mut self,
        _: &mut RunWorkflowMachine,
        event: &RunWorkflowCommand,
        _: &mut (),
    ) -> statig::Outcome<Self> {
        match event {
            RunWorkflowCommand::Start { first_step } if matches!(self, Self::Idle) => {
                Transition(Self::from_role_step(first_step.clone()))
            }
            RunWorkflowCommand::Dispatch { target } if self.is_active() => {
                Transition(Self::from_role_step(target.clone()))
            }
            RunWorkflowCommand::CompleteLane { next } if self.is_active() => next
                .clone()
                .map(Self::from_role_step)
                .map_or(Transition(Self::Completed), Transition),
            RunWorkflowCommand::Block { reason } if self.is_active() => {
                Transition(blocked_state(reason.clone()))
            }
            RunWorkflowCommand::Recover { target } if self.is_blocked() => {
                Transition(Self::from_role_step(target.clone()))
            }
            RunWorkflowCommand::Close if self.is_closure_step() => Transition(Self::Completed),
            RunWorkflowCommand::Fail { .. } if !self.is_terminal() => Transition(Self::Failed),
            RunWorkflowCommand::RepairReopen { target } if self.is_terminal() => {
                Transition(Self::from_role_step(target.clone()))
            }
            _ => Super,
        }
    }

    fn superstate(&mut self) -> Option<RunWorkflowSuperstate> {
        if self.is_active() {
            Some(RunWorkflowSuperstate::Active)
        } else if self.is_blocked() {
            Some(RunWorkflowSuperstate::Blocked)
        } else if self.is_terminal() {
            Some(RunWorkflowSuperstate::Terminal)
        } else {
            None
        }
    }
}

impl<'sub> Superstate<RunWorkflowMachine> for RunWorkflowSuperstate
where
    Self: 'sub,
{
    fn call_handler(
        &mut self,
        _: &mut RunWorkflowMachine,
        _: &RunWorkflowCommand,
        _: &mut (),
    ) -> statig::Outcome<RunWorkflowState>
    where
        Self: Sized,
    {
        Handled
    }

    fn superstate(&mut self) -> Option<RunWorkflowSuperstate> {
        None
    }
}

impl RunWorkflowState {
    fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    fn is_blocked(&self) -> bool {
        matches!(
            self,
            Self::ApprovalBlocked | Self::LaneBlocked | Self::RecoveryBlocked
        )
    }

    fn is_closure_step(&self) -> bool {
        matches!(self, Self::Active { step } if step.closes_workflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunWorkflowSuperstate {
    Active,
    Blocked,
    Terminal,
}

fn blocked_state(reason: BlockReason) -> RunWorkflowState {
    match reason {
        BlockReason::Approval => RunWorkflowState::ApprovalBlocked,
        BlockReason::Lane => RunWorkflowState::LaneBlocked,
        BlockReason::Recovery => RunWorkflowState::RecoveryBlocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn happy_path_commands() -> Vec<RunWorkflowCommand> {
        vec![
            RunWorkflowCommand::Start {
                first_step: RoleStep::planning(),
            },
            RunWorkflowCommand::Dispatch {
                target: RoleStep::developer(),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(RoleStep::tester()),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(RoleStep::closure()),
            },
            RunWorkflowCommand::Close,
        ]
    }

    #[test]
    fn transition_matrix_generates_mermaid_diagram() {
        let matrix = transition_matrix();
        let diagram = transition_matrix_mermaid();

        assert!(
            matrix
                .iter()
                .any(|row| row.to == RunWorkflowState::Completed)
        );
        assert!(matrix.iter().any(|row| !row.admitted));
        assert!(diagram.starts_with("stateDiagram-v2"));
        assert!(diagram.contains("role_developer --> lane_blocked: block_lane"));
        assert!(diagram.contains("completed --> completed: terminal_reject rejected"));
    }

    #[test]
    fn replay_is_deterministic() {
        let commands = happy_path_commands();
        let initial = RunWorkflowAggregate::new("run-020", "ldr-020");

        let first = replay_events(initial.clone(), &commands);
        let second = replay_events(initial, &commands);

        assert_eq!(first, second);
        assert_eq!(first.0.state, RunWorkflowState::Completed);
        assert_eq!(first.0.version, 5);
    }

    #[test]
    fn snapshot_replay_hash_matches_event_replay() {
        let commands = happy_path_commands();
        let initial = RunWorkflowAggregate::new("run-031", "ldr-031");
        let (replayed, _) = replay_events(initial, &commands);
        let snapshot = RunWorkflowAggregate::from_snapshot(
            "run-031",
            "ldr-031",
            replayed.state.clone(),
            replayed.version,
        );

        assert_eq!(
            snapshot.snapshot_replay_hash(),
            replayed.snapshot_replay_hash()
        );
    }

    #[test]
    fn terminal_states_reject_mutation_except_repair() {
        let mut aggregate = RunWorkflowAggregate::from_snapshot(
            "run-020",
            "ldr-020",
            RunWorkflowState::Completed,
            9,
        );
        let rejected = aggregate.handle(RunWorkflowCommand::Dispatch {
            target: RoleStep::developer(),
        });

        assert_eq!(aggregate.state, RunWorkflowState::Completed);
        assert_eq!(aggregate.version, 9);
        assert_eq!(
            rejected.blocker_code.as_deref(),
            Some("terminal_state_mutation_rejected")
        );
        assert_eq!(
            rejected.effect_intents,
            vec![RunWorkflowEffectIntent::RejectMutation]
        );

        let reopened = aggregate.handle(RunWorkflowCommand::RepairReopen {
            target: RoleStep::developer(),
        });
        assert_eq!(
            reopened.state_after,
            RunWorkflowState::from_role_step(RoleStep::developer())
        );
        assert_eq!(aggregate.version, 10);
    }

    #[test]
    fn status_mapping_corpus_maps_known_stages_and_blocks_unknowns() {
        let corpus = status_mapping_corpus();

        assert!(corpus.iter().any(|case| {
            case.lifecycle_stage == "tester_blocked"
                && case.decision == StatusMappingDecision::State(RunWorkflowState::LaneBlocked)
        }));
        assert!(corpus.iter().any(|case| {
            matches!(
                case.decision,
                StatusMappingDecision::Blocked {
                    blocker_code: "status_mapping_unknown"
                }
            )
        }));
    }

    #[test]
    fn aggregate_actions_emit_effect_intents_without_io_payloads() {
        let mut aggregate = RunWorkflowAggregate::new("run-020", "ldr-020");
        let event = aggregate.handle(RunWorkflowCommand::Start {
            first_step: RoleStep::planning(),
        });

        assert_eq!(
            event.effect_intents,
            vec![
                RunWorkflowEffectIntent::PersistSnapshot,
                RunWorkflowEffectIntent::DispatchRole
            ]
        );
        assert_eq!(
            aggregate.state,
            RunWorkflowState::from_role_step(RoleStep::planning())
        );
    }

    #[test]
    fn configured_dev_team_role_steps_replay_in_declared_order() {
        let analyst = RoleStep::new(
            "analyst",
            "business_analyst",
            "specification",
            "design_gate",
        );
        let developer = RoleStep::new("developer", "worker", "implementation", "implementation");
        let coach = RoleStep::new(
            "coach_implementation_gate",
            "coach",
            "coach",
            "implementation_gate",
        );
        let tester = RoleStep::new("tester", "verifier", "verification", "verification");
        let prover =
            RoleStep::new("prover", "prover", "release_readiness", "release_readiness").closing();

        let commands = vec![
            RunWorkflowCommand::Start {
                first_step: analyst.clone(),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(developer.clone()),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(coach.clone()),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(tester.clone()),
            },
            RunWorkflowCommand::CompleteLane {
                next: Some(prover.clone()),
            },
            RunWorkflowCommand::Close,
        ];

        let (aggregate, events) =
            replay_events(RunWorkflowAggregate::new("run-022", "ldr-022"), &commands);

        assert_eq!(
            events[0].state_after,
            RunWorkflowState::from_role_step(analyst)
        );
        assert_eq!(
            events[1].state_after,
            RunWorkflowState::from_role_step(developer)
        );
        assert_eq!(
            events[2].state_after,
            RunWorkflowState::from_role_step(coach)
        );
        assert_eq!(
            events[3].state_after,
            RunWorkflowState::from_role_step(tester)
        );
        assert_eq!(
            events[4].state_after,
            RunWorkflowState::from_role_step(prover)
        );
        assert_eq!(aggregate.state, RunWorkflowState::Completed);
        assert_eq!(aggregate.version, 6);
    }
}
