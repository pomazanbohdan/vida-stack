use serde_json::Value;

pub const MODULE: &str = "completion_authority";

pub const BLOCKER_OUTCOME_CONTRADICTION: &str = "host_bridge_completion_outcome_contradiction";
pub const BLOCKER_PROVENANCE_REJECTED: &str = "host_bridge_completion_provenance_rejected";
pub const BLOCKER_RECEIPT_NOT_BOUND: &str = "host_bridge_completion_receipt_not_bound";
pub const BLOCKER_TYPED_BLOCKED_OUTCOME: &str = "host_bridge_completion_blocked";
pub const BLOCKER_TYPED_FAILED_OUTCOME: &str = "host_bridge_completion_failed";
pub const BLOCKER_SUMMARY_DERIVED: &str = "host_bridge_completion_summary_blocked";

pub const BLOCKER_ATTEMPT_SCOPE_INCOMPLETE: &str = "implementation_attempt_scope_incomplete";
pub const BLOCKER_ATTEMPT_EMPTY_PATCH: &str = "implementation_artifact_has_no_changed_files";
pub const BLOCKER_ATTEMPT_SCOPE_GUARD: &str = "implementation_attempt_scope_guard_violation";
pub const BLOCKER_ATTEMPT_CANONICAL_WORKTREE: &str = "isolated_worktree_canonical_worktree_touched";
pub const BLOCKER_ATTEMPT_CANONICAL_EVIDENCE: &str =
    "isolated_worktree_canonical_worktree_evidence_missing";
pub const BLOCKER_ATTEMPT_LINE_ENDING_CHURN: &str =
    "implementation_artifact_broad_line_ending_churn";
pub const BLOCKER_ATTEMPT_CAPABILITY: &str = "host_bridge_capability_blocked";
pub const BLOCKER_ATTEMPT_NO_REPEAT: &str = "host_bridge_retry_no_repeat";
pub const BLOCKER_ATTEMPT_RETRY_RECEIPT: &str = "host_bridge_retry_receipt_mismatch";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBridgeImplementationAttemptAdmission {
    pub decision: String,
    pub verdict: String,
    pub blocker_codes: Vec<String>,
    pub reroute_allowed: bool,
    pub reroute_carrier_id: Option<String>,
    pub fingerprint: String,
}

