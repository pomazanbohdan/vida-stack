pub const MODULE: &str = "completion_authority";

pub const BLOCKER_OUTCOME_CONTRADICTION: &str = "host_bridge_completion_outcome_contradiction";
pub const BLOCKER_PROVENANCE_REJECTED: &str = "host_bridge_completion_provenance_rejected";
pub const BLOCKER_RECEIPT_NOT_BOUND: &str = "host_bridge_completion_receipt_not_bound";
pub const BLOCKER_TYPED_BLOCKED_OUTCOME: &str = "host_bridge_completion_blocked";
pub const BLOCKER_SUMMARY_DERIVED: &str = "host_bridge_completion_summary_blocked";

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
    blockers.extend(summary_blocker_codes(input.summary.as_deref()));
    dedup_blockers(&mut blockers);
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
            name: "passed_summary_mentions_resolved_blocker",
            input: input(
                "approve",
                "pass",
                [],
                Some("proof passed; previous blocker was resolved"),
            ),
            expected_state: HostBridgeCompletionState::Passed,
        },
        HostBridgeCompletionTransitionCase {
            name: "summary_only_blocked_outcome",
            input: input(
                "approve",
                "pass",
                [],
                Some(
                    "verdict: blocker; rework required; implementation evidence missing; not closure-ready",
                ),
            ),
            expected_state: HostBridgeCompletionState::Blocked,
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

#[must_use]
pub fn summary_blocker_codes(summary: Option<&str>) -> Vec<String> {
    let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };
    if summary_text_reports_blocked_completion(summary) {
        vec![BLOCKER_SUMMARY_DERIVED.to_string()]
    } else {
        Vec::new()
    }
}

#[must_use]
pub fn summary_text_reports_blocked_completion(summary: &str) -> bool {
    let normalized = normalized_summary(summary);
    let resolved_context = [
        "resolved blocker",
        "blocker resolved",
        "previous blocker was resolved",
        "no blocker",
        "no blockers",
        "not blocked",
        "not a blocker",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));
    if resolved_context {
        return false;
    }
    [
        "verdict blocker",
        "verdict blocked",
        "decision blocked",
        "completion blocked",
        "closure blocked",
        "not closure ready",
        "not closure_ready",
        "closure ready false",
        "rework required",
        "implementation evidence missing",
        "evidence missing",
        "execution failed",
        "completion failed",
        "failed completion",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn normalized_summary(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn dedup_blockers(blockers: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for blocker in blockers.drain(..) {
        push_unique(&mut deduped, &blocker);
    }
    *blockers = deduped;
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
        BLOCKER_OUTCOME_CONTRADICTION, BLOCKER_SUMMARY_DERIVED, HostBridgeCompletionEffectIntent,
        HostBridgeCompletionEvent, HostBridgeCompletionState,
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
            !decision
                .events
                .contains(&HostBridgeCompletionEvent::CompletionRejected)
        );
        assert!(
            !decision
                .effect_intents
                .contains(&HostBridgeCompletionEffectIntent::RecordBlocker)
        );
        assert!(
            decision
                .effect_intents
                .contains(&HostBridgeCompletionEffectIntent::PlanNextStepPacket)
        );
    }

    #[test]
    fn summary_text_derived_blocker_rejects_summary_only_negative_outcome() {
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

        assert_eq!(pass.final_state, HostBridgeCompletionState::Blocked);
        assert!(!pass.accepted);
        assert_eq!(pass.blocker_codes, vec![BLOCKER_SUMMARY_DERIVED]);
        assert_eq!(blocked.final_state, HostBridgeCompletionState::Blocked);
        assert!(!blocked.accepted);
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
            decision
                .events
                .contains(&HostBridgeCompletionEvent::CompletionRejected)
        );
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
            assert_eq!(
                decision.accepted,
                case.expected_state == HostBridgeCompletionState::Passed,
                "{}",
                case.name
            );
            assert_eq!(
                decision.next_step_packet_admitted,
                decision.accepted
                    && decision
                        .effect_intents
                        .contains(&HostBridgeCompletionEffectIntent::PlanNextStepPacket),
                "{}",
                case.name
            );
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
