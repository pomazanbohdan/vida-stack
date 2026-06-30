pub const MODULE: &str = "completion_authority";

pub const BLOCKER_OUTCOME_CONTRADICTION: &str = "host_bridge_completion_outcome_contradiction";
pub const BLOCKER_PROVENANCE_REJECTED: &str = "host_bridge_completion_provenance_rejected";
pub const BLOCKER_RECEIPT_NOT_BOUND: &str = "host_bridge_completion_receipt_not_bound";
pub const BLOCKER_TYPED_BLOCKED_OUTCOME: &str = "host_bridge_completion_blocked";
pub const BLOCKER_TYPED_FAILED_OUTCOME: &str = "host_bridge_completion_failed";
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
    pub expected_events: Vec<HostBridgeCompletionEvent>,
    pub expected_effect_intents: Vec<HostBridgeCompletionEffectIntent>,
    pub expected_next_step_packet_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeCompletionTransition {
    pub from_state: HostBridgeCompletionState,
    pub event: HostBridgeCompletionEvent,
    pub to_state: HostBridgeCompletionState,
    pub effect_intents: Vec<HostBridgeCompletionEffectIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeCompletionFsm {
    state: HostBridgeCompletionState,
    transitions: Vec<HostBridgeCompletionTransition>,
}

impl Default for HostBridgeCompletionFsm {
    fn default() -> Self {
        Self::new()
    }
}

impl HostBridgeCompletionFsm {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: HostBridgeCompletionState::Pending,
            transitions: Vec::new(),
        }
    }

    #[must_use]
    pub fn state(&self) -> HostBridgeCompletionState {
        self.state
    }

    #[must_use]
    pub fn transitions(&self) -> &[HostBridgeCompletionTransition] {
        &self.transitions
    }

    #[must_use]
    pub fn decide(
        self,
        input: HostBridgeCompletionAuthorityInput,
    ) -> HostBridgeCompletionAuthorityDecision {
        let next_step_packet_requested = input.next_step_packet_requested;
        let (outcome, blockers) = derive_completion_authority_outcome(&input);
        self.apply_outcome(outcome, blockers, next_step_packet_requested)
    }

    fn apply_outcome(
        mut self,
        outcome: HostBridgeCompletionOutcome,
        blocker_codes: Vec<String>,
        next_step_packet_requested: bool,
    ) -> HostBridgeCompletionAuthorityDecision {
        self.transition(
            HostBridgeCompletionEvent::ResultReceived,
            HostBridgeCompletionState::ResultReceived,
            Vec::new(),
        );

        let row = COMPLETION_AUTHORITY_TRANSITIONS
            .iter()
            .find(|row| row.outcome == outcome)
            .expect("host bridge completion outcome should have a transition row");
        let next_step_packet_admitted = matches!(
            row.next_step_packet_policy,
            NextStepPacketPolicy::AdmitWhenRequested
        ) && next_step_packet_requested;
        let mut effect_intents = row.effect_intents.to_vec();
        if next_step_packet_admitted {
            effect_intents.push(HostBridgeCompletionEffectIntent::PlanNextStepPacket);
        }
        self.transition(row.event, row.final_state, effect_intents);
        if row.accepted {
            self.transition(
                HostBridgeCompletionEvent::EvidenceCommitted,
                row.final_state,
                Vec::new(),
            );
        }

        let events = self
            .transitions
            .iter()
            .map(|transition| transition.event)
            .collect::<Vec<_>>();
        let effect_intents = self
            .transitions
            .iter()
            .flat_map(|transition| transition.effect_intents.iter().copied())
            .collect::<Vec<_>>();

        HostBridgeCompletionAuthorityDecision {
            final_state: row.final_state,
            accepted: row.accepted,
            blocker_codes,
            events,
            effect_intents,
            next_step_packet_admitted,
        }
    }

    fn transition(
        &mut self,
        event: HostBridgeCompletionEvent,
        to_state: HostBridgeCompletionState,
        effect_intents: Vec<HostBridgeCompletionEffectIntent>,
    ) {
        let from_state = self.state;
        self.transitions.push(HostBridgeCompletionTransition {
            from_state,
            event,
            to_state,
            effect_intents,
        });
        self.state = to_state;
    }
}