impl HostBridgeImplementationAttemptAdmission {
    #[must_use]
    pub fn admit(request: &Value, artifacts: Option<&Value>) -> Self {
        let mut blockers = Vec::new();
        let isolation = request
            .get("implementation_isolation")
            .filter(|value| value.is_object());
        let implementation_request = request
            .get("task_class")
            .and_then(Value::as_str)
            .is_some_and(|value| {
                matches!(
                    value.trim(),
                    "implementation"
                        | "implementation_medium"
                        | "delivery_task"
                        | "execution_block"
                        | "writer"
                )
            });
        if implementation_request {
            let Some(isolation) = isolation.and_then(Value::as_object) else {
                push_unique(&mut blockers, BLOCKER_ATTEMPT_SCOPE_INCOMPLETE);
                return Self::terminal(blockers, fingerprint(request, &[]));
            };
            let owned_paths = string_array(isolation.get("owned_paths"));
            let scope_policy_ok = isolation
                .get("scope_policy")
                .and_then(Value::as_object)
                .and_then(|policy| {
                    policy
                        .get("changed_files_must_be_subset_of_owned_paths")
                        .and_then(Value::as_bool)
                })
                == Some(true);
            if owned_paths.is_empty()
                || isolation
                    .get("canonical_worktree_writes_allowed")
                    .and_then(Value::as_bool)
                    != Some(false)
                || !scope_policy_ok
            {
                push_unique(&mut blockers, BLOCKER_ATTEMPT_SCOPE_INCOMPLETE);
            }

            if let Some(artifacts) = artifacts {
                validate_artifacts(artifacts, &owned_paths, &mut blockers);
            }
        }

        let capability_blockers = capability_blockers(request);
        let current_fingerprint = fingerprint(request, &capability_blockers);
        if !capability_blockers.is_empty() {
            push_unique(&mut blockers, BLOCKER_ATTEMPT_CAPABILITY);
            if retry_receipt_is_present(request) && !retry_receipt_matches(request) {
                push_unique(&mut blockers, BLOCKER_ATTEMPT_RETRY_RECEIPT);
            }
            let retry_count = request
                .get("retry_count")
                .and_then(Value::as_u64)
                .or_else(|| {
                    request
                        .pointer("/retry_context/retry_count")
                        .and_then(Value::as_u64)
                })
                .unwrap_or_default();
            let previous_fingerprint = request
                .get("previous_attempt_fingerprint")
                .and_then(Value::as_str)
                .or_else(|| {
                    request
                        .pointer("/retry_context/previous_fingerprint")
                        .and_then(Value::as_str)
                });
            if retry_count > 0 && previous_fingerprint == Some(current_fingerprint.as_str()) {
                push_unique(&mut blockers, BLOCKER_ATTEMPT_NO_REPEAT);
            }
            if !blockers.iter().any(|code| {
                matches!(
                    code.as_str(),
                    BLOCKER_ATTEMPT_SCOPE_INCOMPLETE
                        | BLOCKER_ATTEMPT_EMPTY_PATCH
                        | BLOCKER_ATTEMPT_SCOPE_GUARD
                        | BLOCKER_ATTEMPT_CANONICAL_WORKTREE
                        | BLOCKER_ATTEMPT_CANONICAL_EVIDENCE
                        | BLOCKER_ATTEMPT_LINE_ENDING_CHURN
                        | BLOCKER_ATTEMPT_RETRY_RECEIPT
                        | BLOCKER_ATTEMPT_NO_REPEAT
                )
            }) && retry_count == 0
            {
                if let Some(carrier_id) = cheapest_eligible_carrier(request) {
                    return Self {
                        decision: "reroute_once".to_string(),
                        verdict: "capability_blocked".to_string(),
                        blocker_codes: blockers,
                        reroute_allowed: true,
                        reroute_carrier_id: Some(carrier_id),
                        fingerprint: current_fingerprint,
                    };
                }
            }
        }

        if blockers.is_empty() {
            Self {
                decision: "admit".to_string(),
                verdict: "pass".to_string(),
                blocker_codes: blockers,
                reroute_allowed: false,
                reroute_carrier_id: None,
                fingerprint: current_fingerprint,
            }
        } else {
            Self::terminal(blockers, current_fingerprint)
        }
    }

    fn terminal(blocker_codes: Vec<String>, fingerprint: String) -> Self {
        Self {
            decision: "terminal_blocker".to_string(),
            verdict: "blocked".to_string(),
            blocker_codes,
            reroute_allowed: false,
            reroute_carrier_id: None,
            fingerprint,
        }
    }
}

#[must_use]
pub fn admit_host_bridge_implementation_attempt(
    request: &Value,
    artifacts: Option<&Value>,
) -> HostBridgeImplementationAttemptAdmission {
    HostBridgeImplementationAttemptAdmission::admit(request, artifacts)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn validate_artifacts(artifacts: &Value, owned_paths: &[String], blockers: &mut Vec<String>) {
    let Some(rows) = artifacts.as_array() else {
        push_unique(blockers, BLOCKER_ATTEMPT_SCOPE_INCOMPLETE);
        return;
    };
    if rows.is_empty() {
        push_unique(blockers, BLOCKER_ATTEMPT_EMPTY_PATCH);
    }
    for artifact in rows {
        let kind = artifact
            .get("artifact_kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let changed_files = string_array(artifact.get("changed_files"));
        if matches!(kind, "patch_proposal" | "isolated_worktree_manifest")
            && changed_files.is_empty()
        {
            push_unique(blockers, BLOCKER_ATTEMPT_EMPTY_PATCH);
        }
        if kind == "isolated_worktree_manifest" {
            if artifact
                .get("canonical_worktree_touched")
                .and_then(Value::as_bool)
                == Some(true)
                || artifact
                    .get("canonical_worktree_changed")
                    .and_then(Value::as_bool)
                    == Some(true)
            {
                push_unique(blockers, BLOCKER_ATTEMPT_CANONICAL_WORKTREE);
            } else if !canonical_worktree_untouched_is_proven(artifact) {
                push_unique(blockers, BLOCKER_ATTEMPT_CANONICAL_EVIDENCE);
            }
        }
        if changed_files
            .iter()
            .any(|path| !owned_paths.iter().any(|owned| path_within(path, owned)))
        {
            push_unique(blockers, BLOCKER_ATTEMPT_SCOPE_GUARD);
        }
        if reports_broad_line_ending_churn(artifact) {
            push_unique(blockers, BLOCKER_ATTEMPT_LINE_ENDING_CHURN);
        }
    }
}

fn canonical_worktree_untouched_is_proven(artifact: &Value) -> bool {
    artifact
        .get("canonical_worktree_unchanged")
        .and_then(Value::as_bool)
        == Some(true)
        || artifact
            .get("canonical_worktree_touched")
            .and_then(Value::as_bool)
            == Some(false)
        || artifact
            .get("canonical_worktree_changed")
            .and_then(Value::as_bool)
            == Some(false)
        || artifact
            .get("canonical_worktree")
            .and_then(Value::as_str)
            .is_some_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "unchanged" | "untouched" | "clean"
                )
            })
}

