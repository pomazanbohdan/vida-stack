//! P0/P1 semantic test corpus.
//!
//! This module is intentionally test-only: it drives pure TaskFlow aggregates
//! and in-memory journals and never invokes VIDA runtime activation/dispatch.

use crate::failure_injection::{
    benchmark_review_artifact, failure_matrix_review_artifact, run_ldrk_qualification,
};
use crate::shadow_diff::compare_management_dispatch_projection;
use common_format_jsonl::{decode_line as common_decode, encode_line as common_encode};
use common_format_toon::{render_toon_value_block, sanitize_toon_scalar};
use docflow_format_jsonl::{decode_line as docflow_decode, encode_line as docflow_encode};
use proptest::prelude::*;
use proptest_state_machine::{prop_state_machine, ReferenceStateMachine, StateMachineTest};
use serde::{Deserialize, Serialize};
use taskflow_core::{
    canonical_issue_type, canonical_task_status, normalize_issue_type, parse_task_status,
    path_policy::normalize_repo_relative_path,
    role_step::TaskRoleStep,
    run_workflow::{
        replay_events, BlockReason, RunWorkflowAggregate, RunWorkflowCommand,
        RunWorkflowEffectIntent, RunWorkflowState,
    },
};
use taskflow_format_jsonl::{decode_line as taskflow_decode, encode_line as taskflow_encode};

#[derive(Clone, Debug)]
struct ReferenceState {
    state: RunWorkflowState,
    version: u64,
}

#[derive(Clone, Debug)]
enum Transition {
    Start,
    Dispatch,
    CompleteLaneToTester,
    CompleteLaneToClosure,
    CompleteLaneTerminal,
    Block(BlockReason),
    Recover,
    Close,
    Fail,
    RepairReopen,
}

fn transition_strategy() -> impl Strategy<Value = Transition> {
    prop_oneof![
        Just(Transition::Start),
        Just(Transition::Dispatch),
        Just(Transition::CompleteLaneToTester),
        Just(Transition::CompleteLaneToClosure),
        Just(Transition::CompleteLaneTerminal),
        Just(Transition::Block(BlockReason::Approval)),
        Just(Transition::Block(BlockReason::Lane)),
        Just(Transition::Block(BlockReason::Recovery)),
        Just(Transition::Recover),
        Just(Transition::Close),
        Just(Transition::Fail),
        Just(Transition::RepairReopen),
    ]
}

fn command_for(transition: &Transition) -> RunWorkflowCommand {
    match transition {
        Transition::Start => RunWorkflowCommand::Start {
            first_step: TaskRoleStep::planning(),
        },
        Transition::Dispatch => RunWorkflowCommand::Dispatch {
            target: TaskRoleStep::developer(),
        },
        Transition::CompleteLaneToTester => RunWorkflowCommand::CompleteLane {
            next: Some(TaskRoleStep::tester()),
        },
        Transition::CompleteLaneToClosure => RunWorkflowCommand::CompleteLane {
            next: Some(TaskRoleStep::closure()),
        },
        Transition::CompleteLaneTerminal => RunWorkflowCommand::CompleteLane { next: None },
        Transition::Block(reason) => RunWorkflowCommand::Block {
            reason: reason.clone(),
        },
        Transition::Recover => RunWorkflowCommand::Recover {
            target: TaskRoleStep::developer(),
        },
        Transition::Close => RunWorkflowCommand::Close,
        Transition::Fail => RunWorkflowCommand::Fail {
            code: "semantic_failure".to_string(),
            retryable: true,
        },
        Transition::RepairReopen => RunWorkflowCommand::RepairReopen {
            target: TaskRoleStep::developer(),
        },
    }
}

struct Reference;

impl ReferenceStateMachine for Reference {
    type State = ReferenceState;
    type Transition = Transition;

    fn init_state() -> BoxedStrategy<Self::State> {
        Just(ReferenceState {
            state: RunWorkflowState::Idle,
            version: 0,
        })
        .boxed()
    }

    fn transitions(_state: &Self::State) -> BoxedStrategy<Self::Transition> {
        transition_strategy().boxed()
    }

    fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
        let before = state.state.clone();
        let after = reference_transition(&before, transition);
        if after != before {
            state.state = after;
            state.version += 1;
        }
        state
    }
}