#[must_use]
pub fn decide_host_bridge_completion_authority(
    input: HostBridgeCompletionAuthorityInput,
) -> HostBridgeCompletionAuthorityDecision {
    HostBridgeCompletionFsm::new().decide(input)
}

#[must_use]
fn derive_completion_authority_outcome(
    input: &HostBridgeCompletionAuthorityInput,
) -> (HostBridgeCompletionOutcome, Vec<String>) {
    let mut blockers = typed_blockers(&input);
    blockers.extend(summary_blocker_codes(input.summary.as_deref()));
    dedup_blockers(&mut blockers);
    let passed = completion_tuple_is_passed(&input);
    let failed = completion_tuple_is_failed(&input);
    let blocked = completion_tuple_is_blocked(&input) || failed;

    if passed && blocked {
        push_unique(&mut blockers, BLOCKER_OUTCOME_CONTRADICTION);
    }
    if !input.provenance_valid {
        push_unique(&mut blockers, BLOCKER_PROVENANCE_REJECTED);
    }
    if !input.receipt_bound {
        push_unique(&mut blockers, BLOCKER_RECEIPT_NOT_BOUND);
    }

    if blockers
        .iter()
        .any(|blocker| failure_blocker_code(blocker.as_str()))
    {
        return (HostBridgeCompletionOutcome::Failed, blockers);
    }

    if failed {
        push_unique(&mut blockers, BLOCKER_TYPED_FAILED_OUTCOME);
        return (HostBridgeCompletionOutcome::Failed, blockers);
    }

    if blockers.is_empty() && passed {
        return (HostBridgeCompletionOutcome::Passed, blockers);
    }

    if blockers.is_empty() {
        push_unique(&mut blockers, BLOCKER_TYPED_BLOCKED_OUTCOME);
    }
    (HostBridgeCompletionOutcome::Blocked, blockers)
}