fn path_within(path: &str, owned: &str) -> bool {
    let path = path.trim().replace('\\', "/");
    let owned = owned.trim().replace('\\', "/");
    path == owned || path.starts_with(&(owned + "/"))
}

fn reports_broad_line_ending_churn(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase().replace(['-', ' '], "_");
            ((key.contains("line") && key.contains("ending")) || key.contains("churn"))
                && (value.as_bool() == Some(true)
                    || value.as_u64().is_some_and(|count| count > 1_000)
                    || value.as_str().is_some_and(|text| {
                        let text = text.to_ascii_lowercase();
                        text.contains("line-ending")
                            || text.contains("line ending")
                            || text.contains("normalization")
                            || text.contains("churn")
                    }))
                || reports_broad_line_ending_churn(value)
        }),
        Value::Array(values) => values.iter().any(reports_broad_line_ending_churn),
        _ => false,
    }
}

fn capability_blockers(request: &Value) -> Vec<String> {
    let mut blockers = string_array(request.get("capability_blockers"));
    blockers.extend(
        string_array(request.get("blocker_codes"))
            .into_iter()
            .filter(|code| {
                code.contains("capability")
                    || code.contains("host_tool")
                    || code.contains("host_agent")
            }),
    );
    blockers.sort();
    blockers.dedup();
    blockers
}

fn retry_receipt_is_present(request: &Value) -> bool {
    request.get("retry_receipt").is_some() || request.get("retry_receipt_id").is_some()
}

fn retry_receipt_matches(request: &Value) -> bool {
    let Some(receipt) = request.get("retry_receipt").and_then(Value::as_object) else {
        return false;
    };
    receipt.get("receipt_backed").and_then(Value::as_bool) == Some(true)
        && [
            "run_id",
            "task_id",
            "dispatch_target",
            "backend_id",
            "carrier_id",
        ]
        .iter()
        .all(|field| {
            request.get(*field).and_then(Value::as_str).is_none()
                || receipt.get(*field) == request.get(*field)
        })
        && request
            .get("retry_receipt_id")
            .and_then(Value::as_str)
            .is_none_or(|expected| {
                receipt.get("receipt_id").and_then(Value::as_str) == Some(expected)
            })
}