fn reference_transition(current: &RunWorkflowState, transition: &Transition) -> RunWorkflowState {
    if current.is_terminal() && !matches!(transition, Transition::RepairReopen) {
        return current.clone();
    }

    match (current, transition) {
        (RunWorkflowState::Idle, Transition::Start) => RunWorkflowState::Active {
            step: TaskRoleStep::planning(),
        },
        (RunWorkflowState::Active { .. }, Transition::Dispatch) => RunWorkflowState::Active {
            step: TaskRoleStep::developer(),
        },
        (RunWorkflowState::Active { .. }, Transition::CompleteLaneToTester) => {
            RunWorkflowState::Active {
                step: TaskRoleStep::tester(),
            }
        }
        (RunWorkflowState::Active { .. }, Transition::CompleteLaneToClosure) => {
            RunWorkflowState::Active {
                step: TaskRoleStep::closure(),
            }
        }
        (RunWorkflowState::Active { .. }, Transition::CompleteLaneTerminal) => {
            RunWorkflowState::Completed
        }
        (RunWorkflowState::Active { .. }, Transition::Block(reason)) => match reason {
            BlockReason::Approval => RunWorkflowState::ApprovalBlocked,
            BlockReason::Lane => RunWorkflowState::LaneBlocked,
            BlockReason::Recovery => RunWorkflowState::RecoveryBlocked,
        },
        (
            RunWorkflowState::ApprovalBlocked
            | RunWorkflowState::LaneBlocked
            | RunWorkflowState::RecoveryBlocked,
            Transition::Recover,
        ) => RunWorkflowState::Active {
            step: TaskRoleStep::developer(),
        },
        (RunWorkflowState::Active { step }, Transition::Close) if step.closes_workflow => {
            RunWorkflowState::Completed
        }
        (state, Transition::Fail) if !state.is_terminal() => RunWorkflowState::Failed,
        (RunWorkflowState::Completed | RunWorkflowState::Failed, Transition::RepairReopen) => {
            RunWorkflowState::Active {
                step: TaskRoleStep::developer(),
            }
        }
        _ => current.clone(),
    }
}

#[derive(Debug)]
struct SemanticStateMachine;

#[derive(Debug)]
struct SystemUnderTest {
    aggregate: RunWorkflowAggregate,
    events: Vec<taskflow_core::run_workflow::RunWorkflowEvent>,
}

impl StateMachineTest for SemanticStateMachine {
    type SystemUnderTest = SystemUnderTest;
    type Reference = Reference;

    fn init_test(_ref_state: &ReferenceState) -> Self::SystemUnderTest {
        SystemUnderTest {
            aggregate: RunWorkflowAggregate::new("semantic-run", "semantic-task"),
            events: Vec::new(),
        }
    }

    fn apply(
        mut state: Self::SystemUnderTest,
        ref_state: &ReferenceState,
        transition: Transition,
    ) -> Self::SystemUnderTest {
        let before = state.aggregate.clone();
        let event = state.aggregate.handle(command_for(&transition));
        let expected_after = reference_transition(&ref_state.state, &transition);

        assert_eq!(state.aggregate.state, ref_state.state);
        assert_eq!(state.aggregate.version, ref_state.version);
        assert_eq!(event.state_before, before.state);
        assert_eq!(event.state_after, expected_after);
        assert_eq!(
            event.effect_intents,
            reference_effect_intents(&transition, &before.state, &expected_after)
        );
        assert_eq!(
            event.blocker_code,
            if before.state.is_terminal() && !matches!(transition, Transition::RepairReopen) {
                Some("terminal_state_mutation_rejected".to_string())
            } else {
                None
            }
        );
        if event
            .effect_intents
            .contains(&RunWorkflowEffectIntent::RejectMutation)
        {
            assert_eq!(before.version, state.aggregate.version);
            assert_eq!(event.state_before, event.state_after);
        }
        state.events.push(event);
        state
    }

    fn check_invariants(state: &Self::SystemUnderTest, ref_state: &ReferenceState) {
        assert_eq!(state.aggregate.state, ref_state.state);
        assert_eq!(state.aggregate.version, ref_state.version);
        assert!(state.events.len() <= 128);
        assert!(state
            .events
            .windows(2)
            .all(|events| events[1].state_before == events[0].state_after));
    }
}

fn reference_effect_intents(
    transition: &Transition,
    before: &RunWorkflowState,
    after: &RunWorkflowState,
) -> Vec<RunWorkflowEffectIntent> {
    if before.is_terminal() && !matches!(transition, Transition::RepairReopen) {
        return vec![RunWorkflowEffectIntent::RejectMutation];
    }
    if before == after {
        return vec![RunWorkflowEffectIntent::PersistSnapshot];
    }
    match transition {
        Transition::Start
        | Transition::Dispatch
        | Transition::Recover
        | Transition::RepairReopen
        | Transition::CompleteLaneToTester
        | Transition::CompleteLaneToClosure => vec![
            RunWorkflowEffectIntent::PersistSnapshot,
            RunWorkflowEffectIntent::DispatchRole,
        ],
        Transition::CompleteLaneTerminal | Transition::Close | Transition::Fail => vec![
            RunWorkflowEffectIntent::PersistSnapshot,
            RunWorkflowEffectIntent::RecordTerminal,
        ],
        Transition::Block(_) => vec![
            RunWorkflowEffectIntent::PersistSnapshot,
            RunWorkflowEffectIntent::RecordBlocker,
        ],
    }
}

