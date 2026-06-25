pub const MODULE: &str = "completion_authority";

pub const BLOCKER_OUTCOME_CONTRADICTION: &str = "host_bridge_completion_outcome_contradiction";
pub const BLOCKER_PROVENANCE_REJECTED: &str = "host_bridge_completion_provenance_rejected";
pub const BLOCKER_RECEIPT_NOT_BOUND: &str = "host_bridge_completion_receipt_not_bound";
pub const BLOCKER_TYPED_BLOCKED_OUTCOME: &str = "host_bridge_completion_blocked";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostBridgeCompletionState {
    Pending,
    ResultReceived,
    Validating,
    Passed,
    Blocked,
    Failed,
    EvidenceCommitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostBridgeCompletionEvent {
    ResultReceived,
    CompletionAccepted,
    CompletionRejected,
    EvidenceCommitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostBridgeCompletionEffectIntent {
    CommitEvidence,
    PlanNextStepPacket,
    RecordBlocker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeCompletionAuthorityInput {
    pub decision: String,
    pub verdict: String,
    pub blocker_codes: Vec<String>,
    pub summary: Option<String>,
    pub provenance_valid: bool,
    pub receipt_bound: bool,
    pub next_step_packet_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeCompletionAuthorityDecision {
    pub final_state: HostBridgeCompletionState,
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub events: Vec<HostBridgeCompletionEvent>,
    pub effect_intents: Vec<HostBridgeCompletionEffectIntent>,
    pub next_step_packet_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeCompletionTransitionCase {
    pub name: &'static str,
    pub input: HostBridgeCompletionAuthorityInput,
    pub expected_state: HostBridgeCompletionState,
}

#[must_use]
pub fn decide_host_bridge_completion_authority(
    input: HostBridgeCompletionAuthorityInput,
) -> HostBridgeCompletionAuthorityDecision {
    let mut blockers = typed_blockers(&input);
    let passed = completion_tuple_is_passed(&input);
    let blocked = completion_tuple_is_blocked(&input);

    if passed && blocked {
        push_unique(&mut blockers, BLOCKER_OUTCOME_CONTRADICTION);
    }
    if !input.provenance_valid {
        push_unique(&mut blockers, BLOCKER_PROVENANCE_REJECTED);
    }
    if !input.receipt_bound {
        push_unique(&mut blockers, BLOCKER_RECEIPT_NOT_BOUND);
    }

    if blockers.is_empty() && passed {
        return accepted_decision(input.next_step_packet_requested);
    }

    rejected_decision(if blockers.is_empty() {
        vec![BLOCKER_TYPED_BLOCKED_OUTCOME.to_string()]
    } else {
        blockers
    })
}

#[must_use]
pub fn completion_authority_transition_matrix() -> Vec<HostBridgeCompletionTransitionCase> {
    vec![
        HostBridgeCompletionTransitionCase {
            name: "passed_empty_blockers",
            input: input("approve", "pass", [], Some("proof passed")),
            expected_state: HostBridgeCompletionState::Passed,
        },
        HostBridgeCompletionTransitionCase {
            name: "passed_summary_mentions_blocker",
            input: input(
                "approve",
                "pass",
                [],
                Some("proof passed; previous blocker was resolved"),
            ),
            expected_state: HostBridgeCompletionState::Passed,
        },
        HostBridgeCompletionTransitionCase {
            name: "blocked_typed_blocker",
            input: input("approve", "blocked", ["implementation_missing"], None),
            expected_state: HostBridgeCompletionState::Blocked,
        },
        HostBridgeCompletionTransitionCase {
            name: "contradictory_pass_with_blocker",
            input: input("approve", "pass", ["host_agent_execution_failed"], None),
            expected_state: HostBridgeCompletionState::Failed,
        },
        HostBridgeCompletionTransitionCase {
            name: "receipt_not_bound",
            input: HostBridgeCompletionAuthorityInput {
                receipt_bound: false,
                ..input("approve", "pass", [], None)
            },
            expected_state: HostBridgeCompletionState::Failed,
        },
        HostBridgeCompletionTransitionCase {
            name: "provenance_rejected",
            input: HostBridgeCompletionAuthorityInput {
                provenance_valid: false,
                ..input("approve", "pass", [], None)
            },
            expected_state: HostBridgeCompletionState::Failed,
        },
    ]
}

fn accepted_decision(next_step_packet_requested: bool) -> HostBridgeCompletionAuthorityDecision {
    let mut effect_intents = vec![HostBridgeCompletionEffectIntent::CommitEvidence];
    if next_step_packet_requested {
        effect_intents.push(HostBridgeCompletionEffectIntent::PlanNextStepPacket);
    }
    HostBridgeCompletionAuthorityDecision {
        final_state: HostBridgeCompletionState::Passed,
        accepted: true,
        blocker_codes: Vec::new(),
        events: vec![
            HostBridgeCompletionEvent::ResultReceived,
            HostBridgeCompletionEvent::CompletionAccepted,
            HostBridgeCompletionEvent::EvidenceCommitted,
        ],
        effect_intents,
        next_step_packet_admitted: next_step_packet_requested,
    }
}

fn rejected_decision(blocker_codes: Vec<String>) -> HostBridgeCompletionAuthorityDecision {
    let final_state = if blocker_codes.iter().any(|blocker| {
        matches!(
            blocker.as_str(),
            BLOCKER_OUTCOME_CONTRADICTION | BLOCKER_PROVENANCE_REJECTED | BLOCKER_RECEIPT_NOT_BOUND
        )
    }) {
        HostBridgeCompletionState::Failed
    } else {
        HostBridgeCompletionState::Blocked
    };
    HostBridgeCompletionAuthorityDecision {
        final_state,
        accepted: false,
        blocker_codes,
        events: vec![
            HostBridgeCompletionEvent::ResultReceived,
            HostBridgeCompletionEvent::CompletionRejected,
        ],
        effect_intents: vec![HostBridgeCompletionEffectIntent::RecordBlocker],
        next_step_packet_admitted: false,
    }
}

fn typed_blockers(input: &HostBridgeCompletionAuthorityInput) -> Vec<String> {
    input
        .blocker_codes
        .iter()
        .map(|blocker| blocker.trim())
        .filter(|blocker| !blocker.is_empty())
        .map(ToOwned::to_owned)
        .fold(Vec::new(), |mut blockers, blocker| {
            push_unique(&mut blockers, &blocker);
            blockers
        })
}

fn completion_tuple_is_passed(input: &HostBridgeCompletionAuthorityInput) -> bool {
    normalized(&input.decision) == "approve"
        && matches!(normalized(&input.verdict).as_str(), "pass" | "passed")
}

fn completion_tuple_is_blocked(input: &HostBridgeCompletionAuthorityInput) -> bool {
    !input.blocker_codes.is_empty()
        || matches!(
            normalized(&input.verdict).as_str(),
            "blocked" | "fail" | "failed" | "rework_required"
        )
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn push_unique(blockers: &mut Vec<String>, blocker: &str) {
    if !blockers.iter().any(|candidate| candidate == blocker) {
        blockers.push(blocker.to_string());
    }
}

fn input<const N: usize>(
    decision: &str,
    verdict: &str,
    blocker_codes: [&str; N],
    summary: Option<&str>,
) -> HostBridgeCompletionAuthorityInput {
    HostBridgeCompletionAuthorityInput {
        decision: decision.to_string(),
        verdict: verdict.to_string(),
        blocker_codes: blocker_codes
            .iter()
            .map(|blocker| (*blocker).to_string())
            .collect(),
        summary: summary.map(ToOwned::to_owned),
        provenance_valid: true,
        receipt_bound: true,
        next_step_packet_requested: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BLOCKER_OUTCOME_CONTRADICTION, HostBridgeCompletionEffectIntent, HostBridgeCompletionState,
        completion_authority_transition_matrix, decide_host_bridge_completion_authority, input,
    };

    #[test]
    fn passed_with_empty_blockers_cannot_emit_blocked_event_or_blocker() {
        let decision = decide_host_bridge_completion_authority(input(
            "approve",
            "pass",
            [],
            Some("blocked text in resolved summary"),
        ));

        assert!(decision.accepted);
        assert_eq!(decision.final_state, HostBridgeCompletionState::Passed);
        assert!(decision.blocker_codes.is_empty());
        assert!(decision.next_step_packet_admitted);
        assert!(
            decision
                .effect_intents
                .contains(&HostBridgeCompletionEffectIntent::PlanNextStepPacket)
        );
    }

    #[test]
    fn summary_text_cannot_change_typed_outcome() {
        let pass = decide_host_bridge_completion_authority(input(
            "approve",
            "pass",
            [],
            Some("verdict: blocked; rework required"),
        ));
        let blocked = decide_host_bridge_completion_authority(input(
            "approve",
            "blocked",
            ["coach_rework_required"],
            Some("everything looks fine"),
        ));

        assert_eq!(pass.final_state, HostBridgeCompletionState::Passed);
        assert_eq!(blocked.final_state, HostBridgeCompletionState::Blocked);
        assert_eq!(blocked.blocker_codes, vec!["coach_rework_required"]);
    }

    #[test]
    fn next_step_packet_planning_requires_accepted_completion() {
        let decision = decide_host_bridge_completion_authority(input(
            "approve",
            "blocked",
            ["implementation_missing"],
            None,
        ));

        assert!(!decision.accepted);
        assert!(!decision.next_step_packet_admitted);
        assert!(
            !decision
                .effect_intents
                .contains(&HostBridgeCompletionEffectIntent::PlanNextStepPacket)
        );
    }

    #[test]
    fn exhaustive_transition_matrix_matches_expected_states() {
        for case in completion_authority_transition_matrix() {
            let decision = decide_host_bridge_completion_authority(case.input);
            assert_eq!(decision.final_state, case.expected_state, "{}", case.name);
        }
    }

    #[test]
    fn contradictory_pass_with_blockers_is_failed_not_blocked() {
        let decision = decide_host_bridge_completion_authority(input(
            "approve",
            "pass",
            ["host_agent_execution_failed"],
            None,
        ));

        assert_eq!(decision.final_state, HostBridgeCompletionState::Failed);
        assert!(
            decision
                .blocker_codes
                .iter()
                .any(|blocker| blocker == BLOCKER_OUTCOME_CONTRADICTION)
        );
        assert!(!decision.next_step_packet_admitted);
    }
}