fn cheapest_eligible_carrier(request: &Value) -> Option<String> {
    request
        .get("eligible_carriers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|carrier| carrier.get("status").and_then(Value::as_str) != Some("blocked"))
        .filter_map(|carrier| {
            let id = carrier
                .get("carrier_id")
                .or_else(|| carrier.get("id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())?;
            let cost = carrier
                .get("normalized_cost_units")
                .or_else(|| carrier.get("cost"))
                .or_else(|| carrier.get("rate"))
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            Some((cost, id.to_string()))
        })
        .min_by(|left, right| left.cmp(right))
        .map(|(_, id)| id)
}

fn fingerprint(request: &Value, blocker_codes: &[String]) -> String {
    [
        request.get("backend_fingerprint").and_then(Value::as_str),
        request.get("backend_id").and_then(Value::as_str),
        request.get("carrier_fingerprint").and_then(Value::as_str),
        request.get("carrier_id").and_then(Value::as_str),
        request
            .get("capability_fingerprint")
            .and_then(Value::as_str),
        request.get("adapter_capability_id").and_then(Value::as_str),
        request.get("blocker_fingerprint").and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .chain(blocker_codes.iter().map(String::as_str))
    .collect::<Vec<_>>()
    .join("|")
}

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
    let rework = completion_tuple_is_rework(&input);
    let failed = completion_tuple_is_failed(&input);
    let blocked = completion_tuple_is_blocked(&input) || failed;

    if passed && blocked {
        push_unique(&mut blockers, BLOCKER_OUTCOME_CONTRADICTION);
    }
    if completion_tuple_has_contradictory_pass_alias(input) {
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

    if blockers.is_empty() && (passed || (rework && input.next_step_packet_requested)) {
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
            name: "explicit_rework_with_next_step_route_is_accepted",
            input: input("rework_required", "rework_required", [], None),
            expected_state: HostBridgeCompletionState::Passed,
            expected_events: accepted_events(),
            expected_effect_intents: accepted_effect_intents(true),
            expected_next_step_packet_admitted: true,
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
    let decision = normalized(&input.decision);
    let verdict = normalized(&input.verdict);
    matches!(decision.as_str(), "approve" | "pass" | "passed")
        && matches!(verdict.as_str(), "pass" | "passed")
        || decision.starts_with("pass_to_") && verdict == "test_contract_ready_with_expected_red"
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

fn completion_tuple_is_rework(input: &HostBridgeCompletionAuthorityInput) -> bool {
    matches!(normalized(&input.decision).as_str(), "rework_required")
        && matches!(normalized(&input.verdict).as_str(), "rework_required")
}

fn completion_tuple_is_failed(input: &HostBridgeCompletionAuthorityInput) -> bool {
    matches!(normalized(&input.decision).as_str(), "fail" | "failed")
        || matches!(normalized(&input.verdict).as_str(), "fail" | "failed")
}

fn completion_tuple_has_contradictory_pass_alias(
    input: &HostBridgeCompletionAuthorityInput,
) -> bool {
    let decision = normalized(&input.decision);
    let verdict = normalized(&input.verdict);
    decision.starts_with("pass_to_") && verdict != "test_contract_ready_with_expected_red"
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
        BLOCKER_ATTEMPT_CANONICAL_WORKTREE, BLOCKER_ATTEMPT_EMPTY_PATCH,
        BLOCKER_ATTEMPT_LINE_ENDING_CHURN, BLOCKER_ATTEMPT_NO_REPEAT,
        BLOCKER_ATTEMPT_RETRY_RECEIPT, BLOCKER_OUTCOME_CONTRADICTION, BLOCKER_SUMMARY_DERIVED,
        BLOCKER_TYPED_BLOCKED_OUTCOME, BLOCKER_TYPED_FAILED_OUTCOME,
        HostBridgeCompletionEffectIntent, HostBridgeCompletionEvent, HostBridgeCompletionFsm,
        HostBridgeCompletionState, admit_host_bridge_implementation_attempt,
        completion_authority_transition_matrix, decide_host_bridge_completion_authority,
        fingerprint, input,
    };
    use serde_json::{Value, json};

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
    fn expected_red_pass_to_developer_alias_is_accepted_completion() {
        let decision = decide_host_bridge_completion_authority(input(
            "pass_to_developer",
            "test_contract_ready_with_expected_red",
            [],
            None,
        ));

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
    fn pass_to_alias_with_wrong_verdict_fails_with_contradiction() {
        let decision = decide_host_bridge_completion_authority(input(
            "pass_to_developer",
            "blocked",
            [],
            None,
        ));

        assert!(!decision.accepted);
        assert_eq!(decision.final_state, HostBridgeCompletionState::Failed);
        assert!(
            decision
                .blocker_codes
                .iter()
                .any(|blocker| blocker == BLOCKER_OUTCOME_CONTRADICTION)
        );
        assert!(!decision.next_step_packet_admitted);
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
    fn explicit_rework_with_next_step_route_is_accepted_not_blocked() {
        let decision = decide_host_bridge_completion_authority(input(
            "rework_required",
            "rework_required",
            [],
            None,
        ));

        assert!(decision.accepted);
        assert_eq!(decision.final_state, HostBridgeCompletionState::Passed);
        assert!(decision.blocker_codes.is_empty());
        assert!(decision.next_step_packet_admitted);
    }

    #[test]
    fn explicit_rework_without_next_step_route_is_blocked_not_failed() {
        let mut input = input("rework_required", "rework_required", [], None);
        input.next_step_packet_requested = false;
        let decision = decide_host_bridge_completion_authority(input);

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

    fn implementation_request() -> Value {
        json!({
            "task_class": "implementation",
            "run_id": "run-1",
            "task_id": "run-1",
            "dispatch_target": "coder",
            "backend_id": "internal_subagents",
            "carrier_id": "coder",
            "implementation_isolation": {
                "canonical_worktree_writes_allowed": false,
                "owned_paths": ["crates/vida/src/lib.rs"],
                "scope_policy": {
                    "changed_files_must_be_subset_of_owned_paths": true
                }
            }
        })
    }

    #[test]
    fn implementation_attempt_admission_fails_closed_for_empty_scope_and_churn() {
        let request = implementation_request();
        let artifacts = json!([{
            "artifact_kind": "isolated_worktree_manifest",
            "changed_files": [],
            "canonical_worktree_touched": true,
            "line_ending_churn": true
        }]);
        let admission = admit_host_bridge_implementation_attempt(&request, Some(&artifacts));

        assert_eq!(admission.decision, "terminal_blocker");
        assert!(
            admission
                .blocker_codes
                .contains(&BLOCKER_ATTEMPT_EMPTY_PATCH.to_string())
        );
        assert!(
            admission
                .blocker_codes
                .contains(&BLOCKER_ATTEMPT_CANONICAL_WORKTREE.to_string())
        );
        assert!(
            admission
                .blocker_codes
                .contains(&BLOCKER_ATTEMPT_LINE_ENDING_CHURN.to_string())
        );
    }

    #[test]
    fn implementation_attempt_admission_reroutes_once_to_cheapest_carrier() {
        let mut request = implementation_request();
        request["capability_blockers"] = json!(["host_tool_capability_missing"]);
        request["eligible_carriers"] = json!([
            {"carrier_id": "expensive", "normalized_cost_units": 5},
            {"carrier_id": "cheap", "normalized_cost_units": 1}
        ]);
        let admission = admit_host_bridge_implementation_attempt(&request, None);

        assert_eq!(admission.decision, "reroute_once");
        assert!(admission.reroute_allowed);
        assert_eq!(admission.reroute_carrier_id.as_deref(), Some("cheap"));
    }

    #[test]
    fn unchanged_capability_fingerprint_is_terminal_and_retry_receipt_must_match() {
        let mut request = implementation_request();
        request["capability_blockers"] = json!(["host_tool_capability_missing"]);
        request["retry_count"] = json!(1);
        let fingerprint = fingerprint(&request, &["host_tool_capability_missing".to_string()]);
        request["previous_attempt_fingerprint"] = json!(fingerprint);
        request["retry_receipt"] = json!({
            "receipt_backed": false,
            "run_id": "other-run"
        });
        let admission = admit_host_bridge_implementation_attempt(&request, None);

        assert_eq!(admission.decision, "terminal_blocker");
        assert!(
            admission
                .blocker_codes
                .contains(&BLOCKER_ATTEMPT_NO_REPEAT.to_string())
        );
        assert!(
            admission
                .blocker_codes
                .contains(&BLOCKER_ATTEMPT_RETRY_RECEIPT.to_string())
        );
    }

    #[test]
    fn scoped_untouched_manifest_is_admitted() {
        let request = implementation_request();
        let artifacts = json!([{
            "artifact_kind": "isolated_worktree_manifest",
            "changed_files": ["crates/vida/src/lib.rs"],
            "canonical_worktree_unchanged": true,
            "line_ending_churn": false
        }]);
        let admission = admit_host_bridge_implementation_attempt(&request, Some(&artifacts));

        assert_eq!(admission.decision, "admit");
        assert_eq!(admission.verdict, "pass");
        assert!(admission.blocker_codes.is_empty());
    }

    #[test]
    fn legacy_nonimplementation_isolation_does_not_activate_admission() {
        let request = json!({
            "task_class": "coach",
            "dispatch_target": "coach",
            "implementation_isolation": {
                "artifact_contract": "stage_attempt_implementation_artifact_v1",
                "owned_paths": ["crates/vida/src/lib.rs"]
            }
        });
        let admission = admit_host_bridge_implementation_attempt(&request, None);

        assert_eq!(admission.decision, "admit");
        assert_eq!(admission.verdict, "pass");
        assert!(admission.blocker_codes.is_empty());
    }
}