prop_state_machine! {
    #[test]
    fn taskflow_reference_model_is_stateful_and_shrinkable(sequential 1..32 => SemanticStateMachine);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JsonlRow {
    id: String,
    value: u64,
}

proptest! {
    #[test]
    fn repo_path_normalization_is_idempotent(value in "[a-zA-Z0-9_./\\-]{0,48}") {
        if let Ok(first) = normalize_repo_relative_path(&value) {
            prop_assert_eq!(normalize_repo_relative_path(&first), Ok(first));
        }
    }

    #[test]
    fn canonical_status_and_issue_tokens_are_stable(value in "[a-zA-Z0-9 _-]{0,32}") {
        if let Some(canonical) = canonical_task_status(&value) {
            prop_assert_eq!(canonical_task_status(canonical), Some(canonical));
            prop_assert!(parse_task_status(canonical).is_some());
        }

        let canonical_issue = canonical_issue_type(&value);
        prop_assert_eq!(canonical_issue_type(&canonical_issue), canonical_issue.clone());
        prop_assert_eq!(normalize_issue_type(&canonical_issue), canonical_issue.clone());
        prop_assert!(!canonical_issue.is_empty() || value.trim().is_empty());
    }

    #[test]
    fn jsonl_paths_have_one_canonical_round_trip(id in "[a-z]{1,12}", value in 0u64..100_000u64) {
        let row = JsonlRow { id, value };
        let common_line = common_encode(&row).expect("common JSONL encode");
        let docflow_line = docflow_encode(&row).expect("DocFlow JSONL encode");
        let taskflow_line = taskflow_encode(&row).expect("TaskFlow JSONL encode");
        prop_assert_eq!(common_line.clone(), docflow_line);
        prop_assert_eq!(common_line.clone(), taskflow_line);
        prop_assert_eq!(common_decode::<JsonlRow>(&common_line).expect("common decode"), row.clone());
        prop_assert_eq!(docflow_decode::<JsonlRow>(&common_line).expect("DocFlow decode"), row.clone());
        prop_assert_eq!(taskflow_decode::<JsonlRow>(&common_line).expect("TaskFlow decode"), row);
    }
}

#[test]
fn toon_rendering_is_deterministic_and_scalar_sanitization_is_stable() {
    let value = serde_json::json!({"message": "line\nvalue", "count": 2});
    let first = render_toon_value_block("semantic", &value);
    let second = render_toon_value_block("semantic", &value);
    assert_eq!(first, second);
    assert_eq!(sanitize_toon_scalar("line\nvalue"), r"line\nvalue");
}

#[test]
fn projection_and_fault_qualification_are_gate_reachable() {
    let management = serde_json::json!({
        "state": "active",
        "projection": "management",
        "authoritative_write_count": 0,
        "external_effect_count": 0
    });
    let dispatch = serde_json::json!({
        "state": "active",
        "projection": "dispatch",
        "authoritative_write_count": 0,
        "external_effect_count": 0
    });
    let comparison =
        compare_management_dispatch_projection(&management, &dispatch, &["projection"]);
    assert_eq!(comparison.parity_gate, "pass");

    let report = run_ldrk_qualification();
    assert_eq!(report.lost_command_count, 0);
    assert_eq!(report.duplicate_semantic_effect_count, 0);
    assert_eq!(report.concurrency_violation_count, 0);
    assert!(report.benchmark_comparison.within_threshold);
    assert_eq!(
        failure_matrix_review_artifact()["lost_command_count"],
        serde_json::json!(0)
    );
    assert_eq!(
        benchmark_review_artifact()["within_threshold"],
        serde_json::json!(true)
    );
}

#[test]
fn replay_and_snapshot_are_metamorphically_equal() {
    let commands = vec![
        RunWorkflowCommand::Start {
            first_step: TaskRoleStep::planning(),
        },
        RunWorkflowCommand::Dispatch {
            target: TaskRoleStep::developer(),
        },
        RunWorkflowCommand::CompleteLane {
            next: Some(TaskRoleStep::tester()),
        },
    ];
    let initial = RunWorkflowAggregate::new("semantic-run", "semantic-task");
    let (replayed, events) = replay_events(initial.clone(), &commands);
    let (checkpoint, prefix_events) = replay_events(initial, &commands[..2]);
    let snapshot_bytes = serde_json::to_vec(&checkpoint).expect("snapshot serializes");
    let restored: RunWorkflowAggregate =
        serde_json::from_slice(&snapshot_bytes).expect("snapshot restores");
    let (resumed, suffix_events) = replay_events(restored, &commands[2..]);

    assert_eq!(prefix_events.len() + suffix_events.len(), events.len());
    assert_eq!(
        events.last().map(|event| &event.state_after),
        Some(&resumed.state)
    );
    assert_eq!(replayed, resumed);
    assert_eq!(
        replayed.snapshot_replay_hash(),
        resumed.snapshot_replay_hash()
    );
}