#[must_use]
pub fn completion_authority_transition_matrix() -> Vec<HostBridgeCompletionTransitionCase> {
    vec![
        HostBridgeCompletionTransitionCase {
            name: "passed_empty_blockers",
            input: input("approve", "pass", [], Some("proof passed")),
            expected_state: HostBridgeCompletionState::Passed,
            expected_events: accepted_events(),
            expected_effect_intents: accepted_effect_intents(true),
            expected_next_step_packet_admitted: true,
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
            expected_events: accepted_events(),
            expected_effect_intents: accepted_effect_intents(true),
            expected_next_step_packet_admitted: true,
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
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "blocked_typed_blocker",
            input: input("approve", "blocked", ["implementation_missing"], None),
            expected_state: HostBridgeCompletionState::Blocked,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "explicit_rework_without_blocker_derives_typed_blocked_outcome",
            input: input("rework_required", "rework_required", [], None),
            expected_state: HostBridgeCompletionState::Blocked,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "explicit_failed_without_blocker_is_failed",
            input: input("failed", "failed", [], None),
            expected_state: HostBridgeCompletionState::Failed,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "contradictory_pass_with_blocker",
            input: input("approve", "pass", ["host_agent_execution_failed"], None),
            expected_state: HostBridgeCompletionState::Failed,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "receipt_not_bound",
            input: HostBridgeCompletionAuthorityInput {
                receipt_bound: false,
                ..input("approve", "pass", [], None)
            },
            expected_state: HostBridgeCompletionState::Failed,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
        HostBridgeCompletionTransitionCase {
            name: "provenance_rejected",
            input: HostBridgeCompletionAuthorityInput {
                provenance_valid: false,
                ..input("approve", "pass", [], None)
            },
            expected_state: HostBridgeCompletionState::Failed,
            expected_events: rejected_events(),
            expected_effect_intents: rejected_effect_intents(),
            expected_next_step_packet_admitted: false,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostBridgeCompletionOutcome {
    Passed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextStepPacketPolicy {
    AdmitWhenRequested,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HostBridgeCompletionTransitionRow {
    outcome: HostBridgeCompletionOutcome,
    final_state: HostBridgeCompletionState,
    accepted: bool,
    event: HostBridgeCompletionEvent,
    events: &'static [HostBridgeCompletionEvent],
    effect_intents: &'static [HostBridgeCompletionEffectIntent],
    next_step_packet_policy: NextStepPacketPolicy,
}

const ACCEPTED_EVENTS: &[HostBridgeCompletionEvent] = &[
    HostBridgeCompletionEvent::ResultReceived,
    HostBridgeCompletionEvent::CompletionAccepted,
    HostBridgeCompletionEvent::EvidenceCommitted,
];
const REJECTED_EVENTS: &[HostBridgeCompletionEvent] = &[
    HostBridgeCompletionEvent::ResultReceived,
    HostBridgeCompletionEvent::CompletionRejected,
];
const ACCEPTED_EFFECT_INTENTS: &[HostBridgeCompletionEffectIntent] =
    &[HostBridgeCompletionEffectIntent::CommitEvidence];
const REJECTED_EFFECT_INTENTS: &[HostBridgeCompletionEffectIntent] =
    &[HostBridgeCompletionEffectIntent::RecordBlocker];

const COMPLETION_AUTHORITY_TRANSITIONS: &[HostBridgeCompletionTransitionRow] = &[
    HostBridgeCompletionTransitionRow {
        outcome: HostBridgeCompletionOutcome::Passed,
        final_state: HostBridgeCompletionState::Passed,
        accepted: true,
        event: HostBridgeCompletionEvent::CompletionAccepted,
        events: ACCEPTED_EVENTS,
        effect_intents: ACCEPTED_EFFECT_INTENTS,
        next_step_packet_policy: NextStepPacketPolicy::AdmitWhenRequested,
    },
    HostBridgeCompletionTransitionRow {
        outcome: HostBridgeCompletionOutcome::Blocked,
        final_state: HostBridgeCompletionState::Blocked,
        accepted: false,
        event: HostBridgeCompletionEvent::CompletionRejected,
        events: REJECTED_EVENTS,
        effect_intents: REJECTED_EFFECT_INTENTS,
        next_step_packet_policy: NextStepPacketPolicy::Never,
    },
    HostBridgeCompletionTransitionRow {
        outcome: HostBridgeCompletionOutcome::Failed,
        final_state: HostBridgeCompletionState::Failed,
        accepted: false,
        event: HostBridgeCompletionEvent::CompletionRejected,
        events: REJECTED_EVENTS,
        effect_intents: REJECTED_EFFECT_INTENTS,
        next_step_packet_policy: NextStepPacketPolicy::Never,
    },
];

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
    matches!(
        normalized(&input.decision).as_str(),
        "approve" | "pass" | "passed"
    ) && matches!(normalized(&input.verdict).as_str(), "pass" | "passed")
}

fn completion_tuple_is_blocked(input: &HostBridgeCompletionAuthorityInput) -> bool {
    !input.blocker_codes.is_empty()
        || matches!(
            normalized(&input.decision).as_str(),
            "blocked" | "rework_required"
        )
        || matches!(
            normalized(&input.verdict).as_str(),
            "blocked" | "rework_required"
        )
}

fn completion_tuple_is_failed(input: &HostBridgeCompletionAuthorityInput) -> bool {
    matches!(normalized(&input.decision).as_str(), "fail" | "failed")
        || matches!(normalized(&input.verdict).as_str(), "fail" | "failed")
}

fn failure_blocker_code(blocker: &str) -> bool {
    matches!(
        blocker,
        BLOCKER_OUTCOME_CONTRADICTION
            | BLOCKER_PROVENANCE_REJECTED
            | BLOCKER_RECEIPT_NOT_BOUND
            | BLOCKER_TYPED_FAILED_OUTCOME
    )
}

fn accepted_events() -> Vec<HostBridgeCompletionEvent> {
    ACCEPTED_EVENTS.to_vec()
}

fn rejected_events() -> Vec<HostBridgeCompletionEvent> {
    REJECTED_EVENTS.to_vec()
}

fn accepted_effect_intents(
    next_step_packet_requested: bool,
) -> Vec<HostBridgeCompletionEffectIntent> {
    let mut effect_intents = ACCEPTED_EFFECT_INTENTS.to_vec();
    if next_step_packet_requested {
        effect_intents.push(HostBridgeCompletionEffectIntent::PlanNextStepPacket);
    }
    effect_intents
}

fn rejected_effect_intents() -> Vec<HostBridgeCompletionEffectIntent> {
    REJECTED_EFFECT_INTENTS.to_vec()
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
        BLOCKER_OUTCOME_CONTRADICTION, BLOCKER_SUMMARY_DERIVED, BLOCKER_TYPED_BLOCKED_OUTCOME,
        BLOCKER_TYPED_FAILED_OUTCOME, HostBridgeCompletionEffectIntent, HostBridgeCompletionEvent,
        HostBridgeCompletionFsm, HostBridgeCompletionState, completion_authority_transition_matrix,
        decide_host_bridge_completion_authority, input,
    };

    #[test]
    fn passed_with_empty_blockers_cannot_emit_blocked_event_or_blocker() {
        let decision = decide_host_bridge_completion_authority(input(
            "pass",
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
    fn typed_pass_decision_alias_is_accepted_completion() {
        let decision = decide_host_bridge_completion_authority(input("pass", "pass", [], None));

        assert!(decision.accepted);
        assert_eq!(decision.final_state, HostBridgeCompletionState::Passed);
        assert!(decision.blocker_codes.is_empty());
        assert!(decision.next_step_packet_admitted);
        assert!(
            decision
                .events
                .contains(&HostBridgeCompletionEvent::CompletionAccepted)
        );
        assert!(
            !decision
                .events
                .contains(&HostBridgeCompletionEvent::CompletionRejected)
        );
    }

    #[test]
    fn fsm_boundary_is_the_public_completion_decision_owner() {
        let decision = HostBridgeCompletionFsm::new().decide(input(
            "approve",
            "pass",
            [],
            Some("proof passed"),
        ));

        assert_eq!(decision.final_state, HostBridgeCompletionState::Passed);
        assert_eq!(
            decision.events,
            vec![
                HostBridgeCompletionEvent::ResultReceived,
                HostBridgeCompletionEvent::CompletionAccepted,
                HostBridgeCompletionEvent::EvidenceCommitted,
            ]
        );
        assert_eq!(
            decision.effect_intents,
            vec![
                HostBridgeCompletionEffectIntent::CommitEvidence,
                HostBridgeCompletionEffectIntent::PlanNextStepPacket,
            ]
        );
        assert!(decision.next_step_packet_admitted);
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
                decision.next_step_packet_admitted, case.expected_next_step_packet_admitted,
                "{}",
                case.name
            );
            assert_eq!(decision.events, case.expected_events, "{}", case.name);
            assert_eq!(
                decision.effect_intents, case.expected_effect_intents,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn explicit_rework_without_blocker_is_blocked_not_failed() {
        let decision = decide_host_bridge_completion_authority(input(
            "rework_required",
            "rework_required",
            [],
            None,
        ));

        assert_eq!(decision.final_state, HostBridgeCompletionState::Blocked);
        assert_eq!(decision.blocker_codes, vec![BLOCKER_TYPED_BLOCKED_OUTCOME]);
        assert!(!decision.next_step_packet_admitted);
    }

    #[test]
    fn explicit_failed_without_blocker_is_failed_not_retryable_blocked() {
        let decision = decide_host_bridge_completion_authority(input("failed", "failed", [], None));

        assert_eq!(decision.final_state, HostBridgeCompletionState::Failed);
        assert_eq!(decision.blocker_codes, vec![BLOCKER_TYPED_FAILED_OUTCOME]);
        assert!(!decision.next_step_packet_admitted);
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
